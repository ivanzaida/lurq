//! An angle slider: a dial that sets an angle, in degrees, by a drag round it, a press on it or the keys.

use std::{cell::RefCell, collections::HashMap, sync::Arc};

use super::{
  names,
  part_style::{FontFamily, Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::fading,
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    events::{DragEvent, KeyboardEvent, MouseButtonMask, TouchEvent},
    theme::{OpacityLevel, PaletteColor, RadiusSize, Theme},
  },
  components::{Row, Stack},
  core::{ElementRef, Signal},
  layout::stack_alignment::StackAlignment,
  node::{CursorIcon, Element, border::Border, dimension::Dimension},
};

/// The parts of an angle slider a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AngleSliderPart {
  /// The dial: its circle and its ring, with the ring of the focus around it. What is dragged, pressed and focused.
  Root,
  /// The angle in the middle of the dial: "45°".
  Value,
  /// The dot on the dial's ring at the angle.
  Handle,
}

names!(AngleSliderPart {
  Root => "root",
  Value => "value",
  Handle => "handle",
});

/// The states of an angle slider a recipe styles, besides what the pointer and the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AngleSliderState {
  /// It takes no input.
  Disabled,
}

names!(AngleSliderState {
  Disabled => "disabled",
});

/// What builds an angle slider's nodes: a theme's in place of the default one.
pub type AngleSliderRenderer = Arc<dyn Fn(AngleSliderView) -> Element + Send + Sync>;

type Told = Arc<dyn Fn(f64) + Send + Sync>;

/// What an angle slider says its value is, on its dial and to tooling.
pub type AngleText = Arc<dyn Fn(f64) -> String + Send + Sync>;

