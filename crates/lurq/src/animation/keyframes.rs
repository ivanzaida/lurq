use super::{
  easing::Easing,
  interpolate::{AnimatableProperty, AnimatableValue},
};
use crate::node::transform::Decomposed;

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

  /// The opacity at this frame, from 0 (none of it seen) to 1.
  pub fn opacity(&mut self, opacity: f32) -> &mut Self {
    self.set(AnimatableProperty::Opacity, opacity)
  }

  /// How far the element is moved at this frame, in logical pixels: right and down. It is the element's transform,
  /// with the scale of [`Self::scale`] at the same frame.
  pub fn translate(&mut self, x: f32, y: f32) -> &mut Self {
    let at = self.transform().with_translate(x, y);
    self.put_transform(at)
  }

  /// How large the element is drawn at this frame, about its middle: 1 is its size. It is the element's transform,
  /// with the move of [`Self::translate`] at the same frame.
  pub fn scale(&mut self, scale: f32) -> &mut Self {
    let at = self.transform().with_scale(scale, scale);
    self.put_transform(at)
  }

  /// The transform this frame sets so far.
  fn transform(&self) -> Decomposed {
    self
      .values
      .iter()
      .rev()
      .find_map(|(prop, value)| match (prop, value) {
        (AnimatableProperty::Transform, AnimatableValue::Transform(at)) => Some(*at),
        _ => None,
      })
      .unwrap_or(Decomposed::IDENTITY)
  }

  fn put_transform(&mut self, at: Decomposed) -> &mut Self {
    self.values.retain(|(prop, _)| *prop != AnimatableProperty::Transform);
    self.set(AnimatableProperty::Transform, at)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_frame_that_moves_and_scales_sets_one_transform_of_both() {
    let keyframes = Keyframes::new(1).frame(0.0, |k| {
      k.opacity(0.5).translate(0.0, 4.0).scale(0.98);
    });
    let values = &keyframes.frames[0].values;
    assert_eq!(values.len(), 2, "{values:?}");
    let at = Decomposed::IDENTITY.with_translate(0.0, 4.0).with_scale(0.98, 0.98);
    assert!(values.contains(&(AnimatableProperty::Opacity, AnimatableValue::Float(0.5))));
    assert!(values.contains(&(AnimatableProperty::Transform, AnimatableValue::Transform(at))));
  }
}
