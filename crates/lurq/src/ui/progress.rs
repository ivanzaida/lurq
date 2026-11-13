//! A progress bar: how far a piece of work is, along a rail; and a ring progress, the same value around a circle.

use std::{sync::Arc, time::Duration};

use super::{
  names,
  part_style::{Length, PartTransition},
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  theme::{Themed, current_theme},
};
use crate::{
  animation::{AnimatableProperty, AnimatableValue, Animation, Easing, Keyframes, KeyframesId, Transition},
  app::theme::{MotionDuration, PaletteColor, RadiusSize, Theme, TypographyStyle},
  components::{Row, Stack},
  layout::{Alignment, StackAlignment, layout_kind::Justify, text_style::FontWeight},
  node::{BackgroundColor, Element, Gradient, GradientStop, color::Color, dimension::Dimension},
};

/// The keyframes of the room before the moving segment of a progress that does not know how far it is: it grows
/// from nothing to the whole rail, pushing the segment across and out at the end.
pub const PROGRESS_LEAD: KeyframesId = KeyframesId::new(0xFF21);

/// The keyframes of the moving segment of a progress that does not know how far it is: it grows out of the start
/// of the rail to its width, and keeps it as it is pushed across.
pub const PROGRESS_SEGMENT: KeyframesId = KeyframesId::new(0xFF22);

/// How long the segment of a progress that does not know how far it is takes to cross the rail once.
pub const PROGRESS_PERIOD: Duration = Duration::from_millis(1600);

/// The share of the rail the segment of a progress that does not know how far it is fills, in percent: 90 of 240
/// (`pencil/lurq-design-system.pen`, Loader/Linear).
const SEGMENT: f32 = 37.5;

/// Where the segment stands while motion is reduced: 70 of 240 from the start, as the board draws it.
const STILL_LEAD: f32 = 70.0 / 240.0 * 100.0;

/// How far through a crossing the segment has grown to its width.
const GROWN: f32 = SEGMENT / 100.0;

/// The keyframes of a progress that does not know how far it is: [`PROGRESS_LEAD`] and [`PROGRESS_SEGMENT`],
/// which every window has. Both play in phase, so the room and the segment of one bar move together, and every
/// such bar of a window moves as the others do.
pub(crate) fn keyframes() -> [Keyframes; 2] {
  let share = |frame: &mut crate::animation::KeyframeBuilder, percent: f32| {
    frame.set(AnimatableProperty::Width, AnimatableValue::Percent(percent));
  };
  [
    Keyframes::new(PROGRESS_LEAD)
      .frame(0.0, |frame| share(frame, 0.0))
      .frame(1.0, |frame| share(frame, 100.0)),
    Keyframes::new(PROGRESS_SEGMENT)
      .frame(0.0, |frame| share(frame, 0.0))
      .frame(GROWN, |frame| share(frame, SEGMENT))
      .frame(1.0, |frame| share(frame, SEGMENT)),
  ]
}

/// The parts of a progress bar a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProgressPart {
  /// The bar with its label and its value above it.
  Root,
  /// The row of the label and the value, over the rail.
  Header,
  /// What the work is: "Uploading theme.json".
  Label,
  /// How far it is, at the end of the header: "62%".
  Value,
  /// The rail: as long as the whole of the work.
  Track,
  /// What is filled of the rail: as long as what is done. Of a progress that does not know how far it is, the
  /// segment that crosses the rail.
  Fill,
}

names!(ProgressPart {
  Root => "root",
  Header => "header",
  Label => "label",
  Value => "value",
  Track => "track",
  Fill => "fill",
});

/// The states of a progress bar a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProgressState {
  /// It does not know how far the work is: a segment crosses the rail again and again.
  Indeterminate,
  /// It is the thin bar across the top of a window or a surface: no rail, square at the start.
  TopBar,
}

names!(ProgressState {
  Indeterminate => "indeterminate",
  TopBar => "top_bar",
});

