//! An audio player: what plays, where it is, and the controls that tell the app what it is to be.
//!
//! One component in four forms (`AudioPlayerVariant`): the full card, one row of 48 for a list or a message, a voice
//! message, and a mini player docked to the bottom of a window. Each holds the same values, which are the app's:
//! whether it plays, where it is and how long it is, how much is loaded, the volume, whether it is muted, the speed,
//! shuffle, repeat, whether it is loading and what went wrong. A press of a control tells the app what it is to be and
//! changes nothing by itself: the app answers by setting the value, or binds it to a signal the press sets.

use std::sync::Arc;
#[cfg(feature = "media")]
use std::sync::{
  Mutex,
  atomic::{AtomicBool, Ordering},
};

use super::{
  Button, ButtonPart, ButtonShape, ButtonSize, ButtonVariant, Link, MediaSpeedMenu, MediaTime, MediaVolume, PlayButton,
  PlayButtonPart, PlayButtonSize, SeekBar, Skeleton, Toggle, Tooltip, Waveform, icons,
  link::LinkPart,
  media_time_text, names,
  part_style::{FontFamily, Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{fading, glyph, pressable},
  theme::{Themed, current_theme},
  toggle::TogglePart,
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{KeyboardEvent, MouseEvent},
    theme::{BorderSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{Column, Row, Stack, TextOverflow},
  core::{ElementRef, Signal},
  layout::{Alignment, StackAlignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, border::Border, dimension::Dimension},
};

/// The forms of an audio player.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AudioPlayerVariant {
  /// The card of 360: the artwork, the title and who it is by, the seek bar with the times under its ends, the
  /// transport (shuffle, previous, play, next, repeat), and the volume and the speed under it.
  #[default]
  Full,
  /// One row of 48 for a list or a message: a play button, the seek bar (or a waveform), the time.
  Compact,
  /// A voice message: a play button, the waveform, the time, and the speed while it plays or a dot while it has not
  /// been heard.
  Voice,
  /// Docked to the bottom of a window, 56 high: a progress line along its top edge, the track, the transport, the
  /// time, the volume, the queue and the expand buttons. Under 480 wide, the track, play and next only.
  Mini,
}

names!(AudioPlayerVariant {
  Full => "full",
  Compact => "compact",
  Voice => "voice",
  Mini => "mini",
});

impl AudioPlayerVariant {
  fn of(name: Name) -> Self {
    [Self::Compact, Self::Voice, Self::Mini]
      .into_iter()
      .find(|variant| Name::from(*variant) == name)
      .unwrap_or(Self::Full)
  }
}

/// What plays after the end of what plays: nothing, the whole list again, or the same track again.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AudioRepeat {
  #[default]
  Off,
  All,
  One,
}

names!(AudioRepeat {
  Off => "off",
  All => "all",
  One => "one",
});

impl AudioRepeat {
  /// What a press of the repeat button makes it: off, all, one, and off again.
  ///
  /// ```
  /// use lurq::ui::AudioRepeat;
  ///
  /// assert_eq!(AudioRepeat::Off.next(), AudioRepeat::All);
  /// assert_eq!(AudioRepeat::All.next(), AudioRepeat::One);
  /// assert_eq!(AudioRepeat::One.next(), AudioRepeat::Off);
  /// ```
  pub fn next(self) -> Self {
    match self {
      Self::Off => Self::All,
      Self::All => Self::One,
      Self::One => Self::Off,
    }
  }
}

/// The speeds a press of a voice message's badge steps through, while it plays.
pub const VOICE_RATES: [f32; 3] = [1.0, 1.5, 2.0];

/// A speed as the button of a player reads it: "1×", "1.5×", "0.75×".
///
/// ```
/// use lurq::ui::audio_rate_text;
///
/// assert_eq!(audio_rate_text(1.0), "1×");
/// assert_eq!(audio_rate_text(1.5), "1.5×");
/// ```
pub fn audio_rate_text(rate: f64) -> String {
  format!("{}×", rate as f32)
}

/// The parts of an audio player a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AudioPlayerPart {
  /// The player.
  Root,
  /// The mini player's row under its progress line.
  Bar,
  /// What the mini player's progress line lies over: the line of the border along its top edge.
  Edge,
  /// The artwork, the title and who it is by, and the "more" button of the full card; the track of the mini player.
  Header,
  /// The artwork's box: the app's picture, or the `music` glyph on `Muted` where there is none.
  Media,
  /// The `music` glyph where there is no artwork.
  MediaIcon,
  /// The column of the title and who it is by.
  Text,
  /// The title.
  Title,
  /// Who it is by, under the title.
  Description,
  /// The loading lines in place of the title and who it is by.
  Skeleton,
  /// The "more" button of the full card.
  More,
  /// The seek bar and the times under it.
  Progress,
  /// The row of the elapsed and the total time under the seek bar.
  Times,
  /// The elapsed time.
  Elapsed,
  /// The total time.
  Total,
  /// The one time of the compact player and of a voice message.
  Time,
  /// The previous, play and next buttons, with shuffle and repeat in the full card.
  Transport,
  /// The shuffle button: muted while off, the accent fill while on.
  Shuffle,
  /// The repeat button: muted while off, the accent fill while on.
  Repeat,
  /// The row under the transport: the volume and the speed.
  Footer,
  /// The speed button, which opens the speed menu.
  Speed,
  /// The line that says what went wrong, in place of the times or of the bar.
  Error,
  /// The icon before the error.
  ErrorIcon,
  /// The words of the error.
  ErrorText,
  /// "Try again" at the end of the full card's error.
  Retry,
  /// A voice message's speed, which a press steps.
  Rate,
  /// The dot of a voice message not heard yet.
  Dot,
  /// The mini player's progress line: what is not played.
  Track,
  /// What is played of the progress line.
  Fill,
  /// What stands at the right of the mini player: the time, the volume, the queue and expand buttons.
  Tools,
}

names!(AudioPlayerPart {
  Root => "root",
  Bar => "bar",
  Edge => "edge",
  Header => "header",
  Media => "media",
  MediaIcon => "media_icon",
  Text => "text",
  Title => "title",
  Description => "description",
  Skeleton => "skeleton",
  More => "more",
  Progress => "progress",
  Times => "times",
  Elapsed => "elapsed",
  Total => "total",
  Time => "time",
  Transport => "transport",
  Shuffle => "shuffle",
  Repeat => "repeat",
  Footer => "footer",
  Speed => "speed",
  Error => "error",
  ErrorIcon => "error_icon",
  ErrorText => "error_text",
  Retry => "retry",
  Rate => "rate",
  Dot => "dot",
  Track => "track",
  Fill => "fill",
  Tools => "tools",
});

/// The states of an audio player a recipe styles, besides what the pointer and the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AudioPlayerState {
  /// It plays.
  Playing,
  /// What it would play is being fetched.
  Loading,
  /// It could not play: the line of the error stands in place of the times or of the bar.
  Error,
  /// It takes no input.
  Disabled,
  /// It has artwork.
  Media,
  /// A voice message not heard yet.
  Unheard,
  /// Shuffle is on.
  Shuffle,
  /// A mini player under 480 wide: the track, play and next only.
  Narrow,
  /// The pointer is on the mini player's progress line.
  Scrubbing,
}

names!(AudioPlayerState {
  Playing => "playing",
  Loading => "loading",
  Error => "error",
  Disabled => "disabled",
  Media => "media",
  Unheard => "unheard",
  Shuffle => "shuffle",
  Narrow => "narrow",
  Scrubbing => "scrubbing",
});

/// How wide a mini player is at least to show more than the track, play and next.
pub const AUDIO_MINI_NARROW: f32 = 480.0;

/// How wide a mini player is at least to read the time out: narrower, the queue and the expand buttons and the
/// volume's button are all that stands at its end.
pub const AUDIO_MINI_TIME_FROM: f32 = 560.0;

/// How wide a mini player is at least to show the volume's slider beside its button: with the time, the slider
/// and the two buttons its end is 264 wide, as its start is, around a transport of 108.
pub const AUDIO_MINI_VOLUME_FROM: f32 = 690.0;

/// What a mini player's width folds away: nothing, the volume's slider, or the time too.
fn mini_fold(width: f32) -> u8 {
  if width < AUDIO_MINI_TIME_FROM {
    2
  } else if width < AUDIO_MINI_VOLUME_FROM {
    1
  } else {
    0
  }
}

/// What builds an audio player's nodes: a theme's in place of the default one.
pub type AudioPlayerRenderer = Arc<dyn Fn(AudioPlayerView) -> Element + Send + Sync>;

type Tell<T> = Arc<dyn Fn(T) + Send + Sync>;
type Command = Arc<dyn Fn() + Send + Sync>;

/// A value of the app's and who is told what a person makes it: the app's signal first where it is bound.
#[derive(Clone)]
struct Held<T> {
  value: T,
  tell: Vec<Tell<T>>,
}

impl<T: Copy + Send + Sync + 'static> Held<T> {
  fn new(value: T) -> Self {
    Self {
      value,
      tell: Vec::new(),
    }
  }

  fn bind<S: Clone + Send + Sync + crate::core::SignalValue + 'static>(
    &mut self,
    signal: Signal<S>,
    from: impl Fn(S) -> T,
    to: impl Fn(T) -> S + Send + Sync + 'static,
  ) {
    self.value = from(signal.get());
    self.tell.insert(0, Arc::new(move |next| signal.set(to(next))));
  }

  /// What tells every handler `next`.
  fn teller(&self) -> impl Fn(T) + Send + Sync + Clone + 'static {
    let tell = self.tell.clone();
    move |next| {
      for handler in &tell {
        handler(next);
      }
    }
  }
}

