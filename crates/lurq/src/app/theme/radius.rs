use std::collections::HashMap;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RadiusId(u8);

impl RadiusId {
  pub const fn new(id: u8) -> Self {
    Self(id)
  }

  pub const fn get(self) -> u8 {
    self.0
  }
}

impl From<u8> for RadiusId {
  fn from(id: u8) -> Self {
    Self::new(id)
  }
}

#[derive(Clone)]
pub struct ThemeRadii {
  values: HashMap<RadiusId, f32>,
}

impl ThemeRadii {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn from_values<I, K>(values: I) -> Self
  where
    I: IntoIterator<Item = (K, f32)>,
    K: Into<RadiusId>,
  {
    Self {
      values: values.into_iter().map(|(id, value)| (id.into(), value)).collect(),
    }
  }

  pub fn values(&self) -> &HashMap<RadiusId, f32> {
    &self.values
  }

  pub fn set(&mut self, id: impl Into<RadiusId>, value: f32) {
    self.values.insert(id.into(), value);
  }

  pub fn register(&mut self, value: f32) -> RadiusId {
    let id = self.next_available_id();
    self.values.insert(id, value);
    id
  }

  pub fn get(&self, id: impl Into<RadiusId>) -> Option<f32> {
    self.values.get(&id.into()).copied()
  }

  fn next_available_id(&self) -> RadiusId {
    for raw in 0..=u8::MAX {
      let id = RadiusId::new(raw);
      if !self.values.contains_key(&id) {
        return id;
      }
    }
    panic!("no radius ids available");
  }
}

impl Default for ThemeRadii {
  fn default() -> Self {
    Self { values: HashMap::new() }
  }
}
