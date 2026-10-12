//! One measured source-to-caret map shared by decorated input layout and paint.
use std::sync::Arc;

use super::{DEFAULT_TEXT_INPUT_WIDTH, LayoutEngine, TextInputOutput, multiline_preferred_height};
use crate::{
  app::glyph_engine::GlyphEngine,
  components::{InputTokenStyle, InputTokens},
  layout::{
    Constraints, Size,
    layout_result::LayoutResult,
    text_style::{TextAlign, TextStyle},
  },
  node::{
    color::Color,
    node_kind::{TextInputOverflow, TextInputState},
    text_selection::{CaretPosition, CaretPositions},
  },
};

pub(crate) struct TokenPiece {
  pub(super) text: String,
  pub(super) style: TextStyle,
  pub(super) padding: f32,
  pub(super) x: f32,
  pub(super) y: f32,
  pub(super) width: f32,
  pub(super) height: f32,
  pub(super) backgrounds: Vec<(f32, f32, f32)>,
  pub(super) background: Option<(Color, f32)>,
}

pub(crate) struct TokenLayout {
  pub(crate) source: Arc<str>,
  pub(super) pieces: Vec<TokenPiece>,
}

impl TokenLayout {
  pub(super) fn estimated_shared_memory_bytes(layout: &Arc<Self>) -> usize {
    let pieces = layout.pieces.capacity() * std::mem::size_of::<TokenPiece>();
    let heap: usize = layout
      .pieces
      .iter()
      .map(|piece| piece.text.capacity() + piece.backgrounds.capacity() * std::mem::size_of::<(f32, f32, f32)>())
      .sum();
    (std::mem::size_of::<Self>() + pieces + heap + layout.source.len()) / Arc::strong_count(layout)
  }
}

struct Flow<'a> {
  glyphs: &'a mut GlyphEngine,
  width: f32,
  wrap: bool,
  line_height: f32,
  x: f32,
  y: f32,
  used_width: f32,
  positions: Vec<CaretPosition>,
  pieces: Vec<TokenPiece>,
}

impl Flow<'_> {
  fn line_break(&mut self, index: usize) {
    self.used_width = self.used_width.max(self.x);
    self.x = 0.0;
    self.y += self.line_height;
    self.positions.push(CaretPosition {
      index,
      x: 0.0,
      y: self.y,
    });
  }

  fn piece(&mut self, text: &str, start: usize, mut style: TextStyle, pad: f32, fill: Option<(Color, f32)>) {
    if text.is_empty() {
      return;
    }
    style.text_align = TextAlign::Left;
    style.line_height = self.line_height / style.font_size.max(1.0);
    let natural = self.glyphs.measure_text(text, &style, f32::MAX).width + pad * 2.0;
    if self.wrap && self.x > 0.0 && self.x + natural > self.width {
      self.line_break(start);
    }
    let available = if self.wrap {
      (self.width - self.x - pad * 2.0).max(1.0)
    } else {
      f32::MAX
    };
    let rows = self.glyphs.caret_positions(text, &style, available, self.wrap);
    let mut ranges: Vec<(f32, usize, usize)> = Vec::new();
    for caret in rows.iter() {
      if let Some(row) = ranges.last_mut().filter(|row| row.0 == caret.y) {
        row.1 = row.1.min(caret.index);
        row.2 = row.2.max(caret.index);
      } else {
        ranges.push((caret.y, caret.index, caret.index));
      }
    }
    for (row, (_, from, to)) in ranges.into_iter().enumerate() {
      if row > 0 {
        self.line_break(start + from);
      }
      self.row(&text[from..to], start + from, style.clone(), pad, fill);
    }
  }

  fn row(&mut self, text: &str, start: usize, style: TextStyle, pad: f32, fill: Option<(Color, f32)>) {
    let carets = self.glyphs.caret_positions(text, &style, f32::MAX, false);
    let origin = (self.x + pad, self.y);
    let mut backgrounds: Vec<(f32, f32, f32)> = Vec::new();
    let mut final_position = (0.0, 0.0);
    let mut widest = 0.0_f32;
    for caret in carets.iter() {
      self.positions.push(CaretPosition {
        index: start + caret.index,
        x: origin.0 + caret.x,
        y: origin.1 + caret.y,
      });
      if let Some(row) = backgrounds.last_mut().filter(|row| row.1 == caret.y) {
        row.2 = row.2.max(caret.x + pad * 2.0);
      } else {
        backgrounds.push((0.0, caret.y, caret.x + pad * 2.0));
      }
      widest = widest.max(caret.x);
      if caret.index == text.len() {
        final_position = (caret.x, caret.y);
      }
    }
    self.used_width = self.used_width.max(origin.0 + widest + pad);
    self.pieces.push(TokenPiece {
      text: text.to_owned(),
      style,
      padding: pad,
      x: origin.0,
      y: origin.1,
      width: widest.max(1.0),
      height: final_position.1 + self.line_height,
      backgrounds,
      background: fill,
    });
    self.x = origin.0 + final_position.0 + pad;
    self.y = origin.1 + final_position.1;
  }

  fn plain(&mut self, text: &str, start: usize, style: &TextStyle) {
    let mut from = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
      if !matches!(ch, '\r' | '\n') {
        continue;
      }
      self.words(&text[from..index], start + from, style);
      from = index + ch.len_utf8();
      if ch == '\r' && chars.peek().is_some_and(|(_, ch)| *ch == '\n') {
        chars.next();
        from += 1;
      }
      self.line_break(start + from);
    }
    self.words(&text[from..], start + from, style);
  }

  fn words(&mut self, text: &str, mut start: usize, style: &TextStyle) {
    for word in text.split_inclusive(char::is_whitespace) {
      self.piece(word, start, style.clone(), 0.0, None);
      start += word.len();
    }
  }

  fn row_width(&self, y: f32) -> f32 {
    self
      .pieces
      .iter()
      .filter(|piece| piece.y == y)
      .fold(0.0_f32, |width, piece| width.max(piece.x + piece.width + piece.padding))
  }

  fn align_rows(&mut self, width: f32, align: TextAlign) {
    if align == TextAlign::Left {
      return;
    }
    let mut first = 0;
    while first < self.positions.len() {
      let y = self.positions[first].y;
      let end = first + self.positions[first..].iter().take_while(|caret| caret.y == y).count();
      let row_width = self.row_width(y);
      let shift = (width - row_width).max(0.0) * if align == TextAlign::Center { 0.5 } else { 1.0 };
      for caret in &mut self.positions[first..end] {
        caret.x += shift;
      }
      for piece in self.pieces.iter_mut().filter(|piece| piece.y == y) {
        piece.x += shift;
      }
      first = end;
    }
  }
}

