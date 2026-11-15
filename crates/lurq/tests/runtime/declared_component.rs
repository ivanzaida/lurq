//! A component an element declares where it stands (`Element::component`): mounted by the component whose
//! render returns the element, once, with a state of its own, and gone when a render no longer returns it.

use std::{
  collections::HashMap,
  sync::{Arc, Mutex},
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Column, Row, Text},
  core::Signal,
  node::Element,
};

use crate::support::TestSurface;

/// What the counters of a test did, and the count each keeps.
#[derive(Default)]
struct Book {
  log: Mutex<Vec<String>>,
  counts: Mutex<HashMap<String, Signal<u32>>>,
}

impl Book {
  fn log(&self) -> Vec<String> {
    self.log.lock().unwrap().clone()
  }

  /// Counts one more in the counter called `name`.
  fn count(&self, name: &str) {
    let count = self.counts.lock().unwrap()[name].clone();
    count.set(count.get_untracked() + 1);
  }
}

#[derive(Clone, lurq::DevtoolsInspectable)]
struct CounterProps {
  name: &'static str,
  #[devtools_ignore]
  book: Arc<Book>,
}

impl PartialEq for CounterProps {
  fn eq(&self, other: &Self) -> bool {
    self.name == other.name
  }
}

/// A component with a count of its own, which shows its name and the count.
struct Counter {
  count: Signal<u32>,
  book: Arc<Book>,
}

impl Component for Counter {
  type Props = CounterProps;

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<CounterProps>().clone();
    let count = ctx.signal(0);
    props.book.log.lock().unwrap().push(format!("created {}", props.name));
    props
      .book
      .counts
      .lock()
      .unwrap()
      .insert(props.name.to_owned(), count.clone());
    Self {
      count,
      book: props.book,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let name = ctx.props::<CounterProps>().name;
    Text::new(&format!("{name} {}", self.count.get()))
  }

  fn on_unmounted(&self) {
    self.book.log.lock().unwrap().push("gone".to_owned());
  }
}

fn counter(name: &'static str, book: &Arc<Book>) -> Element {
  Element::component::<Counter>(CounterProps {
    name,
    book: book.clone(),
  })
}

type Scene = Arc<dyn Fn(u32) -> Element + Send + Sync>;

#[derive(Clone, lurq::DevtoolsInspectable)]
struct PageProps {
  step: Signal<u32>,
  #[devtools_ignore]
  scene: Scene,
}

impl PartialEq for PageProps {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.scene, &other.scene)
  }
}

/// A component that renders a scene from a step, with no `Ctx` given to what builds it.
struct Page;

impl Component for Page {
  type Props = PageProps;

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<PageProps>().clone();
    (props.scene)(props.step.get())
  }
}

struct Window {
  app: App,
  tree: Tree,
  step: Signal<u32>,
}

impl Window {
  fn new(scene: impl Fn(u32) -> Element + Send + Sync + 'static) -> Self {
    let mut app = App::new();
    let mut tree = Tree::new();
    let step = Signal::new(0);
    tree.mount_root::<Page>(
      &mut app,
      PageProps {
        step: step.clone(),
        scene: Arc::new(scene),
      },
    );
    tree.pass(&mut app, &TestSurface);
    Self { app, tree, step }
  }

  fn pass(&mut self) {
    self.tree.pass(&mut self.app, &TestSurface);
  }

  fn step(&mut self, step: u32) {
    self.step.set(step);
    self.pass();
  }

  fn shows(&mut self, text: &str) -> bool {
    self
      .tree
      .find_element(|element| element.text_content() == Some(text))
      .is_some()
  }
}

#[test]
fn a_declared_component_is_created_once_and_keeps_its_state() {
  let book = Arc::new(Book::default());
  let its = book.clone();
  let mut window = Window::new(move |step| {
    Column::new()
      .child(Text::new(&format!("step {step}")))
      .child(Row::new().child(counter("a", &its)))
      .into()
  });
  assert!(window.shows("a 0"));
  assert_eq!(book.log(), ["created a"]);

  // Its count is its own: it renders again by itself.
  book.count("a");
  window.pass();
  assert!(window.shows("a 1") && window.shows("step 0"));

  // What declares it renders again: it is the component it was, with the count it had.
  window.step(1);
  assert!(window.shows("step 1") && window.shows("a 1"));
  assert_eq!(book.log(), ["created a"]);
}

