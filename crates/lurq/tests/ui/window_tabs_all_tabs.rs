//! The list of every open tab as an app hosts it (Orchester's title bar): whether it is open is the app's, its keys
//! are its own while it has the focus, a row says a page and its project with the app's mark, it stands under the
//! title bar and inside the window, the row of the selected tab is kept in view, and the More button says the app's
//! words.

use std::sync::{Arc, Mutex};

use lurq::{
  app::events::KeyboardEvent,
  components::{Column, Row},
  core::{ElementRef, Signal},
  ui::{WindowTab, WindowTabs, WindowTabsMenu, WindowTabsOverflow},
};

use super::{
  stage::Stage,
  window_tabs::{close_to, wide},
};

/// What the strip and the page told: the menu opened or closed, the tabs closed, the keys the page heard.
#[derive(Clone, Default)]
struct Heard {
  opened: Arc<Mutex<Vec<bool>>>,
  closed: Arc<Mutex<Vec<usize>>>,
  page: Arc<Mutex<Vec<String>>>,
  /// How high the window is.
  window: Arc<Mutex<f32>>,
}

/// What a host is made of: how many tabs, which is selected, and the strip as the app sets it.
struct Host {
  count: usize,
  selected: usize,
  room: f32,
  height: f32,
  /// Whether the list stands clear of the title bar.
  under_bar: bool,
  strip: fn(WindowTabs<usize>) -> WindowTabs<usize>,
}

impl Host {
  fn new(count: usize, room: f32) -> Self {
    Self {
      count,
      selected: 0,
      room,
      height: 300.0,
      under_bar: false,
      strip: |strip| strip,
    }
  }
}

/// A page that hears every key, then a title bar of 36 (`bar`) holding the strip of `host` in the list mode with the
/// list of every tab, bound to `open`; a tab says its page and project, and the app's mark in its row.
fn hosted(host: Host, open: Signal<bool>, bar: ElementRef, heard: &Heard) -> (Stage, Signal<usize>) {
  let value = Signal::new(host.selected);
  let tabs = Signal::new((0..host.count).collect::<Vec<usize>>());
  let (bound, heard) = (value.clone(), heard.clone());
  let stage = wide(move |ctx| {
    let (closed, held, opened, page) = (
      heard.closed.clone(),
      tabs.clone(),
      heard.opened.clone(),
      heard.page.clone(),
    );
    let strip = WindowTabs::new(
      bound.clone(),
      tabs.get().into_iter().map(|index| {
        WindowTab::new(index, &format!("Project {index} · Page {index}"))
          .id(format!("t{index}"))
          .list_label(&format!("Page {index}"))
          .list_detail(&format!("Project {index}"))
          .list_trailing(Row::new().id(format!("t{index}-item-mark")).size(10.0, 10.0))
      }),
    )
    .id("strip")
    .overflow(WindowTabsOverflow::List)
    .menu(WindowTabsMenu::All)
    .menu_open(open.clone())
    .on_menu_open_change(move |open| opened.lock().unwrap().push(open))
    .on_close(move |tab| {
      closed.lock().unwrap().push(*tab);
      held.update(|tabs| tabs.retain(|held| held != tab));
    });
    let strip = if host.under_bar {
      strip.list_clear_of(bar.clone())
    } else {
      strip
    };
    let strip = (host.strip)(strip).mount(ctx);
    *heard.window.lock().unwrap() = ctx.window().info().logical_height();
    Column::new()
      .size(800.0, host.height)
      .child(Row::new().size(10.0, 10.0).on_key_down(move |event: KeyboardEvent| {
        let held = if event.ctrl || event.meta { "Ctrl+" } else { "" };
        page.lock().unwrap().push(format!("{held}{}", event.key));
      }))
      .child(
        Row::new()
          .ref_element(bar.clone())
          .id("bar")
          .width(800.0)
          .height(36.0)
          .child(Row::new().width(host.room).child(strip)),
      )
      .into()
  });
  (stage, value)
}

/// Whether the element `id` is laid out with a size.
fn shown(stage: &mut Stage, id: &str) -> bool {
  stage.tree.get_element_by_id_mut(id).is_some() && stage.bounds(id).2 > 0.0
}

/// The id of the element the focus is on.
fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// The window's close shortcut: Cmd+W on macOS, Ctrl+W elsewhere.
fn close_key(stage: &mut Stage) {
  let mac = cfg!(target_os = "macos");
  stage
    .tree
    .key_down_with_meta("w".to_owned(), "w".to_owned(), false, !mac, false, mac);
  stage.tree.key_up("w".to_owned(), "w".to_owned(), false, !mac, false);
  stage.pass();
}

