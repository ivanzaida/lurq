//! A video player: a picture the app gives, the controls' bar over it, and the app's own layer over the picture.
//!
//! Nothing is decoded here. The player draws what the app gives it as its picture and the values the app holds
//! (whether it plays, where it is, how loud), and tells the app what a person asks of it: a press on a control
//! changes nothing by itself, the app answers by setting the prop.

use std::{
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering},
  },
  time::Duration,
};

use super::{
  MEDIA_RATES, MediaCaptions, MediaSpeedMenu, MediaTime, MediaTimeSize, MediaVolume, MediaVolumeSize, Menu, MenuItem,
  MenuItemPart, MenuPart, PlayButton, PlayButtonSize, PlayButtonVariant, SeekBar, Tooltip, icons, media_rate_text,
  media_time_text, names,
  part_style::{Length, PartTransition},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{fading, glyph, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  animation::Transition,
  app::{
    component::Component,
    ctx::{Ctx, Placement, Timeout},
    events::{GesturePhase, KeyboardEvent, MouseEvent, PanEvent},
    theme::{MotionDuration, MotionEasing, RadiusSize, Theme, TypographyStyle},
  },
  components::{Column, Modal, Root, Row, Stack},
  core::{ElementRef, Signal},
  layout::{
    Alignment,
    layout_kind::Justify,
    text_style::{FontWeight, TextAlign},
  },
  node::{CursorIcon, Element, HitTestBehavior, color::Color, dimension::Dimension, outline::Outline},
  time::Instant,
};

/// How long the pointer rests over a playing video before its controls fade out.
pub const VIDEO_PLAYER_HIDE_AFTER: Duration = Duration::from_secs(3);

/// Under this width a player whose form follows its width is the compact one.
pub const VIDEO_PLAYER_COMPACT_BELOW: f32 = 400.0;

/// Under this width a player whose form follows its width is the minimal one: play, the seek bar and the time.
pub const VIDEO_PLAYER_MINIMAL_BELOW: f32 = 280.0;

/// Under this width the volume's slider of the large player folds into its button.
pub const VIDEO_PLAYER_VOLUME_FOLDS_BELOW: f32 = 480.0;

/// The shape of a picture unless the app names another: its width over its height, 16 by 9.
pub const VIDEO_PLAYER_ASPECT: f32 = 16.0 / 9.0;

/// How wide the window of a player in picture in picture is; it is as high as the picture's aspect makes it.
pub const VIDEO_PLAYER_PICTURE_WIDTH: u32 = 400;

/// How far a double tap on a side of the touch form seeks, in seconds, and each tap after it.
pub const VIDEO_PLAYER_TAP_SEEKS: f64 = 10.0;

/// How long the mark of a double tap stays after the last tap.
pub const VIDEO_PLAYER_MARK_STAYS: Duration = Duration::from_millis(800);

/// How far a drag down takes a player in fullscreen before letting go leaves fullscreen, and how fast a throw
/// down does: a lightbox's.
const VIDEO_PLAYER_DROP_LEAVES: f32 = 120.0;
const VIDEO_PLAYER_THROW_LEAVES: f32 = 900.0;
/// How soon a second press of the picture asks for fullscreen.
pub const VIDEO_PLAYER_DOUBLE_PRESS: Duration = Duration::from_millis(500);

/// How far Left and Right seek, in seconds; J and L seek twice as far.
pub const VIDEO_PLAYER_SEEK_STEP: f64 = 5.0;

/// How far Up and Down move the volume, of 1.
pub const VIDEO_PLAYER_VOLUME_STEP: f64 = 0.05;

/// The parts of a video player a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VideoPlayerPart {
  /// The player's box: the picture's size, rounded and clipped.
  Root,
  /// The layer of the picture the app gives, and of the poster over it.
  Media,
  /// The dark layer over the picture while it ended or failed.
  Veil,
  /// The app's own layer over the whole picture, in both forms.
  Overlay,
  /// The middle of the picture: the play disc, the spinner, the error.
  Center,
  /// The spinner in the middle while it waits for what it plays.
  Spinner,
  /// The block in the middle that says it cannot play.
  Error,
  /// The glyph at the top of the error.
  ErrorIcon,
  /// The column of the error's title and description.
  Text,
  /// The error's title.
  Title,
  /// The line under the error's title.
  Description,
  /// The button that asks the app to try again.
  Retry,
  /// The room of the captions' line: above the controls, or at the bottom while they are hidden.
  Captions,
  /// The box of the captions' line and its text.
  Caption,
  /// The room of the poster's length at its bottom end.
  Chip,
  /// The length on the poster.
  ChipValue,
  /// The controls' bar at the bottom: the seek bar and the row, on a gradient. Its background is the color the
  /// gradient ends in at the bottom, from nothing at its top.
  Controls,
  /// The row of buttons under the seek bar.
  Row,
  /// A button of the bar.
  Control,
  /// The glyph of a button of the bar.
  ControlIcon,
  /// The speed in the bar, a button of text: "1×".
  Speed,
  /// The room of the time in the bar.
  Time,
  /// The time of the minimal player: how far it is, alone.
  Elapsed,
  /// How far a menu of the bar stands over the row: its gap.
  Menus,
  /// The settings menu's panel.
  SettingsMenu,
  /// The panel of the compact player's menu.
  MoreMenu,
  /// The value at the end of a row of a menu of the player: "1080p", "Normal".
  MenuValue,
  /// The touch form: the dark layer over the whole picture while its controls show.
  Scrim,
  /// The touch form: the row at the top, of what leaves the player, its title, the captions and the settings.
  Top,
  /// The touch form: the column of the title and the line under it, in the top row of the landscape one.
  Heading,
  /// The touch form: the line under the title.
  Line,
  /// The touch form: a round button of the top and the bottom rows.
  Touch,
  /// The touch form: the glyph of such a button.
  TouchIcon,
  /// The touch form: the row in the middle, of ten seconds back, play, and ten seconds on.
  Transport,
  /// The touch form: a button of ten seconds back or on.
  Skip,
  /// The touch form: the glyph of such a button.
  SkipIcon,
  /// The touch form: the disc of play in the middle.
  Disc,
  /// The touch form: the glyph of the disc.
  DiscIcon,
  /// The touch form: the row at the bottom, of the time and of fullscreen.
  Foot,
  /// The touch form: the speed in the bottom row of the landscape one, a pill of text: "1×".
  Pill,
  /// The touch form: the room of the seek bar under the bottom row.
  Rail,
  /// The touch form: the mark of a double tap, an ellipse that comes in from the side tapped, half of it.
  Mark,
  /// The touch form: the glyph of the mark.
  MarkIcon,
  /// The touch form: the words of the mark, "10 seconds".
  MarkText,
}

names!(VideoPlayerPart {
  Root => "root",
  Media => "media",
  Veil => "veil",
  Overlay => "overlay",
  Center => "center",
  Spinner => "spinner",
  Error => "error",
  ErrorIcon => "error_icon",
  Text => "text",
  Title => "title",
  Description => "description",
  Retry => "retry",
  Captions => "captions",
  Caption => "caption",
  Chip => "chip",
  ChipValue => "chip_value",
  Controls => "controls",
  Row => "row",
  Control => "control",
  ControlIcon => "control_icon",
  Speed => "speed",
  Time => "time",
  Elapsed => "elapsed",
  Menus => "menus",
  SettingsMenu => "settings_menu",
  MoreMenu => "more_menu",
  MenuValue => "menu_value",
  Scrim => "scrim",
  Top => "top",
  Heading => "heading",
  Line => "line",
  Touch => "touch",
  TouchIcon => "touch_icon",
  Transport => "transport",
  Skip => "skip",
  SkipIcon => "skip_icon",
  Disc => "disc",
  DiscIcon => "disc_icon",
  Foot => "foot",
  Pill => "pill",
  Rail => "rail",
  Mark => "mark",
  MarkIcon => "mark_icon",
  MarkText => "mark_text",
});

/// The states of a video player a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VideoPlayerState {
  /// It plays.
  Playing,
  /// It waits for what it plays: the spinner in the middle.
  Waiting,
  /// It played to the end: the veil and replay.
  Ended,
  /// It cannot play: the veil and the error.
  Error,
  /// What it plays has no end: the live badge for the time.
  Live,
  /// The sound is off.
  Muted,
  /// The captions are on.
  Captions,
  /// It fills the screen.
  Fullscreen,
  /// It shows the poster before the first play.
  Poster,
  /// It has no controls: the picture and the app's layer alone.
  Bare,
  /// Its controls have faded out while it plays.
  Hidden,
  /// It is narrower than the volume's slider needs.
  Narrow,
  /// A button of the bar that is on: the captions while they show, the button of a menu while it is open. Resolved
  /// for those buttons alone.
  On,
  /// It is the touch form: larger controls over a scrim, shown and hidden by a tap.
  Touch,
  /// The touch form as wide as a phone on its side: in fullscreen.
  Landscape,
}

names!(VideoPlayerState {
  Playing => "playing",
  Waiting => "waiting",
  Ended => "ended",
  Error => "error",
  Live => "live",
  Muted => "muted",
  Captions => "captions",
  Fullscreen => "fullscreen",
  Poster => "poster",
  Bare => "bare",
  Hidden => "hidden",
  Narrow => "narrow",
  On => "on",
  Touch => "touch",
  Landscape => "landscape",
});

/// The forms of a video player, by the room it has.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VideoPlayerSize {
  /// The minimal player, as small as 200 by 113: play, the seek bar and the time in one row.
  Sm,
  /// The compact player, as 320 by 180: play, the time, mute, a menu of the rest, and fullscreen.
  Md,
  /// The player of the board, 640 by 360: every control in the bar.
  #[default]
  Lg,
}

names!(VideoPlayerSize {
  Sm => "sm",
  Md => "md",
  Lg => "lg",
});

impl VideoPlayerSize {
  /// The form that fits `width`.
  pub fn for_width(width: f32) -> Self {
    if width < VIDEO_PLAYER_MINIMAL_BELOW {
      Self::Sm
    } else if width < VIDEO_PLAYER_COMPACT_BELOW {
      Self::Md
    } else {
      Self::Lg
    }
  }

  fn of(name: Name) -> Self {
    [Self::Sm, Self::Md]
      .into_iter()
      .find(|size| Name::from(*size) == name)
      .unwrap_or(Self::Lg)
  }
}

/// A menu of the player's bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VideoPlayerMenu {
  /// The large player's settings: quality, speed and captions, each with its choices beside it.
  Settings,
  /// The large player's speeds.
  Speed,
  /// The compact player's menu: captions, speed, quality and picture in picture.
  More,
}

/// What builds a video player's nodes: a theme's in place of the default one.
pub type VideoPlayerRenderer = Arc<dyn Fn(VideoPlayerView) -> Element + Send + Sync>;

type Handler<T> = Arc<dyn Fn(T) + Send + Sync>;
type Action = Arc<dyn Fn() + Send + Sync>;
/// What builds a layer of the app's from what the player is doing: see [`VideoPlayer::child_with`].
type Layer = Arc<Mutex<Box<dyn FnMut(&VideoPlayback) -> Element + Send>>>;

/// What a player is doing, as a layer of the app's own is built from it ([`VideoPlayer::child_with`]): where
/// it is and how long, whether it plays, and the ways to move it. It is what the player's own bar would show,
/// so a seek line and a time of the app's are in step with the picture, with a source or without one.
#[derive(Clone)]
pub struct VideoPlayback {
  /// Whether it plays.
  pub playing: bool,
  /// Whether it waits for what it plays.
  pub waiting: bool,
  /// Whether it played to its end.
  pub ended: bool,
  /// Whether it cannot play.
  pub failed: bool,
  /// Whether its source has shown no frame yet: it opens, or its first frame is still to come.
  pub opening: bool,
  /// Whether the poster is over the picture.
  pub poster_shown: bool,
  /// Where it is, in seconds.
  pub position: f64,
  /// How long it is, in seconds, once that is known.
  pub duration: Option<f64>,
  /// How far it is loaded, in seconds, where that is known.
  pub buffered: Option<f64>,
  /// How loud it is, from 0 to 1, and whether it is silenced.
  pub volume: f64,
  pub muted: bool,
  /// How fast it goes: 1 is as it was made.
  pub rate: f64,
  seek: Handler<f64>,
  seek_near: Handler<f64>,
  play: Handler<bool>,
  mute: Handler<bool>,
}

impl VideoPlayback {
  /// Sends it to `seconds`, kept within what it plays, as its own seek bar does: its source goes there, and the
  /// app's handlers of its position are told.
  pub fn seek(&self, seconds: f64) {
    (self.seek)(seconds);
  }

  /// Sends it near `seconds`, to the frame there its source shows quickest, as [`Media::seek_near`] does: where a
  /// finger lets go of a line of time. The app's handlers of its position are told `seconds`.
  ///
  /// [`Media::seek_near`]: crate::media::Media::seek_near
  pub fn seek_near(&self, seconds: f64) {
    (self.seek_near)(seconds);
  }

  /// Plays or pauses it, as its own button does: one that ended starts over.
  pub fn set_playing(&self, playing: bool) {
    (self.play)(playing);
  }

  /// Silences it, or not.
  pub fn set_muted(&self, muted: bool) {
    (self.mute)(muted);
  }

  /// The share of it that is played, from 0 to 1: nothing where its length is not known.
  pub fn progress(&self) -> f64 {
    match self.duration {
      Some(duration) if duration > 0.0 => (self.position / duration).clamp(0.0, 1.0),
      _ => 0.0,
    }
  }
}

impl std::fmt::Debug for VideoPlayback {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("VideoPlayback")
      .field("playing", &self.playing)
      .field("waiting", &self.waiting)
      .field("ended", &self.ended)
      .field("failed", &self.failed)
      .field("opening", &self.opening)
      .field("position", &self.position)
      .field("duration", &self.duration)
      .finish_non_exhaustive()
  }
}

/// A value the app holds and the handlers told what a person makes it.
#[derive(Clone)]
struct Held<T: Copy> {
  value: T,
  handlers: Vec<Handler<T>>,
}

impl<T: Copy + Send + Sync + 'static> Held<T> {
  fn new(value: T) -> Self {
    Self {
      value,
      handlers: Vec::new(),
    }
  }

  /// The value of `signal`, which what a person does sets before any handler is told.
  fn bind(&mut self, value: T, set: impl Fn(T) + Send + Sync + 'static) {
    self.value = value;
    self.handlers.insert(0, Arc::new(set));
  }

  fn hold(&mut self, value: T, handler: impl Fn(T) + Send + Sync + 'static) {
    self.value = value;
    self.handlers.push(Arc::new(handler));
  }

  fn add(&mut self, handler: impl Fn(T) + Send + Sync + 'static) {
    self.handlers.push(Arc::new(handler));
  }

  /// What tells every handler `value`.
  fn teller(&self) -> impl Fn(T) + Send + Sync + Clone + 'static {
    let handlers = self.handlers.clone();
    move |value| {
      for handler in &handlers {
        handler(value);
      }
    }
  }
}

/// The words a player says and the names of its controls.
#[derive(Clone)]
struct Words {
  label: Arc<str>,
  play: Arc<str>,
  pause: Arc<str>,
  replay: Arc<str>,
  mute: Arc<str>,
  unmute: Arc<str>,
  volume: Arc<str>,
  seek: Arc<str>,
  captions: Arc<str>,
  speed: Arc<str>,
  speed_heading: Arc<str>,
  settings: Arc<str>,
  quality: Arc<str>,
  picture_in_picture: Arc<str>,
  in_picture: Arc<str>,
  back: Arc<str>,
  unplayable: Arc<str>,
  fullscreen: Arc<str>,
  exit_fullscreen: Arc<str>,
  more: Arc<str>,
  retry: Arc<str>,
  rewind: Arc<str>,
  forward: Arc<str>,
  seconds: Arc<str>,
  minimize: Arc<str>,
  leave: Arc<str>,
  auto: Arc<str>,
  on: Arc<str>,
  off: Arc<str>,
  live: Arc<str>,
}

impl Words {
  /// The words a locale has for a player.
  fn of(words: &crate::core::MediaWords) -> Self {
    Self {
      label: words.video_player.clone(),
      play: words.play.clone(),
      pause: words.pause.clone(),
      replay: words.replay.clone(),
      mute: words.mute.clone(),
      unmute: words.unmute.clone(),
      volume: words.volume.clone(),
      seek: words.seek.clone(),
      captions: words.captions.clone(),
      speed: words.speed.clone(),
      speed_heading: words.playback_speed.clone(),
      settings: words.settings.clone(),
      quality: words.quality.clone(),
      picture_in_picture: words.picture_in_picture.clone(),
      in_picture: words.in_picture.clone(),
      back: words.back_to_window.clone(),
      unplayable: words.unplayable_video.clone(),
      fullscreen: words.fullscreen.clone(),
      exit_fullscreen: words.exit_fullscreen.clone(),
      more: words.more.clone(),
      retry: words.try_again.clone(),
      rewind: words.rewind.clone(),
      forward: words.forward.clone(),
      seconds: words.seconds.clone(),
      minimize: words.minimize.clone(),
      leave: words.back.clone(),
      auto: words.auto.clone(),
      on: words.on.clone(),
      off: words.off.clone(),
      live: words.live.clone(),
    }
  }
}

/// The words of the locale of what is being built: the app's in a render, English outside one.
impl Default for Words {
  fn default() -> Self {
    Self::of(&super::current_locale().media)
  }
}

/// What a player that is in fullscreen fills.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VideoFullscreen {
  /// Its window, at the size the window has: the default. The video is as large as the app, and the app stays
  /// where it is on the desktop.
  #[default]
  Window,
  /// Its window, which goes to the whole screen.
  Screen,
  /// Nothing: it is drawn where it stands, and the app lays a player in fullscreen out itself.
  App,
}

