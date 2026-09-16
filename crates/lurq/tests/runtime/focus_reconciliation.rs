use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Column, Form, FormHandle, FormOptions, FormProps, Text, TextInput},
  core::{ElementRef, Signal},
  node::Element,
};

use crate::support::TestSurface;

struct State {
  step: Signal<u32>,
  value: Signal<String>,
  reference: ElementRef,
  blurs: AtomicUsize,
  renders: AtomicUsize,
  request: bool,
}
#[derive(Clone, lurq::DevtoolsInspectable)]
struct Props(#[devtools_ignore] Arc<State>);
impl PartialEq for Props {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}
impl Props {
  fn new(request: bool) -> Self {
    Self(Arc::new(State {
      step: Signal::new(0),
      value: Signal::new(String::new()),
      reference: ElementRef::new(),
      blurs: AtomicUsize::new(0),
      renders: AtomicUsize::new(0),
      request,
    }))
  }
}
struct Fields {
  props: Props,
  form: FormHandle,
}
impl Component for Fields {
  type Props = Props;
  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Props>().clone();
    if props.0.request {
      ctx.focus(&props.0.reference);
    }
    Self {
      props,
      form: FormHandle::new(FormOptions::new()),
    }
  }
  fn render(&self, _: &mut Ctx) -> impl Into<Element> {
    let state = &self.props.0;
    state.renders.fetch_add(1, Ordering::SeqCst);
    let step = state.step.get();
    let mut children = Column::new();
    if step == 1 {
      children = children.child(Text::new("Error"));
    }
    if step != 2 {
      let props = self.props.clone();
      // Deliberately no key or element ID: the retained signal/ref identifies it.
      children = children.child(
        TextInput::new(state.value.clone())
          .single_line()
          .ref_element(state.reference.clone())
          .on_blur(move || {
            props.0.blurs.fetch_add(1, Ordering::SeqCst);
          }),
      );
    }
    children = children.child(Text::new(if state.reference.focused() {
      "focused"
    } else {
      "blurred"
    }));
    Form::element(FormProps::new(self.form.clone()), children)
  }
}
fn key(tree: &mut Tree, key: &str, code: &str) {
  tree.key_down(key.into(), code.into(), false, false, false);
}

#[test]
fn unkeyed_field_keeps_focus_and_typing_after_sibling_insertion() {
  let props = Props::new(false);
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<Fields>(&mut app, props.clone());
  tree.pass(&mut app, &TestSurface);
  key(&mut tree, "Tab", "Tab");
  key(&mut tree, "a", "KeyA");
  props.0.step.set(1);
  tree.pass(&mut app, &TestSurface);
  key(&mut tree, "b", "KeyB");
  assert_eq!(props.0.value.get_untracked(), "ab");
  assert!(props.0.reference.focused());
  assert_eq!(props.0.blurs.load(Ordering::SeqCst), 0);
}

#[test]
fn removed_field_emits_one_blur_and_updates_reactive_ref() {
  let props = Props::new(true);
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<Fields>(&mut app, props.clone());
  tree.pass(&mut app, &TestSurface);
  assert!(props.0.reference.focused());
  props.0.step.set(2);
  tree.pass(&mut app, &TestSurface);
  tree.pass(&mut app, &TestSurface);
  assert!(!props.0.reference.focused());
  assert_eq!(props.0.blurs.load(Ordering::SeqCst), 1);
  assert!(tree.find_element(|e| e.text_content() == Some("blurred")).is_some());
  let settled = props.0.renders.load(Ordering::SeqCst);
  tree.pass(&mut app, &TestSurface);
  assert_eq!(
    props.0.renders.load(Ordering::SeqCst),
    settled,
    "focus tracking must not cause a render loop"
  );
}

#[cfg(feature = "router")]
mod navigation {
  use std::sync::Mutex;

  use lurq::router::{RouterHandle, Routes};

