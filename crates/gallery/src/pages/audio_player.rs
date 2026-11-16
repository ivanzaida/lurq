//! The page of the audio player: its playground, driven by a clock of the page's own while it plays, then the card's
//! states, the compact row, the voice message, the mini player along the bottom of a window, and its props.

use std::{sync::Arc, time::Duration};

use lurq::{
  app::{
    component::Component,
    ctx::{Ctx, Interval},
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify},
  node::{Element, gradient::Gradient},
  ui::{AudioPlayer, AudioPlayerVariant, AudioRepeat, Interaction},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, line, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// The forms, in the order of the select.
const VARIANTS: [(&str, AudioPlayerVariant, &str); 4] = [
  ("Full", AudioPlayerVariant::Full, "AudioPlayerVariant::Full"),
  ("Compact", AudioPlayerVariant::Compact, "AudioPlayerVariant::Compact"),
  ("Voice", AudioPlayerVariant::Voice, "AudioPlayerVariant::Voice"),
  ("Mini", AudioPlayerVariant::Mini, "AudioPlayerVariant::Mini"),
];

/// How long what each form plays is: a track of 4:10, a voice message of 0:42.
fn length_of(variant: usize) -> f32 {
  if variant == 2 { 42.0 } else { 250.0 }
}

/// The widths of the window the mini player is docked in.
const WINDOWS: [(&str, f32); 3] = [("640", 640.0), ("520 (under 560)", 520.0), ("400 (under 480)", 400.0)];

/// The loudness of a voice message, one for each of 36 bars.
const PEAKS: [f32; 36] = [
  0.25, 0.4, 0.7, 0.55, 0.9, 1.0, 0.65, 0.45, 0.8, 0.6, 0.35, 0.5, 0.85, 0.7, 0.4, 0.3, 0.55, 0.75, 0.95, 0.6, 0.4,
  0.5, 0.7, 0.45, 0.3, 0.6, 0.8, 0.5, 0.35, 0.65, 0.9, 0.55, 0.4, 0.3, 0.45, 0.25,
];

/// The artwork the board draws: a gradient from its chart colors, `side` square.
fn artwork(side: f32) -> Row {
  Row::new()
    .width(side)
    .height(side)
    .background_gradient(Gradient::linear(
      135.0,
      [PaletteColor::Chart4, PaletteColor::Chart1, PaletteColor::Chart3],
    ))
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

/// The gallery's own track: twenty-four seconds of plucked notes, in the gallery's binary.
#[cfg(not(target_arch = "wasm32"))]
const TRACK: &[u8] = include_bytes!("../../assets/media/track.m4a");

/// A sound anyone may fetch: the address the playground plays until another is typed.
const ADDRESS: &str = "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3";

/// What the playground's player is given to play, in the order of the select: nothing, so that the page's
/// clock moves it; the gallery's track from its bytes or from a file; a sound at an address.
const SOURCES: [(&str, usize); 4] = [("None", 0), ("Bytes", 1), ("A file", FILED), ("An address", ADDRESSED)];

/// The places of a file and of an address among the sources.
const FILED: usize = 2;
const ADDRESSED: usize = 3;

/// The track as bytes: one source for every render, so that it is one sound.
#[cfg(not(target_arch = "wasm32"))]
fn track_bytes() -> lurq::resources::Source {
  static TRACK_BYTES: std::sync::OnceLock<lurq::resources::Source> = std::sync::OnceLock::new();
  TRACK_BYTES
    .get_or_init(|| lurq::resources::Source::bytes(TRACK.to_vec()))
    .clone()
}

/// Where the track is as a file on this machine's disk, written there once: the path the playground plays as a
/// file until another is typed.
#[cfg(not(target_arch = "wasm32"))]
fn track_path() -> String {
  static TRACK_FILE: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
  TRACK_FILE
    .get_or_init(|| {
      let folder = std::env::temp_dir().join("lurq-gallery");
      let path = folder.join("track.m4a");
      let _ = std::fs::create_dir_all(&folder);
      let _ = std::fs::write(&path, TRACK);
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
    1 => Some(track_bytes()),
    FILED if !path.trim().is_empty() => Some(lurq::resources::Source::file(path.trim())),
    ADDRESSED if !address.trim().is_empty() => Some(lurq::resources::Source::url(address.trim())),
    _ => None,
  }
}

/// What a key in a path or an address does: what plays stops, so that half of one is not asked for.
#[cfg(not(target_arch = "wasm32"))]
fn stops(playing: &Signal<bool>) -> impl Fn() + Send + Sync + 'static {
  let playing = playing.clone();
  move || {
    if playing.get_untracked() {
      playing.set(false);
    }
  }
}

/// Under the address: whether its sound is kept and how much is kept in all, and what lets go of it.
#[cfg(not(target_arch = "wasm32"))]
fn kept_line(source: Option<&lurq::resources::Source>, kept: u64) -> Element {
  let cache = lurq::media::sound_cache();
  let holds = source.is_some_and(|source| cache.holds(source));
  let megabytes = kept as f64 / (1024.0 * 1024.0);
  let said = match (holds, kept) {
    (true, _) => format!("kept · {megabytes:.1} MB of sounds"),
    (false, 0) => "loaded when it is first played, then kept".to_owned(),
    (false, _) => format!("not kept yet · {megabytes:.1} MB of sounds"),
  };
  let mut line = Row::new()
    .width(FILL)
    .min_height(28.0)
    .spacing(8.0)
    .align_items(Alignment::Center)
    .child(quiet(&said, 12.0).flex(1.0).min_width(0.0));
  if let (true, Some(source)) = (holds, source) {
    let source = source.clone();
    line = line.child(
      lurq::ui::Button::new("Forget")
        .variant(lurq::ui::ButtonVariant::Ghost)
        .size(lurq::ui::ButtonSize::Sm)
        .on_click(move |_| {
          lurq::media::sound_cache().forget(&source);
        }),
    );
  }
  line.into()
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new()
    .width(FILL)
    .spacing(28.0)
    .wrap()
    .align_items(Alignment::Start)
    .overflow_visible()
}

/// A window `width` wide that stands in for an app's, its content above and `docked` along its bottom.
fn window(width: f32, docked: impl Into<Element>) -> Column {
  Column::new()
    .width(width)
    .max_width(FILL)
    .height(220.0)
    .clip()
    .rounded(RadiusSize::Lg)
    .border(line())
    .background(PaletteColor::Canvas)
    .child(
      Column::new()
        .width(FILL)
        .flex(1.0)
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .child(quiet("The window's content", 13.0)),
    )
    .child(docked)
}

/// A state of the board that a player of a section starts in.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Scene {
  CardPaused,
  CardPlaying,
  CardLoading,
  CardError,
  RowPlaying,
  RowWave,
  RowLoading,
  RowError,
  RowDisabled,
  RowHover,
  RowFocus,
  VoicePlaying,
  VoiceUnheard,
  /// The mini player, in a window this wide.
  Mini(u16),
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for Scene {
  fn inspect(&self, formatter: &mut lurq::app::component::DevtoolsFormatter<'_>) {
    formatter.value_debug(self);
  }
}

impl Scene {
  fn variant(self) -> AudioPlayerVariant {
    match self {
      Self::CardPaused | Self::CardPlaying | Self::CardLoading | Self::CardError => AudioPlayerVariant::Full,
      Self::VoicePlaying | Self::VoiceUnheard => AudioPlayerVariant::Voice,
      Self::Mini(_) => AudioPlayerVariant::Mini,
      _ => AudioPlayerVariant::Compact,
    }
  }

  /// How long what it plays is: a voice message of 0:42, a track of 4:10.
  fn length(self) -> f32 {
    match self {
      Self::RowWave | Self::VoicePlaying | Self::VoiceUnheard => 42.0,
      _ => 250.0,
    }
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
  shuffle: Signal<bool>,
  repeat: Signal<AudioRepeat>,
  error: Signal<bool>,
  unheard: Signal<bool>,
  _clock: Interval,
}

impl Component for Demo {
  type Props = Scene;

  fn create(ctx: &mut Ctx) -> Self {
    let scene = *ctx.props::<Scene>();
    let playing = ctx.signal(matches!(
      scene,
      Scene::CardPlaying | Scene::RowPlaying | Scene::RowWave | Scene::VoicePlaying | Scene::Mini(_)
    ));
    let position = ctx.signal(match scene {
      Scene::CardPlaying => 158.0,
      Scene::RowWave | Scene::VoicePlaying => 17.0,
      Scene::CardLoading | Scene::CardError | Scene::RowLoading | Scene::RowError | Scene::VoiceUnheard => 0.0,
      _ => 84.0,
    });
    let (rate, error) = (
      ctx.signal(1.0_f32),
      ctx.signal(matches!(scene, Scene::CardError | Scene::RowError)),
    );
    let (loading, length) = (matches!(scene, Scene::CardLoading | Scene::RowLoading), scene.length());
    // While it plays its clock moves it on, and from the end it goes round: it stays the state it shows.
    let tick = Duration::from_millis(250);
    let clock = ctx.create_interval(tick, {
      let (playing, position, rate, error) = (playing.clone(), position.clone(), rate.clone(), error.clone());
      move || {
        if !playing.get_untracked() || loading || error.get_untracked() {
          return;
        }
        let next = position.get_untracked() + tick.as_secs_f32() * rate.get_untracked();
        position.set(if next >= length { 0.0 } else { next });
      }
    });
    clock.start();
    Self {
      playing,
      position,
      volume: ctx.signal(if matches!(scene, Scene::Mini(_)) { 0.6 } else { 0.8 }),
      muted: ctx.signal(false),
      rate,
      shuffle: ctx.signal(scene == Scene::CardPlaying),
      repeat: ctx.signal(if scene == Scene::CardPlaying {
        AudioRepeat::One
      } else {
        AudioRepeat::Off
      }),
      error,
      unheard: ctx.signal(scene == Scene::VoiceUnheard),
      _clock: clock,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let scene = *ctx.props::<Scene>();
    let (variant, length) = (scene.variant(), scene.length());
    let mut player = AudioPlayer::new(self.position.clone(), length)
      .variant(variant)
      .playing(self.playing.clone())
      .volume(self.volume.clone())
      .muted(self.muted.clone())
      .rate(self.rate.clone())
      .shuffle(self.shuffle.clone())
      .repeat(self.repeat.clone())
      .loading(matches!(scene, Scene::CardLoading | Scene::RowLoading))
      .unheard(self.unheard.get())
      .disabled(scene == Scene::RowDisabled);
    player = match variant {
      AudioPlayerVariant::Full => player
        .title("Night Drive")
        .description("Aurora Lane · Low Tide")
        .media(artwork(64.0))
        .buffered((self.position.get() + 56.0).min(length)),
      AudioPlayerVariant::Mini => player
        .title("Night Drive")
        .description("Aurora Lane")
        .media(artwork(36.0)),
      AudioPlayerVariant::Voice => player.peaks(&PEAKS),
      AudioPlayerVariant::Compact if scene == Scene::RowWave => player.peaks(&PEAKS),
      AudioPlayerVariant::Compact => player.buffered((self.position.get() + 56.0).min(length)),
    };
    player = match scene {
      Scene::RowHover => player.force(Some(Interaction::Hovered)),
      Scene::RowFocus => player.force(Some(Interaction::FocusVisible)),
      _ => player,
    };
    if self.error.get() {
      let (error, playing) = (self.error.clone(), self.playing.clone());
      player = player
        .error(Some(if variant == AudioPlayerVariant::Full {
          "This track can't be played."
        } else {
          "Couldn't load the audio."
        }))
        // Trying again, it loads this time and plays.
        .on_retry(move || {
          error.set(false);
          playing.set(true);
        });
    }
    let unheard = self.unheard.clone();
    let player = player
      .on_playing_change(move |to| {
        if to && unheard.get_untracked() {
          unheard.set(false);
        }
      })
      .key("demo")
      .mount(ctx);
    match scene {
      Scene::Mini(width) => window(f32::from(width), player).into(),
      _ => player,
    }
  }
}

pub struct AudioPlayerPage {
  frame: Frame,
  playground: Playground,
  variant: Signal<usize>,
  window: Signal<usize>,
  title: Signal<String>,
  description: Signal<String>,
  artwork: Signal<bool>,
  playing: Signal<bool>,
  position: Signal<f32>,
  volume: Signal<f32>,
  muted: Signal<bool>,
  rate: Signal<f32>,
  shuffle: Signal<bool>,
  repeat: Signal<AudioRepeat>,
  waveform: Signal<bool>,
  loading: Signal<bool>,
  error: Signal<bool>,
  unheard: Signal<bool>,
  disabled: Signal<bool>,
  /// What the playground's player plays, of [`SOURCES`]; the address typed for the last of them; and how many
  /// bytes of sounds are kept, which the clock reads again for the line under the address.
  source: Signal<usize>,
  address: Signal<String>,
  /// The path typed for a file: the gallery's own track on this disk until another is.
  path: Signal<String>,
  #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
  kept: Signal<u64>,
  /// The page's clock: while the playground plays with no source, it moves the position on, and stops it at
  /// the end; and it reads how much is kept.
  _clock: Interval,
}

impl AudioPlayerPage {
  /// The playground: the player as the panel's props make it, bound to the page's signals, mounted.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let variant_index = self.variant.get().min(VARIANTS.len() - 1);
    let (variant_name, variant, variant_code) = VARIANTS[variant_index];
    let length = length_of(variant_index);
    let window_index = self.window.get().min(WINDOWS.len() - 1);
    let (title, description) = (self.title.get(), self.description.get());
    let (artwork_on, waveform) = (self.artwork.get(), self.waveform.get());
    let (loading, error, unheard, disabled) = (
      self.loading.get(),
      self.error.get(),
      self.unheard.get(),
      self.disabled.get(),
    );
    let full = variant == AudioPlayerVariant::Full;
    let mini = variant == AudioPlayerVariant::Mini;
    let texts = full || mini;
    let waves = variant == AudioPlayerVariant::Voice || (variant == AudioPlayerVariant::Compact && waveform);

    // What the props make, with nothing listening: what a window of its own shows, and the base of the live one.
    let plain = {
      let (title, description) = (title.clone(), description.clone());
      move || {
        let mut player = AudioPlayer::controlled(84.0_f64.min(f64::from(length)), length, |_| {})
          .variant(variant)
          .loading(loading)
          .error(error.then_some("This track can't be played."))
          .unheard(unheard && variant == AudioPlayerVariant::Voice)
          .disabled(disabled);
        if texts {
          player = player.title(&title).description(&description);
          if artwork_on {
            player = player.media(artwork(if mini { 36.0 } else { 64.0 }));
          }
        }
        if waves {
          player = player.peaks(&PEAKS);
        }
        player
      }
    };

    // What it really plays, where a source is chosen: where it is and how long are then the sound's.
    let chosen = self.source.get().min(SOURCES.len() - 1);
    #[cfg(not(target_arch = "wasm32"))]
    let source = source_of(chosen, &self.path.get(), &self.address.get());
    #[cfg(target_arch = "wasm32")]
    let source: Option<()> = None;
    let sourced = chosen != 0;
    let record = self.playground.recorder();
    let say = |what: &'static str| {
      let record = record.clone();
      move || record(format!("{what}()"))
    };
    let position = self.position.clone();
    let mut player = AudioPlayer::new(self.position.clone(), if sourced { 0.0 } else { length })
      .variant(variant)
      .loading(loading)
      .error(error.then_some("This track can't be played."))
      .unheard(unheard && variant == AudioPlayerVariant::Voice)
      .disabled(disabled)
      .force(self.playground.forced())
      .playing(self.playing.clone())
      .volume(self.volume.clone())
      .muted(self.muted.clone())
      .rate(self.rate.clone())
      .shuffle(self.shuffle.clone())
      .repeat(self.repeat.clone())
      .on_change({
        let record = record.clone();
        move |to| record(format!("on_change({to:.1})"))
      })
      .on_change_end({
        let record = record.clone();
        move |to| record(format!("on_change_end({to:.1})"))
      })
      .on_playing_change({
        let (record, unheard) = (record.clone(), self.unheard.clone());
        move |playing| {
          // What is played is heard.
          if playing {
            unheard.set(false);
          }
          record(format!("on_playing_change({playing})"))
        }
      })
      .on_volume_change({
        let record = record.clone();
        move |volume| record(format!("on_volume_change({volume:.2})"))
      })
      .on_muted_change({
        let record = record.clone();
        move |muted| record(format!("on_muted_change({muted})"))
      })
      .on_rate_change({
        let record = record.clone();
        move |rate| record(format!("on_rate_change({rate})"))
      })
      .on_shuffle_change({
        let record = record.clone();
        move |on| record(format!("on_shuffle_change({on})"))
      })
      .on_repeat_change({
        let record = record.clone();
        move |repeat| record(format!("on_repeat_change(AudioRepeat::{repeat:?})"))
      })
      .on_previous({
        let (said, position) = (say("on_previous"), position.clone());
        move || {
          said();
          position.set(0.0);
        }
      })
      .on_next({
        let (said, position) = (say("on_next"), position.clone());
        move || {
          said();
          position.set(0.0);
        }
      })
      .on_more(say("on_more"))
      .on_queue(say("on_queue"))
      .on_expand(say("on_expand"))
      .on_retry({
        let (said, error) = (say("on_retry"), self.error.clone());
        move || {
          said();
          error.set(false);
        }
      });
    if texts {
      player = player.title(&title).description(&description);
      if artwork_on {
        player = player.media(artwork(if mini { 36.0 } else { 64.0 }));
      }
    }
    if waves {
      player = player.peaks(&PEAKS);
    }
    // The window of its own shows the player with no source: two players of one sound would be heard twice.
    let alone = player.clone().buffered(f64::from(length).min(200.0));
    #[cfg(not(target_arch = "wasm32"))]
    {
      player = match &source {
        Some(source) => player.source(source),
        // The page reads nothing that moves as it plays or is dragged: the player is a component of its own,
        // and a position read here would build the whole page again for every step of a drag.
        None if !sourced => player.buffered(f64::from(length).min(200.0)),
        None => player,
      };
    }
    #[cfg(target_arch = "wasm32")]
    {
      let _ = &source;
      player = player.buffered(f64::from(length).min(200.0));
    }

    let mut code = vec![
      "// position: Signal<f32>, playing: Signal<bool>, ...".to_owned(),
      if sourced {
        "AudioPlayer::new(position, 0.0)".to_owned()
      } else {
        format!("AudioPlayer::new(position, {length:.1})")
      },
    ];
    match chosen {
      1 => code.push("    .source(&Source::bytes(include_bytes!(\"track.m4a\").to_vec()))".to_owned()),
      FILED => code.push(format!("    .source(&Source::file({:?}))", self.path.get().trim())),
      ADDRESSED => code.push(format!("    .source(&Source::url({:?}))", self.address.get().trim())),
      _ => {}
    }
    if !full {
      code.push(format!("    .variant({variant_code})"));
    }
    if texts {
      code.push(format!("    .title({title:?})"));
      code.push(format!("    .description({description:?})"));
      if artwork_on {
        code.push("    .media(artwork)".to_owned());
      }
    }
    if waves {
      code.push("    .peaks(&peaks)".to_owned());
    }
    code.push("    .playing(playing)".to_owned());
    if full || mini {
      code.push("    .volume(volume)".to_owned());
      code.push("    .muted(muted)".to_owned());
    }
    if full || variant == AudioPlayerVariant::Voice {
      code.push("    .rate(rate)".to_owned());
    }
    if full {
      code.push("    .shuffle(shuffle)".to_owned());
      code.push("    .repeat(repeat)".to_owned());
    }
    for (name, on) in [
      ("loading", loading),
      ("unheard", unheard && variant == AudioPlayerVariant::Voice),
      ("disabled", disabled),
    ] {
      if on {
        code.push(format!("    .{name}(true)"));
      }
    }
    if error {
      code.push("    .error(Some(\"This track can't be played.\"))".to_owned());
      code.push("    .on_retry(|| load_again())".to_owned());
    }
    if full {
      code.push("    .on_previous(|| previous()).on_next(|| next())".to_owned());
    } else if mini {
      code.push("    .on_previous(|| previous()).on_next(|| next())".to_owned());
      code.push("    .on_queue(|| show_queue()).on_expand(|| expand())".to_owned());
    }
    code.push("    .mount(ctx)".to_owned());

    let tree = match variant {
      AudioPlayerVariant::Full => vec![
        format!("root          {}", self.playground.measured()),
        "├─ header      artwork 64 · title · description · more 32".to_owned(),
        "├─ progress    seek bar · elapsed / total".to_owned(),
        "├─ transport   shuffle 32 · previous 36 · play 48 · next 36 · repeat 32".to_owned(),
        "└─ footer      volume (96) · speed".to_owned(),
      ],
      AudioPlayerVariant::Compact => vec![
        format!("root          {}", self.playground.measured()),
        "├─ play        32".to_owned(),
        format!("├─ {}", if waveform { "waveform" } else { "seek bar" }),
        "└─ time        mono 11".to_owned(),
      ],
      AudioPlayerVariant::Voice => vec![
        format!("root          {}", self.playground.measured()),
        "├─ play        36".to_owned(),
        "├─ waveform    36 bars".to_owned(),
        "├─ time        mono 11".to_owned(),
        "└─ rate | dot  while it plays | not heard".to_owned(),
      ],
      AudioPlayerVariant::Mini => vec![
        format!("root          {}", self.playground.measured()),
        "├─ track       progress line 2 (4 under the pointer)".to_owned(),
        "└─ bar         54".to_owned(),
        "   ├─ header   artwork 36 · title · description".to_owned(),
        "   ├─ transport previous 32 · play · next 32".to_owned(),
        "   └─ tools    time · volume · queue 32 · expand 32 (from 480)".to_owned(),
      ],
    };

    let variants: Vec<(&str, usize)> = VARIANTS
      .iter()
      .enumerate()
      .map(|(index, item)| (item.0, index))
      .collect();
    let repeats = [
      ("Off", AudioRepeat::Off),
      ("All", AudioRepeat::All),
      ("One", AudioRepeat::One),
    ];
    let windows: Vec<(&str, usize)> = WINDOWS
      .iter()
      .enumerate()
      .map(|(index, item)| (item.0, index))
      .collect();
    let (to_variant, to_repeat, to_window, rewind) = (
      self.variant.clone(),
      self.repeat.clone(),
      self.window.clone(),
      self.position.clone(),
    );
    let mut controls = vec![prop(
      "variant",
      "AudioPlayerVariant",
      self.playground.select(0, &variants, &variant_index, move |index| {
        to_variant.set(index);
        rewind.set(0.0);
      }),
    )];
    if mini {
      controls.push(prop(
        "window",
        "px",
        self
          .playground
          .select(2, &windows, &window_index, move |index| to_window.set(index)),
      ));
    }
    if texts {
      controls.push(prop("title", "&str", field(&self.title)));
      controls.push(prop("description", "&str", field(&self.description)));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
      let (to_source, stop, rewind) = (self.source.clone(), self.playing.clone(), self.position.clone());
      controls.push(prop(
        "source",
        "&Source",
        self.playground.select(3, &SOURCES, &chosen, move |index| {
          // Another sound: what played stops, and the new one starts from its start.
          stop.set(false);
          rewind.set(0.0);
          to_source.set(index);
        }),
      ));
      // A file of the person's own is its path, and a sound on the network its address: typed here.
      if chosen == FILED {
        controls.push(prop(
          "path",
          "impl Into<PathBuf>",
          crate::playground::field_telling(&self.path, stops(&self.playing)),
        ));
      }
      if chosen == ADDRESSED {
        controls.push(prop("address", "&str", crate::playground::field_telling(&self.address, stops(&self.playing))));
        controls.push(kept_line(source.as_ref(), self.kept.get()));
      }
    }
    if full {
      controls.push(prop(
        "repeat",
        "Signal<AudioRepeat>",
        self
          .playground
          .select(1, &repeats, &self.repeat.get(), move |repeat| to_repeat.set(repeat)),
      ));
    }
    controls.push(divider());
    controls.push(flag("playing", &self.playing));
    if texts {
      controls.push(flag("media", &self.artwork));
    }
    if full || mini {
      controls.push(flag("muted", &self.muted));
    }
    if full {
      controls.push(flag("shuffle", &self.shuffle));
    }
    if variant == AudioPlayerVariant::Compact {
      controls.push(flag("peaks", &self.waveform));
    }
    if variant == AudioPlayerVariant::Voice {
      controls.push(flag("unheard", &self.unheard));
    }
    controls.push(flag("loading", &self.loading));
    controls.push(flag("error", &self.error));
    controls.push(flag("disabled", &self.disabled));

    let changed = usize::from(variant_index != 0)
      + usize::from(sourced)
      + usize::from(title != "Night Drive")
      + usize::from(description != "Aurora Lane · Low Tide")
      + [
        !artwork_on,
        self.playing.get(),
        self.muted.get(),
        self.shuffle.get(),
        self.repeat.get() != AudioRepeat::Off,
        waveform,
        loading,
        error,
        unheard,
        disabled,
      ]
      .into_iter()
      .filter(|changed| *changed)
      .count();
    let reset = {
      let (variant, window, title, description) = (
        self.variant.clone(),
        self.window.clone(),
        self.title.clone(),
        self.description.clone(),
      );
      let (artwork, playing, position, volume, muted, rate) = (
        self.artwork.clone(),
        self.playing.clone(),
        self.position.clone(),
        self.volume.clone(),
        self.muted.clone(),
        self.rate.clone(),
      );
      let (shuffle, repeat, waveform, loading, error, unheard, disabled) = (
        self.shuffle.clone(),
        self.repeat.clone(),
        self.waveform.clone(),
        self.loading.clone(),
        self.error.clone(),
        self.unheard.clone(),
        self.disabled.clone(),
      );
      let (source, address, path) = (self.source.clone(), self.address.clone(), self.path.clone());
      move || {
        source.set(0);
        address.set(ADDRESS.to_owned());
        #[cfg(not(target_arch = "wasm32"))]
        path.set(track_path());
        #[cfg(target_arch = "wasm32")]
        let _ = &path;
        variant.set(0);
        window.set(0);
        title.set("Night Drive".to_owned());
        description.set("Aurora Lane · Low Tide".to_owned());
        artwork.set(true);
        playing.set(false);
        position.set(84.0);
        volume.set(0.6);
        muted.set(false);
        rate.set(1.0);
        shuffle.set(false);
        repeat.set(AudioRepeat::Off);
        waveform.set(false);
        loading.set(false);
        error.set(false);
        unheard.set(false);
        disabled.set(false);
      }
    };
    let facts = if sourced {
      format!("{variant_name} · plays its source")
    } else {
      format!(
        "{variant_name} · {} long",
        lurq::ui::media_time_text(f64::from(length), false)
      )
    };
    self.playground.window_shows({
      // Behind a lock: the artwork is an element, which threads do not share.
      let player = std::sync::Mutex::new(alone.force(None));
      move |ctx| {
        let player = player.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone();
        player.mount(ctx)
      }
    });
    let playground = &self.playground;
    self.playground.card(
      ctx,
      Spec {
        title: "Audio Player",
        make: Arc::new(move || plain().build(&lurq::ui::current_theme())),
        shown: if mini {
          let width = WINDOWS[window_index].1;
          Shown::built(move |ctx| window(width, playground.measure(player.mount(ctx)).width(FILL)).into()).measured()
        } else {
          Shown::built(move |ctx| player.mount(ctx))
        },
        fill: true,
        forceable: true,
        facts,
        event: "on_playing_change",
        code,
        tree,
        controls,
        changed,
        reset: Arc::new(reset),
      },
    )
  }
}

impl Component for AudioPlayerPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let (variant, playing, position, rate, repeat) = (
      ctx.signal(0_usize),
      ctx.signal(false),
      ctx.signal(84.0_f32),
      ctx.signal(1.0_f32),
      ctx.signal(AudioRepeat::Off),
    );
    let (source, kept) = (ctx.signal(0_usize), ctx.signal(0_u64));
    // A quarter of a second at a time, at the speed chosen; at the end it stops, or begins again on repeat one.
    let clock = ctx.create_interval(Duration::from_millis(250), {
      let (variant, playing, position, rate, repeat) = (
        variant.clone(),
        playing.clone(),
        position.clone(),
        rate.clone(),
        repeat.clone(),
      );
      let (source, kept) = (source.clone(), kept.clone());
      move || {
        #[cfg(target_arch = "wasm32")]
        let _ = &kept;
        // How much is kept changes when a sound was loaded, and when one was let go.
        #[cfg(not(target_arch = "wasm32"))]
        {
          let now = lurq::media::sound_cache().bytes();
          if kept.get_untracked() != now {
            kept.set(now);
          }
        }
        // A player with a source is moved by its sound, not by this clock.
        if !playing.get_untracked() || source.get_untracked() != 0 {
          return;
        }
        let length = length_of(variant.get_untracked());
        let next = position.get_untracked() + 0.25 * rate.get_untracked();
        if next < length {
          position.set(next);
        } else if repeat.get_untracked() == AudioRepeat::One {
          position.set(0.0);
        } else {
          position.set(length);
          playing.set(false);
        }
      }
    });
    clock.start();
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 4),
      variant,
      window: ctx.signal(0),
      title: ctx.signal("Night Drive".to_owned()),
      description: ctx.signal("Aurora Lane · Low Tide".to_owned()),
      artwork: ctx.signal(true),
      playing,
      position,
      volume: ctx.signal(0.6),
      muted: ctx.signal(false),
      rate,
      shuffle: ctx.signal(false),
      repeat,
      waveform: ctx.signal(false),
      loading: ctx.signal(false),
      error: ctx.signal(false),
      unheard: ctx.signal(false),
      disabled: ctx.signal(false),
      source,
      address: ctx.signal(ADDRESS.to_owned()),
      #[cfg(not(target_arch = "wasm32"))]
      path: ctx.signal(track_path()),
      #[cfg(target_arch = "wasm32")]
      path: ctx.signal(String::new()),
      kept,
      _clock: clock,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    // Each is a player of its own, live from the state the board draws: a press of anything on it is answered.
    let mut cards = items();
    for (scene, caption) in [
      (Scene::CardPaused, "paused"),
      (Scene::CardPlaying, "playing, shuffle on, repeat one"),
      (Scene::CardLoading, "loading"),
      (Scene::CardError, "error: Try again loads it"),
    ] {
      cards = cards.child(item(ctx.mount::<Demo>(scene), caption));
    }
    let mut rows = Column::new().spacing(16.0).overflow_visible();
    for (scene, caption) in [
      (Scene::RowPlaying, "playing"),
      (Scene::RowWave, "a waveform for the bar"),
      (Scene::RowLoading, "loading"),
      (Scene::RowError, "error: the button retries"),
      (Scene::RowDisabled, "disabled"),
      (Scene::RowHover, "hover on the button"),
      (Scene::RowFocus, "focus on the button"),
    ] {
      rows = rows.child(item(ctx.mount::<Demo>(scene), caption));
    }
    let mut voices = items();
    for (scene, caption) in [
      (Scene::VoicePlaying, "playing: the speed, which a press steps"),
      (Scene::VoiceUnheard, "not heard yet: the dot, until it is played"),
    ] {
      voices = voices.child(item(ctx.mount::<Demo>(scene), caption));
    }
    let mut docked = Column::new().width(FILL).spacing(16.0);
    for (width, caption) in [
      (
        640,
        "640 wide: under 690 the volume is its button; from 690 its slider stands beside it",
      ),
      (520, "under 560: no time"),
      (400, "under 480: the track, play and next"),
    ] {
      docked = docked.child(item(ctx.mount::<Demo>(Scene::Mini(width)), caption));
    }

    let rows_table = [
      (
        "new",
        "Signal<f32>, f64",
        "",
        "Bound to the position, in seconds, of something the second number of seconds long. A drag on the bar \
         sets the signal.",
      ),
      (
        "controlled",
        "f64, f64, impl Fn(f64)",
        "",
        "In place of new: the position held by the app, which is told where a person moves it.",
      ),
      (
        "variant",
        "AudioPlayerVariant",
        "Full",
        "Full, Compact (a row of 48), Voice (a voice message) or Mini (docked along the bottom of a window).",
      ),
      (
        "on_change / on_change_end",
        "impl Fn(f64)",
        "",
        "Where a person moves the position, as a drag moves it and where it is let go (or a key's step, or a \
         press of the mini player's line).",
      ),
      (
        "buffered",
        "impl Into<f64>",
        "",
        "How far it is loaded, in seconds: drawn on the bar.",
      ),
      (
        "playing / playing_controlled",
        "Signal<bool> / bool, impl Fn(bool)",
        "false",
        "Whether it plays. A press of play, or Space on the bar or the volume, tells on_playing_change what it \
         is to be.",
      ),
      (
        "volume / volume_controlled",
        "Signal<f32> / f64, impl Fn(f64)",
        "1",
        "The volume, from 0 to 1, with on_volume_change. In the card and the mini player.",
      ),
      (
        "muted / muted_controlled",
        "Signal<bool> / bool, impl Fn(bool)",
        "false",
        "Whether it is muted, with on_muted_change. Muting keeps the volume.",
      ),
      (
        "rate / rate_controlled",
        "Signal<f32> / f64, impl Fn(f64)",
        "1",
        "The speed, with on_rate_change: the card's speed menu, a voice message's badge (1×, 1.5×, 2×).",
      ),
      (
        "shuffle / shuffle_controlled",
        "Signal<bool> / bool, impl Fn(bool)",
        "false",
        "Shuffle, with on_shuffle_change: muted while off, the accent fill while on.",
      ),
      (
        "repeat / repeat_controlled",
        "Signal<AudioRepeat> / AudioRepeat, impl Fn(AudioRepeat)",
        "Off",
        "Off, All or One, with on_repeat_change; a press steps it, and One has a glyph of its own.",
      ),
      (
        "title / description",
        "&str",
        "",
        "What plays and who it is by, each on one line, cut with an ellipsis.",
      ),
      (
        "media",
        "impl Into<Element>",
        "",
        "The artwork, cut to 64 (36 in the mini player); the music glyph on the muted fill where there is none.",
      ),
      (
        "peaks",
        "&[f32]",
        "",
        "The loudness of each bar of a waveform, from 0 to 1: a voice message draws it, the compact row in place \
         of the bar.",
      ),
      (
        "source",
        "&Source",
        "",
        "What it plays (feature media): a file, bytes, a resource or an address. Its time, its length and \
         whether it plays, loads or failed are the sound's. An address is loaded when it is first played, and \
         kept.",
      ),
      (
        "loading",
        "bool",
        "false",
        "Lines for the texts, an empty bar, a spinner in play, previous and next dimmed.",
      ),
      (
        "error",
        "Option<&str>",
        "None",
        "What went wrong: a line in the destructive color in place of the times (of the bar in a row).",
      ),
      (
        "unheard",
        "bool",
        "false",
        "A voice message not heard yet: a dot, and the whole waveform in the color of what is not played.",
      ),
      (
        "on_previous / on_next / on_more / on_queue / on_expand / on_retry",
        "impl Fn()",
        "",
        "The commands of the buttons; each call adds a handler.",
      ),
      (
        "more_anchor",
        "ElementRef",
        "",
        "The ref of the card's more button, which a menu of the app hangs from.",
      ),
      (
        "label / *_label / retry_text",
        "&str",
        "",
        "What the player and each of its buttons are called, and the words of Try again.",
      ),
      ("disabled", "bool", "false", "Half the opacity; it takes nothing."),
      (
        "mount",
        "&mut Ctx",
        "",
        "Mounts it at once: it keeps the tooltips of the buttons, the speed menu, the mini player's form from \
         its width and the time over its line. Given to a tree as it is (.child(player)), it is the same \
         component.",
      ),
    ];
    let playground = self.play(ctx);
    let sections = vec![
        Section::new(
          "Playground",
          "Every prop is a control on the right; the player on the stage is bound to the page's signals, and a \
           clock of the page moves the position on while it plays and stops it at the end. With a source chosen \
           (the gallery's own track from its bytes, a file by its path, a sound at an address) the player really \
           plays it: where it is and how long are then the sound's. A file of your own is its path, typed in \
           place of the gallery's. An address is loaded when it is first played, and kept.",
          playground,
        ),
        Section::new(
          "Card",
          "The board's card: paused; playing with shuffle on and repeat one; loading, with lines for the texts and \
           a spinner in play; and an error, which says what went wrong in place of the times.",
          self.frame.demo(
            0,
            cards,
            "AudioPlayer::new(position, 250.0).title(..).description(..)",
            &[
              "AudioPlayer::new(position, 250.0)",
              "    .title(\"Night Drive\")",
              "    .description(\"Aurora Lane · Low Tide\")",
              "    .media(artwork)",
              "    .playing(playing)",
              "    .shuffle(shuffle)",
              "    .repeat(repeat)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Compact",
          "One row of 48 for a list or a message: play, the bar (or a waveform), the time. After an error the \
           button turns to retry.",
          self.frame.demo(
            1,
            rows,
            "AudioPlayer::new(..).variant(AudioPlayerVariant::Compact)",
            &[
              "AudioPlayer::new(position, 250.0)",
              "    .variant(AudioPlayerVariant::Compact)",
              "    .playing(playing)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Voice message",
          "A play button, the waveform, the time, and the speed while it plays, which a press steps through 1×, \
           1.5× and 2×; a dot while it has not been heard.",
          self.frame.demo(
            2,
            voices,
            "AudioPlayer::new(..).variant(AudioPlayerVariant::Voice).peaks(&peaks)",
            &[
              "AudioPlayer::new(position, 42.0)",
              "    .variant(AudioPlayerVariant::Voice)",
              "    .peaks(&peaks)",
              "    .unheard(!heard)",
              "    .rate(rate)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Mini player",
          "Docked along the bottom of a window, as wide as it: a progress line along its top edge, the track, the \
           transport, the time, the volume, the queue and expand. Under 480 wide, the track, play and next.",
          self.frame.demo(
            3,
            docked,
            "AudioPlayer::new(..).variant(AudioPlayerVariant::Mini)",
            &[
              "Column::new()",
              "    .child(content.flex(1.0))",
              "    .child(",
              "        AudioPlayer::new(position, 250.0)",
              "            .variant(AudioPlayerVariant::Mini)",
              "            .title(\"Night Drive\")",
              "            .description(\"Aurora Lane\")",
              "            .on_queue(|| show_queue())",
              "            .on_expand(|| expand())",
              "            .mount(ctx),",
              "    )",
            ],
          ),
        ),
        Section::new("Props", "", props_table(&rows_table, &["new"], self.frame.room())),
    ];
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Media", "Audio Player"],
        title: "Audio Player",
        description: "Sound without a picture: a full card, a row for lists and messages, a voice message, and a \
                      mini player docked to the bottom of a window. Every value is the app's; a control tells what \
                      it is to be. Given a source, it plays it.",
        import: Some("use lurq::ui::{AudioPlayer, AudioPlayerVariant, AudioRepeat};"),
        status: Some("Preview"),
        since: Some("0.63"),
        source: file!(),
      },
      sections,
    )
  }
}
