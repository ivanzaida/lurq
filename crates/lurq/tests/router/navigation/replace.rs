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

struct ReplaceRoot {
  router: RouterHandle,
}

impl Component for ReplaceRoot {
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

#[test]
fn replace_changes_current_route_without_adding_history() {
  let router_out = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<ReplaceRoot>(&mut app, SharedRouter(router_out.clone()));
  run_pass(&mut tree);

  let router = router_out.lock().unwrap().clone().unwrap();

  router.push("/b");
  run_pass(&mut tree);
  assert!(tree.find_element(|e| e.text_content() == Some("b")).is_some());

  router.replace("/c");
  run_pass(&mut tree);
  assert!(tree.find_element(|e| e.text_content() == Some("c")).is_some());

  // back should go to /a, not /b — /b was replaced
  assert!(router.back());
  run_pass(&mut tree);
  assert!(tree.find_element(|e| e.text_content() == Some("a")).is_some());
}
