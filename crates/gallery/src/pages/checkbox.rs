//! The page of the checkbox: its playground, the checkbox bound to a signal,
//! its states, a box over a list that is indeterminate while some of the
//! list are picked, a checkbox pressed as a whole in a row pressed of its own,
//! its mark alone in rows pressed as a whole, and one the app focuses by its
//! element ref.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Row},
  core::{ElementRef, Signal},
  layout::Alignment,
  node::Element,
  ui::{Button, ButtonVariant, Checkbox, CheckboxGroup, Choice, Interaction, Orientation},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, line, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

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

/// The files of the list under "Select all".
const FILES: [&str; 3] = ["blur.rs", "canvas.rs", "theme.rs"];

pub struct CheckboxPage {
  frame: Frame,
  playground: Playground,
  label: Signal<String>,
  checked: Signal<bool>,
  indeterminate: Signal<bool>,
  invalid: Signal<bool>,
  label_hidden: Signal<bool>,
  whole: Signal<bool>,
  disabled: Signal<bool>,
  sync: Signal<bool>,
  reports: Signal<bool>,
  files: [Signal<bool>; 3],
  days: Signal<Vec<&'static str>>,
  /// The checkbox pressed as a whole in a row of its own, and how many times the row was pressed.
  agreed: Signal<bool>,
  row_presses: Signal<usize>,
  /// The options of a filter, each a row pressed as a whole with a checkbox's mark alone in it.
  kinds: [Signal<bool>; 2],
  /// What refers to a checkbox the page moves the focus to.
  focused: ElementRef,
  /// The checkboxes whose labels go on to more lines.
  long_labels: [Signal<bool>; 2],
}

impl CheckboxPage {
  /// The playground: the checkbox as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (label, checked, indeterminate) = (self.label.get(), self.checked.get(), self.indeterminate.get());
    let (invalid, label_hidden, disabled) = (self.invalid.get(), self.label_hidden.get(), self.disabled.get());
    let whole = self.whole.get();
    let record = self.playground.recorder();
    let (set, unmix) = (self.checked.clone(), self.indeterminate.clone());
    let plain = {
      let label = label.clone();
      move || {
        Checkbox::new(&label)
          .checked(checked)
          .indeterminate(indeterminate)
          .invalid(invalid)
          .label_hidden(label_hidden)
          .whole(whole)
          .disabled(disabled)
      }
    };
    let checkbox = plain()
      .force(self.playground.forced())
      .bind(set)
      .on_change(move |next| {
        unmix.set(false);
        record(format!("on_change({next})"));
      });

    let mut code = vec![format!("Checkbox::new({label:?})")];
    for (on, call) in [
      (indeterminate, "    .indeterminate(true)"),
      (invalid, "    .invalid(true)"),
      (label_hidden, "    .label_hidden(true)"),
      (whole, "    .whole(true)"),
      (disabled, "    .disabled(true)"),
    ] {
      if on {
        code.push(call.to_owned());
      }
    }
    code.push("    .bind(sync)".to_owned());

    let mark = if indeterminate {
      "dash"
    } else if checked {
      "check"
    } else {
      "nothing"
    };
    let mut tree = vec![
      format!("root          {}", self.playground.measured()),
      format!(
        "{} box         16 × 16 px   {}",
        if label_hidden { "└─" } else { "├─" },
        if indeterminate {
          "indeterminate"
        } else if checked {
          "checked"
        } else {
          "unchecked"
        }
      ),
      format!(
        "{}  └─ indicator  12 × 12 px   {mark}",
        if label_hidden { " " } else { "│" },
      ),
    ];
    if !label_hidden {
      tree.push(format!("└─ label       {label:?}"));
    }
    let facts = [
      (indeterminate, "indeterminate"),
      (!indeterminate && checked, "checked"),
      (!indeterminate && !checked, "unchecked"),
      (invalid, "invalid"),
    ]
    .into_iter()
    .filter_map(|(on, what)| on.then_some(what))
    .collect::<Vec<_>>()
    .join(", ");

