#![cfg(feature = "canvas")]

mod support;

use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, render_engine::RenderEngine},
  canvas::{ArcDirection, CanvasError, CanvasFont, CanvasHandle, Context2D, FillRule, Path2D, TextAlign},
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
  let snapshot = canvas.snapshot();
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
  tree.set_root(Canvas::new().ref_element(reference.clone()).width(64.0).height(64.0));
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
  tree.set_root(Canvas::new().ref_element(reference.clone()).width(64.0).height(64.0));
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
  assert_eq!(draw.canvas().snapshot().rgba.len(), 64 * 64 * 4);
}

#[test]
fn changing_ref_binding_preserves_surface_and_invalidates_old_ref() {
  let (mut app, mut tree, reference) = setup(64.0, 64.0);
  let id = reference.as_canvas().unwrap().surface_id();
  let replacement = ElementRef::new();
  tree.set_root(Canvas::new().ref_element(replacement.clone()).width(64.0).height(64.0));
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
  assert!(canvas.snapshot().rgba.iter().all(|v| *v == 0));
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
  assert_eq!(d.fill_style().r(), 255);
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
  assert!(c.snapshot().rgba.chunks_exact(4).any(|p| p[3] > 0 && p[3] < 255));
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
  tree.set_root(Canvas::new().ref_element(r.clone()).width(60.0).height(40.0));
  tree.pass(&mut app, &support::TestSurface);
  assert_eq!(c.surface_id(), r.as_canvas().unwrap().surface_id());
  assert_eq!(c.pixel_size(), (120, 80));
  assert!(c.snapshot().rgba.iter().all(|v| *v == 0));
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
  assert_eq!(c.snapshot().rgba.len(), 32 * 32 * 4);
}

#[test]
fn submitted_alpha_is_not_reapplied_on_window_redraw() {
  let (mut app, mut tree, r) = setup(32.0, 32.0);
  let c = r.as_canvas().unwrap();
  let d = c.context_2d();
  d.set_fill_style("#ff000080");
  d.fill_rect(0.0, 0.0, 32.0, 32.0);
  d.fill_rect(0.0, 0.0, 16.0, 32.0);
  let expected = c.snapshot().rgba;
  for _ in 0..4 {
    tree.request_redraw();
    tree.pass(&mut app, &support::TestSurface);
  }
  assert_eq!(c.snapshot().rgba, expected);
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
  assert!(c.snapshot().rgba.chunks_exact(4).any(|p| p[3] > 0));
  let centered = d.measure_text("Canvas").unwrap();
  assert!((centered.width - m.width).abs() < 0.01);
  assert!((centered.actual_bounding_box_left - m.actual_bounding_box_left - m.width * 0.5).abs() < 0.01);
  assert!(d.measure_text("مرحبا").unwrap().width > 0.0);
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
  tree.set_root(Canvas::new().ref_element(r.clone()).width(40.0).height(40.0));
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
  let make_a = || Canvas::new().key("a").ref_element(a.clone()).width(32.0).height(32.0);
  let make_b = || Canvas::new().key("b").ref_element(b.clone()).width(32.0).height(32.0);
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
    Column::new()
      .child(make_b())
      .child(Canvas::new().key("new").ref_element(a.clone()).width(32.0).height(32.0)),
  );
  tree.pass(&mut app, &support::TestSurface);
  assert_ne!(first.surface_id(), a.as_canvas().unwrap().surface_id());
  assert!(!first.is_attached());
}

#[test]
fn cloned_elements_have_independent_surfaces() {
  let (mut app, mut tree, _) = setup(32.0, 32.0);
  let element: Element = Canvas::new().width(32.0).height(32.0).into();
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
  tree.set_root(Canvas::new().ref_element(r).width(32.0).height(32.0));
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
