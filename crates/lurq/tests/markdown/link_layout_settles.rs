//! A Markdown document settles once it is shown, with a link in it or not: an
//! idle pass finds no layout dirty, so a window showing a reply with a link is
//! not laid out again on every frame. Hovering a link changes only what is
//! painted.

use lurq::{
  app::{App, Tree},
  components::MarkdownProps,
};

use super::links::{MarkdownRoot, bounds_of};

const PLAIN: &str = "See the run, which is green.";
const LINKED: &str = "See [the run](https://example.invalid/runs/12), which is green.";

fn show(source: &'static str) -> (App, Tree) {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<MarkdownRoot>(&mut app, MarkdownProps::new(source).width(400.0));
  for _ in 0..3 {
    tree.pass_headless(&mut app);
  }
  (app, tree)
}

/// How many of ten idle passes after the document was shown found the layout dirty.
fn dirty_passes(app: &mut App, tree: &mut Tree) -> usize {
  (0..10).filter(|_| tree.pass_headless(app).reasons.layout_dirty).count()
}

#[test]
fn a_document_without_a_link_settles() {
  let (mut app, mut tree) = show(PLAIN);
  assert_eq!(dirty_passes(&mut app, &mut tree), 0);
}

#[test]
fn a_document_with_a_link_settles() {
  let (mut app, mut tree) = show(LINKED);
  assert_eq!(dirty_passes(&mut app, &mut tree), 0, "a link keeps the layout dirty");
}

#[test]
fn hovering_a_link_marks_paint_not_layout() {
  let (mut app, mut tree) = show(LINKED);
  let link = bounds_of(&mut tree, "the run");
  tree.mouse_move(link.x + link.width / 2.0, link.y + link.height / 2.0);
  let report = tree.pass_headless(&mut app);
  assert!(report.required, "the hover is drawn");
  assert!(
    !report.layout_recalculated,
    "a hovered link whose style keeps its metrics is painted, not laid out again"
  );
  assert_eq!(dirty_passes(&mut app, &mut tree), 0, "a hovered link settles too");
}
