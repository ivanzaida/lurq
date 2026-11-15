//! A code block of a document mounted on its own, built as its light stand-in while it is out of view.
//!
//! A long document's code blocks are most of what it builds: each is a bar with a copy button, a room that scrolls
//! and its lines. Out of the window's view, a block is built as `ui::CodeBlock`'s stand-in of itself, of its size,
//! with its code and its bar's name; the frame it comes near the view in (before that frame is painted), it builds
//! the block itself, and only itself: the document is not built again.

use std::sync::Arc;

use crate::{
  app::{component::Component, ctx::Ctx},
  core::{ElementRef, Signal},
  node::{Element, NodeUpdate, text_highlight::TextHighlight},
  ui::CodeBlock,
};

/// How far out of the window's view a block is built as itself already: a block scrolled in is there in full.
const AHEAD: f32 = 600.0;

#[derive(Clone, PartialEq)]
pub(super) struct LazyCodeProps {
  pub(super) code: Arc<str>,
  pub(super) language: Option<Arc<str>>,
  /// The document fits its content: the block is as wide as its longest line.
  pub(super) fit: bool,
  /// The document's words are selectable.
  pub(super) selectable: bool,
  /// The document's highlights over the code, in bytes of the code.
  pub(super) highlights: Arc<[TextHighlight]>,
}

impl crate::app::component::DevtoolsInspectable for LazyCodeProps {}

/// What mounts a document's code blocks, each on its own: the component's context, behind a cell, as a trait so
/// that what holds it (the render's context) shrinks to where it is used.
pub(super) trait MountCode {
  fn mount_code(&self, key: &str, props: LazyCodeProps) -> Element;
}

impl MountCode for std::cell::RefCell<&mut Ctx> {
  fn mount_code(&self, key: &str, props: LazyCodeProps) -> Element {
    self.borrow_mut().mount_keyed::<LazyCode>(key, props)
  }
}

pub(super) struct LazyCode {
  /// Whether it has come near the view: from then on it is the block itself.
  shown: Signal<bool>,
  reference: ElementRef,
}

impl Component for LazyCode {
  type Props = LazyCodeProps;

  fn create(ctx: &mut Ctx) -> Self {
    let shown = ctx.signal(false);
    let reference = ElementRef::new();
    match ctx.window_state().cloned() {
      Some(window) => {
        let near = shown.clone();
        reference.observe_rect_before_paint(move |rect| {
          if near.get_untracked() {
            return;
          }
          let size = window.info().logical_size();
          let (width, height) = (size.width, size.height);
          let in_reach = rect.y + rect.height > -AHEAD
            && rect.y < height + AHEAD
            && rect.x + rect.width > -AHEAD
            && rect.x < width + AHEAD;
          if in_reach {
            near.set(true);
          }
        });
      }
      // With no window to be out of, the block is itself.
      None => shown.set(true),
    }
    Self { shown, reference }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<LazyCodeProps>().clone();
    let shown = self.shown.get();
    let mut block = CodeBlock::new(&props.code).selectable(false).light(!shown);
    if let Some(language) = &props.language {
      block = block.language(language);
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
    NodeUpdate::ref_element(&mut *element.node, self.reference.clone());
    element
  }
}
