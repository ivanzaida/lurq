//! What every chart of the design system stands on: the series and their colors, the scales and their round
//! ticks, the frame (the title and the description over the plot, the axis under it, the legend), the
//! [`ChartLegend`] and the [`ChartTooltip`] as the board draws them, the pointer finding the nearest point, one stop
//! of Tab with the arrows between points and series, the text alternative tooling reads, the empty and loading
//! states, and the plot drawn on a [`Canvas`](crate::components::Canvas) again only when what it shows changes.
//!
//! A chart is drawn on the canvas, behind the `canvas` feature. Its words (the title, the axis, the legend, the
//! tooltip) are elements, in the theme's type.
//!
//! # What a chart's author calls
//!
//! A chart is a builder of the design system (its parts, its states, its recipe, `build`, `mount`, `From`), whose
//! plot is a [`ChartPlot`]: what it draws and where its points are. The rest is the ground's:
//!
//! 1. Its data: [`ChartSeries`] (a name and values, or points), told apart by their index; a place in it is a
//!    [`ChartPoint`].
//! 2. Its recipe: [`chart_recipe`] laid under its own rules, which styles the frame's parts ([`ChartPart`]) in the two
//!    sizes of the board ([`ChartSize`]); the chart adds the parts of its own marks.
//! 3. Its plot: `impl ChartPlot`, in the plot's box ([`ChartArea`]), with the colors of [`ChartLook`] (a series' color,
//!    a part's), a [`LinearScale`] for values and a [`BandScale`] for categories, whose ticks land on round numbers
//!    ([`LinearScale::ticks`]), and [`nearest`] for the point under the pointer.
//! 4. Its frame: [`ChartFrame`], given the resolved recipe, the plot, the series, the words and the handlers, which
//!    builds the header, the plot with its grid, its axis, its tooltip and its keys, and the legend; `mount(ctx)` keeps
//!    the pointer, the focus and the drawing, `build()` is its still form.
//!
//! The names every chart gives its props:
//!
//! | Prop | |
//! | --- | --- |
//! | `new(categories, series)` (or `new(series)` where it has no categories) | What it shows |
//! | `title(&str)`, `description(&str)`, `label(&str)` | The header, and the name to tooling (the title's unless told) |
//! | `size(ChartSize)`, `height(impl Into<Length>)` | How roomy the frame is, and how high the plot |
//! | `legend(bool)`, `hidden(Signal<Vec<String>>)`, `hidden_controlled(..)`, `on_hidden_change(..)` | The legend, and the series a press of its item hides |
//! | `value_text(Fn(f64) -> String)` | Every number it writes: a tick, a tooltip's value, what tooling reads |
//! | `grid(bool)`, `value_axis(bool)` | The lines across the plot at its ticks, and the ticks' numbers |
//! | `empty_text(&str)`, `loading(bool)` | What it says with no data, and its skeleton while the data comes |
//! | `on_select(Fn(ChartPoint))` | A press of a point, or Enter or Space on the focused one |
//! | `style`, `rules`, `state`, `id`, `key`, `force` | As every themed builder has them |

#![cfg(feature = "canvas")]

mod frame;
mod legend;
mod scale;
mod tooltip;

use std::{
  hash::{Hash, Hasher},
  sync::Arc,
};

pub use frame::{ChartFrame, ChartProps, ChartRenderer, ChartView, chart_recipe, snap};
pub use legend::{
  ChartLegend, ChartLegendItem, ChartLegendPart, ChartLegendRenderer, ChartLegendShape, ChartLegendState,
  ChartLegendView,
};
pub use scale::{BandScale, LinearScale};
pub use tooltip::{ChartTooltip, ChartTooltipPart, ChartTooltipRenderer, ChartTooltipRow, ChartTooltipView};

use super::{
  names,
  recipe::{Name, ResolvedPart, ResolvedRecipe},
};
use crate::{
  app::theme::{PaletteColor, Theme},
  canvas::{CanvasItem, Context2D},
  node::{BackgroundColor, color::Color},
  ui::part_style::Opacity,
};

