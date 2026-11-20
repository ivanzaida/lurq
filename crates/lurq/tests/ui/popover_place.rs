//! Where a popover stands: its offset away from its anchor on each side (over it too), its offset along the edge
//! (`Popover::cross_offset`), an inset of the box it is kept in (`Popover::within_inset`), and an anchor that is no
//! trigger of a dropdown (`Popover::switch_on_anchor_press(false)`).

use std::sync::{Arc, Mutex};

use lurq::{
  components::{Column, Placement, Rect, Row},
  core::{ElementRef, Signal},
  node::{Element, padding::Padding},
  ui::{Button, Menu, MenuItem, Popover},
};

use super::stage::Stage;

/// A page with an anchor of `width` by 32 at `(x, y)`, and what `overlay` hangs from it.
fn page((x, y): (f32, f32), width: f32, overlay: impl Fn(ElementRef) -> Element + Send + Sync + 'static) -> Stage {
  let anchor = ElementRef::new();
  Stage::new(move |_| {
    Column::new()
      .size(400.0, 400.0)
      .padding_left(x)
      .padding_top(y)
      .child(
        Row::new()
          .child(Rect::new(width, 32.0).id("anchor").ref_element(anchor.clone()))
          .child(overlay(anchor.clone())),
      )
      .into()
  })
}

fn above(offset: f32) -> impl Fn(ElementRef) -> Element + Send + Sync + 'static {
  move |anchor| {
    Popover::new(Signal::new(true), anchor)
      .id("panel")
      .title("Above")
      .placement(Placement::TopStart)
      .offset(offset)
      .into()
  }
}

#[test]
fn over_its_anchor_a_popovers_offset_is_away_from_it_and_a_negative_one_toward_it() {
  // 6 over the control: its bottom 6 above the anchor's top.
  let mut stage = page((20.0, 300.0), 120.0, above(6.0));
  let (_, y, _, height) = stage.bounds("panel");
  assert!((y + height - (300.0 - 6.0)).abs() < 0.01, "6 above: {}", y + height);

  // A negative offset is toward the anchor, as under it: -6 overlaps the control by 6. Not a fault: the offset is
  // counted away from the anchor on whichever side the popover stands.
  let mut stage = page((20.0, 300.0), 120.0, above(-6.0));
  let (_, y, _, height) = stage.bounds("panel");
  assert!(
    (y + height - (300.0 + 6.0)).abs() < 0.01,
    "6 into the control: {}",
    y + height
  );

  // Turned over its anchor for want of room under it in the box it is kept in, it keeps its offset away from it.
  let (frame, anchor) = (ElementRef::new(), ElementRef::new());
  let mut stage = Stage::new(move |_| {
    Column::new()
      .size(400.0, 300.0)
      .padding(20.0)
      .child(
        Column::new()
          .size(300.0, 200.0)
          .padding_top(160.0)
          .id("frame")
          .ref_element(frame.clone())
          .child(Rect::new(120.0, 32.0).id("anchor").ref_element(anchor.clone()))
          .child(
            Popover::new(Signal::new(true), anchor.clone())
              .id("panel")
              .title("Flipped")
              .offset(6.0)
              .within(frame.clone()),
          ),
      )
      .into()
  });
  let (_, ay, ..) = stage.bounds("anchor");
  let (_, y, _, height) = stage.bounds("panel");
  assert!(
    (y + height - (ay - 6.0)).abs() < 0.01,
    "6 above: {} for {ay}",
    y + height
  );

  // A menu over its control is the same.
  let mut stage = page((20.0, 300.0), 120.0, |anchor| {
    Menu::controlled(true, anchor, |_| {})
      .id("menu")
      .placement(Placement::TopStart)
      .offset(6.0)
      .item(MenuItem::new("Open"))
      .into()
  });
  let (_, y, _, height) = stage.bounds("menu");
  assert!((y + height - (300.0 - 6.0)).abs() < 0.01, "6 above: {}", y + height);
}

