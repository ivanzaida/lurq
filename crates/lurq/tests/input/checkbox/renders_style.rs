use lurq::{app::Tree, core::Signal, node::color::Color};

use crate::support::render_pass;

#[test]
fn checked_checkbox_style_controls_box_fill() {
  let checked = Signal::new(true);
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::Checkbox::new(checked)
      .checked_box(|style| style.background("#111827"))
      .size(20.0, 20.0),
  );
  let snapshot = render_pass(&mut runtime);

  assert!(
    snapshot
      .rects
      .iter()
      .any(|rect| { rect.color == Color::from_hex("#111827") && rect.width == 20.0 && rect.height == 20.0 })
  );
}

#[test]
fn generic_fill_still_styles_unchecked_checkbox() {
  let checked = Signal::new(false);
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::Checkbox::new(checked).background("#f8fafc"));
  let snapshot = render_pass(&mut runtime);

  assert!(
    snapshot
      .rects
      .iter()
      .any(|rect| rect.color == Color::from_hex("#f8fafc"))
  );
}

#[cfg(feature = "image")]
#[test]
fn checked_checkbox_renders_indicator_image() {
  let checked = Signal::new(true);
  let indicator = lurq::images::ImageData::from_rgba(vec![255; 4 * 2 * 2], 2, 2);
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::Checkbox::new(checked)
      .box_part(|style| style.indicator_image(indicator).indicator_size(8.0, 8.0))
      .size(20.0, 20.0),
  );
  let snapshot = render_pass(&mut runtime);

  assert_eq!(snapshot.image_orders.len(), 1);
}

#[cfg(feature = "image")]
#[test]
fn unchecked_checkbox_does_not_render_indicator_image() {
  let checked = Signal::new(false);
  let indicator = lurq::images::ImageData::from_rgba(vec![255; 4 * 2 * 2], 2, 2);
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::Checkbox::new(checked)
      .box_part(|style| style.indicator_image(indicator).indicator_size(8.0, 8.0))
      .size(20.0, 20.0),
  );
  let snapshot = render_pass(&mut runtime);

  assert!(snapshot.image_orders.is_empty());
}
