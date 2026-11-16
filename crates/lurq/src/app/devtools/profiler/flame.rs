//! The flame chart of an analysis: its spans to scale, a track after the other, each span a block under the
//! one it is in. The wheel zooms, a drag pans, the pointer says what a span took and a press selects it.

use std::sync::Arc;

use super::{
  analysis::{
    Analysis, AnalysisState, BLOCK_HEIGHT, Block, ChartGeometry, ChartRoom, Drag, Hover, MAX_TRACK_ROWS, PADDING,
    ROW_HEIGHT, TRACK_HEADER_HEIGHT, TrackRows, hit, row_blocks, scroll_limit,
  },
  capture::{TimeRange, format_ms, format_span},
  phase_color,
};
use crate::{
  app::{
    devtools::kit::{FILL, gives_way, line, mono, sans},
    events::{MouseEvent, ScrollEvent},
    profiler::{ApplicationLane, ForestSpan, PASS_SPAN_NAME, SpanTrack},
    theme::{PaletteColor, TypographyStyle},
  },
  components::{Column, Row, Spacer, Stack},
  core::{ElementRef, Signal},
  layout::{Alignment, text_style::FontWeight},
  node::{CursorIcon, Element, border::Border},
  ui::{Button, ButtonSize, ButtonVariant, Tooltip, TooltipPart},
};

/// Element ids of the chart, what the pointer says of a span, the time the chart shows and the selected span.
pub(crate) const CHART_ID: &str = "devtools-profiler-flame-chart";
pub(crate) const TOOLTIP_ID: &str = "devtools-profiler-flame-tooltip";
pub(crate) const VIEW_ID: &str = "devtools-profiler-flame-view";
pub(crate) const SELECTION_ID: &str = "devtools-profiler-flame-selection";
/// Blocks at least this wide carry their span's name.
const LABEL_MIN_PX: f32 = 48.0;
/// A press that moves less than this before its release is a click, not a pan.
const DRAG_THRESHOLD_PX: f32 = 3.0;
/// Zoom factor per logical pixel of wheel delta.
const ZOOM_PER_PX: f64 = 0.004;
const TOOLTIP_WIDTH: f32 = 230.0;
/// The three lines of what the pointer says, with the room around them.
const TOOLTIP_HEIGHT: f32 = 66.0;

