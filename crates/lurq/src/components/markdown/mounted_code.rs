//! A code block of a document mounted on its own.
//!
//! A long document's code blocks are most of what it builds: each is a bar with a copy button, a room that scrolls
//! and its lines. Mounted each as a component of its own, keyed by its place, a block whose code, language and
//! highlights did not change is not built again when the document is: a document that grows at its end, as a
//! streamed answer does, builds only what changed in it.

use std::sync::Arc;

use crate::{
  app::{component::Component, ctx::Ctx},
  node::{Element, text_highlight::TextHighlight},
  ui::CodeBlock,
};

#[derive(Clone, PartialEq)]
pub(super) struct MountedCodeProps {
  pub(super) code: Arc<str>,
  pub(super) language: Option<Arc<str>>,
  /// The document fits its content: the block is as wide as its longest line.
  pub(super) fit: bool,
  /// The document's words are selectable.
  pub(super) selectable: bool,
  /// The document's highlights over the code, in bytes of the code.
  pub(super) highlights: Arc<[TextHighlight]>,
}

impl crate::app::component::DevtoolsInspectable for MountedCodeProps {}

/// What mounts a document's code blocks, each on its own: the component's context, behind a cell, as a trait so
/// that what holds it (the render's context) shrinks to where it is used.
pub(super) trait MountCode {
  fn mount_code(&self, key: &str, props: MountedCodeProps) -> Element;
}

impl MountCode for std::cell::RefCell<&mut Ctx> {
  fn mount_code(&self, key: &str, props: MountedCodeProps) -> Element {
    self.borrow_mut().mount_keyed::<MountedCode>(key, props)
  }
}

pub(super) struct MountedCode;

impl Component for MountedCode {
  type Props = MountedCodeProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<MountedCodeProps>().clone();
    let mut block = CodeBlock::new(&props.code).selectable(false);
    if let Some(language) = &props.language {
      block = block.language(language);
    }
    // A long block lays out the lines in the window alone, as a block of its own: its lines are not one text the
    // document's highlights and selection run through, so it is selectable by itself, and as wide as the document.
    if props.code.bytes().filter(|byte| *byte == b'\n').count() >= crate::ui::CODE_BLOCK_WINDOWED_LINES {
      return block.selectable(props.selectable).mount(ctx);
    }
    let mut element = block.build(ctx.theme());
    if !props.highlights.is_empty()
      && let Some(node) = super::code_text(&mut element.node, &props.code)
    {
      node.set_text_highlights(props.highlights.clone());
    }
    if props.fit {
      super::fit(&mut element.node, 3);
    }
    if props.selectable {
      element.node.selectable_recursive(true);
    }
    element
  }
}
