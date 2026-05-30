use std::sync::{Arc, Mutex};

use lurq::{
  app::{Tree, component::Component, ctx::Ctx, events::MouseButton, theme::Theme},
  components::{DragContainer, DragContainerProps, Draggable, DraggableProps, DropZone, DropZoneProps, Rect, Row},
  core::Signal,
  node::{Element, color::Color},
};

use crate::support::run_pass;

const DRAG_COLOR: Color = Color::new(59, 130, 246, 255);

#[test]
fn drag_move_continues_after_pointer_leaves_source_bounds() {
  let moves = Arc::new(Mutex::new(Vec::new()));
  let captured = moves.clone();

  let mut runtime = Tree::new();
  runtime.set_root(Rect::new(100.0, 100.0).on_drag_move(move |event| {
    captured
      .lock()
      .unwrap()
      .push((event.delta_x, event.delta_y, event.total_delta_x, event.total_delta_y));
  }));

  run_pass(&mut runtime);
  runtime.mouse_down(10.0, 10.0, MouseButton::Left);
  runtime.mouse_move(200.0, 220.0);

  assert_eq!(*moves.lock().unwrap(), vec![(190.0, 210.0, 190.0, 210.0)]);
}

#[test]
fn drag_end_fires_even_when_pointer_is_released_outside_source_bounds() {
  let ends = Arc::new(Mutex::new(Vec::new()));
  let captured = ends.clone();

  let mut runtime = Tree::new();
  runtime.set_root(Rect::new(100.0, 100.0).on_drag_end(move |event| {
    captured
      .lock()
      .unwrap()
      .push((event.x, event.y, event.total_delta_x, event.total_delta_y));
  }));

  run_pass(&mut runtime);
  runtime.mouse_down(20.0, 25.0, MouseButton::Left);
  runtime.mouse_up(160.0, 175.0, MouseButton::Left);

  assert_eq!(*ends.lock().unwrap(), vec![(160.0, 175.0, 140.0, 150.0)]);
}

#[test]
fn drag_reports_incremental_and_total_deltas() {
  let moves = Arc::new(Mutex::new(Vec::new()));
  let captured = moves.clone();

  let mut runtime = Tree::new();
  runtime.set_root(Rect::new(100.0, 100.0).on_drag_move(move |event| {
    captured
      .lock()
      .unwrap()
      .push((event.delta_x, event.delta_y, event.total_delta_x, event.total_delta_y));
  }));

  run_pass(&mut runtime);
  runtime.mouse_down(5.0, 8.0, MouseButton::Left);
  runtime.mouse_move(15.0, 18.0);
  runtime.mouse_move(35.0, 23.0);

  assert_eq!(
    *moves.lock().unwrap(),
    vec![(10.0, 10.0, 10.0, 10.0), (20.0, 5.0, 30.0, 15.0)]
  );
}

#[test]
fn drop_dispatches_to_hit_drop_target_on_release() {
  let drops = Arc::new(Mutex::new(Vec::new()));

  let mut runtime = Tree::new();
  runtime.mount_root::<DropDispatch>(Theme::default(), Shared(drops.clone()));

  run_pass(&mut runtime);
  runtime.mouse_down(10.0, 10.0, MouseButton::Left);
  runtime.mouse_up(90.0, 20.0, MouseButton::Left);

  assert_eq!(*drops.lock().unwrap(), vec![(90.0, 20.0, 80.0, 10.0, true, true)]);
}

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

struct DropDispatch {
  drops: Arc<Mutex<Vec<(f32, f32, f32, f32, bool, bool)>>>,
}

impl Component for DropDispatch {
  type Props = Shared<Mutex<Vec<(f32, f32, f32, f32, bool, bool)>>>;

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
        move |event| {
          drops.lock().unwrap().push((
            event.x,
            event.y,
            event.total_delta_x,
            event.total_delta_y,
            event.source_id.is_assigned(),
            event.target_id.is_assigned(),
          ));
        }
      }),
      Rect::new(80.0, 50.0),
    );

    Row::new()
      .spacing(20.0)
      .child(Rect::new(50.0, 50.0).on_drag_move(|_| {}))
      .child(drop_zone)
  }
}

