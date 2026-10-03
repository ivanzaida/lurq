//! Give-way order (`shrink_order`) and limits (`ShrinkLimit`) of shrinking
//! children in single-line Rows and Columns.

mod cache;
mod content_limit;
mod drop;
mod order;
mod sub_pixel;

use lurq::{
  app::{App, Tree},
  components::{Text, TextOverflow},
  layout::{Constraints, Size, layout_result::LayoutResult, quad::QuadContent},
  node::Element,
};

const LINE_HEIGHT: f32 = 20.0;
const WIDE: f32 = 2000.0;
const SPACING: f32 = 6.0;
/// Drawn widths are compared with this tolerance where they are sums of
/// measured text widths.
const EPSILON: f32 = 0.01;

const DETAIL: &str = "Designer asks to use pencil batch_design";
const RUN: &str = "Run 3 · KON-3";
const TIME: &str = "15:32";
const COUNT: &str = "1 approval waiting";

/// A single-line label that ellipsizes when it is laid out narrower than its
/// text.
fn label(text: &str) -> Text {
  Text::new(text).nowrap().text_overflow(TextOverflow::Elipsis)
}

/// One pass under `constraints` with a persistent `App`: a fresh `App`
/// changes the theme version and force-dirties every layout.
fn layout_at(tree: &mut Tree, app: &mut App, constraints: Constraints) -> LayoutResult {
  tree.set_layout_constraints_override(Some(constraints));
  tree.pass_headless(app);
  tree.last_layout().expect("layout result").clone()
}

fn tight(width: f32, height: f32) -> Constraints {
  Constraints::tight(Size::new(width, height))
}

/// Lays `root` out once, in a fresh tree, at a tight size.
fn layout_once(root: impl Into<Element>, width: f32, height: f32) -> (Tree, LayoutResult) {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(root);
  let layout = layout_at(&mut tree, &mut app, tight(width, height));
  (tree, layout)
}

/// The width of each direct child of `layout`.
fn child_widths(layout: &LayoutResult) -> Vec<f32> {
  layout.children.iter().map(|child| child.result.size.width).collect()
}

/// The height of each direct child of `layout`.
fn child_heights(layout: &LayoutResult) -> Vec<f32> {
  layout.children.iter().map(|child| child.result.size.height).collect()
}

/// Every string the renderer draws, in paint order.
fn drawn_texts(tree: &Tree, layout: &LayoutResult) -> Vec<String> {
  tree
    .resolve_quads(layout)
    .into_iter()
    .filter_map(|quad| match quad.content {
      QuadContent::Text { text, .. } => Some(text),
      _ => None,
    })
    .collect()
}

fn assert_close(actual: f32, expected: f32, what: &str) {
  assert!((actual - expected).abs() <= EPSILON, "{what}: {actual} != {expected}");
}
