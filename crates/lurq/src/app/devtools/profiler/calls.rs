//! The tables of an analysis: the call tree of one track, each span under what called it, and the same spans
//! bottom-up, each over what called it. Both are the design system's table with a tree in its first column.

use std::sync::Arc;

use super::{
  analysis::{Analysis, AnalysisState, PADDING},
  capture::format_ms,
  flame::lane_name,
  phase_color, span_color,
};
use crate::{
  app::{
    ctx::Ctx,
    devtools::kit::{self, FILL, mono},
    events::MouseEvent,
    profiler::{CallSort, CallTree, SpanTrack},
    theme::PaletteColor,
  },
  components::{Column, Row, Spacer},
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element},
  ui::{ButtonSize, Choice, Icon, SegmentedControl, SortDirection, Table, TableColumn, TablePart, TableSort, icons},
};

/// The sizes the board gives the tables (`pencil/lurq-devtools.pen`, Profiler · dark).
const TIME_WIDTH: f32 = 92.0;
const SHARE_WIDTH: f32 = 76.0;
const COUNT_WIDTH: f32 = 64.0;
const INDENT: f32 = 14.0;
const CHEVRON: f32 = 12.0;

/// A row of a table as it is drawn.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Line {
  /// Its place among the nodes of its table: what it is told apart by.
  pub(crate) index: u32,
  pub(crate) name: &'static str,
  pub(crate) depth: u32,
  pub(crate) children: usize,
  pub(crate) open: bool,
  pub(crate) total_ns: u64,
  pub(crate) self_ns: u64,
  pub(crate) count: u64,
  /// What the two times are of the table's whole, in hundredths.
  pub(crate) total_share: f64,
  pub(crate) self_share: f64,
  color: PaletteColor,
}

/// The rows of `tree` that are shown, each parent before its open children. In the call tree a component is
/// in the color of the phase it is in; bottom-up, a row is over what called it, and has the color of its name.
pub(crate) fn lines(
  tree: &CallTree,
  track: SpanTrack,
  bottom_up: bool,
  expanded: impl Fn(u32, &crate::app::profiler::CallNode) -> bool,
) -> Vec<Line> {
  let mut lines = Vec::new();
  // What is still to be shown, the next on top, each with the color it is in.
  let mut stack: Vec<(u32, PaletteColor)> = tree
    .roots
    .iter()
    .rev()
    .map(|root| (*root, span_color("", track)))
    .collect();
  while let Some((index, within)) = stack.pop() {
    let node = &tree.nodes[index as usize];
    let color = match track {
      SpanTrack::Application(_) => span_color(node.name, track),
      SpanTrack::Frames if bottom_up => span_color(node.name, track),
      SpanTrack::Frames => phase_color(node.name).unwrap_or(within),
    };
    // The pass's own color is not its phases': a phase names its own.
    let inherited = if node.depth == 0 {
      span_color("", track)
    } else {
      color.clone()
    };
    let open = !node.children.is_empty() && expanded(index, node);
    lines.push(Line {
      index,
      name: node.name,
      depth: node.depth,
      children: node.children.len(),
      open,
      total_ns: node.total_ns,
      self_ns: node.self_ns,
      count: node.count,
      total_share: tree.percent(node.total_ns),
      self_share: tree.percent(node.self_ns),
      color,
    });
    if open {
      stack.extend(node.children.iter().rev().map(|child| (*child, inherited.clone())));
    }
  }
  lines
}

