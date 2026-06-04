use super::route_match::RouteMatch;

pub enum GuardAction {
  Allow,
  Deny,
  Redirect(String),
}

pub(crate) type GuardFn = dyn Fn(&RouteMatch) -> GuardAction + Send + Sync;
