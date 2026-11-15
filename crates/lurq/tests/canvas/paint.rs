use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{App, Tree},
  components::Canvas,
  layout::{Constraints, Size},
  node::Element,
};

use super::support;

/// A canvas of `width` drawn for `key`, counting its drawings.
fn painted(key: u64, width: f32, drawn: &Arc<AtomicUsize>) -> Element {
  let drawn = drawn.clone();
  Canvas::new()
    .software()
    .width(width)
    .height(20.0)
    .paint(key, move |canvas| {
      drawn.fetch_add(1, Ordering::SeqCst);
      canvas.context_2d().fill_rect(0.0, 0.0, 4.0, 4.0);
    })
    .into()
}

#[test]
fn a_painted_canvas_is_drawn_when_its_key_its_size_or_its_scale_changes_and_not_otherwise() {
  let (mut app, mut tree) = (App::new(), Tree::new());
  tree.resize(512, 512);
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(512.0, 512.0))));
  tree.set_render_engine_factory(|| Box::new(super::Sink));
  let drawn = Arc::new(AtomicUsize::new(0));
  let mut pass = |tree: &mut Tree, root: Element| {
    tree.set_root(root);
    tree.request_redraw();
    tree.pass(&mut app, &support::TestSurface);
  };
  pass(&mut tree, painted(1, 40.0, &drawn));
  assert_eq!(drawn.load(Ordering::SeqCst), 1, "drawn when first laid out");
  pass(&mut tree, painted(1, 40.0, &drawn));
  pass(&mut tree, painted(1, 40.0, &drawn));
  assert_eq!(
    drawn.load(Ordering::SeqCst),
    1,
    "renders that change nothing draw nothing"
  );
  pass(&mut tree, painted(2, 40.0, &drawn));
  assert_eq!(drawn.load(Ordering::SeqCst), 2, "another key");
  pass(&mut tree, painted(2, 60.0, &drawn));
  assert_eq!(drawn.load(Ordering::SeqCst), 3, "another size");
  tree.set_scale_factor(2.0);
  pass(&mut tree, painted(2, 60.0, &drawn));
  assert_eq!(drawn.load(Ordering::SeqCst), 4, "another scale");
}
