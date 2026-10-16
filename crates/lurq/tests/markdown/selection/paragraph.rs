//! A chat reply's paragraph with inline code in it: the case a person
//! reported, where a drag from "The design marks" to "planned" copied from
//! "ttachments" to the end of the paragraph, the highlight ran past the
//! last glyph of a line, and no selection could be started after it.

use lurq::{app::theme::ThemeMarkdown, components::MarkdownProps};

use super::page;
use crate::support::selection::{assert_drags_copy_their_glyphs, pairs};

/// The reported paragraph, as the reply's Markdown source.
pub(super) const REPORTED: &str = "Nothing behind those buttons is built. In the docs, the `messages` table says \
\"attachments are not built yet\" (`docs/architecture/storage.md:490`). The design marks attachments as not built \
(`docs/design/orchester-workspace.md:1215`) but does list a planned `task.attachments` prompt variable (line 2423).";

/// A theme whose inline code is much wider than the text around it, as a
/// chat's monospace font is next to its sans-serif: laid out in the text's
/// font instead, the code would be narrower and the lines would wrap
/// elsewhere.
fn wide_code() -> ThemeMarkdown {
  let mut theme = ThemeMarkdown::default();
  theme.inline_code.text.font_size_scale = Some(1.5);
  theme
}

#[test]
fn a_drag_over_the_reported_paragraph_copies_the_words_it_covers() {
  let mut page = page(MarkdownProps::new(REPORTED), 560.0);
  assert!(
    page.inks.iter().map(|ink| ink.row).max().unwrap_or(0) >= 3,
    "the paragraph wraps onto several lines"
  );
  let from = page.ink_at("The design marks", 0);
  let (_, to) = page.inks_of("planned", 0);
  page.drag_inks(from, to);
  assert_eq!(
    page.selected().as_deref(),
    Some("The design marks attachments as not built (docs/design/orchester-workspace.md:1215) but does list a planned")
  );
  page.assert_highlight_covers(from, to);
}

#[test]
fn every_drag_over_the_reported_paragraph_copies_the_glyphs_it_covers() {
  let mut page = page(MarkdownProps::new(REPORTED), 560.0);
  let count = page.inks.len();
  assert_drags_copy_their_glyphs(&mut page, pairs(count, 7));
}

#[test]
fn every_drag_over_the_reported_paragraph_in_wide_code_copies_the_glyphs_it_covers() {
  let mut page = page(MarkdownProps::new(REPORTED).theme(wide_code()), 560.0);
  let count = page.inks.len();
  assert_drags_copy_their_glyphs(&mut page, pairs(count, 7));
}

#[test]
fn the_highlight_of_the_reported_paragraph_stays_on_its_glyphs() {
  for theme in [ThemeMarkdown::default(), wide_code()] {
    let mut page = page(MarkdownProps::new(REPORTED).theme(theme), 560.0);
    let count = page.inks.len();
    for (from, to) in pairs(count, 23).chain([(0, count - 1)]) {
      page.drag_inks(from, to);
      page.assert_highlight_covers(from, to);
    }
  }
}

#[test]
fn a_selection_can_be_started_again_after_a_drag_over_the_reported_paragraph() {
  for theme in [ThemeMarkdown::default(), wide_code()] {
    let mut page = page(MarkdownProps::new(REPORTED).theme(theme), 560.0);
    let from = page.ink_at("The design marks", 0);
    let (_, to) = page.inks_of("planned", 0);
    page.drag_inks(from, to);
    let last = page.inks.len() - 1;
    // Past where the first drag ended, to the end of the last line.
    page.drag_inks(to + 3, last);
    assert_eq!(page.selected().as_deref(), Some(&*page.text_between(to + 3, last)));
    // On the last line alone.
    let (line_start, _) = page.inks_of("(line 2423)", 0);
    page.drag_inks(line_start, last);
    assert_eq!(page.selected().as_deref(), Some(&*page.text_between(line_start, last)));
    // Before it, from the start.
    page.drag_inks(0, from);
    assert_eq!(page.selected().as_deref(), Some(&*page.text_between(0, from)));
  }
}
