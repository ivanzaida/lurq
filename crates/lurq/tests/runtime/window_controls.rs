//! The window's controls of a title bar the app draws itself (`.window_control(..)`): what the tree answers a
//! shell that asks which control is at a point of the window, as the system asks a window on Windows.

use std::sync::{
  Arc,
  atomic::{AtomicU32, Ordering},
};

use lurq::{
  app::{
    Tree,
    events::{MouseButton, ScrollPhase},
  },
  components::{Column, Rect, Row, ScrollVertical, Stack},
  core::ElementRef,
  node::{Element, HitTestBehavior, WindowControl},
};

use crate::support::run_pass;

const BAR_HEIGHT: f32 = 36.0;
const CONTROL_WIDTH: f32 = 46.0;

/// A title bar: room of `room` wide that drags the window, then the three controls, each `CONTROL_WIDTH` wide.
fn title_bar(room: f32) -> Row {
  let control = |control: WindowControl| Rect::new(CONTROL_WIDTH, BAR_HEIGHT).window_control(control);
  Row::new()
    .height(BAR_HEIGHT)
    .child(Rect::new(room, BAR_HEIGHT))
    .child(control(WindowControl::Minimize))
    .child(control(WindowControl::Maximize))
    .child(control(WindowControl::Close))
}

fn tree_of(root: impl Into<Element>) -> Tree {
  let mut tree = Tree::new();
  tree.resize(400, 300);
  tree.set_root(root);
  run_pass(&mut tree);
  tree
}

#[test]
fn a_point_on_a_marked_element_is_its_control() {
  let tree = tree_of(title_bar(200.0));
  let y = BAR_HEIGHT / 2.0;
  assert_eq!(tree.window_control_at(100.0, y), None, "the bar's own room");
  assert_eq!(tree.window_control_at(200.0, y), Some(WindowControl::Minimize));
  assert_eq!(tree.window_control_at(245.9, y), Some(WindowControl::Minimize));
  assert_eq!(tree.window_control_at(246.0, y), Some(WindowControl::Maximize));
  assert_eq!(tree.window_control_at(292.0, y), Some(WindowControl::Close));
  assert_eq!(tree.window_control_at(337.9, y), Some(WindowControl::Close));
  // Past the last control, and under the bar.
  assert_eq!(tree.window_control_at(338.0, y), None);
  assert_eq!(tree.window_control_at(250.0, BAR_HEIGHT), None);
  assert_eq!(tree.window_control_at(250.0, -1.0), None);
}

#[test]
fn nothing_is_a_control_where_no_element_is_marked() {
  let tree = tree_of(
    Row::new()
      .height(BAR_HEIGHT)
      .child(Rect::new(200.0, BAR_HEIGHT))
      .child(Rect::new(CONTROL_WIDTH, BAR_HEIGHT)),
  );
  for x in [0.0, 100.0, 210.0, 399.0] {
    assert_eq!(tree.window_control_at(x, BAR_HEIGHT / 2.0), None, "{x}");
  }
  // A tree with nothing laid out yet answers nothing either.
  assert_eq!(Tree::new().window_control_at(10.0, 10.0), None);
}

#[test]
fn the_answer_follows_a_layout_change() {
  let mut tree = tree_of(title_bar(200.0));
  let y = BAR_HEIGHT / 2.0;
  assert_eq!(tree.window_control_at(250.0, y), Some(WindowControl::Maximize));
  // The bar's room grows: the controls move along.
  tree.set_root(title_bar(300.0));
  run_pass(&mut tree);
  assert_eq!(tree.window_control_at(250.0, y), None);
  assert_eq!(tree.window_control_at(300.0, y), Some(WindowControl::Minimize));
  assert_eq!(tree.window_control_at(350.0, y), Some(WindowControl::Maximize));
}

#[test]
fn the_answer_is_in_physical_pixels_at_any_scale() {
  let mut tree = Tree::new();
  tree.set_scale_factor(1.5);
  tree.resize(600, 450);
  tree.set_root(title_bar(200.0));
  run_pass(&mut tree);
  // The maximize control is at 246..292 logical: 369..438 physical.
  assert_eq!(tree.window_control_at(368.0, 20.0), Some(WindowControl::Minimize));
  assert_eq!(tree.window_control_at(370.0, 20.0), Some(WindowControl::Maximize));
  assert_eq!(tree.window_control_at(437.0, 20.0), Some(WindowControl::Maximize));
  assert_eq!(tree.window_control_at(439.0, 20.0), Some(WindowControl::Close));
  // The bar is 36 logical high: 54 physical.
  assert_eq!(tree.window_control_at(370.0, 53.0), Some(WindowControl::Maximize));
  assert_eq!(tree.window_control_at(370.0, 55.0), None);
}

