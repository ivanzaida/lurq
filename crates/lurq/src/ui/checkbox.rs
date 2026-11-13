//! A checkbox: a choice that is made or not, among others a form takes at once.

use std::sync::Arc;

use super::{
  icons, names,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{fading, glyph, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{BorderSize, IconSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  components::{self, Row},
  layout::Alignment,
  node::{CursorIcon, Element, border::Border},
};

/// The parts of a checkbox a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CheckboxPart {
  /// The box and its label, side by side.
  Root,
  /// What is pressed: the square that holds the mark.
  Box,
  /// The mark in the box: a check while it is checked, a dash while it is indeterminate.
  Indicator,
  /// Its text, beside the box.
  Label,
}

names!(CheckboxPart {
  Root => "root",
  Box => "box",
  Indicator => "indicator",
  Label => "label",
});

/// The states of a checkbox a recipe styles, besides what the pointer and
/// the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CheckboxState {
  /// It is checked, and not indeterminate.
  Checked,
  /// Some of what it stands for is checked and some is not: a box over a list.
  Indeterminate,
  /// What it holds is wrong: a form says so.
  Invalid,
  /// It takes no input.
  Disabled,
}

names!(CheckboxState {
  Checked => "checked",
  Indeterminate => "indeterminate",
  Invalid => "invalid",
  Disabled => "disabled",
});

/// What builds a checkbox's nodes: a theme's in place of the default one.
pub type CheckboxRenderer = Arc<dyn Fn(CheckboxView) -> Element + Send + Sync>;

