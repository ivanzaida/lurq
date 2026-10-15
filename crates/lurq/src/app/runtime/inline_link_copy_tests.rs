//! Copying a selection across a Markdown link leaves out what the paragraph
//! draws as presentation: the link's isolate marks and its trailing runs.

use super::{Tree, selected_selectable_text};
use crate::{
  app::{App, component::Component, ctx::Ctx, events::MouseButton},
  components::{Markdown, MarkdownLinkPresentation, MarkdownProps, MarkdownRun},
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

#[test]
fn copied_markdown_text_leaves_out_isolate_marks_and_trailing_runs() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<Root>(
    &mut app,
    MarkdownProps::new("Open [PR 42](https://github.com/acme/pull/42) now, or [mail](mailto:a@b.c).")
      .selectable(true)
      .link_presentation(|link| {
        if link.destination().starts_with("mailto:") {
          MarkdownLinkPresentation::text().trailing([MarkdownRun::plain(format!(" ({})", link.destination()))])
        } else {
          MarkdownLinkPresentation::link().trailing([MarkdownRun::plain(" github.com")])
        }
      })
      .width(900.0),
  );
  tree.pass_headless(&mut app);
  let size = tree.last_layout().expect("laid out").size;
  let y = 8.0;

  tree.mouse_down(1.0, y, MouseButton::Left);
  tree.mouse_move(size.width - 1.0, y);
  tree.mouse_up(size.width - 1.0, y, MouseButton::Left);
  tree.pass_headless(&mut app);

  let root = tree.root.as_ref().expect("root");
  let layout = tree.last_layout.as_ref().expect("layout");
  assert_eq!(
    selected_selectable_text(root, layout).as_deref(),
    Some("Open PR 42 now, or mail.")
  );
}

fn reply(source: &str) -> MarkdownProps {
  MarkdownProps::new(source.to_owned())
    .selectable(true)
    .link_presentation(|_| MarkdownLinkPresentation::link().trailing([MarkdownRun::plain(" github.com")]))
    .width(900.0)
}

fn selected(tree: &Tree) -> Option<String> {
  selected_selectable_text(tree.root.as_ref()?, tree.last_layout.as_ref()?)
}

#[test]
fn a_selection_across_a_link_keeps_its_place_when_a_streamed_reply_grows() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<Root>(&mut app, reply("Open [PR 42](https://github.com/acme/pull/42) now"));
  tree.pass_headless(&mut app);
  let size = tree.last_layout().expect("laid out").size;
  let y = 8.0;
  tree.mouse_down(1.0, y, MouseButton::Left);
  tree.mouse_move(size.width - 1.0, y);
  tree.mouse_up(size.width - 1.0, y, MouseButton::Left);
  tree.pass_headless(&mut app);
  assert_eq!(selected(&tree).as_deref(), Some("Open PR 42 now"));

  // The reply streams on: the paragraph renders again with more text.
  tree.update_root_props::<Root>(reply(
    "Open [PR 42](https://github.com/acme/pull/42) now, and more is coming.",
  ));
  tree.pass_headless(&mut app);
  assert_eq!(
    selected(&tree).as_deref(),
    Some("Open PR 42 now"),
    "the selection covers the same text after the paragraph renders again"
  );
}
