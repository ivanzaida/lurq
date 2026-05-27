use std::sync::Arc;

use lurq::resources::{LoadResourceResult, ResourceConfig, ResourceLoader};

#[test]
fn flush_clears_cached_resources() {
  let dir = std::env::temp_dir().join("lurq_test_flush_cached");
  std::fs::create_dir_all(&dir).unwrap();
  std::fs::write(dir.join("cached.txt"), b"data").unwrap();

  let mut loader = ResourceLoader::new();
  loader.set_root(dir.clone());

  let key: Arc<str> = "cached.txt".into();
  let config = ResourceConfig { ttl: 5000, retries: 0 };
  loader.load_resource(&key, Some(config));

  let result = super::super::poll_get(&loader, &key);
  assert!(matches!(result, LoadResourceResult::Loaded(_)));

  loader.flush_cache();

  let after_flush = loader.get(&key);
  assert!(after_flush.is_none(), "expected None after flush");

  drop(loader);
  std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn flush_clears_resolved_resources() {
  let dir = std::env::temp_dir().join("lurq_test_flush_resolved");
  std::fs::create_dir_all(&dir).unwrap();
  std::fs::write(dir.join("resolved.txt"), b"data").unwrap();

  let mut loader = ResourceLoader::new();
  loader.set_root(dir.clone());

  let key: Arc<str> = "resolved.txt".into();
  loader.load_resource(&key, None);

  // Wait for background thread to finish without consuming from resolved
  std::thread::sleep(std::time::Duration::from_millis(100));

  loader.flush_cache();

  let after_flush = loader.get(&key);
  assert!(after_flush.is_none(), "expected None after flush");

  drop(loader);
  std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn flush_does_not_clear_errors() {
  let dir = std::env::temp_dir().join("lurq_test_flush_errors");
  std::fs::create_dir_all(&dir).unwrap();

  let mut loader = ResourceLoader::new();
  loader.set_root(dir.clone());

  let key: Arc<str> = "missing.txt".into();
  loader.load_resource(&key, None);
  super::super::poll_get(&loader, &key);

  loader.flush_cache();

  let after_flush = loader.get(&key);
  assert!(after_flush.is_some(), "errors should survive flush");
  assert!(matches!(after_flush.unwrap(), LoadResourceResult::Error(_)));

  drop(loader);
  std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn flushed_resource_can_be_reloaded() {
  let dir = std::env::temp_dir().join("lurq_test_flush_reload");
  std::fs::create_dir_all(&dir).unwrap();
  std::fs::write(dir.join("reload.txt"), b"old").unwrap();

  let mut loader = ResourceLoader::new();
  loader.set_root(dir.clone());

  let key: Arc<str> = "reload.txt".into();
  let config = ResourceConfig { ttl: 5000, retries: 0 };
  loader.load_resource(&key, Some(config));
  super::super::poll_get(&loader, &key);

  loader.flush_cache();
  std::fs::write(dir.join("reload.txt"), b"new").unwrap();

  let config = ResourceConfig { ttl: 5000, retries: 0 };
  let reload = loader.load_resource(&key, Some(config));
  assert!(matches!(reload, LoadResourceResult::Pending));

  let reloaded = super::super::poll_get(&loader, &key);
  match reloaded {
    LoadResourceResult::Loaded(data) => {
      assert_eq!(data.as_ref(), b"new");
    }
    _ => panic!("expected Loaded result after reload"),
  }

  drop(loader);
  std::fs::remove_dir_all(&dir).ok();
}
