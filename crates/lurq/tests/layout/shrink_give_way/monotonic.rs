//! What gives way whole only accumulates as a line narrows: a child dropped
//! (or collapsed) at one width is never back at a narrower one, whatever a
//! later order's drop frees.

use lurq::{
  app::{App, Tree},
  components::{Rect, Row, Spacer},
  layout::{layout_kind::ShrinkLimit, layout_result::LayoutResult},
  node::{Element, dimension::Dimension},
};

use super::{
  DETAIL, LINE_HEIGHT, SPACING, TIME, WIDE, assert_close, child_widths, drawn_texts, label, layout_at, layout_once,
  tight,
};

const VERSION: &str = "v0.1.0";

/// Paths of the leaves hidden in `layout`: inside a dropped child.
fn hidden_leaves(layout: &LayoutResult) -> Vec<String> {
  fn walk(result: &LayoutResult, path: &str, hidden: bool, out: &mut Vec<String>) {
    let hidden = hidden || result.is_dropped();
    if result.children.is_empty() {
      if hidden {
        out.push(path.to_owned());
      }
      return;
    }
    for (index, child) in result.children.iter().enumerate() {
      walk(&child.result, &format!("{path}/{index}"), hidden, out);
    }
  }
  let mut out = Vec::new();
  walk(layout, "", false, &mut out);
  out
}

/// The most padding an item of these lines has on its end.
const MAX_PADDING: f32 = 8.0;
const TOLERANCE: f32 = 0.5;

fn has_dropped(result: &LayoutResult) -> bool {
  result
    .children
    .iter()
    .any(|child| child.result.is_dropped() || has_dropped(&child.result))
}

/// No blank inside an item: its last child that is still there reaches its
/// end, padding aside.
fn assert_items_filled(name: &str, width: f32, result: &LayoutResult) {
  for child in &result.children {
    let item = &child.result;
    if item.is_dropped() || item.children.is_empty() {
      continue;
    }
    let end = item
      .children
      .iter()
      .filter(|inner| !inner.result.is_dropped())
      .map(|inner| inner.offset.x + inner.result.size.width)
      .fold(0.0_f32, f32::max);
    assert!(
      end >= item.size.width - MAX_PADDING - TOLERANCE,
      "{name} at {width}: an item {} wide holds only {end}",
      item.size.width
    );
    assert_items_filled(name, width, item);
  }
}

/// No unexplained blank at the end of the line: it is filled, or every child
/// that is there and gave nothing way whole is back at its natural size.
fn assert_line_filled(name: &str, width: f32, spacing: f32, layout: &LayoutResult, natural: &[f32]) {
  let kept: Vec<(usize, &LayoutResult)> = layout
    .children
    .iter()
    .enumerate()
    .filter(|(_, child)| !child.result.is_dropped())
    .map(|(index, child)| (index, child.result.as_ref()))
    .collect();
  let used: f32 =
    kept.iter().map(|(_, child)| child.size.width).sum::<f32>() + spacing * (kept.len() as f32 - 1.0).max(0.0);
  if used >= width - TOLERANCE {
    return;
  }
  for (index, child) in kept {
    assert!(
      has_dropped(child) || (child.size.width - natural[index]).abs() <= TOLERANCE,
      "{name} at {width}: {} px blank while child {index} is {} wide, not its natural {}",
      width - used,
      child.size.width,
      natural[index]
    );
  }
}

/// Every leaf's width, by path.
fn leaf_widths(result: &LayoutResult, path: &str, out: &mut Vec<(String, f32)>) {
  if result.children.is_empty() {
    out.push((path.to_owned(), result.size.width));
  }
  for (index, child) in result.children.iter().enumerate() {
    leaf_widths(&child.result, &format!("{path}/{index}"), out);
  }
}

