//! Window tabs that leave the strip for a list, as Orchester's title bar has them: a gap between tabs that the
//! fitting counts, so a tab that left comes back when the strip has room for it again; separators that stand the
//! same whether a tab is in the strip or not; and a tab as wide as what it shows, up to a cap.

use std::{
  sync::{Arc, Mutex},
  time::Duration,
};

use lurq::{
  app::events::MouseButton,
  components::{Column, Row},
  core::Signal,
  ui::{Recipe, WindowTab, WindowTabs, WindowTabsMenu, WindowTabsOverflow, WindowTabsPart},
};

use super::{
  stage::Stage,
  window_tabs::{close_to, wide},
};

/// The rules of a strip whose tabs are 2 apart, and 2 before the add button.
fn gapped(separators: bool) -> Recipe {
  let rules = Recipe::new()
    .base(WindowTabsPart::List, |s| s.gap(2.0))
    .base(WindowTabsPart::Root, |s| s.gap(2.0));
  if separators {
    rules
  } else {
    rules.base(WindowTabsPart::Separator, |s| s.width(0.0).height(0.0))
  }
}

/// `count` tabs of ids `t0`.. in a room as wide as `room` says, in the list mode with `rules`, the first selected.
fn roomy(count: usize, room: Signal<f32>, rules: Recipe) -> Stage {
  wide(move |ctx| {
    let tabs = WindowTabs::controlled(
      0,
      (0..count).map(|index| WindowTab::new(index, &format!("{index}.rs")).id(format!("t{index}"))),
      |_| {},
    )
    .id("strip")
    .overflow(WindowTabsOverflow::List)
    .rules(&rules)
    .on_add(|| {})
    .mount(ctx);
    Column::new()
      .size(800.0, 300.0)
      .child(Row::new().width(room.get()).child(tabs))
      .into()
  })
}

/// The tabs of `count` that are in the strip, by index.
fn in_strip(stage: &mut Stage, count: usize) -> Vec<usize> {
  (0..count)
    .filter(|index| stage.bounds(&format!("t{index}")).2 > 0.0)
    .collect()
}

/// Lays the stage out again, until the tabs that leave and come back are settled.
fn settle(stage: &mut Stage) {
  for _ in 0..3 {
    stage.pass();
  }
}

#[test]
fn tabs_keep_the_lists_gap_and_one_that_left_comes_back_when_the_room_is_there_again() {
  for separators in [false, true] {
    // Three tabs of 128, 2 apart, then 2 before the add button of 30: all three fit in 420 with no separators. A
    // separator of 1 stands between the two plain tabs, with its gap, and one before the add button.
    let all = if separators { 420.0 + 1.0 + 2.0 + 1.0 + 2.0 } else { 420.0 };
    let room = Signal::new(all);
    let mut stage = roomy(3, room.clone(), gapped(separators));
    assert_eq!(in_strip(&mut stage, 3), [0, 1, 2], "all fit: {separators}");
    let (x0, ..) = stage.bounds("t0");
    let (x1, ..) = stage.bounds("t1");
    assert!(close_to(x1, x0 + 128.0 + 2.0), "2 apart: {x0} {x1}");
    let (_, _, strip, _) = stage.bounds("strip");
    assert!(close_to(strip, all), "the strip as wide as they are with their gaps: {strip}");

    room.set(all - 1.0);
    settle(&mut stage);
    assert_eq!(in_strip(&mut stage, 3), [0, 1], "the last left: {separators}");
    let (more_x, ..) = stage.bounds("strip-more");
    let (x1, _, w1, _) = stage.bounds("t1");
    assert!(close_to(more_x, x1 + w1 + 2.0), "the More button 2 after the tabs: {more_x}");

    room.set(all);
    settle(&mut stage);
    assert_eq!(in_strip(&mut stage, 3), [0, 1, 2], "back with the room: {separators}");
    assert_eq!(stage.bounds("strip-more").2, 0.0, "no More button");
  }
}

