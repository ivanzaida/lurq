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
}
