use lurq::{
  app::Tree,
  components::{Image, Rect, Stack},
  images::ImageData,
  node::{border::Border, color::Color},
};

use crate::support::render_pass;

#[test]
fn image_command_carries_structural_order() {
  let image = ImageData::from_rgba(vec![255, 255, 255, 255], 1, 1);
  let mut runtime = Tree::new();
  runtime.set_root(
    Stack::new()
      .child(Rect::new(10.0, 10.0).background("#ef4444"))
      .child(Image::new(image).size(10.0, 10.0))
      .child(Rect::new(10.0, 10.0).background("#22c55e")),
  );

  let snapshot = render_pass(&mut runtime);

  assert_eq!(snapshot.rects.len(), 2);
  assert_eq!(snapshot.image_orders, vec![1]);
}

#[test]
fn left_border_emits_only_left_stroke() {
  let mut runtime = Tree::new();
  runtime.set_root(
    Rect::new(20.0, 10.0)
      .background("#111827")
      .border_left(Border::inside(2.0, Color::from_hex("#8b5cf6"))),
  );

  let snapshot = render_pass(&mut runtime);

  assert_eq!(snapshot.rects.len(), 2);
  assert_eq!(snapshot.rects[1].stroke, [0.0, 0.0, 0.0, 2.0]);
  assert_eq!(snapshot.rects[1].stroke_color, Color::from_hex("#8b5cf6"));
}

#[test]
fn all_sides_border_stays_grouped() {
  let mut runtime = Tree::new();
  runtime.set_root(
    Rect::new(20.0, 10.0)
      .background("#111827")
      .border(Border::inside(1.0, Color::from_hex("#8b5cf6"))),
  );

  let snapshot = render_pass(&mut runtime);

  assert_eq!(snapshot.rects.len(), 2);
  assert_eq!(snapshot.rects[1].stroke, [1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn corner_radius_shorthand_and_per_corner_setters_emit_radii() {
  let mut runtime = Tree::new();
  runtime.set_root(
    Rect::new(20.0, 10.0)
      .background("#111827")
      .corner_radius(2.0)
      .corner_radius_top_right(4.0)
      .corner_radius_bottom_left(6.0),
  );

  let snapshot = render_pass(&mut runtime);

  assert_eq!(snapshot.rects.len(), 1);
  assert_eq!(snapshot.rects[0].radii, [2.0, 4.0, 2.0, 5.0]);
}
