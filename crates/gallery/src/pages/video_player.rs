//! The page of the video player: its playground, driven by a clock of the page's own, its states, its three forms,
//! the player without controls, and one with an app's own controls laid over the picture.

use std::{
  sync::{Arc, OnceLock},
  time::Duration,
};

#[cfg(windows)]
use lurq::ui::{Badge, BadgeVariant, Table, TableColumn};
use lurq::{
  app::{
    component::Component,
    ctx::{Ctx, Interval},
  },
  components::{Column, Row, Stack},
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, Gradient, HitTestBehavior, color::Color, dimension::Dimension, gradient::GradientStop},
  ui::{
    Button, ButtonPart, ButtonShape, ButtonVariant, Length, MediaCaptions, PlayButton, PlayButtonVariant,
    VideoFullscreen, VideoPlayer, VideoPlayerSize, icons, media_time_text,
  },
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, fit, props_table, quiet, text},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// A decoder that stands for an app's own, with what it gives a frame as and which card it is on.
#[cfg(windows)]
type Decoder = (crate::card_demo::Decoded, &'static str, &'static str);

/// Where a decoder is that decodes on the card lurq draws on.
#[cfg(windows)]
const THIS_CARD: &str = "The window's card";

/// A row of the table of decoders: its frames, what it gives a frame as, which card it is on, and how lurq
/// found its frames come to the screen.
#[cfg(windows)]
struct Stream {
  image: lurq::images::ImageData,
  gives: &'static str,
  on: &'static str,
  way: Option<lurq::images::CardFramesWay>,
}

/// How long the playground's video is, in seconds.
const LENGTH: f64 = 250.0;

/// What the playground's player is given to play, in the order of the select: nothing, so that the page's
/// clock moves a picture that stands for a video; the gallery's clip from its bytes; a file by its path; a
/// video at an address.
const SOURCES: [(&str, usize); 4] = [("None", 0), ("Bytes", 1), ("A file", FILED), ("An address", ADDRESSED)];

/// The places of a file and of an address among the sources.
const FILED: usize = 2;
const ADDRESSED: usize = 3;

/// A video anyone may fetch: the address the playground plays until another is typed.
const ADDRESS: &str = "https://interactive-examples.mdn.mozilla.net/media/cc0-videos/flower.mp4";

/// How often the page's clock moves the position on while it plays.
const TICK: Duration = Duration::from_millis(250);

/// The forms the playground offers: the one its width gives it, or one it is told.
const SIZES: [(&str, Option<VideoPlayerSize>); 4] = [
  ("from the width", None),
  ("Lg", Some(VideoPlayerSize::Lg)),
  ("Md", Some(VideoPlayerSize::Md)),
  ("Sm", Some(VideoPlayerSize::Sm)),
];

/// The widths the playground offers, each with a height of 16 by 9.
const WIDTHS: [(&str, f32); 4] = [("432", 432.0), ("640", 640.0), ("320", 320.0), ("200", 200.0)];

