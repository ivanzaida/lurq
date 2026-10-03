//! How far a shrinking flex child gives way: its floor from its
//! [`ShrinkLimit`], its content minimum, and, for a child whose own line holds
//! droppable children, the size that keeps them all and what it collapses to
//! without them.

use std::collections::BTreeMap;

use super::{
  LayoutEngine,
  flex_shrink::main_size,
  shrink_distribution::{Collapse, ShrinkItem},
};
use crate::{
  layout::{
    layout_kind::{FlexWrap, LayoutKind, ShrinkLimit},
    layout_result::LayoutResult,
  },
  node::{node::Node, spacing_value::SpacingValue},
};

/// How much more than its collapsed size the size that keeps every droppable
/// child must be for a child to collapse at all.
const COLLAPSE_TOLERANCE: f32 = 0.01;

impl LayoutEngine {
  pub(super) fn shrink_item(&self, child: &Node, result: &LayoutResult, factor: f32, vertical: bool) -> ShrinkItem {
    let rule = child.shrink_rule();
    let natural = main_size(result, vertical);
    let floor = match rule.limit {
      ShrinkLimit::MinSize | ShrinkLimit::Drop => child.min_main_size(vertical),
      ShrinkLimit::Content => child
        .min_main_size(vertical)
        .max(self.content_min_main(child, result, vertical)),
    };
    let droppable = rule.limit == ShrinkLimit::Drop;
    // `.shrink_drop_below(size)`: shrink no further than `size`, then drop.
    let drop_below = rule.drop_below.filter(|_| !droppable);
    let mut item = ShrinkItem {
      factor,
      natural,
      floor: drop_below.map_or(floor, |size| floor.max(size.min(natural))),
      order: rule.order,
      droppable,
      drops_at_floor: drop_below.is_some(),
      collapse: None,
    };
    if !droppable && drop_below.is_none() && holds_droppable(child, vertical) {
      // Shrink only as far as keeps every droppable child inside, then
      // collapse without them.
      let keep = self.keep_min_main(child, result, vertical).max(floor);
      let without = self.content_min_main(child, result, vertical).max(floor).min(natural);
      if keep > without + COLLAPSE_TOLERANCE {
        item.floor = keep;
        item.collapse = Some(Collapse {
          natural: without,
          floor,
        });
      }
    }
    item
  }

  /// The main size `node`, laid out as `result`, keeps when every shrinking
  /// child of its own line is at its limit ([`ShrinkLimit::Content`]).
  fn content_min_main(&self, node: &Node, result: &LayoutResult, vertical: bool) -> f32 {
    let natural = main_size(result, vertical);
    let content = match node.layout_kind() {
      LayoutKind::Row { spacing, wrap, .. } if !vertical && *wrap != FlexWrap::Wrap => {
        self.line_content_min(node, result, spacing, vertical)
      }
      LayoutKind::Column { spacing, wrap, .. } if vertical && *wrap != FlexWrap::Wrap => {
        self.line_content_min(node, result, spacing, vertical)
      }
      LayoutKind::LogicalModifier => match (node.children().first(), result.children.first()) {
        (Some(child), Some(layout)) => self.content_min_main(child, &layout.result, vertical),
        _ => return natural,
      },
      _ => return natural,
    };
    let padding = self.resolved_padding_for_size(node, result.size);
    let padding_main = if vertical {
      padding.top + padding.bottom
    } else {
      padding.left + padding.right
    };
    (content + padding_main).min(natural)
  }

  /// The content minimum of a line on its own axis: its spacing, the laid-out
  /// size of each child that does not shrink and the limit of each child that
  /// does. A child that can drop counts as gone, its spacing included.
  fn line_content_min(&self, node: &Node, result: &LayoutResult, spacing: &SpacingValue, vertical: bool) -> f32 {
    let spacing = spacing.resolve(&self.spacing.borrow(), main_size(result, vertical));
    let mut total = 0.0;
    let mut counted = 0usize;
    for (child, layout) in node.children().iter().zip(&result.children) {
      if child.is_overlay_declaration() {
        continue;
      }
      let child_main = main_size(&layout.result, vertical);
      let shrinks = child.state_flex().is_some_and(|params| params.shrink > 0.0);
      let rule = child.shrink_rule();
      total += match rule.limit {
        _ if !shrinks => child_main,
        _ if rule.drop_below.is_some() => continue,
        ShrinkLimit::Drop => continue,
        ShrinkLimit::MinSize => child.min_main_size(vertical),
        ShrinkLimit::Content => {
          child
            .min_main_size(vertical)
            .max(self.content_min_main(child, &layout.result, vertical))
        }
      };
      counted += 1;
    }
    total + spacing * (counted as f32 - 1.0).max(0.0)
  }

  /// The main size `node`, laid out as `result`, can shrink to while every
  /// droppable child in its line (or a nested one) keeps its place. Its line
  /// gives way by order, so a droppable child of order `o` stays while the
  /// overflow fits what the shrinking children of orders up to `o` can
  /// absorb; the tightest such order bounds how far `node` can shrink.
  fn keep_min_main(&self, node: &Node, result: &LayoutResult, vertical: bool) -> f32 {
    let natural = main_size(result, vertical);
    match node.layout_kind() {
      LayoutKind::Row { wrap, .. } if !vertical && *wrap != FlexWrap::Wrap => {}
      LayoutKind::Column { wrap, .. } if vertical && *wrap != FlexWrap::Wrap => {}
      LayoutKind::LogicalModifier => {
        return match node.children().first().zip(result.children.first()) {
          Some((child, layout)) => self.keep_min_main(child, &layout.result, vertical),
          None => natural,
        };
      }
      _ => return natural,
    }
    let mut capacities: BTreeMap<i32, f32> = BTreeMap::new();
    let mut giving_way: Vec<i32> = Vec::new();
    for (child, layout) in node.children().iter().zip(&result.children) {
      let Some(factor) = child
        .state_flex()
        .map(|params| params.shrink)
        .filter(|shrink| *shrink > 0.0 && !child.is_overlay_declaration())
      else {
        continue;
      };
      let item = self.shrink_item(child, &layout.result, factor, vertical);
      if item.droppable || item.drops_at_floor || item.collapse.is_some() {
        giving_way.push(item.order);
      }
      if !item.droppable {
        *capacities.entry(item.order).or_insert(0.0) += item.natural - item.floor;
      }
    }
    let absorbable = giving_way
      .iter()
      .map(|&order| capacities.range(..=order).map(|(_, capacity)| capacity).sum::<f32>())
      .fold(f32::INFINITY, f32::min);
    if absorbable.is_finite() {
      (natural - absorbable).max(0.0)
    } else {
      natural
    }
  }
}

/// Whether the line of `node` (a Row in a row, a Column in a column, or one
/// nested in a shrinking child of it) holds a shrinking child that can drop.
fn holds_droppable(node: &Node, vertical: bool) -> bool {
  match node.layout_kind() {
    LayoutKind::Row { wrap, .. } if !vertical && *wrap != FlexWrap::Wrap => {}
    LayoutKind::Column { wrap, .. } if vertical && *wrap != FlexWrap::Wrap => {}
    LayoutKind::LogicalModifier => {
      return node
        .children()
        .first()
        .is_some_and(|child| holds_droppable(child, vertical));
    }
    _ => return false,
  }
  node.children().iter().any(|child| {
    let rule = child.shrink_rule();
    !child.is_overlay_declaration()
      && child.state_flex().is_some_and(|params| params.shrink > 0.0)
      && (rule.limit == ShrinkLimit::Drop || rule.drop_below.is_some() || holds_droppable(child, vertical))
  })
}