#[test]
fn whether_the_list_is_open_is_the_apps_signal_and_a_persons_changes_are_told() {
  let (open, heard) = (Signal::new(false), Heard::default());
  let (mut stage, _) = hosted(Host::new(5, 340.0), open.clone(), ElementRef::new(), &heard);
  stage.click("strip-more");
  assert!(open.get(), "the button opens it, and the app's signal says so");
  assert!(shown(&mut stage, "t0-item"));
  stage.key("Escape");
  assert!(!open.get());
  assert!(!shown(&mut stage, "t0-item"));
  assert_eq!(*heard.opened.lock().unwrap(), [true, false], "a person opened it and closed it");

  open.set(true);
  stage.pass();
  stage.pass();
  assert!(shown(&mut stage, "t0-item"), "the app opens it by its signal");
  stage.click("t3-item");
  assert!(!open.get(), "a choice closes it");
  assert_eq!(
    *heard.opened.lock().unwrap(),
    [true, false, false],
    "what the app set is not told; the choice is"
  );
}

#[test]
fn while_the_list_has_the_focus_its_keys_are_not_the_pages() {
  let (open, heard) = (Signal::new(false), Heard::default());
  let (mut stage, value) = hosted(Host::new(5, 340.0), open, ElementRef::new(), &heard);
  stage.click("strip-more");
  assert_eq!(focused(&stage).as_deref(), Some("t0-item"));
  stage.key("ArrowDown");
  close_key(&mut stage);
  assert_eq!(*heard.closed.lock().unwrap(), [1], "Ctrl+W closes the row's tab");
  stage.pass();
  stage.key("Enter");
  assert_eq!(value.get(), 2, "Enter chooses the row the keys are on");
  assert!(
    heard.page.lock().unwrap().is_empty(),
    "the page heard none of the list's keys: {:?}",
    heard.page.lock().unwrap()
  );
  close_key(&mut stage);
  assert_eq!(
    *heard.page.lock().unwrap(),
    ["Ctrl+w"],
    "with the list closed, the page hears the keys again"
  );
}

#[test]
fn a_row_says_its_page_its_project_and_the_apps_mark_and_a_search_finds_the_project() {
  let (open, heard) = (Signal::new(false), Heard::default());
  let (mut stage, _) = hosted(Host::new(10, 400.0), open, ElementRef::new(), &heard);
  stage.click("strip-more");
  assert!(stage.paints("Page 4"), "the row's own label");
  assert!(stage.paints("Project 4"), "and the quiet text after it");
  assert!(!stage.paints("Project 4 · Page 4"), "not the tab's label");
  let (row_x, _, row_width, _) = stage.bounds("t4-item");
  let (page, project) = (
    stage.text_quad("Page 4").expect("painted"),
    stage.text_quad("Project 4").expect("painted"),
  );
  assert!(project.x > page.x + page.width, "the project after the page");
  let (mark_x, _, mark_width, _) = stage.bounds("t4-item-mark");
  assert!(mark_width > 0.0 && mark_x > project.x, "the app's mark after the words");
  assert!(mark_x + mark_width < row_x + row_width);
  stage.tree.text_input_replace(0..0, "project 7");
  stage.pass();
  assert!(shown(&mut stage, "t7-item"), "the search matches the project");
  assert!(!shown(&mut stage, "t6-item"));
}

#[test]
fn the_list_stands_under_the_title_bar_inside_the_window_and_no_lower_than_its_inset() {
  for under_bar in [false, true] {
    let (open, heard) = (Signal::new(false), Heard::default());
    let mut host = Host::new(30, 790.0);
    host.under_bar = under_bar;
    host.strip = |strip| strip.list_inset(12.0);
    let (mut stage, _) = hosted(host, open, ElementRef::new(), &heard);
    stage.click("strip-more");
    let window = *heard.window.lock().unwrap();
    let (more_x, more_y, _, more_height) = stage.bounds("strip-more");
    let (bar_y, bar_height) = (stage.bounds("bar").1, 36.0);
    let (x, y, width, height) = stage.bounds("strip-panel");
    let top = if under_bar {
      bar_y + bar_height + 4.0
    } else {
      more_y + more_height + 4.0
    };
    assert!(close_to(y, top), "4 under the bar or the button: {y} {top} {under_bar}");
    assert!(x < more_x, "moved in from the window's end");
    assert!(close_to(x + width, 800.0 - 12.0), "12 inside the window's end: {x} {width}");
    assert!(close_to(y + height, window - 12.0), "12 above its bottom: {y} {height} {window}");
  }
}

#[test]
fn rows_under_the_search_field_abut_and_rows_alone_stand_apart() {
  for (count, room, gap) in [(10, 400.0, 0.0), (5, 340.0, 2.0)] {
    let (open, heard) = (Signal::new(false), Heard::default());
    let (mut stage, _) = hosted(Host::new(count, room), open, ElementRef::new(), &heard);
    stage.click("strip-more");
    let (_, y0, _, h0) = stage.bounds("t0-item");
    let (_, y1, ..) = stage.bounds("t1-item");
    assert!(close_to(y1, y0 + h0 + gap), "{count} tabs: {y0} {h0} {y1}");
  }
}

