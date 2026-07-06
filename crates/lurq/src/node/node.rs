use std::sync::{
  Arc,
  atomic::{AtomicU64, Ordering},
};

#[cfg(feature = "devtools")]
use crate::app::ctx::{
  ComponentContextDebug, ComponentEffectDebug, ComponentMemoDebug, ComponentSignalDebug, DevtoolsInspectableDebug,
};
use crate::{
  animation::{Animation, Transition},
  app::{
    events::{
      DragEvent, DropEvent, KeyboardEvent, MouseButton, MouseButtonMask, MouseEvent, ScrollEvent, TextInputEvent,
    },
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

static EVENT_HANDLER_IDS: AtomicU64 = AtomicU64::new(1);

pub struct EventHandler<T> {
  id: u64,
  callback: Arc<dyn Fn(*const T) + Send + Sync>,
}

impl<T> Clone for EventHandler<T> {
  fn clone(&self) -> Self {
    Self {
      id: self.id,
      callback: self.callback.clone(),
    }
  }
}

impl<T> EventHandler<T> {
  pub fn new(f: impl Fn(&T) + Send + Sync + 'static) -> Self {
    Self {
      id: EVENT_HANDLER_IDS.fetch_add(1, Ordering::Relaxed),
      callback: Arc::new(move |event| {
        // The pointer is produced from a live shared reference in `call` and
        // is only used for the duration of this synchronous callback.
        f(unsafe { &*event });
      }),
    }
  }

  pub fn from_event(f: impl Fn(T) + Send + Sync + 'static) -> Self
  where
    T: Clone + 'static,
  {
    Self::new(move |event| f(event.clone()))
  }

  pub(crate) fn call(&self, event: &T) {
    (self.callback)(event);
  }

  pub(crate) fn same_handler(&self, other: &Self) -> bool {
    self.id == other.id
  }
}

pub trait IntoEventHandler<T> {
  fn into_event_handler(self) -> EventHandler<T>;
}

impl<T> IntoEventHandler<T> for EventHandler<T> {
  fn into_event_handler(self) -> EventHandler<T> {
    self
  }
}

pub trait IntoMouseEventHandler {
  fn into_event_handler(self) -> EventHandler<MouseEvent>;
}

impl IntoMouseEventHandler for EventHandler<MouseEvent> {
  fn into_event_handler(self) -> EventHandler<MouseEvent> {
    self
  }
}

impl<F> IntoMouseEventHandler for F
where
  F: Fn(MouseEvent) + Send + Sync + 'static,
{
  fn into_event_handler(self) -> EventHandler<MouseEvent> {
    EventHandler::from_event(self)
  }
}

pub trait IntoDragEventHandler {
  fn into_event_handler(self) -> EventHandler<DragEvent>;
}

impl IntoDragEventHandler for EventHandler<DragEvent> {
  fn into_event_handler(self) -> EventHandler<DragEvent> {
    self
  }
}

impl<F> IntoDragEventHandler for F
where
  F: Fn(DragEvent) + Send + Sync + 'static,
{
  fn into_event_handler(self) -> EventHandler<DragEvent> {
    EventHandler::from_event(self)
  }
}

pub trait IntoDropEventHandler {
  fn into_event_handler(self) -> EventHandler<DropEvent>;
}

impl IntoDropEventHandler for EventHandler<DropEvent> {
  fn into_event_handler(self) -> EventHandler<DropEvent> {
    self
  }
}

impl<F> IntoDropEventHandler for F
where
  F: Fn(DropEvent) + Send + Sync + 'static,
{
  fn into_event_handler(self) -> EventHandler<DropEvent> {
    EventHandler::from_event(self)
  }
}

pub trait IntoKeyboardEventHandler {
  fn into_event_handler(self) -> EventHandler<KeyboardEvent>;
}

impl IntoKeyboardEventHandler for EventHandler<KeyboardEvent> {
  fn into_event_handler(self) -> EventHandler<KeyboardEvent> {
    self
  }
}

impl<F> IntoKeyboardEventHandler for F
where
  F: Fn(KeyboardEvent) + Send + Sync + 'static,
{
  fn into_event_handler(self) -> EventHandler<KeyboardEvent> {
    EventHandler::from_event(self)
  }
}

pub trait IntoScrollEventHandler {
  fn into_event_handler(self) -> EventHandler<ScrollEvent>;
}

impl IntoScrollEventHandler for EventHandler<ScrollEvent> {
  fn into_event_handler(self) -> EventHandler<ScrollEvent> {
    self
  }
}

impl<F> IntoScrollEventHandler for F
where
  F: Fn(ScrollEvent) + Send + Sync + 'static,
{
  fn into_event_handler(self) -> EventHandler<ScrollEvent> {
    EventHandler::from_event(self)
  }
}

pub trait IntoTextInputEventHandler {
  fn into_event_handler(self) -> EventHandler<TextInputEvent>;
}

impl IntoTextInputEventHandler for EventHandler<TextInputEvent> {
  fn into_event_handler(self) -> EventHandler<TextInputEvent> {
    self
  }
}

impl<F> IntoTextInputEventHandler for F
where
  F: Fn(TextInputEvent) + Send + Sync + 'static,
{
  fn into_event_handler(self) -> EventHandler<TextInputEvent> {
    EventHandler::from_event(self)
  }
}

#[derive(Clone)]
pub struct VoidEventHandler {
  id: u64,
  callback: Arc<dyn Fn() + Send + Sync>,
}

impl VoidEventHandler {
  pub fn new(f: impl Fn() + Send + Sync + 'static) -> Self {
    Self {
      id: EVENT_HANDLER_IDS.fetch_add(1, Ordering::Relaxed),
      callback: Arc::new(f),
    }
  }

  pub(crate) fn call(&self) {
    (self.callback)();
  }

  pub(crate) fn same_handler(&self, other: &Self) -> bool {
    self.id == other.id
  }
}

pub trait IntoVoidEventHandler {
  fn into_void_event_handler(self) -> VoidEventHandler;
}

impl IntoVoidEventHandler for VoidEventHandler {
  fn into_void_event_handler(self) -> VoidEventHandler {
    self
  }
}

impl<F> IntoVoidEventHandler for F
where
  F: Fn() + Send + Sync + 'static,
{
  fn into_void_event_handler(self) -> VoidEventHandler {
    VoidEventHandler::new(self)
  }
}

type Callback<T> = EventHandler<T>;
type VoidCallback = VoidEventHandler;
type ScrollbarStyleCallback = Arc<dyn Fn(ScrollBarStyle) -> ScrollBarStyle + Send + Sync>;

#[allow(private_bounds)]
pub(crate) trait NodeUpdate {
  fn child(&mut self, child: Node);
  fn with_children(&mut self, children: impl IntoIterator<Item = Node>);
  fn spacing(&mut self, spacing: impl Into<SpacingValue>);
  fn align_items(&mut self, align: Alignment);
  fn justify(&mut self, justify: crate::layout::layout_kind::Justify);
  fn wrap(&mut self);
  fn stack_align(&mut self, align: StackAlignment);
  fn size(&mut self, width: impl Into<Dimension>, height: impl Into<Dimension>);
  fn width(&mut self, width: impl Into<Dimension>);
  fn height(&mut self, height: impl Into<Dimension>);
  fn min_width(&mut self, width: impl Into<Dimension>);
  fn max_width(&mut self, width: impl Into<Dimension>);
  fn min_height(&mut self, height: impl Into<Dimension>);
  fn max_height(&mut self, height: impl Into<Dimension>);
  fn min_size(&mut self, width: impl Into<Dimension>, height: impl Into<Dimension>);
  fn max_size(&mut self, width: impl Into<Dimension>, height: impl Into<Dimension>);
  fn padding_left(&mut self, val: impl Into<SpacingValue>);
  fn padding_right(&mut self, val: impl Into<SpacingValue>);
  fn padding_top(&mut self, val: impl Into<SpacingValue>);
  fn padding_bottom(&mut self, val: impl Into<SpacingValue>);
  fn padding_horizontal(&mut self, val: impl Into<SpacingValue>);
  fn padding_vertical(&mut self, val: impl Into<SpacingValue>);
  fn padding(&mut self, padding: impl Into<Padding>);
  fn padding_custom(&mut self, padding: Padding);
  fn frame(&mut self, frame: FrameConstraints);
  fn offset(&mut self, x: f32, y: f32);
  fn relative(&mut self, x: f32, y: f32);
  fn absolute(&mut self, x: f32, y: f32, width: impl Into<Dimension>, height: impl Into<Dimension>);
  fn absolute_position(&mut self, x: f32, y: f32);
  fn align(&mut self, alignment: Alignment);
  fn flex(&mut self, factor: f32);
  fn flex_shrink(&mut self, factor: f32);
  fn flex_full(&mut self, grow: f32, shrink: f32, basis: Option<f32>);
  fn background(&mut self, color: impl Into<BackgroundColor>);
  fn background_gradient(&mut self, gradient: impl Into<Gradient>);
  fn caret_color(&mut self, color: impl Into<TextColor>);
  fn selection_color(&mut self, color: impl Into<TextColor>);
  fn text_input_caret_mode(&mut self, mode: CaretMode);
  fn corner_radius(&mut self, radius: impl Into<RadiusValue>);
  fn corner_radius_custom(&mut self, radius: BorderRadius);
  fn corner_radius_top_left(&mut self, radius: impl Into<RadiusValue>);
  fn corner_radius_top_right(&mut self, radius: impl Into<RadiusValue>);
  fn corner_radius_bottom_right(&mut self, radius: impl Into<RadiusValue>);
  fn corner_radius_bottom_left(&mut self, radius: impl Into<RadiusValue>);
  fn rounded(&mut self, radius: impl Into<RadiusValue>);
  fn border_inside(&mut self, width: impl Into<BorderSizeValue>, color: impl Into<BackgroundColor>);
  fn border_outside(&mut self, width: impl Into<BorderSizeValue>, color: impl Into<BackgroundColor>);
  fn border_center(&mut self, width: impl Into<BorderSizeValue>, color: impl Into<BackgroundColor>);
  fn border(&mut self, border: Border);
  fn border_custom(&mut self, border: Borders);
  fn border_top(&mut self, border: Border);
  fn border_right(&mut self, border: Border);
  fn border_bottom(&mut self, border: Border);
  fn border_left(&mut self, border: Border);
  fn cursor(&mut self, cursor: CursorIcon);
  #[cfg(feature = "image")]
  fn background_image(&mut self, data: impl Into<crate::images::ImageKind>);
  #[cfg(feature = "image")]
  fn background_size(&mut self, size: BackgroundSize);
  #[cfg(feature = "image")]
  fn background_cover(&mut self);
  #[cfg(feature = "image")]
  fn background_contain(&mut self);
  fn hovered_style(&mut self, style: Style);
  fn active_style(&mut self, style: Style);
  fn focused_style(&mut self, style: Style);
  fn hovered(&mut self, f: impl FnOnce(Style) -> Style);
  fn active(&mut self, f: impl FnOnce(Style) -> Style);
  fn focused(&mut self, f: impl FnOnce(Style) -> Style);
  fn on_click(&mut self, f: impl IntoMouseEventHandler);
  fn off_click(&mut self, f: impl IntoMouseEventHandler);
  fn on_mouse_click(&mut self, button: MouseButton, f: impl IntoMouseEventHandler);
  fn off_mouse_click(&mut self, button: MouseButton, f: impl IntoMouseEventHandler);
  fn on_dblclick(&mut self, f: impl IntoMouseEventHandler);
  fn off_dblclick(&mut self, f: impl IntoMouseEventHandler);
  fn on_mouse_down(&mut self, f: impl IntoMouseEventHandler);
  fn off_mouse_down(&mut self, f: impl IntoMouseEventHandler);
  fn on_mouse_up(&mut self, f: impl IntoMouseEventHandler);
  fn off_mouse_up(&mut self, f: impl IntoMouseEventHandler);
  fn on_mouse_move(&mut self, f: impl IntoMouseEventHandler);
  fn off_mouse_move(&mut self, f: impl IntoMouseEventHandler);
  fn start_drag_buttons(&mut self, buttons: MouseButtonMask);
  fn on_drag_start(&mut self, f: impl IntoDragEventHandler);
  fn off_drag_start(&mut self, f: impl IntoDragEventHandler);
  fn on_drag_move(&mut self, f: impl IntoDragEventHandler);
  fn off_drag_move(&mut self, f: impl IntoDragEventHandler);
  fn on_drag_end(&mut self, f: impl IntoDragEventHandler);
  fn off_drag_end(&mut self, f: impl IntoDragEventHandler);
  fn on_drop(&mut self, f: impl IntoDropEventHandler);
  fn off_drop(&mut self, f: impl IntoDropEventHandler);
  fn on_mouse_enter(&mut self, f: impl IntoVoidEventHandler);
  fn off_mouse_enter(&mut self, f: impl IntoVoidEventHandler);
  fn on_mouse_leave(&mut self, f: impl IntoVoidEventHandler);
  fn off_mouse_leave(&mut self, f: impl IntoVoidEventHandler);
  fn on_key_down(&mut self, f: impl IntoKeyboardEventHandler);
  fn off_key_down(&mut self, f: impl IntoKeyboardEventHandler);
  fn on_key_up(&mut self, f: impl IntoKeyboardEventHandler);
  fn off_key_up(&mut self, f: impl IntoKeyboardEventHandler);
  fn on_focus(&mut self, f: impl IntoVoidEventHandler);
  fn off_focus(&mut self, f: impl IntoVoidEventHandler);
  fn on_blur(&mut self, f: impl IntoVoidEventHandler);
  fn off_blur(&mut self, f: impl IntoVoidEventHandler);
  fn on_scroll(&mut self, f: impl IntoScrollEventHandler);
  fn off_scroll(&mut self, f: impl IntoScrollEventHandler);
  fn on_scroll_start(&mut self, f: impl IntoScrollEventHandler);
  fn off_scroll_start(&mut self, f: impl IntoScrollEventHandler);
  fn on_scroll_end(&mut self, f: impl IntoScrollEventHandler);
  fn off_scroll_end(&mut self, f: impl IntoScrollEventHandler);
  fn on_scroll_reach_top(&mut self, f: impl IntoScrollEventHandler);
  fn off_scroll_reach_top(&mut self, f: impl IntoScrollEventHandler);
  fn on_scroll_reach_bottom(&mut self, f: impl IntoScrollEventHandler);
  fn off_scroll_reach_bottom(&mut self, f: impl IntoScrollEventHandler);
  fn opacity(&mut self, value: f32);
  fn transform(&mut self, t: Transform2D);
  fn transition(&mut self, spec: Transition);
  fn animation(&mut self, spec: Animation);
  fn scrollbar(&mut self, style: ScrollBarStyle);
  fn scrollbar_hovered(&mut self, f: impl Fn(ScrollBarStyle) -> ScrollBarStyle + Send + Sync + 'static);
  fn culling(&mut self, enabled: bool);
  fn hit_test(&mut self, behavior: HitTestBehavior);
  fn pointer_events_none(&mut self);
  fn ref_element(&mut self, element_ref: impl Into<CoreElementRef>);
  fn interactive(&mut self, state: InteractionState);
  fn focusable(&mut self, focusable: bool);
  fn tab_index(&mut self, tab_index: i32);
  fn button_kind(&mut self, kind: ButtonKind);
  #[cfg(feature = "form")]
  fn name(&mut self, name: impl Into<Arc<str>>);
  fn text_wrap(&mut self, wrap: bool);
  fn text_overflow(&mut self, overflow: TextOverflow);
  fn selectable(&mut self, selectable: bool);
  fn text_transform_mode(&mut self, mode: TextTransformMode);
  fn text_variant(&mut self, typography_style: impl Into<TypographyStyle>);
  fn text_color(&mut self, color: impl Into<TextColor>);
  fn text_shadow(&mut self, shadow: crate::layout::text_style::TextShadow);
  fn text_align(&mut self, align: impl Into<TextAlign>);
  fn on_input(&mut self, f: impl IntoTextInputEventHandler);
  fn off_input(&mut self, f: impl IntoTextInputEventHandler);
  fn placeholder(&mut self, placeholder: &str);
  fn text_input_overflow(&mut self, overflow: crate::node::node_kind::TextInputOverflow);
  fn text_input_mask(&mut self);
  fn text_input_mask_char(&mut self, mask: char);
  fn text_input_unmask(&mut self);
  fn text_input_style(&mut self, text_style: TextStyle);
  fn text_input_placeholder_style(&mut self, text_style: TextStyle);
  fn text_input_align(&mut self, align: impl Into<TextAlign>);
  fn text_input_rows(&mut self, min_rows: usize, max_rows: usize);
  fn text_input_min_rows(&mut self, min_rows: usize);
  fn text_input_max_rows(&mut self, max_rows: usize);
  fn text_input_rows_exact(&mut self, rows: usize);
  fn range(&mut self, min: i32, max: i32);
  fn range_f32(&mut self, min: f32, max: f32);
  fn slider_step(&mut self, step: f32);
  fn slider_track_style(&mut self, style: SliderPartStyle);
  fn slider_track_hovered_style(&mut self, style: SliderPartStyle);
  fn slider_fill_style(&mut self, style: SliderPartStyle);
  fn slider_fill_hovered_style(&mut self, style: SliderPartStyle);
  fn slider_thumb_style(&mut self, style: SliderPartStyle);
  fn slider_thumb_hovered_style(&mut self, style: SliderPartStyle);
  fn checkbox_box_style(&mut self, style: CheckboxStyle);
  fn checkbox_checked_box_style(&mut self, style: CheckboxStyle);
  fn checkbox_box_hovered_style(&mut self, style: CheckboxStyle);
  fn checkbox_checked_box_hovered_style(&mut self, style: CheckboxStyle);
  fn clip(&mut self);
  fn overflow_visible(&mut self);
  fn intrinsic(&mut self, width: f32, height: f32);
  fn with_scroll_state(&mut self, existing: crate::layout::layout_kind::ScrollState);
}
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HitTestBehavior {
  #[default]
  Auto,
  None,
  ContentOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SyntheticNodeRole {
  OverlayHost,
  SelectMenu,
}

#[derive(Default, Clone)]
pub struct EventHandlers {
  pub on_click: Vec<Callback<MouseEvent>>,
  pub on_mouse_click: Vec<(MouseButton, Callback<MouseEvent>)>,
  pub on_dblclick: Vec<Callback<MouseEvent>>,
  pub on_mouse_down: Vec<Callback<MouseEvent>>,
  pub on_mouse_up: Vec<Callback<MouseEvent>>,
  pub on_mouse_move: Vec<Callback<MouseEvent>>,
  pub start_drag_buttons: MouseButtonMask,
  pub on_drag_start: Vec<Callback<DragEvent>>,
  pub on_drag_move: Vec<Callback<DragEvent>>,
  pub on_drag_end: Vec<Callback<DragEvent>>,
  pub on_drop: Vec<Callback<DropEvent>>,
  pub on_mouse_enter: Vec<VoidCallback>,
  pub on_mouse_leave: Vec<VoidCallback>,
  pub on_key_down: Vec<Callback<KeyboardEvent>>,
  pub on_key_up: Vec<Callback<KeyboardEvent>>,
  pub on_focus: Vec<VoidCallback>,
  pub on_blur: Vec<VoidCallback>,
  #[cfg(feature = "form")]
  pub on_submit: Option<FormSubmitCallback>,
  pub on_scroll: Vec<Callback<ScrollEvent>>,
  pub on_scroll_start: Vec<Callback<ScrollEvent>>,
  pub on_scroll_end: Vec<Callback<ScrollEvent>>,
  pub on_scroll_reach_top: Vec<Callback<ScrollEvent>>,
  pub on_scroll_reach_bottom: Vec<Callback<ScrollEvent>>,
}

pub(crate) struct Node {
  pub(crate) node_id: NodeId,
  pub(crate) tag_name: Arc<str>,
  pub(crate) component_slot_id: Option<u64>,
  pub(crate) component_key: Option<Arc<str>>,
  pub(crate) overlay_declaration: Option<Box<crate::app::ctx::OverlaySpec>>,
  pub(crate) modal_declaration: Option<Box<crate::app::ctx::ModalSpec>>,
  pub(crate) layout_neutral: bool,
  pub(crate) synthetic_role: Option<SyntheticNodeRole>,
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
  pub(crate) selection_color: Guard<Option<TextColor>>,
  pub(crate) caret_mode: Guard<Option<CaretMode>>,
  pub(crate) cursor: Option<CursorIcon>,
  pub(crate) hit_test: HitTestBehavior,
  #[cfg(feature = "image")]
  pub(crate) background_image: Guard<Option<crate::images::ImageData>>,
  #[cfg(feature = "image")]
  pub(crate) background_size: BackgroundSize,
  #[cfg(all(feature = "image", feature = "resources"))]
  pub(crate) background_resource_image: Option<Arc<str>>,
  pub(crate) scrollbar_style: Guard<Option<ScrollBarStyle>>,
  pub(crate) scrollbar_hovered_style: Option<ScrollbarStyleCallback>,
  pub(crate) element_ref: Option<CoreElementRef>,
  pub(crate) drag_payload: Option<crate::app::events::DragPayload>,
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

impl NodeUpdate for Node {
  fn child(&mut self, child: Node) {
    self.children.push(child);
  }

  fn with_children(&mut self, children: impl IntoIterator<Item = Node>) {
    self.children.extend(children);
  }

  fn spacing(&mut self, spacing: impl Into<SpacingValue>) {
    let spacing = spacing.into();
    match &mut self.layout_kind {
      LayoutKind::Row { spacing: s, .. } | LayoutKind::Column { spacing: s, .. } => *s = spacing,
      _ => {}
    }
  }

  fn align_items(&mut self, align: Alignment) {
    match &mut self.layout_kind {
      LayoutKind::Row { align: a, .. } | LayoutKind::Column { align: a, .. } => *a = align,
      _ => {}
    }
  }

  fn justify(&mut self, justify: crate::layout::layout_kind::Justify) {
    match &mut self.layout_kind {
      LayoutKind::Row { justify: j, .. } | LayoutKind::Column { justify: j, .. } => *j = justify,
      _ => {}
    }
  }

  fn wrap(&mut self) {
    match &mut self.layout_kind {
      LayoutKind::Row { wrap, .. } | LayoutKind::Column { wrap, .. } => {
        *wrap = crate::layout::layout_kind::FlexWrap::Wrap
      }
      _ => {}
    }
  }

  fn stack_align(&mut self, align: StackAlignment) {
    if let LayoutKind::Stack { align: a } = &mut self.layout_kind {
      *a = align;
    }
  }

  fn size(&mut self, width: impl Into<Dimension>, height: impl Into<Dimension>) {
    NodeUpdate::frame(
      self,
      FrameConstraints {
        width: Some(width.into()),
        height: Some(height.into()),
        ..Default::default()
      },
    );
  }

  fn width(&mut self, width: impl Into<Dimension>) {
    NodeUpdate::frame(
      self,
      FrameConstraints {
        width: Some(width.into()),
        ..Default::default()
      },
    );
  }

  fn height(&mut self, height: impl Into<Dimension>) {
    NodeUpdate::frame(
      self,
      FrameConstraints {
        height: Some(height.into()),
        ..Default::default()
      },
    );
  }

  fn min_width(&mut self, width: impl Into<Dimension>) {
    NodeUpdate::frame(
      self,
      FrameConstraints {
        min_width: Some(width.into()),
        ..Default::default()
      },
    );
  }

  fn max_width(&mut self, width: impl Into<Dimension>) {
    NodeUpdate::frame(
      self,
      FrameConstraints {
        max_width: Some(width.into()),
        ..Default::default()
      },
    );
  }

  fn min_height(&mut self, height: impl Into<Dimension>) {
    NodeUpdate::frame(
      self,
      FrameConstraints {
        min_height: Some(height.into()),
        ..Default::default()
      },
    );
  }

  fn max_height(&mut self, height: impl Into<Dimension>) {
    NodeUpdate::frame(
      self,
      FrameConstraints {
        max_height: Some(height.into()),
        ..Default::default()
      },
    );
  }

  fn min_size(&mut self, width: impl Into<Dimension>, height: impl Into<Dimension>) {
    NodeUpdate::frame(
      self,
      FrameConstraints {
        min_width: Some(width.into()),
        min_height: Some(height.into()),
        ..Default::default()
      },
    );
  }

  fn max_size(&mut self, width: impl Into<Dimension>, height: impl Into<Dimension>) {
    NodeUpdate::frame(
      self,
      FrameConstraints {
        max_width: Some(width.into()),
        max_height: Some(height.into()),
        ..Default::default()
      },
    );
  }

  fn padding_left(&mut self, val: impl Into<SpacingValue>) {
    NodeUpdate::padding(self, Padding::new().left(val.into()));
  }

  fn padding_right(&mut self, val: impl Into<SpacingValue>) {
    NodeUpdate::padding(self, Padding::new().right(val.into()));
  }

  fn padding_top(&mut self, val: impl Into<SpacingValue>) {
    NodeUpdate::padding(self, Padding::new().top(val.into()));
  }

  fn padding_bottom(&mut self, val: impl Into<SpacingValue>) {
    NodeUpdate::padding(self, Padding::new().bottom(val.into()));
  }

  fn padding_horizontal(&mut self, val: impl Into<SpacingValue>) {
    NodeUpdate::padding(self, Padding::horizontal(val.into()));
  }

  fn padding_vertical(&mut self, val: impl Into<SpacingValue>) {
    NodeUpdate::padding(self, Padding::vertical(val.into()));
  }

  fn padding(&mut self, padding: impl Into<Padding>) {
    let padding = padding.into();
    self.padding.merge_from(&padding);
    self.layout_cache.invalidate();
  }

  fn padding_custom(&mut self, padding: Padding) {
    NodeUpdate::padding(self, padding);
  }

  fn frame(&mut self, frame: FrameConstraints) {
    self.frame = merge_frame(self.frame, frame);
    self.layout_cache.invalidate();
  }

  fn offset(&mut self, x: f32, y: f32) {
    self.offset = Some(Offset::new(x, y));
    self.layout_cache.invalidate();
  }

  fn relative(&mut self, x: f32, y: f32) {
    NodeUpdate::offset(self, x, y);
  }

  fn absolute(&mut self, x: f32, y: f32, width: impl Into<Dimension>, height: impl Into<Dimension>) {
    self.position = Position::Absolute {
      x,
      y,
      width: Some(width.into()),
      height: Some(height.into()),
    };
    self.layout_cache.invalidate();
  }

  fn absolute_position(&mut self, x: f32, y: f32) {
    self.position = Position::Absolute {
      x,
      y,
      width: None,
      height: None,
    };
    self.layout_cache.invalidate();
  }

  fn align(&mut self, alignment: Alignment) {
    self.align_self = Some(alignment);
    self.layout_cache.invalidate();
  }

  fn flex(&mut self, factor: f32) {
    self.flex = Some(FlexParams::grow(factor));
    self.layout_cache.invalidate();
  }

  fn flex_shrink(&mut self, factor: f32) {
    self.flex = Some(FlexParams {
      grow: 0.0,
      shrink: factor,
      basis: None,
    });
    self.layout_cache.invalidate();
  }

  fn flex_full(&mut self, grow: f32, shrink: f32, basis: Option<f32>) {
    self.flex = Some(FlexParams { grow, shrink, basis });
    self.layout_cache.invalidate();
  }

  fn background(&mut self, color: impl Into<BackgroundColor>) {
    self.color.set(Some(color.into()));
  }

  fn background_gradient(&mut self, gradient: impl Into<Gradient>) {
    self.gradient.set(Some(gradient.into()));
  }

  fn caret_color(&mut self, color: impl Into<TextColor>) {
    self.set_caret_color(color.into());
  }

  fn selection_color(&mut self, color: impl Into<TextColor>) {
    self.set_selection_color(color.into());
  }

  fn text_input_caret_mode(&mut self, mode: CaretMode) {
    if matches!(self.node_kind, NodeKind::TextInput { .. }) {
      self.caret_mode.set(Some(mode));
    }
  }

  fn corner_radius(&mut self, radius: impl Into<RadiusValue>) {
    self.border_radius.set(Some(ThemedBorderRadius::all(radius)));
  }

  fn corner_radius_custom(&mut self, radius: BorderRadius) {
    self.border_radius.set(Some(radius.into()));
  }

  fn corner_radius_top_left(&mut self, radius: impl Into<RadiusValue>) {
    let mut border_radius = (*self.border_radius).unwrap_or_default();
    border_radius.top_left = radius.into();
    self.border_radius.set(Some(border_radius));
  }

  fn corner_radius_top_right(&mut self, radius: impl Into<RadiusValue>) {
    let mut border_radius = (*self.border_radius).unwrap_or_default();
    border_radius.top_right = radius.into();
    self.border_radius.set(Some(border_radius));
  }

  fn corner_radius_bottom_right(&mut self, radius: impl Into<RadiusValue>) {
    let mut border_radius = (*self.border_radius).unwrap_or_default();
    border_radius.bottom_right = radius.into();
    self.border_radius.set(Some(border_radius));
  }

  fn corner_radius_bottom_left(&mut self, radius: impl Into<RadiusValue>) {
    let mut border_radius = (*self.border_radius).unwrap_or_default();
    border_radius.bottom_left = radius.into();
    self.border_radius.set(Some(border_radius));
  }

  fn rounded(&mut self, radius: impl Into<RadiusValue>) {
    NodeUpdate::corner_radius(self, radius);
  }

  fn border_inside(&mut self, width: impl Into<BorderSizeValue>, color: impl Into<BackgroundColor>) {
    self.border.set(Some(Borders::all(Border::inside(width, color))));
  }

  fn border_outside(&mut self, width: impl Into<BorderSizeValue>, color: impl Into<BackgroundColor>) {
    self.border.set(Some(Borders::all(Border::outside(width, color))));
  }

  fn border_center(&mut self, width: impl Into<BorderSizeValue>, color: impl Into<BackgroundColor>) {
    self.border.set(Some(Borders::all(Border::center(width, color))));
  }

  fn border(&mut self, border: Border) {
    self.border.set(Some(Borders::all(border)));
  }

  fn border_custom(&mut self, border: Borders) {
    self.border.set(Some(border));
  }

  fn border_top(&mut self, border: Border) {
    let mut borders = <Option<Borders> as Clone>::clone(&self.border).unwrap_or_default();
    borders.top = Some(border);
    self.border.set(Some(borders));
  }

  fn border_right(&mut self, border: Border) {
    let mut borders = <Option<Borders> as Clone>::clone(&self.border).unwrap_or_default();
    borders.right = Some(border);
    self.border.set(Some(borders));
  }

  fn border_bottom(&mut self, border: Border) {
    let mut borders = <Option<Borders> as Clone>::clone(&self.border).unwrap_or_default();
    borders.bottom = Some(border);
    self.border.set(Some(borders));
  }

  fn border_left(&mut self, border: Border) {
    let mut borders = <Option<Borders> as Clone>::clone(&self.border).unwrap_or_default();
    borders.left = Some(border);
    self.border.set(Some(borders));
  }

  fn cursor(&mut self, cursor: CursorIcon) {
    self.cursor = Some(cursor);
  }

  #[cfg(feature = "image")]
  fn background_image(&mut self, data: impl Into<crate::images::ImageKind>) {
    match data.into() {
      crate::images::ImageKind::Bytes(data) => self.background_image.set(Some(data)),
      crate::images::ImageKind::Native(data) => self.background_image.set(Some(data.image_data())),
      #[cfg(feature = "resources")]
      crate::images::ImageKind::Resource(path) => self.background_resource_image = Some(path),
    }
  }

  #[cfg(feature = "image")]
  fn background_size(&mut self, size: BackgroundSize) {
    self.background_size = size;
  }

  #[cfg(feature = "image")]
  fn background_cover(&mut self) {
    self.background_size = BackgroundSize::Cover;
  }

  #[cfg(feature = "image")]
  fn background_contain(&mut self) {
    self.background_size = BackgroundSize::Contain;
  }

  fn hovered_style(&mut self, style: Style) {
    self.state_styles.hovered = Some(style);
  }

  fn active_style(&mut self, style: Style) {
    self.state_styles.active = Some(style);
  }

  fn focused_style(&mut self, style: Style) {
    self.state_styles.focused = Some(style);
  }

  fn hovered(&mut self, f: impl FnOnce(Style) -> Style) {
    NodeUpdate::hovered_style(self, f(Style::new()));
  }

  fn active(&mut self, f: impl FnOnce(Style) -> Style) {
    NodeUpdate::active_style(self, f(Style::new()));
  }

  fn focused(&mut self, f: impl FnOnce(Style) -> Style) {
    NodeUpdate::focused_style(self, f(Style::new()));
  }

  fn on_click(&mut self, f: impl IntoMouseEventHandler) {
    self.events.on_click.push(f.into_event_handler());
  }

  fn off_click(&mut self, f: impl IntoMouseEventHandler) {
    let handler = f.into_event_handler();
    self.events.on_click.retain(|existing| !existing.same_handler(&handler));
  }

  fn on_mouse_click(&mut self, button: MouseButton, f: impl IntoMouseEventHandler) {
    self.events.on_mouse_click.push((button, f.into_event_handler()));
  }

  fn off_mouse_click(&mut self, button: MouseButton, f: impl IntoMouseEventHandler) {
    let handler = f.into_event_handler();
    self
      .events
      .on_mouse_click
      .retain(|(handler_button, existing)| *handler_button != button || !existing.same_handler(&handler));
  }

  fn on_dblclick(&mut self, f: impl IntoMouseEventHandler) {
    self.events.on_dblclick.push(f.into_event_handler());
  }

  fn off_dblclick(&mut self, f: impl IntoMouseEventHandler) {
    let handler = f.into_event_handler();
    self
      .events
      .on_dblclick
      .retain(|existing| !existing.same_handler(&handler));
  }

  fn on_mouse_down(&mut self, f: impl IntoMouseEventHandler) {
    self.events.on_mouse_down.push(f.into_event_handler());
  }

  fn off_mouse_down(&mut self, f: impl IntoMouseEventHandler) {
    let handler = f.into_event_handler();
    self
      .events
      .on_mouse_down
      .retain(|existing| !existing.same_handler(&handler));
  }

  fn on_mouse_up(&mut self, f: impl IntoMouseEventHandler) {
    self.events.on_mouse_up.push(f.into_event_handler());
  }

  fn off_mouse_up(&mut self, f: impl IntoMouseEventHandler) {
    let handler = f.into_event_handler();
    self
      .events
      .on_mouse_up
      .retain(|existing| !existing.same_handler(&handler));
  }

  fn on_mouse_move(&mut self, f: impl IntoMouseEventHandler) {
    self.events.on_mouse_move.push(f.into_event_handler());
  }

  fn off_mouse_move(&mut self, f: impl IntoMouseEventHandler) {
    let handler = f.into_event_handler();
    self
      .events
      .on_mouse_move
      .retain(|existing| !existing.same_handler(&handler));
  }

  fn start_drag_buttons(&mut self, buttons: MouseButtonMask) {
    self.events.start_drag_buttons = buttons;
  }

  fn on_drag_start(&mut self, f: impl IntoDragEventHandler) {
    self.events.on_drag_start.push(f.into_event_handler());
  }

  fn off_drag_start(&mut self, f: impl IntoDragEventHandler) {
    let handler = f.into_event_handler();
    self
      .events
      .on_drag_start
      .retain(|existing| !existing.same_handler(&handler));
  }

  fn on_drag_move(&mut self, f: impl IntoDragEventHandler) {
    self.events.on_drag_move.push(f.into_event_handler());
  }

  fn off_drag_move(&mut self, f: impl IntoDragEventHandler) {
    let handler = f.into_event_handler();
    self
      .events
      .on_drag_move
      .retain(|existing| !existing.same_handler(&handler));
  }

  fn on_drag_end(&mut self, f: impl IntoDragEventHandler) {
    self.events.on_drag_end.push(f.into_event_handler());
  }

  fn off_drag_end(&mut self, f: impl IntoDragEventHandler) {
    let handler = f.into_event_handler();
    self
      .events
      .on_drag_end
      .retain(|existing| !existing.same_handler(&handler));
  }

  fn on_drop(&mut self, f: impl IntoDropEventHandler) {
    self.events.on_drop.push(f.into_event_handler());
  }

  fn off_drop(&mut self, f: impl IntoDropEventHandler) {
    let handler = f.into_event_handler();
    self.events.on_drop.retain(|existing| !existing.same_handler(&handler));
  }

  fn on_mouse_enter(&mut self, f: impl IntoVoidEventHandler) {
    self.events.on_mouse_enter.push(f.into_void_event_handler());
  }

  fn off_mouse_enter(&mut self, f: impl IntoVoidEventHandler) {
    let handler = f.into_void_event_handler();
    self
      .events
      .on_mouse_enter
      .retain(|existing| !existing.same_handler(&handler));
  }

  fn on_mouse_leave(&mut self, f: impl IntoVoidEventHandler) {
    self.events.on_mouse_leave.push(f.into_void_event_handler());
  }

  fn off_mouse_leave(&mut self, f: impl IntoVoidEventHandler) {
    let handler = f.into_void_event_handler();
    self
      .events
      .on_mouse_leave
      .retain(|existing| !existing.same_handler(&handler));
  }

  fn on_key_down(&mut self, f: impl IntoKeyboardEventHandler) {
    self.events.on_key_down.push(f.into_event_handler());
  }

  fn off_key_down(&mut self, f: impl IntoKeyboardEventHandler) {
    let handler = f.into_event_handler();
    self
      .events
      .on_key_down
      .retain(|existing| !existing.same_handler(&handler));
  }

  fn on_key_up(&mut self, f: impl IntoKeyboardEventHandler) {
    self.events.on_key_up.push(f.into_event_handler());
  }

  fn off_key_up(&mut self, f: impl IntoKeyboardEventHandler) {
    let handler = f.into_event_handler();
    self
      .events
      .on_key_up
      .retain(|existing| !existing.same_handler(&handler));
  }

  fn on_focus(&mut self, f: impl IntoVoidEventHandler) {
    self.events.on_focus.push(f.into_void_event_handler());
  }

  fn off_focus(&mut self, f: impl IntoVoidEventHandler) {
    let handler = f.into_void_event_handler();
    self.events.on_focus.retain(|existing| !existing.same_handler(&handler));
  }

  fn on_blur(&mut self, f: impl IntoVoidEventHandler) {
    self.events.on_blur.push(f.into_void_event_handler());
  }

  fn off_blur(&mut self, f: impl IntoVoidEventHandler) {
    let handler = f.into_void_event_handler();
    self.events.on_blur.retain(|existing| !existing.same_handler(&handler));
  }

  fn on_scroll(&mut self, f: impl IntoScrollEventHandler) {
    self.events.on_scroll.push(f.into_event_handler());
  }

  fn off_scroll(&mut self, f: impl IntoScrollEventHandler) {
    let handler = f.into_event_handler();
    self
      .events
      .on_scroll
      .retain(|existing| !existing.same_handler(&handler));
  }

  fn on_scroll_start(&mut self, f: impl IntoScrollEventHandler) {
    self.events.on_scroll_start.push(f.into_event_handler());
  }

  fn off_scroll_start(&mut self, f: impl IntoScrollEventHandler) {
    let handler = f.into_event_handler();
    self
      .events
      .on_scroll_start
      .retain(|existing| !existing.same_handler(&handler));
  }

  fn on_scroll_end(&mut self, f: impl IntoScrollEventHandler) {
    self.events.on_scroll_end.push(f.into_event_handler());
  }

  fn off_scroll_end(&mut self, f: impl IntoScrollEventHandler) {
    let handler = f.into_event_handler();
    self
      .events
      .on_scroll_end
      .retain(|existing| !existing.same_handler(&handler));
  }

  fn on_scroll_reach_top(&mut self, f: impl IntoScrollEventHandler) {
    self.events.on_scroll_reach_top.push(f.into_event_handler());
  }

  fn off_scroll_reach_top(&mut self, f: impl IntoScrollEventHandler) {
    let handler = f.into_event_handler();
    self
      .events
      .on_scroll_reach_top
      .retain(|existing| !existing.same_handler(&handler));
  }

  fn on_scroll_reach_bottom(&mut self, f: impl IntoScrollEventHandler) {
    self.events.on_scroll_reach_bottom.push(f.into_event_handler());
  }

  fn off_scroll_reach_bottom(&mut self, f: impl IntoScrollEventHandler) {
    let handler = f.into_event_handler();
    self
      .events
      .on_scroll_reach_bottom
      .retain(|existing| !existing.same_handler(&handler));
  }

  fn opacity(&mut self, value: f32) {
    self.set_opacity(value);
  }

  fn transform(&mut self, t: Transform2D) {
    self.transform = t;
  }

  fn transition(&mut self, spec: Transition) {
    self.push_transition(spec);
  }

  fn animation(&mut self, spec: Animation) {
    self.set_animation(spec);
  }

  fn scrollbar(&mut self, style: ScrollBarStyle) {
    self.scrollbar_style.set(Some(style));
  }

  fn scrollbar_hovered(&mut self, f: impl Fn(ScrollBarStyle) -> ScrollBarStyle + Send + Sync + 'static) {
    self.scrollbar_hovered_style = Some(Arc::new(f));
  }

  fn culling(&mut self, enabled: bool) {
    if let LayoutKind::ScrollModifier { culling, .. } = &mut self.layout_kind {
      *culling = enabled;
    }
  }

  fn hit_test(&mut self, behavior: HitTestBehavior) {
    self.hit_test = behavior;
  }

  fn pointer_events_none(&mut self) {
    self.hit_test = HitTestBehavior::None;
  }

  fn ref_element(&mut self, element_ref: impl Into<CoreElementRef>) {
    self.element_ref = Some(element_ref.into());
  }

  fn interactive(&mut self, state: InteractionState) {
    self.interaction = Some(state);
  }

  fn focusable(&mut self, focusable: bool) {
    self.focusable = focusable;
  }

  fn tab_index(&mut self, tab_index: i32) {
    self.tab_index = Some(tab_index);
  }

  fn button_kind(&mut self, kind: ButtonKind) {
    self.button_kind = Some(kind);
    self.focusable = true;
  }

  #[cfg(feature = "form")]
  fn name(&mut self, name: impl Into<Arc<str>>) {
    self.form_name = Some(name.into());
  }

  fn text_wrap(&mut self, wrap: bool) {
    self.text_wrap = wrap;
    self.layout_cache.invalidate();
  }

  fn text_overflow(&mut self, overflow: TextOverflow) {
    self.text_overflow = overflow;
    self.layout_cache.invalidate();
  }

  fn selectable(&mut self, selectable: bool) {
    match &self.node_kind {
      NodeKind::Text { state, .. } => state.set_selectable(selectable),
      #[cfg(feature = "markdown")]
      NodeKind::RichText { state, .. } => state.set_selectable(selectable),
      _ => {}
    }
  }

  fn text_transform_mode(&mut self, mode: TextTransformMode) {
    if let NodeKind::Text { transform_mode, .. } = &mut self.node_kind {
      *transform_mode = mode;
    }
  }

  fn text_variant(&mut self, typography_style: impl Into<TypographyStyle>) {
    if let NodeKind::Text { style, .. } = &mut self.node_kind {
      style.set_variant(typography_style);
      self.layout_cache.invalidate();
    }
  }

  fn text_color(&mut self, color: impl Into<TextColor>) {
    if let NodeKind::Text { style, .. } = &mut self.node_kind {
      style.set_color(color);
    }
  }

  fn text_shadow(&mut self, shadow: crate::layout::text_style::TextShadow) {
    if let NodeKind::Text { style, .. } = &mut self.node_kind {
      style.set_shadow(shadow);
    }
  }

  fn text_align(&mut self, align: impl Into<TextAlign>) {
    if let NodeKind::Text { style, .. } = &mut self.node_kind {
      style.set_text_align(align);
      self.layout_cache.invalidate();
    }
  }

  fn on_input(&mut self, f: impl IntoTextInputEventHandler) {
    if let NodeKind::TextInput { state, .. } = &self.node_kind {
      state.set_on_input(f);
    }
  }

  fn off_input(&mut self, f: impl IntoTextInputEventHandler) {
    if let NodeKind::TextInput { state, .. } = &self.node_kind {
      state.clear_on_input(f);
    }
  }

  fn placeholder(&mut self, placeholder: &str) {
    self.set_placeholder(placeholder);
  }

  fn text_input_overflow(&mut self, overflow: crate::node::node_kind::TextInputOverflow) {
    self.set_text_input_overflow(overflow);
  }

  fn text_input_mask(&mut self) {
    NodeUpdate::text_input_mask_char(self, '*');
  }

  fn text_input_mask_char(&mut self, mask: char) {
    if let NodeKind::TextInput { state, .. } = &self.node_kind {
      state.set_mask(Some(mask));
    }
  }

  fn text_input_unmask(&mut self) {
    if let NodeKind::TextInput { state, .. } = &self.node_kind {
      state.set_mask(None);
    }
  }

  fn text_input_style(&mut self, text_style: TextStyle) {
    self.set_text_input_style(text_style);
  }

  fn text_input_placeholder_style(&mut self, text_style: TextStyle) {
    self.set_text_input_placeholder_style(text_style);
  }

  fn text_input_align(&mut self, align: impl Into<TextAlign>) {
    self.set_text_input_align(align);
  }

  fn text_input_rows(&mut self, min_rows: usize, max_rows: usize) {
    self.set_text_input_rows(min_rows, max_rows);
  }

  fn text_input_min_rows(&mut self, min_rows: usize) {
    self.set_text_input_min_rows(min_rows);
  }

  fn text_input_max_rows(&mut self, max_rows: usize) {
    self.set_text_input_max_rows(max_rows);
  }

  fn text_input_rows_exact(&mut self, rows: usize) {
    self.set_text_input_rows_exact(rows);
  }

  fn range(&mut self, min: i32, max: i32) {
    if let Some(state) = self.slider_state() {
      state.set_range(min, max);
    }
  }

  fn range_f32(&mut self, min: f32, max: f32) {
    if let Some(state) = self.slider_state() {
      state.set_range_f32(min, max);
    }
  }

  fn slider_step(&mut self, step: f32) {
    if let Some(state) = self.slider_state() {
      state.set_step(step);
    }
  }

  fn slider_track_style(&mut self, style: SliderPartStyle) {
    if let Some(state) = self.slider_state() {
      state.set_track_style(style);
    }
  }

  fn slider_track_hovered_style(&mut self, style: SliderPartStyle) {
    if let Some(state) = self.slider_state() {
      state.set_track_hovered_style(style);
    }
  }

  fn slider_fill_style(&mut self, style: SliderPartStyle) {
    if let Some(state) = self.slider_state() {
      state.set_fill_style(style);
    }
  }

  fn slider_fill_hovered_style(&mut self, style: SliderPartStyle) {
    if let Some(state) = self.slider_state() {
      state.set_fill_hovered_style(style);
    }
  }

  fn slider_thumb_style(&mut self, style: SliderPartStyle) {
    if let Some(state) = self.slider_state() {
      state.set_thumb_style(style);
    }
  }

  fn slider_thumb_hovered_style(&mut self, style: SliderPartStyle) {
    if let Some(state) = self.slider_state() {
      state.set_thumb_hovered_style(style);
    }
  }

  fn checkbox_box_style(&mut self, style: CheckboxStyle) {
    if let Some(state) = self.checkbox_state() {
      state.set_style(style);
    }
  }

  fn checkbox_checked_box_style(&mut self, style: CheckboxStyle) {
    if let Some(state) = self.checkbox_state() {
      state.set_checked_style(style);
    }
  }

  fn checkbox_box_hovered_style(&mut self, style: CheckboxStyle) {
    if let Some(state) = self.checkbox_state() {
      state.set_hovered_style(style);
    }
  }

  fn checkbox_checked_box_hovered_style(&mut self, style: CheckboxStyle) {
    if let Some(state) = self.checkbox_state() {
      state.set_checked_hovered_style(style);
    }
  }

  fn clip(&mut self) {
    self.set_overflow_through_logical(Overflow::Hidden);
  }

  fn overflow_visible(&mut self) {
    self.set_overflow_through_logical(Overflow::Visible);
  }

  fn intrinsic(&mut self, width: f32, height: f32) {
    self.intrinsic_size = Some(Size::new(width, height));
  }

  fn with_scroll_state(&mut self, existing: crate::layout::layout_kind::ScrollState) {
    if let LayoutKind::ScrollModifier { state, .. } = &mut self.layout_kind {
      *state = existing;
    }
  }
}

#[allow(dead_code)]
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
      overlay_declaration: None,
      modal_declaration: None,
      layout_neutral: false,
      synthetic_role: None,
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
      selection_color: Guard::new(None),
      caret_mode: Guard::new(None),
      cursor: None,
      hit_test: HitTestBehavior::default(),
      #[cfg(feature = "image")]
      background_image: Guard::new(None),
      #[cfg(feature = "image")]
      background_size: BackgroundSize::default(),
      #[cfg(all(feature = "image", feature = "resources"))]
      background_resource_image: None,
      scrollbar_style: Guard::new(None),
      scrollbar_hovered_style: None,
      element_ref: None,
      drag_payload: None,
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

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.component_key = Some(key.into());
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

  #[cfg(feature = "markdown")]
  pub(crate) fn rich_text(spans: Vec<crate::layout::quad::RichTextSpan>) -> Self {
    let text = spans.iter().map(|span| span.text.as_str()).collect::<String>();
    Self::from_parts(
      LayoutKind::Leaf,
      NodeKind::RichText {
        state: TextState::new(),
        spans,
        transform_mode: TextTransformMode::default(),
      },
      vec![],
    )
    .with_text_content(&text)
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
    node
      .events
      .on_mouse_down
      .push(EventHandler::new(move |event: &MouseEvent| {
        if event.button == MouseButton::Left {
          toggle.toggle_open();
        }
      }));
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
  pub fn video(data: crate::images::ImageData) -> Self {
    let mut node = Self::from_parts(
      LayoutKind::Leaf,
      NodeKind::Video {
        data: data.clone(),
        fit: BackgroundSize::Contain,
      },
      vec![],
    );
    node.intrinsic_size = Some(Size::new(data.width() as f32, data.height() as f32));
    node
  }

  #[cfg(feature = "image")]
  pub(crate) fn set_video_fit(&mut self, next_fit: BackgroundSize) {
    if let NodeKind::Video { fit, .. } = &mut self.node_kind {
      *fit = next_fit;
    }
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

  pub(crate) fn selection_color(mut self, color: impl Into<TextColor>) -> Self {
    self.set_selection_color(color.into());
    self
  }

  fn set_selection_color(&mut self, color: TextColor) {
    self.selection_color.set(Some(color));
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

  pub fn on_click(mut self, f: impl IntoMouseEventHandler) -> Self {
    self.events.on_click.push(f.into_event_handler());
    self
  }

  pub fn off_click(mut self, f: impl IntoMouseEventHandler) -> Self {
    let handler = f.into_event_handler();
    self.events.on_click.retain(|existing| !existing.same_handler(&handler));
    self
  }

  pub fn on_mouse_click(mut self, button: MouseButton, f: impl IntoMouseEventHandler) -> Self {
    self.events.on_mouse_click.push((button, f.into_event_handler()));
    self
  }

  pub fn off_mouse_click(mut self, button: MouseButton, f: impl IntoMouseEventHandler) -> Self {
    let handler = f.into_event_handler();
    self
      .events
      .on_mouse_click
      .retain(|(handler_button, existing)| *handler_button != button || !existing.same_handler(&handler));
    self
  }

  pub fn on_dblclick(mut self, f: impl IntoMouseEventHandler) -> Self {
    self.events.on_dblclick.push(f.into_event_handler());
    self
  }

  pub fn off_dblclick(mut self, f: impl IntoMouseEventHandler) -> Self {
    let handler = f.into_event_handler();
    self
      .events
      .on_dblclick
      .retain(|existing| !existing.same_handler(&handler));
    self
  }

  pub fn on_mouse_down(mut self, f: impl IntoMouseEventHandler) -> Self {
    self.events.on_mouse_down.push(f.into_event_handler());
    self
  }

  pub fn off_mouse_down(mut self, f: impl IntoMouseEventHandler) -> Self {
    let handler = f.into_event_handler();
    self
      .events
      .on_mouse_down
      .retain(|existing| !existing.same_handler(&handler));
    self
  }

  pub fn on_mouse_up(mut self, f: impl IntoMouseEventHandler) -> Self {
    self.events.on_mouse_up.push(f.into_event_handler());
    self
  }

  pub fn off_mouse_up(mut self, f: impl IntoMouseEventHandler) -> Self {
    let handler = f.into_event_handler();
    self
      .events
      .on_mouse_up
      .retain(|existing| !existing.same_handler(&handler));
    self
  }

  pub fn on_mouse_move(mut self, f: impl IntoMouseEventHandler) -> Self {
    self.events.on_mouse_move.push(f.into_event_handler());
    self
  }

  pub fn off_mouse_move(mut self, f: impl IntoMouseEventHandler) -> Self {
    let handler = f.into_event_handler();
    self
      .events
      .on_mouse_move
      .retain(|existing| !existing.same_handler(&handler));
    self
  }

  pub fn start_drag_buttons(mut self, buttons: MouseButtonMask) -> Self {
    self.events.start_drag_buttons = buttons;
    self
  }

  pub fn on_drag_start(mut self, f: impl IntoDragEventHandler) -> Self {
    self.events.on_drag_start.push(f.into_event_handler());
    self
  }

  pub fn off_drag_start(mut self, f: impl IntoDragEventHandler) -> Self {
    let handler = f.into_event_handler();
    self
      .events
      .on_drag_start
      .retain(|existing| !existing.same_handler(&handler));
    self
  }

  pub fn on_drag_move(mut self, f: impl IntoDragEventHandler) -> Self {
    self.events.on_drag_move.push(f.into_event_handler());
    self
  }

  pub fn off_drag_move(mut self, f: impl IntoDragEventHandler) -> Self {
    let handler = f.into_event_handler();
    self
      .events
      .on_drag_move
      .retain(|existing| !existing.same_handler(&handler));
    self
  }

  pub fn on_drag_end(mut self, f: impl IntoDragEventHandler) -> Self {
    self.events.on_drag_end.push(f.into_event_handler());
    self
  }

  pub fn off_drag_end(mut self, f: impl IntoDragEventHandler) -> Self {
    let handler = f.into_event_handler();
    self
      .events
      .on_drag_end
      .retain(|existing| !existing.same_handler(&handler));
    self
  }

  pub fn on_drop(mut self, f: impl IntoDropEventHandler) -> Self {
    self.events.on_drop.push(f.into_event_handler());
    self
  }

  pub fn off_drop(mut self, f: impl IntoDropEventHandler) -> Self {
    let handler = f.into_event_handler();
    self.events.on_drop.retain(|existing| !existing.same_handler(&handler));
    self
  }

  pub fn on_mouse_enter(mut self, f: impl IntoVoidEventHandler) -> Self {
    self.events.on_mouse_enter.push(f.into_void_event_handler());
    self
  }

  pub fn off_mouse_enter(mut self, f: impl IntoVoidEventHandler) -> Self {
    let handler = f.into_void_event_handler();
    self
      .events
      .on_mouse_enter
      .retain(|existing| !existing.same_handler(&handler));
    self
  }

  pub fn on_mouse_leave(mut self, f: impl IntoVoidEventHandler) -> Self {
    self.events.on_mouse_leave.push(f.into_void_event_handler());
    self
  }

  pub fn off_mouse_leave(mut self, f: impl IntoVoidEventHandler) -> Self {
    let handler = f.into_void_event_handler();
    self
      .events
      .on_mouse_leave
      .retain(|existing| !existing.same_handler(&handler));
    self
  }

  pub fn on_key_down(mut self, f: impl IntoKeyboardEventHandler) -> Self {
    self.events.on_key_down.push(f.into_event_handler());
    self
  }

  pub fn off_key_down(mut self, f: impl IntoKeyboardEventHandler) -> Self {
    let handler = f.into_event_handler();
    self
      .events
      .on_key_down
      .retain(|existing| !existing.same_handler(&handler));
    self
  }

  pub fn on_key_up(mut self, f: impl IntoKeyboardEventHandler) -> Self {
    self.events.on_key_up.push(f.into_event_handler());
    self
  }

  pub fn off_key_up(mut self, f: impl IntoKeyboardEventHandler) -> Self {
    let handler = f.into_event_handler();
    self
      .events
      .on_key_up
      .retain(|existing| !existing.same_handler(&handler));
    self
  }

  pub fn on_focus(mut self, f: impl IntoVoidEventHandler) -> Self {
    self.events.on_focus.push(f.into_void_event_handler());
    self
  }

  pub fn off_focus(mut self, f: impl IntoVoidEventHandler) -> Self {
    let handler = f.into_void_event_handler();
    self.events.on_focus.retain(|existing| !existing.same_handler(&handler));
    self
  }

  pub fn on_blur(mut self, f: impl IntoVoidEventHandler) -> Self {
    self.events.on_blur.push(f.into_void_event_handler());
    self
  }

  pub fn off_blur(mut self, f: impl IntoVoidEventHandler) -> Self {
    let handler = f.into_void_event_handler();
    self.events.on_blur.retain(|existing| !existing.same_handler(&handler));
    self
  }

  pub fn on_scroll(mut self, f: impl IntoScrollEventHandler) -> Self {
    self.events.on_scroll.push(f.into_event_handler());
    self
  }

  pub fn off_scroll(mut self, f: impl IntoScrollEventHandler) -> Self {
    let handler = f.into_event_handler();
    self
      .events
      .on_scroll
      .retain(|existing| !existing.same_handler(&handler));
    self
  }

  pub fn on_scroll_start(mut self, f: impl IntoScrollEventHandler) -> Self {
    self.events.on_scroll_start.push(f.into_event_handler());
    self
  }

  pub fn off_scroll_start(mut self, f: impl IntoScrollEventHandler) -> Self {
    let handler = f.into_event_handler();
    self
      .events
      .on_scroll_start
      .retain(|existing| !existing.same_handler(&handler));
    self
  }

  pub fn on_scroll_end(mut self, f: impl IntoScrollEventHandler) -> Self {
    self.events.on_scroll_end.push(f.into_event_handler());
    self
  }

  pub fn off_scroll_end(mut self, f: impl IntoScrollEventHandler) -> Self {
    let handler = f.into_event_handler();
    self
      .events
      .on_scroll_end
      .retain(|existing| !existing.same_handler(&handler));
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

  pub fn culling(mut self, enabled: bool) -> Self {
    if let LayoutKind::ScrollModifier { culling, .. } = &mut self.layout_kind {
      *culling = enabled;
    }
    self
  }

  pub fn hit_test(mut self, behavior: HitTestBehavior) -> Self {
    self.hit_test = behavior;
    self
  }

  pub fn pointer_events_none(mut self) -> Self {
    self.hit_test = HitTestBehavior::None;
    self
  }

  pub fn ref_element(mut self, element_ref: impl Into<CoreElementRef>) -> Self {
    self.element_ref = Some(element_ref.into());
    self
  }

  /// Attach data to drags starting on this node; the drop target's handler
  /// receives it through `DropEvent::payload`.
  pub fn drag_payload(mut self, payload: crate::app::events::DragPayload) -> Self {
    self.drag_payload = Some(payload);
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
    match &self.node_kind {
      NodeKind::Text { state, .. } => state.set_selectable(selectable),
      #[cfg(feature = "markdown")]
      NodeKind::RichText { state, .. } => state.set_selectable(selectable),
      _ => {}
    }
    self
  }

  #[cfg(feature = "markdown")]
  pub(crate) fn selectable_recursive(&mut self, selectable: bool) {
    match &self.node_kind {
      NodeKind::Text { state, .. } => state.set_selectable(selectable),
      NodeKind::RichText { state, .. } => state.set_selectable(selectable),
      _ => {}
    }
    for child in &mut self.children {
      child.selectable_recursive(selectable);
    }
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
      let placeholder_changed = state.placeholder().as_deref() != Some(placeholder);
      state.set_placeholder(placeholder);
      if placeholder_changed && state.value().is_empty() {
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
      if *style == text_style {
        return;
      }
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
      if placeholder_style.as_ref() == Some(&text_style) {
        return;
      }
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
      if style.text_align == align && placeholder_style.as_ref().is_none_or(|style| style.text_align == align) {
        return;
      }
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

  pub fn slider_fill_style(self, style: SliderPartStyle) -> Self {
    if let Some(state) = self.slider_state() {
      state.set_fill_style(style);
    }
    self
  }

  pub fn slider_fill_hovered_style(self, style: SliderPartStyle) -> Self {
    if let Some(state) = self.slider_state() {
      state.set_fill_hovered_style(style);
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

  pub(crate) fn set_overlay_declaration(&mut self, spec: crate::app::ctx::OverlaySpec) {
    self.overlay_declaration = Some(Box::new(spec));
  }

  pub(crate) fn set_modal_declaration(&mut self, spec: crate::app::ctx::ModalSpec) {
    self.modal_declaration = Some(Box::new(spec));
  }

  pub(crate) fn set_layout_neutral(&mut self, layout_neutral: bool) {
    self.layout_neutral = layout_neutral;
  }

  pub(crate) fn overlay_declaration(&self) -> Option<&crate::app::ctx::OverlaySpec> {
    self.overlay_declaration.as_deref()
  }

  pub(crate) fn modal_declaration(&self) -> Option<&crate::app::ctx::ModalSpec> {
    self.modal_declaration.as_deref()
  }

  pub(crate) fn is_overlay_declaration(&self) -> bool {
    self.layout_neutral || self.overlay_declaration.is_some() || self.modal_declaration.is_some()
  }

  pub(crate) fn set_synthetic_role(&mut self, role: SyntheticNodeRole) {
    self.synthetic_role = Some(role);
  }

  pub(crate) fn has_synthetic_role(&self, role: SyntheticNodeRole) -> bool {
    self.synthetic_role == Some(role)
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

  pub(crate) fn selection_color_value(&self) -> Option<TextColor> {
    <Option<TextColor> as Clone>::clone(&self.selection_color)
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

  pub(crate) fn hit_test_behavior(&self) -> HitTestBehavior {
    self.hit_test
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

  pub(crate) fn take_element_override_cleared(&self) -> bool {
    self
      .element_ref
      .as_ref()
      .is_some_and(CoreElementRef::take_override_cleared)
  }

  pub(crate) fn state_styles_affect_layout(&self) -> bool {
    self.state_styles.affects_layout()
  }

  pub(crate) fn take_style_layout_dirty(&self) -> bool {
    self.style_state.take_layout_dirty()
  }

  pub(crate) fn has_style_layout_dirty(&self) -> bool {
    self.style_state.has_layout_dirty()
  }

  pub(crate) fn has_render_dirty(&self) -> bool {
    self.text_content.is_changed()
      || self.color.is_changed()
      || self.gradient.is_changed()
      || self.border_radius.is_changed()
      || self.border.is_changed()
      || self.caret_color.is_changed()
      || self.selection_color.is_changed()
      || self.caret_mode.is_changed()
      || self.scrollbar_style.is_changed()
      || {
        #[cfg(feature = "image")]
        {
          self.background_image.is_changed()
        }
        #[cfg(not(feature = "image"))]
        {
          false
        }
      }
      || self.children.iter().any(Node::has_render_dirty)
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
    self.gradient.clear_changed();
    self.border_radius.clear_changed();
    self.border.clear_changed();
    self.caret_color.clear_changed();
    self.selection_color.clear_changed();
    self.caret_mode.clear_changed();
    #[cfg(feature = "image")]
    self.background_image.clear_changed();
    self.scrollbar_style.clear_changed();
    for child in &self.children {
      child.clear_guards();
    }
  }

  pub(crate) fn sync_dynamic_content_recursive(&mut self) {
    if let NodeKind::TextInput { state, .. } = &self.node_kind {
      state.sync_external_value();
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
    let own_layout_signature_matches = self.own_layout_signature_matches(old);
    if own_layout_signature_matches && self.children.len() == old.children.len() {
      self.layout_cache.preserve_from(&old.layout_cache);
      if !self.child_layout_signatures_match(old) {
        self.layout_cache.mark_descendant_dirty();
      }
    }

    match (&self.node_kind, &old.node_kind) {
      (NodeKind::Text { state, .. }, NodeKind::Text { state: old_state, .. }) => {
        state.copy_runtime_state_from(
          old_state,
          self.text_content().unwrap_or_default(),
          own_layout_signature_matches,
        );
      }
      #[cfg(feature = "markdown")]
      (NodeKind::RichText { state, .. }, NodeKind::RichText { state: old_state, .. }) => {
        state.copy_runtime_state_from(
          old_state,
          self.text_content().unwrap_or_default(),
          own_layout_signature_matches,
        );
      }
      (NodeKind::TextInput { state, .. }, NodeKind::TextInput { state: old_state, .. }) => {
        state.copy_runtime_state_from(old_state, own_layout_signature_matches);
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
      LayoutKind::ScrollModifier { state, direction, .. },
      LayoutKind::ScrollModifier {
        state: old_state,
        direction: old_direction,
        ..
      },
    ) = (&mut self.layout_kind, &old.layout_kind)
    {
      if direction == old_direction {
        *state = old_state.clone();
      }
    }

    for index in 0..self.children.len() {
      let child_slot_id = self.children[index].component_slot_id;
      let old_child = old
        .children
        .get(index)
        .filter(|old_child| self.children[index].can_preserve_runtime_state_from(old_child))
        .or_else(|| {
          child_slot_id.and_then(|slot_id| {
            old
              .children
              .iter()
              .find(|old_child| old_child.component_slot_id == Some(slot_id))
              .filter(|old_child| self.children[index].can_preserve_runtime_state_from(old_child))
          })
        });

      if let Some(old_child) = old_child {
        self.children[index].preserve_runtime_state_from(old_child);
      }
    }
  }

  fn can_preserve_runtime_state_from(&self, old: &Node) -> bool {
    std::mem::discriminant(&self.node_kind) == std::mem::discriminant(&old.node_kind)
      && std::mem::discriminant(&self.layout_kind) == std::mem::discriminant(&old.layout_kind)
      && self.component_slot_id == old.component_slot_id
      && self.component_key == old.component_key
  }

  fn clear_unchanged_guard_flags_from(&self, old: &Node) {
    if self.text_content.as_ref() == old.text_content.as_ref() {
      self.text_content.clear_changed();
    }
    if self.color.as_ref() == old.color.as_ref() {
      self.color.clear_changed();
    }
    if self.gradient.as_ref() == old.gradient.as_ref() {
      self.gradient.clear_changed();
    }
    if self.border_radius.as_ref() == old.border_radius.as_ref() {
      self.border_radius.clear_changed();
    }
    if self.border.as_ref() == old.border.as_ref() {
      self.border.clear_changed();
    }
    if self.caret_color.as_ref() == old.caret_color.as_ref() {
      self.caret_color.clear_changed();
    }
    if self.selection_color.as_ref() == old.selection_color.as_ref() {
      self.selection_color.clear_changed();
    }
    if self.caret_mode.as_ref() == old.caret_mode.as_ref() {
      self.caret_mode.clear_changed();
    }
  }

  fn layout_signature_matches(&self, old: &Node) -> bool {
    self.own_layout_signature_matches(old)
      && self.children.len() == old.children.len()
      && self.child_layout_signatures_match(old)
  }

  fn child_layout_signatures_match(&self, old: &Node) -> bool {
    self
      .children
      .iter()
      .zip(old.children.iter())
      .all(|(child, old_child)| child.layout_signature_matches(old_child))
  }

  fn own_layout_signature_matches(&self, old: &Node) -> bool {
    self.component_slot_id == old.component_slot_id
      && self.component_key == old.component_key
      && self.layout_kind_matches_for_cache(old)
      && self.node_kind_matches_for_cache(old)
      && self.frame == old.frame
      && self.padding == old.padding
      && self.position == old.position
      && self.offset == old.offset
      && self.align_self == old.align_self
      && self.flex == old.flex
      && self.text_content.as_ref() == old.text_content.as_ref()
      && self.text_wrap == old.text_wrap
      && self.text_overflow == old.text_overflow
      && self.overflow == old.overflow
      && self.intrinsic_size == old.intrinsic_size
      && self.animation_overrides.is_empty()
      && old.animation_overrides.is_empty()
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
      #[cfg(feature = "markdown")]
      (
        NodeKind::RichText {
          state,
          spans,
          transform_mode,
        },
        NodeKind::RichText {
          state: old_state,
          spans: old_spans,
          transform_mode: old_transform_mode,
        },
      ) => spans == old_spans && state.selectable() == old_state.selectable() && transform_mode == old_transform_mode,
      (
        NodeKind::TextInput {
          state,
          style,
          placeholder_style,
        },
        NodeKind::TextInput {
          state: old_state,
          style: old_style,
          placeholder_style: old_placeholder_style,
        },
      ) => {
        style == old_style
          && placeholder_style == old_placeholder_style
          && state.layout_signature() == old_state.layout_signature()
      }
      (NodeKind::Checkbox { state }, NodeKind::Checkbox { state: old_state }) => {
        state.layout_signature() == old_state.layout_signature()
      }
      (NodeKind::Slider { state }, NodeKind::Slider { state: old_state }) => {
        state.layout_signature() == old_state.layout_signature()
      }
      #[cfg(feature = "image")]
      (NodeKind::Image { data }, NodeKind::Image { data: old_data }) => data.id() == old_data.id(),
      #[cfg(feature = "image")]
      (
        NodeKind::Video { data, fit },
        NodeKind::Video {
          data: old_data,
          fit: old_fit,
        },
      ) => data.id() == old_data.id() && fit == old_fit,
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

    // Keyed/slotted children can move between renders (a windowed or reordered
    // list). Match them to their old twin by identity so node ids — and the
    // hover/active/focus state the runtime tracks by id — follow the element
    // instead of its position. Subtrees with no such children keep the cheap
    // positional zip. This mirrors `preserve_runtime_state_from`, which already
    // reconciles moved children by slot id.
    let reorderable = self
      .children
      .iter()
      .any(|child| child.component_key.is_some() || child.component_slot_id.is_some());
    if !reorderable {
      for (child, old_child) in self.children.iter_mut().zip(old.children.iter_mut()) {
        child.preserve_ids_from(old_child);
      }
      return;
    }

    let mut claimed = vec![false; old.children.len()];
    for (index, child) in self.children.iter_mut().enumerate() {
      let matched = old
        .children
        .get(index)
        .filter(|old_child| !claimed[index] && child.can_reuse_id_from(old_child))
        .map(|_| index)
        .or_else(|| {
          old
            .children
            .iter()
            .enumerate()
            .find_map(|(offset, old_child)| (!claimed[offset] && child.identity_matches(old_child)).then_some(offset))
        });
      if let Some(offset) = matched {
        claimed[offset] = true;
        child.preserve_ids_from(&mut old.children[offset]);
      }
    }
  }

  fn can_reuse_id_from(&self, old: &Node) -> bool {
    self.component_slot_id == old.component_slot_id
      && self.component_key == old.component_key
      && std::mem::discriminant(&self.node_kind) == std::mem::discriminant(&old.node_kind)
      && std::mem::discriminant(&self.layout_kind) == std::mem::discriminant(&old.layout_kind)
  }

  /// A moved keyed/slotted child's twin in the old children: only nodes that
  /// carry a stable identity (key or component slot) match across a reorder;
  /// unkeyed nodes fall back to positional pairing in `preserve_ids_from`.
  fn identity_matches(&self, old: &Node) -> bool {
    (self.component_key.is_some() || self.component_slot_id.is_some())
      && self.component_slot_id == old.component_slot_id
      && self.component_key == old.component_key
      && std::mem::discriminant(&self.node_kind) == std::mem::discriminant(&old.node_kind)
      && std::mem::discriminant(&self.layout_kind) == std::mem::discriminant(&old.layout_kind)
  }

  pub(crate) fn clone_for_reuse(&self) -> Self {
    Self {
      node_id: NodeId::UNASSIGNED,
      tag_name: self.tag_name.clone(),
      component_slot_id: self.component_slot_id,
      component_key: self.component_key.clone(),
      overlay_declaration: self
        .overlay_declaration
        .as_ref()
        .map(|spec| Box::new(spec.clone_for_reuse())),
      modal_declaration: self
        .modal_declaration
        .as_ref()
        .map(|spec| Box::new(spec.clone_for_reuse())),
      layout_neutral: self.layout_neutral,
      synthetic_role: self.synthetic_role,
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
      selection_color: self.selection_color.clone(),
      caret_mode: self.caret_mode.clone(),
      cursor: self.cursor,
      hit_test: self.hit_test,
      #[cfg(feature = "image")]
      background_image: self.background_image.clone(),
      #[cfg(feature = "image")]
      background_size: self.background_size,
      #[cfg(all(feature = "image", feature = "resources"))]
      background_resource_image: self.background_resource_image.clone(),
      scrollbar_style: self.scrollbar_style.clone(),
      scrollbar_hovered_style: self.scrollbar_hovered_style.clone(),
      element_ref: self.element_ref.clone(),
      drag_payload: self.drag_payload.clone(),
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

    if let Some(spec) = self.modal_declaration.as_mut()
      && spec.node.replace_component_slot_in(slot_id, replacement)
    {
      return true;
    }

    if let Some(spec) = self.overlay_declaration.as_mut()
      && spec.node.replace_component_slot_in(slot_id, replacement)
    {
      return true;
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

#[cfg(test)]
mod tests {
  use super::Node;
  use crate::{
    core::Signal,
    layout::{
      Alignment, Constraints, Offset, Size,
      layout_result::{ChildLayout, LayoutResult},
    },
  };

  #[test]
  fn changed_text_content_does_not_match_layout_cache_signature() {
    let old = Node::text("Hi");
    let new = Node::text("Vitayu vitayu vitayu");

    assert!(!new.layout_signature_matches(&old));
  }

  #[test]
  fn unchanged_text_content_matches_layout_cache_signature() {
    let old = Node::text("Hi");
    let new = Node::text("Hi");

    assert!(new.layout_signature_matches(&old));
  }

  #[test]
  fn changed_component_key_does_not_match_layout_cache_signature() {
    let mut old = Node::text("Hi");
    old.set_component_slot_id(1);
    old.set_component_key(Some("message-1"));
    let mut new = Node::text("Hi");
    new.set_component_slot_id(2);
    new.set_component_key(Some("message-2"));

    assert!(!new.layout_signature_matches(&old));
  }

  #[test]
  fn changed_slider_value_matches_layout_cache_signature() {
    let old_value = Signal::new(10);
    let new_value = Signal::new(20);
    let old = Node::slider(old_value);
    let new = Node::slider(new_value);

    assert!(new.layout_signature_matches(&old));
  }

  #[test]
  fn parent_layout_cache_survives_changed_child_text() {
    let old = Node::row(
      0.0,
      Alignment::Start,
      vec![Node::text("Value: 1"), Node::text("Stable")],
    );
    old.layout_cache.store(
      Constraints::loose(Size::new(400.0, 400.0)),
      LayoutResult {
        size: Size::new(100.0, 20.0),
        children: vec![
          ChildLayout {
            offset: Offset::default(),
            result: LayoutResult {
              size: Size::new(50.0, 20.0),
              children: Vec::new(),
            }
            .into(),
          },
          ChildLayout {
            offset: Offset::new(50.0, 0.0),
            result: LayoutResult {
              size: Size::new(50.0, 20.0),
              children: Vec::new(),
            }
            .into(),
          },
        ],
      },
    );
    let mut new = Node::row(
      0.0,
      Alignment::Start,
      vec![Node::text("Value: 2"), Node::text("Stable")],
    );

    new.preserve_runtime_state_from(&old);

    assert!(new.layout_cache.has_cached_result());
    assert!(!new.children[0].layout_cache.has_cached_result());
  }

  #[test]
  fn preserved_parent_cache_is_dirty_when_child_signature_changes() {
    let old = Node::column(
      0.0,
      Alignment::Start,
      vec![Node::row(0.0, Alignment::Start, vec![Node::text("Old")])],
    );
    old.layout_cache.store(
      Constraints::loose(Size::new(400.0, 400.0)),
      LayoutResult {
        size: Size::new(100.0, 20.0),
        children: vec![ChildLayout {
          offset: Offset::default(),
          result: LayoutResult {
            size: Size::new(100.0, 20.0),
            children: vec![ChildLayout {
              offset: Offset::default(),
              result: LayoutResult {
                size: Size::new(30.0, 20.0),
                children: Vec::new(),
              }
              .into(),
            }],
          }
          .into(),
        }],
      },
    );

    let mut new = Node::column(
      0.0,
      Alignment::Start,
      vec![Node::row(0.0, Alignment::Start, vec![Node::text("Different")])],
    );

    new.preserve_runtime_state_from(&old);

    assert!(new.layout_cache.has_cached_result());
    assert!(new.layout_cache.is_descendant_dirty());
  }

  #[test]
  fn preserved_parent_cache_is_dirty_when_child_component_key_changes() {
    let mut old_child = Node::row(0.0, Alignment::Start, vec![Node::text("Same text")]);
    old_child.set_component_slot_id(1);
    old_child.set_component_key(Some("message-1"));
    let old = Node::column(0.0, Alignment::Start, vec![old_child]);
    old.layout_cache.store(
      Constraints::loose(Size::new(400.0, 400.0)),
      LayoutResult {
        size: Size::new(100.0, 20.0),
        children: vec![ChildLayout {
          offset: Offset::default(),
          result: LayoutResult {
            size: Size::new(100.0, 20.0),
            children: Vec::new(),
          }
          .into(),
        }],
      },
    );

    let mut new_child = Node::row(0.0, Alignment::Start, vec![Node::text("Same text")]);
    new_child.set_component_slot_id(2);
    new_child.set_component_key(Some("message-2"));
    let mut new = Node::column(0.0, Alignment::Start, vec![new_child]);

    new.preserve_runtime_state_from(&old);

    assert!(new.layout_cache.has_cached_result());
    assert!(new.layout_cache.is_descendant_dirty());
  }

  #[test]
  fn preserve_ids_follows_keyed_children_across_a_window_shift() {
    use crate::core::{IdGenerator, NodeId};

    fn keyed_row(key: &str) -> Node {
      let mut wrapper = Node::column(0.0, Alignment::Start, vec![Node::text(key)]);
      wrapper.set_component_key(Some(key));
      wrapper
    }
    fn id_for(parent: &Node, key: &str) -> NodeId {
      parent
        .children
        .iter()
        .find(|child| child.component_key() == Some(key))
        .expect("row present")
        .node_id()
    }
    fn collect_ids(node: &Node, out: &mut Vec<NodeId>) {
      out.push(node.node_id());
      for child in &node.children {
        collect_ids(child, out);
      }
    }

    let id_gen = IdGenerator::new();
    // Old window: rows a, b, c.
    let mut old = Node::column(
      0.0,
      Alignment::Start,
      vec![keyed_row("a"), keyed_row("b"), keyed_row("c")],
    );
    old.assign_ids(&id_gen);
    let old_b = id_for(&old, "b");
    let old_c = id_for(&old, "c");

    // New window scrolled by one: rows b, c, d — b and c each moved up a slot.
    let mut new = Node::column(
      0.0,
      Alignment::Start,
      vec![keyed_row("b"), keyed_row("c"), keyed_row("d")],
    );
    new.preserve_ids_from(&mut old);
    new.assign_ids(&id_gen);

    // b and c keep their node ids despite moving position, so hover/focus
    // tracked by id stays glued to the same rows instead of jumping.
    assert_eq!(id_for(&new, "b"), old_b);
    assert_eq!(id_for(&new, "c"), old_c);

    // No id collision between preserved and freshly assigned nodes.
    let mut ids = Vec::new();
    collect_ids(&new, &mut ids);
    let unique: std::collections::HashSet<u64> = ids.iter().map(|id| id.value()).collect();
    assert_eq!(unique.len(), ids.len());
  }
}