/// What a progress says of the work: the color of what is filled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ProgressVariant {
  /// Work under way: the primary color.
  #[default]
  Default,
  /// Work that went well, or a share that is good to have: the success color.
  Success,
  /// A share that wants a look: the warning color.
  Warning,
  /// Work that failed, or a share that is too much: the destructive color.
  Destructive,
}

names!(ProgressVariant {
  Default => "default",
  Success => "success",
  Warning => "warning",
  Destructive => "destructive",
});

/// How large a progress is: how thick a bar's rail is, and how wide a ring is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ProgressSize {
  /// A rail of 4, in a row of a list (a file being uploaded); a ring of 40.
  Sm,
  /// A rail of 6, under a label or in a dialog; a ring of 56.
  Md,
  /// A rail of 8, the board's progress; a ring of 72, the board's ring.
  #[default]
  Lg,
}

names!(ProgressSize {
  Sm => "sm",
  Md => "md",
  Lg => "lg",
});

/// What says a value as text: given the value and the greatest value.
pub type ProgressFormat = Arc<dyn Fn(f32, f32) -> String + Send + Sync>;

/// How far the work is: a value between nothing and a greatest, or not known.
#[derive(Clone)]
struct Amount {
  /// `None` while it is not known.
  value: Option<f32>,
  max: f32,
  format: Option<ProgressFormat>,
}

impl Amount {
  fn new(value: Option<f32>) -> Self {
    Self {
      value,
      max: 1.0,
      format: None,
    }
  }

  /// The value kept between nothing and the greatest. A greatest of nothing or less has no room: nothing is done.
  fn value(&self) -> Option<f32> {
    let value = self.value?;
    Some(if self.max > 0.0 && value > 0.0 {
      value.min(self.max)
    } else {
      0.0
    })
  }

  fn fraction(&self) -> Option<f32> {
    let value = self.value()?;
    Some(if self.max > 0.0 { value / self.max } else { 0.0 })
  }

  fn text(&self) -> Option<String> {
    let value = self.value()?;
    Some(match &self.format {
      Some(format) => format(value, self.max),
      None => percent(self.fraction()?),
    })
  }
}

/// A fraction as a whole percentage: "62%".
fn percent(fraction: f32) -> String {
  format!("{}%", (fraction * 100.0).round() as i32)
}

/// What builds a progress bar's nodes: a theme's in place of the default one.
pub type ProgressRenderer = Arc<dyn Fn(ProgressView) -> Element + Send + Sync>;

