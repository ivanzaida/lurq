//! A navbar: its bar, brand, links and end as the board draws them, a link that opens a panel, the keyboard through
//! its links, the brand that is pressed, the narrow form with its links over the page behind the menu button, the
//! bar that stays while the page scrolls, what tooling reads, and a theme's rules.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App,
    ctx::Ctx,
    events::ScrollPhase,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Button, Column, Row, ScrollVertical},
  core::Signal,
  layout::{Constraints, Size, quad::QuadContent},
  node::{Element, color::Color},
  ui::{Avatar, NavItem, NavLink, NavPanel, Navbar, NavbarBrand, NavbarPart, icons},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn close(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.5
}

/// The board's navbar, 760 wide: the brand, Docs (the page shown, with a panel), Components and Blog, and an avatar
/// at its end.
fn board() -> Navbar {
  board_with(|| {})
}

/// The board's navbar, with `on_blog` run when Blog is chosen.
fn board_with(on_blog: impl Fn() + Send + Sync + 'static) -> Navbar {
  Navbar::new()
    .id("bar")
    .label("Site")
    .brand(NavbarBrand::new("lurq").icon(icons::BOX).id("brand"))
    .item(
      NavItem::new("Docs")
        .current(true)
        .panel(NavPanel::new().link(NavLink::new("Getting started").id("start"))),
    )
    .item(NavItem::new("Components").id("components"))
    .item(NavItem::new("Blog").id("blog").on_select(on_blog))
    .trailing(Avatar::new("Ivan Zaida").id("avatar"))
}

/// A window of 800 by 500 with the bar 760 wide at its top and a button under it.
fn stage(make: impl Fn() -> Navbar + Send + Sync + 'static) -> Stage {
  stage_in(App::new(), make)
}

fn stage_in(app: App, make: impl Fn() -> Navbar + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::in_app(app, move |ctx: &mut Ctx| -> Element {
    Column::new()
      .size(800.0, 500.0)
      .child(Column::new().width(760.0).child(make().mount(ctx)))
      .child(Row::new().child(Button::new("Page").id("page").tab_index(0)))
      .into()
  });
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(800.0, 500.0))));
  stage.tree.resize(800, 500);
  stage.pass();
  stage
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

#[test]
fn at_rest_the_bar_is_as_the_board_draws_it() {
  let mut stage = stage(|| board().narrow(false));
  let (x, y, width, height) = stage.bounds("bar");
  assert_eq!((width, height), (760.0, 56.0));
  assert_eq!(stage.fill_color("bar"), Some(light(PaletteColor::Background)));
  // A line of 1 in the border's color along its bottom.
  let edge = light(PaletteColor::Border);
  assert!(
    stage.quads().iter().any(|quad| {
      let line = matches!(&quad.content, QuadContent::Rect { color, .. } if *color == edge)
        && close(quad.height, 1.0)
        && close(quad.y, y + height - 1.0);
      let side = quad
        .border
        .and_then(|border| border.bottom)
        .is_some_and(|side| side.color == edge && side.width == 1.0);
      (line || side) && close(quad.width, width)
    }),
    "the line along its bottom"
  );
  // The brand, 16 in: its mark of 20, 8 before its name in 16.
  let (bx, ..) = stage.bounds("brand");
  assert!(close(bx - x, 16.0));
  assert_eq!(stage.text("lurq"), (light(PaletteColor::Foreground), 16.0));
  let name = stage.text_quad("lurq").unwrap();
  assert!(close(name.x - bx, 28.0), "after the mark and 8: {}", name.x - bx);
  // The links 24 after the brand, 20 apart, in 13: the page shown in the foreground, the others muted.
  let (_, _, bw, _) = stage.bounds("brand");
  let (lx, ..) = stage.bounds("bar-links-0");
  assert!(close(lx - (bx + bw), 24.0), "24 after the brand: {}", lx - (bx + bw));
  assert_eq!(stage.text("Docs"), (light(PaletteColor::Foreground), 13.0));
  assert_eq!(stage.text("Blog"), (light(PaletteColor::MutedForeground), 13.0));
  let (cx, _, cw, _) = stage.bounds("components");
  let (blog, ..) = stage.bounds("blog");
  assert!(close(blog - (cx + cw), 20.0));
  // The avatar 16 from its end.
  let (ax, _, aw, _) = stage.bounds("avatar");
  assert!(close(x + width - (ax + aw), 16.0));
  assert!(!stage.paints("Getting started"));
}

#[test]
fn a_link_with_a_panel_opens_it_under_the_bar() {
  let open = Signal::new(None);
  let held = open.clone();
  let mut stage = stage(move || board().narrow(false).open(held.clone()));
  stage.click("bar-links-0");
  assert_eq!(open.get(), Some(0));
  stage.pass();
  let (_, y, _, h) = stage.bounds("bar");
  let (_, py, ..) = stage.bounds("bar-links-0-panel");
  assert!(py >= y + h, "under the bar: {py}");
  stage.click("start");
  assert_eq!(open.get(), None, "a choice closes it");
}

