//! A seek bar: where a player is in what it plays, how much of it is loaded, and the handle that moves it; over
//! the pointer, the time it is on, and a picture of that moment where the app has one.

use std::sync::{
  Arc, Mutex,
  atomic::{AtomicBool, Ordering},
};

use super::{
  FontFamily, media_time_text, names,
  part_style::{Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  slider::engine_part,
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::{CollisionStrategy, Ctx, Overlay, Placement},
    events::{KeyboardEvent, MouseEvent},
    theme::{OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{self, Column, Row, Stack},
  core::{ElementRef, Signal},
  layout::{Alignment, StackAlignment},
  node::{CursorIcon, Element, HitTestBehavior, SliderPartStyle, border, color::Color, dimension::Dimension},
};

/// The parts of a seek bar a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SeekBarPart {
  /// The bar: as wide as it is given, as high as what is pressed on it.
  Root,
  /// The rail across the bar, from the start of what is played to its end, cut into its chapters.
  Track,
  /// One chapter of the track; the room between two is its padding at the side it shares with the other.
  Chapter,
  /// What is played: from the start to the thumb.
  Played,
  /// What is loaded and not played yet: from the thumb to as far as it is loaded.
  Buffered,
  /// The rest of the track.
  Rest,
  /// What is dragged, pressed and focused: the engine's slider over the track, as large as the bar.
  Control,
  /// The handle at the position, on the control.
  Thumb,
  /// What stands over the pointer: the preview, then the time.
  Hover,
  /// The frame the app's picture of the moment under the pointer is shown in.
  Preview,
  /// The time under the pointer, in a chip.
  Tip,
}

names!(SeekBarPart {
  Root => "root",
  Track => "track",
  Chapter => "chapter",
  Played => "played",
  Buffered => "buffered",
  Rest => "rest",
  Control => "control",
  Thumb => "thumb",
  Hover => "hover",
  Preview => "preview",
  Tip => "tip",
});

/// The states of a seek bar a recipe styles, besides what the pointer and the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SeekBarState {
  /// It takes no input.
  Disabled,
  /// It stands over a picture: white, the same in a light and a dark theme.
  OnMedia,
  /// Its track is cut into chapters.
  Chapters,
  /// It shows how far what is played is loaded.
  Buffered,
  /// The app gives a picture of the moment under the pointer.
  Preview,
  /// Nothing is loaded: the track alone, with no thumb, and it takes no input.
  Empty,
  /// A live stream with nothing to go back in: full, the thumb at the end, and it takes no input.
  Live,
}

names!(SeekBarState {
  Disabled => "disabled",
  OnMedia => "on_media",
  Chapters => "chapters",
  Buffered => "buffered",
  Preview => "preview",
  Empty => "empty",
  Live => "live",
});

/// The sizes of a seek bar: the one of a desktop's player, and the one a finger drags on a phone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SeekBarSize {
  /// 16 high, the thumb 16 (12 over a picture).
  #[default]
  Md,
  /// A phone's: 28 high with a thumb of 28 on a surface, the thumb 16 over a picture.
  Lg,
}

names!(SeekBarSize {
  Md => "md",
  Lg => "lg",
});

/// What builds a seek bar's nodes: a theme's in place of the default one.
pub type SeekBarRenderer = Arc<dyn Fn(SeekBarView) -> Element + Send + Sync>;

/// What hears a time, or a fraction of a whole.
pub(super) type SeekHandler = Arc<dyn Fn(f64) + Send + Sync>;

/// What builds the picture of a moment, given its time in seconds.
pub type SeekPreview = Arc<dyn Fn(f64) -> Element + Send + Sync>;

