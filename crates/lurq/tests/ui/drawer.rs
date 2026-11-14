//! A drawer: the room it takes beside the page at each side of a layout as the
//! board draws it, that it opens and closes with its width, that a closed one
//! has nothing Tab reaches, one that lies over the page, and a theme's rule.

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
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row, Text},
  core::Signal,
  layout::quad::QuadContent,
  node::{Element, color::Color},
  ui::{Button, Drawer, DrawerPart, SheetSide},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// A stage whose window is 600 by 300.
fn stage(scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Stage {
  stage_in(App::new(), scene)
}

fn stage_in(app: App, scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::in_app(app, scene);
  stage.tree.resize(600, 300);
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

/// The board's navigation drawer: a few places to go.
fn navigation(open: Signal<bool>) -> Drawer {
  Drawer::new(open)
    .id("drawer")
    .label("Navigation")
    .child(Text::new("Inbox"))
    .child(Text::new("Starred"))
}

/// The page beside the drawer, taking what room the drawer leaves.
fn page() -> Column {
  Column::new().id("page").flex(1.0).child(Text::new("The page"))
}

/// A layout of 600 by 300: `drawer` beside the page, in a row, or over and under it, in a column.
fn layout(drawer: Drawer, side: SheetSide) -> Element {
  match side {
    SheetSide::Start => Row::new().size(600.0, 300.0).child(drawer).child(page()).into(),
    SheetSide::End => Row::new().size(600.0, 300.0).child(page()).child(drawer).into(),
    SheetSide::Top => Column::new().size(600.0, 300.0).child(drawer).child(page()).into(),
    SheetSide::Bottom => Column::new().size(600.0, 300.0).child(page()).child(drawer).into(),
  }
}

/// Which sides of the drawer's panel have a line of 1 in the border's color: top, right, bottom, left.
fn edges(stage: &mut Stage) -> [bool; 4] {
  let quad = stage.border("drawer").expect("the panel has an edge");
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
fn an_open_drawer_at_the_start_takes_300_beside_the_page_as_the_board_draws_it() {
  let open = Signal::new(true);
  let mut stage = stage(move |_| layout(navigation(open.clone()), SheetSide::Start));
  assert_eq!(stage.bounds("drawer"), (0.0, 0.0, 300.0, 300.0));
  assert_eq!(stage.fill_color("drawer"), Some(light(PaletteColor::Background)));
  assert_eq!(edges(&mut stage), [false, true, false, false], "a line toward the page");
  let (page_x, _, page_width, _) = stage.bounds("page");
  assert_eq!((page_x, page_width), (300.0, 300.0), "the page is pushed aside");
  assert!(stage.paints("Inbox"));
  assert_eq!(stage.text("Inbox").0, light(PaletteColor::Foreground));
}

#[test]
fn a_closed_drawer_takes_no_room_and_paints_nothing() {
  let open = Signal::new(false);
  let mut stage = stage(move |_| layout(navigation(open.clone()), SheetSide::Start));
  let (page_x, _, page_width, _) = stage.bounds("page");
  assert_eq!((page_x, page_width), (0.0, 600.0));
  assert!(!stage.paints("Inbox"));
}

#[test]
fn it_opens_and_closes_with_its_width() {
  let open = Signal::new(false);
  let held = open.clone();
  let mut stage = stage(move |_| layout(navigation(held.clone()), SheetSide::Start));
  open.set(true);
  stage.pass_after(Duration::ZERO);
  stage.pass_after(Duration::from_millis(120));
  let (page_x, ..) = stage.bounds("page");
  assert!(page_x > 0.5 && page_x < 299.5, "on its way open: {page_x}");
  stage.pass();
  assert_eq!(stage.bounds("page").0, 300.0);

  open.set(false);
  stage.pass_after(Duration::ZERO);
  stage.pass_after(Duration::from_millis(120));
  let (page_x, ..) = stage.bounds("page");
  assert!(page_x > 0.5 && page_x < 299.5, "on its way closed: {page_x}");
  stage.pass();
  assert_eq!(stage.bounds("page").0, 0.0);
  assert!(!stage.paints("Inbox"), "what it held is gone");
}

#[test]
fn a_drawer_at_the_end_has_its_line_on_its_start() {
  let open = Signal::new(true);
  let mut stage = stage(move |_| layout(navigation(open.clone()).side(SheetSide::End), SheetSide::End));
  assert_eq!(stage.bounds("drawer"), (300.0, 0.0, 300.0, 300.0));
  assert_eq!(edges(&mut stage), [false, false, false, true]);
  assert_eq!(stage.bounds("page").2, 300.0);
}

#[test]
fn a_drawer_at_the_top_or_the_bottom_opens_with_the_height_of_what_it_holds() {
  for side in [SheetSide::Top, SheetSide::Bottom] {
    let open = Signal::new(true);
    let mut stage = stage(move |_| layout(navigation(open.clone()).side(side), side));
    let (x, y, width, height) = stage.bounds("drawer");
    assert_eq!((x, width), (0.0, 600.0), "{side:?}");
    assert!(
      height > 10.0 && height < 100.0,
      "{side:?}: as high as two lines: {height}"
    );
    let (_, page_y, _, page_height) = stage.bounds("page");
    assert!((page_height - (300.0 - height)).abs() < 0.5, "{side:?}");
    if side == SheetSide::Top {
      assert_eq!(y, 0.0);
      assert!((page_y - height).abs() < 0.5);
      assert_eq!(edges(&mut stage), [false, false, true, false]);
    } else {
      assert!((y + height - 300.0).abs() < 0.5);
      assert_eq!(page_y, 0.0);
      assert_eq!(edges(&mut stage), [true, false, false, false]);
    }
  }
}

#[test]
fn a_width_of_its_own() {
  let open = Signal::new(true);
  let mut stage = stage(move |_| layout(navigation(open.clone()).width(240.0), SheetSide::Start));
  assert_eq!(stage.bounds("drawer").2, 240.0);
  assert_eq!(stage.bounds("page").0, 240.0);
}

#[test]
fn tab_does_not_reach_what_a_closed_drawer_held() {
  let open = Signal::new(false);
  let (in_drawer, press_in_drawer) = counter();
  let (on_page, press_on_page) = counter();
  let mut stage = stage(move |_| {
    let (press_in_drawer, press_on_page) = (press_in_drawer.clone(), press_on_page.clone());
    Row::new()
      .size(600.0, 300.0)
      .child(Drawer::new(open.clone()).child(Button::new("Settings").on_click(move |_| press_in_drawer())))
      .child(Button::new("Compose").on_click(move |_| press_on_page()))
      .into()
  });
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!(
    (in_drawer.load(Ordering::SeqCst), on_page.load(Ordering::SeqCst)),
    (0, 1)
  );
}

#[test]
fn a_drawer_over_the_page_is_a_sheet_at_its_edge() {
  let open = Signal::new(true);
  let held = open.clone();
  let mut stage = stage(move |_| layout(navigation(held.clone()).overlay(true), SheetSide::Start));
  // It takes no room: the page starts at the layout's start and is as wide as it, under the drawer.
  let (page_x, _, page_width, _) = stage.bounds("page");
  assert_eq!((page_x, page_width), (0.0, 600.0));
  assert_eq!(stage.bounds("drawer"), (0.0, 0.0, 300.0, 300.0));
  assert!(
    stage.quads().iter().any(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Scrim))
        && (quad.width, quad.height) == (600.0, 300.0)
    }),
    "over a veil"
  );
  // What it holds starts at its edge, as in the drawer.
  assert!((stage.text_quad("Inbox").unwrap().x - 0.0).abs() < 0.5);
  stage.key("Escape");
  assert!(!open.get());
}

#[test]
fn a_theme_restyles_the_panel() {
  let app = App::new();
  app
    .theme()
    .patch_recipe::<Drawer>(|rules| rules.base(DrawerPart::Panel, |s| s.background(PaletteColor::Muted)));
  let open = Signal::new(true);
  let mut stage = stage_in(app, move |_| layout(navigation(open.clone()), SheetSide::Start));
  assert_eq!(stage.fill_color("drawer"), Some(light(PaletteColor::Muted)));
}
