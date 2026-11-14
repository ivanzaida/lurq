//! A color picker: the board's panel, its area of saturation and brightness with its circle at the color, the rail of
//! hue, what a press, a drag and the keys of the area and the rails make of the color, the hue a grey keeps, the rail
//! of alpha over a checkerboard, the swatches, the fields of hex, RGB and HSL, a controlled picker, a disabled one,
//! and what tooling reads.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    events::MouseButton,
    theme::{PaletteColor, ThemePalette},
  },
  components::Column,
  core::Signal,
  eyedropper::EyeDropper,
  layout::quad::{Quad, QuadContent},
  node::color::Color,
  ui::{ColorFormat, ColorPicker},
};

use super::stage::Stage;

const BLUE: &str = "#2563EB";

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn blue() -> Color {
  Color::from_hex(BLUE)
}

/// What a picker told: each change, and each end of one.
#[derive(Clone, Default)]
struct Heard {
  changes: Arc<Mutex<Vec<Color>>>,
  ends: Arc<Mutex<Vec<Color>>>,
}

impl Heard {
  fn changes(&self) -> Vec<Color> {
    std::mem::take(&mut *self.changes.lock().unwrap())
  }

  fn ends(&self) -> Vec<Color> {
    std::mem::take(&mut *self.ends.lock().unwrap())
  }

  fn hear(&self, picker: ColorPicker) -> ColorPicker {
    let (changes, ends) = (self.changes.clone(), self.ends.clone());
    picker
      .on_change(move |color| changes.lock().unwrap().push(color))
      .on_change_end(move |color| ends.lock().unwrap().push(color))
  }
}

/// A stage of a picker of `value`, dressed by `dress`, with what it tells heard.
fn picker(value: &Signal<Color>, dress: impl Fn(ColorPicker) -> ColorPicker + Send + Sync + 'static) -> (Stage, Heard) {
  let (value, heard) = (value.clone(), Heard::default());
  let hear = heard.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .size(400.0, 400.0)
      .padding(8.0)
      .child(dress(hear.hear(ColorPicker::new(value.clone()).id("picker"))))
      .into()
  });
  stage.pass();
  (stage, heard)
}

fn press_at(stage: &mut Stage, (x, y): (f32, f32)) {
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

/// The place at `(across, down)`, from 0 to 1, of the element `id`.
fn at(stage: &mut Stage, id: &str, across: f32, down: f32) -> (f32, f32) {
  let (x, y, width, height) = stage.bounds(id);
  (x + across * width, y + down * height)
}

fn rects_with_gradient(stage: &Stage) -> Vec<Quad> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Rect { gradient: Some(_), .. }))
    .collect()
}

fn near(a: f32, b: f32, by: f32) -> bool {
  (a - b).abs() <= by
}

#[test]
fn the_panel_is_232_wide_with_an_area_of_208_by_120_and_a_rail_of_10_under_it() {
  let value = Signal::new(blue());
  let (mut stage, _) = picker(&value, |picker| picker);
  let panel = stage.bounds("picker");
  assert_eq!(panel.2, 232.0);
  assert_eq!(stage.fill_color("picker"), Some(light(PaletteColor::Popover)));
  let area = stage.bounds("picker-area");
  assert_eq!((area.2, area.3), (208.0, 120.0));
  assert_eq!((area.0 - panel.0, area.1 - panel.1), (12.0, 12.0), "12 in");
  let hue = stage.bounds("picker-hue");
  assert_eq!((hue.2, hue.3), (208.0, 10.0));
  assert_eq!(hue.1 - (area.1 + area.3), 12.0, "12 under the area");
  // Under the hue's pure color: white across from the start, black up from the bottom, and the rail of every hue.
  let pure = Color::from(lurq::node::color::Hsv {
    h: blue().to_hsv().h,
    s: 1.0,
    v: 1.0,
    a: 1.0,
  });
  assert!(stage.quads().iter().any(|quad| {
    matches!(&quad.content, QuadContent::Rect { color, gradient: None } if *color == pure)
      && (quad.width, quad.height) == (208.0, 120.0)
  }));
  assert_eq!(rects_with_gradient(&stage).len(), 3, "white, black and the hues");
  // The hex is the field's, in capitals.
  assert_eq!(stage.text("#2563EB").0, light(PaletteColor::Foreground));
}

