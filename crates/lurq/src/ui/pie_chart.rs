//! A pie chart: the parts of a whole, as slices of a disc, or of a ring where it is a donut.

use std::{
  f32::consts::{PI, TAU},
  hash::Hash,
  sync::Arc,
};

use super::{
  chart::{
    ChartArea, ChartFrame, ChartLabel, ChartLook, ChartMotion, ChartPart, ChartPlot, ChartPoint, ChartProps,
    ChartRenderer, ChartSeries, ChartSize, ChartTip, ValueTextFn, bits, chart_recipe, signature_of,
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
  layout::text_style::FontWeight,
  node::{BackgroundColor, Element},
};

/// The parts of a pie chart a recipe styles, besides its frame's ([`ChartPart`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PieChartPart {
  /// A slice that gives way to the one under the pointer or the focus: its opacity.
  Slice,
  /// The line between two slices of a pie, where the card shows through: its color.
  Edge,
  /// The total written in a donut's hole.
  Total,
  /// The words under the total.
  TotalLabel,
}

names!(PieChartPart {
  Slice => "slice",
  Edge => "edge",
  Total => "total",
  TotalLabel => "total_label",
});

/// The states of a pie chart a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PieChartState {
  /// It is a ring, with a hole in its middle.
  Donut,
}

names!(PieChartState {
  Donut => "donut",
});

/// What each item of a pie's legend says after the slice's name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PieLegendValue {
  /// The slice's share of the whole: `42%`, through [`PieChart::share_text`].
  #[default]
  Share,
  /// The slice's value, through `value_text`.
  Value,
  /// Nothing: the name alone.
  None,
}

/// How a pie writes a share, from 0 to 1.
pub type ShareTextFn = Arc<dyn Fn(f64) -> String + Send + Sync>;

/// A share as a pie writes it unless told: a whole percent, `42%`; under one, `<1%`.
pub fn default_share_text(share: f64) -> String {
  let percent = share * 100.0;
  if percent > 0.0 && percent < 0.5 {
    return String::from("<1%");
  }
  format!("{percent:.0}%")
}

/// How wide the line between two slices is: the board's edge of 1.5 on each of them.
const EDGE: f32 = 3.0;
/// The hole of a donut, a share of its radius: the board's 0.66.
const HOLE: f32 = 0.66;

/// A pie chart: the board's Pie Chart (`dDHPj`) and, with a hole, its Donut Chart (`GJ5Mg`). Each value of one series
/// is a slice, from the top and clockwise, as large as its share of the whole; the legend beside the pie names the
/// slices and writes each one's share.
///
/// ```
/// use lurq::ui::{ChartSeries, PieChart};
///
/// let chart = PieChart::new(
///   ["render", "layout", "text", "other"],
///   ChartSeries::new("Size", [4.2, 2.8, 1.9, 1.1]),
/// )
/// .title("Crates by size")
/// .description("Share of compiled code")
/// .value_text(|mb| format!("{mb} MB"));
/// ```
///
/// Each slice takes the theme's next chart color unless the chart is given its colors ([`colors`](Self::colors)). A
/// donut ([`donut`](Self::donut)) is the same chart with a hole of 0.66 of its radius, in which it writes the total
/// over the app's words where it is given them ([`total_text`](Self::total_text)). Mounted ([`mount`](Self::mount)),
/// the slice under the pointer or the focus is picked out (the others give way to it) and has its tooltip, the plot is
/// one stop of Tab whose arrows go round the slices, a press of an item of the legend hides its slice where the app
/// holds the hidden ones, and a press, Enter or Space tells `on_select` the slice (`ChartPoint { series: 0, index }`).
#[derive(Clone)]
pub struct PieChart {
  names: Arc<[Arc<str>]>,
  series: ChartSeries,
  colors: Vec<BackgroundColor>,
  donut: bool,
  legend_value: PieLegendValue,
  share_text: Option<ShareTextFn>,
  total_text: Option<Arc<str>>,
  props: ChartProps,
}

