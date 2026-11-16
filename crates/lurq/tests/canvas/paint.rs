use std::{
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
  time::{Duration, Instant},
};

use lurq::{
  animation::Easing,
  app::{App, Tree},
  canvas::{CanvasMotion, CanvasMove},
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

/// A moving canvas of `width` drawn toward `value`, keeping each drawing's progress and where it came from.
fn moving(value: u64, width: f32, drawn: &Arc<std::sync::Mutex<Vec<(Option<u64>, u64, f32)>>>) -> Element {
  let drawn = drawn.clone();
  Canvas::new()
    .software()
    .width(width)
    .height(20.0)
    .paint_moving(
      value,
      CanvasMotion::new(Duration::from_millis(300)).easing(Easing::Linear),
      value,
      value,
      move |canvas, at: CanvasMove<'_, u64>| {
        drawn.lock().unwrap().push((at.from.copied(), *at.to, at.progress));
        canvas.context_2d().fill_rect(0.0, 0.0, 4.0 * at.progress, 4.0);
      },
    )
    .into()
}

/// A tree whose canvases move on a clock the test sets.
struct Clocked {
  app: App,
  tree: Tree,
  start: Instant,
}

impl Clocked {
  fn new() -> Self {
    let mut tree = Tree::new();
    tree.resize(512, 512);
    tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(512.0, 512.0))));
    tree.set_render_engine_factory(|| Box::new(super::Sink));
    Self {
      app: App::new(),
      tree,
      start: Instant::now(),
    }
  }

  /// A pass `millis` after the start, with `root` set where it is given.
  fn at(&mut self, millis: u64, root: Option<Element>) {
    if let Some(root) = root {
      self.tree.set_root(root);
    }
    self
      .tree
      .set_animation_clock(Some(self.start + Duration::from_millis(millis)));
    self.tree.request_redraw();
    self.tree.pass(&mut self.app, &support::TestSurface);
  }
}

#[test]
fn a_moving_canvas_is_drawn_at_each_frame_while_it_moves_and_not_at_rest() {
  let mut stage = Clocked::new();
  let drawn = Arc::new(std::sync::Mutex::new(Vec::new()));
  let take = |drawn: &Arc<std::sync::Mutex<Vec<(Option<u64>, u64, f32)>>>| std::mem::take(&mut *drawn.lock().unwrap());

  stage.at(0, Some(moving(1, 40.0, &drawn)));
  assert_eq!(take(&drawn), [(None, 1, 0.0)], "it enters from nothing");
  assert!(stage.tree.has_moving_canvas(), "and the shell is asked for frames");
  stage.at(150, None);
  assert_eq!(
    take(&drawn),
    [(None, 1, 0.5)],
    "drawn on its own at the next frame, halfway"
  );
  stage.at(300, None);
  assert_eq!(take(&drawn), [(None, 1, 1.0)], "and where it goes at its end");
  assert!(!stage.tree.has_moving_canvas());
  stage.at(400, None);
  stage.at(500, Some(moving(1, 40.0, &drawn)));
  assert!(
    take(&drawn).is_empty(),
    "at rest, frames and renders that change nothing draw nothing"
  );

  stage.at(1000, Some(moving(2, 40.0, &drawn)));
  assert_eq!(
    take(&drawn),
    [(Some(1), 2, 0.0)],
    "a new value moves from the one before"
  );
  stage.at(1100, Some(moving(2, 40.0, &drawn)));
  let frame = take(&drawn);
  assert_eq!(
    frame.len(),
    1,
    "a render that changes nothing while it moves draws the frame once"
  );
  assert_eq!(frame[0].0, Some(1));
  assert!((frame[0].2 - 1.0 / 3.0).abs() < 1e-3, "{frame:?}");
  stage.at(1300, None);
  assert_eq!(take(&drawn), [(None, 2, 1.0)]);
  stage.at(1400, None);
  assert!(take(&drawn).is_empty());
}

#[test]
fn a_resize_or_reduced_motion_stops_a_canvas_where_it_goes() {
  let mut stage = Clocked::new();
  let drawn = Arc::new(std::sync::Mutex::new(Vec::new()));
  let take = |drawn: &Arc<std::sync::Mutex<Vec<(Option<u64>, u64, f32)>>>| std::mem::take(&mut *drawn.lock().unwrap());

  stage.at(0, Some(moving(1, 40.0, &drawn)));
  stage.at(100, Some(moving(1, 60.0, &drawn)));
  let frames = take(&drawn);
  assert_eq!(
    frames.last(),
    Some(&(None, 1, 1.0)),
    "a resize ends the motion: {frames:?}"
  );
  assert!(!stage.tree.has_moving_canvas());
  stage.at(200, Some(moving(2, 80.0, &drawn)));
  assert_eq!(
    take(&drawn),
    [(None, 2, 1.0)],
    "a change while it is resized moves nothing"
  );
  assert!(!stage.tree.has_moving_canvas());

  stage.app.theme().set_reduced_motion(Some(true));
  stage.at(1000, Some(moving(3, 80.0, &drawn)));
  assert_eq!(
    take(&drawn),
    [(None, 3, 1.0)],
    "where motion is reduced, a change is drawn where it goes"
  );
  assert!(!stage.tree.has_moving_canvas());
}

#[test]
fn an_offscreen_canvas_is_drawn_at_once_in_its_own_pixels() {
  use lurq::canvas::CanvasHandle;
  let canvas = CanvasHandle::offscreen(10.0, 4.0, 2.0).expect("a canvas of 10 by 4");
  assert_eq!(canvas.pixel_size(), (20, 8));
  assert!(canvas.is_attached());
  let draw = canvas.context_2d();
  draw.set_fill_style("#ff0000");
  draw.fill_rect(0.0, 0.0, 5.0, 4.0);
  let image = canvas.to_image_data().expect("its pixels");
  assert_eq!((image.width(), image.height()), (20, 8));
  let pixel = |x: usize, y: usize| image.data()[(y * 20 + x) * 4..][..4].to_vec();
  assert_eq!(pixel(2, 2), [255, 0, 0, 255], "drawn at once, in device pixels");
  assert_eq!(pixel(15, 2), [0, 0, 0, 0], "and nothing where it was not drawn");
  let snapshot = canvas
    .snapshot()
    .try_take()
    .expect("ready when it returns")
    .expect("its pixels");
  assert_eq!(snapshot.rgba, image.data());
  assert!(CanvasHandle::offscreen(0.0, 4.0, 1.0).is_none(), "a canvas of nothing");
  assert!(CanvasHandle::offscreen(f32::NAN, 4.0, 1.0).is_none());
}
