//! A field: a control with what it is called above it and what it needs under it.

use std::sync::Arc;

use parking_lot::Mutex;

use super::{
  Button, ButtonGroup, Fab, Link, Toggle, names,
  part_style::{Length, PartStyle},
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{Orientation, glyph},
  text_box::{self, Judgment},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    ctx::Ctx,
    theme::{ControlSize, IconSize, PaletteColor, Theme, TypographyStyle},
  },
  components::{Column, Row, Stack, TextOverflow},
  core::{ElementRef, ElementRefMut},
  layout::{Alignment, StackAlignment, text_style::FontWeight},
  node::{Element, HitTestBehavior, dimension::Dimension},
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
  /// Whether the field is fluid and draws its label inside the control's box: said only to a control that has a
  /// fluid form ([`FieldControl::has_fluid_form`]), which then is the box, with the room of the label over what it
  /// holds.
  pub fluid: bool,
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
///
/// A control that keeps state of its own between renders, which outside a field the app mounts (a shortcut recorder
/// that records, a date field that keeps what is half typed, a picker that opens), is mounted by a field the app
/// mounts ([`Field::mount`]): the field tells it what it says, then mounts it, and it is the same control as outside
/// a field. A field converted into an element builds it as it is converted, which keeps nothing.
pub trait FieldControl {
  /// The control as the field holds it.
  fn in_field(self, field: &InField) -> Element;

  /// The control as a field mounted where it is declared holds it: told what the field says, as by
  /// [`in_field`](Self::in_field), and mounted, where it keeps state of its own. Any other control is as
  /// `in_field` gives it, which is what this does unless a control says otherwise.
  fn mount_in_field(self, field: &InField, ctx: &mut Ctx) -> Element
  where
    Self: Sized,
  {
    let _ = ctx;
    self.in_field(field)
  }

  /// Whether the control has a form for a fluid field ([`Field::fluid`]): a box as wide as the field, with the room
  /// of the field's label inside it, over what it holds ([`InField::fluid`] tells it to take it). Every text control
  /// of the design system has: an [`Input`](super::Input), a [`Textarea`](super::Textarea), a
  /// [`Select`](super::Select), a password input, a search field, a number input, a combobox, a multi select, a
  /// token input, a mentions field, a color field, the date and time fields and their pickers. A fluid field of a
  /// control that has none (a switch, a slider) is drawn as any field, its label above the control.
  fn has_fluid_form(&self) -> bool {
    false
  }
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
  use crate::node::{NodeUpdate, TOOLING, node::Node};
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
  if !TOOLING {
    return control;
  }
  control.decorate_root(move |mut node| {
    if !walk(&mut node, &description) {
      NodeUpdate::describe(&mut node, Arc::from("a11y_description"), description.clone());
    }
    node
  })
}

/// What a field holds until it is built: the control, to be finished with what the field says, and mounted where the
/// field is built with a `Ctx`.
type Unfinished = Box<dyn FnOnce(&InField, Option<&mut Ctx>) -> Element>;

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
  /// The icon before the description of an invalid field: see [`Field::error_icon`].
  Icon,
  /// The label and the description, one over the other, where the control is at the row's end.
  Text,
}

