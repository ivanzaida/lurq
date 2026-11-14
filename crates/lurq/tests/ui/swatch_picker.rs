//! A swatch picker: the board's panel and grid, the ring of the chosen swatch, the keys in two directions, a click,
//! Enter and Space choosing, `on_change` told a person's choice and not the app's, a picker held by the app, a
//! disabled swatch passed by, the bare grid, and what tooling reads.

use std::sync::{Arc, Mutex};

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::Column,
  core::Signal,
  layout::quad::QuadContent,
  node::color::Color,
  ui::{Swatch, SwatchPicker},
};

use super::stage::Stage;

/// The board's swatches: reds to greens, teals to pinks, and the greys.
const BOARD: [&str; 18] = [
  "#EF4444", "#F97316", "#F59E0B", "#EAB308", "#84CC16", "#22C55E", "#14B8A6", "#06B6D4", "#3B82F6", "#6366F1",
  "#A855F7", "#EC4899", "#FAFAFA", "#D4D4D8", "#A1A1AA", "#71717A", "#3F3F46", "#18181B",
];

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn hex(text: &str) -> Color {
  Color::from_hex(text)
}

/// The board's picker, each swatch with an id of its hex without the `#`, bound to `value`.
fn board_swatches() -> Vec<Swatch> {
  BOARD
    .iter()
    .map(|text| Swatch::new(hex(text)).id(text.trim_start_matches('#').to_owned()))
    .collect()
}

fn staged<T: Clone + PartialEq + Send + Sync + 'static>(
  scene: impl Fn() -> SwatchPicker<T> + Send + Sync + 'static,
) -> Stage {
  let mut stage = Stage::new(move |_| Column::new().padding(8.0).child(scene()).into());
  stage.pass();
  stage
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// Whether a ring of `width` in `color` is drawn around the box of `id`.
fn ringed(stage: &mut Stage, id: &str, width: f32, color: Color) -> bool {
  let (x, y, w, h) = stage.bounds(id);
  stage.quads().iter().any(|quad| {
    matches!(&quad.content, QuadContent::Rect { color: fill, gradient: None } if fill.a() == 0)
      && quad
        .border
        .and_then(|border| border.top)
        .is_some_and(|side| side.width == width && side.color == color)
      && quad.x <= x
      && quad.y <= y
      && quad.x + quad.width >= x + w
      && quad.y + quad.height >= y + h
  })
}

#[test]
fn the_panel_is_padded_10_with_rows_of_six_swatches_of_24_6_apart() {
  let value = Signal::new(hex("#3B82F6"));
  let mut stage = staged(move || SwatchPicker::new(value.clone(), board_swatches()).id("picker"));
  let (x, y, width, height) = stage.bounds("picker");
  assert_eq!(
    width,
    10.0 + 6.0 * 24.0 + 5.0 * 6.0 + 10.0,
    "six of 24, 6 apart, padded 10"
  );
  assert_eq!(height, 10.0 + 3.0 * 24.0 + 2.0 * 6.0 + 10.0, "three rows 6 apart");
  assert_eq!(stage.fill_color("picker"), Some(light(PaletteColor::Popover)));
  let edge = stage.border("picker").expect("the panel's edge");
  let side = edge.border.and_then(|border| border.top).unwrap();
  assert_eq!((side.width, side.color), (1.0, light(PaletteColor::Border)));
  assert!(
    edge.border_radius.is_some_and(|radius| radius.top_left == 8.0),
    "the large radius"
  );

  let first = stage.bounds("EF4444");
  assert_eq!((first.0 - x, first.1 - y, first.2, first.3), (10.0, 10.0, 24.0, 24.0));
  let second = stage.bounds("F97316");
  assert_eq!(second.0 - first.0, 30.0, "6 apart");
  let below = stage.bounds("14B8A6");
  assert_eq!((below.0, below.1 - first.1), (first.0, 30.0), "the next row 6 under");
  // Each swatch has an edge of 1 in the veil of a press, the board's black at 8%.
  let swatch_edge = stage.border("EF4444").expect("a swatch's edge");
  let side = swatch_edge.border.and_then(|border| border.top).unwrap();
  assert_eq!((side.width, side.color), (1.0, light(PaletteColor::StatePressed)));
}

#[test]
fn the_chosen_swatch_is_ringed_in_the_foreground() {
  let value = Signal::new(hex("#3B82F6"));
  let mut stage = staged(move || SwatchPicker::new(value.clone(), board_swatches()).id("picker"));
  assert!(ringed(&mut stage, "3B82F6", 2.0, light(PaletteColor::Foreground)));
  assert!(!ringed(&mut stage, "6366F1", 2.0, light(PaletteColor::Foreground)));
}

#[test]
fn tab_enters_at_the_chosen_swatch_and_the_arrows_move_in_two_directions() {
  let value = Signal::new(hex("#3B82F6"));
  let bound = value.clone();
  let mut stage = staged(move || SwatchPicker::new(bound.clone(), board_swatches()).id("picker"));
  stage.key("Tab");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("3B82F6"),
    "one stop, at the chosen swatch"
  );
  stage.key("ArrowRight");
  assert_eq!(focused(&stage).as_deref(), Some("6366F1"));
  stage.key("ArrowUp");
  assert_eq!(focused(&stage).as_deref(), Some("EAB308"), "a row up");
  stage.key("ArrowUp");
  assert_eq!(focused(&stage).as_deref(), Some("EAB308"), "not past the first row");
  stage.key("ArrowDown");
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("71717A"), "two rows down");
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("71717A"), "not past the last row");
  stage.key("Home");
  assert_eq!(focused(&stage).as_deref(), Some("EF4444"));
  stage.key("End");
  assert_eq!(focused(&stage).as_deref(), Some("18181B"));
  stage.key("ArrowRight");
  assert_eq!(focused(&stage).as_deref(), Some("18181B"), "the keys do not go round");
  assert_eq!(value.get(), hex("#3B82F6"), "moving the focus chooses nothing");

  stage.key("Enter");
  assert_eq!(value.get(), hex("#18181B"), "Enter chooses");
  stage.key("Home");
  stage.key(" ");
  assert_eq!(value.get(), hex("#EF4444"), "Space chooses");
  stage.key("Tab");
  assert_ne!(focused(&stage).as_deref(), Some("F97316"), "Tab leaves the grid");
}

