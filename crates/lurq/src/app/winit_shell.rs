use std::time::{Duration, Instant};

use winit::{
  application::ApplicationHandler,
  event::{ElementState, MouseScrollDelta, TouchPhase, WindowEvent},
  event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
  keyboard::{Key, ModifiersState, NamedKey, PhysicalKey},
  window::{CursorIcon as WinitCursorIcon, Window, WindowAttributes, WindowId},
};

#[cfg(feature = "devtools")]
use crate::app::render_engine::RenderEngine;
use crate::{
  app::{
    App, Tree,
    events::{MouseButton, ScrollPhase},
  },
  node::CursorIcon,
};

type TickFn = Box<dyn FnMut(&mut Tree)>;
const TICK_INTERVAL: Duration = Duration::from_millis(16);

#[cfg(feature = "devtools")]
type RenderEngineFactory = Box<dyn Fn() -> Box<dyn RenderEngine>>;

pub struct WinitWindow {
  app: App,
  tree: Tree,
  attrs: WindowAttributes,
  on_tick: Option<TickFn>,
  #[cfg(feature = "devtools")]
  devtools: Option<DevToolsConfig>,
}

#[cfg(feature = "devtools")]
struct DevToolsConfig {
  attrs: WindowAttributes,
  render_engine_factory: RenderEngineFactory,
}

impl WinitWindow {
  pub fn new(app: App, tree: Tree) -> Self {
    Self {
      app,
      tree,
      attrs: WindowAttributes::default(),
      on_tick: None,
      #[cfg(feature = "devtools")]
      devtools: None,
    }
  }

  pub fn with_title(mut self, title: &str) -> Self {
    self.attrs = self.attrs.with_title(title);
    self
  }

  pub fn with_size(mut self, width: u32, height: u32) -> Self {
    self.attrs = self.attrs.with_inner_size(winit::dpi::LogicalSize::new(width, height));
    self
  }

  pub fn with_min_size(mut self, width: u32, height: u32) -> Self {
    self.attrs = self
      .attrs
      .with_min_inner_size(winit::dpi::LogicalSize::new(width, height));
    self
  }

  pub fn with_max_size(mut self, width: u32, height: u32) -> Self {
    self.attrs = self
      .attrs
      .with_max_inner_size(winit::dpi::LogicalSize::new(width, height));
    self
  }

  pub fn with_resizable(mut self, resizable: bool) -> Self {
    self.attrs = self.attrs.with_resizable(resizable);
    self
  }

  pub fn with_decorations(mut self, decorations: bool) -> Self {
    self.attrs = self.attrs.with_decorations(decorations);
    self
  }

  pub fn with_transparent(mut self, transparent: bool) -> Self {
    self.attrs = self.attrs.with_transparent(transparent);
    self
  }

  pub fn on_tick<F>(mut self, tick: F) -> Self
  where
    F: FnMut(&mut Tree) + 'static,
  {
    self.on_tick = Some(Box::new(tick));
    self
  }

  #[cfg(feature = "devtools")]
  pub fn with_devtools<F>(mut self, render_engine_factory: F) -> Self
  where
    F: Fn() -> Box<dyn RenderEngine> + 'static,
  {
    self.devtools = Some(DevToolsConfig {
      attrs: WindowAttributes::default()
        .with_title("lurq DevTools")
        .with_inner_size(winit::dpi::LogicalSize::new(1440, 900)),
      render_engine_factory: Box::new(render_engine_factory),
    });
    self
  }

  pub fn run(self) {
    let event_loop = EventLoop::new().unwrap();

    #[cfg(feature = "devtools")]
    let (tree, devtools) = {
      let mut tree = self.tree;
      let devtools = if let Some(config) = self.devtools {
        tree.mount_devtools(self.app.theme().clone(), (config.render_engine_factory)());
        Some(ManagedDevtoolsWindow::new(config.attrs))
      } else {
        None
      };
      (tree, devtools)
    };
    #[cfg(not(feature = "devtools"))]
    let tree = self.tree;

    let mut handler = WinitHandler {
      app: self.app,
      main: ManagedWindow::new(tree, self.attrs, self.on_tick, true),
      #[cfg(feature = "devtools")]
      devtools,
    };
    event_loop.run_app(&mut handler).unwrap();
  }
}

