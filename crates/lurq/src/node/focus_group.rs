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
  /// None: see [`FocusGroup::none`].
  None,
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
  pub(crate) follow_pointer: bool,
  pub(crate) typeahead: bool,
}

impl FocusGroup {
  fn new(axis: FocusAxis) -> Self {
    Self {
      axis,
      wrap: true,
      activate: false,
      one_stop: false,
      follow_pointer: false,
      typeahead: false,
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

  /// No key moves the focus within it, and what is in it is out of the group around it: the arrow keys of
  /// that group pass it by, and do nothing while the focus is in it. For what a group holds beside what its
  /// keys move between: the content of an accordion's open item, between two of its headers.
  ///
  /// ```
  /// use lurq::{
  ///   components::{Button, Column},
  ///   node::FocusGroup,
  /// };
  ///
  /// // Up and Down move between the two headers, past the button of the open one.
  /// let list = Column::new()
  ///   .focus_group(FocusGroup::vertical())
  ///   .child(Button::new("First").tab_index(0))
  ///   .child(
  ///     Column::new()
  ///       .focus_group(FocusGroup::none())
  ///       .child(Button::new("Read more").tab_index(0)),
  ///   )
  ///   .child(Button::new("Second").tab_index(0));
  /// ```
  pub fn none() -> Self {
    Self::new(FocusAxis::None)
  }

  /// Whether it is [`none`](Self::none): what is in it is in no group.
  pub(crate) fn is_none(&self) -> bool {
    self.axis == FocusAxis::None
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

  /// Whether the focus follows the pointer within the group: while the
  /// focus is on one of its elements, the pointer coming onto another moves
  /// the focus there, as it is in a menu, so that what the keyboard and what
  /// the pointer are on are one. The focus shows no ring for it. The pointer
  /// never takes the focus into the group from outside it.
  pub fn follow_pointer(mut self, follow: bool) -> Self {
    self.follow_pointer = follow;
    self
  }

  /// Whether typing a letter moves the focus to the element whose label
  /// starts with it, as in a menu or a list: letters typed within a second of
  /// one another are one prefix, the same letter again goes on to the next
  /// element that starts with it, and a pause starts again. An element's
  /// label is its `focus_label`, or, for a button with one text in it, that
  /// text. A text input in the group keeps its letters, and Space still
  /// presses what has the focus.
  pub fn typeahead(mut self, typeahead: bool) -> Self {
    self.typeahead = typeahead;
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
      FocusAxis::None => return None,
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
