//! A radar chart: a value for each of several axes round a center, a polygon for each series, to compare their
//! profiles.

use std::{
  f32::consts::{PI, TAU},
  hash::Hash,
  sync::Arc,
};

use super::{
  chart::{
    ChartArea, ChartFrame, ChartLabel, ChartLook, ChartMotion, ChartPart, ChartPlot, ChartPoint, ChartProps,
    ChartRenderer, ChartSeries, ChartSize, ChartTip, LinearScale, bits, chart_recipe, signature_of,
  },
  names,
  part_style::Opacity,
  recipe::Recipe,
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    ctx::Ctx,
    theme::{PaletteColor, Theme},
  },
  canvas::{ArcDirection, CanvasItem, Context2D, LineJoin},
  layout::layout_kind::Justify,
  node::Element,
};

/// The parts of a radar chart a recipe styles, besides its frame's ([`ChartPart`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RadarChartPart {
  /// The rings at the scale's ticks and the spokes of the axes: their color.
  Ring,
  /// A series' area and its edge, in its color: their opacity.
  Area,
  /// The area of the series under the pointer or the focus: its opacity; its edge is in the series' full color.
  ActiveArea,
}

names!(RadarChartPart {
  Ring => "ring",
  Area => "area",
  ActiveArea => "active_area",
});

/// How far the axes' names stand from the outer ring, and the room the plot keeps round it for them.
const NAME_GAP: f32 = 6.0;
const ROOM: f32 = 15.0;
/// The edge of a series' area, as the board draws it.
const EDGE: f32 = 1.5;
/// The dot at the point under the pointer or the focus.
const DOT: f32 = 3.5;

/// A radar chart: the board's Radar Chart (`cokgK`). Its axes stand round the center, the first at the top and the
/// others clockwise, each with its name; the rings are at the scale's round ticks, and each series is a polygon
/// through its values, filled and edged in its color.
///
/// ```
/// use lurq::ui::{ChartSeries, RadarChart};
///
/// let chart = RadarChart::new(
///   ["Draw", "Text", "Layout", "Memory", "Startup", "Input"],
///   [
///     ChartSeries::new("wgpu", [92.0, 70.0, 64.0, 58.0, 75.0, 66.0]),
///     ChartSeries::new("DX12", [68.0, 76.0, 70.0, 80.0, 62.0, 86.0]),
///   ],
/// )
/// .title("Renderer profile")
/// .description("Relative score per area");
/// ```
///
/// The plot is a square as high as the plot (150 in `Sm`, the board's, and 180 in `Md`), in the middle of the frame,
/// the names round it. Mounted ([`mount`](Self::mount)), the series and the axis under the pointer or the focus are
/// picked out with the tooltip of every series' value on that axis; the plot is one stop of Tab whose Left and
/// Right go round the axes and Up and Down between the series, and a press, Enter or Space tells `on_select`.
#[derive(Clone)]
pub struct RadarChart {
  axes: Arc<[Arc<str>]>,
  series: Vec<ChartSeries>,
  props: ChartProps,
}

impl RadarChart {
  /// A chart of `series`, each with a value for each of `axes`, in their order.
  pub fn new<S: AsRef<str>>(axes: impl IntoIterator<Item = S>, series: impl IntoIterator<Item = ChartSeries>) -> Self {
    Self {
      axes: axes.into_iter().map(|name| Arc::from(name.as_ref())).collect(),
      series: series.into_iter().collect(),
      props: ChartProps::default(),
    }
  }

  crate::chart_props!();

