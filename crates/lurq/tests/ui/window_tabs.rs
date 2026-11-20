//! Window tabs: the strip as the board draws it, its states, what a press,
//! a close, the add button, the window's keys and a drag tell the app, a
//! label longer than its room, more tabs than the strip's room, and a
//! theme's other anatomy.

use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App,
    events::{MouseButton, MouseEvent},
    theme::{OpacityLevel, PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  core::Signal,
  layout::{
    Constraints, Size,
    quad::{Quad, QuadContent},
  },
  node::{Element, color::Color},
  ui::{
    Case, Interaction, Opacity, WindowTab, WindowTabs, WindowTabsPart, WindowTabsRenderer, WindowTabsState,
    WindowTabsView, icons,
  },
};

use super::stage::Stage;

pub(super) fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The board's three tabs: blur.rs, theme.rs with unsaved changes, canvas.rs; ids `blur`, `theme`, `canvas`.
pub(super) fn files() -> [WindowTab<u8>; 3] {
  [
    WindowTab::new(0, "blur.rs").icon(icons::FILE_CODE).id("blur"),
    WindowTab::new(1, "theme.rs")
      .icon(icons::FILE_CODE)
      .dirty(true)
      .id("theme"),
    WindowTab::new(2, "canvas.rs").icon(icons::FILE_CODE).id("canvas"),
  ]
}

/// The board's strip: its three tabs, the first selected, and the add button.
pub(super) fn board() -> WindowTabs<u8> {
  WindowTabs::controlled(0, files(), |_| {}).on_add(|| {}).id("strip")
}

/// A window of 800 by 300 that renders `scene`: room for the board's strip.
pub(super) fn wide(scene: impl Fn(&mut lurq::app::ctx::Ctx) -> Element + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::new(scene);
  stage
    .tree
    .set_layout_constraints_override(Some(Constraints::loose(Size::new(800.0, 300.0))));
  stage.pass();
  stage
}

/// A stage of `tabs` 8 in from its corner, as a title bar holds them.
pub(super) fn staged<T: Clone + PartialEq + Send + Sync + 'static>(
  tabs: impl Fn() -> WindowTabs<T> + Send + Sync + 'static,
) -> Stage {
  wide(move |_| Row::new().padding(8.0).child(tabs()).into())
}

/// The filled rectangles of `color` that are `width` by `height`.
pub(super) fn filled(stage: &Stage, color: Color, width: f32, height: f32) -> Vec<Quad> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color)
        && (quad.width - width).abs() < 0.01
        && (quad.height - height).abs() < 0.01
    })
    .collect()
}

pub(super) fn close_to(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.5
}

/// A key with Shift, Ctrl or both, pressed with the focus wherever it is.
pub(super) fn key_with(stage: &mut Stage, key: &str, shift: bool, ctrl: bool) {
  stage.tree.key_down(key.to_owned(), key.to_owned(), shift, ctrl, false);
  stage.tree.key_up(key.to_owned(), key.to_owned(), shift, ctrl, false);
  stage.pass();
}

/// The window's close shortcut: Cmd+W on macOS, Ctrl+W elsewhere.
pub(super) fn close_shortcut(stage: &mut Stage) {
  let mac = cfg!(target_os = "macos");
  let key = "w".to_owned();
  stage
    .tree
    .key_down_with_meta(key.clone(), "KeyW".to_owned(), false, !mac, false, mac);
  stage.tree.key_up(key, "KeyW".to_owned(), false, !mac, false);
  stage.pass();
}

/// A press at the middle of `id`, a move of `by` to the side and `down` down in small steps, and the release.
pub(super) fn drag(stage: &mut Stage, id: &str, by: f32, down: f32) {
  let (x, y) = stage.center(id);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  for step in 1..=10 {
    let share = step as f32 / 10.0;
    stage.tree.mouse_move(x + by * share, y + down * share);
    stage.pass();
  }
  stage.tree.mouse_up(x + by, y + down, MouseButton::Left);
  stage.pass();
}

/// What the app was told, in order.
type Told = Arc<Mutex<Vec<String>>>;

