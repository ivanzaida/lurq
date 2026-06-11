use raw_window_handle::{DisplayHandle, WindowHandle};

use crate::{app::profiler::RenderProfile, layout::render_list::RenderList};

pub trait RenderEngine {
  fn resize(&mut self, width: u32, height: u32);
  fn render(&mut self, list: &RenderList, window: WindowHandle<'_>, display: DisplayHandle<'_>);

  fn release_window_surface(&mut self) {}

  fn last_profile(&self) -> Option<RenderProfile> {
    None
  }
}

pub type RenderEngineFactory = std::sync::Arc<dyn Fn() -> Box<dyn RenderEngine>>;
