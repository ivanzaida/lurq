//! Decorations of known source ranges; the authoritative editor value remains a plain String.
use std::{ops::Range, sync::Arc};

use crate::{
  app::theme::{IconGlyph, TypographyStyle},
  layout::text_style::TextStyle,
  node::{TextColor, radius_value::RadiusValue, spacing_value::SpacingValue},
};

/// How a token is drawn: the color of its text and of its fill, the room on either side of its text, its
/// corners and its type; and, where it has them, its height, a line inside its fill, an icon at its end and
/// the looks it takes under the pointer and while the selection covers it. What the style does not set is
/// as an engine token always was: no line, no icon, as high as its text's line, padded alike at both ends.
#[derive(Clone, PartialEq)]
pub struct InputTokenStyle {
  pub(crate) foreground: TextColor,
  pub(crate) background: TextColor,
  pub(crate) padding: SpacingValue,
  pub(crate) padding_end: Option<SpacingValue>,
  pub(crate) radius: RadiusValue,
  pub(crate) typography: Option<TypographyStyle>,
  pub(crate) text: Option<TextStyle>,
  pub(crate) height: Option<f32>,
  pub(crate) border: Option<(f32, TextColor)>,
  pub(crate) icon: Option<InputTokenIcon>,
  pub(crate) hovered: Option<Arc<InputTokenStyle>>,
  pub(crate) selected: Option<Arc<InputTokenStyle>>,
}

impl InputTokenStyle {
  pub fn new(foreground: impl Into<TextColor>, background: impl Into<TextColor>) -> Self {
    Self {
      foreground: foreground.into(),
      background: background.into(),
      padding: 0.0.into(),
      padding_end: None,
      radius: 0.0.into(),
      typography: None,
      text: None,
      height: None,
      border: None,
      icon: None,
      hovered: None,
      selected: None,
    }
  }

  /// The room at both ends of the text, inside the fill. [`Self::padding_end`] sets the end's apart.
  pub fn padding(mut self, horizontal: impl Into<SpacingValue>) -> Self {
    self.padding = horizontal.into();
    self
  }

  /// The room after the text (after the icon, where there is one), in place of [`Self::padding`]'s.
  pub fn padding_end(mut self, end: impl Into<SpacingValue>) -> Self {
    self.padding_end = Some(end.into());
    self
  }

  /// The height of the fill, centred on the text's line, in place of the line's height. The line a token
  /// stands in is as high as its type makes it: give the type a line height (see [`Self::text_style`]) that
  /// leaves room around a fill taller than the text.
  pub fn height(mut self, height: f32) -> Self {
    self.height = Some(height);
    self
  }

  /// A line of `width` inside the fill, in `color`.
  pub fn border(mut self, width: f32, color: impl Into<TextColor>) -> Self {
    self.border = Some((width, color.into()));
    self
  }

  /// The token's type as it is, in place of [`Self::typography`]: its family, size, weight and line
  /// height. Its color is the foreground's. The rows of a wrapping input are as high as the highest line in
  /// them.
  pub fn text_style(mut self, text: TextStyle) -> Self {
    self.text = Some(text);
    self
  }

  /// An icon at the end of the token, after its text, in the room the token keeps for it.
  pub fn icon(mut self, icon: InputTokenIcon) -> Self {
    self.icon = Some(icon);
    self
  }

  /// How the token is drawn while the pointer is on it. Its geometry is the token's own: give it the same
  /// padding, height, type and icon, and change colors.
  pub fn hovered(mut self, hovered: InputTokenStyle) -> Self {
    self.hovered = Some(Arc::new(hovered));
    self
  }

  /// How the token is drawn while the selection covers all of it, over [`Self::hovered`]. Its geometry is
  /// the token's own, as for the hovered look.
  pub fn selected(mut self, selected: InputTokenStyle) -> Self {
    self.selected = Some(Arc::new(selected));
    self
  }

  pub fn radius(mut self, radius: impl Into<RadiusValue>) -> Self {
    self.radius = radius.into();
    self
  }

  pub fn typography(mut self, typography: TypographyStyle) -> Self {
    self.typography = Some(typography);
    self
  }
}