/// Bound tabs of the board's files, which tell `told` what each handler hears: the app's signal and what was said.
fn told_stage(shortcuts: bool, mounted: bool) -> (Stage, Signal<u8>, Told) {
  let value = Signal::new(0_u8);
  let told: Told = Arc::new(Mutex::new(Vec::new()));
  let (bound, heard) = (value.clone(), told.clone());
  let stage = wide(move |ctx| {
    let say = |heard: &Told| {
      let heard = heard.clone();
      move |what: String| heard.lock().unwrap().push(what)
    };
    let (change, close, add, reorder, detach) = (say(&heard), say(&heard), say(&heard), say(&heard), say(&heard));
    let seen = bound.clone();
    let tabs = WindowTabs::new(bound.clone(), files())
      .id("strip")
      .shortcuts(shortcuts)
      .on_change(move |value| change(format!("change {value} (the signal has {})", seen.get_untracked())))
      .on_close(move |value| close(format!("close {value}")))
      .on_add(move || add("add".to_owned()))
      .on_reorder(move |from, to| reorder(format!("reorder {from} {to}")))
      .on_detach(move |value| detach(format!("detach {value}")));
    let tabs = if mounted { tabs.mount(ctx) } else { tabs.into() };
    Column::new().size(600.0, 300.0).padding(8.0).child(tabs).into()
  });
  (stage, value, told)
}

fn said(told: &Told) -> Vec<String> {
  std::mem::take(&mut *told.lock().unwrap())
}

#[test]
fn the_strip_is_the_boards_three_tabs_and_its_add_button() {
  let mut stage = staged(board);
  let (x, y, width, height) = stage.bounds("strip");
  assert_eq!((width, height), (416.0, 32.0), "128 + 128 + 1 + 128 + 1 + 30, 32 high");
  for (id, at) in [("blur", 0.0), ("theme", 128.0), ("canvas", 257.0)] {
    let (tab_x, tab_y, tab_width, tab_height) = stage.bounds(id);
    assert!(close_to(tab_x, x + at) && close_to(tab_y, y), "{id} at {at}");
    assert_eq!((tab_width, tab_height), (128.0, 32.0), "{id}");
  }
  // The add button is 30 by 28, in the middle of the strip's height, at its end.
  let (add_x, add_y, add_width, add_height) = stage.bounds("strip-add");
  assert_eq!((add_width, add_height), (30.0, 28.0));
  assert!(close_to(add_x, x + 386.0) && close_to(add_y, y + 2.0));

  // The selected tab is the background, round by 8 at its top only; the others have no fill.
  let fill = stage.fill("blur").expect("the selected tab is filled");
  assert!(matches!(fill.content, QuadContent::Rect { color, .. } if color == light(PaletteColor::Background)));
  let radius = fill.border_radius.expect("round at the top");
  assert_eq!(
    (
      radius.top_left,
      radius.top_right,
      radius.bottom_right,
      radius.bottom_left
    ),
    (8.0, 8.0, 0.0, 0.0)
  );
  // Edged at its top and sides by a line of 1 in the border's color, open at its bottom: on the light palette's white
  // bar its fill alone would not tell it apart.
  let border = light(PaletteColor::Border);
  let edge = stage
    .border("blur")
    .and_then(|quad| quad.border)
    .expect("the selected tab is edged");
  let side = |line: Option<lurq::node::border::ResolvedBorder>| line.map(|line| (line.width, line.color));
  assert_eq!(side(edge.top), Some((1.0, border)));
  assert_eq!(side(edge.left), Some((1.0, border)));
  assert_eq!(side(edge.right), Some((1.0, border)));
  assert_eq!(side(edge.bottom), None, "open to the content under it");
  assert!(stage.border("theme").is_none(), "another tab has no edge");
  assert_eq!(stage.fill_color("theme"), None);
  assert_eq!(stage.fill_color("canvas"), None);
  assert_eq!(stage.text("blur.rs"), (light(PaletteColor::Foreground), 12.0));
  assert_eq!(stage.text("theme.rs"), (light(PaletteColor::MutedForeground), 12.0));
  assert_eq!(stage.text("canvas.rs"), (light(PaletteColor::MutedForeground), 12.0));

  // The parts of a tab: the icon 10 in, the label after it, the close slot of 18 at 104, 7.
  let label = stage.text_quad("blur.rs").unwrap();
  assert!(close_to(label.x, x + 32.0), "the label at 32: {}", label.x);
  assert!(
    stage
      .quads()
      .iter()
      .any(|quad| matches!(&quad.content, QuadContent::Text { .. })
        && quad.x >= x + 9.5
        && quad.x + quad.width <= x + 24.5
        && quad.y >= y + 8.5),
    "an icon of 14 at 10"
  );
  let (close_x, close_y, close_width, close_height) = stage.bounds("blur-close");
  assert_eq!((close_width, close_height), (18.0, 18.0));
  assert!(close_to(close_x, x + 104.0) && close_to(close_y, y + 7.0));

  // A separator of 1 by 16 between the two tabs that are not selected and before the add button: none beside
  // the selected tab.
  let lines = filled(&stage, light(PaletteColor::Border), 1.0, 16.0);
  let mut at: Vec<f32> = lines.iter().map(|line| line.x - x).collect();
  at.sort_by(f32::total_cmp);
  assert_eq!(at.len(), 2, "two separators: {at:?}");
  assert!(close_to(at[0], 256.0) && close_to(at[1], 385.0), "{at:?}");
  assert!(lines.iter().all(|line| close_to(line.y, y + 8.0)));
}

