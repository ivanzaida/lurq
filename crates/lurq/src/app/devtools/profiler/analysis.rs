//! The analysis of the selected range or frame under the timeline: a flame chart
//! of its spans (toolkit phases on the frames track, application scopes on a
//! track per lane) with zoom, pan, hover and selection, and the call tree and
//! bottom-up tables of one track. The aggregation is `profiler::SpanForest`'s;
//! this module holds what is analysed and what is shown of it, `flame` draws the chart and `calls` the tables.
//!
//! The chart draws one element per block, not per span: spans narrower than
//! `CELL_PX` are snapped to cells of that width and runs of occupied cells
//! merge into one block, so a row never holds more than its width allows
//! however many spans the range has. A single handler on the chart hit-tests
//! the forest for hover, click, drag and wheel.

use std::{
  collections::{HashMap, HashSet},
  sync::{Arc, Mutex},
};

use super::{
  calls,
  capture::{ProfilerCapture, TimeRange, format_ms, ms, pass_of},
  flame,
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    devtools::kit::{FILL, mono},
    profiler::{CallNode, CallSort, CallTree, ForestSpan, SpanForest, SpanTrack, forest_of},
    theme::PaletteColor,
  },
  components::{Column, Row, Spacer},
  core::{ElementRef, Signal},
  layout::{Alignment, text_style::FontWeight},
  node::Element,
  ui::{ButtonSize, Choice, SegmentedControl},
};

/// The sizes the board gives the chart (`pencil/lurq-devtools.pen`, Profiler · dark): a row of spans with the
/// room under it, the name over a track, and the room between two tracks.
pub(crate) const ROW_HEIGHT: f32 = 22.0;
pub(crate) const BLOCK_HEIGHT: f32 = 20.0;
pub(crate) const TRACK_HEADER_HEIGHT: f32 = 20.0;
pub(crate) const TRACK_GAP: f32 = 8.0;
const HEADER_HEIGHT: f32 = 44.0;
pub(super) const PADDING: f32 = 20.0;
/// Rows a track draws at most; deeper spans are left out and counted.
pub(crate) const MAX_TRACK_ROWS: usize = 24;
/// Spans narrower than this snap to cells of this width, merged when adjacent.
pub(crate) const CELL_PX: f32 = 3.0;
/// How far from a thin span the pointer still hits it.
const HIT_TOLERANCE_PX: f32 = 3.0;
/// The narrowest view the chart zooms to: a microsecond.
pub(super) const MIN_VIEW_MS: f64 = 0.001;
/// Call tree rows open by default: the passes and their phases.
const CALL_TREE_OPEN_DEPTH: u32 = 2;

#[derive(Clone)]
pub(crate) struct AnalysisProps {
  pub capture: Arc<ProfilerCapture>,
  pub range: TimeRange,
  /// The pass selected in the timeline; the Frame scope analyses it.
  pub selected: Option<u64>,
  /// What is shown of the analysis: held by the tab, which reads what is selected in it.
  pub state: Signal<AnalysisState>,
  /// The name of what is selected, which the tab's sidebar picks out too.
  pub picked: Signal<Option<&'static str>>,
  /// Takes the pass of a clicked frame span, to select it in the timeline.
  pub on_select_pass: Option<Arc<dyn Fn(u64) + Send + Sync>>,
}

impl PartialEq for AnalysisProps {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.capture, &other.capture) && self.range == other.range && self.selected == other.selected
  }
}

impl crate::app::component::DevtoolsInspectable for AnalysisProps {
  fn write_info(&self, buffer: &mut Vec<crate::app::component::ComponentInfo>) {
    buffer.push(crate::app::component::ComponentInfo::with_value(
      "passes",
      std::any::type_name::<usize>(),
      self.capture.passes_in(self.range).len().to_string(),
    ));
  }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) enum AnalysisTab {
  #[default]
  Flame,
  CallTree,
  BottomUp,
}

/// What the analysis covers: the timeline's range, or its selected pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum AnalysisScope {
  #[default]
  Range,
  Frame,
}

/// What an analysis was made from; spans are indices into its forest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AnalysisKey {
  capture: usize,
  scope: AnalysisScope,
  range: TimeRange,
  frame: Option<u64>,
}

