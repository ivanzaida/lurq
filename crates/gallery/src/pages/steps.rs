//! The page of the steps: their playground, a wizard that goes back and on,
//! the states of a step, and steps in a column with descriptions.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify},
  node::Element,
  ui::{Button, ButtonVariant, Step, Steps, StepsOrientation},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Spec, divider, flag, prop},
};

/// The steps of the playground and of the wizard.
const WIZARD: [(&str, &str); 4] = [
  ("Account", "Name and email"),
  ("Billing", "Card and address"),
  ("Workspace", "Name and members"),
  ("Finish", "Review and confirm"),
];

/// The current steps the playground offers: each of the four, and past the last.
const CURRENTS: [(&str, usize); 5] = [
  ("0 · Account", 0),
  ("1 · Billing", 1),
  ("2 · Workspace", 2),
  ("3 · Finish", 3),
  ("4 · all done", 4),
];

/// The ways the playground's steps can run.
const ORIENTATIONS: [(&str, StepsOrientation); 2] = [
  ("Horizontal", StepsOrientation::Horizontal),
  ("Vertical", StepsOrientation::Vertical),
];

/// The wizard's steps, with their descriptions or without, and the second in error or not.
fn wizard(described: bool, error: bool) -> Vec<Step> {
  WIZARD
    .iter()
    .enumerate()
    .map(|(index, (title, description))| {
      let step = Step::new(title).error(error && index == 1);
      if described { step.description(description) } else { step }
    })
    .collect()
}

/// Something on a stage with what it is under it, as wide as the stage.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .width(FILL)
    .spacing(9.0)
    .child(Row::new().width(FILL).child(shown))
    .child(quiet(caption, 12.0))
}

pub struct StepsPage {
  frame: Frame,
  playground: Playground,
  current: Signal<usize>,
  orientation: Signal<StepsOrientation>,
  described: Signal<bool>,
  error: Signal<bool>,
  pressable: Signal<bool>,
  ahead: Signal<bool>,
  disabled: Signal<bool>,
  at: Signal<usize>,
}

impl StepsPage {
  /// The playground: the steps as the panel's props make them.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (current, orientation) = (self.current.get(), self.orientation.get());
    let (described, error) = (self.described.get(), self.error.get());
    let (pressable, ahead, disabled) = (self.pressable.get(), self.ahead.get(), self.disabled.get());
    let vertical = orientation == StepsOrientation::Vertical;
    let plain = {
      let bound = self.current.clone();
      move || {
        Steps::new(&bound, wizard(described, error))
          .orientation(orientation)
          .pressable(pressable)
          .ahead(ahead)
          .disabled(disabled)
          .label("Sign up")
      }
    };
    let record = self.playground.recorder();
    let steps = plain()
      .force(self.playground.forced())
      .on_select(move |index| record(format!("on_select({index})")));
    // A handler makes the steps that are done pressable: the playground gives one only where they are.
    let shown: Element = if pressable { steps.into() } else { plain().into() };

