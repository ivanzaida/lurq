//! Public RAII API. This does not drive the UI, redraw, or start a session.
#![cfg_attr(not(feature = "perf_profile"), allow(dead_code))]
use std::{marker::PhantomData, rc::Rc};

use super::{ProfilingHandle, application_model::*};

/// An explicit temporal parent; children require this scope to still be live.
/// Clone this token for worker handoff, never move a guard between threads.
#[derive(Clone)]
pub struct ApplicationScopeParent {
  pub(super) handle: ProfilingHandle,
  pub(super) id: ApplicationScopeId,
}

/// Inclusive synchronous wall scope. Drop before an `.await` or scheduling work.
///
/// This guard is deliberately neither Send nor Sync. It cannot prevent holding
/// a guard across a single-thread executor's await; that remains a caller rule.
#[must_use = "keep the guard alive for the synchronous operation being measured"]
pub struct ApplicationScope {
  handle: Option<ProfilingHandle>,
  id: Option<ApplicationScopeId>,
  status: ApplicationScopeStatus,
  _same_thread: PhantomData<Rc<()>>,
}

impl ApplicationScope {
  pub fn status(&self) -> ApplicationScopeStatus {
    self.status
  }

  pub fn id(&self) -> Option<ApplicationScopeId> {
    self.id
  }

  pub fn parent(&self) -> Option<ApplicationScopeParent> {
    Some(ApplicationScopeParent {
      handle: self.handle.as_ref()?.clone(),
      id: self.id?,
    })
  }

  /// Begins an explicitly nested scope on the same registered window.
  /// Concurrent children can have different declared lanes.
  pub fn child(&self, label: &'static str, lane: ApplicationLane) -> Self {
    match self.parent() {
      Some(parent) => parent.handle.application_scope_child(&parent, label, lane),
      None => Self::inert(self.status),
    }
  }

  /// Ends this synchronous scope, useful immediately before an await.
  pub fn finish(self) {}

  pub(super) fn inert(status: ApplicationScopeStatus) -> Self {
    Self {
      handle: None,
      id: None,
      status,
      _same_thread: PhantomData,
    }
  }

  #[cfg(feature = "perf_profile")]
  pub(super) fn active(handle: ProfilingHandle, id: ApplicationScopeId) -> Self {
    Self {
      handle: Some(handle),
      id: Some(id),
      status: ApplicationScopeStatus::Active,
      _same_thread: PhantomData,
    }
  }
}

impl Drop for ApplicationScope {
  fn drop(&mut self) {
    #[cfg(feature = "perf_profile")]
    if let (Some(handle), Some(id)) = (&self.handle, self.id) {
      handle.finish_application_scope(id);
    }
  }
}

impl ProfilingHandle {
  /// Measures a synchronous application operation on a registered window.
  ///
  /// Labels must be static ASCII identifiers (letters/digits/underscore), 1–64
  /// bytes, containing no user data. Refusals yield an inert guard. With profiling
  /// disabled this has no clock, collector lock, or allocation. Enabled idle
  /// calls retain fixed live-slot metadata so mid-operation sessions are honest;
  /// they take a short collector lock and timestamps but allocate no history.
  pub fn application_scope(&self, window: &str, label: &'static str, lane: ApplicationLane) -> ApplicationScope {
    #[cfg(feature = "perf_profile")]
    {
      self.begin_application_scope(Some(window), None, label, lane)
    }
    #[cfg(not(feature = "perf_profile"))]
    {
      let _ = (window, label, lane);
      ApplicationScope::inert(ApplicationScopeStatus::FeatureDisabled)
    }
  }

  /// Begins a child only when the opaque parent belongs to this collector and
  /// remains live at entry. A token is not a causal link to an already ended job.
  pub fn application_scope_child(
    &self,
    parent: &ApplicationScopeParent,
    label: &'static str,
    lane: ApplicationLane,
  ) -> ApplicationScope {
    #[cfg(feature = "perf_profile")]
    {
      self.begin_application_scope(None, Some(parent), label, lane)
    }
    #[cfg(not(feature = "perf_profile"))]
    {
      let _ = (parent, label, lane);
      ApplicationScope::inert(ApplicationScopeStatus::FeatureDisabled)
    }
  }
}
