//! A dialog: its panel over the veil, its title, its description and its
//! buttons where the board has them, what closes it, that nothing under the
//! veil takes the pointer or the keyboard, the form whose body scrolls, and
//! a theme's rule and renderer.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App,
    ctx::Ctx,
    events::MouseButton,
    theme::{PaletteColor, ThemePalette},
  },
  components::{ChromeTitleBar, Column, Text, WindowChrome, WindowChromeMode},
  core::Signal,
  layout::quad::{Quad, QuadContent},
  node::{Element, color::Color},
  ui::{Button, ButtonVariant, Dialog, DialogPart, DialogRenderer, DialogSize, DialogView},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The scene of a test: a window of its own size, so that the layer of the
/// dialog has room, with `content` in it.
fn window(content: impl Into<Element>) -> Element {
  Column::new().size(400.0, 300.0).child(content).into()
}

/// A stage whose window is 400 by 300, the size of the page it lays out. A
/// dialog covers the window, and the stage's window is otherwise larger than
/// its page.
fn stage(scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Stage {
  stage_in(App::new(), scene)
}

fn stage_in(app: App, scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::in_app(app, scene);
  stage.tree.resize(400, 300);
  stage.pass();
  stage
}

/// A press of the left button at a point of the window.
fn press_at(stage: &mut Stage, x: f32, y: f32) {
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
}

/// Counts what is pressed.
fn counter() -> (Arc<AtomicUsize>, impl Fn() + Send + Sync + Clone + 'static) {
  let count = Arc::new(AtomicUsize::new(0));
  let add = count.clone();
  (count, move || {
    add.fetch_add(1, Ordering::SeqCst);
  })
}

/// The board's dialog: a question, a line more of it, and two buttons.
fn delete_workspace(open: Signal<bool>) -> Dialog {
  Dialog::new(open)
    .size(DialogSize::Sm)
    .id("dialog")
    .title("Delete workspace?")
    .description("It cannot be undone.")
    .footer(Button::new("Cancel").variant(ButtonVariant::Outline).id("cancel"))
    .footer(Button::new("Delete").variant(ButtonVariant::Destructive).id("delete"))
}

/// The filled rectangles of `color` as large as the window: the veil.
fn veils(stage: &Stage, color: Color) -> Vec<Quad> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color)
        && (quad.width, quad.height) == (400.0, 300.0)
    })
    .collect()
}

/// The lines across a panel `width` wide: filled rectangles 1 high in the border's color.
fn separators(stage: &Stage) -> Vec<Quad> {
  let mut lines: Vec<Quad> = stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Border))
        && quad.height == 1.0
        && quad.width > 100.0
    })
    .collect();
  lines.sort_by(|a, b| a.y.total_cmp(&b.y));
  lines
}

#[test]
fn an_open_dialog_is_its_panel_over_a_veil_as_the_board_draws_it() {
  let open = Signal::new(true);
  let mut stage = stage(move |_| window(delete_workspace(open.clone())));

  assert_eq!(
    veils(&stage, light(PaletteColor::Scrim)).len(),
    1,
    "the veil covers the window"
  );

  // A panel of 360 in the middle of the window.
  let (x, y, width, height) = stage.bounds("dialog");
  assert_eq!(width, 360.0);
  assert_eq!(x, 20.0);
  assert!((y + height / 2.0 - 150.0).abs() < 0.5, "in the middle: {y} {height}");
  assert_eq!(stage.fill_color("dialog"), Some(light(PaletteColor::Popover)));
  let fill = stage.fill("dialog").unwrap();
  assert!(
    fill.border_radius.is_some_and(|radius| radius.top_left == 12.0),
    "radius-xl"
  );
  let panel = stage.border("dialog").expect("the panel has an edge");
  let edge = panel.border.and_then(|border| border.top).unwrap();
  assert_eq!((edge.width, edge.color), (1.0, light(PaletteColor::Border)));
  assert!(
    stage
      .quads()
      .iter()
      .any(|quad| matches!(quad.content, QuadContent::BoxShadow(_))),
    "the panel casts a shadow"
  );

  // The title and the description, padded 24, 16/600 and 14 muted.
  assert_eq!(
    stage.text("Delete workspace?"),
    (light(PaletteColor::PopoverForeground), 16.0)
  );
  assert_eq!(
    stage.text("It cannot be undone."),
    (light(PaletteColor::MutedForeground), 14.0)
  );
  let title = stage.text_quad("Delete workspace?").unwrap();
  assert!((title.x - (x + 24.0)).abs() < 0.5);
  assert!((title.y - (y + 24.0)).abs() < 2.0, "24 under the top: {} {y}", title.y);

  // The buttons at the end, 8 apart, 24 from the panel's edges.
  let (cancel_x, _, cancel_width, _) = stage.bounds("cancel");
  let (delete_x, delete_y, delete_width, delete_height) = stage.bounds("delete");
  assert!((delete_x + delete_width - (x + width - 24.0)).abs() < 0.5);
  assert!((delete_x - (cancel_x + cancel_width) - 8.0).abs() < 0.5);
  assert!((delete_y + delete_height - (y + height - 24.0)).abs() < 0.5);
}

