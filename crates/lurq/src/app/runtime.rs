#[cfg(feature = "devtools")]
use std::sync::Mutex;
use std::{
  sync::Arc,
  time::{Duration, Instant},
};

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

#[cfg(feature = "devtools")]
use crate::app::devtools::{
  DevTools, DevToolsBoolCallback, DevToolsDebugOverlayCallback, DevToolsProps, DevToolsSnapshot,
};
#[cfg(feature = "form")]
use crate::node::ButtonKind;
use crate::{
  animation::{AnimationEngine, Keyframes, TransitionEngine},
  app::{
    app_state::App,
    component::Component,
    ctx::{Ctx, component_tag_name},
    events::{
      DragEvent, DropEvent, DropResult, KeyboardEvent, MouseButton, MouseEvent, MouseEventKind, ScrollEvent,
      ScrollPhase,
    },
    hit_test::{hit_test_tree, hit_test_tree_all},
    profiler::{FrameProfile, PerfMeterStats, ProfileScope, RuntimeMemoryProfile},
    render_engine::{RenderEngine, RenderEngineFactory},
  },
  core::{ElementRect, ElementRef as OwnedElementRef, ElementRefMut as OwnedElementRefMut, IdGenerator, NodeId},
  layout::{
    Constraints, Size,
    layout_engine::LayoutEngine,
    layout_kind::{LayoutKind, ScrollAxis, ScrollDirection, ScrollState},
    layout_result::LayoutResult,
    quad::{ClipRect, Quad, QuadContent},
    render_list::{GlyphCmd, RectCmd, RenderGradient, RenderList},
    text_style::{FontWeight, TextStyle},
  },
  node::{
    Element, ElementRef, Node, TextTransformMode,
    border::{BorderPlacement, BorderRadius, ResolvedBorder, ResolvedBorders, ThemedBorderRadius},
    color::Color,
    cursor::CursorIcon,
    node_kind::{NodeKind, SliderState, TextInputOverflow, TextInputState, TextState},
    radius_value::RadiusValue,
    transform::Transform2D,
  },
};

const DOUBLE_CLICK_INTERVAL: Duration = Duration::from_millis(500);
const DOUBLE_CLICK_DISTANCE: f32 = 4.0;
const SUPPRESSED_CLICK_INTERVAL: Duration = Duration::from_millis(250);
const SUPPRESSED_CLICK_DISTANCE: f32 = 4.0;
const PERF_SAMPLE_INTERVAL: Duration = Duration::from_secs(1);
const TRANSPARENT_COLOR: Color = Color::new(0, 0, 0, 0);
const DEFAULT_CLEAR_COLOR: Color = Color::new(255, 255, 255, 255);
const DEFAULT_SLIDER_THUMB_MIN_SIZE: f32 = 12.0;
const MODAL_HOST_TAG: &str = "__lurq_modal_host";
#[cfg(feature = "devtools")]
const DEVTOOLS_SYNC_INTERVAL: Duration = Duration::from_millis(100);
#[cfg(feature = "devtools")]
const DEVTOOLS_INTERACTION_SYNC_DELAY: Duration = Duration::from_millis(250);

#[cfg(feature = "clipboard")]
fn read_clipboard_text() -> Option<String> {
  crate::clipboard::read_from_clipboard()
}

#[cfg(not(feature = "clipboard"))]
fn read_clipboard_text() -> Option<String> {
  None
}

#[cfg(feature = "clipboard")]
fn write_clipboard_text(text: &str) -> bool {
  crate::clipboard::copy_to_clipboard(text)
}

#[cfg(not(feature = "clipboard"))]
fn write_clipboard_text(_text: &str) -> bool {
  false
}

#[cfg(feature = "devtools")]
fn set_component_debug_metadata(node: &mut Node, ctx: &Ctx) {
  node.set_component_props_debug(ctx.props_debug());
  node.set_component_signals_debug(ctx.signals_debug());
  node.set_component_effects_debug(ctx.effects_debug());
  node.set_component_contexts_debug(ctx.contexts_debug());
}

trait AnyRootComponent: Send + Sync {
  fn render(&self, ctx: &mut Ctx) -> Element;
  fn on_mounted(&self);
  fn on_unmounted(&self);
  fn tag_name(&self) -> Arc<str>;
}

struct RootComponentWrapper<C: Component> {
  component: C,
}

impl<C: Component> AnyRootComponent for RootComponentWrapper<C> {
  fn render(&self, ctx: &mut Ctx) -> Element {
    self.component.render(ctx).into()
  }

  fn on_mounted(&self) {
    self.component.on_mounted();
  }

  fn on_unmounted(&self) {
    self.component.on_unmounted();
  }

  fn tag_name(&self) -> Arc<str> {
    component_tag_name::<C>()
  }
}

pub struct Tree {
  id_gen: IdGenerator,
  layout_engine: LayoutEngine,
  render_engine: Option<Box<dyn RenderEngine>>,
  render_engine_factory: Option<RenderEngineFactory>,
  root: Option<Node>,
  root_component: Option<Box<dyn AnyRootComponent>>,
  root_ctx: Option<Ctx>,
  last_layout: Option<LayoutResult>,
  layout_constraints_override: Option<Constraints>,
  viewport_physical: Size,
  scale_factor: f32,
  window: crate::app::window::Window,
  hover_path: Vec<NodeId>,
  active_path: Vec<NodeId>,
  dragging_scroll: Option<ScrollDrag>,
  dragging_slider: Option<SliderDrag>,
  dragging_text_selection: Option<TextSelectionDrag>,
  active_drag: Option<ActiveDrag>,
  focused_node: Option<NodeId>,
  focused_event_node: Option<NodeId>,
  focused_path: Option<Vec<usize>>,
  focused_event_path: Option<Vec<usize>>,
  cursor: CursorIcon,
  click_tracker: ClickTracker,
  text_click_tracker: TextClickTracker,
  click_press: Option<ClickPress>,
  suppressed_click: Option<SuppressedClick>,
  needs_redraw: bool,
  perf_overlay_enabled: bool,
  perf_overlay_stats: PerfMeterStats,
  perf_overlay_last_sample: Instant,
  perf_overlay_last_seen_frame: u64,
  perf_overlay_frames_since_sample: u64,
  secondary_windows: Vec<SecondaryWindow>,
  #[cfg(feature = "devtools")]
  pub(crate) devtools: Option<DevToolsWindow>,
  #[cfg(feature = "devtools")]
  devtools_state: DevToolsState,
  #[cfg(feature = "devtools")]
  debug_overlay_node_path: Option<Vec<usize>>,
  frame_count: u64,
  last_profile: FrameProfile,
  transition_engine: TransitionEngine,
  animation_engine: AnimationEngine,
  last_theme_version: u64,
}

#[cfg_attr(not(feature = "winit"), allow(dead_code))]
pub(crate) struct SecondaryWindow {
  title: String,
  width: u32,
  height: u32,
  tree: Tree,
  open: bool,
  metadata: SecondaryWindowMetadata,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SecondaryWindowMetadata {
  pub(crate) window_id: Option<String>,
  pub(crate) raw_window_handle: Option<String>,
  pub(crate) raw_display_handle: Option<String>,
  pub(crate) hwnd: Option<isize>,
}

#[cfg(feature = "devtools")]
#[cfg_attr(not(feature = "winit"), allow(dead_code))]
pub(crate) struct DevToolsWindow {
  secondary_index: usize,
  pub(crate) metadata: SecondaryWindowMetadata,
}

#[cfg_attr(not(feature = "winit"), allow(dead_code))]
impl SecondaryWindow {
  fn new(title: impl Into<String>, width: u32, height: u32, tree: Tree) -> Self {
    Self {
      title: title.into(),
      width,
      height,
      tree,
      open: true,
      metadata: SecondaryWindowMetadata::default(),
    }
  }

  pub(crate) fn title(&self) -> &str {
    &self.title
  }

  pub(crate) fn width(&self) -> u32 {
    self.width
  }

  pub(crate) fn height(&self) -> u32 {
    self.height
  }

  pub(crate) fn tree(&self) -> &Tree {
    &self.tree
  }

  pub(crate) fn tree_mut(&mut self) -> &mut Tree {
    &mut self.tree
  }

  pub(crate) fn set_metadata(&mut self, metadata: SecondaryWindowMetadata) {
    self.metadata = metadata;
  }
}

#[cfg(feature = "devtools")]
struct DevToolsState {
  debug_overlay_path: Arc<Mutex<Option<Vec<usize>>>>,
  overlay_enabled: Arc<Mutex<bool>>,
  pick_mode: Arc<Mutex<bool>>,
  picked_path: Option<Vec<usize>>,
  picked_revision: u64,
  last_sync: Instant,
  last_input_interaction: Instant,
}

#[cfg(feature = "devtools")]
impl Default for DevToolsState {
  fn default() -> Self {
    Self {
      debug_overlay_path: Arc::new(Mutex::new(None)),
      overlay_enabled: Arc::new(Mutex::new(true)),
      pick_mode: Arc::new(Mutex::new(false)),
      picked_path: None,
      picked_revision: 0,
      last_sync: Instant::now() - DEVTOOLS_SYNC_INTERVAL,
      last_input_interaction: Instant::now() - DEVTOOLS_INTERACTION_SYNC_DELAY,
    }
  }
}

#[cfg(feature = "devtools")]
fn devtools_debug_overlay_callback(debug_overlay_path: Arc<Mutex<Option<Vec<usize>>>>) -> DevToolsDebugOverlayCallback {
  Arc::new(move |path| {
    *debug_overlay_path.lock().unwrap() = path;
  })
}

#[cfg(feature = "devtools")]
fn devtools_bool_callback(value: Arc<Mutex<bool>>) -> DevToolsBoolCallback {
  Arc::new(move |enabled| {
    *value.lock().unwrap() = enabled;
  })
}

impl Default for Tree {
  fn default() -> Self {
    Self::new()
  }
}

impl Tree {
  pub fn new() -> Self {
    let tree = Self {
      id_gen: IdGenerator::new(),
      layout_engine: LayoutEngine::new(),
      render_engine: None,
      render_engine_factory: None,
      root: None,
      root_component: None,
      root_ctx: None,
      last_layout: None,
      layout_constraints_override: None,
      viewport_physical: Size::new(800.0, 600.0),
      scale_factor: 1.0,
      window: crate::app::window::Window::new(),
      hover_path: Vec::new(),
      active_path: Vec::new(),
      dragging_scroll: None,
      dragging_slider: None,
      dragging_text_selection: None,
      active_drag: None,
      focused_node: None,
      focused_event_node: None,
      focused_path: None,
      focused_event_path: None,
      cursor: CursorIcon::Default,
      click_tracker: ClickTracker::default(),
      text_click_tracker: TextClickTracker::default(),
      click_press: None,
      suppressed_click: None,
      needs_redraw: false,
      perf_overlay_enabled: false,
      perf_overlay_stats: PerfMeterStats::default(),
      perf_overlay_last_sample: Instant::now(),
      perf_overlay_last_seen_frame: 0,
      perf_overlay_frames_since_sample: 0,
      secondary_windows: Vec::new(),
      #[cfg(feature = "devtools")]
      devtools: None,
      #[cfg(feature = "devtools")]
      devtools_state: DevToolsState::default(),
      #[cfg(feature = "devtools")]
      debug_overlay_node_path: None,
      frame_count: 0,
      last_profile: FrameProfile::default(),
      transition_engine: TransitionEngine::new(),
      animation_engine: AnimationEngine::new(),
      last_theme_version: u64::MAX,
    };
    tree
      .window
      .set_resolved_size(tree.viewport_physical.width, tree.viewport_physical.height);
    tree.window.set_scale_factor(tree.scale_factor);
    tree
  }

  pub fn scale_factor(&self) -> f32 {
    self.scale_factor
  }

  pub fn set_scale_factor(&mut self, scale: f32) {
    self.scale_factor = scale;
    self.window.set_scale_factor(scale);
  }

  /// The reactive window handle for this tree's window. The shell pushes
  /// position updates here; size and scale are kept in sync by `resize` and
  /// `set_scale_factor`.
  pub fn window(&self) -> &crate::app::window::Window {
    &self.window
  }

  pub fn set_window_position(&mut self, x: i32, y: i32) {
    self.window.set_position(x, y);
  }

  pub(crate) fn memory_profile_with_glyph(&self, glyph_engine_bytes: usize) -> RuntimeMemoryProfile {
    let runtime_struct_bytes = std::mem::size_of::<Self>();
    let root_tree_bytes = self.root.as_ref().map(Node::estimated_memory_bytes).unwrap_or(0);
    let root_context_bytes = self.root_ctx.as_ref().map(Ctx::estimated_memory_bytes).unwrap_or(0);
    let root_component_bytes = self
      .root_component
      .as_ref()
      .map(|_| std::mem::size_of::<Box<dyn AnyRootComponent>>())
      .unwrap_or(0);
    let last_layout_bytes = self
      .last_layout
      .as_ref()
      .map(LayoutResult::estimated_memory_bytes)
      .unwrap_or(0);
    let render_engine_bytes = self
      .render_engine
      .as_ref()
      .map(|_| std::mem::size_of::<Box<dyn RenderEngine>>())
      .unwrap_or(0);
    let hover_path_bytes = self.hover_path.capacity() * std::mem::size_of::<NodeId>();
    let active_path_bytes = self.active_path.capacity() * std::mem::size_of::<NodeId>();
    let dragging_scroll_bytes = self
      .dragging_scroll
      .as_ref()
      .map(|_| std::mem::size_of::<ScrollDrag>())
      .unwrap_or(0);
    let total_bytes = runtime_struct_bytes
      + root_tree_bytes
      + root_context_bytes
      + root_component_bytes
      + last_layout_bytes
      + glyph_engine_bytes
      + render_engine_bytes
      + hover_path_bytes
      + active_path_bytes
      + dragging_scroll_bytes;

    RuntimeMemoryProfile {
      total_bytes,
      runtime_struct_bytes,
      root_tree_bytes,
      root_context_bytes,
      root_component_bytes,
      last_layout_bytes,
      glyph_engine_bytes,
      render_engine_bytes,
      hover_path_bytes,
      active_path_bytes,
      dragging_scroll_bytes,
    }
  }

  pub fn last_profile(&self) -> &FrameProfile {
    &self.last_profile
  }

  pub fn draw_perf_overlay(&mut self) {
    if self.perf_overlay_enabled {
      return;
    }
    self.perf_overlay_enabled = true;
    self.perf_overlay_last_sample = Instant::now();
    self.perf_overlay_last_seen_frame = self.frame_count;
    self.perf_overlay_frames_since_sample = 0;
    self.needs_redraw = true;
  }

  pub fn request_redraw(&mut self) {
    self.needs_redraw = true;
  }

  pub fn tick_timers(&mut self) {
    self.tick_timers_at(Instant::now());
  }

  pub fn tick_timers_at(&mut self, now: Instant) {
    let fired = self.root_ctx.as_mut().is_some_and(|ctx| ctx.tick_timers(now));
    if fired {
      self.needs_redraw = true;
      self.apply_reactive_updates_after_event();
    }
  }

  pub fn tick_futures(&mut self) {
    let completed = self.root_ctx.as_mut().is_some_and(Ctx::tick_futures);
    if completed {
      self.needs_redraw = true;
      self.apply_reactive_updates_after_event();
    }
  }

  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn perf_overlay_enabled(&self) -> bool {
    self.perf_overlay_enabled
  }

  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn tick_perf_overlay(&mut self) {
    if self.perf_overlay_enabled && Instant::now().duration_since(self.perf_overlay_last_sample) >= PERF_SAMPLE_INTERVAL
    {
      self.needs_redraw = true;
    }
  }

  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn has_active_timeline(&self) -> bool {
    self.transition_engine.has_active || self.animation_engine.has_active
  }

  pub fn frame_count(&self) -> u64 {
    self.frame_count
  }

  fn viewport_logical(&self) -> Size {
    let s = self.scale_factor();
    Size::new(self.viewport_physical.width / s, self.viewport_physical.height / s)
  }

  pub fn set_render_engine_factory<F>(&mut self, factory: F)
  where
    F: Fn() -> Box<dyn RenderEngine> + 'static,
  {
    let factory: RenderEngineFactory = Arc::new(factory);
    self.render_engine = Some((factory)());
    self.render_engine_factory = Some(factory);
    self.apply_render_engine_factory_to_secondaries();
  }

  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn secondary_window_count(&self) -> usize {
    self.secondary_windows.len()
  }

  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn secondary_window(&self, index: usize) -> Option<&SecondaryWindow> {
    self.secondary_windows.get(index).filter(|window| window.open)
  }

  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn secondary_window_mut(&mut self, index: usize) -> Option<&mut SecondaryWindow> {
    self.secondary_windows.get_mut(index).filter(|window| window.open)
  }

  #[cfg_attr(not(any(feature = "winit", feature = "devtools")), allow(dead_code))]
  fn push_secondary_window(&mut self, mut window: SecondaryWindow) -> usize {
    let index = self.secondary_windows.len();
    self.apply_render_engine_factory_to_secondary(&mut window);
    self.secondary_windows.push(window);
    index
  }

