use std::sync::Arc;

use crate::{
  app::events::{KeyboardEvent, MouseEvent, ScrollEvent},
  core::{Guard, IdGenerator, NodeId, NodeRef, Signal},
  layout::{
    Alignment, Size, StackAlignment,
    layout_kind::{FrameConstraints, LayoutKind, Overflow},
    scrollbar::ScrollBarStyle,
    text_style::TextStyle,
  },
  node::{
    border::{Border, BorderPlacement, BorderRadius, BorderWidth},
    color::Color,
    interaction_state::InteractionState,
    node_kind::{CheckboxState, NodeKind, SliderState, TextInputState},
    padding::Padding,
  },
};

type Callback<T> = Arc<dyn Fn(&T) + Send + Sync>;
type VoidCallback = Arc<dyn Fn() + Send + Sync>;

#[derive(Default)]
pub struct EventHandlers {
  pub on_click: Option<Callback<MouseEvent>>,
  pub on_dblclick: Option<Callback<MouseEvent>>,
  pub on_mouse_down: Option<Callback<MouseEvent>>,
  pub on_mouse_up: Option<Callback<MouseEvent>>,
  pub on_mouse_move: Option<Callback<MouseEvent>>,
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
  pub(crate) layout_kind: LayoutKind,
  pub(crate) node_kind: NodeKind,
  pub(crate) text_content: Guard<Option<String>>,
  pub(crate) overflow: Overflow,
  pub(crate) intrinsic_size: Option<Size>,
  pub(crate) color: Guard<Option<Color>>,
  pub(crate) border_radius: Guard<Option<BorderRadius>>,
  pub(crate) border: Guard<Option<Border>>,
  pub(crate) scrollbar_style: Guard<Option<ScrollBarStyle>>,
  pub(crate) node_ref: Option<NodeRef>,
  pub(crate) interaction: Option<InteractionState>,
  pub(crate) layout_cache: crate::node::layout_cache::LayoutCache,
  pub(crate) runtime_rect: Option<RuntimeRect>,
  pub(crate) children: Vec<Node>,
  pub(crate) events: EventHandlers,
}

