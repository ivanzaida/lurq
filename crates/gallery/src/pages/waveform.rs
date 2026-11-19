//! The page of the waveform: its playground, played by a clock of the page's own; shown and operated, unheard,
//! and in a voice message.

use std::{sync::Arc, time::Duration};

use lurq::{
  app::{
    component::Component,
    ctx::{Ctx, Interval},
    theme::{BorderSize, PaletteColor, RadiusSize},
  },
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::{Element, border::Border},
  ui::{Badge, BadgeVariant, MediaTime, MediaTimeSize, Waveform},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(10.0)
    .child(Row::new().max_width(FILL).child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new().width(FILL).spacing(28.0).wrap().align_items(Alignment::End)
}

/// The board's 36 bars (`Media/Waveform`), each of 24.
const BOARD: [f32; 36] = [
  6.0, 10.0, 16.0, 12.0, 20.0, 24.0, 14.0, 8.0, 12.0, 18.0, 22.0, 16.0, 10.0, 6.0, 12.0, 20.0, 16.0, 24.0, 18.0, 10.0,
  8.0, 14.0, 20.0, 12.0, 6.0, 10.0, 16.0, 22.0, 14.0, 8.0, 12.0, 18.0, 10.0, 6.0, 8.0, 12.0,
];

/// The heights of the board's bars as peaks, from 0 to 1, `count` of them: the board's again from its start.
fn peaks(count: usize) -> Vec<f32> {
  (0..count).map(|index| BOARD[index % BOARD.len()] / 24.0).collect()
}

/// How the playground's waveform holds how far it is.
#[derive(Clone, Copy, PartialEq, lurq::DevtoolsInspectable)]
enum Form {
  /// `Waveform::new(signal, peaks)`.
  Bound,
  /// `Waveform::controlled(progress, peaks, on_change)`.
  Controlled,
  /// `Waveform::at(progress, peaks)`: shown and not operated.
  At,
}

const FORMS: [(&str, Form); 3] = [
  ("new (bound)", Form::Bound),
  ("controlled", Form::Controlled),
  ("at (shown)", Form::At),
];
const COUNTS: [(&str, usize); 3] = [("36 (the board's)", 36), ("12", 12), ("60", 60)];
const STEPS: [(&str, f64); 3] = [("0.05", 0.05), ("0.1", 0.1), ("0.25", 0.25)];
/// How long what the playground plays is, in seconds, and how often the page's clock ticks.
const LENGTH: f64 = 42.0;
const TICK: Duration = Duration::from_millis(100);

pub struct WaveformPage {
  frame: Frame,
  playground: Playground,
  progress: Signal<f32>,
  playing: Signal<bool>,
  form: Signal<Form>,
  count: Signal<usize>,
  step: Signal<f64>,
  label: Signal<String>,
  unheard: Signal<bool>,
  disabled: Signal<bool>,
  voice: Signal<f32>,
  _clock: Interval,
}

impl WaveformPage {
  /// The playground: the waveform as the panel's props make it, played by the page's clock.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (progress, form, count, step) = (self.progress.get(), self.form.get(), self.count.get(), self.step.get());
    let (label, unheard, disabled) = (self.label.get(), self.unheard.get(), self.disabled.get());
    let heights = peaks(count);
    let plain = {
      let (bound, label, heights) = (self.progress.clone(), label.clone(), heights.clone());
      move || {
        let wave = match form {
          Form::Bound => Waveform::new(bound.clone(), &heights),
          Form::Controlled => {
            let set = bound.clone();
            Waveform::controlled(bound.get_untracked(), &heights, move |to| set.set(to as f32))
          }
          Form::At => Waveform::at(bound.get_untracked(), &heights),
        };
        wave.step(step).label(&label).unheard(unheard).disabled(disabled)
      }
    };
    let (record, record_end) = (self.playground.recorder(), self.playground.recorder());
    let wave = plain()
      .force(self.playground.forced())
      .on_change(move |to| record(format!("on_change({to})")))
      .on_change_end(move |to| record_end(format!("on_change_end({to})")));

    let mut code = vec![
      format!("// peaks: [f32; {count}], each from 0 to 1"),
      match form {
        Form::Bound => "Waveform::new(progress.clone(), &peaks)".to_owned(),
        Form::Controlled => "Waveform::controlled(progress, &peaks, |to| seek(to))".to_owned(),
        Form::At => format!("Waveform::at({progress:.2}, &peaks)"),
      },
    ];
    if form != Form::At && step != 0.05 {
      code.push(format!("    .step({step:?})"));
    }
    if label != "Seek" {
      code.push(format!("    .label({label:?})"));
    }
    if unheard {
      code.push("    .unheard(true)".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    let played = if unheard {
      0
    } else {
      (f64::from(progress) * count as f64).round() as usize
    };
    let mut tree = vec![
      format!("root        {}", self.playground.measured()),
      format!("├─ bars     {count}, 3 wide, 2 apart"),
      format!("│  ├─ played  {played}"),
      format!("│  └─ bar     {}", count - played),
    ];
    if form != Form::At {
      tree.push("└─ control  the engine's slider, over the bars".to_owned());
    }

    let (to_progress, to_playing, to_form, to_count, to_step, to_label, to_unheard, to_disabled) = (
      self.progress.clone(),
      self.playing.clone(),
      self.form.clone(),
      self.count.clone(),
      self.step.clone(),
      self.label.clone(),
      self.unheard.clone(),
      self.disabled.clone(),
    );
    let (set_form, set_count, set_step) = (self.form.clone(), self.count.clone(), self.step.clone());
    let mut controls = vec![
      flag("playing (the page's clock)", &self.playing),
      divider(),
      prop(
        "form",
        "new / controlled / at",
        self.playground.select(0, &FORMS, &form, move |next| set_form.set(next)),
      ),
      prop(
        "peaks",
        "&[f32]",
        self
          .playground
          .select(1, &COUNTS, &count, move |next| set_count.set(next)),
      ),
    ];
    if form != Form::At {
      controls.push(prop(
        "step",
        "impl Into<f64>",
        self.playground.select(2, &STEPS, &step, move |next| set_step.set(next)),
      ));
      controls.push(prop("label", "&str", field(&self.label)));
    }
    controls.push(divider());
    controls.push(flag("unheard", &self.unheard));
    if form != Form::At {
      controls.push(flag("disabled", &self.disabled));
    }
    self.playground.card(
      ctx,
      Spec {
        title: "Waveform",
        make: Arc::new(move || plain().into()),
        shown: Shown::of(self.playground.styled(wave)),
        fill: false,
        forceable: form != Form::At,
        facts: format!("{:.0}% played", f64::from(progress) * 100.0),
        event: if form == Form::At {
          ""
        } else {
          "on_change / on_change_end"
        },
        code,
        tree,
        controls,
        changed: [
          form != Form::Bound,
          count != 36,
          step != 0.05,
          label != "Seek",
          unheard,
          disabled,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_playing.set(false);
          to_progress.set(0.4);
          to_form.set(Form::Bound);
          to_count.set(36);
          to_step.set(0.05);
          to_label.set("Seek".to_owned());
          to_unheard.set(false);
          to_disabled.set(false);
        }),
      },
    )
  }
}

impl Component for WaveformPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let (progress, playing) = (ctx.signal(0.4_f32), ctx.signal(false));
    // The page's clock plays a recording of 42 seconds while "playing" is on, to its end.
    let clock = ctx.create_interval(TICK, {
      let (progress, playing) = (progress.clone(), playing.clone());
      move || {
        let next = (f64::from(progress.get_untracked()) + TICK.as_secs_f64() / LENGTH).min(1.0);
        progress.set(next as f32);
        if next >= 1.0 {
          playing.set(false);
        }
      }
    });
    clock.stop();
    {
      let clock = clock.clone();
      ctx.watch(&playing, move |on| if *on { clock.start() } else { clock.stop() });
    }
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 3),
      progress,
      playing,
      form: ctx.signal(Form::Bound),
      count: ctx.signal(36),
      step: ctx.signal(0.05),
      label: ctx.signal("Seek".to_owned()),
      unheard: ctx.signal(false),
      disabled: ctx.signal(false),
      voice: ctx.signal(0.0),
      _clock: clock,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = peaks(36);
    let forms = items()
      .child(item(
        Waveform::at(14.0 / 36.0, &board),
        "waveform · 3px bars, 2px gaps, 24 tall",
      ))
      .child(item(
        Waveform::at(0.0, &board).unheard(true),
        "unheard · every bar in the input's color",
      ))
      .child(item(Waveform::new(Signal::new(0.4), &board).disabled(true), "disabled"));
    // A voice message, as the audio player draws it: the waveform between the time and the play button's place.
    let voice = self.voice.get();
    let message = Row::new()
      .max_width(FILL)
      .padding_left(12.0)
      .padding_right(12.0)
      .padding_top(8.0)
      .padding_bottom(8.0)
      .spacing(10.0)
      .align_items(Alignment::Center)
      .rounded(RadiusSize::Xl)
      .background(PaletteColor::Card)
      .border(Border::inside(BorderSize::Sm, PaletteColor::Border))
      .child(Waveform::new(self.voice.clone(), &board).label("Voice message"))
      .child(
        MediaTime::new(f64::from(voice) * LENGTH, Some(LENGTH))
          .size(MediaTimeSize::Sm)
          .remaining(true),
      )
      .child(Badge::new("1×").variant(BadgeVariant::Secondary));
    let operated = items().child(item(message, &format!("the signal has: {voice:.2}")));
    let rows = [
      (
        "peaks",
        "&[f32]",
        "",
        "The heights of its bars, each from 0 to 1, one for each bar: about 36 for a voice message.",
      ),
      (
        "new",
        "(Signal<f32>, peaks)",
        "",
        "Bound to how far it is played, from 0 to 1, which a press, a drag and the keys set.",
      ),
      (
        "controlled",
        "(progress, peaks, on_change)",
        "",
        "The app holds how far it is and is told where a person presses or drags to.",
      ),
      (
        "at",
        "(progress, peaks)",
        "",
        "Shown and not operated: no stop of Tab, and a press does nothing.",
      ),
      (
        "on_change",
        "Fn(f64)",
        "",
        "Where a person moves it, from 0 to 1, as a press or a drag moves it. Each call adds.",
      ),
      (
        "on_change_end",
        "Fn(f64)",
        "",
        "Where it ends: the drag let go, the press or a key's step made. Each call adds.",
      ),
      (
        "step",
        "impl Into<f64>",
        "0.05",
        "How far an arrow key moves it, from 0 to 1. Page keys move a tenth, Home and End to the ends.",
      ),
      (
        "unheard",
        "bool",
        "false",
        "Nothing of it was heard yet: every bar in the color of what is not played.",
      ),
      (
        "label",
        "&str",
        "\"Seek\"",
        "What it is called, to tooling that reads the tree.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Half its opacity; it takes no input, and Tab passes it by.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Media", "Waveform"],
        title: "Waveform",
        description: "The loudness of a recording as a row of bars, the played part of it marked: a voice message's \
                      seek bar.",
        import: Some("use lurq::ui::Waveform;"),
        status: Some("Preview"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Waveform is a control on the right. Nothing plays: the page's own clock moves it while \
           \"playing\" is on. Operated, press or drag on it, or Tab to it and use the arrow keys.",
          playground,
        ),
        Section::new(
          "Forms",
          "Bars 3 wide and 2 apart, at most 24 high and fully round; played in the primary color, the rest in the \
           input's color.",
          self.frame.demo(
            0,
            forms,
            "Waveform::at(0.4, &peaks)",
            &[
              "Waveform::at(0.4, &peaks)",
              "Waveform::at(0.0, &peaks).unheard(true)",
              "Waveform::new(progress.clone(), &peaks).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "In a voice message",
          "Bound to the page's signal: a press or a drag on the bars seeks, and the time beside it reads what is \
           left.",
          self.frame.demo(
            1,
            operated,
            "Waveform::new(progress.clone(), &peaks)",
            &[
              "// progress: Signal<f32>, from 0 to 1",
              "Row::new()",
              "    .child(Waveform::new(progress.clone(), &peaks).label(\"Voice message\"))",
              "    .child(MediaTime::new(left, Some(42.0)).size(MediaTimeSize::Sm).remaining(true))",
              "    .child(Badge::new(\"1×\").variant(BadgeVariant::Secondary))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a waveform is given.",
          props_table(&rows, &["peaks"], self.frame.room()),
        ),
      ],
    )
  }
}
