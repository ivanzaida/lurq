//! A group of buttons joined into one shape.

use std::sync::Arc;

use super::{
  button::{Button, ButtonPart, ButtonSize, ButtonVariant},
  names,
  recipe::{Case, Interaction, Name, Recipe},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{BorderSize, PaletteColor, RadiusSize, Theme},
  components::Row,
  layout::Alignment,
  node::{Element, FocusGroup, border::Border},
};

/// How a group of buttons is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ButtonGroupVariant {
  /// One line around the group and a line between its buttons, which have no fill of their own.
  #[default]
  Outline,
  /// Buttons in the primary fill, a hair apart: the main action and what goes with it.
  Primary,
  /// Buttons in the secondary fill, a hair apart.
  Secondary,
  /// Buttons in the destructive fill, a hair apart.
  Destructive,
}

names!(ButtonGroupVariant {
  Outline => "outline",
  Primary => "primary",
  Secondary => "secondary",
  Destructive => "destructive",
});

/// The parts of a group of buttons a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ButtonGroupPart {
  /// The group: its corners, its line, the room between its buttons.
  Root,
  /// What the group says of each button in it: laid over the theme's recipe for a button's root.
  Item,
  /// The line between two buttons.
  Separator,
}

names!(ButtonGroupPart {
  Root => "root",
  Item => "item",
  Separator => "separator",
});

/// The states of a group of buttons a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ButtonGroupState {
  /// The whole group takes no input.
  Disabled,
}

names!(ButtonGroupState {
  Disabled => "disabled",
});

/// The sizes of a group's buttons: a button's.
pub type ButtonGroupSize = ButtonSize;

/// What builds a group of buttons' nodes: a theme's in place of the default one.
pub type ButtonGroupRenderer = Arc<dyn Fn(ButtonGroup) -> Element + Send + Sync>;

