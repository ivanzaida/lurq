//! Document tabs: the bar and its tabs against the board, the tab shown, a document not saved, a preview and a
//! pinned tab, the close control, a middle press and Ctrl+W, the keys of one stop of Tab, moving a tab by a drag and
//! by the keys, the tabs that do not fit going into the menu at the end, a name longer than its room, the density,
//! the context menu, the editor's keys from anywhere in the window and beside a window's tabs, and a theme's rules
//! and renderer.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    events::MouseButton,
    theme::{Density, PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  core::Signal,
  layout::{quad::QuadContent, text_style::FontStyle},
  node::{Element, color::Color},
  ui::{
    Button, DocumentTab, DocumentTabs, DocumentTabsPart, DocumentTabsRenderer, DocumentTabsView, MenuItem,
    WindowChrome, WindowPlatform, WindowTabs, icons,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn close(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.5
}

/// The board's tabs: a pinned one, blur.rs, theme.rs not saved, canvas.rs and README.md a preview.
fn files() -> Vec<DocumentTab<&'static str>> {
  vec![
    DocumentTab::new("main.rs", "main.rs")
      .icon(icons::FILE_CODE)
      .pinned(true)
      .id("main"),
    DocumentTab::new("blur.rs", "blur.rs").icon(icons::FILE_CODE).id("blur"),
    DocumentTab::new("theme.rs", "theme.rs")
      .icon(icons::FILE_CODE)
      .dirty(true)
      .id("theme"),
    DocumentTab::new("canvas.rs", "canvas.rs")
      .icon(icons::FILE_CODE)
      .id("canvas"),
    DocumentTab::new("README.md", "README.md")
      .icon(icons::FILE_TEXT)
      .preview(true)
      .id("readme"),
  ]
}

/// Everything a handler is told, in order.
type Log = Arc<Mutex<Vec<String>>>;

fn log() -> Log {
  Arc::new(Mutex::new(Vec::new()))
}

fn said(log: &Log) -> Vec<String> {
  log.lock().unwrap().clone()
}

/// The board's tabs, mounted in a column of `width`, bound to `open`, each handler telling `told`.
fn stage(width: f32, open: &Signal<&'static str>, told: &Log) -> Stage {
  let (open, told) = (open.clone(), told.clone());
  Stage::new(move |ctx| {
    let (change, closed, moved, added, kept) = (told.clone(), told.clone(), told.clone(), told.clone(), told.clone());
    Column::new()
      .size(width, 300.0)
      .child(
        DocumentTabs::new(open.clone(), files())
          .label("Open files")
          .id("bar")
          .on_change(move |value: &&str| change.lock().unwrap().push(format!("change {value}")))
          .on_close(move |value| closed.lock().unwrap().push(format!("close {value}")))
          .on_reorder(move |from, to| moved.lock().unwrap().push(format!("move {from} {to}")))
          .on_add(move || added.lock().unwrap().push("add".to_owned()))
          .on_keep(move |value| kept.lock().unwrap().push(format!("keep {value}")))
          .mount(ctx),
      )
      .into()
  })
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

fn key(stage: &mut Stage, key: &str, shift: bool, ctrl: bool) {
  stage.tree.key_down(key.to_owned(), key.to_owned(), shift, ctrl, false);
  stage.tree.key_up(key.to_owned(), key.to_owned(), shift, ctrl, false);
  stage.pass();
}

/// Tab into the bar, from the start of the window.
fn tab_in(stage: &mut Stage) {
  stage.key("Tab");
}

#[test]
fn the_bar_and_its_tabs_are_the_boards() {
  let (open, told) = (Signal::new("blur.rs"), log());
  let mut stage = stage(860.0, &open, &told);
  let (_, _, width, height) = stage.bounds("bar");
  assert_eq!((width, height), (860.0, 38.0), "a bar 38 high across its room");
  // The pinned tab is a square of 38; the others are padded 12 and 8, their icon 14, their close control 20.
  let (_, _, pinned_width, pinned_height) = stage.bounds("main");
  assert_eq!((pinned_width, pinned_height), (38.0, 38.0));
  assert!(!stage.paints("main.rs"), "a pinned tab is its pin alone");
  let (blur_x, _, blur_width, blur_height) = stage.bounds("blur");
  assert_eq!(blur_height, 38.0);
  let (close_x, _, close_width, close_height) = stage.bounds("blur-close");
  assert_eq!((close_width, close_height), (20.0, 20.0));
  assert!(
    close(blur_x + blur_width - 8.0, close_x + close_width),
    "8 after the close control"
  );
  let (label_color, label_size) = stage.text("blur.rs");
  assert_eq!(label_size, 13.0);
  assert_eq!(
    label_color,
    light(PaletteColor::Foreground),
    "the tab shown is in the foreground"
  );
  assert_eq!(
    stage.text("canvas.rs").0,
    light(PaletteColor::MutedForeground),
    "another in the muted foreground"
  );
  // The tab shown is the background's color, with a line of 2 in the primary color along its top.
  assert_eq!(stage.fill_color("blur"), Some(light(PaletteColor::Background)));
  let primary = light(PaletteColor::Primary);
  let line = stage.quads().into_iter().any(|quad| {
    close(quad.x, blur_x)
      && close(quad.width, blur_width)
      && quad
        .border
        .is_some_and(|border| border.top.is_some_and(|top| top.width == 2.0 && top.color == primary))
  });
  assert!(line, "a line of 2 along the top of the tab shown");
  // The bar is muted.
  let muted = stage.quads().into_iter().any(|quad| {
    matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Muted))
      && close(quad.width, 860.0)
  });
  assert!(muted, "the bar in the muted fill");
}

