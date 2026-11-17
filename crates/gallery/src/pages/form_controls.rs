//! The page of the form controls of `lurq::components`: the design system's field, input, textarea, checkbox,
//! sliders and buttons bound to a form. A playground with a small form that validates and submits, then its
//! errors and how an edit clears them, disabled and submitting controls, the legacy theme beside the design's,
//! and a field built of the components for what the form controls do not have.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize, Theme, ThemeDefinition},
  },
  components::{
    Column, ErrorVisibility, FormCheckboxInput, FormCheckboxInputProps, FormHandle, FormOptions, FormPrimaryButton,
    FormPrimaryButtonProps, FormRangeSliderInput, FormRangeSliderInputProps, FormSecondaryButton,
    FormSecondaryButtonProps, FormSliderInput, FormSliderInputProps, FormTextInput, FormTextInputProps, FormValues,
    Row, Text, ValidationResult, validators,
  },
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Field, Input, InputSize, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// When the form shows a field's error.
const ERRORS: [(&str, usize); 3] = [("TouchedOrSubmitted", 0), ("Always", 1), ("Never", 2)];

fn visibility(index: usize) -> ErrorVisibility {
  match index {
    1 => ErrorVisibility::Always,
    2 => ErrorVisibility::Never,
    _ => ErrorVisibility::TouchedOrSubmitted,
  }
}

/// What the scene's form tells the page: a line for the Events tab.
#[derive(Clone)]
struct Tell(Arc<dyn Fn(String) + Send + Sync>);

/// The page's recorder is the page's for its life: two are one.
impl PartialEq for Tell {
  fn eq(&self, _other: &Self) -> bool {
    true
  }
}

/// The page's signal of whether the playground's form is submitting, which is the page's for its life.
#[derive(Clone)]
struct Submitting(Signal<bool>);

impl PartialEq for Submitting {
  fn eq(&self, _other: &Self) -> bool {
    true
  }
}

/// Which controls a scene's form has.
#[derive(Clone, Copy, PartialEq, Eq, lurq::DevtoolsInspectable)]
enum Kind {
  /// The whole form: every control and the two buttons.
  Whole,
  /// The email alone, already wrong, and its button.
  Errors,
}

/// How a scene's form is made and how its controls are told.
#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct Setup {
  kind: Kind,
  label: Arc<str>,
  hint: Arc<str>,
  placeholder: Arc<str>,
  required: bool,
  disabled: bool,
  errors: usize,
  /// The form is submitting from its first render.
  submitting: bool,
  /// What the page sets while the playground's form is submitting.
  #[devtools_ignore]
  submitting_signal: Option<Submitting>,
  #[devtools_ignore]
  tell: Option<Tell>,
}

impl Setup {
  fn whole() -> Self {
    Self {
      kind: Kind::Whole,
      label: Arc::from("Email"),
      hint: Arc::from("We never share it."),
      placeholder: Arc::from("you@example.com"),
      required: true,
      disabled: false,
      errors: 0,
      submitting: false,
      submitting_signal: None,
      tell: None,
    }
  }
}

/// What the scene's form submits, as a line of the Events tab.
fn said(values: &FormValues) -> String {
  let (low, high) = values.get_range("price").unwrap_or_default();
  format!(
    "on_submit(email: {:?}, terms: {}, volume: {}, price: {low}..{high})",
    values.get_string("email").unwrap_or_default(),
    values.get_bool("terms").unwrap_or_default(),
    values.get_number("volume").unwrap_or_default(),
  )
}

/// A form of the form controls, made as its setup says.
struct FormScene {
  form: FormHandle,
}

impl Component for FormScene {
  type Props = Setup;

