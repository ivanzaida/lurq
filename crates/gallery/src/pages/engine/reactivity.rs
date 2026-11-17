//! The page of the engine's reactive state: a signal that renders what reads it again when it changes, a memo
//! that is derived from signals and tells only of a new value, writes that come together or one by one, and
//! timers. Its playground is a counter kept in a signal; its sections are the demo's Reactivity page.

use std::{sync::Arc, time::Duration};

use lurq::{
  app::{
    component::Component,
    ctx::{Ctx, Interval, Timeout},
    events::MouseEvent,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Row},
  core::{Memo, Ref, Signal},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::Element,
  ui::{Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, icons},
};

use super::kit::{named, well};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, mono, props_table, quiet, text},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

const STEPS: [(&str, i32); 3] = [("1", 1), ("2", 2), ("5", 5)];
/// How long the button of the timers' section says it copied before it is back.
const FEEDBACK: Duration = Duration::from_secs(1);
/// How often the interval of the timers' section ticks.
const TICK: Duration = Duration::from_secs(1);
/// How long the thread of the batch section waits between its writes: long enough for the window to render each.
#[cfg(not(target_arch = "wasm32"))]
const APART: Duration = Duration::from_millis(120);

/// A small button of the page's own: what changes a value a section shows.
fn small(label: &str) -> Button {
  Button::new(label).variant(ButtonVariant::Outline).size(ButtonSize::Sm)
}

/// A value in a box, large: what a counter counts.
fn shown_value(value: i32) -> Row {
  Row::new()
    .width(88.0)
    .height(48.0)
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .rounded(RadiusSize::Md)
    .background(PaletteColor::Muted)
    .border(super::kit::well_line())
    .child(text(&value.to_string(), 24.0, FontWeight::Bold).nowrap())
}

/// What a component that reads a value is given: the value, and nothing else, so that an equal value is no
/// reason to render it again.
#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct ParityProps {
  even: bool,
}

/// A component that shows whether a number is even, and counts its own renders: the parent mounts it at each of
/// its renders, and its render is skipped while its props are the same.
struct Parity {
  renders: Ref<u32>,
}

impl Component for Parity {
  type Props = ParityProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      renders: ctx.create_ref(0),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    self.renders.update(|count| *count += 1);
    let even = ctx.props::<ParityProps>().even;
    named(
      "Parity",
      Column::new()
        .width(FILL)
        .spacing(4.0)
        .align_items(Alignment::Start)
        .child(Badge::new(if even { "even" } else { "odd" }).variant(BadgeVariant::Secondary))
        .child(quiet(&format!("rendered {} times", self.renders.get()), 12.0).width(FILL)),
    )
  }
}

/// What the component of the batch section is given: the three values it adds up.
#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct SumProps {
  a: i32,
  b: i32,
  c: i32,
}

/// A component that reads three values and counts its renders: how many times three writes rendered it.
struct Sum {
  renders: Ref<u32>,
}

impl Component for Sum {
  type Props = SumProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      renders: ctx.create_ref(0),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    self.renders.update(|count| *count += 1);
    let SumProps { a, b, c } = ctx.props::<SumProps>().clone();
    Column::new()
      .width(FILL)
      .spacing(4.0)
      .child(mono(&format!("a: {a}    b: {b}    c: {c}    a + b + c: {}", a + b + c), 14.0).width(FILL))
      .child(quiet(
        &format!("what reads them rendered {} times", self.renders.get()),
        12.0,
      ))
  }
}

pub struct ReactivityPage {
  frame: Frame,
  playground: Playground,
  count: Signal<i32>,
  step: Signal<i32>,
  in_place: Signal<bool>,
  /// The number of the memo section, and what is derived from it.
  number: Signal<i32>,
  doubled: Memo<i32>,
  even: Memo<bool>,
  label: Memo<String>,
  /// How many times the memo `even` was computed, and how many times it told of a new value.
  computed: Ref<u32>,
  published: Signal<i32>,
  /// The three values of the batch section.
  a: Signal<i32>,
  b: Signal<i32>,
  c: Signal<i32>,
  /// The timers' section: whether the button says it copied, and the timeout that puts it back.
  copied: Signal<bool>,
  feedback: Timeout,
  ticks: Signal<i32>,
  ticking: Signal<bool>,
  clock: Interval,
}

