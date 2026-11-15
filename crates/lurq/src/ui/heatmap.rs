//! A heatmap: a grid of cells, a row for each series and a column for each category, each cell colored by its
//! value's step on a scale between two roles.

use std::{hash::Hash, sync::Arc};

use super::{
  chart::{
    ChartArea, ChartFrame, ChartLabel, ChartLook, ChartMotion, ChartPart, ChartPlot, ChartPoint, ChartProps,
    ChartRenderer, ChartSeries, ChartSize, ChartTip, bits, chart_recipe, resolve, signature_of,
  },
  names,
  part_style::{FontFamily, Length},
  recipe::{Recipe, ResolvedRecipe},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    ctx::Ctx,
    theme::{PaletteColor, Theme},
  },
  canvas::{CanvasItem, Context2D},
  components::Row,
  layout::{Alignment, layout_kind::Justify},
  node::{Element, color::Color},
};

/// The parts of a heatmap a recipe styles, besides its frame's ([`ChartPart`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HeatmapPart {
  /// A cell at the scale's lowest step, a value at or under its least: its color (`Muted`).
  Cell,
  /// The color of the scale's highest step (`Chart1`); the steps between are it at their share of opacity over the
  /// card.
  Fill,
  /// The edge of a cell that has no value.
  Missing,
  /// The ring round the cell under the pointer or the focus.
  Ring,
  /// The scale under the grid: the room between its words and its steps.
  Scale,
  /// A step of the scale.
  Step,
  /// The scale's words at its ends.
  ScaleLabel,
}

names!(HeatmapPart {
  Cell => "cell",
  Fill => "fill",
  Missing => "missing",
  Ring => "ring",
  Scale => "scale",
  Step => "step",
  ScaleLabel => "scale_label",
});

/// The room between two cells, and their corners, as the board's.
const GAP: f32 = 4.0;
const RADIUS: f32 = 3.0;
/// The board's cell.
const CELL: f32 = 14.0;
/// The room under the grid for the columns' names, where they are written.
const COLUMN_ROOM: f32 = 18.0;
/// The advance of the rows' names, mono 10, and the room after the widest.
const NAME_ADVANCE: f32 = 6.0;
const NAME_GAP: f32 = 8.0;

/// A heatmap: the board's Heatmap (`rSu67`). Each series is a row and each category a column; a cell's color is its
/// value's step on the scale, from the muted color at the least to the first chart color at the most, the scale
/// written under the grid from "Less" to "More".
///
/// ```
/// use lurq::ui::{ChartSeries, Heatmap};
///
/// let weeks: Vec<String> = (1..=18).map(|week| format!("Week {week}")).collect();
/// let commits = |day: usize| (0..18).map(move |week| ((week * 7 + day) * 37 % 9) as f64);
/// let chart = Heatmap::new(
///   weeks,
///   ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
///     .iter()
///     .enumerate()
///     .map(|(day, name)| ChartSeries::new(name, commits(day))),
/// )
/// .title("Commits")
/// .description("Per day, last 18 weeks");
/// ```
///
/// The scale has five steps unless told ([`steps`](Self::steps)): a value at or under its least (zero, or the least
/// value where one is under zero, unless told a [`range`](Self::range)) is in `Muted`, and the others are `Chart1` at
/// a quarter, a half, three quarters and the whole of its opacity over the card, as the board draws them; a step of
/// the scale under the grid is drawn in the same color as a cell of that step. A cell that has no value (not a
/// number) is an edge alone. Mounted ([`mount`](Self::mount)), the cell under the pointer or the focus is ringed and
/// has its tooltip; the plot is one stop of Tab whose arrows go from cell to cell, Left and Right along a row and Up
/// and Down along a column, and a press, Enter or Space tells `on_select` the cell (`ChartPoint { series: row,
/// index: column }`).
#[derive(Clone)]
pub struct Heatmap {
  columns: Arc<[Arc<str>]>,
  rows: Vec<ChartSeries>,
  steps: usize,
  range: Option<(f64, f64)>,
  cell_size: f32,
  column_axis: bool,
  less_text: Arc<str>,
  more_text: Arc<str>,
  missing_text: Arc<str>,
  props: ChartProps,
}