#[test]
fn the_dot_of_unsaved_changes_stands_in_the_close_slot_and_the_x_in_its_place_under_the_pointer() {
  let mut stage = staged(board);
  let (x, y, ..) = stage.bounds("theme-close");
  let dots = filled(&stage, light(PaletteColor::Foreground), 7.0, 7.0);
  assert_eq!(dots.len(), 1, "one dot");
  assert!(
    close_to(dots[0].x, x + 5.5) && close_to(dots[0].y, y + 5.5),
    "in the middle of the slot"
  );
  assert!(
    dots[0].border_radius.is_some_and(|radius| radius.top_left >= 3.5),
    "round"
  );

  let theme = stage.app.theme().clone();
  let dirty = theme.resolve::<WindowTabs>(&Case::new().state(WindowTabsState::Dirty, true), None);
  let x_icon = dirty.part(WindowTabsPart::CloseIcon);
  assert_eq!(
    x_icon.styles().rest.opacity,
    Some(Opacity::Value(0.0)),
    "the x hidden at rest"
  );
  assert_eq!(
    x_icon.styles().within_hovered.opacity,
    Some(Opacity::Value(1.0)),
    "and shown under the pointer"
  );
  let dot = dirty.part(WindowTabsPart::Dot);
  assert_eq!(
    dot.styles().within_hovered.opacity,
    Some(Opacity::Value(0.0)),
    "the dot goes"
  );

  // Live: a rule of the test's paints the x while it shows.
  let lit = light(PaletteColor::Primary);
  let mut stage = staged(move || {
    board().style(WindowTabsPart::CloseIcon, |s| s).rules(
      &lurq::ui::Recipe::new().when(
        lurq::ui::Selector::new()
          .state(WindowTabsState::Dirty)
          .within()
          .hovered(),
        WindowTabsPart::CloseIcon,
        |s| s.background(PaletteColor::Primary),
      ),
    )
  });
  assert!(filled(&stage, lit, 12.0, 12.0).is_empty(), "not lit at rest");
  stage.hover("theme");
  assert_eq!(filled(&stage, lit, 12.0, 12.0).len(), 1, "lit under the pointer");
}

#[test]
fn a_tab_under_the_pointer_is_the_secondary_hover_and_its_x_is_whole() {
  let mut stage = staged(board);
  stage.hover("canvas");
  assert_eq!(stage.fill_color("canvas"), Some(light(PaletteColor::SecondaryHover)));
  assert_eq!(
    stage.text("canvas.rs").0,
    light(PaletteColor::MutedForeground),
    "its text stays muted"
  );
  stage.hover("blur");
  assert_eq!(
    stage.fill_color("blur"),
    Some(light(PaletteColor::Background)),
    "the selected tab stays"
  );

  let theme = stage.app.theme().clone();
  let idle = theme.resolve::<WindowTabs>(&Case::new(), None);
  let x_icon = idle.part(WindowTabsPart::CloseIcon);
  assert_eq!(
    x_icon.styles().rest.opacity,
    Some(Opacity::Value(0.6)),
    "dimmed at rest"
  );
  assert_eq!(x_icon.styles().within_hovered.opacity, Some(Opacity::Value(1.0)));
  assert_eq!(x_icon.rest().foreground, Some(PaletteColor::MutedForeground.into()));
  let selected = theme.resolve::<WindowTabs>(&Case::new().state(WindowTabsState::Selected, true), None);
  assert_eq!(
    selected.part(WindowTabsPart::CloseIcon).rest().opacity,
    Some(Opacity::Value(1.0)),
    "whole on the selected tab"
  );
}

#[test]
fn a_press_selects_a_tab_and_on_change_is_not_told_what_the_app_sets() {
  let (mut stage, value, told) = told_stage(true, false);
  stage.click("canvas");
  assert_eq!(value.get(), 2);
  assert_eq!(said(&told), ["change 2 (the signal has 2)"], "the signal is set first");
  stage.click("canvas");
  assert!(said(&told).is_empty(), "a press of the selected tab tells nothing");
  value.set(1);
  stage.pass();
  assert_eq!(stage.fill_color("theme"), Some(light(PaletteColor::Background)));
  assert!(said(&told).is_empty(), "what the app sets is told to nobody");
}

