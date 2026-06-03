use lurq::{
  app::{Tree, component::Component, ctx::Ctx},
  components::{DragContainer, DragContainerProps, Draggable, DraggableProps, DropZone, DropZoneProps, Rect},
  node::Element,
};

struct DraggableWithoutExplicitChild;

impl Component for DraggableWithoutExplicitChild {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    ctx.mount::<Draggable>(DraggableProps::new())
  }
}

struct DraggableWithSlotChild;

impl Component for DraggableWithSlotChild {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    ctx.mount_with::<Draggable>(DraggableProps::new(), vec![Rect::new(10.0, 10.0).into()])
  }
}

struct DropZoneWithoutExplicitChild;

impl Component for DropZoneWithoutExplicitChild {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    ctx.mount::<DropZone>(DropZoneProps::new())
  }
}

struct DropZoneWithSlotChild;

impl Component for DropZoneWithSlotChild {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    ctx.mount_with::<DropZone>(DropZoneProps::new(), vec![Rect::new(10.0, 10.0).into()])
  }
}

struct DragContainerWithoutExplicitChild;

impl Component for DragContainerWithoutExplicitChild {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    ctx.mount::<DragContainer>(DragContainerProps::new())
  }
}

struct DragContainerWithSlotChild;

impl Component for DragContainerWithSlotChild {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    ctx.mount_with::<DragContainer>(DragContainerProps::new(), vec![Rect::new(10.0, 10.0).into()])
  }
}

#[test]
#[should_panic(expected = "Draggable requires an explicit child")]
fn draggable_rejects_missing_explicit_child() {
  Tree::new().mount_root::<DraggableWithoutExplicitChild>(&mut lurq::app::App::new(), ());
}

#[test]
#[should_panic(expected = "Draggable does not accept slot children")]
fn draggable_rejects_slot_children() {
  Tree::new().mount_root::<DraggableWithSlotChild>(&mut lurq::app::App::new(), ());
}

#[test]
#[should_panic(expected = "DropZone requires an explicit child")]
fn drop_zone_rejects_missing_explicit_child() {
  Tree::new().mount_root::<DropZoneWithoutExplicitChild>(&mut lurq::app::App::new(), ());
}

#[test]
#[should_panic(expected = "DropZone does not accept slot children")]
fn drop_zone_rejects_slot_children() {
  Tree::new().mount_root::<DropZoneWithSlotChild>(&mut lurq::app::App::new(), ());
}

#[test]
#[should_panic(expected = "DragContainer requires an explicit child")]
fn drag_container_rejects_missing_explicit_child() {
  Tree::new().mount_root::<DragContainerWithoutExplicitChild>(&mut lurq::app::App::new(), ());
}

#[test]
#[should_panic(expected = "DragContainer does not accept slot children")]
fn drag_container_rejects_slot_children() {
  Tree::new().mount_root::<DragContainerWithSlotChild>(&mut lurq::app::App::new(), ());
}
