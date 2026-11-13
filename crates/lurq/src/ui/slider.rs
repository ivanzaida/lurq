//! A slider: a value picked along a track, between a least and a greatest; and a range slider, two values
//! picked on one track.

use std::sync::Arc;

use super::{
  names,
  part_style::{Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  theme::{Themed, current_theme},
};
/// Which way a slider's track runs: across, or up with the least value at the bottom.
pub use crate::node::SliderOrientation;
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
  /// The part of the track from its start to the thumb: how much of the range the value is. Of a range
  /// slider, the part between its thumbs.
  Range,
  /// What is dragged: at the value, along the track. A range slider has two.
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
  /// Its track runs up, the least value at the bottom.
  Vertical,
  /// It is a [`RangeSlider`]: two thumbs, and the range between them.
  Range,
}

names!(SliderState {
  Disabled => "disabled",
  Vertical => "vertical",
  Range => "range",
});

/// What builds a slider's nodes: a theme's in place of the default one.
pub type SliderRenderer = Arc<dyn Fn(SliderView) -> Element + Send + Sync>;

/// The value a slider is bound to: whole numbers, or fractions; of a range slider, a low one and a high one.
#[derive(Clone)]
enum Bound {
  Int(Signal<i32>),
  Float(Signal<f32>),
  IntRange(Signal<(i32, i32)>),
  FloatRange(Signal<(f32, f32)>),
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
/// and the keys set it, snapped to the step and kept between the least and
/// the greatest value. The arrow keys move it a step (right and up toward the
/// greatest), Page Up and Page Down a tenth of the range in whole steps and at
/// least one, and Home and End to the least and the greatest. It is the engine's
/// [`Slider`](crate::components::Slider) dressed by the theme's recipe.
pub struct Slider {
  value: Bound,
  min: f32,
  max: f32,
  step: Option<f32>,
  orientation: SliderOrientation,
  length: Option<f32>,
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
      orientation: SliderOrientation::Horizontal,
      length: None,
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

  /// Which way the track runs. A vertical slider is 16 wide and 140 high,
  /// its least value at the bottom; the up and down arrows move it as the
  /// right and left ones do.
  pub fn orientation(mut self, orientation: SliderOrientation) -> Self {
    self.orientation = orientation;
    self
  }