#[test]
fn controlled_tabs_show_what_the_app_answers() {
  let asked = Arc::new(Mutex::new(Vec::new()));
  let heard = asked.clone();
  let mut stage = staged(move || {
    let heard = heard.clone();
    WindowTabs::controlled(0, files(), move |value| heard.lock().unwrap().push(*value))
  });
  stage.click("canvas");
  assert_eq!(*asked.lock().unwrap(), [2]);
  assert_eq!(
    stage.fill_color("blur"),
    Some(light(PaletteColor::Background)),
    "the app did not answer"
  );
}

#[test]
fn a_close_tells_the_tab_and_the_strip_and_selects_nothing() {
  let order = Arc::new(Mutex::new(Vec::new()));
  let heard = order.clone();
  let mut stage = staged(move || {
    let (own, list, change) = (heard.clone(), heard.clone(), heard.clone());
    WindowTabs::controlled(
      0,
      [
        WindowTab::new(0, "blur.rs").id("blur"),
        WindowTab::new(1, "canvas.rs")
          .id("canvas")
          .on_close(move || own.lock().unwrap().push("tab".to_owned())),
        WindowTab::new(2, "pinned.rs").id("pinned").close_button(false),
        WindowTab::new(3, "home").id("home").closable(false),
      ],
      move |value| change.lock().unwrap().push(format!("change {value}")),
    )
    .on_close(move |value| list.lock().unwrap().push(format!("strip {value}")))
  });
  stage.click("canvas-close");
  assert_eq!(
    *order.lock().unwrap(),
    ["tab", "strip 1"],
    "the tab's own first; no change"
  );
  order.lock().unwrap().clear();

  // Delete on the focused tab closes it.
  stage.key("Tab");
  stage.key("Delete");
  assert_eq!(*order.lock().unwrap(), ["strip 0"]);
  order.lock().unwrap().clear();

  // A tab without a close button has none, and is still closed by the keys.
  assert!(stage.tree.get_element_by_id_mut("pinned-close").is_none());
  stage.click("pinned");
  order.lock().unwrap().clear();
  stage.key("Delete");
  assert_eq!(*order.lock().unwrap(), ["strip 2"]);
  order.lock().unwrap().clear();

  // A tab that is not closable has no close button, and nothing closes it.
  assert!(stage.tree.get_element_by_id_mut("home-close").is_none());
  stage.click("home");
  order.lock().unwrap().clear();
  stage.key("Delete");
  assert!(order.lock().unwrap().is_empty());
}

#[test]
fn a_press_of_the_middle_button_on_a_tab_closes_it() {
  let (mut stage, value, told) = told_stage(true, false);
  let (x, y) = stage.center("canvas");
  stage.tree.mouse_down(x, y, MouseButton::Middle);
  stage.tree.mouse_up(x, y, MouseButton::Middle);
  stage.pass();
  assert_eq!(said(&told), ["close 2"], "closed, and not selected");
  assert_eq!(value.get(), 0);
  // Pressed on one tab and let go on another, it closes neither.
  let (to_x, to_y) = stage.center("theme");
  stage.tree.mouse_down(x, y, MouseButton::Middle);
  stage.tree.mouse_up(to_x, to_y, MouseButton::Middle);
  stage.pass();
  assert!(said(&told).is_empty());
}

#[test]
fn optional_tabs_may_have_none_selected_and_are_told_some_of_a_tab() {
  let value: Signal<Option<u8>> = Signal::new(None);
  let told = Arc::new(Mutex::new(Vec::new()));
  let (bound, heard) = (value.clone(), told.clone());
  let mut stage = staged(move || {
    let heard = heard.clone();
    WindowTabs::optional(bound.clone(), files()).on_change(move |value: &Option<u8>| heard.lock().unwrap().push(*value))
  });
  for id in ["blur", "theme", "canvas"] {
    assert_eq!(stage.fill_color(id), None, "{id}: none is selected");
  }
  stage.click("theme");
  assert_eq!(value.get(), Some(1));
  assert_eq!(*told.lock().unwrap(), [Some(1)]);
  assert_eq!(stage.fill_color("theme"), Some(light(PaletteColor::Background)));

  // Held by the app, with a tab added after the others.
  let asked = Arc::new(Mutex::new(Vec::new()));
  let heard = asked.clone();
  let mut stage = staged(move || {
    let heard = heard.clone();
    WindowTabs::controlled_optional(None, files(), move |value: &Option<u8>| {
      heard.lock().unwrap().push(*value)
    })
    .tab(WindowTab::new(3, "layout.rs").id("layout"))
  });
  stage.click("layout");
  assert_eq!(*asked.lock().unwrap(), [Some(3)]);
}

