mod inspector;
mod snapshot;
mod style;
mod top_bar;
mod tree_panel;

use std::sync::{
  Arc,
  atomic::{AtomicU64, Ordering},
};

pub use snapshot::{DevToolsNode, DevToolsSnapshot, FrameProfileSnapshot};

pub use self::snapshot::DevToolsSnapshot as Snapshot;
use crate::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::{NodeId, Signal},
  node::Element,
};

static LUCIDE_TTF: &[u8] = include_bytes!("../../../assets/lucide.ttf");

pub type DevToolsDebugOverlayCallback = Arc<dyn Fn(Option<Vec<usize>>) + Send + Sync>;
pub type DevToolsBoolCallback = Arc<dyn Fn(bool) + Send + Sync>;

pub fn load_fonts(app: &mut crate::app::App) {
  app.load_font(LUCIDE_TTF.to_vec());
  app.register_font("lucide", "lucide");
}

#[derive(Clone, crate::DevtoolsInspectable)]
pub struct DevToolsProps {
  pub snapshot: DevToolsSnapshot,
  pub picked_path: Option<Vec<usize>>,
  pub picked_revision: u64,
  #[devtools_ignore]
  pub on_debug_overlay_path: Option<DevToolsDebugOverlayCallback>,
  #[devtools_ignore]
  pub on_overlay_enabled: Option<DevToolsBoolCallback>,
  #[devtools_ignore]
  pub on_pick_inspected: Option<DevToolsBoolCallback>,
}

pub struct DevTools {
  selected_path: Signal<Vec<usize>>,
  collapsed_nodes: Signal<Vec<NodeId>>,
  show_inspected_overlay: Signal<bool>,
  pick_inspected: Signal<bool>,
  last_picked_revision: AtomicU64,
}

impl PartialEq for DevToolsProps {
  fn eq(&self, other: &Self) -> bool {
    self.snapshot.root == other.snapshot.root && self.picked_revision == other.picked_revision
  }
}

impl Component for DevTools {
  type Props = DevToolsProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      selected_path: ctx.signal(Vec::new()),
      collapsed_nodes: ctx.signal(Vec::new()),
      show_inspected_overlay: ctx.signal(true),
      pick_inspected: ctx.signal(false),
      last_picked_revision: AtomicU64::new(0),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<DevToolsProps>().clone();
    let snapshot = props.snapshot;
    let mut selected_path = self.selected_path.get();
    let collapsed_nodes = self.collapsed_nodes.get();
    let show_inspected_overlay = self.show_inspected_overlay.get();
    let mut pick_inspected = self.pick_inspected.get();

    if props.picked_revision != self.last_picked_revision.load(Ordering::Relaxed) {
      self
        .last_picked_revision
        .store(props.picked_revision, Ordering::Relaxed);
      self.pick_inspected.set(false);
      pick_inspected = false;
      if let Some(path) = props.picked_path.clone() {
        self.selected_path.set(path.clone());
        selected_path = path;
      }
    }

    let selected = snapshot.selected_node(&selected_path);

    Column::new()
      .child(top_bar::top_bar(
        snapshot.node_count(),
        show_inspected_overlay,
        self.show_inspected_overlay.clone(),
        pick_inspected,
        self.pick_inspected.clone(),
        selected_path.clone(),
        selected.is_some(),
        props.on_debug_overlay_path.clone(),
        props.on_overlay_enabled.clone(),
        props.on_pick_inspected.clone(),
      ))
      .child(
        Row::new()
          .child(tree_panel::tree_panel(
            &snapshot,
            selected_path,
            self.selected_path.clone(),
            collapsed_nodes,
            self.collapsed_nodes.clone(),
            show_inspected_overlay,
            props.on_debug_overlay_path.clone(),
          ))
          .child(
            Column::new()
              .child(inspector::inspector_panel(selected, snapshot.frame))
              .width(style::FILL)
              .height(style::FILL)
              .flex(1.0),
          )
          .width(style::FILL)
          .height(style::FILL)
          .flex(1.0),
      )
      .width(style::FILL)
      .height(style::FILL)
      .fill(style::BG)
  }
}

