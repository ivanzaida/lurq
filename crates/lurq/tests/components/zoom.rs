//! `ZoomScroll`: two fingers pinch its content larger about their middle, and a double tap goes in and out.

use std::time::Duration;

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::Ctx,
    events::{GesturePhase, ScrollPhase, TouchPhase},
  },
  components::{Spacer, ZoomScroll, ZoomScrollProps},
  core::Signal,
  layout::layout_kind::ScrollState,
  node::Element,
};

use crate::support::run_pass;

#[derive(Clone)]
struct Setup {
  zoom: Signal<f32>,
  scroll: ScrollState,
  range: (f32, f32),
}

impl PartialEq for Setup {
  fn eq(&self, other: &Self) -> bool {
    self.zoom.id() == other.zoom.id()
  }
}

impl lurq::app::component::DevtoolsInspectable for Setup {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

struct Page;

impl Component for Page {
  type Props = Setup;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let setup = ctx.props::<Setup>().clone();
    ZoomScroll::mount(
      ctx,
      ZoomScrollProps::new(400.0, 400.0)
        .range(setup.range.0, setup.range.1)
        .zoom(setup.zoom)
        .scroll(setup.scroll),
      Spacer::new().width(400.0).height(400.0),
    )
  }
}

/// Content 400 by 400 in a window 200 by 200, pinched between the scales of `range`.
fn stage(range: (f32, f32)) -> (Tree, Signal<f32>, ScrollState) {
  let (zoom, scroll) = (Signal::new(1.0), ScrollState::new());
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(200, 200);
  tree.mount_root::<Page>(
    &mut app,
    Setup {
      zoom: zoom.clone(),
      scroll: scroll.clone(),
      range,
    },
  );
  run_pass(&mut tree);
  run_pass(&mut tree);
  (tree, zoom, scroll)
}

/// Two fingers land `from` apart about a point and move to `to` apart about it, in steps.
fn pinch(tree: &mut Tree, about: (f32, f32), from: f32, to: f32) {
  let at = |apart: f32| ((about.0 - apart / 2.0, about.1), (about.0 + apart / 2.0, about.1));
  let (first, second) = at(from);
  tree.touch_event(1, TouchPhase::Start, first.0, first.1, None);
  tree.touch_event(2, TouchPhase::Start, second.0, second.1, None);
  for step in 1..=20 {
    let (first, second) = at(from + (to - from) * step as f32 / 20.0);
    tree.touch_event(1, TouchPhase::Move, first.0, first.1, None);
    tree.touch_event(2, TouchPhase::Move, second.0, second.1, None);
    run_pass(tree);
  }
  let (first, second) = at(to);
  tree.touch_event(1, TouchPhase::End, first.0, first.1, None);
  tree.touch_event(2, TouchPhase::End, second.0, second.1, None);
  run_pass(tree);
}

#[test]
fn two_fingers_moving_apart_make_the_content_larger_about_their_middle() {
  let (mut tree, zoom, scroll) = stage((1.0, 4.0));
  assert_eq!((scroll.content_width(), scroll.content_height()), (400.0, 400.0));

  pinch(&mut tree, (100.0, 100.0), 40.0, 100.0);

  let now = zoom.get_untracked();
  assert!(
    now > 1.5 && now < 2.6,
    "larger, by about how far the fingers went apart: {now}"
  );
  assert!(
    (scroll.content_width() - 400.0 * now).abs() < 0.5,
    "the list scrolls all of it"
  );
  // The point of the content the fingers took, 100 in from its corner, is still under their middle.
  let under = ((scroll.scroll_x() + 100.0) / now, (scroll.scroll_y() + 100.0) / now);
  assert!(
    (under.0 - 100.0).abs() < 1.0 && (under.1 - 100.0).abs() < 1.0,
    "{under:?}"
  );
}

#[test]
fn a_pinch_stops_at_the_largest_and_the_smallest_scale() {
  let (mut tree, zoom, _scroll) = stage((1.0, 1.5));

  pinch(&mut tree, (100.0, 100.0), 30.0, 180.0);
  assert_eq!(zoom.get_untracked(), 1.5);

  pinch(&mut tree, (100.0, 100.0), 180.0, 30.0);
  assert_eq!(zoom.get_untracked(), 1.0);
}

