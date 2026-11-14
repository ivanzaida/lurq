//! Where Tab goes on from when nothing has the focus, a browser's sequential focus navigation starting point:
//! the focus dropped by a key, by what had it leaving the tree or being disabled, or by a modal that closed with
//! nothing to give it back to leaves the place it was, and Tab and Shift+Tab go on from there in the order of
//! Tab, not from the top of the window. A press on what takes no focus puts the place there.

use std::sync::Arc;

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Button, Column, Modal, Rect, Root, Text, TextInput},
  core::Signal,
  node::Element,
};

use super::{click_id, focused_id, headless, shift_tab, tab};

fn press(tree: &mut Tree, key: &str) {
  tree.key_down(key.to_owned(), key.to_owned(), false, false, false);
  tree.key_up(key.to_owned(), key.to_owned(), false, false, false);
}

fn focus(tree: &mut Tree, id: &str) {
  for _ in 0..8 {
    if focused_id(tree).as_deref() == Some(id) {
      return;
    }
    tab(tree);
  }
  panic!("Tab never reached #{id}");
}

fn form() -> Column {
  Column::new()
    .child(Button::new("First").id("first").tab_index(0))
    .child(Button::new("Before").id("before").tab_index(0))
    .child(
      TextInput::new(Signal::new(String::new()))
        .id("field")
        .tab_index(0)
        .size(200.0, 30.0),
    )
    .child(Button::new("After").id("after").tab_index(0))
}

#[test]
fn enter_and_escape_in_a_field_leave_tab_going_on_from_the_field() {
  let mut tree = headless(form());
  focus(&mut tree, "field");
  press(&mut tree, "Enter");
  assert_eq!(focused_id(&tree), None, "Enter blurs the field");
  tab(&mut tree);
  assert_eq!(
    focused_id(&tree).as_deref(),
    Some("after"),
    "Tab goes on from the field"
  );

  focus(&mut tree, "field");
  press(&mut tree, "Escape");
  assert_eq!(focused_id(&tree), None, "Escape blurs the field");
  shift_tab(&mut tree);
  assert_eq!(
    focused_id(&tree).as_deref(),
    Some("before"),
    "Shift+Tab goes back from the field"
  );
}

#[test]
fn a_press_on_what_takes_no_focus_puts_the_place_there() {
  let mut tree = headless(
    Column::new()
      .child(Button::new("First").id("first").tab_index(0))
      .child(Button::new("Second").id("second").tab_index(0))
      .child(Text::new("Some words").id("words"))
      .child(Button::new("Third").id("third").tab_index(0))
      .child(Rect::new(200.0, 40.0)),
  );
  focus(&mut tree, "first");
  click_id(&mut tree, "words");
  assert_eq!(focused_id(&tree), None, "the press blurs the button");
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("third"));

  click_id(&mut tree, "words");
  shift_tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("second"));
}

#[derive(Clone, lurq::DevtoolsInspectable)]
struct Switches {
  shown: Arc<Signal<bool>>,
  disabled: Arc<Signal<bool>>,
  open: Arc<Signal<bool>>,
}

impl PartialEq for Switches {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.shown, &other.shown)
  }
}

impl std::fmt::Debug for Switches {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("Switches")
  }
}

/// `before`, `middle` (while shown; disabled while disabled), `after`, and a modal of two buttons.
struct Screen {
  shown: Signal<bool>,
  disabled: Signal<bool>,
  open: Signal<bool>,
}

impl Component for Screen {
  type Props = Switches;

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>();
    Self {
      shown: (*props.shown).clone(),
      disabled: (*props.disabled).clone(),
      open: (*props.open).clone(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let mut column = Column::new()
      .child(Button::new("First").id("first").tab_index(0))
      .child(Button::new("Before").id("before").tab_index(0));
    if self.shown.get() {
      column = column.child(
        Button::new("Middle")
          .id("middle")
          .tab_index(0)
          .disabled(self.disabled.get()),
      );
    }
    column.child(Button::new("After").id("after").tab_index(0)).child(
      Modal::new(
        Column::new()
          .child(Button::new("Cancel").id("cancel").tab_index(0))
          .child(Button::new("Confirm").id("confirm").tab_index(0)),
      )
      .open(self.open.clone())
      .target(Root)
      .takes_focus(true),
    )
  }
}

fn mounted() -> (Tree, App, Switches) {
  let switches = Switches {
    shown: Arc::new(Signal::new(true)),
    disabled: Arc::new(Signal::new(false)),
    open: Arc::new(Signal::new(false)),
  };
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<Screen>(&mut app, switches.clone());
  tree.pass_headless(&mut app);
  (tree, app, switches)
}

#[test]
fn what_had_the_focus_leaving_the_tree_leaves_its_place() {
  let (mut tree, mut app, switches) = mounted();
  focus(&mut tree, "middle");
  switches.shown.set(false);
  tree.pass_headless(&mut app);
  assert_eq!(focused_id(&tree), None);
  tab(&mut tree);
  assert_eq!(
    focused_id(&tree).as_deref(),
    Some("after"),
    "Tab goes to what took its place"
  );

  switches.shown.set(true);
  tree.pass_headless(&mut app);
  focus(&mut tree, "middle");
  switches.shown.set(false);
  tree.pass_headless(&mut app);
  shift_tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("before"));
}

#[test]
fn what_had_the_focus_being_disabled_leaves_its_place() {
  let (mut tree, mut app, switches) = mounted();
  focus(&mut tree, "middle");
  switches.disabled.set(true);
  tree.pass_headless(&mut app);
  assert_eq!(focused_id(&tree), None, "a disabled button keeps no focus");
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("after"));

  focus(&mut tree, "before");
  switches.disabled.set(false);
  tree.pass_headless(&mut app);
  focus(&mut tree, "middle");
  switches.disabled.set(true);
  tree.pass_headless(&mut app);
  shift_tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("before"));
}

#[test]
fn a_modal_closed_with_nothing_to_give_the_focus_back_to_leaves_tab_where_its_opener_was() {
  let (mut tree, mut app, switches) = mounted();
  focus(&mut tree, "middle");
  switches.open.set(true);
  tree.pass_headless(&mut app);
  assert_eq!(
    focused_id(&tree).as_deref(),
    Some("cancel"),
    "the modal takes the focus"
  );

  // What opened it goes while it is open: the focus has nothing to go back to as it closes.
  switches.shown.set(false);
  switches.open.set(false);
  for _ in 0..4 {
    tree.pass_headless(&mut app);
  }
  assert_eq!(focused_id(&tree), None);
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("after"));
}

#[test]
fn with_no_place_tab_starts_at_the_top_and_a_focus_given_forgets_the_place() {
  let mut tree = headless(form());
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("first"));

  focus(&mut tree, "field");
  press(&mut tree, "Enter");
  // A press on a button focuses it: the place of the field is forgotten.
  click_id(&mut tree, "before");
  assert_eq!(focused_id(&tree).as_deref(), Some("before"));
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("field"));
}