  /// The frame of the chart in `theme`: what `build` and `mount` render.
  fn frame(&self, theme: &Theme) -> ChartFrame {
    let mut plot = RadarPlot {
      axes: self.axes.clone(),
      values: self.series.iter().map(|series| Arc::from(series.values())).collect(),
      shown: self.series.iter().map(|series| !self.props.is_hidden(series)).collect(),
      names: self.series.iter().map(|series| Arc::from(series.name())).collect(),
      rings: false,
    };
    let case = self.props.case(ChartSize::Sm, plot.is_empty());
    let recipe = theme.resolve::<RadarChart>(&case, Some(&self.props.rules));
    plot.rings = self.props.value_axis.unwrap_or(false);
    let side = self.props.plot_height(&recipe);
    self
      .props
      .frame(
        recipe,
        Arc::new(plot),
        self.series.clone(),
        theme.renderer::<RadarChart>(),
      )
      .plot_width(side)
      .plot_justify(Justify::Center)
  }

  /// Builds the still chart in `theme`: drawn and read by tooling, with nothing that follows the pointer or the keys.
  pub fn build(self, theme: &Theme) -> Element {
    self.frame(theme).build()
  }

  /// The chart mounted where it is declared: the series and the axis under the pointer or the focus picked out with
  /// their tooltip, the arrows round the axes and between the series, and `on_select` told a press.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.props.key.clone();
    ChartFrame::mount(ctx, key, move |theme| self.frame(theme))
  }
}

impl From<RadarChart> for Element {
  fn from(chart: RadarChart) -> Self {
    chart.build(&current_theme())
  }
}

/// What a radar's plot draws: the rings, the spokes and a polygon for each series shown.
#[derive(Clone)]
struct RadarPlot {
  axes: Arc<[Arc<str>]>,
  values: Vec<Arc<[f64]>>,
  shown: Vec<bool>,
  names: Vec<Arc<str>>,
  /// Whether the rings' numbers are written.
  rings: bool,
}

impl RadarPlot {
  fn value(&self, series: usize, axis: usize) -> Option<f64> {
    self
      .values
      .get(series)
      .and_then(|values| values.get(axis))
      .copied()
      .filter(|value| value.is_finite())
  }

  fn shown(&self) -> Vec<usize> {
    (0..self.values.len()).filter(|series| self.shown[*series]).collect()
  }

  /// The center and the outer ring's radius.
  fn geometry(&self, area: ChartArea) -> (f32, f32, f32) {
    let side = area.width.min(area.height);
    (area.width / 2.0, area.height / 2.0, (side / 2.0 - ROOM).max(0.0))
  }

  /// The scale from the center to the outer ring: from zero, or the least value where one is under it, to a round
  /// number at or over the most, with about three rings.
  fn scale(&self, radius: f32) -> LinearScale {
    let values = self
      .shown()
      .into_iter()
      .flat_map(|series| (0..self.axes.len()).filter_map(move |axis| self.value(series, axis)))
      .collect::<Vec<_>>();
    LinearScale::nice(values, (0.0, radius), 3)
  }

  /// The direction of an axis: the first at the top, the others clockwise.
  fn angle(&self, axis: usize) -> f32 {
    -PI / 2.0 + axis as f32 / self.axes.len().max(1) as f32 * TAU
  }

  /// Where `value` stands on `axis`.
  fn at(&self, area: ChartArea, axis: usize, value: f64) -> (f32, f32) {
    let (cx, cy, radius) = self.geometry(area);
    let distance = self.scale(radius).map(value).clamp(0.0, radius);
    let angle = self.angle(axis);
    (cx + distance * angle.cos(), cy + distance * angle.sin())
  }

  /// Where the value of `series` on `axis` stands: at the center where it has none.
  fn vertex(&self, area: ChartArea, series: usize, axis: usize) -> (f32, f32) {
    let scale = self.scale(self.geometry(area).2);
    let least = scale.invert(0.0);
    self.at(area, axis, self.value(series, axis).unwrap_or(least))
  }

  /// A ring of `radius` round the center: a polygon through the axes, or a circle where there are fewer than three.
  fn ring(&self, canvas: &Context2D, (cx, cy): (f32, f32), radius: f32) {
    canvas.begin_path();
    let count = self.axes.len();
    if count < 3 {
      let _ = canvas.arc(cx, cy, radius, 0.0, TAU, ArcDirection::Clockwise);
    } else {
      for axis in 0..count {
        let angle = self.angle(axis);
        let (x, y) = (cx + radius * angle.cos(), cy + radius * angle.sin());
        if axis == 0 {
          canvas.move_to(x, y);
        } else {
          canvas.line_to(x, y);
        }
      }
    }
    canvas.close_path();
  }

