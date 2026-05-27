use crate::node::{color::Color, node::Node};

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

  pub fn node_id(&self) -> crate::core::NodeId {
    self.node.node_id()
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
  use crate::{
    components::{Column, Rect, Text},
    node::Element,
  };

  #[test]
  fn element_builders_create_node_tree() {
    let node = Element::from(
      Column::new()
        .spacing(8.0)
        .child(Text::new("hello"))
        .child(Rect::new(10.0, 20.0).rounded(4.0)),
    )
    .node;

    assert_eq!(node.children().len(), 2);
  }
}
