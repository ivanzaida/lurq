//! What the components of the button family are built from.

use std::sync::Arc;

use super::{
  icon::Icon,
  part_style::{Length, PartStyle, PartTransition},
  recipe::{Name, ResolvedPart},
};
use crate::{
  animation::{KeyframesId, Transition},
  app::events::{KeyboardEvent, MouseButton, MouseEvent},
  app::theme::{IconSize, MotionDuration},
  components::{self, Row, TextOverflow},
  core::{ElementRef, Signal, SignalValue},
  layout::{Alignment, layout_kind::Justify},
  node::{Element, EventHandler, dimension::Dimension, padding::Padding},
};

/// Which way a component runs: along a row, or down a column. One type for every component that has a way, of
/// which each component's own name (`TabsOrientation`, `SliderOrientation`) is an alias.
///
/// ```
/// use lurq::ui::Orientation;
///
/// assert_eq!(Orientation::default(), Orientation::Horizontal);
/// assert_eq!(Orientation::Vertical.as_str(), "vertical");
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Orientation {
  /// Along a row.
  #[default]
  Horizontal,
  /// Down a column.
  Vertical,
}

super::names!(Orientation {
  Horizontal => "horizontal",
  Vertical => "vertical",
});

impl Orientation {
  /// Whether it runs down a column.
  pub fn is_vertical(self) -> bool {
    self == Self::Vertical
  }
}

/// The icon a component with an icon of its own is given: one of the theme's set by its name, or none (`None`),
/// in place of the one it would show. What `icon(..)` of an alert, a banner, a toast or a message box takes.
///
/// ```
/// use lurq::ui::{IconChoice, icons};
///
/// let named: IconChoice = icons::CHECK.into();
/// let none: IconChoice = None.into();
/// assert!(named.0.is_some() && none.0.is_none());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IconChoice(pub Option<Name>);

impl From<&str> for IconChoice {
  fn from(name: &str) -> Self {
    Self(Some(Name::from(name)))
  }
}

impl From<Name> for IconChoice {
  fn from(name: Name) -> Self {
    Self(Some(name))
  }
}

impl From<Option<Name>> for IconChoice {
  fn from(name: Option<Name>) -> Self {
    Self(name)
  }
}

/// The icon a component shows: the one it was given, none where it was given none, and its own (`default`) where
/// it was given nothing.
pub(crate) fn shown_icon(given: Option<IconChoice>, default: &'static str) -> Option<Name> {
  match given {
    Some(IconChoice(icon)) => icon,
    None => Some(Name::from(default)),
  }
}

/// The keyframes of a fade in, from nothing to whole, which every window
/// has: what a panel over the window plays as it opens.
pub const FADE_IN: KeyframesId = KeyframesId::new(0xFF01);

/// The keyframes of a fade out, from whole to nothing, which every window
/// has: what a panel over the window plays as it closes.
pub const FADE_OUT: KeyframesId = KeyframesId::new(0xFF02);

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

/// `button` as `tab_stop(stops)` of a pressable says: where it is `false`, Tab passes the button by and a press of
/// it with the pointer leaves the focus where it was. It still takes the focus the app gives it, which Enter and
/// Space press it from, and the arrows of a focus group it is in still move to it.
pub(crate) fn tab_stop(button: components::Button, stops: bool) -> components::Button {
  if stops {
    button
  } else {
    button.tab_stop(false).focus_on_press(false)
  }
}

/// Whether a press asks for what it is on beside what is open, where a press of the primary button can: with Ctrl
/// held, or Cmd where that is the platform's key (Ctrl and a press is the secondary press of a Mac).
pub(crate) fn command_held(event: &MouseEvent) -> bool {
  if cfg!(any(target_os = "macos", target_os = "ios")) {
    event.meta
  } else {
    event.ctrl
  }
}

/// Whether a key was pressed with the platform's key that asks for something aside held: Ctrl, or Cmd where that is
/// the platform's key, as [`command_held`] says of a press.
pub(crate) fn command_held_key(event: &KeyboardEvent) -> bool {
  if cfg!(any(target_os = "macos", target_os = "ios")) {
    event.meta
  } else {
    event.ctrl
  }
}