  /// The path through `corners`.
  fn polygon(canvas: &Context2D, corners: &[(f32, f32)]) {
    canvas.begin_path();
    for (index, (x, y)) in corners.iter().enumerate() {
      if index == 0 {
        canvas.move_to(*x, *y);
      } else {
        canvas.line_to(*x, *y);
      }
    }
    canvas.close_path();
  }

  /// The corners of the polygon of `series`, at rest.
  fn corners(&self, area: ChartArea, series: usize) -> Vec<(f32, f32)> {
    (0..self.axes.len())
      .map(|axis| self.vertex(area, series, axis))
      .collect()
  }

  /// Draws the rings, the spokes, and each of `polygons` (a series and its corners, last over the others) filled and
  /// edged in its color, the one of `active` picked out with the dot at its corner.
  fn paint_polygons(
    &self,
    canvas: &Context2D,
    area: ChartArea,
    look: &ChartLook,
    active: Option<ChartPoint>,
    polygons: &[(usize, Vec<(f32, f32)>)],
  ) {
    let (cx, cy, radius) = self.geometry(area);
    if radius <= 0.0 {
      return;
    }
    let scale = self.scale(radius);
    canvas.set_line_width(1.0);
    canvas.set_line_join(LineJoin::Miter);
    canvas.set_stroke_style(look.color(RadarChartPart::Ring));
    for tick in scale.ticks() {
      let at = scale.map(tick);
      if at > 0.5 {
        self.ring(canvas, (cx, cy), at);
        canvas.stroke();
      }
    }
    for axis in 0..self.axes.len() {
      let angle = self.angle(axis);
      let on = active.is_some_and(|point| point.index == axis);
      canvas.set_stroke_style(if on {
        look.color(ChartPart::Cursor)
      } else {
        look.color(RadarChartPart::Ring)
      });
      canvas.begin_path();
      canvas.move_to(cx, cy);
      canvas.line_to(cx + radius * angle.cos(), cy + radius * angle.sin());
      canvas.stroke();
    }
    let (rest, lit) = (
      opacity(look, RadarChartPart::Area, 0.2),
      opacity(look, RadarChartPart::ActiveArea, 0.35),
    );
    canvas.set_line_width(EDGE);
    canvas.set_line_join(LineJoin::Round);
    for (series, corners) in polygons {
      let color = look.series(*series);
      let on = active.is_some_and(|point| point.series == *series);
      let alpha = color.a() as f32 / 255.0;
      Self::polygon(canvas, corners);
      canvas.set_fill_style(color.with_opacity(alpha * if on { lit } else { rest }));
      canvas.fill();
      canvas.set_stroke_style(if on { color } else { color.with_opacity(alpha * rest) });
      canvas.stroke();
    }
    if let Some(point) = active
      && let Some((_, corners)) = polygons.iter().find(|(series, _)| *series == point.series)
      && let Some((x, y)) = corners.get(point.index)
    {
      canvas.begin_path();
      let _ = canvas.arc(*x, *y, DOT, 0.0, TAU, ArcDirection::Clockwise);
      canvas.set_fill_style(look.series(point.series));
      canvas.fill();
      canvas.set_line_width(2.0);
      canvas.set_stroke_style(look.theme().palette_color(PaletteColor::Card));
      canvas.stroke();
    }
  }

  /// The series shown, the one of `active` last, over the others.
  fn order(&self, active: Option<ChartPoint>) -> Vec<usize> {
    let mut order = self.shown();
    if let Some(point) = active
      && let Some(place) = order.iter().position(|series| *series == point.series)
    {
      let series = order.remove(place);
      order.push(series);
    }
    order
  }
}