fn run(commands: &[Command]) -> impl Fn() + Send + Sync + Clone + 'static {
  let commands = commands.to_vec();
  move || {
    for command in &commands {
      command();
    }
  }
}

/// The words an audio player writes itself.
#[derive(Clone)]
struct Words {
  label: Arc<str>,
  play: Arc<str>,
  pause: Arc<str>,
  previous: Arc<str>,
  next: Arc<str>,
  shuffle: Arc<str>,
  repeat: Arc<str>,
  more: Arc<str>,
  speed: Arc<str>,
  queue: Arc<str>,
  expand: Arc<str>,
  seek: Arc<str>,
  retry: Arc<str>,
}

impl Words {
  /// The words a locale has for a player.
  fn of(words: &crate::core::MediaWords) -> Self {
    Self {
      label: words.audio_player.clone(),
      play: words.play.clone(),
      pause: words.pause.clone(),
      previous: words.previous.clone(),
      next: words.next.clone(),
      shuffle: words.shuffle.clone(),
      repeat: words.repeat.clone(),
      more: words.more.clone(),
      speed: words.playback_speed.clone(),
      queue: words.queue.clone(),
      expand: words.expand.clone(),
      seek: words.seek.clone(),
      retry: words.try_again.clone(),
    }
  }
}

/// The words of the locale of what is being built: the app's in a render, English outside one.
impl Default for Words {
  fn default() -> Self {
    Self::of(&super::current_locale().media)
  }
}

/// An audio player.
///
/// ```
/// use lurq::{
///   core::Signal,
///   ui::{AudioPlayer, AudioPlayerVariant, AudioRepeat},
/// };
///
/// // Bound to the app's position, in seconds, of a track 250 seconds long, and to whether it plays.
/// let (position, playing) = (Signal::new(84.0_f32), Signal::new(false));
/// let player = AudioPlayer::new(position, 250.0)
///   .title("Night Drive")
///   .description("Aurora Lane · Low Tide")
///   .playing(playing)
///   .repeat_controlled(AudioRepeat::Off, |repeat| println!("repeat {repeat:?}"))
///   .on_next(|| println!("next"));
/// // A row of a chat: the app holds where it is and is told where a person moves it.
/// let message = AudioPlayer::controlled(17.0, 42.0, |to| println!("seek to {to}"))
///   .variant(AudioPlayerVariant::Voice)
///   .peaks(&[0.3, 0.8, 0.5, 1.0, 0.4]);
/// ```
///
/// The position is the player's first value: bound with [`AudioPlayer::new`], held by the app with
/// [`AudioPlayer::controlled`]; [`AudioPlayer::on_change`] hears it as a drag moves it and
/// [`AudioPlayer::on_change_end`] where the drag is let go. Every other value has a bound form, a controlled form and a
/// handler of its own: `playing` with `on_playing_change`, `volume` with `on_volume_change`, `muted` with
/// `on_muted_change`, `rate` with `on_rate_change`, `shuffle` with `on_shuffle_change`, `repeat` with
/// `on_repeat_change`. Previous, next, more, the queue, expand and "try again" are commands. A focused player plays and
/// pauses on Space, and its seek bar and volume move on the arrow keys.
///
/// [`AudioPlayer::mount`] keeps what the player needs between renders (its tooltips, its speed menu, a mini
/// player's own width). A player given to a tree as it is (`.child(AudioPlayer::new(..))`) is the same component,
/// declared: what renders the tree mounts it. [`AudioPlayer::build`] is a still picture of it, for a test.
#[derive(Clone)]
pub struct AudioPlayer {
  variant: Name,
  position: Held<f64>,
  on_change_end: Vec<Tell<f64>>,
  duration: f64,
  buffered: Option<f64>,
  playing: Held<bool>,
  volume: Held<f64>,
  muted: Held<bool>,
  rate: Held<f64>,
  shuffle: Held<bool>,
  repeat: Held<AudioRepeat>,
  loading: bool,
  error: Option<Arc<str>>,
  unheard: bool,
  disabled: bool,
  title: Option<Arc<str>>,
  description: Option<Arc<str>>,
  media: Option<Element>,
  peaks: Vec<f32>,
  on_previous: Vec<Command>,
  on_next: Vec<Command>,
  on_more: Vec<Command>,
  on_queue: Vec<Command>,
  on_expand: Vec<Command>,
  on_retry: Vec<Command>,
  more_anchor: Option<ElementRef>,
  words: Words,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
  /// The words it was told to speak in, in place of the app's: what its source's errors are said in.
  locale: Option<crate::core::MediaWords>,
  #[cfg(feature = "media")]
  source: Option<crate::resources::Source>,
}

impl AudioPlayer {
  fn blank(position: f64, duration: f64) -> Self {
    Self {
      variant: AudioPlayerVariant::Full.into(),
      position: Held::new(position),
      on_change_end: Vec::new(),
      duration,
      buffered: None,
      playing: Held::new(false),
      volume: Held::new(1.0),
      muted: Held::new(false),
      rate: Held::new(1.0),
      shuffle: Held::new(false),
      repeat: Held::new(AudioRepeat::Off),
      loading: false,
      error: None,
      unheard: false,
      disabled: false,
      title: None,
      description: None,
      media: None,
      peaks: Vec::new(),
      on_previous: Vec::new(),
      on_next: Vec::new(),
      on_more: Vec::new(),
      on_queue: Vec::new(),
      on_expand: Vec::new(),
      on_retry: Vec::new(),
      more_anchor: None,
      words: Words::default(),
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
      locale: None,
      #[cfg(feature = "media")]
      source: None,
    }
  }

  /// A player bound to `position`, in seconds, of something `duration` seconds long. What a person does on the seek
  /// bar sets the signal.
  pub fn new(position: Signal<f32>, duration: impl Into<f64>) -> Self {
    let mut player = Self::blank(0.0, duration.into());
    player.position.bind(position, f64::from, |to| to as f32);
    player
  }

  /// A player that shows `position`, held by the app, and tells `on_change` where a person moves it.
  pub fn controlled(
    position: impl Into<f64>,
    duration: impl Into<f64>,
    on_change: impl Fn(f64) + Send + Sync + 'static,
  ) -> Self {
    Self::blank(position.into(), duration.into()).on_change(on_change)
  }

  /// Its form, an [`AudioPlayerVariant`].
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// Called with the position, in seconds, each time a person moves it. Each call adds a handler.
  pub fn on_change(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.position.tell.push(Arc::new(handler));
    self
  }

  /// Called with the position, in seconds, where a drag is let go, a key's step is made or the line is pressed: a
  /// player that seeks only then listens to this. Each call adds a handler.
  pub fn on_change_end(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.on_change_end.push(Arc::new(handler));
    self
  }

  /// How far what plays is loaded, in seconds from its start.
  pub fn buffered(mut self, buffered: impl Into<f64>) -> Self {
    self.buffered = Some(buffered.into());
    self
  }

  /// Playing while `playing` is true, which a press of the play button turns.
  pub fn playing(mut self, playing: Signal<bool>) -> Self {
    self.playing.bind(playing, |on| on, |on| on);
    self
  }

  /// Playing while `playing` is true, held by the app, which is told what a press makes it.
  pub fn playing_controlled(mut self, playing: bool, on_playing_change: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.playing.value = playing;
    self.on_playing_change(on_playing_change)
  }

