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
  /// How often each counter rendered, by its name.
  renders: Mutex<HashMap<String, usize>>,
}

impl Book {
  fn log(&self) -> Vec<String> {
    self.log.lock().unwrap().clone()
  }

  fn renders(&self, name: &str) -> usize {
    self.renders.lock().unwrap().get(name).copied().unwrap_or(0)
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
    *self.book.renders.lock().unwrap().entry(name.to_owned()).or_default() += 1;
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

/// A component with a turn of its own that declares three counters, in the order its turn says.
struct Rack {
  turn: Signal<u32>,
}

impl Component for Rack {
  type Props = CounterProps;

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<CounterProps>().clone();
    let turn = ctx.signal(0);
    props
      .book
      .counts
      .lock()
      .unwrap()
      .insert("rack".to_owned(), turn.clone());
    Self { turn }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let book = ctx.props::<CounterProps>().book.clone();
    let turn = self.turn.get();
    let keyed = |name: &'static str| counter(name, &book).key(name);
    let row = Row::new().child(Text::new(&format!("turn {turn}")));
    // The counters stand in a box of their own, so that they are no direct children of the rack's root.
    let inner = match turn % 2 {
      0 => Row::new().child(keyed("a")).child(keyed("b")).child(keyed("c")),
      _ => Row::new().child(keyed("c")).child(keyed("a")).child(keyed("b")),
    };
    row.child(inner)
  }
}

/// The id of the node that draws `text`.
fn node_of(window: &mut Window, text: &str) -> lurq::core::NodeId {
  let found = std::cell::Cell::new(None);
  window.tree.find_element(|element| {
    let drawn = element.text_content() == Some(text);
    if drawn {
      found.set(Some(element.node_id()));
    }
    drawn
  });
  found.get().unwrap_or_else(|| panic!("{text:?} is drawn"))
}

#[test]
fn unchanged_ones_are_the_nodes_they_were_when_what_declares_them_renders_again() {
  let book = Arc::new(Book::default());
  let its = book.clone();
  let mut window = Window::new(move |step| {
    Column::new()
      .child(Text::new(&format!("step {step}")))
      .child(Element::component::<Rack>(CounterProps {
        name: "rack",
        book: its.clone(),
      }))
      .into()
  });
  book.count("b");
  window.pass();
  let before = ["a 0", "b 1", "c 0"].map(|text| node_of(&mut window, text));
  let rendered = ["a", "b", "c"].map(|name| book.renders(name));

  // The rack renders again by itself, with its counters in another order: none of them is rendered, each
  // is the node it was, and each keeps what it counted.
  book.count("rack");
  window.pass();
  assert!(window.shows("turn 1"));
  assert_eq!(["a 0", "b 1", "c 0"].map(|text| node_of(&mut window, text)), before);
  assert_eq!(["a", "b", "c"].map(|name| book.renders(name)), rendered);

  // And when the page above the rack renders again, which hands the rack back unchanged.
  window.step(1);
  assert!(window.shows("step 1") && window.shows("turn 1"));
  assert_eq!(["a 0", "b 1", "c 0"].map(|text| node_of(&mut window, text)), before);
  assert_eq!(["a", "b", "c"].map(|name| book.renders(name)), rendered);

  // One of them still renders by itself, where it stands now.
  book.count("c");
  window.pass();
  assert!(window.shows("c 1") && window.shows("a 0") && window.shows("b 1"));
  assert_eq!(book.log(), ["created a", "created b", "created c"]);
}

#[test]
fn one_keeps_what_it_is_where_it_moves_to_another_box() {
  let book = Arc::new(Book::default());
  let its = book.clone();
  // The counter is declared by the page and shown inside one box or another: its slot is the page's all
  // along, and the box it was in is gone from one render to the next.
  let mut window = Window::new(move |step| {
    let counter = counter("a", &its).key("a");
    match step % 2 {
      0 => Column::new().child(Row::new().id("left").child(counter)).into(),
      _ => Column::new()
        .child(Text::new("moved"))
        .child(Row::new().id("right").child(Row::new().child(counter)))
        .into(),
    }
  });
  book.count("a");
  window.pass();
  assert!(window.shows("a 1"));

  window.step(1);
  assert!(
    window.shows("moved") && window.shows("a 1"),
    "it is drawn where it stands now"
  );
  window.step(2);
  assert!(!window.shows("moved") && window.shows("a 1"));
  assert_eq!(book.log(), ["created a"]);
  assert_eq!(book.renders("a"), 2, "once at first and once for its own count");
}

#[test]
fn one_whose_root_is_dressed_otherwise_is_made_again_from_what_it_rendered_last() {
  let book = Arc::new(Book::default());
  let its = book.clone();
  let mut window = Window::new(move |step| {
    let class = if step == 0 { "first" } else { "second" };
    Column::new()
      .child(counter("a", &its).id("counter").class(class))
      .into()
  });
  book.count("a");
  window.pass();
  let classes = |window: &mut Window| {
    let element = window
      .tree
      .get_element_by_id("counter")
      .expect("the counter is found by its id");
    (
      element.text_content().map(str::to_owned),
      element.has_class("first"),
      element.has_class("second"),
    )
  };
  assert_eq!(classes(&mut window), (Some("a 1".to_owned()), true, false));

  // The page puts another class on its root: the nodes it had carry the old one, so it is put together
  // again from what it rendered last, and it is not rendered for that.
  window.step(1);
  assert_eq!(classes(&mut window), (Some("a 1".to_owned()), false, true));
  assert_eq!(book.renders("a"), 2, "once at first and once for its own count");
  assert_eq!(book.log(), ["created a"]);
}

/// What an app writes for a counter: a value that stands in a tree as the component `Counter`.
struct CounterTag {
  name: &'static str,
  key: Option<&'static str>,
  book: Arc<Book>,
}

impl lurq::app::component::IntoComponent for CounterTag {
  type Component = Counter;

  fn into_props(self) -> CounterProps {
    CounterProps {
      name: self.name,
      book: self.book,
    }
  }

  fn component_key(&self) -> Option<Arc<str>> {
    self.key.map(Arc::from)
  }
}

#[test]
fn a_value_that_is_a_component_converts_into_its_component_declared() {
  let book = Arc::new(Book::default());
  let its = book.clone();
  let mut window = Window::new(move |step| {
    let tag = |name: &'static str, key: Option<&'static str>| CounterTag {
      name,
      key,
      book: its.clone(),
    };
    let column = Column::new().child(Text::new(&format!("step {step}")));
    // Given as it is, with no `Ctx` and no conversion written: the two with keys change places.
    match step {
      0 => column
        .child(tag("plain", None))
        .child(tag("a", Some("a")))
        .child(tag("b", Some("b"))),
      _ => column
        .child(tag("plain", None))
        .child(tag("b", Some("b")))
        .child(tag("a", Some("a"))),
    }
    .into()
  });
  assert!(window.shows("plain 0") && window.shows("a 0") && window.shows("b 0"));
  book.count("plain");
  book.count("a");
  window.pass();
  assert!(window.shows("plain 1") && window.shows("a 1"));

  // What holds them renders again: each is the component it was, the keyed ones by their keys.
  window.step(1);
  assert!(window.shows("step 1") && window.shows("plain 1") && window.shows("a 1") && window.shows("b 0"));
  assert_eq!(book.log(), ["created plain", "created a", "created b"]);
}
