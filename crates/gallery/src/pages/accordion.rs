//! The page of the accordion: its playground, one item open at a time and
//! any number, a disabled item, and the collapsible alone.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::RadiusSize},
  components::{Column, Row},
  core::Signal,
  node::Element,
  ui::{Accordion, AccordionItem, AccordionPart, Collapsible, CollapsiblePart, Interaction},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{line, mono, props_table},
  playground::{Playground, Spec, divider, flag},
};

/// The questions of the board, as items of `T`.
const QUESTIONS: [(&str, &str); 3] = [
  (
    "Is lurq GPU-accelerated?",
    "Yes. Rendering goes through wgpu, with a DirectX 12 backend on Windows.",
  ),
  (
    "Does it support dark mode?",
    "Every theme has a light and a dark appearance, and follows the system's unless the app says otherwise.",
  ),
  (
    "Can I bring my own theme?",
    "Yes: a theme is a value the app builds, with its own palette, type, radii and recipes.",
  ),
];

/// The questions as items, the second disabled where `disabled` says so.
fn questions(disabled: bool) -> Vec<AccordionItem<usize>> {
  QUESTIONS
    .iter()
    .enumerate()
    .map(|(index, (question, answer))| AccordionItem::text(index, question, answer).disabled(disabled && index == 1))
    .collect()
}

/// The crates of the workspace, each in a box of its own, as the board's collapsible holds them.
fn crates() -> Column {
  ["lurq", "lurq_macros", "demo"]
    .into_iter()
    .fold(Column::new().spacing(8.0), |column, name| {
      column.child(
        Row::new()
          .padding_horizontal(12.0)
          .padding_vertical(8.0)
          .rounded(RadiusSize::Md)
          .border(line())
          .child(mono(name, 13.0)),
      )
    })
}

/// The width of the board's accordion.
const WIDE: f32 = 360.0;

pub struct AccordionPage {
  frame: Frame,
  playground: Playground,
  multiple: Signal<bool>,
  disabled: Signal<bool>,
  item_disabled: Signal<bool>,
  played_one: Signal<Option<usize>>,
  played_many: Signal<Vec<usize>>,
  one: Signal<Option<usize>>,
  many: Signal<Vec<usize>>,
  crates_open: Signal<bool>,
}

impl AccordionPage {
  /// The playground: the accordion as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (multiple, disabled, item_disabled) = (self.multiple.get(), self.disabled.get(), self.item_disabled.get());
    let open: Vec<usize> = if multiple {
      self.played_many.get()
    } else {
      self.played_one.get().into_iter().collect()
    };
    // The app holds what is open, so that each press is counted: `new` and `multiple` are this with the
    // signal set for it.
    let record = self.playground.recorder();
    let (one, many) = (self.played_one.clone(), self.played_many.clone());
    let shown = Accordion::controlled(open.clone(), questions(item_disabled), move |value| {
      if multiple {
        many.update(|open| match open.iter().position(|held| *held == value) {
          Some(index) => {
            open.remove(index);
          }
          None => open.push(value),
        });
      } else {
        one.update(|open| *open = if *open == Some(value) { None } else { Some(value) });
      }
      record(format!("press({value})"));
    })
    .disabled(disabled)
    .style(AccordionPart::Root, |s| s.width(WIDE));

    let make = {
      let open = open.clone();
      move || {
        Accordion::controlled(open.clone(), questions(item_disabled), |_| {})
          .disabled(disabled)
          .style(AccordionPart::Root, |s| s.width(WIDE))
          .into()
      }
    };

    let mut code = vec![if multiple {
      "// open: Signal<Vec<usize>>, which `Accordion::multiple(open, items)` sets itself".to_owned()
    } else {
      "// open: Signal<Option<usize>>, which `Accordion::new(open, items)` sets itself".to_owned()
    }];
    code.push("Accordion::controlled(open.get(), [".to_owned());
    for (index, (question, answer)) in QUESTIONS.iter().enumerate() {
      let tail = if item_disabled && index == 1 {
        ".disabled(true)"
      } else {
        ""
      };
      code.push(format!(
        "    AccordionItem::text({index}, {question:?}, {answer:?}){tail},"
      ));
    }
    code.push("], move |value| /* open or close `value` in `open` */)".to_owned());
    if disabled {
      code.push(".disabled(true)".to_owned());
    }