  fn create(ctx: &mut Ctx) -> Self {
    let setup = ctx.props::<Setup>().clone();
    let options = FormOptions::new()
      .field("name", "Ivan Zaida")
      .field("email", if setup.kind == Kind::Errors { "ivan@" } else { "" })
      .field("notes", "")
      .field("terms", false)
      .field("volume", 40.0)
      .field("price", (20, 80))
      .validate_string("name", validators::required("Enter your name"))
      .validate_string("email", validators::required("Enter your email"))
      .validate_string("email", validators::email("Enter a full email address"))
      .validate_bool("terms", |accepted, _| {
        if accepted {
          ValidationResult::valid()
        } else {
          ValidationResult::invalid("Accept the terms to go on")
        }
      })
      .validate_range("price", validators::range_span(0.0, 100.0, 10.0, "Keep them 10 apart"));
    let (submitted, invalid) = (setup.tell.clone(), setup.tell.clone());
    let form = ctx
      .form(options)
      .on_submit(move |values| {
        if let Some(tell) = &submitted {
          (tell.0)(said(&values));
        }
      })
      .on_invalid(move |errors| {
        if let Some(tell) = &invalid {
          (tell.0)(format!("on_invalid({} fields)", errors.len()));
        }
      });
    if setup.kind == Kind::Errors {
      form.validate_field("email");
    }
    if setup.submitting {
      form.set_submitting(true);
    }
    if let Some(Submitting(submitting)) = &setup.submitting_signal {
      form.set_submitting(submitting.get_untracked());
      let held = form.clone();
      ctx.watch(submitting, move |on| held.set_submitting(*on));
    }
    Self { form }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let setup = ctx.props::<Setup>().clone();
    let form = self.form.clone();
    let email = form
      .string_control("email")
      .required(setup.required)
      .disabled(setup.disabled)
      .error_visibility(visibility(setup.errors));
    let email = FormTextInputProps::new(email)
      .label(setup.label.clone())
      .hint(setup.hint.clone())
      .placeholder(setup.placeholder.clone());
    ctx.form_view(form.clone(), |ctx| {
      // Taller than the playground's stage, which scrolls it: room above and below, from its edges.
      let mut column = Column::new()
        .width(360.0)
        .max_width(FILL)
        .padding_top(16.0)
        .padding_bottom(16.0)
        .spacing(16.0)
        .overflow_visible();
      if setup.kind == Kind::Whole {
        column = column.child(ctx.mount::<FormTextInput>(
          FormTextInputProps::new(form.string_control("name").required(true).disabled(setup.disabled)).label("Name"),
        ));
      }
      column = column.child(ctx.mount::<FormTextInput>(email));
      if setup.kind == Kind::Whole {
        let reset = form.clone();
        let tell = setup.tell.clone();
        column = column
          .child(
            ctx.mount::<FormTextInput>(
              FormTextInputProps::new(form.string_control("notes").disabled(setup.disabled))
                .label("Notes")
                .placeholder("Anything we should know")
                .multiline()
                .height(88.0),
            ),
          )
          .child(
            ctx.mount::<FormCheckboxInput>(
              FormCheckboxInputProps::new(form.bool_control("terms").disabled(setup.disabled))
                .label("I accept the terms")
                .hint("Read them before you go on."),
            ),
          )
          .child(
            ctx.mount::<FormSliderInput>(
              FormSliderInputProps::new(form.number_control("volume").disabled(setup.disabled))
                .label("Volume")
                .range(0, 100),
            ),
          )
          .child(
            ctx.mount::<FormRangeSliderInput>(
              FormRangeSliderInputProps::new(form.range_control("price").disabled(setup.disabled))
                .label("Price")
                .hint("At least 10 apart.")
                .range(0, 100),
            ),
          )
          .child(ctx.mount::<FormPrimaryButton>(FormPrimaryButtonProps::new("Create account")))
          .child(
            ctx.mount::<FormSecondaryButton>(FormSecondaryButtonProps::new("Reset").on_click(move |_| {
              reset.reset();
              if let Some(tell) = &tell {
                (tell.0)("on_click(\"Reset\")".to_owned());
              }
            })),
          );
      } else {
        column = column.child(ctx.mount::<FormPrimaryButton>(FormPrimaryButtonProps::new("Save")));
      }
      column
    })
  }
}

/// A field of the design system's components bound to a form's control, for what the form controls do not have:
/// a large input with an icon.
struct BuiltField {
  form: FormHandle,
}

impl Component for BuiltField {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      form: ctx.form(
        FormOptions::new()
          .field("email", "")
          .validate_string("email", validators::email("Enter a full email address")),
      ),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    ctx.form_view(self.form.clone(), |ctx| {
      let email = ctx.string_control("email");
      let mut field = Field::new(
        "Email",
        Input::new(email.value())
          .name(email.name())
          .icon(icons::MAIL)
          .size(InputSize::Lg)
          .placeholder("you@example.com")
          .full_width(true),
      )
      .description("Press Save with an address that has no domain.")
      .full_width(true);
      if let Some(error) = email.visible_error() {
        field = field.error(&error);
      }
      Column::new()
        .width(360.0)
        .max_width(FILL)
        .spacing(16.0)
        .overflow_visible()
        .child(field)
        .child(ctx.mount::<FormPrimaryButton>(FormPrimaryButtonProps::new("Save")))
    })
  }
}

