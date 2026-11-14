//! A sidebar: what it paints at rest against the board's numbers, the selected
//! item and the one under the pointer, a disabled one, nested items that open
//! and close, the keyboard, the rail with its tooltips and menus, the width
//! that moves between the two, the sidebar over the page on a narrow window,
//! what stays while the items scroll, the trigger, and a theme's rule.

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
    theme::{OpacityLevel, PaletteColor, ThemePalette},
  },
  components::{Column, Row, Text},
  core::Signal,
  layout::quad::QuadContent,
  node::{Element, color::Color},
  ui::{
    Case, Opacity, Sidebar, SidebarGroup, SidebarHeading, SidebarItem, SidebarPart, SidebarState, SidebarTrigger, icons,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// A stage whose window is a desktop's, 1024 by 600: wide enough for a sidebar beside the page.
fn stage(scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Stage {
  stage_in(App::new(), scene)
}

fn stage_in(app: App, scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::in_app(app, scene);
  stage.tree.resize(1024, 600);
  stage.pass();
  stage
}

/// Counts what is chosen.
fn counter() -> (Arc<AtomicUsize>, impl Fn() + Send + Sync + Clone + 'static) {
  let count = Arc::new(AtomicUsize::new(0));
  let add = count.clone();
  (count, move || {
    add.fetch_add(1, Ordering::SeqCst);
  })
}

/// The board's sidebar: a workspace at the top, two groups, a nested list and who is signed in at the bottom.
fn board() -> Sidebar {
  Sidebar::new()
    .id("sidebar")
    .label("Workspace")
    .header(
      SidebarHeading::new("lurq")
        .description("Workspace")
        .icon(icons::BOX)
        .trailing_icon(icons::CHEVRONS_VERTICAL)
        .id("heading"),
    )
    .header_collapsed(SidebarHeading::new("lurq").icon(icons::BOX).collapsed(true))
    .group(
      SidebarGroup::new("Project")
        .item(SidebarItem::new("Overview").icon(icons::HOME).id("overview"))
        .item(
          SidebarItem::new("Inbox")
            .icon(icons::INBOX)
            .count(12)
            .selected(true)
            .id("inbox"),
        )
        .item(
          SidebarItem::new("Components")
            .icon(icons::FOLDER)
            .id("components")
            .child(SidebarItem::new("Buttons").icon(icons::FILE).id("buttons"))
            .child(SidebarItem::new("Inputs").icon(icons::FILE).id("inputs")),
        ),
    )
    .group(
      SidebarGroup::new("Team")
        .item(SidebarItem::new("Members").icon(icons::USERS).id("members"))
        .item(SidebarItem::new("Settings").icon(icons::SETTINGS).id("settings")),
    )
    .footer(Text::new("Ivan Zaida"))
    .narrow(false)
}

/// The page beside the sidebar.
fn page() -> Column {
  Column::new().id("page").flex(1.0).child(Text::new("The page"))
}

/// The sidebar mounted beside the page, in a window of 1024 by 600.
fn layout(ctx: &mut Ctx, sidebar: Sidebar) -> Element {
  Row::new()
    .size(1024.0, 600.0)
    .child(sidebar.mount(ctx))
    .child(page())
    .into()
}

fn close(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.5
}

#[test]
fn a_sidebar_is_260_wide_on_the_muted_fill_with_a_line_toward_the_page_as_the_board_draws_it() {
  let mut stage = stage(|ctx| layout(ctx, board()));
  assert_eq!(stage.bounds("sidebar"), (0.0, 0.0, 260.0, 600.0));
  assert_eq!(stage.fill_color("sidebar"), Some(light(PaletteColor::Muted)));
  let edge = stage
    .border("sidebar")
    .expect("the sidebar has an edge")
    .border
    .unwrap();
  assert!(
    edge
      .right
      .is_some_and(|side| side.width == 1.0 && side.color == light(PaletteColor::Border))
  );
  assert!(edge.left.is_none_or(|side| side.width == 0.0));
  assert_eq!(stage.bounds("page").0, 260.0, "the page is beside it");
}

#[test]
fn an_item_is_32_high_with_its_icon_and_its_name_in_13() {
  let mut stage = stage(|ctx| layout(ctx, board()));
  let (x, _, width, height) = stage.bounds("overview");
  assert_eq!((x, height), (8.0, 32.0), "8 in from the sidebar's start");
  // At its end the room of the scroll bar's lane, which the bar is drawn in: the theme's is 8 wide, 2 from each side.
  let bar = stage.app.theme().scrollbar().clone();
  let lane = bar.width + bar.resolved_edge_inset() * 2.0;
  assert_eq!((lane, x + width), (12.0, 260.0 - 12.0));
  let heading = stage.bounds("heading");
  assert_eq!(
    (heading.0, heading.2),
    (8.0, 244.0),
    "the header has the board's 8 at either side"
  );
  assert_eq!(stage.text("Overview"), (light(PaletteColor::Foreground), 13.0));
  // 8 in, an icon of 16 and 8 after it.
  assert!(close(stage.text_quad("Overview").unwrap().x, 8.0 + 8.0 + 16.0 + 8.0));
  assert_eq!(stage.fill_color("overview"), None, "an item at rest has no fill");
}

#[test]
fn the_selected_item_is_on_the_background_with_an_edge_and_its_count_at_its_end() {
  let mut stage = stage(|ctx| layout(ctx, board()));
  assert_eq!(stage.fill_color("inbox"), Some(light(PaletteColor::Background)));
  let edge = stage
    .border("inbox")
    .expect("the selected item has an edge")
    .border
    .unwrap();
  assert!(
    edge
      .top
      .is_some_and(|side| side.width == 1.0 && side.color == light(PaletteColor::Border))
  );
  assert_eq!(stage.text("12"), (light(PaletteColor::MutedForeground), 11.0));
  let (x, _, width, _) = stage.bounds("inbox");
  let count = stage.text_quad("12").unwrap();
  assert!(close(count.x + count.width, x + width - 8.0), "at the item's end, 8 in");
}

#[test]
fn a_group_has_its_label_in_capitals_in_the_muted_foreground() {
  let stage = stage(|ctx| layout(ctx, board()));
  assert_eq!(stage.text("PROJECT"), (light(PaletteColor::MutedForeground), 11.0));
  assert!(stage.paints("TEAM"));
  assert!(!stage.paints("Project"), "the label is written in capitals");
}

#[test]
fn a_groups_count_stands_at_the_end_of_its_label_before_its_action() {
  let mut stage = stage(|ctx| {
    layout(
      ctx,
      Sidebar::new().narrow(false).group(
        SidebarGroup::new("Project")
          .count(4)
          .action(Column::new().id("add").size(24.0, 24.0))
          .item(SidebarItem::new("Overview")),
      ),
    )
  });
  let count = stage.text_quad("4").expect("the count");
  let (x, _, width, _) = stage.bounds("add");
  assert!(
    close(x + width, 260.0 - 12.0),
    "the action at the end of the rows: {}",
    x + width
  );
  assert!(
    close(count.x + count.width, x - 8.0),
    "the count 8 before the action: ends at {}, the action at {x}",
    count.x + count.width
  );
}

#[test]
fn the_item_under_the_pointer_is_veiled_and_a_press_chooses_it() {
  let (chosen, choose) = counter();
  let mut stage = stage(move |ctx| {
    let choose = choose.clone();
    layout(
      ctx,
      Sidebar::new()
        .narrow(false)
        .item(SidebarItem::new("Overview").id("overview").on_select(move || choose()))
        .item(SidebarItem::new("Inbox").id("inbox")),
    )
  });
  stage.hover("overview");
  assert_eq!(stage.fill_color("overview"), Some(light(PaletteColor::StateHover)));
  stage.click("overview");
  assert_eq!(chosen.load(Ordering::SeqCst), 1);
}

#[test]
fn a_disabled_item_is_dimmed_and_takes_nothing() {
  let (chosen, choose) = counter();
  let mut stage = stage(move |ctx| {
    let choose = choose.clone();
    layout(
      ctx,
      Sidebar::new().narrow(false).item(
        SidebarItem::new("Billing")
          .id("billing")
          .disabled(true)
          .on_select(move || choose()),
      ),
    )
  });
  stage.click("billing");
  assert_eq!(chosen.load(Ordering::SeqCst), 0);
  stage.hover("billing");
  assert_eq!(stage.fill_color("billing"), None, "no veil under the pointer");
  // Dimmed by the theme's opacity for what is disabled: a layer's, not a quad's.
  let theme = stage.app.theme().clone();
  let case = Case::new().state(SidebarState::Disabled, true);
  let item = theme.resolve::<Sidebar>(&case, None).part(SidebarPart::Item).rest();
  assert_eq!(item.opacity, Some(Opacity::Level(OpacityLevel::Disabled)));
}

#[test]
fn nested_items_open_and_close_with_a_press_and_are_18_further_in() {
  let mut stage = stage(|ctx| layout(ctx, board()));
  assert!(!stage.paints("Buttons"), "closed, its items are not there");
  stage.click("components");
  assert!(stage.paints("Buttons") && stage.paints("Inputs"));
  // The parent's name is after its chevron of 14 and its icon; a nested one's is a step of 18 in.
  let overview = stage.text_quad("Overview").unwrap().x;
  assert!(close(stage.text_quad("Components").unwrap().x, overview + 14.0 + 8.0));
  assert!(close(stage.text_quad("Buttons").unwrap().x, overview + 18.0));
  stage.click("components");
  assert!(!stage.paints("Buttons"), "closed again");
}

#[test]
fn a_nested_list_holding_the_selected_item_starts_open() {
  let mut stage = stage(|ctx| {
    layout(
      ctx,
      Sidebar::new().narrow(false).item(
        SidebarItem::new("Components")
          .id("components")
          .child(SidebarItem::new("Buttons").selected(true).id("buttons")),
      ),
    )
  });
  assert!(stage.paints("Buttons"));
  assert_eq!(stage.fill_color("buttons"), Some(light(PaletteColor::Background)));
}

#[test]
fn tab_stops_at_the_selected_item_and_the_arrows_move_through_the_items_shown() {
  let (chosen, choose) = counter();
  let (other, choose_other) = counter();
  let mut stage = stage(move |ctx| {
    let (choose, choose_other) = (choose.clone(), choose_other.clone());
    layout(
      ctx,
      Sidebar::new()
        .narrow(false)
        .item(SidebarItem::new("Overview").id("overview"))
        .item(SidebarItem::new("Inbox").id("inbox").selected(true))
        .item(SidebarItem::new("Drafts").id("drafts").on_select(move || choose()))
        .item(SidebarItem::new("Sent").id("sent").on_select(move || choose_other())),
    )
  });
  stage.key("Tab");
  assert!(
    stage.has_ring(light(PaletteColor::Ring)),
    "the ring of the focus from the keyboard"
  );
  stage.key("ArrowDown");
  stage.key("Enter");
  assert_eq!(
    (chosen.load(Ordering::SeqCst), other.load(Ordering::SeqCst)),
    (1, 0),
    "Down went from Inbox to Drafts"
  );
  stage.key("ArrowDown");
  stage.key(" ");
  assert_eq!(other.load(Ordering::SeqCst), 1, "Space chooses too");
}

#[test]
fn right_opens_an_item_with_nested_items_and_left_closes_it() {
  let (chosen, choose) = counter();
  let mut stage = stage(move |ctx| {
    let choose = choose.clone();
    layout(
      ctx,
      Sidebar::new()
        .narrow(false)
        .item(
          SidebarItem::new("Components")
            .id("components")
            .selected(true)
            .child(SidebarItem::new("Buttons").id("buttons").on_select(move || choose())),
        )
        .item(SidebarItem::new("Tokens").id("tokens")),
    )
  });
  stage.key("Tab");
  stage.key("ArrowRight");
  assert!(stage.paints("Buttons"), "Right opened it");
  // The nested items are in the list the arrows move through.
  stage.key("ArrowDown");
  stage.key("Enter");
  assert_eq!(chosen.load(Ordering::SeqCst), 1);
  stage.key("ArrowUp");
  stage.key("ArrowLeft");
  stage.pass();
  assert!(!stage.paints("Buttons"), "Left closed it");
}

#[test]
fn the_items_are_a_tree_to_the_keys() {
  let (inputs, choose_inputs) = counter();
  let (tokens, choose_tokens) = counter();
  let mut stage = stage(move |ctx| {
    let (choose_inputs, choose_tokens) = (choose_inputs.clone(), choose_tokens.clone());
    layout(
      ctx,
      Sidebar::new()
        .narrow(false)
        .item(SidebarItem::new("Overview").selected(true))
        .item(
          SidebarItem::new("Components")
            .id("components")
            .child(SidebarItem::new("Buttons").id("buttons"))
            .child(
              SidebarItem::new("Inputs")
                .id("inputs")
                .on_select(move || choose_inputs()),
            ),
        )
        .item(SidebarItem::new("Tokens").on_select(move || choose_tokens())),
    )
  });
  stage.key("Tab");
  stage.key("ArrowDown");
  // On a closed item Right opens it, and on an open one goes to the first item in it.
  stage.key("ArrowRight");
  assert!(stage.paints("Buttons"), "Right opened Components");
  stage.key("ArrowRight");
  stage.key("ArrowDown");
  stage.key("Enter");
  assert_eq!(
    inputs.load(Ordering::SeqCst),
    1,
    "Right went into it, to Buttons, and Down to Inputs"
  );
  // On a nested item Left goes to the item it is in, and there it closes it.
  stage.key("ArrowLeft");
  assert!(stage.paints("Buttons"), "Left went to Components and closed nothing");
  stage.key("ArrowLeft");
  assert!(!stage.paints("Buttons"), "Left on Components closed it");
  // What is closed is passed by: Down from Components is Tokens.
  stage.key("ArrowDown");
  stage.key("Enter");
  assert_eq!(tokens.load(Ordering::SeqCst), 1);
}

#[test]
fn a_group_that_opens_and_closes_is_an_item_its_items_are_in_to_the_keys() {
  let (chosen, choose) = counter();
  let mut stage = stage(move |ctx| {
    let choose = choose.clone();
    layout(
      ctx,
      Sidebar::new().narrow(false).group(
        SidebarGroup::new("Team")
          .collapsible(true)
          .item(SidebarItem::new("Members").selected(true))
          .item(SidebarItem::new("Settings").on_select(move || choose())),
      ),
    )
  });
  stage.key("Tab");
  stage.key("ArrowLeft");
  assert!(stage.paints("Members"), "Left went from Members to the group's label");
  stage.key("ArrowLeft");
  assert!(!stage.paints("Members"), "Left on the label closed the group");
  stage.key("ArrowRight");
  stage.key("ArrowRight");
  stage.key("ArrowDown");
  stage.key("Enter");
  assert_eq!(
    chosen.load(Ordering::SeqCst),
    1,
    "Right opened it, went to Members, Down to Settings"
  );
}

#[test]
fn typing_a_letter_goes_to_the_item_that_starts_with_it() {
  let (chosen, choose) = counter();
  let mut stage = stage(move |ctx| {
    let choose = choose.clone();
    layout(
      ctx,
      Sidebar::new()
        .narrow(false)
        .item(SidebarItem::new("Overview").selected(true))
        .item(SidebarItem::new("Inbox"))
        .item(SidebarItem::new("Members").on_select(move || choose())),
    )
  });
  stage.key("Tab");
  stage.key("m");
  stage.key("Enter");
  assert_eq!(chosen.load(Ordering::SeqCst), 1);
}

#[test]
fn collapsed_it_is_the_rail_of_the_items_icons_52_wide() {
  let mut stage = stage(|ctx| layout(ctx, board().collapsed(true)));
  assert_eq!(stage.bounds("sidebar").2, 52.0);
  assert!(!stage.paints("Overview") && !stage.paints("PROJECT") && !stage.paints("12"));
  let (x, _, width, height) = stage.bounds("inbox");
  assert_eq!((width, height), (36.0, 32.0));
  assert!(close(x, 8.0), "in the middle of the rail: {x}");
  assert_eq!(stage.fill_color("inbox"), Some(light(PaletteColor::Background)));
  assert_eq!(stage.bounds("page").0, 52.0);
}

#[test]
fn on_the_rail_an_item_says_its_name_in_a_tooltip_beside_it() {
  let mut stage = stage(|ctx| layout(ctx, board().collapsed(true)));
  assert!(!stage.paints("Inbox"));
  // The focus from the keyboard shows the tooltip at once: Tab stops at the selected item.
  stage.key("Tab");
  let name = stage.text_quad("Inbox").expect("the tooltip of the focused item");
  let (x, _, width, _) = stage.bounds("inbox");
  assert!(
    name.x > x + width,
    "beside the item, not over it: {} after {}",
    name.x,
    x + width
  );
}

#[test]
fn on_the_rail_an_item_with_nested_items_opens_them_in_a_menu() {
  let (chosen, choose) = counter();
  let mut stage = stage(move |ctx| {
    let choose = choose.clone();
    layout(
      ctx,
      Sidebar::new().narrow(false).collapsed(true).item(
        SidebarItem::new("Components")
          .icon(icons::FOLDER)
          .id("components")
          .child(SidebarItem::new("Buttons").id("buttons").on_select(move || choose())),
      ),
    )
  });
  assert!(!stage.paints("Buttons"));
  stage.click("components");
  assert!(stage.paints("Buttons"), "the menu is open");
  let (x, _, width, _) = stage.bounds("components");
  assert!(stage.text_quad("Buttons").unwrap().x > x + width, "beside the rail");
  stage.click("buttons");
  assert_eq!(chosen.load(Ordering::SeqCst), 1);
  assert!(!stage.paints("Buttons"), "choosing closes the menu");
}

#[test]
fn its_width_moves_between_the_sidebar_and_the_rail() {
  let collapsed = Signal::new(false);
  let held = collapsed.clone();
  let mut stage = stage(move |ctx| layout(ctx, board().collapsed(held.clone())));
  assert_eq!(stage.bounds("sidebar").2, 260.0);
  collapsed.set(true);
  stage.pass_after(Duration::ZERO);
  stage.pass_after(Duration::from_millis(120));
  let width = stage.bounds("sidebar").2;
  assert!(width > 52.5 && width < 259.5, "on its way to the rail: {width}");
  stage.pass();
  assert_eq!(stage.bounds("sidebar").2, 52.0);
  collapsed.set(false);
  stage.pass();
  assert_eq!(stage.bounds("sidebar").2, 260.0);
}

#[test]
fn the_trigger_collapses_it_and_brings_it_back() {
  let collapsed = Signal::new(false);
  let held = collapsed.clone();
  let mut stage = stage(move |ctx| {
    Row::new()
      .size(1024.0, 600.0)
      .child(board().collapsed(held.clone()).mount(ctx))
      .child(
        Column::new()
          .flex(1.0)
          .child(SidebarTrigger::new(held.clone()).narrow(false).id("trigger")),
      )
      .into()
  });
  stage.click("trigger");
  assert!(collapsed.get());
  stage.pass();
  assert_eq!(stage.bounds("sidebar").2, 52.0);
  stage.click("trigger");
  assert!(!collapsed.get());
}

#[test]
fn on_a_narrow_window_it_lies_over_the_page_and_closes_when_an_item_is_chosen() {
  let open = Signal::new(true);
  let held = open.clone();
  let mut stage = stage(move |ctx| {
    layout(
      ctx,
      board()
        .narrow(true)
        .open(held.clone())
        .item(SidebarItem::new("Archive").id("archive").on_select(|| {})),
    )
  });
  assert_eq!(stage.bounds("page").0, 0.0, "it takes no room beside the page");
  assert!(
    stage
      .quads()
      .iter()
      .any(|quad| { matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Scrim)) }),
    "over a veil"
  );
  let (x, _, width, _) = stage.bounds("sidebar");
  assert!(
    close(x + width, 260.0),
    "as wide as beside the page: ends at {}",
    x + width
  );
  // The press is the item's own: the panel may still be sliding in on the process's clock.
  stage.tree.get_element_by_id_mut("archive").unwrap().click();
  stage.pass();
  assert!(!open.get(), "choosing an item closes it");
  stage.pass();
  assert!(!stage.paints("Archive"));
}

