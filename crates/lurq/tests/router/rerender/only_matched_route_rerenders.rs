use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Column, Outlet, Text},
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

struct CountedPage {
  renders: Arc<AtomicUsize>,
  label: &'static str,
}

#[derive(Clone, Debug, PartialEq, lurq::DevtoolsInspectable)]
struct CountedPageProps {
  label: &'static str,
  #[devtools_ignore]
  renders: Shared<AtomicUsize>,
}

impl Component for CountedPage {
  type Props = CountedPageProps;

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>().clone();
    Self {
      renders: props.renders.0,
      label: props.label,
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    self.renders.fetch_add(1, Ordering::Relaxed);
    Text::new(self.label)
  }
}

struct Shell {
  renders: Arc<AtomicUsize>,
}

impl Component for Shell {
  type Props = Shared<AtomicUsize>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      renders: ctx.props::<Self::Props>().0.clone(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    self.renders.fetch_add(1, Ordering::Relaxed);
    Column::new().child(Text::new("shell")).child(Outlet::mount(ctx))
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

struct RerenderRoot {
  router: RouterHandle,
}

#[derive(Clone, Debug, PartialEq, lurq::DevtoolsInspectable)]
struct RerenderRootProps {
  #[devtools_ignore]
  router_out: SharedRouter,
  #[devtools_ignore]
  shell_renders: Shared<AtomicUsize>,
  #[devtools_ignore]
  page_a_renders: Shared<AtomicUsize>,
  #[devtools_ignore]
  page_b_renders: Shared<AtomicUsize>,
}

impl Component for RerenderRoot {
  type Props = RerenderRootProps;

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>().clone();
    let shell_renders = props.shell_renders.clone();
    let page_a_renders = props.page_a_renders.clone();
    let page_b_renders = props.page_b_renders.clone();

    let router = ctx.router(
      Routes::new().layout("/", move |ctx| ctx.mount::<Shell>(shell_renders.clone()), {
        let page_a_renders = page_a_renders.clone();
        let page_b_renders = page_b_renders.clone();
        move |r| {
          r.route("/a", {
            let renders = page_a_renders.clone();
            move |ctx| {
              ctx.mount::<CountedPage>(CountedPageProps {
                label: "page-a",
                renders: renders.clone(),
              })
            }
          })
          .route("/b", {
            let renders = page_b_renders.clone();
            move |ctx| {
              ctx.mount::<CountedPage>(CountedPageProps {
                label: "page-b",
                renders: renders.clone(),
              })
            }
          })
        }
      }),
    );
    *props.router_out.0.lock().unwrap() = Some(router.clone());
    router.push("/a");
    Self { router }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Router::mount(ctx, self.router.clone())
  }
}

#[test]
fn initial_render_only_renders_matched_page() {
  let router_out = Arc::new(Mutex::new(None));
  let shell_renders = Arc::new(AtomicUsize::new(0));
  let page_a_renders = Arc::new(AtomicUsize::new(0));
  let page_b_renders = Arc::new(AtomicUsize::new(0));

  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<RerenderRoot>(
    &mut app,
    RerenderRootProps {
      router_out: SharedRouter(router_out.clone()),
      shell_renders: Shared(shell_renders.clone()),
      page_a_renders: Shared(page_a_renders.clone()),
      page_b_renders: Shared(page_b_renders.clone()),
    },
  );
  run_pass(&mut tree);

  assert_eq!(shell_renders.load(Ordering::Relaxed), 1);
  assert_eq!(page_a_renders.load(Ordering::Relaxed), 1);
  assert_eq!(page_b_renders.load(Ordering::Relaxed), 0);
}

#[test]
fn navigation_does_not_rerender_unmounted_page() {
  let router_out = Arc::new(Mutex::new(None));
  let shell_renders = Arc::new(AtomicUsize::new(0));
  let page_a_renders = Arc::new(AtomicUsize::new(0));
  let page_b_renders = Arc::new(AtomicUsize::new(0));

  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<RerenderRoot>(
    &mut app,
    RerenderRootProps {
      router_out: SharedRouter(router_out.clone()),
      shell_renders: Shared(shell_renders.clone()),
      page_a_renders: Shared(page_a_renders.clone()),
      page_b_renders: Shared(page_b_renders.clone()),
    },
  );
  run_pass(&mut tree);

  let router = router_out.lock().unwrap().clone().unwrap();
  router.push("/b");
  run_pass(&mut tree);

  assert_eq!(page_b_renders.load(Ordering::Relaxed), 1);
  // page_a was rendered once initially, should not render again
  assert_eq!(page_a_renders.load(Ordering::Relaxed), 1);
}
