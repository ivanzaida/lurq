//! The tree of the inspected window, at the start of the Components tab: a toolbar over a table whose first column
//! is the tree, a row for each node that is shown.

use std::sync::Arc;

use super::{ComponentsState, Inspected, search::SearchQuery};
use crate::{
  app::{
    ctx::Ctx,
    devtools::{
      kit::{FILL, line, mono, sans, short_tag},
      snapshot::{DevToolsNode, DevToolsNodeKind, DevToolsSnapshot},
    },
    events::MouseEvent,
    theme::{BorderSize, Density, PaletteColor},
  },
  components::{Column, Row},
  core::NodeId,
  layout::{Alignment, text_style::FontWeight},
  node::{CursorIcon, Element, border::Border},
  ui::{
    Icon, Length, Recipe, SearchField, SearchFieldSize, Table, TableColumn, TableCount, TablePart, Toggle, TogglePart,
    ToggleSize, ToggleState, icons,
  },
};

/// The sizes the board gives the tree (`pencil/lurq-devtools.pen`, Components · dark, Tree Panel).
const TOOLBAR_HEIGHT: f32 = 44.0;
const HEADER_HEIGHT: f32 = 28.0;
const ROW_HEIGHT: f32 = 26.0;
const INDENT: f32 = 12.0;
const CHEVRON: f32 = 12.0;
const KIND_WIDTH: f32 = 76.0;
const CHILDREN_WIDTH: f32 = 52.0;

/// What the search box says reaches it: the key the window binds.
const SEARCH_KEY: &str = if cfg!(target_os = "macos") { "⌘F" } else { "Ctrl+F" };

/// One row of the tree: a node of the inspected window that is shown.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Line {
  /// The way to the node from the root, by the place of each child: what the node is told apart by.
  pub(crate) path: Vec<usize>,
  pub(crate) id: NodeId,
  pub(crate) depth: usize,
  pub(crate) name: String,
  /// What is written after the name: the node's key, its attributes, the text of a text.
  pub(crate) attrs: String,
  pub(crate) component: bool,
  pub(crate) children: usize,
  /// Its children are shown under it.
  pub(crate) open: bool,
  /// It is one the search asks for, and not only on the way to one: every row is while nothing is searched.
  pub(crate) found: bool,
}

/// The rows the tree shows, from the top. A closed node hides what is in it; a search shows the nodes it asks
/// for with those they are in, every one of them open.
pub(crate) fn lines(snapshot: &DevToolsSnapshot, closed: &[NodeId], query: Option<&SearchQuery>) -> Vec<Line> {
  let mut lines = Vec::new();
  if let Some(root) = &snapshot.root {
    collect(root, &mut Vec::new(), closed, query, &mut lines);
  }
  lines
}

/// Adds the node and what is shown of it, and says whether the search asked for any of it: a node the search
/// passes by with all it holds leaves no row.
fn collect(
  node: &DevToolsNode,
  path: &mut Vec<usize>,
  closed: &[NodeId],
  query: Option<&SearchQuery>,
  lines: &mut Vec<Line>,
) -> bool {
  let at = lines.len();
  let open = !node.children.is_empty() && (query.is_some() || !closed.contains(&node.id));
  let mut found = query.is_none_or(|query| query.matches(node));
  lines.push(Line {
    path: path.clone(),
    id: node.id,
    depth: path.len(),
    name: short_tag(&node.tag).to_owned(),
    attrs: attrs_text(node),
    component: node.kind == DevToolsNodeKind::Component,
    children: node.children.len(),
    open,
    found,
  });
  if open {
    for (index, child) in node.children.iter().enumerate() {
      path.push(index);
      found |= collect(child, path, closed, query, lines);
      path.pop();
    }
  }
  if !found {
    lines.truncate(at);
  }
  found
}

/// What a row says after the node's name, as a tag of markup would: `key="a" id="app-root"`.
fn attrs_text(node: &DevToolsNode) -> String {
  let key = node.key.as_deref().map(|key| ("key", key));
  let attrs = node.attrs.iter().map(|(name, value)| (name.as_str(), value.as_str()));
  let text = node
    .text
    .as_deref()
    .filter(|_| short_tag(&node.tag) == "Text")
    .map(|text| ("text", text));
  key
    .into_iter()
    .chain(attrs)
    .chain(text)
    .map(|(name, value)| format!("{name}={}", quoted(value)))
    .collect::<Vec<_>>()
    .join(" ")
}