pub(crate) fn debug_overlay_path_for_selection(
  overlay_enabled: bool,
  selected_path: Vec<usize>,
  has_selection: bool,
) -> Option<Vec<usize>> {
  if overlay_enabled && has_selection {
    Some(selected_path)
  } else {
    None
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    app::{devtools::snapshot::DevToolsNodeKind, theme::Theme},
    components::Column,
    layout::text_style::FontWeight,
  };

  #[test]
  fn snapshot_collects_tree_nodes() {
    let mut tree = crate::app::Tree::new();
    tree.mount_root::<SnapshotTestApp>(Theme::default(), ());
    assert!(tree.draw_debug_overlay_over_node(vec![0]));
    assert!(!tree.draw_debug_overlay_over_node(vec![0]));
    assert!(tree.clear_debug_overlay());
    tree.draw_perf_overlay();
    assert!(tree.perf_overlay_enabled());
    tree.tick_perf_overlay();

    let snapshot = DevToolsSnapshot::from_tree(&tree);

    assert!(snapshot.node_count() >= 2);
    assert_eq!(
      snapshot.root.as_ref().map(|node| style::short_tag(&node.tag)),
      Some("SnapshotTestApp")
    );
    assert_eq!(
      snapshot.root.as_ref().map(|node| node.kind),
      Some(DevToolsNodeKind::Component)
    );
    assert_eq!(
      snapshot
        .root
        .as_ref()
        .and_then(|node| node.children.first())
        .map(|node| node.kind),
      Some(DevToolsNodeKind::Element)
    );
    let child_shape = snapshot
      .root
      .as_ref()
      .and_then(|node| node.children.first())
      .map(|node| node.shape.as_slice())
      .unwrap_or(&[]);
    assert_eq!(
      child_shape
        .iter()
        .find(|row| row.label == "layout")
        .and_then(|row| row.value.as_deref()),
      Some("Padding")
    );
    let padding = child_shape.iter().find(|row| row.label == "padding").unwrap();
    assert_eq!(padding.value, None);
    assert_eq!(
      padding
        .children
        .iter()
        .map(|row| (row.label.as_str(), row.value.as_deref()))
        .collect::<Vec<_>>(),
      vec![
        ("left", Some("8px")),
        ("right", Some("8px")),
        ("top", Some("8px")),
        ("bottom", Some("8px")),
      ]
    );
    let hover_style = child_shape.iter().find(|row| row.label == "hover style").unwrap();
    assert_eq!(
      hover_style
        .children
        .iter()
        .find(|row| row.label == "fill")
        .and_then(|row| row.value.as_deref()),
      Some("#ff0000")
    );
    let hover_padding = hover_style.children.iter().find(|row| row.label == "padding").unwrap();
    assert_eq!(
      hover_padding
        .children
        .iter()
        .find(|row| row.label == "left")
        .and_then(|row| row.value.as_deref()),
      Some("4px")
    );
    let active_style = child_shape.iter().find(|row| row.label == "active style").unwrap();
    let active_frame = active_style.children.iter().find(|row| row.label == "frame").unwrap();
    assert_eq!(
      active_frame
        .children
        .iter()
        .find(|row| row.label == "width")
        .and_then(|row| row.value.as_deref()),
      Some("32px")
    );
    assert_eq!(
      snapshot
        .root
        .as_ref()
        .and_then(|node| node
          .children
          .iter()
          .find(|child| style::short_tag(&child.tag) == "KeyedChild"))
        .and_then(|node| node.key.as_deref()),
      Some("child-key")
    );
    assert_eq!(
      snapshot
        .root
        .as_ref()
        .and_then(|node| node.props.as_ref())
        .map(|props| props.fields.len()),
      Some(0)
    );
    assert_eq!(
      snapshot
        .root
        .as_ref()
        .and_then(|node| node.signals.first())
        .map(|signal| signal.type_name.as_ref()),
      Some("i32")
    );
    assert_eq!(snapshot.root.as_ref().map(|node| node.contexts.len()), Some(2));
  }

  #[test]
  fn debug_overlay_path_for_selection_respects_toggle_and_selection() {
    assert_eq!(
      debug_overlay_path_for_selection(true, vec![1, 2], true),
      Some(vec![1, 2])
    );
    assert_eq!(debug_overlay_path_for_selection(false, vec![1, 2], true), None);
    assert_eq!(debug_overlay_path_for_selection(true, vec![1, 2], false), None);
  }

  struct SnapshotTestApp {
    count: crate::core::Signal<i32>,
  }

  impl Component for SnapshotTestApp {
    type Props = ();

    fn create(ctx: &mut Ctx) -> Self {
      Self { count: ctx.signal(0) }
    }

    fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
      ctx.provide(42_i32);
      let _ = ctx.use_context::<i32>();
      Column::new()
        .child(
          style::text(
            &format!("child {}", self.count.get()),
            12.0,
            FontWeight::Normal,
            style::TEXT,
          )
          .padding(8.0)
          .hovered(|style| style.fill("#ff0000").padding_left(4.0))
          .active(|style| style.width(32.0)),
        )
        .child(ctx.mount_keyed::<KeyedChild>("child-key", ()))
    }
  }

  struct KeyedChild;

  impl Component for KeyedChild {
    type Props = ();

    fn create(_ctx: &mut Ctx) -> Self {
      Self
    }

    fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
      style::text("keyed child", 12.0, FontWeight::Normal, style::TEXT)
    }
  }
}
