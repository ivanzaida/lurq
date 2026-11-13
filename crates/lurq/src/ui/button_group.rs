//! A group of buttons joined into one shape.

use std::sync::Arc;

use super::{
  button::{Button, ButtonPart, ButtonSize, ButtonVariant},
  names,
  recipe::{Case, Name, Recipe},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{BorderSize, PaletteColor, RadiusSize, Theme},
  components::Row,
  layout::Alignment,
  node::{Element, border::Border},
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

/// Buttons that belong together, joined into one shape: the units of a
/// period, the steps of a zoom, an action and the choice of how to do it.
///
/// ```
/// use lurq::ui::{Button, ButtonGroup, ButtonGroupVariant, icons};
///
/// let period = ButtonGroup::new()
///   .child(Button::new("Years"))
///   .child(Button::new("Months"))
///   .child(Button::new("Days"));
/// let merge = ButtonGroup::new()
///   .variant(ButtonGroupVariant::Primary)
///   .child(Button::new("Merge"))
///   .child(Button::new("How to merge").icon(icons::CHEVRON_DOWN).icon_only(true));
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
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Default for ButtonGroup {
  fn default() -> Self {
    Self::new()
  }
}

impl ButtonGroup {
  pub fn new() -> Self {
    Self {
      items: Vec::new(),
      variant: ButtonGroupVariant::default().into(),
      size: ButtonSize::default().into(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// A [`ButtonGroupVariant`], or a variant the theme's recipe names.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// The size of every button in the group: a [`ButtonSize`].
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// A button of the group, after the ones it has.
  pub fn child(mut self, button: Button) -> Self {
    self.items.push(button);
    self
  }

  pub fn children(mut self, buttons: impl IntoIterator<Item = Button>) -> Self {
    self.items.extend(buttons);
    self
  }

  /// Styles one part of this group over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
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
    let case = Case::new()
      .variant(self.variant)
      .size(self.size)
      .breakpoint(super::current_breakpoint());
    let recipe = theme.resolve::<ButtonGroup>(&case, Some(&self.rules));
    let item = recipe.rules_for(ButtonGroupPart::Item, ButtonPart::Root);
    // In a line around the group a button has no fill of its own; in a filled group it has the group's.
    let of_button: Name = if self.variant == Name::from(ButtonGroupVariant::Outline) {
      ButtonVariant::Ghost.into()
    } else {
      self.variant
    };
    let separator = recipe.part(ButtonGroupPart::Separator);
    let separated = separator.rest().width.is_some();
    // The group is clipped to its corners: the buttons at its ends are round where it is.
    let mut root = Row::new().part(&recipe.part(ButtonGroupPart::Root))
      // The line between two of them is as high as they are.
      .align_items(Alignment::Stretch)
      .clip();
    for (index, button) in self.items.into_iter().enumerate() {
      if index > 0 && separated {
        root = root.child(Row::new().part(&separator));
      }
      root = root.child(button.variant(of_button).size(self.size).rules(&item).build(theme));
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

impl Themed for ButtonGroup {
  const NAME: &'static str = "button_group";
  type Renderer = Arc<dyn Fn(ButtonGroup) -> Element + Send + Sync>;

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