#[test]
fn on_change_is_told_a_persons_choice_and_not_the_one_chosen_already_nor_the_apps() {
  let value = Signal::new(hex("#3B82F6"));
  let told = Arc::new(Mutex::new(Vec::new()));
  let second = Arc::new(Mutex::new(0));
  let (bound, tell, also) = (value.clone(), told.clone(), second.clone());
  let mut stage = staged(move || {
    let (tell, also) = (tell.clone(), also.clone());
    SwatchPicker::new(bound.clone(), board_swatches())
      .on_change(move |color: &Color| tell.lock().unwrap().push(color.to_hex_upper()))
      .on_change(move |_| *also.lock().unwrap() += 1)
  });
  stage.click("22C55E");
  assert_eq!(value.get(), hex("#22C55E"));
  assert_eq!(*told.lock().unwrap(), ["#22C55E"]);
  assert_eq!(*second.lock().unwrap(), 1, "a second handler after the first");
  stage.click("22C55E");
  assert_eq!(told.lock().unwrap().len(), 1, "the one chosen already tells nothing");
  value.set(hex("#EF4444"));
  stage.pass();
  assert_eq!(told.lock().unwrap().len(), 1, "nor does the app's set");
  assert!(ringed(&mut stage, "EF4444", 2.0, light(PaletteColor::Foreground)));
}

#[test]
fn a_picker_held_by_the_app_shows_what_it_answers_with() {
  // The app is told each choice, and keeps red whatever is asked.
  let asked = Arc::new(Mutex::new(Vec::new()));
  let tell = asked.clone();
  let mut stage = staged(move || {
    let tell = tell.clone();
    SwatchPicker::controlled(hex("#EF4444"), board_swatches(), move |color: &Color| {
      tell.lock().unwrap().push(color.to_hex_upper())
    })
  });
  stage.click("3B82F6");
  assert_eq!(*asked.lock().unwrap(), ["#3B82F6"]);
  assert!(
    ringed(&mut stage, "EF4444", 2.0, light(PaletteColor::Foreground)),
    "the app's answer"
  );
  assert!(!ringed(&mut stage, "3B82F6", 2.0, light(PaletteColor::Foreground)));
}

