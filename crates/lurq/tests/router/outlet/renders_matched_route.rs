use std::sync::{Arc, Mutex};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Column, Outlet, Text},
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

struct Shell;

impl Component for Shell {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    Column::new().child(Text::new("header")).child(Outlet::mount(ctx))
  }
}

struct OutletRoot {
  router: RouterHandle,
}

impl Component for OutletRoot {
  type Props = SharedRouter;

  fn create(ctx: &mut Ctx) -> Self {
    let router = ctx.router(Routes::new().layout(
      "/",
      |ctx| ctx.mount::<Shell>(()),
      |r| {
        r.route("/home", |_ctx| Text::new("home-content").into())
          .route("/about", |_ctx| Text::new("about-content").into())
      },
    ));
    *ctx.props::<Self::Props>().0.lock().unwrap() = Some(router.clone());
    router.push("/home");
    Self { router }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Router::mount(ctx, self.router.clone())
  }
}

#[test]
fn outlet_renders_matched_child_route() {
  let router_out = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<OutletRoot>(&mut app, SharedRouter(router_out.clone()));
  run_pass(&mut tree);

  assert!(tree.find_element(|e| e.text_content() == Some("header")).is_some());
  assert!(
    tree
      .find_element(|e| e.text_content() == Some("home-content"))
      .is_some()
  );
}

#[test]
fn outlet_switches_content_on_navigation() {
  let router_out = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<OutletRoot>(&mut app, SharedRouter(router_out.clone()));
  run_pass(&mut tree);

  let router = router_out.lock().unwrap().clone().unwrap();
  router.push("/about");
  run_pass(&mut tree);

  assert!(tree.find_element(|e| e.text_content() == Some("header")).is_some());
  assert!(
    tree
      .find_element(|e| e.text_content() == Some("about-content"))
      .is_some()
  );
  assert!(
    tree
      .find_element(|e| e.text_content() == Some("home-content"))
      .is_none()
  );
}

#[cfg(feature = "devtools")]
#[test]
fn outlet_devtools_snapshot_includes_matched_path() {
  let router_out = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<OutletRoot>(&mut app, SharedRouter(router_out.clone()));
  run_pass(&mut tree);

  let snapshot = lurq::app::devtools::DevToolsSnapshot::from_tree(&tree);
  let outlet = find_snapshot_node(snapshot.root.as_ref().unwrap(), "Outlet").unwrap();

  assert_eq!(
    outlet
      .attrs
      .iter()
      .find(|(name, _)| name == "path")
      .map(|(_, value)| value.as_str()),
    Some("/home")
  );
}

#[cfg(feature = "devtools")]
fn find_snapshot_node<'a>(
  node: &'a lurq::app::devtools::DevToolsNode,
  tag: &str,
) -> Option<&'a lurq::app::devtools::DevToolsNode> {
  if node.tag == tag {
    return Some(node);
  }

  node.children.iter().find_map(|child| find_snapshot_node(child, tag))
}
