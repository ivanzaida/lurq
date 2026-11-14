//! A hover card: its panel against the board, where it stands under what it
//! is about, its delays, the pointer crossing to it, the keyboard's focus and
//! Tab through its controls, Escape, its arrow on either side, and a theme's
//! rules and renderer.

use std::{
  sync::Arc,
  time::{Duration, Instant},
};

use lurq::{
  app::{
    App,
    ctx::Ctx,
    theme::{PaletteColor, ShadowStyle, Theme, ThemePalette},
  },
  components::{Column, Placement, Row},
  layout::{
    Alignment,
    quad::{Quad, QuadContent},
  },
  node::{BoxShadowValue, Element, color::Color},
  ui::{Button, ButtonVariant, Case, HoverCard, HoverCardPart, HoverCardRenderer, HoverCardView, Link, icons},
};

use super::stage::Stage;

const NAME: &str = "@lurq";
const LINE: &str = "A Rust UI toolkit with GPU-backed rendering.";
const JOINED: &str = "Joined March 2024";

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn rect_color(quad: &Quad) -> Option<Color> {
  match quad.content {
    QuadContent::Rect { color, .. } if color.a() > 0 => Some(color),
    _ => None,
  }
}

fn close(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.5
}

/// The board's card: the name, the line and a fact.
fn card() -> HoverCard {
  HoverCard::new()
    .title(NAME)
    .description(LINE)
    .fact(icons::CALENDAR_DAYS, JOINED)
    .id("card")
}

/// The button the card is about: outlined, so that its fill is not the card's.
fn author() -> Button {
  Button::new("Author").variant(ButtonVariant::Outline).id("author")
}

/// `card` on the button, in the middle of the window's width and `top` from its top, and a button after it.
fn stage_at(top: f32, card: impl Fn() -> HoverCard + Send + Sync + 'static) -> Stage {
  Stage::new(move |ctx: &mut Ctx| -> Element {
    Column::new()
      .size(400.0, 300.0)
      .padding_top(top)
      .align_items(Alignment::Center)
      .child(card().mount(ctx, author()))
      .child(Button::new("Next").variant(ButtonVariant::Outline).id("next"))
      .into()
  })
}

fn stage(card: impl Fn() -> HoverCard + Send + Sync + 'static) -> Stage {
  stage_at(40.0, card)
}

