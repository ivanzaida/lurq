use std::sync::Arc;

use crate::{
  app::{component::Component, ctx::Ctx},
  node::Element,
  router::{
    Navigator, RouterHandle,
    route_match::{OutletDepth, RouteMatch, RouterMatches},
  },
};

#[derive(Clone)]
pub struct RouterProps {
  pub handle: RouterHandle,
}

impl std::fmt::Debug for RouterProps {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("RouterProps").field("handle", &self.handle).finish()
  }
}

impl PartialEq for RouterProps {
  fn eq(&self, other: &Self) -> bool {
    self.handle == other.handle
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for RouterProps {
  fn write_info(&self, _buffer: &mut Vec<crate::app::component::ComponentInfo>) {}
}

pub struct Router;

impl Component for Router {
  type Props = RouterProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<RouterProps>().clone();
    let handle = props.handle;

    ctx.provide(Navigator { handle: handle.clone() });

    let path = handle.inner.current_path.get();
    let matches = handle.inner.routes.resolve(&path);
    tracing::debug!(
      target: "lurq::router",
      "[lurq/router] render path={} matches={}",
      path,
      matches.len()
    );

    if matches.is_empty() {
      return Element::new();
    }

    let matches_arc = Arc::new(matches);
    ctx.provide(RouterMatches(matches_arc.clone()));
    ctx.provide(OutletDepth(0));

    render_route_match(ctx, &matches_arc[0])
  }
}

impl Router {
  pub fn mount(ctx: &mut Ctx, handle: RouterHandle) -> Element {
    ctx.mount::<Self>(RouterProps { handle })
  }
}

#[derive(Clone)]
pub(crate) struct RouteViewProps {
  route_match: RouteMatch,
}

impl std::fmt::Debug for RouteViewProps {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("RouteViewProps")
      .field("route_match", &self.route_match)
      .finish()
  }
}

impl PartialEq for RouteViewProps {
  fn eq(&self, other: &Self) -> bool {
    self.route_match == other.route_match
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for RouteViewProps {
  fn write_info(&self, _buffer: &mut Vec<crate::app::component::ComponentInfo>) {}
}

pub(crate) struct RouteView;

impl Component for RouteView {
  type Props = RouteViewProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let route_match = ctx.props::<RouteViewProps>().route_match.clone();
    tracing::debug!(
      target: "lurq::router",
      "[lurq/router] route_view render index={} pattern={} path={}",
      route_match.route_index(),
      route_match.pattern_raw(),
      route_match.path()
    );
    (route_match.render)(ctx)
  }
}

pub(crate) fn render_route_match(ctx: &mut Ctx, route_match: &RouteMatch) -> Element {
  let key = route_match.route_index().to_string();
  ctx.mount_keyed::<RouteView>(
    &key,
    RouteViewProps {
      route_match: route_match.clone(),
    },
  )
}
