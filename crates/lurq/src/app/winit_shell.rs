use std::time::{Duration, Instant};

use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawWindowHandle};
use winit::{
  application::ApplicationHandler,
  dpi::{PhysicalPosition, PhysicalSize, Position},
  event::{ElementState, MouseScrollDelta, TouchPhase, WindowEvent},
  event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
  keyboard::{Key, ModifiersState, NamedKey, PhysicalKey},
  window::{
    CursorIcon as WinitCursorIcon, Fullscreen, ResizeDirection as WinitResizeDirection, Window, WindowAttributes,
    WindowId,
  },
};

use crate::{
  app::{
    App, Tree,
    events::{MouseButton, ScrollPhase},
    runtime::{SecondaryWindow, SecondaryWindowMetadata},
    window::{WindowCommand, WindowResizeDirection},
  },
  node::CursorIcon,
};

type TickFn = Box<dyn FnMut(&mut Tree)>;
type PositionChangedFn = Box<dyn FnMut(i32, i32)>;
type SizeChangedFn = Box<dyn FnMut(u32, u32)>;
const TICK_INTERVAL: Duration = Duration::from_millis(16);

pub struct WinitWindow {
  app: App,
  tree: Tree,
  attrs: WindowAttributes,
  on_tick: Option<TickFn>,
  on_position_changed: Option<PositionChangedFn>,
  on_size_changed: Option<SizeChangedFn>,
}

