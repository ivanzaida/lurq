//! A navigation menu: its items and panel as the board draws them, a press and the pointer opening a panel and
//! handing it over, the delays, the keyboard (a stop for each item, Enter and Down into a panel, Left and Right to
//! the next item, Tab out of a panel, Escape), choosing a link, a press outside, a disabled item, the links variant,
//! what tooling reads, and a theme's rules and anatomy.

use std::{
  sync::{Arc, Mutex},
  time::{Duration, Instant},
};

use lurq::{
  app::{
    App,
    ctx::Ctx,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Button, Column, Row},
  core::Signal,
  layout::{
    Constraints, Size,
    quad::{Quad, QuadContent},
  },
  node::{Element, color::Color},
  ui::{
    NavItem, NavLink, NavPanel, NavigationMenu, NavigationMenuPart, NavigationMenuRenderer, NavigationMenuVariant,
    NavigationMenuView,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn close(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.5
}

/// What was chosen, in order.
type Log = Arc<Mutex<Vec<&'static str>>>;

fn tell(log: &Log, name: &'static str) -> impl Fn() + Send + Sync + 'static {
  let log = log.clone();
  move || log.lock().unwrap().push(name)
}

/// The board's menu: Docs with a panel of two columns, Components with a panel of one, Blog a link, and Old, a link
/// that is disabled.
fn board(log: &Log) -> NavigationMenu {
  NavigationMenu::new()
    .id("nav")
    .label("Site")
    .item(
      NavItem::new("Docs").panel(
        NavPanel::new()
          .columns(2)
          .link(
            NavLink::new("Getting started")
              .description("Install lurq and open your first window.")
              .on_select(tell(log, "start")),
          )
          .link(NavLink::new("Theming").description("Roles, palettes and dark mode.")),
      ),
    )
    .item(
      NavItem::new("Components").panel(
        NavPanel::new()
          .link(NavLink::new("Button").on_select(tell(log, "button")))
          .link(NavLink::new("Input")),
      ),
    )
    .item(NavItem::new("Blog").on_select(tell(log, "blog")))
    .item(NavItem::new("Old").disabled(true).on_select(tell(log, "old")))
}

/// A window of 800 by 500 with a button before the menu (`before`), the menu at 20, 20 and a button after it
/// (`after`). `make` builds the menu, which is mounted with the open signal of the scene.
fn scene_with(make: impl Fn(&Log) -> NavigationMenu + Send + Sync + 'static) -> (Stage, Signal<Option<usize>>, Log) {
  let open = Signal::new(None);
  let log: Log = Arc::new(Mutex::new(Vec::new()));
  let (held, told) = (open.clone(), log.clone());
  let mut stage = Stage::new(move |ctx: &mut Ctx| -> Element {
    Column::new()
      .size(800.0, 500.0)
      .padding(20.0)
      .spacing(8.0)
      .child(Row::new().child(Button::new("Before").id("before").tab_index(0)))
      .child(Row::new().child(make(&told).open(held.clone()).mount(ctx)))
      .child(Row::new().child(Button::new("After").id("after").tab_index(0)))
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(800.0, 500.0))));
  stage.tree.resize(800, 500);
  stage.pass();
  (stage, open, log)
}

fn scene() -> (Stage, Signal<Option<usize>>, Log) {
  scene_with(board)
}

/// Lets the timers run `later` from now, and paints.
fn after(stage: &mut Stage, later: Duration) {
  stage.tree.tick_timers_at(Instant::now() + later);
  stage.pass();
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

fn rect_color(quad: &Quad) -> Option<Color> {
  match quad.content {
    QuadContent::Rect { color, .. } if color.a() > 0 => Some(color),
    _ => None,
  }
}

#[test]
fn at_rest_its_items_are_as_the_board_draws_them() {
  let (mut stage, ..) = scene();
  let (x0, y0, w0, h0) = stage.bounds("nav-0");
  let (x1, ..) = stage.bounds("nav-1");
  assert_eq!(h0, 36.0, "36 tall");
  assert!(close(x1 - (x0 + w0), 4.0), "4 apart: {}", x1 - (x0 + w0));
  // Padded 12 at the start: the text is 12 in.
  let label = stage.text_quad("Docs").expect("the label");
  assert!(close(label.x - x0, 12.0), "12 in: {}", label.x - x0);
  let (color, size) = stage.text("Docs");
  assert_eq!((color, size), (light(PaletteColor::Foreground), 13.0));
  // A chevron of 14 after the text of an item with a panel, none after a link's.
  assert!(
    stage.quads().iter().any(|quad| {
      quad.x > label.x + label.width && quad.x < x0 + w0 && close(quad.width, 14.0) && quad.y >= y0 && quad.y < y0 + h0
    }),
    "the chevron"
  );
  // Nothing is lit, and no panel is open.
  assert_eq!(stage.fill_color("nav-0"), None);
  assert!(!stage.paints("Getting started"));
}

#[test]
fn a_press_opens_the_panel_under_its_item_as_the_board_draws_it() {
  let (mut stage, open, _) = scene();
  stage.click("nav-0");
  assert_eq!(open.get(), Some(0));
  stage.pass();
  assert_eq!(
    stage.fill_color("nav-0"),
    Some(light(PaletteColor::Highlight)),
    "the open item is lit"
  );
  let (x, y, _, h) = stage.bounds("nav-0");
  let (px, py, pw, _) = stage.bounds("nav-0-panel");
  assert!(close(px, x), "under its item's start: {px} {x}");
  assert!(close(py, y + h + 6.0), "6 under it: {py}");
  assert!(close(pw, 440.0), "two columns of 208, padded 8, 8 apart: {pw}");
  assert_eq!(stage.fill_color("nav-0-panel"), Some(light(PaletteColor::Popover)));
  // A link: its title 14 in the foreground over its description 13 in the muted foreground, padded 12.
  assert_eq!(stage.text("Getting started"), (light(PaletteColor::Foreground), 14.0));
  assert_eq!(
    stage.text("Install lurq and open your first window."),
    (light(PaletteColor::MutedForeground), 13.0)
  );
  let (lx, ly, lw, _) = stage.bounds("nav-0-0");
  assert!(close(lw, 208.0), "a column wide: {lw}");
  let title = stage.text_quad("Getting started").unwrap();
  assert!(close(title.x - lx, 12.0) && close(title.y - ly, 12.0));
  let (theming, ..) = stage.bounds("nav-0-1");
  assert!(close(theming - (lx + lw), 8.0), "the second column 8 after the first");
  // A press of its item again closes it.
  stage.click("nav-0");
  assert_eq!(open.get(), None);
}

#[test]
fn the_pointer_opens_a_panel_after_its_delay_and_hands_it_over_at_once() {
  let (mut stage, open, _) = scene();
  stage.hover("nav-0");
  assert_eq!(open.get(), None, "not before its delay");
  after(&mut stage, Duration::from_millis(200));
  assert_eq!(open.get(), Some(0), "after the theme's base duration, 160");
  assert!(stage.paints("Getting started"));
  // With one open, the pointer on another item opens that one at once.
  stage.hover("nav-1");
  assert_eq!(open.get(), Some(1));
  assert!(stage.paints("Button") && !stage.paints("Getting started"));
  // The pointer going clear of both closes it after the close delay.
  stage.tree.mouse_move(780.0, 480.0);
  stage.pass();
  assert_eq!(open.get(), Some(1), "not at once");
  after(&mut stage, Duration::from_millis(300));
  assert_eq!(open.get(), None, "after the theme's slow duration, 240");
}

#[test]
fn the_pointer_crossing_to_the_panel_keeps_it() {
  let (mut stage, open, _) = scene();
  stage.hover("nav-0");
  after(&mut stage, Duration::from_millis(200));
  // Through the room between the item and the panel, onto a link.
  let (x, y, _, h) = stage.bounds("nav-0");
  stage.tree.mouse_move(x + 10.0, y + h + 3.0);
  stage.pass();
  stage.hover("nav-0-1");
  after(&mut stage, Duration::from_millis(400));
  assert_eq!(open.get(), Some(0));
  assert_eq!(
    stage.fill_color("nav-0-1"),
    Some(light(PaletteColor::Highlight)),
    "the link under the pointer is lit"
  );
}

#[test]
fn a_pointer_resting_before_a_press_keeps_the_panel_open() {
  let (mut stage, open, _) = scene();
  stage.hover("nav-0");
  after(&mut stage, Duration::from_millis(200));
  assert_eq!(open.get(), Some(0));
  stage.click("nav-0");
  assert_eq!(
    open.get(),
    Some(0),
    "the press that follows the pointer opening it keeps it"
  );
  stage.click("nav-0");
  assert_eq!(open.get(), None);
}

#[test]
fn tab_stops_at_each_item_and_passes_a_disabled_one_by() {
  let (mut stage, open, _) = scene();
  stage.click("before");
  for id in ["nav-0", "nav-1", "nav-2", "after"] {
    stage.key("Tab");
    assert_eq!(focused(&stage).as_deref(), Some(id));
  }
  assert_eq!(open.get(), None, "Tab opens nothing");
  // Left and Right move between the items as well.
  stage.click("before");
  stage.key("Tab");
  stage.key("ArrowRight");
  assert_eq!(focused(&stage).as_deref(), Some("nav-1"));
  stage.key("ArrowLeft");
  assert_eq!(focused(&stage).as_deref(), Some("nav-0"));
  assert_eq!(
    stage.fill_color("nav-0"),
    Some(light(PaletteColor::Highlight)),
    "focused from the keyboard, it is lit"
  );
}

#[test]
fn enter_opens_a_panel_with_the_focus_on_its_first_link_and_escape_gives_it_back() {
  let (mut stage, open, _) = scene();
  stage.click("before");
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!(open.get(), Some(0));
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("nav-0-0"));
  assert_eq!(stage.fill_color("nav-0-0"), Some(light(PaletteColor::Highlight)));
  // Down and Up go through the links.
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("nav-0-1"));
  stage.key("ArrowUp");
  assert_eq!(focused(&stage).as_deref(), Some("nav-0-0"));
  stage.key("Escape");
  assert_eq!(open.get(), None);
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("nav-0"));
}

