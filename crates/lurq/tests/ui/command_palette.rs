//! A command palette: its panel near the top of the window as the board draws it, what typing finds, the highlight
//! the keys and the pointer move, what Enter, a click and Escape do, the empty and the loading list, and a theme's
//! rules over it.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    ctx::Ctx,
    events::MouseButton,
    theme::{PaletteColor, ThemePalette},
  },
  components::Column,
  core::Signal,
  layout::{Constraints, Size, quad::QuadContent},
  node::{Element, color::Color},
  ui::{Button, CommandItem, CommandPalette, CommandPalettePart, icons},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// A window of 600 by 400 with `content` in it: the palette's layer covers the window, and a scene with a layer
/// has a root of the window's size. The window lays out what it holds at its own size, as a window does, in place of
/// the stage's 400 by 300.
fn stage(scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Stage {
  stage_in(App::new(), scene)
}

fn stage_in(app: App, scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::in_app(app, move |ctx| {
    Column::new().size(600.0, 400.0).child(scene(ctx)).into()
  });
  stage.tree.resize(600, 400);
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(600.0, 400.0))));
  stage.pass();
  stage
}

/// What was run, in order, and a handler for each command that says so.
fn log() -> (
  Arc<Mutex<Vec<&'static str>>>,
  impl Fn(&'static str) -> Box<dyn Fn() + Send + Sync> + Clone + Send + Sync + 'static,
) {
  let ran = Arc::new(Mutex::new(Vec::new()));
  let told = ran.clone();
  (ran, move |name| {
    let told = told.clone();
    Box::new(move || told.lock().unwrap().push(name))
  })
}

/// The board's palette: three suggestions, and a group of two settings of which the second is disabled.
fn palette(open: Signal<bool>, tell: impl Fn(&'static str) -> Box<dyn Fn() + Send + Sync>) -> CommandPalette {
  CommandPalette::new(open)
    .id("palette")
    .group(
      "Suggestions",
      [
        CommandItem::new("Toggle dark mode")
          .id("dark")
          .icon(icons::MOON)
          .shortcut("Ctrl+D")
          .keywords(["theme"])
          .on_select(tell("dark")),
        CommandItem::new("Open DevTools")
          .id("devtools")
          .icon(icons::TERMINAL)
          .shortcut("F12")
          .on_select(tell("devtools")),
        CommandItem::new("New window")
          .id("window")
          .icon(icons::PLUS)
          .shortcut("Ctrl+N")
          .on_select(tell("window")),
      ],
    )
    .group(
      "Settings",
      [
        CommandItem::new("Keyboard shortcuts")
          .id("keys")
          .detail("Preferences")
          .on_select(tell("keys")),
        CommandItem::new("Sign out")
          .id("out")
          .disabled(true)
          .on_select(tell("out")),
      ],
    )
}

/// The rows shown that are filled with the highlight.
fn highlighted(stage: &mut Stage) -> Vec<&'static str> {
  ["dark", "devtools", "window", "keys", "out"]
    .into_iter()
    .filter(|id| {
      let shown = stage
        .tree
        .get_element_by_id_mut(id)
        .and_then(|element| element.bounds())
        .is_some_and(|rect| rect.width > 0.0);
      shown && stage.fill_color(id) == Some(light(PaletteColor::Highlight))
    })
    .collect()
}

/// Types `text` a key at a time, as a person does.
fn type_in(stage: &mut Stage, text: &str) {
  for ch in text.chars() {
    stage.key(&ch.to_string());
  }
}

#[test]
fn an_open_palette_is_the_boards_panel_near_the_top_of_the_window() {
  let open = Signal::new(true);
  let (_, tell) = log();
  let mut stage = stage(move |_| palette(open.clone(), tell.clone()).into());
  // 420 wide in the middle of the window, 72 from its top, in the popover's color with an edge of the border's.
  let (x, y, width, _) = stage.bounds("palette");
  assert_eq!((x, y, width), (90.0, 72.0, 420.0));
  assert_eq!(stage.fill_color("palette"), Some(light(PaletteColor::Popover)));
  let edge = stage
    .border("palette")
    .and_then(|quad| quad.border)
    .and_then(|border| border.top);
  assert_eq!(
    edge.map(|side| (side.color, side.width)),
    Some((light(PaletteColor::Border), 1.0))
  );
  // The search row: the placeholder in the muted color at 14, and the key of Escape at its end in the mono type at
  // 11, 14 from the panel's edge.
  assert_eq!(
    stage.text("Type a command or search"),
    (light(PaletteColor::MutedForeground), 14.0)
  );
  assert_eq!(stage.text("Esc"), (light(PaletteColor::MutedForeground), 11.0));
  let esc = stage.text_quad("Esc").unwrap();
  assert!(esc.x + esc.width < x + width - 14.0 && esc.x + esc.width > x + width - 14.0 - 12.0);
  // The key is a Kbd's: a box of 20 on the muted fill around its text.
  let key = stage
    .quads()
    .into_iter()
    .find(|quad| {
      matches!(quad.content, QuadContent::Rect { color, .. } if color == light(PaletteColor::Muted))
        && quad.x < esc.x
        && quad.x + quad.width > esc.x + esc.width
    })
    .expect("the key's box");
  assert_eq!(key.height, 20.0);
  // The first group's label is 12 in the muted color, over the first row: a row is 32 tall, 4 in from the panel,
  // and the first one is highlighted.
  assert_eq!(stage.text("Suggestions"), (light(PaletteColor::MutedForeground), 12.0));
  let (rx, ry, rw, rh) = stage.bounds("dark");
  assert_eq!((rx - x, rw, rh), (4.0, 420.0 - 8.0, 32.0));
  assert!(stage.text_quad("Suggestions").unwrap().y < ry);
  assert!(ry > y + 44.0, "under the search row of 44");
  assert_eq!(highlighted(&mut stage), ["dark"]);
  // A shortcut is drawn as a menu draws it: the mono type at 11, muted, at the row's end.
  assert_eq!(stage.text("Ctrl+D"), (light(PaletteColor::MutedForeground), 11.0));
  let shortcut = stage.text_quad("Ctrl+D").unwrap();
  assert!((rx + rw - 8.0 - (shortcut.x + shortcut.width)).abs() < 0.5);
  // A detail is after its label, muted; a disabled row is dimmed.
  let (label, detail) = (
    stage.text_quad("Keyboard shortcuts").unwrap(),
    stage.text_quad("Preferences").unwrap(),
  );
  assert!(detail.x > label.x + label.width);
  assert_eq!(stage.text("Preferences").0, light(PaletteColor::MutedForeground));
}

#[test]
fn a_closed_palette_paints_nothing() {
  let open = Signal::new(false);
  let (_, tell) = log();
  let stage = stage(move |_| palette(open.clone(), tell.clone()).into());
  assert!(!stage.paints("Toggle dark mode"));
  assert!(!stage.paints("Type a command or search"));
}

#[test]
fn typing_finds_the_commands_whose_words_or_keywords_hold_every_word() {
  let open = Signal::new(true);
  let (_, tell) = log();
  let mut stage = stage(move |_| palette(open.clone(), tell.clone()).into());
  // The focus is in the input as the palette opens: what is typed goes there.
  type_in(&mut stage, "dev");
  assert!(stage.paints("Open DevTools"));
  for hidden in ["Toggle dark mode", "New window", "Keyboard shortcuts"] {
    assert!(!stage.paints(hidden), "{hidden}");
  }
  assert!(!stage.paints("Settings"), "a group with nothing shown is not shown");
  assert_eq!(
    highlighted(&mut stage),
    ["devtools"],
    "the first row shown is highlighted"
  );
  // A keyword finds a command, and every word must be found, in any order.
  for _ in 0..3 {
    stage.key("Backspace");
  }
  type_in(&mut stage, "theme");
  assert!(stage.paints("Toggle dark mode"));
  assert!(!stage.paints("Open DevTools"));
  for _ in 0..5 {
    stage.key("Backspace");
  }
  type_in(&mut stage, "window new");
  assert!(stage.paints("New window"));
  assert!(!stage.paints("Toggle dark mode"));
}

#[test]
fn down_and_up_move_the_highlight_across_groups_round_the_ends_past_a_disabled_row() {
  let open = Signal::new(true);
  let (_, tell) = log();
  let mut stage = stage(move |_| palette(open.clone(), tell.clone()).into());
  assert_eq!(highlighted(&mut stage), ["dark"]);
  for expected in ["devtools", "window", "keys", "dark"] {
    stage.key("ArrowDown");
    assert_eq!(
      highlighted(&mut stage),
      [expected],
      "Sign out is disabled and passed by"
    );
  }
  stage.key("ArrowUp");
  assert_eq!(
    highlighted(&mut stage),
    ["keys"],
    "round from the first to the last that can be run"
  );
}

#[test]
fn enter_runs_the_highlighted_command_and_closes_the_palette() {
  let open = Signal::new(true);
  let (ran, tell) = log();
  let held = open.clone();
  let mut stage = stage(move |_| palette(held.clone(), tell.clone()).into());
  stage.key("ArrowDown");
  stage.key("Enter");
  assert_eq!(*ran.lock().unwrap(), ["devtools"]);
  assert!(!open.get());
}

#[test]
fn the_pointer_moves_the_highlight_and_a_click_runs_a_command() {
  let open = Signal::new(true);
  let (ran, tell) = log();
  let held = open.clone();
  let mut stage = stage(move |_| palette(held.clone(), tell.clone()).into());
  stage.hover("window");
  assert_eq!(
    highlighted(&mut stage),
    ["window"],
    "one row is highlighted, the one under the pointer"
  );
  // A disabled row is passed by: the highlight stays, and a click runs nothing.
  stage.hover("out");
  assert_eq!(highlighted(&mut stage), ["window"]);
  stage.click("out");
  assert!(ran.lock().unwrap().is_empty());
  assert!(open.get());
  stage.click("keys");
  assert_eq!(*ran.lock().unwrap(), ["keys"]);
  assert!(!open.get());
}

#[test]
fn escape_clears_what_is_typed_and_then_closes() {
  let open = Signal::new(true);
  let (_, tell) = log();
  let held = open.clone();
  let mut stage = stage(move |_| palette(held.clone(), tell.clone()).into());
  type_in(&mut stage, "dev");
  assert!(!stage.paints("New window"));
  stage.key("Escape");
  assert!(open.get(), "the first Escape clears");
  assert!(stage.paints("New window"));
  assert!(stage.paints("Type a command or search"));
  stage.key("Escape");
  assert!(!open.get(), "the second closes");
}

#[test]
fn a_press_on_the_veil_closes_it_and_a_press_on_the_panel_does_not() {
  let open = Signal::new(true);
  let (_, tell) = log();
  let held = open.clone();
  let mut stage = stage(move |_| palette(held.clone(), tell.clone()).into());
  let press = |stage: &mut Stage, px: f32, py: f32| {
    stage.tree.mouse_move(px, py);
    stage.tree.mouse_down(px, py, MouseButton::Left);
    stage.tree.mouse_up(px, py, MouseButton::Left);
    stage.pass();
  };
  let (x, y, ..) = stage.bounds("palette");
  press(&mut stage, x + 20.0, y + 20.0);
  assert!(open.get(), "a press on the panel goes no further");
  // Low in the window and beside the panel: the veil.
  press(&mut stage, 10.0, 390.0);
  assert!(!open.get(), "a press on the veil closes it");
}

#[test]
fn nothing_found_says_so_and_more_on_the_way_shows_a_spinner() {
  let open = Signal::new(true);
  let (_, tell) = log();
  let held = open.clone();
  let mut empty = stage(move |_| palette(held.clone(), tell.clone()).empty("Nothing like that").into());
  type_in(&mut empty, "zzz");
  assert_eq!(empty.text("Nothing like that").0, light(PaletteColor::MutedForeground));

  let loading = Signal::new(true);
  let (_, tell) = log();
  let mut stage = stage(move |_| {
    CommandPalette::new(loading.clone())
      .loading(true)
      .item(CommandItem::new("Lisbon").on_select(tell("lisbon")))
      .into()
  });
  assert!(stage.paints("Lisbon"));
  assert!(stage.paints("Loading…"));
  assert!(!stage.paints("No results"), "a list still loading is not empty");
}

#[test]
fn an_app_that_fetches_hears_what_is_typed_and_shows_every_command_it_gives() {
  let (open, query) = (Signal::new(true), Signal::new(String::new()));
  let heard = Arc::new(Mutex::new(Vec::<String>::new()));
  let (held, typed, told) = (open.clone(), query.clone(), heard.clone());
  let mut stage = stage(move |_| {
    let told = told.clone();
    let found: Vec<CommandItem> = ["Lisbon", "London"]
      .into_iter()
      .filter(|city| !typed.get().is_empty() && city.to_lowercase().starts_with(&typed.get()))
      .map(CommandItem::new)
      .collect();
    CommandPalette::new(held.clone())
      .query(typed.clone())
      .filter(|_, _| true)
      .on_input(move |text| told.lock().unwrap().push(text.to_owned()))
      .items(found)
      .into()
  });
  type_in(&mut stage, "lo");
  assert_eq!(*heard.lock().unwrap(), ["l", "lo"]);
  assert_eq!(query.get(), "lo");
  assert!(stage.paints("London"));
  assert!(!stage.paints("Lisbon"), "the app gave London alone for \"lo\"");
}

#[test]
fn the_focus_goes_back_to_what_opened_the_palette() {
  let open = Signal::new(false);
  let (_, tell) = log();
  let held = open.clone();
  let mut stage = stage(move |_| {
    let toggle = held.clone();
    Column::new()
      .child(Button::new("Search").id("search").on_click(move |_| toggle.set(true)))
      .child(palette(held.clone(), tell.clone()))
      .into()
  });
  stage.key("Tab");
  stage.key("Enter");
  assert!(open.get());
  // Tab stays in the palette, whose one stop is its input.
  stage.key("Tab");
  assert!(open.get());
  stage.key("Escape");
  assert!(!open.get());
  stage.key("Enter");
  assert!(open.get(), "the button had the focus back");
}

#[test]
fn a_themes_rules_change_it() {
  let app = App::new();
  app.theme().patch_recipe::<CommandPalette>(|recipe| {
    recipe.base(CommandPalettePart::Panel, |s| {
      s.width(500.0).background(PaletteColor::Muted)
    })
  });
  let open = Signal::new(true);
  let (_, tell) = log();
  let mut stage = stage_in(app, move |_| palette(open.clone(), tell.clone()).into());
  let (x, _, width, _) = stage.bounds("palette");
  assert_eq!((x, width), (50.0, 500.0));
  assert_eq!(stage.fill_color("palette"), Some(light(PaletteColor::Muted)));
}