impl WinitWindow {
  pub fn new(app: App, tree: Tree) -> Self {
    Self {
      app,
      tree,
      attrs: WindowAttributes::default(),
      on_tick: None,
      on_position_changed: None,
      on_size_changed: None,
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

  pub fn with_position(mut self, x: i32, y: i32) -> Self {
    self.attrs = self
      .attrs
      .with_position(Position::Physical(PhysicalPosition::new(x, y)));
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
    self.tree.window().set_decorated(decorations);
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

  pub fn on_position_changed<F>(mut self, callback: F) -> Self
  where
    F: FnMut(i32, i32) + 'static,
  {
    self.on_position_changed = Some(Box::new(callback));
    self
  }

  pub fn on_size_changed<F>(mut self, callback: F) -> Self
  where
    F: FnMut(u32, u32) + 'static,
  {
    self.on_size_changed = Some(Box::new(callback));
    self
  }

  pub fn run(self) {
    let event_loop = EventLoop::new().unwrap();

    let tree = self.tree;
    let secondaries = (0..tree.secondary_window_count())
      .filter_map(|index| {
        tree
          .secondary_window(index)
          .map(|secondary| ManagedSecondaryWindow::new(index, secondary))
      })
      .collect();

    let mut handler = WinitHandler {
      app: self.app,
      main: ManagedWindow::new(
        tree,
        self.attrs,
        self.on_tick,
        self.on_position_changed,
        self.on_size_changed,
        true,
      ),
      secondaries,
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
  on_position_changed: Option<PositionChangedFn>,
  on_size_changed: Option<SizeChangedFn>,
  redraw_pending: bool,
  close_exits: bool,
}

impl ManagedWindow {
  fn new(
    tree: Tree,
    attrs: WindowAttributes,
    on_tick: Option<TickFn>,
    on_position_changed: Option<PositionChangedFn>,
    on_size_changed: Option<SizeChangedFn>,
    close_exits: bool,
  ) -> Self {
    Self {
      tree,
      window: None,
      cursor_pos: (0.0, 0.0),
      cursor: CursorIcon::Default,
      modifiers: ModifiersState::empty(),
      attrs: Some(attrs),
      on_tick,
      on_position_changed,
      on_size_changed,
      redraw_pending: false,
      close_exits,
    }
  }

  fn window_id(&self) -> Option<WindowId> {
    self.window.as_ref().map(Window::id)
  }

  fn has_tick(&self) -> bool {
    true
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
    self.notify_size_changed(size.width, size.height);
    if let Ok(position) = window.outer_position() {
      self.tree.set_window_position(position.x, position.y);
      self.notify_position_changed(position.x, position.y);
    }
    self.window = Some(window);
    self.sync_window_state();
    self.request_redraw();
  }

  fn sync_window_state(&mut self) {
    if let Some(window) = &self.window {
      if let Some(minimized) = window.is_minimized() {
        self.tree.window().set_minimized(minimized);
      }
      self.tree.window().set_maximized(window.is_maximized());
      self.tree.window().set_full_screen(window.fullscreen().is_some());
    }
  }

  fn apply_window_commands(&mut self, event_loop: &ActiveEventLoop) -> bool {
    let commands = self.tree.window().take_commands();
    if commands.is_empty() {
      return false;
    }

    let mut closed = false;
    for command in commands {
      match command {
        WindowCommand::Close => {
          if self.close_exits {
            event_loop.exit();
          } else {
            self.window = None;
            self.redraw_pending = false;
            closed = true;
          }
        }
        WindowCommand::SetMinimized(minimized) => {
          if let Some(window) = &self.window {
            window.set_minimized(minimized);
          }
          self.tree.window().set_minimized(minimized);
        }
        WindowCommand::SetMaximized(maximized) => {
          if let Some(window) = &self.window {
            window.set_maximized(maximized);
          }
          self.tree.window().set_maximized(maximized);
        }
        WindowCommand::SetFullScreen(full_screen) => {
          if let Some(window) = &self.window {
            window.set_fullscreen(full_screen.then(|| Fullscreen::Borderless(None)));
          }
          self.tree.window().set_full_screen(full_screen);
        }
        WindowCommand::SetDecorated(decorated) => {
          if let Some(window) = &self.window {
            window.set_decorations(decorated);
          }
          self.tree.window().set_decorated(decorated);
        }
        WindowCommand::Move { x, y } => {
          if let Some(window) = &self.window {
            window.set_outer_position(PhysicalPosition::new(x, y));
          }
          self.tree.set_window_position(x, y);
          self.notify_position_changed(x, y);
        }
        WindowCommand::Resize { width, height } => {
          if let Some(window) = &self.window {
            let _ = window.request_inner_size(PhysicalSize::new(width, height));
          }
          self.notify_size_changed(width, height);
        }
        WindowCommand::StartDrag => {
          if let Some(window) = &self.window {
            let _ = window.drag_window();
          }
        }
        WindowCommand::StartResize(direction) => {
          if let Some(window) = &self.window {
            let _ = window.drag_resize_window(to_winit_resize_direction(direction));
          }
        }
        WindowCommand::StopDrag => {}
      }
    }
    self.sync_window_state();
    closed
  }

  fn check_redraw(&mut self) {
    if self.tree.needs_redraw() {
      self.request_redraw();
    }
  }

  fn request_redraw(&mut self) {
    if let Some(w) = &self.window {
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
    self.tree.tick_timers();
    self.tree.tick_futures();
    self.tree.request_redraw();
    self.tree.tick_perf_overlay();
    if self.tree.perf_overlay_enabled() || self.tree.has_active_timeline() {
      self.tree.request_redraw();
    }
    self.check_redraw();
  }

  fn notify_position_changed(&mut self, x: i32, y: i32) {
    if let Some(callback) = &mut self.on_position_changed {
      callback(x, y);
    }
  }

  fn notify_size_changed(&mut self, width: u32, height: u32) {
    if let Some(callback) = &mut self.on_size_changed {
      let scale = self.tree.window().info().scale_factor.max(f32::EPSILON);
      callback(
        ((width as f32) / scale).round().max(1.0) as u32,
        ((height as f32) / scale).round().max(1.0) as u32,
      );
    }
  }

  fn handle_event(&mut self, app: &mut App, event_loop: &ActiveEventLoop, event: WindowEvent) -> bool {
    self.tree.set_app_ref(app);
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
        self.notify_size_changed(size.width, size.height);
        self.sync_window_state();
        self.request_redraw();
      }
      WindowEvent::Focused(focused) => {
        self.tree.window().set_focused(focused);
        self.request_redraw();
      }
      WindowEvent::Moved(position) => {
        self.tree.set_window_position(position.x, position.y);
        self.notify_position_changed(position.x, position.y);
      }
      WindowEvent::CursorMoved { position, .. } => {
        self.cursor_pos = (position.x, position.y);
        self.tree.mouse_move_with_modifiers(
          position.x as f32,
          position.y as f32,
          self.modifiers.shift_key(),
          self.modifiers.control_key(),
          self.modifiers.alt_key(),
        );
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
          ElementState::Pressed => self.tree.mouse_down_with_modifiers(
            x,
            y,
            btn,
            self.modifiers.shift_key(),
            self.modifiers.control_key(),
            self.modifiers.alt_key(),
          ),
          ElementState::Released => self.tree.mouse_up_with_modifiers(
            x,
            y,
            btn,
            self.modifiers.shift_key(),
            self.modifiers.control_key(),
            self.modifiers.alt_key(),
          ),
        }
        self.apply_window_commands(event_loop);
        self.apply_cursor();
        self.check_redraw();
        return false;
      }
      WindowEvent::ModifiersChanged(modifiers) => {
        self.modifiers = modifiers.state();
      }
      WindowEvent::KeyboardInput { event, .. } => {
        match event.state {
          ElementState::Pressed => self.tree.key_down(
            key_to_string(&event),
            physical_key_to_string(&event.physical_key),
            self.modifiers.shift_key(),
            self.modifiers.control_key(),
            self.modifiers.alt_key(),
          ),
          ElementState::Released => self.tree.key_up(
            key_to_string(&event),
            physical_key_to_string(&event.physical_key),
            self.modifiers.shift_key(),
            self.modifiers.control_key(),
            self.modifiers.alt_key(),
          ),
        }
        self.apply_cursor();
        self.check_redraw();
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
        let presented = self.present_now(app);
        self.apply_window_commands(event_loop);
        return presented;
      }
      _ => {}
    }
    self.apply_window_commands(event_loop);
    self.check_redraw();
    false
  }
}

struct ManagedSecondaryWindow {
  index: usize,
  window: Option<Window>,
  cursor_pos: (f64, f64),
  cursor: CursorIcon,
  modifiers: ModifiersState,
  attrs: Option<WindowAttributes>,
  redraw_pending: bool,
}

impl ManagedSecondaryWindow {
  fn new(index: usize, secondary: &SecondaryWindow) -> Self {
    Self {
      index,
      window: None,
      cursor_pos: (0.0, 0.0),
      cursor: CursorIcon::Default,
      modifiers: ModifiersState::empty(),
      attrs: Some(
        WindowAttributes::default()
          .with_title(secondary.title())
          .with_inner_size(winit::dpi::LogicalSize::new(secondary.width(), secondary.height())),
      ),
      redraw_pending: false,
    }
  }

  fn index(&self) -> usize {
    self.index
  }

  fn window_id(&self) -> Option<WindowId> {
    self.window.as_ref().map(Window::id)
  }

  fn has_tick(&self, tree: Option<&Tree>) -> bool {
    tree.is_some()
  }

  fn create_window(&mut self, event_loop: &ActiveEventLoop, tree: &mut Tree) -> Option<SecondaryWindowMetadata> {
    if self.window.is_some() {
      return None;
    }

    let attrs = self.attrs.take().unwrap_or_default();
    let window = event_loop.create_window(attrs).unwrap();
    let size = window.inner_size();
    let metadata = secondary_window_metadata(&window);
    tree.set_scale_factor(window.scale_factor() as f32);
    tree.resize(size.width, size.height);
    if let Ok(position) = window.outer_position() {
      tree.set_window_position(position.x, position.y);
    }
    self.window = Some(window);
    self.sync_window_state(tree);
    self.request_redraw();
    Some(metadata)
  }

  fn sync_window_state(&mut self, tree: &mut Tree) {
    if let Some(window) = &self.window {
      if let Some(minimized) = window.is_minimized() {
        tree.window().set_minimized(minimized);
      }
      tree.window().set_maximized(window.is_maximized());
      tree.window().set_full_screen(window.fullscreen().is_some());
    }
  }

  fn apply_window_commands(&mut self, tree: &mut Tree) -> bool {
    let commands = tree.window().take_commands();
    if commands.is_empty() {
      return false;
    }

    let mut closed = false;
    for command in commands {
      match command {
        WindowCommand::Close => {
          self.window = None;
          self.redraw_pending = false;
          closed = true;
        }
        WindowCommand::SetMinimized(minimized) => {
          if let Some(window) = &self.window {
            window.set_minimized(minimized);
          }
          tree.window().set_minimized(minimized);
        }
        WindowCommand::SetMaximized(maximized) => {
          if let Some(window) = &self.window {
            window.set_maximized(maximized);
          }
          tree.window().set_maximized(maximized);
        }
        WindowCommand::SetFullScreen(full_screen) => {
          if let Some(window) = &self.window {
            window.set_fullscreen(full_screen.then(|| Fullscreen::Borderless(None)));
          }
          tree.window().set_full_screen(full_screen);
        }
        WindowCommand::SetDecorated(decorated) => {
          if let Some(window) = &self.window {
            window.set_decorations(decorated);
          }
          tree.window().set_decorated(decorated);
        }
        WindowCommand::Move { x, y } => {
          if let Some(window) = &self.window {
            window.set_outer_position(PhysicalPosition::new(x, y));
          }
          tree.set_window_position(x, y);
        }
        WindowCommand::Resize { width, height } => {
          if let Some(window) = &self.window {
            let _ = window.request_inner_size(PhysicalSize::new(width, height));
          }
        }
        WindowCommand::StartDrag => {
          if let Some(window) = &self.window {
            let _ = window.drag_window();
          }
        }
        WindowCommand::StartResize(direction) => {
          if let Some(window) = &self.window {
            let _ = window.drag_resize_window(to_winit_resize_direction(direction));
          }
        }
        WindowCommand::StopDrag => {}
      }
    }
    self.sync_window_state(tree);
    closed
  }

  fn check_redraw(&mut self, tree: &Tree) {
    if tree.needs_redraw() {
      self.request_redraw();
    }
  }

  fn request_redraw(&mut self) {
    if let Some(w) = &self.window {
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
    tree.tick_timers();
    tree.tick_futures();
    tree.request_redraw();
    tree.tick_perf_overlay();
    if tree.perf_overlay_enabled() || tree.has_active_timeline() {
      tree.request_redraw();
    }
    self.check_redraw(tree);
  }

  fn handle_event(
    &mut self,
    app: &mut App,
    _event_loop: &ActiveEventLoop,
    event: WindowEvent,
    tree: &mut Tree,
  ) -> bool {
    tree.set_app_ref(app);
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
        self.sync_window_state(tree);
        self.request_redraw();
      }
      WindowEvent::Focused(focused) => {
        tree.window().set_focused(focused);
        self.request_redraw();
      }
      WindowEvent::Moved(position) => {
        tree.set_window_position(position.x, position.y);
      }
      WindowEvent::CursorMoved { position, .. } => {
        self.cursor_pos = (position.x, position.y);
        tree.mouse_move_with_modifiers(
          position.x as f32,
          position.y as f32,
          self.modifiers.shift_key(),
          self.modifiers.control_key(),
          self.modifiers.alt_key(),
        );
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
          ElementState::Pressed => tree.mouse_down_with_modifiers(
            x,
            y,
            btn,
            self.modifiers.shift_key(),
            self.modifiers.control_key(),
            self.modifiers.alt_key(),
          ),
          ElementState::Released => tree.mouse_up_with_modifiers(
            x,
            y,
            btn,
            self.modifiers.shift_key(),
            self.modifiers.control_key(),
            self.modifiers.alt_key(),
          ),
        }
        self.apply_window_commands(tree);
        self.apply_cursor(tree);
        self.check_redraw(tree);
        return false;
      }
      WindowEvent::ModifiersChanged(modifiers) => {
        self.modifiers = modifiers.state();
      }
      WindowEvent::KeyboardInput { event, .. } => {
        match event.state {
          ElementState::Pressed => tree.key_down(
            key_to_string(&event),
            physical_key_to_string(&event.physical_key),
            self.modifiers.shift_key(),
            self.modifiers.control_key(),
            self.modifiers.alt_key(),
          ),
          ElementState::Released => tree.key_up(
            key_to_string(&event),
            physical_key_to_string(&event.physical_key),
            self.modifiers.shift_key(),
            self.modifiers.control_key(),
            self.modifiers.alt_key(),
          ),
        }
        self.apply_cursor(tree);
        self.check_redraw(tree);
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
        let presented = self.present_now(app, tree);
        self.apply_window_commands(tree);
        return presented;
      }
      _ => {}
    }
    self.apply_window_commands(tree);
    self.check_redraw(tree);
    false
  }
}

fn secondary_window_metadata(window: &Window) -> SecondaryWindowMetadata {
  let raw_window = window.window_handle().ok().map(|handle| handle.as_raw());
  let raw_display = window.display_handle().ok().map(|handle| handle.as_raw());
  let hwnd = raw_window.and_then(|handle| match handle {
    RawWindowHandle::Win32(handle) => Some(handle.hwnd.get()),
    _ => None,
  });

  SecondaryWindowMetadata {
    window_id: Some(format!("{:?}", window.id())),
    raw_window_handle: raw_window.map(|handle| format!("{handle:?}")),
    raw_display_handle: raw_display.map(|handle| format!("{handle:?}")),
    hwnd,
  }
}

struct WinitHandler {
  app: App,
  main: ManagedWindow,
  secondaries: Vec<ManagedSecondaryWindow>,
}

impl WinitHandler {
  fn check_secondary_redraw(&mut self) {
    for secondary_window in &mut self.secondaries {
      let index = secondary_window.index();
      if let Some(secondary) = self.main.tree.secondary_window(index) {
        secondary_window.check_redraw(secondary.tree());
      }
    }
  }

