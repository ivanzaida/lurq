#[derive(Clone, Default)]
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
