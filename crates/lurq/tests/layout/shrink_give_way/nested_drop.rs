//! A shrunk child whose own line drops a child to fit gives the released
//! space back to its line: it is laid out at what it still holds, and the line
//! is distributed again, instead of keeping a blank where the dropped child
//! was.

use lurq::{
  app::{App, Tree},
  components::{Rect, Row},
  layout::{layout_kind::ShrinkLimit, layout_result::LayoutResult},
};

use super::{
  COUNT, DETAIL, LINE_HEIGHT, SPACING, WIDE, assert_close, child_widths, drawn_texts, label, layout_at, layout_once,
  tight,
};

const PAD: f32 = 8.0;
const GAP: f32 = 4.0;
const ICON: f32 = 14.0;
const ASKS_MIN: f32 = 100.0;

/// A status item: padding, an icon and words that drop out whole, giving way
/// last.
fn runner_item() -> Row {
  Row::new()
    .spacing(GAP)
    .padding_horizontal(PAD)
    .child(Rect::new(ICON, ICON))
    .child(label(COUNT).flex_shrink(1.0).shrink_limit(ShrinkLimit::Drop))
    .flex_shrink(1.0)
    .shrink_order(9)
    .shrink_limit(ShrinkLimit::Content)
}

/// An item whose words drop one line further down: the icon, then a count
/// line with the count and its words.
fn approvals_item() -> Row {
  let count_line = Row::new()
    .child(label("2"))
    .child(label(COUNT).flex_shrink(1.0).shrink_limit(ShrinkLimit::Drop))
    .flex_shrink(1.0)
    .shrink_limit(ShrinkLimit::Content);
  Row::new()
    .spacing(GAP)
    .padding_horizontal(PAD)
    .child(Rect::new(ICON, ICON))
    .child(count_line)
    .flex_shrink(1.0)
    .shrink_order(8)
    .shrink_limit(ShrinkLimit::Content)
}

/// What the request asks trims first, down to 100 px; then `item` gives way.
fn bar(item: Row) -> Row {
  Row::new()
    .spacing(SPACING)
    .child(label(DETAIL).flex_shrink(1.0).min_width(ASKS_MIN))
    .child(item)
}

fn natural(item: Row) -> Vec<f32> {
  let (_, layout) = layout_once(Row::new().child(bar(item)), WIDE, LINE_HEIGHT);
  child_widths(&layout.children[0].result)
}

/// The width at which the item has to give up `amount` after the request
/// reached its minimum.
fn squeezed(natural: &[f32], amount: f32) -> f32 {
  ASKS_MIN + SPACING + natural[1] - amount
}

/// What the item's children take: padding, spacing and the ones still there.
fn held(item: &LayoutResult) -> f32 {
  let kept: Vec<f32> = item
    .children
    .iter()
    .filter(|child| !child.result.is_dropped())
    .map(|child| child.result.size.width)
    .collect();
  PAD * 2.0 + kept.iter().sum::<f32>() + GAP * (kept.len() as f32 - 1.0)
}

#[test]
fn give_way_nested_drop_releases_its_space() {
  let natural = natural(runner_item());
  let width = squeezed(&natural, 10.0);
  let (tree, layout) = layout_once(bar(runner_item()), width, LINE_HEIGHT);

  let item = &layout.children[1].result;
  assert!(item.children[1].result.is_dropped(), "the words drop");
  assert_close(item.size.width, PAD * 2.0 + ICON, "the item keeps no blank");
  assert_close(item.size.width, held(item), "the item is as wide as what it holds");
  assert_close(
    layout.children[0].result.size.width,
    width - SPACING - item.size.width,
    "the request takes the released space back",
  );
  assert!(
    !drawn_texts(&tree, &layout).contains(&COUNT.to_string()),
    "the words are not drawn"
  );
}

#[test]
fn give_way_drop_two_lines_down_releases_its_space() {
  let natural = natural(approvals_item());
  let width = squeezed(&natural, 10.0);
  let (tree, layout) = layout_once(bar(approvals_item()), width, LINE_HEIGHT);

  let item = &layout.children[1].result;
  let count_line = &item.children[1].result;
  assert!(count_line.children[1].result.is_dropped(), "the words drop");
  assert_close(
    count_line.size.width,
    count_line.children[0].result.size.width,
    "the count line holds only the count",
  );
  assert_close(item.size.width, held(item), "the item is as wide as what it holds");
  assert_close(
    layout.children[0].result.size.width + SPACING + item.size.width,
    width,
    "the line is filled again",
  );
  assert!(drawn_texts(&tree, &layout).contains(&"2".to_string()));
}

#[test]
fn give_way_nested_drop_restores_on_widen() {
  let natural = natural(runner_item());
  let wide = natural[0] + SPACING + natural[1];
  let narrow = squeezed(&natural, 10.0);
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(bar(runner_item()));
  let passes: Vec<Vec<f32>> = [wide, narrow, wide, narrow]
    .into_iter()
    .map(|width| child_widths(&layout_at(&mut tree, &mut app, tight(width, LINE_HEIGHT))))
    .collect();
  assert_eq!(passes[0], natural, "the wide bar has its natural widths");
  assert_close(
    passes[1][1],
    PAD * 2.0 + ICON,
    "the narrow bar releases the words' space",
  );
  assert_eq!(passes[2], passes[0], "the wide bar is restored");
  assert_eq!(passes[3], passes[1], "the narrow bar releases it again");
}
