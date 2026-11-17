pub use crate::node::node_kind::TextOverflow;
use crate::{
  app::theme::TypographyStyle,
  impl_into_node,
  layout::text_style::{FontFeatures, TextAlign, TextStyle},
  node::{TextColor, TextTransformMode},
};

impl_into_node!(Text);

impl Text {
  pub fn new(content: &str) -> Self {
    Self::from_node(crate::node::Node::text(content))
  }

  pub fn styled(content: &str, style: TextStyle) -> Self {
    Self::from_node(crate::node::Node::text_styled(content, style))
  }

  /// A paragraph of runs, each in a style of its own: a word in another weight, a run in another color. It is
  /// laid out, wrapped, drawn and selected as one text.
  ///
  /// ```
  /// use lurq::{
  ///   components::Text,
  ///   layout::{quad::RichTextSpan, text_style::{FontWeight, TextStyle}},
  /// };
  ///
  /// let body = TextStyle { font_size: 14.0, ..TextStyle::default() };
  /// let bold = TextStyle { weight: FontWeight::Bold, ..body.clone() };
  /// let line = Text::rich([
  ///   RichTextSpan::new("Saved ", body.clone()),
  ///   RichTextSpan::new("3 files", bold),
  ///   RichTextSpan::new(" to the folder.", body),
  /// ]);
  /// ```
  pub fn rich(spans: impl IntoIterator<Item = crate::layout::quad::RichTextSpan>) -> Self {
    Self::from_node(crate::node::Node::rich_text(spans.into_iter().collect()))
  }

  /// The element that stands in the paragraph's line at its next object: a span of a [`Text::rich`] that is one
  /// [`OBJECT_CHARACTER`](super::OBJECT_CHARACTER)
  /// ([`RichTextSpan::object`](crate::layout::quad::RichTextSpan::object)). The first call gives the first object,
  /// the next the second, in the order of the text.
  ///
  /// The element is laid out by itself, no wider than the paragraph, and the object takes its width in the line:
  /// it wraps with the words as one of them, and the line keeps the height of its text, with the element on the
  /// middle of it. Unlike an input's objects ([`TextInput::objects`](super::TextInput::objects)) it is an
  /// element like any other: a press, the focus and the pointer reach what is in it, Tab stops at what it holds
  /// in the order of the text, and tooling reads it. A mention, a chip, a picture among the words, an inline
  /// link preview. A paragraph whose objects stand above or below its lines draws them whole.
  ///
  /// ```
  /// use lurq::{
  ///   components::{Column, Text},
  ///   layout::{quad::RichTextSpan, text_style::TextStyle},
  /// };
  ///
  /// let body = TextStyle { font_size: 14.0, ..TextStyle::default() };
  /// let sentence = Text::rich([
  ///   RichTextSpan::new("The gaps are explained in ", body.clone()),
  ///   RichTextSpan::object(body.clone()),
  ///   RichTextSpan::new(" — read it before the review.", body),
  /// ])
  /// .object(Column::new().width(120.0).height(20.0));
  /// ```
  ///
  /// It needs the feature `markdown`, which the input's objects need too.
  #[cfg(feature = "markdown")]
  pub fn object(mut self, element: impl Into<crate::node::Element>) -> Self {
    let element = element.into();
    self.update_node(|node| {
      if matches!(node.node_kind(), crate::node::node_kind::NodeKind::RichText { .. }) {
        node.push_child_element(element);
        // What stands over or under the line is drawn whole.
        crate::node::NodeUpdate::overflow_visible(node);
      }
    });
    self
  }

  /// The elements of the paragraph's objects, in the order of the text: [`Text::object`] for each.
  #[cfg(feature = "markdown")]
  pub fn objects(mut self, elements: impl IntoIterator<Item = impl Into<crate::node::Element>>) -> Self {
    for element in elements {
      self = self.object(element);
    }
    self
  }

