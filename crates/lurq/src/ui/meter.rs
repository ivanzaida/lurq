//! A meter: how much of a known range a value takes, in segments, and whether that is good or not. The space a disk
//! has left, a battery's charge, the share of a plan used.

use std::sync::Arc;

use super::{
  names,
  part_style::{FontFamily, Length, PartStyle},
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{PaletteColor, Theme, TypographyStyle},
  components::{Column, Row},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, dimension::Dimension},
};

/// The tone of a meter's segments, or of a part of its range: whether a value there is good or not.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MeterVariant {
  /// A value with no judgment of its own: the primary color.
  #[default]
  Neutral,
  /// A value to know of.
  Info,
  /// A value that is good: a disk with room.
  Success,
  /// A value to look at: a disk running low.
  Warning,
  /// A value that is bad: a disk that is full.
  Destructive,
}

names!(MeterVariant {
  Neutral => "neutral",
  Info => "info",
  Success => "success",
  Warning => "warning",
  Destructive => "destructive",
});

impl MeterVariant {
  /// The variant called `name`, where it is one of these: a variant a theme adds is none of them.
  pub fn named(name: Name) -> Option<Self> {
    Self::all().iter().copied().find(|variant| Name::from(*variant) == name)
  }

  fn color(self) -> PaletteColor {
    match self {
      Self::Neutral => PaletteColor::Primary,
      Self::Info => PaletteColor::Info,
      Self::Success => PaletteColor::Success,
      Self::Warning => PaletteColor::Warning,
      Self::Destructive => PaletteColor::Destructive,
    }
  }
}

/// The parts of a meter a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MeterPart {
  /// The column of the header, the segments and the description.
  Root,
  /// The row of the label and the value.
  Header,
  /// What the meter measures: "Disk space".
  Label,
  /// The value as it is written: "96 of 128 GB".
  Value,
  /// The row of the segments.
  Segments,
  /// A segment: filled in the tone of its part of the range, or empty.
  Segment,
  /// The line under the segments, in the tone of the value where that is a warning.
  Description,
}

names!(MeterPart {
  Root => "root",
  Header => "header",
  Label => "label",
  Value => "value",
  Segments => "segments",
  Segment => "segment",
  Description => "description",
});

/// The states of a meter's parts a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MeterState {
  /// Of a segment: the value reaches it.
  Filled,
}

names!(MeterState {
  Filled => "filled",
});

/// What builds a meter's nodes: a theme's in place of the default one.
pub type MeterRenderer = Arc<dyn Fn(MeterView) -> Element + Send + Sync>;

type ValueText = Arc<dyn Fn(f64) -> String + Send + Sync>;