/// Buttons that belong together, joined into one shape: the units of a
/// period, the steps of a zoom, an action and the choice of how to do it.
///
/// ```
/// use lurq::ui::{Button, ButtonGroup, ButtonGroupVariant, icons};
///
/// let period = ButtonGroup::new([Button::new("Years"), Button::new("Months"), Button::new("Days")]).label("Period");
/// let merge = ButtonGroup::new([Button::new("Merge")])
///   .variant(ButtonGroupVariant::Primary)
///   .button(Button::new("How to merge").icon(icons::CHEVRON_DOWN).icon_only(true));
/// ```
///
/// The group gives its buttons their variant and their size, and squares
/// their corners: the corners of the group are the ones that are round. A
/// button keeps everything else it was given: its icon, what a press does,
/// whether it is disabled.
pub struct ButtonGroup {
  items: Vec<Button>,
  variant: Name,
  size: Name,
  label: Option<Arc<str>>,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ButtonGroup {
  /// A group of `buttons`, in their order.
  pub fn new(buttons: impl IntoIterator<Item = Button>) -> Self {
    Self {
      items: buttons.into_iter().collect(),
      variant: ButtonGroupVariant::default().into(),
      size: ButtonSize::default().into(),
      label: None,
      disabled: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// A [`ButtonGroupVariant`], or a variant the theme's recipe names.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// The size of every button in the group: a [`ButtonGroupSize`].
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// A button of the group, after the ones it has.
  pub fn button(mut self, button: Button) -> Self {
    self.items.push(button);
    self
  }

  /// Buttons of the group, after the ones it has.
  pub fn buttons(mut self, buttons: impl IntoIterator<Item = Button>) -> Self {
    self.items.extend(buttons);
    self
  }

  /// What the group is called to what reads the tree: what its buttons are for together.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Whether the whole group takes no input: every button in it is disabled.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Puts the group in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this group over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this group: what the component
  /// it is in says of it in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the first button the group can take as it is under the pointer,
  /// held down or focused from the keyboard, with nothing pointing at it:
  /// for a gallery or a test.
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

  /// Builds the group in `theme`. Converting a group into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let mut case = Case::new()
      .variant(self.variant.clone())
      .size(self.size.clone())
      .breakpoint(super::current_breakpoint())
      .state(ButtonGroupState::Disabled, self.disabled);
    for state in &self.states {
      case = case.state(state.clone(), true);
    }
    let recipe = theme
      .resolve::<ButtonGroup>(&case, Some(&self.rules))
      .owned_by(self.id.as_ref());
    // The group pads its buttons across; a button that is an icon alone keeps its square, with its icon in the
    // middle, as it is out of a group.
    let item = recipe.rules_for(ButtonGroupPart::Item, ButtonPart::Root).state(
      super::ButtonState::IconOnly,
      ButtonPart::Root,
      |s| s.padding_x(0.0),
    );
    // In a line around the group a button has no fill of its own; in a filled group it has the group's.
    let of_button: Name = if self.variant == Name::from(ButtonGroupVariant::Outline) {
      ButtonVariant::Ghost.into()
    } else {
      self.variant
    };
    let separator = recipe.part(ButtonGroupPart::Separator);
    let separated = separator.rest().width.is_some();
    // The group is clipped to its corners: the buttons at its ends are round where it is.
    // Each button is its own stop of Tab, as each is its own action; the arrow keys move between them too.
    let mut root = Row::new().part(&recipe.part(ButtonGroupPart::Root))
      .focus_group(FocusGroup::horizontal())
      // The line between two of them is as high as they are.
      .align_items(Alignment::Stretch)
      .clip()
      .describe("a11y_role", "group");
    if let Some(label) = &self.label {
      root = root.describe("a11y_name", label.clone());
    }
    for (index, mut button) in self.items.into_iter().enumerate() {
      if index > 0 && separated {
        root = root.child(Row::new().part(&separator));
      }
      if self.disabled {
        button = button.disabled(true);
      }
      if index == 0 && self.forced.is_some() {
        button = button.force(self.forced);
      }
      root = root.child(
        button
          .variant(of_button.clone())
          .size(self.size.clone())
          .rules(&item)
          .build(theme),
      );
    }
    if let Some(id) = self.id {
      root = root.id(id);
    }
    if let Some(key) = self.key {
      root = root.key(key);
    }
    root.into()
  }
}

impl From<ButtonGroup> for Element {
  fn from(group: ButtonGroup) -> Self {
    group.build(&current_theme())
  }
}

impl crate::ui::Styled for ButtonGroup {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for ButtonGroup {
  const NAME: &'static str = "button_group";
  type Renderer = ButtonGroupRenderer;

  fn parts() -> crate::ui::Declared {
    ButtonGroupPart::declared()
  }

  fn states() -> crate::ui::Declared {
    ButtonGroupState::declared()
  }

  /// The design's group (`pencil/lurq-design-system.pen`, Button Group and
  /// Split Button): a line around the buttons and one between them, or
  /// filled buttons a pixel apart; the corners of a control; a button padded 14.
  fn recipe() -> Recipe {
    use ButtonGroupPart::{Item, Root, Separator};
    use PaletteColor::{Background, Input};
    let filled = |recipe: Recipe, variant: ButtonGroupVariant| recipe.variant(variant, Root, |s| s.gap(1.0));
    let recipe = Recipe::new()
      .base(Root, |s| s.radius(RadiusSize::Md))
      .base(Item, |s| s.radius(0.0).padding_x(14.0))
      .variant(ButtonGroupVariant::Outline, Root, |s| {
        s.background(Background).border(Border::inside(BorderSize::Sm, Input))
      })
      .variant(ButtonGroupVariant::Outline, Separator, |s| {
        s.width(1.0).background(Input)
      });
    let recipe = filled(recipe, ButtonGroupVariant::Primary);
    let recipe = filled(recipe, ButtonGroupVariant::Secondary);
    filled(recipe, ButtonGroupVariant::Destructive)
  }
}
