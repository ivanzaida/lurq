//! What laying out a text leaf writes into its node's runtime state.
//!
//! Text layout has side effects that the [`LayoutResult`] size does not
//! capture: the ellipsized string and the wrap decision the renderer draws,
//! and the caret geometry that selection, caret placement and pointer
//! hit-testing read. They live in the node's text state, which keeps whatever
//! the last real layout wrote. A layout served from a node's cache skips text
//! layout, so each text leaf's result carries the output that produced it, and
//! serving a cached result writes that output back. Without this, a layout
//! cached at one width and served again after a layout at another width keeps
//! the other width's ellipsis, wrapping and caret geometry.

use std::sync::Arc;

use super::LayoutEngine;
use crate::{
  layout::{Size, layout_result::LayoutResult},
  node::{
    node::Node,
    node_kind::{NodeKind, TextInputOverflow, TextInputOverflowAnchor, TextInputState, TextState},
    text_selection::CaretPositions,
  },
};

pub(crate) enum TextLayoutOutput {
  /// A `Text` or `RichText` leaf.
  Text(TextOutput),
  /// A `TextInput` leaf.
  TextInput(TextInputOutput),
}

/// Output of laying out a `Text` or `RichText` leaf.
pub(crate) struct TextOutput {
  /// The string drawn instead of the content (the ellipsized text, or the
  /// concatenated spans of selectable rich text).
  pub(super) display_text: Option<Arc<str>>,
  pub(super) render_wrap: bool,
  /// Only selectable text measures caret geometry; other text leaves the
  /// state's positions as they are.
  pub(super) caret_positions: Option<CaretPositions>,
}

impl TextLayoutOutput {
  /// The output's allocation and display text, plus its share of the caret
  /// data it holds.
  pub(crate) fn estimated_memory_bytes(&self) -> usize {
    let heap = match self {
      Self::Text(output) => {
        output.display_text.as_ref().map_or(0, |text| text.len())
          + output
            .caret_positions
            .as_ref()
            .map_or(0, CaretPositions::estimated_shared_memory_bytes)
      }
      Self::TextInput(output) => output.caret_positions.estimated_shared_memory_bytes(),
    };
    // Reference counts of the `Arc` holding the output.
    2 * std::mem::size_of::<usize>() + std::mem::size_of::<Self>() + heap
  }
}

impl TextOutput {
  fn apply(&self, state: &TextState) {
    state.set_layout_output(
      self.display_text.as_ref(),
      self.render_wrap,
      self.caret_positions.as_ref(),
    );
  }
}

/// Measurements of a `TextInput` leaf. Caret metrics and scroll offsets also
/// depend on the caret, focus and scroll position, which change between
/// layouts, so they are derived from these measurements each time the output
/// is applied instead of being stored.
pub(crate) struct TextInputOutput {
  pub(super) caret_positions: CaretPositions,
  /// The input's box.
  pub(super) size: Size,
  /// Width of the laid-out text, which bounds horizontal scrolling.
  pub(super) text_width: f32,
  /// Height of the laid-out text, which bounds vertical scrolling.
  pub(super) text_height: f32,
  pub(super) line_height: f32,
  pub(super) text_band: Option<(f32, f32)>,
}

impl TextInputOutput {
  fn apply(&self, state: &TextInputState) {
    state.set_caret_positions(self.caret_positions.clone());
    state.set_caret_height(self.line_height);
    state.set_text_band(self.text_band);
    state.sync_caret_metrics_to_position(self.line_height);
    match state.overflow() {
      TextInputOverflow::Scroll => {
        state.set_scroll_x(self.horizontal_scroll(state));
        state.set_scroll_y(0.0);
      }
      TextInputOverflow::Multiline => {
        state.set_scroll_x(0.0);
        state.set_scroll_y(self.vertical_scroll(state));
      }
    }
  }

  /// Keeps the caret of a focused single-line input in view; an unfocused one
  /// shows its configured end.
  fn horizontal_scroll(&self, state: &TextInputState) -> f32 {
    let caret_x = state.caret_x() + state.scroll_x();
    let caret_width = 1.0;
    let max_scroll = (self.text_width + caret_width - self.size.width).max(0.0);
    if !state.is_focused() {
      return match state.unfocused_overflow_anchor() {
        TextInputOverflowAnchor::Start => 0.0,
        TextInputOverflowAnchor::End => max_scroll,
      };
    }
    let scroll_x = state.scroll_x().min(max_scroll);
    if caret_x < scroll_x {
      caret_x
    } else if caret_x + caret_width > scroll_x + self.size.width {
      (caret_x + caret_width - self.size.width).min(max_scroll)
    } else {
      scroll_x
    }
  }

  /// Keeps the caret's line of a multiline input in view.
  fn vertical_scroll(&self, state: &TextInputState) -> f32 {
    let caret_y = state.caret_y() + state.scroll_y();
    let max_scroll = (self.text_height - self.size.height).max(0.0);
    let scroll_y = state.scroll_y().min(max_scroll);
    if caret_y < scroll_y {
      caret_y
    } else if caret_y + self.line_height > scroll_y + self.size.height {
      (caret_y + self.line_height - self.size.height).min(max_scroll)
    } else {
      scroll_y
    }
  }
}

impl LayoutEngine {
  /// Writes back the text output of every text leaf in `result`, a layout of
  /// `node` that is reused without laying the subtree out again.
  pub(super) fn restore_text_layout_tree(node: &Node, result: &LayoutResult) {
    Self::restore_text_layout_output(node, result);
    for (child, child_layout) in node.children().iter().zip(&result.children) {
      Self::restore_text_layout_tree(child, &child_layout.result);
    }
  }

  /// Writes the text output a cached result was laid out with back into the
  /// node's state.
  pub(super) fn restore_text_layout_output(node: &Node, result: &LayoutResult) {
    let Some(output) = result.text_layout.as_deref() else {
      return;
    };
    match (node.node_kind(), output) {
      (NodeKind::Text { state, .. }, TextLayoutOutput::Text(output)) => output.apply(state),
      #[cfg(feature = "markdown")]
      (NodeKind::RichText { state, .. }, TextLayoutOutput::Text(output)) => output.apply(state),
      (NodeKind::TextInput { state, .. }, TextLayoutOutput::TextInput(output)) => output.apply(state),
      _ => {}
    }
  }

  /// Applies a freshly computed text output to the node's state and attaches
  /// it to the result, so a later cache hit can restore it.
  pub(super) fn text_result(state: &TextState, size: Size, output: TextOutput) -> LayoutResult {
    output.apply(state);
    Self::leaf_with_output(size, TextLayoutOutput::Text(output))
  }

  pub(super) fn text_input_result(state: &TextInputState, output: TextInputOutput) -> LayoutResult {
    output.apply(state);
    Self::leaf_with_output(output.size, TextLayoutOutput::TextInput(output))
  }

  fn leaf_with_output(size: Size, output: TextLayoutOutput) -> LayoutResult {
    LayoutResult {
      size,
      children: vec![],
      text_layout: Some(Arc::new(output)),
    }
  }
}
