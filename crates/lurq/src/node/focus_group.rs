//! A group the arrow keys move the focus within: the choices of a radio
//! group, the segments of a control, the items of a menu.

/// The arrow keys that move the focus within a [`FocusGroup`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusAxis {
  /// Left and Right.
  Horizontal,
  /// Up and Down.
  Vertical,
  /// All four: Left and Up go back, Right and Down go on.
  Both,
}

/// What makes an element a group of focusable things the arrow keys move
/// between, while the focus is on one of them: `.focus_group(..)` on the
/// element that holds them. Home and End go to the first and the last. Tab
/// is not changed by it, unless the group is [one stop](Self::one_stop).
///
/// ```
/// use lurq::{
///   components::{Button, Row},
///   node::FocusGroup,
/// };
///
/// // A toolbar: Tab reaches it once, the arrows move within it.
/// let toolbar = Row::new()
///   .focus_group(FocusGroup::horizontal().one_stop(true))
///   .child(Button::new("Cut").tab_index(0))
///   .child(Button::new("Copy").tab_index(0))
///   .child(Button::new("Paste").tab_index(0));
/// ```
///
/// What the keys move between is what Tab would stop at inside the element
/// (a `tab_index` of `0` or more, or a control of a form), in the order of
/// the tree; a disabled one is passed by. A text input, a slider and a
/// select keep the arrow keys for themselves while they are focused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FocusGroup {
  pub(crate) axis: FocusAxis,
  pub(crate) wrap: bool,
  pub(crate) activate: bool,
  pub(crate) one_stop: bool,
}

impl FocusGroup {
  fn new(axis: FocusAxis) -> Self {
    Self {
      axis,
      wrap: true,
      activate: false,
      one_stop: false,
    }
  }

  /// Left and Right move the focus.
  pub fn horizontal() -> Self {
    Self::new(FocusAxis::Horizontal)
  }

  /// Up and Down move the focus.
  pub fn vertical() -> Self {
    Self::new(FocusAxis::Vertical)
  }

  /// All four arrows move the focus: a radio group, whose choices may be laid out either way.
  pub fn both() -> Self {
    Self::new(FocusAxis::Both)
  }

  /// Whether the focus goes from the last to the first and back. It does unless told otherwise.
  pub fn wrap(mut self, wrap: bool) -> Self {
    self.wrap = wrap;
    self
  }

  /// Whether what the focus is moved to is pressed as well, as Space would
  /// press it: the choice follows the focus, as in a radio group.
  pub fn activate(mut self, activate: bool) -> Self {
    self.activate = activate;
    self
  }

  /// Whether Tab stops in the group once: on the element marked with
  /// `.focus_entry(true)` (the chosen one), or the first, and from inside
  /// the group Tab leaves it.
  pub fn one_stop(mut self, one_stop: bool) -> Self {
    self.one_stop = one_stop;
    self
  }
}

/// What an arrow key, Home or End asks of a focus group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FocusStep {
  Back,
  On,
  First,
  Last,
}

impl FocusGroup {
  /// What `key` asks of this group, if anything.
  pub(crate) fn step(&self, key: &str) -> Option<FocusStep> {
    let (horizontal, vertical) = match self.axis {
      FocusAxis::Horizontal => (true, false),
      FocusAxis::Vertical => (false, true),
      FocusAxis::Both => (true, true),
    };
    match key {
      "ArrowLeft" if horizontal => Some(FocusStep::Back),
      "ArrowRight" if horizontal => Some(FocusStep::On),
      "ArrowUp" if vertical => Some(FocusStep::Back),
      "ArrowDown" if vertical => Some(FocusStep::On),
      "Home" => Some(FocusStep::First),
      "End" => Some(FocusStep::Last),
      _ => None,
    }
  }
}
