use std::sync::{
  Arc,
  atomic::{AtomicU64, Ordering},
};

static NEXT_IMAGE_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub struct ImageData {
  id: u64,
  data: Arc<Vec<u8>>,
  width: u32,
  height: u32,
}

impl ImageData {
  pub fn from_rgba(data: Vec<u8>, width: u32, height: u32) -> Self {
    assert_eq!(data.len(), (width * height * 4) as usize);
    Self {
      id: NEXT_IMAGE_ID.fetch_add(1, Ordering::Relaxed),
      data: Arc::new(data),
      width,
      height,
    }
  }

  pub fn from_bytes(bytes: &[u8]) -> Result<Self, image::ImageError> {
    let img = image::load_from_memory(bytes)?.into_rgba8();
    let width = img.width();
    let height = img.height();
    Ok(Self::from_rgba(img.into_raw(), width, height))
  }

  pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, image::ImageError> {
    let img = image::open(path)?.into_rgba8();
    let width = img.width();
    let height = img.height();
    Ok(Self::from_rgba(img.into_raw(), width, height))
  }

  pub fn id(&self) -> u64 {
    self.id
  }

  pub fn data(&self) -> &[u8] {
    &self.data
  }

  pub fn data_arc(&self) -> Arc<Vec<u8>> {
    Arc::clone(&self.data)
  }

  pub fn width(&self) -> u32 {
    self.width
  }

  pub fn height(&self) -> u32 {
    self.height
  }
}
