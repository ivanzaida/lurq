//! A gauge: a value of a known range as the share of a half ring, its reading in it. The load of a processor, the
//! speed of a link, the share of a quota.

use std::{hash::Hasher, sync::Arc};

use super::{
  meter::{MeterVariant, number},
  names,
  part_style::{FontFamily, PartStyle},
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  sparkline::{color_of, paint_key, width_of},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{PaletteColor, Theme, TypographyStyle},
  canvas::{ArcDirection, CanvasItem, LineCap},
  components::{Canvas, Column, Row, Stack},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, dimension::Dimension},
};

/// The parts of a gauge a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GaugePart {
  /// The column of the arc and the range.
  Root,
  /// The canvas the half ring is drawn on: its width and its height.
  Arc,
  /// The whole half ring: drawn in its part's background, as thick as its part's width.
  Track,
  /// The share of the half ring the value takes: drawn in its part's background, as thick as the track.
  Value,
  /// The reading in the ring: "70%".
  Reading,
  /// The row under the ring: the least value, the label and the greatest value.
  Range,
  /// What the gauge measures: "CPU".
  Label,
  /// The least and the greatest value, at the ends of the range.
  Bound,
}

names!(GaugePart {
  Root => "root",
  Arc => "arc",
  Track => "track",
  Value => "value",
  Reading => "reading",
  Range => "range",
  Label => "label",
  Bound => "bound",
});

/// What builds a gauge's nodes: a theme's in place of the default one.
pub type GaugeRenderer = Arc<dyn Fn(GaugeView) -> Element + Send + Sync>;

type NumberText = Arc<dyn Fn(f64) -> String + Send + Sync>;

