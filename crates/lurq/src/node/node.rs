use std::sync::Arc;

use crate::{
  animation::{Animation, Transition},
  app::events::{DragEvent, DropEvent, KeyboardEvent, MouseEvent, ScrollEvent},
  core::{ElementRef as CoreElementRef, Guard, IdGenerator, NodeId, Signal},
  layout::{
    Alignment, Size, StackAlignment,
    layout_kind::{FlexParams, FrameConstraints, LayoutKind, Overflow},
    scrollbar::ScrollBarStyle,
    text_style::TextStyle,
  },
  node::{
    border::{Border, BorderRadius, Borders},
    color::Color,
    cursor::CursorIcon,
    dimension::Dimension,
    interaction_state::InteractionState,
    node_kind::{CheckboxState, NodeKind, SliderState, TextInputState},
    padding::Padding,
    style::{StateStyles, Style},
    transform::Transform2D,
  },
};

type Callback<T> = Arc<dyn Fn(&T) + Send + Sync>;
type VoidCallback = Arc<dyn Fn() + Send + Sync>;
type ScrollbarStyleCallback = Arc<dyn Fn(ScrollBarStyle) -> ScrollBarStyle + Send + Sync>;

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
  pub on_scroll: Option<Callback<ScrollEvent>>,
  pub on_scroll_start: Option<Callback<ScrollEvent>>,
  pub on_scroll_end: Option<Callback<ScrollEvent>>,
}

pub(crate) struct Node {
  pub(crate) node_id: NodeId,
  pub(crate) tag_name: Arc<str>,
  pub(crate) component_slot_id: Option<u64>,
  pub(crate) layout_kind: LayoutKind,
  pub(crate) node_kind: NodeKind,
  pub(crate) text_content: Guard<Option<String>>,
  pub(crate) text_wrap: bool,
  pub(crate) overflow: Overflow,
  pub(crate) intrinsic_size: Option<Size>,
  pub(crate) color: Guard<Option<Color>>,
  pub(crate) border_radius: Guard<Option<BorderRadius>>,
  pub(crate) border: Guard<Option<Borders>>,
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
      node_kind,
      node_id: NodeId::UNASSIGNED,
      tag_name: Arc::from("Node"),
      component_slot_id: None,
      text_content: Guard::new(None),
      text_wrap: true,
      overflow: Overflow::Hidden,
      intrinsic_size: None,
      color: Guard::new(None),
      border_radius: Guard::new(None),
      border: Guard::new(None),
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
      style_state: InteractionState::new(),
      state_styles: StateStyles::default(),
      opacity: 1.0,
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

  fn from_modifier(layout_kind: LayoutKind, mut child: Node) -> Self {
    let tag_name = child.tag_name.clone();
    let hoist_visuals = !matches!(layout_kind, LayoutKind::OffsetModifier { .. })
      && !matches!(child.layout_kind, LayoutKind::OffsetModifier { .. });
    let color = hoist_visuals.then(|| (*child.color).clone()).flatten();
    let border = hoist_visuals.then(|| (*child.border).clone()).flatten();
    let border_radius = hoist_visuals.then(|| (*child.border_radius).clone()).flatten();
    if hoist_visuals {
      if color.is_some() {
        child.color.set(None);
      }
      if border.is_some() {
        child.border.set(None);
      }
      if border_radius.is_some() {
        child.border_radius.set(None);
      }
    }
    let mut wrapper = Self::from_parts(layout_kind, NodeKind::Empty, vec![child]).with_tag_name(tag_name);
    if let Some(c) = color {
      wrapper.color.set(Some(c));
    }
    if let Some(b) = border {
      wrapper.border.set(Some(b));
    }
    if let Some(r) = border_radius {
      wrapper.border_radius.set(Some(r));
    }
    wrapper
  }

  pub fn new() -> Self {
    Self::from_parts(LayoutKind::Leaf, NodeKind::Empty, vec![])
  }

  pub fn text(content: &str) -> Self {
    let node = Self::from_parts(
      LayoutKind::Leaf,
      NodeKind::Text {
        style: TextStyle::default(),
      },
      vec![],
    );
    node.with_text_content(content)
  }

  pub fn text_styled(content: &str, style: TextStyle) -> Self {
    let node = Self::from_parts(LayoutKind::Leaf, NodeKind::Text { style }, vec![]);
    node.with_text_content(content)
  }

