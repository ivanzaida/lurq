#![cfg(feature = "canvas")]

mod support;

use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, render_engine::RenderEngine},
  canvas::{
    ArcDirection, BlendMode, CanvasError, CanvasFont, CanvasHandle, Context2D, FillRule, Filter, Gradient,
    MAX_BLUR_RADIUS, MAX_GRADIENT_STOPS, MAX_LAYER_DEPTH, Path2D, Shadow, TextAlign,
  },
  components::{Canvas, Column, Rect},
  core::ElementRef,
  images::ImageData,
  layout::{Constraints, Size, render_list::RenderList},
  node::{Element, transform::Transform2D},
};
use raw_window_handle::{DisplayHandle, WindowHandle};

struct Sink;
impl RenderEngine for Sink {
  fn resize(&mut self, _: u32, _: u32) {}
  fn render(&mut self, _: &RenderList, _: WindowHandle<'_>, _: DisplayHandle<'_>) -> bool {
    true
  }
}

fn setup(w: f32, h: f32) -> (App, Tree, ElementRef) {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(512, 512);
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(512.0, 512.0))));
  tree.set_render_engine_factory(|| Box::new(Sink));
  let reference = ElementRef::new();
  tree.set_root(
    Canvas::new()
      .software()
      .ref_element(reference.clone())
      .width(w)
      .height(h)
      .id("canvas"),
  );
  assert!(reference.as_canvas().is_none());
  tree.pass(&mut app, &support::TestSurface);
  (app, tree, reference)
}
fn pixel(canvas: &CanvasHandle, x: u32, y: u32) -> [u8; 4] {
  let snapshot = canvas.snapshot().try_take().unwrap().unwrap();
  snapshot.rgba[((y * snapshot.width + x) * 4) as usize..((y * snapshot.width + x) * 4 + 4) as usize]
    .try_into()
    .unwrap()
}

#[test]
fn refs_produce_owned_shared_contexts_and_find_access() {
  let (_app, mut tree, reference) = setup(64.0, 64.0);
  let canvas = reference.as_canvas().unwrap();
  assert_eq!(
    reference.mutable().as_canvas().unwrap().surface_id(),
    canvas.surface_id()
  );
  assert_eq!(
    tree
      .get_element_by_id("canvas")
      .unwrap()
      .as_canvas()
      .unwrap()
      .surface_id(),
    canvas.surface_id()
  );
  assert_eq!(
    tree
      .find_element(|n| n.id() == Some("canvas"))
      .unwrap()
      .as_canvas()
      .unwrap()
      .surface_id(),
    canvas.surface_id()
  );
  let first = canvas.context_2d();
  let second = canvas.context_2d();
  first.set_fill_style("#ff0000");
  second.fill_rect(0.0, 0.0, 20.0, 20.0);
  assert_eq!(pixel(&canvas, 5, 5), [255, 0, 0, 255]);
  first.set_fill_style("#0000ff");
  first.fill_rect(30.0, 0.0, 20.0, 20.0);
  assert_eq!(pixel(&canvas, 5, 5), [255, 0, 0, 255]);
  assert_eq!(pixel(&canvas, 35, 5), [0, 0, 255, 255]);
}

#[test]
fn same_size_reconciliation_preserves_pixels_and_context_state() {
  let (mut app, mut tree, reference) = setup(64.0, 64.0);
  let old = reference.as_canvas().unwrap();
  let draw = old.context_2d();
  draw.set_fill_style("#ff0000");
  draw.fill_rect(2.0, 2.0, 10.0, 10.0);
  draw.translate(8.0, 0.0);
  tree.set_root(
    Canvas::new()
      .software()
      .ref_element(reference.clone())
      .width(64.0)
      .height(64.0),
  );
  tree.pass(&mut app, &support::TestSurface);
  let new = reference.as_canvas().unwrap();
  assert_eq!(old.surface_id(), new.surface_id());
  assert_eq!(pixel(&new, 5, 5), [255, 0, 0, 255]);
  assert_eq!(new.context_2d().get_transform().tx, 8.0);
}

#[test]
fn removal_detaches_refs_and_old_context_never_retargets() {
  let (mut app, mut tree, reference) = setup(64.0, 64.0);
  let old = reference.as_canvas().unwrap();
  let draw = old.context_2d();
  tree.set_root(Rect::new(64.0, 64.0).ref_element(reference.clone()));
  assert!(reference.as_canvas().is_none());
  assert!(!old.is_attached());
  tree.pass(&mut app, &support::TestSurface);
  tree.set_root(
    Canvas::new()
      .software()
      .ref_element(reference.clone())
      .width(64.0)
      .height(64.0),
  );
  tree.pass(&mut app, &support::TestSurface);
  let new = reference.as_canvas().unwrap();
  assert_ne!(old.surface_id(), new.surface_id());
  draw.set_fill_style("#ff0000");
  draw.fill_rect(0.0, 0.0, 64.0, 64.0);
  assert_eq!(pixel(&old, 10, 10), [255, 0, 0, 255]);
  assert_eq!(pixel(&new, 10, 10), [0, 0, 0, 0]);
  drop(tree);
  assert!(reference.as_canvas().is_none());
  assert!(!new.is_attached());
  assert_eq!(
    draw.canvas().snapshot().try_take().unwrap().unwrap().rgba.len(),
    64 * 64 * 4
  );
}

#[test]
fn changing_ref_binding_preserves_surface_and_invalidates_old_ref() {
  let (mut app, mut tree, reference) = setup(64.0, 64.0);
  let id = reference.as_canvas().unwrap().surface_id();
  let replacement = ElementRef::new();
  tree.set_root(
    Canvas::new()
      .software()
      .ref_element(replacement.clone())
      .width(64.0)
      .height(64.0),
  );
  tree.pass(&mut app, &support::TestSurface);
  assert!(reference.as_canvas().is_none());
  assert_eq!(replacement.as_canvas().unwrap().surface_id(), id);
}

