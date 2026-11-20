//! The bounds of an element, to what reads the tree, are where it is drawn: its box in the layout with its
//! own transform and those of what it is in. A strip moved by a transform (a pager of an app's own) has its
//! elements found, pressed and captured where they show, and not where the layout put them before the move.

use std::sync::{
  Arc,
  atomic::{AtomicU32, Ordering},
};

use serde_json::json;

use super::{
  tests::{TestSurface, call, state},
  *,
};
use crate::{
  app::{capture_test_engine::GlyphPainter, events::MouseEvent},
  components::{Column, Rect, Row},
  mcp::{McpToolResult, shared::McpToolOutput},
  node::transform::Transform2D,
};

fn said(output: McpToolResult) -> String {
  match output.expect("the tool answers") {
    McpToolOutput::Text(text) => text,
    _ => panic!("the tool answers in text"),
  }
}

/// The ref in the first line of an answer that has one.
fn first_ref(answer: &str) -> String {
  let from = answer.find("ref_").expect("a ref in the answer");
  answer[from..]
    .chars()
    .take_while(|letter| letter.is_ascii_alphanumeric() || *letter == '_')
    .collect()
}

#[test]
fn an_element_in_a_strip_moved_by_a_transform_is_where_it_is_drawn() {
  let pressed = Arc::new(AtomicU32::new(0));
  let mut tree = Tree::new();
  let mut app = App::new();
  let state = state();
  let (factory, _screen) = GlyphPainter::factory();
  tree.set_render_engine_factory(factory);
  tree.resize(400, 300);
  let third = {
    let pressed = pressed.clone();
    Rect::new(400.0, 100.0)
      .flex_shrink(0.0)
      .id("third")
      .on_click(move |_: MouseEvent| {
        pressed.fetch_add(1, Ordering::Relaxed);
      })
  };
  // Three pages of 400 side by side, moved two pages to the left: the third is the one that shows.
  let strip = Row::new()
    .id("strip")
    .size(1200.0, 100.0)
    .flex_shrink(0.0)
    .transform(Transform2D::translate(-800.0, 0.0))
    .child(Rect::new(400.0, 100.0).flex_shrink(0.0).id("first"))
    .child(Rect::new(400.0, 100.0).flex_shrink(0.0))
    .child(third);
  // And a square drawn at twice its size, about its middle.
  let grown = Rect::new(100.0, 100.0)
    .id("grown")
    .transform(Transform2D::scale_uniform(2.0));
  tree.set_root(Column::new().size(400.0, 300.0).child(strip).child(grown));
  tree.pass(&mut app, &TestSurface);

  let found = said(call(
    &mut tree,
    &mut app,
    &state,
    "lurq_find_by_id",
    json!({"id": "third"}),
  ));
  assert!(found.contains("@0,0 400x100"), "where it is drawn, not at 800: {found}");
  let first = said(call(
    &mut tree,
    &mut app,
    &state,
    "lurq_find_by_id",
    json!({"id": "first"}),
  ));
  assert!(first.contains("@-800,0 400x100"), "{first}");
  let grown = said(call(
    &mut tree,
    &mut app,
    &state,
    "lurq_find_by_id",
    json!({"id": "grown"}),
  ));
  assert!(
    grown.contains("@-50,50 200x200"),
    "twice as large about its middle: {grown}"
  );

  // The outline says the same.
  let outline = said(call(&mut tree, &mut app, &state, "lurq_read_tree", json!({})));
  let line = outline
    .lines()
    .find(|line| line.contains("#third"))
    .unwrap_or_else(|| panic!("{outline}"));
  assert!(line.contains("@0,0 400x100"), "{line}");

  // A press on its ref lands on it, and a capture of it has something to show.
  let found = said(call(
    &mut tree,
    &mut app,
    &state,
    "lurq_find_by_id",
    json!({"id": "third"}),
  ));
  let third = first_ref(&found);
  call(
    &mut tree,
    &mut app,
    &state,
    "lurq_interact",
    json!({"action": "click", "ref": third}),
  )
  .expect("the press lands");
  assert_eq!(pressed.load(Ordering::Relaxed), 1);
  let (reply, mut answer) = tokio::sync::oneshot::channel();
  execute(
    &mut tree,
    &mut app,
    &state,
    McpRequest {
      tool: "lurq_screenshot".into(),
      args: json!({"ref": third}),
      reply,
      session: None,
    },
  );
  // The capture is taken from the next frame drawn.
  tree.pass(&mut app, &TestSurface);
  match answer.try_recv().expect("the capture answered") {
    Ok(McpToolOutput::Image { .. }) => {}
    Ok(_) => panic!("a capture is a picture"),
    Err(error) => panic!("it was drawn at 0,0, and the capture said: {error}"),
  }
}