/// `button` calling `handlers` (its `on_open_aside`) where a press asks for it beside what is open: a click of the
/// middle button, or of the primary one with the platform's key held ([`command_held`]). Such a press is theirs
/// alone: the button's own handlers are not called, it takes no focus, submits no form, and does not reach what
/// is around the button. Given before the button's own `on_click`, which it runs ahead of. No handlers, no change.
pub(crate) fn opens_aside(button: components::Button, handlers: &[EventHandler<MouseEvent>]) -> components::Button {
  if handlers.is_empty() {
    return button;
  }
  let call = |handlers: Vec<EventHandler<MouseEvent>>| {
    move |event: &MouseEvent| {
      for handler in &handlers {
        handler.call(event);
      }
    }
  };
  let (primary, middle) = (call(handlers.to_vec()), call(handlers.to_vec()));
  button
    // A middle press is the start of this click, not of a scroll that follows the pointer.
    .on_mouse_down(|event: MouseEvent| {
      if event.button == MouseButton::Middle {
        event.prevent_default();
      }
    })
    .on_mouse_click(MouseButton::Middle, move |event: MouseEvent| {
      event.stop_propagation();
      middle(&event);
    })
    .on_click(move |event: MouseEvent| {
      if command_held(&event) {
        event.prevent_default();
        event.stop_immediate_propagation();
        primary(&event);
      }
    })
}

/// `text` on one line in the style of `part`, which gives way first where
/// what holds it is narrower than it and is cut with an ellipsis: the label
/// of a button, which keeps its padding and its icon in a room too small.
pub(crate) fn one_line(part: &ResolvedPart, text: &str) -> Element {
  Row::new()
    .flex_shrink(1.0)
    .min_width(0.0)
    .child(part.text(text).nowrap().text_overflow(TextOverflow::Elipsis).part(part))
    .into()
}

/// The label of a checkbox, a radio or a switch in the style of its `part`: on one line, or, where it wraps
/// (`Some`), on as many as the width the control is given asks, taking the room the mark leaves it, as far down as
/// it is told so that its first line stands in the middle of a mark taller than it.
pub(crate) fn control_label(part: &ResolvedPart, label: &str, wraps: Option<f32>) -> Row {
  let Some(top) = wraps else {
    return Row::new().part(part).child(part.text(label).nowrap());
  };
  let fill = Dimension::Pct(100.0);
  let label = Row::new().part(part).width(fill).child(part.text(label).width(fill));
  Row::new()
    .flex_shrink(1.0)
    .min_width(0.0)
    .padding(Padding::new().top(top))
    .child(label)
}

/// The height of the first line of a label in the style of `label`.
fn first_line(label: &ResolvedPart) -> f32 {
  let style = label.rest().text.resolve(label.theme());
  style.font_size * if style.trim_line_box { 1.0 } else { style.line_height }
}

/// How far down a label that wraps (`label`, its part) starts beside a mark in the style of `mark`: half of what
/// the mark is taller than its first line, so that the line stands in the mark's middle; nothing beside a mark no
/// taller than the line, which [`beside_first_line`] centers on it.
pub(crate) fn label_top(label: &ResolvedPart, mark: &ResolvedPart) -> f32 {
  let height = match mark.rest().height.and_then(|height| height.resolve(mark.theme())) {
    Some(Dimension::Px(height)) => height,
    _ => return 0.0,
  };
  ((height - first_line(label)) / 2.0).max(0.0)
}

