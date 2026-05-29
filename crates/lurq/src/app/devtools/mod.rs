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

#[derive(Clone)]
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
          .child(inspector::inspector_panel(selected, snapshot.frame))
          .width(style::FILL)
          .height(style::FILL),
      )
      .width(style::FILL)
      .height(style::FILL)
      .fill(style::BG)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{app::theme::Theme, components::Column, layout::text_style::FontWeight};

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
  }

  struct SnapshotTestApp;

  impl Component for SnapshotTestApp {
    type Props = ();

    fn create(_ctx: &mut Ctx) -> Self {
      Self
    }

    fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
      Column::new().child(style::text("child", 12.0, FontWeight::Normal, style::TEXT))
    }
  }
}