struct ManagedWindow {
  tree: Tree,
  window: Option<Window>,
  cursor_pos: (f64, f64),
  cursor: CursorIcon,
  modifiers: ModifiersState,
  attrs: Option<WindowAttributes>,
  on_tick: Option<TickFn>,
  redraw_pending: bool,
  close_exits: bool,
}

impl ManagedWindow {
  fn new(tree: Tree, attrs: WindowAttributes, on_tick: Option<TickFn>, close_exits: bool) -> Self {
    Self {
      tree,
      window: None,
      cursor_pos: (0.0, 0.0),
      cursor: CursorIcon::Default,
      modifiers: ModifiersState::empty(),
      attrs: Some(attrs),
      on_tick,
      redraw_pending: false,
      close_exits,
    }
  }

  fn window_id(&self) -> Option<WindowId> {
    self.window.as_ref().map(Window::id)
  }

  fn has_tick(&self) -> bool {
    self.on_tick.is_some() || self.tree.perf_overlay_enabled()
  }

  fn create_window(&mut self, event_loop: &ActiveEventLoop) {
    if self.window.is_some() {
      return;
    }

    let attrs = self.attrs.take().unwrap_or_default();
    let window = event_loop.create_window(attrs).unwrap();
    let size = window.inner_size();
    self.tree.set_scale_factor(window.scale_factor() as f32);
    self.tree.resize(size.width, size.height);
    self.window = Some(window);
    self.request_redraw();
  }

  fn check_redraw(&mut self) {
    if self.tree.needs_redraw() {
      self.request_redraw();
    }
  }

  fn request_redraw(&mut self) {
    if !self.redraw_pending
      && let Some(w) = &self.window
    {
      self.redraw_pending = true;
      w.request_redraw();
    }
  }

  fn present_now(&mut self, app: &mut App) -> bool {
    if let Some(w) = &self.window {
      let size = w.inner_size();
      self.tree.set_scale_factor(w.scale_factor() as f32);
      self.tree.resize(size.width, size.height);
      self.redraw_pending = false;
      self.tree.clear_needs_redraw();
      self.tree.pass(app, w);
      self.check_redraw();
      return true;
    }
    false
  }

  fn apply_cursor(&mut self) {
    let cursor = self.tree.cursor();
    if cursor == self.cursor {
      return;
    }

    if let Some(window) = &self.window {
      window.set_cursor(to_winit_cursor(cursor));
    }
    self.cursor = cursor;
  }

  fn tick(&mut self) {
    if let Some(tick) = &mut self.on_tick {
      tick(&mut self.tree);
    }
    self.tree.tick_perf_overlay();
    self.check_redraw();
  }

