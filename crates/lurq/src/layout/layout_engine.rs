use std::cell::Cell;

use crate::{
  app::glyph_engine::GlyphEngine,
  layout::{
    Alignment, Constraints, Offset, Size, StackAlignment,
    layout_kind::{
      FlexParams, FlexWrap, FrameConstraints, Justify, LayoutKind, Overflow, ScrollAxis, ScrollDirection, ScrollState,
    },
    layout_result::{ChildLayout, LayoutResult},
    quad::{ClipRect, Quad, QuadContent},
    scrollbar::{ScrollBarPlacement, ScrollBarStyle, ScrollBarVisibility},
    text_style::TextStyle,
  },
  node::{
    CheckboxStyle, TextTransformMode,
    border::{BorderRadius, Borders},
    color::Color,
    dimension::Dimension,
    node::Node,
    node_kind::{NodeKind, SliderPartRect},
    padding::Padding,
    slider_style::SliderPartStyle,
    transform::Transform2D,
  },
};

const DEFAULT_CHECKBOX_WIDTH: f32 = 18.0;
const DEFAULT_CHECKBOX_HEIGHT: f32 = 18.0;
const DEFAULT_SLIDER_WIDTH: f32 = 120.0;
const DEFAULT_SLIDER_HEIGHT: f32 = 20.0;
const DEFAULT_SLIDER_THUMB_MIN_SIZE: f32 = 12.0;
const DEFAULT_TEXT_INPUT_WIDTH: f32 = 120.0;
#[cfg(any(feature = "image", feature = "svg"))]
const DEFAULT_RESOURCE_WIDTH: f32 = 0.0;
#[cfg(any(feature = "image", feature = "svg"))]
const DEFAULT_RESOURCE_HEIGHT: f32 = 0.0;
const DEFAULT_QUAD_OPACITY: f32 = 1.0;

const DEFAULT_CONTROL_SURFACE_COLOR: Color = Color::new(255, 255, 255, 255);
const DEFAULT_TRANSPARENT_COLOR: Color = Color::new(0, 0, 0, 0);
const DEFAULT_CHECKBOX_CHECKED_COLOR: Color = Color::new(34, 197, 94, 255);
const DEFAULT_SLIDER_TRACK_COLOR: Color = Color::new(203, 213, 225, 255);
const DEFAULT_SLIDER_THUMB_COLOR: Color = Color::new(71, 85, 105, 255);
const DEFAULT_TEXT_SELECTION_COLOR: Color = Color::new(191, 219, 254, 255);
const DEFAULT_CARET_COLOR: Color = Color::new(15, 23, 42, 255);

fn text_input_display_style<'a>(
  state: &crate::node::node_kind::TextInputState,
  style: &'a TextStyle,
  placeholder_style: Option<&'a TextStyle>,
) -> &'a TextStyle {
  if state.is_showing_placeholder() {
    placeholder_style.unwrap_or(style)
  } else {
    style
  }
}

pub(crate) struct LayoutEngine {
  last_recalculated: Cell<bool>,
}

#[cfg(feature = "image")]
struct BackgroundImagePlacement {
  x: f32,
  y: f32,
  width: f32,
  height: f32,
  uv_min: [f32; 2],
  uv_max: [f32; 2],
}

#[cfg(feature = "image")]
fn background_image_placement(
  size_mode: crate::node::BackgroundSize,
  box_width: f32,
  box_height: f32,
  image_width: f32,
  image_height: f32,
) -> BackgroundImagePlacement {
  let full = BackgroundImagePlacement {
    x: 0.0,
    y: 0.0,
    width: box_width,
    height: box_height,
    uv_min: [0.0, 0.0],
    uv_max: [1.0, 1.0],
  };

  if box_width <= 0.0 || box_height <= 0.0 || image_width <= 0.0 || image_height <= 0.0 {
    return full;
  }

  match size_mode {
    crate::node::BackgroundSize::Stretch => full,
    crate::node::BackgroundSize::Contain => {
      let scale = (box_width / image_width).min(box_height / image_height);
      let width = image_width * scale;
      let height = image_height * scale;
      BackgroundImagePlacement {
        x: (box_width - width) * 0.5,
        y: (box_height - height) * 0.5,
        width,
        height,
        uv_min: [0.0, 0.0],
        uv_max: [1.0, 1.0],
      }
    }
    crate::node::BackgroundSize::Cover => {
      let box_aspect = box_width / box_height;
      let image_aspect = image_width / image_height;
      let mut uv_min = [0.0, 0.0];
      let mut uv_max = [1.0, 1.0];

      if image_aspect > box_aspect {
        let visible_u = box_aspect / image_aspect;
        uv_min[0] = (1.0 - visible_u) * 0.5;
        uv_max[0] = 1.0 - uv_min[0];
      } else if image_aspect < box_aspect {
        let visible_v = image_aspect / box_aspect;
        uv_min[1] = (1.0 - visible_v) * 0.5;
        uv_max[1] = 1.0 - uv_min[1];
      }

      BackgroundImagePlacement { uv_min, uv_max, ..full }
    }
  }
}

fn push_slider_part_quads(
  quads: &mut Vec<Quad>,
  rect: SliderPartRect,
  style: &SliderPartStyle,
  color: Color,
  border_radius: Option<BorderRadius>,
  border: Option<Borders>,
  opacity: f32,
  transform: Transform2D,
  clip: ClipRect,
) {
  #[cfg(not(feature = "image"))]
  let _ = style;
  #[cfg(feature = "image")]
  let has_image = style.background_image.is_some();
  #[cfg(not(feature = "image"))]
  let has_image = false;

  let (rect_x, rect_y, rect_transform, rect_transform_origin) = transformed_quad_frame(rect.x, rect.y, transform);
  quads.push(Quad {
    x: rect_x,
    y: rect_y,
    width: rect.width,
    height: rect.height,
    opacity,
    transform: rect_transform,
    transform_origin: rect_transform_origin,
    content: QuadContent::Rect { color },
    border_radius,
    border: if has_image { None } else { border },
    clip,
  });

  #[cfg(feature = "image")]
  if let Some(ref bg_image) = style.background_image {
    let placement = background_image_placement(
      style.background_size,
      rect.width,
      rect.height,
      bg_image.width() as f32,
      bg_image.height() as f32,
    );
    let image_x = rect.x + placement.x;
    let image_y = rect.y + placement.y;
    let (image_x, image_y, image_transform, image_transform_origin) =
      transformed_quad_frame(image_x, image_y, transform);
    quads.push(Quad {
      x: image_x,
      y: image_y,
      width: placement.width,
      height: placement.height,
      opacity,
      transform: image_transform,
      transform_origin: image_transform_origin,
      content: QuadContent::Image {
        data: bg_image.clone(),
        uv_min: placement.uv_min,
        uv_max: placement.uv_max,
      },
      border_radius,
      border: None,
      clip,
    });
  }

  if has_image && border.is_some() {
    let (rect_x, rect_y, rect_transform, rect_transform_origin) = transformed_quad_frame(rect.x, rect.y, transform);
    quads.push(Quad {
      x: rect_x,
      y: rect_y,
      width: rect.width,
      height: rect.height,
      opacity,
      transform: rect_transform,
      transform_origin: rect_transform_origin,
      content: QuadContent::Rect {
        color: DEFAULT_TRANSPARENT_COLOR,
      },
      border_radius,
      border,
      clip,
    });
  }
}

fn push_checkbox_quads(
  quads: &mut Vec<Quad>,
  rect: SliderPartRect,
  style: &CheckboxStyle,
  color: Color,
  border_radius: Option<BorderRadius>,
  border: Option<Borders>,
  checked: bool,
  opacity: f32,
  transform: Transform2D,
  clip: ClipRect,
) {
  #[cfg(not(feature = "image"))]
  let _ = (style, checked);

  let (rect_x, rect_y, rect_transform, rect_transform_origin) = transformed_quad_frame(rect.x, rect.y, transform);
  quads.push(Quad {
    x: rect_x,
    y: rect_y,
    width: rect.width,
    height: rect.height,
    opacity,
    transform: rect_transform,
    transform_origin: rect_transform_origin,
    content: QuadContent::Rect { color },
    border_radius,
    border,
    clip,
  });

  #[cfg(feature = "image")]
  if checked && let Some(ref indicator_image) = style.indicator_image {
    let indicator_width = style
      .indicator_width
      .unwrap_or(rect.width * 0.65)
      .min(rect.width)
      .max(0.0);
    let indicator_height = style
      .indicator_height
      .unwrap_or(rect.height * 0.65)
      .min(rect.height)
      .max(0.0);
    let indicator_x = rect.x + (rect.width - indicator_width) * 0.5;
    let indicator_y = rect.y + (rect.height - indicator_height) * 0.5;
    let placement = background_image_placement(
      style.indicator_size,
      indicator_width,
      indicator_height,
      indicator_image.width() as f32,
      indicator_image.height() as f32,
    );
    let image_x = indicator_x + placement.x;
    let image_y = indicator_y + placement.y;
    let (image_x, image_y, image_transform, image_transform_origin) =
      transformed_quad_frame(image_x, image_y, transform);
    quads.push(Quad {
      x: image_x,
      y: image_y,
      width: placement.width,
      height: placement.height,
      opacity,
      transform: image_transform,
      transform_origin: image_transform_origin,
      content: QuadContent::Image {
        data: indicator_image.clone(),
        uv_min: placement.uv_min,
        uv_max: placement.uv_max,
      },
      border_radius: None,
      border: None,
      clip,
    });
  }
}

