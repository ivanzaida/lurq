//! A month picker and a year picker: the board's panel, heading and grid, a choice, the year (or the years) before and
//! after, the keys, the range, and what tooling is told.

use std::sync::{Arc, Mutex};

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::Column,
  core::{DateLocale, Signal, YearMonth},
  node::color::Color,
  ui::{MonthPicker, YearPicker},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn month(year: i32, month: u8) -> YearMonth {
  YearMonth::new(year, month).unwrap()
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// A month picker with the id `mp`, mounted, this month March 2026.
fn months(value: &Signal<YearMonth>, make: impl Fn(MonthPicker) -> MonthPicker + Send + Sync + 'static) -> Stage {
  let value = value.clone();
  Stage::new(move |ctx| {
    Column::new()
      .padding(10.0)
      .child(make(MonthPicker::new(value.clone()).today(month(2026, 3)).id("mp")).mount(ctx))
      .into()
  })
}

/// A year picker with the id `yp`, mounted, this year 2024.
fn years(value: &Signal<i32>, make: impl Fn(YearPicker) -> YearPicker + Send + Sync + 'static) -> Stage {
  let value = value.clone();
  Stage::new(move |ctx| {
    Column::new()
      .padding(10.0)
      .child(make(YearPicker::new(value.clone()).today(2024).id("yp")).mount(ctx))
      .into()
  })
}

#[test]
fn a_month_picker_is_the_panel_and_the_grid_of_the_design() {
  let value = Signal::new(month(2026, 11));
  let mut stage = months(&value, |picker| picker);
  let (x, y, width, height) = stage.bounds("mp");
  assert_eq!(width, 252.0);
  // 12 of padding, a heading of 28, 8, three rows of 34 8 apart.
  assert!(
    (height - (24.0 + 28.0 + 8.0 + 3.0 * 34.0 + 2.0 * 8.0)).abs() < 0.5,
    "{height}"
  );
  assert_eq!(stage.fill_color("mp"), Some(light(PaletteColor::Popover)));
  assert_eq!(stage.text("2026"), (light(PaletteColor::Foreground), 14.0));
  let (cx, cy, cwidth, cheight) = stage.bounds("mp-2026-01");
  assert_eq!((cx, cheight), (x + 12.0, 34.0));
  assert!(
    (cwidth - (252.0 - 24.0 - 12.0) / 4.0).abs() < 0.5,
    "four to a row, 4 apart"
  );
  assert!((cy - (y + 12.0 + 28.0 + 8.0)).abs() < 0.5);
  assert_eq!(stage.text("Jan"), (light(PaletteColor::Foreground), 13.0));
  // The chosen month on the primary fill, this month on the highlight.
  assert_eq!(stage.fill_color("mp-2026-11"), Some(light(PaletteColor::Primary)));
  assert_eq!(stage.text("Nov").0, light(PaletteColor::PrimaryForeground));
  assert_eq!(stage.fill_color("mp-2026-03"), Some(light(PaletteColor::Highlight)));
  assert_eq!(stage.fill_color("mp-2026-04"), None);
  let (nx, _, nwidth, _) = stage.bounds("mp-next");
  assert_eq!((nx + nwidth, nwidth), (x + 240.0, 16.0));
}

#[test]
fn a_press_chooses_a_month_and_the_buttons_show_another_year() {
  let value = Signal::new(month(2026, 11));
  let told = Arc::new(Mutex::new(Vec::new()));
  let heard = told.clone();
  let mut stage = months(&value, move |picker| {
    let heard = heard.clone();
    picker.on_change(move |chosen| heard.lock().unwrap().push(chosen))
  });
  stage.click("mp-2026-11");
  assert!(told.lock().unwrap().is_empty(), "chosen already");
  stage.click("mp-2026-02");
  assert_eq!(value.get_untracked(), month(2026, 2));
  stage.click("mp-next");
  assert!(stage.paints("2027"));
  assert_eq!(value.get_untracked(), month(2026, 2), "the year shown is not the value");
  stage.click("mp-2027-05");
  assert_eq!(*told.lock().unwrap(), vec![month(2026, 2), month(2027, 5)]);
}

#[test]
fn the_keys_move_by_a_month_a_row_and_a_year() {
  let value = Signal::new(month(2026, 11));
  let mut stage = months(&value, |picker| picker);
  stage.key("Tab");
  stage.key("Tab");
  stage.key("Tab");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("mp-2026-11"),
    "Tab enters at the chosen month"
  );
  stage.key("ArrowUp");
  assert_eq!(focused(&stage).as_deref(), Some("mp-2026-07"));
  stage.key("End");
  assert_eq!(focused(&stage).as_deref(), Some("mp-2026-08"));
  stage.key("ArrowRight");
  assert_eq!(focused(&stage).as_deref(), Some("mp-2026-09"));
  stage.key("PageDown");
  assert_eq!(focused(&stage).as_deref(), Some("mp-2027-09"));
  assert!(stage.paints("2027"));
  stage.key("Enter");
  assert_eq!(value.get_untracked(), month(2027, 9));
}

