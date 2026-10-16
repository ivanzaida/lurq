//! A paragraph wrapped at fractional widths and display scales: the paint
//! wraps at the box's width rounded up in device pixels, `ceil(width *
//! scale)`, which is not the logical width at a fractional width or scale.
//! Every row must copy exactly the glyphs drawn on it, so a selection takes
//! its carets from the painted layout.

use lurq::components::MarkdownProps;

use super::{paragraph::REPORTED, scaled};
use crate::support::selection::rows_copying_other_text;

/// The widths swept at every scale: fractional, 6.37 px apart, from 300 to
/// 460 logical pixels.
fn fractional_widths() -> impl Iterator<Item = f32> {
  (0..26).map(|step| 300.17 + step as f32 * 6.37)
}

#[test]
fn every_row_of_the_reported_paragraph_copies_itself_at_fractional_widths_and_scales() {
  let mut failures = Vec::new();
  let mut pages = 0;
  for scale in [1.0_f32, 1.25, 1.5] {
    for width in fractional_widths() {
      let mut page = scaled(MarkdownProps::new(REPORTED), width, scale);
      assert!(
        page.inks.iter().map(|ink| ink.row).max().unwrap_or(0) >= 3,
        "the paragraph wraps onto several lines at {width} ({scale}x)"
      );
      pages += 1;
      failures.extend(rows_copying_other_text(
        &mut page,
        &format!("width {width} scale {scale}"),
      ));
    }
  }
  assert!(
    failures.is_empty(),
    "{} rows over {pages} pages copied other text:\n{}",
    failures.len(),
    failures.join("\n")
  );
}

#[test]
fn the_highlight_stays_on_the_glyphs_at_fractional_widths_and_scales() {
  for scale in [1.0_f32, 1.25, 1.5] {
    for width in fractional_widths().step_by(5) {
      let mut page = scaled(MarkdownProps::new(REPORTED), width, scale);
      let count = page.inks.len();
      for (from, to) in [(0, count - 1), (count / 3, 2 * count / 3), (count / 2, count / 2)] {
        page.drag_inks(from, to);
        page.assert_highlight_covers(from, to);
      }
    }
  }
}
