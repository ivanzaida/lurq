//! A clip inside a clip with rounded corners: what clips and lies along an
//! edge of a rounded panel (a rail along its bottom, a band across its top, a
//! scrolling list down to its bottom corners, a child with corners of its own)
//! keeps the panel's round corners, as nested `overflow: hidden` boxes with
//! `border-radius` do in CSS. The shape is the panel's, not the inner box's:
//! a rail 2 high cannot carry a radius of 8.

use lurq::{
  app::Tree,
  components::{Column, Rect, Row, ScrollVertical, Stack},
  layout::{
    Constraints, Size,
    quad::{ClipRect, Quad, QuadContent, RoundedClip},
  },
  node::{border::BorderRadius, color::Color, transform::Transform2D},
};

use super::PassLayoutExt;

const PANEL: Color = Color::new(0x20, 0x20, 0x20, 0xff);
const FILL: Color = Color::new(0xf0, 0x40, 0x40, 0xff);
const OTHER: Color = Color::new(0x40, 0x40, 0xf0, 0xff);

fn quads(root: impl Into<lurq::node::Element>) -> Vec<Quad> {
  let mut tree = Tree::new();
  tree.set_root(root);
  let result = tree.pass_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
  tree.resolve_quads(&result)
}

/// The clip of the first rect painted `color`.
fn clip_of(quads: &[Quad], color: Color) -> ClipRect {
  quads
    .iter()
    .find(|quad| matches!(quad.content, QuadContent::Rect { color: painted, .. } if painted == color))
    .unwrap_or_else(|| panic!("a rect painted {color:?}"))
    .clip
}

fn rect_of(clip: &ClipRect) -> [f32; 4] {
  [clip.x, clip.y, clip.width, clip.height]
}

/// The panel's own rounded clip: 200 x 100 at the origin, corners of `radius`.
fn panel_shape(radius: f32) -> RoundedClip {
  RoundedClip {
    x: 0.0,
    y: 0.0,
    width: 200.0,
    height: 100.0,
    radius: BorderRadius::all(radius),
  }
}

fn panel(radius: f32) -> Column {
  Column::new()
    .spacing(0.0)
    .size(200.0, 100.0)
    .background(PANEL)
    .rounded(radius)
    .clip()
}

#[test]
fn a_rail_along_the_bottom_of_a_rounded_panel_is_cut_by_the_panel_corners() {
  let quads = quads(
    panel(8.0)
      .child(Rect::new(200.0, 98.0).background(OTHER))
      // The rail clips, as nodes do by default.
      .child(Row::new().size(200.0, 2.0).child(Rect::new(200.0, 2.0).background(FILL))),
  );
  let clip = clip_of(&quads, FILL);

  assert_eq!(rect_of(&clip), [0.0, 98.0, 200.0, 2.0], "the rail's rectangle");
  assert_eq!(
    clip.rounded,
    [Some(panel_shape(8.0)), None],
    "the panel's corners, at the panel's size"
  );
  // The bottom-left pixel of the rail is past the panel's corner.
  assert!(!clip.contains(0.5, 99.5));
  assert!(clip.contains(100.0, 99.0));
}

#[test]
fn a_band_across_the_top_of_a_rounded_panel_is_cut_by_the_panel_corners() {
  let quads = quads(
    panel(12.0)
      .child(
        Row::new()
          .size(200.0, 30.0)
          .child(Rect::new(200.0, 30.0).background(FILL)),
      )
      .child(Rect::new(200.0, 70.0).background(OTHER)),
  );
  let clip = clip_of(&quads, FILL);

  assert_eq!(rect_of(&clip), [0.0, 0.0, 200.0, 30.0]);
  assert_eq!(clip.rounded, [Some(panel_shape(12.0)), None]);
  assert!(!clip.contains(1.0, 1.0) && !clip.contains(199.0, 1.0));
}

#[test]
fn a_scrolling_list_down_to_the_bottom_corners_is_cut_by_them() {
  let list = Column::new()
    .spacing(0.0)
    .with_children((0..6).map(|_| Rect::new(200.0, 30.0).background(FILL)));
  let quads = quads(
    panel(10.0)
      .child(Rect::new(200.0, 30.0).background(OTHER))
      .child(ScrollVertical::new(list).size(200.0, 70.0)),
  );
  let clip = clip_of(&quads, FILL);

  assert_eq!([clip.x, clip.y, clip.height], [0.0, 30.0, 70.0], "the list's viewport");
  assert_eq!(clip.rounded, [Some(panel_shape(10.0)), None]);
}

