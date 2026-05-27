use std::sync::Arc;

use lurq::resources::{LoadResourceResult, ResourceConfig, ResourceLoader};

#[test]
fn cached_resource_expires_after_ttl() {
  let dir = std::env::temp_dir().join("lurq_test_expires_after_ttl");
  std::fs::create_dir_all(&dir).unwrap();
  std::fs::write(dir.join("expiring.txt"), b"temporary").unwrap();

  let mut loader = ResourceLoader::new();
  loader.set_root(dir.clone());

  let key: Arc<str> = "expiring.txt".into();
  let config = ResourceConfig { ttl: 100, retries: 0 };
  loader.load_resource(&key, Some(config));

  let result = super::super::poll_get(&loader, &key);
  assert!(matches!(result, LoadResourceResult::Loaded(_)));

  std::thread::sleep(std::time::Duration::from_millis(150));

  let after_expiry = loader.get(&key);
  assert!(after_expiry.is_none(), "expected None after TTL expired");

  drop(loader);
  std::fs::remove_dir_all(&dir).ok();
}
