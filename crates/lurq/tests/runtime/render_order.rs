use lurq::{
  app::Tree,
  components::{Image, Rect, Stack},
  images::ImageData,
};

use crate::support::render_pass;

#[test]
fn image_command_carries_structural_order() {
  let image = ImageData::from_rgba(vec![255, 255, 255, 255], 1, 1);
  let mut runtime = Tree::new();
  runtime.set_root(
    Stack::new()
      .child(Rect::new(10.0, 10.0).fill("#ef4444"))
      .child(Image::new(image).size(10.0, 10.0))
      .child(Rect::new(10.0, 10.0).fill("#22c55e")),
  );

  let snapshot = render_pass(&mut runtime);

  assert_eq!(snapshot.rects.len(), 2);
  assert_eq!(snapshot.image_orders, vec![1]);
}