/// The call tree or the bottom-up table of the chosen track.
pub(super) fn table(
  ctx: &mut Ctx,
  analysis: &Arc<Analysis>,
  state: &AnalysisState,
  signal: Signal<AnalysisState>,
  picked: Signal<Option<&'static str>>,
  bottom_up: bool,
) -> Element {
  let track = state.table_track(analysis);
  let (sort, descending) = state.sort();
  let tree = analysis.table(bottom_up, track, sort, descending);
  let lines = Arc::new(lines(&tree, track, bottom_up, |index, node| {
    state.expanded(bottom_up, track, index, node)
  }));

  let total = match track {
    SpanTrack::Frames => format!(
      "total {} = pass time of {}",
      format_ms(tree.total_ns as f64 / 1e6),
      super::passes(analysis.passes)
    ),
    SpanTrack::Application(_) => format!("total {} of root scopes", format_ms(tree.total_ns as f64 / 1e6)),
  };
  let chosen = signal.clone();
  let bar = Row::new()
    .width(FILL)
    .height(36.0)
    .padding_horizontal(PADDING)
    .padding_bottom(8.0)
    .spacing(12.0)
    .align_items(Alignment::Center)
    .child(
      SegmentedControl::controlled(
        track,
        analysis
          .tracks
          .iter()
          .map(|rows| Choice::new(rows.track, track_title(rows.track))),
        move |track: &SpanTrack| {
          let track = *track;
          chosen.update(|state| state.table_track = Some(track));
        },
      )
      .size(ButtonSize::Xs)
      .label("Track")
      .id("devtools-profiler-track"),
    )
    .child(Spacer::new().flex(1.0))
    .child(
      mono(&total, 12.0, FontWeight::Normal)
        .color(PaletteColor::MutedForeground)
        .id("devtools-profiler-total"),
    );

  let toggle: Arc<dyn Fn(u32) + Send + Sync> = {
    let signal = signal.clone();
    Arc::new(move |index| signal.update(|state| state.toggle(bottom_up, track, index)))
  };
  let end =
    |text: crate::components::Text| -> Element { Row::new().width(FILL).justify(Justify::End).child(text).into() };
  let quiet = move |text: String| end(mono(&text, 12.0, FontWeight::Normal).color(PaletteColor::MutedForeground));
  let columns = vec![
    TableColumn::new("name", "Name")
      .cell({
        let toggle = toggle.clone();
        move |line: &Line| name_cell(line, toggle.clone())
      })
      .sortable(true),
    TableColumn::new("total", "Total")
      .width(TIME_WIDTH)
      .cell(move |line: &Line| end(mono(&format_ms(line.total_ns as f64 / 1e6), 12.0, FontWeight::Medium)))
      .numeric()
      .sortable(true),
    TableColumn::new("total-share", "Total %")
      .width(SHARE_WIDTH)
      .cell(move |line: &Line| quiet(format!("{:.1}%", line.total_share)))
      .numeric(),
    TableColumn::new("self", "Self")
      .width(TIME_WIDTH)
      .cell(move |line: &Line| end(mono(&format_ms(line.self_ns as f64 / 1e6), 12.0, FontWeight::Normal)))
      .numeric()
      .sortable(true),
    TableColumn::new("self-share", "Self %")
      .width(SHARE_WIDTH)
      .cell(move |line: &Line| quiet(format!("{:.1}%", line.self_share)))
      .numeric(),
    TableColumn::new("count", "Count")
      .width(COUNT_WIDTH)
      .cell(move |line: &Line| quiet(line.count.to_string()))
      .numeric()
      .sortable(true),
  ];
  // The rows of the name that is picked, in the chart or here, are the selected ones.
  let name = picked.get();
  let selected: Vec<u32> = lines
    .iter()
    .filter(|line| name == Some(line.name))
    .map(|line| line.index)
    .collect();
  let (by_index, opened) = (lines.clone(), lines.clone());
  let (picking, sorting) = (signal.clone(), signal);
  let table = Table::new_shared(columns, lines)
    .row_key(|line: &Line| line.index)
    .label(if bottom_up { "Bottom-up" } else { "Call tree" })
    .id("devtools-profiler-calls")
    .checkboxes(false)
    .selected_controlled(selected, move |rows: Vec<u32>| {
      let name = rows
        .last()
        .and_then(|index| by_index.iter().find(|line| line.index == *index))
        .map(|line| line.name);
      if name.is_some() && picked.get_untracked() != name {
        // A span of the chart with another name is the selected one no longer.
        picking.update(|state| state.selected_span = None);
        picked.set(name);
      }
    })
    // Enter and a double press open and close the row, as its chevron does.
    .on_row_activate(move |index: &u32| {
      if opened.iter().any(|line| line.index == *index && line.children > 0) {
        toggle(*index);
      }
    })
    .sort_controlled(
      vec![TableSort {
        column: Arc::from(sort_column(sort)),
        direction: if descending {
          SortDirection::Descending
        } else {
          SortDirection::Ascending
        },
      }],
      move |next: Vec<TableSort>| {
        sorting.update(|state| {
          let (current, descending) = state.sort();
          // The header pressed: the one the table would sort by next, or the sorted one it would leave.
          let pressed = next
            .first()
            .and_then(|sort| sort_of(&sort.column))
            .unwrap_or(current);
          state.sort = Some(if pressed == current {
            (pressed, !descending)
          } else {
            // The longest and the most first; names from the first.
            (pressed, pressed != CallSort::Name)
          });
        });
      },
    )
    .empty_text("No spans in this range");
  let table = kit::dense_table(table)
    .style(TablePart::Header, |s| s.padding_x(PADDING))
    .style(TablePart::Row, |s| s.padding_x(PADDING))
    .mount(ctx);
  Column::new()
    .width(FILL)
    .flex(1.0)
    .child(bar)
    .child(Column::new().width(FILL).flex(1.0).child(table))
    .into()
}

fn track_title(track: SpanTrack) -> String {
  match track {
    SpanTrack::Frames => "Frames".to_owned(),
    SpanTrack::Application(lane) => format!("App {}", lane_name(lane)),
  }
}

fn sort_column(sort: CallSort) -> &'static str {
  match sort {
    CallSort::Name => "name",
    CallSort::Total => "total",
    CallSort::SelfTime => "self",
    CallSort::Count => "count",
  }
}

fn sort_of(column: &str) -> Option<CallSort> {
  Some(match column {
    "name" => CallSort::Name,
    "total" => CallSort::Total,
    "self" => CallSort::SelfTime,
    "count" => CallSort::Count,
    _ => return None,
  })
}

/// The tree's cell of a row: its indent, its chevron where it holds something, its color and its name.
fn name_cell(line: &Line, toggle: Arc<dyn Fn(u32) + Send + Sync>) -> Element {
  let mut cell = Row::new()
    .width(FILL)
    .height(kit::DENSE_ROW_HEIGHT)
    .spacing(6.0)
    .padding_left(line.depth as f32 * INDENT)
    .align_items(Alignment::Center);
  cell = if line.children == 0 {
    cell.child(Row::new().size(CHEVRON, CHEVRON))
  } else {
    let index = line.index;
    cell.child(
      Row::new()
        .size(CHEVRON, kit::DENSE_ROW_HEIGHT)
        .align_items(Alignment::Center)
        .cursor(CursorIcon::Pointer)
        .on_click(move |event: MouseEvent| {
          // The press is the chevron's: it does not select the row as well.
          event.stop_propagation();
          toggle(index);
        })
        .child(
          Icon::new(if line.open {
            icons::CHEVRON_DOWN
          } else {
            icons::CHEVRON_RIGHT
          })
          .size(CHEVRON)
          .color(PaletteColor::MutedForeground),
        ),
    )
  };
  cell
    .child(Row::new().size(8.0, 8.0).rounded(2.0).background(line.color.clone()))
    .child(mono(
      line.name,
      12.0,
      if line.depth == 0 {
        FontWeight::Medium
      } else {
        FontWeight::Normal
      },
    ))
    .into()
}