struct DragRerender {
  status: Signal<&'static str>,
  moves: Arc<Mutex<Vec<f32>>>,
}

impl Component for DragRerender {
  type Props = Shared<Mutex<Vec<f32>>>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      status: ctx.signal("Idle"),
      moves: ctx.props::<Self::Props>().0.clone(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let _status = self.status.get();

    let draggable = Draggable::mount(
      ctx,
      DraggableProps::new()
        .on_drag_start({
          let status = self.status.clone();
          move |_| status.set("Dragging")
        })
        .on_drag_move({
          let moves = self.moves.clone();
          move |event| moves.lock().unwrap().push(event.delta_x)
        }),
      Rect::new(50.0, 50.0).background(DRAG_COLOR).absolute_position(0.0, 0.0),
    );

    DragContainer::mount(
      ctx,
      DragContainerProps::new(),
      lurq::components::Stack::new().size(240.0, 80.0).child(draggable),
    )
  }
}

#[test]
fn signal_driven_rerender_does_not_cancel_active_drag() {
  let moves = Arc::new(Mutex::new(Vec::new()));
  let mut runtime = Tree::new();
  runtime.mount_root::<DragRerender>(Theme::default(), Shared(moves.clone()));

  run_pass(&mut runtime);
  runtime.mouse_down(10.0, 10.0, MouseButton::Left);
  runtime.mouse_move(20.0, 10.0);
  runtime.mouse_move(180.0, 10.0);

  assert_eq!(*moves.lock().unwrap(), vec![10.0, 160.0]);

  run_pass(&mut runtime);
  let dragged = runtime
    .find_element(|element| element.color() == Some(DRAG_COLOR))
    .unwrap();
  assert_eq!(dragged.bounds().x, 170.0);
}

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
      Rect::new(50.0, 50.0).background(DRAG_COLOR).absolute_position(0.0, 0.0),
    );

    DragContainer::mount(
      ctx,
      DragContainerProps::new(),
      lurq::components::Stack::new().size(240.0, 80.0).child(draggable),
    )
  }
}

#[test]
fn drag_container_clamps_draggable_to_container_bounds() {
  let mut runtime = Tree::new();
  runtime.mount_root::<BoundedDrag>(Theme::default(), ());

  run_pass(&mut runtime);
  runtime.mouse_down(10.0, 10.0, MouseButton::Left);
  runtime.mouse_move(400.0, 100.0);

  run_pass(&mut runtime);
  let dragged = runtime
    .find_element(|element| element.color() == Some(DRAG_COLOR))
    .unwrap();
  assert_eq!(dragged.bounds().x, 190.0);
  assert_eq!(dragged.bounds().y, 30.0);
}

struct DemoBoundedDrag {
  status: Signal<&'static str>,
}

impl Component for DemoBoundedDrag {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      status: ctx.signal("Idle"),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let _status = self.status.get();
    let draggable = Draggable::mount(
      ctx,
      DraggableProps::new().on_drag_start({
        let status = self.status.clone();
        move |_| status.set("Dragging")
      }),
      Row::new()
        .align_items(lurq::layout::Alignment::Center)
        .justify(lurq::layout::layout_kind::Justify::Center)
        .size(120.0, 80.0)
        .background(DRAG_COLOR)
        .absolute_position(20.0, 40.0),
    );

    DragContainer::mount(
      ctx,
      DragContainerProps::new(),
      lurq::components::Stack::new().size(400.0, 280.0).child(draggable),
    )
  }
}

#[test]
fn drag_container_clamp_survives_drag_start_rerender() {
  let mut runtime = Tree::new();
  runtime.mount_root::<DemoBoundedDrag>(Theme::default(), ());

  run_pass(&mut runtime);
  runtime.mouse_down(30.0, 50.0, MouseButton::Left);
  run_pass(&mut runtime);
  runtime.mouse_move(700.0, 500.0);
  run_pass(&mut runtime);
  runtime.mouse_move(900.0, 700.0);

  let dragged = runtime
    .find_element(|element| element.color() == Some(DRAG_COLOR))
    .unwrap();
  assert_eq!(dragged.bounds().x, 280.0);
  assert_eq!(dragged.bounds().y, 200.0);
}
