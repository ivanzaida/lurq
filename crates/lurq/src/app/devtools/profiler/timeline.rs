//! The two zones over the analysis: the whole recording in a strip a range is selected on, and the passes of
//! that range on a real time axis, with what else happened in it track by track.
//!
//! Every track of the frames is drawn on one time scale: its room less the width of a bar, so that a bar at the
//! range's end stays inside. A time is at the same place in every track.

use std::sync::Arc;

use super::{
  Budget, OVERVIEW_BUCKETS, ProfilerState, ProfilerTab,
  capture::{
    self, CaptureScreenshot, Counter, FRAME_BUDGET_MS, MAX_TIMELINE_PASSES, ProfilerCapture, TimeRange, bucket_of,
    format_bytes, format_ms, format_span, ms, pass_of,
  },
  format_offset,
};
use crate::{
  app::{
    devtools::kit::{FILL, gives_way, line, mono, sans},
    events::MouseEvent,
    profiler::{ProfileSample, SampleData},
    theme::{BorderSize, PaletteColor},
  },
  components::{Column, Row, Spacer, Stack},
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, HitTestBehavior, border::Border},
  ui::{Button, ButtonSize, ButtonVariant, EmptyState, Separator, icons},
};

/// The sizes the board gives the zones (`pencil/lurq-devtools.pen`, Profiler · dark).
const PADDING: f32 = 20.0;
const TITLE_HEIGHT: f32 = 36.0;
const LABEL_WIDTH: f32 = 140.0;
const LABEL_GAP: f32 = 12.0;
const OVERVIEW_HEIGHT: f32 = 40.0;
const FILMSTRIP_HEIGHT: f32 = 40.0;
const PASSES_HEIGHT: f32 = 88.0;
pub(super) const GPU_HEIGHT: f32 = 22.0;
pub(super) const COUNTER_HEIGHT: f32 = 16.0;
const LANE_HEIGHT: f32 = 10.0;
/// The room the board's tracks have, which the width of a bar is worked out for.
const NOMINAL_TRACK: f32 = 908.0;
/// Equal parts of the selected range in the lanes of marks.
const MARKER_BUCKETS: usize = 240;
/// Equal parts of the selected range in the filmstrip, each showing the
/// first screenshot placed in it.
const FILMSTRIP_CELLS: usize = 16;

/// The element id of the filmstrip thumbnail of the capture's screenshot `index`.
pub(crate) fn filmstrip_id(index: usize) -> String {
  format!("devtools-profiler-screenshot-{index}")
}

/// The title of a zone, and what is said of the zone beside it.
fn zone_title(title: &str, said: &str) -> Row {
  Row::new()
    .width(FILL)
    .height(TITLE_HEIGHT)
    .padding_horizontal(PADDING)
    .spacing(10.0)
    .align_items(Alignment::Center)
    .child(sans(title, 13.0, FontWeight::Medium))
    .child(gives_way(
      mono(said, 12.0, FontWeight::Normal).color(PaletteColor::MutedForeground),
      1.0,
    ))
}

fn quiet(text: &str) -> crate::components::Text {
  sans(text, 12.0, FontWeight::Normal).color(PaletteColor::MutedForeground)
}

/// The element id of the row of the track called `name`: `devtools-profiler-track-draw-calls`.
pub(crate) fn track_id(name: &str) -> String {
  format!("devtools-profiler-track-{}", name.replace(' ', "-"))
}

/// A row of a zone: what the track is called and what it comes to, then the track, which must flex.
fn track(name: &str, value: impl Into<Element>, height: f32, align: Alignment, body: impl Into<Element>) -> Row {
  Row::new()
    .id(track_id(name))
    .width(FILL)
    .height(height)
    .padding_horizontal(PADDING)
    .spacing(LABEL_GAP)
    .align_items(align)
    .child(
      Row::new()
        .width(LABEL_WIDTH)
        .align_items(align)
        .child(mono(name, 11.0, FontWeight::Normal).color(PaletteColor::MutedForeground))
        .child(Spacer::new().flex(1.0))
        .child(value),
    )
    .child(body)
}

fn value(text: &str) -> Element {
  mono(text, 11.0, FontWeight::Normal).into()
}

