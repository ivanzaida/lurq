//! A waveform: the loudness of a recording as a row of bars, the played part of it marked; operated, a surface a
//! press and a drag seek on.

use std::sync::Arc;

use super::{
  names,
  part_style::{Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  seek_bar::{SeekHandler, seek_control},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{OpacityLevel, PaletteColor, RadiusSize, Theme},
  components::{self, Row, Stack},
  core::Signal,
  layout::{Alignment, StackAlignment},
  node::{CursorIcon, Element, SliderPartStyle, color::Color, dimension::Dimension},
};

/// The parts of a waveform a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WaveformPart {
  /// The waveform: as high as its tallest bar can be, as wide as its bars.
  Root,
  /// The row of the bars.
  Bars,
  /// A bar not played yet.
  Bar,
  /// A bar played.
  Played,
  /// What is pressed, dragged and focused on a waveform that is operated: the engine's slider over the bars.
  Control,
}

names!(WaveformPart {
  Root => "root",
  Bars => "bars",
  Bar => "bar",
  Played => "played",
  Control => "control",
});

/// The states of a waveform a recipe styles, besides what the pointer and the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WaveformState {
  /// It takes no input.
  Disabled,
  /// Nothing of it was heard yet: every bar is in the color of the part not played.
  Unheard,
  /// A press, a drag and the keys seek on it.
  Operated,
}

names!(WaveformState {
  Disabled => "disabled",
  Unheard => "unheard",
  Operated => "operated",
});

/// What builds a waveform's nodes: a theme's in place of the default one.
pub type WaveformRenderer = Arc<dyn Fn(WaveformView) -> Element + Send + Sync>;

