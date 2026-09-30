use std::sync::{Arc, Mutex};

use super::{
  format_ref_line,
  tests::{call, json, output_text, state},
};
use crate::{
  app::{App, Tree, events::MouseEvent},
  canvas::{CanvasHandle, CanvasItem},
  components::{Canvas, Column, ScrollVertical},
  core::ElementRef,
  layout::layout_kind::ScrollState,
};

type Seen = Arc<Mutex<Vec<String>>>;

/// A bar chart: a padded canvas whose handlers hit-test the registered items,
/// the way an app opens a tooltip on hover.
fn chart(reference: &ElementRef, hovered: &Seen, clicked: &Seen) -> Column {
  let (hover_ref, hover_log) = (reference.clone(), hovered.clone());
  let (click_ref, click_log) = (reference.clone(), clicked.clone());
  let item_under = |reference: &ElementRef, event: &MouseEvent| {
    let canvas = reference.as_canvas()?;
    let (x, y) = canvas.point_from_window(event.x, event.y)?;
    canvas.item_at(x, y).map(|item| item.id)
  };
  Column::new().padding(10.0).child(
    Canvas::new()
      .software()
      .ref_element(reference.clone())
      .id("chart")
      .width(200.0)
      .height(100.0)
      .padding(5.0)
      .on_mouse_move(move |event: MouseEvent| {
        if let Some(id) = item_under(&hover_ref, &event) {
          hover_log.lock().unwrap().push(id);
        }
      })
      .on_click(move |event: MouseEvent| {
        if let Some(id) = item_under(&click_ref, &event) {
          click_log.lock().unwrap().push(id);
        }
      }),
  )
}

fn bars(canvas: &CanvasHandle) {
  canvas.set_items([
    CanvasItem::rect("mon", "bar", 10.0, 40.0, 20.0, 50.0)
      .label("Mon")
      .value("12 runs"),
    CanvasItem::rect("tue", "bar", 40.0, 20.0, 20.0, 70.0)
      .label("Tue")
      .value("18 runs"),
  ]);
}

struct Fixture {
  tree: Tree,
  app: App,
  canvas: CanvasHandle,
  hovered: Seen,
  clicked: Seen,
}

fn fixture() -> Fixture {
  let mut tree = Tree::new();
  let mut app = App::new();
  let reference = ElementRef::new();
  let (hovered, clicked) = (Seen::default(), Seen::default());
  tree.set_scale_factor(2.0);
  tree.resize(800, 600);
  tree.set_root(chart(&reference, &hovered, &clicked));
  tree.pass_headless(&mut app);
  let canvas = reference.as_canvas().expect("canvas is bound after layout");
  bars(&canvas);
  Fixture {
    tree,
    app,
    canvas,
    hovered,
    clicked,
  }
}

fn ref_of(tree_text: &str, marker: &str) -> String {
  let line = tree_text
    .lines()
    .find(|line| line.contains(marker))
    .unwrap_or_else(|| panic!("{marker} not in:\n{tree_text}"));
  // Tree lines carry `[ref_N]`; lookup lines start with `ref_N`.
  let start = line.find("ref_").expect("line has a ref");
  let end = start + line[start..].find([']', ' ']).expect("ref is delimited");
  line[start..end].to_owned()
}

#[test]
fn read_tree_lists_items_under_their_canvas_in_screenshot_pixels() {
  let mut f = fixture();
  let state = state();
  let text = output_text(call(
    &mut f.tree,
    &mut f.app,
    &state,
    "lurq_read_tree",
    serde_json::json!({}),
  ));
  // Canvas content origin: column padding 10 + canvas padding 5 = 15 logical,
  // 30 screenshot pixels at 2x. "mon" is at (10, 40) 20x50 in the canvas.
  assert!(text.contains("- Canvas #chart ["), "{text}");
  assert!(text.contains("{items=2}"), "{text}");
  let mon_line = text.lines().find(|line| line.contains("#mon")).unwrap();
  assert!(mon_line.starts_with("    - bar #mon [ref_"), "{text}");
  assert!(
    mon_line.ends_with("\"Mon\" @50,110 40x100 {value=12 runs}"),
    "{mon_line}"
  );

  let refs = state.shared.refs.lock().unwrap();
  let record = refs
    .records
    .iter()
    .find(|record| record.canvas_item.as_deref() == Some("tue"))
    .unwrap();
  let line = format_ref_line(record);
  assert!(
    line.contains("[main] bar role=bar #tue name=\"Tue\" {value=18 runs} @110,70 40x140 (canvas item)"),
    "{line}"
  );
}

