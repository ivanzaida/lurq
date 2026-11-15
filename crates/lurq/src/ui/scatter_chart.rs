//! A scatter chart: points by an x and a y on two value axes, of one series or of several, and the line of their
//! trend.

use std::sync::Arc;

use super::{
  chart::{
    ChartArea, ChartAxis, ChartFrame, ChartLook, ChartPlot, ChartPoint, ChartProps, ChartRenderer, ChartSeries,
    ChartSize, ChartTick, ChartTip, LinearScale, ValueTextFn, bits, chart_recipe, nearest, signature_of,
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
  canvas::{ArcDirection, CanvasItem, Context2D, LineCap},
  node::Element,
};

/// The parts of a scatter chart a recipe styles, besides its frame's ([`ChartPart`](super::ChartPart)).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScatterChartPart {
  /// A point: its opacity over its series' color.
  Point,
  /// The ring of the point under the pointer or the focus.
  Ring,
  /// The line of the trend: its color and its opacity.
  Trend,
}

names!(ScatterChartPart {
  Point => "point",
  Ring => "ring",
  Trend => "trend",
});

/// The states of a scatter chart a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScatterChartState {
  /// It draws the line of its trend.
  Trend,
}

names!(ScatterChartState {
  Trend => "trend",
});

/// A point as the board draws it: a dot of 8; the one under the pointer of 10, ringed 2 in the card's color.
const POINT_RADIUS: f32 = 4.0;
const ACTIVE_RADIUS: f32 = 5.0;
const RING: f32 = 2.0;
/// How far from a point the pointer finds it.
const REACH: f32 = 24.0;
/// How wide the line of the trend is.
const TREND_WIDTH: f32 = 1.0;

/// A scatter chart: the board's Scatter Chart (`s9y62`). Each series is points by an x and a y
/// ([`ChartSeries::points`]), drawn on a value axis across, written under the plot, and one up, its lines across the
/// plot; [`trend`](Self::trend) draws the straight line that fits the first series best.
///
/// ```
/// use lurq::ui::{ChartSeries, ScatterChart};
///
/// let chart = ScatterChart::new([
///   ChartSeries::points("Scenes", [(550.0, 4.2), (1600.0, 5.1), (3200.0, 7.8), (5000.0, 10.6), (7300.0, 14.2)]),
///   ChartSeries::points("Outlier", [(1850.0, 8.0), (6150.0, 15.0)]),
/// ])
/// .title("Nodes vs frame time")
/// .description("One dot per scene")
/// .trend(true)
/// .x_text(|nodes| format!("{}k", nodes / 1000.0));
/// ```
///
/// Mounted ([`mount`](Self::mount)), the point nearest the pointer has a ring and a tooltip of its x and its y; the
/// plot is one stop of Tab whose Left and Right go between the points of a series, in the order of their x, and Up and
/// Down between the series; a press, Enter or Space tells `on_select`. The legend is off unless told
/// ([`legend`](Self::legend)), as the board draws it.
#[derive(Clone)]
pub struct ScatterChart {
  series: Vec<ChartSeries>,
  trend: bool,
  x_text: Option<ValueTextFn>,
  props: ChartProps,
}

impl ScatterChart {
  /// A chart of `series`, each of points by an x and a y.
  pub fn new(series: impl IntoIterator<Item = ChartSeries>) -> Self {
    Self {
      series: series.into_iter().collect(),
      trend: false,
      x_text: None,
      props: ChartProps::default(),
    }
  }

  /// Whether the straight line that fits the first series shown best (by least squares) is drawn over the points,
  /// from its least x to its most: what the others stand apart from.
  pub fn trend(mut self, trend: bool) -> Self {
    self.trend = trend;
    self
  }

  /// How a number on the axis across is written, under the plot and in a tooltip: `2k`. As every other number unless
  /// told ([`value_text`](Self::value_text)).
  pub fn x_text(mut self, text: impl Fn(f64) -> String + Send + Sync + 'static) -> Self {
    self.x_text = Some(Arc::new(text));
    self
  }

  crate::chart_props!();

  /// The frame of the chart in `theme`: what `build` and `mount` render.
  fn frame(&self, theme: &Theme) -> ChartFrame {
    let plot = ScatterPlot::new(
      &self.series,
      self.series.iter().map(|series| !self.props.is_hidden(series)).collect(),
      self.trend,
      self.x_text.clone().unwrap_or_else(|| self.props.value_text()),
    );
    let empty = plot.is_empty();
    let case = self
      .props
      .case(ChartSize::Sm, empty)
      .state(ScatterChartState::Trend, self.trend);
    let recipe = theme.resolve::<ScatterChart>(&case, Some(&self.props.rules));
    // The board draws the lines across the plot at its ticks, and no legend.
    let mut props = self.props.clone();
    props.grid.get_or_insert(true);
    props.legend.get_or_insert(false);
    props.frame(
      recipe,
      Arc::new(plot),
      self.series.clone(),
      theme.renderer::<ScatterChart>(),
    )
  }

