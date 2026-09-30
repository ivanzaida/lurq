//! Clips created inside a scaled subtree (a canvas-like zoom) follow the scale,
//! so clipped cards keep their text instead of cutting it at the unscaled size.

use lurq::{
  app::Tree,
  components::{Column, Stack, Text},
  layout::{
    Constraints, Size,
    quad::{ClipRect, Quad, QuadContent},
  },
  node::{color::Color, padding::Padding, transform::Transform2D},
};

use crate::support::{GlyphSnapshot, render_pass, run_pass};

const CARD: Color = Color::new(0x20, 0x20, 0x20, 0xff);
const CARD_WIDTH: f32 = 200.0;
const CARD_HEIGHT: f32 = 40.0;
const CARD_RADIUS: f32 = 8.0;

/// A 200x40 clipped, rounded card at (150, 80) inside a 400x200 board scaled about its
/// centre, the way a zoomable canvas scales its content.
fn board(transform: Transform2D) -> Tree {
  let card = Column::new()
    .size(CARD_WIDTH, CARD_HEIGHT)
    .padding(10.0)
    .background(CARD)
    .rounded(CARD_RADIUS)
    .clip()
    .child(Text::new("Card title"));
  let board = Stack::new()
    .size(400.0, 200.0)
    .padding(Padding::new().left(150.0).top(80.0))
    .child(card)
    .transform(transform);
  let mut tree = Tree::new();
  tree.set_root(Stack::new().size(800.0, 600.0).child(board));
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(800.0, 600.0))));
  tree
}

fn card_quad(quads: &[Quad]) -> &Quad {
  quads
    .iter()
    .find(|quad| matches!(quad.content, QuadContent::Rect { color, .. } if color == CARD))
    .expect("card background quad")
}

fn text_clip(quads: &[Quad]) -> ClipRect {
  quads
    .iter()
    .find(|quad| matches!(quad.content, QuadContent::Text { .. }))
    .expect("text quad")
    .clip
}

fn assert_clip_matches_scaled_card(scale: f32) {
  let mut tree = board(Transform2D::scale_uniform(scale));
  run_pass(&mut tree);
  let layout = tree.last_layout().expect("layout");
  let quads = tree.resolve_quads(layout);
  let card = card_quad(&quads);
  let clip = text_clip(&quads);

  assert!(clip.active, "the card clips its text");
  let expected = [card.x, card.y, CARD_WIDTH * scale, CARD_HEIGHT * scale];
  let actual = [clip.x, clip.y, clip.width, clip.height];
  for (actual, expected) in actual.iter().zip(expected) {
    assert!(
      (actual - expected).abs() < 0.01,
      "scale {scale}: text clip {actual:?} should cover the scaled card {expected:?}"
    );
  }
  let radius = clip.border_radius.expect("the clip keeps the card's corners");
  assert!(
    (radius.top_left - CARD_RADIUS * scale).abs() < 0.01,
    "radius {radius:?}"
  );
}

#[test]
fn clip_inside_upscaled_subtree_covers_the_scaled_card() {
  assert_clip_matches_scaled_card(2.0);
}

#[test]
fn clip_inside_downscaled_subtree_covers_the_scaled_card() {
  assert_clip_matches_scaled_card(0.5);
}

#[test]
fn clip_inside_rotated_subtree_is_the_bounding_box_of_the_rotated_card() {
  let mut tree = board(Transform2D::rotate_deg(90.0));
  run_pass(&mut tree);
  let layout = tree.last_layout().expect("layout");
  let quads = tree.resolve_quads(layout);
  let card = card_quad(&quads);
  let clip = text_clip(&quads);

  // The quad's origin is the rotated top-left corner; a quarter turn puts the
  // 200x40 card's bounding box to the left of it.
  let expected = [card.x - CARD_HEIGHT, card.y, CARD_HEIGHT, CARD_WIDTH];
  let actual = [clip.x, clip.y, clip.width, clip.height];
  for (actual, expected) in actual.iter().zip(expected) {
    assert!(
      (actual - expected).abs() < 0.01,
      "rotated text clip {actual:?} should be the card's bounding box {expected:?}"
    );
  }
  assert!(clip.border_radius.is_none());
}

/// Screen-space bounds of a glyph after its instance transform, as the glyph
/// shader places it.
fn glyph_screen_bounds(glyph: &GlyphSnapshot) -> [f32; 4] {
  let [a, b, c, d] = glyph.transform;
  let [ox, oy] = glyph.transform_origin;
  let corners = [
    (0.0, 0.0),
    (glyph.width, 0.0),
    (0.0, glyph.height),
    (glyph.width, glyph.height),
  ];
  let points = corners.map(|(x, y)| {
    let (cx, cy) = (x - ox, y - oy);
    (glyph.x + a * cx + c * cy + ox, glyph.y + b * cx + d * cy + oy)
  });
  let xs = points.map(|point| point.0);
  let ys = points.map(|point| point.1);
  [
    xs.into_iter().fold(f32::INFINITY, f32::min),
    ys.into_iter().fold(f32::INFINITY, f32::min),
    xs.into_iter().fold(f32::NEG_INFINITY, f32::max),
    ys.into_iter().fold(f32::NEG_INFINITY, f32::max),
  ]
}

#[test]
fn scaled_card_keeps_all_of_its_text_inside_its_clip() {
  let mut tree = board(Transform2D::scale_uniform(2.0));
  let snapshot = render_pass(&mut tree);

  let glyphs: Vec<_> = snapshot
    .glyphs
    .iter()
    .filter(|glyph| glyph.shadow_sigma == 0.0)
    .collect();
  assert!(!glyphs.is_empty(), "the card text is painted");
  for glyph in glyphs {
    let [x0, y0, x1, y1] = glyph_screen_bounds(glyph);
    let clip = glyph.clip;
    assert!(clip.active, "glyphs carry the card clip");
    assert!(
      x0 >= clip.x - 0.5 && y0 >= clip.y - 0.5 && x1 <= clip.x + clip.width + 0.5 && y1 <= clip.y + clip.height + 0.5,
      "glyph at ({x0}, {y0})..({x1}, {y1}) is cut by clip {clip:?}"
    );
  }
}