#[test]
fn down_and_space_open_a_panel_from_the_keyboard() {
  let (mut stage, open, _) = scene();
  stage.click("before");
  stage.key("Tab");
  stage.key("ArrowDown");
  assert_eq!(open.get(), Some(0));
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("nav-0-0"));
  stage.key("Escape");
  stage.key(" ");
  assert_eq!(open.get(), Some(0));
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("nav-0-0"));
}

#[test]
fn left_and_right_in_a_panel_go_to_the_next_item_opening_its_panel_and_pressing_no_link() {
  let (mut stage, open, log) = scene();
  stage.click("before");
  stage.key("Tab");
  stage.key("Enter");
  stage.pass();
  stage.key("ArrowRight");
  assert_eq!(open.get(), Some(1), "Components opens");
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("nav-1-0"), "on its first link");
  // The next is a link: the panel closes and the focus goes to it, which is not followed.
  stage.key("ArrowRight");
  assert_eq!(open.get(), None);
  assert_eq!(focused(&stage).as_deref(), Some("nav-2"));
  assert!(log.lock().unwrap().is_empty(), "Blog is not chosen");
  // Alt and a letter are a menubar's, not a site's.
  stage
    .tree
    .key_down("Alt".to_owned(), "AltLeft".to_owned(), false, false, true);
  stage
    .tree
    .key_down("d".to_owned(), "KeyD".to_owned(), false, false, true);
  stage.tree.key_up("d".to_owned(), "KeyD".to_owned(), false, false, true);
  stage
    .tree
    .key_up("Alt".to_owned(), "AltLeft".to_owned(), false, false, false);
  stage.pass();
  assert_eq!(open.get(), None);
  assert_eq!(focused(&stage).as_deref(), Some("nav-2"));
}

