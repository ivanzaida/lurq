use lurq::core::{ContextMap, ReactiveContext};

// ============================================================================
// ContextMap (non-reactive)
// ============================================================================

#[test]
fn context_map_provide_and_get() {
  let mut map = ContextMap::default();
  map.provide(42_i32);
  assert_eq!(map.get::<i32>(), Some(42));
}

#[test]
fn context_map_get_missing_returns_none() {
  let map = ContextMap::default();
  assert_eq!(map.get::<i32>(), None);
}

#[test]
fn context_map_different_types() {
  let mut map = ContextMap::default();
  map.provide(42_i32);
  map.provide("hello".to_owned());
  map.provide(228_f64);
  assert_eq!(map.get::<i32>(), Some(42));
  assert_eq!(map.get::<String>(), Some("hello".to_owned()));
  assert_eq!(map.get::<f64>(), Some(228.0));
}

#[test]
fn context_map_overwrite_same_type() {
  let mut map = ContextMap::default();
  map.provide(1_i32);
  map.provide(2_i32);
  assert_eq!(map.get::<i32>(), Some(2));
}

#[test]
fn context_map_clone_inherits() {
  let mut parent = ContextMap::default();
  parent.provide(42_i32);
  let child = parent.clone();
  assert_eq!(child.get::<i32>(), Some(42));
}

#[test]
fn context_map_child_override_does_not_affect_parent() {
  let mut parent = ContextMap::default();
  parent.provide(1_i32);
  let mut child = parent.clone();
  child.provide(2_i32);
  assert_eq!(child.get::<i32>(), Some(2));
  assert_eq!(parent.get::<i32>(), Some(1));
}

// ============================================================================
// ReactiveContext
// ============================================================================

#[test]
fn reactive_context_get_initial() {
  let ctx = ReactiveContext::new(42_i32);
  assert_eq!(ctx.get(), 42);
}

#[test]
fn reactive_context_set_updates() {
  let ctx = ReactiveContext::new(0_i32);
  ctx.set(99);
  assert_eq!(ctx.get(), 99);
}

#[test]
fn reactive_context_clone_shares() {
  let ctx1 = ReactiveContext::new(0_i32);
  let ctx2 = ctx1.clone();
  ctx1.set(99);
  assert_eq!(ctx2.get(), 99);
}

#[test]
fn reactive_context_with_string() {
  let ctx = ReactiveContext::new("hello".to_owned());
  ctx.set("world".to_owned());
  assert_eq!(ctx.get(), "world");
}

#[test]
fn reactive_context_same_value_no_change() {
  let ctx = ReactiveContext::new(42_i32);
  ctx.set(42);
  assert_eq!(ctx.get(), 42);
}
