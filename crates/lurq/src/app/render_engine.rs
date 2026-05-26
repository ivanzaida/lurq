use raw_window_handle::{DisplayHandle, WindowHandle};

use crate::layout::render_list::RenderList;

pub trait RenderEngine {
  fn resize(&mut self, width: u32, height: u32);
  fn render(&mut self, list: &RenderList, window: WindowHandle<'_>, display: DisplayHandle<'_>);
}
