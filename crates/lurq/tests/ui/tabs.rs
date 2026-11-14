//! Tabs: the rail, the line, the pills and the list down the side as the
//! board draws them, what a press or a key selects, the panel of the
//! selected tab, the overflowing form, and a theme's other anatomy.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    theme::{OpacityLevel, PaletteColor, ThemePalette},
  },
  components::{Button, Column, Row, Text},
  core::Signal,
  layout::quad::{Quad, QuadContent},
  node::{Element, color::Color},
  ui::{
    Case, Interaction, Opacity, Tab, Tabs, TabsOrientation, TabsOverflow, TabsPart, TabsRenderer, TabsState,
    TabsVariant, TabsView,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The tabs of the board's default and pill drawings, with ids `general`, `appearance`, `shortcuts`.
fn settings() -> [Tab<u8>; 3] {
  [
    Tab::new(0, "General").id("general"),
    Tab::new(1, "Appearance").id("appearance"),
    Tab::new(2, "Shortcuts").id("shortcuts"),
  ]
}

/// The filled rectangle of `color` that is `height` high, if one is painted.
fn filled(stage: &Stage, color: Color, height: f32) -> Option<Quad> {
  stage.quads().into_iter().find(|quad| {
    matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color)
      && (quad.height - height).abs() < 0.01
  })
}

/// The quad with a line at its bottom of `color`, as wide as `width`, if one is painted.
fn underlined(stage: &Stage, color: Color, width: f32) -> Option<Quad> {
  stage.quads().into_iter().find(|quad| {
    quad
      .border
      .and_then(|border| border.bottom)
      .is_some_and(|side| side.color == color && side.width == width)
  })
}

/// The quad with a line at its start of `color`, as wide as `width`, if one is painted.
fn start_lined(stage: &Stage, color: Color, width: f32) -> Option<Quad> {
  stage.quads().into_iter().find(|quad| {
    quad
      .border
      .and_then(|border| border.left)
      .is_some_and(|side| side.color == color && side.width == width)
  })
}

#[test]
fn the_default_tabs_are_a_muted_rail_with_the_selected_tab_lifted_out_of_it() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(Tabs::controlled(0, settings(), |_| {}))
      .into()
  });
  for id in ["general", "appearance", "shortcuts"] {
    let (_, _, _, height) = stage.bounds(id);
    assert_eq!(height, 28.0, "{id}");
  }
  let (x, y, width, _) = stage.bounds("general");
  let (next, ..) = stage.bounds("appearance");
  assert!((next - (x + width) - 2.0).abs() < 0.01, "2 apart");

  // The rail is muted, 3 around the tabs.
  let rail = filled(&stage, light(PaletteColor::Muted), 34.0).expect("a muted rail 34 high");
  assert!((rail.x - (x - 3.0)).abs() < 0.01 && (rail.y - (y - 3.0)).abs() < 0.01);
  assert!(
    rail.border_radius.is_some_and(|radius| radius.top_left == 8.0),
    "large radius"
  );

  // The selected tab is the background's color with the small shadow; the others have no fill.
  assert_eq!(stage.fill_color("general"), Some(light(PaletteColor::Background)));
  assert_eq!(stage.fill_color("appearance"), None);
  assert_eq!(stage.text("General"), (light(PaletteColor::Foreground), 13.0));
  assert_eq!(stage.text("Appearance"), (light(PaletteColor::MutedForeground), 13.0));
}

#[test]
fn a_count_follows_its_label_and_an_icon_comes_before_it() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(Tabs::controlled(
        0,
        [
          Tab::new(0, "General").count(12).id("general"),
          Tab::new(1, "Appearance")
            .count(4)
            .icon(lurq::ui::icons::SETTINGS)
            .id("appearance"),
        ],
        |_| {},
      ))
      .into()
  });
  let label = stage.text_quad("General").unwrap();
  let count = stage.text_quad("12").unwrap();
  assert!(
    (count.x - (label.x + label.width) - 6.0).abs() < 0.5,
    "6 after the label"
  );
  assert_eq!(stage.text("12"), (light(PaletteColor::MutedForeground), 12.0));
  // The icon is a glyph of the icon font, in the tab past its padding of 12 and before the label.
  let label = stage.text_quad("Appearance").unwrap();
  let (x, ..) = stage.bounds("appearance");
  assert!(
    stage
      .quads()
      .iter()
      .any(|quad| matches!(&quad.content, QuadContent::Text { .. })
        && quad.x >= x + 11.5
        && quad.x + quad.width <= label.x),
    "an icon before the label"
  );
}

