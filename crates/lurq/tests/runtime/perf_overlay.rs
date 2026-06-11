use std::sync::{Arc, Mutex};

use lurq::{
  app::{App, Tree, render_engine::RenderEngine},
  layout::render_list::RenderList,
};
use raw_window_handle::{DisplayHandle, WindowHandle};

use crate::support::TestSurface;

#[test]
fn draw_perf_overlay_renders_overlay() {
  let render_calls = Arc::new(Mutex::new(0));
  let calls = render_calls.clone();
  let mut app = App::new();
  let mut tree = Tree::new();

  tree.set_root(lurq::components::Rect::new(100.0, 40.0).background("#22c55e"));
  tree.set_render_engine_factory(move || {
    Box::new(PerfOverlayCaptureRenderEngine {
      render_calls: calls.clone(),
    })
  });

  tree.draw_perf_overlay();
  tree.pass(&mut app, &TestSurface);

  assert_eq!(*render_calls.lock().unwrap(), 1);
  #[cfg(feature = "perf_profile")]
  assert!(tree.last_profile().total > std::time::Duration::ZERO);
}

struct PerfOverlayCaptureRenderEngine {
  render_calls: Arc<Mutex<usize>>,
}

impl RenderEngine for PerfOverlayCaptureRenderEngine {
  fn resize(&mut self, _width: u32, _height: u32) {}

  fn render(&mut self, _list: &RenderList, _window: WindowHandle<'_>, _display: DisplayHandle<'_>) {
    *self.render_calls.lock().unwrap() += 1;
  }
}