names!(FieldPart {
  Root => "root",
  Label => "label",
  Required => "required",
  Optional => "optional",
  Body => "body",
  Description => "description",
  Icon => "icon",
  Text => "text",
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
  /// Its label is above its control.
  Vertical,
  /// It is as wide as what holds it.
  FullWidth,
  /// It takes no input: its control is disabled.
  Disabled,
  /// Its label is inside its control's box, over what the control holds.
  Fluid,
  /// Its label and its description are on the left, taking the row's room, and its control at the row's end.
  ControlEnd,
}

names!(FieldState {
  Invalid => "invalid",
  Warning => "warning",
  Valid => "valid",
  Horizontal => "horizontal",
  Vertical => "vertical",
  FullWidth => "full_width",
  Disabled => "disabled",
  Fluid => "fluid",
  ControlEnd => "control_end",
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
///
/// A fluid field ([`Field::fluid`]) has its label inside the box of its control, over what the control holds: the
/// design's Fluid Field, for a form whose fields stand edge to edge.
///
/// ```
/// use lurq::{core::Signal, ui::{Field, Input}};
///
/// let workspace = Signal::new("lurq-demo".to_owned());
/// let fluid = Field::new("Workspace name", Input::new(workspace)).fluid(true);
/// ```
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
  info: Option<Arc<str>>,
  orientation: Orientation,
  full_width: bool,
  fluid: bool,
  /// Whether the control has a form for a fluid field.
  fluid_control: bool,
  error_icon: Option<Name>,
  control_end: bool,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Field {
  /// A field called `label` that holds `control`, which the field finishes: see [`FieldControl`]. An element of
  /// the engine's goes in with `.into()`. A field with an empty label draws none above its control, and gives the
  /// control no name: a checkbox that has its label beside it, with the field's description under it.
  pub fn new(label: &str, control: impl FieldControl + 'static) -> Self {
    Self {
      fluid: false,
      error_icon: None,
      control_end: false,
      fluid_control: control.has_fluid_form(),
      label: Arc::from(label),
      control: Box::new(move |field: &InField, ctx: Option<&mut Ctx>| match ctx {
        Some(ctx) => control.mount_in_field(field, ctx),
        None => control.in_field(field),
      }),
      description: None,
      error: None,
      invalid: false,
      warning: false,
      valid: false,
      disabled: false,
      required: false,
      optional: None,
      info: None,
      orientation: Orientation::Vertical,
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

  /// An icon before the description while the field is invalid, by its name in the theme's set: the mark of an
  /// error (`icons::ALERT_CIRCLE`), in the description's color. A field that is not invalid draws none.
  pub fn error_icon(mut self, name: impl Into<Name>) -> Self {
    self.error_icon = Some(name.into());
    self
  }

  /// Whether the label and the description stand on the left, one over the other, taking the room of the row, and
  /// the control at the row's end, in the middle of their height: a row of settings with its switch or its select.
  /// Its orientation is not read; a fluid field is not drawn so.
  pub fn control_end(mut self, control_end: bool) -> Self {
    self.control_end = control_end;
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
    self.optional = optional.then(|| super::current_locale().fields.optional.clone());
    self
  }

  /// Says that the field may be left empty in words of the app's own: in another language.
  pub fn optional_text(mut self, text: &str) -> Self {
    self.optional = Some(Arc::from(text));
    self
  }

  /// What the label needs explained, in a few words: the mark of an [`InfoLabel`](super::InfoLabel) after the
  /// label, an "i" in a circle, with the words in a tooltip above it. The tooltip shows on a field that is
  /// mounted ([`Field::mount`]); on one converted into an element the mark is there and says the words to
  /// tooling alone. An empty text is no mark.
  pub fn info(mut self, info: &str) -> Self {
    self.info = (!info.is_empty()).then(|| Arc::from(info));
    self
  }

  /// Where the label is: above the control (`Vertical`, the default), or beside it in a column of its own
  /// (`Horizontal`).
  pub fn orientation(mut self, orientation: Orientation) -> Self {
    self.orientation = orientation;
    self
  }

  /// Whether the field is as wide as what holds it. A control in it that is to be as wide too says so itself
  /// (`Input::full_width`).
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self
  }

  /// Whether the field is fluid: its label is inside the box of its control, over what the control holds, and the
  /// box is as wide as the field. For a control that has a fluid form ([`FieldControl::has_fluid_form`]): an
  /// [`Input`](super::Input), a [`Textarea`](super::Textarea), a [`Select`](super::Select) and every other text
  /// control of the design system. A field of any other control is drawn as it is without this. A fluid field's
  /// label is always over the value: its orientation is not read.
  pub fn fluid(mut self, fluid: bool) -> Self {
    self.fluid = fluid;
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
  /// this in the theme of what is being rendered. A control that keeps state
  /// of its own is built as it is, keeping nothing: see [`Field::mount`].
  pub fn build(self, theme: &Theme) -> Element {
    self.finish(theme, None)
  }

  /// The field built where it is declared, in the theme of what is being rendered, with its control mounted: a
  /// control that keeps state of its own between renders (a [`ShortcutRecorder`](super::ShortcutRecorder), a
  /// [`DateField`](super::DateField), a picker, a [`PasswordInput`](super::PasswordInput), a
  /// [`Rating`](super::Rating)) is the same control in the field as outside it, and the field shows its label, its
  /// description and its error. Any other control is as converting the field gives it.
  ///
  /// ```
  /// use lurq::{app::{Accelerator, ctx::Ctx}, core::Signal, node::Element, ui::{Field, ShortcutRecorder}};
  ///
  /// fn palette(ctx: &mut Ctx, shortcut: Signal<Accelerator>) -> Element {
  ///   Field::new("Command palette", ShortcutRecorder::new(shortcut))
  ///     .description("Opens the palette from anywhere in the app.")
  ///     .mount(ctx)
  /// }
  /// ```
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let theme = current_theme();
    self.finish(&theme, Some(ctx))
  }

  fn finish(self, theme: &Theme, mut ctx: Option<&mut Ctx>) -> Element {
    // One judgment is shown, as an input shows it: an error makes the field invalid.
    let invalid = self.invalid || self.error.is_some();
    let warning = self.warning && !invalid;
    let valid = self.valid && !invalid && !warning;
    let fluid = self.fluid && self.fluid_control;
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
    .state(FieldState::Horizontal, self.orientation == Orientation::Horizontal)
    .state(FieldState::Vertical, self.orientation == Orientation::Vertical)
    .state(FieldState::FullWidth, self.full_width)
    .state(FieldState::Disabled, self.disabled)
    .state(FieldState::Fluid, fluid)
    .state(FieldState::ControlEnd, self.control_end && !fluid);
    let judgment = Judgment::of(invalid, warning, valid);
    let description = self.error.or(self.description);
    let control = (self.control)(
      &InField {
        label: self.label.clone(),
        judgment,
        disabled: self.disabled,
        description: description.clone(),
        fluid,
      },
      ctx.as_deref_mut(),
    );
    let info = self
      .info
      .as_deref()
      .map(|info| super::InfoLabel::mark_alone(info, theme, ctx.as_deref_mut()));
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
      info,
      orientation: self.orientation,
      fluid,
      error_icon: self.error_icon.filter(|_| invalid),
      control_end: self.control_end && !fluid,
      recipe: theme
        .resolve::<Field>(&case, Some(&self.rules))
        .owned_by(self.id.as_ref()),
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
  /// The mark that explains the label, with its tooltip on it where the field is mounted: see [`Field::info`].
  pub info: Option<Element>,
  /// Where the label is: above the control, or beside it.
  pub orientation: Orientation,
  /// Whether the field is fluid, and its control has taken the fluid form: the label is drawn inside the control's
  /// box.
  pub fluid: bool,
  /// The icon before the description, where the field is invalid and was given one.
  pub error_icon: Option<Name>,
  /// Whether the control is at the row's end, the label and the description on the left.
  pub control_end: bool,
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
    if let Some(info) = &self.info {
      // 6 after what is before it, as an info label has its mark: 2 more than the label's own gap.
      label = label
        .overflow_visible()
        .child(Row::new().padding_left(2.0).overflow_visible().child(info.clone()));
    }
    label
  }

  /// What is under the control, where there is something: as wide as the
  /// field, a longer one going on to the next line.
  pub fn description(&self) -> Option<Row> {
    let description = self.description.as_ref()?;
    let part = self.part(FieldPart::Description);
    let Some(name) = self.error_icon else {
      return Some(
        Row::new()
          .part(&part)
          .child(part.text(description).width(Dimension::Pct(100.0))),
      );
    };
    // The icon in the middle of the words, which take the room it leaves.
    Some(
      Row::new()
        .align_items(Alignment::Center)
        .part(&part)
        .child(
          Row::new()
            .flex_shrink(0.0)
            .child(glyph(&self.part(FieldPart::Icon), name, false)),
        )
        .child(
          part
            .text(description)
            .flex_full(1.0, 1.0, Some(0.0))
            .min_width(Dimension::Px(0.0)),
        ),
    )
  }

  /// How the default renderer builds a field whose control is at the row's end: the label and the description in
  /// a column that takes the room, and the control after them, in the middle of their height.
  fn render_control_end(self) -> Element {
    let mut text = Column::new()
      .align_items(Alignment::Stretch)
      .flex_full(1.0, 1.0, Some(0.0))
      .min_width(Dimension::Px(0.0))
      .part(&self.part(FieldPart::Text));
    if !self.label.is_empty() {
      text = text.child(self.label());
    }
    if let Some(description) = self.description() {
      text = text.child(description);
    }
    let root_part = self.part(FieldPart::Root);
    // Keyed, as the row of any field's control is, so that the ref made anew in each build keeps its node.
    let control = Row::new()
      .key("control")
      .flex_shrink(0.0)
      .align_items(Alignment::Center)
      .ref_element(self.focus.clone())
      .child(self.control);
    let mut root = Row::new()
      .align_items(Alignment::Center)
      .part(&root_part)
      .child(text)
      .child(control);
    if let Some(id) = self.id {
      root = root.id(id);
    }
    if let Some(key) = self.key {
      root = root.key(key);
    }
    root.into()
  }

  /// The label of a fluid field, laid over the top of the control's box: on one line, cut with an ellipsis where
  /// the box is narrower than it, with the field's mark after it and the mark that explains it ([`Field::info`]).
  /// The pointer passes through the label and the field's mark to the control, which a press there focuses; the
  /// mark that explains it takes the pointer, for its tooltip.
  pub fn fluid_label(&self) -> Row {
    let part = self.part(FieldPart::Label);
    let mut label = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .hit_test(HitTestBehavior::ContentOnly)
      .part(&part)
      // On one line, which gives way first and is cut with an ellipsis: the marks keep their room.
      .child(
        Row::new().flex_shrink(1.0).min_width(0.0).pointer_events_none().child(
          part
            .text(&self.label)
            .nowrap()
            .text_overflow(TextOverflow::Elipsis),
        ),
      );
    if self.required {
      label = label.child(self.text(FieldPart::Required, "*").pointer_events_none());
    } else if let Some(optional) = &self.optional {
      label = label.child(self.text(FieldPart::Optional, optional).pointer_events_none());
    }
    if let Some(info) = &self.info {
      // 6 after what is before it, as in a field's label above its control.
      label = label.overflow_visible().child(
        Row::new()
          .padding_left(2.0)
          .flex_shrink(0.0)
          .overflow_visible()
          .hit_test(HitTestBehavior::ContentOnly)
          .child(info.clone()),
      );
    }
    label
  }

  /// How the default renderer builds a fluid field: the control, which is the box, with the label over its top,
  /// and the description under them.
  fn render_fluid(self) -> Element {
    let label = self.fluid_label();
    let description = self.description();
    // The stack carries a ref made anew in each build, and a key, as the row of any field's control does.
    let boxed = Stack::new()
      .key("control")
      .width(Dimension::Pct(100.0))
      .stack_align(StackAlignment::TopStart)
      .overflow_visible()
      .ref_element(self.focus.clone())
      .child(self.control);
    let boxed = if self.label.is_empty() {
      boxed
    } else {
      boxed.child(label)
    };
    let mut root = Column::new()
      .align_items(Alignment::Stretch)
      .overflow_visible()
      .part(&self.recipe.part(FieldPart::Root))
      .child(boxed);
    if let Some(description) = description {
      root = root.child(description);
    }
    if let Some(id) = self.id {
      root = root.id(id);
    }
    if let Some(key) = self.key {
      root = root.key(key);
    }
    root.into()
  }

  /// How the default renderer builds a field: the label, the control and the
  /// description one under another, or the label beside the other two. A fluid one has its label over the top of
  /// its control's box.
  pub fn render(self) -> Element {
    if self.fluid {
      return self.render_fluid();
    }
    if self.control_end {
      return self.render_control_end();
    }
    let label = self.label();
    let description = self.description();
    let root_part = self.part(FieldPart::Root);
    let body_part = self.part(FieldPart::Body);
    // A control that fills the field (`full_width`), or a component, which may fill it from inside, is held at the
    // field's width. Any other keeps its own, and the field is at least as wide as it: as wide as the recipe says,
    // or wider for a wider control, with the description as wide as the recipe leaves it. Beside the label, the
    // field is the label's column, the gap and the wider of that room and the control.
    let fills =
      matches!(self.control.node.frame.width, Some(Dimension::Pct(_))) || self.control.node.component_key().is_some();
    let px = |length: Option<Length>| match length.and_then(|length| length.resolve(self.theme())) {
      Some(Dimension::Px(px)) => Some(px),
      _ => None,
    };
    let least = px(root_part.rest().width).filter(|_| !fills);
    // The room of the label and of what is under it: the field's width, or beside the label what the label's
    // column and the gap leave of it.
    let room = least.and_then(|width| {
      if self.orientation != Orientation::Horizontal {
        return Some(width);
      }
      let label_width = px(self.part(FieldPart::Label).rest().width)?;
      let gap = root_part
        .rest()
        .gap
        .map(|gap| gap.resolve(&self.theme().spacing(), 0.0))
        .unwrap_or(0.0);
      Some((width - label_width - gap).max(0.0))
    });
    let (label, description) = match room {
      Some(width) => (
        if self.orientation == Orientation::Horizontal {
          label
        } else {
          // As wide as the field, which is no wider than the room it has.
          label.width(Dimension::Px(width)).max_width(Dimension::Pct(100.0))
        },
        description.map(|description| description.width(Dimension::Px(width)).max_width(Dimension::Pct(100.0))),
      ),
      None => (label, description),
    };
    // What the form with the label beside needs of the recipe: the label's column and its height there, and the
    // least a control that fills has beside the label: what the recipe leaves it, or the label's own column in a
    // field as wide as what holds it.
    let label_part = self.part(FieldPart::Label);
    let label_height = label_part.rest().height.and_then(|height| height.resolve(self.theme()));
    let beside_least = {
      let gap = root_part
        .rest()
        .gap
        .map(|gap| gap.resolve(&self.theme().spacing(), 0.0))
        .unwrap_or(0.0);
      let label = px(label_part.rest().width);
      px(root_part.rest().width)
        .zip(label)
        .map(|(width, label)| (width - label - gap).max(0.0))
        .or(label)
    };
    // The control keeps its own width: a row the width of the field holds it. The row carries a ref made anew in
    // each build, and a key, so that the new ref does not make it another node to the engine and its control
    // keeps the focus and what is typed in it.
    let beside = (self.orientation == Orientation::Horizontal)
      .then(|| self.part(FieldPart::Label).rest().height)
      .flatten()
      .and_then(|height| height.resolve(self.theme()));
    let mut control = Row::new()
      .key("control")
      .ref_element(self.focus.clone())
      .child(self.control);
    // Beside the label a control lower than an input stands in the middle of the label's height, level with it.
    if let Some(height) = beside {
      control = control.min_height(height).align_items(Alignment::Center);
    }
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
    if self.orientation == Orientation::Horizontal {
      // The label and what is beside it are a row that wraps: on one line where the field has room for the
      // label's column, the gap and the body, and the label over the body where it has not, in a narrow panel
      // or a window as narrow as a phone. The body is never wider than the field.
      let mut body = Column::new()
        .align_items(Alignment::Stretch)
        .part(&body_part)
        .max_width(Dimension::Pct(100.0));
      let mut root = Row::new().wrap().align_items(Alignment::Start).part(&root_part);
      match (least, room) {
        (Some(width), Some(room)) => {
          // As wide as the wider of the room the recipe leaves it and the control, and the field with it; not
          // stretched by what holds the field.
          body = body.min_width(Dimension::Px(room));
          root = root
            .width(Dimension::Auto)
            .min_width(Dimension::Px(width))
            .align(Alignment::Start);
        }
        _ => {
          // A control that fills takes the room the label leaves, and no less than the least it has beside the
          // label: under that it has the line under the label, all of it.
          body = match beside_least {
            Some(least) => body.flex_full(1.0, 1.0, Some(least)).min_width(Dimension::Px(least)),
            None => body.flex(1.0),
          };
        }
      }
      let mut body = body.child(control);
      if let Some(description) = description {
        body = body.child(description);
      }
      // Over the body the label is as high as its words: the height it has beside a control, to stand level
      // with it, is for that line. Which it is is read from where the two were laid out, before the frame is
      // painted, so no frame shows the label with the wrong height.
      let (label_at, body_at) = (ElementRefMut::new(), ElementRef::new());
      if let Some(height) = label_height {
        // Where each was laid out last, as it says itself: the label's top, and the body's.
        let tops = Arc::new(Mutex::new((None::<f32>, None::<f32>)));
        let settle = {
          let label_at = label_at.clone();
          move |tops: &(Option<f32>, Option<f32>)| {
            if let (Some(label), Some(body)) = *tops {
              label_at.set_size(None, Some(if body > label + 0.5 { Dimension::Auto } else { height }));
            }
          }
        };
        {
          let (tops, settle) = (tops.clone(), settle.clone());
          label_at.observe_rect_before_paint(move |rect| {
            let mut tops = tops.lock();
            tops.0 = Some(rect.y);
            settle(&tops);
          });
        }
        body_at.observe_rect_before_paint(move |rect| {
          let mut tops = tops.lock();
          tops.1 = Some(rect.y);
          settle(&tops);
        });
      }
      named!(
        root
          .child(label.ref_element(label_at.as_ref()))
          .child(body.ref_element(body_at))
      )
    } else {
      let mut root = Column::new().align_items(Alignment::Stretch).part(&root_part);
      if let Some(width) = room {
        // Not stretched by what holds it either: as wide as the wider of the recipe's width and the control.
        root = root
          .width(Dimension::Auto)
          .min_width(Dimension::Px(width))
          .align(Alignment::Start);
      }
      // An empty label is none: the control is at the top.
      if !self.label.is_empty() {
        root = root.child(label);
      }
      let mut root = root.child(control);
      if let Some(description) = description {
        root = root.child(description);
      }
      named!(root)
    }
  }
}

impl crate::ui::Styled for Field {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for Field {
  const NAME: &'static str = "field";
  type Renderer = FieldRenderer;

  fn parts() -> crate::ui::Declared {
    FieldPart::declared()
  }

  fn states() -> crate::ui::Declared {
    FieldState::declared()
  }

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
    use FieldPart::{Body, Description, Icon, Label, Optional, Required, Root, Text};
    use PaletteColor::{Destructive, Foreground, MutedForeground, Success, Warning};
    Recipe::new()
      .base(Root, |s| s.width(260.0).max_width(Length::Fill).gap(6.0))
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
      // The icon of an error (lurq's, the board has none) is 14, 6 before the words.
      .base(Description, |s| s.gap(6.0))
      .base(Icon, |s| s.square(IconSize::Sm))
      // The control at the row's end (lurq's): the field as wide as what holds it, the words 2 apart and 16 from
      // the control.
      .base(Text, |s| s.gap(2.0))
      .state(FieldState::ControlEnd, Root, |s| s.width(Length::Fill).gap(16.0))
      .state(FieldState::Invalid, Description, |s| s.foreground(Destructive))
      .state(FieldState::Warning, Description, |s| s.foreground(Warning))
      .state(FieldState::Valid, Description, |s| s.foreground(Success))
      .state(FieldState::Horizontal, Root, |s| s.width(360.0).gap(12.0))
      .state(FieldState::Horizontal, Label, |s| {
        s.width(110.0).height(ControlSize::Md)
      })
      .state(FieldState::FullWidth, Root, |s| s.width(Length::Fill))
      // A fluid field (`imRnW`): 240 wide, its label inside the box, 12 from its sides and 10 from its top, in a
      // caption's type (12) and the muted foreground.
      .state(FieldState::Fluid, Root, |s| s.width(240.0))
      .state(FieldState::Fluid, Label, |s| {
        s.padding_x(12.0)
          .padding_top(text_box::FLUID_PAD)
          .typography(TypographyStyle::Caption)
          .foreground(MutedForeground)
      })
      .when(
        Selector::new().state(FieldState::Fluid).state(FieldState::FullWidth),
        Root,
        |s| s.width(Length::Fill),
      )
  }
}
