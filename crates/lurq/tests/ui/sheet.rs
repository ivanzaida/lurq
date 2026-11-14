//! A sheet: its panel at each edge of the window as the board draws it, its
//! sizes, its header, body and footer, what closes it, that nothing under the
//! veil takes the pointer or the keyboard, its slide and the slide held back
//! with motion reduced, a sheet under a window's own title bar, the bottom
//! sheet a finger drags, and a theme's rule and renderer.

use std::{
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
  time::{Duration, Instant},
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
  ui::{Button, ButtonVariant, Sheet, SheetPart, SheetRenderer, SheetSide, SheetSize, SheetView},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The scene of a test: a window of its own size, so that the layer of the
/// sheet has room, with `content` in it.
fn window(content: impl Into<Element>) -> Element {
  Column::new().size(400.0, 300.0).child(content).into()
}

/// A stage whose window is 400 by 300, the size of the page it lays out.
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

/// The board's sheet: a profile to edit, with a field, and two buttons.
fn edit_profile(open: Signal<bool>) -> Sheet {
  Sheet::new(open)
    .id("panel")
    .title("Edit profile")
    .description("Changes save when you close the panel.")
    .close_button(true)
    .child(Text::new("Display name"))
    .footer(Button::new("Close").variant(ButtonVariant::Outline).id("close"))
    .footer(Button::new("Save").id("save"))
}

/// The board's sheet at the bottom: a goal to set, and two buttons.
fn move_goal(open: Signal<bool>) -> Sheet {
  Sheet::new(open)
    .side(SheetSide::Bottom)
    .id("panel")
    .title("Move goal")
    .description("Set your daily activity goal.")
    .footer(Button::new("Cancel").variant(ButtonVariant::Outline).id("cancel"))
    .footer(Button::new("Submit").id("submit"))
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

/// The lines across the panel: filled rectangles 1 high in the border's color.
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

/// Which sides of the panel's box have a line of 1 in the border's color: top, right, bottom, left.
fn edges(stage: &mut Stage, id: &str) -> [bool; 4] {
  let quad = stage.border(id).expect("the panel has an edge");
  let border = quad.border.unwrap();
  let line = |side: Option<lurq::node::border::ResolvedBorder>| {
    side.is_some_and(|side| side.width == 1.0 && side.color == light(PaletteColor::Border))
  };
  [
    line(border.top),
    line(border.right),
    line(border.bottom),
    line(border.left),
  ]
}

#[test]
fn an_open_sheet_at_the_end_is_its_panel_at_the_windows_edge_as_the_board_draws_it() {
  let open = Signal::new(true);
  let mut stage = stage(move |_| window(edit_profile(open.clone())));

  assert_eq!(
    veils(&stage, light(PaletteColor::Scrim)).len(),
    1,
    "the veil covers the window"
  );

  // A panel of 320 at the window's end, as high as the window.
  let (x, y, width, height) = stage.bounds("panel");
  assert_eq!((x, y, width, height), (80.0, 0.0, 320.0, 300.0));
  assert_eq!(stage.fill_color("panel"), Some(light(PaletteColor::Background)));
  assert_eq!(
    edges(&mut stage, "panel"),
    [false, false, false, true],
    "a line along the edge it shows the page"
  );
  assert!(
    stage
      .quads()
      .iter()
      .any(|quad| matches!(quad.content, QuadContent::BoxShadow(_))),
    "the panel casts a shadow"
  );

  // The title over the description, 16/600 and 13 muted, 20 from the panel's top and its start.
  assert_eq!(stage.text("Edit profile"), (light(PaletteColor::Foreground), 16.0));
  assert_eq!(
    stage.text("Changes save when you close the panel."),
    (light(PaletteColor::MutedForeground), 13.0)
  );
  let title = stage.text_quad("Edit profile").unwrap();
  assert!((title.x - (x + 20.0)).abs() < 0.5, "{} {x}", title.x);
  assert!((title.y - (y + 20.0)).abs() < 2.0, "{} {y}", title.y);

  // The body 20 under the header, 20 from the edge.
  let field = stage.text_quad("Display name").unwrap();
  assert!((field.x - (x + 20.0)).abs() < 0.5);

  // The buttons at the panel's bottom, at its end, 8 apart, under a line.
  let (close_x, _, close_width, _) = stage.bounds("close");
  let (save_x, save_y, save_width, save_height) = stage.bounds("save");
  assert!((save_x + save_width - (x + width - 20.0)).abs() < 0.5);
  assert!((save_x - (close_x + close_width) - 8.0).abs() < 0.5);
  assert!((save_y + save_height - (y + height - 16.0)).abs() < 0.5);
  let lines = separators(&stage);
  assert_eq!(lines.len(), 1, "a line over the footer");
  assert!((lines[0].y - (save_y - 16.0 - 1.0)).abs() < 0.5);
}

#[test]
fn a_sheet_at_the_start_stands_at_the_windows_start_with_its_line_toward_the_page() {
  let open = Signal::new(true);
  let mut stage = stage(move |_| window(edit_profile(open.clone()).side(SheetSide::Start)));
  assert_eq!(stage.bounds("panel"), (0.0, 0.0, 320.0, 300.0));
  assert_eq!(edges(&mut stage, "panel"), [false, true, false, false]);
}

#[test]
fn a_sheet_at_the_top_is_as_wide_as_the_window_and_as_high_as_what_it_holds() {
  let open = Signal::new(true);
  let mut stage = stage(move |_| window(edit_profile(open.clone()).side(SheetSide::Top)));
  let (x, y, width, height) = stage.bounds("panel");
  assert_eq!((x, y, width), (0.0, 0.0, 400.0));
  assert!(
    height < 250.0,
    "as high as its header, its body and its footer: {height}"
  );
  assert_eq!(edges(&mut stage, "panel"), [false, false, true, false]);
  // The footer at the panel's bottom.
  let (_, save_y, _, save_height) = stage.bounds("save");
  assert!((save_y + save_height - (y + height - 16.0)).abs() < 0.5);
}

#[test]
fn a_sheet_at_the_bottom_is_the_boards_drawer() {
  let open = Signal::new(true);
  let mut stage = stage(move |_| window(move_goal(open.clone())));
  let (x, y, width, height) = stage.bounds("panel");
  assert_eq!((x, width), (0.0, 400.0));
  assert!((y + height - 300.0).abs() < 0.5, "at the window's bottom: {y} {height}");
  assert!(height < 250.0, "as high as what it holds: {height}");
  assert_eq!(edges(&mut stage, "panel"), [true, true, true, true], "a line all round");
  let fill = stage.fill("panel").unwrap();
  let radius = fill.border_radius.expect("round at its top");
  assert_eq!(
    (radius.top_left, radius.top_right, radius.bottom_left),
    (12.0, 12.0, 0.0)
  );

  // The handle, 40 by 4 in the muted color, in the middle, 8 under the top edge.
  let handle = stage
    .quads()
    .into_iter()
    .find(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Muted))
        && (quad.width, quad.height) == (40.0, 4.0)
    })
    .expect("the grabber's handle");
  assert!((handle.x - 180.0).abs() < 0.5);
  assert!((handle.y - (y + 8.0)).abs() < 0.5);

  // The title in the middle, 20 under the handle.
  let title = stage.text_quad("Move goal").unwrap();
  assert!((title.x + title.width / 2.0 - 200.0).abs() < 1.0, "in the middle");
  assert!((title.y - (handle.y + 4.0 + 20.0)).abs() < 2.0);

  // The buttons as wide as the panel less 20 on each side, the main one on top, 8 apart, 20 from the bottom.
  let (submit_x, submit_y, submit_width, submit_height) = stage.bounds("submit");
  let (cancel_x, cancel_y, cancel_width, cancel_height) = stage.bounds("cancel");
  assert_eq!((submit_x, submit_width), (20.0, 360.0));
  assert_eq!((cancel_x, cancel_width), (20.0, 360.0));
  assert!((cancel_y - (submit_y + submit_height + 8.0)).abs() < 0.5);
  assert!((cancel_y + cancel_height - (y + height - 20.0)).abs() < 0.5);
  assert!(separators(&stage).is_empty(), "no line over its buttons");
}

