//! A menu: its panel under its anchor, its items, separators and labels,
//! what choosing an item does, what closes it, and a theme's other anatomy.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    events::MouseButton,
    theme::{OpacityLevel, PaletteColor, ThemePalette},
  },
  components::{Column, Placement, Rect, Row},
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
      Menu::controlled(true, anchor.clone(), |_| {})
        .id("menu")
        .heading("Edit")
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
      Menu::new(Signal::new(false), anchor.clone()).item(MenuItem::new("Rename")),
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
      Menu::new(held.clone(), anchor.clone())
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
      Menu::new(held.clone(), anchor.clone()).item(MenuItem::new("Rename")),
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
      Menu::new(held.clone(), anchor.clone())
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
  // The menu put the focus on its first item as it opened; a key shows it there, in the board's accent. Tab would
  // close the menu.
  stage.key("Home");
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
      Menu::controlled(true, anchor.clone(), |_| {})
        .heading("Zoom")
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
      Menu::new(held.clone(), anchor.clone())
        .style(MenuPart::Panel, |s| s.min_width(160.0))
        .id("menu")
        .item(MenuItem::new("Undo").id("undo"))
        .item(
          MenuItem::new("Export as").id("export").submenu(
            Menu::new(sub.clone(), export.clone())
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
      Menu::controlled(true, anchor.clone(), |_| {})
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
      Menu::controlled(true, anchor.clone(), |_| {})
        .item(MenuItem::new("Log out").icon(icons::SIGN_OUT).destructive(true)),
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
        Menu::controlled(false, anchor.clone(), |_| {})
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
      Menu::new(held.clone(), anchor.clone()).item(
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
        Menu::new(open.clone(), anchor.clone())
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
      Menu::new(held.clone(), anchor.clone())
        .style(MenuPart::Panel, |s| s.min_width(160.0))
        .item(MenuItem::new("Undo").id("undo"))
        .item(
          MenuItem::new("Export as").id("export").submenu(
            Menu::new(sub.clone(), export.clone())
              .item(MenuItem::new("PNG").id("png").on_select(tell("png")))
              .item(MenuItem::new("SVG").id("svg")),
          ),
        ),
    )
  });
  let lit = |stage: &mut Stage, id: &str| stage.fill_color(id) == Some(light(PaletteColor::Highlight));
  // The focus is on the first item, where the menu put it as it opened.
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
      Menu::controlled(true, anchor.clone(), |_| {})
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

#[test]
fn the_focus_goes_back_to_what_opened_the_menu_and_not_the_first_button_of_its_anchor() {
  let (chosen, tell) = log();
  let merged = Arc::new(Mutex::new(0));
  let open = Signal::new(false);
  let (anchor, held, presses) = (ElementRef::new(), open.clone(), merged.clone());
  // A split button: the anchor is the group, and its second button opens the menu.
  let mut stage = Stage::new(move |_| {
    let (toggle, presses) = (held.clone(), presses.clone());
    Column::new()
      .size(400.0, 300.0)
      .padding(20.0)
      .child(
        Row::new()
          .ref_element(anchor.clone())
          .child(Button::new("Merge").on_click(move |_| *presses.lock().unwrap() += 1))
          .child(
            Button::new("How to merge")
              .icon_only(true)
              .on_click(move |_| toggle.update(|open| *open = !*open)),
          ),
      )
      .child(
        Menu::new(held.clone(), anchor.clone())
          .placement(Placement::BottomEnd)
          .item(MenuItem::new("Squash and merge").on_select(tell("squash"))),
      )
      .into()
  });
  stage.key("Tab");
  stage.key("Tab");
  stage.key("Enter");
  assert!(open.get());
  stage.key("Escape");
  assert!(!open.get());
  // The focus is back on the button that opened the menu: Enter opens it again, and Merge was not pressed.
  stage.key("Enter");
  assert!(open.get());
  stage.key("Enter");
  assert_eq!(*chosen.lock().unwrap(), ["squash"]);
  stage.key("Enter");
  assert!(open.get(), "back on the button that opened it");
  assert_eq!(*merged.lock().unwrap(), 0);
}

