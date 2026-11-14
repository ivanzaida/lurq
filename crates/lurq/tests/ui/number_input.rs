//! A number input: the board's box with a button at each end, its sizes and layouts, what typing takes and what
//! Enter and the focus leaving put right, what the keys and the buttons step, a press held down, its states, and
//! what a theme's rules and renderer change.

use std::{
  sync::{Arc, Mutex},
  time::{Duration, Instant},
};

use lurq::{
  app::{
    App,
    events::MouseButton,
    theme::{PaletteColor, RadiusSize, ThemePalette},
  },
  components::{Column, Row},
  core::Signal,
  layout::quad::{Quad, QuadContent},
  node::{border::Border, color::Color},
  ui::{
    Field, NumberFormat, NumberInput, NumberInputLayout, NumberInputPart, NumberInputRenderer, NumberInputSize,
    NumberInputView, Recipe,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The width and the color of the edge of the element with `id`.
fn edge(stage: &mut Stage, id: &str) -> (f32, Color) {
  let quad: Quad = stage.border(id).unwrap_or_else(|| panic!("#{id} should have an edge"));
  let side = quad.border.and_then(|border| border.top).expect("an edge on top");
  (side.width, side.color)
}

/// Whether a line of 2 in `color` is drawn inside the box of the element with `id`: the ring of its focus.
fn ring_inside(stage: &mut Stage, id: &str, color: Color) -> bool {
  let (x, y, width, height) = stage.bounds(id);
  stage.quads().into_iter().any(|quad| {
    quad
      .border
      .and_then(|border| border.top)
      .map(|side| (side.width, side.color))
      == Some((2.0, color))
      && quad.x >= x
      && quad.y >= y
      && quad.x + quad.width <= x + width
      && quad.y + quad.height <= y + height
  })
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// Puts `text` in place of the first `typed` bytes of the focused input, as typing over a selection does.
fn type_over(stage: &mut Stage, typed: usize, text: &str) {
  stage.tree.text_input_replace(0..typed, text);
  stage.pass();
}

/// Types `text` at byte `at` of the focused input.
fn type_at(stage: &mut Stage, at: usize, text: &str) {
  stage.tree.text_input_replace(at..at, text);
  stage.pass();
}

/// The middle of the button that steps down, and of the one that steps up, of the split input `id`.
fn buttons(stage: &mut Stage, id: &str) -> ((f32, f32), (f32, f32)) {
  let (x, y, width, height) = stage.bounds(id);
  ((x + 18.0, y + height / 2.0), (x + width - 18.0, y + height / 2.0))
}

fn press(stage: &mut Stage, (x, y): (f32, f32)) {
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

/// The filled rectangles of `color`, as x, y, width and height.
fn fills(stage: &Stage, color: Color) -> Vec<(f32, f32, f32, f32)> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color))
    .map(|quad| (quad.x, quad.y, quad.width, quad.height))
    .collect()
}

/// Takes the focus off the focused element, as a press elsewhere does.
fn leave(stage: &mut Stage) {
  let id = focused(stage).expect("something is focused");
  stage
    .tree
    .get_element_by_id_mut(&id)
    .expect("the focused element")
    .blur();
  stage.pass();
}

fn near(a: f32, b: f32) -> bool {
  (a - b).abs() < 1.0
}

#[test]
fn a_number_input_is_the_boards_box_with_a_button_at_each_end_and_the_number_between() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(NumberInput::new(Signal::new(12.0)).id("qty"))
      .into()
  });
  let (x, y, width, height) = stage.bounds("qty");
  assert_eq!((width, height), (132.0, 36.0));
  assert_eq!(stage.fill_color("qty"), Some(light(PaletteColor::Background)));
  assert_eq!(edge(&mut stage, "qty"), (1.0, light(PaletteColor::Input)));
  assert!(
    stage
      .border("qty")
      .unwrap()
      .border_radius
      .is_some_and(|radius| radius.top_left == 6.0),
    "6 round"
  );
  // The number in the code's type at 14, in the middle of the box.
  assert_eq!(stage.text("12"), (light(PaletteColor::Foreground), 14.0));
  assert_eq!(stage.font_of("12"), stage.app.theme().mono_family().to_string());
  let number = stage.text_quad("12").unwrap();
  assert!(
    (number.x + number.width / 2.0 - (x + width / 2.0)).abs() < 2.0,
    "in the middle: {} of {}",
    number.x + number.width / 2.0 - x,
    width
  );
  // A line of 1 in the color of an input's 36 from each end, the height of the box inside its edge.
  let lines = fills(&stage, light(PaletteColor::Input));
  for at in [x + 35.0, x + width - 36.0] {
    assert!(
      lines
        .iter()
        .any(|&(lx, ly, lw, lh)| near(lx, at) && lw == 1.0 && near(ly, y + 1.0) && near(lh, height - 2.0)),
      "a line at {}: {lines:?}",
      at - x
    );
  }
}

