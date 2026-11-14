//! A rating: a number of stars, chosen by a press of the pointer or by the keys, or shown and not operated.

use std::sync::Arc;

use super::{
  icons, names,
  part_style::{Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{fading, glyph},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::KeyboardEvent,
    theme::{OpacityLevel, PaletteColor, RadiusSize, Theme},
  },
  components::{Row, Stack},
  core::{ElementRef, Signal},
  layout::Alignment,
  node::{CursorIcon, Element, dimension::Dimension},
};

/// The parts of a rating a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RatingPart {
  /// The stars in a row: what takes the focus and the keys, with the ring of the focus around it.
  Root,
  /// The room of one star, which a press of the pointer chooses.
  Star,
  /// A star that is lit, or the part of one that is.
  Filled,
  /// A star that is not lit, under the part of one that is.
  Empty,
}

names!(RatingPart {
  Root => "root",
  Star => "star",
  Filled => "filled",
  Empty => "empty",
});

/// The states of a rating a recipe styles, besides what the pointer and the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RatingState {
  /// It is chosen from: a press and the keys change it.
  Interactive,
  /// It is shown and not chosen from.
  ReadOnly,
  /// What it holds is wrong: a form says so.
  Invalid,
  /// It takes no input.
  Disabled,
}

names!(RatingState {
  Interactive => "interactive",
  ReadOnly => "read_only",
  Invalid => "invalid",
  Disabled => "disabled",
});

/// The sizes of a rating's stars.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RatingSize {
  /// Stars of 16, 2 apart.
  Sm,
  /// Stars of 20, 4 apart: the board's.
  #[default]
  Md,
  /// Stars of 24, 4 apart.
  Lg,
}

names!(RatingSize {
  Sm => "sm",
  Md => "md",
  Lg => "lg",
});

/// What a press and a key choose: whole stars, or halves of them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RatingPrecision {
  /// A whole star: 1, 2, 3.
  #[default]
  Whole,
  /// Half a star: 0.5, 1, 1.5.
  Half,
}

impl RatingPrecision {
  /// How far a key moves the value.
  pub fn step(self) -> f64 {
    match self {
      Self::Whole => 1.0,
      Self::Half => 0.5,
    }
  }
}

/// The value a rating holds: `f64`, the number of stars, of which `0.0` is none, or `Option<f64>`, of which `None`
/// is none: a rating that has not been given.
pub trait RatingValue: Copy + PartialEq + Send + Sync + 'static {
  /// The number of stars, `None` where it is none.
  fn stars(self) -> Option<f64>;
  /// The value of `stars` stars, or of none.
  fn from_stars(stars: Option<f64>) -> Self;
}

impl RatingValue for f64 {
  fn stars(self) -> Option<f64> {
    (self > 0.0).then_some(self)
  }

  fn from_stars(stars: Option<f64>) -> Self {
    stars.unwrap_or(0.0)
  }
}

impl RatingValue for Option<f64> {
  fn stars(self) -> Option<f64> {
    self.filter(|stars| *stars > 0.0)
  }

  fn from_stars(stars: Option<f64>) -> Self {
    stars
  }
}

/// What a rating says it holds to tooling: given the number of stars (`None` for none) and how many there are.
pub type RatingValueText = Arc<dyn Fn(Option<f64>, u32) -> String + Send + Sync>;

/// What builds a rating's nodes: a theme's in place of the default one.
pub type RatingRenderer = Arc<dyn Fn(RatingView) -> Element + Send + Sync>;

/// What a rating is told a new value with, as a number of stars or none.
type Change = Arc<dyn Fn(Option<f64>) + Send + Sync>;

