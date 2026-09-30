//! A bar chart on a canvas whose bars are described as items, so agents can
//! read and hover them through the embedded MCP server.
//!
//! cargo run -p lurq --example canvas_chart --features canvas,winit,wgpu,mcp
use std::sync::Mutex;

use lurq::{
  app::{
    App, Tree, component::Component, ctx::Ctx, events::MouseEvent, wgpu_render::WgpuRenderEngine,
    winit_shell::WinitWindow,
  },
  canvas::{CanvasFont, CanvasHandle, CanvasItem, CanvasObserver, TextAlign},
  components::{Canvas, Column, Text},
  core::{ElementRef, Signal},
  mcp::McpConfig,
  node::Element,
};

const RUNS: [(&str, &str, f32); 5] = [
  ("mon", "Mon", 12.0),
  ("tue", "Tue", 18.0),
  ("wed", "Wed", 7.0),
  ("thu", "Thu", 15.0),
  ("fri", "Fri", 10.0),
];

struct Chart {
  reference: ElementRef,
  hovered: Signal<Option<String>>,
  observer: Mutex<Option<CanvasObserver>>,
}

impl Component for Chart {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      reference: ctx.element_ref(),
      hovered: ctx.signal(None),
      observer: Mutex::new(None),
    }
  }

  fn render(&self, _: &mut Ctx) -> impl Into<Element> {
    let (reference, hovered) = (self.reference.clone(), self.hovered.clone());
    let tooltip = self.hovered.get().and_then(|id| {
      let (_, label, runs) = RUNS.into_iter().find(|(bar, ..)| *bar == id)?;
      Some(format!("{label}: {runs} runs"))
    });
    Column::new()
      .padding(16.0)
      .spacing(8.0)
      .background("#182434")
      .child(
        Canvas::new()
          .ref_element(self.reference.clone())
          .id("runs-chart")
          .width(360.0)
          .height(200.0)
          .on_mouse_move(move |event: MouseEvent| {
            let Some(canvas) = reference.as_canvas() else {
              return;
            };
            let item = canvas
              .point_from_window(event.x, event.y)
              .and_then(|(x, y)| canvas.item_at(x, y));
            let bar = item.filter(|item| item.role == "bar").map(|item| item.id);
            if hovered.get_untracked() != bar {
              draw(&canvas, bar.as_deref());
              hovered.set(bar);
            }
          }),
      )
      .child(
        Text::new(tooltip.as_deref().unwrap_or("Hover a bar"))
          .id("tooltip")
          .color("#e2e8f0"),
      )
  }

  fn after_layout(&self) {
    let mut observer = self.observer.lock().unwrap();
    if observer.is_none() {
      let canvas = self.reference.as_canvas().expect("canvas is bound after layout");
      let (target, hovered) = (canvas.clone(), self.hovered.clone());
      *observer = Some(canvas.observe_metrics(move |_| draw(&target, hovered.get_untracked().as_deref())));
    }
  }
}

/// Paint the bars and their day labels, and describe each of them as an item.
fn draw(canvas: &CanvasHandle, hovered: Option<&str>) {
  let context = canvas.context_2d();
  context.reset();
  context.set_font(CanvasFont::new("", 13.0));
  context.set_text_align(TextAlign::Center);
  let mut items = Vec::new();
  for (index, (id, label, runs)) in RUNS.into_iter().enumerate() {
    let (x, height) = (20.0 + index as f32 * 70.0, runs * 8.0);
    let top = 170.0 - height;
    context.set_fill_style(if hovered == Some(id) { "#fbbf24" } else { "#60a5fa" });
    context.fill_rect(x, top, 50.0, height);
    context.set_fill_style("#cbd5e1");
    context
      .fill_text(label, x + 25.0, 190.0)
      .expect("the day label fits the text limits");
    items.push(
      CanvasItem::rect(id, "bar", x, top, 50.0, height)
        .label(label)
        .value(format!("{runs} runs")),
    );
    items.push(CanvasItem::rect(format!("{id}-label"), "label", x, 176.0, 50.0, 18.0).label(label));
  }
  canvas.set_items(items);
}

fn main() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_render_engine_factory(|| Box::new(WgpuRenderEngine::new()));
  tree.mount_root::<Chart>(&mut app, ());
  let mcp = tree.enable_mcp(McpConfig::new().app_name("lurq-canvas-chart"));
  println!("MCP on http://127.0.0.1:{}/mcp", mcp.port());
  WinitWindow::new(app, tree)
    .with_size(420, 300)
    .with_title("Canvas chart: items for agents")
    .with_start_without_focus(true)
    .run();
}
