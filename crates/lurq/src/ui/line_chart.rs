//! A line chart and an area chart: a value for each category, joined by a line from the first to the last, with the
//! fill under it where it is an area, of one series or of several, overlaid or stacked.

use std::sync::Arc;

use super::{
  chart::{
    ChartArea, ChartAxis, ChartFrame, ChartLook, ChartPart, ChartPlot, ChartPoint, ChartProps, ChartRenderer,
    ChartSeries, ChartSize, ChartTick, ChartTip, LinearScale, TipPlace, bits, chart_recipe, signature_of,
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
  canvas::{ArcDirection, CanvasItem, Context2D, LineCap, LineJoin},
  node::{Element, color::Color},
};

/// The parts of a line chart and of an area chart a recipe styles, besides their frame's
/// ([`ChartPart`](super::ChartPart)).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LineChartPart {
  /// A series' line: its opacity. Its color is the series'.
  Line,
  /// The fill under a series' line in an area chart: its opacity over the series' color.
  Area,
  /// The mark of a series' value at the place under the pointer or the focus: the color of its ring, which the
  /// series' color fills.
  Point,
}

names!(LineChartPart {
  Line => "line",
  Area => "area",
  Point => "point",
});

/// The states of a line chart and of an area chart a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LineChartState {
  /// Its lines are smoothed.
  Smooth,
  /// Its areas are stacked, one on another.
  Stacked,
}

names!(LineChartState {
  Smooth => "smooth",
  Stacked => "stacked",
});

/// How a line goes from one value to the next.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LineCurve {
  /// Straight, as the board draws it.
  #[default]
  Straight,
  /// A smooth curve through every value that never goes past the values on either side of it: a rise stays a rise.
  Smooth,
}

/// How wide a line is, as the board draws it.
const LINE_WIDTH: f32 = 2.0;
/// The mark at the place under the pointer: a dot of 10 in a series' color, ringed 2 in the card's.
const POINT_RADIUS: f32 = 5.0;
const POINT_RING: f32 = 2.0;
/// The most category names written under the plot where the chart is given no texts for its axis.
const AXIS_TEXTS: usize = 8;

/// What a line chart and an area chart are made of.
#[derive(Clone)]
struct Lines {
  categories: Arc<[Arc<str>]>,
  series: Vec<ChartSeries>,
  curve: LineCurve,
  axis: Option<Vec<String>>,
}

impl Lines {
  fn new<S: AsRef<str>>(
    categories: impl IntoIterator<Item = S>,
    series: impl IntoIterator<Item = ChartSeries>,
  ) -> Self {
    Self {
      categories: categories.into_iter().map(|name| Arc::from(name.as_ref())).collect(),
      series: series.into_iter().collect(),
      curve: LineCurve::Straight,
      axis: None,
    }
  }

  /// The frame of the chart `C` in `theme`, with an area under each line where `area` is, stacked where `stacked` is.
  fn frame<C: Themed<Renderer = ChartRenderer>>(
    &self,
    props: &ChartProps,
    theme: &Theme,
    area: bool,
    stacked: bool,
  ) -> ChartFrame {
    let plot = LinePlot {
      categories: self.categories.clone(),
      names: self.series.iter().map(|series| Arc::from(series.name())).collect(),
      values: self.series.iter().map(|series| Arc::from(series.values())).collect(),
      shown: self.series.iter().map(|series| !props.is_hidden(series)).collect(),
      curve: self.curve,
      area,
      stacked: area && stacked,
      axis: self.axis.clone(),
    };
    let empty = plot.is_empty();
    let case = props
      .case(ChartSize::Sm, empty)
      .state(LineChartState::Smooth, self.curve == LineCurve::Smooth)
      .state(LineChartState::Stacked, plot.stacked);
    let recipe = theme.resolve::<C>(&case, Some(&props.rules));
    // The board draws the lines across the plot at its ticks.
    let mut props = props.clone();
    props.grid.get_or_insert(true);
    props.frame(recipe, Arc::new(plot), self.series.clone(), theme.renderer::<C>())
  }
}