#[test]
fn on_a_narrow_window_the_trigger_opens_it_over_the_page() {
  let (collapsed, open) = (Signal::new(false), Signal::new(false));
  let (held, shown) = (collapsed.clone(), open.clone());
  let mut stage = stage(move |ctx| {
    Row::new()
      .size(1024.0, 600.0)
      .child(
        board()
          .narrow(true)
          .collapsed(held.clone())
          .open(shown.clone())
          .mount(ctx),
      )
      .child(
        SidebarTrigger::new(held.clone())
          .open(shown.clone())
          .narrow(true)
          .id("trigger"),
      )
      .into()
  });
  assert!(!stage.paints("Overview"), "closed over the page, it paints nothing");
  stage.click("trigger");
  assert!(open.get() && !collapsed.get());
  stage.pass();
  assert!(stage.paints("Overview"));
}

#[test]
fn its_items_scroll_and_its_header_and_footer_stay() {
  let mut stage = stage(|ctx| {
    let mut sidebar = Sidebar::new()
      .id("sidebar")
      .narrow(false)
      .header(SidebarHeading::new("lurq").icon(icons::BOX).id("heading"))
      .footer(Column::new().id("footer").child(Text::new("Ivan Zaida")));
    for index in 0..40 {
      sidebar = sidebar.item(SidebarItem::new(&format!("Page {index}")).id(format!("page-{index}")));
    }
    Row::new()
      .size(1024.0, 300.0)
      .child(sidebar.mount(ctx))
      .child(page())
      .into()
  });
  assert_eq!(
    stage.bounds("sidebar").3,
    300.0,
    "as high as the window, not as its items"
  );
  // The bar is drawn over what scrolls, at the sidebar's edge: the rows end before its lane.
  let bar = stage.app.theme().scrollbar().clone();
  let lane = bar.width + bar.resolved_edge_inset() * 2.0;
  let (x, _, width, _) = stage.bounds("page-0");
  assert!(
    x + width <= 260.0 - lane + 0.01,
    "the row ends at {}, the bar's lane starts at {}",
    x + width,
    260.0 - lane
  );
  let (_, y, _, height) = stage.bounds("heading");
  assert_eq!((y, height), (8.0, 44.0), "the header at the top");
  let (_, y, _, height) = stage.bounds("footer");
  assert!(
    close(y + height, 300.0 - 8.0 - 2.0),
    "the footer at the bottom: ends at {}",
    y + height
  );
  assert!(
    !stage.paints("Page 39"),
    "what is past the height is scrolled out of sight"
  );
}

