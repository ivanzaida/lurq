//! A toggle: a button that stays on.

use std::sync::Arc;

use super::{
  button::ButtonSize,
  names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{fading, glyph, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{ControlSize, Density, IconSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  node::Element,
};

/// The parts of a toggle a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TogglePart {
  /// The toggle itself: what is pressed.
  Root,
  /// Its text.
  Label,
  /// Its icon: its side, and a color of its own.
  Icon,
}

names!(TogglePart {
  Root => "root",
  Label => "label",
  Icon => "icon",
});

/// The states of a toggle a recipe styles, besides what the pointer and the
/// keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToggleState {
  /// It is on.
  On,
  /// It takes no input.
  Disabled,
  /// It is an icon alone, as wide as it is high.
  IconOnly,
}

names!(ToggleState {
  On => "on",
  Disabled => "disabled",
  IconOnly => "icon_only",
});

/// What builds a toggle's nodes: a theme's in place of the default one.
pub type ToggleRenderer = Arc<dyn Fn(ToggleView) -> Element + Send + Sync>;

/// A toggle: a button that stays on until it is pressed again. Bold in a
/// text editor, a filter that holds, a panel that is shown.
///
/// ```
/// use lurq::ui::{Toggle, icons};
///
/// let bold = Toggle::new("Bold").icon(icons::BOLD).on(true).on_change(|on| println!("bold: {on}"));
/// // An icon alone: its label is still what it is called.
/// let italic = Toggle::new("Italic").icon(icons::ITALIC).icon_only(true);
/// ```
///
/// A toggle does not keep whether it is on: the app says so with
/// [`Toggle::on`], and is told of a press with [`Toggle::on_change`]. Its
/// sizes are a button's ([`ButtonSize`]).
pub struct Toggle {
  label: Arc<str>,
  size: Name,
  icon: Option<Name>,
  icon_only: bool,
  on: bool,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Toggle {
  pub fn new(label: &str) -> Self {
    Self {
      label: Arc::from(label),
      size: ButtonSize::default().into(),
      icon: None,
      icon_only: false,
      on: false,
      disabled: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_change: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// A [`ButtonSize`], or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// An icon before the label, by its name in the theme's set.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// Whether the toggle is its icon alone: a square as wide as it is high.
  pub fn icon_only(mut self, icon_only: bool) -> Self {
    self.icon_only = icon_only;
    self
  }

  /// Whether the toggle is on.
  pub fn on(mut self, on: bool) -> Self {
    self.on = on;
    self
  }

  /// Whether the toggle takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Puts the toggle in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Called on a press, with what the toggle is to be: on if it was off.
  pub fn on_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Styles one part of this toggle over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this toggle: what the component
  /// it is in says of the toggle in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the toggle as it is under the pointer, held down or focused from
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

  /// What an item of a menu that stands for this toggle takes from it, checked
  /// while the toggle is on: a toolbar moves a toggle that does not fit into
  /// its menu with this. Choosing it turns the toggle, as a press would.
  pub(crate) fn command(&self) -> super::toolbar::ToolbarCommand {
    let (on, handlers) = (self.on, self.on_change.clone());
    super::toolbar::ToolbarCommand {
      label: self.label.clone(),
      icon: self.icon,
      icon_only: self.icon_only,
      disabled: self.disabled,
      checked: Some(on),
      run: Arc::new(move || {
        for handler in &handlers {
          handler(!on);
        }
      }),
      id: self.id.clone(),
    }
  }

  /// Builds the toggle in `theme`. Converting a toggle into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: Some(self.size),
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(ToggleState::On, self.on)
    .state(ToggleState::Disabled, self.disabled)
    .state(ToggleState::IconOnly, self.icon_only);
    let view = ToggleView {
      label: self.label,
      size: self.size,
      icon: self.icon,
      icon_only: self.icon_only,
      on: self.on,
      disabled: self.disabled,
      recipe: theme.resolve::<Toggle>(&case, Some(&self.rules)),
      forced: self.forced,
      on_change: self.on_change,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Toggle>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Toggle> for Element {
  fn from(toggle: Toggle) -> Self {
    toggle.build(&current_theme())
  }
}

/// A toggle as a renderer sees it: what it was given, the style of its
/// parts, and its behavior ready to be dressed.
pub struct ToggleView {
  pub label: Arc<str>,
  pub size: Name,
  /// The name of its icon, if it has one.
  pub icon: Option<Name>,
  pub icon_only: bool,
  pub on: bool,
  pub disabled: bool,
  /// The style of each part of this toggle.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  on_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ToggleView {
  /// The style of a part, as this toggle is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The toggle's behavior with the style of its root: the element that is
  /// pressed, focused and disabled, with nothing in it yet.
  pub fn root(&self) -> crate::components::Button {
    let mut root = pressable(&self.part(TogglePart::Root), self.disabled);
    let (handlers, next) = (self.on_change.clone(), !self.on);
    if !handlers.is_empty() {
      root = root.on_click(move |_| {
        for handler in &handlers {
          handler(next);
        }
      });
    }
    if self.icon_only {
      root = root.describe("a11y_name", self.label.clone());
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// How the default renderer builds a toggle: the root, with its icon and its label.
  pub fn render(self) -> Element {
    let mut root = self.root();
    if let Some(name) = self.icon {
      root = root.child(glyph(&self.part(TogglePart::Icon), name, false));
    }
    if !self.icon_only {
      let part = self.part(TogglePart::Label);
      root = root.child(part.text(&self.label).nowrap().part(&part));
    }
    root.into()
  }
}

impl Themed for Toggle {
  const NAME: &'static str = "toggle";
  type Renderer = ToggleRenderer;

  /// The design's toggle (`pencil/lurq-design-system.pen`, Toggle): a
  /// button's height, padded 10, with no fill until the pointer is on it or
  /// it is on, and then the fill of what is picked out.
  fn recipe() -> Recipe {
    use PaletteColor::*;
    use TogglePart::{Icon, Label, Root};
    let picked_out = |s: super::PartStyle| s.background(Highlight).foreground(HighlightForeground);
    let recipe = Recipe::new()
      .base(Root, |s| {
        fading(
          s.height(ControlSize::Md)
            .padding_x(10.0)
            .gap(8.0)
            .radius(RadiusSize::Md)
            .foreground(Foreground),
        )
      })
      .base(Label, |s| s.typography(TypographyStyle::Button))
      .base(Icon, |s| s.square(IconSize::Md))
      .hovered(Root, picked_out)
      .pressed(Root, |s| s.veil(StatePressed))
      .focus_visible(Root, |s| s.focus_ring())
      .size(ButtonSize::Xs, Root, |s| s.height(Density::Compact).padding_x(8.0))
      .size(ButtonSize::Xs, Label, |s| s.font_size(12.0))
      .size(ButtonSize::Sm, Root, |s| s.height(ControlSize::Sm).padding_x(8.0))
      .size(ButtonSize::Lg, Root, |s| s.height(ControlSize::Lg).padding_x(12.0))
      .size(ButtonSize::Xl, Root, |s| s.height(Density::Comfortable).padding_x(14.0))
      .size(ButtonSize::Xl, Label, |s| s.font_size(15.0))
      .state(ToggleState::On, Root, picked_out)
      // A toggle that is on has its fill already: under the pointer it is a step darker.
      .when(Selector::new().state(ToggleState::On).hovered(), Root, |s| {
        s.background(SecondaryHover)
      })
      .state(ToggleState::Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
      .state(ToggleState::IconOnly, Root, |s| s.padding_x(0.0).width(ControlSize::Md));
    let sized = |recipe: Recipe, size: ButtonSize, icon: Length, side: Length| {
      recipe.size(size, Icon, |s| s.square(icon)).when(
        Selector::new().size(size).state(ToggleState::IconOnly),
        Root,
        |s| s.width(side),
      )
    };
    let recipe = sized(recipe, ButtonSize::Xs, IconSize::Sm.into(), Density::Compact.into());
    let recipe = sized(recipe, ButtonSize::Sm, IconSize::Md.into(), ControlSize::Sm.into());
    let recipe = sized(recipe, ButtonSize::Lg, Length::Px(18.0), ControlSize::Lg.into());
    sized(recipe, ButtonSize::Xl, IconSize::Lg.into(), Density::Comfortable.into())
  }
}
