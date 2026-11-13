//! A field: a control with what it is called above it and what it needs under it.

use std::sync::Arc;

use super::{
  names,
  part_style::{Length, PartStyle},
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{ControlSize, PaletteColor, Theme, TypographyStyle},
  components::{Column, Row},
  layout::{Alignment, text_style::FontWeight},
  node::Element,
};

/// The parts of a field a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldPart {
  /// The field: its label, its control and its description, one under another, or the label beside the rest.
  Root,
  /// What the control is called, with its mark.
  Label,
  /// The mark after the label of a field that has to be filled in.
  Required,
  /// The mark after the label of a field that may be left empty.
  Optional,
  /// The control and its description, where the label is beside them.
  Body,
  /// What the control is for, or what is wrong with what it holds, under it.
  Description,
}

names!(FieldPart {
  Root => "root",
  Label => "label",
  Required => "required",
  Optional => "optional",
  Body => "body",
  Description => "description",
});

/// The states of a field a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldState {
  /// What its control holds is wrong: its description says so in the destructive color.
  Invalid,
  /// What its control holds is allowed, with a caveat, which its description says in the warning color.
  Warning,
  /// What its control holds was checked and is right, as its description says in the success color.
  Valid,
  /// Its label is beside its control, not above it.
  Horizontal,
  /// It is as wide as what holds it.
  FullWidth,
}

names!(FieldState {
  Invalid => "invalid",
  Warning => "warning",
  Valid => "valid",
  Horizontal => "horizontal",
  FullWidth => "full_width",
});

/// What builds a field's nodes: a theme's in place of the default one.
pub type FieldRenderer = Arc<dyn Fn(FieldView) -> Element + Send + Sync>;