#[test]
fn the_circle_of_the_area_stands_at_the_color() {
  let value = Signal::new(blue());
  let (mut stage, _) = picker(&value, |picker| picker);
  let area = stage.bounds("picker-area");
  let hsv = blue().to_hsv();
  let circle = stage
    .quads()
    .into_iter()
    .find(|quad| quad.width == 12.0 && quad.height == 12.0 && quad.border.is_some())
    .expect("the circle of 12 with its ring");
  assert!(
    near(circle.x + 6.0, area.0 + hsv.s * 208.0, 0.5),
    "across at the saturation"
  );
  assert!(
    near(circle.y + 6.0, area.1 + (1.0 - hsv.v) * 120.0, 0.5),
    "up at the brightness"
  );
}

#[test]
fn a_drag_in_the_area_sets_saturation_and_brightness_and_ends_once() {
  let value = Signal::new(blue());
  let (mut stage, heard) = picker(&value, |picker| picker);
  let start = at(&mut stage, "picker-area", 0.5, 0.5);
  stage.tree.mouse_move(start.0, start.1);
  stage.tree.mouse_down(start.0, start.1, MouseButton::Left);
  stage.pass();
  let hsv = value.get().to_hsv();
  assert!(
    near(hsv.s, 0.5, 0.01) && near(hsv.v, 0.5, 0.01),
    "the press sets it: {hsv:?}"
  );
  assert!(near(hsv.h, blue().to_hsv().h, 1.0), "and keeps the hue");
  let end = at(&mut stage, "picker-area", 1.0, 0.0);
  stage.tree.mouse_move(end.0 - 20.0, end.1 + 10.0);
  stage.pass();
  // Past the area's edge the color stays at its end.
  stage.tree.mouse_move(end.0 + 40.0, end.1 - 40.0);
  stage.pass();
  let hsv = value.get().to_hsv();
  assert!(near(hsv.s, 1.0, 0.01) && near(hsv.v, 1.0, 0.01), "{hsv:?}");
  assert_eq!(heard.changes().len(), 3, "told each change as it is made");
  assert!(heard.ends().is_empty(), "held, the change is not over");
  stage.tree.mouse_up(end.0 + 40.0, end.1 - 40.0, MouseButton::Left);
  stage.pass();
  assert_eq!(heard.ends(), [value.get()], "told once, as it is let go");
}

#[test]
fn the_keys_of_the_area_step_by_one_and_by_ten_with_shift() {
  let value = Signal::new(Color::from(lurq::node::color::Hsv {
    h: 200.0,
    s: 0.5,
    v: 0.5,
    a: 1.0,
  }));
  let (mut stage, heard) = picker(&value, |picker| picker);
  stage.key("Tab");
  stage.key("ArrowRight");
  assert!(near(value.get().to_hsv().s, 0.51, 0.006));
  stage
    .tree
    .key_down("ArrowUp".to_owned(), "ArrowUp".to_owned(), true, false, false);
  stage.pass();
  assert!(near(value.get().to_hsv().v, 0.6, 0.006), "Shift: ten");
  stage.key("PageDown");
  assert!(near(value.get().to_hsv().v, 0.5, 0.006));
  stage.key("Home");
  assert!(near(value.get().to_hsv().s, 0.0, 0.006));
  assert_eq!(heard.changes().len(), 4);
  assert_eq!(heard.ends().len(), 4, "a key's step is over as it is made");
}

#[test]
fn a_grey_keeps_the_hue_and_the_rail_of_hue_sets_it() {
  let value = Signal::new(blue());
  let (mut stage, _) = picker(&value, |picker| picker);
  // To the left edge: no saturation, a grey; then back. The hue is the blue's still.
  let grey = at(&mut stage, "picker-area", 0.0, 0.3);
  press_at(&mut stage, grey);
  assert_eq!(value.get().to_hsv().s, 0.0);
  let back = at(&mut stage, "picker-area", 0.9, 0.3);
  press_at(&mut stage, back);
  assert!(
    near(value.get().to_hsv().h, blue().to_hsv().h, 1.5),
    "{:?}",
    value.get().to_hsv()
  );

  let middle = at(&mut stage, "picker-hue", 0.5, 0.5);
  press_at(&mut stage, middle);
  assert!(
    near(value.get().to_hsv().h, 180.0, 1.5),
    "the middle of the rail is cyan"
  );
  // The press focused the rail: its keys go on from there.
  stage.key("End");
  assert!(near(value.get().to_hsv().h, 0.0, 1.5) || near(value.get().to_hsv().h, 360.0, 1.5));
}

#[test]
fn what_the_app_sets_is_shown_and_not_told() {
  let value = Signal::new(blue());
  let (mut stage, heard) = picker(&value, |picker| picker);
  value.set(Color::from_hex("#22C55E"));
  stage.pass();
  assert!(stage.paints("#22C55E"));
  assert!(heard.changes().is_empty() && heard.ends().is_empty());
}

