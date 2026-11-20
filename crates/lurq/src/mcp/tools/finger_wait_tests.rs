//! What a finger leaves moving is not an animation the tree counts: the shell steps it between frames. A
//! `lurq_wait` after a swipe answers once it is over, and the list says meanwhile that it moves and where to.

use std::time::{Duration, Instant};

use serde_json::json;

use super::{
  tests::{TestSurface, call, state},
  *,
};
use crate::{
  app::{capture_test_engine::GlyphPainter, events::TouchPhase},
  components::{Column, Rect, ScrollVertical},
  layout::layout_kind::ScrollState,
  mcp::shared::McpToolOutput,
};

/// What `lurq_find_by_id` says of the feed.
fn feed(tree: &mut Tree, app: &mut App, state: &McpState) -> String {
  match call(tree, app, state, "lurq_find_by_id", json!({"id": "feed"})).expect("the feed is found") {
    McpToolOutput::Text(text) => text,
    _ => panic!("the lookup answers in text"),
  }
}

#[test]
fn a_wait_after_a_flick_answers_when_the_list_is_on_its_page() {
  let scroll = ScrollState::new();
  scroll.set_paging(true);
  let mut tree = Tree::new();
  let mut app = App::new();
  let state = state();
  let (factory, _screen) = GlyphPainter::factory();
  tree.set_render_engine_factory(factory);
  tree.resize(200, 200);
  // Three pages, each as large as the feed's viewport of 100.
  tree.set_root(
    Column::new().child(
      ScrollVertical::new(Rect::new(100.0, 300.0))
        .with_scroll_state(scroll.clone())
        .id("feed")
        .size(100.0, 100.0),
    ),
  );
  tree.pass(&mut app, &TestSurface);
  let said = feed(&mut tree, &mut app, &state);
  assert!(said.contains(r#"{scroll_offset="0,0"}"#), "{said}");
  assert!(
    said.contains("{scroll_page=0}") && said.contains("{scroll_target_page=0}"),
    "{said}"
  );
  assert!(!said.contains("scroll_moving"), "{said}");

  // A short flick up: the finger lifts 20 into the first page, thrown at the second.
  let start = Instant::now();
  tree.set_press_clock(Some(start));
  tree.finger(1, TouchPhase::Start, 50.0, 90.0, None);
  for (millis, y) in [(8, 75.0), (16, 65.0), (24, 55.0)] {
    tree.set_press_clock(Some(start + Duration::from_millis(millis)));
    tree.finger(1, TouchPhase::Move, 50.0, y, None);
  }
  tree.set_press_clock(Some(start + Duration::from_millis(28)));
  tree.finger(1, TouchPhase::End, 50.0, 55.0, None);
  tree.pass(&mut app, &TestSurface);
  assert_eq!(scroll.scroll_y(), 20.0);
  let said = feed(&mut tree, &mut app, &state);
  assert!(said.contains("{scroll_moving=true}"), "it is on its way: {said}");
  assert!(
    said.contains("{scroll_page=0}") && said.contains("{scroll_target_page=1}"),
    "{said}"
  );

  let (reply, mut answer) = tokio::sync::oneshot::channel();
  execute(
    &mut tree,
    &mut app,
    &state,
    McpRequest {
      tool: "lurq_wait".into(),
      args: json!({}),
      reply,
      session: None,
    },
  );
  let mut frames = 0;
  while answer.try_recv().is_err() {
    assert!(
      scroll.scroll_y() < 100.0,
      "the wait goes on while the list is on its way"
    );
    frames += 1;
    assert!(frames < 400, "the wait never answers");
    // What a shell does between two frames: it steps what the finger left, and draws.
    tree.finger_tick(start + Duration::from_millis(28 + 8 * frames));
    tree.pass(&mut app, &TestSurface);
  }
  assert!(frames > 4, "the way to the page takes frames: {frames}");
  assert_eq!(scroll.scroll_y(), 100.0);
  let said = feed(&mut tree, &mut app, &state);
  assert!(said.contains(r#"{scroll_offset="0,100"}"#), "{said}");
  assert!(
    said.contains("{scroll_page=1}") && said.contains("{scroll_target_page=1}"),
    "{said}"
  );
  assert!(!said.contains("scroll_moving"), "{said}");
}