#[test]
fn a_child_with_corners_of_its_own_at_the_panel_corner_is_cut_by_both() {
  let quads = quads(
    panel(16.0).child(
      Stack::new()
        .size(100.0, 50.0)
        .rounded(4.0)
        .clip()
        .child(Rect::new(100.0, 50.0).background(FILL)),
    ),
  );
  let clip = clip_of(&quads, FILL);

  assert_eq!(rect_of(&clip), [0.0, 0.0, 100.0, 50.0]);
  assert_eq!(
    clip.rounded,
    [
      Some(RoundedClip {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 50.0,
        radius: BorderRadius::all(4.0),
      }),
      Some(panel_shape(16.0)),
    ],
    "the child's corners, then the panel's"
  );
  // Inside the child's corner of 4, outside the panel's of 16.
  assert!(!clip.contains(2.0, 2.0));
  // The child's bottom-right corner is its own.
  assert!(!clip.contains(99.5, 49.5));
  assert!(clip.contains(50.0, 25.0));
}

#[test]
fn a_rounded_child_well_inside_the_panel_is_cut_by_its_own_corners_only() {
  let quads = quads(
    panel(16.0).padding(20.0).child(
      Stack::new()
        .size(100.0, 50.0)
        .rounded(4.0)
        .clip()
        .child(Rect::new(100.0, 50.0).background(FILL)),
    ),
  );
  let clip = clip_of(&quads, FILL);

  assert_eq!(rect_of(&clip), [20.0, 20.0, 100.0, 50.0]);
  assert_eq!(
    clip.rounded,
    [
      Some(RoundedClip {
        x: 20.0,
        y: 20.0,
        width: 100.0,
        height: 50.0,
        radius: BorderRadius::all(4.0),
      }),
      None,
    ],
    "the panel's corners cut nothing there"
  );
}

#[test]
fn a_clip_away_from_the_panel_corners_stays_a_rectangle() {
  let quads = quads(
    panel(8.0).child(Rect::new(200.0, 40.0).background(OTHER)).child(
      Row::new()
        .size(200.0, 2.0)
        .child(Rect::new(200.0, 2.0).background(FILL)),
    ),
  );
  let clip = clip_of(&quads, FILL);

  assert_eq!(rect_of(&clip), [0.0, 40.0, 200.0, 2.0]);
  assert!(!clip.is_rounded(), "{clip:?}");
}

#[test]
fn rectangles_under_rectangles_stay_rectangles() {
  let quads = quads(
    Column::new().size(200.0, 100.0).clip().child(
      Row::new()
        .size(200.0, 2.0)
        .child(Rect::new(200.0, 2.0).background(FILL)),
    ),
  );
  let clip = clip_of(&quads, FILL);

  assert_eq!(rect_of(&clip), [0.0, 0.0, 200.0, 2.0]);
  assert_eq!(clip.rounded, [None, None]);
}

#[test]
fn the_panel_corners_follow_a_scaled_subtree() {
  let quads = quads(
    Stack::new().size(400.0, 200.0).child(
      panel(8.0)
        .child(Rect::new(200.0, 98.0).background(OTHER))
        .child(
          Row::new()
            .size(200.0, 2.0)
            .child(Rect::new(200.0, 2.0).background(FILL)),
        )
        .transform(Transform2D::scale_uniform(2.0)),
    ),
  );
  let clip = clip_of(&quads, FILL);
  let shape = clip.rounded[0].expect("the panel's corners");

  // Scaled about the panel's centre (100, 50): the panel covers -100..300 x -50..150.
  assert_eq!(
    [shape.x, shape.y, shape.width, shape.height],
    [-100.0, -50.0, 400.0, 200.0]
  );
  assert_eq!(shape.radius, BorderRadius::all(16.0));
  assert_eq!(
    rect_of(&clip),
    [0.0, 146.0, 300.0, 4.0],
    "the rail, scaled, within the stack"
  );
  assert_eq!(clip.rounded[1], None);
}
