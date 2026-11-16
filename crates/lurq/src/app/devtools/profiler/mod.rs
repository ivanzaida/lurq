//! The Profiler tab: a recording of the inspected windows' passes, the range of it a person selects, the pass
//! selected in that range, and the spans of either as a flame chart and as tables. Built from the board
//! (`pencil/lurq-devtools.pen`, Profiler · dark) with the design system's components.
//!
//! What is recorded and how it is read is not this tab's: the samples are the engine's profiler's
//! (`app::profiler`), `capture` puts a finished recording in the order it is shown in, and `analysis` holds the
//! spans of a range. The rest of the files draw them.

mod analysis;
mod calls;
pub(crate) mod capture;
mod flame;
mod sidebar;
mod timeline;
mod toolbar;

use std::{sync::Arc, time::Duration};

use self::{
  analysis::{AnalysisProps, AnalysisState, ProfilerAnalysis},
  capture::{FRAME_BUDGET_MS, ProfilerCapture, TimeRange, format_span},
  sidebar::DetailTab,
};
use super::{
  changes,
  kit::{FILL, line},
};
use crate::{
  app::{
    ctx::{Ctx, Interval},
    profiler::{
      ProfileReport, ProfilingHandle, RecordingOptions, ScreenshotOptions, SessionId, SessionStatus, SpanTrack,
    },
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify},
  node::Element,
  ui::{EmptyState, Length, ScrollArea, icons},
};

/// How often the status of a running recording is reread.
pub(crate) const STATUS_INTERVAL: Duration = Duration::from_millis(250);
/// Equal parts of the whole recording in the overview strip.
const OVERVIEW_BUCKETS: usize = 120;
/// The least the frames keep of a low window, and the least the analysis does: its header, the chart's bar, a
/// track of a few rows and the selected span.
const FRAMES_LEAST_HEIGHT: f32 = 120.0;
const ANALYSIS_LEAST_HEIGHT: f32 = 220.0;
/// How wide the board has what the tab says while it has nothing to show.
const EMPTY_WIDTH: f32 = 440.0;
// iOS has no file dialog to load a recording with.
#[cfg_attr(target_os = "ios", allow(dead_code))]
pub(crate) const LOAD_WHILE_RECORDING: &str = "Load skipped: a recording is running; Stop it, then Load again";

/// The Profiler tab: the recording Start opened on the inspected tree's
/// profiling handle, or the capture Stop finished or Load read, and what is
/// selected in it.
#[derive(Clone, Default)]
pub(crate) struct ProfilerState {
  recording: Option<Recording>,
  /// Records DevTools' own windows too; read when a recording starts.
  include_devtools: bool,
  /// Records component spans and render causes; read when a recording starts.
  detailed: bool,
  /// Takes screenshots of presented frames (`RecordingOptions::screenshots`);
  /// read when a recording starts. Off by default: each one reads a frame
  /// back from the GPU.
  screenshots: bool,
  /// The report `capture` was made from, which Save writes.
  report: Option<Arc<ProfileReport>>,
  capture: Option<Arc<ProfilerCapture>>,
  range: Option<TimeRange>,
  /// The overview bucket the range was pressed on; a shift-press extends from it.
  range_anchor: Option<usize>,
  /// True from a press on the overview until the button is released over it.
  range_dragging: bool,
  /// The selected pass's sample sequence.
  selected: Option<u64>,
  /// The filmstrip screenshot under the pointer, by its index in the
  /// capture's: the preview shows it instead of the selected pass's.
  hovered_screenshot: Option<usize>,
  error: Option<String>,
}

#[derive(Clone)]
struct Recording {
  handle: ProfilingHandle,
  id: SessionId,
  status: Option<SessionStatus>,
}

impl crate::app::component::DevtoolsInspectable for ProfilerState {
  fn write_info(&self, buffer: &mut Vec<crate::app::component::ComponentInfo>) {
    buffer.push(crate::app::component::ComponentInfo::with_value(
      "recording",
      std::any::type_name::<bool>(),
      self.recording.is_some().to_string(),
    ));
    buffer.push(crate::app::component::ComponentInfo::with_value(
      "passes",
      std::any::type_name::<usize>(),
      self
        .capture
        .as_ref()
        .map_or(0, |capture| capture.passes.len())
        .to_string(),
    ));
  }
}

impl ProfilerState {
  pub(crate) fn recording(&self) -> bool {
    self.recording.is_some()
  }

