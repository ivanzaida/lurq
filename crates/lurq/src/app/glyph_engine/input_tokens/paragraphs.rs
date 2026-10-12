//! Canonical hard paragraphs retain actual endings and whole-paragraph attrs.
use cosmic_text::{AttrsList, Buffer, Metrics, Shaping};

use super::{GlyphEngine, InputConfig};
use crate::app::glyph_engine::attrs_for_style;

impl GlyphEngine {
  pub(super) fn input_paragraphs(&mut self, config: &InputConfig, presentation: &str) -> Buffer {
    let families: Vec<_> = config
      .styles
      .iter()
      .map(|token| self.resolve_family(&token.text))
      .collect();
    let attrs: Vec<_> = config
      .styles
      .iter()
      .zip(&families)
      .enumerate()
      .map(|(index, (token, family))| {
        let text = &token.text;
        attrs_for_style(text, family, self.face_weight(family, text), text.font_size)
          .metrics(Metrics::new(text.font_size, text.font_size * text.line_height))
          .metadata(index + 1)
      })
      .collect();
    let base = &config.base;
    let family = self.resolve_family(base);
    let defaults = attrs_for_style(base, &family, self.face_weight(&family, base), base.font_size)
      .metrics(Metrics::new(base.font_size, base.font_size * base.line_height));
    // acquire_buffer clears old lines. set_text uses cosmic LineIter, keeps
    // CRLF/LFCR lengths, and supplies exactly one final empty paragraph.
    let mut buffer = self.acquire_buffer(base, f32::MAX, false);
    buffer.set_text(presentation, &defaults, Shaping::Advanced, None);
    let mut offset = 0;
    let mut first = 0;
    for line in &mut buffer.lines {
      let end = offset + line.text().len();
      while first < config.styles.len() && config.styles[first].range.end <= offset {
        first += 1;
      }
      let mut list = AttrsList::new(&defaults);
      for (token, attrs) in config.styles.iter().zip(&attrs).skip(first) {
        let range = &token.range;
        if range.start >= end {
          break;
        }
        let start = range.start.max(offset);
        let stop = range.end.min(end);
        if start < stop {
          list.add_span(start - offset..stop - offset, attrs);
        }
      }
      line.set_attrs_list(list);
      offset = end + line.ending().as_str().len();
    }
    buffer
  }
}
