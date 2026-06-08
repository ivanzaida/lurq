use std::sync::{Arc, Mutex};

use lurq::{
  app::{Tree, component::Component, ctx::Ctx},
  core::Signal,
  layout::{Alignment, Constraints, Size, layout_result::LayoutResult},
  node::Element,
};

use crate::support::run_pass;

fn pass_layout(runtime: &mut Tree, constraints: Constraints) -> LayoutResult {
  runtime.set_layout_constraints_override(Some(constraints));
  run_pass(runtime);
  let result = runtime.last_layout().cloned();
  runtime.set_layout_constraints_override(None);
  result.unwrap()
}

struct Shared<T>(Arc<T>);

impl<T> Clone for Shared<T> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<T> PartialEq for Shared<T> {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

impl<T> lurq::app::component::DevtoolsInspectable for Shared<T> {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

// A self-contained switch: the knob (second child) is pushed horizontally by a
// spacer rect whose *width* depends on the switch's own signal.
struct Switch {
  enabled: Signal<bool>,
}

impl Component for Switch {
  type Props = Shared<Mutex<Option<Signal<bool>>>>;

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>().clone();
    let enabled = ctx.signal(false);
    *props.0.lock().unwrap() = Some(enabled.clone());
    Self { enabled }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let knob_offset = if self.enabled.get() { 20.0 } else { 2.0 };
    lurq::components::Row::new()
      .width(40.0)
      .height(22.0)
      .align_items(Alignment::Center)
      .child(lurq::components::Rect::new(knob_offset, 1.0))
      .child(lurq::components::Rect::new(18.0, 18.0))
  }
}

// A stable parent that never re-renders itself; only the nested Switch is dirty
// when its signal flips. This forces the incremental update path.
struct Host;

impl Component for Host {
  type Props = Shared<Mutex<Option<Signal<bool>>>>;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    lurq::components::Column::new().child(ctx.mount::<Switch>(props))
  }
}

#[test]
fn nested_component_layout_change_relayouts_ancestors() {
  let signal_out = Arc::new(Mutex::new(None));
  let mut runtime = Tree::new();
  runtime.mount_root::<Host>(&mut lurq::app::App::new(), Shared(signal_out.clone()));

  let result = pass_layout(&mut runtime, Constraints::loose(Size::new(400.0, 600.0)));
  // Host Column -> Switch Row -> [spacer, knob]. Knob sits at spacer width = 2.0.
  let knob_x = result.children[0].result.children[1].offset.x;
  assert_eq!(knob_x, 2.0);

  signal_out.lock().unwrap().as_ref().unwrap().set(true);

  let result = pass_layout(&mut runtime, Constraints::loose(Size::new(400.0, 600.0)));
  // Enabled: spacer grows to 20.0, so the knob must move to x = 20.0.
  let knob_x = result.children[0].result.children[1].offset.x;
  assert_eq!(knob_x, 20.0);
}
