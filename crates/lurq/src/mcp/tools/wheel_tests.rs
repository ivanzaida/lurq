//! `lurq_interact`'s `wheel` is a notch of a mouse's wheel, which the window animates: the content is where the
//! wheel sent it once the window is idle, which is what `lurq_wait` waits for.

use std::time::{Duration, Instant};

use serde_json::json;

use super::{
  tests::{TestSurface, call, state},
  *,
};
use crate::{
  app::capture_test_engine::GlyphPainter,
  components::{Rect, ScrollVertical},
  layout::layout_kind::ScrollState,
  mcp::shared::McpToolOutput,
};

#[test]
fn a_wait_after_a_wheel_answers_when_the_content_is_where_the_wheel_sent_it() {
  let scroll = ScrollState::new();
  let mut tree = Tree::new();
  let mut app = App::new();
  let state = state();
  let (factory, _screen) = GlyphPainter::factory();
  tree.set_render_engine_factory(factory);
  tree.resize(300, 200);
  tree.set_root(
    ScrollVertical::new(Rect::new(300.0, 2000.0))
      .with_scroll_state(scroll.clone())
      .size(300.0, 200.0),
  );
  let start = Instant::now();
  tree.set_animation_clock(Some(start));
  tree.pass(&mut app, &TestSurface);

  call(
    &mut tree,
    &mut app,
    &state,
    "lurq_interact",
    json!({"action": "wheel", "x": 150.0, "y": 100.0, "delta_y": -40.0}),
  )
  .expect("the wheel turns");
  assert_eq!(scroll.scroll_target().1, 40.0);

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
      scroll.scroll_y() < 40.0,
      "the wait goes on while the content is on its way"
    );
    frames += 1;
    assert!(frames < 100, "the wait never answers");
    tree.set_animation_clock(Some(start + Duration::from_millis(8 * frames)));
    tree.pass(&mut app, &TestSurface);
  }
  assert!(frames > 4, "the wheel is animated: {frames} frames");
  assert_eq!(
    scroll.scroll_y(),
    40.0,
    "the same offset a wheel that scrolls at once leaves"
  );
  assert!(!tree.has_active_timeline());

  // Idle again: a wait answers at once.
  let idle = call(&mut tree, &mut app, &state, "lurq_wait", json!({})).expect("a wait");
  assert!(matches!(idle, McpToolOutput::Text(text) if text == "already idle"));
}