pub struct FormControlsPage {
  frame: Frame,
  playground: Playground,
  label: Signal<String>,
  hint: Signal<String>,
  placeholder: Signal<String>,
  required: Signal<bool>,
  disabled: Signal<bool>,
  errors: Signal<usize>,
  submitting: Signal<bool>,
  /// The theme of 0.62, beside the design's.
  legacy: Theme,
}

impl FormControlsPage {
  fn setup(&self) -> Setup {
    let tell = self.playground.recorder();
    Setup {
      label: Arc::from(self.label.get().as_str()),
      hint: Arc::from(self.hint.get().as_str()),
      placeholder: Arc::from(self.placeholder.get().as_str()),
      required: self.required.get(),
      disabled: self.disabled.get(),
      errors: self.errors.get().min(ERRORS.len() - 1),
      submitting_signal: Some(Submitting(self.submitting.clone())),
      tell: Some(Tell(Arc::new(tell))),
      ..Setup::whole()
    }
  }

  fn play(&self, ctx: &mut Ctx) -> Element {
    let setup = self.setup();
    {
      let setup = setup.clone();
      self
        .playground
        .window_shows(move |ctx| ctx.mount::<FormScene>(setup.clone()));
    }
    let shown = {
      let setup = setup.clone();
      Shown::built(move |ctx| ctx.mount::<FormScene>(setup))
    };
    let make = {
      let setup = setup.clone();
      move || -> Element { Element::component::<FormScene>(setup.clone()) }
    };

    let mut email = vec![format!("form.string_control(\"email\")")];
    if setup.required {
      email.push(".required(true)".to_owned());
    }
    if setup.disabled {
      email.push(".disabled(true)".to_owned());
    }
    if setup.errors != 0 {
      email.push(format!(
        ".error_visibility(ErrorVisibility::{})",
        ERRORS[setup.errors].0
      ));
    }
    let mut code = vec![
      "ctx.form_view(form.clone(), |ctx| Column::new()".to_owned(),
      "    .child(ctx.mount::<FormTextInput>(FormTextInputProps::new(form.string_control(\"name\").required(true)).label(\"Name\")))".to_owned(),
      format!("    .child(ctx.mount::<FormTextInput>(FormTextInputProps::new({})", email.concat()),
      format!(
        "        .label({:?}).hint({:?}).placeholder({:?})))",
        &*setup.label, &*setup.hint, &*setup.placeholder
      ),
      "    .child(ctx.mount::<FormTextInput>(FormTextInputProps::new(form.string_control(\"notes\")).label(\"Notes\").multiline().height(88.0)))".to_owned(),
      "    .child(ctx.mount::<FormCheckboxInput>(FormCheckboxInputProps::new(form.bool_control(\"terms\")).label(\"I accept the terms\")))".to_owned(),
      "    .child(ctx.mount::<FormSliderInput>(FormSliderInputProps::new(form.number_control(\"volume\")).label(\"Volume\").range(0, 100)))".to_owned(),
      "    .child(ctx.mount::<FormRangeSliderInput>(FormRangeSliderInputProps::new(form.range_control(\"price\")).label(\"Price\").range(0, 100)))".to_owned(),
      "    .child(ctx.mount::<FormPrimaryButton>(FormPrimaryButtonProps::new(\"Create account\")))".to_owned(),
      "    .child(ctx.mount::<FormSecondaryButton>(FormSecondaryButtonProps::new(\"Reset\").on_click(move |_| form.reset()))))".to_owned(),
    ];
    if self.submitting.get() {
      code.push("form.set_submitting(true);".to_owned());
    }

    let tree = vec![
      format!("form          {}   16 apart", self.playground.measured()),
      "├─ field      Name · ui::Field + ui::Input, required".to_owned(),
      format!("├─ field      {} · ui::Field + ui::Input", &*setup.label),
      "├─ field      Notes · ui::Field + ui::Textarea, 88 high".to_owned(),
      "├─ field      ui::Checkbox, its label beside the box".to_owned(),
      "├─ field      Volume · ui::Slider, as wide as the field".to_owned(),
      "├─ field      Price · ui::RangeSlider".to_owned(),
      "├─ button     ui::Button, Primary, submits".to_owned(),
      "└─ button     ui::Button, Outline".to_owned(),
    ];

    let (to_label, to_hint, to_placeholder) = (self.label.clone(), self.hint.clone(), self.placeholder.clone());
    let (to_required, to_disabled, to_submitting) =
      (self.required.clone(), self.disabled.clone(), self.submitting.clone());
    let (to_errors, pick_errors) = (self.errors.clone(), self.errors.clone());
    let changed = [
      &*setup.label != "Email",
      &*setup.hint != "We never share it.",
      &*setup.placeholder != "you@example.com",
      !setup.required,
      setup.disabled,
      setup.errors != 0,
      self.submitting.get(),
    ]
    .into_iter()
    .filter(|changed| *changed)
    .count();
    self.playground.card(
      ctx,
      Spec {
        title: "Form controls",
        make: Arc::new(make),
        shown,
        fill: false,
        forceable: false,
        facts: if self.submitting.get() {
          "submitting".to_owned()
        } else {
          "the email is validated on submit".to_owned()
        },
        event: "on_submit",
        code,
        tree,
        controls: vec![
          prop("label", "&str", field(&self.label)),
          prop("hint", "&str", field(&self.hint)),
          prop("placeholder", "&str", field(&self.placeholder)),
          divider(),
          flag("required", &self.required),
          flag("disabled", &self.disabled),
          prop(
            "error_visibility",
            "ErrorVisibility",
            self
              .playground
              .select(0, &ERRORS, &setup.errors, move |which| pick_errors.set(which)),
          ),
          divider(),
          flag("submitting", &self.submitting),
        ],
        changed,
        reset: Arc::new(move || {
          to_label.set("Email".to_owned());
          to_hint.set("We never share it.".to_owned());
          to_placeholder.set("you@example.com".to_owned());
          to_required.set(true);
          to_disabled.set(false);
          to_errors.set(0);
          to_submitting.set(false);
        }),
      },
    )
  }
}

