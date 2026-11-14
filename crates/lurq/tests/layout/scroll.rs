use lurq::{
  app::{App, Tree, events::ScrollEvent},
  components::{Column, Row, ScrollVertical, Spacer},
  layout::{
    Constraints, Size,
    layout_kind::{FrameConstraints, ScrollState},
    quad::QuadContent,
    scrollbar::{ScrollBarPlacement, ScrollBarStyle, ScrollBarVisibility},
  },
  node::{Element, color::Color, dimension::Dimension},
};

use super::PassLayoutExt;
use crate::support::{TestSurface, run_pass};

fn rt() -> Tree {
  Tree::new()
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
  let column_child = &result.children[0].result;
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
  assert_eq!(result.children[0].offset.y, 0.0);
}

#[test]
fn scroll_prevent_default_blocks_auto_scroll() {
  let mut rt = rt();
  let node = lurq::components::ScrollVertical::new(lurq::components::Spacer::new().height(500.0))
    .size(100.0, 100.0)
    .on_scroll(|event: ScrollEvent| event.prevent_default());

  rt.set_root(node);
  run_pass(&mut rt);
  rt.scroll(50.0, 50.0, 0.0, 60.0, lurq::app::events::ScrollPhase::Scroll);

  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.children[0].offset.y, 0.0);
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
  let row_child = &result.children[0].result;
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
  assert_eq!(scroll_child.size.width, 800.0);
  assert_eq!(scroll_child.size.height, 600.0);
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
fn flexed_scroll_with_zero_height_frame_uses_parent_flex_size() {
  let mut rt = rt();
  let scroll_state = ScrollState::new();
  let scroll = ScrollVertical::new(Spacer::new().height(400.0))
    .height(0.0)
    .flex(1.0)
    .with_scroll_state(scroll_state.clone())
    .scrollbar(ScrollBarStyle {
      placement: ScrollBarPlacement::Reserved,
      ..Default::default()
    });
  let node = Column::new()
    .width(100.0)
    .height(200.0)
    .child(scroll)
    .child(Spacer::new().height(64.0));

  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();

  assert_eq!(result.children[0].result.size.height, 136.0);
  assert_eq!(scroll_state.viewport_height(), 136.0);
  assert_eq!(scroll_state.content_height(), 400.0);
  assert!(scroll_state.content_height() > scroll_state.viewport_height());
}

#[test]
#[cfg_attr(
  target_os = "ios",
  ignore = "a desktop's scrollbar; a phone's default is an indicator that fades"
)]
fn flexed_scroll_with_zero_height_frame_consumes_wheel_scroll() {
  let mut rt = rt();
  let scroll_state = ScrollState::new();
  let scroll = ScrollVertical::new(Spacer::new().height(400.0))
    .height(0.0)
    .flex(1.0)
    .with_scroll_state(scroll_state.clone())
    .scrollbar(ScrollBarStyle {
      placement: ScrollBarPlacement::Reserved,
      ..Default::default()
    });
  let node = Column::new()
    .width(100.0)
    .height(200.0)
    .child(scroll)
    .child(Spacer::new().height(64.0));

  rt.set_root(node);
  run_pass(&mut rt);
  rt.scroll(10.0, 10.0, 0.0, -80.0, lurq::app::events::ScrollPhase::Scroll);

  assert!(scroll_state.scroll_y() > 0.0);
}

#[test]
#[cfg_attr(
  target_os = "ios",
  ignore = "a desktop's scrollbar; a phone's default is an indicator that fades"
)]
fn flexed_scroll_with_zero_height_frame_keeps_scroll_metrics_after_cached_pass() {
  let mut rt = rt();
  let scroll_state = ScrollState::new();
  let scroll = ScrollVertical::new(Spacer::new().height(400.0))
    .height(0.0)
    .flex(1.0)
    .with_scroll_state(scroll_state.clone())
    .scrollbar(ScrollBarStyle {
      placement: ScrollBarPlacement::Reserved,
      ..Default::default()
    });
  let node = Column::new()
    .width(100.0)
    .height(200.0)
    .child(scroll)
    .child(Spacer::new().height(64.0));

  rt.set_root(node);
  run_pass(&mut rt);
  run_pass(&mut rt);

  assert_eq!(scroll_state.viewport_height(), 136.0);
  assert_eq!(scroll_state.content_height(), 400.0);
  rt.scroll(10.0, 10.0, 0.0, -80.0, lurq::app::events::ScrollPhase::Scroll);
  assert!(scroll_state.scroll_y() > 0.0);
}

