//! A skeleton: a placeholder in the shape of what is loading.

use std::{sync::Arc, time::Duration};

use super::{
  names,
  part_style::Length,
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  theme::{Themed, current_theme},
};
use crate::{
  animation::{AnimatableProperty, Animation, Easing, Keyframes, KeyframesId},
  app::theme::{PaletteColor, RadiusSize, Theme},
  components::{Column, Row},
  node::{BackgroundColor, Element, Gradient, GradientStop, dimension::Dimension, transform::Decomposed},
};

/// The keyframes of a skeleton that pulses, which every window has: whole,
/// then half, then whole again.
pub const SKELETON_PULSE: KeyframesId = KeyframesId::new(0xFF33);

/// The keyframes of the sheen of a skeleton that shimmers, which every window
/// has: from before the skeleton's start to [`SKELETON_SHIMMER_TRAVEL`] past it.
pub const SKELETON_SHIMMER: KeyframesId = KeyframesId::new(0xFF34);

/// How long a skeleton takes to pulse once.
pub const SKELETON_PULSE_PERIOD: Duration = Duration::from_millis(2000);

/// How long the sheen of a skeleton takes to cross it once.
pub const SKELETON_SHIMMER_PERIOD: Duration = Duration::from_millis(2000);

/// How far past the start of a skeleton the sheen goes, in logical pixels: a skeleton wider than this sees
/// it cross only so far. The sheen moves by pixels, not by a share of the skeleton's width, so that every
/// skeleton on screen, whatever its width, shimmers at one speed and in phase.
pub const SKELETON_SHIMMER_TRAVEL: f32 = 1200.0;

/// How wide the sheen is where the recipe does not say: the design's shimmer, a line of 200.
const SHEEN: f32 = 200.0;

/// How long the lines of a paragraph are before its last: all of the width, and the one before the last a
/// little less; the last is shorter still. The design's 300, 300, 276 and 180.
const NEXT_TO_LAST: f32 = 92.0;
const LAST: f32 = 60.0;

/// The keyframes of a skeleton: its pulse and its sheen. They play in phase, so every skeleton on screen
/// pulses as one, and the sheen crosses every one at once.
pub(crate) fn keyframes() -> [Keyframes; 2] {
  let pulse = Keyframes::new(SKELETON_PULSE)
    .frame(0.0, |frame| {
      frame.set(AnimatableProperty::Opacity, 1.0);
    })
    .frame(0.5, |frame| {
      frame.set(AnimatableProperty::Opacity, 0.5);
    })
    .frame(1.0, |frame| {
      frame.set(AnimatableProperty::Opacity, 1.0);
    });
  let shimmer = Keyframes::new(SKELETON_SHIMMER)
    .frame(0.0, |frame| {
      frame.set(
        AnimatableProperty::Transform,
        Decomposed::IDENTITY.with_translate(-SHEEN, 0.0),
      );
    })
    .frame(1.0, |frame| {
      frame.set(
        AnimatableProperty::Transform,
        Decomposed::IDENTITY.with_translate(SKELETON_SHIMMER_TRAVEL, 0.0),
      );
    });
  [pulse, shimmer]
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
  animation: SkeletonAnimation,
  width: Option<Length>,
  height: Option<Length>,
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
      animation: SkeletonAnimation::default(),
      width: None,
      height: None,
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

  /// How the skeleton moves while it waits.
  pub fn animation(mut self, animation: SkeletonAnimation) -> Self {
    self.animation = animation;
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
    let rules = sizes.extend(&self.rules);
    let view = SkeletonView {
      shape: self.shape,
      lines: self.lines,
      // Where motion is reduced, a skeleton stands still.
      animation: if theme.reduced_motion() {
        SkeletonAnimation::None
      } else {
        self.animation
      },
      recipe: theme.resolve::<Skeleton>(&case, Some(&rules)),
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
  /// The style of each part of this skeleton.
  pub recipe: ResolvedRecipe,
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

  /// What crosses a shape that shimmers: a band of the sheen's width whose fill runs from the shape's to the
  /// sheen's and back, as the design draws it, moved from before the shape's start to past its end.
  pub fn sheen(&self) -> Row {
    let (bone, part) = (self.part(SkeletonPart::Bone), self.part(SkeletonPart::Sheen));
    let (edge, middle) = (bone.rest().background, part.rest().background);
    let mut sheen = Row::new()
      .width(SHEEN)
      .height(Dimension::Pct(100.0))
      .part(&part)
      .animation(
        Animation::new(SKELETON_SHIMMER)
          .duration(SKELETON_SHIMMER_PERIOD)
          .easing(Easing::Linear)
          .infinite(),
      );
    if let (Some(edge), Some(middle)) = (edge, middle) {
      let stop = |color: &BackgroundColor, at: f32| GradientStop::at(color.clone(), at);
      sheen = sheen.background_gradient(Gradient::linear(
        90.0,
        [stop(&edge, 0.0), stop(&middle, 0.5), stop(&edge, 1.0)],
      ));
    }
    sheen
  }

  /// One shape, in the style of its part, moving as the skeleton does.
  pub fn bone(&self) -> Row {
    let bone = Row::new().part(&self.part(SkeletonPart::Bone));
    match self.animation {
      SkeletonAnimation::Pulse => bone.animation(
        Animation::new(SKELETON_PULSE)
          .duration(SKELETON_PULSE_PERIOD)
          .easing(Easing::EASE_IN_OUT)
          .infinite(),
      ),
      // The bone clips the sheen to its corners.
      SkeletonAnimation::Shimmer => bone.child(self.sheen()),
      SkeletonAnimation::None => bone,
    }
  }

  /// How the default renderer builds a skeleton: its root with a bone in it, or a paragraph's lines one under
  /// another, the next to last and the last shorter. Nothing of it is described, so tooling that reads the
  /// tree leaves it out, as it leaves out any box with no text, no id and no handler.
  pub fn render(self) -> Element {
    let mut root = Column::new().part(&self.part(SkeletonPart::Root));
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
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

impl Themed for Skeleton {
  const NAME: &'static str = "skeleton";
  type Renderer = SkeletonRenderer;

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
