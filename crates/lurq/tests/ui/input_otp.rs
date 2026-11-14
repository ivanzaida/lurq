//! A one-time code: its cells and the separator between groups, what typing, Backspace, the arrows, a paste and a
//! click do, the cell that has the focus with its ring and caret, its sizes, its judgments, a masked and a disabled
//! code, a code in a field, and a theme's rule and renderer.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    events::{ClipboardContent, MouseButton},
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row, Stack},
  core::Signal,
  layout::quad::{Quad, QuadContent},
  node::{Element, color::Color},
  ui::{
    Field, Input, InputOtp, InputOtpKind, InputOtpPart, InputOtpRenderer, InputOtpSize, InputOtpState, InputOtpView,
    Interaction, Themed,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// A stage with `scene` on it, in a theme that holds the caret still, so that it is painted whenever it is asserted.
fn staged<E: Into<Element>>(scene: impl Fn() -> E + Send + Sync + 'static) -> Stage {
  let app = App::new();
  app.theme().set_reduced_motion(Some(true));
  Stage::in_app(app, move |_| Column::new().padding(8.0).child(scene()).into())
}

/// The edges of the cells, from the first: boxes of `size` with a line of 1 around them.
fn cells(stage: &Stage, size: (f32, f32)) -> Vec<Quad> {
  let mut cells: Vec<Quad> = stage
    .quads()
    .into_iter()
    .filter(|quad| {
      (quad.width, quad.height) == size
        && quad
          .border
          .and_then(|border| border.top)
          .is_some_and(|side| side.width == 1.0)
    })
    .collect();
  cells.sort_by(|a, b| a.x.total_cmp(&b.x));
  cells.dedup_by(|a, b| a.x == b.x);
  cells
}

/// The color of the edge of a cell.
fn edge(cell: &Quad) -> Color {
  cell.border.and_then(|border| border.top).expect("an edge").color
}

/// The fill of the box at `cell`.
fn fill(stage: &Stage, cell: &Quad) -> Option<Color> {
  stage.quads().into_iter().find_map(|quad| match quad.content {
    QuadContent::Rect { color, .. }
      if color.a() > 0 && (quad.x, quad.y, quad.width, quad.height) == (cell.x, cell.y, cell.width, cell.height) =>
    {
      Some(color)
    }
    _ => None,
  })
}

/// The characters the cells show, from the first: the painted texts of one character a code holds, or the mask's
/// dot. The input under the cells paints nothing that can be seen.
fn shown(stage: &Stage) -> String {
  let mut texts: Vec<(f32, String)> = stage
    .quads()
    .into_iter()
    .filter_map(|quad| match quad.content {
      QuadContent::Text { text, style, .. }
        if style.color.a() > 0
          && text.chars().count() == 1
          && text
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '\u{2022}') =>
      {
        Some((quad.x, text.to_string()))
      }
      _ => None,
    })
    .collect();
  texts.sort_by(|a, b| a.0.total_cmp(&b.0));
  texts.into_iter().map(|(_, text)| text).collect()
}

/// The color and the size of the text `content` as a cell paints it: not the input's under the cells, which is
/// painted in no color at all.
fn seen(stage: &Stage, content: &str) -> (Color, f32) {
  stage
    .quads()
    .into_iter()
    .find_map(|quad| match quad.content {
      QuadContent::Text { text, style, .. } if text == content && style.color.a() > 0 => {
        Some((style.color, style.font_size))
      }
      _ => None,
    })
    .unwrap_or_else(|| panic!("no text {content:?} is seen"))
}

/// The index of the cell a line of 2 in `color` is drawn inside: the ring of the cell the focus is on.
fn ringed(stage: &Stage, cells: &[Quad], color: Color) -> Option<usize> {
  cells.iter().position(|cell| {
    stage.quads().into_iter().any(|quad| {
      quad
        .border
        .and_then(|border| border.top)
        .map(|side| (side.width, side.color))
        == Some((2.0, color))
        && quad.x >= cell.x
        && quad.y >= cell.y
        && quad.x + quad.width <= cell.x + cell.width
        && quad.y + quad.height <= cell.y + cell.height
    })
  })
}

