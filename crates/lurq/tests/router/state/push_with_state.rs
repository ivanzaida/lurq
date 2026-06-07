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

struct NavState {
  from: String,
}

struct DetailPage;

impl Component for DetailPage {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let from = ctx
      .route_state::<NavState>()
      .map(|state| state.from.clone())
      .unwrap_or_else(|| "none".to_owned());
    Text::new(&format!("from-{}", from))
  }
}

struct StateRoot {
  router: RouterHandle,
}

impl Component for StateRoot {
  type Props = SharedRouter;

  fn create(ctx: &mut Ctx) -> Self {
    let router = ctx.router(
      Routes::new()
        .route("/list", |_ctx| Text::new("list").into())
        .route("/detail", |ctx| ctx.mount::<DetailPage>(())),
    );
    *ctx.props::<Self::Props>().0.lock().unwrap() = Some(router.clone());
    router.push("/list");
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
  tree.mount_root::<StateRoot>(&mut app, SharedRouter(router_out.clone()));
  run_pass(&mut tree);

  let router = router_out.lock().unwrap().clone().unwrap();
  (tree, router)
}

#[test]
fn state_is_available_at_the_navigation_destination() {
  let (mut tree, router) = setup();

  router.push_with_state(
    "/detail",
    NavState {
      from: "list".to_owned(),
    },
  );
  run_pass(&mut tree);

  assert!(tree.find_element(|e| e.text_content() == Some("from-list")).is_some());
}

#[test]
fn state_is_absent_when_navigating_without_it() {
  let (mut tree, router) = setup();

  router.push("/detail");
  run_pass(&mut tree);

  assert!(tree.find_element(|e| e.text_content() == Some("from-none")).is_some());
}
