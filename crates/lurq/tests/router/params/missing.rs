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

struct NoParamPage;

impl Component for NoParamPage {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let params = ctx.route_params();
    let has_id = params.get("id").is_some();
    Text::new(&format!("has_id={}", has_id))
  }
}

struct MissingParamRoot {
  router: RouterHandle,
}

impl Component for MissingParamRoot {
  type Props = SharedRouter;

  fn create(ctx: &mut Ctx) -> Self {
    let router = ctx.router(Routes::new().route("/static", |ctx| ctx.mount::<NoParamPage>(())));
    *ctx.props::<Self::Props>().0.lock().unwrap() = Some(router.clone());
    router.push("/static");
    Self { router }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Router::mount(ctx, self.router.clone())
  }
}

#[test]
fn route_params_empty_on_static_route() {
  let router_out = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<MissingParamRoot>(&mut app, SharedRouter(router_out.clone()));
  run_pass(&mut tree);

  assert!(
    tree
      .find_element(|e| e.text_content() == Some("has_id=false"))
      .is_some()
  );
}

#[test]
fn route_params_returns_default_outside_router() {
  let params = lurq::router::Params::default();
  assert!(params.get("anything").is_none());
  assert!(params.is_empty());
}