/// The card's panel: the smallest fill around its title.
fn panel(stage: &Stage) -> Option<Quad> {
  let words = stage.text_quad(NAME)?;
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

/// The edge painted around `quad`: on the quad itself, or, where the box holds something, on a quad of its own
/// over what it holds.
fn edge_of(stage: &Stage, quad: &Quad) -> Option<(Color, f32)> {
  let same = |other: &Quad| {
    close(other.x, quad.x)
      && close(other.y, quad.y)
      && close(other.width, quad.width)
      && close(other.height, quad.height)
  };
  quad
    .border
    .and_then(|border| border.top)
    .or_else(|| {
      stage
        .quads()
        .into_iter()
        .filter(|other| same(other))
        .find_map(|other| other.border.and_then(|border| border.top))
    })
    .map(|side| (side.color, side.width))
}

/// Lets the timers run `later` from now, and paints.
fn after(stage: &mut Stage, later: Duration) {
  stage.tree.tick_timers_at(Instant::now() + later);
  stage.pass();
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

#[test]
fn it_opens_under_what_the_pointer_rests_on_after_its_delay() {
  let mut stage = stage(card);
  stage.hover("author");
  assert!(!stage.paints(NAME), "not before its delay");
  after(&mut stage, Duration::from_millis(450));
  assert!(stage.paints(NAME), "after the theme's slower duration, 400");

  let shown = panel(&stage).expect("the panel");
  let (x, y, width, height) = stage.bounds("author");
  assert_eq!(rect_color(&shown), Some(light(PaletteColor::Popover)));
  assert!(close(shown.width, 300.0), "300 wide: {}", shown.width);
  assert!(
    close(shown.y, y + height + 8.0),
    "8 under the button: {}, the button's bottom at {}",
    shown.y,
    y + height
  );
  assert!(
    close(shown.x + shown.width / 2.0, x + width / 2.0),
    "centered on the button"
  );
  let (color, width) = edge_of(&stage, &shown).expect("an edge");
  assert_eq!(color, light(PaletteColor::Border));
  assert!(close(width, 1.0));
}

#[test]
fn its_text_is_as_the_board_draws_it() {
  let mut stage = stage(|| card().open(Some(true)));
  stage.pass();
  let shown = panel(&stage).unwrap();
  let (name, line, joined) = (
    stage.text_quad(NAME).unwrap(),
    stage
      .quads()
      .into_iter()
      .find(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if !text.is_empty() && LINE.starts_with(text.as_str())))
      .unwrap(),
    stage.text_quad(JOINED).unwrap(),
  );
  assert!(close(name.x - shown.x, 16.0), "16 in from the panel's side");
  assert!(close(name.y - shown.y, 16.0), "16 down from its top");
  assert_eq!(stage.text(NAME).1, 14.0);
  assert!(line.y >= name.y + name.height + 6.0 - 0.5, "the line 6 under the name");
  assert!(joined.y > line.y, "the fact under the line");
  assert_eq!(stage.text(JOINED), (light(PaletteColor::MutedForeground), 12.0));
}

#[test]
fn media_stands_12_before_the_text() {
  let mut stage = stage(|| {
    card()
      .media(Row::new().size(40.0, 40.0).background(PaletteColor::Muted).id("avatar"))
      .open(Some(true))
  });
  stage.pass();
  let shown = panel(&stage).unwrap();
  let (ax, ay, aw, _) = stage.bounds("avatar");
  assert!(
    close(ax - shown.x, 16.0) && close(ay - shown.y, 16.0),
    "at the panel's start"
  );
  let name = stage.text_quad(NAME).unwrap();
  assert!(close(name.x, ax + aw + 12.0), "the text 12 after the media");
}

#[test]
fn the_pointer_can_cross_to_the_card_and_it_closes_a_moment_after_leaving_both() {
  let mut stage = stage(|| card().open_delay(Duration::ZERO));
  stage.hover("author");
  assert!(stage.paints(NAME));

  // Across the room between them.
  let (x, y, width, height) = stage.bounds("author");
  stage.tree.mouse_move(x + width / 2.0, y + height + 4.0);
  stage.pass();
  after(&mut stage, Duration::from_millis(100));
  assert!(stage.paints(NAME), "the room between them is the card's");

  let shown = panel(&stage).unwrap();
  stage
    .tree
    .mouse_move(shown.x + shown.width / 2.0, shown.y + shown.height / 2.0);
  stage.pass();
  after(&mut stage, Duration::from_millis(500));
  assert!(stage.paints(NAME), "the pointer is on it");

  stage.tree.mouse_move(5.0, 295.0);
  stage.pass();
  assert!(stage.paints(NAME), "it stays for its close delay");
  after(&mut stage, Duration::from_millis(300));
  assert!(!stage.paints(NAME), "the pointer left both");
}

#[test]
fn the_keyboards_focus_opens_it_at_once_and_tab_goes_through_its_controls_and_on() {
  let mut stage = stage(|| card().child(Button::new("Follow").variant(ButtonVariant::Outline).id("follow")));
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("author"));
  assert!(stage.paints(NAME), "the focus opens it with no delay");
  stage.pass();
  assert_eq!(
    focused(&stage).as_deref(),
    Some("author"),
    "the focus is not moved into it as it opens"
  );

  stage.key("Tab");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("follow"),
    "from the button into the card"
  );
  assert!(stage.paints(NAME), "the focus in the card keeps it open");

  stage.key("Tab");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("next"),
    "on to what follows the button"
  );
  assert!(!stage.paints(NAME), "the focus left both");
}

#[test]
fn escape_closes_it_and_the_focus_goes_back_to_what_it_is_about() {
  let mut stage = stage(|| card().child(Link::new("Profile").id("profile")));
  stage.key("Tab");
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("profile"));
  stage.key("Escape");
  stage.pass();
  assert!(!stage.paints(NAME), "Escape closes it");
  assert_eq!(focused(&stage).as_deref(), Some("author"), "back to the button");
  stage.pass();
  assert!(!stage.paints(NAME), "and it stays closed while the focus stays");
}

#[test]
fn escape_closes_what_the_pointer_opened_until_the_pointer_comes_again() {
  let mut stage = stage(|| card().open_delay(Duration::ZERO));
  stage.hover("author");
  stage.key("Escape");
  assert!(!stage.paints(NAME));
  stage.tree.mouse_move(5.0, 295.0);
  stage.pass();
  stage.hover("author");
  assert!(stage.paints(NAME), "it opens again when the pointer comes again");
}

#[test]
fn one_card_of_the_pointer_shows_at_a_time() {
  let mut stage = Stage::new(|ctx| {
    Row::new()
      .size(400.0, 300.0)
      .padding(40.0)
      .spacing(40.0)
      .child(
        HoverCard::new()
          .title("First")
          .open_delay(Duration::ZERO)
          .mount(ctx, Button::new("A").variant(ButtonVariant::Outline).id("a")),
      )
      .child(
        HoverCard::new()
          .title("Second")
          .open_delay(Duration::ZERO)
          .mount(ctx, Button::new("B").variant(ButtonVariant::Outline).id("b")),
      )
      .into()
  });
  stage.hover("a");
  assert!(stage.paints("First"));
  stage.hover("b");
  assert!(stage.paints("Second"));
  assert!(!stage.paints("First"), "the one before goes at once");
}

