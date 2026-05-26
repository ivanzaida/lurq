use crate::{
  app::events::{KeyboardEvent, MouseEvent, ScrollEvent},
  core::NodeRef,
  layout::{
    layout_kind::{FrameConstraints, Justify, ScrollState},
    scrollbar::ScrollBarStyle,
    text_style::TextStyle,
    Alignment, StackAlignment,
  },
  node::{
    border::{Border, BorderRadius},
    color::Color,
    dimension::Dimension,
    interaction_state::InteractionState,
    node::Node,
    padding::Padding,
  },
};

pub struct Element {
  pub(crate) node: Node,
}

#[derive(Clone, Copy)]
pub struct ElementRef<'a> {
  pub(crate) node: &'a Node,
}

pub struct ElementChildren<'a> {
  nodes: &'a [Node],
}

pub struct ElementIter<'a> {
  inner: std::slice::Iter<'a, Node>,
}

impl Element {
  pub(crate) fn from_node(node: Node) -> Self {
    Self { node }
  }

  pub fn new() -> Self {
    Self { node: Node::new() }
  }

  pub fn row() -> Self {
    Self {
      node: Node::row(0.0, Alignment::Start, vec![]),
    }
  }

  pub fn row_with(spacing: f32, align: Alignment, children: Vec<Element>) -> Self {
    Self {
      node: Node::row(spacing, align, children.into_iter().map(|child| child.node).collect()),
    }
  }

  pub fn column() -> Self {
    Self {
      node: Node::column(0.0, Alignment::Start, vec![]),
    }
  }

  pub fn column_with(spacing: f32, align: Alignment, children: Vec<Element>) -> Self {
    Self {
      node: Node::column(spacing, align, children.into_iter().map(|child| child.node).collect()),
    }
  }

  pub fn stack() -> Self {
    Self {
      node: Node::stack(StackAlignment::TopStart, vec![]),
    }
  }

  pub fn stack_with(align: StackAlignment, children: Vec<Element>) -> Self {
    Self {
      node: Node::stack(align, children.into_iter().map(|child| child.node).collect()),
    }
  }

  pub fn text(content: &str) -> Self {
    Self {
      node: Node::text(content),
    }
  }

  pub fn styled_text(content: &str, style: TextStyle) -> Self {
    Self {
      node: Node::text_styled(content, style),
    }
  }

  pub fn rect(width: f32, height: f32) -> Self {
    Self::new().frame(FrameConstraints {
      width: Some(width),
      height: Some(height),
      ..Default::default()
    })
  }

  pub fn spacer() -> Self {
    Self::new()
  }

  pub fn scroll_vertical(child: impl Into<Element>) -> Self {
    Self::from_node(crate::node::dsl::scroll_vertical(child.into().node))
  }

  pub fn scroll_horizontal(child: impl Into<Element>) -> Self {
    Self::from_node(crate::node::dsl::scroll_horizontal(child.into().node))
  }

  pub fn scroll_both(child: impl Into<Element>) -> Self {
    Self::from_node(crate::node::dsl::scroll_both(child.into().node))
  }

  pub fn child(mut self, child: impl Into<Element>) -> Self {
    self.node = self.node.child(child.into().node);
    self
  }

  pub fn with_children(mut self, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.node = self
      .node
      .with_children(children.into_iter().map(|child| child.into().node));
    self
  }

  pub fn node_id(&self) -> crate::core::NodeId {
    self.node.node_id()
  }

  pub fn spacing(mut self, spacing: f32) -> Self {
    self.node = self.node.spacing(spacing);
    self
  }

  pub fn align_items(mut self, align: Alignment) -> Self {
    self.node = self.node.align_items(align);
    self
  }

  pub fn justify(mut self, justify: Justify) -> Self {
    self.node = self.node.justify(justify);
    self
  }

  pub fn wrap(mut self) -> Self {
    self.node = self.node.wrap();
    self
  }

  pub fn stack_align(mut self, align: StackAlignment) -> Self {
    self.node = self.node.stack_align(align);
    self
  }

  pub fn fill(mut self, hex: &str) -> Self {
    self.node = self.node.fill(hex);
    self
  }

