//! A multi select: its box with the chosen values as tags, the list of checkboxes that stays open while one
//! chooses, the keys, taking values away by a tag's cross, by Backspace and by the clear control, the count past a
//! maximum, tags going on to the next line, and a field around it.

use lurq::{
  app::{
    events::MouseButton,
    theme::{PaletteColor, ThemePalette},
  },
  components::Column,
  core::Signal,
  layout::quad::{Quad, QuadContent},
  node::color::Color,
  ui::{ComboboxSize, Field, MultiSelect, icons},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The languages the multi selects here choose from.
fn languages() -> Vec<(String, &'static str)> {
  vec![
    ("rust".to_owned(), "Rust"),
    ("wgsl".to_owned(), "WGSL"),
    ("typescript".to_owned(), "TypeScript"),
    ("swift".to_owned(), "Swift"),
  ]
}

fn chosen(values: &[&str]) -> Signal<Vec<String>> {
  Signal::new(values.iter().map(|value| (*value).to_owned()).collect())
}

/// The page as a window holds it: as large as the stage's window, so that the open list is painted.
fn window() -> Column {
  Column::new().size(400.0, 300.0)
}

fn page(value: &Signal<Vec<String>>, dress: impl Fn(MultiSelect) -> MultiSelect + Send + Sync + 'static) -> Stage {
  let value = value.clone();
  Stage::new(move |_| {
    window()
      .padding(8.0)
      .child(dress(
        MultiSelect::new(value.clone(), languages())
          .placeholder("Select languages")
          .id("languages"),
      ))
      .into()
  })
}

fn is_open(stage: &mut Stage) -> bool {
  stage
    .tree
    .get_element_by_id_mut("languages")
    .and_then(|element| element.as_select())
    .is_some_and(|select| select.is_open())
}

fn typed(stage: &mut Stage) -> String {
  stage
    .tree
    .get_element_by_id_mut("languages-input")
    .and_then(|element| element.as_text_input())
    .map(|input| input.value())
    .expect("the multi select's input")
}

fn type_in(stage: &mut Stage, text: &str) {
  for ch in text.chars() {
    stage.key(&ch.to_string());
  }
}

fn values(signal: &Signal<Vec<String>>) -> Vec<String> {
  signal.get()
}

fn glyph(stage: &Stage, name: &str) -> String {
  stage
    .app
    .theme()
    .icon(name)
    .expect("the theme has the icon")
    .glyph
    .to_string()
}

/// The quads of the text `content`.
fn texts(stage: &Stage, content: &str) -> Vec<Quad> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == content))
    .collect()
}

/// The middle of the text `content` as painted last: in the open list, over the page.
fn middle_of(stage: &Stage, content: &str) -> (f32, f32) {
  let quad = texts(stage, content)
    .pop()
    .unwrap_or_else(|| panic!("no text {content:?} was painted"));
  (quad.x + quad.width / 2.0, quad.y + quad.height / 2.0)
}

/// Whether `content` is painted under the box: in the open list.
fn listed(stage: &mut Stage, content: &str) -> bool {
  let (_, y, _, height) = stage.bounds("languages");
  texts(stage, content).iter().any(|quad| quad.y > y + height)
}

/// The filled rectangles of `color`, `side` square.
fn boxes(stage: &Stage, color: Color, side: f32) -> Vec<Quad> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color)
        && (quad.width - side).abs() < 0.01
        && (quad.height - side).abs() < 0.01
    })
    .collect()
}