#[test]
fn a_document_not_saved_shows_a_dot_and_a_preview_is_in_italics() {
  let (open, told) = (Signal::new("blur.rs"), log());
  let stage = stage(860.0, &open, &told);
  // The dot: a round fill of 8 in the foreground.
  let dot = stage.quads().into_iter().any(|quad| {
    matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Foreground))
      && close(quad.width, 8.0)
      && close(quad.height, 8.0)
  });
  assert!(dot, "a dot of 8 in the foreground");
  let italic = |content: &str| {
    stage.quads().into_iter().any(|quad| {
      matches!(&quad.content, QuadContent::Text { text, style, .. } if text == content && style.style == FontStyle::Italic)
    })
  };
  assert!(italic("README.md"), "a preview's name in italics");
  assert!(!italic("canvas.rs"));
}

#[test]
fn a_press_shows_another_tab_and_tells_on_change_but_not_what_the_app_sets() {
  let (open, told) = (Signal::new("blur.rs"), log());
  let mut stage = stage(860.0, &open, &told);
  stage.click("canvas");
  assert_eq!(open.get(), "canvas.rs", "the signal is set");
  assert_eq!(said(&told), ["change canvas.rs"]);
  // The tab shown: nothing.
  stage.click("canvas");
  assert_eq!(said(&told).len(), 1);
  open.set("theme.rs");
  stage.pass();
  assert_eq!(said(&told).len(), 1, "what the app sets is told to none");
  assert_eq!(stage.fill_color("theme"), Some(light(PaletteColor::Background)));
}

#[test]
fn a_controlled_bar_shows_nothing_itself_and_an_optional_one_is_told_an_option() {
  let told = log();
  let tell = told.clone();
  let mut stage = Stage::new(move |_| {
    let tell = tell.clone();
    Column::new()
      .size(860.0, 300.0)
      .child(DocumentTabs::controlled_optional(
        None,
        files(),
        move |value: &Option<&str>| tell.lock().unwrap().push(format!("{value:?}")),
      ))
      .into()
  });
  stage.click("theme");
  assert_eq!(said(&told), ["Some(\"theme.rs\")"]);
  assert_ne!(
    stage.fill_color("theme"),
    Some(light(PaletteColor::Background)),
    "the app shows it, or not"
  );
}