/// Types `keys` one after another with no time between them: a word typed at speed. The stage's passes move its
/// clock on by seconds, so each `Stage::key` is a word of its own; these keys are pressed at a time later than any
/// pass of a test reaches, so that the word does not go on from the last of those.
fn type_quickly(stage: &mut Stage, keys: &[&str]) {
  let now = std::time::Instant::now() + std::time::Duration::from_secs(3600);
  for key in keys {
    stage.tree.set_animation_clock(Some(now));
    stage
      .tree
      .key_down(key.to_string(), key.to_string(), false, false, false);
    stage.tree.key_up(key.to_string(), key.to_string(), false, false, false);
  }
  stage.pass();
}

#[test]
fn typing_a_letter_goes_to_the_item_whose_label_starts_with_it() {
  let (chosen, tell) = log();
  let anchor = ElementRef::new();
  let mut stage = Stage::new(move |_| {
    page(
      anchor.clone(),
      Menu::controlled(true, anchor.clone(), |_| {})
        .item(MenuItem::new("Undo").shortcut("Ctrl+Z").id("undo"))
        .item(MenuItem::new("Redo").disabled(true).id("redo"))
        .separator()
        .item(MenuItem::new("Cut").id("cut").on_select(tell("cut")))
        .item(MenuItem::new("Copy").id("copy").on_select(tell("copy")))
        .item(MenuItem::new("Paste").icon(icons::PASTE).id("paste")),
    )
  });
  let lit = |stage: &mut Stage| {
    ["undo", "redo", "cut", "copy", "paste"]
      .into_iter()
      .filter(|id| stage.fill_color(id) == Some(light(PaletteColor::Highlight)))
      .collect::<Vec<_>>()
  };
  // A letter goes to the item whose label starts with it, lit as the keyboard lights it; an icon is no letter.
  stage.key("p");
  assert_eq!(lit(&mut stage), ["paste"]);
  // From the one after the focused, round the menu, past the disabled one.
  stage.key("r");
  assert_eq!(lit(&mut stage), ["paste"], "Redo is disabled");
  stage.key("c");
  assert_eq!(lit(&mut stage), ["cut"]);
  // The label, not the shortcut: Undo's text has Ctrl+Z in it.
  stage.key("u");
  assert_eq!(lit(&mut stage), ["undo"]);
  stage.key("z");
  assert_eq!(lit(&mut stage), ["undo"]);
  // Letters typed at speed are one prefix: "c", "o" goes past Cut to Copy, and Enter chooses it.
  type_quickly(&mut stage, &["c", "o"]);
  assert_eq!(lit(&mut stage), ["copy"]);
  stage.key("Enter");
  assert_eq!(*chosen.lock().unwrap(), ["copy"]);
}

#[test]
fn an_items_height_follows_the_density() {
  use lurq::app::theme::Density;
  for (density, height) in [
    (Density::Compact, 24.0),
    (Density::Default, 32.0),
    (Density::Comfortable, 40.0),
    (Density::Touch, 44.0),
  ] {
    let app = App::new();
    app.theme().set_density(density);
    let anchor = ElementRef::new();
    let mut stage = Stage::in_app(app, move |_| {
      page(
        anchor.clone(),
        Menu::controlled(true, anchor.clone(), |_| {}).item(MenuItem::new("Undo").id("undo")),
      )
    });
    assert_eq!(stage.bounds("undo").3, height, "{density:?}");
  }
}

