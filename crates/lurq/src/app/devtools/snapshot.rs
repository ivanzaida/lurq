use std::time::Duration;

use crate::{
  app::{profiler::FrameProfile, runtime::Tree},
  core::NodeId,
  node::ElementRef,
};

#[derive(Clone, PartialEq)]
pub struct DevToolsSnapshot {
  pub root: Option<DevToolsNode>,
  pub frame: FrameProfileSnapshot,
}

#[derive(Clone, PartialEq)]
pub struct DevToolsNode {
  pub id: NodeId,
  pub tag: String,
  pub text: Option<String>,
  pub color: Option<String>,
  pub children: Vec<DevToolsNode>,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub struct FrameProfileSnapshot {
  pub total_ms: f32,
  pub layout_ms: f32,
  pub quad_ms: f32,
  pub glyph_ms: f32,
  pub render_ms: f32,
  pub encode_ms: f32,
  pub present_ms: f32,
  pub quad_count: usize,
  pub rect_count: usize,
  pub glyph_count: usize,
  pub memory_kib: f32,
}

impl DevToolsSnapshot {
  pub fn from_tree(tree: &Tree) -> Self {
    Self {
      root: tree.root().map(snapshot_node),
      frame: FrameProfileSnapshot::from_profile(tree.last_profile()),
    }
  }

  pub fn empty() -> Self {
    Self {
      root: None,
      frame: FrameProfileSnapshot::default(),
    }
  }

  pub fn node_count(&self) -> usize {
    self.root.as_ref().map(count_nodes).unwrap_or(0)
  }

  pub(crate) fn selected_node<'a>(&'a self, path: &[usize]) -> Option<&'a DevToolsNode> {
    let mut node = self.root.as_ref()?;
    for index in path {
      node = node.children.get(*index)?;
    }
    Some(node)
  }
}

impl FrameProfileSnapshot {
  pub fn from_profile(profile: &FrameProfile) -> Self {
    Self {
      total_ms: ms(profile.total),
      layout_ms: ms(profile.layout),
      quad_ms: ms(profile.quad_resolve),
      glyph_ms: ms(profile.glyph_rasterize),
      render_ms: ms(profile.gpu_submit),
      encode_ms: ms(profile.render.encode),
      present_ms: ms(profile.render.present),
      quad_count: profile.quad_count,
      rect_count: profile.rect_count,
      glyph_count: profile.glyph_count,
      memory_kib: profile.memory.total_kib(),
    }
  }
}

fn snapshot_node(element: ElementRef<'_>) -> DevToolsNode {
  DevToolsNode {
    id: element.node_id(),
    tag: element.tag_name().to_owned(),
    text: element.text_content().map(str::to_owned),
    color: element.color().map(|color| color.to_hex()),
    children: element.children().into_iter().map(snapshot_node).collect(),
  }
}

fn count_nodes(node: &DevToolsNode) -> usize {
  1 + node.children.iter().map(count_nodes).sum::<usize>()
}

fn ms(duration: Duration) -> f32 {
  duration.as_secs_f32() * 1000.0
}
