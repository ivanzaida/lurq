//! What gives way whole only accumulates as a line narrows: a child dropped
//! (or collapsed) at one width is never back at a narrower one, whatever a
//! later order's drop frees.

use lurq::{
  app::{App, Tree},
  components::{Rect, Row, Spacer},
  layout::{layout_kind::ShrinkLimit, layout_result::LayoutResult},
  node::{Element, dimension::Dimension},
};

use super::{LINE_HEIGHT, SPACING, TIME, WIDE, child_widths, label, layout_at, layout_once, tight};

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

/// Narrows `root` from `start` to `end` in 0.5 px steps through one tree and
/// checks that every leaf hidden at one width stays hidden at the next.
/// Returns the hidden leaves at each width.
fn assert_monotone(name: &str, root: impl Into<Element>, start: f32, end: f32) -> Vec<(f32, Vec<String>)> {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(root);
  let mut passes: Vec<(f32, Vec<String>)> = Vec::new();
  let steps = ((start - end) * 2.0) as usize;
  for step in 0..=steps {
    let width = start - step as f32 * 0.5;
    let hidden = hidden_leaves(&layout_at(&mut tree, &mut app, tight(width, LINE_HEIGHT)));
    if let Some((wider, before)) = passes.last() {
      let back: Vec<&String> = before.iter().filter(|leaf| !hidden.contains(leaf)).collect();
      assert!(
        back.is_empty(),
        "{name}: {back:?} hidden at {wider} but shown at {width}"
      );
    }
    passes.push((width, hidden));
  }
  passes
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
  let passes = assert_monotone("status bar", status_bar(), content + 20.0, 150.0);

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
  for case in 0..60 {
    let natural_line = |lines: &mut Lines| {
      let seed = lines.0;
      let line = lines.line();
      (seed, line)
    };
    let (seed, line) = natural_line(&mut lines);
    let (_, natural) = layout_once(Row::new().child(line), WIDE, LINE_HEIGHT);
    let width = natural.children[0].result.size.width;
    let mut replay = Lines(seed);
    assert_monotone(
      &format!("line {case}"),
      replay.line(),
      width + 10.0,
      (width * 0.2).max(10.0),
    );
  }
}