#[test]
fn the_close_control_a_middle_press_and_ctrl_w_close_a_tab() {
  let (open, told) = (Signal::new("blur.rs"), log());
  let mut stage = stage(860.0, &open, &told);
  stage.click("canvas-close");
  assert_eq!(said(&told), ["close canvas.rs"], "and it shows nothing");
  assert_eq!(open.get(), "blur.rs");
  let (x, y) = stage.center("theme");
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Middle);
  stage.tree.mouse_up(x, y, MouseButton::Middle);
  stage.pass();
  assert_eq!(said(&told)[1..], ["close theme.rs"], "a middle press");
  // Ctrl+W on the tab focused closes it, and the focus goes to the tab after it.
  tab_in(&mut stage);
  assert_eq!(focused(&stage).as_deref(), Some("blur"), "Tab stops at the tab shown");
  key(&mut stage, "w", false, true);
  assert_eq!(said(&told)[2..], ["close blur.rs"]);
  assert_eq!(
    focused(&stage).as_deref(),
    Some("theme"),
    "the focus goes to the tab beside it"
  );
}

#[test]
fn a_tabs_own_handler_is_told_before_the_bars() {
  let told = log();
  let (first, second) = (told.clone(), told.clone());
  let mut stage = Stage::new(move |_| {
    let (own, bar) = (first.clone(), second.clone());
    Column::new()
      .size(860.0, 300.0)
      .child(
        DocumentTabs::controlled(
          "a",
          [DocumentTab::new("a", "a.rs")
            .id("a")
            .on_close(move || own.lock().unwrap().push("own".to_owned()))],
          |_| {},
        )
        .on_close(move |_| bar.lock().unwrap().push("bar".to_owned())),
      )
      .into()
  });
  stage.click("a-close");
  assert_eq!(said(&told), ["own", "bar"]);
}

#[test]
fn the_arrows_move_the_focus_and_enter_shows_the_tab() {
  let (open, told) = (Signal::new("blur.rs"), log());
  let mut stage = stage(860.0, &open, &told);
  tab_in(&mut stage);
  assert_eq!(focused(&stage).as_deref(), Some("blur"));
  stage.key("ArrowRight");
  assert_eq!(focused(&stage).as_deref(), Some("theme"));
  assert_eq!(open.get(), "blur.rs", "an arrow moves the focus, not what is shown");
  stage.key("Enter");
  assert_eq!(open.get(), "theme.rs");
  // Tab leaves the tabs for the new tab button.
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("bar-add"));
  stage.key("Enter");
  assert!(said(&told).contains(&"add".to_owned()));
}

#[test]
fn ctrl_shift_page_keys_move_a_tab_among_its_kind_and_the_focus_stays_on_it() {
  let (open, told) = (Signal::new("blur.rs"), log());
  let mut stage = stage(860.0, &open, &told);
  tab_in(&mut stage);
  key(&mut stage, "PageDown", true, true);
  assert_eq!(said(&told), ["move 1 2"]);
  // The pinned tab is before it: blur.rs cannot go before main.rs.
  key(&mut stage, "PageUp", true, true);
  assert_eq!(said(&told), ["move 1 2"], "a tab moves among its kind");
  assert_eq!(focused(&stage).as_deref(), Some("blur"));
}

