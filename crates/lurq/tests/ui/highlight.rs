//! A highlight: its marks and its current one against the board, a mark
//! over the end of a line, the current mark brought into view as it changes,
//! and the ranges `find_matches` finds under a find bar's options.

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::{Column, ScrollVertical},
  core::Signal,
  layout::{
    layout_kind::ScrollState,
    quad::{Quad, QuadContent},
  },
  node::color::Color,
  ui::{FindError, FindOptions, Highlight, find_matches},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The fills drawn in `color`, top to bottom.
fn fills(stage: &Stage, color: Color) -> Vec<Quad> {
  let mut quads: Vec<Quad> = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color))
    .collect();
  quads.sort_by(|a, b| a.y.total_cmp(&b.y));
  quads
}

/// The quads with an edge in `color`.
fn edges(stage: &Stage, color: Color) -> usize {
  stage
    .quads()
    .iter()
    .filter(|quad| {
      quad
        .border
        .and_then(|border| border.top)
        .is_some_and(|side| side.color == color && side.width > 0.0)
    })
    .count()
}

#[test]
fn every_mark_is_on_the_muted_warning_fill_and_the_current_one_has_an_edge() {
  let stage = Stage::new(|_| {
    Highlight::new("the cat and the hat and the bat")
      .marks([0..3, 12..15, 24..27])
      .current(Some(1))
      .into()
  });
  let marks = fills(&stage, light(PaletteColor::WarningMuted));
  assert_eq!(marks.len(), 3, "a fill for each mark");
  assert_eq!(
    edges(&stage, light(PaletteColor::Warning)),
    1,
    "the current one alone has an edge"
  );
  // The board's mark: radius 2, and 3 past its words on either side.
  let radius = marks[0].border_radius.expect("round").top_left;
  assert_eq!(radius, 2.0);
  let (color, size) = stage.text("the cat and the hat and the bat");
  assert_eq!((color, size), (light(PaletteColor::Foreground), 14.0));
}

#[test]
fn a_mark_over_the_end_of_a_line_goes_on_at_the_start_of_the_next() {
  let text = "one two three four five six seven";
  let at = text.find("three four").unwrap();
  let stage = Stage::new(move |_| {
    Column::new()
      // "three four" is wider than this: the line breaks between the two words.
      .width(60.0)
      .child(Highlight::new(text).mark(at..at + "three four".len()))
      .into()
  });
  let marks = fills(&stage, light(PaletteColor::WarningMuted));
  assert_eq!(marks.len(), 2, "one fill a line: {}", marks.len());
  assert!(marks[1].y > marks[0].y, "on two lines");
}

#[test]
fn mounted_the_current_mark_is_brought_into_view_as_it_changes() {
  let text: String = (0..30).map(|line| format!("line {line} of the text\n")).collect();
  let found = find_matches(&text, "the", FindOptions::default()).unwrap();
  let current = Signal::new(0usize);
  let state = ScrollState::new();
  let (held, scroller, marks) = (current.clone(), state.clone(), found.clone());
  let mut stage = Stage::new(move |ctx| {
    ScrollVertical::new(
      Highlight::new(&text)
        .marks(marks.clone())
        .current(Some(held.get()))
        .mount(ctx),
    )
    .with_scroll_state(scroller.clone())
    .size(300.0, 60.0)
    .into()
  });
  assert_eq!(state.scroll_y(), 0.0, "shown first, the page stays where it is");
  current.set(25);
  stage.pass();
  stage.pass();
  let scrolled = state.scroll_y();
  assert!(scrolled > 0.0, "the 26th line is brought into view");
  // Back to the first: the view goes back up to it.
  current.set(0);
  stage.pass();
  stage.pass();
  assert!(state.scroll_y() < scrolled, "and back up to the first");
  assert_eq!(found.len(), 30);
}

#[test]
fn find_matches_finds_under_the_options() {
  let text = "The other theme, the THE";
  let plain = FindOptions::default();
  assert_eq!(
    find_matches(text, "the", plain).unwrap(),
    [0..3, 5..8, 10..13, 17..20, 21..24]
  );
  let case = FindOptions {
    match_case: true,
    ..plain
  };
  assert_eq!(find_matches(text, "the", case).unwrap(), [5..8, 10..13, 17..20]);
  let whole = FindOptions {
    whole_word: true,
    ..plain
  };
  assert_eq!(find_matches(text, "the", whole).unwrap(), [0..3, 17..20, 21..24]);
  assert!(
    find_matches(text, "", plain).unwrap().is_empty(),
    "an empty query finds nothing"
  );
  // Letters of another script match whatever their case, at their own bytes.
  assert_eq!(find_matches("Ünïcode ünïcode", "ÜNÏ", plain).unwrap(), [0..5, 10..15]);
}

#[test]
fn a_regular_expression_is_found_where_lurq_has_its_feature() {
  let regex = FindOptions {
    regex: true,
    ..FindOptions::default()
  };
  let found = find_matches("tame tile toe", r"t\w+e", regex);
  #[cfg(feature = "regex")]
  {
    assert_eq!(found.unwrap(), [0..4, 5..9, 10..13]);
    assert!(matches!(
      find_matches("x", "(", regex),
      Err(FindError::InvalidPattern(_))
    ));
  }
  #[cfg(not(feature = "regex"))]
  assert_eq!(found, Err(FindError::RegexUnavailable));
}
