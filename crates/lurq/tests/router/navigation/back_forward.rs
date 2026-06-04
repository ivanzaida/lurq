use std::sync::{Arc, Mutex};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::Text,
  node::Element,
  router::{RouterHandle, Routes},
};

use crate::support::run_pass;

#[derive(lurq::DevtoolsInspectable)]
struct SharedRouter(Arc<Mutex<Option<RouterHandle>>>);

impl Clone for SharedRouter {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl PartialEq for SharedRouter {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

struct HistoryRoot {
  router: RouterHandle,
}

impl Component for HistoryRoot {
  type Props = SharedRouter;

  fn create(ctx: &mut Ctx) -> Self {
    let router = ctx.router(
      Routes::new()
        .route("/a", |_ctx| Text::new("a").into())
        .route("/b", |_ctx| Text::new("b").into())
        .route("/c", |_ctx| Text::new("c").into()),
    );
    *ctx.props::<Self::Props>().0.lock().unwrap() = Some(router.clone());
    router.push("/a");
    Self { router }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Router::mount(ctx, self.router.clone())
  }
}

fn setup() -> (Tree, RouterHandle) {
  let router_out = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<HistoryRoot>(&mut app, SharedRouter(router_out.clone()));
  run_pass(&mut tree);

  let router = router_out.lock().unwrap().clone().unwrap();
  (tree, router)
}

#[test]
fn back_returns_to_previous_route() {
  let (mut tree, router) = setup();

  router.push("/b");
  run_pass(&mut tree);
  assert!(tree.find_element(|e| e.text_content() == Some("b")).is_some());

  assert!(router.back());
  run_pass(&mut tree);
  assert!(tree.find_element(|e| e.text_content() == Some("a")).is_some());
}

#[test]
fn forward_restores_after_back() {
  let (mut tree, router) = setup();

  router.push("/b");
  run_pass(&mut tree);

  router.push("/c");
  run_pass(&mut tree);

  assert!(router.back());
  run_pass(&mut tree);
  assert!(tree.find_element(|e| e.text_content() == Some("b")).is_some());

  assert!(router.forward());
  run_pass(&mut tree);
  assert!(tree.find_element(|e| e.text_content() == Some("c")).is_some());
}

#[test]
fn back_returns_false_at_beginning_of_history() {
  let (_tree, router) = setup();
  assert!(!router.back());
}

#[test]
fn forward_returns_false_at_end_of_history() {
  let (_tree, router) = setup();
  assert!(!router.forward());
}

#[test]
fn push_after_back_discards_forward_history() {
  let (mut tree, router) = setup();

  router.push("/b");
  run_pass(&mut tree);

  router.push("/c");
  run_pass(&mut tree);

  assert!(router.back());
  run_pass(&mut tree);

  // push new route, should discard /c from forward stack
  router.push("/a");
  run_pass(&mut tree);

  assert!(!router.forward());
}

#[test]
fn multiple_back_steps() {
  let (mut tree, router) = setup();

  router.push("/b");
  run_pass(&mut tree);

  router.push("/c");
  run_pass(&mut tree);

  assert!(router.back());
  run_pass(&mut tree);
  assert!(tree.find_element(|e| e.text_content() == Some("b")).is_some());

  assert!(router.back());
  run_pass(&mut tree);
  assert!(tree.find_element(|e| e.text_content() == Some("a")).is_some());

  assert!(!router.back());
}