/// A value between quotes on one line, cut where it is long.
fn quoted(value: &str) -> String {
  const MOST: usize = 48;
  let mut escaped = String::new();
  for ch in value.chars().take(MOST) {
    match ch {
      '\\' => escaped.push_str("\\\\"),
      '"' => escaped.push_str("\\\""),
      '\n' => escaped.push_str("\\n"),
      '\r' => escaped.push_str("\\r"),
      '\t' => escaped.push_str("\\t"),
      ch => escaped.push(ch),
    }
  }
  if value.chars().count() > MOST {
    escaped.push('…');
  }
  format!("\"{escaped}\"")
}

/// The nodes on the way to `path`, the node itself left out: what is opened to show it.
pub(crate) fn ancestors(snapshot: &DevToolsSnapshot, path: &[usize]) -> Vec<NodeId> {
  let mut ids = Vec::new();
  let mut node = snapshot.root.as_ref();
  for index in path {
    let Some(at) = node else { break };
    ids.push(at.id);
    node = at.children.get(*index);
  }
  ids
}

/// The panel: the toolbar, and the tree under it in all the height that is left.
pub(super) fn panel(
  ctx: &mut Ctx,
  snapshot: &DevToolsSnapshot,
  state: &ComponentsState,
  inspected: &Inspected,
) -> Element {
  if state.search_wanted() {
    ctx.focus(&state.search_field);
  }
  let query = SearchQuery::parse(&state.search.get());
  let lines = lines(snapshot, &state.closed.get(), query.as_ref());
  Column::new()
    .size(FILL, FILL)
    .child(toolbar(state, inspected))
    .child(Column::new().width(FILL).flex(1.0).border_top(line()).child(table(
      ctx,
      lines,
      query.is_some(),
      state,
      inspected,
    )))
    .into()
}

fn toolbar(state: &ComponentsState, inspected: &Inspected) -> Row {
  let (picking, inspected) = (state.clone(), inspected.clone());
  Row::new()
    .width(FILL)
    .height(TOOLBAR_HEIGHT)
    .padding_horizontal(12.0)
    .spacing(8.0)
    .align_items(Alignment::Center)
    .child(
      Toggle::new("Pick")
        .icon(icons::CROSSHAIR)
        .size(ToggleSize::Xs)
        .rules(&outlined_toggle())
        .on(state.pick.get())
        .on_change(move |on| picking.set_pick(on, &inspected))
        .id("devtools-pick"),
    )
    .child(
      Row::new().flex(1.0).child(
        SearchField::new(state.search.clone())
          .placeholder("Search components...")
          .label("Search components")
          .shortcut(SEARCH_KEY)
          .ref_element(state.search_field.clone())
          .size(SearchFieldSize::Sm)
          .input_rules(&compact_input())
          .full_width(true)
          .id("devtools-search"),
      ),
    )
}

/// The board's Pick: a toggle in an edge, on the window's fill, picked out while it is on.
fn outlined_toggle() -> Recipe {
  use PaletteColor::{Background, Highlight, HighlightForeground, Input, MutedForeground};
  Recipe::new()
    .base(TogglePart::Root, |s| {
      s.background(Background)
        .border(Border::inside(BorderSize::Sm, Input))
        .foreground(MutedForeground)
        .padding_x(10.0)
        .gap(6.0)
    })
    .state(ToggleState::On, TogglePart::Root, |s| {
      s.background(Highlight).foreground(HighlightForeground)
    })
}

/// The board's search: as high as the toolbar's other controls, in their type.
fn compact_input() -> Recipe {
  use crate::ui::InputPart;
  Recipe::new().base(InputPart::Control, |s| s.height(Density::Compact).font_size(12.0))
}

