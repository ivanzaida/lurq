//! A time picker: the board's field and panel, its columns, a choice of an hour and of a minute, what typing sets,
//! the step, the range, a clock of twelve hours, and what tooling is told.

use std::sync::{Arc, Mutex};

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::Column,
  core::{DateLocale, Signal, Time},
  layout::{Constraints, Size},
  node::color::Color,
  ui::{DateMemory, TimePicker},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn at(hour: u8, minute: u8) -> Time {
  Time::new(hour, minute, 0).unwrap()
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// A picker with the id `tp`, mounted at the top of a window of 400 by 500.
fn picker(value: &Signal<Time>, make: impl Fn(TimePicker) -> TimePicker + Send + Sync + 'static) -> Stage {
  let value = value.clone();
  let mut stage = Stage::new(move |ctx| {
    Column::new()
      .size(400.0, 500.0)
      .padding(10.0)
      .child(make(TimePicker::new(value.clone()).id("tp")).mount(ctx))
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 500.0))));
  stage.pass();
  stage
}

fn opened(stage: &mut Stage) -> bool {
  stage.tree.get_element_by_id_mut("tp-panel").is_some()
}

#[test]
fn a_time_picker_is_the_field_of_the_design_with_its_time() {
  let value = Signal::new(at(9, 30));
  let mut stage = picker(&value, |picker| picker);
  let (x, _, width, height) = stage.bounds("tp");
  assert_eq!((width, height), (160.0, 36.0));
  assert_eq!(stage.text("09:30"), (light(PaletteColor::Foreground), 14.0));
  let text = stage.text_quad("09:30").unwrap();
  assert!(
    (text.x - (x + 12.0 + 16.0 + 8.0)).abs() < 0.5,
    "8 after a glyph of 16, 12 from the edge"
  );
  assert!(!opened(&mut stage));
}

#[test]
fn its_panel_has_the_hours_and_the_minutes_a_quarter_apart() {
  let value = Signal::new(at(9, 30));
  let mut stage = picker(&value, |picker| picker);
  stage.click("tp-trigger");
  stage.pass();
  assert!(opened(&mut stage));
  let (x, y, _, height) = stage.bounds("tp");
  let (px, py, pwidth, pheight) = stage.bounds("tp-panel");
  assert_eq!((px, pwidth), (x, 160.0));
  assert!((py - (y + height + 4.0)).abs() < 0.5, "4 under the field");
  assert!((pheight - 134.0).abs() < 0.5, "four items of a column are seen");
  assert_eq!(stage.fill_color("tp-panel"), Some(light(PaletteColor::Popover)));
  let (_, _, iwidth, iheight) = stage.bounds("tp-minute-15");
  assert_eq!((iwidth, iheight), (74.0, 30.0));
  assert!(stage.tree.get_element_by_id_mut("tp-minute-10").is_none(), "15 apart");
  // The held hour and minute are on the highlight; the hour has the focus.
  assert_eq!(stage.fill_color("tp-hour-9"), Some(light(PaletteColor::Highlight)));
  assert_eq!(stage.fill_color("tp-minute-30"), Some(light(PaletteColor::Highlight)));
  assert_eq!(focused(&stage).as_deref(), Some("tp-hour-9"));
}

#[test]
fn an_hour_sets_the_hour_and_a_minute_ends_the_choice() {
  let value = Signal::new(at(9, 30));
  let memory = DateMemory::new();
  let seen = Arc::new(Mutex::new(Vec::new()));
  let (kept, heard) = (memory.clone(), seen.clone());
  let mut stage = picker(&value, move |picker| {
    let (asked, heard) = (kept.clone(), heard.clone());
    picker
      .memory(&kept)
      .on_change(move |time| heard.lock().unwrap().push((time, asked.is_open())))
  });
  stage.click("tp-trigger");
  stage.pass();
  stage.key("ArrowDown");
  stage.key("Enter");
  stage.pass();
  assert_eq!(value.get_untracked(), at(10, 30));
  assert!(memory.is_open(), "an hour leaves it open");
  stage.click("tp-minute-45");
  stage.pass();
  assert_eq!(value.get_untracked(), at(10, 45));
  assert!(!memory.is_open());
  assert_eq!(
    *seen.lock().unwrap(),
    vec![(at(10, 30), true), (at(10, 45), false)],
    "closed before the handler of the minute runs"
  );
  assert_eq!(focused(&stage).as_deref(), Some("tp"));
  assert!(stage.paints("10:45"));
}

