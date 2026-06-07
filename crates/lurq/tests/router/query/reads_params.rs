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

struct SearchPage;

impl Component for SearchPage {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let query = ctx.route_query();
    let tab = query.get("tab").unwrap_or("none").to_owned();
    let page: u32 = query.get_parsed("page").unwrap_or(0);
    Text::new(&format!("tab={};page={}", tab, page))
  }
}

struct SearchRoot {
  router: RouterHandle,
}

impl Component for SearchRoot {
  type Props = SharedRouter;

  fn create(ctx: &mut Ctx) -> Self {
    let router = ctx.router(Routes::new().route("/search", |ctx| ctx.mount::<SearchPage>(())));
    *ctx.props::<Self::Props>().0.lock().unwrap() = Some(router.clone());
    router.push("/search?tab=images&page=3");
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
  tree.mount_root::<SearchRoot>(&mut app, SharedRouter(router_out.clone()));
  run_pass(&mut tree);

  let router = router_out.lock().unwrap().clone().unwrap();
  (tree, router)
}

#[test]
fn exposes_query_string_values_to_component() {
  let (mut tree, _router) = setup();

  assert!(tree.find_element(|e| e.text_content() == Some("tab=images;page=3")).is_some());
}

#[test]
fn parses_query_value_into_target_type() {
  let (mut tree, router) = setup();

  router.push("/search?tab=videos&page=7");
  run_pass(&mut tree);

  assert!(tree.find_element(|e| e.text_content() == Some("tab=videos;page=7")).is_some());
  assert!(tree.find_element(|e| e.text_content() == Some("tab=images;page=3")).is_none());
}

#[test]
fn falls_back_to_defaults_when_query_param_is_absent() {
  let (mut tree, router) = setup();

  router.push("/search");
  run_pass(&mut tree);

  assert!(tree.find_element(|e| e.text_content() == Some("tab=none;page=0")).is_some());
}