#[test]
fn a_double_tap_goes_in_at_the_point_tapped_and_back_out() {
  let (mut tree, zoom, scroll) = stage((1.0, 4.0));
  // A tap is a press of the pointer: its two taps are given at one time on the tree's clock of presses, a double
  // tap however slow the machine.
  let start = std::time::Instant::now();
  tree.set_press_clock(Some(start));

  // A finger's double tap: two taps of one finger, which the tree tells as `on_double_tap` (not a double click).
  let finger_tap = |tree: &mut Tree| {
    tree.touch_event(1, TouchPhase::Start, 50.0, 50.0, None);
    tree.touch_event(1, TouchPhase::End, 50.0, 50.0, None);
  };
  finger_tap(&mut tree);
  finger_tap(&mut tree);
  run_pass(&mut tree);
  assert_eq!(zoom.get_untracked(), 2.0);
  // What was tapped, 50 in from the corner, is drawn 100 in, and is still under the finger.
  assert_eq!((scroll.scroll_x(), scroll.scroll_y()), (50.0, 50.0));

  // Another double tap, a while after the first.
  tree.set_press_clock(Some(start + Duration::from_millis(600)));
  finger_tap(&mut tree);
  finger_tap(&mut tree);
  run_pass(&mut tree);
  assert_eq!(zoom.get_untracked(), 1.0);
  assert_eq!((scroll.scroll_x(), scroll.scroll_y()), (0.0, 0.0));
}

#[test]
fn a_mouse_double_click_goes_in_too() {
  let (mut tree, zoom, _scroll) = stage((1.0, 4.0));
  tree.set_press_clock(Some(std::time::Instant::now()));
  for _ in 0..2 {
    tree.mouse_move(50.0, 50.0);
    tree.mouse_down(50.0, 50.0, lurq::app::events::MouseButton::Left);
    tree.mouse_up(50.0, 50.0, lurq::app::events::MouseButton::Left);
  }
  run_pass(&mut tree);
  assert_eq!(zoom.get_untracked(), 2.0);
}

#[test]
fn content_shown_smaller_than_the_list_scrolls_nowhere() {
  let (mut tree, zoom, scroll) = stage((0.25, 4.0));

  zoom.set(0.25);
  run_pass(&mut tree);
  run_pass(&mut tree);

  // 100 by 100 in a list of 200 by 200: what scrolls is as large as the list, with the content in its middle.
  assert_eq!((scroll.content_width(), scroll.content_height()), (200.0, 200.0));
  assert_eq!((scroll.scroll_x(), scroll.scroll_y()), (0.0, 0.0));
}

/// As [`Page`], for a mouse: the wheel brings the content closer, and a drag carries it.
struct MousePage;

impl Component for MousePage {
  type Props = Setup;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let setup = ctx.props::<Setup>().clone();
    ZoomScroll::mount(
      ctx,
      ZoomScrollProps::new(400.0, 400.0)
        .range(setup.range.0, setup.range.1)
        .wheel_zoom(2.0)
        .drag_pan(true)
        .zoom(setup.zoom)
        .scroll(setup.scroll),
      Spacer::new().width(400.0).height(400.0),
    )
  }
}

/// Content 400 by 400 in a window 200 by 200, for a mouse, shown at `at` to start with.
fn mouse_stage(range: (f32, f32), at: f32) -> (Tree, Signal<f32>, ScrollState) {
  let (zoom, scroll) = (Signal::new(at), ScrollState::new());
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(200, 200);
  tree.mount_root::<MousePage>(
    &mut app,
    Setup {
      zoom: zoom.clone(),
      scroll: scroll.clone(),
      range,
    },
  );
  run_pass(&mut tree);
  run_pass(&mut tree);
  (tree, zoom, scroll)
}

/// A mouse's notch of the wheel at a point: a positive `delta_y` is a notch away from the hand, which scrolls a
/// list up (see the runtime's tests of the wheel).
fn wheel(tree: &mut Tree, at: (f32, f32), delta_y: f32) {
  tree.wheel(at.0, at.1, 0.0, delta_y, ScrollPhase::Scroll);
  run_pass(tree);
  run_pass(tree);
}

