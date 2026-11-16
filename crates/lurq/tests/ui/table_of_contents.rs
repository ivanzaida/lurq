//! `ui::TableOfContents`: the design's list (`i7m0J`), the entry it marks as the page scrolls, where a press goes,
//! and its keys.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    events::ScrollPhase,
    theme::{Density, PaletteColor, ThemePalette},
  },
  components::{Column, Row, ScrollVertical, Text},
  core::{ElementRef, Signal},
  layout::{
    quad::{Quad, QuadContent},
    text_style::FontWeight,
  },
  node::{Element, color::Color},
  ui::{Button, TableOfContents, TableOfContentsEntry, TableOfContentsPart},
};

use super::stage::Stage;

/// The board's entries, and the level of each.
const ENTRIES: [(&str, usize); 5] = [
  ("Runtime theme", 1),
  ("Palette", 1),
  ("Extra roles", 2),
  ("Typography", 1),
  ("Font weight", 2),
];

/// How high a section of the page is, and the page's view.
const SECTION: f32 = 200.0;
const VIEW: f32 = 300.0;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn light_app() -> App {
  let app = App::new();
  app
    .theme()
    .set_preferred_appearance(Some(lurq::app::window::Appearance::Light));
  app
}

/// The refs of the sections: made once, as an app keeps them.
fn anchors() -> Arc<Vec<ElementRef>> {
  Arc::new(ENTRIES.iter().map(|_| ElementRef::new()).collect())
}

fn entries(anchors: &[ElementRef]) -> Vec<TableOfContentsEntry> {
  ENTRIES
    .iter()
    .zip(anchors)
    .enumerate()
    .map(|(index, ((label, level), anchor))| {
      TableOfContentsEntry::new(label, anchor)
        .level(*level)
        .id(format!("entry-{index}"))
    })
    .collect()
}

/// The list alone, still, with the second entry marked: the board's picture.
fn still(change: fn(TableOfContents) -> TableOfContents) -> Stage {
  let anchors = anchors();
  Stage::in_app(light_app(), move |_| {
    Column::new()
      .size(400.0, 300.0)
      .child(change(
        TableOfContents::new(entries(&anchors))
          .current(Signal::new(1))
          .id("contents"),
      ))
      .into()
  })
}

/// A page of five sections in a scroll of 260 by 300 at the window's start, and the list beside it, mounted.
fn page(
  current: Option<Signal<usize>>,
  change: impl Fn(TableOfContents) -> TableOfContents + Send + Sync + 'static,
) -> Stage {
  let (anchors, scroll) = (anchors(), ElementRef::new());
  Stage::in_app(light_app(), move |ctx| {
    let mut sections = Column::new().width(260.0);
    for (index, (label, _)) in ENTRIES.iter().enumerate() {
      sections = sections.child(
        Column::new()
          .size(260.0, SECTION)
          .id(format!("section-{index}"))
          .ref_element(anchors[index].clone())
          .child(Text::new(&format!("About {label}"))),
      );
    }
    let mut contents = TableOfContents::new(entries(&anchors))
      .container(&scroll)
      .id("contents");
    if let Some(current) = &current {
      contents = contents.current(current.clone());
    }
    Row::new()
      .size(480.0, VIEW)
      .child(
        ScrollVertical::new(sections)
          .size(260.0, VIEW)
          .id("page")
          .ref_element(scroll.clone()),
      )
      .child(change(contents).mount(ctx))
      // What Tab goes on to from the list.
      .child(Button::new("After").id("after"))
      .into()
  })
}

fn scroll_by(stage: &mut Stage, down: f32) {
  stage.tree.scroll(100.0, 100.0, 0.0, -down, ScrollPhase::Scroll);
  stage.pass();
}

fn text_of(stage: &Stage, content: &str) -> Quad {
  stage
    .text_quad(content)
    .unwrap_or_else(|| panic!("{content:?} is painted"))
}

/// The width and the color of the line at the start of the element with `id`.
fn start_line(stage: &mut Stage, id: &str) -> (f32, Color) {
  let side = stage
    .border(id)
    .and_then(|quad| quad.border)
    .and_then(|border| border.left)
    .unwrap_or_else(|| panic!("#{id} should have a line at its start"));
  (side.width, side.color)
}

