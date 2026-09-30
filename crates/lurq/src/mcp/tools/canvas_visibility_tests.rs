//! Canvas item visibility: clipping to the canvas, scroll viewports,
//! transformed ancestors and the window, and refusing input off the canvas.

use std::sync::{Arc, Mutex};

use super::{
  canvas_items_tests::{Seen, ref_of},
  tests::{call, json, output_text, state},
};
use crate::{
  app::{App, Tree, events::MouseEvent},
  canvas::{CanvasHandle, CanvasItem},
  components::{Canvas, Column, Rect, ScrollVertical, Spacer},
  core::ElementRef,
  layout::layout_kind::ScrollState,
  node::transform::Transform2D,
};

/// The reviewer's case: a canvas above a destructive control, with an item
/// that overhangs the canvas bottom into it.
struct Overhang {
  tree: Tree,
  app: App,
  canvas: CanvasHandle,
  clicked: Seen,
  deleted: Arc<Mutex<u32>>,
}

fn overhang() -> Overhang {
  let mut tree = Tree::new();
  let mut app = App::new();
  let reference = ElementRef::new();
  let (clicked, deleted) = (Seen::default(), Arc::new(Mutex::new(0)));
  let (canvas_ref, click_log, delete_count) = (reference.clone(), clicked.clone(), deleted.clone());
  tree.resize(400, 300);
  tree.set_root(
    Column::new()
      .child(
        Canvas::new()
          .software()
          .ref_element(reference.clone())
          .width(200.0)
          .height(100.0)
          .on_click(move |event: MouseEvent| {
            let canvas = canvas_ref.as_canvas().unwrap();
            let (x, y) = canvas.point_from_window(event.x, event.y).unwrap();
            if let Some(item) = canvas.item_at(x, y) {
              click_log.lock().unwrap().push(item.id);
            }
          }),
      )
      .child(
        Rect::new(200.0, 60.0)
          .id("delete")
          .on_click(move |_| *delete_count.lock().unwrap() += 1),
      ),
  );
  tree.pass_headless(&mut app);
  let canvas = reference.as_canvas().unwrap();
  canvas.set_items([
    CanvasItem::rect("tall", "bar", 10.0, 90.0, 20.0, 50.0),
    CanvasItem::rect("below", "bar", 10.0, 120.0, 20.0, 20.0),
  ]);
  Overhang {
    tree,
    app,
    canvas,
    clicked,
    deleted,
  }
}

#[test]
fn item_bounds_are_clipped_to_the_canvas_and_input_never_lands_beside_it() {
  let mut f = overhang();
  let state = state();
  let text = output_text(call(
    &mut f.tree,
    &mut f.app,
    &state,
    "lurq_read_tree",
    serde_json::json!({}),
  ));
  // Only the 10 px of "tall" inside the canvas are reported; "below" is off it.
  assert!(text.contains("item:tall [") && text.contains("@10,90 20x10"), "{text}");
  assert!(
    text
      .lines()
      .any(|line| line.contains("item:below [") && line.ends_with("(not visible)")),
    "{text}"
  );

  let tall = ref_of(&text, "item:tall");
  let below = ref_of(&text, "item:below");
  let interact = |f: &mut Overhang, action: &str, ref_id: &str| {
    call(
      &mut f.tree,
      &mut f.app,
      &state,
      "lurq_interact",
      serde_json::json!({"action": action, "ref": ref_id}),
    )
  };
  interact(&mut f, "click", &tall).unwrap();
  assert_eq!(*f.clicked.lock().unwrap(), ["tall"]);
  for action in ["click", "move", "scroll_to"] {
    let error = interact(&mut f, action, &below).err().unwrap();
    assert!(error.contains("canvas item ref"), "{action}: {error}");
  }
  let screenshot = call(
    &mut f.tree,
    &mut f.app,
    &state,
    "lurq_screenshot",
    serde_json::json!({"ref": below}),
  );
  assert!(screenshot.err().unwrap().contains("not visible"));
  assert_eq!(*f.deleted.lock().unwrap(), 0, "no input reached the control below");

  let inspected = json(call(
    &mut f.tree,
    &mut f.app,
    &state,
    "lurq_inspect",
    serde_json::json!({"query": "below"}),
  ));
  let below = &inspected["matches"][0];
  assert_eq!(below["bounds"], serde_json::Value::Null);
  assert_eq!(below["state"]["hidden"], true);
  assert_eq!(f.canvas.item_window_bounds("below"), None);
  assert_eq!(f.canvas.item_window_bounds("tall"), Some((10.0, 90.0, 20.0, 10.0)));
}

