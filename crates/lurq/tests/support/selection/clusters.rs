//! Which characters each drawn glyph of a text stands for. A font may draw
//! several characters as one glyph, a ligature (DejaVu Sans, the fallback
//! font on Linux, draws "fi" as one), or one character as several glyphs.
//! A page reads the glyphs of its texts in reading order and needs to know
//! how many characters each one covers.
//!
//! The answer comes from drawing, not from the selection under test. Every
//! stretch of a text is drawn alone in a probe, in the style the tree paints
//! it in (`Tree::painted_quads`: a `Text`'s style, a rich text span's), at the
//! page's scale. Where a stretch draws other than one glyph per character,
//! each of its words is drawn again one character longer at a time: a
//! character that adds no glyph is drawn with the glyph before it, and one
//! that adds several is drawn as all of them. Text layout shapes a word
//! span by span and never across whitespace, so a stretch of one span drawn
//! alone is shaped as the page shapes it.

use std::{collections::HashMap, ops::Range};

use lurq::{
  app::{App, Tree},
  components::Text,
  layout::{quad::QuadContent, text_style::TextStyle},
};

use super::super::render_pass_with_app;

/// Characters of a text drawn together, and how many glyphs draw them.
#[derive(Clone, Debug)]
pub struct Cluster {
  /// The characters, as a byte range of the text.
  pub bytes: Range<usize>,
  pub glyphs: usize,
}

/// Characters of a text painted in one style, by one text run or span: from
/// its first non-whitespace character to its last.
pub struct Stretch {
  pub bytes: Range<usize>,
  pub style: TextStyle,
}

/// The characters a tree paints, in paint order, each with the style of the
/// text run or span that paints it.
pub struct Painted {
  chars: Vec<(char, usize)>,
  styles: Vec<TextStyle>,
  /// Where the next text is looked for: after the last one read.
  cursor: usize,
}

impl Painted {
  /// What `tree` painted in its last pass.
  pub fn new(tree: &Tree) -> Self {
    let mut painted = Self {
      chars: Vec::new(),
      styles: Vec::new(),
      cursor: 0,
    };
    for quad in tree.painted_quads() {
      match quad.content {
        QuadContent::Text { text, style, .. } => painted.push(&text, style),
        QuadContent::RichText { spans, .. } => {
          for span in spans {
            painted.push(&span.text, span.style);
          }
        }
        _ => {}
      }
    }
    painted
  }

  fn push(&mut self, text: &str, style: TextStyle) {
    let run = self.styles.len();
    self.styles.push(style);
    self.chars.extend(text.chars().map(|ch| (ch, run)));
  }

  /// The stretches of `text`, the next text the tree paints: its
  /// non-whitespace characters are found in paint order after the texts
  /// read before it. Painted characters that are not `text`'s (a link's
  /// trailing runs, isolate marks) are passed over.
  pub fn stretches(&mut self, text: &str) -> Vec<Stretch> {
    let mut cursor = self.cursor;
    let stretches = self.stretches_from(&mut cursor, text);
    self.cursor = cursor;
    stretches
  }

  /// The stretches of `text` where the tree paints it: the first place its
  /// non-whitespace characters follow each other with nothing between them
  /// but whitespace and invisible marks.
  pub fn find(&self, text: &str) -> Vec<Stretch> {
    let wanted: Vec<char> = text.chars().filter(|ch| !ch.is_whitespace()).collect();
    let mut start = (0..self.chars.len())
      .find(|&start| self.follows(start, &wanted))
      .unwrap_or_else(|| panic!("{text:?} is painted"));
    self.stretches_from(&mut start, text)
  }

  /// Whether the painted characters from `start` on are `wanted`, with
  /// nothing but whitespace and invisible marks between them.
  fn follows(&self, start: usize, wanted: &[char]) -> bool {
    let mut painted = self.chars[start..].iter().map(|&(ch, _)| ch);
    wanted.iter().all(|&ch| {
      painted
        .by_ref()
        .find(|&next| next == ch || !(next.is_whitespace() || is_invisible(next)))
        == Some(ch)
    })
  }

  fn stretches_from(&self, cursor: &mut usize, text: &str) -> Vec<Stretch> {
    let mut stretches: Vec<(Range<usize>, usize)> = Vec::new();
    for (at, ch) in text.char_indices().filter(|(_, ch)| !ch.is_whitespace()) {
      let found = self.chars[*cursor..]
        .iter()
        .position(|&(painted, _)| painted == ch)
        .unwrap_or_else(|| panic!("{ch:?} at byte {at} of {text:?} is painted after the texts before it"));
      let (_, run) = self.chars[*cursor + found];
      *cursor += found + 1;
      let end = at + ch.len_utf8();
      match stretches.last_mut() {
        Some((bytes, last)) if *last == run => bytes.end = end,
        _ => stretches.push((at..end, run)),
      }
    }
    stretches
      .into_iter()
      .map(|(bytes, run)| Stretch {
        bytes,
        style: self.styles[run].clone(),
      })
      .collect()
  }
}