/// The place of the entry drawn as the marked one: the one with the line of 2.
fn marked(stage: &mut Stage) -> usize {
  let marked: Vec<usize> = (0..ENTRIES.len())
    .filter(|index| start_line(stage, &format!("entry-{index}")).0 == 2.0)
    .collect();
  assert_eq!(marked.len(), 1, "one entry is marked: {marked:?}");
  marked[0]
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

#[test]
fn the_list_is_the_boards() {
  let mut stage = still(|contents| contents);
  let (x, y, width, height) = stage.bounds("contents");
  assert_eq!((x, y, width), (0.0, 0.0, 200.0));
  // A title of 21 and five entries of 28, 2 apart: 171 on the board.
  assert!((height - 171.0).abs() < 0.01, "{height}");
  // The title: the mono face at 11, in capitals, in MutedForeground, 12 in.
  let title = text_of(&stage, "ON THIS PAGE");
  assert_eq!(stage.text("ON THIS PAGE"), (light(PaletteColor::MutedForeground), 11.0));
  assert_eq!(title.x, 12.0);
  match &title.content {
    QuadContent::Text { style, .. } => assert_eq!(style.letter_spacing, 1.0),
    _ => unreachable!(),
  }
  let mut last_bottom = None;
  for (index, (label, level)) in ENTRIES.iter().enumerate() {
    let id = format!("entry-{index}");
    let (entry_x, entry_y, entry_width, entry_height) = stage.bounds(&id);
    assert_eq!((entry_x, entry_width, entry_height), (0.0, 200.0, 28.0), "{label}");
    if let Some(bottom) = last_bottom {
      assert_eq!(entry_y - bottom, 2.0, "{label}: 2 under the entry before it");
    }
    last_bottom = Some(entry_y + entry_height);
    // The label: 13, 12 after the line, and 12 further in at the second level; in the middle of its row.
    let text = text_of(&stage, label);
    assert_eq!(text.x, 12.0 * *level as f32, "{label}");
    assert!((text.y + text.height / 2.0 - (entry_y + 14.0)).abs() < 0.51, "{label}");
    let (color, size) = stage.text(label);
    assert_eq!(size, 13.0);
    let weight = match &text.content {
      QuadContent::Text { style, .. } => style.weight,
      _ => unreachable!(),
    };
    if index == 1 {
      // The marked one: a line of 2 in Primary, its label in Foreground at medium weight.
      assert_eq!(start_line(&mut stage, &id), (2.0, light(PaletteColor::Primary)));
      assert_eq!((color, weight), (light(PaletteColor::Foreground), FontWeight::Medium));
    } else {
      assert_eq!(start_line(&mut stage, &id), (1.0, light(PaletteColor::Border)));
      assert_eq!(
        (color, weight),
        (light(PaletteColor::MutedForeground), FontWeight::Normal)
      );
    }
  }
}

#[test]
fn it_marks_the_section_being_read_as_the_page_scrolls() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let tell = told.clone();
  let mut stage = page(None, move |contents| {
    let tell = tell.clone();
    contents.on_change(move |index| tell.lock().unwrap().push(index))
  });
  assert_eq!(marked(&mut stage), 0, "at the top the first section is read");
  // Not yet: the second section's top is 200 down, and the reading line 24 under the top of the page.
  scroll_by(&mut stage, 150.0);
  assert_eq!(marked(&mut stage), 0);
  // Its top has come up to the line.
  scroll_by(&mut stage, 30.0);
  assert_eq!(marked(&mut stage), 1);
  scroll_by(&mut stage, 350.0);
  assert_eq!(marked(&mut stage), 2);
  // And back.
  scroll_by(&mut stage, -530.0);
  assert_eq!(marked(&mut stage), 0);
  assert_eq!(*told.lock().unwrap(), [1, 2, 0], "told of each change, once");
}

#[test]
fn the_reading_line_is_where_the_app_says() {
  let mut stage = page(None, |contents| contents.reading_line(100.0));
  scroll_by(&mut stage, 110.0);
  assert_eq!(
    marked(&mut stage),
    1,
    "the second section's top is 90 under the top of the page"
  );
}

