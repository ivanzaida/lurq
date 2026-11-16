//! An overlay that holds Tab (`Overlay::traps_tab`): while the focus is in it Tab goes round its elements as it
//! does in a modal, Shift+Tab the other way, and it closes nothing behind it: Escape and a press outside close it
//! as they do, the focus goes back as it would, and with the focus on the page Tab goes through the page.

use std::sync::Arc;

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Button, Column, Popup},
  core::{ElementRef, Signal},
  node::Element,
};

use super::{click_id, focused_id, shift_tab, tab};

/// What the test turns: whether the popup is open, and whether it holds Tab.
#[derive(Clone, lurq::DevtoolsInspectable)]
struct Switches {
  open: Arc<Signal<bool>>,
  traps: Arc<Signal<bool>>,
}

impl PartialEq for Switches {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.open, &other.open)
  }
}

/// `before`, the `anchor`, `after`, and a popup of two buttons hung from the anchor.
struct Screen {
  open: Signal<bool>,
  traps: Signal<bool>,
  anchor: ElementRef,
}

impl Component for Screen {
  type Props = Switches;

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>();
    Self {
      open: (*props.open).clone(),
      traps: (*props.traps).clone(),
      anchor: ElementRef::new(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Column::new()
      .size(400.0, 300.0)
      .child(Button::new("Before").id("before").tab_index(0))
      .child(
        Button::new("Anchor")
          .id("anchor")
          .tab_index(0)
          .ref_element(self.anchor.clone()),
      )
      .child(Button::new("After").id("after").tab_index(0))
      .child(
        Popup::new(
          self.anchor.clone(),
          Column::new()
            .child(Button::new("One").id("one").tab_index(0))
            .child(Button::new("Two").id("two").tab_index(0)),
        )
        .open(self.open.clone())
        .dismiss_on_escape(true)
        .dismiss_on_outside_click(true)
        .takes_focus(true)
        .traps_tab(self.traps.get()),
      )
  }
}

fn mounted(traps: bool) -> (Tree, App, Switches) {
  let switches = Switches {
    open: Arc::new(Signal::new(false)),
    traps: Arc::new(Signal::new(traps)),
  };
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<Screen>(&mut app, switches.clone());
  tree.pass_headless(&mut app);
  (tree, app, switches)
}

/// Opens the popup with the focus on its anchor.
fn opened(traps: bool) -> (Tree, App, Switches) {
  let (mut tree, mut app, switches) = mounted(traps);
  tab(&mut tree);
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("anchor"));
  switches.open.set(true);
  tree.pass_headless(&mut app);
  assert_eq!(
    focused_id(&tree).as_deref(),
    Some("one"),
    "it takes the focus as it opens"
  );
  (tree, app, switches)
}

#[test]
fn tab_goes_round_an_overlay_that_holds_it_and_shift_tab_the_other_way() {
  let (mut tree, ..) = opened(true);
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("two"));
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("one"), "from its last to its first");
  shift_tab(&mut tree);
  assert_eq!(
    focused_id(&tree).as_deref(),
    Some("two"),
    "and from its first to its last"
  );
  shift_tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("one"));
}

#[test]
fn an_overlay_that_does_not_hold_tab_lets_it_go_on_after_its_anchor() {
  let (mut tree, ..) = opened(false);
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("two"));
  tab(&mut tree);
  assert_eq!(
    focused_id(&tree).as_deref(),
    Some("after"),
    "on to what follows the anchor"
  );
  shift_tab(&mut tree);
  shift_tab(&mut tree);
  shift_tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("anchor"), "and back out before it");
}

#[test]
fn escape_closes_an_overlay_that_holds_tab_and_the_focus_goes_back() {
  let (mut tree, mut app, switches) = opened(true);
  tab(&mut tree);
  tree.key_down("Escape".to_owned(), "Escape".to_owned(), false, false, false);
  tree.pass_headless(&mut app);
  assert!(!switches.open.get(), "Escape closes it as it does any overlay");
  assert_eq!(
    focused_id(&tree).as_deref(),
    Some("anchor"),
    "back to what had the focus as it opened"
  );
  // Closed, it holds nothing.
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("after"));
}

#[test]
fn a_press_outside_closes_it_as_it_does_any_overlay_and_the_focus_goes_back() {
  let (mut tree, mut app, switches) = opened(true);
  click_id(&mut tree, "before");
  tree.pass_headless(&mut app);
  assert!(
    !switches.open.get(),
    "a press outside closes it: nothing behind it is closed to the pointer"
  );
  assert_eq!(
    focused_id(&tree).as_deref(),
    Some("anchor"),
    "back to what had the focus as it opened"
  );
}

#[test]
fn with_the_focus_on_the_page_tab_goes_through_the_page_and_is_held_once_in_the_overlay() {
  let (mut tree, mut app, switches) = mounted(true);
  switches.open.set(true);
  tree.pass_headless(&mut app);
  // Nothing had the focus as it opened: it took it. A person puts it back on the page.
  tree.get_element_by_id_mut("before").unwrap().focus();
  assert_eq!(focused_id(&tree).as_deref(), Some("before"));
  assert!(switches.open.get());
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("anchor"), "the page's own order");
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("one"), "into it after its anchor");
  tab(&mut tree);
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("one"), "and held there");
}