#[test]
fn items_scrolled_out_of_view_are_not_visible_until_scrolled_to() {
  let mut tree = Tree::new();
  let mut app = App::new();
  let reference = ElementRef::new();
  tree.resize(400, 300);
  tree.set_root(
    ScrollVertical::new(
      Canvas::new()
        .software()
        .ref_element(reference.clone())
        .width(200.0)
        .height(2000.0)
        .on_click(|_: MouseEvent| {}),
    )
    .height(200.0),
  );
  tree.pass_headless(&mut app);
  reference
    .as_canvas()
    .unwrap()
    .set_items([CanvasItem::rect("deep", "bar", 10.0, 1500.0, 20.0, 20.0)]);
  let state = state();
  let text = output_text(call(
    &mut tree,
    &mut app,
    &state,
    "lurq_read_tree",
    serde_json::json!({}),
  ));
  assert!(text.contains("(not visible)"), "{text}");
  let deep = ref_of(&text, "item:deep");
  let click = |tree: &mut Tree, app: &mut App| {
    call(
      tree,
      app,
      &state,
      "lurq_interact",
      serde_json::json!({"action": "click", "ref": deep}),
    )
  };
  assert!(click(&mut tree, &mut app).err().unwrap().contains("use scroll_to"));
  call(
    &mut tree,
    &mut app,
    &state,
    "lurq_interact",
    serde_json::json!({"action": "scroll_to", "ref": deep}),
  )
  .unwrap();
  tree.pass_headless(&mut app);
  click(&mut tree, &mut app).unwrap();
}

#[test]
fn lookups_say_when_an_item_has_nothing_visible() {
  let mut f = overhang();
  let state = state();
  let found = output_text(call(
    &mut f.tree,
    &mut f.app,
    &state,
    "lurq_find_by_id",
    serde_json::json!({"id": "below"}),
  ));
  assert!(found.ends_with("(not visible)"), "{found}");
  assert!(!found.contains("@0,0"), "{found}");
}

/// A canvas that logs which item each click lands on.
fn logged_canvas(reference: &ElementRef, log: &Seen, width: f32, height: f32) -> Canvas {
  let (canvas_ref, log) = (reference.clone(), log.clone());
  Canvas::new()
    .software()
    .ref_element(reference.clone())
    .width(width)
    .height(height)
    .on_click(move |event: MouseEvent| {
      let canvas = canvas_ref.as_canvas().unwrap();
      let item = canvas
        .point_from_window(event.x, event.y)
        .and_then(|(x, y)| canvas.item_at(x, y));
      log
        .lock()
        .unwrap()
        .push(item.map_or_else(|| "none".to_owned(), |item| item.id));
    })
}

#[test]
fn items_under_a_transformed_ancestor_above_a_scroll_viewport_stay_reachable() {
  let (mut tree, mut app) = (Tree::new(), App::new());
  let (reference, log) = (ElementRef::new(), Seen::default());
  tree.resize(600, 400);
  tree.set_root(
    Column::new().child(
      Column::new().transform(Transform2D::translate(250.0, 0.0)).child(
        ScrollVertical::new(logged_canvas(&reference, &log, 200.0, 300.0))
          .height(150.0)
          .width(200.0),
      ),
    ),
  );
  tree.pass_headless(&mut app);
  reference
    .as_canvas()
    .unwrap()
    .set_items([CanvasItem::rect("a", "bar", 10.0, 10.0, 20.0, 20.0)]);
  let state = state();
  let text = output_text(call(
    &mut tree,
    &mut app,
    &state,
    "lurq_read_tree",
    serde_json::json!({}),
  ));
  assert!(text.contains("item:a [") && text.contains("@260,10 20x20"), "{text}");
  let a = ref_of(&text, "item:a");
  call(
    &mut tree,
    &mut app,
    &state,
    "lurq_interact",
    serde_json::json!({"action": "click", "ref": a}),
  )
  .unwrap();
  assert_eq!(*log.lock().unwrap(), ["a"]);
}

