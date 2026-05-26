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
