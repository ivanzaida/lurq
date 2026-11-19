//! The roles of a document (the design's Markdown `hMuPL` and Prose `cS7gJ`), the line of each role, and the
//! case a style draws its text in.

use std::borrow::Cow;

use lurq::{
  app::{
    App, Tree,
    theme::{ThemeTypography, TypographyStyle},
  },
  components::{Column, Text},
  layout::{
    Constraints, Size,
    text_style::{FontWeight, TextCase, TextStyle},
  },
};

use crate::support::TestSurface;

#[test]
fn the_headings_of_a_document_are_the_designs() {
  let typography = ThemeTypography::default();
  let shown = |role: TypographyStyle| {
    let style = typography.get(role);
    (
      style.font_size,
      style.weight,
      style.letter_spacing,
      style.line_height,
      style.case,
    )
  };
  assert_eq!(
    shown(TypographyStyle::Heading1),
    (30.0, FontWeight::Bold, -0.6, 1.2, TextCase::AsWritten)
  );
  assert_eq!(
    shown(TypographyStyle::Heading2),
    (24.0, FontWeight::SemiBold, -0.4, 1.2, TextCase::AsWritten)
  );
  assert_eq!(
    shown(TypographyStyle::Heading3),
    (20.0, FontWeight::SemiBold, 0.0, 1.2, TextCase::AsWritten)
  );
  assert_eq!(
    shown(TypographyStyle::Heading4),
    (16.0, FontWeight::SemiBold, 0.0, 1.2, TextCase::AsWritten)
  );
  assert_eq!(
    shown(TypographyStyle::Heading5),
    (14.0, FontWeight::SemiBold, 0.0, 1.2, TextCase::AsWritten)
  );
  assert_eq!(
    shown(TypographyStyle::Heading6),
    (12.0, FontWeight::SemiBold, 0.8, 1.2, TextCase::Upper)
  );
  // Text read at length, in the interface and in a document.
  assert_eq!(
    shown(TypographyStyle::Prose),
    (14.0, FontWeight::Normal, 0.0, 1.6, TextCase::AsWritten)
  );
  assert_eq!(
    shown(TypographyStyle::Article),
    (15.0, FontWeight::Normal, 0.0, 1.65, TextCase::AsWritten)
  );
}

#[test]
fn every_role_has_a_line_of_its_own() {
  let typography = ThemeTypography::default();
  // What the interface draws on one line is on its family's own line, as the design draws it; code on the mono
  // family's, 17 at 13.
  for role in [
    TypographyStyle::Heading,
    TypographyStyle::Title,
    TypographyStyle::Body,
    TypographyStyle::Description,
    TypographyStyle::Caption,
    TypographyStyle::Label,
    TypographyStyle::FieldLabel,
    TypographyStyle::Button,
    TypographyStyle::Link,
  ] {
    assert_eq!(typography.get(role).line_height, 1.2, "{}", role.as_str());
  }
  assert_eq!(typography.get(TypographyStyle::Mono).line_height, 1.3);
  assert_eq!(
    typography.get(TypographyStyle::Mono).font_family,
    typography.mono_family
  );
  // The roles of a document are on the interface's family, and follow it.
  let mut moved = typography.clone();
  moved.set_sans("Inter");
  assert_eq!(&*moved.get(TypographyStyle::Heading1).font_family, "Inter");
  assert_eq!(&*moved.get(TypographyStyle::Article).font_family, "Inter");
  // A person who asked for larger text has larger headings.
  assert_eq!(typography.scaled(2.0).get(TypographyStyle::Heading6).font_size, 24.0);
}

/// The size `text` is laid out at in `style`.
fn laid_out(text: Text) -> (f32, f32) {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(Column::new().child(text.id("t")));
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  tree.pass(&mut app, &TestSurface);
  let bounds = tree
    .get_element_by_id_mut("t")
    .and_then(|text| text.bounds())
    .expect("the text");
  (bounds.width, bounds.height)
}

#[test]
fn a_sixth_heading_is_drawn_in_capitals() {
  let capitals = laid_out(Text::new("Heading 6").variant(TypographyStyle::Heading6));
  let written = laid_out(Text::new("HEADING 6").variant(TypographyStyle::Heading6));
  assert_eq!(capitals, written, "the letters drawn are the capitals");
  let small = laid_out(Text::new("Heading 6").variant(TypographyStyle::Heading5));
  assert!(capitals.0 > 0.0 && small.0 > 0.0);
}

#[test]
fn text_read_at_length_is_on_lines_further_apart() {
  // One line of each: the line of the role.
  let (_, prose) = laid_out(Text::new("A line").variant(TypographyStyle::Prose));
  assert!((prose - 14.0 * 1.6).abs() < 0.5, "{prose}");
  let (_, article) = laid_out(Text::new("A line").variant(TypographyStyle::Article));
  assert!((article - 15.0 * 1.65).abs() < 0.5, "{article}");
  let (_, body) = laid_out(Text::new("A line"));
  assert!((body - 14.0 * 1.2).abs() < 0.5, "{body}");
}

#[test]
fn a_case_keeps_every_offset_of_the_text() {
  assert_eq!(TextCase::Upper.apply("Heading 6"), "HEADING 6");
  assert_eq!(TextCase::Lower.apply("Heading 6"), "heading 6");
  assert!(matches!(TextCase::AsWritten.apply("Heading 6"), Cow::Borrowed(_)));
  assert!(matches!(TextCase::Upper.apply("ALREADY"), Cow::Borrowed(_)));
  // `ß` is two letters in capitals: it stays, so that the text drawn is as long as the text written.
  let street = TextCase::Upper.apply("Straße");
  assert_eq!(street, "STRAßE");
  assert_eq!(street.len(), "Straße".len());
  assert_eq!(TextCase::Upper.apply("привіт"), "ПРИВІТ");
  // A style names its case.
  assert_eq!(TextStyle::default().case, TextCase::AsWritten);
}
