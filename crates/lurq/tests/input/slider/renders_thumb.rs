use lurq::{app::Runtime, core::Signal, node::color::Color};

use crate::support::render_pass;

#[test]
fn slider_renders_track_and_thumb() {
  let value = Signal::new(5.0);
  let mut runtime = Runtime::new();

  runtime.set_root(lurq::components::Slider::new(value).range(0.0, 10.0).width(100.0));
  let snapshot = render_pass(&mut runtime);

  assert!(
    snapshot
      .rects
      .iter()
      .any(|rect| rect.color == Color::from_hex("#cbd5e1"))
  );
  assert!(
    snapshot
      .rects
      .iter()
      .any(|rect| rect.color == Color::from_hex("#475569") && rect.width > 0.0 && rect.height > 0.0)
  );
}
