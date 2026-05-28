use std::sync::Arc;

use lurq::resources::{LoadResourceResult, ResourceError, ResourceLoader};

#[test]
fn errors_persist_across_multiple_get_calls() {
  let dir = std::env::temp_dir().join("lurq_test_error_persist");
  std::fs::create_dir_all(&dir).unwrap();

  let mut loader = ResourceLoader::new();
  loader.set_root(dir.clone());

  let key: Arc<str> = "missing.txt".into();
  loader.load_resource(&key, None);

  let first = super::super::poll_get(&loader, &key);
  assert!(matches!(first, LoadResourceResult::Error(ResourceError::NotFound)));

  let second = loader.get(&key);
  assert!(second.is_some());
  assert!(matches!(
    second.unwrap(),
    LoadResourceResult::Error(ResourceError::NotFound)
  ));

  let third = loader.get(&key);
  assert!(third.is_some());
  assert!(matches!(
    third.unwrap(),
    LoadResourceResult::Error(ResourceError::NotFound)
  ));

  drop(loader);
  std::fs::remove_dir_all(&dir).ok();
}
