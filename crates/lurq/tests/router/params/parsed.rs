use lurq::router::Params;

#[test]
fn get_parsed_returns_typed_value() {
  let params = Params::from_pairs([("id", "42"), ("score", "3.14")]);
  assert_eq!(params.get_parsed::<u64>("id"), Some(42));
  assert_eq!(params.get_parsed::<f64>("score"), Some(3.14));
}

#[test]
fn get_parsed_returns_none_for_invalid_type() {
  let params = Params::from_pairs([("id", "abc")]);
  assert_eq!(params.get_parsed::<u64>("id"), None);
}

#[test]
fn get_parsed_returns_none_for_missing_key() {
  let params = Params::from_pairs([("id", "42")]);
  assert_eq!(params.get_parsed::<u64>("other"), None);
}

#[test]
fn entries_iterates_all_params() {
  let params = Params::from_pairs([("a", "1"), ("b", "2"), ("c", "3")]);
  let entries: Vec<_> = params.entries().collect();
  assert_eq!(entries.len(), 3);
}

#[test]
fn default_params_is_empty() {
  let params = Params::default();
  assert!(params.is_empty());
  assert_eq!(params.len(), 0);
}
