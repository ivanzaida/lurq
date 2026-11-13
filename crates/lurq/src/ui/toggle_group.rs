//! A group of toggles of which one, or any number, are on.

use std::sync::Arc;

use super::{
  button::ButtonSize,
  names,
  recipe::{Case, Name, Recipe},
  shared::{Choice, Chosen},
  theme::{Themed, current_theme},
  toggle::{Toggle, TogglePart},
};
use crate::{
  app::theme::{BorderSize, PaletteColor, RadiusSize, Theme},
  components::Row,
  core::{Signal, SignalValue},
  layout::Alignment,
  node::{Element, FocusGroup, border::Border},
};

/// The parts of a group of toggles a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToggleGroupPart {
  /// The group: its corners and its line.
  Root,
  /// What the group says of each toggle in it: laid over the theme's recipe for a toggle's root.
  Item,
  /// The line between two toggles.
  Separator,
}

names!(ToggleGroupPart {
  Root => "root",
  Item => "item",
  Separator => "separator",
});

/// One toggle of a group, with what a press of it does.
struct Item {
  label: Arc<str>,
  icon: Option<Name>,
  icon_only: bool,
  disabled: bool,
  on: bool,
  pick: Arc<dyn Fn() + Send + Sync>,
}

/// Toggles that are one choice: the alignment of a paragraph, of which one
/// is on, or the marks of a text, of which any number are.
///
/// ```
/// use lurq::{core::Signal, ui::{Choice, ToggleGroup, icons}};
///
/// # fn groups(align: Signal<u8>, marks: Signal<Vec<&'static str>>) {
/// // One of them is on: the signal's value.
/// let alignment = ToggleGroup::new(align, [(0, "Left"), (1, "Center"), (2, "Right")]);
/// // Any number are: the signal's values.
/// let style = ToggleGroup::multiple(marks, [
///   Choice::new("bold", "Bold").icon(icons::BOLD).icon_only(true),
///   Choice::new("italic", "Italic").icon(icons::ITALIC).icon_only(true),
/// ]);
/// # }
/// ```
pub struct ToggleGroup {
  items: Vec<Item>,
  size: Name,
  disabled: bool,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ToggleGroup {
  fn of<T: Clone + PartialEq + Send + Sync + 'static>(
    chosen: Chosen<T>,
    choices: impl IntoIterator<Item = impl Into<Choice<T>>>,
  ) -> Self {
    let items = choices
      .into_iter()
      .map(Into::into)
      .map(|choice: Choice<T>| Item {
        on: chosen.has(&choice.value),
        pick: Arc::new(chosen.picker(choice.value)),
        label: choice.label,
        icon: choice.icon,
        icon_only: choice.icon_only,
        disabled: choice.disabled,
      })
      .collect();
    Self {
      items,
      size: ButtonSize::default().into(),
      disabled: false,
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// A group of which exactly one toggle is on: the one whose value the
  /// signal has. A press of another sets the signal.
  pub fn new<T>(value: Signal<T>, choices: impl IntoIterator<Item = impl Into<Choice<T>>>) -> Self
  where
    T: Clone + PartialEq + Send + Sync + SignalValue + 'static,
  {
    Self::of(Chosen::one(value), choices)
  }

  /// A group of which any number of toggles are on: those whose values the
  /// signal has. A press adds a value, or takes it out.
  pub fn multiple<T>(values: Signal<Vec<T>>, choices: impl IntoIterator<Item = impl Into<Choice<T>>>) -> Self
  where
    T: Clone + PartialEq + Send + Sync + 'static,
    Vec<T>: SignalValue,
  {
    Self::of(Chosen::many(values), choices)
  }

  /// A group whose state the app holds: the toggles of `on` are on, and
  /// `press` is told the value of each one pressed.
  pub fn controlled<T: Clone + PartialEq + Send + Sync + 'static>(
    on: impl IntoIterator<Item = T>,
    choices: impl IntoIterator<Item = impl Into<Choice<T>>>,
    press: impl Fn(T) + Send + Sync + 'static,
  ) -> Self {
    Self::of(Chosen::held(on.into_iter().collect(), press), choices)
  }

  /// The size of every toggle in the group: a [`ButtonSize`].
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// Whether the whole group takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Styles one part of this group over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
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

  /// Builds the group in `theme`. Converting a group into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case::new().size(self.size).breakpoint(super::current_breakpoint());
    let recipe = theme.resolve::<ToggleGroup>(&case, Some(&self.rules));
    let item = recipe.rules_for(ToggleGroupPart::Item, TogglePart::Root);
    let separator = recipe.part(ToggleGroupPart::Separator);
    let separated = separator.rest().width.is_some();
    // A toolbar of toggles: Tab stops in it once, the arrow keys move between them, and Space turns the
    // one the focus is on.
    let mut root = Row::new().part(&recipe.part(ToggleGroupPart::Root))
      .focus_group(FocusGroup::horizontal().one_stop(true))
      // The line between two of them is as high as they are.
      .align_items(Alignment::Stretch)
      .clip();
    for (index, choice) in self.items.into_iter().enumerate() {
      if index > 0 && separated {
        root = root.child(Row::new().part(&separator));
      }
      let pick = choice.pick;
      let mut toggle = Toggle::new(&choice.label)
        .size(self.size)
        .icon_only(choice.icon_only)
        .on(choice.on)
        .disabled(self.disabled || choice.disabled)
        .rules(&item)
        .on_change(move |_| pick());
      if let Some(name) = choice.icon {
        toggle = toggle.icon(name);
      }
      root = root.child(toggle.build(theme));
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

impl From<ToggleGroup> for Element {
  fn from(group: ToggleGroup) -> Self {
    group.build(&current_theme())
  }
}

impl Themed for ToggleGroup {
  const NAME: &'static str = "toggle_group";
  type Renderer = Arc<dyn Fn(ToggleGroup) -> Element + Send + Sync>;

  /// The design's group (`pencil/lurq-design-system.pen`, Toggle Group): a
  /// line around the toggles and one between them, the corners of a control.
  fn recipe() -> Recipe {
    use PaletteColor::{Background, Input};
    use ToggleGroupPart::{Item, Root, Separator};
    Recipe::new()
      .base(Root, |s| {
        s.radius(RadiusSize::Md)
          .background(Background)
          .border(Border::inside(BorderSize::Sm, Input))
      })
      .base(Item, |s| s.radius(0.0))
      .base(Separator, |s| s.width(1.0).background(Input))
  }
}
