use lurq::{
  app::Runtime,
  core::Signal,
  layout::{Constraints, Size, quad::QuadContent},
  node::{Element, color::Color},
};

#[test]
fn slider_renders_track_and_thumb() {
  let value = Signal::new(5.0);
  let mut runtime = Runtime::new();

  runtime.set_root(Element::slider(value).range(0.0, 10.0).width(100.0));
  let layout = runtime
    .compute_layout(Constraints::tight(Size::new(200.0, 80.0)))
    .unwrap();
  let quads = runtime.resolve_quads(&layout);

  assert!(quads.iter().any(|quad| {
    matches!(
      quad.content,
      QuadContent::Rect { color } if color == Color::from_hex("#cbd5e1")
    )
  }));
  assert!(quads.iter().any(|quad| {
    matches!(
      quad.content,
      QuadContent::Rect { color } if color == Color::from_hex("#475569")
    ) && quad.width > 0.0
      && quad.height > 0.0
  }));
}
