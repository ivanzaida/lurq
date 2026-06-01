use lurq::{app::Tree, core::Signal, node::color::Color};

use crate::support::render_pass;

#[test]
fn slider_renders_track_and_thumb() {
  let value = Signal::new(5);
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::Slider::new(value).range(0, 10).width(100.0));
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

#[test]
fn slider_customizes_track_and_thumb_geometry_and_visuals() {
  let value = Signal::new(5);
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::Slider::new(value)
      .range(0, 10)
      .width(100.0)
      .height(20.0)
      .track(|style| {
        style
          .size(80.0, 2.0)
          .fill("#111827")
          .rounded(1.0)
          .border_inside(1.0, Color::from_hex("#38bdf8"))
      })
      .thumb(|style| {
        style
          .size(10.0, 10.0)
          .fill("#f97316")
          .rounded(5.0)
          .border_inside(2.0, Color::from_hex("#0f172a"))
      }),
  );
  let snapshot = render_pass(&mut runtime);

  let track = snapshot
    .rects
    .iter()
    .find(|rect| rect.color == Color::from_hex("#111827"))
    .expect("custom track should render");
  let thumb = snapshot
    .rects
    .iter()
    .find(|rect| rect.color == Color::from_hex("#f97316"))
    .expect("custom thumb should render");

  assert_eq!(track.width, 80.0);
  assert_eq!(track.height, 2.0);
  assert_eq!(track.radii, [1.0; 4]);
  assert_eq!(thumb.width, 10.0);
  assert_eq!(thumb.height, 10.0);
  assert_eq!(thumb.radii, [5.0; 4]);
  assert!(snapshot.rects.iter().any(|rect| {
    rect.stroke == [1.0; 4] && rect.stroke_color == Color::from_hex("#38bdf8") && rect.radii == [1.0; 4]
  }));
  assert!(snapshot.rects.iter().any(|rect| {
    rect.stroke == [2.0; 4] && rect.stroke_color == Color::from_hex("#0f172a") && rect.radii == [5.0; 4]
  }));
  assert_eq!(track.y + track.height / 2.0, thumb.y + thumb.height / 2.0);
  assert_eq!(track.x + track.width / 2.0, thumb.x + thumb.width / 2.0);
}

#[test]
fn slider_applies_track_and_thumb_hover_styles() {
  let value = Signal::new(5);
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::Slider::new(value)
      .range(0, 10)
      .width(100.0)
      .height(20.0)
      .track(|style| style.height(4.0).fill("#64748b"))
      .track_hovered(|style| style.fill("#22c55e"))
      .thumb(|style| style.size(10.0, 10.0).fill("#0f172a"))
      .thumb_hovered(|style| style.fill("#eab308")),
  );
  render_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.mouse_move(x, y);
  let snapshot = render_pass(&mut runtime);

  assert!(
    snapshot
      .rects
      .iter()
      .any(|rect| rect.color == Color::from_hex("#22c55e"))
  );
  assert!(
    snapshot
      .rects
      .iter()
      .any(|rect| rect.color == Color::from_hex("#eab308"))
  );
}