/// A video player.
///
/// ```
/// use lurq::{
///   components::Column,
///   core::Signal,
///   ui::VideoPlayer,
/// };
///
/// # fn build(playing: Signal<bool>, position: Signal<f32>, volume: Signal<f32>, muted: Signal<bool>) {
/// // The app decodes; the player is given the frame to draw, and the values the app holds.
/// let player = VideoPlayer::new()
///   .media(Column::new())
///   .duration(250.0)
///   .playing(playing)
///   .position(position)
///   .volume(volume)
///   .muted(muted)
///   .on_position_change_end(|to| println!("seek to {to}"));
/// // Only the picture, with the app's own controls over it, as in a vertical feed.
/// let feed = VideoPlayer::new().media(Column::new()).controls(false).child(Column::new());
/// # }
/// ```
///
/// Each value the app holds has three forms: bound to a signal (`playing(signal)`), which a person's press sets
/// before the handlers are told; held by the app (`playing_controlled(value, handler)`), which a press does not
/// change until the app answers; and a handler alone (`on_playing_change`). A press tells what the value is to be.
///
/// With [`controls`](Self::controls) (the default) the bar and the middle of the picture are drawn; without them
/// only the picture and the app's layer ([`child`](Self::child)), which is drawn in both forms. Which of the three
/// forms of the bar is drawn follows the player's own width, unless [`size`](Self::size) says.
///
/// [`mount`](Self::mount) keeps what the player needs between renders: the timer that fades the controls out while
/// it plays, its width, its menus, and the tooltips of its controls. A player given to a tree as it is
/// (`.child(VideoPlayer::new())`) is the same component, declared: what renders the tree mounts it.
/// [`build`](Self::build) is a still picture of it, for a test.
#[derive(Clone)]
pub struct VideoPlayer {
  media: Option<Element>,
  poster: Option<Element>,
  children: Vec<Element>,
  /// The layers of the app's built from what it is doing, over its other children.
  layers: Vec<Layer>,
  /// What it plays, as tooling is told it: the source it was given.
  said_source: Option<Arc<str>>,
  /// Its source has shown no frame yet: it opens, or its first frame is to come.
  opening: bool,
  /// The poster stands only until its source has a frame to show.
  poster_until_frame: bool,
  /// Whether its source has shown a frame: `None` for a player without a source.
  framed: Option<bool>,
  controls: bool,
  playing: Held<bool>,
  position: Held<f64>,
  on_position_change_end: Vec<Handler<f64>>,
  duration: Option<f64>,
  buffered: Option<f64>,
  chapters: Vec<f64>,
  preview: Option<Arc<dyn Fn(f64) -> Element + Send + Sync>>,
  live: bool,
  waiting: bool,
  ended: bool,
  error: Option<Arc<str>>,
  error_description: Option<Arc<str>>,
  /// The words it was told to speak in, in place of the app's: what its source's errors are said in.
  locale: Option<crate::core::MediaWords>,
  on_retry: Vec<Action>,
  /// The handlers of its source's events.
  events: super::media_events::MediaEvents,
  #[cfg(feature = "media")]
  source: Option<crate::resources::Source>,
  touch: Option<bool>,
  title: Option<Arc<str>>,
  description: Option<Arc<str>>,
  on_back: Vec<Action>,
  volume: Held<f64>,
  on_volume_change_end: Vec<Handler<f64>>,
  muted: Held<bool>,
  rate: Held<f64>,
  rates: Vec<f32>,
  captions: Held<bool>,
  caption_text: Option<Arc<str>>,
  caption_cues: Option<MediaCaptions>,
  captions_track: Option<Arc<str>>,
  captions_button: bool,
  settings_button: bool,
  looped: bool,
  ahead: Option<f64>,
  #[cfg(feature = "media")]
  priority: Option<crate::media::Priority>,
  qualities: Vec<Arc<str>>,
  quality: Held<Option<usize>>,
  fullscreen: Held<bool>,
  fullscreen_fills: VideoFullscreen,
  picture_in_picture: Held<bool>,
  picture_in_picture_window: bool,
  size: Option<Name>,
  width: Option<Length>,
  height: Option<Length>,
  max_width: Option<Length>,
  aspect: f32,
  force_menu: Option<VideoPlayerMenu>,
  force_controls_shown: Option<bool>,
  words: Words,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Default for VideoPlayer {
  fn default() -> Self {
    Self::new()
  }
}

/// The three forms of a value of the player: bound, held, and a handler alone.
macro_rules! held {
  ($field:ident, $controlled:ident, $on:ident, $ty:ty, $signal:ty, $doc:literal) => {
    #[doc = concat!("Bound to `signal`: ", $doc, " What a person does sets the signal, and then the handlers are told.")]
    pub fn $field(mut self, signal: Signal<$signal>) -> Self {
      let value = signal.get();
      self.$field.bind(value.into(), move |next: $ty| signal.set(next as $signal));
      self
    }

    #[doc = concat!("Held by the app: ", $doc, " A press tells `handler` what it is to be, and changes nothing until the app answers.")]
    pub fn $controlled(mut self, value: $ty, handler: impl Fn($ty) + Send + Sync + 'static) -> Self {
      self.$field.hold(value, handler);
      self
    }

    #[doc = concat!("Called with what a person makes it: ", $doc, " A second handler is called after the first.")]
    pub fn $on(mut self, handler: impl Fn($ty) + Send + Sync + 'static) -> Self {
      self.$field.add(handler);
      self
    }
  };
}

/// Words lurq writes itself, each with its own method.
macro_rules! words {
  ($($method:ident => $field:ident, $doc:literal;)*) => {
    $(
      #[doc = $doc]
      pub fn $method(mut self, words: &str) -> Self {
        self.words.$field = Arc::from(words);
        self
      }
    )*
  };
}

impl VideoPlayer {
  /// A player of nothing yet: a black picture, paused at the start, of no known length.
  pub fn new() -> Self {
    Self {
      media: None,
      poster: None,
      children: Vec::new(),
      layers: Vec::new(),
      said_source: None,
      opening: false,
      poster_until_frame: false,
      framed: None,
      controls: true,
      playing: Held::new(false),
      position: Held::new(0.0),
      on_position_change_end: Vec::new(),
      duration: None,
      buffered: None,
      chapters: Vec::new(),
      preview: None,
      live: false,
      waiting: false,
      ended: false,
      error: None,
      error_description: None,
      locale: None,
      on_retry: Vec::new(),
      events: super::media_events::MediaEvents::default(),
      #[cfg(feature = "media")]
      source: None,
      touch: None,
      title: None,
      description: None,
      on_back: Vec::new(),
      volume: Held::new(1.0),
      on_volume_change_end: Vec::new(),
      muted: Held::new(false),
      rate: Held::new(1.0),
      rates: MEDIA_RATES.to_vec(),
      captions: Held::new(false),
      caption_text: None,
      caption_cues: None,
      captions_track: None,
      captions_button: true,
      settings_button: true,
      looped: false,
      ahead: None,
      #[cfg(feature = "media")]
      priority: None,
      qualities: Vec::new(),
      quality: Held::new(None),
      fullscreen: Held::new(false),
      fullscreen_fills: VideoFullscreen::Window,
      picture_in_picture: Held::new(false),
      picture_in_picture_window: true,
      size: None,
      width: None,
      height: None,
      max_width: None,
      aspect: VIDEO_PLAYER_ASPECT,
      force_menu: None,
      force_controls_shown: None,
      words: Words::default(),
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// What is drawn as the picture: the frame the app has, filling the player.
  pub fn media(mut self, media: impl Into<Element>) -> Self {
    self.media = Some(media.into());
    self
  }

  /// What is drawn over the picture before the first play, with the length at its bottom end: until it plays, is
  /// somewhere past the start, waits or has ended. See [`poster_until_frame`](Self::poster_until_frame) for a
  /// poster that gives way to the video's first frame.
  pub fn poster(mut self, poster: impl Into<Element>) -> Self {
    self.poster = Some(poster.into());
    self
  }

  /// Whether the poster stands only until its [`source`](Self::source) has a frame of its own to show, also
  /// where it has not played yet. `false` unless told: the poster is the picture of a player that was never
  /// played, as a player with a play button on its poster wants.
  ///
  /// A video opened ahead and left paused decodes its first frame, and a feed that slides such a player in under
  /// a finger shows that frame with this, where it showed the poster: when it is then told to play, it goes on
  /// from the very picture that was seen, and not from another moment of the video the poster was made of. The
  /// poster stays until that frame is decoded (the box is never the black a media opens with), and for good
  /// over a source with no picture, a sound. A player without a source has no frame to wait for: this says
  /// nothing to it.
  pub fn poster_until_frame(mut self, until_frame: bool) -> Self {
    self.poster_until_frame = until_frame;
    self
  }

  /// Something of the app's own over the whole picture, with the controls and without them: a layer the size of
  /// the picture, under the middle of the player and its bar.
  pub fn child(mut self, child: impl Into<Element>) -> Self {
    self.children.push(child.into());
    self
  }

  /// Things of the app's own over the picture, in order.
  pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.children.extend(children.into_iter().map(Into::into));
    self
  }

  /// A layer of the app's own over the picture, built from what the player is doing each time the player is
  /// rendered: its time and its length, whether it plays, waits or ended, and the ways to move it
  /// ([`VideoPlayback`]). It is how an app's own controls show and set the time of a player with a
  /// [`source`](Self::source), whose time is the media's and no signal of the app's:
  ///
  /// ```
  /// use lurq::ui::{MediaTime, SeekBar, VideoPlayer};
  ///
  /// let feed = VideoPlayer::new().controls(false).child_with(|playback| {
  ///   let to = playback.clone();
  ///   let line = SeekBar::controlled(playback.position, playback.duration.unwrap_or(0.0), move |at| to.seek(at));
  ///   lurq::components::Column::new()
  ///     .child(MediaTime::new(playback.position, playback.duration))
  ///     .child(line)
  /// });
  /// ```
  ///
  /// It lies over the layers given with [`child`](Self::child), in the order it was added among its like.
  pub fn child_with<E: Into<Element>>(mut self, mut layer: impl FnMut(&VideoPlayback) -> E + Send + 'static) -> Self {
    let build: Box<dyn FnMut(&VideoPlayback) -> Element + Send> = Box::new(move |playback| layer(playback).into());
    self.layers.push(Arc::new(Mutex::new(build)));
    self
  }

  /// What is behind the picture: the box's fill where the picture does not cover it (beside a video of another
  /// shape than the box, and before its first frame). Black unless told. With a clear color
  /// (`Color::new(0, 0, 0, 0)`) what the app has under the player shows there, a blurred still of the same
  /// video as in a feed. The same as styling the part: `style(VideoPlayerPart::Root, |s| s.background(..))`.
  pub fn background(self, color: impl Into<crate::node::BackgroundColor>) -> Self {
    let color = color.into();
    self.style(VideoPlayerPart::Root, move |s| s.background(color))
  }

  /// Whether the bar and the middle of the picture are drawn. Without them the player is its picture and the
  /// app's layer, as in a vertical feed.
  pub fn controls(mut self, controls: bool) -> Self {
    self.controls = controls;
    self
  }

  held!(
    playing,
    playing_controlled,
    on_playing_change,
    bool,
    bool,
    "whether it plays."
  );

  /// Bound to `signal`: where it is, in seconds. A drag of the seek bar and the keys that seek set it.
  pub fn position(mut self, signal: Signal<f32>) -> Self {
    let value = f64::from(signal.get());
    self.position.bind(value, move |next| signal.set(next as f32));
    self
  }

  /// Held by the app: where it is, in seconds. A drag tells `handler` where it is to be as the handle moves, and
  /// changes nothing until the app answers.
  pub fn position_controlled(
    mut self,
    position: impl Into<f64>,
    handler: impl Fn(f64) + Send + Sync + 'static,
  ) -> Self {
    self.position.hold(position.into(), handler);
    self
  }

  /// Called with where a person moves it, in seconds, as the seek bar's handle moves and on each key that seeks. A
  /// second handler is called after the first.
  pub fn on_position_change(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.position.add(handler);
    self
  }

  /// Called with where a seek ends, in seconds: where the handle is let go, and on each key that seeks. A player
  /// that seeks only when the handle is let go listens to this. A second handler is called after the first.
  pub fn on_position_change_end(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.on_position_change_end.push(Arc::new(handler));
    self
  }

  /// How long what it plays is, in seconds. Until it is said, the time reads `-:-- / -:--`.
  pub fn duration(mut self, duration: impl Into<f64>) -> Self {
    self.duration = Some(duration.into());
    self
  }

  /// How far what it plays is loaded, in seconds from its start: drawn in the seek bar.
  pub fn buffered(mut self, buffered: impl Into<f64>) -> Self {
    self.buffered = Some(buffered.into());
    self
  }

  /// Where its chapters start, in seconds: the seek bar is cut there.
  pub fn preview(mut self, preview: impl Fn(f64) -> Element + Send + Sync + 'static) -> Self {
    self.preview = Some(Arc::new(preview));
    self
  }

  /// Where its chapters start, in seconds: the seek bar is cut there.
  pub fn chapters(mut self, starts: &[f64]) -> Self {
    self.chapters = starts.to_vec();
    self
  }

  /// Whether what it plays has no end: the live badge stands for the time, the seek bar is full (or the window
  /// that can be gone back in, given as the duration), and there is no speed.
  pub fn live(mut self, live: bool) -> Self {
    self.live = live;
    self
  }

  /// Whether it waits for what it plays: a spinner in the middle of the picture, and the bar stays.
  pub fn waiting(mut self, waiting: bool) -> Self {
    self.waiting = waiting;
    self
  }

  /// Whether it played to the end: the veil, replay in the middle and in the bar, the seek bar full, and the bar
  /// stays. A press of replay tells the playing handlers `true`: the app plays again from the start.
  pub fn ended(mut self, ended: bool) -> Self {
    self.ended = ended;
    self
  }

  /// That it cannot play, and the title that says so: the veil and the error in the middle, and no bar.
  pub fn error(mut self, title: &str) -> Self {
    self.error = Some(Arc::from(title));
    self
  }

  /// The line under the error's title: what can be done.
  pub fn error_description(mut self, description: &str) -> Self {
    self.error_description = Some(Arc::from(description));
    self
  }

  /// Called on a press of the error's button: the app tries again. The button is drawn once this is given. A second
  /// handler is called after the first.
  pub fn on_retry(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_retry.push(Arc::new(handler));
    self
  }

  /// Called once its [`source`](Self::source) opened and is ready to play, with its length, its picture's size and
  /// whether it has sound: once for a source, and again for one tried again. A player without a source is never
  /// told: what it shows is the app's. A second handler is called after the first.
  ///
  /// Each event of a source is told on the UI thread once each time it happens, in the order the media lives
  /// them: it opened, it began or stopped waiting, it began or stopped playing, it ended, it failed.
  pub fn on_load(mut self, handler: impl Fn(&crate::core::MediaInfo) + Send + Sync + 'static) -> Self {
    self.events.load.push(Arc::new(handler));
    self
  }

  /// Called once its [`source`](Self::source) cannot play, with why: a kind, which the app's locale puts into the
  /// words the player shows under its error's title ([`MediaWords::error`](crate::core::MediaWords::error)). Once
  /// until the person tries again, which opens the source anew. A player without a source is never told. A second
  /// handler is called after the first.
  pub fn on_error(mut self, handler: impl Fn(&crate::core::MediaError) + Send + Sync + 'static) -> Self {
    self.events.error.push(Arc::new(handler));
    self
  }

  /// Called each time its [`source`](Self::source) played to its end: again only after it played again. A player
  /// without a source is never told: its [`ended`](Self::ended) is the app's. A second handler is called after the
  /// first.
  pub fn on_ended(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.events.ended.push(Arc::new(handler));
    self
  }

  /// Called each time its [`source`](Self::source) really starts to play: it was told to, it does not wait for
  /// what it plays, and the first frame since is shown ([`Media::on_started`](crate::media::Media::on_started)).
  /// After [`on_load`](Self::on_load), which says it opened with nothing moving yet, and after
  /// [`on_playing_change`](Self::on_playing_change) said it was told to play: where a poster of the app's covers
  /// the player until the video moves, this is when to take it away. Told again after a pause, after it waited
  /// and after its end; not for a round of a [looped](Self::looped) one, which did not stop. A player without a
  /// source is never told. A second handler is called after the first.
  pub fn on_started(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.events.started.push(Arc::new(handler));
    self
  }

  /// Called with whether its [`source`](Self::source) waits for what it plays, each time that changes once it
  /// opened: `true` when it began waiting (an address read slower than it plays), `false` when it goes on. A
  /// player without a source is never told: its [`waiting`](Self::waiting) is the app's. A second handler is
  /// called after the first.
  pub fn on_waiting_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.events.waiting.push(Arc::new(handler));
    self
  }

  /// Bound to `signal`: how loud it is, from 0 to 1. Muting keeps it.
  pub fn volume(mut self, signal: Signal<f32>) -> Self {
    let value = f64::from(signal.get());
    self.volume.bind(value, move |next| signal.set(next as f32));
    self
  }

  /// Held by the app: how loud it is, from 0 to 1. A press tells `handler` what it is to be.
  pub fn volume_controlled(mut self, volume: impl Into<f64>, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.volume.hold(volume.into(), handler);
    self
  }

  /// Called with how loud a person makes it, as the slider moves and on Up and Down. A second handler is called
  /// after the first.
  pub fn on_volume_change(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.volume.add(handler);
    self
  }

  /// Called with the volume where the slider is let go. A second handler is called after the first.
  pub fn on_volume_change_end(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.on_volume_change_end.push(Arc::new(handler));
    self
  }

  held!(
    muted,
    muted_controlled,
    on_muted_change,
    bool,
    bool,
    "whether the sound is off."
  );

  /// Bound to `signal`: how fast it plays, 1 being the speed it was made at.
  pub fn rate(mut self, signal: Signal<f32>) -> Self {
    let value = f64::from(signal.get());
    self.rate.bind(value, move |next| signal.set(next as f32));
    self
  }

  /// Held by the app: how fast it plays. A choice tells `handler` what it is to be.
  pub fn rate_controlled(mut self, rate: impl Into<f64>, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.rate.hold(rate.into(), handler);
    self
  }

  /// Called with the speed a person chooses, where it is another than the one in use. A second handler is called
  /// after the first.
  pub fn on_rate_change(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.rate.add(handler);
    self
  }

  /// The speeds it offers, in place of [`MEDIA_RATES`].
  pub fn rates(mut self, rates: &[f32]) -> Self {
    self.rates = rates.to_vec();
    self
  }

  held!(
    captions,
    captions_controlled,
    on_captions_change,
    bool,
    bool,
    "whether the captions show."
  );

  /// The line of the captions to show now, while they are on: the cue the app has for the time. It is shown in
  /// place of what a track given with [`caption_cues`](Self::caption_cues) says.
  pub fn caption_text(mut self, text: &str) -> Self {
    self.caption_text = (!text.is_empty()).then(|| Arc::from(text));
    self
  }

  /// The track of the captions, read from a WebVTT or a SubRip file ([`MediaCaptions::parse`]): while the
  /// captions are on, the player shows what the track says at the position it is at, and nothing between two
  /// cues.
  pub fn caption_cues(mut self, track: &MediaCaptions) -> Self {
    self.caption_cues = Some(track.clone());
    self
  }

  /// The name of the captions' track, which the settings say while they are on: "English".
  pub fn captions_track(mut self, name: &str) -> Self {
    self.captions_track = Some(Arc::from(name));
    self
  }

  /// Whether the captions' button is drawn, with their row in the menus and the key C, where the player has
  /// captions: it is unless told. A player without captions has neither, whatever this says: one has them where
  /// it is given a track ([`caption_cues`](Self::caption_cues), [`captions_track`](Self::captions_track)) or a
  /// line ([`caption_text`](Self::caption_text)), where the app holds whether they show
  /// ([`captions`](Self::captions), [`on_captions_change`](Self::on_captions_change)), or where they are on.
  pub fn captions_button(mut self, shown: bool) -> Self {
    self.captions_button = shown;
    self
  }

  /// Whether the settings' button is drawn, with its menu, in the large form and the touch form: it is unless
  /// told, where the menu has something in it (the qualities, the speed, the captions). The large form keeps its
  /// speed's button without it; the compact form has no settings, and its "more" menu stays.
  pub fn settings_button(mut self, shown: bool) -> Self {
    self.settings_button = shown;
    self
  }

  /// Whether its [`source`](Self::source) goes on from its start at its end, as a clip in a feed does: the media
  /// is sent to its start in what it has open ([`Media::set_looped`](crate::media::Media::set_looped)), so
  /// nothing is opened or read again. `on_ended` is told each time round, and `playing` stays true: the player
  /// is not drawn as ended. Off unless told. A player without a source is drawn from what the app gives it, and
  /// this does nothing.
  pub fn looped(mut self, looped: bool) -> Self {
    self.looped = looped;
    self
  }

