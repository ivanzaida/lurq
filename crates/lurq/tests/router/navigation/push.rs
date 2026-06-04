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

struct NavRoot {
  router: RouterHandle,
}

impl Component for NavRoot {
  type Props = SharedRouter;

  fn create(ctx: &mut Ctx) -> Self {
    let router = ctx.router(
      Routes::new()
        .route("/", |_ctx| Text::new("home").into())
        .route("/about", |_ctx| Text::new("about").into())
        .route("/contact", |_ctx| Text::new("contact").into()),
    );
    *ctx.props::<Self::Props>().0.lock().unwrap() = Some(router.clone());
    router.push("/");
    Self { router }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Router::mount(ctx, self.router.clone())
  }
}

#[test]
fn push_changes_rendered_route() {
  let router_out = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<NavRoot>(&mut app, SharedRouter(router_out.clone()));
  run_pass(&mut tree);

  assert!(tree.find_element(|e| e.text_content() == Some("home")).is_some());

  router_out.lock().unwrap().as_ref().unwrap().push("/about");
  run_pass(&mut tree);

  assert!(tree.find_element(|e| e.text_content() == Some("about")).is_some());
  assert!(tree.find_element(|e| e.text_content() == Some("home")).is_none());
}

#[test]
fn push_updates_path_signal() {
  let router_out = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<NavRoot>(&mut app, SharedRouter(router_out.clone()));
  run_pass(&mut tree);

  let router = router_out.lock().unwrap().clone().unwrap();
  assert_eq!(&*router.path().get(), "/");

  router.push("/contact");
  assert_eq!(&*router.path().get(), "/contact");
}

#[test]
fn push_same_path_is_noop() {
  let router_out = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<NavRoot>(&mut app, SharedRouter(router_out.clone()));
  run_pass(&mut tree);

  let router = router_out.lock().unwrap().clone().unwrap();
  router.push("/");
  // no panic, no duplicate history entry
  assert_eq!(&*router.path().get(), "/");
}