impl PieChart {
  /// A pie of `series`, a value for each of the slices `names`, in their order.
  pub fn new<S: AsRef<str>>(names: impl IntoIterator<Item = S>, series: ChartSeries) -> Self {
    Self {
      names: names.into_iter().map(|name| Arc::from(name.as_ref())).collect(),
      series,
      colors: Vec::new(),
      donut: false,
      legend_value: PieLegendValue::Share,
      share_text: None,
      total_text: None,
      props: ChartProps::default(),
    }
  }

  /// Whether it is a donut: a ring with a hole of 0.66 of its radius, as the board's Donut Chart, `Md` unless told a
  /// size. A pie, `Sm`, unless told.
  pub fn donut(mut self, donut: bool) -> Self {
    self.donut = donut;
    self
  }

  /// The color of each slice, in their order, a role of the palette or a color, in place of the theme's chart colors:
  /// for more slices than the five chart colors, or a slice whose color means something.
  pub fn colors<C: Into<BackgroundColor>>(mut self, colors: impl IntoIterator<Item = C>) -> Self {
    self.colors = colors.into_iter().map(Into::into).collect();
    self
  }

  /// What each item of the legend says after its slice's name: its share unless told.
  pub fn legend_value(mut self, value: PieLegendValue) -> Self {
    self.legend_value = value;
    self
  }

  /// How a share, from 0 to 1, is written in the legend and to tooling: a whole percent unless told.
  pub fn share_text(mut self, text: impl Fn(f64) -> String + Send + Sync + 'static) -> Self {
    self.share_text = Some(Arc::new(text));
    self
  }

  /// The words a donut writes in its hole under the total of the slices shown (written with `value_text`): what the
  /// total is. Nothing in the hole unless told, as the board's; a pie has no hole to write in.
  pub fn total_text(mut self, text: &str) -> Self {
    self.total_text = Some(Arc::from(text));
    self
  }

  crate::chart_props!();

  /// The size of the chart unless told: the board's pie is `Sm`, its donut `Md`.
  fn default_size(&self) -> ChartSize {
    if self.donut { ChartSize::Md } else { ChartSize::Sm }
  }

  /// The frame of the chart in `theme`: what `build` and `mount` render.
  fn frame(&self, theme: &Theme) -> ChartFrame {
    let values: Vec<f64> = (0..self.names.len())
      .map(|index| self.series.values().get(index).copied().unwrap_or(0.0))
      .collect();
    // Each slice is a series of its own to the frame: the legend's item, the tooltip's row, its color.
    let slices: Vec<ChartSeries> = self
      .names
      .iter()
      .enumerate()
      .map(|(index, name)| {
        let slice = ChartSeries::new(name, [values[index]]);
        match self.colors.get(index) {
          Some(color) => slice.color(color.clone()),
          None => slice,
        }
      })
      .collect();
    let shown: Vec<bool> = slices.iter().map(|slice| !self.props.is_hidden(slice)).collect();
    let size = self.props.size.unwrap_or(self.default_size());
    let mut plot = PiePlot {
      names: self.names.clone(),
      series: Arc::from(self.series.name()),
      values: values.into(),
      shown: shown.into(),
      donut: self.donut,
      total_text: self.total_text.clone(),
      value_text: self.props.value_text(),
      share_text: self.share_text.clone().unwrap_or_else(|| Arc::new(default_share_text)),
    };
    let empty = plot.is_empty();
    let case = self
      .props
      .case(self.default_size(), empty)
      .state(PieChartState::Donut, self.donut);
    let recipe = theme
      .resolve::<PieChart>(&case, Some(&self.props.rules))
      .owned_by(self.props.id.as_ref());
    let side = self.props.plot_height(&recipe);
    plot.total_text = plot.total_text.filter(|_| self.donut);
    let legend: Vec<String> = (0..self.names.len())
      .map(|index| match self.legend_value {
        PieLegendValue::Share => plot
          .share(index)
          .map(|share| (plot.share_text)(share))
          .unwrap_or_default(),
        PieLegendValue::Value => (plot.value_text)(plot.values[index]),
        PieLegendValue::None => String::new(),
      })
      .collect();
    // The legend names the slices however many there are, unless the app turns it off.
    let mut props = self.props.clone();
    props.legend = Some(props.legend.unwrap_or(true));
    props
      .frame(recipe, Arc::new(plot), slices, theme.renderer::<PieChart>())
      .plot_width(side)
      .legend_beside(legend, size)
  }

