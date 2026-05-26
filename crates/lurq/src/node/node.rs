use std::sync::Arc;

use crate::{
  app::events::{KeyboardEvent, MouseEvent, ScrollEvent},
  core::{Guard, NodeRef},
  layout::{
    layout_kind::{FrameConstraints, LayoutKind}, scrollbar::ScrollBarStyle,
    text_style::TextStyle,
    Alignment,
    StackAlignment,
  },
  node::{
    border::{Border, BorderPlacement, BorderRadius, BorderWidth},
    color::Color,
    interaction_state::InteractionState,
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

pub struct Node {
  pub(crate) kind: LayoutKind,
  pub(crate) color: Guard<Option<Color>>,
  pub(crate) border_radius: Guard<Option<BorderRadius>>,
  pub(crate) border: Guard<Option<Border>>,
  pub(crate) scrollbar_style: Guard<Option<ScrollBarStyle>>,
  pub(crate) node_ref: Option<NodeRef>,
  pub(crate) interaction: Option<InteractionState>,
  pub(crate) layout_cache: crate::node::layout_cache::LayoutCache,
  pub(crate) children: Vec<Node>,
  pub(crate) events: EventHandlers,
}

impl Node {
  pub fn new() -> Self {
    Self {
      kind: LayoutKind::Leaf,
      color: Guard::new(None),
      border_radius: Guard::new(None),
      border: Guard::new(None),
      scrollbar_style: Guard::new(None),
      node_ref: None,
      interaction: None,
      layout_cache: Default::default(),
      children: vec![],
      events: EventHandlers::default(),
    }
  }

  pub fn text(content: &str) -> Self {
    Self {
      kind: LayoutKind::Text {
        content: content.to_owned(),
        style: TextStyle::default(),
      },
      color: Guard::new(None),
      border_radius: Guard::new(None),
      border: Guard::new(None),
      scrollbar_style: Guard::new(None),
      node_ref: None,
      interaction: None,
      layout_cache: Default::default(),
      children: vec![],
      events: EventHandlers::default(),
    }
  }

  pub fn text_styled(content: &str, style: TextStyle) -> Self {
    Self {
      kind: LayoutKind::Text {
        content: content.to_owned(),
        style,
      },
      color: Guard::new(None),
      border_radius: Guard::new(None),
      border: Guard::new(None),
      scrollbar_style: Guard::new(None),
      node_ref: None,
      interaction: None,
      layout_cache: Default::default(),
      children: vec![],
      events: EventHandlers::default(),
    }
  }

  pub fn row(spacing: f32, align: Alignment, children: Vec<Node>) -> Self {
    Self {
      kind: LayoutKind::Row { spacing, align },
      color: Guard::new(None),
      border_radius: Guard::new(None),
      border: Guard::new(None),
      scrollbar_style: Guard::new(None),
      node_ref: None,
      interaction: None,
      layout_cache: Default::default(),
      children,
      events: EventHandlers::default(),
    }
  }

  pub fn column(spacing: f32, align: Alignment, children: Vec<Node>) -> Self {
    Self {
      kind: LayoutKind::Column { spacing, align },
      color: Guard::new(None),
      border_radius: Guard::new(None),
      border: Guard::new(None),
      scrollbar_style: Guard::new(None),
      node_ref: None,
      interaction: None,
      layout_cache: Default::default(),
      children,
      events: EventHandlers::default(),
    }
  }

  pub fn stack(align: StackAlignment, children: Vec<Node>) -> Self {
    Self {
      kind: LayoutKind::Stack { align },
      color: Guard::new(None),
      border_radius: Guard::new(None),
      border: Guard::new(None),
      scrollbar_style: Guard::new(None),
      node_ref: None,
      interaction: None,
      layout_cache: Default::default(),
      children,
      events: EventHandlers::default(),
    }
  }

  pub fn padding(self, padding: Padding) -> Self {
    Self {
      kind: LayoutKind::PaddingModifier(padding),
      color: Guard::new(None),
      border_radius: Guard::new(None),
      border: Guard::new(None),
      scrollbar_style: Guard::new(None),
      node_ref: None,
      interaction: None,
      layout_cache: Default::default(),
      children: vec![self],
      events: EventHandlers::default(),
    }
  }

  pub fn frame(self, frame: FrameConstraints) -> Self {
    Self {
      kind: LayoutKind::FrameModifier(frame),
      color: Guard::new(None),
      border_radius: Guard::new(None),
      border: Guard::new(None),
      scrollbar_style: Guard::new(None),
      node_ref: None,
      interaction: None,
      layout_cache: Default::default(),
      children: vec![self],
      events: EventHandlers::default(),
    }
  }

  pub fn offset(self, x: f32, y: f32) -> Self {
    Self {
      kind: LayoutKind::OffsetModifier { x, y },
      color: Guard::new(None),
      border_radius: Guard::new(None),
      border: Guard::new(None),
      scrollbar_style: Guard::new(None),
      node_ref: None,
      interaction: None,
      layout_cache: Default::default(),
      children: vec![self],
      events: EventHandlers::default(),
    }
  }

  pub fn align(self, alignment: Alignment) -> Self {
    Self {
      kind: LayoutKind::AlignModifier(alignment),
      color: Guard::new(None),
      border_radius: Guard::new(None),
      border: Guard::new(None),
      scrollbar_style: Guard::new(None),
      node_ref: None,
      interaction: None,
      layout_cache: Default::default(),
      children: vec![self],
      events: EventHandlers::default(),
    }
  }

  pub fn flex(self, factor: f32) -> Self {
    Self {
      kind: LayoutKind::FlexModifier(factor),
      color: Guard::new(None),
      border_radius: Guard::new(None),
      border: Guard::new(None),
      scrollbar_style: Guard::new(None),
      node_ref: None,
      interaction: None,
      layout_cache: Default::default(),
      children: vec![self],
      events: EventHandlers::default(),
    }
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

  // --- Accessors ---

  pub fn kind(&self) -> &LayoutKind {
    &self.kind
  }

  pub fn scroll_state(&self) -> Option<crate::layout::layout_kind::ScrollState> {
    match &self.kind {
      LayoutKind::ScrollModifier { state, .. } => Some(state.clone()),
      _ => None,
    }
  }

  pub fn with_scroll_state(mut self, existing: crate::layout::layout_kind::ScrollState) -> Self {
    if let LayoutKind::ScrollModifier { state, .. } = &mut self.kind {
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
}
