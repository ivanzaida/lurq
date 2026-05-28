use lurq::{
  app::{Runtime, component::Component, ctx::Ctx},
  components::{DragContainer, DragContainerProps, Draggable, DraggableProps, DropZone, DropZoneProps, Rect, Stack},
  node::{Element, color::Color},
};

const CHILD_COLOR: Color = Color::new(59, 130, 246, 255);

struct ExplicitDraggableChild;

impl Component for ExplicitDraggableChild {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    Draggable::mount(
      ctx,
      DraggableProps::new(),
      Rect::new(20.0, 20.0).background(CHILD_COLOR),
    )
  }
}

struct ExplicitDropZoneChild;

impl Component for ExplicitDropZoneChild {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    DropZone::mount(ctx, DropZoneProps::new(), Rect::new(20.0, 20.0).background(CHILD_COLOR))
  }
}

struct ExplicitDragContainerChild;

impl Component for ExplicitDragContainerChild {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    DragContainer::mount(
      ctx,
      DragContainerProps::new(),
      Stack::new()
        .size(40.0, 40.0)
        .child(Rect::new(20.0, 20.0).background(CHILD_COLOR)),
    )
  }
}

#[test]
fn draggable_mount_accepts_one_explicit_child() {
  let mut runtime = Runtime::new();
  runtime.mount_root::<ExplicitDraggableChild>(());

  let child = runtime.find_element(|element| element.color() == Some(CHILD_COLOR));

  assert!(child.is_some());
}

#[test]
fn drop_zone_mount_accepts_one_explicit_child() {
  let mut runtime = Runtime::new();
  runtime.mount_root::<ExplicitDropZoneChild>(());

  let child = runtime.find_element(|element| element.color() == Some(CHILD_COLOR));

  assert!(child.is_some());
}

#[test]
fn drag_container_mount_accepts_one_explicit_child() {
  let mut runtime = Runtime::new();
  runtime.mount_root::<ExplicitDragContainerChild>(());

  let child = runtime.find_element(|element| element.color() == Some(CHILD_COLOR));

  assert!(child.is_some());
}
