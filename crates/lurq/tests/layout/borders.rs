use lurq::{
  app::Tree,
  components::Rect,
  layout::{Constraints, Size},
  node::{BackgroundColor, border::Border, color::Color},
};

use super::PassLayoutExt;
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

/// A pill-shaped clipping container (the common `radius: 999` idiom) must
/// clamp its rounded-clip radii to the box, or the clip has no interior and
/// every child pixel is discarded by the shader-side rounded clip
/// (regression: chip/pill labels rendering invisible).
#[test]
fn pill_radius_clip_keeps_a_usable_interior_for_children() {
  let mut rt = Tree::new();
  let node = lurq::components::Row::new()
    .align_items(lurq::layout::Alignment::Center)
    .padding_horizontal(9.0)
    .padding_vertical(5.0)
    .rounded(999.0)
    .background("#16302A")
    .clip()
    .child(lurq::components::Text::new("auto_attack"));
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  let text_clip = quads
    .iter()
    .find_map(|quad| match &quad.content {
      lurq::layout::quad::QuadContent::Text { text, .. } if text == "auto_attack" => Some(quad.clip),
      _ => None,
    })
    .expect("the label text quad should be emitted");

  assert!(text_clip.active, "the pill must clip its children");
  let radius = text_clip.border_radius.expect("the pill clip should stay rounded");
  let max_radius = (text_clip.width.min(text_clip.height)) / 2.0 + 0.001;
  assert!(
    radius.top_left <= max_radius && radius.bottom_right <= max_radius,
    "clip radii must be clamped to the box (radius {} in {}x{})",
    radius.top_left,
    text_clip.width,
    text_clip.height,
  );
  assert!(radius.top_left > 0.0, "clamping must keep the pill rounded");
}
