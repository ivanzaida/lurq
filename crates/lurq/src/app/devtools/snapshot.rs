use std::time::Duration;

use crate::{
  app::{
    ctx::{ComponentContextDebug, ComponentPropsDebug, ComponentSignalDebug},
    profiler::FrameProfile,
    runtime::Tree,
  },
  core::NodeId,
  layout::{
    Alignment, StackAlignment,
    layout_kind::{FlexParams, FlexWrap, FrameConstraints, Justify, LayoutKind, ScrollDirection},
  },
  node::{
    ElementRef,
    border::{Border, BorderPlacement, BorderRadius, Borders},
    dimension::Dimension,
    node_kind::NodeKind,
    padding::Padding,
  },
};

#[derive(Clone, Debug, PartialEq)]
pub struct DevToolsSnapshot {
  pub root: Option<DevToolsNode>,
  pub frame: FrameProfileSnapshot,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DevToolsNode {
  pub id: NodeId,
  pub tag: String,
  pub kind: DevToolsNodeKind,
  pub key: Option<String>,
  pub text: Option<String>,
  pub color: Option<String>,
  pub props: Option<ComponentPropsDebug>,
  pub signals: Vec<ComponentSignalDebug>,
  pub contexts: Vec<ComponentContextDebug>,
  pub shape: Vec<DevToolsShapeRow>,
  pub children: Vec<DevToolsNode>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DevToolsNodeKind {
  Component,
  Element,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DevToolsShapeRow {
  pub label: String,
  pub value: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameProfileSnapshot {
  pub total_ms: f32,
  pub layout_ms: f32,
  pub quad_ms: f32,
  pub glyph_ms: f32,
  pub render_ms: f32,
  pub encode_ms: f32,
  pub present_ms: f32,
  pub quad_count: usize,
  pub rect_count: usize,
  pub glyph_count: usize,
  pub memory_kib: f32,
}

impl DevToolsSnapshot {
  pub fn from_tree(tree: &Tree) -> Self {
    Self {
      root: tree.root().map(snapshot_node),
      frame: FrameProfileSnapshot::from_profile(tree.last_profile()),
    }
  }

  pub fn empty() -> Self {
    Self {
      root: None,
      frame: FrameProfileSnapshot::default(),
    }
  }

  pub fn node_count(&self) -> usize {
    self.root.as_ref().map(count_nodes).unwrap_or(0)
  }

  pub(crate) fn selected_node<'a>(&'a self, path: &[usize]) -> Option<&'a DevToolsNode> {
    let mut node = self.root.as_ref()?;
    for index in path {
      node = node.children.get(*index)?;
    }
    Some(node)
  }
}

impl FrameProfileSnapshot {
  pub fn from_profile(profile: &FrameProfile) -> Self {
    Self {
      total_ms: ms(profile.total),
      layout_ms: ms(profile.layout),
      quad_ms: ms(profile.quad_resolve),
      glyph_ms: ms(profile.glyph_rasterize),
      render_ms: ms(profile.gpu_submit),
      encode_ms: ms(profile.render.encode),
      present_ms: ms(profile.render.present),
      quad_count: profile.quad_count,
      rect_count: profile.rect_count,
      glyph_count: profile.glyph_count,
      memory_kib: profile.memory.total_kib(),
    }
  }
}

fn snapshot_node(element: ElementRef<'_>) -> DevToolsNode {
  let props = element.component_props_debug().cloned();
  let signals = element.component_signals_debug().to_vec();
  let contexts = element.component_contexts_debug().to_vec();
  let kind = if props.is_some() || !signals.is_empty() || !contexts.is_empty() {
    DevToolsNodeKind::Component
  } else {
    DevToolsNodeKind::Element
  };

  DevToolsNode {
    id: element.node_id(),
    tag: element.tag_name().to_owned(),
    kind,
    key: element.component_key().map(str::to_owned),
    text: element.text_content().map(str::to_owned),
    color: element.color().map(|color| color.to_hex()),
    props,
    signals,
    contexts,
    shape: shape_rows(element),
    children: element.children().into_iter().map(snapshot_node).collect(),
  }
}

fn shape_rows(element: ElementRef<'_>) -> Vec<DevToolsShapeRow> {
  let mut rows = Vec::new();
  push_shape_row(&mut rows, "layout", layout_name(element.node.layout_kind()));
  push_shape_row(&mut rows, "node", node_kind_name(element.node.node_kind()));
  push_layout_rows(&mut rows, element.node.layout_kind());

  if let Some(text) = element.text_content() {
    push_shape_row(&mut rows, "text", text);
  }
  if let Some(color) = element.color() {
    push_shape_row(&mut rows, "fill", color.to_hex());
  }
  if let Some(radius) = element.node.get_border_radius() {
    push_shape_row(&mut rows, "radius", format_radius(radius));
  }
  if let Some(border) = element.node.get_border() {
    push_shape_row(&mut rows, "border", format_borders(border));
  }
  if (element.node.opacity - 1.0).abs() > f32::EPSILON {
    push_shape_row(&mut rows, "opacity", format_number(element.node.opacity));
  }

  rows
}

fn push_layout_rows(rows: &mut Vec<DevToolsShapeRow>, layout: &LayoutKind) {
  match layout {
    LayoutKind::Leaf => {}
    LayoutKind::Row {
      spacing,
      align,
      justify,
      wrap,
    }
    | LayoutKind::Column {
      spacing,
      align,
      justify,
      wrap,
    } => {
      push_shape_row(rows, "spacing", format_px(*spacing));
      push_shape_row(rows, "align", alignment_name(*align));
      push_shape_row(rows, "justify", justify_name(*justify));
      push_shape_row(rows, "wrap", flex_wrap_name(*wrap));
    }
    LayoutKind::Stack { align } => {
      push_shape_row(rows, "align", stack_alignment_name(*align));
    }
    LayoutKind::PaddingModifier(padding) => {
      push_shape_row(rows, "padding", format_padding(padding));
    }
    LayoutKind::FrameModifier(frame) => {
      push_frame_rows(rows, frame);
    }
    LayoutKind::OffsetModifier { x, y } => {
      push_shape_row(rows, "offset", format!("x {}, y {}", format_px(*x), format_px(*y)));
    }
    LayoutKind::AbsoluteModifier { x, y, width, height } => {
      push_shape_row(rows, "position", format!("x {}, y {}", format_px(*x), format_px(*y)));
      if let Some(width) = width {
        push_shape_row(rows, "width", format_dimension(width));
      }
      if let Some(height) = height {
        push_shape_row(rows, "height", format_dimension(height));
      }
    }
    LayoutKind::AlignModifier(align) => {
      push_shape_row(rows, "align", alignment_name(*align));
    }
    LayoutKind::FlexModifier(flex) => {
      push_flex_rows(rows, *flex);
    }
    LayoutKind::ScrollModifier { state, direction } => {
      push_shape_row(rows, "direction", scroll_direction_name(*direction));
      push_shape_row(
        rows,
        "scroll",
        format!("x {}, y {}", format_px(state.scroll_x()), format_px(state.scroll_y())),
      );
      push_shape_row(
        rows,
        "viewport",
        format!(
          "{} x {}",
          format_px(state.viewport_width()),
          format_px(state.viewport_height())
        ),
      );
      push_shape_row(
        rows,
        "content",
        format!(
          "{} x {}",
          format_px(state.content_width()),
          format_px(state.content_height())
        ),
      );
    }
  }
}

fn push_frame_rows(rows: &mut Vec<DevToolsShapeRow>, frame: &FrameConstraints) {
  if let Some(value) = frame.width {
    push_shape_row(rows, "width", format_dimension(&value));
  }
  if let Some(value) = frame.height {
    push_shape_row(rows, "height", format_dimension(&value));
  }
  if let Some(value) = frame.min_width {
    push_shape_row(rows, "min width", format_dimension(&value));
  }
  if let Some(value) = frame.max_width {
    push_shape_row(rows, "max width", format_dimension(&value));
  }
  if let Some(value) = frame.min_height {
    push_shape_row(rows, "min height", format_dimension(&value));
  }
  if let Some(value) = frame.max_height {
    push_shape_row(rows, "max height", format_dimension(&value));
  }
}

fn push_flex_rows(rows: &mut Vec<DevToolsShapeRow>, flex: FlexParams) {
  push_shape_row(rows, "grow", format_number(flex.grow));
  push_shape_row(rows, "shrink", format_number(flex.shrink));
  if let Some(basis) = flex.basis {
    push_shape_row(rows, "basis", format_px(basis));
  }
}

fn push_shape_row(rows: &mut Vec<DevToolsShapeRow>, label: impl Into<String>, value: impl Into<String>) {
  rows.push(DevToolsShapeRow {
    label: label.into(),
    value: value.into(),
  });
}

fn layout_name(layout: &LayoutKind) -> &'static str {
  match layout {
    LayoutKind::Leaf => "Leaf",
    LayoutKind::Row { .. } => "Row",
    LayoutKind::Column { .. } => "Column",
    LayoutKind::Stack { .. } => "Stack",
    LayoutKind::PaddingModifier(_) => "Padding",
    LayoutKind::FrameModifier(_) => "Frame",
    LayoutKind::OffsetModifier { .. } => "Offset",
    LayoutKind::AbsoluteModifier { .. } => "Absolute",
    LayoutKind::AlignModifier(_) => "Align",
    LayoutKind::FlexModifier(_) => "Flex",
    LayoutKind::ScrollModifier { .. } => "Scroll",
  }
}

fn node_kind_name(kind: &NodeKind) -> &'static str {
  match kind {
    NodeKind::Empty => "Empty",
    NodeKind::Text { .. } => "Text",
    NodeKind::TextInput { .. } => "TextInput",
    NodeKind::Checkbox { .. } => "Checkbox",
    NodeKind::Slider { .. } => "Slider",
    #[cfg(feature = "image")]
    NodeKind::Image { .. } => "Image",
    #[cfg(feature = "image")]
    NodeKind::ResourceImage { .. } => "ResourceImage",
    #[cfg(feature = "svg")]
    NodeKind::Svg { .. } => "Svg",
    #[cfg(all(feature = "svg", feature = "resources"))]
    NodeKind::ResourceSvg { .. } => "ResourceSvg",
  }
}

