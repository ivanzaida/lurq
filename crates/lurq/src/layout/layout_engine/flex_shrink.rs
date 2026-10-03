//! Flex shrink for single-line Rows and Columns: distributing the overflow
//! over shrinkable children and laying each shrunk child out again at its
//! final main size, so its inner layout (scroll viewport, nested flex
//! distribution, alignment) matches the box it is given.

use super::{
  ChildLayoutOverride, LayoutEngine,
  shrink_distribution::{LineSpace, ShrinkOutcome, distribute},
};
use crate::{
  app::glyph_engine::GlyphEngine,
  layout::{
    Constraints, Size,
    layout_kind::{FlexParams, FlexWrap, LayoutKind, ShrinkRule},
    layout_result::LayoutResult,
  },
  node::node::Node,
};

/// How much less than its given size a shrunk child must hold, after a drop
/// inside it, to be laid out again at what it holds.
const RELEASE_TOLERANCE: f32 = 0.5;

/// Inputs of one flex line's shrink step.
pub(super) struct FlexShrinkLine<'a> {
  pub(super) children: &'a [Node],
  pub(super) params: &'a [FlexParams],
  pub(super) constraints: Constraints,
  pub(super) max_main: f32,
  pub(super) spacing: f32,
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
  ///
  /// A shrunk child whose own line dropped a child to fit (directly or in a
  /// nested line on the same axis) holds less than the size it was given; it
  /// is laid out again at what it holds, and the line is distributed again
  /// with that child fixed at that size, so the space the drop released goes
  /// back to the line instead of staying blank inside the child.
  pub(super) fn shrink_flex_line(
    &self,
    glyph_engine: &mut GlyphEngine,
    line: &FlexShrinkLine<'_>,
    results: &mut [LayoutResult],
    child_overrides: Option<&[Option<ChildLayoutOverride>]>,
  ) {
    let mut fixed = vec![false; results.len()];
    let mut outcomes = self.shrink_outcomes(line, results, &fixed);
    if outcomes.is_empty() {
      return;
    }
    let naturals = results.to_vec();
    loop {
      let mut released = false;
      // What gave way whole this round; it stays so if the line is
      // distributed again, so nothing comes back at the same width.
      let mut gave_way = Vec::new();
      for (index, outcome) in outcomes {
        let child = &line.children[index];
        match outcome {
          ShrinkOutcome::Keep => {}
          ShrinkOutcome::Resize(new_main) | ShrinkOutcome::Collapsed(new_main) => {
            if matches!(outcome, ShrinkOutcome::Collapsed(_)) {
              gave_way.push(index);
            }
            if new_main == main_size(&results[index], line.vertical) {
              continue;
            }
            let child_constraints = shrunk_constraints(line, new_main);
            results[index] = self.layout_child_node(glyph_engine, child_overrides, index, child, child_constraints);
            let Some(held) = self.released_main(child, &results[index], line.vertical) else {
              continue;
            };
            let held = held.max(child.min_main_size(line.vertical));
            if held < new_main - RELEASE_TOLERANCE {
              let child_constraints = shrunk_constraints(line, held);
              results[index] = self.layout_child_node(glyph_engine, child_overrides, index, child, child_constraints);
              fixed[index] = true;
              released = true;
            }
          }
          ShrinkOutcome::Drop => {
            let zero = Constraints::tight(Size::default());
            let mut dropped = self.layout_child_node(glyph_engine, child_overrides, index, child, zero);
            dropped.size = Size::default();
            dropped.dropped = true;
            results[index] = dropped;
            gave_way.push(index);
          }
        }
      }
      if !released {
        return;
      }
      for index in gave_way {
        fixed[index] = true;
      }
      // Distribute again from the natural sizes, the released children fixed.
      for (index, natural) in naturals.iter().enumerate() {
        if !fixed[index] {
          results[index] = natural.clone();
        }
      }
      outcomes = self.shrink_outcomes(line, results, &fixed);
    }
  }

  /// What the overflow does to each shrinking child that is not `fixed`, by
  /// child index.
  fn shrink_outcomes(
    &self,
    line: &FlexShrinkLine<'_>,
    results: &[LayoutResult],
    fixed: &[bool],
  ) -> Vec<(usize, ShrinkOutcome)> {
    if line.shrink_total <= 0.0 || !line.max_main.is_finite() {
      return Vec::new();
    }
    let total_children_main: f32 = results
      .iter()
      .zip(line.children)
      .filter(|(_, child)| !child.is_overlay_declaration())
      .map(|(result, _)| main_size(result, line.vertical))
      .sum();
    // A child dropped in an earlier round takes no spacing.
    let occupied = results
      .iter()
      .zip(line.children)
      .filter(|(result, child)| occupies_line(child, result))
      .count();
    let total_spacing = line.spacing * (occupied as f32 - 1.0).max(0.0);
    let overflow = total_children_main + total_spacing - line.max_main;
    if overflow <= 0.0 {
      return Vec::new();
    }

    let mut indices = Vec::new();
    let mut items = Vec::new();
    // Whole-pixel sharing is part of the give-way rules: a line without
    // orders or limits shares exactly as before them.
    let mut whole_pixels = false;
    for (index, child) in line.children.iter().enumerate() {
      let factor = line.params[index].shrink;
      if child.is_overlay_declaration() || factor <= 0.0 {
        continue;
      }
      whole_pixels |= child.shrink_rule() != ShrinkRule::default();
      if fixed[index] {
        continue;
      }
      indices.push(index);
      items.push(self.shrink_item(child, &results[index], factor, line.vertical));
    }
    let space = LineSpace {
      overflow,
      gap: line.spacing,
      occupied,
      whole_pixels,
    };
    indices.into_iter().zip(distribute(&items, space)).collect()
  }

  /// The main size a shrunk `node`, laid out as `result`, really holds when
  /// its line (or a nested line on the same axis) dropped a child to fit:
  /// padding, spacing and the children that stayed. `None` when nothing in
  /// it dropped, or it is not a line on this axis.
  fn released_main(&self, node: &Node, result: &LayoutResult, vertical: bool) -> Option<f32> {
    let spacing = match node.layout_kind() {
      LayoutKind::Row { spacing, wrap, .. } if !vertical && *wrap != FlexWrap::Wrap => spacing,
      LayoutKind::Column { spacing, wrap, .. } if vertical && *wrap != FlexWrap::Wrap => spacing,
      LayoutKind::LogicalModifier => {
        let (child, layout) = node.children().first().zip(result.children.first())?;
        return self.released_main(child, &layout.result, vertical);
      }
      _ => return None,
    };
    let mut any_dropped = false;
    let mut total = 0.0;
    let mut counted = 0usize;
    for (child, layout) in node.children().iter().zip(&result.children) {
      if child.is_overlay_declaration() {
        continue;
      }
      if layout.result.dropped {
        any_dropped = true;
        continue;
      }
      any_dropped |= self.released_main(child, &layout.result, vertical).is_some();
      total += main_size(&layout.result, vertical);
      counted += 1;
    }
    if !any_dropped {
      return None;
    }
    let spacing = spacing.resolve(&self.spacing.borrow(), main_size(result, vertical));
    let padding = self.resolved_padding_for_size(node, result.size);
    let padding_main = if vertical {
      padding.top + padding.bottom
    } else {
      padding.left + padding.right
    };
    Some(total + spacing * (counted as f32 - 1.0).max(0.0) + padding_main)
  }
}

pub(super) fn main_size(result: &LayoutResult, vertical: bool) -> f32 {
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
