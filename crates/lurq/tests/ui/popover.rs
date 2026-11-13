//! A popover: its panel beside its anchor, its header and its body, where it
//! is on each side, what closes it, its trigger, where the focus goes, its
//! fade, and a theme's other panel.

use std::{sync::Arc, time::Duration};

use lurq::{
  app::{
    App,
    theme::{MotionDuration, MotionEasing, PaletteColor, ShadowStyle, Theme, ThemePalette},
    window::Appearance,
  },
  components::{Column, Placement, Rect, Row},
  core::{ElementRef, Signal},
  layout::quad::{Quad, QuadContent},
  node::{BoxShadowValue, SpacingValue, color::Color},
  ui::{
    Button, Case, FADE_IN, FADE_OUT, Popover, PopoverPart, PopoverRenderer, PopoverView, TransitionDuration,
    TransitionEasing,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// A page of 400 by 300 with the anchor at `(x, y)`, `width` wide and 32 high, and what `popover` makes of it
/// beside it. The page has a size of its own, so the popup has room in it.
fn page(
  (x, y): (f32, f32),
  width: f32,
  open: &Signal<bool>,
  popover: impl Fn(ElementRef, Signal<bool>) -> Popover + Send + Sync + 'static,
) -> impl Fn(&mut lurq::app::ctx::Ctx) -> lurq::node::Element + Send + Sync + 'static {
  let (anchor, open) = (ElementRef::new(), open.clone());
  move |_| {
    Column::new()
      .size(400.0, 300.0)
      .padding_left(x)
      .padding_top(y)
      .child(
        Row::new()
          .child(Rect::new(width, 32.0).id("anchor").ref_element(anchor.clone()))
          .child(popover(anchor.clone(), open.clone())),
      )
      // Clear of the panel under the anchor.
      .child(Rect::new(1.0, 190.0))
      .child(Rect::new(40.0, 20.0).id("outside"))
      .into()
  }
}

/// The popover of the design: the title, the description, and something 36 high to hold.
fn dimensions(anchor: ElementRef, open: Signal<bool>) -> Popover {
  Popover::new(anchor, open)
    .id("panel")
    .title("Dimensions")
    .description("Set the size of the layer.")
    .child(Row::new().height(36.0).id("content"))
}

/// The edge of the element with `id`: its width and its color.
fn edge(stage: &mut Stage, id: &str) -> (f32, Color) {
  let quad: Quad = stage.border(id).unwrap_or_else(|| panic!("#{id} should have an edge"));
  let side = quad.border.and_then(|border| border.top).expect("an edge on top");
  (side.width, side.color)
}

#[test]
fn an_open_popover_is_the_panel_of_the_design_under_its_anchor() {
  let open = Signal::new(true);
  let mut stage = Stage::new(page((20.0, 20.0), 120.0, &open, dimensions));
  let (x, y, width, height) = stage.bounds("panel");
  // Under the anchor at its start, the theme's smallest spacing from it.
  assert_eq!((x, y), (20.0, 20.0 + 32.0 + 4.0));
  assert_eq!(width, 292.0);
  assert_eq!(stage.fill_color("panel"), Some(light(PaletteColor::Popover)));
  assert_eq!(edge(&mut stage, "panel"), (1.0, light(PaletteColor::Border)));
  let panel = stage.fill("panel").unwrap();
  assert!(
    panel.border_radius.is_some_and(|radius| radius.top_left == 8.0),
    "the large radius"
  );
  let shadows: Vec<_> = stage
    .quads()
    .into_iter()
    .filter_map(|quad| match quad.content {
      QuadContent::BoxShadow(shadow) if (quad.x, quad.y) == (x, y) => Some((shadow.offset_y, shadow.blur)),
      _ => None,
    })
    .collect();
  assert!(
    shadows.contains(&(4.0, 8.0)) && shadows.contains(&(1.0, 2.0)),
    "the medium shadow: {shadows:?}"
  );

  // The title in the body's type and the popover's foreground, the description in 13 and the muted one, 16
  // in from the edge.
  assert_eq!(stage.text("Dimensions"), (light(PaletteColor::PopoverForeground), 14.0));
  assert_eq!(
    stage.text("Set the size of the layer."),
    (light(PaletteColor::MutedForeground), 13.0)
  );
  let (title, description) = (
    stage.text_quad("Dimensions").unwrap(),
    stage.text_quad("Set the size of the layer.").unwrap(),
  );
  assert_eq!(title.x - x, 16.0);
  assert!((title.y - y - 16.0).abs() < 0.5, "16 from the top: {}", title.y - y);
  assert!(description.y > title.y, "the description under the title");

  // What it holds is as wide as the panel less its padding, and 16 from its bottom.
  let (cx, cy, cwidth, cheight) = stage.bounds("content");
  assert_eq!((cx - x, cwidth, cheight), (16.0, 260.0, 36.0));
  assert!((y + height - (cy + cheight) - 16.0).abs() < 0.5, "16 from the bottom");
}

#[test]
fn the_header_and_the_body_are_spaced_as_the_design_has_them() {
  let theme = Theme::new();
  let recipe = theme.resolve::<Popover>(&Case::new(), None);
  let gap = |part: PopoverPart| recipe.part(part).rest().gap;
  assert_eq!(gap(PopoverPart::Root), Some(SpacingValue::from(14.0)));
  assert_eq!(gap(PopoverPart::Header), Some(SpacingValue::from(4.0)));
  assert_eq!(gap(PopoverPart::Body), Some(SpacingValue::from(14.0)));
  assert_eq!(
    recipe.part(PopoverPart::Root).rest().shadow,
    Some(BoxShadowValue::from(ShadowStyle::Md))
  );

  // Without a title or a description, what it holds is at the top of the panel.
  let open = Signal::new(true);
  let mut stage = Stage::new(page((20.0, 20.0), 120.0, &open, |anchor, open| {
    Popover::new(anchor, open)
      .id("panel")
      .child(Row::new().height(36.0).id("content"))
  }));
  let (_, y, _, height) = stage.bounds("panel");
  let (_, cy, ..) = stage.bounds("content");
  assert_eq!((cy - y, height), (16.0, 16.0 + 36.0 + 16.0));
}

#[test]
fn a_popover_is_shown_while_its_signal_says_so() {
  let open = Signal::new(false);
  let mut stage = Stage::new(page((20.0, 20.0), 120.0, &open, dimensions));
  assert!(!stage.paints("Dimensions"));
  open.set(true);
  stage.pass();
  assert!(stage.paints("Dimensions"));
  // It fades out, and then it is gone.
  open.set(false);
  stage.pass();
  stage.pass();
  assert!(!stage.paints("Dimensions"));
}

#[test]
fn escape_or_a_press_outside_closes_it_and_a_press_inside_does_not() {
  let open = Signal::new(true);
  let mut stage = Stage::new(page((20.0, 20.0), 120.0, &open, dimensions));
  stage.click("content");
  assert!(open.get(), "a press inside leaves it open");
  stage.click("anchor");
  assert!(open.get(), "a press of the anchor is not outside it");
  stage.key("Escape");
  assert!(!open.get(), "Escape closes it");
  stage.pass();
  assert!(!stage.paints("Dimensions"));

  open.set(true);
  stage.pass();
  assert!(stage.paints("Dimensions"));
  stage.click("outside");
  assert!(!open.get(), "a press outside closes it");
}

#[test]
fn a_popover_is_on_the_side_it_is_placed_on() {
  // Over the anchor, its bottom 4 from the anchor's top.
  let open = Signal::new(true);
  let mut stage = Stage::new(page((20.0, 200.0), 120.0, &open, |anchor, open| {
    Popover::new(anchor, open)
      .id("panel")
      .title("Above")
      .placement(Placement::TopStart)
  }));
  let (x, y, _, height) = stage.bounds("panel");
  assert_eq!(x, 20.0);
  assert!((y + height - (200.0 - 4.0)).abs() < 0.01, "4 over the anchor");

  // Beside it, 4 from its end, at its top.
  let mut stage = Stage::new(page((20.0, 20.0), 60.0, &open, |anchor, open| {
    Popover::new(anchor, open)
      .id("panel")
      .title("Beside")
      .placement(Placement::RightStart)
  }));
  let (x, y, ..) = stage.bounds("panel");
  assert_eq!((x, y), (20.0 + 60.0 + 4.0, 20.0));

  // A width and an offset of its own.
  let mut stage = Stage::new(page((20.0, 20.0), 60.0, &open, |anchor, open| {
    Popover::new(anchor, open)
      .id("panel")
      .title("Wide")
      .width(340.0)
      .offset(8.0)
  }));
  let (_, y, width, _) = stage.bounds("panel");
  assert_eq!((y, width), (20.0 + 32.0 + 8.0, 340.0));
}

#[test]
fn a_popover_in_a_subtree_with_a_theme_is_in_that_theme() {
  let dark = ThemePalette::dark().get(PaletteColor::Popover);
  let app = App::new();
  app.theme().set_preferred_appearance(Some(Appearance::Light));
  let panel = Theme::new();
  panel.set_preferred_appearance(Some(Appearance::Dark));
  let open = Signal::new(true);
  let scene = page((20.0, 20.0), 120.0, &open, dimensions);
  let mut stage = Stage::in_app(app, move |ctx| {
    let inside = ctx.in_theme(&panel, |ctx| scene(ctx));
    Column::new().size(400.0, 300.0).child(inside).into()
  });
  assert_eq!(stage.fill_color("panel"), Some(dark));
}

#[test]
fn a_theme_gives_the_popover_another_panel_and_it_is_still_beside_its_anchor() {
  let app = App::new();
  // A panel with the title alone in it, over what it holds.
  let renderer: PopoverRenderer = Arc::new(|view: PopoverView| {
    let title = view.title.clone().unwrap_or_default();
    view
      .panel()
      .child(view.part(PopoverPart::Title).text(&format!("{title}!")))
      .into()
  });
  app.theme().set_renderer::<Popover>(renderer);
  let open = Signal::new(true);
  let mut stage = Stage::in_app(app, page((20.0, 20.0), 120.0, &open, dimensions));
  assert!(stage.paints("Dimensions!"));
  assert!(!stage.paints("Set the size of the layer."));
  assert_eq!(
    stage.fill_color("panel"),
    Some(light(PaletteColor::Popover)),
    "the recipe still styles it"
  );
  assert_eq!(stage.bounds("panel").1, 56.0);
  stage.key("Escape");
  assert!(!open.get(), "and it still closes");
}

/// A page with two popovers, each opened by its own button, and something outside both. The first holds a
/// button to press.
fn two_with_triggers(first: &Signal<bool>, second: &Signal<bool>) -> Stage {
  let (first, second) = (first.clone(), second.clone());
  Stage::new(move |_| {
    Column::new()
      .size(400.0, 300.0)
      .padding(20.0)
      .spacing(8.0)
      .child(
        Row::new()
          .spacing(120.0)
          .child(
            Popover::new(ElementRef::new(), first.clone())
              .id("first-panel")
              .trigger(Button::new("First").id("first"))
              .title("First")
              .child(Button::new("Apply").id("apply")),
          )
          .child(
            Popover::new(ElementRef::new(), second.clone())
              .id("second-panel")
              .trigger(Button::new("Second").id("second"))
              .title("Second"),
          ),
      )
      // Clear of either panel.
      .child(Rect::new(1.0, 170.0))
      .child(Rect::new(40.0, 20.0).id("outside"))
      .into()
  })
}

#[test]
fn its_trigger_opens_and_closes_it_and_is_its_anchor() {
  let (first, second) = (Signal::new(false), Signal::new(false));
  let mut stage = two_with_triggers(&first, &second);
  stage.click("first");
  assert!(first.get(), "a press of the trigger opens it");
  let (tx, ty, _, theight) = stage.bounds("first");
  let (px, py, ..) = stage.bounds("first-panel");
  assert_eq!((px, py), (tx, ty + theight + 4.0), "under its trigger");
  stage.click("first");
  assert!(!first.get(), "and closes it");

  // One press of the other trigger closes the open popover and opens the other.
  stage.click("first");
  stage.click("second");
  assert_eq!((first.get(), second.get()), (false, true));
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

#[test]
fn the_focus_goes_into_it_as_it_opens_and_back_to_its_trigger_as_it_closes() {
  let (first, second) = (Signal::new(false), Signal::new(false));
  let mut stage = two_with_triggers(&first, &second);
  stage.click("first");
  stage.pass();
  assert_eq!(
    focused(&stage).as_deref(),
    Some("apply"),
    "the first thing in it that takes the focus"
  );
  stage.key("Escape");
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("first"), "back to the trigger");

  // From the keyboard: Enter on the trigger opens it, and a press outside closes it.
  stage.key("Enter");
  stage.pass();
  assert!(first.get());
  assert_eq!(focused(&stage).as_deref(), Some("apply"));
  stage.click("outside");
  stage.pass();
  assert!(!first.get());
  assert_eq!(focused(&stage).as_deref(), Some("first"));

  // With nothing in it that takes the focus, the focus stays on the trigger.
  stage.click("second");
  stage.pass();
  assert!(second.get());
  assert_eq!(focused(&stage).as_deref(), Some("second"));
}

#[test]
fn it_fades_in_over_the_base_duration_and_out_over_the_fast_one() {
  let theme = Theme::new();
  let root = theme
    .resolve::<Popover>(&Case::new(), None)
    .part(PopoverPart::Root)
    .rest();
  let enter = root.enter.expect("an animation as it opens");
  assert_eq!(enter.keyframes, FADE_IN);
  assert_eq!(enter.duration, TransitionDuration::Role(MotionDuration::Base));
  assert_eq!(enter.easing, TransitionEasing::Role(MotionEasing::Enter));
  let exit = root.exit.expect("an animation as it closes");
  assert_eq!(exit.keyframes, FADE_OUT);
  assert_eq!(exit.duration, TransitionDuration::Role(MotionDuration::Fast));
  assert_eq!(exit.easing, TransitionEasing::Role(MotionEasing::Exit));

  // As it closes it is still drawn while it fades, and then it is gone.
  let open = Signal::new(true);
  let mut stage = Stage::new(page((20.0, 20.0), 120.0, &open, dimensions));
  open.set(false);
  stage.pass_after(Duration::ZERO);
  assert!(stage.paints("Dimensions"), "fading out");
  stage.pass();
  assert!(!stage.paints("Dimensions"));
}
