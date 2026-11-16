//! The design system's carousel: slides between two round buttons, dots under them, turned by the buttons, the
//! dots, the keys, a drag and by itself.

use std::{
  sync::{Arc, Mutex},
  time::{Duration, Instant},
};

use lurq::{
  app::{
    events::MouseButton,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row, Text},
  core::Signal,
  node::color::Color,
  ui::Carousel,
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// A slide as the board draws one: a card of 240 by 140 with its number in it.
fn slide(number: usize) -> Column {
  Column::new().height(140.0).child(Text::new(&number.to_string()))
}

fn slides(count: usize) -> Vec<Column> {
  (1..=count).map(slide).collect()
}

/// A carousel in a window of 600 by 400, 8 in from its corner: 336 wide, as the board draws it.
fn on_stage(carousel: impl Fn() -> Carousel + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::first_frame(600.0, 400.0, move |_| {
    Column::new()
      .size(600.0, 400.0)
      .padding(8.0)
      .child(carousel().width(336.0).id("c"))
      .into()
  });
  stage.pass();
  stage
}

/// Where a slide is across the window.
fn slide_x(stage: &mut Stage, at: usize) -> f32 {
  stage.bounds(&format!("c-slide-{at}")).0
}

#[test]
fn it_is_the_slide_between_two_round_buttons_and_the_dots_under_it() {
  let mut stage = on_stage(|| Carousel::new(slides(4)));
  assert_eq!(stage.bounds("c"), (8.0, 8.0, 336.0, 140.0 + 12.0 + 12.0));
  let (x, y, width, height) = stage.bounds("c-previous");
  assert_eq!((x, width, height), (8.0, 36.0, 36.0));
  assert_eq!(y, 8.0 + 52.0, "in the middle of the slide's height");
  assert_eq!(stage.bounds("c-viewport"), (8.0 + 48.0, 8.0, 240.0, 140.0));
  assert_eq!(stage.bounds("c-next").0, 8.0 + 48.0 + 252.0);
  assert_eq!(slide_x(&mut stage, 0), 56.0);
  assert_eq!(
    stage.bounds("c-slide-0").2,
    240.0 + 12.0,
    "a slide's room has the gap after it"
  );

  // Four dots of 6, 6 apart, 12 under the slides, in the middle.
  let marks: Vec<_> = (0..4).map(|at| stage.bounds(&format!("c-dot-{at}-mark"))).collect();
  for (at, (x, y, width, height)) in marks.iter().enumerate() {
    assert_eq!((*width, *height), (6.0, 6.0));
    assert_eq!(*y, 8.0 + 140.0 + 12.0 + 3.0);
    assert_eq!(*x, 8.0 + 168.0 - 21.0 + at as f32 * 12.0);
  }
  assert_eq!(stage.fill_color("c-dot-0-mark"), Some(light(PaletteColor::Primary)));
  assert_eq!(stage.fill_color("c-dot-1-mark"), Some(light(PaletteColor::Input)));
}

#[test]
fn the_buttons_turn_to_the_slide_before_and_after() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let tell = told.clone();
  let mut stage = on_stage(move || {
    let tell = tell.clone();
    Carousel::new(slides(3)).on_index_change(move |to| tell.lock().unwrap().push(to))
  });
  // At the first there is none before.
  stage.click("c-previous");
  assert!(told.lock().unwrap().is_empty());

  stage.click("c-next");
  stage.pass();
  assert_eq!(slide_x(&mut stage, 1), 56.0, "the second is shown");
  assert_eq!(stage.fill_color("c-dot-1-mark"), Some(light(PaletteColor::Primary)));
  stage.click("c-next");
  stage.pass();
  assert_eq!(slide_x(&mut stage, 2), 56.0);
  // At the last there is none after.
  stage.click("c-next");
  stage.click("c-previous");
  stage.pass();
  assert_eq!(slide_x(&mut stage, 1), 56.0);
  assert_eq!(*told.lock().unwrap(), [1, 2, 1]);
}