  /// How far past where it stands its [source](Self::source) is read before it is first told to play, in seconds
  /// ([`Media::set_ahead`](crate::media::Media::set_ahead)): what a player opened ahead (the next one of a feed)
  /// has there when it is shown. A second unless told; at least ten once it plays.
  pub fn ahead(mut self, seconds: f64) -> Self {
    self.ahead = Some(seconds);
    self
  }

  /// How much what its [source](Self::source) reads is wanted beside what other players of the same host read
  /// ([`Media::set_priority`](crate::media::Media::set_priority)): `High` for the video shown, `Low` for one two
  /// swipes away. Normal unless told; of one priority, a player that plays goes first.
  #[cfg(feature = "media")]
  pub fn priority(mut self, priority: crate::media::Priority) -> Self {
    self.priority = Some(priority);
    self
  }

  /// Whether the captions' button, their row in the menus and their key are there: see
  /// [`captions_button`](Self::captions_button).
  fn has_captions(&self) -> bool {
    self.captions_button
      && (self.caption_cues.is_some()
        || self.captions_track.is_some()
        || self.caption_text.is_some()
        || self.captions.value
        || !self.captions.handlers.is_empty())
  }

  /// The qualities it can play in, best first: "1080p", "720p". The settings offer them after "Auto".
  pub fn qualities(mut self, qualities: &[&str]) -> Self {
    self.qualities = qualities.iter().map(|quality| Arc::from(*quality)).collect();
    self
  }

  /// Bound to `signal`: which of the qualities it plays in, `None` for the one the player picks.
  pub fn quality(mut self, signal: Signal<Option<usize>>) -> Self {
    let value = signal.get();
    self.quality.bind(value, move |next| signal.set(next));
    self
  }

  /// Held by the app: which of the qualities it plays in, `None` for the one it picks. A choice tells `handler`.
  pub fn quality_controlled(
    mut self,
    quality: Option<usize>,
    handler: impl Fn(Option<usize>) + Send + Sync + 'static,
  ) -> Self {
    self.quality.hold(quality, handler);
    self
  }

  /// Called with the quality a person chooses, `None` for "Auto". A second handler is called after the first.
  pub fn on_quality_change(mut self, handler: impl Fn(Option<usize>) + Send + Sync + 'static) -> Self {
    self.quality.add(handler);
    self
  }

  held!(
    fullscreen,
    fullscreen_controlled,
    on_fullscreen_change,
    bool,
    bool,
    "whether it fills the screen; its button then exits."
  );

  /// What a mounted player in fullscreen fills: its window as it is ([`VideoFullscreen::Window`], the default),
  /// its window taken to the whole screen, or nothing where the app does it itself.
  /// Filling its window it keeps the room it had where it stands, and Escape asks to leave.
  pub fn fullscreen_fills(mut self, fills: VideoFullscreen) -> Self {
    self.fullscreen_fills = fills;
    self
  }

  held!(
    picture_in_picture,
    picture_in_picture_controlled,
    on_picture_in_picture_change,
    bool,
    bool,
    "whether it plays in a small window of its own over the others."
  );

  /// Whether a mounted player in picture in picture opens that window itself (the default): a small one with no
  /// frame, kept over the other windows of the desktop, in which it plays while what it says stands where it
  /// was. Without it the value is told and drawn, and the window is the app's to make.
  pub fn picture_in_picture_window(mut self, opens: bool) -> Self {
    self.picture_in_picture_window = opens;
    self
  }

  /// What it plays: a file, bytes, a resource of the app or an address. A mounted player opens it with the app's
  /// loader, decodes it with the system's decoder ([`Media`](crate::media::Media)) and draws its frames as its
  /// picture; where it is, how long it is, whether it plays, waits, ended or failed are the media's, in place of
  /// what the app would say. A press on a control still tells the app's handlers, and does what it says to the
  /// media: plays, pauses, goes to a time, changes the speed, the volume and the mute. An app that holds
  /// `playing`, `rate`, `volume` or `muted` moves the media by changing them. With no source the player is
  /// drawn from the values it is given, as before.
  ///
  /// Its sound is played through the system's output, and the video goes by what is heard.
  #[cfg(feature = "media")]
  pub fn source(mut self, source: &crate::resources::Source) -> Self {
    self.source = Some(source.clone());
    self
  }

  /// The touch form, for a finger: round buttons of 44 over a scrim, ten seconds back and on beside a larger play
  /// in the middle, the time and fullscreen over the seek bar at the bottom, and no volume (a phone has keys for
  /// it). A tap on the picture shows the controls and hides them; two on its left or its right third seek ten
  /// seconds, and more of them add up. In fullscreen it is the form for a phone on its side, with its
  /// [`title`](Self::title) at the top and its speed at the bottom. Unless told, it is the touch form where the
  /// person points with a finger ([`Pointer::Coarse`](crate::app::Pointer::Coarse),
  /// [`touch_form`](super::touch_form)), whatever its width and the theme's density.
  pub fn touch(mut self, touch: bool) -> Self {
    self.touch = Some(touch);
    self
  }

  /// Whether it is the touch form: as told, or as the person's pointer says.
  fn is_touch(&self) -> bool {
    self.touch.unwrap_or_else(super::touch_form)
  }

  /// What plays, said at the top of the touch form on its side.
  pub fn title(mut self, title: &str) -> Self {
    self.title = (!title.is_empty()).then(|| Arc::from(title));
    self
  }

  /// The line under the [`title`](Self::title).
  pub fn description(mut self, description: &str) -> Self {
    self.description = (!description.is_empty()).then(|| Arc::from(description));
    self
  }

  /// The touch form: called by the first button of the top row, which is drawn upright where a handler is given
  /// (to make the player small) and always on its side (to go back, which leaves fullscreen where there is none).
  pub fn on_back(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_back.push(Arc::new(handler));
    self
  }

  /// Which form it is drawn in, a [`VideoPlayerSize`], in place of the one its width gives it.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = Some(size.into());
    self
  }

  /// How wide it is, in place of the width of its form (640 for the large one). Mounted, with no
  /// [`height`](Self::height), it is as high as a picture of its [`aspect`](Self::aspect) that wide, from the
  /// first frame.
  pub fn width(mut self, width: impl Into<Length>) -> Self {
    self.width = Some(width.into());
    self
  }

  /// The most it is wide: what holds it may be narrower than its width, a window as narrow as a phone. Mounted,
  /// with no [`height`](Self::height), it keeps its [`aspect`](Self::aspect) at the width that leaves it.
  pub fn max_width(mut self, width: impl Into<Length>) -> Self {
    self.max_width = Some(width.into());
    self
  }

  /// The shape of the picture, its width over its height: 16 by 9 unless told, 9.0 / 16.0 for an upright one. A
  /// player with no [`height`](Self::height) of its own is as high as this makes it at its width. Nothing that is
  /// not a number over nothing is taken.
  pub fn aspect(mut self, aspect: f32) -> Self {
    if aspect.is_finite() && aspect > 0.0 {
      self.aspect = aspect;
    }
    self
  }

  /// How high it is, in place of the height of its form (360 for the large one).
  pub fn height(mut self, height: impl Into<Length>) -> Self {
    self.height = Some(height.into());
    self
  }

  /// Draws a menu of the bar open (`Some`), with nothing pressed: for a gallery or a test.
  pub fn force_menu(mut self, menu: Option<VideoPlayerMenu>) -> Self {
    self.force_menu = menu;
    self
  }

  /// Shows the controls (`Some(true)`) or fades them out (`Some(false)`) whatever the pointer does; `None` leaves
  /// it to the pointer, the focus and the timer.
  pub fn force_controls_shown(mut self, shown: Option<bool>) -> Self {
    self.force_controls_shown = shown;
    self
  }

  words! {
    label => label, "What the player is called, to what reads the tree: \"Video player\" unless told.";
    play_label => play, "The name of the play button while a press would play.";
    pause_label => pause, "The name of the play button while a press would pause.";
    replay_label => replay, "The name of the play button once it ended.";
    mute_label => mute, "The name of the mute button while a press would mute.";
    unmute_label => unmute, "The name of the mute button while a press would bring the sound back.";
    volume_label => volume, "The name of the volume's slider.";
    seek_label => seek, "The name of the seek bar.";
    captions_label => captions, "The name of the captions' button, and their row in the menus.";
    speed_label => speed, "The name of the speed's button, and its row in the menus.";
    speed_heading_text => speed_heading, "The heading over the speeds: \"Playback speed\".";
    settings_label => settings, "The name of the settings' button.";
    quality_label => quality, "The name of the quality's row in the menus.";
    picture_in_picture_label => picture_in_picture, "The name of the button of picture in picture.";
    in_picture_text => in_picture, "What stands where the player was while it plays in its small window.";
    back_label => back, "The button that brings it back from its small window: \"Back to the window\".";
    unplayable_text => unplayable, "The error's title where its source cannot be played: the reason is under it.";
    fullscreen_label => fullscreen, "The name of the fullscreen button while a press would fill the screen.";
    exit_fullscreen_label => exit_fullscreen, "The name of the fullscreen button while it fills the screen.";
    more_label => more, "The name of the compact player's menu button.";
    retry_label => retry, "The words of the error's button: \"Try again\".";
    rewind_label => rewind, "The touch form: the name of the button of ten seconds back.";
    forward_label => forward, "The touch form: the name of the button of ten seconds on.";
    seconds_text => seconds, "The touch form: the word after the count in the mark of a double tap, \"seconds\".";
    minimize_label => minimize, "The touch form: the name of the first button of the top row, upright.";
    leave_label => leave, "The touch form: the name of the first button of the top row, on its side.";
    auto_text => auto, "The quality the player picks: \"Auto\".";
    on_text => on, "The captions on, where their track has no name: \"On\".";
    off_text => off, "The captions off: \"Off\".";
    live_text => live, "The badge of what is live: \"LIVE\".";
  }

  /// The words this player speaks in, in place of the app's ([`App::set_locale`](crate::app::App::set_locale)):
  /// the names of its controls, its menus' words, and what its source says when it cannot play. Said before the
  /// words that are told one by one (`play_label` and the others), which stay what they were told.
  pub fn locale(mut self, locale: &crate::core::Locale) -> Self {
    self.words = Words::of(&locale.media);
    self.locale = Some(locale.media.clone());
    self
  }

  /// Puts the player in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this player over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this player.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the buttons of the bar as they are under the pointer, held down or focused from the keyboard, with
  /// nothing pointing at them: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the player's box. Its controls have ids of their own after it: `{id}-play`, `{id}-seek`,
  /// `{id}-mute`, `{id}-time`, `{id}-captions`, `{id}-speed`, `{id}-settings`, `{id}-pip`, `{id}-fullscreen`,
  /// `{id}-more`, `{id}-center`, `{id}-retry`, `{id}-controls`, `{id}-caption`, `{id}-overlay`, and its menus'
  /// panels `{id}-settings-menu`, `{id}-speed-menu` and `{id}-more-menu`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Whether it shows the poster: given one, before it first plays, and for as long as its source has shown no
  /// frame, whether or not it was told to play already. Told that the poster stands only until a frame
  /// ([`poster_until_frame`](Self::poster_until_frame)), a player with a source shows it for as long as the
  /// source has shown none, and no longer.
  fn shows_poster(&self) -> bool {
    let standing = match (self.poster_until_frame, self.framed) {
      (true, Some(framed)) => !framed,
      _ => self.opening || (!self.playing.value && self.position.value <= 0.0 && !self.waiting),
    };
    self.poster.is_some() && !self.ended && self.error.is_none() && standing
  }

  /// Builds a still picture of the player in `theme`, for a test or a screenshot: its controls shown, its menus
  /// closed, nothing kept between two of them. An app gives the player to its tree, or mounts it
  /// ([`mount`](Self::mount)).
  pub fn build(self, theme: &Theme) -> Element {
    let held = Live::still();
    view(self, theme, &held, None).render_themed()
  }

  /// The player mounted in `ctx` at once: it keeps its width, the timer that fades its controls out while it
  /// plays, its menus and its tooltips. Converting it into an element does the same once the render that returns
  /// it has returned.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let props = VideoPlayerProps(self);
    match key {
      Some(key) => ctx.mount_keyed::<VideoPlayerHost>(&key, props),
      None => ctx.mount::<VideoPlayerHost>(props),
    }
  }
}

/// What a mounted player keeps between renders; the still form makes a new one each time, which nothing renders
/// again from.
#[derive(Clone)]
struct Live {
  root: ElementRef,
  /// The row of buttons, which the menus hang from.
  row: ElementRef,
  /// What holds the buttons, whose keys (Space, the arrows) are their own while the focus is in it.
  buttons: ElementRef,
  /// Whether each menu is open: settings, speed, more.
  menus: [Signal<bool>; 3],
  /// Which row of the open menu shows its choices in the menu's place, by its place in the menu.
  page: Signal<Option<usize>>,
  /// When the picture was last pressed: a second press soon after asks for fullscreen.
  pressed: Arc<Mutex<Option<Instant>>>,
  /// The touch form: when the picture was last tapped, and on which third of it (-1 the left, 1 the right).
  tapped: Arc<Mutex<Option<(Instant, i8)>>>,
  /// The touch form: the mark of a double tap, forward or back and how many seconds so far.
  mark: Signal<Option<(bool, u32)>>,
  /// What takes the mark away a while after the last tap.
  unmark: Option<Timeout>,
  /// Whether the pointer moved over the player lately.
  shown: Signal<bool>,
  /// What fades the controls out after the pointer rests.
  hide: Option<Timeout>,
  /// What shows them again once it no longer plays.
  show: Option<Timeout>,
  /// Whether the time reads what is left.
  remaining: Signal<bool>,
  /// What the volume keeps between renders: the pointer over it, the focus in it, its slider dragged. Folded, its
  /// slider comes out for them, and the bar gives it the time's room. None of it in the still form.
  volume: Option<super::media_volume::Kept>,
  /// The width it was last laid out at, by what it decides: see [`bucket`].
  bucket: Signal<u8>,
  /// Whether its height follows its width at its aspect: the app gave a width, or a greatest one, and no height.
  follows: Arc<AtomicBool>,
  /// The aspect it follows, as the bits of an `f32`.
  aspect: Arc<AtomicU32>,
  /// Whether it fills its window: in fullscreen, unless the app does that itself.
  filling: Arc<AtomicBool>,
  /// Whether it has taken its window to the whole screen.
  screen: Arc<AtomicBool>,
  /// How large it was laid out where it stands, before it filled its window: the room it keeps there.
  rested: Arc<Mutex<(f32, f32)>>,
  /// In fullscreen: how far down a finger has dragged it, and whether the drag is the player's to leave
  /// fullscreen by.
  dropped: Signal<f32>,
  taken: Arc<AtomicBool>,
  /// The window it took to the whole screen, to give back. It is the window the player is rendered in when it
  /// goes there: the window a component is created under is not always its own (the root of a second window is
  /// mounted before that window is).
  window: Arc<Mutex<Option<crate::app::window::WindowHandle>>>,
  /// What its small window is given, while it plays in picture in picture.
  picture: Option<Picture>,
  /// What tells the small window, after a render, that the player changed.
  told: Option<Timeout>,
  /// What opens the small window.
  opener: Option<crate::app::WindowOpener>,
  /// What it plays, where it was given a source.
  #[cfg(feature = "media")]
  played: Option<Played>,
  mounted: bool,
}

/// What a mounted player with a source keeps: the media it opened, and what renders the player again for it.
#[cfg(feature = "media")]
#[derive(Clone)]
struct Played {
  /// The media, with the key of the source it was opened from: another source is another media.
  media: Arc<Mutex<Option<(u64, crate::media::Media)>>>,
  /// Counted up when the media changed, and while it plays: the player reads it and is rendered again.
  beat: Signal<u64>,
  /// Whether it plays: the clock that counts the beat runs while it does.
  plays: Arc<AtomicBool>,
  /// What the app last said of playing, of the speed, of how loud it is and of whether it is silenced: a
  /// change of them moves the media.
  said: Arc<Mutex<(Option<bool>, Option<f64>, Option<f64>, Option<bool>)>>,
  /// What tells the app the media's events.
  listener: super::media_events::Listener,
  /// Whether the next seek goes near its time, not to it ([`VideoPlayback::seek_near`]).
  near: Arc<AtomicBool>,
  _clock: crate::app::ctx::Interval,
}

/// What a player and its small window share: the player as it was last rendered (none once it left picture in
/// picture, which closes the window), a count the window renders again for, and whether the window was asked for.
#[derive(Clone)]
struct Picture {
  player: Arc<Mutex<Option<VideoPlayer>>>,
  changed: Signal<u64>,
  open: Arc<AtomicBool>,
}

impl Picture {
  fn put(&self, player: Option<VideoPlayer>) {
    *self.player.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = player;
  }

  fn held(&self) -> Option<VideoPlayer> {
    self
      .player
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner())
      .clone()
  }
}

/// The height of a picture of `aspect` (its width over its height) that is `width` wide.
fn shaped(width: f32, aspect: f32) -> f32 {
  (width / aspect).round()
}

/// What a width decides of the player: its form, and whether the volume's slider folds. 0 is not measured yet.
fn bucket(width: f32) -> u8 {
  match VideoPlayerSize::for_width(width) {
    VideoPlayerSize::Sm => 1,
    VideoPlayerSize::Md => 2,
    VideoPlayerSize::Lg if width < VIDEO_PLAYER_VOLUME_FOLDS_BELOW => 3,
    VideoPlayerSize::Lg => 4,
  }
}

impl Live {
  fn still() -> Self {
    Self {
      root: ElementRef::new(),
      row: ElementRef::new(),
      buttons: ElementRef::new(),
      menus: [Signal::new(false), Signal::new(false), Signal::new(false)],
      page: Signal::new(None),
      pressed: Arc::default(),
      tapped: Arc::default(),
      mark: Signal::new(None),
      unmark: None,
      shown: Signal::new(true),
      hide: None,
      show: None,
      remaining: Signal::new(false),
      volume: None,
      bucket: Signal::new(0),
      follows: Arc::new(AtomicBool::new(false)),
      aspect: Arc::new(AtomicU32::new(VIDEO_PLAYER_ASPECT.to_bits())),
      filling: Arc::default(),
      screen: Arc::default(),
      rested: Arc::default(),
      dropped: Signal::new(0.0),
      taken: Arc::default(),
      window: Arc::default(),
      picture: None,
      told: None,
      opener: None,
      #[cfg(feature = "media")]
      played: None,
      mounted: false,
    }
  }

  /// The pointer moved, or a control was used: the controls show, and fade again after the pointer rests.
  fn poke(&self) -> impl Fn() + Send + Sync + Clone + 'static {
    let (shown, hide) = (self.shown.clone(), self.hide.clone());
    move || {
      if !shown.get_untracked() {
        shown.set(true);
      }
      if let Some(hide) = &hide {
        hide.restart();
      }
    }
  }

  fn menu(&self, menu: VideoPlayerMenu) -> &Signal<bool> {
    &self.menus[menu as usize]
  }
}

/// A player in a tree is its component, declared: the component that renders the tree mounts it.
impl crate::app::component::IntoComponent for VideoPlayer {
  type Component = VideoPlayerHost;

  fn into_props(self) -> VideoPlayerProps {
    VideoPlayerProps(self)
  }

  fn component_key(&self) -> Option<Arc<str>> {
    self.key.clone()
  }
}

/// A player to mount.
#[doc(hidden)]
#[derive(Clone)]
pub struct VideoPlayerProps(VideoPlayer);

