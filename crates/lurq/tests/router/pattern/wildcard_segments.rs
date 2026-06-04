use lurq::router::Pattern;

#[test]
fn wildcard_matches_any_single_segment() {
  let pattern = Pattern::new("/files/*/edit");
  assert!(pattern.matches("/files/readme/edit").is_some());
}

#[test]
fn wildcard_does_not_capture() {
  let pattern = Pattern::new("/files/*");
  let params = pattern.matches("/files/anything").unwrap();
  assert!(params.is_empty());
}

#[test]
fn wildcard_does_not_match_multiple_segments() {
  let pattern = Pattern::new("/files/*");
  assert!(pattern.matches("/files/a/b").is_none());
}

#[test]
fn wildcard_does_not_match_empty_segment() {
  let pattern = Pattern::new("/files/*");
  assert!(pattern.matches("/files/").is_none());
}
