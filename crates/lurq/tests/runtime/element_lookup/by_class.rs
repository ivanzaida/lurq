use lurq::{
  app::Tree,
  components::{Column, Rect, Text},
};

#[test]
fn finds_all_elements_by_class_in_tree_order() {
  let mut tree = Tree::new();
  tree.set_root(
    Column::new()
      .child(Text::new("a").class("item"))
      .child(Rect::new(1.0, 1.0))
      .child(Text::new("b").class("item").class("selected"))
      .child(Text::new("c").class("item")),
  );

  let items = tree.get_elements_by_class_name("item");
  let texts: Vec<_> = items.iter().filter_map(|element| element.text_content()).collect();
  assert_eq!(texts, ["a", "b", "c"]);

  assert_eq!(tree.get_elements_by_class_name("selected").len(), 1);
  assert!(tree.get_elements_by_class_name("missing").is_empty());
}

#[test]
fn class_builder_deduplicates_like_dom_class_list() {
  let mut tree = Tree::new();
  tree.set_root(Column::new().child(Text::new("a").id("t").class("x").class("x").classes(["x", "y"])));

  let classes: Vec<_> = tree.get_element_by_id("t").unwrap().classes().collect();
  assert_eq!(classes, ["x", "y"]);
}

#[test]
fn class_list_mutations_are_reflected_in_lookup() {
  let mut tree = Tree::new();
  tree.set_root(
    Column::new()
      .child(Text::new("a").id("first").class("item"))
      .child(Text::new("b").id("second")),
  );

  let mut handle = tree.get_element_by_id_mut("second").unwrap();
  handle.add_class("item");
  assert!(handle.has_class("item"));
  assert_eq!(tree.get_elements_by_class_name("item").len(), 2);

  tree.get_element_by_id_mut("first").unwrap().remove_class("item");
  assert_eq!(tree.get_elements_by_class_name("item").len(), 1);

  let mut handle = tree.get_element_by_id_mut("second").unwrap();
  assert!(!handle.toggle_class("item"));
  assert!(handle.toggle_class("item"));
  assert_eq!(tree.get_elements_by_class_name("item").len(), 1);
}
