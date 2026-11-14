//! A date picker: the board's field, the calendar it opens under it from its button and from Down, a choice closing it
//! with the focus back in the field, what typing sets and what the focus leaving puts right, the button that empties
//! it, a field around it, a disabled one, and what tooling is told.

use std::sync::{Arc, Mutex};

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::Column,
  core::{Date, DateLocale, Signal},
  layout::{Constraints, Size},
  node::color::Color,
  ui::{CalendarMemory, DatePicker, Field},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn day(month: u8, day: u8) -> Date {
  Date::new(2026, month, day).unwrap()
}

/// A picker with the id `dp`, mounted at the top of a window of 400 by 500 that has room for its calendar under it,
/// with today the 9th of November 2026.
fn picker(value: &Signal<Option<Date>>, make: impl Fn(DatePicker) -> DatePicker + Send + Sync + 'static) -> Stage {
  let value = value.clone();
  let mut stage = Stage::new(move |ctx| {
    Column::new()
      .size(400.0, 500.0)
      .padding(10.0)
      .child(make(DatePicker::new(value.clone()).today(day(11, 9)).id("dp")).mount(ctx))
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 500.0))));
  stage.pass();
  stage
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// Puts `text` in place of what the focused field holds, `held` bytes of it.
fn type_over(stage: &mut Stage, held: usize, text: &str) {
  stage.tree.text_input_replace(0..held, text);
  stage.pass();
}

fn opened(stage: &mut Stage) -> bool {
  stage.tree.get_element_by_id_mut("dp-panel").is_some()
}

#[test]
fn a_date_picker_is_the_input_of_the_design_with_its_date() {
  let value = Signal::new(Some(day(11, 12)));
  let mut stage = picker(&value, |picker| picker);
  let (x, _, width, height) = stage.bounds("dp");
  assert_eq!((width, height), (260.0, 36.0));
  assert_eq!(stage.text("Nov 12, 2026"), (light(PaletteColor::Foreground), 14.0));
  // The date 8 after a glyph of 16, 12 from the edge.
  let text = stage.text_quad("Nov 12, 2026").unwrap();
  assert!((text.x - (x + 12.0 + 16.0 + 8.0)).abs() < 0.5, "{}", text.x - x);
  // The button around the glyph, 24 square, centred on it.
  let (tx, _, twidth, theight) = stage.bounds("dp-trigger");
  assert_eq!((twidth, theight), (24.0, 24.0));
  assert_eq!(tx, x + 8.0);
  assert!(!opened(&mut stage));

  let empty = Signal::new(None);
  let stage = picker(&empty, |picker| picker.placeholder("Pick a date"));
  assert_eq!(stage.text("Pick a date").0, light(PaletteColor::MutedForeground));
}

#[test]
fn its_button_opens_the_calendar_under_it_at_the_chosen_day() {
  let value = Signal::new(Some(day(11, 12)));
  let mut stage = picker(&value, |picker| picker);
  stage.click("dp-trigger");
  stage.pass();
  assert!(opened(&mut stage));
  let (x, y, _, height) = stage.bounds("dp");
  let (cx, cy, cwidth, _) = stage.bounds("dp-calendar");
  assert_eq!(
    (cx, cwidth),
    (x, 248.0),
    "the calendar is the panel, as wide as its days"
  );
  assert!((cy - (y + height + 4.0)).abs() < 0.5, "4 under the field");
  assert!(stage.paints("November 2026"));
  assert_eq!(focused(&stage).as_deref(), Some("dp-calendar-2026-11-12"));
  // The panel is the popover's: the calendar draws no fill of its own over it.
  assert_eq!(stage.fill_color("dp-calendar"), None);
  assert_eq!(stage.fill_color("dp-panel"), Some(light(PaletteColor::Popover)));
}

#[test]
fn a_choice_closes_it_and_the_focus_goes_back_to_the_field() {
  let value = Signal::new(Some(day(11, 12)));
  let told = Arc::new(Mutex::new(Vec::new()));
  let heard = told.clone();
  let mut stage = picker(&value, move |picker| {
    let heard = heard.clone();
    picker.on_change(move |date| heard.lock().unwrap().push(date))
  });
  stage.click("dp-trigger");
  stage.pass();
  stage.key("ArrowRight");
  stage.key("Enter");
  stage.pass();
  assert_eq!(value.get_untracked(), Some(day(11, 13)));
  assert_eq!(*told.lock().unwrap(), vec![Some(day(11, 13))]);
  assert!(!opened(&mut stage));
  assert_eq!(focused(&stage).as_deref(), Some("dp"));
  assert!(stage.paints("Nov 13, 2026"));

  // A press of a day chooses it too.
  stage.key("ArrowDown");
  stage.pass();
  assert!(opened(&mut stage), "Down in the field opens it");
  stage.click("dp-calendar-2026-11-20");
  stage.pass();
  assert_eq!(value.get_untracked(), Some(day(11, 20)));
  assert!(!opened(&mut stage));
}

#[test]
fn escape_closes_it_and_changes_nothing() {
  let value = Signal::new(Some(day(11, 12)));
  let mut stage = picker(&value, |picker| picker);
  stage.click("dp");
  stage.key("ArrowDown");
  stage.pass();
  assert!(opened(&mut stage));
  stage.key("ArrowRight");
  stage.key("Escape");
  stage.pass();
  assert!(!opened(&mut stage));
  assert_eq!(value.get_untracked(), Some(day(11, 12)));
  assert_eq!(focused(&stage).as_deref(), Some("dp"));
}

