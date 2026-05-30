mod inspector;
mod snapshot;
mod style;
mod top_bar;
mod tree_panel;

pub use snapshot::{DevToolsNode, DevToolsSnapshot, FrameProfileSnapshot};

pub use self::snapshot::DevToolsSnapshot as Snapshot;
use crate::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  node::Element,
};

static LUCIDE_TTF: &[u8] = include_bytes!("../../../assets/lucide.ttf");

pub fn load_fonts(app: &mut crate::app::App) {
  app.load_font(LUCIDE_TTF.to_vec());
  app.register_font("lucide", "lucide");
}

#[derive(Clone, Debug, crate::DevtoolsInspectable)]
pub struct DevToolsProps {
  pub snapshot: DevToolsSnapshot,
}

pub struct DevTools {
  selected_path: Signal<Vec<usize>>,
}

impl PartialEq for DevToolsProps {
  fn eq(&self, other: &Self) -> bool {
    self.snapshot == other.snapshot
  }
}

impl Component for DevTools {
  type Props = DevToolsProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      selected_path: ctx.signal(Vec::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let snapshot = ctx.props::<DevToolsProps>().snapshot.clone();
    let selected_path = self.selected_path.get();
    let selected = snapshot.selected_node(&selected_path);

    Column::new()
      .child(top_bar::top_bar(snapshot.node_count()))
      .child(
        Row::new()
          .child(tree_panel::tree_panel(
            &snapshot,
            selected_path,
            self.selected_path.clone(),
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
        .map(|row| row.value.as_str()),
      Some("Padding")
    );
    assert_eq!(
      child_shape
        .iter()
        .find(|row| row.label == "padding")
        .map(|row| row.value.as_str()),
      Some("top 8px, right 8px, bottom 8px, left 8px")
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
          .padding(8.0),
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