#[test]
fn it_is_gone_when_a_render_no_longer_returns_it() {
  let book = Arc::new(Book::default());
  let its = book.clone();
  let mut window = Window::new(move |step| {
    let column = Column::new().child(Text::new("page"));
    match step {
      1 => column.into(),
      _ => column.child(counter("a", &its)).into(),
    }
  });
  book.count("a");
  window.pass();
  assert!(window.shows("a 1"));

  window.step(1);
  assert!(!window.shows("a 1"));
  assert_eq!(book.log(), ["created a", "gone"]);

  // Returned again, it is a new one.
  window.step(2);
  assert!(window.shows("a 0"));
  assert_eq!(book.log(), ["created a", "gone", "created a"]);
}

#[test]
fn a_key_keeps_each_ones_state_where_the_order_changes() {
  let book = Arc::new(Book::default());
  let its = book.clone();
  let mut window = Window::new(move |step| {
    let keyed = |name: &'static str| {
      Element::component_keyed::<Counter>(
        name,
        CounterProps {
          name,
          book: its.clone(),
        },
      )
    };
    // The key on the element does as the key given with the component.
    let late = |name: &'static str| counter(name, &its).key(name);
    match step {
      0 => Column::new().child(keyed("a")).child(late("b")).into(),
      _ => Column::new().child(late("b")).child(keyed("a")).into(),
    }
  });
  book.count("a");
  window.pass();
  assert!(window.shows("a 1") && window.shows("b 0"));

  window.step(1);
  assert!(window.shows("a 1") && window.shows("b 0"));
  assert_eq!(book.log(), ["created a", "created b"]);
}

#[test]
fn what_is_put_on_the_element_goes_to_the_components_root() {
  let book = Arc::new(Book::default());
  let its = book.clone();
  let mut window = Window::new(move |_| {
    Column::new()
      .child(counter("a", &its).id("first").class("counter"))
      .into()
  });
  let found = |window: &mut Window| {
    window
      .tree
      .get_element_by_id("first")
      .map(|element| (element.text_content().map(str::to_owned), element.has_class("counter")))
  };
  assert_eq!(found(&mut window), Some((Some("a 0".to_owned()), true)));

  // Also when the component renders again by itself, and its root is a new one.
  book.count("a");
  window.pass();
  assert_eq!(found(&mut window), Some((Some("a 1".to_owned()), true)));
}

/// A component that declares a counter in what it renders: the counter is its own.
struct Shelf;

impl Component for Shelf {
  type Props = CounterProps;

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<CounterProps>().clone();
    Row::new()
      .child(Text::new("shelf"))
      .child(counter(props.name, &props.book))
  }
}

#[test]
fn one_declared_in_a_declared_component_is_mounted_by_it() {
  let book = Arc::new(Book::default());
  let its = book.clone();
  let mut window = Window::new(move |step| {
    Column::new()
      .child(Text::new(&format!("step {step}")))
      .child(Element::component::<Shelf>(CounterProps {
        name: "a",
        book: its.clone(),
      }))
      .child(counter("b", &its))
      .into()
  });
  assert!(window.shows("shelf") && window.shows("a 0") && window.shows("b 0"));
  book.count("a");
  book.count("b");
  book.count("b");
  window.pass();
  window.step(1);
  assert!(window.shows("a 1") && window.shows("b 2"));
  assert_eq!(book.log(), ["created a", "created b"]);
}

#[test]
fn in_a_tree_no_component_renders_it_shows_nothing() {
  let book = Arc::new(Book::default());
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(Column::new().child(Text::new("page")).child(counter("a", &book)));
  tree.pass(&mut app, &TestSurface);
  assert!(
    tree
      .find_element(|element| element.text_content() == Some("page"))
      .is_some()
  );
  assert!(
    tree
      .find_element(|element| element.text_content() == Some("a 0"))
      .is_none()
  );
  assert!(book.log().is_empty(), "nothing is mounted where nothing renders");
}
