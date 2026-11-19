//! The page of the radio: its playground, a group bound to a signal, the
//! states of one radio, a choice that cannot be made, and a choice the app
//! holds that must be made.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Choice, Interaction, Orientation, Radio, RadioGroup},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// The choices of the playground, as the board draws them.
const DENSITIES: [&str; 3] = ["Comfortable", "Compact", "Spacious"];

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(9.0)
    .child(Row::new().max_width(FILL).child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new().width(FILL).spacing(28.0).wrap().align_items(Alignment::End)
}

pub struct RadioPage {
  frame: Frame,
  playground: Playground,
  selected: Signal<usize>,
  orientation: Signal<Orientation>,
  invalid: Signal<bool>,
  disabled: Signal<bool>,
  last_disabled: Signal<bool>,
  density: Signal<usize>,
  period: Signal<usize>,
  plan: Signal<&'static str>,
  /// The answer of a question that has none until it is given.
  answer: Signal<Option<bool>>,
}

impl RadioPage {
  /// The playground: the group as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let selected = self.selected.get().min(DENSITIES.len() - 1);
    let (invalid, disabled, last_disabled) = (self.invalid.get(), self.disabled.get(), self.last_disabled.get());
    let orientation = self.orientation.get();
    let horizontal = orientation == Orientation::Horizontal;
    let choices = move || {
      DENSITIES
        .iter()
        .enumerate()
        .map(move |(index, label)| Choice::new(index, label).disabled(last_disabled && index + 1 == DENSITIES.len()))
    };
    let plain = move |on_change: Box<dyn Fn(&usize) + Send + Sync>| {
      RadioGroup::controlled(Some(selected), choices(), on_change)
        .orientation(orientation)
        .invalid(invalid)
        .disabled(disabled)
    };
    let (record, set) = (self.playground.recorder(), self.selected.clone());
    let group = plain(Box::new(move |index| {
      set.set(*index);
      record(format!("on_change: {}", DENSITIES[*index]));
    }));

    let mut code = vec!["RadioGroup::new(density, [".to_owned()];
    for (index, label) in DENSITIES.iter().enumerate() {
      code.push(if last_disabled && index + 1 == DENSITIES.len() {
        format!("    Choice::new(Density::{label}, {label:?}).disabled(true),")
      } else {
        format!("    (Density::{label}, {label:?}),")
      });
    }
    code.push("])".to_owned());
    if horizontal {
      code.push("    .orientation(Orientation::Horizontal)".to_owned());
    }
    if invalid {
      code.push("    .invalid(true)".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }

    let mut tree = vec![format!(
      "root        {}   {}",
      self.playground.measured(),
      if horizontal {
        "side by side, 24 apart"
      } else {
        "12 apart"
      }
    )];
    for (index, label) in DENSITIES.iter().enumerate() {
      let last = index + 1 == DENSITIES.len();
      let (branch, under) = if last { ("└─", " ") } else { ("├─", "│") };
      tree.push(format!(
        "{branch} radio     {label:?}{}",
        if index == selected { "   selected" } else { "" }
      ));
      tree.push(format!(
        "{under}  ├─ mark   16 × 16 px{}",
        if index == selected { ", the dot in it" } else { "" }
      ));
      tree.push(format!("{under}  └─ label  8 beside it"));
    }

    let (to_selected, to_orientation, flags) = (
      self.selected.clone(),
      self.orientation.clone(),
      [self.invalid.clone(), self.disabled.clone(), self.last_disabled.clone()],
    );
    self.playground.card(
      ctx,
      Spec {
        title: "Radio",
        make: Arc::new(move || plain(Box::new(|_| {})).into()),
        shown: Shown::of(self.playground.styled(group)),
        fill: false,
        forceable: false,
        facts: format!("selected: {}", DENSITIES[selected]),
        event: "on_change",
        code,
        tree,
        controls: vec![
          prop(
            "orientation",
            "Orientation",
            segmented_fill(
              &[
                ("Vertical", Orientation::Vertical),
                ("Horizontal", Orientation::Horizontal),
              ],
              &orientation,
              {
                let set = self.orientation.clone();
                move |orientation| set.set(orientation)
              },
            ),
          ),
          flag("invalid", &self.invalid),
          flag("disabled", &self.disabled),
          divider(),
          flag("Spacious disabled", &self.last_disabled),
        ],
        changed: [selected != 0, horizontal, invalid, disabled, last_disabled]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to_selected.set(0);
          to_orientation.set(Orientation::Vertical);
          for flag in &flags {
            flag.set(false);
          }
        }),
      },
    )
  }
}

