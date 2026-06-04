use std::sync::{Arc, Mutex};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::Text,
  node::Element,
  router::{GuardAction, RouterHandle, Routes},
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

struct DenyRoot {
  router: RouterHandle,
}

impl Component for DenyRoot {
  type Props = SharedRouter;

  fn create(ctx: &mut Ctx) -> Self {
    let router = ctx.router(
      Routes::new()
        .route("/open", |_ctx| Text::new("open").into())
        .route("/locked", |_ctx| Text::new("locked").into())
        .guard(|_match| GuardAction::Deny)
        .fallback(|_ctx| Text::new("denied").into()),
    );
    *ctx.props::<Self::Props>().0.lock().unwrap() = Some(router.clone());
    router.push("/open");
    Self { router }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Router::mount(ctx, self.router.clone())
  }
}

#[test]
fn guard_that_returns_deny_blocks_route() {
  let router_out = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<DenyRoot>(&mut app, SharedRouter(router_out.clone()));
  run_pass(&mut tree);

  let router = router_out.lock().unwrap().clone().unwrap();
  router.push("/locked");
  run_pass(&mut tree);

  assert!(tree.find_element(|e| e.text_content() == Some("locked")).is_none());
  // stays on previous route or falls back
  assert!(
    tree.find_element(|e| e.text_content() == Some("open")).is_some()
      || tree.find_element(|e| e.text_content() == Some("denied")).is_some()
  );
}

#[test]
fn deny_does_not_add_to_history() {
  let router_out = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<DenyRoot>(&mut app, SharedRouter(router_out.clone()));
  run_pass(&mut tree);

  let router = router_out.lock().unwrap().clone().unwrap();
  router.push("/locked");
  run_pass(&mut tree);

  // path should remain /open since navigation was denied
  assert_eq!(&*router.path().get(), "/open");
}
