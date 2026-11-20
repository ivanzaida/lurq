//! A sparkline: a series as a line with no axes, small enough to stand beside a number. The frames of the last
//! minute, the builds of the week.

use std::{hash::Hasher, sync::Arc};

use super::{
  meter::number,
  names,
  part_style::{Length, PartStyle},
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{PaletteColor, Theme},
  canvas::{CanvasItem, LineCap, LineJoin},
  components::Canvas,
  node::{BackgroundColor, Element, color::Color, dimension::Dimension},
};

/// The parts of a sparkline a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SparklinePart {
  /// The canvas the line is drawn on: its width and its height.
  Root,
  /// The line: drawn in its part's background, as wide as its part's width.
  Line,
  /// The line at 0, where the values are on both sides of it: drawn in its part's background, as wide as its
  /// part's width.
  Baseline,
  /// The mark of a sparkline of one value: as wide as its part's width, in its part's background.
  Point,
}

names!(SparklinePart {
  Root => "root",
  Line => "line",
  Baseline => "baseline",
  Point => "point",
});

/// What builds a sparkline's nodes: a theme's in place of the default one.
pub type SparklineRenderer = Arc<dyn Fn(SparklineView) -> Element + Send + Sync>;

type SummaryText = Arc<dyn Fn(&[f64]) -> String + Send + Sync>;

