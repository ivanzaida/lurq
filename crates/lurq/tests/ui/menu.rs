//! A menu: its panel under its anchor, its items, separators and labels,
//! what choosing an item does, what closes it, and a theme's other anatomy.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    events::MouseButton,
    theme::{OpacityLevel, PaletteColor, ThemePalette},
  },
  components::{Column, Rect},
  core::{ElementRef, Signal},
  layout::quad::QuadContent,
  node::{Element, color::Color},
  ui::{
    Case, Interaction, Menu, MenuItem, MenuItemPart, MenuItemRenderer, MenuItemState, MenuItemView, Opacity, icons,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The character lurq's font has for an icon.
fn glyph(name: &str) -> String {
  icons::ALL
    .iter()
    .find_map(|(known, glyph)| (*known == name).then(|| glyph.to_string()))
    .unwrap_or_else(|| panic!("lurq's font has no icon {name:?}"))
}

/// A page of 400 by 300 with an anchor of 120 by 30 at 20, 20 and `menu` hung from it. The page has a size
/// of its own so that the popup has room under the anchor.
fn page(anchor: ElementRef, menu: Menu) -> Element {
  Column::new()
    .size(400.0, 300.0)
    .padding(20.0)
    .child(Rect::new(120.0, 30.0).id("anchor").ref_element(anchor))
    .child(menu)
    .into()
}

/// What was chosen, in order, and a handler for each item that says so.
fn log() -> (
  Arc<Mutex<Vec<&'static str>>>,
  impl Fn(&'static str) -> Box<dyn Fn() + Send + Sync>,
) {
  let chosen = Arc::new(Mutex::new(Vec::new()));
  let told = chosen.clone();
  (chosen, move |name| {
    let told = told.clone();
    Box::new(move || told.lock().unwrap().push(name))
  })
}

#[test]
fn an_open_menu_is_a_panel_under_its_anchor_with_the_items_the_board_draws() {
  let anchor = ElementRef::new();
  let mut stage = Stage::new(move |_| {
    page(
      anchor.clone(),
      Menu::new(anchor.clone(), true)
        .id("menu")
        .label("Edit")
        .item(MenuItem::new("Undo").icon(icons::UNDO).shortcut("Ctrl+Z").id("undo"))
        .separator()
        .item(MenuItem::new("Show grid").checked(true).id("grid"))
        .item(MenuItem::new("Snap to guides").checked(false).id("snap")),
    )
  });
  // Under the anchor from its start, 4 below it, at least 230 wide.
  let (ax, ay, _, ah) = stage.bounds("anchor");
  let (x, y, width, _) = stage.bounds("menu");
  assert_eq!((x, y), (ax, ay + ah + 4.0));
  assert_eq!(width, 230.0);
  assert_eq!(stage.fill_color("menu"), Some(light(PaletteColor::Popover)));
  let edge = stage
    .border("menu")
    .and_then(|quad| quad.border)
    .and_then(|border| border.top);
  assert_eq!(
    edge.map(|side| (side.color, side.width)),
    Some((light(PaletteColor::Border), 1.0))
  );

  // An item is 32 tall and as wide as the panel inside its padding of 4.
  let (ix, iy, iw, ih) = stage.bounds("undo");
  assert_eq!((ix - x, iw, ih), (4.0, 222.0, 32.0));
  let label = stage.text_quad("Undo").unwrap();
  assert_eq!(
    label.x - ix,
    8.0 + 16.0 + 8.0,
    "after its icon of 16, 8 from the edge and 8 from the icon"
  );
  assert_eq!(stage.text("Undo"), (light(PaletteColor::Foreground), 14.0));
  assert_eq!(
    stage.text(&glyph(icons::UNDO)),
    (light(PaletteColor::MutedForeground), 16.0)
  );
  // The shortcut is at the end, 8 from the edge, in the mono type at 11.
  let shortcut = stage.text_quad("Ctrl+Z").unwrap();
  assert!(
    (ix + iw - 8.0 - (shortcut.x + shortcut.width)).abs() < 0.5,
    "at the end"
  );
  assert_eq!(stage.text("Ctrl+Z"), (light(PaletteColor::MutedForeground), 11.0));

  // The label of the group is 11 in the muted foreground; it stands over the first item.
  assert_eq!(stage.text("Edit"), (light(PaletteColor::MutedForeground), 11.0));
  assert!(stage.text_quad("Edit").unwrap().y < iy);

  // The separator is a line of 1 in the border's color, 4 under the item before it and 4 over the next.
  let (_, gy, ..) = stage.bounds("grid");
  let line = stage
    .quads()
    .into_iter()
    .find(|quad| {
      quad.height == 1.0
        && matches!(quad.content, QuadContent::Rect { color, .. } if color == light(PaletteColor::Border))
    })
    .expect("a separator");
  assert_eq!((line.y - (iy + ih), gy - (line.y + 1.0)), (4.0, 4.0));
  assert_eq!(line.width, 222.0);

  // A checked item has the check at its end; one that is not checked has nothing there.
  let checks = stage
    .quads()
    .iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if *text == glyph(icons::CHECK)))
    .count();
  assert_eq!(checks, 1);
  assert_eq!(stage.text(&glyph(icons::CHECK)).1, 16.0);
}

