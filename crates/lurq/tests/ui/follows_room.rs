//! What keeps sizes of its own between renders, laid out in a room that changes, as a window being dragged narrower
//! and wider does: a splitter, a dock layout and a data grid are at every width what one first laid out at that
//! width is. The sizes a narrow room took from the panes come back with the room.

use lurq::{
  components::{Column, Text},
  core::Signal,
  node::dimension::Dimension,
  ui::{
    Button, ColumnPin, DataGrid, DockLayout, DockNode, DockPanel, DockState, Splitter, SplitterPane, Table,
    TableColumn, Toolbar, ToolbarItem, icons,
  },
};

use super::stage::Stage;

const FILL: Dimension = Dimension::Pct(100.0);
/// The widths the room goes through: narrow, wide, and narrow again.
const WIDTHS: [f32; 3] = [300.0, 900.0, 300.0];

/// A window with `scene` in a room `width` wide, which the signal changes.
fn room(
  width: &Signal<f32>,
  scene: impl Fn(&mut lurq::app::ctx::Ctx) -> lurq::node::Element + Send + Sync + 'static,
) -> Stage {
  let width = width.clone();
  let mut stage = Stage::first_frame(1000.0, 400.0, move |ctx| {
    Column::new()
      .size(1000.0, 400.0)
      .child(Column::new().id("room").size(width.get(), 300.0).child(scene(ctx)))
      .into()
  });
  stage.pass();
  stage
}

/// Where each of `ids` starts and how wide it is.
fn boxes(stage: &mut Stage, ids: &[&str]) -> Vec<(f32, f32)> {
  ids
    .iter()
    .map(|id| {
      let (x, _, width, _) = stage.bounds(id);
      (x.round(), width.round())
    })
    .collect()
}

/// Lays `scene` out through [`WIDTHS`] and says that at each it is as one first laid out at that width, by what
/// `probe` reads of it.
#[track_caller]
fn follows_by(
  probe: impl Fn(&mut Stage) -> Vec<(f32, f32)>,
  scene: impl Fn(&mut lurq::app::ctx::Ctx) -> lurq::node::Element + Send + Sync + Clone + 'static,
) {
  let width = Signal::new(WIDTHS[0]);
  let mut stage = room(&width, scene.clone());
  for (step, to) in WIDTHS.into_iter().enumerate() {
    width.set(to);
    stage.pass();
    stage.pass();
    let fresh_width = Signal::new(to);
    let mut fresh = room(&fresh_width, scene.clone());
    fresh.pass();
    assert_eq!(
      probe(&mut stage),
      probe(&mut fresh),
      "at {to}, step {step} of {WIDTHS:?}: the one that came through the other widths, and one first laid out here"
    );
  }
}

/// As [`follows_by`], by the boxes of what `ids` name.
#[track_caller]
fn follows(
  ids: &'static [&'static str],
  scene: impl Fn(&mut lurq::app::ctx::Ctx) -> lurq::node::Element + Send + Sync + Clone + 'static,
) {
  follows_by(move |stage| boxes(stage, ids), scene);
}

fn pane(id: &'static str) -> Column {
  Column::new().id(id).size(FILL, FILL).child(Text::new(id))
}

#[test]
fn a_splitter_follows_its_room_wider_and_narrower() {
  follows(&["explorer", "editor", "outline"], |ctx| {
    Splitter::new()
      .pane(SplitterPane::new(pane("explorer")).length(200.0).min(140.0).max(360.0))
      .pane(SplitterPane::new(pane("editor")).min(120.0))
      .pane(SplitterPane::new(pane("outline")).share(0.2).min(40.0))
      .mount(ctx)
  });
}

#[test]
fn a_splitter_of_shares_follows_its_room_wider_and_narrower() {
  follows(&["explorer", "editor"], |ctx| {
    Splitter::new()
      .pane(SplitterPane::new(pane("explorer")).share(0.25).min(160.0))
      .pane(SplitterPane::new(pane("editor")).share(0.75).min(100.0))
      .mount(ctx)
  });
}

#[test]
fn a_dock_layout_follows_its_room_wider_and_narrower() {
  follows(&["dock-split-root-divider-0"], |ctx| {
    let state = Signal::new(DockState::new(
      DockNode::row([DockNode::group(["explorer"]), DockNode::group(["editor"])]).shares([0.25, 0.75]),
    ));
    DockLayout::new(
      state,
      vec![
        DockPanel::new("explorer", "Explorer")
          .min_size((160.0, 80.0))
          .child(Text::new("files")),
        DockPanel::new("editor", "Editor")
          .min_size((100.0, 80.0))
          .child(Text::new("fn main() {}")),
      ],
    )
    .id("dock")
    .mount(ctx)
  });
}