    let mut code = vec!["// at: Signal<usize>".to_owned(), "Steps::new(&at, [".to_owned()];
    for (index, (title, description)) in WIZARD.iter().enumerate() {
      let mut step = format!("    Step::new({title:?})");
      if described {
        step.push_str(&format!(".description({description:?})"));
      }
      if error && index == 1 {
        step.push_str(".error(true)");
      }
      code.push(format!("{step},"));
    }
    code.push("])".to_owned());
    if vertical {
      code.push("    .orientation(StepsOrientation::Vertical)".to_owned());
    }
    if pressable {
      code.push("    .on_select(|index| println!(\"back to {index}\"))".to_owned());
    }
    if ahead {
      code.push("    .ahead(true)".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }

    let state = |index: usize| match index.cmp(&current) {
      std::cmp::Ordering::Less => "done",
      std::cmp::Ordering::Equal => "current",
      std::cmp::Ordering::Greater => "to come",
    };
    let mut tree = vec![format!("root        {}   list", self.playground.measured())];
    for (index, (title, _)) in WIZARD.iter().enumerate() {
      let last = index + 1 == WIZARD.len();
      let branch = if last { "└─" } else { "├─" };
      let error = if error && index == 1 { ", error" } else { "" };
      tree.push(format!("{branch} step {index}  {title:?}   {}{error}", state(index)));
      if !last {
        tree.push(format!(
          "├─ line {index}  {}",
          if index < current { "primary" } else { "border" }
        ));
      }
    }

    let (set_current, set_orientation) = (self.current.clone(), self.orientation.clone());
    let to_current = self.current.clone();
    let to_orientation = self.orientation.clone();
    let reset_flags = [
      self.described.clone(),
      self.error.clone(),
      self.pressable.clone(),
      self.ahead.clone(),
      self.disabled.clone(),
    ];
    let shown_current = CURRENTS
      .iter()
      .find(|(_, value)| *value == current.min(WIZARD.len()))
      .map(|(name, _)| *name)
      .unwrap_or("");
    self.playground.card(
      ctx,
      Spec {
        title: "Steps",
        make: Arc::new(move || plain().into()),
        shown,
        fill: !vertical,
        forceable: pressable,
        facts: format!("current {shown_current}"),
        event: "on_select",
        code,
        tree,
        controls: vec![
          prop(
            "current",
            "impl Into<CurrentStep>",
            self
              .playground
              .select(0, &CURRENTS, &current.min(WIZARD.len()), move |index| {
                set_current.set(index)
              }),
          ),
          prop(
            "orientation",
            "StepsOrientation",
            self
              .playground
              .select(1, &ORIENTATIONS, &orientation, move |orientation| {
                set_orientation.set(orientation)
              }),
          ),
          divider(),
          flag("description", &self.described),
          flag("error", &self.error),
          flag("on_select", &self.pressable),
          flag("ahead", &self.ahead),
          flag("disabled", &self.disabled),
        ],
        changed: [current != 1, vertical, described, error, pressable, ahead, disabled]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to_current.set(1);
          to_orientation.set(StepsOrientation::Horizontal);
          for flag in &reset_flags {
            flag.set(false);
          }
        }),
      },
    )
  }

  /// A wizard: its steps, and Back and Next under them. A step that is done is pressed to go back to it.
  fn wizard(&self) -> Column {
    let at = self.at.get();
    let last = WIZARD.len() - 1;
    let (back, next) = (self.at.clone(), self.at.clone());
    Column::new()
      .width(FILL)
      .spacing(24.0)
      .child(
        Steps::new(&self.at, wizard(false, false))
          .label("Sign up")
          .pressable(true),
      )
      .child(
        Row::new()
          .width(FILL)
          .spacing(8.0)
          .align_items(Alignment::Center)
          .justify(Justify::End)
          .child(
            Button::new("Back")
              .variant(ButtonVariant::Outline)
              .disabled(at == 0)
              .on_click(move |_| back.update(|at| *at = at.saturating_sub(1))),
          )
          .child(
            Button::new(if at >= last { "Start over" } else { "Next" })
              .variant(ButtonVariant::Primary)
              .on_click(move |_| next.update(|at| *at = if *at >= last { 0 } else { *at + 1 })),
          ),
      )
  }
}