/// One series of a chart: its name, which the legend and the tooltip say, and its values, one for each category, or
/// its points, an x and a y each. Its color is the theme's next chart color unless it is given one.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartSeries {
  name: Arc<str>,
  values: Arc<[f64]>,
  points: Arc<[(f64, f64)]>,
  color: Option<BackgroundColor>,
}

impl ChartSeries {
  /// A series of `values`, one for each category, in their order.
  pub fn new<V: Into<f64>>(name: &str, values: impl IntoIterator<Item = V>) -> Self {
    Self {
      name: Arc::from(name),
      values: values.into_iter().map(Into::into).collect(),
      points: Arc::from(Vec::new()),
      color: None,
    }
  }

  /// A series of points, an x and a y each: a scatter's.
  pub fn points(name: &str, points: impl IntoIterator<Item = (f64, f64)>) -> Self {
    Self {
      name: Arc::from(name),
      values: Arc::from(Vec::new()),
      points: points.into_iter().collect(),
      color: None,
    }
  }

  /// Its color: a role of the palette or a color, in place of the theme's chart color of its place.
  pub fn color(mut self, color: impl Into<BackgroundColor>) -> Self {
    self.color = Some(color.into());
    self
  }

  pub fn name(&self) -> &str {
    &self.name
  }

  pub fn values(&self) -> &[f64] {
    &self.values
  }

  pub fn point_list(&self) -> &[(f64, f64)] {
    &self.points
  }

  /// The color it was given, if it was.
  pub fn given_color(&self) -> Option<&BackgroundColor> {
    self.color.as_ref()
  }

  /// How many points it has: its values, or its points.
  pub fn len(&self) -> usize {
    self.values.len().max(self.points.len())
  }

  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }
}

/// The chart color of the series at `index`: `Chart1` to `Chart5`, then again from `Chart1`.
pub fn series_color(index: usize) -> PaletteColor {
  match index % 5 {
    0 => PaletteColor::Chart1,
    1 => PaletteColor::Chart2,
    2 => PaletteColor::Chart3,
    3 => PaletteColor::Chart4,
    _ => PaletteColor::Chart5,
  }
}

/// A place in a chart's data: a series, and a point of it (a category's value, a point, a slice).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ChartPoint {
  pub series: usize,
  pub index: usize,
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for ChartPoint {}

impl ChartPoint {
  pub fn new(series: usize, index: usize) -> Self {
    Self { series, index }
  }
}

/// How roomy a chart's frame is: the board's Bar Chart is `Md`, its others `Sm`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ChartSize {
  /// Padded 20 with 16 between its rows, a title of 15 and a description of 12, a plot of 140.
  Sm,
  /// Padded 24 with 20 between its rows, a title of 16 and a description of 13, a plot of 160.
  #[default]
  Md,
}

names!(ChartSize {
  Sm => "sm",
  Md => "md",
});

/// The parts of a chart's frame a recipe styles, which every chart has. A chart's own marks (a bar, a line, a
/// slice) are parts of its own enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChartPart {
  /// The box: the card's fill, its edge, its corners, its shadow, its padding and the room between its rows.
  Root,
  /// The title over the description.
  Header,
  Title,
  Description,
  /// The plot's box: its height, and the ring of the focus from the keyboard.
  Plot,
  /// A line across the plot at a tick: its color and its opacity.
  Grid,
  /// The line the values stand on: the plot's bottom, or zero.
  Baseline,
  /// The line or the band at the point under the pointer.
  Cursor,
  /// The row of the categories' names under the plot, the room between them a band's.
  Axis,
  /// A category's name, or a tick's text on the axis under the plot.
  AxisLabel,
  /// A tick's number beside its grid line.
  ValueLabel,
  /// What the plot says where there is no data.
  Empty,
}

names!(ChartPart {
  Root => "root",
  Header => "header",
  Title => "title",
  Description => "description",
  Plot => "plot",
  Grid => "grid",
  Baseline => "baseline",
  Cursor => "cursor",
  Axis => "axis",
  AxisLabel => "axis_label",
  ValueLabel => "value_label",
  Empty => "empty",
});