fn table(ctx: &mut Ctx, lines: Vec<Line>, searching: bool, state: &ComponentsState, inspected: &Inspected) -> Element {
  let toggle = {
    let state = state.clone();
    Arc::new(move |id: NodeId| state.toggle(id))
  };
  let opened = state.clone();
  let (selecting, selected_in) = (state.clone(), inspected.clone());
  // What a search found, without the rows shown only because what it found is in them.
  let found = lines.iter().filter(|line| line.found).count();
  let by_path: Arc<Vec<Line>> = Arc::new(lines);
  let rows = by_path.clone();
  let columns = vec![
    TableColumn::new("node", "Node").cell({
      let toggle = toggle.clone();
      move |line: &Line| node_cell(line, searching, toggle.clone())
    }),
    TableColumn::new("kind", "Kind")
      .width(KIND_WIDTH)
      .cell(|line: &Line| kind_cell(line.component)),
    TableColumn::new("children", "Children")
      .width(CHILDREN_WIDTH)
      .text(|line: &Line| match line.children {
        0 => "—".to_owned(),
        children => children.to_string(),
      })
      .numeric(),
  ];
  Table::new_shared(columns, rows)
    .row_key(|line: &Line| line.path.clone())
    .label("Component tree")
    .id("devtools-tree")
    .checkboxes(false)
    .selected_controlled([state.selected_path.get()], move |paths: Vec<Vec<usize>>| {
      // A press with Ctrl on the selected row would leave none selected: the inspector always shows one.
      if let Some(path) = paths.into_iter().next_back() {
        selecting.select(path, &selected_in);
      }
    })
    // Enter and a double press open and close the row, as its chevron does.
    .on_row_activate(move |path: &Vec<usize>| {
      if !searching && let Some(line) = by_path.iter().find(|line| &line.path == path).filter(|line| line.children > 0) {
        opened.toggle(line.id);
      }
    })
    // A node picked in the inspected window is selected from outside the table: its row is brought into view.
    .reveal(state.selected_path.get(), state.shown())
    .density(Density::Compact)
    .height(Length::Fill)
    .count(true)
    .count_text(move |count: TableCount| match (searching, found, count.total) {
      (true, 1, _) => "1 match".to_owned(),
      (true, matches, _) => format!("{matches} matches"),
      (false, _, 1) => "1 component".to_owned(),
      (false, _, components) => format!("{components} components"),
    })
    .empty_text(if searching {
      "No components match the search"
    } else {
      "Nothing is mounted in the inspected window"
    })
    // The board's tree runs to the panel's edges, with rows as low as a line of code.
    .style(TablePart::Root, |s| {
      s.radius(0.0).border(Border::inside(0.0, PaletteColor::Border))
    })
    .style(TablePart::Header, |s| s.height(HEADER_HEIGHT).padding_x(12.0).gap(12.0))
    .style(TablePart::HeaderCell, |s| s.font_size(12.0))
    .style(TablePart::Row, |s| s.height(ROW_HEIGHT).padding_x(12.0).gap(12.0))
    .style(TablePart::Cell, |s| s.font_size(12.0))
    .style(TablePart::Footer, |s| s.height(HEADER_HEIGHT).padding_x(12.0))
    .style(TablePart::Count, |s| s.font_size(12.0))
    .mount(ctx)
}

/// The tree's cell of a row: its indent, its chevron where it holds something, its name and what is said of it.
fn node_cell(line: &Line, searching: bool, toggle: Arc<dyn Fn(NodeId) + Send + Sync>) -> Element {
  // As wide as its column, so that a long row is cut at the column's edge and not in the room after it.
  let mut cell = Row::new()
    .width(FILL)
    .height(ROW_HEIGHT)
    .spacing(6.0)
    .padding_left(line.depth as f32 * INDENT)
    .align_items(Alignment::Center);
  cell = if line.children == 0 {
    cell.child(Row::new().size(CHEVRON, CHEVRON))
  } else {
    let glyph = Icon::new(if line.open {
      icons::CHEVRON_DOWN
    } else {
      icons::CHEVRON_RIGHT
    })
    .size(CHEVRON)
    .color(PaletteColor::MutedForeground);
    let mut chevron = Row::new()
      .size(CHEVRON, ROW_HEIGHT)
      .align_items(Alignment::Center)
      .child(glyph);
    // A search shows every row it found open: there is nothing to close while it holds.
    if !searching {
      let id = line.id;
      chevron = chevron.cursor(CursorIcon::Pointer).on_click(move |event: MouseEvent| {
        // The press is the chevron's: it does not select the row as well.
        event.stop_propagation();
        toggle(id);
      });
    }
    cell.child(chevron)
  };
  let weight = if line.component {
    FontWeight::SemiBold
  } else {
    FontWeight::Normal
  };
  let name = mono(&line.name, 12.0, weight);
  // A row a search shows only on the way to what it found steps back.
  cell = cell.child(if line.found {
    name
  } else {
    name.color(PaletteColor::MutedForeground)
  });
  if !line.attrs.is_empty() {
    cell = cell.child(mono(&line.attrs, 12.0, FontWeight::Normal).color(PaletteColor::MutedForeground));
  }
  cell.into()
}