fn quad_transform(transform: Transform2D) -> Transform2D {
  transform.linear_part()
}

fn transformed_quad_frame(x: f32, y: f32, transform: Transform2D) -> (f32, f32, Transform2D, Option<[f32; 2]>) {
  if transform.is_identity() {
    return (x, y, Transform2D::IDENTITY, None);
  }

  let (x, y) = transform.transform_point(x, y);
  let linear = quad_transform(transform);
  let origin = if linear.is_identity() { None } else { Some([0.0, 0.0]) };
  (x, y, linear, origin)
}

impl LayoutEngine {
  pub(crate) fn new() -> Self {
    Self {
      last_recalculated: Cell::new(false),
    }
  }

  pub(crate) fn compute(&self, glyph_engine: &mut GlyphEngine, node: &Node, constraints: Constraints) -> LayoutResult {
    self.last_recalculated.set(false);
    Self::mark_layout_dirty(node);
    let result = self.layout_node(glyph_engine, node, constraints);
    node.clear_guards();
    result
  }

  pub(crate) fn last_recalculated(&self) -> bool {
    self.last_recalculated.get()
  }

  fn mark_layout_dirty(node: &Node) -> bool {
    let mut local_dirty = node.text_content.is_changed() || matches!(node.node_kind(), NodeKind::TextInput { .. });

    if let LayoutKind::ScrollModifier { state, .. } = node.layout_kind()
      && state.take_scroll_dirty()
    {
      local_dirty = true;
    }

    if node
      .element_ref
      .as_ref()
      .is_some_and(|element_ref| element_ref.take_layout_dirty())
    {
      local_dirty = true;
    }

    if node.take_style_layout_dirty() {
      local_dirty = true;
    }

    let mut child_dirty = false;
    for child in node.children() {
      child_dirty |= Self::mark_layout_dirty(child);
    }

    if local_dirty {
      node.layout_cache.mark_local_dirty();
    }
    if child_dirty {
      node.layout_cache.mark_descendant_dirty();
    }

    local_dirty || child_dirty
  }

  pub(crate) fn resolve_quads(&self, node: &Node, result: &LayoutResult) -> Vec<Quad> {
    self.resolve_quads_with_viewport(node, result, ClipRect::default())
  }

  pub(crate) fn resolve_quads_with_viewport(
    &self,
    node: &Node,
    result: &LayoutResult,
    viewport: ClipRect,
  ) -> Vec<Quad> {
    let mut quads = Vec::new();
    self.collect_quads(
      node,
      result,
      0.0,
      0.0,
      0.0,
      0.0,
      Transform2D::IDENTITY,
      viewport,
      &mut quads,
    );
    quads
  }