#[test]
#[cfg_attr(
  target_os = "ios",
  ignore = "a desktop's scrollbar; a phone's default is an indicator that fades"
)]
fn flex_shrunk_scroll_state_uses_final_layout_size() {
  let mut rt = rt();
  let scroll_state = ScrollState::new();
  let scroll = ScrollVertical::new(Spacer::new().width(689.0).height(1200.0))
    .width(Dimension::Pct(100.0))
    .height(464.0)
    .flex_full(0.0, 1.0, Some(701.0))
    .with_scroll_state(scroll_state.clone())
    .scrollbar(ScrollBarStyle {
      placement: ScrollBarPlacement::Reserved,
      ..Default::default()
    });
  let node = Row::new()
    .width(733.0)
    .height(464.0)
    .child(Spacer::new().width(701.0).height(1.0))
    .child(scroll);

  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(800.0, 600.0))).unwrap();

  assert_eq!(result.children[1].result.size.width, 32.0);
  assert_eq!(scroll_state.viewport_width(), 20.0);
  assert_eq!(scroll_state.viewport_height(), 464.0);
  assert!(scroll_state.content_height() > scroll_state.viewport_height());
}

#[test]
fn scroll_stays_at_bottom_when_viewport_shrinks() {
  let mut rt = rt();
  let scroll_state = ScrollState::new();
  let node = |height| {
    ScrollVertical::new(Spacer::new().height(1000.0))
      .width(100.0)
      .height(height)
      .with_scroll_state(scroll_state.clone())
  };

  rt.set_root(node(200.0));
  run_pass(&mut rt);
  scroll_state.scroll_to_bottom_pending();
  run_pass(&mut rt);
  assert_eq!(scroll_state.scroll_y(), 800.0);

  rt.set_root(node(100.0));
  run_pass(&mut rt);

  assert_eq!(scroll_state.scroll_y(), 900.0);
}

