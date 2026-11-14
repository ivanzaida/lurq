//! A popconfirm: its panel under what was pressed, its question and its two
//! buttons where the board has them, where the focus goes, what each button,
//! Escape and a press outside do, the destructive look, and a theme's rules.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App,
    ctx::Ctx,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Rect, Row},
  core::{ElementRef, Signal},
  layout::quad::QuadContent,
  node::{Element, color::Color},
  ui::{Button, ButtonVariant, FontFamily, Popconfirm, PopconfirmPart},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// Counts what is pressed.
fn counter() -> (Arc<AtomicUsize>, impl Fn() + Send + Sync + Clone + 'static) {
  let count = Arc::new(AtomicUsize::new(0));
  let add = count.clone();
  (count, move || {
    add.fetch_add(1, Ordering::SeqCst);
  })
}

fn count(counter: &Arc<AtomicUsize>) -> usize {
  counter.load(Ordering::SeqCst)
}

/// A page of 400 by 300 with a button of the app's at (20, 20) that the popconfirm hangs from, and something to
/// press outside it. The page has a size of its own, so the popup has room in it.
fn page(
  popconfirm: impl Fn(ElementRef) -> Popconfirm + Send + Sync + 'static,
) -> impl Fn(&mut Ctx) -> Element + Send + Sync + 'static {
  let anchor = ElementRef::new();
  move |_| {
    Column::new()
      .size(400.0, 300.0)
      .padding_left(20.0)
      .padding_top(20.0)
      .child(Row::new().child(popconfirm(anchor.clone())))
      .child(Rect::new(1.0, 180.0))
      .child(Rect::new(40.0, 20.0).id("outside"))
      .into()
  }
}

/// The board's popconfirm: delete this file, its trigger a button of the app's.
fn delete_file(
  anchor: ElementRef,
  open: Signal<bool>,
  confirmed: impl Fn() + Send + Sync + 'static,
  cancelled: impl Fn() + Send + Sync + 'static,
) -> Popconfirm {
  Popconfirm::new(anchor, open)
    .id("panel")
    .trigger(Button::new("Delete file").variant(ButtonVariant::Outline).id("trigger"))
    .title("Delete this file?")
    .description("It will be moved to the trash.")
    .confirm("Yes", confirmed)
    .cancel("No", cancelled)
}

/// The filled rectangles of `color` 28 high: the buttons.
fn buttons_of(stage: &Stage, color: Color) -> usize {
  stage
    .quads()
    .iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color) && quad.height == 28.0
    })
    .count()
}

#[test]
fn an_open_popconfirm_is_the_panel_of_the_board_under_what_was_pressed() {
  let open = Signal::new(true);
  let held = open.clone();
  let mut stage = Stage::new(page(move |anchor| delete_file(anchor, held.clone(), || {}, || {})));
  let (_, trigger_y, _, trigger_height) = stage.bounds("trigger");
  let (x, y, width, _) = stage.bounds("panel");
  assert_eq!(width, 250.0);
  assert_eq!(x, 20.0, "at the start of what was pressed");
  assert_eq!(y, trigger_y + trigger_height + 4.0, "under it");
  assert_eq!(stage.fill_color("panel"), Some(light(PaletteColor::Popover)));

  // The title of 13 and the description of 12 in the muted color, after the icon of 16 and 8, 12 from the edges.
  assert_eq!(stage.text("Delete this file?"), (light(PaletteColor::Foreground), 13.0));
  assert_eq!(
    stage.text("It will be moved to the trash."),
    (light(PaletteColor::MutedForeground), 12.0)
  );
  let title = stage.text_quad("Delete this file?").unwrap();
  assert!((title.x - (x + 12.0 + 16.0 + 8.0)).abs() < 0.5, "{} {x}", title.x);

  // Two buttons of 28 at the end, No before Yes, their labels 13.
  assert_eq!(buttons_of(&stage, light(PaletteColor::Primary)), 1);
  let (no, yes) = (stage.text_quad("No").unwrap(), stage.text_quad("Yes").unwrap());
  assert!(no.x < yes.x);
  assert_eq!(stage.text("Yes").1, 13.0);
  assert!(
    yes.x + yes.width < x + width - 12.0 - 9.0,
    "padded 10 inside the panel's 12"
  );
  assert!(yes.x + yes.width > x + width - 12.0 - 11.0);
}

