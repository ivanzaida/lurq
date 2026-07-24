//! Stale timeline runs must not outlive their nodes: while `has_active_timeline`
//! is true the runtime recomputes layout every frame, so a run left behind by an
//! unmounted node (or a finished one-shot) would silently force full-tree
//! relayout for the rest of the process.

use lurq::{
  animation::{AnimatableProperty, AnimatableValue, Animation, Keyframes, KeyframesId, Transition},
  app::{App, Tree, component::Component, ctx::Ctx, events::MouseButton},
  core::Signal,
  layout::{Constraints, Size},
  node::{Element, color::Color},
};

use crate::support::{pointer_click, run_pass};

const SPIN: u16 = 31;

fn spin_keyframes() -> Keyframes {
  Keyframes::new(KeyframesId::new(SPIN))
    .frame(0.0, |f| {
      f.set(AnimatableProperty::Opacity, AnimatableValue::Float(0.3));
    })
    .frame(1.0, |f| {
      f.set(AnimatableProperty::Opacity, AnimatableValue::Float(1.0));
    })
}

#[derive(Clone, Copy, Debug, PartialEq, lurq::DevtoolsInspectable)]
struct HostProps;

/// A toggle button plus, while `show` is set, an infinitely animated child —
/// the shape of every loading-spinner screen.
struct SpinnerHost {
  show: Signal<bool>,
}

impl Component for SpinnerHost {
  type Props = HostProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self { show: ctx.signal(true) }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let show = self.show.clone();
    let mut row = lurq::components::Row::new().spacing(8.0).child(
      lurq::components::Rect::new(40.0, 40.0)
        .background("#22c55e")
        .on_click(move |_| show.update(|v| *v = !*v)),
    );
    if self.show.get() {
      row = row.child(
        lurq::components::Rect::new(40.0, 40.0)
          .background("#ef4444")
          .animation(Animation::new(KeyframesId::new(SPIN)).duration_ms(900).linear().infinite()),
      );
    }
    row
  }
}

/// Same shape with a hover transition instead of an animation, to cover a node
/// unmounting while a transition run is still in flight.
struct TransitionHost {
  show: Signal<bool>,
}

impl Component for TransitionHost {
  type Props = HostProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self { show: ctx.signal(true) }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let show = self.show.clone();
    let mut row = lurq::components::Row::new().spacing(8.0).child(
      lurq::components::Rect::new(40.0, 40.0)
        .background("#22c55e")
        .on_click(move |_| show.update(|v| *v = !*v)),
    );
    if self.show.get() {
      row = row.child(
        lurq::components::Rect::new(40.0, 40.0)
          .background("#ef4444")
          .transition(Transition::background_color().duration_ms(60_000).linear())
          .hovered(|s| s.background("#0000ff")),
      );
    }
    row
  }
}

fn toggle(rt: &mut Tree) {
  let bounds = rt
    .find_element(|el| el.color() == Some(Color::from_hex("#22c55e")))
    .expect("toggle button")
    .bounds();
  let (x, y) = bounds.center();
  pointer_click(rt, x, y, MouseButton::Left);
}

#[test]
fn unmounting_infinite_animation_releases_active_timeline() {
  let mut rt = Tree::new();
  rt.register_keyframes(spin_keyframes());
  rt.mount_root::<SpinnerHost>(&mut App::new(), HostProps);
  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));

  run_pass(&mut rt);
  assert!(rt.has_active_timeline(), "infinite animation should keep the timeline active");

  toggle(&mut rt);
  run_pass(&mut rt);
  assert!(
    !rt.has_active_timeline(),
    "unmounting the animated node must release the timeline"
  );
}

#[test]
fn finished_one_shot_animation_releases_active_timeline() {
  let mut rt = Tree::new();
  rt.register_keyframes(spin_keyframes());

  let node = lurq::components::Rect::new(40.0, 40.0)
    .background("#ef4444")
    .animation(Animation::new(KeyframesId::new(SPIN)).duration_ms(1).linear());
  rt.set_root(node);
  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));

  run_pass(&mut rt);
  std::thread::sleep(std::time::Duration::from_millis(25));
  run_pass(&mut rt);
  assert!(
    !rt.has_active_timeline(),
    "a finished one-shot animation must not keep the timeline active"
  );

  // The finished run stays parked for the live node (so the animation does not
  // restart), but further passes must not reactivate the timeline.
  run_pass(&mut rt);
  assert!(!rt.has_active_timeline());
}

#[test]
fn unmounting_node_mid_transition_releases_active_timeline() {
  let mut rt = Tree::new();
  rt.mount_root::<TransitionHost>(&mut App::new(), HostProps);
  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));

  run_pass(&mut rt);

  let bounds = rt
    .find_element(|el| el.color() == Some(Color::from_hex("#ef4444")))
    .expect("transitioned rect")
    .bounds();
  let (x, y) = bounds.center();
  rt.mouse_move(x, y);
  run_pass(&mut rt);
  assert!(rt.has_active_timeline(), "hover change should start a transition");

  toggle(&mut rt);
  run_pass(&mut rt);
  assert!(
    !rt.has_active_timeline(),
    "unmounting a node mid-transition must release the timeline"
  );
}
