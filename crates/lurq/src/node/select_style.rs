use crate::{
  layout::text_style::TextStyle,
  node::{
    BackgroundColor,
    border::{Border, BorderRadius, Borders, ThemedBorderRadius},
    border_size_value::BorderSizeValue,
    color::Color,
    padding::Padding,
    radius_value::RadiusValue,
  },
};

/// Visual style for one box-like part of a `Select` (the trigger, the menu
/// container, or an option row). Every field is optional so state overrides
/// (hovered/focused/open/selected) can merge on top of a base part.
#[derive(Clone, Default)]
pub struct SelectPartStyle {
  pub(crate) background: Option<BackgroundColor>,
  pub(crate) border: Option<Borders>,
  pub(crate) border_radius: Option<ThemedBorderRadius>,
  pub(crate) padding: Option<Padding>,
  pub(crate) text: Option<TextStyle>,
  pub(crate) min_width: Option<f32>,
  pub(crate) min_height: Option<f32>,
}

impl SelectPartStyle {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn background(mut self, color: impl Into<BackgroundColor>) -> Self {
    self.background = Some(color.into());
    self
  }

  pub fn rounded(mut self, radius: impl Into<RadiusValue>) -> Self {
    self.border_radius = Some(ThemedBorderRadius::all(radius));
    self
  }

  pub fn corner_radius_custom(mut self, radius: BorderRadius) -> Self {
    self.border_radius = Some(radius.into());
    self
  }

  pub fn border_inside(mut self, width: impl Into<BorderSizeValue>, color: impl Into<BackgroundColor>) -> Self {
    self.border = Some(Borders::all(Border::inside(width, color)));
    self
  }

  pub fn border_outside(mut self, width: impl Into<BorderSizeValue>, color: impl Into<BackgroundColor>) -> Self {
    self.border = Some(Borders::all(Border::outside(width, color)));
    self
  }

  pub fn border_center(mut self, width: impl Into<BorderSizeValue>, color: impl Into<BackgroundColor>) -> Self {
    self.border = Some(Borders::all(Border::center(width, color)));
    self
  }

  pub fn border_custom(mut self, border: Borders) -> Self {
    self.border = Some(border);
    self
  }

  pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
    self.padding = Some(padding.into());
    self
  }

  pub fn text(mut self, text: TextStyle) -> Self {
    self.text = Some(text);
    self
  }

  pub fn min_width(mut self, width: f32) -> Self {
    self.min_width = Some(width);
    self
  }

  pub fn min_height(mut self, height: f32) -> Self {
    self.min_height = Some(height);
    self
  }

  pub(crate) fn merge_from(&mut self, other: &Self) {
    if other.background.is_some() {
      self.background = other.background.clone();
    }
    if other.border.is_some() {
      self.border = other.border.clone();
    }
    if other.border_radius.is_some() {
      self.border_radius = other.border_radius;
    }
    if other.padding.is_some() {
      self.padding = other.padding.clone();
    }
    if other.text.is_some() {
      self.text = other.text.clone();
    }
    if other.min_width.is_some() {
      self.min_width = other.min_width;
    }
    if other.min_height.is_some() {
      self.min_height = other.min_height;
    }
  }
}

/// Full styling surface for a `Select`. Base parts are always present; the
/// state-specific parts (`*_hovered`, `*_focused`, `trigger_open`,
/// `option_selected`) merge onto their base when active.
#[derive(Clone)]
pub struct SelectStyle {
  pub(crate) trigger: SelectPartStyle,
  pub(crate) trigger_hovered: Option<SelectPartStyle>,
  pub(crate) trigger_focused: Option<SelectPartStyle>,
  pub(crate) trigger_open: Option<SelectPartStyle>,
  pub(crate) placeholder_text: Option<TextStyle>,
  pub(crate) menu: SelectPartStyle,
  pub(crate) option: SelectPartStyle,
  pub(crate) option_hovered: Option<SelectPartStyle>,
  pub(crate) option_selected: Option<SelectPartStyle>,
  pub(crate) option_selected_hovered: Option<SelectPartStyle>,
  pub(crate) chevron_color: Option<Color>,
  pub(crate) chevron_size: f32,
  pub(crate) checkmark_color: Option<Color>,
  pub(crate) max_menu_height: f32,
  pub(crate) menu_gap: f32,
}

impl Default for SelectStyle {
  fn default() -> Self {
    Self {
      trigger: SelectPartStyle::default(),
      trigger_hovered: None,
      trigger_focused: None,
      trigger_open: None,
      placeholder_text: None,
      menu: SelectPartStyle::default(),
      option: SelectPartStyle::default(),
      option_hovered: None,
      option_selected: None,
      option_selected_hovered: None,
      chevron_color: None,
      chevron_size: 10.0,
      checkmark_color: None,
      max_menu_height: 240.0,
      menu_gap: 4.0,
    }
  }
}

impl SelectStyle {
  pub fn new() -> Self {
    Self::default()
  }

  /// The trigger part resolved for the current interaction/open state.
  pub(crate) fn resolved_trigger(&self, hovered: bool, focused: bool, open: bool) -> SelectPartStyle {
    let mut style = self.trigger.clone();
    if hovered && let Some(part) = &self.trigger_hovered {
      style.merge_from(part);
    }
    if focused && let Some(part) = &self.trigger_focused {
      style.merge_from(part);
    }
    if open && let Some(part) = &self.trigger_open {
      style.merge_from(part);
    }
    style
  }

  /// An option row resolved for the current hover/selected state.
  pub(crate) fn resolved_option(&self, hovered: bool, selected: bool) -> SelectPartStyle {
    let mut style = self.option.clone();
    if selected && let Some(part) = &self.option_selected {
      style.merge_from(part);
    }
    if hovered && let Some(part) = &self.option_hovered {
      style.merge_from(part);
    }
    if selected
      && hovered
      && let Some(part) = &self.option_selected_hovered
    {
      style.merge_from(part);
    }
    style
  }
}
