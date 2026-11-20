//! Window tabs as an app's title bar hosts them: the app's marks after a label, what a tab says to tooling, a
//! middle press on any tab a person may close, the tabs that do not fit in the menu of a button, a strip Tab passes
//! by, the dot of unsaved changes beside a close button that stays, and a tab's own widths.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{events::MouseButton, theme::PaletteColor},
  components::{Column, Rect, Row},
  core::Signal,
  ui::{
    Button, Recipe, Selector, WindowTab, WindowTabs, WindowTabsDirtyMark, WindowTabsOverflow, WindowTabsPart,
    WindowTabsState, icons,
  },
};

use super::{
  stage::Stage,
  window_tabs::{board, close_shortcut, close_to, drag, filled, key_with, light, staged, wide},
};

/// A label longer than any tab's room.
const LONG: &str = "a_file_whose_name_is_longer_than_its_tab.rs";

/// The id of the element the focus is on.
fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// A press and a release of the middle button at the middle of `id`.
fn middle_press(stage: &mut Stage, id: &str) {
  let (x, y) = stage.center(id);
  stage.tree.mouse_down(x, y, MouseButton::Middle);
  stage.tree.mouse_up(x, y, MouseButton::Middle);
  stage.pass();
}

#[test]
fn a_tab_holds_the_apps_marks_after_its_label_and_the_label_gives_way_first() {
  let mut stage = staged(|| {
    WindowTabs::controlled(
      0,
      [
        WindowTab::new(0, "blur.rs")
          .id("blur")
          .trailing(Rect::new(14.0, 14.0).id("hand")),
        WindowTab::new(1, LONG)
          .id("long")
          .trailing(Rect::new(14.0, 14.0).id("alert"))
          .trailing(Rect::new(10.0, 10.0).id("count")),
      ],
      |_| {},
    )
    .id("strip")
  });
  // Right after the label, 6 from it, and before the close slot.
  let label = stage.text_quad("blur.rs").expect("the label is painted");
  let (hand_x, _, hand_width, _) = stage.bounds("hand");
  assert!(
    (hand_x - (label.x + label.width + 6.0)).abs() < 1.0,
    "6 after the label: {hand_x} {} {}",
    label.x,
    label.width
  );
  assert_eq!(hand_width, 14.0);
  let (close_x, ..) = stage.bounds("blur-close");
  assert!(hand_x + hand_width <= close_x - 8.0 + 0.5);
  // In a part of their own, named after the tab.
  assert_eq!(stage.bounds("blur-marks").2, 14.0);

  // A label too long for its room is cut, and the marks keep their width, 4 apart, before the close slot.
  let cut = stage.cut_text_quad(LONG).expect("the label is painted");
  assert!(
    matches!(&cut.content, lurq::layout::quad::QuadContent::Text { text, .. } if text.ends_with('…')),
    "cut with an ellipsis"
  );
  let (alert_x, _, alert_width, _) = stage.bounds("alert");
  let (count_x, _, count_width, _) = stage.bounds("count");
  assert_eq!((alert_width, count_width), (14.0, 10.0));
  assert!(close_to(count_x, alert_x + 14.0 + 4.0), "{alert_x} {count_x}");
  assert!(
    cut.x + cut.width <= alert_x - 6.0 + 0.5,
    "the label ends before the marks"
  );
  let (close_x, ..) = stage.bounds("long-close");
  assert!(
    count_x + count_width <= close_x - 8.0 + 0.5,
    "the marks end before the close slot"
  );
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn a_tab_says_what_the_app_describes_before_what_the_strip_says() {
  let mut stage = staged(|| {
    WindowTabs::controlled(
      0,
      [
        WindowTab::new(0, "blur.rs")
          .id("blur")
          .describe("route", "/files/blur")
          .describe("a11y_name", "blur.rs, 2 waiting"),
        WindowTab::new(1, "theme.rs").id("theme"),
      ],
      |_| {},
    )
  });
  let mut read = |id: &str, what: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(what))
  };
  assert_eq!(read("blur", "route").as_deref(), Some("/files/blur"));
  assert_eq!(
    read("blur", "a11y_name").as_deref(),
    Some("blur.rs, 2 waiting"),
    "the app's first"
  );
  assert_eq!(
    read("blur", "a11y_role").as_deref(),
    Some("tab"),
    "the strip's own still said"
  );
  assert_eq!(read("theme", "route"), None);
}

