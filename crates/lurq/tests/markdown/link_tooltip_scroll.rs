//! A Markdown link's tooltip in a scrolling conversation under a page header:
//! scrolled with the pointer still, it never stays over the header.

use std::time::Duration;

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::ScrollPhase, theme::ThemeMarkdown},
  components::{Column, Markdown, MarkdownLinkPresentation, MarkdownProps, Rect, ScrollVertical},
  layout::quad::{Quad, QuadContent},
  node::Element,
};

use super::links::{bounds_of, pass, quads};
use crate::support::TestSurface;

const WORDS: &str = "Pipelines · opens in this tab";
const HEADER_HEIGHT: f32 = 40.0;

/// A header, and under it the conversation: a reply with a link, between
/// earlier and later messages.
struct Conversation;

impl Component for Conversation {
  type Props = MarkdownProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let reply = Markdown::mount(ctx, ctx.props::<Self::Props>().clone());
    let messages = Column::new()
      .child(Rect::new(400.0, 60.0).background("#111827"))
      .child(reply)
      .child(Rect::new(400.0, 600.0).background("#111827"));
    Column::new()
      .child(Rect::new(400.0, HEADER_HEIGHT).background("#1f2937"))
      .child(ScrollVertical::new(messages).size(400.0, 200.0))
  }
}

fn mount() -> (App, Tree) {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<Conversation>(
    &mut app,
    MarkdownProps::new("Open [Pipelines](/pipelines) to see them.")
      .theme(ThemeMarkdown {
        link_tooltip_delay: Duration::ZERO,
        ..ThemeMarkdown::default()
      })
      .link_presentation(|_| MarkdownLinkPresentation::link().tooltip(WORDS))
      .width(400.0),
  );
  tree.pass(&mut app, &TestSurface);
  (app, tree)
}

fn tooltip(tree: &Tree) -> Option<Quad> {
  quads(tree)
    .into_iter()
    .find(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == WORDS))
}

/// Scrolls the conversation down by `dy` with the wheel at `pointer`, which
/// holds still, and draws a frame.
fn scroll_by(app: &mut App, tree: &mut Tree, pointer: (f32, f32), dy: f32) {
  tree.scroll(pointer.0, pointer.1, 0.0, -dy, ScrollPhase::Scroll);
  pass(app, tree);
}

#[test]
fn markdown_link_tooltip_goes_when_scrolling_moves_the_link_from_under_the_still_pointer() {
  let (mut app, mut tree) = mount();
  let link = bounds_of(&mut tree, "Pipelines");
  let pointer = (link.x + 2.0, link.y + 2.0);
  tree.mouse_move(pointer.0, pointer.1);
  pass(&mut app, &mut tree);
  pass(&mut app, &mut tree);
  assert!(tooltip(&tree).is_some(), "resting on the link shows its tooltip");

  scroll_by(&mut app, &mut tree, pointer, 50.0);

  let link = bounds_of(&mut tree, "Pipelines");
  assert!(
    link.y + link.height <= pointer.1,
    "the link moved from under the pointer: {link:?}"
  );
  assert!(tooltip(&tree).is_none(), "its tooltip went with it");
}

#[test]
fn markdown_link_tooltip_for_keyboard_focus_is_not_drawn_over_the_header_the_link_scrolled_under() {
  let (mut app, mut tree) = mount();
  tree.key_down("Tab".into(), "Tab".into(), false, false, false);
  tree.key_up("Tab".into(), "Tab".into(), false, false, false);
  pass(&mut app, &mut tree);
  pass(&mut app, &mut tree);
  assert!(tooltip(&tree).is_some(), "keyboard focus shows the tooltip");

  let away = (300.0, 220.0);
  scroll_by(&mut app, &mut tree, away, 100.0);
  let link = bounds_of(&mut tree, "Pipelines");
  assert!(
    link.y + link.height <= HEADER_HEIGHT,
    "the link is under the header: {link:?}"
  );
  assert!(tooltip(&tree).is_none(), "the tooltip is not drawn over the header");

  scroll_by(&mut app, &mut tree, away, -100.0);
  let shown = tooltip(&tree).expect("back with the link");
  assert!(shown.y >= HEADER_HEIGHT, "below the header: {}", shown.y);
}