  fn collect_quads(
    &self,
    node: &Node,
    result: &LayoutResult,
    abs_x: f32,
    abs_y: f32,
    parent_x: f32,
    parent_y: f32,
    inherited_transform: Transform2D,
    clip: ClipRect,
    quads: &mut Vec<Quad>,
  ) {
    if let Some(ref element_ref) = node.element_ref {
      element_ref.update(
        abs_x,
        abs_y,
        abs_x - parent_x,
        abs_y - parent_y,
        result.size.width,
        result.size.height,
      );
    }

    let has_visual = node.color().is_some() || node.get_border().is_some();
    let content = match node.node_kind() {
      NodeKind::Text {
        style, transform_mode, ..
      } => QuadContent::Text {
        text: node.text_content().unwrap_or_default().to_owned(),
        style: style.clone(),
        wrap: node.text_wrap,
        transform_mode: *transform_mode,
      },
      NodeKind::TextInput {
        state,
        style,
        placeholder_style,
      } => QuadContent::Text {
        text: state.rendered_text_for_layout(),
        style: text_input_display_style(state, style, placeholder_style.as_ref()).clone(),
        wrap: state.overflow() == crate::node::node_kind::TextInputOverflow::Multiline,
        transform_mode: TextTransformMode::Bitmap,
      },
      NodeKind::Checkbox { .. } => QuadContent::None,
      #[cfg(feature = "image")]
      NodeKind::Image { data } => QuadContent::Image {
        data: data.clone(),
        uv_min: [0.0, 0.0],
        uv_max: [1.0, 1.0],
      },
      #[cfg(feature = "image")]
      NodeKind::ResourceImage { .. } => QuadContent::None,
      #[cfg(feature = "svg")]
      NodeKind::Svg { data } => QuadContent::Svg { data: data.clone() },
      #[cfg(all(feature = "svg", feature = "resources"))]
      NodeKind::ResourceSvg { .. } => QuadContent::None,
      NodeKind::Slider { .. } => QuadContent::None,
      _ if has_visual => QuadContent::Rect {
        color: node.color().unwrap_or(DEFAULT_TRANSPARENT_COLOR),
      },
      _ => QuadContent::None,
    };

    let opacity = node.opacity;
    let local_transform = node.effective_transform();
    let local_transform_origin_abs = [abs_x + result.size.width * 0.5, abs_y + result.size.height * 0.5];
    let local_affine = if local_transform.is_identity() {
      Transform2D::IDENTITY
    } else {
      local_transform.around_origin(local_transform_origin_abs)
    };
    let transform = if inherited_transform.is_identity() {
      local_affine
    } else if local_affine.is_identity() {
      inherited_transform
    } else if local_transform.is_identity() {
      inherited_transform
    } else {
      inherited_transform.then(&local_affine)
    };

    match &content {
      QuadContent::None => {}
      _ => {
        if let NodeKind::Text { state, style, .. } = node.node_kind()
          && state.selectable()
        {
          let selection_height = (style.font_size * style.line_height).min(result.size.height).max(1.0);
          let selection_clip = clip;
          for selection in state.selection_ranges(node.text_content().unwrap_or_default()) {
            let selection_x = abs_x + selection.x;
            let selection_y = abs_y + selection.y;
            let (selection_x, selection_y, selection_transform, selection_transform_origin) =
              transformed_quad_frame(selection_x, selection_y, transform);
            quads.push(Quad {
              x: selection_x,
              y: selection_y,
              width: selection.width,
              height: selection_height,
              opacity,
              transform: selection_transform,
              transform_origin: selection_transform_origin,
              content: QuadContent::Rect {
                color: DEFAULT_TEXT_SELECTION_COLOR,
              },
              border_radius: None,
              border: None,
              clip: selection_clip,
            });
          }
        }

        if let NodeKind::TextInput { state, .. } = node.node_kind()
          && state.is_focused()
        {
          let selection_height = state.caret_height().min(result.size.height).max(1.0);
          let selection_clip = intersect_clip(
            clip,
            ClipRect {
              x: abs_x,
              y: abs_y,
              width: result.size.width,
              height: result.size.height,
              active: true,
            },
          );
          for selection in state.selection_ranges() {
            let selection_x = abs_x + selection.x;
            let selection_y = abs_y + selection.y;
            let (selection_x, selection_y, selection_transform, selection_transform_origin) =
              transformed_quad_frame(selection_x, selection_y, transform);
            quads.push(Quad {
              x: selection_x,
              y: selection_y,
              width: selection.width,
              height: selection_height,
              opacity,
              transform: selection_transform,
              transform_origin: selection_transform_origin,
              content: QuadContent::Rect {
                color: DEFAULT_TEXT_SELECTION_COLOR,
              },
              border_radius: None,
              border: None,
              clip: selection_clip,
            });
          }
        }

        let (content_x, content_y, content_width, content_height, content_clip) = match node.node_kind() {
          NodeKind::TextInput { state, .. } => {
            let scroll_x = state.scroll_x();
            let scroll_y = state.scroll_y();
            if matches!(
              state.overflow(),
              crate::node::node_kind::TextInputOverflow::Scroll | crate::node::node_kind::TextInputOverflow::Multiline
            ) {
              (
                abs_x - scroll_x,
                abs_y - scroll_y,
                result.size.width + scroll_x,
                result.size.height + scroll_y,
                intersect_clip(
                  clip,
                  ClipRect {
                    x: abs_x,
                    y: abs_y,
                    width: result.size.width,
                    height: result.size.height,
                    active: true,
                  },
                ),
              )
            } else {
              (abs_x, abs_y, result.size.width, result.size.height, clip)
            }
          }
          _ => (abs_x, abs_y, result.size.width, result.size.height, clip),
        };

        let (content_x, content_y, content_transform, content_transform_origin) =
          transformed_quad_frame(content_x, content_y, transform);
        quads.push(Quad {
          x: content_x,
          y: content_y,
          width: content_width,
          height: content_height,
          opacity,
          transform: content_transform,
          transform_origin: content_transform_origin,
          content,
          border_radius: node.get_border_radius(),
          border: node.get_border(),
          clip: content_clip,
        });
      }
    }

    #[cfg(feature = "image")]
    if let Some(ref bg_image) = *node.background_image {
      let placement = background_image_placement(
        node.background_size,
        result.size.width,
        result.size.height,
        bg_image.width() as f32,
        bg_image.height() as f32,
      );
      let image_x = abs_x + placement.x;
      let image_y = abs_y + placement.y;
      let (image_x, image_y, image_transform, image_transform_origin) =
        transformed_quad_frame(image_x, image_y, transform);
      quads.push(Quad {
        x: image_x,
        y: image_y,
        width: placement.width,
        height: placement.height,
        opacity,
        transform: image_transform,
        transform_origin: image_transform_origin,
        content: QuadContent::Image {
          data: bg_image.clone(),
          uv_min: placement.uv_min,
          uv_max: placement.uv_max,
        },
        border_radius: node.get_border_radius(),
        border: None,
        clip,
      });
    }

    match node.node_kind() {
      NodeKind::TextInput { state, .. } if state.is_focused() => {
        let caret_height = state.caret_height().min(result.size.height).max(1.0);
        let caret_x = abs_x + state.caret_x();
        let caret_y = abs_y + state.caret_y();
        let (caret_x, caret_y, caret_transform, caret_transform_origin) =
          transformed_quad_frame(caret_x, caret_y, transform);
        quads.push(Quad {
          x: caret_x,
          y: caret_y,
          width: 1.0,
          height: caret_height,
          opacity,
          transform: caret_transform,
          transform_origin: caret_transform_origin,
          content: QuadContent::Rect {
            color: DEFAULT_CARET_COLOR,
          },
          border_radius: None,
          border: None,
          clip,
        });
      }
      NodeKind::Checkbox { state } => {
        let checked = state.is_checked();
        let hovered = node.is_style_hovered();
        let style = state.style(checked, hovered);
        let width = style.width.unwrap_or(result.size.width).min(result.size.width).max(0.0);
        let height = style
          .height
          .unwrap_or(result.size.height)
          .min(result.size.height)
          .max(0.0);
        let rect = SliderPartRect {
          x: abs_x + (result.size.width - width) * 0.5,
          y: abs_y + (result.size.height - height) * 0.5,
          width,
          height,
        };
        let color = if checked {
          style.color.unwrap_or(DEFAULT_CHECKBOX_CHECKED_COLOR)
        } else {
          style
            .color
            .or_else(|| node.color())
            .unwrap_or(DEFAULT_CONTROL_SURFACE_COLOR)
        };
        push_checkbox_quads(
          quads,
          rect,
          &style,
          color,
          style.border_radius.or_else(|| node.get_border_radius()),
          style.border.or_else(|| node.get_border()),
          checked,
          opacity,
          transform,
          clip,
        );
      }
      NodeKind::Slider { state } => {
        let hovered = node.is_style_hovered() || state.is_hovered();
        let track_style = state.track_style(hovered);
        let thumb_style = state.thumb_style(hovered);
        let (track_rect, thumb_rect) = state.part_rects(
          abs_x,
          abs_y,
          result.size.width,
          result.size.height,
          hovered,
          DEFAULT_SLIDER_THUMB_MIN_SIZE,
        );
        let track_color = track_style
          .color
          .or_else(|| node.color())
          .unwrap_or(DEFAULT_SLIDER_TRACK_COLOR);
        let track_radius = track_style.border_radius.or_else(|| node.get_border_radius());
        let track_border = track_style.border.or_else(|| node.get_border());
        push_slider_part_quads(
          quads,
          track_rect,
          &track_style,
          track_color,
          track_radius,
          track_border,
          opacity,
          transform,
          clip,
        );
        push_slider_part_quads(
          quads,
          thumb_rect,
          &thumb_style,
          thumb_style.color.unwrap_or(DEFAULT_SLIDER_THUMB_COLOR),
          Some(
            thumb_style
              .border_radius
              .unwrap_or_else(|| BorderRadius::all(thumb_rect.width.min(thumb_rect.height) * 0.5)),
          ),
          thumb_style.border,
          opacity,
          transform,
          clip,
        );
      }
      _ => {}
    }

    let child_clip = if let LayoutKind::ScrollModifier { state, .. } = node.layout_kind() {
      intersect_clip(
        clip,
        ClipRect {
          x: abs_x,
          y: abs_y,
          width: state.viewport_width(),
          height: state.viewport_height(),
          active: true,
        },
      )
    } else if node.overflow == Overflow::Hidden && hidden_overflow_creates_clip(has_visual, transform) {
      intersect_clip(
        clip,
        ClipRect {
          x: abs_x,
          y: abs_y,
          width: result.size.width,
          height: result.size.height,
          active: true,
        },
      )
    } else {
      clip
    };

    for (child_layout, child_node) in result.children.iter().zip(node.children().iter()) {
      let child_abs_x = abs_x + child_layout.offset.x;
      let child_abs_y = abs_y + child_layout.offset.y;
      if clipped_subtree_is_hidden(
        child_node,
        &child_layout.result,
        child_abs_x,
        child_abs_y,
        transform,
        child_clip,
      ) {
        continue;
      }

      self.collect_quads(
        child_node,
        &child_layout.result,
        child_abs_x,
        child_abs_y,
        abs_x,
        abs_y,
        transform,
        child_clip,
        quads,
      );
    }

    if let LayoutKind::ScrollModifier { state, direction } = node.layout_kind() {
      state.set_viewport_position(abs_x, abs_y);
      let sb_style = node.scrollbar_style();
      state.set_style(sb_style.clone());
      let thumb_color = sb_style.thumb_color;

      match direction {
        ScrollDirection::Vertical | ScrollDirection::Both => {
          if let Some(geo) = state.scrollbar_geometry_for_axis(ScrollAxis::Vertical, &sb_style) {
            if sb_style.track_color.a() > 0 {
              quads.push(Quad {
                x: geo.track_x,
                y: geo.track_y,
                width: geo.track_width,
                height: geo.track_height,
                opacity: DEFAULT_QUAD_OPACITY,
                transform: Transform2D::IDENTITY,
                transform_origin: None,
                content: QuadContent::Rect {
                  color: sb_style.track_color,
                },
                border_radius: Some(crate::node::border::BorderRadius::all(sb_style.track_radius)),
                border: None,
                clip,
              });
            }
            quads.push(Quad {
              x: geo.thumb_x,
              y: geo.thumb_y,
              width: geo.thumb_width,
              height: geo.thumb_height,
              opacity: DEFAULT_QUAD_OPACITY,
              transform: Transform2D::IDENTITY,
              transform_origin: None,
              content: QuadContent::Rect { color: thumb_color },
              border_radius: Some(crate::node::border::BorderRadius::all(sb_style.thumb_radius)),
              border: None,
              clip,
            });
          }
        }
        _ => {}
      }
      match direction {
        ScrollDirection::Horizontal | ScrollDirection::Both => {
          if let Some(geo) = state.scrollbar_geometry_for_axis(ScrollAxis::Horizontal, &sb_style) {
            if sb_style.track_color.a() > 0 {
              quads.push(Quad {
                x: geo.track_x,
                y: geo.track_y,
                width: geo.track_width,
                height: geo.track_height,
                opacity: DEFAULT_QUAD_OPACITY,
                transform: Transform2D::IDENTITY,
                transform_origin: None,
                content: QuadContent::Rect {
                  color: sb_style.track_color,
                },
                border_radius: Some(crate::node::border::BorderRadius::all(sb_style.track_radius)),
                border: None,
                clip,
              });
            }
            quads.push(Quad {
              x: geo.thumb_x,
              y: geo.thumb_y,
              width: geo.thumb_width,
              height: geo.thumb_height,
              opacity: DEFAULT_QUAD_OPACITY,
              transform: Transform2D::IDENTITY,
              transform_origin: None,
              content: QuadContent::Rect { color: thumb_color },
              border_radius: Some(crate::node::border::BorderRadius::all(sb_style.thumb_radius)),
              border: None,
              clip,
            });
          }
        }
        _ => {}
      }
    }
  }

