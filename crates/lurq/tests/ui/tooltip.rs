//! A tooltip: its box against the board, where it stands beside what it
//! describes, what shows it and what puts it away, its delay and the delay it
//! skips, and a theme's rules and renderer.

use std::{
  sync::Arc,
  time::{Duration, Instant},
};

use lurq::{
  app::{
    App,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  layout::quad::{Quad, QuadContent},
  node::color::Color,
  ui::{Button, ButtonVariant, Tooltip, TooltipPart, TooltipRenderer, TooltipSide, TooltipView},
};

use super::stage::Stage;

const WORDS: &str = "Copy link";

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The button the tooltips describe: outlined, so that its fill is not the tooltip's.
fn copy() -> Button {
  Button::new("Copy").variant(ButtonVariant::Outline).id("copy")
}

/// A tooltip with no delay on the button, 100 in from the window's corner.
fn stage(tooltip: impl Fn() -> Tooltip + Send + Sync + 'static) -> Stage {
  Stage::new(move |ctx| Column::new().padding(100.0).child(tooltip().mount(ctx, copy())).into())
}

fn rect_color(quad: &Quad) -> Option<Color> {
  match quad.content {
    QuadContent::Rect { color, .. } if color.a() > 0 => Some(color),
    _ => None,
  }
}

/// The tooltip's box: the smallest fill around its text.
fn bubble(stage: &Stage, text: &str) -> Option<Quad> {
  let words = stage.text_quad(text)?;
  stage
    .quads()
    .into_iter()
    .filter(|quad| {
      rect_color(quad).is_some()
        && quad.x <= words.x + 0.01
        && quad.y <= words.y + 0.01
        && quad.x + quad.width >= words.x + words.width - 0.01
        && quad.y + quad.height >= words.y + words.height - 0.01
    })
    .min_by(|a, b| (a.width * a.height).total_cmp(&(b.width * b.height)))
}

/// Where a quad is and how large: x, y, width, height.
fn at(quad: &Quad) -> (f32, f32, f32, f32) {
  (quad.x, quad.y, quad.width, quad.height)
}

fn close(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.5
}

#[test]
fn a_tooltip_shows_above_what_it_describes_while_the_pointer_rests_on_it() {
  let mut stage = stage(|| Tooltip::new(WORDS).delay(Duration::ZERO));
  assert!(!stage.paints(WORDS), "nothing shows at rest");

  stage.hover("copy");
  let shown = bubble(&stage, WORDS).expect("the pointer on the button shows its tooltip");
  let (x, y, width, _) = stage.bounds("copy");
  assert_eq!(rect_color(&shown), Some(light(PaletteColor::Primary)));
  assert!(
    close(shown.y + shown.height, y - 6.0),
    "6 above the button: {:?}, the button at {y}",
    at(&shown)
  );
  assert!(
    close(shown.x + shown.width / 2.0, x + width / 2.0),
    "centered on the button: {:?}",
    at(&shown)
  );

  let (color, size) = stage.text(WORDS);
  assert_eq!(color, light(PaletteColor::PrimaryForeground));
  assert_eq!(size, 12.0);
  let words = stage.text_quad(WORDS).unwrap();
  assert!(close(words.x - shown.x, 10.0), "10 from its sides");
  assert!(close(shown.height - words.height, 12.0), "6 above and below its text");
}

#[test]
fn the_tooltip_goes_when_the_pointer_leaves() {
  let mut stage = stage(|| Tooltip::new(WORDS).delay(Duration::ZERO));
  stage.hover("copy");
  assert!(stage.paints(WORDS));
  stage.tree.mouse_move(390.0, 290.0);
  stage.pass();
  assert!(!stage.paints(WORDS));
}

#[test]
fn the_tooltip_waits_for_its_delay_and_the_next_one_skips_it() {
  let mut stage = Stage::new(|ctx| {
    Row::new()
      .padding(100.0)
      .spacing(40.0)
      .child(Tooltip::new("Cut").mount(ctx, Button::new("A").variant(ButtonVariant::Outline).id("a")))
      .child(Tooltip::new("Paste").mount(ctx, Button::new("B").variant(ButtonVariant::Outline).id("b")))
      .into()
  });
  stage.hover("a");
  assert!(!stage.paints("Cut"), "not before its delay");
  stage.tree.tick_timers_at(Instant::now() + Duration::from_millis(600));
  stage.pass();
  assert!(stage.paints("Cut"), "after its delay");

  // Along a toolbar: the one that went a moment ago lets the next show at once.
  stage.hover("b");
  assert!(!stage.paints("Cut"));
  assert!(stage.paints("Paste"), "the next shows with no delay");
}

#[test]
fn the_keyboards_focus_shows_it_at_once_and_escape_puts_it_away() {
  let mut stage = stage(|| Tooltip::new(WORDS));
  stage.key("Tab");
  assert!(
    stage.paints(WORDS),
    "the focus from the keyboard shows it with no delay"
  );
  stage.key("Escape");
  assert!(!stage.paints(WORDS), "Escape puts it away");
  // The focus did not move: the button is still what Enter presses, and the tooltip stays away.
  stage.pass();
  assert!(!stage.paints(WORDS));
}

#[test]
fn escape_puts_away_what_the_pointer_showed_until_the_pointer_comes_again() {
  let mut stage = stage(|| Tooltip::new(WORDS).delay(Duration::ZERO));
  stage.hover("copy");
  stage.key("Escape");
  assert!(!stage.paints(WORDS));
  stage.tree.mouse_move(390.0, 290.0);
  stage.pass();
  stage.hover("copy");
  assert!(stage.paints(WORDS), "it shows again when the pointer comes again");
}

#[test]
fn a_press_of_what_it_describes_puts_it_away_and_still_presses_it() {
  let pressed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
  let count = pressed.clone();
  let mut stage = Stage::new(move |ctx| {
    let count = count.clone();
    let button = copy().on_click(move |_| {
      count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    });
    Column::new()
      .padding(100.0)
      .child(Tooltip::new(WORDS).delay(Duration::ZERO).mount(ctx, button))
      .into()
  });
  stage.hover("copy");
  assert!(stage.paints(WORDS));
  stage.click("copy");
  assert!(!stage.paints(WORDS));
  assert_eq!(pressed.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn a_tooltip_on_another_side_stands_6_from_that_side() {
  let mut stage = stage(|| Tooltip::new(WORDS).delay(Duration::ZERO).side(TooltipSide::Bottom));
  stage.hover("copy");
  let shown = bubble(&stage, WORDS).unwrap();
  let (_, y, _, height) = stage.bounds("copy");
  assert!(close(shown.y, y + height + 6.0), "6 below the button: {:?}", at(&shown));

  let mut stage = stage_at(|| Tooltip::new(WORDS).delay(Duration::ZERO).side(TooltipSide::Right));
  stage.hover("copy");
  let shown = bubble(&stage, WORDS).unwrap();
  let (x, _, width, _) = stage.bounds("copy");
  assert!(
    close(shown.x, x + width + 6.0),
    "6 right of the button: {:?}",
    at(&shown)
  );
}

/// A tooltip on the button at the window's top left corner.
fn stage_at(tooltip: impl Fn() -> Tooltip + Send + Sync + 'static) -> Stage {
  Stage::new(move |ctx| Column::new().padding(4.0).child(tooltip().mount(ctx, copy())).into())
}

#[test]
fn where_there_is_no_room_above_it_is_below() {
  let mut stage = stage_at(|| Tooltip::new(WORDS).delay(Duration::ZERO).arrow(true));
  stage.hover("copy");
  let shown = bubble(&stage, WORDS).unwrap();
  let (_, y, _, height) = stage.bounds("copy");
  assert!(shown.y >= y + height, "below the button: {:?}", at(&shown));
}

#[test]
fn an_arrow_stands_between_the_box_and_what_it_describes() {
  let mut stage = stage(|| Tooltip::new(WORDS).delay(Duration::ZERO).arrow(true));
  stage.hover("copy");
  let shown = bubble(&stage, WORDS).unwrap();
  let (_, y, ..) = stage.bounds("copy");
  assert!(
    close(shown.y + shown.height, y - 6.0 - 6.0),
    "the arrow's 6 between the box and its room: {:?}",
    at(&shown)
  );
}

#[test]
fn a_shortcut_is_after_the_text_in_another_family_in_10() {
  let mut stage = stage(|| Tooltip::new("Toggle sidebar").shortcut("Ctrl+B").delay(Duration::ZERO));
  stage.hover("copy");
  let (words, keys) = (
    stage.text_quad("Toggle sidebar").unwrap(),
    stage.text_quad("Ctrl+B").unwrap(),
  );
  assert!(keys.x >= words.x + words.width + 8.0 - 0.5, "8 after the text");
  assert_eq!(stage.text("Ctrl+B").1, 10.0);
  assert_ne!(
    stage.font_of("Ctrl+B"),
    stage.font_of("Toggle sidebar"),
    "the keys are in the mono family, the text is not"
  );
}

#[test]
fn a_long_text_wraps_in_a_box_no_wider_than_200() {
  let long = "Blur radius is in logical pixels and scales with the display.";
  let mut stage = stage(move || Tooltip::new(long).delay(Duration::ZERO));
  stage.hover("copy");
  let shown = bubble(&stage, long)
    .or_else(|| {
      // A wrapped text may be painted a line at a time: find the box by its start.
      let start = stage
        .quads()
        .into_iter()
        .find(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if long.starts_with(text.as_str())))?;
      stage
        .quads()
        .into_iter()
        .filter(|quad| rect_color(quad) == Some(light(PaletteColor::Primary)))
        .find(|quad| quad.x <= start.x && quad.y <= start.y)
    })
    .expect("the long tooltip shows");
  assert!(shown.width <= 200.0 + 0.01, "no wider than 200: {:?}", at(&shown));
}

#[test]
fn open_shows_it_with_nothing_on_the_element_and_disabled_never() {
  let stage = stage(|| Tooltip::new(WORDS).open(Some(true)));
  assert!(stage.paints(WORDS));

  let mut stage = stage_disabled();
  stage.hover("copy");
  assert!(!stage.paints(WORDS));
}

fn stage_disabled() -> Stage {
  stage(|| Tooltip::new(WORDS).delay(Duration::ZERO).disabled(true))
}

#[test]
fn a_rule_of_the_theme_changes_the_tooltip() {
  let app = App::new();
  app
    .theme()
    .patch_recipe::<Tooltip>(|recipe| recipe.base(TooltipPart::Content, |s| s.background(PaletteColor::Destructive)));
  let mut stage = Stage::in_app(app, |ctx| {
    Column::new()
      .padding(100.0)
      .child(Tooltip::new(WORDS).delay(Duration::ZERO).mount(ctx, copy()))
      .into()
  });
  stage.hover("copy");
  let shown = bubble(&stage, WORDS).unwrap();
  assert_eq!(rect_color(&shown), Some(light(PaletteColor::Destructive)));
}

#[test]
fn a_theme_gives_the_tooltip_another_anatomy_and_it_still_shows_when_it_should() {
  let app = App::new();
  let renderer: TooltipRenderer = Arc::new(|view: TooltipView| {
    let shouted = view.text.to_uppercase();
    Row::new()
      .part(&view.part(TooltipPart::Content))
      .child(view.part(TooltipPart::Label).text(&shouted))
      .into()
  });
  app.theme().set_renderer::<Tooltip>(renderer);
  let mut stage = Stage::in_app(app, |ctx| {
    Column::new()
      .padding(100.0)
      .child(Tooltip::new(WORDS).delay(Duration::ZERO).mount(ctx, copy()))
      .into()
  });
  assert!(!stage.paints("COPY LINK"));
  stage.hover("copy");
  assert!(stage.paints("COPY LINK"));
  assert!(!stage.paints(WORDS));
}
