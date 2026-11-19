//! A toggletip: its button and its panel against the board, what opens and
//! closes it, where the focus stays and where Tab goes, its arrow, and a
//! theme's rules and renderer.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App,
    ctx::{CollisionStrategy, Ctx},
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Rect, Row},
  layout::{
    Alignment,
    quad::{Quad, QuadContent},
  },
  node::{Element, color::Color},
  ui::{Interaction, Toggletip, ToggletipPart, ToggletipRenderer, ToggletipView},
};

use super::stage::Stage;

const SAYS: &str = "Scales with the display, so 1px stays crisp.";

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

/// The board's toggletip: a line and a link.
fn tip(followed: &Arc<AtomicUsize>) -> Toggletip {
  let followed = followed.clone();
  Toggletip::new("What is a logical pixel?")
    .text(SAYS)
    .link("Learn more", move || {
      followed.fetch_add(1, Ordering::SeqCst);
    })
    .id("help")
}

/// `tip` in the middle of the window, 150 from its top, with a box to press outside it.
fn stage(tip: impl Fn() -> Toggletip + Send + Sync + 'static) -> Stage {
  Stage::new(move |ctx: &mut Ctx| -> Element {
    Column::new()
      .size(400.0, 300.0)
      .padding_top(150.0)
      .align_items(Alignment::Center)
      .child(tip().mount(ctx))
      .child(Rect::new(40.0, 20.0).id("outside"))
      .into()
  })
}

/// The start of the panel's text, which may be painted a line at a time.
fn says(stage: &Stage) -> Option<Quad> {
  stage
    .quads()
    .into_iter()
    .find(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if !text.is_empty() && SAYS.starts_with(text.as_str())))
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

#[test]
fn its_button_opens_the_panel_above_it_and_another_press_closes_it() {
  let followed = Arc::new(AtomicUsize::new(0));
  let mut stage = {
    let followed = followed.clone();
    stage(move || tip(&followed))
  };
  assert!(says(&stage).is_none(), "closed at rest");
  let (bx, by, bwidth, bheight) = stage.bounds("help");
  assert!(close(bwidth, 20.0) && close(bheight, 20.0), "a button of 20");

  stage.click("help");
  assert!(says(&stage).is_some(), "a press opens it");
  let (_, ry, _, rheight) = stage.bounds("help-panel");
  assert!(
    close(ry + rheight, by),
    "its root reaches the button: the room is the root's"
  );
  let panel = stage
    .quads()
    .into_iter()
    .find(|quad| rect_color(quad) == Some(light(PaletteColor::Primary)) && close(quad.width, 230.0))
    .expect("the panel in the primary color, 230 wide");
  assert!(close(panel.y + panel.height, by - 6.0), "6 above the button");
  assert!(close(panel.x + panel.width / 2.0, bx + bwidth / 2.0), "centered on it");
  let text = says(&stage).unwrap();
  assert!(
    close(text.x - panel.x, 12.0) && close(text.y - panel.y, 12.0),
    "12 of padding"
  );
  assert_eq!(stage.text("Learn more").0, light(PaletteColor::PrimaryForeground));
  assert_eq!(stage.text("Learn more").1, 13.0);

  stage.click("help");
  stage.pass();
  assert!(says(&stage).is_none(), "another press closes it");
}

#[test]
fn its_link_is_pressed_and_a_press_outside_closes_it() {
  let followed = Arc::new(AtomicUsize::new(0));
  let mut stage = {
    let followed = followed.clone();
    stage(move || tip(&followed))
  };
  stage.click("help");
  let link = stage.text_quad("Learn more").unwrap();
  let (x, y) = (link.x + link.width / 2.0, link.y + link.height / 2.0);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, lurq::app::events::MouseButton::Left);
  stage.tree.mouse_up(x, y, lurq::app::events::MouseButton::Left);
  stage.pass();
  assert_eq!(followed.load(Ordering::SeqCst), 1, "the link was followed");

  stage.click("outside");
  stage.pass();
  assert!(says(&stage).is_none(), "a press outside closes it");
}