#[test]
fn a_drag_along_the_bar_moves_a_tab_and_a_press_that_does_not_move_shows_it() {
  let (open, told) = (Signal::new("blur.rs"), log());
  let mut stage = stage(860.0, &open, &told);
  let (x, y) = stage.center("theme");
  let (canvas_x, _, canvas_width, _) = stage.bounds("canvas");
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_move(canvas_x + canvas_width * 0.75, y);
  stage.pass();
  // While it is dragged, the tab it goes after says so with a line in the primary color.
  let primary = light(PaletteColor::Primary);
  let (readme_x, ..) = stage.bounds("readme");
  let marked = stage.quads().into_iter().any(|quad| {
    close(quad.x, readme_x)
      && quad.border.is_some_and(|border| {
        border
          .left
          .is_some_and(|side| side.width == 2.0 && side.color == primary)
      })
  });
  assert!(marked, "where it goes is marked");
  stage
    .tree
    .mouse_up(canvas_x + canvas_width * 0.75, y, MouseButton::Left);
  stage.pass();
  assert_eq!(said(&told), ["move 2 3"], "theme.rs goes after canvas.rs");
  assert_eq!(open.get(), "blur.rs", "a drag shows nothing");
  // A press that shakes by less than the drag's start is a press.
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_move(x + 2.0, y);
  stage.tree.mouse_up(x + 2.0, y, MouseButton::Left);
  stage.pass();
  assert_eq!(said(&told)[1..], ["change theme.rs"]);
}

#[test]
fn tabs_that_do_not_fit_go_into_the_menu_but_the_one_shown() {
  let (open, told) = (Signal::new("canvas.rs"), log());
  let mut stage = stage(400.0, &open, &told);
  stage.pass();
  let (_, _, canvas_width, _) = stage.bounds("canvas");
  assert!(canvas_width > 0.0, "the tab shown stays");
  assert_eq!(stage.bounds("main").2, 38.0, "and the pinned one");
  let (_, _, readme_width, _) = stage.bounds("readme");
  assert_eq!(readme_width, 0.0, "the last one leaves first");
  let (more_x, _, more_width, _) = stage.bounds("bar-more");
  assert!(close(more_x + more_width, 400.0), "the More button at the end");
  // It says how many it holds, and its menu lists them.
  let hidden: Vec<&str> = ["blur", "theme", "readme"]
    .into_iter()
    .filter(|id| stage.bounds(id).2 == 0.0)
    .collect();
  assert!(stage.paints(&hidden.len().to_string()));
  stage.click("bar-more");
  assert!(stage.paints("README.md"), "in the menu");
  stage.click("readme-item");
  assert_eq!(open.get(), "README.md");
  stage.pass();
  assert!(stage.bounds("readme").2 > 0.0, "the tab chosen is in the bar now");
  // Wide enough again, the More button is gone.
  let (open, told) = (Signal::new("canvas.rs"), log());
  let mut wide = stage_of(860.0, &open, &told);
  wide.pass();
  assert!(
    wide
      .tree
      .get_element_by_id_mut("bar-more")
      .and_then(|more| more.bounds())
      .is_none_or(|rect| rect.width == 0.0)
  );
}

fn stage_of(width: f32, open: &Signal<&'static str>, told: &Log) -> Stage {
  stage(width, open, told)
}

#[test]
fn a_long_name_ends_in_an_ellipsis_and_the_close_control_keeps_its_room() {
  let long = "a_name_far_too_long_for_any_tab_of_the_bar_to_hold_whole.rs";
  let mut stage = Stage::new(move |_| {
    Column::new()
      .size(860.0, 300.0)
      .child(DocumentTabs::controlled(
        "long",
        [DocumentTab::new("long", long).icon(icons::FILE_CODE).id("long")],
        |_| {},
      ))
      .into()
  });
  let (_, _, width, _) = stage.bounds("long");
  assert!(width <= 240.0, "a tab is 240 at most: {width}");
  assert!(!stage.paints(long));
  assert!(stage.cut_text_quad(long).is_some(), "cut with an ellipsis");
  assert_eq!(stage.bounds("long-close").2, 20.0);
}

