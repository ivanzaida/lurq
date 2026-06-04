use lurq::router::{Pattern, Routes};

#[test]
fn static_beats_param_at_same_position() {
  let pattern_static = Pattern::new("/users/new");
  let pattern_param = Pattern::new("/users/:id");

  assert!(pattern_static.matches("/users/new").is_some());
  assert!(pattern_param.matches("/users/new").is_some());

  // Both match, but static has higher priority
  assert!(pattern_static.priority() > pattern_param.priority());
}

#[test]
fn param_beats_wildcard_at_same_position() {
  let pattern_param = Pattern::new("/files/:name");
  let pattern_wildcard = Pattern::new("/files/*");

  assert!(pattern_param.priority() > pattern_wildcard.priority());
}

#[test]
fn wildcard_beats_catch_all() {
  let pattern_wildcard = Pattern::new("/files/*");
  let pattern_catch_all = Pattern::new("/files/**rest");

  assert!(pattern_wildcard.priority() > pattern_catch_all.priority());
}

#[test]
fn more_static_segments_wins() {
  let pattern_a = Pattern::new("/api/v1/users");
  let pattern_b = Pattern::new("/api/:version/users");

  assert!(pattern_a.priority() > pattern_b.priority());
}

#[test]
fn routes_resolve_highest_priority_first() {
  let routes = Routes::new()
    .route("/users/:id", |_ctx| lurq::components::Text::new("param").into())
    .route("/users/new", |_ctx| lurq::components::Text::new("static").into());

  let matches = routes.resolve("/users/new");
  assert_eq!(matches.len(), 1);
  // static route should win even though param route was defined first
  assert_eq!(matches[0].pattern_raw(), "/users/new");
}

#[test]
fn duplicate_static_routes_resolve_to_first_defined() {
  let routes = Routes::new()
    .route("/home", |_ctx| lurq::components::Text::new("first").into())
    .route("/home", |_ctx| lurq::components::Text::new("second").into());

  let matches = routes.resolve("/home");
  assert_eq!(matches.len(), 1);
  assert_eq!(matches[0].route_index(), 0);
}