/// Narrows `root` from `start` (at least its natural width) to `end` in
/// 0.5 px steps through one tree and checks that every leaf hidden at one
/// width stays hidden at the next, without blanks. Returns the hidden leaves
/// at each width and the largest move of a leaf between two steps that hid
/// nothing new.
fn assert_monotone(
  name: &str,
  root: impl Into<Element>,
  spacing: f32,
  start: f32,
  end: f32,
) -> (Vec<(f32, Vec<String>)>, f32) {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(root);
  let mut passes: Vec<(f32, Vec<String>)> = Vec::new();
  let mut natural = Vec::new();
  let mut previous_leaves: Vec<(String, f32)> = Vec::new();
  let mut largest_move = 0.0_f32;
  let steps = ((start - end) * 2.0) as usize;
  for step in 0..=steps {
    let width = start - step as f32 * 0.5;
    let layout = layout_at(&mut tree, &mut app, tight(width, LINE_HEIGHT));
    if step == 0 {
      natural = child_widths(&layout);
    }
    let hidden = hidden_leaves(&layout);
    let mut leaves = Vec::new();
    leaf_widths(&layout, "", &mut leaves);
    if let Some((wider, before)) = passes.last() {
      let back: Vec<&String> = before.iter().filter(|leaf| !hidden.contains(leaf)).collect();
      assert!(
        back.is_empty(),
        "{name}: {back:?} hidden at {wider} but shown at {width}"
      );
      if &hidden == before {
        for ((_, now), (_, then)) in leaves.iter().zip(&previous_leaves) {
          largest_move = largest_move.max((now - then).abs());
        }
      }
    }
    assert_items_filled(name, width, &layout);
    assert_line_filled(name, width, spacing, &layout, &natural);
    previous_leaves = leaves;
    passes.push((width, hidden));
  }
  (passes, largest_move)
}

/// An item with an icon, a detail that trims in the item's second order and
/// a time that drops in its first: it collapses to what it holds without the
/// time, not to its icon.
#[test]
fn give_way_monotone_collapse_keeps_what_trims_after_the_drop() {
  let item = || {
    Row::new()
      .spacing(4.0)
      .padding_horizontal(8.0)
      .child(Rect::new(14.0, 14.0))
      .child(label(DETAIL).flex_shrink(1.0).shrink_order(1))
      .child(label(TIME).flex_shrink(1.0).shrink_limit(ShrinkLimit::Drop))
      .flex_shrink(1.0)
      .shrink_limit(ShrinkLimit::Content)
  };
  let (_, natural) = layout_once(Row::new().child(item().width(WIDE)), WIDE * 2.0, LINE_HEIGHT);
  let parts = child_widths(&natural.children[0].result);
  let natural_item = 16.0 + parts.iter().sum::<f32>() + 8.0;
  let without_time = natural_item - parts[2] - 4.0;

  let (tree, layout) = layout_once(Row::new().child(item()), natural_item - 0.5, LINE_HEIGHT);
  let held = &layout.children[0].result;
  assert!(held.children[2].result.is_dropped(), "the time drops");
  assert_close(held.size.width, without_time, "the item keeps the detail whole");
  assert_eq!(drawn_texts(&tree, &layout), [DETAIL]);

  let (tree, layout) = layout_once(Row::new().child(item()), without_time - 18.0, LINE_HEIGHT);
  let held = &layout.children[0].result;
  assert_close(held.size.width, without_time - 18.0, "then the detail trims");
  assert!(drawn_texts(&tree, &layout)[0].ends_with('…'));
}

/// The status bar's shape: room between the groups that goes first, what a
/// request asks (trims to 100 px), an item whose time drops out, the version
/// (drops out one order later) and a fixed end.
fn status_bar() -> Row {
  let times = Row::new()
    .spacing(4.0)
    .padding_horizontal(8.0)
    .child(Rect::new(14.0, 14.0))
    .child(label(TIME).flex_shrink(1.0).shrink_limit(ShrinkLimit::Drop))
    .flex_shrink(1.0)
    .shrink_order(2)
    .shrink_limit(ShrinkLimit::Content);
  Row::new()
    .spacing(SPACING)
    .child(
      Spacer::new()
        .width(Dimension::Pct(100.0))
        .flex_shrink(1.0)
        .shrink_order(-1),
    )
    .child(label("Designer asks to use pencil").flex_shrink(1.0).min_width(100.0))
    .child(times)
    .child(
      label(VERSION)
        .flex_shrink(1.0)
        .shrink_order(3)
        .shrink_limit(ShrinkLimit::Drop),
    )
    .child(Rect::new(60.0, 10.0))
}

