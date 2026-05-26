#[derive(Clone, Copy, Default)]
pub struct Offset {
  pub x: f32,
  pub y: f32,
}

impl Offset {
  pub fn new(x: f32, y: f32) -> Self {
    Self { x, y }
  }
}