fn no_value(text: &str) -> Element {
  mono(text, 11.0, FontWeight::Normal)
    .color(PaletteColor::MutedForeground)
    .into()
}

/// The whole recording, one column per equal part with its longest pass;
/// press, drag or shift-press over it to select the range of the frames.
pub(super) fn recording(capture: &ProfilerCapture, range: TimeRange, state: &Signal<ProfilerState>) -> Element {
  let whole = capture.whole();
  let longest = capture.overview(OVERVIEW_BUCKETS);
  let top = longest.iter().copied().fold(0.0, f64::max);
  let scale = top.max(FRAME_BUDGET_MS * 2.0);
  let first = bucket_of(whole, range.start_ms, OVERVIEW_BUCKETS);
  let last = bucket_of(whole, range.end_ms, OVERVIEW_BUCKETS);
  let edge = || Border::inside(BorderSize::Sm, PaletteColor::Foreground);
  let mut strip = Row::new()
    .flex(1.0)
    .height(OVERVIEW_HEIGHT)
    .align_items(Alignment::End)
    .id("devtools-profiler-overview")
    .on_mouse_up({
      let state = state.clone();
      move |_| {
        if state.with_untracked(|state| state.range_dragging) {
          state.update(|state| state.range_dragging = false);
        }
      }
    });
  for (bucket, longest_ms) in longest.into_iter().enumerate() {
    let in_range = (first..=last).contains(&bucket);
    let mut column = Row::new()
      .flex(1.0)
      .height(FILL)
      .padding_horizontal(1.0)
      .align_items(Alignment::End)
      .cursor(CursorIcon::Pointer)
      .on_mouse_down({
        let state = state.clone();
        move |event: MouseEvent| state.update(|state| state.press_bucket(bucket, event.shift))
      })
      .on_mouse_move({
        let state = state.clone();
        move |_| {
          if state.with_untracked(|state| state.drag_changes_range(bucket)) {
            state.update(|state| state.drag_bucket(bucket));
          }
        }
      });
    if in_range {
      column = column.background(PaletteColor::Muted);
      if bucket == first {
        column = column.border_left(edge());
      }
      if bucket == last {
        column = column.border_right(edge());
      }
    }
    if longest_ms > 0.0 {
      column = column.child(
        Row::new()
          .width(FILL)
          .height(((longest_ms / scale) as f32 * OVERVIEW_HEIGHT).max(2.0))
          .rounded(1.0)
          .background(Budget::of(longest_ms).color(PaletteColor::Input))
          .opacity(if in_range { 1.0 } else { 0.45 }),
      );
    }
    strip = strip.child(column);
  }

  let mut title = zone_title(
    "Recording",
    &format!(
      "{} · {} · {}",
      format_span(whole.duration_ms()),
      super::passes(capture.passes.len()),
      format_bytes(capture.size_bytes)
    ),
  );
  if capture.stopped_at_limit {
    title = title.child(sans("stopped at its size limit", 12.0, FontWeight::Normal).color(PaletteColor::Warning));
  }
  if capture.boundary_excluded > 0 {
    title = title.child(quiet(&format!(
      "{} begun before Start, left out",
      capture.boundary_excluded
    )));
  }
  let whole_recording = state.clone();
  Column::new()
    .width(FILL)
    .padding_bottom(12.0)
    .border_bottom(line())
    .child(
      title
        .child(Spacer::new().flex(1.0))
        .child(gives_way(quiet("Drag or shift-click to select a range"), 100.0))
        // The button keeps its width: what is said before it gives way.
        .child(
          Row::new().flex_shrink(0.0).child(
            Button::new("Whole recording")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Xs)
              .icon(icons::REFRESH)
              .on_click(move |_| whole_recording.update(ProfilerState::show_whole))
              .id("devtools-profiler-whole"),
          ),
        ),
    )
    .child(track(
      "longest pass",
      value(&format_ms(top)),
      OVERVIEW_HEIGHT,
      Alignment::End,
      strip,
    ))
    .into()
}