#[test]
fn with_wrap_the_first_comes_after_the_last() {
  let index = Signal::new(0);
  let shown = index.clone();
  let mut stage = on_stage(move || Carousel::new(slides(3)).wrap(true).index(shown.clone()));
  stage.click("c-previous");
  stage.pass();
  assert_eq!(index.get(), 2);
  assert_eq!(slide_x(&mut stage, 2), 56.0);
  stage.click("c-next");
  stage.pass();
  assert_eq!(index.get(), 0);
  assert_eq!(slide_x(&mut stage, 0), 56.0);
}

#[test]
fn a_dot_turns_to_its_slide_and_the_app_turns_it_by_its_signal() {
  let index = Signal::new(0);
  let shown = index.clone();
  let mut stage = on_stage(move || Carousel::new(slides(4)).index(shown.clone()));
  stage.click("c-dot-3");
  stage.pass();
  assert_eq!(index.get(), 3);
  assert_eq!(slide_x(&mut stage, 3), 56.0);

  index.set(1);
  stage.pass();
  stage.pass();
  assert_eq!(slide_x(&mut stage, 1), 56.0);
}

#[test]
fn several_in_view_share_the_room_and_turn_by_one() {
  let index = Signal::new(0);
  let shown = index.clone();
  let mut stage = on_stage(move || Carousel::new(slides(5)).per_view(2).index(shown.clone()));
  // Two of (240 - 12) / 2, 12 apart.
  assert_eq!(stage.bounds("c-slide-0").2, 126.0);
  assert_eq!(slide_x(&mut stage, 1), 56.0 + 126.0);
  // Four places: the last two are shown together.
  assert!(stage.tree.get_element_by_id("c-dot-3").is_some());
  assert!(stage.tree.get_element_by_id("c-dot-4").is_none());
  stage.click("c-next");
  stage.pass();
  assert_eq!(slide_x(&mut stage, 1), 56.0);
  stage.click("c-dot-3");
  stage.pass();
  assert_eq!(slide_x(&mut stage, 3), 56.0);
  assert_eq!(slide_x(&mut stage, 4), 56.0 + 126.0);
}

#[test]
fn the_arrow_keys_on_a_button_turn_as_it_does() {
  let index = Signal::new(0);
  let shown = index.clone();
  let mut stage = on_stage(move || Carousel::new(slides(3)).index(shown.clone()));
  // The button before takes no focus at the first slide: Tab goes to the one after.
  stage.key("Tab");
  stage.key("ArrowRight");
  stage.pass();
  assert_eq!(index.get(), 1);
  stage.key("ArrowRight");
  assert_eq!(index.get(), 2);
  stage.key("ArrowLeft");
  assert_eq!(index.get(), 1);
}

#[test]
fn a_drag_of_the_mouse_carries_the_slides_to_the_next() {
  let index = Signal::new(0);
  let shown = index.clone();
  let mut stage = on_stage(move || Carousel::new(slides(3)).index(shown.clone()));
  let (x, y) = stage.center("c-viewport");
  // Not far enough: it goes back.
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_move(x - 20.0, y);
  stage.tree.mouse_move(x - 30.0, y);
  stage.tree.mouse_up(x - 30.0, y, MouseButton::Left);
  stage.pass();
  stage.pass();
  assert_eq!(index.get(), 0);
  assert_eq!(slide_x(&mut stage, 0), 56.0);
  // Far enough: on to the next.
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_move(x - 40.0, y);
  stage.tree.mouse_move(x - 80.0, y);
  stage.tree.mouse_up(x - 80.0, y, MouseButton::Left);
  stage.pass();
  stage.pass();
  assert_eq!(index.get(), 1);
  assert_eq!(slide_x(&mut stage, 1), 56.0);
}