#[test]
fn tab_goes_through_the_open_panel_and_out_after_its_item_closing_it() {
  let (mut stage, open, _) = scene();
  stage.click("before");
  stage.key("Tab");
  stage.key("Enter");
  stage.pass();
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("nav-0-1"));
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("nav-1"), "on to what follows its item");
  after(&mut stage, Duration::from_millis(10));
  assert_eq!(open.get(), None, "the focus left it");
}

#[test]
fn choosing_a_link_runs_it_and_closes_the_panel() {
  let (mut stage, open, log) = scene();
  stage.click("nav-0");
  stage.click("nav-0-0");
  assert_eq!(*log.lock().unwrap(), vec!["start"]);
  assert_eq!(open.get(), None);
  // An item that is a link is chosen by a press.
  stage.click("nav-2");
  assert_eq!(*log.lock().unwrap(), vec!["start", "blog"]);
}

#[test]
fn a_press_outside_closes_the_panel_and_reaches_what_it_is_on() {
  let (mut stage, open, _) = scene();
  stage.click("nav-0");
  assert_eq!(open.get(), Some(0));
  // The button before the menu: the one after it is under the open panel.
  stage.click("before");
  assert_eq!(open.get(), None);
  assert_eq!(
    focused(&stage).as_deref(),
    Some("before"),
    "the press went on to the button"
  );
}