/// An icon at the end of a token: a glyph of a font, as [`IconGlyph`] names it, as wide and high as its size.
#[derive(Clone, PartialEq)]
pub struct InputTokenIcon {
  pub(crate) glyph: IconGlyph,
  pub(crate) size: f32,
  pub(crate) color: Option<TextColor>,
  pub(crate) gap: SpacingValue,
  pub(crate) removes: bool,
}

impl InputTokenIcon {
  /// The glyph, 12 square, in the color of the token's text, right after it.
  pub fn new(glyph: IconGlyph) -> Self {
    Self {
      glyph,
      size: 12.0,
      color: None,
      gap: 0.0.into(),
      removes: false,
    }
  }

  /// The side of the icon in logical pixels.
  pub fn size(mut self, size: f32) -> Self {
    self.size = size;
    self
  }

  /// The color of the icon, in place of the token's text's.
  pub fn color(mut self, color: impl Into<TextColor>) -> Self {
    self.color = Some(color.into());
    self
  }

  /// The room between the token's text and the icon.
  pub fn gap(mut self, gap: impl Into<SpacingValue>) -> Self {
    self.gap = gap.into();
    self
  }

  /// Whether a click on the icon takes the token away: its text is deleted as an edit is, through the
  /// input's `on_input`, as one step of its undo.
  pub fn removes(mut self, removes: bool) -> Self {
    self.removes = removes;
    self
  }
}

/// A UTF-8 source range drawn as a token. By default it is editable text, not an embedded object: the
/// caret goes inside it and an edit can make it partial. An [`atomic`](Self::atomic) one is one thing to
/// the caret.
#[derive(Clone, PartialEq)]
pub struct InputToken {
  pub(crate) range: Range<usize>,
  pub(crate) style: InputTokenStyle,
  pub(crate) atomic: bool,
}

impl InputToken {
  pub fn new(range: Range<usize>, style: InputTokenStyle) -> Self {
    Self {
      range,
      style,
      atomic: false,
    }
  }

  /// Whether the token is one thing to the caret, as a chip is: the caret never rests inside it, the
  /// arrow keys go past it whole, a click on it selects all of it, a selection that takes part of it
  /// takes all of it, and Backspace after it (Delete before it) selects it, so that the next deletes it.
  /// Its text is still the input's value: copied, pasted, undone and read as text.
  pub fn atomic(mut self, atomic: bool) -> Self {
    self.atomic = atomic;
    self
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputTokenError {
  InvalidRange,
  Overlap,
  MultilineToken,
}

impl std::fmt::Display for InputTokenError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(match self {
      Self::InvalidRange => "input token range must be a nonempty UTF-8 source range",
      Self::Overlap => "input token ranges must not overlap",
      Self::MultilineToken => "an inline input token cannot contain a line break",
    })
  }
}
impl std::error::Error for InputTokenError {}

/// Validated decorations of this EXACT source. An edit invalidates old ranges immediately;
/// pass freshly parsed ranges on the next render. Partial expressions stay ordinary editable text.
/// Debug output deliberately omits source and ranges, including when supplied to a masked input.
#[derive(Clone, PartialEq)]
pub struct InputTokens {
  pub(crate) source: Arc<str>,
  pub(crate) tokens: Arc<[InputToken]>,
}

impl std::fmt::Debug for InputTokens {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("InputTokens").finish_non_exhaustive()
  }
}

impl InputTokens {
  pub fn new(source: &str, mut tokens: Vec<InputToken>) -> Result<Self, InputTokenError> {
    tokens.sort_by_key(|t| t.range.start);
    let mut end = 0;
    for token in &tokens {
      let range = &token.range;
      if range.start >= range.end || !source.is_char_boundary(range.start) || !source.is_char_boundary(range.end) {
        return Err(InputTokenError::InvalidRange);
      }
      if range.start < end {
        return Err(InputTokenError::Overlap);
      }
      if source[range.clone()].contains(['\r', '\n']) {
        return Err(InputTokenError::MultilineToken);
      }
      end = range.end;
    }
    Ok(Self {
      source: source.into(),
      tokens: tokens.into(),
    })
  }
}
