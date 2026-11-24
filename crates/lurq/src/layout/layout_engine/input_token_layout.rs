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
    let now = crate::node::node_kind::text_input::token_fade::now();
    let (fade, length) = state.token_fade_at(now);
    let styles = tokens
      .tokens
      .iter()
      .enumerate()
      .map(|(index, token)| {
        // The selection covering all of a token, then the pointer on it, give it their looks.
        let covered = selection.is_some_and(|(start, end)| start <= token.range.start && token.range.end <= end);
        let share = crate::node::node_kind::text_input::token_fade::share(index, hovered, fade, now, length);
        let mut style = match (&token.style.selected, &token.style.hovered) {
          (Some(selected), _) if covered => self.token_style(selected, token.range.clone(), base, width),
          // On its way between its rest look and the one under the pointer: their colors mixed.
          (_, Some(look)) if share > 0.0 && share < 1.0 => {
            let (rest, over) = (
              self.token_style(&token.style, token.range.clone(), base, width),
              self.token_style(look, token.range.clone(), base, width),
            );
            mix_tokens(rest, over, share)
          }
          (_, Some(look)) if share >= 1.0 => self.token_style(look, token.range.clone(), base, width),
          _ => self.token_style(&token.style, token.range.clone(), base, width),
        };
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
    if let Some(weight) = look.weight {
      text.weight = weight;
    }
    if let Some(style) = look.font_style {
      text.style = style;
    }
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
      object: None,
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
      exact_rows: false,
    });
    let preferred = Size::new(
      if wrap {
        layout.size.width.max(DEFAULT_TEXT_INPUT_WIDTH)
      } else if multiline {
        layout.size.width + self.caret.borrow().width().max(super::TEXT_INPUT_CARET_WIDTH)
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
      caret_width: self.caret.borrow().width().max(super::TEXT_INPUT_CARET_WIDTH),
      token_layout: Some(layout),
      padding,
    };
    Self::text_input_result(state, output)
  }

  /// The width a decorated input lays its text out in: the room it is given
  /// when it wraps, its own width when it scrolls.
  pub(super) fn decorated_input_width(state: &TextInputState, constraints: Constraints) -> f32 {
    let max_width = if state.overflow() == TextInputOverflow::Multiline {
      constraints.max_width
    } else {
      constraints.constrain(Size::new(DEFAULT_TEXT_INPUT_WIDTH, 0.0)).width
    };
    if max_width.is_nan() { 0.0 } else { max_width.max(0.0) }
  }
}

/// A token `share` of the way from its look `rest` to its look `over`: their colors mixed, of its text, its fill,
/// its line and its icon, and the rest of it the look it is nearer.
fn mix_tokens(rest: TokenStyle, over: TokenStyle, share: f32) -> TokenStyle {
  use crate::node::color::Color;
  let mix = |a: Color, b: Color| {
    let channel = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * share).round() as u8;
    Color::new(
      channel(a.r(), b.r()),
      channel(a.g(), b.g()),
      channel(a.b(), b.b()),
      channel(a.a(), b.a()),
    )
  };
  let clear = |color: Color| Color::new(color.r(), color.g(), color.b(), 0);
  let mut style = if share < 0.5 { rest.clone() } else { over.clone() };
  style.text.color = mix(rest.text.color, over.text.color);
  style.fill = match (rest.fill, over.fill) {
    (Some((a, _)), Some((b, radius))) => Some((mix(a, b), radius)),
    (Some((a, radius)), None) => Some((mix(a, clear(a)), radius)),
    (None, Some((b, radius))) => Some((mix(clear(b), b), radius)),
    (None, None) => None,
  };
  style.chip.border = match (rest.chip.border, over.chip.border) {
    (Some((width, a)), Some((_, b))) => Some((width, mix(a, b))),
    (Some((width, a)), None) => Some((width, mix(a, clear(a)))),
    (None, Some((width, b))) => Some((width, mix(clear(b), b))),
    (None, None) => None,
  };
  if let (Some(icon), Some(a), Some(b)) = (
    style.chip.icon.as_mut(),
    rest.chip.icon.as_ref().map(|icon| icon.style.color),
    over.chip.icon.as_ref().map(|icon| icon.style.color),
  ) {
    icon.style.color = mix(a, b);
  }
  style
}
