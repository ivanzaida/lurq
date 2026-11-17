//! A date picker: the board's field, the calendar it opens under it from its button and from Down, a choice closing it
//! with the focus back in the field, what typing sets and what the focus leaving puts right, the button that empties
//! it, a field around it, a disabled one, and what tooling is told.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    ctx::Placement,
    events::TextInputEvent,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Button, Column},
  core::{Date, DateLocale, DateStyle, Signal},
  layout::{Constraints, Size, quad::QuadContent},
  node::color::Color,
  ui::{DateMemory, DatePicker, DatePickerPart, DatePickerSize, Field},
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
  // The panel is the popover's: the calendar draws no fill of its own over it. The panel has no padding, so the
  // calendar's box is the panel's, and the one fill at it is the panel's.
  assert_eq!(stage.bounds("dp-calendar"), stage.bounds("dp-panel"));
  assert_eq!(stage.fill_color("dp-panel"), Some(light(PaletteColor::Popover)));
  let (x, y, width, height) = stage.bounds("dp-calendar");
  let fills = stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if color.a() > 0)
        && (quad.x, quad.y, quad.width, quad.height) == (x, y, width, height)
    })
    .count();
  assert_eq!(fills, 1, "the panel's fill alone");
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
  // A stop after the field, for Tab to leave it to: alone, the field is the stop Tab comes back to.
  let held_value = value.clone();
  let mut stage = Stage::new(move |ctx| {
    Column::new()
      .padding(10.0)
      .spacing(8.0)
      .child(
        DatePicker::new(held_value.clone())
          .today(day(11, 9))
          .id("dp")
          .mount(ctx),
      )
      .child(Button::empty().size(40.0, 20.0).tab_index(0).id("after"))
      .into()
  });
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
  assert_eq!(focused(&stage).as_deref(), Some("after"));
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
  // Outside a form Enter takes the focus out of a field of one line, as from every input: back into it.
  stage.click("dp");
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
  let (shown, memory) = (value.clone(), DateMemory::new());
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

#[test]
fn a_choice_of_the_chosen_day_closes_it_and_tells_nothing() {
  let value = Signal::new(Some(day(11, 12)));
  let told = Arc::new(Mutex::new(Vec::new()));
  let heard = told.clone();
  let mut stage = picker(&value, move |picker| {
    let heard = heard.clone();
    picker.on_change(move |date| heard.lock().unwrap().push(date))
  });
  stage.click("dp-trigger");
  stage.pass();
  stage.click("dp-calendar-2026-11-12");
  stage.pass();
  assert!(!opened(&mut stage));
  assert!(told.lock().unwrap().is_empty());
}

#[test]
fn a_choice_closes_the_calendar_before_its_handler_runs() {
  let value = Signal::new(Some(day(11, 12)));
  let memory = DateMemory::new();
  let seen = Arc::new(Mutex::new(Vec::new()));
  let (kept, heard) = (memory.clone(), seen.clone());
  let mut stage = picker(&value, move |picker| {
    let (asked, heard) = (kept.clone(), heard.clone());
    picker
      .memory(&kept)
      .on_change(move |_| heard.lock().unwrap().push(asked.is_open()))
  });
  stage.click("dp-trigger");
  stage.pass();
  stage.click("dp-calendar-2026-11-20");
  assert_eq!(*seen.lock().unwrap(), vec![false], "closed when the handler runs");
}

#[test]
fn a_controlled_picker_shows_what_the_app_answers() {
  // The app holds the date and takes any but the 20th.
  let held = Signal::new(Some(day(11, 12)));
  let memory = DateMemory::new();
  let (app, kept) = (held.clone(), memory.clone());
  let mut stage = Stage::new(move |_| {
    let answer = app.clone();
    Column::new()
      .size(400.0, 500.0)
      .padding(10.0)
      .child(
        DatePicker::controlled(app.get(), move |date| {
          if date != Some(day(11, 20)) {
            answer.set(date);
          }
        })
        .today(day(11, 9))
        .memory(&kept)
        .id("dp"),
      )
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 500.0))));
  stage.pass();
  stage.click("dp-trigger");
  stage.pass();
  stage.click("dp-calendar-2026-11-20");
  stage.pass();
  assert!(!memory.is_open());
  assert_eq!(held.get_untracked(), Some(day(11, 12)));
  assert!(stage.paints("Nov 12, 2026"), "what the app holds");
  stage.click("dp-trigger");
  stage.pass();
  stage.click("dp-calendar-2026-11-18");
  stage.pass();
  assert_eq!(held.get_untracked(), Some(day(11, 18)));
  assert!(stage.paints("Nov 18, 2026"));
}

