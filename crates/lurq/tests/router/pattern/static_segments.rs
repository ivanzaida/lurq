use lurq::router::Pattern;

#[test]
fn matches_root_path() {
  let pattern = Pattern::new("/");
  assert!(pattern.matches("/").is_some());
}

#[test]
fn matches_single_static_segment() {
  let pattern = Pattern::new("/users");
  assert!(pattern.matches("/users").is_some());
}

#[test]
fn matches_multiple_static_segments() {
  let pattern = Pattern::new("/api/v1/users");
  assert!(pattern.matches("/api/v1/users").is_some());
}

#[test]
fn rejects_non_matching_segment() {
  let pattern = Pattern::new("/users");
  assert!(pattern.matches("/posts").is_none());
}

#[test]
fn rejects_partial_match() {
  let pattern = Pattern::new("/users/list");
  assert!(pattern.matches("/users").is_none());
}

#[test]
fn rejects_longer_path() {
  let pattern = Pattern::new("/users");
  assert!(pattern.matches("/users/42").is_none());
}

#[test]
fn match_is_case_sensitive() {
  let pattern = Pattern::new("/Users");
  assert!(pattern.matches("/users").is_none());
}

#[test]
fn empty_params_on_static_match() {
  let pattern = Pattern::new("/users");
  let params = pattern.matches("/users").unwrap();
  assert!(params.is_empty());
}