  fn present_dirty_main(&mut self) {
    if self.main.tree.needs_redraw() {
      self.main.present_now(&mut self.app);
    }
  }

  fn present_dirty_secondaries(&mut self) {
    for position in 0..self.secondaries.len() {
      let index = self.secondaries[position].index();
      let should_present = self
        .main
        .tree
        .secondary_window(index)
        .is_some_and(|secondary| secondary.tree().needs_redraw());
      if !should_present {
        continue;
      }

      let secondary_window = &mut self.secondaries[position];
      if let Some(secondary) = self.main.tree.secondary_window_mut(index) {
        secondary_window.present_now(&mut self.app, secondary.tree_mut());
      }
    }
  }

  fn present_dirty_windows(&mut self) {
    self.present_dirty_main();
    self.present_dirty_secondaries();
  }

  fn apply_secondary_window_requests(&mut self) {
    if self.main.tree.apply_secondary_window_requests() {
      self.main.request_redraw();
    }
    self.check_secondary_redraw();
    self.present_dirty_windows();
  }

  fn handle_secondary_pick_event(&mut self, event: &WindowEvent) -> bool {
    if !self.main.tree.secondary_pick_mode() {
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
        self.main.tree.pick_secondary_node_at(x, y);
        self.main.request_redraw();
        self.check_secondary_redraw();
        self.present_dirty_windows();
        true
      }
      WindowEvent::KeyboardInput { event, .. }
        if matches!(event.state, ElementState::Pressed)
          && matches!(event.logical_key, Key::Named(NamedKey::Escape)) =>
      {
        self.main.tree.cancel_secondary_pick();
        self.check_secondary_redraw();
        self.present_dirty_windows();
        true
      }
      _ => false,
    }
  }
}

