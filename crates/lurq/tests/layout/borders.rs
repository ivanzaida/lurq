use lurq::{
  app::Tree,
  components::Rect,
  node::{BackgroundColor, border::Border, color::Color},
};

use crate::support::render_pass;

#[test]
fn bottom_only_border_renders_as_single_bottom_rect() {
  let border_color = Color::from_hex("#ff0000");
  let mut tree = Tree::new();
  tree.set_root(Rect::new(100.0, 40.0).border_bottom(Border::inside(1.0, BackgroundColor::Color(border_color))));

  let snapshot = render_pass(&mut tree);
  let border_rects = snapshot
    .rects
    .iter()
    .filter(|rect| rect.color == border_color)
    .collect::<Vec<_>>();

  assert_eq!(border_rects.len(), 1, "bottom-only border should emit one filled rect");
  let rect = border_rects[0];
  assert_eq!(rect.x, 0.0);
  assert_eq!(rect.y, 39.0);
  assert_eq!(rect.width, 100.0);
  assert_eq!(rect.height, 1.0);
  assert_eq!(rect.stroke, [0.0; 4]);
}

#[test]
fn rounded_bottom_only_border_renders_as_single_bottom_rect() {
  let border_color = Color::from_hex("#ff0000");
  let mut tree = Tree::new();
  tree.set_root(
    Rect::new(100.0, 40.0)
      .rounded(8.0)
      .border_bottom(Border::inside(1.0, BackgroundColor::Color(border_color))),
  );

  let snapshot = render_pass(&mut tree);
  let border_rects = snapshot
    .rects
    .iter()
    .filter(|rect| rect.color == border_color)
    .collect::<Vec<_>>();

  assert_eq!(
    border_rects.len(),
    1,
    "rounded bottom-only border should emit one filled rect"
  );
  let rect = border_rects[0];
  assert_eq!(rect.x, 0.0);
  assert_eq!(rect.y, 39.0);
  assert_eq!(rect.width, 100.0);
  assert_eq!(rect.height, 1.0);
  assert_eq!(rect.stroke, [0.0; 4]);
}