/// A line chart: the board's Line Chart (`ufn5w`). Each series is a line through its values, one for each category,
/// from the plot's start to its end; a value that is missing (not a finite number) leaves a gap in its line.
///
/// ```
/// use lurq::ui::{ChartSeries, LineChart};
///
/// let chart = LineChart::new(
///   ["0.54", "0.55", "0.56", "0.57", "0.58", "0.59", "0.60"],
///   [
///     ChartSeries::new("wgpu", [10.4, 9.6, 9.9, 8.2, 8.6, 6.9, 6.3]),
///     ChartSeries::new("DirectX 12", [11.6, 11.2, 10.3, 10.6, 9.4, 9.4, 8.6]),
///   ],
/// )
/// .title("Frame time")
/// .description("Milliseconds per frame, last 7 builds")
/// .value_text(|ms| format!("{ms} ms"));
/// ```
///
/// Mounted ([`mount`](Self::mount)), the place under the pointer has a line across the plot and a mark on each
/// series, and a tooltip beside it with every series' value there; the plot is one stop of Tab whose Left and Right
/// go between the categories and Up and Down between the series; a press, Enter or Space tells `on_select`. A line of
/// ten thousand values is drawn with no mark of its own for each, and the pointer finds its place at once.
#[derive(Clone)]
pub struct LineChart {
  lines: Lines,
  props: ChartProps,
}

impl LineChart {
  /// A chart of `series`, each with a value for each of `categories`, in their order.
  pub fn new<S: AsRef<str>>(
    categories: impl IntoIterator<Item = S>,
    series: impl IntoIterator<Item = ChartSeries>,
  ) -> Self {
    Self {
      lines: Lines::new(categories, series),
      props: ChartProps::default(),
    }
  }

  /// How a line goes from one value to the next: straight, as the board draws it, or smooth.
  pub fn curve(mut self, curve: LineCurve) -> Self {
    self.lines.curve = curve;
    self
  }

  /// The texts under the plot, spread from its start to its end, in place of the categories' names: an axis of its
  /// own, `0 min`, `10`, `20`. Without them the chart writes the names of the categories, at most eight of them.
  pub fn axis_labels<S: AsRef<str>>(mut self, labels: impl IntoIterator<Item = S>) -> Self {
    self.lines.axis = Some(labels.into_iter().map(|label| label.as_ref().to_owned()).collect());
    self
  }

  crate::chart_props!();

  /// Builds the still chart in `theme`: drawn and read by tooling, with nothing that follows the pointer or the keys.
  pub fn build(self, theme: &Theme) -> Element {
    self.lines.frame::<LineChart>(&self.props, theme, false, false).build()
  }

  /// The chart mounted where it is declared: the cursor, the marks and the tooltip of the place under the pointer or
  /// the focus, the arrows between categories and series, and `on_select` told a press.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.props.key.clone();
    ChartFrame::mount(ctx, key, move |theme| {
      self.lines.frame::<LineChart>(&self.props, theme, false, false)
    })
  }
}

impl From<LineChart> for Element {
  fn from(chart: LineChart) -> Self {
    chart.build(&current_theme())
  }
}

impl Themed for LineChart {
  const NAME: &'static str = "line_chart";
  type Renderer = ChartRenderer;

  /// The chart's frame ([`chart_recipe`]) at the board's size, `Sm`, and its lines: 2 wide with round joins and ends
  /// in the series' colors; at the place under the pointer the frame's cursor, a line of 1 in the input's color, and
  /// a mark of 10 in each series' color ringed 2 in the card's color.
  fn recipe() -> Recipe {
    lines_recipe()
  }
}

/// An area chart: the board's Area Chart (`yTmhJ`). A line chart whose lines have the fill under them, down to zero,
/// in their colors at 0.16; several areas overlay one another, or stand one on another where they are stacked
/// ([`stacked`](Self::stacked)), each from the line of the one under it.
///
/// ```
/// use lurq::ui::{AreaChart, ChartSeries};
///
/// let chart = AreaChart::new(
///   ["0", "5", "10", "15", "20", "25", "30", "35"],
///   [ChartSeries::new("Resident", [380.0, 520.0, 470.0, 720.0, 670.0, 920.0, 870.0, 1060.0])],
/// )
/// .title("Memory")
/// .description("Megabytes resident during a session")
/// .axis_labels(["0 min", "10", "20", "30", "40"]);
/// ```
///
/// It has the line chart's tooltip, keys and gaps; a stacked series' missing value counts as none.
#[derive(Clone)]
pub struct AreaChart {
  lines: Lines,
  stacked: bool,
  props: ChartProps,
}

