use std::time::Duration;

#[cfg(feature = "perf_profile")]
use crate::time::Instant;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PerfMeterStats {
  pub fps: u32,
  pub total_ms: f32,
  pub layout_ms: f32,
  pub quad_resolve_ms: f32,
  pub glyph_ms: f32,
  pub render_cpu_ms: f32,
  pub render_acquire_ms: f32,
  pub render_upload_ms: f32,
  pub render_encode_ms: f32,
  pub render_submit_ms: f32,
  pub render_present_ms: f32,
  pub quad_count: usize,
  pub glyph_count: usize,
}

#[cfg_attr(not(feature = "perf_profile"), allow(dead_code))]
pub(crate) struct ProfileScope {
  #[cfg(feature = "perf_profile")]
  start: Instant,
}

impl ProfileScope {
  #[cfg_attr(not(feature = "perf_profile"), allow(dead_code))]
  pub(crate) fn start() -> Self {
    Self {
      #[cfg(feature = "perf_profile")]
      start: Instant::now(),
    }
  }

  #[cfg_attr(not(feature = "perf_profile"), allow(dead_code))]
  pub(crate) fn elapsed(&self) -> Duration {
    #[cfg(feature = "perf_profile")]
    {
      self.start.elapsed()
    }
    #[cfg(not(feature = "perf_profile"))]
    {
      Duration::default()
    }
  }
}

macro_rules! profile_scope {
  () => {{
    #[cfg(feature = "perf_profile")]
    {
      $crate::app::profile_support::ProfileScope::start()
    }
    #[cfg(not(feature = "perf_profile"))]
    {
      ()
    }
  }};
}

macro_rules! profile_elapsed {
  ($scope:expr) => {{
    #[cfg(feature = "perf_profile")]
    {
      $scope.elapsed()
    }
    #[cfg(not(feature = "perf_profile"))]
    {
      let _ = &$scope;
      std::time::Duration::default()
    }
  }};
}

macro_rules! profile_if {
  ($($body:tt)*) => {
    #[cfg(feature = "perf_profile")]
    {
      $($body)*
    }
  };
}

macro_rules! profile_value {
  ($value:expr) => {{
    #[cfg(feature = "perf_profile")]
    {
      $value
    }
    #[cfg(not(feature = "perf_profile"))]
    {
      Default::default()
    }
  }};
}

pub(crate) use profile_elapsed;
pub(crate) use profile_if;
pub(crate) use profile_scope;
pub(crate) use profile_value;

/// What each named section of the frames since the last [`take_frame_sections`] took, on this thread: the walks of
/// a pass, measured where they run (`perf_profile` only).
#[cfg(feature = "perf_profile")]
mod frame_sections {
  use std::{cell::RefCell, time::Duration};

  use crate::time::Instant;

  thread_local! {
    static SECTIONS: RefCell<Vec<(&'static str, Duration, u32)>> = const { RefCell::new(Vec::new()) };
  }

  /// Times a section until it is dropped, and adds that to its name's total.
  pub(crate) struct FrameSection {
    name: &'static str,
    started: Instant,
  }

  impl FrameSection {
    pub(crate) fn new(name: &'static str) -> Self {
      Self {
        name,
        started: Instant::now(),
      }
    }
  }

  impl Drop for FrameSection {
    fn drop(&mut self) {
      let elapsed = self.started.elapsed();
      SECTIONS.with(|sections| {
        let mut sections = sections.borrow_mut();
        match sections.iter_mut().find(|(name, ..)| *name == self.name) {
          Some((_, total, count)) => {
            *total += elapsed;
            *count += 1;
          }
          None => sections.push((self.name, elapsed, 1)),
        }
      });
    }
  }

  /// Each section's total time and how many times it ran since the last call, in the order they first ran.
  pub fn take_frame_sections() -> Vec<(&'static str, Duration, u32)> {
    SECTIONS.with(|sections| std::mem::take(&mut *sections.borrow_mut()))
  }

  thread_local! {
    static PASSES: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
  }

  /// With `LURQ_FRAME_SECTIONS_LOG` set to a file, appends there every 120 passes each section's mean a pass and
  /// the tree's size: where an app's frames spend their time, read while it runs.
  pub(crate) fn log_frame_sections(nodes: impl FnOnce() -> usize) {
    let Some(path) = std::env::var_os("LURQ_FRAME_SECTIONS_LOG") else {
      return;
    };
    let passes = PASSES.with(|passes| {
      passes.set(passes.get() + 1);
      passes.get()
    });
    if passes % 120 != 0 {
      return;
    }
    use std::io::Write;
    let sections = take_frame_sections();
    let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) else {
      return;
    };
    let _ = writeln!(file, "passes {passes} nodes {}", nodes());
    for (name, total, runs) in sections {
      let _ = writeln!(
        file,
        "  {name}: {:.3} ms a pass ({runs} runs)",
        total.as_secs_f64() * 1000.0 / 120.0
      );
    }
  }
}

#[cfg(feature = "perf_profile")]
pub use frame_sections::take_frame_sections;
#[cfg(feature = "perf_profile")]
pub(crate) use frame_sections::{FrameSection, log_frame_sections};

/// Times the rest of the enclosing block as the frame section `$name` (`perf_profile` only).
macro_rules! frame_section {
  ($name:expr) => {
    #[cfg(feature = "perf_profile")]
    let _frame_section = crate::app::profile_support::FrameSection::new($name);
  };
}
pub(crate) use frame_section;

/// Noisy video/frame timing diagnostics are explicitly opt-in, independently
/// of the host application's tracing filter. Read once to keep hot paths cheap.
pub(crate) fn video_diagnostics_enabled() -> bool {
  static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
  *ENABLED.get_or_init(|| std::env::var("LURQ_VIDEO_LOGS").is_ok_and(|value| value == "1"))
}
macro_rules! video_log {
  ($level:ident, $($args:tt)*) => {
    if $crate::app::profile_support::video_diagnostics_enabled() {
      tracing::$level!($($args)*);
    }
  };
}
pub(crate) use video_log;