#[test]
fn the_line_tabs_are_a_row_over_a_line_with_the_selected_one_underlined() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(600.0)
      .child(
        Tabs::controlled(
          "memory",
          [
            Tab::new("all", "All").count(7).id("all"),
            Tab::new("profiles", "Profiles").count(4).id("profiles"),
            Tab::new("memory", "Memory").count(9).id("memory"),
            Tab::new("archived", "Archived").count(0).separated(true).id("archived"),
          ],
          |_| {},
        )
        .variant(TabsVariant::Line),
      )
      .into()
  });
  let (x, _, width, height) = stage.bounds("all");
  assert_eq!(height, 44.0);
  let (next, ..) = stage.bounds("profiles");
  assert!((next - (x + width) - 28.0).abs() < 0.01, "28 apart");

  // The label is 14 in the muted foreground, the selected one in the foreground; the count beside it is 13.
  assert_eq!(stage.text("All"), (light(PaletteColor::MutedForeground), 14.0));
  assert_eq!(stage.text("Memory"), (light(PaletteColor::Foreground), 14.0));
  assert_eq!(stage.text("9").1, 13.0);

  // The selected tab is underlined with 2 in the foreground, as wide as it is; the list with 1 in the border's
  // color, across the width.
  let (_, _, memory, _) = stage.bounds("memory");
  let mark = underlined(&stage, light(PaletteColor::Foreground), 2.0).expect("the selected tab underlined");
  assert_eq!(mark.width, memory);
  let rail = underlined(&stage, light(PaletteColor::Border), 1.0).expect("a line under the list");
  assert_eq!(rail.width, 600.0);

  // A divider of 1 by 16 in the border's color sets the archived apart, 28 from each side.
  let divider = stage
    .quads()
    .into_iter()
    .find(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Border))
        && (quad.width, quad.height) == (1.0, 16.0)
    })
    .expect("a divider");
  let (memory_x, ..) = stage.bounds("memory");
  let (archived, ..) = stage.bounds("archived");
  assert!((divider.x - (memory_x + memory) - 28.0).abs() < 0.01);
  assert!((archived - (divider.x + 1.0) - 28.0).abs() < 0.01);
}

#[test]
fn the_selected_pill_is_in_the_primary_fill() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(Tabs::controlled(0, settings(), |_| {}).variant(TabsVariant::Pill))
      .into()
  });
  let (x, _, width, height) = stage.bounds("general");
  assert_eq!(height, 30.0);
  let (next, ..) = stage.bounds("appearance");
  assert!((next - (x + width) - 4.0).abs() < 0.01, "4 apart");
  assert_eq!(stage.fill_color("general"), Some(light(PaletteColor::Primary)));
  assert_eq!(stage.fill_color("appearance"), None);
  assert_eq!(stage.text("General"), (light(PaletteColor::PrimaryForeground), 13.0));
  let pill = stage.fill("general").unwrap();
  assert!(
    pill.border_radius.is_some_and(|radius| radius.top_left >= 15.0),
    "round"
  );
}

#[test]
fn the_vertical_line_tabs_run_down_a_line_with_the_selected_one_marked_at_its_start() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(
        Tabs::controlled(
          0,
          [
            Tab::new(0, "General")
              .id("general")
              .panel(|| Text::new("General settings").into()),
            Tab::new(1, "Appearance").id("appearance"),
            Tab::new(2, "Keyboard").id("keyboard"),
          ],
          |_| {},
        )
        .variant(TabsVariant::Line)
        .orientation(TabsOrientation::Vertical),
      )
      .into()
  });
  let (x, y, width, height) = stage.bounds("general");
  assert_eq!(height, 32.0);
  assert!(width >= 180.0, "at least 180 wide: {width}");
  let (next_x, next_y, next_width, _) = stage.bounds("appearance");
  assert_eq!(
    (next_x, next_y, next_width),
    (x, y + 32.0, width),
    "one under the other, as wide"
  );
  assert_eq!(stage.text("General"), (light(PaletteColor::Foreground), 13.0));
  assert_eq!(stage.text("Keyboard"), (light(PaletteColor::MutedForeground), 13.0));

  let mark = start_lined(&stage, light(PaletteColor::Foreground), 2.0).expect("the selected tab marked");
  assert_eq!((mark.x, mark.y, mark.height), (x, y, 32.0));
  assert!(
    start_lined(&stage, light(PaletteColor::Border), 1.0).is_some(),
    "a line down the list"
  );

  // The panel is beside the list, 24 from it.
  let panel = stage.text_quad("General settings").unwrap();
  assert!(
    (panel.x - (x + width) - 24.0).abs() < 0.5,
    "24 beside: {}",
    panel.x - (x + width)
  );
}

