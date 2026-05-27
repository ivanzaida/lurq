mod caching;
mod error_handling;
mod local_loading;

use std::sync::Arc;

use lurq::resources::{LoadResourceResult, ResourceLoader};

fn poll_get(loader: &ResourceLoader, path: &Arc<str>) -> LoadResourceResult {
  for _ in 0..200 {
    if let Some(result) = loader.get(path) {
      return result;
    }
    std::thread::sleep(std::time::Duration::from_millis(10));
  }
  panic!("resource did not resolve within 2s");
}