  /// Called on a press of the play button (or Space), with whether it is to play. Each call adds a handler.
  pub fn on_playing_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.playing.tell.push(Arc::new(handler));
    self
  }

  /// The volume, from 0 to 1, bound to `volume`.
  pub fn volume(mut self, volume: Signal<f32>) -> Self {
    self.volume.bind(volume, f64::from, |to| to as f32);
    self
  }

  /// The volume, from 0 to 1, held by the app, which is told what a person makes it.
  pub fn volume_controlled(
    mut self,
    volume: impl Into<f64>,
    on_volume_change: impl Fn(f64) + Send + Sync + 'static,
  ) -> Self {
    self.volume.value = volume.into();
    self.on_volume_change(on_volume_change)
  }

  /// Called with the volume each time a person moves it. Each call adds a handler.
  pub fn on_volume_change(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.volume.tell.push(Arc::new(handler));
    self
  }

  /// Muted while `muted` is true, which a press of the volume's button turns. Muting keeps the volume.
  pub fn muted(mut self, muted: Signal<bool>) -> Self {
    self.muted.bind(muted, |on| on, |on| on);
    self
  }

  /// Muted while `muted` is true, held by the app, which is told what a press makes it.
  pub fn muted_controlled(mut self, muted: bool, on_muted_change: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.muted.value = muted;
    self.on_muted_change(on_muted_change)
  }

  /// Called on a press of the volume's button, with whether it is to be muted. Each call adds a handler.
  pub fn on_muted_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.muted.tell.push(Arc::new(handler));
    self
  }

  /// The speed, 1 for the speed it was made at, bound to `rate`.
  pub fn rate(mut self, rate: Signal<f32>) -> Self {
    self.rate.bind(rate, f64::from, |to| to as f32);
    self
  }

  /// The speed held by the app, which is told what a person chooses.
  pub fn rate_controlled(mut self, rate: impl Into<f64>, on_rate_change: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.rate.value = rate.into();
    self.on_rate_change(on_rate_change)
  }

  /// Called with the speed a person chooses in the speed menu, or steps to with a voice message's badge. Each call
  /// adds a handler.
  pub fn on_rate_change(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.rate.tell.push(Arc::new(handler));
    self
  }

  /// Shuffle, bound to `shuffle`, which a press of its button turns.
  pub fn shuffle(mut self, shuffle: Signal<bool>) -> Self {
    self.shuffle.bind(shuffle, |on| on, |on| on);
    self
  }

  /// Shuffle held by the app, which is told what a press makes it.
  pub fn shuffle_controlled(mut self, shuffle: bool, on_shuffle_change: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.shuffle.value = shuffle;
    self.on_shuffle_change(on_shuffle_change)
  }

  /// Called on a press of the shuffle button, with whether shuffle is to be on. Each call adds a handler.
  pub fn on_shuffle_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.shuffle.tell.push(Arc::new(handler));
    self
  }

  /// What plays after the end, bound to `repeat`, which a press of its button steps.
  pub fn repeat(mut self, repeat: Signal<AudioRepeat>) -> Self {
    self.repeat.bind(repeat, |repeat| repeat, |repeat| repeat);
    self
  }

  /// What plays after the end, held by the app, which is told what a press makes it.
  pub fn repeat_controlled(
    mut self,
    repeat: AudioRepeat,
    on_repeat_change: impl Fn(AudioRepeat) + Send + Sync + 'static,
  ) -> Self {
    self.repeat.value = repeat;
    self.on_repeat_change(on_repeat_change)
  }

  /// Called on a press of the repeat button, with what it is to be: off, all, one, and off again
  /// ([`AudioRepeat::next`]). Each call adds a handler.
  pub fn on_repeat_change(mut self, handler: impl Fn(AudioRepeat) + Send + Sync + 'static) -> Self {
    self.repeat.tell.push(Arc::new(handler));
    self
  }

  /// Whether what it would play is being fetched: lines in place of the texts, an empty bar, a spinner in the play
  /// button, previous and next dimmed.
  pub fn loading(mut self, loading: bool) -> Self {
    self.loading = loading;
    self
  }

  /// What it plays: a file, bytes, a resource of the app or an address. A player in a tree (given to it as it
  /// is, or mounted) opens it as a sound ([`Media::sound`](crate::media::Media::sound)) with the app's loader and
  /// plays it through the system's output: where it is, how long it is, and whether it plays, loads or failed
  /// are the media's, in place of what the app says of them. A sound at an address is loaded when it is first
  /// played, once, and kept ([`sound_cache`](crate::media::sound_cache)): while it loads, the player is drawn
  /// loading, and until then it is drawn from the length the app gave. A file, bytes and a sound that is kept
  /// already are opened as soon as the player is in a tree.
  ///
  /// What a person does on a control is told to the app's handlers and done to the media: play and pause, the
  /// seek bar, the volume, the mute and the speed. An app that holds `playing`, `volume`, `muted` or `rate`
  /// moves the media by changing them. With [`AudioRepeat::One`] the track starts over at its end. "Try again"
  /// loads the source anew. With no source the player is drawn from the values it is given.
  ///
  /// ```
  /// use lurq::{core::Signal, resources::Source, ui::AudioPlayer};
  ///
  /// // The length is the sound's once it opened: what is said here is drawn until then.
  /// let player = AudioPlayer::new(Signal::new(0.0), 0.0)
  ///   .source(&Source::url("https://example.com/night-drive.mp3"))
  ///   .title("Night Drive")
  ///   .description("Aurora Lane");
  /// ```
  #[cfg(feature = "media")]
  pub fn source(mut self, source: &crate::resources::Source) -> Self {
    self.source = Some(source.clone());
    self
  }

  /// What went wrong, where it could not play: a line in the destructive color in place of the times (of the bar in
  /// the compact form and a voice message), with "Try again" where [`AudioPlayer::on_retry`] is given.
  pub fn error(mut self, error: Option<&str>) -> Self {
    self.error = error.map(Arc::from);
    self
  }

  /// Whether a voice message has not been heard yet: a dot at its end, and the whole waveform in the color of what
  /// is not played.
  pub fn unheard(mut self, unheard: bool) -> Self {
    self.unheard = unheard;
    self
  }

  /// Whether it takes no input: half its opacity.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// The title of what plays.
  pub fn title(mut self, title: &str) -> Self {
    self.title = Some(Arc::from(title));
    self
  }

  /// Who it is by, or where it is from, under the title.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// The artwork, cut to the box of the form: 64 in the card, 36 in the mini player. Where there is none, the
  /// `music` glyph on the muted fill stands for it.
  pub fn media(mut self, media: impl Into<Element>) -> Self {
    self.media = Some(media.into());
    self
  }

  /// The loudness of what plays, each from 0 to 1, one for each bar of a waveform: a voice message draws them, and
  /// the compact form draws them in place of the seek bar.
  pub fn peaks(mut self, peaks: &[f32]) -> Self {
    self.peaks = peaks.to_vec();
    self
  }

  /// Called on a press of previous. Each call adds a handler.
  pub fn on_previous(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_previous.push(Arc::new(handler));
    self
  }

  /// Called on a press of next. Each call adds a handler.
  pub fn on_next(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_next.push(Arc::new(handler));
    self
  }

  /// Called on a press of the card's "more" button. Each call adds a handler.
  pub fn on_more(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_more.push(Arc::new(handler));
    self
  }

  /// The ref the card's "more" button takes: what a menu of the app hangs from.
  pub fn more_anchor(mut self, anchor: ElementRef) -> Self {
    self.more_anchor = Some(anchor);
    self
  }

  /// Called on a press of the mini player's queue button. Each call adds a handler.
  pub fn on_queue(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_queue.push(Arc::new(handler));
    self
  }

  /// Called on a press of the mini player's expand button. Each call adds a handler.
  pub fn on_expand(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_expand.push(Arc::new(handler));
    self
  }

  /// Called on a press of "Try again", or of the retry button that stands for play after an error. Each call adds a
  /// handler.
  pub fn on_retry(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_retry.push(Arc::new(handler));
    self
  }

  /// What the player is called, to what reads the tree: "Audio player" unless told.
  pub fn label(mut self, label: &str) -> Self {
    self.words.label = Arc::from(label);
    self
  }

  /// What the play button is called while a press would play.
  pub fn play_label(mut self, label: &str) -> Self {
    self.words.play = Arc::from(label);
    self
  }

  /// What the play button is called while a press would pause.
  pub fn pause_label(mut self, label: &str) -> Self {
    self.words.pause = Arc::from(label);
    self
  }

  /// What the previous button is called.
  pub fn previous_label(mut self, label: &str) -> Self {
    self.words.previous = Arc::from(label);
    self
  }

  /// What the next button is called.
  pub fn next_label(mut self, label: &str) -> Self {
    self.words.next = Arc::from(label);
    self
  }

  /// What the shuffle button is called.
  pub fn shuffle_label(mut self, label: &str) -> Self {
    self.words.shuffle = Arc::from(label);
    self
  }

  /// What the repeat button is called.
  pub fn repeat_label(mut self, label: &str) -> Self {
    self.words.repeat = Arc::from(label);
    self
  }

  /// What the "more" button is called.
  pub fn more_label(mut self, label: &str) -> Self {
    self.words.more = Arc::from(label);
    self
  }

  /// What the speed button and a voice message's badge are called: "Playback speed" unless told.
  pub fn speed_label(mut self, label: &str) -> Self {
    self.words.speed = Arc::from(label);
    self
  }

  /// What the queue button is called.
  pub fn queue_label(mut self, label: &str) -> Self {
    self.words.queue = Arc::from(label);
    self
  }

  /// What the expand button is called.
  pub fn expand_label(mut self, label: &str) -> Self {
    self.words.expand = Arc::from(label);
    self
  }

  /// What the seek bar is called: "Seek" unless told.
  pub fn seek_label(mut self, label: &str) -> Self {
    self.words.seek = Arc::from(label);
    self
  }

  /// The words this player speaks in, in place of the app's ([`App::set_locale`](crate::app::App::set_locale)):
  /// the names of its controls, "Try again", and what its source says when it cannot play. Said before the
  /// words that are told one by one (`play_label` and the others), which stay what they were told.
  pub fn locale(mut self, locale: &crate::core::Locale) -> Self {
    self.words = Words::of(&locale.media);
    self.locale = Some(locale.media.clone());
    self
  }

  /// The words of the control that tries again after an error: "Try again" unless told.
  pub fn retry_text(mut self, text: &str) -> Self {
    self.words.retry = Arc::from(text);
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
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this player.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the play button as it is under the pointer, held down or focused from the keyboard, with nothing pointing
  /// at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the player's box; its controls have it with `-play`, `-seek`, `-previous`, `-next`, `-shuffle`,
  /// `-repeat`, `-more`, `-speed`, `-volume`, `-queue`, `-expand`, `-retry`, `-rate` and `-line` after it.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Puts the player in the tree of `ctx`'s component, in the theme of what is being rendered: its icon buttons
  /// with their tooltips, the speed menu, the mini player's form from its own width and the time over its line.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let mut element = ctx.mount::<AudioPlayerHost>(AudioPlayerProps { player: self });
    if let Some(key) = key {
      element = element.key(key);
    }
    element
  }

  /// Builds a still picture of the player in `theme`, for a test or a screenshot: no tooltips, no speed menu, a
  /// mini player as wide as it is given. An app gives the player to its tree, or mounts it
  /// ([`AudioPlayer::mount`]).
  pub fn build(self, theme: &Theme) -> Element {
    Self::render_view(self.view(theme, None, &Kept::default()))
  }

  fn render_view(view: AudioPlayerView) -> Element {
    match view.theme().renderer::<AudioPlayer>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  fn child_id(&self, what: &str) -> Option<String> {
    self.id.as_ref().map(|id| format!("{id}-{what}"))
  }

  /// The view of the player, its controls built, with their tooltips where there is a `ctx` to mount them in.
  fn view(self, theme: &Theme, mut ctx: Option<&mut Ctx>, kept: &Kept) -> AudioPlayerView {
    let variant = AudioPlayerVariant::of(self.variant);
    let failed = self.error.is_some();
    let still = self.loading || failed;
    let narrow = variant == AudioPlayerVariant::Mini && kept.narrow;
    let case = Case {
      variant: Some(self.variant),
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(AudioPlayerState::Playing, self.playing.value)
    .state(AudioPlayerState::Loading, self.loading)
    .state(AudioPlayerState::Error, failed)
    .state(AudioPlayerState::Disabled, self.disabled)
    .state(AudioPlayerState::Media, self.media.is_some())
    .state(AudioPlayerState::Unheard, self.unheard)
    .state(AudioPlayerState::Shuffle, self.shuffle.value)
    .state(AudioPlayerState::Narrow, narrow)
    .state(
      AudioPlayerState::Scrubbing,
      kept.scrub.as_ref().is_some_and(|scrub| scrub.get() >= 0.0),
    );
    let recipe = theme.resolve::<AudioPlayer>(&case, Some(&self.rules));
    let words = self.words.clone();
    let mut tip = |label: &str, element: Element| match ctx.as_deref_mut() {
      Some(ctx) => Tooltip::new(label).mount(ctx, element),
      None => element,
    };
    let position = if still { 0.0 } else { self.position.value };
    let toggle = {
      let (tell, next) = (self.playing.teller(), !self.playing.value);
      let blocked = self.disabled || still;
      move || {
        if !blocked {
          tell(next);
        }
      }
    };
    // Space plays and pauses while the focus is on the bar or the volume; a button takes its own Space.
    // The bar fills the room it is given; the volume is as wide as it is.
    let spaced = |element: Element, fill: bool| -> Element {
      let toggle = toggle.clone();
      let mut row = Row::new().min_width(Dimension::Px(0.0));
      if fill {
        row = row.width(Dimension::Pct(100.0));
      }
      row
        .on_key_down(move |event: KeyboardEvent| {
          if event.focus_within && event.key == " " && !event.text_input_focused {
            event.prevent_default();
            toggle();
          }
        })
        .child(element)
        .into()
    };
    let mut controls = AudioControls::default();

    // Play, or the retry button that stands for it after an error in a row.
    let retry_in_place = failed && matches!(variant, AudioPlayerVariant::Compact | AudioPlayerVariant::Voice);
    if retry_in_place {
      let mut retry = Button::new(&words.retry)
        .icon(icons::REFRESH)
        .icon_only(true)
        .shape(ButtonShape::Pill)
        .variant(ButtonVariant::Secondary)
        .size(if variant == AudioPlayerVariant::Voice {
          ButtonSize::Md
        } else {
          ButtonSize::Sm
        })
        .force(self.forced)
        .on_click({
          let retry = run(&self.on_retry);
          move |_| retry()
        });
      if let Some(id) = self.child_id("retry") {
        retry = retry.id(id);
      }
      controls.play = Some(tip(&words.retry, retry.build(theme)));
    } else {
      let size = match variant {
        AudioPlayerVariant::Full => PlayButtonSize::Lg,
        AudioPlayerVariant::Compact => PlayButtonSize::Sm,
        AudioPlayerVariant::Voice | AudioPlayerVariant::Mini => PlayButtonSize::Md,
      };
      let mut play = PlayButton::new()
        .playing(self.playing.value)
        .loading(self.loading)
        .disabled(failed)
        .size(size)
        .play_label(&words.play)
        .pause_label(&words.pause)
        .force(self.forced);
      if !self.loading && !failed {
        let tell = self.playing.teller();
        play = play.on_change(move |next| tell(next));
      }
      if matches!(variant, AudioPlayerVariant::Voice | AudioPlayerVariant::Mini) {
        // The board's 36, on the size whose glyph is 16.
        play = play.style(PlayButtonPart::Root, |s| s.square(36.0));
      }
      if let Some(id) = self.child_id("play") {
        play = play.id(id);
      }
      let label = if self.playing.value { &words.pause } else { &words.play };
      controls.play = Some(tip(label, play.build(theme)));
    }

    // The bar, or the waveform that stands for it.
    let wave = !self.peaks.is_empty() && matches!(variant, AudioPlayerVariant::Compact | AudioPlayerVariant::Voice);
    if matches!(
      variant,
      AudioPlayerVariant::Full | AudioPlayerVariant::Compact | AudioPlayerVariant::Voice
    ) && !(failed && variant != AudioPlayerVariant::Full)
    {
      let duration = self.duration.max(0.0);
      let (tell, ends) = (self.position.teller(), self.on_change_end.clone());
      let element = if wave {
        let progress = if duration > 0.0 { position / duration } else { 0.0 };
        let end = {
          let ends = ends.clone();
          move |fraction: f64| {
            for handler in &ends {
              handler(fraction * duration);
            }
          }
        };
        let mut wave = Waveform::controlled(progress, &self.peaks, move |fraction| tell(fraction * duration))
          .on_change_end(end)
          // Loading and after an error nothing of it is played: every bar in the color of what is not.
          .unheard(self.unheard || still)
          .label(&words.seek)
          .disabled(still);
        if let Some(id) = self.child_id("seek") {
          wave = wave.id(id);
        }
        wave.build(theme)
      } else {
        let mut bar = SeekBar::controlled(position, duration, move |to| tell(to))
          .on_change_end(move |to| {
            for handler in &ends {
              handler(to);
            }
          })
          .label(&words.seek)
          // Loading and after an error nothing is loaded: the track alone, with no thumb.
          .empty(still)
          .force(None);
        if let (Some(buffered), false) = (self.buffered, still) {
          bar = bar.buffered(buffered);
        }
        if let Some(id) = self.child_id("seek") {
          bar = bar.id(id);
        }
        bar.build(theme)
      };
      controls.seek = Some(spaced(element, true));
    }

    // Previous and next.
    if matches!(variant, AudioPlayerVariant::Full | AudioPlayerVariant::Mini) {
      let size = if variant == AudioPlayerVariant::Full {
        ButtonSize::Md
      } else {
        ButtonSize::Sm
      };
      let mut icon_button =
        |what: &str, label: &Arc<str>, icon: &'static str, size: ButtonSize, commands: &[Command], off: bool| {
          let command = run(commands);
          let mut button = Button::new(label)
            .icon(icon)
            .icon_only(true)
            .variant(ButtonVariant::Ghost)
            .size(size)
            .disabled(off)
            .on_click(move |_| command());
          if let Some(id) = self.child_id(what) {
            button = button.id(id);
          }
          tip(label, button.build(theme))
        };
      controls.previous = (!narrow).then(|| {
        icon_button(
          "previous",
          &words.previous,
          icons::SKIP_BACK,
          size,
          &self.on_previous,
          self.loading,
        )
      });
      controls.next = Some(icon_button(
        "next",
        &words.next,
        icons::SKIP_FORWARD,
        size,
        &self.on_next,
        self.loading,
      ));
      if variant == AudioPlayerVariant::Full {
        let more = {
          let command = run(&self.on_more);
          let mut button = Button::new(&words.more)
            .icon(icons::MORE_HORIZONTAL)
            .icon_only(true)
            .variant(ButtonVariant::Ghost)
            .size(ButtonSize::Sm)
            .rules(&recipe.rules_for(AudioPlayerPart::More, ButtonPart::Root))
            .on_click(move |_| command());
          if let Some(id) = self.child_id("more") {
            button = button.id(id);
          }
          if let Some(anchor) = &self.more_anchor {
            button = button.ref_element(anchor.clone());
          }
          button.build(theme)
        };
        controls.more = Some(tip(&words.more, more));
      } else if !narrow {
        controls.queue = Some(icon_button(
          "queue",
          &words.queue,
          icons::LIST,
          ButtonSize::Sm,
          &self.on_queue,
          false,
        ));
        controls.expand = Some(icon_button(
          "expand",
          &words.expand,
          icons::CHEVRON_UP,
          ButtonSize::Sm,
          &self.on_expand,
          false,
        ));
      }
    }

    // Shuffle and repeat, and the speed and the volume under them.
    if variant == AudioPlayerVariant::Full {
      let mut shuffle = Toggle::new(&words.shuffle)
        .icon(icons::SHUFFLE)
        .icon_only(true)
        .size(ButtonSize::Sm)
        .on(self.shuffle.value)
        .rules(&recipe.rules_for(AudioPlayerPart::Shuffle, TogglePart::Root))
        .on_change({
          let tell = self.shuffle.teller();
          move |on| tell(on)
        });
      if let Some(id) = self.child_id("shuffle") {
        shuffle = shuffle.id(id);
      }
      controls.shuffle = Some(tip(&words.shuffle, shuffle.build(theme)));
      let repeat = self.repeat.value;
      let mut toggle = Toggle::new(&words.repeat)
        .icon(if repeat == AudioRepeat::One {
          icons::REPEAT_ONE
        } else {
          icons::REPEAT
        })
        .icon_only(true)
        .size(ButtonSize::Sm)
        .on(repeat != AudioRepeat::Off)
        .rules(&recipe.rules_for(AudioPlayerPart::Repeat, TogglePart::Root))
        .on_change({
          let tell = self.repeat.teller();
          move |_| tell(repeat.next())
        });
      if let Some(id) = self.child_id("repeat") {
        toggle = toggle.id(id);
      }
      let mut repeat_element = toggle.build(theme);
      crate::node::NodeUpdate::describe(
        &mut *repeat_element.node,
        Arc::from("a11y_value"),
        Arc::from(repeat.as_str()),
      );
      controls.repeat = Some(tip(&words.repeat, repeat_element));

      let rate = self.rate.value;
      let mut speed = Button::new(&audio_rate_text(rate))
        .variant(ButtonVariant::Ghost)
        .size(ButtonSize::Sm)
        .rules(&recipe.rules_for(AudioPlayerPart::Speed, ButtonPart::Root));
      if let Some((open, anchor)) = &kept.speed {
        let open = open.clone();
        speed = speed
          .ref_element(anchor.clone())
          .on_click(move |_| open.update(|open| *open = !*open));
      }
      if let Some(id) = self.child_id("speed") {
        speed = speed.id(id);
      }
      let mut speed = speed.build(theme);
      crate::node::NodeUpdate::describe(&mut *speed.node, Arc::from("a11y_name"), words.speed.clone());
      crate::node::NodeUpdate::describe(&mut *speed.node, Arc::from("a11y_haspopup"), Arc::from("menu"));
      controls.speed = Some(tip(&words.speed, speed));
      if let Some((open, anchor)) = &kept.speed {
        let tell = self.rate.teller();
        controls.speed_menu = Some(
          MediaSpeedMenu::new(open.clone(), anchor.clone(), rate as f32)
            .on_change(move |rate| tell(rate))
            .build(theme),
        );
      }
    }
    if matches!(variant, AudioPlayerVariant::Full | AudioPlayerVariant::Mini) && !narrow {
      let mut volume = MediaVolume::controlled(self.volume.value, {
        let tell = self.volume.teller();
        move |to| tell(to)
      })
      .muted_controlled(self.muted.value, {
        let tell = self.muted.teller();
        move |muted| tell(muted)
      });
      if variant == AudioPlayerVariant::Full {
        volume = volume.length(96.0);
      }
      // A mini player with no room for the slider keeps the button that mutes.
      if variant == AudioPlayerVariant::Mini && kept.fold >= 1 {
        volume = volume.collapsed(true);
      }
      if let Some(id) = self.child_id("volume") {
        volume = volume.id(id);
      }
      controls.volume = Some(spaced(volume.build(theme), false));
    }

    // The time of the mini player, which a press turns between what is gone and what is left.
    if variant == AudioPlayerVariant::Mini && !narrow && kept.fold < 2 {
      let duration = (!still && self.duration > 0.0).then_some(self.duration);
      let mut time =
        MediaTime::new(position, duration).remaining(kept.remaining.as_ref().is_some_and(|left| left.get()));
      if let Some(remaining) = &kept.remaining {
        let remaining = remaining.clone();
        time = time.on_click(move |_: MouseEvent| remaining.update(|left| *left = !*left));
      }
      controls.time = Some(time.build(theme));
    }

    // The badge of a voice message's speed, which a press steps while it plays.
    if variant == AudioPlayerVariant::Voice && self.playing.value && !still {
      let rate = self.rate.value;
      let part = recipe.part(AudioPlayerPart::Rate);
      let next = VOICE_RATES
        .iter()
        .map(|rate| f64::from(*rate))
        .find(|offered| *offered > rate + 0.01)
        .unwrap_or(f64::from(VOICE_RATES[0]));
      let tell = self.rate.teller();
      let text = audio_rate_text(rate);
      let mut badge = pressable(&part, false)
        .describe("a11y_name", format!("{}: {text}", words.speed))
        .on_click(move |_| tell(next))
        .child(part.text(&text).nowrap());
      if let Some(id) = self.child_id("rate") {
        badge = badge.id(id);
      }
      controls.rate = Some(tip(&words.speed, badge.into()));
    }

    // "Try again" at the end of the card's error.
    if failed && variant == AudioPlayerVariant::Full && !self.on_retry.is_empty() {
      let retry = run(&self.on_retry);
      let mut link = Link::new(&words.retry)
        .rules(&recipe.rules_for(AudioPlayerPart::Retry, LinkPart::Label))
        .on_click(move |_| retry());
      if let Some(id) = self.child_id("retry") {
        link = link.id(id);
      }
      controls.retry = Some(link.build(theme));
    }

    AudioPlayerView {
      variant,
      title: self.title,
      description: self.description,
      media: self.media,
      position,
      duration: self.duration,
      playing: self.playing.value,
      loading: self.loading,
      error: self.error,
      unheard: self.unheard,
      disabled: self.disabled,
      narrow,
      label: words.label,
      recipe,
      controls,
      line: (variant == AudioPlayerVariant::Mini).then(|| Line {
        position,
        duration: if still { 0.0 } else { self.duration },
        tell: self.position.tell.clone(),
        ends: self.on_change_end.clone(),
        scrub: kept.scrub.clone(),
        root: kept.root.clone(),
        id: self.id.as_ref().map(|id| format!("{id}-line")),
      }),
      id: self.id,
    }
  }
}