/// A sparkline: a series as a line with no axes, small enough to stand beside a number. The frames per second of
/// the last minute, the builds of the week, a price over a day.
///
/// ```
/// use lurq::{app::theme::PaletteColor, ui::Sparkline};
///
/// let frames = Sparkline::new([118.0, 124.0, 121.0, 133.0, 129.0, 141.0, 138.0, 144.0])
///   .color(PaletteColor::Success)
///   .label("Frames per second, the last minute");
/// ```
///
/// Its values are drawn from the first at the start to the last at the end, the least at the bottom and the
/// greatest at the top unless [`min`](Self::min) and [`max`](Self::max) say another range, as a line of straight
/// steps with round joins. Where its values are on both sides of 0 a line at 0 is drawn under it; a sparkline of
/// one value is a dot in the middle, and one of none draws nothing. It is drawn on the canvas, sharp at every
/// scale, and drawn again when its values, its size, its scale or the theme change.
///
/// It is a picture to tooling: an `img` called by its [`label`](Self::label) and described by what its values are,
/// which [`summary_text`](Self::summary_text) writes ("8 values from 118 to 144, the last 144"); every value is a
/// point of the canvas a reader can go through.
pub struct Sparkline {
  values: Vec<f64>,
  min: Option<f64>,
  max: Option<f64>,
  color: Option<BackgroundColor>,
  label: Option<Arc<str>>,
  summary_text: Option<SummaryText>,
  empty_text: Option<Arc<str>>,
  width: Option<Length>,
  height: Option<Length>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Sparkline {
  /// A sparkline of `values`, the first at the start.
  pub fn new(values: impl IntoIterator<Item = impl Into<f64>>) -> Self {
    Self {
      values: values.into_iter().map(Into::into).collect(),
      min: None,
      max: None,
      color: None,
      label: None,
      summary_text: None,
      empty_text: None,
      width: None,
      height: None,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// The value at the bottom: the least of the values unless given.
  pub fn min(mut self, min: impl Into<f64>) -> Self {
    self.min = Some(min.into());
    self
  }

  /// The value at the top: the greatest of the values unless given.
  pub fn max(mut self, max: impl Into<f64>) -> Self {
    self.max = Some(max.into());
    self
  }

  /// The color the line is drawn in, a role of the palette or a color, as a chart's series takes it: the first of the
  /// chart colors, `Chart1`, unless given; a tone where the series is good or bad (the board's frames are `Success`).
  pub fn color(mut self, color: impl Into<BackgroundColor>) -> Self {
    self.color = Some(color.into());
    self
  }

  /// What the sparkline shows, to tooling: "Frames per second, the last minute".
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Writes what the values are, which tooling reads as the picture's description: `|values| ..`. Not given, "8
  /// values from 118 to 144, the last 144".
  pub fn summary_text(mut self, text: impl Fn(&[f64]) -> String + Send + Sync + 'static) -> Self {
    self.summary_text = Some(Arc::new(text));
    self
  }

  /// What a sparkline of no values says to tooling: "No data" when not given.
  pub fn empty_text(mut self, text: &str) -> Self {
    self.empty_text = Some(Arc::from(text));
    self
  }

  /// Its width, the board's 80 unless given.
  pub fn width(mut self, width: impl Into<Length>) -> Self {
    self.width = Some(width.into());
    self
  }

  /// Its height, the board's 28 unless given.
  pub fn height(mut self, height: impl Into<Length>) -> Self {
    self.height = Some(height.into());
    self
  }

  /// Puts the sparkline in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this sparkline over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this sparkline.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of its canvas.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the sparkline in `theme`. Converting a sparkline into an element does this in the theme of what is
  /// being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let view = self.view(theme);
    match theme.renderer::<Sparkline>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  fn view(mut self, theme: &Theme) -> SparklineView {
    if let Some(color) = self.color.take() {
      self.rules = self.rules.base(SparklinePart::Line, move |s| s.background(color));
    }
    if let Some(width) = self.width.take() {
      self.rules = self.rules.base(SparklinePart::Root, move |s| s.width(width));
    }
    if let Some(height) = self.height.take() {
      self.rules = self.rules.base(SparklinePart::Root, move |s| s.height(height));
    }
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    };
    let values: Vec<f64> = self.values.iter().copied().filter(|value| value.is_finite()).collect();
    let least = values.iter().copied().fold(f64::INFINITY, f64::min);
    let most = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let summary = if values.is_empty() {
      self.empty_text.map_or_else(
        || super::current_locale().data.no_data.to_string(),
        |text| text.to_string(),
      )
    } else {
      match &self.summary_text {
        Some(text) => text(&values),
        None => {
          let locale = super::current_locale();
          crate::core::fill(
            locale.form(&locale.data.values_summary, values.len()),
            &[
              ("count", &values.len().to_string()),
              ("least", &number(least)),
              ("most", &number(most)),
              ("last", &number(*values.last().unwrap_or(&0.0))),
            ],
          )
        }
      }
    };
    SparklineView {
      min: self.min.unwrap_or(least),
      max: self.max.unwrap_or(most),
      values,
      label: self.label,
      summary,
      recipe: theme
        .resolve::<Sparkline>(&case, Some(&self.rules))
        .owned_by(self.id.as_ref()),
      id: self.id,
      key: self.key,
    }
  }
}

impl From<Sparkline> for Element {
  fn from(sparkline: Sparkline) -> Self {
    sparkline.build(&current_theme())
  }
}

/// The key of a canvas's painting ([`Canvas::paint`]): a hash of what `fill` writes of what it draws.
pub(crate) fn paint_key(fill: impl FnOnce(&mut std::collections::hash_map::DefaultHasher)) -> u64 {
  let mut hash = std::collections::hash_map::DefaultHasher::new();
  fill(&mut hash);
  hash.finish()
}

/// The color of a part's background in `theme`, for what is drawn on a canvas.
pub(crate) fn color_of(part: &ResolvedPart, theme: &Theme) -> Color {
  match &part.rest().background {
    Some(BackgroundColor::Color(color)) => *color,
    Some(BackgroundColor::Palette(role)) => theme.palette_color(role.clone()),
    _ => theme.palette_color(PaletteColor::Foreground),
  }
}

/// The width of a part in pixels, for a line drawn on a canvas.
pub(crate) fn width_of(part: &ResolvedPart, theme: &Theme, otherwise: f32) -> f32 {
  match part.rest().width.and_then(|width| width.resolve(theme)) {
    Some(Dimension::Px(px)) => px,
    _ => otherwise,
  }
}

/// A sparkline as a renderer sees it.
pub struct SparklineView {
  /// The values, the first at the start; those that are not numbers left out.
  pub values: Vec<f64>,
  /// The value at the bottom and the one at the top.
  pub min: f64,
  pub max: f64,
  /// What it shows, to tooling.
  pub label: Option<Arc<str>>,
  /// What its values are, to tooling.
  pub summary: String,
  recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl SparklineView {
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// How the default renderer builds a sparkline: a canvas in the style of its root that paints the line, the
  /// line at 0 or the dot, and gives each value a point a reader can go through; an `img` to tooling.
  pub fn render(self) -> Element {
    let theme = self.theme();
    let line = self.part(SparklinePart::Line);
    let baseline = self.part(SparklinePart::Baseline);
    let point = self.part(SparklinePart::Point);
    let (stroke, stroke_width) = (color_of(&line, theme), width_of(&line, theme, 1.75));
    let (zero, zero_width) = (color_of(&baseline, theme), width_of(&baseline, theme, 1.0));
    let (dot, dot_width) = (color_of(&point, theme), width_of(&point, theme, 4.0));
    let (values, min, max) = (self.values.clone(), self.min, self.max);
    // Painted again only when what it draws changes: its values, their range, its colors and its widths.
    let key = paint_key(|hash| {
      for value in &values {
        hash.write_u64(value.to_bits());
      }
      for number in [min, max] {
        hash.write_u64(number.to_bits());
      }
      for color in [stroke, zero, dot] {
        hash.write(&[color.r(), color.g(), color.b(), color.a()]);
      }
      for width in [stroke_width, zero_width, dot_width] {
        hash.write_u32(width.to_bits());
      }
    });
    let mut canvas = Canvas::new()
      .part(&self.part(SparklinePart::Root))
      .describe("a11y_role", "img")
      .describe("a11y_description", self.summary.clone())
      .paint(key, move |canvas| {
        let (draw, metrics) = (canvas.context_2d(), canvas.metrics());
        draw.reset();
        let (width, height) = (metrics.size.width, metrics.size.height);
        // The line keeps inside the canvas: its half width at the top and the bottom, and at the ends.
        let inset = stroke_width / 2.0;
        let span = (max - min).max(f64::EPSILON);
        let flat = max <= min;
        let y_of = |value: f64| -> f32 {
          if flat {
            return height / 2.0;
          }
          let share = ((value - min) / span).clamp(0.0, 1.0) as f32;
          inset + (height - 2.0 * inset) * (1.0 - share)
        };
        let x_of = |index: usize| -> f32 {
          if values.len() < 2 {
            return width / 2.0;
          }
          inset + (width - 2.0 * inset) * index as f32 / (values.len() - 1) as f32
        };
        if min < 0.0 && max > 0.0 {
          let y = (y_of(0.0) * metrics.scale_factor).round() / metrics.scale_factor;
          draw.set_fill_style(zero);
          draw.fill_rect(0.0, y - zero_width / 2.0, width, zero_width);
        }
        let mut items = Vec::with_capacity(values.len());
        if values.len() == 1 {
          draw.set_fill_style(dot);
          draw.begin_path();
          let _ = draw.arc(
            x_of(0),
            y_of(values[0]),
            dot_width / 2.0,
            0.0,
            std::f32::consts::TAU,
            crate::canvas::ArcDirection::Clockwise,
          );
          draw.fill();
        } else if values.len() > 1 {
          draw.set_stroke_style(stroke);
          draw.set_line_width(stroke_width);
          draw.set_line_join(LineJoin::Round);
          draw.set_line_cap(LineCap::Round);
          draw.begin_path();
          for (index, value) in values.iter().enumerate() {
            let (x, y) = (x_of(index), y_of(*value));
            if index == 0 {
              draw.move_to(x, y);
            } else {
              draw.line_to(x, y);
            }
          }
          draw.stroke();
        }
        for (index, value) in values.iter().enumerate() {
          items
            .push(CanvasItem::point(index.to_string(), "point", x_of(index), y_of(*value), 4.0).value(number(*value)));
        }
        draw.canvas().set_items(items);
      });
    if let Some(label) = &self.label {
      canvas = canvas.describe("a11y_name", label.clone());
    }
    if let Some(id) = &self.id {
      canvas = canvas.id(id.clone());
    }
    if let Some(key) = &self.key {
      canvas = canvas.key(key.clone());
    }
    canvas.into()
  }
}

impl crate::ui::Styled for Sparkline {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for Sparkline {
  const NAME: &'static str = "sparkline";
  type Renderer = SparklineRenderer;

  fn parts() -> crate::ui::Declared {
    SparklinePart::declared()
  }

  /// The design's sparkline (`pencil/lurq-design-system.pen`, Sparkline, in Charts' Parts): a line of 80 by 28 of
  /// straight steps, 1.75 wide with round joins and ends, in a series' color (the board's is the success color,
  /// for frames that went up). The board draws nothing else of it: the line at 0 is 1 in the border's color, and a
  /// single value a dot of 4 in the line's color.
  fn recipe() -> Recipe {
    use SparklinePart::{Baseline, Line, Point, Root};
    Recipe::new()
      .base(Root, |s| s.width(80.0).height(28.0))
      .base(Line, |s| s.width(1.75).background(PaletteColor::Chart1))
      .base(Baseline, |s| s.width(1.0).background(PaletteColor::Border))
      .base(Point, |s| s.width(4.0).background(PaletteColor::Chart1))
  }
}
