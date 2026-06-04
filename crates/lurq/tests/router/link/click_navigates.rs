use std::sync::{Arc, Mutex};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::MouseButton},
  components::{Column, Link, Text},
  node::Element,
  router::{RouterHandle, Routes},
};

use crate::support::run_pass;

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

struct LinkRoot {
  router: RouterHandle,
}

impl Component for LinkRoot {
  type Props = SharedRouter;

  fn create(ctx: &mut Ctx) -> Self {
    let router = ctx.router(
      Routes::new()
        .route("/home", |ctx| {
          Column::new()
            .child(Text::new("home-page"))
            .child(Link::build(ctx, "Go to About", "/about"))
            .into()
        })
        .route("/about", |_ctx| Text::new("about-page").into()),
    );
    *ctx.props::<Self::Props>().0.lock().unwrap() = Some(router.clone());
    router.push("/home");
    Self { router }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Router::mount(ctx, self.router.clone())
  }
}

#[test]
fn link_renders_with_label() {
  let router_out = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<LinkRoot>(&mut app, SharedRouter(router_out.clone()));
  run_pass(&mut tree);

  assert!(tree.find_element(|e| e.text_content() == Some("Go to About")).is_some());
}

#[test]
fn clicking_link_navigates_to_target() {
  let router_out = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<LinkRoot>(&mut app, SharedRouter(router_out.clone()));
  run_pass(&mut tree);

  let link = tree.find_element(|e| e.tag_name() == "Link").unwrap();
  let (x, y, w, h) = link.rect();
  let cx = x + w / 2.0;
  let cy = y + h / 2.0;

  tree.mouse_down(cx, cy, MouseButton::Left);
  tree.mouse_up(cx, cy, MouseButton::Left);
  run_pass(&mut tree);

  let router = router_out.lock().unwrap().clone().unwrap();
  assert_eq!(router.path().get(), "/about");
  assert!(tree.find_element(|e| e.text_content() == Some("about-page")).is_some());
}

#[cfg(feature = "devtools")]
#[test]
fn link_devtools_snapshot_includes_target_path() {
  let router_out = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<LinkRoot>(&mut app, SharedRouter(router_out.clone()));
  run_pass(&mut tree);

  let snapshot = lurq::app::devtools::DevToolsSnapshot::from_tree(&tree);
  let link = find_snapshot_node(snapshot.root.as_ref().unwrap(), "Link").unwrap();

  assert_eq!(
    link
      .attrs
      .iter()
      .find(|(name, _)| name == "to")
      .map(|(_, value)| value.as_str()),
    Some("/about")
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
