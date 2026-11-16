//! Several observers of one element's rect: the ref's own (`ElementRef::observe_rect`, which a second registration
//! replaces) and those added beside it (`ElementRef::add_rect_observer`), each called while the `RectObserver` that
//! added it is kept and removed when that is dropped.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Column, Rect, Row},
  core::{ElementRef, Signal},
  node::{Element, color::Color, dimension::Dimension},
};

use crate::support::{RenderSnapshot, render_pass_with_app};

const WIDE: &str = "#22c55e";
const NARROW: &str = "#0ea5e9";

#[derive(Clone)]
struct Props {
  room: Signal<f32>,
  anchor: ElementRef,
  /// Set by an added observer that shows its change before paint.
  wide: Signal<bool>,
}

impl PartialEq for Props {
  fn eq(&self, _: &Self) -> bool {
    true
  }
}

impl lurq::app::component::DevtoolsInspectable for Props {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

/// A box of the width the app says, on the app's ref, in the color an observer chose.
struct Room;

impl Component for Room {
  type Props = Props;

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Props>().clone();
    let color = if props.wide.get() { WIDE } else { NARROW };
    Row::new().child(
      Column::new()
        .width(Dimension::Px(props.room.get()))
        .ref_element(props.anchor.clone())
        .child(Rect::new(Dimension::Pct(100.0), 20.0).background(color)),
    )
  }
}

fn mounted(room: f32) -> (Tree, App, Props) {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(600, 200);
  let props = Props {
    room: Signal::new(room),
    anchor: ElementRef::new(),
    wide: Signal::new(false),
  };
  tree.mount_root::<Room>(&mut app, props.clone());
  (tree, app, props)
}

fn counter() -> (
  Arc<AtomicUsize>,
  impl Fn(lurq::core::ElementRect) + Send + Sync + 'static,
) {
  let count = Arc::new(AtomicUsize::new(0));
  let counted = count.clone();
  (count, move |_| {
    counted.fetch_add(1, Ordering::SeqCst);
  })
}

fn resized(tree: &mut Tree, app: &mut App, props: &Props, room: f32) -> RenderSnapshot {
  props.room.set(room);
  tree.request_redraw();
  render_pass_with_app(tree, app)
}

#[test]
fn an_added_observer_and_the_refs_own_are_each_called() {
  let (mut tree, mut app, props) = mounted(200.0);
  let (own, observe) = counter();
  props.anchor.observe_rect(observe);
  let (first, observe) = counter();
  let _first = props.anchor.add_rect_observer(observe);
  let (second, observe) = counter();
  let _second = props.anchor.mutable().add_rect_observer(observe);
  render_pass_with_app(&mut tree, &mut app);
  for (name, count) in [("own", &own), ("first", &first), ("second", &second)] {
    assert_eq!(count.load(Ordering::SeqCst), 1, "{name}: told the first rect");
  }
  // The ref's own observer registered again replaces only the ref's own.
  let (again, observe) = counter();
  props.anchor.observe_rect(observe);
  resized(&mut tree, &mut app, &props, 300.0);
  assert_eq!(own.load(Ordering::SeqCst), 1, "the replaced own observer is not called");
  for (name, count) in [("again", &again), ("first", &first), ("second", &second)] {
    assert_eq!(
      count.load(Ordering::SeqCst),
      match name {
        "again" => 1,
        _ => 2,
      },
      "{name}"
    );
  }
  // A rect that did not change calls none.
  tree.request_redraw();
  render_pass_with_app(&mut tree, &mut app);
  assert_eq!(first.load(Ordering::SeqCst), 2);
}

#[test]
fn an_added_observer_is_removed_when_its_handle_is_dropped_and_no_other() {
  let (mut tree, mut app, props) = mounted(200.0);
  let (own, observe) = counter();
  props.anchor.observe_rect(observe);
  let (kept, observe) = counter();
  let _kept = props.anchor.add_rect_observer(observe);
  let (dropped, observe) = counter();
  let handle = props.anchor.add_rect_observer(observe);
  render_pass_with_app(&mut tree, &mut app);
  drop(handle);
  resized(&mut tree, &mut app, &props, 300.0);
  assert_eq!(dropped.load(Ordering::SeqCst), 1, "not called once dropped");
  assert_eq!(kept.load(Ordering::SeqCst), 2, "the other added one is kept");
  assert_eq!(own.load(Ordering::SeqCst), 2, "and the ref's own");
  // A handle that outlives its element's ref removes nothing and does not fail.
  let (late, observe) = counter();
  let orphan = ElementRef::new().add_rect_observer(observe);
  drop(orphan);
  assert_eq!(late.load(Ordering::SeqCst), 0);
}

#[test]
fn handles_replaced_in_each_render_do_not_pile_up() {
  let (mut tree, mut app, props) = mounted(200.0);
  let (count, observe) = counter();
  let observe = Arc::new(observe);
  let mut handle = None;
  for room in [210.0, 220.0, 230.0, 240.0] {
    let observe = observe.clone();
    // What a component does in a render: adds its observer and drops the one of the render before.
    handle = Some(props.anchor.add_rect_observer(move |rect| observe(rect)));
    resized(&mut tree, &mut app, &props, room);
  }
  drop(handle);
  assert_eq!(
    count.load(Ordering::SeqCst),
    4,
    "one call for each change, not one for each render so far"
  );
}

#[test]
fn an_added_observer_before_paint_shows_its_change_in_the_frame_it_watched() {
  let (mut tree, mut app, props) = mounted(400.0);
  // The app's own observer of the ref stays beside it.
  let (own, observe) = counter();
  props.anchor.observe_rect(observe);
  let wide = props.wide.clone();
  let _handle = props.anchor.add_rect_observer_before_paint(move |rect| {
    let fits = rect.width >= 300.0;
    if wide.get_untracked() != fits {
      wide.set(fits);
    }
  });
  let shows = |frame: &RenderSnapshot, hex: &str| frame.rects.iter().any(|rect| rect.color == Color::from_hex(hex));
  let first = render_pass_with_app(&mut tree, &mut app);
  assert!(
    shows(&first, WIDE) && !shows(&first, NARROW),
    "wide from the first frame"
  );
  let narrow = resized(&mut tree, &mut app, &props, 200.0);
  assert!(
    shows(&narrow, NARROW) && !shows(&narrow, WIDE),
    "narrow in the frame that laid it out narrow"
  );
  assert!(own.load(Ordering::SeqCst) >= 2, "the ref's own observer is told too");
}