/// A progress bar: how far a piece of work is. A file uploading, items being
/// copied, the steps of an install.
///
/// ```
/// use lurq::ui::{Progress, ProgressSize, ProgressVariant};
///
/// // A fraction of one.
/// let upload = Progress::new(0.62).label("Uploading theme.json").show_value(true).size(ProgressSize::Md);
/// // 42 of 128 items, said as the app says it.
/// let copy = Progress::new(42.0).max(128.0).show_value(true).format(|done, all| format!("{done} of {all}"));
/// let full = Progress::new(0.92).variant(ProgressVariant::Warning);
/// // Work whose end is not known.
/// let loading = Progress::indeterminate().label("Connecting");
/// ```
///
/// What is filled moves to a new value over the theme's slow duration. A
/// progress that does not know how far it is runs a segment across its rail
/// again and again; while motion is reduced the segment stands still, apart
/// from the start of the rail, where no value would be. A progress is read
/// and not operated: it takes no focus and no press. To tooling that reads
/// the tree it is a `progressbar`, called by its label, with its value, its
/// least and its greatest, and no value while it does not know it.
pub struct Progress {
  amount: Amount,
  variant: Name,
  size: Name,
  label: Option<Arc<str>>,
  show_value: bool,
  top_bar: bool,
  width: Option<Length>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Progress {
  /// A progress at `value`, a fraction of one unless [`Progress::max`] says
  /// otherwise. A value under nothing is nothing, and one over the greatest
  /// is the greatest.
  pub fn new(value: impl Into<f64>) -> Self {
    Self::with(Amount::new(Some(value.into() as f32)))
  }

  /// A progress of work whose end is not known.
  pub fn indeterminate() -> Self {
    Self::with(Amount::new(None))
  }

  fn with(amount: Amount) -> Self {
    Self {
      amount,
      variant: ProgressVariant::default().into(),
      size: ProgressSize::default().into(),
      label: None,
      show_value: false,
      top_bar: false,
      width: None,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// How far the work is, of the greatest value: it then knows.
  pub fn value(mut self, value: impl Into<f64>) -> Self {
    self.amount.value = Some(value.into() as f32);
    self
  }

  /// The greatest value: the whole of the work. 1 when not given.
  pub fn max(mut self, max: impl Into<f64>) -> Self {
    self.amount.max = max.into() as f32;
    self
  }

  /// A [`ProgressVariant`], or a variant the theme's recipe names.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// A [`ProgressSize`], or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// What the work is, over the start of the rail, and what the progress is called.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Whether the value is said over the end of the rail: as a percentage, or
  /// as [`Progress::format`] says it. A progress that does not know how far it
  /// is says none.
  pub fn show_value(mut self, show: bool) -> Self {
    self.show_value = show;
    self
  }

  /// Says the value as the app does, given the value and the greatest: "42 of 128".
  pub fn format(mut self, format: impl Fn(f32, f32) -> String + Send + Sync + 'static) -> Self {
    self.amount.format = Some(Arc::new(format));
    self
  }

  /// Whether it is the thin bar across the top of a window or of a surface:
  /// 3 high, with no rail under what is filled, as wide as what holds it.
  /// Put it first in what it is the top of.
  pub fn top_bar(mut self, top_bar: bool) -> Self {
    self.top_bar = top_bar;
    self
  }

  /// How wide the progress is: 240 when not given, and all the room there is for a top bar.
  pub fn width(mut self, width: impl Into<Length>) -> Self {
    self.width = Some(width.into());
    self
  }

  /// Puts the progress in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this progress over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this progress: what the component
  /// it is in says of the progress in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the root: the progress with its label.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the progress in `theme`. Converting a progress into an element
  /// does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let mut rules = self.rules;
    if let Some(width) = self.width {
      rules = rules.base(ProgressPart::Root, |s| s.width(width));
    }
    let case = Case {
      variant: Some(self.variant),
      size: Some(self.size),
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states,
    }
    .state(ProgressState::Indeterminate, self.amount.value.is_none())
    .state(ProgressState::TopBar, self.top_bar);
    let view = ProgressView {
      label: self.label,
      show_value: self.show_value,
      variant: self.variant,
      size: self.size,
      top_bar: self.top_bar,
      reduced_motion: theme.reduced_motion(),
      recipe: theme.resolve::<Progress>(&case, Some(&rules)),
      amount: self.amount,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Progress>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Progress> for Element {
  fn from(progress: Progress) -> Self {
    progress.build(&current_theme())
  }
}

/// A progress bar as a renderer sees it: what it was given, the style of its
/// parts, and its parts ready to be put together.
pub struct ProgressView {
  pub label: Option<Arc<str>>,
  pub show_value: bool,
  pub variant: Name,
  pub size: Name,
  pub top_bar: bool,
  /// Whether motion is reduced: a progress that does not know how far it is then stands still.
  pub reduced_motion: bool,
  /// The style of each part of this progress.
  pub recipe: ResolvedRecipe,
  amount: Amount,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ProgressView {
  /// The style of a part, as this progress is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The value, kept between nothing and the greatest; `None` while it is not known.
  pub fn value(&self) -> Option<f32> {
    self.amount.value()
  }

  /// The greatest value.
  pub fn max(&self) -> f32 {
    self.amount.max
  }

  /// How much of the work is done, from 0 to 1; `None` while it is not known.
  pub fn fraction(&self) -> Option<f32> {
    self.amount.fraction()
  }

  /// The value as it is said: a percentage, or what the app's format says. `None` while it is not known.
  pub fn value_text(&self) -> Option<String> {
    self.amount.text()
  }

  /// The root in the style of its part, saying what it is to tooling that reads the tree, with nothing in it yet.
  pub fn root(&self) -> crate::components::Column {
    let mut root = crate::components::Column::new().part(&self.part(ProgressPart::Root));
    root = describe(root, self.label.as_deref(), &self.amount);
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// The label and the value over the rail, each where it is given: the label at the start, the value at the end.
  /// `None` where there is neither.
  pub fn header(&self) -> Option<Row> {
    let value = self.show_value.then(|| self.value_text()).flatten();
    if self.label.is_none() && value.is_none() {
      return None;
    }
    let label = self.part(ProgressPart::Label);
    // What holds the label takes the room there is, so that the value stands at the end with no label too.
    let mut start = Row::new().flex(1.0).part(&label);
    if let Some(text) = &self.label {
      start = start.child(label.text(text).nowrap());
    }
    let mut header = Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(ProgressPart::Header))
      .child(start);
    if let Some(text) = value {
      let part = self.part(ProgressPart::Value);
      header = header.child(Row::new().part(&part).child(part.text(&text).nowrap()));
    }
    Some(header)
  }

  /// The rail with what is filled of it: as long as what is done, its width a share of the rail's, which a
  /// transition the recipe gives it moves. Of a progress that does not know how far it is, the room before the
  /// segment and the segment, crossing the rail in [`PROGRESS_PERIOD`] or standing still while motion is reduced.
  pub fn track(&self) -> Row {
    let track = Row::new().part(&self.part(ProgressPart::Track));
    let fill = Row::new().part(&self.part(ProgressPart::Fill));
    match self.fraction() {
      // Keyed, so that a bar that comes to know its value grows from nothing rather than from the segment.
      Some(fraction) => track.child(fill.key("fill").width(Dimension::Pct(fraction * 100.0))),
      None => {
        let mut lead = Row::new().key("lead").flex_shrink(0.0);
        let mut segment = fill.key("segment").flex_shrink(0.0);
        if self.reduced_motion {
          lead = lead.width(Dimension::Pct(STILL_LEAD));
          segment = segment.width(Dimension::Pct(SEGMENT));
        } else {
          let crossing = |keyframes| {
            Animation::new(keyframes)
              .duration(PROGRESS_PERIOD)
              .easing(Easing::Linear)
              .infinite()
          };
          lead = lead.width(Dimension::Pct(0.0)).animation(crossing(PROGRESS_LEAD));
          segment = segment
            .width(Dimension::Pct(SEGMENT))
            .animation(crossing(PROGRESS_SEGMENT));
        }
        track.child(lead).child(segment)
      }
    }
  }

  /// How the default renderer builds a progress: the header, where there is one, over the rail.
  pub fn render(self) -> Element {
    let mut root = self.root();
    if let Some(header) = self.header() {
      root = root.child(header);
    }
    root.child(self.track()).into()
  }
}

/// `node` said to be a progress bar to tooling that reads the tree: called `label`, with its value, its least and
/// its greatest, and no value while it does not know it.
fn describe<N: Describe>(node: N, label: Option<&str>, amount: &Amount) -> N {
  let mut node = node.describe_as("a11y_role", "progressbar");
  if let Some(label) = label {
    node = node.describe_as("a11y_name", label);
  }
  if let Some(value) = amount.value() {
    node = node
      .describe_as("a11y_valuenow", &value.to_string())
      .describe_as("a11y_valuemin", "0")
      .describe_as("a11y_valuemax", &amount.max.to_string());
    if let Some(text) = amount.text() {
      node = node.describe_as("a11y_valuetext", &text);
    }
  }
  node
}

/// What [`describe`] annotates: a column or a stack.
trait Describe: Sized {
  fn describe_as(self, name: &str, value: &str) -> Self;
}

impl Describe for crate::components::Column {
  fn describe_as(self, name: &str, value: &str) -> Self {
    self.describe(name, value)
  }
}

impl Describe for Stack {
  fn describe_as(self, name: &str, value: &str) -> Self {
    self.describe(name, value)
  }
}

impl Themed for Progress {
  const NAME: &'static str = "progress";
  type Renderer = ProgressRenderer;

  /// The design's progress (`pencil/lurq-design-system.pen`, Progress,
  /// Loader/Progress Labeled, Loader/Linear, Loader/Top Bar): a rail of 240
  /// by 8, round, in the secondary color, filled in the primary color, or in
  /// the success, warning or destructive one. The rail is 6 under a label
  /// and in a dialog, 4 in a row of a list. Over it, 8 above, the label in
  /// the label's type at the start and the value at the end in the mono type
  /// of 11 in the muted foreground. The top bar is 3 high with no rail, filled
  /// with corners of 2.
  fn recipe() -> Recipe {
    use PaletteColor::*;
    use ProgressPart::{Fill, Header, Label, Root, Track, Value};
    Recipe::new()
      .base(Root, |s| s.width(240.0).gap(8.0).foreground(Foreground))
      .base(Header, |s| s.width(Length::Fill).gap(8.0))
      .base(Label, |s| s.typography(TypographyStyle::Label))
      .base(Value, |s| {
        s.typography(TypographyStyle::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      .base(Track, |s| {
        s.width(Length::Fill)
          .height(8.0)
          .radius(RadiusSize::Full)
          .background(Secondary)
      })
      .base(Fill, |s| {
        s.height(Length::Fill)
          .radius(RadiusSize::Full)
          .background(Primary)
          .transition(PartTransition::new(Transition::width(), MotionDuration::Slow))
      })
      .variant(ProgressVariant::Success, Fill, |s| s.background(Success))
      .variant(ProgressVariant::Warning, Fill, |s| s.background(Warning))
      .variant(ProgressVariant::Destructive, Fill, |s| s.background(Destructive))
      .size(ProgressSize::Sm, Track, |s| s.height(4.0))
      .size(ProgressSize::Md, Track, |s| s.height(6.0))
      .size(ProgressSize::Lg, Track, |s| s.height(8.0))
      .state(ProgressState::TopBar, Root, |s| s.width(Length::Fill))
      .state(ProgressState::TopBar, Track, |s| {
        s.height(3.0).radius(0.0).background(Color::new(0, 0, 0, 0))
      })
      .state(ProgressState::TopBar, Fill, |s| s.radius(2.0))
  }
}

/// The parts of a ring progress a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RingProgressPart {
  /// The ring with its value in the middle: a square.
  Root,
  /// The disc the ring is cut from: its background is the color of what is not done.
  Track,
  /// What is done, from the top and clockwise: its background is the color of the arc.
  Fill,
  /// The middle of the disc, over it: as large as the ring is not wide, in the color of what the ring is on.
  Middle,
  /// How far it is, in the middle.
  Value,
}

names!(RingProgressPart {
  Root => "root",
  Track => "track",
  Fill => "fill",
  Middle => "middle",
  Value => "value",
});

/// What builds a ring progress's nodes: a theme's in place of the default one.
pub type RingProgressRenderer = Arc<dyn Fn(RingProgressView) -> Element + Send + Sync>;

/// A ring progress: how far a piece of work is, around a circle, with the
/// value in the middle. A storage quota, the share of a goal, an export in a
/// card.
///
/// ```
/// use lurq::{
///   app::theme::PaletteColor,
///   ui::{ProgressVariant, RingProgress, RingProgressPart},
/// };
///
/// let export = RingProgress::new(0.7).label("Export");
/// let quota = RingProgress::new(96.0)
///   .max(128.0)
///   .variant(ProgressVariant::Warning)
///   .format(|used, _| format!("{used} GB"));
/// // On a card: the middle is the card's color.
/// let on_card = RingProgress::new(0.4).style(RingProgressPart::Middle, |s| s.background(PaletteColor::Card));
/// ```
///
/// It takes the [`ProgressVariant`]s and the [`ProgressSize`]s of a
/// [`Progress`]. The ring always knows how far the work is: what does not
/// is a spinner's. To tooling it is a `progressbar`, as a progress is.
///
/// The ring is a disc filled clockwise from the top in the arc's color and
/// the rest of the way in the track's, with its middle painted over it in the
/// color of the page: on a surface of another color, give
/// [`RingProgressPart::Middle`] that color. The arc moves to a new value at
/// once.
pub struct RingProgress {
  amount: Amount,
  variant: Name,
  size: Name,
  label: Option<Arc<str>>,
  show_value: bool,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl RingProgress {
  /// A ring at `value`, a fraction of one unless [`RingProgress::max`] says otherwise.
  pub fn new(value: impl Into<f64>) -> Self {
    Self {
      amount: Amount::new(Some(value.into() as f32)),
      variant: ProgressVariant::default().into(),
      size: ProgressSize::default().into(),
      label: None,
      show_value: true,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// How far the work is, of the greatest value.
  pub fn value(mut self, value: impl Into<f64>) -> Self {
    self.amount.value = Some(value.into() as f32);
    self
  }

  /// The greatest value: the whole of the work. 1 when not given.
  pub fn max(mut self, max: impl Into<f64>) -> Self {
    self.amount.max = max.into() as f32;
    self
  }

  /// A [`ProgressVariant`], or a variant the theme's recipe names.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// A [`ProgressSize`], or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// What the ring is called, to what reads the tree. The board draws no text beside a ring.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Whether the value is said in the middle: as a percentage, or as [`RingProgress::format`] says it. It is
  /// unless told otherwise.
  pub fn show_value(mut self, show: bool) -> Self {
    self.show_value = show;
    self
  }

  /// Says the value as the app does, given the value and the greatest.
  pub fn format(mut self, format: impl Fn(f32, f32) -> String + Send + Sync + 'static) -> Self {
    self.amount.format = Some(Arc::new(format));
    self
  }

  /// Puts the ring in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this ring over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this ring.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the root.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the ring in `theme`. Converting a ring into an element does this
  /// in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: Some(self.variant),
      size: Some(self.size),
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states,
    };
    let view = RingProgressView {
      label: self.label,
      show_value: self.show_value,
      variant: self.variant,
      size: self.size,
      recipe: theme.resolve::<RingProgress>(&case, Some(&self.rules)),
      amount: self.amount,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<RingProgress>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<RingProgress> for Element {
  fn from(ring: RingProgress) -> Self {
    ring.build(&current_theme())
  }
}

/// A ring progress as a renderer sees it.
pub struct RingProgressView {
  pub label: Option<Arc<str>>,
  pub show_value: bool,
  pub variant: Name,
  pub size: Name,
  /// The style of each part of this ring.
  pub recipe: ResolvedRecipe,
  amount: Amount,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl RingProgressView {
  /// The style of a part, as this ring is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The value, kept between nothing and the greatest.
  pub fn value(&self) -> f32 {
    self.amount.value().unwrap_or(0.0)
  }

  /// The greatest value.
  pub fn max(&self) -> f32 {
    self.amount.max
  }

  /// How much of the work is done, from 0 to 1.
  pub fn fraction(&self) -> f32 {
    self.amount.fraction().unwrap_or(0.0)
  }

  /// The value as it is said: a percentage, or what the app's format says.
  pub fn value_text(&self) -> String {
    self.amount.text().unwrap_or_default()
  }

  /// The root in the style of its part, saying what it is to tooling that reads the tree, with nothing in it yet.
  /// What is in it is laid over one another, in the middle.
  pub fn root(&self) -> Stack {
    let mut root = Stack::new()
      .stack_align(StackAlignment::Center)
      .part(&self.part(RingProgressPart::Root));
    root = describe(root, self.label.as_deref(), &self.amount);
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// The disc in the style of the track, filled from the top and clockwise in the fill's background as far as
  /// what is done, and in the track's background the rest of the way.
  pub fn disc(&self) -> Row {
    let disc = Row::new().part(&self.part(RingProgressPart::Track));
    let fraction = self.fraction();
    match self.part(RingProgressPart::Fill).rest().background {
      Some(fill) if fraction >= 1.0 => disc.background(fill),
      Some(fill) if fraction > 0.0 => {
        let track = self
          .part(RingProgressPart::Track)
          .rest()
          .background
          .unwrap_or(BackgroundColor::Color(Color::new(0, 0, 0, 0)));
        // Two stops at one place: the arc ends there, with no blend into the track.
        disc.background_gradient(Gradient::conic(
          0.0,
          [
            GradientStop::at(fill.clone(), 0.0),
            GradientStop::at(fill, fraction),
            GradientStop::at(track.clone(), fraction),
            GradientStop::at(track, 1.0),
          ],
        ))
      }
      _ => disc,
    }
  }

  /// The middle of the disc, in the style of its part, with the value in it where it is shown.
  pub fn middle(&self) -> Row {
    let mut middle = Row::new()
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .part(&self.part(RingProgressPart::Middle));
    if self.show_value {
      let part = self.part(RingProgressPart::Value);
      middle = middle.child(Row::new().part(&part).child(part.text(&self.value_text()).nowrap()));
    }
    middle
  }

  /// How the default renderer builds a ring: the disc, and its middle over it with the value.
  pub fn render(self) -> Element {
    self.root().child(self.disc()).child(self.middle()).into()
  }
}

impl Themed for RingProgress {
  const NAME: &'static str = "ring_progress";
  type Renderer = RingProgressRenderer;

  /// The design's ring progress (`pencil/lurq-design-system.pen`, Ring
  /// Progress): a ring of 72 whose width is a tenth of its side, its middle
  /// four fifths of it across, in the secondary color, with what is done over
  /// it in the primary color from the top and clockwise, and the value in the
  /// middle in the label's type of 13 at a weight of 600. The variants color
  /// the arc as they color a bar; the smaller rings are 56 and 40, their value
  /// 12 and 11.
  fn recipe() -> Recipe {
    use PaletteColor::*;
    use RingProgressPart::{Fill, Middle, Root, Track, Value};
    let middle = |side: f32| move |s: super::PartStyle| s.square(side * 0.8);
    Recipe::new()
      .base(Root, |s| s.square(72.0).foreground(Foreground))
      .base(Track, |s| {
        s.width(Length::Fill)
          .height(Length::Fill)
          .radius(RadiusSize::Full)
          .background(Secondary)
      })
      .base(Fill, |s| s.background(Primary))
      .base(Middle, |s| {
        s.square(57.6).radius(RadiusSize::Full).background(Background)
      })
      .base(Value, |s| {
        s.typography(TypographyStyle::Label)
          .font_size(13.0)
          .weight(FontWeight::SemiBold)
      })
      .variant(ProgressVariant::Success, Fill, |s| s.background(Success))
      .variant(ProgressVariant::Warning, Fill, |s| s.background(Warning))
      .variant(ProgressVariant::Destructive, Fill, |s| s.background(Destructive))
      .size(ProgressSize::Sm, Root, |s| s.square(40.0))
      .size(ProgressSize::Sm, Middle, middle(40.0))
      .size(ProgressSize::Sm, Value, |s| s.font_size(11.0))
      .size(ProgressSize::Md, Root, |s| s.square(56.0))
      .size(ProgressSize::Md, Middle, middle(56.0))
      .size(ProgressSize::Md, Value, |s| s.font_size(12.0))
  }
}
