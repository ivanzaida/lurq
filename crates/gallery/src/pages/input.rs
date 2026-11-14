//! The page of the input and the field: the input's playground, an input
//! bound to a signal, its states and sizes, and the field that holds a control
//! with its label and its description, names and judges it, and focuses it
//! from its label.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Field, Input, InputSize, Interaction, Switch, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// The icons the playground offers: what the select says, the glyph, and how the code names it.
const ICONS: [(&str, Option<&str>, &str); 4] = [
  ("none", None, ""),
  ("search", Some(icons::SEARCH), "icons::SEARCH"),
  ("mail", Some(icons::MAIL), "icons::MAIL"),
  ("check", Some(icons::CHECK), "icons::CHECK"),
];

/// The sizes, as the board names them, and how the code names them.
const SIZES: [(InputSize, &str, &str, f32); 3] = [
  (InputSize::Sm, "sm", "InputSize::Sm", 32.0),
  (InputSize::Md, "md", "InputSize::Md", 36.0),
  (InputSize::Lg, "lg", "InputSize::Lg", 40.0),
];

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

/// An input that holds `value` and cannot be typed in: for a stage that shows a state.
fn showing(value: &str) -> Input {
  Input::new(Signal::new(value.to_owned()))
}

pub struct InputPage {
  frame: Frame,
  playground: Playground,
  value: Signal<String>,
  placeholder: Signal<String>,
  label: Signal<String>,
  icon: Signal<usize>,
  size: Signal<usize>,
  invalid: Signal<bool>,
  warning: Signal<bool>,
  valid: Signal<bool>,
  read_only: Signal<bool>,
  disabled: Signal<bool>,
  full_width: Signal<bool>,
  name: Signal<String>,
  email: Signal<String>,
  workspace: Signal<String>,
}

impl InputPage {
  /// The playground: the input as the panel's props make it, bound to the page's signal.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let placeholder = self.placeholder.get();
    let name = self.label.get();
    let (label, glyph, glyph_code) = ICONS[self.icon.get().min(ICONS.len() - 1)];
    let (size, size_name, size_code, height) = SIZES[self.size.get().min(SIZES.len() - 1)];
    let flags = [
      ("invalid", self.invalid.get()),
      ("warning", self.warning.get()),
      ("valid", self.valid.get()),
      ("read_only", self.read_only.get()),
      ("disabled", self.disabled.get()),
      ("full_width", self.full_width.get()),
    ];
    let on = |name: &str| flags.iter().any(|(flag, set)| *flag == name && *set);
    let make = {
      let (placeholder, value, name) = (placeholder.clone(), self.value.clone(), name.clone());
      let set: Vec<bool> = flags.iter().map(|(_, set)| *set).collect();
      move || {
        let mut input = Input::new(value.clone())
          .placeholder(&placeholder)
          .size(size)
          .invalid(set[0])
          .warning(set[1])
          .valid(set[2])
          .read_only(set[3])
          .disabled(set[4])
          .full_width(set[5]);
        if let Some(glyph) = glyph {
          input = input.icon(glyph);
        }
        if !name.is_empty() {
          input = input.label(&name);
        }
        input
      }
    };
    let record = self.playground.recorder();
    let shown = make()
      .force(self.playground.forced())
      .on_input(move |event| record(format!("on_input({:?})", event.new_value)));

    let mut code = vec![
      "// value: Signal<String>".to_owned(),
      "Input::new(value.clone())".to_owned(),
      format!("    .placeholder({placeholder:?})"),
    ];
    if !name.is_empty() {
      code.push(format!("    .label({name:?})"));
    }
    if glyph.is_some() {
      code.push(format!("    .icon({glyph_code})"));
    }
    if size != InputSize::Md {
      code.push(format!("    .size({size_code})"));
    }
    for (name, set) in flags {
      if set {
        code.push(format!("    .{name}(true)"));
      }
    }

    let empty = self.value.get().is_empty();
    let shows = if empty {
      format!("placeholder {placeholder:?}")
    } else {
      format!("value {:?}", self.value.get())
    };
    let mut tree = vec![
      format!("root        {}", self.playground.measured()),
      format!(
        "{} control  {height} px high   {shows}",
        if glyph.is_some() { "├─" } else { "└─" }
      ),
    ];
    if glyph.is_some() {
      tree.push(format!("└─ icon     16 × 16 px   {label}"));
    }

