//! The pointer on the edge between two rows. A row is under the pointer from its first row of pixels to its
//! last: the row of pixels under it is the next row's, or the gap's, and the column beside it is no part of it
//! either. At a scale of 150% a row of 32 is 48 device pixels and a gap of 2 is 3.

use lurq::{
  app::{App, Tree},
  components::{Column, Rect},
  node::color::Color,
};

use crate::support::{RenderSnapshot, render_pass_with_app};

const REST: &str = "#334155";
const HOVER: &str = "#22c55e";
const SCALE: f32 = 1.5;
const ROW: f32 = 32.0;
const WIDTH: f32 = 200.0;

/// Three rows of 200 by 32, `gap` apart, at 150%.
fn rows(gap: f32) -> (Tree, App) {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_scale_factor(SCALE);
  tree.resize(600, 600);
  tree.set_root(Column::new().spacing(gap).with_children((0..3).map(|_| {
    Rect::new(WIDTH, ROW)
      .background(REST)
      .hovered(|style| style.background(HOVER))
  })));
  render_pass_with_app(&mut tree, &mut app);
  (tree, app)
}

/// Which rows the frame draws hovered, top to bottom.
fn hovered(frame: &RenderSnapshot) -> Vec<usize> {
  let mut rows: Vec<_> = frame.rects.iter().filter(|rect| rect.height == ROW * SCALE).collect();
  rows.sort_by(|a, b| a.y.total_cmp(&b.y));
  rows
    .iter()
    .enumerate()
    .filter(|(_, rect)| rect.color == Color::from_hex(HOVER))
    .map(|(index, _)| index)
    .collect()
}

/// The rows drawn hovered with the pointer on the device pixel `(x, y)`.
fn hovered_at(tree: &mut Tree, app: &mut App, x: f32, y: f32) -> Vec<usize> {
  tree.mouse_move(x, y);
  hovered(&render_pass_with_app(tree, app))
}

#[test]
fn of_two_rows_that_touch_each_has_the_pointer_on_its_own_pixels() {
  let (mut tree, mut app) = rows(0.0);
  let edge = ROW * SCALE;

  assert_eq!(
    hovered_at(&mut tree, &mut app, 30.0, edge - 1.0),
    [0],
    "the first row's last pixels"
  );
  assert_eq!(
    hovered_at(&mut tree, &mut app, 30.0, edge),
    [1],
    "the second row's first pixels"
  );
  assert_eq!(hovered_at(&mut tree, &mut app, 30.0, 2.0 * edge - 1.0), [1]);
  assert_eq!(hovered_at(&mut tree, &mut app, 30.0, 2.0 * edge), [2]);
}

#[test]
fn the_pixels_under_a_row_are_the_gap_s() {
  let (mut tree, mut app) = rows(2.0);
  let bottom = ROW * SCALE;

  assert_eq!(hovered_at(&mut tree, &mut app, 30.0, bottom - 1.0), [0]);
  for below in 0..3 {
    assert_eq!(
      hovered_at(&mut tree, &mut app, 30.0, bottom + below as f32),
      [] as [usize; 0],
      "{below} pixels into the gap: no row is under the pointer"
    );
  }
  assert_eq!(
    hovered_at(&mut tree, &mut app, 30.0, bottom + 3.0),
    [1],
    "the next row's first pixels"
  );
}

#[test]
fn the_pixels_beside_a_row_are_not_the_row_s() {
  let (mut tree, mut app) = rows(2.0);
  let end = WIDTH * SCALE;

  assert_eq!(
    hovered_at(&mut tree, &mut app, end - 1.0, 10.0),
    [0],
    "the row's last column"
  );
  assert_eq!(
    hovered_at(&mut tree, &mut app, end, 10.0),
    [] as [usize; 0],
    "the column after it"
  );
  assert_eq!(
    hovered_at(&mut tree, &mut app, 0.0, 0.0),
    [0],
    "its first pixel is its own"
  );
}
