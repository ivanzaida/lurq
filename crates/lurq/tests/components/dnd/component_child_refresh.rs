//! A `Draggable` or `DropZone` around a component keeps working after that
//! component renders again by itself.

use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{Tree, component::Component, ctx::Ctx, events::MouseButton},
  components::{DragMovement, Draggable, DraggableProps, DropZone, DropZoneProps, Rect, Row},
  core::Signal,
  node::{Element, color::Color},
};

use crate::support::run_pass;

const CARD_COLOR: Color = Color::new(59, 130, 246, 255);

#[derive(Clone, Default)]
struct Probe {
  card: Arc<Mutex<Option<Signal<f32>>>>,
  drag_starts: Arc<AtomicUsize>,
  drops: Arc<AtomicUsize>,
}

impl PartialEq for Probe {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.card, &other.card)
  }
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for Probe {}

struct Board;

impl Component for Board {
  type Props = Probe;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let probe = ctx.props::<Self::Props>().clone();
    let drag_starts = probe.drag_starts.clone();
    let card = ctx.mount::<Card>(probe);
    Draggable::mount(
      ctx,
      DraggableProps::new()
        .movement(DragMovement::Manual)
        .on_drag_start(move |_| {
          drag_starts.fetch_add(1, Ordering::Relaxed);
        }),
      card,
    )
  }
}

struct Card {
  width: Signal<f32>,
}

impl Component for Card {
  type Props = Probe;

  fn create(ctx: &mut Ctx) -> Self {
    let width = ctx.signal(30.0);
    *ctx.props::<Self::Props>().card.lock().unwrap() = Some(width.clone());
    Self { width }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Rect::new(self.width.get(), 20.0).background(CARD_COLOR)
  }
}

#[test]
fn a_draggable_component_still_drags_after_its_own_render() {
  let probe = Probe::default();
  let mut tree = Tree::new();
  tree.mount_root::<Board>(&mut lurq::app::App::new(), probe.clone());
  run_pass(&mut tree);

  probe
    .card
    .lock()
    .unwrap()
    .clone()
    .expect("the card made its signal")
    .set(40.0);
  run_pass(&mut tree);

  tree.mouse_down(10.0, 10.0, MouseButton::Left);
  tree.mouse_move(60.0, 40.0);
  run_pass(&mut tree);

  assert_eq!(probe.drag_starts.load(Ordering::Relaxed), 1);
}

/// A plain draggable square beside a drop zone around a `Card`.
struct DropBoard;

impl Component for DropBoard {
  type Props = Probe;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let probe = ctx.props::<Self::Props>().clone();
    let drops = probe.drops.clone();
    let source = Draggable::mount(
      ctx,
      DraggableProps::new().movement(DragMovement::Manual),
      Rect::new(30.0, 20.0),
    );
    let card = ctx.mount::<Card>(probe);
    let zone = DropZone::mount(
      ctx,
      DropZoneProps::new().on_drop(move |_| {
        drops.fetch_add(1, Ordering::Relaxed);
      }),
      card,
    );
    Row::new().child(source).child(zone)
  }
}

#[test]
fn a_drop_zone_component_still_takes_drops_after_its_own_render() {
  let probe = Probe::default();
  let mut tree = Tree::new();
  tree.mount_root::<DropBoard>(&mut lurq::app::App::new(), probe.clone());
  run_pass(&mut tree);

  probe
    .card
    .lock()
    .unwrap()
    .clone()
    .expect("the card made its signal")
    .set(40.0);
  run_pass(&mut tree);

  tree.mouse_down(10.0, 10.0, MouseButton::Left);
  tree.mouse_move(50.0, 10.0);
  run_pass(&mut tree);
  tree.mouse_up(50.0, 10.0, MouseButton::Left);
  run_pass(&mut tree);

  assert_eq!(probe.drops.load(Ordering::Relaxed), 1);
}
