use std::{
  collections::{HashMap, hash_map::DefaultHasher},
  hash::{Hash, Hasher},
  time::{SystemTime, UNIX_EPOCH},
};

use super::{
  DevToolsSnapshot, FrameProfileSnapshot,
  style::{BORDER, FILL, MUTED, PRIMARY, SIGNAL_GREEN, SURFACE, SURFACE_2, TEXT, YELLOW, icon, text},
};
use crate::{
  components::{Column, Rect, Row, Spacer, Text},
  core::Signal,
  layout::{
    Alignment,
    layout_kind::Justify,
    text_style::{FontWeight, TextStyle},
  },
  node::{CursorIcon, Element, border::Border, color::Color, dimension::Dimension},
};

const RED: &str = "#ef4444";
const MAX_COMMITS: usize = 96;
pub(crate) const EMPTY_FRAME_SIGNATURE: u64 = u64::MAX;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProfilerCommitSnapshot {
  pub index: usize,
  pub frame: FrameProfileSnapshot,
  pub captured_at: String,
  signals: Vec<ProfilerSignalSnapshot>,
  triggers: Vec<ProfilerTriggerSnapshot>,
}

impl crate::app::component::DevtoolsInspectable for ProfilerCommitSnapshot {
  fn write_info(&self, buffer: &mut Vec<crate::app::component::ComponentInfo>) {
    buffer.push(crate::app::component::ComponentInfo::with_value(
      "commit",
      std::any::type_name::<usize>(),
      format!("#{}", self.index),
    ));
    buffer.push(crate::app::component::ComponentInfo::with_value(
      "captured_at",
      std::any::type_name::<String>(),
      self.captured_at.clone(),
    ));
  }
}