#[derive(Clone, Copy)]
pub(crate) struct RuntimeRect {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
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
      text_content: Guard::new(None),
      overflow: Overflow::Visible,
      intrinsic_size: None,
      color: Guard::new(None),
      border_radius: Guard::new(None),
      border: Guard::new(None),
      scrollbar_style: Guard::new(None),
      node_ref: None,
      interaction: None,
      layout_cache: Default::default(),
      runtime_rect: None,
      children,
      events: EventHandlers::default(),
    }
  }

  fn with_text_content(mut self, content: &str) -> Self {
    self.text_content.set(Some(content.to_owned()));
    self
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

  pub fn padding(self, padding: Padding) -> Self {
    Self::from_parts(LayoutKind::PaddingModifier(padding), NodeKind::Empty, vec![self])
  }

  pub fn frame(self, frame: FrameConstraints) -> Self {
    Self::from_parts(LayoutKind::FrameModifier(frame), NodeKind::Empty, vec![self])
  }

  pub fn offset(self, x: f32, y: f32) -> Self {
    Self::from_parts(LayoutKind::OffsetModifier { x, y }, NodeKind::Empty, vec![self])
  }

  pub(crate) fn absolute_modifier(self, x: f32, y: f32, width: Option<f32>, height: Option<f32>) -> Self {
    Self::from_parts(
      LayoutKind::AbsoluteModifier { x, y, width, height },
      NodeKind::Empty,
      vec![self],
    )
  }

  pub fn align(self, alignment: Alignment) -> Self {
    Self::from_parts(LayoutKind::AlignModifier(alignment), NodeKind::Empty, vec![self])
  }

  pub fn flex(self, factor: f32) -> Self {
    Self::from_parts(
      LayoutKind::FlexModifier(crate::layout::layout_kind::FlexParams::grow(factor)),
      NodeKind::Empty,
      vec![self],
    )
  }

  pub fn flex_shrink(self, factor: f32) -> Self {
    Self::from_parts(
      LayoutKind::FlexModifier(crate::layout::layout_kind::FlexParams {
        grow: 0.0,
        shrink: factor,
        basis: None,
      }),
      NodeKind::Empty,
      vec![self],
    )
  }

  pub fn flex_full(self, grow: f32, shrink: f32, basis: Option<f32>) -> Self {
    Self::from_parts(
      LayoutKind::FlexModifier(crate::layout::layout_kind::FlexParams { grow, shrink, basis }),
      NodeKind::Empty,
      vec![self],
    )
  }

  pub fn background(mut self, color: Color) -> Self {
    self.color.set(Some(color));
    self
  }

  pub fn corner_radius(mut self, radius: BorderRadius) -> Self {
    self.border_radius.set(Some(radius));
    self
  }

  pub fn rounded(mut self, radius: f32) -> Self {
    self.border_radius.set(Some(BorderRadius::all(radius)));
    self
  }

  pub fn border_inside(mut self, width: f32, color: Color) -> Self {
    self.border.set(Some(Border {
      width: BorderWidth::all(width),
      color,
      placement: BorderPlacement::Inside,
    }));
    self
  }

  pub fn border_outside(mut self, width: f32, color: Color) -> Self {
    self.border.set(Some(Border {
      width: BorderWidth::all(width),
      color,
      placement: BorderPlacement::Outside,
    }));
    self
  }

  pub fn border_center(mut self, width: f32, color: Color) -> Self {
    self.border.set(Some(Border {
      width: BorderWidth::all(width),
      color,
      placement: BorderPlacement::Center,
    }));
    self
  }

  pub fn border_custom(mut self, border: Border) -> Self {
    self.border.set(Some(border));
    self
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

  pub fn scrollbar(mut self, style: ScrollBarStyle) -> Self {
    self.scrollbar_style.set(Some(style));
    self
  }

  pub fn ref_node(mut self, node_ref: NodeRef) -> Self {
    self.node_ref = Some(node_ref);
    self
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
    self.overflow = Overflow::Hidden;
    self
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

  pub(crate) fn layout_kind(&self) -> &LayoutKind {
    &self.layout_kind
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
    (*self.scrollbar_style).clone().unwrap_or_default()
  }

  pub fn color(&self) -> Option<Color> {
    *self.color
  }

  pub fn get_border_radius(&self) -> Option<BorderRadius> {
    *self.border_radius
  }

  pub fn get_border(&self) -> Option<&Border> {
    self.border.as_ref()
  }

  pub fn children(&self) -> &[Node] {
    &self.children
  }

  pub(crate) fn runtime_rect(&self) -> Option<RuntimeRect> {
    self.runtime_rect
  }

  pub(crate) fn set_runtime_rect(&mut self, rect: RuntimeRect) {
    self.runtime_rect = Some(rect);
    self.layout_cache.invalidate();
  }

  pub(crate) fn invalidate_layout_recursive(&self) {
    self.layout_cache.invalidate();
    for child in &self.children {
      child.invalidate_layout_recursive();
    }
  }

  pub(crate) fn min_main_size(&self, vertical: bool) -> f32 {
    match &self.layout_kind {
      LayoutKind::FlexModifier(_) | LayoutKind::PaddingModifier(_) | LayoutKind::AlignModifier(_) => {
        self.children.first().map(|c| c.min_main_size(vertical)).unwrap_or(0.0)
      }
      LayoutKind::FrameModifier(frame) => {
        if vertical {
          frame.min_height.unwrap_or(0.0)
        } else {
          frame.min_width.unwrap_or(0.0)
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

    for (child, old_child) in self.children.iter_mut().zip(old.children.iter()) {
      child.preserve_runtime_state_from(old_child);
    }
  }

  pub(crate) fn estimated_memory_bytes(&self) -> usize {
    std::mem::size_of::<Self>()
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
    self.text_content.as_ref().map(|text| text.capacity()).unwrap_or(0)
      + self.children.capacity() * std::mem::size_of::<Node>()
      + self.layout_cache.estimated_memory_bytes()
      + self
        .children
        .iter()
        .map(Node::estimated_child_heap_bytes)
        .sum::<usize>()
  }
}
