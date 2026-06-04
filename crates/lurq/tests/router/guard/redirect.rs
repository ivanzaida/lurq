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

struct RedirectRoot {
  router: RouterHandle,
}

impl Component for RedirectRoot {
  type Props = SharedRouter;

  fn create(ctx: &mut Ctx) -> Self {
    let router = ctx.router(
      Routes::new()
        .route("/login", |_ctx| Text::new("login").into())
        .route("/dashboard", |_ctx| Text::new("dashboard").into())
        .guard(|_match| GuardAction::Redirect("/login".to_owned())),
    );
    *ctx.props::<Self::Props>().0.lock().unwrap() = Some(router.clone());
    router.push("/dashboard");
    Self { router }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Router::mount(ctx, self.router.clone())
  }
}

#[test]
fn guard_redirect_navigates_to_target() {
  let router_out = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<RedirectRoot>(&mut app, SharedRouter(router_out.clone()));
  run_pass(&mut tree);

  let router = router_out.lock().unwrap().clone().unwrap();
  assert_eq!(&*router.path().get(), "/login");
  assert!(tree.find_element(|e| e.text_content() == Some("login")).is_some());
}

#[test]
fn redirect_does_not_create_intermediate_history_entry() {
  let router_out = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<RedirectRoot>(&mut app, SharedRouter(router_out.clone()));
  run_pass(&mut tree);

  let router = router_out.lock().unwrap().clone().unwrap();
  // only /login should be in history, not /dashboard
  assert!(!router.back());
}
