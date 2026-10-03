//! `ShrinkLimit::Content`: a child never shrinks below its non-shrinking
//! content.

use lurq::{
  components::{Column, Rect, Row},
  layout::layout_kind::ShrinkLimit,
};

use super::{
  COUNT, DETAIL, LINE_HEIGHT, RUN, SPACING, WIDE, assert_close, child_heights, child_widths, drawn_texts, label,
  layout_once,
};

const ICON: f32 = 14.0;
const PAD: f32 = 8.0;

/// An icon and a count that must stay whole, then a detail that gives way.
fn approvals_item(factor: f32) -> Row {
  Row::new()
    .spacing(SPACING)
    .padding_horizontal(PAD)
    .child(Rect::new(ICON, ICON))
    .child(label(COUNT))
    .child(label(DETAIL).flex_shrink(1.0))
    .flex_shrink(factor)
    .shrink_limit(ShrinkLimit::Content)
}

fn status_bar(item_factor: f32) -> Row {
  Row::new()
    .spacing(SPACING)
    .child(approvals_item(item_factor).shrink_order(1))
    .child(label(RUN).flex_shrink(1.0).shrink_order(2))
}

/// Natural widths of the item's icon, count and detail, and of the item and
/// the run label.
fn natural() -> (Vec<f32>, Vec<f32>) {
  let (_, layout) = layout_once(Row::new().child(status_bar(1.0)), WIDE, LINE_HEIGHT);
  let bar = &layout.children[0].result;
  (child_widths(&bar.children[0].result), child_widths(bar))
}

/// The item's width with its detail at zero: padding, icon, count, and the
/// spacing around the count.
fn item_content_width(parts: &[f32]) -> f32 {
  PAD * 2.0 + parts[0] + SPACING + parts[1] + SPACING
}

#[test]
fn give_way_content_limit_holds_with_the_largest_factor() {
  let (parts, _) = natural();
  let row = Row::new()
    .spacing(SPACING)
    .child(approvals_item(1000.0))
    .child(label(RUN).flex_shrink(1.0));
  let (tree, layout) = layout_once(row, 60.0, LINE_HEIGHT);

  let item = &layout.children[0].result;
  assert_close(
    item.size.width,
    item_content_width(&parts),
    "the item stops at its content",
  );
  assert_eq!(child_widths(item)[1], parts[1], "the count keeps its width");
  assert!(
    drawn_texts(&tree, &layout).iter().any(|text| text == COUNT),
    "the count is drawn whole"
  );
}

#[test]
fn give_way_content_limit_lets_the_inner_detail_give_way_first() {
  let (parts, bar) = natural();
  let natural_bar = bar[0] + SPACING + bar[1];
  let (tree, layout) = layout_once(status_bar(1.0), natural_bar - 10.0, LINE_HEIGHT);

  let widths = child_widths(&layout);
  assert_close(widths[0], bar[0] - 10.0, "the item gives way first");
  assert_eq!(widths[1], bar[1], "the run label keeps its width");
  let item = child_widths(&layout.children[0].result);
  assert_eq!(item[1], parts[1], "the count keeps its width");
  assert_close(item[2], parts[2] - 10.0, "the detail inside the item shrinks");
  let drawn = drawn_texts(&tree, &layout);
  assert_eq!(drawn[0], COUNT);
  assert!(drawn[1].ends_with('…'), "the detail is ellipsized: {drawn:?}");
  assert_eq!(drawn[2], RUN);
}

#[test]
fn give_way_content_limit_spills_into_the_next_order() {
  let (parts, bar) = natural();
  let content = item_content_width(&parts);
  let spill = 12.0;
  let (tree, layout) = layout_once(status_bar(1.0), content + SPACING + bar[1] - spill, LINE_HEIGHT);

  let widths = child_widths(&layout);
  assert_close(widths[0], content, "the item stops at its content");
  assert_close(widths[1], bar[1] - spill, "the run label takes the rest");
  assert_eq!(drawn_texts(&tree, &layout)[0], COUNT);
}

#[test]
fn give_way_content_limit_never_goes_below_an_explicit_minimum() {
  let item = Row::new()
    .spacing(0.0)
    .child(Rect::new(20.0, 10.0))
    .child(Rect::new(80.0, 10.0).flex_shrink(1.0))
    .flex_shrink(1.0)
    .shrink_limit(ShrinkLimit::Content)
    .min_width(50.0);
  let (_, layout) = layout_once(Row::new().spacing(0.0).child(item), 10.0, 10.0);
  assert_eq!(child_widths(&layout)[0], 50.0);
}

#[test]
fn give_way_content_limit_in_a_column() {
  let panel = Column::new()
    .spacing(0.0)
    .child(Rect::new(10.0, 20.0))
    .child(Rect::new(10.0, 100.0).flex_shrink(1.0))
    .flex_shrink(4.0)
    .shrink_limit(ShrinkLimit::Content);
  let column = Column::new()
    .spacing(0.0)
    .child(panel)
    .child(Rect::new(10.0, 200.0).flex_shrink(1.0));
  let (_, layout) = layout_once(column, 10.0, 150.0);
  assert_eq!(child_heights(&layout), [20.0, 130.0]);
  assert_eq!(child_heights(&layout.children[0].result), [20.0, 0.0]);
}
