//! Tabs that keep their places as the selection moves (`Tabs::reserve_selected_width`), an icon after a label
//! (`Tab::trailing_icon`), and tabs at the end of the line (`Tab::at_end`).

use lurq::{
  components::Column,
  core::Signal,
  ui::{Recipe, Selector, Tab, Tabs, TabsPart, TabsState, TabsVariant, icons},
};

use super::stage::Stage;

fn laid_out(stage: &mut Stage, id: &str) -> bool {
  stage
    .tree
    .get_element_by_id_mut(id)
    .and_then(|element| element.bounds())
    .is_some()
}

/// Mounted line tabs General, Appearance and Shortcuts, in a room 600 wide; the selected label is drawn at 20, so a
/// selection that took its room would move the tabs after it.
fn line_tabs(section: &Signal<u8>, reserve: bool) -> Stage {
  let section = section.clone();
  Stage::first_frame(800.0, 200.0, move |ctx| {
    let rules = Recipe::new().when(Selector::new().state(TabsState::Selected), TabsPart::Label, |s| {
      s.font_size(20.0)
    });
    Column::new()
      .width(600.0)
      .child(
        Tabs::new(
          section.clone(),
          [
            Tab::new(0, "General").id("general"),
            Tab::new(1, "Appearance").id("appearance"),
            Tab::new(2, "Shortcuts").id("shortcuts"),
          ],
        )
        .variant(TabsVariant::Line)
        .rules(&rules)
        .reserve_selected_width(reserve)
        .id("tabs")
        .mount(ctx),
      )
      .into()
  })
}

#[test]
fn a_list_that_reserves_the_selected_width_moves_no_tab_as_the_selection_moves() {
  let section = Signal::new(0u8);
  let mut stage = line_tabs(&section, true);
  let before = stage.bounds("shortcuts").0;
  section.set(1);
  stage.frame();
  assert_eq!(stage.bounds("shortcuts").0, before, "Shortcuts stays where it was");

  let section = Signal::new(0u8);
  let mut stage = line_tabs(&section, false);
  let before = stage.bounds("shortcuts").0;
  section.set(1);
  stage.frame();
  assert!(stage.bounds("shortcuts").0 > before, "without it the wider label pushes it on");
}

#[test]
fn a_trailing_icon_stands_after_the_label_and_its_count() {
  let section = Signal::new(0u8);
  let held = section.clone();
  let mut stage = Stage::first_frame(800.0, 200.0, move |ctx| {
    Column::new()
      .width(600.0)
      .child(
        Tabs::new(
          held.clone(),
          [
            Tab::new(0, "General").id("general"),
            Tab::new(1, "Billing")
              .id("billing")
              .count(3)
              .trailing_icon(icons::LOCK),
          ],
        )
        .variant(TabsVariant::Line)
        .id("tabs")
        .mount(ctx),
      )
      .into()
  });
  let (count_x, _, count_width, _) = stage.bounds("billing-count");
  let (icon_x, _, icon_width, icon_height) = stage.bounds("billing-trailing_icon");
  let (tab_x, _, tab_width, _) = stage.bounds("billing");
  assert!(icon_x >= count_x + count_width, "after the count");
  assert!(icon_x + icon_width <= tab_x + tab_width + 0.5, "inside the tab");
  assert_eq!((icon_width, icon_height), (14.0, 14.0), "the small icon's size");
}

/// Mounted line tabs of four lists and a Settings tab at the end, in a room `width` wide whose id is `room`.
fn with_settings_at_end(width: &Signal<f32>, section: &Signal<u8>) -> Stage {
  let (width, section) = (width.clone(), section.clone());
  Stage::first_frame(1000.0, 200.0, move |ctx| {
    let lists = ["Inbox", "Drafts", "Sent", "Archive"]
      .into_iter()
      .enumerate()
      .map(|(index, name)| Tab::new(index as u8, name).id(name.to_lowercase()));
    let tabs = lists.chain([Tab::new(9, "Settings").id("settings").at_end(true)]);
    Column::new()
      .size(1000.0, 200.0)
      .child(
        Column::new().id("room").width(width.get()).child(
          Tabs::new(section.clone(), tabs)
            .variant(TabsVariant::Line)
            .note("Read 2 minutes ago")
            .id("tabs")
            .mount(ctx),
        ),
      )
      .into()
  })
}

#[test]
fn a_tab_at_the_end_stands_at_the_lines_end_after_the_note() {
  let (width, section) = (Signal::new(800.0f32), Signal::new(0u8));
  let mut stage = with_settings_at_end(&width, &section);
  assert!(!laid_out(&mut stage, "tabs-more"), "the list fits");
  let (room_x, _, room_width, _) = stage.bounds("room");
  let (settings_x, _, settings_width, _) = stage.bounds("settings");
  let (archive_x, _, archive_width, _) = stage.bounds("archive");
  let (note_x, _, note_width, _) = stage.bounds("tabs-note");
  assert!(
    (settings_x + settings_width - (room_x + room_width)).abs() < 0.5,
    "at the end of the line"
  );
  assert!(note_x >= archive_x + archive_width, "the note after the tabs at the start");
  assert!(settings_x >= note_x + note_width, "and the tab after the note");

  stage.click("settings");
  assert_eq!(section.get_untracked(), 9, "it is a tab: a press selects it");
  stage.click("archive");
  stage.key("ArrowRight");
  assert_eq!(section.get_untracked(), 9, "and the arrows reach it after the tabs at the start");
}

#[test]
fn tabs_at_the_end_count_in_what_the_list_needs_to_fit() {
  let (width, section) = (Signal::new(800.0f32), Signal::new(0u8));
  let mut stage = with_settings_at_end(&width, &section);
  assert!(!laid_out(&mut stage, "tabs-more"));
  let settings_width = stage.bounds("settings").2;
  let archive = stage.bounds("archive");
  let room_x = stage.bounds("room").0;
  // The tabs at the start, the gap of the list (28) and the tab at the end: what the list needs.
  let needs = archive.0 + archive.2 - room_x + 28.0 + settings_width;
  for (room, overflows) in [(needs + 20.0, false), (needs - 10.0, true), (needs + 1.0, false)] {
    width.set(room);
    stage.frame();
    assert_eq!(laid_out(&mut stage, "tabs-more"), overflows, "at {room}");
  }
}
