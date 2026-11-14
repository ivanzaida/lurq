//! A teaching tip and a tour: the tip against the board, where it stands by
//! what it points at, that it leaves the focus where it is, what closes it
//! and what it tells, Tab into its buttons, the primary look, a tour's steps,
//! and a theme's rules and renderer.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App,
    ctx::Ctx,
    events::MouseButton,
    theme::{PaletteColor, ShadowStyle, Theme, ThemePalette},
  },
  components::{Column, Rect, Row},
  core::{ElementRef, Signal},
  layout::{
    Alignment,
    quad::{Quad, QuadContent},
  },
  node::{BoxShadowValue, Element, color::Color},
  ui::{
    Button, ButtonVariant, Case, TeachingTip, TeachingTipPart, TeachingTipRenderer, TeachingTipVariant,
    TeachingTipView, Tour, TourStep,
  },
};

use super::stage::Stage;

const TITLE: &str = "Dock panels anywhere";
const SAYS: &str = "Drag a panel by its header and drop it on any edge of the window.";

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

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// The board's teaching tip, told of each close in `closed`.
fn dock(anchor: ElementRef, open: Signal<bool>, closed: &Arc<AtomicUsize>) -> TeachingTip {
  let (done, closed) = (open.clone(), closed.clone());
  TeachingTip::new(anchor, open)
    .title(TITLE)
    .description(SAYS)
    .primary("Got it", move |_| done.set(false))
    .on_close(move || {
      closed.fetch_add(1, Ordering::SeqCst);
    })
    .id("tip")
}

/// A field to type in, the button the tip points at in the middle of the window 40 from its top, a button
/// after it, and the tip `make` makes.
fn stage(open: &Signal<bool>, make: impl Fn(ElementRef, Signal<bool>) -> TeachingTip + Send + Sync + 'static) -> Stage {
  let (anchor, open) = (ElementRef::new(), open.clone());
  Stage::new(move |ctx: &mut Ctx| -> Element {
    Column::new()
      .size(400.0, 300.0)
      .padding_top(40.0)
      .align_items(Alignment::Center)
      .child(Button::new("Field").variant(ButtonVariant::Outline).id("field"))
      .child(
        Row::new()
          .ref_element(anchor.clone())
          .child(Button::new("Panels").variant(ButtonVariant::Outline).id("panels")),
      )
      .child(Button::new("After").variant(ButtonVariant::Outline).id("after"))
      .child(make(anchor.clone(), open.clone()).mount(ctx))
      .into()
  })
}

/// The tip's panel: the smallest fill around its title.
fn panel(stage: &Stage, title: &str) -> Option<Quad> {
  let words = stage.text_quad(title)?;
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

/// Presses the painted text `content`.
fn press_text(stage: &mut Stage, content: &str) {
  let quad = stage
    .text_quad(content)
    .unwrap_or_else(|| panic!("{content:?} is painted"));
  let (x, y) = (quad.x + quad.width / 2.0, quad.y + quad.height / 2.0);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

#[test]
fn it_points_at_its_anchor_from_under_it_and_leaves_the_focus_where_it_is() {
  let (open, closed) = (Signal::new(false), Arc::new(AtomicUsize::new(0)));
  let mut stage = {
    let closed = closed.clone();
    stage(&open, move |anchor, open| dock(anchor, open, &closed))
  };
  assert!(!stage.paints(TITLE), "closed while its signal is false");
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("field"));

  open.set(true);
  stage.pass();
  stage.pass();
  assert!(stage.paints(TITLE), "open while its signal is true");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("field"),
    "the focus stays with what the person was doing"
  );

  let shown = panel(&stage, TITLE).expect("the panel");
  let (x, y, width, height) = stage.bounds("panels");
  assert_eq!(rect_color(&shown), Some(light(PaletteColor::Popover)));
  assert!(close(shown.width, 300.0), "300 wide: {}", shown.width);
  assert!(
    close(shown.y, y + height + 4.0 + 9.0),
    "4 and its arrow's 9 under what it points at: {}",
    shown.y
  );
  assert!(close(shown.x + shown.width / 2.0, x + width / 2.0), "centered on it");

  let title = stage.text_quad(TITLE).unwrap();
  assert!(close(title.x - shown.x, 14.0), "14 of padding at its side");
  assert!(
    title.y >= shown.y + 14.0 - 0.5 && title.y <= shown.y + 14.0 + 4.0,
    "14 of padding over the header, the title in the middle of its row: {}",
    title.y - shown.y
  );
  assert_eq!(stage.text(TITLE).1, 14.0);
  assert_eq!(stage.text("Got it").0, light(PaletteColor::PrimaryForeground));
  let (_, _, _, button_height) = stage.bounds("tip-primary");
  assert!(close(button_height, 30.0), "a button 30 high");
}

