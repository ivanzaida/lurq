//! The page of the checkbox: its playground, the checkbox bound to a signal,
//! its states, and a box over a list that is indeterminate while some of the
//! list are picked.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Checkbox, Interaction},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, line, props_table, quiet},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .child(Row::new().child(shown))
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
  disabled: Signal<bool>,
  sync: Signal<bool>,
  reports: Signal<bool>,
  files: [Signal<bool>; 3],
}

impl CheckboxPage {
  /// The playground: the checkbox as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (label, checked, indeterminate) = (self.label.get(), self.checked.get(), self.indeterminate.get());
    let (invalid, label_hidden, disabled) = (self.invalid.get(), self.label_hidden.get(), self.disabled.get());
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
          .disabled(disabled)
      }
    };
    let checkbox = plain().force(self.playground.forced()).on_change(move |next| {
      set.set(next);
      unmix.set(false);
      record(format!("on_change({next})"));
    });

    let mut code = vec![format!("Checkbox::new({label:?})")];
    for (on, call) in [
      (indeterminate, "    .indeterminate(true)"),
      (invalid, "    .invalid(true)"),
      (label_hidden, "    .label_hidden(true)"),
      (disabled, "    .disabled(true)"),
    ] {
      if on {
        code.push(call.to_owned());
      }
    }
    code.push("    .checked(sync.get())".to_owned());
    code.push("    .on_change(move |checked| sync.set(checked))".to_owned());

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
      self.disabled.clone(),
    ];
    self.playground.card(
      ctx,
      Spec {
        title: "Checkbox",
        make: Arc::new(move || plain().into()),
        shown: checkbox.into(),
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
          flag("disabled", &self.disabled),
        ],
        changed: [checked, indeterminate, invalid, label_hidden, disabled]
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

  /// A box over a list, and the list: the box is checked while all of the
  /// list are, indeterminate while some are, and a press of it picks all or none.
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
      disabled: ctx.signal(false),
      sync: ctx.signal(true),
      reports: ctx.signal(false),
      files: [ctx.signal(true), ctx.signal(false), ctx.signal(false)],
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let bound = |label: &str, checked: &Signal<bool>| {
      let set = checked.clone();
      Checkbox::new(label)
        .checked(checked.get())
        .on_change(move |next| set.set(next))
    };
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
        "on_change",
        "impl Fn(bool)",
        "",
        "Called on a press with what the checkbox is to be: checked if it was not, or was indeterminate.",
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
        "disabled",
        "bool",
        "false",
        "Half the opacity, with its label, and an empty box muted; it takes no input.",
      ),
    ];
    let playground = self.play(ctx);
    let select_all = self.select_all();
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
          "The page holds whether each is checked, in a signal. Tab reaches the box, and Space or Enter presses it.",
          self.frame.demo(
            0,
            live,
            "Checkbox::new(\"Sync settings\").checked(sync.get()).on_change(..)",
            &[
              "// sync, reports: Signal<bool>",
              "let set = sync.clone();",
              "Checkbox::new(\"Sync settings\")",
              "    .checked(sync.get())",
              "    .on_change(move |checked| set.set(checked))",
              "",
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
          "Props",
          "What a checkbox is given.",
          props_table(&rows, &["label"], self.frame.room()),
        ),
      ],
    )
  }
}