  #[cfg_attr(not(any(feature = "winit", feature = "devtools")), allow(dead_code))]
  fn apply_render_engine_factory_to_secondary(&self, window: &mut SecondaryWindow) {
    if window.tree.render_engine.is_none()
      && let Some(factory) = &self.render_engine_factory
    {
      window.tree.render_engine = Some((factory)());
    }
  }

  fn apply_render_engine_factory_to_secondaries(&mut self) {
    let Some(factory) = self.render_engine_factory.clone() else {
      return;
    };
    for window in &mut self.secondary_windows {
      window.tree.render_engine = Some((factory)());
    }
  }

  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn set_secondary_window_metadata(&mut self, index: usize, metadata: SecondaryWindowMetadata) {
    let Some(window) = self.secondary_windows.get_mut(index) else {
      return;
    };
    window.set_metadata(metadata.clone());

    #[cfg(feature = "devtools")]
    if let Some(devtools) = &mut self.devtools
      && devtools.secondary_index == index
    {
      devtools.metadata = metadata;
    }
  }

  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn close_secondary_window(&mut self, index: usize) -> bool {
    let Some(window) = self.secondary_windows.get_mut(index) else {
      return false;
    };
    if !window.open {
      return false;
    }
    window.open = false;
    window.set_metadata(SecondaryWindowMetadata::default());

    #[cfg(feature = "devtools")]
    if self
      .devtools
      .as_ref()
      .is_some_and(|devtools| devtools.secondary_index == index)
    {
      if let Some(devtools) = &mut self.devtools {
        devtools.metadata = SecondaryWindowMetadata::default();
      }
      *self.devtools_state.debug_overlay_path.lock().unwrap() = None;
      *self.devtools_state.pick_mode.lock().unwrap() = false;
      return self.clear_debug_overlay();
    }

    false
  }

  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn apply_secondary_window_requests(&mut self) -> bool {
    #[cfg(feature = "devtools")]
    {
      return self.apply_devtools_requests();
    }
    #[cfg(not(feature = "devtools"))]
    false
  }

  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn secondary_pick_mode(&self) -> bool {
    #[cfg(feature = "devtools")]
    {
      return self.devtools_pick_mode();
    }
    #[cfg(not(feature = "devtools"))]
    false
  }

  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn pick_secondary_node_at(&mut self, _x: f32, _y: f32) -> Option<Vec<usize>> {
    #[cfg(feature = "devtools")]
    {
      return self.pick_devtools_node_at(_x, _y);
    }
    #[cfg(not(feature = "devtools"))]
    None
  }

  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn cancel_secondary_pick(&mut self) {
    #[cfg(feature = "devtools")]
    self.cancel_devtools_pick();
  }

  #[cfg(feature = "devtools")]
  pub fn mount_devtools(&mut self, app: &mut App) {
    let mut devtools = Tree::new();
    devtools.mount_root::<DevTools>(app, self.devtools_props(DevToolsSnapshot::from_tree(self)));
    let index = self.push_secondary_window(SecondaryWindow::new("lurq DevTools", 1440, 900, devtools));
    self.devtools = Some(DevToolsWindow {
      secondary_index: index,
      metadata: SecondaryWindowMetadata::default(),
    });
  }

  #[cfg(feature = "devtools")]
  fn devtools_tree_mut(&mut self) -> Option<&mut Tree> {
    let index = self.devtools.as_ref()?.secondary_index;
    self.secondary_window_mut(index).map(SecondaryWindow::tree_mut)
  }

  #[cfg(feature = "devtools")]
  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn devtools_pick_mode(&self) -> bool {
    *self.devtools_state.pick_mode.lock().unwrap()
  }

  #[cfg(feature = "devtools")]
  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn pick_devtools_node_at(&mut self, x: f32, y: f32) -> Option<Vec<usize>> {
    let picked_path = self.debug_overlay_node_at(x, y);
    *self.devtools_state.pick_mode.lock().unwrap() = false;
    self.devtools_state.picked_path = picked_path.clone();
    self.devtools_state.picked_revision = self.devtools_state.picked_revision.saturating_add(1);

    let overlay_enabled = *self.devtools_state.overlay_enabled.lock().unwrap();
    *self.devtools_state.debug_overlay_path.lock().unwrap() = if overlay_enabled { picked_path.clone() } else { None };
    self.apply_devtools_requests();
    self.sync_devtools_now();
    picked_path
  }

  #[cfg(feature = "devtools")]
  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn cancel_devtools_pick(&mut self) {
    *self.devtools_state.pick_mode.lock().unwrap() = false;
    self.devtools_state.picked_path = None;
    self.devtools_state.picked_revision = self.devtools_state.picked_revision.saturating_add(1);
    self.sync_devtools_now();
  }

  #[cfg(feature = "devtools")]
  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn apply_devtools_requests(&mut self) -> bool {
    let debug_overlay_path = self.devtools_state.debug_overlay_path.lock().unwrap().clone();
    match debug_overlay_path {
      Some(path) => self.draw_debug_overlay_over_node(path),
      None => self.clear_debug_overlay(),
    }
  }

  #[cfg(feature = "devtools")]
  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn sync_devtools_now(&mut self) {
    self.sync_devtools_inner(true);
  }

  #[cfg(feature = "devtools")]
  fn sync_devtools(&mut self) {
    self.sync_devtools_inner(false);
  }

  #[cfg(feature = "devtools")]
  fn sync_devtools_inner(&mut self, force: bool) {
    if self.devtools.is_none() {
      return;
    }

    let now = Instant::now();

    if !force && self.has_active_input_interaction() {
      self.devtools_state.last_input_interaction = now;
      return;
    }

    if !force && now.duration_since(self.devtools_state.last_input_interaction) < DEVTOOLS_INTERACTION_SYNC_DELAY {
      return;
    }

    if !force && now.duration_since(self.devtools_state.last_sync) < DEVTOOLS_SYNC_INTERVAL {
      return;
    }

    let props = self.devtools_props(DevToolsSnapshot::from_tree(self));
    let Some(devtools) = self.devtools_tree_mut() else {
      return;
    };
    devtools.update_root_props::<DevTools>(props);
    self.devtools_state.last_sync = now;
  }

  #[cfg(feature = "devtools")]
  fn has_active_input_interaction(&self) -> bool {
    self.dragging_scroll.is_some()
      || self.dragging_slider.is_some()
      || self.dragging_text_selection.is_some()
      || self.active_drag.is_some()
  }

  #[cfg(feature = "devtools")]
  fn devtools_props(&self, snapshot: DevToolsSnapshot) -> DevToolsProps {
    DevToolsProps {
      snapshot,
      picked_path: self.devtools_state.picked_path.clone(),
      picked_revision: self.devtools_state.picked_revision,
      on_debug_overlay_path: Some(devtools_debug_overlay_callback(
        self.devtools_state.debug_overlay_path.clone(),
      )),
      on_overlay_enabled: Some(devtools_bool_callback(self.devtools_state.overlay_enabled.clone())),
      on_pick_inspected: Some(devtools_bool_callback(self.devtools_state.pick_mode.clone())),
    }
  }

  fn clear_animation_runtime_state(&mut self) {
    self.transition_engine.clear_state();
    self.animation_engine.clear_state();
  }

  pub fn mount_root<C: Component>(&mut self, app: &mut App, props: C::Props) {
    self.clear_hover_path();
    if let Some(component) = self.root_component.take() {
      component.on_unmounted();
    }
    if let Some(old) = &mut self.root {
      reset_element_ref_flags_recursive(old);
      old.free_ids(&self.id_gen);
    }
    self.clear_animation_runtime_state();
    let mut ctx = Ctx::new_root()
      .with_theme(app.theme().clone())
      .with_window(self.window.clone());
    #[cfg(feature = "i18n")]
    {
      ctx = ctx.with_i18n(app.i18n().clone());
    }
    ctx.set_app_ref(app);
    ctx.set_root_props(props);
    let component = C::create(&mut ctx);
    let wrapper = RootComponentWrapper { component };
    ctx.begin_render();
    let mut node = wrapper.render(&mut ctx).node;
    ctx.end_render();
    node.set_tag_name(wrapper.tag_name());
    #[cfg(feature = "devtools")]
    set_component_debug_metadata(&mut node, &ctx);
    wrapper.on_mounted();
    self.root = Some(node);
    self.sync_modal_layer();
    if let Some(root) = &mut self.root {
      root.assign_ids(&self.id_gen);
    }
    self.root_component = Some(Box::new(wrapper));
    self.root_ctx = Some(ctx);
    self.last_layout = None;
    self.last_theme_version = u64::MAX;
    self.active_path.clear();
    self.clear_focus();
  }

  pub(crate) fn set_app_ref(&mut self, app: &mut App) {
    if let Some(ctx) = &mut self.root_ctx {
      ctx.set_app_ref(app);
    }
    for secondary in &mut self.secondary_windows {
      secondary.tree.set_app_ref(app);
    }
  }

  pub fn update_root_props<C: Component>(&mut self, props: C::Props) {
    let changed = self.root_ctx.as_mut().is_some_and(|ctx| ctx.update_root_props(props));
    if changed {
      self.needs_redraw = true;
    }
  }

  pub fn rebuild(&mut self) {
    if self.root_component.is_none() || self.root_ctx.is_none() {
      return;
    }
    if let (Some(component), Some(ctx)) = (&self.root_component, &mut self.root_ctx) {
      let mut old_root = self.root.take().map(|old| {
        let old = modal_host_base(old);
        reset_element_ref_flags_recursive(&old);
        old
      });
      ctx.begin_render();
      let mut node = component.render(ctx).node;
      ctx.end_render();
      node.set_tag_name(component.tag_name());
      #[cfg(feature = "devtools")]
      set_component_debug_metadata(&mut node, ctx);
      if let Some(old) = old_root.as_mut() {
        node.preserve_runtime_state_from(old);
        node.preserve_ids_from(old);
        old.free_ids(&self.id_gen);
      }
      self.root = Some(node);
      self.sync_modal_layer();
      if let Some(root) = &mut self.root {
        root.assign_ids(&self.id_gen);
      }
      self.refresh_interaction_state();
    }
  }

  pub fn set_root(&mut self, element: impl Into<Element>) {
    self.clear_hover_path();
    if let Some(component) = self.root_component.take() {
      component.on_unmounted();
    }
    let mut old_root = self.root.take();
    if let Some(old) = &mut old_root {
      reset_element_ref_flags_recursive(old);
    }
    self.clear_animation_runtime_state();
    let mut node = element.into().node;
    if let Some(old) = old_root.as_mut() {
      node.preserve_runtime_state_from(old);
      node.preserve_ids_from(old);
      old.free_ids(&self.id_gen);
    }
    node.assign_ids(&self.id_gen);
    self.root = Some(node);
    self.root_component = None;
    self.root_ctx = None;
    self.last_layout = None;
    self.last_theme_version = u64::MAX;
    self.active_path.clear();
    self.active_drag = None;
    self.clear_focus();
  }

  pub fn root(&self) -> Option<ElementRef<'_>> {
    self.root.as_ref().map(ElementRef::new)
  }