/// A player is built anew by every render of what holds it, so props are never the same as before.
impl PartialEq for VideoPlayerProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl crate::app::component::DevtoolsInspectable for VideoPlayerProps {}

/// What keeps a mounted player's timer, width and menus.
#[doc(hidden)]
pub struct VideoPlayerHost {
  live: Live,
}

impl Component for VideoPlayerHost {
  type Props = VideoPlayerProps;

  fn create(ctx: &mut Ctx) -> Self {
    let shown = ctx.signal(true);
    let hide = ctx.create_timeout(VIDEO_PLAYER_HIDE_AFTER, {
      let shown = shown.clone();
      move || shown.set(false)
    });
    let show = ctx.create_timeout(Duration::ZERO, {
      let shown = shown.clone();
      move || shown.set(true)
    });
    let picture = Picture {
      player: Arc::default(),
      // Not the player's own signal: a change of it renders the small window again, and not the player.
      changed: Signal::new(0),
      open: Arc::default(),
    };
    // After the render that changed the player: a signal is not set while something renders.
    let told = ctx.create_timeout(Duration::ZERO, {
      let changed = picture.changed.clone();
      move || changed.update(|count| *count += 1)
    });
    let mark = ctx.signal(None);
    let unmark = ctx.create_timeout(VIDEO_PLAYER_MARK_STAYS, {
      let mark = mark.clone();
      move || mark.set(None)
    });
    #[cfg(feature = "media")]
    let played = {
      let (beat, plays) = (ctx.signal(0_u64), Arc::new(AtomicBool::new(false)));
      // While it plays the time and the seek bar move: four times a second is what they show.
      let clock = ctx.create_interval(Duration::from_millis(250), {
        let (beat, plays) = (beat.clone(), plays.clone());
        move || {
          if plays.load(Ordering::Relaxed) {
            beat.update(|count| *count += 1);
          }
        }
      });
      clock.start();
      Played {
        media: Arc::default(),
        beat,
        plays,
        said: Arc::default(),
        listener: super::media_events::Listener::default(),
        near: Arc::default(),
        _clock: clock,
      }
    };
    let live = Live {
      root: ElementRef::new(),
      row: ElementRef::new(),
      buttons: ElementRef::new(),
      menus: [ctx.signal(false), ctx.signal(false), ctx.signal(false)],
      page: ctx.signal(None),
      pressed: Arc::default(),
      tapped: Arc::default(),
      mark,
      unmark: Some(unmark),
      shown,
      hide: Some(hide),
      show: Some(show),
      remaining: ctx.signal(false),
      volume: Some(super::media_volume::Kept::new(ctx)),
      bucket: ctx.signal(0),
      follows: Arc::new(AtomicBool::new(false)),
      aspect: Arc::new(AtomicU32::new(VIDEO_PLAYER_ASPECT.to_bits())),
      filling: Arc::default(),
      screen: Arc::default(),
      rested: Arc::default(),
      dropped: ctx.signal(0.0),
      taken: Arc::default(),
      window: Arc::default(),
      picture: Some(picture),
      told: Some(told),
      opener: Some(ctx.window_opener()),
      #[cfg(feature = "media")]
      played: Some(played),
      mounted: true,
    };
    // Before the frame is painted: the form that fits the width it was laid out at, so that no frame shows another.
    // Its height follows a width the app gave it at its aspect, in the same frame.
    let (bucket_of, last) = (live.bucket.clone(), Arc::new(AtomicU8::new(0)));
    let (root, follows, aspect) = (live.root.mutable(), live.follows.clone(), live.aspect.clone());
    let (filling, rested) = (live.filling.clone(), live.rested.clone());
    live.root.observe_rect_before_paint(move |rect| {
      let now = bucket(rect.width);
      if last.swap(now, Ordering::Relaxed) != now {
        bucket_of.set(now);
      }
      // Filling its window it is as large as the window, and has no shape of its own to keep.
      if filling.load(Ordering::Relaxed) {
        return;
      }
      let mut height = rect.height;
      if follows.load(Ordering::Relaxed) {
        let aspect = f32::from_bits(aspect.load(Ordering::Relaxed));
        height = shaped(rect.width, aspect);
        root.set_size(None, Some(Dimension::Px(height)));
      }
      *rested.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = (rect.width, height);
    });
    Self { live }
  }

  /// A player taken away while it fills the screen gives the window back.
  fn on_unmounted(&self) {
    if self.live.screen.swap(false, Ordering::Relaxed) {
      let taken = self
        .live
        .window
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take();
      if let Some(window) = taken {
        window.set_full_screen(false);
      }
    }
    // And its small window closes with it.
    if let Some(picture) = &self.live.picture
      && picture.open.swap(false, Ordering::Relaxed)
    {
      picture.put(None);
      picture.changed.update(|count| *count += 1);
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let player = ctx.props::<VideoPlayerProps>().0.clone();
    let live = &self.live;
    #[cfg(feature = "media")]
    let player = playing(player, live, ctx);
    // In fullscreen it fills its window, and takes the window to the whole screen, unless the app does it.
    let fills = player.fullscreen.value && player.fullscreen_fills != VideoFullscreen::App;
    live.filling.store(fills, Ordering::Relaxed);
    let screen = fills && player.fullscreen_fills == VideoFullscreen::Screen;
    if live.mounted && live.screen.swap(screen, Ordering::Relaxed) != screen {
      // The window it is rendered in now, asked for only when it changes.
      let window = ctx.window();
      window.set_full_screen(screen);
      *live.window.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = screen.then_some(window);
    }
    let follows = !fills && player.height.is_none() && (player.width.is_some() || player.max_width.is_some());
    live.follows.store(follows, Ordering::Relaxed);
    live.aspect.store(player.aspect.to_bits(), Ordering::Relaxed);
    let root = live.root.mutable();
    if !follows {
      root.set_size(None, None);
    } else if live.root.is_attached() {
      root.set_size(None, Some(Dimension::Px(shaped(live.root.width(), player.aspect))));
    }
    // Playing, the controls fade out once the pointer has rested; not playing, they come back.
    let plays = player.playing.value && !player.waiting && !player.ended && player.error.is_none();
    let shown = live.shown.get();
    if plays && shown {
      if let Some(hide) = &live.hide
        && !hide.is_active()
      {
        hide.start();
      }
    } else if !plays {
      if let Some(hide) = &live.hide {
        hide.cancel();
      }
      // The touch form's controls are the finger's to hide while it is paused too.
      if !shown
        && !player.is_touch()
        && let Some(show) = &live.show
      {
        show.start();
      }
    }
    let theme = current_theme();
    // In picture in picture it plays in a small window of its own, kept over the others, and says so where it
    // stood. A player that fills its window is not in one.
    let away = !fills && player.picture_in_picture.value && player.picture_in_picture_window;
    if let (Some(picture), Some(told)) = (&live.picture, &live.told) {
      if away {
        picture.put(Some(player.clone()));
        if !picture.open.swap(true, Ordering::Relaxed)
          && let Some(opener) = &live.opener
        {
          let (wide, props) = (VIDEO_PLAYER_PICTURE_WIDTH, PictureProps(picture.clone()));
          let high = shaped(wide as f32, player.aspect).max(1.0) as u32;
          opener.open_with(
            crate::app::WindowOptions::new(player.words.picture_in_picture.to_string(), wide, high)
              .undecorated()
              .always_on_top()
              .window_name("picture-in-picture"),
            move |app, tree| tree.mount_root::<PictureWindow>(app, props),
          );
        }
        told.restart();
      } else if picture.open.swap(false, Ordering::Relaxed) {
        picture.put(None);
        told.restart();
      }
    }
    if away {
      return view(in_its_window(player, &theme), &theme, live, Some(ctx)).render_themed();
    }
    if !fills {
      return view(player, &theme, live, Some(ctx)).render_themed();
    }
    // The player itself is in a layer over the whole window; where it stood a box keeps its room, so nothing
    // around it moves while it is away and it comes back to the same place.
    let rested = *live.rested.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let (width, height, most) = (player.width, player.height, player.max_width);
    // A finger that drags it down leaves fullscreen, as a picture that fills a phone's screen is closed: far
    // enough, or thrown. It follows the finger until then.
    let leave = {
      let (tell, poke) = (player.fullscreen.teller(), live.poke());
      move || {
        tell(false);
        poke();
      }
    };
    let (dropped, taken) = (live.dropped.clone(), live.taken.clone());
    let drop = live.dropped.get();
    // Over the whole of a phone's screen it keeps clear of what the system draws over its edges: the black goes
    // under them, the picture and the controls do not.
    let safe = ctx.safe_area();
    let mut inside = player;
    inside.width = Some(Length::Fill);
    inside.height = Some(Length::Fill);
    inside.max_width = None;
    let filled = view(inside, &theme, live, Some(ctx));
    let mut place = Stack::new().part(&filled.part(VideoPlayerPart::Root));
    if rested.0 > 0.0 {
      place = place.width(rested.0).height(rested.1);
    } else {
      // It was in fullscreen from its first frame: the size it was given, or its form's.
      if let Some(width) = width.and_then(|width| width.resolve(&theme)) {
        place = place.width(width);
      }
      if let Some(height) = height.and_then(|height| height.resolve(&theme)) {
        place = place.height(height);
      }
    }
    if let Some(most) = most.and_then(|most| most.resolve(&theme)) {
      place = place.max_width(most);
    }
    let full = Dimension::Pct(100.0);
    let layer = Modal::new(
      Stack::new()
        .size(full, full)
        .background(Color::new(0, 0, 0, 255))
        .padding_top(safe.top)
        .padding_bottom(safe.bottom)
        .padding_left(safe.left)
        .padding_right(safe.right)
        .on_pan(move |event: PanEvent| match event.phase {
          GesturePhase::Start => {
            let down = event.translation_y > 0.0 && event.translation_y.abs() > event.translation_x.abs();
            if down && event.fingers == 1 {
              event.prevent_default();
              taken.store(true, Ordering::Relaxed);
              dropped.set(event.translation_y);
            }
          }
          GesturePhase::Change if taken.load(Ordering::Relaxed) => dropped.set(event.translation_y.max(0.0)),
          GesturePhase::End if taken.swap(false, Ordering::Relaxed) => {
            dropped.set(0.0);
            if event.translation_y > VIDEO_PLAYER_DROP_LEAVES || event.velocity_y > VIDEO_PLAYER_THROW_LEAVES {
              leave();
            }
          }
          _ => {}
        })
        .child(
          Stack::new()
            .size(full, full)
            .offset(0.0, drop)
            .child(filled.render_themed()),
        ),
    )
    .target(Root)
    .takes_focus(true);
    place.child(layer).into()
  }
}

/// `player` as the media of its source has it: opened where it is not yet, its frames as the picture, its time,
/// its length and its state in place of the app's, and each control doing to the media what it tells the app.
/// A player with no source is given back as it is.
#[cfg(feature = "media")]
fn playing(mut player: VideoPlayer, live: &Live, ctx: &mut Ctx) -> VideoPlayer {
  use crate::media::Media;
  let Some(played) = &live.played else {
    return player;
  };
  let mut held = played.media.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
  let Some(source) = player.source.take() else {
    // No source any more: the media ends with what held it.
    *held = None;
    played.plays.store(false, Ordering::Relaxed);
    return player;
  };
  // What reads the tree is told what plays: the file, the address or how many bytes. Never its headers.
  #[cfg(any(feature = "devtools", feature = "mcp"))]
  {
    player.said_source = Some(Arc::from(format!("{source:?}")));
  }
  // Rendered again when the media says it changed, and as its clock goes.
  played.beat.get();
  let key = source.cache_key();
  if held.as_ref().is_none_or(|(of, _)| *of != key) {
    // The app's loader, with its resource root and what it allows; a tree with no app has a loader of its own.
    let loader = ctx
      .app_handle()
      .map_or_else(crate::resources::Loader::new, |app| app.loader().clone());
    let media = Media::open(&loader, &source);
    let beat = played.beat.clone();
    media.on_change(move || beat.update(|count| *count += 1));
    played.listener.listen(&media);
    *held = Some((key, media));
    *played.said.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = (None, None, None, None);
  }
  let media = held
    .as_ref()
    .map(|(_, media)| media.clone())
    .expect("the media just held");
  drop(held);
  // What the app says of playing and of the speed moves the media when it changes: an app that holds them
  // plays, pauses and changes the speed by setting them.
  {
    let mut said = played.said.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if said.0 != Some(player.playing.value) {
      // A player that starts paused has nothing to tell a media that opens paused, and an app that answers what
      // the media told it (it ended, it started over) tells it nothing.
      let answers = played.listener.answers(player.playing.value);
      if player.playing.value && !answers {
        media.play();
      } else if said.0.is_some() && !answers {
        media.pause();
      }
      // What the app set it is not told again when the media plays or stops.
      if said.0.is_some() {
        played.listener.known(player.playing.value);
      } else {
        played.listener.first(player.playing.value);
      }
      said.0 = Some(player.playing.value);
    }
    if said.1 != Some(player.rate.value) {
      media.set_rate(player.rate.value);
      said.1 = Some(player.rate.value);
    }
    if said.2 != Some(player.volume.value) {
      media.set_volume(player.volume.value as f32);
      said.2 = Some(player.volume.value);
    }
    if said.3 != Some(player.muted.value) {
      media.set_muted(player.muted.value);
      said.3 = Some(player.muted.value);
    }
    if media.looped() != player.looped {
      media.set_looped(player.looped);
    }
    if let Some(ahead) = player.ahead
      && media.ahead() != ahead
    {
      media.set_ahead(ahead);
    }
    if let Some(priority) = player.priority
      && media.priority() != priority
    {
      media.set_priority(priority);
    }
  }
  // What a person does on a control is done to the media, after the app's own handlers heard of it.
  let again = {
    let beat = played.beat.clone();
    move || beat.update(|count| *count += 1)
  };
  // The media's events go to the handlers of this render; whether it plays, to the app's own.
  played.listener.update(&player.events, &player.playing.handlers);
  // What the app last said stays what the app said: a player whose app holds nothing is paused in its props
  // for as long as it lives, and that is not the app pausing it after a person pressed play.
  player.playing.add({
    let (media, again, listener) = (media.clone(), again.clone(), played.listener.clone());
    move |to| {
      listener.known(to);
      if to {
        media.play();
      } else {
        media.pause();
      }
      again();
    }
  });
  player.position.add({
    let (media, again, near) = (media.clone(), again.clone(), played.near.clone());
    move |to| {
      if near.swap(false, Ordering::Relaxed) {
        media.seek_near(to);
      } else {
        media.seek(to);
      }
      again();
    }
  });
  player.rate.add({
    let (media, again) = (media.clone(), again.clone());
    move |to| {
      media.set_rate(to);
      again();
    }
  });
  player.volume.add({
    let (media, again) = (media.clone(), again.clone());
    move |to| {
      media.set_volume(to as f32);
      again();
    }
  });
  player.muted.add({
    let (media, again) = (media.clone(), again.clone());
    move |to| {
      media.set_muted(to);
      again();
    }
  });
  // Trying again opens the source anew.
  player.on_retry.push({
    let (held, again) = (played.media.clone(), again);
    Arc::new(move || {
      *held.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
      again();
    })
  });
  let state = media.state();
  played.plays.store(state.playing, Ordering::Relaxed);
  player.playing.value = state.playing;
  player.position.value = state.position;
  player.rate.value = state.rate;
  player.volume.value = f64::from(state.volume);
  player.muted.value = state.muted;
  if state.duration.is_some() {
    player.duration = state.duration;
  }
  // How far the media says it is fetched, unless the app says how far: all of a file, the ranges held of an
  // address, and nothing drawn as loaded where it is not known.
  player.buffered = player.buffered.or(state.buffered);
  player.waiting = player.waiting || state.waiting;
  player.ended = player.ended || state.ended;
  if let (None, Some(why)) = (&player.error, &state.error) {
    player.error = Some(player.words.unplayable.clone());
    // The reason, in the words of the locale.
    player.error_description = Some(match &player.locale {
      Some(words) => words.error(why),
      None => super::current_locale().media.error(why),
    });
  }
  // Its picture is there once its first frame is: until then its image is black, and is not drawn. What has
  // no picture (a sound) has nothing to wait for.
  let pictured = media.has_frame();
  player.framed = Some(pictured);
  player.opening = state.error.is_none() && (!media.is_open() || (state.size.is_some() && !pictured));
  if let Some(image) = media.frame().filter(|_| pictured) {
    player.media = Some(
      crate::components::Video::new(image)
        .contain()
        .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
        .into(),
    );
  }
  player
}

/// What a player says of what it plays to what reads the tree (the MCP server, DevTools): which of several
/// videos a screen is playing is then read from the tree, and not from logs of the app's own. `media_state`
/// (`opening`, `playing`, `paused`, `waiting`, `ended` or `failed`), `media_position` and `media_duration` in
/// seconds, and `media_source` where it plays a source. Nothing in a build with no tooling.
fn said_to_tooling(player: &VideoPlayer) -> Vec<(&'static str, Arc<str>)> {
  if !cfg!(any(feature = "devtools", feature = "mcp")) {
    return Vec::new();
  }
  let state = if player.error.is_some() {
    "failed"
  } else if player.opening {
    "opening"
  } else if player.ended {
    "ended"
  } else if player.waiting {
    "waiting"
  } else if player.playing.value {
    "playing"
  } else {
    "paused"
  };
  super::media_time::said_to_tooling(
    state,
    player.position.value,
    player.duration,
    player.said_source.as_ref(),
  )
}

/// The style of the parts of `player` at rest, in the form `size`.
fn parts(player: &VideoPlayer, theme: &Theme, size: VideoPlayerSize) -> ResolvedRecipe {
  let case = Case {
    variant: None,
    size: Some(size.into()),
    density: None,
    breakpoint: super::current_breakpoint(),
    states: player.states.clone(),
  };
  theme
    .resolve::<VideoPlayer>(&case, Some(&player.rules))
    .owned_by(player.id.as_ref())
}

/// `player` as it stands where it was while it plays in its small window: no picture and no controls, what it
/// says in the middle (the error's glyph, title and button, with other words), and a button that brings it back.
fn in_its_window(mut player: VideoPlayer, theme: &Theme) -> VideoPlayer {
  let recipe = parts(&player, theme, VideoPlayerSize::Lg);
  let (title, button) = (recipe.part(VideoPlayerPart::Title), recipe.part(VideoPlayerPart::Retry));
  let back = player.picture_in_picture.teller();
  let mut leave = pressable(&button, false)
    .describe("a11y_role", "button")
    .describe("a11y_name", player.words.back.clone())
    .child(button.text(&player.words.back).nowrap())
    .on_click(move |_| back(false));
  if let Some(id) = &player.id {
    leave = leave.id(format!("{id}-back"));
  }
  let said = Column::new()
    .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .spacing(12.0)
    .child(glyph(
      &recipe.part(VideoPlayerPart::ErrorIcon),
      Name::new(icons::PICTURE_IN_PICTURE),
      false,
    ))
    .child(
      title
        .text(&player.words.in_picture)
        .text_align(TextAlign::Center)
        .part(&title),
    )
    .child(leave);
  player.media = None;
  player.poster = None;
  player.controls = false;
  player.children = vec![said.into()];
  player
}

/// What the small window of a player in picture in picture is given.
#[derive(Clone)]
struct PictureProps(Picture);

/// The same window as long as it is the same player's.
impl PartialEq for PictureProps {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0.player, &other.0.player)
  }
}