/// The selected range on a real time axis: one bar per pass at its start,
/// its height the pass's duration, with the frame budget line, the tracks of what else was measured of each
/// pass, the lanes of what happened between them, and the axis.
pub(super) fn frames(
  capture: &Arc<ProfilerCapture>,
  range: TimeRange,
  state: &ProfilerState,
  signal: &Signal<ProfilerState>,
) -> Element {
  let passes = capture.passes_in(range);
  let fps = capture::fps(passes).map_or_else(|| "–".to_owned(), |fps| format!("{fps:.1}"));
  let over_budget = passes
    .iter()
    .filter_map(|sample| pass_of(sample))
    .filter(|pass| ms(pass.total) > FRAME_BUDGET_MS)
    .count();
  let missed = passes.iter().filter(|sample| capture::missed_vsync(sample)).count();
  let counted = |count: usize, what: &str, color: PaletteColor| {
    sans(&format!("{count} {what}"), 12.0, FontWeight::Normal).color(if count > 0 {
      color
    } else {
      PaletteColor::MutedForeground
    })
  };
  let title = zone_title(
    "Frames",
    &format!(
      "{} – {} ({})",
      format_offset(capture, range.start_ms),
      format_offset(capture, range.end_ms),
      format_span(range.duration_ms())
    ),
  )
  .child(Spacer::new().flex(1.0))
  .child(quiet(&super::passes(passes.len())))
  .child(counted(over_budget, "over budget", PaletteColor::Warning))
  .child(counted(missed, "missed vsync", PaletteColor::Destructive))
  .child(Separator::vertical().length(16.0))
  .child(quiet("FPS"))
  .child(mono(&fps, 12.0, FontWeight::Medium).id("devtools-profiler-fps"));

  let mut zone = Column::new()
    .width(FILL)
    .padding_bottom(6.0)
    .spacing(6.0)
    .border_bottom(line())
    .id("devtools-profiler-frames")
    .child(title);
  if passes.len() > MAX_TIMELINE_PASSES {
    return zone
      .child(
        Column::new()
          .width(FILL)
          .height(324.0)
          .align_items(Alignment::Center)
          .justify(Justify::Center)
          .child(
            EmptyState::new(&format!("{} passes in this range", passes.len()))
              .description(&format!(
                "Select at most {MAX_TIMELINE_PASSES} in the recording above to draw each one."
              ))
              .bordered(false)
              .id("devtools-profiler-too-many"),
          ),
      )
      .into();
  }

  let view = View {
    capture,
    passes,
    range,
    selected: state.selected,
    bar: bar_width(passes.len()),
    signal,
  };
  if !capture.screenshots.is_empty() {
    zone = zone.child(view.filmstrip(state.hovered_screenshot));
  }
  // Missed vsyncs sit right under the bars they mark, then the frames that
  // paid for a screenshot; GPU time and the counters follow.
  zone = zone.child(view.pass_bars()).child(view.missed_vsync());
  if passes
    .iter()
    .any(|sample| pass_of(sample).is_some_and(|pass| pass.screenshot))
  {
    zone = zone.child(view.captured());
  }
  zone = zone.child(view.gpu());
  for counter in Counter::ALL {
    zone = zone.child(view.counter(counter));
  }
  zone
    .child(view.input())
    .child(view.ui_updates())
    .child(view.axis())
    .child(legend())
    .into()
}

/// How wide a bar of a pass is: the board's nine where the passes are as few as it draws, and less where they
/// are more, down to two.
pub(super) fn bar_width(passes: usize) -> f32 {
  (NOMINAL_TRACK / passes.max(1) as f32 * 0.7).round().clamp(2.0, 9.0)
}

/// `content` at `fraction` of the width less its own, in a full-size layer
/// that only its content hits.
fn placed(fraction: f32, content: impl Into<Element>, align: Alignment) -> Element {
  let mut row = Row::new()
    .align_items(align)
    .width(FILL)
    .height(FILL)
    .hit_test(HitTestBehavior::ContentOnly);
  if fraction > 0.0 {
    row = row.child(Spacer::new().flex(fraction).pointer_events_none());
  }
  row = row.child(content);
  if fraction < 1.0 {
    row = row.child(Spacer::new().flex(1.0 - fraction).pointer_events_none());
  }
  row.into()
}

