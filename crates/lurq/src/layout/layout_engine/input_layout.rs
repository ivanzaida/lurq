use super::*;

impl LayoutEngine {
  pub(super) fn layout_text_input(
    &self,
    glyph_engine: &mut GlyphEngine,
    state: &crate::node::node_kind::TextInputState,
    text: &str,
    style: &TextStyle,
    placeholder_style: Option<&TextStyle>,
    constraints: Constraints,
  ) -> LayoutResult {
    if let Some(tokens) = state.tokens().filter(|tokens| !tokens.tokens.is_empty()) {
      return self.layout_input_tokens(glyph_engine, state, style, constraints, tokens);
    }
    let display_style = text_input_display_style(state, style, placeholder_style);
    let overflow = state.overflow();
    let text_constraints = Constraints {
      min_width: 0.0,
      min_height: 0.0,
      ..constraints
    };
    let text_result = self.layout_text(
      glyph_engine,
      text,
      display_style,
      text_constraints,
      overflow == crate::node::node_kind::TextInputOverflow::Multiline,
    );
    let line_height = (style.font_size * style.line_height).max(1.0);
    let text_height = text_result
      .size
      .height
      .max(explicit_multiline_height(text, line_height));
    let preferred = match overflow {
      crate::node::node_kind::TextInputOverflow::Multiline => {
        let preferred_height = multiline_preferred_height(state, text_height, line_height);
        Size::new(text_result.size.width.max(DEFAULT_TEXT_INPUT_WIDTH), preferred_height)
      }
      crate::node::node_kind::TextInputOverflow::Scroll => Size::new(DEFAULT_TEXT_INPUT_WIDTH, line_height),
    };
    let size = constraints.constrain(preferred);
    // Caret positions must wrap exactly like the rendered text, otherwise
    // soft-wrapped rows collapse onto the first row's y and pointer hit-testing
    // can never reach them. Single-line (Scroll) inputs never wrap, but aligned
    // text still needs the finite content width so caret/selection geometry
    // follows centered or right-aligned glyphs.
    let wraps = overflow == crate::node::node_kind::TextInputOverflow::Multiline;
    let caret_width = if wraps || display_style.text_align != crate::layout::text_style::TextAlign::Left {
      size.width
    } else {
      f32::MAX
    };
    let caret_source = state.caret_layout_text();
    let mut caret_positions = glyph_engine.caret_positions(&caret_source, style, caret_width, wraps);
    state.remap_caret_positions(&mut caret_positions);
    let output = TextInputOutput {
      token_layout: None,
      caret_positions,
      size,
      text_width: text_result.size.width,
      text_height,
      line_height,
      text_band: glyph_engine.line_content_band(style),
    };
    Self::text_input_result(state, output)
  }
}