fn kind_cell(component: bool) -> Element {
  if component {
    sans("Component", 12.0, FontWeight::Medium).into()
  } else {
    sans("Element", 12.0, FontWeight::Normal)
      .color(PaletteColor::MutedForeground)
      .into()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::IdGenerator;

  /// `Root > Child > Grandchild`, and their ids.
  fn family() -> (DevToolsSnapshot, [NodeId; 3]) {
    let ids = IdGenerator::new();
    let (root, child, grandchild) = (ids.next(), ids.next(), ids.next());
    let mut leaf = DevToolsNode::for_test(grandchild, "Grandchild", DevToolsNodeKind::Element, vec![]);
    leaf.key = Some("target".to_owned());
    let snapshot = DevToolsSnapshot {
      root: Some(DevToolsNode::for_test(
        root,
        "app::Root",
        DevToolsNodeKind::Component,
        vec![DevToolsNode::for_test(
          child,
          "Child",
          DevToolsNodeKind::Element,
          vec![leaf],
        )],
      )),
      frame: Default::default(),
    };
    (snapshot, [root, child, grandchild])
  }

  fn names(lines: &[Line]) -> Vec<&str> {
    lines.iter().map(|line| line.name.as_str()).collect()
  }

  #[test]
  fn every_node_is_a_row_under_the_one_it_is_in() {
    let (snapshot, _) = family();
    let lines = lines(&snapshot, &[], None);
    assert_eq!(names(&lines), ["Root", "Child", "Grandchild"]);
    assert!(
      lines.iter().all(|line| line.found),
      "with nothing searched every row is one to show"
    );
    assert_eq!(lines.iter().map(|line| line.depth).collect::<Vec<_>>(), [0, 1, 2]);
    assert_eq!(lines[2].path, [0, 0]);
    assert!(lines[0].component && lines[0].open);
    assert!(!lines[2].open, "a node that holds nothing has nothing to show open");
    assert_eq!(lines[2].attrs, "key=\"target\"");
  }

  #[test]
  fn a_closed_node_hides_what_it_holds() {
    let (snapshot, [_, child, _]) = family();
    let lines = lines(&snapshot, &[child], None);
    assert_eq!(names(&lines), ["Root", "Child"]);
    assert!(!lines[1].open);
    assert_eq!(lines[1].children, 1);
    assert!(lines.iter().all(|line| line.path != [0, 0]));
  }

  #[test]
  fn a_search_shows_what_it_found_with_the_nodes_it_is_in() {
    let (snapshot, [_, child, _]) = family();
    let query = SearchQuery::parse("key:target");
    // The node found is in a closed one: the search opens it.
    let found = lines(&snapshot, &[child], query.as_ref());
    assert_eq!(names(&found), ["Root", "Child", "Grandchild"]);
    assert!(found[1].open);
    // The two it is in are shown on the way to it, and are not what was found.
    assert_eq!(
      found.iter().map(|line| line.found).collect::<Vec<_>>(),
      [false, false, true]
    );

    let query = SearchQuery::parse("child -grandchild");
    assert_eq!(names(&lines(&snapshot, &[], query.as_ref())), ["Root", "Child"]);

    let query = SearchQuery::parse("missing");
    assert!(lines(&snapshot, &[], query.as_ref()).is_empty());
  }

  #[test]
  fn the_way_to_a_node_is_the_nodes_it_is_in() {
    let (snapshot, [root, child, _]) = family();
    assert_eq!(ancestors(&snapshot, &[0, 0]), [root, child]);
    assert_eq!(ancestors(&snapshot, &[]), []);
    // A path that leads nowhere stops where the tree does.
    assert_eq!(ancestors(&snapshot, &[0, 0, 5, 1]).len(), 3);
  }

  #[test]
  fn a_value_is_quoted_on_one_line_and_cut_where_it_is_long() {
    assert_eq!(quoted("a \"b\"\n"), "\"a \\\"b\\\"\\n\"");
    let long = "x".repeat(60);
    assert_eq!(quoted(&long), format!("\"{}…\"", "x".repeat(48)));
  }
}