#[test]
fn a_data_grid_follows_its_room_wider_and_narrower() {
  follows(&["g-cell-0-0", "g-cell-0-1", "g-cell-0-2"], |ctx| {
    DataGrid::new(
      vec![
        TableColumn::new("name", "Name")
          .min_width(160.0)
          .text(|row: &(&str, &str, u32)| row.0.to_owned()),
        TableColumn::new("status", "Status")
          .width(120.0)
          .text(|row: &(&str, &str, u32)| row.1.to_owned()),
        TableColumn::new("size", "Size")
          .width(90.0)
          .text(|row: &(&str, &str, u32)| format!("{} KB", row.2)),
      ],
      vec![("blur.rs", "Passing", 12), ("theme.rs", "Failed", 48)],
    )
    .row_key(|row: &(&str, &str, u32)| row.0.to_owned())
    .id("g")
    .mount(ctx)
  });
}

#[test]
fn a_grid_with_a_column_pinned_at_each_end_follows_its_room_wider_and_narrower() {
  follows(&["g-cell-0-0", "g-cell-0-1", "g-cell-0-2", "g-cell-0-3"], |ctx| {
    DataGrid::new(
      vec![
        TableColumn::new("name", "Name")
          .min_width(160.0)
          .pin(ColumnPin::Start)
          .text(|row: &(&str, &str, u32)| row.0.to_owned()),
        TableColumn::new("status", "Status")
          .width(150.0)
          .text(|row: &(&str, &str, u32)| row.1.to_owned()),
        TableColumn::new("note", "Note")
          .width(150.0)
          .text(|row: &(&str, &str, u32)| format!("{} · note", row.0)),
        TableColumn::new("size", "Size")
          .width(90.0)
          .pin(ColumnPin::End)
          .text(|row: &(&str, &str, u32)| format!("{} KB", row.2)),
      ],
      vec![("blur.rs", "Passing", 12), ("theme.rs", "Failed", 48)],
    )
    .row_key(|row: &(&str, &str, u32)| row.0.to_owned())
    .id("g")
    .mount(ctx)
  });
}

#[test]
fn a_table_of_shares_follows_its_room_wider_and_narrower() {
  // A table's cells have no ids: where each column's text of the first row starts.
  let texts = |stage: &mut Stage| {
    ["blur.rs", "Passing", "12 KB"]
      .iter()
      .map(|text| {
        let quad = stage.text_quad(text).unwrap_or_else(|| panic!("{text:?} is drawn"));
        (quad.x.round(), 0.0)
      })
      .collect()
  };
  follows_by(texts, |ctx| {
    Table::new(
      vec![
        TableColumn::new("name", "Name")
          .share(2.0)
          .text(|row: &(&str, &str, u32)| row.0.to_owned()),
        TableColumn::new("status", "Status")
          .share(1.0)
          .min_width(120.0)
          .text(|row: &(&str, &str, u32)| row.1.to_owned()),
        TableColumn::new("size", "Size")
          .fit()
          .text(|row: &(&str, &str, u32)| format!("{} KB", row.2)),
      ],
      vec![("blur.rs", "Passing", 12), ("theme.rs", "Failed", 48)],
    )
    .row_key(|row: &(&str, &str, u32)| row.0)
    .mount(ctx)
  });
}

#[test]
fn a_toolbar_follows_its_room_through_scrolling_to_what_stays_and_back() {
  // At 300 the field that stays, the pin and More are wider than the bar, which scrolls; at 900 all of it fits.
  follows(&["bar", "field", "a", "b", "pin", "bar-more"], |ctx| {
    Toolbar::new()
      .id("bar")
      .overflow(true)
      .item(ToolbarItem::element(
        lurq::components::Row::new().id("field").size(260.0, 32.0),
      ))
      .item(Button::new("A").icon(icons::BOLD).icon_only(true).id("a"))
      .item(Button::new("B").icon(icons::ITALIC).icon_only(true).id("b"))
      .item(ToolbarItem::from(Button::new("Pin").icon(icons::LINK).icon_only(true).id("pin")).stays(true))
      .mount(ctx)
  });
}
