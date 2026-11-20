//! Pressables that Tab passes by (`tab_stop(false)`) and a press that asks for a place beside what is open
//! (`on_open_aside`), on every component that has them: a button, a list's row, tabs, a segmented control, a
//! breadcrumb and a tag.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::events::{MouseButton, MouseEvent},
  components::{Column, Row, TextInput},
  core::Signal,
  node::Element,
  ui::{Breadcrumb, BreadcrumbStep, Button, List, ListItem, SegmentedControl, Tab, Tabs, Tag},
};

use super::stage::Stage;

fn counter() -> (Arc<AtomicUsize>, impl Fn() + Clone + Send + Sync + 'static) {
  let count = Arc::new(AtomicUsize::new(0));
  let counted = count.clone();
  (count, move || {
    counted.fetch_add(1, Ordering::SeqCst);
  })
}

fn count(counter: &AtomicUsize) -> usize {
  counter.load(Ordering::SeqCst)
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// A field first, so that Tab and a press have something to leave the focus in.
fn with_field(stage_content: impl Fn() -> Element + Send + Sync + 'static) -> Stage {
  Stage::new(move |_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        TextInput::new(Signal::new(String::new()))
          .id("field")
          .width(160.0)
          .tab_index(0),
      )
      .child(stage_content())
      .child(Button::new("Next").id("next"))
      .into()
  })
}

fn tab(stage: &mut Stage) {
  stage.key("Tab");
}

/// From the field, Tab goes straight to the button after what is between.
fn tab_passes_by(stage: &mut Stage) {
  stage.click("field");
  assert_eq!(focused(stage).as_deref(), Some("field"));
  tab(stage);
  assert_eq!(focused(stage).as_deref(), Some("next"), "Tab passes it by");
}

fn middle_click(stage: &mut Stage, id: &str) {
  let (x, y) = stage.center(id);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Middle);
  stage.tree.mouse_up(x, y, MouseButton::Middle);
  stage.pass();
}

/// A press with the platform's key held, Ctrl or Cmd on Apple's platforms, and Shift if `shift`.
fn command_click(stage: &mut Stage, id: &str, shift: bool) {
  let mac = cfg!(any(target_os = "macos", target_os = "ios"));
  let (x, y) = stage.center(id);
  stage.tree.mouse_move(x, y);
  stage
    .tree
    .mouse_down_with_meta(x, y, MouseButton::Left, shift, !mac, false, mac);
  stage
    .tree
    .mouse_up_with_meta(x, y, MouseButton::Left, shift, !mac, false, mac);
  stage.pass();
}

#[test]
fn a_button_off_tab_is_passed_by_keeps_the_focus_and_is_pressed_by_enter_once_focused() {
  let (presses, press) = counter();
  let mut stage = with_field(move || {
    let press = press.clone();
    Row::new()
      .child(
        Button::new("Bold")
          .id("bold")
          .tab_stop(false)
          .on_click(move |_| press()),
      )
      .into()
  });
  tab_passes_by(&mut stage);

  stage.click("field");
  stage.click("bold");
  assert_eq!(count(&presses), 1, "a click presses it");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("field"),
    "and leaves the focus in the field"
  );

  stage.tree.get_element_by_id_mut("bold").unwrap().focus();
  stage.pass();
  assert_eq!(focused(&stage).as_deref(), Some("bold"), "the app can focus it");
  stage.key("Enter");
  stage.key(" ");
  assert_eq!(count(&presses), 3, "Enter and Space press it");
}

#[test]
fn a_button_asked_for_aside_calls_its_handler_alone_and_a_plain_press_still_presses_it() {
  let (presses, press) = counter();
  let (asides, aside) = counter();
  let shifted = Arc::new(AtomicUsize::new(0));
  let told = shifted.clone();
  let mut stage = with_field(move || {
    let (press, aside, told) = (press.clone(), aside.clone(), told.clone());
    Row::new()
      .child(
        Button::new("Open")
          .id("open")
          .on_click(move |_| press())
          .on_open_aside(move |event: MouseEvent| {
            if event.shift {
              told.fetch_add(1, Ordering::SeqCst);
            }
            aside();
          }),
      )
      .into()
  });
  stage.click("field");

  middle_click(&mut stage, "open");
  assert_eq!(
    (count(&presses), count(&asides)),
    (0, 1),
    "the middle button asks for it aside"
  );
  command_click(&mut stage, "open", false);
  assert_eq!(
    (count(&presses), count(&asides)),
    (0, 2),
    "so does a press with the key held"
  );
  command_click(&mut stage, "open", true);
  assert_eq!(
    (count(&asides), count(&shifted)),
    (3, 1),
    "the handler is told Shift was held"
  );
  assert_eq!(
    focused(&stage).as_deref(),
    Some("field"),
    "a press aside takes no focus, though the button is a stop of Tab"
  );

  stage.click("open");
  assert_eq!((count(&presses), count(&asides)), (1, 3), "a plain press presses it");
}

#[test]
fn a_press_aside_does_not_reach_a_row_around_the_button() {
  let (rows, row) = counter();
  let (asides, aside) = counter();
  let mut stage = Stage::new(move |_| {
    let (row, aside) = (row.clone(), aside.clone());
    Column::new()
      .size(400.0, 300.0)
      .child(
        List::new().item(
          ListItem::new("Report")
            .id("row")
            .on_click(move |_| row())
            .on_open_aside(|_| {})
            .trailing(Button::new("Open").id("open").on_open_aside(move |_| aside())),
        ),
      )
      .into()
  });
  middle_click(&mut stage, "open");
  command_click(&mut stage, "open", false);
  assert_eq!((count(&rows), count(&asides)), (0, 2));
}

