//! The page of the fieldset: its playground with the board's profile in it, a fieldset that is disabled with
//! everything in it, and the error summary at the top of a form, whose lines focus their fields.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::Justify},
  node::Element,
  ui::{
    Button, ButtonVariant, CheckboxCard, CheckboxCardGroup, ErrorSummary, Field, FieldError, Fieldset, Input, Switch,
  },
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// The fields of the board's fieldset, in it: a required one, an optional one, one with its label beside it, and
/// the row of its actions.
fn profile(fieldset: Fieldset, name: &Signal<String>, bio: &Signal<String>, zone: &Signal<String>) -> Fieldset {
  fieldset
    .field(Field::new("Display name", Input::new(name.clone()).full_width(true)).required(true))
    .field(
      Field::new(
        "Bio",
        Input::new(bio.clone())
          .placeholder("Tell us what happened")
          .full_width(true),
      )
      .optional(true),
    )
    .field(Field::new("Time zone", Input::new(zone.clone()).full_width(true)).horizontal(true))
    .child(
      Row::new()
        .width(FILL)
        .spacing(8.0)
        .padding_top(4.0)
        .justify(Justify::End)
        .child(Button::new("Reset").variant(ButtonVariant::Outline))
        .child(Button::new("Save profile")),
    )
}

/// What is wrong with the name and the email, as the form of the page judges them.
fn judged(name: &str, email: &str) -> (Option<&'static str>, Option<&'static str>) {
  let at = email.find('@');
  let email_ok = at.is_some_and(|index| index > 0 && email[index + 1..].contains('.'));
  (
    name.trim().is_empty().then_some("Display name is required"),
    (!email_ok).then_some("Email must include a domain"),
  )
}

pub struct FieldsetPage {
  frame: Frame,
  playground: Playground,
  legend: Signal<String>,
  description: Signal<bool>,
  disabled: Signal<bool>,
  full_width: Signal<bool>,
  name: Signal<String>,
  bio: Signal<String>,
  zone: Signal<String>,
  reports: Signal<Vec<&'static str>>,
  sync: Signal<bool>,
  form_name: Signal<String>,
  form_email: Signal<String>,
  attempted: Signal<bool>,
  name_field: ElementRef,
  email_field: ElementRef,
}

impl FieldsetPage {
  /// The playground: the board's fieldset as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let legend = self.legend.get();
    let (description, disabled, full_width) = (self.description.get(), self.disabled.get(), self.full_width.get());
    let (name, bio, zone) = (self.name.clone(), self.bio.clone(), self.zone.clone());
    let plain = {
      let legend = legend.clone();
      move || {
        let mut fieldset = Fieldset::new(&legend).disabled(disabled).full_width(full_width);
        if description {
          fieldset = fieldset.description("Shown to people you share a workspace with.");
        }
        profile(fieldset, &name, &bio, &zone)
      }
    };
    let fieldset = plain();
    let make = {
      let plain = plain.clone();
      move || plain().into()
    };

    let mut code = vec![format!("Fieldset::new({legend:?})")];
    if description {
      code.push("    .description(\"Shown to people you share a workspace with.\")".to_owned());
    }
    code.extend(
      [
        "    .field(Field::new(\"Display name\", Input::new(name).full_width(true)).required(true))",
        "    .field(Field::new(\"Bio\", Input::new(bio).full_width(true)).optional(true))",
        "    .field(Field::new(\"Time zone\", Input::new(zone).full_width(true)).horizontal(true))",
        "    .child(Row::new().spacing(8.0).justify(Justify::End)",
        "        .child(Button::new(\"Reset\").variant(ButtonVariant::Outline))",
        "        .child(Button::new(\"Save profile\")))",
      ]
      .map(str::to_owned),
    );
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    if full_width {
      code.push("    .full_width(true)".to_owned());
    }

    let mut tree = vec![format!(
      "root          {}   padded 20 · edge 1, border · radius-lg · 16 apart",
      self.playground.measured()
    )];
    tree.push("├─ legend     4 apart".to_owned());
    tree.push(format!(
      "│  {} title    15, semibold",
      if description { "├─" } else { "└─" }
    ));
    if description {
      tree.push("│  └─ description  13, muted".to_owned());
    }
    tree.push("├─ field      Display name, required".to_owned());
    tree.push("├─ field      Bio, optional".to_owned());
    tree.push("├─ field      Time zone, its label beside it".to_owned());
    tree.push("└─ row        Reset · Save profile".to_owned());