impl ReactivityPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (count, step, in_place) = (self.count.get(), self.step.get(), self.in_place.get());
    let signal = self.count.clone();
    let build = move || -> Element {
      let change = |by: i32| {
        let signal = signal.clone();
        move |_: MouseEvent| {
          if in_place {
            signal.update(|count| *count += by);
          } else {
            signal.set(signal.get() + by);
          }
        }
      };
      let reset = signal.clone();
      Column::new()
        .spacing(16.0)
        .align_items(Alignment::Center)
        .child(
          Row::new()
            .spacing(16.0)
            .align_items(Alignment::Center)
            .child(
              Button::new("Less")
                .variant(ButtonVariant::Outline)
                .icon(icons::MINUS)
                .icon_only(true)
                .on_click(change(-step)),
            )
            .child(shown_value(signal.get()))
            .child(
              Button::new("More")
                .variant(ButtonVariant::Outline)
                .icon(icons::PLUS)
                .icon_only(true)
                .on_click(change(step)),
            ),
        )
        .child(
          Row::new()
            .spacing(8.0)
            .child(
              Button::new("Reset")
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Sm)
                .on_click(move |_| reset.set(0)),
            )
            .child(
              Button::new("+10")
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Sm)
                .on_click(change(10)),
            ),
        )
        .into()
    };

    let write = |by: &str| {
      if in_place {
        format!("count.update(|count| *count += {by})")
      } else {
        format!("count.set(count.get() + {by})")
      }
    };
    let code = vec![
      "// In create:".to_owned(),
      "let count = ctx.signal(0);".to_owned(),
      String::new(),
      "// In render: reading it renders this again when it changes.".to_owned(),
      "Row::new()".to_owned(),
      format!(
        "    .child(Button::new(\"Less\").on_click(move |_| {}))",
        write(&format!("-{step}"))
      ),
      "    .child(Text::new(&count.get().to_string()))".to_owned(),
      format!(
        "    .child(Button::new(\"More\").on_click(move |_| {}))",
        write(&step.to_string())
      ),
    ];
    let tree = vec![
      format!("column   {}", self.playground.measured()),
      "├─ row".to_owned(),
      "│  ├─ button   Less".to_owned(),
      format!("│  ├─ value    {count}"),
      "│  └─ button   More".to_owned(),
      "└─ row   Reset, +10".to_owned(),
    ];

    let set_step = self.step.clone();
    let reset = {
      let (count, step, in_place) = (self.count.clone(), self.step.clone(), self.in_place.clone());
      Arc::new(move || {
        count.set(0);
        step.set(1);
        in_place.set(false);
      })
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Signal",
        make: Arc::new(build.clone()),
        shown: Shown::built(move |_| build()),
        fill: false,
        forceable: false,
        facts: format!("count {count}"),
        event: "count.set",
        code,
        tree,
        controls: vec![
          prop(
            "step",
            "i32",
            self
              .playground
              .select(0, &STEPS, &step, move |value| set_step.set(value)),
          ),
          divider(),
          flag("update in place", &self.in_place),
        ],
        changed: [count != 0, step != 1, in_place]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset,
      },
    )
  }

  /// A number, what three memos derive from it, and a component that reads one of them.
  fn memos(&self, ctx: &mut Ctx) -> Column {
    let number = self.number.get();
    let (doubled, even, label) = (self.doubled.get(), self.even.get(), self.label.get());
    let by = |step: i32| {
      let number = self.number.clone();
      move |_: MouseEvent| number.set(number.get() + step)
    };
    let reset = self.number.clone();
    Column::new()
      .width(FILL)
      .spacing(12.0)
      .child(
        Row::new()
          .width(FILL)
          .spacing(8.0)
          .wrap()
          .child(small("+1").on_click(by(1)))
          .child(small("+2").on_click(by(2)))
          .child(small("Reset").on_click(move |_| reset.set(7))),
      )
      .child(
        well()
          .padding(16.0)
          .spacing(8.0)
          .child(named("count", mono(&number.to_string(), 14.0)))
          .child(named("doubled", mono(&doubled.to_string(), 14.0)))
          .child(named("is_even", mono(&even.to_string(), 14.0)))
          .child(named("label", mono(&format!("{label:?}"), 14.0)))
          .child(ctx.mount::<Parity>(ParityProps { even })),
      )
      .child(quiet(
        &format!(
          "is_even was computed {} times and told of a new value {} times",
          self.computed.get(),
          self.published.get()
        ),
        12.0,
      ))
  }

  /// Three values and what reads them: written by a handler, or by a thread a moment apart.
  fn batches(&self, ctx: &mut Ctx) -> Column {
    let (a, b, c) = (self.a.get(), self.b.get(), self.c.get());
    let all = {
      let (a, b, c) = (self.a.clone(), self.b.clone(), self.c.clone());
      move |_: MouseEvent| {
        a.set(a.get() + 1);
        b.set(b.get() + 1);
        c.set(c.get() + 1);
      }
    };
    let mut buttons = Row::new()
      .width(FILL)
      .spacing(8.0)
      .wrap()
      .child(small("Update all in a handler").on_click(all));
    #[cfg(not(target_arch = "wasm32"))]
    {
      let (a, b, c) = (self.a.clone(), self.b.clone(), self.c.clone());
      buttons = buttons.child(small("Update all from a thread").on_click(move |_| {
        let (a, b, c) = (a.clone(), b.clone(), c.clone());
        std::thread::spawn(move || {
          a.set(a.get() + 1);
          std::thread::sleep(APART);
          b.set(b.get() + 1);
          std::thread::sleep(APART);
          c.set(c.get() + 1);
        });
      }));
    }
    let mut column = Column::new()
      .width(FILL)
      .spacing(12.0)
      .child(buttons)
      .child(well().padding(16.0).child(ctx.mount::<Sum>(SumProps { a, b, c })));
    if cfg!(target_arch = "wasm32") {
      column = column.child(quiet("A browser's page has no thread of its own to write from.", 12.0));
    }
    column
  }

  /// A button that says it copied for a moment, and a clock that ticks while it is started.
  fn timers(&self) -> Column {
    let copied = self.copied.get();
    let (set, feedback) = (self.copied.clone(), self.feedback.clone());
    let copy = Button::new(if copied { "Copied" } else { "Copy" })
      .variant(if copied {
        ButtonVariant::Secondary
      } else {
        ButtonVariant::Primary
      })
      .icon(if copied { icons::CHECK } else { icons::COPY })
      .on_click(move |_| {
        lurq::clipboard::copy_to_clipboard("ctx.create_timeout(Duration::from_secs(1), reset)");
        set.set(true);
        feedback.restart();
      });
    let (ticks, ticking) = (self.ticks.get(), self.ticking.get());
    let (clock, running) = (self.clock.clone(), self.ticking.clone());
    let toggle = small(if ticking { "Stop" } else { "Start" }).on_click(move |_| {
      if running.get() {
        clock.stop();
        running.set(false);
      } else {
        clock.start();
        running.set(true);
      }
    });
    let zero = self.ticks.clone();
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(
        Row::new()
          .width(FILL)
          .spacing(12.0)
          .wrap()
          .align_items(Alignment::Center)
          .child(copy)
          .child(text(
            if copied { "Resetting" } else { "Idle" },
            14.0,
            FontWeight::SemiBold,
          )),
      )
      .child(
        Row::new()
          .width(FILL)
          .spacing(12.0)
          .wrap()
          .align_items(Alignment::Center)
          .child(toggle)
          .child(small("Zero").on_click(move |_| zero.set(0)))
          .child(mono(
            &format!("ticks: {ticks}   {}", if ticking { "ticking" } else { "stopped" }),
            14.0,
          )),
      )
  }
}

