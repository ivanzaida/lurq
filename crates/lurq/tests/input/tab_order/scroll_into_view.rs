use lurq::{
  app::App,
  components::{Button, Column, ScrollVertical},
};

use super::{focused_id, headless, shift_tab, tab};

fn list() -> Column {
  let mut column = Column::new();
  for index in 0..10 {
    column = column.child(
      Button::new(&format!("Item {index}"))
        .id(format!("item-{index}"))
        .height(40.0)
        .tab_index(0),
    );
  }
  column
}

fn item_top(tree: &mut lurq::app::Tree, index: usize) -> f32 {
  tree
    .get_element_by_id_mut(&format!("item-{index}"))
    .and_then(|element| element.bounds())
    .expect("item should be laid out")
    .y
}

#[test]
fn tab_scrolls_the_focused_element_into_view() {
  let mut tree = headless(ScrollVertical::new(list()).height(100.0));
  let mut app = App::new();

  // Items 0 and 1 are fully visible; item 2 starts at 80 and ends past 100.
  for _ in 0..3 {
    tab(&mut tree);
  }
  assert_eq!(focused_id(&tree).as_deref(), Some("item-2"));
  tree.pass_headless(&mut app);
  let top = item_top(&mut tree, 2);
  assert!(
    (top - 60.0).abs() < 0.5,
    "item-2 bottom aligns with the viewport bottom, top = {top}"
  );

  shift_tab(&mut tree);
  shift_tab(&mut tree);
  tree.pass_headless(&mut app);
  let top = item_top(&mut tree, 0);
  assert!(top.abs() < 0.5, "Shift+Tab scrolls back up, top = {top}");
}

#[test]
fn tab_does_not_scroll_when_the_element_is_visible() {
  let mut tree = headless(ScrollVertical::new(list()).height(100.0));

  tab(&mut tree);
  tree.pass_headless(&mut App::new());
  assert_eq!(item_top(&mut tree, 0), 0.0);
}
