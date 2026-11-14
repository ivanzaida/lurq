//! A nested focus group, a tree: Right opens a closed element and goes into an open one, Left closes an open
//! element and goes out of a closed one, `*` opens the elements at the focused one's level, and Up, Down, Home,
//! End and typeahead go through what is shown. Opening and closing is the app's: the keys call the element's
//! toggle, and what it holds is there with the next render.

use std::sync::{Arc, Mutex};

use lurq::{
  app::Tree,
  components::{Button, Column},
  node::FocusGroup,
};

use super::{focused_id, headless, tab};

fn key(tree: &mut Tree, key: &str) {
  tree.key_down(key.to_owned(), key.to_owned(), false, false, false);
  tree.key_up(key.to_owned(), key.to_owned(), false, false, false);
}

type Asked = Arc<Mutex<Vec<(&'static str, bool)>>>;

/// A row of a tree: a button at `level`, which opens where `open` is `Some`, telling `asked` what it is asked.
fn row(id: &'static str, level: u32, open: Option<bool>, asked: &Asked) -> Button {
  let button = Button::new(id).id(id.to_owned()).tab_index(0).focus_level(level);
  match open {
    Some(open) => {
      let asked = asked.clone();
      button.focus_expanded(open, move |to| asked.lock().unwrap().push((id, to)))
    }
    None => button,
  }
}

/// A file tree, flat as a tree view lays it out:
///
/// ```text
/// crates      (open)
///   lurq      (open)
///     lib.rs
///     theme.rs
///   demo      (closed)
///   gallery   (closed)
/// Cargo.toml
/// docs        (closed)
/// ```
fn files(asked: &Asked) -> Column {
  Column::new()
    .focus_group(
      FocusGroup::vertical()
        .nested(true)
        .one_stop(true)
        .typeahead(true)
        .wrap(false),
    )
    .with_children([
      row("crates", 1, Some(true), asked),
      row("lurq", 2, Some(true), asked),
      row("lib.rs", 3, None, asked),
      row("theme.rs", 3, None, asked),
      row("demo", 2, Some(false), asked),
      row("gallery", 2, Some(false), asked),
      row("Cargo.toml", 1, None, asked),
      row("docs", 1, Some(false), asked),
    ])
}

#[test]
fn right_opens_a_closed_element_and_goes_into_an_open_one() {
  let asked = Asked::default();
  let mut tree = headless(files(&asked));
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("crates"));
  // Open: into it, to the first element it holds.
  key(&mut tree, "ArrowRight");
  assert_eq!(focused_id(&tree).as_deref(), Some("lurq"));
  key(&mut tree, "ArrowRight");
  assert_eq!(focused_id(&tree).as_deref(), Some("lib.rs"));
  // An element that does not open: nothing, and the key goes to nothing else.
  key(&mut tree, "ArrowRight");
  assert_eq!(focused_id(&tree).as_deref(), Some("lib.rs"));
  assert!(asked.lock().unwrap().is_empty());
  // Closed: asked to open, and the focus stays.
  key(&mut tree, "ArrowDown");
  key(&mut tree, "ArrowDown");
  assert_eq!(focused_id(&tree).as_deref(), Some("demo"));
  key(&mut tree, "ArrowRight");
  assert_eq!(focused_id(&tree).as_deref(), Some("demo"));
  assert_eq!(*asked.lock().unwrap(), [("demo", true)]);
}

#[test]
fn left_closes_an_open_element_and_goes_out_of_a_closed_one_or_a_leaf() {
  let asked = Asked::default();
  let mut tree = headless(files(&asked));
  tab(&mut tree);
  for _ in 0..3 {
    key(&mut tree, "ArrowDown");
  }
  assert_eq!(focused_id(&tree).as_deref(), Some("theme.rs"));
  // A leaf: out to the element it is in, past its sibling.
  key(&mut tree, "ArrowLeft");
  assert_eq!(focused_id(&tree).as_deref(), Some("lurq"));
  // Open: asked to close, and the focus stays.
  key(&mut tree, "ArrowLeft");
  assert_eq!(focused_id(&tree).as_deref(), Some("lurq"));
  assert_eq!(*asked.lock().unwrap(), [("lurq", false)]);
  // A closed one: out, to the element it is in.
  key(&mut tree, "End");
  assert_eq!(focused_id(&tree).as_deref(), Some("docs"));
  key(&mut tree, "ArrowUp");
  key(&mut tree, "ArrowUp");
  assert_eq!(focused_id(&tree).as_deref(), Some("gallery"));
  key(&mut tree, "ArrowLeft");
  assert_eq!(focused_id(&tree).as_deref(), Some("crates"));
  // At the top level there is nowhere out to.
  key(&mut tree, "End");
  key(&mut tree, "ArrowLeft");
  assert_eq!(focused_id(&tree).as_deref(), Some("docs"));
  assert_eq!(
    *asked.lock().unwrap(),
    [("lurq", false)],
    "a closed one is not asked again"
  );
}

