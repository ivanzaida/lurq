use std::sync::Arc;

use parking_lot::Mutex;

use super::EventControl;
use crate::core::NodeId;

/// A copy or a cut from the focused element where it is not a text input: Ctrl+C (Cmd+C on macOS) and
/// Ctrl+Insert copy, Ctrl+X and Shift+Delete cut. What is copied is the app's: a handler on the focused element,
/// or on an ancestor it bubbles to, says the text with [`set_text`](Self::set_text), and the engine puts it on the
/// clipboard once the handlers have run. A cut is a copy the handler also takes away
/// ([`is_cut`](Self::is_cut)). With no text set nothing is written.
///
/// A text input copies and cuts its own selection and makes no copy event; a selectable text with a selection is
/// copied as before where no handler set a text.
#[derive(Clone)]
pub struct CopyEvent {
  text: Arc<Mutex<Option<String>>>,
  cut: bool,
  /// The node whose handlers run now: the focused element, or an ancestor the event bubbled to.
  pub target_id: NodeId,
  /// The focused element the copy is from.
  pub focus_id: NodeId,
  pub(crate) control: EventControl,
}

impl CopyEvent {
  pub(crate) fn new(focus_id: NodeId, cut: bool) -> Self {
    Self {
      text: Arc::new(Mutex::new(None)),
      cut,
      target_id: focus_id,
      focus_id,
      control: EventControl::new(),
    }
  }

  pub(crate) fn at(&self, target_id: NodeId) -> Self {
    Self {
      text: Arc::clone(&self.text),
      cut: self.cut,
      target_id,
      focus_id: self.focus_id,
      control: self.control.clone(),
    }
  }

  /// What goes on the clipboard: a later call takes the place of an earlier one, a handler's on an ancestor
  /// too.
  pub fn set_text(&self, text: impl Into<String>) {
    *self.text.lock() = Some(text.into());
  }

  /// The text a handler set so far.
  pub fn text(&self) -> Option<String> {
    self.text.lock().clone()
  }

  /// Whether it is a cut: the handler that copies takes away what it copied.
  pub fn is_cut(&self) -> bool {
    self.cut
  }

  /// Stops the event from bubbling to the element's ancestors.
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

impl std::fmt::Debug for CopyEvent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CopyEvent")
      .field("cut", &self.cut)
      .field("target_id", &self.target_id)
      .field("focus_id", &self.focus_id)
      .finish()
  }
}
