use crate::node::Element;

pub struct Slot;

impl From<Slot> for Element {
  fn from(_value: Slot) -> Self {
    Element::new()
  }
}
