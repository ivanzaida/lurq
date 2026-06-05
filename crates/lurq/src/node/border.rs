use crate::{
  app::theme::{ThemePalette, ThemeRadii},
  node::{BackgroundColor, color::Color, radius_value::RadiusValue},
};

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct BorderRadius {
  pub top_left: f32,
  pub top_right: f32,
  pub bottom_right: f32,
  pub bottom_left: f32,
}

impl BorderRadius {
  pub fn new(top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self {
    Self {
      top_left,
      top_right,
      bottom_right,
      bottom_left,
    }
  }

  pub fn all(r: f32) -> Self {
    Self {
      top_left: r,
      top_right: r,
      bottom_right: r,
      bottom_left: r,
    }
  }

  pub fn to_array(&self) -> [f32; 4] {
    [self.top_left, self.top_right, self.bottom_right, self.bottom_left]
  }
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct ThemedBorderRadius {
  pub top_left: RadiusValue,
  pub top_right: RadiusValue,
  pub bottom_right: RadiusValue,
  pub bottom_left: RadiusValue,
}

impl ThemedBorderRadius {
  pub fn new(
    top_left: impl Into<RadiusValue>,
    top_right: impl Into<RadiusValue>,
    bottom_right: impl Into<RadiusValue>,
    bottom_left: impl Into<RadiusValue>,
  ) -> Self {
    Self {
      top_left: top_left.into(),
      top_right: top_right.into(),
      bottom_right: bottom_right.into(),
      bottom_left: bottom_left.into(),
    }
  }

  pub fn all(radius: impl Into<RadiusValue>) -> Self {
    let radius = radius.into();
    Self {
      top_left: radius,
      top_right: radius,
      bottom_right: radius,
      bottom_left: radius,
    }
  }

  pub fn resolve(&self, radii: &ThemeRadii) -> BorderRadius {
    BorderRadius {
      top_left: self.top_left.resolve(radii),
      top_right: self.top_right.resolve(radii),
      bottom_right: self.bottom_right.resolve(radii),
      bottom_left: self.bottom_left.resolve(radii),
    }
  }

  pub fn as_border_radius(&self) -> Option<BorderRadius> {
    Some(BorderRadius {
      top_left: self.top_left.as_px()?,
      top_right: self.top_right.as_px()?,
      bottom_right: self.bottom_right.as_px()?,
      bottom_left: self.bottom_left.as_px()?,
    })
  }
}

impl From<f32> for ThemedBorderRadius {
  fn from(value: f32) -> Self {
    Self::all(value)
  }
}

impl From<crate::app::theme::RadiusSize> for ThemedBorderRadius {
  fn from(value: crate::app::theme::RadiusSize) -> Self {
    Self::all(value)
  }
}

impl From<BorderRadius> for ThemedBorderRadius {
  fn from(value: BorderRadius) -> Self {
    Self::new(value.top_left, value.top_right, value.bottom_right, value.bottom_left)
  }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Border {
  pub width: f32,
  pub color: BackgroundColor,
  pub placement: BorderPlacement,
}

impl Border {
  pub fn new(width: f32, color: impl Into<BackgroundColor>, placement: BorderPlacement) -> Self {
    Self {
      width,
      color: color.into(),
      placement,
    }
  }

  pub fn inside(width: f32, color: impl Into<BackgroundColor>) -> Self {
    Self::new(width, color, BorderPlacement::Inside)
  }

  pub fn outside(width: f32, color: impl Into<BackgroundColor>) -> Self {
    Self::new(width, color, BorderPlacement::Outside)
  }

  pub fn center(width: f32, color: impl Into<BackgroundColor>) -> Self {
    Self::new(width, color, BorderPlacement::Center)
  }

  pub(crate) fn resolve(&self, palette: &ThemePalette) -> Option<ResolvedBorder> {
    Some(ResolvedBorder {
      width: self.width,
      color: self.color.resolve(palette)?,
      placement: self.placement,
    })
  }
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Borders {
  pub top: Option<Border>,
  pub right: Option<Border>,
  pub bottom: Option<Border>,
  pub left: Option<Border>,
}

impl Borders {
  pub fn all(border: Border) -> Self {
    Self {
      top: Some(border),
      right: Some(border),
      bottom: Some(border),
      left: Some(border),
    }
  }

  pub fn any(&self) -> bool {
    self.top.is_some() || self.right.is_some() || self.bottom.is_some() || self.left.is_some()
  }

  pub fn color(&self) -> Option<BackgroundColor> {
    self
      .top
      .or(self.right)
      .or(self.bottom)
      .or(self.left)
      .map(|border| border.color)
  }

  pub fn set_color(&mut self, color: impl Into<BackgroundColor>) {
    let color = color.into();
    if let Some(border) = &mut self.top {
      border.color = color;
    }
    if let Some(border) = &mut self.right {
      border.color = color;
    }
    if let Some(border) = &mut self.bottom {
      border.color = color;
    }
    if let Some(border) = &mut self.left {
      border.color = color;
    }
  }

  pub(crate) fn resolve(&self, palette: &ThemePalette) -> Option<ResolvedBorders> {
    let borders = ResolvedBorders {
      top: self.top.and_then(|border| border.resolve(palette)),
      right: self.right.and_then(|border| border.resolve(palette)),
      bottom: self.bottom.and_then(|border| border.resolve(palette)),
      left: self.left.and_then(|border| border.resolve(palette)),
    };
    borders.any().then_some(borders)
  }

  pub fn top_width(&self) -> Option<f32> {
    self.top.map(|border| border.width)
  }

  pub fn right_width(&self) -> Option<f32> {
    self.right.map(|border| border.width)
  }

  pub fn bottom_width(&self) -> Option<f32> {
    self.bottom.map(|border| border.width)
  }

  pub fn left_width(&self) -> Option<f32> {
    self.left.map(|border| border.width)
  }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum BorderPlacement {
  #[default]
  Inside,
  Outside,
  Center,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedBorder {
  pub width: f32,
  pub color: Color,
  pub placement: BorderPlacement,
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct ResolvedBorders {
  pub top: Option<ResolvedBorder>,
  pub right: Option<ResolvedBorder>,
  pub bottom: Option<ResolvedBorder>,
  pub left: Option<ResolvedBorder>,
}

impl ResolvedBorders {
  pub(crate) fn any(&self) -> bool {
    self.top.is_some() || self.right.is_some() || self.bottom.is_some() || self.left.is_some()
  }
}