#[test]
fn clear_rect_respects_transform_and_clip_but_clear_and_reset_are_distinct() {
  let (_app, _tree, reference) = setup(40.0, 40.0);
  let canvas = reference.as_canvas().unwrap();
  let d = canvas.context_2d();
  d.set_fill_style("#ff0000");
  d.fill_rect(0.0, 0.0, 40.0, 40.0);
  d.begin_path();
  d.rect(10.0, 10.0, 10.0, 10.0);
  d.clip();
  d.translate(10.0, 0.0);
  d.set_global_alpha(0.1);
  d.clear_rect(0.0, 0.0, 20.0, 40.0);
  assert_eq!(pixel(&canvas, 15, 15)[3], 0);
  assert_eq!(pixel(&canvas, 5, 15)[3], 255);
  d.clear();
  assert!(
    canvas
      .snapshot()
      .try_take()
      .unwrap()
      .unwrap()
      .rgba
      .iter()
      .all(|v| *v == 0)
  );
  assert_eq!(d.get_transform().tx, 10.0);
  assert_eq!(d.global_alpha(), 0.1);
  d.reset();
  assert_eq!(d.get_transform(), Transform2D::IDENTITY);
  assert_eq!(d.global_alpha(), 1.0);
  d.fill_rect(0.0, 0.0, 5.0, 5.0);
  assert_eq!(pixel(&canvas, 2, 2), [0, 0, 0, 255]);
}

#[test]
fn save_restore_does_not_restore_the_path_or_erase_pixels() {
  let (_app, _tree, r) = setup(40.0, 40.0);
  let c = r.as_canvas().unwrap();
  let d = c.context_2d();
  d.rect(0.0, 0.0, 10.0, 10.0);
  d.set_fill_style("#ff0000");
  d.save();
  d.begin_path();
  d.rect(20.0, 0.0, 10.0, 10.0);
  d.set_fill_style("#0000ff");
  d.restore();
  d.fill();
  assert_eq!(pixel(&c, 5, 5)[3], 0);
  assert_eq!(pixel(&c, 25, 5), [255, 0, 0, 255]);
  d.restore();
  assert_eq!(d.fill_style().color().unwrap().r(), 255);
}

#[test]
fn current_paths_capture_transforms_but_reusable_paths_transform_at_use() {
  let (_app, _tree, r) = setup(80.0, 40.0);
  let c = r.as_canvas().unwrap();
  let d = c.context_2d();
  d.translate(20.0, 0.0);
  d.rect(0.0, 0.0, 10.0, 10.0);
  d.reset_transform();
  d.fill();
  assert_eq!(pixel(&c, 25, 5)[3], 255);
  assert_eq!(pixel(&c, 5, 5)[3], 0);
  let mut path = Path2D::new();
  path.rect(0.0, 0.0, 10.0, 10.0);
  d.translate(50.0, 0.0);
  d.fill_path(&path, FillRule::NonZero);
  assert_eq!(pixel(&c, 55, 5)[3], 255);
  path.rect(0.0, 20.0, 20.0, 10.0);
  assert_eq!(pixel(&c, 55, 25)[3], 0);
}

#[test]
fn fill_rules_clip_curves_and_geometry_queries() {
  let (_app, _tree, r) = setup(80.0, 80.0);
  let c = r.as_canvas().unwrap();
  let d = c.context_2d();
  let mut ring = Path2D::new();
  ring.rect(5.0, 5.0, 70.0, 70.0);
  ring.rect(20.0, 20.0, 40.0, 40.0);
  d.fill_path(&ring, FillRule::EvenOdd);
  assert_eq!(pixel(&c, 10, 10)[3], 255);
  assert_eq!(pixel(&c, 40, 40)[3], 0);
  assert!(!d.is_point_in_path2d(&ring, 40.0, 40.0, FillRule::EvenOdd));
  assert!(d.is_point_in_path2d(&ring, 40.0, 40.0, FillRule::NonZero));
  d.clear();
  d.begin_path();
  d.arc(40.0, 40.0, 20.0, 0.0, std::f32::consts::TAU, ArcDirection::Clockwise)
    .unwrap();
  d.clip();
  d.fill_rect(0.0, 0.0, 80.0, 80.0);
  assert_eq!(pixel(&c, 40, 40)[3], 255);
  assert_eq!(pixel(&c, 22, 22)[3], 0);
  assert!(
    c.snapshot()
      .try_take()
      .unwrap()
      .unwrap()
      .rgba
      .chunks_exact(4)
      .any(|p| p[3] > 0 && p[3] < 255)
  );
  assert!(d.is_point_in_path(40.0, 40.0, FillRule::NonZero));
}

#[test]
fn nonuniform_strokes_and_dash_hit_tests() {
  let (_app, _tree, r) = setup(100.0, 60.0);
  let c = r.as_canvas().unwrap();
  let d = c.context_2d();
  let mut path = Path2D::new();
  path.move_to(10.0, 10.0);
  path.line_to(10.0, 40.0);
  d.scale(3.0, 1.0);
  d.set_line_width(4.0);
  d.stroke_path(&path);
  assert_eq!(pixel(&c, 25, 20)[3], 255);
  assert_eq!(pixel(&c, 22, 20)[3], 0);
  assert!(d.is_point_in_stroke_path(&path, 25.0, 20.0));
  d.reset_transform();
  d.begin_path();
  d.move_to(0.0, 50.0);
  d.line_to(90.0, 50.0);
  d.set_line_dash(&[10.0, 10.0]);
  assert!(d.is_point_in_stroke(5.0, 50.0));
  assert!(!d.is_point_in_stroke(15.0, 50.0));
}