fn hex(rgb: u32) -> Color {
  Color::new((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, 255)
}

/// A picture that stands for a frame of the video: the board's sunset, from the top down.
fn picture() -> Column {
  let stops = [
    (0x132544, 0.0),
    (0x1E3A8A, 0.3),
    (0x7C3AED, 0.58),
    (0xF472B6, 0.78),
    (0xF59E0B, 0.9),
    (0x3B1212, 1.0),
  ]
  .map(|(rgb, at)| GradientStop::at(hex(rgb), at));
  Column::new()
    .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
    .background_gradient(Gradient::linear(180.0, stops))
}

/// The poster's picture: the board's teal, from the top left.
fn poster() -> Column {
  let stops = [(0x0F2E1B, 0.0), (0x0D9488, 0.45), (0x132544, 1.0)].map(|(rgb, at)| GradientStop::at(hex(rgb), at));
  Column::new()
    .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
    .background_gradient(Gradient::linear(135.0, stops))
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(9.0)
    .overflow_visible()
    .child(Row::new().max_width(FILL).overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new()
    .width(FILL)
    .spacing(28.0)
    .wrap()
    .align_items(Alignment::End)
    .overflow_visible()
}

/// The gallery's own clip: fifteen seconds of a test picture, 640 by 360, in the gallery's binary.
#[cfg(not(target_arch = "wasm32"))]
fn clip() -> lurq::resources::Source {
  static CLIP: OnceLock<lurq::resources::Source> = OnceLock::new();
  CLIP
    .get_or_init(|| lurq::resources::Source::bytes(include_bytes!("../../assets/media/sample.mp4").to_vec()))
    .clone()
}

/// Where the clip is as a file on this machine's disk, written there once: the path the playground plays as a
/// file until another is typed.
#[cfg(not(target_arch = "wasm32"))]
fn clip_path() -> String {
  static CLIP_FILE: OnceLock<std::path::PathBuf> = OnceLock::new();
  CLIP_FILE
    .get_or_init(|| {
      let folder = std::env::temp_dir().join("lurq-gallery");
      let path = folder.join("sample.mp4");
      let _ = std::fs::create_dir_all(&folder);
      let _ = std::fs::write(&path, include_bytes!("../../assets/media/sample.mp4"));
      path
    })
    .display()
    .to_string()
}

/// The source the playground's player plays: none for the first of [`SOURCES`], and for a path or an address
/// that is not typed yet.
#[cfg(not(target_arch = "wasm32"))]
fn source_of(chosen: usize, path: &str, address: &str) -> Option<lurq::resources::Source> {
  match chosen {
    1 => Some(clip()),
    FILED if !path.trim().is_empty() => Some(lurq::resources::Source::file(path.trim())),
    ADDRESSED if !address.trim().is_empty() => Some(lurq::resources::Source::url(address.trim())),
    _ => None,
  }
}

/// A player of the states' size, paused at 1:24 of 4:10, for a picture of a state: 432 wide and no wider than
/// what holds it, as high as 16 by 9 makes it.
fn state() -> VideoPlayer {
  VideoPlayer::new()
    .media(picture())
    .width(432.0)
    .max_width(Length::Fill)
    .duration(LENGTH)
    .buffered(140.0)
    .position_controlled(84.0, |_| {})
}

/// A state of the board that a player of a section starts in.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Scene {
  Idle,
  Playing,
  Paused,
  Waiting,
  Ended,
  Error,
  Live,
  Captions,
  Muted,
  Fullscreen,
  Large,
  Compact,
  Minimal,
  Touch,
  TouchSide,
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for Scene {
  fn inspect(&self, formatter: &mut lurq::app::component::DevtoolsFormatter<'_>) {
    formatter.value_debug(self);
  }
}

/// A player of a section, live: it starts in the state its `Scene` names, its values are its own, and a clock of
/// its own moves it on while it plays, so every control on it answers a press as it would in an app. It is a
/// component of its own, and what changes in it renders it again and not the page.
struct Demo {
  playing: Signal<bool>,
  position: Signal<f32>,
  volume: Signal<f32>,
  muted: Signal<bool>,
  rate: Signal<f32>,
  captions: Signal<bool>,
  quality: Signal<Option<usize>>,
  fullscreen: Signal<bool>,
  picture_in_picture: Signal<bool>,
  ended: Signal<bool>,
  error: Signal<bool>,
  _clock: Interval,
}

impl Component for Demo {
  type Props = Scene;

  fn create(ctx: &mut Ctx) -> Self {
    let scene = *ctx.props::<Scene>();
    let playing = ctx.signal(matches!(
      scene,
      Scene::Playing | Scene::Waiting | Scene::Live | Scene::Captions
    ));
    let position = ctx.signal(match scene {
      Scene::Idle => 0.0,
      Scene::Ended => LENGTH as f32,
      _ => 84.0,
    });
    let (rate, error) = (ctx.signal(1.0_f32), ctx.signal(scene == Scene::Error));
    let waiting = scene == Scene::Waiting;
    // While it plays its clock moves it on, and from the end it goes round: it stays the state it shows.
    let clock = ctx.create_interval(TICK, {
      let (playing, position, rate, error) = (playing.clone(), position.clone(), rate.clone(), error.clone());
      move || {
        if !playing.get_untracked() || waiting || error.get_untracked() {
          return;
        }
        let next = position.get_untracked() + TICK.as_secs_f32() * rate.get_untracked();
        position.set(if f64::from(next) >= LENGTH { 0.0 } else { next });
      }
    });
    clock.start();
    Self {
      playing,
      position,
      volume: ctx.signal(0.8),
      muted: ctx.signal(scene == Scene::Muted),
      rate,
      captions: ctx.signal(scene == Scene::Captions),
      quality: ctx.signal(None),
      fullscreen: ctx.signal(matches!(scene, Scene::Fullscreen | Scene::TouchSide)),
      picture_in_picture: ctx.signal(false),
      ended: ctx.signal(scene == Scene::Ended),
      error,
      _clock: clock,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let scene = *ctx.props::<Scene>();
    let (width, size) = match scene {
      Scene::Large => (640.0, None),
      Scene::Compact => (320.0, Some(VideoPlayerSize::Md)),
      Scene::Minimal => (200.0, Some(VideoPlayerSize::Sm)),
      // The board's phone, upright and on its side.
      Scene::Touch => (360.0, None),
      Scene::TouchSide => (780.0, None),
      _ => (432.0, None),
    };
    let position = f64::from(self.position.get());
    // As wide as the board draws it and no wider than what holds it, a window as narrow as a phone; its height is
    // what 16 by 9 makes of the width it has.
    let mut player = VideoPlayer::new()
      .media(picture())
      .width(width)
      .max_width(Length::Fill)
      .live(scene == Scene::Live)
      .waiting(scene == Scene::Waiting)
      .ended(self.ended.get())
      .playing(self.playing.clone())
      .position(self.position.clone())
      .volume(self.volume.clone())
      .muted(self.muted.clone())
      .rate(self.rate.clone())
      .captions(self.captions.clone())
      .caption_cues(&VideoPlayerPage::captions())
      .captions_track("English")
      .qualities(&["1080p", "720p", "480p"])
      .quality(self.quality.clone())
      .fullscreen(self.fullscreen.clone())
      .picture_in_picture(self.picture_in_picture.clone())
      .preview(|_| picture().into());
    if scene != Scene::Live {
      player = player.duration(LENGTH).buffered((position + 56.0).min(LENGTH));
    }
    if let Some(size) = size {
      player = player.size(size);
    }
    if scene == Scene::Idle {
      player = player.poster(poster());
    }
    if matches!(scene, Scene::Touch | Scene::TouchSide) {
      player = player
        .touch(true)
        .title("Night Drive")
        .description("Aurora Lane · Low Tide");
    }
    if scene == Scene::TouchSide {
      player = player.aspect(780.0 / 360.0);
    }
    // The state is drawn where it stands: a player that filled the window would leave nothing of the page.
    if matches!(scene, Scene::Fullscreen | Scene::TouchSide) {
      player = player.fullscreen_fills(VideoFullscreen::App);
    }
    if self.error.get() {
      player = player
        .error("This video can't be played")
        .error_description("Check the connection and try again.");
    }
    let (replayed, seeked, retried) = (
      (self.ended.clone(), self.position.clone()),
      self.ended.clone(),
      (self.error.clone(), self.playing.clone()),
    );
    player
      .on_playing_change(move |to| {
        // Played again once it ended: from the start.
        if to && replayed.0.get_untracked() {
          replayed.0.set(false);
          replayed.1.set(0.0);
        }
      })
      .on_position_change(move |to| {
        if to < LENGTH && seeked.get_untracked() {
          seeked.set(false);
        }
      })
      .on_retry(move || {
        retried.0.set(false);
        retried.1.set(true);
      })
      .key("demo")
      .mount(ctx)
  }
}

pub struct VideoPlayerPage {
  frame: Frame,
  playground: Playground,
  size: Signal<usize>,
  width: Signal<usize>,
  playing: Signal<bool>,
  position: Signal<f32>,
  volume: Signal<f32>,
  muted: Signal<bool>,
  rate: Signal<f32>,
  captions: Signal<bool>,
  quality: Signal<Option<usize>>,
  fullscreen: Signal<bool>,
  picture_in_picture: Signal<bool>,
  /// Whether the player in the playground's own window fills that window, and whether it is in picture in
  /// picture: they are of a window, and a press in one window is not a press in the other.
  window_fullscreen: Signal<bool>,
  window_picture: Signal<bool>,
  waiting: Signal<bool>,
  ended: Signal<bool>,
  live: Signal<bool>,
  error: Signal<bool>,
  poster: Signal<bool>,
  controls: Signal<bool>,
  overlay: Signal<bool>,
  feed_playing: Signal<bool>,
  feed_muted: Signal<bool>,
  /// What the playground's player plays, of [`SOURCES`], and the path and the address typed for the last two.
  source: Signal<usize>,
  path: Signal<String>,
  address: Signal<String>,
  /// The decoders that stand for an app's own, each with what its frames are.
  #[cfg(windows)]
  decoded: Arc<std::sync::Mutex<Vec<Decoder>>>,
  /// Whether they run: they start once the window says which card it draws on, which is after this page is made.
  #[cfg(windows)]
  decoding: Signal<bool>,
  /// How the frames of each come to the screen, as lurq found.
  #[cfg(windows)]
  ways: Signal<Vec<Option<lurq::images::CardFramesWay>>>,
  #[cfg(windows)]
  _decoders: Interval,
  /// On macOS and iOS: the decoder that stands for an app's own, once something draws its frames where they are.
  #[cfg(any(target_os = "macos", target_os = "ios"))]
  decoder: Arc<std::sync::Mutex<Option<crate::cv_demo::Decoded>>>,
  #[cfg(any(target_os = "macos", target_os = "ios"))]
  decoding: Signal<bool>,
  #[cfg(any(target_os = "macos", target_os = "ios"))]
  _decoders: Interval,
  _clock: Interval,
}

impl VideoPlayerPage {
  /// The track of the captions, read from the text of a WebVTT file as an app would read its own: a cue every
  /// six seconds, to the end.
  fn captions() -> MediaCaptions {
    static TRACK: OnceLock<MediaCaptions> = OnceLock::new();
    TRACK
      .get_or_init(|| {
        const CUES: [&str; 4] = [
          "The bar fades out while the video plays.",
          "Move the pointer and it comes back.",
          "Space or K plays and pauses; the arrows seek.",
          "C turns these captions off.",
        ];
        let stamp = |seconds: usize| format!("{:02}:{:02}.000", seconds / 60, seconds % 60);
        let mut vtt = String::from("WEBVTT\n");
        for (index, start) in (0..LENGTH as usize).step_by(6).enumerate() {
          let cue = CUES[index % CUES.len()];
          vtt.push_str(&format!("\n{} --> {}\n{cue}\n", stamp(start), stamp(start + 6)));
        }
        MediaCaptions::parse(&vtt)
      })
      .clone()
  }

  /// The playground: the player as the panel's props make it, its values held by the page and moved on by its clock.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let size = SIZES[self.size.get().min(SIZES.len() - 1)].1;
    let width = WIDTHS[self.width.get().min(WIDTHS.len() - 1)].1;
    let (waiting, ended, live, error) = (self.waiting.get(), self.ended.get(), self.live.get(), self.error.get());
    let (poster_on, controls, overlay) = (self.poster.get(), self.controls.get(), self.overlay.get());
    // What it really plays, where a source is chosen: its picture, its length and where it is are then the
    // video's own.
    let chosen = self.source.get().min(SOURCES.len() - 1);
    #[cfg(not(target_arch = "wasm32"))]
    let source = source_of(chosen, &self.path.get(), &self.address.get());
    let sourced = chosen != 0;
    // The page reads nothing that moves as it plays or is dragged: the player is a component of its own, and a
    // position read here would build the whole page again for every step of a drag. The player finds the cue of
    // its position in the track it is given.
    let make = {
      let (playing, position, volume, muted, rate) = (
        self.playing.clone(),
        self.position.clone(),
        self.volume.clone(),
        self.muted.clone(),
        self.rate.clone(),
      );
      let (captions, quality, fullscreen, pip) = (
        self.captions.clone(),
        self.quality.clone(),
        self.fullscreen.clone(),
        self.picture_in_picture.clone(),
      );
      let buffered = LENGTH.min(200.0);
      let (was_ended, failed) = (self.ended.clone(), self.error.clone());
      let own = (self.window_fullscreen.clone(), self.window_picture.clone());
      // `window`: for the playground's own window, whose fullscreen and picture in picture are its own.
      move |window: bool| {
        let (fullscreen, pip) = if window {
          (own.0.clone(), own.1.clone())
        } else {
          (fullscreen.clone(), pip.clone())
        };
        let mut player = VideoPlayer::new().width(width).live(live).waiting(waiting);
        // The window of its own shows the player with no source: two players of one video would be heard twice.
        #[cfg(not(target_arch = "wasm32"))]
        let played = source.clone().filter(|_| !window);
        #[cfg(target_arch = "wasm32")]
        let played: Option<()> = None;
        player = match played {
          #[cfg(not(target_arch = "wasm32"))]
          Some(source) => player.source(&source),
          // With none, a picture stands for the video, as long and as far loaded as the page says.
          _ => {
            let stood = player
              .media(picture())
              .buffered(buffered)
              // The frame under the pointer of the seek bar: the picture stands for it.
              .preview(|_| picture().into());
            if live { stood } else { stood.duration(LENGTH) }
          }
        };
        let mut player = player
          .ended(ended)
          .controls(controls)
          .playing(playing.clone())
          .position(position.clone())
          .volume(volume.clone())
          .muted(muted.clone())
          .rate(rate.clone())
          .captions(captions.clone())
          .caption_cues(&Self::captions())
          .captions_track("English")
          .qualities(&["1080p", "720p", "480p"])
          .quality(quality.clone())
          .fullscreen(fullscreen.clone())
          .picture_in_picture(pip.clone());
        if let Some(size) = size {
          player = player.size(size);
        }
        if poster_on {
          player = player.poster(poster());
        }
        if error {
          player = player
            .error("This video can't be played")
            .error_description("Check the connection and try again.");
        }
        if overlay {
          player = player.child(
            Row::new()
              .padding(12.0)
              .child(text("Your layer", 12.0, FontWeight::Medium).color(Color::new(255, 255, 255, 255))),
          );
        }
        // What an app does with what it is told, here and in a window of its own: played again once it ended
        // it starts over, sought back from the end it has not ended, and tried again it loads.
        let (replayed, seeked, retried) = ((was_ended.clone(), position.clone()), was_ended.clone(), failed.clone());
        player
          .on_playing_change(move |to| {
            if to && replayed.0.get_untracked() {
              replayed.0.set(false);
              replayed.1.set(0.0);
            }
          })
          .on_position_change(move |to| {
            if to < LENGTH && seeked.get_untracked() {
              seeked.set(false);
            }
          })
          .on_retry(move || retried.set(false))
      }
    };
    let record = self.playground.recorder();
    let shown = {
      let make = make.clone();
      let forced = self.playground.forced();
      // No wider than the stage, as an app's player is no wider than its window: in a window as narrow as a
      // phone the width chosen is more than there is, and half a player scrolled to shows nothing of its bar.
      let room = self.playground.stage_room();
      let playground = &self.playground;
      move |ctx: &mut Ctx| {
        let say = |name: &'static str| {
          let record = record.clone();
          move |value: String| record(format!("{name}({value})"))
        };
        let (p, s, e, v, m, r, c, q, f, i, t) = (
          say("on_playing_change"),
          say("on_position_change"),
          say("on_position_change_end"),
          say("on_volume_change"),
          say("on_muted_change"),
          say("on_rate_change"),
          say("on_captions_change"),
          say("on_quality_change"),
          say("on_fullscreen_change"),
          say("on_picture_in_picture_change"),
          say("on_retry"),
        );
        let mut player = playground.styled(make(false));
        // The stage scrolls to what is wider than it, so it has no width to be a share of: the room is told.
        if room > 0.0 {
          player = player.max_width(room);
        }
        player
          .force(forced)
          .on_playing_change(move |to| p(to.to_string()))
          .on_position_change(move |to| s(format!("{to:.1}")))
          .on_position_change_end(move |to| e(format!("{to:.1}")))
          .on_volume_change(move |to| v(format!("{to:.2}")))
          .on_muted_change(move |to| m(to.to_string()))
          .on_rate_change(move |to| r(to.to_string()))
          .on_captions_change(move |to| c(to.to_string()))
          .on_quality_change(move |to| q(format!("{to:?}")))
          .on_fullscreen_change(move |to| f(to.to_string()))
          .on_picture_in_picture_change(move |to| i(to.to_string()))
          .on_retry(move || t(String::new()))
          .key("played")
          .mount(ctx)
      }
    };

    let mut code = vec!["VideoPlayer::new()".to_owned()];
    code.push(match chosen {
      1 => "    .source(&Source::bytes(include_bytes!(\"clip.mp4\").to_vec()))".to_owned(),
      FILED => format!("    .source(&Source::file({:?}))", self.path.get().trim()),
      ADDRESSED => format!("    .source(&Source::url({:?}))", self.address.get().trim()),
      _ => "    .media(frame)".to_owned(),
    });
    code.push(format!("    .width({width}.0)"));
    if let Some(size) = size {
      code.push(format!("    .size(VideoPlayerSize::{size:?})"));
    }
    if !live && !sourced {
      code.push(format!("    .duration({LENGTH:.1})"));
    }
    for (on, line) in [
      (live, "    .live(true)"),
      (waiting, "    .waiting(true)"),
      (ended, "    .ended(true)"),
      (!controls, "    .controls(false)"),
      (poster_on, "    .poster(poster)"),
      (error, "    .error(\"This video can't be played\")"),
      (overlay, "    .child(your_layer)"),
    ] {
      if on {
        code.push(line.to_owned());
      }
    }
    code.extend(
      [
        "    .playing(playing)",
        "    .position(position)",
        "    .volume(volume)",
        "    .muted(muted)",
        "    .rate(rate)",
        "    .captions(captions)",
        "    .caption_cues(&track)",
        "    .qualities(&[\"1080p\", \"720p\", \"480p\"])",
        "    .quality(quality)",
        "    .fullscreen(fullscreen)",
        "    .picture_in_picture(pip)",
        "    .preview(|at| frame_at(at))",
        "    .mount(ctx)",
      ]
      .map(str::to_owned),
    );
    let tree = vec![
      format!("root        {} · radius lg, clipped", self.playground.measured()),
      "├─ media     the frame the app gives, the poster over it".to_owned(),
      "├─ veil      #00000099 while ended or failed".to_owned(),
      "├─ overlay   the app's own layer, with and without controls".to_owned(),
      "├─ center    the 64 disc · a spinner · the error".to_owned(),
      "├─ captions  16/500 on #000000B3, 12 over the bar".to_owned(),
      "└─ controls  88 · seek bar · row of 36 on a gradient to #000000B3".to_owned(),
    ];
    let facts = format!(
      "{} · {}×",
      if sourced {
        "plays its source".to_owned()
      } else if live {
        "live".to_owned()
      } else {
        format!("{} long", media_time_text(LENGTH, false))
      },
      self.rate.get()
    );
    let (set_size, set_width) = (self.size.clone(), self.width.clone());
    let reset = (
      [self.size.clone(), self.width.clone(), self.source.clone()],
      [
        self.playing.clone(),
        self.muted.clone(),
        self.captions.clone(),
        self.fullscreen.clone(),
        self.picture_in_picture.clone(),
        self.waiting.clone(),
        self.ended.clone(),
        self.live.clone(),
        self.error.clone(),
        self.overlay.clone(),
      ],
      [self.poster.clone(), self.controls.clone()],
      (
        self.position.clone(),
        self.volume.clone(),
        self.rate.clone(),
        self.quality.clone(),
      ),
      (self.path.clone(), self.address.clone()),
    );
    let flags = [
      flag("playing", &self.playing),
      flag("waiting", &self.waiting),
      flag("ended", &self.ended),
      flag("live", &self.live),
      flag("error", &self.error),
      flag("poster", &self.poster),
      flag("muted", &self.muted),
      flag("captions", &self.captions),
      flag("fullscreen", &self.fullscreen),
      flag("picture_in_picture", &self.picture_in_picture),
      flag("controls", &self.controls),
      flag("child: your layer", &self.overlay),
    ];
    let mut controls_list = vec![
      prop(
        "size",
        "VideoPlayerSize",
        self.playground.select(0, &SIZES, &size, move |size| {
          set_size.set(SIZES.iter().position(|(_, chosen)| *chosen == size).unwrap_or(0))
        }),
      ),
      prop(
        "width",
        "Length",
        self.playground.select(1, &WIDTHS, &width, move |width| {
          set_width.set(WIDTHS.iter().position(|(_, chosen)| *chosen == width).unwrap_or(0))
        }),
      ),
    ];
    #[cfg(not(target_arch = "wasm32"))]
    {
      let (to_source, stop, rewind, over) = (
        self.source.clone(),
        self.playing.clone(),
        self.position.clone(),
        self.ended.clone(),
      );
      controls_list.push(prop(
        "source",
        "&Source",
        self.playground.select(2, &SOURCES, &chosen, move |index| {
          // Another video: what played stops, and the new one starts from its start.
          stop.set(false);
          over.set(false);
          rewind.set(0.0);
          to_source.set(index);
        }),
      ));
      // What a key in a path or an address does: what plays stops, so that half of one is not asked for.
      let stops = |playing: &Signal<bool>| {
        let playing = playing.clone();
        move || {
          if playing.get_untracked() {
            playing.set(false);
          }
        }
      };
      // A file of the person's own is its path, and a video on the network its address: typed here.
      if chosen == FILED {
        controls_list.push(prop(
          "path",
          "impl Into<PathBuf>",
          crate::playground::field_telling(&self.path, stops(&self.playing)),
        ));
      }
      if chosen == ADDRESSED {
        controls_list.push(prop(
          "address",
          "&str",
          crate::playground::field_telling(&self.address, stops(&self.playing)),
        ));
      }
    }
    controls_list.push(divider());
    controls_list.extend(flags);
    self.playground.window_shows({
      let make = make.clone();
      move |ctx| make(true).mount(ctx)
    });
    self.playground.card(
      ctx,
      Spec {
        title: "VideoPlayer",
        make: Arc::new(move || -> Element { make(false).build(&lurq::ui::current_theme()) }),
        shown: Shown::built(shown),
        fill: false,
        forceable: true,
        facts,
        event: "on_playing_change / on_position_change / on_volume_change / …",
        code,
        tree,
        controls: controls_list,
        changed: [
          sourced,
          size.is_some(),
          width != 432.0,
          self.playing.get(),
          waiting,
          ended,
          live,
          error,
          !poster_on,
          self.muted.get(),
          self.captions.get(),
          self.fullscreen.get(),
          self.picture_in_picture.get(),
          !controls,
          overlay,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          for select in &reset.0 {
            select.set(0);
          }
          for flag in &reset.1 {
            flag.set(false);
          }
          for flag in &reset.2 {
            flag.set(true);
          }
          reset.3.0.set(0.0);
          reset.3.1.set(0.8);
          reset.3.2.set(1.0);
          reset.3.3.set(None);
          reset.4.1.set(ADDRESS.to_owned());
          #[cfg(not(target_arch = "wasm32"))]
          reset.4.0.set(clip_path());
        }),
      },
    )
  }

  /// A round button of the feed's column: a glyph on a dark disc, and a count under it.
  fn feed_action(glyph: &'static str, label: &str, count: &str) -> Column {
    Column::new()
      .spacing(4.0)
      .align_items(Alignment::Center)
      .child(
        Button::new(label)
          .icon(glyph)
          .icon_only(true)
          .shape(ButtonShape::Pill)
          .variant(ButtonVariant::Ghost)
          .style(ButtonPart::Root, |s| {
            s.square(40.0)
              .background(Color::new(0, 0, 0, 0x66))
              .foreground(Color::new(255, 255, 255, 255))
          }),
      )
      .child(text(count, 11.0, FontWeight::Medium).color(Color::new(255, 255, 255, 255)))
  }

  /// The board's vertical example: a player with no controls, and the app's own over its picture, made of the
  /// system's parts.
  fn feed(&self) -> VideoPlayer {
    let (playing, muted) = (self.feed_playing.get(), self.feed_muted.get());
    let (play, mute) = (self.feed_playing.clone(), self.feed_muted.clone());
    let white = Color::new(255, 255, 255, 255);
    let center = Row::new()
      .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .child(
        PlayButton::new()
          .variant(PlayButtonVariant::Overlay)
          .size(lurq::ui::PlayButtonSize::Xl)
          .playing(playing)
          .on_change(move |to| play.set(to)),
      );
    let top = Row::new().width(FILL).padding(12.0).justify(Justify::End).child(
      Button::new(if muted { "Unmute" } else { "Mute" })
        .icon(if muted { icons::VOLUME_OFF } else { icons::VOLUME })
        .icon_only(true)
        .shape(ButtonShape::Pill)
        .variant(ButtonVariant::Ghost)
        .style(ButtonPart::Root, |s| {
          s.square(32.0).background(Color::new(0, 0, 0, 0x66)).foreground(white)
        })
        .on_click(move |_| mute.update(|muted| *muted = !*muted)),
    );
    let shade = Gradient::linear(
      180.0,
      [
        GradientStop::at(Color::new(0, 0, 0, 0), 0.0),
        GradientStop::at(Color::new(0, 0, 0, 0x99), 1.0),
      ],
    );
    let bottom = Row::new()
      .width(FILL)
      .padding(12.0)
      .spacing(12.0)
      .align_items(Alignment::End)
      .background_gradient(shade)
      .child(
        Column::new()
          .flex(1.0)
          .min_width(0.0)
          .spacing(4.0)
          .child(text("@lurq.dev", 14.0, FontWeight::SemiBold).color(white))
          .child(
            text(
              "Frosted glass over a moving picture, drawn by the GPU.",
              13.0,
              FontWeight::Normal,
            )
            .color(white),
          ),
      )
      .child(
        Column::new()
          .spacing(12.0)
          .child(Self::feed_action(icons::HEART, "Like", "12.4K"))
          .child(Self::feed_action(icons::MESSAGE, "Comments", "318"))
          .child(Self::feed_action(icons::SHARE, "Share", "Share")),
      );
    let progress = Row::new()
      .width(FILL)
      .height(2.0)
      .background(Color::new(255, 255, 255, 0x33))
      .child(Row::new().width(Dimension::Pct(34.0)).height(2.0).background(white));
    // The play button is the top of the layer: the shade behind the words reaches up under it, and would take
    // its presses from above. What holds it is as large as the picture and is nothing to the pointer itself, so
    // the buttons at the two ends are pressed through it.
    let layer = Stack::new()
      .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
      .child(
        Column::new()
          .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
          .justify(Justify::SpaceBetween)
          .child(top)
          .child(Column::new().width(FILL).child(bottom).child(progress)),
      )
      .child(center.hit_test(HitTestBehavior::ContentOnly));
    VideoPlayer::new()
      .media(poster())
      .width(243.0)
      .max_width(Length::Fill)
      // Upright, as a feed's picture is.
      .aspect(9.0 / 16.0)
      .controls(false)
      .playing_controlled(playing, |_| {})
      .muted_controlled(muted, |_| {})
      .child(layer)
  }
}

