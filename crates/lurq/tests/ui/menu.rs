//! A menu: its panel under its anchor, its items, separators and labels,
//! what choosing an item does, what closes it, and a theme's other anatomy.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    events::MouseButton,
    theme::{OpacityLevel, PaletteColor, ThemePalette},
  },
  components::{Column, Rect, Row},
  core::{ElementRef, Signal},
  layout::quad::QuadContent,
  node::{Element, color::Color},
  ui::{
    Button, Case, Interaction, Menu, MenuItem, MenuItemPart, MenuItemRenderer, MenuItemState, MenuItemView, MenuPart,
    Opacity, icons,
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
  // Under the anchor from its start, 4 below it, and 230 wide: the board's width, which its items are narrower
  // than. The page and the window are 400 wide; a menu is never as wide as the room it is given.
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
fn tab_reaches_the_menu_once_and_the_arrows_move_between_its_items() {
  let (chosen, tell) = log();
  let open = Signal::new(true);
  let (anchor, held) = (ElementRef::new(), open.clone());
  let mut stage = Stage::new(move |_| {
    page(
      anchor.clone(),
      Menu::new(anchor.clone(), held.clone())
        .item(MenuItem::new("Cut").id("cut").on_select(tell("cut")))
        .item(MenuItem::new("Copy").disabled(true).id("copy").on_select(tell("copy")))
        .separator()
        .item(MenuItem::new("Paste").id("paste").on_select(tell("paste")))
        .item(MenuItem::new("Select all").id("all").on_select(tell("all"))),
    )
  });
  let highlighted = |stage: &mut Stage| {
    ["cut", "copy", "paste", "all"]
      .into_iter()
      .filter(|id| stage.fill_color(id) == Some(light(PaletteColor::Highlight)))
      .collect::<Vec<_>>()
  };
  assert!(highlighted(&mut stage).is_empty(), "at rest an item has no fill");
  // Tab reaches the menu at its first item, which the keyboard's focus highlights: the board's accent.
  stage.key("Tab");
  assert_eq!(highlighted(&mut stage), ["cut"]);
  // Down passes the disabled item and the separator by; Up goes back; End and Home go to the ends.
  stage.key("ArrowDown");
  assert_eq!(highlighted(&mut stage), ["paste"]);
  stage.key("ArrowUp");
  assert_eq!(highlighted(&mut stage), ["cut"]);
  stage.key("End");
  assert_eq!(highlighted(&mut stage), ["all"]);
  stage.key("Home");
  assert_eq!(highlighted(&mut stage), ["cut"]);
  // From the last it goes round to the first.
  stage.key("ArrowUp");
  assert_eq!(highlighted(&mut stage), ["all"]);
  // The arrows only move the focus: nothing is chosen until Enter.
  assert!(chosen.lock().unwrap().is_empty());
  stage.key("ArrowUp");
  stage.key("Enter");
  assert_eq!(*chosen.lock().unwrap(), ["paste"]);
  assert!(!open.get());
}

#[test]
fn a_chosen_radio_item_has_a_dot_at_its_end() {
  let (chosen, tell) = log();
  let anchor = ElementRef::new();
  let mut stage = Stage::new(move |_| {
    page(
      anchor.clone(),
      Menu::new(anchor.clone(), true)
        .label("Zoom")
        .item(MenuItem::new("50%").radio(false).id("half").on_select(tell("50%")))
        .item(MenuItem::new("100%").radio(true).id("actual"))
        .item(MenuItem::new("Show grid").checked(true)),
    )
  });
  let painted = |stage: &Stage, glyph: &str| {
    stage
      .quads()
      .iter()
      .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == glyph))
      .count()
  };
  let (dot, check) = (glyph(icons::DOT_CIRCLE), glyph(icons::CHECK));
  assert_eq!((painted(&stage, &dot), painted(&stage, &check)), (1, 1));
  assert_eq!(stage.text(&dot), (light(PaletteColor::Foreground), 16.0));
  // The dot is at the end of the chosen one, 8 from its edge.
  let (x, y, width, height) = stage.bounds("actual");
  let mark = stage.text_quad(&dot).unwrap();
  assert!(mark.y >= y && mark.y + mark.height <= y + height);
  assert!((x + width - 8.0 - (mark.x + mark.width)).abs() < 0.5);
  // Choosing another tells the app, which says which is chosen.
  stage.click("half");
  assert_eq!(*chosen.lock().unwrap(), ["50%"]);
}