    let to_legend = self.legend.clone();
    let flags = [self.disabled.clone(), self.full_width.clone()];
    let to_description = self.description.clone();
    self.playground.card(
      ctx,
      Spec {
        title: "Fieldset",
        make: Arc::new(make),
        shown: fieldset.into(),
        fill: full_width,
        forceable: false,
        facts: if disabled { "disabled" } else { "enabled" }.to_owned(),
        event: "",
        code,
        tree,
        controls: vec![
          prop("legend", "&str", field(&self.legend)),
          divider(),
          flag("description", &self.description),
          flag("disabled", &self.disabled),
          flag("full_width", &self.full_width),
        ],
        changed: [legend != "Profile", !description, disabled, full_width]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to_legend.set("Profile".to_owned());
          to_description.set(true);
          for flag in &flags {
            flag.set(false);
          }
        }),
      },
    )
  }

  /// The form of the error summary: the summary at its top once Save was pressed, over the two fields it links to.
  fn form(&self) -> Element {
    let (name, email) = (self.form_name.get(), self.form_email.get());
    let attempted = self.attempted.get();
    let (name_error, email_error) = if attempted { judged(&name, &email) } else { (None, None) };
    let mut errors = Vec::new();
    if let Some(message) = name_error {
      errors.push(FieldError::new("Display name", message).focuses(&self.name_field));
    }
    if let Some(message) = email_error {
      errors.push(FieldError::new("Email", message).focuses(&self.email_field));
    }
    let mut name_field = Field::new("Display name", Input::new(self.form_name.clone()).full_width(true))
      .required(true)
      .full_width(true);
    if let Some(message) = name_error {
      name_field = name_field.error(message);
    }
    let mut email_field = Field::new("Email", Input::new(self.form_email.clone()).full_width(true))
      .required(true)
      .full_width(true);
    if let Some(message) = email_error {
      email_field = email_field.error(message);
    }
    let save = self.attempted.clone();
    Fieldset::new("Account")
      .child(ErrorSummary::new(errors).full_width(true))
      .child(Column::new().ref_element(self.name_field.clone()).child(name_field))
      .child(Column::new().ref_element(self.email_field.clone()).child(email_field))
      .child(
        Row::new()
          .width(FILL)
          .justify(Justify::End)
          .child(Button::new("Save").on_click(move |_| save.set(true))),
      )
      .into()
  }
}

