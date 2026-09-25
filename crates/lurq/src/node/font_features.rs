//! OpenType feature modifiers for text and text-input nodes.

use super::{node::Node, node_kind::NodeKind};
use crate::layout::text_style::{FontFeatures, TextStyle};

impl Node {
  /// Overrides the resolved text style's feature settings, whatever its source
  /// (default, typography variant or explicit style).
  pub(crate) fn set_text_font_features(&mut self, font_features: FontFeatures) {
    if let NodeKind::Text { style, .. } = &mut self.node_kind {
      style.set_font_features(font_features);
      self.layout_cache.invalidate();
    }
  }

  /// Sets the feature settings of the value and placeholder text.
  pub(crate) fn set_text_input_font_features(&mut self, font_features: FontFeatures) {
    if let NodeKind::TextInput {
      style,
      placeholder_style,
      ..
    } = &mut self.node_kind
    {
      let unchanged = |style: &TextStyle| style.font_features == font_features;
      if unchanged(style) && placeholder_style.as_ref().is_none_or(unchanged) {
        return;
      }
      style.font_features = font_features.clone();
      if let Some(placeholder_style) = placeholder_style {
        placeholder_style.font_features = font_features;
      }
      self.layout_cache.invalidate();
    }
  }
}