#[test]
fn the_bar_follows_the_density_and_its_squares_stay_square() {
  for (density, step) in [
    (Density::Compact, -8.0),
    (Density::Default, 0.0),
    (Density::Comfortable, 8.0),
    (Density::Touch, 12.0),
  ] {
    let app = App::new();
    app.theme().set_density(density);
    let mut stage = Stage::in_app(app, move |_| {
      Column::new()
        .size(860.0, 300.0)
        .child(
          DocumentTabs::controlled("blur.rs", files(), |_| {})
            .id("bar")
            .on_add(|| {}),
        )
        .into()
    });
    assert_eq!(stage.bounds("bar").3, 38.0 + step, "{density:?}");
    let (_, _, width, height) = stage.bounds("main");
    assert_eq!((width, height), (38.0 + step, 38.0 + step), "{density:?}");
    let (_, _, width, height) = stage.bounds("bar-add");
    assert_eq!((width, height), (38.0 + step, 38.0 + step), "{density:?}");
  }
}

#[test]
fn a_secondary_press_opens_the_apps_context_menu_of_the_tab() {
  let chosen = log();
  let tell = chosen.clone();
  let mut stage = Stage::new(move |ctx| {
    let tell = tell.clone();
    Column::new()
      .size(860.0, 300.0)
      .child(
        DocumentTabs::controlled("blur.rs", files(), |_| {})
          .context_menu(move |value: &&str| {
            let (tell, value) = (tell.clone(), value.to_string());
            vec![
              MenuItem::new("Close Others")
                .id("others")
                .on_select(move || tell.lock().unwrap().push(format!("others of {value}"))),
            ]
          })
          .mount(ctx),
      )
      .into()
  });
  let (x, y) = stage.center("theme");
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Right);
  stage.tree.mouse_up(x, y, MouseButton::Right);
  stage.pass();
  assert!(stage.paints("Close Others"));
  stage.click("others");
  assert_eq!(said(&chosen), ["others of theme.rs"]);
}

#[test]
fn a_double_press_of_a_preview_keeps_it() {
  let (open, told) = (Signal::new("blur.rs"), log());
  let mut stage = stage(860.0, &open, &told);
  let (x, y) = stage.center("readme");
  stage.tree.mouse_move(x, y);
  for _ in 0..2 {
    stage.tree.mouse_down(x, y, MouseButton::Left);
    stage.tree.mouse_up(x, y, MouseButton::Left);
  }
  stage.pass();
  assert!(said(&told).contains(&"keep README.md".to_owned()), "{:?}", said(&told));
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn tooling_reads_a_list_of_tabs_the_one_shown_and_each_close_control() {
  let (open, told) = (Signal::new("blur.rs"), log());
  let mut stage = stage(860.0, &open, &told);
  let mut say = |id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(say("blur", "a11y_role").as_deref(), Some("tab"));
  assert_eq!(say("blur", "a11y_selected").as_deref(), Some("true"));
  assert_eq!(say("canvas", "a11y_selected").as_deref(), Some("false"));
  assert_eq!(
    say("main", "a11y_name").as_deref(),
    Some("main.rs"),
    "a pinned tab is called by its name"
  );
  assert_eq!(say("blur-close", "a11y_name").as_deref(), Some("Close blur.rs"));
  assert_eq!(say("bar-add", "a11y_name").as_deref(), Some("New tab"));
}

#[test]
fn a_themes_rules_and_renderer_change_it() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .size(860.0, 300.0)
      .child(
        DocumentTabs::controlled("blur.rs", files(), |_| {})
          .id("bar")
          .style(DocumentTabsPart::Root, |s| s.height(30.0)),
      )
      .into()
  });
  assert_eq!(stage.bounds("bar").3, 30.0);
  let app = App::new();
  let renderer: DocumentTabsRenderer = Arc::new(|view: DocumentTabsView| {
    Row::new()
      .id("drawn")
      .child(lurq::components::Text::new(&format!("{} tabs", view.tabs.len())))
      .into()
  });
  app.theme().set_renderer::<DocumentTabs>(renderer);
  let stage = Stage::in_app(app, |_| {
    Column::new()
      .child(Element::from(DocumentTabs::controlled("blur.rs", files(), |_| {})))
      .into()
  });
  assert!(stage.paints("5 tabs"));
}

