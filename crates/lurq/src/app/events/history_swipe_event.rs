use super::EventControl;
use crate::core::NodeId;

/// Which way a history swipe goes: fingers to the right go back, as in a browser. It is the command the swipe asks
/// for.
pub type HistoryDirection = super::NavigationCommand;

/// A swipe between pages: `on_history_swipe`. Two fingers moved across a trackpad sideways and lifted, and nothing
/// under the pointer scrolled with them; or a finger came in from the left or the right edge of the window far
/// enough (see [`EdgePanEvent`](super::EdgePanEvent), which tells it as it moves). `x` and `y` are logical pixels
/// where the swipe ended. The event bubbles from the innermost element under the pointer to its ancestors.
///
/// Its default is the command it asks for ([`NavigationCommand`](super::NavigationCommand)): lurq goes back or forward
/// from the element under the pointer, as the mouse's side buttons do (`on_history_back`, `on_history_forward`, the
/// router). A handler that does something else with the swipe calls `prevent_default`; one that only goes back
/// leaves it to the command, and handles `on_history_back` to go back from every source alike.
#[derive(Debug, Clone)]
pub struct HistorySwipeEvent {
  pub direction: HistoryDirection,
  pub x: f32,
  pub y: f32,
  pub target_id: NodeId,
  pub(crate) control: EventControl,
}

impl HistorySwipeEvent {
  pub fn prevent_default(&self) {
    self.control.prevent_default();
  }

  pub fn default_prevented(&self) -> bool {
    self.control.default_prevented()
  }

  pub fn stop_propagation(&self) {
    self.control.stop_propagation();
  }

  pub fn propagation_stopped(&self) -> bool {
    self.control.propagation_stopped()
  }

  pub fn stop_immediate_propagation(&self) {
    self.control.stop_immediate_propagation();
  }

  pub fn immediate_propagation_stopped(&self) -> bool {
    self.control.immediate_propagation_stopped()
  }
}