#[test]
fn an_edit_is_heard_first_and_enter_submits() {
  let value = Signal::new(Some(day(11, 12)));
  let (edits, submitted) = (Arc::new(Mutex::new(Vec::new())), Arc::new(Mutex::new(Vec::new())));
  let (heard, sent) = (edits.clone(), submitted.clone());
  let mut stage = picker(&value, move |picker| {
    let (heard, sent) = (heard.clone(), sent.clone());
    picker
      .on_input(move |event: &TextInputEvent| {
        heard.lock().unwrap().push(event.new_value().to_owned());
        // No letter x.
        if event.new_value().contains('x') {
          event.prevent_default();
        }
      })
      .on_submit(move |date| sent.lock().unwrap().push(date))
  });
  stage.click("dp");
  type_over(&mut stage, "Nov 12, 2026".len(), "Dec 1, 2026x");
  assert_eq!(value.get_untracked(), Some(day(11, 12)), "refused, and not read");
  type_over(&mut stage, "Nov 12, 2026".len(), "Dec 1, 2026");
  assert_eq!(value.get_untracked(), Some(day(12, 1)));
  assert_eq!(
    *edits.lock().unwrap(),
    vec!["Dec 1, 2026x".to_owned(), "Dec 1, 2026".to_owned()]
  );
  stage.key("Enter");
  assert_eq!(*submitted.lock().unwrap(), vec![Some(day(12, 1))]);
}

#[test]
fn opening_closing_and_dismissing_are_told() {
  let value = Signal::new(Some(day(11, 12)));
  let (changes, dismissed) = (Arc::new(Mutex::new(Vec::new())), Arc::new(Mutex::new(0)));
  let (heard, gone) = (changes.clone(), dismissed.clone());
  let mut stage = picker(&value, move |picker| {
    let (heard, gone) = (heard.clone(), gone.clone());
    picker
      .on_open_change(move |open| heard.lock().unwrap().push(open))
      .on_dismiss(move || *gone.lock().unwrap() += 1)
  });
  stage.click("dp-trigger");
  stage.pass();
  stage.key("Escape");
  stage.pass();
  assert!(!opened(&mut stage));
  assert_eq!(*changes.lock().unwrap(), vec![true, false]);
  assert_eq!(*dismissed.lock().unwrap(), 1);
  // A choice closes it too, and is no dismissal.
  stage.click("dp-trigger");
  stage.pass();
  stage.click("dp-calendar-2026-11-20");
  stage.pass();
  assert_eq!(*changes.lock().unwrap(), vec![true, false, true, false]);
  assert_eq!(*dismissed.lock().unwrap(), 1);
}

#[test]
fn placement_and_offset_move_the_panel() {
  let value = Signal::new(Some(day(11, 12)));
  let mut stage = picker(&value, |picker| picker.placement(Placement::BottomEnd).offset(12.0));
  stage.click("dp-trigger");
  stage.pass();
  let (x, y, width, height) = stage.bounds("dp");
  let (px, py, pwidth, _) = stage.bounds("dp-panel");
  assert_eq!(px + pwidth, x + width, "its end under the field's end");
  assert!((py - (y + height + 12.0)).abs() < 0.5, "12 under it");
}

#[test]
fn inline_the_panel_is_drawn_in_place() {
  let value = Signal::new(Some(day(11, 12)));
  let mut stage = picker(&value, |picker| picker.inline(true));
  let (x, y, _, height) = stage.bounds("dp");
  let (px, py, pwidth, _) = stage.bounds("dp-panel");
  assert_eq!((px, pwidth), (x, 248.0));
  assert!((py - (y + height + 4.0)).abs() < 0.5, "4 under the field");
  assert_eq!(stage.fill_color("dp-panel"), Some(light(PaletteColor::Popover)));
  assert_eq!(focused(&stage), None, "shown, it takes no focus");
  stage.click("dp-calendar-2026-11-20");
  assert_eq!(value.get_untracked(), Some(day(11, 20)));
  stage.bounds("dp-panel");
}

