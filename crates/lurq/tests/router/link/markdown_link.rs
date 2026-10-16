//! A Markdown link to an in-app path navigates the router when the app does
//! not take its activations.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::MouseButton},
  components::{Markdown, MarkdownProps, Text},
  node::Element,
  router::{RouterHandle, Routes},
};

use crate::support::{pointer_click, run_pass};

#[derive(Clone, Debug, lurq::DevtoolsInspectable)]
struct Props {
  router: Arc<Mutex<Option<RouterHandle>>>,
  takes_activations: bool,
}

impl PartialEq for Props {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.router, &other.router) && self.takes_activations == other.takes_activations
  }
}

struct MarkdownLinkRoot {
  router: RouterHandle,
}

impl Component for MarkdownLinkRoot {
  type Props = Props;

  fn create(ctx: &mut Ctx) -> Self {
    let takes_activations = ctx.props::<Self::Props>().takes_activations;
    let router = ctx.router(
      Routes::new()
        .route("/home", move |ctx| {
          let props = MarkdownProps::new("Go [about](/about).").width(400.0);
          let props = if takes_activations {
            props.on_link_activate(|_, _| {})
          } else {
            props
          };
          Markdown::mount(ctx, props)
        })
        .route("/about", |_ctx| Text::new("about-page").into()),
    );
    *ctx.props::<Self::Props>().router.lock().unwrap() = Some(router.clone());
    router.push("/home");
    Self { router }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Router::mount(ctx, self.router.clone())
  }
}

/// The router's path after clicking the link with each of `buttons`.
fn path_after(takes_activations: bool, buttons: &[MouseButton]) -> String {
  let router = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<MarkdownLinkRoot>(
    &mut app,
    Props {
      router: router.clone(),
      takes_activations,
    },
  );
  run_pass(&mut tree);
  let (x, y) = tree
    .find_element(|element| element.text_content() == Some("about"))
    .expect("the link")
    .bounds()
    .center();
  for button in buttons {
    pointer_click(&mut tree, x, y, *button);
    run_pass(&mut tree);
  }
  router.lock().unwrap().clone().unwrap().path().get()
}

#[test]
fn a_markdown_link_navigates_the_router_on_a_left_click_only() {
  assert_eq!(path_after(false, &[MouseButton::Middle]), "/home");
  assert_eq!(path_after(false, &[MouseButton::Left]), "/about");
}

#[test]
fn a_markdown_link_whose_activations_the_app_takes_does_not_navigate() {
  assert_eq!(path_after(true, &[MouseButton::Left, MouseButton::Middle]), "/home");
}