#[test]
fn tab_goes_through_the_brand_and_each_link() {
  let chosen = Arc::new(AtomicUsize::new(0));
  let count = chosen.clone();
  let mut stage = stage(move || {
    let count = count.clone();
    board().narrow(false).brand(
      NavbarBrand::new("lurq")
        .icon(icons::BOX)
        .id("brand")
        .on_select(move || {
          count.fetch_add(1, Ordering::SeqCst);
        }),
    )
  });
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("brand"));
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  stage.key("Enter");
  assert_eq!(
    chosen.load(Ordering::SeqCst),
    1,
    "the brand is pressed from the keyboard"
  );
  for id in ["bar-links-0", "components", "blog", "page"] {
    stage.key("Tab");
    assert_eq!(focused(&stage).as_deref(), Some(id));
  }
}

#[test]
fn on_a_narrow_window_the_links_are_behind_the_menu_button() {
  let menu_open = Signal::new(false);
  let chosen = Arc::new(AtomicUsize::new(0));
  let (held, told) = (menu_open.clone(), chosen.clone());
  let mut stage = stage(move || {
    let told = told.clone();
    board_with(move || {
      told.fetch_add(1, Ordering::SeqCst);
    })
    .narrow(true)
    .menu_open(held.clone())
  });
  assert!(!stage.paints("Blog"), "no links in the bar");
  let (x, ..) = stage.bounds("bar");
  let (mx, ..) = stage.bounds("bar-menu");
  assert!(close(mx - x, 8.0), "the menu button first, 8 in");
  assert!(stage.bounds("brand").0 > mx, "the brand after it");
  stage.click("bar-menu");
  assert!(menu_open.get());
  stage.pass();
  // Over the page: each link, and the links of a panel under its name.
  assert!(stage.paints("Blog") && stage.paints("Getting started"));
  stage.tree.get_element_by_id_mut("blog").unwrap().click();
  stage.pass();
  assert_eq!(chosen.load(Ordering::SeqCst), 1, "the link is chosen");
  assert!(!menu_open.get(), "choosing a link closes it");
}

#[cfg(feature = "mcp")]
#[test]
fn tooling_reads_a_navigation_of_links() {
  let mut wide = stage(|| board().narrow(false));
  let said = |stage: &mut Stage, id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(said(&mut wide, "bar", "a11y_role").as_deref(), Some("navigation"));
  assert_eq!(said(&mut wide, "bar", "a11y_name").as_deref(), Some("Site"));
  assert_eq!(said(&mut wide, "blog", "a11y_role").as_deref(), Some("link"));
  assert_eq!(said(&mut wide, "bar-links-0", "a11y_role").as_deref(), Some("button"));
  assert_eq!(
    said(&mut wide, "bar-links-0", "a11y_expanded").as_deref(),
    Some("false")
  );

  let mut narrow = stage(|| board().narrow(true));
  assert_eq!(said(&mut narrow, "bar-menu", "a11y_expanded").as_deref(), Some("false"));
  assert_eq!(
    said(&mut narrow, "bar-menu", "a11y_controls").as_deref(),
    Some("bar-drawer")
  );
  // The menu button has one name, open or not: whether the links are open it says on its own.
  assert_eq!(said(&mut narrow, "bar-menu", "a11y_name").as_deref(), Some("Menu"));
  narrow.click("bar-menu");
  narrow.pass();
  assert_eq!(said(&mut narrow, "bar-menu", "a11y_name").as_deref(), Some("Menu"));
  assert_eq!(said(&mut narrow, "bar-menu", "a11y_expanded").as_deref(), Some("true"));
  let mut named = stage(|| board().narrow(true).menu_label("Site menu"));
  assert_eq!(said(&mut named, "bar-menu", "a11y_name").as_deref(), Some("Site menu"));
}

#[test]
fn a_theme_restyles_its_parts() {
  let app = App::new();
  app
    .theme()
    .patch_recipe::<Navbar>(|rules| rules.base(NavbarPart::Root, |s| s.height(64.0).background(PaletteColor::Muted)));
  let mut stage = stage_in(app, || board().narrow(false));
  let (.., height) = stage.bounds("bar");
  assert_eq!(height, 64.0);
  assert_eq!(stage.fill_color("bar"), Some(light(PaletteColor::Muted)));
}

#[test]
fn a_brand_shows_its_media_in_place_of_an_icon() {
  let mut stage = stage(|| {
    Navbar::new()
      .id("bar")
      .brand(NavbarBrand::new("lurq").media(Row::new().size(24.0, 24.0).id("logo")))
      .narrow(false)
  });
  let (logo_x, _, logo_width, _) = stage.bounds("logo");
  let (bar_x, ..) = stage.bounds("bar");
  assert_eq!(logo_x - bar_x, 16.0, "at the start, in the bar's padding");
  let name = stage.text_quad("lurq").unwrap();
  assert!(
    (name.x - (logo_x + logo_width) - 8.0).abs() < 0.5,
    "the name 8 after it"
  );
}