#[test]
fn a_submenu_opens_toward_its_chevron_in_a_window_that_lays_its_page_out_at_its_own_size() {
  let (open, exporting) = (Signal::new(false), Signal::new(false));
  let (anchor, export) = (ElementRef::new(), ElementRef::new());
  let (held, sub) = (open.clone(), exporting.clone());
  let mut stage = Stage::new(move |_| {
    // A window of 900 with the anchor in a narrow scroll view at its start, a side bar: the menu runs past the
    // scroll view, and the submenu has all the window's room beside it.
    Row::new()
      .size(900.0, 300.0)
      .child(
        lurq::components::ScrollBoth::new(
          Column::new()
            .width(260.0)
            .padding_left(120.0)
            .padding_top(20.0)
            .child(Rect::new(120.0, 30.0).id("anchor").ref_element(anchor.clone()))
            .child(
              Menu::new(held.clone(), anchor.clone())
                .id("menu")
                .heading("Edit")
                .item(MenuItem::new("Undo").id("undo").shortcut("Ctrl+Z"))
                .item(
                  MenuItem::new("Export as").id("export").submenu(
                    Menu::new(sub.clone(), export.clone())
                      .id("formats")
                      .item(MenuItem::new("PNG").id("png")),
                  ),
                )
                .separator()
                .item(MenuItem::new("Delete").destructive(true)),
            ),
        )
        .size(260.0, 300.0),
      )
      .into()
  });
  // A window lays its page out at its own size, as the stage's loose 400 by 300 does not: a layer laid out on its
  // own at that size had its items as wide as the window, and the submenu, placed by its item, no room beside it.
  stage.tree.resize(900, 300);
  stage
    .tree
    .set_layout_constraints_override(Some(lurq::layout::Constraints::tight(lurq::layout::Size::new(
      900.0, 300.0,
    ))));
  stage.pass();
  open.set(true);
  stage.pass();
  stage.hover("export");
  assert!(exporting.get());
  let (mx, _, mw, _) = stage.bounds("menu");
  let (sx, ..) = stage.bounds("formats");
  assert!(mx + mw > 260.0, "the menu runs past the scroll view: {mx} {mw}");
  assert_eq!(
    sx,
    mx + mw + 4.0,
    "the submenu is beside the menu on the side of its chevron"
  );
}

#[test]
fn choosing_an_item_of_a_submenu_gives_the_focus_back_to_what_opened_the_menu() {
  let (chosen, tell) = log();
  let (open, exporting) = (Signal::new(false), Signal::new(false));
  let (anchor, export) = (ElementRef::new(), ElementRef::new());
  let (held, sub) = (open.clone(), exporting.clone());
  let mut stage = Stage::new(move |_| {
    let toggle = held.clone();
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
        Menu::new(held.clone(), anchor.clone())
          .item(MenuItem::new("Undo").id("undo"))
          .item(
            MenuItem::new("Export as").id("export").submenu(
              Menu::new(sub.clone(), export.clone())
                .item(MenuItem::new("PNG").id("png").on_select(tell("png")))
                .item(MenuItem::new("SVG").id("svg").on_select(tell("svg"))),
            ),
          ),
      )
      .into()
  });
  let focused = |stage: &Stage| {
    stage
      .tree
      .focused_element()
      .and_then(|element| element.id().map(str::to_owned))
  };
  // From the keyboard.
  stage.key("Tab");
  stage.key("Enter");
  stage.key("ArrowDown");
  stage.key("ArrowRight");
  assert!(exporting.get());
  stage.key("Enter");
  assert_eq!(*chosen.lock().unwrap(), ["png"]);
  assert!(!exporting.get() && !open.get());
  assert_eq!(focused(&stage).as_deref(), Some("button"));
  // By the pointer.
  stage.click("button");
  stage.hover("export");
  assert!(exporting.get());
  stage.click("svg");
  assert_eq!(*chosen.lock().unwrap(), ["png", "svg"]);
  assert!(!exporting.get() && !open.get());
  assert_eq!(focused(&stage).as_deref(), Some("button"));
}