/// A waveform.
///
/// ```
/// use lurq::{core::Signal, ui::Waveform};
///
/// // A third of it played, shown and not operated.
/// let wave = Waveform::at(0.33, &[0.2, 0.6, 1.0, 0.4, 0.7, 0.3]);
/// // Bound to how far it is played, which a press, a drag and the keys set.
/// let progress = Signal::new(0.33_f32);
/// let bound = Waveform::new(progress, &[0.2, 0.6, 1.0, 0.4]);
/// // The app holds how far it is and is told where a person presses or drags to.
/// let seek = Waveform::controlled(0.33, &[0.2, 0.6, 1.0, 0.4], |to| println!("{to}"));
/// ```
///
/// `peaks` are the heights of its bars, each from 0 to 1, one for each bar. How far it is played runs from 0 to 1.
/// Operated ([`Waveform::new`], [`Waveform::controlled`]) it is a seek surface: a press and a drag tell
/// [`Waveform::on_change`] where they are, [`Waveform::on_change_end`] where they end, and it is a stop of Tab whose
/// arrow keys move it a [`step`](Waveform::step), Page keys a tenth, Home and End to the ends.
#[derive(Clone)]
pub struct Waveform {
  progress: f64,
  bound: Option<Signal<f32>>,
  operated: bool,
  peaks: Vec<f32>,
  unheard: bool,
  step: f64,
  label: Arc<str>,
  disabled: bool,
  on_change: Vec<SeekHandler>,
  on_change_end: Vec<SeekHandler>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Waveform {
  /// A waveform shown and not operated, `progress` of it played.
  pub fn at(progress: impl Into<f64>, peaks: &[f32]) -> Self {
    Self {
      progress: progress.into(),
      bound: None,
      operated: false,
      peaks: peaks.to_vec(),
      unheard: false,
      step: 0.05,
      label: Arc::from("Seek"),
      disabled: false,
      on_change: Vec::new(),
      on_change_end: Vec::new(),
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// A waveform bound to `progress`, from 0 to 1, which a press, a drag and the keys set.
  pub fn new(progress: Signal<f32>, peaks: &[f32]) -> Self {
    let mut wave = Self::at(0.0, peaks);
    wave.bound = Some(progress);
    wave.operated = true;
    wave
  }

  /// A waveform that shows `progress`, held by the app, and tells `on_change` where a person presses or drags
  /// to, from 0 to 1.
  pub fn controlled(progress: impl Into<f64>, peaks: &[f32], on_change: impl Fn(f64) + Send + Sync + 'static) -> Self {
    let mut wave = Self::at(progress, peaks).on_change(on_change);
    wave.operated = true;
    wave
  }

  /// Whether nothing of it was heard yet: every bar is then in the color of the part not played.
  pub fn unheard(mut self, unheard: bool) -> Self {
    self.unheard = unheard;
    self
  }

  /// How far an arrow key moves it, from 0 to 1: a twentieth unless told.
  pub fn step(mut self, step: impl Into<f64>) -> Self {
    self.step = step.into();
    self
  }

  /// What it is called, to what reads the tree: "Seek" unless told.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Arc::from(label);
    self
  }

  /// Whether it takes no input: half its opacity.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Called with where a person moves it to, from 0 to 1. Each call adds a handler.
  pub fn on_change(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Called with where it is when a drag is let go, a press is over or a key's step is made, where it moved. Each
  /// call adds a handler.
  pub fn on_change_end(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.on_change_end.push(Arc::new(handler));
    self
  }

  /// Puts the waveform in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this waveform over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this waveform: what the component it is in says of it in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws it as it is under the pointer or focused from the keyboard, with nothing pointing at it: for a gallery
  /// or a test. Only one that is operated shows it.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the waveform: of what is pressed and focused, where it is operated.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the waveform in `theme`. Converting it into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    // What reads the signal is rendered again when it changes.
    let progress = match &self.bound {
      Some(bound) => f64::from(bound.get()),
      None => self.progress,
    };
    let progress = if progress.is_finite() {
      progress.clamp(0.0, 1.0)
    } else {
      0.0
    };
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(WaveformState::Disabled, self.disabled)
    .state(WaveformState::Unheard, self.unheard)
    .state(WaveformState::Operated, self.operated);
    let view = WaveformView {
      progress,
      peaks: self.peaks,
      unheard: self.unheard,
      operated: self.operated,
      step: self.step,
      label: self.label,
      disabled: self.disabled,
      recipe: theme.resolve::<Waveform>(&case, Some(&self.rules)),
      bound: self.bound,
      on_change: self.on_change,
      on_change_end: self.on_change_end,
      forced: self.forced,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Waveform>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Waveform> for Element {
  fn from(wave: Waveform) -> Self {
    wave.build(&current_theme())
  }
}

/// A waveform as a renderer sees it: what it was given, the style of its parts, and its behavior ready to be
/// dressed.
pub struct WaveformView {
  /// How far it is played, from 0 to 1.
  pub progress: f64,
  /// The heights of its bars, from 0 to 1.
  pub peaks: Vec<f32>,
  pub unheard: bool,
  /// Whether a press, a drag and the keys seek on it.
  pub operated: bool,
  /// How far an arrow key moves it, from 0 to 1.
  pub step: f64,
  pub label: Arc<str>,
  pub disabled: bool,
  /// The style of each part of this waveform.
  pub recipe: ResolvedRecipe,
  bound: Option<Signal<f32>>,
  on_change: Vec<SeekHandler>,
  on_change_end: Vec<SeekHandler>,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl WaveformView {
  /// The style of a part, as this waveform is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced.filter(|_| self.operated))
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  fn px(&self, length: Option<Length>) -> Option<f32> {
    match length?.resolve(self.theme())? {
      Dimension::Px(px) => Some(px),
      _ => None,
    }
  }

  /// How many of its bars are played.
  pub fn played(&self) -> usize {
    if self.unheard {
      0
    } else {
      (self.progress * self.peaks.len() as f64).round() as usize
    }
  }

  /// How wide its bars are, side by side.
  pub fn width(&self) -> f32 {
    let bar = self.px(self.part(WaveformPart::Bar).rest().width).unwrap_or(3.0);
    let gap = self
      .part(WaveformPart::Bars)
      .rest()
      .gap
      .map_or(2.0, |gap| gap.resolve(&self.theme().spacing(), 0.0));
    let count = self.peaks.len() as f32;
    (count * bar + (count - 1.0).max(0.0) * gap).max(0.0)
  }

  /// The row of the bars, each as high as its peak of the most the waveform is high and no lower than the least a
  /// bar is, the played ones first.
  pub fn bars(&self) -> Row {
    let tallest = self.px(self.part(WaveformPart::Root).rest().height).unwrap_or(24.0);
    let played = self.played();
    let (bar, played_bar) = (self.part(WaveformPart::Bar), self.part(WaveformPart::Played));
    self.peaks.iter().enumerate().fold(
      Row::new()
        .part(&self.part(WaveformPart::Bars))
        .align_items(Alignment::Center)
        .describe("a11y_hidden", "true"),
      |row, (index, peak)| {
        let part = if index < played { &played_bar } else { &bar };
        let peak = if peak.is_finite() { peak.clamp(0.0, 1.0) } else { 0.0 };
        row.child(Row::new().part(part).height(peak * tallest))
      },
    )
  }

  /// The engine's slider over the bars, as large as they are: what is pressed, dragged, moved by the keys and
  /// focused, telling the waveform's handlers of what a person does. Nothing of it is drawn but the ring of the
  /// focus. `None` for a waveform that is shown and not operated.
  pub fn control(&self) -> Option<components::Slider> {
    if !self.operated {
      return None;
    }
    let height = self.px(self.part(WaveformPart::Root).rest().height).unwrap_or(24.0);
    let clear = || SliderPartStyle::new().background(Color::new(0, 0, 0, 0));
    let mut slider = seek_control(
      self.progress,
      1.0,
      self.step,
      self.bound.clone(),
      &self.on_change,
      &self.on_change_end,
      self.disabled,
    )
    .part(&self.part(WaveformPart::Control))
    .width(self.width())
    .height(height)
    .track_style(clear().height(height))
    .thumb_style(clear().size(0.0, 0.0))
    .describe("a11y_role", "slider")
    .describe("a11y_name", self.label.clone())
    .describe("a11y_value", format!("{}%", (self.progress * 100.0).round()));
    if let Some(id) = &self.id {
      slider = slider.id(id.clone());
    }
    Some(slider)
  }

  /// How the default renderer builds a waveform: the bars, and over them what is pressed where it is operated.
  pub fn render(self) -> Element {
    let mut root = Stack::new()
      .stack_align(StackAlignment::Center)
      .part(&self.part(WaveformPart::Root))
      // The ring of the focus is around the bars, past their box.
      .overflow_visible()
      .child(self.bars());
    if let Some(control) = self.control() {
      root = root.child(control);
    } else if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

impl Themed for Waveform {
  const NAME: &'static str = "waveform";
  type Renderer = WaveformRenderer;

  /// The design's waveform (`pencil/lurq-design-system.pen`, Media/Waveform): bars 3 wide and 2 apart, fully
  /// round, at most 24 high and no lower than they are wide; the played ones in the primary color, the rest in the
  /// color of an input's edge. Operated, the ring of the focus is around the bars, at a radius sm; disabled, it is
  /// at half its opacity.
  fn recipe() -> Recipe {
    use PaletteColor::*;
    use WaveformPart::{Bar, Bars, Control, Played, Root};
    Recipe::new()
      .base(Root, |s| s.height(24.0))
      .base(Bars, |s| s.gap(2.0))
      .base(Bar, |s| {
        s.width(3.0).min_height(3.0).radius(RadiusSize::Full).background(Input)
      })
      .base(Played, |s| {
        s.width(3.0)
          .min_height(3.0)
          .radius(RadiusSize::Full)
          .background(Primary)
      })
      .base(Control, |s| s.radius(RadiusSize::Sm).cursor(CursorIcon::Pointer))
      .focus_visible(Control, |s| s.focus_ring())
      .state(WaveformState::Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
      .state(WaveformState::Disabled, Control, |s| s.cursor(CursorIcon::Default))
  }
}
