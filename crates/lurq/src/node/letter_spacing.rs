//! Letter-spacing modifiers for text and text-input nodes.

use super::{node::Node, node_kind::NodeKind};

impl Node {
  /// Overrides the resolved text style's letter spacing, whatever its source
  /// (default, typography variant or explicit style).
  pub(crate) fn set_text_letter_spacing(&mut self, letter_spacing: f32) {
    if let NodeKind::Text { style, .. } = &mut self.node_kind {
      style.set_letter_spacing(letter_spacing);
      self.layout_cache.invalidate();
    }
  }

  /// Sets the letter spacing of the value and placeholder text.
  pub(crate) fn set_text_input_letter_spacing(&mut self, letter_spacing: f32) {
    if let NodeKind::TextInput {
      style,
      placeholder_style,
      ..
    } = &mut self.node_kind
    {
      let unchanged = |style: &crate::layout::text_style::TextStyle| style.letter_spacing == letter_spacing;
      if unchanged(style) && placeholder_style.as_ref().is_none_or(unchanged) {
        return;
      }
      style.letter_spacing = letter_spacing;
      if let Some(placeholder_style) = placeholder_style {
        placeholder_style.letter_spacing = letter_spacing;
      }
      self.layout_cache.invalidate();
    }
  }
}
