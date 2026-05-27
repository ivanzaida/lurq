use lurq::{
  app::Runtime,
  layout::{
    Constraints, Size,
    layout_kind::FrameConstraints,
    quad::QuadContent,
    scrollbar::{ScrollBarStyle, ScrollBarVisibility},
  },
  node::{Element, color::Color},
};

use super::PassLayoutExt;

fn rt() -> Runtime {
  Runtime::new()
}

#[test]
fn scroll_vertical_child_grows_unbounded() {
  let mut rt = rt();
  let node = lurq::components::ScrollVertical::new(lurq::components::Column::new().spacing(0.0).with_children(
    (0..10).map(|_| {
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(100.0)),
        height: Some(lurq::node::dimension::Dimension::Px(50.0)),
        ..Default::default()
      })
    }),
  ))
  .size(100.0, 200.0);

  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  // Scroll container itself is 200 tall
  assert_eq!(result.size.height, 200.0);
  assert_eq!(result.size.width, 100.0);
  // The child (column) inside should be 500 tall (10 * 50)
  let scroll_child = &result.children[0].result;
  let column_child = &scroll_child.children[0].result;
  assert_eq!(column_child.size.height, 500.0);
}

#[test]
fn scroll_vertical_offset_applied() {
  let mut rt = rt();
  let node = lurq::components::ScrollVertical::new(lurq::components::Spacer::new().frame(FrameConstraints {
    width: Some(lurq::node::dimension::Dimension::Px(100.0)),
    height: Some(lurq::node::dimension::Dimension::Px(500.0)),
    ..Default::default()
  }))
  .size(100.0, 200.0);

  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  // Default scroll offset is 0
  let scroll_child = &result.children[0].result;
  assert_eq!(scroll_child.children[0].offset.y, 0.0);
}

#[test]
fn scroll_horizontal_child_grows_unbounded() {
  let mut rt = rt();
  let node = lurq::components::ScrollHorizontal::new(lurq::components::Row::new().spacing(0.0).with_children(
    (0..10).map(|_| {
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(100.0)),
        height: Some(lurq::node::dimension::Dimension::Px(50.0)),
        ..Default::default()
      })
    }),
  ))
  .size(200.0, 50.0);

  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 200.0);
  assert_eq!(result.size.height, 50.0);
  let scroll_child = &result.children[0].result;
  let row_child = &scroll_child.children[0].result;
  assert_eq!(row_child.size.width, 1000.0);
}

#[test]
fn scroll_both_unbounded() {
  let mut rt = rt();
  let node = lurq::components::ScrollBoth::new(lurq::components::Spacer::new().frame(FrameConstraints {
    width: Some(lurq::node::dimension::Dimension::Px(800.0)),
    height: Some(lurq::node::dimension::Dimension::Px(600.0)),
    ..Default::default()
  }))
  .size(200.0, 150.0);

  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 200.0);
  assert_eq!(result.size.height, 150.0);
  let scroll_child = &result.children[0].result;
  assert_eq!(scroll_child.children[0].result.size.width, 800.0);
  assert_eq!(scroll_child.children[0].result.size.height, 600.0);
}

#[test]
fn scroll_container_without_frame_uses_parent_constraints() {
  let mut rt = rt();
  let node = lurq::components::ScrollVertical::new(lurq::components::Column::new().spacing(0.0).with_children(
    (0..5).map(|_| {
      lurq::components::Spacer::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(100.0)),
        height: Some(lurq::node::dimension::Dimension::Px(40.0)),
        ..Default::default()
      })
    }),
  ));

  rt.set_root(node);
  let result = rt.pass_layout(Constraints::tight(Size::new(300.0, 100.0))).unwrap();
  // Container takes parent's tight constraint
  assert_eq!(result.size.width, 300.0);
  assert_eq!(result.size.height, 100.0);
  // Child column is taller
  assert_eq!(result.children[0].result.size.height, 200.0);
}

#[test]
fn scroll_empty_child() {
  let mut rt = rt();
  let node = lurq::components::ScrollVertical::new(Element::new()).size(100.0, 100.0);

  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 100.0);
  assert_eq!(result.size.height, 100.0);
}

#[test]
fn scrollbar_hovered_overrides_scrollbar_style() {
  let mut rt = rt();
  let thumb_color = Color::from_hex("#ef4444");

  let node = lurq::components::ScrollVertical::new(lurq::components::Spacer::new().height(300.0))
    .scrollbar(ScrollBarStyle {
      visible: ScrollBarVisibility::Always,
      ..Default::default()
    })
    .scrollbar_hovered(move |style| style.with_thumb_color(thumb_color))
    .size(100.0, 100.0);

  rt.set_root(node);
  let rect = rt.find_element(|_| true).unwrap().bounds();
  rt.mouse_move(rect.x + rect.width - 4.0, rect.y + 10.0);

  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);

  assert!(quads.iter().any(|quad| match &quad.content {
    QuadContent::Rect { color } => *color == thumb_color,
    _ => false,
  }));
}

#[test]
fn scrollbar_auto_does_not_render_when_content_fits() {
  let mut rt = rt();
  let thumb_color = Color::from_hex("#3b82f6");

  let node = lurq::components::ScrollVertical::new(lurq::components::Spacer::new().height(80.0))
    .scrollbar(ScrollBarStyle {
      visible: ScrollBarVisibility::Auto,
      thumb_color,
      ..Default::default()
    })
    .size(100.0, 100.0);

  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);

  assert!(!quads.iter().any(|quad| match &quad.content {
    QuadContent::Rect { color } => *color == thumb_color,
    _ => false,
  }));
}

#[test]
fn scrollbar_renders_when_styled_before_width_and_fill() {
  let mut rt = rt();
  let thumb_color = Color::from_hex("#3b82f6");

  let node = lurq::components::ScrollVertical::new(lurq::components::Spacer::new().height(300.0).fill("#162032"))
    .scrollbar(ScrollBarStyle {
      visible: ScrollBarVisibility::Always,
      width: 6.0,
      thumb_color,
      thumb_radius: 4.0,
      ..ScrollBarStyle::default()
    })
    .scrollbar_hovered(|style| style.with_thumb_color(Color::from_hex("#06b6d4")))
    .width(200.0)
    .fill("#162032");

  rt.set_root(node);
  let result = rt.pass_layout(Constraints::tight(Size::new(200.0, 100.0))).unwrap();
  let quads = rt.resolve_quads(&result);

  assert!(quads.iter().any(|quad| match &quad.content {
    QuadContent::Rect { color } => *color == thumb_color && quad.width == 6.0,
    _ => false,
  }));
}
