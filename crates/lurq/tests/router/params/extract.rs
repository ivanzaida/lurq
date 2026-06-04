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

struct UserPage;

impl Component for UserPage {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let id = ctx.route_params().get("id").unwrap_or("none").to_owned();
    Text::new(&format!("user-{}", id))
  }
}

struct ParamRoot {
  router: RouterHandle,
}

impl Component for ParamRoot {
  type Props = SharedRouter;

  fn create(ctx: &mut Ctx) -> Self {
    let router = ctx.router(Routes::new().route("/users/:id", |ctx| ctx.mount::<UserPage>(())));
    *ctx.props::<Self::Props>().0.lock().unwrap() = Some(router.clone());
    router.push("/users/42");
    Self { router }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Router::mount(ctx, self.router.clone())
  }
}

#[test]
fn route_params_available_in_component() {
  let router_out = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<ParamRoot>(&mut app, SharedRouter(router_out.clone()));
  run_pass(&mut tree);

  assert!(tree.find_element(|e| e.text_content() == Some("user-42")).is_some());
}

#[test]
fn params_update_on_navigation_to_same_pattern() {
  let router_out = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<ParamRoot>(&mut app, SharedRouter(router_out.clone()));
  run_pass(&mut tree);

  let router = router_out.lock().unwrap().clone().unwrap();
  router.push("/users/99");
  run_pass(&mut tree);

  assert!(tree.find_element(|e| e.text_content() == Some("user-99")).is_some());
  assert!(tree.find_element(|e| e.text_content() == Some("user-42")).is_none());
}
