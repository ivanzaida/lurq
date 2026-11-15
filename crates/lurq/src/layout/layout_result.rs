use std::sync::Arc;

use crate::layout::{Offset, Size, layout_engine::TextLayoutOutput};

#[derive(Clone)]
pub struct LayoutResult {
  pub size: Size,
  pub children: Vec<ChildLayout>,
  /// What laying out a text leaf wrote into its node's text state, written
  /// back when the result is served from a layout cache.
  pub(crate) text_layout: Option<Arc<TextLayoutOutput>>,
  /// Dropped from its single-line `Row`/`Column` by
  /// [`ShrinkLimit::Drop`](crate::layout::layout_kind::ShrinkLimit::Drop):
  /// laid out at zero size, neither drawn nor hit.
  pub(crate) dropped: bool,
  /// What in this subtree a result served from a layout cache must still
  /// write back into its nodes. Set when the layout engine prepares the
  /// result; a result built anywhere else may hold anything.
  pub(crate) holds: SubtreeHolds,
  /// The subtree generation this geometry was prepared for. Carried caches
  /// retain their old generation and must be checked against the new nodes.
  pub(crate) generation: u64,
  /// How far each sticky child's offset in `children` is from its place in
  /// flow, by child index: what scrolling moved it by, which the next scroll
  /// takes back out before it places the child again. `None` where no child
  /// is moved. Behind one pointer, where a `Vec` is three: few results have
  /// one, and every node of a tree has a result.
  #[allow(clippy::box_collection)]
  pub(crate) sticky_shifts: Option<Box<Vec<(usize, Offset)>>>,
  /// Where the subtree may paint, from the node's own corner: what culls a subtree that does not clip its
  /// children when it is out of view. Set when the layout engine prepares the result.
  pub(crate) reach: Reach,
}

/// The box a laid-out subtree paints in, from its node's corner: its own box with what it draws outside it (a
/// shadow, an outline, a border outside), and its children's where it does not clip them. Unknown where it may move
/// without a layout (a transform, a transition or an animation of where it is, a canvas): such a subtree is never
/// culled for being out of view.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Reach {
  pub(crate) left: f32,
  pub(crate) top: f32,
  pub(crate) right: f32,
  pub(crate) bottom: f32,
  pub(crate) known: bool,
}

impl Reach {
  pub(crate) const UNKNOWN: Self = Self {
    left: 0.0,
    top: 0.0,
    right: 0.0,
    bottom: 0.0,
    known: false,
  };

  /// A box `width` by `height`, `outset` past each side.
  pub(crate) fn of_box(width: f32, height: f32, outset: f32) -> Self {
    Self {
      left: -outset,
      top: -outset,
      right: width + outset,
      bottom: height + outset,
      known: true,
    }
  }

  /// This reach and `other`'s, placed at `offset`; unknown where either is.
  pub(crate) fn union_at(self, other: Self, offset: Offset) -> Self {
    if !self.known || !other.known {
      return Self::UNKNOWN;
    }
    Self {
      left: self.left.min(other.left + offset.x),
      top: self.top.min(other.top + offset.y),
      right: self.right.max(other.right + offset.x),
      bottom: self.bottom.max(other.bottom + offset.y),
      known: true,
    }
  }
}

/// What a laid-out subtree holds that serving it from a cache writes back:
/// a scroll container syncs its scroll state and its content's offset, a
/// text leaf restores its text layout output. A subtree holding neither is
/// served as it is: the layout engine neither walks nor copies it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct SubtreeHolds {
  pub(crate) scroll: bool,
  pub(crate) text: bool,
  /// A sticky node, which a scroll container above places again when it
  /// scrolls.
  pub(crate) sticky: bool,
}

impl SubtreeHolds {
  /// For a result nobody has looked into yet: it may hold either.
  pub(crate) const UNKNOWN: Self = Self {
    scroll: true,
    text: true,
    sticky: true,
  };
}

#[derive(Clone)]
pub struct ChildLayout {
  pub offset: Offset,
  pub result: Arc<LayoutResult>,
}

impl LayoutResult {
  /// Whether its single-line `Row`/`Column` dropped this child to make room
  /// ([`ShrinkLimit::Drop`](crate::layout::layout_kind::ShrinkLimit::Drop)).
  /// A dropped child has zero size and is neither drawn nor hit.
  pub fn is_dropped(&self) -> bool {
    self.dropped
  }

  pub(crate) fn estimated_memory_bytes(&self) -> usize {
    std::mem::size_of::<Self>()
      + self
        .text_layout
        .as_ref()
        .map_or(0, |output| output.estimated_memory_bytes())
      + self.children.len() * std::mem::size_of::<ChildLayout>()
      + self
        .children
        .iter()
        .map(|child| child.result.estimated_memory_bytes())
        .sum::<usize>()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn cloning_layout_result_shares_nested_child_results() {
    let result = LayoutResult {
      size: Size::new(10.0, 10.0),
      children: vec![ChildLayout {
        offset: Offset::default(),
        result: LayoutResult {
          size: Size::new(5.0, 5.0),
          children: Vec::new(),
          text_layout: None,
          dropped: false,
          holds: SubtreeHolds::UNKNOWN,
          generation: 0,
          sticky_shifts: None,
          reach: crate::layout::layout_result::Reach::UNKNOWN,
        }
        .into(),
      }],
      text_layout: None,
      dropped: false,
      holds: SubtreeHolds::UNKNOWN,
      generation: 0,
      sticky_shifts: None,
      reach: crate::layout::layout_result::Reach::UNKNOWN,
    };

    let mut cloned = result.clone();

    assert_ne!(result.children.as_ptr(), cloned.children.as_ptr());
    assert!(Arc::ptr_eq(&result.children[0].result, &cloned.children[0].result));
    cloned.children[0].offset.x = 3.0;
    assert_eq!(result.children[0].offset.x, 0.0);
    assert_eq!(cloned.children[0].offset.x, 3.0);
    assert!(Arc::ptr_eq(&result.children[0].result, &cloned.children[0].result));
  }
}