    let mut tree = vec![format!("root        {}", self.playground.measured())];
    for (index, (question, _)) in QUESTIONS.iter().enumerate() {
      let is_open = open.contains(&index);
      let branch = if index + 1 == QUESTIONS.len() {
        "└─"
      } else {
        "├─"
      };
      let rail = if index + 1 == QUESTIONS.len() { "  " } else { "│ " };
      tree.push(format!(
        "{branch} item      line under it   {}",
        if is_open { "open" } else { "closed" }
      ));
      tree.push(format!(
        "{rail}  ├─ trigger  {question:?}  chevron {}",
        if is_open { "up" } else { "down" }
      ));
      tree.push(format!(
        "{rail}  └─ panel    {}",
        if is_open { "as high as its content" } else { "0 px" }
      ));
    }

    let flags = [self.multiple.clone(), self.disabled.clone(), self.item_disabled.clone()];
    let (one, many) = (self.played_one.clone(), self.played_many.clone());
    self.playground.card(
      ctx,
      Spec {
        title: "Accordion",
        make: Arc::new(make),
        shown: shown.into(),
        fill: false,
        forceable: false,
        facts: format!("{} open", open.len()),
        event: "press",
        code,
        tree,
        controls: vec![
          flag("multiple", &self.multiple),
          divider(),
          flag("disabled", &self.disabled),
          flag("second item disabled", &self.item_disabled),
        ],
        changed: [multiple, disabled, item_disabled]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          for flag in &flags {
            flag.set(false);
          }
          one.set(Some(0));
          many.set(vec![0]);
        }),
      },
    )
  }
}