#[test]
fn logical_resize_resets_but_scale_change_preserves_pixels_and_state() {
  let (mut app, mut tree, r) = setup(40.0, 40.0);
  let c = r.as_canvas().unwrap();
  let d = c.context_2d();
  d.set_fill_style("#ff0000");
  d.fill_rect(0.0, 0.0, 40.0, 40.0);
  d.translate(3.0, 4.0);
  let notifications = Arc::new(Mutex::new(Vec::new()));
  let seen = notifications.clone();
  let _observer = c.observe_metrics(move |metrics| seen.lock().unwrap().push(metrics));
  tree.set_scale_factor(2.0);
  tree.pass(&mut app, &support::TestSurface);
  assert_eq!(c.pixel_size(), (80, 80));
  assert_eq!(pixel(&c, 20, 20), [255, 0, 0, 255]);
  assert_eq!(d.get_transform().tx, 3.0);
  tree.set_root(Canvas::new().software().ref_element(r.clone()).width(60.0).height(40.0));
  tree.pass(&mut app, &support::TestSurface);
  assert_eq!(c.surface_id(), r.as_canvas().unwrap().surface_id());
  assert_eq!(c.pixel_size(), (120, 80));
  assert!(c.snapshot().try_take().unwrap().unwrap().rgba.iter().all(|v| *v == 0));
  assert_eq!(d.get_transform(), Transform2D::IDENTITY);
  assert_eq!(notifications.lock().unwrap().len(), 3);
}

#[test]
fn detached_worker_writes_are_bounded_and_visible_worker_writes_wake_once() {
  fn thread_safe<T: Send + Sync + Clone>() {}
  thread_safe::<Context2D>();
  let (mut app, mut tree, r) = setup(32.0, 32.0);
  let c = r.as_canvas().unwrap();
  let d = c.context_2d();
  let wake = Arc::new(AtomicUsize::new(0));
  let wake_count = wake.clone();
  tree.set_canvas_waker(move || {
    wake_count.fetch_add(1, Ordering::SeqCst);
  });
  let worker = d.clone();
  std::thread::spawn(move || {
    for _ in 0..10 {
      worker.fill_rect(0.0, 0.0, 5.0, 5.0);
    }
  })
  .join()
  .unwrap();
  assert_eq!(wake.load(Ordering::SeqCst), 1);
  assert!(tree.needs_redraw());
  let report = tree.pass(&mut app, &support::TestSurface);
  assert!(report.rendered);
  assert!(!report.layout_updated);
  assert!(!tree.needs_redraw());
  assert!(!tree.pass(&mut app, &support::TestSurface).required);
  drop(tree);
  let before = wake.load(Ordering::SeqCst);
  for _ in 0..1000 {
    d.fill_rect(0.0, 0.0, 5.0, 5.0);
  }
  assert_eq!(wake.load(Ordering::SeqCst), before);
  assert_eq!(c.snapshot().try_take().unwrap().unwrap().rgba.len(), 32 * 32 * 4);
}

#[test]
fn submitted_alpha_is_not_reapplied_on_window_redraw() {
  let (mut app, mut tree, r) = setup(32.0, 32.0);
  let c = r.as_canvas().unwrap();
  let d = c.context_2d();
  d.set_fill_style("#ff000080");
  d.fill_rect(0.0, 0.0, 32.0, 32.0);
  d.fill_rect(0.0, 0.0, 16.0, 32.0);
  let expected = c.snapshot().try_take().unwrap().unwrap().rgba;
  for _ in 0..4 {
    tree.request_redraw();
    tree.pass(&mut app, &support::TestSurface);
  }
  assert_eq!(c.snapshot().try_take().unwrap().unwrap().rgba, expected);
  assert!((i32::from(pixel(&c, 8, 8)[3]) - 192).abs() <= 1);
  assert_eq!(pixel(&c, 24, 8)[3], 128);
}

#[test]
fn static_images_crop_and_copy_without_live_source_dependencies() {
  let (_app, _tree, r) = setup(40.0, 40.0);
  let c = r.as_canvas().unwrap();
  let d = c.context_2d();
  d.set_image_smoothing_enabled(false);
  let image = ImageData::from_rgba(vec![255, 0, 0, 255, 0, 0, 255, 255], 2, 1);
  d.draw_image_region(&image, [1.0, 0.0, 1.0, 1.0], [0.0, 0.0, 20.0, 20.0])
    .unwrap();
  drop(image);
  assert_eq!(pixel(&c, 10, 10), [0, 0, 255, 255]);
  assert_eq!(pixel(&c, 30, 10)[3], 0);
  let streaming = ImageData::streaming_rgba(vec![0; 4], 1, 1);
  assert_eq!(d.draw_image(&streaming, 0.0, 0.0), Err(CanvasError::UnsupportedImage));
}

#[test]
fn text_shapes_measures_and_renders_with_alignment() {
  let (_app, _tree, r) = setup(240.0, 80.0);
  let c = r.as_canvas().unwrap();
  let d = c.context_2d();
  d.set_font(CanvasFont::new("Segoe UI", 24.0));
  let m = d.measure_text("Canvas").unwrap();
  assert!(m.width > 40.0);
  assert!(m.actual_bounding_box_ascent > 0.0);
  d.set_text_align(TextAlign::Center);
  d.fill_text("Canvas", 120.0, 45.0).unwrap();
  assert!(
    c.snapshot()
      .try_take()
      .unwrap()
      .unwrap()
      .rgba
      .chunks_exact(4)
      .any(|p| p[3] > 0)
  );
  let centered = d.measure_text("Canvas").unwrap();
  assert!((centered.width - m.width).abs() < 0.01);
  assert!((centered.actual_bounding_box_left - m.actual_bounding_box_left - m.width * 0.5).abs() < 0.01);
  assert!(d.measure_text("مرحبا").unwrap().width > 0.0);
}

