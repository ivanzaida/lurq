//! An interaction group (`.interaction_group(true)`): the pointer on it, or the focus on it or in it, lights the
//! `within_style` of what it holds, and only of what it holds; a group inside it lights its own; leaving puts
//! them out.

use lurq::{
  app::Tree,
  components::{Button, Column, Row},
  node::{Style, color::Color},
};

use crate::support::{RenderSnapshot, render_pass};

const LIT: &str = "#16a34a";
const INNER: &str = "#dc2626";

fn has_fill(snapshot: &RenderSnapshot, hex: &str) -> bool {
  snapshot.rects.iter().any(|rect| rect.color == Color::from_hex(hex))
}

fn count_fill(snapshot: &RenderSnapshot, hex: &str) -> usize {
  snapshot
    .rects
    .iter()
    .filter(|rect| rect.color == Color::from_hex(hex))
    .count()
}

/// An action of 20 that shows `hex` while the group it is in is lit.
fn action(id: &str, hex: &str) -> Row {
  Row::new()
    .id(id.to_owned())
    .size(20.0, 20.0)
    .within_style(Style::new().background(hex))
}

/// Two rows, each a group holding a label and an action; the second holds a group of its own.
fn screen() -> Column {
  Column::new()
    .child(
      Row::new()
        .id("first")
        .size(200.0, 40.0)
        .interaction_group(true)
        .child(Button::new("First").id("first-button").tab_index(0))
        .child(action("first-action", LIT)),
    )
    .child(
      Row::new()
        .id("second")
        .size(200.0, 40.0)
        .interaction_group(true)
        .child(action("second-action", LIT))
        .child(
          Row::new()
            .id("inner")
            .size(60.0, 30.0)
            .interaction_group(true)
            .child(action("inner-action", INNER)),
        ),
    )
}

fn hover(tree: &mut Tree, id: &str) {
  let (x, y) = tree
    .get_element_by_id_mut(id)
    .and_then(|element| element.bounds())
    .expect("element should be laid out")
    .center();
  tree.mouse_move(x, y);
}

#[test]
fn the_pointer_on_a_group_lights_what_it_holds_and_leaving_puts_it_out() {
  let mut tree = Tree::new();
  tree.set_root(screen());
  assert!(!has_fill(&render_pass(&mut tree), LIT), "nothing is lit at rest");

  hover(&mut tree, "first");
  let snapshot = render_pass(&mut tree);
  assert_eq!(count_fill(&snapshot, LIT), 1, "the first row's action alone");
  assert_eq!(
    tree
      .get_element_by_id("first-action")
      .and_then(|element| element.color()),
    Some(Color::from_hex(LIT))
  );

  // On the second row: the first goes out, the second lights; the group inside it lights only its own.
  hover(&mut tree, "second-action");
  let snapshot = render_pass(&mut tree);
  assert_eq!(count_fill(&snapshot, LIT), 1);
  assert!(!has_fill(&snapshot, INNER), "a group inside answers to itself");
  assert_eq!(
    tree
      .get_element_by_id("first-action")
      .and_then(|element| element.color()),
    None
  );
  hover(&mut tree, "inner");
  let snapshot = render_pass(&mut tree);
  assert!(has_fill(&snapshot, INNER));
  assert!(has_fill(&snapshot, LIT), "the pointer is on the outer group too");

  tree.mouse_leave_window();
  assert!(!has_fill(&render_pass(&mut tree), LIT));
}

#[test]
fn the_focus_in_a_group_lights_it_too() {
  let mut tree = Tree::new();
  tree.set_root(screen());
  render_pass(&mut tree);
  tree.key_down("Tab".to_owned(), "Tab".to_owned(), false, false, false);
  let snapshot = render_pass(&mut tree);
  assert_eq!(count_fill(&snapshot, LIT), 1, "the first row holds the focus");
  assert_eq!(
    tree
      .get_element_by_id("first-action")
      .and_then(|element| element.color()),
    Some(Color::from_hex(LIT))
  );
}