#[test]
fn a_closed_dialog_paints_nothing() {
  let open = Signal::new(false);
  let stage = stage(move |_| window(delete_workspace(open.clone())));
  assert!(!stage.paints("Delete workspace?"));
  assert!(veils(&stage, light(PaletteColor::Scrim)).is_empty());
}

#[test]
fn the_sizes_and_a_width_of_its_own() {
  let sizes: [(fn() -> Dialog, f32); 3] = [
    (|| Dialog::new(Signal::new(true)).width(300.0), 300.0),
    // As wide as the window less the veil's margin of 16 on each side.
    (|| Dialog::new(Signal::new(true)).size(DialogSize::Full), 368.0),
    // Never wider than the window.
    (|| Dialog::new(Signal::new(true)).size(DialogSize::Lg), 368.0),
  ];
  for (make, expected) in sizes {
    let mut stage = stage(move |_| window(make().id("dialog").title("Rename")));
    assert_eq!(stage.bounds("dialog").2, expected);
  }
}

#[test]
fn a_press_on_the_veil_closes_it_and_a_press_on_the_panel_does_not() {
  let open = Signal::new(true);
  let held = open.clone();
  let mut stage = stage(move |_| window(delete_workspace(held.clone())));
  stage.click("dialog");
  assert!(open.get(), "a press on the panel goes no further");
  press_at(&mut stage, 5.0, 5.0);
  assert!(!open.get(), "a press on the veil closes it");
  assert!(!stage.paints("Delete workspace?"));
}

#[test]
fn a_dialog_that_has_to_be_answered_stays_on_a_press_of_the_veil_and_escape_closes_it() {
  let open = Signal::new(true);
  let held = open.clone();
  let mut stage = stage(move |_| window(delete_workspace(held.clone()).dismiss_on_veil(false)));
  press_at(&mut stage, 5.0, 5.0);
  assert!(open.get());
  stage.key("Escape");
  assert!(!open.get(), "Escape closes it");
}

#[test]
fn nothing_under_the_veil_takes_the_pointer_or_the_keyboard() {
  let open = Signal::new(true);
  let held = open.clone();
  let (behind, press_behind) = counter();
  let (cancel, press_cancel) = counter();
  let (delete, press_delete) = counter();
  let mut stage = stage(move |_| {
    let (press_behind, press_cancel, press_delete) = (press_behind.clone(), press_cancel.clone(), press_delete.clone());
    Column::new()
      .size(400.0, 300.0)
      .child(Button::new("Behind").id("behind").on_click(move |_| press_behind()))
      .child(
        Dialog::new(held.clone())
          .dismiss_on_veil(false)
          .title("Delete workspace?")
          .footer(Button::new("Cancel").on_click(move |_| press_cancel()))
          .footer(Button::new("Delete").on_click(move |_| press_delete())),
      )
      .into()
  });
  // The focus went to the dialog's first button as it opened. Tab goes round its two buttons and never
  // reaches the one behind it: the second Tab is on the first button again, and Enter presses it.
  for _ in 0..2 {
    stage.key("Tab");
  }
  stage.key("Enter");
  assert_eq!(behind.load(Ordering::SeqCst), 0);
  assert_eq!((cancel.load(Ordering::SeqCst), delete.load(Ordering::SeqCst)), (1, 0));

  // A press where the button behind it is lands on the veil.
  stage.click("behind");
  assert_eq!(behind.load(Ordering::SeqCst), 0, "the veil takes the press");
  assert!(open.get());
}