#[test]
fn a_disabled_item_is_dimmed_and_takes_nothing() {
  let (mut stage, open, log) = scene();
  assert!(stage.paints("Old"));
  stage.click("nav-3");
  assert!(log.lock().unwrap().is_empty());
  assert_eq!(open.get(), None);
  // A disabled menu opens nothing.
  let (mut stage, open, _) = scene_with(|log| board(log).disabled(true));
  stage.click("nav-0");
  stage.hover("nav-1");
  after(&mut stage, Duration::from_millis(400));
  assert_eq!(open.get(), None);
}

#[test]
fn a_panel_with_an_icon_and_a_featured_card() {
  let (mut stage, ..) = scene_with(|_| {
    NavigationMenu::new().id("nav").item(
      NavItem::new("Docs").panel(
        NavPanel::new()
          .featured(Column::new().size(100.0, 80.0).id("featured"))
          .link(NavLink::new("Getting started").icon(lurq::ui::icons::HOME)),
      ),
    )
  });
  stage.click("nav-0");
  let (fx, _, fw, _) = stage.bounds("featured");
  let (px, _, pw, _) = stage.bounds("nav-0-panel");
  assert!(close(fx, px + 8.0), "first, inside the panel's padding");
  assert!(close(pw, 8.0 + 208.0 + 8.0 + 208.0 + 8.0), "a column of its own: {pw}");
  assert!(fw <= 208.0);
  // The icon of 16 stands before the title, 12 before it.
  let (lx, ..) = stage.bounds("nav-0-0");
  let title = stage.text_quad("Getting started").unwrap();
  assert!(
    close(title.x - lx, 12.0 + 16.0 + 12.0),
    "after the icon: {}",
    title.x - lx
  );
}

