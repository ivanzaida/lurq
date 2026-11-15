//! A component whose form follows its own room, read through an element ref: with
//! `ElementRef::observe_rect_before_paint` the frame that lays it out shows the form that fits, from the first frame
//! and as the room is dragged across the width it watches; with `observe_rect` the change waits for the next frame.

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Column, Rect, Row},
  core::{ElementRef, Signal},
  node::{Element, color::Color, dimension::Dimension},
};

use crate::support::{RenderSnapshot, render_pass_with_app};

const WIDE: &str = "#22c55e";
const NARROW: &str = "#0ea5e9";
/// The room from which the component is drawn wide.
const LEAST: f32 = 300.0;

#[derive(Clone, lurq::DevtoolsInspectable)]
struct Props {
  /// The width of the room it is given, the app's, as a panel dragged.
  room: Signal<f32>,
  /// Whether it watches its room before paint, or after.
  before_paint: bool,
}

impl PartialEq for Props {
  fn eq(&self, other: &Self) -> bool {
    self.before_paint == other.before_paint
  }
}

/// What fills its room: one bar wide, another narrow, by the width it was laid out at.
struct Folding {
  anchor: ElementRef,
  wide: Signal<bool>,
}

impl Component for Folding {
  type Props = Props;

  fn create(ctx: &mut Ctx) -> Self {
    let (anchor, wide) = (ElementRef::new(), ctx.signal(false));
    let watch = wide.clone();
    let observe = move |rect: lurq::core::ElementRect| {
      let fits = rect.width >= LEAST;
      if watch.get_untracked() != fits {
        watch.set(fits);
      }
    };
    if ctx.props::<Props>().before_paint {
      anchor.observe_rect_before_paint(observe);
    } else {
      anchor.observe_rect(observe);
    }
    Self { anchor, wide }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let room = ctx.props::<Props>().room.get();
    let color = if self.wide.get() { WIDE } else { NARROW };
    Row::new().child(
      Column::new()
        .width(Dimension::Px(room))
        .ref_element(self.anchor.clone())
        .child(Rect::new(Dimension::Pct(100.0), 20.0).background(color)),
    )
  }
}

fn shows(snapshot: &RenderSnapshot, hex: &str) -> bool {
  snapshot.rects.iter().any(|rect| rect.color == Color::from_hex(hex))
}

fn mounted(room: f32, before_paint: bool) -> (Tree, App, Signal<f32>) {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(600, 200);
  let room = Signal::new(room);
  tree.mount_root::<Folding>(
    &mut app,
    Props {
      room: room.clone(),
      before_paint,
    },
  );
  (tree, app, room)
}

#[test]
fn the_first_frame_shows_the_form_its_room_fits() {
  let (mut tree, mut app, _) = mounted(400.0, true);
  let first = render_pass_with_app(&mut tree, &mut app);
  assert!(
    shows(&first, WIDE) && !shows(&first, NARROW),
    "wide from the first frame"
  );

  let (mut tree, mut app, _) = mounted(200.0, true);
  let first = render_pass_with_app(&mut tree, &mut app);
  assert!(
    shows(&first, NARROW) && !shows(&first, WIDE),
    "narrow from the first frame"
  );
}

#[test]
fn a_room_dragged_across_the_width_changes_form_in_that_frame() {
  let (mut tree, mut app, room) = mounted(400.0, true);
  render_pass_with_app(&mut tree, &mut app);
  for (width, wide) in [
    (310.0, true),
    (290.0, false),
    (260.0, false),
    (305.0, true),
    (500.0, true),
    (120.0, false),
  ] {
    room.set(width);
    tree.request_redraw();
    let frame = render_pass_with_app(&mut tree, &mut app);
    assert_eq!(shows(&frame, WIDE), wide, "{width}: the wide form");
    assert_eq!(shows(&frame, NARROW), !wide, "{width}: the narrow form");
  }
}

/// Watched after paint, the first frame shows the form the component started with, and the next the one that fits.
#[test]
fn watched_after_paint_the_form_waits_a_frame() {
  let (mut tree, mut app, _) = mounted(400.0, false);
  let first = render_pass_with_app(&mut tree, &mut app);
  assert!(shows(&first, NARROW), "the first frame is the form it started with");
  assert!(tree.needs_redraw(), "and asks for another");
  let second = render_pass_with_app(&mut tree, &mut app);
  assert!(shows(&second, WIDE));
}

/// What gives a part of it half its own room through the part's ref, as a splitter gives its panes their sizes:
/// nothing is rendered again, the part is laid out again.
struct Halving {
  anchor: ElementRef,
  half: lurq::core::ElementRefMut,
}

impl Component for Halving {
  type Props = Props;

  fn create(ctx: &mut Ctx) -> Self {
    let (anchor, half) = (ElementRef::new(), lurq::core::ElementRefMut::new());
    let sized = half.clone();
    let observe = move |rect: lurq::core::ElementRect| {
      sized.set_size(Some(Dimension::Px(rect.width / 2.0)), None);
    };
    if ctx.props::<Props>().before_paint {
      anchor.observe_rect_before_paint(observe);
    } else {
      anchor.observe_rect(observe);
    }
    Self { anchor, half }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let room = ctx.props::<Props>().room.get();
    Row::new().child(
      Row::new()
        .width(Dimension::Px(room))
        .ref_element(self.anchor.clone())
        .child(
          Column::new()
            .width(Dimension::Px(10.0))
            .ref_element(self.half.clone())
            .child(Rect::new(Dimension::Pct(100.0), 20.0).background(WIDE)),
        ),
    )
  }
}

/// The width of the bar the halving component draws.
fn bar(snapshot: &RenderSnapshot) -> f32 {
  snapshot
    .rects
    .iter()
    .find(|rect| rect.color == Color::from_hex(WIDE))
    .map(|rect| rect.width)
    .expect("the bar is drawn")
}

fn halving(room: f32, before_paint: bool) -> (Tree, App, Signal<f32>) {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(600, 200);
  let room = Signal::new(room);
  tree.mount_root::<Halving>(
    &mut app,
    Props {
      room: room.clone(),
      before_paint,
    },
  );
  (tree, app, room)
}

#[test]
fn a_size_given_through_a_ref_is_laid_out_in_the_frame_that_measured_the_room() {
  let (mut tree, mut app, room) = halving(400.0, true);
  let first = render_pass_with_app(&mut tree, &mut app);
  assert_eq!(bar(&first), 200.0, "half its room from the first frame");
  room.set(300.0);
  let next = render_pass_with_app(&mut tree, &mut app);
  assert_eq!(bar(&next), 150.0, "and in the frame its room changes");
}

#[test]
fn a_size_given_through_a_ref_after_layout_waits_for_the_next_frame() {
  let (mut tree, mut app, _) = halving(400.0, false);
  let first = render_pass_with_app(&mut tree, &mut app);
  assert_eq!(bar(&first), 10.0, "the first frame is laid out as it was built");
  let second = render_pass_with_app(&mut tree, &mut app);
  assert_eq!(bar(&second), 200.0);
}