pub(super) fn chart(
  analysis: &Arc<Analysis>,
  state: &AnalysisState,
  signal: Signal<AnalysisState>,
  picked: Signal<Option<&'static str>>,
  room: ChartRoom,
  on_select_pass: Option<Arc<dyn Fn(u64) + Send + Sync>>,
) -> Element {
  let (width, chart_ref, room_ref) = (room.width, room.chart, room.room);
  let view = state.view_of(analysis);
  let geometry = Arc::new(ChartGeometry::of(analysis));
  // A chart higher than its room is moved up by as far as it was dragged, and no further than its last row.
  let scroll_y = state.scroll_y.min(scroll_limit(geometry.height, room.height));
  let mut chart = Stack::new()
    .width(FILL)
    .height(geometry.height)
    .offset(0.0, -scroll_y)
    .clip()
    .cursor(CursorIcon::Pointer)
    .ref_element(chart_ref.clone())
    .id(CHART_ID);
  for (track, (top, rows)) in analysis.tracks.iter().zip(&geometry.tracks) {
    chart = chart.child(
      Row::new()
        .align_items(Alignment::Center)
        .absolute(
          0.0,
          top - TRACK_HEADER_HEIGHT,
          width.max(1.0),
          TRACK_HEADER_HEIGHT - 2.0,
        )
        .pointer_events_none()
        .child(mono(&track_label(track), 11.0, FontWeight::Normal).color(PaletteColor::MutedForeground)),
    );
    for (depth, row) in track.rows.iter().take(*rows).enumerate() {
      let y = top + depth as f32 * ROW_HEIGHT;
      for block in row_blocks(&analysis.forest, row, view, width) {
        chart = chart.child(block_element(analysis, state, block, y));
      }
    }
  }
  if let Some(hover) = state.hovered {
    chart = chart.child(tooltip(analysis, hover, width, geometry.height));
  }
  let chart = chart
    .on_mouse_move({
      let (analysis, geometry, signal, chart_ref, room_ref) = (
        analysis.clone(),
        geometry.clone(),
        signal.clone(),
        chart_ref.clone(),
        room_ref.clone(),
      );
      move |event: MouseEvent| {
        let (x, y) = local(&chart_ref, &event);
        let width = chart_ref.width();
        let next = signal.with_untracked(|state| {
          let state = state.for_analysis(&analysis);
          match state.drag {
            Some(drag) => {
              let moved =
                drag.moved || (x - drag.x).abs() >= DRAG_THRESHOLD_PX || (event.y - drag.y).abs() >= DRAG_THRESHOLD_PX;
              moved.then(|| {
                let delta_ms = -f64::from(x - drag.x) * drag.view.duration_ms() / f64::from(width.max(1.0));
                let mut next = state.clone();
                // A drag straight down leaves the time the chart shows as it is: all of it, where it was.
                if delta_ms != 0.0 {
                  next.pan(&analysis, drag.view, delta_ms);
                }
                // Down the chart it is dragged by the pointer's way in the window, the chart moving under it.
                next.scroll_y =
                  (drag.scroll_y - (event.y - drag.y)).clamp(0.0, scroll_limit(geometry.height, room_ref.height()));
                next.drag = Some(Drag { moved, ..drag });
                next.hovered = None;
                next
              })
            }
            None => {
              let span = hit(&analysis, &geometry, state.view_of(&analysis), width, x, y);
              (span != state.hovered.map(|hover| hover.span)).then(|| {
                let mut next = state.clone();
                next.hovered = span.map(|span| Hover { span, x, y });
                next
              })
            }
          }
        });
        if let Some(next) = next {
          signal.set(next);
        }
      }
    })
    .on_mouse_down({
      let (analysis, signal, chart_ref) = (analysis.clone(), signal.clone(), chart_ref.clone());
      move |event: MouseEvent| {
        let (x, _) = local(&chart_ref, &event);
        signal.update(|state| {
          state.sync(analysis.key);
          state.drag = Some(Drag {
            x,
            y: event.y,
            view: state.view_of(&analysis),
            scroll_y: state.scroll_y,
            moved: false,
          });
        });
      }
    })
    .on_mouse_up({
      let (analysis, geometry, signal, chart_ref) =
        (analysis.clone(), geometry.clone(), signal.clone(), chart_ref.clone());
      move |event: MouseEvent| {
        let (x, y) = local(&chart_ref, &event);
        let width = chart_ref.width();
        let (mut clicked, mut picked_pass, mut name) = (false, None, None);
        signal.update(|state| {
          state.sync(analysis.key);
          clicked = state.drag.take().is_none_or(|drag| !drag.moved);
          if clicked {
            state.selected_span = hit(&analysis, &geometry, state.view_of(&analysis), width, x, y);
            name = state.selected_span.map(|span| analysis.span(span).name);
            picked_pass = state.selected_span.and_then(|span| analysis.span(span).sample);
          }
        });
        // Its name is what the tables and the sidebar pick out.
        if clicked && picked.get_untracked() != name {
          picked.set(name);
        }
        if let (Some(sequence), Some(on_select_pass)) = (picked_pass, &on_select_pass) {
          on_select_pass(sequence);
        }
      }
    })
    .on_mouse_leave({
      let signal = signal.clone();
      move || {
        if signal.with_untracked(|state| state.hovered.is_some() || state.drag.is_some()) {
          signal.update(|state| {
            state.hovered = None;
            state.drag = None;
          });
        }
      }
    })
    .on_scroll({
      let (analysis, signal, chart_ref) = (analysis.clone(), signal.clone(), chart_ref.clone());
      move |event: ScrollEvent| {
        event.prevent_default();
        let width = chart_ref.width().max(1.0);
        let at = f64::from((event.x - chart_ref.x()) / width);
        signal.update(|state| {
          if event.delta_y != 0.0 {
            state.zoom(&analysis, (-f64::from(event.delta_y) * ZOOM_PER_PX).exp(), at);
          }
          if event.delta_x != 0.0 {
            let view = state.view_of(&analysis);
            state.pan(
              &analysis,
              view,
              -f64::from(event.delta_x) * view.duration_ms() / f64::from(width),
            );
          }
          state.hovered = None;
        });
      }
    });

  Column::new()
    .width(FILL)
    .flex(1.0)
    .child(toolbar(analysis, state, view, signal))
    // The wheel is the chart's, so the chart is in nothing that scrolls: rows there is no room for are reached
    // by dragging the chart up, as its time is by dragging it along.
    .child(
      Row::new()
        .width(FILL)
        .flex(1.0)
        .padding_horizontal(PADDING)
        .clip()
        .ref_element(room_ref)
        .child(chart),
    )
    .child(selection(analysis, state))
    .into()
}