#[derive(Clone, Debug, PartialEq)]
struct ProfilerSignalSnapshot {
  id: usize,
  type_name: String,
  value: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct ProfilerTriggerSnapshot {
  signal_id: usize,
  label: String,
  details: String,
  from_value: String,
  to_value: String,
  kind: ProfilerTriggerKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProfilerTriggerKind {
  Signal,
}

pub(crate) fn record_frame(
  snapshot: &DevToolsSnapshot,
  commits_signal: &Signal<Vec<ProfilerCommitSnapshot>>,
  selected_commit_signal: &Signal<usize>,
  last_recorded_signature_signal: &Signal<u64>,
) {
  let frame = snapshot.frame;
  if frame_is_empty(frame) {
    return;
  }

  let signature = frame_signature(frame);
  if signature == last_recorded_signature_signal.get_untracked() {
    return;
  }

  last_recorded_signature_signal.set(signature);
  let previous_signals = commits_signal.with_untracked(|commits| commits.last().map(|commit| commit.signals.clone()));
  let signals = collect_signals(snapshot);
  let triggers = signal_triggers(previous_signals.as_deref(), &signals);
  let next_index = commits_signal.with_untracked(|commits| commits.last().map(|commit| commit.index + 1).unwrap_or(1));
  commits_signal.update(|commits| {
    commits.push(ProfilerCommitSnapshot {
      index: next_index,
      frame,
      captured_at: current_timestamp(),
      signals,
      triggers,
    });
    if commits.len() > MAX_COMMITS {
      let overflow = commits.len() - MAX_COMMITS;
      commits.drain(0..overflow);
    }
  });
  selected_commit_signal.set(next_index);
}

pub(crate) fn profiler_view(
  snapshot: &DevToolsSnapshot,
  commits: &[ProfilerCommitSnapshot],
  selected_commit: usize,
  selected_commit_signal: Signal<usize>,
  recording: bool,
) -> Element {
  let selected = selected_commit_snapshot(commits, selected_commit);
  let frame = selected.map(|commit| commit.frame).unwrap_or(snapshot.frame);
  let commit_index = selected.map(|commit| commit.index).unwrap_or(0);
  let triggers = selected.map(|commit| commit.triggers.as_slice()).unwrap_or(&[]);
  let captured_at = selected
    .map(|commit| commit.captured_at.as_str())
    .unwrap_or(if recording { "recording" } else { "not recorded" });

  Row::new()
    .child(
      Column::new()
        .child(commit_timeline(commits, selected_commit, selected_commit_signal))
        .child(ranked_chart(frame, commit_index))
        .height(FILL)
        .width(FILL)
        .flex(1.0),
    )
    .child(profiler_sidebar(snapshot, frame, commit_index, captured_at, triggers))
    .height(FILL)
    .width(FILL)
    .flex(1.0)
    .into()
}

fn commit_timeline(
  commits: &[ProfilerCommitSnapshot],
  selected_commit: usize,
  selected_commit_signal: Signal<usize>,
) -> Element {
  let sample_count = commits.len();
  let max_sample = commits
    .iter()
    .map(|commit| commit.frame.total_ms)
    .fold(1.0_f32, f32::max);
  let mut bars = Row::new()
    .align_items(Alignment::End)
    .spacing(2.0)
    .height(60.0)
    .width(FILL)
    .padding_custom(padding(4.0, 16.0, 10.0, 16.0));

  for commit in commits {
    let sample = commit.frame.total_ms;
    let selected = commit.index == selected_commit;
    let commit_index = commit.index;
    bars = bars.child(
      Row::new()
        .align_items(Alignment::End)
        .child(
          Rect::new(8.0, (sample / max_sample * 40.0).max(4.0))
            .fill(if selected { PRIMARY } else { duration_color(sample) })
            .rounded(1.0)
            .opacity(if selected { 1.0 } else { 0.6 }),
        )
        .height(FILL)
        .cursor(CursorIcon::Pointer)
        .on_click({
          let selected_commit_signal = selected_commit_signal.clone();
          move |_| selected_commit_signal.set(commit_index)
        }),
    );
  }

  Column::new()
    .child(
      Row::new()
        .align_items(Alignment::Center)
        .spacing(6.0)
        .child(text("COMMITS", 11.0, FontWeight::Bold, MUTED))
        .child(Spacer::new().flex(1.0))
        .child(mono_text(
          &format!("{sample_count} commits recorded"),
          10.0,
          FontWeight::Normal,
          MUTED,
        ))
        .padding_custom(padding(8.0, 16.0, 6.0, 16.0))
        .width(FILL),
    )
    .child(bars)
    .width(FILL)
    .fill(SURFACE)
    .border_bottom(divider())
    .into()
}

fn ranked_chart(frame: FrameProfileSnapshot, commit_index: usize) -> Element {
  let frame_color = duration_color(frame.total_ms);
  let scale = if frame.total_ms > 0.0 {
    frame.total_ms / 45.2
  } else {
    1.0
  };
  let rows = [
    ("DemoApp", 45.2),
    ("ReactivityDemo", 38.1),
    ("Counter", 22.4),
    ("TabBar", 8.2),
    ("MemoDemo", 5.1),
    ("Text", 2.3),
    ("Row", 1.8),
    ("Column", 1.2),
  ];
  let rows = rows.map(|(label, ms)| (label, ms * scale));
  let max_ms = rows.iter().map(|(_, ms)| *ms).fold(1.0_f32, f32::max);
  let mut body = Column::new()
    .spacing(4.0)
    .height(FILL)
    .width(FILL)
    .padding_custom(padding(12.0, 16.0, 12.0, 16.0));

  for (label, ms) in rows {
    body = body.child(profile_bar(label, ms, max_ms));
  }

  Column::new()
    .child(
      Row::new()
        .align_items(Alignment::Center)
        .spacing(8.0)
        .child(text("RANKED CHART", 11.0, FontWeight::Bold, MUTED))
        .child(Spacer::new().flex(1.0))
        .child(mono_text(
          &format_commit_label(commit_index),
          10.0,
          FontWeight::Normal,
          PRIMARY,
        ))
        .child(mono_text(
          &format!("{:.2}ms", frame.total_ms),
          10.0,
          FontWeight::Normal,
          frame_color,
        ))
        .padding_custom(padding(8.0, 16.0, 8.0, 16.0))
        .width(FILL)
        .border_bottom(divider()),
    )
    .child(body)
    .height(FILL)
    .width(FILL)
    .into()
}

fn profile_bar(label: &str, ms: f32, max_ms: f32) -> Element {
  let color = duration_color(ms);
  let width = (ms / max_ms * 100.0).clamp(2.0, 100.0);

  Row::new()
    .align_items(Alignment::Center)
    .spacing(8.0)
    .child(mono_text(label, 11.0, FontWeight::Medium, TEXT).width(140.0))
    .child(
      Row::new()
        .child(
          Row::new()
            .align_items(Alignment::Center)
            .justify(Justify::End)
            .child(mono_text(&format!("{:.2}ms", ms), 10.0, FontWeight::Medium, color))
            .padding_horizontal(8.0)
            .height(FILL)
            .width(Dimension::Pct(width))
            .fill(Color::from_hex(&with_alpha(color, "40")))
            .rounded(2.0),
        )
        .height(20.0)
        .width(FILL)
        .fill(SURFACE_2)
        .rounded(2.0),
    )
    .height(28.0)
    .width(FILL)
    .into()
}

fn profiler_sidebar(
  snapshot: &DevToolsSnapshot,
  frame: FrameProfileSnapshot,
  commit_index: usize,
  captured_at: &str,
  triggers: &[ProfilerTriggerSnapshot],
) -> Element {
  Column::new()
    .child(
      Row::new()
        .align_items(Alignment::Center)
        .child(text("COMMIT DETAILS", 11.0, FontWeight::Bold, MUTED))
        .padding_custom(padding(10.0, 16.0, 10.0, 16.0))
        .width(FILL)
        .border_bottom(divider()),
    )
    .child(commit_details(snapshot, frame, commit_index, captured_at))
    .child(perf_stats_section(frame))
    .child(trigger_section(triggers))
    .width(320.0)
    .height(FILL)
    .fill(SURFACE)
    .border_left(divider())
    .into()
}

fn commit_details(
  snapshot: &DevToolsSnapshot,
  frame: FrameProfileSnapshot,
  commit_index: usize,
  captured_at: &str,
) -> Element {
  let signal_count = snapshot.root.as_ref().map(count_signals).unwrap_or_default();
  let duration_color = duration_color(frame.total_ms);
  let details = [
    ("Commit", format_commit_value(commit_index), PRIMARY),
    ("Duration", format!("{:.2}ms", frame.total_ms), duration_color),
    ("Components rendered", snapshot.node_count().to_string(), TEXT),
    ("Signals tracked", signal_count.to_string(), SIGNAL_GREEN),
    (
      "Layout recalc",
      if frame.layout_recalculated { "yes" } else { "no" }.to_owned(),
      YELLOW,
    ),
    ("Timestamp", captured_at.to_owned(), MUTED),
  ];
  let mut column = Column::new()
    .spacing(12.0)
    .padding_custom(padding(12.0, 16.0, 12.0, 16.0))
    .width(FILL)
    .border_bottom(divider());

  for (label, value, color) in details {
    column = column.child(detail_row(label, &value, color));
  }

  column.into()
}

fn detail_row(label: &str, value: &str, color: &str) -> Element {
  Row::new()
    .align_items(Alignment::Center)
    .child(text(label, 11.0, FontWeight::Normal, MUTED))
    .child(Spacer::new().flex(1.0))
    .child(mono_text(value, 11.0, FontWeight::Medium, color))
    .width(FILL)
    .into()
}

fn perf_stats_section(frame: FrameProfileSnapshot) -> Element {
  let rows = vec![
    ("FPS", frame.fps.to_string(), TEXT),
    ("total", format_ms(frame.total_ms), duration_color(frame.total_ms)),
    ("layout", format_ms(frame.layout_ms), duration_color(frame.layout_ms)),
    ("resolve", format_ms(frame.quad_ms), duration_color(frame.quad_ms)),
    ("glyph", format_ms(frame.glyph_ms), duration_color(frame.glyph_ms)),
    ("acquire", format_ms(frame.acquire_ms), duration_color(frame.acquire_ms)),
    ("upload", format_ms(frame.upload_ms), duration_color(frame.upload_ms)),
    ("encode", format_ms(frame.encode_ms), duration_color(frame.encode_ms)),
    ("submit", format_ms(frame.submit_ms), duration_color(frame.submit_ms)),
    ("present", format_ms(frame.present_ms), duration_color(frame.present_ms)),
    ("quads", frame.quad_count.to_string(), TEXT),
    ("glyphs", frame.glyph_count.to_string(), TEXT),
  ];
  let mut section = Column::new()
    .spacing(8.0)
    .child(text("PERF STATS", 11.0, FontWeight::Bold, MUTED))
    .padding_custom(padding(12.0, 16.0, 12.0, 16.0))
    .width(FILL)
    .border_bottom(divider());

  for (label, value, color) in rows {
    section = section.child(detail_row(label, &value, color));
  }

  section.into()
}

fn trigger_section(triggers: &[ProfilerTriggerSnapshot]) -> Element {
  let mut section = Column::new()
    .spacing(8.0)
    .child(text("WHAT CAUSED THIS RENDER?", 11.0, FontWeight::Bold, MUTED))
    .padding_custom(padding(12.0, 16.0, 12.0, 16.0))
    .width(FILL);

  if triggers.is_empty() {
    section = section.child(
      mono_text(
        "No signal changes captured for this commit",
        10.0,
        FontWeight::Normal,
        MUTED,
      )
      .padding_horizontal(8.0)
      .padding_vertical(6.0),
    );
  } else {
    for trigger in triggers.iter().take(8) {
      section = section.child(trigger_row(trigger));
    }
  }

  section.into()
}

fn trigger_row(trigger: &ProfilerTriggerSnapshot) -> Element {
  let (icon_name, color) = match trigger.kind {
    ProfilerTriggerKind::Signal => ("zap", SIGNAL_GREEN),
  };

  Row::new()
    .align_items(Alignment::Center)
    .spacing(6.0)
    .child(icon(icon_name, 12.0, color))
    .child(
      Column::new()
        .spacing(1.0)
        .child(mono_text(&trigger.label, 11.0, FontWeight::Bold, color))
        .child(mono_text(&trigger.details, 9.0, FontWeight::Normal, MUTED).width(FILL))
        .width(FILL),
    )
    .padding_horizontal(8.0)
    .padding_vertical(6.0)
    .width(FILL)
    .fill(SURFACE_2)
    .rounded(4.0)
    .into()
}

fn duration_color(ms: f32) -> &'static str {
  if ms >= 30.0 {
    RED
  } else if ms >= 16.0 {
    YELLOW
  } else {
    SIGNAL_GREEN
  }
}

fn selected_commit_snapshot(
  commits: &[ProfilerCommitSnapshot],
  selected_commit: usize,
) -> Option<&ProfilerCommitSnapshot> {
  commits
    .iter()
    .find(|commit| commit.index == selected_commit)
    .or_else(|| commits.last())
}

fn frame_is_empty(frame: FrameProfileSnapshot) -> bool {
  frame.total_ms <= f32::EPSILON
    && frame.layout_ms <= f32::EPSILON
    && frame.quad_ms <= f32::EPSILON
    && frame.glyph_ms <= f32::EPSILON
    && frame.render_ms <= f32::EPSILON
    && frame.acquire_ms <= f32::EPSILON
    && frame.upload_ms <= f32::EPSILON
    && frame.encode_ms <= f32::EPSILON
    && frame.submit_ms <= f32::EPSILON
    && frame.present_ms <= f32::EPSILON
    && frame.quad_count == 0
    && frame.rect_count == 0
    && frame.glyph_count == 0
}

fn frame_signature(frame: FrameProfileSnapshot) -> u64 {
  let mut hasher = DefaultHasher::new();
  frame.fps.hash(&mut hasher);
  frame.total_ms.to_bits().hash(&mut hasher);
  frame.layout_ms.to_bits().hash(&mut hasher);
  frame.layout_recalculated.hash(&mut hasher);
  frame.quad_ms.to_bits().hash(&mut hasher);
  frame.glyph_ms.to_bits().hash(&mut hasher);
  frame.render_ms.to_bits().hash(&mut hasher);
  frame.acquire_ms.to_bits().hash(&mut hasher);
  frame.upload_ms.to_bits().hash(&mut hasher);
  frame.encode_ms.to_bits().hash(&mut hasher);
  frame.submit_ms.to_bits().hash(&mut hasher);
  frame.present_ms.to_bits().hash(&mut hasher);
  frame.quad_count.hash(&mut hasher);
  frame.rect_count.hash(&mut hasher);
  frame.glyph_count.hash(&mut hasher);
  hasher.finish()
}

fn collect_signals(snapshot: &DevToolsSnapshot) -> Vec<ProfilerSignalSnapshot> {
  let mut signals = Vec::new();
  if let Some(root) = &snapshot.root {
    collect_node_signals(root, &mut signals);
  }
  signals.sort_by_key(|signal| signal.id);
  signals
}

fn collect_node_signals(node: &super::DevToolsNode, out: &mut Vec<ProfilerSignalSnapshot>) {
  for signal in &node.signals {
    out.push(ProfilerSignalSnapshot {
      id: signal.id,
      type_name: short_type_name(signal.type_name.as_ref()),
      value: signal.formatted_value(),
    });
  }
  for child in &node.children {
    collect_node_signals(child, out);
  }
}

fn signal_triggers(
  previous: Option<&[ProfilerSignalSnapshot]>,
  current: &[ProfilerSignalSnapshot],
) -> Vec<ProfilerTriggerSnapshot> {
  let Some(previous) = previous else {
    return Vec::new();
  };
  let previous = previous
    .iter()
    .map(|signal| (signal.id, signal))
    .collect::<HashMap<_, _>>();

  current
    .iter()
    .filter_map(|signal| {
      let previous = previous.get(&signal.id)?;
      if previous.value == signal.value {
        return None;
      }
      let from_value = format_signal_value(previous.value.as_deref());
      let to_value = format_signal_value(signal.value.as_deref());
      Some(ProfilerTriggerSnapshot {
        signal_id: signal.id,
        label: format!("#{}", signal.id),
        details: format!("Signal<{}> changed: {} -> {}", signal.type_name, from_value, to_value),
        from_value,
        to_value,
        kind: ProfilerTriggerKind::Signal,
      })
    })
    .collect()
}

fn format_signal_value(value: Option<&str>) -> String {
  value.unwrap_or("<unknown>").to_owned()
}

fn format_ms(value: f32) -> String {
  format!("{value:.2}ms")
}

fn short_type_name(type_name: &str) -> String {
  type_name.rsplit("::").next().unwrap_or(type_name).to_owned()
}

fn current_timestamp() -> String {
  let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
  format_unix_timestamp(duration.as_secs(), duration.subsec_millis())
}

fn format_unix_timestamp(secs: u64, millis: u32) -> String {
  let days = (secs / 86_400) as i64;
  let seconds_of_day = secs % 86_400;
  let (year, month, day) = civil_from_days(days);
  let hour = seconds_of_day / 3_600;
  let minute = seconds_of_day % 3_600 / 60;
  let second = seconds_of_day % 60;

  format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}.{millis:03} UTC")
}

