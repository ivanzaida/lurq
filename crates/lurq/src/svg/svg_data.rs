use std::sync::{
  Arc,
  atomic::{AtomicU64, Ordering},
};

use crate::node::color::Color;

static NEXT_SVG_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub struct SvgData {
  id: u64,
  tree: Arc<usvg::Tree>,
  overrides: SvgOverrides,
}

#[derive(Clone, Default)]
struct SvgOverrides {
  fill: Option<Color>,
  stroke: Option<Color>,
  opacity: Option<f32>,
}

impl SvgData {
  pub fn from_bytes(data: &[u8]) -> Self {
    let tree = usvg::Tree::from_data(data, &usvg::Options::default()).expect("invalid SVG data");
    Self {
      id: NEXT_SVG_ID.fetch_add(1, Ordering::Relaxed),
      tree: Arc::new(tree),
      overrides: SvgOverrides::default(),
    }
  }

  pub fn from_str(svg: &str) -> Self {
    Self::from_bytes(svg.as_bytes())
  }

  pub fn with_fill(mut self, color: Color) -> Self {
    self.overrides.fill = Some(color);
    self.id = NEXT_SVG_ID.fetch_add(1, Ordering::Relaxed);
    self
  }

  pub fn with_stroke(mut self, color: Color) -> Self {
    self.overrides.stroke = Some(color);
    self.id = NEXT_SVG_ID.fetch_add(1, Ordering::Relaxed);
    self
  }

  pub fn with_opacity(mut self, opacity: f32) -> Self {
    self.overrides.opacity = Some(opacity.clamp(0.0, 1.0));
    self.id = NEXT_SVG_ID.fetch_add(1, Ordering::Relaxed);
    self
  }

  pub fn id(&self) -> u64 {
    self.id
  }

  pub fn tree(&self) -> &usvg::Tree {
    &self.tree
  }

  pub fn fill_override(&self) -> Option<Color> {
    self.overrides.fill
  }

  pub fn stroke_override(&self) -> Option<Color> {
    self.overrides.stroke
  }

  pub fn opacity_override(&self) -> Option<f32> {
    self.overrides.opacity
  }

  pub fn viewbox_width(&self) -> f32 {
    self.tree.size().width()
  }

  pub fn viewbox_height(&self) -> f32 {
    self.tree.size().height()
  }
}
