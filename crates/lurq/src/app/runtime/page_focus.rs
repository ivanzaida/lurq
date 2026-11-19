//! PageUp and PageDown in a focus group that has pages ([`FocusGroup::pages`](crate::node::FocusGroup::pages)):
//! the focus goes to the stop as far before or after the focused one as the scroll the group is in shows, as it does
//! in a listbox. A group that does not scroll is one page: the keys go to its first and its last stop.

use std::collections::HashMap;

use super::{FocusTarget, Tree};
use crate::{
  core::NodeId,
  layout::{
    layout_kind::{LayoutKind, ScrollDirection},
    layout_result::LayoutResult,
  },
  node::Node,
};

impl Tree {
  /// The index among `stops` a page back (`on` false) or on from `current`: the farthest stop no more than a page
  /// from it, and the next one where even that is farther. `current` where there is none that way.
  pub(super) fn focus_group_page(&self, stops: &[FocusTarget], current: usize, on: bool) -> usize {
    let (Some(root), Some(layout)) = (&self.root, &self.last_layout) else {
      return current;
    };
    let wanted: HashMap<NodeId, usize> = stops
      .iter()
      .enumerate()
      .map(|(index, stop)| (stop.input_id, index))
      .collect();
    let mut walk = Walk {
      wanted,
      spans: vec![None; stops.len()],
      focused: stops[current].input_id,
      viewport: None,
      scrolls: Vec::new(),
    };
    walk.visit(root, layout, 0.0);
    let Some((top, bottom)) = walk.spans[current] else {
      return current;
    };
    // Without a scroll around it the group is one page.
    let page = walk.viewport.unwrap_or(f32::INFINITY);
    let span = |index: usize| walk.spans[index];
    if on {
      let mut target = None;
      for index in current + 1..stops.len() {
        match span(index) {
          Some((_, end)) if end - bottom <= page + 0.5 => target = Some(index),
          Some(_) => break,
          None => {}
        }
      }
      target.unwrap_or((current + 1).min(stops.len() - 1))
    } else {
      let mut target = None;
      for index in (0..current).rev() {
        match span(index) {
          Some((start, _)) if top - start <= page + 0.5 => target = Some(index),
          Some(_) => break,
          None => {}
        }
      }
      target.unwrap_or(current.saturating_sub(1))
    }
  }
}

/// One walk of the laid out tree: where each stop is down the window, and how high the innermost scroll that runs
/// down around the focused one shows.
struct Walk {
  wanted: HashMap<NodeId, usize>,
  spans: Vec<Option<(f32, f32)>>,
  focused: NodeId,
  viewport: Option<f32>,
  /// The heights of the scrolls that run down around the node visited, innermost last.
  scrolls: Vec<f32>,
}

impl Walk {
  fn visit(&mut self, node: &Node, layout: &LayoutResult, y: f32) {
    if layout.dropped {
      return;
    }
    let id = node.node_id();
    if let Some(&index) = self.wanted.get(&id) {
      self.spans[index] = Some((y, y + layout.size.height));
      if id == self.focused {
        self.viewport = self.scrolls.last().copied();
      }
    }
    let scrolls = matches!(
      node.layout_kind(),
      LayoutKind::ScrollModifier {
        direction: ScrollDirection::Vertical | ScrollDirection::Both,
        ..
      }
    );
    if scrolls {
      self.scrolls.push(layout.size.height);
    }
    for (child, child_layout) in node.children().iter().zip(&layout.children) {
      self.visit(child, &child_layout.result, y + child_layout.offset.y);
    }
    if scrolls {
      self.scrolls.pop();
    }
  }
}