#[test]
fn text_letter_spacing_widens_measured_advances() {
  // The weight-probe "a" advances 5px at 10px; spacing follows every glyph.
  let mut app = App::new();
  app.install_fonts(
    [include_bytes!("assets/weight_probe/LurqWeightProbe-Regular.ttf").to_vec()],
    std::iter::empty::<(&str, &str)>(),
  );
  let mut tree = Tree::new();
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(512.0, 512.0))));
  tree.set_render_engine_factory(|| Box::new(Sink));
  let r = ElementRef::new();
  tree.set_root(
    Canvas::new()
      .software()
      .ref_element(r.clone())
      .width(100.0)
      .height(40.0),
  );
  tree.pass(&mut app, &support::TestSurface);
  let d = r.as_canvas().unwrap().context_2d();
  for (letter_spacing, width) in [(0.0, 20.0), (1.5, 26.0), (-1.0, 16.0)] {
    d.set_font(CanvasFont {
      letter_spacing,
      ..CanvasFont::new("Lurq Weight Probe", 10.0)
    });
    let measured = d.measure_text("aaaa").unwrap().width;
    assert!((measured - width).abs() < 0.01, "{letter_spacing}: {measured}");
  }
  d.scale(2.0, 2.0);
  assert!(
    (d.measure_text("aaaa").unwrap().width - 16.0).abs() < 0.01,
    "measured in user space"
  );
}

#[test]
fn padding_and_ancestor_transform_have_a_checked_pointer_conversion() {
  let (mut app, mut tree, r) = setup(100.0, 80.0);
  tree.set_root(
    Column::new()
      .padding(10.0)
      .transform(Transform2D::translate(20.0, 30.0))
      .child(
        Canvas::new()
          .software()
          .ref_element(r.clone())
          .width(100.0)
          .height(80.0)
          .padding(5.0),
      ),
  );
  tree.pass(&mut app, &support::TestSurface);
  let c = r.as_canvas().unwrap();
  assert_eq!(c.size(), Size::new(90.0, 70.0));
  let p = c.point_from_window(35.0, 45.0).unwrap();
  assert!(p.0.abs() < 0.01 && p.1.abs() < 0.01, "{p:?}");
  c.context_2d().scale(4.0, 4.0);
  assert_eq!(c.point_from_window(39.0, 49.0), Some((4.0, 4.0)));
}

#[test]
fn zero_size_and_invalid_inputs_do_not_spin_or_panic() {
  let (mut app, mut tree, r) = setup(0.0, 0.0);
  let c = r.as_canvas().unwrap();
  let d = c.context_2d();
  assert_eq!(c.pixel_size(), (0, 0));
  d.fill_rect(0.0, 0.0, 30.0, 30.0);
  assert!(!tree.needs_redraw());
  d.set_fill_style("#💥");
  d.set_fill_style("#ggg");
  d.set_line_width(f32::NAN);
  d.set_global_alpha(2.0);
  assert_eq!(d.line_width(), 1.0);
  assert_eq!(d.global_alpha(), 1.0);
  assert_eq!(
    d.arc(0.0, 0.0, -1.0, 0.0, 1.0, ArcDirection::Clockwise),
    Err(CanvasError::InvalidGeometry)
  );
  tree.pass(&mut app, &support::TestSurface);
  assert!(!tree.needs_redraw());
}

struct InitialPaint {
  reference: ElementRef,
}
impl Component for InitialPaint {
  type Props = ();
  fn create(ctx: &mut Ctx) -> Self {
    Self {
      reference: ctx.element_ref(),
    }
  }
  fn render(&self, _: &mut Ctx) -> impl Into<Element> {
    Canvas::new()
      .software()
      .ref_element(self.reference.clone())
      .width(40.0)
      .height(40.0)
  }
  fn after_layout(&self) {
    self
      .reference
      .as_canvas()
      .expect("ready after layout")
      .context_2d()
      .fill_rect(0.0, 0.0, 40.0, 40.0);
  }
}
#[test]
fn after_layout_and_rect_observers_can_access_the_context() {
  let (mut app, mut tree, r) = setup(32.0, 32.0);
  let count = Arc::new(AtomicUsize::new(0));
  let calls = count.clone();
  let captured = r.clone();
  r.observe_rect(move |_| {
    assert!(captured.as_canvas().is_some());
    calls.fetch_add(1, Ordering::Relaxed);
  });
  tree.set_root(Canvas::new().software().ref_element(r.clone()).width(40.0).height(40.0));
  tree.pass(&mut app, &support::TestSurface);
  assert!(count.load(Ordering::Relaxed) > 0);
  tree.mount_root::<InitialPaint>(&mut app, ());
  tree.pass(&mut app, &support::TestSurface);
  let c = tree.root().unwrap().as_canvas().unwrap();
  assert_eq!(pixel(&c, 20, 20)[3], 255);
}

#[test]
fn keyed_reordering_preserves_surfaces_and_key_changes_replace_them() {
  let (mut app, mut tree, a) = setup(32.0, 32.0);
  let b = ElementRef::new();
  let make_a = || {
    Canvas::new()
      .software()
      .key("a")
      .ref_element(a.clone())
      .width(32.0)
      .height(32.0)
  };
  let make_b = || {
    Canvas::new()
      .software()
      .key("b")
      .ref_element(b.clone())
      .width(32.0)
      .height(32.0)
  };
  tree.set_root(Column::new().child(make_a()).child(make_b()));
  tree.pass(&mut app, &support::TestSurface);
  let first = a.as_canvas().unwrap();
  let second = b.as_canvas().unwrap();
  first.context_2d().fill_rect(0.0, 0.0, 32.0, 32.0);
  tree.set_root(Column::new().child(make_b()).child(make_a()));
  tree.pass(&mut app, &support::TestSurface);
  assert_eq!(first.surface_id(), a.as_canvas().unwrap().surface_id());
  assert_eq!(second.surface_id(), b.as_canvas().unwrap().surface_id());
  assert_eq!(pixel(&first, 10, 10)[3], 255);
  tree.set_root(
    Column::new().child(make_b()).child(
      Canvas::new()
        .software()
        .key("new")
        .ref_element(a.clone())
        .width(32.0)
        .height(32.0),
    ),
  );
  tree.pass(&mut app, &support::TestSurface);
  assert_ne!(first.surface_id(), a.as_canvas().unwrap().surface_id());
  assert!(!first.is_attached());
}