#[test]
fn its_control_part_styles_the_box_and_its_sizes_are_an_inputs() {
  let value = Signal::new(Some(day(11, 12)));
  let mut stage = picker(&value, |picker| {
    picker
      .size(DatePickerSize::Lg)
      .style(DatePickerPart::Control, |s| s.background(PaletteColor::Muted))
  });
  assert_eq!(stage.bounds("dp").3, 40.0);
  assert_eq!(stage.fill_color("dp"), Some(light(PaletteColor::Muted)));
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn its_words_can_be_said_otherwise() {
  let value = Signal::new(Some(day(11, 12)));
  let mut stage = picker(&value, |picker| {
    picker
      .clearable(true)
      .trigger_label("Календар")
      .clear_label("Очистити")
      .panel_label("Оберіть дату")
      .next_label("Далі")
  });
  let said = |stage: &mut Stage, id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(said(&mut stage, "dp-trigger", "a11y_name").as_deref(), Some("Календар"));
  assert_eq!(said(&mut stage, "dp-clear", "a11y_name").as_deref(), Some("Очистити"));
  stage.click("dp-trigger");
  stage.pass();
  assert_eq!(
    said(&mut stage, "dp-panel", "a11y_name").as_deref(),
    Some("Оберіть дату")
  );
  assert_eq!(
    said(&mut stage, "dp-calendar-next", "a11y_name").as_deref(),
    Some("Далі")
  );
}

#[test]
fn an_apps_own_date_text_writes_and_reads_the_field() {
  let value = Signal::new(Some(day(11, 12)));
  let mut stage = picker(&value, |picker| picker.locale(super::calendar::Roman));
  assert!(stage.paints("12.XI.2026"));
  stage.click("dp");
  type_over(&mut stage, "12.XI.2026".len(), "3.XII.2026");
  assert_eq!(value.get_untracked(), Some(day(12, 3)));
  stage.click("dp-trigger");
  stage.pass();
  assert!(stage.paints("XII 2026"), "its calendar too");
}

#[test]
fn the_apps_date_locale_reaches_a_mounted_picker_and_a_change_of_it_renders_it_again() {
  let app = lurq::app::App::new();
  app.set_date_locale(DateLocale::for_tag("uk"));
  let value = Signal::new(Some(day(11, 12)));
  let shown = value.clone();
  let mut stage = Stage::in_app(app.clone(), move |ctx| {
    Column::new()
      .padding(10.0)
      .child(DatePicker::new(shown.clone()).today(day(11, 9)).id("dp").mount(ctx))
      .into()
  });
  assert!(stage.paints("12.11.2026"));
  app.set_date_locale(DateLocale::english().with_style(DateStyle::Long));
  stage.pass();
  assert!(stage.paints("November 12, 2026"));
  // Its own wins over the app's.
  let own = value.clone();
  let stage = Stage::in_app(app, move |ctx| {
    Column::new()
      .child(
        DatePicker::new(own.clone())
          .locale(DateLocale::for_tag("de"))
          .id("dp")
          .mount(ctx),
      )
      .into()
  });
  assert!(stage.paints("12.11.2026"));
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn its_locale_names_its_controls_and_its_props_win() {
  let value = Signal::new(Some(day(11, 12)));
  let mut stage = picker(&value, |picker| {
    picker.clearable(true).locale(DateLocale::for_tag("uk"))
  });
  let said = |stage: &mut Stage, id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(
    said(&mut stage, "dp-trigger", "a11y_name").as_deref(),
    Some("Виберіть дату")
  );
  assert_eq!(said(&mut stage, "dp-clear", "a11y_name").as_deref(), Some("Очистити"));
  let mut stage = picker(&value, |picker| {
    picker.locale(DateLocale::for_tag("uk")).trigger_label("Календар")
  });
  assert_eq!(said(&mut stage, "dp-trigger", "a11y_name").as_deref(), Some("Календар"));
}

#[test]
fn under_a_finger_a_tap_of_the_field_opens_the_calendar_and_asks_for_no_keyboard() {
  let value = Signal::new(Some(day(11, 12)));
  let held = value.clone();
  let app = lurq::app::App::new();
  app.set_pointer(lurq::app::Pointer::Coarse);
  let mut stage = Stage::in_app(app, move |ctx| {
    Column::new()
      .size(400.0, 500.0)
      .padding(10.0)
      .child(DatePicker::new(held.clone()).today(day(11, 9)).id("dp").mount(ctx))
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 500.0))));
  stage.pass();
  assert!(!opened(&mut stage));
  // In the field, well clear of its button.
  let (x, y, width, height) = stage.bounds("dp");
  stage.tap_at(x + width - 20.0, y + height / 2.0);
  assert!(opened(&mut stage), "the whole field opens it");
  // Closed, the focus is the field's again, which asks for no keyboard on the screen: a date is chosen.
  stage.key("Escape");
  assert!(!opened(&mut stage));
  assert_eq!(focused(&stage).as_deref(), Some("dp"), "the field has the focus");
  assert!(stage.tree.keyboard_traits().is_none(), "and no keyboard comes up");
}
