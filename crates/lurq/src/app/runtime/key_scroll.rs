//! The keys of a scroll container that has the focus itself, as a browser scrolls a focused scrollable region: Up
//! and Down by a line, Left and Right likewise across, PageUp and PageDown (and Space, Shift+Space) by what it
//! shows, Home and End to its start and its end. Only the container itself: a control inside it keeps its keys, and
//! so does a multiline text input, which scrolls its own text.

use super::{Tree, find_node_by_id, find_node_by_path};
use crate::{
  layout::layout_kind::{ScrollAxis, ScrollDirection},
  node::node_kind::NodeKind,
};

/// How far an arrow key scrolls: a notch of a mouse's wheel.
const LINE: f32 = 40.0;

impl Tree {
  /// Scrolls the focused element by `key` where it is a scroll container and the key is one of its keys, and says
  /// whether it did.
  pub(super) fn scroll_focused_container(&mut self, key: &str, code: &str, shift: bool) -> bool {
    let Some(root) = self.root.as_ref() else {
      return false;
    };
    let Some(focused) = self.focused_node else {
      return false;
    };
    let node = self
      .focused_path
      .as_deref()
      .and_then(|path| find_node_by_path(root, path))
      .filter(|node| node.node_id() == focused)
      .or_else(|| find_node_by_id(root, focused));
    let Some(node) = node else {
      return false;
    };
    if matches!(node.node_kind(), NodeKind::TextInput { .. }) {
      return false;
    }
    let Some((state, direction)) = node.scroll_container() else {
      return false;
    };
    let (vertical, horizontal) = match direction {
      ScrollDirection::Vertical => (true, false),
      ScrollDirection::Horizontal => (false, true),
      ScrollDirection::Both => (true, true),
    };
    // The way the page keys go: down where it scrolls down, else across.
    let along = if vertical {
      ScrollAxis::Vertical
    } else {
      ScrollAxis::Horizontal
    };
    let named = |name: &str| key == name || code == name;
    if named("ArrowDown") && vertical {
      state.scroll_by(0.0, LINE);
    } else if named("ArrowUp") && vertical {
      state.scroll_by(0.0, -LINE);
    } else if named("ArrowRight") && horizontal {
      state.scroll_by(LINE, 0.0);
    } else if named("ArrowLeft") && horizontal {
      state.scroll_by(-LINE, 0.0);
    } else if named("PageDown") || (key == " " || code == "Space") && !shift {
      state.page_axis(along, true);
    } else if named("PageUp") || (key == " " || code == "Space") && shift {
      state.page_axis(along, false);
    } else if named("Home") {
      match along {
        ScrollAxis::Vertical => state.set_scroll(state.scroll_x(), 0.0),
        ScrollAxis::Horizontal => state.set_scroll(0.0, state.scroll_y()),
      }
    } else if named("End") {
      let (end_x, end_y) = (
        (state.content_width() - state.viewport_width()).max(0.0),
        (state.content_height() - state.viewport_height()).max(0.0),
      );
      match along {
        ScrollAxis::Vertical => state.set_scroll(state.scroll_x(), end_y),
        ScrollAxis::Horizontal => state.set_scroll(end_x, state.scroll_y()),
      }
    } else {
      return false;
    }
    true
  }
}
