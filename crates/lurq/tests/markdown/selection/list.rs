//! Paragraphs in a `VirtualizedList`, and a paragraph stretched taller than
//! its text by the app's own list view.

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{MarkdownProps, Row, VirtualizedList},
  layout::Alignment,
  markdown::MarkdownBlock,
  node::Element,
};

use super::{super::links::MarkdownRoot, page, paragraph::REPORTED};
use crate::support::{TestSurface, selection::Page};

const OTHER: &str = "A shorter reply with `inline code` and *emphasis*, wrapped over two lines at this width.";

struct ListRoot;

impl Component for ListRoot {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    VirtualizedList::new(ctx, (0..4usize).collect::<Vec<_>>())
      .size(600.0, 900.0)
      .mount_keyed::<MarkdownRoot, _, _, _>(
        |item| *item,
        |item| {
          MarkdownProps::new(if item % 2 == 0 { REPORTED } else { OTHER })
            .selectable(true)
            .width(560.0)
        },
      )
  }
}

#[test]
fn drags_in_and_across_the_rows_of_a_virtualized_list_copy_their_glyphs() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(640, 960);
  tree.mount_root::<ListRoot>(&mut app, ());
  tree.pass(&mut app, &TestSurface);
  let mut page = Page::new(app, tree, 1.0);
  assert_eq!(page.text.matches("Nothing behind").count(), 2, "every row is drawn");

  let (from, to) = page.inks_of("inline code", 0);
  page.drag_inks(from, to);
  assert_eq!(page.selected().as_deref(), Some("inline code"));
  page.assert_highlight_covers(from, to);

  let (from, _) = page.inks_of("The design marks", 1);
  let (_, to) = page.inks_of("emphasis", 1);
  page.drag_inks(from, to);
  assert_eq!(page.selected().as_deref(), Some(&*page.text_between(from, to)));
  page.assert_highlight_covers(from, to);
}

/// The document's one list item, drawn by the app in a row 120 px tall that
/// stretches the paragraph to its height: its text is drawn centered in the
/// paragraph's box, and a selection must meet it there.
#[test]
fn a_drag_over_a_paragraph_stretched_taller_than_its_text_copies_its_glyphs() {
  let props = MarkdownProps::new("- an item with `inline code` in a stretched row").list_view(|list| {
    let MarkdownBlock::Paragraph(inlines) = list.items().first()?.blocks.first()? else {
      return None;
    };
    Some(
      Row::new()
        .height(120.0)
        .align_items(Alignment::Stretch)
        .child(list.inlines(inlines, list.style()))
        .into(),
    )
  });
  let mut page = page(props, 400.0);
  let bounds = page
    .tree
    .find_element(|element| element.text_content().is_some_and(|text| text.starts_with("an item")))
    .expect("the paragraph")
    .bounds();
  assert!(bounds.height > 100.0, "the paragraph is stretched: {bounds:?}");
  assert!(
    page.inks[0].y > bounds.y + 40.0,
    "its text is drawn centered, not at its top"
  );

  let (from, to) = page.inks_of("inline code", 0);
  page.drag_inks(from, to);
  assert_eq!(page.selected().as_deref(), Some("inline code"));
  page.assert_highlight_covers(from, to);
}
