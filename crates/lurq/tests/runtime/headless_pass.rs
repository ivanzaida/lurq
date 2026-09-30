use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{App, Tree, events::MouseButton, render_engine::RenderEngine},
  components::{Button, Column, Modal, Root},
  layout::render_list::RenderList,
};
use raw_window_handle::{DisplayHandle, WindowHandle};

use crate::support::pointer_click;

struct CountingEngine(Arc<AtomicUsize>);

impl RenderEngine for CountingEngine {
  fn resize(&mut self, _width: u32, _height: u32) {}

  fn render(&mut self, _list: &RenderList, _window: WindowHandle<'_>, _display: DisplayHandle<'_>) -> bool {
    self.0.fetch_add(1, Ordering::SeqCst);
    true
  }
}

#[test]
fn headless_pass_lays_out_without_a_surface_and_never_renders() {
  let renders = Arc::new(AtomicUsize::new(0));
  let mut tree = Tree::new();
  tree.set_render_engine_factory({
    let renders = renders.clone();
    move || Box::new(CountingEngine(renders.clone()))
  });
  tree.resize(400, 300);
  tree.set_root(Column::new().child(Button::new("Save").id("save").size(100.0, 30.0)));

  let report = tree.pass_headless(&mut App::new());

  assert!(!report.rendered);
  assert_eq!(renders.load(Ordering::SeqCst), 0);
  let bounds = tree.get_element_by_id_mut("save").unwrap().bounds().unwrap();
  assert_eq!((bounds.width, bounds.height), (100.0, 30.0));
}

#[test]
fn focused_element_reports_the_control_that_has_focus() {
  let mut tree = Tree::new();
  tree.set_root(
    Column::new()
      .child(Button::new("Behind").id("behind"))
      .child(Modal::new(Button::new("Inside").id("inside").size(100.0, 30.0)).target(Root)),
  );
  tree.pass_headless(&mut App::new());
  assert!(tree.focused_element().is_none());

  let (x, y) = tree.get_element_by_id_mut("inside").unwrap().bounds().unwrap().center();
  pointer_click(&mut tree, x, y, MouseButton::Left);

  let focused = tree.focused_element().expect("the clicked modal button has focus");
  assert_eq!(focused.id(), Some("inside"));
}
