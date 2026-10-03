//! A child its Row dropped to make room (`ShrinkLimit::Drop`) is not drawn:
//! nothing inside it is a Tab stop, and focus leaves it when it drops.

use lurq::{
  app::{App, Tree},
  components::{Button, Rect, Row},
  layout::{Constraints, Size, layout_kind::ShrinkLimit},
};

use super::{click_id, focused_id, headless, tab};

/// A toolbar: a button, a droppable group with a second button, and a fixed
/// 150 px block that leaves no room for the group in a narrow bar.
fn toolbar() -> Row {
  Row::new()
    .child(Button::new("Keep").id("keep").tab_index(0))
    .child(
      Row::new()
        .child(Button::new("Extra").id("extra").tab_index(0))
        .flex_shrink(1.0)
        .shrink_limit(ShrinkLimit::Drop),
    )
    .child(Rect::new(150.0, 10.0))
}

fn tab_order(tree: &mut Tree, stops: usize) -> Vec<String> {
  (0..stops)
    .map(|_| {
      tab(tree);
      focused_id(tree).unwrap_or_default()
    })
    .collect()
}

fn extra_width(tree: &mut Tree) -> f32 {
  tree
    .get_element_by_id_mut("extra")
    .and_then(|element| element.bounds())
    .expect("the extra button has bounds")
    .width
}

#[test]
fn tab_visits_a_child_that_fits() {
  let mut tree = headless(toolbar().width(700.0));
  assert_eq!(tab_order(&mut tree, 3), ["keep", "extra", "keep"]);
}

#[test]
fn tab_skips_a_dropped_child() {
  let mut tree = headless(toolbar().width(200.0));
  assert_eq!(
    extra_width(&mut tree),
    0.0,
    "the dropped button reports a zero-size rect"
  );
  assert_eq!(tab_order(&mut tree, 2), ["keep", "keep"]);
}

#[test]
fn focus_leaves_a_child_when_it_drops() {
  // The same tree, filling a narrower window: only the layout drops the group.
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(toolbar());
  let mut layout_at = |tree: &mut Tree, width: f32| {
    tree.set_layout_constraints_override(Some(Constraints::tight(Size::new(width, 40.0))));
    tree.pass_headless(&mut app);
  };
  layout_at(&mut tree, 700.0);
  click_id(&mut tree, "extra");
  assert_eq!(focused_id(&tree).as_deref(), Some("extra"));

  layout_at(&mut tree, 200.0);
  assert_eq!(extra_width(&mut tree), 0.0, "the group is dropped");
  assert_eq!(focused_id(&tree), None, "a dropped button keeps no focus");
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("keep"));
}
