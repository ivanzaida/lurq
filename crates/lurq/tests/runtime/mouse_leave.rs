use std::sync::{Arc, Mutex};

use lurq::{
  animation::{Easing, Transition},
  app::{Tree, component::Component, ctx::Ctx, theme::Theme},
  layout::{Alignment, text_style::FontWeight},
  node::{CursorIcon, Element, color::Color},
};

use crate::support::run_pass;

#[derive(Clone)]
struct Shared<T>(Arc<T>);

impl<T> PartialEq for Shared<T> {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

struct LeaveRoot {
  leaves: Arc<Mutex<u32>>,
}

impl Component for LeaveRoot {
  type Props = Shared<Mutex<u32>>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      leaves: ctx.props::<Self::Props>().0.clone(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Rect::new(100.0, 40.0).on_mouse_leave({
      let leaves = self.leaves.clone();
      move || *leaves.lock().unwrap() += 1
    })
  }
}

#[test]
fn mouse_leave_window_fires_on_mouse_leave() {
  let leaves = Arc::new(Mutex::new(0));
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::Rect::new(100.0, 40.0)
      .cursor(CursorIcon::Pointer)
      .on_mouse_leave({
        let leaves = leaves.clone();
        move || *leaves.lock().unwrap() += 1
      }),
  );
  run_pass(&mut runtime);

  runtime.mouse_move(10.0, 10.0);
  assert_eq!(runtime.cursor(), CursorIcon::Pointer);

  runtime.mouse_leave_window();

  assert_eq!(*leaves.lock().unwrap(), 1);
  assert_eq!(runtime.cursor(), CursorIcon::Default);

  runtime.mouse_leave_window();
  assert_eq!(*leaves.lock().unwrap(), 1);
}

#[test]
fn node_mouse_leave_fires_when_pointer_exits_node() {
  let enters = Arc::new(Mutex::new(0));
  let leaves = Arc::new(Mutex::new(0));
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::Rect::new(120.0, 32.0)
      .fill("#2563eb")
      .rounded(6.0)
      .transition(Transition::background_color().duration_ms(800).easing(Easing::Linear))
      .transition(Transition::all().duration_ms(800).easing(Easing::Linear))
      .hovered(|s| s.fill("#22c55e").size(240.0, 32.0))
      .cursor(CursorIcon::Pointer)
      .on_mouse_enter({
        let enters = enters.clone();
        move || *enters.lock().unwrap() += 1
      })
      .on_mouse_leave({
        let leaves = leaves.clone();
        move || *leaves.lock().unwrap() += 1
      }),
  );
  run_pass(&mut runtime);

  runtime.mouse_move(10.0, 10.0);
  runtime.mouse_move(300.0, 10.0);

  assert_eq!(*enters.lock().unwrap(), 1);
  assert_eq!(*leaves.lock().unwrap(), 1);
}

#[test]
fn hovering_stable_left_edge_does_not_emit_repeated_leave() {
  let enters = Arc::new(Mutex::new(0));
  let leaves = Arc::new(Mutex::new(0));
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::Row::new()
      .spacing(16.0)
      .align_items(Alignment::Center)
      .child(
        lurq::components::Text::styled(
          "Linear",
          lurq::layout::text_style::TextStyle {
            font_size: 11.0,
            weight: FontWeight::Medium,
            ..lurq::layout::text_style::TextStyle::default()
          },
        )
        .width(90.0),
      )
      .child(
        lurq::components::Rect::new(120.0, 32.0)
          .fill("#2563eb")
          .transition(Transition::background_color().duration_ms(800).easing(Easing::Linear))
          .transition(Transition::all().duration_ms(800).easing(Easing::Linear))
          .hovered(|s| s.fill("#22c55e").size(240.0, 32.0))
          .cursor(CursorIcon::Pointer)
          .on_mouse_enter({
            let enters = enters.clone();
            move || *enters.lock().unwrap() += 1
          })
          .on_mouse_leave({
            let leaves = leaves.clone();
            move || *leaves.lock().unwrap() += 1
          }),
      )
      .width(400.0)
      .height(40.0),
  );
  run_pass(&mut runtime);

  let rect = runtime
    .find_element(|el| el.color() == Some(Color::from_hex("#2563eb")))
    .unwrap()
    .bounds();
  let x = rect.x + 1.0;
  let y = rect.y + rect.height / 2.0;

  runtime.mouse_move(x, y);
  for _ in 0..8 {
    run_pass(&mut runtime);
    runtime.mouse_move(x, y);
  }

  assert_eq!(*enters.lock().unwrap(), 1);
  assert_eq!(*leaves.lock().unwrap(), 0);
}

#[test]
fn rebuild_preserves_hover_without_dispatching_mouse_leave() {
  let leaves = Arc::new(Mutex::new(0));
  let mut runtime = Tree::new();

  runtime.mount_root::<LeaveRoot>(Theme::default(), Shared(leaves.clone()));
  run_pass(&mut runtime);
  runtime.mouse_move(10.0, 10.0);

  runtime.rebuild();

  assert_eq!(*leaves.lock().unwrap(), 0);
}

#[test]
fn set_root_dispatches_mouse_leave_before_replacing_root() {
  let leaves = Arc::new(Mutex::new(0));
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::Rect::new(100.0, 40.0).on_mouse_leave({
    let leaves = leaves.clone();
    move || *leaves.lock().unwrap() += 1
  }));
  run_pass(&mut runtime);
  runtime.mouse_move(10.0, 10.0);

  runtime.set_root(lurq::components::Rect::new(100.0, 40.0));

  assert_eq!(*leaves.lock().unwrap(), 1);
}