/// The opacity of a part at rest, or `or`.
fn opacity(look: &ChartLook, part: RadarChartPart, or: f32) -> f32 {
  match look.part(part).rest().opacity {
    Some(Opacity::Value(value)) => value,
    _ => or,
  }
}

impl ChartPlot for RadarPlot {
  fn paint(&self, canvas: &Context2D, area: ChartArea, look: &ChartLook, active: Option<ChartPoint>) {
    let polygons: Vec<(usize, Vec<(f32, f32)>)> = self
      .order(active)
      .into_iter()
      .map(|series| (series, self.corners(area, series)))
      .collect();
    self.paint_polygons(canvas, area, look, active, &polygons);
  }

  /// The polygons grow from the center as the chart first shows; on a change each corner moves from where it was to
  /// its new value, a series shown again grows from the center, and one hidden shrinks to it.
  fn paint_moving(
    &self,
    canvas: &Context2D,
    area: ChartArea,
    look: &ChartLook,
    active: Option<ChartPoint>,
    motion: ChartMotion<'_>,
  ) {
    if motion.at_rest() {
      return self.paint(canvas, area, look, active);
    }
    let (cx, cy, _) = self.geometry(area);
    let between = |from: (f32, f32), to: (f32, f32)| (motion.lerp(from.0, to.0), motion.lerp(from.1, to.1));
    let from = motion.from_as::<RadarPlot>();
    // Where a corner of `series` on `axis` was: its place in the plot before, or the center where it had none.
    let was = |series: usize, axis: usize| {
      from
        .filter(|from| from.shown.get(series).copied().unwrap_or(false) && axis < from.axes.len())
        .map_or((cx, cy), |from| from.vertex(area, series, axis))
    };
    let mut polygons: Vec<(usize, Vec<(f32, f32)>)> = Vec::new();
    if let Some(from) = from {
      // A series no longer shown shrinks to the center, under the others.
      for series in from.shown() {
        if self.shown.get(series).copied().unwrap_or(false) {
          continue;
        }
        let corners = (0..from.axes.len())
          .map(|axis| between(from.vertex(area, series, axis), (cx, cy)))
          .collect();
        polygons.push((series, corners));
      }
    }
    for series in self.order(active) {
      let corners = (0..self.axes.len())
        .map(|axis| between(was(series, axis), self.vertex(area, series, axis)))
        .collect();
      polygons.push((series, corners));
    }
    self.paint_polygons(canvas, area, look, active, &polygons);
  }

  fn point_at(&self, area: ChartArea, x: f32, y: f32) -> Option<ChartPoint> {
    let (cx, cy, radius) = self.geometry(area);
    let (dx, dy) = (x - cx, y - cy);
    if self.axes.is_empty() || (dx * dx + dy * dy).sqrt() > radius + ROOM {
      return None;
    }
    // The axis nearest the pointer's direction, and on it the series whose value is nearest.
    let mut turn = (dy.atan2(dx) + PI / 2.0).rem_euclid(TAU);
    let step = TAU / self.axes.len() as f32;
    turn += step / 2.0;
    let axis = ((turn / step) as usize) % self.axes.len();
    self
      .shown()
      .into_iter()
      .map(|series| {
        let (px, py) = self.vertex(area, series, axis);
        (series, (px - x).powi(2) + (py - y).powi(2))
      })
      .min_by(|a, b| a.1.total_cmp(&b.1))
      .map(|(series, _)| ChartPoint::new(series, axis))
  }

  fn anchor(&self, area: ChartArea, point: ChartPoint) -> Option<(f32, f32)> {
    (point.index < self.axes.len()).then(|| self.vertex(area, point.series, point.index))
  }

  fn counts(&self) -> Vec<usize> {
    (0..self.values.len())
      .map(|series| if self.shown[series] { self.axes.len() } else { 0 })
      .collect()
  }

