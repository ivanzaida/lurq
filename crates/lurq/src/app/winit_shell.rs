use std::time::{Duration, Instant};

use winit::{
  application::ApplicationHandler,
  event::{ElementState, MouseScrollDelta, TouchPhase, WindowEvent},
  event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
  keyboard::{Key, ModifiersState, NamedKey, PhysicalKey},
  window::{CursorIcon as WinitCursorIcon, Window, WindowAttributes, WindowId},
};

use crate::{
  app::{
    Runtime,
    events::{MouseButton, ScrollPhase},
  },
  node::CursorIcon,
};

type TickFn = Box<dyn FnMut(&mut Runtime)>;
const FRAME_INTERVAL: Duration = Duration::from_millis(16);
const DOUBLE_CLICK_INTERVAL: Duration = Duration::from_millis(500);
const DOUBLE_CLICK_DISTANCE: f64 = 4.0;

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
      cursor: CursorIcon::Default,
      modifiers: ModifiersState::empty(),
      attrs: Some(self.attrs),
      on_tick: self.on_tick,
      redraw_pending: false,
      last_present: Instant::now() - FRAME_INTERVAL,
      click_tracker: ClickTracker::default(),
    };
    event_loop.run_app(&mut handler).unwrap();
  }
}

struct WinitHandler {
  runtime: Runtime,
  window: Option<Window>,
  cursor_pos: (f64, f64),
  cursor: CursorIcon,
  modifiers: ModifiersState,
  attrs: Option<WindowAttributes>,
  on_tick: Option<TickFn>,
  redraw_pending: bool,
  last_present: Instant,
  click_tracker: ClickTracker,
}

impl WinitHandler {
  fn check_redraw(&mut self) {
    if !self.runtime.needs_redraw() {
      return;
    }

    if self.last_present.elapsed() >= FRAME_INTERVAL {
      self.present_now();
    } else if !self.redraw_pending {
      if let Some(w) = &self.window {
        self.redraw_pending = true;
        w.request_redraw();
      }
    }
  }

  fn present_now(&mut self) {
    if let Some(w) = &self.window {
      self.redraw_pending = false;
      self.runtime.clear_needs_redraw();
      self.runtime.pass(w);
      self.last_present = Instant::now();
    }
  }

  fn apply_cursor(&mut self) {
    let cursor = self.runtime.cursor();
    if cursor == self.cursor {
      return;
    }

    if let Some(window) = &self.window {
      window.set_cursor(to_winit_cursor(cursor));
    }
    self.cursor = cursor;
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
        self.apply_cursor();
        self.check_redraw();
      }
      WindowEvent::CursorLeft { .. } => {
        self.runtime.mouse_leave_window();
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
          ElementState::Pressed => self.runtime.mouse_down(x, y, btn),
          ElementState::Released => {
            self.runtime.mouse_up(x, y, btn);
            self.runtime.click(x, y, btn);
            if self.click_tracker.record_click(Instant::now(), self.cursor_pos, btn) {
              self.runtime.dblclick(x, y, btn);
            }
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
          self.runtime.key_down(
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
          .runtime
          .scroll(self.cursor_pos.0 as f32, self.cursor_pos.1 as f32, dx, dy, scroll_phase);
        self.apply_cursor();
        self.check_redraw();
      }
      WindowEvent::RedrawRequested => {
        self.present_now();
        self.check_redraw();
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

#[derive(Default)]
struct ClickTracker {
  last_click: Option<LastClick>,
}

impl ClickTracker {
  fn record_click(&mut self, now: Instant, position: (f64, f64), button: MouseButton) -> bool {
    let is_double_click = self.last_click.is_some_and(|last| {
      last.button == button
        && now.duration_since(last.time) <= DOUBLE_CLICK_INTERVAL
        && distance_squared(last.position, position) <= DOUBLE_CLICK_DISTANCE * DOUBLE_CLICK_DISTANCE
    });

    if is_double_click {
      self.last_click = None;
    } else {
      self.last_click = Some(LastClick {
        time: now,
        position,
        button,
      });
    }

    is_double_click
  }
}

#[derive(Clone, Copy)]
struct LastClick {
  time: Instant,
  position: (f64, f64),
  button: MouseButton,
}

fn distance_squared(a: (f64, f64), b: (f64, f64)) -> f64 {
  let dx = a.0 - b.0;
  let dy = a.1 - b.1;
  dx * dx + dy * dy
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn click_tracker_detects_second_nearby_click_within_interval() {
    let mut tracker = ClickTracker::default();
    let now = Instant::now();

    assert!(!tracker.record_click(now, (20.0, 30.0), MouseButton::Left));
    assert!(tracker.record_click(now + Duration::from_millis(250), (22.0, 31.0), MouseButton::Left));
  }

  #[test]
  fn click_tracker_rejects_slow_or_distant_or_different_button_clicks() {
    let mut tracker = ClickTracker::default();
    let now = Instant::now();

    assert!(!tracker.record_click(now, (20.0, 30.0), MouseButton::Left));
    assert!(!tracker.record_click(
      now + DOUBLE_CLICK_INTERVAL + Duration::from_millis(1),
      (20.0, 30.0),
      MouseButton::Left
    ));
    assert!(!tracker.record_click(
      now + DOUBLE_CLICK_INTERVAL + Duration::from_millis(2),
      (20.0, 30.0),
      MouseButton::Right
    ));
    assert!(!tracker.record_click(
      now + DOUBLE_CLICK_INTERVAL + Duration::from_millis(3),
      (20.0 + DOUBLE_CLICK_DISTANCE + 1.0, 30.0),
      MouseButton::Right
    ));
  }
}