#[test]
fn links_that_do_not_fit_give_way_in_their_own_room_and_never_run_under_what_follows_them() {
  let mut stage = Stage::new(|ctx: &mut Ctx| -> Element {
    Column::new()
      .size(480.0, 300.0)
      .child(
        Column::new().width(480.0).child(
          board()
            .narrow(false)
            .trailing(Row::new().size(200.0, 32.0).id("search"))
            .mount(ctx),
        ),
      )
      .into()
  });
  stage.tree.resize(480, 300);
  stage.pass();
  // What follows the links starts with the board's avatar.
  let search = stage.bounds("avatar").0.min(stage.bounds("search").0);
  for label in ["Docs", "Components", "Blog"] {
    for quad in stage
      .quads()
      .iter()
      .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == label))
    {
      let right = if quad.clip.active {
        (quad.x + quad.width).min(quad.clip.x + quad.clip.width)
      } else {
        quad.x + quad.width
      };
      assert!(
        right <= search + 0.5,
        "{label} is drawn to {right}, under what starts at {search}"
      );
    }
  }
}

#[test]
fn a_sticky_bar_is_at_the_top_after_the_page_has_scrolled_under_it() {
  for wide in [false, true] {
    let mut stage = Stage::new(move |ctx| {
      ScrollVertical::new(
        Column::new()
          .width(380.0)
          .child(board().narrow(wide).sticky(true).mount(ctx))
          .child(Row::new().size(380.0, 40.0).id("first"))
          .child(Row::new().size(380.0, 900.0)),
      )
      .size(380.0, 280.0)
      .into()
    });
    stage.tree.resize(400, 300);
    stage.pass();
    let (_, top, ..) = stage.bounds("bar");
    let (_, first, ..) = stage.bounds("first");
    stage.tree.mouse_move(150.0, 150.0);
    stage.tree.scroll(150.0, 150.0, 0.0, -200.0, ScrollPhase::Scroll);
    stage.pass();
    let (_, scrolled, ..) = stage.bounds("first");
    assert!(scrolled < first - 100.0, "the page scrolled: {first} to {scrolled}");
    let name = stage.text_quad("lurq").expect("the brand is painted");
    assert!(
      name.y >= top && name.y < top + 56.0,
      "the bar is at the top still: its brand at {}",
      name.y
    );
    let (_, bar, ..) = stage.bounds("bar");
    assert!((bar - top).abs() < 0.5, "the bar at {bar}, the top at {top}");
  }
}

#[test]
fn a_bar_made_sticky_once_it_is_shown_sticks() {
  // The playground turns `sticky` on in a bar already laid out: what it was laid out as must not keep it from
  // sticking.
  let on = Signal::new(false);
  let read = on.clone();
  let mut stage = Stage::new(move |ctx| {
    ScrollVertical::new(
      Column::new()
        .width(380.0)
        .child(board().narrow(false).sticky(read.get()).mount(ctx))
        .child(Row::new().size(380.0, 40.0).id("first"))
        .child(Row::new().size(380.0, 900.0)),
    )
    .size(380.0, 280.0)
    .into()
  });
  stage.tree.resize(400, 300);
  stage.pass();
  let (_, top, ..) = stage.bounds("bar");
  on.set(true);
  stage.pass();
  let (_, first, ..) = stage.bounds("first");
  stage.tree.mouse_move(150.0, 150.0);
  stage.tree.scroll(150.0, 150.0, 0.0, -200.0, ScrollPhase::Scroll);
  stage.pass();
  let (_, scrolled, ..) = stage.bounds("first");
  assert!(scrolled < first - 100.0, "the page scrolled: {first} to {scrolled}");
  let name = stage
    .text_quad("lurq")
    .expect("the brand is painted: the bar is at the top");
  assert!(name.y >= top && name.y < top + 56.0, "the brand at {}", name.y);
}

#[test]
fn over_the_page_a_link_after_a_panel_stands_beside_its_item_and_not_among_its_links() {
  let menu_open = Signal::new(true);
  let held = menu_open.clone();
  let stage = stage(move || board().narrow(true).menu_open(held.clone()));
  let quad = |label: &str| stage.text_quad(label).unwrap_or_else(|| panic!("{label} is painted"));
  let (docs, start, components, blog) = (quad("Docs"), quad("Getting started"), quad("Components"), quad("Blog"));
  // Docs is an item as Components and Blog are (its text after the chevron that opens it), and its links are
  // under it, in from where the items stand.
  assert!(
    docs.x > components.x,
    "Docs, an item with its chevron: {} / {}",
    docs.x,
    components.x
  );
  assert!(
    start.y > docs.y && start.x > components.x + 8.0,
    "Getting started in from the items: {}",
    start.x
  );
  assert_eq!(blog.x, components.x, "Components and Blog stand where the items do");
  assert!(components.y > start.y);
}
