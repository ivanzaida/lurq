//! The page of the engine's canvas: a surface an app draws on with a 2D context, as a browser's canvas is drawn on,
//! whose drawing stays until it is drawn over. Its playground draws a shape with the fill, stroke and shadow its
//! props say; its sections draw paths, gradients, effects and text, take a drawing from the pointer, move a chart to
//! new values, and draw a picture with no window. The demo had no page of it.

use std::{
  f32::consts::TAU,
  hash::{Hash, Hasher},
  sync::{Arc, Mutex},
  time::Duration,
};

use lurq::{
  app::{component::Component, ctx::Ctx, events::MouseEvent, theme::PaletteColor},
  canvas::{
    ArcDirection, BlendMode, CanvasFont, CanvasHandle, CanvasItem, CanvasMotion, CanvasMove, Context2D, FillRule,
    Filter, Gradient, LineCap, LineJoin, Shadow, TextAlign, TextBaseline,
  },
  components::{Canvas, Column, Row},
  core::{ElementRef, Signal},
  images::ImageData,
  layout::{Alignment, text_style::FontWeight},
  node::{Element, color::Color},
  ui::{Button, ButtonSize, ButtonVariant},
};

use super::kit::{controls, well};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, mono, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// The theme's colors a drawing is made in, read where the page is built: a canvas is given colors, not roles.
#[derive(Clone, Copy)]
struct Look {
  foreground: Color,
  muted: Color,
  card: Color,
  scrim: Color,
  charts: [Color; 5],
}

impl Look {
  fn now() -> Self {
    let theme = lurq::ui::current_theme();
    let color = |role: PaletteColor| theme.palette_color(role);
    Self {
      foreground: color(PaletteColor::Foreground),
      muted: color(PaletteColor::MutedForeground),
      card: color(PaletteColor::Card),
      scrim: color(PaletteColor::Scrim),
      charts: [
        color(PaletteColor::Chart1),
        color(PaletteColor::Chart2),
        color(PaletteColor::Chart3),
        color(PaletteColor::Chart4),
        color(PaletteColor::Chart5),
      ],
    }
  }

  /// A hash of the colors: a drawing made in them is drawn again when the theme changes them.
  fn hash_into(&self, hasher: &mut impl Hasher) {
    let all = [self.foreground, self.muted, self.card, self.scrim]
      .into_iter()
      .chain(self.charts);
    for color in all {
      [color.r(), color.g(), color.b(), color.a()].hash(hasher);
    }
  }
}

/// The key of a drawing made in `look` from `what`: a canvas's painter runs again when it changes.
fn key(look: &Look, what: impl Hash) -> u64 {
  let mut hasher = std::collections::hash_map::DefaultHasher::new();
  look.hash_into(&mut hasher);
  what.hash(&mut hasher);
  hasher.finish()
}

/// How much smaller than the width it is drawn for a scene is drawn on a canvas `width` wide: none where there is
/// room for it.
fn shrink(width: f32, drawn_for: f32) -> f32 {
  (width / drawn_for).clamp(0.1, 1.0)
}

/// A star of five points about (`x`, `y`), its points `radius` from the middle, drawn as one line that crosses
/// itself: filled by the even-odd rule, its middle is left out.
fn star(draw: &Context2D, x: f32, y: f32, radius: f32) {
  draw.begin_path();
  for point in 0..5 {
    let angle = -TAU / 4.0 + point as f32 * 2.0 * TAU / 5.0;
    let (px, py) = (x + radius * angle.cos(), y + radius * angle.sin());
    if point == 0 {
      draw.move_to(px, py);
    } else {
      draw.line_to(px, py);
    }
  }
  draw.close_path();
}

const SHAPES: [(&str, usize); 6] = [
  ("Rect", 0),
  ("Round rect", 1),
  ("Circle", 2),
  ("Star", 3),
  ("Curve", 4),
  ("Text", 5),
];
const FILLS: [(&str, usize); 5] = [("None", 0), ("Solid", 1), ("Linear", 2), ("Radial", 3), ("Angular", 4)];
const STROKES: [(&str, i32); 3] = [("0", 0), ("2", 2), ("6", 6)];
/// How high the playground's canvas is.
const PLAY_HEIGHT: f32 = 240.0;

/// The fill of a shape whose box is (`x`, `y`, `w`, `h`): a color, or a gradient in that box.
fn fill_paint(fill: usize, look: &Look, x: f32, y: f32, w: f32, h: f32) -> Option<lurq::canvas::Paint> {
  let [one, two, three, four, five] = look.charts;
  match fill {
    1 => Some(lurq::canvas::Paint::Solid(one)),
    2 => Some(
      Gradient::linear()
        .stop(0.0, one)
        .stop(1.0, three)
        .rotation(TAU / 8.0)
        .in_box(x, y, w, h),
    ),
    3 => Some(Gradient::radial().stop(0.0, four).stop(1.0, two).in_box(x, y, w, h)),
    4 => Some(
      Gradient::angular()
        .stop(0.0, one)
        .stop(0.25, two)
        .stop(0.5, three)
        .stop(0.75, five)
        .stop(1.0, one)
        .in_box(x, y, w, h),
    ),
    _ => None,
  }
}

