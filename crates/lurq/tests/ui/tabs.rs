//! Tabs: the rail, the line, the pills and the list down the side as the
//! board draws them, what a press or a key selects, the panel of the
//! selected tab, the overflowing form, and a theme's other anatomy.

use std::{
  sync::{Arc, Mutex},
  time::Duration,
};

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
    Case, Interaction, Opacity, Recipe, Selector, Tab, Tabs, TabsOrientation, TabsPart, TabsRenderer, TabsState,
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

/// Whether the element with `id` is laid out: in the tree, and with a box.
fn laid_out(stage: &mut Stage, id: &str) -> bool {
  stage
    .tree
    .get_element_by_id_mut(id)
    .and_then(|element| element.bounds())
    .is_some()
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

  // A separator of 1 by 16 in the border's color sets the archived apart, 28 from each side.
  let separator = stage
    .quads()
    .into_iter()
    .find(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Border))
        && (quad.width, quad.height) == (1.0, 16.0)
    })
    .expect("a separator");
  let (memory_x, ..) = stage.bounds("memory");
  let (archived, ..) = stage.bounds("archived");
  assert!((separator.x - (memory_x + memory) - 28.0).abs() < 0.01);
  assert!((archived - (separator.x + 1.0) - 28.0).abs() < 0.01);
}

/// Overflowing tabs whose More button is told `more` and whose scroll buttons `back` and `on`, or nothing.
fn worded(more: Option<&'static str>, back: Option<&'static str>, on: Option<&'static str>) -> Stage {
  Stage::new(move |ctx| {
    let tabs = (0..12).map(|index| Tab::new(index, "A long tab"));
    let mut tabs = Tabs::controlled(0, tabs, |_| {})
      .variant(TabsVariant::Line)
      .overflow(true)
      .id("tabs");
    if let Some(more) = more {
      tabs = tabs.more_label(more);
    }
    if let Some(back) = back {
      tabs = tabs.scroll_back_label(back);
    }
    if let Some(on) = on {
      tabs = tabs.scroll_on_label(on);
    }
    Column::new().width(320.0).child(tabs.mount(ctx)).into()
  })
}

#[test]
fn the_more_button_says_what_the_app_gives_it() {
  let stage = worded(None, None, None);
  assert!(stage.paints("More"));
  let stage = worded(Some("Plus"), None, None);
  assert!(stage.paints("Plus"));
  assert!(!stage.paints("More"));
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn tooling_reads_the_words_of_the_overflowing_form_the_app_gives() {
  let said = |stage: &mut Stage, id: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description("a11y_name"))
  };
  let mut stage = worded(None, None, None);
  assert_eq!(said(&mut stage, "tabs-more").as_deref(), Some("More tabs"));
  assert_eq!(said(&mut stage, "tabs-back").as_deref(), Some("Scroll back"));
  assert_eq!(said(&mut stage, "tabs-on").as_deref(), Some("Scroll on"));
  let mut stage = worded(Some("Plus"), Some("Retour"), Some("Suite"));
  assert_eq!(
    said(&mut stage, "tabs-more").as_deref(),
    Some("Plus"),
    "what it says is its name"
  );
  assert_eq!(said(&mut stage, "tabs-back").as_deref(), Some("Retour"));
  assert_eq!(said(&mut stage, "tabs-on").as_deref(), Some("Suite"));
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
        move |value| told.lock().unwrap().push(*value),
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
fn on_change_is_told_what_a_person_selects_and_not_what_the_app_sets() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let tell = told.clone();
  let section = Signal::new(0u8);
  let held = section.clone();
  let mut stage = Stage::new(move |_| {
    let (first, second) = (tell.clone(), tell.clone());
    Column::new()
      .padding(8.0)
      .child(Button::new("Before").tab_index(0))
      .child(
        Tabs::new(
          held.clone(),
          [
            Tab::new(0, "One").id("one"),
            Tab::new(1, "Two").id("two"),
            Tab::new(2, "Three").id("three"),
          ],
        )
        .on_change(move |value| first.lock().unwrap().push(("first", *value)))
        .on_change(move |value| second.lock().unwrap().push(("second", *value))),
      )
      .into()
  });
  // A press: the signal is set, and each handler is told the new value, the first before the second.
  stage.click("two");
  assert_eq!(section.get(), 1);
  assert_eq!(*told.lock().unwrap(), [("first", 1), ("second", 1)]);
  // A press of the selected tab tells nothing.
  stage.click("two");
  assert_eq!(told.lock().unwrap().len(), 2, "what is already so");
  // The app sets the signal: nothing is told.
  section.set(2);
  stage.pass();
  assert_eq!(told.lock().unwrap().len(), 2, "what the app sets");
  // The arrow keys select, and are told as a press is.
  stage.key("Tab");
  stage.key("Tab");
  stage.key("ArrowLeft");
  assert_eq!(section.get(), 1);
  assert_eq!(told.lock().unwrap()[2..], [("first", 1), ("second", 1)]);
}

