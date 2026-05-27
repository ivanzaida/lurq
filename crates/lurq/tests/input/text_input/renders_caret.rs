use lurq::{
  app::{Runtime, events::MouseButton},
  core::Signal,
  layout::{Constraints, Size, quad::QuadContent},
  node::Element,
};

#[test]
fn renders_caret_after_text_input_is_focused() {
  let value = Signal::new("A".to_owned());
  let mut runtime = Runtime::new();

  runtime.set_root(Element::text_input(value));
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);
  let layout = runtime
    .compute_layout(Constraints::tight(Size::new(200.0, 80.0)))
    .unwrap();
  let quads = runtime.resolve_quads(&layout);

  assert!(
    quads
      .iter()
      .any(|quad| { matches!(quad.content, QuadContent::Rect { .. }) && quad.width == 1.0 && quad.height > 0.0 })
  );
}
