//! A field: a control with what it is called above it and what it needs under it.

use std::sync::Arc;

use super::{
  Button, ButtonGroup, Fab, Link, Toggle, names,
  part_style::{Length, PartStyle},
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  text_box::Judgment,
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{ControlSize, PaletteColor, Theme, TypographyStyle},
  components::{Column, Row, Stack},
  core::ElementRef,
  layout::{Alignment, text_style::FontWeight},
  node::Element,
};

/// What a field tells the control it holds.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct InField {
  /// What the field is called: the control's name to what reads the tree, unless it has its own.
  pub label: Arc<str>,
  /// What the field says of what the control holds, where it says anything.
  pub judgment: Option<Judgment>,
  /// Whether the field takes no input.
  pub disabled: bool,
  /// What is under the control: the field's error while it has one, else its description. The field gives it to
  /// the control as its description to what reads the tree itself; a control need not.
  pub description: Option<Arc<str>>,
}

/// A control a [`Field`] holds, which the field finishes: it gives it its label as its name, its judgment and its
/// being disabled.
///
/// ```
/// use lurq::{node::Element, ui::{FieldControl, InField, Input}};
///
/// // A control of an app's own that a field names.
/// struct Code(Input);
///
/// impl FieldControl for Code {
///   fn in_field(self, field: &InField) -> Element {
///     self.0.icon(lurq::ui::icons::SEARCH).in_field(field)
///   }
/// }
/// ```
///
/// Each control of the design system that a form holds takes what it has a prop for, only where the app did not say
/// it of the control itself: an [`Input`](super::Input), a [`Textarea`](super::Textarea) and a
/// [`Select`](super::Select) the name, the judgment and disabled; a checkbox, a radio and a switch the name where
/// their own label is hidden, a checkbox's and a radio's judgment where it is invalid, and disabled; a group of
/// checkboxes or radios its name, invalid and disabled; a slider its name and disabled; a segmented control and a
/// toggle group disabled. Any [`Element`] takes none of them: an element built already cannot be told anything.
/// Whatever the control, the field gives its description, or its error, as the control's description to what
/// reads the tree.
pub trait FieldControl {
  /// The control as the field holds it.
  fn in_field(self, field: &InField) -> Element;
}

impl FieldControl for Element {
  fn in_field(self, _field: &InField) -> Element {
    self
  }
}

/// What a field holds as it is: nothing a field says is anything to it (a button, a link, a row).
macro_rules! as_they_are {
  ($($control:ty),* $(,)?) => {
    $(
      impl FieldControl for $control {
        fn in_field(self, _field: &InField) -> Element {
          self.into()
        }
      }
    )*
  };
}

as_they_are!(Button, ButtonGroup, Fab, Link, Toggle, Row, Column, Stack);

/// `control` with `description` as its description to what reads the tree: on each outermost node in it that says
/// what it is (`a11y_role`) or takes the focus, so that a group is described and not each radio in it; on the root
/// where there is none. Kept only where there is tooling to tell, as every `a11y_` attribute is.
fn described(control: Element, description: Arc<str>) -> Element {
  #[cfg(any(feature = "devtools", feature = "mcp"))]
  {
    use crate::node::{NodeUpdate, node::Node};
    fn walk(node: &mut Node, description: &Arc<str>) -> bool {
      let says = node.debug_attrs().iter().any(|(name, _)| &**name == "a11y_role");
      if says || node.is_focusable() {
        NodeUpdate::describe(node, Arc::from("a11y_description"), description.clone());
        return true;
      }
      let mut any = false;
      for child in &mut node.children {
        any |= walk(child, description);
      }
      any
    }
    control.decorate_root(move |mut node| {
      if !walk(&mut node, &description) {
        NodeUpdate::describe(&mut node, Arc::from("a11y_description"), description.clone());
      }
      node
    })
  }
  #[cfg(not(any(feature = "devtools", feature = "mcp")))]
  {
    let _ = description;
    control
  }
}

/// What a field holds until it is built: the control, to be finished with what the field says.
type Unfinished = Box<dyn FnOnce(&InField) -> Element>;

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
  /// It takes no input: its control is disabled.
  Disabled,
}

names!(FieldState {
  Invalid => "invalid",
  Warning => "warning",
  Valid => "valid",
  Horizontal => "horizontal",
  FullWidth => "full_width",
  Disabled => "disabled",
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
/// // The input is invalid because the field is: its edge is destructive.
/// let email = Field::new("Email", Input::new(Signal::new("ivan@".to_owned())).full_width(true))
///   .error("Enter a full email address.");
/// ```
///
/// Its control is a [`FieldControl`]: an [`Input`](super::Input), a
/// [`Textarea`](super::Textarea), a switch, any element. The field finishes
/// it: an input in it is called by the field's label and is invalid while
/// the field has an error, unless the app said otherwise of the input
/// itself. A press of the label focuses the control.
pub struct Field {
  label: Arc<str>,
  control: Unfinished,
  description: Option<Arc<str>>,
  error: Option<Arc<str>>,
  invalid: bool,
  warning: bool,
  valid: bool,
  disabled: bool,
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
  /// A field called `label` that holds `control`, which the field finishes: see [`FieldControl`]. An element of
  /// the engine's goes in with `.into()`.
  pub fn new(label: &str, control: impl FieldControl + 'static) -> Self {
    Self {
      label: Arc::from(label),
      control: Box::new(move |field: &InField| control.in_field(field)),
      description: None,
      error: None,
      invalid: false,
      warning: false,
      valid: false,
      disabled: false,
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

  /// Whether the field takes no input: its control is disabled, and a press of its label focuses nothing.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
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
    .state(FieldState::FullWidth, self.full_width)
    .state(FieldState::Disabled, self.disabled);
    let judgment = Judgment::of(invalid, warning, valid);
    let description = self.error.or(self.description);
    let control = (self.control)(&InField {
      label: self.label.clone(),
      judgment,
      disabled: self.disabled,
      description: description.clone(),
    });
    let control = match &description {
      Some(description) => described(control, description.clone()),
      None => control,
    };
    let view = FieldView {
      label: self.label,
      control,
      focus: ElementRef::new(),
      disabled: self.disabled,
      description,
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
  /// What the field holds, finished by the field.
  pub control: Element,
  /// What is put on the node that holds the control, and what a press of the label focuses: the first node under
  /// it that takes the focus. A renderer that holds the control another way puts it on what holds it.
  pub focus: ElementRef,
  /// Whether the field takes no input: its label focuses nothing.
  pub disabled: bool,
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

  /// The label, with the mark of a field that has to be filled in or may be left empty. A press of it focuses the
  /// control, unless the field is disabled.
  pub fn label(&self) -> Row {
    let part = self.part(FieldPart::Label);
    let mut label = Row::new()
      .align_items(Alignment::Center)
      .part(&part)
      .child(part.text(&self.label).nowrap());
    if !self.disabled {
      label = label.focuses(&self.focus);
    }
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
    // The control keeps its own width: a row the width of the field holds it. The row carries a ref made anew in
    // each build, and a key, so that the new ref does not make it another node to the engine and its control
    // keeps the focus and what is typed in it.
    let control = Row::new()
      .key("control")
      .ref_element(self.focus.clone())
      .child(self.control);
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