fn civil_from_days(days_since_unix_epoch: i64) -> (i32, u32, u32) {
  let z = days_since_unix_epoch + 719_468;
  let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
  let day_of_era = z - era * 146_097;
  let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
  let year = year_of_era + era * 400;
  let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
  let month_prime = (5 * day_of_year + 2) / 153;
  let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
  let month = month_prime + if month_prime < 10 { 3 } else { -9 };
  let year = year + if month <= 2 { 1 } else { 0 };

  (year as i32, month as u32, day as u32)
}

fn format_commit_label(commit_index: usize) -> String {
  if commit_index == 0 {
    "Commit -".to_owned()
  } else {
    format!("Commit #{commit_index}")
  }
}

fn format_commit_value(commit_index: usize) -> String {
  if commit_index == 0 {
    "-".to_owned()
  } else {
    format!("#{commit_index}")
  }
}

fn count_signals(node: &super::DevToolsNode) -> usize {
  node.signals.len() + node.children.iter().map(count_signals).sum::<usize>()
}

fn with_alpha(color: &str, alpha: &str) -> String {
  format!("{}{}", color.trim_end_matches(|ch| ch == '0' && color.len() > 7), alpha)
}

fn padding(top: f32, right: f32, bottom: f32, left: f32) -> crate::node::padding::Padding {
  crate::node::padding::Padding {
    top: Dimension::Px(top),
    right: Dimension::Px(right),
    bottom: Dimension::Px(bottom),
    left: Dimension::Px(left),
  }
}

