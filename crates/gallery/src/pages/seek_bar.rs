//! The page of the seek bar: its playground, driven by a clock of the page's own while it plays; the bar playing,
//! its states, over a picture, with chapters, with a preview over the pointer, and what the keys do.

use std::{sync::Arc, time::Duration};

use lurq::{
  app::{
    component::Component,
    ctx::{Ctx, Interval},
    theme::RadiusSize,
  },
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::{Element, Gradient, GradientStop, color::Color},
  ui::{Button, ButtonSize, ButtonVariant, Interaction, Kbd, MediaTime, SeekBar, SeekBarSize, icons, media_time_text},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// Something on a stage with what it is under it, as wide as the stage.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .width(FILL)
    .spacing(10.0)
    .overflow_visible()
    .child(Row::new().width(FILL).overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

/// How long what the playground plays is.
const DURATIONS: [(&str, f64); 3] = [("4:10", 250.0), ("0:45", 45.0), ("2:10:00", 7800.0)];
/// How many seconds an arrow key moves it.
const STEPS: [(&str, f64); 3] = [("5", 5.0), ("10", 10.0), ("1", 1.0)];
const SIZES: [(&str, SeekBarSize); 2] = [("Md", SeekBarSize::Md), ("Lg", SeekBarSize::Lg)];
/// Where the chapters of the playground's recording start, as fractions of it.
const CHAPTERS: [f64; 3] = [0.28, 0.6, 0.84];
/// How far past the position the page's clock has it loaded.
const AHEAD: f64 = 40.0;
/// How often the page's clock ticks.
const TICK: Duration = Duration::from_millis(100);
/// What each key does to a focused seek bar.
const KEYS: [(&str, &str); 6] = [
  ("Right  Up", "One step later: 5 seconds unless told."),
  ("Left  Down", "One step earlier."),
  ("Page Up", "A tenth of the whole later, in whole steps."),
  ("Page Down", "A tenth of the whole earlier."),
  ("Home", "To the start."),
  ("End", "To the end."),
];

/// A picture for the bar to stand over: the gradient the board draws in place of a frame of a video, darkened
/// toward its bottom as a player's bar is.
pub fn picture(content: impl Into<Element>, height: f32) -> Column {
  Column::new()
    .width(FILL)
    .height(height)
    .rounded(RadiusSize::Lg)
    .overflow_visible()
    .background_gradient(Gradient::linear(
      180.0,
      [
        GradientStop::at(Color::from_hex("#60A5FA"), 0.0),
        GradientStop::at(Color::from_hex("#DBEAFE"), 0.55),
        GradientStop::at(Color::from_hex("#FEF3C7"), 0.8),
        GradientStop::at(Color::from_hex("#2DD4BF"), 1.0),
      ],
    ))
    .justify(lurq::layout::layout_kind::Justify::End)
    .child(
      Column::new()
        .width(FILL)
        .padding_horizontal(12.0)
        .padding_top(24.0)
        .padding_bottom(12.0)
        .overflow_visible()
        .background_gradient(Gradient::linear(
          180.0,
          [
            GradientStop::at(Color::from_hex("#00000000"), 0.0),
            GradientStop::at(Color::from_hex("#000000B3"), 1.0),
          ],
        ))
        .child(content),
    )
}

/// A frame of the recording at `time`: what the app's preview gives, here a gradient that turns with the time.
fn frame_at(time: f64, duration: f64) -> Element {
  let along = if duration > 0.0 { (time / duration) as f32 } else { 0.0 };
  Column::new()
    .size(FILL, FILL)
    .justify(lurq::layout::layout_kind::Justify::End)
    .padding(6.0)
    .background_gradient(Gradient::linear(
      120.0 + 180.0 * along,
      [
        GradientStop::at(Color::from_hex("#60A5FA"), 0.0),
        GradientStop::at(Color::from_hex("#FEF3C7"), 0.6),
        GradientStop::at(Color::from_hex("#2DD4BF"), 1.0),
      ],
    ))
    .into()
}

pub struct SeekBarPage {
  frame: Frame,
  playground: Playground,
  position: Signal<f32>,
  playing: Signal<bool>,
  duration: Signal<f64>,
  step: Signal<f64>,
  size: Signal<SeekBarSize>,
  label: Signal<String>,
  buffered: Signal<bool>,
  chapters: Signal<bool>,
  on_media: Signal<bool>,
  preview: Signal<bool>,
  disabled: Signal<bool>,
  empty: Signal<bool>,
  live: Signal<bool>,
  /// The bar of the section "Playing", and whether it plays.
  song: Signal<f32>,
  song_playing: Signal<bool>,
  /// The page's clock: it ticks while either plays.
  _clock: Interval,
}

impl SeekBarPage {
  /// The playground: the bar as the panel's props make it, bound to the position the page's clock advances.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (duration, step, size, label) = (self.duration.get(), self.step.get(), self.size.get(), self.label.get());
    let (buffered, chapters, on_media, preview, disabled) = (
      self.buffered.get(),
      self.chapters.get(),
      self.on_media.get(),
      self.preview.get(),
      self.disabled.get(),
    );
    let (empty, live) = (self.empty.get(), self.live.get());
    let (position, playing) = (f64::from(self.position.get()), self.playing.get());
    let starts: Vec<f64> = CHAPTERS.iter().map(|share| (share * duration).round()).collect();
    let plain = {
      let (bound, label, starts) = (self.position.clone(), label.clone(), starts.clone());
      move || {
        let mut bar = SeekBar::new(bound.clone(), duration)
          .step(step)
          .size(size)
          .label(&label)
          .on_media(on_media)
          .disabled(disabled)
          .empty(empty)
          .live(live);
        if buffered {
          bar = bar.buffered((f64::from(bound.get_untracked()) + AHEAD).min(duration));
        }
        if chapters {
          bar = bar.chapters(&starts);
        }
        if preview {
          bar = bar.preview(move |time| frame_at(time, duration));
        }
        bar
      }
    };
    let (record, record_end, record_hover) = (
      self.playground.recorder(),
      self.playground.recorder(),
      self.playground.recorder(),
    );
    let bar = plain()
      .force(self.playground.forced())
      .on_change(move |to| record(format!("on_change({to})")))
      .on_change_end(move |to| record_end(format!("on_change_end({to})")))
      .on_hover(move |time| record_hover(format!("on_hover({time:?})")));

    let mut code = vec![
      "// position: Signal<f32>, in seconds".to_owned(),
      format!("SeekBar::new(position.clone(), {duration:?})"),
    ];
    if buffered {
      code.push("    .buffered(loaded)".to_owned());
    }
    if chapters {
      code.push(format!("    .chapters(&{starts:?})"));
    }
    if step != 5.0 {
      code.push(format!("    .step({step:?})"));
    }
    if size == SeekBarSize::Lg {
      code.push("    .size(SeekBarSize::Lg)".to_owned());
    }
    if on_media {
      code.push("    .on_media(true)".to_owned());
    }
    if preview {
      code.push("    .preview(|time| frame_of(time))".to_owned());
    }
    if label != "Seek" {
      code.push(format!("    .label({label:?})"));
    }
    if empty {
      code.push("    .empty(true)".to_owned());
    }
    if live {
      code.push("    .live(true)".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    code.push("    .on_change_end(|to| seek(to))".to_owned());
    code.push("    .mount(ctx)".to_owned());

    let played = if duration > 0.0 {
      (position / duration * 100.0).round()
    } else {
      0.0
    };
    let mut tree = vec![
      format!("root        {}", self.playground.measured()),
      format!(
        "├─ track    4 px   {}",
        if chapters {
          "in 4 chapters, 2 apart"
        } else {
          "one stretch"
        }
      ),
      format!("│  ├─ played    {played}%"),
    ];
    if buffered && !disabled && !empty && !live {
      tree.push(format!(
        "│  ├─ buffered  to {}",
        media_time_text((position + AHEAD).min(duration), false)
      ));
    }
    tree.push("│  └─ rest".to_owned());
    tree.push(format!(
      "└─ control  the engine's slider, thumb {}",
      match (size, on_media) {
        (SeekBarSize::Lg, false) => "28",
        (SeekBarSize::Md, true) => "12",
        _ => "16",
      }
    ));

    let defaults = Defaults {
      position: self.position.clone(),
      playing: self.playing.clone(),
      duration: self.duration.clone(),
      step: self.step.clone(),
      size: self.size.clone(),
      label: self.label.clone(),
      flags: [
        self.buffered.clone(),
        self.chapters.clone(),
        self.on_media.clone(),
        self.preview.clone(),
        self.disabled.clone(),
        self.empty.clone(),
        self.live.clone(),
      ],
    };
    let (set_duration, set_step, set_size, clamp) = (
      self.duration.clone(),
      self.step.clone(),
      self.size.clone(),
      self.position.clone(),
    );
    self.playground.window_shows({
      let bar = bar.clone().force(None);
      move |ctx| {
        let mounted = bar.clone().mount(ctx);
        if on_media {
          picture(Row::new().width(FILL).overflow_visible().child(mounted), 96.0).into()
        } else {
          mounted
        }
      }
    });
    let shown = if on_media {
      let playground = &self.playground;
      Shown::built(move |ctx| {
        let measured = playground.measure(Row::new().width(FILL).overflow_visible().child(bar.mount(ctx)));
        picture(measured.width(FILL), 96.0).into()
      })
      .measured()
    } else {
      Shown::built(move |ctx| bar.mount(ctx))
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Seek Bar",
        make: Arc::new(move || plain().build(&lurq::ui::current_theme())),
        shown,
        fill: true,
        forceable: true,
        facts: format!(
          "{} of {}{}",
          media_time_text(position, duration >= 3600.0),
          media_time_text(duration, duration >= 3600.0),
          if playing { ", playing" } else { "" }
        ),
        event: "on_change / on_change_end / on_hover",
        code,
        tree,
        controls: vec![
          flag("playing (the page's clock)", &self.playing),
          divider(),
          prop(
            "duration",
            "impl Into<f64>",
            self.playground.select(0, &DURATIONS, &duration, move |next| {
              set_duration.set(next);
              if f64::from(clamp.get_untracked()) > next {
                clamp.set(next as f32);
              }
            }),
          ),
          prop(
            "step",
            "impl Into<f64>",
            self.playground.select(1, &STEPS, &step, move |next| set_step.set(next)),
          ),
          prop(
            "size",
            "SeekBarSize",
            self.playground.select(2, &SIZES, &size, move |next| set_size.set(next)),
          ),
          prop("label", "&str", field(&self.label)),
          divider(),
          flag("buffered", &self.buffered),
          flag("chapters", &self.chapters),
          flag("on_media", &self.on_media),
          flag("preview", &self.preview),
          flag("empty", &self.empty),
          flag("live", &self.live),
          flag("disabled", &self.disabled),
        ],
        changed: [
          duration != 250.0,
          step != 5.0,
          size != SeekBarSize::Md,
          label != "Seek",
          !buffered,
          chapters,
          on_media,
          preview,
          disabled,
          empty,
          live,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || defaults.reset()),
      },
    )
  }
}

/// What the playground starts with.
struct Defaults {
  position: Signal<f32>,
  playing: Signal<bool>,
  duration: Signal<f64>,
  step: Signal<f64>,
  size: Signal<SeekBarSize>,
  label: Signal<String>,
  flags: [Signal<bool>; 7],
}

impl Defaults {
  fn reset(&self) {
    self.playing.set(false);
    self.position.set(84.0);
    self.duration.set(250.0);
    self.step.set(5.0);
    self.size.set(SeekBarSize::Md);
    self.label.set("Seek".to_owned());
    for (index, flag) in self.flags.iter().enumerate() {
      // Buffered is on to start with.
      flag.set(index == 0);
    }
  }
}

impl Component for SeekBarPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let (position, playing, duration) = (ctx.signal(84.0_f32), ctx.signal(false), ctx.signal(250.0_f64));
    let (song, song_playing) = (ctx.signal(42.0_f32), ctx.signal(false));
    // The page's clock: a tenth of a second each tick, for each bar that plays; one that comes to its end stops.
    let clock = ctx.create_interval(TICK, {
      let (position, playing, duration) = (position.clone(), playing.clone(), duration.clone());
      let (song, song_playing) = (song.clone(), song_playing.clone());
      move || {
        let tick = TICK.as_secs_f32();
        for (at, on, whole) in [
          (&position, &playing, duration.get_untracked() as f32),
          (&song, &song_playing, 250.0),
        ] {
          if on.get_untracked() {
            let next = (at.get_untracked() + tick).min(whole);
            at.set(next);
            if next >= whole {
              on.set(false);
            }
          }
        }
      }
    });
    clock.stop();
    // It ticks only while something plays: nothing the page starts is left running.
    for on in [&playing, &song_playing] {
      let (clock, either) = (clock.clone(), [playing.clone(), song_playing.clone()]);
      ctx.watch(on, move |_| {
        if either.iter().any(Signal::get_untracked) {
          clock.start();
        } else {
          clock.stop();
        }
      });
    }
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 3),
      position,
      playing,
      duration,
      step: ctx.signal(5.0),
      size: ctx.signal(SeekBarSize::Md),
      label: ctx.signal("Seek".to_owned()),
      buffered: ctx.signal(true),
      chapters: ctx.signal(false),
      on_media: ctx.signal(false),
      preview: ctx.signal(false),
      disabled: ctx.signal(false),
      empty: ctx.signal(false),
      live: ctx.signal(false),
      song,
      song_playing,
      _clock: clock,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let song = f64::from(self.song.get());
    let playing = self.song_playing.get();
    let toggle = self.song_playing.clone();
    let live = Row::new()
      .width(FILL)
      .spacing(12.0)
      .align_items(Alignment::Center)
      .overflow_visible()
      .child(
        Button::new(if playing { "Pause" } else { "Play" })
          .icon(if playing { icons::PAUSE } else { icons::PLAY })
          .icon_only(true)
          .variant(ButtonVariant::Ghost)
          .size(ButtonSize::Sm)
          .on_click(move |_| toggle.set(!toggle.get_untracked())),
      )
      .child(
        Row::new().flex(1.0).min_width(0.0).overflow_visible().child(
          SeekBar::new(self.song.clone(), 250.0)
            .buffered((song + AHEAD).min(250.0))
            .mount(ctx),
        ),
      )
      .child(MediaTime::new(song, Some(250.0)));
    let still = |position: f32| SeekBar::new(Signal::new(position), 250.0).buffered(f64::from(position) + 64.0);
    let states = Column::new()
      .width(FILL)
      .spacing(28.0)
      .overflow_visible()
      .child(item(still(100.0).mount(ctx), "seek bar · played, buffered, rest"))
      // Mounted: the chip is a mounted bar's. It stands 32 over the bar, in the room the column leaves.
      .child(item(
        Column::new()
          .width(FILL)
          .padding_top(24.0)
          .overflow_visible()
          .child(still(100.0).force(Some(Interaction::Hovered)).mount(ctx)),
        "hover · the time under the pointer",
      ))
      .child(item(
        still(100.0).force(Some(Interaction::FocusVisible)).mount(ctx),
        "focus-visible · the ring on the thumb",
      ))
      .child(item(still(100.0).disabled(true).mount(ctx), "disabled · nothing loaded is drawn"))
      .child(item(still(100.0).empty(true).mount(ctx), "empty · nothing loaded yet, no thumb, no input"))
      .child(item(still(100.0).live(true).mount(ctx), "live · full, the thumb at the end, no input"));
    let over = Column::new()
      .width(FILL)
      .spacing(28.0)
      .overflow_visible()
      .child(item(
        picture(still(100.0).on_media(true).mount(ctx), 96.0),
        "over a picture · white, a thumb of 12",
      ))
      .child(item(
        picture(
          SeekBar::new(Signal::new(100.0), 250.0)
            .buffered(135.0)
            .chapters(&[70.0, 150.0, 212.0])
            .on_media(true)
            .mount(ctx),
          96.0,
        ),
        "chapters · 2 apart",
      ))
      .child(item(
        Column::new()
          .width(FILL)
          .spacing(12.0)
          .child(still(100.0).mount(ctx))
          .child(
            SeekBar::new(Signal::new(100.0), 250.0)
              .chapters(&[70.0, 150.0, 212.0])
              .buffered(135.0)
              .mount(ctx),
          ),
        "on a surface, plain and with chapters",
      ));
    let previewed = item(
      picture(
        SeekBar::new(Signal::new(100.0), 250.0)
          .buffered(160.0)
          .on_media(true)
          .preview(|time| frame_at(time, 250.0))
          .mount(ctx),
        200.0,
      ),
      "hover · a frame of the moment and its time",
    );
    let keyboard = KEYS
      .into_iter()
      .fold(Column::new().width(FILL).spacing(10.0), |list, (key, does)| {
        let chord = key
          .split("  ")
          .fold(Row::new().width(140.0).spacing(6.0), |chord, key| {
            chord.child(Kbd::shortcut(key))
          });
        list.child(
          Row::new()
            .width(FILL)
            .align_items(Alignment::Center)
            .child(chord)
            .child(quiet(does, 13.0)),
        )
      });
    let rows = [
      (
        "position",
        "Signal<f32>",
        "",
        "SeekBar::new(position, duration): where it is, in seconds, bound to the app's signal, which a drag, a press \
         and the keys set.",
      ),
      (
        "controlled",
        "(position, duration, on_change)",
        "",
        "SeekBar::controlled(position, duration, on_change): the app holds the position and is told where a person \
         moves it; the bar shows what the app answers.",
      ),
      (
        "duration",
        "impl Into<f64>",
        "",
        "How long the whole is, in seconds; 0 where it is not known yet.",
      ),
      (
        "buffered",
        "impl Into<f64>",
        "",
        "How far it is loaded, in seconds: drawn after what is played, in the muted foreground at half its opacity.",
      ),
      (
        "chapters",
        "&[f64]",
        "",
        "Where the chapters start, in seconds: the track is cut there, 2 apart.",
      ),
      (
        "step",
        "impl Into<f64>",
        "5",
        "How many seconds an arrow key moves it. A drag is not snapped to it.",
      ),
      (
        "on_media",
        "bool",
        "false",
        "Over a picture: white in a light and a dark theme, a solid thumb of 12.",
      ),
      (
        "preview",
        "Fn(f64) -> Element",
        "",
        "A picture of the moment under the pointer, given its time: shown in a frame of 128 by 72 over the time. \
         Shown by a mounted bar.",
      ),
      (
        "size",
        "SeekBarSize",
        "Md",
        "Md is a desktop's, 16 high. Lg is a phone's: 28 high with a thumb of 28, 16 over a picture.",
      ),
      (
        "on_change",
        "Fn(f64)",
        "",
        "The position, in seconds, as a person moves it: a drag as it moves, a press, a key. Each call adds.",
      ),
      (
        "on_change_end",
        "Fn(f64)",
        "",
        "Where it ends: the drag let go, the press or the key's step made. Each call adds.",
      ),
      (
        "on_hover",
        "Fn(Option<f64>)",
        "",
        "The time under the pointer as it moves over a mounted bar, and None when it leaves. Each call adds.",
      ),
      (
        "label",
        "&str",
        "\"Seek\"",
        "What it is called, to tooling that reads the tree.",
      ),
      (
        "empty",
        "bool",
        "false",
        "Nothing is loaded: the track alone, no thumb; it takes no input. A player loading, or after an error.",
      ),
      (
        "live",
        "bool",
        "false",
        "A live stream with nothing to go back in: full, the thumb at its end; a press and the keys tell nothing.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Half its opacity, nothing loaded drawn; it takes no input, and Tab passes it by.",
      ),
      (
        "mount",
        "(ctx)",
        "",
        "Puts it where it goes, the only way, keeping the time under the pointer: the chip with the time and \
         the preview.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Media", "Seek Bar"],
        title: "Seek Bar",
        description: "Where a player is in what it plays, how much of it is loaded, and the handle that moves it; \
                      over the pointer, the time it is on.",
        import: Some("use lurq::ui::SeekBar;"),
        status: Some("Preview"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of SeekBar is a control on the right. Nothing plays: the page's own clock moves the position \
           while \"playing\" is on, and a drag, a press or the keys move it as they would a player's.",
          playground,
        ),
        Section::new(
          "Playing",
          "A bar bound to the position the page's clock advances, with the time beside it. Move the pointer over \
           the bar for the time under it.",
          self.frame.demo(
            0,
            live,
            "SeekBar::new(position.clone(), 250.0)",
            &[
              "// position: Signal<f32>, in seconds",
              "Row::new()",
              "    .child(PlayButton::new(playing).on_click(toggle))",
              "    .child(",
              "        SeekBar::new(position.clone(), 250.0)",
              "            .buffered(loaded)",
              "            .on_change_end(|to| seek(to))",
              "            .mount(ctx),",
              "    )",
              "    .child(MediaTime::new(position.get(), Some(250.0)))",
            ],
          ),
        ),
        Section::new(
          "States",
          "Played in the primary color, loaded in the muted foreground at half its opacity, the rest in the secondary \
           color, and the Slider's thumb. Under the pointer the time is in a chip 4 above the bar; the ring of the \
           focus is the thumb's.",
          self.frame.demo(
            1,
            states,
            ".force(Some(Interaction::Hovered))",
            &[
              "SeekBar::new(position.clone(), 250.0).buffered(164.0)",
              "",
              "// Drawn as under the pointer or focused, with nothing pointing at it: for a gallery, a test.",
              "// The chip is a mounted bar's.",
              "SeekBar::new(position.clone(), 250.0).force(Some(Interaction::Hovered)).mount(ctx)",
              "SeekBar::new(position.clone(), 250.0).force(Some(Interaction::FocusVisible))",
              "",
              "SeekBar::new(position.clone(), 250.0).disabled(true)",
              "",
              "// Loading, or after an error: nothing loaded.",
              "SeekBar::new(position.clone(), 250.0).empty(true)",
              "// A live stream with nothing to go back in.",
              "SeekBar::new(position.clone(), 250.0).live(true)",
            ],
          ),
        ),
        Section::new(
          "Over a picture, and chapters",
          "Over a video the bar is white whatever the theme, its thumb solid and 12. Chapters cut the track 2 apart \
           where each starts.",
          self.frame.demo(
            2,
            over,
            ".on_media(true)",
            &[
              "SeekBar::new(position.clone(), 250.0).buffered(164.0).on_media(true)",
              "",
              "SeekBar::new(position.clone(), 250.0)",
              "    .buffered(135.0)",
              "    .chapters(&[70.0, 150.0, 212.0])",
              "    .on_media(true)",
              "",
              "SeekBar::new(position.clone(), 250.0)",
              "    .chapters(&[70.0, 150.0, 212.0])",
              "    .buffered(135.0)",
            ],
          ),
        ),
        Section::new(
          "Preview",
          "Given a picture of the moment under the pointer, a mounted bar shows it in a frame of 128 by 72 over the \
           time. Here a gradient stands for the frame of a video.",
          self.frame.demo(
            3,
            previewed,
            ".preview(|time| frame_of(time))",
            &[
              "SeekBar::new(position.clone(), 250.0)",
              "    .buffered(160.0)",
              "    .on_media(true)",
              "    .preview(|time| frame_of(time))",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Keyboard",
          "What the keys do to the bar Tab is on. A disabled bar is not a stop and takes no key.",
          keyboard,
        ),
        Section::new(
          "Props",
          "What a seek bar is given.",
          props_table(&rows, &["position", "duration"], self.frame.room()),
        ),
      ],
    )
  }
}