#[test]
fn escape_puts_a_dragged_tab_back_and_its_release_tells_nothing() {
  for mounted in [false, true] {
    let (mut stage, _, told) = told_stage(true, mounted);
    let (x, y) = stage.center("blur");
    stage.tree.mouse_move(x, y);
    stage.tree.mouse_down(x, y, MouseButton::Left);
    for step in 1..=5 {
      stage.tree.mouse_move(x + 60.0 * step as f32, y);
      stage.pass();
    }
    stage.key("Escape");
    stage.tree.mouse_up(x + 300.0, y, MouseButton::Left);
    stage.pass();
    assert!(said(&told).is_empty(), "mounted: {mounted}");
    let (back, ..) = stage.bounds("blur");
    assert!(close_to(back, 8.0), "back at its place: {back}");
  }
}

#[test]
fn the_add_button_is_shown_once_the_app_hears_it() {
  let (mut stage, _, told) = told_stage(true, false);
  stage.click("strip-add");
  assert_eq!(said(&told), ["add"]);
  let mut stage = staged(|| WindowTabs::controlled(0, files(), |_| {}).id("strip"));
  assert!(stage.tree.get_element_by_id_mut("strip-add").is_none());
  let (_, _, width, _) = stage.bounds("strip");
  assert_eq!(
    width, 385.0,
    "the tabs and a separator between the two that are not selected"
  );
}

#[test]
fn the_windows_keys_select_close_and_move_from_anywhere() {
  let (mut stage, value, told) = told_stage(true, false);
  key_with(&mut stage, "Tab", false, true);
  assert_eq!(value.get(), 1, "Ctrl+Tab: the next");
  key_with(&mut stage, "Tab", true, true);
  key_with(&mut stage, "Tab", true, true);
  assert_eq!(value.get(), 2, "Ctrl+Shift+Tab: the one before, round at the start");
  key_with(&mut stage, "Tab", false, true);
  assert_eq!(value.get(), 0, "round at the end");
  if !cfg!(target_os = "macos") {
    key_with(&mut stage, "PageDown", false, true);
    assert_eq!(value.get(), 1, "Ctrl+PageDown");
    key_with(&mut stage, "PageUp", false, true);
    assert_eq!(value.get(), 0, "Ctrl+PageUp");
  }
  said(&told);
  key_with(&mut stage, "PageUp", true, true);
  assert!(said(&told).is_empty(), "the first tab moves no further back");
  key_with(&mut stage, "PageDown", true, true);
  assert_eq!(
    said(&told),
    ["reorder 0 1"],
    "Ctrl+Shift+PageDown moves the selected tab on"
  );
  close_shortcut(&mut stage);
  assert_eq!(
    said(&told),
    ["close 0"],
    "the window's close shortcut closes the selected tab"
  );
}

#[test]
fn the_windows_keys_are_off_when_the_app_says_so() {
  let (mut stage, value, told) = told_stage(false, false);
  // Within the strip the arrows still select.
  stage.key("Tab");
  stage.key("ArrowRight");
  assert_eq!(value.get(), 1, "Right selects the next");
  stage.key("ArrowLeft");
  said(&told);
  key_with(&mut stage, "Tab", false, true);
  key_with(&mut stage, "PageDown", false, true);
  key_with(&mut stage, "PageDown", true, true);
  close_shortcut(&mut stage);
  assert_eq!(value.get(), 0);
  assert!(said(&told).is_empty());
}

#[test]
fn the_tabs_are_one_stop_whose_arrows_select_and_the_add_button_another() {
  let (mut stage, value, _) = told_stage(true, false);
  value.set(1);
  stage.pass();
  stage.key("Tab");
  let focused = stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned));
  assert_eq!(focused.as_deref(), Some("theme"), "the selected tab");
  stage.key("ArrowRight");
  assert_eq!(value.get(), 2);
  stage.key("ArrowLeft");
  assert_eq!(value.get(), 1);
  stage.key("Tab");
  let focused = stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned));
  assert_eq!(focused.as_deref(), Some("strip-add"), "Tab goes on to the add button");
}

#[test]
fn a_disabled_tab_is_passed_by_and_a_disabled_strip_takes_nothing() {
  let value = Signal::new(0_u8);
  let bound = value.clone();
  let mut stage = staged(move || {
    WindowTabs::new(
      bound.clone(),
      [
        WindowTab::new(0, "blur.rs").id("blur"),
        WindowTab::new(1, "theme.rs").id("theme").disabled(true),
        WindowTab::new(2, "canvas.rs").id("canvas"),
      ],
    )
  });
  key_with(&mut stage, "Tab", false, true);
  assert_eq!(value.get(), 2, "the disabled one is passed by");
  stage.click("theme");
  assert_eq!(value.get(), 2, "and not selected by a press");
  let theme = stage.app.theme().clone();
  let disabled = theme.resolve::<WindowTabs>(&Case::new().state(WindowTabsState::Disabled, true), None);
  assert_eq!(
    disabled.part(WindowTabsPart::Tab).rest().opacity,
    Some(Opacity::Level(OpacityLevel::Disabled))
  );

  let (closed, heard) = (Arc::new(AtomicUsize::new(0)), value.clone());
  let count = closed.clone();
  let mut stage = staged(move || {
    let count = count.clone();
    WindowTabs::new(heard.clone(), files())
      .disabled(true)
      .on_close(move |_| {
        count.fetch_add(1, Ordering::SeqCst);
      })
  });
  value.set(0);
  stage.pass();
  stage.click("canvas");
  stage.click("blur-close");
  key_with(&mut stage, "Tab", false, true);
  close_shortcut(&mut stage);
  assert_eq!(value.get(), 0);
  assert_eq!(closed.load(Ordering::SeqCst), 0);
}

