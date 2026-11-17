//! Going back and forward as a person does (`NavigationCommand`): what is on the way takes it first (a dialog that
//! closes on it), and with nothing taking it the router mounted there, or elsewhere in the window, moves.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::Ctx,
    events::{HistoryEvent, MouseButton, NavigationCommand},
    window::WindowHandle,
  },
  components::{Column, Rect, Router, Row, Text},
  core::Signal,
  node::Element,
  router::{RouterHandle, Routes},
};

use crate::support::{pointer_click, run_pass};

thread_local! {
  /// The routers of this test's app, its window, whether the dialog of `/page` is open, and what passed the
  /// command on: each test runs on a thread of its own.
  static OUTER: std::cell::RefCell<Option<RouterHandle>> = const { std::cell::RefCell::new(None) };
  static INNER: std::cell::RefCell<Option<RouterHandle>> = const { std::cell::RefCell::new(None) };
  static WINDOW: std::cell::RefCell<Option<WindowHandle>> = const { std::cell::RefCell::new(None) };
  static DIALOG: Signal<bool> = Signal::new(false);
  static PASSED: Arc<Mutex<Vec<String>>> = Arc::default();
}

fn dialog() -> Signal<bool> {
  DIALOG.with(Clone::clone)
}

fn passed() -> Vec<String> {
  PASSED.with(|passed| std::mem::take(&mut *passed.lock().unwrap()))
}

/// A page with a dialog over it while `DIALOG` is set, which closes on going back instead of leaving the page.
struct Page;

impl Component for Page {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let open = dialog();
    let mut page = Column::new().size(300.0, 300.0).child(Text::new("page"));
    if open.get() {
      page = page.child(
        Column::new()
          .id("dialog")
          .focusable(true)
          .size(200.0, 100.0)
          .child(Text::new("dialog"))
          .on_history_back(move |_: HistoryEvent| open.set(false)),
      );
    }
    page
  }
}

/// A page with a router of its own, at `/one` and then `/two`.
struct Nested {
  router: RouterHandle,
}

impl Component for Nested {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let router = ctx.router(
      Routes::new()
        .route("/one", |_| Text::new("one").into())
        .route("/two", |_| {
          Column::new()
            .child(Text::new("two"))
            .child(Rect::new(100.0, 30.0).id("inner-screen").focusable(true))
            .into()
        }),
    );
    router.replace("/one");
    INNER.with(|inner| *inner.borrow_mut() = Some(router.clone()));
    Self { router }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    Column::new()
      .size(300.0, 300.0)
      .child(Rect::new(100.0, 30.0).id("inner-field").focusable(true))
      .child(Router::mount(ctx, self.router.clone()))
  }
}

/// A side bar beside the router, which a button that goes back from a handler is in.
struct Root {
  router: RouterHandle,
}

impl Component for Root {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let router = ctx.router(
      Routes::new()
        .route("/home", |_| Text::new("home").into())
        .route("/page", |ctx| ctx.mount::<Page>(()))
        .route("/inner", |ctx| ctx.mount::<Nested>(()))
        .route("/passing", |_| {
          Column::new()
            .id("passing")
            .focusable(true)
            .size(100.0, 100.0)
            .on_history_back(|event: HistoryEvent| {
              PASSED.with(|passed| passed.lock().unwrap().push("passing".into()));
              event.pass_on();
            })
            .into()
        }),
    );
    router.replace("/home");
    OUTER.with(|outer| *outer.borrow_mut() = Some(router.clone()));
    WINDOW.with(|window| *window.borrow_mut() = Some(ctx.window()));
    Self { router }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let history = ctx.history_runner();
    Row::new()
      .size(600.0, 300.0)
      .child(
        Column::new()
          .size(150.0, 300.0)
          .child(Rect::new(100.0, 30.0).id("side").focusable(true))
          .child(
            Rect::new(100.0, 30.0)
              .id("back-button")
              .on_click(move |_| history.back()),
          ),
      )
      .child(Router::mount(ctx, self.router.clone()))
  }
}

fn setup() -> (Tree, RouterHandle) {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(600, 300);
  tree.mount_root::<Root>(&mut app, ());
  run_pass(&mut tree);
  (tree, OUTER.with(|outer| outer.borrow().clone().unwrap()))
}

fn path(router: &RouterHandle) -> String {
  router.path().get_untracked()
}

fn go(tree: &mut Tree, router: &RouterHandle, to: &str) {
  router.push(to);
  run_pass(tree);
}

fn focus(tree: &mut Tree, id: &str) {
  tree.get_element_by_id_mut(id).unwrap().focus();
  run_pass(tree);
}

fn back_key(tree: &mut Tree) {
  if cfg!(any(target_os = "macos", target_os = "ios")) {
    tree.key_down_with_meta("[".into(), "BracketLeft".into(), false, false, false, true);
  } else {
    tree.key_down("ArrowLeft".into(), "ArrowLeft".into(), false, false, true);
  }
  run_pass(tree);
}

fn forward_key(tree: &mut Tree) {
  if cfg!(any(target_os = "macos", target_os = "ios")) {
    tree.key_down_with_meta("]".into(), "BracketRight".into(), false, false, false, true);
  } else {
    tree.key_down("ArrowRight".into(), "ArrowRight".into(), false, false, true);
  }
  run_pass(tree);
}

fn center(tree: &mut Tree, id: &str) -> (f32, f32) {
  tree
    .find_element(|element| element.id() == Some(id))
    .unwrap()
    .bounds()
    .center()
}

#[test]
fn with_nothing_taking_it_the_router_goes_back_and_forward() {
  let (mut tree, router) = setup();
  go(&mut tree, &router, "/page");

  back_key(&mut tree);
  assert_eq!(path(&router), "/home");
  forward_key(&mut tree);
  assert_eq!(path(&router), "/page");
}

