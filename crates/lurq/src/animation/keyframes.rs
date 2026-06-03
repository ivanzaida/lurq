use super::{
  easing::Easing,
  interpolate::{AnimatableProperty, AnimatableValue},
};

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyframesId(u16);

impl KeyframesId {
  pub const fn new(id: u16) -> Self {
    Self(id)
  }

  pub const fn get(self) -> u16 {
    self.0
  }
}

impl From<u16> for KeyframesId {
  fn from(value: u16) -> Self {
    Self::new(value)
  }
}

#[derive(Clone, Debug)]
pub struct KeyframeEntry {
  pub offset: f32,
  pub values: Vec<(AnimatableProperty, AnimatableValue)>,
  pub easing: Option<Easing>,
}

#[derive(Clone, Debug)]
pub struct Keyframes {
  pub id: KeyframesId,
  pub frames: Vec<KeyframeEntry>,
}

impl Keyframes {
  pub fn new(id: impl Into<KeyframesId>) -> Self {
    Self {
      id: id.into(),
      frames: Vec::new(),
    }
  }

  pub fn frame(mut self, offset: f32, f: impl FnOnce(&mut KeyframeBuilder)) -> Self {
    let mut builder = KeyframeBuilder {
      values: Vec::new(),
      easing: None,
    };
    f(&mut builder);
    self.frames.push(KeyframeEntry {
      offset,
      values: builder.values,
      easing: builder.easing,
    });
    self.frames.sort_by(|a, b| a.offset.partial_cmp(&b.offset).unwrap());
    self
  }
}

pub struct KeyframeBuilder {
  values: Vec<(AnimatableProperty, AnimatableValue)>,
  easing: Option<Easing>,
}

impl KeyframeBuilder {
  pub fn set(&mut self, prop: AnimatableProperty, value: impl Into<AnimatableValue>) -> &mut Self {
    self.values.push((prop, value.into()));
    self
  }

  pub fn easing(&mut self, e: Easing) -> &mut Self {
    self.easing = Some(e);
    self
  }
}
