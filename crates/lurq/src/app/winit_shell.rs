use std::time::{Duration, Instant};

use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawWindowHandle};
#[cfg(windows)]
use winit::platform::windows::{
  Color as WinitWindowsColor, CornerPreference as WinitCornerPreference, WindowAttributesExtWindows, WindowExtWindows,
};
use winit::{
  application::ApplicationHandler,
  dpi::{PhysicalPosition, PhysicalSize, Position},
  event::{ElementState, MouseScrollDelta, TouchPhase, WindowEvent},
  event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
  keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey},
  window::{
    CursorIcon as WinitCursorIcon, Fullscreen, Icon as WinitIcon, ResizeDirection as WinitResizeDirection, Window,
    WindowAttributes, WindowId,
  },
};

use crate::{
  app::{
    App, Tree,
    events::{MouseButton, ScrollPhase},
    runtime::{PassReport, SecondaryWindow, SecondaryWindowMetadata},
    window::{WindowCommand, WindowCornerRadius, WindowIcon, WindowResizeDirection},
  },
  node::{CursorIcon, color::Color},
};

type TickFn = Box<dyn FnMut(&mut Tree, Duration)>;
type PaintFn = Box<dyn FnMut(&Tree, Duration, PassReport)>;
type PositionChangedFn = Box<dyn FnMut(i32, i32)>;
type SizeChangedFn = Box<dyn FnMut(u32, u32)>;

pub struct WinitWindow {
  app: App,
  tree: Tree,
  attrs: WindowAttributes,
  corner_radius: Option<WindowCornerRadius>,
  on_tick: Option<TickFn>,
  on_paint: Option<PaintFn>,
  on_position_changed: Option<PositionChangedFn>,
  on_size_changed: Option<SizeChangedFn>,
}

