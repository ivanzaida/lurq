//! The focus follows an element that a render moves under another parent: a text input, by the value it edits, and an
//! element with a ref, by its ref. Under another parent the element is a new node to the tree, with a new id. (Its
//! caret and its scroll are kept where it is the output of a component, which is reconciled with its own last output:
//! see the dock layout's tests.)

use std::sync::Arc;

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::MouseButton},
  components::{Button, Column, Row, TextInput},
  core::{ElementRef, Signal},
  node::Element,
};

use crate::support::{TestSurface, pointer_click};

/// What a scene shows: the control it builds, in the first column or moved into a row in the second.
#[derive(Clone)]
struct Props {
  moved: Signal<bool>,
  control: Arc<dyn Fn() -> Element + Send + Sync>,
}

impl PartialEq for Props {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for Props {}

struct Scene;

impl Component for Scene {
  type Props = Props;

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Props>().clone();
    let control = (props.control)();
    let first = Column::new().size(200.0, 40.0);
    let second = Column::new().size(200.0, 40.0);
    if props.moved.get() {
      Column::new()
        .child(first)
        .child(second.child(Row::new().child(control)))
    } else {
      Column::new().child(first.child(control)).child(second)
    }
  }
}

fn tree(app: &mut App, moved: &Signal<bool>, control: impl Fn() -> Element + Send + Sync + 'static) -> Tree {
  let mut tree = Tree::new();
  tree.mount_root::<Scene>(
    app,
    Props {
      moved: moved.clone(),
      control: Arc::new(control),
    },
  );
  tree.pass(app, &TestSurface);
  tree
}

fn focused(tree: &Tree) -> Option<String> {
  tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

fn press(tree: &mut Tree, app: &mut App, id: &str) {
  let (x, y) = tree
    .get_element_by_id_mut(id)
    .and_then(|element| element.bounds())
    .expect("element should be laid out")
    .center();
  pointer_click(tree, x, y, MouseButton::Left);
  tree.pass(app, &TestSurface);
}

fn key(tree: &mut Tree, app: &mut App, key: &str, code: &str) {
  tree.key_down(key.to_owned(), code.to_owned(), false, false, false);
  tree.key_up(key.to_owned(), code.to_owned(), false, false, false);
  tree.pass(app, &TestSurface);
}

#[test]
fn a_text_input_moved_under_another_parent_keeps_the_focus() {
  let mut app = App::new();
  let (value, moved) = (Signal::new("abc".to_owned()), Signal::new(false));
  let bound = value.clone();
  let mut tree = tree(&mut app, &moved, move || {
    TextInput::new(bound.clone()).id("field").size(180.0, 30.0).into()
  });
  press(&mut tree, &mut app, "field");
  assert_eq!(focused(&tree).as_deref(), Some("field"));
  key(&mut tree, &mut app, "End", "End");
  key(&mut tree, &mut app, "ArrowLeft", "ArrowLeft");
  moved.set(true);
  tree.pass(&mut app, &TestSurface);
  assert_eq!(focused(&tree).as_deref(), Some("field"), "the focus follows the input");
  key(&mut tree, &mut app, "X", "KeyX");
  assert!(value.get_untracked().contains('X'), "what is typed goes into it");
}

#[test]
fn an_element_with_a_ref_moved_under_another_parent_keeps_the_focus() {
  let mut app = App::new();
  let (at, moved) = (ElementRef::new(), Signal::new(false));
  let mut tree = tree(&mut app, &moved, move || {
    Button::new("Go")
      .id("go")
      .size(100.0, 30.0)
      .tab_index(0)
      .ref_element(at.clone())
      .into()
  });
  press(&mut tree, &mut app, "go");
  assert_eq!(focused(&tree).as_deref(), Some("go"));
  moved.set(true);
  tree.pass(&mut app, &TestSurface);
  assert_eq!(focused(&tree).as_deref(), Some("go"));
}

#[test]
fn an_element_with_nothing_to_find_it_by_loses_the_focus_as_before() {
  let mut app = App::new();
  let moved = Signal::new(false);
  let mut tree = tree(&mut app, &moved, || {
    Button::new("Plain").id("plain").size(100.0, 30.0).tab_index(0).into()
  });
  press(&mut tree, &mut app, "plain");
  assert_eq!(focused(&tree).as_deref(), Some("plain"));
  moved.set(true);
  tree.pass(&mut app, &TestSurface);
  assert_eq!(
    focused(&tree),
    None,
    "a new node, which nothing says is the same element"
  );
}
