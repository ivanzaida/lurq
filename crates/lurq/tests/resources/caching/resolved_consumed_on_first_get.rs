use std::sync::Arc;

use lurq::resources::{LoadResourceResult, ResourceLoader};

#[test]
fn resolved_resource_consumed_on_first_get() {
  let dir = std::env::temp_dir().join("lurq_test_resolved_consume");
  std::fs::create_dir_all(&dir).unwrap();
  std::fs::write(dir.join("once.txt"), b"one time").unwrap();

  let mut loader = ResourceLoader::new();
  loader.set_root(dir.clone());

  let key: Arc<str> = "once.txt".into();
  loader.load_resource(&key, None);

  let first = super::super::poll_get(&loader, &key);
  match first {
    LoadResourceResult::Loaded(data) => {
      assert_eq!(data.as_ref(), b"one time");
    }
    _ => panic!("expected Loaded result"),
  }

  let second = loader.get(&key);
  assert!(second.is_none(), "expected None after consuming resolved resource");

  drop(loader);
  std::fs::remove_dir_all(&dir).ok();
}
