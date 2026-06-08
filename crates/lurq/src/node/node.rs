use std::sync::Arc;

#[cfg(feature = "devtools")]
use crate::app::ctx::{
  ComponentContextDebug, ComponentEffectDebug, ComponentMemoDebug, ComponentSignalDebug, DevtoolsInspectableDebug,
};
use crate::{
  animation::{Animation, Transition},
  app::{
    events::{DragEvent, DropEvent, KeyboardEvent, MouseButton, MouseEvent, ScrollEvent},
    theme::{CaretMode, TypographyStyle},
  },
  core::{ElementRef as CoreElementRef, Guard, IdGenerator, NodeId, Signal},
  layout::{
    Alignment, Offset, Size, StackAlignment,
    layout_kind::{FlexParams, FrameConstraints, LayoutKind, Overflow, Position},
    scrollbar::ScrollBarStyle,
    text_style::{TextAlign, TextStyle},
  },
  node::{
    BackgroundColor, TextColor, TextTransformMode,
    border::{Border, BorderRadius, Borders, ThemedBorderRadius},
    border_size_value::BorderSizeValue,
    checkbox_style::CheckboxStyle,
    color::Color,
    cursor::CursorIcon,
    dimension::Dimension,
    gradient::Gradient,
    interaction_state::InteractionState,
    node_kind::{
      CheckboxState, NodeKind, SelectChangeCallback, SelectState, SliderState, TextInputState, TextOverflow, TextState,
      TextStyleSource,
    },
    padding::Padding,
    radius_value::RadiusValue,
    slider_style::SliderPartStyle,
    spacing_value::SpacingValue,
    style::{StateStyles, Style},
    transform::Transform2D,
  },
};

type Callback<T> = Arc<dyn Fn(&T) + Send + Sync>;
type VoidCallback = Arc<dyn Fn() + Send + Sync>;
type ScrollbarStyleCallback = Arc<dyn Fn(ScrollBarStyle) -> ScrollBarStyle + Send + Sync>;
#[cfg(feature = "form")]
type FormSubmitCallback = Arc<dyn Fn(FormData) + Send + Sync>;
const DEFAULT_TEXT_WRAP: bool = true;
const DEFAULT_OPACITY: f32 = 1.0;

#[cfg(feature = "form")]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FormData {
  fields: Vec<(String, String)>,
}

#[cfg(feature = "form")]
impl FormData {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn append(&mut self, name: impl Into<String>, value: impl Into<String>) {
    self.fields.push((name.into(), value.into()));
  }

  pub fn get(&self, name: &str) -> Option<&str> {
    self
      .fields
      .iter()
      .find(|(field_name, _)| field_name == name)
      .map(|(_, value)| value.as_str())
  }

  pub fn get_all(&self, name: &str) -> Vec<&str> {
    self
      .fields
      .iter()
      .filter(|(field_name, _)| field_name == name)
      .map(|(_, value)| value.as_str())
      .collect()
  }

  pub fn entries(&self) -> &[(String, String)] {
    &self.fields
  }

  pub fn len(&self) -> usize {
    self.fields.len()
  }

