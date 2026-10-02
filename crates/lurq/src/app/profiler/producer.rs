#![cfg_attr(not(feature = "perf_profile"), allow(dead_code))]
#[cfg(all(
  feature = "canvas",
  any(feature = "wgpu", all(feature = "dx12", target_os = "windows"))
))]
use std::sync::atomic::Ordering;
use std::{
  sync::Arc,
  time::{Duration, Instant},
};

use super::{FrameProfile, ProfilingHandle, collector::Progress, model::*};
use crate::app::PassReport;

/// Lightweight context passed to render backends for current-phase publication.
#[derive(Clone)]
pub struct ProfileContext {
  handle: ProfilingHandle,
  window: Arc<str>,
  frame_id: Option<u64>,
}

impl ProfileContext {
  pub fn phase(&self, phase: Phase) -> PhaseGuard {
    let previous = self.handle.enter_phase(&self.window, self.frame_id, phase);
    PhaseGuard {
      context: self.clone(),
      previous,
    }
  }

  /// Detailed Canvas phases avoid per-command-group locks with no capture.
  #[cfg(all(
    feature = "canvas",
    any(feature = "wgpu", all(feature = "dx12", target_os = "windows"))
  ))]
  pub(crate) fn detail_phase(&self, phase: Phase) -> Option<PhaseGuard> {
    self
      .handle
      .inner
      .active
      .load(Ordering::Acquire)
      .then(|| self.phase(phase))
  }

  pub(crate) fn input(&self, kind: InputKind) -> InputGuard {
    InputGuard {
      context: self.clone(),
      kind,
      started: Instant::now(),
      _phase: self.phase(Phase::InputDispatch),
    }
  }

  pub(crate) fn update(&self, kind: UiUpdateKind) -> UpdateGuard {
    UpdateGuard {
      context: self.clone(),
      kind,
      started: Instant::now(),
      commit_started: None,
      _phase: self.phase(Phase::Rebuild),
      commit_phase: None,
    }
  }
}

pub(crate) struct InputGuard {
  context: ProfileContext,
  kind: InputKind,
  started: Instant,
  _phase: PhaseGuard,
}

impl Drop for InputGuard {
  fn drop(&mut self) {
    self.context.handle.record(
      &self.context.window,
      self.started,
      SampleData::InputDispatch(InputDispatchSample {
        frame_id: self.context.frame_id,
        kind: self.kind,
        total: self.started.elapsed(),
      }),
    );
  }
}

pub(crate) struct UpdateGuard {
  context: ProfileContext,
  kind: UiUpdateKind,
  started: Instant,
  commit_started: Option<Instant>,
  _phase: PhaseGuard,
  commit_phase: Option<PhaseGuard>,
}

impl UpdateGuard {
  pub(crate) fn commit(&mut self) {
    self.commit_started = Some(Instant::now());
    self.commit_phase = Some(self.context.phase(Phase::Commit));
  }
}

impl Drop for UpdateGuard {
  fn drop(&mut self) {
    self.context.handle.record(
      &self.context.window,
      self.started,
      SampleData::UiUpdate(UiUpdateSample {
        frame_id: self.context.frame_id,
        kind: self.kind,
        total: self.started.elapsed(),
        commit: self.commit_started.map(|start| start.elapsed()).unwrap_or_default(),
      }),
    );
    // Restore nested scopes in reverse order before the outer field drops.
    self.commit_phase.take();
  }
}

pub struct PhaseGuard {
  context: ProfileContext,
  previous: Option<Progress>,
}

impl Drop for PhaseGuard {
  fn drop(&mut self) {
    self.context.handle.progress(&self.context.window, self.previous);
  }
}

pub(crate) struct WindowProfiler {
  pub(crate) context: ProfileContext,
  root: bool,
  layout: Duration,
  canvas_recording: Duration,
  canvas_preparation: Duration,
}

impl WindowProfiler {
  pub(crate) fn new() -> Self {
    let handle = ProfilingHandle::new();
    handle.register("main", false);
    Self {
      context: ProfileContext {
        handle,
        window: "main".into(),
        frame_id: None,
      },
      root: true,
      layout: Duration::ZERO,
      canvas_recording: Duration::ZERO,
      canvas_preparation: Duration::ZERO,
    }
  }

  pub(crate) fn attach(&mut self, parent: &Self, id: String, devtools: bool) {
    if self.root {
      self.context.handle.close();
    } else {
      self.context.handle.set_open(&self.context.window, false);
    }
    self.context = ProfileContext {
      handle: parent.context.handle.clone(),
      window: id.into(),
      frame_id: None,
    };
    self.root = false;
    self.context.handle.register(&self.context.window, devtools);
  }

  pub(crate) fn handle(&self) -> ProfilingHandle {
    self.context.handle.clone()
  }

  pub(crate) fn child_id(&self, id: u64) -> String {
    if self.context.window.as_ref() == "main" {
      format!("w{id}")
    } else {
      format!("{}/w{id}", self.context.window)
    }
  }

  pub(crate) fn set_open(&self, open: bool) {
    self.context.handle.set_open(&self.context.window, open);
  }
  #[cfg(feature = "devtools")]
  pub(crate) fn mark_devtools(&self) {
    self.context.handle.mark_devtools(&self.context.window);
  }

  pub(crate) fn begin_pass(&mut self, frame_id: u64) -> (Instant, PhaseGuard) {
    self.layout = Duration::ZERO;
    self.canvas_recording = Duration::ZERO;
    self.canvas_preparation = Duration::ZERO;
    self.context.frame_id = Some(frame_id);
    (Instant::now(), self.context.phase(Phase::PassSetup))
  }

  pub(crate) fn layout(&mut self, duration: Duration) {
    self.layout = duration;
  }
  #[cfg(feature = "canvas")]
  pub(crate) fn canvas_recording(&mut self, duration: Duration) {
    self.canvas_recording += duration;
  }
  #[cfg(feature = "canvas")]
  pub(crate) fn canvas_preparation(&mut self, duration: Duration) {
    self.canvas_preparation += duration;
  }

  pub(crate) fn finish_pass(
    &mut self,
    start: Instant,
    report: &PassReport,
    backend: &'static str,
    frame: Option<&FrameProfile>,
  ) {
    if report.required {
      self.context.handle.record(
        &self.context.window,
        start,
        SampleData::Pass(PassSample {
          frame_id: report.rendered.then_some(self.context.frame_id.unwrap()),
          rendered: report.rendered,
          cached_render_list: report.used_cached_render_list,
          total: start.elapsed(),
          layout_update: self.layout,
          layout_recalculated: report.layout_recalculated,
          canvas_recording: self.canvas_recording,
          canvas_preparation: self.canvas_preparation,
          backend,
          frame: frame.cloned(),
        }),
      );
    }
    self.context.frame_id = None;
  }
}

impl Drop for WindowProfiler {
  fn drop(&mut self) {
    self.set_open(false);
    if self.root {
      self.context.handle.close();
    }
  }
}