/// A field: a control with what it is called above it, and under it what it
/// is for or what is wrong with what it holds.
///
/// ```
/// use lurq::{core::Signal, ui::{Field, Input}};
///
/// let name = Signal::new(String::new());
/// let field = Field::new("Display name", Input::new(name.clone()).full_width(true))
///   .description("Shown in the title bar.")
///   .required(true);
/// let email = Field::new("Email", Input::new(Signal::new("ivan@".to_owned())).invalid(true).full_width(true))
///   .error("Enter a full email address.");
/// ```
///
/// Its control is any element: an [`Input`](super::Input), a select, a
/// switch. The field does not reach into it: a control whose content is
/// wrong is told so itself (`Input::invalid`), and the field says why
/// ([`Field::error`]).
pub struct Field {
  label: Arc<str>,
  control: Element,
  description: Option<Arc<str>>,
  error: Option<Arc<str>>,
  invalid: bool,
  warning: bool,
  valid: bool,
  required: bool,
  optional: Option<Arc<str>>,
  horizontal: bool,
  full_width: bool,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Field {
  /// A field called `label` that holds `control`.
  pub fn new(label: &str, control: impl Into<Element>) -> Self {
    Self {
      label: Arc::from(label),
      control: control.into(),
      description: None,
      error: None,
      invalid: false,
      warning: false,
      valid: false,
      required: false,
      optional: None,
      horizontal: false,
      full_width: false,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// What the control is for, under it: a hint, a count of characters, a caveat.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// What is wrong with what the control holds, under it in place of the description. The field is invalid.
  pub fn error(mut self, error: &str) -> Self {
    self.error = Some(Arc::from(error));
    self
  }

  /// Whether what the control holds is wrong: the description is in the destructive color.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.invalid = invalid;
    self
  }

  /// Whether what the control holds is allowed with a caveat, which the description says.
  pub fn warning(mut self, warning: bool) -> Self {
    self.warning = warning;
    self
  }

  /// Whether what the control holds was checked and is right, as the description says.
  pub fn valid(mut self, valid: bool) -> Self {
    self.valid = valid;
    self
  }

  /// Whether the field has to be filled in: a mark after its label.
  pub fn required(mut self, required: bool) -> Self {
    self.required = required;
    self
  }

  /// Whether the field may be left empty: "Optional" after its label. A required field shows no such mark.
  pub fn optional(mut self, optional: bool) -> Self {
    self.optional = optional.then(|| Arc::from("Optional"));
    self
  }

  /// Says that the field may be left empty in words of the app's own: in another language.
  pub fn optional_text(mut self, text: &str) -> Self {
    self.optional = Some(Arc::from(text));
    self
  }

  /// Whether the label is beside the control, in a column of its own, and not above it.
  pub fn horizontal(mut self, horizontal: bool) -> Self {
    self.horizontal = horizontal;
    self
  }

  /// Whether the field is as wide as what holds it. A control in it that is to be as wide too says so itself
  /// (`Input::full_width`).
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self
  }

  /// Puts the field in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this field over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this field.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the field's root.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the field in `theme`. Converting a field into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    // One judgment is shown, as an input shows it: an error makes the field invalid.
    let invalid = self.invalid || self.error.is_some();
    let warning = self.warning && !invalid;
    let valid = self.valid && !invalid && !warning;
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(FieldState::Invalid, invalid)
    .state(FieldState::Warning, warning)
    .state(FieldState::Valid, valid)
    .state(FieldState::Horizontal, self.horizontal)
    .state(FieldState::FullWidth, self.full_width);
    let view = FieldView {
      label: self.label,
      control: self.control,
      description: self.error.or(self.description),
      required: self.required,
      optional: self.optional.filter(|_| !self.required),
      horizontal: self.horizontal,
      recipe: theme.resolve::<Field>(&case, Some(&self.rules)),
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Field>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Field> for Element {
  fn from(field: Field) -> Self {
    field.build(&current_theme())
  }
}

/// A field as a renderer sees it: what it was given and the style of its parts.
pub struct FieldView {
  pub label: Arc<str>,
  /// What the field holds.
  pub control: Element,
  /// What is under the control: the error where there is one, else the description.
  pub description: Option<Arc<str>>,
  pub required: bool,
  /// The words of the mark of a field that may be left empty, where it has the mark.
  pub optional: Option<Arc<str>>,
  pub horizontal: bool,
  /// The style of each part of this field.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl FieldView {
  /// The style of a part, as this field is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// A text in the style of a part, in a node that has the part's color.
  fn text(&self, part: FieldPart, content: &str) -> Row {
    let part = self.part(part);
    Row::new().part(&part).child(part.text(content))
  }

  /// The label, with the mark of a field that has to be filled in or may be left empty.
  pub fn label(&self) -> Row {
    let part = self.part(FieldPart::Label);
    let mut label = Row::new()
      .align_items(Alignment::Center)
      .part(&part)
      .child(part.text(&self.label).nowrap());
    if self.required {
      label = label.child(self.text(FieldPart::Required, "*"));
    } else if let Some(optional) = &self.optional {
      label = label.child(self.text(FieldPart::Optional, optional));
    }
    label
  }

  /// What is under the control, where there is something.
  pub fn description(&self) -> Option<Row> {
    let description = self.description.as_ref()?;
    Some(self.text(FieldPart::Description, description))
  }

  /// How the default renderer builds a field: the label, the control and the
  /// description one under another, or the label beside the other two.
  pub fn render(self) -> Element {
    let label = self.label();
    let description = self.description();
    let root_part = self.part(FieldPart::Root);
    let body_part = self.part(FieldPart::Body);
    // The control keeps its own width: a row the width of the field holds it.
    let control = Row::new().child(self.control);
    let (id, key) = (self.id, self.key);
    macro_rules! named {
      ($node:expr) => {{
        let mut node = $node;
        if let Some(id) = id {
          node = node.id(id);
        }
        if let Some(key) = key {
          node = node.key(key);
        }
        node.into()
      }};
    }
    if self.horizontal {
      let mut body = Column::new()
        .flex(1.0)
        .align_items(Alignment::Stretch)
        .part(&body_part)
        .child(control);
      if let Some(description) = description {
        body = body.child(description);
      }
      named!(
        Row::new()
          .align_items(Alignment::Start)
          .part(&root_part)
          .child(label)
          .child(body)
      )
    } else {
      let mut root = Column::new()
        .align_items(Alignment::Stretch)
        .part(&root_part)
        .child(label)
        .child(control);
      if let Some(description) = description {
        root = root.child(description);
      }
      named!(root)
    }
  }
}

impl Themed for Field {
  const NAME: &'static str = "field";
  type Renderer = FieldRenderer;

  /// The design's field (`pencil/lurq-design-system.pen`, Field): 260 wide,
  /// its label above its control and its description under it, 6 apart. The
  /// label is in a field label's type (13, medium) and the foreground; the
  /// description in a caption's (12) and the muted foreground, or in the
  /// destructive, warning or success color for what the control holds. A
  /// required field's label has a mark of `*` 4 after it in the destructive
  /// color and semibold; an optional one's says "Optional" in 11 and the
  /// muted foreground. Beside its control (Fieldset), the label is 110 wide,
  /// 12 from the control, and as high as a control.
  fn recipe() -> Recipe {
    use FieldPart::{Body, Description, Label, Optional, Required, Root};
    use PaletteColor::{Destructive, Foreground, MutedForeground, Success, Warning};
    Recipe::new()
      .base(Root, |s| s.width(260.0).gap(6.0))
      .base(Label, |s| {
        s.gap(4.0)
          .typography(TypographyStyle::FieldLabel)
          .foreground(Foreground)
      })
      .base(Required, |s| {
        s.typography(TypographyStyle::FieldLabel)
          .weight(FontWeight::SemiBold)
          .foreground(Destructive)
      })
      .base(Optional, |s| {
        s.typography(TypographyStyle::Caption)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      .base(Body, |s| s.gap(6.0))
      .base(Description, |s| {
        s.typography(TypographyStyle::Caption).foreground(MutedForeground)
      })
      .state(FieldState::Invalid, Description, |s| s.foreground(Destructive))
      .state(FieldState::Warning, Description, |s| s.foreground(Warning))
      .state(FieldState::Valid, Description, |s| s.foreground(Success))
      .state(FieldState::Horizontal, Root, |s| s.width(360.0).gap(12.0))
      .state(FieldState::Horizontal, Label, |s| {
        s.width(110.0).height(ControlSize::Md)
      })
      .state(FieldState::FullWidth, Root, |s| s.width(Length::Fill))
  }
}