/// The range that is drawn, and what every track of it is drawn with.
struct View<'a> {
  capture: &'a Arc<ProfilerCapture>,
  passes: &'a [Arc<ProfileSample>],
  range: TimeRange,
  selected: Option<u64>,
  /// The width of a bar of a pass.
  bar: f32,
  signal: &'a Signal<ProfilerState>,
}

impl View<'_> {
  /// A bar of pass `sequence` at `started_ms`, in `color` where the pass is not the selected one, which is
  /// never faded: a press of it selects the pass.
  fn bar(&self, sequence: u64, started_ms: f64, height: f32, color: PaletteColor, faded: bool) -> Element {
    let signal = self.signal.clone();
    let selected = self.selected == Some(sequence);
    let bar = Row::new()
      .width(self.bar)
      .height(height)
      .rounded(1.0)
      .background(if selected { PaletteColor::Foreground } else { color })
      .opacity(if faded && !selected { 0.4 } else { 1.0 })
      .cursor(CursorIcon::Pointer)
      .on_click(move |_| ProfilerTab::select_pass(&signal, sequence));
    placed(self.range.fraction(started_ms) as f32, bar, Alignment::End)
  }

  /// The selected pass, where it is one of those drawn.
  fn selected_sample(&self) -> Option<&ProfileSample> {
    let selected = self.selected?;
    self
      .passes
      .iter()
      .find(|sample| sample.sequence == selected)
      .map(Arc::as_ref)
  }

  /// One bar per pass, as high as the pass was long, over the line of the frame budget.
  fn pass_bars(&self) -> Row {
    let longest = self
      .passes
      .iter()
      .filter_map(|sample| pass_of(sample))
      .map(|pass| ms(pass.total))
      .fold(0.0, f64::max);
    let scale = longest.max(FRAME_BUDGET_MS * 2.0);
    let budget_height = (FRAME_BUDGET_MS / scale) as f32 * PASSES_HEIGHT;
    // Behind the bars and never hit, so every bar stays clickable.
    let budget = Column::new()
      .size(FILL, FILL)
      .pointer_events_none()
      .child(Spacer::new().flex(1.0))
      .child(
        Row::new()
          .width(FILL)
          .justify(Justify::End)
          .padding_right(2.0)
          .padding_bottom(2.0)
          .child(
            mono(
              &format!("budget {}", format_ms(FRAME_BUDGET_MS)),
              10.0,
              FontWeight::Normal,
            )
            .color(PaletteColor::Warning),
          ),
      )
      .child(
        Row::new()
          .width(FILL)
          .height(1.0)
          .background(PaletteColor::Warning)
          .opacity(0.7),
      )
      .child(Spacer::new().height((budget_height - 1.0).max(0.0)));
    let mut chart = Stack::new().flex(1.0).height(PASSES_HEIGHT).child(budget);
    for sample in self.passes {
      let Some(pass) = pass_of(sample) else {
        continue;
      };
      let total = ms(pass.total);
      chart = chart.child(self.bar(
        sample.sequence,
        sample.started_ms,
        ((total / scale) as f32 * PASSES_HEIGHT).max(2.0),
        Budget::of(total).color(PaletteColor::Input),
        // A pass that presented nothing.
        !pass.rendered,
      ));
    }
    let shown = self
      .selected_sample()
      .and_then(pass_of)
      .map_or(longest, |pass| ms(pass.total));
    track("passes", value(&format_ms(shown)), PASSES_HEIGHT, Alignment::End, chart)
  }

  /// A lane of equal parts of the time scale: a mark in its color where a part has one, the lane's line where
  /// it has none.
  fn lane(&self, name: &str, said: impl Into<Element>, cells: &[Option<PaletteColor>]) -> Row {
    // The parts span the time scale, the track less a bar.
    let mut lane = Row::new()
      .flex(1.0)
      .height(LANE_HEIGHT)
      .padding_right(self.bar)
      .align_items(Alignment::Center);
    // One cell per run of equal parts keeps the lane small.
    let mut start = 0;
    while start < cells.len() {
      let mark = &cells[start];
      let end = cells[start..]
        .iter()
        .position(|cell| cell != mark)
        .map_or(cells.len(), |offset| start + offset);
      let cell = Row::new().flex((end - start) as f32);
      lane = lane.child(match mark {
        Some(color) => cell.height(8.0).rounded(1.0).background(color.clone()),
        None => cell.height(2.0).background(PaletteColor::Border),
      });
      start = end;
    }
    track(name, said, LANE_HEIGHT, Alignment::Center, lane)
  }

  /// The frames that missed a vsync inside a run of frames, under their bars.
  fn missed_vsync(&self) -> Row {
    let (missed, count) = self.capture.missed_vsync_buckets(self.range, MARKER_BUCKETS);
    let cells: Vec<_> = missed
      .into_iter()
      .map(|missed| missed.then_some(PaletteColor::Destructive))
      .collect();
    self.lane("missed vsync", value(&count.to_string()), &cells)
  }

  /// The passes whose frame was copied for a screenshot, under their bars:
  /// their render timings include that copy.
  fn captured(&self) -> Row {
    let mut cells = vec![None; MARKER_BUCKETS];
    let mut count = 0;
    for sample in self.passes {
      if pass_of(sample).is_some_and(|pass| pass.screenshot) {
        cells[bucket_of(self.range, sample.started_ms, MARKER_BUCKETS)] = Some(PaletteColor::MutedForeground);
        count += 1;
      }
    }
    self.lane("captured", value(&count.to_string()), &cells)
  }

  /// The GPU time of each pass that has one (UI and Canvas passes, from wgpu
  /// timestamp queries), a bar at the pass's start; or why none was measured.
  fn gpu(&self) -> Row {
    let measured: Vec<_> = self
      .passes
      .iter()
      .filter_map(|sample| Some((sample, ms(pass_of(sample)?.gpu?.total()))))
      .collect();
    let mut chart = Stack::new().flex(1.0).height(GPU_HEIGHT);
    if measured.is_empty() {
      let note = if self.capture.gpu_timestamps.available {
        "No GPU results arrived for these passes".to_owned()
      } else {
        format!("GPU time not measured: {}", self.capture.gpu_missing_reason())
      };
      let note = Row::new()
        .size(FILL, FILL)
        .align_items(Alignment::Center)
        .child(sans(&note, 11.0, FontWeight::Normal).color(PaletteColor::MutedForeground));
      return track("gpu", no_value("–"), GPU_HEIGHT, Alignment::End, chart.child(note));
    }
    let longest = measured.iter().map(|(_, gpu)| *gpu).fold(0.0, f64::max);
    let scale = longest.max(FRAME_BUDGET_MS);
    for (sample, gpu) in &measured {
      chart = chart.child(self.bar(
        sample.sequence,
        sample.started_ms,
        ((gpu / scale) as f32 * GPU_HEIGHT).max(2.0),
        PaletteColor::Input,
        false,
      ));
    }
    let shown = measured
      .iter()
      .find(|(sample, _)| self.selected == Some(sample.sequence))
      .map_or(longest, |(_, gpu)| *gpu);
    track("gpu", value(&format_ms(shown)), GPU_HEIGHT, Alignment::End, chart)
  }

  /// One counter's value per pass, a bar at the pass's start as in the passes' own track. What it comes to is
  /// the selected pass's value, or the range's highest, or that the counter is unavailable.
  fn counter(&self, counter: Counter) -> Row {
    let mut chart = Stack::new().flex(1.0).height(COUNTER_HEIGHT);
    let Some(points) = capture::counter_points(counter, self.passes) else {
      return track(
        counter.label(),
        no_value("unavailable"),
        COUNTER_HEIGHT,
        Alignment::End,
        chart,
      );
    };
    let highest = points.iter().map(|point| point.value).fold(0.0, f64::max);
    for point in &points {
      let height = if highest > 0.0 {
        (point.value / highest) as f32 * COUNTER_HEIGHT
      } else {
        0.0
      };
      chart = chart.child(self.bar(
        point.sequence,
        point.started_ms,
        height.max(1.0),
        PaletteColor::Border,
        false,
      ));
    }
    let shown = points
      .iter()
      .find(|point| self.selected == Some(point.sequence))
      .map(|point| point.value)
      .or_else(|| (!points.is_empty()).then_some(highest));
    let said = match shown {
      Some(shown) => value(&counter.format(shown)),
      None => no_value("–"),
    };
    track(counter.label(), said, COUNTER_HEIGHT, Alignment::End, chart)
  }

  /// Each input where the shell received it: in the warning and the destructive color where the longest
  /// input-to-present latency of its part is long.
  fn input(&self) -> Row {
    let buckets = self.capture.input_buckets(self.range, MARKER_BUCKETS);
    let count: usize = buckets.iter().map(|bucket| bucket.inputs).sum();
    let longest = buckets
      .iter()
      .filter_map(|bucket| bucket.max_latency_ms)
      .fold(None, |max: Option<f64>, latency| {
        Some(max.map_or(latency, |max| max.max(latency)))
      });
    let cells: Vec<_> = buckets
      .iter()
      .map(|bucket| {
        (bucket.inputs > 0).then(|| {
          bucket.max_latency_ms.map_or(PaletteColor::MutedForeground, |latency| {
            Budget::of_latency(latency).color(PaletteColor::MutedForeground)
          })
        })
      })
      .collect();
    let said = match longest {
      Some(longest) => format!("{count} · max {}", format_ms(longest)),
      None => count.to_string(),
    };
    self.lane("input", value(&said), &cells)
  }

  /// The updates of the UI where they began: those that led to the selected pass stand out.
  fn ui_updates(&self) -> Row {
    let own: Vec<usize> = self
      .selected_sample()
      .map(|sample| {
        self
          .capture
          .markers_in(self.range)
          .iter()
          .filter(|marker| matches!(marker.data, SampleData::UiUpdate(_)))
          .filter(|marker| led_to(self.capture, marker, sample))
          .map(|marker| bucket_of(self.range, marker.started_ms, MARKER_BUCKETS))
          .collect()
      })
      .unwrap_or_default();
    let mut cells = vec![None; MARKER_BUCKETS];
    let mut count = 0;
    for marker in self.capture.markers_in(self.range) {
      if matches!(marker.data, SampleData::UiUpdate(_)) {
        cells[bucket_of(self.range, marker.started_ms, MARKER_BUCKETS)] = Some(PaletteColor::MutedForeground);
        count += 1;
      }
    }
    for bucket in own {
      cells[bucket] = Some(PaletteColor::Foreground);
    }
    self.lane("ui update", value(&count.to_string()), &cells)
  }

  /// The recording's screenshots over equal parts of the time scale, each part
  /// showing the first one placed in it, at its frame's pass. Hovering one
  /// previews it in the sidebar; a click selects its pass.
  fn filmstrip(&self, hovered: Option<usize>) -> Row {
    let screenshots = &self.capture.screenshots;
    let first = screenshots.partition_point(|shot| shot.at_ms < self.range.start_ms);
    let shown = self.capture.screenshots_in(self.range);
    let mut cells = [None; FILMSTRIP_CELLS];
    for (offset, shot) in shown.iter().enumerate() {
      cells[bucket_of(self.range, shot.at_ms, FILMSTRIP_CELLS)].get_or_insert(first + offset);
    }
    // The parts span the time scale, the track less a bar.
    let mut strip = Row::new().flex(1.0).height(FILMSTRIP_HEIGHT).padding_right(self.bar);
    for cell in cells {
      let mut part = Row::new().height(FILL).flex(1.0).padding_horizontal(2.0);
      if let Some(index) = cell {
        let shot = &screenshots[index];
        let picked = hovered == Some(index) || (shot.sequence.is_some() && shot.sequence == self.selected);
        let sequence = shot.sequence;
        let (entered, left, pressed) = (self.signal.clone(), self.signal.clone(), self.signal.clone());
        part = part.child(
          Row::new()
            .id(filmstrip_id(index))
            .size(FILL, FILL)
            .rounded(2.0)
            .clip()
            .background(PaletteColor::Muted)
            .border(Border::inside(
              BorderSize::Sm,
              if picked {
                PaletteColor::Foreground
              } else {
                PaletteColor::Border
              },
            ))
            .cursor(CursorIcon::Pointer)
            .child(screenshot_image(shot))
            .on_mouse_enter(move || {
              if entered.with_untracked(|state| state.hovered_screenshot != Some(index)) {
                entered.update(|state| state.hovered_screenshot = Some(index));
              }
            })
            .on_mouse_leave(move || {
              if left.with_untracked(|state| state.hovered_screenshot == Some(index)) {
                left.update(|state| state.hovered_screenshot = None);
              }
            })
            .on_click(move |_| {
              if let Some(sequence) = sequence {
                ProfilerTab::select_pass(&pressed, sequence);
              }
            }),
        );
      }
      strip = strip.child(part);
    }
    track(
      "screenshots",
      value(&shown.len().to_string()),
      FILMSTRIP_HEIGHT,
      Alignment::End,
      strip,
    )
  }

  /// The time at each quarter of the time scale, written from where it is; the range's end is written up to
  /// where it is.
  fn axis(&self) -> Row {
    const TICKS: usize = 4;
    let time =
      |at: f64| mono(&format_offset(self.capture, at), 10.0, FontWeight::Normal).color(PaletteColor::MutedForeground);
    let mut ticks = Row::new()
      .flex(1.0)
      .padding_right(self.bar)
      .id("devtools-profiler-axis");
    for tick in 0..TICKS {
      let at = self.range.start_ms + self.range.duration_ms() * tick as f64 / TICKS as f64;
      // A time gives way to the one after it where the track is too narrow for both.
      let mut part = Row::new().flex(1.0).child(
        Row::new()
          .flex_full(1.0, 1.0, Some(0.0))
          .min_width(0.0)
          .clip()
          .child(time(at)),
      );
      if tick + 1 == TICKS {
        part = part.child(time(self.range.end_ms));
      }
      ticks = ticks.child(part);
    }
    Row::new()
      .width(FILL)
      .padding_horizontal(PADDING)
      .spacing(LABEL_GAP)
      .child(Spacer::new().width(LABEL_WIDTH))
      .child(ticks)
  }
}