#[test]
fn a_button_under_the_pointer_is_in_the_highlight() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(NumberInput::new(Signal::new(12.0)).id("qty"))
      .into()
  });
  let (x, y, width, _) = stage.bounds("qty");
  let (_, plus) = buttons(&mut stage, "qty");
  stage.tree.mouse_move(plus.0, plus.1);
  stage.pass();
  let lit = fills(&stage, light(PaletteColor::Highlight));
  assert!(
    lit
      .iter()
      .any(|&(lx, ly, lw, _)| near(lx, x + width - 35.0) && near(ly, y + 1.0) && near(lw, 34.0)),
    "the plus is lit: {lit:?}"
  );
}

#[test]
fn the_sizes_are_the_heights_of_the_controls_with_their_type_and_buttons_as_wide() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(NumberInput::new(Signal::new(1.0)).size(NumberInputSize::Sm).id("sm"))
      .child(NumberInput::new(Signal::new(2.0)).id("md"))
      .child(NumberInput::new(Signal::new(3.0)).size(NumberInputSize::Lg).id("lg"))
      .into()
  });
  let lines = fills(&stage, light(PaletteColor::Input));
  for (id, text, height, size) in [
    ("sm", "1", 32.0, 13.0),
    ("md", "2", 36.0, 14.0),
    ("lg", "3", 40.0, 15.0),
  ] {
    let (x, ..) = stage.bounds(id);
    assert_eq!(stage.bounds(id).3, height, "{id}");
    assert_eq!(stage.text(text).1, size, "{id}");
    // The line after the minus: the box's edge and a button as wide as the box is high, less the two lines.
    assert!(
      lines
        .iter()
        .any(|&(lx, _, lw, lh)| near(lx, x + height - 1.0) && lw == 1.0 && near(lh, height - 2.0)),
      "{id}: a line at {}",
      height - 1.0
    );
  }
}

#[test]
fn stacked_buttons_are_at_the_end_and_the_number_at_the_start() {
  let value = Signal::new(4.0);
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(
        NumberInput::new(bound.clone())
          .layout(NumberInputLayout::Stacked)
          .id("stacked"),
      )
      .child(
        NumberInput::new(Signal::new(9.0))
          .layout(NumberInputLayout::Bare)
          .id("bare"),
      )
      .into()
  });
  let (x, y, width, height) = stage.bounds("stacked");
  let number = stage.text_quad("4").unwrap();
  assert!(near(number.x, x + 12.0), "12 in: {}", number.x - x);
  // The upper half of the end steps up, the lower half down.
  press(&mut stage, (x + width - 12.0, y + height / 4.0));
  assert_eq!(value.get(), 5.0);
  press(&mut stage, (x + width - 12.0, y + height * 3.0 / 4.0));
  press(&mut stage, (x + width - 12.0, y + height * 3.0 / 4.0));
  assert_eq!(value.get(), 3.0);
  // A bare one has no lines and no buttons: the number at its start.
  let (bx, by, ..) = stage.bounds("bare");
  assert!(near(stage.text_quad("9").unwrap().x, bx + 12.0));
  assert!(
    fills(&stage, light(PaletteColor::Input))
      .iter()
      .all(|&(_, ly, ..)| (ly - by).abs() > 2.0),
    "no line in the bare one"
  );
}