impl AreaChart {
  /// A chart of `series`, each with a value for each of `categories`, in their order.
  pub fn new<S: AsRef<str>>(
    categories: impl IntoIterator<Item = S>,
    series: impl IntoIterator<Item = ChartSeries>,
  ) -> Self {
    Self {
      lines: Lines::new(categories, series),
      stacked: false,
      props: ChartProps::default(),
    }
  }

  /// Whether the areas are stacked, one on another, the first at the bottom: a whole and its parts. Overlaid unless
  /// told.
  pub fn stacked(mut self, stacked: bool) -> Self {
    self.stacked = stacked;
    self
  }

  /// How a line goes from one value to the next: straight, as the board draws it, or smooth.
  pub fn curve(mut self, curve: LineCurve) -> Self {
    self.lines.curve = curve;
    self
  }

  /// The texts under the plot, spread from its start to its end, in place of the categories' names.
  pub fn axis_labels<S: AsRef<str>>(mut self, labels: impl IntoIterator<Item = S>) -> Self {
    self.lines.axis = Some(labels.into_iter().map(|label| label.as_ref().to_owned()).collect());
    self
  }

  crate::chart_props!();

  /// Builds the still chart in `theme`: drawn and read by tooling, with nothing that follows the pointer or the keys.
  pub fn build(self, theme: &Theme) -> Element {
    self
      .lines
      .frame::<AreaChart>(&self.props, theme, true, self.stacked)
      .build()
  }

  /// The chart mounted where it is declared: the line chart's cursor, marks, tooltip and keys.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.props.key.clone();
    ChartFrame::mount(ctx, key, move |theme| {
      self.lines.frame::<AreaChart>(&self.props, theme, true, self.stacked)
    })
  }
}

impl From<AreaChart> for Element {
  fn from(chart: AreaChart) -> Self {
    chart.build(&current_theme())
  }
}

impl Themed for AreaChart {
  const NAME: &'static str = "area_chart";
  type Renderer = ChartRenderer;

  /// The line chart's recipe, with the fill under each line in its series' color at 0.16, as the board's Area Chart
  /// has it. Its parts and states are the line chart's (`LineChartPart`, `LineChartState`).
  fn recipe() -> Recipe {
    lines_recipe()
  }
}

/// The recipe of a line chart and of an area chart.
fn lines_recipe() -> Recipe {
  chart_recipe()
    .base(LineChartPart::Line, |s| s.opacity(1.0))
    .base(LineChartPart::Area, |s| s.opacity(0.16))
    .base(LineChartPart::Point, |s| s.background(PaletteColor::Card))
}

/// What a line chart's plot draws: the values of the series shown, by category.
#[derive(Clone)]
struct LinePlot {
  categories: Arc<[Arc<str>]>,
  names: Vec<Arc<str>>,
  values: Vec<Arc<[f64]>>,
  shown: Vec<bool>,
  curve: LineCurve,
  area: bool,
  stacked: bool,
  axis: Option<Vec<String>>,
}

/// A piece of a line from one value to the next: where it starts, its two control points, and where it ends.
type Piece = ((f32, f32), (f32, f32), (f32, f32), (f32, f32));

impl LinePlot {
  fn shown(&self) -> Vec<usize> {
    (0..self.values.len()).filter(|series| self.shown[*series]).collect()
  }

  fn value(&self, series: usize, index: usize) -> Option<f64> {
    self
      .values
      .get(series)
      .and_then(|values| values.get(index))
      .copied()
      .filter(|value| value.is_finite())
  }

  /// Where a series' line is at `index`: its own value, or in a stack the values of the series shown under it and its
  /// own. A stacked series has a place wherever the stack has; one that is not has none where its value is missing.
  fn level(&self, series: usize, index: usize) -> Option<f64> {
    if !self.stacked {
      return self.value(series, index);
    }
    let own = self.value(series, index);
    let below: f64 = self
      .shown()
      .into_iter()
      .take_while(|other| *other != series)
      .filter_map(|other| self.value(other, index))
      .sum();
    Some(below + own.unwrap_or(0.0))
  }