#[test]
fn a_press_selects_a_tab_and_a_disabled_or_the_selected_one_asks_for_nothing() {
  let asked = Arc::new(Mutex::new(Vec::new()));
  let told = asked.clone();
  let section = Signal::new(0u8);
  let held = section.clone();
  let mut stage = Stage::new(move |_| {
    let told = told.clone();
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(Tabs::new(
        held.clone(),
        [
          Tab::new(0, "One").id("one"),
          Tab::new(1, "Two").id("two"),
          Tab::new(2, "Three").disabled(true).id("three"),
        ],
      ))
      .child(Tabs::controlled(
        "a",
        [Tab::new("a", "A").id("a"), Tab::new("b", "B").id("b")],
        move |value| told.lock().unwrap().push(value),
      ))
      .into()
  });
  stage.click("two");
  assert_eq!(section.get(), 1);
  stage.click("three");
  assert_eq!(section.get(), 1, "disabled");
  assert_eq!(stage.fill_color("two"), Some(light(PaletteColor::Background)));

  stage.click("a");
  stage.click("b");
  assert_eq!(*asked.lock().unwrap(), ["b"], "the selected one asks for nothing");
}

#[test]
fn the_list_is_one_stop_whose_arrows_home_and_end_select() {
  let section = Signal::new(1u8);
  let held = section.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .padding(8.0)
      .child(Button::new("Before").tab_index(0))
      .child(Tabs::new(
        held.clone(),
        [
          Tab::new(0, "One"),
          Tab::new(1, "Two"),
          Tab::new(2, "Three").disabled(true),
          Tab::new(3, "Four"),
        ],
      ))
      .into()
  });
  // Tab reaches the button before, then the list at its selected tab, with the ring.
  stage.key("Tab");
  stage.key("Tab");
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  // Right selects the next tab, passing the disabled one by; Down is not this list's.
  stage.key("ArrowRight");
  assert_eq!(section.get(), 3);
  stage.key("ArrowDown");
  assert_eq!(section.get(), 3);
  stage.key("Home");
  assert_eq!(section.get(), 0);
  stage.key("End");
  assert_eq!(section.get(), 3);
  stage.key("ArrowLeft");
  assert_eq!(section.get(), 1);
  // Shift+Tab leaves the list at once, for the button before: the list is one stop.
  stage
    .tree
    .key_down("Tab".to_owned(), "Tab".to_owned(), true, false, false);
  stage
    .tree
    .key_up("Tab".to_owned(), "Tab".to_owned(), true, false, false);
  stage.pass();
  stage.key("Tab");
  stage.key("ArrowRight");
  assert_eq!(section.get(), 3, "back in at the selected tab");
}

#[test]
fn a_vertical_list_takes_up_and_down_and_a_manual_one_selects_on_space() {
  let section = Signal::new(0u8);
  let held = section.clone();
  let mut stage = Stage::new(move |_| {
    Row::new()
      .padding(8.0)
      .child(
        Tabs::new(held.clone(), [(0, "One"), (1, "Two"), (2, "Three")])
          .orientation(TabsOrientation::Vertical)
          .manual(true),
      )
      .into()
  });
  stage.key("Tab");
  stage.key("ArrowRight");
  assert_eq!(section.get(), 0, "Right is not a vertical list's");
  stage.key("ArrowDown");
  assert_eq!(section.get(), 0, "manual: the focus moves alone");
  stage.key("ArrowDown");
  stage.key(" ");
  assert_eq!(section.get(), 2);
}

#[test]
fn only_the_selected_tab_has_its_panel_and_tab_goes_from_the_list_into_it() {
  let saved = Arc::new(Mutex::new(0));
  let told = saved.clone();
  let section = Signal::new(0u8);
  let held = section.clone();
  let mut stage = Stage::new(move |_| {
    let told = told.clone();
    Column::new()
      .padding(8.0)
      .child(Tabs::new(
        held.clone(),
        [
          Tab::new(0, "General").id("general").panel(move || {
            let told = told.clone();
            Column::new()
              .child(Text::new("General settings"))
              .child(
                Button::new("Save")
                  .tab_index(0)
                  .on_click(move |_| *told.lock().unwrap() += 1),
              )
              .into()
          }),
          Tab::new(1, "Appearance")
            .id("appearance")
            .panel(|| Text::new("Appearance settings").into()),
        ],
      ))
      .into()
  });
  assert!(stage.paints("General settings"));
  assert!(!stage.paints("Appearance settings"), "built only while selected");

  // Tab goes from the list to the panel, and on to what is in it.
  stage.key("Tab");
  stage.key("Tab");
  stage.key("Tab");
  stage.key(" ");
  assert_eq!(*saved.lock().unwrap(), 1);

  stage.click("appearance");
  assert!(stage.paints("Appearance settings"));
  assert!(!stage.paints("General settings"));
  let (_, y, _, height) = stage.bounds("appearance");
  let panel = stage.text_quad("Appearance settings").unwrap();
  // 16 under the rail, whose padding is 3 under the tab.
  assert!((panel.y - (y + height) - 19.0).abs() < 0.5, "16 under the list");
}