#[test]
fn an_optional_picker_of_the_apps_values_starts_at_the_first_and_chooses_a_value() {
  let value: Signal<Option<&'static str>> = Signal::new(None);
  let bound = value.clone();
  let mut stage = staged(move || {
    SwatchPicker::optional(
      bound.clone(),
      [
        Swatch::of("red", hex("#EF4444")).label("Red").id("red"),
        Swatch::of("blue", hex("#3B82F6")).label("Blue").id("blue"),
      ],
    )
  });
  stage.key("Tab");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("red"),
    "none chosen: Tab enters at the first"
  );
  stage.key("ArrowRight");
  stage.key("Enter");
  assert_eq!(value.get(), Some("blue"));
}

#[test]
fn a_disabled_swatch_is_passed_by_and_takes_nothing() {
  let value = Signal::new(hex("#EF4444"));
  let bound = value.clone();
  let mut stage = staged(move || {
    let swatches = BOARD.iter().map(|text| {
      let swatch = Swatch::new(hex(text)).id(text.trim_start_matches('#').to_owned());
      // The second swatch of each row cannot be chosen, and the first of the second row.
      swatch.disabled(matches!(*text, "#F97316" | "#06B6D4" | "#D4D4D8" | "#14B8A6"))
    });
    SwatchPicker::new(bound.clone(), swatches).columns(6)
  });
  stage.key("Tab");
  stage.key("ArrowRight");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("F59E0B"),
    "Right passes the disabled one by"
  );
  stage.key("Home");
  stage.key("ArrowDown");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("FAFAFA"),
    "Down passes a disabled one row after row"
  );
  stage.click("F97316");
  assert_eq!(
    value.get(),
    hex("#EF4444"),
    "a press of a disabled swatch chooses nothing"
  );
}

#[test]
fn bare_it_is_the_grid_alone() {
  let value = Signal::new(hex("#3B82F6"));
  let mut stage = staged(move || {
    SwatchPicker::new(value.clone(), board_swatches())
      .framed(false)
      .columns(3)
      .id("picker")
  });
  let (_, _, width, height) = stage.bounds("picker");
  assert_eq!(width, 3.0 * 24.0 + 2.0 * 6.0, "no padding, three to a row");
  assert_eq!(height, 6.0 * 24.0 + 5.0 * 6.0, "six rows");
  assert_eq!(stage.fill_color("picker"), None, "no panel");
}

/// To tooling the picker is a listbox called by its label, of options called by theirs, the chosen one selected.
#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn tooling_reads_a_listbox_of_options() {
  let value = Signal::new(hex("#3B82F6"));
  let mut stage = staged(move || {
    SwatchPicker::new(
      value.clone(),
      [
        Swatch::new(hex("#EF4444")).label("Red").id("red"),
        Swatch::new(hex("#3B82F6")).id("blue"),
        Swatch::new(hex("#22C55E")).disabled(true).id("green"),
      ],
    )
    .label("Label color")
    .id("picker")
  });
  let mut said = |id: &str, what: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(what))
  };
  assert_eq!(said("picker", "a11y_role").as_deref(), Some("listbox"));
  assert_eq!(said("picker", "a11y_name").as_deref(), Some("Label color"));
  assert_eq!(said("red", "a11y_role").as_deref(), Some("option"));
  assert_eq!(said("red", "a11y_name").as_deref(), Some("Red"));
  assert_eq!(said("red", "a11y_selected").as_deref(), Some("false"));
  assert_eq!(
    said("blue", "a11y_name").as_deref(),
    Some("#3B82F6"),
    "its hex where it has no label"
  );
  assert_eq!(said("blue", "a11y_selected").as_deref(), Some("true"));
  assert_eq!(said("green", "a11y_disabled").as_deref(), Some("true"));
}
