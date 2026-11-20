//! A segmented control: one of a few choices, side by side on a track.

use std::sync::Arc;

use super::{
  button::ButtonSize,
  names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedRecipe},
  shared::{ChangeHandler, Choice, Chosen, fading, glyph, pressable, tab_stop},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{
    ControlSize, Density, IconSize, OpacityLevel, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle,
  },
  components::Row,
  core::{ElementRef, Signal, SignalValue},
  node::{Element, FocusGroup, border::Border as Line},
};

/// The parts of a segmented control a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SegmentedControlPart {
  /// The track the segments are on.
  Root,
  /// One choice: what is pressed.
  Segment,
  /// The text of a segment.
  Label,
  /// The icon of a segment.
  Icon,
  /// The number after the label of a segment that has one: how many things the choice holds.
  Count,
}

names!(SegmentedControlPart {
  Root => "root",
  Segment => "segment",
  Label => "label",
  Icon => "icon",
  Count => "count",
});

/// The states of a segmented control a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SegmentedControlState {
  /// Of a segment: it is the one that is chosen.
  Selected,
  /// It takes no input.
  Disabled,
  /// It is as wide as what it is in, and its segments share that width.
  FullWidth,
  /// What is chosen is wrong: the track has a line in the destructive color.
  Invalid,
}

names!(SegmentedControlState {
  Selected => "selected",
  Disabled => "disabled",
  FullWidth => "full_width",
  Invalid => "invalid",
});

/// The sizes of a segmented control: a button's.
pub type SegmentedControlSize = ButtonSize;

/// What builds a segmented control's nodes: a theme's in place of the default one.
pub type SegmentedControlRenderer = Arc<dyn Fn(SegmentedControl) -> Element + Send + Sync>;

/// One segment, with what a press of it does.
struct Segment {
  label: Arc<str>,
  icon: Option<Name>,
  icon_only: bool,
  disabled: bool,
  selected: bool,
  id: Option<Arc<str>>,
  count: Option<Arc<str>>,
  states: Vec<Name>,
  element: Option<ElementRef>,
  pick: Arc<dyn Fn() + Send + Sync>,
}

