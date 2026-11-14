//! How far in from each edge of a scroll container what sticks to that edge reaches: its inset and its size.
//! What is scrolled into view is shown clear of it (`app/runtime/scroll_into_view.rs`), and the bars of a
//! container that asks run beside what scrolls and not over what stays (`scrollbars_clear_of_sticky`).

use crate::{
  core::NodeId,
  layout::{layout_kind::LayoutKind, layout_result::LayoutResult},
  node::Node,
};

/// How far in from each edge of a scroll container's viewport what sticks to it reaches.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct StickyReach {
  pub(crate) top: f32,
  pub(crate) bottom: f32,
  pub(crate) left: f32,
  pub(crate) right: f32,
}

/// What sticks in `content`, the content of a scroll container laid out as `layout`, reaches from each edge:
/// the widest reach of each edge's sticky elements, leaving out one that holds `except`. What sticks in a
/// container inside this one sticks to that one, and is not counted.
pub(crate) fn sticky_reach(content: &Node, layout: &LayoutResult, except: Option<NodeId>) -> StickyReach {
  fn contains(node: &Node, target: NodeId) -> bool {
    node.node_id() == target || node.children().iter().any(|child| contains(child, target))
  }
  fn walk(node: &Node, layout: &LayoutResult, except: Option<NodeId>, reach: &mut StickyReach) {
    for (child, child_layout) in node.children().iter().zip(&layout.children) {
      let result = &child_layout.result;
      if let Some(sticky) = child.sticky()
        && !except.is_some_and(|target| contains(child, target))
      {
        let (width, height) = (result.size.width, result.size.height);
        if let Some(inset) = sticky.top {
          reach.top = reach.top.max(inset + height);
        }
        if let Some(inset) = sticky.bottom {
          reach.bottom = reach.bottom.max(inset + height);
        }
        if let Some(inset) = sticky.left {
          reach.left = reach.left.max(inset + width);
        }
        if let Some(inset) = sticky.right {
          reach.right = reach.right.max(inset + width);
        }
      }
      if result.holds.sticky && !matches!(child.layout_kind(), LayoutKind::ScrollModifier { .. }) {
        walk(child, result, except, reach);
      }
    }
  }
  let mut reach = StickyReach::default();
  walk(content, layout, except, &mut reach);
  reach
}
