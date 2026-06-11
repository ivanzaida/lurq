use std::sync::Arc;

use crate::layout::{Offset, Size};

#[derive(Clone)]
pub struct LayoutResult {
  pub size: Size,
  pub children: Vec<ChildLayout>,
}

#[derive(Clone)]
pub struct ChildLayout {
  pub offset: Offset,
  pub result: Arc<LayoutResult>,
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn cloning_layout_result_shares_nested_child_results() {
    let result = LayoutResult {
      size: Size::new(10.0, 10.0),
      children: vec![ChildLayout {
        offset: Offset::default(),
        result: LayoutResult {
          size: Size::new(5.0, 5.0),
          children: Vec::new(),
        }
        .into(),
      }],
    };

    let cloned = result.clone();

    assert!(Arc::ptr_eq(&result.children[0].result, &cloned.children[0].result));
  }
}
