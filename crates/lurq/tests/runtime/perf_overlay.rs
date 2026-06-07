use std::sync::{Arc, Mutex};

use lurq::{
  app::{App, Tree, render_engine::RenderEngine},
  layout::render_list::RenderList,
};
use raw_window_handle::{DisplayHandle, WindowHandle};

use crate::support::TestSurface;

#[test]
fn draw_perf_overlay_enables_frame_profiling() {
  let profiling_flags = Arc::new(Mutex::new(Vec::new()));
  let flags = profiling_flags.clone();
  let mut app = App::new();
  let mut tree = Tree::new();

  tree.set_root(lurq::components::Rect::new(100.0, 40.0).background("#22c55e"));
  tree.set_render_engine_factory(move || {
    Box::new(ProfilingCaptureRenderEngine {
      flags: flags.clone(),
      profiling_enabled: false,
    })
  });

  tree.draw_perf_overlay();
  tree.pass(&mut app, &TestSurface);

  assert!(!app.profiling_enabled());
  assert_eq!(*profiling_flags.lock().unwrap(), vec![true]);
  assert!(tree.last_profile().total > std::time::Duration::ZERO);
}

struct ProfilingCaptureRenderEngine {
  flags: Arc<Mutex<Vec<bool>>>,
  profiling_enabled: bool,
}

impl RenderEngine for ProfilingCaptureRenderEngine {
  fn resize(&mut self, _width: u32, _height: u32) {}

  fn set_profiling_enabled(&mut self, enabled: bool) {
    self.profiling_enabled = enabled;
    self.flags.lock().unwrap().push(enabled);
  }

  fn render(&mut self, _list: &RenderList, _window: WindowHandle<'_>, _display: DisplayHandle<'_>) {}
}
