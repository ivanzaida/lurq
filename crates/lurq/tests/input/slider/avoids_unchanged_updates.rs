use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{Tree, component::Component, ctx::Ctx, theme::Theme},
  core::Signal,
  node::Element,
};

use crate::support::run_pass;

#[derive(Clone, lurq::DevtoolsInspectable)]
struct Shared<T>(Arc<T>);

impl<T> PartialEq for Shared<T> {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

struct RangedSlider {
  value: Signal<i32>,
  renders: Arc<AtomicUsize>,
}

impl Component for RangedSlider {
  type Props = Shared<AtomicUsize>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      value: ctx.signal(42),
      renders: ctx.props::<Self::Props>().0.clone(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    self.renders.fetch_add(1, Ordering::Relaxed);
    lurq::components::Slider::new(self.value.clone())
      .range(0, 100)
      .width(100.0)
  }
}

#[test]
fn range_does_not_dirty_component_when_value_is_unchanged() {
  let renders = Arc::new(AtomicUsize::new(0));
  let mut runtime = Tree::new();
  runtime.mount_root::<RangedSlider>(Theme::default(), Shared(renders.clone()));

  run_pass(&mut runtime);
  run_pass(&mut runtime);

  assert_eq!(renders.load(Ordering::Relaxed), 1);
}