#[test]
fn a_press_goes_to_the_section_and_marks_its_entry() {
  let (selected, changed) = (Arc::new(Mutex::new(Vec::new())), Arc::new(Mutex::new(Vec::new())));
  let (select, change) = (selected.clone(), changed.clone());
  let mut stage = page(None, move |contents| {
    let (select, change) = (select.clone(), change.clone());
    contents
      .on_select(move |index| select.lock().unwrap().push(index))
      .on_change(move |index| change.lock().unwrap().push(index))
  });
  stage.click("entry-2");
  // The section's top is at the top of the page, where a nearest scroll would have left its end at the bottom.
  let (_, page_y, ..) = stage.bounds("page");
  let (_, section_y, ..) = stage.bounds("section-2");
  assert!((section_y - page_y).abs() < 0.5, "{section_y}");
  assert_eq!(marked(&mut stage), 2);
  assert_eq!(
    (selected.lock().unwrap().clone(), changed.lock().unwrap().clone()),
    (vec![2], vec![2])
  );
  // The last section cannot come up to the line: the page ends first. Its entry is marked because it was pressed.
  stage.click("entry-4");
  let (_, last_y, _, last_height) = stage.bounds("section-4");
  assert!(
    (last_y + last_height - (page_y + VIEW)).abs() < 0.5,
    "the page is at its end"
  );
  assert_eq!(marked(&mut stage), 4);
  assert_eq!(*changed.lock().unwrap(), [2, 4]);
  // A press of the marked entry goes there again and tells of no change.
  stage.click("entry-4");
  assert_eq!((selected.lock().unwrap().len(), changed.lock().unwrap().len()), (3, 2));
}

#[test]
fn the_apps_signal_has_the_entry_and_is_told_nothing_when_the_app_sets_it() {
  let current = Signal::new(0);
  let told = Arc::new(Mutex::new(Vec::new()));
  let tell = told.clone();
  let mut stage = page(Some(current.clone()), move |contents| {
    let tell = tell.clone();
    contents.on_change(move |index| tell.lock().unwrap().push(index))
  });
  scroll_by(&mut stage, 200.0);
  assert_eq!(current.get(), 1, "the scroll sets it");
  stage.click("entry-3");
  assert_eq!(current.get(), 3, "a press sets it");
  assert_eq!(*told.lock().unwrap(), [1, 3]);
  current.set(0);
  stage.pass();
  assert_eq!(marked(&mut stage), 0, "what the app sets is marked");
  assert_eq!(told.lock().unwrap().len(), 2, "and told to no handler");
}

#[test]
fn it_is_one_stop_of_tab_and_the_arrows_move_between_its_entries() {
  let mut stage = page(None, |contents| contents);
  scroll_by(&mut stage, 200.0);
  stage.key("Tab");
  assert_eq!(
    focused(&stage).as_deref(),
    Some("entry-1"),
    "Tab stops at the marked entry"
  );
  assert!(stage.has_ring(light(PaletteColor::Ring)), "with a ring");
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("entry-2"));
  stage.key("End");
  assert_eq!(focused(&stage).as_deref(), Some("entry-4"));
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("entry-4"), "it does not wrap");
  stage.key("Home");
  stage.key("ArrowDown");
  stage.key("ArrowDown");
  // Enter goes to the section of the focused entry.
  stage.key("Enter");
  assert_eq!(marked(&mut stage), 2);
  let (_, page_y, ..) = stage.bounds("page");
  assert!((stage.bounds("section-2").1 - page_y).abs() < 0.5);
  assert_eq!(
    focused(&stage).as_deref(),
    Some("entry-2"),
    "the focus stays on the entry"
  );
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("after"), "Tab leaves it");
}

#[test]
fn a_disabled_entry_takes_nothing_and_still_alone_a_press_scrolls_nothing() {
  let anchors = anchors();
  let selected = Arc::new(Mutex::new(Vec::new()));
  let (select, current) = (selected.clone(), Signal::new(0));
  let bound = current.clone();
  let mut stage = Stage::in_app(light_app(), move |_| {
    let select = select.clone();
    let list: Element = TableOfContents::new([
      TableOfContentsEntry::new("Runtime theme", &anchors[0]).id("one"),
      TableOfContentsEntry::new("Palette", &anchors[1])
        .id("two")
        .disabled(true),
      TableOfContentsEntry::new("Typography", &anchors[2]).id("three"),
    ])
    .current(bound.clone())
    .on_select(move |index| select.lock().unwrap().push(index))
    .into();
    Column::new().size(400.0, 300.0).child(list).into()
  });
  stage.click("two");
  assert!(selected.lock().unwrap().is_empty(), "a disabled entry is not pressed");
  assert_eq!(current.get(), 0);
  // Converted into an element, it marks what its signal says, and a press sets it and tells its handlers.
  stage.click("three");
  assert_eq!(*selected.lock().unwrap(), [2]);
  assert_eq!(current.get(), 2);
  assert_eq!(start_line(&mut stage, "three").0, 2.0);
  // The keys pass the disabled entry by.
  stage.key("ArrowUp");
  assert_eq!(focused(&stage).as_deref(), Some("one"));
}