    let to_label = self.label.clone();
    let flags = [
      self.checked.clone(),
      self.indeterminate.clone(),
      self.invalid.clone(),
      self.label_hidden.clone(),
      self.whole.clone(),
      self.disabled.clone(),
    ];
    self.playground.card(
      ctx,
      Spec {
        title: "Checkbox",
        make: Arc::new(move || plain().into()),
        shown: Shown::of(self.playground.styled(checkbox)),
        fill: false,
        forceable: true,
        facts,
        event: "on_change",
        code,
        tree,
        controls: vec![
          prop("label", "&str", field(&self.label)),
          divider(),
          flag("checked", &self.checked),
          flag("indeterminate", &self.indeterminate),
          flag("invalid", &self.invalid),
          flag("label_hidden", &self.label_hidden),
          flag("whole", &self.whole),
          flag("disabled", &self.disabled),
        ],
        changed: [checked, indeterminate, invalid, label_hidden, whole, disabled]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to_label.set("Sync settings".to_owned());
          for flag in &flags {
            flag.set(false);
          }
        }),
      },
    )
  }

  /// Checkboxes whose long labels go on to more lines in a narrow column, the box beside the first line: one alone,
  /// one pressed as a whole.
  fn long_labels(&self) -> Column {
    Column::new()
      .width(280.0)
      .max_width(FILL)
      .spacing(12.0)
      .child(
        Checkbox::new("Keep the worktree of every run in ~/projects/orchester/.orchester/worktrees until it is merged")
          .label_wraps(true)
          .bind(self.long_labels[0].clone()),
      )
      .child(
        Checkbox::new("Ask before a run deletes a branch with commits that nobody has merged yet")
          .label_wraps(true)
          .whole(true)
          .bind(self.long_labels[1].clone()),
      )
  }

  /// A box over a list, and the list: the box is checked while all of the
  /// list are, indeterminate while some are, and a press of it picks all or none.
  /// A filter's options, each a row pressed as a whole that says it is checked, with a checkbox's mark alone; and a
  /// checkbox a button focuses by its element ref.
  fn marks(&self, ctx: &mut Ctx) -> Column {
    let option = |label: &str, kind: &Signal<bool>| {
      let (on, set) = (kind.get(), kind.clone());
      Row::new()
        .width(240.0)
        .max_width(FILL)
        .padding_horizontal(8.0)
        .padding_vertical(6.0)
        .spacing(8.0)
        .align_items(Alignment::Center)
        .rounded(RadiusSize::Sm)
        .describe("a11y_role", "checkbox")
        .describe("a11y_name", label.to_owned())
        .describe("a11y_checked", if on { "true" } else { "false" })
        .on_click(move |_| set.update(|on| *on = !*on))
        .child(Checkbox::new(label).checked(on).mark_only(true))
        .child(quiet(label, 14.0))
    };
    let focus = ctx.focus_handle();
    let target = self.focused.clone();
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(
        Column::new()
          .spacing(2.0)
          .child(option("Bug", &self.kinds[0]))
          .child(option("Feature", &self.kinds[1])),
      )
      .child(
        Row::new()
          .spacing(16.0)
          .align_items(Alignment::Center)
          .child(
            Button::new("Focus the checkbox")
              .variant(ButtonVariant::Outline)
              .on_click(move |_| focus.focus(&target)),
          )
          .child(
            Checkbox::new("Send crash reports")
              .element_ref(self.focused.clone())
              .bind(self.reports.clone()),
          ),
      )
  }

  fn select_all(&self) -> Column {
    let picked = self.files.iter().filter(|file| file.get()).count();
    let all = self.files.clone();
    let head = Checkbox::new("Select all")
      .checked(picked == FILES.len())
      .indeterminate(picked > 0 && picked < FILES.len())
      .on_change(move |next| {
        for file in &all {
          file.set(next);
        }
      });
    let mut list = Column::new()
      .width(FILL)
      .clip()
      .rounded(RadiusSize::Md)
      .background(PaletteColor::Background)
      .border(line())
      .child(Row::new().padding_horizontal(16.0).padding_vertical(10.0).child(head));
    for (name, file) in FILES.iter().zip(&self.files) {
      let set = file.clone();
      list = list.child(
        Row::new()
          .width(FILL)
          .padding_horizontal(16.0)
          .padding_vertical(10.0)
          .border_top(line())
          .child(
            Checkbox::new(name)
              .checked(file.get())
              .on_change(move |next| set.set(next)),
          ),
      );
    }
    list
  }
}

