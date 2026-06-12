use std::sync::{Arc, RwLock};

#[derive(Clone, Default)]
pub struct ElementRef {
  inner: Arc<RwLock<ElementRefInner>>,
}

#[derive(Clone, Default)]
pub struct ElementRefMut {
  inner: Arc<RwLock<ElementRefInner>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ElementRect {
  pub x: f32,
  pub y: f32,
  pub relative_x: f32,
  pub relative_y: f32,
  pub width: f32,
  pub height: f32,
}

impl ElementRect {
  pub fn center(&self) -> (f32, f32) {
    (self.x + self.width / 2.0, self.y + self.height / 2.0)
  }
}

#[derive(Default)]
struct ElementRefInner {
  x: f32,
  y: f32,
  relative_x: f32,
  relative_y: f32,
  width: f32,
  height: f32,
  attached: bool,
  hovered: bool,
  active: bool,
  focused: bool,
  override_rect: Option<ElementRect>,
  layout_dirty: bool,
}

impl ElementRef {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn mutable(&self) -> ElementRefMut {
    ElementRefMut {
      inner: self.inner.clone(),
    }
  }

  pub fn x(&self) -> f32 {
    self.inner.read().unwrap().x
  }

  pub fn y(&self) -> f32 {
    self.inner.read().unwrap().y
  }

  pub fn width(&self) -> f32 {
    self.inner.read().unwrap().width
  }

  pub fn height(&self) -> f32 {
    self.inner.read().unwrap().height
  }

  pub fn rect(&self) -> (f32, f32, f32, f32) {
    let inner = self.inner.read().unwrap();
    (inner.x, inner.y, inner.width, inner.height)
  }

  pub fn bounds(&self) -> ElementRect {
    let inner = self.inner.read().unwrap();
    ElementRect {
      x: inner.x,
      y: inner.y,
      relative_x: inner.relative_x,
      relative_y: inner.relative_y,
      width: inner.width,
      height: inner.height,
    }
  }

  pub fn is_attached(&self) -> bool {
    self.inner.read().unwrap().attached
  }

  pub fn hovered(&self) -> bool {
    self.inner.read().unwrap().hovered
  }

  pub fn active(&self) -> bool {
    self.inner.read().unwrap().active
  }

  pub fn focused(&self) -> bool {
    self.inner.read().unwrap().focused
  }

  pub(crate) fn update(&self, x: f32, y: f32, relative_x: f32, relative_y: f32, width: f32, height: f32) {
    let mut inner = self.inner.write().unwrap();
    inner.x = x;
    inner.y = y;
    inner.relative_x = relative_x;
    inner.relative_y = relative_y;
    inner.width = width;
    inner.height = height;
    inner.attached = true;
  }

  pub(crate) fn same_handle(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.inner, &other.inner)
  }

  pub(crate) fn override_rect(&self) -> Option<ElementRect> {
    self.inner.read().unwrap().override_rect
  }

  pub(crate) fn has_layout_dirty(&self) -> bool {
    self.inner.read().unwrap().layout_dirty
  }

  pub(crate) fn take_layout_dirty(&self) -> bool {
    let mut inner = self.inner.write().unwrap();
    let dirty = inner.layout_dirty;
    inner.layout_dirty = false;
    dirty
  }

  pub(crate) fn set_hovered(&self, hovered: bool) {
    self.inner.write().unwrap().hovered = hovered;
  }

  pub(crate) fn set_active(&self, active: bool) {
    self.inner.write().unwrap().active = active;
  }

  pub(crate) fn set_focused(&self, focused: bool) {
    self.inner.write().unwrap().focused = focused;
  }
}

impl ElementRefMut {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn as_ref(&self) -> ElementRef {
    ElementRef {
      inner: self.inner.clone(),
    }
  }

  pub fn x(&self) -> f32 {
    self.as_ref().x()
  }

  pub fn y(&self) -> f32 {
    self.as_ref().y()
  }

  pub fn width(&self) -> f32 {
    self.as_ref().width()
  }

  pub fn height(&self) -> f32 {
    self.as_ref().height()
  }

  pub fn rect(&self) -> (f32, f32, f32, f32) {
    self.as_ref().rect()
  }

  pub fn bounds(&self) -> ElementRect {
    self.as_ref().bounds()
  }

  pub fn set_bounds(&self, rect: ElementRect) {
    let mut inner = self.inner.write().unwrap();
    inner.x = rect.x;
    inner.y = rect.y;
    inner.relative_x = rect.relative_x;
    inner.relative_y = rect.relative_y;
    inner.width = rect.width;
    inner.height = rect.height;
    inner.override_rect = Some(rect);
    inner.layout_dirty = true;
  }

  pub fn set_relative_bounds(&self, relative_x: f32, relative_y: f32, width: f32, height: f32) {
    let current = self.bounds();
    let parent_x = current.x - current.relative_x;
    let parent_y = current.y - current.relative_y;
    self.set_bounds(ElementRect {
      x: parent_x + relative_x,
      y: parent_y + relative_y,
      relative_x,
      relative_y,
      width,
      height,
    });
  }

  pub fn clear_bounds_override(&self) {
    let mut inner = self.inner.write().unwrap();
    inner.override_rect = None;
    inner.layout_dirty = true;
  }

  pub fn is_attached(&self) -> bool {
    self.as_ref().is_attached()
  }

  pub fn hovered(&self) -> bool {
    self.as_ref().hovered()
  }

  pub fn active(&self) -> bool {
    self.as_ref().active()
  }

  pub fn focused(&self) -> bool {
    self.as_ref().focused()
  }
}

impl From<ElementRefMut> for ElementRef {
  fn from(value: ElementRefMut) -> Self {
    value.as_ref()
  }
}