#[test]
fn controlled_tabs_show_what_the_app_answers_and_not_what_was_pressed() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let tell = told.clone();
  let mut stage = Stage::new(move |_| {
    let tell = tell.clone();
    Row::new()
      .padding(8.0)
      .child(Tabs::controlled(0, settings(), move |value| {
        tell.lock().unwrap().push(*value)
      }))
      .into()
  });
  stage.click("appearance");
  assert_eq!(*told.lock().unwrap(), [1]);
  // The app answered nothing: the first is still the selected one.
  assert_eq!(stage.fill_color("general"), Some(light(PaletteColor::Background)));
  assert_eq!(stage.fill_color("appearance"), None);
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
  let held = section.clone();
  let names = [
    "Canvas", "Layout", "Text", "Theme", "Router", "Forms", "Motion", "Charts",
  ];
  let mut stage = Stage::new(move |ctx| {
    let tabs = names
      .iter()
      .enumerate()
      .map(|(index, name)| Tab::new(index, name).id(name.to_lowercase()));
    let tabs = Tabs::new(held.clone(), tabs)
      .variant(TabsVariant::Line)
      .overflow(true)
      .id("tabs")
      .mount(ctx);
    Column::new()
      .size(400.0, 300.0)
      .child(Column::new().width(320.0).child(tabs))
      .into()
  });
  // The row is 40 high, its tabs 40 with labels of 13, 20 apart.
  let (x, _, width, height) = stage.bounds("canvas");
  assert_eq!(height, 40.0);
  assert_eq!(stage.text("Layout").1, 13.0);
  let (next, ..) = stage.bounds("layout");
  assert!((next - (x + width) - 20.0).abs() < 0.01, "20 apart");

  // The button at the end of the list scrolls it on, the one at its start back; the tabs keep where it is.
  let before = stage.bounds("canvas").0;
  stage.click("tabs-on");
  stage.pass();
  assert!(stage.bounds("canvas").0 < before, "scrolled on");
  stage.click("tabs-back");
  stage.pass();
  assert_eq!(stage.bounds("canvas").0, before, "back");
  let (back_x, _, back_width, back_height) = stage.bounds("tabs-back");
  assert_eq!((back_width, back_height), (28.0, 40.0));
  assert!(x >= back_x + 28.0, "the list after the scroll button");

  // More opens the menu of every tab, which the tabs keep open; choosing one there selects it and closes it.
  assert!(!laid_out(&mut stage, "charts-item"));
  stage.click("tabs-more");
  stage.pass();
  assert!(laid_out(&mut stage, "charts-item"), "open");
  stage.click("charts-item");
  assert_eq!(section.get(), 7);
  stage.pass();
  assert!(!laid_out(&mut stage, "charts-item"), "closed");
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
    let tabs: Element = Tabs::controlled(0, settings(), move |value| told.lock().unwrap().push(*value)).into();
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

