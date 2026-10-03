//! How far a shrinking flex child gives way: its floor from its
//! [`ShrinkLimit`], its content minimum, and, for a child whose own line holds
//! droppable children, what it holds once collapsed without them.

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
      // It keeps its natural size until its line collapses it; its own line
      // then drops every droppable child, so it holds the rest.
      let without = self.collapsed_main(child, result, vertical).min(natural);
      item.floor = natural;
      item.collapse = Some(Collapse {
        natural: without,
        floor: floor.min(without),
      });
    }
    item
  }

  /// The main size `node`, laid out naturally as `result`, holds once its
  /// line (through logical wrappers) is collapsed: its padding and spacing,
  /// every child at its natural size except those that drop, which are gone
  /// with their spacing, and those that collapse, at what they hold
  /// collapsed. Its collapsed layout starts from exactly this.
  fn collapsed_main(&self, node: &Node, result: &LayoutResult, vertical: bool) -> f32 {
    let natural = main_size(result, vertical);
    let spacing = match node.layout_kind() {
      LayoutKind::Row { spacing, wrap, .. } if !vertical && *wrap != FlexWrap::Wrap => spacing,
      LayoutKind::Column { spacing, wrap, .. } if vertical && *wrap != FlexWrap::Wrap => spacing,
      LayoutKind::LogicalModifier => {
        return match node.children().first().zip(result.children.first()) {
          Some((child, layout)) => self.collapsed_main(child, &layout.result, vertical),
          None => natural,
        };
      }
      _ => return natural,
    };
    let spacing = spacing.resolve(&self.spacing.borrow(), natural);
    let mut total = 0.0;
    let mut counted = 0usize;
    for (child, layout) in node.children().iter().zip(&result.children) {
      if child.is_overlay_declaration() {
        continue;
      }
      let child_main = main_size(&layout.result, vertical);
      let rule = child.shrink_rule();
      let shrinks = child.state_flex().is_some_and(|params| params.shrink > 0.0);
      total += if !shrinks {
        child_main
      } else if rule.limit == ShrinkLimit::Drop || rule.drop_below.is_some() {
        continue;
      } else if holds_droppable(child, vertical) {
        self.collapsed_main(child, &layout.result, vertical).min(child_main)
      } else {
        child_main
      };
      counted += 1;
    }
    let padding = self.resolved_padding_for_size(node, result.size);
    let padding_main = if vertical {
      padding.top + padding.bottom
    } else {
      padding.left + padding.right
    };
    total + spacing * (counted as f32 - 1.0).max(0.0) + padding_main
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
