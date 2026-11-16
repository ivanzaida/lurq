//! `.snap_to_pixels(true)`: the fills and borders of an element and of what is in it are drawn on whole device
//! pixels, and two boxes that meet still meet; what is laid out is as before.

use lurq::{
  app::{App, Tree},
  components::{Column, Row, Stack},
  layout::{
    Constraints, Size,
    quad::{Quad, QuadContent},
  },
  node::{border::Border, color::Color, dimension::Dimension},
};

use crate::support::TestSurface;

const BLUE: &str = "#2040ff";

/// The blue fills of a scene at `scale`, with a box of 5 at each 5 from a start of 3.3: the cells of a grid that
/// is not on a device pixel, and the box that holds them.
fn painted(scale: f32, snap: bool) -> (Vec<Quad>, Quad) {
  let blue = Color::from_hex(BLUE);
  let mut tree = Tree::new();
  tree.set_scale_factor(scale);
  let cells = (0..6).map(|index| {
    Row::new()
      .absolute(index as f32 * 5.0, 0.0, Dimension::Px(5.0), Dimension::Px(5.0))
      .background(blue)
  });
  tree.set_root(
    Column::new().padding(3.3).child(
      Stack::new()
        .size(30.0, 5.0)
        .border(Border::inside(1.0, Color::from_hex("#000000")))
        .background(Color::from_hex("#ffffff"))
        .snap_to_pixels(snap)
        .with_children(cells),
    ),
  );
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(200.0, 200.0))));
  let mut app = App::new();
  tree.pass(&mut app, &TestSurface);
  let quads = tree.resolve_quads(tree.last_layout().unwrap());
  let holder = quads
    .iter()
    .find(|quad| matches!(&quad.content, QuadContent::Rect { color, .. } if *color == Color::from_hex("#ffffff")))
    .map(|quad| Quad {
      x: quad.x,
      y: quad.y,
      width: quad.width,
      height: quad.height,
      opacity: quad.opacity,
      transform: quad.transform,
      transform_origin: quad.transform_origin,
      content: QuadContent::None,
      border_radius: None,
      border: None,
      clip: quad.clip,
    })
    .expect("the box that holds the cells");
  let cells = quads
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Rect { color, .. } if *color == blue))
    .collect();
  (cells, holder)
}

fn on_device_pixels(quad: &Quad, scale: f32) -> bool {
  [quad.x, quad.y, quad.x + quad.width, quad.y + quad.height]
    .into_iter()
    .all(|edge| ((edge * scale) - (edge * scale).round()).abs() < 1e-3)
}

#[test]
fn the_boxes_of_a_snapped_element_are_on_whole_device_pixels_at_every_scale() {
  for scale in [1.0f32, 1.25, 1.5, 1.75, 2.0] {
    let (cells, holder) = painted(scale, true);
    assert_eq!(cells.len(), 6);
    assert!(on_device_pixels(&holder, scale), "the box itself at {scale}");
    for cell in &cells {
      assert!(on_device_pixels(cell, scale), "{scale}: {:?}", (cell.x, cell.width));
    }
    // Side by side they meet: no seam and no overlap, whatever the rounding.
    for pair in cells.windows(2) {
      assert_eq!(pair[0].x + pair[0].width, pair[1].x, "at {scale}");
    }
    // Each is within a device pixel of where it is laid out.
    for (index, cell) in cells.iter().enumerate() {
      let laid = 3.3 + index as f32 * 5.0;
      assert!(
        (cell.x - laid).abs() <= 0.5 / scale + 1e-3,
        "{scale}: {} for {laid}",
        cell.x
      );
    }
  }
}

#[test]
fn an_element_that_does_not_snap_is_drawn_where_it_is() {
  let (cells, _) = painted(1.5, false);
  assert!((cells[0].x - 3.3).abs() < 1e-4);
  assert!(!on_device_pixels(&cells[0], 1.5));
}