/// A seek bar.
///
/// ```
/// use lurq::{core::Signal, ui::SeekBar};
///
/// // Bound to the app's position, in seconds, of a recording 250 seconds long.
/// let position = Signal::new(84.0_f32);
/// let bar = SeekBar::new(position, 250.0).buffered(140.0).on_change_end(|to| println!("seek to {to}"));
/// // The app holds the position and is told where a person moves it.
/// let held = SeekBar::controlled(84.0, 250.0, |to| println!("{to}"));
/// ```
///
/// It is the engine's slider dressed by the recipe: a drag, a press on the track and the keys move it. The arrow
/// keys move it a [`step`](SeekBar::step) (5 seconds), Page Up and Page Down a tenth of the whole in whole steps,
/// Home and End to the start and the end. [`SeekBar::on_change`] hears the position as a drag moves it,
/// [`SeekBar::on_change_end`] where it is let go: a player that seeks only when the handle is let go listens to the
/// second.
///
/// Mounted ([`SeekBar::mount`]) it shows the time under the pointer in a chip over the bar, and the app's
/// [`preview`](SeekBar::preview) of that moment over the chip, and tells [`SeekBar::on_hover`] the time under the
/// pointer. A bar given to a tree as it is (`.child(SeekBar::new(..))`) is the same component, declared: what
/// renders the tree mounts it. [`SeekBar::build`] is a still picture of it, for a test.
///
/// [`SeekBar::empty`] is a bar with nothing loaded (no thumb, no input); [`SeekBar::live`] a live stream with
/// nothing to go back in (the bar full, the thumb at its end, no input).
#[derive(Clone)]
pub struct SeekBar {
  position: Signal<f32>,
  controlled: Option<f64>,
  duration: f64,
  buffered: Option<f64>,
  chapters: Vec<f64>,
  step: f64,
  on_media: bool,
  empty: bool,
  live: bool,
  size: Name,
  label: Arc<str>,
  disabled: bool,
  on_change: Vec<SeekHandler>,
  on_change_end: Vec<SeekHandler>,
  on_hover: Vec<Arc<dyn Fn(Option<f64>) + Send + Sync>>,
  preview: Option<SeekPreview>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
  /// Of a mounted bar, what it keeps between renders: the time under the pointer, what the chip hangs from, and
  /// how wide the bar is laid out.
  hover: Option<Hover>,
}

impl SeekBar {
  /// A seek bar bound to `position`, in seconds, of something `duration` seconds long. What a person does sets
  /// the signal.
  pub fn new(position: Signal<f32>, duration: impl Into<f64>) -> Self {
    Self {
      position,
      controlled: None,
      duration: duration.into(),
      buffered: None,
      chapters: Vec::new(),
      step: 5.0,
      on_media: false,
      empty: false,
      live: false,
      size: SeekBarSize::default().into(),
      label: Arc::from("Seek"),
      disabled: false,
      on_change: Vec::new(),
      on_change_end: Vec::new(),
      on_hover: Vec::new(),
      preview: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
      hover: None,
    }
  }

  /// A seek bar that shows `position`, held by the app, and tells `on_change` where a person moves it.
  pub fn controlled(
    position: impl Into<f64>,
    duration: impl Into<f64>,
    on_change: impl Fn(f64) + Send + Sync + 'static,
  ) -> Self {
    let position = position.into();
    let mut bar = Self::new(Signal::new(position as f32), duration).on_change(on_change);
    bar.controlled = Some(position);
    bar
  }

  /// How far what is played is loaded, in seconds from its start: drawn between what is played and the rest.
  pub fn buffered(mut self, buffered: impl Into<f64>) -> Self {
    self.buffered = Some(buffered.into());
    self
  }

  /// Where the chapters start, in seconds: the bar is cut there. The first starts at 0, given or not.
  pub fn chapters(mut self, starts: &[f64]) -> Self {
    self.chapters = starts.to_vec();
    self
  }

  /// How many seconds an arrow key moves it: 5 unless told.
  pub fn step(mut self, step: impl Into<f64>) -> Self {
    self.step = step.into();
    self
  }

  /// Whether it stands over a picture: white, the same in a light and a dark theme.
  pub fn on_media(mut self, on_media: bool) -> Self {
    self.on_media = on_media;
    self
  }

  /// Whether nothing is loaded: the track alone, with nothing played or loaded and no thumb, and it takes no input.
  /// A player draws it while it loads and after an error.
  pub fn empty(mut self, empty: bool) -> Self {
    self.empty = empty;
    self
  }

  /// Whether what plays is a live stream with nothing to go back in: the bar is full with the thumb at its end, and
  /// a press and the keys tell nothing. It is no stop of Tab.
  pub fn live(mut self, live: bool) -> Self {
    self.live = live;
    self
  }

