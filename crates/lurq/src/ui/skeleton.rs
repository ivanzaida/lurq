//! A skeleton: a placeholder in the shape of what is loading.

use std::{sync::Arc, time::Duration};

use super::{
  names,
  node_edits::NodeEdits,
  part_style::Length,
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  theme::{Themed, current_theme},
};
use crate::{
  animation::{AnimatableProperty, AnimatableValue, Animation, Easing, Keyframes, KeyframesId, StepPosition},
  app::theme::{PaletteColor, RadiusSize, Theme},
  components::{Column, Row, Stack},
  layout::StackAlignment,
  node::{BackgroundColor, Element, Gradient, GradientStop, RadiusValue, dimension::Dimension, node::NodeUpdate},
};

/// The keyframes of a skeleton that pulses, which every window has: whole,
/// then half, then whole again. A pulse that dims to another opacity
/// ([`SkeletonMotion::dim_to`]) has keyframes of its own, which every window
/// has too.
pub const SKELETON_PULSE: KeyframesId = KeyframesId::new(0xFF33);

/// The keyframes of the sheen of a skeleton that shimmers, which every window
/// has: the width of what is before the sheen in a row three times the
/// skeleton's width, from nothing to two thirds of it. The sheen, a third of
/// the row and as wide as the skeleton, goes from wholly before the
/// skeleton's start to wholly past its end.
pub const SKELETON_SHIMMER: KeyframesId = KeyframesId::new(0xFF34);

/// How long a skeleton takes to pulse once.
pub const SKELETON_PULSE_PERIOD: Duration = Duration::from_millis(2000);

/// How long the sheen of a skeleton takes to cross it once, whatever its width.
pub const SKELETON_SHIMMER_PERIOD: Duration = Duration::from_millis(2000);

/// How wide the row the sheen crosses in is, in shares of the skeleton: from one width before it to one past it.
const SWEEP: f32 = 300.0;

/// How long the lines of a paragraph are before its last: all of the width, and the one before the last a
/// little less; the last is shorter still. The design's 300, 300, 276 and 180.
const NEXT_TO_LAST: f32 = 92.0;
const LAST: f32 = 60.0;

/// How far a skeleton's pulse dims unless told, in hundredths of its opacity: to half.
const DIM_DEFAULT: u8 = 50;

/// The first of the keyframes of the pulses that dim to another opacity than half, one for each hundredth from 0 to
/// 100: lurq's own, as every identifier from `0xFF00` up is.
const PULSE_DEPTHS: u16 = 0xFF9B;

/// The keyframes of the pulse that dims to `dim` hundredths of the opacity: [`SKELETON_PULSE`] for half.
fn pulse_id(dim: u8) -> KeyframesId {
  if dim == DIM_DEFAULT {
    SKELETON_PULSE
  } else {
    KeyframesId::new(PULSE_DEPTHS + u16::from(dim.min(100)))
  }
}

/// A pulse that dims to `dim` hundredths of the opacity at its middle, as `id`.
fn pulse(id: KeyframesId, dim: u8) -> Keyframes {
  Keyframes::new(id)
    .frame(0.0, |frame| {
      frame.set(AnimatableProperty::Opacity, 1.0);
    })
    .frame(0.5, |frame| {
      frame.set(AnimatableProperty::Opacity, f32::from(dim) / 100.0);
    })
    .frame(1.0, |frame| {
      frame.set(AnimatableProperty::Opacity, 1.0);
    })
}

/// The keyframes of a skeleton: its pulses, one for each depth it may dim to, and its sheen. They play in phase, so
/// every skeleton on screen that dims alike pulses as one, and the sheen crosses every one at once.
pub(crate) fn keyframes() -> Vec<Keyframes> {
  let pulses = (0..=100u8)
    .filter(|dim| *dim != DIM_DEFAULT)
    .map(|dim| pulse(pulse_id(dim), dim));
  let shimmer = Keyframes::new(SKELETON_SHIMMER)
    .frame(0.0, |frame| {
      frame.set(AnimatableProperty::Width, AnimatableValue::Percent(0.0));
    })
    .frame(1.0, |frame| {
      frame.set(AnimatableProperty::Width, AnimatableValue::Percent(200.0 / 3.0));
    });
  std::iter::once(pulse(SKELETON_PULSE, DIM_DEFAULT))
    .chain(pulses)
    .chain(std::iter::once(shimmer))
    .collect()
}

