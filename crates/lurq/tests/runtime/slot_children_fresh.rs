//! The children a component was handed (`ctx.mount_with`, `ctx.children()`) are components' outputs as they
//! were when the parent last rendered. A child that renders again by itself since is shown as it is now when the
//! component that holds it renders again by itself too: a row that follows a finger, a sheet that moves, with a
//! child in it that changed meanwhile.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Column, Row, Text},
  core::Signal,
  node::Element,
};

use crate::support::TestSurface;

/// The signals the components of a test made, by name, and how many times each component was created.
#[derive(Default)]
struct Book {
  signals: Mutex<Vec<(&'static str, Signal<u32>)>>,
  created: Mutex<Vec<&'static str>>,
}

/// One book for the whole of a test.
impl PartialEq for Book {
  fn eq(&self, _: &Self) -> bool {
    true
  }
}

impl Book {
  fn keep(&self, name: &'static str, signal: &Signal<u32>) {
    self.signals.lock().unwrap().push((name, signal.clone()));
    self.created.lock().unwrap().push(name);
  }

  /// One more in the signal called `name`: its component renders again by itself.
  fn bump(&self, name: &str) {
    let signals = self.signals.lock().unwrap();
    let (_, signal) = signals
      .iter()
      .find(|(kept, _)| *kept == name)
      .unwrap_or_else(|| panic!("no component called {name}"));
    signal.set(signal.get_untracked() + 1);
  }

  fn created(&self) -> Vec<&'static str> {
    self.created.lock().unwrap().clone()
  }
}

#[derive(Clone)]
struct Named {
  name: &'static str,
  book: Arc<Book>,
}

impl PartialEq for Named {
  fn eq(&self, other: &Self) -> bool {
    self.name == other.name
  }
}

impl lurq::app::component::DevtoolsInspectable for Named {}

/// A component with a count of its own, which it shows.
struct Counter {
  name: &'static str,
  count: Signal<u32>,
}

impl Component for Counter {
  type Props = Named;

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Named>().clone();
    let count = ctx.signal(0);
    props.book.keep(props.name, &count);
    Self {
      name: props.name,
      count,
    }
  }

  fn render(&self, _: &mut Ctx) -> impl Into<Element> {
    Text::new(&format!("{} {}", self.name, self.count.get())).id(self.name)
  }
}

/// A component with a counter in it, which it mounts itself.
struct Wrapping;

impl Component for Wrapping {
  type Props = Named;

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Named>().clone();
    Row::new().child(ctx.mount::<Counter>(props))
  }
}

/// A component that holds the children it was handed, and renders again by itself as its own count goes.
struct Holder {
  turns: Signal<u32>,
}

impl Component for Holder {
  type Props = Named;

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Named>().clone();
    let turns = ctx.signal(0);
    props.book.keep(props.name, &turns);
    Self { turns }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    Column::new()
      .child(Text::new(&format!("turn {}", self.turns.get())).id("turn"))
      .with_children(ctx.children().to_vec())
  }
}

/// The parent: a holder, handed a counter, a counter in a row of the parent's own, and a component with a
/// counter in it.
struct Parent;

impl Component for Parent {
  type Props = Arc<Book>;

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let book = ctx.props::<Arc<Book>>().clone();
    let named = |name: &'static str| Named {
      name,
      book: book.clone(),
    };
    let alone = ctx.mount_keyed::<Counter>("alone", named("alone"));
    let in_a_row = Row::new().child(ctx.mount_keyed::<Counter>("in-a-row", named("in-a-row")));
    let wrapped = ctx.mount_keyed::<Wrapping>("wrapped", named("wrapped"));
    ctx.mount_with::<Holder>(named("holder"), vec![alone, in_a_row.into(), wrapped])
  }
}

fn says(tree: &mut Tree, id: &str) -> String {
  tree
    .get_element_by_id(id)
    .and_then(|element| element.text_content().map(str::to_owned))
    .unwrap_or_else(|| panic!("nothing called {id}"))
}

#[test]
fn a_holder_that_renders_again_alone_shows_its_children_as_they_are_now() {
  let book = Arc::new(Book::default());
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(300, 300);
  tree.mount_root::<Parent>(&mut app, book.clone());
  tree.pass(&mut app, &TestSurface);
  assert_eq!(says(&mut tree, "alone"), "alone 0");

  for name in ["alone", "in-a-row", "wrapped"] {
    // The child renders again by itself: the tree shows it.
    book.bump(name);
    tree.pass(&mut app, &TestSurface);
    assert_eq!(says(&mut tree, name), format!("{name} 1"));
    // Then the holder does, with the children the parent handed it before that.
    book.bump("holder");
    tree.pass(&mut app, &TestSurface);
    assert_eq!(
      says(&mut tree, name),
      format!("{name} 1"),
      "the holder shows what the child is now, and not what it was when the parent rendered"
    );
    // And again: what it kept is the child as it is.
    book.bump("holder");
    tree.pass(&mut app, &TestSurface);
    assert_eq!(says(&mut tree, name), format!("{name} 1"));
  }
  assert_eq!(says(&mut tree, "turn"), "turn 6");

  // Both in one pass: the child's, and the holder's.
  book.bump("alone");
  book.bump("holder");
  tree.pass(&mut app, &TestSurface);
  assert_eq!(says(&mut tree, "alone"), "alone 2");
  assert_eq!(says(&mut tree, "turn"), "turn 7");
  book.bump("holder");
  tree.pass(&mut app, &TestSurface);
  assert_eq!(says(&mut tree, "alone"), "alone 2");

  // A child's count is still its own: it goes on from where it was.
  book.bump("alone");
  tree.pass(&mut app, &TestSurface);
  assert_eq!(says(&mut tree, "alone"), "alone 3");
  assert_eq!(
    book.created(),
    ["alone", "in-a-row", "wrapped", "holder"],
    "no component was created again"
  );
}
