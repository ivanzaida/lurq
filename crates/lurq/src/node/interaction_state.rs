use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct InteractionState {
  inner: Arc<Mutex<InteractionStateInner>>,
}

#[derive(Default)]
struct InteractionStateInner {
  hovered: bool,
  active: bool,
  focused: bool,
}

impl InteractionState {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn is_hovered(&self) -> bool {
    self.inner.lock().unwrap().hovered
  }

  pub fn is_active(&self) -> bool {
    self.inner.lock().unwrap().active
  }

  pub fn is_focused(&self) -> bool {
    self.inner.lock().unwrap().focused
  }

  pub(crate) fn set_hovered(&self, val: bool) {
    self.inner.lock().unwrap().hovered = val;
  }

  pub(crate) fn set_active(&self, val: bool) {
    self.inner.lock().unwrap().active = val;
  }
}
