use std::sync::Arc;

#[derive(Clone, Debug)]
pub enum ResourceError {
  NotFound,
  NetworkError(u16),
  Unknown(String),
  OsError(i32),
}

impl ResourceError {
  pub fn os_err(e: i32) -> Self {
    Self::OsError(e)
  }
}

#[derive(Default)]
pub struct ResourceConfig {
  pub ttl: u32,
  pub retries: u8,
}

#[derive(Clone, Default)]
pub enum LoadResourceResult {
  Loaded(Arc<Vec<u8>>),
  Error(ResourceError),
  #[default]
  Pending,
}
