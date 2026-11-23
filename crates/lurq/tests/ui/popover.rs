//! A popover: its panel beside its anchor, its text and its body, where it
//! is on each side, what closes it and whom it tells, its trigger, where the
//! focus goes, its fade, drawn in place, and a theme's other panel.

use std::{
  sync::{Arc, Mutex},
  time::Duration,
};

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
  Popover::new(open, anchor)
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
fn the_text_and_the_body_are_spaced_as_the_design_has_them() {
  let theme = Theme::new();
  let recipe = theme.resolve::<Popover>(&Case::new(), None);
  let gap = |part: PopoverPart| recipe.part(part).rest().gap;
  assert_eq!(gap(PopoverPart::Panel), Some(SpacingValue::from(14.0)));
  assert_eq!(gap(PopoverPart::Text), Some(SpacingValue::from(4.0)));
  assert_eq!(gap(PopoverPart::Body), Some(SpacingValue::from(14.0)));
  assert_eq!(
    recipe.part(PopoverPart::Panel).rest().shadow,
    Some(BoxShadowValue::from(ShadowStyle::Md))
  );

  // Without a title or a description, what it holds is at the top of the panel.
  let open = Signal::new(true);
  let mut stage = Stage::new(page((20.0, 20.0), 120.0, &open, |anchor, open| {
    Popover::new(open, anchor)
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
    Popover::new(open, anchor)
      .id("panel")
      .title("Above")
      .placement(Placement::TopStart)
  }));
  let (x, y, _, height) = stage.bounds("panel");
  assert_eq!(x, 20.0);
  assert!((y + height - (200.0 - 4.0)).abs() < 0.01, "4 over the anchor");

  // Beside it, 4 from its end, at its top.
  let mut stage = Stage::new(page((20.0, 20.0), 60.0, &open, |anchor, open| {
    Popover::new(open, anchor)
      .id("panel")
      .title("Beside")
      .placement(Placement::RightStart)
  }));
  let (x, y, ..) = stage.bounds("panel");
  assert_eq!((x, y), (20.0 + 60.0 + 4.0, 20.0));

  // A width and an offset of its own.
  let mut stage = Stage::new(page((20.0, 20.0), 60.0, &open, |anchor, open| {
    Popover::new(open, anchor)
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
/// button to press. The page keeps their anchors from one render to the next, as an app does.
fn two_with_triggers(first: &Signal<bool>, second: &Signal<bool>) -> Stage {
  let (first, second) = (first.clone(), second.clone());
  let (first_anchor, second_anchor) = (ElementRef::new(), ElementRef::new());
  Stage::new(move |_| {
    Column::new()
      .size(400.0, 300.0)
      .padding(20.0)
      .spacing(8.0)
      .child(
        Row::new()
          .spacing(120.0)
          .child(
            Popover::new(first.clone(), first_anchor.clone())
              .id("first-panel")
              .trigger(Button::new("First").id("first"))
              .title("First")
              .child(Button::new("Apply").id("apply")),
          )
          .child(
            Popover::new(second.clone(), second_anchor.clone())
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
    .part(PopoverPart::Panel)
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

fn shift_tab(stage: &mut Stage) {
  let tab = || "Tab".to_owned();
  stage.tree.key_down(tab(), tab(), true, false, false);
  stage.tree.key_up(tab(), tab(), true, false, false);
  stage.pass();
}

#[test]
fn tab_goes_through_an_open_popover_right_after_its_trigger() {
  let (first, second) = (Signal::new(false), Signal::new(false));
  let mut stage = two_with_triggers(&first, &second);
  stage.click("first");
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("apply"));
  // From the last stop in the popover, Tab goes on to what follows its trigger, not to the page's first stop.
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("second"));
  // And back: into the popover, then to its trigger.
  shift_tab(&mut stage);
  assert_eq!(focused(&stage).as_deref(), Some("apply"));
  shift_tab(&mut stage);
  assert_eq!(focused(&stage).as_deref(), Some("first"));
  stage.key("Tab");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("apply"),
    "from the trigger into the popover"
  );
  assert!(first.get(), "moving through it leaves it open");
}

#[test]
fn a_title_and_a_description_longer_than_the_panel_go_on_to_more_lines_inside_it() {
  const TITLE: &str = "Dimensions of the layer and of everything it holds";
  const DESCRIPTION: &str = "Set the size of the layer; what it holds keeps its place against the edges.";
  let open = Signal::new(true);
  let mut stage = Stage::new(page((20.0, 20.0), 120.0, &open, |anchor, open| {
    Popover::new(open, anchor)
      .id("panel")
      .title(TITLE)
      .description(DESCRIPTION)
      .child(Row::new().height(36.0).id("content"))
  }));
  let (x, _, width, _) = stage.bounds("panel");
  for text in [TITLE, DESCRIPTION] {
    let quad = stage.text_quad(text).unwrap();
    assert!(
      quad.x + quad.width <= x + width - 15.5,
      "{text:?} inside the padding: {} in {}",
      quad.x + quad.width,
      x + width
    );
    assert!(quad.height > 24.0, "{text:?} on more lines: {}", quad.height);
  }
}

#[test]
fn every_handler_of_on_dismiss_is_told_when_a_person_closes_it_and_not_when_the_app_does() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let open = Signal::new(true);
  let held = told.clone();
  let mut stage = Stage::new(page((20.0, 20.0), 120.0, &open, move |anchor, open| {
    let (first, second) = (held.clone(), held.clone());
    dimensions(anchor, open)
      .on_dismiss(move || first.lock().unwrap().push("first"))
      .on_dismiss(move || second.lock().unwrap().push("second"))
  }));
  stage.key("Escape");
  assert!(!open.get());
  assert_eq!(*told.lock().unwrap(), ["first", "second"], "the second after the first");

  open.set(true);
  stage.pass();
  stage.click("outside");
  assert!(!open.get());
  assert_eq!(told.lock().unwrap().len(), 4, "a press outside tells both too");

  open.set(true);
  stage.pass();
  open.set(false);
  stage.pass();
  assert_eq!(told.lock().unwrap().len(), 4, "the app closing it tells no one");
}

#[test]
fn escape_and_a_press_outside_can_each_be_kept_from_closing_it() {
  let open = Signal::new(true);
  let mut stage = Stage::new(page((20.0, 20.0), 120.0, &open, |anchor, open| {
    dimensions(anchor, open).dismiss_on_escape(false)
  }));
  stage.key("Escape");
  assert!(open.get(), "Escape leaves it open");
  stage.click("outside");
  assert!(!open.get(), "a press outside still closes it");

  let open = Signal::new(true);
  let mut stage = Stage::new(page((20.0, 20.0), 120.0, &open, |anchor, open| {
    dimensions(anchor, open).dismiss_on_outside_press(false)
  }));
  stage.click("outside");
  assert!(open.get(), "a press outside leaves it open");
  stage.key("Escape");
  assert!(!open.get(), "Escape still closes it");
}

#[test]
fn an_inline_popover_is_drawn_where_it_is_declared_open_or_not() {
  let open = Signal::new(false);
  let mut stage = Stage::new(page((20.0, 20.0), 120.0, &open, |anchor, open| {
    dimensions(anchor, open).inline(true)
  }));
  assert!(stage.paints("Dimensions"), "closed, it is drawn");
  // In the row, after the anchor, and not under it.
  let (x, y, width, _) = stage.bounds("panel");
  assert_eq!((x, y, width), (20.0 + 120.0, 20.0, 292.0));
  stage.key("Escape");
  stage.pass();
  assert!(stage.paints("Dimensions"));
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn its_label_is_what_tooling_calls_it_in_place_of_its_title() {
  let said = |stage: &mut Stage, name: &str| {
    stage
      .tree
      .get_element_by_id_mut("panel")
      .and_then(|element| element.description(name))
  };
  let open = Signal::new(true);
  let mut stage = Stage::new(page((20.0, 20.0), 120.0, &open, dimensions));
  assert_eq!(said(&mut stage, "a11y_role").as_deref(), Some("dialog"));
  assert_eq!(said(&mut stage, "a11y_name").as_deref(), Some("Dimensions"));
  let mut stage = Stage::new(page((20.0, 20.0), 120.0, &open, |anchor, open| {
    dimensions(anchor, open).label("Size of the layer")
  }));
  assert_eq!(said(&mut stage, "a11y_name").as_deref(), Some("Size of the layer"));
}

/// A button that is a component of its own, with how often it was pressed in what it says.
struct Pressed {
  times: Signal<u32>,
}

impl lurq::app::component::Component for Pressed {
  type Props = ();

  fn create(ctx: &mut lurq::app::ctx::Ctx) -> Self {
    Self { times: ctx.signal(0) }
  }

  fn render(&self, _: &mut lurq::app::ctx::Ctx) -> impl Into<lurq::node::Element> {
    let times = self.times.clone();
    Button::new(&format!("Pressed {}", self.times.get()))
      .id("own")
      .on_click(move |_| times.set(times.get_untracked() + 1))
  }
}

#[test]
fn a_trigger_that_is_a_component_opens_it_too_and_still_after_it_renders_again_by_itself() {
  let open = Signal::new(false);
  let (shown, anchor) = (open.clone(), ElementRef::new());
  let mut stage = Stage::new(move |_| {
    Column::new()
      .size(400.0, 300.0)
      .padding(20.0)
      .child(
        Popover::new(shown.clone(), anchor.clone())
          .id("panel")
          .trigger(lurq::node::Element::component::<Pressed>(()))
          .title("Options"),
      )
      .into()
  });
  // The press is the trigger's own and the popover's: the button counts it, and the popover opens.
  stage.click("own");
  assert!(
    open.get(),
    "what the popover puts on its trigger is on the root the component rendered"
  );
  assert!(stage.paints("Pressed 1"));
  let (tx, ty, _, theight) = stage.bounds("own");
  let (px, py, ..) = stage.bounds("panel");
  assert_eq!((px, py), (tx, ty + theight + 4.0), "and that root is its anchor");

  // The button rendered again by itself, for its count: its new root has it still.
  stage.click("own");
  assert!(!open.get());
  assert!(stage.paints("Pressed 2"));
}

/// The app's keyframes: the panel rises 4 as it fades in.
const RISE: lurq::animation::KeyframesId = lurq::animation::KeyframesId::new(7);

/// An app whose popover's panel rises as it opens, by keyframes set on its theme, with what `more` says of the
/// panel besides and motion reduced or not.
fn rising(reduced: bool, more: impl FnOnce(lurq::ui::PartStyle) -> lurq::ui::PartStyle + 'static) -> App {
  use lurq::{animation::Keyframes, ui::PartAnimation};
  let app = App::new();
  let theme = app.theme().clone();
  theme.set_reduced_motion(Some(reduced));
  theme.set_keyframes(
    Keyframes::new(RISE)
      .frame(0.0, |k| {
        k.opacity(0.0).translate(0.0, 4.0);
      })
      .frame(1.0, |k| {
        k.opacity(1.0).translate(0.0, 0.0);
      }),
  );
  theme.patch_recipe::<Popover>(|recipe| {
    recipe.base(PopoverPart::Panel, |s| {
      more(s.enter(PartAnimation::new(RISE, MotionDuration::Base, MotionEasing::Enter)))
    })
  });
  app
}

/// How much of the open panel is seen: the opacity of the faded layer it is painted in, 1 where none is.
fn seen(stage: &mut Stage) -> f32 {
  let snapshot = crate::support::render_pass_with_app(&mut stage.tree, &mut stage.app);
  snapshot.layers.iter().map(|layer| layer.opacity).fold(1.0, f32::min)
}

/// Opens the popover of `app`, and says how far below its place its title is at the first frame and how much of
/// the panel is seen then; the panel is left at rest.
fn opened(app: App) -> (Stage, Signal<bool>, (f32, f32)) {
  let open = Signal::new(false);
  let mut stage = Stage::in_app(app, page((20.0, 20.0), 120.0, &open, dimensions));
  open.set(true);
  stage.pass_after(Duration::ZERO);
  let first = stage.text_quad("Dimensions").expect("the title is painted").y;
  let faded = seen(&mut stage);
  stage.pass();
  let rest = stage.text_quad("Dimensions").expect("the title is painted").y;
  (stage, open, (first - rest, faded))
}

fn near(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.01
}

#[test]
fn keyframes_set_on_the_theme_are_what_a_recipe_has_the_panel_play() {
  let (mut stage, _, (down, faded)) = opened(rising(false, |s| s));
  assert!(
    near(down, 4.0) && near(faded, 0.0),
    "it starts 4 down, unseen: {down} {faded}"
  );
  assert!(near(seen(&mut stage), 1.0), "and rests seen");
}

#[test]
fn keyframes_set_after_the_window_opened_are_played_too() {
  use lurq::{animation::Keyframes, ui::PartAnimation};
  const DROP: lurq::animation::KeyframesId = lurq::animation::KeyframesId::new(8);
  let app = App::new();
  let theme = app.theme().clone();
  let open = Signal::new(false);
  let mut stage = Stage::in_app(app, page((20.0, 20.0), 120.0, &open, dimensions));
  theme.set_keyframes(
    Keyframes::new(DROP)
      .frame(0.0, |k| {
        k.translate(0.0, -6.0);
      })
      .frame(1.0, |k| {
        k.translate(0.0, 0.0);
      }),
  );
  theme.patch_recipe::<Popover>(|recipe| {
    recipe.base(PopoverPart::Panel, |s| {
      s.enter(PartAnimation::new(DROP, MotionDuration::Base, MotionEasing::Enter))
    })
  });
  open.set(true);
  stage.pass_after(Duration::ZERO);
  let first = stage.text_quad("Dimensions").unwrap().y;
  stage.pass();
  let rest = stage.text_quad("Dimensions").unwrap().y;
  assert!(near(first - rest, -6.0), "{first} {rest}");
}

#[test]
fn under_reduced_motion_the_panel_plays_what_its_recipe_says_it_plays_then() {
  use lurq::ui::PartAnimation;
  // Unsaid, it rises as ever.
  let (_, _, (down, _)) = opened(rising(true, |s| s));
  assert!(near(down, 4.0), "{down}");

  // A fade in place of the rise.
  let (_, _, (down, faded)) = opened(rising(true, |s| {
    s.reduced_enter(Some(PartAnimation::new(
      FADE_IN,
      MotionDuration::Base,
      MotionEasing::Enter,
    )))
  }));
  assert!(
    near(down, 0.0) && near(faded, 0.0),
    "it fades in where it is: {down} {faded}"
  );

  // Nothing: it is there at once, and gone at once.
  let (mut stage, open, (down, faded)) = opened(rising(true, |s| s.reduced_enter(None).reduced_exit(None)));
  assert!(near(down, 0.0) && near(faded, 1.0), "there at once: {down} {faded}");
  open.set(false);
  stage.pass_after(Duration::ZERO);
  assert!(!stage.paints("Dimensions"), "gone at once");

  // With motion not reduced, the same recipe rises and fades out.
  let (mut stage, open, (down, _)) = opened(rising(false, |s| s.reduced_enter(None).reduced_exit(None)));
  assert!(near(down, 4.0), "{down}");
  open.set(false);
  stage.pass_after(Duration::ZERO);
  assert!(stage.paints("Dimensions"), "fading out");
}

/// An app whose popover's panel moves in away from its anchor: down from 4 over it where it is under it, up from 4
/// under it where it is over it.
fn away_from_the_anchor() -> App {
  use lurq::{
    animation::Keyframes,
    ui::{PartAnimation, TooltipSide},
  };
  const DOWN: lurq::animation::KeyframesId = lurq::animation::KeyframesId::new(9);
  const UP: lurq::animation::KeyframesId = lurq::animation::KeyframesId::new(10);
  let app = App::new();
  let theme = app.theme().clone();
  for (id, from) in [(DOWN, -4.0), (UP, 4.0)] {
    theme.set_keyframes(
      Keyframes::new(id)
        .frame(0.0, move |k| {
          k.translate(0.0, from);
        })
        .frame(1.0, |k| {
          k.translate(0.0, 0.0);
        }),
    );
  }
  theme.patch_recipe::<Popover>(|recipe| {
    recipe
      .state(TooltipSide::Bottom, PopoverPart::Panel, |s| {
        s.enter(PartAnimation::new(DOWN, MotionDuration::Base, MotionEasing::Enter))
      })
      .state(TooltipSide::Top, PopoverPart::Panel, |s| {
        s.enter(PartAnimation::new(UP, MotionDuration::Base, MotionEasing::Enter))
      })
  });
  app
}

/// How far below its place at rest the popover's title is as it opens over an anchor `y` down the page.
fn opened_at(y: f32) -> f32 {
  let open = Signal::new(false);
  let over = |anchor, open| dimensions(anchor, open).placement(lurq::app::ctx::Placement::TopStart);
  let mut stage = Stage::in_app(away_from_the_anchor(), page((20.0, y), 120.0, &open, over));
  open.set(true);
  stage.pass_after(Duration::ZERO);
  let first = stage.text_quad("Dimensions").expect("the title is painted").y;
  stage.pass();
  let rest = stage.text_quad("Dimensions").expect("the title is painted").y;
  first - rest
}

#[test]
fn a_recipe_moves_the_panel_in_away_from_its_anchor_on_the_side_it_opened_on() {
  let over = opened_at(200.0);
  assert!(near(over, 4.0), "over its anchor it comes up: {over}");
  let flipped = opened_at(20.0);
  assert!(
    near(flipped, -4.0),
    "with no room over it, it flips under its anchor and comes down: {flipped}"
  );
}
