//! `GpuViewport`'s box: the size it has when it is given none, and when it is given one side, as a `Canvas`'s.

use lurq::{
  app::Tree,
  components::{Column, GpuViewport, Row},
  images::WgpuExternalImageSlot,
};

use crate::support::run_pass;

/// The size of the viewport with the id `viewport` once `root` is laid out in a window of 800 by 600.
fn laid_out(root: impl Into<lurq::node::Element>) -> (f32, f32) {
  let mut tree = Tree::new();
  tree.resize(800, 600);
  tree.set_root(root);
  run_pass(&mut tree);
  let bounds = tree
    .get_element_by_id_mut("viewport")
    .and_then(|element| element.bounds())
    .expect("the viewport is laid out");
  (bounds.width, bounds.height)
}

#[test]
fn a_viewport_given_no_size_is_a_canvas_of_300_by_150() {
  let viewport = || GpuViewport::new(WgpuExternalImageSlot::new()).id("viewport");
  // In a column and in a row, whose heights are their content's.
  assert_eq!(laid_out(Column::new().child(viewport())), (300.0, 150.0));
  assert_eq!(laid_out(Row::new().child(viewport())), (300.0, 150.0));
}

#[test]
fn a_viewport_given_one_side_keeps_the_other_as_a_canvas_does() {
  let viewport = || GpuViewport::new(WgpuExternalImageSlot::new()).id("viewport");
  // Not a picture's shape: the app draws to whatever box it is.
  assert_eq!(laid_out(Column::new().child(viewport().width(400.0))), (400.0, 150.0));
  assert_eq!(laid_out(Column::new().child(viewport().height(100.0))), (300.0, 100.0));
  let canvas = lurq::components::Canvas::new().width(400.0).id("viewport");
  assert_eq!(laid_out(Column::new().child(canvas)), (400.0, 150.0));
  assert_eq!(
    laid_out(Column::new().child(viewport().size(480.0, 270.0))),
    (480.0, 270.0)
  );
  // A share of the room, as any box.
  let filled = Column::new()
    .size(800.0, 600.0)
    .child(viewport().width(lurq::node::dimension::Dimension::Pct(100.0)).flex(1.0));
  assert_eq!(laid_out(filled), (800.0, 600.0));
}