#[test]
fn a_month_out_of_the_range_cannot_be_chosen() {
  let value = Signal::new(month(2026, 11));
  let mut stage = months(&value, |picker| picker.min(month(2026, 6)).max(month(2026, 12)));
  stage.click("mp-2026-02");
  assert_eq!(value.get_untracked(), month(2026, 11));
  assert_eq!(
    stage.text("Feb").0,
    light(PaletteColor::MutedForeground),
    "muted, and at half opacity"
  );
  stage.click("mp-next");
  assert!(stage.paints("2026"), "no year after the greatest month's");
}

#[test]
fn an_optional_month_picker_holds_none_until_a_choice() {
  let value = Signal::new(None);
  let shown = value.clone();
  let mut stage = Stage::new(move |ctx| {
    Column::new()
      .padding(10.0)
      .child(
        MonthPicker::optional(shown.clone())
          .today(month(2026, 3))
          .locale(DateLocale::for_tag("uk"))
          .id("mp")
          .mount(ctx),
      )
      .into()
  });
  assert!(stage.paints("Бер"));
  stage.click("mp-2026-04");
  assert_eq!(value.get_untracked(), Some(month(2026, 4)));
}

#[test]
fn a_year_picker_shows_twelve_years_from_a_decade() {
  let value = Signal::new(2026);
  let told = Arc::new(Mutex::new(Vec::new()));
  let heard = told.clone();
  let mut stage = years(&value, move |picker| {
    let heard = heard.clone();
    picker.on_change(move |year| heard.lock().unwrap().push(year))
  });
  assert!(stage.paints("2020 – 2031"));
  assert_eq!(stage.bounds("yp").2, 252.0);
  assert_eq!(stage.fill_color("yp-2026"), Some(light(PaletteColor::Primary)));
  assert_eq!(stage.fill_color("yp-2024"), Some(light(PaletteColor::Highlight)));
  stage.bounds("yp-2031");
  stage.click("yp-next");
  assert!(stage.paints("2030 – 2041"));
  stage.click("yp-2035");
  assert_eq!(value.get_untracked(), 2035);
  // PageUp goes ten years.
  stage.key("PageUp");
  assert_eq!(focused(&stage).as_deref(), Some("yp-2025"));
  assert!(stage.paints("2020 – 2031"));
  assert_eq!(*told.lock().unwrap(), vec![2035]);
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn tooling_reads_a_grid_of_months() {
  let value = Signal::new(month(2026, 11));
  let mut stage = months(&value, |picker| picker.label("Report month"));
  let said = |stage: &mut Stage, id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(said(&mut stage, "mp", "a11y_name").as_deref(), Some("Report month"));
  assert_eq!(said(&mut stage, "mp-2026-11", "a11y_role").as_deref(), Some("gridcell"));
  assert_eq!(
    said(&mut stage, "mp-2026-11", "a11y_name").as_deref(),
    Some("November 2026")
  );
  assert_eq!(said(&mut stage, "mp-2026-11", "a11y_selected").as_deref(), Some("true"));
  assert_eq!(said(&mut stage, "mp-2026-03", "a11y_current").as_deref(), Some("date"));
  assert_eq!(said(&mut stage, "mp-next", "a11y_name").as_deref(), Some("Next year"));
}