#[test]
fn cloned_elements_have_independent_surfaces() {
  let (mut app, mut tree, _) = setup(32.0, 32.0);
  let element: Element = Canvas::new().software().width(32.0).height(32.0).into();
  tree.set_root(Column::new().child(element.clone()).child(element));
  tree.pass(&mut app, &support::TestSurface);
  let canvases: Vec<_> = tree
    .root()
    .unwrap()
    .children()
    .iter()
    .map(|n| n.as_canvas().unwrap())
    .collect();
  assert_ne!(canvases[0].surface_id(), canvases[1].surface_id());
  canvases[0].context_2d().fill_rect(0.0, 0.0, 32.0, 32.0);
  assert_eq!(pixel(&canvases[1], 10, 10)[3], 0);
}

struct ReactiveCanvas {
  reference: ElementRef,
  count: lurq::core::Signal<i32>,
}
impl Component for ReactiveCanvas {
  type Props = ();
  fn create(ctx: &mut Ctx) -> Self {
    Self {
      reference: ctx.element_ref(),
      count: ctx.signal(0),
    }
  }
  fn render(&self, _: &mut Ctx) -> impl Into<Element> {
    let value = self.count.get();
    let count = self.count.clone();
    Column::new()
      .child(
        Canvas::new()
          .software()
          .ref_element(self.reference.clone())
          .id("retained")
          .width(32.0)
          .height(32.0),
      )
      .child(lurq::components::Text::new(&value.to_string()))
      .child(
        Rect::new(32.0, 32.0)
          .id("increment")
          .on_click(move |_| count.update(|v| *v += 1)),
      )
  }
}
#[test]
fn component_rerender_keeps_surface_and_paint() {
  let (mut app, mut tree, _) = setup(32.0, 32.0);
  tree.mount_root::<ReactiveCanvas>(&mut app, ());
  tree.pass(&mut app, &support::TestSurface);
  let old = tree.get_element_by_id("retained").unwrap().as_canvas().unwrap();
  old.context_2d().fill_rect(0.0, 0.0, 32.0, 32.0);
  tree.get_element_by_id_mut("increment").unwrap().click();
  tree.pass(&mut app, &support::TestSurface);
  let new = tree.get_element_by_id("retained").unwrap().as_canvas().unwrap();
  assert_eq!(old.surface_id(), new.surface_id());
  assert_eq!(pixel(&new, 10, 10)[3], 255);
}

#[test]
fn saved_clips_survive_display_scale_changes() {
  let (mut app, mut tree, r) = setup(40.0, 40.0);
  let c = r.as_canvas().unwrap();
  let d = c.context_2d();
  d.rect(0.0, 0.0, 20.0, 40.0);
  d.clip();
  d.save();
  d.begin_path();
  d.rect(0.0, 0.0, 40.0, 20.0);
  d.clip();
  tree.set_scale_factor(2.0);
  tree.pass(&mut app, &support::TestSurface);
  d.fill_rect(0.0, 0.0, 40.0, 40.0);
  assert_eq!(pixel(&c, 20, 20)[3], 255);
  assert_eq!(pixel(&c, 20, 60)[3], 0);
  d.restore();
  d.fill_rect(0.0, 0.0, 40.0, 40.0);
  assert_eq!(pixel(&c, 20, 60)[3], 255);
  assert_eq!(pixel(&c, 60, 20)[3], 0);
}

#[test]
fn failed_presentation_retries_without_repeating_alpha() {
  struct FailOnce(bool);
  impl RenderEngine for FailOnce {
    fn resize(&mut self, _: u32, _: u32) {}
    fn render(&mut self, _: &RenderList, _: WindowHandle<'_>, _: DisplayHandle<'_>) -> bool {
      let was_failed = self.0;
      self.0 = true;
      was_failed
    }
  }
  let (mut app, mut tree, r) = setup(32.0, 32.0);
  let c = r.as_canvas().unwrap();
  let d = c.context_2d();
  tree.set_render_engine_factory(|| Box::new(FailOnce(false)));
  d.set_fill_style("#ff000080");
  d.fill_rect(0.0, 0.0, 32.0, 32.0);
  assert!(!tree.pass(&mut app, &support::TestSurface).rendered);
  assert!(tree.needs_redraw());
  assert!(tree.pass(&mut app, &support::TestSurface).rendered);
  assert_eq!(pixel(&c, 10, 10)[3], 128);
  assert!(!tree.needs_redraw());
}

#[test]
fn pointer_events_are_logical_at_high_display_scale() {
  let (mut app, mut tree, r) = setup(64.0, 64.0);
  let reference = r.clone();
  let point = Arc::new(Mutex::new(None));
  let captured = point.clone();
  tree.set_root(
    Canvas::new()
      .software()
      .ref_element(r.clone())
      .width(64.0)
      .height(64.0)
      .padding(8.0)
      .on_click(move |event: lurq::app::events::MouseEvent| {
        *captured.lock().unwrap() = reference.as_canvas().unwrap().point_from_window(event.x, event.y);
      }),
  );
  tree.set_scale_factor(2.0);
  tree.pass(&mut app, &support::TestSurface);
  support::pointer_click(&mut tree, 40.0, 48.0, lurq::app::events::MouseButton::Left);
  assert_eq!(*point.lock().unwrap(), Some((12.0, 16.0)));
}