#[test]
fn move_and_click_by_item_ref_drive_the_canvas_handlers_at_the_item() {
  let mut f = fixture();
  let state = state();
  let text = output_text(call(
    &mut f.tree,
    &mut f.app,
    &state,
    "lurq_read_tree",
    serde_json::json!({}),
  ));
  let tue = ref_of(&text, "#tue");
  let reply = json(call(
    &mut f.tree,
    &mut f.app,
    &state,
    "lurq_interact",
    serde_json::json!({"action": "move", "ref": tue}),
  ));
  assert_eq!((reply["x"].as_f64(), reply["y"].as_f64()), (Some(130.0), Some(140.0)));
  assert_eq!(f.hovered.lock().unwrap().last().map(String::as_str), Some("tue"));

  let mon = ref_of(&text, "#mon");
  call(
    &mut f.tree,
    &mut f.app,
    &state,
    "lurq_interact",
    serde_json::json!({"action": "click", "ref": mon}),
  )
  .unwrap();
  assert_eq!(*f.clicked.lock().unwrap(), ["mon"]);
}

#[test]
fn item_refs_track_redraws_and_find_by_id_reaches_items() {
  let mut f = fixture();
  let state = state();
  let found = output_text(call(
    &mut f.tree,
    &mut f.app,
    &state,
    "lurq_find_by_id",
    serde_json::json!({"id": "tue"}),
  ));
  assert!(found.contains("bar role=bar #tue name=\"Tue\""), "{found}");
  let tue = ref_of(&found, "#tue");

  // The next draw moves "tue" and drops "mon": the ref follows the live item.
  f.canvas
    .set_items([CanvasItem::rect("tue", "bar", 100.0, 20.0, 20.0, 70.0)]);
  let reply = json(call(
    &mut f.tree,
    &mut f.app,
    &state,
    "lurq_interact",
    serde_json::json!({"action": "move", "ref": tue}),
  ));
  assert_eq!(reply["x"].as_f64(), Some(250.0));
  let missing = output_text(call(
    &mut f.tree,
    &mut f.app,
    &state,
    "lurq_find_by_id",
    serde_json::json!({"id": "mon"}),
  ));
  assert!(missing.starts_with("no element or canvas item"), "{missing}");

  f.canvas.set_items([]);
  let stale = call(
    &mut f.tree,
    &mut f.app,
    &state,
    "lurq_interact",
    serde_json::json!({"action": "click", "ref": tue}),
  );
  assert!(stale.err().unwrap().contains("no longer resolves"));
}

#[test]
fn scroll_to_an_item_ref_brings_the_item_into_the_viewport() {
  let mut tree = Tree::new();
  let mut app = App::new();
  let reference = ElementRef::new();
  let scroll = ScrollState::new();
  tree.resize(400, 300);
  tree.set_root(
    ScrollVertical::new(
      Canvas::new()
        .software()
        .ref_element(reference.clone())
        .width(200.0)
        .height(2000.0),
    )
    .with_scroll_state(scroll.clone())
    .height(200.0),
  );
  tree.pass_headless(&mut app);
  let canvas = reference.as_canvas().unwrap();
  canvas.set_items([CanvasItem::point("deep", "point", 50.0, 1500.0, 5.0)]);
  let state = state();
  let text = output_text(call(
    &mut tree,
    &mut app,
    &state,
    "lurq_read_tree",
    serde_json::json!({}),
  ));
  let deep = ref_of(&text, "#deep");
  call(
    &mut tree,
    &mut app,
    &state,
    "lurq_interact",
    serde_json::json!({"action": "scroll_to", "ref": deep}),
  )
  .unwrap();
  tree.pass_headless(&mut app);
  let (_, y, _, height) = canvas.item_window_bounds("deep").unwrap();
  assert!(scroll.scroll_y() > 1000.0, "{}", scroll.scroll_y());
  assert!(y >= 0.0 && y + height <= 200.0, "item at {y}");
}
