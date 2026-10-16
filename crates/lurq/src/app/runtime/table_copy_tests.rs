//! Copying a selection across a Markdown table: the cells of a row are one
//! line, in column order, however tall each cell is and wherever its words
//! sit in it. Paragraphs, list items and wrapped text copy as before.

use super::{Tree, selected_selectable_text};
use crate::{
  app::{App, component::Component, ctx::Ctx, events::MouseButton},
  components::{Markdown, MarkdownProps},
  node::Element,
};

struct Root;

impl Component for Root {
  type Props = MarkdownProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    Markdown::mount(ctx, ctx.props::<Self::Props>().clone())
  }
}

/// A cell's words that wrap onto several lines at the tests' width.
const WRAPPING: &str = "two two two two two two two two two two two two two two two two two two two two";

/// What copying `source`, laid out `width` wide, gives after a drag from the
/// start of the text `first` to the end of the text `last`. The drag starts
/// and ends inside the words: one past a centered cell's words would select
/// from the nearest text instead.
fn copy(source: &str, width: f32, first: &str, last: &str) -> Option<String> {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<Root>(&mut app, MarkdownProps::new(source).selectable(true).width(width));
  tree.pass_headless(&mut app);
  let bounds = |tree: &mut Tree, text: &str| {
    tree
      .find_element(|element| element.text_content() == Some(text))
      .unwrap_or_else(|| panic!("no text {text:?}"))
      .bounds()
  };
  let (first, last) = (bounds(&mut tree, first), bounds(&mut tree, last));
  let (end_x, end_y) = (last.x + last.width - 1.0, last.y + last.height - 2.0);
  tree.mouse_down(first.x + 1.0, first.y + 2.0, MouseButton::Left);
  tree.mouse_move(end_x, end_y);
  tree.mouse_up(end_x, end_y, MouseButton::Left);
  tree.pass_headless(&mut app);
  selected_selectable_text(tree.root.as_ref()?, tree.last_layout.as_ref()?)
}

#[test]
fn a_short_cell_before_a_wrapping_cell_copies_in_column_order() {
  assert_eq!(
    copy(
      &format!("| a | b |\n|---|---|\n| one | {WRAPPING} |"),
      300.0,
      "a",
      WRAPPING
    )
    .as_deref(),
    Some(format!("a b\none {WRAPPING}").as_str())
  );
}

#[test]
fn a_wrapping_cell_before_a_short_cell_copies_in_column_order() {
  assert_eq!(
    copy(
      &format!("| a | b |\n|---|---|\n| {WRAPPING} | one |"),
      300.0,
      "a",
      "one"
    )
    .as_deref(),
    Some(format!("a b\n{WRAPPING} one").as_str())
  );
}

#[test]
fn a_header_row_with_a_wrapping_header_copies_as_one_line() {
  assert_eq!(
    copy(
      &format!("| a | {WRAPPING} |\n|---|---|\n| one | three |"),
      300.0,
      "a",
      "three"
    )
    .as_deref(),
    Some(format!("a {WRAPPING}\none three").as_str())
  );
}

#[test]
fn centered_cells_of_different_heights_copy_in_column_order() {
  assert_eq!(
    copy(
      &format!("| a | b | c |\n|:-:|:-:|:-:|\n| one | {WRAPPING} | three |"),
      360.0,
      "a",
      "three",
    )
    .as_deref(),
    Some(format!("a b c\none {WRAPPING} three").as_str())
  );
}

#[test]
fn paragraphs_list_items_and_wrapped_text_copy_one_block_per_line() {
  assert_eq!(
    copy(
      &format!("First paragraph.\n\n{WRAPPING}\n\n- an item\n- {WRAPPING}\n- last item"),
      300.0,
      "First paragraph.",
      "last item",
    )
    .as_deref(),
    Some(format!("First paragraph.\n{WRAPPING}\n- an item\n- {WRAPPING}\n- last item").as_str())
  );
}
