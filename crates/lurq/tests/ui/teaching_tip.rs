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
    ctx::{CollisionStrategy, Ctx},
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
    Button, ButtonVariant, Case, Interaction, TeachingTip, TeachingTipPart, TeachingTipRenderer, TeachingTipVariant,
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
  TeachingTip::new(open, anchor)
    .title(TITLE)
    .description(SAYS)
    .primary("Got it", move || done.set(false))
    .on_dismiss(move || {
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
fn its_close_control_and_escape_close_it_and_tell_on_dismiss_and_a_press_elsewhere_does_not() {
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
  assert_eq!(closed.load(Ordering::SeqCst), 2, "and tells on_dismiss");
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
      dock(anchor, open.clone(), &closed).secondary("Not now", move || open.set(false))
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
    TeachingTip::new(open, anchor)
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
      .child(TeachingTip::new(open.clone(), anchor.clone()).title(TITLE).mount(ctx))
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
        .child(TeachingTip::new(open.clone(), anchor.clone()).title(TITLE).mount(ctx))
        .into()
    })
  };
  assert!(!stage.paints("DOCK PANELS ANYWHERE"));
  open.set(true);
  stage.pass();
  assert!(stage.paints("DOCK PANELS ANYWHERE"));
}

#[test]
fn it_closes_before_it_tells_on_dismiss_and_every_handler_is_told() {
  // What the signal said as each handler was told: a handler that opens something else finds the tip closed.
  let said = Arc::new(std::sync::Mutex::new(Vec::new()));
  let open = Signal::new(true);
  let mut stage = {
    let said = said.clone();
    stage(&open, move |anchor, open| {
      let (first, second) = ((said.clone(), open.clone()), (said.clone(), open.clone()));
      TeachingTip::new(open, anchor)
        .title(TITLE)
        .id("tip")
        .on_dismiss(move || first.0.lock().unwrap().push(("first", first.1.get_untracked())))
        .on_dismiss(move || second.0.lock().unwrap().push(("second", second.1.get_untracked())))
    })
  };
  stage.pass();
  stage.click("tip-close");
  assert_eq!(
    *said.lock().unwrap(),
    [("first", false), ("second", false)],
    "closed by its close control first, then each handler in turn"
  );

  said.lock().unwrap().clear();
  open.set(true);
  stage.pass();
  stage.key("Escape");
  stage.pass();
  assert_eq!(
    *said.lock().unwrap(),
    [("first", false), ("second", false)],
    "and by Escape"
  );
}

#[test]
fn without_dismiss_on_escape_escape_leaves_it_open() {
  let open = Signal::new(true);
  let mut stage = stage(&open, |anchor, open| {
    TeachingTip::new(open, anchor)
      .title(TITLE)
      .id("tip")
      .dismiss_on_escape(false)
  });
  stage.pass();
  stage.key("Escape");
  stage.pass();
  assert!(open.get(), "Escape does not close it");
  assert!(stage.paints(TITLE));
  stage.click("tip-close");
  assert!(!open.get(), "its close control still does");
}

#[test]
fn with_dismiss_on_outside_press_a_press_elsewhere_closes_it_and_tells_on_dismiss() {
  let (open, closed) = (Signal::new(true), Arc::new(AtomicUsize::new(0)));
  let mut stage = {
    let closed = closed.clone();
    stage(&open, move |anchor, open| {
      let closed = closed.clone();
      TeachingTip::new(open, anchor)
        .title(TITLE)
        .id("tip")
        .dismiss_on_outside_press(true)
        .on_dismiss(move || {
          closed.fetch_add(1, Ordering::SeqCst);
        })
    })
  };
  stage.pass();
  assert!(stage.paints(TITLE));
  stage.click("field");
  stage.pass();
  assert!(!open.get(), "a press elsewhere closes it");
  assert_eq!(closed.load(Ordering::SeqCst), 1, "and tells on_dismiss");
  assert!(!stage.paints(TITLE));
}