/// The playground's drawing: one shape in the middle of the canvas, filled, stroked and with a shadow as its props
/// say.
fn paint_shape(canvas: &CanvasHandle, look: &Look, shape: usize, fill: usize, stroke: i32, shadow: bool, dashed: bool) {
  let draw = canvas.context_2d();
  draw.reset();
  let size = canvas.size();
  let (cx, cy) = (size.width / 2.0, size.height / 2.0);
  let side = (size.height - 64.0).min(size.width - 64.0).max(16.0);
  let (x, y, w, h) = (cx - side * 0.7, cy - side / 2.0, side * 1.4, side);
  if shadow {
    draw.set_shadow(Some(Shadow::new(look.scrim).offset(0.0, 8.0).blur(18.0)));
  }
  if shape == 5 {
    let mut font = CanvasFont::new("", (side * 0.6).min(96.0));
    font.weight = FontWeight::Bold;
    draw.set_font(font);
    draw.set_text_align(TextAlign::Center);
    draw.set_text_baseline(TextBaseline::Middle);
    // Shaped text is drawn in one color: a gradient is refused, and the text is not drawn.
    if let Some(paint) = fill_paint(fill, look, x, y, w, h) {
      draw.set_fill_style(paint);
      let _ = draw.fill_text("lurq", cx, cy);
    }
    return;
  }
  draw.begin_path();
  match shape {
    0 => draw.rect(x, y, w, h),
    1 => {
      let _ = draw.round_rect(x, y, w, h, 24.0);
    }
    2 => {
      let _ = draw.arc(cx, cy, side / 2.0, 0.0, TAU, ArcDirection::Clockwise);
    }
    3 => star(&draw, cx, cy + side * 0.05, side / 2.0),
    _ => {
      draw.move_to(x, y + h);
      draw.bezier_curve_to(x + w * 0.25, y - h * 0.4, x + w * 0.6, y + h * 1.4, x + w, y);
    }
  }
  if let Some(paint) = fill_paint(fill, look, x, y, w, h) {
    draw.set_fill_style(paint);
    if shape == 3 {
      draw.fill_with_rule(FillRule::EvenOdd);
    } else {
      draw.fill();
    }
  }
  draw.set_shadow(None);
  if stroke > 0 {
    draw.set_stroke_style(look.foreground);
    draw.set_line_width(stroke as f32);
    draw.set_line_join(LineJoin::Round);
    draw.set_line_cap(LineCap::Round);
    if dashed {
      draw.set_line_dash(&[14.0, 10.0]);
    }
    draw.stroke();
  }
}

/// The Rust of the playground's drawing, as its props are.
fn shape_code(shape: usize, fill: usize, stroke: i32, shadow: bool, dashed: bool, software: bool) -> Vec<String> {
  let mut code = vec![
    "Canvas::new()".to_owned(),
    "    .size(Dimension::Pct(100.0), 240.0)".to_owned(),
  ];
  if software {
    code.push("    .software()".to_owned());
  }
  code.push("    .paint(key, move |canvas| {".to_owned());
  code.push("        let draw = canvas.context_2d();".to_owned());
  code.push("        draw.reset();".to_owned());
  let body = |line: &str| format!("        {line}");
  if shadow {
    code.push(body(
      "draw.set_shadow(Some(Shadow::new(scrim).offset(0.0, 8.0).blur(18.0)));",
    ));
  }
  let paint = match fill {
    1 => Some("chart1"),
    2 => Some("Gradient::linear().stop(0.0, chart1).stop(1.0, chart3).rotation(TAU / 8.0).in_box(x, y, w, h)"),
    3 => Some("Gradient::radial().stop(0.0, chart4).stop(1.0, chart2).in_box(x, y, w, h)"),
    4 => Some("Gradient::angular().stop(0.0, chart1).stop(0.25, chart2) /* .. */ .in_box(x, y, w, h)"),
    _ => None,
  };
  if shape == 5 {
    code.push(body(
      "draw.set_font(CanvasFont { weight: FontWeight::Bold, ..CanvasFont::new(\"\", 96.0) });",
    ));
    code.push(body("draw.set_text_align(TextAlign::Center);"));
    code.push(body("draw.set_text_baseline(TextBaseline::Middle);"));
    if let Some(paint) = paint {
      code.push(body(&format!("draw.set_fill_style({paint});")));
      code.push(body("draw.fill_text(\"lurq\", cx, cy).ok();"));
    }
    code.push("    })".to_owned());
    return code;
  }
  code.push(body("draw.begin_path();"));
  code.push(body(match shape {
    0 => "draw.rect(x, y, w, h);",
    1 => "draw.round_rect(x, y, w, h, 24.0).ok();",
    2 => "draw.arc(cx, cy, r, 0.0, TAU, ArcDirection::Clockwise).ok();",
    3 => "star(&draw, cx, cy, r); // five line_to that cross",
    _ => "draw.move_to(x, y + h); draw.bezier_curve_to(..);",
  }));
  if let Some(paint) = paint {
    code.push(body(&format!("draw.set_fill_style({paint});")));
    code.push(body(if shape == 3 {
      "draw.fill_with_rule(FillRule::EvenOdd);"
    } else {
      "draw.fill();"
    }));
  }
  if shadow {
    code.push(body("draw.set_shadow(None);"));
  }
  if stroke > 0 {
    code.push(body("draw.set_stroke_style(foreground);"));
    code.push(body(&format!("draw.set_line_width({stroke}.0);")));
    if dashed {
      code.push(body("draw.set_line_dash(&[14.0, 10.0]);"));
    }
    code.push(body("draw.stroke();"));
  }
  code.push("    })".to_owned());
  code
}

