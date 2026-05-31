use super::{
  DevToolsBoolCallback, DevToolsDebugOverlayCallback, debug_overlay_path_for_selection,
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
    .child(tab("layers", "Components", true))
    .child(tab("activity", "Profiler", false))
    .child(tab("zap", "Signals", false))
    .child(Spacer::new().flex(1.0))
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
    .child(search_box())
    .child(icon_button("settings"))
    .height(48.0)
    .width(FILL)
    .padding_horizontal(16.0)
    .padding_vertical(0.0)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
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

fn tab(icon_name: &str, label: &str, active: bool) -> Element {
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
    .into()
}

fn search_box() -> Element {
  Row::new()
    .align_items(Alignment::Center)
    .spacing(8.0)
    .child(icon("search", 13.0, MUTED))
    .child(text("Search components...", 12.0, FontWeight::Normal, MUTED))
    .child(text("Ctrl+F", 10.0, FontWeight::Normal, MUTED))
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