#[test]
fn a_submenu_opens_beside_its_item_and_choosing_in_it_closes_both() {
  let (chosen, tell) = log();
  let (open, exporting) = (Signal::new(true), Signal::new(false));
  let (anchor, export) = (ElementRef::new(), ElementRef::new());
  let (held, sub) = (open.clone(), exporting.clone());
  let mut stage = Stage::new(move |_| {
    page(
      anchor.clone(),
      Menu::new(anchor.clone(), held.clone())
        .style(MenuPart::Panel, |s| s.min_width(160.0))
        .id("menu")
        .item(MenuItem::new("Undo").id("undo"))
        .item(
          MenuItem::new("Export as").id("export").submenu(
            Menu::new(export.clone(), sub.clone())
              .id("formats")
              .item(MenuItem::new("PNG").id("png").on_select(tell("png")))
              .item(MenuItem::new("SVG")),
          ),
        ),
    )
  });
  // Closed, the item has a chevron at its end, muted.
  assert!(!stage.paints("PNG"));
  assert_eq!(
    stage.text(&glyph(icons::CHEVRON_RIGHT)),
    (light(PaletteColor::MutedForeground), 16.0)
  );

  // A press of the item opens it, and the item stays highlighted while it is open.
  stage.click("export");
  assert!(exporting.get());
  assert!(stage.paints("PNG"));
  assert_eq!(stage.fill_color("export"), Some(light(PaletteColor::Highlight)));
  // Beside the menu, 4 from it; its first item level with the item it hangs from.
  let (mx, _, mw, _) = stage.bounds("menu");
  let (sx, ..) = stage.bounds("formats");
  assert_eq!(sx, mx + mw + 4.0);
  let (_, ey, ..) = stage.bounds("export");
  let (_, py, ..) = stage.bounds("png");
  assert_eq!(py, ey);
  let (.., sw, _) = stage.bounds("formats");
  assert_eq!(sw, 160.0, "a submenu is at least 160 wide");

  // The pointer on another item of the menu closes it; on the item again, opens it.
  stage.hover("undo");
  assert!(!exporting.get());
  stage.hover("export");
  assert!(exporting.get());

  // Choosing in the submenu closes it and the menu.
  stage.click("png");
  assert_eq!(*chosen.lock().unwrap(), ["png"]);
  assert!(!exporting.get() && !open.get());
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

/// A page with a button that opens and closes `open`, and the menu of Cut, Copy and Paste hung from it.
fn with_button(
  open: Signal<bool>,
  tell: impl Fn(&'static str) -> Box<dyn Fn() + Send + Sync> + Send + Sync + 'static,
) -> Stage {
  let anchor = ElementRef::new();
  Stage::new(move |_| {
    let toggle = open.clone();
    Column::new()
      .size(400.0, 300.0)
      .padding(20.0)
      .child(
        Row::new().ref_element(anchor.clone()).child(
          Button::new("Edit")
            .id("button")
            .on_click(move |_| toggle.update(|open| *open = !*open)),
        ),
      )
      .child(
        Menu::new(anchor.clone(), open.clone())
          .item(MenuItem::new("Cut").id("cut").on_select(tell("cut")))
          .item(MenuItem::new("Copy").id("copy").on_select(tell("copy")))
          .item(MenuItem::new("Paste").id("paste").on_select(tell("paste"))),
      )
      .into()
  })
}

#[test]
fn opened_from_the_keyboard_the_menu_has_its_first_item_focused_and_gives_the_focus_back() {
  let (chosen, tell) = log();
  let open = Signal::new(false);
  let mut stage = with_button(open.clone(), tell);
  let highlighted = |stage: &mut Stage| {
    ["cut", "copy", "paste"]
      .into_iter()
      .filter(|id| stage.fill_color(id) == Some(light(PaletteColor::Highlight)))
      .collect::<Vec<_>>()
  };
  // Tab reaches the button and Enter opens the menu: its first item has the focus, and shows it.
  stage.key("Tab");
  stage.key("Enter");
  assert!(open.get());
  assert_eq!(highlighted(&mut stage), ["cut"]);
  stage.key("ArrowDown");
  assert_eq!(highlighted(&mut stage), ["copy"]);
  // Escape closes it, and the focus is back on the button: Enter opens the menu again, at its first item.
  stage.key("Escape");
  assert!(!open.get());
  stage.key("Enter");
  assert!(open.get(), "the button had the focus back");
  assert_eq!(highlighted(&mut stage), ["cut"]);
  // Choosing an item gives the focus back too.
  stage.key("ArrowDown");
  stage.key("ArrowDown");
  stage.key("Enter");
  assert_eq!(*chosen.lock().unwrap(), ["paste"]);
  assert!(!open.get());
  stage.key("Enter");
  assert!(open.get(), "the button had the focus back");
}

#[test]
fn opened_by_the_pointer_the_menu_shows_no_focus_until_a_key() {
  let (_, tell) = log();
  let open = Signal::new(false);
  let mut stage = with_button(open.clone(), tell);
  stage.click("button");
  assert!(open.get());
  for id in ["cut", "copy", "paste"] {
    assert_eq!(stage.fill_color(id), None, "{id}");
  }
  // The focus is on the first item all the same: Down goes on from it.
  stage.key("ArrowDown");
  assert_eq!(stage.fill_color("copy"), Some(light(PaletteColor::Highlight)));
}

#[test]
fn right_opens_a_submenu_at_its_first_item_and_left_or_escape_go_back() {
  let (chosen, tell) = log();
  let (open, exporting) = (Signal::new(true), Signal::new(false));
  let (anchor, export) = (ElementRef::new(), ElementRef::new());
  let (held, sub) = (open.clone(), exporting.clone());
  let mut stage = Stage::new(move |_| {
    page(
      anchor.clone(),
      Menu::new(anchor.clone(), held.clone())
        .style(MenuPart::Panel, |s| s.min_width(160.0))
        .item(MenuItem::new("Undo").id("undo"))
        .item(
          MenuItem::new("Export as").id("export").submenu(
            Menu::new(export.clone(), sub.clone())
              .item(MenuItem::new("PNG").id("png").on_select(tell("png")))
              .item(MenuItem::new("SVG").id("svg")),
          ),
        ),
    )
  });
  let lit = |stage: &mut Stage, id: &str| stage.fill_color(id) == Some(light(PaletteColor::Highlight));
  stage.key("Tab");
  stage.key("ArrowDown");
  assert!(lit(&mut stage, "export"));
  // Right on an item with a submenu opens it, with the focus on its first item.
  stage.key("ArrowRight");
  assert!(exporting.get());
  assert!(lit(&mut stage, "png"));
  // Down moves within the submenu, not the menu.
  stage.key("ArrowDown");
  assert!(lit(&mut stage, "svg"));
  // Left closes it, and the focus is back on its item, whose submenu is closed.
  stage.key("ArrowLeft");
  assert!(!exporting.get() && open.get());
  stage.key("ArrowUp");
  assert!(lit(&mut stage, "undo"), "the focus was on Export as");
  // Right on an item without a submenu does nothing.
  stage.key("ArrowRight");
  assert!(!exporting.get());
  // Escape in the submenu closes it alone.
  stage.key("ArrowDown");
  stage.key("ArrowRight");
  assert!(exporting.get());
  stage.key("Escape");
  assert!(!exporting.get() && open.get());
  // Enter in the submenu chooses, and closes both.
  stage.key("ArrowRight");
  stage.key("Enter");
  assert_eq!(*chosen.lock().unwrap(), ["png"]);
  assert!(!exporting.get() && !open.get());
}

#[test]
fn the_focus_follows_the_pointer_so_one_item_is_highlighted() {
  let (_, tell) = log();
  let open = Signal::new(false);
  let mut stage = with_button(open.clone(), tell);
  let highlighted = |stage: &mut Stage| {
    ["cut", "copy", "paste"]
      .into_iter()
      .filter(|id| stage.fill_color(id) == Some(light(PaletteColor::Highlight)))
      .collect::<Vec<_>>()
  };
  // Opened from the keyboard, at its first item.
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!(highlighted(&mut stage), ["cut"]);
  // The pointer onto the last item takes the focus there: one item is highlighted, not two.
  stage.hover("paste");
  assert_eq!(highlighted(&mut stage), ["paste"]);
  // The keyboard goes on from where the pointer is, and what it moves to is the one item lit: the item
  // under a pointer that holds still is not.
  stage.key("ArrowUp");
  assert_eq!(highlighted(&mut stage), ["copy"]);
  // The pointer moving again takes the focus back to what it is on.
  stage.hover("paste");
  assert_eq!(highlighted(&mut stage), ["paste"]);
}

#[test]
fn a_menu_is_as_wide_as_its_widest_item_and_never_as_its_window() {
  let anchor = ElementRef::new();
  let mut stage = Stage::new(move |_| {
    page(
      anchor.clone(),
      Menu::new(anchor.clone(), true)
        .id("menu")
        .item(MenuItem::new("Undo").id("undo"))
        .item(
          MenuItem::new("Show the hidden files")
            .shortcut("Ctrl+Shift+H")
            .id("hidden"),
        ),
    )
  });
  // The window (and the page) is 400 wide: the menu is narrower, as wide as its widest item and its padding.
  let (_, _, width, _) = stage.bounds("menu");
  assert!(width > 230.0 && width < 300.0, "{width}");
  let (x, _, item, _) = stage.bounds("hidden");
  assert_eq!(item, width - 8.0);
  // Its widest item holds its label and its shortcut 8 apart, 8 from its edges.
  let (label, keys) = (
    stage.text_quad("Show the hidden files").unwrap(),
    stage.text_quad("Ctrl+Shift+H").unwrap(),
  );
  assert!((label.x - (x + 8.0)).abs() < 0.5);
  assert!(
    (keys.x - (label.x + label.width) - 8.0).abs() < 0.5,
    "{} {}",
    label.x + label.width,
    keys.x
  );
  assert!((x + item - 8.0 - (keys.x + keys.width)).abs() < 0.5);
  // Every item is as wide as the widest.
  assert_eq!(stage.bounds("undo").2, item);
}