#[test]
fn a_tabs_height_follows_the_density_in_each_form() {
  use lurq::app::theme::Density;
  for (density, step) in [
    (Density::Compact, -8.0),
    (Density::Default, 0.0),
    (Density::Comfortable, 8.0),
    (Density::Touch, 12.0),
  ] {
    for (variant, height) in [
      (TabsVariant::Default, 28.0),
      (TabsVariant::Line, 44.0),
      (TabsVariant::Pill, 30.0),
    ] {
      let app = App::new();
      app.theme().set_density(density);
      let mut stage = Stage::in_app(app, move |_| {
        Row::new()
          .padding(8.0)
          .child(Tabs::controlled(0, settings(), |_| {}).variant(variant))
          .into()
      });
      assert_eq!(stage.bounds("general").3, height + step, "{variant:?} at {density:?}");
    }
  }
}

#[test]
fn the_ring_of_a_focused_panel_stands_clear_of_its_text() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(16.0)
      .width(400.0)
      .child(Tabs::controlled(
        0,
        [
          Tab::new(0, "General")
            .id("general")
            .panel(|| Text::new("General settings").into()),
          Tab::new(1, "Appearance").id("appearance"),
        ],
        |_| {},
      ))
      .into()
  });
  // Tab reaches the selected tab, and then the panel.
  stage.key("Tab");
  stage.key("Tab");
  let ring = stage
    .quads()
    .into_iter()
    .find(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, gradient: None } if color.a() == 0)
        && quad.border.and_then(|border| border.top).map(|side| side.color) == Some(light(PaletteColor::Ring))
    })
    .expect("the panel's ring");
  let width = ring.border.and_then(|border| border.top).map_or(0.0, |side| side.width);
  let text = stage.text_quad("General settings").unwrap();
  let (inner_x, inner_y) = (ring.x + width, ring.y + width);
  let inner_bottom = ring.y + ring.height - width;
  assert!(text.x - inner_x >= 4.0, "4 clear at its start: {}", text.x - inner_x);
  assert!(text.y - inner_y >= 4.0, "4 clear above: {}", text.y - inner_y);
  assert!(
    inner_bottom - (text.y + text.height) >= 4.0,
    "4 clear under: {}",
    inner_bottom - (text.y + text.height)
  );
}

/// The names of tabs too many for a narrow room, with ids of the same in lower case.
const MANY: [&str; 8] = [
  "Canvas", "Layout", "Text", "Theme", "Router", "Forms", "Motion", "Charts",
];

/// Mounted tabs of `variant` that the app did not ask to overflow, in a room `width` wide whose id is `room`.
fn roomed(width: &Signal<f32>, section: &Signal<usize>, variant: TabsVariant) -> Stage {
  let (width, section) = (width.clone(), section.clone());
  Stage::first_frame(1000.0, 300.0, move |ctx| {
    let tabs = MANY
      .iter()
      .enumerate()
      .map(|(index, name)| Tab::new(index, name).id(name.to_lowercase()));
    Column::new()
      .size(1000.0, 300.0)
      .child(
        Column::new()
          .id("room")
          .width(width.get())
          .child(Tabs::new(section.clone(), tabs).variant(variant).id("tabs").mount(ctx)),
      )
      .into()
  })
}

/// Says that the element with `id` is whole within the one with `within`, across.
#[track_caller]
fn inside(stage: &mut Stage, id: &str, within: &str) {
  let (x, _, width, _) = stage.bounds(id);
  let (room, _, room_width, _) = stage.bounds(within);
  assert!(
    x >= room - 0.5 && x + width <= room + room_width + 0.5,
    "{id} at {x}..{} in {within} at {room}..{}",
    x + width,
    room + room_width
  );
}