#[test]
fn the_middle_button_closes_any_tab_a_person_may_close_and_none_other() {
  let closed = Arc::new(Mutex::new(Vec::new()));
  let heard = closed.clone();
  let mut stage = staged(move || {
    let heard = heard.clone();
    WindowTabs::controlled(
      2,
      [
        WindowTab::new(0, "blur.rs").id("blur"),
        WindowTab::new(1, "pinned.rs").id("pinned").close_button(false),
        WindowTab::new(2, "home").id("home").closable(false),
      ],
      |_| {},
    )
    .on_close(move |value| heard.lock().unwrap().push(*value))
  });
  middle_press(&mut stage, "pinned");
  assert_eq!(*closed.lock().unwrap(), [1], "closed, though it shows no close button");
  middle_press(&mut stage, "blur");
  assert_eq!(*closed.lock().unwrap(), [1, 0]);
  closed.lock().unwrap().clear();

  // A tab that is not closable shows no close button, and neither the middle button, Delete nor the window's
  // shortcut closes it.
  assert!(stage.tree.get_element_by_id_mut("home-close").is_none());
  middle_press(&mut stage, "home");
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("home"));
  stage.key("Delete");
  close_shortcut(&mut stage);
  assert!(closed.lock().unwrap().is_empty());
}

/// What `on_reorder` was told: from where, and to where.
type Moves = Arc<Mutex<Vec<(usize, usize)>>>;

/// `count` tabs of ids `t0`.. in a room `room` wide that leave the strip where they do not fit, `selected` the
/// selected one, mounted; what `on_reorder` hears is in `moves`.
fn listed(count: usize, room: f32, selected: usize) -> (Stage, Signal<usize>, Moves) {
  let value = Signal::new(selected);
  let moves = Arc::new(Mutex::new(Vec::new()));
  let (bound, heard) = (value.clone(), moves.clone());
  let stage = wide(move |ctx| {
    let heard = heard.clone();
    let tabs = WindowTabs::new(
      bound.clone(),
      (0..count).map(|index| WindowTab::new(index, &format!("{index}.rs")).id(format!("t{index}"))),
    )
    .id("strip")
    .label("Open files")
    .overflow(WindowTabsOverflow::List)
    .on_add(|| {})
    .on_reorder(move |from, to| heard.lock().unwrap().push((from, to)))
    .mount(ctx);
    // The menu is a layer: the root has a size of its own for it to be painted in.
    Column::new()
      .size(800.0, 300.0)
      .child(Row::new().width(room).child(tabs))
      .into()
  });
  (stage, value, moves)
}

/// The tabs of `count` that are in the strip, by index.
fn in_strip(stage: &mut Stage, count: usize) -> Vec<usize> {
  (0..count)
    .filter(|index| stage.bounds(&format!("t{index}")).2 > 0.0)
    .collect()
}

#[test]
fn in_the_list_mode_the_tabs_that_do_not_fit_leave_the_strip_for_the_menu_of_a_counted_button() {
  let (mut stage, value, _) = listed(10, 400.0, 0);
  // The tabs keep their width: two fit beside the More button and the add button, the others left.
  assert_eq!(in_strip(&mut stage, 10), [0, 1]);
  assert_eq!(stage.bounds("t0").2, 128.0);
  assert_eq!(stage.bounds("t1").2, 128.0);
  let (strip_x, _, strip, _) = stage.bounds("strip");
  assert!(strip <= 400.0 + 0.5, "{strip}");
  let (more_x, _, more, more_height) = stage.bounds("strip-more");
  assert!(more > 0.0 && more_height == 28.0, "{more} {more_height}");
  assert!(
    close_to(more_x, strip_x + 256.0),
    "right after the tabs that stay: {more_x}"
  );
  assert!(stage.paints("8"), "it shows how many left");
  let (add_x, ..) = stage.bounds("strip-add");
  assert!(add_x > more_x, "the add button after it");

  // Its menu lists the tabs that left; choosing one selects it, and it comes back into the strip.
  stage.click("strip-more");
  for index in 2..10 {
    assert!(stage.paints(&format!("{index}.rs")), "{index}.rs in the menu");
  }
  stage.click("t7-item");
  assert_eq!(value.get(), 7);
  stage.pass();
  let shown = in_strip(&mut stage, 10);
  assert!(shown.contains(&7), "the selected tab is in the strip: {shown:?}");
  assert_eq!(stage.bounds("t7").2, 128.0);

  // The window's keys reach a tab that left: it is selected and comes back.
  key_with(&mut stage, "Tab", false, true);
  assert_eq!(value.get(), 8);
  assert!(in_strip(&mut stage, 10).contains(&8));
}