/// The same with Ctrl held.
fn ctrl_wheel(tree: &mut Tree, at: (f32, f32), delta_y: f32) {
  tree.wheel_with_meta(at.0, at.1, 0.0, delta_y, ScrollPhase::Scroll, false, true, false, false);
  run_pass(tree);
  run_pass(tree);
}

/// A touchpad's two fingers at a point, `delta_y` as far as they went, with Ctrl held where `ctrl`: that is a
/// touchpad's pinch on Windows.
fn touchpad(tree: &mut Tree, at: (f32, f32), delta_y: f32, ctrl: bool) {
  tree.scroll_with_meta(at.0, at.1, 0.0, delta_y, ScrollPhase::Scroll, false, ctrl, false, false);
  run_pass(tree);
  run_pass(tree);
}

#[test]
fn the_wheel_brings_the_content_closer_about_the_pointer() {
  let (mut tree, zoom, scroll) = mouse_stage((1.0, 4.0), 1.0);
  wheel(&mut tree, (50.0, 50.0), 120.0);
  assert_eq!(zoom.get(), 2.0, "a notch away from the hand is a step closer");
  assert_eq!(
    (scroll.scroll_x(), scroll.scroll_y()),
    (50.0, 50.0),
    "the point under the pointer is under it still"
  );
  wheel(&mut tree, (50.0, 50.0), -120.0);
  assert_eq!(
    zoom.get(),
    1.0,
    "a notch toward the hand, as a list scrolls down, is a step further"
  );
  assert_eq!((scroll.scroll_x(), scroll.scroll_y()), (0.0, 0.0));
}

#[test]
fn the_wheel_is_the_contents_where_the_list_has_nowhere_to_scroll() {
  // At half its size the content is as large as the window: a list that cannot move hears of no scroll.
  let (mut tree, zoom, _) = mouse_stage((0.5, 4.0), 0.5);
  wheel(&mut tree, (100.0, 100.0), 120.0);
  assert_eq!(zoom.get(), 1.0);
}

#[test]
fn without_it_the_wheel_scrolls_the_content() {
  let (mut tree, zoom, scroll) = stage((1.0, 4.0));
  wheel(&mut tree, (50.0, 50.0), -60.0);
  assert_eq!(zoom.get(), 1.0);
  assert!(scroll.scroll_target().1 > 0.0, "down, as in any list");
}

#[test]
fn ctrl_and_the_wheel_zoom_without_wheel_zoom_too() {
  let (mut tree, zoom, scroll) = stage((1.0, 4.0));
  ctrl_wheel(&mut tree, (50.0, 50.0), 40.0);
  assert_eq!(zoom.get(), 1.5, "a notch away from the hand, a step of 1.5 closer");
  assert_eq!(
    (scroll.scroll_x(), scroll.scroll_y()),
    (25.0, 25.0),
    "about the pointer: the point 50 in is drawn 75 in, under it still"
  );
  ctrl_wheel(&mut tree, (50.0, 50.0), -40.0);
  assert_eq!(zoom.get(), 1.0);
}

#[test]
fn a_synthetic_wheel_with_ctrl_zooms_as_the_mcp_servers_does() {
  use lurq::app::{SyntheticInput, SyntheticModifiers, synthetic_input};

  let (mut tree, zoom, _) = stage((1.0, 4.0));
  let notch = SyntheticInput::wheel(50.0, 50.0, 0.0, 40.0).with_modifiers(SyntheticModifiers::default().ctrl());
  synthetic_input::apply(&mut tree, &notch);
  run_pass(&mut tree);
  assert_eq!(zoom.get(), 1.5);
}

#[test]
fn a_touchpads_two_fingers_scroll_the_content_where_the_wheel_zooms() {
  let (mut tree, zoom, scroll) = mouse_stage((1.0, 4.0), 1.0);
  for _ in 0..6 {
    touchpad(&mut tree, (100.0, 100.0), -10.0, false);
  }
  assert_eq!(zoom.get(), 1.0, "two fingers are no wheel's notches");
  assert_eq!(scroll.scroll_y(), 60.0, "they move the content as far as they went");
}