#[test]
fn a_tab_that_fits_its_content_is_as_wide_as_what_it_shows_up_to_its_cap() {
  let label = |stage: &Stage, text: &str| stage.text_quad(text).map(|quad| quad.width);
  for overflow in [WindowTabsOverflow::Scroll, WindowTabsOverflow::List] {
    let mut stage = wide(move |ctx| {
      let tabs = WindowTabs::controlled(
        0,
        [
          WindowTab::new(0, "a.rs").id("short").fit_content(240.0),
          WindowTab::new(1, "a_label_that_is_far_longer_than_its_cap.rs")
            .id("long")
            .fit_content(160.0),
          WindowTab::new(2, "b.rs").id("plain"),
        ],
        |_| {},
      )
      .id("strip")
      .overflow(overflow)
      .rules(&Recipe::new().base(WindowTabsPart::Tab, |s| s.min_width(0.0)));
      Row::new().child(tabs.mount(ctx)).into()
    });
    let (short_x, _, short, _) = stage.bounds("short");
    let close = stage.bounds("short-close");
    let text = label(&stage, "a.rs").expect("the label is painted");
    // Padded 10, the label, 8, the close slot of 18, padded 6.
    assert!(
      close_to(short, 10.0 + text + 8.0 + 18.0 + 6.0),
      "as wide as what it shows: {short} {text} {overflow:?}"
    );
    assert!(close_to(close.0 + close.2, short_x + short - 6.0));
    assert_eq!(stage.bounds("long").2, 160.0, "the cap: {overflow:?}");
    assert!(
      stage
        .cut_text_quad("a_label_that_is_far_longer_than_its_cap.rs")
        .is_some(),
      "cut with an ellipsis"
    );
    assert_eq!(stage.bounds("plain").2, 128.0, "another keeps the recipe's");
    let (_, _, strip, _) = stage.bounds("strip");
    assert!(strip >= short + 160.0 + 128.0 - 0.5, "{strip}");
  }
}

/// What the app was told of the list of every tab: the tabs closed.
type Closed = Arc<Mutex<Vec<usize>>>;

/// `count` tabs of ids `t0`.. that the app holds and removes when closed, in a room `room` wide, in the list mode
/// whose More button lists every tab, `selected` the selected one, with the list shortcut.
fn every_tab(count: usize, room: f32, selected: usize) -> (Stage, Signal<usize>, Closed) {
  let value = Signal::new(selected);
  let open = Signal::new((0..count).collect::<Vec<usize>>());
  let closed = Arc::new(Mutex::new(Vec::new()));
  let (bound, heard) = (value.clone(), closed.clone());
  let stage = wide(move |ctx| {
    let (heard, held) = (heard.clone(), open.clone());
    let tabs = WindowTabs::new(
      bound.clone(),
      open
        .get()
        .into_iter()
        .map(|index| WindowTab::new(index, &format!("{index}.rs")).id(format!("t{index}"))),
    )
    .id("strip")
    .overflow(WindowTabsOverflow::List)
    .menu(WindowTabsMenu::All)
    .list_shortcut(true)
    .on_add(|| {})
    .on_close(move |closed| {
      heard.lock().unwrap().push(*closed);
      held.update(|tabs| tabs.retain(|tab| tab != closed));
    })
    .mount(ctx);
    Column::new()
      .size(800.0, 400.0)
      .child(Row::new().width(room).child(tabs))
      .into()
  });
  (stage, value, closed)
}

/// The id of the element the focus is on.
fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// Whether the element `id` is laid out with a size.
fn shown(stage: &mut Stage, id: &str) -> bool {
  stage.tree.get_element_by_id_mut(id).is_some() && stage.bounds(id).2 > 0.0
}

/// A key with the platform's command key held (Cmd on macOS, Ctrl elsewhere), and Shift or not.
fn command(stage: &mut Stage, key: &str, shift: bool) {
  let mac = cfg!(target_os = "macos");
  stage
    .tree
    .key_down_with_meta(key.to_owned(), key.to_owned(), shift, !mac, false, mac);
  stage.tree.key_up(key.to_owned(), key.to_owned(), shift, !mac, false);
  stage.pass();
}

/// Types `text` into the focused field.
fn type_text(stage: &mut Stage, text: &str) {
  stage.tree.text_input_replace(0..0, text);
  stage.pass();
}

