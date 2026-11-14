//! A field the app mounts (`Field::mount`): each control that keeps state of its own between renders is mounted by
//! it, and is the same control in the field as outside it, with the field's label, description and error.

use std::path::PathBuf;

use lurq::{
  app::{
    Accelerator,
    ctx::Ctx,
    events::{DraggedPaths, DroppedPath, MouseButton},
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, SelectOption},
  core::{Date, DateLocale, Signal, Time, YearMonth},
  layout::quad::QuadContent,
  node::{Element, color::Color},
  ui::{
    DateField, DatePicker, DateRangePicker, DropZone, Field, FileUpload, KeyPlatform, Listbox, ListboxPart,
    MonthPicker, PasswordInput, Rating, ShortcutRecorder, TimeField, TimePicker, YearPicker, icons,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// A window of 600 by 600 with `field`, mounted where it is declared, in a column.
fn mounted(field: impl Fn() -> Field + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::new(move |ctx: &mut Ctx| -> Element {
    Column::new()
      .size(600.0, 600.0)
      .padding(10.0)
      .child(field().mount(ctx))
      .into()
  });
  stage.tree.resize(600, 600);
  stage.pass();
  stage
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

fn press(stage: &mut Stage, keys: &str) {
  for key in keys.chars() {
    stage.key(&key.to_string());
  }
}

fn press_at(stage: &mut Stage, (x, y): (f32, f32)) {
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

fn exists(stage: &mut Stage, id: &str) -> bool {
  stage.tree.get_element_by_id_mut(id).is_some()
}

fn shortcut(text: &str) -> Accelerator {
  Accelerator::parse(text).unwrap()
}

#[test]
fn a_shortcut_recorder_records_in_a_mounted_field_with_the_fields_words() {
  let held = Signal::new(shortcut("Ctrl+K"));
  let bound = held.clone();
  let mut stage = mounted(move || {
    Field::new(
      "Command palette",
      ShortcutRecorder::new(bound.clone())
        .platform(KeyPlatform::Standard)
        .validate(|pressed: &Accelerator| match pressed.to_string().as_str() {
          "Ctrl+S" => Err("Already used by Save".to_owned()),
          _ => Ok(()),
        })
        .id("recorder"),
    )
    .description("Opens the palette from anywhere.")
  });
  assert!(stage.paints("Command palette") && stage.paints("Opens the palette from anywhere."));
  stage.click("recorder");
  assert!(stage.paints("Press shortcut…"), "it records");
  // A modifier held shows while it records.
  stage
    .tree
    .key_down("Control".to_owned(), "ControlLeft".to_owned(), false, true, false);
  stage.pass();
  assert!(stage.paints("Ctrl") && !stage.paints("Press shortcut…"));
  stage
    .tree
    .key_down("p".to_owned(), "KeyP".to_owned(), true, true, false);
  stage.pass();
  assert_eq!(held.get_untracked(), shortcut("Ctrl+Shift+P"));
  stage
    .tree
    .key_up("Control".to_owned(), "ControlLeft".to_owned(), false, false, false);
  // A combination the app refuses is not taken, and says why.
  stage.click("recorder");
  stage
    .tree
    .key_down("s".to_owned(), "KeyS".to_owned(), false, true, false);
  stage.pass();
  assert_eq!(held.get_untracked(), shortcut("Ctrl+Shift+P"));
  assert!(stage.paints("Already used by Save"));
}

#[test]
fn a_shortcut_recorder_in_a_mounted_field_with_an_error_is_invalid() {
  let mut stage = mounted(|| {
    Field::new(
      "Save",
      ShortcutRecorder::controlled(shortcut("Ctrl+S"), |_| {})
        .platform(KeyPlatform::Standard)
        .id("recorder"),
    )
    .error("Taken by the system.")
  });
  assert!(stage.paints("Save") && stage.paints("Taken by the system."));
  let edge = stage
    .border("recorder")
    .and_then(|quad| quad.border)
    .and_then(|border| border.top)
    .map(|side| side.color);
  assert_eq!(edge, Some(light(PaletteColor::Destructive)));
  stage.click("recorder");
  assert!(stage.paints("Press shortcut…"), "and records");
}

#[test]
fn a_date_field_in_a_mounted_field_keeps_what_is_half_typed() {
  let value = Signal::new(Date::new(2026, 4, 3).unwrap());
  let bound = value.clone();
  let mut stage = mounted(move || {
    Field::new(
      "Due",
      DateField::new(bound.clone())
        .locale(DateLocale::for_tag("en-GB"))
        .id("df"),
    )
  });
  stage.click("df-year");
  press(&mut stage, "202");
  assert!(stage.paints("202"), "the digits as typed");
  assert_eq!(value.get_untracked(), Date::new(2026, 4, 3).unwrap());
  press(&mut stage, "7");
  assert_eq!(value.get_untracked(), Date::new(2027, 4, 3).unwrap());
}

#[test]
fn a_time_field_in_a_mounted_field_keeps_the_first_digit_of_an_hour() {
  let value = Signal::new(Time::new(9, 30, 0).unwrap());
  let bound = value.clone();
  let mut stage = mounted(move || Field::new("Start", TimeField::new(bound.clone()).id("tf")));
  stage.click("tf-hour");
  press(&mut stage, "14");
  assert_eq!(value.get_untracked(), Time::new(14, 30, 0).unwrap());
  assert_eq!(focused(&stage).as_deref(), Some("tf-minute"));
}

#[test]
fn a_date_picker_in_a_mounted_field_opens_its_calendar() {
  let value = Signal::new(None);
  let bound = value.clone();
  let mut stage = mounted(move || Field::new("Due", DatePicker::new(bound.clone()).id("dp")));
  stage.click("dp-trigger");
  stage.pass();
  assert!(exists(&mut stage, "dp-panel"));
}

#[test]
fn a_date_range_picker_in_a_mounted_field_opens_its_calendar() {
  let value = Signal::new(None);
  let bound = value.clone();
  let mut stage = mounted(move || Field::new("Stay", DateRangePicker::new(bound.clone()).id("rp")));
  stage.click("rp-trigger");
  stage.pass();
  assert!(exists(&mut stage, "rp-panel"));
}

#[test]
fn a_time_picker_in_a_mounted_field_opens_its_panel() {
  let value = Signal::new(Time::new(9, 30, 0).unwrap());
  let bound = value.clone();
  let mut stage = mounted(move || Field::new("Start", TimePicker::new(bound.clone()).id("tp")));
  stage.click("tp-trigger");
  stage.pass();
  assert!(exists(&mut stage, "tp-panel"));
}

#[test]
fn a_month_picker_in_a_mounted_field_keeps_the_year_it_shows() {
  let value = Signal::new(YearMonth::new(2026, 2).unwrap());
  let bound = value.clone();
  let mut stage = mounted(move || Field::new("Month", MonthPicker::new(bound.clone()).id("mp")));
  stage.click("mp-next");
  assert!(stage.paints("2027"));
  stage.click("mp-2027-05");
  assert_eq!(value.get_untracked(), YearMonth::new(2027, 5).unwrap());
}

#[test]
fn a_year_picker_in_a_mounted_field_keeps_the_years_it_shows() {
  let value = Signal::new(2026);
  let bound = value.clone();
  let mut stage = mounted(move || Field::new("Year", YearPicker::new(bound.clone()).id("yp")));
  assert!(stage.paints("2020 – 2031"));
  stage.click("yp-next");
  assert!(stage.paints("2030 – 2041"));
  stage.click("yp-2035");
  assert_eq!(value.get_untracked(), 2035);
}

#[test]
fn a_password_input_in_a_mounted_field_turns_its_eye() {
  let mut stage = mounted(|| {
    Field::new(
      "Password",
      PasswordInput::new(Signal::new("hunter2".to_owned())).id("password"),
    )
  });
  let painted = |stage: &Stage| {
    stage
      .quads()
      .into_iter()
      .any(|quad| matches!(quad.content, QuadContent::Text { ref text, .. } if text.contains("hunter2")))
  };
  assert!(!painted(&stage), "masked at first");
  let (x, y, width, height) = stage.bounds("password");
  press_at(&mut stage, (x + width - 20.0, y + height / 2.0));
  assert!(painted(&stage), "the eye showed it");
}

#[test]
fn a_rating_in_a_mounted_field_lights_the_stars_under_the_pointer() {
  let held = Signal::new(1.0_f64);
  let bound = held.clone();
  let mut stage = mounted(move || Field::new("Quality", Rating::new(bound.clone()).id("rating")));
  let star = icons::ALL
    .iter()
    .find_map(|(name, glyph)| (*name == icons::STAR_FILL).then(|| glyph.to_string()))
    .unwrap();
  let lit = |stage: &Stage| {
    stage
      .quads()
      .into_iter()
      .filter(|quad| {
        matches!(&quad.content, QuadContent::Text { text, style, .. }
          if *text == star && style.color == light(PaletteColor::Warning))
      })
      .count()
  };
  assert_eq!(lit(&stage), 1);
  let (x, y, _, height) = stage.bounds("rating");
  // The middle of the fourth star: 20 wide, 4 apart.
  stage.tree.mouse_move(x + 3.0 * 24.0 + 10.0, y + height / 2.0);
  stage.pass();
  assert_eq!(lit(&stage), 4, "a preview of what a press sets");
  assert_eq!(held.get_untracked(), 1.0);
}

fn dragged(names: &[&str]) -> DraggedPaths {
  DraggedPaths::new(
    names
      .iter()
      .map(|name| DroppedPath::with_kind(PathBuf::from(name), false)),
  )
}

#[test]
fn a_file_upload_in_a_mounted_field_shows_a_drag_over_its_zone() {
  let files = Signal::new(Vec::new());
  let bound = files.clone();
  let mut stage = mounted(move || Field::new("Photos", FileUpload::new(bound.clone()).zone(true).id("upload")));
  let (x, y) = stage.center("upload-trigger");
  stage.tree.file_drag_enter(dragged(&["a.png"]), x, y);
  stage.pass();
  assert_eq!(stage.fill_color("upload-trigger"), Some(light(PaletteColor::InfoMuted)));
}

#[test]
fn a_drop_zone_in_a_mounted_field_shows_a_drag_over_it() {
  let mut stage = mounted(|| Field::new("Attachments", DropZone::new().accept(["png"]).id("zone")));
  let (x, y) = stage.center("zone");
  stage.tree.file_drag_enter(dragged(&["photo.png"]), x, y);
  stage.pass();
  assert_eq!(stage.tree.file_drag_accepted(), Some(true));
  assert_eq!(stage.fill_color("zone"), Some(light(PaletteColor::InfoMuted)));
}

#[test]
fn a_listbox_of_thousands_in_a_mounted_field_builds_the_options_in_view() {
  let chosen = Signal::new(0u32);
  let bound = chosen.clone();
  let mut stage = mounted(move || {
    Field::new(
      "Options",
      Listbox::new(
        bound.clone(),
        (0..5000u32).map(|i| SelectOption::new(i, format!("Option {i}"))),
      )
      .style(ListboxPart::Root, |s| s.height(240.0))
      .id("big"),
    )
  });
  // A virtual list: the options near the view are built, and no other.
  assert!(exists(&mut stage, "big-0"));
  assert!(!exists(&mut stage, "big-4000"), "an option far down is not built");
  stage.key("Tab");
  stage.key("End");
  stage.pass();
  stage.pass();
  assert_eq!(chosen.get_untracked(), 4999);
  assert_eq!(focused(&stage).as_deref(), Some("big-4999"));
}