#[test]
fn typing_takes_digits_a_minus_and_one_separator_and_the_number_is_the_value() {
  let value = Signal::new(0.0);
  let told = Arc::new(Mutex::new(Vec::new()));
  let (bound, tell) = (value.clone(), told.clone());
  let mut stage = Stage::new(move |_| {
    let tell = tell.clone();
    Column::new()
      .padding(8.0)
      .child(
        NumberInput::new(bound.clone())
          .id("n")
          .on_change(move |value| tell.lock().unwrap().push(value)),
      )
      .into()
  });
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("n"));
  type_over(&mut stage, 1, "5");
  assert_eq!(value.get(), 5.0);
  // A letter is not a number being typed.
  type_at(&mut stage, 1, "a");
  assert!(stage.paints("5"));
  assert_eq!(value.get(), 5.0);
  // A minus at the start, where the range has room below nought.
  type_at(&mut stage, 0, "-");
  assert!(stage.paints("-5"));
  assert_eq!(value.get(), -5.0);
  // Whole steps keep no decimals: no separator is taken.
  type_at(&mut stage, 2, ".");
  assert!(stage.paints("-5"));
  assert_eq!(*told.lock().unwrap(), [Some(5.0), Some(-5.0)]);
}

#[test]
fn a_minus_is_not_taken_where_the_range_starts_at_nought() {
  let value = Signal::new(3.0);
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(NumberInput::new(bound.clone()).min(0.0).id("n"))
      .into()
  });
  stage.key("Tab");
  type_at(&mut stage, 0, "-");
  assert!(stage.paints("3"));
  assert_eq!(value.get(), 3.0);
}

#[test]
fn what_is_half_typed_stays_while_the_input_renders_again() {
  let value = Signal::new(0.0);
  let other = Signal::new(0);
  let (bound, read) = (value.clone(), other.clone());
  let mut stage = Stage::new(move |_| {
    // The scene reads another signal too: a change of it renders the input again.
    let _ = read.get();
    Column::new()
      .padding(8.0)
      .child(NumberInput::new(bound.clone()).step(0.1).id("n"))
      .into()
  });
  stage.key("Tab");
  // The step has one decimal: so does the value, and the separator is taken.
  assert!(stage.paints("0.0"));
  type_over(&mut stage, 3, "1");
  assert_eq!(value.get(), 1.0);
  type_at(&mut stage, 1, ".");
  assert!(stage.paints("1."), "the separator stays though 1. is 1");
  other.set(1);
  stage.pass();
  assert!(stage.paints("1."), "and stays when the input renders again");
  assert_eq!(focused(&stage).as_deref(), Some("n"), "which keeps the focus");
  type_at(&mut stage, 2, "5");
  assert_eq!(value.get(), 1.5);
  assert!(stage.paints("1.5"));
}

#[test]
fn enter_and_leaving_put_the_number_right_rounded_and_in_the_range() {
  let value = Signal::new(10.0);
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(NumberInput::new(bound.clone()).min(0.0).max(100.0).step(0.5).id("n"))
      .into()
  });
  stage.key("Tab");
  assert!(stage.paints("10.0"));
  type_over(&mut stage, 4, "150");
  assert_eq!(value.get(), 150.0, "the value as it is typed");
  stage.key("Enter");
  assert_eq!(value.get(), 100.0, "kept in the range");
  assert!(stage.paints("100.0"));
  // Enter leaves a line that is typed in, as it does any; Tab comes back.
  stage.key("Tab");
  type_over(&mut stage, 5, "2.26");
  leave(&mut stage);
  assert_eq!(value.get(), 2.3, "rounded to the step's one decimal");
  assert!(stage.paints("2.3"));
}

#[test]
fn an_emptied_field_is_none_and_not_nought_or_takes_back_its_number() {
  let maybe = Signal::new(Some(7.0));
  let always = Signal::new(7.0);
  let (bound_maybe, bound_always) = (maybe.clone(), always.clone());
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(
        NumberInput::optional(bound_maybe.clone())
          .placeholder("No limit")
          .id("maybe"),
      )
      .child(NumberInput::new(bound_always.clone()).id("always"))
      .into()
  });
  stage.key("Tab");
  type_over(&mut stage, 1, "");
  assert_eq!(maybe.get(), None);
  stage.key("Tab");
  assert!(stage.paints("No limit"), "left empty");
  assert_eq!(maybe.get(), None);
  assert_eq!(focused(&stage).as_deref(), Some("always"));
  type_over(&mut stage, 1, "");
  assert_eq!(always.get(), 7.0);
  leave(&mut stage);
  assert_eq!(always.get(), 7.0);
  assert!(stage.paints("7"), "the number it had");
}