  /// Starts an unbounded recording session on `handle`, the inspected tree's,
  /// and forgets the previous capture. False when the session did not start.
  pub(crate) fn start(&mut self, handle: &ProfilingHandle) -> bool {
    if self.recording.is_some() {
      return true;
    }
    let options = RecordingOptions {
      include_devtools: self.include_devtools,
      detailed: self.detailed,
      screenshots: self.screenshots.then(ScreenshotOptions::default),
      ..RecordingOptions::default()
    };
    match handle.start_recording(options) {
      Ok(started) => {
        *self = Self {
          recording: Some(Recording {
            handle: handle.clone(),
            id: started.id,
            status: handle.status(started.id).ok(),
          }),
          include_devtools: self.include_devtools,
          detailed: self.detailed,
          screenshots: self.screenshots,
          ..Self::default()
        };
        true
      }
      Err(error) => {
        self.error = Some(error.to_string());
        false
      }
    }
  }

  /// Ends the recording and keeps every sample it took as the capture.
  pub(crate) fn stop(&mut self) {
    let Some(recording) = self.recording.take() else {
      return;
    };
    match recording.handle.end(recording.id) {
      Ok(report) => {
        // What went wrong during the recording (a file picked while it ran)
        // is not about the capture it made.
        self.error = None;
        self.show(report);
      }
      Err(error) => self.error = Some(error.to_string()),
    }
  }

  /// Shows the report of a recording file Save wrote
  /// (`ProfileReport::from_recording_file`) in place of the capture; keeps
  /// the capture when the file could not be read. A file picked while a
  /// recording runs (one started while the dialog was open) is not loaded,
  /// and the toolbar says so.
  #[cfg_attr(target_os = "ios", allow(dead_code))]
  pub(crate) fn load(&mut self, file: Result<ProfileReport, String>) {
    if self.recording.is_some() {
      self.error = Some(LOAD_WHILE_RECORDING.to_owned());
      return;
    }
    match file {
      Ok(report) => {
        *self = Self {
          include_devtools: self.include_devtools,
          detailed: self.detailed,
          screenshots: self.screenshots,
          ..Self::default()
        };
        self.show(report);
      }
      Err(error) => self.error = Some(error),
    }
  }

  fn show(&mut self, report: ProfileReport) {
    let capture = ProfilerCapture::from_report(&report);
    let range = capture.default_range();
    self.selected = capture.slowest_in(range);
    self.range = Some(range);
    self.capture = Some(Arc::new(capture));
    self.report = Some(Arc::new(report));
  }

  /// Rereads the running recording's duration and size.
  pub(crate) fn refresh_status(&mut self) {
    if let Some(recording) = &mut self.recording {
      recording.status = recording.handle.status(recording.id).ok();
    }
  }

  /// Ends a running recording without keeping it, and forgets the capture.
  pub(crate) fn clear(&mut self) {
    if let Some(recording) = self.recording.take() {
      let _ = recording.handle.end(recording.id);
    }
    *self = Self {
      include_devtools: self.include_devtools,
      detailed: self.detailed,
      screenshots: self.screenshots,
      ..Self::default()
    };
  }

  fn press_bucket(&mut self, bucket: usize, extend: bool) {
    let anchor = match (extend, self.range_anchor) {
      (true, Some(anchor)) => anchor,
      _ => bucket,
    };
    self.range_anchor = Some(anchor);
    self.range_dragging = true;
    self.select_buckets(anchor, bucket);
  }

  fn drag_bucket(&mut self, bucket: usize) {
    if let (true, Some(anchor)) = (self.range_dragging, self.range_anchor) {
      self.select_buckets(anchor, bucket);
    }
  }

  /// True when dragging onto `bucket` would change the range, so a move
  /// within one bucket renders nothing again.
  fn drag_changes_range(&self, bucket: usize) -> bool {
    match (self.range_dragging, self.range_anchor, &self.capture) {
      (true, Some(anchor), Some(capture)) => self.range != Some(capture.bucket_range(anchor, bucket, OVERVIEW_BUCKETS)),
      _ => false,
    }
  }

  fn select_buckets(&mut self, first: usize, last: usize) {
    if let Some(capture) = &self.capture {
      let range = capture.bucket_range(first, last, OVERVIEW_BUCKETS);
      self.set_range(range);
    }
  }