  pub fn find_element(&mut self, predicate: impl for<'a> Fn(ElementRef<'a>) -> bool) -> Option<OwnedElementRef> {
    let root = self.root.as_mut()?;
    let layout = self.last_layout.as_ref()?;
    find_element_recursive(root, layout, 0.0, 0.0, 0.0, 0.0, &predicate)
  }

  pub fn find_element_mut(&mut self, predicate: impl for<'a> Fn(ElementRef<'a>) -> bool) -> Option<OwnedElementRefMut> {
    self.find_element(predicate).map(|element_ref| element_ref.mutable())
  }

  pub fn id_gen(&self) -> &IdGenerator {
    &self.id_gen
  }

  pub fn resize(&mut self, width: u32, height: u32) {
    let size = Size::new(width as f32, height as f32);
    if self.viewport_physical == size {
      return;
    }

    self.viewport_physical = size;
    self.window.set_resolved_size(size.width, size.height);
    if let Some(engine) = &mut self.render_engine {
      engine.resize(width, height);
    }
  }

  pub fn pass(&mut self, app: &mut App, surface: &(impl HasWindowHandle + HasDisplayHandle)) {
    self.set_app_ref(app);
    let profiling_enabled = app.profiling_enabled;
    let frame_start = ProfileScope::maybe_start(profiling_enabled);
    let scale = self.scale_factor();
    if profiling_enabled {
      app.glyph_engine.reset_stats();
    }
    self.update_perf_overlay_stats();

    self.flush_due_pending_click(Instant::now());

    let layout_start = ProfileScope::maybe_start(profiling_enabled);
    self.update_layout(app);
    let layout_dur = ProfileScope::elapsed_or_default(&layout_start);
    let layout_recalculated = self.layout_engine.last_recalculated();

    let root = match &self.root {
      Some(r) => r,
      None => return,
    };
    if self.render_engine.is_none() {
      return;
    }
    let clear_color = root.color().unwrap_or(DEFAULT_CLEAR_COLOR);

    let window = surface.window_handle().unwrap();
    let display = surface.display_handle().unwrap();

    let result = match self.last_layout.take() {
      Some(result) => result,
      None => return,
    };

    let quad_start = ProfileScope::maybe_start(profiling_enabled);
    let viewport_clip = ClipRect {
      x: 0.0,
      y: 0.0,
      width: self.viewport_physical.width / scale,
      height: self.viewport_physical.height / scale,
      active: true,
    };
    let quads = self
      .layout_engine
      .resolve_quads_with_viewport(root, &result, viewport_clip);
    let quad_dur = ProfileScope::elapsed_or_default(&quad_start);
    let quad_count = quads.len();
    #[cfg(feature = "devtools")]
    let devtools_overlay = self.devtools_overlay_target(root, &result);

    self.last_layout = Some(result);

    let glyph_start = ProfileScope::maybe_start(profiling_enabled);
    let mut rects = Vec::with_capacity(quad_count);
    let mut glyphs = Vec::with_capacity(quad_count * 4);
    #[cfg(feature = "image")]
    let mut images = Vec::new();
    #[cfg(feature = "image")]
    let image_frame_time = std::time::Instant::now();
    #[cfg(all(feature = "svg", feature = "image"))]
    let svgs = Vec::new();
    #[cfg(all(feature = "svg", not(feature = "image")))]
    let mut svgs = Vec::new();

    for (order, quad) in quads.iter().enumerate() {
      let scaled_clip = if quad.clip.active {
        ClipRect {
          x: quad.clip.x * scale,
          y: quad.clip.y * scale,
          width: quad.clip.width * scale,
          height: quad.clip.height * scale,
          active: true,
        }
      } else {
        ClipRect::default()
      };

      let scaled_x = quad.x * scale;
      let scaled_y = quad.y * scale;
      let scaled_width = quad.width * scale;
      let scaled_height = quad.height * scale;
      if quad.transform.is_identity()
        && scaled_clip.active
        && !rect_intersects_clip(scaled_x, scaled_y, scaled_width, scaled_height, scaled_clip)
      {
        continue;
      }

      match &quad.content {
        QuadContent::Rect { color, gradient } => {
          let (x, y, w, h) = (quad.x * scale, quad.y * scale, quad.width * scale, quad.height * scale);
          let radii = scaled_radii(quad.border_radius, scale, w, h);
          let final_color = apply_opacity(*color, quad.opacity);
          let gradient = gradient
            .as_ref()
            .map(|gradient| apply_opacity_gradient(gradient, quad.opacity));
          let xf = quad.transform.matrix_2x2();
          let xf_origin = quad
            .transform_origin
            .map(|[x, y]| [x * scale, y * scale])
            .unwrap_or([w * 0.5, h * 0.5]);

          rects.push(RectCmd {
            order,
            x,
            y,
            width: w,
            height: h,
            color: final_color,
            radii,
            stroke: [0.0; 4],
            stroke_color: TRANSPARENT_COLOR,
            transform: xf,
            transform_origin: xf_origin,
            clip: scaled_clip,
            gradient,
          });

          if let Some(borders) = quad.border {
            push_border_rects(
              &mut rects,
              order,
              x,
              y,
              w,
              h,
              scale,
              quad.border_radius,
              borders,
              quad.opacity,
              xf,
              xf_origin,
              scaled_clip,
            );
          }
        }
        QuadContent::Text {
          text,
          style,
          wrap,
          transform_mode,
        } => {
          let mut scaled_style = style.clone();
          scaled_style.font_size *= scale;
          let max_width =
            if (*wrap || style.text_align != crate::layout::text_style::TextAlign::Left) && quad.width > 0.0 {
              quad.width * scale
            } else {
              f32::MAX
            };
          let glyph_xf = quad.transform.matrix_2x2();
          let glyph_origin = quad
            .transform_origin
            .map(|[x, y]| [(quad.x + x) * scale, (quad.y + y) * scale])
            .unwrap_or([
              quad.x * scale + quad.width * scale * 0.5,
              quad.y * scale + quad.height * scale * 0.5,
            ]);
          let mut glyph_cmds = if quad.transform.is_identity() {
            app.glyph_engine.rasterize_text_with_wrap(
              text,
              &scaled_style,
              max_width,
              *wrap,
              quad.x * scale,
              quad.y * scale,
            )
          } else if *transform_mode == TextTransformMode::Rasterized {
            app.glyph_engine.rasterize_text_with_baked_transform(
              text,
              &scaled_style,
              max_width,
              *wrap,
              quad.x * scale,
              quad.y * scale,
              quad.transform,
              glyph_origin,
            )
          } else {
            let raster_scale = transformed_text_raster_scale(quad.transform);
            scaled_style.font_size *= raster_scale;
            let raster_max_width = if max_width.is_finite() {
              max_width * raster_scale
            } else {
              f32::MAX
            };
            let raster_x = quad.x * scale * raster_scale;
            let raster_y = quad.y * scale * raster_scale;
            let mut glyphs = app.glyph_engine.rasterize_text_unsnapped_with_wrap(
              text,
              &scaled_style,
              raster_max_width,
              *wrap,
              raster_x,
              raster_y,
            );
            for glyph in &mut glyphs {
              glyph.x /= raster_scale;
              glyph.y /= raster_scale;
              glyph.width /= raster_scale;
              glyph.height /= raster_scale;
            }
            glyphs
          };
          let glyph_clip = expand_text_clip_for_rasterization(scaled_clip);
          for g in &mut glyph_cmds {
            g.order = order;
            g.clip = glyph_clip;
            if !quad.transform.is_identity() && *transform_mode == TextTransformMode::Bitmap {
              g.transform = glyph_xf;
              g.transform_origin = [glyph_origin[0] - g.x, glyph_origin[1] - g.y];
            }
          }
          glyphs.extend(glyph_cmds);
        }
        #[cfg(feature = "image")]
        QuadContent::Image { data, uv_min, uv_max } => {
          let frame = data.frame_at(image_frame_time);
          if data.is_animated() {
            self.needs_redraw = true;
          }
          let image_transform = quad.transform.matrix_2x2();
          let image_transform_origin = quad
            .transform_origin
            .map(|[x, y]| [x * scale, y * scale])
            .unwrap_or([quad.width * scale * 0.5, quad.height * scale * 0.5]);
          let max_r = scaled_width.min(scaled_height) * 0.5;
          let radii = quad
            .border_radius
            .map(|r| {
              [
                (r.top_left * scale).min(max_r),
                (r.top_right * scale).min(max_r),
                (r.bottom_right * scale).min(max_r),
                (r.bottom_left * scale).min(max_r),
              ]
            })
            .unwrap_or([0.0; 4]);
          images.push(crate::images::ImageCmd {
            order,
            x: quad.x * scale,
            y: quad.y * scale,
            width: quad.width * scale,
            height: quad.height * scale,
            image_id: data.id(),
            frame_index: frame.frame_index,
            data: frame.data,
            image_width: frame.width,
            image_height: frame.height,
            uv_min: *uv_min,
            uv_max: *uv_max,
            radii,
            transform: image_transform,
            transform_origin: image_transform_origin,
            clip: scaled_clip,
          });
        }
        #[cfg(all(feature = "svg", feature = "image"))]
        QuadContent::Svg { data } => {
          let w = quad.width * scale;
          let h = quad.height * scale;
          let raster = crate::svg::rasterize::rasterize(&data, w, h);
          let image_transform = quad.transform.matrix_2x2();
          let image_transform_origin = quad
            .transform_origin
            .map(|[x, y]| [x * scale, y * scale])
            .unwrap_or([w * 0.5, h * 0.5]);
          images.push(crate::images::ImageCmd {
            order,
            x: quad.x * scale,
            y: quad.y * scale,
            width: w,
            height: h,
            image_id: raster.image_id,
            frame_index: 0,
            data: raster.data,
            image_width: raster.width,
            image_height: raster.height,
            uv_min: [0.0, 0.0],
            uv_max: [1.0, 1.0],
            radii: [0.0; 4],
            transform: image_transform,
            transform_origin: image_transform_origin,
            clip: scaled_clip,
          });
        }
        #[cfg(all(feature = "svg", not(feature = "image")))]
        QuadContent::Svg { data } => {
          let w = quad.width * scale;
          let h = quad.height * scale;
          let mesh = crate::svg::tessellate::tessellate(&data, w, h);
          svgs.push(crate::svg::SvgCmd {
            order,
            x: quad.x * scale,
            y: quad.y * scale,
            width: w,
            height: h,
            svg_id: data.id(),
            mesh: std::sync::Arc::new(mesh),
            clip: scaled_clip,
          });
        }
        QuadContent::None => {}
      }
    }

    #[cfg(feature = "devtools")]
    push_devtools_overlay(
      &mut rects,
      &mut glyphs,
      &mut app.glyph_engine,
      devtools_overlay,
      quad_count,
      scale,
      self.viewport_physical,
    );
    push_perf_meter(
      &mut rects,
      &mut glyphs,
      &mut app.glyph_engine,
      self.perf_overlay_enabled.then_some(self.perf_overlay_stats),
      quad_count + 20_000,
      scale,
      self.viewport_physical,
    );

    let glyph_dur = ProfileScope::elapsed_or_default(&glyph_start);
    let rect_count = rects.len();
    let glyph_count = glyphs.len();

    let list = RenderList {
      clear_color,
      rects,
      glyphs,
      #[cfg(feature = "image")]
      images,
      #[cfg(feature = "svg")]
      svgs,
      atlas: app.glyph_engine.atlas(),
    };

    let gpu_start = ProfileScope::maybe_start(profiling_enabled);
    let Some(render_engine) = &mut self.render_engine else {
      return;
    };
    render_engine.set_profiling_enabled(profiling_enabled);
    render_engine.render(&list, window, display);
    let gpu_dur = ProfileScope::elapsed_or_default(&gpu_start);

    if profiling_enabled {
      let render_profile = render_engine.last_profile().unwrap_or_default();
      self.last_profile = FrameProfile {
        layout: layout_dur,
        layout_recalculated,
        quad_resolve: quad_dur,
        glyph_rasterize: glyph_dur,
        gpu_submit: gpu_dur,
        render: render_profile,
        total: ProfileScope::elapsed_or_default(&frame_start),
        quad_count,
        rect_count,
        glyph_count,
        glyph_cache_hits: app.glyph_engine.glyph_hits,
        glyph_cache_misses: app.glyph_engine.glyph_misses,
        text_measure_cache_hits: app.glyph_engine.measure_hits,
        text_measure_cache_misses: app.glyph_engine.measure_misses,
        memory: self.memory_profile_with_glyph(app.glyph_engine.estimated_memory_bytes()),
      };
    }
    self.frame_count += 1;
    #[cfg(feature = "devtools")]
    self.sync_devtools();
  }

  pub fn mouse_move(&mut self, x: f32, y: f32) {
    self.dispatch_mouse(x, y, MouseButton::Left, MouseEventKind::Move);
    self.apply_reactive_updates_after_event();
  }

  pub fn mouse_leave_window(&mut self) {
    self.clear_hover_path();
    self.apply_reactive_updates_after_event();
  }

  pub fn mouse_down(&mut self, x: f32, y: f32, button: MouseButton) {
    self.click_press = Some(ClickPress {
      position: (x, y),
      button,
      target_ids: self.hit_target_ids_at(x, y),
    });
    self.dispatch_mouse(x, y, button, MouseEventKind::Down);
    self.apply_reactive_updates_after_event();
  }

  pub fn mouse_up(&mut self, x: f32, y: f32, button: MouseButton) {
    self.dispatch_mouse(x, y, button, MouseEventKind::Up);
    self.synthesize_click(x, y, button);
  }

  fn synthesize_click(&mut self, x: f32, y: f32, button: MouseButton) {
    let now = Instant::now();
    let position = (x, y);

    if self.should_suppress_click(now, position, button) {
      self.click_press = None;
      self.click_tracker.take_pending();
      self.apply_reactive_updates_after_event();
      return;
    }

    if !self.take_matching_click_press(position, button) {
      self.apply_reactive_updates_after_event();
      return;
    }

    if self.click_tracker.pending_matches(now, position, button) {
      self.click_tracker.take_pending();
      self.dispatch_mouse(x, y, button, MouseEventKind::DoubleClick);
      self.apply_reactive_updates_after_event();
      return;
    }

    self.flush_pending_click();

    if self.click_target_has_dblclick_handler(x, y) {
      self.click_tracker.set_pending(now, position, button);
      self.needs_redraw = true;
    } else {
      self.dispatch_mouse(x, y, button, MouseEventKind::Click);
      self.apply_reactive_updates_after_event();
    }
  }

  pub fn scroll(&mut self, x: f32, y: f32, delta_x: f32, delta_y: f32, phase: ScrollPhase) {
    self.dispatch_scroll(x, y, delta_x, delta_y, phase);
    self.apply_reactive_updates_after_event();
  }

  pub fn key_down(&mut self, key: String, code: String, shift: bool, ctrl: bool, alt: bool) {
    self.rebuild_if_dirty();
    let handled = if matches!((key.as_str(), code.as_str()), ("Tab", _) | (_, "Tab")) {
      #[cfg(feature = "form")]
      {
        self.focus_form_tab(shift)
      }
      #[cfg(not(feature = "form"))]
      {
        false
      }
    } else if matches!(
      (key.as_str(), code.as_str()),
      ("Enter" | " ", _) | (_, "Enter" | "Space")
    ) && self.activate_focused_button()
    {
      true
    } else if matches!((key.as_str(), code.as_str()), ("Enter", _) | (_, "Enter")) && {
      #[cfg(feature = "form")]
      {
        self.submit_focused_single_line_text_input()
      }
      #[cfg(not(feature = "form"))]
      {
        false
      }
    } {
      true
    } else {
      false
    };

    if handled {
      self.needs_redraw = true;
    } else if self.dispatch_select_key(&key, &code) {
      self.needs_redraw = true;
    } else if self.dispatch_text_input(&key, &code, shift, ctrl) {
      self.needs_redraw = true;
    } else {
      self.dispatch_selectable_text_clipboard(&key, &code, shift, ctrl);
    }

    let mut evt = KeyboardEvent {
      key,
      code,
      shift,
      ctrl,
      alt,
      target_id: NodeId::UNASSIGNED,
    };
    let root = match &self.root {
      Some(r) => r,
      None => return,
    };
    fire_keyboard_recursive(root, &mut evt);
    self.apply_reactive_updates_after_event();
  }

  #[cfg(feature = "form")]
  fn focus_form_tab(&mut self, reverse: bool) -> bool {
    let target = {
      let Some(root) = &self.root else {
        return false;
      };
      let form_path = match self.focused_path.as_deref() {
        Some(path) => nearest_form_path_for_path(root, path),
        None => first_form_path(root),
      };
      let Some(form_path) = form_path else {
        return false;
      };
      let Some(form) = find_node_by_path(root, &form_path) else {
        return false;
      };
      let mut candidates = Vec::new();
      collect_focus_candidates(form, None, &mut candidates);
      sort_focus_candidates(&mut candidates);
      if candidates.is_empty() {
        return false;
      }

      let current_index = self
        .focused_node
        .and_then(|id| candidates.iter().position(|candidate| candidate.input_id == id));
      let next_index = match (current_index, reverse) {
        (Some(0), true) => candidates.len() - 1,
        (Some(index), true) => index - 1,
        (Some(index), false) => (index + 1) % candidates.len(),
        (None, true) => candidates.len() - 1,
        (None, false) => 0,
      };
      candidates[next_index].target()
    };

    self.focus_node(target);
    true
  }

  #[cfg(feature = "form")]
  fn submit_focused_single_line_text_input(&mut self) -> bool {
    let Some(focused) = self.focused_node else {
      return false;
    };
    let is_single_line = {
      let Some(root) = &self.root else {
        return false;
      };
      let node = self
        .focused_path
        .as_deref()
        .and_then(|path| find_node_by_path(root, path))
        .or_else(|| find_node_by_id(root, focused));
      matches!(node.map(Node::node_kind), Some(NodeKind::TextInput { state, .. }) if state.overflow() != TextInputOverflow::Multiline)
    };
    is_single_line && self.submit_nearest_form_for_node_id(focused)
  }

  fn dispatch_select_key(&mut self, key: &str, code: &str) -> bool {
    let Some(focused) = self.focused_node else {
      return false;
    };
    let Some(root) = &self.root else {
      return false;
    };
    let Some(node) = find_node_by_id(root, focused) else {
      return false;
    };
    let NodeKind::Select { state } = node.node_kind() else {
      return false;
    };

    let down = matches!(key, "ArrowDown") || code == "ArrowDown";
    let up = matches!(key, "ArrowUp") || code == "ArrowUp";
    let activate = matches!(key, "Enter" | " ") || matches!(code, "Enter" | "Space");
    let escape = matches!(key, "Escape") || code == "Escape";

    if escape {
      if state.is_open() {
        state.set_open(false);
        return true;
      }
      return false;
    }
    if !state.is_open() {
      if down || up || activate {
        state.open_with_highlight();
        return true;
      }
      return false;
    }
    if down {
      state.move_highlight(1);
      return true;
    }
    if up {
      state.move_highlight(-1);
      return true;
    }
    if activate {
      state.activate();
      return true;
    }
    false
  }

  fn activate_focused_button(&mut self) -> bool {
    let Some(focused) = self.focused_node else {
      return false;
    };
    let (_kind, click) = {
      let Some(root) = &self.root else {
        return false;
      };
      let node = self
        .focused_path
        .as_deref()
        .and_then(|path| find_node_by_path(root, path))
        .or_else(|| find_node_by_id(root, focused));
      let Some(node) = node else {
        return false;
      };
      let Some(kind) = node.button_kind_value() else {
        return false;
      };
      (kind, node.events.on_click.clone())
    };

    if let Some(handler) = click {
      handler(&MouseEvent {
        x: 0.0,
        y: 0.0,
        button: MouseButton::Left,
        kind: MouseEventKind::Click,
        target_id: focused,
      });
    }

    #[cfg(feature = "form")]
    if _kind == ButtonKind::Submit {
      self.submit_nearest_form_for_node_id(focused);
    }
    true
  }

  #[cfg(feature = "form")]
  fn submit_nearest_form_for_node_id(&mut self, node_id: NodeId) -> bool {
    let submission = {
      let Some(root) = &self.root else {
        return false;
      };
      let Some(path) = find_path_by_id(root, node_id) else {
        return false;
      };
      nearest_form_submission(root, &path)
    };
    let Some((handler, data)) = submission else {
      return false;
    };
    handler(data);
    true
  }

  pub fn needs_redraw(&self) -> bool {
    self.needs_redraw
      || self.click_tracker.has_pending()
      || self.root_ctx.as_ref().is_some_and(Ctx::any_dirty)
      || self.root.as_ref().is_some_and(has_dirty_element_ref_recursive)
  }

  pub fn cursor(&self) -> CursorIcon {
    self.cursor
  }

  pub fn clear_needs_redraw(&mut self) {
    self.needs_redraw = false;
  }

  fn flush_due_pending_click(&mut self, now: Instant) {
    if self.click_tracker.pending_is_due(now) {
      self.flush_pending_click();
    } else if self.click_tracker.has_pending() {
      self.needs_redraw = true;
    }
  }

  fn flush_pending_click(&mut self) {
    let Some(click) = self.click_tracker.take_pending() else {
      return;
    };

    self.dispatch_mouse(click.position.0, click.position.1, click.button, MouseEventKind::Click);
    self.apply_reactive_updates_after_event();
  }

  fn click_target_has_dblclick_handler(&mut self, x: f32, y: f32) -> bool {
    let scale = self.scale_factor();
    let lx = x / scale;
    let ly = y / scale;

    self.rebuild_if_dirty();

    let root = match &self.root {
      Some(r) => r,
      None => return false,
    };
    let result = match &self.last_layout {
      Some(r) => r,
      None => return false,
    };

    let mut hits = Vec::new();
    hit_test_tree(root, result, 0.0, 0.0, lx, ly, &mut hits);
    hits.iter().any(|(node, _)| node.events.on_dblclick.is_some())
  }

  fn take_matching_click_press(&mut self, position: (f32, f32), button: MouseButton) -> bool {
    let Some(press) = self.click_press.take() else {
      return false;
    };

    if press.button != button {
      return false;
    }

    if distance_squared(press.position, position) > DOUBLE_CLICK_DISTANCE * DOUBLE_CLICK_DISTANCE {
      return false;
    }

    press.target_ids == self.hit_target_ids_at(position.0, position.1)
  }

  fn hit_target_ids_at(&mut self, x: f32, y: f32) -> Vec<NodeId> {
    let scale = self.scale_factor();
    let lx = x / scale;
    let ly = y / scale;

    self.rebuild_if_dirty();

    let root = match &self.root {
      Some(r) => r,
      None => return Vec::new(),
    };
    let result = match &self.last_layout {
      Some(r) => r,
      None => return Vec::new(),
    };

    let mut hits = Vec::new();
    hit_test_tree(root, result, 0.0, 0.0, lx, ly, &mut hits);
    hits.into_iter().map(|(node, _)| node.node_id()).collect()
  }

  fn suppress_click(&mut self, position: (f32, f32), button: MouseButton) {
    self.suppressed_click = Some(SuppressedClick {
      time: Instant::now(),
      position,
      button,
    });
  }

  fn should_suppress_click(&mut self, now: Instant, position: (f32, f32), button: MouseButton) -> bool {
    let Some(suppressed) = self.suppressed_click else {
      return false;
    };

    self.suppressed_click = None;

    now.duration_since(suppressed.time) <= SUPPRESSED_CLICK_INTERVAL
      && suppressed.button == button
      && distance_squared(suppressed.position, position) <= SUPPRESSED_CLICK_DISTANCE * SUPPRESSED_CLICK_DISTANCE
  }

  fn dispatch_mouse(&mut self, x: f32, y: f32, button: MouseButton, kind: MouseEventKind) {
    let mut evt = MouseEvent {
      x,
      y,
      button,
      kind,
      target_id: NodeId::UNASSIGNED,
    };
    let scale = self.scale_factor();
    let lx = evt.x / scale;
    let ly = evt.y / scale;

    self.rebuild_if_dirty();

    // Handle active scrollbar drag
    if let Some(ref drag) = self.dragging_scroll.clone() {
      match evt.kind {
        MouseEventKind::Move => {
          drag.state.drag_to_axis(drag.axis, lx, ly, &drag.state.style());
          self.needs_redraw = true;
          return;
        }
        MouseEventKind::Up => {
          drag.state.end_drag();
          self.dragging_scroll = None;
          self.clear_active_path();
          self.suppress_click((evt.x, evt.y), button);
          self.needs_redraw = true;
          return;
        }
        _ => {}
      }
    }

    if let Some(drag) = self.dragging_slider.clone() {
      match evt.kind {
        MouseEventKind::Move => {
          drag.update(lx);
          self.needs_redraw = true;
          return;
        }
        MouseEventKind::Up => {
          drag.update(lx);
          self.dragging_slider = None;
          self.clear_active_path();
          self.needs_redraw = true;
          return;
        }
        _ => {}
      }
    }

    if let Some(drag) = self.dragging_text_selection.clone() {
      match evt.kind {
        MouseEventKind::Move => {
          drag.update(lx, ly);
          self.needs_redraw = true;
          return;
        }
        MouseEventKind::Up => {
          drag.update(lx, ly);
          let has_selection = drag.has_selection();
          self.dragging_text_selection = None;
          self.clear_active_path();
          if has_selection {
            self.suppress_click((evt.x, evt.y), button);
          }
          self.needs_redraw = true;
          return;
        }
        _ => {}
      }
    }

    if self.active_drag.is_some() {
      match evt.kind {
        MouseEventKind::Move => {
          let (event, handler) = {
            let drag = self.active_drag.as_mut().unwrap();
            let event = drag.event(lx, ly, None);
            drag.last_x = lx;
            drag.last_y = ly;
            (event, drag.on_move.clone())
          };
          if let Some(handler) = handler {
            handler(&event);
          }
          self.needs_redraw = true;
          return;
        }
        MouseEventKind::Up => {
          let drag = self.active_drag.take().unwrap();
          if drag.button != button {
            self.active_drag = Some(drag);
            return;
          }
          let drop_target = self.drop_target_at(lx, ly);
          let drop_result = drop_target
            .as_ref()
            .map(|(target_id, _)| DropResult::Accepted { target_id: *target_id })
            .unwrap_or(DropResult::Missed);
          let drag_event = drag.event(lx, ly, Some(drop_result));
          if let Some((target_id, handler)) = drop_target {
            handler(&DropEvent {
              x: lx,
              y: ly,
              start_x: drag.start_x,
              start_y: drag.start_y,
              total_delta_x: lx - drag.start_x,
              total_delta_y: ly - drag.start_y,
              button,
              source_id: drag.target_id,
              target_id,
            });
          }
          if let Some(handler) = drag.on_end {
            handler(&drag_event);
          }
          self.clear_active_path();
          self.needs_redraw = true;
          return;
        }
        _ => {}
      }
    }

    let root = match &self.root {
      Some(r) => r,
      None => return,
    };
    let result = match &self.last_layout {
      Some(r) => r,
      None => {
        self.needs_redraw = true;
        return;
      }
    };

    let mut hits = Vec::new();
    hit_test_tree(root, result, 0.0, 0.0, lx, ly, &mut hits);
    let mut pending_focus = None;
    let mut builtin_needs_redraw = false;
    let mut pending_slider_drag = None;
    let mut pending_text_selection_drag = None;
    #[cfg(feature = "form")]
    let pending_submit = if matches!(evt.kind, MouseEventKind::Click) {
      hits
        .iter()
        .find(|(node, _)| node.button_kind_value() == Some(ButtonKind::Submit))
        .and_then(|(node, _)| find_path_by_id(root, node.node_id()))
        .and_then(|path| nearest_form_submission(root, &path))
    } else {
      None
    };

    if matches!(evt.kind, MouseEventKind::Down) {
      // Focus the select trigger on press so keyboard navigation works.
      if let Some((node, _)) = hits
        .iter()
        .find(|(node, _)| matches!(node.node_kind(), NodeKind::Select { .. }))
      {
        pending_focus = Some(FocusTarget {
          input_id: node.node_id(),
          event_id: node.node_id(),
        });
        builtin_needs_redraw = true;
      }

      // Dismiss open selects when the press lands outside their menu and
      // outside the open trigger (the trigger's own click toggles it).
      let on_menu = hits.iter().any(|(node, _)| node.tag_name() == SELECT_MENU_TAG);
      let on_open_trigger = hits
        .iter()
        .any(|(node, _)| matches!(node.node_kind(), NodeKind::Select { state } if state.is_open()));
      if !on_menu && !on_open_trigger && close_all_open_selects(root) {
        builtin_needs_redraw = true;
      }
    }

    if matches!(evt.kind, MouseEventKind::Click | MouseEventKind::Down) {
      if matches!(evt.kind, MouseEventKind::Down) {
        if let Some((node, rect)) = hits
          .iter()
          .find(|(node, _)| matches!(node.node_kind(), NodeKind::TextInput { .. }))
        {
          if let NodeKind::TextInput { state, .. } = node.node_kind() {
            let padding = self
              .layout_engine
              .resolved_padding_for_size(node, Size::new(rect.width, rect.height));
            let content_height = (rect.height - padding.top - padding.bottom).max(0.0);
            let vertical_offset = padding.top + text_input_vertical_offset(state, content_height);
            state.begin_selection_at_point(
              rect.local_x - rect.x - padding.left,
              rect.local_y - rect.y - vertical_offset,
            );
            pending_text_selection_drag = Some(TextSelectionDrag {
              kind: TextSelectionDragKind::Input(state.clone()),
              x: rect.x + padding.left,
              y: rect.y + vertical_offset,
              transform: rect.transform,
            });
            pending_focus = Some(FocusTarget {
              input_id: node.node_id(),
              event_id: node.node_id(),
            });
            builtin_needs_redraw = true;
          }
        }

        if pending_text_selection_drag.is_none()
          && let Some((node, rect)) = hits
            .iter()
            .find(|(node, _)| matches!(node.node_kind(), NodeKind::Text { state, .. } if state.selectable()))
        {
          if let NodeKind::Text { state, .. } = node.node_kind() {
            let value = node.text_content().unwrap_or_default().to_owned();
            state.begin_selection_at_point(&value, rect.local_x - rect.x, rect.local_y - rect.y);
            pending_text_selection_drag = Some(TextSelectionDrag {
              kind: TextSelectionDragKind::Text {
                state: state.clone(),
                value,
              },
              x: rect.x,
              y: rect.y,
              transform: rect.transform,
            });
            builtin_needs_redraw = true;
          }
        }

        if pending_text_selection_drag.is_none()
          && let Some((node, rect)) = hits
            .iter()
            .find(|(node, _)| matches!(node.node_kind(), NodeKind::Slider { .. }))
        {
          if let NodeKind::Slider { state } = node.node_kind() {
            let (track_rect, thumb_rect) = state.part_rects(
              rect.x,
              rect.y,
              rect.width,
              rect.height,
              node.is_style_hovered(),
              DEFAULT_SLIDER_THUMB_MIN_SIZE,
            );
            let travel_width = track_rect.width - thumb_rect.width;
            let (drag_x, drag_width) = if travel_width > 0.0 {
              (track_rect.x + thumb_rect.width * 0.5, travel_width)
            } else {
              (track_rect.x, track_rect.width)
            };
            let drag = SliderDrag {
              state: state.clone(),
              x: drag_x,
              width: drag_width,
            };
            drag.update(lx);
            pending_slider_drag = Some(drag);
            pending_focus = Some(FocusTarget {
              input_id: node.node_id(),
              event_id: node.node_id(),
            });
            builtin_needs_redraw = true;
          }
        }
      } else if let Some((node, rect)) = hits
        .iter()
        .find(|(node, _)| matches!(node.node_kind(), NodeKind::TextInput { .. }))
      {
        if let NodeKind::TextInput { state, .. } = node.node_kind() {
          let text_click_count = self
            .text_click_tracker
            .record(Instant::now(), (evt.x, evt.y), button, node.node_id());
          let padding = self
            .layout_engine
            .resolved_padding_for_size(node, Size::new(rect.width, rect.height));
          let content_height = (rect.height - padding.top - padding.bottom).max(0.0);
          let vertical_offset = padding.top + text_input_vertical_offset(state, content_height);
          let text_x = rect.local_x - rect.x - padding.left;
          let text_y = rect.local_y - rect.y - vertical_offset;
          match text_click_count {
            1 => state.set_caret_from_point(text_x, text_y),
            2 => state.select_word_at_point(text_x, text_y),
            _ => state.select_line_at_point(text_x, text_y),
          }
          pending_focus = Some(FocusTarget {
            input_id: node.node_id(),
            event_id: node.node_id(),
          });
          builtin_needs_redraw = true;
        }
      } else if let Some((node, rect)) = hits
        .iter()
        .find(|(node, _)| matches!(node.node_kind(), NodeKind::Text { state, .. } if state.selectable()))
      {
        if let NodeKind::Text { state, .. } = node.node_kind() {
          let value = node.text_content().unwrap_or_default();
          let text_click_count = self
            .text_click_tracker
            .record(Instant::now(), (evt.x, evt.y), button, node.node_id());
          match text_click_count {
            1 => state.clear_selection_at_point(value, rect.local_x - rect.x, rect.local_y - rect.y),
            2 => state.select_word_at_point(value, rect.local_x - rect.x, rect.local_y - rect.y),
            _ => state.select_line_at_point(value, rect.local_x - rect.x, rect.local_y - rect.y),
          }
          builtin_needs_redraw = true;
        }
      }
      if let Some(target) = dispatch_builtin_pointer(&hits, lx, matches!(evt.kind, MouseEventKind::Click)) {
        pending_focus = Some(target);
        builtin_needs_redraw = true;
      }
      if hits.is_empty() && matches!(evt.kind, MouseEventKind::Click) {
        if let Some((node, rect)) = find_slider_by_y_recursive(root, result, 0.0, 0.0, ly) {
          if let NodeKind::Slider { state } = node.node_kind() {
            let (track_rect, thumb_rect) = state.part_rects(
              rect.x,
              rect.y,
              rect.width,
              rect.height,
              node.is_style_hovered(),
              DEFAULT_SLIDER_THUMB_MIN_SIZE,
            );
            let ratio = state.pointer_ratio(lx, track_rect, thumb_rect);
            state.set_from_ratio(ratio);
            pending_focus = Some(FocusTarget {
              input_id: node.node_id(),
              event_id: node.node_id(),
            });
            builtin_needs_redraw = true;
          }
        }
      }
    }

    // Check scrollbar thumb hover/press
    for (node, _) in &hits {
      if let LayoutKind::ScrollModifier { state, direction } = node.layout_kind() {
        let sb_style = node.scrollbar_style();
        let mut on_thumb = false;
        let mut pressed_axis = None;

        for &axis in scroll_axes(*direction) {
          let Some((tx, ty, tw, th)) = state.thumb_rect_for_axis(axis, &sb_style) else {
            continue;
          };
          let on_axis_thumb = lx >= tx && lx <= tx + tw && ly >= ty && ly <= ty + th;
          on_thumb |= on_axis_thumb;
          if on_axis_thumb && matches!(evt.kind, MouseEventKind::Down) && pressed_axis.is_none() {
            pressed_axis = Some(axis);
          }
        }

        if on_thumb != state.is_thumb_hovered() {
          state.set_thumb_hovered(on_thumb);
          self.needs_redraw = true;
        }

        if let Some(axis) = pressed_axis {
          state.begin_drag_axis(axis, lx, ly);
          self.dragging_scroll = Some(ScrollDrag {
            state: state.clone(),
            axis,
          });
          self.needs_redraw = true;
          return;
        }
      }
    }

    let pending_drag = if matches!(evt.kind, MouseEventKind::Down)
      && pending_slider_drag.is_none()
      && pending_text_selection_drag.is_none()
    {
      hits
        .iter()
        .find(|(node, _)| {
          node.events.on_drag_start.is_some() || node.events.on_drag_move.is_some() || node.events.on_drag_end.is_some()
        })
        .map(|(node, _)| {
          let event = DragEvent {
            x: lx,
            y: ly,
            start_x: lx,
            start_y: ly,
            delta_x: 0.0,
            delta_y: 0.0,
            total_delta_x: 0.0,
            total_delta_y: 0.0,
            button,
            target_id: node.node_id(),
            drop_result: None,
          };
          (
            event,
            node.events.on_drag_start.clone(),
            ActiveDrag {
              target_id: node.node_id(),
              start_x: lx,
              start_y: ly,
              last_x: lx,
              last_y: ly,
              button,
              on_move: node.events.on_drag_move.clone(),
              on_end: node.events.on_drag_end.clone(),
            },
          )
        })
    } else {
      None
    };

    // Normal event dispatch
    for (node, _rect) in &hits {
      evt.target_id = node.node_id();
      match evt.kind {
        MouseEventKind::Click => {
          if let Some(ref handler) = node.events.on_click {
            handler(&evt);
          }
        }
        MouseEventKind::DoubleClick => {
          if let Some(ref handler) = node.events.on_dblclick {
            handler(&evt);
          }
        }
        MouseEventKind::Down => {
          if let Some(ref handler) = node.events.on_mouse_down {
            handler(&evt);
          }
        }
        MouseEventKind::Up => {
          if let Some(ref handler) = node.events.on_mouse_up {
            handler(&evt);
          }
        }
        MouseEventKind::Move => {
          if let Some(ref handler) = node.events.on_mouse_move {
            handler(&evt);
          }
        }
      }
    }
    #[cfg(feature = "form")]
    if let Some((handler, data)) = pending_submit {
      handler(data);
      self.needs_redraw = true;
    }

    let current_ids: Vec<NodeId> = hits.iter().map(|(n, _)| n.node_id()).collect();

    for old_id in &self.hover_path {
      if !current_ids.contains(old_id) {
        let Some(node) = find_node_by_id(root, *old_id) else {
          continue;
        };
        set_node_hovered(node, false);
        self.needs_redraw = true;
        if let Some(ref handler) = node.events.on_mouse_leave {
          handler();
        }
      }
    }

    for (node, _) in &hits {
      let id = node.node_id();
      if !self.hover_path.contains(&id) {
        set_node_hovered(node, true);
        self.needs_redraw = true;
        if let Some(ref handler) = node.events.on_mouse_enter {
          handler();
        }
      }
    }

    let clear_active_after_dispatch = matches!(evt.kind, MouseEventKind::Up | MouseEventKind::Click);

    for (node, _) in &hits {
      match evt.kind {
        MouseEventKind::Down => {
          set_node_active(node, true);
          self.needs_redraw = true;
        }
        MouseEventKind::Up | MouseEventKind::Click => {
          set_node_active(node, false);
          self.needs_redraw = true;
        }
        _ => {}
      }
    }

    self.hover_path = current_ids;
    self.cursor = hits
      .iter()
      .find_map(|(node, _)| node.cursor_icon())
      .unwrap_or(CursorIcon::Default);
    if matches!(evt.kind, MouseEventKind::Down) {
      self.active_path = self.hover_path.clone();
    }
    if builtin_needs_redraw {
      self.needs_redraw = true;
    }
    drop(hits);
    if clear_active_after_dispatch {
      self.clear_active_path();
    }
    if let Some(drag) = pending_slider_drag {
      self.dragging_slider = Some(drag);
    }
    if let Some(drag) = pending_text_selection_drag {
      self.dragging_text_selection = Some(drag);
    }
    if let Some((event, handler, drag)) = pending_drag {
      if let Some(handler) = handler {
        handler(&event);
      }
      self.active_drag = Some(drag);
      self.needs_redraw = true;
    }
    if let Some(target) = pending_focus {
      self.focus_node(target);
    }
  }

  fn dispatch_text_input(&mut self, key: &str, code: &str, shift: bool, ctrl: bool) -> bool {
    let focused = match self.focused_node {
      Some(id) => id,
      None => return false,
    };
    let root = match &self.root {
      Some(root) => root,
      None => return false,
    };
    let node = self
      .focused_path
      .as_deref()
      .and_then(|path| find_node_by_path(root, path))
      .or_else(|| find_node_by_id(root, focused));
    let node = match node {
      Some(node) => node,
      None => return false,
    };

    let command = code;
    let logical = key;

    match node.node_kind() {
      NodeKind::TextInput { state, .. } => match (logical, command) {
        ("a" | "A", _) | (_, "KeyA") if ctrl => state.select_all(),
        ("c" | "C", _) | (_, "KeyC") if ctrl => {
          let Some(selected) = state.selected_text() else {
            return false;
          };
          return write_clipboard_text(&selected);
        }
        ("x" | "X", _) | (_, "KeyX") if ctrl => {
          let Some(selected) = state.selected_text() else {
            return false;
          };
          if !write_clipboard_text(&selected) {
            return false;
          }
          let _ = state.cut_selection();
        }
        ("v" | "V", _) | (_, "KeyV") if ctrl => {
          let Some(text) = read_clipboard_text().filter(|text| !text.is_empty()) else {
            return false;
          };
          state.insert(&text);
        }
        ("Insert", _) | (_, "Insert") if ctrl => {
          let Some(selected) = state.selected_text() else {
            return false;
          };
          return write_clipboard_text(&selected);
        }
        ("Insert", _) | (_, "Insert") if shift => {
          let Some(text) = read_clipboard_text().filter(|text| !text.is_empty()) else {
            return false;
          };
          state.insert(&text);
        }
        ("Delete", _) | (_, "Delete") if shift => {
          let Some(selected) = state.selected_text() else {
            return false;
          };
          if !write_clipboard_text(&selected) {
            return false;
          }
          let _ = state.cut_selection();
        }
        ("z" | "Z", _) | (_, "KeyZ") if ctrl && shift => {
          if !state.redo() {
            return false;
          }
        }
        ("z" | "Z", _) | (_, "KeyZ") if ctrl => {
          if !state.undo() {
            return false;
          }
        }
        ("y" | "Y", _) | (_, "KeyY") if ctrl => {
          if !state.redo() {
            return false;
          }
        }
        ("Enter", _) | (_, "Enter") => {
          if !state.insert_newline() {
            return false;
          }
        }
        ("Backspace", _) | (_, "Backspace") => state.backspace(),
        ("Delete", _) | (_, "Delete") => state.delete(),
        ("ArrowLeft", _) | (_, "ArrowLeft") if ctrl => state.move_word_left(shift),
        ("ArrowRight", _) | (_, "ArrowRight") if ctrl => state.move_word_right(shift),
        ("ArrowLeft", _) | (_, "ArrowLeft") => state.move_left(shift),
        ("ArrowRight", _) | (_, "ArrowRight") => state.move_right(shift),
        ("ArrowUp", _) | (_, "ArrowUp") => state.move_up(shift),
        ("ArrowDown", _) | (_, "ArrowDown") => state.move_down(shift),
        ("Home", _) | (_, "Home") => state.move_home(shift),
        ("End", _) | (_, "End") => state.move_end(shift),
        _ if !ctrl && key.chars().count() == 1 => state.insert(key),
        _ => return false,
      },
      NodeKind::Checkbox { state } => match (logical, command) {
        (" " | "Space", _) | (_, "Space") => state.toggle(),
        _ => return false,
      },
      NodeKind::Slider { state } => match (logical, command) {
        ("ArrowRight" | "ArrowUp", _) | (_, "ArrowRight" | "ArrowUp") => state.nudge(1),
        ("ArrowLeft" | "ArrowDown", _) | (_, "ArrowLeft" | "ArrowDown") => state.nudge(-1),
        _ => return false,
      },
      _ => return false,
    }

    true
  }

  fn dispatch_selectable_text_clipboard(&mut self, key: &str, code: &str, shift: bool, ctrl: bool) -> bool {
    let command = code;
    let logical = key;
    if !matches!(
      (logical, command),
      ("c" | "C", _) | (_, "KeyC") if ctrl
    ) && !matches!(
      (logical, command),
      ("Insert", _) | (_, "Insert") if ctrl && !shift
    ) {
      return false;
    }

    let Some(root) = &self.root else {
      return false;
    };
    let Some(selected) = selected_selectable_text(root) else {
      return false;
    };
    write_clipboard_text(&selected)
  }

  fn drop_target_at(&self, x: f32, y: f32) -> Option<(NodeId, DropCallback)> {
    let root = self.root.as_ref()?;
    let result = self.last_layout.as_ref()?;
    let mut hits = Vec::new();
    hit_test_tree_all(root, result, 0.0, 0.0, x, y, &mut hits);
    hits
      .into_iter()
      .find_map(|(node, _)| node.events.on_drop.clone().map(|handler| (node.node_id(), handler)))
  }

  fn focus_node(&mut self, target: FocusTarget) {
    let Some(root) = self.root.as_ref() else {
      return;
    };
    let Some(input_path) = find_path_by_id(root, target.input_id) else {
      return;
    };
    let event_path = find_path_by_id(root, target.event_id).unwrap_or_else(|| input_path.clone());

    if self.focused_path.as_ref() == Some(&input_path) && self.focused_event_path.as_ref() == Some(&event_path) {
      return;
    }

    let blur = self
      .focused_event_path
      .as_deref()
      .and_then(|path| self.root.as_ref().and_then(|root| find_node_by_path(root, path)))
      .and_then(|node| node.events.on_blur.clone());
    let focus = self
      .root
      .as_ref()
      .and_then(|root| find_node_by_path(root, &event_path))
      .and_then(|node| node.events.on_focus.clone());

    if let Some(node) = self
      .focused_path
      .as_deref()
      .and_then(|path| self.root.as_ref().and_then(|root| find_node_by_path(root, path)))
    {
      set_node_focused(node, false);
      if let NodeKind::TextInput { state, .. } = node.node_kind() {
        state.set_focused(false);
      }
    }
    if let Some(node) = self
      .focused_event_path
      .as_deref()
      .and_then(|path| self.root.as_ref().and_then(|root| find_node_by_path(root, path)))
    {
      set_node_focused(node, false);
    }
    if let Some(node) = self.root.as_ref().and_then(|root| find_node_by_path(root, &input_path)) {
      set_node_focused(node, true);
      if let NodeKind::TextInput { state, .. } = node.node_kind() {
        state.set_focused(true);
      }
    }
    if let Some(node) = self.root.as_ref().and_then(|root| find_node_by_path(root, &event_path)) {
      set_node_focused(node, true);
    }

    if let Some(handler) = blur {
      handler();
    }
    self.focused_node = Some(target.input_id);
    self.focused_event_node = Some(target.event_id);
    self.focused_path = Some(input_path);
    self.focused_event_path = Some(event_path);
    if let Some(handler) = focus {
      handler();
    }
  }

  fn dispatch_scroll(&mut self, x: f32, y: f32, delta_x: f32, delta_y: f32, phase: ScrollPhase) {
    let mut evt = ScrollEvent {
      x,
      y,
      delta_x,
      delta_y,
      phase,
      target_id: NodeId::UNASSIGNED,
    };
    let root = match &self.root {
      Some(r) => r,
      None => return,
    };
    let result = match &self.last_layout {
      Some(r) => r,
      None => return,
    };

    let scale = self.scale_factor();
    let lx = evt.x / scale;
    let ly = evt.y / scale;

    let mut hits = Vec::new();
    hit_test_tree(root, result, 0.0, 0.0, lx, ly, &mut hits);

    // Auto-scroll from the innermost scroll container outward, preserving
    // any delta an edge-clamped child could not consume.
    let mut handled = false;
    let mut remaining_dx = -evt.delta_x;
    let mut remaining_dy = -evt.delta_y;
    for (node, _) in &hits {
      if let LayoutKind::ScrollModifier { state, direction } = node.layout_kind() {
        let dx = if scroll_direction_has_axis(*direction, ScrollAxis::Horizontal) {
          remaining_dx
        } else {
          0.0
        };
        let dy = if scroll_direction_has_axis(*direction, ScrollAxis::Vertical) {
          remaining_dy
        } else {
          0.0
        };

        if dx == 0.0 && dy == 0.0 {
          continue;
        }

        let (overflow_dx, overflow_dy) = state.scroll_by_with_overflow(dx, dy);
        if overflow_dx != dx || overflow_dy != dy {
          self.needs_redraw = true;
          handled = true;
        }
        if scroll_direction_has_axis(*direction, ScrollAxis::Horizontal) {
          remaining_dx = overflow_dx;
        }
        if scroll_direction_has_axis(*direction, ScrollAxis::Vertical) {
          remaining_dy = overflow_dy;
        }
        if remaining_dx == 0.0 && remaining_dy == 0.0 {
          break;
        }
      }
    }

    // Fire user handlers
    for (node, _) in &hits {
      evt.target_id = node.node_id();
      match evt.phase {
        ScrollPhase::Start => {
          if let Some(ref handler) = node.events.on_scroll_start {
            handler(&evt);
          }
        }
        ScrollPhase::Scroll => {
          if let Some(ref handler) = node.events.on_scroll {
            handler(&evt);
          }
        }
        ScrollPhase::End => {
          if let Some(ref handler) = node.events.on_scroll_end {
            handler(&evt);
          }
        }
      }
    }

    let _ = handled;
  }

  pub fn resolve_quads(&self, result: &LayoutResult) -> Vec<Quad> {
    match &self.root {
      Some(root) => self.layout_engine.resolve_quads(root, result),
      None => vec![],
    }
  }

  pub fn resolve_quads_with_viewport(&self, result: &LayoutResult, viewport: ClipRect) -> Vec<Quad> {
    match &self.root {
      Some(root) => self.layout_engine.resolve_quads_with_viewport(root, result, viewport),
      None => vec![],
    }
  }

  #[doc(hidden)]
  pub fn set_layout_constraints_override(&mut self, constraints: Option<Constraints>) {
    self.layout_constraints_override = constraints;
    self.last_layout = None;
  }

  #[doc(hidden)]
  pub fn last_layout(&self) -> Option<&LayoutResult> {
    self.last_layout.as_ref()
  }

  #[cfg(feature = "devtools")]
  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn draw_debug_overlay_over_node(&mut self, path: Vec<usize>) -> bool {
    self.set_debug_overlay_node_path(Some(path))
  }

  #[cfg(feature = "devtools")]
  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn clear_debug_overlay(&mut self) -> bool {
    self.set_debug_overlay_node_path(None)
  }

  #[cfg(feature = "devtools")]
  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  pub(crate) fn debug_overlay_node_at(&mut self, x: f32, y: f32) -> Option<Vec<usize>> {
    self.rebuild_if_dirty();
    let root = self.root.as_ref()?;
    let result = self.last_layout.as_ref()?;
    let scale = self.scale_factor();
    let mut hits = Vec::new();
    hit_test_tree(root, result, 0.0, 0.0, x / scale, y / scale, &mut hits);
    let node = hits.first()?.0;
    find_path_by_id(root, node.node_id())
  }

  #[cfg(feature = "devtools")]
  #[cfg_attr(not(feature = "winit"), allow(dead_code))]
  fn set_debug_overlay_node_path(&mut self, path: Option<Vec<usize>>) -> bool {
    if self.debug_overlay_node_path == path {
      return false;
    }
    self.debug_overlay_node_path = path;
    self.needs_redraw = true;
    true
  }

  fn rebuild_if_dirty(&mut self) {
    if self.root_ctx.as_ref().is_some_and(Ctx::is_dirty) {
      self.rebuild();
    } else if self.root_ctx.as_ref().is_some_and(Ctx::any_dirty) {
      self.refresh_dirty_subtrees();
    }
  }

  fn apply_reactive_updates_after_event(&mut self) {
    if self.root_ctx.as_ref().is_some_and(Ctx::any_dirty) {
      self.needs_redraw = true;
      self.rebuild_if_dirty();
    }
  }

  fn update_perf_overlay_stats(&mut self) {
    if !self.perf_overlay_enabled {
      return;
    }

    let frame_count = self.frame_count;
    if frame_count > self.perf_overlay_last_seen_frame {
      self.perf_overlay_frames_since_sample += frame_count - self.perf_overlay_last_seen_frame;
      self.perf_overlay_last_seen_frame = frame_count;
    }

    let now = Instant::now();
    let elapsed = now.duration_since(self.perf_overlay_last_sample);
    if elapsed >= PERF_SAMPLE_INTERVAL {
      let profile = &self.last_profile;
      self.perf_overlay_stats = PerfMeterStats {
        fps: (self.perf_overlay_frames_since_sample as f32 / elapsed.as_secs_f32()).round() as u32,
        total_ms: ms(profile.total),
        layout_ms: ms(profile.layout),
        quad_resolve_ms: ms(profile.quad_resolve),
        glyph_ms: ms(profile.glyph_rasterize),
        render_acquire_ms: ms(profile.render.acquire),
        render_upload_ms: ms(profile.render.globals_upload + profile.render.atlas_upload),
        render_encode_ms: ms(profile.render.encode),
        render_submit_ms: ms(profile.render.submit),
        render_present_ms: ms(profile.render.present),
        quad_count: profile.quad_count,
        glyph_count: profile.glyph_count,
      };
      self.perf_overlay_frames_since_sample = 0;
      self.perf_overlay_last_sample = now;
    }
  }

  fn refresh_dirty_subtrees(&mut self) {
    let replacements = match &mut self.root_ctx {
      Some(ctx) => ctx.refresh_dirty_subtrees(),
      None => return,
    };

    if replacements.is_empty() {
      return;
    }

    if let Some(root) = &mut self.root {
      for (slot_id, replacement) in replacements {
        replace_live_component_slot(root, slot_id, replacement, &self.id_gen);
      }
    }

    self.sync_modal_layer();
    self.needs_redraw = true;
    self.refresh_interaction_state();
  }

  fn sync_modal_layer(&mut self) {
    let modals = self.root_ctx.as_ref().map(Ctx::modal_nodes).unwrap_or_default();
    let Some(root) = self.root.take() else {
      return;
    };

    let base = modal_host_base(root);

    if modals.is_empty() {
      self.root = Some(base);
      return;
    }

    let mut children = Vec::with_capacity(1 + modals.len());
    children.push(base);
    children.extend(modals);
    let mut host = Node::stack(crate::layout::StackAlignment::TopStart, children);
    host.set_tag_name(MODAL_HOST_TAG);
    self.root = Some(host);
  }

  pub fn register_keyframes(&mut self, keyframes: Keyframes) {
    self.animation_engine.register_keyframes(keyframes);
  }

  fn update_layout(&mut self, app: &mut App) {
    self.rebuild_if_dirty();
    self.sync_dynamic_content();
    let select_overlay_parts = self.detach_select_overlay();
    #[cfg(all(feature = "image", feature = "resources"))]
    self.resolve_resource_images(app);
    #[cfg(all(feature = "svg", feature = "resources"))]
    self.resolve_resource_svgs(app);

    if let Some(root) = self.root.as_mut() {
      let now = Instant::now();
      self.transition_engine.tick(root, now);
      self.animation_engine.tick(root, now);
      if self.transition_engine.has_active || self.animation_engine.has_active {
        self.needs_redraw = true;
      }
    }

    if let Some(root) = self.root.as_ref() {
      let constraints = self
        .layout_constraints_override
        .unwrap_or_else(|| Constraints::tight(self.viewport_logical()));
      let theme_version = self
        .root_ctx
        .as_ref()
        .map(|ctx| ctx.theme().version())
        .unwrap_or_else(|| app.theme().version());
      let typography = self
        .root_ctx
        .as_ref()
        .map(|ctx| ctx.theme().typography().clone())
        .unwrap_or_else(|| app.theme().typography().clone());
      let palette = self
        .root_ctx
        .as_ref()
        .map(|ctx| ctx.theme().palette().clone())
        .unwrap_or_else(|| app.theme().palette().clone());
      let spacing = self
        .root_ctx
        .as_ref()
        .map(|ctx| ctx.theme().spacing().clone())
        .unwrap_or_else(|| app.theme().spacing().clone());
      let radii = self
        .root_ctx
        .as_ref()
        .map(|ctx| ctx.theme().radii().clone())
        .unwrap_or_else(|| app.theme().radii().clone());
      let border_sizes = self
        .root_ctx
        .as_ref()
        .map(|ctx| ctx.theme().border_sizes().clone())
        .unwrap_or_else(|| app.theme().border_sizes().clone());
      let theme_changed = self.last_theme_version != theme_version;
      let mut layout = self.layout_engine.compute(
        &mut app.glyph_engine,
        root,
        constraints,
        palette.clone(),
        border_sizes,
        spacing,
        radii,
        typography.clone(),
        theme_changed,
      );
      self.sync_select_overlay_from_layout(select_overlay_parts, &layout);
      if let Some(root) = self.root.as_ref()
        && root.tag_name() == SELECT_OVERLAY_TAG
      {
        layout = self.layout_engine.compute(
          &mut app.glyph_engine,
          root,
          constraints,
          palette,
          border_sizes,
          spacing,
          radii,
          typography,
          theme_changed,
        );
      }
      self.last_theme_version = theme_version;
      if let Some(root) = self.root.as_mut() {
        update_element_refs_recursive(root, &layout, 0.0, 0.0, 0.0, 0.0);
      }
      self.last_layout = Some(layout);
    }
  }

  fn detach_select_overlay(&mut self) -> SelectOverlayReuse {
    let Some(root) = self.root.take() else {
      return SelectOverlayReuse {
        old_host: None,
        old_overlays: Vec::new(),
      };
    };
    let parts = select_overlay_parts(root);
    self.root = Some(parts.base);
    SelectOverlayReuse {
      old_host: parts.old_host,
      old_overlays: parts.old_overlays,
    }
  }

  fn sync_select_overlay_from_layout(&mut self, mut old_parts: SelectOverlayReuse, base_layout: &LayoutResult) {
    let Some(base) = self.root.take() else {
      return;
    };
    let viewport = self.viewport_logical();
    let mut overlays = Vec::new();
    collect_select_menus(&base, base_layout, 0.0, 0.0, viewport, &mut overlays);
    if overlays.is_empty() {
      self.root = Some(base);
      return;
    }
    for (overlay, old_overlay) in overlays.iter_mut().zip(old_parts.old_overlays.iter_mut()) {
      overlay.preserve_runtime_state_from(old_overlay);
      overlay.preserve_ids_from(old_overlay);
    }
    let mut children = Vec::with_capacity(1 + overlays.len());
    children.push(base);
    children.extend(overlays);
    let mut host = Node::stack(crate::layout::StackAlignment::TopStart, children);
    host.set_tag_name(SELECT_OVERLAY_TAG);
    if let Some(old_host) = old_parts.old_host.as_mut() {
      host.preserve_ids_from(old_host);
    }
    host.assign_ids(&self.id_gen);
    self.root = Some(host);
  }

  fn sync_dynamic_content(&mut self) {
    if let Some(root) = &mut self.root {
      root.sync_dynamic_content_recursive();
    }
  }

  #[cfg(all(feature = "image", feature = "resources"))]
  fn resolve_resource_images(&mut self, app: &mut App) {
    if let Some(root) = &mut self.root {
      Self::resolve_resource_images_recursive(root, &app.resource_loader, &mut app.image_resource_cache);
    }
  }

  #[cfg(all(feature = "image", feature = "resources"))]
  fn resolve_resource_images_recursive(
    node: &mut Node,
    loader: &crate::resources::ResourceLoader,
    image_cache: &mut std::collections::HashMap<Arc<str>, crate::images::ImageData>,
  ) -> bool {
    let mut layout_dirty = false;

    if let NodeKind::ResourceImage { path } = node.node_kind() {
      let key: std::sync::Arc<str> = path.clone();
      if let Some(img) = Self::resolve_image_resource(&key, loader, image_cache) {
        node.intrinsic_size = Some(Size::new(img.width() as f32, img.height() as f32));
        node.node_kind = NodeKind::Image { data: img };
        layout_dirty = true;
      }
    }

    if let Some(key) = node.background_resource_image.clone() {
      if let Some(img) = Self::resolve_image_resource(&key, loader, image_cache) {
        let current_id = node.background_image.as_ref().map(crate::images::ImageData::id);
        if current_id != Some(img.id()) {
          node.background_image.set(Some(img));
        }
      }
    }

    match node.node_kind() {
      NodeKind::Checkbox { state } => {
        state.resolve_resource_images(|key| Self::resolve_image_resource(key, loader, image_cache));
      }
      NodeKind::Slider { state } => {
        state.resolve_resource_images(|key| Self::resolve_image_resource(key, loader, image_cache));
      }
      _ => {}
    }

    for child in &mut node.children {
      if Self::resolve_resource_images_recursive(child, loader, image_cache) {
        layout_dirty = true;
      }
    }

    if layout_dirty {
      node.layout_cache.invalidate();
    }

    layout_dirty
  }

  #[cfg(all(feature = "image", feature = "resources"))]
  fn resolve_image_resource(
    key: &Arc<str>,
    loader: &crate::resources::ResourceLoader,
    image_cache: &mut std::collections::HashMap<Arc<str>, crate::images::ImageData>,
  ) -> Option<crate::images::ImageData> {
    if let Some(img) = image_cache.get(key) {
      return Some(img.clone());
    }

    let crate::resources::LoadResourceResult::Loaded(bytes) = loader.load_resource(key, None) else {
      return None;
    };

    let img = crate::images::ImageData::from_bytes(&bytes).ok()?;
    image_cache.insert(key.clone(), img.clone());
    Some(img)
  }

  #[cfg(all(feature = "svg", feature = "resources"))]
  fn resolve_resource_svgs(&mut self, app: &mut App) {
    if let Some(root) = &mut self.root {
      Self::resolve_resource_svgs_recursive(root, &app.resource_loader, &mut app.svg_resource_cache);
    }
  }

  #[cfg(all(feature = "svg", feature = "resources"))]
  fn resolve_resource_svgs_recursive(
    node: &mut Node,
    loader: &crate::resources::ResourceLoader,
    svg_cache: &mut std::collections::HashMap<Arc<str>, crate::svg::SvgData>,
  ) -> bool {
    let mut layout_dirty = false;

    if let NodeKind::ResourceSvg { path } = node.node_kind() {
      let key: Arc<str> = path.clone();
      if let Some(svg) = Self::resolve_svg_resource(&key, loader, svg_cache) {
        node.intrinsic_size = Some(Size::new(svg.viewbox_width(), svg.viewbox_height()));
        node.node_kind = NodeKind::Svg { data: svg };
        layout_dirty = true;
      }
    }

    for child in &mut node.children {
      if Self::resolve_resource_svgs_recursive(child, loader, svg_cache) {
        layout_dirty = true;
      }
    }

    if layout_dirty {
      node.layout_cache.invalidate();
    }

    layout_dirty
  }

  #[cfg(all(feature = "svg", feature = "resources"))]
  fn resolve_svg_resource(
    key: &Arc<str>,
    loader: &crate::resources::ResourceLoader,
    svg_cache: &mut std::collections::HashMap<Arc<str>, crate::svg::SvgData>,
  ) -> Option<crate::svg::SvgData> {
    if let Some(svg) = svg_cache.get(key) {
      return Some(svg.clone());
    }

    let crate::resources::LoadResourceResult::Loaded(bytes) = loader.load_resource(key, None) else {
      return None;
    };

    let svg = crate::svg::SvgData::from_bytes(&bytes);
    svg_cache.insert(key.clone(), svg.clone());
    Some(svg)
  }

  fn clear_active_path(&mut self) {
    let active_path = std::mem::take(&mut self.active_path);
    if let Some(root) = self.root.as_ref() {
      for node_id in active_path {
        if let Some(node) = find_node_by_id(root, node_id) {
          set_node_active(node, false);
          self.needs_redraw = true;
        }
      }
    }
  }

  fn clear_hover_path(&mut self) {
    let hover_path = std::mem::take(&mut self.hover_path);
    if hover_path.is_empty() {
      self.cursor = CursorIcon::Default;
      return;
    }

    if let Some(root) = self.root.as_ref() {
      for node_id in hover_path {
        if let Some(node) = find_node_by_id(root, node_id) {
          set_node_hovered(node, false);
          if let Some(ref handler) = node.events.on_mouse_leave {
            handler();
          }
        }
      }
    }

    self.cursor = CursorIcon::Default;
    self.needs_redraw = true;
  }

  fn refresh_interaction_state(&mut self) {
    let Some(root) = self.root.as_ref() else {
      self.hover_path.clear();
      self.active_path.clear();
      self.cursor = CursorIcon::Default;
      self.clear_focus();
      return;
    };

    reset_element_ref_flags_recursive(root);

    let mut hover_path = Vec::new();
    for node_id in &self.hover_path {
      if let Some(node) = find_node_by_id(root, *node_id) {
        set_node_hovered(node, true);
        hover_path.push(*node_id);
      }
    }
    self.hover_path = hover_path;

    let mut active_path = Vec::new();
    for node_id in &self.active_path {
      if let Some(node) = find_node_by_id(root, *node_id) {
        set_node_active(node, true);
        active_path.push(*node_id);
      }
    }
    self.active_path = active_path;

    self.cursor = self
      .hover_path
      .iter()
      .filter_map(|node_id| find_node_by_id(root, *node_id))
      .find_map(Node::cursor_icon)
      .unwrap_or(CursorIcon::Default);
    self.refresh_focus_ids();
  }

  fn clear_focus(&mut self) {
    if let Some(node) = self
      .focused_path
      .as_deref()
      .and_then(|path| self.root.as_ref().and_then(|root| find_node_by_path(root, path)))
    {
      set_node_focused(node, false);
      if let NodeKind::TextInput { state, .. } = node.node_kind() {
        state.set_focused(false);
      }
    }
    if let Some(node) = self
      .focused_event_path
      .as_deref()
      .and_then(|path| self.root.as_ref().and_then(|root| find_node_by_path(root, path)))
    {
      set_node_focused(node, false);
    }
    self.focused_node = None;
    self.focused_event_node = None;
    self.focused_path = None;
    self.focused_event_path = None;
  }

  fn refresh_focus_ids(&mut self) {
    let Some(root) = self.root.as_ref() else {
      self.clear_focus();
      return;
    };

    let Some(focused_node) = self.focused_node else {
      self.clear_focus();
      return;
    };
    let Some(input_path) = find_path_by_id(root, focused_node) else {
      self.clear_focus();
      return;
    };
    let event_id = self.focused_event_node.unwrap_or(focused_node);
    let event_path = find_path_by_id(root, event_id).unwrap_or_else(|| input_path.clone());

    self.focused_path = Some(input_path.clone());
    self.focused_event_path = Some(event_path.clone());

    if let Some(node) = find_node_by_path(root, &input_path) {
      set_node_focused(node, true);
      if let NodeKind::TextInput { state, .. } = node.node_kind() {
        state.set_focused(true);
      }
    }
    if let Some(node) = find_node_by_path(root, &event_path) {
      set_node_focused(node, true);
    }
  }
}