/// The board's tabs after a button the focus is on, their keys `shortcuts`, each handler telling `told`.
fn elsewhere(open: &Signal<&'static str>, told: &Log, shortcuts: bool) -> Stage {
  let (open, told) = (open.clone(), told.clone());
  let mut stage = Stage::new(move |ctx| {
    let (change, closed, moved) = (told.clone(), told.clone(), told.clone());
    Column::new()
      .size(860.0, 300.0)
      .child(Button::new("Elsewhere").id("elsewhere"))
      .child(
        DocumentTabs::new(open.clone(), files())
          .id("bar")
          .shortcuts(shortcuts)
          .on_change(move |value: &&str| change.lock().unwrap().push(format!("change {value}")))
          .on_close(move |value| closed.lock().unwrap().push(format!("close {value}")))
          .on_reorder(move |from, to| moved.lock().unwrap().push(format!("move {from} {to}")))
          .mount(ctx),
      )
      .into()
  });
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("elsewhere"));
  stage
}

/// Ctrl+W, or Cmd+W on macOS.
fn close_key(stage: &mut Stage) {
  let mac = cfg!(target_os = "macos");
  stage
    .tree
    .key_down_with_meta("w".to_owned(), "KeyW".to_owned(), false, !mac, false, mac);
  stage.tree.key_up("w".to_owned(), "KeyW".to_owned(), false, !mac, false);
  stage.pass();
}

#[test]
fn the_editors_keys_work_with_the_focus_elsewhere_and_a_pinned_tab_is_not_closed() {
  let (open, told) = (Signal::new("blur.rs"), log());
  let mut stage = elsewhere(&open, &told, true);
  key(&mut stage, "Tab", false, true);
  assert_eq!(open.get(), "theme.rs", "Ctrl+Tab shows the next tab");
  key(&mut stage, "Tab", true, true);
  assert_eq!(open.get(), "blur.rs", "Ctrl+Shift+Tab the one before");
  key(&mut stage, "Tab", true, true);
  assert_eq!(open.get(), "main.rs", "a pinned one too");
  key(&mut stage, "Tab", true, true);
  assert_eq!(open.get(), "README.md", "round at the ends");
  if !cfg!(target_os = "macos") {
    key(&mut stage, "PageUp", false, true);
    assert_eq!(open.get(), "canvas.rs", "Ctrl+PageUp the one before");
    key(&mut stage, "PageDown", false, true);
    assert_eq!(open.get(), "README.md");
  }
  assert_eq!(
    focused(&stage).as_deref(),
    Some("elsewhere"),
    "the focus stays where it was"
  );
  told.lock().unwrap().clear();
  // The tab shown moves among its kind, and closes; a pinned one is not closed.
  key(&mut stage, "PageUp", true, true);
  assert_eq!(said(&told), ["move 4 3"]);
  close_key(&mut stage);
  assert_eq!(said(&told)[1..], ["close README.md"]);
  open.set("main.rs");
  stage.pass();
  close_key(&mut stage);
  assert_eq!(said(&told).len(), 2, "a pinned tab is not closed by the keys");
  assert_eq!(focused(&stage).as_deref(), Some("elsewhere"));

  // Off, the keys are not the bar's.
  let (open, told) = (Signal::new("blur.rs"), log());
  let mut stage = elsewhere(&open, &told, false);
  close_key(&mut stage);
  assert!(said(&told).is_empty(), "Ctrl+W closes nothing");
  key(&mut stage, "Tab", false, true);
  assert_eq!(open.get(), "blur.rs", "Ctrl+Tab shows nothing");
}