impl crate::app::component::DevtoolsInspectable for PictureProps {}

/// The small window of a player in picture in picture: the player as large as the window, in its minimal form,
/// with a strip at its top that moves the window (it has no frame) and a button that brings the player back.
struct PictureWindow;

impl Component for PictureWindow {
  type Props = PictureProps;

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let picture = ctx.props::<PictureProps>().0.clone();
    // Rendered again each time the player is.
    picture.changed.get();
    let window = ctx.window();
    // Closed by the system, it asks the app to leave picture in picture as its button does. Said at each render,
    // of the window it is rendered in: the root of a second window is created before that window is its own.
    window.on_close_requested({
      let picture = picture.clone();
      move |request| {
        if let Some(player) = picture.held() {
          (player.picture_in_picture.teller())(false);
        }
        request.proceed();
      }
    });
    let black = Color::new(0, 0, 0, 255);
    let Some(mut player) = picture.held() else {
      // It left picture in picture, or was taken away.
      window.close();
      let empty: Element = Stack::new()
        .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
        .background(black)
        .into();
      return empty;
    };
    let theme = current_theme();
    let recipe = parts(&player, &theme, VideoPlayerSize::Md);
    let leave = player.picture_in_picture.teller();
    let back = pressable(&recipe.part(VideoPlayerPart::Control), false)
      .describe("a11y_role", "button")
      .describe("a11y_name", player.words.back.clone())
      .child(glyph(
        &recipe.part(VideoPlayerPart::ControlIcon),
        Name::new(icons::PICTURE_IN_PICTURE),
        false,
      ))
      .on_click(move |_| leave(false));
    let grip = Row::new()
      .flex(1.0)
      .height(Dimension::Pct(100.0))
      .on_mouse_down(move |_: MouseEvent| window.start_drag());
    let top = Row::new()
      .width(Dimension::Pct(100.0))
      .height(36.0)
      .padding(4.0)
      .align_items(Alignment::Center)
      .child(grip)
      .child(back);
    player.children.push(
      Column::new()
        .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
        .hit_test(HitTestBehavior::ContentOnly)
        .child(top)
        .into(),
    );
    player.width = Some(Length::Fill);
    player.height = Some(Length::Fill);
    player.max_width = None;
    player.size = Some(VideoPlayerSize::Sm.into());
    // The window is the picture: its corners are the window's.
    player = player.style(VideoPlayerPart::Root, |s| s.radius(0.0));
    // It is the picture in picture: it opens no window of its own, and its fullscreen is the app's.
    player.picture_in_picture_window = false;
    player.fullscreen_fills = VideoFullscreen::App;
    #[cfg(feature = "media")]
    {
      player.source = None;
    }
    player.id = None;
    player.key = None;
    Stack::new()
      .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
      .background(black)
      .child(player.mount(ctx))
      .into()
  }
}

/// Builds the view of `player` in `theme`, with its controls made, and their tooltips where `ctx` is given.
fn view(mut player: VideoPlayer, theme: &Theme, live: &Live, ctx: Option<&mut Ctx>) -> VideoPlayerView {
  // Which form it is, asked of the person's pointer once.
  player.touch = Some(player.is_touch());
  // Not measured yet (the still form, and a mounted player's first render): a width given in pixels decides
  // what the measure would, so that a player told to be 432 wide folds its volume without being mounted.
  let measured = match live.bucket.get() {
    0 => match player.width.and_then(|width| width.resolve(theme)) {
      Some(Dimension::Px(width)) => bucket(width),
      _ => 0,
    },
    measured => measured,
  };
  let form = match player.size {
    Some(size) => VideoPlayerSize::of(size),
    None => match measured {
      1 => VideoPlayerSize::Sm,
      2 => VideoPlayerSize::Md,
      _ => VideoPlayerSize::Lg,
    },
  };
  let narrow = matches!(measured, 1..=3);
  let poster = player.shows_poster();
  let menu_open = player.force_menu.is_some() || live.menus.iter().any(|open| open.get());
  let resting = player.playing.value && !player.waiting && !player.ended && player.error.is_none();
  let shown = player.force_controls_shown.unwrap_or_else(|| {
    // A tap shows and hides the touch form's controls whether it plays or not.
    (!resting && !player.is_touch())
      || live.shown.get()
      || menu_open
      || (live.mounted && live.root.focus_visible_within())
  });
  let has_bar = player.controls && player.error.is_none() && !poster;
  let landscape = player.is_touch() && player.fullscreen.value;
  let case = Case {
    variant: None,
    size: Some(form.into()),
    density: None,
    breakpoint: super::current_breakpoint(),
    states: player.states.clone(),
  }
  .state(VideoPlayerState::Playing, player.playing.value)
  .state(VideoPlayerState::Waiting, player.waiting)
  .state(VideoPlayerState::Ended, player.ended)
  .state(VideoPlayerState::Error, player.error.is_some())
  .state(VideoPlayerState::Live, player.live)
  .state(VideoPlayerState::Muted, player.muted.value)
  .state(VideoPlayerState::Captions, player.captions.value)
  .state(VideoPlayerState::Fullscreen, player.fullscreen.value)
  .state(VideoPlayerState::Poster, poster)
  .state(VideoPlayerState::Bare, !player.controls)
  .state(VideoPlayerState::Hidden, has_bar && !shown && !player.is_touch())
  .state(VideoPlayerState::Touch, player.is_touch())
  .state(VideoPlayerState::Landscape, landscape)
  .state(VideoPlayerState::Narrow, narrow);
  let recipe = theme
    .resolve::<VideoPlayer>(&case, Some(&player.rules))
    .owned_by(player.id.as_ref());
  let on_recipe = theme
    .resolve::<VideoPlayer>(&case.clone().state(VideoPlayerState::On, true), Some(&player.rules))
    .owned_like(&recipe);
  let mut make = Make {
    player: &player,
    recipe: &recipe,
    on_recipe: &on_recipe,
    live,
    ctx,
    form,
    shown,
  };
  let touch = (player.is_touch() && player.controls).then(|| make.touch(landscape));
  let bar = (has_bar && !player.is_touch()).then(|| make.bar(narrow));
  // The touch form has its transport in the middle: only the error and the spinner stand there.
  let center = (player.controls && (!player.is_touch() || player.error.is_some() || player.waiting))
    .then(|| make.center())
    .flatten();
  let tap = (player.is_touch() && player.controls && player.error.is_none()).then(|| make.tap());
  let retry = make.retry();
  let root_keys: Arc<dyn Fn(KeyboardEvent) + Send + Sync> = Arc::new(make.keys());
  // The app's layers that follow what it is doing, over its other children.
  let mut children = player.children.clone();
  if !player.layers.is_empty() {
    let playback = VideoPlayback {
      playing: player.playing.value,
      waiting: player.waiting,
      ended: player.ended,
      failed: player.error.is_some(),
      opening: player.opening,
      poster_shown: poster,
      position: if player.ended {
        player.duration.unwrap_or(player.position.value)
      } else {
        player.position.value
      },
      duration: player.duration,
      buffered: player.buffered,
      volume: player.volume.value,
      muted: player.muted.value,
      rate: player.rate.value,
      seek: Arc::new(make.seeker()),
      seek_near: Arc::new(make.near_seeker()),
      play: Arc::new(player.playing.teller()),
      mute: Arc::new(player.muted.teller()),
    };
    children.extend(player.layers.iter().map(|layer| {
      let mut build = layer.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
      build(&playback)
    }));
  }
  VideoPlayerView {
    form,
    narrow,
    controls: player.controls,
    bar_shown: shown,
    poster_shown: poster,
    playing: player.playing.value,
    waiting: player.waiting,
    ended: player.ended,
    live: player.live,
    duration: player.duration,
    error: player.error.clone(),
    error_description: player.error_description.clone(),
    caption: (player.controls && player.captions.value && player.error.is_none())
      .then(|| {
        player.caption_text.clone().or_else(|| {
          player
            .caption_cues
            .as_ref()
            .and_then(|track| track.at(player.position.value))
        })
      })
      .flatten(),
    label: player.words.label.clone(),
    width: player.width,
    height: player.height,
    max_width: player.max_width,
    aspect: player.aspect,
    // Under the poster nothing of the picture is drawn: a poster that does not cover the box would show it
    // at its sides.
    media: player.media.clone().filter(|_| !poster),
    poster: player.poster.clone().filter(|_| poster),
    children,
    bar,
    center,
    retry,
    keys: root_keys,
    touch,
    tap,
    toggle: make.toggle(),
    pressed: live.pressed.clone(),
    fullscreen: make.fullscreen_toggle(),
    // A finger does not hover: the touch form's controls are shown by a tap, not by the pointer moving.
    poke: (live.mounted && !player.is_touch()).then(|| Arc::new(live.poke()) as Action),
    root: live.root.clone(),
    tooling: said_to_tooling(&player),
    // A menu drawn open by force hangs from its anchor in the still form too: a picture of it, not dragged.
    mounted: live.mounted || player.force_menu.is_some(),
    filling: live.filling.load(Ordering::Relaxed),
    recipe,
    id: player.id.clone(),
  }
}

/// What builds the controls of a player, with their tooltips where a context is given.
struct Make<'a, 'c> {
  player: &'a VideoPlayer,
  recipe: &'a ResolvedRecipe,
  on_recipe: &'a ResolvedRecipe,
  live: &'a Live,
  ctx: Option<&'c mut Ctx>,
  form: VideoPlayerSize,
  /// Whether the controls show, or have faded out while it plays.
  shown: bool,
}

/// The controls of the touch form, made: a renderer lays them out.
pub struct VideoPlayerTouch {
  /// Whether it is the form for a phone on its side: in fullscreen.
  pub landscape: bool,
  /// The first button of the top row, which leaves the player: none upright where the app does not listen.
  pub lead: Option<Element>,
  /// The title and its line, on its side.
  pub heading: Option<Element>,
  /// The buttons at the end of the top row: the captions and the settings.
  pub tools: Vec<Element>,
  /// The middle: ten seconds back, play, ten seconds on. None while it waits or failed.
  pub transport: Option<Element>,
  /// The time, or the live badge.
  pub time: Element,
  /// The buttons at the end of the bottom row: the speed (on its side) and fullscreen.
  pub foot: Vec<Element>,
  /// The seek bar.
  pub seek: Element,
  /// The menus, which take no room.
  pub menus: Vec<Element>,
  /// The mark of a double tap, while it stays.
  pub mark: Option<Element>,
  /// The top row, which the settings hang from.
  pub top: ElementRef,
  /// The bottom row, which the speeds hang from.
  pub row: ElementRef,
}

/// The controls of a player's bar, made: a renderer lays them out.
pub struct VideoPlayerBar {
  /// The seek bar: over the row, or in it in the minimal form.
  pub seek: Element,
  /// The play button.
  pub play: Element,
  /// The volume (large) or the mute button (compact); none in the minimal form.
  pub volume: Option<Element>,
  /// The time, or the live badge.
  pub time: Element,
  /// Whether the folded volume's slider is out: it stands in the time's room, and the time is not drawn.
  pub volume_out: bool,
  /// The buttons after the spacer, in order: captions, speed, settings, picture in picture, fullscreen (large), or
  /// mute, more, fullscreen (compact).
  pub end: Vec<Element>,
  /// The menus, which take no room: declared in the bar, they hang from its row.
  pub menus: Vec<Element>,
  /// The row of buttons, which the menus hang from.
  pub row: ElementRef,
  /// What holds the buttons.
  pub buttons: ElementRef,
  /// How far the captions' line moves down while the bar is hidden: the bar's height.
  pub height: f32,
}

impl Make<'_, '_> {
  fn part(&self, part: VideoPlayerPart) -> ResolvedPart {
    self.recipe.part(part).forced(self.player.forced)
  }

  fn id(&self, suffix: &str) -> Option<String> {
    self.player.id.as_ref().map(|id| format!("{id}-{suffix}"))
  }

  /// `element` with a tooltip of `text` and `keys` over it, where the player is mounted. The tooltip goes with
  /// the controls when they fade out: the pointer that rested on a button rests there still, and a tooltip over
  /// a button that is not seen says a thing of nothing. It hears the pointer all the while, and is back with the
  /// controls where the pointer is on its button still.
  fn tip(&mut self, element: Element, text: &str, keys: &str) -> Element {
    let faded = (!self.shown).then_some(false);
    match self.ctx.as_deref_mut() {
      Some(ctx) => Tooltip::new(text)
        .shortcut(keys)
        .placement(Placement::Top)
        .arrow(true)
        .offset(4.0)
        .force_open(faded)
        .mount(ctx, element),
      None => element,
    }
  }