/// The caret: a line `width` wide and `height` high in the foreground.
fn caret(stage: &Stage, width: f32, height: f32) -> Option<Quad> {
  let foreground = light(PaletteColor::Foreground);
  stage.quads().into_iter().find(|quad| {
    (quad.width, quad.height) == (width, height)
      && matches!(quad.content, QuadContent::Rect { color, .. } if color == foreground)
  })
}

/// The index of the cell the caret is in.
fn caret_in(stage: &Stage, cells: &[Quad]) -> Option<usize> {
  let caret = caret(stage, 1.5, 18.0)?;
  cells
    .iter()
    .position(|cell| caret.x >= cell.x && caret.x + caret.width <= cell.x + cell.width)
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// Types `text` into the focused input, as a key or an input method does.
fn type_in(stage: &mut Stage, text: &str) {
  stage.tree.text_input_replace(0..0, text);
  stage.pass();
}

/// Presses the pointer on the middle of `cell`.
fn click(stage: &mut Stage, cell: &Quad) {
  let (x, y) = (cell.x + cell.width / 2.0, cell.y + cell.height / 2.0);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

const MD: (f32, f32) = (36.0, 40.0);

#[test]
fn a_code_is_six_cells_in_two_groups_with_a_dash_between() {
  let mut stage = staged(|| InputOtp::new(Signal::new("427".to_owned())).groups([3, 3]).id("code"));
  let cells = cells(&stage, MD);
  assert_eq!(cells.len(), 6, "a cell for each character");
  for cell in &cells {
    assert_eq!(edge(cell), light(PaletteColor::Input));
    assert_eq!(fill(&stage, cell), Some(light(PaletteColor::Background)));
    assert!(
      cell.border_radius.is_some_and(|radius| radius.top_left == 6.0),
      "6 round"
    );
  }
  // 6 apart within a group; the dash of 16 with 6 on either side between the groups.
  for pair in [(0, 1), (1, 2), (3, 4), (4, 5)] {
    let gap = cells[pair.1].x - cells[pair.0].x - 36.0;
    assert!((gap - 6.0).abs() < 0.01, "{pair:?}: {gap}");
  }
  let between = cells[3].x - cells[2].x - 36.0;
  assert!((between - 28.0).abs() < 0.01, "6, the dash, 6: {between}");
  assert_eq!(shown(&stage), "427");
  assert_eq!(seen(&stage, "4"), (light(PaletteColor::Foreground), 16.0));
  // Nothing has the focus: no ring, no caret.
  assert_eq!(ringed(&stage, &cells, light(PaletteColor::Ring)), None);
  assert!(caret(&stage, 1.5, 18.0).is_none());
  // Under the cells is one input: the code is its value.
  assert_eq!(stage.bounds("code").3, 40.0);
}

#[test]
fn tab_puts_the_focus_on_the_first_empty_cell_and_typing_fills_it_and_goes_on() {
  let code = Signal::new(String::new());
  let completed = Arc::new(Mutex::new(Vec::new()));
  let (bound, told) = (code.clone(), completed.clone());
  let mut stage = staged(move || {
    let told = told.clone();
    InputOtp::new(bound.clone())
      .id("code")
      .on_complete(move |code| told.lock().unwrap().push(code.to_owned()))
  });
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("code"), "one stop of Tab");
  let at_rest = cells(&stage, MD);
  assert_eq!(ringed(&stage, &at_rest, light(PaletteColor::Ring)), Some(0));
  assert_eq!(caret_in(&stage, &at_rest), Some(0), "the caret in the empty cell");

  type_in(&mut stage, "4");
  assert_eq!(code.get(), "4");
  let now = cells(&stage, MD);
  assert_eq!(
    ringed(&stage, &now, light(PaletteColor::Ring)),
    Some(1),
    "on to the next"
  );
  assert_eq!(caret_in(&stage, &now), Some(1));
  // What is not a digit is not taken.
  type_in(&mut stage, "x");
  assert_eq!(code.get(), "4");
  for digit in ["2", "7", "1", "9"] {
    type_in(&mut stage, digit);
  }
  assert!(completed.lock().unwrap().is_empty(), "not yet");
  type_in(&mut stage, "3");
  assert_eq!(code.get(), "427193");
  assert_eq!(shown(&stage), "427193");
  assert_eq!(
    *completed.lock().unwrap(),
    ["427193"],
    "told once, when the last cell is filled"
  );
  // The focus stays on the last cell, which has its character and no caret.
  let full = cells(&stage, MD);
  assert_eq!(ringed(&stage, &full, light(PaletteColor::Ring)), Some(5));
  assert!(caret(&stage, 1.5, 18.0).is_none());
  // A code is no longer than its cells: typing on the last cell replaces its character.
  type_in(&mut stage, "8");
  assert_eq!(code.get(), "427198");
  assert_eq!(completed.lock().unwrap().len(), 2, "the last cell was filled again");
}

#[test]
fn backspace_empties_the_cell_and_goes_back_and_the_arrows_move() {
  let code = Signal::new(String::new());
  let bound = code.clone();
  let mut stage = staged(move || InputOtp::new(bound.clone()).id("code"));
  stage.key("Tab");
  type_in(&mut stage, "123");
  assert_eq!(code.get(), "123");
  let ring = light(PaletteColor::Ring);
  // On the empty cell after them, Backspace goes back and empties the one before.
  stage.key("Backspace");
  assert_eq!(code.get(), "12");
  assert_eq!(ringed(&stage, &cells(&stage, MD), ring), Some(2));
  // Left moves onto a filled cell; typing there replaces its character.
  stage.key("ArrowLeft");
  assert_eq!(ringed(&stage, &cells(&stage, MD), ring), Some(1));
  type_in(&mut stage, "9");
  assert_eq!(code.get(), "19");
  assert_eq!(ringed(&stage, &cells(&stage, MD), ring), Some(2));
  // Right goes no further than the first empty cell.
  stage.key("ArrowRight");
  stage.key("ArrowRight");
  assert_eq!(ringed(&stage, &cells(&stage, MD), ring), Some(2));
  // Home and End go to the first cell and to the first empty one.
  stage.key("Home");
  assert_eq!(ringed(&stage, &cells(&stage, MD), ring), Some(0));
  // Backspace on a filled cell empties it, and the cells after it move up: a code has no gaps.
  stage.key("Backspace");
  assert_eq!(code.get(), "9");
  assert_eq!(ringed(&stage, &cells(&stage, MD), ring), Some(0));
  stage.key("End");
  assert_eq!(ringed(&stage, &cells(&stage, MD), ring), Some(1));
  stage.key("Backspace");
  stage.key("Backspace");
  assert_eq!(code.get(), "", "nothing before the first cell to empty");
}

#[test]
fn a_paste_of_a_whole_code_fills_every_cell_from_the_first() {
  let code = Signal::new("99".to_owned());
  let completed = Arc::new(Mutex::new(Vec::new()));
  let (bound, told) = (code.clone(), completed.clone());
  let mut stage = staged(move || {
    let told = told.clone();
    InputOtp::new(bound.clone())
      .groups([3, 3])
      .id("code")
      .on_complete(move |code| told.lock().unwrap().push(code.to_owned()))
  });
  stage.key("Tab");
  // Spaces and dashes in it are dropped.
  assert!(stage.tree.paste(ClipboardContent::text(" 123-456 ")));
  stage.pass();
  assert_eq!(code.get(), "123456");
  assert_eq!(shown(&stage), "123456");
  assert_eq!(*completed.lock().unwrap(), ["123456"]);
  // A paste shorter than the code fills the cells from the one that has the focus.
  stage.key("Home");
  stage.key("ArrowRight");
  stage.tree.paste(ClipboardContent::text("78"));
  stage.pass();
  assert_eq!(code.get(), "178456");
  assert_eq!(ringed(&stage, &cells(&stage, MD), light(PaletteColor::Ring)), Some(3));
  // A platform's suggestion of a code that just came, put in as an input method puts text.
  stage.tree.text_input_replace(0..0, "246 810");
  stage.pass();
  assert_eq!(code.get(), "246810");
}

#[test]
fn a_click_puts_the_focus_on_its_cell_or_on_the_first_empty_one() {
  let code = Signal::new("12".to_owned());
  let bound = code.clone();
  let mut stage = staged(move || InputOtp::new(bound.clone()).id("code"));
  let ring = light(PaletteColor::Ring);
  let at_rest = cells(&stage, MD);
  // A cell past the first empty one: the focus goes on the first empty one.
  click(&mut stage, &at_rest[4]);
  assert_eq!(focused(&stage).as_deref(), Some("code"));
  assert_eq!(ringed(&stage, &cells(&stage, MD), ring), Some(2));
  // A filled cell: the focus goes on it, and what is typed replaces its character.
  click(&mut stage, &at_rest[0]);
  assert_eq!(ringed(&stage, &cells(&stage, MD), ring), Some(0));
  type_in(&mut stage, "7");
  assert_eq!(code.get(), "72");
}

#[test]
fn a_code_of_letters_and_digits_takes_both_and_capitals_where_it_says() {
  let code = Signal::new(String::new());
  let bound = code.clone();
  let mut stage = staged(move || {
    InputOtp::new(bound.clone())
      .length(4)
      .kind(InputOtpKind::Alphanumeric)
      .uppercase(true)
      .id("code")
  });
  assert_eq!(cells(&stage, MD).len(), 4);
  stage.key("Tab");
  type_in(&mut stage, "a");
  type_in(&mut stage, "7");
  type_in(&mut stage, "-");
  type_in(&mut stage, "q");
  assert_eq!(code.get(), "A7Q");
  assert_eq!(shown(&stage), "A7Q");
}

#[test]
fn a_masked_code_shows_a_dot_for_each_character() {
  let stage = staged(|| InputOtp::new(Signal::new("4271".to_owned())).mask(true).id("code"));
  assert_eq!(shown(&stage), "\u{2022}\u{2022}\u{2022}\u{2022}");
  assert!(!stage.paints("4"));
}

#[test]
fn the_sizes_are_smaller_and_larger_cells_with_their_type() {
  let stage = staged(|| {
    InputOtp::new(Signal::new("12".to_owned()))
      .length(2)
      .size(InputOtpSize::Sm)
  });
  assert_eq!(cells(&stage, (32.0, 36.0)).len(), 2);
  assert_eq!(seen(&stage, "1").1, 14.0);
  let stage = staged(|| {
    InputOtp::new(Signal::new("12".to_owned()))
      .length(2)
      .size(InputOtpSize::Lg)
  });
  assert_eq!(cells(&stage, (40.0, 44.0)).len(), 2);
  assert_eq!(seen(&stage, "1").1, 18.0);
}

#[test]
fn a_judgment_colors_every_edge_and_the_ring() {
  let code = Signal::new("42".to_owned());
  let bound = code.clone();
  let mut stage = staged(move || InputOtp::new(bound.clone()).invalid(true).id("code"));
  let destructive = light(PaletteColor::Destructive);
  assert!(cells(&stage, MD).iter().all(|cell| edge(cell) == destructive));
  stage.key("Tab");
  assert_eq!(ringed(&stage, &cells(&stage, MD), destructive), Some(2));
  let stage = staged(|| InputOtp::new(Signal::new(String::new())).valid(true));
  assert!(
    cells(&stage, MD)
      .iter()
      .all(|cell| edge(cell) == light(PaletteColor::Success))
  );
}

#[test]
fn a_disabled_code_takes_no_focus_and_no_input() {
  let code = Signal::new("4".to_owned());
  let bound = code.clone();
  let mut stage = staged(move || {
    Column::new()
      .child(InputOtp::new(bound.clone()).disabled(true).id("code"))
      .child(Input::new(Signal::new(String::new())).id("after"))
  });
  let at_rest = cells(&stage, MD);
  assert!(
    at_rest
      .iter()
      .all(|cell| fill(&stage, cell) == Some(light(PaletteColor::Muted)))
  );
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("after"), "Tab passes it by");
  click(&mut stage, &at_rest[1]);
  assert_ne!(focused(&stage).as_deref(), Some("code"));
  assert_eq!(code.get(), "4");
}