#[test]
fn its_close_control_and_escape_close_it_and_tell_on_close_and_a_press_elsewhere_does_not() {
  let (open, closed) = (Signal::new(true), Arc::new(AtomicUsize::new(0)));
  let mut stage = {
    let closed = closed.clone();
    stage(&open, move |anchor, open| dock(anchor, open, &closed))
  };
  stage.pass();
  assert!(stage.paints(TITLE));

  stage.click("field");
  assert!(open.get(), "a press elsewhere is the person going on beside it");
  assert!(stage.paints(TITLE));

  stage.click("tip-close");
  assert!(!open.get(), "its close control closes it");
  assert_eq!(closed.load(Ordering::SeqCst), 1);
  stage.pass();
  assert!(!stage.paints(TITLE));

  open.set(true);
  stage.pass();
  assert!(stage.paints(TITLE));
  stage.key("Escape");
  stage.pass();
  assert!(!open.get(), "Escape closes it");
  assert_eq!(closed.load(Ordering::SeqCst), 2, "and tells on_close");
  assert!(!stage.paints(TITLE));

  // Closed by the app, it tells nothing.
  open.set(true);
  stage.pass();
  press_text(&mut stage, "Got it");
  assert!(!open.get());
  assert_eq!(closed.load(Ordering::SeqCst), 2, "its own button says what it does");
}

#[test]
fn tab_goes_from_what_it_points_at_into_its_buttons_and_on() {
  let (open, closed) = (Signal::new(true), Arc::new(AtomicUsize::new(0)));
  let mut stage = {
    let closed = closed.clone();
    stage(&open, move |anchor, open| {
      dock(anchor, open.clone(), &closed).secondary("Not now", move |_| open.set(false))
    })
  };
  stage.pass();
  stage.key("Tab");
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("panels"));
  stage.key("Tab");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("tip-close"),
    "into the tip after what it points at"
  );
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("tip-secondary"));
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("tip-primary"));
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("after"), "and on");
  assert!(open.get(), "moving through it leaves it open");
}

#[test]
fn taking_the_focus_it_moves_it_in_as_it_opens_and_back_as_it_closes() {
  let (open, closed) = (Signal::new(false), Arc::new(AtomicUsize::new(0)));
  let mut stage = {
    let closed = closed.clone();
    stage(&open, move |anchor, open| dock(anchor, open, &closed).takes_focus(true))
  };
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("field"));
  open.set(true);
  stage.pass();
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("tip-close"), "the first thing in it");
  stage.key("Escape");
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("field"), "back to what had it");
}

#[test]
fn its_media_is_96_high_at_its_top_and_the_primary_look_is_in_the_primary_color() {
  let open = Signal::new(true);
  let mut stage = stage(&open, |anchor, open| {
    TeachingTip::new(anchor, open)
      .title(TITLE)
      .media(Rect::new(24.0, 24.0).id("sparkles"))
      .variant(TeachingTipVariant::Primary)
  });
  stage.pass();
  let shown = panel(&stage, TITLE).expect("the panel");
  assert_eq!(rect_color(&shown), Some(light(PaletteColor::Primary)));
  assert_eq!(stage.text(TITLE).0, light(PaletteColor::PrimaryForeground));
  let (sx, sy, sw, sh) = stage.bounds("sparkles");
  assert!(
    close(sy + sh / 2.0, shown.y + 48.0),
    "in the middle of the media's 96 at the panel's top"
  );
  assert!(close(sx + sw / 2.0, shown.x + shown.width / 2.0), "centered in it");
  let title = stage.text_quad(TITLE).unwrap();
  assert!(title.y >= shown.y + 96.0 + 14.0 - 0.5, "the text under the media");

  let theme = Theme::new();
  let plain = theme
    .resolve::<TeachingTip>(&Case::new(), None)
    .part(TeachingTipPart::Panel)
    .rest();
  assert!(matches!(plain.shadow, Some(BoxShadowValue::Theme(ShadowStyle::Lg))));
}

