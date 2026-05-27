use std::sync::Arc;

use lurq::resources::{LoadResourceResult, ResourceError, ResourceLoader};

#[test]
fn load_resource_returns_error_immediately_after_prior_failure() {
  let dir = std::env::temp_dir().join("lurq_test_cached_error");
  std::fs::create_dir_all(&dir).unwrap();

  let mut loader = ResourceLoader::new();
  loader.set_root(dir.clone());

  let key: Arc<str> = "nonexistent.txt".into();

  let initial = loader.load_resource(&key, None);
  assert!(matches!(initial, LoadResourceResult::Pending));

  super::super::poll_get(&loader, &key);

  let retry = loader.load_resource(&key, None);
  assert!(matches!(retry, LoadResourceResult::Error(ResourceError::NotFound)));

  drop(loader);
  std::fs::remove_dir_all(&dir).ok();
}