#[test]
fn a_closed_sheet_paints_nothing() {
  let open = Signal::new(false);
  let stage = stage(move |_| window(edit_profile(open.clone())));
  assert!(!stage.paints("Edit profile"));
  assert!(veils(&stage, light(PaletteColor::Scrim)).is_empty());
}

#[test]
fn the_sizes_are_its_width_at_a_side_and_its_height_at_the_top_or_the_bottom() {
  let sizes: [(SheetSide, Option<SheetSize>, f32, f32); 8] = [
    (SheetSide::End, None, 320.0, 300.0),
    (SheetSide::End, Some(SheetSize::Sm), 320.0, 300.0),
    // Never wider than the window.
    (SheetSide::End, Some(SheetSize::Md), 400.0, 300.0),
    (SheetSide::Start, Some(SheetSize::Full), 400.0, 300.0),
    (SheetSide::Bottom, Some(SheetSize::Sm), 400.0, 240.0),
    // Never higher than the window.
    (SheetSide::Bottom, Some(SheetSize::Md), 400.0, 300.0),
    (SheetSide::Top, Some(SheetSize::Sm), 400.0, 240.0),
    (SheetSide::Top, Some(SheetSize::Full), 400.0, 300.0),
  ];
  for (side, size, width, height) in sizes {
    let mut stage = stage(move |_| {
      let mut sheet = Sheet::new(Signal::new(true)).side(side).id("panel").title("Filters");
      if let Some(size) = size {
        sheet = sheet.size(size);
      }
      window(sheet)
    });
    let (_, _, shown_width, shown_height) = stage.bounds("panel");
    assert_eq!((shown_width, shown_height), (width, height), "{side:?} {size:?}");
  }
}