/// How a skeleton pulses or shimmers: smoothly, or in steps, and how long one pulse or one crossing of the sheen
/// takes. Every skeleton moves in one phase whatever its motion, for they all move on one clock: a stepped one holds
/// each pose until the next and asks for no frame between, `per_second` frames a second however many skeletons are
/// shown. A theme gives the motion of every skeleton not told its own ([`Theme::set_skeleton_motion`]). Where motion
/// is reduced a skeleton stands still, whatever its motion.
///
/// ```
/// use std::time::Duration;
/// use lurq::{app::theme::Theme, ui::{Skeleton, SkeletonMotion}};
///
/// // A pulse of 1.6 s in 20 poses a second, for every skeleton of the app.
/// let theme = Theme::new();
/// theme.set_skeleton_motion(SkeletonMotion::stepped(20).period(Duration::from_millis(1600)));
/// // One skeleton's own.
/// let line = Skeleton::line().motion(SkeletonMotion::SMOOTH);
/// // A pulse that dims to 0.55 and not to half.
/// theme.set_skeleton_motion(SkeletonMotion::stepped(8).dim_to(0.55));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SkeletonMotion {
  /// The poses a second, or `None` for a smooth motion.
  steps_per_second: Option<u32>,
  period: Duration,
  /// The opacity the pulse dims to at its middle, in hundredths.
  dim: u8,
}

impl Default for SkeletonMotion {
  fn default() -> Self {
    Self::SMOOTH
  }
}

impl SkeletonMotion {
  /// A smooth motion, a pulse in [`SKELETON_PULSE_PERIOD`]: what a skeleton does unless told otherwise.
  pub const SMOOTH: Self = Self {
    steps_per_second: None,
    period: SKELETON_PULSE_PERIOD,
    dim: DIM_DEFAULT,
  };

  /// A motion in `per_second` poses a second, each held until the next. A half of a pulse that is no whole number
  /// of steps is taken in the nearest number of them. Fewer than one a second is one.
  pub const fn stepped(per_second: u32) -> Self {
    Self {
      steps_per_second: Some(if per_second == 0 { 1 } else { per_second }),
      period: SKELETON_PULSE_PERIOD,
      dim: DIM_DEFAULT,
    }
  }

  /// How long one pulse, whole to half and back, or one crossing of the sheen takes: [`SKELETON_PULSE_PERIOD`]
  /// unless told. None at all is a millisecond.
  pub const fn period(mut self, period: Duration) -> Self {
    self.period = if period.is_zero() {
      Duration::from_millis(1)
    } else {
      period
    };
    self
  }

  /// The opacity a pulse dims to at its middle, from its whole at its start and its end: 0.5, half, unless told.
  /// It is taken in hundredths, between 0 and 1. A sheen that crosses a skeleton does not dim it.
  pub fn dim_to(mut self, opacity: f32) -> Self {
    // `as` saturates, and NaN is 0; the hundredths are 0 to 100 after the clamp.
    self.dim = (opacity.clamp(0.0, 1.0) * 100.0).round() as u8;
    self
  }

  /// The opacity a pulse dims to at its middle.
  pub fn dimmed(&self) -> f32 {
    f32::from(self.dim) / 100.0
  }

  /// The poses a second; `None` where it is smooth.
  pub const fn steps_per_second(&self) -> Option<u32> {
    self.steps_per_second
  }

  /// How long one pulse or one crossing of the sheen takes.
  pub const fn period_length(&self) -> Duration {
    self.period
  }