impl Heatmap {
  /// A grid of `rows`, each a series with a value for each of `columns`, in their order.
  pub fn new<S: AsRef<str>>(columns: impl IntoIterator<Item = S>, rows: impl IntoIterator<Item = ChartSeries>) -> Self {
    Self {
      columns: columns.into_iter().map(|name| Arc::from(name.as_ref())).collect(),
      rows: rows.into_iter().collect(),
      steps: 5,
      range: None,
      cell_size: CELL,
      column_axis: false,
      less_text: Arc::from("Less"),
      more_text: Arc::from("More"),
      missing_text: Arc::from("No value"),
      props: ChartProps::default(),
    }
  }

  /// How many steps the scale has, two at least: five unless told, the board's.
  pub fn steps(mut self, steps: usize) -> Self {
    self.steps = steps.max(2);
    self
  }

  /// The values the scale runs between: a value at or under `least` is the lowest step, one at or over `most` the
  /// highest. From zero (or the least value, where one is under zero) to the most value unless told.
  pub fn range(mut self, least: f64, most: f64) -> Self {
    self.range = Some((least, most));
    self
  }

  /// The side of a cell: the board's 14 unless told. Cells are smaller where the grid is wider than the frame (the
  /// plot keeps the height of cells of this side, so give a grid of many columns a side that fits), and as tall as the
  /// rows can be in the plot's [`height`](Self::height) where it is given one.
  pub fn cell_size(mut self, size: f32) -> Self {
    self.cell_size = size.max(1.0);
    self
  }

  /// Whether the columns' names are written under the grid, as many as have room, the first at its start. Off
  /// unless told, as the board's; the tooltip and tooling say a cell's column either way.
  pub fn column_axis(mut self, column_axis: bool) -> Self {
    self.column_axis = column_axis;
    self
  }

  /// The word at the scale's lowest end: "Less" unless told.
  pub fn less_text(mut self, text: &str) -> Self {
    self.less_text = Arc::from(text);
    self
  }

  /// The word at the scale's highest end: "More" unless told.
  pub fn more_text(mut self, text: &str) -> Self {
    self.more_text = Arc::from(text);
    self
  }

  /// What the tooltip and tooling say of a cell that has no value: "No value" unless told.
  pub fn missing_text(mut self, text: &str) -> Self {
    self.missing_text = Arc::from(text);
    self
  }

  crate::chart_props!();

  /// The plot of the chart, its cells `cell` on a side.
  fn plot(&self, cell: f32) -> HeatPlot {
    let values: Vec<Arc<[f64]>> = self
      .rows
      .iter()
      .map(|row| {
        (0..self.columns.len())
          .map(|column| row.values().get(column).copied().unwrap_or(f64::NAN))
          .collect()
      })
      .collect();
    let finite = || {
      values
        .iter()
        .flat_map(|row| row.iter().copied())
        .filter(|value| value.is_finite())
    };
    let (least, most) = self.range.unwrap_or_else(|| {
      (
        finite().fold(0.0_f64, f64::min),
        finite().fold(f64::NEG_INFINITY, f64::max),
      )
    });
    let widest = self
      .rows
      .iter()
      .map(|row| row.name().chars().count())
      .max()
      .unwrap_or(0);
    HeatPlot {
      columns: self.columns.clone(),
      rows: self.rows.iter().map(|row| Arc::from(row.name())).collect(),
      values,
      steps: self.steps,
      least,
      most,
      cell,
      room: if widest > 0 {
        (widest as f32 * NAME_ADVANCE).ceil() + NAME_GAP
      } else {
        0.0
      },
      column_axis: self.column_axis,
      missing: self.missing_text.clone(),
      fixed: self.props.height.is_some(),
    }
  }

