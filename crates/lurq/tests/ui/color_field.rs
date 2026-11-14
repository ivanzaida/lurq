//! A color field: the board's box of 180 by 36 with its swatch, its hex and its eyedropper, what is typed and read
//! on Enter and as the focus leaves, an optional field emptied, the picker that opens under it and where the focus
//! goes as it closes, a controlled field, read only and disabled, and what tooling reads.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    events::MouseButton,
    theme::{PaletteColor, ThemePalette},
  },
  components::Column,
  core::Signal,
  layout::quad::QuadContent,
  node::color::Color,
  ui::{ColorField, icons},
};

use super::stage::Stage;

const BLUE: &str = "#2563EB";

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn blue() -> Color {
  Color::from_hex(BLUE)
}

/// A page of 480 by 480 with `field` at 8, 8: it has a size of its own, so that the popover has room.
fn page(field: impl Fn() -> lurq::node::Element + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::new(move |_| Column::new().size(480.0, 480.0).padding(8.0).child(field()).into());
  stage.pass();
  stage
}

fn told() -> (Arc<Mutex<Vec<Color>>>, impl Fn(Color) + Send + Sync + Clone + 'static) {
  let heard = Arc::new(Mutex::new(Vec::new()));
  let tell = heard.clone();
  (heard, move |color| tell.lock().unwrap().push(color))
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
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

/// A press on the page where nothing is: what takes the focus from the box.
fn press_blank(stage: &mut Stage) {
  stage.tree.mouse_move(470.0, 470.0);
  stage.tree.mouse_down(470.0, 470.0, MouseButton::Left);
  stage.tree.mouse_up(470.0, 470.0, MouseButton::Left);
  stage.pass();
}

fn is_open(stage: &mut Stage) -> bool {
  stage.tree.get_element_by_id_mut("field-picker-area").is_some()
}

#[test]
fn a_field_is_180_by_36_with_its_swatch_its_hex_and_an_eyedropper() {
  let value = Signal::new(blue());
  let mut stage = page(move || ColorField::new(value.clone()).id("field").into());
  let (x, y, width, height) = stage.bounds("field");
  assert_eq!((width, height), (180.0, 36.0));
  let edge = stage.border("field").expect("an input's edge");
  assert_eq!(
    edge.border.and_then(|border| border.top).unwrap().color,
    light(PaletteColor::Input)
  );
  let swatch = stage.bounds("field-swatch");
  assert_eq!((swatch.2, swatch.3), (24.0, 24.0));
  assert_eq!(swatch.0 - x, 6.0, "6 from the start");
  assert!((swatch.1 - (y + 6.0)).abs() < 0.5, "in the middle");
  let hex = stage.text_quad("#2563EB").expect("the hex");
  assert!((hex.x - (x + 38.0)).abs() < 0.5, "8 after the swatch: {}", hex.x - x);
  assert_eq!(stage.text("#2563EB"), (light(PaletteColor::Foreground), 13.0));
  assert!(
    stage.font_of("#2563EB").to_lowercase().contains("mono"),
    "{}",
    stage.font_of("#2563EB")
  );
  let dropper = glyph(&stage, icons::EYEDROPPER);
  let (color, size) = stage.text(&dropper);
  assert_eq!((color, size), (light(PaletteColor::MutedForeground), 16.0));
  let icon = stage.text_quad(&dropper).unwrap();
  assert!((x + width - (icon.x + icon.width) - 6.0).abs() < 1.0, "6 from the end");
}

#[test]
fn what_is_typed_is_read_on_enter_and_as_the_focus_leaves_and_what_is_no_color_is_put_back() {
  let value = Signal::new(blue());
  let (heard, tell) = told();
  let bound = value.clone();
  let mut stage = page(move || {
    ColorField::new(bound.clone())
      .on_change(tell.clone())
      .id("field")
      .into()
  });
  stage.click("field");
  stage.tree.text_input_replace(0..7, "rgb(34, 197, 94)");
  stage.pass();
  assert_eq!(value.get(), blue(), "nothing is read while it is typed");
  stage.key("Enter");
  assert_eq!(value.get(), Color::from_hex("#22C55E"));
  assert!(stage.paints("#22C55E"), "written as hex again");
  // Enter leaves the box, as it does a line typed in; a click comes back.
  stage.click("field");
  stage.tree.text_input_replace(0..7, "#f59e0b");
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("field"));
  press_blank(&mut stage);
  assert_eq!(focused(&stage), None, "the press takes the focus");
  assert_eq!(value.get(), Color::from_hex("#F59E0B"), "the focus leaving reads it");
  stage.click("field");
  stage.tree.text_input_replace(0..7, "orange-ish");
  stage.pass();
  stage.key("Enter");
  assert_eq!(value.get(), Color::from_hex("#F59E0B"));
  assert!(stage.paints("#F59E0B"), "what is no color is put back");
  assert_eq!(
    *heard.lock().unwrap(),
    [Color::from_hex("#22C55E"), Color::from_hex("#F59E0B")]
  );
  value.set(blue());
  stage.pass();
  assert!(stage.paints("#2563EB"));
  assert_eq!(heard.lock().unwrap().len(), 2, "what the app sets is not told");
}

