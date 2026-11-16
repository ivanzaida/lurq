//! A parent patched where one child changed keeps what its other children were prepared with, and does not walk
//! them again, only while no text's layout was written anywhere since. A paragraph beside the child that changed,
//! laid out at another width in between, holds what that width cut it to: the entry of the first width, served or
//! patched, puts the paragraph back to what that entry holds. Each frame is checked against the same scene built
//! fresh.

use std::cell::RefCell;

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Column, Rect, Text, TextOverflow},
  core::Signal,
  node::{Element, color::Color, dimension::Dimension},
};

use crate::support::{RenderSnapshot, render_pass_with_app};

const WORDS: &str = "A paragraph long enough to take three lines in the wide room and five in the narrow one, so that \
                     the lines it holds say which room it was laid out in last.";
const WIDE: u32 = 300;
const NARROW: u32 = 200;

thread_local! {
  static HEIGHT: RefCell<Option<Signal<f32>>> = const { RefCell::new(None) };
}

/// What changes: built again on its own when its height does.
struct Grower;

impl Component for Grower {
  type Props = ();

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, _: &mut Ctx) -> impl Into<Element> {
    let height = HEIGHT.with(|height| height.borrow().clone().expect("a height is set"));
    Rect::new(40.0, height.get()).background(Color::new(30, 60, 90, 255))
  }
}

struct Page;

impl Component for Page {
  type Props = ();

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    Column::new()
      .width(Dimension::full())
      .child(cut())
      .child(ctx.mount::<Grower>(()))
      .child(cut())
  }
}

/// A paragraph cut at two lines with an ellipsis: the words it draws are what its layout wrote, another string in
/// each room.
fn cut() -> Text {
  Text::new(WORDS).max_lines(2).text_overflow(TextOverflow::Elipsis)
}

fn mount(width: u32, height: &Signal<f32>) -> (App, Tree) {
  HEIGHT.with(|slot| *slot.borrow_mut() = Some(height.clone()));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(width, 600);
  tree.mount_root::<Page>(&mut app, ());
  (app, tree)
}

fn glyphs(snapshot: &RenderSnapshot) -> Vec<[u32; 4]> {
  snapshot
    .glyphs
    .iter()
    .map(|glyph| [glyph.x, glyph.y, glyph.width, glyph.height].map(f32::to_bits))
    .collect()
}

/// The glyphs of the scene built fresh at `width` with the grower `height` high.
fn fresh(width: u32, height: f32) -> Vec<[u32; 4]> {
  // The scene under test keeps its own height: a fresh one is built with another, and gives the first back.
  let kept = HEIGHT.with(|slot| slot.borrow().clone());
  let (mut app, mut tree) = mount(width, &Signal::new(height));
  let shown = glyphs(&render_pass_with_app(&mut tree, &mut app));
  HEIGHT.with(|slot| *slot.borrow_mut() = kept);
  shown
}

#[test]
fn a_paragraph_laid_out_at_another_width_is_put_back_when_its_parent_is_patched() {
  let height = Signal::new(20.0);
  let (mut app, mut tree) = mount(WIDE, &height);
  render_pass_with_app(&mut tree, &mut app);
  assert_ne!(
    fresh(WIDE, 20.0),
    fresh(NARROW, 20.0),
    "the paragraph has other lines in the narrow room"
  );

  // The column is patched in the wide room: its entry is the one last prepared.
  for grown in [28.0, 36.0] {
    height.set(grown);
    let shown = glyphs(&render_pass_with_app(&mut tree, &mut app));
    assert_eq!(shown, fresh(WIDE, grown), "patched in the wide room, {grown} high");
  }

  // The narrow room lays the paragraphs out again: they hold its lines now.
  tree.resize(NARROW, 600);
  let shown = glyphs(&render_pass_with_app(&mut tree, &mut app));
  assert_eq!(shown, fresh(NARROW, 36.0), "the narrow room");

  // Back in the wide room in the frame the grower changes: the wide entry is patched, and the paragraphs beside
  // the grower, which did not change, show the wide room's lines.
  tree.resize(WIDE, 600);
  height.set(52.0);
  let shown = glyphs(&render_pass_with_app(&mut tree, &mut app));
  assert_eq!(shown, fresh(WIDE, 52.0), "patched back in the wide room");

  // And once more with nothing laid out in between, which the patch may take as it stands.
  height.set(60.0);
  let shown = glyphs(&render_pass_with_app(&mut tree, &mut app));
  assert_eq!(shown, fresh(WIDE, 60.0), "patched again");
}

#[test]
fn a_paragraph_laid_out_at_another_width_is_put_back_when_its_parent_is_served_as_it_was() {
  let height = Signal::new(20.0);
  let (mut app, mut tree) = mount(WIDE, &height);
  render_pass_with_app(&mut tree, &mut app);
  // Patched, then served: the wide entry is the one last prepared, and is served as it was prepared.
  height.set(28.0);
  render_pass_with_app(&mut tree, &mut app);
  tree.request_redraw();
  let shown = glyphs(&render_pass_with_app(&mut tree, &mut app));
  assert_eq!(shown, fresh(WIDE, 28.0), "served in the wide room");

  // The narrow room lays the paragraphs out again and keeps the wide entry beside its own.
  tree.resize(NARROW, 600);
  let shown = glyphs(&render_pass_with_app(&mut tree, &mut app));
  assert_eq!(shown, fresh(NARROW, 28.0), "the narrow room");

  // Wide again with nothing changed: the wide entry is served, with the paragraphs put back to its lines.
  tree.resize(WIDE, 600);
  let shown = glyphs(&render_pass_with_app(&mut tree, &mut app));
  assert_eq!(shown, fresh(WIDE, 28.0), "served again in the wide room");

  // And patched from there.
  height.set(44.0);
  let shown = glyphs(&render_pass_with_app(&mut tree, &mut app));
  assert_eq!(shown, fresh(WIDE, 44.0), "patched after it was served again");
}
