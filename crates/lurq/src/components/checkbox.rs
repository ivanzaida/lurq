// The type is deprecated for apps; the engine's checkbox node it builds stays, and the form's checkbox still uses it.
#![allow(deprecated)]

use crate::{core::Signal, impl_into_node, node::CheckboxStyle};

impl_into_node!(
  /// The engine's checkbox node: a box bound to a signal, with styles of its own for each state and no label.
  #[deprecated(
    since = "0.63.0",
    note = "use `lurq::ui::Checkbox`: the design's checkbox, with its label, an indeterminate state, `bind(signal)` \
            and a recipe in the theme"
  )]
  Checkbox
);

impl Checkbox {
  pub fn new(value: Signal<bool>) -> Self {
    Self::from_node(crate::node::Node::checkbox(value))
  }

  pub fn box_style(mut self, style: CheckboxStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::checkbox_box_style(node, style));
    self
  }

  pub fn box_part(mut self, f: impl FnOnce(CheckboxStyle) -> CheckboxStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::checkbox_box_style(node, f(CheckboxStyle::new())));
    self
  }

  pub fn checked_box_style(mut self, style: CheckboxStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::checkbox_checked_box_style(node, style));
    self
  }

  pub fn checked_box(mut self, f: impl FnOnce(CheckboxStyle) -> CheckboxStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::checkbox_checked_box_style(node, f(CheckboxStyle::new())));
    self
  }

  pub fn box_hovered_style(mut self, style: CheckboxStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::checkbox_box_hovered_style(node, style));
    self
  }

  pub fn box_hovered(mut self, f: impl FnOnce(CheckboxStyle) -> CheckboxStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::checkbox_box_hovered_style(node, f(CheckboxStyle::new())));
    self
  }

  pub fn checked_box_hovered_style(mut self, style: CheckboxStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::checkbox_checked_box_hovered_style(node, style));
    self
  }

  pub fn checked_box_hovered(mut self, f: impl FnOnce(CheckboxStyle) -> CheckboxStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::checkbox_checked_box_hovered_style(node, f(CheckboxStyle::new())));
    self
  }

  /// Box style while the checkbox has keyboard focus (focus-visible, like a
  /// browser's checkbox ring: a click does not show it), layered over the
  /// checked or unchecked box and under the hovered styles. Size is ignored:
  /// focus changes paint, never layout. For a style on any focus, use the
  /// element's `focused_style`.
  pub fn box_focused_style(mut self, style: CheckboxStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::checkbox_box_focused_style(node, style));
    self
  }

  pub fn box_focused(mut self, f: impl FnOnce(CheckboxStyle) -> CheckboxStyle) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::checkbox_box_focused_style(node, f(CheckboxStyle::new())));
    self
  }
}

/// What the engine's checkbox node fills its box with where its style sets no fill comes from the theme, through
/// this recipe, by the parts and states of [`ui::Checkbox`](crate::ui::Checkbox): `CheckboxPart::Box`'s fill, and
/// its fill in `CheckboxState::Checked`.
impl crate::ui::Themed for Checkbox {
  const NAME: &'static str = "engine_checkbox";
  type Renderer = ();

  /// The design's roles for 0.62's look: the box on the background, on the primary color while it is checked.
  fn recipe() -> crate::ui::Recipe {
    use crate::app::theme::PaletteColor::{Background, Primary};
    fills(Background, Primary)
  }
}

fn fills(blank: crate::app::theme::PaletteColor, checked: crate::app::theme::PaletteColor) -> crate::ui::Recipe {
  use crate::ui::{CheckboxPart, CheckboxState, Recipe};
  Recipe::new()
    .base(CheckboxPart::Box, |s| s.background(blank))
    .state(CheckboxState::Checked, CheckboxPart::Box, |s| s.background(checked))
}

impl Checkbox {
  /// 0.62's roles: the recipe [`UiDefinition::legacy`](crate::ui::UiDefinition::legacy) gives the checkbox node.
  pub(crate) fn legacy_recipe() -> crate::ui::Recipe {
    use crate::layout::layout_engine::control_fill::legacy;
    fills(legacy::BOX, legacy::CHECKED)
  }
}

impl Default for Checkbox {
  fn default() -> Self {
    Self::new(Signal::new(false))
  }
}
