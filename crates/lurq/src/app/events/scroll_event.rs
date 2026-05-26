use crate::core::NodeId;

#[derive(Debug)]
pub struct ScrollEvent {
  pub x: f32,
  pub y: f32,
  pub delta_x: f32,
  pub delta_y: f32,
  pub phase: ScrollPhase,
  pub target_id: NodeId,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum ScrollPhase {
  Start,
  #[default]
  Scroll,
  End,
}