#[test]
fn a_press_on_the_veil_closes_it_and_a_press_on_the_panel_does_not() {
  let open = Signal::new(true);
  let held = open.clone();
  let mut stage = stage(move |_| window(edit_profile(held.clone())));
  stage.click("panel");
  assert!(open.get(), "a press on the panel goes no further");
  press_at(&mut stage, 20.0, 150.0);
  assert!(!open.get(), "a press on the veil closes it");
  stage.pass();
  assert!(!stage.paints("Edit profile"), "gone once it has slid out");
}

#[test]
fn a_sheet_that_has_to_be_answered_stays_and_escape_closes_what_it_is_told_to() {
  let open = Signal::new(true);
  let held = open.clone();
  let mut stage = stage(move |_| window(edit_profile(held.clone()).dismiss_on_veil(false)));
  press_at(&mut stage, 20.0, 150.0);
  assert!(open.get());
  stage.key("Escape");
  assert!(!open.get(), "Escape closes it");

  let open = Signal::new(true);
  let held = open.clone();
  let mut kept = self::stage(move |_| window(edit_profile(held.clone()).dismiss_on_escape(false)));
  kept.key("Escape");
  assert!(open.get(), "Escape is not for this one");
}

#[test]
fn the_close_button_closes_it() {
  let open = Signal::new(true);
  let held = open.clone();
  let mut stage = stage(move |_| window(edit_profile(held.clone())));
  // The icon of 16 at the header's end, 20 from the panel's top and its end, in a button of 24.
  let (x, y, width, _) = stage.bounds("panel");
  press_at(&mut stage, x + width - 20.0 - 8.0, y + 20.0 + 8.0);
  assert!(!open.get());
}

#[test]
fn nothing_under_the_veil_takes_the_pointer_or_the_keyboard() {
  let open = Signal::new(true);
  let held = open.clone();
  let (behind, press_behind) = counter();
  let (cancel, press_cancel) = counter();
  let (save, press_save) = counter();
  let mut stage = stage(move |_| {
    let (press_behind, press_cancel, press_save) = (press_behind.clone(), press_cancel.clone(), press_save.clone());
    Column::new()
      .size(400.0, 300.0)
      .child(Button::new("Behind").id("behind").on_click(move |_| press_behind()))
      .child(
        Sheet::new(held.clone())
          .side(SheetSide::Start)
          .dismiss_on_veil(false)
          .title("Filters")
          .footer(Button::new("Cancel").on_click(move |_| press_cancel()))
          .footer(Button::new("Save").on_click(move |_| press_save())),
      )
      .into()
  });
  // The focus went to the sheet's first button as it opened. Tab goes round its two buttons and never reaches
  // the one behind it: the second Tab is on the first button again, and Enter presses it.
  for _ in 0..2 {
    stage.key("Tab");
  }
  stage.key("Enter");
  assert_eq!(behind.load(Ordering::SeqCst), 0);
  assert_eq!((cancel.load(Ordering::SeqCst), save.load(Ordering::SeqCst)), (1, 0));

  // A press where the button behind it is lands on the panel, which stands over it.
  stage.click("behind");
  assert_eq!(behind.load(Ordering::SeqCst), 0, "the sheet takes the press");
  assert!(open.get());
}

