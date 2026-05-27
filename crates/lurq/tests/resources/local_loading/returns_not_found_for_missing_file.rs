use std::sync::Arc;

use lurq::resources::{LoadResourceResult, ResourceError, ResourceLoader};

#[test]
fn returns_not_found_for_missing_file() {
  let dir = std::env::temp_dir().join("lurq_test_not_found");
  std::fs::create_dir_all(&dir).unwrap();

  let mut loader = ResourceLoader::new();
  loader.set_root(dir.clone());

  let key: Arc<str> = "does_not_exist.txt".into();
  loader.load_resource(&key, None);

  let result = super::super::poll_get(&loader, &key);
  assert!(matches!(result, LoadResourceResult::Error(ResourceError::NotFound)));

  drop(loader);
  std::fs::remove_dir_all(&dir).ok();
}