#[test]
fn a_long_label_is_cut_in_its_row_and_a_narrow_room_narrows_the_list() {
  const LONG: &str = "How the theme of a running window is read by every recipe of it";
  let anchors = anchors();
  let mut stage = Stage::in_app(light_app(), move |_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        Row::new().id("room").width(150.0).child(
          TableOfContents::new([
            TableOfContentsEntry::new(LONG, &anchors[0]).id("long"),
            TableOfContentsEntry::new("Palette", &anchors[1]).level(2),
          ])
          .title("In this very long chapter of the guide")
          .id("contents"),
        ),
      )
      .into()
  });
  let (_, _, width, _) = stage.bounds("contents");
  assert_eq!(width, 150.0, "no wider than its room");
  let (_, _, _, height) = stage.bounds("long");
  assert_eq!(height, 28.0, "on one line");
  let label = stage.cut_text_quad(LONG).expect("the label, cut");
  assert!(matches!(&label.content, QuadContent::Text { text, .. } if text.ends_with('…')));
  for quad in stage.quads() {
    if let QuadContent::Text { text, .. } = &quad.content {
      assert!(quad.x + quad.width <= 150.5, "{text:?} runs past the room");
    }
  }
}

#[test]
fn its_entries_follow_the_density_and_a_theme_draws_it_otherwise() {
  let app = light_app();
  app.theme().set_density(Density::Compact);
  let anchors = anchors();
  let mut stage = Stage::in_app(app, move |_| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        TableOfContents::new(entries(&anchors))
          .title("")
          .style(TableOfContentsPart::Label, |s| s.foreground(PaletteColor::Primary)),
      )
      .into()
  });
  assert_eq!(stage.bounds("entry-0").3, 20.0, "8 lower in a compact theme");
  assert_eq!(stage.bounds("entry-0").1, 0.0, "an empty title draws none");
  assert!(!stage.paints("ON THIS PAGE"));
  assert_eq!(stage.text("Palette").0, light(PaletteColor::Primary));
}

#[test]
fn the_first_frame_marks_what_the_next_does() {
  let (anchors, scroll) = (anchors(), ElementRef::new());
  let scene = move |ctx: &mut lurq::app::ctx::Ctx| -> Element {
    let mut sections = Column::new().width(260.0);
    for (index, anchor) in anchors.iter().enumerate() {
      sections = sections.child(
        Column::new()
          .size(260.0, SECTION)
          .id(format!("section-{index}"))
          .ref_element(anchor.clone()),
      );
    }
    Row::new()
      .size(480.0, VIEW)
      .child(
        ScrollVertical::new(sections)
          .size(260.0, VIEW)
          .ref_element(scroll.clone()),
      )
      .child(
        TableOfContents::new(entries(&anchors))
          .container(&scroll)
          // The line is under the second section's top from the start: the first frame marks the second entry.
          .reading_line(220.0)
          .mount(ctx),
      )
      .into()
  };
  let mut stage = Stage::first_frame_in(light_app(), 480.0, VIEW, scene);
  let first = stage.picture();
  assert_eq!(marked(&mut stage), 1);
  stage.frame();
  assert_eq!(first, stage.picture(), "nothing settles in a second frame");
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn to_tooling_it_is_a_navigation_of_links_of_which_one_is_current() {
  let mut stage = still(|contents| contents);
  let mut said = |id: &str, key: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(key))
  };
  assert_eq!(said("contents", "a11y_role").as_deref(), Some("navigation"));
  assert_eq!(said("contents", "a11y_name").as_deref(), Some("On this page"));
  assert_eq!(said("entry-0", "a11y_role").as_deref(), Some("link"));
  assert_eq!(said("entry-0", "a11y_name").as_deref(), Some("Runtime theme"));
  assert_eq!(said("entry-0", "a11y_current"), None);
  assert_eq!(said("entry-1", "a11y_current").as_deref(), Some("location"));
  let mut stage = still(|contents| contents.label("Sections of the theme guide"));
  let name = stage
    .tree
    .get_element_by_id_mut("contents")
    .and_then(|element| element.description("a11y_name"));
  assert_eq!(name.as_deref(), Some("Sections of the theme guide"));
}
