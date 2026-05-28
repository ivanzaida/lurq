use lurq::{
  app::{Runtime, component::Component, ctx::Ctx, events::MouseButton},
  components::{DragContainer, DragContainerProps, Draggable, DraggableProps, Rect, Stack},
  node::{Element, color::Color},
};

use crate::support::run_pass;

const DRAG_COLOR: Color = Color::new(59, 130, 246, 255);

struct BoundedDrag;

impl Component for BoundedDrag {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let draggable = Draggable::mount(
      ctx,
      DraggableProps::new(),
      Rect::new(50.0, 50.0)
        .background(DRAG_COLOR)
        .absolute_position(20.0, 20.0),
    );

    DragContainer::mount(
      ctx,
      DragContainerProps::new(),
      Stack::new().size(200.0, 100.0).child(draggable),
    )
  }
}

fn dragged_bounds(runtime: &mut Runtime) -> lurq::core::ElementRect {
  runtime
    .find_element(|element| element.color() == Some(DRAG_COLOR))
    .unwrap()
    .bounds()
}

#[test]
fn draggable_stops_at_container_edge_while_pointer_remains_outside() {
  let mut runtime = Runtime::new();
  runtime.mount_root::<BoundedDrag>(());

  run_pass(&mut runtime);
  runtime.mouse_down(30.0, 30.0, MouseButton::Left);
  runtime.mouse_move(300.0, 30.0);
  run_pass(&mut runtime);

  assert_eq!(dragged_bounds(&mut runtime).x, 150.0);
  assert_eq!(dragged_bounds(&mut runtime).y, 20.0);

  runtime.mouse_move(300.0, 80.0);
  run_pass(&mut runtime);

  assert_eq!(dragged_bounds(&mut runtime).x, 150.0);
  assert_eq!(dragged_bounds(&mut runtime).y, 20.0);
}

#[test]
fn draggable_does_not_resume_when_pointer_reenters_after_exit() {
  let mut runtime = Runtime::new();
  runtime.mount_root::<BoundedDrag>(());

  run_pass(&mut runtime);
  runtime.mouse_down(30.0, 30.0, MouseButton::Left);
  runtime.mouse_move(300.0, 30.0);
  run_pass(&mut runtime);
  runtime.mouse_move(300.0, 80.0);
  run_pass(&mut runtime);
  runtime.mouse_move(180.0, 80.0);
  run_pass(&mut runtime);

  assert_eq!(dragged_bounds(&mut runtime).x, 150.0);
  assert_eq!(dragged_bounds(&mut runtime).y, 20.0);
}