/// What a mounted player keeps: none of it in the still form.
#[derive(Default)]
struct Kept {
  /// Whether the speed menu is open, and the button it hangs from.
  speed: Option<(Signal<bool>, ElementRef)>,
  /// The player's box, which the mini player measures and its line is pointed on.
  root: Option<ElementRef>,
  /// Whether a mini player is under 480 wide.
  narrow: bool,
  /// What the width of a mini player folds away at its end: see [`mini_fold`].
  fold: u8,
  /// Where the pointer is on the mini player's line, from its start, in pixels: below nothing where it is not.
  scrub: Option<Signal<f32>>,
  /// Whether the mini player's time reads what is left.
  remaining: Option<Signal<bool>>,
}

/// What a mini player's progress line is given.
struct Line {
  position: f64,
  duration: f64,
  tell: Vec<Tell<f64>>,
  ends: Vec<Tell<f64>>,
  scrub: Option<Signal<f32>>,
  root: Option<ElementRef>,
  id: Option<String>,
}

/// The controls of a player, built for its form: what a renderer lays out. Each is there where the form has it.
#[derive(Default)]
pub struct AudioControls {
  pub play: Option<Element>,
  pub seek: Option<Element>,
  pub previous: Option<Element>,
  pub next: Option<Element>,
  pub shuffle: Option<Element>,
  pub repeat: Option<Element>,
  pub more: Option<Element>,
  pub speed: Option<Element>,
  pub speed_menu: Option<Element>,
  pub volume: Option<Element>,
  pub queue: Option<Element>,
  pub expand: Option<Element>,
  pub time: Option<Element>,
  pub rate: Option<Element>,
  pub retry: Option<Element>,
}

