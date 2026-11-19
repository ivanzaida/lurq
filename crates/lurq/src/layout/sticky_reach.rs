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

/// Where a sticky element is looked at from: the place of the content's start, and the box what is to be shown
/// takes, in the same coordinates. A sticky element covers it only where it spans the box across the edge it
/// sticks to: a column that sticks to the top at the side of a page covers nothing in the middle of it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Over {
  pub(crate) origin: (f32, f32),
  pub(crate) target: (f32, f32, f32, f32),
}

/// What sticks in `content`, the content of a scroll container laid out as `layout`, reaches from each edge:
/// the widest reach of each edge's sticky elements, leaving out one that holds `except`, and, `over` a box, those
/// that do not span it across their edge. What sticks in a container inside this one sticks to that one, and is
/// not counted.
pub(crate) fn sticky_reach(
  content: &Node,
  layout: &LayoutResult,
  except: Option<NodeId>,
  over: Option<Over>,
) -> StickyReach {
  fn contains(node: &Node, target: NodeId) -> bool {
    node.node_id() == target || node.children().iter().any(|child| contains(child, target))
  }
  fn walk(
    node: &Node,
    layout: &LayoutResult,
    origin: (f32, f32),
    except: Option<NodeId>,
    target: Option<(f32, f32, f32, f32)>,
    reach: &mut StickyReach,
  ) {
    for (child, child_layout) in node.children().iter().zip(&layout.children) {
      let result = &child_layout.result;
      let at = (origin.0 + child_layout.offset.x, origin.1 + child_layout.offset.y);
      if let Some(sticky) = child.sticky()
        && !except.is_some_and(|target| contains(child, target))
      {
        let (width, height) = (result.size.width, result.size.height);
        // What sticks to the top or the bottom covers the box only where it is over it across; what sticks to a
        // side, only where it is beside it.
        let across = target.is_none_or(|(x, _, w, _)| at.0 < x + w && at.0 + width > x);
        let along = target.is_none_or(|(_, y, _, h)| at.1 < y + h && at.1 + height > y);
        if let Some(inset) = sticky.top.filter(|_| across) {
          reach.top = reach.top.max(inset + height);
        }
        if let Some(inset) = sticky.bottom.filter(|_| across) {
          reach.bottom = reach.bottom.max(inset + height);
        }
        if let Some(inset) = sticky.left.filter(|_| along) {
          reach.left = reach.left.max(inset + width);
        }
        if let Some(inset) = sticky.right.filter(|_| along) {
          reach.right = reach.right.max(inset + width);
        }
      }
      if result.holds.sticky && !matches!(child.layout_kind(), LayoutKind::ScrollModifier { .. }) {
        walk(child, result, at, except, target, reach);
      }
    }
  }
  let mut reach = StickyReach::default();
  let (origin, target) = match over {
    Some(over) => (over.origin, Some(over.target)),
    None => ((0.0, 0.0), None),
  };
  walk(content, layout, origin, except, target, &mut reach);
  reach
}
