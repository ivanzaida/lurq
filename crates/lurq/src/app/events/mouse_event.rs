#[derive(Debug, Clone, Copy, Default)]
pub enum MouseButton {
  #[default]
  Left,
  Right,
  Middle,
  Other(u8),
}

#[derive(Debug)]
pub struct MouseEvent {
  pub x: f32,
  pub y: f32,
  pub button: MouseButton,
  pub kind: MouseEventKind,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum MouseEventKind {
  #[default]
  Click,
  Move,
  Up,
  Down,
  DoubleClick,
}