#[test]
fn mounted_tabs_too_wide_for_their_room_scroll_between_their_buttons_from_the_first_frame() {
  for variant in [TabsVariant::Line, TabsVariant::Default, TabsVariant::Pill] {
    let (width, section) = (Signal::new(320.0f32), Signal::new(0usize));
    let mut stage = roomed(&width, &section, variant);
    // The first frame painted is the one that scrolls: none shows the list past its room.
    assert!(laid_out(&mut stage, "tabs-more"), "{variant:?}: the overflowing form");
    for id in ["tabs", "tabs-back", "tabs-on", "tabs-more", "canvas"] {
      inside(&mut stage, id, "room");
    }
    let first = stage.picture();
    stage.frame();
    assert_eq!(stage.picture(), first, "{variant:?}: settled in its first frame");

    // It is the form the app asks for with `overflow`: the buttons scroll, and More's menu selects.
    let before = stage.bounds("canvas").0;
    stage.click("tabs-on");
    stage.pass();
    assert!(stage.bounds("canvas").0 < before, "{variant:?}: scrolled on");
    stage.click("tabs-more");
    stage.pass();
    stage.click("charts-item");
    assert_eq!(section.get_untracked(), 7, "{variant:?}: chosen from the menu");
  }
}

#[test]
fn mounted_tabs_with_room_are_the_plain_form_and_take_it_back_when_their_room_grows() {
  let places = |stage: &mut Stage| -> Vec<(f32, f32, f32, f32)> {
    MANY.iter().map(|name| stage.bounds(&name.to_lowercase())).collect()
  };
  let (width, section) = (Signal::new(900.0f32), Signal::new(0usize));
  let mut stage = roomed(&width, &section, TabsVariant::Line);
  // With room for every tab the tabs are as they were: a row 44 high, labels of 14, no button.
  assert!(!laid_out(&mut stage, "tabs-more"));
  assert_eq!(stage.bounds("canvas").3, 44.0);
  assert_eq!(stage.text("Layout").1, 14.0);
  let wide = places(&mut stage);
  // What the list needs: from the start of its room to the end of its last tab.
  let needs = wide[7].0 + wide[7].2 - stage.bounds("room").0;

  width.set(320.0);
  stage.frame();
  assert!(laid_out(&mut stage, "tabs-more"), "the frame the room narrowed in");
  inside(&mut stage, "tabs-more", "room");
  let narrow = stage.picture();
  stage.frame();
  assert_eq!(stage.picture(), narrow, "settled in that frame");

  width.set(900.0);
  stage.frame();
  assert!(!laid_out(&mut stage, "tabs-more"), "the frame the room grew in");
  assert_eq!(places(&mut stage), wide, "as tabs laid out at 900 first");

  // A room dragged across the width they need: the form of each side of it, at once.
  for (room, overflows) in [
    (needs - 1.0, true),
    (needs - 20.0, true),
    (needs, false),
    (needs + 20.0, false),
    (needs - 1.0, true),
  ] {
    width.set(room);
    stage.frame();
    assert_eq!(laid_out(&mut stage, "tabs-more"), overflows, "at {room}");
  }
}

#[test]
fn tabs_that_take_the_overflowing_form_show_their_selected_tab() {
  let (width, section) = (Signal::new(320.0f32), Signal::new(7usize));
  let mut stage = roomed(&width, &section, TabsVariant::Line);
  // The last is the selected one: the list is scrolled the least that shows it whole, before the first frame.
  let (x, _, charts, _) = stage.bounds("charts");
  let (back, _, back_width, _) = stage.bounds("tabs-back");
  let on = stage.bounds("tabs-on").0;
  assert!(
    x >= back + back_width - 0.5 && x + charts <= on + 0.5,
    "{x}..{} between {} and {on}",
    x + charts,
    back + back_width
  );
  let first = stage.picture();
  stage.frame();
  assert_eq!(stage.picture(), first, "settled in its first frame");
}

#[test]
fn tabs_whose_labels_change_take_the_form_their_new_labels_have_room_for() {
  let long = Signal::new(true);
  let held = long.clone();
  let mut stage = Stage::first_frame(600.0, 300.0, move |ctx| {
    let names: [&str; 3] = if held.get() {
      ["A general section", "The look of things", "Every keyboard shortcut"]
    } else {
      ["One", "Two", "Three"]
    };
    let tabs = names
      .into_iter()
      .enumerate()
      .map(|(index, name)| Tab::new(index, name).id(format!("tab-{index}")));
    Column::new()
      .size(600.0, 300.0)
      .child(
        Column::new().id("room").width(300.0).child(
          Tabs::controlled(0usize, tabs, |_| {})
            .variant(TabsVariant::Line)
            .id("tabs")
            .mount(ctx),
        ),
      )
      .into()
  });
  assert!(laid_out(&mut stage, "tabs-more"), "three long labels do not fit 300");
  long.set(false);
  stage.frame();
  assert!(!laid_out(&mut stage, "tabs-more"), "three short ones do");
  assert_eq!(stage.text("Two").1, 14.0);
  long.set(true);
  stage.frame();
  assert!(laid_out(&mut stage, "tabs-more"), "and the long ones again do not");
}