#[test]
fn with_the_focus_on_a_tab_the_editors_keys_take_the_focus_along_and_a_key_acts_once() {
  let (open, told) = (Signal::new("blur.rs"), log());
  let mut stage = stage(860.0, &open, &told);
  tab_in(&mut stage);
  assert_eq!(focused(&stage).as_deref(), Some("blur"));
  key(&mut stage, "Tab", false, true);
  assert_eq!(open.get(), "theme.rs");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("theme"),
    "the focus follows the tab shown"
  );
  // Ctrl+W on the focused tab closes it once: the tab's own key, not the window's as well.
  stage.key("ArrowRight");
  assert_eq!(focused(&stage).as_deref(), Some("canvas"));
  told.lock().unwrap().clear();
  close_key(&mut stage);
  assert_eq!(said(&told), ["close canvas.rs"], "the focused tab, and only it");
}

#[test]
fn in_a_window_whose_title_bar_has_tabs_a_key_is_the_documents_and_the_windows_pass_it_by() {
  let (window, documents, told) = (Signal::new(1_u8), Signal::new("blur.rs"), log());
  let (seen_window, seen_documents, heard) = (window.clone(), documents.clone(), told.clone());
  let mut stage = Stage::new(move |ctx| {
    let (window_said, document_said) = (heard.clone(), heard.clone());
    let strip = WindowTabs::new(seen_window.clone(), [(1_u8, "Project"), (2, "Notes"), (3, "Scratch")])
      .on_change(move |value| window_said.lock().unwrap().push(format!("window {value}")))
      .mount(ctx);
    let editor = Column::new()
      .size(600.0, 200.0)
      .child(Button::new("Elsewhere").id("elsewhere"))
      .child(
        DocumentTabs::new(seen_documents.clone(), files())
          .on_change(move |value: &&str| document_said.lock().unwrap().push(format!("documents {value}")))
          .mount(ctx),
      );
    Column::new()
      .size(640.0, 300.0)
      .child(
        WindowChrome::new()
          .platform(WindowPlatform::Windows)
          .title("Editor")
          .tabs(strip)
          .content(editor)
          .mount(ctx),
      )
      .into()
  });
  stage.tree.resize(640, 300);
  stage.pass();
  stage.click("elsewhere");
  key(&mut stage, "Tab", false, true);
  assert_eq!(
    said(&told),
    ["documents theme.rs"],
    "one of them, and it is the documents'"
  );
  assert_eq!(window.get(), 1);
}

#[test]
fn a_tab_dragged_away_from_the_bar_is_the_apps_and_moves_nothing_in_the_bar() {
  let (open, told) = (Signal::new("blur.rs"), log());
  let away = log();
  let (bound, heard, moved) = (open.clone(), away.clone(), told.clone());
  let mut stage = Stage::new(move |ctx| {
    let (heard, moved) = (heard.clone(), moved.clone());
    Column::new()
      .size(860.0, 300.0)
      .child(
        DocumentTabs::new(bound.clone(), files())
          .id("bar")
          .on_reorder(move |from, to| moved.lock().unwrap().push(format!("move {from} {to}")))
          .on_drag_away(move |value: &&str, event| {
            let end = if event.drop_result.is_some() { "let go" } else { "away" };
            heard.lock().unwrap().push(format!("{end} {value}"));
          })
          .mount(ctx),
      )
      .into()
  });
  let (x, y) = stage.center("theme");
  let (canvas_x, _, canvas_width, _) = stage.bounds("canvas");
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  // Along the bar first: a place among the tabs, nothing told.
  stage.tree.mouse_move(canvas_x + canvas_width * 0.75, y);
  stage.pass();
  assert!(said(&away).is_empty());
  // Then down, farther from the bar than it is high: the app's.
  stage.tree.mouse_move(canvas_x + canvas_width * 0.75, y + 120.0);
  stage.pass();
  assert_eq!(said(&away), ["away theme.rs"]);
  let primary = light(PaletteColor::Primary);
  let marked = stage.quads().into_iter().any(|quad| {
    quad.border.is_some_and(|border| {
      border
        .left
        .is_some_and(|side| side.width == 2.0 && side.color == primary)
    })
  });
  assert!(!marked, "no place among the tabs while it is away");
  stage
    .tree
    .mouse_up(canvas_x + canvas_width * 0.75, y + 120.0, MouseButton::Left);
  stage.pass();
  assert_eq!(said(&away), ["away theme.rs", "let go theme.rs"]);
  assert!(said(&told).is_empty(), "the bar moves nothing");
  assert_eq!(open.get(), "blur.rs");
}