/// One of a few choices that are all in view: the period of a chart, the
/// appearance of an app, the way a list is laid out.
///
/// ```
/// use lurq::{core::Signal, ui::SegmentedControl};
///
/// # fn period(chosen: Signal<u8>) {
/// let period = SegmentedControl::new(chosen, [(0, "Day"), (1, "Week"), (2, "Month")]);
/// // The app holds what is chosen, and is told what it is to be.
/// let appearance = SegmentedControl::controlled("light", [("system", "System"), ("light", "Light")], |choice| {
///   println!("{choice}")
/// })
/// .label("Appearance");
/// # }
/// ```
///
/// Exactly one segment is chosen. Each is a button: Tab reaches it, and a
/// click, Enter or Space chooses it. A choice may have an id of its own, after which its label and its count are
/// named, a count after its label, a state of the theme's and a ref to its segment ([`Choice`]). Its sizes are a button's
/// ([`SegmentedControlSize`]). [`on_change`](Self::on_change) is told the
/// value a person chooses, a `&T` the compiler checks; a press of the chosen
/// segment tells nothing. `T` is `()` where nothing names it: the theme's
/// recipe is `SegmentedControl`'s, whatever its values.
pub struct SegmentedControl<T = ()> {
  choices: Vec<Choice<T>>,
  chosen: Chosen<T>,
  on_change: Vec<ChangeHandler<T>>,
  size: Name,
  label: Option<Arc<str>>,
  disabled: bool,
  invalid: bool,
  full_width: bool,
  tab_stop: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> SegmentedControl<T> {
  fn of(chosen: Chosen<T>, choices: impl IntoIterator<Item = impl Into<Choice<T>>>) -> Self {
    Self {
      choices: choices.into_iter().map(Into::into).collect(),
      chosen,
      on_change: Vec::new(),
      size: ButtonSize::default().into(),
      label: None,
      disabled: false,
      invalid: false,
      full_width: false,
      tab_stop: true,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// A control whose chosen segment is the one with the signal's value. A
  /// press of another sets the signal.
  pub fn new(value: Signal<T>, choices: impl IntoIterator<Item = impl Into<Choice<T>>>) -> Self
  where
    T: SignalValue,
  {
    Self::of(Chosen::one(value), choices)
  }

  /// A control whose value the app holds: the segment of `value` is the
  /// chosen one, and a press of another tells `on_change` the value it is to
  /// be. A press of the chosen one tells nothing.
  pub fn controlled(
    value: T,
    choices: impl IntoIterator<Item = impl Into<Choice<T>>>,
    on_change: impl Fn(&T) + Send + Sync + 'static,
  ) -> Self {
    Self::of(Chosen::held_one(value), choices).on_change(on_change)
  }

  /// Called as a person chooses another segment, after the handlers there
  /// are, with its value (`|period: &Period| ..`). Not called where the app
  /// sets the signal, nor for a press of the chosen one.
  pub fn on_change(mut self, handler: impl Fn(&T) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// What the control is called to what reads the tree: what its segments are a choice of.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Whether what is chosen is wrong: the track has a line in the destructive color.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.invalid = invalid;
    self
  }

  /// Puts the control in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Lays rules over the theme's recipe for this control: what the
  /// component it is in says of it in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// A [`SegmentedControlSize`], or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// Whether the control takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Whether the control is as wide as what it is in, with its segments sharing the width.
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self
  }

  /// Whether Tab stops in the control; it does, once, unless told otherwise. `false` is a control the pointer and
  /// the app's own shortcuts reach: Tab passes its segments by, and a press of one chooses it and leaves the focus
  /// where it was. A segment the app focuses is pressed by Enter and Space and its arrow keys move along the
  /// segments as before, and they are the same to what reads the tree.
  pub fn tab_stop(mut self, stops: bool) -> Self {
    self.tab_stop = stops;
    self
  }

  /// Styles one part of this control over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Draws the segment Tab would stop on (the chosen one, else the first that
  /// is not disabled) as it is under the pointer, held down or focused from
  /// the keyboard, with nothing pointing at it: for a gallery or a test. The
  /// other segments are drawn as they are.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
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

  /// Builds the control in `theme`. Converting a control into an element
  /// does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let mut case = Case::new()
      .size(self.size)
      .breakpoint(super::current_breakpoint())
      .state(SegmentedControlState::Disabled, self.disabled)
      .state(SegmentedControlState::FullWidth, self.full_width)
      .state(SegmentedControlState::Invalid, self.invalid);
    for state in &self.states {
      case = case.state(state.clone(), true);
    }
    // A segment is in the state of the control, and the chosen one in its own too, and in the states of its choice.
    let resolve = |selected: bool, states: &[Name]| -> ResolvedRecipe {
      let case = states
        .iter()
        .fold(case.clone(), |case, state| case.state(*state, true))
        .state(SegmentedControlState::Selected, selected);
      theme.resolve::<SegmentedControl>(&case, Some(&self.rules))
    };
    let plain = resolve(false, &[]).owned_by(self.id.as_ref());
    let chosen = resolve(true, &[]).owned_like(&plain);
    // One choice: Tab stops in it once, at the chosen segment, and an arrow key moves the focus and the
    // choice with it.
    let mut root = Row::new()
      .part(&plain.part(SegmentedControlPart::Root))
      .focus_group(FocusGroup::horizontal().activate(true).one_stop(true))
      .describe("a11y_role", "group");
    if let Some(label) = &self.label {
      root = root.describe("a11y_name", label.clone());
    }
    if self.invalid {
      root = root.describe("a11y_invalid", "true");
    }
    // Each segment with what a press of it does, of no type of the app's.
    let handlers: Arc<[ChangeHandler<T>]> = self.on_change.into();
    let segments: Vec<Segment> = self
      .choices
      .into_iter()
      .map(|choice| Segment {
        selected: self.chosen.has(&choice.value),
        pick: self.chosen.picker(choice.value, handlers.clone()),
        label: choice.label,
        icon: choice.icon,
        icon_only: choice.icon_only,
        disabled: choice.disabled,
        id: choice.id,
        count: choice.count,
        states: choice.states,
        element: choice.element,
      })
      .collect();
    // Where Tab enters the group: the interaction forced is drawn there alone, as it can only be.
    let entry = segments
      .iter()
      .position(|segment| segment.selected && !segment.disabled)
      .or_else(|| segments.iter().position(|segment| !segment.disabled));
    for (index, segment) in segments.into_iter().enumerate() {
      let own;
      let recipe = match (segment.states.is_empty(), segment.selected) {
        (true, true) => &chosen,
        (true, false) => &plain,
        (false, selected) => {
          own = resolve(selected, &segment.states).owned_like(&plain);
          &own
        }
      };
      // A segment with an id of its own names its parts after it.
      let recipe = recipe.item(segment.id.as_ref(), SegmentedControlPart::Segment);
      let forced = self.forced.filter(|_| Some(index) == entry);
      let part = |part: SegmentedControlPart| recipe.part(part).forced(forced);
      let pick = segment.pick;
      let pressed = pressable(&part(SegmentedControlPart::Segment), self.disabled || segment.disabled);
      let mut pressed = tab_stop(pressed, self.tab_stop)
        .focus_entry(segment.selected)
        .on_click(move |_| pick());
      if let Some(id) = segment.id {
        pressed = pressed.id(id);
      }
      if let Some(element) = segment.element {
        pressed = pressed.ref_element(element);
      }
      if self.full_width {
        pressed = pressed.flex(1.0);
      }
      if let Some(name) = segment.icon {
        pressed = pressed.child(glyph(&part(SegmentedControlPart::Icon), name, false));
      }
      if segment.icon_only {
        pressed = pressed.describe("a11y_name", segment.label.clone());
      } else {
        let label = part(SegmentedControlPart::Label);
        pressed = pressed.child(label.text(&segment.label).nowrap().part(&label));
      }
      if let Some(count) = &segment.count {
        let style = part(SegmentedControlPart::Count);
        pressed = pressed.child(style.text(count).nowrap().part(&style));
      }
      root = root.child(pressed);
    }
    if let Some(id) = self.id {
      root = root.id(id);
    }
    if let Some(key) = self.key {
      root = root.key(key);
    }
    root.into()
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> From<SegmentedControl<T>> for Element {
  fn from(control: SegmentedControl<T>) -> Self {
    control.build(&current_theme())
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> crate::ui::Styled for SegmentedControl<T> {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl<T: Send + Sync + 'static> Themed for SegmentedControl<T> {
  const NAME: &'static str = "segmented_control";
  type Renderer = SegmentedControlRenderer;

  fn parts() -> crate::ui::Declared {
    SegmentedControlPart::declared()
  }

  fn states() -> crate::ui::Declared {
    SegmentedControlState::declared()
  }

  /// The design's control (`pencil/lurq-design-system.pen`, Segmented
  /// Control): a muted track as high as a control, padded 3, with segments 2
  /// apart; the chosen one is a surface with a small shadow.
  fn recipe() -> Recipe {
    use PaletteColor::*;
    use SegmentedControlPart::{Count, Icon, Label, Root, Segment};
    Recipe::new()
      .base(Root, |s| {
        s.height(ControlSize::Md)
          .padding_x(3.0)
          .padding_y(3.0)
          .gap(2.0)
          .radius(RadiusSize::Md)
          .background(Muted)
      })
      .base(Segment, |s| {
        fading(
          s.height(Length::Fill)
            .padding_x(14.0)
            .gap(6.0)
            .radius(RadiusSize::Sm)
            .foreground(MutedForeground),
        )
      })
      .base(Label, |s| s.typography(TypographyStyle::Button))
      .base(Icon, |s| s.square(IconSize::Md))
      // A count is quieter than the label it follows, on the chosen segment too.
      .base(Count, |s| s.typography(TypographyStyle::Caption).foreground(MutedForeground))
      .hovered(Segment, |s| s.foreground(Foreground))
      .focus_visible(Segment, |s| s.focus_ring())
      .size(ButtonSize::Xs, Root, |s| s.height(Density::Compact))
      .size(ButtonSize::Xs, Segment, |s| s.padding_x(10.0))
      .size(ButtonSize::Xs, Label, |s| s.font_size(12.0))
      .size(ButtonSize::Xs, Icon, |s| s.square(IconSize::Sm))
      .size(ButtonSize::Sm, Root, |s| s.height(ControlSize::Sm))
      .size(ButtonSize::Sm, Segment, |s| s.padding_x(12.0))
      .size(ButtonSize::Lg, Root, |s| s.height(ControlSize::Lg))
      .size(ButtonSize::Lg, Segment, |s| s.padding_x(16.0))
      .size(ButtonSize::Xl, Root, |s| s.height(Density::Comfortable))
      .size(ButtonSize::Xl, Segment, |s| s.padding_x(18.0))
      .size(ButtonSize::Xl, Label, |s| s.font_size(15.0))
      .size(ButtonSize::Xl, Icon, |s| s.square(IconSize::Lg))
      .state(SegmentedControlState::Selected, Segment, |s| {
        s.background(Background).foreground(Foreground).shadow(ShadowStyle::Sm)
      })
      .state(SegmentedControlState::Disabled, Root, |s| {
        s.opacity(OpacityLevel::Disabled)
      })
      .state(SegmentedControlState::FullWidth, Root, |s| s.width(Length::Fill))
      .state(SegmentedControlState::Invalid, Root, |s| {
        s.border(Line::inside(crate::app::theme::BorderSize::Sm, Destructive))
      })
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> super::FieldControl for SegmentedControl<T> {
  /// Called by the field's label unless it has a name of its own, invalid where the field is, and disabled
  /// where the field is. Each segment is called by its own label.
  fn in_field(mut self, field: &super::InField) -> Element {
    if self.label.is_none() {
      self.label = Some(field.label.clone());
    }
    self.invalid |= field.judgment == Some(super::Judgment::Invalid);
    self.disabled |= field.disabled;
    self.into()
  }
}
