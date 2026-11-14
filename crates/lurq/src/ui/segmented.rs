//! A segmented control: one of a few choices, side by side on a track.

use std::sync::Arc;

use super::{
  button::ButtonSize,
  names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedRecipe},
  shared::{Choice, Chosen, fading, glyph, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{
    ControlSize, Density, IconSize, OpacityLevel, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle,
  },
  components::Row,
  core::{Signal, SignalValue},
  node::{Element, FocusGroup},
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
}

names!(SegmentedControlPart {
  Root => "root",
  Segment => "segment",
  Label => "label",
  Icon => "icon",
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
}

names!(SegmentedControlState {
  Selected => "selected",
  Disabled => "disabled",
  FullWidth => "full_width",
});

/// One segment, with what a press of it does.
struct Segment {
  label: Arc<str>,
  icon: Option<Name>,
  icon_only: bool,
  disabled: bool,
  selected: bool,
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
/// // The app holds what is chosen, and is told of a press.
/// let appearance = SegmentedControl::controlled("light", [("system", "System"), ("light", "Light")], |choice| {
///   println!("{choice}")
/// });
/// # }
/// ```
///
/// Exactly one segment is chosen. Each is a button: Tab reaches it, and a
/// click, Enter or Space chooses it. Its sizes are a button's
/// ([`ButtonSize`]).
pub struct SegmentedControl {
  segments: Vec<Segment>,
  size: Name,
  disabled: bool,
  full_width: bool,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl SegmentedControl {
  fn of<T: Clone + PartialEq + Send + Sync + 'static>(
    chosen: Chosen<T>,
    choices: impl IntoIterator<Item = impl Into<Choice<T>>>,
  ) -> Self {
    let segments = choices
      .into_iter()
      .map(Into::into)
      .map(|choice: Choice<T>| Segment {
        selected: chosen.has(&choice.value),
        pick: Arc::new(chosen.picker(choice.value)),
        label: choice.label,
        icon: choice.icon,
        icon_only: choice.icon_only,
        disabled: choice.disabled,
      })
      .collect();
    Self {
      segments,
      size: ButtonSize::default().into(),
      disabled: false,
      full_width: false,
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// A control whose chosen segment is the one with the signal's value. A
  /// press of another sets the signal.
  pub fn new<T>(value: Signal<T>, choices: impl IntoIterator<Item = impl Into<Choice<T>>>) -> Self
  where
    T: Clone + PartialEq + Send + Sync + SignalValue + 'static,
  {
    Self::of(Chosen::one(value), choices)
  }

  /// A control whose state the app holds: the segment of `value` is the
  /// chosen one, and `choose` is told the value of each one pressed.
  pub fn controlled<T: Clone + PartialEq + Send + Sync + 'static>(
    value: T,
    choices: impl IntoIterator<Item = impl Into<Choice<T>>>,
    choose: impl Fn(T) + Send + Sync + 'static,
  ) -> Self {
    Self::of(Chosen::held(vec![value], choose), choices)
  }

  /// A [`ButtonSize`], or a size the theme's recipe names.
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

  /// Styles one part of this control over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Draws every segment as it is under the pointer, held down or focused
  /// from the keyboard, with nothing pointing at it: for a gallery or a test.
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
    let case = Case::new()
      .size(self.size)
      .breakpoint(super::current_breakpoint())
      .state(SegmentedControlState::Disabled, self.disabled)
      .state(SegmentedControlState::FullWidth, self.full_width);
    // A segment is in the state of the control, and the chosen one in its own too.
    let resolve = |selected: bool| -> ResolvedRecipe {
      theme.resolve::<SegmentedControl>(
        &case.clone().state(SegmentedControlState::Selected, selected),
        Some(&self.rules),
      )
    };
    let (plain, chosen) = (resolve(false), resolve(true));
    // One choice: Tab stops in it once, at the chosen segment, and an arrow key moves the focus and the
    // choice with it.
    let mut root = Row::new()
      .part(&plain.part(SegmentedControlPart::Root))
      .focus_group(FocusGroup::horizontal().activate(true).one_stop(true));
    for segment in self.segments {
      let recipe = if segment.selected { &chosen } else { &plain };
      let part = |part: SegmentedControlPart| recipe.part(part).forced(self.forced);
      let pick = segment.pick;
      let mut pressed = pressable(&part(SegmentedControlPart::Segment), self.disabled || segment.disabled)
        .focus_entry(segment.selected)
        .on_click(move |_| pick());
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

impl From<SegmentedControl> for Element {
  fn from(control: SegmentedControl) -> Self {
    control.build(&current_theme())
  }
}

impl Themed for SegmentedControl {
  const NAME: &'static str = "segmented_control";
  type Renderer = Arc<dyn Fn(SegmentedControl) -> Element + Send + Sync>;

  /// The design's control (`pencil/lurq-design-system.pen`, Segmented
  /// Control): a muted track as high as a control, padded 3, with segments 2
  /// apart; the chosen one is a surface with a small shadow.
  fn recipe() -> Recipe {
    use PaletteColor::*;
    use SegmentedControlPart::{Icon, Label, Root, Segment};
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
  }
}

impl super::FieldControl for SegmentedControl {
  /// Disabled where the field is. Each segment is called by its own label, and the control has no judgment.
  fn in_field(mut self, field: &super::InField) -> Element {
    self.disabled |= field.disabled;
    self.into()
  }
}