#[test]
fn a_long_label_gives_way_with_an_ellipsis_and_the_shortcut_keeps_its_room_and_its_gap() {
  const LONG: &str = "Show the hidden files of every folder of the workspace and of the folders that are in them too";
  let anchor = ElementRef::new();
  let mut stage = Stage::new(move |_| {
    page(
      anchor.clone(),
      Menu::controlled(true, anchor.clone(), |_| {})
        .id("menu")
        .item(MenuItem::new(LONG).shortcut("Ctrl+Shift+H").id("long"))
        .item(
          MenuItem::new("Undo")
            .shortcut("Ctrl+Alt+Shift+Meta+Z, then Ctrl+Y")
            .id("keys"),
        ),
    )
  });
  // The panel stays in the window.
  let (mx, _, mw, _) = stage.bounds("menu");
  assert!(mx >= 0.0 && mx + mw <= 400.0, "{mx} {mw}");
  let texts: Vec<(String, f32, f32)> = stage
    .quads()
    .into_iter()
    .filter_map(|quad| match &quad.content {
      QuadContent::Text { text, .. } => Some((text.to_string(), quad.x, quad.width)),
      _ => None,
    })
    .collect();
  let find = |start: &str| {
    texts
      .iter()
      .find(|(text, ..)| text.starts_with(start))
      .unwrap_or_else(|| panic!("no text {start:?} in {texts:?}"))
      .clone()
  };
  for (item, label, keys) in [("long", "Show the", "Ctrl+Shift+H"), ("keys", "Undo", "Ctrl+Alt")] {
    let (x, _, width, _) = stage.bounds(item);
    let (label_text, label_x, label_width) = find(label);
    let (keys_text, keys_x, keys_width) = find(keys);
    // The shortcut is whole, 8 from the item's end.
    assert!(
      (x + width - 8.0 - (keys_x + keys_width)).abs() < 0.5,
      "{item}: the shortcut {keys_text:?} ends at {} in an item that ends at {}",
      keys_x + keys_width,
      x + width
    );
    // The label ends 8 or more before it.
    assert!(
      label_x + label_width <= keys_x - 8.0 + 0.5,
      "{item}: the label {label_text:?} ends at {}, the shortcut starts at {keys_x}",
      label_x + label_width
    );
  }
  // The long label gives way with an ellipsis.
  assert!(find("Show the").0.ends_with('…'), "{:?}", find("Show the").0);
}

#[test]
fn tab_closes_the_menu_and_its_submenus_and_the_focus_goes_on_from_what_opened_it() {
  let (open, exporting) = (Signal::new(false), Signal::new(false));
  let (anchor, export) = (ElementRef::new(), ElementRef::new());
  let (held, sub) = (open.clone(), exporting.clone());
  let mut stage = Stage::new(move |_| {
    let toggle = held.clone();
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
      .child(Button::new("After").id("after"))
      .child(
        Menu::new(held.clone(), anchor.clone())
          .item(MenuItem::new("Undo").id("undo"))
          .item(
            MenuItem::new("Export as")
              .id("export")
              .submenu(Menu::new(sub.clone(), export.clone()).item(MenuItem::new("PNG").id("png"))),
          ),
      )
      .into()
  });
  let focused = |stage: &Stage| {
    stage
      .tree
      .focused_element()
      .and_then(|element| element.id().map(str::to_owned))
  };
  stage.key("Tab");
  stage.key("Enter");
  assert!(open.get());
  stage.key("Tab");
  assert!(!open.get(), "Tab closes the menu");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("after"),
    "and goes on from the button that opened it"
  );
  // From a submenu, both close.
  stage
    .tree
    .key_down("Tab".to_owned(), "Tab".to_owned(), true, false, false);
  stage
    .tree
    .key_up("Tab".to_owned(), "Tab".to_owned(), true, false, false);
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("button"));
  stage.key("Enter");
  stage.key("ArrowDown");
  stage.key("ArrowRight");
  assert!(exporting.get());
  stage.key("Tab");
  assert!(!open.get() && !exporting.get());
  assert_eq!(focused(&stage).as_deref(), Some("after"));
}