#[test]
fn a_read_only_code_moves_and_does_not_change() {
  let code = Signal::new("42".to_owned());
  let bound = code.clone();
  let mut stage = staged(move || InputOtp::new(bound.clone()).read_only(true).id("code"));
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("code"));
  type_in(&mut stage, "7");
  stage.key("Backspace");
  stage.tree.paste(ClipboardContent::text("123456"));
  stage.pass();
  assert_eq!(code.get(), "42");
  stage.key("ArrowLeft");
  assert_eq!(ringed(&stage, &cells(&stage, MD), light(PaletteColor::Ring)), Some(1));
}

#[test]
fn a_field_names_the_code_judges_it_and_its_label_focuses_it() {
  let code = Signal::new(String::new());
  let bound = code.clone();
  let app = App::new();
  app.theme().set_reduced_motion(Some(true));
  let mut stage = Stage::in_app(app, move |_| {
    Column::new()
      .padding(8.0)
      .child(
        Field::new("Verification code", InputOtp::new(bound.clone()).id("code"))
          .error("That code has expired.")
          .id("field"),
      )
      .into()
  });
  let destructive = light(PaletteColor::Destructive);
  assert!(
    cells(&stage, MD).iter().all(|cell| edge(cell) == destructive),
    "the field's error"
  );
  let label = stage.text_quad("Verification code").expect("the label is painted");
  let (x, y) = (label.x + 4.0, label.y + label.height / 2.0);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
  assert_eq!(
    focused(&stage).as_deref(),
    Some("code"),
    "a press of the label focuses the code"
  );
  type_in(&mut stage, "5");
  assert_eq!(code.get(), "5");
}

