//! `force(..)` of a component made of items draws the interaction on one item, the one Tab would stop on (its
//! focus entry): a ring on every segment, row or tab at once is a picture that never happens.

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::{Column, Row},
  core::Signal,
  layout::quad::QuadContent,
  node::{Element, color::Color},
  ui::{
    Breadcrumb, BreadcrumbStep, Interaction, MenuEntry, MenuItem, Menubar, SegmentedControl, Tab, Tabs, TreeItem,
    TreeView,
  },
};

use super::stage::Stage;

fn ring() -> Color {
  ThemePalette::light().get(PaletteColor::Ring)
}

/// The boxes a line of the ring color is painted around: the outlines of what is drawn focused.
fn rings(stage: &Stage) -> Vec<(f32, f32)> {
  stage
    .quads()
    .iter()
    .filter(|quad| {
      matches!(&quad.content, QuadContent::Rect { color: fill, gradient: None } if fill.a() == 0)
        && quad.border.and_then(|border| border.top).map(|side| side.color) == Some(ring())
    })
    .map(|quad| (quad.x, quad.y))
    .collect()
}

fn stage(scene: impl Fn() -> Element + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::new(move |_| Column::new().size(600.0, 400.0).padding(20.0).child(scene()).into());
  stage.pass();
  stage
}

/// The ring is around the item `id` alone.
fn ring_on(stage: &mut Stage, id: &str) {
  let found = rings(stage);
  assert_eq!(found.len(), 1, "one ring, not one on each item: {found:?}");
  let (x, y, width, height) = stage.bounds(id);
  let (ring_x, ring_y) = found[0];
  assert!(
    ring_x >= x - 4.0 && ring_x <= x + width && ring_y >= y - 4.0 && ring_y <= y + height,
    "the ring at {:?} is around #{id} at {:?}",
    found[0],
    (x, y)
  );
}

#[test]
fn a_segmented_control_draws_it_on_the_chosen_segment() {
  let mut stage = stage(|| {
    SegmentedControl::controlled(1, [(0, "Day"), (1, "Week"), (2, "Month")], |_| {})
      .id("period")
      .force(Some(Interaction::FocusVisible))
      .into()
  });
  assert_eq!(rings(&stage).len(), 1, "one ring, not one on each segment");
  // The chosen segment is the second of three: the ring is right of the first third.
  let (x, _, width, _) = stage.bounds("period");
  assert!(rings(&stage)[0].0 > x + width / 4.0);
}

#[test]
fn a_tree_draws_it_on_the_selected_row_else_the_first() {
  let items = || {
    [
      TreeItem::new("src", "src")
        .id("src")
        .item(TreeItem::new("lib", "lib.rs").id("lib")),
      TreeItem::new("cargo", "Cargo.toml").id("cargo"),
      TreeItem::new("readme", "README.md").id("readme"),
    ]
  };
  let mut first = stage(move || {
    TreeView::new(Signal::new(None), items())
      .force(Some(Interaction::FocusVisible))
      .into()
  });
  ring_on(&mut first, "src");
  let mut chosen = stage(move || {
    TreeView::new(Signal::new(vec!["cargo"]), items())
      .force(Some(Interaction::FocusVisible))
      .into()
  });
  ring_on(&mut chosen, "cargo");
}

#[test]
fn tabs_draw_it_on_the_selected_tab_and_not_on_the_panel() {
  let mut stage = stage(|| {
    Row::new()
      .child(
        Tabs::controlled(
          1,
          [
            Tab::new(0, "General").id("general"),
            Tab::new(1, "Appearance").id("appearance"),
            Tab::new(2, "Shortcuts").id("shortcuts"),
          ],
          |_| {},
        )
        .force(Some(Interaction::FocusVisible)),
      )
      .into()
  });
  ring_on(&mut stage, "appearance");
}

#[test]
fn a_menubar_draws_it_on_the_first_title() {
  let mut stage = stage(|| {
    Menubar::new(Signal::new(None))
      .id("bar")
      .menu("File", [MenuEntry::from(MenuItem::new("New"))])
      .menu("Edit", [MenuEntry::from(MenuItem::new("Undo"))])
      .menu("View", [MenuEntry::from(MenuItem::new("Zoom"))])
      .force(Some(Interaction::FocusVisible))
      .into()
  });
  // A title drawn focused is filled with the highlight.
  let highlight = Some(ThemePalette::light().get(PaletteColor::Highlight));
  assert_eq!(stage.fill_color("bar-0"), highlight);
  assert_ne!(stage.fill_color("bar-1"), highlight);
  assert_ne!(stage.fill_color("bar-2"), highlight);
}

#[test]
fn a_breadcrumb_draws_it_on_the_first_link() {
  let mut stage = stage(|| {
    Breadcrumb::default()
      .step(BreadcrumbStep::new("Settings").id("settings"))
      .step(BreadcrumbStep::new("Appearance").id("appearance"))
      .step(BreadcrumbStep::new("Theme").id("theme"))
      .force(Some(Interaction::FocusVisible))
      .into()
  });
  ring_on(&mut stage, "settings");
}
