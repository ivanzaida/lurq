//! An overlay that takes the focus and is given another anchor while it is open, as the card of a tour goes from
//! one element to the next: it is the same overlay. It does not take the focus again, and as it closes the focus
//! goes back to what had it before it first opened.

use std::sync::Arc;

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Button, Column, Popup, Rect},
  core::{ElementRef, Signal},
  node::Element,
};

use super::{focused_id, tab};

/// What the test turns: which target the popup hangs from, and whether it is open.
#[derive(Clone, lurq::DevtoolsInspectable)]
struct Switches {
  #[devtools_ignore]
  at: Arc<Signal<usize>>,
  #[devtools_ignore]
  open: Arc<Signal<bool>>,
}

impl PartialEq for Switches {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.at, &other.at)
  }
}

/// A button, two targets, and one popup of two buttons that hangs from the target the app says.
struct Screen {
  at: Signal<usize>,
  open: Signal<bool>,
  targets: [ElementRef; 2],
}

impl Component for Screen {
  type Props = Switches;

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>();
    Self {
      at: (*props.at).clone(),
      open: (*props.open).clone(),
      targets: [ElementRef::new(), ElementRef::new()],
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let anchor = self.targets[self.at.get().min(1)].clone();
    Column::new()
      .size(400.0, 300.0)
      .child(Button::new("Start").id("start").tab_index(0))
      .child(Rect::new(80.0, 24.0).ref_element(self.targets[0].clone()))
      .child(Rect::new(80.0, 24.0).ref_element(self.targets[1].clone()))
      .child(
        Popup::new(
          anchor,
          Column::new()
            .child(Button::new("Back").id("back").tab_index(0))
            .child(Button::new("Next").id("next").tab_index(0)),
        )
        .open(self.open.clone())
        .takes_focus(true),
      )
  }
}

fn mounted() -> (Tree, App, Switches) {
  let switches = Switches {
    at: Arc::new(Signal::new(0)),
    open: Arc::new(Signal::new(false)),
  };
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<Screen>(&mut app, switches.clone());
  tree.pass_headless(&mut app);
  (tree, app, switches)
}

#[test]
fn an_overlay_given_another_anchor_keeps_the_focus_where_it_is_and_gives_it_back_as_it_first_would() {
  let (mut tree, mut app, switches) = mounted();
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("start"));

  switches.open.set(true);
  tree.pass_headless(&mut app);
  assert_eq!(
    focused_id(&tree).as_deref(),
    Some("back"),
    "it takes the focus as it opens"
  );
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("next"));

  // To the other target: the same popup, the focus where a person left it.
  switches.at.set(1);
  tree.pass_headless(&mut app);
  assert_eq!(
    focused_id(&tree).as_deref(),
    Some("next"),
    "it does not take the focus again"
  );
  switches.at.set(0);
  tree.pass_headless(&mut app);
  assert_eq!(focused_id(&tree).as_deref(), Some("next"));

  switches.at.set(1);
  tree.pass_headless(&mut app);
  switches.open.set(false);
  tree.pass_headless(&mut app);
  assert_eq!(
    focused_id(&tree).as_deref(),
    Some("start"),
    "back to what had the focus before it first opened"
  );
}

#[test]
fn an_overlay_opened_again_takes_the_focus_again() {
  let (mut tree, mut app, switches) = mounted();
  tab(&mut tree);
  switches.open.set(true);
  tree.pass_headless(&mut app);
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("next"));
  switches.open.set(false);
  tree.pass_headless(&mut app);
  assert_eq!(focused_id(&tree).as_deref(), Some("start"));

  // Closed and opened at the other target: it opens anew.
  switches.at.set(1);
  switches.open.set(true);
  tree.pass_headless(&mut app);
  assert_eq!(focused_id(&tree).as_deref(), Some("back"));
}