impl Component for VideoPlayerPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let (playing, position, rate) = (ctx.signal(false), ctx.signal(0.0_f32), ctx.signal(1.0_f32));
    let (waiting, ended, error) = (ctx.signal(false), ctx.signal(false), ctx.signal(false));
    let source = ctx.signal(0_usize);
    // The page's clock: while it plays it moves the position on by the time gone at the speed chosen, and at the
    // end it stops and says it ended. A player with a source is moved by its video, and not by this clock.
    let clock = ctx.create_interval(TICK, {
      let (playing, position, rate) = (playing.clone(), position.clone(), rate.clone());
      let (waiting, ended, error) = (waiting.clone(), ended.clone(), error.clone());
      let source = source.clone();
      move || {
        if !playing.get_untracked() || waiting.get_untracked() || error.get_untracked() || source.get_untracked() != 0 {
          return;
        }
        let next = position.get_untracked() + TICK.as_secs_f32() * rate.get_untracked();
        if f64::from(next) >= LENGTH {
          position.set(LENGTH as f32);
          playing.set(false);
          ended.set(true);
        } else {
          position.set(next);
        }
      }
    });
    clock.start();
    // The decoders that stand for an app's own start once something draws frames of the card: a page made with
    // the window is made before the window has its device.
    #[cfg(windows)]
    let (decoded, decoding) = (
      Arc::new(std::sync::Mutex::new(Vec::<Decoder>::new())),
      ctx.signal(false),
    );
    #[cfg(windows)]
    let ways = ctx.signal(Vec::<Option<lurq::images::CardFramesWay>>::new());
    #[cfg(windows)]
    let decoders = ctx.create_interval(TICK, {
      let (decoded, decoding, ways) = (decoded.clone(), decoding.clone(), ways.clone());
      move || {
        if decoding.get() {
          // How each one's frames come is known once lurq has drawn them.
          let found: Vec<_> = decoded
            .lock()
            .expect("the decoders of the page")
            .iter()
            .map(|(decoded, ..)| decoded.way())
            .collect();
          if ways.get() != found {
            ways.set(found);
          }
          return;
        }
        let started: Vec<_> = [
          (crate::card_demo::Kind::Nv12, "One NV12 texture", THIS_CARD),
          (
            crate::card_demo::Kind::Planes,
            "Two textures: gray, and color",
            THIS_CARD,
          ),
          (
            crate::card_demo::Kind::Rgba,
            "One texture of red, green and blue",
            THIS_CARD,
          ),
          (
            crate::card_demo::Kind::OtherCard,
            "Two textures: gray, and color",
            "Another card",
          ),
        ]
        .into_iter()
        .filter_map(|(kind, gives, on)| Some((crate::card_demo::start(kind)?, gives, on)))
        .collect();
        if !started.is_empty() {
          *decoded.lock().expect("the decoders of the page") = started;
          decoding.set(true);
        }
      }
    });
    #[cfg(windows)]
    decoders.start();
    // On macOS and iOS it starts once the window draws on Metal: a page made with the window is made before the
    // window has its device.
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    let (decoder, decoding) = (
      Arc::new(std::sync::Mutex::new(None::<crate::cv_demo::Decoded>)),
      ctx.signal(false),
    );
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    let decoders = ctx.create_interval(TICK, {
      let (decoder, decoding) = (decoder.clone(), decoding.clone());
      move || {
        if decoding.get() {
          return;
        }
        if let Some(started) = crate::cv_demo::start() {
          *decoder.lock().expect("the decoder of the page") = Some(started);
          decoding.set(true);
        }
      }
    });
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    decoders.start();
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 3),
      size: ctx.signal(0),
      width: ctx.signal(0),
      playing,
      position,
      volume: ctx.signal(0.8),
      muted: ctx.signal(false),
      rate,
      captions: ctx.signal(false),
      quality: ctx.signal(None),
      fullscreen: ctx.signal(false),
      picture_in_picture: ctx.signal(false),
      window_fullscreen: ctx.signal(false),
      window_picture: ctx.signal(false),
      waiting,
      ended,
      live: ctx.signal(false),
      error,
      poster: ctx.signal(true),
      controls: ctx.signal(true),
      overlay: ctx.signal(false),
      feed_playing: ctx.signal(false),
      feed_muted: ctx.signal(true),
      source,
      #[cfg(not(target_arch = "wasm32"))]
      path: ctx.signal(clip_path()),
      #[cfg(target_arch = "wasm32")]
      path: ctx.signal(String::new()),
      address: ctx.signal(ADDRESS.to_owned()),
      #[cfg(windows)]
      decoded,
      #[cfg(windows)]
      decoding,
      #[cfg(windows)]
      ways,
      #[cfg(windows)]
      _decoders: decoders,
      #[cfg(any(target_os = "macos", target_os = "ios"))]
      decoder,
      #[cfg(any(target_os = "macos", target_os = "ios"))]
      decoding,
      #[cfg(any(target_os = "macos", target_os = "ios"))]
      _decoders: decoders,
      _clock: clock,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let playground = self.play(ctx);
    // What a stage has of width: a state's player is no wider, as an app's is no wider than its window.
    let room = self.frame.stage_room();
    // Each is a player of its own, live from the state the board draws: a press of anything on it is answered.
    let mut states = items();
    for (scene, caption) in [
      (Scene::Idle, "idle · the poster and its length"),
      (Scene::Playing, "playing · the bar fades 3 s after the pointer rests"),
      (Scene::Paused, "paused · the bar stays"),
      (Scene::Waiting, "waiting"),
      (Scene::Ended, "ended · replay"),
      (Scene::Error, "error"),
      (Scene::Live, "live · no duration, the bar at the edge"),
      (Scene::Captions, "captions · above the bar"),
      (Scene::Muted, "muted"),
      (Scene::Fullscreen, "fullscreen · the glyph exits (drawn in place)"),
    ] {
      states = states.child(item(ctx.mount::<Demo>(scene), caption));
    }
    // A size is drawn at its own width, which is what it shows: in a stage narrower than it, it scrolls sideways
    // under its name, and the sizes stand one under another.
    let sized: Vec<(Element, &str, f32)> = [
      (Scene::Large, "large · 640 × 360", 640.0),
      (Scene::Compact, "compact · 320 × 180", 320.0),
      (Scene::Minimal, "minimal · 200 × 113", 200.0),
    ]
    .into_iter()
    .map(|(scene, caption, width)| (ctx.mount::<Demo>(scene), caption, width))
    .collect();
    let sizes: Element = if room > 0.0 && room < 640.0 {
      let mut sizes = Column::new().width(FILL).spacing(28.0).overflow_visible();
      for (player, caption, width) in sized {
        sizes = sizes.child(
          Column::new()
            .width(FILL)
            .spacing(9.0)
            .child(fit(caption, room, width, player))
            .child(quiet(caption, 12.0)),
        );
      }
      sizes.into()
    } else {
      let mut sizes = items();
      for (player, caption, _) in sized {
        sizes = sizes.child(item(player, caption));
      }
      sizes.into()
    };
    // A real clip, decoded by the system: the player is given its source and nothing else, and the media says
    // where it is. A browser has no decoder of ours yet.
    #[cfg(not(target_arch = "wasm32"))]
    let real: Element = items()
      .child(item(
        VideoPlayer::new()
          .source(&clip())
          .width(640.0)
          .max_width(Length::Fill)
          .key("clip")
          .mount(ctx),
        if cfg!(windows) {
          "the gallery's clip · 15 s of 640 × 360, decoded by Media Foundation"
        } else {
          "the gallery's clip · 15 s of 640 × 360, played by the system's AVPlayer"
        },
      ))
      .into();
    #[cfg(target_arch = "wasm32")]
    let real: Element = quiet("A video is not decoded in a browser build yet.", 13.0).into();
    // The frames of a decoder that is not lurq's: each is a plain picture, with no player around it.
    #[cfg(windows)]
    let decoded: Element = if !self.decoding.get() {
      quiet(
        "Nothing draws frames of the card in this run: it takes Direct3D 12 or Vulkan.",
        13.0,
      )
      .into()
    } else {
      // A row for each decoder: its frames as they are drawn, what it gives a frame as, which card it is on, and
      // how lurq found its frames come to the screen.
      let ways = self.ways.get();
      let streams: Vec<Stream> = self
        .decoded
        .lock()
        .expect("the decoders of the page")
        .iter()
        .enumerate()
        .map(|(at, (decoded, gives, on))| Stream {
          image: decoded.image.clone(),
          gives,
          on,
          way: ways.get(at).copied().flatten(),
        })
        .collect();
      Table::new(
        [
          TableColumn::new("frames", "Frames")
            .width(144.0)
            .cell(|stream: &Stream| {
              // A plain picture, as a tile of a grid of streams is, drawn smaller than its frames.
              Row::new()
                .padding_vertical(8.0)
                .child(
                  lurq::components::Video::new(stream.image.clone())
                    .stretch()
                    .size(Dimension::Px(112.0), Dimension::Px(63.0)),
                )
                .into()
            }),
          TableColumn::new("gives", "A frame is")
            .share(1.0)
            .min_width(120.0)
            .wrap(true)
            .text(|stream: &Stream| stream.gives.to_owned()),
          TableColumn::new("on", "Decoded on")
            .width(124.0)
            .wrap(true)
            .muted(true)
            .text(|stream: &Stream| stream.on.to_owned()),
          TableColumn::new("way", "Drawn").width(148.0).cell(|stream: &Stream| {
            let (label, variant) = match stream.way {
              Some(lurq::images::CardFramesWay::Card) => ("On the card", BadgeVariant::Success),
              Some(lurq::images::CardFramesWay::Memory) => ("Through memory", BadgeVariant::Warning),
              Some(lurq::images::CardFramesWay::Nowhere) => ("Not drawn", BadgeVariant::Destructive),
              None => return quiet("…", 13.0).into(),
            };
            Badge::new(label).variant(variant).into()
          }),
        ],
        streams,
      )
      .label("Decoders and how their frames come to the screen")
      .into()
    };
    // On macOS and iOS: the frames of a thread of the gallery, drawn where it wrote them.
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    let decoded: Element = {
      let image = self
        .decoding
        .get()
        .then(|| {
          self
            .decoder
            .lock()
            .expect("the decoder of the page")
            .as_ref()
            .map(|decoded| decoded.image.clone())
        })
        .flatten();
      match image {
        Some(image) => items()
          .child(item(
            lurq::components::Video::new(image)
              .stretch()
              .size(Dimension::Px(320.0), Dimension::Px(180.0)),
            "320 × 180, thirty frames a second · drawn on the card, where the decoder wrote them",
          ))
          .into(),
        None => quiet("Nothing draws frames of the card in this run: it takes Metal.", 13.0).into(),
      }
    };
    #[cfg(not(any(windows, target_os = "macos", target_os = "ios")))]
    let decoded: Element = quiet(
      "Frames of a decoder of the app's own are drawn on Windows, macOS and iOS so far.",
      13.0,
    )
    .into();
    // The touch form on its side is as wide as a phone on its side: where the stage is narrower it scrolls.
    let upright = ctx.mount::<Demo>(Scene::Touch);
    let side = ctx.mount::<Demo>(Scene::TouchSide);
    let touch = Column::new()
      .width(FILL)
      .spacing(28.0)
      .overflow_visible()
      .child(item(
        upright,
        "upright · a tap shows and hides the controls, two on a side seek 10 s",
      ))
      .child(
        Column::new()
          .width(FILL)
          .spacing(9.0)
          .child(fit("touch-side", room, 780.0, side))
          .child(quiet(
            "on its side, as in fullscreen · the title, the speed, 44 at each side for the notch",
            12.0,
          )),
      );
    let bare = items().child(item(
      state().controls(false).mount(ctx),
      "the picture alone, as in a feed",
    ));
    let feed = self.feed().key("feed").mount(ctx);
    let own = items().child(item(feed, "your own controls in the overlay"));
    let rows = [
      (
        "media",
        "impl Into<Element>",
        "",
        "What is drawn as the picture, filling the player: the frame the app has.",
      ),
      (
        "poster",
        "impl Into<Element>",
        "",
        "Drawn over the picture before the first play, with the length at its end.",
      ),
      (
        "child / children",
        "impl Into<Element>",
        "",
        "The app's own layer over the whole picture, with the controls and without them.",
      ),
      (
        "controls",
        "bool",
        "true",
        "The bar and the middle of the picture. Without them the player is its picture and the app's layer.",
      ),
      (
        "playing",
        "Signal<bool>",
        "",
        "Whether it plays. Also playing_controlled(bool, handler) and on_playing_change.",
      ),
      (
        "position",
        "Signal<f32>",
        "",
        "Where it is, in seconds. Also position_controlled; on_position_change as the handle moves, on_position_change_end where it is let go.",
      ),
      (
        "duration",
        "f64",
        "",
        "How long it is. Until said, the time reads -:-- / -:--.",
      ),
      ("buffered", "f64", "", "How far it is loaded, drawn in the seek bar."),
      (
        "preview",
        "Fn(f64) -> Element",
        "",
        "The frame for a time, shown over the time under the pointer of the seek bar (mounted).",
      ),
      (
        "volume",
        "Signal<f32>",
        "",
        "How loud, from 0 to 1. Also volume_controlled, on_volume_change, on_volume_change_end.",
      ),
      (
        "muted",
        "Signal<bool>",
        "",
        "Whether the sound is off; the volume is kept. Also muted_controlled and on_muted_change.",
      ),
      (
        "rate / rates",
        "Signal<f32>, &[f32]",
        "1, MEDIA_RATES",
        "How fast it plays, and the speeds offered. Also rate_controlled and on_rate_change.",
      ),
      (
        "captions",
        "Signal<bool>",
        "",
        "Whether the captions show. Also captions_controlled and on_captions_change.",
      ),
      (
        "caption_text / captions_track",
        "&str",
        "",
        "The line to show now, and the track's name the settings say.",
      ),
      (
        "source",
        "&Source",
        "",
        "What it plays (feature media): a mounted player decodes it and draws its frames, and its time, its \
         length and whether it plays are the media's. Its sound is heard, as loud as the volume says.",
      ),
      (
        "touch",
        "bool",
        "by the pointer",
        "The touch form: larger controls over a scrim, shown and hidden by a tap, two taps on a side seeking ten \
         seconds. On its side while in fullscreen. Unless told, it is the touch form under a finger \
         (Pointer::Coarse), whatever its width.",
      ),
      (
        "title / description / on_back",
        "&str / &str / impl Fn()",
        "",
        "The touch form: what plays, said at its top on its side, and what its first button calls.",
      ),
      (
        "picture_in_picture_window",
        "bool",
        "true",
        "Whether picture in picture opens its small window itself: no frame, kept over the other windows, the \
         player in it and a button that brings it back. Without it the value is told and the window is the app's.",
      ),
      (
        "fullscreen_fills",
        "VideoFullscreen",
        "Window",
        "What a player in fullscreen fills: its window as it is (Window); its window taken to the whole screen \
         (Screen); or nothing, where the app lays it out itself (App). Escape asks to leave.",
      ),
      (
        "caption_cues",
        "&MediaCaptions",
        "",
        "A track read from a WebVTT or a SubRip file (MediaCaptions::parse): the player shows what it says at its \
         position.",
      ),
      (
        "qualities / quality",
        "&[&str], Signal<Option<usize>>",
        "None",
        "The qualities offered after Auto, and the one it plays in. Also quality_controlled and on_quality_change.",
      ),
      (
        "fullscreen",
        "Signal<bool>",
        "",
        "Whether it fills the screen; its button then exits. Also fullscreen_controlled and on_fullscreen_change.",
      ),
      (
        "picture_in_picture",
        "Signal<bool>",
        "",
        "Whether it plays in a small window of its own. Also its _controlled form and handler.",
      ),
      (
        "waiting",
        "bool",
        "false",
        "It waits for what it plays: a spinner in the middle.",
      ),
      (
        "ended",
        "bool",
        "false",
        "It played to the end: the veil and replay. A press of replay tells playing true.",
      ),
      (
        "live",
        "bool",
        "false",
        "No end: the LIVE badge for the time, the bar full, no speed.",
      ),
      (
        "error / error_description",
        "&str",
        "",
        "That it cannot play: the veil, the title and the line in the middle, and no bar.",
      ),
      (
        "on_retry",
        "impl Fn()",
        "",
        "The error's button, drawn once this is given.",
      ),
      (
        "size",
        "VideoPlayerSize",
        "from the width",
        "Lg, Md or Sm in place of the form its width gives it (Md under 400, Sm under 280).",
      ),
      (
        "width / height",
        "Length",
        "640 × 360",
        "Its size, in place of its form's. With a width and no height it is as high as its aspect makes it.",
      ),
      (
        "max_width",
        "Length",
        "",
        "The most it is wide: narrower where what holds it is, a window as narrow as a phone, and its aspect holds.",
      ),
      (
        "aspect",
        "f32",
        "16.0 / 9.0",
        "The shape of the picture, its width over its height: 9.0 / 16.0 for an upright one.",
      ),
      (
        "force_menu / force_controls_shown",
        "Option<..>",
        "None",
        "A menu drawn open, or the bar shown or faded, with nothing pointing at it.",
      ),
      (
        "mount",
        "&mut Ctx",
        "",
        "Mounts it at once: it keeps its width, the timer that fades the bar, its menus and its tooltips. \
         Given to a tree as it is (.child(player)), it is the same component.",
      ),
    ];
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Media", "Video Player"],
        title: "Video Player",
        description: "A picture the app gives, the controls over it, and a layer of the app's own: an ordinary \
                      player, or only the picture as in a vertical feed.",
        import: Some("use lurq::ui::VideoPlayer;"),
        status: Some("Preview"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of VideoPlayer is a control on the right. With no source nothing is decoded: a clock of the \
           page's own moves the position on while it plays, and at the end it stops and says it ended. With a \
           source (the gallery's clip from its bytes, a file by its path, a video at an address) the player \
           really plays it, and its picture, its length and where it is are the video's own. A file of your own \
           is its path, typed in place of the gallery's. Every press of the player tells the page what it is to \
           be, and the page sets it.",
          playground,
        ),
        Section::new(
          "States",
          "Paused the play disc stands in the middle and the bar stays; playing, the bar fades out 3 s after the \
           pointer rests. Ended and failed are under a veil. Live has its badge for the time.",
          self.frame.demo(
            0,
            states,
            ".ended(true)",
            &[
              "VideoPlayer::new().media(frame).duration(250.0).position(position)",
              "    .ended(true)",
              "",
              "VideoPlayer::new().media(frame)",
              "    .error(\"This video can't be played\")",
              "    .error_description(\"Check the connection and try again.\")",
              "    .on_retry(move || retry())",
            ],
          ),
        ),
        Section::new(
          "Playing a file",
          "Given a source (a file, bytes, a resource, an address), a mounted player decodes it with the system's \
           decoder and draws its frames: play, pause, the seek bar, the speed and the keys move the video itself. \
           Its sound is heard through the system's output, and the volume and the mute are its own.",
          self.frame.demo(
            5,
            real,
            ".source(&Source::file(\"clip.mp4\"))",
            &[
              "VideoPlayer::new()",
              "    .source(&Source::file(\"clips/lake.mp4\"))",
              "    .width(640.0)",
              "    .max_width(Length::Fill)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Frames of a decoder of your own",
          if cfg!(windows) {
            "An app that decodes by itself (a stream, a codec the system has not) gives lurq the textures its \
             frames are in, and lurq draws them where they are: a frame is never in the memory of the program. \
             Each row is a decoder, which a thread of the gallery stands for: the three ways one gives a \
             frame, and one on another card than the window's, as on a laptop with two. The last column is \
             what frames.way() says: on the card, the frame is drawn where the decoder put it, with no copy; \
             through memory, it is copied once, which is how a frame of another card comes. The textures are \
             larger than the picture, as a codec's are, with pink past the picture, none of which shows."
          } else {
            "An app that decodes by itself (a stream, a codec the system has not, a camera) gives lurq the \
             buffers its frames are in, and lurq draws them where they are, on Metal: a frame is not copied on \
             its way to the screen. A thread of the gallery stands for the decoder: it writes a moving test \
             picture into buffers of the system's that Metal makes textures of, three in turn, and shows each. \
             The system's own decoders give such buffers already, and a video lurq plays comes the same way."
          },
          self.frame.demo(
            6,
            decoded,
            if cfg!(windows) {
              "Video::new(NativeImageData::from_card_frames(w, h, frames).image_data())"
            } else {
              "Video::new(NativeImageData::from_cv_pixel_buffer_frames(w, h, frames).image_data())"
            },
            if cfg!(windows) {
              &[
                "// once: the textures the decoder writes, by their shared handles, and its fence",
                "let frames = unsafe { CardFrames::with_fence(surfaces, fence) };",
                "let image = NativeImageData::from_card_frames(1920, 1080, frames.clone()).image_data();",
                "",
                "// the decoder's thread, for each frame",
                "frames.show(",
                "  CardFrame::of(surface)",
                "    .crop(CardCrop { left: 0, top: 0, width: 1920, height: 1080 })",
                "    .ready_at(fence_value)",
                "    .lease(gives_the_surface_back),",
                ");",
                "",
                "// the screen",
                "Video::new(image)",
              ]
            } else {
              &[
                "// once: the frames, and the image of them",
                "let frames = CvPixelBufferFrames::new();",
                "let image = NativeImageData::from_cv_pixel_buffer_frames(1920, 1080, frames.clone()).image_data();",
                "",
                "// the decoder's thread, for each frame: a buffer the system's decoder gave,",
                "frames.show(unsafe { MacosCvPixelBuffer::retain(pixel_buffer) });",
                "// or one of the app's own, written into from memory",
                "let buffer = MacosCvPixelBuffer::nv12(1920, 1080)?;",
                "buffer.write_nv12(|gray, gray_stride, color, color_stride| decode_into(gray, color));",
                "frames.show(buffer);",
                "",
                "// the screen",
                "Video::new(image)",
              ]
            },
          ),
        ),
        Section::new(
          "Sizes",
          "The form follows the player's width: compact under 400, minimal under 280. The large form folds its \
           volume into its button under 480. size says the form where the app wants another.",
          self.frame.demo(
            1,
            sizes,
            ".size(VideoPlayerSize::Md)",
            &[
              "VideoPlayer::new().media(frame).width(640.0).height(360.0)",
              "VideoPlayer::new().media(frame).width(320.0).height(180.0)",
              "VideoPlayer::new().media(frame).width(200.0).height(113.0)",
            ],
          ),
        ),
        Section::new(
          "Touch",
          "For a finger: round buttons of 44 over a scrim, ten seconds back and on beside a larger play, the time \
           and fullscreen over a seek bar with a thumb of 16, and no volume. In fullscreen it is the form for a \
           phone on its side. It is the form under a finger (Pointer::Coarse, as on iOS and Android), and \
           touch(true) chooses it anywhere: here on a desktop.",
          self.frame.demo(
            4,
            touch,
            ".touch(true)",
            &[
              "VideoPlayer::new()",
              "    .media(frame)",
              "    .touch(true)",
              "    .title(\"Night Drive\")",
              "    .description(\"Aurora Lane · Low Tide\")",
              "    .on_back(|| minimize())",
            ],
          ),
        ),
        Section::new(
          "Without controls",
          "The picture and the app's layer alone, with no bar and nothing in the middle.",
          self.frame.demo(
            2,
            bare,
            ".controls(false)",
            &["VideoPlayer::new().media(frame).controls(false)"],
          ),
        ),
        Section::new(
          "Your own controls",
          "With no controls of its own, the player holds the app's over its picture: here made of the system's \
           parts, the overlay play button and round buttons on dark discs.",
          self.frame.demo(
            3,
            own,
            ".controls(false).child(layer)",
            &[
              "VideoPlayer::new()",
              "    .media(frame)",
              "    .width(243.0)",
              "    .aspect(9.0 / 16.0)",
              "    .controls(false)",
              "    .child(",
              "        Stack::new()",
              "            .child(PlayButton::new().variant(PlayButtonVariant::Overlay).on_change(play))",
              "            .child(your_column_of_actions),",
              "    )",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new("Props", "", props_table(&rows, &[], self.frame.room())),
      ],
    )
  }
}
