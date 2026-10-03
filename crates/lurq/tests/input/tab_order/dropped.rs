//! A child its Row dropped to make room (`ShrinkLimit::Drop`) is not drawn:
//! nothing inside it is a Tab stop or takes focus, focus leaves it when it
//! drops, and keys never activate it.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Button, Rect, Row},
  core::{ElementRef, Signal},
  layout::{Constraints, Size, layout_kind::ShrinkLimit},
  node::Element,
};

use super::{click_id, focused_id, headless, tab};

fn extra_button() -> Button {
  Button::new("Extra").id("extra").tab_index(0)
}

/// A toolbar: a button, a droppable group with `extra`, and a fixed 150 px
/// block that leaves no room for the group in a narrow bar.
fn toolbar_with(extra: Button) -> Row {
  Row::new()
    .child(Button::new("Keep").id("keep").tab_index(0))
    .child(Row::new().child(extra).flex_shrink(1.0).shrink_limit(ShrinkLimit::Drop))
    .child(Rect::new(150.0, 10.0))
}

fn toolbar() -> Row {
  toolbar_with(extra_button())
}

/// Lays the tree out filling a `width` wide window.
fn layout_at(tree: &mut Tree, app: &mut App, width: f32) {
  tree.set_layout_constraints_override(Some(Constraints::tight(Size::new(width, 40.0))));
  tree.pass_headless(app);
}

fn press(tree: &mut Tree, key: &str, code: &str) {
  tree.key_down(key.to_owned(), code.to_owned(), false, false, false);
  tree.key_up(key.to_owned(), code.to_owned(), false, false, false);
}

/// Presses Enter and Space and returns how many clicks the extra button got.
fn activations(tree: &mut Tree, clicks: &AtomicUsize) -> usize {
  press(tree, "Enter", "Enter");
  press(tree, " ", "Space");
  clicks.load(Ordering::SeqCst)
}

fn tab_order(tree: &mut Tree, stops: usize) -> Vec<String> {
  (0..stops)
    .map(|_| {
      tab(tree);
      focused_id(tree).unwrap_or_default()
    })
    .collect()
}

fn extra_width(tree: &mut Tree) -> f32 {
  tree
    .get_element_by_id_mut("extra")
    .and_then(|element| element.bounds())
    .expect("the extra button has bounds")
    .width
}

#[test]
fn tab_visits_a_child_that_fits() {
  let mut tree = headless(toolbar().width(700.0));
  assert_eq!(tab_order(&mut tree, 3), ["keep", "extra", "keep"]);
}

#[test]
fn tab_skips_a_dropped_child() {
  let mut tree = headless(toolbar().width(200.0));
  assert_eq!(
    extra_width(&mut tree),
    0.0,
    "the dropped button reports a zero-size rect"
  );
  assert_eq!(tab_order(&mut tree, 2), ["keep", "keep"]);
}

#[test]
fn focus_leaves_a_child_when_it_drops() {
  // The same tree, filling a narrower window: only the layout drops the group.
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(toolbar());
  layout_at(&mut tree, &mut app, 700.0);
  click_id(&mut tree, "extra");
  assert_eq!(focused_id(&tree).as_deref(), Some("extra"));

  layout_at(&mut tree, &mut app, 200.0);
  assert_eq!(extra_width(&mut tree), 0.0, "the group is dropped");
  assert_eq!(focused_id(&tree), None, "a dropped button keeps no focus");
  tab(&mut tree);
  assert_eq!(focused_id(&tree).as_deref(), Some("keep"));
}

fn counting_toolbar(clicks: &Arc<AtomicUsize>) -> Row {
  let counter = clicks.clone();
  toolbar_with(extra_button().on_click(move |_: lurq::app::events::MouseEvent| {
    counter.fetch_add(1, Ordering::SeqCst);
  }))
}

#[test]
fn focus_on_a_dropped_child_is_refused_and_keys_do_not_activate_it() {
  let clicks = Arc::new(AtomicUsize::new(0));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(counting_toolbar(&clicks));

  layout_at(&mut tree, &mut app, 700.0);
  tree.get_element_by_id_mut("extra").expect("extra").focus();
  assert_eq!(
    focused_id(&tree).as_deref(),
    Some("extra"),
    "a fitting button takes focus"
  );
  assert_eq!(activations(&mut tree, &clicks), 2, "Enter and Space activate it");

  layout_at(&mut tree, &mut app, 200.0);
  tree.get_element_by_id_mut("extra").expect("extra").focus();
  assert_eq!(focused_id(&tree), None, "a dropped button refuses focus");
  assert_eq!(activations(&mut tree, &clicks), 2, "Enter and Space do not reach it");
}

struct Refocus {
  reference: ElementRef,
  blurs: Signal<u32>,
}

#[derive(Clone, lurq::DevtoolsInspectable)]
struct RefocusProps(#[devtools_ignore] Arc<Refocus>);

impl PartialEq for RefocusProps {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

/// A toolbar whose extra button asks for focus again whenever it loses it.
struct RefocusingToolbar;

impl Component for RefocusingToolbar {
  type Props = RefocusProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let state = ctx.props::<Self::Props>().0.clone();
    if state.blurs.get() > 0 {
      ctx.focus(&state.reference);
    }
    let blurs = state.blurs.clone();
    toolbar_with(
      extra_button()
        .ref_element(state.reference.clone())
        .on_blur(move || blurs.set(blurs.get() + 1)),
    )
  }
}

#[test]
fn refocusing_a_dropped_child_from_its_blur_handler_does_not_loop() {
  let state = Arc::new(Refocus {
    reference: ElementRef::new(),
    blurs: Signal::new(0),
  });
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<RefocusingToolbar>(&mut app, RefocusProps(state.clone()));
  layout_at(&mut tree, &mut app, 700.0);
  click_id(&mut tree, "extra");
  assert_eq!(focused_id(&tree).as_deref(), Some("extra"));

  for _ in 0..5 {
    layout_at(&mut tree, &mut app, 200.0);
  }
  assert_eq!(state.blurs.get(), 1, "the button is blurred once when it drops");
  assert_eq!(
    focused_id(&tree),
    None,
    "the focus request for the dropped button is refused"
  );
}
