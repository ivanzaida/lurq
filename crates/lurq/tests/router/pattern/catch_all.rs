use lurq::router::Pattern;

#[test]
fn catch_all_matches_remaining_segments() {
  let pattern = Pattern::new("/files/**rest");
  let params = pattern.matches("/files/a/b/c").unwrap();
  assert_eq!(params.get("rest"), Some("a/b/c"));
}

#[test]
fn catch_all_matches_single_remaining_segment() {
  let pattern = Pattern::new("/files/**rest");
  let params = pattern.matches("/files/readme").unwrap();
  assert_eq!(params.get("rest"), Some("readme"));
}

#[test]
fn catch_all_does_not_match_zero_segments() {
  let pattern = Pattern::new("/files/**rest");
  assert!(pattern.matches("/files").is_none());
}

#[test]
fn catch_all_after_static_prefix() {
  let pattern = Pattern::new("/api/v1/**path");
  let params = pattern.matches("/api/v1/users/42/posts").unwrap();
  assert_eq!(params.get("path"), Some("users/42/posts"));
}

#[test]
fn catch_all_with_param_before() {
  let pattern = Pattern::new("/org/:org/**rest");
  let params = pattern.matches("/org/acme/settings/billing").unwrap();
  assert_eq!(params.get("org"), Some("acme"));
  assert_eq!(params.get("rest"), Some("settings/billing"));
}
