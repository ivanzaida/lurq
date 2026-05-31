use crate::layout::StackAlignment;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Alignment {
  #[default]
  Start,
  Center,
  End,
  Stretch,
}

impl Alignment {
  pub fn to_stack_alignment(&self) -> StackAlignment {
    match self {
      Self::Start => StackAlignment::TopStart,
      Self::Center => StackAlignment::Center,
      Self::End => StackAlignment::BottomEnd,
      Self::Stretch => StackAlignment::Center,
    }
  }

  pub fn cross_offset(&self, container_cross: f32, child_cross: f32) -> f32 {
    match self {
      Self::Start => 0.0,
      Self::Center => (container_cross - child_cross) / 2.0,
      Self::End => container_cross - child_cross,
      Self::Stretch => 0.0,
    }
  }
}
