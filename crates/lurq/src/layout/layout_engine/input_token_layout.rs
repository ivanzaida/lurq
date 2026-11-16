//! Styled paragraph geometry and paint share the exact input layout.
use std::{ops::Range, sync::Arc};

use super::{DEFAULT_TEXT_INPUT_WIDTH, LayoutEngine, ResolvedPadding, TextInputOutput, multiline_preferred_height};
pub(crate) use crate::app::glyph_engine::input_tokens::DecoratedInputLayout as TokenLayout;
use crate::{
  app::glyph_engine::{
    GlyphEngine,
    input_tokens::{InputConfig, ParagraphFormat, TextLines, TokenChip, TokenIcon, TokenStyle},
  },
  components::{InputTokenStyle, InputTokens},
  layout::{
    Constraints, Size,
    text_style::{TextAlign, TextStyle},
  },
  node::node_kind::{TextInputOverflow, TextInputState},
};

/// The room a multiline input whose lines do not wrap lays them out in: wider than any line of code.
const UNWRAPPED_INPUT_WIDTH: f32 = 1_000_000.0;

impl LayoutEngine {
  pub(super) fn layout_input_tokens(
    &self,
    glyphs: &mut GlyphEngine,
    state: &TextInputState,
    base: &TextStyle,
    constraints: Constraints,
    tokens: InputTokens,
    padding: ResolvedPadding,
  ) -> crate::layout::layout_result::LayoutResult {
    let width = Self::decorated_input_width(state, constraints);
    let (hovered, selection) = (state.hovered_token(), state.selection_bounds());
    let styles = tokens
      .tokens
      .iter()
      .enumerate()
      .map(|(index, token)| {
        // The selection covering all of a token, then the pointer on it, give it their looks.
        let covered = selection.is_some_and(|(start, end)| start <= token.range.start && token.range.end <= end);
        let look = match (&token.style.selected, &token.style.hovered) {
          (Some(selected), _) if covered => selected.as_ref(),
          (_, Some(look)) if hovered == Some(index) => look.as_ref(),
          _ => &token.style,
        };
        let mut style = self.token_style(look, token.range.clone(), base, width);
        style.chip.atomic = token.atomic;
        // A token in its selected look is its own sign of the selection: it is drawn over the highlight.
        style.chip.over_selection = covered && token.style.selected.is_some();
        style
      })
      .collect();
    self.layout_decorated_input(
      glyphs,
      state,
      base,
      constraints,
      (tokens.source, styles, Vec::new()),
      padding,
    )
  }

  /// The glyph engine's style of a token drawn with `look`, its roles resolved in this engine's theme.
  fn token_style(&self, look: &InputTokenStyle, range: Range<usize>, base: &TextStyle, width: f32) -> TokenStyle {
    let mut text = look.text.clone().unwrap_or_else(|| {
      look
        .typography
        .and_then(|role| self.typography.borrow().try_get(role))
        .unwrap_or_else(|| base.clone())
    });
    let palette = self.palette.borrow();
    text.color = look
      .foreground
      .resolve(&palette)
      .unwrap_or_else(|| palette.text_color(base));
    let spacing = self.spacing.borrow();
    let length = |value: &crate::node::spacing_value::SpacingValue| {
      let px = value.resolve(&spacing, width);
      if px.is_finite() { px.max(0.0) } else { 0.0 }
    };
    let padding = length(&look.padding);
    let end = look.padding_end.as_ref().map_or(padding, &length);
    let radius = look.radius.resolve(&self.radii.borrow());
    let radius = if radius.is_finite() { radius.max(0.0) } else { 0.0 };
    let icon = look.icon.as_ref().map(|icon| {
      let size = if icon.size.is_finite() { icon.size.max(0.0) } else { 0.0 };
      let color = icon
        .color
        .as_ref()
        .and_then(|color| color.resolve(&palette))
        .unwrap_or(text.color);
      (
        TokenIcon {
          style: TextStyle {
            font_family: icon.glyph.family.clone(),
            font_size: size,
            line_height: 1.0,
            text_align: TextAlign::Center,
            color,
            ..TextStyle::default()
          },
          glyph: icon.glyph.glyph,
          size,
          inset: end,
        },
        length(&icon.gap) + size,
      )
    });
    let chip = TokenChip {
      padding_end: match &icon {
        Some((_, room)) => Some(end + room),
        None => look.padding_end.is_some().then_some(end),
      },
      height: look
        .height
        .filter(|height| height.is_finite())
        .map(|height| height.max(0.0)),
      border: look.border.as_ref().and_then(|(width, color)| {
        let color = color.resolve(&palette)?;
        (width.is_finite() && *width > 0.0).then_some((*width, color))
      }),
      icon: icon.map(|(icon, _)| icon),
      atomic: false,
      over_selection: false,
    };
    TokenStyle {
      range,
      text,
      padding,
      fill: look.background.resolve(&palette).map(|color| (color, radius)),
      lines: TextLines::default(),
      chip,
    }
  }

  /// Lays out `source` with a text style per range, sharing the geometry
  /// with paint: input tokens and styled spans both draw this way.
  pub(super) fn layout_decorated_input(
    &self,
    glyphs: &mut GlyphEngine,
    state: &TextInputState,
    base: &TextStyle,
    constraints: Constraints,
    (source, styles, paragraphs): (Arc<str>, Vec<TokenStyle>, Vec<ParagraphFormat>),
    padding: ResolvedPadding,
  ) -> crate::layout::layout_result::LayoutResult {
    let multiline = state.overflow() == TextInputOverflow::Multiline;
    // A multiline input whose lines do not wrap is as wide as its longest line, and the caret after it: its
    // paragraphs are laid out in a room no line reaches the end of.
    let wrap = state.wraps_lines();
    let width = if multiline && !wrap {
      UNWRAPPED_INPUT_WIDTH
    } else {
      Self::decorated_input_width(state, constraints)
    };
    let layout = glyphs.decorated_input(InputConfig {
      source,
      base: base.clone(),
      styles,
      width,
      min_width: constraints.min_width.max(DEFAULT_TEXT_INPUT_WIDTH).min(width),
      wrap: multiline,
      // Kinds of paragraphs are laid out where the text wraps, a paragraph at a time.
      paragraphs: if wrap { paragraphs } else { Vec::new() },
    });
    let preferred = Size::new(
      if wrap {
        layout.size.width.max(DEFAULT_TEXT_INPUT_WIDTH)
      } else if multiline {
        layout.size.width + super::TEXT_INPUT_CARET_WIDTH
      } else {
        DEFAULT_TEXT_INPUT_WIDTH
      },
      if multiline {
        multiline_preferred_height(state, layout.size.height, layout.line_height)
      } else {
        layout.line_height
      },
    );
    let output = TextInputOutput {
      caret_positions: layout.positions(),
      size: constraints.constrain(preferred),
      text_width: layout.size.width,
      text_height: layout.size.height,
      line_height: layout.line_height,
      text_band: Some(layout.selection_band(0.0)),
      token_layout: Some(layout),
      padding,
    };
    Self::text_input_result(state, output)
  }

  /// The width a decorated input lays its text out in: the room it is given
  /// when it wraps, its own width when it scrolls.
  fn decorated_input_width(state: &TextInputState, constraints: Constraints) -> f32 {
    let max_width = if state.overflow() == TextInputOverflow::Multiline {
      constraints.max_width
    } else {
      constraints.constrain(Size::new(DEFAULT_TEXT_INPUT_WIDTH, 0.0)).width
    };
    if max_width.is_nan() { 0.0 } else { max_width.max(0.0) }
  }
}