#[test]
fn a_touchpads_pinch_on_windows_zooms_as_far_as_the_fingers_went() {
  let (mut tree, zoom, scroll) = mouse_stage((1.0, 4.0), 1.0);
  // Moving apart, Ctrl held: as far as a notch scrolls is a step (of 2 here), in parts as the fingers go.
  for _ in 0..4 {
    touchpad(&mut tree, (50.0, 50.0), 10.0, true);
  }
  assert!((zoom.get() - 2.0).abs() < 1e-4, "{}", zoom.get());
  assert!(
    (scroll.scroll_x() - 50.0).abs() < 0.01 && (scroll.scroll_y() - 50.0).abs() < 0.01,
    "about the pointer: {:?}",
    (scroll.scroll_x(), scroll.scroll_y())
  );
  // Moving together, half as far.
  touchpad(&mut tree, (50.0, 50.0), -20.0, true);
  assert!((zoom.get() - 2f32.sqrt()).abs() < 1e-4, "{}", zoom.get());
}

#[test]
fn a_trackpads_pinch_on_a_mac_zooms_about_the_pointer() {
  let (mut tree, zoom, scroll) = stage((1.0, 4.0));
  // The shell gives AppKit's magnification as a pinch's delta: 1 + magnification.
  tree.trackpad_pinch(50.0, 50.0, GesturePhase::Start, 1.0);
  tree.trackpad_pinch(50.0, 50.0, GesturePhase::Change, 1.5);
  run_pass(&mut tree);
  tree.trackpad_pinch(50.0, 50.0, GesturePhase::Change, 4.0 / 3.0);
  run_pass(&mut tree);
  tree.trackpad_pinch(50.0, 50.0, GesturePhase::End, 1.0);
  run_pass(&mut tree);
  assert!((zoom.get() - 2.0).abs() < 1e-4, "{}", zoom.get());
  assert!(
    (scroll.scroll_x() - 50.0).abs() < 0.01 && (scroll.scroll_y() - 50.0).abs() < 0.01,
    "the point under the pointer is under it still: {:?}",
    (scroll.scroll_x(), scroll.scroll_y())
  );
}

#[test]
fn a_trackpads_smart_zoom_goes_in_at_the_pointer_and_back_out() {
  let (mut tree, zoom, scroll) = stage((1.0, 4.0));
  assert!(tree.trackpad_double_tap(50.0, 50.0));
  run_pass(&mut tree);
  assert_eq!(zoom.get(), 2.0);
  assert_eq!((scroll.scroll_x(), scroll.scroll_y()), (50.0, 50.0));
  tree.trackpad_double_tap(50.0, 50.0);
  run_pass(&mut tree);
  assert_eq!(zoom.get(), 1.0);
}

#[test]
fn a_scale_set_from_outside_keeps_the_middle_of_the_view() {
  let (mut tree, zoom, scroll) = mouse_stage((1.0, 4.0), 1.0);
  // The middle of the window is over the content's point (100, 100).
  zoom.set(2.0);
  run_pass(&mut tree);
  run_pass(&mut tree);
  assert_eq!(
    (scroll.scroll_x(), scroll.scroll_y()),
    (100.0, 100.0),
    "and is over it still"
  );
}

#[test]
fn a_press_held_and_moved_carries_the_content() {
  use lurq::app::events::MouseButton;

  let (mut tree, _, scroll) = mouse_stage((1.0, 4.0), 1.0);
  tree.mouse_move(120.0, 120.0);
  tree.mouse_down(120.0, 120.0, MouseButton::Left);
  tree.mouse_move(100.0, 90.0);
  tree.mouse_move(80.0, 70.0);
  tree.mouse_up(80.0, 70.0, MouseButton::Left);
  run_pass(&mut tree);
  assert_eq!(
    (scroll.scroll_x(), scroll.scroll_y()),
    (40.0, 50.0),
    "by as far as the pointer went, the other way"
  );
}