/// Paths, fills and strokes: a rectangle with a hole cleared in it, two that are see-through, stripes clipped to a
/// circle, a curve, a dashed line, a star filled by the even-odd rule and a picture of two pixels drawn large.
fn paint_paths(canvas: &CanvasHandle, look: &Look) {
  let draw = canvas.context_2d();
  draw.reset();
  let k = shrink(canvas.size().width, 480.0);
  draw.scale(k, k);
  let [one, two, three, four, five] = look.charts;
  draw.set_fill_style(one);
  draw.fill_rect(16.0, 16.0, 120.0, 72.0);
  draw.clear_rect(44.0, 36.0, 24.0, 24.0);
  draw.set_fill_style(two.with_opacity(0.6));
  draw.fill_rect(160.0, 16.0, 72.0, 72.0);
  draw.set_fill_style(three.with_opacity(0.6));
  draw.fill_rect(196.0, 40.0, 72.0, 72.0);

  draw.save();
  draw.begin_path();
  let _ = draw.arc(360.0, 64.0, 48.0, 0.0, TAU, ArcDirection::Clockwise);
  draw.clip();
  for stripe in 0..8 {
    draw.set_fill_style(if stripe % 2 == 0 { four } else { five });
    draw.fill_rect(304.0 + stripe as f32 * 14.0, 0.0, 14.0, 130.0);
  }
  draw.restore();

  draw.set_stroke_style(look.foreground);
  draw.set_line_width(4.0);
  draw.set_line_cap(LineCap::Round);
  draw.begin_path();
  draw.move_to(24.0, 220.0);
  draw.bezier_curve_to(80.0, 110.0, 160.0, 280.0, 220.0, 150.0);
  draw.stroke();

  draw.set_line_width(2.0);
  draw.set_line_dash(&[10.0, 6.0]);
  draw.begin_path();
  draw.move_to(250.0, 146.0);
  draw.line_to(464.0, 146.0);
  draw.stroke();
  draw.set_line_dash(&[]);

  star(&draw, 300.0, 200.0, 40.0);
  draw.set_fill_style(one);
  draw.fill_with_rule(FillRule::EvenOdd);

  let pixel = |color: Color| [color.r(), color.g(), color.b(), 255];
  let image = ImageData::from_rgba([pixel(four), pixel(two)].concat(), 2, 1);
  draw.set_image_smoothing_enabled(false);
  let _ = draw.draw_image_scaled(&image, 368.0, 176.0, 96.0, 48.0);
}

/// Three gradients side by side: along a line turned by an eighth of a turn, out from a middle, and around it.
fn paint_gradients(canvas: &CanvasHandle, look: &Look) {
  let draw = canvas.context_2d();
  draw.reset();
  let size = canvas.size();
  let gap = 12.0;
  let w = ((size.width - 2.0 * gap) / 3.0).max(1.0);
  let h = size.height;
  for (index, fill) in [2, 3, 4].into_iter().enumerate() {
    let x = index as f32 * (w + gap);
    if let Some(paint) = fill_paint(fill, look, x, 0.0, w, h) {
      draw.set_fill_style(paint);
      draw.begin_path();
      let _ = draw.round_rect(x, 0.0, w, h, 12.0);
      draw.fill();
    }
  }
}

/// A shadow, a blur, a blend mode, and two pairs of squares at half: drawn each at half, and drawn whole into a
/// layer that is then put down at half, where their overlap is no darker.
fn paint_effects(canvas: &CanvasHandle, look: &Look) {
  let draw = canvas.context_2d();
  draw.reset();
  let k = shrink(canvas.size().width, 480.0);
  draw.scale(k, k);
  let [one, two, three, four, _] = look.charts;

  draw.set_shadow(Some(Shadow::new(look.scrim).offset(0.0, 8.0).blur(16.0)));
  draw.set_fill_style(look.card);
  draw.begin_path();
  let _ = draw.round_rect(20.0, 20.0, 110.0, 80.0, 12.0);
  draw.fill();
  draw.set_shadow(None);

  draw.set_filter(Filter::Blur(6.0));
  draw.set_fill_style(two);
  draw.begin_path();
  let _ = draw.arc(220.0, 60.0, 40.0, 0.0, TAU, ArcDirection::Clockwise);
  draw.fill();
  draw.set_filter(Filter::None);

  draw.set_fill_style(one);
  draw.begin_path();
  let _ = draw.arc(340.0, 60.0, 40.0, 0.0, TAU, ArcDirection::Clockwise);
  draw.fill();
  draw.set_global_composite_operation(BlendMode::Multiply);
  draw.set_fill_style(three);
  draw.begin_path();
  let _ = draw.arc(390.0, 60.0, 40.0, 0.0, TAU, ArcDirection::Clockwise);
  draw.fill();
  draw.set_global_composite_operation(BlendMode::Normal);

  draw.set_global_alpha(0.5);
  draw.set_fill_style(four);
  draw.fill_rect(40.0, 140.0, 70.0, 70.0);
  draw.fill_rect(80.0, 170.0, 70.0, 70.0);
  draw.set_global_alpha(1.0);

  if draw.begin_layer(0.5, BlendMode::Normal).is_ok() {
    draw.set_fill_style(four);
    draw.fill_rect(260.0, 140.0, 70.0, 70.0);
    draw.fill_rect(300.0, 170.0, 70.0, 70.0);
    let _ = draw.end_layer();
  }
}

