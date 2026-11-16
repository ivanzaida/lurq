//! A window with no decorations of the system's, whose title bar the app draws from plain elements: its three
//! controls say which of the window's they are with `.window_control(..)`, so the system knows them.
//!
//! cargo run -p lurq --example own_title_bar --features winit,wgpu
//!
//! On Windows 11, rest the pointer on the maximize control: the system's snap layouts open over it. A press on it
//! maximizes the window and restores it, the others minimize and close it, and the bar's own room drags it.

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::Ctx,
    events::{MouseButton, MouseEvent},
    wgpu_render::WgpuRenderEngine,
    winit_shell::WinitWindow,
  },
  components::{Column, Row, Text},
  layout::{Alignment, layout_kind::Justify},
  node::{CursorIcon, Element, Style, WindowControl, color::Color, dimension::Dimension},
};

const BAR_HEIGHT: f32 = 36.0;
const CONTROL_WIDTH: f32 = 46.0;

struct Page;

/// One of the window's controls: an element like any other, marked for the system.
fn control(glyph: &str, control: WindowControl, hover: &str) -> Row {
  Row::new()
    .width(CONTROL_WIDTH)
    .height(BAR_HEIGHT)
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .cursor(CursorIcon::Pointer)
    .hovered_style(Style::new().background(Color::from_hex(hover)))
    .active_style(Style::new().background(Color::from_hex("#3a4250")))
    .window_control(control)
    // A press of a control is the control's: it does not drag the window.
    .on_mouse_down(|event: MouseEvent| event.stop_propagation())
    .child(Text::new(glyph).color(Color::from_hex("#d9dee7")))
}

impl Component for Page {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let window = ctx.window();
    let maximized = window.is_maximized;
    let (drag, minimize, maximize, close) = (window.clone(), window.clone(), window.clone(), window);
    let bar = Row::new()
      .width(Dimension::Pct(100.0))
      .height(BAR_HEIGHT)
      .align_items(Alignment::Center)
      .background(Color::from_hex("#101215"))
      .on_mouse_down(move |event: MouseEvent| {
        if event.button == MouseButton::Left {
          drag.start_drag();
        }
      })
      .child(
        Row::new()
          .flex(1.0)
          .padding_left(12.0)
          .child(Text::new("A title bar of the app's own").color(Color::from_hex("#d9dee7"))),
      )
      .child(control("-", WindowControl::Minimize, "#232934").on_click(move |_| minimize.set_minimized(true)))
      .child(
        control(if maximized { "▢" } else { "□" }, WindowControl::Maximize, "#232934")
          .on_click(move |_| maximize.set_maximized(!maximized)),
      )
      .child(control("x", WindowControl::Close, "#c0392b").on_click(move |_| close.request_close()));
    Column::new()
      .width(Dimension::Pct(100.0))
      .height(Dimension::Pct(100.0))
      .background(Color::from_hex("#181b20"))
      .child(bar)
      .child(
        Column::new().flex(1.0).padding(24.0).child(
          Text::new("Rest the pointer on the maximize control: Windows 11 opens its snap layouts over it.")
            .color(Color::from_hex("#94a3b8")),
        ),
      )
  }
}

fn main() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_render_engine_factory(|| Box::new(WgpuRenderEngine::new()));
  tree.mount_root::<Page>(&mut app, ());
  WinitWindow::new(app, tree)
    .with_title("own title bar")
    .with_size(720, 420)
    .with_decorations(false)
    .run();
}
