//! Markdown links and input: Tab order, Enter, focus looks and ring,
//! tooltips, and telling a person's press from synthetic input.

use std::{
  sync::{Arc, Mutex},
  time::{Duration, Instant},
};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::Ctx,
    events::{MouseButton, is_synthetic_input, with_synthetic_input},
    theme::{MarkdownFocusRing, MarkdownLinkState, ThemeMarkdown},
  },
  components::{Column, Markdown, MarkdownLinkPresentation, MarkdownProps},
  layout::quad::{Quad, QuadContent},
  node::{Element, color::Color},
};

use super::links::{bounds_of, colored, mount, pass, quads, rich_text_quads, span};
use crate::support::pointer_click;

const RING: &str = "#ff00ff";

/// The document inside a padded column, away from the window's edge.
struct PaddedRoot;

impl Component for PaddedRoot {
  type Props = MarkdownProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    Column::new().padding(20.0).child(Markdown::mount(ctx, props))
  }
}

fn mount_padded(props: MarkdownProps) -> (App, Tree) {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<PaddedRoot>(&mut app, props);
  tree.pass(&mut app, &crate::support::TestSurface);
  (app, tree)
}

fn key(tree: &mut Tree, key: &str, code: &str, shift: bool) {
  tree.key_down(key.into(), code.into(), shift, false, false);
  tree.key_up(key.into(), code.into(), shift, false, false);
}

fn tab(tree: &mut Tree) {
  key(tree, "Tab", "Tab", false);
}

fn focused_text(tree: &Tree) -> Option<String> {
  tree
    .focused_element()
    .and_then(|element| element.text_content().map(str::to_owned))
}

fn rings(quads: &[Quad]) -> Vec<&Quad> {
  quads
    .iter()
    .filter(|quad| {
      quad
        .border
        .and_then(|border| border.top)
        .is_some_and(|top| top.color == Color::from_hex(RING))
    })
    .collect()
}

fn tooltip_shown(quads: &[Quad], words: &str) -> bool {
  quads
    .iter()
    .any(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == words))
}

fn theme(delay: Duration) -> ThemeMarkdown {
  let mut theme = ThemeMarkdown::default();
  theme.link_focus_ring = Some(MarkdownFocusRing::new(2.0, Color::from_hex(RING)).offset(1.0));
  theme.link_tooltip_delay = delay;
  theme
}

/// Records each followed link's index and destination, and whether the input
/// that followed it was synthetic.
fn recorder() -> (
  Arc<Mutex<Vec<(usize, String, bool)>>>,
  impl Fn(&lurq::components::MarkdownLink) + Send + Sync,
) {
  let followed = Arc::new(Mutex::new(Vec::new()));
  let record = {
    let followed = followed.clone();
    move |link: &lurq::components::MarkdownLink| {
      followed
        .lock()
        .unwrap()
        .push((link.index(), link.destination().to_owned(), is_synthetic_input()))
    }
  };
  (followed, record)
}

#[test]
fn markdown_links_are_tab_stops_in_document_order_and_enter_follows_them() {
  let (followed, record) = recorder();
  let (mut app, mut tree) = mount(
    MarkdownProps::new("First [one](/1) and [two](/2).\n\n- an item with [three](/3)\n\n| a |\n|---|\n| [four](/4) |")
      .theme(theme(Duration::ZERO))
      .on_link_click(record)
      .width(500.0),
  );

  let mut order = Vec::new();
  for _ in 0..5 {
    tab(&mut tree);
    pass(&mut app, &mut tree);
    order.push(focused_text(&tree).unwrap_or_default());
  }
  assert_eq!(order, ["one", "two", "three", "four", "one"]);

  tab(&mut tree);
  key(&mut tree, "Enter", "Enter", false);
  assert_eq!(*followed.lock().unwrap(), vec![(1, "/2".to_owned(), false)]);

  key(&mut tree, " ", "Space", false);
  assert_eq!(followed.lock().unwrap().len(), 1, "Space does not follow a link");

  key(&mut tree, "Tab", "Tab", true);
  pass(&mut app, &mut tree);
  assert_eq!(focused_text(&tree).as_deref(), Some("one"), "Shift+Tab goes back");
}