impl WinitWindow {
  pub fn new(app: App, tree: Tree) -> Self {
    Self {
      app,
      tree,
      attrs: WindowAttributes::default(),
      corner_radius: None,
      on_tick: None,
      on_paint: None,
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

  pub fn with_title_bar_color(mut self, color: impl Into<Option<Color>>) -> Self {
    self.attrs = with_title_bar_color(self.attrs, color.into());
    self
  }

  pub fn with_icon(mut self, icon: impl Into<Option<WindowIcon>>) -> Self {
    self.attrs = self.attrs.with_window_icon(icon.into().and_then(to_winit_icon));
    self
  }

  pub fn with_corner_radius(mut self, radius: WindowCornerRadius) -> Self {
    self.attrs = with_corner_radius(self.attrs, radius);
    self.corner_radius = Some(radius);
    self.tree.window().set_corner_radius(radius);
    self
  }

  pub fn with_rounded_corners(self, rounded: bool) -> Self {
    self.with_corner_radius(if rounded {
      WindowCornerRadius::Rounded
    } else {
      WindowCornerRadius::None
    })
  }

  pub fn with_transparent(mut self, transparent: bool) -> Self {
    self.attrs = self.attrs.with_transparent(transparent);
    self
  }

  /// Runs during event-loop ticking before paint and may mutate the tree.
  pub fn on_tick<F>(mut self, tick: F) -> Self
  where
    F: FnMut(&mut Tree, Duration) + 'static,
  {
    self.on_tick = Some(Box::new(tick));
    self
  }

  /// Runs after a frame is presented. Use this for paint profiling.
  pub fn on_paint<F>(mut self, paint: F) -> Self
  where
    F: FnMut(&Tree, Duration, PassReport) + 'static,
  {
    self.on_paint = Some(Box::new(paint));
    self
  }

  #[deprecated(note = "use on_paint for callbacks after a frame is presented")]
  pub fn on_frame<F>(self, frame: F) -> Self
  where
    F: FnMut(&Tree, Duration, PassReport) + 'static,
  {
    self.on_paint(frame)
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
        self.corner_radius,
        self.on_tick,
        self.on_paint,
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
  corner_radius: Option<WindowCornerRadius>,
  on_tick: Option<TickFn>,
  on_paint: Option<PaintFn>,
  on_position_changed: Option<PositionChangedFn>,
  on_size_changed: Option<SizeChangedFn>,
  redraw_pending: bool,
  close_exits: bool,
  last_tick: Instant,
  last_paint: Instant,
}

impl ManagedWindow {
  fn new(
    tree: Tree,
    attrs: WindowAttributes,
    corner_radius: Option<WindowCornerRadius>,
    on_tick: Option<TickFn>,
    on_paint: Option<PaintFn>,
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
      corner_radius,
      on_tick,
      on_paint,
      on_position_changed,
      on_size_changed,
      redraw_pending: false,
      close_exits,
      last_tick: Instant::now(),
      last_paint: Instant::now(),
    }
  }

  fn window_id(&self) -> Option<WindowId> {
    self.window.as_ref().map(Window::id)
  }

  fn has_tick(&self) -> bool {
    self.on_tick.is_some() || self.tree.has_active_tick_sources()
  }

  fn create_window(&mut self, event_loop: &ActiveEventLoop, app: &mut App) {
    if self.window.is_some() {
      return;
    }

    let attrs = self.attrs.take().unwrap_or_default();
    let show_after_first_present = attrs.visible;
    let attrs = if show_after_first_present {
      attrs.with_visible(false)
    } else {
      attrs
    };
    let window = event_loop.create_window(attrs).unwrap();
    if let Some(radius) = self.corner_radius {
      set_corner_radius(&window, radius);
      self.tree.window().set_corner_radius(radius);
    }
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
    let presented = show_after_first_present && self.present_now(app);
    if show_after_first_present {
      if let Some(window) = &self.window {
        window.set_visible(true);
      }
      self.tree.request_redraw();
      self.check_redraw();
    }
    if !presented {
      self.request_redraw();
    }
  }

  fn sync_window_state(&mut self) -> bool {
    let Some(window) = &self.window else {
      return false;
    };

    let size = window.inner_size();
    let minimized = window.is_minimized();
    let maximized = window.is_maximized();
    let full_screen = window.fullscreen().is_some();
    let current = self.tree.window().info();
    let size_changed =
      current.resolved_width.round() as u32 != size.width || current.resolved_height.round() as u32 != size.height;

    if size_changed {
      self.tree.resize(size.width, size.height);
      self.notify_size_changed(size.width, size.height);
    }
    if let Some(minimized) = minimized {
      self.tree.window().set_minimized(minimized);
    }
    self.tree.window().set_maximized(maximized);
    self.tree.window().set_full_screen(full_screen);

    size_changed
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
        WindowCommand::SetTitleBarColor(color) => {
          if let Some(window) = &self.window {
            set_title_bar_color(window, color);
          }
        }
        WindowCommand::SetIcon(icon) => {
          if let Some(window) = &self.window {
            window.set_window_icon(icon.and_then(to_winit_icon));
          }
        }
        WindowCommand::SetCornerRadius(radius) => {
          if let Some(window) = &self.window {
            set_corner_radius(window, radius);
          }
          self.tree.window().set_corner_radius(radius);
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
            if !begin_native_window_drag(window) {
              let _ = window.drag_window();
            }
          }
        }
        WindowCommand::StartResize(direction) => {
          if let Some(window) = &self.window {
            if !begin_native_window_resize(window, direction) {
              let _ = window.drag_resize_window(to_winit_resize_direction(direction));
            }
          }
        }
        WindowCommand::StopDrag => {}
        WindowCommand::OpenDevtools => {
          self.tree.open_devtools();
        }
        WindowCommand::CloseDevtools => {
          self.tree.close_devtools();
        }
        WindowCommand::ToggleDevtools => {
          self.tree.toggle_devtools();
        }
      }
    }
    if self.sync_window_state() {
      self.request_redraw();
    }
    closed
  }

  fn check_redraw(&mut self) {
    if self.tree.needs_redraw() {
      self.request_redraw();
    }
  }

  fn request_redraw(&mut self) {
    if self.redraw_pending {
      return;
    }
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
      let report = self.tree.pass(app, w);
      let presented = report.rendered;
      if presented {
        let now = Instant::now();
        let delta = now.duration_since(self.last_paint);
        self.last_paint = now;
        if let Some(paint) = &mut self.on_paint {
          paint(&self.tree, delta, report);
        }
      }
      self.check_redraw();
      return presented;
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
    let now = Instant::now();
    let delta = now.duration_since(self.last_tick);
    self.last_tick = now;

    if let Some(tick) = &mut self.on_tick {
      tick(&mut self.tree, delta);
    }
    self.tree.tick_timers();
    self.tree.tick_futures();
    self.tree.tick_perf_overlay();
    self.tree.tick_scheduled_redraw(now);
    if self.tree.has_active_timeline() || self.tree.perf_overlay_enabled() {
      self.request_redraw_for_next_refresh(now);
    }
    self.check_redraw();
  }

  fn request_redraw_for_next_refresh(&mut self, now: Instant) {
    let next_frame_at = self.last_paint + self.refresh_interval();
    if now >= next_frame_at {
      self.tree.request_redraw();
    } else {
      self.tree.request_redraw_at(next_frame_at);
    }
  }

  fn refresh_interval(&self) -> Duration {
    self
      .window
      .as_ref()
      .and_then(Window::current_monitor)
      .and_then(|monitor| monitor.refresh_rate_millihertz())
      .filter(|rate| *rate > 0)
      .map(|rate| Duration::from_nanos((1_000_000_000_000u64 / rate as u64).max(1)))
      .unwrap_or_else(|| Duration::from_micros(16_667))
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
        if is_open_devtools_shortcut(&event, self.modifiers) {
          self.tree.open_devtools();
          self.apply_cursor();
          self.check_redraw();
          return false;
        }
        match event.state {
          ElementState::Pressed => self.tree.key_down_with_meta(
            key_to_string(&event),
            physical_key_to_string(&event.physical_key),
            self.modifiers.shift_key(),
            self.modifiers.control_key(),
            self.modifiers.alt_key(),
            self.modifiers.super_key(),
          ),
          ElementState::Released => self.tree.key_up_with_meta(
            key_to_string(&event),
            physical_key_to_string(&event.physical_key),
            self.modifiers.shift_key(),
            self.modifiers.control_key(),
            self.modifiers.alt_key(),
            self.modifiers.super_key(),
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
  close_requested: bool,
  last_paint: Instant,
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
      close_requested: false,
      last_paint: Instant::now(),
    }
  }

  fn index(&self) -> usize {
    self.index
  }

  fn window_id(&self) -> Option<WindowId> {
    self.window.as_ref().map(Window::id)
  }

  fn request_close(&mut self) {
    self.redraw_pending = false;
    self.close_requested = true;
  }

  fn close_requested(&self) -> bool {
    self.close_requested
  }

  fn has_tick(&self, tree: Option<&Tree>) -> bool {
    tree.is_some_and(Tree::has_active_tick_sources)
  }

  fn create_window(
    &mut self,
    event_loop: &ActiveEventLoop,
    app: &mut App,
    tree: &mut Tree,
  ) -> Option<SecondaryWindowMetadata> {
    if self.window.is_some() {
      return None;
    }

    let attrs = self.attrs.take().unwrap_or_default();
    let show_after_first_present = attrs.visible;
    let attrs = if show_after_first_present {
      attrs.with_visible(false)
    } else {
      attrs
    };
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
    let presented = show_after_first_present && self.present_now(app, tree);
    if show_after_first_present {
      if let Some(window) = &self.window {
        window.set_visible(true);
      }
    }
    if !presented {
      self.request_redraw();
    }
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
          self.request_close();
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
        WindowCommand::SetTitleBarColor(color) => {
          if let Some(window) = &self.window {
            set_title_bar_color(window, color);
          }
        }
        WindowCommand::SetIcon(icon) => {
          if let Some(window) = &self.window {
            window.set_window_icon(icon.and_then(to_winit_icon));
          }
        }
        WindowCommand::SetCornerRadius(radius) => {
          if let Some(window) = &self.window {
            set_corner_radius(window, radius);
          }
          tree.window().set_corner_radius(radius);
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
            if !begin_native_window_drag(window) {
              let _ = window.drag_window();
            }
          }
        }
        WindowCommand::StartResize(direction) => {
          if let Some(window) = &self.window {
            if !begin_native_window_resize(window, direction) {
              let _ = window.drag_resize_window(to_winit_resize_direction(direction));
            }
          }
        }
        WindowCommand::StopDrag => {}
        WindowCommand::OpenDevtools => {
          tree.open_devtools();
        }
        WindowCommand::CloseDevtools => {
          tree.close_devtools();
        }
        WindowCommand::ToggleDevtools => {
          tree.toggle_devtools();
        }
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
    if self.redraw_pending {
      return;
    }
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
      let presented = tree.pass(app, w).rendered;
      if presented {
        self.last_paint = Instant::now();
      }
      self.check_redraw(tree);
      return presented;
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
    let now = Instant::now();
    tree.tick_timers();
    tree.tick_futures();
    tree.tick_perf_overlay();
    tree.tick_scheduled_redraw(now);
    if tree.has_active_timeline() || tree.perf_overlay_enabled() {
      self.request_redraw_for_next_refresh(tree, now);
    }
    self.check_redraw(tree);
  }

  fn request_redraw_for_next_refresh(&mut self, tree: &mut Tree, now: Instant) {
    let next_frame_at = self.last_paint + self.refresh_interval();
    if now >= next_frame_at {
      tree.request_redraw();
    } else {
      tree.request_redraw_at(next_frame_at);
    }
  }

  fn refresh_interval(&self) -> Duration {
    self
      .window
      .as_ref()
      .and_then(Window::current_monitor)
      .and_then(|monitor| monitor.refresh_rate_millihertz())
      .filter(|rate| *rate > 0)
      .map(|rate| Duration::from_nanos((1_000_000_000_000u64 / rate as u64).max(1)))
      .unwrap_or_else(|| Duration::from_micros(16_667))
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
        self.request_close();
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
          ElementState::Pressed => tree.key_down_with_meta(
            key_to_string(&event),
            physical_key_to_string(&event.physical_key),
            self.modifiers.shift_key(),
            self.modifiers.control_key(),
            self.modifiers.alt_key(),
            self.modifiers.super_key(),
          ),
          ElementState::Released => tree.key_up_with_meta(
            key_to_string(&event),
            physical_key_to_string(&event.physical_key),
            self.modifiers.shift_key(),
            self.modifiers.control_key(),
            self.modifiers.alt_key(),
            self.modifiers.super_key(),
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
  fn close_secondary_at_position(&mut self, position: usize) {
    let index = self.secondaries[position].index();
    if self.main.tree.close_secondary_window(index) {
      self.main.request_redraw();
    }
    self.secondaries.remove(position);
  }

  fn sync_secondary_windows(&mut self, event_loop: &ActiveEventLoop) {
    self
      .secondaries
      .retain(|secondary| self.main.tree.secondary_window(secondary.index()).is_some());

    let count = self.main.tree.secondary_window_count();
    for index in 0..count {
      if self.secondaries.iter().any(|secondary| secondary.index() == index) {
        continue;
      }

      let Some(secondary) = self.main.tree.secondary_window(index) else {
        continue;
      };
      let mut managed = ManagedSecondaryWindow::new(index, secondary);
      self.main.tree.ensure_secondary_window_render_engine(index);
      let metadata = self
        .main
        .tree
        .secondary_window_mut(index)
        .and_then(|secondary| managed.create_window(event_loop, &mut self.app, secondary.tree_mut()));
      if let Some(metadata) = metadata {
        self.main.tree.set_secondary_window_metadata(index, metadata);
      }
      self.secondaries.push(managed);
    }
  }

  fn check_secondary_redraw(&mut self) {
    for secondary_window in &mut self.secondaries {
      let index = secondary_window.index();
      if let Some(secondary) = self.main.tree.secondary_window(index) {
        secondary_window.check_redraw(secondary.tree());
      }
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

  fn apply_secondary_window_requests(&mut self, event_loop: &ActiveEventLoop) {
    if self.main.tree.apply_secondary_window_requests() {
      self.main.request_redraw();
    }
    self.sync_secondary_windows(event_loop);
    self.check_secondary_redraw();
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
        true
      }
      WindowEvent::KeyboardInput { event, .. }
        if matches!(event.state, ElementState::Pressed)
          && matches!(event.logical_key, Key::Named(NamedKey::Escape)) =>
      {
        self.main.tree.cancel_secondary_pick();
        self.check_secondary_redraw();
        true
      }
      _ => false,
    }
  }
}

impl ApplicationHandler for WinitHandler {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    self.main.create_window(event_loop, &mut self.app);
    self.sync_secondary_windows(event_loop);
  }

  fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
    if self.main.window_id() == Some(id) {
      if self.handle_secondary_pick_event(&event) {
        return;
      }
      let presented = self.main.handle_event(&mut self.app, event_loop, event);
      self.apply_secondary_window_requests(event_loop);
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
      if is_open_devtools_window_event(&event, self.secondaries[position].modifiers) {
        self.main.tree.open_devtools();
        self.apply_secondary_window_requests(event_loop);
        return;
      }
      let index = self.secondaries[position].index();
      let mut closed = false;
      if let Some(secondary) = self.main.tree.secondary_window_mut(index) {
        self.secondaries[position].handle_event(&mut self.app, event_loop, event, secondary.tree_mut());
        closed = self.secondaries[position].close_requested();
      }
      if closed {
        self.close_secondary_at_position(position);
      } else {
        self.apply_secondary_window_requests(event_loop);
      }
    }
  }

  fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
    self.main.tick();
    self.main.apply_window_commands(event_loop);
    self.apply_secondary_window_requests(event_loop);
    let mut position = 0;
    while position < self.secondaries.len() {
      let secondary_index = self.secondaries[position].index();
      let mut closed = false;
      if let Some(secondary) = self.main.tree.secondary_window_mut(secondary_index) {
        let secondary_window = &mut self.secondaries[position];
        secondary_window.tick(secondary.tree_mut());
        closed = secondary_window.apply_window_commands(secondary.tree_mut());
      }

      if closed {
        self.close_secondary_at_position(position);
      } else {
        position += 1;
      }
    }
    self.sync_secondary_windows(event_loop);

    let secondary_has_tick = self.secondaries.iter().any(|secondary_window| {
      self
        .main
        .tree
        .secondary_window(secondary_window.index())
        .is_some_and(|secondary| secondary_window.has_tick(Some(secondary.tree())))
    });
    let redraw_pending = self.main.redraw_pending || self.secondaries.iter().any(|secondary| secondary.redraw_pending);
    let next_scheduled_redraw = self
      .secondaries
      .iter()
      .filter_map(|secondary_window| {
        self
          .main
          .tree
          .secondary_window(secondary_window.index())
          .and_then(|secondary| secondary.tree().next_scheduled_redraw())
      })
      .chain(self.main.tree.next_scheduled_redraw())
      .min();

    if let Some(next_redraw) = next_scheduled_redraw.filter(|_| !redraw_pending) {
      event_loop.set_control_flow(ControlFlow::WaitUntil(next_redraw));
    } else if (self.main.has_tick() || secondary_has_tick) && !redraw_pending {
      event_loop.set_control_flow(ControlFlow::Poll);
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

fn is_open_devtools_shortcut(event: &winit::event::KeyEvent, modifiers: ModifiersState) -> bool {
  event.state == ElementState::Pressed
    && modifiers.control_key()
    && modifiers.shift_key()
    && !modifiers.alt_key()
    && !modifiers.super_key()
    && matches!(event.physical_key, PhysicalKey::Code(KeyCode::F12))
}

fn is_open_devtools_window_event(event: &WindowEvent, modifiers: ModifiersState) -> bool {
  matches!(event, WindowEvent::KeyboardInput { event, .. } if is_open_devtools_shortcut(event, modifiers))
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

fn to_winit_icon(icon: WindowIcon) -> Option<WinitIcon> {
  let (rgba, width, height) = icon.into_rgba();
  WinitIcon::from_rgba(rgba, width, height).ok()
}

#[cfg(windows)]
fn with_title_bar_color(attrs: WindowAttributes, color: Option<Color>) -> WindowAttributes {
  attrs.with_title_background_color(color.map(to_winit_windows_color))
}

#[cfg(not(windows))]
fn with_title_bar_color(attrs: WindowAttributes, color: Option<Color>) -> WindowAttributes {
  let _ = color;
  attrs
}

#[cfg(windows)]
fn set_title_bar_color(window: &Window, color: Option<Color>) {
  window.set_title_background_color(color.map(to_winit_windows_color));
}

#[cfg(not(windows))]
fn set_title_bar_color(window: &Window, color: Option<Color>) {
  let _ = (window, color);
}

#[cfg(windows)]
fn to_winit_windows_color(color: Color) -> WinitWindowsColor {
  WinitWindowsColor::from_rgb(color.r(), color.g(), color.b())
}

#[cfg(windows)]
fn with_corner_radius(attrs: WindowAttributes, radius: WindowCornerRadius) -> WindowAttributes {
  attrs.with_corner_preference(to_winit_corner_preference(radius))
}

#[cfg(not(windows))]
fn with_corner_radius(attrs: WindowAttributes, radius: WindowCornerRadius) -> WindowAttributes {
  let _ = radius;
  attrs
}

#[cfg(windows)]
fn set_corner_radius(window: &Window, radius: WindowCornerRadius) {
  window.set_corner_preference(to_winit_corner_preference(radius));
}

#[cfg(target_os = "macos")]
fn set_corner_radius(window: &Window, radius: WindowCornerRadius) {
  let Ok(handle) = window.window_handle() else {
    return;
  };

  let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
    return;
  };

  let radius = to_macos_corner_radius(radius);
  unsafe {
    use objc2::{msg_send, runtime::AnyObject};

    let view = handle.ns_view.as_ptr().cast::<AnyObject>();
    let wants_layer: bool = msg_send![view, wantsLayer];
    if !wants_layer {
      if radius.is_none() {
        return;
      }
      let _: () = msg_send![view, setWantsLayer: true];
    }

    let layer: *mut AnyObject = msg_send![view, layer];
    if layer.is_null() {
      return;
    }

    let _: () = msg_send![layer, setCornerRadius: radius.unwrap_or(0.0)];
    let _: () = msg_send![layer, setMasksToBounds: radius.is_some()];
  }
}

#[cfg(all(not(windows), not(target_os = "macos")))]
fn set_corner_radius(window: &Window, radius: WindowCornerRadius) {
  let _ = (window, radius);
}

#[cfg(windows)]
fn to_winit_corner_preference(radius: WindowCornerRadius) -> WinitCornerPreference {
  match radius {
    WindowCornerRadius::Default => WinitCornerPreference::Default,
    WindowCornerRadius::None => WinitCornerPreference::DoNotRound,
    WindowCornerRadius::Rounded => WinitCornerPreference::Round,
    WindowCornerRadius::RoundedSmall => WinitCornerPreference::RoundSmall,
  }
}

#[cfg(target_os = "macos")]
fn to_macos_corner_radius(radius: WindowCornerRadius) -> Option<f64> {
  match radius {
    WindowCornerRadius::Default => None,
    WindowCornerRadius::None => Some(0.0),
    WindowCornerRadius::Rounded => Some(10.0),
    WindowCornerRadius::RoundedSmall => Some(4.0),
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

#[cfg(windows)]
fn begin_native_window_drag(window: &Window) -> bool {
  use windows::Win32::UI::WindowsAndMessaging::HTCAPTION;

  send_native_non_client_mouse_down(window, HTCAPTION)
}

#[cfg(not(windows))]
fn begin_native_window_drag(window: &Window) -> bool {
  let _ = window;
  false
}

#[cfg(windows)]
fn begin_native_window_resize(window: &Window, direction: WindowResizeDirection) -> bool {
  use windows::Win32::UI::WindowsAndMessaging::{
    HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTLEFT, HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT,
  };

  let hit_test = match direction {
    WindowResizeDirection::North => HTTOP,
    WindowResizeDirection::South => HTBOTTOM,
    WindowResizeDirection::West => HTLEFT,
    WindowResizeDirection::East => HTRIGHT,
    WindowResizeDirection::NorthWest => HTTOPLEFT,
    WindowResizeDirection::NorthEast => HTTOPRIGHT,
    WindowResizeDirection::SouthWest => HTBOTTOMLEFT,
    WindowResizeDirection::SouthEast => HTBOTTOMRIGHT,
  };
  send_native_non_client_mouse_down(window, hit_test)
}

#[cfg(not(windows))]
fn begin_native_window_resize(window: &Window, direction: WindowResizeDirection) -> bool {
  let _ = (window, direction);
  false
}

#[cfg(windows)]
fn send_native_non_client_mouse_down(window: &Window, hit_test: u32) -> bool {
  use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    UI::{
      Input::KeyboardAndMouse::ReleaseCapture,
      WindowsAndMessaging::{SendMessageW, WM_NCLBUTTONDOWN},
    },
  };

  let Ok(handle) = window.window_handle() else {
    return false;
  };
  let RawWindowHandle::Win32(handle) = handle.as_raw() else {
    return false;
  };
  let hwnd = HWND(handle.hwnd.get() as *mut _);
  if hwnd.is_invalid() {
    return false;
  }

  unsafe {
    let _ = ReleaseCapture();
    SendMessageW(hwnd, WM_NCLBUTTONDOWN, WPARAM(hit_test as usize), LPARAM(0));
  }
  true
}
