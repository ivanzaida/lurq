use std::{
  collections::HashMap,
  sync::Arc,
  time::{Duration, Instant},
};

use parking_lot::RwLock;

use crate::resources::ResourceConfig;

#[derive(Clone)]
pub(crate) struct CacheItem {
  pub data: Arc<Vec<u8>>,
  expires_at: Instant,
}

#[derive(Default, Clone)]
pub struct ResourceCache {
  cache: Arc<RwLock<HashMap<Arc<str>, CacheItem>>>,
}

impl ResourceCache {
  pub fn get(&self, key: &Arc<str>) -> Option<CacheItem> {
    let r = self.cache.read();
    let item = r.get(key)?;

    if Instant::now() >= item.expires_at {
      drop(r);
      self.cache.write().remove(key);
      return None;
    }

    Some(item.clone())
  }

  pub fn evict_expired(&self) {
    let now = Instant::now();
    self.cache.write().retain(|_, item| now < item.expires_at);
  }

  pub fn flush(&self) {
    self.cache.write().clear();
  }

  pub fn insert(&self, key: &Arc<str>, data: Arc<Vec<u8>>, config: ResourceConfig) -> CacheItem {
    let expires_at = Instant::now() + Duration::from_millis(config.ttl as u64);
    let item = CacheItem { data, expires_at };
    let clone = item.clone();

    self.cache.write().insert(key.clone(), item);

    clone
  }
}