fn alignment_name(align: Alignment) -> &'static str {
  match align {
    Alignment::Start => "Start",
    Alignment::Center => "Center",
    Alignment::End => "End",
    Alignment::Stretch => "Stretch",
  }
}

fn stack_alignment_name(align: StackAlignment) -> &'static str {
  match align {
    StackAlignment::TopStart => "TopStart",
    StackAlignment::TopCenter => "TopCenter",
    StackAlignment::TopEnd => "TopEnd",
    StackAlignment::CenterStart => "CenterStart",
    StackAlignment::Center => "Center",
    StackAlignment::CenterEnd => "CenterEnd",
    StackAlignment::BottomStart => "BottomStart",
    StackAlignment::BottomCenter => "BottomCenter",
    StackAlignment::BottomEnd => "BottomEnd",
  }
}

fn justify_name(justify: Justify) -> &'static str {
  match justify {
    Justify::Start => "Start",
    Justify::End => "End",
    Justify::Center => "Center",
    Justify::SpaceBetween => "SpaceBetween",
    Justify::SpaceAround => "SpaceAround",
    Justify::SpaceEvenly => "SpaceEvenly",
  }
}

fn flex_wrap_name(wrap: FlexWrap) -> &'static str {
  match wrap {
    FlexWrap::NoWrap => "NoWrap",
    FlexWrap::Wrap => "Wrap",
  }
}

