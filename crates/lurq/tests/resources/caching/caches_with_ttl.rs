use std::sync::Arc;

use lurq::resources::{LoadResourceResult, ResourceConfig, ResourceLoader};

#[test]
fn cached_resource_persists_across_get_calls() {
  let dir = std::env::temp_dir().join("lurq_test_cache_ttl");
  std::fs::create_dir_all(&dir).unwrap();
  std::fs::write(dir.join("cached.txt"), b"cached data").unwrap();

  let mut loader = ResourceLoader::new();
  loader.set_root(dir.clone());

  let key: Arc<str> = "cached.txt".into();
  let config = ResourceConfig { ttl: 5000, retries: 0 };
  loader.load_resource(&key, Some(config));

  let first = super::super::poll_get(&loader, &key);
  match first {
    LoadResourceResult::Loaded(data) => {
      assert_eq!(data.as_ref(), b"cached data");
    }
    _ => panic!("expected Loaded result on first get"),
  }

  let second = loader.get(&key);
  assert!(second.is_some());
  match second.unwrap() {
    LoadResourceResult::Loaded(data) => {
      assert_eq!(data.as_ref(), b"cached data");
    }
    _ => panic!("expected Loaded result on second get"),
  }

  let third = loader.get(&key);
  assert!(third.is_some());

  drop(loader);
  std::fs::remove_dir_all(&dir).ok();
}