#[test]
fn culled_canvas_writes_persist_without_an_idle_redraw_loop() {
  let (mut app, mut tree, r) = setup(32.0, 32.0);
  tree.set_root(
    Canvas::new()
      .software()
      .ref_element(r.clone())
      .width(32.0)
      .height(32.0)
      .offset(600.0, 600.0),
  );
  tree.pass(&mut app, &support::TestSurface);
  let c = r.as_canvas().unwrap();
  c.context_2d().fill_rect(0.0, 0.0, 32.0, 32.0);
  tree.pass(&mut app, &support::TestSurface);
  assert!(!tree.needs_redraw());
  assert_eq!(pixel(&c, 10, 10)[3], 255);
  tree.set_root(Canvas::new().software().ref_element(r).width(32.0).height(32.0));
  tree.pass(&mut app, &support::TestSurface);
  assert_eq!(pixel(&c, 10, 10)[3], 255);
}

#[test]
fn image_cropping_and_negative_extents_preserve_direction() {
  let (_app, _tree, r) = setup(40.0, 20.0);
  let c = r.as_canvas().unwrap();
  let d = c.context_2d();
  let image = ImageData::from_rgba(vec![255, 0, 0, 255, 0, 0, 255, 255], 2, 1);
  d.set_image_smoothing_enabled(false);
  d.draw_image_region(&image, [-1.0, 0.0, 2.0, 1.0], [0.0, 0.0, 20.0, 10.0])
    .unwrap();
  assert_eq!(pixel(&c, 5, 5)[3], 0);
  assert_eq!(pixel(&c, 15, 5), [255, 0, 0, 255]);
  d.draw_image_region(&image, [2.0, 0.0, -2.0, 1.0], [40.0, 10.0, -20.0, 10.0])
    .unwrap();
  assert_eq!(pixel(&c, 25, 15), [255, 0, 0, 255]);
  assert_eq!(pixel(&c, 35, 15), [0, 0, 255, 255]);
}

#[test]
fn negative_rounded_rectangles_reverse_winding_and_clear_ignores_zero_alpha() {
  let (_app, _tree, r) = setup(40.0, 40.0);
  let c = r.as_canvas().unwrap();
  let d = c.context_2d();
  d.rect(0.0, 0.0, 40.0, 40.0);
  d.round_rect(30.0, 10.0, -20.0, 20.0, 3.0).unwrap();
  d.fill();
  assert_eq!(pixel(&c, 20, 20)[3], 0);
  assert_eq!(pixel(&c, 5, 5)[3], 255);
  d.set_global_alpha(0.0);
  d.clear_rect(0.0, 0.0, 10.0, 10.0);
  assert_eq!(pixel(&c, 5, 5)[3], 0);
}

#[test]
fn oversized_scale_keeps_existing_pixels_and_reports_failure() {
  let (mut app, mut tree, r) = setup(32.0, 32.0);
  let c = r.as_canvas().unwrap();
  let d = c.context_2d();
  d.fill_rect(0.0, 0.0, 32.0, 32.0);
  tree.set_scale_factor(1000.0);
  tree.pass(&mut app, &support::TestSurface);
  assert_eq!(c.pixel_size(), (32, 32));
  assert_eq!(pixel(&c, 10, 10)[3], 255);
  assert_eq!(c.status().error, Some(CanvasError::SurfaceTooLarge));
}

#[test]
fn writes_after_frame_snapshot_schedule_another_frame() {
  struct LateWrite(Option<Context2D>);
  impl RenderEngine for LateWrite {
    fn resize(&mut self, _: u32, _: u32) {}
    fn render(&mut self, _: &RenderList, _: WindowHandle<'_>, _: DisplayHandle<'_>) -> bool {
      if let Some(draw) = self.0.take() {
        draw.fill_rect(0.0, 0.0, 10.0, 10.0);
      }
      true
    }
  }
  let (mut app, mut tree, r) = setup(32.0, 32.0);
  let c = r.as_canvas().unwrap();
  let draw = c.context_2d();
  tree.set_render_engine_factory(move || Box::new(LateWrite(Some(draw.clone()))));
  tree.request_redraw();
  tree.pass(&mut app, &support::TestSurface);
  assert!(tree.needs_redraw());
  assert_eq!(pixel(&c, 5, 5)[3], 255);
  assert!(tree.pass(&mut app, &support::TestSurface).rendered);
  assert!(!tree.needs_redraw());
}

#[test]
fn linear_gradients_run_across_their_box_and_follow_the_transform() {
  let (_app, _tree, r) = setup(120.0, 40.0);
  let c = r.as_canvas().unwrap();
  let d = c.context_2d();
  let ramp = Gradient::linear().stop(0.0, "#ff0000").stop(1.0, "#0000ff");
  d.set_fill_style(ramp.clone().in_box(0.0, 0.0, 120.0, 40.0));
  d.fill_rect(0.0, 0.0, 120.0, 40.0);
  let left = pixel(&c, 2, 20);
  let right = pixel(&c, 117, 20);
  let middle = pixel(&c, 60, 20);
  assert!(left[0] > 240 && left[2] < 16, "{left:?}");
  assert!(right[2] > 240 && right[0] < 16, "{right:?}");
  assert!(
    middle[0] > 100 && middle[2] > 100,
    "the middle mixes both stops: {middle:?}"
  );
  // A quarter turn puts the same ramp along y instead of x.
  d.clear();
  d.set_fill_style(ramp.rotation(std::f32::consts::FRAC_PI_2).in_box(0.0, 0.0, 120.0, 40.0));
  d.fill_rect(0.0, 0.0, 120.0, 40.0);
  assert!(pixel(&c, 60, 2)[0] > 200, "{:?}", pixel(&c, 60, 2));
  assert!(pixel(&c, 60, 37)[2] > 200, "{:?}", pixel(&c, 60, 37));
}