#[test]
fn its_buttons_call_what_they_are_given_and_children_add_what_it_holds() {
  let pressed = Arc::new(std::sync::Mutex::new(Vec::new()));
  let open = Signal::new(true);
  let mut stage = {
    let pressed = pressed.clone();
    stage(&open, move |anchor, open| {
      let (on, back) = (pressed.clone(), pressed.clone());
      TeachingTip::new(open, anchor)
        .title(TITLE)
        .children([Rect::new(20.0, 10.0).id("one"), Rect::new(20.0, 10.0).id("two")])
        .secondary("Not now", move || back.lock().unwrap().push("secondary"))
        .primary("Got it", move || on.lock().unwrap().push("primary"))
    })
  };
  stage.pass();
  let (_, one, _, height) = stage.bounds("one");
  let (_, two, ..) = stage.bounds("two");
  assert!(two >= one + height - 0.5, "one under the other");
  press_text(&mut stage, "Not now");
  press_text(&mut stage, "Got it");
  assert_eq!(*pressed.lock().unwrap(), ["secondary", "primary"]);
  assert!(open.get(), "its own buttons do what their handlers say, and no more");
}

#[test]
fn its_label_is_what_tooling_calls_it_and_its_title_where_it_has_none() {
  let app = App::new();
  let renderer: TeachingTipRenderer = Arc::new(|view: TeachingTipView| {
    Row::new()
      .part(&view.part(TeachingTipPart::Panel))
      .child(
        view
          .part(TeachingTipPart::Title)
          .text(&format!("called {}", view.label.as_deref().unwrap_or("nothing"))),
      )
      .into()
  });
  app.theme().set_renderer::<TeachingTip>(renderer);
  let (open, first, second) = (Signal::new(true), ElementRef::new(), ElementRef::new());
  let mut stage = Stage::in_app(app, move |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .padding(40.0)
      .spacing(120.0)
      .child(Rect::new(80.0, 24.0).ref_element(first.clone()))
      .child(Rect::new(80.0, 24.0).ref_element(second.clone()))
      .child(TeachingTip::new(open.clone(), first.clone()).title(TITLE).mount(ctx))
      .child(
        TeachingTip::new(open.clone(), second.clone())
          .title(TITLE)
          .label("Where the panels go")
          .mount(ctx),
      )
      .into()
  });
  stage.pass();
  assert!(stage.paints(&format!("called {TITLE}")), "by its title");
  assert!(stage.paints("called Where the panels go"), "by its label");
}

#[test]
fn an_offset_stands_it_that_far_from_what_it_points_at() {
  let open = Signal::new(true);
  let mut stage = stage(&open, |anchor, open| {
    TeachingTip::new(open, anchor).title(TITLE).offset(20.0)
  });
  stage.pass();
  let shown = panel(&stage, TITLE).unwrap();
  let (_, y, _, height) = stage.bounds("panels");
  assert!(
    close(shown.y, y + height + 20.0 + 9.0),
    "20 and its arrow's 9 under what it points at: {}",
    shown.y
  );
}

#[test]
fn a_collision_that_only_clamps_keeps_it_on_its_own_side() {
  let at_bottom = |collision: CollisionStrategy| {
    let (open, anchor) = (Signal::new(true), ElementRef::new());
    let mut stage = Stage::new(move |ctx: &mut Ctx| -> Element {
      Column::new()
        .size(400.0, 300.0)
        .padding_top(250.0)
        .align_items(Alignment::Center)
        .child(Rect::new(80.0, 24.0).id("target").ref_element(anchor.clone()))
        .child(
          TeachingTip::new(open.clone(), anchor.clone())
            .title(TITLE)
            .collision(collision)
            .mount(ctx),
        )
        .into()
    });
    stage.tree.resize(400, 300);
    stage.pass();
    stage.pass();
    let shown = panel(&stage, TITLE).unwrap();
    let (_, y, ..) = stage.bounds("target");
    shown.y + shown.height <= y
  };
  assert!(
    at_bottom(CollisionStrategy::FlipThenClamp),
    "no room under it: it goes above"
  );
  assert!(
    !at_bottom(CollisionStrategy::Clamp),
    "it stays on its side, moved into the window"
  );
}

#[test]
fn force_draws_its_close_control_focused_with_nothing_on_it() {
  let open = Signal::new(true);
  let mut forced = stage(&open, |anchor, open| {
    TeachingTip::new(open, anchor)
      .title(TITLE)
      .force(Some(Interaction::FocusVisible))
  });
  forced.pass();
  assert!(
    forced.has_ring(light(PaletteColor::Ring)),
    "the focus ring of its close control"
  );
  let mut rest = stage(&open, |anchor, open| TeachingTip::new(open, anchor).title(TITLE));
  rest.pass();
  assert!(!rest.has_ring(light(PaletteColor::Ring)));
}