/// A rating: a number of stars out of five, chosen by a press of the pointer or by the keys. The quality of a
/// call, how much a person liked a release; shown and not chosen, what others gave a package.
///
/// ```
/// use lurq::{core::Signal, ui::{Rating, RatingPrecision}};
///
/// # fn ratings(stars: Signal<f64>, given: Signal<Option<f64>>) {
/// // Bound to the app's signal, which a press or a key sets.
/// let quality = Rating::new(stars).label("Call quality");
/// // None until it is given, in halves of a star; a press of the star that is the value takes it back.
/// let review = Rating::optional(given)
///   .precision(RatingPrecision::Half)
///   .clearable(true)
///   .on_change(|stars| println!("{stars:?}"));
/// // Shown and not chosen: the part of the last star is lit.
/// let average = Rating::at(3.7).label("Average rating");
/// # }
/// ```
///
/// To tooling it is a slider called by its label, of the range from none (or the first step) to the number of
/// stars, that says its value in words ("3 of 5 stars"). It is one stop of Tab: Left and Down take a step off,
/// Right and Up add one, Home and End go to the least and the most, a digit sets that many stars, and where it is
/// clearable 0, Backspace and Delete set none. Under the pointer the stars up to the one under it are lit, in
/// halves where it is chosen in halves, as a press would set them: a rating does that where it is mounted
/// ([`Rating::mount`]), which keeps where the pointer is; converted into an element it takes the presses and the
/// keys and lights nothing under the pointer.
pub struct Rating<V = f64> {
  value: V,
  set: Option<Arc<dyn Fn(V) + Send + Sync>>,
  on_change: Vec<Arc<dyn Fn(V) + Send + Sync>>,
  max: u32,
  precision: RatingPrecision,
  clearable: bool,
  read_only: bool,
  disabled: bool,
  invalid: bool,
  size: Name,
  icon: Name,
  empty_icon: Name,
  label: Option<Arc<str>>,
  value_text: RatingValueText,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<V: RatingValue> Rating<V> {
  fn of(value: V, set: Option<Arc<dyn Fn(V) + Send + Sync>>) -> Self {
    Self {
      value,
      set,
      on_change: Vec::new(),
      max: 5,
      precision: RatingPrecision::Whole,
      clearable: false,
      read_only: false,
      disabled: false,
      invalid: false,
      size: Name::from(RatingSize::Md),
      icon: Name::new(icons::STAR_FILL),
      empty_icon: Name::new(icons::STAR_FILL),
      label: None,
      value_text: Arc::new(|stars, max| match stars {
        Some(stars) => format!("{} of {max} stars", number(stars)),
        None => "No rating".to_owned(),
      }),
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }
}

impl Rating<f64> {
  /// A rating of the stars the signal holds, `0.0` for none; a press or a key sets the signal.
  pub fn new(stars: Signal<f64>) -> Self {
    Self::of(stars.get(), Some(Arc::new(move |value| stars.set(value))))
  }

  /// A rating whose stars the app holds: `on_change` is told the stars a press or a key asks for, and what is
  /// drawn is what the app says next.
  pub fn controlled(stars: f64, on_change: impl Fn(f64) + Send + Sync + 'static) -> Self {
    Self::of(stars, None).on_change(on_change)
  }

  /// A rating shown and not chosen from: `stars` lit, the part of a star too (3.7 lights seven tenths of the
  /// fourth). It tells nothing and is no stop of Tab.
  pub fn at(stars: f64) -> Self {
    Self::of(stars, None).read_only(true)
  }
}

impl Rating<Option<f64>> {
  /// A rating of the stars the signal holds, none while it holds `None`: one that has not been given.
  pub fn optional(stars: Signal<Option<f64>>) -> Self {
    Self::of(stars.get(), Some(Arc::new(move |value| stars.set(value))))
  }

  /// A rating whose stars the app holds, none while they are `None`: `on_change` is told what a press or a key
  /// asks for, and what is drawn is what the app says next.
  pub fn controlled_optional(stars: Option<f64>, on_change: impl Fn(Option<f64>) + Send + Sync + 'static) -> Self {
    Self::of(stars, None).on_change(on_change)
  }
}

impl<V: RatingValue> Rating<V> {
  /// How many stars there are: 5 unless told.
  pub fn max(mut self, max: u32) -> Self {
    self.max = max.max(1);
    self
  }

  /// Whether a press and a key choose whole stars or halves: whole unless told. A value the app gives is drawn as
  /// it is, whatever this says.
  pub fn precision(mut self, precision: RatingPrecision) -> Self {
    self.precision = precision;
    self
  }

  /// Whether a press of the star that is the value takes it back, to none; and 0, Backspace and Delete with it.
  pub fn clearable(mut self, clearable: bool) -> Self {
    self.clearable = clearable;
    self
  }

  /// Whether it is shown and not chosen from: no press, no key, no stop of Tab.
  pub fn read_only(mut self, read_only: bool) -> Self {
    self.read_only = read_only;
    self
  }

  /// Whether it takes no input: dimmed, and no stop of Tab.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Whether what it holds is wrong: its stars that are not lit are in the destructive color, at half its opacity.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.invalid = invalid;
    self
  }

  /// A [`RatingSize`], or a size the theme's recipe names: `Md` unless told, the board's.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// The icon of a lit star, by its name in the theme's set: the solid star unless told. A heart for a rating of
  /// how much something is liked.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = name.into();
    self
  }

