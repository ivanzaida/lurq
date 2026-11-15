//! The inspector, beside the tree: what the selected node is, what rendered it, its props, contexts, reactive
//! values and attributes in a table of their own each, and its layout and style at the end.

use super::{ComponentsState, DetailTab, Inspected, summary};
use crate::{
  app::{
    component::ComponentInfo,
    ctx::ComponentContextKind,
    devtools::{
      kit::{FILL, line, mono, sans, short_tag, short_type_name},
      snapshot::{DevToolsNode, DevToolsNodeKind, DevToolsSnapshot},
    },
    theme::{Density, PaletteColor},
  },
  components::{Column, Row, Spacer},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, border::Border},
  ui::{
    Badge, BadgeSize, BadgeVariant, Button, ButtonSize, ButtonVariant, EmptyState, Icon, Length, Separator, Switch,
    Tab, Table, TableColumn, TablePart, Tabs, TabsPart, TabsVariant, icons,
  },
};

/// The sizes the board gives the inspector (`pencil/lurq-devtools.pen`, Components · dark, Inspector).
const HEADER_HEIGHT: f32 = 48.0;
const RENDER_INFO_HEIGHT: f32 = 36.0;
const TABS_HEIGHT: f32 = 40.0;
const LABELS_HEIGHT: f32 = 32.0;
const PADDING: f32 = 20.0;
const ID_WIDTH: f32 = 104.0;
const NAME_WIDTH: f32 = 200.0;
const VALUE_WIDTH: f32 = 200.0;
const FIELD_INDENT: f32 = 16.0;

pub(super) fn panel(snapshot: &DevToolsSnapshot, state: &ComponentsState, inspected: &Inspected) -> Element {
  let path = state.selected_path.get();
  let Some(node) = snapshot.selected_node(&path) else {
    return Column::new()
      .size(FILL, FILL)
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .child(
        EmptyState::new("Nothing selected")
          .description("Select a node of the tree to see what it is made of.")
          .icon(icons::CURSOR),
      )
      .into();
  };
  Column::new()
    .size(FILL, FILL)
    .child(header(node, &path, state, inspected))
    .child(render_info(snapshot, node, &path))
    .child(
      Row::new()
        .width(FILL)
        .flex(1.0)
        .align_items(Alignment::Stretch)
        .child(detail(node, state))
        .child(summary::panel(node)),
    )
    .into()
}

/// The node's name, what it is, and what is done with it in the inspected window.
fn header(node: &DevToolsNode, path: &[usize], state: &ComponentsState, inspected: &Inspected) -> Row {
  let kind = match node.kind {
    DevToolsNodeKind::Component => "Component",
    DevToolsNodeKind::Element => "Element",
  };
  let (overlaid, overlay_told) = (state.clone(), inspected.clone());
  let (shot, shot_path) = (inspected.on_save_node_screenshot.clone(), path.to_vec());
  Row::new()
    .width(FILL)
    .height(HEADER_HEIGHT)
    .padding_left(PADDING)
    .padding_right(16.0)
    .spacing(10.0)
    .align_items(Alignment::Center)
    .border_bottom(line())
    .child(mono(short_tag(&node.tag), 15.0, FontWeight::SemiBold))
    .child(Badge::new(kind).variant(BadgeVariant::Outline).size(BadgeSize::Sm))
    .child(mono(&format!("NodeId({})", node.id.value()), 12.0, FontWeight::Normal).color(PaletteColor::MutedForeground))
    .child(Spacer::new().flex(1.0))
    .child(
      Row::new()
        .spacing(8.0)
        .align_items(Alignment::Center)
        .child(sans("Overlay", 12.0, FontWeight::Normal).color(PaletteColor::MutedForeground))
        .child(
          Switch::new("Overlay")
            .label_hidden(true)
            .on(state.overlay.get())
            .on_change(move |on| overlaid.set_overlay(on, &overlay_told))
            .id("devtools-overlay"),
        ),
    )
    .child(Separator::vertical().length(16.0))
    .child(
      Button::new("Screenshot")
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Xs)
        .icon(icons::IMAGE)
        .disabled(shot.is_none())
        .on_click(move |_| {
          if let Some(save) = &shot {
            save(shot_path.clone());
          }
        })
        .id("devtools-screenshot"),
    )
}