/// `event`'s position in the chart.
fn local(chart: &ElementRef, event: &MouseEvent) -> (f32, f32) {
  (event.x - chart.x(), event.y - chart.y())
}

pub(super) fn track_label(track: &TrackRows) -> String {
  let hidden = track.rows.len().saturating_sub(MAX_TRACK_ROWS);
  let label = match track.track {
    SpanTrack::Frames => "UI · toolkit phases".to_owned(),
    SpanTrack::Application(lane) => format!("{} · application scopes", lane_name(lane)),
  };
  if hidden > 0 {
    format!("{label} · {hidden} deeper rows not shown")
  } else {
    label
  }
}

pub(super) fn lane_name(lane: ApplicationLane) -> &'static str {
  match lane {
    ApplicationLane::Ui => "UI lane",
    ApplicationLane::Worker => "worker lane",
  }
}

/// The color a span of the chart is told apart by: its phase's, and a component's that of the phase it is in.
pub(crate) fn color_of(analysis: &Analysis, span: &ForestSpan) -> PaletteColor {
  if matches!(span.track, SpanTrack::Application(_)) {
    return PaletteColor::Chart5;
  }
  let mut at = Some(span);
  while let Some(span) = at {
    if let Some(color) = phase_color(span.name) {
      return color;
    }
    at = span.parent.map(|parent| analysis.span(parent));
  }
  PaletteColor::Chart1
}

fn block_element(analysis: &Analysis, state: &AnalysisState, block: Block, y: f32) -> Element {
  let element = Row::new()
    .align_items(Alignment::Center)
    .absolute(block.x, y, block.width.max(1.0), BLOCK_HEIGHT)
    .rounded(2.0)
    .pointer_events_none();
  // Spans too narrow to be told apart, drawn as one.
  let Some(index) = block.span else {
    return element.background(PaletteColor::Muted).into();
  };
  let span = analysis.span(index);
  let selected = state.selected_span == Some(index);
  let hovered = state.hovered.is_some_and(|hover| hover.span == index);
  let (fill, text, edge) = if selected {
    (
      PaletteColor::Primary,
      PaletteColor::PrimaryForeground,
      PaletteColor::Foreground,
    )
  } else {
    (
      if hovered {
        PaletteColor::SecondaryHover
      } else {
        PaletteColor::Secondary
      },
      PaletteColor::Foreground,
      color_of(analysis, span),
    )
  };
  let mut element = element.background(fill).border_top(Border::inside(2.0, edge));
  if block.width >= LABEL_MIN_PX {
    element = element
      .padding_top(2.0)
      .padding_horizontal(5.0)
      .clip()
      .child(mono(span.name, 10.0, FontWeight::Normal).color(text));
  }
  element.into()
}

/// What the pointer says of the span it is on: under and right of it, or above it where the chart ends below.
fn tooltip(analysis: &Analysis, hover: Hover, width: f32, height: f32) -> Element {
  let span = analysis.span(hover.span);
  let x = (hover.x + 12.0).min(width - TOOLTIP_WIDTH).max(0.0);
  let y = if hover.y + 14.0 + TOOLTIP_HEIGHT <= height {
    hover.y + 14.0
  } else {
    (hover.y - 8.0 - TOOLTIP_HEIGHT).max(0.0)
  };
  let said = format!(
    "{}\ntotal {} · self {}\n{}",
    span.name,
    format_ms(span.duration_ns() as f64 / 1e6),
    format_ms(span.self_ns as f64 / 1e6),
    span_origin(analysis, span)
  );
  Row::new()
    .absolute_position(x, y)
    .pointer_events_none()
    .id(TOOLTIP_ID)
    .child(Tooltip::new(&said).style(TooltipPart::Label, |s| {
      s.typography(TypographyStyle::Mono).font_size(11.0).line_height(1.5)
    }))
    .into()
}