  /// What tells the app a press, and shows the controls again.
  fn pressed<T: Copy + Send + Sync + 'static>(&self, tell: impl Fn(T) + Send + Sync + 'static, value: T) -> Action {
    let poke = self.live.mounted.then(|| self.live.poke());
    Arc::new(move || {
      tell(value);
      if let Some(poke) = &poke {
        poke();
      }
    })
  }

  /// A button of the bar: its glyph, its name, and on where `on` is.
  fn control(&mut self, suffix: &str, icon: &'static str, label: &str, keys: &str, on: bool, press: Action) -> Element {
    let recipe = if on { self.on_recipe } else { self.recipe };
    let part = recipe.part(VideoPlayerPart::Control).forced(self.player.forced);
    let mut button = pressable(&part, false)
      .describe("a11y_role", "button")
      .describe("a11y_name", Arc::<str>::from(label))
      .child(glyph(
        &recipe.part(VideoPlayerPart::ControlIcon),
        Name::new(icon),
        false,
      ))
      .on_click(move |_| press());
    if let Some(id) = self.id(suffix) {
      button = button.id(id);
    }
    self.tip(button.into(), label, keys)
  }

  /// A button of the bar that turns something on and off, and says whether it is on.
  fn toggle_control(
    &mut self,
    suffix: &str,
    icon: &'static str,
    label: &str,
    keys: &str,
    on: bool,
    press: Action,
  ) -> Element {
    let mut element = self.control(suffix, icon, label, keys, on, press);
    crate::node::NodeUpdate::describe(
      &mut *element.node,
      Arc::from("a11y_pressed"),
      Arc::from(if on { "true" } else { "false" }),
    );
    element
  }

  /// What a press of play does: tells the app it is to play, or to pause.
  fn toggle(&self) -> Action {
    let next = self.player.ended || !self.player.playing.value;
    self.pressed(self.player.playing.teller(), next)
  }

  fn fullscreen_toggle(&self) -> Action {
    self.pressed(self.player.fullscreen.teller(), !self.player.fullscreen.value)
  }

  /// The play button of the bar.
  fn play(&mut self) -> Element {
    let player = self.player;
    let (icon, label) = if player.ended {
      (icons::ROTATE_LEFT, player.words.replay.clone())
    } else if player.playing.value {
      (icons::PAUSE, player.words.pause.clone())
    } else {
      (icons::PLAY, player.words.play.clone())
    };
    let press = self.toggle();
    self.control("play", icon, &label, "K", false, press)
  }

  /// Seeking to `to` seconds, kept within what it plays: the position's handlers are told, then where it ends.
  fn seeker(&self) -> impl Fn(f64) + Send + Sync + Clone + 'static {
    let (tell, ends) = (
      self.player.position.teller(),
      self.player.on_position_change_end.clone(),
    );
    let end = self.player.duration.unwrap_or(f64::INFINITY);
    move |to: f64| {
      let to = to.clamp(0.0, end);
      tell(to);
      for handler in &ends {
        handler(to);
      }
    }
  }

  /// A seeker whose seek goes near its time, where the player has a source; elsewhere it is [`Self::seeker`].
  fn near_seeker(&self) -> impl Fn(f64) + Send + Sync + Clone + 'static {
    let seek = self.seeker();
    #[cfg(feature = "media")]
    let near = self.live.played.as_ref().map(|played| played.near.clone());
    move |to: f64| {
      #[cfg(feature = "media")]
      if let Some(near) = &near {
        near.store(true, Ordering::Relaxed);
      }
      seek(to);
      #[cfg(feature = "media")]
      if let Some(near) = &near {
        near.store(false, Ordering::Relaxed);
      }
    }
  }

  fn seek(&mut self) -> Element {
    let player = self.player;
    let (position, duration) = match (player.live, player.duration) {
      // Live with nothing to go back in: the bar full, at the edge.
      (true, None) => (1.0, 1.0),
      (_, duration) => {
        let duration = duration.unwrap_or(0.0);
        let position = if player.ended { duration } else { player.position.value };
        (position.min(duration), duration)
      }
    };
    // A live stream with nothing to go back in has nowhere to seek to: its bar tells nothing.
    let seekable = !player.live || player.duration.is_some();
    let tell = player.position.teller();
    let ends = player.on_position_change_end.clone();
    let poke = self.live.mounted.then(|| self.live.poke());
    let mut bar = SeekBar::controlled(position, duration, move |to| {
      if !seekable {
        return;
      }
      tell(to);
      if let Some(poke) = &poke {
        poke();
      }
    })
    .on_change_end(move |to| {
      if !seekable {
        return;
      }
      for handler in &ends {
        handler(to);
      }
    })
    .on_media(true)
    // The seek bar's own form for it: full, the thumb at its end, no stop of Tab.
    .live(!seekable)
    .label(&player.words.seek)
    .step(VIDEO_PLAYER_SEEK_STEP)
    .chapters(&player.chapters);
    if let Some(buffered) = player.buffered {
      bar = bar.buffered(buffered);
    }
    if let Some(preview) = player.preview.clone() {
      bar = bar.preview(move |at| preview(at));
    }
    if let Some(id) = self.id("seek") {
      bar = bar.id(id);
    }
    // A finger's: 28 high, the thumb 16.
    if player.is_touch() {
      bar = bar.size(super::SeekBarSize::Lg);
    }
    // Mounted, the bar shows the time and the frame under the pointer.
    match self.ctx.as_deref_mut() {
      Some(ctx) => bar.mount(ctx),
      None => bar.build(self.recipe.theme()),
    }
  }

  /// The time, the live badge, or in the minimal form how far it is alone.
  fn time(&mut self) -> Element {
    let player = self.player;
    let part = self.part(VideoPlayerPart::Time);
    let shown: Element = if player.live {
      let mut time = MediaTime::new(player.position.value, None)
        .live(true)
        .live_text(&player.words.live)
        .on_media(true);
      if let Some(id) = self.id("time") {
        time = time.id(id);
      }
      time.build(self.recipe.theme())
    } else if self.form == VideoPlayerSize::Sm && !player.is_touch() {
      let elapsed = self.part(VideoPlayerPart::Elapsed);
      let hours = player.duration.is_some_and(|duration| duration >= 3600.0);
      Row::new()
        .part(&elapsed)
        .child(elapsed.text(&media_time_text(player.position.value, hours)).nowrap())
        .into()
    } else {
      let remaining = self.live.remaining.clone();
      // Mono 12 in the large bar, 11 in the compact one; the touch form's is 12, and 13 on its side.
      let size = match (player.is_touch(), player.fullscreen.value, self.form) {
        (true, true, _) => MediaTimeSize::Lg,
        (true, false, _) => MediaTimeSize::Md,
        (false, _, VideoPlayerSize::Md) => MediaTimeSize::Sm,
        (false, ..) => MediaTimeSize::Md,
      };
      let mut time = MediaTime::new(player.position.value, player.duration)
        .remaining(remaining.get())
        .size(size)
        .on_media(true)
        .on_click(move |_| remaining.update(|remaining| *remaining = !*remaining));
      if let Some(id) = self.id("time") {
        time = time.id(id);
      }
      time.build(self.recipe.theme())
    };
    // The time in it has the part's id.
    Row::new()
      .part(&part.unnamed())
      .align_items(Alignment::Center)
      .child(shown)
      .into()
  }

  /// The volume of the large form: its button and its slider, folded where the bar is narrow.
  fn volume(&mut self, narrow: bool) -> Element {
    let player = self.player;
    let (tell, poke) = (player.volume.teller(), self.live.mounted.then(|| self.live.poke()));
    let ends = player.on_volume_change_end.clone();
    let mut volume = MediaVolume::controlled(player.volume.value, move |to| {
      tell(to);
      if let Some(poke) = &poke {
        poke();
      }
    })
    .muted_controlled(player.muted.value, player.muted.teller())
    .on_change_end(move |to| {
      for handler in &ends {
        handler(to);
      }
    })
    .on_media(true)
    .size(MediaVolumeSize::Lg)
    .length(64.0)
    .collapsed(narrow)
    .label(&player.words.volume)
    .mute_label(&player.words.mute)
    .unmute_label(&player.words.unmute)
    .force(player.forced);
    if let Some(id) = self.id("mute") {
      volume = volume.id(id);
    }
    // The player keeps what the control needs between renders: folded, its slider comes out under the pointer.
    if let Some(kept) = &self.live.volume {
      volume = volume.kept(kept.clone());
    }
    let text = if player.muted.value {
      player.words.unmute.clone()
    } else {
      player.words.mute.clone()
    };
    // The tooltip is the button's: over the slider it would say "Mute" of what sets the volume.
    let recipe = self.recipe;
    let faded = (!self.shown).then_some(false);
    match self.ctx.as_deref_mut() {
      Some(ctx) => volume.build_with(recipe.theme(), &mut |button| {
        Tooltip::new(&text)
          .shortcut("M")
          .placement(Placement::Top)
          .arrow(true)
          .offset(4.0)
          .force_open(faded)
          .mount(ctx, button)
      }),
      None => volume.build(recipe.theme()),
    }
  }

  /// The mute button of the compact form, alone.
  fn mute(&mut self) -> Element {
    let player = self.player;
    let level = player.volume.value;
    let icon = if player.muted.value || level <= 0.0 {
      icons::VOLUME_OFF
    } else if level < 0.5 {
      icons::VOLUME_LOW
    } else {
      icons::VOLUME
    };
    let label = if player.muted.value {
      player.words.unmute.clone()
    } else {
      player.words.mute.clone()
    };
    let press = self.pressed(player.muted.teller(), !player.muted.value);
    self.toggle_control("mute", icon, &label, "M", false, press)
  }

  /// A button that opens one of the bar's menus.
  fn menu_button(&mut self, menu: VideoPlayerMenu, suffix: &str, icon: Option<&'static str>, label: &str) -> Element {
    let open = self.live.menu(menu).clone();
    let is_open = self.player.force_menu == Some(menu) || open.get();
    let poke = self.live.mounted.then(|| self.live.poke());
    let press: Action = Arc::new(move || {
      open.update(|open| *open = !*open);
      if let Some(poke) = &poke {
        poke();
      }
    });
    let mut element = match icon {
      Some(icon) => self.control(suffix, icon, label, "", is_open, press),
      None => {
        // The speed is a button of text: "1×".
        let recipe = if is_open { self.on_recipe } else { self.recipe };
        let part = recipe.part(VideoPlayerPart::Speed).forced(self.player.forced);
        let words = format!("{}×", self.player.rate.value);
        let mut button = pressable(&part, false)
          .describe("a11y_role", "button")
          .describe("a11y_name", Arc::<str>::from(label))
          .child(part.text(&words).nowrap())
          .on_click(move |_| press());
        if let Some(id) = self.id(suffix) {
          button = button.id(id);
        }
        self.tip(button.into(), label, "")
      }
    };
    let node = &mut *element.node;
    crate::node::NodeUpdate::describe(node, Arc::from("a11y_haspopup"), Arc::from("menu"));
    crate::node::NodeUpdate::describe(
      node,
      Arc::from("a11y_expanded"),
      Arc::from(if is_open { "true" } else { "false" }),
    );
    element
  }

  /// What a menu of the bar is given: open by its signal, or held open where the app forces it.
  fn menu_shell(&self, menu: VideoPlayerMenu, panel: VideoPlayerPart, label: &str) -> Menu {
    let suffix = match menu {
      VideoPlayerMenu::Settings => "settings-menu",
      VideoPlayerMenu::Speed => "speed-menu",
      VideoPlayerMenu::More => "more-menu",
    };
    // The touch form's settings are at its top, and open under it.
    let (anchor, placement) = if self.player.is_touch() {
      (self.live.buttons.clone(), Placement::BottomEnd)
    } else {
      (self.live.row.clone(), Placement::TopEnd)
    };
    let shell = if self.player.force_menu == Some(menu) {
      Menu::controlled(true, anchor, |_| {})
    } else {
      Menu::new(self.live.menu(menu).clone(), anchor)
    };
    let above = self.above();
    let shell = match self.id(suffix) {
      Some(id) => shell.id(id),
      None => shell,
    };
    shell
      .label(label)
      .placement(placement)
      .offset(above)
      .page(self.live.page.clone())
      .rules(&self.recipe.rules_for(panel, MenuPart::Panel))
  }

  /// How far a menu stands over the bar's row.
  fn above(&self) -> f32 {
    let theme = self.recipe.theme();
    self
      .recipe
      .part(VideoPlayerPart::Menus)
      .rest()
      .gap
      .map(|gap| gap.resolve(&theme.spacing(), 0.0))
      .unwrap_or(4.0)
  }

  /// An item of a menu with a value at its end, whose choices it shows in the menu's place.
  fn page(&self, icon: &'static str, label: &str, value: &str, choices: Vec<MenuItem>) -> MenuItem {
    MenuItem::new(label)
      .icon(icon)
      .shortcut(value)
      .rules(
        &self
          .recipe
          .rules_for(VideoPlayerPart::MenuValue, MenuItemPart::Shortcut),
      )
      .page(choices)
  }

  fn quality_page(&self) -> Option<MenuItem> {
    let player = self.player;
    if player.qualities.is_empty() {
      return None;
    }
    let chosen = player.quality.value;
    let value = chosen
      .and_then(|index| player.qualities.get(index).cloned())
      .unwrap_or_else(|| player.words.auto.clone());
    let tell = player.quality.teller();
    let choices = std::iter::once((None, player.words.auto.clone()))
      .chain(
        player
          .qualities
          .iter()
          .enumerate()
          .map(|(index, name)| (Some(index), name.clone())),
      )
      .map(|(quality, name)| {
        let tell = tell.clone();
        let is = quality == chosen;
        MenuItem::new(&name).checked(is).on_select(move || {
          if !is {
            tell(quality);
          }
        })
      })
      .collect();
    Some(self.page(icons::SLIDERS, &player.words.quality, &value, choices))
  }

  fn speed_items(&self) -> Vec<MenuItem> {
    let player = self.player;
    let tell = player.rate.teller();
    player
      .rates
      .iter()
      .map(|offered| {
        let (offered, tell) = (f64::from(*offered), tell.clone());
        let is = (offered - player.rate.value).abs() < 1e-6;
        MenuItem::new(&media_rate_text(offered as f32))
          .checked(is)
          .on_select(move || {
            if !is {
              tell(offered);
            }
          })
      })
      .collect()
  }

  fn speed_page(&self) -> Option<MenuItem> {
    let player = self.player;
    (!player.live).then(|| {
      let value = media_rate_text(player.rate.value as f32);
      self.page(icons::GAUGE, &player.words.speed, &value, self.speed_items())
    })
  }

  /// The captions' row of a menu, where the player has captions.
  fn captions_row(&self) -> Option<MenuItem> {
    self.player.has_captions().then(|| self.captions_page())
  }

  /// Whether the settings' button is drawn: where it is not left out and its menu has something in it.
  fn has_settings(&self) -> bool {
    let player = self.player;
    player.settings_button && (!player.qualities.is_empty() || !player.live || player.has_captions())
  }

  fn captions_page(&self) -> MenuItem {
    let player = self.player;
    let track = player.captions_track.clone().unwrap_or_else(|| player.words.on.clone());
    let on = player.captions.value;
    let tell = player.captions.teller();
    let choices = [(false, player.words.off.clone()), (true, track.clone())]
      .into_iter()
      .map(|(choice, name)| {
        let tell = tell.clone();
        MenuItem::new(&name).checked(choice == on).on_select(move || {
          if choice != on {
            tell(choice);
          }
        })
      })
      .collect();
    let value = if on { track } else { player.words.off.clone() };
    self.page(icons::CAPTIONS, &player.words.captions, &value, choices)
  }

  /// The large form's settings: quality, speed, captions.
  fn settings_menu(&self) -> Element {
    let player = self.player;
    let items: Vec<MenuItem> = [self.quality_page(), self.speed_page(), self.captions_row()]
      .into_iter()
      .flatten()
      .collect();
    self
      .menu_shell(
        VideoPlayerMenu::Settings,
        VideoPlayerPart::SettingsMenu,
        &player.words.settings,
      )
      .items(items)
      .build(self.recipe.theme())
  }

  /// The large form's speeds: the shared speed menu, hung over the bar as the settings are.
  fn speed_menu(&self) -> Element {
    let player = self.player;
    let open = if player.force_menu == Some(VideoPlayerMenu::Speed) {
      Signal::new(true)
    } else {
      self.live.menu(VideoPlayerMenu::Speed).clone()
    };
    let mut menu = MediaSpeedMenu::new(open, self.live.row.clone(), player.rate.value as f32)
      .rates(&player.rates)
      .heading_text(&player.words.speed_heading)
      .label(&player.words.speed)
      .placement(Placement::TopEnd)
      .offset(self.above())
      .on_change(player.rate.teller());
    if let Some(id) = self.id("speed-menu") {
      menu = menu.id(id);
    }
    menu.build(self.recipe.theme())
  }

  /// The compact form's menu: captions, speed, quality, picture in picture.
  fn more_menu(&self) -> Element {
    let player = self.player;
    let pip = player.picture_in_picture.value;
    let tell = player.picture_in_picture.teller();
    let picture = MenuItem::new(&player.words.picture_in_picture)
      .icon(icons::PICTURE_IN_PICTURE)
      .checked(pip)
      .on_select(move || tell(!pip));
    let items: Vec<MenuItem> = [
      self.captions_row(),
      self.speed_page(),
      self.quality_page(),
      Some(picture),
    ]
    .into_iter()
    .flatten()
    .collect();
    self
      .menu_shell(VideoPlayerMenu::More, VideoPlayerPart::MoreMenu, &player.words.more)
      .items(items)
      .build(self.recipe.theme())
  }

  /// A round button of the touch form: its glyph and its name, on where `on` is.
  fn touch_button(
    &mut self,
    suffix: &str,
    (button, glyph_of): (VideoPlayerPart, VideoPlayerPart),
    icon: &'static str,
    label: &str,
    on: bool,
    press: Action,
  ) -> Element {
    let recipe = if on { self.on_recipe } else { self.recipe };
    let part = recipe.part(button).forced(self.player.forced);
    let mut made = pressable(&part, false)
      .describe("a11y_role", "button")
      .describe("a11y_name", Arc::<str>::from(label))
      .child(glyph(&recipe.part(glyph_of), Name::new(icon), false))
      .on_click(move |_| press());
    if let Some(id) = self.id(suffix) {
      made = made.id(id);
    }
    made.into()
  }

  /// The controls of the touch form.
  fn touch(&mut self, landscape: bool) -> VideoPlayerTouch {
    use VideoPlayerPart::{Disc, DiscIcon, Skip, SkipIcon, Touch, TouchIcon};
    let player = self.player;
    let seek = self.seek();
    let time = self.time();
    let poke = self.live.mounted.then(|| self.live.poke());
    // What leaves the player: the app's, or out of fullscreen where it is on its side and the app says nothing.
    let lead = (landscape || !player.on_back.is_empty()).then(|| {
      let (handlers, exit) = (player.on_back.clone(), self.fullscreen_toggle());
      let press: Action = Arc::new(move || {
        if handlers.is_empty() {
          exit();
        }
        for handler in &handlers {
          handler();
        }
      });
      let (icon, label) = if landscape {
        (icons::CHEVRON_LEFT, player.words.leave.clone())
      } else {
        (icons::CHEVRON_DOWN, player.words.minimize.clone())
      };
      self.touch_button("back", (Touch, TouchIcon), icon, &label, false, press)
    });
    let heading = (landscape && player.title.is_some()).then(|| {
      let (title, line) = (self.part(VideoPlayerPart::Title), self.part(VideoPlayerPart::Line));
      let mut words = Column::new().part(&self.part(VideoPlayerPart::Heading));
      if let Some(said) = &player.title {
        words = words.child(title.text(said).nowrap().part(&title));
      }
      if let Some(said) = &player.description {
        words = words.child(line.text(said).nowrap().part(&line));
      }
      words.into()
    });
    let mut tools = Vec::new();
    if player.has_captions() {
      let captions_on = player.captions.value;
      let press = self.pressed(player.captions.teller(), !captions_on);
      tools.push(self.touch_button(
        "captions",
        (Touch, TouchIcon),
        icons::CAPTIONS,
        &player.words.captions,
        captions_on,
        press,
      ));
    }
    let mut menus = Vec::new();
    if self.has_settings() {
      let open = self.live.menu(VideoPlayerMenu::Settings).clone();
      let is_open = player.force_menu == Some(VideoPlayerMenu::Settings) || open.get();
      let poke = poke.clone();
      let press: Action = Arc::new(move || {
        open.update(|open| *open = !*open);
        if let Some(poke) = &poke {
          poke();
        }
      });
      tools.push(self.touch_button(
        "settings",
        (Touch, TouchIcon),
        icons::SETTINGS,
        &player.words.settings,
        is_open,
        press,
      ));
      menus.push(self.settings_menu());
    }
    // The middle: none while it waits or failed, where the spinner or the error stands.
    let transport = (player.error.is_none() && !player.waiting).then(|| {
      let seekable = !player.live || player.duration.is_some();
      let step = |by: f64| -> Action {
        let (seek, position, poke) = (self.seeker(), player.position.value, poke.clone());
        Arc::new(move || {
          if seekable {
            seek(position + by);
          }
          if let Some(poke) = &poke {
            poke();
          }
        })
      };
      let (back, on) = (step(-VIDEO_PLAYER_TAP_SEEKS), step(VIDEO_PLAYER_TAP_SEEKS));
      let rewind = self.touch_button(
        "rewind",
        (Skip, SkipIcon),
        icons::REWIND_10,
        &player.words.rewind,
        false,
        back,
      );
      let forward = self.touch_button(
        "forward",
        (Skip, SkipIcon),
        icons::FORWARD_10,
        &player.words.forward,
        false,
        on,
      );
      let (icon, label) = if player.ended {
        (icons::ROTATE_LEFT, player.words.replay.clone())
      } else if player.playing.value {
        (icons::PAUSE, player.words.pause.clone())
      } else {
        (icons::PLAY, player.words.play.clone())
      };
      let press = self.toggle();
      let disc = self.touch_button("play", (Disc, DiscIcon), icon, &label, false, press);
      Row::new()
        .part(&self.part(VideoPlayerPart::Transport))
        .align_items(Alignment::Center)
        .child(rewind)
        .child(disc)
        .child(forward)
        .into()
    });
    let mut foot = Vec::new();
    if landscape && !player.live {
      let open = self.live.menu(VideoPlayerMenu::Speed).clone();
      let is_open = player.force_menu == Some(VideoPlayerMenu::Speed) || open.get();
      let recipe = if is_open { self.on_recipe } else { self.recipe };
      let part = recipe.part(VideoPlayerPart::Pill).forced(player.forced);
      let poke = poke.clone();
      let mut pill = pressable(&part, false)
        .describe("a11y_role", "button")
        .describe("a11y_name", player.words.speed.clone())
        .child(part.text(&format!("{}×", player.rate.value)).nowrap())
        .on_click(move |_| {
          open.update(|open| *open = !*open);
          if let Some(poke) = &poke {
            poke();
          }
        });
      if let Some(id) = self.id("speed") {
        pill = pill.id(id);
      }
      foot.push(pill.into());
      menus.push(self.speed_menu());
    }
    let full = player.fullscreen.value;
    let (icon, label) = if full {
      (icons::FULLSCREEN_EXIT, player.words.exit_fullscreen.clone())
    } else {
      (icons::FULLSCREEN, player.words.fullscreen.clone())
    };
    let press = self.fullscreen_toggle();
    foot.push(self.touch_button("fullscreen", (Touch, TouchIcon), icon, &label, false, press));
    VideoPlayerTouch {
      landscape,
      lead,
      heading,
      tools,
      transport,
      time,
      foot,
      seek,
      menus,
      mark: self.mark(),
      top: self.live.buttons.clone(),
      row: self.live.row.clone(),
    }
  }

  /// The mark of a double tap: half an ellipse that comes in from the side tapped, with the glyph of ten seconds
  /// and how many it has gone so far.
  fn mark(&self) -> Option<Element> {
    let (forward, seconds) = self.live.mark.get()?;
    let (mark, text) = (self.part(VideoPlayerPart::Mark), self.part(VideoPlayerPart::MarkText));
    let wide = match mark.rest().width.and_then(|width| width.resolve(self.recipe.theme())) {
      Some(Dimension::Px(px)) => px,
      _ => 288.0,
    };
    let said = Column::new()
      .width(wide / 2.0)
      .height(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .spacing(mark.rest().gap.unwrap_or_default())
      .child(glyph(
        &self.part(VideoPlayerPart::MarkIcon),
        Name::new(if forward { icons::FORWARD_10 } else { icons::REWIND_10 }),
        false,
      ))
      .child(
        text
          .text(&format!("{seconds} {}", self.player.words.seconds))
          .nowrap()
          .part(&text),
      );
    let blank = Row::new().width(wide / 2.0);
    // The half of the ellipse that is in the picture holds what it says; the other is past the picture's edge.
    let halves: [Element; 2] = if forward {
      [said.into(), blank.into()]
    } else {
      [blank.into(), said.into()]
    };
    let ellipse = Row::new()
      .part(&mark)
      .height(Dimension::Pct(100.0))
      .flex_shrink(0.0)
      .with_children(halves)
      .offset(if forward { wide / 2.0 } else { -wide / 2.0 }, 0.0);
    Some(
      Row::new()
        .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
        .justify(if forward { Justify::End } else { Justify::Start })
        .pointer_events_none()
        .describe("a11y_hidden", "true")
        .child(ellipse)
        .into(),
    )
  }

  /// What a tap on the picture of the touch form does, given where across the window it was: it shows the
  /// controls or hides them; a second one soon after on the same side third seeks ten seconds that way and hides
  /// them, and each one after it ten more.
  fn tap(&self) -> Arc<dyn Fn(f32) + Send + Sync> {
    let player = self.player;
    let (shown, hide) = (self.live.shown.clone(), self.live.hide.clone());
    let (mark, unmark, tapped) = (
      self.live.mark.clone(),
      self.live.unmark.clone(),
      self.live.tapped.clone(),
    );
    let (root, mounted) = (self.live.root.clone(), self.live.mounted);
    let (seek, position) = (self.seeker(), player.position.value);
    let seekable = !player.live || player.duration.is_some();
    Arc::new(move |x: f32| {
      let side: i8 = match (mounted, root.width()) {
        (true, width) if width > 0.0 => match (x - root.x()) / width {
          at if at < 1.0 / 3.0 => -1,
          at if at > 2.0 / 3.0 => 1,
          _ => 0,
        },
        _ => 0,
      };
      let now = Instant::now();
      let again = {
        let mut last = tapped.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let again = matches!(*last, Some((then, was)) if side != 0 && was == side
          && now.duration_since(then) <= VIDEO_PLAYER_DOUBLE_PRESS);
        *last = Some((now, side));
        again
      };
      if again && seekable {
        let forward = side > 0;
        let so_far = match mark.get_untracked() {
          Some((was, seconds)) if was == forward => seconds,
          _ => 0,
        };
        mark.set(Some((forward, so_far + VIDEO_PLAYER_TAP_SEEKS as u32)));
        if let Some(unmark) = &unmark {
          unmark.restart();
        }
        seek(position + f64::from(side) * VIDEO_PLAYER_TAP_SEEKS);
        // Nothing but the mark shows.
        if shown.get_untracked() {
          shown.set(false);
        }
        if let Some(hide) = &hide {
          hide.cancel();
        }
        return;
      }
      if shown.get_untracked() {
        shown.set(false);
        if let Some(hide) = &hide {
          hide.cancel();
        }
      } else {
        shown.set(true);
      }
    })
  }

  /// The controls of the bar, made for the form.
  fn bar(&mut self, narrow: bool) -> VideoPlayerBar {
    let player = self.player;
    let seek = self.seek();
    let play = self.play();
    let time = self.time();
    let full = player.fullscreen.value;
    let fullscreen_label = if full {
      player.words.exit_fullscreen.clone()
    } else {
      player.words.fullscreen.clone()
    };
    let fullscreen_icon = if full {
      icons::FULLSCREEN_EXIT
    } else {
      icons::FULLSCREEN
    };
    let (volume, end, menus) = match self.form {
      VideoPlayerSize::Sm => (None, Vec::new(), Vec::new()),
      VideoPlayerSize::Md => {
        let mute = self.mute();
        let more = self.menu_button(
          VideoPlayerMenu::More,
          "more",
          Some(icons::MORE_VERTICAL),
          &player.words.more,
        );
        let press = self.fullscreen_toggle();
        let fullscreen = self.toggle_control("fullscreen", fullscreen_icon, &fullscreen_label, "F", false, press);
        (None, vec![mute, more, fullscreen], vec![self.more_menu()])
      }
      VideoPlayerSize::Lg => {
        let volume = self.volume(narrow);
        let mut end = Vec::new();
        let mut menus = Vec::new();
        if player.has_captions() {
          let captions_on = player.captions.value;
          let press = self.pressed(player.captions.teller(), !captions_on);
          end.push(self.toggle_control(
            "captions",
            icons::CAPTIONS,
            &player.words.captions,
            "C",
            captions_on,
            press,
          ));
        }
        if !player.live {
          end.push(self.menu_button(VideoPlayerMenu::Speed, "speed", None, &player.words.speed));
          menus.push(self.speed_menu());
        }
        if self.has_settings() {
          end.push(self.menu_button(
            VideoPlayerMenu::Settings,
            "settings",
            Some(icons::SETTINGS),
            &player.words.settings,
          ));
          menus.insert(0, self.settings_menu());
        }
        let pip = player.picture_in_picture.value;
        let press = self.pressed(player.picture_in_picture.teller(), !pip);
        end.push(self.toggle_control(
          "pip",
          icons::PICTURE_IN_PICTURE,
          &player.words.picture_in_picture,
          "",
          pip,
          press,
        ));
        let press = self.fullscreen_toggle();
        end.push(self.toggle_control("fullscreen", fullscreen_icon, &fullscreen_label, "F", false, press));
        (Some(volume), end, menus)
      }
    };
    let height = match self
      .part(VideoPlayerPart::Controls)
      .rest()
      .height
      .and_then(|height| height.resolve(self.recipe.theme()))
    {
      Some(Dimension::Px(px)) => px,
      _ => 0.0,
    };
    // The folded volume's slider is out while the pointer is over the control, the focus is in it or it is
    // dragged: read here, the bar is laid out again when that changes.
    let volume_out =
      narrow && self.form == VideoPlayerSize::Lg && self.live.volume.as_ref().is_some_and(|kept| kept.asks());
    VideoPlayerBar {
      seek,
      play,
      volume,
      time,
      volume_out,
      end,
      menus,
      row: self.live.row.clone(),
      buttons: self.live.buttons.clone(),
      height,
    }
  }

  /// The middle of the picture: the error, the spinner, or the play disc while it does not play.
  fn center(&mut self) -> Option<Element> {
    let player = self.player;
    if let Some(title) = &player.error {
      let part = self.part(VideoPlayerPart::Error);
      let text = self.part(VideoPlayerPart::Text);
      let title_part = self.part(VideoPlayerPart::Title);
      let mut words = Column::new()
        .part(&text)
        .align_items(Alignment::Center)
        .child(title_part.text(title).text_align(TextAlign::Center).part(&title_part));
      if let Some(description) = &player.error_description {
        let part = self.part(VideoPlayerPart::Description);
        words = words.child(part.text(description).text_align(TextAlign::Center).part(&part));
      }
      return Some(
        Column::new()
          .part(&part)
          .align_items(Alignment::Center)
          .describe("a11y_role", "alert")
          .child(glyph(
            &self.part(VideoPlayerPart::ErrorIcon),
            Name::new(icons::ALERT_CIRCLE),
            false,
          ))
          .child(words)
          .into(),
      );
    }
    if player.waiting {
      return Some(
        Row::new()
          .describe("a11y_role", "progressbar")
          .describe("a11y_busy", "true")
          .child(glyph(
            &self.part(VideoPlayerPart::Spinner),
            Name::new(icons::SPINNER),
            true,
          ))
          .into(),
      );
    }
    if player.playing.value && !player.ended || self.form == VideoPlayerSize::Sm {
      return None;
    }
    let size = if self.form == VideoPlayerSize::Md {
      PlayButtonSize::Lg
    } else {
      PlayButtonSize::Xl
    };
    let next = self.toggle();
    let mut button = PlayButton::new()
      .variant(PlayButtonVariant::Overlay)
      .size(size)
      .replay(player.ended)
      .play_label(&player.words.play)
      .replay_label(&player.words.replay)
      .on_change(move |_| next());
    if let Some(id) = self.id("center") {
      button = button.id(id);
    }
    Some(button.build(self.recipe.theme()))
  }

  /// The error's button, where the app listens for it.
  fn retry(&mut self) -> Option<Element> {
    let player = self.player;
    if player.error.is_none() || player.on_retry.is_empty() {
      return None;
    }
    let part = self.part(VideoPlayerPart::Retry);
    let handlers = player.on_retry.clone();
    let mut button = pressable(&part, false)
      .describe("a11y_role", "button")
      .describe("a11y_name", player.words.retry.clone())
      .child(part.text(&player.words.retry).nowrap())
      .on_click(move |_| {
        for handler in &handlers {
          handler();
        }
      });
    if let Some(id) = self.id("retry") {
      button = button.id(id);
    }
    Some(button.into())
  }

  /// What the keys do while the focus is in the player.
  fn keys(&self) -> impl Fn(KeyboardEvent) + Send + Sync + 'static {
    let player = self.player;
    let toggle = self.toggle();
    let seek = self.seeker();
    let position = player.position.value;
    let duration = player.duration;
    let seekable = !player.live || duration.is_some();
    let volume = (player.volume.value, player.volume.teller());
    let muted = (player.muted.value, player.muted.teller());
    let captions = (player.captions.value, player.captions.teller(), player.has_captions());
    let fullscreen = self.fullscreen_toggle();
    let full = player.fullscreen.value;
    let buttons = self.live.buttons.clone();
    let poke = self.live.mounted.then(|| self.live.poke());
    let controls = player.controls && player.error.is_none();
    move |event: KeyboardEvent| {
      if !controls || !event.focus_within || event.ctrl || event.alt || event.meta || event.text_input_focused {
        return;
      }
      // Space and the arrows are the buttons' and the sliders' own while the focus is on one of them.
      let on_a_control = buttons.focus_visible_within() || buttons.focused();
      let step = |by: f64| {
        if seekable {
          seek(position + by);
        }
      };
      let handled = match event.key.as_str() {
        " " if !on_a_control => {
          toggle();
          true
        }
        "k" | "K" => {
          toggle();
          true
        }
        "ArrowLeft" if !on_a_control => {
          step(-VIDEO_PLAYER_SEEK_STEP);
          true
        }
        "ArrowRight" if !on_a_control => {
          step(VIDEO_PLAYER_SEEK_STEP);
          true
        }
        "j" | "J" => {
          step(-2.0 * VIDEO_PLAYER_SEEK_STEP);
          true
        }
        "l" | "L" => {
          step(2.0 * VIDEO_PLAYER_SEEK_STEP);
          true
        }
        "Home" if !on_a_control && seekable => {
          seek(0.0);
          true
        }
        "End" if !on_a_control && seekable && duration.is_some() => {
          seek(duration.unwrap_or_default());
          true
        }
        "ArrowUp" | "ArrowDown" if !on_a_control => {
          let by = if event.key == "ArrowUp" {
            VIDEO_PLAYER_VOLUME_STEP
          } else {
            -VIDEO_PLAYER_VOLUME_STEP
          };
          (volume.1)(((volume.0 + by) * 100.0).round().clamp(0.0, 100.0) / 100.0);
          true
        }
        "m" | "M" => {
          (muted.1)(!muted.0);
          true
        }
        "c" | "C" if captions.2 => {
          (captions.1)(!captions.0);
          true
        }
        "f" | "F" => {
          fullscreen();
          true
        }
        // Escape leaves fullscreen, as it does everywhere.
        "Escape" if full => {
          fullscreen();
          true
        }
        _ => false,
      };
      if handled {
        event.prevent_default();
        if let Some(poke) = &poke {
          poke();
        }
      }
    }
  }
}