/// A gauge: a value of a known range as the share of a half ring, its reading in it. The load of a processor, the
/// speed of a link, the share of a quota.
///
/// ```
/// use lurq::ui::Gauge;
///
/// let cpu = Gauge::new(70).label("CPU");
/// // A temperature, bad past 80, its reading as the app writes it.
/// let heat = Gauge::new(86)
///   .min(20)
///   .max(100)
///   .low(60)
///   .high(80)
///   .label("GPU")
///   .value_text(|degrees| format!("{degrees}°"));
/// ```
///
/// The half ring runs from the least value at its start to the greatest at its end, and the value takes its share
/// from the start, in the first of the chart colors unless given another, or, where the gauge has
/// [`low`](Self::low) and [`high`](Self::high), in the tone of the part of the range the value is in, as a
/// [`Meter`](super::Meter) judges it. A value outside the range takes none or all of it; its reading says what it
/// is. The ring is drawn on the canvas, sharp at every scale; its reading, its label and its bounds are text.
///
/// To tooling it is a `meter` called by its label, with its value, its range and its reading.
pub struct Gauge {
  value: f64,
  min: f64,
  max: f64,
  low: Option<f64>,
  high: Option<f64>,
  optimum: Option<f64>,
  color: Option<crate::node::BackgroundColor>,
  label: Option<Arc<str>>,
  value_text: Option<NumberText>,
  bound_text: Option<NumberText>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Gauge {
  /// A gauge of `value`, in a range from 0 to 100 unless told otherwise.
  pub fn new(value: impl Into<f64>) -> Self {
    Self {
      value: value.into(),
      min: 0.0,
      max: 100.0,
      low: None,
      high: None,
      optimum: None,
      color: None,
      label: None,
      value_text: None,
      bound_text: None,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// The value.
  pub fn value(mut self, value: impl Into<f64>) -> Self {
    self.value = value.into();
    self
  }

  /// The least value, at the start of the ring: 0 unless given.
  pub fn min(mut self, min: impl Into<f64>) -> Self {
    self.min = min.into();
    self
  }

  /// The greatest value, at its end: 100 unless given.
  pub fn max(mut self, max: impl Into<f64>) -> Self {
    self.max = max.into();
    self
  }

  /// Where the low part of the range ends, as a [`Meter`](super::Meter)'s: with it or `high`, the value's share is
  /// in the tone of its part.
  pub fn low(mut self, low: impl Into<f64>) -> Self {
    self.low = Some(low.into());
    self
  }

  /// Where the high part of the range starts.
  pub fn high(mut self, high: impl Into<f64>) -> Self {
    self.high = Some(high.into());
    self
  }

  /// The best value: the part of the range it is in is good. The least value unless given.
  pub fn optimum(mut self, optimum: impl Into<f64>) -> Self {
    self.optimum = Some(optimum.into());
    self
  }

  /// The color of the value's share where the gauge has no parts, a role of the palette or a color: `Chart1` unless
  /// given.
  pub fn color(mut self, color: impl Into<crate::node::BackgroundColor>) -> Self {
    self.color = Some(color.into());
    self
  }

  /// What the gauge measures, under the ring between its bounds: "CPU". Its name to tooling.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Writes the reading as the app does: `|degrees| format!("{degrees}°")`. Not given, the value's share of the
  /// range in percent, "70%".
  pub fn value_text(mut self, text: impl Fn(f64) -> String + Send + Sync + 'static) -> Self {
    self.value_text = Some(Arc::new(text));
    self
  }

  /// Writes the bounds under the ring as the app does. Not given, as they are: "0", "100".
  pub fn bound_text(mut self, text: impl Fn(f64) -> String + Send + Sync + 'static) -> Self {
    self.bound_text = Some(Arc::new(text));
    self
  }

  /// Puts the gauge in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this gauge over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this gauge.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the gauge's root.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The tone of the value, where the gauge has parts.
  fn tone(&self) -> Option<Name> {
    if self.low.is_none() && self.high.is_none() {
      return None;
    }
    let mut meter = super::Meter::new(self.value).min(self.min).max(self.max);
    if let Some(low) = self.low {
      meter = meter.low(low);
    }
    if let Some(high) = self.high {
      meter = meter.high(high);
    }
    if let Some(optimum) = self.optimum {
      meter = meter.optimum(optimum);
    }
    Some(meter.tone_of_value())
  }

  /// Builds the gauge in `theme`. Converting a gauge into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let view = self.view(theme);
    match theme.renderer::<Gauge>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  fn view(mut self, theme: &Theme) -> GaugeView {
    if let Some(color) = self.color.take() {
      self.rules = self.rules.base(GaugePart::Value, move |s| s.background(color));
    }
    let tone = self.tone();
    let case = Case {
      variant: tone,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    };
    let span = self.max - self.min;
    let fraction = if span > 0.0 {
      ((self.value - self.min) / span).clamp(0.0, 1.0)
    } else {
      0.0
    };
    let reading = match &self.value_text {
      Some(text) => text(self.value),
      None => {
        let percent = if span > 0.0 {
          (self.value - self.min) / span * 100.0
        } else {
          0.0
        };
        format!("{}%", number(percent.round()))
      }
    };
    let bound = |value: f64| match &self.bound_text {
      Some(text) => text(value),
      None => number(value),
    };
    GaugeView {
      value: self.value,
      min: self.min,
      max: self.max,
      fraction: fraction as f32,
      reading,
      min_text: bound(self.min),
      max_text: bound(self.max),
      label: self.label.clone(),
      recipe: theme.resolve::<Gauge>(&case, Some(&self.rules)),
      id: self.id,
      key: self.key,
    }
  }
}

impl From<Gauge> for Element {
  fn from(gauge: Gauge) -> Self {
    gauge.build(&current_theme())
  }
}

/// A gauge as a renderer sees it.
pub struct GaugeView {
  /// The value, and the least and greatest of its range.
  pub value: f64,
  pub min: f64,
  pub max: f64,
  /// How much of the ring the value takes, from 0 to 1.
  pub fraction: f32,
  /// The reading, as it is written.
  pub reading: String,
  /// The bounds, as they are written.
  pub min_text: String,
  pub max_text: String,
  /// What the gauge measures.
  pub label: Option<Arc<str>>,
  recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl GaugeView {
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// A text of `part`, in the color of its part.
  fn text(&self, part: GaugePart, content: &str) -> Row {
    let part = self.part(part);
    Row::new().part(&part).child(part.text(content).nowrap())
  }

  /// The half ring with the reading in it: a canvas that paints the track and the value's share, under the
  /// reading.
  pub fn arc(&self) -> Stack {
    let theme = self.theme();
    let track = self.part(GaugePart::Track);
    let share = self.part(GaugePart::Value);
    let (rail, thickness) = (color_of(&track, theme), width_of(&track, theme, 15.4));
    let fill = color_of(&share, theme);
    let (fraction, reading) = (self.fraction, self.reading.clone());
    // Painted again only when what it draws changes: the value's share, its colors and the ring's thickness.
    let key = paint_key(|hash| {
      hash.write_u32(fraction.to_bits());
      hash.write_u32(thickness.to_bits());
      for color in [rail, fill] {
        hash.write(&[color.r(), color.g(), color.b(), color.a()]);
      }
      hash.write(reading.as_bytes());
    });
    let canvas = Canvas::new()
      .part(&self.part(GaugePart::Arc))
      .paint(key, move |canvas| {
        let (draw, metrics) = (canvas.context_2d(), canvas.metrics());
        draw.reset();
        let (width, height) = (metrics.size.width, metrics.size.height);
        // The ring's middle line: its centre at the middle of the bottom, as wide as the canvas.
        let (cx, cy) = (width / 2.0, height.min(width / 2.0));
        let radius = (width / 2.0 - thickness / 2.0).max(0.0);
        let start = std::f32::consts::PI;
        draw.set_line_width(thickness);
        draw.set_line_cap(LineCap::Butt);
        draw.set_stroke_style(rail);
        draw.begin_path();
        let _ = draw.arc(cx, cy, radius, start, start * 2.0, ArcDirection::Clockwise);
        draw.stroke();
        if fraction > 0.0 {
          draw.set_stroke_style(fill);
          draw.begin_path();
          let _ = draw.arc(cx, cy, radius, start, start + start * fraction, ArcDirection::Clockwise);
          draw.stroke();
        }
        let end = start + start * fraction;
        draw.canvas().set_items([CanvasItem::point(
          "value",
          "value",
          cx + radius * end.cos(),
          cy + radius * end.sin(),
          thickness / 2.0,
        )
        .value(reading.clone())]);
      });
    let reading = self.part(GaugePart::Reading);
    // The reading stands over the canvas, as large as it: a stack's percentages do not reach its children.
    let arc = self.part(GaugePart::Arc);
    let side = |length: Option<super::part_style::Length>| match length.and_then(|length| length.resolve(theme)) {
      Some(dimension) => dimension,
      None => Dimension::Pct(100.0),
    };
    let (width, height) = (side(arc.rest().width), side(arc.rest().height));
    Stack::new().child(canvas).child(
      Column::new()
        .size(width, height)
        .align_items(Alignment::Center)
        .justify(Justify::End)
        .part(&reading)
        .child(reading.text(&self.reading).nowrap()),
    )
  }

  /// The least value, the label and the greatest value, across the foot of the ring.
  pub fn range(&self) -> Row {
    let mut range = Row::new()
      .width(Dimension::Pct(100.0))
      .justify(Justify::SpaceBetween)
      .align_items(Alignment::Center)
      .part(&self.part(GaugePart::Range))
      .child(self.text(GaugePart::Bound, &self.min_text));
    if let Some(label) = &self.label {
      range = range.child(self.text(GaugePart::Label, label));
    }
    range.child(self.text(GaugePart::Bound, &self.max_text))
  }

  /// The root in the style of its part, a `meter` to tooling with its value, its range and its reading, holding
  /// `content`.
  pub fn root(&self, content: Vec<Element>) -> Column {
    let mut root = Column::new()
      .align_items(Alignment::Center)
      .part(&self.part(GaugePart::Root))
      .describe("a11y_role", "meter")
      .describe("a11y_valuenow", number(self.value))
      .describe("a11y_valuemin", number(self.min))
      .describe("a11y_valuemax", number(self.max))
      .describe("a11y_valuetext", self.reading.clone());
    if let Some(label) = &self.label {
      root = root.describe("a11y_name", label.clone());
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.with_children(content)
  }

  /// How the default renderer builds a gauge: the ring with its reading, over its range.
  pub fn render(self) -> Element {
    let content = vec![self.arc().into(), self.range().into()];
    self.root(content).into()
  }
}

impl Themed for Gauge {
  const NAME: &'static str = "gauge";
  type Renderer = GaugeRenderer;

  /// The design's gauge (`pencil/lurq-design-system.pen`, Gauge, in Slider & Progress' Meters): 140 wide, a half
  /// ring 140 across and 72 high whose inner edge is 0.78 of its outer, so 15.4 thick: the track in the secondary
  /// color, the value's share in the first of the chart colors, both with square ends. The reading is 20 at 600 in
  /// the foreground in the ring, its foot 10 over the ring's. Under the ring, 4 from it, the least value and the
  /// greatest at the ends, the mono type at 11 in the muted color, and the label between them, 12 at 500 in the
  /// foreground. Where the gauge has parts, the value's share is in the tone of its part, as a meter's segment.
  fn recipe() -> Recipe {
    use GaugePart::{Arc, Bound, Label, Range, Reading, Root, Track, Value};
    use PaletteColor::{Foreground, MutedForeground, Secondary};
    let mut recipe = Recipe::new()
      .base(Root, |s| s.width(140.0).gap(4.0))
      .base(Arc, |s| s.width(140.0).height(72.0))
      .base(Track, |s| s.width(15.4).background(Secondary))
      .base(Value, |s| s.background(PaletteColor::Chart1))
      .base(Reading, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(20.0)
          .weight(FontWeight::SemiBold)
          .foreground(Foreground)
          .padding_bottom(10.0)
      })
      .base(Range, |s| s.gap(4.0))
      .base(Label, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(12.0)
          .weight(FontWeight::Medium)
          .foreground(Foreground)
      })
      .base(Bound, |s| {
        s.font_family(FontFamily::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      });
    for variant in MeterVariant::all().iter().copied() {
      if variant == MeterVariant::Neutral {
        continue;
      }
      let role = match variant {
        MeterVariant::Info => PaletteColor::Info,
        MeterVariant::Success => PaletteColor::Success,
        MeterVariant::Warning => PaletteColor::Warning,
        _ => PaletteColor::Destructive,
      };
      recipe = recipe.when(Selector::new().variant(variant), Value, move |s| s.background(role));
    }
    recipe
  }
}