#[test]
fn items_whose_clipping_ancestor_is_scrolled_away_are_not_visible() {
  let (mut tree, mut app) = (Tree::new(), App::new());
  let (reference, log) = (ElementRef::new(), Seen::default());
  let outer = ScrollState::new();
  tree.resize(400, 400);
  tree.set_root(
    ScrollVertical::new(
      Column::new()
        .child(Spacer::new().height(100.0))
        .child(ScrollVertical::new(logged_canvas(&reference, &log, 200.0, 600.0)).height(200.0))
        .child(Spacer::new().height(600.0)),
    )
    .with_scroll_state(outer.clone())
    .height(200.0),
  );
  tree.pass_headless(&mut app);
  outer.set_scroll(0.0, 400.0);
  tree.pass_headless(&mut app);
  reference
    .as_canvas()
    .unwrap()
    .set_items([CanvasItem::rect("a", "bar", 10.0, 10.0, 20.0, 20.0)]);
  let state = state();
  let text = output_text(call(
    &mut tree,
    &mut app,
    &state,
    "lurq_read_tree",
    serde_json::json!({}),
  ));
  assert!(
    text
      .lines()
      .any(|line| line.contains("item:a [") && line.ends_with("(not visible)")),
    "{text}"
  );
  let a = ref_of(&text, "item:a");
  for (tool, args) in [
    ("lurq_interact", serde_json::json!({"action": "click", "ref": a})),
    ("lurq_screenshot", serde_json::json!({"ref": a})),
  ] {
    let error = call(&mut tree, &mut app, &state, tool, args).err().unwrap();
    assert!(error.contains("not visible"), "{tool}: {error}");
  }
  assert!(log.lock().unwrap().is_empty());
  let inspected = json(call(
    &mut tree,
    &mut app,
    &state,
    "lurq_inspect",
    serde_json::json!({"role": "bar"}),
  ));
  assert_eq!(inspected["matches"][0]["state"]["hidden"], true);
}

#[test]
fn scroll_to_an_item_in_nested_scroll_containers_makes_it_clickable() {
  let (mut tree, mut app) = (Tree::new(), App::new());
  let (reference, log) = (ElementRef::new(), Seen::default());
  tree.resize(400, 400);
  tree.set_root(
    ScrollVertical::new(
      Column::new()
        .child(Spacer::new().height(100.0))
        .child(ScrollVertical::new(logged_canvas(&reference, &log, 200.0, 600.0)).height(200.0))
        .child(Spacer::new().height(600.0)),
    )
    .height(200.0),
  );
  tree.pass_headless(&mut app);
  reference
    .as_canvas()
    .unwrap()
    .set_items([CanvasItem::rect("deep", "bar", 10.0, 500.0, 20.0, 20.0)]);
  let state = state();
  let text = output_text(call(
    &mut tree,
    &mut app,
    &state,
    "lurq_read_tree",
    serde_json::json!({}),
  ));
  let deep = ref_of(&text, "item:deep");
  let interact = |tree: &mut Tree, app: &mut App, action: &str| {
    call(
      tree,
      app,
      &state,
      "lurq_interact",
      serde_json::json!({"action": action, "ref": deep}),
    )
  };
  interact(&mut tree, &mut app, "scroll_to").unwrap();
  tree.pass_headless(&mut app);
  let (_, y, _, height) = reference.as_canvas().unwrap().item_window_bounds("deep").unwrap();
  assert!(y >= 0.0 && y + height <= 200.0, "item at {y}");
  interact(&mut tree, &mut app, "click").unwrap();
  assert_eq!(*log.lock().unwrap(), ["deep"]);
}
