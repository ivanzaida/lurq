//! A child keeps its natural size or loses at least a pixel: shares under a
//! pixel go to the other children of the same order.

use lurq::components::{Rect, Row};

use super::{
  DETAIL, LINE_HEIGHT, RUN, SPACING, TIME, WIDE, assert_close, child_widths, drawn_texts, label, layout_once,
};

fn labels() -> Row {
  Row::new()
    .spacing(SPACING)
    .child(label(DETAIL).flex_shrink(1.0))
    .child(label(RUN).flex_shrink(1.0))
    .child(label(TIME).flex_shrink(1.0))
}

#[test]
fn give_way_sub_pixel_overflow_ellipsizes_one_label_only() {
  let (_, natural) = layout_once(Row::new().child(labels()), WIDE, LINE_HEIGHT);
  let natural = child_widths(&natural.children[0].result);
  let line = natural.iter().sum::<f32>() + SPACING * 2.0;

  let (tree, layout) = layout_once(labels(), line - 0.9, LINE_HEIGHT);
  let widths = child_widths(&layout);
  assert_close(widths[0], natural[0] - 0.9, "one label takes the whole overflow");
  assert_eq!(widths[1..], natural[1..], "the others keep their widths exactly");
  let drawn = drawn_texts(&tree, &layout);
  assert_eq!(
    drawn[1..],
    [RUN.to_string(), TIME.to_string()],
    "no stray ellipsis: {drawn:?}"
  );
}

#[test]
fn give_way_sub_pixel_share_goes_to_the_larger_shares() {
  // Overflow 5 by factors 10 and 1: the second child's share (0.45) goes to
  // the first.
  let row = Row::new()
    .spacing(0.0)
    .child(Rect::new(100.0, 10.0).flex_shrink(10.0))
    .child(Rect::new(100.0, 10.0).flex_shrink(1.0));
  let (_, layout) = layout_once(row, 195.0, 10.0);
  assert_eq!(child_widths(&layout), [95.0, 100.0]);
}

#[test]
fn give_way_shares_of_a_pixel_or_more_stay_proportional() {
  let row = Row::new()
    .spacing(0.0)
    .child(Rect::new(100.0, 10.0).flex_shrink(10.0))
    .child(Rect::new(100.0, 10.0).flex_shrink(1.0));
  let (_, layout) = layout_once(row, 178.0, 10.0);
  let widths = child_widths(&layout);
  assert_close(widths[0], 80.0, "the first child's share");
  assert_close(widths[1], 98.0, "the second child's share");
}