#[test]
fn over_the_page_its_header_and_footer_stay_while_its_items_scroll() {
  let mut stage = stage(|ctx| {
    let mut sidebar = Sidebar::new()
      .id("sidebar")
      .narrow(true)
      .open(Signal::new(true))
      .header(SidebarHeading::new("lurq").icon(icons::BOX).id("heading"))
      .footer(Column::new().id("footer").child(Text::new("Ivan Zaida")));
    for index in 0..40 {
      sidebar = sidebar.item(SidebarItem::new(&format!("Page {index}")));
    }
    Row::new()
      .size(1024.0, 300.0)
      .child(sidebar.mount(ctx))
      .child(page())
      .into()
  });
  stage.tree.resize(1024, 300);
  stage.pass();
  assert_eq!(
    stage.bounds("sidebar").3,
    300.0,
    "as high as the drawer, not as its items"
  );
  let (_, y, _, height) = stage.bounds("heading");
  assert_eq!((y, height), (8.0, 44.0), "the header at the top");
  let (_, y, _, height) = stage.bounds("footer");
  assert!(
    close(y + height, 300.0 - 8.0 - 2.0),
    "the footer at the bottom: ends at {}",
    y + height
  );
  assert!(
    !stage.paints("Page 39"),
    "what is past the height is scrolled out of sight"
  );
}

#[test]
fn a_heading_has_its_mark_of_28_on_the_primary_fill() {
  let mut stage = stage(|ctx| layout(ctx, board()));
  let (_, _, _, height) = stage.bounds("heading");
  assert_eq!(height, 44.0);
  assert_eq!(stage.text("lurq"), (light(PaletteColor::Foreground), 13.0));
  assert_eq!(stage.text("Workspace").0, light(PaletteColor::MutedForeground));
  assert!(
    stage.quads().iter().any(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Primary))
        && (quad.width, quad.height) == (28.0, 28.0)
    }),
    "the mark"
  );
}

#[test]
fn a_theme_restyles_its_parts() {
  let app = App::new();
  app
    .theme()
    .patch_recipe::<Sidebar>(|rules| rules.base(SidebarPart::Root, |s| s.background(PaletteColor::Canvas)));
  let mut stage = stage_in(app, |ctx| layout(ctx, board()));
  assert_eq!(stage.fill_color("sidebar"), Some(light(PaletteColor::Canvas)));
}
