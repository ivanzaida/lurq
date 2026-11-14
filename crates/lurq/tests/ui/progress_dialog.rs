//! A progress dialog: its panel, its header, its bar, how much is done and
//! what is left where the board has them, that nothing dismisses it, its
//! cancel button and its other buttons, its detail, and a theme's rules.

use std::{
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
  time::Duration,
};

use lurq::{
  app::{
    App,
    ctx::Ctx,
    events::MouseButton,
    theme::{PaletteColor, ThemePalette},
  },
  components::Column,
  core::Signal,
  layout::{
    Constraints, Size,
    quad::{Quad, QuadContent},
  },
  node::{Element, color::Color},
  ui::{ProgressDialog, ProgressDialogPart, icons},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// A stage whose window is 600 by 400, the size of its page.
fn stage(scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Stage {
  stage_in(App::new(), scene)
}

fn stage_in(app: App, scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::in_app(app, move |ctx| {
    Column::new().size(600.0, 400.0).child(scene(ctx)).into()
  });
  // The stage lays its scene out in 400 by 300 and culls what is outside that: a window of 600 by 400 is that too.
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(600.0, 400.0))));
  stage.tree.resize(600, 400);
  stage.pass();
  stage
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

fn press_at(stage: &mut Stage, x: f32, y: f32) {
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

fn press_text(stage: &mut Stage, content: &str) {
  let quad = stage
    .text_quad(content)
    .unwrap_or_else(|| panic!("no text {content:?} was painted"));
  press_at(stage, quad.x + quad.width / 2.0, quad.y + quad.height / 2.0);
}

/// The filled rectangles of `color` `height` high.
fn bars(stage: &Stage, color: Color, height: f32) -> Vec<Quad> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color) && quad.height == height
    })
    .collect()
}

/// The board's progress dialog: copying 128 items.
fn copying(
  open: Signal<bool>,
  detail: Signal<bool>,
  cancel: impl Fn() + Send + Sync + 'static,
  pause: impl Fn() + Send + Sync + 'static,
) -> ProgressDialog {
  ProgressDialog::new(open)
    .id("dialog")
    .icon(icons::COPY)
    .title("Copying 128 items")
    .message("from Downloads to projects/lurq/assets")
    .value(32.0)
    .max(128.0)
    .format(|done, all| format!("{done} of {all}"))
    .status("about 2 min left")
    .detail("assets/icons/warning.svg", detail)
    .action("Pause", pause)
    .on_cancel(cancel)
}

#[test]
fn an_open_progress_dialog_is_the_panel_of_the_board() {
  let mut stage = stage(|_| copying(Signal::new(true), Signal::new(false), || {}, || {}).into());
  let (x, y, width, height) = stage.bounds("dialog");
  assert_eq!(width, 420.0);
  assert!((y + height / 2.0 - 200.0).abs() < 0.6, "in the middle: {y} {height}");
  assert_eq!(stage.fill_color("dialog"), Some(light(PaletteColor::Popover)));

  // The title of 15 and the line of 12 in the muted color, after the icon of 20 and 12.
  assert_eq!(stage.text("Copying 128 items"), (light(PaletteColor::Foreground), 15.0));
  assert_eq!(
    stage.text("from Downloads to projects/lurq/assets"),
    (light(PaletteColor::MutedForeground), 12.0)
  );
  let title = stage.text_quad("Copying 128 items").unwrap();
  assert!((title.x - (x + 20.0 + 20.0 + 12.0)).abs() < 0.5, "{} {x}", title.x);

  // The bar of 6 across the panel, a quarter of it filled.
  let rails = bars(&stage, light(PaletteColor::Secondary), 6.0);
  assert_eq!(rails.len(), 1);
  assert!((rails[0].width - 380.0).abs() < 0.5 && (rails[0].x - (x + 20.0)).abs() < 0.5);
  stage.pass_after(Duration::from_secs(2));
  let filled = bars(&stage, light(PaletteColor::Primary), 6.0);
  assert_eq!(filled.len(), 1);
  assert!((filled[0].width - 95.0).abs() < 0.6, "{}", filled[0].width);

  // How much is done at the start and what is left at the end, in the mono type of 11 in the muted color.
  assert_eq!(stage.text("32 of 128"), (light(PaletteColor::MutedForeground), 11.0));
  let (done, left) = (
    stage.text_quad("32 of 128").unwrap(),
    stage.text_quad("about 2 min left").unwrap(),
  );
  assert!((done.x - (x + 20.0)).abs() < 0.5);
  assert!((left.x + left.width - (x + width - 20.0)).abs() < 1.0);
  assert!(done.y > rails[0].y);

  // Details at the start of the footer, Pause and Cancel at its end, Cancel last, 20 from the bottom.
  let (details, pause, cancel) = (
    stage.text_quad("Details").unwrap(),
    stage.text_quad("Pause").unwrap(),
    stage.text_quad("Cancel").unwrap(),
  );
  assert!(details.x < pause.x && pause.x < cancel.x);
  assert!(
    (details.x - (x + 20.0 + 14.0 + 4.0)).abs() < 0.5,
    "after the arrow of 14 and 4"
  );
  assert!(cancel.y + cancel.height < y + height - 20.0);
  assert!(
    cancel.y > done.y + done.height + 12.0,
    "4 further down than the content's 12"
  );
}

