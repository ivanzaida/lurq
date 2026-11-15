//! A bar chart: a value for each category, as a bar from zero, of one series or of several side by side or stacked.

use std::{hash::Hash, sync::Arc};

use super::{
  chart::{
    BandScale, BarEnd, ChartArea, ChartAxis, ChartFrame, ChartLook, ChartPlot, ChartPoint, ChartProps, ChartRenderer,
    ChartSeries, ChartSize, ChartTick, ChartTip, LinearScale, bits, chart_recipe, fill_bar, signature_of,
  },
  names,
  recipe::Recipe,
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    ctx::Ctx,
    theme::{PaletteColor, Theme},
  },
  canvas::{CanvasItem, Context2D},
  node::Element,
};

/// The parts of a bar chart a recipe styles, besides its frame's ([`ChartPart`](super::ChartPart)).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BarChartPart {
  /// A bar where the chart has a current category: the color of the others, which give way to it.
  Bar,
  /// The band behind the category under the pointer or the focus.
  Band,
}

names!(BarChartPart {
  Bar => "bar",
  Band => "band",
});

/// The states of a bar chart a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BarChartState {
  /// Its series are stacked, one on another in each category.
  Stacked,
}

names!(BarChartState {
  Stacked => "stacked",
});

/// The corners of a bar's end, as the board rounds them.
const BAR_RADIUS: f32 = 4.0;
/// The room between the bars of one category, where its series stand side by side.
const GROUP_GAP: f32 = 4.0;
/// The room between the segments of a stack, where the card shows through.
const SEGMENT_GAP: f32 = 1.0;

/// A bar chart: the board's Bar Chart (`Sk5h8`) and its stacked form (`jJ85X`). A category's value is a bar from
/// zero, up for a value over it and down for one under; several series stand side by side in each category, or one
/// on another where they are stacked ([`stacked`](Self::stacked)).
///
/// ```
/// use lurq::ui::{BarChart, ChartSeries};
///
/// let chart = BarChart::new(
///   ["0.55", "0.56", "0.57", "0.58", "0.59", "0.60"],
///   [ChartSeries::new("Frame time", [11.8, 13.2, 9.6, 8.4, 7.0, 5.2])],
/// )
/// .title("Frame time")
/// .description("Median per build, last six releases")
/// .current(5)
/// .value_text(|ms| format!("{ms} ms"));
/// ```
///
/// Each series takes the theme's next chart color unless it is given one ([`ChartSeries::color`]). With a current
/// category ([`current`](Self::current)) the other bars are in the input's color and give way to it, as the board
/// draws the latest release. Mounted ([`mount`](Self::mount)), the bar under the pointer has its tooltip, the plot is
/// one stop of Tab whose arrows go between the categories and the series, and a press, Enter or Space tells
/// `on_select`; converted into an element it is drawn and read, and follows neither.
#[derive(Clone)]
pub struct BarChart {
  categories: Arc<[Arc<str>]>,
  series: Vec<ChartSeries>,
  stacked: bool,
  current: Option<usize>,
  props: ChartProps,
}

impl BarChart {
  /// A chart of `series`, each with a value for each of `categories`, in their order.
  pub fn new<S: AsRef<str>>(
    categories: impl IntoIterator<Item = S>,
    series: impl IntoIterator<Item = ChartSeries>,
  ) -> Self {
    Self {
      categories: categories.into_iter().map(|name| Arc::from(name.as_ref())).collect(),
      series: series.into_iter().collect(),
      stacked: false,
      current: None,
      props: ChartProps::default(),
    }
  }

  /// Whether the series are stacked, one on another in each category, the first at the bottom: a whole and its
  /// parts. Side by side unless told.
  pub fn stacked(mut self, stacked: bool) -> Self {
    self.stacked = stacked;
    self
  }

  /// The category that is current, the latest release: its bars are in their colors and the others in the input's.
  pub fn current(mut self, category: usize) -> Self {
    self.current = Some(category);
    self
  }

  crate::chart_props!();

