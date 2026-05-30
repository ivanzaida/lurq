use std::sync::{Arc, Mutex};

use lurq::{
  app::{Tree, component::Component, ctx::Ctx, events::MouseButton, theme::Theme},
  components::{
    DragContainer, DragContainerProps, Draggable, DraggableProps, DropMissBehavior, DropZone, DropZoneProps, Rect,
    Stack,
  },
  node::{Element, color::Color},
};

use crate::support::run_pass;

const DRAG_COLOR: Color = Color::new(59, 130, 246, 255);

#[derive(Clone, Debug, lurq::DevtoolsInspectable)]
struct SharedDrops(Arc<Mutex<Vec<&'static str>>>);

impl PartialEq for SharedDrops {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

struct RevertOnMiss {
  drops: Arc<Mutex<Vec<&'static str>>>,
}

impl Component for RevertOnMiss {
  type Props = SharedDrops;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      drops: ctx.props::<Self::Props>().0.clone(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let drop_zone = DropZone::mount(
      ctx,
      DropZoneProps::new().on_drop({
        let drops = self.drops.clone();
        move |_| drops.lock().unwrap().push("accepted")
      }),
      Rect::new(80.0, 80.0)
        .background(Color::new(34, 197, 94, 255))
        .absolute_position(120.0, 20.0),
    );

    let draggable = Draggable::mount(
      ctx,
      DraggableProps::new().drop_miss_behavior(DropMissBehavior::RevertToDragStart),
      Rect::new(50.0, 50.0)
        .background(DRAG_COLOR)
        .absolute_position(20.0, 20.0),
    );

    DragContainer::mount(
      ctx,
      DragContainerProps::new(),
      Stack::new().size(240.0, 120.0).child(drop_zone).child(draggable),
    )
  }
}

fn dragged_bounds(runtime: &mut Tree) -> lurq::core::ElementRect {
  runtime
    .find_element(|element| element.color() == Some(DRAG_COLOR))
    .unwrap()
    .bounds()
}

#[test]
fn draggable_reverts_to_start_when_released_outside_drop_target() {
  let drops = Arc::new(Mutex::new(Vec::new()));
  let mut runtime = Tree::new();
  runtime.mount_root::<RevertOnMiss>(Theme::default(), SharedDrops(drops.clone()));

  run_pass(&mut runtime);
  runtime.mouse_down(30.0, 30.0, MouseButton::Left);
  runtime.mouse_move(80.0, 30.0);
  run_pass(&mut runtime);
  runtime.mouse_up(80.0, 30.0, MouseButton::Left);
  run_pass(&mut runtime);

  assert_eq!(dragged_bounds(&mut runtime).x, 20.0);
  assert_eq!(dragged_bounds(&mut runtime).y, 20.0);
  assert!(drops.lock().unwrap().is_empty());
}

#[test]
fn draggable_keeps_position_when_released_on_drop_target() {
  let drops = Arc::new(Mutex::new(Vec::new()));
  let mut runtime = Tree::new();
  runtime.mount_root::<RevertOnMiss>(Theme::default(), SharedDrops(drops.clone()));

  run_pass(&mut runtime);
  runtime.mouse_down(30.0, 30.0, MouseButton::Left);
  runtime.mouse_move(130.0, 30.0);
  run_pass(&mut runtime);
  runtime.mouse_up(130.0, 30.0, MouseButton::Left);
  run_pass(&mut runtime);

  assert_eq!(dragged_bounds(&mut runtime).x, 120.0);
  assert_eq!(dragged_bounds(&mut runtime).y, 20.0);
  assert_eq!(*drops.lock().unwrap(), vec!["accepted"]);
}