#[test]
fn scroll_resize_preserves_detached_offset() {
  let mut rt = rt();
  let scroll_state = ScrollState::new();
  let node = |height| {
    ScrollVertical::new(Spacer::new().height(1000.0))
      .width(100.0)
      .height(height)
      .with_scroll_state(scroll_state.clone())
  };

  rt.set_root(node(200.0));
  run_pass(&mut rt);
  scroll_state.set_scroll(0.0, 500.0);

  rt.set_root(node(100.0));
  run_pass(&mut rt);

  assert_eq!(scroll_state.scroll_y(), 500.0);
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

/// The colors of the thumbs painted for a scroll container of 100 whose content is 300 high.
fn thumb_colors(app: &mut App) -> Vec<Color> {
  // The thumb is the theme's lane less its inset on each side.
  let style = app.theme().scrollbar().clone();
  let thumb = style.width - 2.0 * style.thumb_inset;
  let mut rt = rt();
  rt.set_root(lurq::components::ScrollVertical::new(lurq::components::Spacer::new().height(300.0)).size(100.0, 100.0));
  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  rt.pass(app, &TestSurface);
  rt.resolve_quads(rt.last_layout().unwrap())
    .iter()
    .filter_map(|quad| match &quad.content {
      QuadContent::Rect { color, .. } if quad.width == thumb => Some(*color),
      _ => None,
    })
    .collect()
}

#[test]
fn a_scrollbar_that_sets_no_color_is_in_the_ring_color_of_the_theme() {
  use lurq::app::theme::PaletteColor;

  // The default bar is drawn in the palette's ring, which shows on a light theme and on a dark one: a fixed
  // dark thumb was lost on a dark background.
  let mut app = App::new();
  let ring = app.theme().palette_color(PaletteColor::Ring);
  assert_eq!(thumb_colors(&mut app), vec![ring]);
  app.theme().set_appearance(lurq::app::window::Appearance::Dark);
  let ring = app.theme().palette_color(PaletteColor::Ring);
  assert_eq!(thumb_colors(&mut app), vec![ring]);
  assert!(
    ring.a() == 255,
    "a color of its own, not a veil that needs a light background"
  );

  // It follows the palette.
  let violet = Color::from_hex("#8b5cf6");
  app.theme().set_palette_color(PaletteColor::Ring, violet);
  assert_eq!(thumb_colors(&mut app), vec![violet]);

  // A style with a color of its own keeps it.
  let teal = Color::from_hex("#14b8a6");
  app
    .theme()
    .set_scrollbar(ScrollBarStyle::default().with_thumb_color(teal));
  assert_eq!(thumb_colors(&mut app), vec![teal]);
}

#[test]
fn scrollbar_uses_theme_style_by_default() {
  let mut app = App::new();
  let thumb_color = Color::from_hex("#8b5cf6");
  app.theme().set_scrollbar(ScrollBarStyle {
    visible: ScrollBarVisibility::Always,
    width: 6.0,
    thumb_color,
    ..Default::default()
  });

  let mut rt = rt();
  rt.set_root(lurq::components::ScrollVertical::new(lurq::components::Spacer::new().height(300.0)).size(100.0, 100.0));
  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  rt.pass(&mut app, &TestSurface);

  let quads = rt.resolve_quads(rt.last_layout().unwrap());
  assert!(quads.iter().any(|quad| match &quad.content {
    QuadContent::Rect { color, .. } => *color == thumb_color && quad.width == 6.0,
    _ => false,
  }));
}

#[test]
fn scrollbar_node_style_overrides_theme_style() {
  let mut app = App::new();
  let theme_thumb_color = Color::from_hex("#8b5cf6");
  let node_thumb_color = Color::from_hex("#14b8a6");
  app.theme().set_scrollbar(ScrollBarStyle {
    visible: ScrollBarVisibility::Always,
    width: 6.0,
    thumb_color: theme_thumb_color,
    ..Default::default()
  });

  let mut rt = rt();
  let node = lurq::components::ScrollVertical::new(lurq::components::Spacer::new().height(300.0))
    .scrollbar(ScrollBarStyle {
      visible: ScrollBarVisibility::Always,
      width: 9.0,
      thumb_color: node_thumb_color,
      ..Default::default()
    })
    .size(100.0, 100.0);
  rt.set_root(node);
  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  rt.pass(&mut app, &TestSurface);

  let quads = rt.resolve_quads(rt.last_layout().unwrap());
  assert!(quads.iter().any(|quad| match &quad.content {
    QuadContent::Rect { color, .. } => *color == node_thumb_color && quad.width == 9.0,
    _ => false,
  }));
  assert!(!quads.iter().any(|quad| match &quad.content {
    QuadContent::Rect { color, .. } => *color == theme_thumb_color && quad.width == 6.0,
    _ => false,
  }));
}

#[test]
#[cfg_attr(
  target_os = "ios",
  ignore = "a desktop's scrollbar; a phone's default is an indicator that fades"
)]
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
  run_pass(&mut rt);
  rt.mouse_move(100.0 - 4.0, 10.0);

  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);

  assert!(quads.iter().any(|quad| match &quad.content {
    QuadContent::Rect { color, .. } => *color == thumb_color,
    _ => false,
  }));
}

#[test]
#[cfg_attr(
  target_os = "ios",
  ignore = "a desktop's scrollbar; a phone's default is an indicator that fades"
)]
fn horizontal_scrollbar_hovered_overrides_scrollbar_style() {
  let mut rt = rt();
  let thumb_color = Color::from_hex("#ef4444");

  let node = lurq::components::ScrollHorizontal::new(lurq::components::Spacer::new().width(300.0))
    .scrollbar(ScrollBarStyle {
      visible: ScrollBarVisibility::Always,
      ..Default::default()
    })
    .scrollbar_hovered(move |style| style.with_thumb_color(thumb_color))
    .size(100.0, 100.0);

  rt.set_root(node);
  run_pass(&mut rt);
  rt.mouse_move(10.0, 100.0 - 4.0);

  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);

  assert!(quads.iter().any(|quad| match &quad.content {
    QuadContent::Rect { color, .. } => *color == thumb_color,
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
    QuadContent::Rect { color, .. } => *color == thumb_color,
    _ => false,
  }));
}

#[test]
fn scrollbar_renders_when_styled_before_width_and_fill() {
  let mut rt = rt();
  let thumb_color = Color::from_hex("#3b82f6");

  let node = lurq::components::ScrollVertical::new(lurq::components::Spacer::new().height(300.0).background("#162032"))
    .scrollbar(ScrollBarStyle {
      visible: ScrollBarVisibility::Always,
      width: 6.0,
      thumb_color,
      thumb_radius: 4.0,
      ..ScrollBarStyle::default()
    })
    .scrollbar_hovered(|style| style.with_thumb_color(Color::from_hex("#06b6d4")))
    .width(200.0)
    .background("#162032");

  rt.set_root(node);
  let result = rt.pass_layout(Constraints::tight(Size::new(200.0, 100.0))).unwrap();
  let quads = rt.resolve_quads(&result);

  assert!(quads.iter().any(|quad| match &quad.content {
    QuadContent::Rect { color, .. } => *color == thumb_color && quad.width == 6.0,
    _ => false,
  }));
}

