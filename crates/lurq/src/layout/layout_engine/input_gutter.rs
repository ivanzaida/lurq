//! The gutter of a multiline `TextInput`: the numbers of its lines in its start padding, at the first row of each
//! line as layout laid it out, and the bands behind the lines picked out, across the input and over every row a
//! wrapped line takes. See [`crate::components::LineGutter`].

use super::*;
use crate::{layout::text_style::VerticalAlign, node::TextTransformMode};

impl LayoutEngine {
  /// Pushes the gutter's bands and numbers, under the text: the bands first, so that a selection and the glyphs
  /// are over them.
  #[allow(clippy::too_many_arguments)]
  pub(super) fn push_text_input_gutter(
    &self,
    node: &Node,
    state: &TextInputState,
    result: &LayoutResult,
    (abs_x, abs_y): (f32, f32),
    transform: Transform2D,
    clip: ClipRect,
    opacity: f32,
    quads: &mut Vec<Quad>,
  ) {
    let Some(gutter) = state.gutter() else {
      return;
    };
    if state.overflow() != TextInputOverflow::Multiline {
      return;
    }
    let padding = self.resolved_padding_for_size(node, result.size);
    let content_height = (result.size.height - padding.top - padding.bottom).max(0.0);
    let top = abs_y + padding.top + text_input_vertical_offset(state, content_height);
    let line_height = state.caret_height().max(1.0);
    let text = state.caret_layout_text();
    // Where each line starts: the byte after each line break.
    let starts: Vec<usize> = std::iter::once(0)
      .chain(text.match_indices('\n').map(|(index, _)| index + 1))
      .collect();
    let table = state.caret_table(usize::MAX);
    // The top of the row a caret at `index` stands on: the lowest of its places, as a caret after a line break
    // stands at the start of the next line.
    let row_of = |index: usize| {
      table
        .iter()
        .filter(|(at, ..)| *at == index)
        .map(|(_, _, y)| *y)
        .reduce(f32::max)
    };
    let end = table.iter().map(|(_, _, y)| *y).reduce(f32::max).unwrap_or(0.0) + line_height;
    let palette = self.palette.borrow();
    let resolve = |color: &crate::node::TextColor| color.resolve(&palette);
    let number_color = gutter
      .number_color
      .as_ref()
      .and_then(resolve)
      .unwrap_or_else(|| palette.text_color(&gutter.style));
    let marked_number = gutter.marked_number.as_ref().and_then(resolve).unwrap_or(number_color);
    let band = gutter.marked_band.as_ref().and_then(resolve);
    let box_clip = intersect_clip(
      clip,
      ClipRect::rect(abs_x, abs_y, result.size.width, result.size.height).transformed(transform),
    );
    let numbers_width = (padding.left - gutter.gap).max(0.0);
    for (line, start) in starts.iter().enumerate() {
      let Some(y) = row_of(*start) else {
        continue;
      };
      let number = gutter.first_line + line;
      let marked = gutter.marked.contains(&number);
      if marked && let Some(band) = band {
        let next = starts.get(line + 1).and_then(|next| row_of(*next)).unwrap_or(end);
        let (x, band_y, band_transform, origin) = transformed_quad_frame(abs_x, top + y, transform);
        quads.push(Quad {
          x,
          y: band_y,
          width: result.size.width,
          height: (next - y).max(line_height),
          opacity,
          transform: band_transform,
          transform_origin: origin,
          content: QuadContent::Rect {
            color: band,
            gradient: None,
          },
          border_radius: None,
          border: None,
          clip: box_clip,
        });
      }
      if gutter.numbers && numbers_width > 0.0 {
        let mut style = gutter.style.clone();
        style.color = if marked { marked_number } else { number_color };
        style.text_align = crate::layout::text_style::TextAlign::Right;
        let (x, number_y, number_transform, origin) = transformed_quad_frame(abs_x, top + y, transform);
        quads.push(Quad {
          x,
          y: number_y,
          width: numbers_width,
          height: line_height,
          opacity,
          transform: number_transform,
          transform_origin: origin,
          content: QuadContent::Text {
            text: number.to_string(),
            style,
            wrap: false,
            vertical_align: VerticalAlign::LineBox,
            transform_mode: TextTransformMode::Bitmap,
          },
          border_radius: None,
          border: None,
          clip: box_clip,
        });
      }
    }
  }
}
