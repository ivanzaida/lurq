use lurq::router::Pattern;

#[test]
fn captures_single_param() {
  let pattern = Pattern::new("/users/:id");
  let params = pattern.matches("/users/42").unwrap();
  assert_eq!(params.get("id"), Some("42"));
}

#[test]
fn captures_multiple_params() {
  let pattern = Pattern::new("/users/:user_id/posts/:post_id");
  let params = pattern.matches("/users/7/posts/99").unwrap();
  assert_eq!(params.get("user_id"), Some("7"));
  assert_eq!(params.get("post_id"), Some("99"));
}

#[test]
fn param_matches_any_non_empty_segment() {
  let pattern = Pattern::new("/files/:name");
  let params = pattern.matches("/files/readme.txt").unwrap();
  assert_eq!(params.get("name"), Some("readme.txt"));
}

#[test]
fn param_does_not_match_empty_segment() {
  let pattern = Pattern::new("/users/:id");
  assert!(pattern.matches("/users/").is_none());
}

#[test]
fn param_mixed_with_static() {
  let pattern = Pattern::new("/api/:version/users");
  let params = pattern.matches("/api/v2/users").unwrap();
  assert_eq!(params.get("version"), Some("v2"));
}

#[test]
fn rejects_when_param_segment_missing() {
  let pattern = Pattern::new("/users/:id/posts");
  assert!(pattern.matches("/users//posts").is_none());
}