#[test]
fn built_tabs_too_wide_for_their_room_end_their_labels_in_an_ellipsis_inside_it() {
  for variant in [TabsVariant::Line, TabsVariant::Default, TabsVariant::Pill] {
    let names = ["General", "Appearance", "Shortcuts", "Advanced"];
    let mut stage = Stage::new(move |_| {
      let tabs = names
        .iter()
        .enumerate()
        .map(|(index, name)| Tab::new(index, name).id(name.to_lowercase()));
      let tabs: Element = Tabs::controlled(0usize, tabs, |_| {})
        .variant(variant)
        .id("tabs")
        .into();
      Column::new()
        .size(400.0, 300.0)
        .child(Column::new().id("room").width(240.0).child(tabs))
        .into()
    });
    // Built with nothing kept, they cannot measure their room: every tab stays in it, its label cut.
    for name in names {
      inside(&mut stage, &name.to_lowercase(), "room");
    }
    assert!(!stage.paints("Appearance"), "{variant:?}: cut");
    assert!(
      stage.cut_text_quad("Appearance").is_some(),
      "{variant:?}: with an ellipsis"
    );
  }
  // With room they are whole, and as wide as they were.
  let mut stage = Stage::new(|_| {
    let tabs: Element = Tabs::controlled(0u8, settings(), |_| {}).id("tabs").into();
    Column::new().size(400.0, 300.0).child(tabs).into()
  });
  assert!(stage.paints("Appearance"));
  let (_, _, width, _) = stage.bounds("appearance");
  let label = stage.text_quad("Appearance").unwrap();
  assert!(
    (width - label.width - 24.0).abs() < 0.5,
    "padded 12 each side of its label"
  );
}

#[test]
fn a_tab_in_a_state_of_its_own_paints_its_parts_in_it_and_the_others_do_not() {
  let rules = Recipe::new().when(Selector::new().state("waits"), TabsPart::Count, |s| {
    s.foreground(PaletteColor::Warning)
  });
  let mut stage = Stage::new(move |_| {
    Row::new()
      .padding(8.0)
      .child(
        Tabs::controlled(
          0,
          [
            Tab::new(0, "Running").count(4),
            Tab::new(1, "Waiting").count(2).state("waits", true),
            Tab::new(2, "Done").count(9).state("waits", false),
          ],
          |_| {},
        )
        .variant(TabsVariant::Line)
        .rules(&rules),
      )
      .into()
  });
  assert_eq!(stage.text("2").0, light(PaletteColor::Warning), "the tab in the state");
  assert_ne!(stage.text("4").0, light(PaletteColor::Warning));
  assert_ne!(
    stage.text("9").0,
    light(PaletteColor::Warning),
    "a state taken out is none"
  );
}

#[test]
fn a_tab_that_opens_shows_its_tooltip_and_the_apps_tooltip_wins_over_a_reason() {
  let mut stage = Stage::new(|ctx| {
    Column::new()
      .size(400.0, 300.0)
      .padding(40.0)
      .child(
        Tabs::controlled(
          0,
          [
            Tab::new(0, "Runs").id("runs"),
            Tab::new(1, "Agents").id("agents").hint("Two agents wait for you"),
            Tab::new(2, "Insights")
              .id("insights")
              .disabled_reason("Comes later")
              .tooltip(lurq::ui::Tooltip::new("Insights come with the next release")),
          ],
          |_| {},
        )
        .mount(ctx),
      )
      .into()
  });
  stage.own_timers();
  assert!(!stage.paints("Two agents wait for you"));
  stage.hover("agents");
  stage.tick_timers(Duration::from_secs(2));
  stage.pass();
  assert!(stage.paints("Two agents wait for you"), "the hint of a tab that opens");
  stage.hover("insights");
  stage.tick_timers(Duration::from_secs(4));
  stage.pass();
  assert!(stage.paints("Insights come with the next release"));
  assert!(
    !stage.paints("Comes later"),
    "the app's tooltip in place of the reason's"
  );
}