fn modal_host_base(mut root: Node) -> Node {
  if root.tag_name() == MODAL_HOST_TAG && !root.children.is_empty() {
    root.children.remove(0)
  } else {
    root
  }
}

const SELECT_OVERLAY_TAG: &str = "__lurq_select_overlay_host";
const SELECT_MENU_TAG: &str = "__lurq_select_menu";

struct SelectOverlayParts {
  base: Node,
  old_host: Option<Node>,
  old_overlays: Vec<Node>,
}

struct SelectOverlayReuse {
  old_host: Option<Node>,
  old_overlays: Vec<Node>,
}

/// Undo a previous select-overlay wrap so we can recompute it each pass while
/// preserving ids for the synthetic overlay nodes across redraws.
fn select_overlay_parts(mut root: Node) -> SelectOverlayParts {
  if root.tag_name() == SELECT_OVERLAY_TAG && !root.children.is_empty() {
    let mut children = std::mem::take(&mut root.children);
    let base = children.remove(0);
    let old_overlays = children;
    SelectOverlayParts {
      base,
      old_host: Some(root),
      old_overlays,
    }
  } else {
    SelectOverlayParts {
      base: root,
      old_host: None,
      old_overlays: Vec::new(),
    }
  }
}

/// Walk the tree and build a floating menu for every open select, anchored to
/// the trigger's resolved layout rect.
fn collect_select_menus(
  node: &Node,
  layout: &LayoutResult,
  abs_x: f32,
  abs_y: f32,
  viewport: Size,
  out: &mut Vec<Node>,
) {
  if let NodeKind::Select { state } = node.node_kind()
    && state.is_open()
    && state.option_count() > 0
  {
    let bounds = ElementRect {
      x: abs_x,
      y: abs_y,
      relative_x: abs_x,
      relative_y: abs_y,
      width: layout.size.width,
      height: layout.size.height,
    };
    if bounds.width > 0.0 {
      out.push(build_select_menu(state, bounds, viewport));
    }
  }
  for (child_layout, child) in layout.children.iter().zip(node.children()) {
    collect_select_menus(
      child,
      &child_layout.result,
      abs_x + child_layout.offset.x,
      abs_y + child_layout.offset.y,
      viewport,
      out,
    );
  }
}

