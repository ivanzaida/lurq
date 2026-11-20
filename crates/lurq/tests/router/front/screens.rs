//! `Ctx::is_in_front`: a screen, and what is mounted inside it, reads whether it is the one in front: the top of
//! its stack and, where the stack is in a set of tabs, in the tab that is shown. What plays in a screen is
//! paused with it.

use std::{
  cell::RefCell,
  collections::HashMap,
  sync::{Arc, Mutex},
  time::{Duration, Instant},
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::TouchPhase},
  components::{Column, NavigationItem, Router, StackOutlet, TabItem, TabOutlet, TabOutletProps, Tabs, Text},
  node::Element,
  router::{RouterHandle, Routes},
};

use crate::support::run_pass;

thread_local! {
  /// What each screen, and the component two deep inside it, last read of being in front, and each change of
  /// it in the order they read it. A test runs on a thread of its own.
  static SAID: RefCell<HashMap<(&'static str, bool), bool>> = RefCell::new(HashMap::new());
  static TOLD: RefCell<Vec<(&'static str, bool)>> = const { RefCell::new(Vec::new()) };
}

fn say(name: &'static str, deep: bool, front: bool) {
  let before = SAID.with(|said| said.borrow_mut().insert((name, deep), front));
  if !deep && before != Some(front) {
    TOLD.with(|told| told.borrow_mut().push((name, front)));
  }
}

/// What the screen called `name` last read, and the component inside it: `None` where it was never rendered.
fn said(name: &'static str) -> Option<(bool, bool)> {
  SAID.with(|said| {
    let said = said.borrow();
    Some((*said.get(&(name, false))?, *said.get(&(name, true))?))
  })
}

/// The changes the screens read since the last call, in order.
fn told() -> Vec<(&'static str, bool)> {
  TOLD.with(|told| std::mem::take(&mut *told.borrow_mut()))
}

const IN_FRONT: Option<(bool, bool)> = Some((true, true));
const NOT_IN_FRONT: Option<(bool, bool)> = Some((false, false));

#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct Named(&'static str);

/// A screen that reads whether it is in front.
struct Screen;

impl Component for Screen {
  type Props = Named;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let named = ctx.props::<Named>().clone();
    let front = ctx.is_in_front();
    say(named.0, false, front);
    // A screen of a stack is taken back by a finger from the edge once it has said what it is.
    ctx.navigation_item(NavigationItem::new(named.0));
    Column::new()
      .child(Text::new(named.0))
      .child(ctx.mount::<Holder>(named))
  }
}

/// A component of a screen that reads nothing of it, with one inside that does.
struct Holder;

impl Component for Holder {
  type Props = Named;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let named = ctx.props::<Named>().clone();
    Column::new().child(ctx.mount::<Player>(named))
  }
}

/// What plays in a screen, two components deep: it reads whether its screen is in front.
struct Player;

impl Component for Player {
  type Props = Named;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let name = ctx.props::<Named>().0;
    let front = ctx.is_in_front();
    say(name, true, front);
    Text::new(if front { "plays" } else { "paused" })
  }
}

/// What a test's root is made of: a stack alone, or a set of tabs whose first is that stack.
#[derive(Clone, Copy, PartialEq)]
enum Shape {
  Stack,
  Tabs,
}

#[derive(Clone)]
struct Made {
  stack: RouterHandle,
  tabs: Tabs,
}

struct Shared(Arc<Mutex<Option<Made>>>, Shape);

impl lurq::app::component::DevtoolsInspectable for Shared {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

impl PartialEq for Shared {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

struct Root {
  made: Made,
  shape: Shape,
}

impl Component for Root {
  type Props = Shared;