#[test]
fn the_keys_step_page_and_go_to_the_ends_of_the_range() {
  let value = Signal::new(5.0);
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(NumberInput::new(bound.clone()).min(0.0).max(20.0).id("n"))
      .into()
  });
  stage.key("Tab");
  for (key, after) in [
    ("ArrowUp", 6.0),
    ("ArrowDown", 5.0),
    ("PageUp", 15.0),
    ("PageUp", 20.0),
    ("Home", 0.0),
    ("End", 20.0),
    ("PageDown", 10.0),
  ] {
    stage.key(key);
    assert_eq!(value.get(), after, "after {key}");
  }
  assert!(stage.paints("10"));
}

#[test]
fn a_step_from_between_two_steps_goes_to_the_next_one() {
  let value = Signal::new(2.5);
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(NumberInput::new(bound.clone()).precision(1).id("n"))
      .into()
  });
  stage.key("Tab");
  stage.key("ArrowUp");
  assert_eq!(value.get(), 3.0);
  leave(&mut stage);
  value.set(2.5);
  stage.pass();
  stage.key("Tab");
  stage.key("ArrowDown");
  assert_eq!(value.get(), 2.0);
  // Without a least value Home is the caret's.
  stage.key("Home");
  assert_eq!(value.get(), 2.0);
}

#[test]
fn the_buttons_step_and_the_one_toward_an_end_reached_takes_nothing() {
  let value = Signal::new(8.0);
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(NumberInput::new(bound.clone()).min(0.0).max(10.0).id("n"))
      .into()
  });
  let (minus, plus) = buttons(&mut stage, "n");
  press(&mut stage, plus);
  press(&mut stage, plus);
  assert_eq!(value.get(), 10.0);
  assert!(stage.paints("10"));
  press(&mut stage, plus);
  assert_eq!(value.get(), 10.0, "the plus is disabled at the greatest");
  press(&mut stage, minus);
  assert_eq!(value.get(), 9.0);
  // A press of a button leaves the focus where it was: nowhere, here.
  assert_eq!(focused(&stage), None);
}

#[test]
fn a_press_of_a_button_keeps_the_focus_on_the_number_being_typed() {
  let value = Signal::new(1.0);
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(
        NumberInput::new(bound.clone())
          .format(NumberFormat::new().suffix(" kg"))
          .id("n"),
      )
      .into()
  });
  stage.key("Tab");
  let (_, plus) = buttons(&mut stage, "n");
  press(&mut stage, plus);
  assert_eq!(value.get(), 2.0);
  assert_eq!(focused(&stage).as_deref(), Some("n"));
  assert!(stage.paints("2"), "typed as it is, without the unit, while focused");
}

#[test]
fn a_button_held_down_steps_again_and_again_until_it_is_let_go() {
  let value = Signal::new(0.0);
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(NumberInput::new(bound.clone()).max(100.0).id("n"))
      .into()
  });
  let (_, (x, y)) = buttons(&mut stage, "n");
  let pressed = Instant::now();
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.pass();
  assert_eq!(value.get(), 1.0, "a step at the press");
  stage.tree.tick_timers_at(pressed + Duration::from_millis(100));
  assert_eq!(value.get(), 1.0, "not yet");
  stage.tree.tick_timers_at(pressed + Duration::from_millis(450));
  assert_eq!(value.get(), 2.0, "after the first delay");
  stage.tree.tick_timers_at(pressed + Duration::from_millis(520));
  assert_eq!(value.get(), 3.0, "and then on");
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
  assert_eq!(value.get(), 3.0, "the release adds no click");
  stage.tree.tick_timers_at(pressed + Duration::from_secs(5));
  assert_eq!(value.get(), 3.0, "let go, it stops");
}