#[test]
fn a_press_on_a_tab_its_close_or_the_add_button_goes_no_further_and_on_the_strips_own_room_it_does() {
  let pressed = Arc::new(AtomicUsize::new(0));
  let count = pressed.clone();
  let mut stage = wide(move |_| {
    let count = count.clone();
    // A title bar that drags the window from a press.
    Row::new()
      .id("bar")
      .padding(8.0)
      .on_mouse_down(move |_: MouseEvent| {
        count.fetch_add(1, Ordering::SeqCst);
      })
      .child(board())
      .into()
  });
  for id in ["canvas", "theme-close", "strip-add"] {
    let (x, y) = stage.center(id);
    stage.tree.mouse_down(x, y, MouseButton::Left);
    stage.tree.mouse_up(x, y, MouseButton::Left);
    stage.pass();
    assert_eq!(pressed.load(Ordering::SeqCst), 0, "{id}");
  }
  // The room of the separator between theme.rs and canvas.rs, and above the add button, is the strip's own.
  let (x, y, ..) = stage.bounds("strip");
  for (at_x, at_y) in [(x + 256.5, y + 16.0), (x + 400.0, y + 1.0)] {
    stage.tree.mouse_down(at_x, at_y, MouseButton::Left);
    stage.tree.mouse_up(at_x, at_y, MouseButton::Left);
  }
  assert_eq!(pressed.load(Ordering::SeqCst), 2);
}

#[test]
fn a_drag_along_the_strip_selects_the_tab_and_tells_where_it_was_let_go() {
  for mounted in [false, true] {
    let (mut stage, value, told) = told_stage(true, mounted);
    drag(&mut stage, "canvas", -260.0, 0.0);
    assert_eq!(
      said(&told),
      ["change 2 (the signal has 2)", "reorder 2 0"],
      "mounted: {mounted}"
    );
    assert_eq!(value.get(), 2);
    // Over a tab's room by less than half its width it stays.
    drag(&mut stage, "blur", 50.0, 0.0);
    assert_eq!(said(&told), ["change 0 (the signal has 0)"], "mounted: {mounted}");
    // A move of a pixel or two is a press.
    drag(&mut stage, "theme", 2.0, 0.0);
    assert_eq!(said(&told), ["change 1 (the signal has 1)"], "mounted: {mounted}");
  }
}

#[test]
fn a_mounted_tab_follows_the_pointer_while_it_is_dragged() {
  let (mut stage, ..) = told_stage(true, true);
  let (x, y, ..) = stage.bounds("blur");
  let (cx, cy) = stage.center("blur");
  stage.tree.mouse_move(cx, cy);
  stage.tree.mouse_down(cx, cy, MouseButton::Left);
  for step in 1..=8 {
    stage.tree.mouse_move(cx + 20.0 * step as f32, cy);
    stage.pass();
  }
  let (now, now_y, width, _) = stage.bounds("blur");
  assert!(
    close_to(now, x + 160.0) && close_to(now_y, y),
    "under the pointer: {now}"
  );
  assert_eq!(width, 128.0);
  // The others made room for it where it would be let go: theme.rs went back to the start.
  let (theme, ..) = stage.bounds("theme");
  assert!(close_to(theme, x), "{theme}");
  assert_eq!(stage.fill_color("blur"), Some(light(PaletteColor::Background)));
  stage.tree.mouse_up(cx + 160.0, cy, MouseButton::Left);
  stage.pass();
}