#[test]
fn a_closed_menu_paints_nothing() {
  let anchor = ElementRef::new();
  let stage = Stage::new(move |_| {
    page(
      anchor.clone(),
      Menu::new(anchor.clone(), Signal::new(false)).item(MenuItem::new("Rename")),
    )
  });
  assert!(!stage.paints("Rename"));
}

#[test]
fn choosing_an_item_calls_its_handler_and_closes_the_menu() {
  let (chosen, tell) = log();
  let open = Signal::new(true);
  let (anchor, held) = (ElementRef::new(), open.clone());
  let mut stage = Stage::new(move |_| {
    page(
      anchor.clone(),
      Menu::new(anchor.clone(), held.clone())
        .item(MenuItem::new("Rename").id("rename").on_select(tell("rename")))
        .item(MenuItem::new("Redo").disabled(true).id("redo").on_select(tell("redo"))),
    )
  });
  // A disabled item takes nothing, and the press is inside the menu: it stays open.
  stage.click("redo");
  assert!(open.get());
  assert!(chosen.lock().unwrap().is_empty());

  stage.click("rename");
  assert_eq!(*chosen.lock().unwrap(), ["rename"]);
  assert!(!open.get(), "closed");
  stage.pass();
  assert!(!stage.paints("Rename"));
}

#[test]
fn escape_and_a_press_outside_close_the_menu() {
  let open = Signal::new(true);
  let (anchor, held) = (ElementRef::new(), open.clone());
  let mut stage = Stage::new(move |_| {
    page(
      anchor.clone(),
      Menu::new(anchor.clone(), held.clone()).item(MenuItem::new("Rename")),
    )
  });
  assert!(stage.paints("Rename"));
  stage.key("Escape");
  assert!(!open.get(), "Escape");

  open.set(true);
  stage.pass();
  assert!(stage.paints("Rename"));
  stage.tree.mouse_move(390.0, 290.0);
  stage.tree.mouse_down(390.0, 290.0, MouseButton::Left);
  stage.tree.mouse_up(390.0, 290.0, MouseButton::Left);
  stage.pass();
  assert!(!open.get(), "a press outside");
}

#[test]
fn tab_reaches_the_items_and_enter_chooses_the_focused_one() {
  let (chosen, tell) = log();
  let open = Signal::new(true);
  let (anchor, held) = (ElementRef::new(), open.clone());
  let mut stage = Stage::new(move |_| {
    page(
      anchor.clone(),
      Menu::new(anchor.clone(), held.clone())
        .item(MenuItem::new("Cut").id("cut").on_select(tell("cut")))
        .item(MenuItem::new("Copy").disabled(true).id("copy").on_select(tell("copy")))
        .item(MenuItem::new("Paste").id("paste").on_select(tell("paste"))),
    )
  });
  assert_eq!(stage.fill_color("cut"), None, "at rest an item has no fill");
  // Focused from the keyboard an item is highlighted: the board's accent.
  stage.key("Tab");
  assert_eq!(stage.fill_color("cut"), Some(light(PaletteColor::Highlight)));
  // The disabled one is passed by.
  stage.key("Tab");
  assert_eq!(stage.fill_color("paste"), Some(light(PaletteColor::Highlight)));
  stage.key("Enter");
  assert_eq!(*chosen.lock().unwrap(), ["paste"]);
  assert!(!open.get());
}