/// The states of a chart's frame a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChartState {
  /// It has no data: the plot says so.
  Empty,
  /// Its data is on its way: the plot is a skeleton.
  Loading,
  /// It has a header.
  Header,
}

names!(ChartState {
  Empty => "empty",
  Loading => "loading",
  Header => "header",
});

/// The box the plot is drawn in, in the canvas's logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ChartArea {
  pub width: f32,
  pub height: f32,
}

/// What a chart's numbers are written with.
pub type ValueTextFn = Arc<dyn Fn(f64) -> String + Send + Sync>;

/// A number as a chart writes it unless told: whole where it is, else to two places, with no zeros at the end.
pub fn default_value_text(value: f64) -> String {
  if !value.is_finite() {
    return String::from("–");
  }
  if (value - value.round()).abs() < 1e-9 {
    return format!("{value:.0}");
  }
  let text = format!("{value:.2}");
  text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// The colors a plot is drawn in, read from the theme: a series' color, a part's, and how a number is written.
#[derive(Clone)]
pub struct ChartLook {
  pub recipe: ResolvedRecipe,
  /// The color of each series, by its index.
  pub series: Arc<[Color]>,
  pub value_text: ValueTextFn,
}

impl ChartLook {
  /// The look of `series` in `recipe`'s theme.
  pub fn new(recipe: ResolvedRecipe, series: &[ChartSeries], value_text: ValueTextFn) -> Self {
    let theme = recipe.theme().clone();
    let colors = series
      .iter()
      .enumerate()
      .map(|(index, series)| resolve(&theme, series.color.clone().unwrap_or(series_color(index).into())))
      .collect();
    Self {
      recipe,
      series: colors,
      value_text,
    }
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  /// The color of the series at `index`.
  pub fn series(&self, index: usize) -> Color {
    self
      .series
      .get(index)
      .copied()
      .unwrap_or_else(|| self.theme().palette_color(series_color(index)))
  }

  /// The fill of `part` at rest, with its opacity: a line's or a mark's color.
  pub fn color(&self, part: impl Into<Name>) -> Color {
    let rest = self.part(part).rest();
    let color = rest
      .background
      .map(|background| resolve(self.theme(), background))
      .unwrap_or(Color::new(0, 0, 0, 0));
    match rest.opacity {
      Some(Opacity::Value(opacity)) => color.with_opacity(opacity),
      _ => color,
    }
  }

  /// What every color of the look is, for a signature of a drawing: a change of theme draws it again.
  pub fn hash_into(&self, state: &mut impl Hasher) {
    for color in self.series.iter() {
      color_key(*color).hash(state);
    }
    for part in [ChartPart::Grid, ChartPart::Baseline, ChartPart::Cursor] {
      color_key(self.color(part)).hash(state);
    }
    color_key(self.theme().palette_color(PaletteColor::Card)).hash(state);
  }
}

/// A color as one number, which hashes.
pub fn color_key(color: Color) -> u32 {
  u32::from_be_bytes([color.r(), color.g(), color.b(), color.a()])
}

/// A role or a color as the theme has it.
pub fn resolve(theme: &Theme, color: BackgroundColor) -> Color {
  match color {
    BackgroundColor::Color(color) => color,
    BackgroundColor::Palette(role) => theme.palette_color(role),
    BackgroundColor::CurrentForeground => theme.palette_color(PaletteColor::Foreground),
  }
}

/// A tick of a scale as the frame draws it: a grid line across the plot at `position` (from the plot's top, or its
/// start where the values run across), and its text.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartTick {
  pub position: f32,
  pub text: String,
  /// Whether it is the line the values stand on, drawn in the baseline's color.
  pub baseline: bool,
}

/// What stands under the plot: nothing, the names of the categories each under its band, or texts spread from
/// the plot's start to its end.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum ChartAxis {
  #[default]
  None,
  /// The names of the categories, each as wide as a band, `gap` between them.
  Bands { labels: Vec<String>, gap: f32 },
  /// Texts from the start to the end, the first at the start and the last at the end.
  Spread(Vec<String>),
}

