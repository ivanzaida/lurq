//! A press where nothing can take focus blurs the focused element, like HTML.

use lurq::{
  app::{App, Tree, events::MouseButton},
  components::{Button, Column, Rect, TextInput},
  core::Signal,
};

use crate::support::pointer_click;

fn screen() -> Tree {
  let mut tree = Tree::new();
  tree.set_root(
    Column::new()
      .child(
        TextInput::new(Signal::new(String::new()))
          .id("editor")
          .size(200.0, 30.0),
      )
      .child(Button::new("Save").id("save").size(100.0, 30.0))
      .child(Button::new("Bold").id("bold").size(100.0, 30.0).focusable(false))
      .child(Rect::new(200.0, 60.0).id("blank")),
  );
  tree.pass_headless(&mut App::new());
  tree
}

fn click(tree: &mut Tree, id: &str) {
  let (x, y) = tree
    .get_element_by_id_mut(id)
    .and_then(|element| element.bounds())
    .expect("element should be laid out")
    .center();
  pointer_click(tree, x, y, MouseButton::Left);
}

fn focused(tree: &Tree) -> Option<String> {
  tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

#[test]
fn clicking_where_nothing_can_take_focus_blurs_a_focused_button() {
  let mut tree = screen();
  click(&mut tree, "save");
  assert_eq!(focused(&tree).as_deref(), Some("save"));

  click(&mut tree, "blank");
  assert_eq!(focused(&tree), None);
}

#[test]
fn clicking_empty_window_space_blurs_a_focused_input() {
  let mut tree = screen();
  click(&mut tree, "editor");
  assert_eq!(focused(&tree).as_deref(), Some("editor"));

  pointer_click(&mut tree, 700.0, 500.0, MouseButton::Left);
  assert_eq!(focused(&tree), None);
}

#[test]
fn clicking_a_focusable_element_moves_focus_to_it() {
  let mut tree = screen();
  click(&mut tree, "editor");
  click(&mut tree, "save");
  assert_eq!(focused(&tree).as_deref(), Some("save"));
}

#[test]
fn clicking_a_focusable_false_button_keeps_the_editor_focused() {
  let mut tree = screen();
  click(&mut tree, "editor");

  click(&mut tree, "bold");
  assert_eq!(focused(&tree).as_deref(), Some("editor"));

  tree.key_down("a".to_owned(), "KeyA".to_owned(), false, false, false);
  assert_eq!(
    tree
      .get_element_by_id_mut("editor")
      .and_then(|element| element.as_text_input())
      .map(|input| input.value()),
    Some("a".to_owned()),
    "typing still reaches the editor"
  );
}