/// An audio player as a renderer sees it: what it was given, the style of its parts, and its controls built.
pub struct AudioPlayerView {
  pub variant: AudioPlayerVariant,
  pub title: Option<Arc<str>>,
  pub description: Option<Arc<str>>,
  pub media: Option<Element>,
  /// Where it is, in seconds: nothing while it loads or after an error.
  pub position: f64,
  pub duration: f64,
  pub playing: bool,
  pub loading: bool,
  pub error: Option<Arc<str>>,
  pub unheard: bool,
  pub disabled: bool,
  /// Whether a mini player is under 480 wide.
  pub narrow: bool,
  pub label: Arc<str>,
  /// The style of each part of this player.
  pub recipe: ResolvedRecipe,
  pub controls: AudioControls,
  line: Option<Line>,
  id: Option<Arc<str>>,
}

impl AudioPlayerView {
  /// The style of a part, as this player is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// A time as the player reads it, "-:--" while nothing is known of it.
  pub fn time_text(&self, seconds: f64) -> String {
    if self.loading || self.duration <= 0.0 || !self.duration.is_finite() {
      "-:--".to_owned()
    } else {
      media_time_text(seconds, self.duration >= 3600.0)
    }
  }

  /// A text on one line in the style of `part`, cut with an ellipsis where its room is narrower.
  fn line_of(&self, part: AudioPlayerPart, text: &str) -> Column {
    let part = self.part(part);
    Column::new()
      .width(Dimension::Pct(100.0))
      .min_width(Dimension::Px(0.0))
      .part(&part)
      .child(
        part
          .text(text)
          .nowrap()
          .text_overflow(TextOverflow::Elipsis)
          .width(Dimension::Pct(100.0)),
      )
  }