/// Where a plot's tooltip stands: [`ChartPlot::tip_place`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TipPlace {
  /// Over the point, kept inside the plot; under it where there is no room over it.
  #[default]
  Over,
  /// Beside the line at the point's x, 10 from it and 4 from the plot's top: before it, or after it where there is no
  /// room before it.
  Beside,
}

/// What the tooltip of a point says: its title, and a row for each series the point has a value of.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChartTip {
  pub title: String,
  /// The series and its value.
  pub rows: Vec<(usize, f64)>,
}

/// A chart's plot: what it draws in its box, where its points are, and what they say. The frame draws the grid, the
/// axis, the tooltip and the focus around it, keeps the pointer and the keys, and draws it again when what it draws
/// changes (its [`signature`](Self::signature), the size, the theme or the point under the pointer).
pub trait ChartPlot: Send + Sync + 'static {
  /// Draws the plot, `active` the point under the pointer or the focus: the grid is drawn before it.
  fn paint(&self, canvas: &Context2D, area: ChartArea, look: &ChartLook, active: Option<ChartPoint>);

  /// The ticks the frame draws a grid line at, and writes beside it.
  fn ticks(&self, _area: ChartArea, _look: &ChartLook) -> Vec<ChartTick> {
    Vec::new()
  }

  /// What stands under the plot.
  fn axis(&self) -> ChartAxis {
    ChartAxis::None
  }

  /// The point the pointer at `(x, y)` is on, or nearest to where the chart finds one near enough.
  fn point_at(&self, area: ChartArea, x: f32, y: f32) -> Option<ChartPoint>;

  /// Where the tooltip of `point` points to.
  fn anchor(&self, area: ChartArea, point: ChartPoint) -> Option<(f32, f32)>;

  /// Where the tooltip stands: over the point (a bar's), or beside the line the plot draws at the point's x, at the
  /// plot's top, before it where there is room and after it where there is not (a line chart's cursor).
  fn tip_place(&self) -> TipPlace {
    TipPlace::Over
  }

  /// How many points each series has, in the order the arrows go through them.
  fn counts(&self) -> Vec<usize>;

  /// What the tooltip of `point` says.
  fn tip(&self, point: ChartPoint) -> ChartTip;

  /// The data as tooling reads it: an item for each point, its shape where it is drawn, its name and its value.
  fn items(&self, area: ChartArea, look: &ChartLook) -> Vec<CanvasItem>;

  /// What the drawing is made of besides the size, the theme and the point under the pointer: a hash of the data
  /// and of the props that change what is drawn.
  fn signature(&self) -> u64;

  /// Whether there is nothing to draw.
  fn is_empty(&self) -> bool {
    self.counts().iter().all(|count| *count == 0)
  }
}

/// The candidate of `candidates` nearest to `(x, y)`, where it is within `reach`: a point's place and where it is
/// drawn. Where `along_x`, only the distance across is counted (a line's points under the pointer), else the
/// distance itself (a scatter's).
pub fn nearest(
  candidates: impl IntoIterator<Item = (ChartPoint, f32, f32)>,
  x: f32,
  y: f32,
  reach: f32,
  along_x: bool,
) -> Option<ChartPoint> {
  candidates
    .into_iter()
    .map(|(point, px, py)| {
      let distance = if along_x {
        (px - x).abs()
      } else {
        ((px - x).powi(2) + (py - y).powi(2)).sqrt()
      };
      (point, distance)
    })
    .filter(|(_, distance)| *distance <= reach)
    .min_by(|a, b| a.1.total_cmp(&b.1))
    .map(|(point, _)| point)
}

