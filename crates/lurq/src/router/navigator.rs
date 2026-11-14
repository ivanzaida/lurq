use std::{any::Any, sync::Arc};

use super::{Query, handle::RouterHandle};
use crate::core::signal::Signal;

#[derive(Clone)]
pub struct Navigator {
  pub(crate) handle: RouterHandle,
}

impl Navigator {
  pub fn push(&self, path: impl Into<String>) {
    self.handle.push(path);
  }

  pub fn replace(&self, path: impl Into<String>) {
    self.handle.replace(path);
  }

  /// Navigate to `path`, attaching in-memory `state` to the new history entry.
  /// Read it back at the destination with [`Navigator::state`] or `ctx.route_state`.
  pub fn push_with_state<S: Any + Send + Sync>(&self, path: impl Into<String>, state: S) {
    self.handle.push_with_state(path, state);
  }

  /// Like [`Navigator::replace`], but attaches in-memory `state` to the entry.
  pub fn replace_with_state<S: Any + Send + Sync>(&self, path: impl Into<String>, state: S) {
    self.handle.replace_with_state(path, state);
  }

  /// Parsed query string of the current location.
  pub fn query(&self) -> Query {
    Query::from_path(&self.handle.path().get_untracked())
  }

  /// In-memory state attached to the current location, downcast to `T`.
  pub fn state<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
    self.handle.current_state()?.downcast::<T>().ok()
  }

  pub fn back(&self) -> bool {
    self.handle.back()
  }

  /// Goes back several entries at once; see [`RouterHandle::back_by`].
  pub fn back_by(&self, steps: usize) -> bool {
    self.handle.back_by(steps)
  }

  pub fn forward(&self) -> bool {
    self.handle.forward()
  }

  /// Whether [`Navigator::back`] would move — there is a prior entry.
  pub fn can_back(&self) -> bool {
    self.handle.can_back()
  }

  /// Whether [`Navigator::forward`] would move — there is a later entry.
  pub fn can_forward(&self) -> bool {
    self.handle.can_forward()
  }

  pub fn path(&self) -> Signal<String> {
    self.handle.path()
  }
}

impl std::hash::Hash for Navigator {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    (Arc::as_ptr(&self.handle.inner) as usize).hash(state);
  }
}

thread_local! {
  /// The navigator of each component being rendered, innermost last: what a builder with a path to go to reads.
  static RENDERING_UNDER: std::cell::RefCell<Vec<Option<Navigator>>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// The navigator of the router around what is being built, as `Ctx::navigator` reads it: for a builder that has no
/// `Ctx`, a list item with a path to go to. `None` outside a router and outside a render.
pub fn current_navigator() -> Option<Navigator> {
  RENDERING_UNDER.with(|under| under.borrow().last().cloned().flatten())
}

/// A component's render starts under `navigator`.
pub(crate) fn enter_render(navigator: Option<Navigator>) {
  RENDERING_UNDER.with(|under| under.borrow_mut().push(navigator));
}

/// The component being rendered provides `navigator` to what it builds.
pub(crate) fn provide_in_render(navigator: Navigator) {
  RENDERING_UNDER.with(|under| {
    if let Some(top) = under.borrow_mut().last_mut() {
      *top = Some(navigator);
    }
  });
}

/// A component's render ends.
pub(crate) fn leave_render() {
  RENDERING_UNDER.with(|under| {
    under.borrow_mut().pop();
  });
}
