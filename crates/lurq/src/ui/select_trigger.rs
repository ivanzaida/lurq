//! The trigger of a select that opens a panel of its own in a popover, and whether that panel is open: what a tree
//! select and a cascader share. Its parts are named as a tree select's are: `trigger`, `value`, `placeholder`,
//! `chevron`, `clear` and `clear_icon`; a recipe that starts from a select's has them.

use std::sync::Arc;

use super::{
  combobox, icons,
  recipe::{Interaction, Name, ResolvedPart, ResolvedRecipe},
  shared::{glyph, pressable},
  tree_select::TreeSelectPart,
};
use crate::{
  app::{
    events::{KeyboardEvent, MouseEvent},
    theme::Theme,
  },
  components::{self, Row},
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::Justify},
  node::{Element, node_kind::TextOverflow},
};

/// Whether the panel is open, and who hears it open and close.
pub(crate) struct OpenModel {
  pub open: Signal<bool>,
  pub on_open_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  pub on_dismiss: Vec<Arc<dyn Fn() + Send + Sync>>,
  /// Whether the panel does not open: the select is disabled or read only.
  pub inert: bool,
}

impl OpenModel {
  /// Opens or closes the panel, and tells the handlers where it changed.
  pub fn set_open(&self, open: bool) {
    if self.open.get_untracked() == open || (open && self.inert) {
      return;
    }
    self.open.set(open);
    for handler in &self.on_open_change {
      handler(open);
    }
  }

  /// What Escape or a press outside does after the popover closed: tells the handlers.
  pub fn dismissed(&self) {
    for handler in &self.on_dismiss {
      handler();
    }
    for handler in &self.on_open_change {
      handler(false);
    }
  }
}

/// The trigger as it is to be drawn: what it says, its states, and what opens its panel.
pub(crate) struct SelectTrigger<'a> {
  pub recipe: &'a ResolvedRecipe,
  pub forced: Option<Interaction>,
  /// What it says of what is chosen: `None` while nothing is.
  pub text: Option<&'a Arc<str>>,
  pub placeholder: Option<&'a Arc<str>>,
  pub label: Option<&'a Arc<str>>,
  pub open: bool,
  pub invalid: bool,
  pub read_only: bool,
  pub disabled: bool,
  pub multiple: bool,
  /// What the panel is to what reads the tree: `tree`, `listbox`.
  pub popup: &'static str,
  pub model: &'a Arc<OpenModel>,
  pub anchor: &'a ElementRef,
  // Read only where the engine has forms.
  #[cfg_attr(not(feature = "form"), allow(dead_code))]
  pub name: Option<&'a Arc<str>>,
  pub id: Option<&'a Arc<str>>,
}

impl SelectTrigger<'_> {
  fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// What the trigger says, in the room the chevron leaves it: what is chosen, else the placeholder. A longer
  /// text ends in an ellipsis.
  pub fn value(&self) -> Row {
    // The type of the trigger, in the size it is, with the value's laid over it and the placeholder's over that.
    let mut style = self
      .part(TreeSelectPart::Trigger)
      .rest()
      .merged(&self.part(TreeSelectPart::Value).rest());
    if self.text.is_none() {
      style = style.merged(&self.part(TreeSelectPart::Placeholder).rest());
    }
    let said = self
      .text
      .map(|text| &**text)
      .or(self.placeholder.map(|text| &**text))
      .unwrap_or("");
    Row::new().flex_full(1.0, 1.0, Some(0.0)).min_width(0.0).child(
      components::Text::styled(said, super::text_box::ink(self.theme(), &style))
        .nowrap()
        .text_overflow(TextOverflow::Elipsis),
    )
  }

  /// The cross that runs `clear`, where there is one: pressed apart from the trigger, and called `label` to what
  /// reads the tree.
  pub fn clear(&self, clear: Option<Arc<dyn Fn() + Send + Sync>>, label: &Arc<str>) -> Option<Element> {
    let clear = clear?;
    let icon = self.part(TreeSelectPart::ClearIcon);
    let mut control = pressable(&self.part(TreeSelectPart::Clear), false)
      .describe("a11y_role", "button")
      .describe("a11y_name", label.clone())
      .flex_shrink(0.0)
      .child(glyph(&icon, Name::new(icons::CLOSE), false))
      .on_click(move |event: MouseEvent| {
        // The press is the cross's: it does not open the panel as well.
        event.stop_propagation();
        clear();
      });
    if let Some(id) = self.id {
      control = control.id(format!("{id}-clear"));
    }
    Some(combobox::apart(control.into()))
  }

  /// The glyph at the end of the trigger: a chevron down.
  pub fn chevron(&self) -> Element {
    Row::new()
      .flex_shrink(0.0)
      .pointer_events_none()
      .child(glyph(
        &self.part(TreeSelectPart::Chevron),
        Name::new(icons::CHEVRON_DOWN),
        false,
      ))
      .into()
  }

  /// The trigger in the style of its part, with nothing in it yet: what is pressed and focused, and what the panel
  /// hangs from. A press, Enter and Space open and close the panel, and Down and Up open it. It is a combobox that
  /// opens its `popup` to what reads the tree, and says whether it is open.
  pub fn trigger(&self) -> components::Button {
    let flag = |on: bool| if on { "true" } else { "false" };
    let (press, keys) = (self.model.clone(), self.model.clone());
    let mut trigger = pressable(&self.part(TreeSelectPart::Trigger), self.disabled)
      .justify(Justify::Start)
      .align_items(Alignment::Center)
      .ref_element(self.anchor.clone())
      .describe("a11y_role", "combobox")
      .describe("a11y_haspopup", self.popup)
      .describe("a11y_expanded", flag(self.open))
      .describe("a11y_invalid", flag(self.invalid))
      .on_click(move |_: MouseEvent| press.set_open(!press.open.get_untracked()))
      .on_key_down(move |event: KeyboardEvent| {
        let plain = !(event.ctrl || event.alt || event.meta || event.shift || event.composing);
        if event.focus_within && plain && matches!(event.key.as_str(), "ArrowDown" | "ArrowUp") {
          event.prevent_default();
          keys.set_open(true);
        }
      });
    if let Some(label) = self.label {
      trigger = trigger.describe("a11y_name", label.clone());
    }
    if let Some(text) = self.text {
      trigger = trigger.describe("a11y_value", text.clone());
    }
    if self.read_only {
      trigger = trigger.describe("a11y_readonly", "true");
    }
    if self.multiple {
      trigger = trigger.describe("a11y_multiselectable", "true");
    }
    #[cfg(feature = "form")]
    if let Some(name) = self.name {
      trigger = trigger.name(name.clone()).form_value(self.text.cloned());
    }
    if let Some(id) = self.id {
      trigger = trigger.id(id.clone());
    }
    trigger
  }
}
