use std::time::Duration;

use crate::{
  app::{
    ctx::{
      ComponentContextDebug, ComponentEffectDebug, ComponentMemoDebug, ComponentSignalDebug, DevtoolsInspectableDebug,
    },
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
    border::{Border, BorderPlacement, Borders, ThemedBorderRadius},
    cursor::CursorIcon,
    dimension::Dimension,
    node_kind::NodeKind,
    padding::Padding,
    radius_value::RadiusValue,
    spacing_value::SpacingValue,
    style::Style,
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
  pub props: Option<DevtoolsInspectableDebug>,
  pub signals: Vec<ComponentSignalDebug>,
  pub memos: Vec<ComponentMemoDebug>,
  pub contexts: Vec<ComponentContextDebug>,
  pub shape: Vec<DevToolsShapeRow>,
  pub effects: Vec<ComponentEffectDebug>,
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
  pub value: Option<String>,
  pub children: Vec<DevToolsShapeRow>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameProfileSnapshot {
  pub fps: u32,
  pub total_ms: f32,
  pub layout_ms: f32,
  pub layout_recalculated: bool,
  pub quad_ms: f32,
  pub glyph_ms: f32,
  pub render_ms: f32,
  pub acquire_ms: f32,
  pub upload_ms: f32,
  pub encode_ms: f32,
  pub submit_ms: f32,
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
      fps: if profile.total.is_zero() {
        0
      } else {
        (1000.0 / ms(profile.total)).round() as u32
      },
      total_ms: ms(profile.total),
      layout_ms: ms(profile.layout),
      layout_recalculated: profile.layout_recalculated,
      quad_ms: ms(profile.quad_resolve),
      glyph_ms: ms(profile.glyph_rasterize),
      render_ms: ms(profile.gpu_submit),
      acquire_ms: ms(profile.render.acquire),
      upload_ms: ms(profile.render.globals_upload + profile.render.atlas_upload),
      encode_ms: ms(profile.render.encode),
      submit_ms: ms(profile.render.submit),
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
  let memos = element.component_memos_debug().to_vec();
  let effects = element.component_effects_debug().to_vec();
  let contexts = element.component_contexts_debug().to_vec();
  let kind = if props.is_some() || !signals.is_empty() || !memos.is_empty() || !contexts.is_empty() {
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
    memos,
    effects,
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
  if let Some(radius) = element.node.state_style().border_radius.or(*element.node.border_radius) {
    push_shape_group(&mut rows, "radius", themed_radius_rows(radius));
  }
  if let Some(border) = element.node.get_border() {
    push_shape_group(&mut rows, "border", border_rows(border));
  }
  if (element.node.opacity - 1.0).abs() > f32::EPSILON {
    push_shape_row(&mut rows, "opacity", format_number(element.node.opacity));
  }
  push_state_style_rows(&mut rows, "hover style", element.node.state_styles.hovered.as_ref());
  push_state_style_rows(&mut rows, "active style", element.node.state_styles.active.as_ref());
  push_state_style_rows(&mut rows, "focused style", element.node.state_styles.focused.as_ref());

  rows
}

fn push_state_style_rows(rows: &mut Vec<DevToolsShapeRow>, label: &str, style: Option<&Style>) {
  let Some(style) = style else {
    return;
  };
  let children = style_rows(style);
  if !children.is_empty() {
    push_shape_group(rows, label, children);
  }
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
      push_shape_row(rows, "spacing", format_spacing_value(spacing));
      push_shape_row(rows, "align", alignment_name(*align));
      push_shape_row(rows, "justify", justify_name(*justify));
      push_shape_row(rows, "wrap", flex_wrap_name(*wrap));
    }
    LayoutKind::Stack { align } => {
      push_shape_row(rows, "align", stack_alignment_name(*align));
    }
    LayoutKind::PaddingModifier(padding) => {
      push_shape_group(rows, "padding", padding_rows(padding));
    }
    LayoutKind::FrameModifier(frame) => {
      push_frame_rows(rows, frame);
    }
    LayoutKind::OffsetModifier { x, y } => {
      push_shape_group(
        rows,
        "offset",
        vec![shape_leaf("x", format_px(*x)), shape_leaf("y", format_px(*y))],
      );
    }
    LayoutKind::AbsoluteModifier { x, y, width, height } => {
      let mut position = vec![shape_leaf("x", format_px(*x)), shape_leaf("y", format_px(*y))];
      if let Some(width) = width {
        position.push(shape_leaf("width", format_dimension(width)));
      }
      if let Some(height) = height {
        position.push(shape_leaf("height", format_dimension(height)));
      }
      push_shape_group(rows, "position", position);
    }
    LayoutKind::AlignModifier(align) => {
      push_shape_row(rows, "align", alignment_name(*align));
    }
    LayoutKind::FlexModifier(flex) => {
      push_flex_rows(rows, *flex);
    }
    LayoutKind::ScrollModifier { state, direction } => {
      push_shape_row(rows, "direction", scroll_direction_name(*direction));
      push_shape_group(
        rows,
        "scroll",
        vec![
          shape_leaf("x", format_px(state.scroll_x())),
          shape_leaf("y", format_px(state.scroll_y())),
        ],
      );
      push_shape_group(
        rows,
        "viewport",
        vec![
          shape_leaf("width", format_px(state.viewport_width())),
          shape_leaf("height", format_px(state.viewport_height())),
        ],
      );
      push_shape_group(
        rows,
        "content",
        vec![
          shape_leaf("width", format_px(state.content_width())),
          shape_leaf("height", format_px(state.content_height())),
        ],
      );
    }
  }
}

fn push_frame_rows(rows: &mut Vec<DevToolsShapeRow>, frame: &FrameConstraints) {
  rows.extend(frame_rows(frame));
}

fn frame_rows(frame: &FrameConstraints) -> Vec<DevToolsShapeRow> {
  let mut rows = Vec::new();
  if let Some(value) = frame.width {
    push_shape_row(&mut rows, "width", format_dimension(&value));
  }
  if let Some(value) = frame.height {
    push_shape_row(&mut rows, "height", format_dimension(&value));
  }
  if let Some(value) = frame.min_width {
    push_shape_row(&mut rows, "min width", format_dimension(&value));
  }
  if let Some(value) = frame.max_width {
    push_shape_row(&mut rows, "max width", format_dimension(&value));
  }
  if let Some(value) = frame.min_height {
    push_shape_row(&mut rows, "min height", format_dimension(&value));
  }
  if let Some(value) = frame.max_height {
    push_shape_row(&mut rows, "max height", format_dimension(&value));
  }
  rows
}

fn push_flex_rows(rows: &mut Vec<DevToolsShapeRow>, flex: FlexParams) {
  rows.extend(flex_rows(flex));
}

fn flex_rows(flex: FlexParams) -> Vec<DevToolsShapeRow> {
  let mut rows = vec![
    shape_leaf("grow", format_number(flex.grow)),
    shape_leaf("shrink", format_number(flex.shrink)),
  ];
  if let Some(basis) = flex.basis {
    rows.push(shape_leaf("basis", format_px(basis)));
  }
  rows
}

fn push_shape_row(rows: &mut Vec<DevToolsShapeRow>, label: impl Into<String>, value: impl Into<String>) {
  rows.push(shape_leaf(label, value));
}

fn push_shape_group(rows: &mut Vec<DevToolsShapeRow>, label: impl Into<String>, children: Vec<DevToolsShapeRow>) {
  rows.push(shape_group(label, children));
}

fn shape_leaf(label: impl Into<String>, value: impl Into<String>) -> DevToolsShapeRow {
  DevToolsShapeRow {
    label: label.into(),
    value: Some(value.into()),
    children: Vec::new(),
  }
}

fn shape_group(label: impl Into<String>, children: Vec<DevToolsShapeRow>) -> DevToolsShapeRow {
  DevToolsShapeRow {
    label: label.into(),
    value: None,
    children,
  }
}

fn style_rows(style: &Style) -> Vec<DevToolsShapeRow> {
  let mut rows = Vec::new();
  if let Some(color) = style.color {
    match color {
      crate::node::BackgroundColor::Color(color) => rows.push(shape_leaf("background", color.to_hex())),
      crate::node::BackgroundColor::Palette(id) => {
        rows.push(shape_leaf("background", format!("palette({})", id.get())))
      }
    }
  }
  if let Some(radius) = style.border_radius {
    rows.push(shape_group("radius", themed_radius_rows(radius)));
  }
  if let Some(border) = style.border {
    rows.push(shape_group("border", border_rows(border)));
  }
  if let Some(cursor) = style.cursor {
    rows.push(shape_leaf("cursor", cursor_name(cursor)));
  }
  if let Some(frame) = &style.frame {
    rows.push(shape_group("frame", frame_rows(frame)));
  }
  if let Some(padding) = &style.padding {
    rows.push(shape_group("padding", padding_rows(padding)));
  }
  if let Some(flex) = style.flex {
    rows.push(shape_group("flex", flex_rows(flex)));
  }
  rows
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

fn padding_rows(padding: &Padding) -> Vec<DevToolsShapeRow> {
  vec![
    shape_leaf("left", format_spacing_value(&padding.left)),
    shape_leaf("right", format_spacing_value(&padding.right)),
    shape_leaf("top", format_spacing_value(&padding.top)),
    shape_leaf("bottom", format_spacing_value(&padding.bottom)),
  ]
}

fn themed_radius_rows(radius: ThemedBorderRadius) -> Vec<DevToolsShapeRow> {
  vec![
    shape_leaf("top left", format_radius_value(radius.top_left)),
    shape_leaf("top right", format_radius_value(radius.top_right)),
    shape_leaf("bottom right", format_radius_value(radius.bottom_right)),
    shape_leaf("bottom left", format_radius_value(radius.bottom_left)),
  ]
}

fn border_rows(borders: Borders) -> Vec<DevToolsShapeRow> {
  [
    ("top", borders.top),
    ("right", borders.right),
    ("bottom", borders.bottom),
    ("left", borders.left),
  ]
  .into_iter()
  .filter_map(|(side, border)| border.map(|border| shape_group(side, single_border_rows(border))))
  .collect()
}

fn single_border_rows(border: Border) -> Vec<DevToolsShapeRow> {
  vec![
    shape_leaf("width", format_px(border.width)),
    shape_leaf("color", format_background_color(border.color)),
    shape_leaf("placement", border_placement_name(border.placement)),
  ]
}

fn format_background_color(color: crate::node::BackgroundColor) -> String {
  match color {
    crate::node::BackgroundColor::Color(color) => color.to_hex(),
    crate::node::BackgroundColor::Palette(id) => format!("palette({})", id.get()),
  }
}

fn border_placement_name(placement: BorderPlacement) -> &'static str {
  match placement {
    BorderPlacement::Inside => "inside",
    BorderPlacement::Outside => "outside",
    BorderPlacement::Center => "center",
  }
}

fn cursor_name(cursor: CursorIcon) -> String {
  format!("{cursor:?}")
}

fn format_dimension(value: &Dimension) -> String {
  match value {
    Dimension::Auto => "auto".to_owned(),
    Dimension::Px(value) => format_px(*value),
    Dimension::Pct(value) => format!("{}%", format_number(*value)),
  }
}

fn format_spacing_value(value: &SpacingValue) -> String {
  match value {
    SpacingValue::Dimension(value) => format_dimension(value),
    SpacingValue::Theme(id) => format!("spacing({})", id.get()),
  }
}

fn format_radius_value(value: RadiusValue) -> String {
  match value {
    RadiusValue::Px(value) => format_px(value),
    RadiusValue::Theme(id) => format!("radius({})", id.get()),
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
