pub use crate::app::profile_types::{FrameProfile, RenderProfile, RuntimeMemoryProfile};

#[cfg(feature = "perf_profile")]
mod observer {
  use std::sync::OnceLock;

  use super::FrameProfile;

  type FrameProfileObserver = Box<dyn Fn(&FrameProfile) + Send + Sync>;

  static OBSERVER: OnceLock<FrameProfileObserver> = OnceLock::new();

  /// Registers a process-wide observer invoked with every completed frame
  /// profile. Hosts embedding external renderers use this to fold the UI
  /// frame's layout/raster/present timings into their own profilers. Only the
  /// first registration wins; later calls are ignored.
  pub fn set_frame_profile_observer(observer: impl Fn(&FrameProfile) + Send + Sync + 'static) {
    let _ = OBSERVER.set(Box::new(observer));
  }

  pub(crate) fn notify_frame_profile(profile: &FrameProfile) {
    if let Some(observer) = OBSERVER.get() {
      observer(profile);
    }
  }
}

#[cfg(feature = "perf_profile")]
pub use observer::set_frame_profile_observer;

#[cfg(feature = "perf_profile")]
pub(crate) use observer::notify_frame_profile;

#[cfg(all(test, feature = "perf_profile"))]
mod tests {
  use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  };
  use std::time::Duration;

  use super::FrameProfile;

  #[test]
  fn frame_profile_observer_receives_completed_frames() {
    let seen = Arc::new(AtomicUsize::new(0));
    let seen_by_observer = seen.clone();
    super::set_frame_profile_observer(move |profile| {
      if profile.layout == Duration::from_millis(3) {
        seen_by_observer.fetch_add(1, Ordering::SeqCst);
      }
    });

    let profile = FrameProfile {
      layout: Duration::from_millis(3),
      ..FrameProfile::default()
    };
    super::notify_frame_profile(&profile);
    super::notify_frame_profile(&profile);

    assert_eq!(seen.load(Ordering::SeqCst), 2);
  }
}