#[test]
fn the_focus_stays_on_its_button_and_tab_goes_into_the_panel_then_escape_brings_it_back() {
  let followed = Arc::new(AtomicUsize::new(0));
  let mut stage = {
    let followed = followed.clone();
    stage(move || tip(&followed))
  };
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("help"));
  stage.key("Enter");
  stage.pass();
  assert!(says(&stage).is_some(), "Enter opens it");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("help"),
    "the focus stays on the button"
  );

  stage.key("Tab");
  assert_ne!(
    focused(&stage).as_deref(),
    Some("help"),
    "Tab goes into the panel's link"
  );
  assert!(says(&stage).is_some(), "and it stays open");
  stage.key("Enter");
  assert_eq!(followed.load(Ordering::SeqCst), 1, "Enter on the link follows it");

  stage.key("Escape");
  stage.pass();
  assert!(says(&stage).is_none(), "Escape closes it");
  assert_eq!(focused(&stage).as_deref(), Some("help"), "back to the button");
}

#[test]
fn its_arrow_points_down_at_the_button() {
  let mut stage = stage(|| {
    Toggletip::new("Help")
      .text(SAYS)
      .arrow(true)
      .id("help")
      .force_open(Some(true))
  });
  stage.pass();
  let (bx, by, bwidth, _) = stage.bounds("help");
  let side = 12.0 / std::f32::consts::SQRT_2;
  let arrow = stage
    .quads()
    .into_iter()
    .find(|quad| {
      rect_color(quad) == Some(light(PaletteColor::Primary)) && close(quad.width, side) && close(quad.height, side)
    })
    .expect("an arrow in the panel's color");
  let (t, half) = (arrow.transform, side / 2.0);
  let (ax, ay) = (arrow.x + t.a * half + t.c * half, arrow.y + t.b * half + t.d * half);
  assert!(close(ax, bx + bwidth / 2.0), "at the button's middle: {ax}");
  assert!(ay < by, "over the button: {ay}");
  let panel = stage
    .quads()
    .into_iter()
    .find(|quad| rect_color(quad) == Some(light(PaletteColor::Primary)) && close(quad.width, 230.0))
    .unwrap();
  assert!(
    close(panel.y + panel.height, by - 6.0 - 6.0),
    "the arrow's 6 between the panel and its room"
  );
  assert!(ay > panel.y + panel.height - 1.0, "under the panel");
}

#[test]
fn a_disabled_one_does_not_open() {
  let mut stage = stage(|| Toggletip::new("Help").text(SAYS).id("help").disabled(true));
  stage.click("help");
  assert!(says(&stage).is_none());
}

#[test]
fn a_rule_of_the_theme_changes_the_panel() {
  let app = App::new();
  app
    .theme()
    .patch_recipe::<Toggletip>(|recipe| recipe.base(ToggletipPart::Panel, |s| s.background(PaletteColor::Destructive)));
  let mut stage = Stage::in_app(app, |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .padding_top(150.0)
      .align_items(Alignment::Center)
      .child(Toggletip::new("Help").text(SAYS).force_open(Some(true)).mount(ctx))
      .into()
  });
  stage.pass();
  assert!(
    stage
      .quads()
      .iter()
      .any(|quad| rect_color(quad) == Some(light(PaletteColor::Destructive)) && close(quad.width, 230.0))
  );
}

#[test]
fn a_theme_gives_the_panel_another_anatomy_and_the_button_still_opens_it() {
  let app = App::new();
  let renderer: ToggletipRenderer = Arc::new(|view: ToggletipView| {
    let shouted = view.text.as_deref().unwrap_or_default().to_uppercase();
    Row::new()
      .part(&view.part(ToggletipPart::Panel))
      .child(view.part(ToggletipPart::Label).text(&shouted))
      .into()
  });
  app.theme().set_renderer::<Toggletip>(renderer);
  let mut stage = Stage::in_app(app, |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .padding_top(150.0)
      .align_items(Alignment::Center)
      .child(Toggletip::new("Help").text("Crisp").id("help").mount(ctx))
      .into()
  });
  assert!(!stage.paints("CRISP"));
  stage.click("help");
  assert!(stage.paints("CRISP"));
}

