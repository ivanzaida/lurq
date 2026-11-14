//! The page of the password input: its playground, a password the page holds with its eye, the meter of its
//! strength, its states and its sizes, and a password input in a field.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Field, InputSize, Interaction, PasswordInput, PasswordStrength},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// The strengths the playground offers, and how the code names them.
const STRENGTHS: [(&str, Option<PasswordStrength>, &str); 5] = [
  ("none", None, "None"),
  ("weak", Some(PasswordStrength::Weak), "Some(PasswordStrength::Weak)"),
  ("fair", Some(PasswordStrength::Fair), "Some(PasswordStrength::Fair)"),
  ("good", Some(PasswordStrength::Good), "Some(PasswordStrength::Good)"),
  (
    "strong",
    Some(PasswordStrength::Strong),
    "Some(PasswordStrength::Strong)",
  ),
];

/// The sizes, as the board names them, and how the code names them.
const SIZES: [(InputSize, &str, &str); 3] = [
  (InputSize::Sm, "sm", "InputSize::Sm"),
  (InputSize::Md, "md", "InputSize::Md"),
  (InputSize::Lg, "lg", "InputSize::Lg"),
];

/// How strong a password is, by a rough count of what is in it: what the page's own password shows.
fn judge(password: &str) -> Option<PasswordStrength> {
  if password.is_empty() {
    return None;
  }
  let kinds = [
    password.chars().any(|c| c.is_ascii_lowercase()),
    password.chars().any(|c| c.is_ascii_uppercase()),
    password.chars().any(|c| c.is_ascii_digit()),
    password.chars().any(|c| !c.is_ascii_alphanumeric()),
  ]
  .into_iter()
  .filter(|kind| *kind)
  .count();
  Some(match (password.chars().count(), kinds) {
    (0..8, _) => PasswordStrength::Weak,
    (_, 0..=2) => PasswordStrength::Fair,
    (8..12, _) => PasswordStrength::Good,
    _ => PasswordStrength::Strong,
  })
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .child(Row::new().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new()
    .width(FILL)
    .spacing(28.0)
    .wrap()
    .align_items(Alignment::Start)
}

/// A password input that holds `value`: for a stage that shows a state.
fn showing(value: &str) -> PasswordInput {
  PasswordInput::new(Signal::new(value.to_owned())).on_reveal(|_| {})
}

pub struct PasswordInputPage {
  frame: Frame,
  playground: Playground,
  value: Signal<String>,
  placeholder: Signal<String>,
  revealed: Signal<bool>,
  strength: Signal<usize>,
  size: Signal<usize>,
  invalid: Signal<bool>,
  disabled: Signal<bool>,
  read_only: Signal<bool>,
  full_width: Signal<bool>,
  password: Signal<String>,
  shown: Signal<bool>,
  new_password: Signal<String>,
}

impl PasswordInputPage {
  /// The playground: the password input as the panel's props make it, bound to the page's signals.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let placeholder = self.placeholder.get();
    let revealed = self.revealed.get();
    let (strength_name, strength, strength_code) = STRENGTHS[self.strength.get().min(STRENGTHS.len() - 1)];
    let (size, size_name, size_code) = SIZES[self.size.get().min(SIZES.len() - 1)];
    let flags = [
      ("invalid", self.invalid.get()),
      ("read_only", self.read_only.get()),
      ("disabled", self.disabled.get()),
      ("full_width", self.full_width.get()),
    ];
    let on = |name: &str| flags.iter().any(|(flag, set)| *flag == name && *set);
    let make = {
      let (placeholder, value) = (placeholder.clone(), self.value.clone());
      let set: Vec<bool> = flags.iter().map(|(_, set)| *set).collect();
      move || {
        PasswordInput::new(value.clone())
          .placeholder(&placeholder)
          .reveal(revealed)
          .on_reveal(|_| {})
          .strength(strength)
          .size(size)
          .invalid(set[0])
          .read_only(set[1])
          .disabled(set[2])
          .full_width(set[3])
      }
    };
    let (record, set_reveal) = (self.playground.recorder(), self.revealed.clone());
    let shown = make().force(self.playground.forced()).on_reveal(move |next| {
      set_reveal.set(next);
      record(format!("on_reveal({next})"));
    });

    let mut code = vec![
      "// value: Signal<String>, revealed: Signal<bool>".to_owned(),
      "PasswordInput::new(value.clone())".to_owned(),
      format!("    .placeholder({placeholder:?})"),
      "    .bind_reveal(revealed.clone())".to_owned(),
    ];
    if strength.is_some() {
      code.push(format!("    .strength({strength_code})"));
    }
    if size != InputSize::Md {
      code.push(format!("    .size({size_code})"));
    }
    for (name, set) in flags {
      if set {
        code.push(format!("    .{name}(true)"));
      }
    }

    let mut tree = vec![
      format!("root        {}", self.playground.measured()),
      format!(
        "├─ control  {}",
        if revealed {
          "shown as it is"
        } else {
          "masked: • for each character"
        }
      ),
      format!(
        "{} toggle   16 × 16 px   eye, \"{}\"",
        if strength.is_some() { "├─" } else { "└─" },
        if revealed { "Hide password" } else { "Show password" }
      ),
    ];
    if let Some(strength) = strength {
      tree.push(format!("├─ strength 4 bars, {} filled", strength.bars()));
      tree.push(format!("└─ message  {:?}", strength.message()));
    }

    let strengths: Vec<(&str, usize)> = STRENGTHS
      .iter()
      .enumerate()
      .map(|(index, item)| (item.0, index))
      .collect();
    let sizes: Vec<(&str, usize)> = SIZES.iter().enumerate().map(|(index, size)| (size.1, index)).collect();
    let (to_strength, to_size) = (self.strength.clone(), self.size.clone());
    let reset = {
      let (value, placeholder, revealed, strength, size) = (
        self.value.clone(),
        self.placeholder.clone(),
        self.revealed.clone(),
        self.strength.clone(),
        self.size.clone(),
      );
      let flags = [
        self.invalid.clone(),
        self.read_only.clone(),
        self.disabled.clone(),
        self.full_width.clone(),
      ];
      move || {
        value.set(String::new());
        placeholder.set("Password".to_owned());
        revealed.set(false);
        strength.set(0);
        size.set(1);
        for flag in &flags {
          flag.set(false);
        }
      }
    };
    let changed = flags.iter().filter(|(_, set)| *set).count()
      + usize::from(revealed)
      + usize::from(strength.is_some())
      + usize::from(size != InputSize::Md)
      + usize::from(!self.value.get().is_empty())
      + usize::from(placeholder != "Password");
    self.playground.card(
      ctx,
      Spec {
        title: "Password Input",
        make: Arc::new(move || make().into()),
        shown: shown.into(),
        fill: on("full_width"),
        forceable: true,
        facts: format!("{size_name} · {strength_name}"),
        event: "on_reveal",
        code,
        tree,
        controls: vec![
          prop("value", "Signal<String>", field(&self.value)),
          prop("placeholder", "&str", field(&self.placeholder)),
          prop(
            "strength",
            "Option<PasswordStrength>",
            self
              .playground
              .select(0, &strengths, &self.strength.get(), move |index| to_strength.set(index)),
          ),
          prop(
            "size",
            "InputSize",
            segmented_fill(&sizes, &self.size.get(), move |index| to_size.set(index)),
          ),
          divider(),
          flag("reveal", &self.revealed),
          flag("invalid", &self.invalid),
          flag("read_only", &self.read_only),
          flag("disabled", &self.disabled),
          flag("full_width", &self.full_width),
        ],
        changed,
        reset: Arc::new(reset),
      },
    )
  }
}