  /// A text on one line in the style of `part`, as wide as it is.
  fn word(&self, part: AudioPlayerPart, text: &str) -> Row {
    let part = self.part(part);
    Row::new().part(&part).flex_shrink(0.0).child(part.text(text).nowrap())
  }

  /// The artwork in its box, or the `music` glyph on the muted fill.
  pub fn media(&mut self) -> Row {
    let part = self.part(AudioPlayerPart::Media);
    let mut held = Row::new()
      .part(&part)
      .flex_shrink(0.0)
      .clip()
      .align_items(Alignment::Center)
      .justify(Justify::Center);
    held = match self.media.take() {
      Some(media) => held.child(media),
      None => held.child(glyph(
        &self.part(AudioPlayerPart::MediaIcon),
        Name::from(icons::MUSIC),
        false,
      )),
    };
    held
  }

  /// The title over who it is by, cut where there is no room; two lines that stand for them while it loads.
  pub fn text(&self) -> Column {
    let mut column = Column::new()
      .flex(1.0)
      .min_width(Dimension::Px(0.0))
      .part(&self.part(AudioPlayerPart::Text));
    if self.loading {
      let skeleton = self.part(AudioPlayerPart::Skeleton);
      let theme = self.theme();
      let lines = if self.variant == AudioPlayerVariant::Mini {
        [(96.0, 10.0), (64.0, 8.0)]
      } else {
        [(140.0, 12.0), (96.0, 10.0)]
      };
      for (width, height) in lines {
        column = column.child(
          // A line is as long as the board draws it, and no longer than the room beside the artwork.
          Row::new()
            .part(&skeleton)
            .width(width)
            .max_width(Dimension::Pct(100.0))
            .child(Skeleton::line().width(Length::Fill).height(height).build(theme)),
        );
      }
      return column;
    }
    if let Some(title) = &self.title {
      column = column.child(self.line_of(AudioPlayerPart::Title, title));
    }
    match (&self.error, &self.description) {
      (Some(error), _) if self.variant == AudioPlayerVariant::Mini => {
        column = column.child(self.line_of(AudioPlayerPart::ErrorText, error));
      }
      (_, Some(description)) => column = column.child(self.line_of(AudioPlayerPart::Description, description)),
      _ => {}
    }
    column
  }

  /// The line of the error: its icon and its words, with "Try again" at its end in the card.
  pub fn error_line(&mut self) -> Option<Row> {
    let error = self.error.clone()?;
    let mut said = Row::new()
      .flex(1.0)
      .min_width(Dimension::Px(0.0))
      .align_items(Alignment::Center)
      .part(&self.part(AudioPlayerPart::Error))
      .describe("a11y_role", "alert");
    if self.variant == AudioPlayerVariant::Full {
      said = said.child(glyph(
        &self.part(AudioPlayerPart::ErrorIcon),
        Name::from(icons::ALERT_CIRCLE),
        false,
      ));
    }
    said = said.child(
      Row::new()
        .flex(1.0)
        .min_width(Dimension::Px(0.0))
        .child(self.line_of(AudioPlayerPart::ErrorText, &error)),
    );
    let mut row = Row::new()
      .flex(1.0)
      .min_width(Dimension::Px(0.0))
      .align_items(Alignment::Center)
      .justify(Justify::SpaceBetween)
      .child(said);
    if let Some(retry) = self.controls.retry.take() {
      row = row.child(Row::new().flex_shrink(0.0).child(retry));
    }
    Some(row)
  }

  /// The one time of a row: what is gone while it plays or has played, how long it is before.
  pub fn time(&self) -> Row {
    let started = self.playing || self.position > 0.0;
    let text = self.time_text(if started { self.position } else { self.duration });
    self.word(AudioPlayerPart::Time, &text)
  }

  /// The mini player's progress line along its top edge, with what the pointer does on it: the time in a tooltip
  /// over where it is, a press seeks there.
  pub fn line(&mut self) -> Option<Element> {
    let line = self.line.take()?;
    let fraction = if line.duration > 0.0 {
      (line.position / line.duration).clamp(0.0, 1.0) as f32
    } else {
      0.0
    };
    let track = self.part(AudioPlayerPart::Track);
    let fill = self.part(AudioPlayerPart::Fill);
    let rail = Row::new().part(&track).width(Dimension::Pct(100.0)).child(
      Row::new()
        .part(&fill)
        .height(Dimension::Pct(100.0))
        .width(Dimension::Pct(fraction * 100.0)),
    );
    // The room the pointer finds the line in: 8 from the top edge, over the bar's empty top.
    let mut hit = Stack::new()
      .stack_align(StackAlignment::TopStart)
      .width(Dimension::Pct(100.0))
      .height(8.0)
      .overflow_visible()
      .child(rail)
      .describe("a11y_role", "progressbar")
      .describe("a11y_valuenow", format!("{}", line.position.floor()));
    if let Some(id) = &line.id {
      hit = hit.id(id.clone());
    }
    if let (Some(scrub), Some(root)) = (&line.scrub, &line.root)
      && line.duration > 0.0
    {
      let at = scrub.get();
      let duration = line.duration;
      let point = {
        let (scrub, root) = (scrub.clone(), root.clone());
        move |event: MouseEvent| {
          let bounds = root.bounds();
          let x = (event.x - bounds.x).clamp(0.0, bounds.width.max(0.0));
          if (scrub.get_untracked() - x).abs() >= 0.5 {
            scrub.set(x);
          }
        }
      };
      let leave = {
        let scrub = scrub.clone();
        move || {
          if scrub.get_untracked() >= 0.0 {
            scrub.set(-1.0);
          }
        }
      };
      let seek = {
        let (root, tell, ends) = (root.clone(), line.tell.clone(), line.ends.clone());
        move |event: MouseEvent| {
          let bounds = root.bounds();
          if bounds.width <= 0.0 {
            return;
          }
          let to = f64::from(((event.x - bounds.x) / bounds.width).clamp(0.0, 1.0)) * duration;
          for handler in tell.iter().chain(ends.iter()) {
            handler(to);
          }
        }
      };
      hit = hit
        .cursor(CursorIcon::Pointer)
        .on_mouse_move(point)
        .on_mouse_leave(leave)
        .on_click(seek);
      if at >= 0.0 {
        let width = root.bounds().width.max(1.0);
        let seconds = f64::from(at / width) * duration;
        let text = media_time_text(seconds, duration >= 3600.0);
        hit = hit.child(
          Row::new()
            .offset(at, -4.0)
            .offset_by_size(-0.5, -1.0)
            .child(Tooltip::new(&text).build(self.theme())),
        );
      }
    }
    Some(hit.into())
  }

  /// How the default renderer builds a player: the form its variant says.
  pub fn render(self) -> Element {
    match self.variant {
      AudioPlayerVariant::Full => self.render_full(),
      AudioPlayerVariant::Compact | AudioPlayerVariant::Voice => self.render_row(),
      AudioPlayerVariant::Mini => self.render_mini(),
    }
  }

  /// What every form's box has: its name to what reads the tree, its id, and no input while disabled.
  fn finish(&self, element: impl Into<Element>) -> Element {
    let mut element: Element = element.into();
    let node = &mut *element.node;
    crate::node::NodeUpdate::describe(node, Arc::from("a11y_role"), Arc::from("group"));
    crate::node::NodeUpdate::describe(node, Arc::from("a11y_name"), self.label.clone());
    if self.disabled {
      crate::node::NodeUpdate::disabled(node, true);
    }
    if let Some(id) = &self.id {
      crate::node::NodeUpdate::id(node, id.clone());
    }
    element
  }

  fn render_full(mut self) -> Element {
    let header = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .part(&self.part(AudioPlayerPart::Header))
      .child(self.media())
      .child(self.text())
      .with_children(self.controls.more.take());
    let times = match self.error_line() {
      Some(error) => Row::new().width(Dimension::Pct(100.0)).child(error),
      None => Row::new()
        .width(Dimension::Pct(100.0))
        .justify(Justify::SpaceBetween)
        .part(&self.part(AudioPlayerPart::Times))
        .child(self.word(AudioPlayerPart::Elapsed, &self.time_text(self.position)))
        .child(self.word(AudioPlayerPart::Total, &self.time_text(self.duration))),
    };
    let progress = Column::new()
      .width(Dimension::Pct(100.0))
      .part(&self.part(AudioPlayerPart::Progress))
      .with_children(
        self
          .controls
          .seek
          .take()
          .map(|seek| Column::new().width(Dimension::Pct(100.0)).child(seek)),
      )
      .child(times);
    let transport = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .part(&self.part(AudioPlayerPart::Transport))
      .with_children(self.controls.shuffle.take())
      .with_children(self.controls.previous.take())
      .with_children(self.controls.play.take())
      .with_children(self.controls.next.take())
      .with_children(self.controls.repeat.take());
    let footer = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .justify(Justify::SpaceBetween)
      .part(&self.part(AudioPlayerPart::Footer))
      .with_children(self.controls.volume.take())
      .child(
        Row::new()
          .flex_shrink(0.0)
          .with_children(self.controls.speed.take())
          .with_children(self.controls.speed_menu.take()),
      );
    let root = Column::new()
      .part(&self.part(AudioPlayerPart::Root))
      .child(header)
      .child(progress)
      .child(transport)
      .child(footer);
    self.finish(root)
  }