  /// The frame of the chart in `theme`: what `build` and `mount` render.
  fn frame(&self, theme: &Theme) -> ChartFrame {
    let rows = self.rows.len() as f32;
    let under = if self.column_axis { COLUMN_ROOM } else { 0.0 };
    let empty = self.columns.is_empty() || self.rows.is_empty();
    let case = self.props.case(ChartSize::Sm, empty);
    let recipe = theme.resolve::<Heatmap>(&case, Some(&self.props.rules));
    // The cells' side: the board's, or what the plot's height leaves each row where it is given one.
    let cell = match self.props.height {
      Some(_) if rows > 0.0 => ((self.props.plot_height(&recipe) - under - GAP * (rows - 1.0)) / rows).max(1.0),
      _ => self.cell_size,
    };
    let plot = self.plot(cell);
    let mut props = self.props.clone();
    props.height = Some(Length::from((rows * (cell + GAP) - GAP).max(cell) + under));
    let scale = self.scale(&recipe, &plot);
    // The rows are named in the plot; the scale stands in place of their legend.
    props
      .frame(recipe, Arc::new(plot), self.rows.clone(), theme.renderer::<Heatmap>())
      .own_legend(scale)
  }

  /// The scale under the grid, at the frame's end: its lowest word, a step of each color, its highest word.
  fn scale(&self, recipe: &ResolvedRecipe, plot: &HeatPlot) -> Element {
    let part = recipe.part(HeatmapPart::Scale);
    let label = recipe.part(HeatmapPart::ScaleLabel);
    let step = recipe.part(HeatmapPart::Step);
    let colors = Colors::of(recipe);
    let mut row = Row::new()
      .width(crate::node::dimension::Dimension::Pct(100.0))
      .justify(Justify::End)
      .align_items(Alignment::Center)
      .part(&part)
      .describe("a11y_role", "img")
      .describe("a11y_name", format!("{} to {}", self.less_text, self.more_text))
      .child(label.text(&self.less_text).part(&label).nowrap());
    for at in 0..plot.steps {
      let mut swatch = Row::new().part(&step).background(colors.step(at, plot.steps));
      if let Some(id) = &self.props.id {
        swatch = swatch.id(format!("{id}-scale-{at}"));
      }
      row = row.child(swatch);
    }
    if let Some(id) = &self.props.id {
      row = row.id(format!("{id}-scale"));
    }
    row.child(label.text(&self.more_text).part(&label).nowrap()).into()
  }

  /// Builds the still chart in `theme`: drawn and read by tooling, with nothing that follows the pointer or the keys.
  pub fn build(self, theme: &Theme) -> Element {
    self.frame(theme).build()
  }

  /// The chart mounted where it is declared: the cell under the pointer or the focus ringed with its tooltip, the
  /// arrows from cell to cell, and `on_select` told a press.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.props.key.clone();
    ChartFrame::mount(ctx, key, move |theme| self.frame(theme))
  }
}

impl From<Heatmap> for Element {
  fn from(chart: Heatmap) -> Self {
    chart.build(&current_theme())
  }
}

/// The two roles of the scale as the theme has them.
struct Colors {
  cell: Color,
  fill: Color,
}

impl Colors {
  /// The two roles in a plot's look.
  fn from_look(look: &ChartLook) -> Self {
    Self {
      cell: look.color(HeatmapPart::Cell),
      fill: look.color(HeatmapPart::Fill),
    }
  }

  fn of(recipe: &ResolvedRecipe) -> Self {
    let color = |part: HeatmapPart, or: PaletteColor| {
      recipe
        .part(part)
        .rest()
        .background
        .map(|background| resolve(recipe.theme(), background))
        .unwrap_or_else(|| recipe.theme().palette_color(or))
    };
    Self {
      cell: color(HeatmapPart::Cell, PaletteColor::Muted),
      fill: color(HeatmapPart::Fill, PaletteColor::Chart1),
    }
  }

  /// The color of step `at` of `steps`: the cell's at the lowest, else the fill at the step's share of its opacity.
  fn step(&self, at: usize, steps: usize) -> Color {
    if at == 0 {
      return self.cell;
    }
    let share = at as f32 / (steps - 1).max(1) as f32;
    self.fill.with_opacity(self.fill.a() as f32 / 255.0 * share)
  }
}

/// What a heatmap's plot draws: the rows' names at its start and the grid of cells after them.
#[derive(Clone)]
struct HeatPlot {
  columns: Arc<[Arc<str>]>,
  rows: Vec<Arc<str>>,
  values: Vec<Arc<[f64]>>,
  steps: usize,
  least: f64,
  most: f64,
  /// The cells' side, where the plot is wide enough.
  cell: f32,
  /// The room of the rows' names.
  room: f32,
  column_axis: bool,
  missing: Arc<str>,
  /// Whether the app gave the plot its height: the cells fill it, and it does not follow the width.
  fixed: bool,
}