#[test]
fn a_menu_hung_from_a_button_with_a_tooltip_takes_the_focus_as_it_opens_from_the_keyboard() {
  let (chosen, tell) = log();
  let open = Signal::new(false);
  let (anchor, held) = (ElementRef::new(), open.clone());
  let mut stage = Stage::new(move |ctx| {
    let toggle = held.clone();
    // The button is the tooltip's anchor and the menu's: two layers hang from one element.
    let button = lurq::components::Button::new("Edit")
      .id("button")
      .tab_index(0)
      .ref_element(anchor.clone())
      .on_click(move |_| toggle.update(|open| *open = !*open));
    Column::new()
      .size(400.0, 300.0)
      .padding(40.0)
      .child(lurq::ui::Tooltip::new("Edit the selection").mount(ctx, button))
      .child(
        Menu::new(held.clone(), anchor.clone())
          .item(MenuItem::new("Cut").id("cut").on_select(tell("cut")))
          .item(MenuItem::new("Copy").id("copy").on_select(tell("copy"))),
      )
      .into()
  });
  let focused = |stage: &Stage| {
    stage
      .tree
      .focused_element()
      .and_then(|element| element.id().map(str::to_owned))
  };
  // Tab shows the tooltip at once, and Enter opens the menu with the tooltip's layer still on the button.
  stage.key("Tab");
  assert!(stage.paints("Edit the selection"));
  stage.key("Enter");
  assert!(open.get());
  assert_eq!(
    focused(&stage).as_deref(),
    Some("cut"),
    "the menu's first item has the focus"
  );
  stage.key("ArrowDown");
  stage.key("Enter");
  assert_eq!(*chosen.lock().unwrap(), ["copy"]);
  assert_eq!(focused(&stage).as_deref(), Some("button"));
}

/// A press of the left button at a point of the window, outside every menu of a test's page.
fn press_outside(stage: &mut Stage) {
  stage.tree.mouse_move(390.0, 290.0);
  stage.tree.mouse_down(390.0, 290.0, MouseButton::Left);
  stage.tree.mouse_up(390.0, 290.0, MouseButton::Left);
  stage.pass();
}

#[test]
fn choosing_an_item_closes_the_menu_and_its_submenu_first_and_then_calls_the_handler() {
  let (open, exporting) = (Signal::new(true), Signal::new(false));
  let (anchor, export) = (ElementRef::new(), ElementRef::new());
  let seen = Arc::new(Mutex::new(Vec::new()));
  let (held, sub, told) = (open.clone(), exporting.clone(), seen.clone());
  let mut stage = Stage::new(move |_| {
    let (on_cut, on_png) = (told.clone(), told.clone());
    let (cut_open, png_open, png_sub) = (held.clone(), held.clone(), sub.clone());
    page(
      anchor.clone(),
      Menu::new(held.clone(), anchor.clone())
        .item(MenuItem::new("Cut").id("cut").on_select(move || {
          on_cut.lock().unwrap().push(("cut", cut_open.get_untracked(), false));
        }))
        .item(
          MenuItem::new("Export as")
            .id("export")
            .submenu(
              Menu::new(sub.clone(), export.clone()).item(MenuItem::new("PNG").id("png").on_select(move || {
                on_png
                  .lock()
                  .unwrap()
                  .push(("png", png_open.get_untracked(), png_sub.get_untracked()));
              })),
            ),
        ),
    )
  });
  // The handler sees the menu closed already: one that opens a dialog keeps the focus the dialog takes.
  stage.click("cut");
  assert_eq!(*seen.lock().unwrap(), [("cut", false, false)]);
  open.set(true);
  stage.pass();
  stage.hover("export");
  assert!(exporting.get());
  stage.click("png");
  assert_eq!(
    seen.lock().unwrap()[1],
    ("png", false, false),
    "the submenu and its menu are closed"
  );
}

#[test]
fn on_dismiss_hears_a_person_close_the_menu_and_not_a_choice_or_the_app() {
  let open = Signal::new(true);
  let dismissed = Arc::new(Mutex::new(Vec::new()));
  let (anchor, held, told) = (ElementRef::new(), open.clone(), dismissed.clone());
  let mut stage = Stage::new(move |_| {
    let (first, second) = (told.clone(), told.clone());
    page(
      anchor.clone(),
      Menu::new(held.clone(), anchor.clone())
        .on_dismiss(move || first.lock().unwrap().push("first"))
        .on_dismiss(move || second.lock().unwrap().push("second"))
        .item(MenuItem::new("Cut").id("cut")),
    )
  });
  // Escape: both handlers, in order.
  stage.key("Escape");
  assert!(!open.get());
  assert_eq!(*dismissed.lock().unwrap(), ["first", "second"]);
  // A press outside.
  open.set(true);
  stage.pass();
  press_outside(&mut stage);
  assert!(!open.get());
  assert_eq!(dismissed.lock().unwrap().len(), 4);
  // A choice and the app closing it are not dismissals.
  open.set(true);
  stage.pass();
  stage.click("cut");
  assert!(!open.get());
  open.set(true);
  stage.pass();
  open.set(false);
  stage.pass();
  assert_eq!(dismissed.lock().unwrap().len(), 4);
}

