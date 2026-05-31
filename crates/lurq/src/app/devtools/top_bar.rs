use super::{
  DevToolsBoolCallback, DevToolsDebugOverlayCallback, DevToolsTab, debug_overlay_path_for_selection, profiler,
  style::{BORDER, FILL, MUTED, PRIMARY, SELECTED, SURFACE, SURFACE_2, TEXT, badge, icon, text},
};
use crate::{
  components::{Column, Rect, Row, Spacer},
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  node::{CursorIcon, Element, color::Color},
};

pub(crate) fn top_bar(
  _node_count: usize,
  overlay_enabled: bool,
  overlay_enabled_signal: Signal<bool>,
  pick_enabled: bool,
  pick_enabled_signal: Signal<bool>,
  selected_path: Vec<usize>,
  has_selection: bool,
  on_debug_overlay_path: Option<DevToolsDebugOverlayCallback>,
  on_overlay_enabled: Option<DevToolsBoolCallback>,
  on_pick_inspected: Option<DevToolsBoolCallback>,
  active_tab: DevToolsTab,
  active_tab_signal: Signal<DevToolsTab>,
  profiler_recording: bool,
  profiler_recording_signal: Signal<bool>,
  profiler_commits_signal: Signal<Vec<profiler::ProfilerCommitSnapshot>>,
  profiler_selected_commit_signal: Signal<usize>,
  profiler_last_recorded_signature_signal: Signal<u64>,
) -> Element {
  Row::new()
    .align_items(Alignment::Center)
    .child(
      Row::new()
        .align_items(Alignment::Center)
        .spacing(8.0)
        .child(icon("bug", 16.0, PRIMARY))
        .child(text("lurq DevTools", 14.0, FontWeight::Bold, TEXT))
        .child(badge("v0.1.0", MUTED, SURFACE_2))
        .width(200.0),
    )
    .child(tab(
      "layers",
      "Components",
      active_tab == DevToolsTab::Components,
      DevToolsTab::Components,
      active_tab_signal.clone(),
    ))
    .child(tab(
      "activity",
      "Profiler",
      active_tab == DevToolsTab::Profiler,
      DevToolsTab::Profiler,
      active_tab_signal.clone(),
    ))
    .child(tab(
      "zap",
      "Signals",
      active_tab == DevToolsTab::Signals,
      DevToolsTab::Signals,
      active_tab_signal,
    ))
    .child(Spacer::new().flex(1.0))
    .child(match active_tab {
      DevToolsTab::Profiler => profiler_actions(
        profiler_recording,
        profiler_recording_signal,
        profiler_commits_signal,
        profiler_selected_commit_signal,
        profiler_last_recorded_signature_signal,
      ),
      DevToolsTab::Signals => Row::new()
        .align_items(Alignment::Center)
        .child(search_box("Filter signals...", None))
        .into(),
      _ => Row::new()
        .align_items(Alignment::Center)
        .child(pick_toggle(pick_enabled, pick_enabled_signal, on_pick_inspected))
        .child(Spacer::new().width(8.0))
        .child(overlay_toggle(
          overlay_enabled,
          overlay_enabled_signal,
          selected_path,
          has_selection,
          on_debug_overlay_path,
          on_overlay_enabled,
        ))
        .child(Spacer::new().width(10.0))
        .child(search_box("Search components...", Some("Ctrl+F")))
        .child(icon_button("settings"))
        .into(),
    })
    .height(48.0)
    .width(FILL)
    .padding_horizontal(16.0)
    .padding_vertical(0.0)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .into()
}

fn profiler_actions(
  recording: bool,
  recording_signal: Signal<bool>,
  commits_signal: Signal<Vec<profiler::ProfilerCommitSnapshot>>,
  selected_commit_signal: Signal<usize>,
  last_recorded_signature_signal: Signal<u64>,
) -> Element {
  let color = if recording { "#ef4444" } else { PRIMARY };
  let label = if recording { "Recording..." } else { "Start recording" };
  Row::new()
    .align_items(Alignment::Center)
    .spacing(8.0)
    .child(
      Row::new()
        .align_items(Alignment::Center)
        .spacing(6.0)
        .child(icon("circle", 10.0, color))
        .child(text(label, 11.0, FontWeight::Medium, color))
        .padding_horizontal(12.0)
        .padding_vertical(6.0)
        .fill(if recording { "#ef444420" } else { SURFACE_2 })
        .border_inside(1.0, Color::from_hex(if recording { "#ef444460" } else { BORDER }))
        .rounded(4.0)
        .cursor(CursorIcon::Pointer)
        .on_click(move |_| recording_signal.set(!recording)),
    )
    .child(
      Row::new()
        .align_items(Alignment::Center)
        .spacing(4.0)
        .child(icon("trash-2", 12.0, MUTED))
        .child(text("Clear", 11.0, FontWeight::Normal, MUTED))
        .padding_horizontal(10.0)
        .padding_vertical(6.0)
        .fill(SURFACE_2)
        .rounded(4.0)
        .cursor(CursorIcon::Pointer)
        .on_click(move |_| {
          commits_signal.set(Vec::new());
          selected_commit_signal.set(0);
          last_recorded_signature_signal.set(profiler::EMPTY_FRAME_SIGNATURE);
        }),
    )
    .into()
}