/// The forest of one range or pass, with what the chart and tables need.
pub(crate) struct Analysis {
  pub(super) key: AnalysisKey,
  pub forest: SpanForest,
  /// The tracks drawn, frames first, then the lanes that have scopes.
  pub tracks: Vec<TrackRows>,
  /// The time the chart shows unzoomed: the range, or the pass.
  pub basis: TimeRange,
  pub passes: usize,
  /// Sum of `PassSample::total` over the passes analysed.
  pub pass_time_ms: f64,
  /// The capture's start, the origin of the times shown.
  pub origin_ms: f64,
  /// Application scopes of the recording that the session no longer kept.
  pub dropped_scopes: u64,
  tables: Mutex<Tables>,
}

/// Tables made so far, by bottom-up, track, sort and direction.
type Tables = HashMap<(bool, SpanTrack, SortKey, bool), Arc<CallTree>>;

pub(crate) struct TrackRows {
  pub track: SpanTrack,
  pub rows: Vec<Vec<u32>>,
}

/// `CallSort` with `Hash` for the table cache.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct SortKey(u8);

impl From<CallSort> for SortKey {
  fn from(sort: CallSort) -> Self {
    Self(match sort {
      CallSort::Total => 0,
      CallSort::SelfTime => 1,
      CallSort::Count => 2,
      CallSort::Name => 3,
    })
  }
}

impl Analysis {
  pub(crate) fn new(
    capture: &Arc<ProfilerCapture>,
    scope: AnalysisScope,
    range: TimeRange,
    frame: Option<u64>,
  ) -> Self {
    let key = AnalysisKey {
      capture: Arc::as_ptr(capture) as usize,
      scope,
      range,
      frame: if scope == AnalysisScope::Frame { frame } else { None },
    };
    let passes = match scope {
      AnalysisScope::Range => capture.passes_in(range),
      AnalysisScope::Frame => frame
        .and_then(|sequence| capture.passes.iter().position(|sample| sample.sequence == sequence))
        .map_or(&[][..], |index| &capture.passes[index..=index]),
    };
    let basis = match scope {
      // Passes that start in the range are drawn to their end.
      AnalysisScope::Range => TimeRange {
        end_ms: passes
          .iter()
          .map(|sample| sample.completed_ms)
          .fold(range.end_ms, f64::max),
        ..range
      },
      AnalysisScope::Frame => passes.first().map_or(range, |sample| TimeRange {
        start_ms: sample.started_ms,
        end_ms: sample.completed_ms.max(sample.started_ms + MIN_VIEW_MS),
      }),
    };
    let forest = forest_of(passes, capture.scopes_in(basis));
    let tracks = SpanTrack::ALL
      .into_iter()
      .map(|track| TrackRows {
        track,
        rows: forest.rows(track),
      })
      .filter(|track| track.track == SpanTrack::Frames || !track.rows.is_empty())
      .collect();
    Self {
      key,
      tracks,
      basis,
      passes: passes.len(),
      pass_time_ms: passes
        .iter()
        .filter_map(|sample| pass_of(sample))
        .map(|pass| ms(pass.total))
        .sum(),
      origin_ms: capture.started_ms,
      dropped_scopes: capture.dropped_scopes,
      forest,
      tables: Mutex::new(HashMap::new()),
    }
  }

  /// The call tree, or the bottom-up table, of `track`, sorted.
  pub(crate) fn table(&self, bottom_up: bool, track: SpanTrack, sort: CallSort, descending: bool) -> Arc<CallTree> {
    let mut tables = self.tables.lock().unwrap_or_else(|error| error.into_inner());
    tables
      .entry((bottom_up, track, sort.into(), descending))
      .or_insert_with(|| {
        let mut tree = if bottom_up {
          self.forest.bottom_up(track)
        } else {
          self.forest.call_tree(track)
        };
        tree.sort(sort, descending);
        Arc::new(tree)
      })
      .clone()
  }

  pub(super) fn span(&self, index: u32) -> &ForestSpan {
    &self.forest.spans()[index as usize]
  }
}

/// The chart's rows from the top: each track's header, then its rows.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ChartGeometry {
  /// Per track of the analysis: the y of its first row and the rows drawn.
  pub tracks: Vec<(f32, usize)>,
  pub height: f32,
}

impl ChartGeometry {
  pub(crate) fn of(analysis: &Analysis) -> Self {
    let mut y = 0.0;
    let mut tracks = Vec::with_capacity(analysis.tracks.len());
    for (index, track) in analysis.tracks.iter().enumerate() {
      if index > 0 {
        y += TRACK_GAP;
      }
      y += TRACK_HEADER_HEIGHT;
      let rows = track.rows.len().clamp(1, MAX_TRACK_ROWS);
      tracks.push((y, rows));
      y += rows as f32 * ROW_HEIGHT;
    }
    Self { tracks, height: y }
  }

