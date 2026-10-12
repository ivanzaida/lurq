//! Decorations of known source ranges; the authoritative editor value remains a plain String.
use std::{ops::Range, sync::Arc};

use crate::{
  app::theme::TypographyStyle,
  node::{TextColor, radius_value::RadiusValue, spacing_value::SpacingValue},
};

#[derive(Clone, PartialEq)]
pub struct InputTokenStyle {
  pub(crate) foreground: TextColor,
  pub(crate) background: TextColor,
  pub(crate) padding: SpacingValue,
  pub(crate) radius: RadiusValue,
  pub(crate) typography: Option<TypographyStyle>,
}

impl InputTokenStyle {
  pub fn new(foreground: impl Into<TextColor>, background: impl Into<TextColor>) -> Self {
    Self {
      foreground: foreground.into(),
      background: background.into(),
      padding: 0.0.into(),
      radius: 0.0.into(),
      typography: None,
    }
  }

  pub fn padding(mut self, horizontal: impl Into<SpacingValue>) -> Self {
    self.padding = horizontal.into();
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

/// An editable UTF-8 source range, not an embedded object or atomic word.
#[derive(Clone, PartialEq)]
pub struct InputToken {
  pub(crate) range: Range<usize>,
  pub(crate) style: InputTokenStyle,
}

impl InputToken {
  pub fn new(range: Range<usize>, style: InputTokenStyle) -> Self {
    Self { range, style }
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