#[test]
fn the_focus_goes_into_the_sheet_as_it_opens_and_back_to_its_opener_as_it_closes() {
  let open = Signal::new(false);
  let held = open.clone();
  let (cancel, press_cancel) = counter();
  let mut stage = stage(move |_| {
    let (show, close, press_cancel) = (held.clone(), held.clone(), press_cancel.clone());
    Column::new()
      .size(400.0, 300.0)
      .child(Button::new("Edit profile…").on_click(move |_| show.set(true)))
      .child(
        Sheet::new(held.clone())
          .title("Edit profile")
          .footer(Button::new("Cancel").on_click(move |_| {
            press_cancel();
            close.set(false);
          }))
          .footer(Button::new("Save")),
      )
      .into()
  });
  stage.key("Tab");
  stage.key("Enter");
  assert!(open.get());
  // The focus is on the sheet's first button: Enter presses it, and it closes the sheet.
  stage.key("Enter");
  assert_eq!(cancel.load(Ordering::SeqCst), 1);
  assert!(!open.get());
  // The focus is back on the button that opened it: Enter opens it again.
  stage.key("Enter");
  assert!(open.get());
}

#[test]
fn the_body_scrolls_between_a_header_and_a_footer_that_stay() {
  let open = Signal::new(true);
  let mut stage = stage(move |_| {
    window(edit_profile(open.clone()).children((1..=30).map(|n| Text::new(&format!("Paragraph {n} of the profile.")))))
  });
  let (_, y, _, height) = stage.bounds("panel");
  assert_eq!((y, height), (0.0, 300.0), "no higher than the window");
  let title = stage.text_quad("Edit profile").unwrap();
  assert!((title.y - 20.0).abs() < 2.0, "the header stays at the top");
  let (_, save_y, _, save_height) = stage.bounds("save");
  assert!(
    (save_y + save_height - 284.0).abs() < 0.5,
    "the footer stays at the bottom"
  );
  let last = stage.text_quad("Paragraph 30 of the profile.").map(|quad| quad.y);
  assert!(
    last.is_none_or(|y| y > 284.0),
    "the last is out of view under the footer: {last:?}"
  );
}

#[test]
fn a_sheet_with_little_in_it_at_the_bottom_is_as_high_as_what_it_holds() {
  let open = Signal::new(true);
  let mut stage = stage(move |_| window(move_goal(open.clone()).child(Text::new("Steps"))));
  let (_, y, _, height) = stage.bounds("panel");
  // The grabber of 12, the header, the body and the footer of two buttons with 20 between them and at the
  // bottom: less than the window.
  assert!(height < 260.0, "{height}");
  assert!((y + height - 300.0).abs() < 0.5);
}

#[test]
fn a_sheet_in_a_window_with_its_own_title_bar_is_under_the_bar() {
  let open = Signal::new(true);
  let mut stage = stage(move |ctx| {
    WindowChrome::new()
      .mode(WindowChromeMode::AlwaysCustom)
      .title_bar(ChromeTitleBar::new().height(40.0))
      .content(window(edit_profile(open.clone())))
      .mount(ctx)
  });
  stage.pass();
  let (_, y, _, height) = stage.bounds("panel");
  assert!((y - 40.0).abs() < 0.6, "{y}");
  assert!((height - 260.0).abs() < 0.6, "{height}");
}

#[test]
fn it_slides_in_from_its_edge_and_fades_in_where_motion_is_reduced() {
  for reduced in [false, true] {
    let app = App::new();
    app.theme().set_reduced_motion(Some(reduced));
    let open = Signal::new(false);
    let held = open.clone();
    let mut stage = stage_in(app, move |_| window(edit_profile(held.clone())));
    open.set(true);
    stage.pass_after(Duration::ZERO);
    stage.pass_after(Duration::from_millis(60));
    let (x, _, width, _) = stage.bounds("panel");
    let title = stage.text_quad("Edit profile").unwrap();
    if reduced {
      assert!((title.x - (x + 20.0)).abs() < 0.5, "it stands where it is to be");
      assert!((title.x - 100.0).abs() < 0.5, "{}", title.x);
    } else {
      assert!(title.x > 100.5, "on its way in from the end: {}", title.x);
      assert!(title.x < 400.0 + 20.0, "{}", title.x);
    }
    assert_eq!(width, 320.0);
    // At rest once its time is up.
    stage.pass();
    let title = stage.text_quad("Edit profile").unwrap();
    assert!((title.x - 100.0).abs() < 0.5, "reduced {reduced}: {}", title.x);
  }
}