#[test]
fn on_dismiss_is_told_of_each_way_a_person_closes_it_and_every_handler_is() {
  let (first, second) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
  let mut stage = {
    let (first, second) = (first.clone(), second.clone());
    stage(move || {
      let (first, second) = (first.clone(), second.clone());
      Toggletip::new("Help")
        .text(SAYS)
        .id("help")
        .on_dismiss(move || {
          first.fetch_add(1, Ordering::SeqCst);
        })
        .on_dismiss(move || {
          second.fetch_add(1, Ordering::SeqCst);
        })
    })
  };
  let told = |count: &Arc<AtomicUsize>| count.load(Ordering::SeqCst);

  stage.click("help");
  assert!(says(&stage).is_some());
  assert_eq!(told(&first), 0, "opening it is no dismissal");
  stage.click("help");
  stage.pass();
  assert!(says(&stage).is_none());
  assert_eq!(told(&first), 1, "another press of its button");

  stage.click("help");
  stage.click("outside");
  stage.pass();
  assert!(says(&stage).is_none());
  assert_eq!(told(&first), 2, "a press outside it");

  stage.click("help");
  stage.key("Escape");
  stage.pass();
  assert!(says(&stage).is_none());
  assert_eq!(told(&first), 3, "Escape");
  assert_eq!(told(&second), 3, "the second handler after the first, not in its place");
}

#[test]
fn without_dismiss_on_escape_escape_leaves_it_open() {
  let mut stage = stage(|| Toggletip::new("Help").text(SAYS).id("help").dismiss_on_escape(false));
  stage.click("help");
  stage.key("Escape");
  stage.pass();
  assert!(says(&stage).is_some(), "Escape does not close it");
  stage.click("help");
  stage.pass();
  assert!(says(&stage).is_none(), "its button still does");
}

#[test]
fn without_dismiss_on_outside_press_a_press_outside_leaves_it_open() {
  let mut stage = stage(|| {
    Toggletip::new("Help")
      .text(SAYS)
      .id("help")
      .dismiss_on_outside_press(false)
  });
  stage.click("help");
  stage.pass();
  stage.click("outside");
  stage.pass();
  assert!(says(&stage).is_some(), "a press outside does not close it");
  stage.key("Escape");
  stage.pass();
  assert!(says(&stage).is_none(), "Escape still does");
}

#[test]
fn an_offset_stands_the_panel_that_far_from_the_button() {
  let mut stage = stage(|| {
    Toggletip::new("Help")
      .text(SAYS)
      .id("help")
      .offset(16.0)
      .force_open(Some(true))
  });
  stage.pass();
  let (_, by, ..) = stage.bounds("help");
  let panel = stage
    .quads()
    .into_iter()
    .find(|quad| rect_color(quad) == Some(light(PaletteColor::Primary)) && close(quad.width, 230.0))
    .expect("the panel");
  assert!(close(panel.y + panel.height, by - 16.0), "16 above the button");
}

#[test]
fn a_collision_that_only_clamps_keeps_the_panel_on_its_own_side() {
  let at_top = |collision: CollisionStrategy| {
    Stage::new(move |ctx: &mut Ctx| -> Element {
      Column::new()
        .size(400.0, 300.0)
        .padding_top(4.0)
        .align_items(Alignment::Center)
        .child(
          Toggletip::new("Help")
            .text(SAYS)
            .id("help")
            .collision(collision)
            .force_open(Some(true))
            .mount(ctx),
        )
        .into()
    })
  };
  let below = |stage: &mut Stage| {
    stage.pass();
    let (_, by, _, bheight) = stage.bounds("help");
    let panel = stage
      .quads()
      .into_iter()
      .find(|quad| rect_color(quad) == Some(light(PaletteColor::Primary)) && close(quad.width, 230.0))
      .expect("the panel");
    panel.y >= by + bheight
  };
  assert!(
    below(&mut at_top(CollisionStrategy::FlipThenClamp)),
    "no room above: it goes below"
  );
  assert!(
    !below(&mut at_top(CollisionStrategy::Clamp)),
    "it stays on its side, moved into the window"
  );
}

#[test]
fn force_draws_its_button_focused_with_nothing_on_it() {
  let forced = stage(|| Toggletip::new("Help").text(SAYS).force(Some(Interaction::FocusVisible)));
  assert!(
    forced.has_ring(light(PaletteColor::Ring)),
    "the focus ring of its button"
  );
  let rest = stage(|| Toggletip::new("Help").text(SAYS));
  assert!(!rest.has_ring(light(PaletteColor::Ring)));
}