/// Two targets 60 apart down the middle of the window, and the tour `make` makes of a step on each.
fn tour_stage(step: &Signal<Option<usize>>, make: impl Fn(Tour) -> Tour + Send + Sync + 'static) -> Stage {
  let (step, first, second) = (step.clone(), ElementRef::new(), ElementRef::new());
  Stage::new(move |ctx: &mut Ctx| -> Element {
    let tour = Tour::new(step.clone())
      .step(TourStep::new(first.clone(), "Switch themes here"))
      .step(TourStep::new(second.clone(), "Dock panels anywhere").id("dock"));
    Column::new()
      .size(400.0, 300.0)
      .padding_top(20.0)
      .align_items(Alignment::Center)
      .child(Rect::new(80.0, 24.0).id("first").ref_element(first.clone()))
      .child(Rect::new(1.0, 60.0))
      .child(Rect::new(80.0, 24.0).id("second").ref_element(second.clone()))
      .child(make(tour).mount(ctx))
      .into()
  })
}

#[test]
fn a_tours_labels_ids_and_rules_are_its_own_and_a_steps_id_comes_first() {
  let step = Signal::new(Some(0_usize));
  let mut stage = tour_stage(&step, |tour| {
    tour
      .back_label("Previous")
      .next_label("Onward")
      .done_label("Finish")
      .id("tour")
      .style(TeachingTipPart::Panel, |s| s.background(PaletteColor::Destructive))
  });
  stage.pass();
  assert!(stage.paints("Onward"));
  assert_eq!(
    rect_color(&panel(&stage, "Switch themes here").unwrap()),
    Some(light(PaletteColor::Destructive)),
    "its style is its tips'"
  );
  stage.click("tour-primary");
  assert_eq!(step.get(), Some(1));
  stage.pass();
  assert!(stage.paints("Previous"));
  assert!(stage.paints("Finish"));
  stage.click("dock-primary");
  assert_eq!(step.get(), None, "its done button ends it");
}

#[test]
fn escape_leaving_a_tour_tells_on_dismiss_and_done_does_not() {
  let (step, dismissed, finished) = (
    Signal::new(Some(1_usize)),
    Arc::new(AtomicUsize::new(0)),
    Arc::new(AtomicUsize::new(0)),
  );
  let mut stage = {
    let (dismissed, finished) = (dismissed.clone(), finished.clone());
    tour_stage(&step, move |tour| {
      let (dismissed, finished) = (dismissed.clone(), finished.clone());
      tour
        .on_dismiss(move || {
          dismissed.fetch_add(1, Ordering::SeqCst);
        })
        .on_finish(move || {
          finished.fetch_add(1, Ordering::SeqCst);
        })
    })
  };
  stage.pass();
  press_text(&mut stage, "Done");
  assert_eq!(step.get(), None);
  assert_eq!(
    (finished.load(Ordering::SeqCst), dismissed.load(Ordering::SeqCst)),
    (1, 0),
    "Done finishes it"
  );

  step.set(Some(0));
  stage.pass();
  stage.key("Escape");
  stage.pass();
  assert_eq!(step.get(), None, "Escape leaves the tour");
  assert_eq!(
    (finished.load(Ordering::SeqCst), dismissed.load(Ordering::SeqCst)),
    (1, 1),
    "and tells on_dismiss"
  );
}

#[test]
fn without_dismiss_on_escape_escape_leaves_the_tour_on() {
  let step = Signal::new(Some(0_usize));
  let mut stage = tour_stage(&step, |tour| tour.dismiss_on_escape(false));
  stage.pass();
  stage.key("Escape");
  stage.pass();
  assert_eq!(step.get(), Some(0));
  assert!(stage.paints("Switch themes here"));
}

#[test]
fn a_tour_converted_on_its_own_is_the_panel_of_its_step() {
  let shows = |current: Option<usize>| {
    let step = Signal::new(current);
    let mut stage = Stage::new(move |_| {
      let tour = Tour::new(step.clone())
        .step(TourStep::new(ElementRef::new(), "Switch themes here"))
        .step(TourStep::new(ElementRef::new(), "Dock panels anywhere"));
      Column::new().size(400.0, 300.0).padding(20.0).child(tour).into()
    });
    stage.pass();
    (
      stage.paints("Switch themes here"),
      stage.paints("Dock panels anywhere"),
      stage.paints("2 of 2"),
    )
  };
  assert_eq!(shows(None), (true, false, false), "its first step while it says none");
  assert_eq!(shows(Some(1)), (false, true, true), "the step it says");
}
