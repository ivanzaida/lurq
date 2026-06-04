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

struct AppShell;

impl Component for AppShell {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    Column::new().child(Text::new("app-shell")).child(Outlet::mount(ctx))
  }
}

struct SettingsShell;

impl Component for SettingsShell {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    Column::new().child(Text::new("settings-nav")).child(Outlet::mount(ctx))
  }
}

struct NestedRoot {
  router: RouterHandle,
}

impl Component for NestedRoot {
  type Props = SharedRouter;

  fn create(ctx: &mut Ctx) -> Self {
    let router = ctx.router(Routes::new().layout(
      "/app",
      |ctx| ctx.mount::<AppShell>(()),
      |r| {
        r.route("/dashboard", |_ctx| Text::new("dashboard").into()).layout(
          "/settings",
          |ctx| ctx.mount::<SettingsShell>(()),
          |r| {
            r.route("/profile", |_ctx| Text::new("profile-page").into())
              .route("/billing", |_ctx| Text::new("billing-page").into())
          },
        )
      },
    ));
    *ctx.props::<Self::Props>().0.lock().unwrap() = Some(router.clone());
    router.push("/app/settings/profile");
    Self { router }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Router::mount(ctx, self.router.clone())
  }
}

#[test]
fn deeply_nested_outlets_render_full_chain() {
  let router_out = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<NestedRoot>(&mut app, SharedRouter(router_out.clone()));
  run_pass(&mut tree);

  assert!(tree.find_element(|e| e.text_content() == Some("app-shell")).is_some());
  assert!(
    tree
      .find_element(|e| e.text_content() == Some("settings-nav"))
      .is_some()
  );
  assert!(
    tree
      .find_element(|e| e.text_content() == Some("profile-page"))
      .is_some()
  );
}

#[test]
fn navigating_within_nested_layout_preserves_parent_shell() {
  let router_out = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<NestedRoot>(&mut app, SharedRouter(router_out.clone()));
  run_pass(&mut tree);

  let router = router_out.lock().unwrap().clone().unwrap();
  router.push("/app/settings/billing");
  run_pass(&mut tree);

  assert!(tree.find_element(|e| e.text_content() == Some("app-shell")).is_some());
  assert!(
    tree
      .find_element(|e| e.text_content() == Some("settings-nav"))
      .is_some()
  );
  assert!(
    tree
      .find_element(|e| e.text_content() == Some("billing-page"))
      .is_some()
  );
  assert!(
    tree
      .find_element(|e| e.text_content() == Some("profile-page"))
      .is_none()
  );
}

#[test]
fn navigating_out_of_nested_layout_replaces_entire_subtree() {
  let router_out = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<NestedRoot>(&mut app, SharedRouter(router_out.clone()));
  run_pass(&mut tree);

  let router = router_out.lock().unwrap().clone().unwrap();
  router.push("/app/dashboard");
  run_pass(&mut tree);

  assert!(tree.find_element(|e| e.text_content() == Some("app-shell")).is_some());
  assert!(tree.find_element(|e| e.text_content() == Some("dashboard")).is_some());
  assert!(
    tree
      .find_element(|e| e.text_content() == Some("settings-nav"))
      .is_none()
  );
}