#[test]
fn radial_and_angular_gradients_read_their_own_parameter() {
  let (_app, _tree, r) = setup(80.0, 80.0);
  let c = r.as_canvas().unwrap();
  let d = c.context_2d();
  d.set_fill_style(
    Gradient::radial()
      .stop(0.0, "#ffffff")
      .stop(1.0, "#000000")
      .in_box(0.0, 0.0, 80.0, 80.0),
  );
  d.fill_rect(0.0, 0.0, 80.0, 80.0);
  assert!(pixel(&c, 40, 40)[0] > 240, "the centre is the first stop");
  assert!(pixel(&c, 40, 4)[0] < 40, "the rim is the last stop");
  assert!(pixel(&c, 40, 22)[0].abs_diff(128) < 40, "and it falls off with radius");
  d.clear();
  d.set_fill_style(
    Gradient::angular()
      .stop(0.0, "#ff0000")
      .stop(0.5, "#00ff00")
      .stop(1.0, "#ff0000")
      .in_box(0.0, 0.0, 80.0, 80.0),
  );
  d.fill_rect(0.0, 0.0, 80.0, 80.0);
  // t is 0 along +x and half a turn to its left, where the middle stop is.
  assert!(pixel(&c, 76, 40)[0] > 200, "{:?}", pixel(&c, 76, 40));
  assert!(pixel(&c, 4, 40)[1] > 200, "{:?}", pixel(&c, 4, 40));
}

#[test]
fn gradients_paint_strokes_and_an_invalid_one_paints_nothing() {
  let (_app, _tree, r) = setup(60.0, 60.0);
  let c = r.as_canvas().unwrap();
  let d = c.context_2d();
  d.set_line_width(8.0);
  d.set_stroke_style(
    Gradient::linear()
      .stop(0.0, "#00ff00")
      .stop(1.0, "#00ff00")
      .in_box(0.0, 0.0, 60.0, 60.0),
  );
  d.stroke_rect(10.0, 10.0, 40.0, 40.0);
  assert!(pixel(&c, 30, 10)[1] > 200, "the stroke takes the gradient");
  assert_eq!(pixel(&c, 30, 30)[3], 0, "and only the stroke");
  d.clear();
  let one_stop = Gradient::linear().stop(0.0, "#ff0000");
  assert!(!one_stop.is_valid());
  d.set_fill_style(one_stop.in_box(0.0, 0.0, 60.0, 60.0));
  d.fill_rect(0.0, 0.0, 60.0, 60.0);
  assert_eq!(pixel(&c, 30, 30)[3], 0, "an unusable paint draws nothing at all");
  let too_many = (0..MAX_GRADIENT_STOPS + 4).fold(Gradient::linear(), |g, i| {
    g.stop(i as f32 / (MAX_GRADIENT_STOPS + 4) as f32, "#123456")
  });
  assert_eq!(
    too_many.stops().len(),
    MAX_GRADIENT_STOPS,
    "stops are capped, not grown"
  );
}

#[test]
fn an_outer_shadow_falls_behind_the_shape_and_an_inner_one_inside_it() {
  let (_app, _tree, r) = setup(120.0, 120.0);
  let c = r.as_canvas().unwrap();
  let d = c.context_2d();
  d.set_shadow(Some(
    Shadow::new(lurq::node::color::Color::new(0, 0, 0, 255))
      .offset(10.0, 10.0)
      .blur(8.0),
  ));
  d.set_fill_style("#ffffff");
  d.fill_rect(30.0, 30.0, 40.0, 40.0);
  assert_eq!(pixel(&c, 50, 50), [255, 255, 255, 255], "the shape is not darkened");
  let cast = pixel(&c, 76, 76);
  assert!(
    cast[3] > 20 && cast[0] < 80,
    "the shadow falls down and right: {cast:?}"
  );
  assert_eq!(pixel(&c, 10, 10)[3], 0, "and not up and left");
  d.clear();
  d.set_shadow(Some(
    Shadow::new(lurq::node::color::Color::new(0, 0, 0, 255))
      .offset(8.0, 8.0)
      .blur(6.0)
      .inset(true),
  ));
  d.fill_rect(30.0, 30.0, 60.0, 60.0);
  let inside_edge = pixel(&c, 34, 34);
  let inside_far = pixel(&c, 84, 84);
  assert!(
    inside_edge[0] < 160,
    "the inner shadow darkens the near edge: {inside_edge:?}"
  );
  assert!(inside_far[0] > 230, "and not the far one: {inside_far:?}");
  assert_eq!(pixel(&c, 20, 20)[3], 0, "an inner shadow never leaves the shape");
}

#[test]
fn a_layer_blur_spreads_a_shape_beyond_its_own_edge() {
  let (_app, _tree, r) = setup(100.0, 100.0);
  let c = r.as_canvas().unwrap();
  let d = c.context_2d();
  d.set_fill_style("#ff0000");
  d.set_filter(Filter::Blur(12.0));
  assert_eq!(d.filter(), Filter::Blur(12.0));
  d.fill_rect(30.0, 30.0, 40.0, 40.0);
  assert!(pixel(&c, 50, 50)[3] > 230, "the middle stays nearly opaque");
  assert!(pixel(&c, 26, 50)[3] > 10, "coverage reaches outside the shape");
  assert!(pixel(&c, 50, 50)[3] > pixel(&c, 31, 50)[3], "and the edge is softened");
  d.set_filter(Filter::Blur(MAX_BLUR_RADIUS * 2.0));
  assert_eq!(d.filter(), Filter::Blur(12.0), "an out-of-range radius is refused");
}

