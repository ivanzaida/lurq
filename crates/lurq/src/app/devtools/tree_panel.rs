use super::{
  DevToolsDebugOverlayCallback, DevToolsPathCallback, debug_overlay_path_for_selection,
  snapshot::{DevToolsNode, DevToolsSnapshot},
  style::{
    BLUE, BORDER, FILL, GREEN, MUTED, ORANGE, PRIMARY, SELECTED, SURFACE, SURFACE_2, badge, empty_state, icon,
    section_header, short_tag, text,
  },
};
use crate::{
  components::{Column, Row, ScrollBoth, Spacer},
  core::{NodeId, Signal},
  layout::{
    Alignment,
    layout_kind::{FrameConstraints, ScrollState},
    text_style::FontWeight,
  },
  node::{CursorIcon, Element, border::Border, color::Color, dimension::Dimension},
};

const TREE_PANEL_WIDTH: f32 = 380.0;
const TREE_CONTENT_MIN_WIDTH: f32 = 640.0;
pub(crate) const TREE_ROW_HEIGHT: f32 = 26.0;
const TREE_OVERSCAN_ROWS: usize = 16;
const TREE_INITIAL_ROWS: usize = 80;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TreeRowWindow {
  start: usize,
  end: usize,
}

pub(crate) fn tree_panel(
  snapshot: &DevToolsSnapshot,
  selected_path: Vec<usize>,
  selected: Signal<Vec<usize>>,
  collapsed_nodes: Vec<NodeId>,
  collapsed: Signal<Vec<NodeId>>,
  scroll_state: ScrollState,
  overlay_enabled: bool,
  on_debug_overlay_path: Option<DevToolsDebugOverlayCallback>,
  on_selected_path: Option<DevToolsPathCallback>,
) -> Element {
  let mut rows = Vec::new();
  let mut row_count = 0;
  if let Some(root) = &snapshot.root {
    let row_window = tree_row_window(&scroll_state);
    collect_tree_rows(
      root,
      &mut Vec::new(),
      0,
      row_window,
      &mut row_count,
      &selected_path,
      selected,
      &collapsed_nodes,
      collapsed,
      overlay_enabled,
      on_debug_overlay_path,
      on_selected_path,
      &mut rows,
    );

    let top_spacer = row_window.start.min(row_count) as f32 * TREE_ROW_HEIGHT;
    let rendered_rows = rows.len();
    let bottom_rows = row_count.saturating_sub(row_window.start.min(row_count) + rendered_rows);
    let bottom_spacer = bottom_rows as f32 * TREE_ROW_HEIGHT;
    if top_spacer > 0.0 {
      rows.insert(0, Spacer::new().height(top_spacer).into());
    }
    if bottom_spacer > 0.0 {
      rows.push(Spacer::new().height(bottom_spacer).into());
    }
  } else {
    rows.push(empty_state("No mounted root"));
  }

  Column::new()
    .child(section_header("COMPONENT TREE", &format!("{row_count} components")))
    .child(
      ScrollBoth::new(Column::new().with_children(rows))
        .with_scroll_state(scroll_state)
        .height(FILL)
        .width(FILL)
        .flex(1.0),
    )
    .width(TREE_PANEL_WIDTH)
    .height(FILL)
    .background(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .into()
}

fn tree_row_window(scroll_state: &ScrollState) -> TreeRowWindow {
  let scroll_y = scroll_state.scroll_y().max(0.0);
  let viewport_height = scroll_state.viewport_height();
  let first_visible = (scroll_y / TREE_ROW_HEIGHT).floor().max(0.0) as usize;
  let start = first_visible.saturating_sub(TREE_OVERSCAN_ROWS);
  let visible_rows = if viewport_height > 0.0 {
    (viewport_height / TREE_ROW_HEIGHT).ceil().max(1.0) as usize
  } else {
    TREE_INITIAL_ROWS
  };
  let end = first_visible
    .saturating_add(visible_rows)
    .saturating_add(TREE_OVERSCAN_ROWS);

  TreeRowWindow { start, end }
}

fn collect_tree_rows(
  node: &DevToolsNode,
  path: &mut Vec<usize>,
  depth: usize,
  row_window: TreeRowWindow,
  row_count: &mut usize,
  selected_path: &[usize],
  selected: Signal<Vec<usize>>,
  collapsed_nodes: &[NodeId],
  collapsed: Signal<Vec<NodeId>>,
  overlay_enabled: bool,
  on_debug_overlay_path: Option<DevToolsDebugOverlayCallback>,
  on_selected_path: Option<DevToolsPathCallback>,
  rows: &mut Vec<Element>,
) {
  let is_collapsed = !node.children.is_empty() && collapsed_nodes.contains(&node.id);
  let row_index = *row_count;
  *row_count += 1;
  if row_index >= row_window.start && row_index < row_window.end {
    rows.push(tree_row(
      node,
      path.clone(),
      depth,
      path.as_slice() == selected_path,
      is_collapsed,
      selected.clone(),
      collapsed.clone(),
      overlay_enabled,
      on_debug_overlay_path.clone(),
      on_selected_path.clone(),
    ));
  }
  if is_collapsed {
    return;
  }

  for (index, child) in node.children.iter().enumerate() {
    path.push(index);
    collect_tree_rows(
      child,
      path,
      depth + 1,
      row_window,
      row_count,
      selected_path,
      selected.clone(),
      collapsed_nodes,
      collapsed.clone(),
      overlay_enabled,
      on_debug_overlay_path.clone(),
      on_selected_path.clone(),
      rows,
    );
    path.pop();
  }
}

fn tree_row(
  node: &DevToolsNode,
  path: Vec<usize>,
  depth: usize,
  selected: bool,
  collapsed: bool,
  selected_path: Signal<Vec<usize>>,
  collapsed_nodes: Signal<Vec<NodeId>>,
  overlay_enabled: bool,
  on_debug_overlay_path: Option<DevToolsDebugOverlayCallback>,
  on_selected_path: Option<DevToolsPathCallback>,
) -> Element {
  let indent = 8.0 + depth as f32 * 16.0;
  let child_count = node.children.len();
  let click_path = path;
  let collapse_id = node.id;
  let tag = short_tag(&node.tag);
  let key_preview = node.key.as_deref().map(format_attr_value);
  let text_preview = node.text.as_deref().filter(|_| tag == "Text").map(format_attr_value);
  let tag_color = if node.tag.starts_with("lurq::") || node.tag.contains("Demo") {
    BLUE
  } else {
    MUTED
  };
  let mut row = Row::new()
    .align_items(Alignment::Center)
    .spacing(6.0)
    .child(Spacer::new().width(indent))
    .child(if child_count > 0 {
      let icon_name = if collapsed { "chevron-right" } else { "chevron-down" };
      icon(icon_name, 12.0, MUTED)
        .width(12.0)
        .height(18.0)
        .cursor(CursorIcon::Pointer)
        .on_click(move |_| toggle_collapsed(&collapsed_nodes, collapse_id))
    } else {
      text("", 12.0, FontWeight::Normal, MUTED).width(12.0)
    })
    .child(text("<", 12.0, FontWeight::Normal, MUTED))
    .child(text(tag, 12.0, FontWeight::Bold, tag_color).nowrap());

  if let Some(preview) = key_preview {
    row = row
      .child(text(" key=", 12.0, FontWeight::Normal, MUTED))
      .child(text(&preview, 12.0, FontWeight::Normal, ORANGE).nowrap());
  }

  for (name, value) in &node.attrs {
    let preview = format_attr_value(value);
    row = row
      .child(text(&format!(" {name}="), 12.0, FontWeight::Normal, MUTED))
      .child(text(&preview, 12.0, FontWeight::Normal, ORANGE).nowrap());
  }

  if let Some(preview) = text_preview {
    row = row
      .child(text(" text=", 12.0, FontWeight::Normal, MUTED))
      .child(text(&preview, 12.0, FontWeight::Normal, ORANGE).nowrap());
  }

  row = row.child(text("/>", 12.0, FontWeight::Normal, MUTED));
  if child_count > 0 {
    row = row.child(badge(&child_count.to_string(), GREEN, SURFACE_2));
  }

  row = row
    .height(TREE_ROW_HEIGHT)
    .background(if selected { SELECTED } else { "#00000000" })
    .frame(FrameConstraints {
      min_width: Some(Dimension::Px(TREE_CONTENT_MIN_WIDTH)),
      ..Default::default()
    })
    .cursor(CursorIcon::Pointer)
    .on_click(move |_| {
      selected_path.set(click_path.clone());
      if let Some(on_selected_path) = &on_selected_path {
        on_selected_path(click_path.clone());
      }
      if let Some(on_debug_overlay_path) = &on_debug_overlay_path {
        on_debug_overlay_path(debug_overlay_path_for_selection(
          overlay_enabled,
          click_path.clone(),
          true,
        ));
      }
    });
  if selected {
    row = row.border_left(Border::inside(2.0, Color::from_hex(PRIMARY)));
  }

  row.into()
}

fn format_attr_value(content: &str) -> String {
  const MAX_CHARS: usize = 48;

  let mut escaped = content
    .replace('\\', "\\\\")
    .replace('"', "\\\"")
    .replace('\n', "\\n")
    .replace('\r', "\\r")
    .replace('\t', "\\t");

  if escaped.chars().count() > MAX_CHARS {
    escaped = escaped.chars().take(MAX_CHARS).collect::<String>();
    escaped.push_str("...");
  }

  format!("\"{escaped}\"")
}

fn toggle_collapsed(collapsed_nodes: &Signal<Vec<NodeId>>, id: NodeId) {
  collapsed_nodes.update(|nodes| {
    if let Some(index) = nodes.iter().position(|node_id| *node_id == id) {
      nodes.remove(index);
    } else {
      nodes.push(id);
    }
  });
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::IdGenerator;

  #[test]
  fn toggle_collapsed_adds_and_removes_node_id() {
    let id = IdGenerator::new().next();
    let collapsed = Signal::new(Vec::new());

    toggle_collapsed(&collapsed, id);
    assert_eq!(collapsed.get_untracked(), vec![id]);

    toggle_collapsed(&collapsed, id);
    assert!(collapsed.get_untracked().is_empty());
  }

  #[test]
  fn collect_tree_rows_skips_children_below_collapsed_node() {
    let ids = IdGenerator::new();
    let child_id = ids.next();
    let root = DevToolsNode {
      id: ids.next(),
      tag: "Root".to_owned(),
      kind: super::super::snapshot::DevToolsNodeKind::Component,
      key: None,
      attrs: Vec::new(),
      text: None,
      color: None,
      props: None,
      signals: Vec::new(),
      memos: Vec::new(),
      contexts: Vec::new(),
      shape: Vec::new(),
      effects: Vec::new(),
      layout_box: None,
      hovered: false,
      active: false,
      focused: false,
      children: vec![DevToolsNode {
        id: child_id,
        tag: "Child".to_owned(),
        kind: super::super::snapshot::DevToolsNodeKind::Element,
        key: None,
        attrs: Vec::new(),
        text: None,
        color: None,
        props: None,
        signals: Vec::new(),
        memos: Vec::new(),
        contexts: Vec::new(),
        shape: Vec::new(),
        effects: Vec::new(),
        layout_box: None,
        hovered: false,
        active: false,
        focused: false,
        children: vec![DevToolsNode {
          id: ids.next(),
          tag: "Grandchild".to_owned(),
          kind: super::super::snapshot::DevToolsNodeKind::Element,
          key: None,
          attrs: Vec::new(),
          text: None,
          color: None,
          props: None,
          signals: Vec::new(),
          memos: Vec::new(),
          contexts: Vec::new(),
          shape: Vec::new(),
          effects: Vec::new(),
          layout_box: None,
          hovered: false,
          active: false,
          focused: false,
          children: Vec::new(),
        }],
      }],
    };
    let collapsed = Signal::new(vec![child_id]);
    let mut rows = Vec::new();
    let mut row_count = 0;

    collect_tree_rows(
      &root,
      &mut Vec::new(),
      0,
      TreeRowWindow { start: 0, end: 16 },
      &mut row_count,
      &[],
      Signal::new(Vec::new()),
      &collapsed.get_untracked(),
      collapsed,
      true,
      None,
      None,
      &mut rows,
    );

    assert_eq!(rows.len(), 2);
    assert_eq!(row_count, 2);
  }

  #[test]
  fn collect_tree_rows_only_builds_requested_window() {
    let ids = IdGenerator::new();
    let root = DevToolsNode {
      id: ids.next(),
      tag: "Root".to_owned(),
      kind: super::super::snapshot::DevToolsNodeKind::Component,
      key: None,
      attrs: Vec::new(),
      text: None,
      color: None,
      props: None,
      signals: Vec::new(),
      memos: Vec::new(),
      contexts: Vec::new(),
      shape: Vec::new(),
      effects: Vec::new(),
      layout_box: None,
      hovered: false,
      active: false,
      focused: false,
      children: (0..6)
        .map(|index| DevToolsNode {
          id: ids.next(),
          tag: format!("Child{index}"),
          kind: super::super::snapshot::DevToolsNodeKind::Element,
          key: None,
          attrs: Vec::new(),
          text: None,
          color: None,
          props: None,
          signals: Vec::new(),
          memos: Vec::new(),
          contexts: Vec::new(),
          shape: Vec::new(),
          effects: Vec::new(),
          layout_box: None,
          hovered: false,
          active: false,
          focused: false,
          children: Vec::new(),
        })
        .collect(),
    };
    let collapsed = Signal::new(Vec::new());
    let mut rows = Vec::new();
    let mut row_count = 0;

    collect_tree_rows(
      &root,
      &mut Vec::new(),
      0,
      TreeRowWindow { start: 2, end: 5 },
      &mut row_count,
      &[],
      Signal::new(Vec::new()),
      &collapsed.get_untracked(),
      collapsed,
      true,
      None,
      None,
      &mut rows,
    );

    assert_eq!(row_count, 7);
    assert_eq!(rows.len(), 3);
  }
}
