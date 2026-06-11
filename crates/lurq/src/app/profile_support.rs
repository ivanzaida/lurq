use std::time::Duration;
#[cfg(feature = "perf_profile")]
use std::time::Instant;

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