/// Whether the update `marker` is one of those that led to the pass `sample`.
fn led_to(capture: &ProfilerCapture, marker: &ProfileSample, sample: &ProfileSample) -> bool {
  let SampleData::UiUpdate(update) = &marker.data else {
    return false;
  };
  capture.updates_of(sample).iter().any(|own| std::ptr::eq(*own, update))
}

/// What the colors of the bars say.
fn legend() -> Row {
  let swatch = |color: PaletteColor, label: &str| {
    Row::new()
      .spacing(6.0)
      .align_items(Alignment::Center)
      .child(Row::new().size(8.0, 8.0).rounded(2.0).background(color))
      .child(sans(label, 11.0, FontWeight::Normal).color(PaletteColor::MutedForeground))
  };
  // What it says of the bars goes onto a row of its own where the window is too narrow for one.
  Row::new()
    .width(FILL)
    .min_height(24.0)
    .padding_horizontal(PADDING)
    .spacing(16.0)
    .align_items(Alignment::Center)
    .justify(Justify::SpaceBetween)
    .wrap()
    .child(
      Row::new()
        .spacing(16.0)
        .align_items(Alignment::Center)
        .child(Spacer::new().width(LABEL_WIDTH - 4.0))
        .child(swatch(PaletteColor::Input, "within budget"))
        .child(swatch(PaletteColor::Warning, "over budget"))
        .child(swatch(PaletteColor::Destructive, "over 2× budget"))
        .child(swatch(PaletteColor::Foreground, "selected")),
    )
    .child(
      sans(
        "faded: not presented · click a bar to inspect it",
        11.0,
        FontWeight::Normal,
      )
      .color(PaletteColor::MutedForeground),
    )
}

/// `shot`'s pixels filling the box, aspect kept; an empty box in a build
/// without the `raster` feature, which draws no image.
// The engine's image node, which `ui::Picture` draws with: a frame of the recording is pixels at hand.
#[allow(deprecated)]
pub(super) fn screenshot_image(shot: &CaptureScreenshot) -> Element {
  #[cfg(feature = "raster")]
  {
    crate::components::Image::new(shot.image.clone())
      .contain()
      .width(FILL)
      .height(FILL)
      .into()
  }
  #[cfg(not(feature = "raster"))]
  {
    let _ = shot;
    Row::new().width(FILL).height(FILL).into()
  }
}