  fn layout_node(&self, glyph_engine: &mut GlyphEngine, node: &Node, constraints: Constraints) -> LayoutResult {
    if let Some(cached) = self.layout_node_from_cache(glyph_engine, node, constraints) {
      return cached;
    }

    self.layout_node_uncached(glyph_engine, node, constraints)
  }

  fn layout_node_from_cache(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
  ) -> Option<LayoutResult> {
    if !node.layout_cache.is_dirty() {
      return node
        .layout_cache
        .get(constraints)
        .map(|cached| Self::prepare_cached_result(node, cached));
    }

    if node.layout_cache.is_local_dirty() || !node.layout_cache.is_descendant_dirty() {
      return None;
    }

    // A descendant changed, but this node may still be able to keep its own
    // geometry. Patch dirty child results into the cached tree and only force
    // this parent to relayout if an immediate child's size or parent-owned
    // offset changed.
    let mut cached = node.layout_cache.get_dirty(constraints)?;
    if cached.children.len() != node.children().len() {
      return None;
    }

    let mut child_size_changed = false;
    for (index, child) in node.children().iter().enumerate() {
      if !child.layout_cache.is_dirty() {
        continue;
      }

      let child_constraints = child.layout_cache.constraints()?;
      let repaired = self.layout_node(glyph_engine, child, child_constraints);
      if repaired.size != cached.children[index].result.size {
        child_size_changed = true;
      }
      if let Some(rect) = child.element_override_rect()
        && (rect.relative_x != cached.children[index].offset.x || rect.relative_y != cached.children[index].offset.y)
      {
        child_size_changed = true;
      }
      cached.children[index].result = repaired;
    }

    if child_size_changed {
      return None;
    }

    let prepared = Self::prepare_cached_result(node, cached);
    node.layout_cache.store(constraints, prepared.clone());
    Some(prepared)
  }

  fn prepare_cached_result(node: &Node, mut cached: LayoutResult) -> LayoutResult {
    if let LayoutKind::ScrollModifier { state, .. } = node.layout_kind() {
      if let Some(child) = cached.children.first_mut() {
        child.offset.x = -state.scroll_x();
        child.offset.y = -state.scroll_y();
      }
      state.update_layout(
        cached.children.first().map(|c| c.result.size.width).unwrap_or(0.0),
        cached.children.first().map(|c| c.result.size.height).unwrap_or(0.0),
        cached.size.width,
        cached.size.height,
      );
    }
    cached
  }

  fn layout_node_uncached(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
  ) -> LayoutResult {
    self.last_recalculated.set(true);
    let frame_handled_by_layout_kind = matches!(node.layout_kind(), LayoutKind::FrameModifier(_));
    let mut result = match node.layout_kind() {
      LayoutKind::Leaf => self.layout_leaf(glyph_engine, node, constraints),
      LayoutKind::Row {
        spacing,
        align,
        justify,
        wrap,
      } => self.layout_flex(
        glyph_engine,
        node,
        constraints,
        *spacing,
        *align,
        *justify,
        *wrap,
        false,
      ),
      LayoutKind::Column {
        spacing,
        align,
        justify,
        wrap,
      } => self.layout_flex(glyph_engine, node, constraints, *spacing, *align, *justify, *wrap, true),
      LayoutKind::Stack { align } => self.layout_stack(glyph_engine, node, constraints, *align),
      LayoutKind::PaddingModifier(padding) => {
        let padding = node.effective_padding(padding);
        self.layout_padding(glyph_engine, node, constraints, &padding)
      }
      LayoutKind::FrameModifier(frame) => {
        let frame = node.effective_frame(*frame);
        self.layout_frame(glyph_engine, node, constraints, &frame)
      }
      LayoutKind::OffsetModifier { x, y } => self.layout_offset(glyph_engine, node, constraints, *x, *y),
      LayoutKind::AbsoluteModifier { width, height, .. } => {
        self.layout_absolute(glyph_engine, node, constraints, *width, *height)
      }
      LayoutKind::AlignModifier(_) => self.layout_passthrough(glyph_engine, node, constraints),
      LayoutKind::FlexModifier(_) => self.layout_passthrough(glyph_engine, node, constraints),
      LayoutKind::ScrollModifier { state, direction } => {
        self.layout_scroll(glyph_engine, node, constraints, state, *direction)
      }
    };

    if !frame_handled_by_layout_kind {
      Self::apply_state_frame(node, &mut result, constraints);
    }
    Self::apply_runtime_rect(node, &mut result);
    node.layout_cache.store(constraints, result.clone());
    result
  }

  fn apply_state_frame(node: &Node, result: &mut LayoutResult, constraints: Constraints) {
    let Some(frame) = node.state_frame() else {
      return;
    };

    if let Some(width) = frame
      .width
      .and_then(|size| Self::resolve_dimension(size, constraints.max_width))
    {
      result.size.width = width;
    }
    if let Some(height) = frame
      .height
      .and_then(|size| Self::resolve_dimension(size, constraints.max_height))
    {
      result.size.height = height;
    }
    if let Some(min_width) = frame
      .min_width
      .and_then(|size| Self::resolve_dimension(size, constraints.max_width))
    {
      result.size.width = result.size.width.max(min_width);
    }
    if let Some(max_width) = frame
      .max_width
      .and_then(|size| Self::resolve_dimension(size, constraints.max_width))
    {
      result.size.width = result.size.width.min(max_width);
    }
    if let Some(min_height) = frame
      .min_height
      .and_then(|size| Self::resolve_dimension(size, constraints.max_height))
    {
      result.size.height = result.size.height.max(min_height);
    }
    if let Some(max_height) = frame
      .max_height
      .and_then(|size| Self::resolve_dimension(size, constraints.max_height))
    {
      result.size.height = result.size.height.min(max_height);
    }
  }

  fn apply_runtime_rect(node: &Node, result: &mut LayoutResult) {
    if let Some(rect) = node.element_override_rect() {
      result.size.width = rect.width;
      result.size.height = rect.height;
    }

    for (child_layout, child_node) in result.children.iter_mut().zip(node.children()) {
      if let Some(rect) = child_node.element_override_rect() {
        child_layout.offset.x = rect.relative_x;
        child_layout.offset.y = rect.relative_y;
      }
    }
  }

  fn layout_leaf(&self, glyph_engine: &mut GlyphEngine, node: &Node, constraints: Constraints) -> LayoutResult {
    match node.node_kind() {
      NodeKind::Text { state, style, .. } => {
        let content = node.text_content().unwrap_or_default();
        return self.layout_text_node(glyph_engine, content, state, style, constraints, node.text_wrap);
      }
      NodeKind::TextInput {
        state,
        style,
        placeholder_style,
      } => {
        let content = state.rendered_text_for_layout();
        return self.layout_text_input(
          glyph_engine,
          state,
          &content,
          style,
          placeholder_style.as_ref(),
          constraints,
        );
      }
      NodeKind::Checkbox { .. } => {
        let preferred = if let NodeKind::Checkbox { state } = node.node_kind() {
          let (width, height) = state.preferred_size(DEFAULT_CHECKBOX_WIDTH, DEFAULT_CHECKBOX_HEIGHT);
          node.intrinsic_size.unwrap_or(Size::new(width, height))
        } else {
          node
            .intrinsic_size
            .unwrap_or(Size::new(DEFAULT_CHECKBOX_WIDTH, DEFAULT_CHECKBOX_HEIGHT))
        };
        return LayoutResult {
          size: constraints.constrain(preferred),
          children: vec![],
        };
      }
      NodeKind::Slider { .. } => {
        let preferred = if let NodeKind::Slider { state } = node.node_kind() {
          let (width, height) = state.preferred_size(
            DEFAULT_SLIDER_WIDTH,
            DEFAULT_SLIDER_HEIGHT,
            DEFAULT_SLIDER_THUMB_MIN_SIZE,
          );
          node.intrinsic_size.unwrap_or(Size::new(width, height))
        } else {
          node
            .intrinsic_size
            .unwrap_or(Size::new(DEFAULT_SLIDER_WIDTH, DEFAULT_SLIDER_HEIGHT))
        };
        return LayoutResult {
          size: constraints.constrain(preferred),
          children: vec![],
        };
      }
      #[cfg(feature = "image")]
      NodeKind::Image { data } => {
        let preferred = node
          .intrinsic_size
          .unwrap_or(Size::new(data.width() as f32, data.height() as f32));
        return LayoutResult {
          size: constraints.constrain(preferred),
          children: vec![],
        };
      }
      #[cfg(feature = "image")]
      NodeKind::ResourceImage { .. } => {
        let preferred = node
          .intrinsic_size
          .unwrap_or(Size::new(DEFAULT_RESOURCE_WIDTH, DEFAULT_RESOURCE_HEIGHT));
        return LayoutResult {
          size: constraints.constrain(preferred),
          children: vec![],
        };
      }
      #[cfg(feature = "svg")]
      NodeKind::Svg { data } => {
        let preferred = node
          .intrinsic_size
          .unwrap_or(Size::new(data.viewbox_width(), data.viewbox_height()));
        return LayoutResult {
          size: constraints.constrain(preferred),
          children: vec![],
        };
      }
      #[cfg(all(feature = "svg", feature = "resources"))]
      NodeKind::ResourceSvg { .. } => {
        let preferred = node
          .intrinsic_size
          .unwrap_or(Size::new(DEFAULT_RESOURCE_WIDTH, DEFAULT_RESOURCE_HEIGHT));
        return LayoutResult {
          size: constraints.constrain(preferred),
          children: vec![],
        };
      }
      NodeKind::Empty => {}
    }

    let preferred = node.intrinsic_size.unwrap_or_default();
    LayoutResult {
      size: constraints.constrain(preferred),
      children: vec![],
    }
  }

