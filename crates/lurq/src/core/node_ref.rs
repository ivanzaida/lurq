use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct NodeRef {
  inner: Arc<Mutex<NodeRefInner>>,
}

#[derive(Default)]
struct NodeRefInner {
  x: f32,
  y: f32,
  width: f32,
  height: f32,
  attached: bool,
}

impl NodeRef {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn x(&self) -> f32 {
    self.inner.lock().unwrap().x
  }

  pub fn y(&self) -> f32 {
    self.inner.lock().unwrap().y
  }

  pub fn width(&self) -> f32 {
    self.inner.lock().unwrap().width
  }

  pub fn height(&self) -> f32 {
    self.inner.lock().unwrap().height
  }

  pub fn rect(&self) -> (f32, f32, f32, f32) {
    let inner = self.inner.lock().unwrap();
    (inner.x, inner.y, inner.width, inner.height)
  }

  pub fn is_attached(&self) -> bool {
    self.inner.lock().unwrap().attached
  }

  pub(crate) fn update(&self, x: f32, y: f32, width: f32, height: f32) {
    let mut inner = self.inner.lock().unwrap();
    inner.x = x;
    inner.y = y;
    inner.width = width;
    inner.height = height;
    inner.attached = true;
  }
}