  pub fn size(mut self, width: f32, height: f32) -> Self {
    self.node = self.node.size(width, height);
    self
  }

  pub fn width(mut self, width: f32) -> Self {
    self.node = self.node.width(width);
    self
  }

  pub fn height(mut self, height: f32) -> Self {
    self.node = self.node.height(height);
    self
  }

  pub fn pad(mut self, all: f32) -> Self {
    self.node = self.node.pad(all);
    self
  }

  pub fn pad_xy(mut self, horizontal: f32, vertical: f32) -> Self {
    self.node = self.node.pad_xy(horizontal, vertical);
    self
  }

  pub fn pad_left(mut self, val: f32) -> Self {
    self.node = self.node.pad_left(val);
    self
  }

  pub fn pad_right(mut self, val: f32) -> Self {
    self.node = self.node.pad_right(val);
    self
  }

  pub fn pad_top(mut self, val: f32) -> Self {
    self.node = self.node.pad_top(val);
    self
  }

  pub fn pad_bottom(mut self, val: f32) -> Self {
    self.node = self.node.pad_bottom(val);
    self
  }

  pub fn padding(mut self, padding: Padding) -> Self {
    self.node = self.node.padding(padding);
    self
  }

  pub fn frame(mut self, frame: FrameConstraints) -> Self {
    self.node = self.node.frame(frame);
    self
  }

  pub fn offset(mut self, x: f32, y: f32) -> Self {
    self.node = self.node.offset(x, y);
    self
  }

  pub fn align(mut self, alignment: Alignment) -> Self {
    self.node = self.node.align(alignment);
    self
  }

  pub fn flex(mut self, factor: f32) -> Self {
    self.node = self.node.flex(factor);
    self
  }

  pub fn flex_shrink(mut self, factor: f32) -> Self {
    self.node = self.node.flex_shrink(factor);
    self
  }

  pub fn flex_full(mut self, grow: f32, shrink: f32, basis: Option<f32>) -> Self {
    self.node = self.node.flex_full(grow, shrink, basis);
    self
  }

  pub fn background(mut self, color: Color) -> Self {
    self.node = self.node.background(color);
    self
  }

  pub fn corner_radius(mut self, radius: BorderRadius) -> Self {
    self.node = self.node.corner_radius(radius);
    self
  }

  pub fn rounded(mut self, radius: f32) -> Self {
    self.node = self.node.rounded(radius);
    self
  }

  pub fn border_inside(mut self, width: f32, color: Color) -> Self {
    self.node = self.node.border_inside(width, color);
    self
  }

  pub fn border_outside(mut self, width: f32, color: Color) -> Self {
    self.node = self.node.border_outside(width, color);
    self
  }

  pub fn border_center(mut self, width: f32, color: Color) -> Self {
    self.node = self.node.border_center(width, color);
    self
  }

  pub fn border_custom(mut self, border: Border) -> Self {
    self.node = self.node.border_custom(border);
    self
  }

  pub fn on_click(mut self, f: impl Fn(&MouseEvent) + Send + Sync + 'static) -> Self {
    self.node = self.node.on_click(f);
    self
  }

  pub fn on_dblclick(mut self, f: impl Fn(&MouseEvent) + Send + Sync + 'static) -> Self {
    self.node = self.node.on_dblclick(f);
    self
  }

  pub fn on_mouse_down(mut self, f: impl Fn(&MouseEvent) + Send + Sync + 'static) -> Self {
    self.node = self.node.on_mouse_down(f);
    self
  }

  pub fn on_mouse_up(mut self, f: impl Fn(&MouseEvent) + Send + Sync + 'static) -> Self {
    self.node = self.node.on_mouse_up(f);
    self
  }

  pub fn on_mouse_move(mut self, f: impl Fn(&MouseEvent) + Send + Sync + 'static) -> Self {
    self.node = self.node.on_mouse_move(f);
    self
  }

  pub fn on_mouse_enter(mut self, f: impl Fn() + Send + Sync + 'static) -> Self {
    self.node = self.node.on_mouse_enter(f);
    self
  }

  pub fn on_mouse_leave(mut self, f: impl Fn() + Send + Sync + 'static) -> Self {
    self.node = self.node.on_mouse_leave(f);
    self
  }