  /// Builds the still chart in `theme`: drawn and read by tooling, with nothing that follows the pointer or the keys.
  pub fn build(self, theme: &Theme) -> Element {
    self.frame(theme).build()
  }

  /// The chart mounted where it is declared: the ring and the tooltip of the point nearest the pointer or the focus,
  /// the arrows between points and series, and `on_select` told a press.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.props.key.clone();
    ChartFrame::mount(ctx, key, move |theme| self.frame(theme))
  }
}

impl From<ScatterChart> for Element {
  fn from(chart: ScatterChart) -> Self {
    chart.build(&current_theme())
  }
}

impl Themed for ScatterChart {
  const NAME: &'static str = "scatter_chart";
  type Renderer = ChartRenderer;

  /// The chart's frame ([`chart_recipe`]) at the board's size, `Sm`, and its points: dots of 8 in their series'
  /// colors at 0.85; the line of the trend 1 wide in the muted foreground at 0.6, over the points; the point under the
  /// pointer a dot of 10 ringed 2 in the card's color.
  fn recipe() -> Recipe {
    chart_recipe()
      .base(ScatterChartPart::Point, |s| s.opacity(0.85))
      .base(ScatterChartPart::Ring, |s| s.background(PaletteColor::Card))
      .base(ScatterChartPart::Trend, |s| {
        s.background(PaletteColor::MutedForeground).opacity(0.6)
      })
  }
}

/// What a scatter chart's plot draws: the points of the series shown, each series in the order of its x.
#[derive(Clone)]
struct ScatterPlot {
  names: Vec<Arc<str>>,
  /// Each series' points, in the order of their x: the order the arrows go through them.
  points: Vec<Arc<[(f64, f64)]>>,
  shown: Vec<bool>,
  trend: bool,
  x_text: ValueTextFn,
}

impl ScatterPlot {
  fn new(series: &[ChartSeries], shown: Vec<bool>, trend: bool, x_text: ValueTextFn) -> Self {
    let points = series
      .iter()
      .map(|series| {
        let mut points: Vec<(f64, f64)> = series
          .point_list()
          .iter()
          .copied()
          .filter(|(x, y)| x.is_finite() && y.is_finite())
          .collect();
        points.sort_by(|a, b| a.0.total_cmp(&b.0));
        Arc::from(points)
      })
      .collect();
    Self {
      names: series.iter().map(|series| Arc::from(series.name())).collect(),
      points,
      shown,
      trend,
      x_text,
    }
  }

  fn shown(&self) -> Vec<usize> {
    (0..self.points.len()).filter(|series| self.shown[*series]).collect()
  }

  fn all(&self) -> impl Iterator<Item = (usize, usize, (f64, f64))> + '_ {
    self.shown().into_iter().flat_map(move |series| {
      self.points[series]
        .iter()
        .enumerate()
        .map(move |(index, point)| (series, index, *point))
    })
  }

  /// The scale across, onto the plot's width, and the one up, onto its height.
  fn scales(&self, area: ChartArea) -> (LinearScale, LinearScale) {
    (
      LinearScale::nice(self.all().map(|(.., (x, _))| x), (0.0, area.width), 4),
      LinearScale::nice(self.all().map(|(.., (_, y))| y), (area.height, 0.0), 4),
    )
  }

  fn place(&self, area: ChartArea, point: (f64, f64)) -> (f32, f32) {
    let (across, up) = self.scales(area);
    (across.map(point.0), up.map(point.1))
  }

  /// The line that fits the first series shown best, from its least x to its most: where it starts and ends.
  fn trend_line(&self) -> Option<((f64, f64), (f64, f64))> {
    let series = *self.shown().first()?;
    let points = &self.points[series];
    let count = points.len() as f64;
    if points.len() < 2 {
      return None;
    }
    let (mean_x, mean_y) = (
      points.iter().map(|(x, _)| x).sum::<f64>() / count,
      points.iter().map(|(_, y)| y).sum::<f64>() / count,
    );
    let spread: f64 = points.iter().map(|(x, _)| (x - mean_x).powi(2)).sum();
    if spread <= f64::EPSILON {
      return None;
    }
    let slope = points.iter().map(|(x, y)| (x - mean_x) * (y - mean_y)).sum::<f64>() / spread;
    let at = |x: f64| mean_y + slope * (x - mean_x);
    let (first, last) = (points[0].0, points[points.len() - 1].0);
    Some(((first, at(first)), (last, at(last))))
  }
}