#[test]
fn what_is_typed_is_the_time_as_soon_as_it_is_one() {
  let value = Signal::new(at(9, 30));
  let submitted = Arc::new(Mutex::new(Vec::new()));
  let sent = submitted.clone();
  let mut stage = picker(&value, move |picker| {
    let sent = sent.clone();
    picker.on_submit(move |time| sent.lock().unwrap().push(time))
  });
  stage.click("tp");
  stage.tree.text_input_replace(0.."09:30".len(), "14:5");
  stage.pass();
  assert_eq!(value.get_untracked(), at(9, 30), "half typed is no time yet");
  stage.tree.text_input_replace(0.."14:5".len(), "2:05 pm");
  stage.pass();
  assert_eq!(value.get_untracked(), at(14, 5));
  stage.key("Enter");
  assert!(stage.paints("14:05"), "written as the locale writes times");
  assert_eq!(*submitted.lock().unwrap(), vec![at(14, 5)]);
}

#[test]
fn the_range_and_a_clock_of_twelve_hours() {
  let value = Signal::new(at(9, 30));
  let mut stage = picker(&value, |picker| picker.min(at(8, 0)).max(at(18, 0)));
  stage.click("tp-trigger");
  stage.pass();
  stage.click("tp-hour-8");
  assert_eq!(value.get_untracked(), at(8, 30));
  // The hour is reached by the keys, the column scrolling to it; 7 is before the least time.
  stage.key("ArrowUp");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("tp-hour-8"),
    "7 is passed by: it cannot be chosen"
  );
  for _ in 0..9 {
    stage.key("ArrowDown");
  }
  stage.key("Enter");
  assert_eq!(value.get_untracked(), at(17, 30));

  let mut stage = picker(&value, |picker| picker.locale(DateLocale::for_tag("en-US")));
  assert!(stage.paints("5:30 PM"));
  stage.click("tp-trigger");
  stage.pass();
  stage.click("tp-period-am");
  assert_eq!(value.get_untracked(), at(5, 30));
  stage.bounds("tp-hour-12");
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn tooling_reads_a_combobox_of_listboxes() {
  let value = Signal::new(at(9, 30));
  let mut stage = picker(&value, |picker| picker.label("Starts at"));
  let said = |stage: &mut Stage, id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(said(&mut stage, "tp", "a11y_role").as_deref(), Some("combobox"));
  assert_eq!(
    said(&mut stage, "tp-trigger", "a11y_name").as_deref(),
    Some("Choose time")
  );
  stage.click("tp-trigger");
  stage.pass();
  assert_eq!(said(&mut stage, "tp-panel", "a11y_role").as_deref(), Some("dialog"));
  assert_eq!(said(&mut stage, "tp-minute-30", "a11y_role").as_deref(), Some("option"));
  assert_eq!(
    said(&mut stage, "tp-minute-30", "a11y_selected").as_deref(),
    Some("true")
  );
}

