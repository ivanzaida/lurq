use std::sync::Arc;

use super::router_component::render_route_match;
use crate::{
  app::{component::Component, ctx::Ctx},
  node::Element,
  router::route_match::{OutletDepth, RouterMatches},
};

pub struct Outlet;

impl Component for Outlet {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let depth = ctx.use_context::<OutletDepth>().unwrap_or(OutletDepth(0));
    let next_depth = depth.0 + 1;

    let Some(matches) = ctx.use_context::<RouterMatches>() else {
      return ctx.children().first().cloned().unwrap_or_else(Element::new);
    };

    let Some(child_match) = matches.0.get(next_depth) else {
      return ctx.children().first().cloned().unwrap_or_else(Element::new);
    };

    ctx.provide(OutletDepth(next_depth));

    render_route_match(ctx, child_match).outlet_debug_attr(child_match.path.clone())
  }
}

impl Outlet {
  pub fn mount(ctx: &mut Ctx) -> Element {
    ctx.mount::<Self>(())
  }
}

trait OutletDebugAttr {
  fn outlet_debug_attr(self, path: Arc<str>) -> Self;
}

impl OutletDebugAttr for Element {
  #[cfg(feature = "devtools")]
  fn outlet_debug_attr(mut self, path: Arc<str>) -> Self {
    self.node = self.node.debug_attr("path", path);
    self
  }

  #[cfg(not(feature = "devtools"))]
  fn outlet_debug_attr(self, _path: Arc<str>) -> Self {
    self
  }
}
