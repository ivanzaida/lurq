//! What the components of the button family are built from.

use std::sync::Arc;

use super::{
  icon::Icon,
  part_style::{Length, PartStyle, PartTransition},
  recipe::{Name, ResolvedPart},
};
use crate::{
  animation::Transition,
  app::theme::{IconSize, MotionDuration},
  components::{self, Row},
  core::{Signal, SignalValue},
  layout::layout_kind::Justify,
  node::Element,
};

/// A style whose fill, ink, shadow and outline change over the theme's fast
/// duration: what a part that answers the pointer starts from.
pub(crate) fn fading(style: PartStyle) -> PartStyle {
  [
    Transition::background_color(),
    Transition::foreground(),
    Transition::box_shadow(),
    Transition::outline(),
  ]
  .into_iter()
  .fold(style, |style, what| {
    style.transition(PartTransition::new(what, MotionDuration::Fast))
  })
}

/// What is pressed, in the style of `part`, with nothing in it yet: a click,
/// Enter and Space press it, Tab reaches it, and a disabled one takes nothing.
pub(crate) fn pressable(part: &ResolvedPart, disabled: bool) -> components::Button {
  components::Button::empty()
    .justify(Justify::Center)
    // A plain button is a stop of Tab only in a form or with an index of its own. A control of the design
    // system is one wherever it is, in the order of the tree.
    .tab_index(0)
    .disabled(disabled)
    .part(part)
}

/// An icon of the theme's set in the style of `part`, at the side the recipe gives that part.
pub(crate) fn glyph(part: &ResolvedPart, name: Name, spin: bool) -> Element {
  let side = part.rest().width.unwrap_or(Length::Icon(IconSize::Md));
  Row::new()
    .part(part)
    .child(Icon::new(name).size(side).spin(spin).build(part.theme()))
    .into()
}

/// One of the things a group lets a person choose between: a segment, a toggle.
///
/// ```
/// use lurq::ui::{Choice, icons};
///
/// let plain: Choice<u8> = (1, "Day").into();
/// let with_icon = Choice::new("bold", "Bold").icon(icons::BOLD).icon_only(true);
/// ```
#[derive(Clone)]
pub struct Choice<T> {
  pub(crate) value: T,
  pub(crate) label: Arc<str>,
  pub(crate) icon: Option<Name>,
  pub(crate) icon_only: bool,
  pub(crate) disabled: bool,
}

impl<T> Choice<T> {
  /// A choice of `value`, called `label`.
  pub fn new(value: T, label: impl AsRef<str>) -> Self {
    Self {
      value,
      label: Arc::from(label.as_ref()),
      icon: None,
      icon_only: false,
      disabled: false,
    }
  }

  /// An icon before the label, by its name in the theme's set.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// Whether the choice is its icon alone. Its label stays what it is called.
  pub fn icon_only(mut self, icon_only: bool) -> Self {
    self.icon_only = icon_only;
    self
  }

  /// Whether the choice cannot be made.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }
}

impl<T, L: AsRef<str>> From<(T, L)> for Choice<T> {
  fn from((value, label): (T, L)) -> Self {
    Self::new(value, label)
  }
}

/// What a group of choices reads and writes: which of them are chosen, and
/// what choosing one does.
#[derive(Clone)]
pub(crate) struct Chosen<T> {
  has: Arc<dyn Fn(&T) -> bool + Send + Sync>,
  pick: Arc<dyn Fn(T) + Send + Sync>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> Chosen<T> {
  /// Exactly one is chosen: the signal's value. Choosing another sets it.
  pub(crate) fn one(signal: Signal<T>) -> Self
  where
    T: SignalValue,
  {
    let read = signal.clone();
    Self {
      has: Arc::new(move |value| read.get() == *value),
      pick: Arc::new(move |value| {
        if signal.get() != value {
          signal.set(value);
        }
      }),
    }
  }

  /// Any number are chosen: the signal's values. Choosing one adds it, or takes it out.
  pub(crate) fn many(signal: Signal<Vec<T>>) -> Self
  where
    Vec<T>: SignalValue,
  {
    let read = signal.clone();
    Self {
      has: Arc::new(move |value| read.get().contains(value)),
      pick: Arc::new(move |value| {
        signal.update(|chosen| match chosen.iter().position(|held| *held == value) {
          Some(index) => {
            chosen.remove(index);
          }
          None => chosen.push(value),
        })
      }),
    }
  }

  /// The app holds what is chosen: `chosen` are, and `pick` is told of each press.
  pub(crate) fn held(chosen: Vec<T>, pick: impl Fn(T) + Send + Sync + 'static) -> Self {
    Self {
      has: Arc::new(move |value| chosen.contains(value)),
      pick: Arc::new(pick),
    }
  }

  pub(crate) fn has(&self, value: &T) -> bool {
    (self.has)(value)
  }

  /// What a press of the choice `value` does.
  pub(crate) fn picker(&self, value: T) -> impl Fn() + Send + Sync + 'static {
    let pick = self.pick.clone();
    move || pick(value.clone())
  }
}
