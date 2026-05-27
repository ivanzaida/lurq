use crate::{impl_into_node, layout::layout_kind::FrameConstraints, node::dimension::Dimension};

impl_into_node!(Rect);

impl Rect {
  pub fn new(width: impl Into<Dimension>, height: impl Into<Dimension>) -> Self {
    Self::from_node(crate::node::Node::new().frame(FrameConstraints {
      width: Some(width.into()),
      height: Some(height.into()),
      ..FrameConstraints::default()
    }))
  }
}

impl Default for Rect {
  fn default() -> Self {
    Self::new(Dimension::Auto, Dimension::Auto)
  }
}