  use super::*;
  struct RouteState {
    router: Mutex<Option<RouterHandle>>,
    first: Props,
    second: Props,
  }
  #[derive(Clone, lurq::DevtoolsInspectable)]
  struct RouteProps(#[devtools_ignore] Arc<RouteState>);
  impl PartialEq for RouteProps {
    fn eq(&self, other: &Self) -> bool {
      Arc::ptr_eq(&self.0, &other.0)
    }
  }
  struct Root {
    router: RouterHandle,
  }
  impl Component for Root {
    type Props = RouteProps;
    fn create(ctx: &mut Ctx) -> Self {
      let props = ctx.props::<RouteProps>().clone();
      let first = props.0.first.clone();
      let second = props.0.second.clone();
      let router = ctx.router(
        Routes::new()
          .route("/a", move |ctx| ctx.mount::<Fields>(first.clone()))
          .route("/b", move |ctx| ctx.mount::<Fields>(second.clone())),
      );
      router.push("/a");
      *props.0.router.lock().unwrap() = Some(router.clone());
      Self { router }
    }
    fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
      lurq::components::Router::mount(ctx, self.router.clone())
    }
  }
  fn check_navigation(request: bool) {
    let props = RouteProps(Arc::new(RouteState {
      router: Mutex::new(None),
      first: Props::new(false),
      second: Props::new(request),
    }));
    let mut app = App::new();
    let mut tree = Tree::new();
    tree.mount_root::<Root>(&mut app, props.clone());
    tree.pass(&mut app, &TestSurface);
    key(&mut tree, "Tab", "Tab");
    assert!(props.0.first.0.reference.focused());
    props.0.router.lock().unwrap().as_ref().unwrap().push("/b");
    tree.pass(&mut app, &TestSurface);
    assert!(!props.0.first.0.reference.focused());
    assert_eq!(props.0.first.0.blurs.load(Ordering::SeqCst), 1);
    if !request {
      key(&mut tree, "Tab", "Tab");
    }
    assert!(props.0.second.0.reference.focused());
    key(&mut tree, "x", "KeyX");
    assert_eq!(props.0.second.0.value.get_untracked(), "x");
  }
  #[test]
  fn tab_enters_the_new_form_after_route_replacement() {
    check_navigation(false);
  }
  #[test]
  fn component_focus_request_reaches_newly_mounted_route() {
    check_navigation(true);
  }
}

#[test]
fn dropping_tree_clears_focus_and_emits_blur() {
  let props = Props::new(true);
  let mut tree = Tree::new();
  tree.mount_root::<Fields>(&mut App::new(), props.clone());
  assert!(props.0.reference.focused());
  drop(tree);
  assert!(!props.0.reference.focused());
  assert_eq!(props.0.blurs.load(Ordering::SeqCst), 1);
}

struct OverlayFields {
  props: Props,
  anchor: ElementRef,
}
impl Component for OverlayFields {
  type Props = Props;
  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Props>().clone();
    ctx.focus(&props.0.reference);
    Self {
      props,
      anchor: ElementRef::new(),
    }
  }
  fn render(&self, _: &mut Ctx) -> impl Into<Element> {
    Column::new()
      .child(lurq::components::Rect::new(100., 30.).ref_element(self.anchor.clone()))
      .child(
        lurq::components::Overlay::new(
          TextInput::new(self.props.0.value.clone()).ref_element(self.props.0.reference.clone()),
        )
        .anchor(self.anchor.clone()),
      )
  }
}
#[test]
fn focus_request_waits_for_overlay_layout() {
  let props = Props::new(true);
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<OverlayFields>(&mut app, props.clone());
  tree.pass(&mut app, &TestSurface);
  assert!(props.0.reference.focused());
  key(&mut tree, "x", "KeyX");
  assert_eq!(props.0.value.get_untracked(), "x");
}

#[test]
fn absent_focus_target_does_not_keep_requesting_frames() {
  let props = Props::new(true);
  props.0.step.set(2);
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<Fields>(&mut app, props.clone());
  tree.pass(&mut app, &TestSurface);
  assert!(!tree.pass(&mut app, &TestSurface).required);
  assert!(!props.0.reference.focused());
}