  fn handle_event(&mut self, app: &mut App, event_loop: &ActiveEventLoop, event: WindowEvent) -> bool {
    match event {
      WindowEvent::CloseRequested => {
        if self.close_exits {
          event_loop.exit();
        } else {
          self.window = None;
          self.redraw_pending = false;
        }
      }
      WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
        self.tree.set_scale_factor(scale_factor as f32);
        if let Some(w) = &self.window {
          let size = w.inner_size();
          self.tree.resize(size.width, size.height);
          self.request_redraw();
        }
      }
      WindowEvent::Resized(size) => {
        self.tree.resize(size.width, size.height);
        self.request_redraw();
      }
      WindowEvent::CursorMoved { position, .. } => {
        self.cursor_pos = (position.x, position.y);
        self.tree.mouse_move(position.x as f32, position.y as f32);
        self.apply_cursor();
        self.check_redraw();
      }
      WindowEvent::CursorLeft { .. } => {
        self.tree.mouse_leave_window();
        self.apply_cursor();
        self.check_redraw();
      }
      WindowEvent::MouseInput { state, button, .. } => {
        let btn = match button {
          winit::event::MouseButton::Left => MouseButton::Left,
          winit::event::MouseButton::Right => MouseButton::Right,
          winit::event::MouseButton::Middle => MouseButton::Middle,
          _ => MouseButton::Left,
        };
        let (x, y) = (self.cursor_pos.0 as f32, self.cursor_pos.1 as f32);
        match state {
          ElementState::Pressed => self.tree.mouse_down(x, y, btn),
          ElementState::Released => {
            self.tree.mouse_up(x, y, btn);
            self.tree.click(x, y, btn);
          }
        }
        self.apply_cursor();
        self.check_redraw();
      }
      WindowEvent::ModifiersChanged(modifiers) => {
        self.modifiers = modifiers.state();
      }
      WindowEvent::KeyboardInput { event, .. } => {
        if matches!(event.state, ElementState::Pressed) {
          self.tree.key_down(
            key_to_string(&event),
            physical_key_to_string(&event.physical_key),
            self.modifiers.shift_key(),
            self.modifiers.control_key(),
            self.modifiers.alt_key(),
          );
          self.apply_cursor();
          self.check_redraw();
        }
      }
      WindowEvent::MouseWheel { delta, phase, .. } => {
        let (mut dx, mut dy) = match delta {
          MouseScrollDelta::LineDelta(x, y) => (x * 40.0, y * 40.0),
          MouseScrollDelta::PixelDelta(p) => (p.x as f32, p.y as f32),
        };
        if self.modifiers.shift_key() && dx == 0.0 {
          dx = dy;
          dy = 0.0;
        }
        let scroll_phase = match phase {
          TouchPhase::Started => ScrollPhase::Start,
          TouchPhase::Moved => ScrollPhase::Scroll,
          TouchPhase::Ended | TouchPhase::Cancelled => ScrollPhase::End,
        };
        self
          .tree
          .scroll(self.cursor_pos.0 as f32, self.cursor_pos.1 as f32, dx, dy, scroll_phase);
        self.apply_cursor();
        self.check_redraw();
      }
      WindowEvent::RedrawRequested => {
        return self.present_now(app);
      }
      _ => {}
    }
    false
  }
}

#[cfg(feature = "devtools")]
struct ManagedDevtoolsWindow {
  window: Option<Window>,
  cursor_pos: (f64, f64),
  cursor: CursorIcon,
  modifiers: ModifiersState,
  attrs: Option<WindowAttributes>,
  redraw_pending: bool,
}

#[cfg(feature = "devtools")]
impl ManagedDevtoolsWindow {
  fn new(attrs: WindowAttributes) -> Self {
    Self {
      window: None,
      cursor_pos: (0.0, 0.0),
      cursor: CursorIcon::Default,
      modifiers: ModifiersState::empty(),
      attrs: Some(attrs),
      redraw_pending: false,
    }
  }

  fn window_id(&self) -> Option<WindowId> {
    self.window.as_ref().map(Window::id)
  }

  fn has_tick(&self, tree: Option<&Tree>) -> bool {
    tree.is_some_and(Tree::perf_overlay_enabled)
  }

  fn create_window(&mut self, event_loop: &ActiveEventLoop, tree: &mut Tree) {
    if self.window.is_some() {
      return;
    }

    let attrs = self.attrs.take().unwrap_or_default();
    let window = event_loop.create_window(attrs).unwrap();
    let size = window.inner_size();
    tree.set_scale_factor(window.scale_factor() as f32);
    tree.resize(size.width, size.height);
    self.window = Some(window);
    self.request_redraw();
  }

  fn check_redraw(&mut self, tree: &Tree) {
    if tree.needs_redraw() {
      self.request_redraw();
    }
  }

  fn request_redraw(&mut self) {
    if !self.redraw_pending
      && let Some(w) = &self.window
    {
      self.redraw_pending = true;
      w.request_redraw();
    }
  }