#[test]
fn a_tab_let_go_away_from_the_strip_is_detached() {
  for mounted in [false, true] {
    let (mut stage, _, told) = told_stage(true, mounted);
    drag(&mut stage, "canvas", 0.0, 120.0);
    assert_eq!(
      said(&told),
      ["change 2 (the signal has 2)", "detach 2"],
      "mounted: {mounted}"
    );
    // Let go within the strip's height of it, it is not.
    drag(&mut stage, "blur", 0.0, 30.0);
    assert_eq!(said(&told), ["change 0 (the signal has 0)"], "mounted: {mounted}");
    // Out of the window over its top edge, it is.
    drag(&mut stage, "theme", 0.0, -30.0);
    assert_eq!(
      said(&told),
      ["change 1 (the signal has 1)", "detach 1"],
      "mounted: {mounted}"
    );
    // And out of the window past its end edge, beside the strip: the engine says the pointer is out of it.
    drag(&mut stage, "canvas", 3000.0, 0.0);
    assert_eq!(
      said(&told),
      ["change 2 (the signal has 2)", "detach 2"],
      "mounted: {mounted}"
    );
  }
}

#[test]
fn a_label_longer_than_its_room_ends_in_an_ellipsis_and_the_close_slot_keeps_its_place() {
  let long = "a_file_whose_name_is_much_longer_than_a_tab.rs";
  let mut stage =
    staged(move || WindowTabs::controlled(0, [WindowTab::new(0, long).icon(icons::FILE_CODE).id("long")], |_| {}));
  let (x, _, width, _) = stage.bounds("long");
  assert_eq!(width, 128.0, "the tab keeps its width");
  let cut = stage.cut_text_quad(long).expect("the label is painted");
  assert!(
    matches!(&cut.content, QuadContent::Text { text, .. } if text.ends_with('…')),
    "cut with an ellipsis"
  );
  assert!(
    cut.x + cut.width <= x + 104.0 - 8.0 + 0.5,
    "it ends before the close slot"
  );
  let (close_x, _, close_width, _) = stage.bounds("long-close");
  assert!(close_to(close_x, x + 104.0) && close_width == 18.0);
}

/// `count` tabs of ids `t0`.. in a room `room` wide, the first selected.
fn many(count: usize, room: f32, mounted: bool) -> (Stage, Signal<usize>) {
  many_in(count, room, mounted, false)
}

/// As [`many`], in a column of `room` where `column`, in a row of it where not.
fn many_in(count: usize, room: f32, mounted: bool, column: bool) -> (Stage, Signal<usize>) {
  let value = Signal::new(0_usize);
  let bound = value.clone();
  let stage = wide(move |ctx| {
    let tabs = WindowTabs::new(
      bound.clone(),
      (0..count).map(|index| WindowTab::new(index, &format!("{index}.rs")).id(format!("t{index}"))),
    )
    .id("strip")
    .on_add(|| {});
    let tabs = if mounted { tabs.mount(ctx) } else { tabs.into() };
    if column {
      lurq::components::Column::new().width(room).child(tabs).into()
    } else {
      Row::new().width(room).child(tabs).into()
    }
  });
  (stage, value)
}

#[test]
fn in_a_column_too_the_tabs_share_its_room_and_the_strip_is_no_wider() {
  let (mut stage, _) = many_in(4, 400.0, false, true);
  let widths: Vec<f32> = (0..4).map(|index| stage.bounds(&format!("t{index}")).2).collect();
  assert!(
    widths
      .iter()
      .all(|width| *width < 128.0 && *width >= 72.0 && close_to(*width, widths[0])),
    "{widths:?}"
  );
  let (_, _, strip, _) = stage.bounds("strip");
  assert!(close_to(strip, 400.0), "{strip}");
  let (add, _, add_width, _) = stage.bounds("strip-add");
  assert!(close_to(add + add_width, 400.0), "the add button stays at the end");
}

#[test]
fn a_wheel_turned_up_and_down_over_the_strip_scrolls_its_tabs_sideways() {
  let (mut stage, _) = many(10, 400.0, true);
  let (x0, ..) = stage.bounds("t0");
  let (cx, cy) = stage.center("t1");
  stage.tree.mouse_move(cx, cy);
  stage
    .tree
    .scroll(cx, cy, 0.0, -60.0, lurq::app::events::ScrollPhase::Scroll);
  stage.pass();
  let (x1, ..) = stage.bounds("t0");
  assert!(x1 < x0 - 30.0, "the tabs moved along: {x0} -> {x1}");
}

