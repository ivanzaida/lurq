//! `.drop_outside(...)`: a child of a `Stack` is laid out only while the stack is within bounds.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    Tree,
    events::{MouseButton, MouseEvent},
  },
  components::{Column, Rect, Row, Stack},
  layout::{Alignment, Constraints, Size, StackAlignment, layout_kind::SizeBounds, layout_result::LayoutResult},
  node::{Element, EventHandler, dimension::Dimension},
};

use super::PassLayoutExt;
use crate::support::{pointer_click, run_pass};

fn room() -> Constraints {
  Constraints::loose(Size::new(400.0, 300.0))
}

fn laid_out(root: impl Into<Element>, constraints: Constraints) -> (Tree, LayoutResult) {
  let mut tree = Tree::new();
  tree.set_root(root.into());
  let layout = tree.pass_layout(constraints).unwrap();
  (tree, layout)
}

/// A picture `width` by 90 with a bar over it that lives from 240 wide.
fn picture(width: f32) -> Stack {
  Stack::with(
    StackAlignment::BottomStart,
    vec![
      Element::from(Rect::new(width, 90.0)),
      Element::from(
        Rect::new(Dimension::full(), 36.0)
          .background("#000000")
          .drop_outside(SizeBounds::new().min_width(240.0)),
      ),
    ],
  )
}

#[test]
fn a_child_is_laid_out_while_its_stack_is_within_the_bounds() {
  let (_, layout) = laid_out(picture(240.0), room());
  let bar = &layout.children[1];
  assert!(!bar.result.is_dropped());
  assert_eq!((bar.result.size.width, bar.result.size.height), (240.0, 36.0));
  assert_eq!(bar.offset.y, 54.0, "the bar lies on the stack's bottom edge");
}

#[test]
fn a_child_is_dropped_outside_the_bounds() {
  let (tree, layout) = laid_out(picture(239.0), room());
  let bar = &layout.children[1];
  assert!(bar.result.is_dropped());
  assert_eq!((bar.result.size.width, bar.result.size.height), (0.0, 0.0));
  assert_eq!(
    tree.resolve_quads(&layout).len(),
    0,
    "neither the plain picture nor the dropped bar draws a quad"
  );
}

#[test]
fn the_bounds_are_asked_of_the_size_the_other_children_make() {
  // The stack is offered 400 and is as wide as its picture: the bar is asked of 200, not of 400.
  let (_, layout) = laid_out(picture(200.0), room());
  assert_eq!(layout.size.width, 200.0);
  assert!(layout.children[1].result.is_dropped());
}

#[test]
fn a_child_with_bounds_never_makes_its_stack_larger() {
  let stack = Stack::with(
    StackAlignment::TopStart,
    vec![
      Element::from(Rect::new(100.0, 50.0)),
      Element::from(Rect::new(300.0, 200.0).drop_outside(SizeBounds::new())),
    ],
  );
  let (_, layout) = laid_out(stack, room());
  assert_eq!((layout.size.width, layout.size.height), (100.0, 50.0));
  assert!(
    !layout.children[1].result.is_dropped(),
    "bounds with no bound hold at every size"
  );
}

#[test]
fn a_share_of_the_stack_is_a_share_of_the_size_it_came_to() {
  let stack = Stack::with(
    StackAlignment::TopStart,
    vec![
      Element::from(Rect::new(120.0, 60.0)),
      Element::from(
        Rect::new(Dimension::full(), Dimension::Pct(50.0)).drop_outside(SizeBounds::new().min_width(100.0)),
      ),
    ],
  );
  let (_, layout) = laid_out(stack, room());
  let layer = &layout.children[1].result;
  assert_eq!((layer.size.width, layer.size.height), (120.0, 30.0));
}

#[test]
fn each_end_of_a_bound_is_within_or_not_as_it_says() {
  let cases: [(SizeBounds, [bool; 3]); 6] = [
    (SizeBounds::new().min_width(100.0), [false, true, true]),
    (SizeBounds::new().above_width(100.0), [false, false, true]),
    (SizeBounds::new().max_width(100.0), [true, true, false]),
    (SizeBounds::new().below_width(100.0), [true, false, false]),
    (SizeBounds::new().min_height(100.0), [false, true, true]),
    (
      SizeBounds::new().above_height(100.0).max_height(101.0),
      [false, false, true],
    ),
  ];
  for (bounds, expected) in cases {
    for (side, holds) in [99.0, 100.0, 101.0].into_iter().zip(expected) {
      assert_eq!(bounds.holds(Size::new(side, side)), holds, "{bounds:?} at {side}");
    }
  }
  assert!(!SizeBounds::new().min_width(f32::NAN).holds(Size::new(10.0, 10.0)));
}

#[test]
fn complementary_bounds_show_one_form_at_every_size() {
  let forms = |width: f32| {
    let stack = Stack::with(
      StackAlignment::TopStart,
      vec![
        Element::from(Rect::new(width, 40.0)),
        Element::from(Rect::new(10.0, 10.0).drop_outside(SizeBounds::new().min_width(240.0))),
        Element::from(Rect::new(10.0, 10.0).drop_outside(SizeBounds::new().above_width(96.0).below_width(240.0))),
        Element::from(Rect::new(10.0, 10.0).drop_outside(SizeBounds::new().max_width(96.0))),
      ],
    );
    let (_, layout) = laid_out(stack, room());
    [1, 2, 3].map(|index| !layout.children[index].result.is_dropped())
  };
  assert_eq!(forms(320.0), [true, false, false]);
  assert_eq!(forms(240.0), [true, false, false]);
  assert_eq!(forms(239.5), [false, true, false]);
  assert_eq!(forms(96.5), [false, true, false]);
  assert_eq!(forms(96.0), [false, false, true]);
  assert_eq!(forms(40.0), [false, false, true]);
}