/// The place of the value nearest to `x` among `sorted`, values in rising order: a line of ten thousand points finds
/// the one under the pointer by halves.
pub fn nearest_sorted(sorted: &[f32], x: f32) -> Option<usize> {
  if sorted.is_empty() {
    return None;
  }
  let after = sorted.partition_point(|value| *value < x);
  match after {
    0 => Some(0),
    _ if after >= sorted.len() => Some(sorted.len() - 1),
    _ => Some(if (sorted[after] - x).abs() < (x - sorted[after - 1]).abs() {
      after
    } else {
      after - 1
    }),
  }
}

/// Where a key moves the focused point among series of `counts` points: Left and Right along the series, Up and
/// Down to the series before and after (the same place, or its last), Home and End to its ends. `None` for a key that
/// moves nothing.
pub fn step(counts: &[usize], from: ChartPoint, key: &str) -> Option<ChartPoint> {
  let count = |series: usize| counts.get(series).copied().unwrap_or(0);
  let index = |series: usize, index: usize| index.min(count(series).saturating_sub(1));
  let to = match key {
    "ArrowLeft" => ChartPoint::new(from.series, from.index.saturating_sub(1)),
    "ArrowRight" => ChartPoint::new(from.series, index(from.series, from.index + 1)),
    "Home" => ChartPoint::new(from.series, 0),
    "End" => ChartPoint::new(from.series, count(from.series).saturating_sub(1)),
    "ArrowUp" | "ArrowDown" => {
      let series = if key == "ArrowUp" {
        (0..from.series).rev().find(|series| count(*series) > 0)?
      } else {
        (from.series + 1..counts.len()).find(|series| count(*series) > 0)?
      };
      ChartPoint::new(series, index(series, from.index))
    }
    _ => return None,
  };
  (count(to.series) > 0).then_some(to)
}

/// A hash of what a plot draws, for its [`signature`](ChartPlot::signature).
pub fn signature_of(data: impl Hash) -> u64 {
  let mut hasher = std::collections::hash_map::DefaultHasher::new();
  data.hash(&mut hasher);
  hasher.finish()
}

/// The bits of a number, which hash.
pub fn bits(value: f64) -> u64 {
  value.to_bits()
}

/// The end of a bar that is rounded: the one away from the line the bar stands on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BarEnd {
  Top,
  Bottom,
  Start,
  End,
}

/// Fills a bar of `(x, y, width, height)` with the corners of its `end` rounded by `radius` (no more than half of
/// it), the others square: a bar's, or the last segment of a stack's.
pub fn fill_bar(canvas: &Context2D, (x, y, width, height): (f32, f32, f32, f32), radius: f32, end: BarEnd) {
  if width <= 0.0 || height <= 0.0 {
    return;
  }
  let r = radius.min(width / 2.0).min(height / 2.0).max(0.0);
  if r <= 0.0 {
    canvas.fill_rect(x, y, width, height);
    return;
  }
  let (right, bottom) = (x + width, y + height);
  canvas.begin_path();
  let corner = |cx: f32, cy: f32, tx: f32, ty: f32| {
    let _ = canvas.arc_to(cx, cy, tx, ty, r);
  };
  match end {
    BarEnd::Top => {
      canvas.move_to(x, bottom);
      canvas.line_to(x, y + r);
      corner(x, y, x + r, y);
      corner(right, y, right, y + r);
      canvas.line_to(right, bottom);
    }
    BarEnd::Bottom => {
      canvas.move_to(x, y);
      canvas.line_to(right, y);
      canvas.line_to(right, bottom - r);
      corner(right, bottom, right - r, bottom);
      corner(x, bottom, x, bottom - r);
    }
    BarEnd::End => {
      canvas.move_to(x, y);
      canvas.line_to(right - r, y);
      corner(right, y, right, y + r);
      corner(right, bottom, right - r, bottom);
      canvas.line_to(x, bottom);
    }
    BarEnd::Start => {
      canvas.move_to(right, y);
      canvas.line_to(right, bottom);
      canvas.line_to(x + r, bottom);
      corner(x, bottom, x, bottom - r);
      corner(x, y, x + r, y);
    }
  }
  canvas.close_path();
  canvas.fill();
}
