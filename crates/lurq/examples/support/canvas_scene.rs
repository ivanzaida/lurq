use std::sync::Mutex;

use lurq::{
  app::{component::Component, ctx::Ctx},
  canvas::{ArcDirection, CanvasFont, CanvasHandle, CanvasObserver},
  components::Canvas,
  core::ElementRef,
  images::ImageData,
  node::Element,
};

pub struct CanvasScene {
  reference: ElementRef,
  observer: Mutex<Option<CanvasObserver>>,
}

impl Component for CanvasScene {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      reference: ctx.element_ref(),
      observer: Mutex::new(None),
    }
  }

  fn render(&self, _: &mut Ctx) -> impl Into<Element> {
    let reference = self.reference.clone();
    Canvas::new()
      .ref_element(self.reference.clone())
      .id("canvas")
      .width(256.0)
      .height(256.0)
      .padding(8.0)
      .background("#182434")
      .on_click(move |event: lurq::app::events::MouseEvent| {
        let Some(canvas) = reference.as_canvas() else {
          return;
        };
        let Some((x, y)) = canvas.point_from_window(event.x, event.y) else {
          return;
        };
        let draw = canvas.context_2d();
        draw.save();
        draw.set_fill_style("#fbbf24");
        draw.fill_rect(x - 3.0, y - 3.0, 6.0, 6.0);
        draw.restore();
      })
  }

  fn after_layout(&self) {
    let mut observer = self.observer.lock().unwrap();
    if observer.is_some() {
      return;
    }
    let canvas = self.reference.as_canvas().expect("canvas is ready after layout");
    let target = canvas.clone();
    *observer = Some(canvas.observe_metrics(move |_| paint_scene(&target)));
  }
}

fn paint_scene(canvas: &CanvasHandle) {
  let draw = canvas.context_2d();
  draw.reset();
  draw.set_fill_style("#ef4444");
  draw.fill_rect(16.0, 16.0, 100.0, 64.0);
  draw.clear_rect(40.0, 32.0, 20.0, 20.0);
  draw.set_fill_style("#4080ff80");
  draw.fill_rect(140.0, 16.0, 64.0, 64.0);
  draw.save();
  draw.begin_path();
  draw
    .arc(66.0, 138.0, 36.0, 0.0, std::f32::consts::TAU, ArcDirection::Clockwise)
    .unwrap();
  draw.clip();
  draw.set_fill_style("#34d399");
  draw.fill_rect(0.0, 100.0, 120.0, 80.0);
  draw.restore();
  draw.set_stroke_style("#60a5fa");
  draw.set_line_width(4.0);
  draw.begin_path();
  draw.move_to(130.0, 165.0);
  draw.bezier_curve_to(135.0, 65.0, 205.0, 195.0, 216.0, 110.0);
  draw.stroke();
  let image = ImageData::from_rgba(vec![251, 191, 36, 255, 167, 139, 250, 255], 2, 1);
  draw.set_image_smoothing_enabled(false);
  draw.draw_image_scaled(&image, 142.0, 180.0, 64.0, 16.0).unwrap();
  draw.set_font(CanvasFont::new("", 18.0));
  draw.set_fill_style("#ffffff");
  draw.fill_text("Canvas through a ref", 16.0, 226.0).unwrap();
}