fn press_at(stage: &mut Stage, (x, y): (f32, f32)) {
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

/// A press in the box past its tags: on the input, which takes the focus and opens the list.
fn press_input(stage: &mut Stage) {
  let input = stage.center("languages-input");
  press_at(stage, input);
}

#[test]
fn the_chosen_values_are_tags_in_a_box_of_280() {
  let value = chosen(&["rust", "wgsl"]);
  let mut stage = page(&value, |select| select);
  let (x, y, width, height) = stage.bounds("languages");
  assert_eq!((width, height), (280.0, 36.0));
  assert_eq!(stage.fill_color("languages"), Some(light(PaletteColor::Background)));
  // A tag is 24 high on the fill of what is secondary, its text 13 and 8 in from its start, which is 6 in.
  let rust = stage.text_quad("Rust").expect("the tag of Rust");
  assert_eq!(stage.text("Rust"), (light(PaletteColor::SecondaryForeground), 13.0));
  let fill = stage
    .quads()
    .into_iter()
    .find(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Secondary))
        && quad.x <= rust.x
        && quad.x + quad.width >= rust.x + rust.width
    })
    .expect("the tag's fill");
  assert_eq!(fill.height, 24.0);
  assert!((fill.x - x - 6.0).abs() < 1.0, "6 in: {}", fill.x - x);
  assert!(
    (rust.x - fill.x - 8.0).abs() < 1.0,
    "its text 8 in: {}",
    rust.x - fill.x
  );
  assert!((fill.y - y - 6.0).abs() < 1.0, "in the middle of the box");
  assert!(stage.paints("WGSL"));
  // Beside tags the placeholder is not said again; the chevron is at the end, 10 from it.
  assert!(!stage.paints("Select languages"));
  let chevron = glyph(&stage, icons::CHEVRON_DOWN);
  let quad = stage.text_quad(&chevron).expect("the chevron");
  assert!(
    (quad.x + 16.0 - (x + width - 10.0)).abs() < 1.0,
    "10 from the end: {}",
    x + width - quad.x - 16.0
  );
  // With nothing chosen, the placeholder is.
  let none = chosen(&[]);
  let stage = page(&none, |select| select);
  assert_eq!(
    stage.text("Select languages"),
    (light(PaletteColor::MutedForeground), 14.0)
  );
}

#[test]
fn the_list_has_a_box_before_each_option_and_stays_open_while_one_chooses() {
  let value = chosen(&["rust", "wgsl"]);
  let mut stage = page(&value, |select| select);
  press_input(&mut stage);
  assert!(is_open(&mut stage));
  for label in ["Rust", "WGSL", "TypeScript", "Swift"] {
    assert!(listed(&mut stage, label), "{label}");
  }
  // Two boxes of 16 on the primary fill, the chosen ones, and two on the background's in an input's edge.
  assert_eq!(boxes(&stage, light(PaletteColor::Primary), 16.0).len(), 2);
  let empty = boxes(&stage, light(PaletteColor::Background), 16.0);
  assert_eq!(empty.len(), 2);
  assert!(
    empty
      .iter()
      .all(|quad| quad.border.and_then(|border| border.top).map(|side| side.color) == Some(light(PaletteColor::Input)))
  );
  // The box is 8 before the label.
  let typescript = texts(&stage, "TypeScript").pop().unwrap();
  assert!(
    empty
      .iter()
      .any(|quad| (typescript.x - (quad.x + 16.0) - 8.0).abs() < 1.0),
    "8 before the label"
  );
  let point = middle_of(&stage, "TypeScript");
  press_at(&mut stage, point);
  assert_eq!(values(&value), ["rust", "wgsl", "typescript"]);
  assert!(is_open(&mut stage), "the list stays open");
  assert_eq!(boxes(&stage, light(PaletteColor::Primary), 16.0).len(), 3);
  // A press of a chosen one takes it back.
  let point = middle_of(&stage, "Rust");
  press_at(&mut stage, point);
  assert_eq!(values(&value), ["wgsl", "typescript"]);
}

#[test]
fn typing_filters_enter_chooses_and_backspace_takes_the_last_away() {
  let value = chosen(&["rust"]);
  let mut stage = page(&value, |select| select);
  press_input(&mut stage);
  stage.key("Escape");
  assert!(!is_open(&mut stage));
  type_in(&mut stage, "sw");
  assert!(is_open(&mut stage));
  assert!(listed(&mut stage, "Swift") && !listed(&mut stage, "Rust"));
  stage.key("Enter");
  assert_eq!(values(&value), ["rust", "swift"]);
  assert!(is_open(&mut stage), "the list stays open");
  assert_eq!(typed(&mut stage), "sw", "what was typed stays, to choose another");
  // Backspace edits what is typed while there is any; in an empty input it takes the last value away.
  stage.key("Backspace");
  stage.key("Backspace");
  assert_eq!(typed(&mut stage), "");
  assert_eq!(values(&value), ["rust", "swift"]);
  stage.key("Backspace");
  assert_eq!(values(&value), ["rust"]);
  stage.key("Backspace");
  assert!(values(&value).is_empty());
  // Closed, Down opens the list on the first option; Down again moves to the next.
  stage.key("Escape");
  assert!(!is_open(&mut stage));
  stage.key("ArrowDown");
  stage.key("ArrowDown");
  stage.key("Enter");
  assert_eq!(values(&value), ["wgsl"]);
}