const SELECT_OPTION_ROW_HEIGHT: f32 = 34.0;

fn build_select_menu(
  state: &crate::node::node_kind::SelectState,
  bounds: crate::core::ElementRect,
  viewport: Size,
) -> Node {
  use crate::node::dimension::Dimension;
  let style = state.style();
  let labels = state.labels();
  let multiple = state.multiple();
  let highlighted = state.highlighted();
  let checkmark_color = style.checkmark_color;

  let mut options = Vec::with_capacity(labels.len());
  for (index, label) in labels.iter().enumerate() {
    let selected = state.is_selected(index);
    let active = highlighted == Some(index);
    let mut part = style.resolved_option(active && !selected, selected);
    apply_select_menu_edge_radius(&mut part, &style.menu, index, labels.len());
    let text_style = part.text.clone();

    let label_node = text_style
      .as_ref()
      .map(|style| Node::text_styled(label, style.clone()))
      .unwrap_or_else(|| Node::text(label));
    let mut row = if multiple {
      let mut check_style = text_style.clone().unwrap_or_default();
      if let Some(color) = checkmark_color {
        check_style.color = color;
      }
      let mark = if selected { "\u{2713}" } else { " " };
      let check_node = Node::text_styled(mark, check_style).width(Dimension::Px(16.0));
      Node::row(6.0, crate::layout::Alignment::Center, vec![check_node, label_node])
    } else {
      Node::row(0.0, crate::layout::Alignment::Center, vec![label_node])
    };

    row = row.width(Dimension::Pct(100.0)).apply_select_part(&part);
    if part.padding.is_none() {
      row = row.padding_horizontal(10.0).padding_vertical(7.0);
    }
    let commit_state = state.clone();
    row.events.on_click = Some(Arc::new(move |_| commit_state.commit(index)));
    if let Some(hover) = style.resolved_option(true, selected).background {
      row = row.hovered(move |s| s.background(hover));
    }
    options.push(row);
  }

  let list = Node::column(0.0, crate::layout::Alignment::Start, options).width(Dimension::Pct(100.0));
  let mut menu = crate::node::dsl::scroll_vertical(list).apply_select_part(&style.menu);
  menu.set_tag_name(SELECT_MENU_TAG);

  // Estimate height to decide whether to drop down or flip up, then clamp the
  // final rect inside the viewport so the menu is always visible/clickable.
  let estimated = (labels.len() as f32 * SELECT_OPTION_ROW_HEIGHT).min(style.max_menu_height);
  let below_y = bounds.y + bounds.height + style.menu_gap;
  let space_below = (viewport.height - below_y).max(0.0);
  let flip = estimated > space_below && bounds.y - style.menu_gap > space_below;
  let raw_y = if flip {
    bounds.y - style.menu_gap - estimated
  } else {
    below_y
  };
  let max_y = (viewport.height - estimated).max(0.0);
  let y = raw_y.clamp(0.0, max_y);

  let width = bounds.width.min(viewport.width.max(0.0));
  let max_x = (viewport.width - width).max(0.0);
  let x = bounds.x.clamp(0.0, max_x);

  menu
    .max_height(Dimension::Px(style.max_menu_height))
    .absolute_positioned(x, y, Some(Dimension::Px(width)), None)
}