  /// The easing of a run whose keyframes are `segments` spans of equal length: in its steps, each pose held from
  /// its start, or `smooth`. An easing eases each span between two keyframes, so the steps are counted in one span.
  fn easing(&self, segments: u32, smooth: Easing) -> Easing {
    let Some(per_second) = self.steps_per_second else {
      return smooth;
    };
    let span = self.period / segments.max(1);
    // `as` saturates: a span of years in steps of a second is the most there are.
    let count = ((span.as_secs_f64() * f64::from(per_second)).round() as u32).max(1);
    Easing::Steps {
      count,
      position: StepPosition::JumpEnd,
    }
  }
}

/// The shape of a skeleton.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SkeletonShape {
  /// A line of text: as wide as the room it has, 12 high.
  #[default]
  Line,
  /// A face, an icon: round.
  Circle,
  /// An image, a chart, a card: more rounded than a line.
  Block,
  /// Lines of text one under another, the last shorter.
  Paragraph,
}

names!(SkeletonShape {
  Line => "line",
  Circle => "circle",
  Block => "block",
  Paragraph => "paragraph",
});

/// How a skeleton moves while it waits. Where motion is reduced it stands still, whatever it is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SkeletonAnimation {
  /// It fades to half and back.
  #[default]
  Pulse,
  /// A lighter sheen crosses it: the design's shimmer.
  Shimmer,
  /// It stands still.
  None,
}

/// The parts of a skeleton a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SkeletonPart {
  /// What holds the shapes: the column of a paragraph's lines, and the box of a single shape.
  Root,
  /// A shape: a line, the circle, the block. Its fill is where the sheen starts and ends.
  Bone,
  /// What crosses a shape that shimmers: its width, and its fill, which is the middle of the sheen.
  Sheen,
}

names!(SkeletonPart {
  Root => "root",
  Bone => "bone",
  Sheen => "sheen",
});

/// What builds a skeleton's nodes: a theme's in place of the default one.
pub type SkeletonRenderer = Arc<dyn Fn(SkeletonView) -> Element + Send + Sync>;

