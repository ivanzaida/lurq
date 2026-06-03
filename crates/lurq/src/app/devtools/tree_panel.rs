use super::{
  DevToolsDebugOverlayCallback, debug_overlay_path_for_selection,
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

pub(crate) fn tree_panel(
  snapshot: &DevToolsSnapshot,
  selected_path: Vec<usize>,
  selected: Signal<Vec<usize>>,
  collapsed_nodes: Vec<NodeId>,
  collapsed: Signal<Vec<NodeId>>,
  scroll_state: ScrollState,
  overlay_enabled: bool,
  on_debug_overlay_path: Option<DevToolsDebugOverlayCallback>,
) -> Element {
  let mut rows = Vec::new();
  if let Some(root) = &snapshot.root {
    collect_tree_rows(
      root,
      &mut Vec::new(),
      0,
      &selected_path,
      selected,
      &collapsed_nodes,
      collapsed,
      overlay_enabled,
      on_debug_overlay_path,
      &mut rows,
    );
  } else {
    rows.push(empty_state("No mounted root"));
  }

  Column::new()
    .child(section_header(
      "COMPONENT TREE",
      &format!("{} components", snapshot.node_count()),
    ))
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

fn collect_tree_rows(
  node: &DevToolsNode,
  path: &mut Vec<usize>,
  depth: usize,
  selected_path: &[usize],
  selected: Signal<Vec<usize>>,
  collapsed_nodes: &[NodeId],
  collapsed: Signal<Vec<NodeId>>,
  overlay_enabled: bool,
  on_debug_overlay_path: Option<DevToolsDebugOverlayCallback>,
  rows: &mut Vec<Element>,
) {
  let is_collapsed = !node.children.is_empty() && collapsed_nodes.contains(&node.id);
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
  ));
  if is_collapsed {
    return;
  }

  for (index, child) in node.children.iter().enumerate() {
    path.push(index);
    collect_tree_rows(
      child,
      path,
      depth + 1,
      selected_path,
      selected.clone(),
      collapsed_nodes,
      collapsed.clone(),
      overlay_enabled,
      on_debug_overlay_path.clone(),
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
      text: None,
      color: None,
      props: None,
      signals: Vec::new(),
      memos: Vec::new(),
      contexts: Vec::new(),
      shape: Vec::new(),
      effects: Vec::new(),
      children: vec![DevToolsNode {
        id: child_id,
        tag: "Child".to_owned(),
        kind: super::super::snapshot::DevToolsNodeKind::Element,
        key: None,
        text: None,
        color: None,
        props: None,
        signals: Vec::new(),
        memos: Vec::new(),
        contexts: Vec::new(),
        shape: Vec::new(),
        effects: Vec::new(),
        children: vec![DevToolsNode {
          id: ids.next(),
          tag: "Grandchild".to_owned(),
          kind: super::super::snapshot::DevToolsNodeKind::Element,
          key: None,
          text: None,
          color: None,
          props: None,
          signals: Vec::new(),
          memos: Vec::new(),
          contexts: Vec::new(),
          shape: Vec::new(),
          effects: Vec::new(),
          children: Vec::new(),
        }],
      }],
    };
    let collapsed = Signal::new(vec![child_id]);
    let mut rows = Vec::new();

    collect_tree_rows(
      &root,
      &mut Vec::new(),
      0,
      &[],
      Signal::new(Vec::new()),
      &collapsed.get_untracked(),
      collapsed,
      true,
      None,
      &mut rows,
    );

    assert_eq!(rows.len(), 2);
  }
}
