//! Fills behind ranges of a text, such as the matches of a search.

use std::ops::Range;

use crate::node::TextColor;

/// A fill behind a range of a [`Text`](crate::components::Text::highlights)
/// or a `Markdown` document (`MarkdownProps::highlights`): one rounded
/// rectangle per laid-out line the range covers, under the glyphs and under
/// a selection. It does not change how the text is shaped, so it follows the
/// text when it re-wraps.
///
/// `range` is in bytes of the text: of a `Text`'s content, or of a
/// document's rendered text (`MarkdownProps::rendered_text`). Ends inside a
/// character are moved back to its start; a range past the text's end is
/// cut at it.
///
/// ```
/// use lurq::{components::TextHighlight, node::color::Color};
///
/// let current = TextHighlight::new(4..9, Color::new(255, 200, 0, 255)).radius(3.0);
/// let other = TextHighlight::new(20..25, Color::new(255, 200, 0, 90));
/// // A mark that reaches past its words, with an edge: a search's current match.
/// let edged = TextHighlight::new(4..9, Color::new(254, 243, 199, 255))
///   .padding(3.0, 1.0)
///   .border(1.5, Color::new(180, 83, 9, 255));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct TextHighlight {
  pub range: Range<usize>,
  /// The fill: a color or a palette role.
  pub color: TextColor,
  /// Corner radius of each line's fill, in logical pixels.
  pub radius: f32,
  /// How far each line's fill reaches past the range's glyphs, across and
  /// up and down, in logical pixels: less than none draws a fill inside
  /// them, lower than the line (inline code in a paragraph of tall lines).
  /// The text is not moved by it.
  pub padding: (f32, f32),
  /// A line around each line's fill, inside it: its width and its color.
  pub border: Option<(f32, TextColor)>,
}

impl TextHighlight {
  pub fn new(range: Range<usize>, color: impl Into<TextColor>) -> Self {
    Self {
      range,
      color: color.into(),
      radius: 0.0,
      padding: (0.0, 0.0),
      border: None,
    }
  }

  /// Corner radius of each line's fill, in logical pixels.
  pub fn radius(mut self, radius: f32) -> Self {
    self.radius = radius;
    self
  }

  /// How far each line's fill reaches past the range's glyphs: `x` before
  /// and after them, `y` above and below. A mark of a search, a little wider
  /// than its words; a negative one draws inside them, as a box of code
  /// lower than the paragraph's line. The text is shaped and placed as
  /// without it.
  pub fn padding(mut self, x: f32, y: f32) -> Self {
    self.padding = (x, y);
    self
  }

  /// A line of `width` around each line's fill, inside it, in `color`: the
  /// current match of a search, set apart from the others.
  pub fn border(mut self, width: f32, color: impl Into<TextColor>) -> Self {
    self.border = Some((width, color.into()));
    self
  }

  /// The padding to draw: a non-finite one draws none.
  pub(crate) fn drawn_padding(&self) -> (f32, f32) {
    let clean = |value: f32| if value.is_finite() { value } else { 0.0 };
    (clean(self.padding.0), clean(self.padding.1))
  }

  /// The radius to draw: a negative or non-finite one draws square corners.
  pub(crate) fn drawn_radius(&self) -> f32 {
    if self.radius.is_finite() {
      self.radius.max(0.0)
    } else {
      0.0
    }
  }
}

impl crate::app::component::DevtoolsInspectable for TextHighlight {
  fn inspect(&self, formatter: &mut crate::app::component::DevtoolsFormatter<'_>) {
    formatter.value_debug(self);
  }
}