fn overlay_toggle(
  overlay_enabled: bool,
  overlay_enabled_signal: Signal<bool>,
  selected_path: Vec<usize>,
  has_selection: bool,
  on_debug_overlay_path: Option<DevToolsDebugOverlayCallback>,
  on_overlay_enabled: Option<DevToolsBoolCallback>,
) -> Element {
  let color = if overlay_enabled { PRIMARY } else { MUTED };
  Row::new()
    .align_items(Alignment::Center)
    .spacing(6.0)
    .child(icon("box", 13.0, color))
    .child(text("Overlay", 12.0, FontWeight::Medium, color))
    .height(30.0)
    .padding_horizontal(10.0)
    .padding_vertical(0.0)
    .fill(if overlay_enabled { SELECTED } else { SURFACE_2 })
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(4.0)
    .cursor(CursorIcon::Pointer)
    .on_click(move |_| {
      let next = !overlay_enabled;
      overlay_enabled_signal.set(next);
      if let Some(on_overlay_enabled) = &on_overlay_enabled {
        on_overlay_enabled(next);
      }
      if let Some(on_debug_overlay_path) = &on_debug_overlay_path {
        on_debug_overlay_path(debug_overlay_path_for_selection(
          next,
          selected_path.clone(),
          has_selection,
        ));
      }
    })
    .into()
}

fn pick_toggle(
  pick_enabled: bool,
  pick_enabled_signal: Signal<bool>,
  on_pick_inspected: Option<DevToolsBoolCallback>,
) -> Element {
  let color = if pick_enabled { PRIMARY } else { MUTED };
  Row::new()
    .align_items(Alignment::Center)
    .spacing(6.0)
    .child(icon("search", 13.0, color))
    .child(text("Pick", 12.0, FontWeight::Medium, color))
    .height(30.0)
    .padding_horizontal(10.0)
    .padding_vertical(0.0)
    .fill(if pick_enabled { SELECTED } else { SURFACE_2 })
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(4.0)
    .cursor(CursorIcon::Pointer)
    .on_click(move |_| {
      let next = !pick_enabled;
      pick_enabled_signal.set(next);
      if let Some(on_pick_inspected) = &on_pick_inspected {
        on_pick_inspected(next);
      }
    })
    .into()
}

fn tab(
  icon_name: &str,
  label: &str,
  active: bool,
  target_tab: DevToolsTab,
  active_tab_signal: Signal<DevToolsTab>,
) -> Element {
  let color = if active { TEXT } else { MUTED };
  Column::new()
    .align_items(Alignment::Center)
    .child(
      Row::new()
        .align_items(Alignment::Center)
        .spacing(6.0)
        .child(icon(icon_name, 14.0, color))
        .child(text(label, 13.0, FontWeight::Medium, color))
        .height(36.0),
    )
    .child(Rect::new(64.0, 2.0).fill(if active { PRIMARY } else { "#00000000" }))
    .padding_horizontal(16.0)
    .padding_vertical(0.0)
    .cursor(CursorIcon::Pointer)
    .on_click(move |_| active_tab_signal.set(target_tab))
    .into()
}

fn search_box(placeholder: &str, shortcut: Option<&str>) -> Element {
  let mut row = Row::new()
    .align_items(Alignment::Center)
    .spacing(8.0)
    .child(icon("search", 13.0, MUTED))
    .child(text(placeholder, 12.0, FontWeight::Normal, MUTED));
  if let Some(shortcut) = shortcut {
    row = row.child(text(shortcut, 10.0, FontWeight::Normal, MUTED));
  }
  row
    .height(30.0)
    .padding_horizontal(10.0)
    .padding_vertical(0.0)
    .fill(SURFACE_2)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(4.0)
    .into()
}

fn icon_button(icon_name: &str) -> Element {
  Row::new()
    .align_items(Alignment::Center)
    .justify(crate::layout::layout_kind::Justify::Center)
    .child(icon(icon_name, 16.0, MUTED))
    .height(30.0)
    .padding_horizontal(8.0)
    .padding_vertical(0.0)
    .rounded(4.0)
    .cursor(CursorIcon::Pointer)
    .into()
}
