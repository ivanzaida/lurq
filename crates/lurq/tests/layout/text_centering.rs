use std::sync::{Arc, Mutex};

use lurq::{
  app::{Runtime, component::Component, ctx::Ctx},
  core::Signal,
  layout::{
    Alignment, Constraints, Size,
    quad::QuadContent,
    text_style::{FontWeight, TextStyle},
  },
  node::{Element, color::Color},
};

use super::PassLayoutExt;

fn rt() -> Runtime {
  Runtime::new()
}

struct Shared<T>(Arc<T>);

impl<T> Clone for Shared<T> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<T> PartialEq for Shared<T> {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

struct TextCounterHost;

impl Component for TextCounterHost {
  type Props = Shared<Mutex<Option<Signal<i32>>>>;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Row::new()
      .align_items(Alignment::Center)
      .child(ctx.mount::<TextCounter>(ctx.props::<Self::Props>().clone()))
  }
}

struct TextCounter {
  count: Signal<i32>,
}

impl Component for TextCounter {
  type Props = Shared<Mutex<Option<Signal<i32>>>>;

  fn create(ctx: &mut Ctx) -> Self {
    let count = ctx.signal(9);
    *ctx.props::<Self::Props>().0.lock().unwrap() = Some(count.clone());
    Self { count }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Text::styled(
      &self.count.get().to_string(),
      TextStyle {
        font_size: 24.0,
        weight: FontWeight::Bold,
        color: Color::from_hex("#1e293b"),
        ..TextStyle::default()
      },
    )
  }
}

#[test]
fn text_height_equals_line_height() {
  let mut rt = rt();
  let style = TextStyle {
    font_size: 24.0,
    ..TextStyle::default()
  };
  let node = lurq::components::Text::styled("0", style.clone());
  rt.set_root(node);
  let r = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let expected_height = style.font_size * style.line_height;
  assert!(
    (r.size.height - expected_height).abs() < 1.0,
    "text height should be ~{} (font_size * line_height), got {}",
    expected_height,
    r.size.height
  );
}

#[test]
fn nowrap_text_keeps_intrinsic_single_line_width() {
  let mut rt = rt();
  let style = TextStyle {
    font_size: 14.0,
    ..TextStyle::default()
  };
  rt.set_root(lurq::components::Text::styled("x: 1279  y: 450  |  entered: true", style).nowrap());
  let r = rt.pass_layout(Constraints::loose(Size::new(120.0, 80.0))).unwrap();

  assert!(
    r.size.width > 120.0,
    "nowrap text should keep its intrinsic width instead of wrapping to the parent constraint"
  );

  let quads = rt.resolve_quads(&r);
  let text_quad = quads
    .iter()
    .find(|quad| matches!(quad.content, QuadContent::Text { .. }))
    .expect("nowrap text should emit a text quad");
  assert_eq!(text_quad.width, r.size.width);
}

#[test]
fn changed_text_content_invalidates_ancestor_layout_cache() {
  let count = Arc::new(Mutex::new(None));
  let mut rt = rt();
  rt.mount_root::<TextCounterHost>(Shared(count.clone()));

  let one_digit = rt.pass_layout(Constraints::loose(Size::new(400.0, 100.0))).unwrap();
  let one_digit_width = one_digit.children[0].result.size.width;
  let one_digit_height = one_digit.children[0].result.size.height;

  count.lock().unwrap().as_ref().unwrap().set(10);

  let two_digit = rt.pass_layout(Constraints::loose(Size::new(400.0, 100.0))).unwrap();
  let two_digit_width = two_digit.children[0].result.size.width;
  let two_digit_height = two_digit.children[0].result.size.height;

  assert!(
    two_digit_width > one_digit_width,
    "updated text should be remeasured wider than the previous single digit: {} <= {}",
    two_digit_width,
    one_digit_width
  );
  assert!(
    (two_digit_height - one_digit_height).abs() < 1.0,
    "updated text should stay single-line height: {} vs {}",
    two_digit_height,
    one_digit_height
  );
}

#[test]
fn text_vertically_centered_in_row_with_rects() {
  let mut rt = rt();
  let node = lurq::components::Row::new()
    .spacing(12.0)
    .align_items(Alignment::Center)
    .child(lurq::components::Rect::new(36.0, 36.0))
    .child(lurq::components::Text::styled(
      "0",
      TextStyle {
        font_size: 24.0,
        weight: FontWeight::Bold,
        color: Color::from_hex("#1e293b"),
        ..TextStyle::default()
      },
    ))
    .child(lurq::components::Rect::new(36.0, 36.0));
  rt.set_root(node);
  let r = rt.pass_layout(Constraints::loose(Size::new(400.0, 100.0))).unwrap();

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
  let node = lurq::components::Row::new()
    .align_items(Alignment::Center)
    .child(lurq::components::Text::styled(
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
  let r = rt.pass_layout(Constraints::loose(Size::new(400.0, 100.0))).unwrap();

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