  /// The icon of a star that is not lit: the solid star unless told, in its own color.
  pub fn empty_icon(mut self, name: impl Into<Name>) -> Self {
    self.empty_icon = name.into();
    self
  }

  /// What it is called to tooling: "Rating" unless told. It draws nothing: a field gives it its own label.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// What it says it holds to tooling, given the stars (`None` for none) and how many there are: "3 of 5 stars"
  /// and "No rating" unless told.
  pub fn value_text(mut self, text: impl Fn(Option<f64>, u32) -> String + Send + Sync + 'static) -> Self {
    self.value_text = Arc::new(text);
    self
  }

  /// Called with the value a press or a key sets, after the signal it is bound to is set. Not for a press that
  /// changes nothing, nor when the app sets the signal. A second handler is called after the first.
  pub fn on_change(mut self, handler: impl Fn(V) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Puts the rating in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this rating over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this rating: what the component it is in says of it in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the rating as it is under the pointer or focused from the keyboard, with nothing pointing at it: for a
  /// gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the row of stars: what takes the focus.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// What tells the rating apart from those beside it.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the rating in `theme`, with nothing lit under the pointer. Converting a rating into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    self.core().build(theme, None)
  }

  /// Mounts the rating, which keeps where the pointer is: the stars up to the one under it are lit as a press
  /// would set them.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    ctx.mount::<RatingHost>(RatingProps { core: self.core() })
  }

  /// What is built, with the app's type left behind: the handlers are told the value of it.
  fn core(self) -> Core {
    let (set, handlers) = (self.set, self.on_change);
    let change: Change = Arc::new(move |stars| {
      let value = V::from_stars(stars);
      if let Some(set) = &set {
        set(value);
      }
      for handler in &handlers {
        handler(value);
      }
    });
    Core {
      stars: self.value.stars(),
      change,
      max: self.max,
      step: self.precision.step(),
      clearable: self.clearable,
      read_only: self.read_only,
      disabled: self.disabled,
      invalid: self.invalid,
      size: self.size,
      icon: self.icon,
      empty_icon: self.empty_icon,
      label: self.label.unwrap_or_else(|| Arc::from("Rating")),
      value_text: self.value_text,
      states: self.states,
      rules: self.rules,
      forced: self.forced,
      id: self.id,
      key: self.key,
    }
  }
}

impl<V: RatingValue> From<Rating<V>> for Element {
  fn from(rating: Rating<V>) -> Self {
    rating.build(&current_theme())
  }
}