  fn layout_text(
    &self,
    glyph_engine: &mut GlyphEngine,
    text: &str,
    style: &TextStyle,
    constraints: Constraints,
    wrap: bool,
  ) -> LayoutResult {
    let max_width = if wrap && constraints.max_width.is_finite() {
      constraints.max_width
    } else {
      f32::MAX
    };
    let measured = glyph_engine.measure_text(text, style, max_width);
    let size = if wrap {
      constraints.constrain(measured)
    } else {
      Size::new(
        measured.width.max(constraints.min_width),
        measured.height.max(constraints.min_height),
      )
    };
    LayoutResult { size, children: vec![] }
  }

  fn layout_text_node(
    &self,
    glyph_engine: &mut GlyphEngine,
    text: &str,
    state: &crate::node::node_kind::TextState,
    style: &TextStyle,
    constraints: Constraints,
    wrap: bool,
  ) -> LayoutResult {
    let max_width = if wrap && constraints.max_width.is_finite() {
      constraints.max_width
    } else {
      f32::MAX
    };
    state.set_caret_positions(glyph_engine.caret_positions(text, style, max_width));
    self.layout_text(glyph_engine, text, style, constraints, wrap)
  }

  fn layout_text_input(
    &self,
    glyph_engine: &mut GlyphEngine,
    state: &crate::node::node_kind::TextInputState,
    text: &str,
    style: &TextStyle,
    placeholder_style: Option<&TextStyle>,
    constraints: Constraints,
  ) -> LayoutResult {
    let display_style = text_input_display_style(state, style, placeholder_style);
    let value = state.value();
    let overflow = state.overflow();
    let caret_positions = glyph_engine.caret_positions(&value, style, f32::MAX);
    state.set_caret_positions(caret_positions);

    let line_height = (style.font_size * style.line_height).max(1.0);
    state.set_caret_height(line_height);
    state.sync_caret_metrics_to_position(line_height);
    let caret_x = state.caret_x() + state.scroll_x();
    let caret_y = state.caret_y() + state.scroll_y();

    let text_constraints = Constraints {
      min_width: 0.0,
      min_height: 0.0,
      ..constraints
    };
    let text_result = self.layout_text(
      glyph_engine,
      text,
      display_style,
      text_constraints,
      overflow == crate::node::node_kind::TextInputOverflow::Multiline,
    );
    let text_height = text_result
      .size
      .height
      .max(explicit_multiline_height(text, line_height));
    let preferred = match overflow {
      crate::node::node_kind::TextInputOverflow::Multiline => {
        let preferred_height = multiline_preferred_height(state, text_height, line_height);
        Size::new(text_result.size.width.max(DEFAULT_TEXT_INPUT_WIDTH), preferred_height)
      }
      crate::node::node_kind::TextInputOverflow::Scroll => Size::new(DEFAULT_TEXT_INPUT_WIDTH, line_height),
    };
    let size = constraints.constrain(preferred);
    match overflow {
      crate::node::node_kind::TextInputOverflow::Scroll => {
        let max_scroll = (text_result.size.width - size.width).max(0.0);
        let mut scroll_x = state.scroll_x().min(max_scroll);
        if caret_x < scroll_x {
          scroll_x = caret_x;
        } else if caret_x > scroll_x + size.width {
          scroll_x = (caret_x - size.width + 1.0).min(max_scroll);
        }
        state.set_scroll_x(scroll_x);
        state.set_scroll_y(0.0);
      }
      crate::node::node_kind::TextInputOverflow::Multiline => {
        state.set_scroll_x(0.0);
        let max_scroll = (text_height - size.height).max(0.0);
        let mut scroll_y = state.scroll_y().min(max_scroll);
        if caret_y < scroll_y {
          scroll_y = caret_y;
        } else if caret_y + line_height > scroll_y + size.height {
          scroll_y = (caret_y + line_height - size.height).min(max_scroll);
        }
        state.set_scroll_y(scroll_y);
      }
    }
    LayoutResult { size, children: vec![] }
  }

