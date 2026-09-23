//! A press that starts a native window move or resize loses its release to the OS loop. The shell then
//! calls `Tree::mouse_press_taken_by_os`; these tests drive the tree the same way.

use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{Tree, component::Component, ctx::Ctx, events::MouseButton},
  components::{
    DragContainer, DragContainerProps, Draggable, DraggableProps, DropMissBehavior, DropZone, DropZoneProps, Rect, Row,
    Stack, Text,
  },
  core::ElementRef,
  node::{Element, color::Color},
};

use crate::support::{pointer_click, render_pass, run_pass};

const SELECTION_COLOR: &str = "#bfdbfe";
const DRAG_COLOR: Color = Color::new(59, 130, 246, 255);

fn counter() -> Arc<AtomicUsize> {
  Arc::new(AtomicUsize::new(0))
}

/// A title-bar-like area: a press on it is what starts the native move.
fn drag_area(active: &ElementRef, clicks: &Arc<AtomicUsize>, ups: &Arc<AtomicUsize>) -> Rect {
  let clicks = clicks.clone();
  let ups = ups.clone();
  Rect::new(200.0, 32.0)
    .background("#22c55e")
    .ref_element(active.clone())
    .on_click(move |_| {
      clicks.fetch_add(1, Ordering::SeqCst);
    })
    .on_mouse_up(move |_| {
      ups.fetch_add(1, Ordering::SeqCst);
    })
}

fn selection_width(runtime: &mut Tree) -> f32 {
  render_pass(runtime)
    .rects
    .iter()
    .filter(|rect| rect.color == Color::from_hex(SELECTION_COLOR) && rect.width > 1.0 && rect.height > 0.0)
    .map(|rect| rect.width)
    .sum()
}

#[test]
fn press_taken_by_os_clears_active_without_clicking() {
  let (active, clicks, ups) = (ElementRef::new(), counter(), counter());
  let mut runtime = Tree::new();
  runtime.set_root(drag_area(&active, &clicks, &ups));
  run_pass(&mut runtime);
  let (x, y) = runtime.find_element(|_| true).unwrap().bounds().center();

  runtime.mouse_move(x, y);
  runtime.mouse_down(x, y, MouseButton::Left);
  assert!(active.active());

  runtime.mouse_press_taken_by_os(x, y, MouseButton::Left);

  assert!(!active.active(), "the press must not stay active after the OS took it");
  assert_eq!(
    ups.load(Ordering::SeqCst),
    1,
    "the release takes the normal mouse-up path"
  );
  assert_eq!(
    clicks.load(Ordering::SeqCst),
    0,
    "a press that moved the window is not a click"
  );
}

#[test]
fn click_after_press_taken_by_os_still_fires_once() {
  let (active, clicks, ups) = (ElementRef::new(), counter(), counter());
  let mut runtime = Tree::new();
  runtime.set_root(drag_area(&active, &clicks, &ups));
  run_pass(&mut runtime);
  let (x, y) = runtime.find_element(|_| true).unwrap().bounds().center();

  runtime.mouse_down(x, y, MouseButton::Left);
  runtime.mouse_press_taken_by_os(x, y, MouseButton::Left);
  pointer_click(&mut runtime, x, y, MouseButton::Left);

  assert_eq!(clicks.load(Ordering::SeqCst), 1);
  assert!(!active.active());
}

#[test]
fn late_release_after_press_taken_by_os_does_not_click() {
  // Should a platform still deliver the release after its loop, it must not complete the old press.
  let (active, clicks, ups) = (ElementRef::new(), counter(), counter());
  let mut runtime = Tree::new();
  runtime.set_root(drag_area(&active, &clicks, &ups));
  run_pass(&mut runtime);
  let (x, y) = runtime.find_element(|_| true).unwrap().bounds().center();

  runtime.mouse_down(x, y, MouseButton::Left);
  runtime.mouse_press_taken_by_os(x, y, MouseButton::Left);
  runtime.mouse_up(x, y, MouseButton::Left);

  assert_eq!(clicks.load(Ordering::SeqCst), 0);
  assert!(!active.active());
}