  /// Builds the still chart in `theme`: drawn and read by tooling, with nothing that follows the pointer or the keys.
  pub fn build(self, theme: &Theme) -> Element {
    self.frame(theme).build()
  }

  /// The chart mounted where it is declared: the slice under the pointer or the focus picked out with its tooltip,
  /// the arrows round the slices, and `on_select` told a press.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.props.key.clone();
    ChartFrame::mount(ctx, key, move |theme| self.frame(theme))
  }
}

impl From<PieChart> for Element {
  fn from(chart: PieChart) -> Self {
    chart.build(&current_theme())
  }
}

/// What a pie's plot draws: the slices shown, from the top and clockwise.
#[derive(Clone)]
struct PiePlot {
  names: Arc<[Arc<str>]>,
  series: Arc<str>,
  values: Arc<[f64]>,
  shown: Arc<[bool]>,
  donut: bool,
  total_text: Option<Arc<str>>,
  value_text: ValueTextFn,
  share_text: ShareTextFn,
}

impl PiePlot {
  /// The value a slice is drawn as: none where it is hidden, not a number, or under zero.
  fn drawn(&self, index: usize) -> f64 {
    match self.values.get(index) {
      Some(value) if self.shown[index] && value.is_finite() && *value > 0.0 => *value,
      _ => 0.0,
    }
  }

  fn total(&self) -> f64 {
    (0..self.values.len()).map(|index| self.drawn(index)).sum()
  }

  /// The share of the whole a slice shown is: `None` where it is hidden or there is no whole.
  fn share(&self, index: usize) -> Option<f64> {
    let total = self.total();
    (self.shown.get(index).copied().unwrap_or(false) && total > 0.0).then(|| self.drawn(index) / total)
  }

  /// Where a slice starts and ends, in radians clockwise from the right, from the top: `None` where it is not drawn.
  fn angles(&self, index: usize) -> Option<(f32, f32)> {
    let total = self.total();
    if total <= 0.0 || self.drawn(index) <= 0.0 {
      return None;
    }
    let before: f64 = (0..index).map(|other| self.drawn(other)).sum();
    let start = -PI / 2.0 + (before / total) as f32 * TAU;
    let end = -PI / 2.0 + ((before + self.drawn(index)) / total) as f32 * TAU;
    Some((start, end))
  }

  /// Where every slice starts and ends, from the top and clockwise: one of no width (closed) where it is not drawn, at
  /// the place it would open from. What a change moves the slices between.
  fn spans(&self) -> Vec<(f32, f32)> {
    let total = self.total();
    let mut at = -PI / 2.0;
    (0..self.values.len())
      .map(|index| {
        let sweep = if total > 0.0 {
          (self.drawn(index) / total) as f32 * TAU
        } else {
          0.0
        };
        let span = (at, at + sweep);
        at += sweep;
        span
      })
      .collect()
  }

  /// Draws the slices at `spans` (by their index; a span of no width is not drawn), the one of `active` in its color
  /// and the others giving way to it, with the lines between them.
  fn paint_spans(
    &self,
    canvas: &Context2D,
    area: ChartArea,
    look: &ChartLook,
    active: Option<usize>,
    spans: &[(f32, f32)],
  ) {
    let faded = opacity(look, PieChartPart::Slice);
    let drawn: Vec<usize> = (0..spans.len())
      .filter(|index| spans[*index].1 - spans[*index].0 > 1e-4)
      .collect();
    for index in &drawn {
      let (start, end) = spans[*index];
      let color = look.series(*index);
      canvas.set_fill_style(match active {
        Some(active) if active != *index => color.with_opacity(color.a() as f32 / 255.0 * faded),
        _ => color,
      });
      self.slice_path(canvas, area, start, end);
      canvas.fill();
    }
    // A pie's slices stand apart by a line of the card's color, as the board draws them; a slice too narrow at the
    // rim to keep its color between two lines joins its neighbour without one.
    if self.donut || drawn.len() < 2 {
      return;
    }
    let (cx, cy, radius, _) = self.geometry(area);
    let wide = |index: usize| (spans[index].1 - spans[index].0) * radius > EDGE * 3.0;
    canvas.set_stroke_style(look.color(PieChartPart::Edge));
    canvas.set_line_width(EDGE);
    canvas.set_line_cap(LineCap::Butt);
    for (place, index) in drawn.iter().enumerate() {
      let before = drawn[(place + drawn.len() - 1) % drawn.len()];
      if !wide(*index) || !wide(before) {
        continue;
      }
      let start = spans[*index].0;
      canvas.begin_path();
      canvas.move_to(cx, cy);
      canvas.line_to(cx + radius * start.cos(), cy + radius * start.sin());
      canvas.stroke();
    }
  }