#[test]
fn give_way_monotone_a_later_drop_does_not_bring_an_earlier_one_back() {
  let (_, natural) = layout_once(Row::new().child(status_bar().width(WIDE)), WIDE * 2.0, LINE_HEIGHT);
  let bar = &natural.children[0].result;
  // Without the room: the asks, the time item, the version and the end.
  let content: f32 = child_widths(bar)[1..].iter().sum::<f32>() + SPACING * 4.0;
  let (passes, _) = assert_monotone("status bar", status_bar(), SPACING, content + 20.0, 150.0);

  let time_leaf = "/2/1";
  let version_leaf = "/3";
  let time_only = passes
    .iter()
    .find(|(_, hidden)| hidden.iter().any(|leaf| leaf == time_leaf) && !hidden.iter().any(|leaf| leaf == version_leaf));
  assert!(time_only.is_some(), "some width drops the time and keeps the version");
  let both = passes
    .iter()
    .find(|(_, hidden)| hidden.iter().any(|leaf| leaf == version_leaf));
  let (at, hidden) = both.expect("a narrower width drops the version");
  assert!(
    hidden.iter().any(|leaf| leaf == time_leaf),
    "at {at} the time stays dropped"
  );
}

/// A small deterministic generator (an LCG) for line shapes.
struct Lines(u64);

impl Lines {
  fn next(&mut self, below: u32) -> u32 {
    self.0 = self
      .0
      .wrapping_mul(6364136223846793005)
      .wrapping_add(1442695040888963407);
    ((self.0 >> 33) % below as u64) as u32
  }

  fn size(&mut self, from: u32, to: u32) -> f32 {
    (from + self.next(to - from + 1)) as f32
  }

  fn order(&mut self) -> i32 {
    self.next(5) as i32 - 1
  }

  fn child(&mut self) -> Element {
    match self.next(6) {
      0 => Rect::new(self.size(10, 60), 10.0).into(),
      1 => {
        let rect = Rect::new(self.size(30, 120), 10.0)
          .flex_shrink(self.size(1, 3))
          .shrink_order(self.order());
        if self.next(2) == 0 {
          rect.min_width(self.size(5, 30)).into()
        } else {
          rect.into()
        }
      }
      2 => Rect::new(self.size(15, 60), 10.0)
        .flex_shrink(1.0)
        .shrink_order(self.order())
        .shrink_limit(ShrinkLimit::Drop)
        .into(),
      3 => Rect::new(self.size(40, 120), 10.0)
        .flex_shrink(self.size(1, 2))
        .shrink_order(self.order())
        .shrink_drop_below(self.size(10, 50))
        .into(),
      _ => self.item(0),
    }
  }

  /// A nested item: a fixed icon, words that drop and, sometimes, a part
  /// that trims or another item.
  fn item(&mut self, depth: u32) -> Element {
    let mut item = Row::new()
      .spacing(self.size(0, 4))
      .padding_horizontal(self.size(0, 8))
      .child(Rect::new(10.0, 10.0))
      .child(
        Rect::new(self.size(20, 70), 10.0)
          .flex_shrink(1.0)
          .shrink_order(self.order())
          .shrink_limit(ShrinkLimit::Drop),
      );
    match self.next(3) {
      0 => {
        item = item.child(
          Rect::new(self.size(20, 60), 10.0)
            .flex_shrink(1.0)
            .shrink_order(self.order()),
        )
      }
      1 if depth < 2 => item = item.child(self.item(depth + 1)),
      _ => {}
    }
    let limit = if self.next(2) == 0 {
      ShrinkLimit::Content
    } else {
      ShrinkLimit::MinSize
    };
    item
      .flex_shrink(self.size(1, 3))
      .shrink_order(self.order())
      .shrink_limit(limit)
      .into()
  }

  fn line(&mut self) -> Row {
    let spacing = self.size(0, 6);
    let count = 3 + self.next(6);
    Row::new()
      .spacing(spacing)
      .with_children((0..count).map(|_| self.child()))
  }
}

#[test]
fn give_way_monotone_over_generated_lines() {
  let mut lines = Lines(0x6c75_7271);
  let mut largest_move = 0.0_f32;
  for case in 0..30 {
    let natural_line = |lines: &mut Lines| {
      let seed = lines.0;
      let line = lines.line();
      (seed, line)
    };
    let (seed, line) = natural_line(&mut lines);
    let (_, natural) = layout_once(Row::new().child(line), WIDE, LINE_HEIGHT);
    let width = natural.children[0].result.size.width;
    let spacing = Lines(seed).size(0, 6);
    let mut replay = Lines(seed);
    let (_, moved) = assert_monotone(
      &format!("line {case}"),
      replay.line(),
      spacing,
      width + 10.0,
      (width * 0.2).max(10.0),
    );
    largest_move = largest_move.max(moved);
  }
  eprintln!("largest move of a leaf in a 0.5 px step: {largest_move}");
}
