//! An icon: a glyph of the theme's set, by name.

use std::{sync::Arc, time::Duration};

use super::{part_style::Length, recipe::Name, theme::current_theme};
use crate::{
  animation::{Animation, AnimationIterationCount, Easing, KeyframesId, StepPosition},
  app::theme::{IconSize, Theme},
  components::{Rect, Text},
  layout::text_style::{TextAlign, TextStyle},
  node::{Element, TextColor, dimension::Dimension},
};

/// The keyframes of a full turn, which every window has: what a spinning
/// icon plays. Every run of them is in phase, so spinners that appear at
/// different moments turn together. Identifiers from here up are lurq's own;
/// an app numbers its keyframes below.
pub const SPIN: KeyframesId = KeyframesId::new(0xFF00);

/// How long a spinning icon takes to turn once.
pub const SPIN_PERIOD: Duration = Duration::from_millis(1000);

/// How a spinning icon and a [`Spinner`](super::Spinner) move: smoothly, or in steps, and whether they stand still
/// where motion is reduced.
///
/// ```
/// use lurq::ui::{Icon, SpinMotion, Spinner, SpinnerKind, icons};
///
/// // 24 poses a second, none drawn between, and still where the system asks for reduced motion.
/// let motion = SpinMotion::stepped(24).still_when_reduced(true);
/// let loader = Icon::new(icons::SPINNER).spin(true).spin_motion(motion);
/// let typing = Spinner::new().kind(SpinnerKind::Dots).motion(motion);
/// ```
///
/// Smooth, a window draws every frame while one moves; stepped, it draws a frame when the pose changes and none
/// between, `per_second` frames a second whatever the number of spinners, for they all move on one clock. A theme
/// gives the motion of every spinner that is not told its own ([`Theme::set_spin_motion`]), the ring of a loading
/// button too. Spinners move under reduced motion unless told to stand still: one that stood still would say
/// nothing, where nothing else says that something is under way.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SpinMotion {
  /// The poses a second, or `None` for a smooth motion.
  steps_per_second: Option<u32>,
  still_when_reduced: bool,
}

impl SpinMotion {
  /// A smooth motion, also under reduced motion: what a spinner does unless told otherwise.
  pub const SMOOTH: Self = Self {
    steps_per_second: None,
    still_when_reduced: false,
  };

  /// A motion in `per_second` poses a second, each held until the next: a ring of 24 turns 15 degrees at each
  /// step of its turn of a second. A period that is no whole number of steps is taken in the nearest number of
  /// them, and the dots and the bars of a spinner start on a step, so that all of them change at the same moment.
  /// Fewer than one a second is one.
  pub const fn stepped(per_second: u32) -> Self {
    Self {
      steps_per_second: Some(if per_second == 0 { 1 } else { per_second }),
      still_when_reduced: false,
    }
  }

  /// Whether it stands still where motion is reduced ([`Theme::reduced_motion`]): the ring at rest, the dots and
  /// the bars as the design draws them still.
  pub const fn still_when_reduced(mut self, still: bool) -> Self {
    self.still_when_reduced = still;
    self
  }

  /// The poses a second; `None` where it is smooth.
  pub const fn steps_per_second(&self) -> Option<u32> {
    self.steps_per_second
  }

  /// Whether it stands still where motion is reduced.
  pub const fn is_still_when_reduced(&self) -> bool {
    self.still_when_reduced
  }

  /// Whether it stands still in `theme`.
  pub(crate) fn is_still(&self, theme: &Theme) -> bool {
    self.still_when_reduced && theme.reduced_motion()
  }

  /// How many steps a span of `span` takes: `None` where it is smooth.
  fn steps_in(&self, span: Duration) -> Option<u32> {
    let per_second = self.steps_per_second?;
    let steps = (span.as_secs_f64() * f64::from(per_second)).round();
    // `as` saturates: a span of years in steps of a second is the most there are.
    Some((steps as u32).max(1))
  }

  /// The easing of a run of `period` whose keyframes are `segments` spans of equal length: in its steps, each pose
  /// held from its start, or `smooth`. An easing eases each span between two keyframes, so the steps are counted
  /// in one span.
  pub(crate) fn easing(&self, period: Duration, segments: u32, smooth: Easing) -> Easing {
    match self.steps_in(period / segments.max(1)) {
      Some(count) => Easing::Steps {
        count,
        position: StepPosition::JumpEnd,
      },
      None => smooth,
    }
  }

  /// `delay`, of a run of `period` in `segments` spans that starts a little after another, put on the nearest of
  /// the run's steps, so that both change at the same moments.
  pub(crate) fn delay(&self, period: Duration, segments: u32, delay: Duration) -> Duration {
    let span = period / segments.max(1);
    let Some(count) = self.steps_in(span) else {
      return delay;
    };
    let step = span / count;
    if step.is_zero() {
      return delay;
    }
    let steps = (delay.as_secs_f64() / step.as_secs_f64()).round();
    step * steps as u32
  }
}

