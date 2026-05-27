use winit::{
  application::ApplicationHandler,
  event::{ElementState, MouseScrollDelta, TouchPhase, WindowEvent},
  event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
  keyboard::{Key, ModifiersState, NamedKey, PhysicalKey},
  window::{Window, WindowAttributes, WindowId},
};

use crate::app::{
  Runtime,
  events::{MouseButton, ScrollPhase},
};

type TickFn = Box<dyn FnMut(&mut Runtime)>;

pub struct WinitWindow {
  runtime: Runtime,
  attrs: WindowAttributes,
  on_tick: Option<TickFn>,
}

impl WinitWindow {
  pub fn new(runtime: Runtime) -> Self {
    Self {
      runtime,
      attrs: WindowAttributes::default(),
      on_tick: None,
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
    F: FnMut(&mut Runtime) + 'static,
  {
    self.on_tick = Some(Box::new(tick));
    self
  }

  pub fn run(self) {
    let event_loop = EventLoop::new().unwrap();
    if self.on_tick.is_some() {
      event_loop.set_control_flow(ControlFlow::Poll);
    }
    let mut handler = WinitHandler {
      runtime: self.runtime,
      window: None,
      cursor_pos: (0.0, 0.0),
      modifiers: ModifiersState::empty(),
      attrs: Some(self.attrs),
      on_tick: self.on_tick,
    };
    event_loop.run_app(&mut handler).unwrap();
  }
}

struct WinitHandler {
  runtime: Runtime,
  window: Option<Window>,
  cursor_pos: (f64, f64),
  modifiers: ModifiersState,
  attrs: Option<WindowAttributes>,
  on_tick: Option<TickFn>,
}

impl WinitHandler {
  fn check_redraw(&mut self) {
    if self.runtime.needs_redraw() {
      self.runtime.clear_needs_redraw();
      if let Some(w) = &self.window {
        w.request_redraw();
      }
    }
  }
}

impl ApplicationHandler for WinitHandler {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    if self.window.is_none() {
      let attrs = self.attrs.take().unwrap_or_default();
      let window = event_loop.create_window(attrs).unwrap();
      let size = window.inner_size();
      self.runtime.set_scale_factor(window.scale_factor() as f32);
      self.runtime.resize(size.width, size.height);
      window.request_redraw();
      self.window = Some(window);
    }
  }

  fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
    match event {
      WindowEvent::CloseRequested => event_loop.exit(),
      WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
        self.runtime.set_scale_factor(scale_factor as f32);
        if let Some(w) = &self.window {
          let size = w.inner_size();
          self.runtime.resize(size.width, size.height);
          w.request_redraw();
        }
      }
      WindowEvent::Resized(size) => {
        self.runtime.resize(size.width, size.height);
        if let Some(w) = &self.window {
          w.request_redraw();
        }
      }
      WindowEvent::CursorMoved { position, .. } => {
        self.cursor_pos = (position.x, position.y);
        self.runtime.mouse_move(position.x as f32, position.y as f32);
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
          ElementState::Pressed => self.runtime.mouse_down(x, y, btn),
          ElementState::Released => {
            self.runtime.mouse_up(x, y, btn);
            self.runtime.click(x, y, btn);
          }
        }
        self.check_redraw();
      }
      WindowEvent::ModifiersChanged(modifiers) => {
        self.modifiers = modifiers.state();
      }
      WindowEvent::KeyboardInput { event, .. } => {
        if matches!(event.state, ElementState::Pressed) {
          self.runtime.key_down(
            key_to_string(&event),
            physical_key_to_string(&event.physical_key),
            self.modifiers.shift_key(),
            self.modifiers.control_key(),
            self.modifiers.alt_key(),
          );
          self.check_redraw();
        }
      }
      WindowEvent::MouseWheel { delta, phase, .. } => {
        let (dx, dy) = match delta {
          MouseScrollDelta::LineDelta(x, y) => (x * 40.0, y * 40.0),
          MouseScrollDelta::PixelDelta(p) => (p.x as f32, p.y as f32),
        };
        let scroll_phase = match phase {
          TouchPhase::Started => ScrollPhase::Start,
          TouchPhase::Moved => ScrollPhase::Scroll,
          TouchPhase::Ended | TouchPhase::Cancelled => ScrollPhase::End,
        };
        self
          .runtime
          .scroll(self.cursor_pos.0 as f32, self.cursor_pos.1 as f32, dx, dy, scroll_phase);
        self.check_redraw();
      }
      WindowEvent::RedrawRequested => {
        if let Some(w) = &self.window {
          self.runtime.pass(w);
        }
      }
      _ => {}
    }
  }

  fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
    if let Some(tick) = &mut self.on_tick {
      tick(&mut self.runtime);
      self.check_redraw();
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
