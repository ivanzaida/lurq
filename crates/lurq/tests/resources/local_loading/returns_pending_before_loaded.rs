use std::sync::Arc;

use lurq::resources::{LoadResourceResult, ResourceLoader};

#[test]
fn returns_pending_on_first_load_call() {
  let dir = std::env::temp_dir().join("lurq_test_pending");
  std::fs::create_dir_all(&dir).unwrap();
  std::fs::write(dir.join("file.txt"), b"content").unwrap();

  let mut loader = ResourceLoader::new();
  loader.set_root(dir.clone());

  let key: Arc<str> = "file.txt".into();
  let result = loader.load_resource(&key, None);

  assert!(matches!(result, LoadResourceResult::Pending));

  drop(loader);
  std::fs::remove_dir_all(&dir).ok();
}
