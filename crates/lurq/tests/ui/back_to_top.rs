//! A back to top button: that it shows once its content is scrolled past
//! its threshold and goes as the content comes back, the board's numbers, a
//! press that takes the content to its top over the frames that follow and at
//! once while motion is reduced, a handler that refuses, the keyboard, the
//! forced and the still forms, a disabled one, what tooling reads, and a
//! theme's rules and anatomy.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App,
    ctx::Ctx,
    events::MouseEvent,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, ScrollVertical, Spacer, Stack},
  layout::{StackAlignment, layout_kind::ScrollState},
  node::{Element, color::Color},
  ui::{BackToTop, BackToTopPart, BackToTopRenderer, BackToTopView, icons},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The character the theme's icon font draws `name` with.
fn glyph(name: &str) -> String {
  icons::ALL
    .iter()
    .find(|(held, _)| *held == name)
    .map(|(_, glyph)| glyph.to_string())
    .unwrap_or_else(|| panic!("no icon {name}"))
}

/// A page of 400 by 300 over 2000 of content, with `button` 16 in from its bottom end corner.
fn page(scroll: &ScrollState, button: Element) -> Element {
  Stack::new()
    .size(400.0, 300.0)
    .stack_align(StackAlignment::BottomEnd)
    .child(
      ScrollVertical::new(Spacer::new().width(400.0).height(2000.0))
        .with_scroll_state(scroll.clone())
        .size(400.0, 300.0),
    )
    .child(Column::new().padding(16.0).overflow_visible().child(button))
    .into()
}

/// The page with the button `make` builds, mounted.
fn mounted(app: App, scroll: &ScrollState, make: impl Fn(&ScrollState) -> BackToTop + Send + Sync + 'static) -> Stage {
  let scroll = scroll.clone();
  Stage::in_app(app, move |ctx: &mut Ctx| {
    page(&scroll, make(&scroll).id("top").mount(ctx))
  })
}

fn there(stage: &mut Stage) -> bool {
  stage.tree.get_element_by_id_mut("top").is_some()
}

#[test]
fn it_shows_once_the_content_is_scrolled_past_its_threshold_and_goes_as_it_comes_back() {
  let scroll = ScrollState::new();
  let mut stage = mounted(App::new(), &scroll, BackToTop::new);
  assert!(!there(&mut stage), "nothing at the top");

  scroll.set_scroll(0.0, 200.0);
  stage.pass();
  assert!(!there(&mut stage), "200 is not past 300");

  scroll.set_scroll(0.0, 301.0);
  stage.pass();
  assert!(there(&mut stage), "past 300 it shows");

  scroll.set_scroll(0.0, 100.0);
  stage.pass();
  stage.pass();
  assert!(!there(&mut stage), "back above the threshold it has faded out and gone");
}

#[test]
fn a_threshold_of_the_apps_own_shows_it_sooner() {
  let scroll = ScrollState::new();
  let mut stage = mounted(App::new(), &scroll, |scroll| BackToTop::new(scroll).threshold(40.0));
  scroll.set_scroll(0.0, 41.0);
  stage.pass();
  assert!(there(&mut stage));
}

#[test]
fn it_is_a_circle_of_40_on_the_background_with_an_edge_and_an_arrow_of_16() {
  let scroll = ScrollState::new();
  let mut stage = mounted(App::new(), &scroll, BackToTop::new);
  scroll.set_scroll(0.0, 500.0);
  stage.pass();
  let (x, y, width, height) = stage.bounds("top");
  assert_eq!((width, height), (40.0, 40.0));
  assert_eq!((x + width, y + height), (384.0, 284.0), "where the app put it");
  assert_eq!(stage.fill_color("top"), Some(light(PaletteColor::Background)));
  let fill = stage.fill("top").unwrap();
  assert!(
    fill.border_radius.is_some_and(|radius| radius.top_left >= 20.0),
    "round"
  );
  let edge = stage.border("top").expect("an edge");
  let side = edge.border.and_then(|border| border.top).expect("a top side");
  assert_eq!((side.width, side.color), (1.0, light(PaletteColor::Border)));
  assert_eq!(
    stage.text(&glyph(icons::ARROW_UP)),
    (light(PaletteColor::Foreground), 16.0)
  );
  let arrow = stage.text_quad(&glyph(icons::ARROW_UP)).unwrap();
  assert!((arrow.x + 8.0 - (x + 20.0)).abs() < 0.01, "in its middle");
}

#[test]
fn a_press_takes_the_content_to_its_top_over_the_frames_that_follow_and_the_button_goes() {
  let scroll = ScrollState::new();
  let mut stage = mounted(App::new(), &scroll, BackToTop::new);
  scroll.set_scroll(0.0, 900.0);
  stage.pass();
  // A frame at the clock's own time: the press that follows is at that time, and each frame after it a sixtieth
  // of a second on.
  stage.frame();
  let (x, y) = stage.center("top");
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, lurq::app::events::MouseButton::Left);
  stage.tree.mouse_up(x, y, lurq::app::events::MouseButton::Left);
  stage.frame();
  stage.frame();
  let on_its_way = scroll.scroll_y();
  assert!(
    0.0 < on_its_way && on_its_way < 900.0,
    "neither where it was nor at its top yet: {on_its_way}"
  );
  assert_eq!(scroll.scroll_target().1, 0.0, "it is going to its top");
  stage.pass();
  stage.pass();
  assert_eq!(scroll.scroll_y(), 0.0);
  assert!(!there(&mut stage), "at the top the button is gone");
}