  fn create(ctx: &mut Ctx) -> Self {
    let stack = ctx.router(Routes::new().layout("/", StackOutlet::mount, |routes| {
      routes
        .route("/first", |ctx| ctx.mount::<Screen>(Named("first")))
        .route("/second", |ctx| ctx.mount::<Screen>(Named("second")))
        .route("/third", |ctx| ctx.mount::<Screen>(Named("third")))
    }));
    stack.replace("/first");
    let other = ctx.router(Routes::new().route("/", |ctx| ctx.mount::<Screen>(Named("other"))));
    other.replace("/");
    let tabs = Tabs::new();
    tabs
      .add("stack", TabItem::new("Stack"), stack.clone())
      .add("other", TabItem::new("Other"), other);
    let made = Made { stack, tabs };
    let props = ctx.props::<Shared>();
    *props.0.lock().unwrap() = Some(made.clone());
    Self { made, shape: props.1 }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    match self.shape {
      Shape::Stack => Router::mount(ctx, self.made.stack.clone()),
      Shape::Tabs => TabOutlet::mount(ctx, TabOutletProps::new(self.made.tabs.clone())),
    }
  }
}

fn setup(shape: Shape) -> (Tree, Made) {
  let out = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(400, 800);
  tree.mount_root::<Root>(&mut app, Shared(out.clone(), shape));
  run_pass(&mut tree);
  let made = out.lock().unwrap().clone().unwrap();
  (tree, made)
}

thread_local! {
  /// How far ahead of now the animations of this test's tree are taken to be.
  static AHEAD: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Lets a move of the stack end: its timers fire, and every transition is at its end.
fn settle(tree: &mut Tree) {
  run_pass(tree);
  tree.tick_timers_at(Instant::now() + Duration::from_secs(1));
  run_pass(tree);
  tree.tick_timers_at(Instant::now() + Duration::from_secs(2));
  run_pass(tree);
  let ahead = AHEAD.with(|ahead| {
    ahead.set(ahead.get() + 10);
    ahead.get()
  });
  tree.set_animation_clock(Some(Instant::now() + Duration::from_secs(ahead)));
  run_pass(tree);
  run_pass(tree);
}

fn shows(tree: &mut Tree, text: &str) -> bool {
  tree
    .find_element(|element| element.text_content() == Some(text))
    .is_some()
}

#[test]
fn the_top_screen_of_a_stack_is_in_front_and_the_one_under_it_is_not() {
  let (mut tree, made) = setup(Shape::Stack);
  assert_eq!(said("first"), IN_FRONT, "alone in its stack");
  assert_eq!(told(), [("first", true)]);

  // Pushed, the new screen is in front from its first frame, with the one under it still seen beside it.
  made.stack.push("/second");
  run_pass(&mut tree);
  assert!(shows(&mut tree, "first") && shows(&mut tree, "second"));
  assert_eq!(said("second"), IN_FRONT);
  assert_eq!(said("first"), NOT_IN_FRONT, "and what is two components deep in it");
  settle(&mut tree);
  assert_eq!((said("first"), said("second")), (NOT_IN_FRONT, IN_FRONT));
  let mut heard = told();
  heard.sort();
  assert_eq!(heard, [("first", false), ("second", true)], "each told once");

  // Back: the screen under is in front from the moment the stack goes back, and the one that leaves is not.
  made.stack.back();
  run_pass(&mut tree);
  assert_eq!((said("first"), said("second")), (IN_FRONT, NOT_IN_FRONT));
  settle(&mut tree);
  assert!(!shows(&mut tree, "second"));
  assert_eq!(said("first"), IN_FRONT);
}

#[test]
fn a_screen_two_under_the_top_hears_it_though_it_was_never_beside_it() {
  let (mut tree, made) = setup(Shape::Stack);
  // Two pushed with no frame between: the first screen is neither the top nor the one under it.
  made.stack.push("/second");
  made.stack.push("/third");
  settle(&mut tree);
  assert_eq!(said("third"), IN_FRONT);
  assert_eq!(said("first"), NOT_IN_FRONT, "rendered once more as it left the stage");
}

#[test]
fn the_screen_under_a_finger_s_drag_is_not_in_front_until_the_stack_goes_back() {
  let (mut tree, made) = setup(Shape::Stack);
  made.stack.push("/second");
  settle(&mut tree);
  told();

  // A finger from the left edge has the top screen half way across: the one under it is seen.
  let mut at = Instant::now();
  let mut finger = |tree: &mut Tree, phase: TouchPhase, x: f32| {
    at += Duration::from_millis(30);
    tree.set_press_clock(Some(at));
    tree.finger(1, phase, x, 300.0, None);
  };
  finger(&mut tree, TouchPhase::Start, 2.0);
  for step in 1..=10 {
    finger(&mut tree, TouchPhase::Move, 2.0 + 20.0 * step as f32);
    run_pass(&mut tree);
  }
  assert!(shows(&mut tree, "first") && shows(&mut tree, "second"));
  assert_eq!(
    (said("first"), said("second")),
    (NOT_IN_FRONT, IN_FRONT),
    "the top screen is the one in front while it is dragged"
  );
  // Let go short of going back, slowly: nothing changed.
  for step in 1..=10 {
    finger(&mut tree, TouchPhase::Move, 202.0 - 12.0 * step as f32);
  }
  finger(&mut tree, TouchPhase::End, 82.0);
  settle(&mut tree);
  assert_eq!(made.stack.path().get_untracked(), "/second");
  assert_eq!(told(), [], "nobody was told anything");

  // Let go far enough: the stack goes back, and the screen under is in front.
  finger(&mut tree, TouchPhase::Start, 2.0);
  for step in 1..=10 {
    finger(&mut tree, TouchPhase::Move, 2.0 + 30.0 * step as f32);
  }
  finger(&mut tree, TouchPhase::End, 302.0);
  run_pass(&mut tree);
  assert_eq!(made.stack.path().get_untracked(), "/first");
  assert_eq!(
    (said("first"), said("second")),
    (IN_FRONT, NOT_IN_FRONT),
    "from the moment the finger let go"
  );
  settle(&mut tree);
  assert_eq!(said("first"), IN_FRONT);
}

#[test]
fn a_tab_that_is_no_longer_shown_hears_it_as_it_leaves() {
  let (mut tree, made) = setup(Shape::Tabs);
  assert_eq!(said("first"), IN_FRONT);
  assert_eq!(said("other"), None, "a tab never shown has rendered nothing");

  made.tabs.select("other");
  run_pass(&mut tree);
  assert!(shows(&mut tree, "other") && !shows(&mut tree, "first"));
  assert_eq!(said("other"), IN_FRONT);
  assert_eq!(
    said("first"),
    NOT_IN_FRONT,
    "rendered once more as it left the stage, and what is two components deep in it"
  );
  assert_eq!(told(), [("first", true), ("first", false), ("other", true)]);

  made.tabs.select("stack");
  run_pass(&mut tree);
  assert_eq!((said("first"), said("other")), (IN_FRONT, NOT_IN_FRONT));
  assert_eq!(told(), [("first", true), ("other", false)]);
}

#[test]
fn a_screen_is_in_front_where_it_is_the_top_of_its_stack_and_its_tab_is_shown() {
  let (mut tree, made) = setup(Shape::Tabs);
  made.stack.push("/second");
  settle(&mut tree);
  assert_eq!((said("first"), said("second")), (NOT_IN_FRONT, IN_FRONT));

  // Another tab: the top of the stack is not in front, and the screen under it is as it was.
  made.tabs.select("other");
  run_pass(&mut tree);
  assert_eq!((said("first"), said("second")), (NOT_IN_FRONT, NOT_IN_FRONT));

  // The stack goes back while its tab is not shown: nobody is in front there, and nothing is rendered.
  made.stack.back();
  settle(&mut tree);
  assert_eq!(said("first"), NOT_IN_FRONT);
  told();

  // Shown again, its top screen is in front.
  made.tabs.select("stack");
  settle(&mut tree);
  assert_eq!((said("first"), said("other")), (IN_FRONT, NOT_IN_FRONT));
  assert_eq!(told(), [("first", true), ("other", false)]);
}