impl Component for CheckboxPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      label: ctx.signal("Sync settings".to_owned()),
      checked: ctx.signal(false),
      indeterminate: ctx.signal(false),
      invalid: ctx.signal(false),
      label_hidden: ctx.signal(false),
      whole: ctx.signal(false),
      disabled: ctx.signal(false),
      sync: ctx.signal(true),
      reports: ctx.signal(false),
      files: [ctx.signal(true), ctx.signal(false), ctx.signal(false)],
      days: ctx.signal(vec!["tue"]),
      agreed: ctx.signal(false),
      row_presses: ctx.signal(0),
      kinds: [ctx.signal(true), ctx.signal(false)],
      focused: ElementRef::new(),
      long_labels: [ctx.signal(true), ctx.signal(false)],
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let bound = |label: &str, checked: &Signal<bool>| Checkbox::new(label).bind(checked.clone());
    let says = |checked: &Signal<bool>| {
      if checked.get() {
        "the signal has: true"
      } else {
        "the signal has: false"
      }
    };
    let live = items()
      .child(item(bound("Sync settings", &self.sync), says(&self.sync)))
      .child(item(bound("Send crash reports", &self.reports), says(&self.reports)));
    let states = items()
      .child(item(Checkbox::new("Unchecked"), "unchecked"))
      .child(item(Checkbox::new("Checked").checked(true), "checked"))
      .child(item(
        Checkbox::new("Indeterminate").indeterminate(true),
        "indeterminate",
      ))
      .child(item(
        Checkbox::new("Focused")
          .checked(true)
          .force(Some(Interaction::FocusVisible)),
        "focus-visible",
      ))
      .child(item(Checkbox::new("Invalid").invalid(true), "invalid"))
      .child(item(Checkbox::new("Disabled").disabled(true), "disabled"))
      .child(item(
        Checkbox::new("Managed by policy").checked(true).disabled(true),
        "disabled, checked",
      ));
    let (agreed, presses) = (self.agreed.clone(), self.row_presses.clone());
    let whole = Column::new()
      .width(FILL)
      .spacing(9.0)
      .child(
        Row::new()
          .width(FILL)
          .padding_horizontal(16.0)
          .padding_vertical(12.0)
          .align_items(Alignment::Center)
          .rounded(RadiusSize::Md)
          .background(PaletteColor::Background)
          .border(line())
          .on_click(move |_| presses.update(|count| *count += 1))
          .child(
            Checkbox::new("I have read what this step changes")
              .whole(true)
              .id("read")
              .checked(agreed.get())
              .on_change(move |next| agreed.set(next)),
          ),
      )
      .child(quiet(
        &format!(
          "the row was pressed {} times; a press of the checkbox is the checkbox's alone",
          self.row_presses.get()
        ),
        12.0,
      ));
    let rows = [
      (
        "label",
        "&str",
        "",
        "The text beside the box, and what the checkbox is called. A press of it is a press of the checkbox.",
      ),
      (
        "checked",
        "bool",
        "false",
        "Whether it is checked. The checkbox does not keep it: the app says so.",
      ),
      (
        "indeterminate",
        "bool",
        "false",
        "Some of what it stands for is checked: a dash, whatever `checked` says. A press asks for checked.",
      ),
      (
        "bind",
        "Signal<bool>",
        "",
        "In place of checked and a handler: checked while the signal is true, which a press turns.",
      ),
      (
        "on_change",
        "impl Fn(bool)",
        "",
        "Called on a press with what the checkbox is to be: checked if it was not, or was indeterminate. Bound, \
         after the signal is set; not when the app sets it.",
      ),
      (
        "invalid",
        "bool",
        "false",
        "What it holds is wrong: the box is edged in the destructive color.",
      ),
      (
        "label_hidden",
        "bool",
        "false",
        "The box alone, where the text is somewhere else. The label is still what the checkbox is called.",
      ),
      (
        "whole",
        "bool",
        "false",
        "The box and the label are one control: pressed, focused and ringed as a whole, the id on the whole and \
         the parts named after it (x-box, x-indicator, x-label).",
      ),
      (
        "label_wraps",
        "bool",
        "false",
        "The label goes on to more lines within the width the checkbox is given, the box beside its first line.",
      ),
      (
        "mark_only",
        "bool",
        "false",
        "The box alone, for a row pressed as a whole that says it is checked: no press, no focus, no label, hidden \
         from tooling; its id on the box.",
      ),
      (
        "element_ref",
        "ElementRef",
        "",
        "Refers to what is pressed and focused: the box, or the whole. What the app moves the focus to.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Half the opacity, with its label, and an empty box muted; it takes no input.",
      ),
    ];
    let playground = self.play(ctx);
    let marks = self.marks(ctx);
    let long_labels = self.long_labels();
    let select_all = self.select_all();
    let picked = self.days.get();
    let group = items()
      .child(item(
        CheckboxGroup::new(
          self.days.clone(),
          [
            Choice::new("mon", "Monday"),
            Choice::new("tue", "Tuesday"),
            Choice::new("wed", "Wednesday").disabled(true),
          ],
        ),
        &format!("the signal has: {picked:?}"),
      ))
      .child(item(
        CheckboxGroup::new(self.days.clone(), [("mon", "Mon"), ("tue", "Tue"), ("wed", "Wed")])
          .orientation(Orientation::Horizontal),
        "orientation(Orientation::Horizontal)",
      ));
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Checkbox"],
        title: "Checkbox",
        description: "A choice that is made or not, among others a form takes at once: send crash reports, keep me \
                      signed in, a row of a list picked.",
        import: Some("use lurq::ui::Checkbox;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Checkbox is a control on the right. The checkbox on the stage is live: a press of its box \
           or of its label checks it or clears it.",
          playground,
        ),
        Section::new(
          "Checkbox",
          "The page holds whether each is checked, in a signal the checkbox is bound to. Tab reaches the box, and \
           Space or Enter presses it.",
          self.frame.demo(
            0,
            live,
            "Checkbox::new(\"Sync settings\").bind(sync)",
            &[
              "// sync, reports: Signal<bool>",
              "Checkbox::new(\"Sync settings\").bind(sync)",
              "Checkbox::new(\"Send crash reports\").bind(reports)",
              "",
              "// The same, with the app's own handler:",
              "let set = reports.clone();",
              "Checkbox::new(\"Send crash reports\")",
              "    .checked(reports.get())",
              "    .on_change(move |checked| set.set(checked))",
            ],
          ),
        ),
        Section::new(
          "States",
          "Empty the box is the background edged like an input; checked or indeterminate it is the primary color with \
           a check or a dash. The ring of the focus is the box's, an invalid box is edged in the destructive color, \
           and a disabled checkbox is dimmed with its label.",
          self.frame.demo(
            1,
            states,
            ".indeterminate(true)",
            &[
              "Checkbox::new(\"Unchecked\")",
              "Checkbox::new(\"Checked\").checked(true)",
              "Checkbox::new(\"Indeterminate\").indeterminate(true)",
              "",
              "// Drawn focused from the keyboard with nothing focused: for a gallery, a test.",
              "Checkbox::new(\"Focused\").checked(true).force(Some(Interaction::FocusVisible))",
              "",
              "Checkbox::new(\"Invalid\").invalid(true)",
              "Checkbox::new(\"Disabled\").disabled(true)",
              "Checkbox::new(\"Managed by policy\").checked(true).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Over a list",
          "A box over a list is checked while all of the list are, and indeterminate while some are. A press of it \
           picks them all, or, when all are, none.",
          self.frame.demo(
            2,
            select_all,
            ".indeterminate(picked > 0 && picked < files.len())",
            &[
              "// files: [Signal<bool>; 3]",
              "let picked = files.iter().filter(|file| file.get()).count();",
              "let all = files.clone();",
              "Checkbox::new(\"Select all\")",
              "    .checked(picked == files.len())",
              "    .indeterminate(picked > 0 && picked < files.len())",
              "    .on_change(move |next| {",
              "        for file in &all {",
              "            file.set(next);",
              "        }",
              "    })",
              "",
              "// and under it, for each file:",
              "let set = file.clone();",
              "Checkbox::new(\"blur.rs\")",
              "    .checked(file.get())",
              "    .on_change(move |checked| set.set(checked))",
            ],
          ),
        ),
        Section::new(
          "A group over a list",
          "CheckboxGroup is bound to a signal of a list: the boxes whose values it has are checked, and a press adds \
           a value or takes it out. on_change is told the whole new list. Each box is its own stop of Tab.",
          self.frame.demo(
            3,
            group,
            "CheckboxGroup::new(days, [(\"mon\", \"Monday\"), ..])",
            &[
              "// days: Signal<Vec<&'static str>>",
              "CheckboxGroup::new(days.clone(), [",
              "    Choice::new(\"mon\", \"Monday\"),",
              "    Choice::new(\"tue\", \"Tuesday\"),",
              "    Choice::new(\"wed\", \"Wednesday\").disabled(true),",
              "])",
              "",
              "// Side by side.",
              "CheckboxGroup::new(days, [(\"mon\", \"Mon\"), (\"tue\", \"Tue\"), (\"wed\", \"Wed\")])",
              "    .orientation(Orientation::Horizontal)",
            ],
          ),
        ),
        Section::new(
          "Pressed as a whole, in a row pressed of its own",
          "With whole(true) the box and the label are one control, one ring around both, and the id is on the \
           whole. A press of a checkbox, whole or not, is its own: the row around it, which counts its presses, is \
           not pressed by it.",
          self.frame.demo(
            4,
            whole,
            ".whole(true)",
            &[
              "Row::new()",
              "    .on_click(move |_| open_step())",
              "    .child(",
              "        Checkbox::new(\"I have read what this step changes\")",
              "            .whole(true)",
              "            .id(\"read\") // read-box, read-indicator, read-label",
              "            .bind(agreed),",
              "    )",
            ],
          ),
        ),
        Section::new(
          "Its mark alone, and focused by the app",
          "In a row pressed as a whole (a filter's option) the row is the control and says it is checked; the \
           checkbox is its mark alone, with mark_only(true): the box in the recipe's look, with no press, focus or \
           label of its own. element_ref refers to what a checkbox's focus is on, which the button moves the focus \
           to.",
          self.frame.demo(
            5,
            marks,
            ".mark_only(true)",
            &[
              "Row::new()",
              "    .describe(\"a11y_role\", \"checkbox\")",
              "    .describe(\"a11y_checked\", if bug { \"true\" } else { \"false\" })",
              "    .on_click(move |_| toggle_bug())",
              "    .child(Checkbox::new(\"Bug\").checked(bug).mark_only(true))",
              "    .child(Text::new(\"Bug\"))",
              "",
              "// A checkbox the app focuses: focus = ctx.focus_handle()",
              "let reports_ref = ElementRef::new();",
              "Checkbox::new(\"Send crash reports\").element_ref(reports_ref.clone()).bind(reports)",
              "Button::new(\"Focus the checkbox\").on_click(move |_| focus.focus(&reports_ref))",
            ],
          ),
        ),
        Section::new(
          "A label over several lines",
          "With label_wraps(true) a long label goes on to more lines within the width the checkbox is given, the box \
           beside its first line. A short label leaves the checkbox as narrow as before.",
          self.frame.demo(
            6,
            long_labels,
            ".label_wraps(true)",
            &[
              "Column::new()",
              "    .width(280.0)",
              "    .child(Checkbox::new(\"Keep the worktree of every run in … until it is merged\").label_wraps(true).bind(keep))",
              "    .child(Checkbox::new(\"Ask before a run deletes a branch …\").label_wraps(true).whole(true).bind(ask))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a checkbox is given.",
          props_table(&rows, &["label"], self.frame.room()),
        ),
      ],
    )
  }
}