#[test]
fn a_control_scrolls_with_what_holds_it_and_is_cut_by_it() {
  // A list 100 high that holds 60 of room, then a control 40 high, then more room.
  let content = Column::new()
    .child(Rect::new(200.0, 60.0))
    .child(Rect::new(200.0, 40.0).window_control(WindowControl::Maximize))
    .child(Rect::new(200.0, 300.0));
  let mut tree = tree_of(
    Column::new()
      .child(Rect::new(200.0, 50.0))
      .child(ScrollVertical::new(content).size(200.0, 100.0)),
  );
  // The list starts at 50: the control is at 110..150.
  assert_eq!(tree.window_control_at(10.0, 109.0), None);
  assert_eq!(tree.window_control_at(10.0, 111.0), Some(WindowControl::Maximize));
  assert_eq!(tree.window_control_at(10.0, 149.0), Some(WindowControl::Maximize));
  // Scrolled up by 80, it is at 30..70, and seen from the list's top edge at 50 only.
  tree.scroll(10.0, 100.0, 0.0, -80.0, ScrollPhase::Scroll);
  run_pass(&mut tree);
  assert_eq!(tree.window_control_at(10.0, 111.0), None);
  assert_eq!(tree.window_control_at(10.0, 60.0), Some(WindowControl::Maximize));
  assert_eq!(tree.window_control_at(10.0, 40.0), None, "cut by the list's top edge");
}

#[test]
fn what_covers_a_control_takes_its_place() {
  // A zone 3 high along the top edge, over the bar, as the zones that resize a window are.
  let mut tree = tree_of(
    Stack::new()
      .width(400.0)
      .height(300.0)
      .child(title_bar(200.0))
      .child(Rect::new(400.0, 3.0)),
  );
  assert_eq!(tree.window_control_at(250.0, 2.0), None, "under the zone");
  assert_eq!(tree.window_control_at(250.0, 3.0), Some(WindowControl::Maximize));
  // A layer over the whole window that takes the pointer: no control is left.
  tree.set_root(
    Stack::new()
      .width(400.0)
      .height(300.0)
      .child(title_bar(200.0))
      .child(Rect::new(400.0, 300.0)),
  );
  run_pass(&mut tree);
  assert_eq!(tree.window_control_at(250.0, 20.0), None);
  // A layer the pointer goes through covers nothing.
  tree.set_root(
    Stack::new()
      .width(400.0)
      .height(300.0)
      .child(title_bar(200.0))
      .child(Rect::new(400.0, 300.0).hit_test(HitTestBehavior::None)),
  );
  run_pass(&mut tree);
  assert_eq!(tree.window_control_at(250.0, 20.0), Some(WindowControl::Maximize));
}

#[test]
fn an_element_that_takes_no_pointer_is_no_control() {
  let control = || Rect::new(CONTROL_WIDTH, BAR_HEIGHT).window_control(WindowControl::Close);
  let tree = tree_of(
    Row::new()
      .height(BAR_HEIGHT)
      .child(control().disabled(true))
      .child(control().hit_test(HitTestBehavior::None))
      .child(Row::new().disabled(true).child(control()))
      .child(control()),
  );
  let y = BAR_HEIGHT / 2.0;
  assert_eq!(tree.window_control_at(10.0, y), None, "disabled");
  assert_eq!(tree.window_control_at(56.0, y), None, "takes no pointer");
  assert_eq!(tree.window_control_at(102.0, y), None, "in a disabled element");
  assert_eq!(tree.window_control_at(148.0, y), Some(WindowControl::Close));
}

/// The control is an element like any other to the pointer: under it, held down, clicked by a press and a release
/// on it and not by a press let go elsewhere. The shell gives the tree the pointer over a control as it gives it
/// anywhere else.
#[test]
fn a_control_is_hovered_pressed_and_clicked_as_any_element() {
  let clicks = Arc::new(AtomicU32::new(0));
  let maximize = ElementRef::new();
  let bar = {
    let clicks = clicks.clone();
    Row::new().height(BAR_HEIGHT).child(Rect::new(200.0, BAR_HEIGHT)).child(
      Rect::new(CONTROL_WIDTH, BAR_HEIGHT)
        .window_control(WindowControl::Maximize)
        .ref_element(maximize.clone())
        .on_click(move |_| {
          clicks.fetch_add(1, Ordering::Relaxed);
        }),
    )
  };
  let mut tree = tree_of(bar);
  let (x, y) = (220.0, BAR_HEIGHT / 2.0);
  assert_eq!(tree.window_control_at(x, y), Some(WindowControl::Maximize));

  tree.mouse_move(x, y);
  assert!(maximize.hovered());
  tree.mouse_down(x, y, MouseButton::Left);
  assert!(maximize.active());
  tree.mouse_up(x, y, MouseButton::Left);
  assert!(!maximize.active());
  assert_eq!(clicks.load(Ordering::Relaxed), 1);

  // A press dragged off the control and let go does not click it.
  tree.mouse_down(x, y, MouseButton::Left);
  tree.mouse_move(100.0, y);
  tree.mouse_up(100.0, y, MouseButton::Left);
  assert_eq!(clicks.load(Ordering::Relaxed), 1);

  // The pointer leaves the window from the control: it is no longer under it.
  tree.mouse_move(x, y);
  assert!(maximize.hovered());
  tree.mouse_leave_window();
  assert!(!maximize.hovered());
}