#[test]
fn it_turns_by_itself_until_a_person_turns_it() {
  let index = Signal::new(0);
  let shown = index.clone();
  let mut stage = on_stage(move || {
    Carousel::new(slides(3))
      .autoplay(Duration::from_secs(3))
      .index(shown.clone())
  });
  let start = Instant::now();
  stage.tree.tick_timers_at(start + Duration::from_secs(4));
  stage.pass();
  assert_eq!(index.get(), 1);
  stage.tree.tick_timers_at(start + Duration::from_secs(7));
  stage.pass();
  assert_eq!(index.get(), 2);
  // From the last to the first.
  stage.tree.tick_timers_at(start + Duration::from_secs(10));
  stage.pass();
  assert_eq!(index.get(), 0);

  stage.click("c-dot-2");
  stage.tree.tick_timers_at(start + Duration::from_secs(20));
  stage.pass();
  assert_eq!(index.get(), 2, "a person turned it");
}

#[test]
fn it_is_no_wider_than_a_room_narrower_than_it() {
  let mut stage = Stage::first_frame(600.0, 400.0, |_| {
    Column::new()
      .size(600.0, 400.0)
      .child(
        Row::new()
          .width(200.0)
          .child(Carousel::new(slides(3)).width(336.0).id("c")),
      )
      .into()
  });
  stage.pass();
  assert_eq!(stage.bounds("c").2, 200.0);
  assert_eq!(stage.bounds("c-viewport").2, 200.0 - 96.0);
  assert_eq!(stage.bounds("c-next").0 + 36.0, 200.0);
  stage.click("c-next");
  stage.pass();
  assert_eq!(slide_x(&mut stage, 1), 48.0, "a slide is as wide as the room shows");
}

#[test]
fn a_disabled_carousel_takes_nothing() {
  let index = Signal::new(0);
  let shown = index.clone();
  let mut stage = on_stage(move || Carousel::new(slides(3)).disabled(true).index(shown.clone()));
  stage.click("c-next");
  stage.click("c-dot-2");
  assert_eq!(index.get(), 0);
}

#[test]
fn one_slide_has_no_dots_and_its_buttons_take_nothing() {
  let mut stage = on_stage(|| Carousel::new(slides(1)));
  assert!(stage.tree.get_element_by_id("c-dot-0").is_none());
  assert_eq!(stage.bounds("c").3, 140.0);
}

#[test]
fn a_sideways_scroll_comes_to_rest_on_a_slide() {
  use lurq::app::events::ScrollPhase;
  let index = Signal::new(0);
  let shown = index.clone();
  let mut stage = on_stage(move || Carousel::new(slides(3)).index(shown.clone()));
  let (x, y) = stage.center("c-viewport");
  // A trackpad: its fingers' way, and the end of it.
  stage.tree.scroll(x, y, -150.0, 0.0, ScrollPhase::Start);
  stage.tree.scroll(x, y, 0.0, 0.0, ScrollPhase::End);
  stage.pass();
  stage.pass();
  assert_eq!(index.get(), 1);
  assert_eq!(slide_x(&mut stage, 1), 56.0);

  // A mouse's wheel turned sideways, which tells of no end: it comes to rest a moment after it stops.
  stage.tree.wheel(x, y, -100.0, 0.0, ScrollPhase::Scroll);
  stage.pass();
  stage.tree.tick_timers_at(Instant::now() + Duration::from_secs(1));
  stage.pass();
  stage.pass();
  assert_eq!(index.get(), 1, "100 of 252 is nearer the slide it was at");
  assert_eq!(slide_x(&mut stage, 1), 56.0);
  stage.tree.wheel(x, y, -200.0, 0.0, ScrollPhase::Scroll);
  stage.pass();
  stage.tree.tick_timers_at(Instant::now() + Duration::from_secs(2));
  stage.pass();
  stage.pass();
  assert_eq!(index.get(), 2);
  assert_eq!(slide_x(&mut stage, 2), 56.0);
}
