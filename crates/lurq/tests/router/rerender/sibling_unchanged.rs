use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Column, Text},
  core::Signal,
  node::Element,
  router::{RouterHandle, Routes},
};

use crate::support::run_pass;

#[derive(Debug, lurq::DevtoolsInspectable)]
struct Shared<T>(Arc<T>);

impl<T> Clone for Shared<T> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<T> PartialEq for Shared<T> {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

struct SiblingCounter {
  renders: Arc<AtomicUsize>,
  count: Signal<i32>,
}

impl Component for SiblingCounter {
  type Props = Shared<AtomicUsize>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      renders: ctx.props::<Self::Props>().0.clone(),
      count: ctx.signal(0),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    self.renders.fetch_add(1, Ordering::Relaxed);
    Text::new(&format!("count={}", self.count.get()))
  }
}

#[derive(Debug, lurq::DevtoolsInspectable)]
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

struct SiblingRoot {
  router: RouterHandle,
}

#[derive(Clone, Debug, PartialEq, lurq::DevtoolsInspectable)]
struct SiblingRootProps {
  #[devtools_ignore]
  router_out: SharedRouter,
  #[devtools_ignore]
  sibling_renders: Shared<AtomicUsize>,
}

impl Component for SiblingRoot {
  type Props = SiblingRootProps;

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>().clone();
    let router = ctx.router(
      Routes::new()
        .route("/a", |_ctx| Text::new("page-a").into())
        .route("/b", |_ctx| Text::new("page-b").into()),
    );
    *props.router_out.0.lock().unwrap() = Some(router.clone());
    router.push("/a");
    Self { router }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    Column::new()
      .child(ctx.mount::<SiblingCounter>(props.sibling_renders))
      .child(lurq::components::Router::mount(ctx, self.router.clone()))
  }
}

#[test]
fn route_change_does_not_rerender_sibling_component() {
  let router_out = Arc::new(Mutex::new(None));
  let sibling_renders = Arc::new(AtomicUsize::new(0));

  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<SiblingRoot>(
    &mut app,
    SiblingRootProps {
      router_out: SharedRouter(router_out.clone()),
      sibling_renders: Shared(sibling_renders.clone()),
    },
  );
  run_pass(&mut tree);

  assert_eq!(sibling_renders.load(Ordering::Relaxed), 1);

  let router = router_out.lock().unwrap().clone().unwrap();
  router.push("/b");
  run_pass(&mut tree);

  // sibling should not rerender when route changes
  assert_eq!(sibling_renders.load(Ordering::Relaxed), 1);
  assert!(tree.find_element(|e| e.text_content() == Some("page-b")).is_some());
}