impl ApplicationHandler for WinitHandler {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    self.main.create_window(event_loop);
    for secondary_window in &mut self.secondaries {
      let index = secondary_window.index();
      let metadata = self
        .main
        .tree
        .secondary_window_mut(index)
        .and_then(|secondary| secondary_window.create_window(event_loop, secondary.tree_mut()));
      if let Some(metadata) = metadata {
        self.main.tree.set_secondary_window_metadata(index, metadata);
      }
    }
  }

  fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
    if self.main.window_id() == Some(id) {
      if self.handle_secondary_pick_event(&event) {
        return;
      }
      let presented = self.main.handle_event(&mut self.app, event_loop, event);
      if presented {
        self.check_secondary_redraw();
        self.present_dirty_secondaries();
      }
      return;
    }

    if let Some(position) = self
      .secondaries
      .iter()
      .position(|secondary| secondary.window_id() == Some(id))
    {
      let index = self.secondaries[position].index();
      let mut closed = false;
      if let Some(secondary) = self.main.tree.secondary_window_mut(index) {
        self.secondaries[position].handle_event(&mut self.app, event_loop, event, secondary.tree_mut());
        closed = self.secondaries[position].window.is_none();
      }
      if closed {
        if self.main.tree.close_secondary_window(index) {
          self.main.request_redraw();
        }
        self.secondaries.remove(position);
      } else {
        self.apply_secondary_window_requests();
      }
    }
  }

  fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
    self.main.tick();
    self.main.apply_window_commands(event_loop);
    for secondary_window in &mut self.secondaries {
      if let Some(secondary) = self.main.tree.secondary_window_mut(secondary_window.index()) {
        secondary_window.tick(secondary.tree_mut());
        secondary_window.apply_window_commands(secondary.tree_mut());
      }
    }

    let secondary_has_tick = self.secondaries.iter().any(|secondary_window| {
      self
        .main
        .tree
        .secondary_window(secondary_window.index())
        .is_some_and(|secondary| secondary_window.has_tick(Some(secondary.tree())))
    });

    if self.main.has_tick() || secondary_has_tick {
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
    NamedKey::Insert => "Insert",
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

fn to_winit_resize_direction(direction: WindowResizeDirection) -> WinitResizeDirection {
  match direction {
    WindowResizeDirection::East => WinitResizeDirection::East,
    WindowResizeDirection::North => WinitResizeDirection::North,
    WindowResizeDirection::NorthEast => WinitResizeDirection::NorthEast,
    WindowResizeDirection::NorthWest => WinitResizeDirection::NorthWest,
    WindowResizeDirection::South => WinitResizeDirection::South,
    WindowResizeDirection::SouthEast => WinitResizeDirection::SouthEast,
    WindowResizeDirection::SouthWest => WinitResizeDirection::SouthWest,
    WindowResizeDirection::West => WinitResizeDirection::West,
  }
}