#[test]
fn the_overflowing_list_scrolls_between_its_buttons_and_more_opens_a_menu_of_every_tab() {
  let section = Signal::new(0usize);
  let overflow = TabsOverflow::new(Signal::new(false));
  let (held, kept) = (section.clone(), overflow.clone());
  let names = [
    "Canvas", "Layout", "Text", "Theme", "Router", "Forms", "Motion", "Charts",
  ];
  let mut stage = Stage::new(move |_| {
    let tabs = names
      .iter()
      .enumerate()
      .map(|(index, name)| Tab::new(index, name).id(name.to_lowercase()));
    Column::new()
      .size(400.0, 300.0)
      .child(
        Column::new().width(320.0).child(
          Tabs::new(held.clone(), tabs)
            .variant(TabsVariant::Line)
            .overflow(kept.clone())
            .id("tabs"),
        ),
      )
      .into()
  });
  // The row is 40 high, its tabs 40 with labels of 13, 20 apart.
  let (x, _, width, height) = stage.bounds("canvas");
  assert_eq!(height, 40.0);
  assert_eq!(stage.text("Layout").1, 13.0);
  let (next, ..) = stage.bounds("layout");
  assert!((next - (x + width) - 20.0).abs() < 0.01, "20 apart");

  // The button at the end of the list scrolls it on, the one at its start back.
  let before = stage.bounds("canvas").0;
  stage.click("tabs-on");
  assert!(overflow.scroll().scroll_x() > 0.0, "scrolled on");
  assert!(stage.bounds("canvas").0 < before);
  stage.click("tabs-back");
  assert_eq!(overflow.scroll().scroll_x(), 0.0, "back");
  let (back_x, _, back_width, back_height) = stage.bounds("tabs-back");
  assert_eq!((back_width, back_height), (28.0, 40.0));
  assert!(x >= back_x + 28.0, "the list after the scroll button");

  // More opens the menu of every tab; choosing one there selects it.
  stage.click("tabs-more");
  assert!(overflow.open().get(), "open");
  stage.click("charts-item");
  assert_eq!(section.get(), 7);
  assert!(!overflow.open().get(), "closed");
  // The list scrolled the least that shows the chosen tab, the last, whole: its end at the scroll button's start.
  stage.pass();
  let (charts_x, _, charts_width, _) = stage.bounds("charts");
  let end_of_list = stage.bounds("tabs-on").0;
  assert!(
    (charts_x + charts_width - end_of_list).abs() < 0.5,
    "{} against {end_of_list}",
    charts_x + charts_width
  );
}

#[test]
fn a_disabled_tab_is_dimmed() {
  let stage = Stage::new(|_| Row::new().child(Tabs::controlled(0, settings(), |_| {})).into());
  let theme = stage.app.theme().clone();
  let case = Case::new()
    .variant(TabsVariant::Default)
    .state(TabsState::Disabled, true);
  let tab = theme.resolve::<Tabs>(&case, None).part(TabsPart::Tab).rest();
  assert_eq!(tab.opacity, Some(Opacity::Level(OpacityLevel::Disabled)));
  let enabled = theme
    .resolve::<Tabs>(&Case::new().variant(TabsVariant::Default), None)
    .part(TabsPart::Tab)
    .rest();
  assert_eq!(enabled.opacity, None);
}

#[test]
fn a_forced_focus_draws_the_ring_with_nothing_pointing_at_it() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(8.0)
      .child(Tabs::controlled(0, settings(), |_| {}).force(Some(Interaction::FocusVisible)))
      .into()
  });
  stage.pass();
  assert!(stage.has_ring(light(PaletteColor::Ring)));
}

#[test]
fn a_theme_gives_the_tabs_another_anatomy_and_they_still_select() {
  let asked = Arc::new(Mutex::new(Vec::new()));
  let told = asked.clone();
  let app = App::new();
  // The tabs as their list alone, with a word before it: other nodes, the same behavior.
  let renderer: TabsRenderer = Arc::new(|view: TabsView| {
    Row::new()
      .child(view.part(TabsPart::Label).text("SECTIONS"))
      .child(view.list())
      .into()
  });
  app.theme().set_renderer::<Tabs>(renderer);
  let mut stage = Stage::in_app(app, move |_| {
    let told = told.clone();
    let tabs: Element = Tabs::controlled(0, settings(), move |value| told.lock().unwrap().push(value)).into();
    Row::new().padding(8.0).child(tabs).into()
  });
  assert!(stage.paints("SECTIONS"));
  assert_eq!(
    stage.fill_color("general"),
    Some(light(PaletteColor::Background)),
    "the recipe still styles it"
  );
  stage.click("shortcuts");
  assert_eq!(*asked.lock().unwrap(), [2]);
}
