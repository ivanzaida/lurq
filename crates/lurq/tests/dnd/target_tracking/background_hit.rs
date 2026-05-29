use std::sync::{Arc, Mutex};

use lurq::{
  app::{Tree, theme::Theme, component::Component, ctx::Ctx, events::MouseButton},
  components::{DragContainer, DragContainerProps, Draggable, DraggableProps, Rect, Stack},
  node::{Element, color::Color},
};

use crate::support::run_pass;

const DRAG_COLOR: Color = Color::new(59, 130, 246, 255);

#[derive(Clone)]
struct Shared<T>(Arc<T>);

impl<T> PartialEq for Shared<T> {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

struct AbsoluteDraggableInContainer {
  starts: Arc<Mutex<u32>>,
}

impl Component for AbsoluteDraggableInContainer {
  type Props = Shared<Mutex<u32>>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      starts: ctx.props::<Self::Props>().0.clone(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let draggable = Draggable::mount(
      ctx,
      DraggableProps::new().on_drag_start({
        let starts = self.starts.clone();
        move |_| *starts.lock().unwrap() += 1
      }),
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

#[test]
fn background_click_inside_drag_container_does_not_start_absolute_draggable_drag() {
  let starts = Arc::new(Mutex::new(0));
  let mut runtime = Tree::new();
  runtime.mount_root::<AbsoluteDraggableInContainer>(Theme::default(), Shared(starts.clone()));

  run_pass(&mut runtime);
  runtime.mouse_down(150.0, 50.0, MouseButton::Left);
  runtime.mouse_move(180.0, 50.0);

  assert_eq!(*starts.lock().unwrap(), 0);

  let dragged = runtime
    .find_element(|element| element.color() == Some(DRAG_COLOR))
    .unwrap();
  assert_eq!(dragged.bounds().x, 20.0);
  assert_eq!(dragged.bounds().y, 20.0);
}