  fn set_range(&mut self, range: TimeRange) {
    let Some(capture) = &self.capture else {
      return;
    };
    let selected_in_range = self.selected.is_some_and(|sequence| {
      capture
        .passes_in(range)
        .iter()
        .any(|sample| sample.sequence == sequence)
    });
    if !selected_in_range {
      self.selected = capture.slowest_in(range);
    }
    self.range = Some(range);
  }

  /// The whole recording as the range, with no bucket to extend a shift-press from.
  fn show_whole(&mut self) {
    if let Some(whole) = self.capture.as_ref().map(|capture| capture.whole()) {
      self.range_anchor = None;
      self.set_range(whole);
    }
  }

  /// Selects the pass of sample `sequence`, where another is selected.
  fn select(&mut self, sequence: u64) {
    self.selected = Some(sequence);
  }
}

/// What the tab keeps between renders.
#[derive(Clone)]
pub(crate) struct ProfilerTab {
  pub(crate) state: Signal<ProfilerState>,
  /// Rereads a running recording's status; runs only while one does.
  ticker: Interval,
  /// Which list of the selected pass the sidebar shows.
  detail: Signal<DetailTab>,
  /// What is shown of the spans: the chart or a table, of the range or of the pass, where the chart is zoomed
  /// to and what is open in a table. It is held here so that it is kept while another tab is shown.
  analysis: Signal<AnalysisState>,
  /// The name of what is selected in the analysis, a span of the chart or a row of a table: it is picked out
  /// wherever it is listed, in the tables and among the phases of the selected pass. It is apart from the rest
  /// of the analysis, which changes as the pointer moves over the chart: the sidebar reads this alone.
  picked: Signal<Option<&'static str>>,
}

impl ProfilerTab {
  pub(crate) fn new(ctx: &mut Ctx) -> Self {
    let state = ctx.signal(ProfilerState::default());
    let ticker = ctx.create_interval(STATUS_INTERVAL, {
      let state = state.clone();
      move || state.update(ProfilerState::refresh_status)
    });
    Self {
      state,
      ticker,
      detail: ctx.signal(DetailTab::default()),
      analysis: ctx.signal(AnalysisState::default()),
      picked: ctx.signal(None),
    }
  }

  /// Ends what the tab left running: a recording would keep counting against the session limit.
  pub(crate) fn close(&self) {
    self.ticker.stop();
    self.state.update(ProfilerState::clear);
  }

  /// Selects a pass from a handler, where it is not the selected one already: a press of the selected bar
  /// renders nothing again.
  fn select_pass(state: &Signal<ProfilerState>, sequence: u64) {
    if state.with_untracked(|state| state.selected != Some(sequence)) {
      state.update(|state| state.select(sequence));
    }
  }
}

/// The tab: its toolbar, and under it what there is to show: nothing yet, a recording that runs, or a capture.
pub(super) fn view(ctx: &mut Ctx, tab: &ProfilerTab, handle: Option<ProfilingHandle>) -> Element {
  let state = tab.state.get();
  let body = body(ctx, tab, &state);
  Column::new()
    .size(FILL, FILL)
    .child(toolbar::toolbar(&state, tab, handle))
    .child(Column::new().width(FILL).flex(1.0).border_top(line()).child(body))
    .into()
}

fn body(ctx: &mut Ctx, tab: &ProfilerTab, state: &ProfilerState) -> Element {
  if state.recording() {
    return changes::centred(
      EmptyState::new("Recording")
        .description("Use the inspected app, then Stop recording to see every pass it made.")
        .media(recording_mark())
        .bordered(false)
        .width(EMPTY_WIDTH)
        .id("devtools-profiler-recording")
        .into(),
    );
  }
  let Some(capture) = state.capture.clone() else {
    return changes::centred(
      EmptyState::new("No recording")
        .description(
          "Start recording, use the inspected app, then Stop: the timeline shows every pass of its windows in between.",
        )
        .icon(icons::PULSE)
        .bordered(false)
        .width(EMPTY_WIDTH)
        .id("devtools-profiler-empty")
        .into(),
    );
  };
  let range = state.range.unwrap_or_else(|| capture.default_range());
  let analysis = ctx.mount::<ProfilerAnalysis>(AnalysisProps {
    capture: capture.clone(),
    range,
    selected: state.selected,
    state: tab.analysis.clone(),
    picked: tab.picked.clone(),
    on_select_pass: Some(Arc::new({
      let state = tab.state.clone();
      move |sequence| ProfilerTab::select_pass(&state, sequence)
    })),
  });
  let main = Column::new()
    .height(FILL)
    .flex(1.0)
    .min_width(0.0)
    .border_right(line())
    .child(timeline::recording(&capture, range, &tab.state))
    // In a window too low for the frames and the analysis, the frames give way and scroll: the analysis keeps
    // a height it can be read at. The chart of the analysis is in nothing that scrolls, as its wheel zooms.
    .child(
      Column::new()
        .width(FILL)
        .flex_shrink(1.0)
        .min_height(FRAMES_LEAST_HEIGHT)
        .child(
          ScrollArea::new()
            .label("Frames")
            .height(Length::Fill)
            .child(timeline::frames(&capture, range, state, &tab.state)),
        ),
    )
    // It grows from its least height and gives none of it up: the frames are what gives way.
    .child(
      Column::new()
        .width(FILL)
        .flex_full(1.0, 0.0, Some(ANALYSIS_LEAST_HEIGHT))
        .child(analysis),
    );
  Row::new()
    .size(FILL, FILL)
    .child(main)
    .child(sidebar::sidebar(&capture, state, tab))
    .into()
}

