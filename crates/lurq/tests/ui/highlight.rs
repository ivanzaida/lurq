//! A highlight: its marks and its current one against the board, their
//! words at the board's weight or the text's, a mark over the end of a line,
//! the current mark brought into view as it changes, and the ranges
//! `find_matches` finds under a find bar's options.

use lurq::{
  app::{
    App,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, ScrollVertical},
  core::Signal,
  layout::{
    layout_kind::ScrollState,
    quad::{Quad, QuadContent, RichTextSpan},
    text_style::FontWeight,
  },
  node::color::Color,
  ui::{FindError, FindOptions, Highlight, HighlightPart, find_matches},
};

use super::stage::Stage;

/// A face whose "a" is as wide as its weight says, at 400 and at 500.
const PROBE: &str = "Lurq Weight Probe";
const REGULAR: &[u8] = include_bytes!("../assets/weight_probe/LurqWeightProbe-Regular.ttf");
const MEDIUM: &[u8] = include_bytes!("../assets/weight_probe/LurqWeightProbe-Medium.ttf");

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
}

/// The runs the text is drawn in, where it is drawn in runs.
fn runs(stage: &Stage) -> Option<Vec<RichTextSpan>> {
  stage.quads().into_iter().find_map(|quad| match quad.content {
    QuadContent::RichText { spans, .. } => Some(spans),
    _ => None,
  })
}

/// The left and right of the fills in `color`, in order.
fn spans_of(stage: &Stage, color: Color) -> Vec<(f32, f32)> {
  let mut quads = fills(stage, color);
  quads.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
  quads.iter().map(|quad| (quad.x, quad.x + quad.width)).collect()
}

#[test]
fn a_marks_words_are_the_boards_500_in_the_texts_foreground() {
  let stage = Stage::new(|_| Highlight::new("Results for backdrop blur in docs").mark(12..25).into());
  let runs = runs(&stage).expect("the text in runs");
  let texts: Vec<&str> = runs.iter().map(|run| run.text.as_str()).collect();
  assert_eq!(texts, ["Results for ", "backdrop blur", " in docs"]);
  assert_eq!(runs[1].style.weight, FontWeight::Medium, "the mark's words are 500");
  assert_eq!(runs[0].style.weight, FontWeight::Normal, "the rest is the text's");
  for run in &runs {
    assert_eq!(
      run.style.color,
      light(PaletteColor::Foreground),
      "{:?} in the foreground",
      run.text
    );
    assert_eq!(run.style.font_size, 14.0);
  }
  assert_eq!(fills(&stage, light(PaletteColor::WarningMuted)).len(), 1);
}

#[test]
fn keep_weight_draws_the_text_as_one_run_and_marks_it_as_it_is() {
  // The probe face draws "a" 500 units wide at 400 and 550 at 500: a word of four is 28 or 30.8 at 14.
  let text = "aaaa aaaa aaaa";
  let probe = |keep: bool| {
    let mut app = App::new();
    app.install_fonts([REGULAR.to_vec(), MEDIUM.to_vec()], std::iter::empty::<(&str, &str)>());
    Stage::in_app(app, move |_| {
      Highlight::new(text)
        .mark(5..9)
        .keep_weight(keep)
        .style(HighlightPart::Root, |s| s.font_family(PROBE))
        .into()
    })
  };
  let (heavy, kept) = (probe(false), probe(true));
  assert!(runs(&kept).is_none(), "one text, shaped as without the marks");
  assert_eq!(kept.text(text).1, 14.0);
  assert_eq!(runs(&heavy).expect("in runs")[1].style.weight, FontWeight::Medium);
  // The fill is behind the words as they are laid out: under the heavier word, as wide as it is at 500.
  let muted = light(PaletteColor::WarningMuted);
  let (heavy_fill, kept_fill) = (spans_of(&heavy, muted)[0], spans_of(&kept, muted)[0]);
  let (heavy_width, kept_width) = (heavy_fill.1 - heavy_fill.0, kept_fill.1 - kept_fill.0);
  assert!(
    (heavy_width - kept_width - 2.8).abs() < 1.0,
    "the 500 word is 2.8 wider: {heavy_fill:?} {kept_fill:?}"
  );
  assert!(
    (heavy_fill.0 - kept_fill.0).abs() < 0.5,
    "both start after the same word"
  );
}

#[test]
fn a_heavier_mark_over_the_end_of_a_line_is_filled_on_each_line_it_is_laid_out_on() {
  let text = "one two three four five six seven";
  let at = text.find("three four").unwrap();
  let stage = Stage::new(move |_| {
    Column::new()
      .width(60.0)
      .child(Highlight::new(text).mark(at..at + "three four".len()))
      .into()
  });
  assert!(runs(&stage).is_some(), "in runs, the mark at 500");
  let marks = fills(&stage, light(PaletteColor::WarningMuted));
  assert_eq!(marks.len(), 2, "a fill on each line");
  assert!(marks[1].y > marks[0].y);
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