#[test]
fn escape_and_a_press_outside_can_be_kept_from_closing_the_menu() {
  let open = Signal::new(true);
  let (anchor, held) = (ElementRef::new(), open.clone());
  let mut stage = Stage::new(move |_| {
    page(
      anchor.clone(),
      Menu::new(held.clone(), anchor.clone())
        .dismiss_on_escape(false)
        .dismiss_on_outside_press(false)
        .item(MenuItem::new("Cut").id("cut")),
    )
  });
  stage.key("Escape");
  assert!(open.get(), "Escape");
  press_outside(&mut stage);
  assert!(open.get(), "a press outside");
  assert!(stage.paints("Cut"));
}

#[test]
fn a_controlled_menu_shows_what_the_app_answers() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let answer = Signal::new(true);
  // The app answers a close by keeping it open while `keep` holds.
  let keep = Arc::new(Mutex::new(true));
  let (anchor, held, heard, kept) = (ElementRef::new(), answer.clone(), told.clone(), keep.clone());
  let mut stage = Stage::new(move |_| {
    let (heard, kept, set) = (heard.clone(), kept.clone(), held.clone());
    page(
      anchor.clone(),
      Menu::controlled(held.get(), anchor.clone(), move |open| {
        heard.lock().unwrap().push(open);
        if !*kept.lock().unwrap() {
          set.set(open);
        }
      })
      .item(MenuItem::new("Cut").id("cut")),
    )
  });
  assert!(stage.paints("Cut"));
  // Escape asks to close; the app keeps it open, and it stays.
  stage.key("Escape");
  stage.pass();
  assert_eq!(*told.lock().unwrap(), [false]);
  assert!(stage.paints("Cut"), "the app said open");
  // Now the app follows: a choice closes it.
  *keep.lock().unwrap() = false;
  stage.click("cut");
  stage.pass();
  assert_eq!(*told.lock().unwrap(), [false, false]);
  assert!(!answer.get());
  assert!(!stage.paints("Cut"));
}

#[cfg(feature = "mcp")]
#[test]
fn a_menus_label_is_its_name_and_a_heading_is_drawn_over_the_items_after_it() {
  let anchor = ElementRef::new();
  let mut stage = Stage::new(move |_| {
    page(
      anchor.clone(),
      Menu::controlled(true, anchor.clone(), |_| {})
        .id("menu")
        .label("Edit")
        .heading("Zoom")
        .item(MenuItem::new("Fit")),
    )
  });
  assert!(stage.paints("Zoom"));
  assert!(!stage.paints("Edit"), "the label is not drawn");
  let named = stage
    .tree
    .get_element_by_id_mut("menu")
    .and_then(|element| element.description("a11y_name"));
  assert_eq!(named.as_deref(), Some("Edit"));
}

#[test]
fn offset_is_how_far_the_panel_is_from_its_anchor_along_its_side() {
  let anchor = ElementRef::new();
  let mut stage = Stage::new(move |_| {
    page(
      anchor.clone(),
      Menu::controlled(true, anchor.clone(), |_| {})
        .id("menu")
        .offset(20.0)
        .collision(lurq::components::CollisionStrategy::None)
        .item(MenuItem::new("Fit")),
    )
  });
  // The anchor is 120 by 30 at 20, 20: the panel is under it from its start, 20 down in place of 4.
  let (x, y, ..) = stage.bounds("menu");
  assert_eq!((x, y), (20.0, 70.0));
  // Beside its anchor, it is that far along.
  let anchor = ElementRef::new();
  let mut stage = Stage::new(move |_| {
    page(
      anchor.clone(),
      Menu::controlled(true, anchor.clone(), |_| {})
        .id("menu")
        .placement(lurq::components::Placement::RightStart)
        .offset(10.0)
        .collision(lurq::components::CollisionStrategy::None)
        .item(MenuItem::new("Fit")),
    )
  });
  let (x, y, ..) = stage.bounds("menu");
  assert_eq!((x, y), (20.0 + 120.0 + 10.0, 20.0));
}