impl HeatPlot {
  fn value(&self, row: usize, column: usize) -> Option<f64> {
    self
      .values
      .get(row)
      .and_then(|values| values.get(column))
      .copied()
      .filter(|value| value.is_finite())
  }

  /// The step of `value` on the scale: the lowest at or under its least, else as high as its share of the range.
  fn step(&self, value: f64) -> usize {
    let top = self.steps - 1;
    if value <= self.least || !(self.most > self.least) {
      return if value > self.least { top } else { 0 };
    }
    let share = ((value - self.least) / (self.most - self.least)).clamp(0.0, 1.0);
    ((share * top as f64).ceil() as usize).clamp(1, top)
  }

  /// The cells' side in `area`: the plot's, smaller where the grid is wider than the plot, its gaps smaller with it.
  fn side(&self, area: ChartArea) -> f32 {
    let columns = self.columns.len() as f32;
    if area.width <= 0.0 || columns == 0.0 {
      return self.cell;
    }
    // A cell and the gap after it are `pitch` of the cell's side; the last has no gap after it.
    let pitch = 1.0 + GAP / self.cell;
    let fits = (area.width - self.room) / (columns * pitch - GAP / self.cell);
    self.cell.min(fits).max(0.5)
  }

  /// The room between two cells of `side`: the board's 4 beside its cell of 14, as much smaller as the cell is.
  fn gap(&self, side: f32) -> f32 {
    GAP * (side / self.cell).min(1.0)
  }

  /// Where the cell of `row` and `column` is drawn: x, y and its side.
  fn cell(&self, area: ChartArea, row: usize, column: usize) -> (f32, f32, f32) {
    let side = self.side(area);
    let pitch = side + self.gap(side);
    (self.room + column as f32 * pitch, row as f32 * pitch, side)
  }

  /// The color of the cell of `row` and `column`: its step's, or none where it has no value.
  fn color(&self, colors: &Colors, row: usize, column: usize) -> Option<Color> {
    self
      .value(row, column)
      .map(|value| colors.step(self.step(value), self.steps))
  }

  /// Draws the cells, each filled with the color `color` gives it (a cell with no value has its edge, and is filled
  /// too where `color` gives it one: its old color fading out), and the ring of `active`.
  fn paint_cells(
    &self,
    canvas: &Context2D,
    area: ChartArea,
    look: &ChartLook,
    active: Option<ChartPoint>,
    color: impl Fn(usize, usize) -> Option<Color>,
  ) {
    for row in 0..self.rows.len() {
      for column in 0..self.columns.len() {
        let (x, y, side) = self.cell(area, row, column);
        let r = RADIUS.min(side / 2.0);
        if self.value(row, column).is_none() {
          canvas.set_stroke_style(look.color(HeatmapPart::Missing));
          canvas.set_line_width(1.0);
          canvas.begin_path();
          let _ = canvas.round_rect(x + 0.5, y + 0.5, side - 1.0, side - 1.0, (r - 0.5).max(0.0));
          canvas.stroke();
        }
        if let Some(fill) = color(row, column) {
          canvas.set_fill_style(fill);
          canvas.begin_path();
          let _ = canvas.round_rect(x, y, side, side, r);
          canvas.fill();
        }
      }
    }
    if let Some(point) = active
      && point.series < self.rows.len()
      && point.index < self.columns.len()
    {
      let (x, y, side) = self.cell(area, point.series, point.index);
      canvas.set_stroke_style(look.color(HeatmapPart::Ring));
      canvas.set_line_width(1.5);
      canvas.begin_path();
      let _ = canvas.round_rect(x - 1.25, y - 1.25, side + 2.5, side + 2.5, RADIUS + 1.25);
      canvas.stroke();
    }
  }

  fn name(&self, row: usize, column: usize) -> String {
    format!("{}, {}", self.rows[row], self.columns[column])
  }
}

impl ChartPlot for HeatPlot {
  fn paint(&self, canvas: &Context2D, area: ChartArea, look: &ChartLook, active: Option<ChartPoint>) {
    let colors = Colors::from_look(look);
    self.paint_cells(canvas, area, look, active, |row, column| {
      self.color(&colors, row, column)
    });
  }

