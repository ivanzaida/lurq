use lurq::app::{App, Tree};

mod focusable;
#[cfg(feature = "form")]
mod forms_in_scope;
mod modal_trap;
mod scroll_into_view;
mod window_chrome;
mod window_scope;

fn tab(tree: &mut Tree) {
  tree.key_down("Tab".to_owned(), "Tab".to_owned(), false, false, false);
}

fn shift_tab(tree: &mut Tree) {
  tree.key_down("Tab".to_owned(), "Tab".to_owned(), true, false, false);
}

fn focused_id(tree: &Tree) -> Option<String> {
  tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

/// Mounts `root` in an 800x600 window and lays it out without a surface.
fn headless(root: impl Into<lurq::node::Element>) -> Tree {
  let mut tree = Tree::new();
  tree.set_root(root);
  tree.pass_headless(&mut App::new());
  tree
}

fn click_id(tree: &mut Tree, id: &str) {
  let bounds = tree
    .get_element_by_id_mut(id)
    .and_then(|element| element.bounds())
    .unwrap_or_else(|| panic!("#{id} should be laid out"));
  let (x, y) = bounds.center();
  crate::support::pointer_click(tree, x, y, lurq::app::events::MouseButton::Left);
}