/// What is known of how the node came to be drawn.
fn render_info(snapshot: &DevToolsSnapshot, node: &DevToolsNode, path: &[usize]) -> Row {
  let children = match node.children.as_slice() {
    [] => "0".to_owned(),
    [child] => format!("1 ({})", short_tag(&child.tag)),
    children => children.len().to_string(),
  };
  let mut row = Row::new()
    .width(FILL)
    .height(RENDER_INFO_HEIGHT)
    .padding_horizontal(PADDING)
    .padding_bottom(10.0)
    .spacing(28.0)
    .align_items(Alignment::Center)
    .border_bottom(line());
  if let Some(parent) = rendered_by(snapshot, path) {
    row = row.child(stat("Rendered by", short_tag(&parent.tag)));
  }
  // A component says how it rendered; an element is drawn by the render of the component it is in.
  if let Some(render) = node.render {
    row = row
      .child(stat("Render count", &render.count.to_string()))
      .child(stat("Last render", &render_time(render.last)));
  }
  row.child(stat("Children", &children))
}

/// How long a render took, as short as it reads: `2.1ms`, `340µs`.
pub(crate) fn render_time(time: std::time::Duration) -> String {
  let micros = time.as_secs_f64() * 1_000_000.0;
  if micros >= 1_000.0 {
    format!("{:.1}ms", micros / 1_000.0)
  } else {
    format!("{micros:.0}µs")
  }
}

fn stat(label: &str, value: &str) -> Row {
  Row::new()
    .spacing(8.0)
    .align_items(Alignment::Center)
    .child(sans(label, 12.0, FontWeight::Normal).color(PaletteColor::MutedForeground))
    .child(mono(value, 12.0, FontWeight::Medium))
}

/// The component whose render built the node: the nearest component it is in.
pub(crate) fn rendered_by<'a>(snapshot: &'a DevToolsSnapshot, path: &[usize]) -> Option<&'a DevToolsNode> {
  let mut node = snapshot.root.as_ref()?;
  let mut nearest = None;
  for index in path {
    if node.kind == DevToolsNodeKind::Component {
      nearest = Some(node);
    }
    node = node.children.get(*index)?;
  }
  nearest
}

/// The lists of the node, one at a time: a tab for each over its table.
fn detail(node: &DevToolsNode, state: &ComponentsState) -> Column {
  let props = prop_rows(node);
  let contexts = context_rows(node);
  let reactive = reactive_rows(node);
  let attributes = attribute_rows(node);
  let tabs = Tabs::new(
    state.detail.clone(),
    [
      // The struct's own row is not one of its props.
      Tab::new(DetailTab::Props, "Props").count(props.len().saturating_sub(1)),
      Tab::new(DetailTab::Context, "Context").count(contexts.len()),
      Tab::new(DetailTab::Reactive, "Reactive").count(reactive.len()),
      Tab::new(DetailTab::Attributes, "Attributes").count(attributes.len()),
    ],
  )
  .variant(TabsVariant::Line)
  .label("Lists of the selected node")
  .id("devtools-detail")
  .style(TabsPart::List, |s| s.height(TABS_HEIGHT).padding_x(PADDING).gap(24.0))
  .style(TabsPart::Tab, |s| s.height(TABS_HEIGHT).gap(6.0))
  .style(TabsPart::Label, |s| s.font_size(13.0))
  .style(TabsPart::Count, |s| s.font_size(12.0));
  let table = match state.detail.get() {
    DetailTab::Props => props_table(props),
    DetailTab::Context => contexts_table(contexts),
    DetailTab::Reactive => reactive_table(reactive),
    DetailTab::Attributes => attributes_table(attributes),
  };
  Column::new()
    .height(FILL)
    .flex(1.0)
    .child(tabs)
    .child(Column::new().width(FILL).flex(1.0).child(table))
}

/// A table of the inspector as the board draws them: flat to the panel's edges, a row as low as the compact
/// density's, its columns one after the other with nothing between.
fn flat<R: Clone + Send + Sync + 'static>(table: Table<R>, label: &str, empty: &str) -> Element {
  table
    .label(label)
    .density(Density::Compact)
    .height(Length::Fill)
    .empty_text(empty)
    .style(TablePart::Root, |s| {
      s.radius(0.0).border(Border::inside(0.0, PaletteColor::Border))
    })
    .style(TablePart::Header, |s| {
      s.height(LABELS_HEIGHT)
        .padding_x(PADDING)
        .gap(0.0)
        .background(PaletteColor::Background)
    })
    .style(TablePart::HeaderCell, |s| s.font_size(12.0))
    .style(TablePart::Row, |s| s.padding_x(PADDING).gap(0.0))
    .into()
}