/// A number of stars as tooling reads it: `3`, `3.5`, `3.7`.
fn number(stars: f64) -> String {
  let rounded = (stars * 100.0).round() / 100.0;
  if rounded.fract() == 0.0 {
    format!("{}", rounded as i64)
  } else {
    format!("{rounded}")
  }
}

/// A rating with the app's type left behind.
#[derive(Clone)]
struct Core {
  stars: Option<f64>,
  change: Change,
  max: u32,
  step: f64,
  clearable: bool,
  read_only: bool,
  disabled: bool,
  invalid: bool,
  size: Name,
  icon: Name,
  empty_icon: Name,
  label: Arc<str>,
  value_text: RatingValueText,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Core {
  fn build(self, theme: &Theme, preview: Option<Signal<Option<f64>>>) -> Element {
    let interactive = !self.read_only && !self.disabled;
    let case = Case {
      variant: None,
      size: Some(self.size.clone()),
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(RatingState::Interactive, interactive)
    .state(RatingState::ReadOnly, self.read_only)
    .state(RatingState::Invalid, self.invalid)
    .state(RatingState::Disabled, self.disabled);
    let recipe = theme.resolve::<Rating>(&case, Some(&self.rules));
    let shown = preview
      .as_ref()
      .filter(|_| interactive)
      .and_then(|preview| preview.get())
      .or(self.stars);
    let view = RatingView {
      stars: self.stars,
      shown,
      max: self.max,
      step: self.step,
      clearable: self.clearable,
      read_only: self.read_only,
      disabled: self.disabled,
      invalid: self.invalid,
      label: self.label,
      recipe,
      forced: self.forced,
      icon: self.icon,
      empty_icon: self.empty_icon,
      value_text: self.value_text,
      change: interactive.then_some(self.change),
      preview: preview.filter(|_| interactive),
      root: ElementRef::new(),
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Rating>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

/// A rating as a renderer sees it: what it holds and shows, the style of its parts, and its behavior ready to be
/// dressed.
pub struct RatingView {
  /// The stars it holds, `None` for none.
  pub stars: Option<f64>,
  /// The stars it shows: those under the pointer where it is mounted and the pointer is on it, else `stars`.
  pub shown: Option<f64>,
  pub max: u32,
  /// How far a key moves it: 1, or 0.5 in halves.
  pub step: f64,
  pub clearable: bool,
  pub read_only: bool,
  pub disabled: bool,
  pub invalid: bool,
  pub label: Arc<str>,
  /// The style of each part of this rating.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  icon: Name,
  empty_icon: Name,
  value_text: RatingValueText,
  change: Option<Change>,
  preview: Option<Signal<Option<f64>>>,
  root: ElementRef,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl RatingView {
  /// The style of a part, as this rating is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The side of a star in pixels, as the recipe gives it: 20 where it gives none.
  pub fn side(&self) -> f32 {
    let theme = self.theme();
    match self
      .part(RatingPart::Star)
      .rest()
      .width
      .and_then(|width: Length| width.resolve(theme))
    {
      Some(Dimension::Px(px)) => px,
      _ => 20.0,
    }
  }

  /// How much of the star `index` (from 0) is lit: from nothing to all of it.
  pub fn lit(&self, index: u32) -> f32 {
    let shown = self.shown.unwrap_or(0.0);
    (shown - f64::from(index)).clamp(0.0, 1.0) as f32
  }

  /// What a press or a key that asks for `stars` sets: none where it is clearable and asks for what is held.
  fn asked(&self, stars: f64) -> Option<f64> {
    let stars = stars.clamp(0.0, f64::from(self.max));
    if stars <= 0.0 || (self.clearable && Some(stars) == self.stars) {
      None
    } else {
      Some(stars)
    }
  }

  /// What sets `stars`, where it is not what is held already.
  fn setter(&self) -> Option<impl Fn(Option<f64>) + Send + Sync + Clone + 'static> {
    let (change, held) = (self.change.clone()?, self.stars);
    Some(move |stars: Option<f64>| {
      if stars != held {
        change(stars);
      }
    })
  }

  /// What a key asks for, from what is held: `None` for a key it does not take, `Some(None)` for none.
  fn keyed(&self, key: &str) -> Option<Option<f64>> {
    let (step, max) = (self.step, f64::from(self.max));
    let held = self.stars.unwrap_or(0.0);
    let least = if self.clearable { 0.0 } else { step };
    let asked = match key {
      "ArrowRight" | "ArrowUp" => ((held / step).floor() * step + step).min(max),
      "ArrowLeft" | "ArrowDown" => ((held / step).ceil() * step - step).max(least),
      "Home" => least,
      "End" => max,
      "Backspace" | "Delete" if self.clearable => 0.0,
      digit if digit.len() == 1 && digit.as_bytes()[0].is_ascii_digit() => {
        let stars = f64::from(digit.as_bytes()[0] - b'0');
        if stars > max || (stars == 0.0 && !self.clearable) {
          return None;
        }
        stars
      }
      _ => return None,
    };
    Some((asked > 0.0).then_some(asked))
  }

  /// The row of stars with the style of its root, with no star in it yet: what takes the focus and the keys, a
  /// slider to tooling that says its range and its value in words.
  pub fn root(&self) -> Row {
    let text = (self.value_text)(self.stars, self.max);
    let mut root = Row::new()
      .align_items(Alignment::Center)
      .overflow_visible()
      .part(&self.part(RatingPart::Root))
      .ref_element(self.root.clone())
      .describe("a11y_role", "slider")
      .describe("a11y_name", self.label.clone())
      .describe("a11y_valuemin", number(if self.clearable { 0.0 } else { self.step }))
      .describe("a11y_valuemax", self.max.to_string())
      .describe("a11y_valuenow", number(self.stars.unwrap_or(0.0)))
      .describe("a11y_valuetext", text);
    if self.read_only {
      root = root.describe("a11y_readonly", "true");
    }
    if self.disabled {
      root = root.describe("a11y_disabled", "true");
    }
    if self.invalid {
      root = root.describe("a11y_invalid", "true");
    }
    if let Some(set) = self.setter() {
      root = root.tab_index(0);
      let asks: Vec<(&'static str, Option<Option<f64>>)> = [
        "ArrowRight",
        "ArrowUp",
        "ArrowLeft",
        "ArrowDown",
        "Home",
        "End",
        "Backspace",
        "Delete",
        "0",
        "1",
        "2",
        "3",
        "4",
        "5",
        "6",
        "7",
        "8",
        "9",
      ]
      .into_iter()
      .map(|key| (key, self.keyed(key)))
      .collect();
      root = root.on_key_down(move |event: KeyboardEvent| {
        if !event.focus_within || event.ctrl || event.alt || event.meta || event.composing {
          return;
        }
        let Some((_, Some(asked))) = asks.iter().find(|(key, _)| *key == event.key) else {
          return;
        };
        event.prevent_default();
        set(*asked);
      });
    }
    if let Some(preview) = self.preview.clone() {
      root = root.on_mouse_leave(move || preview.set(None));
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// The star `index` (from 0) in the style of its part: the empty star, with as much of the lit one over it as is
  /// lit, and where it is chosen from, the room a press chooses it by (the halves of it, in halves).
  pub fn star(&self, index: u32) -> Element {
    let side = self.side();
    let lit = self.lit(index);
    let mut star = Stack::new().part(&self.part(RatingPart::Star));
    if lit < 1.0 {
      star = star.child(glyph(&self.part(RatingPart::Empty), self.empty_icon.clone(), false));
    }
    if lit > 0.0 {
      let filled = glyph(&self.part(RatingPart::Filled), self.icon.clone(), false);
      star = star.child(if lit < 1.0 {
        // The lit part of a star is the lit star in a room as wide as that part, which cuts the rest off.
        Row::new().width(side * lit).height(side).child(filled).into()
      } else {
        filled
      });
    }
    if let Some(set) = self.setter() {
      let halves: &[f64] = if self.step < 1.0 { &[0.5, 1.0] } else { &[1.0] };
      let mut rooms = Row::new().size(side, side);
      for &part in halves {
        let stars = f64::from(index) + part;
        let asked = self.asked(stars);
        let set = set.clone();
        let mut room = Row::new()
          .size(side / halves.len() as f32, side)
          .focuses(&self.root)
          .on_click(move |_| set(asked));
        if let Some(preview) = self.preview.clone() {
          room = room.on_mouse_enter(move || preview.set(Some(stars)));
        }
        rooms = rooms.child(room);
      }
      star = star.child(rooms);
    }
    star.into()
  }

  /// How the default renderer builds a rating: its stars in a row.
  pub fn render(self) -> Element {
    let stars: Vec<Element> = (0..self.max).map(|index| self.star(index)).collect();
    self.root().with_children(stars).into()
  }
}

/// What a mounted rating is given: the rating, built anew by every render of what holds it.
struct RatingProps {
  core: Core,
}

impl PartialEq for RatingProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for RatingProps {}

/// What a mounted rating keeps: the stars under the pointer.
struct RatingHost {
  preview: Signal<Option<f64>>,
}

impl Component for RatingHost {
  type Props = RatingProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      preview: ctx.signal(None),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let core = ctx.props::<RatingProps>().core.clone();
    core.build(&current_theme(), Some(self.preview.clone()))
  }
}

impl Themed for Rating {
  const NAME: &'static str = "rating";
  type Renderer = RatingRenderer;

  /// The design's rating (`pencil/lurq-design-system.pen`, Rating): five solid stars of 20, 4 apart, those lit in
  /// the warning color and the rest in the color of an input's edge. Small they are 16 and 2 apart, large 24. The
  /// board draws no other state: the ring of the focus is around the row, a rating that is chosen from has the
  /// pointer's hand, an invalid one has its stars that are not lit in the destructive color at half its opacity, and a
  /// disabled one is dimmed.
  fn recipe() -> Recipe {
    use PaletteColor::{Destructive, Input, Warning};
    use RatingPart::{Empty, Filled, Root, Star};
    use RatingState::{Disabled, Interactive, Invalid};
    let mut recipe = Recipe::new()
      .base(Root, |s| fading(s.gap(4.0).radius(RadiusSize::Sm)))
      .base(Filled, |s| s.foreground(Warning))
      .base(Empty, |s| s.foreground(Input))
      .focus_visible(Root, |s| s.focus_ring())
      .state(Interactive, Star, |s| s.cursor(CursorIcon::Pointer))
      .state(Invalid, Empty, |s| {
        s.foreground(Destructive).opacity(OpacityLevel::Disabled)
      })
      .state(Disabled, Root, |s| s.opacity(OpacityLevel::Disabled));
    for (size, side, gap) in [
      (RatingSize::Sm, 16.0, 2.0),
      (RatingSize::Md, 20.0, 4.0),
      (RatingSize::Lg, 24.0, 4.0),
    ] {
      recipe = recipe
        .size(size, Root, move |s| s.gap(gap))
        .size(size, Star, move |s| s.square(side))
        .size(size, Filled, move |s| s.square(side))
        .size(size, Empty, move |s| s.square(side));
    }
    recipe
  }
}

impl<V: RatingValue> super::FieldControl for Rating<V> {
  /// Called by the field's label unless it has a name of its own, invalid where the field is, and disabled where
  /// the field is.
  fn in_field(mut self, field: &super::InField) -> Element {
    if self.label.is_none() {
      self.label = Some(field.label.clone());
    }
    self.invalid |= field.judgment == Some(super::Judgment::Invalid);
    self.disabled |= field.disabled;
    self.into()
  }
}
