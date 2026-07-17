use raw_window_handle::{DisplayHandle, WindowHandle};

#[cfg(feature = "perf_profile")]
use crate::app::profile_types::RenderProfile;
use crate::layout::render_list::RenderList;

#[cfg(feature = "screenshot")]
#[derive(Clone)]
pub struct RenderFrameCapture {
  pub x: u32,
  pub y: u32,
  pub width: u32,
  pub height: u32,
  pub output_path: std::path::PathBuf,
  pub window_clip: Option<RenderFrameCaptureWindowClip>,
}

#[cfg(feature = "screenshot")]
#[derive(Clone, Copy)]
pub struct RenderFrameCaptureWindowClip {
  pub width: f32,
  pub height: f32,
  pub radii: [f32; 4],
}

pub trait RenderEngine {
  fn resize(&mut self, width: u32, height: u32);
  fn render(&mut self, list: &RenderList, window: WindowHandle<'_>, display: DisplayHandle<'_>) -> bool;

  #[cfg(feature = "screenshot")]
  fn supports_frame_capture(&self) -> bool {
    false
  }

  #[cfg(feature = "screenshot")]
  fn render_with_capture(
    &mut self,
    list: &RenderList,
    window: WindowHandle<'_>,
    display: DisplayHandle<'_>,
    _capture: Option<RenderFrameCapture>,
  ) -> bool {
    self.render(list, window, display)
  }

  fn release_window_surface(&mut self) {}

  fn wants_redraw(&self) -> bool {
    false
  }

  #[cfg(feature = "perf_profile")]
  fn last_profile(&self) -> Option<RenderProfile> {
    None
  }
}

pub type RenderEngineFactory = std::sync::Arc<dyn Fn() -> Box<dyn RenderEngine>>;
