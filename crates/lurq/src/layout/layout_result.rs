use crate::layout::{Offset, Size};

#[derive(Clone)]
pub struct LayoutResult {
  pub size: Size,
  pub children: Vec<ChildLayout>,
}

#[derive(Clone)]
pub struct ChildLayout {
  pub offset: Offset,
  pub result: LayoutResult,
}

impl LayoutResult {
  pub(crate) fn estimated_memory_bytes(&self) -> usize {
    std::mem::size_of::<Self>()
      + self.children.capacity() * std::mem::size_of::<ChildLayout>()
      + self
        .children
        .iter()
        .map(|child| child.result.estimated_memory_bytes())
        .sum::<usize>()
  }
}
