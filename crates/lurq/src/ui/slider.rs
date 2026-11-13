//! A slider: a value picked along a track, between a least and a greatest.

use std::sync::Arc;

use super::{
  names,
  part_style::{Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{OpacityLevel, PaletteColor, RadiusSize, Theme},
  components,
  core::Signal,
  node::{CursorIcon, Element, SliderPartStyle, border, dimension::Dimension},
};

/// The parts of a slider a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SliderPart {
  /// The slider: what is dragged along, pressed and focused.
  Root,
  /// The rail across the slider, from the least value to the greatest.
  Track,
  /// The part of the track from its start to the thumb: how much of the range the value is.
  Range,
  /// What is dragged: at the value, along the track.
  Thumb,
}

names!(SliderPart {
  Root => "root",
  Track => "track",
  Range => "range",
  Thumb => "thumb",
});

/// The states of a slider a recipe styles, besides what the pointer and the
/// keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SliderState {
  /// It takes no input.
  Disabled,
}

names!(SliderState {
  Disabled => "disabled",
});

/// What builds a slider's nodes: a theme's in place of the default one.
pub type SliderRenderer = Arc<dyn Fn(SliderView) -> Element + Send + Sync>;

/// The value a slider is bound to: whole numbers, or fractions.
#[derive(Clone)]
enum Bound {
  Int(Signal<i32>),
  Float(Signal<f32>),
}