  pub fn nowrap(mut self) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_wrap(node, false));
    self
  }

  pub fn selectable(mut self, selectable: bool) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::selectable(node, selectable));
    self
  }

  /// Keep this text out of inspection: it is laid out and painted on screen
  /// as usual, but DevTools (tree, inspector, snapshots) and the MCP tools
  /// (`lurq_read_tree`, `lurq_inspect`, the `lurq_find*` lookups, and
  /// accessible names derived from it) show [`REDACTED`](crate::core::REDACTED)
  /// in its place, with the attribute `sensitive=true`, and their screenshots
  /// (`lurq_screenshot`, a DevTools node screenshot) show an opaque grey bar
  /// where its glyphs are painted. The app's own screenshots
  /// (`WindowHandle::screenshot`) show the text. For a one-time code or
  /// a secret the user must read. Selection follows `selectable` as for any
  /// text (off by default); a selectable sensitive text can be selected and
  /// copied with Ctrl+C/Cmd+C, which no inspector sees. Keep the value itself
  /// in a [`Sensitive`](crate::core::Sensitive) so that it does not reach
  /// DevTools' signal values and history either.
  pub fn sensitive(mut self) -> Self {
    self.update_node(crate::node::node::Node::set_text_sensitive);
    self
  }

  pub fn text_transform_mode(mut self, mode: TextTransformMode) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_transform_mode(node, mode));
    self
  }

  pub fn variant(mut self, style: impl Into<TypographyStyle>) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_variant(node, style));
    self
  }

  pub fn text_align(mut self, align: impl Into<TextAlign>) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_align(node, align));
    self
  }

  /// Extra space after every glyph in logical pixels (negative tightens),
  /// overriding the style's [`TextStyle::letter_spacing`].
  pub fn letter_spacing(mut self, letter_spacing: f32) -> Self {
    self.update_node(|node| node.set_text_letter_spacing(letter_spacing));
    self
  }

  /// OpenType feature settings, overriding the style's
  /// [`TextStyle::font_features`] (typography roles included).
  pub fn font_features(mut self, font_features: impl Into<FontFeatures>) -> Self {
    let font_features = font_features.into();
    self.update_node(|node| node.set_text_font_features(font_features));
    self
  }

  pub fn text_overflow(mut self, overflow: TextOverflow) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_overflow(node, overflow));
    self
  }

  pub fn color(mut self, color: impl Into<TextColor>) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_color(node, color));
    self
  }

  pub fn text_shadow(mut self, shadow: crate::layout::text_style::TextShadow) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_shadow(node, shadow));
    self
  }

  /// Draws a line under the text, over the underline of its style if it has
  /// one: `TextUnderline::new()` is the font's underline in the text's color.
  pub fn underline(mut self, underline: crate::layout::text_style::TextUnderline) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_underline(node, underline));
    self
  }

  /// Lays the text out in at most `max_lines` lines, like CSS `line-clamp`:
  /// a wrapped text that needs more ends in an ellipsis on the last of
  /// them. `0` is no limit. A text that does not wrap (`nowrap`, or
  /// `TextOverflow::Elipsis`, which keeps it on one line) is not affected.
  pub fn max_lines(mut self, max_lines: u32) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_max_lines(node, max_lines));
    self
  }

  /// Draws a line through the text: `TextStrikethrough::new()` is the font's
  /// strikeout in the text's color.
  pub fn strikethrough(mut self, strikethrough: crate::layout::text_style::TextStrikethrough) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::text_strikethrough(node, strikethrough));
    self
  }

  pub fn caret_color(mut self, color: impl Into<crate::node::TextColor>) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::caret_color(node, color));
    self
  }

  pub fn selection_color(mut self, color: impl Into<crate::node::TextColor>) -> Self {
    self.update_node(|node| crate::node::NodeUpdate::selection_color(node, color));
    self
  }

  /// Fills behind ranges of the text, such as the matches of a search: one
  /// per laid-out line a range covers, each in its own color, under the
  /// glyphs and under a selection. Ranges are in bytes of the content and
  /// stay on their words when the text re-wraps; the text is shaped as
  /// without them. Pass an empty list to clear them.
  ///
  /// ```
  /// use lurq::{
  ///   components::{Text, TextHighlight},
  ///   node::color::Color,
  /// };
  ///
  /// let current = Color::new(250, 204, 21, 255);
  /// let other = Color::new(250, 204, 21, 90);
  /// Text::new("one match, two match")
  ///   .highlights(vec![
  ///     TextHighlight::new(4..9, current).radius(3.0),
  ///     TextHighlight::new(15..20, other).radius(3.0),
  ///   ]);
  /// ```
  pub fn highlights(
    mut self,
    highlights: impl Into<std::sync::Arc<[crate::node::text_highlight::TextHighlight]>>,
  ) -> Self {
    let highlights = highlights.into();
    self.update_node(|node| node.set_text_highlights(highlights));
    self
  }
}

impl Default for Text {
  fn default() -> Self {
    Self::new("")
  }
}