#[test]
fn scroll_culling_culls_offscreen_child_quads_and_preserves_content_extent() {
  let mut rt = rt();
  let node = lurq::components::ScrollVertical::new(colored_scroll_rows())
    .scrollbar(ScrollBarStyle::hidden())
    .size(100.0, 100.0);

  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);

  assert_eq!(result.children[0].result.size.height, 500.0);
  assert_eq!(
    scroll_row_colors(&quads),
    vec![scroll_row_color(0), scroll_row_color(1)]
  );
}

#[test]
fn culling_false_keeps_offscreen_child_quads() {
  let mut rt = rt();
  let node = lurq::components::ScrollVertical::new(colored_scroll_rows())
    .culling(false)
    .scrollbar(ScrollBarStyle::hidden())
    .size(100.0, 100.0);

  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);

  assert_eq!(result.children[0].result.size.height, 500.0);
  assert_eq!(scroll_row_colors(&quads).len(), 10);
}

#[test]
fn scroll_culling_respects_scroll_offset() {
  let mut rt = rt();
  let state = lurq::layout::layout_kind::ScrollState::new();
  state.set_scroll_pending(0.0, 150.0);
  let node = lurq::components::ScrollVertical::new(colored_scroll_rows())
    .with_scroll_state(state.clone())
    .culling(true)
    .scrollbar(ScrollBarStyle::hidden())
    .size(100.0, 100.0);

  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);

  assert_eq!(state.scroll_y(), 150.0);
  assert_eq!(result.children[0].result.size.height, 500.0);
  assert_eq!(
    scroll_row_colors(&quads),
    vec![scroll_row_color(3), scroll_row_color(4)]
  );
}

fn colored_scroll_rows() -> lurq::components::Column {
  lurq::components::Column::new()
    .spacing(0.0)
    .with_children((0..10).map(|index| lurq::components::Rect::new(100.0, 50.0).background(scroll_row_color(index))))
}

fn scroll_row_color(index: u8) -> Color {
  Color::new(index, 32, 64, 255)
}

fn scroll_row_colors(quads: &[lurq::layout::quad::Quad]) -> Vec<Color> {
  quads
    .iter()
    .filter_map(|quad| match &quad.content {
      QuadContent::Rect { color, .. } if color.g() == 32 && color.b() == 64 => Some(*color),
      _ => None,
    })
    .collect()
}

/// Where the red box in a scroll of 100 is drawn, scrolled by `by` at `scale` device pixels to a logical one.
fn scrolled_box_y(scale: f32, by: f32) -> f32 {
  let red = Color::from_hex("#ff0000");
  let state = ScrollState::new();
  let mut rt = rt();
  rt.set_scale_factor(scale);
  rt.set_root(
    ScrollVertical::new(
      Column::new()
        .height(600.0)
        .child(Spacer::new().height(40.0))
        .child(Row::new().size(50.0, 20.0).background(red)),
    )
    .size(100.0, 100.0)
    .with_scroll_state(state.clone()),
  );
  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  let mut app = App::new();
  rt.pass(&mut app, &TestSurface);
  state.scroll_by(0.0, by);
  rt.pass(&mut app, &TestSurface);
  rt.resolve_quads(rt.last_layout().unwrap())
    .iter()
    .find(|quad| matches!(&quad.content, QuadContent::Rect { color, .. } if *color == red))
    .map(|quad| quad.y)
    .expect("the box")
}

#[test]
fn what_a_scroll_holds_is_drawn_on_a_whole_device_pixel() {
  // At a whole scale a logical pixel is a device pixel: the content is where the offset says.
  assert_eq!(scrolled_box_y(1.0, 17.0), 23.0);
  assert_eq!(scrolled_box_y(2.0, 17.5), 22.5);
  // At 150% an offset of 17 is 25.5 device pixels. A text is painted on a whole device pixel and a box where it is:
  // half a pixel moved every label a pixel against its box, and back at the next step of the wheel. The content is
  // drawn at 26 device pixels, and the scroll keeps the 17 it was given.
  let y = scrolled_box_y(1.5, 17.0);
  assert!((y * 1.5 - (y * 1.5).round()).abs() < 1e-3, "{y}");
  assert!((y - (40.0 - 17.0)).abs() <= 0.5 / 1.5 + 1e-3, "{y}");
  // Whatever the offset, the box and what is beside it keep their distance in device pixels.
  for by in [1.0, 3.3, 16.7, 51.0] {
    let y = scrolled_box_y(1.5, by) * 1.5;
    assert!((y - y.round()).abs() < 1e-3, "{by}: {y}");
  }
}