  pub fn is_empty(&self) -> bool {
    self.fields.is_empty()
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonKind {
  Button,
  #[cfg(feature = "form")]
  Submit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackgroundSize {
  Stretch,
  Cover,
  Contain,
}

impl Default for BackgroundSize {
  fn default() -> Self {
    Self::Stretch
  }
}

#[derive(Default, Clone)]
pub struct EventHandlers {
  pub on_click: Option<Callback<MouseEvent>>,
  pub on_mouse_click: Vec<(MouseButton, Callback<MouseEvent>)>,
  pub on_dblclick: Option<Callback<MouseEvent>>,
  pub on_mouse_down: Option<Callback<MouseEvent>>,
  pub on_mouse_up: Option<Callback<MouseEvent>>,
  pub on_mouse_move: Option<Callback<MouseEvent>>,
  pub on_drag_start: Option<Callback<DragEvent>>,
  pub on_drag_move: Option<Callback<DragEvent>>,
  pub on_drag_end: Option<Callback<DragEvent>>,
  pub on_drop: Option<Callback<DropEvent>>,
  pub on_mouse_enter: Option<VoidCallback>,
  pub on_mouse_leave: Option<VoidCallback>,
  pub on_key_down: Option<Callback<KeyboardEvent>>,
  pub on_key_up: Option<Callback<KeyboardEvent>>,
  pub on_focus: Option<VoidCallback>,
  pub on_blur: Option<VoidCallback>,
  #[cfg(feature = "form")]
  pub on_submit: Option<FormSubmitCallback>,
  pub on_scroll: Option<Callback<ScrollEvent>>,
  pub on_scroll_start: Option<Callback<ScrollEvent>>,
  pub on_scroll_end: Option<Callback<ScrollEvent>>,
}

pub(crate) struct Node {
  pub(crate) node_id: NodeId,
  pub(crate) tag_name: Arc<str>,
  pub(crate) component_slot_id: Option<u64>,
  pub(crate) component_key: Option<Arc<str>>,
  #[cfg(feature = "devtools")]
  pub(crate) component_props_debug: Option<DevtoolsInspectableDebug>,
  #[cfg(feature = "devtools")]
  pub(crate) component_signals_debug: Vec<ComponentSignalDebug>,
  #[cfg(feature = "devtools")]
  pub(crate) component_memos_debug: Vec<ComponentMemoDebug>,
  #[cfg(feature = "devtools")]
  pub(crate) component_effects_debug: Vec<ComponentEffectDebug>,
  #[cfg(feature = "devtools")]
  pub(crate) component_contexts_debug: Vec<ComponentContextDebug>,
  #[cfg(feature = "devtools")]
  pub(crate) debug_attrs: Vec<(Arc<str>, Arc<str>)>,
  pub(crate) layout_kind: LayoutKind,
  pub(crate) frame: FrameConstraints,
  pub(crate) padding: Padding,
  pub(crate) position: Position,
  pub(crate) offset: Option<Offset>,
  pub(crate) align_self: Option<Alignment>,
  pub(crate) flex: Option<FlexParams>,
  pub(crate) node_kind: NodeKind,
  pub(crate) text_content: Guard<Option<String>>,
  pub(crate) text_wrap: bool,
  pub(crate) text_overflow: TextOverflow,
  pub(crate) overflow: Overflow,
  pub(crate) intrinsic_size: Option<Size>,
  pub(crate) color: Guard<Option<BackgroundColor>>,
  pub(crate) gradient: Guard<Option<Gradient>>,
  pub(crate) border_radius: Guard<Option<ThemedBorderRadius>>,
  pub(crate) border: Guard<Option<Borders>>,
  pub(crate) caret_color: Guard<Option<TextColor>>,
  pub(crate) caret_mode: Guard<Option<CaretMode>>,
  pub(crate) cursor: Option<CursorIcon>,
  #[cfg(feature = "image")]
  pub(crate) background_image: Guard<Option<crate::images::ImageData>>,
  #[cfg(feature = "image")]
  pub(crate) background_size: BackgroundSize,
  #[cfg(all(feature = "image", feature = "resources"))]
  pub(crate) background_resource_image: Option<Arc<str>>,
  pub(crate) scrollbar_style: Guard<Option<ScrollBarStyle>>,
  pub(crate) scrollbar_hovered_style: Option<ScrollbarStyleCallback>,
  pub(crate) element_ref: Option<CoreElementRef>,
  pub(crate) interaction: Option<InteractionState>,
  pub(crate) focusable: bool,
  pub(crate) tab_index: Option<i32>,
  pub(crate) button_kind: Option<ButtonKind>,
  #[cfg(feature = "form")]
  pub(crate) form_name: Option<Arc<str>>,
  pub(crate) style_state: InteractionState,
  pub(crate) state_styles: StateStyles,
  pub(crate) opacity: f32,
  pub(crate) transform: Transform2D,
  pub(crate) animation_overrides: Vec<(crate::animation::AnimatableProperty, crate::animation::AnimatableValue)>,
  pub(crate) transitions: Vec<Transition>,
  pub(crate) animation: Option<Animation>,
  pub(crate) layout_cache: crate::node::layout_cache::LayoutCache,
  pub(crate) children: Vec<Node>,
  pub(crate) events: EventHandlers,
}

impl Default for Node {
  fn default() -> Self {
    Self::new()
  }
}

impl Node {
  fn from_parts(layout_kind: LayoutKind, node_kind: NodeKind, children: Vec<Node>) -> Self {
    Self {
      layout_kind,
      frame: FrameConstraints::default(),
      padding: Padding::default(),
      position: Position::Static,
      offset: None,
      align_self: None,
      flex: None,
      node_kind,
      node_id: NodeId::UNASSIGNED,
      tag_name: Arc::from("Node"),
      component_slot_id: None,
      component_key: None,
      #[cfg(feature = "devtools")]
      component_props_debug: None,
      #[cfg(feature = "devtools")]
      component_signals_debug: Vec::new(),
      #[cfg(feature = "devtools")]
      component_memos_debug: Vec::new(),
      #[cfg(feature = "devtools")]
      component_effects_debug: Vec::new(),
      #[cfg(feature = "devtools")]
      component_contexts_debug: Vec::new(),
      #[cfg(feature = "devtools")]
      debug_attrs: Vec::new(),
      text_content: Guard::new(None),
      text_wrap: DEFAULT_TEXT_WRAP,
      text_overflow: TextOverflow::default(),
      overflow: Overflow::Hidden,
      intrinsic_size: None,
      color: Guard::new(None),
      gradient: Guard::new(None),
      border_radius: Guard::new(None),
      border: Guard::new(None),
      caret_color: Guard::new(None),
      caret_mode: Guard::new(None),
      cursor: None,
      #[cfg(feature = "image")]
      background_image: Guard::new(None),
      #[cfg(feature = "image")]
      background_size: BackgroundSize::default(),
      #[cfg(all(feature = "image", feature = "resources"))]
      background_resource_image: None,
      scrollbar_style: Guard::new(None),
      scrollbar_hovered_style: None,
      element_ref: None,
      interaction: None,
      focusable: false,
      tab_index: None,
      button_kind: None,
      #[cfg(feature = "form")]
      form_name: None,
      style_state: InteractionState::new(),
      state_styles: StateStyles::default(),
      opacity: DEFAULT_OPACITY,
      transform: Transform2D::IDENTITY,
      animation_overrides: Vec::new(),
      transitions: Vec::new(),
      animation: None,
      layout_cache: Default::default(),
      children,
      events: EventHandlers::default(),
    }
  }

  fn with_text_content(mut self, content: &str) -> Self {
    self.text_content.set(Some(content.to_owned()));
    self
  }

  pub fn text_wrap(mut self, wrap: bool) -> Self {
    self.text_wrap = wrap;
    self.layout_cache.invalidate();
    self
  }

  pub fn text_overflow(mut self, overflow: TextOverflow) -> Self {
    self.text_overflow = overflow;
    self.layout_cache.invalidate();
    self
  }

  pub fn new() -> Self {
    Self::from_parts(LayoutKind::Leaf, NodeKind::Empty, vec![])
  }

  #[cfg_attr(not(feature = "form"), allow(dead_code))]
  pub fn logical() -> Self {
    Self::from_parts(LayoutKind::LogicalModifier, NodeKind::Empty, vec![])
  }

  pub fn text(content: &str) -> Self {
    let node = Self::from_parts(
      LayoutKind::Leaf,
      NodeKind::Text {
        state: TextState::new(),
        style: TextStyleSource::default_style(),
        transform_mode: TextTransformMode::default(),
      },
      vec![],
    );
    node.with_text_content(content)
  }

  pub fn text_styled(content: &str, style: TextStyle) -> Self {
    let node = Self::from_parts(
      LayoutKind::Leaf,
      NodeKind::Text {
        state: TextState::new(),
        style: TextStyleSource::explicit(style),
        transform_mode: TextTransformMode::default(),
      },
      vec![],
    );
    node.with_text_content(content)
  }

  pub fn text_input(value: Signal<String>) -> Self {
    Self::text_input_styled(value, TextStyle::default())
  }

  pub fn text_input_styled(value: Signal<String>, style: TextStyle) -> Self {
    let rendered = value.get_untracked();
    let node = Self::from_parts(
      LayoutKind::Leaf,
      NodeKind::TextInput {
        state: TextInputState::new(value),
        style,
        placeholder_style: None,
      },
      vec![],
    );
    if rendered.is_empty() {
      node
    } else {
      node.with_text_content(&rendered)
    }
  }

  pub fn checkbox(value: Signal<bool>) -> Self {
    Self::from_parts(
      LayoutKind::Leaf,
      NodeKind::Checkbox {
        state: CheckboxState::new(value),
      },
      vec![],
    )
  }

  pub fn checkbox_box_style(self, style: CheckboxStyle) -> Self {
    if let Some(state) = self.checkbox_state() {
      state.set_style(style);
    }
    self
  }

  pub fn checkbox_checked_box_style(self, style: CheckboxStyle) -> Self {
    if let Some(state) = self.checkbox_state() {
      state.set_checked_style(style);
    }
    self
  }

  pub fn checkbox_box_hovered_style(self, style: CheckboxStyle) -> Self {
    if let Some(state) = self.checkbox_state() {
      state.set_hovered_style(style);
    }
    self
  }

  pub fn checkbox_checked_box_hovered_style(self, style: CheckboxStyle) -> Self {
    if let Some(state) = self.checkbox_state() {
      state.set_checked_hovered_style(style);
    }
    self
  }

  pub fn slider(value: Signal<i32>) -> Self {
    Self::from_parts(
      LayoutKind::Leaf,
      NodeKind::Slider {
        state: SliderState::new(value),
      },
      vec![],
    )
  }

  pub fn slider_f32(value: Signal<f32>) -> Self {
    Self::from_parts(
      LayoutKind::Leaf,
      NodeKind::Slider {
        state: SliderState::new_f32(value),
      },
      vec![],
    )
  }

  pub fn select() -> Self {
    let state = SelectState::new();
    let toggle = state.clone();
    let mut node = Self::row(0.0, Alignment::Center, vec![]);
    node.node_kind = NodeKind::Select { state };
    node.events.on_click = Some(std::sync::Arc::new(move |_| toggle.toggle_open()));
    node
  }

  fn select_state(&self) -> Option<&SelectState> {
    match &self.node_kind {
      NodeKind::Select { state } => Some(state),
      _ => None,
    }
  }

  pub fn select_labels(self, labels: Vec<std::sync::Arc<str>>) -> Self {
    if let Some(state) = self.select_state() {
      state.set_labels(labels);
    }
    self
  }

  pub fn select_selected(self, selected: Vec<usize>) -> Self {
    if let Some(state) = self.select_state() {
      state.set_selected(selected);
    }
    self
  }

  pub fn select_multiple(self, multiple: bool) -> Self {
    if let Some(state) = self.select_state() {
      state.set_multiple(multiple);
    }
    self
  }

  pub fn select_placeholder(self, placeholder: Option<std::sync::Arc<str>>) -> Self {
    if let Some(state) = self.select_state() {
      state.set_placeholder(placeholder);
    }
    self
  }

  pub fn select_style(mut self, style: crate::node::SelectStyle) -> Self {
    let trigger = style.resolved_trigger(false, false, false);
    if let Some(padding) = trigger.padding.clone() {
      self = self.padding_custom(padding);
    }
    if let Some(min_width) = trigger.min_width {
      self = self.min_width(min_width);
    }
    if let Some(min_height) = trigger.min_height {
      self = self.min_height(min_height);
    }
    if let Some(state) = self.select_state() {
      state.set_style(style);
    }
    self
  }

  pub fn select_on_change(self, on_change: SelectChangeCallback) -> Self {
    if let Some(state) = self.select_state() {
      state.set_on_change(on_change);
    }
    self
  }

  /// Apply a `SelectPartStyle`'s box fields (background/border/radius/padding/
  /// min size) to this node. Used by the runtime when building the popup menu.
  pub(crate) fn apply_select_part(mut self, part: &crate::node::select_style::SelectPartStyle) -> Self {
    if let Some(background) = &part.background {
      self.color.set(Some(background.clone()));
    }
    if let Some(border) = &part.border {
      self.border.set(Some(border.clone()));
    }
    if let Some(radius) = part.border_radius {
      self.border_radius.set(Some(radius));
    }
    if let Some(padding) = &part.padding {
      self = self.padding_custom(padding.clone());
    }
    if let Some(min_width) = part.min_width {
      self = self.min_width(min_width);
    }
    if let Some(min_height) = part.min_height {
      self = self.min_height(min_height);
    }
    self
  }

  #[cfg(feature = "image")]
  pub fn image(data: crate::images::ImageData) -> Self {
    let mut node = Self::from_parts(LayoutKind::Leaf, NodeKind::Image { data: data.clone() }, vec![]);
    node.intrinsic_size = Some(Size::new(data.width() as f32, data.height() as f32));
    node
  }

  #[cfg(feature = "image")]
  pub fn resource_image(path: &str) -> Self {
    Self::from_parts(LayoutKind::Leaf, NodeKind::ResourceImage { path: path.into() }, vec![])
  }

  #[cfg(feature = "svg")]
  pub fn svg(data: crate::svg::SvgData) -> Self {
    let mut node = Self::from_parts(LayoutKind::Leaf, NodeKind::Svg { data: data.clone() }, vec![]);
    node.intrinsic_size = Some(Size::new(data.viewbox_width(), data.viewbox_height()));
    node
  }

  #[cfg(all(feature = "svg", feature = "resources"))]
  pub fn resource_svg(path: &str) -> Self {
    Self::from_parts(LayoutKind::Leaf, NodeKind::ResourceSvg { path: path.into() }, vec![])
  }

  pub fn row(spacing: impl Into<SpacingValue>, align: Alignment, children: Vec<Node>) -> Self {
    Self::from_parts(
      LayoutKind::Row {
        spacing: spacing.into(),
        align,
        justify: crate::layout::layout_kind::Justify::Start,
        wrap: crate::layout::layout_kind::FlexWrap::NoWrap,
      },
      NodeKind::Empty,
      children,
    )
  }

  pub fn column(spacing: impl Into<SpacingValue>, align: Alignment, children: Vec<Node>) -> Self {
    Self::from_parts(
      LayoutKind::Column {
        spacing: spacing.into(),
        align,
        justify: crate::layout::layout_kind::Justify::Start,
        wrap: crate::layout::layout_kind::FlexWrap::NoWrap,
      },
      NodeKind::Empty,
      children,
    )
  }

  pub fn stack(align: StackAlignment, children: Vec<Node>) -> Self {
    Self::from_parts(LayoutKind::Stack { align }, NodeKind::Empty, children)
  }

  pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
    let padding = padding.into();
    self.padding.merge_from(&padding);
    self.layout_cache.invalidate();
    self
  }

  pub fn padding_custom(self, padding: Padding) -> Self {
    self.padding(padding)
  }

  pub fn frame(mut self, frame: FrameConstraints) -> Self {
    self.frame = merge_frame(self.frame, frame);
    self.layout_cache.invalidate();
    self
  }

  pub fn offset(mut self, x: f32, y: f32) -> Self {
    self.offset = Some(Offset::new(x, y));
    self.layout_cache.invalidate();
    self
  }

  pub(crate) fn absolute_positioned(self, x: f32, y: f32, width: Option<Dimension>, height: Option<Dimension>) -> Self {
    let mut node = self;
    node.position = Position::Absolute { x, y, width, height };
    node.layout_cache.invalidate();
    node
  }

  pub fn align(mut self, alignment: Alignment) -> Self {
    self.align_self = Some(alignment);
    self.layout_cache.invalidate();
    self
  }

  pub fn flex(mut self, factor: f32) -> Self {
    self.flex = Some(crate::layout::layout_kind::FlexParams::grow(factor));
    self.layout_cache.invalidate();
    self
  }

  pub fn flex_shrink(mut self, factor: f32) -> Self {
    self.flex = Some(crate::layout::layout_kind::FlexParams {
      grow: 0.0,
      shrink: factor,
      basis: None,
    });
    self.layout_cache.invalidate();
    self
  }

  pub fn flex_full(mut self, grow: f32, shrink: f32, basis: Option<f32>) -> Self {
    self.flex = Some(crate::layout::layout_kind::FlexParams { grow, shrink, basis });
    self.layout_cache.invalidate();
    self
  }

  pub fn background(mut self, color: impl Into<BackgroundColor>) -> Self {
    self.color.set(Some(color.into()));
    self
  }

  pub fn background_gradient(mut self, gradient: impl Into<Gradient>) -> Self {
    self.gradient.set(Some(gradient.into()));
    self
  }

  pub(crate) fn caret_color(mut self, color: impl Into<TextColor>) -> Self {
    self.set_caret_color(color.into());
    self
  }

  fn set_caret_color(&mut self, color: TextColor) {
    if matches!(self.node_kind, NodeKind::TextInput { .. }) {
      self.caret_color.set(Some(color));
    } else {
      self.caret_color.set(Some(color));
    }
  }

  pub(crate) fn text_input_caret_mode(mut self, mode: CaretMode) -> Self {
    if matches!(self.node_kind, NodeKind::TextInput { .. }) {
      self.caret_mode.set(Some(mode));
    }
    self
  }

  pub fn corner_radius(mut self, radius: impl Into<RadiusValue>) -> Self {
    self.border_radius.set(Some(ThemedBorderRadius::all(radius)));
    self
  }

  pub fn corner_radius_custom(mut self, radius: BorderRadius) -> Self {
    self.border_radius.set(Some(radius.into()));
    self
  }

  pub fn corner_radius_top_left(mut self, radius: impl Into<RadiusValue>) -> Self {
    let mut border_radius = (*self.border_radius).unwrap_or_default();
    border_radius.top_left = radius.into();
    self.border_radius.set(Some(border_radius));
    self
  }

  pub fn corner_radius_top_right(mut self, radius: impl Into<RadiusValue>) -> Self {
    let mut border_radius = (*self.border_radius).unwrap_or_default();
    border_radius.top_right = radius.into();
    self.border_radius.set(Some(border_radius));
    self
  }

  pub fn corner_radius_bottom_right(mut self, radius: impl Into<RadiusValue>) -> Self {
    let mut border_radius = (*self.border_radius).unwrap_or_default();
    border_radius.bottom_right = radius.into();
    self.border_radius.set(Some(border_radius));
    self
  }

  pub fn corner_radius_bottom_left(mut self, radius: impl Into<RadiusValue>) -> Self {
    let mut border_radius = (*self.border_radius).unwrap_or_default();
    border_radius.bottom_left = radius.into();
    self.border_radius.set(Some(border_radius));
    self
  }

  pub fn rounded(mut self, radius: impl Into<RadiusValue>) -> Self {
    self.border_radius.set(Some(ThemedBorderRadius::all(radius)));
    self
  }

  pub fn border_inside(mut self, width: impl Into<BorderSizeValue>, color: impl Into<BackgroundColor>) -> Self {
    self.border.set(Some(Borders::all(Border::inside(width, color))));
    self
  }

  pub fn border_outside(mut self, width: impl Into<BorderSizeValue>, color: impl Into<BackgroundColor>) -> Self {
    self.border.set(Some(Borders::all(Border::outside(width, color))));
    self
  }

  pub fn border_center(mut self, width: impl Into<BorderSizeValue>, color: impl Into<BackgroundColor>) -> Self {
    self.border.set(Some(Borders::all(Border::center(width, color))));
    self
  }

  pub fn border(mut self, border: Border) -> Self {
    self.border.set(Some(Borders::all(border)));
    self
  }

  pub fn border_custom(mut self, border: Borders) -> Self {
    self.border.set(Some(border));
    self
  }

  pub fn border_top(mut self, border: Border) -> Self {
    let mut borders = <Option<Borders> as Clone>::clone(&self.border).unwrap_or_default();
    borders.top = Some(border);
    self.border.set(Some(borders));
    self
  }

  pub fn border_right(mut self, border: Border) -> Self {
    let mut borders = <Option<Borders> as Clone>::clone(&self.border).unwrap_or_default();
    borders.right = Some(border);
    self.border.set(Some(borders));
    self
  }

  pub fn border_bottom(mut self, border: Border) -> Self {
    let mut borders = <Option<Borders> as Clone>::clone(&self.border).unwrap_or_default();
    borders.bottom = Some(border);
    self.border.set(Some(borders));
    self
  }

  pub fn border_left(mut self, border: Border) -> Self {
    let mut borders = <Option<Borders> as Clone>::clone(&self.border).unwrap_or_default();
    borders.left = Some(border);
    self.border.set(Some(borders));
    self
  }

  pub fn cursor(mut self, cursor: CursorIcon) -> Self {
    self.cursor = Some(cursor);
    self
  }

  #[cfg(feature = "image")]
  pub fn background_image(mut self, data: impl Into<crate::images::ImageKind>) -> Self {
    match data.into() {
      crate::images::ImageKind::Bytes(data) => {
        self.background_image.set(Some(data));
      }
      crate::images::ImageKind::Native(data) => {
        self.background_image.set(Some(data.image_data()));
      }
      #[cfg(feature = "resources")]
      crate::images::ImageKind::Resource(path) => {
        self.background_resource_image = Some(path);
      }
    }
    self
  }

  #[cfg(feature = "image")]
  pub fn background_size(mut self, size: BackgroundSize) -> Self {
    self.background_size = size;
    self
  }

  #[cfg(feature = "image")]
  pub fn background_cover(self) -> Self {
    self.background_size(BackgroundSize::Cover)
  }

  #[cfg(feature = "image")]
  pub fn background_contain(self) -> Self {
    self.background_size(BackgroundSize::Contain)
  }

  pub fn cursor_icon(&self) -> Option<CursorIcon> {
    self.state_style().cursor.or(self.cursor)
  }

  pub fn hovered_style(mut self, style: Style) -> Self {
    self.state_styles.hovered = Some(style);
    self
  }

  pub fn active_style(mut self, style: Style) -> Self {
    self.state_styles.active = Some(style);
    self
  }

  pub fn focused_style(mut self, style: Style) -> Self {
    self.state_styles.focused = Some(style);
    self
  }

  pub fn hovered(self, f: impl FnOnce(Style) -> Style) -> Self {
    self.hovered_style(f(Style::new()))
  }

  pub fn active(self, f: impl FnOnce(Style) -> Style) -> Self {
    self.active_style(f(Style::new()))
  }

  pub fn focused(self, f: impl FnOnce(Style) -> Style) -> Self {
    self.focused_style(f(Style::new()))
  }

  // --- Event handlers ---

  pub fn on_click(mut self, f: impl Fn(&MouseEvent) + Send + Sync + 'static) -> Self {
    self.events.on_click = Some(Arc::new(f));
    self
  }

  pub fn on_mouse_click(mut self, button: MouseButton, f: impl Fn(&MouseEvent) + Send + Sync + 'static) -> Self {
    self.events.on_mouse_click.push((button, Arc::new(f)));
    self
  }

  pub fn on_dblclick(mut self, f: impl Fn(&MouseEvent) + Send + Sync + 'static) -> Self {
    self.events.on_dblclick = Some(Arc::new(f));
    self
  }

  pub fn on_mouse_down(mut self, f: impl Fn(&MouseEvent) + Send + Sync + 'static) -> Self {
    self.events.on_mouse_down = Some(Arc::new(f));
    self
  }

  pub fn on_mouse_up(mut self, f: impl Fn(&MouseEvent) + Send + Sync + 'static) -> Self {
    self.events.on_mouse_up = Some(Arc::new(f));
    self
  }

  pub fn on_mouse_move(mut self, f: impl Fn(&MouseEvent) + Send + Sync + 'static) -> Self {
    self.events.on_mouse_move = Some(Arc::new(f));
    self
  }

  pub fn on_drag_start(mut self, f: impl Fn(&DragEvent) + Send + Sync + 'static) -> Self {
    self.events.on_drag_start = Some(Arc::new(f));
    self
  }

  pub fn on_drag_move(mut self, f: impl Fn(&DragEvent) + Send + Sync + 'static) -> Self {
    self.events.on_drag_move = Some(Arc::new(f));
    self
  }

  pub fn on_drag_end(mut self, f: impl Fn(&DragEvent) + Send + Sync + 'static) -> Self {
    self.events.on_drag_end = Some(Arc::new(f));
    self
  }

  pub fn on_drop(mut self, f: impl Fn(&DropEvent) + Send + Sync + 'static) -> Self {
    self.events.on_drop = Some(Arc::new(f));
    self
  }

  pub fn on_mouse_enter(mut self, f: impl Fn() + Send + Sync + 'static) -> Self {
    self.events.on_mouse_enter = Some(Arc::new(f));
    self
  }

  pub fn on_mouse_leave(mut self, f: impl Fn() + Send + Sync + 'static) -> Self {
    self.events.on_mouse_leave = Some(Arc::new(f));
    self
  }

  pub fn on_key_down(mut self, f: impl Fn(&KeyboardEvent) + Send + Sync + 'static) -> Self {
    self.events.on_key_down = Some(Arc::new(f));
    self
  }

  pub fn on_key_up(mut self, f: impl Fn(&KeyboardEvent) + Send + Sync + 'static) -> Self {
    self.events.on_key_up = Some(Arc::new(f));
    self
  }

  pub fn on_focus(mut self, f: impl Fn() + Send + Sync + 'static) -> Self {
    self.events.on_focus = Some(Arc::new(f));
    self
  }

  pub fn on_blur(mut self, f: impl Fn() + Send + Sync + 'static) -> Self {
    self.events.on_blur = Some(Arc::new(f));
    self
  }

  pub fn on_scroll(mut self, f: impl Fn(&ScrollEvent) + Send + Sync + 'static) -> Self {
    self.events.on_scroll = Some(Arc::new(f));
    self
  }

  pub fn on_scroll_start(mut self, f: impl Fn(&ScrollEvent) + Send + Sync + 'static) -> Self {
    self.events.on_scroll_start = Some(Arc::new(f));
    self
  }

  pub fn on_scroll_end(mut self, f: impl Fn(&ScrollEvent) + Send + Sync + 'static) -> Self {
    self.events.on_scroll_end = Some(Arc::new(f));
    self
  }

  pub fn opacity(mut self, value: f32) -> Self {
    self.set_opacity(value);
    self
  }

  fn set_opacity(&mut self, value: f32) {
    self.opacity = value;
  }

  pub fn transform(mut self, t: Transform2D) -> Self {
    self.transform = t;
    self
  }

  pub fn transition(mut self, spec: Transition) -> Self {
    self.push_transition(spec);
    self
  }

  fn push_transition(&mut self, spec: Transition) {
    self.transitions.push(spec);
  }

  pub fn animation(mut self, spec: Animation) -> Self {
    self.set_animation(spec);
    self
  }

  fn set_animation(&mut self, spec: Animation) {
    self.animation = Some(spec);
  }

  pub fn scrollbar(mut self, style: ScrollBarStyle) -> Self {
    self.scrollbar_style.set(Some(style));
    self
  }

  pub fn scrollbar_hovered(mut self, f: impl Fn(ScrollBarStyle) -> ScrollBarStyle + Send + Sync + 'static) -> Self {
    self.scrollbar_hovered_style = Some(Arc::new(f));
    self
  }

  pub fn ref_element(mut self, element_ref: impl Into<CoreElementRef>) -> Self {
    self.element_ref = Some(element_ref.into());
    self
  }

  pub(crate) fn element_ref_handle(&mut self) -> CoreElementRef {
    self.element_ref.get_or_insert_with(CoreElementRef::new).clone()
  }

  pub fn interactive(mut self, state: InteractionState) -> Self {
    self.interaction = Some(state);
    self
  }

  pub fn focusable(mut self, focusable: bool) -> Self {
    self.focusable = focusable;
    self
  }

  pub fn tab_index(mut self, tab_index: i32) -> Self {
    self.tab_index = Some(tab_index);
    self
  }

  pub fn button_kind(mut self, kind: ButtonKind) -> Self {
    self.button_kind = Some(kind);
    self.focusable = true;
    self
  }

  #[cfg(feature = "form")]
  pub fn form(mut self, on_submit: impl Fn(FormData) + Send + Sync + 'static) -> Self {
    self.events.on_submit = Some(Arc::new(on_submit));
    self
  }

  #[cfg(feature = "form")]
  pub fn name(mut self, name: impl Into<Arc<str>>) -> Self {
    self.form_name = Some(name.into());
    self
  }

  pub fn text_content(&self) -> Option<&str> {
    self.text_content.as_deref()
  }

  #[cfg_attr(not(feature = "form"), allow(dead_code))]
  pub(crate) fn is_focusable(&self) -> bool {
    self.focusable
  }

  #[cfg_attr(not(feature = "form"), allow(dead_code))]
  pub(crate) fn tab_index_value(&self) -> Option<i32> {
    self.tab_index
  }

  pub(crate) fn button_kind_value(&self) -> Option<ButtonKind> {
    self.button_kind
  }

  #[cfg(feature = "form")]
  pub(crate) fn submit_handler(&self) -> Option<FormSubmitCallback> {
    self.events.on_submit.clone()
  }

  #[cfg(feature = "form")]
  pub(crate) fn form_name_value(&self) -> Option<&str> {
    self.form_name.as_deref()
  }

  pub fn selectable(self, selectable: bool) -> Self {
    if let NodeKind::Text { state, .. } = &self.node_kind {
      state.set_selectable(selectable);
    }
    self
  }

  pub fn text_transform_mode(mut self, mode: TextTransformMode) -> Self {
    if let NodeKind::Text { transform_mode, .. } = &mut self.node_kind {
      *transform_mode = mode;
    }
    self
  }

  pub fn text_variant(mut self, typography_style: impl Into<TypographyStyle>) -> Self {
    if let NodeKind::Text { style, .. } = &mut self.node_kind {
      style.set_variant(typography_style);
      self.layout_cache.invalidate();
    }
    self
  }

  pub fn text_color(mut self, color: impl Into<TextColor>) -> Self {
    if let NodeKind::Text { style, .. } = &mut self.node_kind {
      style.set_color(color);
    }
    self
  }

  pub fn text_align(mut self, align: impl Into<TextAlign>) -> Self {
    if let NodeKind::Text { style, .. } = &mut self.node_kind {
      style.set_text_align(align);
      self.layout_cache.invalidate();
    }
    self
  }

  pub fn placeholder(mut self, placeholder: &str) -> Self {
    self.set_placeholder(placeholder);
    self
  }

  fn set_placeholder(&mut self, placeholder: &str) {
    if let NodeKind::TextInput { state, .. } = &self.node_kind {
      state.set_placeholder(placeholder);
      if state.value().is_empty() {
        self.text_content.set(Some(placeholder.to_owned()));
      }
    }
  }

  pub fn text_input_overflow(mut self, overflow: crate::node::node_kind::TextInputOverflow) -> Self {
    self.set_text_input_overflow(overflow);
    self
  }

  fn set_text_input_overflow(&mut self, overflow: crate::node::node_kind::TextInputOverflow) {
    if let NodeKind::TextInput { state, .. } = &self.node_kind {
      state.set_overflow(overflow);
    }
  }

  pub fn text_input_mask(self) -> Self {
    self.text_input_mask_char('*')
  }

  pub fn text_input_mask_char(self, mask: char) -> Self {
    if let NodeKind::TextInput { state, .. } = &self.node_kind {
      state.set_mask(Some(mask));
    }
    self
  }

  pub fn text_input_unmask(self) -> Self {
    if let NodeKind::TextInput { state, .. } = &self.node_kind {
      state.set_mask(None);
    }
    self
  }

  pub fn text_input_style(mut self, text_style: TextStyle) -> Self {
    self.set_text_input_style(text_style);
    self
  }

  fn set_text_input_style(&mut self, text_style: TextStyle) {
    if let NodeKind::TextInput { style, .. } = &mut self.node_kind {
      *style = text_style;
      self.layout_cache.invalidate();
    }
  }

  pub fn text_input_placeholder_style(mut self, text_style: TextStyle) -> Self {
    self.set_text_input_placeholder_style(text_style);
    self
  }

  fn set_text_input_placeholder_style(&mut self, mut text_style: TextStyle) {
    if let NodeKind::TextInput {
      style,
      placeholder_style,
      ..
    } = &mut self.node_kind
    {
      text_style.text_align = style.text_align;
      *placeholder_style = Some(text_style);
      self.layout_cache.invalidate();
    }
  }

  pub fn text_input_align(mut self, align: impl Into<TextAlign>) -> Self {
    self.set_text_input_align(align);
    self
  }

  fn set_text_input_align(&mut self, align: impl Into<TextAlign>) {
    if let NodeKind::TextInput {
      style,
      placeholder_style,
      ..
    } = &mut self.node_kind
    {
      let align = align.into();
      style.text_align = align;
      if let Some(placeholder_style) = placeholder_style {
        placeholder_style.text_align = align;
      }
      self.layout_cache.invalidate();
    }
  }

  pub fn text_input_rows(mut self, min_rows: usize, max_rows: usize) -> Self {
    self.set_text_input_rows(min_rows, max_rows);
    self
  }

  fn set_text_input_rows(&mut self, min_rows: usize, max_rows: usize) {
    if let NodeKind::TextInput { state, .. } = &self.node_kind {
      state.set_rows(min_rows, max_rows);
    }
  }

  pub fn text_input_min_rows(mut self, min_rows: usize) -> Self {
    self.set_text_input_min_rows(min_rows);
    self
  }

  fn set_text_input_min_rows(&mut self, min_rows: usize) {
    if let NodeKind::TextInput { state, .. } = &self.node_kind {
      state.set_min_rows(min_rows);
    }
  }

  pub fn text_input_max_rows(mut self, max_rows: usize) -> Self {
    self.set_text_input_max_rows(max_rows);
    self
  }

  fn set_text_input_max_rows(&mut self, max_rows: usize) {
    if let NodeKind::TextInput { state, .. } = &self.node_kind {
      state.set_max_rows(max_rows);
    }
  }

  pub fn text_input_rows_exact(mut self, rows: usize) -> Self {
    self.set_text_input_rows_exact(rows);
    self
  }

  fn set_text_input_rows_exact(&mut self, rows: usize) {
    if let NodeKind::TextInput { state, .. } = &self.node_kind {
      state.set_rows_exact(rows);
    }
  }

  pub fn range(self, min: i32, max: i32) -> Self {
    if let Some(state) = self.slider_state() {
      state.set_range(min, max);
    }
    self
  }

  pub fn range_f32(self, min: f32, max: f32) -> Self {
    if let Some(state) = self.slider_state() {
      state.set_range_f32(min, max);
    }
    self
  }

  pub fn slider_step(self, step: f32) -> Self {
    if let Some(state) = self.slider_state() {
      state.set_step(step);
    }
    self
  }

  pub fn slider_track_style(self, style: SliderPartStyle) -> Self {
    if let Some(state) = self.slider_state() {
      state.set_track_style(style);
    }
    self
  }

  pub fn slider_track_hovered_style(self, style: SliderPartStyle) -> Self {
    if let Some(state) = self.slider_state() {
      state.set_track_hovered_style(style);
    }
    self
  }

  pub fn slider_thumb_style(self, style: SliderPartStyle) -> Self {
    if let Some(state) = self.slider_state() {
      state.set_thumb_style(style);
    }
    self
  }

  pub fn slider_thumb_hovered_style(self, style: SliderPartStyle) -> Self {
    if let Some(state) = self.slider_state() {
      state.set_thumb_hovered_style(style);
    }
    self
  }

  pub(crate) fn checkbox_state(&self) -> Option<&CheckboxState> {
    if let NodeKind::Checkbox { state } = &self.node_kind {
      return Some(state);
    }
    if self.children.len() == 1 {
      return self.children[0].checkbox_state();
    }
    None
  }

  pub(crate) fn slider_state(&self) -> Option<&SliderState> {
    if let NodeKind::Slider { state } = &self.node_kind {
      return Some(state);
    }
    if self.children.len() == 1 {
      return self.children[0].slider_state();
    }
    None
  }

  pub fn clip(mut self) -> Self {
    self.set_overflow_through_logical(Overflow::Hidden);
    self
  }

  pub fn overflow_visible(mut self) -> Self {
    self.set_overflow_through_logical(Overflow::Visible);
    self
  }

  fn set_overflow_through_logical(&mut self, overflow: Overflow) {
    self.overflow = overflow;
    if matches!(&self.layout_kind, LayoutKind::LogicalModifier) && self.children.len() == 1 {
      self.children[0].set_overflow_through_logical(overflow);
    }
  }

  pub fn intrinsic(mut self, width: f32, height: f32) -> Self {
    self.intrinsic_size = Some(Size::new(width, height));
    self
  }

  pub fn assign_ids(&mut self, id_gen: &IdGenerator) {
    if !self.node_id.is_assigned() {
      self.node_id = id_gen.next();
    }
    for child in &mut self.children {
      child.assign_ids(id_gen);
    }
  }

  pub fn free_ids(&mut self, id_gen: &IdGenerator) {
    if self.node_id.is_assigned() {
      id_gen.free(self.node_id);
      self.node_id = NodeId::UNASSIGNED;
    }
    for child in &mut self.children {
      child.free_ids(id_gen);
    }
  }

  // --- Accessors ---

  pub fn node_id(&self) -> NodeId {
    self.node_id
  }

  pub fn tag_name(&self) -> &str {
    &self.tag_name
  }

  pub(crate) fn with_tag_name(mut self, tag_name: impl Into<Arc<str>>) -> Self {
    self.tag_name = tag_name.into();
    self
  }

  pub(crate) fn set_tag_name(&mut self, tag_name: impl Into<Arc<str>>) {
    self.tag_name = tag_name.into();
  }

  pub(crate) fn layout_kind(&self) -> &LayoutKind {
    &self.layout_kind
  }

  pub(crate) fn component_slot_id(&self) -> Option<u64> {
    self.component_slot_id
  }

  pub(crate) fn set_component_slot_id(&mut self, id: u64) {
    self.component_slot_id = Some(id);
  }

  #[allow(dead_code)]
  pub(crate) fn component_key(&self) -> Option<&str> {
    self.component_key.as_deref()
  }

  pub(crate) fn set_component_key(&mut self, key: Option<&str>) {
    self.component_key = key.map(Arc::from);
  }

  #[cfg(feature = "devtools")]
  pub(crate) fn set_component_props_debug(&mut self, props: Option<DevtoolsInspectableDebug>) {
    self.component_props_debug = props;
  }

  #[cfg(feature = "devtools")]
  pub(crate) fn set_component_signals_debug(&mut self, signals: Vec<ComponentSignalDebug>) {
    self.component_signals_debug = signals;
  }

  #[cfg(feature = "devtools")]
  pub(crate) fn set_component_memos_debug(&mut self, memos: Vec<ComponentMemoDebug>) {
    self.component_memos_debug = memos;
  }

  #[cfg(feature = "devtools")]
  pub(crate) fn set_component_effects_debug(&mut self, effects: Vec<ComponentEffectDebug>) {
    self.component_effects_debug = effects;
  }

  #[cfg(feature = "devtools")]
  pub(crate) fn set_component_contexts_debug(&mut self, contexts: Vec<ComponentContextDebug>) {
    self.component_contexts_debug = contexts;
  }

  #[cfg(feature = "devtools")]
  #[cfg_attr(not(feature = "router"), allow(dead_code))]
  pub(crate) fn debug_attr(mut self, name: impl Into<Arc<str>>, value: impl Into<Arc<str>>) -> Self {
    self.debug_attrs.push((name.into(), value.into()));
    self
  }

  #[cfg(feature = "devtools")]
  #[allow(dead_code)]
  pub fn component_props_debug(&self) -> Option<&DevtoolsInspectableDebug> {
    self.component_props_debug.as_ref()
  }

  #[cfg(feature = "devtools")]
  #[allow(dead_code)]
  pub fn component_signals_debug(&self) -> &[ComponentSignalDebug] {
    &self.component_signals_debug
  }

  #[cfg(feature = "devtools")]
  #[allow(dead_code)]
  pub fn component_memos_debug(&self) -> &[ComponentMemoDebug] {
    &self.component_memos_debug
  }

  #[cfg(feature = "devtools")]
  #[allow(dead_code)]
  pub fn component_effects_debug(&self) -> &[ComponentEffectDebug] {
    &self.component_effects_debug
  }

  #[cfg(feature = "devtools")]
  #[allow(dead_code)]
  pub fn component_contexts_debug(&self) -> &[ComponentContextDebug] {
    &self.component_contexts_debug
  }

  #[cfg(feature = "devtools")]
  #[allow(dead_code)]
  pub fn debug_attrs(&self) -> &[(Arc<str>, Arc<str>)] {
    &self.debug_attrs
  }

  pub(crate) fn node_kind(&self) -> &NodeKind {
    &self.node_kind
  }

  pub fn with_scroll_state(mut self, existing: crate::layout::layout_kind::ScrollState) -> Self {
    if let LayoutKind::ScrollModifier { state, .. } = &mut self.layout_kind {
      *state = existing;
    }
    self
  }

  pub fn scrollbar_style(&self, default_style: ScrollBarStyle) -> ScrollBarStyle {
    let mut style = (*self.scrollbar_style).clone().unwrap_or(default_style);
    if let LayoutKind::ScrollModifier { state, .. } = &self.layout_kind {
      if state.is_thumb_hovered() {
        if let Some(ref hovered) = self.scrollbar_hovered_style {
          style = hovered(style);
        }
      }
    }
    style
  }

  pub fn color(&self) -> Option<Color> {
    self.background_color().and_then(BackgroundColor::as_color)
  }

  pub(crate) fn caret_color_value(&self) -> Option<TextColor> {
    <Option<TextColor> as Clone>::clone(&self.caret_color)
  }

  pub(crate) fn caret_mode_value(&self) -> Option<CaretMode> {
    *self.caret_mode
  }

  pub(crate) fn background_color(&self) -> Option<BackgroundColor> {
    if let Some(c) = self.animation_override_color() {
      return Some(BackgroundColor::Color(c));
    }
    self
      .state_style()
      .color
      .or_else(|| <Option<BackgroundColor> as Clone>::clone(&self.color))
  }

  pub(crate) fn resolved_color(&self, palette: &crate::app::theme::ThemePalette) -> Option<Color> {
    if let Some(c) = self.animation_override_color() {
      return Some(c);
    }
    self
      .state_style()
      .color
      .or_else(|| <Option<BackgroundColor> as Clone>::clone(&self.color))
      .and_then(|color| color.resolve(palette))
  }

  pub(crate) fn resolved_gradient(&self) -> Option<Gradient> {
    <Option<Gradient> as Clone>::clone(&self.gradient)
  }

  pub fn get_border_radius(&self, radii: &crate::app::theme::ThemeRadii) -> Option<BorderRadius> {
    let mut r = self
      .state_style()
      .border_radius
      .or(*self.border_radius)
      .map(|radius| radius.resolve(radii));
    if let Some(mut br) = r {
      let overrides = &self.animation_overrides;
      for (prop, val) in overrides {
        if let crate::animation::AnimatableValue::Float(v) = val {
          match prop {
            crate::animation::AnimatableProperty::BorderRadiusTopLeft => br.top_left = *v,
            crate::animation::AnimatableProperty::BorderRadiusTopRight => br.top_right = *v,
            crate::animation::AnimatableProperty::BorderRadiusBottomRight => br.bottom_right = *v,
            crate::animation::AnimatableProperty::BorderRadiusBottomLeft => br.bottom_left = *v,
            _ => {}
          }
        }
      }
      r = Some(br);
    }
    r
  }

  pub fn get_border(&self) -> Option<Borders> {
    let mut b = self
      .state_style()
      .border
      .or_else(|| <Option<Borders> as Clone>::clone(&self.border));
    let overrides = &self.animation_overrides;
    if let Some(ref mut borders) = b {
      for (prop, val) in overrides {
        match (prop, val) {
          (crate::animation::AnimatableProperty::BorderColor, crate::animation::AnimatableValue::Color(c)) => {
            borders.set_color(*c);
          }
          (crate::animation::AnimatableProperty::BorderWidthTop, crate::animation::AnimatableValue::Float(v)) => {
            if let Some(border) = &mut borders.top {
              border.width = BorderSizeValue::Px(*v);
            }
          }
          (crate::animation::AnimatableProperty::BorderWidthRight, crate::animation::AnimatableValue::Float(v)) => {
            if let Some(border) = &mut borders.right {
              border.width = BorderSizeValue::Px(*v);
            }
          }
          (crate::animation::AnimatableProperty::BorderWidthBottom, crate::animation::AnimatableValue::Float(v)) => {
            if let Some(border) = &mut borders.bottom {
              border.width = BorderSizeValue::Px(*v);
            }
          }
          (crate::animation::AnimatableProperty::BorderWidthLeft, crate::animation::AnimatableValue::Float(v)) => {
            if let Some(border) = &mut borders.left {
              border.width = BorderSizeValue::Px(*v);
            }
          }
          _ => {}
        }
      }
    }
    b.filter(Borders::any)
  }

  pub(crate) fn get_resolved_border(
    &self,
    palette: &crate::app::theme::ThemePalette,
    border_sizes: &crate::app::theme::ThemeBorderSizes,
  ) -> Option<crate::node::border::ResolvedBorders> {
    self
      .get_border()
      .and_then(|borders| borders.resolve_with_sizes(palette, border_sizes))
  }

  pub(crate) fn effective_transform(&self) -> Transform2D {
    self
      .animation_overrides
      .iter()
      .find_map(|(prop, val)| match (prop, val) {
        (crate::animation::AnimatableProperty::Transform, crate::animation::AnimatableValue::Transform(d)) => {
          Some(d.to_matrix())
        }
        _ => None,
      })
      .unwrap_or(self.transform)
  }

  fn animation_override_color(&self) -> Option<Color> {
    self
      .animation_overrides
      .iter()
      .find_map(|(prop, val)| match (prop, val) {
        (crate::animation::AnimatableProperty::BackgroundColor, crate::animation::AnimatableValue::Color(c)) => {
          Some(*c)
        }
        _ => None,
      })
  }

  pub fn children(&self) -> &[Node] {
    &self.children
  }

  pub(crate) fn element_override_rect(&self) -> Option<crate::core::ElementRect> {
    self.element_ref.as_ref().and_then(CoreElementRef::override_rect)
  }

  pub(crate) fn state_styles_affect_layout(&self) -> bool {
    self.state_styles.affects_layout()
  }

  pub(crate) fn take_style_layout_dirty(&self) -> bool {
    self.style_state.take_layout_dirty()
  }

  pub(crate) fn is_style_hovered(&self) -> bool {
    self.style_state.is_hovered()
  }

  pub(crate) fn set_style_hovered(&self, hovered: bool) -> bool {
    let changed = self.style_state.is_hovered() != hovered;
    if changed {
      self.style_state.set_hovered(hovered);
    }
    let layout_dirty = changed && self.state_styles_affect_layout();
    if layout_dirty {
      self.style_state.mark_layout_dirty();
    }
    layout_dirty
  }

  pub(crate) fn set_style_active(&self, active: bool) -> bool {
    let changed = self.style_state.is_active() != active;
    if changed {
      self.style_state.set_active(active);
    }
    let layout_dirty = changed && self.state_styles_affect_layout();
    if layout_dirty {
      self.style_state.mark_layout_dirty();
    }
    layout_dirty
  }

  pub(crate) fn set_style_focused(&self, focused: bool) -> bool {
    let changed = self.style_state.is_focused() != focused;
    if changed {
      self.style_state.set_focused(focused);
    }
    let layout_dirty = changed && self.state_styles_affect_layout();
    if layout_dirty {
      self.style_state.mark_layout_dirty();
    }
    layout_dirty
  }

  pub(crate) fn effective_frame(&self, base: FrameConstraints) -> FrameConstraints {
    let base = merge_frame(self.frame, base);
    let mut result = self.state_style().frame.map_or(base, |frame| merge_frame(base, frame));
    for (prop, val) in &self.animation_overrides {
      if let crate::animation::AnimatableValue::Float(v) = val {
        match prop {
          crate::animation::AnimatableProperty::Width => {
            result.width = Some(Dimension::Px(*v));
          }
          crate::animation::AnimatableProperty::Height => {
            result.height = Some(Dimension::Px(*v));
          }
          _ => {}
        }
      }
    }
    result
  }

  pub(crate) fn state_frame(&self) -> Option<FrameConstraints> {
    let frame = self.effective_frame(FrameConstraints::default());
    (frame != FrameConstraints::default()).then_some(frame)
  }

  pub(crate) fn effective_padding(&self, base: &Padding) -> Padding {
    let mut padding = self.padding;
    padding.merge_from(base);
    self.state_style().padding.unwrap_or(padding)
  }

  pub(crate) fn state_flex(&self) -> Option<FlexParams> {
    self
      .state_style()
      .flex
      .or(self.flex)
      .or_else(|| match &self.layout_kind {
        LayoutKind::LogicalModifier => self.children.first().and_then(|child| child.state_flex()),
        _ => None,
      })
  }

  pub(crate) fn align_self(&self) -> Option<Alignment> {
    self.align_self
  }

  pub(crate) fn position(&self) -> Position {
    self.position
  }

  pub(crate) fn offset_position(&self) -> Option<Offset> {
    self.offset
  }

  pub(crate) fn min_main_size(&self, vertical: bool) -> f32 {
    if let Some(frame) = self.state_frame() {
      let size = if vertical { frame.min_height } else { frame.min_width };
      if let Some(size) = size {
        return size.to_px();
      }
    }

    match &self.layout_kind {
      LayoutKind::LogicalModifier => self.children.first().map(|c| c.min_main_size(vertical)).unwrap_or(0.0),
      _ => 0.0,
    }
  }

  pub(crate) fn clear_guards(&self) {
    self.text_content.clear_changed();
    self.color.clear_changed();
    self.border_radius.clear_changed();
    self.border.clear_changed();
    #[cfg(feature = "image")]
    self.background_image.clear_changed();
    self.scrollbar_style.clear_changed();
    for child in &self.children {
      child.clear_guards();
    }
  }

  pub(crate) fn sync_dynamic_content_recursive(&mut self) {
    if let NodeKind::TextInput { state, .. } = &self.node_kind {
      let rendered = state.rendered_text();
      if self.text_content.as_ref() != rendered.as_ref() {
        self.text_content.set(rendered);
      }
    }
    for child in &mut self.children {
      child.sync_dynamic_content_recursive();
    }
  }

  pub(crate) fn preserve_runtime_state_from(&mut self, old: &Node) {
    self.clear_unchanged_guard_flags_from(old);
    let layout_signature_matches = self.layout_signature_matches(old);
    if layout_signature_matches {
      self.layout_cache.preserve_from(&old.layout_cache);
    }

    match (&self.node_kind, &old.node_kind) {
      (NodeKind::Text { state, .. }, NodeKind::Text { state: old_state, .. }) => {
        state.copy_runtime_state_from(
          old_state,
          self.text_content().unwrap_or_default(),
          layout_signature_matches,
        );
      }
      (NodeKind::TextInput { state, .. }, NodeKind::TextInput { state: old_state, .. }) => {
        state.copy_runtime_state_from(old_state);
      }
      (NodeKind::Select { state, .. }, NodeKind::Select { state: old_state, .. }) => {
        state.copy_runtime_state_from(old_state);
        self.element_ref = old.element_ref.clone();
      }
      (NodeKind::Slider { state }, NodeKind::Slider { state: old_state }) => {
        state.copy_runtime_state_from(old_state);
      }
      _ => {}
    }

    if let (
      LayoutKind::ScrollModifier { state, direction },
      LayoutKind::ScrollModifier {
        state: old_state,
        direction: old_direction,
      },
    ) = (&mut self.layout_kind, &old.layout_kind)
    {
      if direction == old_direction {
        *state = old_state.clone();
      }
    }

    for (child, old_child) in self.children.iter_mut().zip(old.children.iter()) {
      child.preserve_runtime_state_from(old_child);
    }
  }

  fn clear_unchanged_guard_flags_from(&self, old: &Node) {
    if self.text_content.as_ref() == old.text_content.as_ref() {
      self.text_content.clear_changed();
    }
    if self.color.as_ref() == old.color.as_ref() {
      self.color.clear_changed();
    }
    if self.border_radius.as_ref() == old.border_radius.as_ref() {
      self.border_radius.clear_changed();
    }
    if self.border.as_ref() == old.border.as_ref() {
      self.border.clear_changed();
    }
  }

  fn layout_signature_matches(&self, old: &Node) -> bool {
    self.layout_kind_matches_for_cache(old)
      && self.node_kind_matches_for_cache(old)
      && self.frame == old.frame
      && self.padding == old.padding
      && self.position == old.position
      && self.offset == old.offset
      && self.align_self == old.align_self
      && self.flex == old.flex
      && self.text_wrap == old.text_wrap
      && self.text_overflow == old.text_overflow
      && self.overflow == old.overflow
      && self.intrinsic_size == old.intrinsic_size
      && self.animation_overrides.is_empty()
      && old.animation_overrides.is_empty()
      && self.children.len() == old.children.len()
      && self
        .children
        .iter()
        .zip(old.children.iter())
        .all(|(child, old_child)| child.layout_signature_matches(old_child))
  }

  fn layout_kind_matches_for_cache(&self, old: &Node) -> bool {
    match (&self.layout_kind, &old.layout_kind) {
      (LayoutKind::Leaf, LayoutKind::Leaf) => true,
      (
        LayoutKind::Row {
          spacing,
          align,
          justify,
          wrap,
        },
        LayoutKind::Row {
          spacing: old_spacing,
          align: old_align,
          justify: old_justify,
          wrap: old_wrap,
        },
      )
      | (
        LayoutKind::Column {
          spacing,
          align,
          justify,
          wrap,
        },
        LayoutKind::Column {
          spacing: old_spacing,
          align: old_align,
          justify: old_justify,
          wrap: old_wrap,
        },
      ) => spacing == old_spacing && align == old_align && justify == old_justify && wrap == old_wrap,
      (LayoutKind::Stack { align }, LayoutKind::Stack { align: old_align }) => align == old_align,
      (LayoutKind::LogicalModifier, LayoutKind::LogicalModifier) => true,
      (
        LayoutKind::ScrollModifier { direction, .. },
        LayoutKind::ScrollModifier {
          direction: old_direction,
          ..
        },
      ) => direction == old_direction,
      _ => false,
    }
  }

  fn node_kind_matches_for_cache(&self, old: &Node) -> bool {
    match (&self.node_kind, &old.node_kind) {
      (NodeKind::Empty, NodeKind::Empty) => true,
      (
        NodeKind::Text {
          state,
          style,
          transform_mode,
        },
        NodeKind::Text {
          state: old_state,
          style: old_style,
          transform_mode: old_transform_mode,
        },
      ) => style == old_style && state.selectable() == old_state.selectable() && transform_mode == old_transform_mode,
      (
        NodeKind::TextInput {
          style,
          placeholder_style,
          ..
        },
        NodeKind::TextInput {
          style: old_style,
          placeholder_style: old_placeholder_style,
          ..
        },
      ) => style == old_style && placeholder_style == old_placeholder_style,
      (NodeKind::Checkbox { state }, NodeKind::Checkbox { state: old_state }) => {
        state.layout_signature() == old_state.layout_signature()
      }
      (NodeKind::Slider { state }, NodeKind::Slider { state: old_state }) => {
        state.layout_signature() == old_state.layout_signature()
      }
      #[cfg(feature = "image")]
      (NodeKind::Image { data }, NodeKind::Image { data: old_data }) => data.id() == old_data.id(),
      #[cfg(feature = "image")]
      (NodeKind::ResourceImage { path }, NodeKind::ResourceImage { path: old_path }) => path == old_path,
      #[cfg(feature = "svg")]
      (NodeKind::Svg { data }, NodeKind::Svg { data: old_data }) => data.id() == old_data.id(),
      #[cfg(all(feature = "svg", feature = "resources"))]
      (NodeKind::ResourceSvg { path }, NodeKind::ResourceSvg { path: old_path }) => path == old_path,
      _ => false,
    }
  }

  pub(crate) fn preserve_ids_from(&mut self, old: &mut Node) {
    if self.can_reuse_id_from(old) && old.node_id.is_assigned() {
      self.node_id = old.node_id;
      old.node_id = NodeId::UNASSIGNED;
    }

    for (child, old_child) in self.children.iter_mut().zip(old.children.iter_mut()) {
      child.preserve_ids_from(old_child);
    }
  }

  fn can_reuse_id_from(&self, old: &Node) -> bool {
    self.component_slot_id == old.component_slot_id
      && std::mem::discriminant(&self.node_kind) == std::mem::discriminant(&old.node_kind)
      && std::mem::discriminant(&self.layout_kind) == std::mem::discriminant(&old.layout_kind)
  }

  pub(crate) fn clone_for_reuse(&self) -> Self {
    Self {
      node_id: NodeId::UNASSIGNED,
      tag_name: self.tag_name.clone(),
      component_slot_id: self.component_slot_id,
      component_key: self.component_key.clone(),
      #[cfg(feature = "devtools")]
      component_props_debug: self.component_props_debug.clone(),
      #[cfg(feature = "devtools")]
      component_signals_debug: self.component_signals_debug.clone(),
      #[cfg(feature = "devtools")]
      component_memos_debug: self.component_memos_debug.clone(),
      #[cfg(feature = "devtools")]
      component_effects_debug: self.component_effects_debug.clone(),
      #[cfg(feature = "devtools")]
      component_contexts_debug: self.component_contexts_debug.clone(),
      #[cfg(feature = "devtools")]
      debug_attrs: self.debug_attrs.clone(),
      layout_kind: self.layout_kind.clone(),
      frame: self.frame,
      padding: self.padding,
      position: self.position,
      offset: self.offset,
      align_self: self.align_self,
      flex: self.flex,
      node_kind: self.node_kind.clone(),
      text_content: self.text_content.clone(),
      text_wrap: self.text_wrap,
      text_overflow: self.text_overflow,
      overflow: self.overflow,
      intrinsic_size: self.intrinsic_size,
      color: self.color.clone(),
      gradient: self.gradient.clone(),
      border_radius: self.border_radius.clone(),
      border: self.border.clone(),
      caret_color: self.caret_color.clone(),
      caret_mode: self.caret_mode.clone(),
      cursor: self.cursor,
      #[cfg(feature = "image")]
      background_image: self.background_image.clone(),
      #[cfg(feature = "image")]
      background_size: self.background_size,
      #[cfg(all(feature = "image", feature = "resources"))]
      background_resource_image: self.background_resource_image.clone(),
      scrollbar_style: self.scrollbar_style.clone(),
      scrollbar_hovered_style: self.scrollbar_hovered_style.clone(),
      element_ref: self.element_ref.clone(),
      interaction: self.interaction.clone(),
      focusable: self.focusable,
      tab_index: self.tab_index,
      button_kind: self.button_kind,
      #[cfg(feature = "form")]
      form_name: self.form_name.clone(),
      style_state: self.style_state.clone(),
      state_styles: self.state_styles.clone(),
      opacity: self.opacity,
      transform: self.transform,
      animation_overrides: Vec::new(),
      transitions: self.transitions.clone(),
      animation: self.animation.clone(),
      layout_cache: Default::default(),
      children: self.children.iter().map(Node::clone_for_reuse).collect(),
      events: self.events.clone(),
    }
  }

  pub(crate) fn replace_component_slot(&mut self, slot_id: u64, replacement: Node) -> bool {
    let mut replacement = Some(replacement);
    self.replace_component_slot_in(slot_id, &mut replacement)
  }

  pub(crate) fn replace_component_slot_in(&mut self, slot_id: u64, replacement: &mut Option<Node>) -> bool {
    if self.component_slot_id == Some(slot_id) {
      *self = replacement
        .take()
        .expect("component replacement should be available when matching slot is found");
      return true;
    }

    for child in &mut self.children {
      if child.replace_component_slot_in(slot_id, replacement) {
        return true;
      }
    }

    false
  }

  pub(crate) fn estimated_memory_bytes(&self) -> usize {
    std::mem::size_of::<Self>()
      + self.tag_name.len()
      + self.estimated_debug_memory_bytes()
      + self.text_content.as_ref().map(|text| text.capacity()).unwrap_or(0)
      + self.children.capacity() * std::mem::size_of::<Node>()
      + self.layout_cache.estimated_memory_bytes()
      + self
        .children
        .iter()
        .map(Node::estimated_child_heap_bytes)
        .sum::<usize>()
  }

  #[cfg(feature = "devtools")]
  fn estimated_debug_memory_bytes(&self) -> usize {
    self
      .component_props_debug
      .as_ref()
      .map(|props| {
        props.type_name.len()
          + props.fields.capacity() * std::mem::size_of::<crate::app::component::ComponentInfo>()
          + props
            .fields
            .iter()
            .map(|field| field.estimated_memory_bytes())
            .sum::<usize>()
      })
      .unwrap_or(0)
      + self
        .component_signals_debug
        .iter()
        .map(|signal| signal.type_name.len() + signal.estimated_memory_bytes())
        .sum::<usize>()
      + self
        .component_memos_debug
        .iter()
        .map(|memo| memo.type_name.len() + memo.estimated_memory_bytes())
        .sum::<usize>()
      + self.component_effects_debug.capacity() * std::mem::size_of::<ComponentEffectDebug>()
      + self
        .component_contexts_debug
        .iter()
        .map(|context| context.type_name.len())
        .sum::<usize>()
      + self
        .debug_attrs
        .iter()
        .map(|(name, value)| name.len() + value.len())
        .sum::<usize>()
      + self.debug_attrs.capacity() * std::mem::size_of::<(Arc<str>, Arc<str>)>()
  }

  #[cfg(not(feature = "devtools"))]
  fn estimated_debug_memory_bytes(&self) -> usize {
    0
  }

  fn estimated_child_heap_bytes(&self) -> usize {
    self.tag_name.len()
      + self.estimated_debug_memory_bytes()
      + self.text_content.as_ref().map(|text| text.capacity()).unwrap_or(0)
      + self.children.capacity() * std::mem::size_of::<Node>()
      + self.layout_cache.estimated_memory_bytes()
      + self
        .children
        .iter()
        .map(Node::estimated_child_heap_bytes)
        .sum::<usize>()
  }

  pub(crate) fn target_style(&self) -> Style {
    self.state_style()
  }

  pub(crate) fn state_style(&self) -> Style {
    let mut style = Style::new();
    if self.style_state.is_focused() {
      if let Some(focused) = &self.state_styles.focused {
        style.merge_from(focused);
      }
    }
    if self.style_state.is_hovered() {
      if let Some(hovered) = &self.state_styles.hovered {
        style.merge_from(hovered);
      }
    }
    if self.style_state.is_active() {
      if let Some(active) = &self.state_styles.active {
        style.merge_from(active);
      }
    }
    style
  }
}

pub(crate) fn merge_frame(mut base: FrameConstraints, overlay: FrameConstraints) -> FrameConstraints {
  if overlay.width.is_some() {
    base.width = overlay.width;
  }
  if overlay.height.is_some() {
    base.height = overlay.height;
  }
  if overlay.min_width.is_some() {
    base.min_width = overlay.min_width;
  }
  if overlay.max_width.is_some() {
    base.max_width = overlay.max_width;
  }
  if overlay.min_height.is_some() {
    base.min_height = overlay.min_height;
  }
  if overlay.max_height.is_some() {
    base.max_height = overlay.max_height;
  }
  base
}