#[test]
fn the_close_button_closes_it() {
  let open = Signal::new(true);
  let held = open.clone();
  let mut stage = stage(move |_| window(delete_workspace(held.clone()).close_button(true)));
  // A button of 28 at the end of the header, 24 from the panel's top and its end.
  let (x, y, width, _) = stage.bounds("dialog");
  press_at(&mut stage, x + width - 24.0 - 14.0, y + 24.0 + 14.0);
  assert!(!open.get());
}

/// The board's dialog whose body scrolls: terms of use, many paragraphs.
fn terms(open: Signal<bool>) -> Dialog {
  Dialog::new(open)
    .size(DialogSize::Sm)
    .scrollable(true)
    .close_button(true)
    .id("dialog")
    .title("Terms of use")
    .children((1..=20).map(|n| Text::new(&format!("Paragraph {n} of the terms."))))
    .footer(Button::new("Decline").variant(ButtonVariant::Outline).id("decline"))
    .footer(Button::new("Accept").variant(ButtonVariant::Primary).id("accept"))
}

#[test]
fn the_body_of_a_scrollable_dialog_scrolls_between_a_header_and_a_footer_that_stay() {
  let open = Signal::new(true);
  let held = open.clone();
  let mut stage = stage(move |_| window(terms(held.clone()).style(DialogPart::Panel, |s| s.height(250.0))));
  let (x, y, width, height) = stage.bounds("dialog");
  assert_eq!((width, height), (360.0, 250.0));

  // A header of 56 with its title 20 from the edge and the close button 12 from it, a line under it.
  let title = stage.text_quad("Terms of use").unwrap();
  assert!((title.x - (x + 20.0)).abs() < 0.5);
  assert!((title.y + title.height / 2.0 - (y + 28.0)).abs() < 1.5);
  let lines = separators(&stage);
  assert_eq!(lines.len(), 2, "a line under the header and one over the footer");
  assert!((lines[0].y - (y + 56.0)).abs() < 0.5);
  // A footer of 60 at the bottom, its buttons 20 from the end, a line over it.
  assert!((lines[1].y - (y + height - 61.0)).abs() < 0.5);
  let (accept_x, accept_y, accept_width, accept_height) = stage.bounds("accept");
  assert!((accept_x + accept_width - (x + width - 20.0)).abs() < 0.5);
  assert!((accept_y + accept_height / 2.0 - (y + height - 30.0)).abs() < 0.5);

  // The body starts 16 under the line, 20 from the edge.
  let first = stage.text_quad("Paragraph 1 of the terms.").unwrap();
  assert!((first.x - (x + 20.0)).abs() < 0.5);
  assert!((first.y - (y + 57.0 + 16.0)).abs() < 2.0);

  // The close button 12 from the end, in the middle of the header.
  press_at(&mut stage, x + width - 12.0 - 14.0, y + 28.0);
  assert!(!open.get());
}

#[test]
fn a_dialog_is_never_higher_than_the_window_and_its_body_scrolls() {
  for scrollable in [true, false] {
    let open = Signal::new(true);
    let mut stage = stage(move |_| window(terms(open.clone()).scrollable(scrollable)));
    let (_, y, _, height) = stage.bounds("dialog");
    // The window less the veil's margin of 16 above and below.
    assert!(height <= 268.5, "scrollable {scrollable}: {height}");
    assert!(y >= 15.5, "scrollable {scrollable}: {y}");
    let (_, accept_y, _, accept_height) = stage.bounds("accept");
    assert!(
      accept_y + accept_height <= y + height,
      "scrollable {scrollable}: the footer stays in the panel"
    );
  }
}

#[test]
fn a_scrollable_dialog_with_little_in_it_is_as_high_as_what_is_in_it() {
  let open = Signal::new(true);
  let mut stage = stage(move |_| {
    window(
      Dialog::new(open.clone())
        .size(DialogSize::Sm)
        .scrollable(true)
        .id("dialog")
        .title("Terms of use")
        .child(Text::new("One paragraph."))
        .footer(Button::new("Accept").id("accept")),
    )
  });
  let (_, y, _, height) = stage.bounds("dialog");
  // A header of 56, a footer of 60, two lines, and a body of one line with 16 over and under it: not the
  // whole of the room the window has, which is 268.
  assert!(height < 180.0, "{height}");
  // In the middle of the window.
  assert!((y + height / 2.0 - 150.0).abs() < 0.6, "{y} {height}");
  let (_, accept_y, _, accept_height) = stage.bounds("accept");
  assert!((accept_y + accept_height / 2.0 - (y + height - 30.0)).abs() < 0.6);
}