#[test]
fn blend_modes_follow_the_specification_and_are_part_of_the_saved_state() {
  let (_app, _tree, r) = setup(40.0, 40.0);
  let c = r.as_canvas().unwrap();
  let d = c.context_2d();
  d.set_fill_style("#804020");
  d.fill_rect(0.0, 0.0, 40.0, 40.0);
  d.save();
  d.set_global_composite_operation(BlendMode::Multiply);
  d.set_fill_style("#808080");
  d.fill_rect(0.0, 0.0, 40.0, 40.0);
  let multiplied = pixel(&c, 20, 20);
  assert_eq!(multiplied[0], 64, "0x80 * 0x80 over an opaque backdrop: {multiplied:?}");
  assert_eq!(multiplied[1], 32);
  d.restore();
  assert_eq!(
    d.global_composite_operation(),
    BlendMode::Normal,
    "restore takes it back"
  );
  d.clear();
  d.set_fill_style("#404040");
  d.fill_rect(0.0, 0.0, 40.0, 40.0);
  d.set_global_composite_operation(BlendMode::Screen);
  d.set_fill_style("#404040");
  d.fill_rect(0.0, 0.0, 40.0, 40.0);
  assert_eq!(pixel(&c, 20, 20)[0], 112, "screen lightens: 0x40 + 0x40 - 0x40*0x40");
  assert_eq!(BlendMode::ALL.len(), 18);
}

#[test]
fn an_isolated_layer_fades_overlapping_shapes_as_one_image() {
  let (_app, _tree, r) = setup(100.0, 100.0);
  let c = r.as_canvas().unwrap();
  let d = c.context_2d();
  // Per-draw alpha darkens the overlap; that is the behaviour a group cannot use.
  d.set_global_alpha(0.5);
  d.set_fill_style("#ff0000");
  d.fill_rect(10.0, 10.0, 40.0, 40.0);
  d.fill_rect(30.0, 30.0, 40.0, 40.0);
  let per_draw_overlap = pixel(&c, 40, 40);
  assert!(
    per_draw_overlap[3] > 170,
    "overlapping alpha accumulates: {per_draw_overlap:?}"
  );
  d.clear();
  d.set_global_alpha(1.0);
  d.begin_layer(0.5, BlendMode::Normal).unwrap();
  d.fill_rect(10.0, 10.0, 40.0, 40.0);
  d.fill_rect(30.0, 30.0, 40.0, 40.0);
  d.end_layer().unwrap();
  let alone = pixel(&c, 20, 20);
  let overlap = pixel(&c, 40, 40);
  assert_eq!(alone, overlap, "the group fades as one: {alone:?} vs {overlap:?}");
  assert!(overlap[3].abs_diff(128) <= 1, "and by exactly its own opacity");
  assert_eq!(d.end_layer(), Err(CanvasError::UnbalancedLayer));
}

#[test]
fn layers_nest_to_a_stated_depth_and_carry_a_blend_mode() {
  let (_app, _tree, r) = setup(40.0, 40.0);
  let c = r.as_canvas().unwrap();
  let d = c.context_2d();
  d.set_fill_style("#ffffff");
  d.fill_rect(0.0, 0.0, 40.0, 40.0);
  d.begin_layer(1.0, BlendMode::Multiply).unwrap();
  d.set_fill_style("#ff0000");
  d.fill_rect(0.0, 0.0, 40.0, 40.0);
  d.end_layer().unwrap();
  assert_eq!(pixel(&c, 20, 20), [255, 0, 0, 255], "red multiplied into white is red");
  for _ in 0..MAX_LAYER_DEPTH {
    d.begin_layer(1.0, BlendMode::Normal).unwrap();
  }
  assert_eq!(d.begin_layer(1.0, BlendMode::Normal), Err(CanvasError::StateLimit));
  for _ in 0..MAX_LAYER_DEPTH {
    d.end_layer().unwrap();
  }
}

#[test]
fn a_gradient_is_refused_for_text_rather_than_reduced_to_one_of_its_colours() {
  let (_app, _tree, r) = setup(120.0, 40.0);
  let c = r.as_canvas().unwrap();
  let d = c.context_2d();
  d.set_font(CanvasFont::new("", 16.0));
  d.set_fill_style(
    Gradient::linear()
      .stop(0.0, "#ff0000")
      .stop(1.0, "#0000ff")
      .in_box(0.0, 0.0, 120.0, 40.0),
  );
  assert_eq!(d.fill_text("gradient", 4.0, 24.0), Err(CanvasError::UnsupportedPaint));
  assert!(matches!(d.measure_text("gradient"), Err(CanvasError::UnsupportedPaint)));
}

#[test]
fn shaped_text_casts_the_shadow_in_force_and_blurs_with_the_filter() {
  let (_app, _tree, r) = setup(200.0, 60.0);
  let c = r.as_canvas().unwrap();
  let d = c.context_2d();
  d.set_font(CanvasFont::new("", 28.0));
  d.set_fill_style("#ffffff");
  d.fill_text("Shadow", 10.0, 40.0).unwrap();
  let plain: u64 = c
    .snapshot()
    .try_take()
    .unwrap()
    .unwrap()
    .rgba
    .chunks_exact(4)
    .map(|p| u64::from(p[3]))
    .sum();
  d.clear();
  d.set_shadow(Some(
    Shadow::new(lurq::node::color::Color::new(0, 0, 0, 255))
      .offset(3.0, 3.0)
      .blur(6.0),
  ));
  d.fill_text("Shadow", 10.0, 40.0).unwrap();
  let with_shadow: u64 = c
    .snapshot()
    .try_take()
    .unwrap()
    .unwrap()
    .rgba
    .chunks_exact(4)
    .map(|p| u64::from(p[3]))
    .sum();
  assert!(
    with_shadow > plain,
    "the shadow adds coverage: {plain} -> {with_shadow}"
  );
  d.save();
  d.set_shadow(None);
  d.restore();
  assert!(d.shadow().is_some(), "the shadow is part of the saved state");
}
