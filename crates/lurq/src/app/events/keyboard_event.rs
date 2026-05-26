use crate::core::NodeId;

pub struct KeyboardEvent {
  pub key: String,
  pub code: String,
  pub shift: bool,
  pub ctrl: bool,
  pub alt: bool,
  pub target_id: NodeId,
}
