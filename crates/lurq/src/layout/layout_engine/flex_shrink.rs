//! Flex shrink for single-line Rows and Columns: distributing the overflow
//! over shrinkable children and laying each shrunk child out again at its
//! final main size, so its inner layout (scroll viewport, nested flex
//! distribution, alignment) matches the box it is given.

use super::{
  ChildLayoutOverride, LayoutEngine,
  shrink_distribution::{LineSpace, ShrinkItem, ShrinkOutcome, distribute},
};
use crate::{
  app::glyph_engine::GlyphEngine,
  layout::{
    Constraints, Size,
    layout_kind::{FlexParams, FlexWrap, LayoutKind, ShrinkLimit},
    layout_result::LayoutResult,
  },
  node::{node::Node, spacing_value::SpacingValue},
};

/// Inputs of one flex line's shrink step.
pub(super) struct FlexShrinkLine<'a> {
  pub(super) children: &'a [Node],
  pub(super) params: &'a [FlexParams],
  pub(super) constraints: Constraints,
  pub(super) max_main: f32,
  pub(super) spacing: f32,
  pub(super) total_spacing: f32,
  pub(super) shrink_total: f32,
  pub(super) vertical: bool,
}

/// Whether a child takes up space in its flex line: overlay declarations and
/// children the shrink step dropped do not, and get no spacing either.
pub(super) fn occupies_line(child: &Node, result: &LayoutResult) -> bool {
  !child.is_overlay_declaration() && !result.dropped
}

impl LayoutEngine {
  /// Whether a dirty child of this Row/Column was last laid out at a size the
  /// shrink step chose. Its cached tight main-axis constraint is an output of
  /// the parent's previous distribution, so repairing the child under it would
  /// pin the old size; the parent must measure it naturally and shrink again.
  pub(super) fn has_dirty_shrunk_child(node: &Node) -> bool {
    let vertical = match node.layout_kind() {
      LayoutKind::Column { wrap, .. } if *wrap != FlexWrap::Wrap => true,
      LayoutKind::Row { wrap, .. } if *wrap != FlexWrap::Wrap => false,
      _ => return false,
    };
    node.children().iter().any(|child| {
      !child.is_overlay_declaration()
        && child.layout_cache.is_dirty()
        && child.state_flex().is_some_and(|params| params.shrink > 0.0)
        && child.layout_cache.constraints().is_some_and(|constraints| {
          if vertical {
            constraints.min_height == constraints.max_height
          } else {
            constraints.min_width == constraints.max_width
          }
        })
    })
  }

  /// Shrinks overflowing children and re-lays out every child whose main size
  /// changed, with that size as a tight main-axis constraint. A dropped child
  /// is laid out at zero size and its result marked dropped.
  pub(super) fn shrink_flex_line(
    &self,
    glyph_engine: &mut GlyphEngine,
    line: &FlexShrinkLine<'_>,
    results: &mut [LayoutResult],
    child_overrides: Option<&[Option<ChildLayoutOverride>]>,
  ) {
    for (index, outcome) in self.shrink_outcomes(line, results) {
      let child = &line.children[index];
      match outcome {
        ShrinkOutcome::Keep => {}
        ShrinkOutcome::Resize(new_main) => {
          if new_main == main_size(&results[index], line.vertical) {
            continue;
          }
          let child_constraints = shrunk_constraints(line, new_main);
          results[index] = self.layout_child_node(glyph_engine, child_overrides, index, child, child_constraints);
        }
        ShrinkOutcome::Drop => {
          let zero = Constraints::tight(Size::default());
          let mut dropped = self.layout_child_node(glyph_engine, child_overrides, index, child, zero);
          dropped.size = Size::default();
          dropped.dropped = true;
          results[index] = dropped;
        }
      }
    }
  }

  /// What the overflow does to each shrinking child, by child index.
  fn shrink_outcomes(&self, line: &FlexShrinkLine<'_>, results: &[LayoutResult]) -> Vec<(usize, ShrinkOutcome)> {
    if line.shrink_total <= 0.0 || !line.max_main.is_finite() {
      return Vec::new();
    }
    let total_children_main: f32 = results
      .iter()
      .zip(line.children)
      .filter(|(_, child)| !child.is_overlay_declaration())
      .map(|(result, _)| main_size(result, line.vertical))
      .sum();
    let overflow = total_children_main + line.total_spacing - line.max_main;
    if overflow <= 0.0 {
      return Vec::new();
    }

    let mut indices = Vec::new();
    let mut items = Vec::new();
    for (index, child) in line.children.iter().enumerate() {
      let factor = line.params[index].shrink;
      if child.is_overlay_declaration() || factor <= 0.0 {
        continue;
      }
      indices.push(index);
      items.push(self.shrink_item(child, &results[index], factor, line.vertical));
    }
    let occupied = line
      .children
      .iter()
      .filter(|child| !child.is_overlay_declaration())
      .count();
    let space = LineSpace {
      overflow,
      gap: line.spacing,
      occupied,
    };
    indices.into_iter().zip(distribute(&items, space)).collect()
  }

  fn shrink_item(&self, child: &Node, result: &LayoutResult, factor: f32, vertical: bool) -> ShrinkItem {
    let rule = child.shrink_rule();
    let floor = match rule.limit {
      ShrinkLimit::MinSize | ShrinkLimit::Drop => child.min_main_size(vertical),
      ShrinkLimit::Content => child
        .min_main_size(vertical)
        .max(self.content_min_main(child, result, vertical)),
    };
    ShrinkItem {
      factor,
      natural: main_size(result, vertical),
      floor,
      order: rule.order,
      droppable: rule.limit == ShrinkLimit::Drop,
    }
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
  /// does. A droppable child counts as gone, its spacing included.
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
      total += match child.shrink_rule().limit {
        _ if !shrinks => child_main,
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

fn main_size(result: &LayoutResult, vertical: bool) -> f32 {
  if vertical {
    result.size.height
  } else {
    result.size.width
  }
}

fn shrunk_constraints(line: &FlexShrinkLine<'_>, new_main: f32) -> Constraints {
  if line.vertical {
    Constraints {
      min_width: 0.0,
      max_width: line.constraints.max_width,
      min_height: new_main,
      max_height: new_main,
    }
  } else {
    Constraints {
      min_width: new_main,
      max_width: new_main,
      min_height: 0.0,
      max_height: line.constraints.max_height,
    }
  }
}
