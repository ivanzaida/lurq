use crate::layout::{Offset, Size};

#[derive(Clone, Copy, Default)]
pub enum StackAlignment {
  TopStart,
  TopCenter,
  TopEnd,
  CenterStart,
  #[default]
  Center,
  CenterEnd,
  BottomStart,
  BottomCenter,
  BottomEnd,
}

impl StackAlignment {
  pub fn resolve_offset(&self, container: Size, child: Size) -> Offset {
    let x = match self {
      Self::TopStart | Self::CenterStart | Self::BottomStart => 0.0,
      Self::TopCenter | Self::Center | Self::BottomCenter => (container.width - child.width) / 2.0,
      Self::TopEnd | Self::CenterEnd | Self::BottomEnd => container.width - child.width,
    };
    let y = match self {
      Self::TopStart | Self::TopCenter | Self::TopEnd => 0.0,
      Self::CenterStart | Self::Center | Self::CenterEnd => (container.height - child.height) / 2.0,
      Self::BottomStart | Self::BottomCenter | Self::BottomEnd => container.height - child.height,
    };
    Offset::new(x, y)
  }
}