#[test]
fn markdown_link_keyboard_focus_shows_the_focused_look_and_a_ring_outside_the_words() {
  let focused = MarkdownLinkState::new()
    .color(Color::from_hex("#0000ee"))
    .background(Color::from_hex("#dddddd"));
  let (mut app, mut tree) = mount_padded(
    MarkdownProps::new("[Docs](/docs) at the very start.")
      .theme(theme(Duration::ZERO))
      .link_presentation(move |_| {
        MarkdownLinkPresentation::link()
          .style(colored("#00aa00"))
          .focused(focused)
      })
      .width(400.0),
  );
  let link = bounds_of(&mut tree, "Docs");
  assert!(rings(&quads(&tree)).is_empty());

  tab(&mut tree);
  pass(&mut app, &mut tree);
  let focused_quads = quads(&tree);
  assert_eq!(
    span(rich_text_quads(&focused_quads)[0], "Docs").style.color,
    Color::from_hex("#0000ee")
  );
  assert_eq!(
    super::links::rects_of(&focused_quads, Color::from_hex("#dddddd")).len(),
    1
  );
  let ring = rings(&focused_quads);
  assert_eq!(ring.len(), 1);
  let ring = ring[0];
  // 1 px gap and a 2 px line on every side of the words.
  assert!((ring.x - (link.x - 3.0)).abs() < 0.01, "{} vs {}", ring.x, link.x);
  assert!((ring.y - (link.y - 3.0)).abs() < 0.01);
  assert!((ring.width - (link.width + 6.0)).abs() < 0.01);
  assert!((ring.height - (link.height + 6.0)).abs() < 0.01);
  let paragraph = focused_quads
    .iter()
    .find(|quad| matches!(quad.content, QuadContent::RichText { .. }))
    .unwrap();
  assert!(
    ring.x < paragraph.x && ring.y < paragraph.y,
    "the ring reaches past the paragraph's box"
  );
  assert!(
    !ring.clip.active || (ring.clip.x <= ring.x && ring.clip.y <= ring.y),
    "the ring is not clipped to the paragraph: ring {} {} clip {} {} {} {} {}",
    ring.x,
    ring.y,
    ring.clip.x,
    ring.clip.y,
    ring.clip.width,
    ring.clip.height,
    ring.clip.active
  );
  let text_order = focused_quads
    .iter()
    .position(|quad| std::ptr::eq(quad, paragraph))
    .unwrap();
  let ring_order = focused_quads.iter().position(|quad| std::ptr::eq(quad, ring)).unwrap();
  assert!(ring_order > text_order, "the ring is drawn over the text");

  // A pointer press focuses the link without a ring.
  pointer_click(&mut tree, link.x + 2.0, link.y + 2.0, MouseButton::Left);
  pass(&mut app, &mut tree);
  assert_eq!(focused_text(&tree).as_deref(), Some("Docs"));
  assert!(rings(&quads(&tree)).is_empty());
}

#[test]
fn markdown_link_tooltip_shows_after_the_pointer_rests_and_at_once_on_keyboard_focus() {
  let (mut app, mut tree) = mount(
    MarkdownProps::new("See [KON-1](/tasks/1) for details.")
      .theme(theme(Duration::from_millis(500)))
      .link_presentation(|link| MarkdownLinkPresentation::link().tooltip(format!("Opens {}", link.destination())))
      .width(400.0),
  );
  let link = bounds_of(&mut tree, "KON-1");

  tree.mouse_move(link.x + 2.0, link.y + 2.0);
  pass(&mut app, &mut tree);
  assert!(!tooltip_shown(&quads(&tree), "Opens /tasks/1"), "not before the delay");

  tree.tick_timers_at(Instant::now() + Duration::from_millis(600));
  pass(&mut app, &mut tree);
  pass(&mut app, &mut tree);
  let shown = quads(&tree);
  assert!(tooltip_shown(&shown, "Opens /tasks/1"));
  let words = shown
    .iter()
    .find(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == "Opens /tasks/1"))
    .unwrap();
  assert!(words.y >= link.y + link.height, "the tooltip is under the link");

  tree.mouse_move(link.x + 300.0, link.y + 2.0);
  pass(&mut app, &mut tree);
  pass(&mut app, &mut tree);
  assert!(
    !tooltip_shown(&quads(&tree), "Opens /tasks/1"),
    "leaving the link hides it"
  );

  tab(&mut tree);
  pass(&mut app, &mut tree);
  pass(&mut app, &mut tree);
  assert!(
    tooltip_shown(&quads(&tree), "Opens /tasks/1"),
    "keyboard focus shows it at once"
  );
}

#[test]
fn markdown_link_click_tells_a_person_from_synthetic_input() {
  let (followed, record) = recorder();
  let (mut app, mut tree) = mount(
    MarkdownProps::new("Open [the page](https://example.test/page) or [this](/here).")
      .theme(theme(Duration::ZERO))
      .on_link_click(record)
      .width(500.0),
  );
  let page = bounds_of(&mut tree, "the page");
  let here = bounds_of(&mut tree, "this");

  pointer_click(&mut tree, page.x + 3.0, page.y + 3.0, MouseButton::Left);
  with_synthetic_input(|| pointer_click(&mut tree, here.x + 3.0, here.y + 3.0, MouseButton::Left));
  tab(&mut tree);
  pass(&mut app, &mut tree);
  with_synthetic_input(|| key(&mut tree, "Enter", "Enter", false));

  assert_eq!(
    *followed.lock().unwrap(),
    vec![
      (0, "https://example.test/page".to_owned(), false),
      (1, "/here".to_owned(), true),
      // The press focused "this"; Tab moved on to the next stop, "the page".
      (0, "https://example.test/page".to_owned(), true),
    ]
  );
}

#[test]
fn markdown_wrapped_link_is_hit_on_its_lines_only() {
  let (followed, record) = recorder();
  let (_app, mut tree) = mount(
    MarkdownProps::new("Some words before [a link whose words wrap onto the next line](/wrapped) after.")
      .theme(theme(Duration::ZERO))
      .on_link_click(record)
      .width(180.0),
  );
  let first_line = bounds_of(&mut tree, "a link whose words wrap onto the next line");
  // Left of the link on its first line: the words before it.
  pointer_click(&mut tree, first_line.x - 10.0, first_line.y + 3.0, MouseButton::Left);
  assert!(followed.lock().unwrap().is_empty());
  // The start of the next line, where the link's words continue.
  pointer_click(
    &mut tree,
    3.0,
    first_line.y + first_line.height + 3.0,
    MouseButton::Left,
  );
  assert_eq!(followed.lock().unwrap().len(), 1);
}