impl Component for FieldsetPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      legend: ctx.signal("Profile".to_owned()),
      description: ctx.signal(true),
      disabled: ctx.signal(false),
      full_width: ctx.signal(false),
      name: ctx.signal("Ivan Zaida".to_owned()),
      bio: ctx.signal(String::new()),
      zone: ctx.signal("Europe/Kyiv".to_owned()),
      reports: ctx.signal(vec!["crash"]),
      sync: ctx.signal(true),
      form_name: ctx.signal(String::new()),
      form_email: ctx.signal("ivan@".to_owned()),
      attempted: ctx.signal(false),
      name_field: ElementRef::new(),
      email_field: ElementRef::new(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let sync = self.sync.clone();
    let disabled = Fieldset::new("Notifications")
      .description("Managed by your workspace: ask an admin to change them.")
      .child(CheckboxCardGroup::new(
        self.reports.clone(),
        [
          (
            "crash",
            CheckboxCard::new("Crash reports").description("Send reports when lurq fails."),
          ),
          (
            "usage",
            CheckboxCard::new("Usage data").description("Which components are used."),
          ),
        ],
      ))
      .child(
        Row::new().align_items(Alignment::Center).child(
          Switch::new("Sync settings")
            .on(self.sync.get())
            .on_change(move |on| sync.set(on)),
        ),
      )
      .disabled(true);
    let fieldset_rows = [
      (
        "legend",
        "&str",
        "",
        "What the fields are, together, in 15 and semibold: what the fieldset is called to what reads the tree.",
      ),
      (
        "description",
        "&str",
        "",
        "More of it, 4 under the legend in 13 of the muted color.",
      ),
      (
        "field",
        "Field",
        "",
        "Adds a field, made as wide as the fieldset. 16 apart from what is over and under it.",
      ),
      (
        "child / children",
        "impl Into<Element>",
        "",
        "Adds anything else: a row of buttons, a group of cards, a summary of errors.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Everything in it takes no input and Tab passes it by; the whole fieldset is dimmed.",
      ),
      (
        "full_width",
        "bool",
        "false",
        "As wide as what holds it, in place of 400.",
      ),
    ];
    let summary_rows = [
      (
        "errors",
        "impl IntoIterator<FieldError>",
        "",
        "ErrorSummary::new: a line for each, in the order of the form. With none, it draws nothing.",
      ),
      (
        "FieldError::new",
        "&str, &str",
        "",
        "The label of the field and its message. The line shows the message; it is called by both.",
      ),
      (
        "focuses",
        "&ElementRef",
        "",
        "On a FieldError: a press of the line, or Enter on it, focuses the first control under the ref.",
      ),
      (
        "form",
        "&FormHandle, [(&str, &str, &ElementRef)]",
        "",
        "With the form feature: the errors of a form once a submit was attempted, a field to each name, label and \
         ref.",
      ),
      (
        "title",
        "&str",
        "\"N fields need attention\"",
        "The title in words of the app's own.",
      ),
      (
        "full_width",
        "bool",
        "false",
        "As wide as what holds it, in place of 400.",
      ),
    ];
    let playground = self.play(ctx);
    let form = self.form();
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Fieldset"],
        title: "Fieldset",
        description: "Fields that go together under a legend, which it disables together; and at the top of a form, \
                      the summary of what is wrong in it, with a link to each field.",
        import: Some("use lurq::ui::{ErrorSummary, FieldError, Fieldset};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Fieldset is a control on the right. The fieldset on the stage is the board's: a required \
           field, an optional one, one with its label beside it, and its actions.",
          playground,
        ),
        Section::new(
          "Disabled",
          "A disabled fieldset takes everything in it out of input, whatever it is, and is dimmed as a whole: no \
           card in it is pressed and Tab passes it by.",
          self.frame.demo(
            0,
            disabled,
            ".disabled(true)",
            &[
              "Fieldset::new(\"Notifications\")",
              "    .description(\"Managed by your workspace: ask an admin to change them.\")",
              "    .child(CheckboxCardGroup::new(reports, [",
              "        (\"crash\", CheckboxCard::new(\"Crash reports\").description(\"Send reports when lurq fails.\")),",
              "        (\"usage\", CheckboxCard::new(\"Usage data\").description(\"Which components are used.\")),",
              "    ]))",
              "    .child(Switch::new(\"Sync settings\").on(sync.get()).on_change(move |on| sync.set(on)))",
              "    .disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Error summary",
          "Press Save with the fields as they are: the summary says what is wrong at the top of the form, and a \
           press of a line, or Enter on it, focuses its field. Fill them in and press Save again: it is gone.",
          self.frame.demo(
            1,
            form,
            "ErrorSummary::new([FieldError::new(\"Email\", message).focuses(&email_field), ..])",
            &[
              "// errors: what the form found wrong once Save was pressed",
              "let mut errors = Vec::new();",
              "if let Some(message) = name_error {",
              "    errors.push(FieldError::new(\"Display name\", message).focuses(&name_field));",
              "}",
              "if let Some(message) = email_error {",
              "    errors.push(FieldError::new(\"Email\", message).focuses(&email_field));",
              "}",
              "Fieldset::new(\"Account\")",
              "    .child(ErrorSummary::new(errors).full_width(true))",
              "    .child(Column::new().ref_element(name_field).child(Field::new(\"Display name\", name_input)))",
              "    .child(Column::new().ref_element(email_field).child(Field::new(\"Email\", email_input)))",
            ],
          ),
        ),
        Section::new(
          "Fieldset props",
          "What a fieldset is given.",
          props_table(&fieldset_rows, &["legend"], self.frame.room()),
        ),
        Section::new(
          "ErrorSummary props",
          "What a summary of errors is given.",
          props_table(&summary_rows, &["errors"], self.frame.room()),
        ),
      ],
    )
  }
}
