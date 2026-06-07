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
    let referrer = ctx.route_query().get("ref").unwrap_or("none").to_owned();
    Text::new(&format!("user-{}-ref-{}", id, referrer))
  }
}

struct UserRoot {
  router: RouterHandle,
}

impl Component for UserRoot {
  type Props = SharedRouter;

  fn create(ctx: &mut Ctx) -> Self {
    let router = ctx.router(Routes::new().route("/users/:id", |ctx| ctx.mount::<UserPage>(())));
    *ctx.props::<Self::Props>().0.lock().unwrap() = Some(router.clone());
    router.push("/users/42?ref=home");
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
  tree.mount_root::<UserRoot>(&mut app, SharedRouter(router_out.clone()));
  run_pass(&mut tree);

  let router = router_out.lock().unwrap().clone().unwrap();
  (tree, router)
}

#[test]
fn route_matches_when_a_query_string_is_present() {
  let (mut tree, _router) = setup();

  assert!(tree.find_element(|e| e.text_content() == Some("user-42-ref-home")).is_some());
}

#[test]
fn query_string_is_excluded_from_path_params() {
  let (mut tree, router) = setup();

  // The `?page=2` must not bleed into the `:id` segment.
  router.push("/users/7?page=2");
  run_pass(&mut tree);

  assert!(tree.find_element(|e| e.text_content() == Some("user-7-ref-none")).is_some());
}