fn apply_select_menu_edge_radius(
  part: &mut crate::node::select_style::SelectPartStyle,
  menu: &crate::node::select_style::SelectPartStyle,
  index: usize,
  count: usize,
) {
  if count == 0 || part.border_radius.is_some() {
    return;
  }
  let Some(menu_radius) = menu.border_radius else {
    return;
  };
  let zero = RadiusValue::Px(0.0);
  let first = index == 0;
  let last = index + 1 == count;
  part.border_radius = Some(ThemedBorderRadius::new(
    if first { menu_radius.top_left } else { zero },
    if first { menu_radius.top_right } else { zero },
    if last { menu_radius.bottom_right } else { zero },
    if last { menu_radius.bottom_left } else { zero },
  ));
}

/// Close every open select; returns whether any were open.
fn close_all_open_selects(node: &Node) -> bool {
  let mut changed = false;
  if let NodeKind::Select { state } = node.node_kind()
    && state.is_open()
  {
    state.set_open(false);
    changed = true;
  }
  for child in node.children() {
    changed |= close_all_open_selects(child);
  }
  changed
}

fn scroll_axes(direction: ScrollDirection) -> &'static [ScrollAxis] {
  match direction {
    ScrollDirection::Horizontal => &[ScrollAxis::Horizontal],
    ScrollDirection::Vertical => &[ScrollAxis::Vertical],
    ScrollDirection::Both => &[ScrollAxis::Vertical, ScrollAxis::Horizontal],
  }
}

fn scroll_direction_has_axis(direction: ScrollDirection, axis: ScrollAxis) -> bool {
  matches!(
    (direction, axis),
    (ScrollDirection::Horizontal, ScrollAxis::Horizontal)
      | (ScrollDirection::Vertical, ScrollAxis::Vertical)
      | (ScrollDirection::Both, _)
  )
}

#[derive(Clone)]
struct ScrollDrag {
  state: ScrollState,
  axis: ScrollAxis,
}

#[derive(Clone)]
struct SliderDrag {
  state: SliderState,
  x: f32,
  width: f32,
}

impl SliderDrag {
  fn update(&self, x: f32) -> bool {
    let ratio = if self.width > 0.0 {
      (x - self.x) / self.width
    } else {
      0.0
    };
    self.state.set_from_ratio(ratio)
  }
}

#[derive(Clone)]
struct TextSelectionDrag {
  kind: TextSelectionDragKind,
  x: f32,
  y: f32,
  transform: Transform2D,
}

#[derive(Clone)]
enum TextSelectionDragKind {
  Input(TextInputState),
  Text { state: TextState, value: String },
}

