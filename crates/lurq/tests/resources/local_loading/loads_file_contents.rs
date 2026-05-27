use std::sync::Arc;

use lurq::resources::{LoadResourceResult, ResourceLoader};

#[test]
fn loads_file_contents_from_disk() {
  let dir = std::env::temp_dir().join("lurq_test_loads_file");
  std::fs::create_dir_all(&dir).unwrap();
  std::fs::write(dir.join("hello.txt"), b"hello world").unwrap();

  let mut loader = ResourceLoader::new();
  loader.set_root(dir.clone());

  let key: Arc<str> = "hello.txt".into();
  loader.load_resource(&key, None);

  let result = super::super::poll_get(&loader, &key);
  match result {
    LoadResourceResult::Loaded(data) => {
      assert_eq!(data.as_ref(), b"hello world");
    }
    _ => panic!("expected Loaded result"),
  }

  drop(loader);
  std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn loads_binary_file_contents() {
  let dir = std::env::temp_dir().join("lurq_test_loads_binary");
  std::fs::create_dir_all(&dir).unwrap();
  let bytes: Vec<u8> = (0..=255).collect();
  std::fs::write(dir.join("data.bin"), &bytes).unwrap();

  let mut loader = ResourceLoader::new();
  loader.set_root(dir.clone());

  let key: Arc<str> = "data.bin".into();
  loader.load_resource(&key, None);

  let result = super::super::poll_get(&loader, &key);
  match result {
    LoadResourceResult::Loaded(data) => {
      assert_eq!(data.as_ref(), &bytes);
    }
    _ => panic!("expected Loaded result"),
  }

  drop(loader);
  std::fs::remove_dir_all(&dir).ok();
}