/// The arrow: a square of 18 / √2 in the panel's color.
fn tip(stage: &Stage) -> Option<Quad> {
  let side = 18.0 / std::f32::consts::SQRT_2;
  stage.quads().into_iter().find(|quad| {
    rect_color(quad) == Some(light(PaletteColor::Popover)) && close(quad.width, side) && close(quad.height, side)
  })
}

/// The middle of a quad as it is painted. A turned quad is its turned corner, turned about that corner.
fn painted_center(quad: &Quad) -> (f32, f32) {
  let (t, half_w, half_h) = (quad.transform, quad.width / 2.0, quad.height / 2.0);
  (
    quad.x + t.a * half_w + t.c * half_h,
    quad.y + t.b * half_w + t.d * half_h,
  )
}

#[test]
fn its_arrow_with_its_edge_stands_between_the_panel_and_the_button() {
  let mut stage = stage(|| card().arrow(true).open(Some(true)));
  stage.pass();
  let shown = panel(&stage).unwrap();
  let (x, y, width, height) = stage.bounds("author");
  assert!(
    close(shown.y, y + height + 8.0 + 9.0),
    "the arrow's 9 between the panel and its room: {}",
    shown.y
  );
  let arrow = tip(&stage).expect("an arrow");
  let (color, _) = edge_of(&stage, &arrow).expect("the arrow has the panel's edge");
  assert_eq!(color, light(PaletteColor::Border));
  let (arrow_x, arrow_y) = painted_center(&arrow);
  assert!(
    close(arrow_x, x + width / 2.0),
    "at the middle of the button: {arrow_x}"
  );
  assert!(
    arrow_y > y + height && arrow_y <= shown.y + 1.5,
    "over the panel's edge, under the button: {arrow_y}, the panel at {}",
    shown.y
  );
}

#[test]
fn where_there_is_no_room_under_it_it_is_above_and_its_arrow_turns() {
  let mut stage = stage_at(230.0, || card().arrow(true).open(Some(true)));
  stage.pass();
  stage.pass();
  let shown = panel(&stage).unwrap();
  let (_, y, ..) = stage.bounds("author");
  assert!(
    shown.y + shown.height <= y,
    "above the button: {:?}",
    (shown.y, shown.height)
  );
  let (_, arrow_y) = painted_center(&tip(&stage).expect("an arrow"));
  assert!(
    arrow_y < y && arrow_y >= shown.y + shown.height - 1.5,
    "under the panel, over the button: {arrow_y}"
  );
}

#[test]
fn it_has_the_medium_shadow_and_a_disabled_one_never_opens() {
  let theme = Theme::new();
  let panel_style = theme
    .resolve::<HoverCard>(&Case::new(), None)
    .part(HoverCardPart::Panel)
    .rest();
  assert!(matches!(
    panel_style.shadow,
    Some(BoxShadowValue::Theme(ShadowStyle::Md))
  ));

  let mut stage = stage(|| card().open_delay(Duration::ZERO).disabled(true));
  stage.hover("author");
  assert!(!stage.paints(NAME));
}

#[test]
fn a_rule_of_the_theme_changes_the_card() {
  let app = App::new();
  app
    .theme()
    .patch_recipe::<HoverCard>(|recipe| recipe.base(HoverCardPart::Panel, |s| s.background(PaletteColor::Destructive)));
  let mut stage = Stage::in_app(app, |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .padding_top(40.0)
      .align_items(Alignment::Center)
      .child(card().open(Some(true)).mount(ctx, author()))
      .into()
  });
  stage.pass();
  assert_eq!(
    rect_color(&panel(&stage).unwrap()),
    Some(light(PaletteColor::Destructive))
  );
}

#[test]
fn a_theme_gives_the_card_another_anatomy_and_it_still_opens_when_it_should() {
  let app = App::new();
  let renderer: HoverCardRenderer = Arc::new(|view: HoverCardView| {
    let shouted = view.title.as_deref().unwrap_or_default().to_uppercase();
    Row::new()
      .part(&view.part(HoverCardPart::Panel))
      .child(view.part(HoverCardPart::Title).text(&shouted))
      .into()
  });
  app.theme().set_renderer::<HoverCard>(renderer);
  let mut stage = Stage::in_app(app, |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .padding_top(40.0)
      .align_items(Alignment::Center)
      .child(card().open_delay(Duration::ZERO).mount(ctx, author()))
      .into()
  });
  assert!(!stage.paints("@LURQ"));
  stage.hover("author");
  assert!(stage.paints("@LURQ"));
  assert!(!stage.paints(NAME));
}

#[test]
fn placed_at_the_start_it_lines_up_with_the_buttons_start() {
  let mut stage = Stage::new(|ctx| {
    Column::new()
      .size(400.0, 300.0)
      .padding(40.0)
      .child(
        card()
          .placement(Placement::BottomStart)
          .open(Some(true))
          .mount(ctx, author()),
      )
      .into()
  });
  stage.pass();
  let shown = panel(&stage).unwrap();
  let (x, ..) = stage.bounds("author");
  assert!(close(shown.x, x), "{} and {x}", shown.x);
}
