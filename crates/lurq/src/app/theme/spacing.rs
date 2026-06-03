use std::collections::HashMap;

use crate::node::dimension::Dimension;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SpacingId(u8);

impl SpacingId {
  pub const fn new(id: u8) -> Self {
    Self(id)
  }

  pub const fn get(self) -> u8 {
    self.0
  }
}

impl From<u8> for SpacingId {
  fn from(id: u8) -> Self {
    Self::new(id)
  }
}

impl From<&SpacingId> for SpacingId {
  fn from(id: &SpacingId) -> Self {
    *id
  }
}

#[derive(Clone)]
pub struct ThemeSpacing {
  values: HashMap<SpacingId, Dimension>,
}

impl ThemeSpacing {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn from_values<I, K, V>(values: I) -> Self
  where
    I: IntoIterator<Item = (K, V)>,
    K: Into<SpacingId>,
    V: Into<Dimension>,
  {
    Self {
      values: values
        .into_iter()
        .map(|(id, value)| (id.into(), value.into()))
        .collect(),
    }
  }

  pub fn values(&self) -> &HashMap<SpacingId, Dimension> {
    &self.values
  }

  pub fn set(&mut self, id: impl Into<SpacingId>, value: impl Into<Dimension>) {
    self.values.insert(id.into(), value.into());
  }

  pub fn register(&mut self, value: impl Into<Dimension>) -> SpacingId {
    let id = self.next_available_id();
    self.values.insert(id, value.into());
    id
  }

  pub fn get(&self, id: impl Into<SpacingId>) -> Option<Dimension> {
    self.values.get(&id.into()).copied()
  }

  fn next_available_id(&self) -> SpacingId {
    for raw in 0..=u8::MAX {
      let id = SpacingId::new(raw);
      if !self.values.contains_key(&id) {
        return id;
      }
    }
    panic!("no spacing ids available");
  }
}

impl Default for ThemeSpacing {
  fn default() -> Self {
    Self { values: HashMap::new() }
  }
}