#[test]
fn a_forced_focus_draws_the_ring_on_the_first_empty_cell() {
  let stage = staged(|| {
    InputOtp::new(Signal::new("42".to_owned()))
      .force(Some(Interaction::FocusVisible))
      .id("code")
  });
  let cells = cells(&stage, MD);
  assert_eq!(ringed(&stage, &cells, light(PaletteColor::Ring)), Some(2));
  assert_eq!(caret_in(&stage, &cells), Some(2));
}

#[test]
fn a_rule_of_a_theme_restyles_the_cells_as_the_mobile_skin_has_them() {
  let app = App::new();
  app.theme().set_recipe::<InputOtp>(
    <InputOtp as Themed>::recipe()
      .base(InputOtpPart::Root, |s| s.gap(8.0))
      .base(InputOtpPart::Cell, |s| s.width(46.0).height(56.0))
      .base(InputOtpPart::Value, |s| s.font_size(22.0))
      .state(InputOtpState::Filled, InputOtpPart::Cell, |s| {
        s.background(PaletteColor::Muted)
      }),
  );
  let mut stage = Stage::in_app(app, |_| {
    Row::new()
      .padding(8.0)
      .child(InputOtp::new(Signal::new("4".to_owned())).length(2).id("code"))
      .into()
  });
  let cells = cells(&stage, (46.0, 56.0));
  assert_eq!(cells.len(), 2);
  assert!((cells[1].x - cells[0].x - 54.0).abs() < 0.01);
  assert_eq!(seen(&stage, "4").1, 22.0);
  assert_eq!(
    fill(&stage, &cells[0]),
    Some(light(PaletteColor::Muted)),
    "a filled cell"
  );
  assert_eq!(fill(&stage, &cells[1]), Some(light(PaletteColor::Background)));
  // The input under the cells takes the room the cells do.
  assert_eq!(stage.bounds("code").2, 46.0 * 2.0 + 8.0);
}

#[test]
fn a_theme_gives_the_code_another_anatomy_and_it_still_takes_what_is_typed() {
  let app = App::new();
  // A code drawn as a row of lines under the characters, with no boxes: other nodes, the same behavior.
  let renderer: InputOtpRenderer = Arc::new(|view: InputOtpView| {
    let mut row = Row::new().spacing(4.0);
    for index in 0..view.length {
      let text = view.shown(index).map(String::from).unwrap_or_else(|| "_".to_owned());
      row = row.child(view.part(InputOtpPart::Value).text(&text));
    }
    Stack::new().child(view.input()).child(row).into()
  });
  app.theme().set_renderer::<InputOtp>(renderer);
  let code = Signal::new(String::new());
  let bound = code.clone();
  let mut stage = Stage::in_app(app, move |_| {
    Row::new()
      .padding(8.0)
      .child(InputOtp::new(bound.clone()).length(4).id("code"))
      .into()
  });
  assert!(cells(&stage, MD).is_empty(), "no boxes");
  stage.key("Tab");
  type_in(&mut stage, "4");
  type_in(&mut stage, "2");
  assert_eq!(code.get(), "42");
  assert_eq!(shown(&stage), "42");
}