impl LayoutEngine {
  pub(super) fn layout_input_tokens(
    &self,
    glyphs: &mut GlyphEngine,
    state: &TextInputState,
    text: &str,
    base: &TextStyle,
    constraints: Constraints,
    tokens: InputTokens,
  ) -> LayoutResult {
    let line_height = self.token_line_height(base, &tokens);
    let wrap = state.overflow() == TextInputOverflow::Multiline;
    let mut flow = Flow {
      glyphs,
      width: constraints.max_width.max(1.0),
      wrap,
      line_height,
      x: 0.0,
      y: 0.0,
      used_width: 0.0,
      positions: vec![CaretPosition {
        index: 0,
        x: 0.0,
        y: 0.0,
      }],
      pieces: Vec::new(),
    };
    let mut offset = 0;
    for token in tokens.tokens.iter() {
      flow.plain(&text[offset..token.range.start], offset, base);
      let (style, pad, fill) = self.token_style(base, &token.style, flow.width);
      flow.piece(&text[token.range.clone()], token.range.start, style, pad, fill);
      offset = token.range.end;
    }
    flow.plain(&text[offset..], offset, base);
    let height = flow.y + line_height;
    let preferred = Size::new(
      flow.used_width.max(DEFAULT_TEXT_INPUT_WIDTH),
      if wrap {
        multiline_preferred_height(state, height, line_height)
      } else {
        line_height
      },
    );
    let size = constraints.constrain(preferred);
    flow.align_rows(size.width, base.text_align);
    let positions: CaretPositions = flow.positions.into();
    let token_layout = Arc::new(TokenLayout {
      source: tokens.source,
      pieces: flow.pieces,
    });
    let output = TextInputOutput {
      caret_positions: positions,
      size,
      text_width: flow.used_width,
      text_height: height,
      line_height,
      text_band: flow.glyphs.line_content_band(base),
      token_layout: Some(token_layout),
    };
    Self::text_input_result(state, output)
  }

  fn token_line_height(&self, base: &TextStyle, tokens: &InputTokens) -> f32 {
    tokens
      .tokens
      .iter()
      .fold((base.font_size * base.line_height).max(1.0), |height, token| {
        let style = token
          .style
          .typography
          .and_then(|role| self.typography.borrow().try_get(role));
        style.map_or(height, |style| height.max(style.font_size * style.line_height))
      })
  }

  fn token_style(
    &self,
    base: &TextStyle,
    token: &InputTokenStyle,
    width: f32,
  ) -> (TextStyle, f32, Option<(Color, f32)>) {
    let mut style = token
      .typography
      .and_then(|role| self.typography.borrow().try_get(role))
      .unwrap_or_else(|| base.clone());
    style.color = token.foreground.resolve(&self.palette.borrow()).unwrap_or(base.color);
    let pad = token.padding.resolve(&self.spacing.borrow(), width);
    let pad = if pad.is_finite() {
      pad.max(0.0).min(width / 4.0)
    } else {
      0.0
    };
    let radius = token.radius.resolve(&self.radii.borrow());
    let fill = token
      .background
      .resolve(&self.palette.borrow())
      .map(|color| (color, radius));
    (style, pad, fill)
  }
}