  fn render_row(mut self) -> Element {
    let mut root = Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(AudioPlayerPart::Root))
      .with_children(self.controls.play.take());
    match self.error_line() {
      Some(error) => root = root.child(error),
      None => {
        root = root.with_children(
          self
            .controls
            .seek
            .take()
            .map(|seek| Row::new().flex(1.0).min_width(Dimension::Px(0.0)).child(seek)),
        );
        root = root.child(self.time());
        if let Some(rate) = self.controls.rate.take() {
          root = root.child(rate);
        } else if self.variant == AudioPlayerVariant::Voice && self.unheard && !self.playing {
          root = root.child(
            Row::new()
              .part(&self.part(AudioPlayerPart::Dot))
              .describe("a11y_name", "Not heard"),
          );
        }
      }
    }
    self.finish(root)
  }

  fn render_mini(mut self) -> Element {
    let header = Row::new()
      .flex(1.0)
      .min_width(Dimension::Px(0.0))
      .align_items(Alignment::Center)
      .part(&self.part(AudioPlayerPart::Header))
      .child(self.media())
      .child(self.text());
    let transport = Row::new()
      .flex_shrink(0.0)
      .align_items(Alignment::Center)
      .part(&self.part(AudioPlayerPart::Transport))
      .with_children(self.controls.previous.take())
      .with_children(self.controls.play.take())
      .with_children(self.controls.next.take());
    let mut bar = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .part(&self.part(AudioPlayerPart::Bar))
      .child(header)
      .child(transport);
    if !self.narrow {
      bar = bar.child(
        Row::new()
          .flex(1.0)
          .min_width(Dimension::Px(0.0))
          .align_items(Alignment::Center)
          .justify(Justify::End)
          .part(&self.part(AudioPlayerPart::Tools))
          .with_children(self.controls.time.take())
          .with_children(self.controls.volume.take())
          .with_children(self.controls.queue.take())
          .with_children(self.controls.expand.take()),
      );
    }
    let root_ref = self.line.as_ref().and_then(|line| line.root.clone());
    let line = self.line();
    let mut root = Stack::new()
      .stack_align(StackAlignment::TopStart)
      .part(&self.part(AudioPlayerPart::Root))
      .child(
        Column::new()
          .width(Dimension::Pct(100.0))
          .part(&self.part(AudioPlayerPart::Edge))
          .child(Row::new().height(2.0))
          .child(bar),
      )
      .with_children(line);
    if let Some(root_ref) = root_ref {
      root = root.ref_element(root_ref);
    }
    self.finish(root)
  }
}

/// A player in a tree is its component, declared: the component that renders the tree mounts it.
impl crate::app::component::IntoComponent for AudioPlayer {
  type Component = AudioPlayerHost;

  fn into_props(self) -> AudioPlayerProps {
    AudioPlayerProps { player: self }
  }

  fn component_key(&self) -> Option<Arc<str>> {
    self.key.clone()
  }
}

/// The mounted player: what its host is given.
#[doc(hidden)]
#[derive(Clone)]
pub struct AudioPlayerProps {
  player: AudioPlayer,
}

/// The player is built anew by every render of what holds it, so props are never the same as before.
impl PartialEq for AudioPlayerProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl crate::app::component::DevtoolsInspectable for AudioPlayerProps {}

/// What a mounted player keeps from one render to the next.
#[doc(hidden)]
pub struct AudioPlayerHost {
  speed_open: Signal<bool>,
  speed_anchor: ElementRef,
  root: ElementRef,
  narrow: Signal<bool>,
  fold: Signal<u8>,
  scrub: Signal<f32>,
  remaining: Signal<bool>,
  #[cfg(feature = "media")]
  played: Played,
}

/// What a mounted player with a source keeps: the media it opened, and what renders the player again for it.
#[cfg(feature = "media")]
struct Played {
  /// The media, with the key of the source it was opened from: another source is another media.
  media: Arc<Mutex<Option<(u64, crate::media::Media)>>>,
  /// Counted up when the media changed, and while it plays: the player reads it and is rendered again.
  beat: Signal<u64>,
  /// Whether it plays: the clock that counts the beat runs while it does.
  plays: Arc<AtomicBool>,
  /// What the app last said of playing, of the speed, of how loud it is and of whether it is silenced: a
  /// change of them moves the media.
  said: Arc<Mutex<Said>>,
  /// The key of the source a person asked to play before it was opened: a sound at an address is opened, and
  /// so loaded, when it is first played.
  asked: Arc<Mutex<Option<u64>>>,
  _clock: crate::app::ctx::Interval,
}

#[cfg(feature = "media")]
type Said = (Option<bool>, Option<f64>, Option<f64>, Option<bool>);

#[cfg(feature = "media")]
impl Played {
  fn new(ctx: &mut Ctx) -> Self {
    let (beat, plays) = (ctx.signal(0_u64), Arc::new(AtomicBool::new(false)));
    // While it plays the time and the seek bar move: four times a second is what they show.
    let clock = ctx.create_interval(std::time::Duration::from_millis(250), {
      let (beat, plays) = (beat.clone(), plays.clone());
      move || {
        if plays.load(Ordering::Relaxed) {
          beat.update(|count| *count += 1);
        }
      }
    });
    clock.start();
    Self {
      media: Arc::default(),
      beat,
      plays,
      said: Arc::default(),
      asked: Arc::default(),
      _clock: clock,
    }
  }
}