#[test]
fn the_open_list_keeps_the_selected_tabs_row_in_view_and_then_the_first_match() {
  let (open, heard) = (Signal::new(false), Heard::default());
  let mut host = Host::new(30, 400.0);
  host.selected = 25;
  let (mut stage, _) = hosted(host, open, ElementRef::new(), &heard);
  stage.click("strip-more");
  stage.pass();
  let (_, top, _, height) = stage.bounds("strip-panel");
  let in_view = |stage: &mut Stage, id: &str| {
    let (_, y, _, h) = stage.bounds(id);
    y >= top - 0.5 && y + h <= top + height + 0.5
  };
  assert!(
    focused(&stage).is_some_and(|id| id.starts_with("strip-search")),
    "the search field has the focus"
  );
  assert!(in_view(&mut stage, "t25-item"), "the selected tab's row is in view");
  assert!(!in_view(&mut stage, "t0-item"));
  stage.tree.text_input_replace(0..0, "page 2");
  stage.pass();
  stage.pass();
  assert!(in_view(&mut stage, "t25-item"), "the selected tab matches, and stays in view");
  stage.tree.text_input_replace(0..6, "page 1");
  stage.pass();
  stage.pass();
  assert!(in_view(&mut stage, "t1-item"), "else the first that matches");
}

#[test]
fn the_more_buttons_tooltip_says_the_apps_words() {
  let (open, heard) = (Signal::new(false), Heard::default());
  let mut host = Host::new(10, 400.0);
  host.strip = |strip| strip.more_tooltip(|count| format!("{count} more tabs"));
  let (mut stage, _) = hosted(host, open, ElementRef::new(), &heard);
  stage.own_timers();
  stage.hover("strip-more");
  stage.tick_timers(std::time::Duration::from_millis(600));
  stage.pass();
  assert!(stage.paints("8 more tabs"));
  assert!(!stage.paints("8 hidden tabs"));
}

/// Ctrl+Shift+A, or Cmd+Shift+A on macOS: the list shortcut.
fn list_key(stage: &mut Stage) {
  crate::support::shortcut(&mut stage.tree, "A", "KeyA", true);
  stage.pass();
  stage.pass();
}

#[test]
fn with_no_tab_hidden_the_list_opens_under_the_tabs_by_the_apps_signal_and_the_shortcut() {
  let (open, heard) = (Signal::new(false), Heard::default());
  let mut host = Host::new(3, 790.0);
  host.strip = |strip| strip.list_shortcut(true);
  let (mut stage, value) = hosted(host, open.clone(), ElementRef::new(), &heard);
  assert!(!shown(&mut stage, "strip-more"), "every tab in the strip: no More button");
  open.set(true);
  stage.pass();
  stage.pass();
  assert!(shown(&mut stage, "t0-item"), "the app opens it by its signal");
  assert!(!shown(&mut stage, "strip-more"), "the More button stays hidden");
  // It hangs from the tabs: 4 under them, its end at theirs.
  let (list_x, list_y, list_width, list_height) = stage.bounds("strip-list");
  let (x, y, width, _) = stage.bounds("strip-panel");
  assert!(close_to(y, list_y + list_height + 4.0), "4 under the tabs: {y}");
  assert!(close_to(x + width, list_x + list_width), "its end at the tabs' end: {x} {width}");
  stage.click("t2-item");
  assert_eq!(value.get(), 2, "a choice selects its tab");
  assert!(!open.get(), "and closes it");

  list_key(&mut stage);
  assert!(open.get(), "the shortcut opens it");
  assert!(shown(&mut stage, "t0-item"));
  assert_eq!(focused(&stage).as_deref(), Some("t2-item"), "on the selected tab's row");
  list_key(&mut stage);
  assert!(!open.get(), "and closes it");
  assert!(!shown(&mut stage, "t0-item"));
  assert_eq!(*heard.opened.lock().unwrap(), [false, true, false]);
}

#[test]
fn with_no_tab_hidden_the_list_stands_clear_of_the_title_bar() {
  let (open, heard) = (Signal::new(false), Heard::default());
  let mut host = Host::new(3, 790.0);
  host.under_bar = true;
  let (mut stage, _) = hosted(host, open.clone(), ElementRef::new(), &heard);
  open.set(true);
  stage.pass();
  stage.pass();
  let (bar_y, bar_height) = (stage.bounds("bar").1, 36.0);
  let (_, y, ..) = stage.bounds("strip-panel");
  assert!(close_to(y, bar_y + bar_height + 4.0), "4 under the bar: {y}");
}