/// A checkbox: a choice that is made or not, among others a form takes at
/// once. Send crash reports, keep me signed in, a row of a list picked.
///
/// ```
/// use lurq::ui::Checkbox;
///
/// let reports = Checkbox::new("Send crash reports").checked(true).on_change(|checked| println!("{checked}"));
/// // The box over a list of which some are picked: a press of it picks them all.
/// let all = Checkbox::new("Select all").indeterminate(true);
/// // The box alone, in a row of a table: its label is still what it is called.
/// let row = Checkbox::new("Select blur.rs").label_hidden(true);
/// ```
///
/// A checkbox does not keep whether it is checked: the app says so with
/// [`Checkbox::checked`], and is told of a press with
/// [`Checkbox::on_change`]. A press of its label is a press of the checkbox.
pub struct Checkbox {
  label: Arc<str>,
  label_hidden: bool,
  checked: bool,
  indeterminate: bool,
  invalid: bool,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Checkbox {
  /// A checkbox called `label`, with the label beside it.
  pub fn new(label: &str) -> Self {
    Self {
      label: Arc::from(label),
      label_hidden: false,
      checked: false,
      indeterminate: false,
      invalid: false,
      disabled: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_change: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// Whether the checkbox is its box alone. Its label stays what it is called.
  pub fn label_hidden(mut self, hidden: bool) -> Self {
    self.label_hidden = hidden;
    self
  }

  /// Whether the checkbox is checked.
  pub fn checked(mut self, checked: bool) -> Self {
    self.checked = checked;
    self
  }

  /// Whether the checkbox is neither checked nor not: some of what it
  /// stands for is checked. It is drawn with a dash, whatever
  /// [`Checkbox::checked`] says, and a press of it asks for checked.
  pub fn indeterminate(mut self, indeterminate: bool) -> Self {
    self.indeterminate = indeterminate;
    self
  }

  /// Whether what the checkbox holds is wrong: its edge is the destructive color.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.invalid = invalid;
    self
  }

  /// Whether the checkbox takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Puts the checkbox in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Called on a press, with what the checkbox is to be: checked if it was
  /// not, or if it was indeterminate.
  pub fn on_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Styles one part of this checkbox over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this checkbox: what the
  /// component it is in says of the checkbox in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the checkbox as it is under the pointer, held down or focused
  /// from the keyboard, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the box: what is pressed.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the checkbox in `theme`. Converting a checkbox into an element
  /// does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(CheckboxState::Checked, self.checked && !self.indeterminate)
    .state(CheckboxState::Indeterminate, self.indeterminate)
    .state(CheckboxState::Invalid, self.invalid)
    .state(CheckboxState::Disabled, self.disabled);
    let view = CheckboxView {
      label: self.label,
      label_hidden: self.label_hidden,
      checked: self.checked,
      indeterminate: self.indeterminate,
      invalid: self.invalid,
      disabled: self.disabled,
      recipe: theme.resolve::<Checkbox>(&case, Some(&self.rules)),
      forced: self.forced,
      on_change: self.on_change,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Checkbox>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Checkbox> for Element {
  fn from(checkbox: Checkbox) -> Self {
    checkbox.build(&current_theme())
  }
}

/// A checkbox as a renderer sees it: what it was given, the style of its
/// parts, and its behavior ready to be dressed.
pub struct CheckboxView {
  pub label: Arc<str>,
  pub label_hidden: bool,
  pub checked: bool,
  pub indeterminate: bool,
  pub invalid: bool,
  pub disabled: bool,
  /// The style of each part of this checkbox.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  on_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl CheckboxView {
  /// The style of a part, as this checkbox is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// What a press does: every handler is told what the checkbox is to be.
  fn press(&self) -> Option<impl Fn() + Send + Sync + Clone + 'static> {
    let (handlers, next) = (self.on_change.clone(), self.indeterminate || !self.checked);
    (!handlers.is_empty() && !self.disabled).then(|| {
      move || {
        for handler in &handlers {
          handler(next);
        }
      }
    })
  }

  /// The checkbox's behavior with the style of its box: the element that is
  /// pressed, focused and disabled, with nothing in it yet. It is a checkbox
  /// to what reads the tree, called by its label, and says whether it is
  /// checked: `mixed` while it is indeterminate.
  pub fn control(&self) -> components::Button {
    let checked = match (self.indeterminate, self.checked) {
      (true, _) => "mixed",
      (false, true) => "true",
      (false, false) => "false",
    };
    let mut control = pressable(&self.part(CheckboxPart::Box), self.disabled)
      .describe("a11y_role", "checkbox")
      .describe("a11y_name", self.label.clone())
      .describe("a11y_checked", checked);
    if let Some(press) = self.press() {
      control = control.on_click(move |_| press());
    }
    if let Some(id) = &self.id {
      control = control.id(id.clone());
    }
    if let Some(key) = &self.key {
      control = control.key(key.clone());
    }
    control
  }

  /// The mark in the box in the style of its part: the theme's dash while
  /// the checkbox is indeterminate, its check while it is checked, and
  /// nothing while it is neither.
  pub fn indicator(&self) -> Option<Element> {
    let name = match (self.indeterminate, self.checked) {
      (true, _) => icons::MINUS,
      (false, true) => icons::CHECK,
      (false, false) => return None,
    };
    Some(glyph(&self.part(CheckboxPart::Indicator), Name::new(name), false))
  }

  /// How the default renderer builds a checkbox: the box with its mark, and the label beside it.
  pub fn render(self) -> Element {
    let mut control = self.control();
    if let Some(indicator) = self.indicator() {
      control = control.child(indicator);
    }
    let mut root = Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(CheckboxPart::Root))
      .child(control);
    if !self.label_hidden {
      let part = self.part(CheckboxPart::Label);
      let mut label = Row::new().part(&part).child(part.text(&self.label).nowrap());
      if let Some(press) = self.press() {
        label = label.on_click(move |_| press());
      }
      root = root.child(label);
    }
    root.into()
  }
}

/// The inner edge of 1 the box has, in `color`.
fn edge(color: PaletteColor) -> Border {
  Border::inside(BorderSize::Sm, color)
}

impl Themed for Checkbox {
  const NAME: &'static str = "checkbox";
  type Renderer = CheckboxRenderer;

  /// The design's checkbox (`pencil/lurq-design-system.pen`, Checkbox): a
  /// box of 16 with the small radius and an inner edge of 1 in the color of
  /// an input's edge, on the background. Checked or indeterminate it is
  /// filled and edged in the primary color, with a check or a dash of 12 in
  /// the color on it. An invalid one is edged in the destructive color; a
  /// disabled one is dimmed with its label, its box muted while it is empty.
  /// Its label is 8 beside it, in the body's type.
  fn recipe() -> Recipe {
    use CheckboxState::{Checked, Disabled, Indeterminate, Invalid};
    use PaletteColor::*;
    let (root, square, indicator, label) = (
      CheckboxPart::Root,
      CheckboxPart::Box,
      CheckboxPart::Indicator,
      CheckboxPart::Label,
    );
    let marked = |s: super::PartStyle| s.background(Primary).border(edge(Primary));
    Recipe::new()
      .base(root, |s| s.gap(8.0).foreground(Foreground))
      .base(square, |s| {
        fading(
          s.square(16.0)
            .radius(RadiusSize::Sm)
            .background(Background)
            .border(edge(Input))
            .cursor(CursorIcon::Pointer),
        )
      })
      .base(indicator, |s| s.square(IconSize::Xs).foreground(PrimaryForeground))
      .base(label, |s| {
        s.typography(TypographyStyle::Body).cursor(CursorIcon::Pointer)
      })
      .focus_visible(square, |s| s.focus_ring())
      .state(Checked, square, marked)
      .state(Indeterminate, square, marked)
      // After the marked box, so that an invalid one that is checked keeps the edge that says so.
      .state(Invalid, square, |s| s.border(edge(Destructive)))
      .state(Disabled, root, |s| s.opacity(OpacityLevel::Disabled))
      .state(Disabled, square, |s| s.background(Muted).cursor(CursorIcon::Default))
      .state(Disabled, label, |s| s.cursor(CursorIcon::Default))
      // A recipe has no case for a state that does not hold: a disabled box that is marked is filled again.
      .when(Selector::new().state(Checked).state(Disabled), square, |s| {
        s.background(Primary)
      })
      .when(Selector::new().state(Indeterminate).state(Disabled), square, |s| {
        s.background(Primary)
      })
  }
}