#[test]
fn the_links_variant_is_the_text_alone_with_the_page_shown_brought_forward() {
  let (mut stage, open, _) = scene_with(|log| {
    board(log)
      .variant(NavigationMenuVariant::Links)
      .item(NavItem::new("About").current(true))
  });
  let (x0, _, w0, h0) = stage.bounds("nav-0");
  let (x1, ..) = stage.bounds("nav-1");
  assert_eq!(h0, 32.0);
  let label = stage.text_quad("Docs").unwrap();
  assert!(close(label.x, x0), "no padding");
  assert!(close(x1 - (x0 + w0), 20.0), "20 apart: {}", x1 - (x0 + w0));
  // The chevron of 14 is 6 after the text, as in the menu variant.
  let chevron = stage
    .quads()
    .into_iter()
    .find(|quad| quad.x > label.x + label.width && quad.x < x0 + w0 && close(quad.width, 14.0))
    .expect("the chevron");
  assert!(
    close(chevron.x - (label.x + label.width), 6.0),
    "{}",
    chevron.x - (label.x + label.width)
  );
  assert_eq!(stage.text("Blog").0, light(PaletteColor::MutedForeground));
  assert_eq!(stage.text("About").0, light(PaletteColor::Foreground), "the page shown");
  stage.click("nav-0");
  assert_eq!(open.get(), Some(0));
  stage.pass();
  assert_eq!(stage.fill_color("nav-0"), None, "an open item is not filled");
  assert_eq!(stage.text("Docs").0, light(PaletteColor::Foreground));
  let (_, py, ..) = stage.bounds("nav-0-panel");
  let (_, y, _, h) = stage.bounds("nav-0");
  assert!(close(py, y + h + 18.0), "under the navbar's edge: {py}");
}

#[cfg(feature = "mcp")]
#[test]
fn tooling_reads_a_navigation_of_links_and_of_buttons_that_open_panels() {
  let (mut stage, ..) = scene_with(|log| board(log).item(NavItem::new("About").current(true)));
  let said = |stage: &mut Stage, id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(said(&mut stage, "nav", "a11y_role").as_deref(), Some("navigation"));
  assert_eq!(said(&mut stage, "nav", "a11y_name").as_deref(), Some("Site"));
  assert_eq!(said(&mut stage, "nav-0", "a11y_role").as_deref(), Some("button"));
  assert_eq!(said(&mut stage, "nav-0", "a11y_expanded").as_deref(), Some("false"));
  assert_eq!(
    said(&mut stage, "nav-0", "a11y_controls").as_deref(),
    Some("nav-0-panel")
  );
  assert_eq!(said(&mut stage, "nav-2", "a11y_role").as_deref(), Some("link"));
  assert_eq!(said(&mut stage, "nav-4", "a11y_current").as_deref(), Some("page"));
  stage.click("nav-0");
  assert_eq!(said(&mut stage, "nav-0", "a11y_expanded").as_deref(), Some("true"));
  assert_eq!(said(&mut stage, "nav-0-0", "a11y_role").as_deref(), Some("link"));
  assert_eq!(
    said(&mut stage, "nav-0-0", "a11y_description").as_deref(),
    Some("Install lurq and open your first window.")
  );
}

#[test]
fn a_theme_restyles_its_parts_and_builds_it_anew() {
  let app = App::new();
  app
    .theme()
    .patch_recipe::<NavigationMenu>(|rules| rules.base(NavigationMenuPart::Item, |s| s.padding_x(20.0)));
  let mut stage = Stage::in_app(app, |ctx| -> Element {
    Column::new()
      .size(400.0, 300.0)
      .child(NavigationMenu::new().id("nav").item(NavItem::new("Blog")).mount(ctx))
      .into()
  });
  let (x, ..) = stage.bounds("nav-0");
  let label = stage.text_quad("Blog").unwrap();
  assert!(close(label.x - x, 20.0));

  // A theme whose menu lays its items out last first: other nodes, the same behavior.
  let app = App::new();
  let renderer: NavigationMenuRenderer = Arc::new(|mut view: NavigationMenuView| {
    let mut bar = view.bar();
    for index in (0..view.items.len()).rev() {
      let item = view.item(index).child(view.label(index));
      bar = bar.child(item).with_children(view.panel(index));
    }
    bar.into()
  });
  app.theme().set_renderer::<NavigationMenu>(renderer);
  let open = Signal::new(None);
  let held = open.clone();
  let mut stage = Stage::in_app(app, move |ctx| -> Element {
    Column::new()
      .size(400.0, 300.0)
      .child(
        NavigationMenu::new()
          .id("nav")
          .open(held.clone())
          .item(NavItem::new("Docs").panel(NavPanel::new().link(NavLink::new("Start"))))
          .item(NavItem::new("Blog"))
          .mount(ctx),
      )
      .into()
  });
  assert!(stage.bounds("nav-1").0 < stage.bounds("nav-0").0);
  stage.click("nav-0");
  assert_eq!(open.get(), Some(0));
  assert!(stage.paints("Start"));
  assert!(
    stage
      .quads()
      .iter()
      .any(|quad| rect_color(quad) == Some(light(PaletteColor::Popover)))
  );
}

