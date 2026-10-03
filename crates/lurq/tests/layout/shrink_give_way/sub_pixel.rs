//! Sharing in whole pixels. A line that uses orders or limits shares each
//! order's overflow in whole pixels, except for the child with the largest
//! factor, which takes the fraction; its shares follow a resize continuously.
//! A line without them shares exactly by factor, as before.

use lurq::{
  app::{App, Tree},
  components::{Rect, Row},
};

use super::{
  DETAIL, LINE_HEIGHT, RUN, SPACING, TIME, WIDE, assert_close, child_widths, drawn_texts, label, layout_at,
  layout_once, tight,
};

/// One 100 px rect per factor, with an optional order and minimum width each.
fn rects(children: &[(f32, Option<i32>, Option<f32>)]) -> Row {
  Row::new()
    .spacing(0.0)
    .with_children(children.iter().map(|&(factor, order, min_width)| {
      let mut rect = Rect::new(100.0, 10.0).flex_shrink(factor);
      if let Some(order) = order {
        rect = rect.shrink_order(order);
      }
      if let Some(min_width) = min_width {
        rect = rect.min_width(min_width);
      }
      rect
    }))
}

fn widths_at(row: Row, width: f32) -> Vec<f32> {
  child_widths(&layout_once(row, width, 10.0).1)
}

fn assert_widths(actual: &[f32], expected: &[f32]) {
  assert_eq!(actual.len(), expected.len(), "{actual:?} vs {expected:?}");
  for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
    assert_close(actual, expected, &format!("child {index} of {actual:?}"));
  }
}

#[test]
fn give_way_whole_pixels_sub_pixel_overflow_ellipsizes_one_label_only() {
  let labels = || {
    Row::new()
      .spacing(SPACING)
      .child(label(DETAIL).flex_shrink(1.0).shrink_order(1))
      .child(label(RUN).flex_shrink(1.0).shrink_order(1))
      .child(label(TIME).flex_shrink(1.0).shrink_order(1))
  };
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
fn give_way_whole_pixels_sub_pixel_share_goes_to_the_largest_factor() {
  // Overflow 5 by factors 10 and 1: the second child's 0.45 px stays with
  // the first.
  let widths = widths_at(rects(&[(10.0, Some(1), None), (1.0, Some(1), None)]), 195.0);
  assert_eq!(widths, [95.0, 100.0]);
}

#[test]
fn give_way_whole_pixels_shares_stay_proportional() {
  let widths = widths_at(rects(&[(10.0, Some(1), None), (1.0, Some(1), None)]), 178.0);
  assert_widths(&widths, &[80.0, 98.0]);
}

#[test]
fn give_way_line_without_rules_shares_exactly_by_factor() {
  let widths = widths_at(rects(&[(10.0, None, None), (1.0, None, None)]), 195.0);
  assert_widths(&widths, &[100.0 - 50.0 / 11.0, 100.0 - 5.0 / 11.0]);
  let even = widths_at(rects(&[(1.0, None, None); 10]), 990.1);
  assert!(even.iter().all(|width| (width - 99.01).abs() < 0.01), "{even:?}");
}

#[test]
fn give_way_floor_after_a_small_share_keeps_the_overflow_in_its_order() {
  // Factors 1 and 9 over a 5 px overflow, the second stopping at 95.4.
  let plain = widths_at(rects(&[(1.0, None, None), (9.0, None, Some(95.4))]), 195.0);
  assert_widths(&plain, &[99.5, 95.5]);
  let ordered = widths_at(rects(&[(1.0, Some(1), None), (9.0, Some(1), Some(95.4))]), 195.0);
  assert_widths(&ordered, &[99.6, 95.4]);
}

#[test]
fn give_way_floor_after_a_small_share_does_not_reach_the_next_order() {
  let row = rects(&[(1.0, None, None), (9.0, None, Some(95.4)), (1.0, Some(1), None)]);
  let widths = widths_at(row, 295.0);
  assert_eq!(widths[2], 100.0, "the next order keeps its width");
  assert_close(widths[0] + widths[1], 195.0, "the first order absorbs the overflow");
  assert!(widths[1] >= 95.4, "the floor holds: {widths:?}");
}

#[test]
fn give_way_floor_of_the_largest_share_passes_the_rest_on() {
  // Three equal factors over a 0.9 px overflow, the first stopping at 99.5.
  let plain = widths_at(
    rects(&[(1.0, None, Some(99.5)), (1.0, None, None), (1.0, None, None)]),
    299.1,
  );
  assert_widths(&plain, &[99.7, 99.7, 99.7]);
  let ordered = widths_at(
    rects(&[(1.0, Some(1), Some(99.5)), (1.0, Some(1), None), (1.0, Some(1), None)]),
    299.1,
  );
  assert_close(ordered.iter().sum(), 299.1, "the overflow is absorbed");
  assert!(ordered[0] >= 99.5, "the floor holds: {ordered:?}");
}

/// Narrows `row` from 1000 to 950 px in 0.1 px steps through one tree and
/// checks that the line always fits and no child moves by more than a pixel
/// between steps.
fn assert_continuous_sweep(row: Row) {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(row);
  let mut previous: Option<Vec<f32>> = None;
  for step in 0..=500 {
    let width = 1000.0 - step as f32 * 0.1;
    let widths = child_widths(&layout_at(&mut tree, &mut app, tight(width, 10.0)));
    assert!(
      (widths.iter().sum::<f32>() - width).abs() < 0.01,
      "the line fits at {width}: {widths:?}"
    );
    if let Some(previous) = &previous {
      for (index, (now, before)) in widths.iter().zip(previous).enumerate() {
        assert!(
          (now - before).abs() <= 1.0 + 0.01,
          "child {index} jumps from {before} to {now} at {width}: {widths:?}"
        );
      }
    }
    previous = Some(widths);
  }
}

#[test]
fn give_way_whole_pixels_follow_a_resize_continuously() {
  assert_continuous_sweep(rects(&[(1.0, Some(1), None); 10]));
}

#[test]
fn give_way_whole_pixels_with_mixed_factors_and_floors_follow_a_resize_continuously() {
  assert_continuous_sweep(rects(&[
    (1.0, Some(1), None),
    (2.0, Some(1), None),
    (3.0, Some(1), Some(95.0)),
    (1.0, Some(1), None),
    (5.0, Some(1), None),
    (1.0, Some(2), None),
    (1.0, Some(2), Some(90.0)),
    (2.0, Some(2), None),
    (1.0, None, None),
    (1.0, None, Some(99.0)),
  ]));
}

#[test]
fn give_way_line_without_rules_follows_a_resize_continuously() {
  assert_continuous_sweep(rects(&[(1.0, None, None); 10]));
}