/// What stands over the word Recording: the dot of the toolbar's Stop, in the well an empty state's icon has.
fn recording_mark() -> Element {
  Row::new()
    .size(40.0, 40.0)
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .rounded(RadiusSize::Lg)
    .background(PaletteColor::Muted)
    .child(
      Row::new()
        .size(10.0, 10.0)
        .rounded(RadiusSize::Full)
        .background(PaletteColor::Destructive),
    )
    .into()
}

/// How many passes, with the word for them: `1 pass`, `72 passes`.
fn passes(count: usize) -> String {
  match count {
    1 => "1 pass".to_owned(),
    count => format!("{count} passes"),
  }
}

/// `at` as a time since the recording started.
fn format_offset(capture: &ProfilerCapture, at: f64) -> String {
  format!("+{}", format_span(at - capture.started_ms))
}

/// What a pass's time is to the frame budget: over twice it, over it, or within it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Budget {
  Within,
  Over,
  OverTwice,
}

impl Budget {
  pub(crate) fn of(ms: f64) -> Self {
    if ms > FRAME_BUDGET_MS * 2.0 {
      Self::OverTwice
    } else if ms > FRAME_BUDGET_MS {
      Self::Over
    } else {
      Self::Within
    }
  }

  /// An input-to-present latency: within three frame budgets (50 ms), within six, or above.
  pub(crate) fn of_latency(ms: f64) -> Self {
    if ms > FRAME_BUDGET_MS * 6.0 {
      Self::OverTwice
    } else if ms > FRAME_BUDGET_MS * 3.0 {
      Self::Over
    } else {
      Self::Within
    }
  }

  /// The color of a bar of it: the theme's warning and destructive over the budget, and `within` within it.
  fn color(self, within: PaletteColor) -> PaletteColor {
    match self {
      Self::Within => within,
      Self::Over => PaletteColor::Warning,
      Self::OverTwice => PaletteColor::Destructive,
    }
  }
}

/// The color a span is told apart by, from the theme's five of a chart: what the toolkit does to the tree, its
/// layout, what it makes of the layout to draw, the drawing, and what the app measured itself. `None` for a
/// name that is no phase of the toolkit's: a component, which is in the color of the phase it is in.
fn phase_color(name: &str) -> Option<PaletteColor> {
  Some(match name {
    crate::app::profiler::PASS_SPAN_NAME => PaletteColor::MutedForeground,
    "pass_setup" | "pass_notifications" | "ui_rebuild" | "ui_commit" | "input_dispatch" => PaletteColor::Chart1,
    "layout_update" | "layout_compute" | "component_after_layout" => PaletteColor::Chart2,
    "quad_resolve" | "glyph_rasterize" | "canvas_recording" | "canvas_preparation" => PaletteColor::Chart3,
    name if name.starts_with("render") || name.starts_with("canvas_") => PaletteColor::Chart4,
    _ => return None,
  })
}

/// The color of a span by its name alone, where what it is in is not known: a component is rebuilt, so it is
/// in the color of the tree's phases; an application scope in the color of its own.
fn span_color(name: &str, track: SpanTrack) -> PaletteColor {
  match track {
    SpanTrack::Frames => phase_color(name).unwrap_or(PaletteColor::Chart1),
    SpanTrack::Application(_) => PaletteColor::Chart5,
  }
}

#[cfg(test)]
mod tests;