#[test]
fn tabs_share_a_room_too_small_down_to_their_least_width_and_then_the_strip_scrolls() {
  // Four tabs in 400: each gives way alike, and the strip fills the room and no more.
  let (mut stage, _) = many(4, 400.0, false);
  let widths: Vec<f32> = (0..4).map(|index| stage.bounds(&format!("t{index}")).2).collect();
  assert!(
    widths
      .iter()
      .all(|width| *width < 128.0 && *width >= 72.0 && close_to(*width, widths[0])),
    "{widths:?}"
  );
  let (_, _, strip, _) = stage.bounds("strip");
  assert!(close_to(strip, 400.0), "{strip}");
  let (add, _, add_width, _) = stage.bounds("strip-add");
  assert!(close_to(add + add_width, 400.0), "the add button stays at the end");

  // Ten: each at 72, and the list scrolls between the strip's start and the add button.
  let (mut stage, value) = many(10, 400.0, true);
  for index in 0..10 {
    assert_eq!(stage.bounds(&format!("t{index}")).2, 72.0, "t{index}");
  }
  let (strip_x, _, strip, _) = stage.bounds("strip");
  assert!(strip <= 400.0 + 0.5);
  let (add, ..) = stage.bounds("strip-add");
  // Selected from the keys, the last is brought into view: its label is painted before the add button.
  let painted = |stage: &Stage| stage.cut_text_quad("9.rs").map(|quad| quad.x + quad.width);
  assert!(painted(&stage).is_none_or(|end| end > add), "not in view at first");
  key_with(&mut stage, "Tab", true, true);
  assert_eq!(value.get(), 9);
  stage.pass();
  let end = painted(&stage).expect("the label of the last is painted");
  assert!(end <= add + 0.5 && end > strip_x, "in view: {end}");
}

#[test]
fn a_forced_focus_draws_the_ring_with_nothing_pointing_at_it() {
  let stage = staged(|| board().force(Some(Interaction::FocusVisible)));
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  let stage = staged(board);
  assert!(!stage.has_ring(light(PaletteColor::Ring)));
}

#[test]
fn a_theme_gives_the_tabs_another_anatomy_and_they_still_select() {
  let asked = Arc::new(Mutex::new(Vec::new()));
  let told = asked.clone();
  let app = App::new();
  // The list alone, with a word before it: other nodes, the same behavior.
  let renderer: WindowTabsRenderer = Arc::new(|view: WindowTabsView| {
    Row::new()
      .child(view.part(WindowTabsPart::Label).text("FILES"))
      .child(view.list())
      .into()
  });
  app.theme().set_renderer::<WindowTabs>(renderer);
  let mut stage = Stage::in_app(app, move |_| {
    let told = told.clone();
    let tabs: Element = WindowTabs::controlled(0, files(), move |value| told.lock().unwrap().push(*value)).into();
    Row::new().padding(8.0).child(tabs).into()
  });
  assert!(stage.paints("FILES"));
  assert_eq!(stage.fill_color("blur"), Some(light(PaletteColor::Background)));
  stage.click("canvas");
  assert_eq!(*asked.lock().unwrap(), [2]);
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn tooling_reads_a_tab_list_of_tabs_with_their_close_buttons() {
  let read = |stage: &mut Stage, id: &str, what: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(what))
  };
  let mut stage = staged(|| board().label("Open files"));
  assert_eq!(read(&mut stage, "strip-list", "a11y_role").as_deref(), Some("tablist"));
  assert_eq!(
    read(&mut stage, "strip-list", "a11y_name").as_deref(),
    Some("Open files")
  );
  assert_eq!(read(&mut stage, "blur", "a11y_role").as_deref(), Some("tab"));
  assert_eq!(read(&mut stage, "blur", "a11y_name").as_deref(), Some("blur.rs"));
  assert_eq!(read(&mut stage, "blur", "a11y_selected").as_deref(), Some("true"));
  assert_eq!(read(&mut stage, "canvas", "a11y_selected").as_deref(), Some("false"));
  assert_eq!(
    read(&mut stage, "canvas-close", "a11y_name").as_deref(),
    Some("Close canvas.rs")
  );
  assert_eq!(read(&mut stage, "strip-add", "a11y_name").as_deref(), Some("New tab"));

  let mut stage = staged(|| board().close_label("Fermer").add_label("Nouvel onglet"));
  assert_eq!(
    read(&mut stage, "canvas-close", "a11y_name").as_deref(),
    Some("Fermer canvas.rs")
  );
  assert_eq!(
    read(&mut stage, "strip-add", "a11y_name").as_deref(),
    Some("Nouvel onglet")
  );
}

#[test]
fn a_press_that_selects_another_tab_leaves_the_focus_on_it_and_the_arrows_go_on_from_it() {
  let (mut stage, value, _) = told_stage(true, true);
  let node = |stage: &mut Stage| stage.tree.get_element_by_id_mut("canvas").unwrap().node_id();
  let before = node(&mut stage);
  stage.click("canvas");
  assert_eq!(value.get(), 2);
  assert_eq!(
    node(&mut stage),
    before,
    "the tab is the same element as it becomes the selected one"
  );
  let focused = stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned));
  assert_eq!(focused.as_deref(), Some("canvas"), "the pressed tab has the focus");
  stage.key("ArrowLeft");
  let focused = stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned));
  assert_eq!(focused.as_deref(), Some("theme"), "and the arrows go on from it");
}