  /// The frame of the chart in `theme`: what `build` and `mount` render.
  fn frame(&self, theme: &Theme) -> ChartFrame {
    let plot = Arc::new(BarPlot {
      categories: self.categories.clone(),
      values: self.series.iter().map(|series| Arc::from(series.values())).collect(),
      shown: self.series.iter().map(|series| !self.props.is_hidden(series)).collect(),
      stacked: self.stacked,
      current: self.current,
      gap: 0.0,
    });
    let empty = plot.is_empty();
    let case = self
      .props
      .case(ChartSize::Md, empty)
      .state(BarChartState::Stacked, self.stacked);
    let recipe = theme.resolve::<BarChart>(&case, Some(&self.props.rules));
    // The bands are as far apart as the names under them.
    let gap = recipe
      .part(super::ChartPart::Axis)
      .rest()
      .gap
      .map(|gap| gap.resolve(&theme.spacing(), 0.0))
      .unwrap_or(12.0);
    let plot = Arc::new(BarPlot {
      gap,
      ..Arc::try_unwrap(plot).unwrap_or_else(|plot| (*plot).clone())
    });
    self
      .props
      .frame(recipe, plot, self.series.clone(), theme.renderer::<BarChart>())
  }

  /// Builds the still chart in `theme`: drawn and read by tooling, with nothing that follows the pointer or the keys.
  pub fn build(self, theme: &Theme) -> Element {
    self.frame(theme).build()
  }

  /// The chart mounted where it is declared: the tooltip of the bar under the pointer or the focus, the arrows
  /// between categories and series, and `on_select` told a press.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.props.key.clone();
    ChartFrame::mount(ctx, key, move |theme| self.frame(theme))
  }
}

impl From<BarChart> for Element {
  fn from(chart: BarChart) -> Self {
    chart.build(&current_theme())
  }
}

/// What a bar chart's plot draws: the values of the series shown, by category.
#[derive(Clone)]
struct BarPlot {
  categories: Arc<[Arc<str>]>,
  values: Vec<Arc<[f64]>>,
  shown: Vec<bool>,
  stacked: bool,
  current: Option<usize>,
  gap: f32,
}

impl BarPlot {
  fn value(&self, series: usize, category: usize) -> Option<f64> {
    self
      .values
      .get(series)
      .and_then(|values| values.get(category))
      .copied()
      .filter(|value| value.is_finite())
  }

  /// The series shown, in their order.
  fn shown(&self) -> Vec<usize> {
    (0..self.values.len()).filter(|series| self.shown[*series]).collect()
  }

  /// The values the scale reaches: each value, or each stack's ends.
  fn extents(&self) -> Vec<f64> {
    let shown = self.shown();
    (0..self.categories.len())
      .flat_map(|category| {
        if self.stacked {
          let (mut up, mut down) = (0.0, 0.0);
          for series in &shown {
            match self.value(*series, category) {
              Some(value) if value >= 0.0 => up += value,
              Some(value) => down += value,
              None => {}
            }
          }
          vec![up, down]
        } else {
          shown
            .iter()
            .filter_map(|series| self.value(*series, category))
            .collect()
        }
      })
      .collect()
  }

  fn scale(&self, area: ChartArea) -> LinearScale {
    LinearScale::nice(self.extents(), (area.height, 0.0), 4)
  }

  fn bands(&self, area: ChartArea) -> BandScale {
    BandScale::new(self.categories.len(), area.width, self.gap)
  }

  /// Where the bar of `series` in `category` is drawn: x, y, width, height, and which end is rounded. `None` where
  /// the series is hidden or has no value there.
  fn bar(&self, area: ChartArea, series: usize, category: usize) -> Option<((f32, f32, f32, f32), BarEnd)> {
    if !self.shown.get(series).copied().unwrap_or(false) {
      return None;
    }
    let value = self.value(series, category)?;
    let (scale, bands) = (self.scale(area), self.bands(area));
    let zero = scale.map(0.0);
    let shown = self.shown();
    let (x, width, from, to) = if self.stacked {
      // Below it, the values of the series before it on the same side of zero.
      let below: f64 = shown
        .iter()
        .take_while(|other| **other != series)
        .filter_map(|other| self.value(*other, category))
        .filter(|other| (*other >= 0.0) == (value >= 0.0))
        .sum();
      (bands.start(category), bands.width(), below, below + value)
    } else {
      let place = shown.iter().position(|other| *other == series)?;
      let count = shown.len() as f32;
      let width = ((bands.width() - GROUP_GAP * (count - 1.0)) / count).max(1.0);
      (
        bands.start(category) + place as f32 * (width + GROUP_GAP),
        width,
        0.0,
        value,
      )
    };
    let (a, b) = (scale.map(from), scale.map(to));
    let (top, bottom) = (a.min(b), a.max(b));
    // A segment over another leaves a pixel of the card between them.
    let gap = if self.stacked && from != 0.0 { SEGMENT_GAP } else { 0.0 };
    let (top, bottom) = if value >= 0.0 {
      (top, (bottom - gap).min(zero))
    } else {
      ((top + gap).max(zero), bottom)
    };
    Some((
      (x, top, width, (bottom - top).max(0.0)),
      if value >= 0.0 { BarEnd::Top } else { BarEnd::Bottom },
    ))
  }