impl TextSelectionDrag {
  fn update(&self, x: f32, y: f32) {
    let (local_x, local_y) = self.local_point(x, y);
    match &self.kind {
      TextSelectionDragKind::Input(state) => state.update_selection_to_point(local_x - self.x, local_y - self.y),
      TextSelectionDragKind::Text { state, value } => {
        state.update_selection_to_point(value, local_x - self.x, local_y - self.y)
      }
    }
  }

  fn has_selection(&self) -> bool {
    match &self.kind {
      TextSelectionDragKind::Input(state) => state.has_selection(),
      TextSelectionDragKind::Text { state, value } => state.has_selection(value),
    }
  }

  fn local_point(&self, x: f32, y: f32) -> (f32, f32) {
    if self.transform.is_identity() {
      return (x, y);
    }

    let Some(inverse) = self.transform.inverse_affine() else {
      return (x, y);
    };
    inverse.transform_point(x, y)
  }
}

fn text_input_vertical_offset(state: &TextInputState, height: f32) -> f32 {
  if state.overflow() == TextInputOverflow::Scroll {
    ((height - state.caret_height()).max(0.0)) * 0.5
  } else {
    0.0
  }
}

type DragCallback = Arc<dyn Fn(&DragEvent) + Send + Sync>;
type DropCallback = Arc<dyn Fn(&DropEvent) + Send + Sync>;

struct ActiveDrag {
  target_id: NodeId,
  start_x: f32,
  start_y: f32,
  last_x: f32,
  last_y: f32,
  button: MouseButton,
  on_move: Option<DragCallback>,
  on_end: Option<DragCallback>,
}

impl ActiveDrag {
  fn event(&self, x: f32, y: f32, drop_result: Option<DropResult>) -> DragEvent {
    DragEvent {
      x,
      y,
      start_x: self.start_x,
      start_y: self.start_y,
      delta_x: x - self.last_x,
      delta_y: y - self.last_y,
      total_delta_x: x - self.start_x,
      total_delta_y: y - self.start_y,
      button: self.button,
      target_id: self.target_id,
      drop_result,
    }
  }
}

#[derive(Default)]
struct ClickTracker {
  pending_click: Option<PendingClick>,
}

impl ClickTracker {
  fn has_pending(&self) -> bool {
    self.pending_click.is_some()
  }

  fn pending_matches(&self, now: Instant, position: (f32, f32), button: MouseButton) -> bool {
    self.pending_click.is_some_and(|pending| {
      pending.button == button
        && now.duration_since(pending.time) <= DOUBLE_CLICK_INTERVAL
        && distance_squared(pending.position, position) <= DOUBLE_CLICK_DISTANCE * DOUBLE_CLICK_DISTANCE
    })
  }

  fn pending_is_due(&self, now: Instant) -> bool {
    self
      .pending_click
      .is_some_and(|pending| now.duration_since(pending.time) > DOUBLE_CLICK_INTERVAL)
  }

  fn set_pending(&mut self, now: Instant, position: (f32, f32), button: MouseButton) {
    self.pending_click = Some(PendingClick {
      time: now,
      position,
      button,
    });
  }

  fn take_pending(&mut self) -> Option<PendingClick> {
    self.pending_click.take()
  }
}

#[derive(Clone, Copy)]
struct PendingClick {
  time: Instant,
  position: (f32, f32),
  button: MouseButton,
}

struct ClickPress {
  position: (f32, f32),
  button: MouseButton,
  target_ids: Vec<NodeId>,
}

#[derive(Default)]
struct TextClickTracker {
  previous: Option<TextClick>,
}

impl TextClickTracker {
  fn record(&mut self, now: Instant, position: (f32, f32), button: MouseButton, target_id: NodeId) -> u8 {
    let count = self
      .previous
      .filter(|previous| {
        previous.button == button
          && previous.target_id == target_id
          && now.duration_since(previous.time) <= DOUBLE_CLICK_INTERVAL
          && distance_squared(previous.position, position) <= DOUBLE_CLICK_DISTANCE * DOUBLE_CLICK_DISTANCE
      })
      .map(|previous| (previous.count + 1).min(3))
      .unwrap_or(1);

    self.previous = Some(TextClick {
      time: now,
      position,
      button,
      target_id,
      count,
    });

    count
  }
}

#[derive(Clone, Copy)]
struct TextClick {
  time: Instant,
  position: (f32, f32),
  button: MouseButton,
  target_id: NodeId,
  count: u8,
}

#[derive(Clone, Copy)]
struct SuppressedClick {
  time: Instant,
  position: (f32, f32),
  button: MouseButton,
}

fn distance_squared(a: (f32, f32), b: (f32, f32)) -> f32 {
  let dx = a.0 - b.0;
  let dy = a.1 - b.1;
  dx * dx + dy * dy
}

impl Drop for Tree {
  fn drop(&mut self) {
    if let Some(component) = self.root_component.take() {
      component.on_unmounted();
    }
  }
}

fn find_element_recursive(
  node: &mut Node,
  layout: &LayoutResult,
  abs_x: f32,
  abs_y: f32,
  parent_x: f32,
  parent_y: f32,
  predicate: &impl for<'b> Fn(ElementRef<'b>) -> bool,
) -> Option<OwnedElementRef> {
  let element = ElementRef::new(node);
  let rect = ElementRect {
    x: abs_x,
    y: abs_y,
    relative_x: abs_x - parent_x,
    relative_y: abs_y - parent_y,
    width: layout.size.width,
    height: layout.size.height,
  };

  if predicate(element) {
    let element_ref = node.element_ref_handle();
    element_ref.update(
      rect.x,
      rect.y,
      rect.relative_x,
      rect.relative_y,
      rect.width,
      rect.height,
    );
    return Some(element_ref);
  }

  for (child_layout, child_node) in layout.children.iter().zip(node.children.iter_mut()) {
    if let Some(found) = find_element_recursive(
      child_node,
      &child_layout.result,
      abs_x + child_layout.offset.x,
      abs_y + child_layout.offset.y,
      abs_x,
      abs_y,
      predicate,
    ) {
      return Some(found);
    }
  }

  None
}

fn find_node_by_id(node: &Node, id: NodeId) -> Option<&Node> {
  if node.node_id() == id {
    return Some(node);
  }

  for child in node.children() {
    if let Some(found) = find_node_by_id(child, id) {
      return Some(found);
    }
  }

  None
}

fn find_node_by_path<'a>(node: &'a Node, path: &[usize]) -> Option<&'a Node> {
  let mut current = node;
  for &index in path {
    current = current.children().get(index)?;
  }
  Some(current)
}

#[cfg(feature = "devtools")]
#[derive(Clone, Copy)]
struct DevtoolsOverlayRect {
  x: f32,
  y: f32,
  width: f32,
  height: f32,
}

#[cfg(feature = "devtools")]
#[derive(Clone, Copy)]
struct DevtoolsOverlayTarget {
  outer: DevtoolsOverlayRect,
  inner: Option<DevtoolsOverlayRect>,
}

#[cfg(feature = "devtools")]
impl Tree {
  fn devtools_overlay_target(&self, root: &Node, layout: &LayoutResult) -> Option<DevtoolsOverlayTarget> {
    let path = self.debug_overlay_node_path.as_deref()?;
    devtools_overlay_target_at_path(root, layout, path, 0.0, 0.0)
  }
}

#[cfg(feature = "devtools")]
fn devtools_overlay_target_at_path(
  node: &Node,
  layout: &LayoutResult,
  path: &[usize],
  abs_x: f32,
  abs_y: f32,
) -> Option<DevtoolsOverlayTarget> {
  if let Some((&index, rest)) = path.split_first() {
    let child_layout = layout.children.get(index)?;
    let child_node = node.children().get(index)?;
    return devtools_overlay_target_at_path(
      child_node,
      &child_layout.result,
      rest,
      abs_x + child_layout.offset.x,
      abs_y + child_layout.offset.y,
    );
  }

  let outer = DevtoolsOverlayRect {
    x: abs_x,
    y: abs_y,
    width: layout.size.width,
    height: layout.size.height,
  };
  let inner = if node.padding != crate::node::padding::Padding::default() {
    layout.children.first().map(|child| DevtoolsOverlayRect {
      x: abs_x + child.offset.x,
      y: abs_y + child.offset.y,
      width: child.result.size.width,
      height: child.result.size.height,
    })
  } else {
    None
  };

  Some(DevtoolsOverlayTarget { outer, inner })
}

fn find_path_by_id(node: &Node, id: NodeId) -> Option<Vec<usize>> {
  fn visit(node: &Node, id: NodeId, path: &mut Vec<usize>) -> bool {
    if node.node_id() == id {
      return true;
    }

    for (index, child) in node.children().iter().enumerate() {
      path.push(index);
      if visit(child, id, path) {
        return true;
      }
      path.pop();
    }

    false
  }

  let mut path = Vec::new();
  visit(node, id, &mut path).then_some(path)
}

#[derive(Clone, Copy)]
struct FocusTarget {
  input_id: NodeId,
  event_id: NodeId,
}

#[derive(Clone)]
#[cfg(feature = "form")]
struct FocusCandidate {
  input_id: NodeId,
  event_id: NodeId,
  tab_index: i32,
  order: usize,
}

#[cfg(feature = "form")]
impl FocusCandidate {
  fn target(&self) -> FocusTarget {
    FocusTarget {
      input_id: self.input_id,
      event_id: self.event_id,
    }
  }
}

fn set_node_hovered(node: &Node, hovered: bool) {
  node.set_style_hovered(hovered);
  if let Some(state) = node.slider_state() {
    state.set_hovered(hovered);
  }
  if let Some(ref state) = node.interaction {
    state.set_hovered(hovered);
  }
  if let Some(ref element_ref) = node.element_ref {
    element_ref.set_hovered(hovered);
  }
}

fn set_node_active(node: &Node, active: bool) {
  node.set_style_active(active);
  if let Some(ref state) = node.interaction {
    state.set_active(active);
  }
  if let Some(ref element_ref) = node.element_ref {
    element_ref.set_active(active);
  }
}

fn set_node_focused(node: &Node, focused: bool) {
  node.set_style_focused(focused);
  if let Some(ref state) = node.interaction {
    state.set_focused(focused);
  }
  if let Some(ref element_ref) = node.element_ref {
    element_ref.set_focused(focused);
  }
}

fn reset_element_ref_flags_recursive(node: &Node) {
  node.set_style_hovered(false);
  node.set_style_active(false);
  node.set_style_focused(false);
  if let Some(ref element_ref) = node.element_ref {
    element_ref.set_hovered(false);
    element_ref.set_active(false);
    element_ref.set_focused(false);
  }
  for child in node.children() {
    reset_element_ref_flags_recursive(child);
  }
}

fn replace_live_component_slot(node: &mut Node, slot_id: u64, replacement: Node, id_gen: &IdGenerator) -> bool {
  let mut replacement = Some(replacement);
  replace_live_component_slot_in(node, slot_id, &mut replacement, id_gen)
}

fn replace_live_component_slot_in(
  node: &mut Node,
  slot_id: u64,
  replacement: &mut Option<Node>,
  id_gen: &IdGenerator,
) -> bool {
  if node.component_slot_id() == Some(slot_id) {
    let mut replacement = replacement
      .take()
      .expect("component replacement should be available when matching slot is found");
    reset_element_ref_flags_recursive(node);
    replacement.preserve_runtime_state_from(node);
    replacement.preserve_ids_from(node);
    node.free_ids(id_gen);
    replacement.assign_ids(id_gen);
    *node = replacement;
    return true;
  }

  for child in &mut node.children {
    if replace_live_component_slot_in(child, slot_id, replacement, id_gen) {
      return true;
    }
  }

  false
}

fn has_dirty_element_ref_recursive(node: &Node) -> bool {
  node
    .element_ref
    .as_ref()
    .is_some_and(|element_ref| element_ref.has_layout_dirty())
    || node.children().iter().any(has_dirty_element_ref_recursive)
}

fn update_element_refs_recursive(
  node: &mut Node,
  layout: &LayoutResult,
  abs_x: f32,
  abs_y: f32,
  parent_x: f32,
  parent_y: f32,
) {
  if let Some(element_ref) = &node.element_ref {
    element_ref.update(
      abs_x,
      abs_y,
      abs_x - parent_x,
      abs_y - parent_y,
      layout.size.width,
      layout.size.height,
    );
  }

  for (child_layout, child_node) in layout.children.iter().zip(node.children.iter_mut()) {
    update_element_refs_recursive(
      child_node,
      &child_layout.result,
      abs_x + child_layout.offset.x,
      abs_y + child_layout.offset.y,
      abs_x,
      abs_y,
    );
  }
}

#[cfg(feature = "form")]
fn first_form_path(root: &Node) -> Option<Vec<usize>> {
  if root.events.on_submit.is_some() {
    return Some(Vec::new());
  }
  for (index, child) in root.children().iter().enumerate() {
    if let Some(mut path) = first_form_path(child) {
      path.insert(0, index);
      return Some(path);
    }
  }
  None
}

#[cfg(feature = "form")]
fn nearest_form_path_for_path(root: &Node, path: &[usize]) -> Option<Vec<usize>> {
  for len in (0..=path.len()).rev() {
    let candidate = &path[..len];
    if find_node_by_path(root, candidate).is_some_and(|node| node.events.on_submit.is_some()) {
      return Some(candidate.to_vec());
    }
  }
  None
}

#[cfg(feature = "form")]
fn nearest_form_submission(
  root: &Node,
  path: &[usize],
) -> Option<(Arc<dyn Fn(crate::node::FormData) + Send + Sync>, crate::node::FormData)> {
  let form_path = nearest_form_path_for_path(root, path)?;
  let form = find_node_by_path(root, &form_path)?;
  let handler = form.submit_handler()?;
  let mut data = crate::node::FormData::new();
  collect_form_data(form, &mut data);
  Some((handler, data))
}

#[cfg(feature = "form")]
fn collect_form_data(node: &Node, data: &mut crate::node::FormData) {
  if let Some(name) = node.form_name_value() {
    match node.node_kind() {
      NodeKind::TextInput { state, .. } => data.append(name, state.value()),
      NodeKind::Checkbox { state } => {
        if state.is_checked() {
          data.append(name, "on");
        }
      }
      NodeKind::Slider { state } => data.append(name, state.value().to_string()),
      NodeKind::Select { state } => {
        for label in state.selected_labels() {
          data.append(name, label.to_string());
        }
      }
      _ => {}
    }
  }

  for child in node.children() {
    collect_form_data(child, data);
  }
}

#[cfg(feature = "form")]
fn collect_focus_candidates(node: &Node, focus_event_id: Option<NodeId>, candidates: &mut Vec<FocusCandidate>) {
  let focus_event_id = if node.events.on_focus.is_some() || node.events.on_blur.is_some() {
    Some(node.node_id())
  } else {
    focus_event_id
  };
  let tab_index = node.tab_index_value().unwrap_or(0);
  if tab_index >= 0 && is_tabbable(node) {
    candidates.push(FocusCandidate {
      input_id: node.node_id(),
      event_id: focus_event_id.unwrap_or_else(|| node.node_id()),
      tab_index,
      order: candidates.len(),
    });
    if node.button_kind_value().is_some() {
      return;
    }
  }

  for child in node.children() {
    collect_focus_candidates(child, focus_event_id, candidates);
  }
}

#[cfg(feature = "form")]
fn sort_focus_candidates(candidates: &mut [FocusCandidate]) {
  candidates.sort_by_key(|candidate| {
    let positive_rank = if candidate.tab_index > 0 { 0 } else { 1 };
    (positive_rank, candidate.tab_index.max(0), candidate.order)
  });
}

#[cfg(feature = "form")]
fn is_tabbable(node: &Node) -> bool {
  node.is_focusable()
    || node.button_kind_value().is_some()
    || matches!(
      node.node_kind(),
      NodeKind::TextInput { .. } | NodeKind::Checkbox { .. } | NodeKind::Slider { .. }
    )
}

fn dispatch_builtin_pointer(
  hits: &[(&Node, crate::app::hit_test::HitRect)],
  x: f32,
  click: bool,
) -> Option<FocusTarget> {
  if !click {
    return None;
  }

  let event_id = hits
    .iter()
    .find(|(node, _)| node.events.on_focus.is_some() || node.events.on_blur.is_some())
    .map(|(node, _)| node.node_id());

  for (node, rect) in hits {
    if node.button_kind_value().is_some() {
      return Some(FocusTarget {
        input_id: node.node_id(),
        event_id: event_id.unwrap_or_else(|| node.node_id()),
      });
    }
    match node.node_kind() {
      NodeKind::TextInput { .. } => {
        return Some(FocusTarget {
          input_id: node.node_id(),
          event_id: event_id.unwrap_or_else(|| node.node_id()),
        });
      }
      NodeKind::Checkbox { state } => {
        state.toggle();
        return Some(FocusTarget {
          input_id: node.node_id(),
          event_id: event_id.unwrap_or_else(|| node.node_id()),
        });
      }
      NodeKind::Slider { state } => {
        let (track_rect, thumb_rect) = state.part_rects(
          rect.x,
          rect.y,
          rect.width,
          rect.height,
          node.is_style_hovered(),
          DEFAULT_SLIDER_THUMB_MIN_SIZE,
        );
        let ratio = state.pointer_ratio(x, track_rect, thumb_rect);
        state.set_from_ratio(ratio);
        return Some(FocusTarget {
          input_id: node.node_id(),
          event_id: event_id.unwrap_or_else(|| node.node_id()),
        });
      }
      _ => {}
    }
  }

  None
}

