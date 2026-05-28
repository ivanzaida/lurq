use crate::{app::events::MouseButton, core::NodeId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropResult {
  Accepted { target_id: NodeId },
  Missed,
}

#[derive(Debug, Clone, Copy)]
pub struct DragEvent {
  pub x: f32,
  pub y: f32,
  pub start_x: f32,
  pub start_y: f32,
  pub delta_x: f32,
  pub delta_y: f32,
  pub total_delta_x: f32,
  pub total_delta_y: f32,
  pub button: MouseButton,
  pub target_id: NodeId,
  pub drop_result: Option<DropResult>,
}

#[derive(Debug, Clone, Copy)]
pub struct DropEvent {
  pub x: f32,
  pub y: f32,
  pub start_x: f32,
  pub start_y: f32,
  pub total_delta_x: f32,
  pub total_delta_y: f32,
  pub button: MouseButton,
  pub source_id: NodeId,
  pub target_id: NodeId,
}
