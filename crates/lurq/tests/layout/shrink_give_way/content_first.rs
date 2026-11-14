//! `ShrinkLimit::ContentFirst`: a child keeps to its content minimum while
//! the others of its order can take the overflow, then gives way below it,
//! the last first. The content minimum of a column in a row is the widest of
//! its children's, a child with a least width of its own at it.

use lurq::{
  app::Tree,
  components::{Column, Row, Spacer},
  layout::{Constraints, Size, layout_kind::ShrinkLimit},
};

use crate::layout::PassLayoutExt;

/// A step: a title of 60 that keeps its width and a description of 150 that may go to nothing.
fn step() -> Column {
  Column::new()
    .flex_shrink(1.0)
    .shrink_limit(ShrinkLimit::ContentFirst)
    .min_width(0.0)
    .child(Spacer::new().size(60.0, 10.0))
    .child(Spacer::new().size(150.0, 10.0).min_width(0.0))
}

fn widths(width: f32) -> Vec<f32> {
  let mut tree = Tree::new();
  tree.set_root(Row::new().spacing(0.0).child(step()).child(step()));
  let result = tree.pass_layout(Constraints::tight(Size::new(width, 40.0))).unwrap();
  result.children.iter().map(|child| child.result.size.width).collect()
}

#[test]
fn children_keep_to_their_content_while_the_others_can_take_the_overflow() {
  assert_eq!(widths(300.0), vec![150.0, 150.0], "room for both");
  // 100 over: both give way above their content minimum of 60.
  assert_eq!(widths(200.0), vec![100.0, 100.0]);
  // 180 over, as much as both hold above 60: both at their content.
  assert_eq!(widths(120.0), vec![60.0, 60.0]);
}

#[test]
fn past_the_content_of_all_the_last_gives_way_below_its_own() {
  assert_eq!(widths(100.0), vec![60.0, 40.0]);
  // Where the last one going to nothing is not enough, the one before it gives way too, and they share it.
  assert_eq!(widths(50.0), vec![25.0, 25.0]);
}