  fn tip(&self, point: ChartPoint) -> ChartTip {
    ChartTip {
      title: self
        .axes
        .get(point.index)
        .map(|name| name.to_string())
        .unwrap_or_default(),
      rows: self
        .shown()
        .into_iter()
        .filter_map(|series| Some((series, self.value(series, point.index)?)))
        .collect(),
    }
  }

  fn items(&self, area: ChartArea, look: &ChartLook) -> Vec<CanvasItem> {
    let mut items = Vec::new();
    for series in self.shown() {
      for axis in 0..self.axes.len() {
        let Some(value) = self.value(series, axis) else {
          continue;
        };
        let (x, y) = self.vertex(area, series, axis);
        items.push(
          CanvasItem::point(format!("point-{series}-{axis}"), "point", x, y, 6.0)
            .label(format!("{}, {}", self.axes[axis], self.names[series]))
            .value((look.value_text)(value)),
        );
      }
    }
    items
  }

  fn labels(&self, area: ChartArea, look: &ChartLook) -> Vec<ChartLabel> {
    let (cx, cy, radius) = self.geometry(area);
    let mut labels: Vec<ChartLabel> = (0..self.axes.len())
      .map(|axis| {
        let angle = self.angle(axis);
        let (dx, dy) = (angle.cos(), angle.sin());
        let (x, y) = (cx + (radius + NAME_GAP) * dx, cy + (radius + NAME_GAP) * dy);
        // Away from the center: before the place on the left, after it on the right, over it at the top.
        let across = if dx > 0.3 {
          0.0
        } else if dx < -0.3 {
          -1.0
        } else {
          -0.5
        };
        let up = if dy < -0.3 {
          -1.0
        } else if dy > 0.3 {
          0.0
        } else {
          -0.5
        };
        ChartLabel::new(x, y, self.axes[axis].to_string(), (across, up))
      })
      .collect();
    if self.rings {
      // The rings' numbers beside the first axis, at its right, each under its ring, clear of the axis's name.
      let scale = self.scale(radius);
      for tick in scale.ticks() {
        let at = scale.map(tick);
        if at > 0.5 {
          labels.push(
            ChartLabel::new(cx + 3.0, cy - at + 1.0, (look.value_text)(tick), (0.0, 0.0)).part(ChartPart::ValueLabel),
          );
        }
      }
    }
    labels
  }

  fn signature(&self) -> u64 {
    signature_of((
      self.axes.iter().map(|name| name.to_string()).collect::<Vec<_>>(),
      self
        .values
        .iter()
        .map(|values| values.iter().map(|value| bits(*value)).collect::<Vec<_>>())
        .collect::<Vec<_>>(),
      &self.shown,
    ))
  }

  fn is_empty(&self) -> bool {
    self.axes.is_empty() || self.shown().iter().all(|series| self.values[*series].is_empty())
  }
}

impl Themed for RadarChart {
  const NAME: &'static str = "radar_chart";
  type Renderer = ChartRenderer;

  /// The chart's frame ([`chart_recipe`]) at the board's Radar Chart's size, `Sm`, its plot a square of 150 (180 in
  /// `Md`) in the middle of the frame: the rings and the spokes 1 in `Border`, the outer ring 15 inside the square;
  /// each series' area and its edge of 1.5 in its color at 0.2, the area of the series under the pointer or the
  /// focus at 0.35 with its edge in its full color, and the spoke of its axis in `Input`; the axes' names mono 11 in
  /// `MutedForeground`, 6 outside the outer ring.
  fn recipe() -> Recipe {
    chart_recipe()
      .base(ChartPart::Plot, |s| s.height(180.0))
      .size(ChartSize::Sm, ChartPart::Plot, |s| s.height(150.0))
      .base(RadarChartPart::Ring, |s| s.background(PaletteColor::Border))
      .base(RadarChartPart::Area, |s| s.opacity(0.2))
      .base(RadarChartPart::ActiveArea, |s| s.opacity(0.35))
  }
}
