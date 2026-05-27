use lurq::{
  app::Runtime,
  layout::{
    Alignment, Constraints, Size,
    text_style::{FontWeight, TextStyle},
  },
  node::{Element, color::Color},
};

fn rt() -> Runtime {
  Runtime::new()
}

#[test]
fn text_height_equals_line_height() {
  let mut rt = rt();
  let style = TextStyle {
    font_size: 24.0,
    ..TextStyle::default()
  };
  let node = Element::styled_text("0", style.clone());
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let expected_height = style.font_size * style.line_height;
  assert!(
    (r.size.height - expected_height).abs() < 1.0,
    "text height should be ~{} (font_size * line_height), got {}",
    expected_height,
    r.size.height
  );
}

#[test]
fn text_vertically_centered_in_row_with_rects() {
  let mut rt = rt();
  let node = Element::row()
    .spacing(12.0)
    .align_items(Alignment::Center)
    .child(Element::rect(36.0, 36.0))
    .child(Element::styled_text(
      "0",
      TextStyle {
        font_size: 24.0,
        weight: FontWeight::Bold,
        color: Color::from_hex("#1e293b"),
        ..TextStyle::default()
      },
    ))
    .child(Element::rect(36.0, 36.0));
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::loose(Size::new(400.0, 100.0))).unwrap();

  let row_height = r.size.height;
  assert!(
    (row_height - 36.0).abs() < 1.0,
    "row height should be 36 (max child), got {}",
    row_height
  );

  let text_child = &r.children[1];
  let text_height = text_child.result.size.height;
  let text_center_y = text_child.offset.y + text_height / 2.0;
  let row_center_y = row_height / 2.0;

  assert!(
    (text_center_y - row_center_y).abs() < 1.0,
    "text center ({}) should match row center ({}), text_y={}, text_h={}",
    text_center_y,
    row_center_y,
    text_child.offset.y,
    text_height
  );
}

#[test]
fn text_vertically_centered_in_fixed_height_row() {
  let mut rt = rt();
  let node = Element::row()
    .align_items(Alignment::Center)
    .child(Element::styled_text(
      "Layout",
      TextStyle {
        font_size: 11.0,
        weight: FontWeight::Bold,
        color: Color::from_hex("#f8fafc"),
        ..TextStyle::default()
      },
    ))
    .width(200.0)
    .height(38.0);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::loose(Size::new(400.0, 100.0))).unwrap();

  let row = &r.children[0].result.children[0].result;
  let text_child = &row.children[0];
  let text_height = text_child.result.size.height;
  let text_center_y = text_child.offset.y + text_height / 2.0;

  assert!(
    text_height < row.size.height,
    "text child should keep intrinsic height inside fixed-height row, text_h={}, row_h={}",
    text_height,
    row.size.height
  );
  assert!(
    (text_center_y - row.size.height / 2.0).abs() < 1.0,
    "text center ({}) should match row center ({})",
    text_center_y,
    row.size.height / 2.0
  );
}
