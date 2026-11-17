//! A floating action button: the one action of a view, raised over it.

use std::sync::Arc;

use super::{
  button::{Button, ButtonPart, ButtonShape, ButtonVariant},
  names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{Density, IconSize, ShadowStyle, Theme},
  node::{Element, EventHandler, IntoMouseEventHandler},
};

/// The parts of a floating action button a recipe styles. Each is laid over
/// the theme's recipe for the same part of a button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FabPart {
  /// The button itself: its size, its shadow.
  Root,
  /// Its icon.
  Icon,
  /// Its text, where it has room for one.
  Label,
}

names!(FabPart {
  Root => "root",
  Icon => "icon",
  Label => "label",
});

/// The states of a floating action button a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FabState {
  /// It shows its label beside its icon.
  Extended,
  /// It takes no input. A button's own rule dims it.
  Disabled,
  /// What it started is under way. A button's own rule turns its spinner.
  Loading,
}

names!(FabState {
  Extended => "extended",
  Disabled => "disabled",
  Loading => "loading",
});

/// What builds a floating action button's nodes: a theme's in place of the default one.
pub type FabRenderer = Arc<dyn Fn(Fab) -> Element + Send + Sync>;

/// A floating action button: the one thing a view is for, raised over it.
/// Compose in a mail app, add in a list.
///
/// ```
/// use lurq::ui::{Fab, icons};
///
/// let compose = Fab::new("Compose", icons::COMPOSE).on_click(|_| println!("a new message"));
/// // With its label beside its icon.
/// let wide = Fab::new("Compose", icons::COMPOSE).extended(true);
/// ```
///
/// It is a [`Button`] that is an icon alone, round, larger than a control
/// and with a shadow, and takes what a button takes: a variant, what a press
/// does, whether it is disabled or loading. Where it floats is the app's to
/// say: it is an element like any other, put in a corner of a `Stack`.
pub struct Fab {
  label: Arc<str>,
  icon: Name,
  variant: Name,
  extended: bool,
  disabled: bool,
  loading: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_click: Vec<EventHandler<crate::app::events::MouseEvent>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Fab {
  /// A floating action button with `icon`, an icon of the theme's set, called `label`.
  pub fn new(label: &str, icon: impl Into<Name>) -> Self {
    Self {
      label: Arc::from(label),
      icon: icon.into(),
      variant: ButtonVariant::Primary.into(),
      extended: false,
      disabled: false,
      loading: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_click: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// A [`ButtonVariant`], or a variant the theme's recipe for a button names.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// Whether the button shows its label beside its icon.
  pub fn extended(mut self, extended: bool) -> Self {
    self.extended = extended;
    self
  }

  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Whether what the button started is under way: see [`Button::loading`].
  pub fn loading(mut self, loading: bool) -> Self {
    self.loading = loading;
    self
  }

  /// Called on a click, and on Enter and Space while the button is focused.
  pub fn on_click(mut self, handler: impl IntoMouseEventHandler) -> Self {
    self.on_click.push(handler.into_event_handler());
    self
  }

  /// Puts the button in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Lays rules over the theme's recipe for this button: what the component
  /// it is in says of it in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Styles one part of this button over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Draws the button as it is under the pointer, held down or focused from
  /// the keyboard, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the button in `theme`. Converting it into an element does this
  /// in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let mut case = Case::new()
      .variant(self.variant)
      .breakpoint(super::current_breakpoint())
      .state(FabState::Extended, self.extended)
      .state(FabState::Disabled, self.disabled)
      .state(FabState::Loading, self.loading);
    for state in &self.states {
      case = case.state(state.clone(), true);
    }
    let recipe = theme.resolve::<Fab>(&case, Some(&self.rules));
    // What the recipe says of each part is laid over the theme's for that part of a button.
    let rules = recipe
      .rules_for(FabPart::Root, ButtonPart::Root)
      .extend(&recipe.rules_for(FabPart::Icon, ButtonPart::Icon))
      .extend(&recipe.rules_for(FabPart::Icon, ButtonPart::Spinner))
      .extend(&recipe.rules_for(FabPart::Label, ButtonPart::Label));
    let mut button = Button::new(&self.label)
      .variant(self.variant)
      .icon(self.icon)
      .icon_only(!self.extended)
      .shape(ButtonShape::Pill)
      .disabled(self.disabled)
      .loading(self.loading)
      .force(self.forced)
      .rules(&rules);
    for handler in self.on_click {
      button = button.on_click(handler);
    }
    if let Some(id) = self.id {
      button = button.id(id);
    }
    if let Some(key) = self.key {
      button = button.key(key);
    }
    button.build(theme)
  }
}

impl From<Fab> for Element {
  fn from(fab: Fab) -> Self {
    fab.build(&current_theme())
  }
}

impl crate::ui::Styled for Fab {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for Fab {
  const NAME: &'static str = "fab";
  type Renderer = FabRenderer;

  fn parts() -> crate::ui::Declared {
    FabPart::declared()
  }

  fn states() -> crate::ui::Declared {
    FabState::declared()
  }

  /// The design's floating action button (`pencil/lurq-design-system.pen`,
  /// FAB): a circle of 48 with an icon of 20 and a shadow under it.
  fn recipe() -> Recipe {
    use FabPart::{Icon, Label, Root};
    Recipe::new()
      .base(Root, |s| s.square(Density::Touch).shadow(ShadowStyle::Lg))
      .base(Icon, |s| s.square(IconSize::Lg))
      .state(FabState::Extended, Root, |s| {
        s.width(Length::Auto).padding_x(20.0).gap(8.0)
      })
      .state(FabState::Extended, Label, |s| s.font_size(14.0))
  }
}
