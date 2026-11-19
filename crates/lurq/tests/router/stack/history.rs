//! A `StackOutlet` goes back through the command (`NavigationCommand`): its way back and a finger from the left
//! edge go back from the top screen, which hears it first (`on_history_back`), and the stack's router after.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::Ctx,
    events::{HistoryEvent, MouseButton, TouchPhase},
  },
  components::{Column, NavigationBar, NavigationItem, Router, StackOutlet, StackOutletProps, Text},
  core::Signal,
  node::Element,
  router::{RouterHandle, Routes},
  time::{Duration, Instant},
};

use crate::support::{pointer_click, run_pass};

thread_local! {
  static ROUTER: std::cell::RefCell<Option<RouterHandle>> = const { std::cell::RefCell::new(None) };
  /// Whether the screen at `/unsaved` has something unsaved, which it asks about before it is left.
  static UNSAVED: Signal<bool> = Signal::new(true);
  static ASKED: Arc<Mutex<usize>> = Arc::default();
}

struct Root {
  router: RouterHandle,
}

impl Component for Root {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let router = ctx.router(Routes::new().layout(
      "/",
      |ctx| StackOutlet::mount_with(ctx, StackOutletProps::new().bar(NavigationBar::new())),
      |routes| {
        routes
          .route("/first", |ctx| {
            ctx.navigation_item(NavigationItem::new("Alpha"));
            Text::new("first screen").into()
          })
          .route("/second", |ctx| {
            ctx.navigation_item(NavigationItem::new("Beta"));
            Text::new("second screen").into()
          })
          .route("/unsaved", |ctx| {
            ctx.navigation_item(NavigationItem::new("Draft"));
            let unsaved = UNSAVED.with(Clone::clone);
            Column::new()
              .size(200.0, 200.0)
              .child(Text::new("unsaved screen"))
              .on_history_back(move |event: HistoryEvent| {
                if unsaved.get_untracked() {
                  // Asks first, and stays.
                  ASKED.with(|asked| *asked.lock().unwrap() += 1);
                } else {
                  event.pass_on();
                }
              })
              .into()
          })
      },
    ));
    router.replace("/first");
    ROUTER.with(|held| *held.borrow_mut() = Some(router.clone()));
    Self { router }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    Router::mount(ctx, self.router.clone())
  }
}

/// The tree, its router and the moment its clocks start at.
fn setup() -> (Tree, RouterHandle, Instant) {
  let start = Instant::now();
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(400, 800);
  tree.set_timer_clock(Some(start));
  tree.set_animation_clock(Some(start));
  tree.set_press_clock(Some(start));
  tree.mount_root::<Root>(&mut app, ());
  run_pass(&mut tree);
  (tree, ROUTER.with(|held| held.borrow().clone().unwrap()), start)
}

/// Lets a move of the stack end: its timers fire, and every transition is at its end. The clocks go on from `at`.
fn settle(tree: &mut Tree, at: &mut Instant) {
  for _ in 0..3 {
    run_pass(tree);
    *at += Duration::from_secs(2);
    tree.set_timer_clock(Some(*at));
    tree.set_animation_clock(Some(*at));
    tree.tick_timers_at(*at);
  }
  run_pass(tree);
}

fn shows(tree: &mut Tree, text: &str) -> bool {
  tree
    .find_element(|element| element.text_content() == Some(text))
    .is_some()
}

fn press_the_way_back(tree: &mut Tree) {
  let (x, y) = tree
    .find_element(|element| element.id() == Some("navigation-back"))
    .expect("the way back")
    .bounds()
    .center();
  pointer_click(tree, x, y, MouseButton::Left);
  run_pass(tree);
}

/// A finger lands at the left edge and goes 300 across before it lifts, on the tree's clocks.
fn swipe_from_the_left_edge(tree: &mut Tree, at: &mut Instant) {
  tree.set_press_clock(Some(*at));
  tree.finger(1, TouchPhase::Start, 2.0, 300.0, None);
  for step in 1..=10 {
    *at += Duration::from_millis(30);
    tree.set_press_clock(Some(*at));
    tree.finger(1, TouchPhase::Move, 2.0 + 30.0 * step as f32, 300.0, None);
  }
  tree.finger(1, TouchPhase::End, 302.0, 300.0, None);
}

#[test]
fn the_way_back_goes_back_through_the_command() {
  let (mut tree, router, mut at) = setup();
  router.push("/second");
  settle(&mut tree, &mut at);

  press_the_way_back(&mut tree);
  settle(&mut tree, &mut at);
  assert_eq!(router.path().get_untracked(), "/first");
  assert!(shows(&mut tree, "first screen"));
}

#[test]
fn a_screen_that_takes_going_back_keeps_the_way_back_from_leaving_it() {
  let (mut tree, router, mut at) = setup();
  UNSAVED.with(|unsaved| unsaved.set(true));
  router.push("/unsaved");
  settle(&mut tree, &mut at);
  let asked = ASKED.with(|asked| *asked.lock().unwrap());

  press_the_way_back(&mut tree);
  settle(&mut tree, &mut at);
  assert_eq!(router.path().get_untracked(), "/unsaved");
  assert_eq!(
    ASKED.with(|asked| *asked.lock().unwrap()),
    asked + 1,
    "the screen asked"
  );

  UNSAVED.with(|unsaved| unsaved.set(false));
  press_the_way_back(&mut tree);
  settle(&mut tree, &mut at);
  assert_eq!(router.path().get_untracked(), "/first", "saved, it is left");
}

#[test]
fn a_finger_from_the_edge_still_takes_the_top_screen_back() {
  let (mut tree, router, mut at) = setup();
  router.push("/second");
  settle(&mut tree, &mut at);

  swipe_from_the_left_edge(&mut tree, &mut at);
  settle(&mut tree, &mut at);
  assert_eq!(router.path().get_untracked(), "/first");
  assert!(shows(&mut tree, "first screen"));
  assert!(!shows(&mut tree, "second screen"));
}

#[test]
fn a_screen_that_takes_the_finger_from_the_edge_comes_back() {
  let (mut tree, router, mut at) = setup();
  UNSAVED.with(|unsaved| unsaved.set(true));
  router.push("/unsaved");
  settle(&mut tree, &mut at);
  let asked = ASKED.with(|asked| *asked.lock().unwrap());

  swipe_from_the_left_edge(&mut tree, &mut at);
  settle(&mut tree, &mut at);
  assert_eq!(router.path().get_untracked(), "/unsaved");
  assert_eq!(ASKED.with(|asked| *asked.lock().unwrap()), asked + 1);
  assert!(shows(&mut tree, "unsaved screen"), "the screen came back");
  assert!(!shows(&mut tree, "first screen"), "and the one under is not shown");
}

#[test]
fn a_key_goes_back_in_the_stack() {
  let (mut tree, router, mut at) = setup();
  router.push("/second");
  settle(&mut tree, &mut at);

  if cfg!(any(target_os = "macos", target_os = "ios")) {
    tree.key_down_with_meta("[".into(), "BracketLeft".into(), false, false, false, true);
  } else {
    tree.key_down("ArrowLeft".into(), "ArrowLeft".into(), false, false, true);
  }
  settle(&mut tree, &mut at);
  assert_eq!(router.path().get_untracked(), "/first");
}