/// `player` as it is while its source plays: the media of the source, opened once, is where it is and how long,
/// whether it plays, loads or failed; and what a person does on a control is done to the media too.
#[cfg(feature = "media")]
fn playing(mut player: AudioPlayer, played: &Played, ctx: &mut Ctx) -> AudioPlayer {
  use crate::media::Media;
  let mut held = played.media.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
  let Some(source) = player.source.take() else {
    // No source any more: the media ends with what held it.
    *held = None;
    played.plays.store(false, Ordering::Relaxed);
    return player;
  };
  // Rendered again when the media says it changed, and as its clock goes.
  played.beat.get();
  let key = source.cache_key();
  // The app's loader, with its resource root and what it allows; a tree with no app has a loader of its own.
  let loader = ctx
    .app_handle()
    .map_or_else(crate::resources::Loader::new, |app| app.loader().clone());
  if held.as_ref().is_none_or(|(of, _)| *of != key) {
    // A sound at an address is not loaded for being drawn: a list of a hundred players loads nothing until
    // one of them is played. What is kept already, a file and bytes are there, and are opened at once.
    let asked = *played.asked.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) == Some(key);
    let there = !source.is_remote() || crate::media::sound_cache().holds(&source);
    if !(there || asked || player.playing.value) {
      *held = None;
      played.plays.store(false, Ordering::Relaxed);
      let (asked, beat) = (played.asked.clone(), played.beat.clone());
      player.playing.tell.push(Arc::new(move |to| {
        if to {
          *asked.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(key);
          beat.update(|count| *count += 1);
        }
      }));
      return player;
    }
    let media = Media::sound(&loader, &source);
    let beat = played.beat.clone();
    media.on_change(move || beat.update(|count| *count += 1));
    // The press that opened it is a press of play: it plays once it is loaded.
    if asked {
      media.play();
    }
    *held = Some((key, media));
    *played.said.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Said::default();
  }
  let media = held
    .as_ref()
    .map(|(_, media)| media.clone())
    .expect("the media just held");
  drop(held);
  // What the app says of playing, of the speed and of the sound moves the media when it changes: an app that
  // holds them plays, pauses, and changes the speed and the volume by setting them.
  {
    let mut said = played.said.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if said.0 != Some(player.playing.value) {
      // A player that starts paused has nothing to tell a media that opens paused.
      if player.playing.value {
        media.play();
      } else if said.0.is_some() {
        media.pause();
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
  }
  // What a person does on a control is done to the media, after the app's own handlers heard of it.
  let again = {
    let beat = played.beat.clone();
    move || beat.update(|count| *count += 1)
  };
  player.playing.tell.push({
    let (media, again) = (media.clone(), again.clone());
    Arc::new(move |to| {
      if to {
        media.play();
      } else {
        media.pause();
      }
      again();
    })
  });
  player.position.tell.push({
    let (media, again) = (media.clone(), again.clone());
    Arc::new(move |to| {
      media.seek(to);
      again();
    })
  });
  player.rate.tell.push({
    let (media, again) = (media.clone(), again.clone());
    Arc::new(move |to| {
      media.set_rate(to);
      again();
    })
  });
  player.volume.tell.push({
    let (media, again) = (media.clone(), again.clone());
    Arc::new(move |to| {
      media.set_volume(to as f32);
      again();
    })
  });
  player.muted.tell.push({
    let (media, again) = (media.clone(), again.clone());
    Arc::new(move |to| {
      media.set_muted(to);
      again();
    })
  });
  // Trying again loads and opens the source anew: a load that failed stays failed until it is forgotten.
  player.on_retry.push({
    let (held, again) = (played.media.clone(), again);
    Arc::new(move || {
      loader.forget(&source);
      *held.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
      again();
    })
  });
  let state = media.state();
  // One track again and again: at its end it starts over. The other ways of repeating are of a queue, and the
  // app's.
  if state.ended && player.repeat.value == AudioRepeat::One && state.error.is_none() {
    media.play();
  }
  played.plays.store(state.playing, Ordering::Relaxed);
  player.playing.value = state.playing;
  player.position.value = state.position;
  player.rate.value = state.rate;
  player.volume.value = f64::from(state.volume);
  player.muted.value = state.muted;
  if let Some(duration) = state.duration {
    player.duration = duration;
    // All of a sound is there to play once it opened: its bar is loaded to its end, unless the app says how far.
    player.buffered = player.buffered.or(Some(duration));
  }
  player.loading = player.loading || state.waiting;
  // Why it cannot play, in the words of the locale: a short line, which the times give their room to.
  if let (None, Some(why)) = (&player.error, &state.error) {
    player.error = Some(match &player.locale {
      Some(words) => words.error(why),
      None => super::current_locale().media.error(why),
    });
  }
  player
}

/// Sets `signal` to `value` where it is not that already: a signal set renders again what reads it.
fn put(signal: &Signal<bool>, value: bool) {
  if signal.get_untracked() != value {
    signal.set(value);
  }
}

impl Component for AudioPlayerHost {
  type Props = AudioPlayerProps;

  fn create(ctx: &mut Ctx) -> Self {
    let (root, narrow, fold) = (ElementRef::new(), ctx.signal(false), ctx.signal(0_u8));
    // The form of a mini player follows its own width, from its first frame.
    root.observe_rect_before_paint({
      let (narrow, fold) = (narrow.clone(), fold.clone());
      move |rect| {
        put(&narrow, rect.width < AUDIO_MINI_NARROW);
        let now = mini_fold(rect.width);
        if fold.get_untracked() != now {
          fold.set(now);
        }
      }
    });
    Self {
      speed_open: ctx.signal(false),
      speed_anchor: ElementRef::new(),
      root,
      narrow,
      fold,
      scrub: ctx.signal(-1.0),
      remaining: ctx.signal(false),
      #[cfg(feature = "media")]
      played: Played::new(ctx),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let player = ctx.props::<AudioPlayerProps>().player.clone();
    #[cfg(feature = "media")]
    let player = playing(player, &self.played, ctx);
    let mini = AudioPlayerVariant::of(player.variant) == AudioPlayerVariant::Mini;
    let kept = Kept {
      speed: Some((self.speed_open.clone(), self.speed_anchor.clone())),
      root: Some(self.root.clone()),
      narrow: mini && self.narrow.get(),
      fold: if mini { self.fold.get() } else { 0 },
      scrub: mini.then(|| self.scrub.clone()),
      remaining: Some(self.remaining.clone()),
    };
    let theme = current_theme();
    let view = player.view(&theme, Some(ctx), &kept);
    AudioPlayer::render_view(view)
  }
}

impl crate::ui::Styled for AudioPlayer {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for AudioPlayer {
  const NAME: &'static str = "audio_player";
  type Renderer = AudioPlayerRenderer;

  fn parts() -> crate::ui::Declared {
    AudioPlayerPart::declared()
  }

  fn states() -> crate::ui::Declared {
    AudioPlayerState::declared()
  }

  /// The design's audio player (`pencil/lurq-design-system.pen`, Audio Player `nKkbL`).
  ///
  /// The card (`QCBpo`): 360 wide, the card's fill edged in the border's color, `radius-xl`, padded 20 with 16
  /// between its rows. The track row 12 apart: the artwork 64 with `radius-md` (the `music` glyph 24 in the muted
  /// ink on the muted fill where there is none), the title 14/600 over who it is by 13 in the muted ink, and the
  /// "more" button 32. The seek bar with the elapsed and the total time 6 under its ends, mono 11 in the muted ink.
  /// The transport 8 apart: shuffle 32, previous 36, play 48, next 36, repeat 32, all icons 16; shuffle and repeat in
  /// the muted ink while off and on the accent fill while on. The volume (a slider of 96) and the speed (a ghost
  /// button 32, padded 10) at the two ends of the last row.
  ///
  /// The compact row (`BkqZ5`): 320 by 48, the card's fill and edge, `radius-lg`, padded 8 at the start and 12 at the
  /// end, 10 apart: play 32, the seek bar, the time. A voice message (`CjYWK`): `radius-xl`, padded 8 12 8 8, 10
  /// apart: play, the waveform, the time, a badge of the speed on the secondary fill or a dot of 8 in the info color.
  ///
  /// The mini player (`Jp10J`): the background, a line of the border's color on top, a progress line of 2 (4 under
  /// the pointer) in the primary color over the secondary, then a row of 54 padded 12 with 12 between: the artwork 36
  /// with `radius-sm`, the title 13/500 over who it is by 12, previous 32, play, next 32 four apart, and the time, the
  /// volume, the queue and expand buttons 8 apart at the end.
  ///
  /// Loading: lines of skeleton for the texts. An error: the alert glyph 14 and a line 12 in the destructive color,
  /// "Try again" 12/500 underlined. Disabled: half the opacity.
  fn recipe() -> Recipe {
    use AudioPlayerPart::*;
    use AudioPlayerVariant::{Compact, Full, Mini, Voice};
    use PaletteColor::{
      Background, Border as Line, Card, Destructive, Foreground, Info, Muted, MutedForeground, Primary, Secondary,
      SecondaryForeground, SecondaryHover,
    };
    let edge = || Border::inside(BorderSize::Sm, Line);
    let mono = |s: PartStyle| {
      s.font_family(FontFamily::Mono)
        .font_size(11.0)
        .foreground(MutedForeground)
    };
    Recipe::new()
      .base(Root, |s| s.foreground(Foreground))
      .variant(Full, Root, |s| {
        s.width(360.0)
          .max_width(Length::Fill)
          .padding_x(20.0)
          .padding_y(20.0)
          .gap(16.0)
          .background(Card)
          .border(edge())
          .radius(RadiusSize::Xl)
      })
      .variant(Compact, Root, |s| {
        s.width(320.0)
          .max_width(Length::Fill)
          .height(48.0)
          .padding_start(8.0)
          .padding_end(12.0)
          .gap(10.0)
          .background(Card)
          .border(edge())
          .radius(RadiusSize::Lg)
      })
      .variant(Voice, Root, |s| {
        s.max_width(Length::Fill)
          .padding_start(8.0)
          .padding_end(12.0)
          .padding_y(8.0)
          .gap(10.0)
          .background(Card)
          .border(edge())
          .radius(RadiusSize::Xl)
      })
      .variant(Mini, Root, |s| {
        s.width(Length::Fill).background(Background)
      })
      // The border's line is under the progress line, which lies over it.
      .base(Edge, |s| s.top_line(Border::inside(BorderSize::Sm, Line)))
      .state(AudioPlayerState::Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
      .base(Bar, |s| s.height(54.0).padding_x(12.0).gap(12.0))
      .base(Header, |s| s.gap(12.0))
      .base(Media, |s| s.square(64.0).radius(RadiusSize::Md).background(Muted))
      .variant(Mini, Media, |s| s.square(36.0).radius(RadiusSize::Sm))
      .base(MediaIcon, |s| s.square(24.0).foreground(MutedForeground))
      .variant(Mini, MediaIcon, |s| s.square(16.0))
      .base(Text, |s| s.gap(2.0))
      .variant(Full, Text, |s| s.gap(4.0))
      .base(Skeleton, |s| s.padding_y(2.0))
      .base(Title, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(14.0)
          .weight(FontWeight::SemiBold)
          .foreground(Foreground)
      })
      .variant(Mini, Title, |s| s.font_size(13.0).weight(FontWeight::Medium))
      .base(Description, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .foreground(MutedForeground)
      })
      .variant(Mini, Description, |s| s.font_size(12.0))
      .base(Progress, |s| s.gap(6.0))
      .base(Times, |s| s.gap(8.0))
      .base(Elapsed, mono)
      .base(Total, mono)
      .base(Time, mono)
      .base(Transport, |s| s.gap(8.0))
      .variant(Mini, Transport, |s| s.gap(4.0))
      .base(Shuffle, |s| s.foreground(MutedForeground))
      .base(Repeat, |s| s.foreground(MutedForeground))
      .base(Speed, |s| s.padding_x(10.0))
      .base(Error, |s| s.gap(6.0))
      .base(ErrorIcon, |s| s.square(14.0).foreground(Destructive))
      .base(ErrorText, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(12.0)
          .foreground(Destructive)
      })
      .base(Retry, |s| {
        s.font_size(12.0)
          .weight(FontWeight::Medium)
          .underline(Border::inside(1.0, Foreground))
      })
      .base(Rate, |s| {
        fading(
          s.height(20.0)
            .padding_x(8.0)
            .radius(RadiusSize::Full)
            .background(Secondary)
            .foreground(SecondaryForeground)
            .typography(TypographyStyle::Caption)
            .font_size(11.0)
            .weight(FontWeight::Medium)
            .cursor(CursorIcon::Pointer),
        )
      })
      .hovered(Rate, |s| s.background(SecondaryHover))
      .focus_visible(Rate, |s| s.focus_ring())
      .base(Dot, |s| s.square(8.0).radius(RadiusSize::Full).background(Info))
      .base(Track, |s| s.height(2.0).background(Secondary))
      .state(AudioPlayerState::Scrubbing, Track, |s| s.height(4.0))
      .base(Fill, |s| s.background(Primary))
      .base(Tools, |s| s.gap(8.0))
  }
}
