//! Flex shrink for single-line Rows and Columns: distributing the overflow
//! over shrinkable children and laying each shrunk child out again at its
//! final main size, so its inner layout (scroll viewport, nested flex
//! distribution, alignment) matches the box it is given.

use super::{ChildLayoutOverride, LayoutEngine};
use crate::{
  app::glyph_engine::GlyphEngine,
  layout::{
    Constraints,
    layout_kind::{FlexParams, FlexWrap, LayoutKind},
    layout_result::LayoutResult,
  },
  node::node::Node,
};

/// Inputs of one flex line's shrink step.
pub(super) struct FlexShrinkLine<'a> {
  pub(super) children: &'a [Node],
  pub(super) params: &'a [FlexParams],
  pub(super) constraints: Constraints,
  pub(super) max_main: f32,
  pub(super) total_spacing: f32,
  pub(super) shrink_total: f32,
  pub(super) vertical: bool,
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
  /// changed, with that size as a tight main-axis constraint.
  pub(super) fn shrink_flex_line(
    &self,
    glyph_engine: &mut GlyphEngine,
    line: &FlexShrinkLine<'_>,
    results: &mut [LayoutResult],
    child_overrides: Option<&[Option<ChildLayoutOverride>]>,
  ) {
    let shrunk = shrunk_main_sizes(line, results);
    for (index, new_main) in shrunk.into_iter().enumerate() {
      let Some(new_main) = new_main.filter(|size| *size != main_size(&results[index], line.vertical)) else {
        continue;
      };
      let child_constraints = if line.vertical {
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
      };
      let child = &line.children[index];
      results[index] = self.layout_child_node(glyph_engine, child_overrides, index, child, child_constraints);
    }
  }
}

fn main_size(result: &LayoutResult, vertical: bool) -> f32 {
  if vertical {
    result.size.height
  } else {
    result.size.width
  }
}

/// The new main size of every child the overflow shrinks (`None` for the
/// rest). Children clamped at their minimum main size freeze and hand their
/// unused share to the others.
fn shrunk_main_sizes(line: &FlexShrinkLine<'_>, results: &[LayoutResult]) -> Vec<Option<f32>> {
  let count = line.children.len();
  let mut sizes = vec![None; count];
  if line.shrink_total <= 0.0 || !line.max_main.is_finite() {
    return sizes;
  }
  let total_children_main: f32 = results
    .iter()
    .zip(line.children)
    .filter(|(_, child)| !child.is_overlay_declaration())
    .map(|(result, _)| main_size(result, line.vertical))
    .sum();
  let overflow = total_children_main + line.total_spacing - line.max_main;
  if overflow <= 0.0 {
    return sizes;
  }

  let mut remaining_overflow = overflow;
  let mut remaining_shrink = line.shrink_total;
  let mut frozen = vec![false; count];
  loop {
    let mut any_clamped = false;
    for index in 0..count {
      let shrink = line.params[index].shrink;
      if frozen[index] || shrink <= 0.0 {
        continue;
      }
      let child_main = main_size(&results[index], line.vertical);
      let shrink_amount = remaining_overflow * (shrink / remaining_shrink);
      let min_main = line.children[index].min_main_size(line.vertical);
      let new_main = (child_main - shrink_amount).max(min_main);
      if new_main > child_main - shrink_amount {
        frozen[index] = true;
        remaining_overflow -= child_main - new_main;
        remaining_shrink -= shrink;
        any_clamped = true;
        sizes[index] = Some(new_main);
      }
    }
    if !any_clamped || remaining_shrink <= 0.0 {
      break;
    }
  }

  for index in 0..count {
    let shrink = line.params[index].shrink;
    if frozen[index] || shrink <= 0.0 {
      continue;
    }
    let child_main = main_size(&results[index], line.vertical);
    let shrink_amount = remaining_overflow * (shrink / remaining_shrink);
    sizes[index] = Some((child_main - shrink_amount).max(0.0));
  }
  sizes
}