  /// The level of the series under `series` in a stack: zero for the first.
  fn floor(&self, series: usize, index: usize) -> f64 {
    let shown = self.shown();
    match shown.iter().position(|other| *other == series) {
      Some(0) | None => 0.0,
      Some(place) => self.level(shown[place - 1], index).unwrap_or(0.0),
    }
  }

  fn scale(&self, area: ChartArea) -> LinearScale {
    let shown = self.shown();
    let levels = (0..self.categories.len())
      .flat_map(|index| shown.iter().filter_map(move |series| self.level(*series, index)))
      .collect::<Vec<_>>();
    LinearScale::nice(levels, (area.height, 0.0), 4)
  }

  /// The x of the category at `index`: the first at the plot's start and the last at its end, one alone in its
  /// middle.
  fn x(&self, area: ChartArea, index: usize) -> f32 {
    match self.categories.len() {
      0 | 1 => area.width / 2.0,
      count => area.width * index as f32 / (count - 1) as f32,
    }
  }

  /// The category nearest to `x`: a line of ten thousand values finds it at once, its categories being evenly apart.
  fn index_at(&self, area: ChartArea, x: f32) -> Option<usize> {
    match self.categories.len() {
      0 => None,
      1 => Some(0),
      count => {
        let step = area.width / (count - 1) as f32;
        Some(((x / step).round().max(0.0) as usize).min(count - 1))
      }
    }
  }

  /// The runs of a series' line with no gap in them, each a list of points, by `place` (a series' level, or the
  /// level under it).
  fn runs(&self, area: ChartArea, scale: &LinearScale, place: impl Fn(usize) -> Option<f64>) -> Vec<Vec<(f32, f32)>> {
    let mut runs = Vec::new();
    let mut run = Vec::new();
    for index in 0..self.categories.len() {
      match place(index) {
        Some(value) => run.push((self.x(area, index), scale.map(value))),
        None if !run.is_empty() => runs.push(std::mem::take(&mut run)),
        None => {}
      }
    }
    if !run.is_empty() {
      runs.push(run);
    }
    runs
  }

  /// The pieces of a line through `points`, straight or smooth.
  fn pieces(&self, points: &[(f32, f32)]) -> Vec<Piece> {
    match self.curve {
      LineCurve::Straight => points
        .windows(2)
        .map(|pair| (pair[0], pair[0], pair[1], pair[1]))
        .collect(),
      LineCurve::Smooth => monotone(points),
    }
  }

  /// Traces the pieces from their start: a line, or the top of an area.
  fn trace(canvas: &Context2D, pieces: &[Piece], first: (f32, f32), move_first: bool) {
    if move_first {
      canvas.move_to(first.0, first.1);
    } else {
      canvas.line_to(first.0, first.1);
    }
    for (_, c1, c2, end) in pieces {
      canvas.bezier_curve_to(c1.0, c1.1, c2.0, c2.1, end.0, end.1);
    }
  }

  /// Traces the pieces back from their end: the bottom of a stacked area.
  fn trace_back(canvas: &Context2D, pieces: &[Piece], last: (f32, f32)) {
    canvas.line_to(last.0, last.1);
    for (start, c1, c2, _) in pieces.iter().rev() {
      canvas.bezier_curve_to(c2.0, c2.1, c1.0, c1.1, start.0, start.1);
    }
  }

  fn paint_area(&self, canvas: &Context2D, area: ChartArea, scale: &LinearScale, series: usize, color: Color) {
    canvas.set_fill_style(color);
    let zero = scale.map(0.0);
    if self.stacked {
      let top: Vec<(f32, f32)> = (0..self.categories.len())
        .map(|index| (self.x(area, index), scale.map(self.level(series, index).unwrap_or(0.0))))
        .collect();
      let bottom: Vec<(f32, f32)> = (0..self.categories.len())
        .map(|index| (self.x(area, index), scale.map(self.floor(series, index))))
        .collect();
      let (Some(first), Some(last)) = (top.first(), bottom.last()) else {
        return;
      };
      canvas.begin_path();
      Self::trace(canvas, &self.pieces(&top), *first, true);
      Self::trace_back(canvas, &self.pieces(&bottom), *last);
      canvas.close_path();
      canvas.fill();
      return;
    }
    for run in self.runs(area, scale, |index| self.value(series, index)) {
      let (first, last) = (run[0], run[run.len() - 1]);
      canvas.begin_path();
      Self::trace(canvas, &self.pieces(&run), first, true);
      canvas.line_to(last.0, zero);
      canvas.line_to(first.0, zero);
      canvas.close_path();
      canvas.fill();
    }
  }