  /// The track and row at `y`, if `y` is on a row.
  fn row_at(&self, y: f32) -> Option<(usize, usize)> {
    self.tracks.iter().enumerate().find_map(|(track, (top, rows))| {
      let row = ((y - top) / ROW_HEIGHT).floor();
      (y >= *top && row < *rows as f32).then_some((track, row as usize))
    })
  }
}

/// The span under the chart-local point (`x`, `y`) when `view` spans `width`.
pub(crate) fn hit(
  analysis: &Analysis,
  geometry: &ChartGeometry,
  view: TimeRange,
  width: f32,
  x: f32,
  y: f32,
) -> Option<u32> {
  let (track, row) = geometry.row_at(y)?;
  let row = analysis.tracks.get(track)?.rows.get(row)?;
  if width <= 0.0 || !(0.0..=width).contains(&x) {
    return None;
  }
  let ns_per_px = view.duration_ms() * 1e6 / f64::from(width);
  let at_ns = (view.start_ms * 1e6 + f64::from(x) * ns_per_px).max(0.0) as u64;
  analysis
    .forest
    .span_at(row, at_ns, (f64::from(HIT_TOLERANCE_PX) * ns_per_px) as u64)
}

/// One drawn block: a span, or adjacent narrow spans merged.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Block {
  pub x: f32,
  pub width: f32,
  /// The span, when the block is one.
  pub span: Option<u32>,
}

/// A run of occupied cells not yet drawn: its first and last cell, and its
/// span while it holds only one.
struct Run {
  first: i64,
  last: i64,
  span: Option<u32>,
}

/// The blocks of one row in `view`, drawn `width` wide, left to right and
/// never overlapping.
pub(crate) fn row_blocks(forest: &SpanForest, row: &[u32], view: TimeRange, width: f32) -> Vec<Block> {
  let mut blocks: Vec<Block> = Vec::new();
  if width <= 0.0 || view.duration_ms() <= 0.0 {
    return blocks;
  }
  let spans = forest.spans();
  let start_ns = view.start_ms * 1e6;
  let end_ns = view.end_ms * 1e6;
  let px_per_ns = f64::from(width) / (end_ns - start_ns);
  let first = row.partition_point(|index| (spans[*index as usize].end_ns as f64) < start_ns);
  let mut run: Option<Run> = None;
  // Draws the run between the last block and `limit`.
  let flush = |run: &mut Option<Run>, blocks: &mut Vec<Block>, limit: f32| {
    let Some(run) = run.take() else {
      return;
    };
    let after = blocks.last().map_or(0.0, |block| block.x + block.width);
    let x = (run.first as f32 * CELL_PX).max(after);
    let end = ((run.last + 1) as f32 * CELL_PX).min(limit).min(width);
    if end > x {
      blocks.push(Block {
        x,
        width: end - x,
        span: run.span,
      });
    }
  };
  for index in &row[first..] {
    let span = &spans[*index as usize];
    if span.start_ns as f64 > end_ns {
      break;
    }
    let x0 = ((span.start_ns as f64 - start_ns) * px_per_ns).max(0.0) as f32;
    let x1 = ((span.end_ns as f64 - start_ns) * px_per_ns).min(f64::from(width)) as f32;
    if x1 - x0 >= CELL_PX {
      flush(&mut run, &mut blocks, x0);
      blocks.push(Block {
        x: x0,
        width: x1 - x0,
        span: Some(*index),
      });
      continue;
    }
    let (first, last) = ((x0 / CELL_PX).floor() as i64, (x1 / CELL_PX).floor() as i64);
    match &mut run {
      Some(current) if first <= current.last + 1 => {
        current.last = current.last.max(last);
        current.span = None;
      }
      _ => {
        flush(&mut run, &mut blocks, f32::INFINITY);
        run = Some(Run {
          first,
          last,
          span: Some(*index),
        });
      }
    }
  }
  flush(&mut run, &mut blocks, f32::INFINITY);
  blocks
}