  fn present_now(&mut self, app: &mut App, tree: &mut Tree) -> bool {
    if let Some(w) = &self.window {
      let size = w.inner_size();
      tree.set_scale_factor(w.scale_factor() as f32);
      tree.resize(size.width, size.height);
      self.redraw_pending = false;
      tree.clear_needs_redraw();
      tree.pass(app, w);
      self.check_redraw(tree);
      return true;
    }
    false
  }

  fn apply_cursor(&mut self, tree: &Tree) {
    let cursor = tree.cursor();
    if cursor == self.cursor {
      return;
    }

    if let Some(window) = &self.window {
      window.set_cursor(to_winit_cursor(cursor));
    }
    self.cursor = cursor;
  }

  fn tick(&mut self, tree: &mut Tree) {
    tree.tick_perf_overlay();
    self.check_redraw(tree);
  }

  fn handle_event(
    &mut self,
    app: &mut App,
    _event_loop: &ActiveEventLoop,
    event: WindowEvent,
    tree: &mut Tree,
  ) -> bool {
    match event {
      WindowEvent::CloseRequested => {
        self.window = None;
        self.redraw_pending = false;
      }
      WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
        tree.set_scale_factor(scale_factor as f32);
        if let Some(w) = &self.window {
          let size = w.inner_size();
          tree.resize(size.width, size.height);
          self.request_redraw();
        }
      }
      WindowEvent::Resized(size) => {
        tree.resize(size.width, size.height);
        self.request_redraw();
      }
      WindowEvent::CursorMoved { position, .. } => {
        self.cursor_pos = (position.x, position.y);
        tree.mouse_move(position.x as f32, position.y as f32);
        self.apply_cursor(tree);
        self.check_redraw(tree);
      }
      WindowEvent::CursorLeft { .. } => {
        tree.mouse_leave_window();
        self.apply_cursor(tree);
        self.check_redraw(tree);
      }
      WindowEvent::MouseInput { state, button, .. } => {
        let btn = match button {
          winit::event::MouseButton::Left => MouseButton::Left,
          winit::event::MouseButton::Right => MouseButton::Right,
          winit::event::MouseButton::Middle => MouseButton::Middle,
          _ => MouseButton::Left,
        };
        let (x, y) = (self.cursor_pos.0 as f32, self.cursor_pos.1 as f32);
        match state {
          ElementState::Pressed => tree.mouse_down(x, y, btn),
          ElementState::Released => {
            tree.mouse_up(x, y, btn);
            tree.click(x, y, btn);
          }
        }
        self.apply_cursor(tree);
        self.check_redraw(tree);
      }
      WindowEvent::ModifiersChanged(modifiers) => {
        self.modifiers = modifiers.state();
      }
      WindowEvent::KeyboardInput { event, .. } => {
        if matches!(event.state, ElementState::Pressed) {
          tree.key_down(
            key_to_string(&event),
            physical_key_to_string(&event.physical_key),
            self.modifiers.shift_key(),
            self.modifiers.control_key(),
            self.modifiers.alt_key(),
          );
          self.apply_cursor(tree);
          self.check_redraw(tree);
        }
      }
      WindowEvent::MouseWheel { delta, phase, .. } => {
        let (mut dx, mut dy) = match delta {
          MouseScrollDelta::LineDelta(x, y) => (x * 40.0, y * 40.0),
          MouseScrollDelta::PixelDelta(p) => (p.x as f32, p.y as f32),
        };
        if self.modifiers.shift_key() && dx == 0.0 {
          dx = dy;
          dy = 0.0;
        }
        let scroll_phase = match phase {
          TouchPhase::Started => ScrollPhase::Start,
          TouchPhase::Moved => ScrollPhase::Scroll,
          TouchPhase::Ended | TouchPhase::Cancelled => ScrollPhase::End,
        };
        tree.scroll(self.cursor_pos.0 as f32, self.cursor_pos.1 as f32, dx, dy, scroll_phase);
        self.apply_cursor(tree);
        self.check_redraw(tree);
      }
      WindowEvent::RedrawRequested => {
        return self.present_now(app, tree);
      }
      _ => {}
    }
    false
  }
}