/// A video player as a renderer sees it: what it was given, the style of its parts, and its controls made.
pub struct VideoPlayerView {
  /// The form it is drawn in.
  pub form: VideoPlayerSize,
  /// Whether it is narrower than the volume's slider needs.
  pub narrow: bool,
  /// Whether it has its controls.
  pub controls: bool,
  /// Whether the bar shows, or has faded out while it plays.
  pub bar_shown: bool,
  /// Whether it shows its poster.
  pub poster_shown: bool,
  pub playing: bool,
  pub waiting: bool,
  pub ended: bool,
  pub live: bool,
  pub duration: Option<f64>,
  pub error: Option<Arc<str>>,
  pub error_description: Option<Arc<str>>,
  /// The captions' line to show, if any.
  pub caption: Option<Arc<str>>,
  /// What the player is called.
  pub label: Arc<str>,
  pub width: Option<Length>,
  pub height: Option<Length>,
  /// The most it is wide, where the app says.
  pub max_width: Option<Length>,
  /// The shape of the picture: its width over its height.
  pub aspect: f32,
  /// The picture.
  pub media: Option<Element>,
  /// The poster, while it shows.
  pub poster: Option<Element>,
  /// The app's layer.
  pub children: Vec<Element>,
  /// The bar's controls, where the bar is drawn.
  pub bar: Option<VideoPlayerBar>,
  /// The middle of the picture: the play disc, the spinner or the error.
  pub center: Option<Element>,
  /// The error's button.
  pub retry: Option<Element>,
  /// The style of each part.
  pub recipe: ResolvedRecipe,
  keys: Arc<dyn Fn(KeyboardEvent) + Send + Sync>,
  /// The touch form's controls, where it is drawn.
  pub touch: Option<VideoPlayerTouch>,
  /// What a tap on the picture of the touch form does, given where across the window it was.
  tap: Option<Arc<dyn Fn(f32) + Send + Sync>>,
  toggle: Action,
  fullscreen: Action,
  pressed: Arc<Mutex<Option<Instant>>>,
  poke: Option<Action>,
  root: ElementRef,
  /// Whether the refs are kept from one render to the next. The still form's are made anew each time, and an
  /// element given a new ref is a new element to the runtime: a slider in it would lose its drag and its focus
  /// each time the player is built again, so the still form gives its elements none.
  mounted: bool,
  /// What it says of what it plays to what reads the tree, by name: see [`said_to_tooling`].
  tooling: Vec<(&'static str, Arc<str>)>,
  /// Whether it fills its window: its corners are square then.
  filling: bool,
  id: Option<Arc<str>>,
}

impl VideoPlayerView {
  /// The style of a part, as this player is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  fn id(&self, suffix: &str) -> Option<String> {
    self.id.as_ref().map(|id| format!("{id}-{suffix}"))
  }

  fn render_themed(self) -> Element {
    match self.theme().renderer::<VideoPlayer>() {
      Some(renderer) => renderer(self),
      None => self.render(),
    }
  }

  /// A layer as large as the player.
  fn layer() -> Stack {
    Stack::new().size(Dimension::Pct(100.0), Dimension::Pct(100.0))
  }

  /// The player's box, with its behavior: a press of the picture plays and pauses, a double press fills the
  /// screen, the keys act while the focus is in it, and the pointer moving shows the controls.
  pub fn root(&self) -> Stack {
    let theme = self.theme();
    let mut root = Stack::new()
      .part(&self.part(VideoPlayerPart::Root))
      .focusable(true)
      .tab_index(-1)
      .describe("a11y_role", "group")
      .describe("a11y_name", self.label.clone());
    for (name, said) in &self.tooling {
      root = root.describe(*name, said.clone());
    }
    if self.mounted {
      root = root.ref_element(self.root.clone());
    }
    if self.filling {
      root = root.rounded(0.0);
    }
    let width = self.width.and_then(|width| width.resolve(theme));
    if let Some(width) = width {
      root = root.width(width);
    }
    if let Some(most) = self.max_width.and_then(|width| width.resolve(theme)) {
      root = root.max_width(most);
    }
    match (self.height.and_then(|height| height.resolve(theme)), width) {
      (Some(height), _) => root = root.height(height),
      // A width of its own and no height: a picture of its aspect. A width that is not a number of pixels, or
      // one that what holds it makes less, is measured by the mounted player, which gives it its height before
      // the frame is painted.
      (None, Some(Dimension::Px(width))) => root = root.height(shaped(width, self.aspect)),
      _ => {}
    }
    if self.controls {
      let keys = self.keys.clone();
      root = root.on_key_down(move |event: KeyboardEvent| keys(event));
    }
    if let Some(poke) = self.poke.clone() {
      root = root.on_mouse_move(move |_| poke());
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    root
  }

  /// The picture, and the poster over it while it shows. With the controls, a press plays and pauses and a double
  /// press asks for the whole screen.
  pub fn media(&self) -> Stack {
    let mut layer = Self::layer().part(&self.part(VideoPlayerPart::Media));
    if let Some(media) = &self.media {
      layer = layer.child(Self::layer().child(media.clone()));
    }
    if let Some(poster) = &self.poster {
      layer = layer.child(Self::layer().child(poster.clone()));
    }
    if let Some(tap) = self.tap.clone() {
      // The touch form: a tap shows and hides the controls, and two on a side seek.
      layer = layer.on_click(move |event: MouseEvent| tap(event.x));
    } else if self.controls && self.error.is_none() {
      // A press plays and pauses at once; a second one soon after asks for fullscreen too, as the engine's double
      // press would only after holding the first press back for its while.
      let (toggle, fullscreen, pressed) = (self.toggle.clone(), self.fullscreen.clone(), self.pressed.clone());
      layer = layer.on_click(move |event: MouseEvent| {
        toggle();
        // The time the window was given the press at, not the time its handler runs at.
        let now = event.time;
        let mut last = pressed.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        match last.take() {
          Some(then) if now.duration_since(then) <= VIDEO_PLAYER_DOUBLE_PRESS => fullscreen(),
          _ => *last = Some(now),
        }
      });
    }
    layer
  }

  /// The veil over the picture while it ended or failed.
  pub fn veil(&self) -> Option<Stack> {
    (self.controls && (self.ended || self.error.is_some())).then(|| {
      Self::layer()
        .part(&self.part(VideoPlayerPart::Veil))
        .pointer_events_none()
    })
  }

  /// The app's layer over the picture.
  pub fn overlay(&self) -> Stack {
    let mut layer = Self::layer()
      .part(&self.part(VideoPlayerPart::Overlay))
      .hit_test(HitTestBehavior::ContentOnly)
      .with_children(self.children.iter().cloned());
    if let Some(id) = self.id("overlay") {
      layer = layer.id(id);
    }
    layer
  }

  /// The middle of the picture, with the error's button under the error.
  pub fn center(&self) -> Option<Column> {
    let center = self.center.clone()?;
    // What stands in the middle (the play button) has the part's id.
    let mut middle = Column::new()
      .part(&self.part(VideoPlayerPart::Center).unnamed())
      .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .hit_test(HitTestBehavior::ContentOnly)
      .child(center);
    if let Some(retry) = &self.retry {
      middle = middle.child(retry.clone());
    }
    Some(middle)
  }

  /// The captions' line, above the bar; while the bar has faded it moves down by the bar's height.
  pub fn captions(&self) -> Option<Column> {
    let caption = self.caption.as_ref()?;
    let part = self.part(VideoPlayerPart::Caption);
    // In columns, so that a long line wraps within the room it has and is not cut.
    let mut line = Column::new()
      .part(&part)
      .align_items(Alignment::Center)
      .child(part.text(caption).text_align(TextAlign::Center));
    if let Some(id) = self.id("caption") {
      line = line.id(id);
    }
    let down = match &self.bar {
      Some(bar) if !self.bar_shown => bar.height,
      _ => 0.0,
    };
    Some(
      // `x-captions` is the id of the button that turns them on: what holds the line is not named after its part.
      Column::new()
        .part(&self.part(VideoPlayerPart::Captions).unnamed())
        .width(Dimension::Pct(100.0))
        .align_items(Alignment::Center)
        .pointer_events_none()
        // It has an offset in both states, so that a transition the recipe gives it runs.
        .offset(0.0, down)
        .child(line),
    )
  }

  /// The length on the poster, at its bottom end.
  pub fn chip(&self) -> Option<Row> {
    let duration = self
      .duration
      .filter(|_| self.controls && self.poster_shown && !self.live)?;
    let value = self.part(VideoPlayerPart::ChipValue);
    Some(
      Row::new()
        .part(&self.part(VideoPlayerPart::Chip))
        .width(Dimension::Pct(100.0))
        .justify(Justify::End)
        .pointer_events_none()
        .child(
          Row::new()
            .part(&value)
            .child(value.text(&media_time_text(duration, false)).nowrap()),
        ),
    )
  }

  /// The bar: the seek bar over the row (or in it, in the minimal form), on its gradient.
  pub fn bar(&self) -> Option<Column> {
    let bar = self.bar.as_ref()?;
    let part = self.part(VideoPlayerPart::Controls);
    let rest = part.rest();
    let end = rest
      .background
      .clone()
      .unwrap_or(crate::node::BackgroundColor::Color(Color::new(0, 0, 0, 0xB3)));
    let clear = crate::node::BackgroundColor::Color(Color::new(0, 0, 0, 0));
    let mut controls = Column::new()
      .part(&part)
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Stretch)
      // The row stands on the bottom padding whatever the seek bar's height.
      .justify(Justify::End)
      .overflow_visible()
      .background_gradient(crate::node::Gradient::linear(180.0, [(clear, 0.0), (end, 1.0)].map(
        |(color, at)| crate::node::gradient::GradientStop::at(color, at),
      )));
    if let Some(id) = self.id("controls") {
      controls = controls.id(id);
    }
    let row_part = self.part(VideoPlayerPart::Row);
    let mut row = Row::new()
      .part(&row_part)
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .overflow_visible();
    let mounted = self.mounted;
    if mounted {
      row = row.ref_element(bar.row.clone());
    }
    let held = |row: Row| {
      if mounted {
        row.ref_element(bar.buttons.clone())
      } else {
        row
      }
    };
    let buttons = |children: Vec<Element>| {
      Row::new()
        .align_items(Alignment::Center)
        .spacing(row_part.rest().gap.unwrap_or_default())
        .overflow_visible()
        .with_children(children)
    };
    if self.form == VideoPlayerSize::Sm {
      // One row: play, the seek bar and the time; the controls' own padding and gap.
      row = row
        .child(held(buttons(vec![bar.play.clone()])))
        .child(Row::new().flex(1.0).min_width(0.0).child(bar.seek.clone()))
        .child(bar.time.clone());
      return Some(controls.child(row).with_children(bar.menus.iter().cloned()));
    }
    let mut start = vec![bar.play.clone()];
    start.extend(bar.volume.clone());
    // A bar too narrow for the volume's slider and the time side by side shows one of them.
    if !bar.volume_out {
      start.push(bar.time.clone());
    }
    let mut all = buttons(start);
    all = all.child(Row::new().flex(1.0));
    all = all.with_children(bar.end.iter().cloned());
    row = row.child(held(all.flex(1.0)));
    Some(
      controls
        .child(bar.seek.clone())
        .child(row)
        .with_children(bar.menus.iter().cloned()),
    )
  }