  fn paint_line(&self, canvas: &Context2D, area: ChartArea, scale: &LinearScale, series: usize, color: Color) {
    canvas.set_stroke_style(color);
    canvas.set_line_width(LINE_WIDTH);
    canvas.set_line_join(LineJoin::Round);
    canvas.set_line_cap(LineCap::Round);
    for run in self.runs(area, scale, |index| self.level(series, index)) {
      canvas.begin_path();
      if run.len() == 1 {
        // A value with none beside it is a dot as wide as the line.
        let _ = canvas.arc(
          run[0].0,
          run[0].1,
          LINE_WIDTH,
          0.0,
          std::f32::consts::TAU,
          ArcDirection::Clockwise,
        );
        canvas.set_fill_style(color);
        canvas.fill();
        continue;
      }
      Self::trace(canvas, &self.pieces(&run), run[0], true);
      canvas.stroke();
    }
  }
}

/// The pieces of a smooth line through `points`, which never goes past the values on either side of a point: the
/// monotone cubic of Fritsch and Carlson.
fn monotone(points: &[(f32, f32)]) -> Vec<Piece> {
  let count = points.len();
  if count < 2 {
    return Vec::new();
  }
  let slopes: Vec<f32> = points
    .windows(2)
    .map(|pair| {
      let dx = pair[1].0 - pair[0].0;
      if dx.abs() < f32::EPSILON {
        0.0
      } else {
        (pair[1].1 - pair[0].1) / dx
      }
    })
    .collect();
  let mut tangents = vec![0.0; count];
  tangents[0] = slopes[0];
  tangents[count - 1] = slopes[count - 2];
  for index in 1..count - 1 {
    let (before, after) = (slopes[index - 1], slopes[index]);
    tangents[index] = if before * after <= 0.0 {
      0.0
    } else {
      (before + after) / 2.0
    };
  }
  for index in 0..count - 1 {
    let slope = slopes[index];
    if slope.abs() < f32::EPSILON {
      tangents[index] = 0.0;
      tangents[index + 1] = 0.0;
      continue;
    }
    let (a, b) = (tangents[index] / slope, tangents[index + 1] / slope);
    let length = a * a + b * b;
    if length > 9.0 {
      let t = 3.0 / length.sqrt();
      tangents[index] = t * a * slope;
      tangents[index + 1] = t * b * slope;
    }
  }
  points
    .windows(2)
    .enumerate()
    .map(|(index, pair)| {
      let (start, end) = (pair[0], pair[1]);
      let third = (end.0 - start.0) / 3.0;
      (
        start,
        (start.0 + third, start.1 + tangents[index] * third),
        (end.0 - third, end.1 - tangents[index + 1] * third),
        end,
      )
    })
    .collect()
}

impl ChartPlot for LinePlot {
  fn paint(&self, canvas: &Context2D, area: ChartArea, look: &ChartLook, active: Option<ChartPoint>) {
    let scale = self.scale(area);
    let shown = self.shown();
    if self.area {
      let opacity = match look.part(LineChartPart::Area).rest().opacity {
        Some(Opacity::Value(opacity)) => opacity,
        _ => 0.16,
      };
      for series in &shown {
        self.paint_area(
          canvas,
          area,
          &scale,
          *series,
          look.series(*series).with_opacity(opacity),
        );
      }
    }
    // The first series is drawn last, over the others, as the board draws it.
    let line = match look.part(LineChartPart::Line).rest().opacity {
      Some(Opacity::Value(opacity)) => opacity,
      _ => 1.0,
    };
    for series in shown.iter().rev() {
      self.paint_line(canvas, area, &scale, *series, look.series(*series).with_opacity(line));
    }
    let Some(point) = active else {
      return;
    };
    let x = self.x(area, point.index);
    canvas.set_fill_style(look.color(ChartPart::Cursor));
    canvas.fill_rect(x - 0.5, 0.0, 1.0, area.height);
    let ring = look.color(LineChartPart::Point);
    for series in shown.iter().rev() {
      let Some(level) = self
        .level(*series, point.index)
        .filter(|_| self.value(*series, point.index).is_some())
      else {
        continue;
      };
      let y = scale.map(level);
      canvas.begin_path();
      let _ = canvas.arc(x, y, POINT_RADIUS, 0.0, std::f32::consts::TAU, ArcDirection::Clockwise);
      canvas.set_fill_style(ring);
      canvas.fill();
      canvas.begin_path();
      let _ = canvas.arc(
        x,
        y,
        POINT_RADIUS - POINT_RING,
        0.0,
        std::f32::consts::TAU,
        ArcDirection::Clockwise,
      );
      canvas.set_fill_style(look.series(*series));
      canvas.fill();
    }
  }