struct WinitHandler {
  app: App,
  main: ManagedWindow,
  #[cfg(feature = "devtools")]
  devtools: Option<ManagedDevtoolsWindow>,
}

impl WinitHandler {
  #[cfg(feature = "devtools")]
  fn check_devtools_redraw(&mut self) {
    let (Some(devtools_window), Some(devtools_tree)) = (&mut self.devtools, self.main.tree.devtools()) else {
      return;
    };
    devtools_window.check_redraw(devtools_tree);
  }

  #[cfg(feature = "devtools")]
  fn apply_devtools_requests(&mut self) {
    if self.main.tree.apply_devtools_requests() {
      self.main.request_redraw();
    }
    self.check_devtools_redraw();
  }

  #[cfg(feature = "devtools")]
  fn handle_devtools_pick_event(&mut self, event: &WindowEvent) -> bool {
    if !self.main.tree.devtools_pick_mode() {
      return false;
    }

    match event {
      WindowEvent::CursorMoved { position, .. } => {
        self.main.cursor_pos = (position.x, position.y);
        true
      }
      WindowEvent::MouseInput { state, button, .. } => {
        if *button != winit::event::MouseButton::Left {
          return true;
        }

        if matches!(state, ElementState::Pressed) {
          return true;
        }

        let (x, y) = (self.main.cursor_pos.0 as f32, self.main.cursor_pos.1 as f32);
        self.main.tree.pick_devtools_node_at(x, y);
        self.main.request_redraw();
        self.check_devtools_redraw();
        true
      }
      WindowEvent::KeyboardInput { event, .. }
        if matches!(event.state, ElementState::Pressed)
          && matches!(event.logical_key, Key::Named(NamedKey::Escape)) =>
      {
        self.main.tree.cancel_devtools_pick();
        self.check_devtools_redraw();
        true
      }
      _ => false,
    }
  }
}

impl ApplicationHandler for WinitHandler {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    self.main.create_window(event_loop);
    #[cfg(feature = "devtools")]
    if let (Some(devtools), Some(devtools_tree)) = (&mut self.devtools, self.main.tree.devtools_mut()) {
      devtools.create_window(event_loop, devtools_tree);
    }
  }

  fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
    if self.main.window_id() == Some(id) {
      #[cfg(feature = "devtools")]
      if self.handle_devtools_pick_event(&event) {
        return;
      }
      #[cfg(feature = "devtools")]
      let presented = self.main.handle_event(&mut self.app, event_loop, event);
      #[cfg(not(feature = "devtools"))]
      self.main.handle_event(&mut self.app, event_loop, event);
      #[cfg(feature = "devtools")]
      if presented {
        self.check_devtools_redraw();
      }
      return;
    }

    #[cfg(feature = "devtools")]
    if self
      .devtools
      .as_ref()
      .is_some_and(|devtools| devtools.window_id() == Some(id))
    {
      let mut closed = false;
      if let (Some(devtools), Some(devtools_tree)) = (&mut self.devtools, self.main.tree.devtools_mut()) {
        devtools.handle_event(&mut self.app, event_loop, event, devtools_tree);
        closed = devtools.window.is_none();
      }
      if closed {
        if self.main.tree.close_devtools() {
          self.main.request_redraw();
        }
        self.devtools = None;
      } else {
        self.apply_devtools_requests();
      }
    }
  }

  fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
    self.main.tick();
    #[cfg(feature = "devtools")]
    if let (Some(devtools), Some(devtools_tree)) = (&mut self.devtools, self.main.tree.devtools_mut()) {
      devtools.tick(devtools_tree);
    }

    #[cfg(feature = "devtools")]
    let devtools_has_tick = self
      .devtools
      .as_ref()
      .is_some_and(|devtools| devtools.has_tick(self.main.tree.devtools()));
    #[cfg(not(feature = "devtools"))]
    let devtools_has_tick = false;

    if self.main.has_tick() || devtools_has_tick {
      event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + TICK_INTERVAL));
    } else {
      event_loop.set_control_flow(ControlFlow::Wait);
    }
  }
}

