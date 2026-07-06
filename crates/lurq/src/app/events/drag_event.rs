use std::{any::Any, fmt, sync::Arc};

use crate::{app::events::MouseButton, core::NodeId};

/// Data a drag source attaches to its drag session, delivered to the drop
/// target's handler through [`DropEvent::payload`].
pub type DragPayload = Arc<dyn Any + Send + Sync>;

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

#[derive(Clone)]
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
  pub payload: Option<DragPayload>,
}

impl DropEvent {
  /// The drag source's payload, downcast to its concrete type.
  pub fn payload<T: Any>(&self) -> Option<&T> {
    self.payload.as_ref()?.downcast_ref()
  }
}

impl fmt::Debug for DropEvent {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("DropEvent")
      .field("x", &self.x)
      .field("y", &self.y)
      .field("start_x", &self.start_x)
      .field("start_y", &self.start_y)
      .field("total_delta_x", &self.total_delta_x)
      .field("total_delta_y", &self.total_delta_y)
      .field("button", &self.button)
      .field("source_id", &self.source_id)
      .field("target_id", &self.target_id)
      .field("payload", &self.payload.as_ref().map(|_| "<payload>"))
      .finish()
  }
}