/// A meter: how much of a known range a value takes, in segments, and whether that is good or not. The space a disk
/// has used, a battery's charge, the share of a plan used.
///
/// ```
/// use lurq::ui::{Meter, MeterVariant};
///
/// // The board's: 96 of 128, good under 64, a warning to 96, bad past it.
/// let disk = Meter::new(96)
///   .max(128)
///   .low(64)
///   .high(96)
///   .label("Disk space")
///   .value_text(|used| format!("{used} of 128 GB"))
///   .description("Running low. Free up space soon.");
/// // A battery: the higher the better.
/// let battery = Meter::new(18).low(20).high(80).optimum(100).label("Battery");
/// // One tone, no zones.
/// let plan = Meter::new(0.4).max(1).variant(MeterVariant::Info).label("Seats used");
/// ```
///
/// Its range, from [`min`](Self::min) to [`max`](Self::max), is cut into [`segments`](Self::segments), and the value
/// fills those it reaches. Where it has [`low`](Self::low) and [`high`](Self::high) its range is three parts, below
/// `low`, between them and above `high`, and each filled segment is in the tone of the part it is in: the part the
/// [`optimum`](Self::optimum) is in is good (`Success`), the part next to it a warning (`Warning`), and the far one
/// bad (`Destructive`), as a web page's meter judges them. Without them every filled segment is in the
/// [`variant`](Self::variant)'s tone. A value outside the range fills none or all of them; its text says what it
/// is.
///
/// To tooling it is a `meter` called by its label, with its value, its range and its value text.
pub struct Meter {
  value: f64,
  min: f64,
  max: f64,
  low: Option<f64>,
  high: Option<f64>,
  optimum: Option<f64>,
  variant: Name,
  segments: usize,
  label: Option<Arc<str>>,
  description: Option<Arc<str>>,
  value_text: Option<ValueText>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Meter {
  /// A meter of `value`, in a range from 0 to 100 unless told otherwise.
  pub fn new(value: impl Into<f64>) -> Self {
    Self {
      value: value.into(),
      min: 0.0,
      max: 100.0,
      low: None,
      high: None,
      optimum: None,
      variant: MeterVariant::default().into(),
      segments: 16,
      label: None,
      description: None,
      value_text: None,
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

  /// The least value of the range, 0 unless given.
  pub fn min(mut self, min: impl Into<f64>) -> Self {
    self.min = min.into();
    self
  }

  /// The greatest value of the range, 100 unless given.
  pub fn max(mut self, max: impl Into<f64>) -> Self {
    self.max = max.into();
    self
  }

  /// Where the low part of the range ends: a value under it is in the low part.
  pub fn low(mut self, low: impl Into<f64>) -> Self {
    self.low = Some(low.into());
    self
  }

  /// Where the high part of the range starts: a value over it is in the high part.
  pub fn high(mut self, high: impl Into<f64>) -> Self {
    self.high = Some(high.into());
    self
  }

  /// The best value: the part of the range it is in is good. The least value unless given: a disk, the less used
  /// the better. A battery's is its greatest.
  pub fn optimum(mut self, optimum: impl Into<f64>) -> Self {
    self.optimum = Some(optimum.into());
    self
  }

  /// A [`MeterVariant`], or a variant a theme adds: the tone of every filled segment of a meter that has no
  /// `low` and no `high`.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// How many segments the range is cut into: the board's 16 unless given, at least 1.
  pub fn segments(mut self, segments: usize) -> Self {
    self.segments = segments.max(1);
    self
  }

  /// What the meter measures, over its segments: "Disk space". Its name to tooling.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// The line under the segments: what the value means, "Running low. Free up space soon." It is in the tone of
  /// the value where the value is in a part that is a warning or bad.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// Writes the value as the app does: `|used| format!("{used} of 128 GB")`. Not given, "96 of 128".
  pub fn value_text(mut self, text: impl Fn(f64) -> String + Send + Sync + 'static) -> Self {
    self.value_text = Some(Arc::new(text));
    self
  }

  /// Puts the meter in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this meter over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this meter.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the meter's root.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The tone of a value at `at`: the variant's where the meter has no parts, else that of the part it is in.
  fn tone_at(&self, at: f64) -> Name {
    if self.low.is_none() && self.high.is_none() {
      return self.variant;
    }
    let low = self.low.unwrap_or(self.min);
    let high = self.high.unwrap_or(self.max).max(low);
    let part = |value: f64| -> i32 {
      if value < low {
        0
      } else if value > high {
        2
      } else {
        1
      }
    };
    let best = part(self.optimum.unwrap_or(self.min));
    match (part(at) - best).abs() {
      0 => MeterVariant::Success,
      1 => MeterVariant::Warning,
      _ => MeterVariant::Destructive,
    }
    .into()
  }

  /// The tone of its value: the variant's where it has no parts, else that of the part the value is in.
  pub(crate) fn tone_of_value(&self) -> Name {
    self.tone_at(self.value)
  }

  /// Builds the meter in `theme`. Converting a meter into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let view = self.view(theme);
    match theme.renderer::<Meter>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  fn view(self, theme: &Theme) -> MeterView {
    let case = |variant: Option<Name>, filled: bool| {
      Case {
        variant,
        size: None,
        density: None,
        breakpoint: super::current_breakpoint(),
        states: self.states.clone(),
      }
      .state(MeterState::Filled, filled)
    };
    let span = self.max - self.min;
    let fraction = if span > 0.0 {
      ((self.value - self.min) / span).clamp(0.0, 1.0)
    } else {
      0.0
    };
    let count = self.segments;
    let filled = (fraction * count as f64).round() as usize;
    let segments = (0..count)
      .map(|index| {
        let middle = self.min + span * (index as f64 + 0.5) / count as f64;
        let tone = self.tone_at(middle);
        let on = index < filled;
        MeterSegment {
          tone,
          filled: on,
          recipe: theme.resolve::<Meter>(&case(Some(tone), on), Some(&self.rules)),
        }
      })
      .collect();
    let tone = self.tone_at(self.value);
    let max = self.max;
    let text = match &self.value_text {
      Some(text) => text(self.value),
      None => crate::core::fill(
        &super::current_locale().common.place_of,
        &[("at", &number(self.value)), ("total", &number(max))],
      ),
    };
    MeterView {
      segments,
      value: self.value,
      min: self.min,
      max: self.max,
      tone,
      value_text: text,
      label: self.label,
      description: self.description,
      recipe: theme.resolve::<Meter>(&case(Some(tone), false), Some(&self.rules)),
      id: self.id,
      key: self.key,
    }
  }
}

impl From<Meter> for Element {
  fn from(meter: Meter) -> Self {
    meter.build(&current_theme())
  }
}

/// `value` as it is written with no more decimals than it has, up to 2: 96, 0.4, 2.25.
pub(crate) fn number(value: f64) -> String {
  let text = format!("{value:.2}");
  let text = text.trim_end_matches('0').trim_end_matches('.');
  if text == "-0" { "0".to_owned() } else { text.to_owned() }
}

/// A segment of a meter as a renderer sees it.
pub struct MeterSegment {
  /// The tone of its part of the range.
  pub tone: Name,
  /// Whether the value reaches it.
  pub filled: bool,
  recipe: ResolvedRecipe,
}

impl MeterSegment {
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  /// The segment in the style of its part: a share of the row.
  pub fn render(&self) -> Element {
    Row::new().flex(1.0).part(&self.part(MeterPart::Segment)).into()
  }
}

/// A meter as a renderer sees it.
pub struct MeterView {
  /// The segments, from the least value up.
  pub segments: Vec<MeterSegment>,
  /// The value.
  pub value: f64,
  /// The least and the greatest value of the range.
  pub min: f64,
  pub max: f64,
  /// The tone of the value: that of the part of the range it is in, or the variant.
  pub tone: Name,
  /// The value as it is written.
  pub value_text: String,
  /// What the meter measures.
  pub label: Option<Arc<str>>,
  /// The line under the segments.
  pub description: Option<Arc<str>>,
  recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl MeterView {
  /// The style of a part, in the tone of the value.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// A text of `part`, in the color of its part.
  fn text(&self, part: MeterPart, content: &str) -> Row {
    let part = self.part(part);
    Row::new()
      .min_width(Dimension::Px(0.0))
      .part(&part)
      .child(part.text(content))
  }

  /// The label at the start and the value at the end, where it has a label; the value alone where it has none.
  pub fn header(&self) -> Row {
    let mut header = Row::new()
      .width(Dimension::Pct(100.0))
      .justify(Justify::SpaceBetween)
      .align_items(Alignment::Center)
      .part(&self.part(MeterPart::Header));
    if let Some(label) = &self.label {
      header = header.child(self.text(MeterPart::Label, label));
    }
    header.child(self.text(MeterPart::Value, &self.value_text))
  }

  /// The row of the segments.
  pub fn segments(&self) -> Row {
    Row::new()
      .width(Dimension::Pct(100.0))
      .part(&self.part(MeterPart::Segments))
      .with_children(self.segments.iter().map(MeterSegment::render))
  }

  /// The line under the segments, where it has one.
  pub fn description(&self) -> Option<Row> {
    let description = self.description.as_ref()?;
    let part = self.part(MeterPart::Description);
    Some(
      Row::new()
        .width(Dimension::Pct(100.0))
        .part(&part)
        .child(part.text(description).width(Dimension::Pct(100.0))),
    )
  }

  /// The root in the style of its part, a `meter` to tooling with its value, its range and its text, holding
  /// `content`.
  pub fn root(&self, content: Vec<Element>) -> Column {
    let mut root = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(MeterPart::Root))
      .describe("a11y_role", "meter")
      .describe("a11y_valuenow", number(self.value))
      .describe("a11y_valuemin", number(self.min))
      .describe("a11y_valuemax", number(self.max))
      .describe("a11y_valuetext", self.value_text.clone());
    if let Some(label) = &self.label {
      root = root.describe("a11y_name", label.clone());
    }
    if let Some(description) = &self.description {
      root = root.describe("a11y_description", description.clone());
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.with_children(content)
  }

  /// How the default renderer builds a meter: its header, its segments and its description, one under another.
  pub fn render(self) -> Element {
    let mut content: Vec<Element> = vec![self.header().into(), self.segments().into()];
    content.extend(self.description().map(Into::into));
    self.root(content).into()
  }
}

impl crate::ui::Styled for Meter {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for Meter {
  const NAME: &'static str = "meter";
  type Renderer = MeterRenderer;

  fn parts() -> crate::ui::Declared {
    MeterPart::declared()
  }

  fn states() -> crate::ui::Declared {
    MeterState::declared()
  }

  /// The design's segmented meter (`pencil/lurq-design-system.pen`, Meter, in Slider & Progress' Meters): 260 on
  /// the board and as wide as what holds it here, its header, its segments and its description 8 apart. The label
  /// is 13 at 500 in the foreground, the value the mono type at 11 in the muted color at the end of the row. The
  /// segments are 16, 8 high, 2 apart, with a radius of 2: a filled one in the tone of its part of the range, an
  /// empty one in the secondary color. The description is 12, in the tone of the value where that is a warning or
  /// bad (the board's is the warning's), and in the muted color where it is good.
  fn recipe() -> Recipe {
    use MeterPart::{Description, Header, Label, Root, Segment, Segments, Value};
    use MeterState::Filled;
    use PaletteColor::{Foreground, MutedForeground, Secondary};
    let mut recipe = Recipe::new()
      .base(Root, |s| s.width(Length::Fill).gap(8.0))
      .base(Header, |s| s.gap(8.0))
      .base(Label, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .weight(FontWeight::Medium)
          .foreground(Foreground)
      })
      .base(Value, |s| {
        s.font_family(FontFamily::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      .base(Segments, |s| s.gap(2.0))
      .base(Segment, |s| s.height(8.0).radius(2.0).background(Secondary))
      .base(Description, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(12.0)
          .foreground(MutedForeground)
      });
    for variant in MeterVariant::all().iter().copied() {
      recipe = recipe.when(Selector::new().variant(variant).state(Filled), Segment, move |s| {
        s.background(variant.color())
      });
    }
    recipe
      .variant(MeterVariant::Warning, Description, |s| {
        s.foreground(PaletteColor::Warning)
      })
      .variant(MeterVariant::Destructive, Description, |s| {
        s.foreground(PaletteColor::Destructive)
      })
  }
}
