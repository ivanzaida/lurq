use std::sync::Arc;

use lurq::resources::{LoadResourceResult, ResourceLoader};

#[test]
fn prepends_asset_root_to_relative_path() {
  let dir = std::env::temp_dir().join("lurq_test_asset_root");
  let nested = dir.join("sub").join("dir");
  std::fs::create_dir_all(&nested).unwrap();
  std::fs::write(nested.join("nested.txt"), b"found it").unwrap();

  let mut loader = ResourceLoader::new();
  loader.set_root(dir.clone());

  let key: Arc<str> = "sub/dir/nested.txt".into();
  loader.load_resource(&key, None);

  let result = super::super::poll_get(&loader, &key);
  match result {
    LoadResourceResult::Loaded(data) => {
      assert_eq!(data.as_ref(), b"found it");
    }
    _ => panic!("expected Loaded result"),
  }

  drop(loader);
  std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn reset_root_clears_asset_root() {
  let dir = std::env::temp_dir().join("lurq_test_reset_root");
  std::fs::create_dir_all(&dir).unwrap();
  std::fs::write(dir.join("file.txt"), b"data").unwrap();

  let mut loader = ResourceLoader::new();
  loader.set_root(dir.clone());
  loader.reset_root();

  let key: Arc<str> = "file.txt".into();
  loader.load_resource(&key, None);

  let result = super::super::poll_get(&loader, &key);
  // Without the root, "file.txt" resolves relative to CWD, not the temp dir
  // so this should fail to find the file
  assert!(matches!(result, LoadResourceResult::Error(_)));

  drop(loader);
  std::fs::remove_dir_all(&dir).ok();
}