/// Lets the engine's sheet come up or go down: its timers fire, and its move runs to its end.
fn settle(stage: &mut Stage) {
  for later in [1, 2] {
    stage.pass();
    stage.tree.tick_timers_at(Instant::now() + Duration::from_secs(later));
  }
  stage.pass();
  stage.pass();
}

#[test]
fn a_mounted_sheet_at_the_bottom_is_the_engines_sheet_in_the_boards_look() {
  let open = Signal::new(true);
  let held = open.clone();
  let mut stage = stage(move |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .child(move_goal(held.clone()).id("goal").mount(ctx))
      .into()
  });
  settle(&mut stage);
  // The engine's panel, which reaches under the window's edge, in the recipe's look: the background's color,
  // round at its top. It has come up as far as what it holds is high, at the window's bottom.
  let (_, _, _, height) = stage.bounds("goal");
  let panel = stage
    .quads()
    .into_iter()
    .find(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Background))
        && quad.width == 400.0
        && quad.height > 300.0
    })
    .expect("the sheet's panel in the background's color");
  assert!(
    (panel.y - (300.0 - height)).abs() < 1.0,
    "up by its content's height: {} {height}",
    panel.y
  );
  assert_eq!(panel.border_radius.unwrap().top_left, 12.0);
  // What it holds is the sheet's: the handle 8 under the panel's top, the title, the buttons.
  let handle = stage
    .quads()
    .into_iter()
    .find(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Muted))
        && (quad.width, quad.height) == (40.0, 4.0)
    })
    .expect("the grabber's handle");
  assert!((handle.y - (panel.y + 8.0)).abs() < 0.5);
  let title = stage.text_quad("Move goal").expect("the title, in the window");
  assert!(title.y > panel.y && title.y < 300.0);
  let (submit_x, _, submit_width, _) = stage.bounds("submit");
  assert_eq!((submit_x, submit_width), (20.0, 360.0));
  // A press beside it puts it away, and so does Escape.
  press_at(&mut stage, 200.0, 5.0);
  assert!(!open.get());
  settle(&mut stage);
  open.set(true);
  settle(&mut stage);
  stage.key("Escape");
  assert!(!open.get());
}

#[test]
fn a_theme_restyles_the_panel_and_gives_the_sheet_another_anatomy() {
  let app = App::new();
  app
    .theme()
    .patch_recipe::<Sheet>(|rules| rules.base(SheetPart::Panel, |s| s.background(PaletteColor::Muted)));
  let open = Signal::new(true);
  let held = open.clone();
  let mut stage = stage_in(app, move |_| window(edit_profile(held.clone())));
  assert_eq!(stage.fill_color("panel"), Some(light(PaletteColor::Muted)));

  // A sheet that is its title alone, in its panel, over the veil: other nodes, the same behavior.
  let app = App::new();
  let renderer: SheetRenderer = Arc::new(|view: SheetView| {
    let part = view.part(SheetPart::Title);
    let title = format!("{} (themed)", view.title.as_deref().unwrap_or(""));
    let panel = view.panel().child(part.text(&title));
    view.modal(panel)
  });
  app.theme().set_renderer::<Sheet>(renderer);
  let held = open.clone();
  let mut stage = stage_in(app, move |_| window(edit_profile(held.clone())));
  assert!(stage.paints("Edit profile (themed)"));
  assert!(!stage.paints("Save"));
  assert_eq!(
    stage.fill_color("panel"),
    Some(light(PaletteColor::Background)),
    "the recipe still styles it"
  );
  stage.key("Escape");
  assert!(!open.get());
}

#[test]
fn an_inline_sheet_is_its_panel_where_it_is_declared() {
  let stage = stage(|_| {
    window(
      edit_profile(Signal::new(false))
        .inline(true)
        .style(SheetPart::Panel, |s| s.height(280.0)),
    )
  });
  assert!(stage.paints("Edit profile"));
  assert!(veils(&stage, light(PaletteColor::Scrim)).is_empty());
}