/// Text at three alignments about a line, at four baselines on a line, and a word with its measured width under it.
fn paint_text(canvas: &CanvasHandle, look: &Look) {
  let draw = canvas.context_2d();
  draw.reset();
  let k = shrink(canvas.size().width, 480.0);
  draw.scale(k, k);
  let guide = |draw: &Context2D, from: (f32, f32), to: (f32, f32)| {
    draw.set_stroke_style(look.muted);
    draw.set_line_width(1.0);
    draw.begin_path();
    draw.move_to(from.0, from.1);
    draw.line_to(to.0, to.1);
    draw.stroke();
  };
  guide(&draw, (240.0, 8.0), (240.0, 112.0));
  draw.set_font(CanvasFont::new("", 18.0));
  draw.set_fill_style(look.foreground);
  for (index, (name, align)) in [
    ("Left", TextAlign::Left),
    ("Center", TextAlign::Center),
    ("Right", TextAlign::Right),
  ]
  .into_iter()
  .enumerate()
  {
    draw.set_text_align(align);
    let _ = draw.fill_text(&format!("TextAlign::{name}"), 240.0, 36.0 + index as f32 * 32.0);
  }

  guide(&draw, (8.0, 160.0), (472.0, 160.0));
  draw.set_text_align(TextAlign::Left);
  for (index, (name, baseline)) in [
    ("Top", TextBaseline::Top),
    ("Middle", TextBaseline::Middle),
    ("Alphabetic", TextBaseline::Alphabetic),
    ("Bottom", TextBaseline::Bottom),
  ]
  .into_iter()
  .enumerate()
  {
    draw.set_text_baseline(baseline);
    let _ = draw.fill_text(name, 12.0 + index as f32 * 116.0, 160.0);
  }

  draw.set_text_baseline(TextBaseline::Alphabetic);
  let mut font = CanvasFont::new("", 24.0);
  font.weight = FontWeight::SemiBold;
  draw.set_font(font);
  draw.set_fill_style(look.charts[0]);
  let _ = draw.fill_text("measure_text", 12.0, 228.0);
  if let Ok(metrics) = draw.measure_text("measure_text") {
    guide(&draw, (12.0, 238.0), (12.0 + metrics.width, 238.0));
    draw.set_font(CanvasFont::new("", 14.0));
    draw.set_fill_style(look.muted);
    let _ = draw.fill_text(&format!("width {:.1}", metrics.width), 24.0 + metrics.width, 228.0);
  }
}

/// What the pointer has drawn: its strokes, a line of points each, and whether a button is held.
#[derive(Default)]
struct Ink {
  strokes: Vec<Vec<(f32, f32)>>,
  down: bool,
}

/// How the pointer's line is drawn: a round pen of 3 in the foreground's color.
fn pen(draw: &Context2D, color: Color) {
  draw.set_stroke_style(color);
  draw.set_line_width(3.0);
  draw.set_line_cap(LineCap::Round);
  draw.set_line_join(LineJoin::Round);
}

/// The whole of what the pointer drew, from a cleared surface: when the canvas is first laid out, resized, or
/// cleared.
fn paint_ink(canvas: &CanvasHandle, ink: &Ink, color: Color) {
  let draw = canvas.context_2d();
  draw.reset();
  pen(&draw, color);
  for stroke in &ink.strokes {
    let Some((first, rest)) = stroke.split_first() else {
      continue;
    };
    draw.begin_path();
    draw.move_to(first.0, first.1);
    if rest.is_empty() {
      draw.line_to(first.0 + 0.01, first.1);
    }
    for point in rest {
      draw.line_to(point.0, point.1);
    }
    draw.stroke();
  }
}

/// The days of the moving chart.
const DAYS: [&str; 5] = ["Mon", "Tue", "Wed", "Thu", "Fri"];