#[test]
fn a_dialog_in_a_window_with_its_own_title_bar_is_under_the_bar() {
  let open = Signal::new(true);
  let mut stage = stage(move |ctx| {
    WindowChrome::new()
      .mode(WindowChromeMode::AlwaysCustom)
      .title_bar(ChromeTitleBar::new().height(40.0))
      .content(window(terms(open.clone())))
      .mount(ctx)
  });
  // The dialog's layer starts under the bar once the chrome has said how high it is.
  stage.pass();
  let (_, y, _, height) = stage.bounds("dialog");
  // Higher than the window has room for, it takes what is under the bar less the veil's 16 above and below,
  // and none of it is under the bar.
  assert!((y - 56.0).abs() < 0.6, "{y}");
  assert!((height - 228.0).abs() < 0.6, "{height}");
  let (_, accept_y, _, accept_height) = stage.bounds("accept");
  assert!(accept_y + accept_height <= y + height);
}

#[test]
fn the_focus_goes_into_the_dialog_as_it_opens_and_back_to_its_opener_as_it_closes() {
  let open = Signal::new(false);
  let held = open.clone();
  let (cancel, press_cancel) = counter();
  let mut stage = stage(move |_| {
    let (show, close, press_cancel) = (held.clone(), held.clone(), press_cancel.clone());
    Column::new()
      .size(400.0, 300.0)
      .child(Button::new("Delete workspace…").on_click(move |_| show.set(true)))
      .child(
        Dialog::new(held.clone())
          .title("Delete workspace?")
          .footer(Button::new("Cancel").on_click(move |_| {
            press_cancel();
            close.set(false);
          }))
          .footer(Button::new("Delete")),
      )
      .into()
  });
  // Tab reaches the button that opens the dialog, and Enter opens it.
  stage.key("Tab");
  stage.key("Enter");
  assert!(open.get());
  // The focus is on the dialog's first button: Enter presses it, and it closes the dialog.
  stage.key("Enter");
  assert_eq!(cancel.load(Ordering::SeqCst), 1);
  assert!(!open.get());
  // The focus is back on the button that opened it: Enter opens it again.
  stage.key("Enter");
  assert!(open.get());
}

#[test]
fn a_theme_restyles_the_panel_and_gives_the_dialog_another_anatomy() {
  let app = App::new();
  app
    .theme()
    .patch_recipe::<Dialog>(|rules| rules.base(DialogPart::Panel, |s| s.background(PaletteColor::Muted)));
  let open = Signal::new(true);
  let held = open.clone();
  let mut stage = stage_in(app, move |_| window(delete_workspace(held.clone())));
  assert_eq!(stage.fill_color("dialog"), Some(light(PaletteColor::Muted)));

  // A dialog that is its title alone, in its panel, over the veil: other nodes, the same behavior.
  let app = App::new();
  let renderer: DialogRenderer = Arc::new(|view: DialogView| {
    let part = view.part(DialogPart::Title);
    let title = format!("{} (themed)", view.title.as_deref().unwrap_or(""));
    let panel = view.panel().child(part.text(&title));
    view.modal(panel)
  });
  app.theme().set_renderer::<Dialog>(renderer);
  let held = open.clone();
  let mut stage = stage_in(app, move |_| window(delete_workspace(held.clone())));
  assert!(stage.paints("Delete workspace? (themed)"));
  assert!(!stage.paints("Cancel"));
  assert_eq!(
    stage.fill_color("dialog"),
    Some(light(PaletteColor::Popover)),
    "the recipe still styles it"
  );
  stage.key("Escape");
  assert!(!open.get());
}

#[test]
fn an_inline_dialog_is_its_panel_where_it_is_declared() {
  let stage = stage(|_| window(delete_workspace(Signal::new(false)).inline(true)));
  assert!(stage.paints("Delete workspace?"));
  assert!(veils(&stage, light(PaletteColor::Scrim)).is_empty());
}