fn selected_selectable_text(node: &Node) -> Option<String> {
  if let NodeKind::Text { state, .. } = node.node_kind() {
    let value = node.text_content().unwrap_or_default();
    if let Some(selected) = state.selected_text(value) {
      return Some(selected);
    }
  }

  for child in node.children() {
    if let Some(selected) = selected_selectable_text(child) {
      return Some(selected);
    }
  }

  None
}

fn find_slider_by_y_recursive<'a>(
  node: &'a Node,
  layout: &'a LayoutResult,
  abs_x: f32,
  abs_y: f32,
  y: f32,
) -> Option<(&'a Node, ElementRect)> {
  for (child_layout, child_node) in layout.children.iter().zip(node.children()) {
    if let Some(found) = find_slider_by_y_recursive(
      child_node,
      &child_layout.result,
      abs_x + child_layout.offset.x,
      abs_y + child_layout.offset.y,
      y,
    ) {
      return Some(found);
    }
  }

  let rect = ElementRect {
    x: abs_x,
    y: abs_y,
    relative_x: 0.0,
    relative_y: 0.0,
    width: layout.size.width,
    height: layout.size.height,
  };

  if matches!(node.node_kind(), NodeKind::Slider { .. }) && y >= rect.y && y <= rect.y + rect.height {
    return Some((node, rect));
  }

  None
}

fn fire_keyboard_recursive(node: &Node, evt: &mut KeyboardEvent) {
  evt.target_id = node.node_id();
  if let Some(ref handler) = node.events.on_key_down {
    handler(evt);
  }
  for child in node.children() {
    fire_keyboard_recursive(child, evt);
  }
}

fn ms(duration: Duration) -> f32 {
  duration.as_secs_f32() * 1000.0
}

fn transformed_text_raster_scale(transform: Transform2D) -> f32 {
  let aa = transform.a * transform.a + transform.b * transform.b;
  let bb = transform.c * transform.c + transform.d * transform.d;
  let ab = transform.a * transform.c + transform.b * transform.d;
  let trace = aa + bb;
  let det = aa * bb - ab * ab;
  let discriminant = (trace * trace - 4.0 * det).max(0.0);
  let max_scale = ((trace + discriminant.sqrt()) * 0.5).max(1.0).sqrt();
  max_scale.max(2.0).min(4.0).ceil()
}

#[cfg(feature = "devtools")]
fn push_devtools_overlay(
  rects: &mut Vec<RectCmd>,
  glyphs: &mut Vec<GlyphCmd>,
  glyph_engine: &mut crate::app::glyph_engine::GlyphEngine,
  target: Option<DevtoolsOverlayTarget>,
  base_order: usize,
  scale: f32,
  viewport: Size,
) {
  let Some(target) = target else {
    return;
  };

  let outer = scale_devtools_rect(target.outer, scale);
  let inner = target.inner.map(|rect| scale_devtools_rect(rect, scale));
  let order = base_order + 10_000;

  if let Some(inner) = inner {
    rects.push(devtools_overlay_rect_cmd(
      order,
      outer,
      Color::new(251, 146, 60, 48),
      Color::new(251, 146, 60, 235),
    ));
    rects.push(devtools_overlay_rect_cmd(
      order + 1,
      inner,
      Color::new(96, 165, 250, 48),
      Color::new(96, 165, 250, 245),
    ));
  } else {
    rects.push(devtools_overlay_rect_cmd(
      order,
      outer,
      Color::new(96, 165, 250, 42),
      Color::new(96, 165, 250, 245),
    ));
  }

  push_devtools_size_label(
    rects,
    glyphs,
    glyph_engine,
    target.outer,
    outer,
    order + 2,
    scale,
    viewport,
  );
}

#[cfg(feature = "devtools")]
fn scale_devtools_rect(rect: DevtoolsOverlayRect, scale: f32) -> DevtoolsOverlayRect {
  DevtoolsOverlayRect {
    x: rect.x * scale,
    y: rect.y * scale,
    width: rect.width * scale,
    height: rect.height * scale,
  }
}

#[cfg(feature = "devtools")]
fn devtools_overlay_rect_cmd(order: usize, rect: DevtoolsOverlayRect, color: Color, stroke_color: Color) -> RectCmd {
  RectCmd {
    order,
    x: rect.x,
    y: rect.y,
    width: rect.width.max(0.0),
    height: rect.height.max(0.0),
    color,
    radii: [0.0; 4],
    stroke: [1.0; 4],
    stroke_color,
    transform: [1.0, 0.0, 0.0, 1.0],
    transform_origin: [0.0, 0.0],
    clip: ClipRect::default(),
    gradient: None,
  }
}

#[cfg(feature = "devtools")]
fn push_devtools_size_label(
  rects: &mut Vec<RectCmd>,
  glyphs: &mut Vec<GlyphCmd>,
  glyph_engine: &mut crate::app::glyph_engine::GlyphEngine,
  logical_rect: DevtoolsOverlayRect,
  physical_rect: DevtoolsOverlayRect,
  order: usize,
  scale: f32,
  viewport: Size,
) {
  let label = format!("{:.0} x {:.0}", logical_rect.width, logical_rect.height);
  let style = TextStyle {
    font_size: 11.0 * scale,
    weight: FontWeight::Medium,
    color: Color::new(229, 231, 235, 255),
    ..Default::default()
  };
  let text_size = glyph_engine.measure_text(&label, &style, f32::MAX);
  let padding_x = 6.0 * scale;
  let padding_y = 3.0 * scale;
  let label_w = text_size.width + padding_x * 2.0;
  let label_h = text_size.height + padding_y * 2.0;
  let gap = 4.0 * scale;
  let outside_x = physical_rect.x.max(0.0);
  let outside_y = physical_rect.y + physical_rect.height + gap;
  let outside_fits =
    outside_x + label_w <= viewport.width && outside_y >= 0.0 && outside_y + label_h <= viewport.height;
  let (label_x, label_y) = if outside_fits {
    (outside_x, outside_y)
  } else {
    let max_x = (viewport.width - label_w).max(0.0);
    let max_y = (viewport.height - label_h).max(0.0);
    (
      (physical_rect.x + gap).clamp(0.0, max_x),
      (physical_rect.y + gap).clamp(0.0, max_y),
    )
  };

  rects.push(RectCmd {
    order,
    x: label_x,
    y: label_y,
    width: label_w,
    height: label_h,
    color: Color::new(17, 24, 39, 235),
    radii: [3.0 * scale; 4],
    stroke: [1.0; 4],
    stroke_color: Color::new(96, 165, 250, 235),
    transform: [1.0, 0.0, 0.0, 1.0],
    transform_origin: [0.0, 0.0],
    clip: ClipRect::default(),
    gradient: None,
  });

  let mut label_glyphs =
    glyph_engine.rasterize_text(&label, &style, f32::MAX, label_x + padding_x, label_y + padding_y);
  for glyph in &mut label_glyphs {
    glyph.order = order + 1;
    glyph.clip = ClipRect::default();
  }
  glyphs.extend(label_glyphs);
}

fn push_perf_meter(
  rects: &mut Vec<RectCmd>,
  glyphs: &mut Vec<GlyphCmd>,
  glyph_engine: &mut crate::app::glyph_engine::GlyphEngine,
  stats: Option<PerfMeterStats>,
  order: usize,
  scale: f32,
  viewport: Size,
) {
  let Some(stats) = stats else {
    return;
  };

  let panel_w = 160.0 * scale;
  let panel_h = 198.0 * scale;
  let margin_top = 12.0 * scale;
  let margin_right = 16.0 * scale;
  let padding_x = 8.0 * scale;
  let padding_y = 8.0 * scale;
  let row_h = 15.0 * scale;
  let x = (viewport.width - panel_w - margin_right).max(0.0);
  let y = margin_top.max(0.0);

  rects.push(RectCmd {
    order,
    x,
    y,
    width: panel_w,
    height: panel_h,
    color: Color::from_hex("#111827"),
    radii: [6.0 * scale; 4],
    stroke: [1.0; 4],
    stroke_color: Color::from_hex("#374151"),
    transform: [1.0, 0.0, 0.0, 1.0],
    transform_origin: [0.0, 0.0],
    clip: ClipRect::default(),
    gradient: None,
  });

  let rows = [
    ("FPS", stats.fps.to_string(), FontWeight::Bold),
    ("total", format!("{:.2} ms", stats.total_ms), FontWeight::Normal),
    ("layout", format!("{:.2} ms", stats.layout_ms), FontWeight::Normal),
    (
      "resolve",
      format!("{:.2} ms", stats.quad_resolve_ms),
      FontWeight::Normal,
    ),
    ("glyph", format!("{:.2} ms", stats.glyph_ms), FontWeight::Normal),
    (
      "acquire",
      format!("{:.2} ms", stats.render_acquire_ms),
      FontWeight::Normal,
    ),
    (
      "upload",
      format!("{:.2} ms", stats.render_upload_ms),
      FontWeight::Normal,
    ),
    (
      "encode",
      format!("{:.2} ms", stats.render_encode_ms),
      FontWeight::Normal,
    ),
    (
      "submit",
      format!("{:.2} ms", stats.render_submit_ms),
      FontWeight::Normal,
    ),
    (
      "present",
      format!("{:.2} ms", stats.render_present_ms),
      FontWeight::Normal,
    ),
    ("quads", stats.quad_count.to_string(), FontWeight::Normal),
    ("glyphs", stats.glyph_count.to_string(), FontWeight::Normal),
  ];

  for (index, (label, value, value_weight)) in rows.iter().enumerate() {
    let row_y = y + padding_y + index as f32 * row_h;
    push_raw_text(
      glyphs,
      glyph_engine,
      label,
      x + padding_x,
      row_y,
      10.0 * scale,
      FontWeight::Normal,
      Color::from_hex("#e5e7eb"),
      order + 1,
    );
    let value_size = measure_raw_text(glyph_engine, value, 10.0 * scale, *value_weight);
    push_raw_text(
      glyphs,
      glyph_engine,
      value,
      x + panel_w - padding_x - value_size.width,
      row_y,
      10.0 * scale,
      *value_weight,
      Color::from_hex("#e5e7eb"),
      order + 1,
    );
  }
}

fn measure_raw_text(
  glyph_engine: &mut crate::app::glyph_engine::GlyphEngine,
  text: &str,
  font_size: f32,
  weight: FontWeight,
) -> Size {
  let style = TextStyle {
    font_size,
    weight,
    color: Color::new(255, 255, 255, 255),
    ..Default::default()
  };
  glyph_engine.measure_text(text, &style, f32::MAX)
}

#[allow(clippy::too_many_arguments)]
fn push_raw_text(
  glyphs: &mut Vec<GlyphCmd>,
  glyph_engine: &mut crate::app::glyph_engine::GlyphEngine,
  text: &str,
  x: f32,
  y: f32,
  font_size: f32,
  weight: FontWeight,
  color: Color,
  order: usize,
) {
  let style = TextStyle {
    font_size,
    weight,
    color,
    ..Default::default()
  };
  let mut text_glyphs = glyph_engine.rasterize_text(text, &style, f32::MAX, x, y);
  for glyph in &mut text_glyphs {
    glyph.order = order;
    glyph.clip = ClipRect::default();
  }
  glyphs.extend(text_glyphs);
}

fn apply_opacity(color: Color, opacity: f32) -> Color {
  if opacity >= 1.0 {
    return color;
  }
  let a = (color.a() as f32 * opacity.clamp(0.0, 1.0)).round() as u8;
  Color::new(color.r(), color.g(), color.b(), a)
}

fn apply_opacity_gradient(gradient: &RenderGradient, opacity: f32) -> RenderGradient {
  if opacity >= 1.0 {
    return gradient.clone();
  }
  let factor = opacity.clamp(0.0, 1.0);
  let mut gradient = gradient.clone();
  for stop in &mut gradient.stops {
    stop.color[3] *= factor;
  }
  gradient
}

fn scaled_radii(border_radius: Option<BorderRadius>, scale: f32, width: f32, height: f32) -> [f32; 4] {
  let max_r = width.min(height) * 0.5;
  border_radius
    .map(|r| {
      [
        (r.top_left * scale).min(max_r),
        (r.top_right * scale).min(max_r),
        (r.bottom_right * scale).min(max_r),
        (r.bottom_left * scale).min(max_r),
      ]
    })
    .unwrap_or([0.0; 4])
}

fn push_border_rects(
  rects: &mut Vec<RectCmd>,
  order: usize,
  base_x: f32,
  base_y: f32,
  base_w: f32,
  base_h: f32,
  scale: f32,
  border_radius: Option<BorderRadius>,
  borders: ResolvedBorders,
  opacity: f32,
  transform: [f32; 4],
  transform_origin: [f32; 2],
  clip: ClipRect,
) {
  let origin_abs = [base_x + transform_origin[0], base_y + transform_origin[1]];
  let sides = [borders.top, borders.right, borders.bottom, borders.left];
  let mut used = [false; 4];

  for side in 0..4 {
    let Some(border) = sides[side] else {
      continue;
    };
    if used[side] {
      continue;
    }

    let mut stroke = [0.0; 4];
    for other_side in side..4 {
      if sides[other_side] == Some(border) {
        used[other_side] = true;
        stroke[other_side] = border.width * scale;
      }
    }

    push_border_rect(
      rects,
      order,
      base_x,
      base_y,
      base_w,
      base_h,
      scale,
      border_radius,
      border,
      stroke,
      opacity,
      transform,
      origin_abs,
      clip,
    );
  }
}

#[allow(clippy::too_many_arguments)]
fn push_border_rect(
  rects: &mut Vec<RectCmd>,
  order: usize,
  base_x: f32,
  base_y: f32,
  base_w: f32,
  base_h: f32,
  scale: f32,
  border_radius: Option<BorderRadius>,
  border: ResolvedBorder,
  stroke: [f32; 4],
  opacity: f32,
  transform: [f32; 4],
  origin_abs: [f32; 2],
  clip: ClipRect,
) {
  if stroke.iter().all(|width| *width <= 0.0) {
    return;
  }

  let (mut x, mut y, mut w, mut h) = (base_x, base_y, base_w, base_h);
  match border.placement {
    BorderPlacement::Outside => {
      x -= stroke[3];
      y -= stroke[0];
      w += stroke[1] + stroke[3];
      h += stroke[0] + stroke[2];
    }
    BorderPlacement::Center => {
      x -= stroke[3] * 0.5;
      y -= stroke[0] * 0.5;
      w += (stroke[1] + stroke[3]) * 0.5;
      h += (stroke[0] + stroke[2]) * 0.5;
    }
    _ => {}
  }

  rects.push(RectCmd {
    order,
    x,
    y,
    width: w,
    height: h,
    color: TRANSPARENT_COLOR,
    radii: scaled_radii(border_radius, scale, w, h),
    stroke,
    stroke_color: apply_opacity(border.color, opacity),
    transform,
    transform_origin: [origin_abs[0] - x, origin_abs[1] - y],
    clip,
    gradient: None,
  });
}

fn rect_intersects_clip(x: f32, y: f32, width: f32, height: f32, clip: ClipRect) -> bool {
  x < clip.x + clip.width && x + width > clip.x && y < clip.y + clip.height && y + height > clip.y
}

fn expand_text_clip_for_rasterization(clip: ClipRect) -> ClipRect {
  if !clip.active {
    return clip;
  }

  const RASTERIZATION_SLOP_PX: f32 = 1.0;
  ClipRect {
    x: clip.x - RASTERIZATION_SLOP_PX,
    y: clip.y - RASTERIZATION_SLOP_PX,
    width: clip.width + RASTERIZATION_SLOP_PX * 2.0,
    height: clip.height + RASTERIZATION_SLOP_PX * 2.0,
    active: true,
  }
}

#[cfg(test)]
mod tests {
  use crate::{
    app::runtime::{expand_text_clip_for_rasterization, transformed_text_raster_scale},
    layout::quad::ClipRect,
    node::transform::Transform2D,
  };

  #[test]
  fn text_clip_expands_by_one_physical_pixel_for_glyph_rasterization() {
    let clip = ClipRect {
      x: 10.0,
      y: 20.0,
      width: 30.0,
      height: 40.0,
      active: true,
    };

    let expanded = expand_text_clip_for_rasterization(clip);

    assert_eq!(expanded.x, 9.0);
    assert_eq!(expanded.y, 19.0);
    assert_eq!(expanded.width, 32.0);
    assert_eq!(expanded.height, 42.0);
    assert!(expanded.active);
  }

  #[test]
  fn inactive_text_clip_stays_inactive() {
    let clip = ClipRect::default();

    let expanded = expand_text_clip_for_rasterization(clip);

    assert!(!expanded.active);
  }

  #[test]
  fn transformed_text_bitmap_mode_uses_moderate_minimum_raster_scale() {
    let transform = Transform2D::rotate_deg(-2.0).then(&Transform2D::scale(1.02, 1.02));

    assert_eq!(transformed_text_raster_scale(transform), 2.0);
  }
}