#[test]
fn choosing_a_link_closes_the_panel_before_its_handler_runs() {
  // The menu says it closed, and then the link's handler runs: a handler that opens a dialog keeps its focus.
  let (mut stage, open, log) = scene_with(|log| {
    let heard = log.clone();
    board(log).on_open_change(move |open| {
      if open.is_none() {
        heard.lock().unwrap().push("closed");
      }
    })
  });
  stage.click("nav-0");
  assert_eq!(open.get(), Some(0));
  stage.click("nav-0-0");
  assert_eq!(*log.lock().unwrap(), vec!["closed", "start"]);
}

#[test]
fn a_variant_is_named_as_the_theme_names_it() {
  // The links variant by its name in the recipe, as a theme's own variant would be given.
  let (mut stage, ..) = scene_with(|log| board(log).variant("links"));
  let (_, _, _, height) = stage.bounds("nav-0");
  assert_eq!(height, 32.0, "the links variant's item");
  assert_eq!(stage.fill_color("nav-0"), None);
}

#[test]
fn the_panels_shadow_is_drawn_whole_around_it() {
  let (mut stage, ..) = scene();
  stage.click("nav-0");
  stage.pass();
  let (x, y, width, height) = stage.bounds("nav-0-panel");
  let shadows: Vec<Quad> = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(quad.content, QuadContent::BoxShadow(_)) && (quad.x, quad.y) == (x, y))
    .collect();
  assert!(!shadows.is_empty(), "the panel has its shadow");
  // The medium shadow reaches past the panel's edges: what is drawn of it is not cut at the panel's box.
  for shadow in &shadows {
    let clip = shadow.clip;
    assert!(
      !clip.active
        || (clip.x <= x - 4.0 && clip.y + clip.height >= y + height + 8.0 && clip.x + clip.width >= x + width + 4.0),
      "the shadow is cut at {clip:?}, the panel is {x} {y} {width} {height}"
    );
  }
}

/// A menu of six items, wider than a narrow room: Home, Docs and Components with a panel each, Blog, Pricing (the
/// page shown) and Old, which is disabled.
fn many(log: &Log) -> NavigationMenu {
  NavigationMenu::new()
    .id("nav")
    .label("Site")
    .item(NavItem::new("Home").on_select(tell(log, "home")))
    .item(
      NavItem::new("Docs").panel(NavPanel::new().link(NavLink::new("Getting started").on_select(tell(log, "start")))),
    )
    .item(
      NavItem::new("Components").panel(
        NavPanel::new()
          .link(NavLink::new("Button").on_select(tell(log, "button")))
          .link(NavLink::new("Input").disabled(true).on_select(tell(log, "input"))),
      ),
    )
    .item(NavItem::new("Blog").on_select(tell(log, "blog")))
    .item(NavItem::new("Pricing").current(true).on_select(tell(log, "pricing")))
    .item(NavItem::new("Old").disabled(true).on_select(tell(log, "old")))
}

/// The menu of [`many`], mounted in a room `width` wide whose id is `room`, with one frame painted.
fn roomed(width: &Signal<f32>) -> (Stage, Signal<Option<usize>>, Log) {
  let open = Signal::new(None);
  let log: Log = Arc::new(Mutex::new(Vec::new()));
  let (width, held, told) = (width.clone(), open.clone(), log.clone());
  let stage = Stage::first_frame(800.0, 500.0, move |ctx: &mut Ctx| -> Element {
    Column::new()
      .size(800.0, 500.0)
      .child(
        Row::new()
          .id("room")
          .width(width.get())
          .child(many(&told).open(held.clone()).mount(ctx)),
      )
      .into()
  });
  (stage, open, log)
}