  /// Whether the bar of `series` in `category` is the outer one of its stack: its end is rounded.
  fn outermost(&self, series: usize, category: usize) -> bool {
    if !self.stacked {
      return true;
    }
    let Some(value) = self.value(series, category) else {
      return false;
    };
    !self
      .shown()
      .into_iter()
      .filter(|other| *other > series)
      .filter_map(|other| self.value(other, category))
      .any(|other| other != 0.0 && (other >= 0.0) == (value >= 0.0))
  }
}

impl ChartPlot for BarPlot {
  fn paint(&self, canvas: &Context2D, area: ChartArea, look: &ChartLook, active: Option<ChartPoint>) {
    let bands = self.bands(area);
    if let Some(point) = active {
      canvas.set_fill_style(look.color(BarChartPart::Band));
      canvas.fill_rect(bands.start(point.index), 0.0, bands.width(), area.height);
    }
    let muted = look.color(BarChartPart::Bar);
    for category in 0..self.categories.len() {
      for series in 0..self.values.len() {
        let Some((rect, end)) = self.bar(area, series, category) else {
          continue;
        };
        let color = match self.current {
          Some(current) if current != category => muted,
          _ => look.series(series),
        };
        canvas.set_fill_style(color);
        let radius = if self.outermost(series, category) {
          BAR_RADIUS
        } else {
          0.0
        };
        fill_bar(canvas, rect, radius, end);
      }
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
    ChartAxis::Bands {
      labels: self.categories.iter().map(|name| name.to_string()).collect(),
      gap: self.gap,
    }
  }

  fn point_at(&self, area: ChartArea, x: f32, y: f32) -> Option<ChartPoint> {
    let category = self.bands(area).index_at(x)?;
    let bars: Vec<(usize, (f32, f32, f32, f32))> = self
      .shown()
      .into_iter()
      .filter_map(|series| Some((series, self.bar(area, series, category)?.0)))
      .collect();
    // The bar the pointer is on; in a stack the segment at its height, in a group the bar across from it.
    let on = |&(_, (bx, by, bw, bh)): &(usize, (f32, f32, f32, f32))| {
      if self.stacked {
        y >= by && y <= by + bh
      } else {
        x >= bx && x <= bx + bw
      }
    };
    let series = bars
      .iter()
      .find(|bar| on(bar))
      .or_else(|| bars.last())
      .map(|(series, _)| *series)?;
    Some(ChartPoint::new(series, category))
  }

  fn anchor(&self, area: ChartArea, point: ChartPoint) -> Option<(f32, f32)> {
    let bands = self.bands(area);
    let top = self
      .shown()
      .into_iter()
      .filter_map(|series| self.bar(area, series, point.index))
      .map(|((_, y, ..), _)| y)
      .fold(area.height, f32::min);
    Some((bands.start(point.index) + bands.width() / 2.0, top))
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
    let mut items = Vec::new();
    for category in 0..self.categories.len() {
      for series in self.shown() {
        let (Some(((x, y, width, height), _)), Some(value)) =
          (self.bar(area, series, category), self.value(series, category))
        else {
          continue;
        };
        let name = &self.categories[category];
        let label = if self.values.len() > 1 {
          format!("{name}, series {}", series + 1)
        } else {
          name.to_string()
        };
        items.push(
          CanvasItem::rect(format!("bar-{series}-{category}"), "bar", x, y, width, height.max(1.0))
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
      self.stacked,
      self.current,
      self.gap.to_bits(),
    ))
  }

  fn is_empty(&self) -> bool {
    self.categories.is_empty() || self.shown().iter().all(|series| self.values[*series].is_empty())
  }
}

impl Themed for BarChart {
  const NAME: &'static str = "bar_chart";
  type Renderer = ChartRenderer;

  /// The chart's frame ([`chart_recipe`]) at the board's Bar Chart's size, `Md`, and its bars: 12 apart, 14 where
  /// they are stacked, as the board's Stacked Bar Chart has them; the bars of the categories that give way to the
  /// current one in the input's color; the band behind the category under the pointer or the focus in the muted
  /// color.
  fn recipe() -> Recipe {
    chart_recipe()
      .base(BarChartPart::Bar, |s| s.background(PaletteColor::Input))
      .base(BarChartPart::Band, |s| s.background(PaletteColor::Muted))
      .state(BarChartState::Stacked, super::ChartPart::Axis, |s| s.gap(14.0))
  }
}