#[test]
fn a_controlled_picker_tells_a_change_and_shows_the_answer() {
  // The app holds the color, and answers every change with its own: it keeps the blue.
  let held = Signal::new(blue());
  let asked = Arc::new(Mutex::new(Vec::new()));
  let (scene, tell) = (held.clone(), asked.clone());
  let mut stage = Stage::new(move |_| {
    let (held, tell) = (scene.clone(), tell.clone());
    Column::new()
      .size(400.0, 400.0)
      .padding(8.0)
      .child(
        ColorPicker::controlled(scene.get(), move |color| {
          tell.lock().unwrap().push(color);
          held.set(blue());
        })
        .id("picker"),
      )
      .into()
  });
  stage.pass();
  let corner = at(&mut stage, "picker-area", 0.0, 1.0);
  press_at(&mut stage, corner);
  assert_eq!(asked.lock().unwrap().len(), 1, "told what was asked");
  assert_eq!(asked.lock().unwrap()[0].to_hsv().v, 0.0);
  assert!(stage.paints("#2563EB"), "the app's answer is shown");
}

#[test]
fn the_rail_of_alpha_stands_over_a_checkerboard_and_sets_the_alpha() {
  let value = Signal::new(blue());
  let (mut stage, heard) = picker(&value, |picker| picker.alpha(true));
  let rail = stage.bounds("picker-alpha");
  assert_eq!((rail.2, rail.3), (208.0, 10.0));
  let cells = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Rect { color, gradient: None } if *color == light(PaletteColor::Muted)))
    .filter(|quad| quad.width == 5.0)
    .count();
  assert!(cells > 0, "the dark squares");
  let quarter = at(&mut stage, "picker-alpha", 0.25, 0.5);
  press_at(&mut stage, quarter);
  assert_eq!(value.get().a(), 64);
  assert!(stage.paints("#2563EB40"), "the hex says the alpha");
  assert_eq!(heard.ends().len(), 1);
}

#[test]
fn a_swatch_makes_the_color_it_and_the_one_of_the_color_is_selected() {
  let value = Signal::new(blue());
  let (mut stage, heard) = picker(&value, |picker| picker.swatches([blue(), Color::from_hex("#F59E0B")]));
  let (x, _, width, _) = stage.bounds("picker-swatch-0");
  assert_eq!(width, 20.0);
  assert_eq!(stage.bounds("picker-swatch-1").0 - (x + width), 8.0, "8 apart");
  stage.click("picker-swatch-1");
  assert_eq!(value.get(), Color::from_hex("#F59E0B"));
  assert_eq!(heard.ends(), [Color::from_hex("#F59E0B")]);
}

#[test]
fn the_hex_field_takes_any_color_written_and_puts_back_what_is_none() {
  let value = Signal::new(blue());
  let (mut stage, heard) = picker(&value, |picker| picker);
  stage.click("picker-hex");
  stage.tree.text_input_replace(0..7, "rgb(34, 197, 94)");
  stage.pass();
  assert_eq!(value.get(), blue(), "nothing is taken while it is typed");
  stage.key("Enter");
  assert_eq!(value.get(), Color::from_hex("#22C55E"));
  assert!(stage.paints("#22C55E"), "written as hex again");
  assert_eq!(heard.ends().len(), 1);

  // Enter left the field; a click comes back.
  stage.click("picker-hex");
  stage.tree.text_input_replace(0..7, "bluish");
  stage.pass();
  assert!(stage.paints("bluish"), "typed");
  stage.key("Enter");
  assert_eq!(value.get(), Color::from_hex("#22C55E"));
  assert!(stage.paints("#22C55E"), "what is no color is put back");
}

#[test]
fn the_fields_switch_to_rgb_and_hsl_and_a_channel_is_typed() {
  let value = Signal::new(blue());
  let (mut stage, _) = picker(&value, |picker| {
    picker.formats([ColorFormat::Hex, ColorFormat::Rgb, ColorFormat::Hsl])
  });
  assert!(stage.paints("HEX"));
  stage.click("picker-format");
  assert!(stage.paints("RGB") && stage.paints("235"), "the channels of the blue");
  stage.click("picker-r");
  stage.tree.text_input_replace(0..2, "200");
  stage.pass();
  stage.key("Enter");
  assert_eq!(value.get(), Color::new(200, 0x63, 0xeb, 255));
  stage.click("picker-format");
  assert!(stage.paints("HSL"));
}