/// The thumbs a container of 100 by 100 whose content is 300 by 300 paints, through `app`'s theme: the vertical one
/// (narrower than it is long) and the horizontal one.
fn thumbs(
  tree: &mut Tree,
  app: &mut App,
) -> (
  Option<crate::support::RectSnapshot>,
  Option<crate::support::RectSnapshot>,
) {
  let snapshot = crate::support::render_pass_with_app(tree, app);
  let style = app.theme().scrollbar().clone();
  let thickness = style.width - 2.0 * style.thumb_inset;
  let find = |vertical: bool| {
    snapshot.rects.iter().copied().find(|rect| {
      let (across, along) = if vertical {
        (rect.width, rect.height)
      } else {
        (rect.height, rect.width)
      };
      (across - thickness).abs() < 0.01 && along > thickness * 2.0
    })
  };
  (find(true), find(false))
}

fn both_ways() -> Tree {
  let mut tree = Tree::new();
  tree
    .set_root(lurq::components::ScrollBoth::new(lurq::components::Spacer::new().size(300.0, 300.0)).size(100.0, 100.0));
  tree
}

#[test]
fn the_design_themes_bar_is_a_lane_of_8_with_a_round_thumb_of_6_and_the_legacy_one_fills_its_lane() {
  use lurq::app::theme::{PaletteColor, ThemeDefinition};

  let mut app = App::new();
  let mut tree = both_ways();
  let (vertical, _) = thumbs(&mut tree, &mut app);
  let vertical = vertical.expect("the design's thumb");
  // The lane is 8, 2 from the edge: 90 to 98. The thumb is 1 inside it.
  assert!((vertical.x - 91.0).abs() < 0.01, "{}", vertical.x);
  assert_eq!(vertical.width, 6.0);
  assert_eq!(vertical.radii, [3.0; 4], "round");
  assert_eq!(vertical.color, app.theme().palette_color(PaletteColor::Ring));

  let mut legacy = App::new();
  legacy.theme().apply(ThemeDefinition::legacy());
  let mut tree = both_ways();
  let (vertical, _) = thumbs(&mut tree, &mut legacy);
  let vertical = vertical.expect("the engine's thumb");
  assert_eq!((vertical.x, vertical.width), (90.0, 8.0), "as wide as its lane");
}

#[test]
fn the_thumb_under_the_pointer_and_dragged_is_in_its_state_and_the_other_bar_is_not() {
  use lurq::app::{events::MouseButton, theme::PaletteColor};

  let mut app = App::new();
  let (ring, hovered) = (
    app.theme().palette_color(PaletteColor::Ring),
    app.theme().palette_color(PaletteColor::MutedForeground),
  );
  let mut tree = both_ways();
  let (vertical, horizontal) = thumbs(&mut tree, &mut app);
  let (vertical, horizontal) = (vertical.unwrap(), horizontal.unwrap());
  assert_eq!((vertical.color, horizontal.color), (ring, ring), "at rest");

  let (x, y) = (vertical.x + vertical.width / 2.0, vertical.y + vertical.height / 2.0);
  tree.mouse_move(x, y);
  let (on, other) = thumbs(&mut tree, &mut app);
  assert_eq!(on.unwrap().color, hovered, "under the pointer");
  assert_eq!(other.unwrap().color, ring, "the bar across is not under it");

  // Held and dragged, it stays in its state, and the drag goes the measure of the thumb's travel.
  tree.mouse_down(x, y, MouseButton::Left);
  tree.mouse_move(x, y + 20.0);
  let (dragged, _) = thumbs(&mut tree, &mut app);
  assert_eq!(dragged.unwrap().color, hovered, "dragged");
  tree.mouse_up(x, y + 20.0, MouseButton::Left);
  tree.mouse_move(10.0, 10.0);
  let (left, _) = thumbs(&mut tree, &mut app);
  assert_eq!(left.unwrap().color, ring, "let go, and the pointer gone");
}

