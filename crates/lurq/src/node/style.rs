use crate::{
  layout::layout_kind::{FlexParams, FrameConstraints},
  node::{
    border::{Border, BorderRadius, Borders},
    color::Color,
    cursor::CursorIcon,
    dimension::Dimension,
    padding::Padding,
  },
};

#[derive(Clone, Default)]
pub struct Style {
  pub(crate) color: Option<Color>,
  pub(crate) border_radius: Option<BorderRadius>,
  pub(crate) border: Option<Borders>,
  pub(crate) cursor: Option<CursorIcon>,
  pub(crate) frame: Option<FrameConstraints>,
  pub(crate) padding: Option<Padding>,
  pub(crate) flex: Option<FlexParams>,
}

#[derive(Clone, Default)]
pub(crate) struct StateStyles {
  pub(crate) hovered: Option<Style>,
  pub(crate) active: Option<Style>,
  pub(crate) focused: Option<Style>,
}

impl Style {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn fill(mut self, hex: &str) -> Self {
    self.color = Some(Color::from_hex(hex));
    self
  }

  pub fn background(mut self, color: Color) -> Self {
    self.color = Some(color);
    self
  }

  pub fn width(mut self, width: impl Into<Dimension>) -> Self {
    self.frame_mut().width = Some(width.into());
    self
  }

  pub fn height(mut self, height: impl Into<Dimension>) -> Self {
    self.frame_mut().height = Some(height.into());
    self
  }

  pub fn size(mut self, width: impl Into<Dimension>, height: impl Into<Dimension>) -> Self {
    let frame = self.frame_mut();
    frame.width = Some(width.into());
    frame.height = Some(height.into());
    self
  }

  pub fn min_width(mut self, width: impl Into<Dimension>) -> Self {
    self.frame_mut().min_width = Some(width.into());
    self
  }

  pub fn max_width(mut self, width: impl Into<Dimension>) -> Self {
    self.frame_mut().max_width = Some(width.into());
    self
  }

  pub fn min_height(mut self, height: impl Into<Dimension>) -> Self {
    self.frame_mut().min_height = Some(height.into());
    self
  }

  pub fn max_height(mut self, height: impl Into<Dimension>) -> Self {
    self.frame_mut().max_height = Some(height.into());
    self
  }

  pub fn frame(mut self, frame: FrameConstraints) -> Self {
    self.frame = Some(frame);
    self
  }

  pub fn pad(mut self, all: impl Into<Dimension>) -> Self {
    self.padding = Some(Padding::all(all.into()));
    self
  }

  pub fn pad_xy(mut self, horizontal: impl Into<Dimension>, vertical: impl Into<Dimension>) -> Self {
    self.padding = Some(Padding::symmetric(horizontal.into(), vertical.into()));
    self
  }

  pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
    self.padding = Some(padding.into());
    self
  }

  pub fn padding_custom(mut self, padding: Padding) -> Self {
    self.padding = Some(padding);
    self
  }

  pub fn padding_left(mut self, value: impl Into<Dimension>) -> Self {
    self.padding = Some(self.padding.unwrap_or_default().left(value.into()));
    self
  }

  pub fn padding_right(mut self, value: impl Into<Dimension>) -> Self {
    self.padding = Some(self.padding.unwrap_or_default().right(value.into()));
    self
  }

  pub fn padding_top(mut self, value: impl Into<Dimension>) -> Self {
    self.padding = Some(self.padding.unwrap_or_default().top(value.into()));
    self
  }

  pub fn padding_bottom(mut self, value: impl Into<Dimension>) -> Self {
    self.padding = Some(self.padding.unwrap_or_default().bottom(value.into()));
    self
  }

  pub fn flex(mut self, factor: f32) -> Self {
    self.flex = Some(FlexParams::grow(factor));
    self
  }

  pub fn flex_shrink(mut self, factor: f32) -> Self {
    self.flex = Some(FlexParams {
      grow: 0.0,
      shrink: factor,
      basis: None,
    });
    self
  }

  pub fn flex_full(mut self, grow: f32, shrink: f32, basis: Option<f32>) -> Self {
    self.flex = Some(FlexParams { grow, shrink, basis });
    self
  }

  pub fn corner_radius(mut self, radius: f32) -> Self {
    self.border_radius = Some(BorderRadius::all(radius));
    self
  }

  pub fn corner_radius_custom(mut self, radius: BorderRadius) -> Self {
    self.border_radius = Some(radius);
    self
  }

  pub fn corner_radius_top_left(mut self, radius: f32) -> Self {
    self.border_radius.get_or_insert_with(BorderRadius::default).top_left = radius;
    self
  }

  pub fn corner_radius_top_right(mut self, radius: f32) -> Self {
    self.border_radius.get_or_insert_with(BorderRadius::default).top_right = radius;
    self
  }

  pub fn corner_radius_bottom_right(mut self, radius: f32) -> Self {
    self.border_radius.get_or_insert_with(BorderRadius::default).bottom_right = radius;
    self
  }

  pub fn corner_radius_bottom_left(mut self, radius: f32) -> Self {
    self.border_radius.get_or_insert_with(BorderRadius::default).bottom_left = radius;
    self
  }

  pub fn rounded(mut self, radius: f32) -> Self {
    self.border_radius = Some(BorderRadius::all(radius));
    self
  }

  pub fn border_inside(mut self, width: f32, color: Color) -> Self {
    self.border = Some(Borders::all(Border::inside(width, color)));
    self
  }

  pub fn border_outside(mut self, width: f32, color: Color) -> Self {
    self.border = Some(Borders::all(Border::outside(width, color)));
    self
  }

  pub fn border_center(mut self, width: f32, color: Color) -> Self {
    self.border = Some(Borders::all(Border::center(width, color)));
    self
  }

  pub fn border(mut self, border: Border) -> Self {
    self.border = Some(Borders::all(border));
    self
  }

  pub fn border_custom(mut self, border: Borders) -> Self {
    self.border = Some(border);
    self
  }

  pub fn border_top(mut self, border: Border) -> Self {
    self.border.get_or_insert_with(Borders::default).top = Some(border);
    self
  }

  pub fn border_right(mut self, border: Border) -> Self {
    self.border.get_or_insert_with(Borders::default).right = Some(border);
    self
  }

  pub fn border_bottom(mut self, border: Border) -> Self {
    self.border.get_or_insert_with(Borders::default).bottom = Some(border);
    self
  }

  pub fn border_left(mut self, border: Border) -> Self {
    self.border.get_or_insert_with(Borders::default).left = Some(border);
    self
  }

  pub fn cursor(mut self, cursor: CursorIcon) -> Self {
    self.cursor = Some(cursor);
    self
  }

  pub(crate) fn merge_from(&mut self, other: &Style) {
    if other.color.is_some() {
      self.color = other.color;
    }
    if other.border_radius.is_some() {
      self.border_radius = other.border_radius;
    }
    if other.border.is_some() {
      self.border = other.border;
    }
    if other.cursor.is_some() {
      self.cursor = other.cursor;
    }
    if other.frame.is_some() {
      self.frame = other.frame;
    }
    if other.padding.is_some() {
      self.padding = other.padding.clone();
    }
    if other.flex.is_some() {
      self.flex = other.flex;
    }
  }

  pub(crate) fn affects_layout(&self) -> bool {
    self.frame.is_some() || self.padding.is_some() || self.flex.is_some()
  }

  fn frame_mut(&mut self) -> &mut FrameConstraints {
    self.frame.get_or_insert_with(FrameConstraints::default)
  }
}

impl StateStyles {
  pub(crate) fn affects_layout(&self) -> bool {
    self.hovered.as_ref().is_some_and(Style::affects_layout)
      || self.active.as_ref().is_some_and(Style::affects_layout)
      || self.focused.as_ref().is_some_and(Style::affects_layout)
  }
}
