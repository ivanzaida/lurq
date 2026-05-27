use crate::{app::ctx::Ctx, node::Element};

pub trait Component: Send + Sync + 'static {
  type Props: Send + 'static;
  fn create(ctx: &mut Ctx, props: Self::Props) -> Self;
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element>;
  fn on_mounted(&self) {}
  fn on_unmounted(&self) {}
}