  /// The center and the radii of the ring: the outer, and the hole's (none for a pie).
  fn geometry(&self, area: ChartArea) -> (f32, f32, f32, f32) {
    let side = area.width.min(area.height);
    let radius = (side / 2.0).max(0.0);
    let inner = if self.donut { radius * HOLE } else { 0.0 };
    (area.width / 2.0, area.height / 2.0, radius, inner)
  }

  /// The middle of a slice: where its tooltip points and tooling finds it.
  fn middle(&self, area: ChartArea, index: usize) -> Option<(f32, f32)> {
    let (start, end) = self.angles(index)?;
    let (cx, cy, radius, inner) = self.geometry(area);
    // A slice that is the whole of a pie has its middle at the center; a donut's is in the middle of its ring.
    let at = if self.donut {
      (radius + inner) / 2.0
    } else if end - start >= TAU - 1e-4 {
      0.0
    } else {
      radius * 0.6
    };
    let angle = (start + end) / 2.0;
    Some((cx + at * angle.cos(), cy + at * angle.sin()))
  }

  fn slice_path(&self, canvas: &Context2D, area: ChartArea, start: f32, end: f32) {
    let (cx, cy, radius, inner) = self.geometry(area);
    canvas.begin_path();
    if inner > 0.0 {
      let _ = canvas.arc(cx, cy, radius, start, end, ArcDirection::Clockwise);
      let _ = canvas.arc(cx, cy, inner, end, start, ArcDirection::CounterClockwise);
    } else {
      canvas.move_to(cx, cy);
      let _ = canvas.arc(cx, cy, radius, start, end, ArcDirection::Clockwise);
    }
    canvas.close_path();
  }
}

/// The opacity of a part at rest, or 1.
fn opacity(look: &ChartLook, part: PieChartPart) -> f32 {
  match look.part(part).rest().opacity {
    Some(Opacity::Value(value)) => value,
    _ => 1.0,
  }
}

impl ChartPlot for PiePlot {
  fn paint(&self, canvas: &Context2D, area: ChartArea, look: &ChartLook, active: Option<ChartPoint>) {
    let active = active
      .filter(|point| self.angles(point.index).is_some())
      .map(|point| point.index);
    self.paint_spans(canvas, area, look, active, &self.spans());
  }