#[test]
fn in_the_list_mode_a_strip_with_room_enough_has_no_more_button() {
  let (mut stage, ..) = listed(3, 600.0, 0);
  assert_eq!(in_strip(&mut stage, 3), [0, 1, 2]);
  assert_eq!(stage.bounds("strip-more").2, 0.0, "laid out only while a tab has left");
  assert!(!stage.paints("0"), "no count");
}

#[test]
fn in_the_list_mode_the_keyboard_reaches_the_more_button_and_the_tab_chosen_takes_the_focus() {
  let (mut stage, value, _) = listed(10, 400.0, 0);
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("t0"));
  stage.key("Tab");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("strip-more"),
    "a stop of its own after the tabs"
  );
  stage.key("ArrowLeft");
  stage.key("Enter");
  assert!(stage.paints("9.rs"), "Enter opens its menu");
  let item = focused(&stage);
  assert_eq!(item.as_deref(), Some("t2-item"), "on the first tab that left");
  stage.key("ArrowDown");
  stage.key("Enter");
  assert_eq!(value.get(), 3);
  stage.pass();
  assert!(in_strip(&mut stage, 10).contains(&3));
  assert_eq!(focused(&stage).as_deref(), Some("t3"), "the focus on the tab chosen");
}

#[test]
fn in_the_list_mode_a_drag_moves_a_tab_among_those_in_the_strip() {
  // With 9 selected, 0 and 9 are in the strip; 0 dragged past 9 goes after it.
  let (mut stage, _, moves) = listed(10, 400.0, 9);
  assert_eq!(in_strip(&mut stage, 10), [0, 9]);
  drag(&mut stage, "t0", 128.0, 0.0);
  assert_eq!(*moves.lock().unwrap(), [(0, 9)]);
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn tooling_reads_the_more_button_and_where_each_tab_is_among_them_all() {
  let (mut stage, ..) = listed(10, 400.0, 0);
  let mut read = |id: &str, what: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(what))
  };
  assert_eq!(read("strip-more", "a11y_name").as_deref(), Some("Show the hidden tabs"));
  assert_eq!(read("strip-more", "a11y_description").as_deref(), Some("8 hidden tabs"));
  assert_eq!(read("strip-more", "a11y_haspopup").as_deref(), Some("menu"));
  assert_eq!(read("strip-more", "a11y_expanded").as_deref(), Some("false"));
  assert_eq!(read("t5", "a11y_posinset").as_deref(), Some("6"));
  assert_eq!(read("t5", "a11y_setsize").as_deref(), Some("10"));
}

#[test]
fn a_strip_tab_passes_by_leaves_the_focus_where_it_was_when_pressed() {
  let value = Signal::new(0_u8);
  let bound = value.clone();
  let mut stage = staged(move || {
    WindowTabs::new(bound.clone(), super::window_tabs::files())
      .id("strip")
      .on_add(|| {})
      .tab_stop(false)
  });
  // Nothing else on the stage takes the focus: Tab finds no stop in the strip.
  stage.key("Tab");
  assert_eq!(focused(&stage), None);
  stage.click("canvas");
  assert_eq!(value.get(), 2, "a press still selects");
  assert_eq!(focused(&stage), None, "and leaves the focus where it was");
  stage.click("strip-add");
  assert_eq!(focused(&stage), None);
  // The window's keys still work.
  key_with(&mut stage, "Tab", false, true);
  assert_eq!(value.get(), 0);

  // Beside a stop of Tab of the app's, Tab goes to it and no further, and a press of a tab leaves it there.
  let mut stage_with = wide(|_| {
    Row::new()
      .padding(8.0)
      .child(Button::new("Search").id("search"))
      .child(board().tab_stop(false))
      .into()
  });
  stage_with.key("Tab");
  assert_eq!(focused(&stage_with).as_deref(), Some("search"));
  stage_with.click("canvas");
  assert_eq!(focused(&stage_with).as_deref(), Some("search"));
  stage_with.key("Tab");
  assert_eq!(focused(&stage_with).as_deref(), Some("search"), "round to it again");
}