/// The box of the element with `id`, if it is laid out with a size: one the row dropped has none.
fn sized(stage: &mut Stage, id: &str) -> Option<(f32, f32, f32, f32)> {
  let bounds = stage
    .tree
    .get_element_by_id_mut(id)
    .and_then(|element| element.bounds())?;
  (bounds.width > 0.0).then_some((bounds.x, bounds.y, bounds.width, bounds.height))
}

/// The items of [`many`] that are in the row, by index.
fn in_the_row(stage: &mut Stage) -> Vec<usize> {
  (0..6)
    .filter(|index| sized(stage, &format!("nav-{index}")).is_some())
    .collect()
}

/// Says that every item in the row, and More where it stands, is whole in the room.
#[track_caller]
fn whole_in_the_room(stage: &mut Stage) {
  let (room, _, room_width, _) = stage.bounds("room");
  let mut ids: Vec<String> = in_the_row(stage)
    .into_iter()
    .map(|index| format!("nav-{index}"))
    .collect();
  ids.push("nav-more".to_owned());
  for id in ids {
    if let Some((x, _, width, _)) = sized(stage, &id) {
      assert!(
        x >= room - 0.5 && x + width <= room + room_width + 0.5,
        "{id} at {x}..{} in a room at {room}..{}",
        x + width,
        room + room_width
      );
    }
  }
}

#[test]
fn a_mounted_menu_puts_the_items_it_has_no_room_for_behind_more_from_its_first_frame() {
  let width = Signal::new(300.0f32);
  let (mut stage, ..) = roomed(&width);
  // Home and Docs fit beside More; the rest are behind it, the last first.
  assert_eq!(in_the_row(&mut stage), [0, 1]);
  let (more, y, more_width, height) = sized(&mut stage, "nav-more").expect("More stands in the row");
  whole_in_the_room(&mut stage);
  // More is an item as the others are: 36 tall, 4 after the last in the row, its text and a chevron.
  let (docs, docs_y, docs_width, _) = stage.bounds("nav-1");
  assert_eq!((y, height), (docs_y, 36.0));
  assert!(
    close(more - (docs + docs_width), 4.0),
    "4 apart: {}",
    more - (docs + docs_width)
  );
  let label = stage.text_quad("More").expect("its text");
  assert!(close(label.x - more, 12.0) && label.x + label.width < more + more_width);
  assert!(!stage.paints("Blog"), "what went is not drawn");
  let first = stage.picture();
  stage.frame();
  assert_eq!(stage.picture(), first, "settled in its first frame");
}

#[test]
fn more_opens_a_menu_of_what_went_and_choosing_there_is_choosing_the_item() {
  let width = Signal::new(300.0f32);
  let (mut stage, _, log) = roomed(&width);
  assert!(sized(&mut stage, "nav-3-item").is_none(), "closed");
  stage.click("nav-more");
  stage.pass();
  // Every item that went, in order: a panel's links under its item's name.
  for id in ["nav-2-0-item", "nav-2-1-item", "nav-3-item", "nav-4-item", "nav-5-item"] {
    assert!(sized(&mut stage, id).is_some(), "{id} is in the menu");
  }
  assert!(sized(&mut stage, "nav-0-item").is_none(), "what is in the row is not");
  assert!(stage.paints("Components") && stage.paints("Button") && stage.paints("Blog"));
  let heading = stage.text_quad("Components").unwrap();
  assert!(heading.y < stage.bounds("nav-2-0-item").1, "the name over its links");

  stage.click("nav-3-item");
  assert_eq!(*log.lock().unwrap(), vec!["blog"]);
  stage.pass();
  assert!(sized(&mut stage, "nav-3-item").is_none(), "choosing closes the menu");

  // A link of a panel is chosen as it is in its panel; a disabled one and a disabled item take nothing.
  stage.click("nav-more");
  stage.pass();
  stage.click("nav-2-1-item");
  stage.click("nav-5-item");
  assert_eq!(*log.lock().unwrap(), vec!["blog"]);
  stage.click("nav-2-0-item");
  assert_eq!(*log.lock().unwrap(), vec!["blog", "button"]);
}

