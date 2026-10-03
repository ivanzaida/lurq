//! `ShrinkLimit::Drop`: a child keeps its natural size or leaves the line.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::events::{MouseButton, MouseEvent},
  components::{Rect, Row, Text},
  layout::{Alignment, layout_kind::ShrinkLimit},
  node::EventHandler,
};

use super::{DETAIL, LINE_HEIGHT, SPACING, TIME, WIDE, assert_close, child_widths, drawn_texts, label, layout_once};
use crate::support::{pointer_click, run_pass};

const STOP: f32 = 30.0;

/// A detail that gives way second, a time that drops first, a fixed button.
fn line() -> Row {
  Row::new()
    .spacing(SPACING)
    .child(label(DETAIL).flex_shrink(1.0).shrink_order(2))
    .child(
      label(TIME)
        .flex_shrink(1.0)
        .shrink_order(1)
        .shrink_limit(ShrinkLimit::Drop),
    )
    .child(Rect::new(STOP, 10.0))
}

fn natural_widths() -> Vec<f32> {
  let (_, layout) = layout_once(Row::new().child(line()), WIDE, LINE_HEIGHT);
  child_widths(&layout.children[0].result)
}

fn natural_line(widths: &[f32]) -> f32 {
  widths.iter().sum::<f32>() + SPACING * 2.0
}

#[test]
fn give_way_drop_removes_the_child_and_its_spacing() {
  let natural = natural_widths();
  let (tree, layout) = layout_once(line(), natural_line(&natural) - 3.0, LINE_HEIGHT);

  let time = &layout.children[1];
  assert!(time.result.is_dropped(), "the time is dropped");
  assert_eq!(time.result.size.width, 0.0);
  assert_eq!(
    layout.children[0].result.size.width, natural[0],
    "the detail keeps its width"
  );
  assert_close(
    layout.children[2].offset.x,
    natural[0] + SPACING,
    "the button follows the detail with one spacing",
  );
  assert_eq!(drawn_texts(&tree, &layout), [DETAIL], "the dropped time is not drawn");
}

#[test]
fn give_way_drop_waits_until_its_order_runs_out_of_room() {
  let natural = natural_widths();
  let row = Row::new()
    .spacing(SPACING)
    .child(label(DETAIL).flex_shrink(1.0).shrink_order(1))
    .child(
      label(TIME)
        .flex_shrink(1.0)
        .shrink_order(1)
        .shrink_limit(ShrinkLimit::Drop),
    )
    .child(Rect::new(STOP, 10.0));
  let (tree, layout) = layout_once(row, natural_line(&natural) - 5.0, LINE_HEIGHT);

  assert!(
    !layout.children[1].result.is_dropped(),
    "the detail of the same order absorbs the overflow"
  );
  assert_close(
    layout.children[0].result.size.width,
    natural[0] - 5.0,
    "the detail shrinks",
  );
  assert_eq!(drawn_texts(&tree, &layout)[1], TIME);
}

#[test]
fn give_way_drop_frees_room_for_a_shrunk_sibling_of_its_order() {
  let row = Row::new()
    .spacing(10.0)
    .child(Rect::new(100.0, 10.0).flex_shrink(1.0).min_width(80.0))
    .child(Rect::new(50.0, 10.0).flex_shrink(1.0).shrink_limit(ShrinkLimit::Drop));
  // Overflow 30: the first child can absorb 20, so the second drops, which
  // frees 60 and leaves the first at its natural width.
  let (_, layout) = layout_once(row, 130.0, 10.0);
  assert_eq!(child_widths(&layout), [100.0, 0.0]);
  assert!(layout.children[1].result.is_dropped());
  assert_eq!(layout.size.width, 130.0);
}

/// Clicks the button inside a droppable row laid out in a line `width` wide;
/// returns whether the row was dropped and how many clicks reached the button.
fn click_droppable_button(width: f32) -> (bool, usize) {
  let clicks = Arc::new(AtomicUsize::new(0));
  let counter = clicks.clone();
  let on_click = EventHandler::new(move |_: &MouseEvent| {
    counter.fetch_add(1, Ordering::SeqCst);
  });
  // Start-aligned and not clipped, the button inside a dropped row stays
  // where a click would reach it if the dropped row were still hit-tested.
  let droppable = Row::new()
    .align_items(Alignment::Start)
    .overflow_visible()
    .child(Rect::new(40.0, 20.0).on_click(on_click))
    .flex_shrink(1.0)
    .shrink_limit(ShrinkLimit::Drop);
  let mut tree = lurq::app::Tree::new();
  tree.set_root(Row::new().size(width, 20.0).child(droppable));
  run_pass(&mut tree);

  let layout = tree.last_layout().expect("layout").clone();
  let row = &layout.children[0];
  let button = &row.result.children[0];
  let x = row.offset.x + button.offset.x + 5.0;
  let y = row.offset.y + button.offset.y + 5.0;
  pointer_click(&mut tree, x, y, MouseButton::Left);
  (row.result.is_dropped(), clicks.load(Ordering::SeqCst))
}

#[test]
fn give_way_dropped_child_is_not_hit() {
  assert_eq!(
    click_droppable_button(50.0),
    (false, 1),
    "a fitting row takes the click"
  );
  assert_eq!(click_droppable_button(30.0), (true, 0), "a dropped row takes no clicks");
}

/// The quads drawn for a droppable, unclipped row in a line `width` wide.
fn droppable_quads(width: f32) -> usize {
  let droppable = Row::new()
    .overflow_visible()
    .child(Rect::new(40.0, 20.0).background("#ff0000"))
    .child(Text::new(TIME).nowrap())
    .flex_shrink(1.0)
    .shrink_limit(ShrinkLimit::Drop);
  let (tree, layout) = layout_once(Row::new().child(droppable), width, LINE_HEIGHT);
  tree.resolve_quads(&layout).len()
}

#[test]
fn give_way_dropped_child_is_not_drawn() {
  assert!(droppable_quads(WIDE) >= 2, "a fitting row draws its rect and text");
  assert_eq!(droppable_quads(30.0), 0, "a dropped row draws nothing");
}

#[test]
fn give_way_dropped_child_does_not_count_toward_a_content_limit() {
  let item = Row::new()
    .spacing(SPACING)
    .child(Rect::new(14.0, 14.0))
    .child(label(TIME).flex_shrink(1.0).shrink_limit(ShrinkLimit::Drop))
    .flex_shrink(1.0)
    .shrink_limit(ShrinkLimit::Content);
  let (_, layout) = layout_once(Row::new().child(item), 5.0, LINE_HEIGHT);
  let item = &layout.children[0].result;
  assert_eq!(item.size.width, 14.0);
  assert!(item.children[1].result.is_dropped());
}
