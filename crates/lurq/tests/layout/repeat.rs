//! `.repeat(Repeat)`: what an element holds painted again across its box, a gap apart, every other row moved by
//! the stagger, cut to the box, with the tiles of a faded child faded each.

use lurq::{
  app::{App, Tree},
  components::{Row, Stack},
  layout::{
    Constraints, Repeat, Size,
    quad::{Quad, QuadContent},
  },
  node::color::Color,
};

use crate::support::TestSurface;

const BLUE: &str = "#2040ff";
const RED: &str = "#ff2020";

fn painted(holder: Stack) -> Vec<Quad> {
  let mut tree = Tree::new();
  tree.set_root(Row::new().padding(10.0).child(holder));
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  let mut app = App::new();
  tree.pass(&mut app, &TestSurface);
  tree.resolve_quads(tree.last_layout().unwrap())
}

fn of(quads: &[Quad], hex: &str) -> Vec<(f32, f32)> {
  let color = Color::from_hex(hex);
  quads
    .iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Rect { color: painted, .. } if *painted == color))
    .map(|quad| (quad.x, quad.y))
    .collect()
}

#[test]
fn the_tile_is_painted_a_gap_apart_across_the_box_and_cut_to_it() {
  let quads = painted(
    Stack::new()
      .size(200.0, 100.0)
      .repeat(Repeat::new().gap(10.0, 10.0))
      .child(Row::new().size(20.0, 20.0).background(Color::from_hex(BLUE))),
  );
  let tiles = of(&quads, BLUE);
  // Seven columns 30 apart from 10, four rows 30 apart from 10.
  assert_eq!(tiles.len(), 28, "{tiles:?}");
  for column in 0..7 {
    for row in 0..4 {
      let at = (10.0 + column as f32 * 30.0, 10.0 + row as f32 * 30.0);
      assert!(tiles.contains(&at), "a tile at {at:?}");
    }
  }
  let blue = Color::from_hex(BLUE);
  for quad in quads
    .iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Rect { color, .. } if *color == blue))
  {
    assert_eq!(
      (quad.clip.x, quad.clip.y, quad.clip.width, quad.clip.height),
      (10.0, 10.0, 200.0, 100.0)
    );
  }
}

#[test]
fn every_other_row_is_moved_by_the_stagger_and_covers_the_start() {
  let quads = painted(
    Stack::new()
      .size(200.0, 60.0)
      .repeat(Repeat::new().gap(10.0, 10.0).stagger(15.0))
      .child(Row::new().size(20.0, 20.0).background(Color::from_hex(BLUE))),
  );
  let tiles = of(&quads, BLUE);
  let second: Vec<f32> = tiles.iter().filter(|(_, y)| *y == 40.0).map(|(x, _)| *x).collect();
  assert!(second.contains(&(10.0 + 15.0)));
  // The one before it, which reaches into the box from its start.
  assert!(second.contains(&(10.0 - 15.0)), "{second:?}");
  assert!(
    tiles
      .iter()
      .filter(|(_, y)| *y == 10.0)
      .all(|(x, _)| (x - 10.0).rem_euclid(30.0) == 0.0)
  );
}

#[test]
fn a_tile_of_more_than_one_box_is_repeated_whole() {
  let quads = painted(
    Stack::new().size(90.0, 30.0).repeat(Repeat::new()).child(
      Row::new()
        .size(30.0, 30.0)
        .opacity(0.5)
        .background(Color::from_hex(BLUE))
        .child(Row::new().size(10.0, 10.0).background(Color::from_hex(RED))),
    ),
  );
  assert_eq!(of(&quads, BLUE).len(), 3);
  assert_eq!(of(&quads, RED), vec![(10.0, 10.0), (40.0, 10.0), (70.0, 10.0)]);
}