/// A mark that draws nothing: a bidi isolate or embedding, a zero-width
/// space or joiner, a byte order mark.
fn is_invisible(ch: char) -> bool {
  matches!(
    ch,
    '\u{200B}'..='\u{200F}' | '\u{2060}' | '\u{2066}'..='\u{2069}' | '\u{202A}'..='\u{202E}' | '\u{061C}' | '\u{FEFF}'
  )
}

/// Probed glyph counts, by the text drawn and its style.
pub struct Probes {
  scale: f32,
  counts: HashMap<String, Vec<(TextStyle, usize)>>,
}

impl Probes {
  /// Probes drawn at the display scale `scale`, as the page is.
  pub fn new(scale: f32) -> Self {
    Self {
      scale,
      counts: HashMap::new(),
    }
  }

  /// How many glyphs `text` draws alone in `style`.
  pub fn glyphs(&mut self, app: &mut App, text: &str, style: &TextStyle) -> usize {
    let known = self.counts.entry(text.to_owned()).or_default();
    if let Some((_, count)) = known.iter().find(|(known, _)| known == style) {
      return *count;
    }
    let mut tree = Tree::new();
    tree.set_scale_factor(self.scale);
    tree.resize((4000.0 * self.scale) as u32, (400.0 * self.scale) as u32);
    tree.set_root(Text::styled(text, style.clone()).nowrap());
    let count = render_pass_with_app(&mut tree, app)
      .glyphs
      .iter()
      .filter(|glyph| glyph.width > 0.0)
      .count();
    known.push((style.clone(), count));
    count
  }

  /// The clusters of the non-whitespace characters of `text`, painted in
  /// `stretches`, in text order: one per character where a stretch draws a
  /// glyph for each.
  pub fn clusters(&mut self, app: &mut App, text: &str, stretches: &[Stretch]) -> Vec<Cluster> {
    let mut clusters = Vec::new();
    for stretch in stretches {
      let drawn = &text[stretch.bytes.clone()];
      let chars: Vec<Range<usize>> = drawn
        .char_indices()
        .filter(|(_, ch)| !ch.is_whitespace())
        .map(|(at, ch)| stretch.bytes.start + at..stretch.bytes.start + at + ch.len_utf8())
        .collect();
      if self.glyphs(app, drawn, &stretch.style) == chars.len() {
        clusters.extend(chars.into_iter().map(|bytes| Cluster { bytes, glyphs: 1 }));
        continue;
      }
      for word in words(drawn) {
        let word = stretch.bytes.start + word.start..stretch.bytes.start + word.end;
        clusters.extend(self.word_clusters(app, text, word, &stretch.style));
      }
    }
    clusters
  }

  /// The clusters of the word at `word` in `text`, from the glyphs each of
  /// its prefixes draws in `style`.
  fn word_clusters(&mut self, app: &mut App, text: &str, word: Range<usize>, style: &TextStyle) -> Vec<Cluster> {
    let mut clusters: Vec<Cluster> = Vec::new();
    let mut drawn = 0;
    for (at, ch) in text[word.clone()].char_indices() {
      let start = word.start + at;
      let end = start + ch.len_utf8();
      let count = self.glyphs(app, &text[word.start..end], style);
      assert!(
        count >= drawn,
        "{:?} draws fewer glyphs than {:?}",
        &text[word.start..end],
        &text[word.start..start]
      );
      if count > drawn {
        clusters.push(Cluster {
          bytes: start..end,
          glyphs: count - drawn,
        });
      } else {
        // No glyph of its own: drawn with the glyph before it (a ligature),
        // unless it draws nothing at all.
        assert!(
          self.glyphs(app, &text[start..end], style) > 0,
          "{ch:?} in {:?} draws no glyph, so no drawn glyph stands for it",
          &text[word.clone()]
        );
        let last = clusters
          .last_mut()
          .unwrap_or_else(|| panic!("{ch:?} starts {:?} but adds no glyph to it", &text[word.clone()]));
        last.bytes.end = end;
      }
      drawn = count;
    }
    clusters
  }
}

/// The byte ranges of the whitespace-separated words of `text`.
fn words(text: &str) -> impl Iterator<Item = Range<usize>> + '_ {
  text.split_whitespace().map(move |word| {
    let start = word.as_ptr() as usize - text.as_ptr() as usize;
    start..start + word.len()
  })
}