  /// The slices sweep in from the top, clockwise, as the pie first shows; on a change each slice's ends move from
  /// where they were to where they are: a hidden slice closes, and the others take its room.
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
    let to = self.spans();
    let top = -PI / 2.0;
    let spans: Vec<(f32, f32)> = match motion.from_as::<PiePlot>() {
      None => to
        .iter()
        .map(|(start, end)| (motion.lerp(top, *start), motion.lerp(top, *end)))
        .collect(),
      Some(from) => {
        let was = from.spans();
        // A slice that was not there opens at the end of the whole; one that is gone closes there.
        let end = top + TAU;
        (0..to.len().max(was.len()))
          .map(|index| {
            let (from, to) = (
              was.get(index).copied().unwrap_or((end, end)),
              to.get(index).copied().unwrap_or((end, end)),
            );
            (motion.lerp(from.0, to.0), motion.lerp(from.1, to.1))
          })
          .collect()
      }
    };
    let active = active.map(|point| point.index).filter(|index| *index < to.len());
    self.paint_spans(canvas, area, look, active, &spans);
  }

  fn point_at(&self, area: ChartArea, x: f32, y: f32) -> Option<ChartPoint> {
    let (cx, cy, radius, inner) = self.geometry(area);
    let (dx, dy) = (x - cx, y - cy);
    let distance = (dx * dx + dy * dy).sqrt();
    if distance > radius + 2.0 || distance < inner - 2.0 {
      return None;
    }
    // The angle clockwise from the top, as the slices run.
    let mut angle = dy.atan2(dx);
    if angle < -PI / 2.0 {
      angle += TAU;
    }
    (0..self.values.len())
      .find(|index| {
        self
          .angles(*index)
          .is_some_and(|(start, end)| angle >= start && angle < end)
      })
      .map(|index| ChartPoint::new(0, index))
  }

  fn anchor(&self, area: ChartArea, point: ChartPoint) -> Option<(f32, f32)> {
    self.middle(area, point.index).or_else(|| {
      let (cx, cy, ..) = self.geometry(area);
      Some((cx, cy))
    })
  }

  fn counts(&self) -> Vec<usize> {
    vec![self.values.len()]
  }

  fn tip(&self, point: ChartPoint) -> ChartTip {
    ChartTip {
      title: self.series.to_string(),
      rows: self
        .values
        .get(point.index)
        .map(|value| (point.index, *value))
        .into_iter()
        .collect(),
    }
  }

  fn items(&self, area: ChartArea, look: &ChartLook) -> Vec<CanvasItem> {
    let (.., radius, inner) = self.geometry(area);
    (0..self.values.len())
      .filter_map(|index| {
        let (x, y) = self.middle(area, index)?;
        let share = self.share(index).map(|share| (self.share_text)(share));
        let value = (look.value_text)(self.values[index]);
        let value = match share {
          Some(share) => format!("{value}, {share}"),
          None => value,
        };
        Some(
          CanvasItem::point(
            format!("slice-{index}"),
            "slice",
            x,
            y,
            ((radius - inner) / 2.0).max(4.0),
          )
          .label(self.names[index].to_string())
          .value(value),
        )
      })
      .collect()
  }

  fn labels(&self, area: ChartArea, look: &ChartLook) -> Vec<ChartLabel> {
    let Some(text) = &self.total_text else {
      return Vec::new();
    };
    if self.is_empty() {
      return Vec::new();
    }
    let (cx, cy, ..) = self.geometry(area);
    vec![
      ChartLabel::new(cx, cy + 1.0, (look.value_text)(self.total()), (-0.5, -1.0)).part(PieChartPart::Total),
      ChartLabel::new(cx, cy + 1.0, text.to_string(), (-0.5, 0.0)).part(PieChartPart::TotalLabel),
    ]
  }

  fn signature(&self) -> u64 {
    signature_of((
      self.names.iter().map(|name| name.to_string()).collect::<Vec<_>>(),
      self.values.iter().map(|value| bits(*value)).collect::<Vec<_>>(),
      self.shown.to_vec(),
      self.donut,
    ))
  }

  fn is_empty(&self) -> bool {
    self.total() <= 0.0
  }
}

impl crate::ui::Styled for PieChart {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for PieChart {
  const NAME: &'static str = "pie_chart";
  type Renderer = ChartRenderer;

  fn parts() -> crate::ui::Declared {
    PieChartPart::declared()
  }

  fn states() -> crate::ui::Declared {
    PieChartState::declared()
  }

  /// The chart's frame ([`chart_recipe`]), `Sm` for a pie and `Md` for a donut as the board has them, a plot of 96
  /// in either: the slices with a line of 1.5 of the card's color on each side of the line between two of them (none
  /// in a donut), the slices that give way to the one under the pointer or the focus at 0.45; in a donut's hole the
  /// total 15/600 in the card's foreground over its words, mono 10 in the muted foreground.
  fn recipe() -> Recipe {
    chart_recipe()
      .base(ChartPart::Plot, |s| s.height(96.0))
      .size(ChartSize::Sm, ChartPart::Plot, |s| s.height(96.0))
      .base(PieChartPart::Slice, |s| s.opacity(0.45))
      .base(PieChartPart::Edge, |s| s.background(PaletteColor::Card))
      .base(PieChartPart::Total, |s| {
        s.font_size(15.0)
          .weight(FontWeight::SemiBold)
          .foreground(PaletteColor::CardForeground)
      })
      .base(PieChartPart::TotalLabel, |s| {
        s.font_family(super::part_style::FontFamily::Mono)
          .font_size(10.0)
          .foreground(PaletteColor::MutedForeground)
      })
  }
}