#[test]
fn a_tags_cross_takes_its_value_away_and_leaving_clears_what_was_typed() {
  let value = chosen(&["rust", "wgsl"]);
  let mut stage = page(&value, |select| select);
  let close = glyph(&stage, icons::CLOSE);
  let wgsl = stage.text_quad("WGSL").unwrap();
  let cross = texts(&stage, &close)
    .into_iter()
    .find(|quad| quad.x > wgsl.x + wgsl.width)
    .expect("the cross of the tag of WGSL");
  press_at(&mut stage, (cross.x + cross.width / 2.0, cross.y + cross.height / 2.0));
  assert_eq!(values(&value), ["rust"]);
  assert!(!is_open(&mut stage), "a press of a cross does not open the list");
  press_input(&mut stage);
  type_in(&mut stage, "ty");
  assert_eq!(typed(&mut stage), "ty");
  press_at(&mut stage, (390.0, 290.0));
  assert!(!is_open(&mut stage));
  assert_eq!(typed(&mut stage), "", "leaving clears what was typed");
}

#[test]
fn past_the_most_shown_the_rest_are_counted_and_the_box_grows_with_its_tags() {
  let value = chosen(&["rust", "wgsl", "typescript", "swift"]);
  let mut stage = page(&value, |select| select.max_shown(2));
  assert!(stage.paints("Rust") && stage.paints("WGSL"));
  assert!(!stage.paints("TypeScript"));
  assert!(stage.paints("+2"));
  assert_eq!(stage.bounds("languages").3, 36.0, "on one line");
  // Every tag shown in a box of 200: they go on to the next line, and the box grows.
  let all = chosen(&["rust", "wgsl", "typescript", "swift"]);
  let mut stage = page(&all, |select| {
    select.style(lurq::ui::MultiSelectPart::Trigger, |s| s.width(200.0))
  });
  let (_, y, _, height) = stage.bounds("languages");
  assert!(height > 36.0, "it grows: {height}");
  let (rust, swift) = (stage.text_quad("Rust").unwrap(), stage.text_quad("Swift").unwrap());
  assert!(swift.y > rust.y + 20.0, "Swift is on a line below Rust");
  assert!(swift.y + swift.height < y + height, "inside the box");
}

#[test]
fn the_clear_control_takes_every_value_away() {
  let value = chosen(&["rust", "wgsl"]);
  let mut stage = page(&value, |select| select.clearable(true));
  let close = glyph(&stage, icons::CLOSE);
  let (x, _, width, _) = stage.bounds("languages");
  // The last cross before the chevron is the clear control's.
  let clear = texts(&stage, &close)
    .into_iter()
    .max_by(|a, b| a.x.total_cmp(&b.x))
    .expect("the clear control");
  assert!(clear.x > x + width - 50.0, "at the end of the box");
  press_at(&mut stage, (clear.x + clear.width / 2.0, clear.y + clear.height / 2.0));
  assert!(values(&value).is_empty());
  assert_eq!(texts(&stage, &close).len(), 0, "no tags and no clear control");
}

#[test]
fn sizes_set_the_least_height_and_the_tags() {
  for (size, height) in [
    (ComboboxSize::Sm, 32.0),
    (ComboboxSize::Md, 36.0),
    (ComboboxSize::Lg, 40.0),
  ] {
    let value = chosen(&["rust"]);
    let mut stage = page(&value, move |select| select.size(size));
    assert_eq!(stage.bounds("languages").3, height, "{size:?}");
  }
}

#[test]
fn a_disabled_multi_select_takes_nothing() {
  let value = chosen(&["rust"]);
  let mut stage = page(&value, |select| select.disabled(true));
  assert_eq!(stage.fill_color("languages"), Some(light(PaletteColor::Muted)));
  press_input(&mut stage);
  assert!(!is_open(&mut stage));
  stage.key("Tab");
  stage.key("Backspace");
  assert_eq!(values(&value), ["rust"]);
}

#[test]
fn in_a_field_it_is_judged_and_chooses() {
  let value = chosen(&[]);
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    window()
      .padding(8.0)
      .child(
        Field::new(
          "Languages",
          MultiSelect::new(bound.clone(), languages()).id("languages"),
        )
        .error("Choose at least one"),
      )
      .into()
  });
  let quad = stage.border("languages").expect("an edge");
  assert_eq!(
    quad.border.and_then(|border| border.top).map(|side| side.color),
    Some(light(PaletteColor::Destructive))
  );
  stage.key("Tab");
  type_in(&mut stage, "wg");
  stage.key("Enter");
  assert_eq!(values(&value), ["wgsl"]);
}