#[test]
fn the_list_of_every_tab_marks_the_selected_one_and_those_not_in_the_strip() {
  let (mut stage, ..) = every_tab(10, 400.0, 0);
  assert_eq!(in_strip(&mut stage, 10), [0, 1]);
  let closed_fill = stage.fill_color("strip-more");
  stage.click("strip-more");
  for index in 0..10 {
    assert!(shown(&mut stage, &format!("t{index}-item")), "the row of {index}");
  }
  assert!(shown(&mut stage, "t0-item-current"), "the selected tab is checked");
  assert!(!shown(&mut stage, "t1-item-current"));
  assert!(!shown(&mut stage, "t1-item-hidden"), "a tab in the strip has no mark");
  assert!(shown(&mut stage, "t2-item-hidden"), "a tab not in the strip is marked");
  // In the strip's order, one under the other.
  assert!(stage.bounds("t1-item").1 > stage.bounds("t0-item").1);
  // More than eight: a search field at its head, which has the focus.
  assert!(stage.paints("Search 10 open tabs"));
  assert!(
    focused(&stage).is_some_and(|id| id.starts_with("strip-search")),
    "{:?}",
    focused(&stage)
  );
  // The button is pressed while the list is open.
  assert_ne!(stage.fill_color("strip-more"), closed_fill);

  // A press of a row chooses its tab, and closes the list.
  stage.click("t6-item");
  stage.pass();
  assert!(!shown(&mut stage, "t6-item"), "closed");
  assert!(in_strip(&mut stage, 10).contains(&6));
}

#[test]
fn the_list_of_every_tab_searches_and_says_where_nothing_matches() {
  let (mut stage, value, _) = every_tab(10, 400.0, 0);
  stage.click("strip-more");
  type_text(&mut stage, "7");
  assert!(shown(&mut stage, "t7-item"));
  assert!(!shown(&mut stage, "t6-item"), "a row that does not match is not listed");
  // Enter chooses the first that matches.
  stage.key("Enter");
  assert_eq!(value.get(), 7);
  stage.pass();
  assert!(!shown(&mut stage, "t7-item"), "closed");

  stage.click("strip-more");
  assert!(shown(&mut stage, "t6-item"), "the search is empty again");
  type_text(&mut stage, "zzz");
  assert!(stage.paints("No open tab matches “zzz”"));
  // Escape empties the search, then closes the list, and the focus goes back to the button.
  stage.key("Escape");
  assert!(shown(&mut stage, "t6-item"));
  stage.key("Escape");
  assert!(!shown(&mut stage, "t6-item"));
  assert_eq!(focused(&stage).as_deref(), Some("strip-more"));
}

#[test]
fn the_keys_open_the_list_move_through_it_and_close_a_rows_tab() {
  let (mut stage, value, closed) = every_tab(10, 400.0, 0);
  command(&mut stage, "a", true);
  assert!(shown(&mut stage, "t0-item"), "Ctrl+Shift+A opens it");
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("t0-item"), "Down from the search field");
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("t1-item"));
  // Ctrl+W (Cmd+W) closes the row's tab and not the selected one, and the keys go on to the next row.
  command(&mut stage, "w", false);
  assert_eq!(*closed.lock().unwrap(), [1]);
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("t2-item"));
  assert_eq!(value.get(), 0);
  stage.key("ArrowUp");
  stage.key("ArrowUp");
  assert!(
    focused(&stage).is_some_and(|id| id.starts_with("strip-search")),
    "Up from the first row"
  );
  command(&mut stage, "a", true);
  assert!(!shown(&mut stage, "t0-item"), "the keys close it again");
}

#[test]
fn a_short_list_has_no_search_and_its_rows_close_their_tabs() {
  let (mut stage, _, closed) = every_tab(5, 340.0, 1);
  assert_eq!(in_strip(&mut stage, 5), [0, 1]);
  stage.click("strip-more");
  assert!(!stage.paints("Search 5 open tabs"), "no search field");
  assert_eq!(focused(&stage).as_deref(), Some("t1-item"), "on the selected tab");
  // Its close button closes the tab, and the list stays open.
  stage.click("t3-item-close");
  assert_eq!(*closed.lock().unwrap(), [3]);
  assert!(shown(&mut stage, "t4-item"));
  // The middle button closes a row's tab.
  let (x, y) = stage.center("t4-item");
  stage.tree.mouse_down(x, y, MouseButton::Middle);
  stage.tree.mouse_up(x, y, MouseButton::Middle);
  stage.pass();
  assert_eq!(*closed.lock().unwrap(), [3, 4]);
}

#[test]
fn the_more_buttons_tooltip_says_how_many_are_hidden_and_the_keys() {
  let (mut stage, ..) = every_tab(10, 400.0, 0);
  stage.own_timers();
  stage.hover("strip-more");
  stage.tick_timers(Duration::from_millis(600));
  stage.pass();
  assert!(stage.paints("8 hidden tabs"));
  let keys = if cfg!(target_os = "macos") { "⇧⌘A" } else { "Ctrl+Shift+A" };
  assert!(stage.paints(keys));
}
