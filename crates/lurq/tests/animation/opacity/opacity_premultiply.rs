use lurq::{
  app::Runtime,
  layout::{Constraints, Size},
  node::color::Color,
};

use crate::support::render_pass;

#[test]
fn half_opacity_halves_alpha_in_rendered_rect() {
  let mut rt = Runtime::new();
  let node = lurq::components::Rect::new(100.0, 50.0)
    .background(Color::new(255, 0, 0, 200))
    .opacity(0.5);
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  let snap = render_pass(&mut rt);

  assert!(!snap.rects.is_empty());
  assert_eq!(snap.rects[0].color.r(), 255);
  assert_eq!(snap.rects[0].color.a(), 100);
}

#[test]
fn full_opacity_preserves_alpha() {
  let mut rt = Runtime::new();
  let node = lurq::components::Rect::new(100.0, 50.0)
    .background(Color::new(255, 0, 0, 200))
    .opacity(1.0);
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  let snap = render_pass(&mut rt);

  assert_eq!(snap.rects[0].color.a(), 200);
}

#[test]
fn zero_opacity_zeros_alpha() {
  let mut rt = Runtime::new();
  let node = lurq::components::Rect::new(100.0, 50.0)
    .background(Color::new(255, 0, 0, 200))
    .opacity(0.0);
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  let snap = render_pass(&mut rt);

  assert_eq!(snap.rects[0].color.a(), 0);
}

#[test]
fn opacity_does_not_change_rgb_channels() {
  let mut rt = Runtime::new();
  let node = lurq::components::Rect::new(100.0, 50.0)
    .background(Color::new(100, 150, 200, 255))
    .opacity(0.5);
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  let snap = render_pass(&mut rt);

  assert_eq!(snap.rects[0].color.r(), 100);
  assert_eq!(snap.rects[0].color.g(), 150);
  assert_eq!(snap.rects[0].color.b(), 200);
  assert_eq!(snap.rects[0].color.a(), 128);
}
