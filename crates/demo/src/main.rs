use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{MouseButton, MouseEvent, MouseEventKind, ScrollEvent, ScrollPhase},
    wgpu_render::WgpuRenderEngine,
    Runtime,
  },
  core::Signal,
  layout::{
    scrollbar::ScrollBarStyle,
    text_style::{FontStyle, FontWeight, TextStyle},
    Alignment,
  },
  node::{color::Color, dsl::*, Node},
};
use winit::{
  application::ApplicationHandler,
  event::{ElementState, MouseScrollDelta, TouchPhase, WindowEvent},
  event_loop::{ActiveEventLoop, EventLoop},
  window::{Window, WindowId},
};

// --- Counter component ---

struct Counter {
  count: Signal<i32>,
}

impl Component for Counter {
  type Props = ();

  fn create(ctx: &mut Ctx, _: ()) -> Self {
    Self { count: ctx.signal(0) }
  }

  fn render(&self, _ctx: &mut Ctx) -> Node {
    let c = self.count.clone();
    let c2 = self.count.clone();
    let val = self.count.get();

    row()
      .spacing(12.0)
      .align_items(Alignment::Center)
      .child(
        rect(36.0, 36.0)
          .fill("#ef4444")
          .rounded(6.0)
          .on_click(move |_| c.update(|n| *n -= 1)),
      )
      .child(styled_text(
        &format!("{}", val),
        TextStyle {
          font_size: 24.0,
          weight: FontWeight::Bold,
          color: Color::from_hex("#1e293b"),
          ..TextStyle::default()
        },
      ))
      .child(
        rect(36.0, 36.0)
          .fill("#22c55e")
          .rounded(6.0)
          .on_click(move |_| c2.update(|n| *n += 1)),
      )
  }
}

// --- ScrollList component ---

struct ScrollList {
  scroll: lurq::layout::layout_kind::ScrollState,
}

impl Component for ScrollList {
  type Props = usize;

  fn create(_ctx: &mut Ctx, _props: usize) -> Self {
    Self {
      scroll: lurq::layout::layout_kind::ScrollState::new(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> Node {
    scroll_vertical(
      column()
        .spacing(4.0)
        .align_items(Alignment::Center)
        .with_children((0..20).map(|i| {
          rect(270.0, 32.0)
            .fill(if i % 2 == 0 { "#93c5fd" } else { "#fca5a5" })
            .rounded(4.0)
            .on_click(move |_| println!("Item {} clicked", i))
        }))
        .pad_xy(0.0, 8.0),
    )
    .with_scroll_state(self.scroll.clone())
    .scrollbar(ScrollBarStyle {
      width: 6.0,
      thumb_color: Color::from_hex("#c0c0c0"),
      thumb_hover_color: Color::from_hex("#808080"),
      thumb_radius: 3.0,
      ..ScrollBarStyle::default()
    })
    .size(300.0, 180.0)
    .rounded(8.0)
    .fill("#ffffff")
    .border_inside(1.0, Color::from_hex("#94a3b8"))
  }
}

// --- Root app component ---

struct DemoApp;

impl Component for DemoApp {
  type Props = ();

  fn create(_ctx: &mut Ctx, _: ()) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> Node {
    column()
      .spacing(16.0)
      .align_items(Alignment::Center)
      .child(styled_text(
        "lurq demo",
        TextStyle {
          font_size: 32.0,
          color: Color::from_hex("#1e293b"),
          ..TextStyle::default()
        },
      ))
      .child(ctx.mount::<Counter>(()))
      .child(ctx.mount::<ScrollList>(20))
      .child(
        rect(300.0, 40.0)
          .fill("#3b82f6")
          .rounded(20.0)
          .border_inside(2.0, Color::from_hex("#1d4ed8"))
          .on_click(|_| println!("Blue button clicked!")),
      )
      .child(
        row()
          .spacing(8.0)
          .align_items(Alignment::Center)
          .child(styled_text(
            "Bold",
            TextStyle {
              weight: FontWeight::Bold,
              color: Color::from_hex("#dc2626"),
              ..TextStyle::default()
            },
          ))
          .child(text("and"))
          .child(styled_text(
            "italic",
            TextStyle {
              style: FontStyle::Italic,
              color: Color::from_hex("#2563eb"),
              ..TextStyle::default()
            },
          ))
          .child(text("text.")),
      )
      .pad(32.0)
  }
}

// --- Winit app ---

struct WinitApp {
  runtime: Runtime,
  window: Option<Window>,
  cursor_pos: (f64, f64),
}

impl WinitApp {
  fn new() -> Self {
    let mut runtime = Runtime::new();
    runtime.set_render_engine(Box::new(WgpuRenderEngine::new()));
    runtime.mount_root::<DemoApp>(());
    Self {
      runtime,
      window: None,
      cursor_pos: (0.0, 0.0),
    }
  }

  fn check_redraw(&mut self) {
    if self.runtime.needs_redraw() {
      self.runtime.clear_needs_redraw();
      if let Some(w) = &self.window {
        w.request_redraw();
      }
    }
  }
}

impl ApplicationHandler for WinitApp {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    if self.window.is_none() {
      let attrs = Window::default_attributes().with_title("lurq demo");
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
        self.runtime.propagate_mouse_event(MouseEvent {
          x: position.x as f32,
          y: position.y as f32,
          button: MouseButton::Left,
          kind: MouseEventKind::Move,
        });
        self.check_redraw();
      }
      WindowEvent::MouseInput { state, button, .. } => {
        let btn = match button {
          winit::event::MouseButton::Left => MouseButton::Left,
          winit::event::MouseButton::Right => MouseButton::Right,
          winit::event::MouseButton::Middle => MouseButton::Middle,
          _ => MouseButton::Left,
        };
        let kind = match state {
          ElementState::Pressed => MouseEventKind::Down,
          ElementState::Released => MouseEventKind::Up,
        };
        self.runtime.propagate_mouse_event(MouseEvent {
          x: self.cursor_pos.0 as f32,
          y: self.cursor_pos.1 as f32,
          button: btn,
          kind,
        });
        if state == ElementState::Released {
          self.runtime.propagate_mouse_event(MouseEvent {
            x: self.cursor_pos.0 as f32,
            y: self.cursor_pos.1 as f32,
            button: btn,
            kind: MouseEventKind::Click,
          });
        }
        self.check_redraw();
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
        self.runtime.propagate_scroll_event(ScrollEvent {
          x: self.cursor_pos.0 as f32,
          y: self.cursor_pos.1 as f32,
          delta_x: dx,
          delta_y: dy,
          phase: scroll_phase,
        });
        self.check_redraw();
      }
      WindowEvent::RedrawRequested => {
        if let Some(w) = &self.window {
          self.runtime.pass(w);
          eprintln!("{}", self.runtime.last_profile());
        }
      }
      _ => {}
    }
  }
}

fn main() {
  let event_loop = EventLoop::new().unwrap();
  let mut app = WinitApp::new();
  event_loop.run_app(&mut app).unwrap();
}
