//! The caret of a text input and the line under what an input method composes are the theme's: how wide and how
//! high the caret is, and how thick the line. The default theme has the design's, the legacy theme and the bare
//! one what lurq 0.62 drew.

use lurq::{
  app::{
    App, Tree,
    events::ImeEvent,
    theme::{CaretHeight, CaretMode, ThemeCaret, ThemeDefinition},
  },
  components::TextInput,
  core::Signal,
  layout::text_style::TextStyle,
  node::{Element, color::Color},
};

use crate::support::render_pass_with_app;

const INPUT: &str = "input";
const CARET: Color = Color::new(255, 0, 255, 255);
const TEXT: Color = Color::new(0, 200, 90, 255);

/// A focused input of type 14 in a line of 21, its caret and its text in colors of their own.
fn focused(app: &mut App) -> Tree {
  let input = TextInput::styled(
    Signal::new("ab".to_owned()),
    TextStyle {
      font_size: 14.0,
      line_height: 1.5,
      color: TEXT.into(),
      ..TextStyle::default()
    },
  )
  .caret_color(CARET)
  .width(240.0)
  .height(32.0);
  let mut tree = Tree::new();
  tree.set_root(Element::from(input).id(INPUT));
  tree.pass_headless(app);
  tree.get_element_by_id_mut(INPUT).expect("the input").focus();
  tree.pass_headless(app);
  tree
}

/// The caret's width and height.
fn caret(app: &mut App) -> (f32, f32) {
  let mut tree = focused(app);
  let snapshot = render_pass_with_app(&mut tree, app);
  let caret = snapshot
    .rects
    .iter()
    .find(|rect| rect.color == CARET)
    .expect("a focused input draws its caret");
  (caret.width, caret.height)
}

/// How thick the lines under a composition are: one for a composition in one row, none where the theme has no
/// line.
fn composition_lines(app: &mut App) -> Vec<f32> {
  let mut tree = focused(app);
  tree.ime(ImeEvent::Preedit {
    text: "nihon".to_owned(),
    cursor: Some((5, 5)),
  });
  tree.pass_headless(app);
  render_pass_with_app(&mut tree, app)
    .rects
    .iter()
    .filter(|rect| rect.color == TEXT && rect.width > 4.0)
    .map(|rect| rect.height)
    .collect()
}

#[test]
fn the_default_theme_draws_the_caret_of_the_design() {
  let mut app = App::new();
  // 1.5 wide and 18 high in type of 14, and a line of 1.5 under a composition.
  let (width, height) = caret(&mut app);
  assert_eq!(width, 1.5);
  assert!((height - 18.0).abs() < 0.01, "the caret is {height} high");
  assert_eq!(composition_lines(&mut app), vec![1.5]);
}

#[test]
fn the_legacy_theme_and_the_bare_one_draw_the_caret_of_0_62() {
  for definition in [ThemeDefinition::legacy(), ThemeDefinition::bare()] {
    let mut app = App::new();
    app.theme().apply(definition);
    // 1 wide and as high as the type is large, and a line of 1.
    let (width, height) = caret(&mut app);
    assert_eq!(width, 1.0);
    assert!((height - 14.0).abs() < 0.01, "the caret is {height} high");
    assert_eq!(composition_lines(&mut app), vec![1.0]);
  }
}

#[test]
fn a_theme_says_how_wide_and_high_its_caret_is_and_how_thick_the_line() {
  let mut app = App::new();
  app.theme().set_caret(
    ThemeCaret::default()
      .with_width(3.0)
      .with_height(CaretHeight::FontSize(1.0))
      .with_composition_line(2.0),
  );
  let (width, height) = caret(&mut app);
  assert_eq!(width, 3.0);
  assert!((height - 14.0).abs() < 0.01, "the caret is {height} high");
  assert_eq!(composition_lines(&mut app), vec![2.0]);

  // As high as the band of the text in its line: more than the type is large, never more than the line.
  app
    .theme()
    .set_caret(ThemeCaret::default().with_height(CaretHeight::TextBand));
  let (_, band) = caret(&mut app);
  assert!(band > 14.0 && band <= 21.0, "the caret is {band} high");
  assert!(
    (band - 18.0).abs() > 0.01,
    "the band is the font's own, not a share of the size"
  );

  // A share that would take it past the box of a single line stops at the box: the line is 21 in a box of 32.
  app
    .theme()
    .set_caret(ThemeCaret::default().with_height(CaretHeight::FontSize(4.0)));
  let (_, capped) = caret(&mut app);
  assert!((capped - 32.0).abs() < 0.01, "the caret is {capped} high");

  // A theme with no line under a composition draws none.
  app.theme().set_caret(ThemeCaret::default().with_composition_line(0.0));
  assert!(composition_lines(&mut app).is_empty());
}

#[test]
fn the_mode_of_the_caret_changes_without_its_size() {
  let mut app = App::new();
  app.theme().apply(ThemeDefinition::legacy());
  app.theme().set_caret_mode(CaretMode::Persistent);
  assert_eq!(caret(&mut app).0, 1.0);
  assert_eq!(
    *app.theme().caret(),
    ThemeCaret::legacy().with_mode(CaretMode::Persistent)
  );

  // The caret a mode alone makes is the design's.
  assert_eq!(ThemeCaret::new(CaretMode::Persistent).width(), 1.5);
}

#[test]
fn a_caret_of_several_lines_stops_at_its_row() {
  // The design's 18 in type of 14 on a line of 1: each row is 14 high, and the caret is no higher than its row.
  let mut app = App::new();
  let input = TextInput::styled(
    Signal::new("ab\ncd".to_owned()),
    TextStyle {
      font_size: 14.0,
      line_height: 1.0,
      color: TEXT.into(),
      ..TextStyle::default()
    },
  )
  .multiline()
  .caret_color(CARET)
  .width(240.0)
  .height(120.0);
  let mut tree = Tree::new();
  tree.set_root(Element::from(input).id(INPUT));
  tree.pass_headless(&mut app);
  tree.get_element_by_id_mut(INPUT).expect("the input").focus();
  tree.pass_headless(&mut app);
  let snapshot = render_pass_with_app(&mut tree, &mut app);
  let caret = snapshot
    .rects
    .iter()
    .find(|rect| rect.color == CARET)
    .expect("a focused input draws its caret");
  assert!((caret.height - 14.0).abs() < 0.01, "the caret is {} high", caret.height);
}