fn scroll_direction_name(direction: ScrollDirection) -> &'static str {
  match direction {
    ScrollDirection::Horizontal => "Horizontal",
    ScrollDirection::Vertical => "Vertical",
    ScrollDirection::Both => "Both",
  }
}

fn format_padding(padding: &Padding) -> String {
  format!(
    "top {}, right {}, bottom {}, left {}",
    format_dimension(&padding.top),
    format_dimension(&padding.right),
    format_dimension(&padding.bottom),
    format_dimension(&padding.left)
  )
}

fn format_radius(radius: BorderRadius) -> String {
  format!(
    "tl {}, tr {}, br {}, bl {}",
    format_px(radius.top_left),
    format_px(radius.top_right),
    format_px(radius.bottom_right),
    format_px(radius.bottom_left)
  )
}

fn format_borders(borders: Borders) -> String {
  [
    ("top", borders.top),
    ("right", borders.right),
    ("bottom", borders.bottom),
    ("left", borders.left),
  ]
  .into_iter()
  .filter_map(|(side, border)| border.map(|border| format!("{side} {}", format_border(border))))
  .collect::<Vec<_>>()
  .join(", ")
}

fn format_border(border: Border) -> String {
  format!(
    "{} {} {}",
    format_px(border.width),
    border.color.to_hex(),
    border_placement_name(border.placement)
  )
}

fn border_placement_name(placement: BorderPlacement) -> &'static str {
  match placement {
    BorderPlacement::Inside => "inside",
    BorderPlacement::Outside => "outside",
    BorderPlacement::Center => "center",
  }
}

fn format_dimension(value: &Dimension) -> String {
  match value {
    Dimension::Auto => "auto".to_owned(),
    Dimension::Px(value) => format_px(*value),
    Dimension::Pct(value) => format!("{}%", format_number(*value)),
  }
}

fn format_px(value: f32) -> String {
  format!("{}px", format_number(value))
}

fn format_number(value: f32) -> String {
  if (value.fract()).abs() < f32::EPSILON {
    format!("{value:.0}")
  } else {
    format!("{value:.2}")
  }
}

fn count_nodes(node: &DevToolsNode) -> usize {
  1 + node.children.iter().map(count_nodes).sum::<usize>()
}

fn ms(duration: Duration) -> f32 {
  duration.as_secs_f32() * 1000.0
}