  pub fn text_input(value: Signal<String>) -> Self {
    let rendered = value.get_untracked();
    let node = Self::from_parts(
      LayoutKind::Leaf,
      NodeKind::TextInput {
        state: TextInputState::new(value),
        style: TextStyle::default(),
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

  pub fn slider(value: Signal<f32>) -> Self {
    Self::from_parts(
      LayoutKind::Leaf,
      NodeKind::Slider {
        state: SliderState::new(value),
      },
      vec![],
    )
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

  pub fn row(spacing: f32, align: Alignment, children: Vec<Node>) -> Self {
    Self::from_parts(
      LayoutKind::Row {
        spacing,
        align,
        justify: crate::layout::layout_kind::Justify::Start,
        wrap: crate::layout::layout_kind::FlexWrap::NoWrap,
      },
      NodeKind::Empty,
      children,
    )
  }

  pub fn column(spacing: f32, align: Alignment, children: Vec<Node>) -> Self {
    Self::from_parts(
      LayoutKind::Column {
        spacing,
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

  pub fn padding(self, padding: impl Into<Padding>) -> Self {
    Self::from_modifier(LayoutKind::PaddingModifier(padding.into()), self)
  }

  pub fn padding_custom(self, padding: Padding) -> Self {
    self.padding(padding)
  }

  pub fn frame(self, frame: FrameConstraints) -> Self {
    Self::from_modifier(LayoutKind::FrameModifier(frame), self)
  }

  pub fn offset(self, x: f32, y: f32) -> Self {
    Self::from_modifier(LayoutKind::OffsetModifier { x, y }, self)
  }

  pub(crate) fn absolute_modifier(self, x: f32, y: f32, width: Option<Dimension>, height: Option<Dimension>) -> Self {
    Self::from_modifier(LayoutKind::AbsoluteModifier { x, y, width, height }, self)
  }

  pub fn align(self, alignment: Alignment) -> Self {
    Self::from_modifier(LayoutKind::AlignModifier(alignment), self)
  }

  pub fn flex(self, factor: f32) -> Self {
    Self::from_modifier(
      LayoutKind::FlexModifier(crate::layout::layout_kind::FlexParams::grow(factor)),
      self,
    )
  }

  pub fn flex_shrink(self, factor: f32) -> Self {
    Self::from_modifier(
      LayoutKind::FlexModifier(crate::layout::layout_kind::FlexParams {
        grow: 0.0,
        shrink: factor,
        basis: None,
      }),
      self,
    )
  }

  pub fn flex_full(self, grow: f32, shrink: f32, basis: Option<f32>) -> Self {
    Self::from_modifier(
      LayoutKind::FlexModifier(crate::layout::layout_kind::FlexParams { grow, shrink, basis }),
      self,
    )
  }

  pub fn background(mut self, color: Color) -> Self {
    self.color.set(Some(color));
    self
  }

  pub fn corner_radius(mut self, radius: f32) -> Self {
    self.border_radius.set(Some(BorderRadius::all(radius)));
    self
  }

  pub fn corner_radius_custom(mut self, radius: BorderRadius) -> Self {
    self.border_radius.set(Some(radius));
    self
  }

  pub fn corner_radius_top_left(mut self, radius: f32) -> Self {
    let mut border_radius = (*self.border_radius).unwrap_or_default();
    border_radius.top_left = radius;
    self.border_radius.set(Some(border_radius));
    self
  }

  pub fn corner_radius_top_right(mut self, radius: f32) -> Self {
    let mut border_radius = (*self.border_radius).unwrap_or_default();
    border_radius.top_right = radius;
    self.border_radius.set(Some(border_radius));
    self
  }

  pub fn corner_radius_bottom_right(mut self, radius: f32) -> Self {
    let mut border_radius = (*self.border_radius).unwrap_or_default();
    border_radius.bottom_right = radius;
    self.border_radius.set(Some(border_radius));
    self
  }

  pub fn corner_radius_bottom_left(mut self, radius: f32) -> Self {
    let mut border_radius = (*self.border_radius).unwrap_or_default();
    border_radius.bottom_left = radius;
    self.border_radius.set(Some(border_radius));
    self
  }

  pub fn rounded(mut self, radius: f32) -> Self {
    self.border_radius.set(Some(BorderRadius::all(radius)));
    self
  }

  pub fn border_inside(mut self, width: f32, color: Color) -> Self {
    self.border.set(Some(Borders::all(Border::inside(width, color))));
    self
  }

  pub fn border_outside(mut self, width: f32, color: Color) -> Self {
    self.border.set(Some(Borders::all(Border::outside(width, color))));
    self
  }

  pub fn border_center(mut self, width: f32, color: Color) -> Self {
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
    let mut borders = (*self.border).unwrap_or_default();
    borders.top = Some(border);
    self.border.set(Some(borders));
    self
  }

  pub fn border_right(mut self, border: Border) -> Self {
    let mut borders = (*self.border).unwrap_or_default();
    borders.right = Some(border);
    self.border.set(Some(borders));
    self
  }

  pub fn border_bottom(mut self, border: Border) -> Self {
    let mut borders = (*self.border).unwrap_or_default();
    borders.bottom = Some(border);
    self.border.set(Some(borders));
    self
  }

  pub fn border_left(mut self, border: Border) -> Self {
    let mut borders = (*self.border).unwrap_or_default();
    borders.left = Some(border);
    self.border.set(Some(borders));
    self
  }

  pub fn cursor(mut self, cursor: CursorIcon) -> Self {
    self.cursor = Some(cursor);
    self
  }

  #[cfg(feature = "image")]
  pub fn background_image(mut self, data: crate::images::ImageData) -> Self {
    self.background_image.set(Some(data));
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

  #[cfg(all(feature = "image", feature = "resources"))]
  pub fn background_image_resource(mut self, path: &str) -> Self {
    self.background_resource_image = Some(path.into());
    self
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

  pub fn text_content(&self) -> Option<&str> {
    self.text_content.as_deref()
  }

  pub fn placeholder(mut self, placeholder: &str) -> Self {
    if let NodeKind::TextInput { state, .. } = &self.node_kind {
      state.set_placeholder(placeholder);
      if state.value().is_empty() {
        self.text_content.set(Some(placeholder.to_owned()));
      }
    }
    self
  }

  pub fn range(self, min: f32, max: f32) -> Self {
    if let NodeKind::Slider { state } = &self.node_kind {
      state.set_range(min, max);
    }
    self
  }

  pub fn clip(mut self) -> Self {
    self.set_overflow_through_modifiers(Overflow::Hidden);
    self
  }

  pub fn overflow_visible(mut self) -> Self {
    self.set_overflow_through_modifiers(Overflow::Visible);
    self
  }

  fn set_overflow_through_modifiers(&mut self, overflow: Overflow) {
    self.overflow = overflow;
    if matches!(
      &self.layout_kind,
      LayoutKind::PaddingModifier(_)
        | LayoutKind::FrameModifier(_)
        | LayoutKind::OffsetModifier { .. }
        | LayoutKind::AbsoluteModifier { .. }
        | LayoutKind::AlignModifier(_)
        | LayoutKind::FlexModifier(_)
    ) && self.children.len() == 1
    {
      self.children[0].set_overflow_through_modifiers(overflow);
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

  pub(crate) fn node_kind(&self) -> &NodeKind {
    &self.node_kind
  }

  pub fn with_scroll_state(mut self, existing: crate::layout::layout_kind::ScrollState) -> Self {
    if let LayoutKind::ScrollModifier { state, .. } = &mut self.layout_kind {
      *state = existing;
    }
    self
  }

  pub fn scrollbar_style(&self) -> ScrollBarStyle {
    let mut style = (*self.scrollbar_style).clone().unwrap_or_default();
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
    if let Some(c) = self.animation_override_color() {
      return Some(c);
    }
    self.state_style().color.or(*self.color)
  }

  pub fn get_border_radius(&self) -> Option<BorderRadius> {
    let mut r = self.state_style().border_radius.or(*self.border_radius);
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
    let mut b = self.state_style().border.or(*self.border);
    let overrides = &self.animation_overrides;
    if let Some(ref mut borders) = b {
      for (prop, val) in overrides {
        match (prop, val) {
          (crate::animation::AnimatableProperty::BorderColor, crate::animation::AnimatableValue::Color(c)) => {
            borders.set_color(*c);
          }
          (crate::animation::AnimatableProperty::BorderWidthTop, crate::animation::AnimatableValue::Float(v)) => {
            if let Some(border) = &mut borders.top {
              border.width = *v;
            }
          }
          (crate::animation::AnimatableProperty::BorderWidthRight, crate::animation::AnimatableValue::Float(v)) => {
            if let Some(border) = &mut borders.right {
              border.width = *v;
            }
          }
          (crate::animation::AnimatableProperty::BorderWidthBottom, crate::animation::AnimatableValue::Float(v)) => {
            if let Some(border) = &mut borders.bottom {
              border.width = *v;
            }
          }
          (crate::animation::AnimatableProperty::BorderWidthLeft, crate::animation::AnimatableValue::Float(v)) => {
            if let Some(border) = &mut borders.left {
              border.width = *v;
            }
          }
          _ => {}
        }
      }
    }
    b.filter(Borders::any)
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

  pub(crate) fn set_style_hovered(&self, hovered: bool) -> bool {
    let changed = self.style_state.is_hovered() != hovered;
    if changed {
      self.style_state.set_hovered(hovered);
    }
    changed && self.state_styles_affect_layout()
  }

  pub(crate) fn set_style_active(&self, active: bool) -> bool {
    let changed = self.style_state.is_active() != active;
    if changed {
      self.style_state.set_active(active);
    }
    changed && self.state_styles_affect_layout()
  }

  pub(crate) fn set_style_focused(&self, focused: bool) -> bool {
    let changed = self.style_state.is_focused() != focused;
    if changed {
      self.style_state.set_focused(focused);
    }
    changed && self.state_styles_affect_layout()
  }

  pub(crate) fn effective_frame(&self, base: FrameConstraints) -> FrameConstraints {
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
    self.state_style().frame
  }

  pub(crate) fn effective_padding(&self, base: &Padding) -> Padding {
    self.state_style().padding.unwrap_or_else(|| base.clone())
  }

  pub(crate) fn effective_flex(&self, base: FlexParams) -> FlexParams {
    self.state_style().flex.unwrap_or(base)
  }

  pub(crate) fn state_flex(&self) -> Option<FlexParams> {
    self.state_style().flex
  }

  pub(crate) fn min_main_size(&self, vertical: bool) -> f32 {
    if let Some(frame) = self.state_frame() {
      let size = if vertical {
        frame.height.or(frame.min_height)
      } else {
        frame.width.or(frame.min_width)
      };
      if let Some(size) = size {
        return size.to_px();
      }
    }

    match &self.layout_kind {
      LayoutKind::FlexModifier(_) | LayoutKind::PaddingModifier(_) | LayoutKind::AlignModifier(_) => {
        self.children.first().map(|c| c.min_main_size(vertical)).unwrap_or(0.0)
      }
      LayoutKind::FrameModifier(frame) => {
        let frame = self.effective_frame(*frame);
        if vertical {
          frame.min_height.map_or(0.0, |size| size.to_px())
        } else {
          frame.min_width.map_or(0.0, |size| size.to_px())
        }
      }
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
    if let (NodeKind::TextInput { state, .. }, NodeKind::TextInput { state: old_state, .. }) =
      (&self.node_kind, &old.node_kind)
    {
      state.copy_runtime_state_from(old_state);
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
      layout_kind: self.layout_kind.clone(),
      node_kind: self.node_kind.clone(),
      text_content: self.text_content.clone(),
      text_wrap: self.text_wrap,
      overflow: self.overflow,
      intrinsic_size: self.intrinsic_size,
      color: self.color.clone(),
      border_radius: self.border_radius.clone(),
      border: self.border.clone(),
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
    if self.component_slot_id == Some(slot_id) {
      *self = replacement;
      return true;
    }

    for child in &mut self.children {
      if child.replace_component_slot(slot_id, replacement.clone_for_reuse()) {
        return true;
      }
    }

    false
  }

  pub(crate) fn estimated_memory_bytes(&self) -> usize {
    std::mem::size_of::<Self>()
      + self.tag_name.len()
      + self.text_content.as_ref().map(|text| text.capacity()).unwrap_or(0)
      + self.children.capacity() * std::mem::size_of::<Node>()
      + self.layout_cache.estimated_memory_bytes()
      + self
        .children
        .iter()
        .map(Node::estimated_child_heap_bytes)
        .sum::<usize>()
  }

  fn estimated_child_heap_bytes(&self) -> usize {
    self.tag_name.len()
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

  fn state_style(&self) -> Style {
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