  /// The cells fade in as the heatmap first shows; on a change each cell goes from its old step's color to its new
  /// one, a cell that comes fading in and one that goes fading out.
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
    let colors = Colors::from_look(look);
    let from = motion.from_as::<HeatPlot>();
    self.paint_cells(canvas, area, look, active, |row, column| {
      let to = self.color(&colors, row, column);
      let was = from.and_then(|from| {
        (row < from.rows.len() && column < from.columns.len())
          .then(|| from.color(&colors, row, column))
          .flatten()
      });
      match (was, to) {
        (Some(was), Some(to)) => Some(mix(was, to, motion.progress)),
        (None, Some(to)) => Some(mix(to.with_alpha(0), to, motion.progress)),
        (Some(was), None) => Some(mix(was, was.with_alpha(0), motion.progress)),
        (None, None) => None,
      }
    });
  }

  fn height_for(&self, width: f32) -> Option<f32> {
    if self.fixed || self.is_empty() {
      return None;
    }
    let side = self.side(ChartArea { width, height: 0.0 });
    let rows = self.rows.len() as f32;
    let under = if self.column_axis { COLUMN_ROOM } else { 0.0 };
    let gap = self.gap(side);
    Some((rows * (side + gap) - gap).max(side) + under)
  }

  fn point_at(&self, area: ChartArea, x: f32, y: f32) -> Option<ChartPoint> {
    let side = self.side(area);
    let gap = self.gap(side);
    let pitch = side + gap;
    if x < self.room || y < 0.0 {
      return None;
    }
    // A cell and half the gaps round it.
    let column = ((x - self.room + gap / 2.0) / pitch) as usize;
    let row = ((y + gap / 2.0) / pitch) as usize;
    (row < self.rows.len() && column < self.columns.len()).then(|| ChartPoint::new(row, column))
  }

  fn anchor(&self, area: ChartArea, point: ChartPoint) -> Option<(f32, f32)> {
    (point.series < self.rows.len() && point.index < self.columns.len()).then(|| {
      let (x, y, side) = self.cell(area, point.series, point.index);
      (x + side / 2.0, y)
    })
  }

  fn counts(&self) -> Vec<usize> {
    vec![self.columns.len(); self.rows.len()]
  }

  fn tip(&self, point: ChartPoint) -> ChartTip {
    let column = self
      .columns
      .get(point.index)
      .map(|name| name.to_string())
      .unwrap_or_default();
    match self.value(point.series, point.index) {
      Some(value) => ChartTip {
        title: column,
        rows: vec![(point.series, value)],
      },
      None => ChartTip {
        title: format!(
          "{column}, {}: {}",
          self.rows.get(point.series).map_or("", |row| row),
          self.missing
        ),
        rows: Vec::new(),
      },
    }
  }

  fn tip_color(&self, point: ChartPoint, _series: usize, look: &ChartLook) -> Option<Color> {
    let value = self.value(point.series, point.index)?;
    let colors = Colors::from_look(look);
    // The key as opaque as the cell looks on the card.
    let color = colors.step(self.step(value), self.steps);
    Some(over(color, look.theme().palette_color(PaletteColor::Popover)))
  }

  fn items(&self, area: ChartArea, look: &ChartLook) -> Vec<CanvasItem> {
    let mut items = Vec::new();
    for row in 0..self.rows.len() {
      for column in 0..self.columns.len() {
        let (x, y, side) = self.cell(area, row, column);
        let value = match self.value(row, column) {
          Some(value) => (look.value_text)(value),
          None => self.missing.to_string(),
        };
        items.push(
          CanvasItem::rect(format!("cell-{row}-{column}"), "cell", x, y, side, side)
            .label(self.name(row, column))
            .value(value),
        );
      }
    }
    items
  }

  fn labels(&self, area: ChartArea, _look: &ChartLook) -> Vec<ChartLabel> {
    let side = self.side(area);
    let pitch = side + self.gap(side);
    // Every row's name where the rows are roomy or few, every other's where there are more than four as close as
    // the board's, and fewer where the cells are smaller still: a name of 10 needs 14 of height.
    let apart = ((14.0 / pitch).ceil() as usize).max(1);
    let every = if pitch < 24.0 && self.rows.len() > 4 {
      apart.max(2)
    } else {
      apart
    };
    let mut labels: Vec<ChartLabel> = (0..self.rows.len())
      .step_by(every)
      .map(|row| {
        ChartLabel::new(
          0.0,
          row as f32 * pitch + side / 2.0,
          self.rows[row].to_string(),
          (0.0, -0.5),
        )
      })
      .collect();
    if self.column_axis && !self.columns.is_empty() {
      // As many names as have room, a name's width apart, from the first.
      let widest = self.columns.iter().map(|name| name.chars().count()).max().unwrap_or(0) as f32;
      let skip = (((widest * NAME_ADVANCE + NAME_GAP) / pitch).ceil() as usize).max(1);
      let under = self.rows.len() as f32 * pitch;
      labels.extend((0..self.columns.len()).step_by(skip).map(|column| {
        ChartLabel::new(
          self.room + column as f32 * pitch,
          under,
          self.columns[column].to_string(),
          (0.0, 0.0),
        )
      }));
    }
    labels
  }

  fn signature(&self) -> u64 {
    signature_of((
      self.columns.iter().map(|name| name.to_string()).collect::<Vec<_>>(),
      self.rows.iter().map(|name| name.to_string()).collect::<Vec<_>>(),
      self
        .values
        .iter()
        .map(|values| values.iter().map(|value| bits(*value)).collect::<Vec<_>>())
        .collect::<Vec<_>>(),
      self.steps,
      bits(self.least),
      bits(self.most),
      self.cell.to_bits(),
      self.room.to_bits(),
    ))
  }

  fn is_empty(&self) -> bool {
    self.columns.is_empty() || self.rows.is_empty()
  }
}