#[test]
fn the_number_is_written_in_its_format_out_of_the_focus_and_typed_plain_in_it() {
  let value = Signal::new(1234.5);
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(
        NumberInput::new(bound.clone())
          .precision(2)
          .format(NumberFormat::new().prefix("$").grouping(true))
          .layout(NumberInputLayout::Stacked)
          .id("price"),
      )
      .into()
  });
  assert!(stage.paints("$1,234.50"));
  stage.key("Tab");
  assert!(stage.paints("1234.50"));
  stage.key("ArrowUp");
  assert_eq!(value.get(), 1235.0);
  assert!(stage.paints("1235.00"));
  leave(&mut stage);
  assert!(stage.paints("$1,235.00"));
  // What is pasted as it is written is read as a number.
  stage.key("Tab");
  type_over(&mut stage, "1235.00".len(), "$2,000");
  assert_eq!(value.get(), 2000.0);
  assert!(stage.paints("2000.00"));
}

#[test]
fn a_locale_writes_its_own_separators_and_types_its_decimal_one() {
  let value = Signal::new(1234.5);
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(
        NumberInput::new(bound.clone())
          .precision(1)
          .format(NumberFormat::new().separators(',', '.').grouping(true).suffix(" €"))
          .layout(NumberInputLayout::Bare)
          .id("n"),
      )
      .into()
  });
  assert!(stage.paints("1.234,5 €"));
  stage.key("Tab");
  assert!(stage.paints("1234,5"));
  type_over(&mut stage, "1234,5".len(), "7,25");
  assert_eq!(value.get(), 7.25);
  leave(&mut stage);
  assert_eq!(value.get(), 7.3);
  assert!(stage.paints("7,3 €"));
}

#[test]
fn a_controlled_input_tells_each_change_and_shows_what_the_app_gives_it() {
  let held = Signal::new(Some(4.0));
  let bound = held.clone();
  let mut stage = Stage::new(move |_| {
    let set = bound.clone();
    Column::new()
      .padding(8.0)
      .child(
        NumberInput::controlled(bound.get())
          .min(0.0)
          .id("n")
          .on_change(move |value| set.set(value)),
      )
      .into()
  });
  stage.key("Tab");
  stage.key("ArrowUp");
  assert_eq!(held.get(), Some(5.0));
  type_over(&mut stage, 1, "");
  assert_eq!(held.get(), None);
  leave(&mut stage);
  assert_eq!(held.get(), None, "left empty");
  held.set(Some(9.0));
  stage.pass();
  assert!(stage.paints("9"));
}

#[test]
fn the_focus_from_the_keyboard_is_a_ring_inside_the_whole_box() {
  let value = Signal::new(1.0);
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(NumberInput::new(bound.clone()).id("n"))
      .into()
  });
  stage.key("Tab");
  stage.key("ArrowUp");
  assert_eq!(value.get(), 2.0, "the keys reach it");
  assert!(ring_inside(&mut stage, "n", light(PaletteColor::Ring)));
  let (_, _, width, _) = stage.bounds("n");
  assert_eq!(width, 132.0, "the box that has the ring is the whole input");
}

#[test]
fn a_judgment_colors_the_edge_and_a_field_judges_it() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(NumberInput::new(Signal::new(120.0)).invalid(true).id("invalid"))
      .child(NumberInput::new(Signal::new(80.0)).warning(true).id("warning"))
      .child(Field::new("Guests", NumberInput::new(Signal::new(14.0)).id("guests")).error("No more than 12."))
      .into()
  });
  assert_eq!(edge(&mut stage, "invalid"), (1.0, light(PaletteColor::Destructive)));
  assert_eq!(edge(&mut stage, "warning"), (1.0, light(PaletteColor::Warning)));
  assert_eq!(edge(&mut stage, "guests"), (1.0, light(PaletteColor::Destructive)));
}

#[test]
fn a_disabled_input_takes_nothing_and_tab_passes_it_by() {
  let value = Signal::new(5.0);
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(NumberInput::new(bound.clone()).disabled(true).id("disabled"))
      .child(NumberInput::new(Signal::new(0.0)).id("next"))
      .into()
  });
  let (minus, plus) = buttons(&mut stage, "disabled");
  press(&mut stage, plus);
  press(&mut stage, minus);
  assert_eq!(value.get(), 5.0);
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("next"));
}

