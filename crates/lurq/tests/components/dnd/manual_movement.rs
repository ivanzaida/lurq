use lurq::{
  app::{Tree, component::Component, ctx::Ctx, events::MouseButton},
  components::{DragMovement, Draggable, DraggableProps, Rect},
  node::{Element, color::Color},
};

use crate::support::run_pass;

const DRAG_COLOR: Color = Color::new(59, 130, 246, 255);

struct ManualDraggable;

impl Component for ManualDraggable {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    Draggable::mount(
      ctx,
      DraggableProps::new().movement(DragMovement::Manual),
      Rect::new(30.0, 20.0).background(DRAG_COLOR),
    )
  }
}

#[test]
fn manual_movement_leaves_positioning_to_the_move_callback() {
  let mut runtime = Tree::new();
  runtime.mount_root::<ManualDraggable>(&mut lurq::app::App::new(), ());
  run_pass(&mut runtime);

  runtime.mouse_down(10.0, 10.0, MouseButton::Left);
  runtime.mouse_move(60.0, 40.0);
  run_pass(&mut runtime);

  let dragged = runtime
    .find_element(|element| element.color() == Some(DRAG_COLOR))
    .expect("manual draggable");
  assert_eq!(dragged.bounds().relative_x, 0.0);
  assert_eq!(dragged.bounds().relative_y, 0.0);
}
