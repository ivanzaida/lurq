use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::Text,
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

struct TrackedPage {
  renders: Arc<AtomicUsize>,
}

impl Component for TrackedPage {
  type Props = Shared<AtomicUsize>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      renders: ctx.props::<Self::Props>().0.clone(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    self.renders.fetch_add(1, Ordering::Relaxed);
    let path = ctx.route_path();
    Text::new(&format!("at:{}", path))
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

struct SinglePassRoot {
  router: RouterHandle,
}

#[derive(Clone, Debug, PartialEq, lurq::DevtoolsInspectable)]
struct SinglePassRootProps {
  #[devtools_ignore]
  router_out: SharedRouter,
  #[devtools_ignore]
  root_renders: Shared<AtomicUsize>,
  #[devtools_ignore]
  page_renders: Shared<AtomicUsize>,
}

impl Component for SinglePassRoot {
  type Props = SinglePassRootProps;

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>().clone();
    let page_renders = props.page_renders.clone();

    let router = ctx.router(
      Routes::new()
        .route("/a", {
          let renders = page_renders.clone();
          move |ctx| ctx.mount::<TrackedPage>(renders.clone())
        })
        .route("/b", {
          let renders = page_renders.clone();
          move |ctx| ctx.mount::<TrackedPage>(renders.clone())
        }),
    );
    *props.router_out.0.lock().unwrap() = Some(router.clone());
    router.push("/a");
    Self { router }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    ctx
      .props::<Self::Props>()
      .root_renders
      .0
      .fetch_add(1, Ordering::Relaxed);
    lurq::components::Router::mount(ctx, self.router.clone())
  }
}

#[test]
fn rapid_navigation_coalesces_into_single_render() {
  let router_out = Arc::new(Mutex::new(None));
  let root_renders = Arc::new(AtomicUsize::new(0));
  let page_renders = Arc::new(AtomicUsize::new(0));

  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<SinglePassRoot>(
    &mut app,
    SinglePassRootProps {
      router_out: SharedRouter(router_out.clone()),
      root_renders: Shared(root_renders.clone()),
      page_renders: Shared(page_renders.clone()),
    },
  );
  run_pass(&mut tree);

  let initial_root = root_renders.load(Ordering::Relaxed);
  let router = router_out.lock().unwrap().clone().unwrap();

  // push multiple times before render pass
  router.push("/b");
  router.push("/a");
  router.push("/b");
  run_pass(&mut tree);

  // root component should NOT re-render — only the Router child re-renders
  let root_delta = root_renders.load(Ordering::Relaxed) - initial_root;
  assert_eq!(root_delta, 0, "root should not re-render when only route changes");

  // final state should be /b
  assert!(tree.find_element(|e| e.text_content() == Some("at:/b")).is_some());
}

#[test]
fn navigation_to_same_path_does_not_trigger_rerender() {
  let router_out = Arc::new(Mutex::new(None));
  let root_renders = Arc::new(AtomicUsize::new(0));
  let page_renders = Arc::new(AtomicUsize::new(0));

  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<SinglePassRoot>(
    &mut app,
    SinglePassRootProps {
      router_out: SharedRouter(router_out.clone()),
      root_renders: Shared(root_renders.clone()),
      page_renders: Shared(page_renders.clone()),
    },
  );
  run_pass(&mut tree);

  let renders_after_init = page_renders.load(Ordering::Relaxed);

  let router = router_out.lock().unwrap().clone().unwrap();
  router.push("/a"); // same path
  run_pass(&mut tree);

  // no additional renders since path didn't change
  assert_eq!(page_renders.load(Ordering::Relaxed), renders_after_init);
}