impl Component for RadioPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      selected: ctx.signal(0),
      orientation: ctx.signal(Orientation::Vertical),
      invalid: ctx.signal(false),
      disabled: ctx.signal(false),
      last_disabled: ctx.signal(false),
      density: ctx.signal(1),
      period: ctx.signal(1),
      plan: ctx.signal("team"),
      answer: ctx.signal(None),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let bound = items().child(item(
      RadioGroup::new(
        self.density.clone(),
        [(0usize, "Comfortable"), (1, "Compact"), (2, "Spacious")],
      ),
      &format!("the signal has: {}", self.density.get()),
    ));
    let states = items()
      .child(item(Radio::new("Not selected"), "not selected"))
      .child(item(Radio::new("Selected").selected(true), "selected"))
      .child(item(
        Radio::new("Focused").force(Some(Interaction::FocusVisible)),
        "focus-visible",
      ))
      .child(item(Radio::new("Invalid").invalid(true), "invalid"))
      .child(item(Radio::new("Disabled").disabled(true), "disabled"))
      .child(item(
        Radio::new("Disabled, selected").selected(true).disabled(true),
        "disabled, selected",
      ));
    let plans = items().child(item(
      RadioGroup::new(
        self.plan.clone(),
        [
          Choice::new("free", "Free"),
          Choice::new("team", "Team"),
          Choice::new("enterprise", "Enterprise").disabled(true),
        ],
      ),
      &format!("the signal has: {:?}", self.plan.get()),
    ));
    let row = items().child(item(
      RadioGroup::new(self.period.clone(), [(0usize, "Day"), (1, "Week"), (2, "Month")])
        .orientation(Orientation::Horizontal),
      &format!("the signal has: {}", self.period.get()),
    ));
    let answer = self.answer.get();
    let set = self.answer.clone();
    let question = items().child(item(
      RadioGroup::controlled(answer, [(true, "Yes"), (false, "No")], move |yes| set.set(Some(*yes)))
        .invalid(answer.is_none()),
      match answer {
        None => "nothing selected yet: invalid",
        Some(true) => "the app holds: Some(true)",
        Some(false) => "the app holds: Some(false)",
      },
    ));
    let group_rows = [
      (
        "selected",
        "Signal<T> or Signal<Option<T>>",
        "",
        "RadioGroup::new: the radio whose value the signal has is selected, none while an Option is None, and a \
         press of another sets it.",
      ),
      (
        "choices",
        "impl Into<Choice<T>>",
        "",
        "One radio each, one under the other: (value, label) or a Choice, which can be disabled.",
      ),
      (
        "controlled",
        "Option<T>, impl Fn(&T)",
        "",
        "In place of a signal: the app holds the choice, none while it is None, and is told each other value \
         pressed.",
      ),
      (
        "on_change",
        "impl Fn(&T)",
        "",
        "Called with the value of a radio pressed that was not the one selected, after the signal is set. A press \
         of the one selected calls nothing.",
      ),
      (
        "orientation",
        "Orientation",
        "Vertical",
        "Horizontal: the radios side by side, 24 apart, going on to the next line where there is no room.",
      ),
      (
        "label",
        "&str",
        "",
        "What the group is called to tooling. It draws nothing: a field gives it its label.",
      ),
      (
        "invalid",
        "bool",
        "false",
        "The edge of every radio in the destructive color: the choice was made wrongly, or not at all.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Every radio dimmed with its label; the group takes no input.",
      ),
    ];
    let radio_rows = [
      (
        "label",
        "&str",
        "",
        "The text beside the mark, and what the radio is called. A press of it is a press of the radio.",
      ),
      (
        "selected",
        "bool",
        "false",
        "Whether it is the one selected: the dot in the mark. The radio does not keep it: the app says so.",
      ),
      (
        "on_change",
        "impl Fn(bool)",
        "",
        "Called with true on a press while the radio is not selected. A press of one that is selected does \
         nothing.",
      ),
      (
        "label_hidden",
        "bool",
        "false",
        "The mark alone. The label is still what the radio is called.",
      ),
      ("invalid", "bool", "false", "The edge in the destructive color."),
      (
        "disabled",
        "bool",
        "false",
        "Half the opacity, with its label; it takes no input.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Radio"],
        title: "Radio",
        description: "One of a few choices that exclude each other, all of them in sight: the density of a list, the \
                      plan of an account.",
        import: Some("use lurq::ui::{Choice, Orientation, Radio, RadioGroup};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of RadioGroup is a control on the right. The group on the stage is live: a press of a radio \
           or of its label selects it, and a press of the one selected does nothing.",
          playground,
        ),
        Section::new(
          "Radio group",
          "The page holds the choice in a signal. Tab stops in the group once, at the selected radio; the arrow \
           keys move the focus to the next radio and select it, and Home and End go to the first and the last.",
          self.frame.demo(
            0,
            bound,
            "RadioGroup::new(density, [(0, \"Comfortable\"), ..])",
            &[
              "// density: Signal<usize>",
              "RadioGroup::new(density, [",
              "    (0, \"Comfortable\"),",
              "    (1, \"Compact\"),",
              "    (2, \"Spacious\"),",
              "])",
            ],
          ),
        ),
        Section::new(
          "States",
          "One radio, alone. Its edge is an input's until it is selected, then the primary color with a dot in it. \
           The ring of the focus is the mark's, and a disabled radio is dimmed with its label.",
          self.frame.demo(
            1,
            states,
            ".selected(true)",
            &[
              "Radio::new(\"Not selected\")",
              "Radio::new(\"Selected\").selected(true)",
              "",
              "// Drawn focused from the keyboard with nothing focused: for a gallery, a test.",
              "Radio::new(\"Focused\").force(Some(Interaction::FocusVisible))",
              "",
              "Radio::new(\"Invalid\").invalid(true)",
              "Radio::new(\"Disabled\").disabled(true)",
              "Radio::new(\"Disabled, selected\").selected(true).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "In a row",
          "A few short choices side by side. The arrow keys move the choice as they do in a column: left and up go \
           back, right and down go on.",
          self.frame.demo(
            4,
            row,
            ".orientation(Orientation::Horizontal)",
            &[
              "// period: Signal<usize>",
              "RadioGroup::new(period, [(0, \"Day\"), (1, \"Week\"), (2, \"Month\")])",
              "    .orientation(Orientation::Horizontal)",
            ],
          ),
        ),
        Section::new(
          "A choice that cannot be made",
          "A choice disabled on its own is drawn, dimmed, and takes nothing; the others are selected as before.",
          self.frame.demo(
            2,
            plans,
            "Choice::new(\"enterprise\", \"Enterprise\").disabled(true)",
            &[
              "// plan: Signal<&'static str>",
              "RadioGroup::new(plan, [",
              "    Choice::new(\"free\", \"Free\"),",
              "    Choice::new(\"team\", \"Team\"),",
              "    Choice::new(\"enterprise\", \"Enterprise\").disabled(true),",
              "])",
            ],
          ),
        ),
        Section::new(
          "A choice the app holds",
          "RadioGroup::controlled takes what is selected, none at first, and is told each press of another. Until \
           a choice is made the group says it is invalid. A Signal<Option<T>> given to RadioGroup::new does the \
           same with no handler.",
          self.frame.demo(
            3,
            question,
            "RadioGroup::controlled(answer, [(true, \"Yes\"), (false, \"No\")], ..)",
            &[
              "// answer: Signal<Option<bool>>",
              "let set = answer.clone();",
              "RadioGroup::controlled(answer.get(), [(true, \"Yes\"), (false, \"No\")], move |yes| {",
              "    set.set(Some(*yes))",
              "})",
              ".invalid(answer.get().is_none())",
            ],
          ),
        ),
        Section::new(
          "RadioGroup props",
          "What a group is given.",
          props_table(&group_rows, &["selected", "choices"], self.frame.room()),
        ),
        Section::new(
          "Radio props",
          "What one radio is given, where it is not in a group.",
          props_table(&radio_rows, &["label"], self.frame.room()),
        ),
      ],
    )
  }
}