#[test]
fn reselecting_tabs_tell_a_press_of_the_selected_tab_again() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let section = Signal::new(0u8);
  let (held, heard) = (section.clone(), told.clone());
  let mut stage = Stage::new(move |ctx| {
    let heard = heard.clone();
    let tabs = Tabs::new(
      held.clone(),
      [Tab::new(0, "One").id("one"), Tab::new(1, "Two").id("two")],
    )
    .reselect(true)
    .on_change(move |value| heard.lock().unwrap().push(*value))
    .mount(ctx);
    Column::new().padding(8.0).child(tabs).into()
  });
  stage.click("one");
  assert_eq!(*told.lock().unwrap(), [0], "the selected tab, told again");
  assert_eq!(section.get(), 0);
  stage.click("two");
  stage.click("two");
  assert_eq!(*told.lock().unwrap(), [0, 1, 1]);
  assert_eq!(section.get(), 1);
}

#[test]
fn tabs_that_do_not_reselect_tell_nothing_of_a_press_of_the_selected_tab() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let heard = told.clone();
  let mut stage = Stage::new(move |_| {
    let heard = heard.clone();
    Column::new()
      .padding(8.0)
      .child(Tabs::controlled(
        0u8,
        [Tab::new(0, "One").id("one"), Tab::new(1, "Two").id("two")],
        move |value| heard.lock().unwrap().push(*value),
      ))
      .into()
  });
  stage.click("one");
  assert!(told.lock().unwrap().is_empty());
}

#[test]
fn the_line_of_mounted_tabs_slides_to_the_tab_chosen_next() {
  let chosen = Signal::new("all");
  let held = chosen.clone();
  let mut stage = Stage::first_frame(600.0, 300.0, move |ctx| {
    Column::new()
      .width(600.0)
      .child(
        Tabs::new(
          held.clone(),
          [
            Tab::new("all", "All").id("all"),
            Tab::new("profiles", "Profiles").id("profiles"),
            Tab::new("memory", "Memory").id("memory"),
          ],
        )
        .variant(TabsVariant::Line)
        .mount(ctx),
      )
      .into()
  });
  // The indicator: a fill of 2 in the foreground.
  let mark = |stage: &Stage| {
    stage
      .quads()
      .into_iter()
      .find(|quad| {
        matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Foreground))
          && (quad.height - 2.0).abs() < 0.01
      })
      .map(|mark| (mark.x, mark.width))
  };
  // Under the selected tab from the first frame.
  let (x, _, width, _) = stage.bounds("all");
  assert_eq!(mark(&stage), Some((x, width)), "where the tab is, in the first frame");
  stage.frame();
  assert_eq!(mark(&stage), Some((x, width)), "and still");

  // A press of the third tab: the line leaves the first and slides to it in the base time.
  let (px, py) = stage.center("memory");
  stage.tree.mouse_move(px, py);
  stage.tree.mouse_down(px, py, lurq::app::events::MouseButton::Left);
  stage.tree.mouse_up(px, py, lurq::app::events::MouseButton::Left);
  stage.frame();
  stage.frame();
  stage.frame();
  assert_eq!(chosen.get_untracked(), "memory");
  let (to, _, to_width, _) = stage.bounds("memory");
  let (on_its_way, _) = mark(&stage).expect("the line");
  assert!(
    on_its_way > x && on_its_way < to,
    "on its way from the first tab: {on_its_way}, to {to}"
  );
  stage.pass();
  assert_eq!(mark(&stage), Some((to, to_width)), "under the tab chosen");
}