/// A caption over a scene beside another.
fn captioned(caption: &str, scene: Element) -> Element {
  Column::new()
    .width(360.0)
    .max_width(FILL)
    .spacing(12.0)
    .overflow_visible()
    .child(Text::new(caption))
    .child(scene)
    .into()
}

impl Component for FormControlsPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let legacy = Theme::new();
    legacy.apply(ThemeDefinition::legacy());
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      label: ctx.signal("Email".to_owned()),
      hint: ctx.signal("We never share it.".to_owned()),
      placeholder: ctx.signal("you@example.com".to_owned()),
      required: ctx.signal(true),
      disabled: ctx.signal(false),
      errors: ctx.signal(0),
      submitting: ctx.signal(false),
      legacy,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let playground = self.play(ctx);
    let errors = ctx.mount::<FormScene>(Setup {
      kind: Kind::Errors,
      errors: 1,
      ..Setup::whole()
    });
    let disabled = ctx.mount::<FormScene>(Setup {
      disabled: true,
      ..Setup::whole()
    });
    let submitting = ctx.mount::<FormScene>(Setup {
      submitting: true,
      ..Setup::whole()
    });
    let states = Row::new()
      .width(FILL)
      .wrap()
      .spacing(32.0)
      .align_items(Alignment::Start)
      .overflow_visible()
      .child(captioned("Disabled", disabled))
      .child(captioned("Submitting", submitting));
    // The scope is on a node of its own around the form: a mounted component's root is the component's. The
    // legacy theme has one palette, light: the form stands on its background, in a dark page too.
    let legacy = ctx.in_theme(&self.legacy, |ctx| {
      Column::new()
        .width(FILL)
        .padding_horizontal(16.0)
        .rounded(RadiusSize::Md)
        .background(PaletteColor::Background)
        .overflow_visible()
        .child(ctx.mount::<FormScene>(Setup::whole()))
    });
    let design = ctx.mount::<FormScene>(Setup::whole());
    let themes = Row::new()
      .width(FILL)
      .wrap()
      .spacing(32.0)
      .align_items(Alignment::Start)
      .overflow_visible()
      .child(captioned("ThemeDefinition::legacy()", legacy))
      .child(captioned("The design's theme", design));
    let built = ctx.mount::<BuiltField>(());

    let rows = [
      (
        "control",
        "Control<T>",
        "",
        "What the control is bound to: form.string_control(name), number_control, bool_control, range_control. \
         Its options say whether it is required, disabled and when its error shows.",
      ),
      (
        "label",
        "impl Into<Arc<str>>",
        "",
        "The field's label above the control; a checkbox's beside its box.",
      ),
      (
        "hint",
        "impl Into<Arc<str>>",
        "",
        "Under the control in the muted color; the form's error takes its place.",
      ),
      (
        "placeholder",
        "impl Into<Arc<str>>",
        "",
        "FormTextInput: what the box says while it is empty.",
      ),
      (
        "multiline",
        "",
        "",
        "FormTextInput: a ui::Textarea, where Enter is a new line, in place of a ui::Input.",
      ),
      (
        "height",
        "impl Into<Dimension>",
        "36",
        "FormTextInput: the height of the box.",
      ),
      (
        "range",
        "i32, i32",
        "0, 100",
        "FormSliderInput, FormRangeSliderInput: the least and the greatest value, in whole steps.",
      ),
      (
        "submit / button",
        "",
        "submit; button",
        "FormPrimaryButton submits unless made button(); FormSecondaryButton submits nothing unless made submit().",
      ),
      (
        "on_click",
        "Fn(&MouseEvent)",
        "",
        "The buttons: called on a press, before the form is submitted.",
      ),
    ];
    let control_rows = [
      (
        "required",
        "bool",
        "false",
        "The field's * after its label. What is required is the validators' to say.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "The control takes no input and its label focuses nothing. Every control of a form that is submitting is \
         disabled too.",
      ),
      (
        "error_visibility",
        "ErrorVisibility",
        "TouchedOrSubmitted",
        "When the error shows: once the control was left or the form submitted, always, or never.",
      ),
    ];

    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Form controls"],
        title: "Form controls",
        description: "The form's own controls: a field, an input, a textarea, a checkbox, the sliders and the two \
                      buttons of the design system, bound to a form that validates and submits them.",
        import: Some(
          "use lurq::components::{FormTextInput, FormCheckboxInput, FormSliderInput, FormRangeSliderInput, \
           FormPrimaryButton, FormSecondaryButton};",
        ),
        status: Some("Stable"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "A form of every control, with the email's props on the right. Press Create account with the email \
           empty: the form says what is wrong under each field. Fill them in, accept the terms and press it \
           again: the Events tab has what was submitted.",
          playground,
        ),
        Section::new(
          "Errors",
          "The form's error is the field's: in the hint's place, in the destructive color, and the box judged \
           invalid. An edit of the box clears it; a submit judges it again.",
          self.frame.demo(
            1,
            errors,
            ".error_visibility(ErrorVisibility::Always)",
            &[
              "let form = ctx.form(FormOptions::new()",
              "    .field(\"email\", \"ivan@\")",
              "    .validate_string(\"email\", validators::email(\"Enter a full email address\")));",
              "form.validate_field(\"email\");",
              "ctx.mount::<FormTextInput>(FormTextInputProps::new(",
              "    form.string_control(\"email\").required(true).error_visibility(ErrorVisibility::Always))",
              "    .label(\"Email\").hint(\"We never share it.\"))",
            ],
          ),
        ),
        Section::new(
          "Disabled and submitting",
          "A control made disabled takes no typing, no press and no focus, and its label focuses nothing. While \
           the form submits, every control of it is disabled the same way, and a second submit does nothing.",
          self.frame.demo(
            2,
            states,
            "form.string_control(\"email\").disabled(true)",
            &[
              "form.string_control(\"email\").disabled(true)",
              "form.bool_control(\"terms\").disabled(true)",
              "// While an action submits it, or as the app says:",
              "form.set_submitting(true);",
            ],
          ),
        ),
        Section::new(
          "The legacy theme",
          "The form under ThemeDefinition::legacy() beside the design's theme: the same components, in the legacy \
           palette's roles, which keep 0.62's accent, error and focus colors.",
          self.frame.demo(
            3,
            themes,
            "app.theme().apply(ThemeDefinition::legacy())",
            &[
              "let legacy = Theme::new();",
              "legacy.apply(ThemeDefinition::legacy());",
              "ctx.in_theme(&legacy, |ctx| form_of_every_control(ctx))",
            ],
          ),
        ),
        Section::new(
          "Built of the components",
          "A form control has its props and no more. A field that needs what they lack (a size, an icon) is built \
           of ui::Field and ui::Input, bound to the form's control: its value, its name and its error.",
          self.frame.demo(
            4,
            built,
            "Field::new(\"Email\", Input::new(email.value()).name(email.name()))",
            &[
              "let email = ctx.string_control(\"email\");",
              "let mut field = Field::new(\"Email\", Input::new(email.value())",
              "    .name(email.name())",
              "    .icon(icons::MAIL)",
              "    .size(InputSize::Lg)",
              "    .full_width(true))",
              "  .full_width(true);",
              "if let Some(error) = email.visible_error() {",
              "    field = field.error(&error);",
              "}",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What the form controls are given.",
          props_table(&rows, &["control"], self.frame.room()),
        ),
        Section::new(
          "Control options",
          "What a control of the form says, which its field shows.",
          props_table(&control_rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
