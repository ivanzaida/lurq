use std::sync::{Arc, Mutex};

use lurq::{
  app::{Tree, component::Component, ctx::Ctx, events::MouseButton},
  core::{ElementRect, ElementRef, Signal},
  node::{Element, color::Color},
};

use crate::support::run_pass;

#[derive(Clone, lurq::DevtoolsInspectable)]
struct Shared<T>(Arc<T>);

impl<T> PartialEq for Shared<T> {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

impl<T> std::fmt::Debug for Shared<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("Shared").field(&(Arc::as_ptr(&self.0) as usize)).finish()
  }
}

struct RefLoggingComponent {
  count: Signal<u32>,
  seen_bounds: Arc<Mutex<Vec<ElementRect>>>,
}

impl Component for RefLoggingComponent {
  type Props = Shared<Mutex<Vec<ElementRect>>>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      count: ctx.signal(0),
      seen_bounds: ctx.props::<Self::Props>().0.clone(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let element_ref = ctx.element_ref_mut();
    self.seen_bounds.lock().unwrap().push(element_ref.bounds());

    lurq::components::Rect::new(100.0, 40.0)
      .ref_element(element_ref)
      .background("#22c55e")
      .on_click({
        let count = self.count.clone();
        move |_| count.update(|value| *value += 1)
      })
  }
}

#[test]
fn element_ref_tracks_hover_and_active_state() {
  let element_ref = ElementRef::new();
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::Rect::new(100.0, 40.0).ref_element(element_ref.clone()));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  assert!(!element_ref.hovered());
  assert!(!element_ref.active());

  runtime.mouse_move(x, y);
  assert!(element_ref.hovered());

  runtime.mouse_down(x, y, MouseButton::Left);
  assert!(element_ref.active());

  runtime.mouse_up(x + 200.0, y + 200.0, MouseButton::Left);
  assert!(!element_ref.active());

  runtime.mouse_move(x + 200.0, y + 200.0);
  assert!(!element_ref.hovered());
}

#[test]
fn element_ref_tracks_focus_state() {
  let first_ref = ElementRef::new();
  let second_ref = ElementRef::new();
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::Row::new()
      .spacing(8.0)
      .child(
        lurq::components::TextInput::new(Signal::new(String::new()))
          .ref_element(first_ref.clone())
          .background("#ef4444")
          .width(100.0),
      )
      .child(
        lurq::components::TextInput::new(Signal::new(String::new()))
          .ref_element(second_ref.clone())
          .background("#22c55e")
          .width(100.0),
      ),
  );
  run_pass(&mut runtime);

  let first = runtime
    .find_element(|el| el.color() == Some(Color::from_hex("#ef4444")))
    .unwrap()
    .bounds();
  let second = runtime
    .find_element(|el| el.color() == Some(Color::from_hex("#22c55e")))
    .unwrap()
    .bounds();

  runtime.click(first.x + 10.0, first.y + first.height / 2.0, MouseButton::Left);

  assert!(first_ref.focused());
  assert!(!second_ref.focused());

  runtime.click(second.x + 10.0, second.y + second.height / 2.0, MouseButton::Left);

  assert!(!first_ref.focused());
  assert!(second_ref.focused());
}

#[test]
fn element_ref_mut_can_override_layout_bounds() {
  let element_ref = ElementRef::new().mutable();
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::Column::new()
      .child(
        lurq::components::Rect::new(10.0, 20.0)
          .ref_element(element_ref.clone())
          .background("#22c55e"),
      )
      .padding(10.0),
  );
  run_pass(&mut runtime);

  let rect = element_ref.bounds();
  assert_eq!(rect.x, 10.0);
  assert_eq!(rect.y, 10.0);

  element_ref.set_relative_bounds(15.0, 20.0, 30.0, 40.0);
  assert!(runtime.needs_redraw());
  run_pass(&mut runtime);

  let found = runtime
    .find_element(|el| el.color() == Some(Color::from_hex("#22c55e")))
    .unwrap();
  let rect = found.bounds();

  assert_eq!(rect.x, 25.0);
  assert_eq!(rect.y, 30.0);
  assert_eq!(rect.relative_x, 15.0);
  assert_eq!(rect.relative_y, 20.0);
  assert_eq!(rect.width, 30.0);
  assert_eq!(rect.height, 40.0);
}

#[test]
fn hovered_style_overrides_visuals_and_layout() {
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::Rect::new(100.0, 40.0)
      .background("#334155")
      .hovered(|el| el.background("#475569").width(120.0).height(50.0)),
  );
  run_pass(&mut runtime);

  let base = runtime
    .find_element(|el| el.color() == Some(Color::from_hex("#334155")))
    .unwrap()
    .bounds();
  assert_eq!(base.width, 100.0);
  assert_eq!(base.height, 40.0);

  runtime.mouse_move(base.x + 1.0, base.y + 1.0);
  run_pass(&mut runtime);

  let hovered = runtime
    .find_element(|el| el.color() == Some(Color::from_hex("#475569")))
    .unwrap()
    .bounds();
  assert_eq!(hovered.width, 120.0);
  assert_eq!(hovered.height, 50.0);
}

#[test]
fn ctx_element_ref_is_stable_across_rerenders() {
  let seen_bounds = Arc::new(Mutex::new(Vec::new()));
  let mut runtime = Tree::new();
  runtime.mount_root::<RefLoggingComponent>(&mut lurq::app::App::new(), Shared(seen_bounds.clone()));
  run_pass(&mut runtime);

  assert_eq!(seen_bounds.lock().unwrap()[0], ElementRect::default());

  let rect = runtime
    .find_element(|el| el.color() == Some(Color::from_hex("#22c55e")))
    .unwrap()
    .bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);

  let seen_bounds = seen_bounds.lock().unwrap();
  assert_eq!(seen_bounds.len(), 2);
  assert_eq!(seen_bounds[1].width, 100.0);
  assert_eq!(seen_bounds[1].height, 40.0);
}