/// The values of the chart for `seed`, between 3 and 20: the same seed gives the same values.
fn runs(seed: usize) -> [f32; 5] {
  let mut state = (seed as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x2545_F491_4F6C_DD1D;
  std::array::from_fn(|_| {
    state ^= state << 13;
    state ^= state >> 7;
    state ^= state << 17;
    3.0 + (state % 18) as f32
  })
}

/// The chart on its way from what it showed to `at.to`: a bar a day, the one under the pointer in another color,
/// each described as an item a pointer and a tool can find.
fn paint_bars(canvas: &CanvasHandle, at: CanvasMove<'_, [f32; 5]>, look: &Look, hovered: Option<usize>) {
  let draw = canvas.context_2d();
  draw.reset();
  let size = canvas.size();
  let (top, foot) = (12.0, size.height - 28.0);
  let slot = size.width / DAYS.len() as f32;
  let bar = (slot * 0.6).min(64.0);
  let scale = (foot - top) / 20.0;
  draw.set_font(CanvasFont::new("", 13.0));
  draw.set_text_align(TextAlign::Center);
  let mut items = Vec::new();
  for (index, day) in DAYS.iter().enumerate() {
    let from = at.from.map(|from| from[index]).unwrap_or(0.0);
    let value = from + (at.to[index] - from) * at.progress;
    let x = index as f32 * slot + (slot - bar) / 2.0;
    let height = value * scale;
    draw.set_fill_style(if hovered == Some(index) {
      look.charts[1]
    } else {
      look.charts[0]
    });
    draw.begin_path();
    let _ = draw.round_rect(x, foot - height, bar, height, 4.0);
    draw.fill();
    draw.set_fill_style(look.muted);
    let _ = draw.fill_text(day, x + bar / 2.0, size.height - 8.0);
    let end = at.to[index] * scale;
    items.push(
      CanvasItem::rect(day.to_lowercase(), "bar", x, foot - end, bar, end)
        .label(*day)
        .value(format!("{} runs", at.to[index])),
    );
  }
  canvas.set_items(items);
}

/// A picture drawn on a canvas in no window, on the CPU, and kept as pixels: drawn again only when the theme's
/// colors change.
fn offscreen(look: &Look) -> Option<ImageData> {
  static KEPT: Mutex<Option<(u64, ImageData)>> = Mutex::new(None);
  let mut kept = KEPT.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
  let key = key(look, "offscreen");
  if let Some((at, image)) = kept.as_ref()
    && *at == key
  {
    return Some(image.clone());
  }
  let canvas = CanvasHandle::offscreen(280.0, 124.0, 2.0)?;
  let draw = canvas.context_2d();
  draw.set_fill_style(
    Gradient::linear()
      .stop(0.0, look.charts[0])
      .stop(1.0, look.charts[2])
      .in_box(0.0, 0.0, 280.0, 124.0),
  );
  draw.begin_path();
  let _ = draw.round_rect(0.0, 0.0, 280.0, 124.0, 16.0);
  draw.fill();
  draw.set_stroke_style(look.foreground);
  draw.set_line_width(3.0);
  draw.set_line_cap(LineCap::Round);
  draw.begin_path();
  draw.move_to(24.0, 90.0);
  draw.bezier_curve_to(80.0, 10.0, 150.0, 120.0, 256.0, 30.0);
  draw.stroke();
  let image = canvas.to_image_data()?;
  *kept = Some((key, image.clone()));
  Some(image)
}

pub struct CanvasPage {
  frame: Frame,
  playground: Playground,
  shape: Signal<usize>,
  fill: Signal<usize>,
  stroke: Signal<i32>,
  shadow: Signal<bool>,
  dashed: Signal<bool>,
  software: Signal<bool>,
  /// The canvas the pointer draws on, what it drew, how many strokes, and how many times it was cleared.
  sketch: ElementRef,
  ink: Arc<Mutex<Ink>>,
  strokes: Signal<usize>,
  cleared: Signal<usize>,
  /// The chart that moves: the seed of its values, and the bar under the pointer (its day, or nothing).
  chart: ElementRef,
  seed: Signal<usize>,
  hovered: Signal<String>,
}

impl CanvasPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (shape, fill, stroke) = (self.shape.get(), self.fill.get(), self.stroke.get());
    let (shadow, dashed, software) = (self.shadow.get(), self.dashed.get(), self.software.get());
    let text = shape == 5;
    let look = Look::now();
    let painted = key(&look, (shape, fill, stroke, shadow, dashed));
    let build = move || -> Element {
      let mut canvas = Canvas::new().size(FILL, PLAY_HEIGHT);
      if software {
        canvas = canvas.software();
      }
      canvas
        .paint(painted, move |canvas| {
          paint_shape(canvas, &look, shape, fill, stroke, shadow, dashed)
        })
        .into()
    };

    let facts = if text && fill >= 2 {
      "fill_text refuses a gradient: nothing is drawn".to_owned()
    } else if software {
      "drawn on the CPU".to_owned()
    } else {
      "drawn on the GPU".to_owned()
    };
    let (set_shape, set_fill, set_stroke) = (self.shape.clone(), self.fill.clone(), self.stroke.clone());
    let reset = {
      let (shape, fill, stroke, shadow, dashed, software) = (
        self.shape.clone(),
        self.fill.clone(),
        self.stroke.clone(),
        self.shadow.clone(),
        self.dashed.clone(),
        self.software.clone(),
      );
      Arc::new(move || {
        shape.set(1);
        fill.set(2);
        stroke.set(2);
        shadow.set(true);
        dashed.set(false);
        software.set(false);
      })
    };
    let mut controls = vec![
      prop(
        "path",
        "Context2D",
        self
          .playground
          .select(0, &SHAPES, &shape, move |value| set_shape.set(value)),
      ),
      prop(
        "set_fill_style",
        "impl CanvasPaint",
        self
          .playground
          .select(1, &FILLS, &fill, move |value| set_fill.set(value)),
      ),
    ];
    // Text is filled and not stroked: stroke_text is not in the canvas's subset.
    if !text {
      controls.push(prop(
        "set_line_width",
        "f32",
        self
          .playground
          .select(2, &STROKES, &stroke, move |value| set_stroke.set(value)),
      ));
    }
    controls.push(divider());
    controls.push(flag("set_shadow", &self.shadow));
    if !text && stroke > 0 {
      controls.push(flag("set_line_dash", &self.dashed));
    }
    controls.push(flag("software", &self.software));
    self.playground.card(
      ctx,
      Spec {
        title: "Canvas",
        make: Arc::new(build),
        shown: Shown::built(move |_| build()),
        fill: true,
        forceable: false,
        facts,
        event: "",
        code: shape_code(shape, fill, stroke, shadow, dashed, software),
        tree: vec![format!("canvas   {}", self.playground.measured())],
        controls,
        changed: [shape != 1, fill != 2, stroke != 2, !shadow, dashed, software]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset,
      },
    )
  }

  /// The canvas the pointer draws on, with the count of its strokes and a button that clears it.
  fn sketch(&self) -> Column {
    let look = Look::now();
    let ink_color = look.foreground;
    let ink = self.ink.clone();
    let painted = key(&look, ("sketch", self.cleared.get()));
    let point = |reference: &ElementRef, event: &MouseEvent| {
      reference
        .as_canvas()
        .and_then(|canvas| Some((canvas.point_from_window(event.x, event.y)?, canvas)))
    };
    let down = {
      let (reference, ink) = (self.sketch.clone(), self.ink.clone());
      move |event: MouseEvent| {
        let Some(((x, y), canvas)) = point(&reference, &event) else {
          return;
        };
        let mut ink = ink.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        ink.down = true;
        ink.strokes.push(vec![(x, y)]);
        // A dot where the press is: a line of no length, with round ends.
        let draw = canvas.context_2d();
        pen(&draw, ink_color);
        draw.begin_path();
        draw.move_to(x, y);
        draw.line_to(x + 0.01, y);
        draw.stroke();
      }
    };
    let moved = {
      let (reference, ink) = (self.sketch.clone(), self.ink.clone());
      move |event: MouseEvent| {
        let mut ink = ink.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if !ink.down {
          return;
        }
        let Some(((x, y), canvas)) = point(&reference, &event) else {
          return;
        };
        let Some(stroke) = ink.strokes.last_mut() else {
          return;
        };
        let last = *stroke.last().unwrap_or(&(x, y));
        stroke.push((x, y));
        // Only the new piece is drawn: what was drawn before stays on the surface.
        let draw = canvas.context_2d();
        pen(&draw, ink_color);
        draw.begin_path();
        draw.move_to(last.0, last.1);
        draw.line_to(x, y);
        draw.stroke();
      }
    };
    let lift = {
      let (ink, strokes) = (self.ink.clone(), self.strokes.clone());
      move || {
        let mut ink = ink.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if ink.down {
          ink.down = false;
          strokes.set(ink.strokes.len());
        }
      }
    };
    let (lift_up, lift_leave) = (lift.clone(), lift);
    let clear = {
      let (ink, strokes, cleared) = (self.ink.clone(), self.strokes.clone(), self.cleared.clone());
      move |_| {
        ink
          .lock()
          .unwrap_or_else(|poisoned| poisoned.into_inner())
          .strokes
          .clear();
        strokes.set(0);
        cleared.update(|count| *count += 1);
      }
    };
    let count = self.strokes.get();
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(
        controls()
          .align_items(Alignment::Center)
          .child(
            Button::new("Clear")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm)
              .disabled(count == 0)
              .on_click(clear),
          )
          .child(mono(&format!("{count} strokes"), 12.0).color(PaletteColor::MutedForeground)),
      )
      .child(
        well().child(
          Canvas::new()
            .ref_element(self.sketch.clone())
            .size(FILL, 260.0)
            .paint(painted, move |canvas| {
              paint_ink(
                canvas,
                &ink.lock().unwrap_or_else(|poisoned| poisoned.into_inner()),
                ink_color,
              )
            })
            .on_mouse_down(down)
            .on_mouse_move(moved)
            .on_mouse_up(move |_: MouseEvent| lift_up())
            .on_mouse_leave(move || lift_leave()),
        ),
      )
  }

  /// The chart that moves to new values at a press, whose bars a pointer finds by the items they are described as.
  fn moving(&self) -> Column {
    let look = Look::now();
    let seed = self.seed.get();
    let hovered_day = self.hovered.get();
    let hovered = DAYS.iter().position(|day| day.to_lowercase() == hovered_day);
    let values = runs(seed);
    let painted = key(&look, (seed, hovered));
    let shuffle = {
      let seed = self.seed.clone();
      move |_| seed.update(|seed| *seed += 1)
    };
    let hover = {
      let (reference, hovered) = (self.chart.clone(), self.hovered.clone());
      move |event: MouseEvent| {
        let Some(canvas) = reference.as_canvas() else {
          return;
        };
        let item = canvas
          .point_from_window(event.x, event.y)
          .and_then(|(x, y)| canvas.item_at(x, y))
          .filter(|item| item.role == "bar")
          .map(|item| item.id)
          .unwrap_or_default();
        if hovered.get_untracked() != item {
          hovered.set(item);
        }
      }
    };
    let leave = {
      let hovered = self.hovered.clone();
      move || {
        if !hovered.get_untracked().is_empty() {
          hovered.set(String::new());
        }
      }
    };
    let said = match hovered {
      Some(index) => format!("{}: {} runs", DAYS[index], values[index]),
      None => "Point at a bar".to_owned(),
    };
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(
        controls()
          .align_items(Alignment::Center)
          .child(
            Button::new("New values")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm)
              .icon("refresh")
              .on_click(shuffle),
          )
          .child(mono(&said, 12.0).color(PaletteColor::MutedForeground)),
      )
      .child(
        well().padding(12.0).child(
          Canvas::new()
            .ref_element(self.chart.clone())
            .id("engine-canvas-runs")
            .size(FILL, 200.0)
            .paint_moving(
              painted,
              CanvasMotion::new(Duration::from_millis(400)),
              seed as u64,
              Arc::new(values),
              move |canvas, at: CanvasMove<'_, [f32; 5]>| paint_bars(canvas, at, &look, hovered),
            )
            .on_mouse_move(hover)
            .on_mouse_leave(leave),
        ),
      )
  }
}