/// What holds the mark of a checkbox, a radio or a switch beside the first line of a label that wraps (`label`, the
/// label's part): a row as high as that line at the least, the mark centered in it.
pub(crate) fn beside_first_line(label: &ResolvedPart, mark: impl Into<Element>) -> Row {
  Row::new()
    .flex_shrink(0.0)
    .min_height(first_line(label))
    .align_items(Alignment::Center)
    .child(mark)
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
/// // A segment or a radio of its own id, with a count after its label and a state of the theme's.
/// let all = Choice::new("all", "All").id("filter-all").count(24).state("warned", false);
/// ```
///
/// What a group draws of a choice: a [`SegmentedControl`](super::SegmentedControl) and a
/// [`RadioGroup`](super::RadioGroup) all of it; a toggle group and a chip group its label, its icon, `disabled`,
/// `id` and `state`, and not its count or its element ref.
#[derive(Clone)]
pub struct Choice<T> {
  pub(crate) value: T,
  pub(crate) label: Arc<str>,
  pub(crate) icon: Option<Name>,
  pub(crate) icon_only: bool,
  pub(crate) disabled: bool,
  pub(crate) id: Option<Arc<str>>,
  pub(crate) count: Option<Arc<str>>,
  pub(crate) states: Vec<Name>,
  pub(crate) element: Option<ElementRef>,
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
      id: None,
      count: None,
      states: Vec::new(),
      element: None,
    }
  }

  /// The id of what is pressed to make the choice (a segment, a radio's mark), after which its parts are named:
  /// `<id>-label`, `<id>-count`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// A number after the label: how many things the choice holds. "24", "99+".
  pub fn count(mut self, count: impl std::fmt::Display) -> Self {
    self.count = Some(Arc::from(count.to_string()));
    self
  }

  /// Puts the choice alone in a state the theme's recipe names, or takes it out: a choice that warns.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Refers to what is pressed to make the choice, once it is laid out: what a popup it opens hangs from.
  pub fn element_ref(mut self, element: ElementRef) -> Self {
    self.element = Some(element);
    self
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

/// A handler of a group's value: told `&T` for a group of one, `&[T]` for a group of any number.
pub(crate) type ChangeHandler<V> = Arc<dyn Fn(&V) + Send + Sync>;

/// What a group of choices holds and what a press of one does with it, for handlers told `&V`: `V` is `T` for a
/// group of one ([`one`](Self::one), [`held_one`](Self::held_one)) and `[T]` for a group of any number
/// ([`many`](Self::many), [`held_many`](Self::held_many)), so that a handler of another type does not compile.
pub(crate) struct Chosen<T, V: ?Sized = T> {
  /// The values chosen now.
  chosen: Vec<T>,
  /// What a press of a value does, given the group's handlers: sets the signal where it is bound, and tells them
  /// the new value where it changed.
  press: Arc<dyn Fn(&T, &[ChangeHandler<V>]) + Send + Sync>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> Chosen<T, T> {
  /// Exactly one is chosen: the signal's value. Choosing another sets it, then tells the handlers.
  pub(crate) fn one(signal: Signal<T>) -> Self
  where
    T: SignalValue,
  {
    Self {
      chosen: vec![signal.get()],
      press: Arc::new(move |value, handlers| {
        if signal.get_untracked() != *value {
          signal.set(value.clone());
          for handler in handlers {
            handler(value);
          }
        }
      }),
    }
  }

  /// The app holds the one chosen, `value`: a press of another tells the handlers the value it is to be, and a
  /// press of the chosen one tells nothing.
  pub(crate) fn held_one(value: T) -> Self {
    let held = value.clone();
    Self {
      chosen: vec![value],
      press: Arc::new(move |value, handlers| {
        if *value != held {
          for handler in handlers {
            handler(value);
          }
        }
      }),
    }
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> Chosen<T, [T]> {
  /// Any number are chosen: the signal's values. Choosing one adds it, or takes it out, then tells the handlers
  /// the whole new set.
  pub(crate) fn many(signal: Signal<Vec<T>>) -> Self
  where
    Vec<T>: SignalValue,
  {
    Self {
      chosen: signal.get(),
      press: Arc::new(move |value, handlers| {
        let mut now = signal.get_untracked();
        toggled(&mut now, value.clone());
        signal.set(now.clone());
        for handler in handlers {
          handler(now.as_slice());
        }
      }),
    }
  }

  /// The app holds the chosen, `values`: a press tells the handlers the whole set it is to be, with the value
  /// pressed added, or taken out.
  pub(crate) fn held_many(values: Vec<T>) -> Self {
    let held = values.clone();
    Self {
      chosen: values,
      press: Arc::new(move |value, handlers| {
        let mut now = held.clone();
        toggled(&mut now, value.clone());
        for handler in handlers {
          handler(now.as_slice());
        }
      }),
    }
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> Chosen<T, V> {
  pub(crate) fn has(&self, value: &T) -> bool {
    self.chosen.contains(value)
  }

  /// What a press of the choice `value` does, told to `handlers`: of no type of the app's, for the group's
  /// controls.
  pub(crate) fn picker(&self, value: T, handlers: Arc<[ChangeHandler<V>]>) -> Arc<dyn Fn() + Send + Sync> {
    let press = self.press.clone();
    Arc::new(move || press(&value, &handlers))
  }
}

/// `chosen` with `value` added, or taken out where it is there.
fn toggled<T: PartialEq>(chosen: &mut Vec<T>, value: T) {
  match chosen.iter().position(|held| *held == value) {
    Some(index) => {
      chosen.remove(index);
    }
    None => chosen.push(value),
  }
}

/// A choice is an option of a list that is chosen from, a [`Listbox`](super::Listbox)'s: its label, its icon before
/// it, and whether it cannot be chosen.
impl<T> From<Choice<T>> for components::SelectOption<T> {
  fn from(choice: Choice<T>) -> Self {
    let mut option = components::SelectOption::new(choice.value, choice.label).disabled(choice.disabled);
    if let Some(name) = choice.icon {
      option = option.leading_with(move |_| Icon::new(name.clone()).build(&super::current_theme()));
    }
    option
  }
}
