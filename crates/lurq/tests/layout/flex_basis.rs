//! A flex basis in a Row or a Column, as the web lays it out: the free room is
//! the line less the bases, the sizes of the children sized on their own and
//! the gaps, and a growing child is its basis and its share of that room.

use lurq::{
  app::Tree,
  components::{Row, Spacer},
  layout::{Constraints, Size},
};

use super::PassLayoutExt;

/// The widths of the children of `row`, laid out `width` wide.
fn widths(row: Row, width: f32) -> Vec<f32> {
  let mut tree = Tree::new();
  tree.set_root(row);
  let result = tree.pass_layout(Constraints::tight(Size::new(width, 40.0))).unwrap();
  result.children.iter().map(|child| child.result.size.width).collect()
}

#[test]
fn a_growing_child_is_its_basis_and_the_room_the_line_leaves_after_it() {
  // The web: free room 200 - 50 - 50 = 100, so 50 + 100.
  let row = Row::new()
    .spacing(0.0)
    .child(Spacer::new().flex_full(1.0, 0.0, Some(50.0)))
    .child(Spacer::new().width(50.0));
  assert_eq!(widths(row, 200.0), vec![150.0, 50.0]);
}

#[test]
fn growing_children_with_other_bases_share_what_is_left_after_both_bases_and_the_gap() {
  // The web: free room 300 - 100 - 0 - 20 = 180, 90 each.
  let row = Row::new()
    .spacing(20.0)
    .child(Spacer::new().flex_full(1.0, 0.0, Some(100.0)))
    .child(Spacer::new().flex_full(1.0, 0.0, Some(0.0)));
  assert_eq!(widths(row, 300.0), vec![190.0, 90.0]);
}

#[test]
fn a_growing_child_that_may_shrink_ends_at_the_same_size() {
  // As above with a shrink factor: nothing overflows, so nothing shrinks.
  let row = Row::new()
    .spacing(0.0)
    .child(Spacer::new().flex_full(1.0, 1.0, Some(50.0)))
    .child(Spacer::new().width(50.0));
  assert_eq!(widths(row, 200.0), vec![150.0, 50.0]);
}

#[test]
fn a_basis_with_no_room_left_for_it_shrinks_by_the_shortage() {
  // The web: free room 200 - 100 - 150 = -50; the child with the basis is the one that shrinks: 50.
  let row = Row::new()
    .spacing(0.0)
    .child(Spacer::new().flex_full(1.0, 1.0, Some(100.0)))
    .child(Spacer::new().width(150.0));
  assert_eq!(widths(row, 200.0), vec![50.0, 150.0]);
}
