use lurq::router::Pattern;

#[test]
fn trailing_slash_is_normalized() {
  let pattern = Pattern::new("/users");
  assert!(pattern.matches("/users/").is_some());
}

#[test]
fn double_slash_is_normalized() {
  let pattern = Pattern::new("/users");
  assert!(pattern.matches("//users").is_some());
}

#[test]
fn leading_slash_required_in_pattern() {
  let pattern = Pattern::new("users");
  // should behave same as "/users"
  assert!(pattern.matches("/users").is_some());
}

#[test]
fn empty_path_matches_root() {
  let pattern = Pattern::new("/");
  assert!(pattern.matches("").is_some());
}

#[test]
fn path_with_only_slashes_matches_root() {
  let pattern = Pattern::new("/");
  assert!(pattern.matches("///").is_some());
}

#[test]
fn unicode_segments_match() {
  let pattern = Pattern::new("/каталог/:id");
  let params = pattern.matches("/каталог/42").unwrap();
  assert_eq!(params.get("id"), Some("42"));
}

#[test]
fn segments_with_hyphens_and_dots() {
  let pattern = Pattern::new("/my-app/:file");
  let params = pattern.matches("/my-app/style.css").unwrap();
  assert_eq!(params.get("file"), Some("style.css"));
}

#[test]
fn percent_encoded_segments_are_literal() {
  let pattern = Pattern::new("/files/:name");
  let params = pattern.matches("/files/hello%20world").unwrap();
  assert_eq!(params.get("name"), Some("hello%20world"));
}