impl Component for PasswordInputPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      value: ctx.signal(String::new()),
      placeholder: ctx.signal("Password".to_owned()),
      revealed: ctx.signal(false),
      strength: ctx.signal(0),
      size: ctx.signal(1),
      invalid: ctx.signal(false),
      disabled: ctx.signal(false),
      read_only: ctx.signal(false),
      full_width: ctx.signal(false),
      password: ctx.signal(String::new()),
      shown: ctx.signal(false),
      new_password: ctx.signal("correct horse".to_owned()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let live = items().child(item(
      PasswordInput::new(self.password.clone())
        .placeholder("Password")
        .bind_reveal(self.shown.clone()),
      &format!(
        "the signals have: {:?}, shown: {}",
        self.password.get(),
        self.shown.get()
      ),
    ));
    let chosen = self.new_password.get();
    let meter = items().child(item(
      PasswordInput::new(self.new_password.clone())
        .placeholder("New password")
        .new_password(true)
        .bind_reveal(self.shown.clone())
        .strength(judge(&chosen)),
      "judged by the page as it is typed",
    ));
    let mut strengths = items();
    for strength in [
      PasswordStrength::Weak,
      PasswordStrength::Fair,
      PasswordStrength::Good,
      PasswordStrength::Strong,
    ] {
      strengths = strengths.child(item(
        showing("hunter2").strength(Some(strength)),
        &format!("{} · {} of 4", strength.message(), strength.bars()),
      ));
    }
    let states = items()
      .child(item(showing("").placeholder("Password"), "default"))
      .child(item(showing("hunter2"), "masked"))
      .child(item(showing("hunter2").reveal(true), "shown"))
      .child(item(showing("hunter2").force(Some(Interaction::FocusVisible)), "focus"))
      .child(item(showing("hunter2").invalid(true), "invalid"))
      .child(item(showing("hunter2").disabled(true), "disabled"));
    let mut sizes = items();
    for (size, name, _) in SIZES {
      sizes = sizes.child(item(showing("hunter2").size(size), name));
    }
    let in_field = items().child(
      Field::new(
        "Password",
        PasswordInput::new(self.password.clone())
          .bind_reveal(self.shown.clone())
          .full_width(true),
      )
      .description("At least 8 characters.")
      .required(true),
    );

    let rows = [
      (
        "value",
        "Signal<String>",
        "",
        "What the input holds; what is typed is set on it.",
      ),
      ("placeholder", "&str", "", "What it says while it is empty."),
      (
        "reveal",
        "bool",
        "false",
        "Whether the password is shown as it is, in place of its dots.",
      ),
      (
        "bind_reveal",
        "Signal<bool>",
        "",
        "Shown while the signal is true, which the eye turns.",
      ),
      (
        "on_reveal",
        "impl Fn(bool)",
        "",
        "Called on a press of the eye with whether to show it. The eye is drawn where something hears it.",
      ),
      (
        "reveal_labels",
        "&str, &str",
        "\"Show password\", \"Hide password\"",
        "What the eye is called to tooling, in another language.",
      ),
      (
        "strength",
        "Option<PasswordStrength>",
        "None",
        "The meter under the input: 1 to 4 bars, destructive, warning or success.",
      ),
      (
        "strength_message",
        "&str",
        "",
        "What the meter says, in place of \"Strong password\" and the others.",
      ),
      (
        "new_password",
        "bool",
        "false",
        "The password is being chosen: the platform may offer a strong one.",
      ),
      ("size", "InputSize", "Md", "Sm 32, Md 36, Lg 40 high, as an input."),
      (
        "invalid / warning / valid",
        "bool",
        "false",
        "The color of the edge, as an input's.",
      ),
      (
        "read_only",
        "bool",
        "false",
        "It can be focused, not changed. A masked one is not copied either.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Half the opacity; neither the input nor the eye takes input.",
      ),
      (
        "full_width",
        "bool",
        "false",
        "As wide as what holds it, in place of 260, with its meter.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Password Input"],
        title: "Password Input",
        description: "An input whose text is masked, with an eye at its end that shows it and hides it again, and \
                      under it, where the app says so, a meter of how strong the password is.",
        import: Some("use lurq::ui::{PasswordInput, PasswordStrength};"),
        status: Some("New"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of PasswordInput is a control on the right. The input on the stage is live: what is typed in \
           it goes into the value, and the eye turns reveal.",
          playground,
        ),
        Section::new(
          "Password Input",
          "The page holds the password and whether it is shown, in two signals. Tab reaches the input and then the \
           eye, which Space or Enter presses. A masked password is not copied, and tooling reads only its dots.",
          self.frame.demo(
            0,
            live,
            "PasswordInput::new(password.clone()).bind_reveal(shown.clone())",
            &[
              "// password: Signal<String>, shown: Signal<bool>",
              "PasswordInput::new(password.clone())",
              "    .placeholder(\"Password\")",
              "    .bind_reveal(shown.clone())",
            ],
          ),
        ),
        Section::new(
          "Strength",
          "The app judges the password and the meter shows it: four bars, as many filled as the strength reaches, \
           and what it says of it under them. Here the page judges what is typed by its length and its kinds of \
           characters.",
          self.frame.demo(
            1,
            meter,
            ".strength(judge(&password.get()))",
            &[
              "// password: Signal<String>; judge: fn(&str) -> Option<PasswordStrength>",
              "PasswordInput::new(password.clone())",
              "    .placeholder(\"New password\")",
              "    .new_password(true)",
              "    .bind_reveal(shown.clone())",
              "    .strength(judge(&password.get()))",
            ],
          ),
        ),
        Section::new(
          "Strengths",
          "Weak in the destructive color, fair in the warning color, good and strong in the success color.",
          self.frame.demo(
            2,
            strengths,
            ".strength(Some(PasswordStrength::Good))",
            &[
              "PasswordInput::new(value).strength(Some(PasswordStrength::Weak))",
              "PasswordInput::new(value).strength(Some(PasswordStrength::Fair))",
              "PasswordInput::new(value).strength(Some(PasswordStrength::Good))",
              "PasswordInput::new(value).strength(Some(PasswordStrength::Strong))",
            ],
          ),
        ),
        Section::new(
          "States",
          "The box is an input's: its edge says its state. Shown, the eye is in the foreground.",
          self.frame.demo(
            3,
            states,
            ".reveal(true)",
            &[
              "PasswordInput::new(value).placeholder(\"Password\").on_reveal(|_| {})",
              "PasswordInput::new(value).on_reveal(|_| {})",
              "PasswordInput::new(value).reveal(true).on_reveal(|_| {})",
              "PasswordInput::new(value).on_reveal(|_| {}).force(Some(Interaction::FocusVisible))",
              "PasswordInput::new(value).on_reveal(|_| {}).invalid(true)",
              "PasswordInput::new(value).on_reveal(|_| {}).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Sizes",
          "An input's three heights. The eye stays 16.",
          self.frame.demo(
            4,
            sizes,
            ".size(InputSize::Sm)",
            &[
              "PasswordInput::new(value).on_reveal(|_| {}).size(InputSize::Sm)",
              "PasswordInput::new(value).on_reveal(|_| {})",
              "PasswordInput::new(value).on_reveal(|_| {}).size(InputSize::Lg)",
            ],
          ),
        ),
        Section::new(
          "In a field",
          "A field names it and judges it as it does an input.",
          self.frame.demo(
            5,
            in_field,
            "Field::new(\"Password\", PasswordInput::new(password))",
            &[
              "Field::new(",
              "    \"Password\",",
              "    PasswordInput::new(password.clone()).bind_reveal(shown.clone()).full_width(true),",
              ")",
              ".description(\"At least 8 characters.\")",
              ".required(true)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a password input is given.",
          props_table(&rows, &["value"], self.frame.room()),
        ),
      ],
    )
  }
}