impl ChartPlot for ScatterPlot {
  fn paint(&self, canvas: &Context2D, area: ChartArea, look: &ChartLook, active: Option<ChartPoint>) {
    let opacity = match look.part(ScatterChartPart::Point).rest().opacity {
      Some(Opacity::Value(opacity)) => opacity,
      _ => 0.85,
    };
    for series in self.shown() {
      canvas.set_fill_style(look.series(series).with_opacity(opacity));
      for point in self.points[series].iter() {
        let (x, y) = self.place(area, *point);
        canvas.begin_path();
        let _ = canvas.arc(x, y, POINT_RADIUS, 0.0, std::f32::consts::TAU, ArcDirection::Clockwise);
        canvas.fill();
      }
    }
    // The trend over the points, as the board draws it.
    if self.trend
      && let Some((start, end)) = self.trend_line()
    {
      let ((x0, y0), (x1, y1)) = (self.place(area, start), self.place(area, end));
      canvas.set_stroke_style(look.color(ScatterChartPart::Trend));
      canvas.set_line_width(TREND_WIDTH);
      canvas.set_line_cap(LineCap::Butt);
      canvas.begin_path();
      canvas.move_to(x0, y0);
      canvas.line_to(x1, y1);
      canvas.stroke();
    }
    if let Some(point) = active
      && let Some(at) = self.points.get(point.series).and_then(|points| points.get(point.index))
    {
      let (x, y) = self.place(area, *at);
      canvas.begin_path();
      let _ = canvas.arc(x, y, ACTIVE_RADIUS, 0.0, std::f32::consts::TAU, ArcDirection::Clockwise);
      canvas.set_fill_style(look.color(ScatterChartPart::Ring));
      canvas.fill();
      canvas.begin_path();
      let _ = canvas.arc(
        x,
        y,
        ACTIVE_RADIUS - RING,
        0.0,
        std::f32::consts::TAU,
        ArcDirection::Clockwise,
      );
      canvas.set_fill_style(look.series(point.series));
      canvas.fill();
    }
  }

  fn ticks(&self, area: ChartArea, look: &ChartLook) -> Vec<ChartTick> {
    let (_, up) = self.scales(area);
    up.ticks()
      .into_iter()
      .map(|tick| ChartTick {
        position: up.map(tick),
        text: (look.value_text)(tick),
        baseline: tick == 0.0,
      })
      .collect()
  }

  fn axis(&self) -> ChartAxis {
    // The scale across starts and ends on its ticks, so its texts spread from the plot's start to its end stand at
    // their ticks.
    let across = LinearScale::nice(self.all().map(|(.., (x, _))| x), (0.0, 1.0), 4);
    ChartAxis::Spread(across.ticks().into_iter().map(|tick| (self.x_text)(tick)).collect())
  }

  fn point_at(&self, area: ChartArea, x: f32, y: f32) -> Option<ChartPoint> {
    nearest(
      self.all().map(|(series, index, point)| {
        let (px, py) = self.place(area, point);
        (ChartPoint::new(series, index), px, py)
      }),
      x,
      y,
      REACH,
      false,
    )
  }

  fn anchor(&self, area: ChartArea, point: ChartPoint) -> Option<(f32, f32)> {
    let at = self.points.get(point.series)?.get(point.index)?;
    Some(self.place(area, *at))
  }

  fn counts(&self) -> Vec<usize> {
    (0..self.points.len())
      .map(|series| {
        if self.shown[series] {
          self.points[series].len()
        } else {
          0
        }
      })
      .collect()
  }

  fn tip(&self, point: ChartPoint) -> ChartTip {
    let Some((x, y)) = self
      .points
      .get(point.series)
      .and_then(|points| points.get(point.index))
      .copied()
    else {
      return ChartTip::default();
    };
    ChartTip {
      title: (self.x_text)(x),
      rows: vec![(point.series, y)],
    }
  }

  fn items(&self, area: ChartArea, look: &ChartLook) -> Vec<CanvasItem> {
    self
      .all()
      .map(|(series, index, point)| {
        let (x, y) = self.place(area, point);
        CanvasItem::point(format!("point-{series}-{index}"), "point", x, y, POINT_RADIUS)
          .label(format!("{}, {}", self.names[series], (self.x_text)(point.0)))
          .value((look.value_text)(point.1))
      })
      .collect()
  }

  fn signature(&self) -> u64 {
    signature_of((
      self
        .points
        .iter()
        .map(|points| points.iter().map(|(x, y)| (bits(*x), bits(*y))).collect::<Vec<_>>())
        .collect::<Vec<_>>(),
      &self.shown,
      self.trend,
      (self.x_text)(1234.5),
    ))
  }
}