#[test]
fn a_row_off_tab_is_passed_by_and_a_row_asked_for_aside_is_not_pressed() {
  let (presses, press) = counter();
  let (asides, aside) = counter();
  let mut stage = with_field(move || {
    let (press, aside) = (press.clone(), aside.clone());
    List::new()
      .item(
        ListItem::new("Report")
          .id("row")
          .tab_stop(false)
          .on_click(move |_| press())
          .on_open_aside(move |_| aside()),
      )
      .into()
  });
  tab_passes_by(&mut stage);

  stage.click("field");
  middle_click(&mut stage, "row");
  command_click(&mut stage, "row", false);
  assert_eq!((count(&presses), count(&asides)), (0, 2));
  stage.click("row");
  assert_eq!((count(&presses), count(&asides)), (1, 2), "a plain press presses it");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("field"),
    "and leaves the focus in the field"
  );

  stage.tree.get_element_by_id_mut("row").unwrap().focus();
  stage.pass();
  stage.key("Enter");
  assert_eq!(count(&presses), 2, "focused by the app, Enter presses it");
}

#[test]
fn tabs_off_tab_are_passed_by_and_a_tab_asked_for_aside_is_not_selected() {
  let section = Signal::new("general");
  let shown = section.clone();
  let (asides, aside) = counter();
  let mut stage = with_field(move || {
    let aside = aside.clone();
    Tabs::new(
      shown.clone(),
      [
        Tab::new("general", "General").id("general"),
        Tab::new("advanced", "Advanced")
          .id("advanced")
          .on_open_aside(move |_| aside()),
      ],
    )
    .tab_stop(false)
    .into()
  });
  tab_passes_by(&mut stage);

  stage.click("field");
  middle_click(&mut stage, "advanced");
  command_click(&mut stage, "advanced", false);
  assert_eq!(count(&asides), 2);
  assert_eq!(section.get_untracked(), "general", "a press aside selects nothing");

  stage.click("advanced");
  assert_eq!(section.get_untracked(), "advanced", "a plain press selects it");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("field"),
    "and leaves the focus in the field"
  );

  stage.tree.get_element_by_id_mut("advanced").unwrap().focus();
  stage.pass();
  stage.key("ArrowLeft");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("general"),
    "the arrows move along the list"
  );
  assert_eq!(section.get_untracked(), "general", "and select as before");
}

#[test]
fn a_segmented_control_off_tab_is_passed_by_and_a_press_chooses_and_keeps_the_focus() {
  let period = Signal::new(1u8);
  let shown = period.clone();
  let mut stage = with_field(move || {
    SegmentedControl::new(shown.clone(), [(1u8, "Day"), (7u8, "Week")])
      .id("period")
      .tab_stop(false)
      .into()
  });
  tab_passes_by(&mut stage);

  stage.click("field");
  let (x, y) = stage
    .text_quad("Week")
    .map(|quad| (quad.x + 2.0, quad.y + 2.0))
    .unwrap();
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, MouseButton::Left);
  stage.tree.mouse_up(x, y, MouseButton::Left);
  stage.pass();
  assert_eq!(period.get_untracked(), 7, "a press chooses it");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("field"),
    "and leaves the focus in the field"
  );
}

#[test]
fn a_breadcrumb_off_tab_is_passed_by_and_a_step_asked_for_aside_is_not_followed() {
  let (followed, follow) = counter();
  let (asides, aside) = counter();
  let mut stage = with_field(move || {
    let (follow, aside) = (follow.clone(), aside.clone());
    Breadcrumb::new([
      BreadcrumbStep::new("Home")
        .id("home")
        .on_select(follow)
        .on_open_aside(move |_| aside()),
      BreadcrumbStep::new("Report"),
    ])
    .tab_stop(false)
    .into()
  });
  tab_passes_by(&mut stage);

  stage.click("field");
  middle_click(&mut stage, "home");
  command_click(&mut stage, "home", false);
  assert_eq!((count(&followed), count(&asides)), (0, 2));
  stage.click("home");
  assert_eq!((count(&followed), count(&asides)), (1, 2), "a plain press follows it");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("field"),
    "and leaves the focus in the field"
  );
}

#[test]
fn a_tags_remove_button_off_tab_is_passed_by_and_still_takes_the_tag_away() {
  let (removed, remove) = counter();
  let mut stage = with_field(move || {
    let remove = remove.clone();
    Row::new()
      .child(Tag::new("rust").id("rust").tab_stop(false).on_remove(remove))
      .into()
  });
  tab_passes_by(&mut stage);

  stage.click("field");
  stage.click("rust-remove");
  assert_eq!(count(&removed), 1);
  assert_eq!(
    focused(&stage).as_deref(),
    Some("field"),
    "a press leaves the focus in the field"
  );

  stage.tree.get_element_by_id_mut("rust-remove").unwrap().focus();
  stage.pass();
  stage.key("Delete");
  assert_eq!(count(&removed), 2, "focused by the app, Delete takes it away");
}

/// Off Tab, a pressable is the same thing to what reads the tree.
#[cfg(feature = "mcp")]
#[test]
fn a_pressable_off_tab_keeps_its_role_and_name() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .child(Button::new("Bold").id("bold").tab_stop(false))
      .child(List::new().item(ListItem::new("Report").id("row").tab_stop(false).on_click(|_| {})))
      .child(Tag::new("rust").id("rust").tab_stop(false).on_remove(|| {}))
      .into()
  });
  let mut said = |id: &str, name: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(name))
  };
  assert_eq!(said("row", "a11y_role").as_deref(), Some("button"));
  assert_eq!(said("row", "a11y_name").as_deref(), Some("Report"));
  assert_eq!(said("rust-remove", "a11y_role").as_deref(), Some("button"));
  assert_eq!(said("rust-remove", "a11y_name").as_deref(), Some("Remove rust"));
}