#[test]
fn an_optional_field_emptied_holds_none_and_shows_its_placeholder() {
  let value: Signal<Option<Color>> = Signal::new(Some(blue()));
  let bound = value.clone();
  let mut stage = page(move || {
    ColorField::optional(bound.clone())
      .placeholder("No tint")
      .id("field")
      .into()
  });
  stage.click("field");
  stage.tree.text_input_replace(0..7, "");
  stage.pass();
  stage.key("Enter");
  assert_eq!(value.get(), None);
  assert!(stage.paints("No tint"));
}

#[test]
fn the_eyedropper_opens_the_picker_and_escape_closes_it_with_the_focus_back_in_the_box() {
  let value = Signal::new(blue());
  let bound = value.clone();
  let mut stage = page(move || ColorField::new(bound.clone()).id("field").into());
  assert!(!is_open(&mut stage));
  stage.click("field-trigger");
  stage.pass();
  assert!(is_open(&mut stage), "the picker opens");
  let (_, y, _, height) = stage.bounds("field");
  let area = stage.bounds("field-picker-area");
  assert!(area.1 > y + height, "under the field");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("field-picker-area"),
    "the focus goes into the area"
  );
  // The area's keys change the field's color.
  stage.key("ArrowLeft");
  assert_ne!(value.get(), blue());
  stage.key("Escape");
  stage.pass();
  assert!(!is_open(&mut stage));
  assert_eq!(focused(&stage).as_deref(), Some("field"), "back in the box");

  // Down in the box opens it too.
  stage.key("ArrowDown");
  stage.pass();
  assert!(is_open(&mut stage));
}

#[test]
fn a_press_in_the_picker_sets_the_fields_color_and_a_press_outside_closes_it() {
  let value = Signal::new(blue());
  let bound = value.clone();
  let mut stage = page(move || ColorField::new(bound.clone()).id("field").into());
  stage.click("field-swatch");
  stage.pass();
  assert!(is_open(&mut stage), "the swatch opens it too");
  let (x, y, _, height) = stage.bounds("field-picker-area");
  stage.tree.mouse_move(x + 1.0, y + height - 1.0);
  stage.tree.mouse_down(x + 1.0, y + height - 1.0, MouseButton::Left);
  stage.tree.mouse_up(x + 1.0, y + height - 1.0, MouseButton::Left);
  stage.pass();
  assert!(value.get().to_hsv().v < 0.02, "near black: {:?}", value.get().to_hsv());
  press_blank(&mut stage);
  assert!(!is_open(&mut stage));
}

#[test]
fn a_controlled_field_tells_a_change_and_shows_the_answer() {
  let held = Signal::new(blue());
  let (heard, tell) = told();
  let scene = held.clone();
  let mut stage = page(move || {
    let (held, tell) = (scene.clone(), tell.clone());
    ColorField::controlled(scene.get(), move |color: Color| {
      tell(color);
      // The app keeps the blue whatever is asked.
      held.set(blue());
    })
    .id("field")
    .into()
  });
  stage.click("field");
  stage.tree.text_input_replace(0..7, "#22C55E");
  stage.pass();
  stage.key("Enter");
  assert_eq!(*heard.lock().unwrap(), [Color::from_hex("#22C55E")]);
  assert!(stage.paints("#2563EB"), "the app's answer is shown");
}

#[test]
fn a_read_only_field_takes_nothing_typed_and_does_not_open_and_a_disabled_one_is_dimmed() {
  let value = Signal::new(blue());
  let bound = value.clone();
  let mut stage = page(move || ColorField::new(bound.clone()).read_only(true).id("field").into());
  stage.click("field-trigger");
  stage.pass();
  assert!(!is_open(&mut stage));
  assert_eq!(
    stage.fill_color("field"),
    Some(light(PaletteColor::Muted)),
    "a read-only box"
  );
  stage.click("field");
  stage.tree.text_input_replace(0..7, "#000000");
  stage.pass();
  stage.key("Enter");
  assert_eq!(value.get(), blue());

  let bound = value.clone();
  let mut stage = page(move || ColorField::new(bound.clone()).disabled(true).id("field").into());
  stage.click("field-trigger");
  stage.pass();
  assert!(!is_open(&mut stage));
  stage.key("Tab");
  assert_ne!(focused(&stage).as_deref(), Some("field"), "no stop of Tab");
  let filled = stage
    .quads()
    .iter()
    .any(|quad| matches!(&quad.content, QuadContent::Rect { .. }));
  assert!(filled);
}

/// To tooling the box is a combobox that opens a dialog and says whether it is open.
#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn tooling_reads_a_combobox_that_opens_a_dialog() {
  let value = Signal::new(blue());
  let mut stage = page(move || ColorField::new(value.clone()).label("Brand color").id("field").into());
  let mut said = |stage: &mut Stage, what: &str| {
    stage
      .tree
      .get_element_by_id_mut("field")
      .and_then(|element| element.description(what))
  };
  assert_eq!(said(&mut stage, "a11y_role").as_deref(), Some("combobox"));
  assert_eq!(said(&mut stage, "a11y_name").as_deref(), Some("Brand color"));
  assert_eq!(said(&mut stage, "a11y_haspopup").as_deref(), Some("dialog"));
  assert_eq!(said(&mut stage, "a11y_expanded").as_deref(), Some("false"));
  stage.click("field-trigger");
  stage.pass();
  assert_eq!(said(&mut stage, "a11y_expanded").as_deref(), Some("true"));
}