/// An icon.
///
/// ```
/// use lurq::{
///   app::theme::{IconSize, PaletteColor},
///   components::Row,
///   ui::{Icon, icons},
/// };
///
/// let row = Row::new()
///   .foreground(PaletteColor::MutedForeground)
///   .child(Icon::new(icons::SEARCH))
///   .child(Icon::new(icons::SETTINGS).size(IconSize::Lg))
///   .child(Icon::new("check").color(PaletteColor::Success));
/// ```
///
/// It is drawn with the glyph the theme has for its name
/// ([`ThemeIcons`](crate::app::theme::ThemeIcons)): one of lurq's own
/// ([`icons`](super::icons)) unless the theme gives the name another, and any
/// name the theme adds. A name the theme has no glyph for takes its room and
/// draws nothing.
///
/// An icon names no color: it is drawn in the foreground of what it is in, as
/// text is, so a button's icon is the color of its label. It is a square the
/// size of a step of the theme's icon sizes, 16 by default.
pub struct Icon {
  name: Name,
  size: Length,
  color: Option<TextColor>,
  spin: bool,
  motion: Option<SpinMotion>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Icon {
  /// The icon the theme has under `name`: a constant of [`icons`](super::icons), or a name the theme added.
  pub fn new(name: impl Into<Name>) -> Self {
    Self {
      name: name.into(),
      size: Length::Icon(IconSize::Md),
      color: None,
      spin: false,
      motion: None,
      id: None,
      key: None,
    }
  }

  /// The side of the icon: a step of the theme's icon sizes, an [`IconSize`] (`Md` unless told), or a number of
  /// logical pixels. It takes a [`Length`], not a size the theme's recipe names as other components' `size` does:
  /// an icon has no recipe, and the components it is in give it the length their own recipes resolve to.
  pub fn size(mut self, size: impl Into<Length>) -> Self {
    self.size = size.into();
    self
  }

  /// A color for the icon in place of the foreground of what it is in.
  pub fn color(mut self, color: impl Into<TextColor>) -> Self {
    self.color = Some(color.into());
    self
  }

  /// Whether the icon turns, once in [`SPIN_PERIOD`]: a spinner. It turns as its [`spin_motion`](Self::spin_motion)
  /// says, or the theme's ([`Theme::spin_motion`]).
  pub fn spin(mut self, spin: bool) -> Self {
    self.spin = spin;
    self
  }

  /// How the icon turns where it spins, in place of the theme's motion: in steps, still where motion is reduced.
  pub fn spin_motion(mut self, motion: SpinMotion) -> Self {
    self.motion = Some(motion);
    self
  }

  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The side of the icon in `theme`, in logical pixels.
  fn side(&self, theme: &Theme) -> f32 {
    match self.size.resolve(theme) {
      Some(Dimension::Px(side)) => side,
      // A size that is no number of pixels, or a step the theme does not have: the default one.
      _ => theme.icon_size_value(IconSize::Md),
    }
  }

  /// Builds the icon in `theme`. Converting an icon into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let side = self.side(theme);
    let Some(glyph) = theme.icon(self.name) else {
      let mut blank = Rect::new(side, side);
      if let Some(id) = self.id {
        blank = blank.id(id);
      }
      if let Some(key) = self.key {
        blank = blank.key(key);
      }
      return blank.into();
    };
    let mut buffer = [0; 4];
    let mut icon = Text::styled(
      glyph.glyph.encode_utf8(&mut buffer),
      TextStyle {
        font_family: glyph.family,
        font_size: side,
        line_height: 1.0,
        text_align: TextAlign::Center,
        ..TextStyle::default()
      },
    )
    .nowrap()
    .width(side)
    .height(side);
    if let Some(color) = self.color {
      icon = icon.color(color);
    }
    let motion = self.motion.unwrap_or_else(|| theme.spin_motion());
    if self.spin && !motion.is_still(theme) {
      icon = icon.animation(
        Animation::new(SPIN)
          .duration(SPIN_PERIOD)
          .easing(motion.easing(SPIN_PERIOD, 1, Easing::Linear))
          .iteration_count(AnimationIterationCount::Infinite),
      );
    }
    if let Some(id) = self.id {
      icon = icon.id(id);
    }
    if let Some(key) = self.key {
      icon = icon.key(key);
    }
    icon.into()
  }
}

impl From<Icon> for Element {
  fn from(icon: Icon) -> Self {
    icon.build(&current_theme())
  }
}
