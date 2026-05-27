use std::sync::Arc;

use lurq::resources::{LoadResourceResult, ResourceConfig, ResourceLoader};

#[test]
fn expired_resource_can_be_reloaded() {
  let dir = std::env::temp_dir().join("lurq_test_reload_after_expiry");
  std::fs::create_dir_all(&dir).unwrap();
  std::fs::write(dir.join("reload.txt"), b"first").unwrap();

  let mut loader = ResourceLoader::new();
  loader.set_root(dir.clone());

  let key: Arc<str> = "reload.txt".into();
  let config = ResourceConfig { ttl: 100, retries: 0 };
  loader.load_resource(&key, Some(config));

  let first = super::super::poll_get(&loader, &key);
  assert!(matches!(first, LoadResourceResult::Loaded(_)));

  std::thread::sleep(std::time::Duration::from_millis(150));

  // Update file contents before reloading
  std::fs::write(dir.join("reload.txt"), b"second").unwrap();

  let config = ResourceConfig { ttl: 5000, retries: 0 };
  let reload = loader.load_resource(&key, Some(config));
  assert!(matches!(reload, LoadResourceResult::Pending));

  let reloaded = super::super::poll_get(&loader, &key);
  match reloaded {
    LoadResourceResult::Loaded(data) => {
      assert_eq!(data.as_ref(), b"second");
    }
    _ => panic!("expected Loaded result after reload"),
  }

  drop(loader);
  std::fs::remove_dir_all(&dir).ok();
}