impl Component for AccordionPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      multiple: ctx.signal(false),
      disabled: ctx.signal(false),
      item_disabled: ctx.signal(false),
      played_one: ctx.signal(Some(0)),
      played_many: ctx.signal(vec![0]),
      one: ctx.signal(Some(0)),
      many: ctx.signal(vec![0, 2]),
      crates_open: ctx.signal(true),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let single = Accordion::new(self.one.clone(), questions(false)).style(AccordionPart::Root, |s| s.width(WIDE));
    let several =
      Accordion::multiple(self.many.clone(), questions(false)).style(AccordionPart::Root, |s| s.width(WIDE));
    let disabled = Accordion::new(self.one.clone(), questions(true)).style(AccordionPart::Root, |s| s.width(WIDE));
    let set = self.crates_open.clone();
    let collapsible = Collapsible::new("3 crates in this workspace", crates())
      .open(self.crates_open.get())
      .on_change(move |open| set.set(open))
      .style(CollapsiblePart::Root, |s| s.width(280.0));
    let collapsible_states = Row::new()
      .spacing(28.0)
      .wrap()
      .child(Collapsible::text("Closed", "What it holds.").style(CollapsiblePart::Root, |s| s.width(200.0)))
      .child(
        Collapsible::text("Open", "What it holds.")
          .open(true)
          .style(CollapsiblePart::Root, |s| s.width(200.0)),
      )
      .child(
        Collapsible::text("Focused", "What it holds.")
          .force(Some(Interaction::FocusVisible))
          .style(CollapsiblePart::Root, |s| s.width(200.0)),
      )
      .child(
        Collapsible::text("Disabled", "What it holds.")
          .disabled(true)
          .style(CollapsiblePart::Root, |s| s.width(200.0)),
      );

    let rows = [
      (
        "open",
        "Signal<Option<T>> / Signal<Vec<T>>",
        "",
        "The item open, or none (`new`), or the items open (`multiple`). A press of a header sets it.",
      ),
      (
        "items",
        "impl IntoIterator<Item = AccordionItem<T>>",
        "",
        "Each with its value, what its header says and what it shows: an element (`new`) or a text (`text`).",
      ),
      (
        "controlled",
        "(open, items, impl Fn(T))",
        "",
        "The app holds what is open, and is told the value of each header pressed.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "No header takes input. An item has its own `disabled` too.",
      ),
      (
        "style / rules",
        "AccordionPart",
        "",
        "Root, Item, Trigger, Label, Indicator, Panel, Content.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Navigation", "Accordion"],
        title: "Accordion",
        description: "Headers one under the other, each of which opens what is under it: questions and their \
                      answers, the sections of a long form, settings in groups.",
        import: Some("use lurq::ui::{Accordion, AccordionItem, Collapsible};"),
        status: Some("Preview"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Accordion is a control on the right. The accordion on the stage is live: a press of a header \
           opens its item, which grows to the height of what it holds, and closes the one that was open.",
          playground,
        ),
        Section::new(
          "One at a time",
          "The page holds the item that is open in a `Signal<Option<usize>>`. A press of the open one closes it. Tab \
           reaches each header, Up and Down move between them, and Space or Enter opens one.",
          self.frame.demo(
            0,
            single,
            "Accordion::new(open, items)",
            &[
              "// open: Signal<Option<usize>>",
              "Accordion::new(open, [",
              "    AccordionItem::text(0, \"Is lurq GPU-accelerated?\", \"Yes. Rendering goes through wgpu, ..\"),",
              "    AccordionItem::text(1, \"Does it support dark mode?\", \"Every theme has a light and a dark ..\"),",
              "    AccordionItem::text(2, \"Can I bring my own theme?\", \"Yes: a theme is a value the app builds, ..\"),",
              "])",
            ],
          ),
        ),
        Section::new(
          "Any number open",
          "The page holds the items that are open in a `Signal<Vec<usize>>`. A press opens an item, or closes it.",
          self.frame.demo(
            1,
            several,
            "Accordion::multiple(open, items)",
            &[
              "// open: Signal<Vec<usize>>",
              "Accordion::multiple(open, [",
              "    AccordionItem::text(0, \"Is lurq GPU-accelerated?\", \"Yes. Rendering goes through wgpu, ..\"),",
              "    AccordionItem::text(1, \"Does it support dark mode?\", \"Every theme has a light and a dark ..\"),",
              "    AccordionItem::text(2, \"Can I bring my own theme?\", \"Yes: a theme is a value the app builds, ..\"),",
              "])",
            ],
          ),
        ),
        Section::new(
          "A disabled item",
          "Its header is dimmed and takes nothing; Tab and the arrows pass it by. It stays as it is, open or closed.",
          self.frame.demo(
            2,
            disabled,
            ".disabled(true)",
            &[
              "Accordion::new(open, [",
              "    AccordionItem::text(0, \"Is lurq GPU-accelerated?\", \"..\"),",
              "    AccordionItem::text(1, \"Does it support dark mode?\", \"..\").disabled(true),",
              "    AccordionItem::text(2, \"Can I bring my own theme?\", \"..\"),",
              "])",
            ],
          ),
        ),
        Section::new(
          "Collapsible",
          "One header and what it opens, with no others around it. Whether it is open is the app's: here a \
           `Signal<bool>`. The ghost button at the end of the header lights under the pointer; the whole header is \
           pressed.",
          self.frame.demo(
            3,
            collapsible,
            "Collapsible::new(label, content).open(open.get())",
            &[
              "// open: Signal<bool>",
              "let set = open.clone();",
              "Collapsible::new(\"3 crates in this workspace\", crates)",
              "    .open(open.get())",
              "    .on_change(move |open| set.set(open))",
            ],
          ),
        ),
        Section::new(
          "Collapsible states",
          "Closed, open, focused from the keyboard and disabled. A disabled header is dimmed; what it opens is the \
           app's, and is not.",
          self.frame.demo(
            4,
            collapsible_states,
            ".open(true)",
            &[
              "Collapsible::text(\"Closed\", \"What it holds.\")",
              "Collapsible::text(\"Open\", \"What it holds.\").open(true)",
              "Collapsible::text(\"Focused\", \"What it holds.\").force(Some(Interaction::FocusVisible))",
              "Collapsible::text(\"Disabled\", \"What it holds.\").disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What an accordion is given.",
          props_table(&rows, &["open", "items"], self.frame.room()),
        ),
      ],
    )
  }
}