#[test]
fn a_disabled_picker_takes_nothing() {
  let value = Signal::new(blue());
  let (mut stage, heard) = picker(&value, |picker| {
    picker.disabled(true).swatches([Color::from_hex("#F59E0B")])
  });
  let middle = at(&mut stage, "picker-area", 0.5, 0.5);
  press_at(&mut stage, middle);
  stage.click("picker-swatch-0");
  stage.key("Tab");
  stage.key("ArrowRight");
  assert_eq!(value.get(), blue());
  assert!(heard.changes().is_empty());
}

/// To tooling the area and the rails are sliders that say their values in words.
#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn the_sliders_say_their_values_in_words() {
  let value = Signal::new(blue());
  let (mut stage, _) = picker(&value, |picker| picker.alpha(true));
  let mut said = |id: &str, what: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(what))
  };
  assert_eq!(said("picker-area", "a11y_role").as_deref(), Some("slider"));
  assert_eq!(
    said("picker-area", "a11y_valuetext").as_deref(),
    Some("Saturation 84%, brightness 92%")
  );
  assert_eq!(said("picker-hue", "a11y_valuetext").as_deref(), Some("Hue 221°"));
  assert_eq!(said("picker-alpha", "a11y_valuetext").as_deref(), Some("Alpha 100%"));
  assert_eq!(said("picker-hue", "a11y_name").as_deref(), Some("Hue"));
}

#[test]
fn the_eyedropper_sets_the_color_picked_from_the_screen_and_tells_its_change_and_its_end_once() {
  let value = Signal::new(Color::from_hex("#2563EB"));
  let changes = Arc::new(Mutex::new(Vec::new()));
  let ends = Arc::new(Mutex::new(Vec::new()));
  let (bound, change, end) = (value.clone(), changes.clone(), ends.clone());
  let picked = Color::from_hex("#22C55E");
  let _screen = EyeDropper::stand_in(move || Ok(Some(picked)));
  let mut stage = Stage::new(move |_| {
    let (change, end) = (change.clone(), end.clone());
    Column::new()
      .padding(8.0)
      .child(
        ColorPicker::new(bound.clone())
          .on_change(move |color| change.lock().unwrap().push(color))
          .on_change_end(move |color| end.lock().unwrap().push(color))
          .id("picker"),
      )
      .into()
  });
  stage.pass();
  stage.click("picker-eyedropper");
  stage.pass();
  assert_eq!(value.get(), picked);
  assert_eq!(*changes.lock().unwrap(), [picked]);
  assert_eq!(*ends.lock().unwrap(), [picked]);
  assert!(stage.paints("#22C55E"));
  // The same color picked again changes nothing.
  stage.click("picker-eyedropper");
  stage.pass();
  assert_eq!(changes.lock().unwrap().len(), 1);
  assert_eq!(ends.lock().unwrap().len(), 1);
}

#[test]
fn a_pick_given_up_changes_nothing_and_eyedropper_false_hides_it() {
  let value = Signal::new(Color::from_hex("#2563EB"));
  let bound = value.clone();
  let _screen = EyeDropper::stand_in(|| Ok(None));
  let mut stage = Stage::new(move |_| Column::new().child(ColorPicker::new(bound.clone()).id("picker")).into());
  stage.pass();
  stage.click("picker-eyedropper");
  stage.pass();
  assert_eq!(value.get(), Color::from_hex("#2563EB"));
  let mut stage = Stage::new(|_| {
    Column::new()
      .child(
        ColorPicker::new(Signal::new(Color::from_hex("#2563EB")))
          .eyedropper(false)
          .id("picker"),
      )
      .into()
  });
  stage.pass();
  assert!(stage.tree.get_element_by_id_mut("picker-eyedropper").is_none());
}

/// Where the platform cannot pick from the screen, no eyedropper is shown.
#[cfg(not(windows))]
#[test]
fn where_the_platform_cannot_pick_there_is_no_eyedropper() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .child(ColorPicker::new(Signal::new(Color::from_hex("#2563EB"))).id("picker"))
      .into()
  });
  stage.pass();
  assert!(stage.tree.get_element_by_id_mut("picker-eyedropper").is_none());
}

/// The eyedropper says what it does.
#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn the_eyedropper_is_a_button_that_says_what_it_does() {
  let _screen = EyeDropper::stand_in(|| Ok(None));
  let mut stage = Stage::new(|_| {
    Column::new()
      .child(ColorPicker::new(Signal::new(Color::from_hex("#2563EB"))).id("picker"))
      .into()
  });
  stage.pass();
  let element = stage.tree.get_element_by_id_mut("picker-eyedropper").unwrap();
  assert_eq!(element.description("a11y_role").as_deref(), Some("button"));
  assert_eq!(
    element.description("a11y_name").as_deref(),
    Some("Pick a color from the screen")
  );
}
