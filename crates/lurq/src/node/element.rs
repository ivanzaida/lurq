#[cfg(feature = "devtools")]
use crate::app::ctx::{
  ComponentContextDebug, ComponentEffectDebug, ComponentMemoDebug, ComponentSignalDebug, DevtoolsInspectableDebug,
};
use crate::node::{color::Color, node::Node};

pub struct Element {
  pub(crate) node: Node,
}

impl Clone for Element {
  fn clone(&self) -> Self {
    Self::from_node(self.node.clone_for_reuse())
  }
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

  pub fn key(mut self, key: impl Into<std::sync::Arc<str>>) -> Self {
    let key = key.into();
    self.node.set_component_key(Some(&key));
    self
  }

  /// HTML-like `id` attribute for `Tree::get_element_by_id` lookup.
  /// Lookup only — never affects reconciliation or state preservation.
  pub fn id(mut self, id: impl Into<std::sync::Arc<str>>) -> Self {
    crate::node::NodeUpdate::id(&mut self.node, id);
    self
  }

  /// Appends an HTML-like class for `Tree::get_elements_by_class_name`
  /// lookup. Lookup only, like [`Element::id`].
  pub fn class(mut self, class: impl Into<std::sync::Arc<str>>) -> Self {
    crate::node::NodeUpdate::class(&mut self.node, class);
    self
  }

  /// Appends several classes at once; see [`Element::class`].
  pub fn classes<C: Into<std::sync::Arc<str>>>(mut self, classes: impl IntoIterator<Item = C>) -> Self {
    crate::node::NodeUpdate::classes(&mut self.node, classes);
    self
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

  pub fn tag_name(&self) -> &'a str {
    self.node.tag_name()
  }

  /// The author-supplied HTML-like `id` attribute, if any.
  pub fn id(&self) -> Option<&'a str> {
    self.node.element_id.as_deref()
  }

  /// The author-supplied HTML-like classes, in insertion order.
  pub fn classes(&self) -> impl Iterator<Item = &'a str> + use<'a> {
    self.node.class_list().iter().map(|class| class.as_ref())
  }

  pub fn has_class(&self, class: &str) -> bool {
    self.node.has_class(class)
  }

  #[allow(dead_code)]
  pub(crate) fn component_key(&self) -> Option<&'a str> {
    self.node.component_key()
  }

  pub fn text_content(&self) -> Option<&'a str> {
    self.node.text_content()
  }

  pub fn color(&self) -> Option<Color> {
    self.node.color()
  }

  #[cfg(feature = "devtools")]
  #[allow(dead_code)]
  pub(crate) fn component_props_debug(&self) -> Option<&DevtoolsInspectableDebug> {
    self.node.component_props_debug()
  }

  #[cfg(feature = "devtools")]
  #[allow(dead_code)]
  pub(crate) fn component_signals_debug(&self) -> &[ComponentSignalDebug] {
    self.node.component_signals_debug()
  }

  #[cfg(feature = "devtools")]
  #[allow(dead_code)]
  pub(crate) fn component_memos_debug(&self) -> &[ComponentMemoDebug] {
    self.node.component_memos_debug()
  }

  #[cfg(feature = "devtools")]
  #[allow(dead_code)]
  pub(crate) fn component_effects_debug(&self) -> &[ComponentEffectDebug] {
    self.node.component_effects_debug()
  }

  #[cfg(feature = "devtools")]
  #[allow(dead_code)]
  pub(crate) fn component_contexts_debug(&self) -> &[ComponentContextDebug] {
    self.node.component_contexts_debug()
  }

  #[cfg(feature = "devtools")]
  #[allow(dead_code)]
  pub(crate) fn debug_attrs(&self) -> &[(std::sync::Arc<str>, std::sync::Arc<str>)] {
    self.node.debug_attrs()
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