#[test]
fn while_motion_is_reduced_a_press_takes_the_content_to_its_top_at_once() {
  let app = App::new();
  app.theme().set_reduced_motion(Some(true));
  let scroll = ScrollState::new();
  let mut stage = mounted(app, &scroll, BackToTop::new);
  scroll.set_scroll(0.0, 900.0);
  stage.pass();
  let (x, y) = stage.center("top");
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, lurq::app::events::MouseButton::Left);
  stage.tree.mouse_up(x, y, lurq::app::events::MouseButton::Left);
  stage.frame();
  stage.frame();
  assert_eq!(scroll.scroll_y(), 0.0, "no frame shows it on its way");
}

#[test]
fn a_handler_hears_the_press_first_and_one_that_refuses_keeps_the_content_where_it_is() {
  let heard = Arc::new(AtomicUsize::new(0));
  let scroll = ScrollState::new();
  let count = heard.clone();
  let mut stage = mounted(App::new(), &scroll, move |scroll| {
    let (first, second) = (count.clone(), count.clone());
    BackToTop::new(scroll)
      .on_click(move |_| {
        first.fetch_add(1, Ordering::SeqCst);
      })
      .on_click(move |event: MouseEvent| {
        second.fetch_add(10, Ordering::SeqCst);
        event.prevent_default();
      })
  });
  scroll.set_scroll(0.0, 900.0);
  stage.pass();
  stage.click("top");
  stage.pass();
  assert_eq!(
    heard.load(Ordering::SeqCst),
    11,
    "both handlers, the second after the first"
  );
  assert_eq!(scroll.scroll_y(), 900.0, "refused: the content stays");
  assert!(there(&mut stage));
}

#[test]
fn tab_reaches_it_while_it_shows_and_enter_presses_it() {
  let scroll = ScrollState::new();
  let mut stage = mounted(App::new(), &scroll, BackToTop::new);
  stage.key("Tab");
  assert!(
    stage.tree.focused_element().is_none(),
    "no stop of Tab while it is gone"
  );

  scroll.set_scroll(0.0, 900.0);
  stage.pass();
  stage.key("Tab");
  let focused = stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned));
  assert_eq!(focused.as_deref(), Some("top"));
  assert!(stage.has_ring(light(PaletteColor::Ring)), "the button's ring");
  stage.key("Enter");
  stage.pass();
  assert_eq!(scroll.scroll_y(), 0.0, "Enter takes the content to its top");
}

#[test]
fn it_is_shown_or_hidden_whatever_the_scroll_when_forced() {
  let scroll = ScrollState::new();
  let mut stage = mounted(App::new(), &scroll, |scroll| {
    BackToTop::new(scroll).force_shown(Some(true))
  });
  assert!(there(&mut stage), "shown at the top");

  let scroll = ScrollState::new();
  let mut stage = mounted(App::new(), &scroll, |scroll| {
    BackToTop::new(scroll).force_shown(Some(false))
  });
  scroll.set_scroll(0.0, 900.0);
  stage.pass();
  assert!(!there(&mut stage), "never shown");
}

#[test]
fn converted_into_an_element_it_is_always_there_and_a_press_jumps_to_the_top() {
  let scroll = ScrollState::new();
  let held = scroll.clone();
  let mut stage = Stage::new(move |_| page(&held, BackToTop::new(&held).id("top").into()));
  assert!(there(&mut stage), "the still form shows at the top too");
  scroll.set_scroll(0.0, 900.0);
  stage.pass();
  stage.click("top");
  assert_eq!(scroll.scroll_y(), 0.0);
}

