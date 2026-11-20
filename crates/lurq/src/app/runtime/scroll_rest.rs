//! Content that comes to rest. What scrolls a container for longer than one event has an end of its own: a
//! finger lifts and its momentum goes on, a list that scrolls by pages goes on to one, content pulled past
//! an end springs back, a wheel's notch is drawn over several frames. The container's `on_scroll_end`
//! handlers hear the end of all of it, once, when nothing moves the content any more ([`Tree::scroll_rests`]),
//! and not when the finger lifts.
//!
//! The page the content rests on ([`ScrollState::rested_page`]) is taken there too, and at every layout
//! besides ([`Tree::rest_pages_after_layout`]): what moves the content at once (a touchpad, the keys, an offset
//! the app set, a layout that holds the offset to shorter content) has no end to tell, and the page follows
//! it all the same. `on_page_change` and [`ScrollState::page_signal`] are what an app that shows one page at a
//! time goes by.
//!
//! The `lurq::scroll` tracing target tells, at debug, when a container's content starts to move with a finger,
//! which page it is sent to at the lift, and where it comes to rest.

use super::{Tree, find_node_by_id, find_scroll_node_id_by_state};
use crate::{
  app::events::{ScrollPhase, ScrollSource},
  core::NodeId,
  layout::layout_kind::{LayoutKind, ScrollDirection, ScrollState},
  node::node::{EventHandler, Node},
};

/// The handlers of a page a container came to rest on, and the page each is told.
type PageCalls = Vec<(Vec<EventHandler<usize>>, usize)>;

impl Tree {
  /// The content of `state` came to rest after `source` moved it: the nodes of `heard` hear the scroll's
  /// end, at the point `at` in logical pixels, and the container, the last of them, the page it rests on
  /// where that is another.
  pub(super) fn scroll_rests(&mut self, state: &ScrollState, heard: &[NodeId], at: (f32, f32), source: ScrollSource) {
    state.set_held(false);
    tracing::debug!(
      target: "lurq::scroll",
      source = source.as_str(),
      x = state.scroll_x(),
      y = state.scroll_y(),
      page = ?state.page(),
      "rest"
    );
    for target in heard {
      self.dispatch_scroll_handlers_for_node(*target, at.0, at.1, 0.0, 0.0, ScrollPhase::End, source);
    }
    self.rest_page_of(state, heard.last().copied());
    self.apply_reactive_updates_after_event();
  }

  /// Takes the page `state`'s content is at as the one it rests on, and tells the container's
  /// `on_page_change` handlers when it is another: `container`, or the node of the tree that has the state.
  fn rest_page_of(&mut self, state: &ScrollState, container: Option<NodeId>) {
    let Some((before, page)) = state.rest_page() else {
      return;
    };
    tracing::debug!(target: "lurq::scroll", from = ?before, to = ?page, "page");
    let calls = self.root.as_ref().and_then(|root| {
      let node = container
        .and_then(|id| find_node_by_id(root, id))
        .filter(|node| node.scroll_container().is_some_and(|(held, _)| held.ptr_eq(state)))
        .or_else(|| find_scroll_node_id_by_state(root, state).and_then(|id| find_node_by_id(root, id)))?;
      page_call(node, before, page)
    });
    if let Some((handlers, page)) = calls {
      for handler in handlers {
        handler.call(&page);
      }
    }
  }

  /// After a layout: every container whose page someone asks for, and whose content nothing moves, rests on
  /// the page it is at. What scrolled it at once since the last layout (a touchpad, the keys, the app) is
  /// heard of here.
  pub(super) fn rest_pages_after_layout(&mut self) {
    let Some(root) = &self.root else {
      return;
    };
    let mut calls = PageCalls::new();
    collect_rested_pages(root, &mut calls);
    if calls.is_empty() {
      return;
    }
    for (handlers, page) in calls {
      for handler in handlers {
        handler.call(&page);
      }
    }
    if self.root_ctx.as_ref().is_some_and(crate::app::ctx::Ctx::any_dirty) {
      self.needs_redraw = true;
    }
  }
}

/// The handlers of `node` for a page that went from `before` to `page`, and the page they are told: the one
/// along the way the container scrolls.
fn page_call(node: &Node, before: (usize, usize), page: (usize, usize)) -> Option<(Vec<EventHandler<usize>>, usize)> {
  if node.events.on_page_change.is_empty() {
    return None;
  }
  let (_, direction) = node.scroll_container()?;
  let told = match direction {
    ScrollDirection::Horizontal => (before.0 != page.0).then_some(page.0),
    ScrollDirection::Vertical => (before.1 != page.1).then_some(page.1),
    ScrollDirection::Both => Some(page.1),
  }?;
  Some((node.events.on_page_change.clone(), told))
}

fn collect_rested_pages(node: &Node, calls: &mut PageCalls) {
  if let LayoutKind::ScrollModifier { state, .. } = node.layout_kind()
    && (!node.events.on_page_change.is_empty() || state.page_watched())
    && let Some((before, page)) = state.rest_page()
  {
    tracing::debug!(target: "lurq::scroll", from = ?before, to = ?page, "page");
    calls.extend(page_call(node, before, page));
  }
  for child in node.children() {
    // A subtree with no scroll container has no page to tell.
    if child.layout_cache.subtree().scroll {
      collect_rested_pages(child, calls);
    }
  }
}