/// A slider: a value picked along a track, between a least and a greatest.
/// Volume, the size of a brush, a price no higher than.
///
/// ```
/// use lurq::{core::Signal, ui::Slider};
///
/// let volume = Signal::new(40);
/// let slider = Slider::new(volume).label("Volume");
/// // A fraction, in steps of a twentieth.
/// let opacity = Signal::new(0.5_f32);
/// let fine = Slider::new_f32(opacity).max(1.0).step(0.05).label("Opacity");
/// ```
///
/// The slider is bound to the app's signal: a drag, a press on the track
/// and the arrow keys set it, snapped to the step and kept between the
/// least and the greatest value. It is the engine's
/// [`Slider`](crate::components::Slider) dressed by the theme's recipe.
pub struct Slider {
  value: Bound,
  min: f32,
  max: f32,
  step: Option<f32>,
  label: Option<Arc<str>>,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Slider {
  /// A slider of whole numbers bound to `value`: from 0 to 100, in steps of 1.
  pub fn new(value: Signal<i32>) -> Self {
    Self::bound(Bound::Int(value), 100.0)
  }

  /// A slider of fractions bound to `value`: from 0 to 1, in steps of a hundredth.
  pub fn new_f32(value: Signal<f32>) -> Self {
    Self::bound(Bound::Float(value), 1.0)
  }

  fn bound(value: Bound, max: f32) -> Self {
    Self {
      value,
      min: 0.0,
      max,
      step: None,
      label: None,
      disabled: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// The least value: the start of the track.
  pub fn min(mut self, min: impl Into<f64>) -> Self {
    self.min = min.into() as f32;
    self
  }

  /// The greatest value: the end of the track.
  pub fn max(mut self, max: impl Into<f64>) -> Self {
    self.max = max.into() as f32;
    self
  }

  /// How far apart the values it can take are, from the least: what an arrow
  /// key moves it by, and what a drag snaps to.
  pub fn step(mut self, step: impl Into<f64>) -> Self {
    self.step = Some(step.into() as f32);
    self
  }

  /// What the slider is called, to what reads the tree. The board draws no
  /// text beside a slider: the field it is in shows its label.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Whether the slider takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Puts the slider in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this slider over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this slider: what the component
  /// it is in says of the slider in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the slider as it is under the pointer or focused from the
  /// keyboard, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the slider's node: what is dragged and focused.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the slider in `theme`. Converting a slider into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(SliderState::Disabled, self.disabled);
    let view = SliderView {
      min: self.min,
      max: self.max.max(self.min),
      step: self.step,
      label: self.label,
      disabled: self.disabled,
      recipe: theme.resolve::<Slider>(&case, Some(&self.rules)),
      value: self.value,
      forced: self.forced,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Slider>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Slider> for Element {
  fn from(slider: Slider) -> Self {
    slider.build(&current_theme())
  }
}

/// A slider as a renderer sees it: what it was given, the style of its
/// parts, and its behavior ready to be dressed.
pub struct SliderView {
  pub min: f32,
  pub max: f32,
  /// The step it was given; `None` for the engine's: 1 for whole numbers, a hundredth for fractions.
  pub step: Option<f32>,
  pub label: Option<Arc<str>>,
  pub disabled: bool,
  /// The style of each part of this slider.
  pub recipe: ResolvedRecipe,
  value: Bound,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl SliderView {
  /// The style of a part, as this slider is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The value, as a fraction or a whole number. What reads it is rendered
  /// again when it changes: a renderer that shows the value beside the track.
  pub fn value(&self) -> f32 {
    match &self.value {
      Bound::Int(value) => value.get() as f32,
      Bound::Float(value) => value.get(),
    }
  }

  /// The engine's slider bound to the value, with its range and step, in the
  /// style of the root: what is dragged, pressed, moved by the arrow keys,
  /// focused and disabled. Its track, range and thumb are not styled yet:
  /// [`SliderView::dressed`] gives them the recipe's.
  pub fn slider(&self) -> components::Slider {
    let mut slider = match &self.value {
      Bound::Int(value) => components::Slider::new(value.clone()),
      Bound::Float(value) => components::Slider::new_f32(value.clone()),
    }
    // The range first: it keeps the value in it, snapped to the step it has, and the step then snaps it to its own.
    .range_f32(self.min, self.max);
    if let Some(step) = self.step {
      slider = slider.step(step);
    }
    // A slider is a stop of Tab only in a form or with an index of its own. A control of the design system is one
    // wherever it is, in the order of the tree.
    let mut slider = slider
      .tab_index(0)
      .disabled(self.disabled)
      .part(&self.part(SliderPart::Root))
      .describe("a11y_role", "slider");
    if let Some(label) = &self.label {
      slider = slider.describe("a11y_name", label.clone());
    }
    if let Some(id) = &self.id {
      slider = slider.id(id.clone());
    }
    if let Some(key) = &self.key {
      slider = slider.key(key.clone());
    }
    slider
  }

  /// `slider` with its track, range and thumb in the style the recipe gives
  /// them, at rest and under the pointer; the thumb also as it is while the
  /// slider is focused from the keyboard. The engine paints the parts: of a
  /// part's style it takes the size, the fill, the corners, the border and
  /// the ring.
  pub fn dressed(&self, slider: components::Slider) -> components::Slider {
    let theme = self.theme();
    let (track, range, thumb) = (
      self.part(SliderPart::Track),
      self.part(SliderPart::Range),
      self.part(SliderPart::Thumb),
    );
    let mut slider = slider
      .track_style(engine_part(&track.rest(), theme))
      .fill_style(engine_part(&range.rest(), theme))
      .thumb_style(engine_part(&thumb.rest(), theme));
    if !track.styles().hovered.is_empty() {
      slider = slider.track_hovered_style(engine_part(&track.styles().hovered, theme));
    }
    if !range.styles().hovered.is_empty() {
      slider = slider.fill_hovered_style(engine_part(&range.styles().hovered, theme));
    }
    if !thumb.styles().hovered.is_empty() {
      slider = slider.thumb_hovered_style(engine_part(&thumb.styles().hovered, theme));
    }
    if !thumb.styles().focus_visible.is_empty() {
      slider = slider.thumb_focused_style(engine_part(&thumb.styles().focus_visible, theme));
    }
    slider
  }

  /// How the default renderer builds a slider: the engine's, dressed.
  pub fn render(self) -> Element {
    self.dressed(self.slider()).into()
  }
}

/// A part's style as the engine's slider paints a part: its size in pixels,
/// its fill in the theme's palette, its corners, its border and its ring.
fn engine_part(style: &PartStyle, theme: &Theme) -> SliderPartStyle {
  let px = |length: Option<Length>| match length?.resolve(theme)? {
    Dimension::Px(px) => Some(px),
    _ => None,
  };
  let mut part = SliderPartStyle::new();
  if let Some(width) = px(style.width) {
    part = part.width(width);
  }
  if let Some(height) = px(style.height) {
    part = part.height(height);
  }
  if let Some(color) = style
    .background
    .as_ref()
    .and_then(|color| color.resolve(&theme.palette()))
  {
    part = part.background(color);
  }
  if let Some(radius) = style.radius {
    part = part.rounded(radius);
  }
  if let Some(border) = &style.border {
    part = part.border(border.clone());
  }
  let ring = match (&style.outline, style.focus_ring) {
    (Some(outline), _) => Some(outline.clone()),
    (None, Some(true)) => Some(theme.focus_ring().outline()),
    _ => None,
  };
  if let Some(ring) = ring {
    part = part.outline(ring);
  }
  part
}

impl Themed for Slider {
  const NAME: &'static str = "slider";
  type Renderer = SliderRenderer;

  /// The design's slider (`pencil/lurq-design-system.pen`, Slider): 240 by
  /// 16, a track of 4 in the secondary color, filled in the primary color up
  /// to the thumb, and a round thumb of 16 in the background's color with an
  /// edge of 1.5 in the primary color inside it. The ring of the focus is
  /// the thumb's.
  fn recipe() -> Recipe {
    use PaletteColor::*;
    use SliderPart::{Range, Root, Thumb, Track};
    Recipe::new()
      .base(Root, |s| s.width(240.0).height(16.0).cursor(CursorIcon::Pointer))
      .base(Track, |s| s.height(4.0).radius(RadiusSize::Full).background(Secondary))
      .base(Range, |s| s.radius(RadiusSize::Full).background(Primary))
      .base(Thumb, |s| {
        s.square(16.0)
          .radius(RadiusSize::Full)
          .background(Background)
          .border(border::Border::inside(1.5, Primary))
      })
      .focus_visible(Thumb, |s| s.focus_ring())
      .state(SliderState::Disabled, Root, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
  }
}