/// Where a span comes from: its pass, or its application lane, and its start.
fn span_origin(analysis: &Analysis, span: &ForestSpan) -> String {
  let at = format_span(span.start_ns as f64 / 1e6 - analysis.origin_ms);
  match (span.track, span.sample) {
    (SpanTrack::Frames, Some(sequence)) => format!("pass #{sequence} · +{at}"),
    (SpanTrack::Application(lane), _) => format!("{} · +{at}", lane_name(lane)),
    _ => format!("+{at}"),
  }
}

/// What kind of thing a span is: the pass, a phase of the toolkit's, a component, or a scope of the app's.
fn span_kind(span: &ForestSpan) -> &'static str {
  match span.track {
    SpanTrack::Application(_) => "scope",
    SpanTrack::Frames if span.name == PASS_SPAN_NAME => "pass",
    SpanTrack::Frames if phase_color(span.name).is_some() => "phase",
    SpanTrack::Frames => "component",
  }
}

fn format_offset_from(origin_ms: f64, at: f64) -> String {
  format!("+{}", format_span(at - origin_ms))
}

/// The time the chart shows, how it is moved, and what brings it to the selected span or back to all of it.
fn toolbar(analysis: &Arc<Analysis>, state: &AnalysisState, view: TimeRange, signal: Signal<AnalysisState>) -> Row {
  let quiet = |label: &str| Button::new(label).variant(ButtonVariant::Ghost).size(ButtonSize::Xs);
  let (zoomed, reset) = (signal.clone(), signal);
  let selected = state.selected_span;
  let zoom_to = analysis.clone();
  Row::new()
    .width(FILL)
    .height(28.0)
    .padding_left(PADDING)
    .padding_right(12.0)
    .spacing(12.0)
    .align_items(Alignment::Center)
    .child(gives_way(
      mono(
        &format!(
          "{} – {} ({})",
          format_offset_from(analysis.origin_ms, view.start_ms),
          format_offset_from(analysis.origin_ms, view.end_ms),
          format_ms(view.duration_ms())
        ),
        12.0,
        FontWeight::Normal,
      )
      .id(VIEW_ID),
      1.0,
    ))
    .child(gives_way(
      sans(
        "Wheel to zoom · drag to pan · click a span to select it",
        12.0,
        FontWeight::Normal,
      )
      .color(PaletteColor::MutedForeground),
      100.0,
    ))
    .child(Spacer::new().flex(1.0))
    // The buttons keep their width: what is said before them gives way.
    .child(
      Row::new().flex_shrink(0.0).child(
        quiet("Zoom to selection")
          .disabled(selected.is_none())
          .on_click(move |_| {
            if let Some(index) = selected {
              zoomed.update(|state| state.zoom_to(&zoom_to, index));
            }
          })
          .id("devtools-profiler-zoom-to-selection"),
      ),
    )
    .child(
      Row::new().flex_shrink(0.0).child(
        quiet("Reset zoom")
          .disabled(state.view.is_none())
          .on_click(move |_| reset.update(|state| state.view = None))
          .id("devtools-profiler-reset-zoom"),
      ),
    )
}

/// The selected span in full, under the chart.
fn selection(analysis: &Analysis, state: &AnalysisState) -> Row {
  let footer = Row::new()
    .width(FILL)
    .height(36.0)
    .padding_horizontal(PADDING)
    .spacing(28.0)
    .align_items(Alignment::Center)
    .border_top(line())
    .id(SELECTION_ID);
  let Some(index) = state.selected_span else {
    return footer.child(
      sans(
        "Hover a span for its times; click one to select it.",
        12.0,
        FontWeight::Normal,
      )
      .color(PaletteColor::MutedForeground),
    );
  };
  let span = analysis.span(index);
  let stat = |label: &str, value: &str| {
    Row::new()
      .spacing(8.0)
      .align_items(Alignment::Center)
      .child(sans(label, 12.0, FontWeight::Normal).color(PaletteColor::MutedForeground))
      .child(mono(value, 12.0, FontWeight::Medium))
  };
  let parent = span.parent.map_or("–", |parent| analysis.span(parent).name);
  footer
    .child(stat("Selected span", span.name))
    .child(stat("Total", &format_ms(span.duration_ns() as f64 / 1e6)))
    .child(stat("Self", &format_ms(span.self_ns as f64 / 1e6)))
    .child(stat("Parent", parent))
    .child(stat(
      "Origin",
      &format!("{} · {}", span_kind(span), span_origin(analysis, span)),
    ))
}