#[test]
fn a_style_says_its_own_pressed_thumb() {
  use lurq::{
    app::{events::MouseButton, theme::PaletteColor},
    layout::scrollbar::ScrollBarStyle,
    node::BackgroundColor,
  };

  let mut app = App::new();
  // Read, then written: the theme's read guard is gone before it is set.
  let style = app.theme().scrollbar().clone();
  app.theme().set_scrollbar(ScrollBarStyle {
    thumb_pressed_color: Some(BackgroundColor::Palette(PaletteColor::Foreground)),
    ..style
  });
  let foreground = app.theme().palette_color(PaletteColor::Foreground);
  let mut tree = both_ways();
  let (vertical, _) = thumbs(&mut tree, &mut app);
  let vertical = vertical.unwrap();
  let (x, y) = (vertical.x + vertical.width / 2.0, vertical.y + vertical.height / 2.0);
  tree.mouse_move(x, y);
  tree.mouse_down(x, y, MouseButton::Left);
  let (pressed, _) = thumbs(&mut tree, &mut app);
  assert_eq!(pressed.unwrap().color, foreground);
}

#[test]
fn where_two_bars_meet_each_track_ends_before_the_others_lane() {
  use lurq::layout::layout_kind::{ScrollAxis, ScrollState};

  let state = ScrollState::new();
  let mut tree = Tree::new();
  tree.set_root(
    lurq::components::ScrollBoth::new(lurq::components::Spacer::new().size(300.0, 300.0))
      .with_scroll_state(state.clone())
      .size(100.0, 100.0),
  );
  let mut app = App::new();
  crate::support::render_pass_with_app(&mut tree, &mut app);
  // At the end of both ways, each thumb ends its own inset before the other bar's lane, which is 8 and 2 from
  // the edge: 100 less 2, less 10, less 1.
  state.set_scroll(200.0, 200.0);
  crate::support::render_pass_with_app(&mut tree, &mut app);
  let style = state.style();
  let corner = style.width + style.resolved_edge_inset();
  let end = 100.0 - style.resolved_end_inset() - corner - style.thumb_inset;
  let (_, y, _, height) = state.thumb_rect_for_axis(ScrollAxis::Vertical, &style).unwrap();
  let (x, _, width, _) = state.thumb_rect_for_axis(ScrollAxis::Horizontal, &style).unwrap();
  assert!((y + height - end).abs() < 0.01, "down: {}", y + height);
  assert!((x + width - end).abs() < 0.01, "across: {}", x + width);

  // A container that scrolls down only keeps its whole track, however wide its content.
  let state = ScrollState::new();
  let mut tree = Tree::new();
  tree.set_root(
    lurq::components::ScrollVertical::new(lurq::components::Spacer::new().size(300.0, 300.0))
      .with_scroll_state(state.clone())
      .size(100.0, 100.0),
  );
  crate::support::render_pass_with_app(&mut tree, &mut app);
  state.set_scroll(0.0, 200.0);
  crate::support::render_pass_with_app(&mut tree, &mut app);
  let (_, y, _, height) = state.thumb_rect_for_axis(ScrollAxis::Vertical, &style).unwrap();
  let whole = 100.0 - style.resolved_end_inset() - style.thumb_inset;
  assert!((y + height - whole).abs() < 0.01, "{}", y + height);
}

/// A horizontal scroll that fills its width lays out a row of growing items at least as wide as its viewport, and
/// wider, to scroll, once their least widths are more than it.
#[test]
fn a_horizontal_scroll_that_fills_its_width_lets_its_items_share_the_viewport() {
  let row_of = |count: usize| {
    Row::new().with_children((0..count).map(|_| {
      Column::new()
        .flex_full(1.0, 0.0, None)
        .width(Dimension::Px(100.0))
        .height(Dimension::Px(10.0))
    }))
  };
  for (count, width, fill) in [(3, 400.0, true), (6, 600.0, true), (3, 300.0, false)] {
    let mut rt = rt();
    rt.set_root(
      lurq::components::ScrollHorizontal::new(Row::new().child(row_of(count).flex(1.0)))
        .fill_width(fill)
        .size(400.0, 40.0),
    );
    let result = rt.pass_layout(Constraints::loose(Size::new(800.0, 400.0))).unwrap();
    let content = &result.children[0].result;
    assert_eq!(content.size.width, width, "{count} items, filling: {fill}");
    // The row in it grows to the viewport, and its items share it: 3 of 133, 6 of their least 100, or their least
    // where the scroll does not fill.
    let row = &content.children[0].result;
    assert_eq!(row.size.width, width);
    let item = &row.children[0].result;
    assert!(
      (item.size.width - width / count as f32).abs() < 0.01,
      "{}",
      item.size.width
    );
  }
}