#[test]
fn star_opens_every_closed_element_at_the_focused_ones_level_in_the_element_it_is_in() {
  let asked = Asked::default();
  let mut tree = headless(files(&asked));
  tab(&mut tree);
  key(&mut tree, "ArrowDown");
  assert_eq!(focused_id(&tree).as_deref(), Some("lurq"));
  key(&mut tree, "*");
  // demo and gallery, the closed ones beside lurq in crates; not docs, at the top, and not lurq, open already.
  assert_eq!(*asked.lock().unwrap(), [("demo", true), ("gallery", true)]);
  assert_eq!(focused_id(&tree).as_deref(), Some("lurq"), "the focus stays");
  // At the top level: docs, the closed one there.
  asked.lock().unwrap().clear();
  key(&mut tree, "Home");
  key(&mut tree, "*");
  assert_eq!(*asked.lock().unwrap(), [("docs", true)]);
}

#[test]
fn up_down_home_end_and_typeahead_go_through_what_is_shown_and_do_not_wrap() {
  let asked = Asked::default();
  let mut tree = headless(files(&asked));
  tab(&mut tree);
  key(&mut tree, "ArrowUp");
  assert_eq!(focused_id(&tree).as_deref(), Some("crates"), "a tree does not wrap");
  key(&mut tree, "End");
  assert_eq!(focused_id(&tree).as_deref(), Some("docs"));
  key(&mut tree, "ArrowDown");
  assert_eq!(focused_id(&tree).as_deref(), Some("docs"));
  key(&mut tree, "Home");
  key(&mut tree, "t");
  assert_eq!(focused_id(&tree).as_deref(), Some("theme.rs"));
  // `*` is not a letter typeahead takes in a nested group.
  key(&mut tree, "*");
  assert_eq!(focused_id(&tree).as_deref(), Some("theme.rs"));
}

#[test]
fn a_group_that_is_not_nested_leaves_right_left_and_star_alone() {
  let asked = Asked::default();
  let mut tree = headless(Column::new().focus_group(FocusGroup::vertical()).with_children([
    row("crates", 1, Some(false), &asked),
    row("docs", 1, Some(false), &asked),
  ]));
  tab(&mut tree);
  key(&mut tree, "ArrowRight");
  key(&mut tree, "*");
  key(&mut tree, "ArrowLeft");
  assert!(asked.lock().unwrap().is_empty());
  assert_eq!(focused_id(&tree).as_deref(), Some("crates"));
}

/// A sidebar's nested items: each item that opens is a button followed by a column of what it holds, there
/// while it is open, rather than one flat list. The levels say what is in what.
#[test]
fn a_nested_list_of_elements_in_elements_climbs_by_the_levels_as_a_flat_one_does() {
  let asked = Asked::default();
  let section = |id: &'static str, open: bool, items: Vec<Button>| {
    let mut column = Column::new().child(row(id, 1, Some(open), &asked));
    if open {
      column = column.child(Column::new().with_children(items));
    }
    column
  };
  let mut tree = headless(
    Column::new()
      .focus_group(FocusGroup::vertical().nested(true).one_stop(true))
      .child(section(
        "projects",
        true,
        vec![row("lurq", 2, None, &asked), row("pencil", 2, None, &asked)],
      ))
      .child(section("archive", false, vec![row("old", 2, None, &asked)])),
  );
  tab(&mut tree);
  key(&mut tree, "ArrowRight");
  assert_eq!(focused_id(&tree).as_deref(), Some("lurq"));
  key(&mut tree, "ArrowDown");
  key(&mut tree, "ArrowDown");
  assert_eq!(
    focused_id(&tree).as_deref(),
    Some("archive"),
    "the closed section's items are not there"
  );
  key(&mut tree, "ArrowRight");
  assert_eq!(*asked.lock().unwrap(), [("archive", true)]);
  key(&mut tree, "ArrowUp");
  key(&mut tree, "ArrowLeft");
  assert_eq!(focused_id(&tree).as_deref(), Some("projects"));
}
