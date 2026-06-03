use std::collections::HashMap;

use crate::node::color::Color;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PaletteId(u8);

impl PaletteId {
  pub const fn new(id: u8) -> Self {
    Self(id)
  }

  pub const fn get(self) -> u8 {
    self.0
  }
}

impl From<u8> for PaletteId {
  fn from(id: u8) -> Self {
    Self::new(id)
  }
}

impl From<&PaletteId> for PaletteId {
  fn from(id: &PaletteId) -> Self {
    *id
  }
}

#[derive(Clone)]
pub struct ThemePalette {
  colors: HashMap<PaletteId, Color>,
}

impl ThemePalette {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn from_colors<I, K>(colors: I) -> Self
  where
    I: IntoIterator<Item = (K, Color)>,
    K: Into<PaletteId>,
  {
    Self {
      colors: colors.into_iter().map(|(id, color)| (id.into(), color)).collect(),
    }
  }

  pub fn colors(&self) -> &HashMap<PaletteId, Color> {
    &self.colors
  }

  pub fn set(&mut self, id: impl Into<PaletteId>, color: Color) {
    self.colors.insert(id.into(), color);
  }

  pub fn register(&mut self, color: Color) -> PaletteId {
    let id = self.next_available_id();
    self.colors.insert(id, color);
    id
  }

  pub fn get(&self, id: impl Into<PaletteId>) -> Option<&Color> {
    self.colors.get(&id.into())
  }

  pub fn resolve(&self, id: impl Into<PaletteId>) -> Color {
    self
      .colors
      .get(&id.into())
      .copied()
      .unwrap_or_else(|| Color::new(0, 0, 0, 0))
  }

  fn next_available_id(&self) -> PaletteId {
    for raw in 0..=u8::MAX {
      let id = PaletteId::new(raw);
      if !self.colors.contains_key(&id) {
        return id;
      }
    }
    panic!("no palette ids available");
  }
}

impl Default for ThemePalette {
  fn default() -> Self {
    Self { colors: HashMap::new() }
  }
}
