use crate::{app::ctx::Ctx, node::node::Node};

pub trait Component: Send + Sync + 'static {
  type Props: Send + 'static;
  fn create(ctx: &mut Ctx, props: Self::Props) -> Self;
  fn render(&self, ctx: &mut Ctx) -> Node;
  fn on_mounted(&self) {}
  fn on_unmounted(&self) {}
}