#[derive(Clone, Default)]
pub(crate) struct AnalysisState {
  pub tab: AnalysisTab,
  pub scope: AnalysisScope,
  pub table_track: Option<SpanTrack>,
  pub sort: Option<(CallSort, bool)>,
  /// The analysis the rest belongs to; it is dropped when the analysis changes.
  key: Option<AnalysisKey>,
  /// The zoomed view; None shows the whole basis.
  pub view: Option<TimeRange>,
  pub hovered: Option<Hover>,
  pub selected_span: Option<u32>,
  pub(super) drag: Option<Drag>,
  /// Table rows opened or closed against their default, by table and track.
  toggled: HashSet<(bool, SpanTrack, u32)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Hover {
  pub span: u32,
  pub x: f32,
  pub y: f32,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Drag {
  pub(super) x: f32,
  pub(super) view: TimeRange,
  pub(super) moved: bool,
}

impl crate::app::component::DevtoolsInspectable for AnalysisState {
  fn write_info(&self, buffer: &mut Vec<crate::app::component::ComponentInfo>) {
    buffer.push(crate::app::component::ComponentInfo::with_value(
      "tab",
      std::any::type_name::<AnalysisTab>(),
      format!("{:?}", self.tab),
    ));
  }
}

impl AnalysisState {
  /// Forgets the view, hover, selection and open rows of another analysis.
  pub(super) fn sync(&mut self, key: AnalysisKey) {
    if self.key != Some(key) {
      *self = Self {
        tab: self.tab,
        scope: self.scope,
        table_track: self.table_track,
        sort: self.sort,
        key: Some(key),
        ..Self::default()
      };
    }
  }

  /// The state as it applies to `analysis`.
  pub(super) fn for_analysis(&self, analysis: &Analysis) -> Self {
    let mut state = self.clone();
    state.sync(analysis.key);
    state
  }

  pub(crate) fn view_of(&self, analysis: &Analysis) -> TimeRange {
    self
      .view
      .filter(|_| self.key == Some(analysis.key))
      .unwrap_or(analysis.basis)
  }

  pub(super) fn sort(&self) -> (CallSort, bool) {
    self.sort.unwrap_or((CallSort::Total, true))
  }

  pub(super) fn table_track(&self, analysis: &Analysis) -> SpanTrack {
    self
      .table_track
      .filter(|track| analysis.tracks.iter().any(|rows| rows.track == *track))
      .unwrap_or(SpanTrack::Frames)
  }

  pub(super) fn expanded(&self, bottom_up: bool, track: SpanTrack, index: u32, node: &CallNode) -> bool {
    let open = !bottom_up && node.depth < CALL_TREE_OPEN_DEPTH;
    open != self.toggled.contains(&(bottom_up, track, index))
  }

  /// Opens a closed row of a table and closes an open one.
  pub(super) fn toggle(&mut self, bottom_up: bool, track: SpanTrack, index: u32) {
    let key = (bottom_up, track, index);
    if !self.toggled.remove(&key) {
      self.toggled.insert(key);
    }
  }

  /// Zooms by `factor` (below 1 zooms in) around `at` (0..1 of the view),
  /// within the basis.
  pub(crate) fn zoom(&mut self, analysis: &Analysis, factor: f64, at: f64) {
    self.sync(analysis.key);
    let view = self.view_of(analysis);
    let basis = analysis.basis;
    let duration = (view.duration_ms() * factor).clamp(MIN_VIEW_MS.min(basis.duration_ms()), basis.duration_ms());
    let anchor = view.start_ms + view.duration_ms() * at.clamp(0.0, 1.0);
    let start = anchor - duration * at.clamp(0.0, 1.0);
    self.view = Some(clamp_view(start, duration, basis));
  }

  /// Moves the view by `delta_ms`, within the basis.
  pub(crate) fn pan(&mut self, analysis: &Analysis, from: TimeRange, delta_ms: f64) {
    self.sync(analysis.key);
    self.view = Some(clamp_view(from.start_ms + delta_ms, from.duration_ms(), analysis.basis));
  }

  /// Shows the span `index` with a tenth of its length at each side.
  pub(super) fn zoom_to(&mut self, analysis: &Analysis, index: u32) {
    self.sync(analysis.key);
    let span = analysis.span(index);
    let duration = ((span.end_ns - span.start_ns) as f64 / 1e6).max(MIN_VIEW_MS);
    let margin = duration * 0.1;
    self.view = Some(clamp_view(
      span.start_ns as f64 / 1e6 - margin,
      (duration + 2.0 * margin).min(analysis.basis.duration_ms()),
      analysis.basis,
    ));
  }
}

fn clamp_view(start_ms: f64, duration_ms: f64, basis: TimeRange) -> TimeRange {
  let start = start_ms.clamp(basis.start_ms, (basis.end_ms - duration_ms).max(basis.start_ms));
  TimeRange {
    start_ms: start,
    end_ms: start + duration_ms,
  }
}

pub(crate) struct ProfilerAnalysis {
  chart_width: Signal<f32>,
  chart: ElementRef,
  analysis: Mutex<Option<Arc<Analysis>>>,
}

impl ProfilerAnalysis {
  /// The analysis of `props` in `scope`, made again only when they changed.
  fn analysis(&self, props: &AnalysisProps, scope: AnalysisScope) -> Arc<Analysis> {
    let mut cached = self.analysis.lock().unwrap_or_else(|error| error.into_inner());
    let key = AnalysisKey {
      capture: Arc::as_ptr(&props.capture) as usize,
      scope,
      range: props.range,
      frame: if scope == AnalysisScope::Frame {
        props.selected
      } else {
        None
      },
    };
    match &*cached {
      Some(analysis) if analysis.key == key => analysis.clone(),
      _ => {
        let analysis = Arc::new(Analysis::new(&props.capture, scope, props.range, props.selected));
        *cached = Some(analysis.clone());
        analysis
      }
    }
  }
}

impl Component for ProfilerAnalysis {
  type Props = AnalysisProps;

  fn create(ctx: &mut Ctx) -> Self {
    let chart_width = ctx.signal(0.0_f32);
    let chart = ElementRef::new();
    chart.observe_rect({
      let chart_width = chart_width.clone();
      move |rect| {
        if chart_width.get_untracked() != rect.width {
          chart_width.set(rect.width);
        }
      }
    });
    Self {
      chart_width,
      chart,
      analysis: Mutex::new(None),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<AnalysisProps>().clone();
    let mut stored = props.state.get();
    // Without a selected pass, the Frame scope shows the range.
    if props.selected.is_none() {
      stored.scope = AnalysisScope::Range;
    }
    let analysis = self.analysis(&props, stored.scope);
    let state = stored.for_analysis(&analysis);
    let body = match state.tab {
      AnalysisTab::Flame => flame::chart(
        &analysis,
        &state,
        props.state.clone(),
        props.picked.clone(),
        self.chart_width.get(),
        self.chart.clone(),
        props.on_select_pass.clone(),
      ),
      AnalysisTab::CallTree => calls::table(ctx, &analysis, &state, props.state.clone(), props.picked.clone(), false),
      AnalysisTab::BottomUp => calls::table(ctx, &analysis, &state, props.state.clone(), props.picked.clone(), true),
    };
    Column::new()
      .width(FILL)
      .flex(1.0)
      .id("devtools-profiler-analysis")
      .child(header(&analysis, &state, props.state.clone(), props.selected.is_some()))
      .child(body)
  }
}

/// What is shown of the spans and of what, and what the analysis covers.
fn header(analysis: &Analysis, state: &AnalysisState, signal: Signal<AnalysisState>, has_selected_pass: bool) -> Row {
  let (shown, covered) = (signal.clone(), signal);
  let mut summary = format!(
    "{} · pass time {} · {} spans",
    super::passes(analysis.passes),
    format_ms(analysis.pass_time_ms),
    analysis.forest.len()
  );
  if analysis.dropped_scopes > 0 {
    summary.push_str(&format!(
      " · {} earlier application scopes not kept",
      analysis.dropped_scopes
    ));
  }
  Row::new()
    .width(FILL)
    .height(HEADER_HEIGHT)
    .padding_horizontal(PADDING)
    .spacing(12.0)
    .align_items(Alignment::Center)
    .child(
      SegmentedControl::controlled(
        state.tab,
        [
          Choice::new(AnalysisTab::Flame, "Flame chart"),
          Choice::new(AnalysisTab::CallTree, "Call tree"),
          Choice::new(AnalysisTab::BottomUp, "Bottom-up"),
        ],
        move |tab: &AnalysisTab| {
          let tab = *tab;
          shown.update(|state| state.tab = tab);
        },
      )
      .size(ButtonSize::Xs)
      .label("What is shown of the spans")
      .id("devtools-profiler-view"),
    )
    .child(
      SegmentedControl::controlled(
        state.scope,
        [
          Choice::new(AnalysisScope::Range, "Range"),
          // There is no frame to analyse with no pass selected.
          Choice::new(AnalysisScope::Frame, "Selected frame").disabled(!has_selected_pass),
        ],
        move |scope: &AnalysisScope| {
          let scope = *scope;
          covered.update(|state| state.scope = scope);
        },
      )
      .size(ButtonSize::Xs)
      .label("What the analysis covers")
      .id("devtools-profiler-scope"),
    )
    .child(Spacer::new().flex(1.0))
    .child(
      mono(&summary, 12.0, FontWeight::Normal)
        .color(PaletteColor::MutedForeground)
        .id("devtools-profiler-analysed"),
    )
}

#[cfg(test)]
pub(super) mod tests;
