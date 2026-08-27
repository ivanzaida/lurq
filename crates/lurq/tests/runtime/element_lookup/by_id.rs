use lurq::{
  app::Tree,
  components::{Column, Rect, Row, Text},
};

use crate::support::run_pass;

#[test]
fn finds_element_by_id_in_tree_order() {
  let mut tree = Tree::new();
  tree.set_root(
    Column::new()
      .child(Text::new("title").id("headline"))
      .child(Row::new().id("toolbar").child(Rect::new(10.0, 10.0).id("icon"))),
  );

  let headline = tree.get_element_by_id("headline").expect("headline should be found");
  assert_eq!(headline.text_content(), Some("title"));
  assert_eq!(headline.id(), Some("headline"));

  assert_eq!(tree.get_element_by_id("toolbar").unwrap().tag_name(), "Row");
  assert_eq!(tree.get_element_by_id("icon").unwrap().tag_name(), "Rect");
  assert!(tree.get_element_by_id("missing").is_none());
}

#[test]
fn lookup_works_before_first_layout_pass() {
  let mut tree = Tree::new();
  tree.set_root(Column::new().child(Text::new("early").id("pre-layout")));

  // No pass has run: `find_element` has no layout to walk, but the by-id
  // walk reads the live tree directly.
  assert_eq!(
    tree.get_element_by_id("pre-layout").unwrap().text_content(),
    Some("early")
  );

  run_pass(&mut tree);
  assert!(tree.get_element_by_id("pre-layout").is_some());
}

#[test]
fn duplicate_ids_resolve_to_first_match_in_tree_order() {
  let mut tree = Tree::new();
  tree.set_root(
    Column::new()
      .child(Text::new("first").id("row"))
      .child(Text::new("second").id("row")),
  );

  assert_eq!(tree.get_element_by_id("row").unwrap().text_content(), Some("first"));
}

#[test]
fn id_and_classes_are_usable_in_find_element_predicates() {
  let mut tree = Tree::new();
  tree.set_root(
    Column::new().child(
      Rect::new(10.0, 20.0)
        .id("target")
        .class("card")
        .classes(["primary", "elevated"]),
    ),
  );
  run_pass(&mut tree);

  assert!(tree.find_element(|element| element.id() == Some("target")).is_some());
  assert!(tree.find_element(|element| element.has_class("elevated")).is_some());
  assert!(tree.find_element(|element| element.has_class("missing")).is_none());

  let classes: Vec<_> = tree
    .get_element_by_id("target")
    .unwrap()
    .classes()
    .map(str::to_owned)
    .collect();
  assert_eq!(classes, ["card", "primary", "elevated"]);
}