#[test]
fn a_cross_offset_moves_the_popover_along_the_edge_it_stands_on() {
  // Over its anchor at its end, 12 back from the end.
  let mut stage = page((20.0, 300.0), 360.0, |anchor| {
    Popover::new(Signal::new(true), anchor)
      .id("panel")
      .title("At the end")
      .width(200.0)
      .placement(Placement::TopEnd)
      .offset(9.0)
      .cross_offset(-12.0)
      .into()
  });
  let (x, y, width, height) = stage.bounds("panel");
  assert_eq!(x + width, 20.0 + 360.0 - 12.0);
  assert!((y + height - (300.0 - 9.0)).abs() < 0.01);

  // Beside it, down from its top.
  let mut stage = page((20.0, 20.0), 60.0, |anchor| {
    Popover::new(Signal::new(true), anchor)
      .id("panel")
      .title("Beside")
      .placement(Placement::RightStart)
      .cross_offset(10.0)
      .into()
  });
  let (x, y, ..) = stage.bounds("panel");
  assert_eq!((x, y), (20.0 + 60.0 + 4.0, 20.0 + 10.0));
}

#[test]
fn a_popover_within_an_element_less_an_inset_ends_that_far_before_its_end() {
  let (frame, anchor) = (ElementRef::new(), ElementRef::new());
  let mut stage = Stage::new(move |_| {
    Column::new()
      .size(400.0, 300.0)
      .padding(20.0)
      .child(
        Row::new()
          .width(300.0)
          .height(260.0)
          .id("frame")
          .ref_element(frame.clone())
          .padding_left(180.0)
          .child(Rect::new(40.0, 30.0).id("anchor").ref_element(anchor.clone()))
          .child(
            Popover::new(Signal::new(true), anchor.clone())
              .id("panel")
              .width(200.0)
              .collision(lurq::components::CollisionStrategy::Clamp)
              .within_inset(frame.clone(), Padding::new().right(12.0))
              .title("Sources"),
          ),
      )
      .into()
  });
  let (fx, _, fw, _) = stage.bounds("frame");
  let (x, y, width, _) = stage.bounds("panel");
  let (_, ay, _, ah) = stage.bounds("anchor");
  assert!(
    (x + width - (fx + fw - 12.0)).abs() < 0.5,
    "its end 12 before the frame's: {x}"
  );
  assert_eq!(y, ay + ah + 4.0, "still under its anchor");
}

type Log = Arc<Mutex<Vec<&'static str>>>;

/// A popover opened by its trigger, and a popover the app opens at a row, whose anchor is the row.
fn trigger_and_row(row_open: &Signal<bool>, switch: bool, log: &Log) -> (Stage, Signal<bool>) {
  let trigger_open = Signal::new(false);
  let (row_open, log, opened) = (row_open.clone(), log.clone(), trigger_open.clone());
  let (trigger_anchor, row) = (ElementRef::new(), ElementRef::new());
  let stage = Stage::new(move |_| {
    let pressed = log.clone();
    Column::new()
      .size(400.0, 400.0)
      .padding(20.0)
      .spacing(200.0)
      .child(
        Popover::new(opened.clone(), trigger_anchor.clone())
          .id("first")
          .trigger(Button::new("Details").id("details"))
          .title("Details"),
      )
      .child(
        Button::new("Row")
          .id("row")
          .ref_element(row.clone())
          .on_click(move |_| pressed.lock().unwrap().push("row")),
      )
      .child(
        Popover::new(row_open.clone(), row.clone())
          .id("question")
          .placement(Placement::RightStart)
          .switch_on_anchor_press(switch)
          .title("Move it?"),
      )
      .into()
  });
  (stage, trigger_open)
}

#[test]
fn a_popover_whose_anchor_is_no_trigger_does_not_let_a_press_of_it_through_another_open_popup() {
  let (row_open, log) = (Signal::new(false), Log::default());
  let (mut stage, first) = trigger_and_row(&row_open, true, &log);
  stage.click("details");
  assert!(first.get());
  stage.click("row");
  assert!(!first.get());
  assert_eq!(*log.lock().unwrap(), ["row"], "a trigger's press goes on to it");

  let (row_open, log) = (Signal::new(false), Log::default());
  let (mut stage, first) = trigger_and_row(&row_open, false, &log);
  stage.click("details");
  assert!(first.get());
  stage.click("row");
  assert!(!first.get());
  assert!(log.lock().unwrap().is_empty(), "the press only closed the open popover");
  assert!(!row_open.get(), "and opened nothing");
}