#[test]
fn a_disabled_one_takes_no_press() {
  let scroll = ScrollState::new();
  let mut stage = mounted(App::new(), &scroll, |scroll| BackToTop::new(scroll).disabled(true));
  scroll.set_scroll(0.0, 900.0);
  stage.pass();
  stage.click("top");
  stage.pass();
  assert_eq!(scroll.scroll_y(), 900.0);
  stage.key("Tab");
  assert!(stage.tree.focused_element().is_none(), "and is no stop of Tab");
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn tooling_reads_a_button_called_back_to_top_or_by_its_label() {
  for (label, name) in [
    (None, "Back to top"),
    (Some("Up to the first message"), "Up to the first message"),
  ] {
    let scroll = ScrollState::new();
    let mut stage = mounted(App::new(), &scroll, move |scroll| {
      let button = BackToTop::new(scroll).force_shown(Some(true));
      match label {
        Some(label) => button.label(label),
        None => button,
      }
    });
    let said = stage
      .tree
      .get_element_by_id_mut("top")
      .and_then(|element| element.description("a11y_name"));
    assert_eq!(said.as_deref(), Some(name));
  }
}

#[test]
fn a_theme_restyles_it_or_gives_it_another_anatomy() {
  let app = App::new();
  app.theme().patch_recipe::<BackToTop>(|rules| {
    rules
      .base(BackToTopPart::Button, |s| {
        s.square(48.0).background(PaletteColor::Primary)
      })
      .base(BackToTopPart::Icon, |s| s.square(20.0))
  });
  let scroll = ScrollState::new();
  let mut stage = mounted(app, &scroll, |scroll| BackToTop::new(scroll).icon(icons::CHEVRON_UP));
  scroll.set_scroll(0.0, 900.0);
  stage.pass();
  assert_eq!(stage.bounds("top").2, 48.0);
  assert_eq!(stage.fill_color("top"), Some(light(PaletteColor::Primary)));
  assert_eq!(
    stage.text(&glyph(icons::CHEVRON_UP)).1,
    20.0,
    "an icon of the app's own"
  );

  // A renderer that draws the button's name over it; when it shows and what a press does stay the component's.
  let app = App::new();
  let renderer: BackToTopRenderer = Arc::new(|view: BackToTopView| {
    Column::new()
      .overflow_visible()
      .child(lurq::components::Text::new(&*view.label))
      .child(view.button().build(view.theme()))
      .into()
  });
  app.theme().set_renderer::<BackToTop>(renderer);
  let scroll = ScrollState::new();
  let mut stage = mounted(app, &scroll, BackToTop::new);
  assert!(!there(&mut stage));
  assert!(!stage.paints("Back to top"));
  scroll.set_scroll(0.0, 900.0);
  stage.pass();
  assert!(stage.paints("Back to top"));
  stage.click("top");
  stage.pass();
  assert_eq!(scroll.scroll_y(), 0.0);
}

/// Whether the button goes at once or fades out, as the frames after the content came back above the threshold
/// show: still drawn a frame after it was taken out, and gone once its fade has run.
fn fades_out(stage: &mut Stage, scroll: &ScrollState, drawn: impl Fn(&mut Stage) -> bool) {
  scroll.set_scroll(0.0, 900.0);
  stage.pass();
  assert!(drawn(stage), "shown past the threshold");
  scroll.set_scroll(0.0, 0.0);
  // The frame that lays the content out at its top, the frame that takes the button out, and one a sixtieth of a
  // second into its fade.
  stage.frame();
  stage.frame();
  stage.frame();
  assert!(drawn(stage), "it is still drawn while it fades out");
  stage.pass();
  stage.pass();
  assert!(!drawn(stage), "and gone once it has faded");
}

#[test]
fn it_fades_out_as_it_goes_whoever_draws_it() {
  // The default one: the button in the root that fades.
  let scroll = ScrollState::new();
  let mut stage = mounted(App::new(), &scroll, BackToTop::new);
  fades_out(&mut stage, &scroll, |stage| stage.paints(&glyph(icons::ARROW_UP)));

  // A theme's renderer whose root is not the button, and one that draws no button at all: the fade is the
  // component's, in the root it puts what they return in.
  let app = App::new();
  let renderer: BackToTopRenderer = Arc::new(|view: BackToTopView| {
    Column::new()
      .overflow_visible()
      .child(lurq::components::Text::new("To the top"))
      .child(view.button().build(view.theme()))
      .into()
  });
  app.theme().set_renderer::<BackToTop>(renderer);
  let scroll = ScrollState::new();
  let mut stage = mounted(app, &scroll, BackToTop::new);
  fades_out(&mut stage, &scroll, |stage| stage.paints("To the top"));

  let app = App::new();
  let renderer: BackToTopRenderer = Arc::new(|_: BackToTopView| lurq::components::Text::new("Scrolled").into());
  app.theme().set_renderer::<BackToTop>(renderer);
  let scroll = ScrollState::new();
  let mut stage = mounted(app, &scroll, BackToTop::new);
  fades_out(&mut stage, &scroll, |stage| stage.paints("Scrolled"));
}

#[test]
fn a_rule_of_the_root_changes_how_it_comes_and_goes() {
  use lurq::{
    app::theme::MotionEasing,
    ui::{FADE_OUT, PartAnimation, TransitionDuration, TransitionEasing},
  };
  // With a fade of a millisecond the button is gone by the frame after the one that takes it out.
  let app = App::new();
  app.theme().patch_recipe::<BackToTop>(|rules| {
    rules.base(BackToTopPart::Root, |s| {
      s.exit(PartAnimation {
        keyframes: FADE_OUT,
        duration: TransitionDuration::Set(std::time::Duration::from_millis(1)),
        easing: TransitionEasing::Role(MotionEasing::Exit),
      })
    })
  });
  let scroll = ScrollState::new();
  let mut stage = mounted(app, &scroll, BackToTop::new);
  scroll.set_scroll(0.0, 900.0);
  stage.pass();
  assert!(there(&mut stage));
  scroll.set_scroll(0.0, 0.0);
  stage.frame();
  stage.frame();
  stage.frame();
  assert!(!there(&mut stage), "no fade: the theme's rule for the root");
}