  fn layout_flex(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
    spacing: f32,
    align: Alignment,
    justify: Justify,
    wrap: FlexWrap,
    vertical: bool,
  ) -> LayoutResult {
    let children = node.children();
    if children.is_empty() {
      return LayoutResult {
        size: constraints.constrain(Size::default()),
        children: vec![],
      };
    }

    if wrap == FlexWrap::Wrap {
      return self.layout_flex_wrap(glyph_engine, node, constraints, spacing, align, justify, vertical);
    }

    let total_spacing = spacing * (children.len() as f32 - 1.0).max(0.0);
    let max_main = if vertical {
      constraints.max_height
    } else {
      constraints.max_width
    };

    let mut grow_total = 0.0_f32;
    let mut shrink_total = 0.0_f32;
    let mut non_flex_results: Vec<Option<LayoutResult>> = Vec::with_capacity(children.len());
    let mut flex_params_list: Vec<FlexParams> = Vec::with_capacity(children.len());

    for child in children {
      let flex_params = match child.layout_kind() {
        LayoutKind::FlexModifier(params) => Some(child.effective_flex(*params)),
        _ => child.state_flex(),
      };

      if let Some(params) = flex_params {
        grow_total += params.grow;
        shrink_total += params.shrink;
        flex_params_list.push(params);
        if params.grow == 0.0 && params.basis.is_none() {
          let child_constraints = if vertical {
            Constraints {
              min_width: 0.0,
              max_width: constraints.max_width,
              min_height: 0.0,
              max_height: f32::INFINITY,
            }
          } else {
            Constraints {
              min_width: 0.0,
              max_width: f32::INFINITY,
              min_height: 0.0,
              max_height: constraints.max_height,
            }
          };
          non_flex_results.push(Some(self.layout_node(glyph_engine, child, child_constraints)));
        } else {
          non_flex_results.push(None);
        }
      } else {
        flex_params_list.push(FlexParams {
          grow: 0.0,
          shrink: 0.0,
          basis: None,
        });
        let child_constraints = if vertical {
          Constraints {
            min_width: 0.0,
            max_width: constraints.max_width,
            min_height: 0.0,
            max_height: f32::INFINITY,
          }
        } else {
          Constraints {
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: 0.0,
            max_height: constraints.max_height,
          }
        };
        non_flex_results.push(Some(self.layout_node(glyph_engine, child, child_constraints)));
      }
    }

    let non_flex_main: f32 = non_flex_results
      .iter()
      .filter_map(|r| r.as_ref())
      .map(|r| if vertical { r.size.height } else { r.size.width })
      .sum();

    let remaining = max_main - total_spacing - non_flex_main;

    let mut results: Vec<LayoutResult> = Vec::with_capacity(children.len());
    for (i, child) in children.iter().enumerate() {
      if let Some(existing) = non_flex_results[i].take() {
        results.push(existing);
      } else {
        let params = &flex_params_list[i];
        let basis_size = params.basis.unwrap_or(0.0);
        let flex_size = if remaining > 0.0 && grow_total > 0.0 {
          basis_size + remaining.max(0.0) * (params.grow / grow_total)
        } else {
          basis_size
        };
        let child_constraints = if vertical {
          Constraints {
            min_width: 0.0,
            max_width: constraints.max_width,
            min_height: flex_size,
            max_height: flex_size,
          }
        } else {
          Constraints {
            min_width: flex_size,
            max_width: flex_size,
            min_height: 0.0,
            max_height: constraints.max_height,
          }
        };
        results.push(self.layout_node(glyph_engine, child, child_constraints));
      }
    }

    if shrink_total > 0.0 {
      let total_children_main: f32 = results
        .iter()
        .map(|r| if vertical { r.size.height } else { r.size.width })
        .sum();
      let overflow = total_children_main + total_spacing - max_main;
      if overflow > 0.0 {
        let mut remaining_overflow = overflow;
        let mut remaining_shrink = shrink_total;
        let mut frozen = vec![false; children.len()];

        loop {
          let mut any_clamped = false;
          for i in 0..children.len() {
            if frozen[i] {
              continue;
            }
            let params = &flex_params_list[i];
            if params.shrink <= 0.0 {
              continue;
            }
            let child_main = if vertical {
              results[i].size.height
            } else {
              results[i].size.width
            };
            let shrink_amount = remaining_overflow * (params.shrink / remaining_shrink);
            let min_main = children[i].min_main_size(vertical);
            let new_main = (child_main - shrink_amount).max(min_main);
            if new_main > child_main - shrink_amount {
              frozen[i] = true;
              let actual_shrink = child_main - new_main;
              remaining_overflow -= actual_shrink;
              remaining_shrink -= params.shrink;
              any_clamped = true;
              if vertical {
                results[i].size.height = new_main;
              } else {
                results[i].size.width = new_main;
              }
            }
          }
          if !any_clamped {
            break;
          }
          if remaining_shrink <= 0.0 {
            break;
          }
        }

        for i in 0..children.len() {
          if frozen[i] {
            continue;
          }
          let params = &flex_params_list[i];
          if params.shrink <= 0.0 {
            continue;
          }
          let child_main = if vertical {
            results[i].size.height
          } else {
            results[i].size.width
          };
          let shrink_amount = remaining_overflow * (params.shrink / remaining_shrink);
          let new_main = (child_main - shrink_amount).max(0.0);
          if vertical {
            results[i].size.height = new_main;
          } else {
            results[i].size.width = new_main;
          }
        }
      }
    }

    let max_cross: f32 = results
      .iter()
      .map(|r| if vertical { r.size.width } else { r.size.height })
      .fold(0.0_f32, f32::max);

    let total_main: f32 = results
      .iter()
      .map(|r| if vertical { r.size.height } else { r.size.width })
      .sum::<f32>()
      + total_spacing;

    let size = if vertical {
      constraints.constrain(Size::new(max_cross, total_main))
    } else {
      constraints.constrain(Size::new(total_main, max_cross))
    };

    let container_cross = if vertical { size.width } else { size.height };

    if matches!(align, Alignment::Stretch) {
      for (i, child) in children.iter().enumerate() {
        let r = &results[i];
        let child_cross = if vertical { r.size.width } else { r.size.height };
        if child_cross < container_cross {
          let stretch_constraints = if vertical {
            Constraints {
              min_width: container_cross,
              max_width: container_cross,
              min_height: r.size.height,
              max_height: r.size.height,
            }
          } else {
            Constraints {
              min_width: r.size.width,
              max_width: r.size.width,
              min_height: container_cross,
              max_height: container_cross,
            }
          };
          results[i] = self.layout_node(glyph_engine, child, stretch_constraints);
        }
      }
    }

    let child_layouts = self.position_flex_line(&results, &size, spacing, align, justify, vertical);

    LayoutResult {
      size,
      children: child_layouts.into(),
    }
  }

  fn position_flex_line(
    &self,
    results: &[LayoutResult],
    container_size: &Size,
    spacing: f32,
    align: Alignment,
    justify: Justify,
    vertical: bool,
  ) -> Vec<ChildLayout> {
    let container_main = if vertical {
      container_size.height
    } else {
      container_size.width
    };
    let container_cross = if vertical {
      container_size.width
    } else {
      container_size.height
    };
    let children_main: f32 = results
      .iter()
      .map(|r| if vertical { r.size.height } else { r.size.width })
      .sum();
    let free_space = (container_main - children_main).max(0.0);
    let n = results.len() as f32;

    let (leading, gap) = match justify {
      Justify::Start => (0.0, spacing),
      Justify::End => (free_space - spacing * (n - 1.0), spacing),
      Justify::Center => ((free_space - spacing * (n - 1.0)) / 2.0, spacing),
      Justify::SpaceBetween => {
        if n > 1.0 {
          (0.0, free_space / (n - 1.0))
        } else {
          (0.0, 0.0)
        }
      }
      Justify::SpaceAround => {
        let g = free_space / n;
        (g / 2.0, g)
      }
      Justify::SpaceEvenly => {
        let g = free_space / (n + 1.0);
        (g, g)
      }
    };

    let mut child_layouts = Vec::with_capacity(results.len());
    let mut main_cursor = leading;

    for (i, result) in results.iter().enumerate() {
      let child_main = if vertical {
        result.size.height
      } else {
        result.size.width
      };
      let child_cross = if vertical {
        result.size.width
      } else {
        result.size.height
      };
      let cross_offset = align.cross_offset(container_cross, child_cross);

      let offset = if vertical {
        Offset::new(cross_offset, main_cursor)
      } else {
        Offset::new(main_cursor, cross_offset)
      };

      main_cursor += child_main + if i < (n as usize - 1) { gap } else { 0.0 };
      child_layouts.push(ChildLayout {
        offset,
        result: result.clone(),
      });
    }

    child_layouts
  }

  fn layout_flex_wrap(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
    spacing: f32,
    align: Alignment,
    justify: Justify,
    vertical: bool,
  ) -> LayoutResult {
    let children = node.children();
    let max_main = if vertical {
      constraints.max_height
    } else {
      constraints.max_width
    };

    let mut child_results: Vec<Option<LayoutResult>> = children
      .iter()
      .map(|child| {
        let c = if vertical {
          Constraints {
            min_width: constraints.min_width,
            max_width: constraints.max_width,
            min_height: 0.0,
            max_height: f32::INFINITY,
          }
        } else {
          Constraints {
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: constraints.min_height,
            max_height: constraints.max_height,
          }
        };
        Some(self.layout_node(glyph_engine, child, c))
      })
      .collect();

    let mut lines: Vec<Vec<usize>> = vec![vec![]];
    let mut line_main = 0.0_f32;

    for (i, r) in child_results.iter().enumerate() {
      let r = r.as_ref().unwrap();
      let child_main = if vertical { r.size.height } else { r.size.width };
      let needed = if lines.last().unwrap().is_empty() {
        child_main
      } else {
        spacing + child_main
      };

      if !lines.last().unwrap().is_empty() && line_main + needed > max_main {
        lines.push(vec![i]);
        line_main = child_main;
      } else {
        lines.last_mut().unwrap().push(i);
        line_main += needed;
      }
    }

    let mut all_layouts = Vec::with_capacity(children.len());
    all_layouts.resize_with(children.len(), || ChildLayout {
      offset: Offset::default(),
      result: LayoutResult {
        size: Size::default(),
        children: vec![],
      },
    });
    let mut cross_cursor = 0.0_f32;
    let mut max_main_used = 0.0_f32;

    for line_indices in &lines {
      let line_cross: f32 = line_indices
        .iter()
        .map(|&i| {
          let r = child_results[i].as_ref().unwrap();
          if vertical { r.size.width } else { r.size.height }
        })
        .fold(0.0_f32, f32::max);

      let children_main: f32 = line_indices
        .iter()
        .map(|&i| {
          let r = child_results[i].as_ref().unwrap();
          if vertical { r.size.height } else { r.size.width }
        })
        .sum::<f32>();
      let line_main_total = children_main + spacing * (line_indices.len() as f32 - 1.0).max(0.0);
      max_main_used = max_main_used.max(line_main_total);

      let container_main = if vertical {
        max_main.min(constraints.max_height)
      } else {
        max_main.min(constraints.max_width)
      };
      let free_space = (container_main - children_main).max(0.0);
      let n = line_indices.len() as f32;
      let (leading, gap) = match justify {
        Justify::Start => (0.0, spacing),
        Justify::End => (free_space - spacing * (n - 1.0), spacing),
        Justify::Center => ((free_space - spacing * (n - 1.0)) / 2.0, spacing),
        Justify::SpaceBetween => {
          if n > 1.0 {
            (0.0, free_space / (n - 1.0))
          } else {
            (0.0, 0.0)
          }
        }
        Justify::SpaceAround => {
          let g = free_space / n;
          (g / 2.0, g)
        }
        Justify::SpaceEvenly => {
          let g = free_space / (n + 1.0);
          (g, g)
        }
      };

      let mut main_cursor = leading;
      for (j, &idx) in line_indices.iter().enumerate() {
        let result = child_results[idx].take().unwrap();
        let child_main = if vertical {
          result.size.height
        } else {
          result.size.width
        };
        let child_cross = if vertical {
          result.size.width
        } else {
          result.size.height
        };
        let cross_offset = align.cross_offset(line_cross, child_cross) + cross_cursor;
        let offset = if vertical {
          Offset::new(cross_offset, main_cursor)
        } else {
          Offset::new(main_cursor, cross_offset)
        };
        main_cursor += child_main + if j < (n as usize - 1) { gap } else { 0.0 };
        all_layouts[idx] = ChildLayout { offset, result };
      }

      cross_cursor += line_cross + spacing;
    }

    let total_cross = (cross_cursor - spacing).max(0.0);
    let size = if vertical {
      constraints.constrain(Size::new(total_cross, max_main_used))
    } else {
      constraints.constrain(Size::new(max_main_used, total_cross))
    };

    LayoutResult {
      size,
      children: all_layouts,
    }
  }

