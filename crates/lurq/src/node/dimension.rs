#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Dimension {
  #[default]
  Auto,
  Px(f32),
  Pct(f32),
}

impl Dimension {
  pub fn to_px(&self) -> f32 {
    match self {
      Dimension::Px(v) => *v,
      _ => 0.0,
    }
  }

  pub fn resolve(&self, parent_size: f32) -> f32 {
    match self {
      Dimension::Px(v) => *v,
      Dimension::Pct(pct) => parent_size * pct / 100.0,
      Dimension::Auto => 0.0,
    }
  }
}

impl From<f32> for Dimension {
  fn from(value: f32) -> Self {
    Self::Px(value)
  }
}

pub trait IntoDimension {
  fn pct(&self) -> Dimension;
  fn px(&self) -> Dimension;
}

impl IntoDimension for f32 {
  fn pct(&self) -> Dimension {
    Dimension::Pct(*self)
  }

  fn px(&self) -> Dimension {
    Dimension::Px(*self)
  }
}

impl IntoDimension for i32 {
  fn pct(&self) -> Dimension {
    Dimension::Pct(*self as f32)
  }

  fn px(&self) -> Dimension {
    Dimension::Px(*self as f32)
  }
}
