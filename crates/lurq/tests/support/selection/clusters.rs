//! Which characters each drawn glyph of a text stands for. A font may draw
//! several characters as one glyph, a ligature (DejaVu Sans, the fallback
//! font on Linux, draws "fi" as one), or one character as several glyphs.
//! A page reads the glyphs of its texts in reading order and needs to know
//! how many characters each one covers.
//!
//! The answer comes from drawing, not from the selection under test: a text
//! is drawn alone in a probe in the page's fonts, and where it draws other
//! than one glyph per character, each word of it is drawn again one
//! character longer at a time. A character that adds no glyph joins the
//! glyph before it; one that adds several is drawn as all of them. Shaping
//! never joins characters across whitespace, so words are drawn on their own.

use std::{collections::HashMap, ops::Range};

use lurq::{
  app::{App, Tree},
  components::Text,
};

use crate::support::render_pass_with_app;

/// Characters of a text drawn together, and how many glyphs draw them.
#[derive(Clone, Debug)]
pub struct Cluster {
  /// The characters, as a byte range of the text.
  pub bytes: Range<usize>,
  pub glyphs: usize,
}

/// Probed glyph counts, by the text drawn.
#[derive(Default)]
pub struct Probes(HashMap<String, usize>);

impl Probes {
  /// How many glyphs `text` draws alone, in the default text style.
  pub fn glyphs(&mut self, app: &mut App, text: &str) -> usize {
    if let Some(&count) = self.0.get(text) {
      return count;
    }
    let mut tree = Tree::new();
    tree.resize(4000, 400);
    tree.set_root(Text::new(text).nowrap());
    let count = render_pass_with_app(&mut tree, app)
      .glyphs
      .iter()
      .filter(|glyph| glyph.width > 0.0)
      .count();
    self.0.insert(text.to_owned(), count);
    count
  }

  /// The clusters of the non-whitespace characters of `text`, in text
  /// order: one per character when the text draws a glyph for each.
  pub fn clusters(&mut self, app: &mut App, text: &str) -> Vec<Cluster> {
    let chars: Vec<Range<usize>> = text
      .char_indices()
      .filter(|(_, ch)| !ch.is_whitespace())
      .map(|(at, ch)| at..at + ch.len_utf8())
      .collect();
    if self.glyphs(app, text) == chars.len() {
      return chars.into_iter().map(|bytes| Cluster { bytes, glyphs: 1 }).collect();
    }
    words(text)
      .flat_map(|word| self.word_clusters(app, text, word))
      .collect()
  }

  /// The clusters of the word at `word` in `text`, from the glyphs each of
  /// its prefixes draws.
  fn word_clusters(&mut self, app: &mut App, text: &str, word: Range<usize>) -> Vec<Cluster> {
    let mut clusters: Vec<Cluster> = Vec::new();
    let mut drawn = 0;
    for (at, ch) in text[word.clone()].char_indices() {
      let end = word.start + at + ch.len_utf8();
      let count = self.glyphs(app, &text[word.start..end]);
      assert!(
        count >= drawn,
        "{:?} draws fewer glyphs than {:?}",
        &text[word.start..end],
        &text[word.start..word.start + at]
      );
      match clusters.last_mut() {
        Some(last) if count == drawn => last.bytes.end = end,
        _ => clusters.push(Cluster {
          bytes: word.start + at..end,
          glyphs: count - drawn,
        }),
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
