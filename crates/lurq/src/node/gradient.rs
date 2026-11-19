use crate::node::background_color::BackgroundColor;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GradientKind {
  Linear,
  Radial,
  Conic,
  /// The blend of four colors, one at each corner of the box: see [`Gradient::corners`].
  Corners,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GradientStop {
  pub color: BackgroundColor,
  /// Position along the gradient line in `0.0..=1.0`. `None` is auto-placed
  /// per the CSS rules (endpoints pinned to 0/1, interior evenly spaced).
  pub position: Option<f32>,
}

impl GradientStop {
  pub fn new(color: impl Into<BackgroundColor>) -> Self {
    Self {
      color: color.into(),
      position: None,
    }
  }

  pub fn at(color: impl Into<BackgroundColor>, position: f32) -> Self {
    Self {
      color: color.into(),
      position: Some(position),
    }
  }
}

impl<C: Into<BackgroundColor>> From<C> for GradientStop {
  fn from(color: C) -> Self {
    Self::new(color)
  }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Gradient {
  pub kind: GradientKind,
  /// For `Linear`: CSS angle in degrees (`0` points up, increasing clockwise).
  /// For `Conic`: the starting angle.
  pub angle_deg: f32,
  /// Center for `Radial`/`Conic`, normalized `0.0..=1.0` within the box.
  pub center: (f32, f32),
  /// `Radial` only: ellipse fitted to the box (`true`) vs. circle (`false`).
  pub radial_ellipse: bool,
  pub stops: Vec<GradientStop>,
}

impl Gradient {
  pub fn linear(angle_deg: f32, stops: impl IntoIterator<Item = impl Into<GradientStop>>) -> Self {
    Self {
      kind: GradientKind::Linear,
      angle_deg,
      center: (0.5, 0.5),
      radial_ellipse: true,
      stops: stops.into_iter().map(Into::into).collect(),
    }
  }

  pub fn radial(stops: impl IntoIterator<Item = impl Into<GradientStop>>) -> Self {
    Self {
      kind: GradientKind::Radial,
      angle_deg: 0.0,
      center: (0.5, 0.5),
      radial_ellipse: true,
      stops: stops.into_iter().map(Into::into).collect(),
    }
  }

  pub fn conic(from_deg: f32, stops: impl IntoIterator<Item = impl Into<GradientStop>>) -> Self {
    Self {
      kind: GradientKind::Conic,
      angle_deg: from_deg,
      center: (0.5, 0.5),
      radial_ellipse: true,
      stops: stops.into_iter().map(Into::into).collect(),
    }
  }

  /// A fill that blends four colors, one at each corner of the box: `top_left`, `top_right`, `bottom_right` and
  /// `bottom_left`. A point is the mix of the top two across, and of the bottom two, mixed down: bilinear, in the
  /// sRGB-encoded values a picture's colors are written in, as CSS mixes the stops of its gradients. The area of a
  /// color picker is one: white, the hue, black and black. One box with one antialiased edge, where boxes stacked
  /// under one rounded clip would let those under show at its edge.
  pub fn corners(
    top_left: impl Into<BackgroundColor>,
    top_right: impl Into<BackgroundColor>,
    bottom_right: impl Into<BackgroundColor>,
    bottom_left: impl Into<BackgroundColor>,
  ) -> Self {
    Self {
      kind: GradientKind::Corners,
      angle_deg: 0.0,
      center: (0.5, 0.5),
      radial_ellipse: true,
      stops: vec![
        GradientStop::at(top_left, 0.0),
        GradientStop::at(top_right, 1.0 / 3.0),
        GradientStop::at(bottom_right, 2.0 / 3.0),
        GradientStop::at(bottom_left, 1.0),
      ],
    }
  }

  pub fn center(mut self, x: f32, y: f32) -> Self {
    self.center = (x, y);
    self
  }

  /// Render a `Radial` gradient as a circle instead of a box-fitted ellipse.
  pub fn circle(mut self) -> Self {
    self.radial_ellipse = false;
    self
  }
}