#[test]
fn without_a_format_it_says_a_percentage_and_without_a_value_nothing() {
  let known = stage(|_| {
    ProgressDialog::new(Signal::new(true))
      .title("Exporting")
      .value(0.333)
      .into()
  });
  assert!(known.paints("33%"));
  assert!(!known.paints("Cancel"), "no cancel button without on_cancel");

  let unknown = stage(|_| ProgressDialog::new(Signal::new(true)).title("Connecting").into());
  assert!(!unknown.paints("0%"));
  assert_eq!(
    bars(&unknown, light(PaletteColor::Secondary), 6.0).len(),
    1,
    "the bar still runs"
  );
}

#[test]
fn nothing_dismisses_it_and_cancel_asks_the_app_to_stop() {
  let open = Signal::new(true);
  let (cancelled, cancel) = counter();
  let (paused, pause) = counter();
  let held = open.clone();
  let mut stage = stage(move |_| copying(held.clone(), Signal::new(false), cancel.clone(), pause.clone()).into());
  // As it opens the focus is on the cancel button: Space presses it, and it stays open.
  stage.key(" ");
  assert_eq!(count(&cancelled), 1);
  assert!(open.get());
  // Escape presses it too.
  stage.key("Escape");
  assert_eq!(count(&cancelled), 2);
  assert!(open.get());

  press_at(&mut stage, 5.0, 5.0);
  assert!(open.get(), "a press of the veil does not close it");
  press_text(&mut stage, "Pause");
  assert_eq!((count(&cancelled), count(&paused)), (2, 1));

  // It closes when the app says so.
  open.set(false);
  stage.pass();
  assert!(!stage.paints("Copying 128 items"));
}

#[test]
fn the_detail_opens_under_the_footer() {
  let detail = Signal::new(false);
  let held = detail.clone();
  let mut stage = stage(move |_| copying(Signal::new(true), held.clone(), || {}, || {}).into());
  assert!(!stage.paints("assets/icons/warning.svg"));
  press_text(&mut stage, "Details");
  assert!(detail.get());
  stage.pass_after(Duration::from_secs(1));
  let text = stage.text_quad("assets/icons/warning.svg").expect("the detail is open");
  let cancel = stage.text_quad("Cancel").unwrap();
  assert!(text.y > cancel.y + cancel.height, "under the footer");
}

#[test]
fn a_theme_restyles_its_parts() {
  let app = App::new();
  app.theme().patch_recipe::<ProgressDialog>(|rules| {
    rules
      .base(ProgressDialogPart::Panel, |s| s.width(360.0))
      .base(ProgressDialogPart::Title, |s| s.font_size(18.0))
  });
  let mut stage = stage_in(app, |_| {
    copying(Signal::new(true), Signal::new(false), || {}, || {}).into()
  });
  assert_eq!(stage.bounds("dialog").2, 360.0);
  assert_eq!(stage.text("Copying 128 items").1, 18.0);
}