impl Component for ReactivityPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let playground = Playground::new(ctx, 1);
    let count = ctx.signal(0);
    playground.watch(ctx, &count, |count| format!("count = {count}"));

    let number = ctx.signal(7);
    let doubled = ctx.memo({
      let number = number.clone();
      move || number.get() * 2
    });
    let computed = ctx.create_ref(0_u32);
    let even = ctx.memo({
      let (number, computed) = (number.clone(), computed.clone());
      move || {
        computed.update(|count| *count += 1);
        number.get() % 2 == 0
      }
    });
    let label = ctx.memo({
      let number = number.clone();
      move || format!("count: {}", number.get())
    });
    // Runs once now, and again each time the memo tells of a new value: an equal one is not told.
    let published = ctx.signal(0);
    ctx.on_effect({
      let (even, published) = (even.clone(), published.clone());
      move || {
        let _ = even.get();
        published.set(published.get_untracked() + 1);
      }
    });

    let copied = ctx.signal(false);
    let feedback = ctx.create_timeout(FEEDBACK, {
      let copied = copied.clone();
      move || copied.set(false)
    });
    let ticks = ctx.signal(0);
    // Started by its button only: a page that ticks by itself would never be still.
    let clock = ctx.create_interval(TICK, {
      let ticks = ticks.clone();
      move || ticks.update(|ticks| *ticks += 1)
    });

    Self {
      frame: Frame::new(ctx),
      playground,
      count,
      step: ctx.signal(1),
      in_place: ctx.signal(false),
      number,
      doubled,
      even,
      label,
      computed,
      published,
      a: ctx.signal(1),
      b: ctx.signal(2),
      c: ctx.signal(3),
      copied,
      feedback,
      ticks,
      ticking: ctx.signal(false),
      clock,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "ctx.signal",
        "T -> Signal<T>",
        "",
        "A value the component keeps. Made in create; a change of it renders the component again.",
      ),
      (
        "get / set / update",
        "Signal<T>",
        "",
        "Reads the value, and is told of its changes where it is read in a render, a memo or an effect. set \
         puts a value; update changes it in place.",
      ),
      (
        "ctx.memo",
        "Fn() -> T",
        "",
        "A value derived from the signals the closure reads, computed again when one changes, and told of only \
         when the result is not equal to the last.",
      ),
      (
        "ctx.on_effect",
        "Fn()",
        "",
        "Runs at once, and again each time a value it read changes.",
      ),
      (
        "ctx.batch",
        "FnOnce() -> R",
        "",
        "Holds back the marks of the writes made in the closure until it ends: one render for all of them.",
      ),
      (
        "ctx.create_ref",
        "T -> Ref<T>",
        "",
        "A value the component keeps whose changes render nothing: a count of renders, a handle.",
      ),
      (
        "ctx.create_timeout",
        "Duration, Fn()",
        "",
        "A timer that fires once, after start or restart. restart moves the time it fires to a duration from now.",
      ),
      (
        "ctx.create_interval",
        "Duration, Fn()",
        "",
        "A timer that fires every duration between start and stop.",
      ),
    ];
    let playground = self.play(ctx);
    let memos = self.memos(ctx);
    let batches = self.batches(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Reactivity"],
        title: "Reactivity",
        description: "Signals hold a component's values: what reads one in a render is rendered again when it \
                      changes. Memos derive values from them, and timers change them later.",
        import: Some("use lurq::core::{Memo, Signal};"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "A counter kept in a signal. Each change of it is told in the Events tab, and the counter is rendered \
           again with the new value. The step is what the buttons add.",
          playground,
        ),
        Section::new(
          "Memo, a derived value",
          "Three memos derive from one signal. Press +2: count and doubled change, is_even is computed again and \
           stays what it was, so it tells nothing, and the component that reads it is not rendered again.",
          self.frame.demo(
            0,
            memos,
            "ctx.memo(move || number.get() % 2 == 0)",
            &[
              "// In create:",
              "let number = ctx.signal(7);",
              "let doubled = ctx.memo({ let number = number.clone(); move || number.get() * 2 });",
              "let is_even = ctx.memo({ let number = number.clone(); move || number.get() % 2 == 0 });",
              "",
              "// In render: an equal prop renders the child no more.",
              "ctx.mount::<Parity>(ParityProps { even: is_even.get() })",
            ],
          ),
        ),
        Section::new(
          "Batch updates",
          "A handler that writes three values renders what reads them once: the writes mark the component, and \
           the window renders it after the handler. A thread that writes them a moment apart wakes the window \
           each time, and it renders three times. ctx.batch holds the marks back until all are written.",
          self.frame.demo(
            1,
            batches,
            "ctx.batch(|| { a.set(..); b.set(..); c.set(..) })",
            &[
              "// In a handler: one render after it.",
              "a.set(a.get() + 1);",
              "b.set(b.get() + 1);",
              "c.set(c.get() + 1);",
              "",
              "// In create or render: the marks wait for the end of the batch.",
              "ctx.batch(|| {",
              "    a.set(1);",
              "    b.set(2);",
              "    c.set(3);",
              "});",
            ],
          ),
        ),
        Section::new(
          "Timers",
          "Copy says Copied for a second, and a timeout puts it back; pressed again, it starts the second over. \
           The interval ticks once a second between Start and Stop.",
          self.frame.demo(
            2,
            self.timers(),
            "ctx.create_timeout(Duration::from_secs(1), ..)",
            &[
              "// In create:",
              "let copied = ctx.signal(false);",
              "let feedback = ctx.create_timeout(Duration::from_secs(1), {",
              "    let copied = copied.clone();",
              "    move || copied.set(false)",
              "});",
              "let clock = ctx.create_interval(Duration::from_secs(1), move || ticks.update(|n| *n += 1));",
              "",
              "// In a handler:",
              "copied.set(true);",
              "feedback.restart();",
              "clock.start();",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a component's context makes, and what a signal and a timer do.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