#[test]
fn the_router_goes_back_when_the_focus_is_outside_it() {
  let (mut tree, router) = setup();
  go(&mut tree, &router, "/page");
  focus(&mut tree, "side");

  back_key(&mut tree);
  assert_eq!(
    path(&router),
    "/home",
    "the side bar is not in the router, which is asked after"
  );
}

#[test]
fn a_dialog_closes_on_going_back_instead_of_leaving_the_page() {
  let (mut tree, router) = setup();
  go(&mut tree, &router, "/page");
  dialog().set(true);
  run_pass(&mut tree);
  focus(&mut tree, "dialog");

  back_key(&mut tree);
  assert!(!dialog().get_untracked(), "the dialog took it");
  assert_eq!(path(&router), "/page", "and the page stayed");

  back_key(&mut tree);
  assert_eq!(path(&router), "/home", "with the dialog gone the router goes back");
}

#[test]
fn a_dialog_closes_on_the_mouses_back_button_over_it_and_with_nothing_focused() {
  let (mut tree, router) = setup();
  go(&mut tree, &router, "/page");
  dialog().set(true);
  run_pass(&mut tree);

  let (x, y) = center(&mut tree, "dialog");
  pointer_click(&mut tree, x, y, MouseButton::Back);
  run_pass(&mut tree);
  assert!(!dialog().get_untracked());
  assert_eq!(path(&router), "/page");

  dialog().set(true);
  run_pass(&mut tree);
  back_key(&mut tree);
  assert!(
    !dialog().get_untracked(),
    "with nothing focused the dialog is the last that hears it"
  );
  assert_eq!(path(&router), "/page");
}

#[test]
fn a_handler_that_passes_it_on_lets_the_router_go_back() {
  let (mut tree, router) = setup();
  go(&mut tree, &router, "/passing");
  focus(&mut tree, "passing");

  back_key(&mut tree);
  assert_eq!(passed(), ["passing"]);
  assert_eq!(path(&router), "/home");
}

#[test]
fn an_inner_router_goes_back_before_the_one_it_is_in() {
  let (mut tree, router) = setup();
  go(&mut tree, &router, "/inner");
  let inner = INNER.with(|inner| inner.borrow().clone().unwrap());
  go(&mut tree, &inner, "/two");
  focus(&mut tree, "inner-screen");

  back_key(&mut tree);
  assert_eq!(path(&inner), "/one");
  assert_eq!(path(&router), "/inner");

  focus(&mut tree, "inner-field");
  back_key(&mut tree);
  assert_eq!(path(&router), "/home", "the inner router has nowhere to go back to");
}

#[test]
fn the_focus_beside_an_inner_router_goes_back_in_the_one_it_is_in() {
  let (mut tree, router) = setup();
  go(&mut tree, &router, "/inner");
  let inner = INNER.with(|inner| inner.borrow().clone().unwrap());
  go(&mut tree, &inner, "/two");
  focus(&mut tree, "inner-field");

  back_key(&mut tree);
  assert_eq!(path(&router), "/home", "the field is not in the inner router's screen");
  assert_eq!(path(&inner), "/two");
}

#[test]
fn with_nothing_focused_the_innermost_router_goes_first() {
  let (mut tree, router) = setup();
  go(&mut tree, &router, "/inner");
  let inner = INNER.with(|inner| inner.borrow().clone().unwrap());
  go(&mut tree, &inner, "/two");

  back_key(&mut tree);
  assert_eq!(path(&inner), "/one");
  assert_eq!(path(&router), "/inner");
}

#[test]
fn a_button_goes_back_through_the_command_from_a_handler() {
  let (mut tree, router) = setup();
  go(&mut tree, &router, "/page");
  dialog().set(true);
  run_pass(&mut tree);
  focus(&mut tree, "dialog");

  let (x, y) = center(&mut tree, "back-button");
  pointer_click(&mut tree, x, y, MouseButton::Left);
  run_pass(&mut tree);
  assert!(
    !dialog().get_untracked(),
    "Ctx::history_back runs on the focus, as a key does: the dialog closes"
  );
  assert_eq!(path(&router), "/page");

  let (x, y) = center(&mut tree, "back-button");
  pointer_click(&mut tree, x, y, MouseButton::Left);
  run_pass(&mut tree);
  assert_eq!(path(&router), "/home");
}

#[test]
fn the_window_handle_goes_back_and_forward() {
  let (mut tree, router) = setup();
  go(&mut tree, &router, "/page");
  let window = WINDOW.with(|window| window.borrow().clone().unwrap());

  window.history_back();
  run_pass(&mut tree);
  assert_eq!(path(&router), "/home");
  window.history_forward();
  run_pass(&mut tree);
  assert_eq!(path(&router), "/page");
  window.run_navigation(NavigationCommand::Back);
  run_pass(&mut tree);
  assert_eq!(path(&router), "/home");
}

#[test]
fn the_mouses_side_buttons_move_the_router_from_anywhere() {
  let (mut tree, router) = setup();
  go(&mut tree, &router, "/page");
  let (x, y) = center(&mut tree, "side");

  pointer_click(&mut tree, x, y, MouseButton::Back);
  run_pass(&mut tree);
  assert_eq!(path(&router), "/home");
  pointer_click(&mut tree, x, y, MouseButton::Forward);
  run_pass(&mut tree);
  assert_eq!(path(&router), "/page");
}

#[test]
fn a_router_with_nowhere_to_go_takes_nothing() {
  let (mut tree, router) = setup();
  assert!(!tree.run_navigation(NavigationCommand::Back));
  assert_eq!(path(&router), "/home");
  assert!(!tree.run_navigation(NavigationCommand::Forward));
}