#[test]
fn the_trigger_opens_it_and_the_focus_goes_to_the_cancel_button() {
  let open = Signal::new(false);
  let (confirmed, confirm) = counter();
  let (cancelled, cancel) = counter();
  let held = open.clone();
  let mut stage = Stage::new(page(move |anchor| {
    delete_file(anchor, held.clone(), confirm.clone(), cancel.clone())
  }));
  stage.click("trigger");
  assert!(open.get());
  // Space presses the focused button: the cancel one.
  stage.key(" ");
  assert_eq!((count(&confirmed), count(&cancelled)), (0, 1));
  assert!(!open.get());
}

#[test]
fn the_confirm_button_closes_it_and_confirms() {
  let open = Signal::new(true);
  let (confirmed, confirm) = counter();
  let (cancelled, cancel) = counter();
  let held = open.clone();
  let mut stage = Stage::new(page(move |anchor| {
    delete_file(anchor, held.clone(), confirm.clone(), cancel.clone())
  }));
  let yes = stage.text_quad("Yes").unwrap();
  stage.tree.mouse_move(yes.x + 2.0, yes.y + 2.0);
  stage
    .tree
    .mouse_down(yes.x + 2.0, yes.y + 2.0, lurq::app::events::MouseButton::Left);
  stage
    .tree
    .mouse_up(yes.x + 2.0, yes.y + 2.0, lurq::app::events::MouseButton::Left);
  stage.pass();
  assert_eq!((count(&confirmed), count(&cancelled)), (1, 0));
  assert!(!open.get());
}

#[test]
fn escape_cancels_and_a_press_outside_closes_it() {
  let open = Signal::new(false);
  let (confirmed, confirm) = counter();
  let (cancelled, cancel) = counter();
  let held = open.clone();
  let mut stage = Stage::new(page(move |anchor| {
    delete_file(anchor, held.clone(), confirm.clone(), cancel.clone())
  }));
  // Opened by its trigger, it has the focus.
  stage.click("trigger");
  assert!(open.get());
  stage.key("Escape");
  assert!(!open.get());
  assert_eq!((count(&confirmed), count(&cancelled)), (0, 1));

  open.set(true);
  stage.pass();
  stage.click("outside");
  assert!(!open.get());
  assert_eq!(count(&confirmed), 0);
}

#[test]
fn a_destructive_popconfirm_confirms_in_the_destructive_color() {
  let stage = Stage::new(page(|anchor| {
    delete_file(anchor, Signal::new(true), || {}, || {}).destructive(true)
  }));
  assert_eq!(buttons_of(&stage, light(PaletteColor::Destructive)), 1);
  assert_eq!(buttons_of(&stage, light(PaletteColor::Primary)), 0);
}

#[test]
fn a_theme_styles_its_buttons_apart_from_buttons_elsewhere() {
  let app = App::new();
  app.theme().patch_recipe::<Popconfirm>(|rules| {
    rules
      .base(PopconfirmPart::ConfirmLabel, |s| s.font_family(FontFamily::Mono))
      .base(PopconfirmPart::Panel, |s| s.width(300.0))
  });
  let mut stage = Stage::in_app(app, page(|anchor| delete_file(anchor, Signal::new(true), || {}, || {})));
  assert_eq!(stage.font_of("Yes"), "monospace");
  let system = stage.app.theme().sans_family().to_string();
  assert_eq!(stage.font_of("No"), system);
  assert_eq!(stage.font_of("Delete file"), system);
  assert_eq!(stage.bounds("panel").2, 300.0);
}