  /// A [`SeekBarSize`], or a size the theme's recipe names: [`SeekBarSize::Lg`] on a phone.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// What it is called, to what reads the tree: "Seek" unless told.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Arc::from(label);
    self
  }

  /// Whether it takes no input: half its opacity, and nothing of what is loaded.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Called with the position, in seconds, each time a person moves it. Each call adds a handler.
  pub fn on_change(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Called with the position, in seconds, when a drag is let go or a key's step is made, where it moved. Each
  /// call adds a handler.
  pub fn on_change_end(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.on_change_end.push(Arc::new(handler));
    self
  }

  /// Called with the time under the pointer, in seconds, as the pointer moves over a mounted bar, and with `None`
  /// when it leaves. Each call adds a handler.
  pub fn on_hover(mut self, handler: impl Fn(Option<f64>) + Send + Sync + 'static) -> Self {
    self.on_hover.push(Arc::new(handler));
    self
  }

  /// What shows the moment under the pointer, given its time in seconds: a frame of the video, shown in a frame of
  /// 128 by 72 over the time. Shown by a mounted bar.
  pub fn preview(mut self, preview: impl Fn(f64) -> Element + Send + Sync + 'static) -> Self {
    self.preview = Some(Arc::new(preview));
    self
  }

  /// Puts the bar in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this bar over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this bar: what the component it is in says of it in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws it as it is under the pointer or focused from the keyboard, with nothing pointing at it: for a gallery
  /// or a test. Mounted and drawn under the pointer, it shows the time of its position in the chip.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of what is dragged and focused.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Puts the bar where it goes, keeping the time under the pointer between renders: it shows the chip with the
  /// time, and the preview over it.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    ctx.mount::<SeekBarHost>(SeekBarProps { bar: self })
  }

  /// Builds a still picture of the seek bar in `theme`, for a test or a screenshot: no chip, nothing read of a
  /// pointer that does not press it. An app gives the bar to its tree, or mounts it ([`SeekBar::mount`]).
  pub fn build(self, theme: &Theme) -> Element {
    let duration = if self.duration.is_finite() {
      self.duration.max(0.0)
    } else {
      0.0
    };
    let mut chapters: Vec<f64> = self
      .chapters
      .iter()
      .copied()
      .filter(|start| start.is_finite() && *start > 0.0 && *start < duration)
      .collect();
    chapters.sort_by(f64::total_cmp);
    chapters.dedup();
    let still = self.disabled || self.empty || self.live;
    let buffered = self.buffered.filter(|_| !still).map(|to| to.clamp(0.0, duration));
    let case = Case {
      variant: None,
      size: Some(self.size.clone()),
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(SeekBarState::Disabled, self.disabled)
    .state(SeekBarState::OnMedia, self.on_media)
    .state(SeekBarState::Chapters, !chapters.is_empty())
    .state(SeekBarState::Buffered, buffered.is_some())
    .state(SeekBarState::Empty, self.empty)
    .state(SeekBarState::Live, self.live && !self.empty)
    .state(SeekBarState::Preview, self.preview.is_some());
    // What reads the position is rendered again when it changes.
    let position = match self.controlled {
      Some(held) => held,
      None => f64::from(self.position.get()),
    };
    let position = if position.is_finite() {
      position.clamp(0.0, duration)
    } else {
      0.0
    };
    // Only a mounted bar shows the chip: it is placed by the width the bar was laid out at, which it keeps.
    let hovered = match (&self.hover, still) {
      (Some(hover), false) => hover
        .time
        .get()
        .or((self.forced == Some(Interaction::Hovered)).then_some(position)),
      _ => None,
    };
    // The still form holds no element ref: one made anew each render makes the bar a new element to the runtime
    // each time, and it loses the focus and the drag at the first change it makes.
    let (hover, anchor, room, held) = match self.hover {
      Some(Hover {
        time,
        anchor,
        width,
        held,
      }) => (Some(time), Some(anchor), width.get(), held),
      None => (None, None, 0.0, Arc::default()),
    };
    let view = SeekBarView {
      position,
      duration,
      buffered,
      chapters,
      step: self.step,
      label: self.label,
      disabled: self.disabled,
      on_media: self.on_media,
      empty: self.empty,
      live: self.live && !self.empty,
      size: self.size,
      hovered,
      room,
      recipe: theme.resolve::<SeekBar>(&case, Some(&self.rules)),
      bound: self.controlled.is_none().then_some(self.position),
      on_change: self.on_change,
      on_change_end: self.on_change_end,
      on_hover: self.on_hover,
      hover,
      held,
      preview: self.preview,
      anchor,
      forced: self.forced,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<SeekBar>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

/// A bar in a tree is its component, declared: the component that renders the tree mounts it.
impl crate::app::component::IntoComponent for SeekBar {
  type Component = SeekBarHost;

  fn into_props(self) -> SeekBarProps {
    SeekBarProps { bar: self }
  }

  fn component_key(&self) -> Option<Arc<str>> {
    self.key.clone()
  }
}

/// A seek bar, mounted.
#[doc(hidden)]
#[derive(Clone)]
pub struct SeekBarProps {
  bar: SeekBar,
}

/// The bar is built anew by every render of what holds it, so props are never the same as before.
impl PartialEq for SeekBarProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for SeekBarProps {}

/// What a mounted seek bar keeps between renders: the time under the pointer, what the chip hangs from, and how
/// wide the bar is laid out, which places the chip over the time.
#[derive(Clone)]
struct Hover {
  time: Signal<Option<f64>>,
  anchor: ElementRef,
  width: Signal<f32>,
  /// Whether the bar is held by the pointer: from a press on it to the end of what the press changes.
  held: Arc<AtomicBool>,
}

/// A seek bar, mounted.
#[doc(hidden)]
pub struct SeekBarHost {
  hover: Hover,
}

impl Component for SeekBarHost {
  type Props = SeekBarProps;

  fn create(ctx: &mut Ctx) -> Self {
    let (anchor, width) = (ElementRef::new(), ctx.signal(0.0_f32));
    {
      let width = width.clone();
      // Only a change of a whole pixel: what the chip stands over does not move for less.
      anchor.observe_rect_before_paint(move |rect| {
        if (width.get_untracked() - rect.width).abs() >= 1.0 {
          width.set(rect.width);
        }
      });
    }
    Self {
      hover: Hover {
        time: ctx.signal(None),
        anchor,
        width,
        held: Arc::default(),
      },
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let mut bar = ctx.props::<SeekBarProps>().bar.clone();
    bar.hover = Some(self.hover.clone());
    bar.build(&current_theme())
  }
}

/// A value the engine's slider holds as an `f32`, as the app is told it: as it is written (84.4, not
/// 84.4000015258789).
pub(super) fn told(value: f32) -> f64 {
  value.to_string().parse().unwrap_or(f64::from(value))
}

/// Where a key moves a seek surface at `now` in `0..=max` by `step`: an arrow a step, Page Up and Page Down a tenth
/// of the whole in whole steps and at least one, Home and End to the ends. `None` for another key.
pub(super) fn keyed(key: &str, now: f64, max: f64, step: f64) -> Option<f64> {
  let step = if step.is_finite() && step > 0.0 {
    step
  } else {
    max / 20.0
  };
  let page = (max / 10.0 / step).round().max(1.0) * step;
  let to = match key {
    "ArrowRight" | "ArrowUp" => now + step,
    "ArrowLeft" | "ArrowDown" => now - step,
    "PageUp" => now + page,
    "PageDown" => now - page,
    "Home" => 0.0,
    "End" => max,
    _ => return None,
  };
  Some(to.clamp(0.0, max))
}

/// The engine's slider of a seek surface at `now` in `0..=max`: what a drag, a press on the track and the keys
/// move, telling `on_change` as it moves and `on_change_end` where it ends. Bound, the app's signal is set first;
/// held by the app (`bound` none), it shows `now` until the app answers.
///
/// The engine's slider is bound to a signal of its own, made anew each render at `now`: its range and its step,
/// set as it is built, snap and keep in range what it holds, which is not to change the app's position. It is not
/// snapped at all (a step of nothing): a drag goes where the pointer is, and the keys' step is the bar's own.
pub(super) fn seek_control(
  now: f64,
  max: f64,
  step: f64,
  bound: Option<Signal<f32>>,
  on_change: &[SeekHandler],
  on_change_end: &[SeekHandler],
  disabled: bool,
) -> components::Slider {
  let shown = Signal::new(now as f32);
  let mut slider = components::Slider::new_f32(shown.clone())
    .range_f32(0.0, max as f32)
    .step(f32::EPSILON)
    .tab_index(0)
    .disabled(disabled);
  if disabled {
    return slider;
  }
  let tell = {
    let (bound, change) = (bound.clone(), on_change.to_vec());
    Arc::new(move |to: f64| {
      if let Some(bound) = &bound {
        bound.set(to as f32);
      }
      for handler in &change {
        handler(to);
      }
    })
  };
  {
    let (tell, shown, held) = (tell.clone(), shown.clone(), bound.is_none());
    // Held by the app, it goes back to what the app holds until the app answers, so the engine's slider sees the
    // place the pointer is let go at as a change again: what was just told is not told twice.
    let last = Mutex::new(None::<f64>);
    slider = slider.on_change(move |low, _| {
      let to = told(low);
      if held {
        let mut last = last.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if *last == Some(to) {
          shown.set(now as f32);
          return;
        }
        *last = Some(to);
      }
      tell(to);
      if held {
        shown.set(now as f32);
      }
    });
  }
  if !on_change_end.is_empty() {
    let end = on_change_end.to_vec();
    slider = slider.on_change_end(move |low, _| {
      for handler in &end {
        handler(told(low));
      }
    });
  }
  // The keys are the bar's: a step of seconds, which a drag does not snap to.
  let end = on_change_end.to_vec();
  slider.on_key_down(move |event: KeyboardEvent| {
    if !event.focus_within || event.ctrl || event.alt || event.meta {
      return;
    }
    let from = f64::from(shown.get_untracked());
    let Some(to) = keyed(event.key.as_str(), from, max, step) else {
      return;
    };
    event.prevent_default();
    if (to - from).abs() < 1e-9 {
      return;
    }
    if bound.is_some() {
      shown.set(to as f32);
    }
    tell(to);
    for handler in &end {
      handler(to);
    }
  })
}

/// A seek bar as a renderer sees it: what it was given, the style of its parts, and its behavior ready to be
/// dressed.
pub struct SeekBarView {
  /// Where it is, in seconds, kept within the whole.
  pub position: f64,
  /// How long the whole is, in seconds: 0 where it is not known.
  pub duration: f64,
  /// How far it is loaded, in seconds; none where it is not told or the bar is disabled.
  pub buffered: Option<f64>,
  /// Where the chapters after the first start, in seconds, in order, each within the whole.
  pub chapters: Vec<f64>,
  /// How many seconds an arrow key moves it.
  pub step: f64,
  pub label: Arc<str>,
  pub disabled: bool,
  pub on_media: bool,
  /// Nothing is loaded: no thumb, nothing played, no input.
  pub empty: bool,
  /// A live stream with nothing to go back in: full, the thumb at the end, no input.
  pub live: bool,
  pub size: Name,
  /// The time the chip shows: under the pointer of a mounted bar, or the position where it is drawn hovered.
  pub hovered: Option<f64>,
  /// How wide a mounted bar was laid out: what places the chip over the time. 0 for the still form.
  pub room: f32,
  /// The style of each part of this bar.
  pub recipe: ResolvedRecipe,
  bound: Option<Signal<f32>>,
  on_change: Vec<SeekHandler>,
  on_change_end: Vec<SeekHandler>,
  on_hover: Vec<Arc<dyn Fn(Option<f64>) + Send + Sync>>,
  hover: Option<Signal<Option<f64>>>,
  /// Whether a mounted bar is held by the pointer, kept between its renders.
  held: Arc<AtomicBool>,
  preview: Option<SeekPreview>,
  /// What the chip hangs from and the pointer is read against: a mounted bar's.
  anchor: Option<ElementRef>,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl SeekBarView {
  /// The style of a part, as this bar is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// How far along the whole `seconds` is, from 0 to 1.
  pub fn fraction(&self, seconds: f64) -> f64 {
    if self.duration > 0.0 {
      (seconds / self.duration).clamp(0.0, 1.0)
    } else {
      0.0
    }
  }

  /// Whether a person can move it: it is not disabled, empty or live.
  pub fn operated(&self) -> bool {
    !self.disabled && !self.empty && !self.live
  }

  /// How far along the whole what is played is, from 0 to 1: all of it live, nothing of it empty.
  pub fn played(&self) -> f64 {
    if self.empty {
      0.0
    } else if self.live {
      1.0
    } else {
      self.fraction(self.position)
    }
  }

  /// A time as the bar reads it out: with hours where the whole is an hour or longer.
  pub fn time_text(&self, seconds: f64) -> String {
    media_time_text(seconds, self.duration >= 3600.0)
  }

  /// How wide the thumb is: the engine's slider runs its middle from half of it in from the start to half of it
  /// in from the end, and the track is drawn to match.
  pub fn thumb_width(&self) -> f32 {
    self.px(self.part(SeekBarPart::Thumb).rest().width).unwrap_or(16.0)
  }

  fn px(&self, length: Option<Length>) -> Option<f32> {
    match length?.resolve(self.theme())? {
      Dimension::Px(px) => Some(px),
      _ => None,
    }
  }

  /// The track: what is played, what is loaded after it and the rest, cut into the chapters. Its ends are half the
  /// thumb wide each and its middle is the room the thumb's middle runs along, so that what is played ends under
  /// the thumb's middle.
  pub fn track(&self) -> Row {
    let half = self.thumb_width() / 2.0;
    let (played, buffered) = (self.played(), self.buffered.map(|to| self.fraction(to)));
    let piece = |part: SeekBarPart| Row::new().part(&self.part(part));
    let mut starts = vec![0.0];
    starts.extend(self.chapters.iter().map(|start| self.fraction(*start)));
    let count = starts.len();
    let mut middle = Row::new()
      .flex_full(1.0, 1.0, Some(0.0))
      .min_width(0.0)
      .align_items(Alignment::Stretch);
    for (index, from) in starts.iter().copied().enumerate() {
      let to = starts.get(index + 1).copied().unwrap_or(1.0);
      let length = to - from;
      if length <= 0.0 {
        continue;
      }
      let share = |until: f64| ((until - from) / length).clamp(0.0, 1.0);
      let mut chapter = Row::new()
        .part(&self.part(SeekBarPart::Chapter))
        .width(Dimension::Pct((length * 100.0) as f32))
        .align_items(Alignment::Stretch);
      // The room between two chapters is theirs: the ends of the track have none.
      if index == 0 {
        chapter = chapter.padding_left(0.0);
      }
      if index + 1 == count {
        chapter = chapter.padding_right(0.0);
      }
      let played_share = share(played);
      if played_share > 0.0 {
        chapter = chapter.child(piece(SeekBarPart::Played).width(Dimension::Pct((played_share * 100.0) as f32)));
      }
      if let Some(buffered) = buffered {
        let loaded = share(buffered) - played_share;
        if loaded > 0.0 {
          chapter = chapter.child(piece(SeekBarPart::Buffered).width(Dimension::Pct((loaded * 100.0) as f32)));
        }
      }
      middle = middle.child(chapter.child(piece(SeekBarPart::Rest).flex_full(1.0, 1.0, Some(0.0))));
    }
    // The end is under the thumb at the end: played there where all is, loaded where all is loaded.
    let tail = if played >= 1.0 {
      SeekBarPart::Played
    } else if buffered.is_some_and(|buffered| buffered >= 1.0) {
      SeekBarPart::Buffered
    } else {
      SeekBarPart::Rest
    };
    Row::new()
      .part(&self.part(SeekBarPart::Track))
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Stretch)
      .describe("a11y_hidden", "true")
      .child(piece(SeekBarPart::Played).width(half))
      .child(middle)
      .child(piece(tail).width(half))
  }

  /// The engine's slider over the track, as large as the bar: what is dragged, pressed, moved by the keys and
  /// focused, bound to the position and telling the bar's handlers of what a person does, and the time under the
  /// pointer to `on_hover` and to the chip. Its track and its fill are not drawn: [`SeekBarView::track`] draws
  /// them. Its thumb is not styled yet: [`SeekBarView::dressed`] gives it the recipe's.
  pub fn control(&self) -> components::Slider {
    // Live, the thumb is at the end of a whole that may have no length: the engine's slider runs from 0 to 1.
    let (now, max) = if self.live {
      (1.0, 1.0)
    } else if self.empty {
      (0.0, self.duration)
    } else {
      (self.position, self.duration)
    };
    // What tells the time under the pointer, to the chip and to the app: a mounted bar's, while it is operated.
    let tell: Option<Arc<dyn Fn(Option<f64>) + Send + Sync>> = match (self.operated(), &self.anchor) {
      (true, Some(_)) => {
        let last = Mutex::new(self.hovered.filter(|_| self.hover.is_some()));
        let (hover, handlers) = (self.hover.clone(), self.on_hover.clone());
        Some(Arc::new(move |time: Option<f64>| {
          let mut held = last.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
          if *held == time {
            return;
          }
          *held = time;
          drop(held);
          if let Some(hover) = &hover {
            hover.set(time);
          }
          for handler in &handlers {
            handler(time);
          }
        }))
      }
      _ => None,
    };
    let (mut on_change, mut on_change_end) = (self.on_change.clone(), self.on_change_end.clone());
    if let Some(tell) = &tell {
      // The runtime gives a slider that is dragged every move of the pointer and tells no one else of them: the
      // time under the pointer is then the one the bar is dragged to.
      let (tell, held) = (tell.clone(), self.held.clone());
      on_change.push(Arc::new(move |to| {
        if held.load(Ordering::Relaxed) {
          tell(Some((to * 100.0).round() / 100.0));
        }
      }));
      let held = self.held.clone();
      on_change_end.push(Arc::new(move |_| held.store(false, Ordering::Relaxed)));
    }
    let mut slider = seek_control(
      now,
      max,
      self.step,
      self.bound.clone(),
      &on_change,
      &on_change_end,
      !self.operated(),
    )
    .part(&self.part(SeekBarPart::Control))
    .width(Dimension::Pct(100.0))
    .height(Dimension::Pct(100.0))
    .describe("a11y_role", "slider")
    .describe("a11y_name", self.label.clone())
    .describe(
      "a11y_value",
      if self.live {
        "live".to_owned()
      } else {
        format!("{} of {}", self.time_text(self.position), self.time_text(self.duration))
      },
    );
    if let (Some(tell), Some(anchor)) = (tell, &self.anchor) {
      let (anchor, duration, half) = (anchor.clone(), self.duration, self.thumb_width() / 2.0);
      let (moved, pressed, left) = (tell.clone(), self.held.clone(), self.held.clone());
      slider = slider
        .on_mouse_down(move |_: MouseEvent| pressed.store(true, Ordering::Relaxed))
        .on_mouse_move(move |event: MouseEvent| {
          let travel = anchor.width() - 2.0 * half;
          let along = if travel > 0.0 {
            f64::from(((event.x - anchor.x() - half) / travel).clamp(0.0, 1.0))
          } else {
            0.0
          };
          // To the hundredth of a second: a move within one tells nothing.
          moved(Some((along * duration * 100.0).round() / 100.0));
        })
        .on_mouse_leave(move || {
          left.store(false, Ordering::Relaxed);
          tell(None);
        });
    }
    if let Some(id) = &self.id {
      slider = slider.id(id.clone());
    }
    slider
  }

  /// `slider` with its thumb in the style the recipe gives it, at rest, under the pointer and focused from the
  /// keyboard, and its track as thick as the recipe's and not painted.
  pub fn dressed(&self, slider: components::Slider) -> components::Slider {
    let theme = self.theme();
    let thumb = self.part(SeekBarPart::Thumb);
    let thickness = self.px(self.part(SeekBarPart::Track).rest().height).unwrap_or(4.0);
    let mut slider = slider
      .track_style(
        SliderPartStyle::new()
          .height(thickness)
          .background(Color::new(0, 0, 0, 0)),
      )
      .thumb_style(engine_part(&thumb.rest(), theme));
    if !thumb.styles().hovered.is_empty() {
      slider = slider.thumb_hovered_style(engine_part(&thumb.styles().hovered, theme));
    }
    if !thumb.styles().focus_visible.is_empty() {
      slider = slider.thumb_focused_style(engine_part(&thumb.styles().focus_visible, theme));
    }
    slider
  }

  /// What stands over the pointer: the app's preview of the moment in its frame, and the time in the chip, above
  /// the bar and centred on where the time is along it. `None` while no time is under the pointer.
  pub fn hover_layer(&self) -> Option<Element> {
    let (time, anchor) = (self.hovered?, self.anchor.clone()?);
    let tip = self.part(SeekBarPart::Tip);
    let mut column = Column::new()
      .part(&self.part(SeekBarPart::Hover))
      .align_items(Alignment::Center)
      .describe("a11y_hidden", "true");
    if let Some(preview) = &self.preview {
      column = column.child(
        Stack::new().part(&self.part(SeekBarPart::Preview)).child(
          Row::new()
            .width(Dimension::Pct(100.0))
            .height(Dimension::Pct(100.0))
            .child(preview(time)),
        ),
      );
    }
    column = column.child(Row::new().part(&tip).child(tip.text(&self.time_text(time)).nowrap()));
    let half = self.thumb_width() / 2.0;
    let width = self.room;
    let at = half + self.fraction(time) as f32 * (width - 2.0 * half).max(0.0);
    Some(
      Overlay::new(column)
        .anchor(anchor)
        .placement(Placement::Top)
        .offset(at - width / 2.0, 0.0)
        .collision(CollisionStrategy::Clamp)
        .hit_test(HitTestBehavior::None)
        .into(),
    )
  }

  /// How the default renderer builds a seek bar: the track, the engine's slider over it, and what stands over the
  /// pointer.
  pub fn render(self) -> Element {
    let mut root = Stack::new()
      .stack_align(StackAlignment::Center)
      .part(&self.part(SeekBarPart::Root))
      // The ring of the focus is around the thumb, past the bar's edge.
      .overflow_visible()
      .flex_shrink(1.0)
      .min_width(0.0)
      .child(match self.hover_layer() {
        // The layer is declared in the track, under the engine's slider: where it is declared it is a point of no
        // size, at the middle of a stack, and the pointer on that point would be on it and not on the slider.
        Some(layer) => self.track().child(layer),
        None => self.track(),
      })
      .child(self.dressed(self.control()));
    if let Some(anchor) = &self.anchor {
      root = root.ref_element(anchor.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

impl Themed for SeekBar {
  const NAME: &'static str = "seek_bar";
  type Renderer = SeekBarRenderer;

  /// The design's seek bar (`pencil/lurq-design-system.pen`, Media/Seek Bar): 16 high, a track of 4 with ends
  /// of radius 2, played in the primary color, loaded in the muted foreground at half its opacity, the rest in the
  /// secondary color, and the Slider's thumb (16, the background's color, an edge of 1.5 in the primary color). The
  /// ring of the focus is the thumb's; disabled, the bar is at half its opacity. Over a picture (Media/Seek
  /// Bar/On Media) it is white whatever the theme: played and a solid thumb of 12 `#FFFFFF`, loaded `#FFFFFF59`, the
  /// rest `#FFFFFF33`, and the ring white. Chapters (Media/Seek Bar/Chapters) are 2 apart. Over the pointer
  /// (Media/Seek Preview) the time is in a tooltip's chip in the mono type, 4 above the bar; a preview is a frame of
  /// 128 by 72 with a radius md and an edge of 2 in white, 6 above the chip, and the two 6 above the bar. On a
  /// phone (Mobile) the bar is 28 high with a thumb of 28 edged in the border's color, and 16 over a picture.
  fn recipe() -> Recipe {
    use PaletteColor::*;
    use SeekBarPart::{Buffered, Chapter, Control, Hover, Played, Preview, Rest, Root, Thumb, Tip, Track};
    let white = || Color::from_hex("#FFFFFF");
    Recipe::new()
      .base(Root, |s| s.width(Length::Fill).height(16.0))
      .base(Track, |s| s.height(4.0).radius(2.0))
      .base(Chapter, |s| s.padding_x(1.0))
      .base(Played, |s| s.background(Primary))
      .base(Buffered, |s| s.background(MutedForeground).opacity(0.5))
      .base(Rest, |s| s.background(Secondary))
      .base(Control, |s| s.cursor(CursorIcon::Pointer))
      .base(Thumb, |s| {
        s.square(16.0)
          .radius(RadiusSize::Full)
          .background(Background)
          .border(border::Border::inside(1.5, Primary))
      })
      .focus_visible(Thumb, |s| s.focus_ring())
      .base(Hover, |s| s.gap(6.0).padding_bottom(4.0))
      .base(Preview, |s| {
        s.width(128.0)
          .height(72.0)
          .radius(RadiusSize::Md)
          .border(border::Border::inside(2.0, white()))
      })
      .base(Tip, |s| {
        s.background(Primary)
          .foreground(PrimaryForeground)
          .radius(RadiusSize::Md)
          .padding_x(10.0)
          .padding_y(6.0)
          .typography(TypographyStyle::Caption)
          .font_family(FontFamily::Mono)
      })
      .state(SeekBarState::Preview, Hover, |s| s.padding_bottom(6.0))
      .state(SeekBarState::OnMedia, Played, |s| s.background(white()))
      .state(SeekBarState::OnMedia, Buffered, |s| {
        s.background(Color::from_hex("#FFFFFF59")).opacity(1.0)
      })
      .state(SeekBarState::OnMedia, Rest, |s| {
        s.background(Color::from_hex("#FFFFFF33"))
      })
      .state(SeekBarState::OnMedia, Thumb, |s| {
        s.square(12.0)
          .background(white())
          .border(border::Border::inside(0.0, white()))
      })
      .when(
        Selector::new().state(SeekBarState::OnMedia).focus_visible(),
        Thumb,
        |s| s.outline(crate::node::outline::Outline::new(2.0, white()).offset(2.0)),
      )
      .size(SeekBarSize::Lg, Root, |s| s.height(28.0))
      .size(SeekBarSize::Lg, Thumb, |s| {
        s.square(28.0).border(border::Border::inside(1.0, PaletteColor::Border))
      })
      .when(
        Selector::new().size(SeekBarSize::Lg).state(SeekBarState::OnMedia),
        Root,
        |s| s.height(16.0),
      )
      .when(
        Selector::new().size(SeekBarSize::Lg).state(SeekBarState::OnMedia),
        Thumb,
        |s| s.square(16.0).border(border::Border::inside(0.0, white())),
      )
      .state(SeekBarState::Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
      .state(SeekBarState::Disabled, Control, |s| s.cursor(CursorIcon::Default))
      // Nothing loaded: the track alone, as the audio player draws it loading and after an error.
      .state(SeekBarState::Empty, Thumb, |s| {
        s.square(0.0).border(border::Border::inside(0.0, white()))
      })
      .state(SeekBarState::Empty, Control, |s| s.cursor(CursorIcon::Default))
      .state(SeekBarState::Live, Control, |s| s.cursor(CursorIcon::Default))
  }
}