    let icons: Vec<(&str, usize)> = ICONS.iter().enumerate().map(|(index, icon)| (icon.0, index)).collect();
    let sizes: Vec<(&str, usize)> = SIZES.iter().enumerate().map(|(index, size)| (size.1, index)).collect();
    let (to_icon, to_size) = (self.icon.clone(), self.size.clone());
    let reset = {
      let (value, placeholder, label, icon, size) = (
        self.value.clone(),
        self.placeholder.clone(),
        self.label.clone(),
        self.icon.clone(),
        self.size.clone(),
      );
      let flags = [
        self.invalid.clone(),
        self.warning.clone(),
        self.valid.clone(),
        self.read_only.clone(),
        self.disabled.clone(),
        self.full_width.clone(),
      ];
      move || {
        value.set(String::new());
        placeholder.set("Placeholder".to_owned());
        label.set(String::new());
        icon.set(0);
        size.set(1);
        for flag in &flags {
          flag.set(false);
        }
      }
    };
    let changed = flags.iter().filter(|(_, set)| *set).count()
      + usize::from(glyph.is_some())
      + usize::from(size != InputSize::Md)
      + usize::from(!empty)
      + usize::from(!name.is_empty())
      + usize::from(placeholder != "Placeholder");
    self.playground.card(
      ctx,
      Spec {
        title: "Input",
        make: Arc::new(move || make().into()),
        shown: shown.into(),
        fill: on("full_width"),
        forceable: true,
        facts: format!("{size_name} · radius 6"),
        event: "on_input",
        code,
        tree,
        controls: vec![
          prop("value", "Signal<String>", field(&self.value)),
          prop("placeholder", "&str", field(&self.placeholder)),
          prop("label", "&str", field(&self.label)),
          prop(
            "icon",
            "impl Into<Name>",
            self
              .playground
              .select(0, &icons, &self.icon.get(), move |index| to_icon.set(index)),
          ),
          prop(
            "size",
            "InputSize",
            segmented_fill(&sizes, &self.size.get(), move |index| to_size.set(index)),
          ),
          divider(),
          flag("invalid", &self.invalid),
          flag("warning", &self.warning),
          flag("valid", &self.valid),
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

impl Component for InputPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      value: ctx.signal(String::new()),
      placeholder: ctx.signal("Placeholder".to_owned()),
      label: ctx.signal(String::new()),
      icon: ctx.signal(0),
      size: ctx.signal(1),
      invalid: ctx.signal(false),
      warning: ctx.signal(false),
      valid: ctx.signal(false),
      read_only: ctx.signal(false),
      disabled: ctx.signal(false),
      full_width: ctx.signal(false),
      name: ctx.signal(String::new()),
      email: ctx.signal("ivan@".to_owned()),
      workspace: ctx.signal(String::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let says = |value: &Signal<String>| format!("the signal has: {:?}", value.get());
    let live = items().child(item(
      Input::new(self.name.clone()).placeholder("Display name"),
      &says(&self.name),
    ));
    let states = items()
      .child(item(showing("").placeholder("Placeholder"), "default"))
      .child(item(showing("Ivan Zaida"), "filled"))
      .child(item(showing("Placeholder").force(Some(Interaction::Hovered)), "hover"))
      .child(item(
        showing("Ivan Zaida").force(Some(Interaction::FocusVisible)),
        "focus",
      ))
      .child(item(showing("not-an-email").invalid(true), "invalid"))
      .child(item(showing("80").warning(true), "warning"))
      .child(item(
        showing("ivan@lurq.dev").valid(true).icon(icons::CHECK_CIRCLE),
        "valid",
      ))
      .child(item(showing("8770b598").read_only(true), "read only"))
      .child(item(showing("").placeholder("Read only").disabled(true), "disabled"))
      .child(item(
        showing("").placeholder("Search components").icon(icons::SEARCH),
        "with icon",
      ));
    let mut sizes = items();
    for (size, name, _, height) in SIZES {
      sizes = sizes.child(item(showing("Ivan Zaida").size(size), &format!("{name} · {height}")));
    }

    let email_ok = self.email.get().contains('@') && !self.email.get().ends_with('@');
    let fields = items()
      .child(
        Field::new(
          "Display name",
          Input::new(self.name.clone())
            .placeholder("Placeholder")
            .full_width(true),
        )
        .description("Shown in the title bar.")
        .required(true),
      )
      .child({
        // The field's error makes the input invalid: the input is not told.
        let field = Field::new("Email", Input::new(self.email.clone()).full_width(true));
        if email_ok {
          field.description("Where receipts go.")
        } else {
          field.error("Enter a full email address.")
        }
      })
      .child(Field::new("Bio", showing("").placeholder("Tell us what happened").full_width(true)).optional(true));
    let judged = items()
      .child(
        Field::new("Project name", showing("lurq design system").full_width(true)).description("18 / 40 characters"),
      )
      .child(
        Field::new("Port", showing("80").full_width(true))
          .description("Ports below 1024 need admin rights.")
          .warning(true),
      )
      .child(
        Field::new("Username", showing("ivanzaida").full_width(true))
          .description("This name is available.")
          .valid(true),
      )
      .child(
        Field::new("Invite code", showing("LURQ-2026").valid(true).full_width(true))
          .error("This code was used a moment ago."),
      );
    let focusing = items()
      .child(
        Field::new(
          "Workspace",
          Input::new(self.workspace.clone()).placeholder("acme").full_width(true),
        )
        .description("Press the label: the caret goes into the input."),
      )
      .child(
        Field::new("Region", showing("eu-central").full_width(true))
          .description("A disabled field's label focuses nothing.")
          .disabled(true),
      );
    let beside = Column::new()
      .spacing(16.0)
      .child(Field::new("Time zone", showing("Europe/Kyiv").full_width(true)).horizontal(true))
      .child(Field::new("Notifications", Switch::new("Pause notifications")).horizontal(true));

    let input_rows = [
      (
        "value",
        "Signal<String>",
        "",
        "What the input shows; what is typed is set on it.",
      ),
      (
        "placeholder",
        "&str",
        "",
        "What it says while it is empty, in the muted foreground.",
      ),
      (
        "label",
        "&str",
        "",
        "What it is called, to what reads the tree. It draws nothing; a field gives it its own label.",
      ),
      (
        "icon",
        "impl Into<Name>",
        "",
        "A glyph of the theme's set at the start of the box.",
      ),
      (
        "size",
        "InputSize",
        "Md",
        "Sm 32, Md 36, Lg 40 high; text of 13, 14, 15.",
      ),
      (
        "invalid",
        "bool",
        "false",
        "Its edge is in the destructive color, and in its focus.",
      ),
      ("warning", "bool", "false", "Its edge is in the warning color."),
      (
        "valid",
        "bool",
        "false",
        "Its edge is in the success color, and so is its icon.",
      ),
      (
        "read_only",
        "bool",
        "false",
        "It can be focused, selected and copied, and not changed.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Half the opacity on the muted fill; it takes no input.",
      ),
      (
        "full_width",
        "bool",
        "false",
        "As wide as what holds it, in place of 260.",
      ),
      (
        "on_input",
        "impl Fn(&TextInputEvent)",
        "",
        "Called before an edit, which it may refuse.",
      ),
      (
        "name",
        "impl Into<Arc<str>>",
        "",
        "The name of its value in the form it is in.",
      ),
    ];
    let field_rows = [
      ("label", "&str", "", "What the control is called, above it."),
      (
        "control",
        "impl FieldControl",
        "",
        "An input, a textarea, a switch, any element. An input or a textarea is called by the label and judged as \
         the field is, unless it says so itself; a press of the label focuses the control.",
      ),
      ("description", "&str", "", "What the control is for, under it."),
      (
        "error",
        "&str",
        "",
        "What is wrong, in place of the description; the field is invalid.",
      ),
      (
        "invalid / warning / valid",
        "bool",
        "false",
        "The color of the description, and the judgment of an input or a textarea in it.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Its control takes no input, and its label focuses nothing.",
      ),
      (
        "required",
        "bool",
        "false",
        "A * after the label, in the destructive color.",
      ),
      (
        "optional",
        "bool",
        "false",
        "\"Optional\" after the label; optional_text in other words.",
      ),
      (
        "horizontal",
        "bool",
        "false",
        "The label in a column of 110 beside the control.",
      ),
      (
        "full_width",
        "bool",
        "false",
        "As wide as what holds it, in place of 260.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Input"],
        title: "Input",
        description: "One line of text that a person types: a name, an email address, a search. A field holds it \
                      with its label above it and what it needs under it.",
        import: Some("use lurq::ui::{Field, Input};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Input is a control on the right. The input on the stage is live: what is typed in it goes \
           into the value on the right, and the other way.",
          playground,
        ),
        Section::new(
          "Input",
          "The page holds the text, in a signal. Tab reaches the input; a press puts the caret where it is.",
          self.frame.demo(
            0,
            live,
            "Input::new(name.clone()).placeholder(\"Display name\")",
            &[
              "// name: Signal<String>",
              "Input::new(name.clone())",
              "    .placeholder(\"Display name\")",
            ],
          ),
        ),
        Section::new(
          "States",
          "The edge is the only signal: the ring's color under the pointer and twice as thick in the focus; the \
           destructive, warning and success colors for what it holds. Read only and disabled are on the muted fill.",
          self.frame.demo(
            1,
            states,
            ".invalid(true)",
            &[
              "Input::new(value).placeholder(\"Placeholder\")",
              "",
              "// Drawn as it is under the pointer or focused, with nothing pointing at it: for a gallery, a test.",
              "Input::new(value).force(Some(Interaction::Hovered))",
              "Input::new(value).force(Some(Interaction::FocusVisible))",
              "",
              "Input::new(value).invalid(true)",
              "Input::new(value).warning(true)",
              "Input::new(value).valid(true).icon(icons::CHECK_CIRCLE)",
              "Input::new(value).read_only(true)",
              "Input::new(value).placeholder(\"Read only\").disabled(true)",
              "Input::new(value).placeholder(\"Search components\").icon(icons::SEARCH)",
            ],
          ),
        ),
        Section::new(
          "Sizes",
          "The heights of the theme's controls, with the text a step smaller or larger. The padding stays 12.",
          self.frame.demo(
            2,
            sizes,
            ".size(InputSize::Sm)",
            &[
              "Input::new(value).size(InputSize::Sm)",
              "Input::new(value)",
              "Input::new(value).size(InputSize::Lg)",
            ],
          ),
        ),
        Section::new(
          "Field",
          "A control with its label above and its description under it, 6 apart. An error takes the place of the \
           description and makes the input invalid; the email here is checked as it is typed. A required field has \
           a mark, an optional one says so.",
          self.frame.demo(
            3,
            fields,
            "Field::new(\"Email\", input).error(\"Enter a full email address.\")",
            &[
              "// name, email: Signal<String>",
              "Field::new(\"Display name\", Input::new(name.clone()).full_width(true))",
              "    .description(\"Shown in the title bar.\")",
              "    .required(true)",
              "",
              "let ok = email.get().contains('@') && !email.get().ends_with('@');",
              "// The field's error makes the input invalid: the input is not told.",
              "let field = Field::new(\"Email\", Input::new(email.clone()).full_width(true));",
              "if ok {",
              "    field.description(\"Where receipts go.\")",
              "} else {",
              "    field.error(\"Enter a full email address.\")",
              "}",
              "",
              "Field::new(\"Bio\", Input::new(bio.clone()).full_width(true)).optional(true)",
            ],
          ),
        ),
        Section::new(
          "Field: what the control holds",
          "The description in the color of a judgment, which the field gives the input in it too. An input the app \
           judged itself keeps its own judgment.",
          self.frame.demo(
            4,
            judged,
            ".warning(true)",
            &[
              "Field::new(\"Project name\", Input::new(name).full_width(true))",
              "    .description(\"18 / 40 characters\")",
              "",
              "Field::new(\"Port\", Input::new(port).full_width(true))",
              "    .description(\"Ports below 1024 need admin rights.\")",
              "    .warning(true)",
              "",
              "Field::new(\"Username\", Input::new(user).full_width(true))",
              "    .description(\"This name is available.\")",
              "    .valid(true)",
              "",
              "// The input says its own judgment, and keeps it.",
              "Field::new(\"Invite code\", Input::new(code).valid(true).full_width(true))",
              "    .error(\"This code was used a moment ago.\")",
            ],
          ),
        ),
        Section::new(
          "Field: the label focuses the control",
          "A press of the label focuses the control, and the field gives the control its label as its name to what \
           reads the tree. A disabled field's control takes no input, and its label focuses nothing.",
          self.frame.demo(
            6,
            focusing,
            ".disabled(true)",
            &[
              "// workspace: Signal<String>",
              "Field::new(\"Workspace\", Input::new(workspace.clone()).placeholder(\"acme\").full_width(true))",
              "    .description(\"Press the label: the caret goes into the input.\")",
              "",
              "Field::new(\"Region\", Input::new(region).full_width(true))",
              "    .description(\"A disabled field's label focuses nothing.\")",
              "    .disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Field: the label beside",
          "In a form whose labels line up, the label is in a column of 110 beside the control. Any element is a \
           control.",
          self.frame.demo(
            5,
            beside,
            ".horizontal(true)",
            &[
              "Field::new(\"Time zone\", Input::new(zone).full_width(true)).horizontal(true)",
              "Field::new(\"Notifications\", Switch::new(\"Pause notifications\")).horizontal(true)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What an input is given.",
          props_table(&input_rows, &["value"], self.frame.room()),
        ),
        Section::new(
          "Field props",
          "What a field is given.",
          props_table(&field_rows, &["label", "control"], self.frame.room()),
        ),
      ],
    )
  }
}