  fn layout_stack(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
    align: StackAlignment,
  ) -> LayoutResult {
    let children = node.children();
    let results: Vec<LayoutResult> = children
      .iter()
      .map(|child| self.layout_node(glyph_engine, child, constraints))
      .collect();

    let normal_results: Vec<&LayoutResult> = children
      .iter()
      .zip(results.iter())
      .filter(|(child, _)| !matches!(child.layout_kind(), LayoutKind::AbsoluteModifier { .. }))
      .map(|(_, result)| result)
      .collect();

    let max_width = normal_results.iter().map(|r| r.size.width).fold(0.0_f32, f32::max);
    let max_height = normal_results.iter().map(|r| r.size.height).fold(0.0_f32, f32::max);
    let size = constraints.constrain(Size::new(max_width, max_height));

    let child_layouts: Vec<ChildLayout> = results
      .into_iter()
      .zip(children.iter())
      .map(|(result, child)| {
        let offset = match child.layout_kind() {
          LayoutKind::AbsoluteModifier { x, y, .. } => Offset::new(*x, *y),
          _ => {
            let child_align = match child.layout_kind() {
              LayoutKind::AlignModifier(a) => a.to_stack_alignment(),
              _ => align,
            };
            child_align.resolve_offset(size, result.size)
          }
        };
        ChildLayout { offset, result }
      })
      .collect();

    LayoutResult {
      size,
      children: child_layouts.into(),
    }
  }

  fn layout_padding(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
    padding: &Padding,
  ) -> LayoutResult {
    let parent_w = constraints.max_width;
    let parent_h = constraints.max_height;
    let left = padding.get_left().resolve(parent_w);
    let right = padding.get_right().resolve(parent_w);
    let top = padding.get_top().resolve(parent_h);
    let bottom = padding.get_bottom().resolve(parent_h);
    let h_pad = left + right;
    let v_pad = top + bottom;

    let inner_constraints = Constraints {
      min_width: (constraints.min_width - h_pad).max(0.0),
      max_width: (constraints.max_width - h_pad).max(0.0),
      min_height: (constraints.min_height - v_pad).max(0.0),
      max_height: (constraints.max_height - v_pad).max(0.0),
    };

    let child = &node.children()[0];
    let child_result = self.layout_node(glyph_engine, child, inner_constraints);

    let size = constraints.constrain(Size::new(
      child_result.size.width + h_pad,
      child_result.size.height + v_pad,
    ));

    let offset = Offset::new(left, top);

    LayoutResult {
      size,
      children: vec![ChildLayout {
        offset,
        result: child_result,
      }],
    }
  }

  fn layout_frame(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
    frame: &FrameConstraints,
  ) -> LayoutResult {
    let child = &node.children()[0];
    let resolved_width = frame
      .width
      .and_then(|size| Self::resolve_dimension(size, constraints.max_width));
    let resolved_height = frame
      .height
      .and_then(|size| Self::resolve_dimension(size, constraints.max_height));

    let mut c = constraints;
    if let Some(w) = resolved_width {
      c.min_width = w;
      c.max_width = w;
    }
    if let Some(h) = resolved_height {
      c.min_height = h;
      c.max_height = h;
    }

    #[cfg(feature = "image")]
    if matches!(
      child.node_kind(),
      NodeKind::Image { .. } | NodeKind::ResourceImage { .. }
    ) {
      if let Some(intrinsic) = child.intrinsic_size {
        if intrinsic.width > 0.0 && intrinsic.height > 0.0 {
          if let (Some(w), None) = (resolved_width, resolved_height) {
            let h = w * intrinsic.height / intrinsic.width;
            c.min_height = h;
            c.max_height = h;
          } else if let (None, Some(h)) = (resolved_width, resolved_height) {
            let w = h * intrinsic.width / intrinsic.height;
            c.min_width = w;
            c.max_width = w;
          }
        }
      }
    }
    #[cfg(all(feature = "svg", feature = "resources"))]
    let is_svg_media = matches!(child.node_kind(), NodeKind::Svg { .. } | NodeKind::ResourceSvg { .. });
    #[cfg(all(feature = "svg", not(feature = "resources")))]
    let is_svg_media = matches!(child.node_kind(), NodeKind::Svg { .. });

    #[cfg(feature = "svg")]
    if is_svg_media {
      if let Some(intrinsic) = child.intrinsic_size {
        if intrinsic.width > 0.0 && intrinsic.height > 0.0 {
          if let (Some(w), None) = (resolved_width, resolved_height) {
            let h = w * intrinsic.height / intrinsic.width;
            c.min_height = h;
            c.max_height = h;
          } else if let (None, Some(h)) = (resolved_width, resolved_height) {
            let w = h * intrinsic.width / intrinsic.height;
            c.min_width = w;
            c.max_width = w;
          }
        }
      }
    }

    if let Some(v) = frame
      .min_width
      .and_then(|size| Self::resolve_dimension(size, constraints.max_width))
    {
      c.min_width = c.min_width.max(v);
    }
    if let Some(v) = frame
      .max_width
      .and_then(|size| Self::resolve_dimension(size, constraints.max_width))
    {
      c.max_width = c.max_width.min(v);
    }
    if let Some(v) = frame
      .min_height
      .and_then(|size| Self::resolve_dimension(size, constraints.max_height))
    {
      c.min_height = c.min_height.max(v);
    }
    if let Some(v) = frame
      .max_height
      .and_then(|size| Self::resolve_dimension(size, constraints.max_height))
    {
      c.max_height = c.max_height.min(v);
    }

    c.min_width = c.min_width.min(c.max_width);
    c.min_height = c.min_height.min(c.max_height);

    let child_result = self.layout_node(glyph_engine, child, c);
    let size = c.constrain(child_result.size);

    LayoutResult {
      size,
      children: vec![ChildLayout {
        offset: Offset::default(),
        result: child_result,
      }],
    }
  }

  fn resolve_dimension(dimension: Dimension, parent_size: f32) -> Option<f32> {
    match dimension {
      Dimension::Auto => None,
      Dimension::Px(value) => Some(value),
      Dimension::Pct(percent) if parent_size.is_finite() => Some(parent_size * percent / 100.0),
      Dimension::Pct(_) => None,
    }
  }

  fn layout_absolute(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
    width: Option<Dimension>,
    height: Option<Dimension>,
  ) -> LayoutResult {
    let child = &node.children()[0];
    let resolved_width = width.and_then(|size| Self::resolve_dimension(size, constraints.max_width));
    let resolved_height = height.and_then(|size| Self::resolve_dimension(size, constraints.max_height));
    let child_constraints = Constraints {
      min_width: resolved_width.unwrap_or(0.0),
      max_width: resolved_width.unwrap_or(constraints.max_width),
      min_height: resolved_height.unwrap_or(0.0),
      max_height: resolved_height.unwrap_or(constraints.max_height),
    };
    let child_result = self.layout_node(glyph_engine, child, child_constraints);
    let size = Size::new(
      resolved_width.unwrap_or(child_result.size.width),
      resolved_height.unwrap_or(child_result.size.height),
    );

    LayoutResult {
      size,
      children: vec![ChildLayout {
        offset: Offset::default(),
        result: child_result,
      }],
    }
  }