#[test]
fn a_stack_that_changes_size_decides_again() {
  let mut tree = Tree::new();
  tree.set_root(picture(300.0));
  let wide = tree.pass_layout(room()).unwrap();
  assert!(!wide.children[1].result.is_dropped());

  tree.set_root(picture(120.0));
  let narrow = tree.pass_layout(room()).unwrap();
  assert!(
    narrow.children[1].result.is_dropped(),
    "the narrower stack drops its bar"
  );

  tree.set_root(picture(300.0));
  let wide_again = tree.pass_layout(room()).unwrap();
  let bar = &wide_again.children[1].result;
  assert!(!bar.is_dropped(), "and the wider one lays it out again");
  assert_eq!((bar.size.width, bar.size.height), (300.0, 36.0));
}

#[test]
fn the_room_a_stack_is_given_decides_where_it_fills_it() {
  // The same tree, laid out narrower: a stack that fills its room asks its bounds of that room.
  let filling = || {
    Stack::with(
      StackAlignment::BottomStart,
      vec![
        Element::from(Rect::new(Dimension::full(), 90.0)),
        Element::from(Rect::new(Dimension::full(), 36.0).drop_outside(SizeBounds::new().min_width(240.0))),
      ],
    )
    .width(Dimension::full())
  };
  let mut tree = Tree::new();
  tree.set_root(filling());
  let wide = tree.pass_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
  assert!(!wide.children[1].result.is_dropped());
  let narrow = tree.pass_layout(Constraints::loose(Size::new(200.0, 300.0))).unwrap();
  assert!(narrow.children[1].result.is_dropped(), "the same nodes in less room");
  let wide_again = tree.pass_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
  assert!(!wide_again.children[1].result.is_dropped());
}

#[test]
fn stacks_in_stacks_each_ask_their_own_size() {
  let inner = Stack::with(
    StackAlignment::TopStart,
    vec![
      Element::from(Rect::new(80.0, 40.0)),
      Element::from(Rect::new(10.0, 10.0).drop_outside(SizeBounds::new().min_width(200.0))),
    ],
  );
  let outer = Stack::with(
    StackAlignment::TopStart,
    vec![
      Element::from(Rect::new(300.0, 100.0)),
      Element::from(inner.drop_outside(SizeBounds::new().min_width(200.0))),
    ],
  );
  let (_, layout) = laid_out(outer, room());
  let inner = &layout.children[1].result;
  assert!(!inner.is_dropped(), "the outer stack is 300 wide");
  assert!(inner.children[1].result.is_dropped(), "the inner one is 80 wide");
}

#[test]
fn bounds_on_a_child_of_a_line_do_nothing() {
  let row = Row::with(
    0.0,
    Alignment::Start,
    vec![Element::from(
      Rect::new(30.0, 10.0).drop_outside(SizeBounds::new().min_width(1000.0)),
    )],
  );
  let (_, row_layout) = laid_out(row, room());
  assert!(!row_layout.children[0].result.is_dropped());
  let column = Column::with(
    0.0,
    Alignment::Start,
    vec![Element::from(
      Rect::new(30.0, 10.0).drop_outside(SizeBounds::new().min_width(1000.0)),
    )],
  );
  let (_, column_layout) = laid_out(column, room());
  assert!(!column_layout.children[0].result.is_dropped());
}

/// Clicks the button of a layer that lives from 240 wide, over a picture `width` wide; returns whether the layer
/// was dropped and how many clicks reached the button.
fn click_layer_button(width: f32) -> (bool, usize) {
  let clicks = Arc::new(AtomicUsize::new(0));
  let counter = clicks.clone();
  let on_click = EventHandler::new(move |_: &MouseEvent| {
    counter.fetch_add(1, Ordering::SeqCst);
  });
  let layer = Row::new()
    .overflow_visible()
    .child(Rect::new(40.0, 20.0).on_click(on_click))
    .drop_outside(SizeBounds::new().min_width(240.0));
  let stack = Stack::with(
    StackAlignment::TopStart,
    vec![Element::from(Rect::new(width, 90.0)), Element::from(layer)],
  );
  // In a row at its start, so that the stack is as wide as its picture and not as the window.
  let mut tree = Tree::new();
  tree.set_root(Row::new().align_items(Alignment::Start).child(stack));
  run_pass(&mut tree);
  let layout = tree.last_layout().expect("layout").clone();
  let stack = &layout.children[0].result;
  assert_eq!(stack.size.width, width);
  pointer_click(&mut tree, 5.0, 5.0, MouseButton::Left);
  (stack.children[1].result.is_dropped(), clicks.load(Ordering::SeqCst))
}

#[test]
fn a_dropped_child_is_not_hit() {
  assert_eq!(
    click_layer_button(300.0),
    (false, 1),
    "a layer that is laid out takes the click"
  );
  assert_eq!(click_layer_button(120.0), (true, 0), "a dropped one takes none");
}

#[test]
fn a_dropped_child_is_not_drawn() {
  let quads = |width: f32| {
    let layer = Row::new()
      .overflow_visible()
      .child(Rect::new(40.0, 20.0).background("#ff0000"))
      .drop_outside(SizeBounds::new().min_width(240.0));
    let stack = Stack::with(
      StackAlignment::TopStart,
      vec![Element::from(Rect::new(width, 90.0)), Element::from(layer)],
    );
    let (tree, layout) = laid_out(stack, room());
    tree.resolve_quads(&layout).len()
  };
  assert_eq!(quads(300.0), 1, "the layer's rect is drawn");
  assert_eq!(quads(120.0), 0, "a dropped layer draws nothing");
}