/// A skeleton: a placeholder in the shape of what is loading.
///
/// ```
/// use lurq::{components::{Column, Row}, ui::{Skeleton, SkeletonAnimation}};
///
/// // A row of a list: a face, and two lines beside it.
/// let row = Row::new()
///   .spacing(12.0)
///   .child(Skeleton::circle(40.0))
///   .child(
///     Column::new()
///       .flex(1.0)
///       .spacing(8.0)
///       .child(Skeleton::line())
///       .child(Skeleton::line().width(140.0).height(10.0)),
///   );
/// let text = Skeleton::paragraph(4).animation(SkeletonAnimation::Shimmer);
/// // What tooling is told: one status around them all.
/// let loading = Skeleton::status("Loading messages", row);
/// ```
///
/// A line and a paragraph are as wide as the room they have, and a circle
/// and a block are the size they are given; any of them takes a width and a
/// height of the app's. A skeleton is nothing to tooling that reads the
/// tree: what is loading is said once, by a status an app puts around a
/// group of them ([`Skeleton::status`]).
#[derive(Clone)]
pub struct Skeleton {
  shape: SkeletonShape,
  lines: usize,
  animation: Option<SkeletonAnimation>,
  motion: Option<SkeletonMotion>,
  width: Option<Length>,
  height: Option<Length>,
  /// The app's rules of layout over the shape's (its least and most sizes, its radius), under what it styles.
  layout: Recipe,
  root_edits: NodeEdits,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Skeleton {
  fn shaped(shape: SkeletonShape) -> Self {
    Self {
      shape,
      lines: 1,
      animation: None,
      motion: None,
      width: None,
      height: None,
      layout: Recipe::new(),
      root_edits: NodeEdits::default(),
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// A line of text, as wide as the room it has and 12 high.
  pub fn line() -> Self {
    Self::shaped(SkeletonShape::Line)
  }

  /// A circle of `size` across: a face, an icon.
  pub fn circle(size: impl Into<Length>) -> Self {
    let size = size.into();
    let mut skeleton = Self::shaped(SkeletonShape::Circle);
    skeleton.width = Some(size);
    skeleton.height = Some(size);
    skeleton
  }

  /// A block of `width` by `height`: an image, a chart, a card. `Length::Fill` is all the room it has.
  pub fn block(width: impl Into<Length>, height: impl Into<Length>) -> Self {
    let mut skeleton = Self::shaped(SkeletonShape::Block);
    skeleton.width = Some(width.into());
    skeleton.height = Some(height.into());
    skeleton
  }

  /// `lines` lines of text one under another, as wide as the room they have; the one before the last is a
  /// little shorter, and the last shorter still.
  pub fn paragraph(lines: usize) -> Self {
    let mut skeleton = Self::shaped(SkeletonShape::Paragraph);
    skeleton.lines = lines.max(1);
    skeleton
  }

  /// A status around `content`, called `label`, for tooling that reads the tree: what the skeletons in it
  /// stand for, said once.
  pub fn status(label: &str, content: impl Into<Element>) -> Column {
    Column::new()
      .describe("a11y_role", "status")
      .describe("a11y_name", label)
      .child(content)
  }

  /// How the skeleton moves while it waits, in place of the theme's ([`Theme::skeleton_animation`]), a pulse
  /// unless the theme says otherwise.
  pub fn animation(mut self, animation: SkeletonAnimation) -> Self {
    self.animation = Some(animation);
    self
  }

  /// How it pulses or shimmers, in place of the theme's ([`Theme::skeleton_motion`]): smoothly, or in steps.
  pub fn motion(mut self, motion: SkeletonMotion) -> Self {
    self.motion = Some(motion);
    self
  }

  /// How wide the skeleton is: a number of logical pixels, or `Length::Fill`. For a paragraph, the width of
  /// all of it.
  pub fn width(mut self, width: impl Into<Length>) -> Self {
    self.width = Some(width.into());
    self
  }

  /// How high the skeleton is. For a paragraph, the height of each line.
  pub fn height(mut self, height: impl Into<Length>) -> Self {
    self.height = Some(height.into());
    self
  }

  /// The least it is wide: `0.0` lets a line that shares a row with others give its room up. For a paragraph,
  /// of all of it.
  pub fn min_width(mut self, width: impl Into<Length>) -> Self {
    let width = width.into();
    self.sized(move |s| s.min_width(width));
    self
  }

  /// The most it is wide, however much room it has. For a paragraph, all of it.
  pub fn max_width(mut self, width: impl Into<Length>) -> Self {
    let width = width.into();
    self.sized(move |s| s.max_width(width));
    self
  }

  /// The least it is high. For a paragraph, each line.
  pub fn min_height(mut self, height: impl Into<Length>) -> Self {
    let height = height.into();
    self.layout = self.layout.base(SkeletonPart::Bone, move |s| s.min_height(height));
    self
  }

  /// The most it is high. For a paragraph, each line.
  pub fn max_height(mut self, height: impl Into<Length>) -> Self {
    let height = height.into();
    self.layout = self.layout.base(SkeletonPart::Bone, move |s| s.max_height(height));
    self
  }

  /// How round its corners are, in place of its shape's: a step of the theme's radii, or logical pixels. For a
  /// paragraph, each line's.
  pub fn radius(mut self, radius: impl Into<RadiusValue>) -> Self {
    let radius = radius.into();
    self.layout = self.layout.base(SkeletonPart::Bone, move |s| s.radius(radius));
    self
  }

  /// Its share of the room left in the row or the column it is in, as an element's `flex` is.
  pub fn flex(mut self, factor: f32) -> Self {
    self.root_edits.push(move |node| NodeUpdate::flex(node, factor));
    self
  }

  /// How much of its size it gives up where the row or the column it is in has too little room: `0.0` keeps it
  /// whole.
  pub fn flex_shrink(mut self, factor: f32) -> Self {
    self.root_edits.push(move |node| NodeUpdate::flex_shrink(node, factor));
    self
  }

  /// Lays a rule of size on the root, and on the bone too where the skeleton is one shape: a paragraph's lines
  /// keep theirs.
  fn sized(&mut self, rule: impl Fn(super::PartStyle) -> super::PartStyle) {
    let layout = std::mem::take(&mut self.layout).base(SkeletonPart::Root, &rule);
    self.layout = if self.shape == SkeletonShape::Paragraph {
      layout
    } else {
      layout.base(SkeletonPart::Bone, rule)
    };
  }

  /// Puts the skeleton in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this skeleton over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this skeleton.
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

  /// Builds the skeleton in `theme`. Converting a skeleton into an element
  /// does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: Some(self.shape.into()),
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    };
    // The app's sizes are over the shape's and under what the app styles.
    let mut sizes = Recipe::new();
    let paragraph = self.shape == SkeletonShape::Paragraph;
    if let Some(width) = self.width {
      sizes = sizes.base(SkeletonPart::Root, |s| s.width(width));
      if !paragraph {
        sizes = sizes.base(SkeletonPart::Bone, |s| s.width(width));
      }
    }
    if let Some(height) = self.height {
      sizes = sizes.base(SkeletonPart::Bone, |s| s.height(height));
    }
    let rules = sizes.extend(&self.layout).extend(&self.rules);
    let view = SkeletonView {
      shape: self.shape,
      lines: self.lines,
      motion: self.motion.unwrap_or_else(|| theme.skeleton_motion()),
      // Where motion is reduced, a skeleton stands still.
      animation: if theme.reduced_motion() {
        SkeletonAnimation::None
      } else {
        self.animation.unwrap_or_else(|| theme.skeleton_animation())
      },
      recipe: theme
        .resolve::<Skeleton>(&case, Some(&rules))
        .owned_by(self.id.as_ref()),
      root_edits: self.root_edits,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Skeleton>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Skeleton> for Element {
  fn from(skeleton: Skeleton) -> Self {
    skeleton.build(&current_theme())
  }
}

/// A skeleton as a renderer sees it.
pub struct SkeletonView {
  pub shape: SkeletonShape,
  /// How many lines it has: one, but for a paragraph.
  pub lines: usize,
  /// How it moves: [`SkeletonAnimation::None`] where motion is reduced.
  pub animation: SkeletonAnimation,
  /// How it pulses or shimmers: its own, else the theme's.
  pub motion: SkeletonMotion,
  /// The style of each part of this skeleton.
  pub recipe: ResolvedRecipe,
  root_edits: NodeEdits,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl SkeletonView {
  /// The style of a part, as this skeleton is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// What crosses a shape that shimmers: a band whose fill runs from the shape's to the sheen's and back, as
  /// the design draws it, a third of the row it crosses in (see [`Self::sweep`]): as wide as the shape.
  pub fn sheen(&self) -> Row {
    let (bone, part) = (self.part(SkeletonPart::Bone), self.part(SkeletonPart::Sheen));
    let (edge, middle) = (bone.rest().background, part.rest().background);
    let mut sheen = Row::new()
      .width(Dimension::Pct(100.0 * 100.0 / SWEEP))
      .height(Dimension::Pct(100.0))
      .part(&part);
    if let (Some(edge), Some(middle)) = (edge, middle) {
      let stop = |color: &BackgroundColor, at: f32| GradientStop::at(color.clone(), at);
      sheen = sheen.background_gradient(Gradient::linear(
        90.0,
        [stop(&edge, 0.0), stop(&middle, 0.5), stop(&edge, 1.0)],
      ));
    }
    sheen
  }

  /// What a shape that shimmers holds: a row three times its width, centred on it, so from one width before
  /// its start to one past its end; in the row, before the sheen, a room whose width runs from nothing to two
  /// thirds of the row ([`SKELETON_SHIMMER`]). The sheen goes from wholly before the shape to wholly past it in
  /// the same time whatever the shape's width, and the shape clips it.
  pub fn sweep(&self) -> Stack {
    let full = Dimension::Pct(100.0);
    let room = Row::new().width(Dimension::Pct(0.0)).height(full).animation(
      Animation::new(SKELETON_SHIMMER)
        .duration(self.motion.period)
        .easing(self.motion.easing(1, Easing::Linear))
        .infinite(),
    );
    // A stack lays a child wider than itself with its middle on its own: the row starts a width before it.
    Stack::new().stack_align(StackAlignment::Center).size(full, full).child(
      Row::new()
        .width(Dimension::Pct(SWEEP))
        .height(full)
        .child(room)
        .child(self.sheen()),
    )
  }

  /// One shape, in the style of its part, moving as the skeleton does.
  pub fn bone(&self) -> Row {
    let bone = Row::new().part(&self.part(SkeletonPart::Bone));
    match self.animation {
      SkeletonAnimation::Pulse => bone.animation(
        Animation::new(pulse_id(self.motion.dim))
          .duration(self.motion.period)
          .easing(self.motion.easing(2, Easing::EASE_IN_OUT))
          .infinite(),
      ),
      // The bone clips the sheen to its corners.
      SkeletonAnimation::Shimmer => bone.child(self.sweep()),
      SkeletonAnimation::None => bone,
    }
  }

  /// How the default renderer builds a skeleton: its root with a bone in it, or a paragraph's lines one under
  /// another, the next to last and the last shorter. Nothing of it is described, so tooling that reads the
  /// tree leaves it out, as it leaves out any box with no text, no id and no handler.
  pub fn render(self) -> Element {
    let mut root = self.root();
    let lines = if self.shape == SkeletonShape::Paragraph {
      self.lines
    } else {
      1
    };
    for index in 0..lines {
      let mut bone = self.bone();
      if lines > 1 && index + 1 == lines {
        bone = bone.width(Dimension::Pct(LAST));
      } else if lines > 2 && index + 2 == lines {
        bone = bone.width(Dimension::Pct(NEXT_TO_LAST));
      }
      root = root.child(bone);
    }
    root.into()
  }

  /// The root in the style of its part, with its id, its key and where the app laid it out (its `flex`), with
  /// nothing in it yet.
  pub fn root(&self) -> Column {
    let mut root = Column::new().part(&self.part(SkeletonPart::Root));
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.update_node(|node| self.root_edits.apply(node));
    root
  }
}

impl crate::ui::Styled for Skeleton {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for Skeleton {
  const NAME: &'static str = "skeleton";
  type Renderer = SkeletonRenderer;

  fn parts() -> crate::ui::Declared {
    SkeletonPart::declared()
  }

  /// The design's skeleton (`pencil/lurq-design-system.pen`, Skeleton/Line,
  /// Circle, Block, Shimmer, Paragraph): shapes in the muted fill. A line is
  /// 12 high and `radius-sm` round, a circle 40 across and round all the
  /// way, a block 200 by 112 and `radius-lg` round; a paragraph's lines are
  /// 10 apart. The shimmer's sheen is the secondary fill under the pointer
  /// between two of the muted fill.
  fn recipe() -> Recipe {
    use SkeletonPart::{Bone, Root, Sheen};
    use SkeletonShape::{Block, Circle, Line, Paragraph};
    Recipe::new()
      .base(Bone, |s| s.background(PaletteColor::Muted).radius(RadiusSize::Sm))
      .base(Sheen, |s| s.background(PaletteColor::SecondaryHover))
      .variant(Line, Root, |s| s.width(Length::Fill))
      .variant(Line, Bone, |s| s.width(Length::Fill).height(12.0))
      .variant(Paragraph, Root, |s| s.width(Length::Fill).gap(10.0))
      .variant(Paragraph, Bone, |s| s.width(Length::Fill).height(12.0))
      .variant(Circle, Bone, |s| s.square(40.0).radius(RadiusSize::Full))
      .variant(Block, Bone, |s| s.width(200.0).height(112.0).radius(RadiusSize::Lg))
  }
}