#[test]
fn a_press_that_shows_another_tab_leaves_the_focus_on_it_and_the_arrows_go_on_from_it() {
  let (open, told) = (Signal::new("blur.rs"), log());
  let mut stage = stage(860.0, &open, &told);
  let node = |stage: &mut Stage| stage.tree.get_element_by_id_mut("canvas").unwrap().node_id();
  let before = node(&mut stage);
  stage.click("canvas");
  assert_eq!(open.get(), "canvas.rs");
  assert_eq!(
    node(&mut stage),
    before,
    "the tab is the same element as it becomes the one shown"
  );
  assert_eq!(
    focused(&stage).as_deref(),
    Some("canvas"),
    "the pressed tab has the focus"
  );
  stage.key("ArrowLeft");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("theme"),
    "and the arrows go on from it"
  );
}

#[test]
fn a_closed_tab_leaves_the_focus_on_the_tab_that_takes_its_place() {
  // The app takes the tab out, as an editor does.
  let open = Signal::new("blur.rs");
  let files_open = Signal::new(vec!["blur.rs", "theme.rs", "canvas.rs"]);
  let (shown, list) = (open.clone(), files_open.clone());
  let mut stage = Stage::new(move |ctx| {
    let (closing, shown_now) = (list.clone(), shown.clone());
    let tabs: Vec<DocumentTab<&'static str>> = list
      .get()
      .into_iter()
      .map(|name| DocumentTab::new(name, name).id(name.trim_end_matches(".rs")))
      .collect();
    Column::new()
      .size(860.0, 300.0)
      .child(
        DocumentTabs::new(shown.clone(), tabs)
          .on_close(move |name| {
            let mut files = closing.get_untracked();
            let at = files.iter().position(|file| file == name).unwrap();
            files.remove(at);
            if shown_now.get_untracked() == *name {
              shown_now.set(files[at.min(files.len() - 1)]);
            }
            closing.set(files);
          })
          .mount(ctx),
      )
      .into()
  });
  stage.click("blur");
  assert_eq!(focused(&stage).as_deref(), Some("blur"));
  let theme = stage.tree.get_element_by_id_mut("theme").unwrap().node_id();
  stage.click("blur-close");
  stage.pass();
  assert_eq!(files_open.get(), ["theme.rs", "canvas.rs"]);
  assert_eq!(
    focused(&stage).as_deref(),
    Some("theme"),
    "the tab that takes its place"
  );
  assert_eq!(
    stage.tree.get_element_by_id_mut("theme").unwrap().node_id(),
    theme,
    "the same element as before, now the one shown"
  );
  stage.key("ArrowRight");
  assert_eq!(focused(&stage).as_deref(), Some("canvas"));
}

#[test]
fn a_tab_chosen_from_the_menu_of_what_does_not_fit_has_the_focus() {
  let (open, told) = (Signal::new("canvas.rs"), log());
  let mut stage = stage(400.0, &open, &told);
  stage.pass();
  stage.click("bar-more");
  stage.click("readme-item");
  stage.pass();
  assert_eq!(open.get(), "README.md");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("readme"),
    "the tab chosen, in the bar now"
  );
}