#[test]
fn a_read_only_input_is_focused_and_its_keys_and_buttons_change_nothing() {
  let value = Signal::new(5.0);
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(NumberInput::new(bound.clone()).read_only(true).id("n"))
      .into()
  });
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("n"));
  stage.key("ArrowUp");
  type_over(&mut stage, 1, "9");
  let (_, plus) = buttons(&mut stage, "n");
  press(&mut stage, plus);
  assert_eq!(value.get(), 5.0);
  assert_eq!(stage.fill_color("n"), Some(light(PaletteColor::Muted)));
}

#[test]
fn the_buttons_are_not_stops_of_tab() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(NumberInput::new(Signal::new(1.0)).id("first"))
      .child(NumberInput::new(Signal::new(2.0)).id("second"))
      .into()
  });
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("first"));
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("second"));
}

/// The board's mobile quantity stepper, as rules over the number input's parts.
fn quantity_stepper() -> Recipe {
  use NumberInputPart::{Buttons, Control, Decrease, Divider, Icon, Increase};
  let edge = || Border::inside(1.0, PaletteColor::Secondary);
  Recipe::new()
    .base(Control, |s| {
      s.height(44.0)
        .radius(RadiusSize::Full)
        .background(PaletteColor::Secondary)
        .border(edge())
    })
    .base(Buttons, |s| s.padding_x(0.0).padding_y(0.0).radius(RadiusSize::Full))
    .base(Decrease, |s| s.width(44.0))
    .base(Increase, |s| s.width(44.0))
    .base(Divider, |s| s.width(0.0))
    .base(Icon, |s| s.square(20.0))
}

#[test]
fn rules_over_its_parts_make_it_the_boards_mobile_quantity_stepper() {
  let value = Signal::new(2.0);
  let bound = value.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(
        NumberInput::new(bound.clone())
          .min(0.0)
          .rules(&quantity_stepper())
          .id("stepper"),
      )
      .into()
  });
  let (x, y, width, height) = stage.bounds("stepper");
  assert_eq!((width, height), (132.0, 44.0));
  assert_eq!(stage.fill_color("stepper"), Some(light(PaletteColor::Secondary)));
  // Its buttons are 44 wide from the ends.
  press(&mut stage, (x + width - 22.0, y + 22.0));
  assert_eq!(value.get(), 3.0);
  press(&mut stage, (x + 22.0, y + 22.0));
  press(&mut stage, (x + 22.0, y + 22.0));
  assert_eq!(value.get(), 1.0);
}

#[test]
fn a_theme_rule_changes_every_number_input() {
  let app = App::new();
  app
    .theme()
    .patch_recipe::<NumberInput>(|rules| rules.base(NumberInputPart::Root, |s| s.width(200.0)));
  let mut stage = Stage::in_app(app, |_| {
    Column::new()
      .padding(8.0)
      .child(NumberInput::new(Signal::new(1.0)).id("n"))
      .into()
  });
  let (x, _, width, _) = stage.bounds("n");
  assert_eq!(width, 200.0);
  // The plus is at the end of the wider box.
  let lines = fills(&stage, light(PaletteColor::Input));
  assert!(lines.iter().any(|&(lx, ..)| near(lx, x + 200.0 - 36.0)), "{lines:?}");
}

#[test]
fn a_theme_renderer_gives_it_another_anatomy_and_it_still_steps() {
  let app = App::new();
  // The control alone: other nodes, the same behavior.
  let renderer: NumberInputRenderer = Arc::new(|view: NumberInputView| view.control().into());
  app.theme().set_renderer::<NumberInput>(renderer);
  let value = Signal::new(1.0);
  let bound = value.clone();
  let mut stage = Stage::in_app(app, move |_| {
    Row::new()
      .padding(8.0)
      .child(
        Column::new()
          .width(200.0)
          .child(NumberInput::new(bound.clone()).id("n")),
      )
      .into()
  });
  assert!(
    fills(&stage, light(PaletteColor::Input)).is_empty(),
    "no buttons, no lines"
  );
  stage.key("Tab");
  stage.key("ArrowUp");
  assert_eq!(value.get(), 2.0);
}