#[test]
fn a_menu_follows_its_room_every_item_back_in_the_row_when_there_is_room() {
  let width = Signal::new(700.0f32);
  let (mut stage, open, _) = roomed(&width);
  // With room for all of it the menu is as it was: every item, and no More.
  assert_eq!(in_the_row(&mut stage), [0, 1, 2, 3, 4, 5]);
  assert!(sized(&mut stage, "nav-more").is_none());
  let wide: Vec<_> = (0..6).map(|index| stage.bounds(&format!("nav-{index}"))).collect();
  let (nav, _, bar, _) = stage.bounds("nav");
  assert!(
    close(nav + bar, wide[5].0 + wide[5].2),
    "as wide as its items, not as its room"
  );

  // The panel of an item that goes behind More closes.
  open.set(Some(2));
  stage.pass();
  assert!(stage.paints("Button"));
  width.set(300.0);
  stage.frame();
  assert_eq!(in_the_row(&mut stage), [0, 1], "in the frame the room narrowed in");
  whole_in_the_room(&mut stage);
  assert_eq!(open.get_untracked(), None);

  width.set(420.0);
  stage.frame();
  assert_eq!(in_the_row(&mut stage), [0, 1, 2, 3], "what a wider room has room for");
  whole_in_the_room(&mut stage);

  width.set(700.0);
  stage.frame();
  assert_eq!(in_the_row(&mut stage), [0, 1, 2, 3, 4, 5]);
  assert!(
    sized(&mut stage, "nav-more").is_none(),
    "More goes with the last of them back"
  );
  let again: Vec<_> = (0..6).map(|index| stage.bounds(&format!("nav-{index}"))).collect();
  assert_eq!(again, wide, "as a menu laid out at 700 first");
}

#[test]
fn more_is_a_stop_of_the_row_whose_keys_open_its_menu_and_it_shows_the_page_shown_behind_it() {
  let width = Signal::new(300.0f32);
  let (mut stage, _, log) = roomed(&width);
  // Pricing, the page shown, is behind More: More is drawn as the item of the page shown is.
  assert_eq!(stage.fill_color("nav-more"), Some(light(PaletteColor::Muted)));
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("nav-0"));
  stage.key("ArrowRight");
  stage.key("ArrowRight");
  assert_eq!(focused(&stage).as_deref(), Some("nav-more"), "Right goes on to More");
  stage.key("ArrowLeft");
  assert_eq!(focused(&stage).as_deref(), Some("nav-1"));
  stage.key("Tab");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("nav-more"),
    "what went is no stop of Tab"
  );
  stage.key("Enter");
  stage.pass();
  assert!(sized(&mut stage, "nav-3-item").is_some(), "Enter opens its menu");
  stage.key("Escape");
  stage.pass();
  assert!(sized(&mut stage, "nav-3-item").is_none(), "Escape closes it");
  assert!(log.lock().unwrap().is_empty());
}

#[test]
fn a_built_menu_too_wide_for_its_room_keeps_what_fits_and_more_in_it() {
  let mut stage = Stage::new(|_| {
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let menu: Element = many(&log).into();
    Column::new()
      .size(400.0, 300.0)
      .child(Row::new().id("room").width(300.0).child(menu))
      .into()
  });
  assert_eq!(in_the_row(&mut stage), [0, 1]);
  assert!(sized(&mut stage, "nav-more").is_some());
  whole_in_the_room(&mut stage);
}
