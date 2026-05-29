use super::{
  snapshot::{DevToolsNode, DevToolsSnapshot},
  style::{
    BLUE, BORDER, FILL, GREEN, MUTED, SELECTED, SURFACE, SURFACE_2, badge, empty_state, section_header, short_tag, text,
  },
};
use crate::{
  components::{Column, Row, ScrollVertical, Spacer},
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  node::{CursorIcon, Element, color::Color},
};

pub(crate) fn tree_panel(
  snapshot: &DevToolsSnapshot,
  selected_path: Vec<usize>,
  selected: Signal<Vec<usize>>,
) -> Element {
  let mut rows = Vec::new();
  if let Some(root) = &snapshot.root {
    collect_tree_rows(root, &mut Vec::new(), 0, &selected_path, selected, &mut rows);
  } else {
    rows.push(empty_state("No mounted root"));
  }

  Column::new()
    .child(section_header(
      "COMPONENT TREE",
      &format!("{} nodes", snapshot.node_count()),
    ))
    .child(
      ScrollVertical::new(Column::new().with_children(rows).width(FILL))
        .height(FILL)
        .width(FILL),
    )
    .width(380.0)
    .height(FILL)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .into()
}

fn collect_tree_rows(
  node: &DevToolsNode,
  path: &mut Vec<usize>,
  depth: usize,
  selected_path: &[usize],
  selected: Signal<Vec<usize>>,
  rows: &mut Vec<Element>,
) {
  rows.push(tree_row(
    node,
    path.clone(),
    depth,
    path.as_slice() == selected_path,
    selected.clone(),
  ));
  for (index, child) in node.children.iter().enumerate() {
    path.push(index);
    collect_tree_rows(child, path, depth + 1, selected_path, selected.clone(), rows);
    path.pop();
  }
}

fn tree_row(
  node: &DevToolsNode,
  path: Vec<usize>,
  depth: usize,
  selected: bool,
  selected_path: Signal<Vec<usize>>,
) -> Element {
  let indent = 8.0 + depth as f32 * 16.0;
  let child_count = node.children.len();
  let click_path = path;
  let tag_color = if node.tag.starts_with("lurq::") || node.tag.contains("Demo") {
    BLUE
  } else {
    MUTED
  };
  let mut row = Row::new()
    .align_items(Alignment::Center)
    .spacing(6.0)
    .child(Spacer::new().width(indent))
    .child(text(if child_count > 0 { "v" } else { "" }, 12.0, FontWeight::Normal, MUTED).width(12.0))
    .child(text("<", 12.0, FontWeight::Normal, MUTED))
    .child(text(short_tag(&node.tag), 12.0, FontWeight::Bold, tag_color).nowrap())
    .child(text("/>", 12.0, FontWeight::Normal, MUTED))
    .width(FILL)
    .height(26.0)
    .fill(if selected { SELECTED } else { "#00000000" })
    .cursor(CursorIcon::Pointer)
    .on_click(move |_| selected_path.set(click_path.clone()));

  if child_count > 0 {
    row = row.child(badge(&child_count.to_string(), GREEN, SURFACE_2));
  }

  row.into()
}