#[test]
fn what_is_typed_is_the_date_as_soon_as_it_is_one() {
  let value = Signal::new(Some(day(11, 12)));
  let mut stage = picker(&value, |picker| picker);
  stage.click("dp");
  let held = "Nov 12, 2026".len();
  type_over(&mut stage, held, "Dec 1, 20");
  assert_eq!(value.get_untracked(), Some(day(11, 12)), "half typed is no date yet");
  assert!(stage.paints("Dec 1, 20"), "and the field keeps it");
  type_over(&mut stage, "Dec 1, 20".len(), "Dec 1, 2026");
  assert_eq!(value.get_untracked(), Some(day(12, 1)));
  // Other forms are read too, and the focus leaving writes it as the picker does.
  type_over(&mut stage, "Dec 1, 2026".len(), "2026-12-24");
  assert_eq!(value.get_untracked(), Some(day(12, 24)));
  stage.key("Tab");
  stage.pass();
  assert!(stage.paints("Dec 24, 2026"));
}

#[test]
fn what_is_no_date_takes_back_the_one_it_had_and_empty_is_none() {
  let value = Signal::new(Some(day(11, 12)));
  let mut stage = picker(&value, |picker| picker.max(day(12, 31)));
  stage.click("dp");
  type_over(&mut stage, "Nov 12, 2026".len(), "someday");
  stage.key("Enter");
  assert!(stage.paints("Nov 12, 2026"), "Enter puts it right");
  type_over(&mut stage, "Nov 12, 2026".len(), "Jan 5, 2027");
  assert_eq!(value.get_untracked(), Some(day(11, 12)), "after the greatest date");
  type_over(&mut stage, "Jan 5, 2027".len(), "");
  assert_eq!(value.get_untracked(), None);
}

#[test]
fn the_app_writes_and_reads_its_own_form() {
  let value = Signal::new(Some(day(11, 12)));
  let stage = picker(&value, |picker| {
    picker.format(|date| date.to_string()).parse(|text| text.parse().ok())
  });
  assert!(stage.paints("2026-11-12"));
  let mut stage = picker(&value, |picker| picker.locale(DateLocale::for_tag("de")));
  assert!(stage.paints("12.11.2026"));
  stage.click("dp");
  type_over(&mut stage, "12.11.2026".len(), "03.12.2026");
  assert_eq!(value.get_untracked(), Some(day(12, 3)));
}

#[test]
fn a_clearable_picker_empties_with_its_button() {
  let value = Signal::new(Some(day(11, 12)));
  let mut stage = picker(&value, |picker| picker.clearable(true));
  let (x, _, width, _) = stage.bounds("dp");
  let (cx, _, cwidth, _) = stage.bounds("dp-clear");
  assert_eq!(cwidth, 24.0);
  assert_eq!(cx + cwidth, x + width - 8.0, "its cross 12 from the end");
  stage.click("dp-clear");
  assert_eq!(value.get_untracked(), None);
  assert!(
    stage.tree.get_element_by_id_mut("dp-clear").is_none(),
    "nothing to empty"
  );
}

#[test]
fn a_disabled_picker_opens_nothing() {
  let value = Signal::new(Some(day(11, 12)));
  let mut stage = picker(&value, |picker| picker.disabled(true));
  stage.click("dp-trigger");
  stage.pass();
  assert!(!opened(&mut stage));
  let mut stage = picker(&value, |picker| picker.read_only(true));
  stage.click("dp");
  stage.key("ArrowDown");
  stage.pass();
  assert!(!opened(&mut stage));
}

#[test]
fn a_field_judges_it_and_its_memory_keeps_it_open() {
  let value = Signal::new(Some(day(11, 12)));
  let (shown, memory) = (value.clone(), CalendarMemory::new());
  let kept = memory.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .size(400.0, 500.0)
      .child(
        Field::new(
          "Due date",
          DatePicker::new(shown.clone()).today(day(11, 9)).memory(&kept).id("dp"),
        )
        .error("Too late"),
      )
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 500.0))));
  stage.pass();
  let edge = stage
    .border("dp")
    .and_then(|quad| quad.border)
    .and_then(|border| border.top);
  assert_eq!(edge.map(|side| side.color), Some(light(PaletteColor::Destructive)));
  stage.click("dp-trigger");
  stage.pass();
  assert!(memory.is_open());
  stage.click("dp-calendar-next");
  assert!(stage.paints("December 2026"), "the memory keeps the month shown");
  stage.click("dp-calendar-2026-12-03");
  assert_eq!(value.get_untracked(), Some(day(12, 3)));
  assert!(!memory.is_open());
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn tooling_reads_a_combobox_that_opens_a_dialog() {
  let value = Signal::new(Some(day(11, 12)));
  let mut stage = picker(&value, |picker| picker.label("Due date"));
  let said = |stage: &mut Stage, id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(said(&mut stage, "dp", "a11y_role").as_deref(), Some("combobox"));
  assert_eq!(said(&mut stage, "dp", "a11y_haspopup").as_deref(), Some("dialog"));
  assert_eq!(said(&mut stage, "dp", "a11y_expanded").as_deref(), Some("false"));
  assert_eq!(said(&mut stage, "dp", "a11y_name").as_deref(), Some("Due date"));
  stage.click("dp-trigger");
  stage.pass();
  assert_eq!(said(&mut stage, "dp", "a11y_expanded").as_deref(), Some("true"));
  assert_eq!(said(&mut stage, "dp-panel", "a11y_role").as_deref(), Some("dialog"));
}
