//! The keys of a scroll container that has the focus itself: the arrows by a line, the page keys and Space by
//! what it shows, Home and End to its ends. A control inside it keeps its own keys.

use lurq::{
  app::Tree,
  components::{Button, Column, ScrollVertical, Spacer},
  layout::layout_kind::ScrollState,
};

use crate::support::render_pass;

fn key(tree: &mut Tree, key: &str, shift: bool) {
  tree.key_down(key.to_owned(), key.to_owned(), shift, false, false);
  tree.key_up(key.to_owned(), key.to_owned(), shift, false, false);
  render_pass(tree);
}

/// A scroll container of 100 that Tab reaches, over content of 1000: a button at its top, then room.
fn scroll(focusable: bool) -> (Tree, ScrollState) {
  let state = ScrollState::new();
  let mut scroll = ScrollVertical::new(
    Column::new()
      .child(Button::new("Top").id("top").tab_index(0))
      .child(Spacer::new().height(1000.0)),
  )
  .with_scroll_state(state.clone())
  .id("area")
  .size(200.0, 100.0);
  if focusable {
    scroll = scroll.tab_index(0);
  }
  let mut tree = Tree::new();
  tree.set_root(scroll);
  render_pass(&mut tree);
  (tree, state)
}

#[test]
fn a_focused_scroll_container_scrolls_with_the_arrows_the_page_keys_space_home_and_end() {
  let (mut tree, state) = scroll(true);
  key(&mut tree, "Tab", false);
  assert_eq!(
    tree
      .focused_element()
      .and_then(|element| element.id().map(str::to_owned))
      .as_deref(),
    Some("area")
  );
  key(&mut tree, "ArrowDown", false);
  assert_eq!(state.scroll_y(), 40.0, "a line");
  key(&mut tree, "ArrowUp", false);
  assert_eq!(state.scroll_y(), 0.0);
  key(&mut tree, "PageDown", false);
  assert_eq!(state.scroll_y(), 100.0, "what it shows");
  key(&mut tree, " ", false);
  assert_eq!(state.scroll_y(), 200.0, "Space pages too");
  key(&mut tree, " ", true);
  assert_eq!(state.scroll_y(), 100.0, "and Shift+Space back");
  key(&mut tree, "End", false);
  assert_eq!(state.scroll_y(), state.content_height() - state.viewport_height());
  key(&mut tree, "Home", false);
  assert_eq!(state.scroll_y(), 0.0);
}

#[test]
fn a_control_inside_keeps_its_keys() {
  let (mut tree, state) = scroll(false);
  key(&mut tree, "Tab", false);
  assert_eq!(
    tree
      .focused_element()
      .and_then(|element| element.id().map(str::to_owned))
      .as_deref(),
    Some("top")
  );
  key(&mut tree, "ArrowDown", false);
  key(&mut tree, "End", false);
  assert_eq!(state.scroll_y(), 0.0, "the button has the focus, not the container");
}