fn key_to_string(event: &winit::event::KeyEvent) -> String {
  match &event.logical_key {
    Key::Character(text) => event
      .text
      .as_ref()
      .map(ToString::to_string)
      .unwrap_or_else(|| text.to_string()),
    Key::Named(named) => named_key_to_string(*named).to_owned(),
    Key::Dead(Some(ch)) => ch.to_string(),
    Key::Dead(None) => String::new(),
    Key::Unidentified(_) => event.text.as_ref().map(ToString::to_string).unwrap_or_default(),
  }
}

fn named_key_to_string(key: NamedKey) -> &'static str {
  match key {
    NamedKey::Backspace => "Backspace",
    NamedKey::Delete => "Delete",
    NamedKey::ArrowLeft => "ArrowLeft",
    NamedKey::ArrowRight => "ArrowRight",
    NamedKey::ArrowUp => "ArrowUp",
    NamedKey::ArrowDown => "ArrowDown",
    NamedKey::Home => "Home",
    NamedKey::End => "End",
    NamedKey::Enter => "Enter",
    NamedKey::Space => " ",
    _ => "",
  }
}

fn physical_key_to_string(key: &PhysicalKey) -> String {
  match key {
    PhysicalKey::Code(code) => format!("{code:?}"),
    PhysicalKey::Unidentified(_) => String::new(),
  }
}

fn to_winit_cursor(cursor: CursorIcon) -> WinitCursorIcon {
  match cursor {
    CursorIcon::Default => WinitCursorIcon::Default,
    CursorIcon::ContextMenu => WinitCursorIcon::ContextMenu,
    CursorIcon::Help => WinitCursorIcon::Help,
    CursorIcon::Pointer => WinitCursorIcon::Pointer,
    CursorIcon::Progress => WinitCursorIcon::Progress,
    CursorIcon::Wait => WinitCursorIcon::Wait,
    CursorIcon::Cell => WinitCursorIcon::Cell,
    CursorIcon::Crosshair => WinitCursorIcon::Crosshair,
    CursorIcon::Text => WinitCursorIcon::Text,
    CursorIcon::VerticalText => WinitCursorIcon::VerticalText,
    CursorIcon::Alias => WinitCursorIcon::Alias,
    CursorIcon::Copy => WinitCursorIcon::Copy,
    CursorIcon::Move => WinitCursorIcon::Move,
    CursorIcon::NoDrop => WinitCursorIcon::NoDrop,
    CursorIcon::NotAllowed => WinitCursorIcon::NotAllowed,
    CursorIcon::Grab => WinitCursorIcon::Grab,
    CursorIcon::Grabbing => WinitCursorIcon::Grabbing,
    CursorIcon::EResize => WinitCursorIcon::EResize,
    CursorIcon::NResize => WinitCursorIcon::NResize,
    CursorIcon::NeResize => WinitCursorIcon::NeResize,
    CursorIcon::NwResize => WinitCursorIcon::NwResize,
    CursorIcon::SResize => WinitCursorIcon::SResize,
    CursorIcon::SeResize => WinitCursorIcon::SeResize,
    CursorIcon::SwResize => WinitCursorIcon::SwResize,
    CursorIcon::WResize => WinitCursorIcon::WResize,
    CursorIcon::EwResize => WinitCursorIcon::EwResize,
    CursorIcon::NsResize => WinitCursorIcon::NsResize,
    CursorIcon::NeswResize => WinitCursorIcon::NeswResize,
    CursorIcon::NwseResize => WinitCursorIcon::NwseResize,
    CursorIcon::ColResize => WinitCursorIcon::ColResize,
    CursorIcon::RowResize => WinitCursorIcon::RowResize,
    CursorIcon::AllScroll => WinitCursorIcon::AllScroll,
    CursorIcon::ZoomIn => WinitCursorIcon::ZoomIn,
    CursorIcon::ZoomOut => WinitCursorIcon::ZoomOut,
    CursorIcon::DndAsk => WinitCursorIcon::DndAsk,
    CursorIcon::AllResize => WinitCursorIcon::AllResize,
  }
}