impl Component for StepsPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      current: ctx.signal(1),
      orientation: ctx.signal(StepsOrientation::Horizontal),
      described: ctx.signal(false),
      error: ctx.signal(false),
      pressable: ctx.signal(false),
      ahead: ctx.signal(false),
      disabled: ctx.signal(false),
      at: ctx.signal(1),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let three = ["Account", "Workspace", "Finish"];
    let states = Column::new()
      .width(FILL)
      .spacing(28.0)
      .child(item(Steps::new(0, three), "at the first step: the others to come"))
      .child(item(
        Steps::new(1, three),
        "at the second: the first done, the line to the current one in the primary color",
      ))
      .child(item(Steps::new(3, three), "past the last: every step done"))
      .child(item(
        Steps::new(
          1,
          [
            Step::new("Account"),
            Step::new("Workspace").error(true),
            Step::new("Finish"),
          ],
        ),
        "a step in error: its mark and its title in the destructive color",
      ));
    let column = Row::new().child(
      Steps::new(2, wizard(true, false))
        .orientation(StepsOrientation::Vertical)
        .label("Sign up"),
    );
    let rows = [
      (
        "current",
        "impl Into<CurrentStep>",
        "",
        "The index of the current step, from 0, or a Signal<usize> that a press sets. Past the last, every step is \
         done.",
      ),
      (
        "steps",
        "impl IntoIterator<Item = impl Into<Step>>",
        "",
        "The steps in order: a title each, or a Step with a description, an icon in place of its number, an error.",
      ),
      (
        "orientation",
        "StepsOrientation",
        "Horizontal",
        "In a row, a line between each step and the next; in a column, a line down from each mark.",
      ),
      (
        "on_select",
        "impl Fn(usize)",
        "",
        "Called with the index of a step pressed. Makes the steps that are done pressable.",
      ),
      (
        "pressable",
        "bool",
        "false",
        "Steps that are done are pressed, with no handler: for steps bound to a signal.",
      ),
      (
        "ahead",
        "bool",
        "false",
        "Where steps are pressed, a step to come is pressed too.",
      ),
      ("disabled", "bool", "false", "Half the opacity; no step is pressed."),
      (
        "label",
        "&str",
        "",
        "What the list of steps is called to tooling that reads the tree.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Navigation", "Steps"],
        title: "Steps",
        description: "The stages of a process in order, with one current: those before it are done, those after it \
                      are to come.",
        import: Some("use lurq::ui::{Step, Steps};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Steps is a control on the right. With on_select on, a press of a step that is done goes \
           back to it, and Tab and the arrow keys move between the steps that are pressed.",
          playground,
        ),
        Section::new(
          "A wizard",
          "The page holds the current step in a signal: Next and Back move it, and a press of a step that is done \
           sets it. Tab enters the steps at the current one.",
          self.frame.demo(
            0,
            self.wizard(),
            "Steps::new(&at, [..]).pressable(true)",
            &[
              "// at: Signal<usize>",
              "Steps::new(&at, [\"Account\", \"Billing\", \"Workspace\", \"Finish\"])",
              "    .label(\"Sign up\")",
              "    .pressable(true)",
              "",
              "let back = at.clone();",
              "Button::new(\"Back\")",
              "    .variant(ButtonVariant::Outline)",
              "    .disabled(at.get() == 0)",
              "    .on_click(move |_| back.update(|at| *at = at.saturating_sub(1)))",
              "let next = at.clone();",
              "Button::new(\"Next\")",
              "    .variant(ButtonVariant::Primary)",
              "    .on_click(move |_| next.update(|at| *at += 1))",
            ],
          ),
        ),
        Section::new(
          "States",
          "A step is done, current or to come by where it stands from the current one. A step done has a check in \
           its mark; a step in error has a cross, whatever else it is.",
          self.frame.demo(
            1,
            states,
            "Steps::new(1, [..])",
            &[
              "let steps = [\"Account\", \"Workspace\", \"Finish\"];",
              "Steps::new(0, steps)",
              "Steps::new(1, steps)",
              "Steps::new(3, steps)",
              "",
              "Steps::new(1, [",
              "    Step::new(\"Account\"),",
              "    Step::new(\"Workspace\").error(true),",
              "    Step::new(\"Finish\"),",
              "])",
            ],
          ),
        ),
        Section::new(
          "In a column",
          "The side of a wizard: each step's mark with a line down to the next, and its title with a description \
           beside them.",
          self.frame.demo(
            2,
            column,
            ".orientation(StepsOrientation::Vertical)",
            &[
              "Steps::new(2, [",
              "    Step::new(\"Account\").description(\"Name and email\"),",
              "    Step::new(\"Billing\").description(\"Card and address\"),",
              "    Step::new(\"Workspace\").description(\"Name and members\"),",
              "    Step::new(\"Finish\").description(\"Review and confirm\"),",
              "])",
              ".orientation(StepsOrientation::Vertical)",
              ".label(\"Sign up\")",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What steps are given.",
          props_table(&rows, &["current", "steps"], self.frame.room()),
        ),
      ],
    )
  }
}