  fn ticks(&self, area: ChartArea, look: &ChartLook) -> Vec<ChartTick> {
    let scale = self.scale(area);
    scale
      .ticks()
      .into_iter()
      .map(|tick| ChartTick {
        position: scale.map(tick),
        text: (look.value_text)(tick),
        baseline: tick == 0.0,
      })
      .collect()
  }

  fn axis(&self) -> ChartAxis {
    if let Some(labels) = &self.axis {
      return ChartAxis::Spread(labels.clone());
    }
    let count = self.categories.len();
    if count <= AXIS_TEXTS {
      return ChartAxis::Spread(self.categories.iter().map(|name| name.to_string()).collect());
    }
    // Of many categories, eight evenly apart, the first and the last among them.
    ChartAxis::Spread(
      (0..AXIS_TEXTS)
        .map(|place| self.categories[place * (count - 1) / (AXIS_TEXTS - 1)].to_string())
        .collect(),
    )
  }

  fn point_at(&self, area: ChartArea, _x: f32, y: f32) -> Option<ChartPoint> {
    let index = self.index_at(area, _x)?;
    let scale = self.scale(area);
    // The series whose value at that place is nearest the pointer.
    self
      .shown()
      .into_iter()
      .filter(|series| self.value(*series, index).is_some())
      .filter_map(|series| Some((series, (scale.map(self.level(series, index)?) - y).abs())))
      .min_by(|a, b| a.1.total_cmp(&b.1))
      .map(|(series, _)| ChartPoint::new(series, index))
  }

  fn anchor(&self, area: ChartArea, point: ChartPoint) -> Option<(f32, f32)> {
    let level = self.level(point.series, point.index)?;
    Some((self.x(area, point.index), self.scale(area).map(level)))
  }

  fn tip_place(&self) -> TipPlace {
    TipPlace::Beside
  }

  fn counts(&self) -> Vec<usize> {
    (0..self.values.len())
      .map(|series| if self.shown[series] { self.categories.len() } else { 0 })
      .collect()
  }

  fn tip(&self, point: ChartPoint) -> ChartTip {
    ChartTip {
      title: self
        .categories
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
    let scale = self.scale(area);
    let several = self.values.len() > 1;
    let mut items = Vec::new();
    for series in self.shown() {
      for index in 0..self.categories.len() {
        let (Some(value), Some(level)) = (self.value(series, index), self.level(series, index)) else {
          continue;
        };
        let name = &self.categories[index];
        let label = if several {
          format!("{name}, {}", self.names[series])
        } else {
          name.to_string()
        };
        items.push(
          CanvasItem::point(
            format!("point-{series}-{index}"),
            "point",
            self.x(area, index),
            scale.map(level),
            POINT_RADIUS,
          )
          .label(label)
          .value((look.value_text)(value)),
        );
      }
    }
    items
  }

  fn signature(&self) -> u64 {
    signature_of((
      self.categories.iter().map(|name| name.to_string()).collect::<Vec<_>>(),
      self
        .values
        .iter()
        .map(|values| values.iter().map(|value| bits(*value)).collect::<Vec<_>>())
        .collect::<Vec<_>>(),
      &self.shown,
      self.curve,
      self.area,
      self.stacked,
      &self.axis,
    ))
  }

  fn is_empty(&self) -> bool {
    self.categories.is_empty()
      || self
        .shown()
        .iter()
        .all(|series| self.values[*series].iter().all(|value| !value.is_finite()))
  }
}