fn divider() -> Border {
  Border::inside(1.0, Color::from_hex(BORDER))
}

fn mono_text(content: &str, size: f32, weight: FontWeight, color: &str) -> Text {
  Text::styled(
    content,
    TextStyle {
      font_family: "monospace".into(),
      font_size: size,
      weight,
      color: Color::from_hex(color),
      ..Default::default()
    },
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    app::{
      ctx::{ComponentSignalDebug, Ctx},
      devtools::{DevToolsNode, snapshot::DevToolsNodeKind},
    },
    core::NodeId,
  };

  #[test]
  fn record_frame_skips_empty_frames() {
    let commits = Signal::new(Vec::new());
    let selected = Signal::new(0);
    let last_signature = Signal::new(EMPTY_FRAME_SIGNATURE);

    record_frame(
      &snapshot_with_frame(FrameProfileSnapshot::default()),
      &commits,
      &selected,
      &last_signature,
    );

    assert!(commits.get_untracked().is_empty());
    assert_eq!(selected.get_untracked(), 0);
    assert_eq!(last_signature.get_untracked(), EMPTY_FRAME_SIGNATURE);
  }

  #[test]
  fn record_frame_deduplicates_by_frame_signature() {
    let commits = Signal::new(Vec::new());
    let selected = Signal::new(0);
    let last_signature = Signal::new(EMPTY_FRAME_SIGNATURE);
    let frame = FrameProfileSnapshot {
      total_ms: 12.0,
      layout_ms: 8.0,
      quad_count: 4,
      glyph_count: 20,
      ..Default::default()
    };

    record_frame(&snapshot_with_frame(frame), &commits, &selected, &last_signature);
    record_frame(&snapshot_with_frame(frame), &commits, &selected, &last_signature);
    record_frame(
      &snapshot_with_frame(FrameProfileSnapshot {
        total_ms: 16.0,
        ..frame
      }),
      &commits,
      &selected,
      &last_signature,
    );

    let commits = commits.get_untracked();
    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].index, 1);
    assert_eq!(commits[1].index, 2);
    assert_eq!(selected.get_untracked(), 2);
  }

  #[test]
  fn record_frame_attaches_signal_change_triggers() {
    let commits = Signal::new(Vec::new());
    let selected = Signal::new(0);
    let last_signature = Signal::new(EMPTY_FRAME_SIGNATURE);
    let mut ctx = Ctx::new_root();
    let signal = ctx.signal(4_i32);

    record_frame(
      &snapshot_with_signals(
        ctx.signals_debug(),
        FrameProfileSnapshot {
          total_ms: 12.0,
          ..Default::default()
        },
      ),
      &commits,
      &selected,
      &last_signature,
    );

    signal.set(5);

    record_frame(
      &snapshot_with_signals(
        ctx.signals_debug(),
        FrameProfileSnapshot {
          total_ms: 16.0,
          ..Default::default()
        },
      ),
      &commits,
      &selected,
      &last_signature,
    );

    let commits = commits.get_untracked();
    assert_eq!(commits.len(), 2);
    assert_eq!(commits[1].triggers.len(), 1);
    assert_eq!(commits[1].triggers[0].label, format!("#{}", signal.id()));
    assert_eq!(commits[1].triggers[0].details, "Signal<i32> changed: 4 -> 5");
  }

  #[test]
  fn timestamp_format_is_human_readable_utc() {
    assert_eq!(format_unix_timestamp(0, 0), "1970-01-01 00:00:00.000 UTC");
    assert_eq!(format_unix_timestamp(1_783_025_839, 738), "2026-07-02 20:57:19.738 UTC");
  }

  fn snapshot_with_frame(frame: FrameProfileSnapshot) -> DevToolsSnapshot {
    DevToolsSnapshot { root: None, frame }
  }

  fn snapshot_with_signals(signals: Vec<ComponentSignalDebug>, frame: FrameProfileSnapshot) -> DevToolsSnapshot {
    DevToolsSnapshot {
      root: Some(DevToolsNode {
        id: NodeId::UNASSIGNED,
        tag: "Root".to_owned(),
        kind: DevToolsNodeKind::Component,
        key: None,
        text: None,
        color: None,
        props: None,
        signals,
        memos: Vec::new(),
        contexts: Vec::new(),
        shape: Vec::new(),
        effects: Vec::new(),
        children: Vec::new(),
      }),
      frame,
    }
  }
}