/// `progress` of the way from `from` to `to`, each channel and the opacity.
fn mix(from: Color, to: Color, progress: f32) -> Color {
  let channel = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * progress.clamp(0.0, 1.0)).round() as u8;
  Color::new(
    channel(from.r(), to.r()),
    channel(from.g(), to.g()),
    channel(from.b(), to.b()),
    channel(from.a(), to.a()),
  )
}

/// `color` over `ground`, as the engine lays a translucent color over an opaque one.
fn over(color: Color, ground: Color) -> Color {
  let alpha = color.a() as f32 / 255.0;
  let mix = |top: u8, under: u8| (top as f32 * alpha + under as f32 * (1.0 - alpha)).round() as u8;
  Color::new(
    mix(color.r(), ground.r()),
    mix(color.g(), ground.g()),
    mix(color.b(), ground.b()),
    255,
  )
}

impl Themed for Heatmap {
  const NAME: &'static str = "heatmap";
  type Renderer = ChartRenderer;

  /// The chart's frame ([`chart_recipe`]) at the board's Heatmap's size, `Sm`: cells of 14 with corners of 3, 4
  /// apart, the lowest step in `Muted` and the others `Chart1` at a quarter, a half, three quarters and the whole of
  /// its opacity; the rows' names mono 10 in `MutedForeground` at the plot's start; a cell with no value an edge of 1
  /// in `Border`, the cell under the pointer or the focus ringed 1.5 in `Foreground`; the scale under the grid at the
  /// frame's end, 6 between its words (mono 11 in `MutedForeground`) and its steps of 12 with corners of 3.
  fn recipe() -> Recipe {
    chart_recipe()
      .base(ChartPart::AxisLabel, |s| s.font_size(10.0))
      .base(HeatmapPart::Cell, |s| s.background(PaletteColor::Muted))
      .base(HeatmapPart::Fill, |s| s.background(PaletteColor::Chart1))
      .base(HeatmapPart::Missing, |s| s.background(PaletteColor::Border))
      .base(HeatmapPart::Ring, |s| s.background(PaletteColor::Foreground))
      .base(HeatmapPart::Scale, |s| s.gap(6.0))
      .base(HeatmapPart::Step, |s| s.square(12.0).radius(RADIUS))
      .base(HeatmapPart::ScaleLabel, |s| {
        s.font_family(FontFamily::Mono)
          .font_size(11.0)
          .foreground(PaletteColor::MutedForeground)
      })
  }
}