#[test]
fn under_a_finger_a_tap_of_the_field_opens_the_panel_and_asks_for_no_keyboard() {
  let value = Signal::new(at(9, 30));
  let held = value.clone();
  let app = lurq::app::App::new();
  app.set_pointer(lurq::app::Pointer::Coarse);
  let mut stage = Stage::in_app(app, move |ctx| {
    Column::new()
      .size(400.0, 500.0)
      .padding(10.0)
      .child(TimePicker::new(held.clone()).id("tp").mount(ctx))
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 500.0))));
  stage.pass();
  assert!(!opened(&mut stage));
  // In the field, well clear of its button.
  let (x, y, width, height) = stage.bounds("tp");
  stage.tap_at(x + width - 20.0, y + height / 2.0);
  assert!(opened(&mut stage), "the whole field opens it");
  // Closed, the focus is the field's again, which asks for no keyboard on the screen: a time is chosen.
  stage.key("Escape");
  assert!(!opened(&mut stage));
  assert_eq!(focused(&stage).as_deref(), Some("tp"), "the field has the focus");
  assert!(stage.tree.keyboard_traits().is_none(), "and no keyboard comes up");
}

#[test]
fn under_a_mouse_a_press_of_the_field_types_and_opens_nothing() {
  let value = Signal::new(at(9, 30));
  let mut stage = picker(&value, |picker| picker);
  let (x, y, width, height) = stage.bounds("tp");
  let (x, y) = (x + width - 20.0, y + height / 2.0);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, lurq::app::events::MouseButton::Left);
  stage.tree.mouse_up(x, y, lurq::app::events::MouseButton::Left);
  stage.pass();
  assert!(!opened(&mut stage), "its button and Down open it");
  assert!(stage.tree.keyboard_traits().is_some(), "it is typed into");
}

/// The quads painted as a line of 1 in the border's color.
fn lines(stage: &Stage) -> Vec<lurq::layout::quad::Quad> {
  let border = light(PaletteColor::Border);
  stage
    .quads()
    .into_iter()
    .filter(|quad| {
      quad.width == 1.0
        && matches!(&quad.content, lurq::layout::quad::QuadContent::Rect { color, .. } if *color == border)
    })
    .collect()
}

#[test]
fn a_recipe_draws_a_separator_between_the_columns_as_high_as_they_are() {
  use lurq::ui::{Recipe, TimePickerPart};
  // None by default: the minutes start the panel's gap after the hours.
  let value = Signal::new(at(9, 30));
  let mut stage = picker(&value, |picker| picker);
  stage.click("tp-trigger");
  stage.pass();
  assert!(lines(&stage).is_empty());
  let (hour_x, _, hour_width, _) = stage.bounds("tp-hour-9");
  let (minute_x, ..) = stage.bounds("tp-minute-30");
  assert!((minute_x - (hour_x + hour_width + 4.0)).abs() < 0.5);

  // A line of 1 in the border's color, the gap on each side of it, as high as a column.
  let mut stage = picker(&value, |picker| {
    picker.rules(&Recipe::new().base(TimePickerPart::Separator, |s| {
      s.width(1.0).background(PaletteColor::Border)
    }))
  });
  stage.click("tp-trigger");
  stage.pass();
  let lines = lines(&stage);
  assert_eq!(lines.len(), 1, "between the hours and the minutes");
  let line = &lines[0];
  let (hour_x, _, hour_width, _) = stage.bounds("tp-hour-9");
  let (minute_x, ..) = stage.bounds("tp-minute-30");
  assert!(
    (line.x - (hour_x + hour_width + 4.0)).abs() < 0.5,
    "{} {hour_x}",
    line.x
  );
  assert!((minute_x - (line.x + 1.0 + 4.0)).abs() < 0.5, "{minute_x} {}", line.x);
  assert!(
    (line.height - 126.0).abs() < 0.5,
    "as high as a column: {}",
    line.height
  );

  // On a clock of twelve hours there is one before the halves of the day too.
  let mut stage = picker(&value, |picker| {
    picker
      .locale(DateLocale::for_tag("en-US"))
      .rules(&Recipe::new().base(TimePickerPart::Separator, |s| {
        s.width(1.0).background(PaletteColor::Border)
      }))
  });
  stage.click("tp-trigger");
  stage.pass();
  assert_eq!(super::time_picker::lines(&stage).len(), 2);
}