  fn layout_offset(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
    x: f32,
    y: f32,
  ) -> LayoutResult {
    let child = &node.children()[0];
    let child_result = self.layout_node(glyph_engine, child, constraints);
    let size = child_result.size;

    LayoutResult {
      size,
      children: vec![ChildLayout {
        offset: Offset::new(x, y),
        result: child_result,
      }],
    }
  }

  fn layout_scroll(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
    state: &ScrollState,
    direction: ScrollDirection,
  ) -> LayoutResult {
    let child = &node.children()[0];
    let style = node.scrollbar_style();

    let mut reserve_vertical = false;
    let mut reserve_horizontal = false;
    let mut child_result = self.layout_node(
      glyph_engine,
      child,
      scroll_child_constraints(direction, constraints, constraints.max_width, constraints.max_height),
    );
    let mut size = scroll_container_size(constraints, &child_result, &style, reserve_vertical, reserve_horizontal);

    for _ in 0..3 {
      let viewport = reserved_viewport(size, &style, reserve_vertical, reserve_horizontal);
      child_result = self.layout_node(
        glyph_engine,
        child,
        scroll_child_constraints(direction, constraints, viewport.width, viewport.height),
      );
      size = scroll_container_size(constraints, &child_result, &style, reserve_vertical, reserve_horizontal);
      let viewport = reserved_viewport(size, &style, reserve_vertical, reserve_horizontal);
      let next_reserve_vertical = should_reserve_scrollbar(
        &style,
        direction,
        ScrollAxis::Vertical,
        child_result.size.height,
        viewport.height,
      );
      let next_reserve_horizontal = should_reserve_scrollbar(
        &style,
        direction,
        ScrollAxis::Horizontal,
        child_result.size.width,
        viewport.width,
      );

      if next_reserve_vertical == reserve_vertical && next_reserve_horizontal == reserve_horizontal {
        break;
      }

      reserve_vertical = next_reserve_vertical;
      reserve_horizontal = next_reserve_horizontal;
    }

    let viewport = reserved_viewport(size, &style, reserve_vertical, reserve_horizontal);

    state.update_layout_with_container(
      child_result.size.width,
      child_result.size.height,
      viewport.width,
      viewport.height,
      size.width,
      size.height,
    );

    LayoutResult {
      size,
      children: vec![ChildLayout {
        offset: Offset::new(-state.scroll_x(), -state.scroll_y()),
        result: child_result,
      }],
    }
  }

  fn layout_passthrough(&self, glyph_engine: &mut GlyphEngine, node: &Node, constraints: Constraints) -> LayoutResult {
    let child = &node.children()[0];
    let child_result = self.layout_node(glyph_engine, child, constraints);
    let size = child_result.size;

    LayoutResult {
      size,
      children: vec![ChildLayout {
        offset: Offset::default(),
        result: child_result,
      }],
    }
  }
}

fn multiline_preferred_height(
  state: &crate::node::node_kind::TextInputState,
  content_height: f32,
  line_height: f32,
) -> f32 {
  let (min_rows, max_rows) = state.row_limits();
  let min_height = min_rows
    .map(|rows| rows.max(1) as f32 * line_height)
    .unwrap_or(line_height);
  let preferred_height = content_height.max(min_height);
  let preferred_height = max_rows
    .map(|rows| preferred_height.min((rows.max(1) as f32 * line_height).max(min_height)))
    .unwrap_or(preferred_height);

  preferred_height
}

fn explicit_multiline_height(text: &str, line_height: f32) -> f32 {
  text.split('\n').count().max(1) as f32 * line_height
}

fn scroll_child_constraints(
  direction: ScrollDirection,
  constraints: Constraints,
  viewport_width: f32,
  viewport_height: f32,
) -> Constraints {
  match direction {
    ScrollDirection::Vertical => Constraints {
      min_width: constraints.min_width.min(viewport_width),
      max_width: viewport_width,
      min_height: 0.0,
      max_height: f32::INFINITY,
    },
    ScrollDirection::Horizontal => Constraints {
      min_width: 0.0,
      max_width: f32::INFINITY,
      min_height: constraints.min_height.min(viewport_height),
      max_height: viewport_height,
    },
    ScrollDirection::Both => Constraints {
      min_width: 0.0,
      max_width: f32::INFINITY,
      min_height: 0.0,
      max_height: f32::INFINITY,
    },
  }
}

fn scroll_container_size(
  constraints: Constraints,
  child_result: &LayoutResult,
  style: &ScrollBarStyle,
  reserve_vertical: bool,
  reserve_horizontal: bool,
) -> Size {
  let reserved_width = if reserve_vertical {
    reserved_scrollbar_size(style)
  } else {
    0.0
  };
  let reserved_height = if reserve_horizontal {
    reserved_scrollbar_size(style)
  } else {
    0.0
  };
  constraints.constrain(Size::new(
    child_result.size.width.max(constraints.min_width) + reserved_width,
    child_result.size.height.max(constraints.min_height) + reserved_height,
  ))
}

fn reserved_viewport(
  container_size: Size,
  style: &ScrollBarStyle,
  reserve_vertical: bool,
  reserve_horizontal: bool,
) -> Size {
  Size::new(
    (container_size.width
      - if reserve_vertical {
        reserved_scrollbar_size(style)
      } else {
        0.0
      })
    .max(0.0),
    (container_size.height
      - if reserve_horizontal {
        reserved_scrollbar_size(style)
      } else {
        0.0
      })
    .max(0.0),
  )
}

fn should_reserve_scrollbar(
  style: &ScrollBarStyle,
  direction: ScrollDirection,
  axis: ScrollAxis,
  content_size: f32,
  viewport_size: f32,
) -> bool {
  if style.placement != ScrollBarPlacement::Reserved || !scroll_direction_has_axis(direction, axis) {
    return false;
  }

  match style.visible {
    ScrollBarVisibility::Never => false,
    ScrollBarVisibility::Always => true,
    ScrollBarVisibility::Auto => content_size > viewport_size,
  }
}

fn reserved_scrollbar_size(style: &ScrollBarStyle) -> f32 {
  style.width + style.padding * 2.0
}

fn scroll_direction_has_axis(direction: ScrollDirection, axis: ScrollAxis) -> bool {
  matches!(
    (direction, axis),
    (ScrollDirection::Horizontal, ScrollAxis::Horizontal)
      | (ScrollDirection::Vertical, ScrollAxis::Vertical)
      | (ScrollDirection::Both, _)
  )
}

fn intersect_clip(parent: ClipRect, child: ClipRect) -> ClipRect {
  if !parent.active {
    return child;
  }
  if !child.active {
    return parent;
  }

  let x1 = parent.x.max(child.x);
  let y1 = parent.y.max(child.y);
  let x2 = (parent.x + parent.width).min(child.x + child.width);
  let y2 = (parent.y + parent.height).min(child.y + child.height);

  ClipRect {
    x: x1,
    y: y1,
    width: (x2 - x1).max(0.0),
    height: (y2 - y1).max(0.0),
    active: true,
  }
}

fn clipped_subtree_is_hidden(
  node: &Node,
  result: &LayoutResult,
  abs_x: f32,
  abs_y: f32,
  inherited_transform: Transform2D,
  clip: ClipRect,
) -> bool {
  if !clip.active
    || !inherited_transform.is_identity()
    || !node.effective_transform().is_identity()
    || node.overflow == Overflow::Visible
    || border_can_paint_outside(node)
  {
    return false;
  }

  !rect_intersects_clip(abs_x, abs_y, result.size.width, result.size.height, clip)
}

fn hidden_overflow_creates_clip(has_visual: bool, transform: Transform2D) -> bool {
  transform.is_identity() || has_visual
}

fn border_can_paint_outside(node: &Node) -> bool {
  let Some(borders) = node.get_border() else {
    return false;
  };

  [borders.top, borders.right, borders.bottom, borders.left]
    .into_iter()
    .flatten()
    .any(|border| {
      matches!(
        border.placement,
        crate::node::border::BorderPlacement::Outside | crate::node::border::BorderPlacement::Center
      )
    })
}

fn rect_intersects_clip(x: f32, y: f32, width: f32, height: f32, clip: ClipRect) -> bool {
  width > 0.0
    && height > 0.0
    && x < clip.x + clip.width
    && x + width > clip.x
    && y < clip.y + clip.height
    && y + height > clip.y
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn clipped_subtree_culling_keeps_partially_visible_rects() {
    let clip = ClipRect {
      x: 0.0,
      y: 0.0,
      width: 100.0,
      height: 100.0,
      active: true,
    };

    assert!(rect_intersects_clip(90.0, 90.0, 20.0, 20.0, clip));
    assert!(!rect_intersects_clip(120.0, 0.0, 20.0, 20.0, clip));
  }
}
