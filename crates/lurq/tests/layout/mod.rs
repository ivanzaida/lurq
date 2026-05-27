use lurq::{
  app::Runtime,
  layout::{Constraints, layout_result::LayoutResult},
};

use crate::support::run_pass;

trait PassLayoutExt {
  fn pass_layout(&mut self, constraints: Constraints) -> Option<LayoutResult>;
}

impl PassLayoutExt for Runtime {
  fn pass_layout(&mut self, constraints: Constraints) -> Option<LayoutResult> {
    self.set_layout_constraints_override(Some(constraints));
    run_pass(self);
    let result = self.last_layout().cloned();
    self.set_layout_constraints_override(None);
    result
  }
}

mod column;
mod constraints;
mod edge_cases;
mod flex;
mod frame;
mod nested;
mod node_ids;
mod offset;
mod padding;
mod ported_flex;
mod quads;
mod row;
mod scroll;
mod stack;
mod text_centering;