/// A canvas a section shows, as wide as its well and `height` high, drawn by `paint` in the theme's colors.
fn scene(height: f32, name: &'static str, paint: fn(&CanvasHandle, &Look)) -> Column {
  let look = Look::now();
  well().padding(12.0).child(
    Canvas::new()
      .size(FILL, height)
      .paint(key(&look, name), move |canvas| paint(canvas, &look)),
  )
}

impl Component for CanvasPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 3),
      shape: ctx.signal(1),
      fill: ctx.signal(2),
      stroke: ctx.signal(2),
      shadow: ctx.signal(true),
      dashed: ctx.signal(false),
      software: ctx.signal(false),
      sketch: ctx.element_ref(),
      ink: Arc::default(),
      strokes: ctx.signal(0),
      cleared: ctx.signal(0),
      chart: ctx.element_ref(),
      seed: ctx.signal(0),
      hovered: ctx.signal(String::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "new",
        "",
        "300 × 150",
        "A surface to draw on, as large as its box less its padding, at the display's scale. Its drawing stays \
         until it is drawn over; a new size clears it.",
      ),
      (
        "paint",
        "u64, Fn(&CanvasHandle)",
        "",
        "Draws the whole of the drawing when the canvas is first laid out, when its size or scale changes, and \
         when the key changes: the key is a hash of what the drawing is made from, the theme's colors among it.",
      ),
      (
        "paint_moving",
        "u64, CanvasMotion, u64, impl Into<Arc<T>>, Fn(&CanvasHandle, CanvasMove<T>)",
        "",
        "Draws as paint does, and moves to a value when its hash changes: the painter is given where it is on its \
         way, at each frame until it is there. Off where motion is reduced.",
      ),
      (
        "software",
        "()",
        "",
        "Drawn on the CPU by the reference renderer and uploaded whole: for tests and hosts with no GPU canvas.",
      ),
      (
        "ref_element",
        "ElementRef",
        "",
        "The ref whose as_canvas() is the canvas's handle once it is laid out: draw on it from a handler, a timer \
         or another thread.",
      ),
      (
        "point_from_window",
        "f32, f32",
        "",
        "On the handle: a pointer's place in the window as a point of the drawing, through padding and \
         transforms.",
      ),
      (
        "set_items / item_at",
        "Vec<CanvasItem> / f32, f32",
        "",
        "On the handle: what was drawn described as items, which a pointer is hit-tested against and MCP's tools \
         read.",
      ),
      (
        "CanvasHandle::offscreen",
        "f32, f32, f32",
        "",
        "A canvas in no window, drawn on the CPU: to_image_data() is its pixels. It draws no text.",
      ),
    ];
    let playground = self.play(ctx);
    let sketch = self.sketch();
    let moving = self.moving();
    let look = Look::now();
    let picture: Element = match offscreen(&look) {
      Some(image) => Row::new()
        .width(280.0)
        .max_width(FILL)
        .height(124.0)
        .background_image(image)
        .background_contain()
        .into(),
      None => quiet("CanvasHandle::offscreen gave no canvas.", 12.0).into(),
    };
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Canvas"],
        title: "Canvas",
        description: "A surface an app draws on with a 2D context, in the words of a browser's canvas: paths, \
                      fills and strokes, gradients, shadows, text and pictures. What is drawn stays until it is drawn \
                      over, and nothing is laid out or rendered again for it.",
        import: Some("use lurq::{canvas::*, components::Canvas};"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "A shape in the middle of a canvas as wide as the stage. Choose its path, its fill and its stroke, give it a \
           shadow, and draw it on the CPU in place of the GPU: it is drawn again only when one of these changes.",
          playground,
        ),
        Section::new(
          "Paths, fills and strokes",
          "A rectangle with a hole cleared in it, two that are see-through, stripes clipped to a circle, a curve with \
           round ends, a dashed line, a star filled by the even-odd rule, and a picture of two pixels drawn large \
           without smoothing.",
          self.frame.demo(
            0,
            scene(250.0, "paths", paint_paths),
            "draw.fill_rect(16.0, 16.0, 120.0, 72.0)",
            &[
              "let draw = canvas.context_2d();",
              "draw.set_fill_style(chart1);",
              "draw.fill_rect(16.0, 16.0, 120.0, 72.0);",
              "draw.clear_rect(44.0, 36.0, 24.0, 24.0);",
              "",
              "draw.save();",
              "draw.begin_path();",
              "draw.arc(360.0, 64.0, 48.0, 0.0, TAU, ArcDirection::Clockwise).ok();",
              "draw.clip();",
              "// .. stripes",
              "draw.restore();",
              "",
              "draw.set_line_cap(LineCap::Round);",
              "draw.begin_path();",
              "draw.move_to(24.0, 220.0);",
              "draw.bezier_curve_to(80.0, 110.0, 160.0, 280.0, 220.0, 150.0);",
              "draw.stroke();",
              "",
              "draw.set_line_dash(&[10.0, 6.0]);",
              "draw.fill_with_rule(FillRule::EvenOdd);",
              "draw.set_image_smoothing_enabled(false);",
              "draw.draw_image_scaled(&two_pixels, 368.0, 176.0, 96.0, 48.0).ok();",
            ],
          ),
        ),
        Section::new(
          "Gradients",
          "A gradient is given a box, and runs along a line turned in it, out from its middle, or around it. Its \
           stops are colors of the theme.",
          self.frame.demo(
            1,
            scene(140.0, "gradients", paint_gradients),
            "Gradient::linear().stop(0.0, chart1).stop(1.0, chart3)",
            &[
              "draw.set_fill_style(",
              "    Gradient::linear()",
              "        .stop(0.0, chart1)",
              "        .stop(1.0, chart3)",
              "        .rotation(TAU / 8.0)",
              "        .in_box(x, y, w, h),",
              ");",
              "Gradient::radial().stop(0.0, chart4).stop(1.0, chart2).in_box(x, y, w, h)",
              "Gradient::angular().stop(0.0, chart1).stop(0.25, chart2) /* .. */ .in_box(x, y, w, h)",
            ],
          ),
        ),
        Section::new(
          "Shadows, blur and layers",
          "A card with a shadow, a circle blurred, two circles the second of which multiplies what is under it, and \
           two pairs of squares at half: each drawn at half, so their overlap is darker, and both drawn into a \
           layer that is put down at half.",
          self.frame.demo(
            2,
            scene(250.0, "effects", paint_effects),
            "draw.begin_layer(0.5, BlendMode::Normal)",
            &[
              "draw.set_shadow(Some(Shadow::new(scrim).offset(0.0, 8.0).blur(16.0)));",
              "draw.set_filter(Filter::Blur(6.0));",
              "draw.set_global_composite_operation(BlendMode::Multiply);",
              "",
              "draw.set_global_alpha(0.5); // each square at half",
              "",
              "draw.begin_layer(0.5, BlendMode::Normal)?; // the pair at half",
              "draw.fill_rect(260.0, 140.0, 70.0, 70.0);",
              "draw.fill_rect(300.0, 170.0, 70.0, 70.0);",
              "draw.end_layer()?;",
            ],
          ),
        ),
        Section::new(
          "Text",
          "Text is shaped with the app's fonts and drawn on one line: aligned about a point, set on one of four \
           baselines, and measured.",
          self.frame.demo(
            3,
            scene(250.0, "text", paint_text),
            "draw.fill_text(\"Center\", 240.0, 68.0)",
            &[
              "draw.set_font(CanvasFont::new(\"\", 18.0));",
              "draw.set_text_align(TextAlign::Center);",
              "draw.fill_text(\"TextAlign::Center\", 240.0, 68.0)?;",
              "",
              "draw.set_text_baseline(TextBaseline::Middle);",
              "draw.fill_text(\"Middle\", 128.0, 160.0)?;",
              "",
              "let metrics = draw.measure_text(\"measure_text\")?;",
              "metrics.width",
            ],
          ),
        ),
        Section::new(
          "Draw with the pointer",
          "Press and draw. Each move draws only the new piece of the line through the canvas's handle: what was \
           drawn stays, and the page is not rendered again for it. The strokes are kept too, so the painter draws \
           them all again when the canvas changes size; Clear changes its key.",
          self.frame.demo(
            4,
            sketch,
            "canvas.point_from_window(event.x, event.y)",
            &[
              "let reference = ctx.element_ref();",
              "Canvas::new()",
              "    .ref_element(reference.clone())",
              "    .size(Dimension::Pct(100.0), 260.0)",
              "    .paint(key, move |canvas| paint_ink(canvas, &ink.lock().unwrap(), color))",
              "    .on_mouse_move(move |event: MouseEvent| {",
              "        let Some(canvas) = reference.as_canvas() else { return };",
              "        let Some((x, y)) = canvas.point_from_window(event.x, event.y) else { return };",
              "        let draw = canvas.context_2d();",
              "        draw.begin_path();",
              "        draw.move_to(last.0, last.1);",
              "        draw.line_to(x, y);",
              "        draw.stroke();",
              "    })",
            ],
          ),
        ),
        Section::new(
          "A drawing that moves",
          "A chart drawn with paint_moving: New values gives it others, and it is drawn at each frame on its way to \
           them, and not after. Each bar is described as an item, which the pointer finds: the bar under it is drawn \
           in another color, without starting the motion over.",
          self.frame.demo(
            5,
            moving,
            "Canvas::new().paint_moving(key, motion, of, values, paint)",
            &[
              "Canvas::new()",
              "    .size(Dimension::Pct(100.0), 200.0)",
              "    .paint_moving(",
              "        key, // the values, the bar pointed at, the theme",
              "        CanvasMotion::new(Duration::from_millis(400)),",
              "        seed, // tells the values apart",
              "        Arc::new(values),",
              "        move |canvas, at: CanvasMove<'_, [f32; 5]>| {",
              "            let from = at.from.map(|from| from[i]).unwrap_or(0.0);",
              "            let value = from + (at.to[i] - from) * at.progress;",
              "            // .. a bar of that height",
              "            canvas.set_items(items);",
              "        },",
              "    )",
            ],
          ),
        ),
        Section::new(
          "Off the screen",
          "A canvas in no window, drawn on the CPU, whose pixels are a picture to save or to send: here, the \
           background of a box.",
          self.frame.demo(
            6,
            picture,
            "CanvasHandle::offscreen(280.0, 124.0, 2.0)",
            &[
              "let canvas = CanvasHandle::offscreen(280.0, 124.0, 2.0)?;",
              "let draw = canvas.context_2d();",
              "draw.set_fill_style(Gradient::linear() /* .. */ .in_box(0.0, 0.0, 280.0, 124.0));",
              "draw.round_rect(0.0, 0.0, 280.0, 124.0, 16.0).ok();",
              "draw.fill();",
              "let pixels = canvas.to_image_data()?; // 560 × 248",
              "Row::new().size(280.0, 124.0).background_image(pixels)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a canvas is given, and what its handle does.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
