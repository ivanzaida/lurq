#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CaretMode {
  Persistent,
  #[default]
  Blinking,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeCaret {
  mode: CaretMode,
}

impl ThemeCaret {
  pub fn new(mode: CaretMode) -> Self {
    Self { mode }
  }

  pub fn mode(&self) -> CaretMode {
    self.mode
  }

  pub fn set_mode(&mut self, mode: CaretMode) {
    self.mode = mode;
  }
}

impl Default for ThemeCaret {
  fn default() -> Self {
    Self {
      mode: CaretMode::Blinking,
    }
  }
}