  pub fn on_key_down(mut self, f: impl Fn(&KeyboardEvent) + Send + Sync + 'static) -> Self {
    self.node = self.node.on_key_down(f);
    self
  }

  pub fn on_key_up(mut self, f: impl Fn(&KeyboardEvent) + Send + Sync + 'static) -> Self {
    self.node = self.node.on_key_up(f);
    self
  }

  pub fn on_focus(mut self, f: impl Fn() + Send + Sync + 'static) -> Self {
    self.node = self.node.on_focus(f);
    self
  }

  pub fn on_blur(mut self, f: impl Fn() + Send + Sync + 'static) -> Self {
    self.node = self.node.on_blur(f);
    self
  }

  pub fn on_scroll(mut self, f: impl Fn(&ScrollEvent) + Send + Sync + 'static) -> Self {
    self.node = self.node.on_scroll(f);
    self
  }

  pub fn on_scroll_start(mut self, f: impl Fn(&ScrollEvent) + Send + Sync + 'static) -> Self {
    self.node = self.node.on_scroll_start(f);
    self
  }

  pub fn on_scroll_end(mut self, f: impl Fn(&ScrollEvent) + Send + Sync + 'static) -> Self {
    self.node = self.node.on_scroll_end(f);
    self
  }

  pub fn scrollbar(mut self, style: ScrollBarStyle) -> Self {
    self.node = self.node.scrollbar(style);
    self
  }

  pub fn ref_node(mut self, node_ref: NodeRef) -> Self {
    self.node = self.node.ref_node(node_ref);
    self
  }

  pub fn interactive(mut self, state: InteractionState) -> Self {
    self.node = self.node.interactive(state);
    self
  }

  pub fn clip(mut self) -> Self {
    self.node = self.node.clip();
    self
  }

  pub fn intrinsic(mut self, width: f32, height: f32) -> Self {
    self.node = self.node.intrinsic(width, height);
    self
  }

  pub fn with_scroll_state(mut self, existing: ScrollState) -> Self {
    self.node = self.node.with_scroll_state(existing);
    self
  }

  pub fn pad_dimension(mut self, padding: Padding) -> Self {
    self.node = self.node.padding(padding);
    self
  }

  pub fn pad_dimension_all(self, all: Dimension) -> Self {
    self.padding(Padding::all(all))
  }
}

impl Default for Element {
  fn default() -> Self {
    Self::new()
  }
}

impl<'a> ElementRef<'a> {
  pub(crate) fn new(node: &'a Node) -> Self {
    Self { node }
  }

  pub fn node_id(&self) -> crate::core::NodeId {
    self.node.node_id()
  }

  pub fn text_content(&self) -> Option<&'a str> {
    self.node.text_content()
  }

  pub fn color(&self) -> Option<Color> {
    self.node.color()
  }

  pub fn children(&self) -> ElementChildren<'a> {
    ElementChildren {
      nodes: self.node.children(),
    }
  }
}

impl<'a> ElementChildren<'a> {
  pub fn len(&self) -> usize {
    self.nodes.len()
  }

  pub fn is_empty(&self) -> bool {
    self.nodes.is_empty()
  }

  pub fn iter(&self) -> ElementIter<'a> {
    ElementIter {
      inner: self.nodes.iter(),
    }
  }
}

impl<'a> IntoIterator for ElementChildren<'a> {
  type Item = ElementRef<'a>;
  type IntoIter = ElementIter<'a>;

  fn into_iter(self) -> Self::IntoIter {
    ElementIter {
      inner: self.nodes.iter(),
    }
  }
}

impl<'a> Iterator for ElementIter<'a> {
  type Item = ElementRef<'a>;

  fn next(&mut self) -> Option<Self::Item> {
    self.inner.next().map(ElementRef::new)
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    self.inner.size_hint()
  }
}

impl ExactSizeIterator for ElementIter<'_> {}

#[cfg(test)]
mod tests {
  use crate::node::Element;

  #[test]
  fn element_builders_create_node_tree() {
    let node = Element::column()
      .spacing(8.0)
      .child(Element::text("hello"))
      .child(Element::rect(10.0, 20.0).rounded(4.0))
      .node;

    assert_eq!(node.children().len(), 2);
  }
}