  /// How the default renderer builds the touch form: the picture, the scrim while its controls show, the app's
  /// layer, the mark of a double tap, the error or the spinner, and its controls: the top row, the transport in
  /// the middle, and at the bottom the captions, the row of the time and the seek bar under it.
  fn render_touch(&self, touch: &VideoPlayerTouch) -> Element {
    let shown = self.bar_shown && self.error.is_none();
    let mut root = self.root().child(self.media());
    if let Some(veil) = self.veil() {
      root = root.child(veil);
    }
    if shown {
      root = root.child(
        Self::layer()
          .part(&self.part(VideoPlayerPart::Scrim))
          .pointer_events_none(),
      );
    }
    root = root.child(self.overlay());
    if let Some(mark) = &touch.mark {
      root = root.child(mark.clone());
    }
    if let Some(center) = self.center() {
      root = root.child(center);
    }
    if shown && let Some(transport) = &touch.transport {
      root = root.child(
        Column::new()
          .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
          .align_items(Alignment::Center)
          .justify(Justify::Center)
          .hit_test(HitTestBehavior::ContentOnly)
          .child(transport.clone()),
      );
    }
    let mut edges = Column::new()
      .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
      .justify(Justify::SpaceBetween)
      .align_items(Alignment::Stretch)
      .overflow_visible()
      .hit_test(HitTestBehavior::ContentOnly);
    let mut bottom = Column::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Stretch)
      .overflow_visible();
    if let Some(captions) = self.captions() {
      bottom = bottom.child(captions);
    }
    if shown {
      let mut top = Row::new()
        .part(&self.part(VideoPlayerPart::Top))
        .width(Dimension::Pct(100.0))
        .align_items(Alignment::Center)
        .overflow_visible();
      if self.mounted {
        top = top.ref_element(touch.top.clone());
      }
      if let Some(lead) = &touch.lead {
        top = top.child(lead.clone());
      }
      top = top.child(match &touch.heading {
        Some(heading) => Row::new().flex(1.0).min_width(0.0).clip().child(heading.clone()),
        None => Row::new().flex(1.0),
      });
      top = top.with_children(touch.tools.iter().cloned());
      let mut foot = Row::new()
        .part(&self.part(VideoPlayerPart::Foot))
        .width(Dimension::Pct(100.0))
        .align_items(Alignment::Center)
        .overflow_visible();
      if self.mounted {
        foot = foot.ref_element(touch.row.clone());
      }
      foot = foot
        .child(touch.time.clone())
        .child(Row::new().flex(1.0))
        .with_children(touch.foot.iter().cloned());
      let rail = Row::new()
        .part(&self.part(VideoPlayerPart::Rail))
        .width(Dimension::Pct(100.0))
        .overflow_visible()
        .child(touch.seek.clone());
      let mut controls = bottom.child(foot).child(rail);
      if let Some(id) = self.id("controls") {
        controls = controls.id(id);
      }
      edges = edges.child(top).child(controls);
    } else {
      edges = edges.child(Row::new()).child(bottom);
    }
    root
      .child(edges.with_children(touch.menus.iter().filter(|_| shown).cloned()))
      .into()
  }

  /// How the default renderer builds a player: the picture, the veil, the app's layer, the middle, and at the
  /// bottom the captions, the poster's length and the bar.
  pub fn render(self) -> Element {
    if let Some(touch) = &self.touch {
      return self.render_touch(touch);
    }
    let mut root = self.root().child(self.media());
    if let Some(veil) = self.veil() {
      root = root.child(veil);
    }
    root = root.child(self.overlay());
    if let Some(center) = self.center() {
      root = root.child(center);
    }
    let mut bottom = Column::new()
      .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
      .justify(Justify::End)
      .align_items(Alignment::Stretch)
      .overflow_visible()
      .hit_test(HitTestBehavior::ContentOnly);
    if let Some(captions) = self.captions() {
      bottom = bottom.child(captions);
    }
    if let Some(chip) = self.chip() {
      bottom = bottom.child(chip);
    }
    if let Some(bar) = self.bar() {
      bottom = bottom.child(bar);
    }
    root.child(bottom).into()
  }
}

impl crate::ui::Styled for VideoPlayer {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for VideoPlayer {
  const NAME: &'static str = "video_player";
  type Renderer = VideoPlayerRenderer;

  fn parts() -> crate::ui::Declared {
    VideoPlayerPart::declared()
  }

  fn states() -> crate::ui::Declared {
    VideoPlayerState::declared()
  }

  /// The design's video player (`pencil/lurq-design-system.pen`, Video Player `yKk3P`, Compact `cak7v`, Minimal
  /// `OUkUD`). What is over the picture is white on black at the board's alphas in a light and a dark theme alike:
  /// the picture is not themed, so themed controls would lose their contrast on it. The menus, the tooltips and the
  /// live badge stay the theme's.
  ///
  /// The large player is 640 by 360 with the large radius. Its bar is 88 high, padded 24 at the top, 12 at the
  /// sides and 8 at the bottom, 4 between the seek bar and the row, on a gradient from nothing to black at 70%; the
  /// row is 36 high with 4 between its buttons: 36 square with a white glyph of 20, the middle radius, white at 12%
  /// under the pointer and at 20% while on, and a white ring of 2 from the keyboard. The speed is "1×" in 13/500
  /// padded 10. The time has 8 on each side. The compact player's bar is 72 high, padded 20, 8 and 6, with a row of
  /// 28 and 2 between buttons of 28 with glyphs of 16, and 6 around its time; the minimal one is one row 48 high,
  /// padded 16 8 4 4, 6 apart. The captions' line is 12 over the bar and 24 from the sides, in a box of black at 70%
  /// padded 2 8 with the small radius, its text 16/500 with a line of 1.4. The poster's length is 12 from the
  /// bottom and the end, mono 11 in a box of black at 70% padded 2 6. The veil is black at 60%. While it plays the
  /// bar fades out over the slow duration and comes back over the base one.
  fn recipe() -> Recipe {
    use VideoPlayerPart::*;
    use VideoPlayerSize::{Md, Sm};
    let white = Color::new(255, 255, 255, 255);
    let white_60 = Color::new(255, 255, 255, 0x99);
    let white_20 = Color::new(255, 255, 255, 0x33);
    let white_12 = Color::new(255, 255, 255, 0x1F);
    let black_60 = Color::new(0, 0, 0, 0x99);
    let black_70 = Color::new(0, 0, 0, 0xB3);
    let ink = Color::new(0x18, 0x18, 0x1B, 255);
    let ring = move |s: super::PartStyle| s.outline(Outline::new(2.0, white));
    Recipe::new()
      .base(Root, |s| {
        s.width(640.0)
          .height(360.0)
          .radius(RadiusSize::Lg)
          .background(Color::new(0, 0, 0, 255))
          .foreground(white)
      })
      .size(Md, Root, |s| s.width(320.0).height(180.0))
      .size(Sm, Root, |s| s.width(200.0).height(113.0))
      // The picture is pressed to play; while the bar has faded the pointer is hidden over it.
      .base(Media, |s| s.cursor(CursorIcon::Pointer))
      .state(VideoPlayerState::Bare, Media, |s| s.cursor(CursorIcon::Default))
      .state(VideoPlayerState::Error, Media, |s| s.cursor(CursorIcon::Default))
      .state(VideoPlayerState::Hidden, Root, |s| s.cursor(CursorIcon::None))
      .state(VideoPlayerState::Hidden, Media, |s| s.cursor(CursorIcon::None))
      .base(Veil, |s| s.background(black_60))
      // The touch form (Mobile/Video Player `Md4tS`, Landscape `HFTze`): no radius; a scrim of black at 40% while
      // its controls show; a top row of 44 padded 4 with round buttons of 44 and glyphs of 24; in the middle ten
      // seconds back and on, 44 with glyphs of 28, beside a play of 56 with a glyph of 32 on black at 40%, 32
      // apart; a bottom row of 44 padded 16 and 4, and the seek bar 16 from the sides and 6 from the bottom. On
      // its side: 44 at each side for the notch, the top row 8 down with a title of 16/600 and a line of 13 at
      // 60%, the transport 56 apart with 48, 32, 72 and 36, the bottom row padded 56 and 44 with the speed in a
      // pill of 44, 15/500, and the seek bar 56 from the sides.
      .state(VideoPlayerState::Touch, Root, |s| s.radius(0.0))
      .base(Scrim, |s| s.background(Color::new(0, 0, 0, 0x66)))
      .base(Top, |s| s.height(44.0).padding_x(4.0))
      .state(VideoPlayerState::Landscape, Top, |s| s.height(52.0).padding_top(8.0).padding_x(44.0).gap(4.0))
      .base(Heading, |s| s.padding_x(8.0))
      .state(VideoPlayerState::Landscape, Title, |s| s.font_size(16.0).weight(FontWeight::SemiBold))
      .base(Line, |s| {
        s.typography(TypographyStyle::Body).font_size(13.0).foreground(white_60)
      })
      .base(Touch, |s| {
        fading(
          s.square(44.0)
            .radius(RadiusSize::Full)
            .foreground(white)
            .cursor(CursorIcon::Pointer),
        )
      })
      .hovered(Touch, |s| s.background(white_12))
      .pressed(Touch, |s| s.background(white_20))
      .focus_visible(Touch, ring)
      .state(VideoPlayerState::On, Touch, |s| s.background(white_20))
      .base(TouchIcon, |s| s.square(24.0))
      .base(Transport, |s| s.gap(32.0))
      .state(VideoPlayerState::Landscape, Transport, |s| s.gap(56.0))
      .base(Skip, |s| {
        fading(
          s.square(44.0)
            .radius(RadiusSize::Full)
            .foreground(white)
            .cursor(CursorIcon::Pointer),
        )
      })
      .state(VideoPlayerState::Landscape, Skip, |s| s.square(48.0))
      .hovered(Skip, |s| s.background(white_12))
      .pressed(Skip, |s| s.background(white_20))
      .focus_visible(Skip, ring)
      .base(SkipIcon, |s| s.square(28.0))
      .state(VideoPlayerState::Landscape, SkipIcon, |s| s.square(32.0))
      .base(Disc, |s| {
        fading(
          s.square(56.0)
            .radius(RadiusSize::Full)
            .background(Color::new(0, 0, 0, 0x66))
            .foreground(white)
            .cursor(CursorIcon::Pointer),
        )
      })
      .state(VideoPlayerState::Landscape, Disc, |s| s.square(72.0))
      .hovered(Disc, |s| s.background(black_60))
      .pressed(Disc, |s| s.background(black_70))
      .focus_visible(Disc, ring)
      .base(DiscIcon, |s| s.square(32.0))
      .state(VideoPlayerState::Landscape, DiscIcon, |s| s.square(36.0))
      .base(Foot, |s| s.height(44.0).padding_start(16.0).padding_end(4.0))
      .state(VideoPlayerState::Landscape, Foot, |s| s.padding_start(56.0).padding_end(44.0).gap(4.0))
      .base(Pill, |s| {
        fading(
          s.height(44.0)
            .min_width(44.0)
            .padding_x(12.0)
            .radius(RadiusSize::Full)
            .foreground(white)
            .typography(TypographyStyle::Body)
            .font_size(15.0)
            .weight(FontWeight::Medium)
            .cursor(CursorIcon::Pointer),
        )
      })
      .hovered(Pill, |s| s.background(white_12))
      .pressed(Pill, |s| s.background(white_20))
      .focus_visible(Pill, ring)
      .state(VideoPlayerState::On, Pill, |s| s.background(white_20))
      .base(Rail, |s| s.padding_x(16.0).padding_bottom(6.0))
      .state(VideoPlayerState::Landscape, Rail, |s| s.padding_x(56.0).padding_bottom(12.0))
      .base(Mark, |s| s.width(288.0).radius(RadiusSize::Full).background(white_20).gap(4.0))
      .base(MarkIcon, |s| s.square(28.0).foreground(white))
      .base(MarkText, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .weight(FontWeight::Medium)
          .foreground(white)
      })
      .base(Center, |s| s.gap(8.0))
      .base(Spinner, |s| s.square(32.0).foreground(white))
      .base(Error, |s| s.gap(12.0).padding_x(24.0).max_width(360.0))
      .base(ErrorIcon, |s| s.square(24.0).foreground(white))
      .base(Text, |s| s.gap(4.0))
      .base(Title, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(14.0)
          .weight(FontWeight::Medium)
          .foreground(white)
      })
      .base(Description, |s| {
        s.typography(TypographyStyle::Body).font_size(13.0).foreground(white_60)
      })
      .base(Retry, |s| {
        fading(
          s.height(32.0)
            .padding_x(12.0)
            .radius(RadiusSize::Md)
            .background(white)
            .foreground(ink)
            .typography(TypographyStyle::Body)
            .font_size(14.0)
            .weight(FontWeight::Medium)
            .cursor(CursorIcon::Pointer),
        )
      })
      .hovered(Retry, |s| s.background(Color::new(255, 255, 255, 0xE6)))
      .focus_visible(Retry, |s| s.outline(Outline::new(2.0, white).offset(2.0)))
      .base(Captions, |s| {
        s.padding_x(24.0)
          .padding_bottom(12.0)
          .transition(PartTransition::new(Transition::offset_y(), MotionDuration::Base).easing(MotionEasing::Enter))
      })
      .state(VideoPlayerState::Hidden, Captions, |s| {
        s.transition(PartTransition::new(Transition::offset_y(), MotionDuration::Slow).easing(MotionEasing::Exit))
      })
      .base(Caption, |s| {
        s.padding_x(8.0)
          .padding_y(2.0)
          .radius(RadiusSize::Sm)
          .background(black_70)
          .foreground(white)
          .typography(TypographyStyle::Body)
          .font_size(16.0)
          .weight(FontWeight::Medium)
          .line_height(1.4)
      })
      .base(Chip, |s| s.padding_x(12.0).padding_bottom(12.0))
      .base(ChipValue, |s| {
        s.padding_x(6.0)
          .padding_y(2.0)
          .radius(RadiusSize::Sm)
          .background(black_70)
          .foreground(white)
          .typography(TypographyStyle::Mono)
          .font_size(11.0)
      })
      .base(Controls, |s| {
        s.height(88.0)
          .padding_top(24.0)
          .padding_x(12.0)
          .padding_bottom(8.0)
          .gap(4.0)
          .background(black_70)
          .opacity(1.0)
          .transition(PartTransition::new(Transition::opacity(), MotionDuration::Base).easing(MotionEasing::Enter))
      })
      .size(Md, Controls, |s| {
        s.height(72.0).padding_top(20.0).padding_x(8.0).padding_bottom(6.0).gap(2.0)
      })
      .size(Sm, Controls, |s| {
        s.height(48.0)
          .padding_top(16.0)
          .padding_start(4.0)
          .padding_end(8.0)
          .padding_bottom(4.0)
          .gap(0.0)
      })
      .state(VideoPlayerState::Hidden, Controls, |s| {
        s.opacity(0.0)
          .transition(PartTransition::new(Transition::opacity(), MotionDuration::Slow).easing(MotionEasing::Exit))
      })
      .base(Row, |s| s.height(36.0).gap(4.0))
      .size(Md, Row, |s| s.height(28.0).gap(2.0))
      .size(Sm, Row, |s| s.height(28.0).gap(6.0))
      .base(Control, |s| {
        fading(
          s.square(36.0)
            .radius(RadiusSize::Md)
            .foreground(white)
            .cursor(CursorIcon::Pointer),
        )
      })
      .size(Md, Control, |s| s.square(28.0))
      .size(Sm, Control, |s| s.square(28.0))
      .hovered(Control, |s| s.background(white_12))
      .focus_visible(Control, ring)
      .state(VideoPlayerState::On, Control, |s| s.background(white_20))
      // On stays on under the pointer.
      .when(Selector::new().state(VideoPlayerState::On).hovered(), Control, |s| {
        s.background(white_20)
      })
      .base(ControlIcon, |s| s.square(20.0))
      .size(Md, ControlIcon, |s| s.square(16.0))
      .size(Sm, ControlIcon, |s| s.square(16.0))
      .base(Speed, |s| {
        fading(
          s.height(36.0)
            .padding_x(10.0)
            .radius(RadiusSize::Md)
            .foreground(white)
            .typography(TypographyStyle::Body)
            .font_size(13.0)
            .weight(FontWeight::Medium)
            .cursor(CursorIcon::Pointer),
        )
      })
      .hovered(Speed, |s| s.background(white_12))
      .focus_visible(Speed, ring)
      .state(VideoPlayerState::On, Speed, |s| s.background(white_20))
      .when(Selector::new().state(VideoPlayerState::On).hovered(), Speed, |s| {
        s.background(white_20)
      })
      .base(Time, |s| s.padding_x(8.0).foreground(white))
      .size(Md, Time, |s| s.padding_x(6.0))
      .size(Sm, Time, |s| s.padding_x(0.0))
      .base(Elapsed, |s| s.typography(TypographyStyle::Mono).font_size(11.0).foreground(white))
      // A menu's bottom is 4 over the seek bar, which is 16 high over the row and the bar's gap.
      .base(Menus, |s| s.gap(24.0))
      .size(Md, Menus, |s| s.gap(22.0))
      // The menu's own least width is 230: the board's settings are 240 and the compact menu 200.
      .base(SettingsMenu, |s| s.width(240.0).min_width(240.0))
      .base(MoreMenu, |s| s.width(200.0).min_width(200.0))
      .base(MenuValue, |s| s.typography(TypographyStyle::Body).font_size(13.0))
  }
}
