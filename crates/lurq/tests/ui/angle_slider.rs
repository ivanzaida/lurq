//! An angle slider: the board's dial of 64 with its readout and its handle at the angle, a press and a drag round it
//! (going round, or held at its ends), the keys, the step, a range of the app's, bound and controlled with
//! `on_change` and `on_change_end`, disabled, a finger, what tooling reads, and a theme's rules and renderer.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    events::{MouseButton, TouchPhase},
    theme::{OpacityLevel, PaletteColor, ThemePalette},
  },
  components::Column,
  core::Signal,
  layout::quad::{Quad, QuadContent},
  node::color::Color,
  ui::{
    AngleSlider, AngleSliderPart, AngleSliderRenderer, AngleSliderState, AngleSliderView, Case, Field, Interaction,
    Opacity,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn scene(dial: impl Fn() -> AngleSlider + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::new(move |_| {
    Column::new()
      .size(400.0, 300.0)
      .padding(40.0)
      .child(dial().id("dial"))
      .into()
  });
  stage.tree.resize(400, 300);
  stage.pass();
  stage
}

/// A dial bound to `angle` that writes down what `on_change` and `on_change_end` are told, with what the signal held
/// then.
#[allow(clippy::type_complexity)]
fn told(angle: &Signal<f64>) -> (Stage, Arc<Mutex<Vec<(&'static str, f64, f64)>>>) {
  let heard = Arc::new(Mutex::new(Vec::new()));
  let (read, write) = (angle.clone(), heard.clone());
  let stage = scene(move || {
    let (seen, write, end_seen, end_write) = (read.clone(), write.clone(), read.clone(), write.clone());
    AngleSlider::new(read.clone())
      .on_change(move |angle| write.lock().unwrap().push(("change", angle, seen.get_untracked())))
      .on_change_end(move |angle| end_write.lock().unwrap().push(("end", angle, end_seen.get_untracked())))
  });
  (stage, heard)
}

/// The point of the window at `bearing` degrees (0 at the top, clockwise) and `reach` from the dial's middle.
fn at(stage: &mut Stage, bearing: f32, reach: f32) -> (f32, f32) {
  let (x, y) = stage.center("dial");
  let radians = bearing.to_radians();
  (x + reach * radians.sin(), y - reach * radians.cos())
}

/// Drags from `from` through `bearings`, 24 from the middle, one frame after each move, and lets go at the last.
fn drag(stage: &mut Stage, bearings: &[f32]) {
  let (x, y) = at(stage, bearings[0], 24.0);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.frame();
  for &bearing in &bearings[1..] {
    let (x, y) = at(stage, bearing, 24.0);
    stage.tree.mouse_move(x, y);
    stage.frame();
  }
  let (x, y) = at(stage, *bearings.last().unwrap(), 24.0);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

/// The handle: the quad of the primary color, 9 wide.
fn handle(stage: &Stage) -> Quad {
  stage
    .quads()
    .into_iter()
    .find(|quad| {
      (quad.width - 9.0).abs() < 0.01
        && matches!(quad.content, QuadContent::Rect { color, .. } if color == light(PaletteColor::Primary))
    })
    .expect("the handle")
}

/// The bearing of the handle's middle from the dial's.
fn handle_bearing(stage: &mut Stage) -> f32 {
  let quad = handle(stage);
  let (x, y) = stage.center("dial");
  let (dx, dy) = (quad.x + quad.width / 2.0 - x, quad.y + quad.height / 2.0 - y);
  dx.atan2(-dy).to_degrees().rem_euclid(360.0)
}

#[test]
fn at_rest_it_is_the_boards_dial_of_64_with_its_angle_and_its_handle() {
  let mut stage = scene(|| AngleSlider::controlled(45.0, |_| {}));
  let (x, y, width, height) = stage.bounds("dial");
  assert_eq!((width, height), (64.0, 64.0));
  let edge = stage.border("dial").unwrap().border.unwrap().top.unwrap();
  assert_eq!((edge.width, edge.color), (1.5, light(PaletteColor::Input)));
  assert_eq!(stage.fill_color("dial"), Some(light(PaletteColor::Background)));
  let (color, size) = stage.text("45°");
  assert_eq!((color, size), (light(PaletteColor::Foreground), 13.0));
  assert!(stage.font_of("45°").to_lowercase().contains("mono"));
  let readout = stage.text_quad("45°").unwrap();
  assert!(
    (readout.x + readout.width / 2.0 - (x + 32.0)).abs() < 0.5,
    "in the middle"
  );
  // The board's handle: 9 at (46, 9), its middle 26 from the dial's at 45 degrees.
  let quad = handle(&stage);
  assert!(
    (quad.x - (x + 46.0 - 0.1)).abs() < 0.5 && (quad.y - (y + 9.0 + 0.1)).abs() < 0.5,
    "{} {}",
    quad.x,
    quad.y
  );
  assert!((handle_bearing(&mut stage) - 45.0).abs() < 0.01);
  let reach = ((quad.x + 4.5 - (x + 32.0)).powi(2) + (quad.y + 4.5 - (y + 32.0)).powi(2)).sqrt();
  assert!(
    (reach - 26.0).abs() < 0.01,
    "inside the ring of 1.5, touching it: {reach}"
  );
}

#[test]
fn a_press_goes_to_its_angle_and_a_drag_goes_on_round_past_0() {
  let angle = Signal::new(45.0);
  let (mut stage, heard) = told(&angle);
  drag(&mut stage, &[300.0, 320.0, 340.0, 355.0, 10.0, 30.0]);
  assert_eq!(angle.get(), 30.0, "round past 360 to 30");
  let heard = heard.lock().unwrap().clone();
  let changes: Vec<f64> = heard
    .iter()
    .filter(|(what, ..)| *what == "change")
    .map(|(_, angle, _)| *angle)
    .collect();
  assert_eq!(changes, [300.0, 320.0, 340.0, 355.0, 10.0, 30.0]);
  assert!(
    heard.iter().all(|(_, told, held)| told == held),
    "the signal is set before the handlers are told"
  );
  assert_eq!(
    heard.last(),
    Some(&("end", 30.0, 30.0)),
    "and the end once, when it is let go"
  );
  assert_eq!(heard.iter().filter(|(what, ..)| *what == "end").count(), 1);
  assert!((handle_bearing(&mut stage) - 30.0).abs() < 0.01);
  assert!(stage.paints("30°"));
}

#[test]
fn not_going_round_a_drag_past_the_end_stays_there_until_it_comes_back() {
  let angle = Signal::new(340.0);
  let read = angle.clone();
  let mut stage = scene(move || AngleSlider::new(read.clone()).wrap(false));
  drag(&mut stage, &[340.0, 355.0, 10.0, 40.0]);
  assert_eq!(angle.get(), 360.0, "held at 360");
  drag(&mut stage, &[350.0, 359.6, 20.0, 340.0]);
  assert_eq!(angle.get(), 340.0, "past the end and back");
}

#[test]
fn a_drag_snaps_to_the_step_and_a_press_where_it_is_tells_nothing() {
  let angle = Signal::new(90.0);
  let heard = Arc::new(Mutex::new(Vec::new()));
  let (read, write) = (angle.clone(), heard.clone());
  let mut stage = scene(move || {
    let (write, end) = (write.clone(), write.clone());
    AngleSlider::new(read.clone())
      .step(15)
      .on_change(move |angle| write.lock().unwrap().push(angle))
      .on_change_end(move |angle| end.lock().unwrap().push(-angle))
  });
  drag(&mut stage, &[50.0, 61.0, 68.0]);
  assert_eq!(angle.get(), 75.0);
  assert_eq!(*heard.lock().unwrap(), [45.0, 60.0, 75.0, -75.0]);
  heard.lock().unwrap().clear();
  drag(&mut stage, &[76.0, 77.0]);
  assert!(
    heard.lock().unwrap().is_empty(),
    "a press at the angle it holds changes nothing"
  );
  // A drag taken back to where it began tells no end.
  drag(&mut stage, &[75.0, 120.0, 75.0]);
  assert_eq!(*heard.lock().unwrap(), [120.0, 75.0]);
}

#[test]
fn the_keys_step_page_and_go_to_the_ends_and_go_round() {
  let angle = Signal::new(358.0);
  let (mut stage, heard) = told(&angle);
  stage.key("Tab");
  stage.key("ArrowRight");
  assert_eq!(angle.get(), 359.0);
  stage.key("ArrowUp");
  assert_eq!(angle.get(), 0.0, "round to 0");
  stage.key("ArrowLeft");
  assert_eq!(angle.get(), 359.0);
  stage.key("PageUp");
  assert_eq!(angle.get(), 14.0, "15 on, round");
  stage.key("PageDown");
  stage.key("ArrowDown");
  assert_eq!(angle.get(), 358.0);
  stage.key("Home");
  assert_eq!(angle.get(), 0.0);
  stage.key("End");
  assert_eq!(angle.get(), 359.0, "the last step before it comes round");
  // A key's step is a change and its end.
  let heard = heard.lock().unwrap().clone();
  assert_eq!(heard[..2], [("change", 359.0, 359.0), ("end", 359.0, 359.0)]);
  assert!(stage.has_ring(light(PaletteColor::Ring)), "the ring is around the dial");
}

#[test]
fn a_range_of_the_app_s_stops_at_its_ends_and_a_press_past_them_goes_to_the_nearer() {
  let angle = Signal::new(0.0);
  let read = angle.clone();
  let mut stage = scene(move || AngleSlider::new(read.clone()).min(-90).max(90).step(5).page_step(30));
  assert!(stage.paints("0°"));
  assert!((handle_bearing(&mut stage) - 0.0).abs() < 0.01 || (handle_bearing(&mut stage) - 360.0).abs() < 0.01);
  stage.key("Tab");
  stage.key("PageDown");
  assert_eq!(angle.get(), -30.0);
  stage.key("Home");
  assert_eq!(angle.get(), -90.0);
  stage.key("ArrowLeft");
  assert_eq!(angle.get(), -90.0, "no less than the least");
  assert!(stage.paints("-90°"));
  assert!(
    (handle_bearing(&mut stage) - 270.0).abs() < 0.01,
    "-90 is a quarter turn back"
  );
  drag(&mut stage, &[160.0]);
  assert_eq!(angle.get(), 90.0, "past the greatest, the nearer end");
  drag(&mut stage, &[330.0]);
  assert_eq!(angle.get(), -30.0);
}

#[test]
fn a_step_of_a_half_says_its_decimals() {
  let mut stage = scene(|| AngleSlider::controlled(22.5, |_| {}).step(0.5));
  assert!(stage.paints("22.5°"));
  let mut stage2 = scene(|| AngleSlider::controlled(90.0, |_| {}).value_text(|angle| format!("{angle} deg")));
  assert!(stage2.paints("90 deg"));
  let _ = (&mut stage, &mut stage2);
}

#[test]
fn a_dial_the_app_holds_shows_what_the_app_answers() {
  let held = Signal::new(10.0);
  let asked = Arc::new(Mutex::new(Vec::new()));
  let (read, told) = (held.clone(), asked.clone());
  // The app takes no angle past 180.
  let mut stage = scene(move || {
    let (set, told) = (read.clone(), told.clone());
    AngleSlider::controlled(read.get(), move |angle| {
      told.lock().unwrap().push(angle);
      if angle <= 180.0 {
        set.set(angle);
      }
    })
  });
  drag(&mut stage, &[90.0]);
  drag(&mut stage, &[270.0]);
  assert_eq!(*asked.lock().unwrap(), [90.0, 270.0]);
  assert_eq!(held.get(), 90.0);
  assert!(stage.paints("90°"), "what the app holds, not what was pressed");
  // The app setting it tells nobody.
  held.set(120.0);
  stage.pass();
  assert_eq!(asked.lock().unwrap().len(), 2);
  assert!(stage.paints("120°"));
}

#[test]
fn a_finger_turns_it_and_is_taken_from_scrolling() {
  let angle = Signal::new(0.0);
  let read = angle.clone();
  let mut stage = scene(move || AngleSlider::new(read.clone()));
  let (x, y) = at(&mut stage, 90.0, 20.0);
  let route = stage.tree.touch_event(3, TouchPhase::Start, x, y, Some(0.4));
  assert_eq!(route, lurq::app::runtime::TouchRoute::Taken);
  let (x, y) = at(&mut stage, 135.0, 20.0);
  stage.tree.touch_event(3, TouchPhase::Move, x, y, Some(0.4));
  stage.tree.touch_event(3, TouchPhase::End, x, y, None);
  stage.pass();
  assert_eq!(angle.get(), 135.0);
}

#[test]
fn disabled_it_takes_nothing_and_is_dimmed() {
  let angle = Signal::new(45.0);
  let read = angle.clone();
  let mut stage = scene(move || AngleSlider::new(read.clone()).disabled(true));
  drag(&mut stage, &[200.0, 250.0]);
  stage.key("Tab");
  stage.key("ArrowRight");
  assert_eq!(angle.get(), 45.0, "neither a press nor a key, and no stop of Tab");
  let root = stage
    .app
    .theme()
    .resolve::<AngleSlider>(&Case::new().state(AngleSliderState::Disabled, true), None)
    .part(AngleSliderPart::Root)
    .rest();
  assert_eq!(root.opacity, Some(Opacity::Level(OpacityLevel::Disabled)));
  // In a field that is disabled, too.
  let held = Signal::new(45.0);
  let read = held.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(Field::new("Rotation", AngleSlider::new(read.clone()).id("dial")).disabled(true))
      .into()
  });
  stage.key("Tab");
  stage.key("ArrowRight");
  assert_eq!(held.get(), 45.0);
}

#[test]
fn a_press_focuses_the_dial_and_the_keys_go_on_from_it() {
  let angle = Signal::new(0.0);
  let read = angle.clone();
  let mut stage = scene(move || AngleSlider::new(read.clone()));
  drag(&mut stage, &[90.0]);
  stage.key("ArrowRight");
  assert_eq!(angle.get(), 91.0, "the press focused it");
}

#[test]
fn a_forced_focus_draws_the_ring_with_nothing_pointing_at_it() {
  let stage = scene(|| AngleSlider::controlled(45.0, |_| {}).force(Some(Interaction::FocusVisible)));
  assert!(stage.has_ring(light(PaletteColor::Ring)));
}

#[test]
fn a_theme_restyles_it_and_gives_it_another_anatomy() {
  const RED: Color = Color::new(200, 30, 30, 255);
  let app = App::new();
  app
    .theme()
    .patch_recipe::<AngleSlider>(|rules| rules.base(AngleSliderPart::Root, |s| s.square(96.0)));
  let mut stage = Stage::in_app(app, |_| {
    Column::new()
      .padding(8.0)
      .child(
        AngleSlider::controlled(90.0, |_| {})
          .id("dial")
          .style(AngleSliderPart::Handle, |s| s.background(RED)),
      )
      .into()
  });
  assert_eq!(stage.bounds("dial").2, 96.0);
  let quad = stage
    .quads()
    .into_iter()
    .find(|quad| matches!(quad.content, QuadContent::Rect { color, .. } if color == RED))
    .expect("the handle, red");
  let (x, y) = stage.center("dial");
  assert!((quad.x + 4.5 - (x + 48.0 - 1.5 - 4.5)).abs() < 0.01 && (quad.y + 4.5 - y).abs() < 0.01);

  let app = App::new();
  let renderer: AngleSliderRenderer = Arc::new(|view: AngleSliderView| {
    let words = format!("{} of {}", view.angle, view.max);
    view.root().child(view.part(AngleSliderPart::Value).text(&words)).into()
  });
  app.theme().set_renderer::<AngleSlider>(renderer);
  let stage = Stage::in_app(app, |_| {
    Column::new().child(AngleSlider::controlled(30.0, |_| {})).into()
  });
  assert!(stage.paints("30 of 360"));
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn tooling_reads_a_slider_of_degrees() {
  let angle = Signal::new(45.0);
  let read = angle.clone();
  let mut stage = scene(move || AngleSlider::new(read.clone()).label("Rotation"));
  let said = |stage: &mut Stage, name: &str| {
    stage
      .tree
      .get_element_by_id_mut("dial")
      .and_then(|element| element.description(name))
  };
  assert_eq!(said(&mut stage, "a11y_role").as_deref(), Some("slider"));
  assert_eq!(said(&mut stage, "a11y_name").as_deref(), Some("Rotation"));
  assert_eq!(said(&mut stage, "a11y_valuemin").as_deref(), Some("0"));
  assert_eq!(said(&mut stage, "a11y_valuemax").as_deref(), Some("360"));
  assert_eq!(said(&mut stage, "a11y_valuenow").as_deref(), Some("45"));
  assert_eq!(said(&mut stage, "a11y_valuetext").as_deref(), Some("45°"));
  angle.set(370.0);
  stage.pass();
  assert_eq!(
    said(&mut stage, "a11y_valuenow").as_deref(),
    Some("10"),
    "an angle round past 360 is shown as 10"
  );
}