#[test]
fn press_taken_by_os_without_a_held_button_does_nothing() {
  let (active, clicks, ups) = (ElementRef::new(), counter(), counter());
  let mut runtime = Tree::new();
  runtime.set_root(drag_area(&active, &clicks, &ups));
  run_pass(&mut runtime);
  let (x, y) = runtime.find_element(|_| true).unwrap().bounds().center();

  runtime.mouse_press_taken_by_os(x, y, MouseButton::Left);
  runtime.mouse_down(x, y, MouseButton::Right);
  runtime.mouse_press_taken_by_os(x, y, MouseButton::Left);

  assert_eq!(ups.load(Ordering::SeqCst), 0, "no left press, so no release to deliver");
  assert_eq!(clicks.load(Ordering::SeqCst), 0);
}

#[test]
fn press_taken_by_os_ends_text_selection_drag_and_keeps_next_click() {
  let clicks = counter();
  let mut runtime = Tree::new();
  runtime.set_root(
    Row::new()
      .on_click({
        let clicks = clicks.clone();
        move |_| {
          clicks.fetch_add(1, Ordering::SeqCst);
        }
      })
      .child(Text::new("Hello selectable world").selectable(true)),
  );
  run_pass(&mut runtime);
  let rect = runtime
    .find_element(|node| node.text_content() == Some("Hello selectable world"))
    .unwrap()
    .bounds();
  let y = rect.y + rect.height / 2.0;
  let middle = rect.x + rect.width / 2.0;

  runtime.mouse_down(rect.x, y, MouseButton::Left);
  runtime.mouse_move(middle, y);
  runtime.mouse_press_taken_by_os(middle, y, MouseButton::Left);
  let selected = selection_width(&mut runtime);
  assert!(selected > 0.0, "the selection made before the OS took the press stays");

  runtime.mouse_move(rect.x + rect.width, y);
  assert_eq!(
    selection_width(&mut runtime),
    selected,
    "the selection must stop following the pointer"
  );
  assert_eq!(clicks.load(Ordering::SeqCst), 0);

  // The selection drag armed a click suppression for its release; it must not eat the next real click.
  pointer_click(&mut runtime, middle, y, MouseButton::Left);
  assert_eq!(clicks.load(Ordering::SeqCst), 1);
}

#[derive(Clone, Debug, lurq::DevtoolsInspectable)]
struct SharedDrops(Arc<Mutex<Vec<&'static str>>>);

impl PartialEq for SharedDrops {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

struct DragOverZone {
  drops: Arc<Mutex<Vec<&'static str>>>,
}

impl Component for DragOverZone {
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
        move |_| drops.lock().unwrap().push("dropped")
      }),
      Rect::new(80.0, 80.0)
        .background(Color::new(34, 197, 94, 255))
        .absolute_position(120.0, 20.0),
    );
    let draggable = Draggable::mount(
      ctx,
      DraggableProps::new()
        .drop_miss_behavior(DropMissBehavior::RevertToDragStart)
        .on_drag_end({
          let drops = self.drops.clone();
          move |event| {
            if event.drop_result == Some(lurq::app::events::DropResult::Missed) {
              drops.lock().unwrap().push("missed");
            }
          }
        }),
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

fn dragged_x(runtime: &mut Tree) -> f32 {
  runtime
    .find_element(|element| element.color() == Some(DRAG_COLOR))
    .unwrap()
    .bounds()
    .x
}

#[test]
fn press_taken_by_os_cancels_drag_without_dropping() {
  let drops = Arc::new(Mutex::new(Vec::new()));
  let mut runtime = Tree::new();
  runtime.mount_root::<DragOverZone>(&mut lurq::app::App::new(), SharedDrops(drops.clone()));
  run_pass(&mut runtime);

  runtime.mouse_down(30.0, 30.0, MouseButton::Left);
  runtime.mouse_move(130.0, 30.0);
  run_pass(&mut runtime);
  runtime.mouse_press_taken_by_os(130.0, 30.0, MouseButton::Left);
  run_pass(&mut runtime);

  assert_eq!(
    *drops.lock().unwrap(),
    vec!["missed"],
    "over a drop zone, but the drop never happened"
  );
  assert_eq!(dragged_x(&mut runtime), 20.0, "a missed drag reverts to its start");

  runtime.mouse_move(60.0, 30.0);
  run_pass(&mut runtime);
  assert_eq!(
    dragged_x(&mut runtime),
    20.0,
    "the drag must not follow the pointer any more"
  );
}