#[test]
fn a_tour_shows_its_steps_one_after_another() {
  let (step, finished) = (Signal::new(Some(0_usize)), Arc::new(AtomicUsize::new(0)));
  let (first, second) = (ElementRef::new(), ElementRef::new());
  let mut stage = {
    let (step, finished) = (step.clone(), finished.clone());
    Stage::new(move |ctx: &mut Ctx| -> Element {
      let finished = finished.clone();
      Column::new()
        .size(400.0, 300.0)
        .padding_top(20.0)
        .align_items(Alignment::Center)
        .child(Rect::new(80.0, 24.0).id("first").ref_element(first.clone()))
        .child(Rect::new(1.0, 60.0))
        .child(Rect::new(80.0, 24.0).id("second").ref_element(second.clone()))
        .child(
          Tour::new(step.clone())
            .step(TourStep::new(first.clone(), "Switch themes here").description("Light and dark."))
            .step(TourStep::new(second.clone(), "Dock panels anywhere"))
            .on_finish(move || {
              finished.fetch_add(1, Ordering::SeqCst);
            })
            .mount(ctx),
        )
        .into()
    })
  };
  stage.pass();
  assert!(stage.paints("Switch themes here"));
  assert!(stage.paints("1 of 2"));
  assert!(!stage.paints("Back"), "nothing to go back to");
  let shown = panel(&stage, "Switch themes here").unwrap();
  assert!(close(shown.width, 280.0), "the board's tour is 280 wide");
  let title = stage.text_quad("Switch themes here").unwrap();
  assert!(close(title.x - shown.x, 16.0), "padded 16");
  assert_ne!(
    stage.font_of("1 of 2"),
    stage.font_of("Switch themes here"),
    "the count in the mono family"
  );

  press_text(&mut stage, "Next");
  assert_eq!(step.get(), Some(1));
  stage.pass();
  assert!(stage.paints("2 of 2"));
  assert!(stage.paints("Dock panels anywhere"));
  let (_, y, _, height) = stage.bounds("second");
  assert!(
    panel(&stage, "Dock panels anywhere").unwrap().y > y + height,
    "under the second"
  );

  press_text(&mut stage, "Back");
  assert_eq!(step.get(), Some(0));
  stage.pass();
  press_text(&mut stage, "Next");
  stage.pass();
  press_text(&mut stage, "Done");
  assert_eq!(step.get(), None, "the last step ends it");
  assert_eq!(finished.load(Ordering::SeqCst), 1);
  stage.pass();
  assert!(!stage.paints("Dock panels anywhere"));

  step.set(Some(1));
  stage.pass();
  assert!(stage.paints("Dock panels anywhere"));
  stage.key("Escape");
  stage.pass();
  assert_eq!(step.get(), None, "Escape leaves the tour");
  assert_eq!(finished.load(Ordering::SeqCst), 1, "without finishing it");
}

#[test]
fn a_rule_of_the_theme_changes_the_tip() {
  let app = App::new();
  app.theme().patch_recipe::<TeachingTip>(|recipe| {
    recipe.base(TeachingTipPart::Panel, |s| s.background(PaletteColor::Destructive))
  });
  let open = Signal::new(true);
  let anchor = ElementRef::new();
  let mut stage = Stage::in_app(app, move |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .padding(40.0)
      .child(Rect::new(80.0, 24.0).ref_element(anchor.clone()))
      .child(TeachingTip::new(anchor.clone(), open.clone()).title(TITLE).mount(ctx))
      .into()
  });
  stage.pass();
  assert_eq!(
    rect_color(&panel(&stage, TITLE).unwrap()),
    Some(light(PaletteColor::Destructive))
  );
}

#[test]
fn a_theme_gives_the_tip_another_anatomy_and_it_shows_when_its_signal_says() {
  let app = App::new();
  let renderer: TeachingTipRenderer = Arc::new(|view: TeachingTipView| {
    let shouted = view.title.as_deref().unwrap_or_default().to_uppercase();
    Row::new()
      .part(&view.part(TeachingTipPart::Panel))
      .child(view.part(TeachingTipPart::Title).text(&shouted))
      .into()
  });
  app.theme().set_renderer::<TeachingTip>(renderer);
  let open = Signal::new(false);
  let mut stage = {
    let (anchor, open) = (ElementRef::new(), open.clone());
    Stage::in_app(app, move |ctx| {
      Column::new()
        .size(400.0, 300.0)
        .padding(40.0)
        .child(Rect::new(80.0, 24.0).ref_element(anchor.clone()))
        .child(TeachingTip::new(anchor.clone(), open.clone()).title(TITLE).mount(ctx))
        .into()
    })
  };
  assert!(!stage.paints("DOCK PANELS ANYWHERE"));
  open.set(true);
  stage.pass();
  assert!(stage.paints("DOCK PANELS ANYWHERE"));
}