#[test]
fn the_dot_may_stand_after_the_label_beside_a_close_button_that_stays() {
  let paint_x = |stage: &Stage| filled(stage, light(PaletteColor::Primary), 12.0, 12.0).len();
  // What a dirty tab's x is painted with: none of it in the close slot of a tab whose dot is after its label.
  let dirty_x = Recipe::new().when(
    Selector::new().state(WindowTabsState::Dirty),
    WindowTabsPart::CloseIcon,
    |s| s.background(PaletteColor::Primary).opacity(1.0),
  );
  let swapped = dirty_x.clone();
  let stage = staged(move || board().rules(&swapped));
  assert_eq!(
    paint_x(&stage),
    1,
    "the default: the dirty tab's x is in its close slot"
  );

  let mut stage = staged(move || board().dirty_mark(WindowTabsDirtyMark::AfterLabel).rules(&dirty_x));
  assert_eq!(paint_x(&stage), 0, "the close button is drawn as a clean tab's");
  let dots = filled(&stage, light(PaletteColor::Foreground), 7.0, 7.0);
  assert_eq!(dots.len(), 1, "one dot");
  let label = stage.text_quad("theme.rs").expect("the label is painted");
  assert!(
    (dots[0].x - (label.x + label.width + 6.0)).abs() < 1.0,
    "6 after the label: {} {} {}",
    dots[0].x,
    label.x,
    label.width
  );
  let (close_x, close_y, close_width, _) = stage.bounds("theme-close");
  assert_eq!(close_width, 18.0, "the close button keeps its slot");
  assert!(dots[0].x + 7.0 < close_x, "the dot is not in it");
  let (_, tab_y, _, tab_height) = stage.bounds("theme");
  assert!(close_to(dots[0].y + 3.5, tab_y + tab_height / 2.0) && close_to(close_y, tab_y + 7.0));
  // Under the pointer both still show.
  stage.hover("theme");
  assert_eq!(filled(&stage, light(PaletteColor::Foreground), 7.0, 7.0).len(), 1);
}

#[test]
fn a_tab_has_a_width_of_its_own_a_cap_or_its_icon_alone() {
  let mut stage = staged(|| {
    WindowTabs::controlled(
      1,
      [
        WindowTab::new(0, "Home")
          .id("home")
          .icon(icons::HOME)
          .icon_only(true)
          .closable(false),
        WindowTab::new(1, LONG).id("capped").max_width(96.0),
        WindowTab::new(2, "wide.rs").id("wide").width(180.0),
      ],
      |_| {},
    )
    .id("strip")
  });
  let (home_x, home_y, home_width, home_height) = stage.bounds("home");
  assert_eq!((home_width, home_height), (28.0, 32.0), "a home tab of its icon alone");
  assert!(!stage.paints("Home"), "no label");
  assert!(
    stage.tree.get_element_by_id_mut("home-close").is_none(),
    "no close slot"
  );
  // Its icon in its middle.
  let side = 14.0;
  assert!(
    stage
      .quads()
      .iter()
      .any(|quad| close_to(quad.x, home_x + (28.0 - side) / 2.0) && close_to(quad.y, home_y + (32.0 - side) / 2.0)),
    "the icon in the middle"
  );
  assert_eq!(stage.bounds("capped").2, 96.0, "the app's cap");
  assert_eq!(stage.bounds("wide").2, 180.0, "a width of its own");
  let (_, _, strip, _) = stage.bounds("strip");
  assert!(
    close_to(strip, 28.0 + 96.0 + 180.0),
    "the strip as wide as they are: {strip}"
  );

  // In a room too small the tabs give way, but a tab of a width of its own keeps it, and the capped one never
  // passes its cap.
  let mut stage = wide(|_| {
    Row::new()
      .width(320.0)
      .child(WindowTabs::controlled(
        0,
        [
          WindowTab::new(0, "Home").id("home").icon(icons::HOME).icon_only(true),
          WindowTab::new(1, "a.rs").id("a").max_width(96.0),
          WindowTab::new(2, "b.rs").id("b"),
          WindowTab::new(3, "c.rs").id("c"),
        ],
        |_| {},
      ))
      .into()
  });
  assert_eq!(stage.bounds("home").2, 28.0);
  assert!(stage.bounds("a").2 <= 96.0 + 0.5);
  assert!(stage.bounds("b").2 < 128.0);
}