fn value_text(value: Option<&str>) -> Element {
  match value {
    Some(value) => mono(value, 13.0, FontWeight::Medium).into(),
    None => mono("—", 13.0, FontWeight::Normal)
      .color(PaletteColor::MutedForeground)
      .into(),
  }
}

fn muted(text: &str) -> Element {
  mono(text, 13.0, FontWeight::Normal)
    .color(PaletteColor::MutedForeground)
    .into()
}

/// An icon and a short word after it, in the first column of a list: `#12`, `uses`.
fn marked(icon: &str, text: &str) -> Element {
  Row::new()
    .spacing(10.0)
    .align_items(Alignment::Center)
    .child(Icon::new(icon).size(14.0).color(PaletteColor::MutedForeground))
    .child(mono(text, 13.0, FontWeight::Normal).color(PaletteColor::MutedForeground))
    .into()
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PropRow {
  /// How deep the field is in the props: 0 for the struct itself.
  depth: usize,
  name: String,
  value: Option<String>,
  type_name: String,
}

/// The props of the node: the struct, and its fields under it, each with the fields it holds.
pub(crate) fn prop_rows(node: &DevToolsNode) -> Vec<PropRow> {
  let Some(props) = &node.props else {
    return Vec::new();
  };
  let mut rows = vec![PropRow {
    depth: 0,
    name: short_type_name(&props.type_name),
    value: None,
    type_name: if props.fields.is_empty() { "()" } else { "struct" }.to_owned(),
  }];
  fn field(info: &ComponentInfo, depth: usize, rows: &mut Vec<PropRow>) {
    rows.push(PropRow {
      depth,
      name: info.name().to_owned(),
      value: info.formatted_value().map(str::to_owned),
      type_name: short_type_name(info.type_name()),
    });
    for child in info.children() {
      field(child, depth + 1, rows);
    }
  }
  for info in &props.fields {
    field(info, 1, &mut rows);
  }
  rows
}

fn props_table(rows: Vec<PropRow>) -> Element {
  let columns = vec![
    TableColumn::new("name", "Name")
      .width(NAME_WIDTH)
      .cell(|row: &PropRow| {
        let weight = if row.depth == 0 {
          FontWeight::Medium
        } else {
          FontWeight::Normal
        };
        Row::new()
          .padding_left(row.depth as f32 * FIELD_INDENT)
          .child(mono(&row.name, 13.0, weight))
          .into()
      }),
    TableColumn::new("value", "Value")
      .width(VALUE_WIDTH)
      .cell(|row: &PropRow| value_text(row.value.as_deref())),
    TableColumn::new("type", "Type").cell(|row: &PropRow| muted(&row.type_name)),
  ];
  flat(Table::new(columns, rows), "Props", "This node takes no props")
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ContextRow {
  relation: &'static str,
  type_name: String,
}

pub(crate) fn context_rows(node: &DevToolsNode) -> Vec<ContextRow> {
  node
    .contexts
    .iter()
    .map(|context| ContextRow {
      relation: match context.kind {
        ComponentContextKind::Provided => "provides",
        ComponentContextKind::Consumed => "uses",
      },
      type_name: short_type_name(&context.type_name),
    })
    .collect()
}

fn contexts_table(rows: Vec<ContextRow>) -> Element {
  let columns = vec![
    TableColumn::new("relation", "Relation")
      .width(ID_WIDTH)
      .cell(|row: &ContextRow| marked(icons::SHARE_NODES, row.relation)),
    TableColumn::new("type", "Type").cell(|row: &ContextRow| value_text(Some(&row.type_name))),
  ];
  flat(
    Table::new(columns, rows),
    "Context",
    "This node provides and uses no context",
  )
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ReactiveRow {
  icon: &'static str,
  id: usize,
  value: Option<String>,
  type_name: String,
}

/// The node's signals, then its memos, then its effects.
pub(crate) fn reactive_rows(node: &DevToolsNode) -> Vec<ReactiveRow> {
  let signals = node.signals.iter().map(|signal| ReactiveRow {
    icon: icons::BOLT,
    id: signal.id,
    value: signal.formatted_value(),
    type_name: format!("Signal<{}>", short_type_name(&signal.type_name)),
  });
  let memos = node.memos.iter().map(|memo| ReactiveRow {
    icon: icons::PULSE,
    id: memo.id,
    value: memo.formatted_value(),
    type_name: format!("Memo<{}>", short_type_name(&memo.type_name)),
  });
  let effects = node.effects.iter().map(|effect| ReactiveRow {
    icon: icons::PLAY_CIRCLE,
    id: effect.id,
    value: None,
    type_name: "Effect".to_owned(),
  });
  signals.chain(memos).chain(effects).collect()
}

fn reactive_table(rows: Vec<ReactiveRow>) -> Element {
  let columns = vec![
    TableColumn::new("id", "ID")
      .width(ID_WIDTH)
      .cell(|row: &ReactiveRow| marked(row.icon, &format!("#{}", row.id))),
    TableColumn::new("value", "Value")
      .width(VALUE_WIDTH)
      .cell(|row: &ReactiveRow| value_text(row.value.as_deref())),
    TableColumn::new("type", "Type").cell(|row: &ReactiveRow| muted(&row.type_name)),
  ];
  flat(
    Table::new(columns, rows),
    "Reactive values",
    "This node has no signals, memos or effects",
  )
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AttributeRow {
  name: String,
  value: String,
}

pub(crate) fn attribute_rows(node: &DevToolsNode) -> Vec<AttributeRow> {
  node
    .attrs
    .iter()
    .map(|(name, value)| AttributeRow {
      name: name.clone(),
      value: format!("{value:?}"),
    })
    .collect()
}

fn attributes_table(rows: Vec<AttributeRow>) -> Element {
  let columns = vec![
    TableColumn::new("name", "Name")
      .width(NAME_WIDTH)
      .cell(|row: &AttributeRow| mono(&row.name, 13.0, FontWeight::Normal).into()),
    TableColumn::new("value", "Value").cell(|row: &AttributeRow| value_text(Some(&row.value))),
  ];
  flat(Table::new(columns, rows), "Attributes", "This node has no attributes")
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::IdGenerator;

  #[test]
  fn a_node_is_rendered_by_the_nearest_component_it_is_in() {
    let ids = IdGenerator::new();
    let leaf = DevToolsNode::for_test(ids.next(), "Text", DevToolsNodeKind::Element, vec![]);
    let column = DevToolsNode::for_test(ids.next(), "Column", DevToolsNodeKind::Element, vec![leaf]);
    let card = DevToolsNode::for_test(ids.next(), "demo::Card", DevToolsNodeKind::Component, vec![column]);
    let row = DevToolsNode::for_test(ids.next(), "Row", DevToolsNodeKind::Element, vec![card]);
    let snapshot = DevToolsSnapshot {
      root: Some(DevToolsNode::for_test(
        ids.next(),
        "demo::App",
        DevToolsNodeKind::Component,
        vec![row],
      )),
      frame: Default::default(),
    };
    let by = |path: &[usize]| rendered_by(&snapshot, path).map(|node| short_tag(&node.tag));
    assert_eq!(by(&[]), None, "nothing of the tree rendered its root");
    assert_eq!(by(&[0]), Some("App"));
    assert_eq!(
      by(&[0, 0]),
      Some("App"),
      "a component is rendered by the one it is in, not by itself"
    );
    assert_eq!(by(&[0, 0, 0]), Some("Card"));
    assert_eq!(by(&[0, 0, 0, 0]), Some("Card"));
    assert_eq!(by(&[4]), None, "a path that leads nowhere has no node");
  }

  #[test]
  fn a_render_time_reads_in_the_unit_it_is_of() {
    use std::time::Duration;
    assert_eq!(render_time(Duration::from_micros(2_100)), "2.1ms");
    assert_eq!(render_time(Duration::from_micros(340)), "340µs");
    assert_eq!(render_time(Duration::ZERO), "0µs");
  }

  #[test]
  fn attributes_are_shown_as_they_would_be_written() {
    let mut node = DevToolsNode::for_test(IdGenerator::new().next(), "Row", DevToolsNodeKind::Element, vec![]);
    node.attrs = vec![("id".to_owned(), "app-root".to_owned())];
    assert_eq!(
      attribute_rows(&node),
      [AttributeRow {
        name: "id".to_owned(),
        value: "\"app-root\"".to_owned(),
      }]
    );
  }

  #[test]
  fn a_node_without_props_has_no_row_for_them() {
    let node = DevToolsNode::for_test(IdGenerator::new().next(), "Row", DevToolsNodeKind::Element, vec![]);
    assert!(prop_rows(&node).is_empty());
    assert!(reactive_rows(&node).is_empty());
    assert!(context_rows(&node).is_empty());
  }
}
