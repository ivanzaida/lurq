//! Children give way by order, lowest first; one order shares by factor.

use lurq::components::{Column, Rect, Row};

use super::{
  DETAIL, LINE_HEIGHT, RUN, SPACING, TIME, WIDE, assert_close, child_heights, child_widths, drawn_texts, label,
  layout_once,
};

/// Three ellipsizing labels that give way in the order detail, run, time.
fn ordered_labels() -> Row {
  Row::new()
    .spacing(SPACING)
    .child(label(DETAIL).flex_shrink(1.0).shrink_order(1))
    .child(label(RUN).flex_shrink(1.0).shrink_order(2))
    .child(label(TIME).flex_shrink(1.0).shrink_order(3))
}

fn natural_label_widths() -> Vec<f32> {
  let (_, layout) = layout_once(Row::new().child(ordered_labels()), WIDE, LINE_HEIGHT);
  child_widths(&layout.children[0].result)
}

fn line_width(widths: &[f32]) -> f32 {
  widths.iter().sum::<f32>() + SPACING * (widths.len() - 1) as f32
}

#[test]
fn give_way_overflow_within_first_order_shrinks_only_that_child() {
  let natural = natural_label_widths();
  let (tree, layout) = layout_once(ordered_labels(), line_width(&natural) - 5.0, LINE_HEIGHT);

  let widths = child_widths(&layout);
  assert_close(widths[0], natural[0] - 5.0, "the first order absorbs the overflow");
  assert_eq!(
    widths[1], natural[1],
    "the second order keeps its natural width exactly"
  );
  assert_eq!(widths[2], natural[2], "the third order keeps its natural width exactly");
  let drawn = drawn_texts(&tree, &layout);
  assert!(drawn[0].ends_with('…'), "the first order is ellipsized: {drawn:?}");
  assert_eq!(
    drawn[1..],
    [RUN.to_string(), TIME.to_string()],
    "no ellipsis further down the order"
  );
}

#[test]
fn give_way_overflow_spills_into_the_next_order_after_the_floor() {
  const FLOOR: f32 = 40.0;
  let natural = natural_label_widths();
  let row = Row::new()
    .spacing(SPACING)
    .child(label(DETAIL).flex_shrink(1.0).shrink_order(1).min_width(FLOOR))
    .child(label(RUN).flex_shrink(1.0).shrink_order(2))
    .child(label(TIME).flex_shrink(1.0).shrink_order(3));
  let spill = 10.0;
  let width = line_width(&natural) - (natural[0] - FLOOR) - spill;
  let (tree, layout) = layout_once(row, width, LINE_HEIGHT);

  let widths = child_widths(&layout);
  assert_eq!(widths[0], FLOOR, "the first order stops at its floor");
  assert_close(widths[1], natural[1] - spill, "the second order takes the rest");
  assert_eq!(widths[2], natural[2], "the third order is untouched");
  assert_eq!(drawn_texts(&tree, &layout)[2], TIME);
}

#[test]
fn give_way_overflow_past_every_floor_leaves_all_orders_at_their_floors() {
  let row = Row::new()
    .spacing(0.0)
    .child(Rect::new(100.0, 10.0).flex_shrink(1.0).shrink_order(1).min_width(60.0))
    .child(Rect::new(100.0, 10.0).flex_shrink(1.0).shrink_order(2).min_width(70.0));
  let (_, layout) = layout_once(row, 100.0, 10.0);
  assert_eq!(child_widths(&layout), [60.0, 70.0]);
}

#[test]
fn give_way_unset_order_gives_way_before_positive_orders() {
  let row = Row::new()
    .spacing(0.0)
    .child(Rect::new(100.0, 10.0).flex_shrink(1.0).shrink_order(1))
    .child(Rect::new(100.0, 10.0).flex_shrink(1.0))
    .child(Rect::new(100.0, 10.0).flex_shrink(1.0).shrink_order(-1));
  let (_, layout) = layout_once(row, 130.0, 10.0);
  assert_eq!(child_widths(&layout), [100.0, 30.0, 0.0]);
}

#[test]
fn give_way_one_order_shares_by_factor() {
  let row = Row::new()
    .spacing(0.0)
    .child(Rect::new(100.0, 10.0).flex_shrink(1.0).shrink_order(1))
    .child(Rect::new(100.0, 10.0).flex_shrink(3.0).shrink_order(1))
    .child(Rect::new(100.0, 10.0).flex_shrink(1.0).shrink_order(2));
  let (_, layout) = layout_once(row, 260.0, 10.0);
  assert_eq!(child_widths(&layout), [90.0, 70.0, 100.0]);
}

#[test]
fn give_way_without_orders_shares_by_factor_as_before() {
  let row = Row::new()
    .spacing(0.0)
    .child(Rect::new(100.0, 10.0).flex_shrink(1.0))
    .child(Rect::new(100.0, 10.0).flex_shrink(3.0));
  let (_, layout) = layout_once(row, 160.0, 10.0);
  assert_eq!(child_widths(&layout), [90.0, 70.0]);
}

#[test]
fn give_way_column_shrinks_by_order() {
  let column = Column::new()
    .spacing(0.0)
    .child(Rect::new(10.0, 100.0).flex_shrink(1.0).shrink_order(2))
    .child(Rect::new(10.0, 100.0).flex_shrink(1.0).shrink_order(1).min_height(80.0))
    .child(Rect::new(10.0, 100.0).flex_shrink(1.0).shrink_order(3));
  let (_, layout) = layout_once(column, 10.0, 270.0);
  assert_eq!(child_heights(&layout), [90.0, 80.0, 100.0]);
  assert_eq!(layout.children[2].offset.y, 170.0);
}