  /// How long the slider is along its track, in pixels: its width across, its
  /// height up. 240 across and 140 up when not given.
  pub fn length(mut self, length: f32) -> Self {
    self.length = Some(length);
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
    .state(SliderState::Disabled, self.disabled)
    .state(SliderState::Vertical, self.orientation == SliderOrientation::Vertical)
    .state(
      SliderState::Range,
      matches!(self.value, Bound::IntRange(_) | Bound::FloatRange(_)),
    );
    let view = SliderView {
      min: self.min,
      max: self.max.max(self.min),
      step: self.step,
      orientation: self.orientation,
      length: self.length,
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
  pub orientation: SliderOrientation,
  /// The length along the track it was given; `None` for the recipe's.
  pub length: Option<f32>,
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

  /// The value, as a fraction or a whole number; of a range slider, its low
  /// value. What reads it is rendered again when it changes: a renderer that
  /// shows the value beside the track.
  pub fn value(&self) -> f32 {
    self.values().0
  }

  /// The low value and the high one of a range slider; of a slider of one
  /// value, that value twice. Read as [`SliderView::value`] is.
  pub fn values(&self) -> (f32, f32) {
    match &self.value {
      Bound::Int(value) => {
        let value = value.get() as f32;
        (value, value)
      }
      Bound::Float(value) => {
        let value = value.get();
        (value, value)
      }
      Bound::IntRange(values) => {
        let (low, high) = values.get();
        (low as f32, high as f32)
      }
      Bound::FloatRange(values) => values.get(),
    }
  }

  /// Whether this is a [`RangeSlider`]: two thumbs bound to a low value and
  /// a high one.
  pub fn is_range(&self) -> bool {
    matches!(self.value, Bound::IntRange(_) | Bound::FloatRange(_))
  }

  /// The engine's slider bound to the value (or a range slider's two), with
  /// its range, step and orientation, in the style of the root and of the
  /// length it was given: what is dragged, pressed, moved by the arrow keys,
  /// focused and disabled. Its track, range and thumbs are not styled yet:
  /// [`SliderView::dressed`] gives them the recipe's.
  pub fn slider(&self) -> components::Slider {
    let mut slider = match &self.value {
      Bound::Int(value) => components::Slider::new(value.clone()),
      Bound::Float(value) => components::Slider::new_f32(value.clone()),
      Bound::IntRange(values) => components::Slider::new_range(values.clone()),
      Bound::FloatRange(values) => components::Slider::new_range_f32(values.clone()),
    }
    .orientation(self.orientation)
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
    if let Some(length) = self.length {
      slider = match self.orientation {
        SliderOrientation::Horizontal => slider.width(length),
        SliderOrientation::Vertical => slider.height(length),
      };
    }
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
  /// the thumb's. The vertical one (Slider/Vertical) is 16 by 140, its track
  /// 4 wide and as high as it; the range slider (Range Slider) is the slider
  /// with two thumbs, the range between them.
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
      .state(SliderState::Vertical, Root, |s| s.width(16.0).height(140.0))
      // The track's height is undone: it is as high as the slider.
      .state(SliderState::Vertical, Track, |s| s.width(4.0).height(Length::Auto))
      .state(SliderState::Disabled, Root, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
  }
}

/// A range slider: two values picked on one track, a low one and a high one.
/// A price between, the hours of a day a shop is open.
///
/// ```
/// use lurq::{core::Signal, ui::RangeSlider};
///
/// let price = Signal::new((20, 80));
/// let slider = RangeSlider::new(price).label("Price");
/// ```
///
/// Bound to the app's signal of the two values, the low one first. A press on
/// the track moves the nearer thumb there, a drag moves the thumb it began on,
/// and neither thumb passes the other. Each thumb is a stop of Tab, the low
/// one first, and the keys move the one focused as they move a [`Slider`]:
/// Home and End take it no further than the other thumb. It is drawn by the
/// [`Slider`]'s recipe and renderer, in the state
/// [`SliderState::Range`]: a theme that restyles the slider restyles it too.
pub struct RangeSlider(Slider);

/// A method of [`RangeSlider`] that is [`Slider`]'s.
macro_rules! as_slider {
  ($($(#[$doc:meta])* fn $name:ident($($arg:ident: $ty:ty),*);)*) => {
    $(
      $(#[$doc])*
      pub fn $name(self, $($arg: $ty),*) -> Self {
        Self(self.0.$name($($arg),*))
      }
    )*
  };
}

impl RangeSlider {
  /// A range slider of whole numbers bound to `values`, the low one first:
  /// from 0 to 100, in steps of 1.
  pub fn new(values: Signal<(i32, i32)>) -> Self {
    Self(Slider::bound(Bound::IntRange(values), 100.0))
  }

  /// A range slider of fractions bound to `values`: from 0 to 1, in steps of
  /// a hundredth.
  pub fn new_f32(values: Signal<(f32, f32)>) -> Self {
    Self(Slider::bound(Bound::FloatRange(values), 1.0))
  }

  as_slider! {
    /// The least value: the start of the track.
    fn min(min: impl Into<f64>);
    /// The greatest value: the end of the track.
    fn max(max: impl Into<f64>);
    /// How far apart the values it can take are, from the least: what an
    /// arrow key moves a thumb by, and what a drag snaps to.
    fn step(step: impl Into<f64>);
    /// Which way the track runs: see [`Slider::orientation`].
    fn orientation(orientation: SliderOrientation);
    /// How long the slider is along its track: see [`Slider::length`].
    fn length(length: f32);
    /// What the slider is called, to what reads the tree.
    fn label(label: &str);
    /// Whether the slider takes no input.
    fn disabled(disabled: bool);
    /// Puts the slider in a state the theme's recipe names, or takes it out.
    fn state(state: impl Into<Name>, on: bool);
    /// Styles one part of this slider over what the theme gives it.
    fn style(part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle);
    /// Lays rules over the theme's recipe for this slider.
    fn rules(rules: &Recipe);
    /// Draws the slider as it is under the pointer or focused from the
    /// keyboard, with nothing pointing at it: for a gallery or a test.
    fn force(interaction: Option<Interaction>);
    /// The id of the slider's node: what is dragged and focused.
    fn id(id: impl Into<Arc<str>>);
    fn key(key: impl Into<Arc<str>>);
  }

  /// Builds the range slider in `theme`. Converting it into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    self.0.build(theme)
  }
}

impl From<RangeSlider> for Element {
  fn from(slider: RangeSlider) -> Self {
    slider.build(&current_theme())
  }
}

impl super::FieldControl for Slider {
  /// Called by the field's label unless it has a name of its own, and disabled where the field is. A slider has no
  /// judgment.
  fn in_field(mut self, field: &super::InField) -> Element {
    if self.label.is_none() {
      self.label = Some(field.label.clone());
    }
    self.disabled |= field.disabled;
    self.into()
  }
}

impl super::FieldControl for RangeSlider {
  /// As a [`Slider`] takes it.
  fn in_field(self, field: &super::InField) -> Element {
    self.0.in_field(field)
  }
}