/// An angle slider: a dial that sets an angle in degrees. The rotation of a layer, the direction of a gradient or
/// of a light.
///
/// ```
/// use lurq::{core::Signal, ui::AngleSlider};
///
/// # fn dials(rotation: Signal<f64>) {
/// // Bound to the app's signal: 0 to 360 in whole degrees, going round past 360 to 0.
/// let dial = AngleSlider::new(rotation).label("Rotation");
/// // The app holds the angle; a half turn either way of 0, in steps of 15, that stops at its ends.
/// let held = AngleSlider::controlled(45.0, |angle| println!("{angle}°"))
///   .min(-180)
///   .max(180)
///   .step(15)
///   .wrap(false);
/// # }
/// ```
///
/// The angle is drawn as it is on a compass: 0 at the top and growing clockwise, the dot on the dial's ring at it and
/// the angle in the middle. A press on the dial sets the angle it is at, and a drag goes on round from there,
/// snapped to the [step](Self::step); a finger or a pen on it does as the mouse does. The arrow keys move it a step
/// (Right and Up clockwise), Page Up and Page Down by [`page_step`](Self::page_step), Home and End to the least and
/// the greatest. Between 0 and 360, the range it has unless told, it goes round: a step past 359 is 0, and a drag goes
/// on round; [`wrap`](Self::wrap) `false`, or a range of less than a turn, stops at the ends.
/// [`on_change`](Self::on_change) hears each change as it is made, [`on_change_end`](Self::on_change_end) the angle
/// when the drag is let go or the key's step is made, as a [`Slider`](super::Slider) tells them. To tooling it is a
/// slider called by its label.
pub struct AngleSlider {
  value: f64,
  bound: Option<Signal<f64>>,
  on_change: Vec<Told>,
  on_change_end: Vec<Told>,
  min: f64,
  max: f64,
  step: f64,
  page_step: Option<f64>,
  wrap: bool,
  label: Option<Arc<str>>,
  value_text: Option<AngleText>,
  /// The words it was told to speak in, in place of the app's.
  locale: Option<crate::core::InputWords>,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl AngleSlider {
  fn of(value: f64, bound: Option<Signal<f64>>) -> Self {
    Self {
      value,
      bound,
      on_change: Vec::new(),
      on_change_end: Vec::new(),
      min: 0.0,
      max: 360.0,
      step: 1.0,
      page_step: None,
      wrap: true,
      label: None,
      value_text: None,
      locale: None,
      disabled: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// A dial of the angle the signal holds, in degrees: a drag, a press and the keys set it.
  pub fn new(angle: Signal<f64>) -> Self {
    Self::of(angle.get(), Some(angle))
  }

  /// A dial that shows `angle`, held by the app, and tells `on_change` the angle a person makes it: what is drawn is
  /// what the app gives next.
  pub fn controlled(angle: f64, on_change: impl Fn(f64) + Send + Sync + 'static) -> Self {
    Self::of(angle, None).on_change(on_change)
  }

  /// The least angle: 0 unless told.
  pub fn min(mut self, min: impl Into<f64>) -> Self {
    self.min = min.into();
    self
  }

  /// The greatest angle: 360 unless told. A range of more than a turn is turned round more than once.
  pub fn max(mut self, max: impl Into<f64>) -> Self {
    self.max = max.into();
    self
  }

  /// How far apart the angles it takes are, from the least: what an arrow key moves it by and what a drag snaps to.
  /// 1 unless told.
  pub fn step(mut self, step: impl Into<f64>) -> Self {
    let step = step.into();
    if step.is_finite() && step > 0.0 {
      self.step = step;
    }
    self
  }

  /// What Page Up and Page Down move it by: 15 unless told, in whole steps and at least one.
  pub fn page_step(mut self, step: impl Into<f64>) -> Self {
    self.page_step = Some(step.into());
    self
  }

  /// Whether it goes round, past its greatest angle to its least and back: where its range is a whole turn, as the
  /// 0 to 360 it has unless told is. On unless told; off, or with a range of less or more than a turn, it stops at
  /// its ends, and a drag past an end stays there until it comes back.
  pub fn wrap(mut self, wrap: bool) -> Self {
    self.wrap = wrap;
    self
  }

  /// Called each time a person changes the angle, with the new angle: as a drag moves it, a press, a key. Not called
  /// when the app sets the signal, nor for a press that leaves it as it is. A second handler is called after the
  /// first.
  pub fn on_change(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Called when an interaction is over, with the angle as it then is, where it is another than it was as the
  /// interaction began: the drag let go, the key's step made. A second handler is called after the first.
  pub fn on_change_end(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.on_change_end.push(Arc::new(handler));
    self
  }

  /// What it is called to tooling: the locale's "Angle" unless told. A field gives it its label.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// What it says the angle is, in the middle of the dial and to tooling: "45°" unless told, with the decimals of the
  /// step ("22.5°").
  pub fn value_text(mut self, text: impl Fn(f64) -> String + Send + Sync + 'static) -> Self {
    self.value_text = Some(Arc::new(text));
    self
  }

  /// The words this dial speaks in, in place of the app's ([`App::set_locale`](crate::app::App::set_locale)): what
  /// it is called. A label told by itself stays what it was told.
  pub fn locale(mut self, locale: &crate::core::Locale) -> Self {
    self.locale = Some(locale.input.clone());
    self
  }

  /// Whether it takes no input: dimmed, and no stop of Tab.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Puts the dial in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this dial over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this dial: what the component it is in says of it in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the dial as it is under the pointer or focused from the keyboard, with nothing pointing at it: for a
  /// gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the dial: what is dragged and focused.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the dial in `theme`. Converting a dial into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let range = Range::new(self.min, self.max, self.step, self.page_step, self.wrap);
    let angle = range.held(self.value);
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(AngleSliderState::Disabled, self.disabled);
    let view = AngleSliderView {
      angle,
      min: range.min,
      max: range.max,
      step: range.step,
      wraps: range.wraps,
      // Its own locale's word, else the app's, read where it is built.
      label: self.label.unwrap_or_else(|| match self.locale {
        Some(words) => words.angle,
        None => super::current_locale().input.angle.clone(),
      }),
      disabled: self.disabled,
      recipe: theme.resolve::<AngleSlider>(&case, Some(&self.rules)),
      value_text: self.value_text.unwrap_or_else(|| {
        let decimals = decimals_of(range.step);
        Arc::new(move |angle| format!("{}°", number(angle, decimals)))
      }),
      model: Model {
        range,
        bound: self.bound,
        given: angle,
        on_change: self.on_change,
        on_change_end: self.on_change_end,
        root: ElementRef::new(),
      },
      forced: self.forced,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<AngleSlider>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<AngleSlider> for Element {
  fn from(slider: AngleSlider) -> Self {
    slider.build(&current_theme())
  }
}

/// How many decimals a step has, up to 3: 0 for 1 and 15, 1 for 0.5, 2 for 0.25.
fn decimals_of(step: f64) -> usize {
  (0..3)
    .find(|decimals| {
      let scaled = step * 10f64.powi(*decimals as i32);
      (scaled - scaled.round()).abs() < 1e-9
    })
    .unwrap_or(3)
}

/// An angle with `decimals` decimals at most, with no zeros at its end: `45`, `22.5`, `-90`.
fn number(angle: f64, decimals: usize) -> String {
  let written = format!("{angle:.decimals$}");
  let written = if written.contains('.') {
    written.trim_end_matches('0').trim_end_matches('.').to_owned()
  } else {
    written
  };
  if written == "-0" { "0".to_owned() } else { written }
}

/// The angle of the point `(dx, dy)` from the middle of the dial, in degrees: 0 at the top, growing clockwise, from 0
/// up to 360.
fn bearing(dx: f32, dy: f32) -> f64 {
  let degrees = f64::from(dx).atan2(f64::from(-dy)).to_degrees();
  degrees.rem_euclid(360.0)
}

/// The way from `from` to `to`, two bearings, the shorter way round: from -180 to 180, clockwise positive.
fn turn(from: f64, to: f64) -> f64 {
  let way = (to - from).rem_euclid(360.0);
  if way > 180.0 { way - 360.0 } else { way }
}

/// The angles a dial takes: its range, its steps, and whether it goes round.
#[derive(Clone, Copy, Debug)]
struct Range {
  min: f64,
  max: f64,
  step: f64,
  page: f64,
  /// Whether it goes round: asked to, and its range is a whole turn.
  wraps: bool,
}

impl Range {
  fn new(min: f64, max: f64, step: f64, page: Option<f64>, wrap: bool) -> Self {
    let (min, max) = if min.is_finite() && max.is_finite() {
      (min, max.max(min))
    } else {
      (0.0, 360.0)
    };
    let page = page.filter(|page| page.is_finite() && *page > 0.0).unwrap_or(15.0);
    // A page is whole steps, at least one.
    let page = ((page / step).round() * step).max(step);
    Self {
      min,
      max,
      step,
      page,
      wraps: wrap && ((max - min) - 360.0).abs() < 1e-9,
    }
  }

  /// `angle` snapped to a step from the least, and kept in the range: brought round into it where it goes round
  /// (360 of 0 to 360 is 0), held at its ends where it does not.
  fn held(&self, angle: f64) -> f64 {
    if !angle.is_finite() {
      return self.min;
    }
    let snapped = self.min + ((angle - self.min) / self.step).round() * self.step;
    let held = if self.wraps {
      self.min + (snapped - self.min).rem_euclid(360.0)
    } else {
      snapped.clamp(self.min, self.max)
    };
    // What comes round to the greatest is the least; and a step that is no divisor of the range ends below it.
    let held = if self.wraps && (held - self.max).abs() < 1e-9 {
      self.min
    } else {
      held
    };
    // No -0, and no 44.99999999 of a sum of fractions.
    let rounded = (held * 1e9).round() / 1e9;
    if rounded == 0.0 { 0.0 } else { rounded }
  }

  /// The greatest angle it takes: of one that goes round, the last step before it comes round to the least.
  fn greatest(&self) -> f64 {
    if self.wraps {
      self.held(self.max - self.step)
    } else {
      self.held(self.max - ((self.max - self.min) % self.step))
    }
  }

  /// What `key` makes of `angle`: `None` for a key it does not take.
  fn key(&self, angle: f64, key: &str) -> Option<f64> {
    let next = match key {
      "ArrowRight" | "ArrowUp" => angle + self.step,
      "ArrowLeft" | "ArrowDown" => angle - self.step,
      "PageUp" => angle + self.page,
      "PageDown" => angle - self.page,
      "Home" => return Some(self.min),
      "End" => return Some(self.greatest()),
      _ => return None,
    };
    Some(self.held(next))
  }

  /// The angle a press at `bearing` sets: of the angles in the range that are at that bearing, the nearest to
  /// `angle`; where none is, the end of the range nearer to it round the dial.
  fn pressed(&self, angle: f64, bearing: f64) -> f64 {
    let at = angle + turn(angle.rem_euclid(360.0), bearing);
    if self.wraps || (self.min..=self.max).contains(&at) {
      return self.held(at);
    }
    // Another turn of the same bearing may be in the range: a range of more than a turn, or one that starts past 0.
    for turns in [-1.0, 1.0, -2.0, 2.0] {
      let other = at + 360.0 * turns;
      if (self.min..=self.max).contains(&other) {
        return self.held(other);
      }
    }
    let from_least = turn(bearing, self.min.rem_euclid(360.0)).abs();
    let from_greatest = turn(bearing, self.max.rem_euclid(360.0)).abs();
    self.held(if from_least <= from_greatest {
      self.min
    } else {
      self.max
    })
  }

  /// Where a drag that was at `raw` (not snapped) goes when the pointer turns by `way`: round where it goes round.
  /// Where it does not, `raw` follows the pointer past an end by up to half a turn, which [`held`](Self::held) holds
  /// at the end: the angle stays there until the pointer comes back to it.
  fn dragged(&self, raw: f64, way: f64) -> f64 {
    let next = raw + way;
    if self.wraps {
      self.min + (next - self.min).rem_euclid(360.0)
    } else {
      next.clamp(self.min - 180.0, self.max + 180.0)
    }
  }
}

/// A drag of the dial: the angle it began at, the angle not snapped and the bearing of the pointer last, and the
/// angle last told.
#[derive(Clone, Copy, Debug)]
struct Drag {
  start: f64,
  raw: f64,
  bearing: f64,
  told: f64,
}

/// What the dial's handlers know of it: where its angle goes, and who is told.
#[derive(Clone)]
struct Model {
  range: Range,
  bound: Option<Signal<f64>>,
  /// The angle as it was built.
  given: f64,
  on_change: Vec<Told>,
  on_change_end: Vec<Told>,
  root: ElementRef,
}

thread_local! {
  /// The drags of the dials of the windows of this thread, by their finger (`None` for the mouse): a drag outlives
  /// the renders it makes, and the events of a finger go to the dial as each render builds it.
  static DRAGS: RefCell<HashMap<Option<u64>, Drag>> = RefCell::new(HashMap::new());
}

impl Model {
  /// The angle now: the signal's, or what the app gave.
  fn current(&self) -> f64 {
    match &self.bound {
      Some(signal) => self.range.held(signal.get_untracked()),
      None => self.given,
    }
  }

  /// Makes `angle` the angle and tells the handlers, where it is another than `was`.
  fn set(&self, angle: f64, was: f64) {
    if angle == was {
      return;
    }
    if let Some(signal) = &self.bound {
      signal.set(angle);
    }
    for handler in &self.on_change {
      handler(angle);
    }
  }

  /// Tells the end of an interaction that began at `start`, where the angle is now another.
  fn end(&self, start: f64, now: f64) {
    if now != start {
      for handler in &self.on_change_end {
        handler(now);
      }
    }
  }

  /// The bearing of the point `(x, y)` of the window from the middle of the dial; `None` at its very middle, where
  /// a point has none.
  fn bearing(&self, x: f32, y: f32) -> Option<f64> {
    let bounds = self.root.bounds();
    let (dx, dy) = (
      x - (bounds.x + bounds.width / 2.0),
      y - (bounds.y + bounds.height / 2.0),
    );
    (dx.hypot(dy) >= 2.0).then(|| bearing(dx, dy))
  }

  /// A press at `(x, y)` of the window: the angle there is set, and a drag begins. A drag of the same finger that is
  /// still held is over: a press of the mouse is a new one.
  fn press(&self, x: f32, y: f32, finger: Option<u64>) {
    let start = self.current();
    let Some(bearing) = self.bearing(x, y) else {
      let drag = Drag {
        start,
        raw: start,
        bearing: start.rem_euclid(360.0),
        told: start,
      };
      DRAGS.with(|drags| drags.borrow_mut().insert(finger, drag));
      return;
    };
    let angle = self.range.pressed(start, bearing);
    // The drag goes on round from the pointer, not from the step it snapped to.
    let raw = angle + turn(angle.rem_euclid(360.0), bearing);
    let drag = Drag {
      start,
      raw: self.range.dragged(raw, 0.0),
      bearing,
      told: angle,
    };
    DRAGS.with(|drags| drags.borrow_mut().insert(finger, drag));
    self.set(angle, start);
  }

  /// The pointer of the drag of `finger` moved to `(x, y)` of the window.
  fn moved(&self, x: f32, y: f32, finger: Option<u64>) {
    let Some(bearing) = self.bearing(x, y) else {
      return;
    };
    let moved = DRAGS.with(|drags| {
      let mut drags = drags.borrow_mut();
      let drag = drags.get_mut(&finger)?;
      drag.raw = self.range.dragged(drag.raw, turn(drag.bearing, bearing));
      drag.bearing = bearing;
      let (angle, was) = (self.range.held(drag.raw), drag.told);
      drag.told = angle;
      Some((angle, was))
    });
    if let Some((angle, was)) = moved {
      self.set(angle, was);
    }
  }

  /// The drag of `finger` is let go.
  fn released(&self, finger: Option<u64>) {
    if let Some(drag) = DRAGS.with(|drags| drags.borrow_mut().remove(&finger)) {
      self.end(drag.start, drag.told);
    }
  }
}

/// An angle slider as a renderer sees it: its angle and its range, the style of its parts, and its behavior ready to
/// be dressed.
pub struct AngleSliderView {
  /// The angle, in degrees, as it is drawn: in the range, snapped to a step.
  pub angle: f64,
  pub min: f64,
  pub max: f64,
  pub step: f64,
  /// Whether it goes round past its greatest angle to its least.
  pub wraps: bool,
  pub label: Arc<str>,
  pub disabled: bool,
  /// The style of each part of this dial.
  pub recipe: ResolvedRecipe,
  value_text: AngleText,
  model: Model,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl AngleSliderView {
  /// The style of a part, as this dial is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// What it says its angle is: "45°".
  pub fn text(&self) -> String {
    (self.value_text)(self.angle)
  }

  /// A length of a part in pixels, where the recipe gives it so.
  fn px(&self, length: Option<Length>, or: f32) -> f32 {
    match length.and_then(|length| length.resolve(self.theme())) {
      Some(Dimension::Px(px)) => px,
      _ => or,
    }
  }

  /// How far the middle of the handle is from the middle of the dial: inside its ring, touching it.
  pub fn reach(&self) -> f32 {
    let root = self.part(AngleSliderPart::Root).rest();
    let side = self.px(root.width, 64.0);
    let ring = root
      .border
      .as_ref()
      .map_or(0.0, |border| border.width.resolve(&self.theme().border_sizes()));
    let handle = self.px(self.part(AngleSliderPart::Handle).rest().width, 9.0);
    side / 2.0 - ring - handle / 2.0
  }

  /// The dial with the style of its root, with nothing in it yet: what is dragged, pressed, focused and moved by the
  /// keys, a slider to tooling that says its angle.
  pub fn root(&self) -> Stack {
    let text = self.text();
    let mut root = Stack::new()
      .stack_align(StackAlignment::Center)
      .overflow_visible()
      .part(&self.part(AngleSliderPart::Root))
      .ref_element(self.model.root.clone())
      .describe("a11y_role", "slider")
      .describe("a11y_name", self.label.clone())
      .describe("a11y_valuemin", number(self.min, 9))
      .describe("a11y_valuemax", number(self.max, 9))
      .describe("a11y_valuenow", number(self.angle, 9))
      .describe("a11y_valuetext", text);
    if self.disabled {
      root = root.describe("a11y_disabled", "true");
    } else {
      let (press, drag, release, keys) = (
        self.model.clone(),
        self.model.clone(),
        self.model.clone(),
        self.model.clone(),
      );
      let (touch_start, touch_move, touch_end) = (self.model.clone(), self.model.clone(), self.model.clone());
      root = root
        .tab_index(0)
        // A press of it focuses it, as a press of the engine's slider does: its keys go on from there.
        .focuses(&self.model.root)
        .start_drag_buttons(MouseButtonMask::LEFT)
        .on_drag_start(move |event: DragEvent| press.press(event.x, event.y, None))
        .on_drag_move(move |event: DragEvent| drag.moved(event.x, event.y, None))
        .on_drag_end(move |_: DragEvent| release.released(None))
        // A finger or a pen on the dial turns it, and is taken from what would scroll with it.
        .on_touch_start(move |event: TouchEvent| {
          event.prevent_default();
          touch_start.press(event.x, event.y, Some(event.id));
        })
        .on_touch_move(move |event: TouchEvent| touch_move.moved(event.x, event.y, Some(event.id)))
        .on_touch_end(move |event: TouchEvent| touch_end.released(Some(event.id)))
        .on_touch_cancel({
          let model = self.model.clone();
          move |event: TouchEvent| model.released(Some(event.id))
        })
        .on_key_down(move |event: KeyboardEvent| {
          if !event.focus_within || event.ctrl || event.alt || event.meta || event.composing {
            return;
          }
          let was = keys.current();
          let Some(next) = keys.range.key(was, &event.key) else {
            return;
          };
          event.prevent_default();
          keys.set(next, was);
          keys.end(was, next);
        });
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// The angle in the middle of the dial.
  pub fn value(&self) -> Element {
    let part = self.part(AngleSliderPart::Value);
    part.text(&self.text()).pointer_events_none().into()
  }

  /// The dot on the dial's ring at the angle.
  pub fn handle(&self) -> Element {
    let radians = self.angle.to_radians() as f32;
    let reach = self.reach();
    Row::new()
      .part(&self.part(AngleSliderPart::Handle))
      .offset(reach * radians.sin(), -reach * radians.cos())
      .pointer_events_none()
      .into()
  }

  /// How the default renderer builds a dial: the angle in its middle and the dot on its ring.
  pub fn render(self) -> Element {
    self.root().child(self.value()).child(self.handle()).into()
  }
}

impl Themed for AngleSlider {
  const NAME: &'static str = "angle_slider";
  type Renderer = AngleSliderRenderer;

  /// The design's angle slider (`pencil/lurq-design-system.pen`, Angle Slider, in Advanced Inputs): a circle of 64 on
  /// the background with a ring of 1.5 in the input's color inside it, the angle in the middle in the mono type of 13
  /// in the foreground's color, and a dot of 9 in the primary color inside the ring at the angle. The board draws it
  /// at rest: the ring of the focus is around the circle, the pointer is a hand over it, and a disabled one is
  /// dimmed.
  fn recipe() -> Recipe {
    use AngleSliderPart::{Handle, Root, Value};
    use PaletteColor::{Background, Foreground, Input, Primary};
    Recipe::new()
      .base(Root, |s| {
        fading(
          s.square(64.0)
            .radius(RadiusSize::Full)
            .background(Background)
            .border(Border::inside(1.5, Input))
            .cursor(CursorIcon::Pointer),
        )
      })
      .base(Value, |s| {
        s.font_family(FontFamily::Mono).font_size(13.0).foreground(Foreground)
      })
      .base(Handle, |s| s.square(9.0).radius(RadiusSize::Full).background(Primary))
      .focus_visible(Root, |s| s.focus_ring())
      .state(AngleSliderState::Disabled, Root, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
  }
}

impl super::FieldControl for AngleSlider {
  /// Called by the field's label unless it has a name of its own, and disabled where the field is. A dial has no
  /// judgment.
  fn in_field(mut self, field: &super::InField) -> Element {
    if self.label.is_none() {
      self.label = Some(field.label.clone());
    }
    self.disabled |= field.disabled;
    self.into()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_whole_turn_goes_round_and_a_part_of_one_stops_at_its_ends() {
    let turn = Range::new(0.0, 360.0, 1.0, None, true);
    assert!(turn.wraps);
    assert_eq!(turn.held(360.0), 0.0);
    assert_eq!(turn.held(-1.0), 359.0);
    assert_eq!(turn.key(359.0, "ArrowRight"), Some(0.0));
    assert_eq!(turn.key(0.0, "ArrowLeft"), Some(359.0));
    assert_eq!(turn.key(350.0, "PageUp"), Some(5.0));
    assert_eq!(turn.key(10.0, "End"), Some(359.0));
    let held = Range::new(0.0, 360.0, 1.0, None, false);
    assert_eq!(held.key(360.0, "ArrowRight"), Some(360.0));
    assert_eq!(held.key(10.0, "End"), Some(360.0));
    let half = Range::new(-90.0, 90.0, 15.0, None, true);
    assert!(!half.wraps, "less than a turn does not go round");
    assert_eq!(half.key(90.0, "ArrowRight"), Some(90.0));
    assert_eq!(half.held(52.0), 45.0);
  }

  #[test]
  fn a_press_goes_to_the_angle_under_it_and_a_drag_goes_on_round() {
    let turn = Range::new(0.0, 360.0, 1.0, None, true);
    assert_eq!(turn.pressed(10.0, 90.0), 90.0);
    assert_eq!(turn.pressed(10.0, 300.4), 300.0);
    assert_eq!(turn.dragged(350.0, 20.0), 10.0);
    let held = Range::new(0.0, 360.0, 1.0, None, false);
    assert_eq!(held.held(held.dragged(350.0, 20.0)), 360.0, "held at its end");
    assert_eq!(held.dragged(350.0, 400.0), 540.0, "no more than half a turn past it");
    let two = Range::new(0.0, 720.0, 1.0, None, true);
    assert_eq!(two.pressed(400.0, 90.0), 450.0, "the turn nearer to the angle");
    let half = Range::new(0.0, 180.0, 1.0, None, true);
    assert_eq!(half.pressed(90.0, 200.0), 180.0, "past an end, the nearer end");
    assert_eq!(half.pressed(90.0, 350.0), 0.0);
    let signed = Range::new(-180.0, 180.0, 1.0, None, true);
    assert_eq!(signed.pressed(0.0, 270.0), -90.0);
  }

  #[test]
  fn bearings_are_a_compass_s() {
    assert_eq!(bearing(0.0, -10.0), 0.0);
    assert_eq!(bearing(10.0, 0.0), 90.0);
    assert_eq!(bearing(0.0, 10.0), 180.0);
    assert_eq!(bearing(-10.0, 0.0), 270.0);
    assert_eq!(number(22.5, decimals_of(0.5)), "22.5");
    assert_eq!(number(45.0, decimals_of(1.0)), "45");
  }
}