#[test]
fn an_item_under_the_pointer_is_highlighted() {
  let anchor = ElementRef::new();
  let mut stage = Stage::new(move |_| {
    page(
      anchor.clone(),
      Menu::new(anchor.clone(), true)
        .item(MenuItem::new("Profile").id("profile"))
        .item(MenuItem::new("Settings").force(Some(Interaction::Hovered)).id("forced")),
    )
  });
  assert_eq!(
    stage.fill_color("forced"),
    Some(light(PaletteColor::Highlight)),
    "forced"
  );
  assert_eq!(stage.fill_color("profile"), None);
  stage.hover("profile");
  assert_eq!(stage.fill_color("profile"), Some(light(PaletteColor::Highlight)));
}

#[test]
fn a_destructive_item_is_in_the_destructive_color_and_a_disabled_one_is_dimmed() {
  let anchor = ElementRef::new();
  let stage = Stage::new(move |_| {
    page(
      anchor.clone(),
      Menu::new(anchor.clone(), true).item(MenuItem::new("Log out").icon(icons::SIGN_OUT).destructive(true)),
    )
  });
  assert_eq!(stage.text("Log out"), (light(PaletteColor::Destructive), 14.0));
  assert_eq!(
    stage.text(&glyph(icons::SIGN_OUT)).0,
    light(PaletteColor::Destructive),
    "its icon too"
  );

  // The item is a layer of what is in it, so the opacity is the layer's and not a quad's.
  let theme = stage.app.theme().clone();
  let case = Case::new().state(MenuItemState::Disabled, true);
  let root = theme.resolve::<MenuItem>(&case, None).part(MenuItemPart::Root).rest();
  assert_eq!(root.opacity, Some(Opacity::Level(OpacityLevel::Disabled)));
}

#[test]
fn an_inline_menu_is_drawn_in_place() {
  let anchor = ElementRef::new();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(10.0)
      .child(
        Menu::new(anchor.clone(), false)
          .inline(true)
          .id("menu")
          .item(MenuItem::new("Fit to window")),
      )
      .into()
  });
  assert!(stage.paints("Fit to window"), "open or not");
  let (x, y, ..) = stage.bounds("menu");
  assert_eq!((x, y), (10.0, 10.0));
}

#[test]
fn a_theme_gives_the_item_another_anatomy_and_it_still_closes_the_menu() {
  let (chosen, tell) = log();
  let app = App::new();
  // An item that writes its shortcut before its label: other nodes, the same behavior.
  let renderer: MenuItemRenderer = Arc::new(|view: MenuItemView| {
    let mut root = view.root();
    if let Some(shortcut) = view.shortcut() {
      root = root.child(shortcut);
    }
    root.child(view.label()).into()
  });
  app.theme().set_renderer::<MenuItem>(renderer);
  let open = Signal::new(true);
  let (anchor, held) = (ElementRef::new(), open.clone());
  let mut stage = Stage::in_app(app, move |_| {
    page(
      anchor.clone(),
      Menu::new(anchor.clone(), held.clone()).item(
        MenuItem::new("Save")
          .shortcut("Ctrl+S")
          .id("save")
          .on_select(tell("save")),
      ),
    )
  });
  assert!(stage.text_quad("Ctrl+S").unwrap().x < stage.text_quad("Save").unwrap().x);
  stage.click("save");
  assert_eq!(*chosen.lock().unwrap(), ["save"]);
  assert!(!open.get());
}
