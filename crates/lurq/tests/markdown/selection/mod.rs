//! Selecting and copying Markdown text with the pointer, checked against
//! the glyphs the document draws (`support::selection`): a drag from the
//! left edge of one drawn glyph to the right edge of another copies exactly
//! the text between them, whatever spans, marks and runs the paragraph is
//! made of, and highlights nothing but those glyphs.

mod blocks;
mod extend;
mod inline;
mod list;
mod paragraph;
mod scripts;
mod streaming;
mod wrap;

use lurq::{
  app::{App, Tree},
  components::MarkdownProps,
  node::color::Color,
};

use super::links::MarkdownRoot;
use crate::support::{TestSurface, selection::Page};

/// A selectable document `width` wide, at the display scale `scale`.
pub(super) fn scaled(props: MarkdownProps, width: f32, scale: f32) -> Page {
  let (app, tree) = mount(props, width, scale);
  Page::new(app, tree, scale)
}

/// A selectable document `width` wide whose glyphs drawn in `ignored` are
/// presentation, not its text (a link's trailing runs).
pub(super) fn page_ignoring(props: MarkdownProps, width: f32, ignored: Color) -> Page {
  let (app, tree) = mount(props, width, 1.0);
  Page::ignoring(app, tree, 1.0, [ignored])
}

fn mount(props: MarkdownProps, width: f32, scale: f32) -> (App, Tree) {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_scale_factor(scale);
  tree.resize(((width + 40.0) * scale).ceil() as u32, (1200.0 * scale) as u32);
  tree.mount_root::<MarkdownRoot>(&mut app, props.selectable(true).width(width));
  tree.pass(&mut app, &TestSurface);
  (app, tree)
}

pub(super) fn page(props: MarkdownProps, width: f32) -> Page {
  scaled(props, width, 1.0)
}

/// Renders the document again (its source changed) and reads its glyphs.
pub(super) fn update(page: &mut Page, props: MarkdownProps) {
  let width = page.tree.find_element(|_| true).expect("a document").bounds().width;
  page
    .tree
    .update_root_props::<MarkdownRoot>(props.selectable(true).width(width));
  page.read_glyphs();
}