#[test]
fn a_selected_radio_item_is_in_the_state_selected_and_a_checked_one_in_checked() {
  let anchor = ElementRef::new();
  let mut stage = Stage::new(move |_| {
    page(
      anchor.clone(),
      Menu::controlled(true, anchor.clone(), |_| {})
        .item(
          MenuItem::new("100%")
            .radio(true)
            .id("zoom")
            .rules(
              &lurq::ui::Recipe::new().state(MenuItemState::Selected, MenuItemPart::Root, |s| {
                s.background(PaletteColor::Destructive)
              }),
            ),
        )
        .item(
          MenuItem::new("Grid")
            .checked(true)
            .id("grid")
            .rules(
              &lurq::ui::Recipe::new().state(MenuItemState::Selected, MenuItemPart::Root, |s| {
                s.background(PaletteColor::Destructive)
              }),
            ),
        ),
    )
  });
  assert_eq!(stage.fill_color("zoom"), Some(light(PaletteColor::Destructive)));
  assert_ne!(stage.fill_color("grid"), Some(light(PaletteColor::Destructive)));
}

#[test]
fn tab_from_a_submenu_tells_on_dismiss_of_every_menu_it_closes() {
  let (open, exporting) = (Signal::new(false), Signal::new(false));
  let (anchor, export) = (ElementRef::new(), ElementRef::new());
  let heard = Arc::new(Mutex::new(Vec::new()));
  let (held, sub, told) = (open.clone(), exporting.clone(), heard.clone());
  let mut stage = Stage::new(move |_| {
    let toggle = held.clone();
    let (menu_told, sub_told) = (told.clone(), told.clone());
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
        Menu::new(held.clone(), anchor.clone())
          .on_dismiss(move || menu_told.lock().unwrap().push("menu"))
          .item(MenuItem::new("Undo").id("undo"))
          .item(
            MenuItem::new("Export as").id("export").submenu(
              Menu::new(sub.clone(), export.clone())
                .on_dismiss(move || sub_told.lock().unwrap().push("submenu"))
                .item(MenuItem::new("PNG").id("png")),
            ),
          ),
      )
      .into()
  });
  stage.key("Tab");
  stage.key("Enter");
  stage.key("ArrowDown");
  stage.key("ArrowRight");
  assert!(exporting.get());
  stage.key("Tab");
  assert!(!open.get() && !exporting.get());
  assert_eq!(*heard.lock().unwrap(), ["submenu", "menu"], "both were dismissed");
}

#[test]
fn a_recipe_sees_the_side_of_its_anchor_the_menu_opened_on() {
  use lurq::ui::{MenuPart, TooltipSide};
  let (over, under) = (Color::new(200, 30, 30, 255), Color::new(30, 30, 200, 255));
  let menu_over = move |placement: Placement| {
    let anchor = ElementRef::new();
    let app = lurq::app::App::new();
    app.theme().patch_recipe::<Menu>(|recipe| {
      recipe
        .state(TooltipSide::Top, MenuPart::Panel, move |s| s.background(over))
        .state(TooltipSide::Bottom, MenuPart::Panel, move |s| s.background(under))
    });
    Stage::in_app(app, move |_| {
      page(
        anchor.clone(),
        Menu::controlled(true, anchor.clone(), |_| {})
          .id("menu")
          .placement(placement)
          .item(MenuItem::new("Undo")),
      )
    })
  };
  let mut below = menu_over(Placement::BottomStart);
  assert_eq!(below.fill_color("menu"), Some(under), "under its anchor, as asked");
  // The anchor is 20 from the top: a menu asked to open over it has no room there, and goes under it.
  let mut flipped = menu_over(Placement::TopStart);
  let (_, ay, _, ah) = flipped.bounds("anchor");
  let (_, y, ..) = flipped.bounds("menu");
  assert!(y >= ay + ah, "it went under its anchor");
  assert_eq!(flipped.fill_color("menu"), Some(under), "and its recipe sees it there");
}
