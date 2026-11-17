//! A group of toggles of which one, or any number, are on.

use std::sync::Arc;

use super::{
  button::ButtonSize,
  names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe},
  shared::{ChangeHandler, Choice, Chosen, Orientation},
  theme::{Themed, current_theme},
  toggle::{Toggle, TogglePart},
};
use crate::{
  app::theme::{BorderSize, PaletteColor, RadiusSize, Theme},
  components::{Column, Row},
  core::{Signal, SignalValue},
  layout::Alignment,
  node::{Element, FocusGroup, NodeUpdate, border::Border},
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

/// The states of a group of toggles a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToggleGroupState {
  /// Its toggles run along a row.
  Horizontal,
  /// Its toggles run down a column.
  Vertical,
  /// The whole group takes no input.
  Disabled,
  /// What is chosen is wrong: its line is in the destructive color.
  Invalid,
}

names!(ToggleGroupState {
  Horizontal => "horizontal",
  Vertical => "vertical",
  Disabled => "disabled",
  Invalid => "invalid",
});

/// The sizes of a group's toggles: a button's.
pub type ToggleGroupSize = ButtonSize;

/// Which way a group's toggles run.
pub type ToggleGroupOrientation = Orientation;

/// What builds a group of toggles' nodes: a theme's in place of the default one.
pub type ToggleGroupRenderer = Arc<dyn Fn(ToggleGroup) -> Element + Send + Sync>;

/// Toggles that are one choice: the alignment of a paragraph, of which one
/// is on, or the marks of a text, of which any number are.
///
/// ```
/// use lurq::{core::Signal, ui::{Choice, ToggleGroup, icons}};
///
/// # fn groups(align: Signal<u8>, marks: Signal<Vec<&'static str>>) {
/// // One of them is on: the signal's value.
/// let alignment = ToggleGroup::new(align, [(0, "Left"), (1, "Center"), (2, "Right")])
///   .label("Alignment")
///   .on_change(|align| println!("aligned {align}"));
/// // Any number are: the signal's values.
/// let style = ToggleGroup::multiple(marks, [
///   Choice::new("bold", "Bold").icon(icons::BOLD).icon_only(true),
///   Choice::new("italic", "Italic").icon(icons::ITALIC).icon_only(true),
/// ])
/// .on_change(|marks| println!("{marks:?}"));
/// # }
/// ```
///
/// Bound to a signal ([`new`](Self::new), [`multiple`](Self::multiple)) the
/// group sets it, or the app holds the value and is told what it is to be
/// ([`controlled`](Self::controlled), [`controlled_multiple`](Self::controlled_multiple)).
/// [`on_change`](Self::on_change) is told the new value as a person changes it:
/// `&T` for a group of one, a `ToggleGroup<T>`, and the whole new set `&[T]`
/// for a group of any number, a `ToggleGroup<T, [T]>`. The compiler checks
/// what a handler is told. `T` is `()` where nothing names it: the theme's
/// recipe is `ToggleGroup`'s, whatever its values.
pub struct ToggleGroup<T = (), V: ?Sized = T> {
  choices: Vec<Choice<T>>,
  chosen: Chosen<T, V>,
  on_change: Vec<ChangeHandler<V>>,
  toggles: Toggles,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> ToggleGroup<T, T> {
  /// A group of which exactly one toggle is on: the one whose value the
  /// signal has. A press of another sets the signal.
  pub fn new(value: Signal<T>, choices: impl IntoIterator<Item = impl Into<Choice<T>>>) -> Self
  where
    T: SignalValue,
  {
    Self::of(Chosen::one(value), choices)
  }

  /// A group of one whose value the app holds: the toggle of `value` is on,
  /// and a press of another tells `on_change` the value it is to be. A press
  /// of the one that is on tells nothing.
  pub fn controlled(
    value: T,
    choices: impl IntoIterator<Item = impl Into<Choice<T>>>,
    on_change: impl Fn(&T) + Send + Sync + 'static,
  ) -> Self {
    Self::of(Chosen::held_one(value), choices).on_change(on_change)
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> ToggleGroup<T, [T]> {
  /// A group of which any number of toggles are on: those whose values the
  /// signal has. A press adds a value, or takes it out.
  pub fn multiple(values: Signal<Vec<T>>, choices: impl IntoIterator<Item = impl Into<Choice<T>>>) -> Self
  where
    Vec<T>: SignalValue,
  {
    Self::of(Chosen::many(values), choices)
  }

  /// A group of any number whose values the app holds: the toggles of
  /// `values` are on, and a press tells `on_change` the whole set it is to
  /// be, with the value pressed added or taken out.
  pub fn controlled_multiple(
    values: impl IntoIterator<Item = T>,
    choices: impl IntoIterator<Item = impl Into<Choice<T>>>,
    on_change: impl Fn(&[T]) + Send + Sync + 'static,
  ) -> Self {
    Self::of(Chosen::held_many(values.into_iter().collect()), choices).on_change(on_change)
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> ToggleGroup<T, V> {
  fn of(chosen: Chosen<T, V>, choices: impl IntoIterator<Item = impl Into<Choice<T>>>) -> Self {
    Self {
      choices: choices.into_iter().map(Into::into).collect(),
      chosen,
      on_change: Vec::new(),
      toggles: Toggles {
        items: Vec::new(),
        size: ButtonSize::default().into(),
        orientation: Orientation::Horizontal,
        label: None,
        disabled: false,
        invalid: false,
        states: Vec::new(),
        rules: Recipe::new(),
        forced: None,
        id: None,
        key: None,
      },
    }
  }

  /// Called as a person changes what is on, after the handlers there are,
  /// with the new value: `&T` for a group of one, the whole new set `&[T]`
  /// for a group of any number. Not called where the app sets the signal,
  /// nor for a press of what is already so.
  pub fn on_change(mut self, handler: impl Fn(&V) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// What the group is called to what reads the tree: what the toggles are a choice of.
  pub fn label(mut self, label: &str) -> Self {
    self.toggles.label = Some(Arc::from(label));
    self
  }

  /// The size of every toggle in the group: a [`ToggleGroupSize`].
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.toggles.size = size.into();
    self
  }

  /// Which way the toggles run: along a row (the default), or down a column.
  pub fn orientation(mut self, orientation: Orientation) -> Self {
    self.toggles.orientation = orientation;
    self
  }

  /// Whether the whole group takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.toggles.disabled = disabled;
    self
  }

  /// Whether what is chosen is wrong: the group's line is in the destructive color.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.toggles.invalid = invalid;
    self
  }

  /// Puts the group in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.toggles.states.retain(|held| *held != state);
    if on {
      self.toggles.states.push(state);
    }
    self
  }

  /// Styles one part of this group over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.toggles.rules = self.toggles.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this group: what the component
  /// it is in says of it in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.toggles.rules = self.toggles.rules.extend(rules);
    self
  }

  /// Draws the toggle where Tab enters the group as it is under the
  /// pointer, held down or focused from the keyboard, with nothing pointing
  /// at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.toggles.forced = interaction;
    self
  }

  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.toggles.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.toggles.key = Some(key.into());
    self
  }

  /// The group with what a press of each toggle does, of no type of the app's: what it is built from, and what a
  /// toolbar holds.
  pub(crate) fn erase(self) -> Toggles {
    let handlers: Arc<[ChangeHandler<V>]> = self.on_change.into();
    let mut toggles = self.toggles;
    toggles.items = self
      .choices
      .into_iter()
      .map(|choice| Item {
        on: self.chosen.has(&choice.value),
        pick: self.chosen.picker(choice.value, handlers.clone()),
        label: choice.label,
        icon: choice.icon,
        icon_only: choice.icon_only,
        disabled: choice.disabled,
      })
      .collect();
    toggles
  }

  /// Builds the group in `theme`. Converting a group into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    self.erase().build(theme)
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> From<ToggleGroup<T, V>> for Element {
  fn from(group: ToggleGroup<T, V>) -> Self {
    group.build(&current_theme())
  }
}

/// A group of toggles of no type of the app's: what each toggle shows and what a press of it does.
pub(crate) struct Toggles {
  items: Vec<Item>,
  size: Name,
  orientation: Orientation,
  label: Option<Arc<str>>,
  disabled: bool,
  invalid: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Toggles {
  /// The size of every toggle: a toolbar gives its own.
  pub(crate) fn size(mut self, size: Name) -> Self {
    self.size = size;
    self
  }

  /// Whether the whole group takes no input: a toolbar that is disabled says so.
  pub(crate) fn disabled(mut self, disabled: bool) -> Self {
    self.disabled |= disabled;
    self
  }

  /// What the items of a menu that stand for this group's toggles take from
  /// them, each checked while its toggle is on: a toolbar moves a group that
  /// does not fit into its menu with these.
  pub(crate) fn commands(&self) -> Vec<super::toolbar::ToolbarCommand> {
    self
      .items
      .iter()
      .map(|item| super::toolbar::ToolbarCommand {
        label: item.label.clone(),
        icon: item.icon,
        icon_only: item.icon_only,
        disabled: self.disabled || item.disabled,
        checked: Some(item.on),
        run: item.pick.clone(),
        id: None,
        key: None,
      })
      .collect()
  }

  /// Builds the group in `theme`.
  pub(crate) fn build(self, theme: &Theme) -> Element {
    let vertical = self.orientation == Orientation::Vertical;
    let mut case = Case::new()
      .size(self.size)
      .breakpoint(super::current_breakpoint())
      .state(ToggleGroupState::Horizontal, !vertical)
      .state(ToggleGroupState::Vertical, vertical)
      .state(ToggleGroupState::Disabled, self.disabled)
      .state(ToggleGroupState::Invalid, self.invalid);
    for state in &self.states {
      case = case.state(state.clone(), true);
    }
    let recipe = theme.resolve::<ToggleGroup>(&case, Some(&self.rules));
    let item = recipe.rules_for(ToggleGroupPart::Item, TogglePart::Root);
    let separator = recipe.part(ToggleGroupPart::Separator);
    let separated = separator.rest().width.is_some() || separator.rest().height.is_some();
    // Where Tab enters the group: the interaction forced is drawn there alone, as it can only be.
    let entry = self
      .items
      .iter()
      .position(|item| item.on && !item.disabled)
      .or_else(|| self.items.iter().position(|item| !item.disabled));
    let mut children = Vec::with_capacity(self.items.len() * 2);
    for (index, choice) in self.items.into_iter().enumerate() {
      if index > 0 && separated {
        children.push(Element::from(Row::new().part(&separator)));
      }
      let pick = choice.pick;
      let mut toggle = Toggle::new(&choice.label)
        .size(self.size)
        .icon_only(choice.icon_only)
        .on(choice.on)
        .disabled(self.disabled || choice.disabled)
        .rules(&item)
        .force(self.forced.filter(|_| Some(index) == entry))
        .on_change(move |_| pick());
      if let Some(name) = choice.icon {
        toggle = toggle.icon(name);
      }
      children.push(toggle.build(theme));
    }
    // A toolbar of toggles: Tab stops in it once, the arrow keys of its way move between them, and Space turns
    // the one the focus is on. The line between two of them is as long as they are across.
    let root_part = recipe.part(ToggleGroupPart::Root);
    let mut root: Element = if vertical {
      children
        .into_iter()
        .fold(
          Column::new()
            .part(&root_part)
            .focus_group(FocusGroup::vertical().one_stop(true))
            .align_items(Alignment::Stretch)
            .clip(),
          Column::child,
        )
        .into()
    } else {
      children
        .into_iter()
        .fold(
          Row::new()
            .part(&root_part)
            .focus_group(FocusGroup::horizontal().one_stop(true))
            .align_items(Alignment::Stretch)
            .clip(),
          Row::child,
        )
        .into()
    };
    let node = &mut *root.node;
    NodeUpdate::describe(node, Arc::from("a11y_role"), Arc::from("group"));
    if let Some(label) = self.label {
      NodeUpdate::describe(node, Arc::from("a11y_name"), label);
    }
    if self.invalid {
      NodeUpdate::describe(node, Arc::from("a11y_invalid"), Arc::from("true"));
    }
    if let Some(id) = self.id {
      NodeUpdate::id(node, id);
    }
    if let Some(key) = self.key {
      root = root.key(key);
    }
    root
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> crate::ui::Styled for ToggleGroup<T, V> {
  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl<T: Send + Sync + 'static, V: ?Sized + 'static> Themed for ToggleGroup<T, V> {
  const NAME: &'static str = "toggle_group";
  type Renderer = ToggleGroupRenderer;

  fn parts() -> crate::ui::Declared {
    ToggleGroupPart::declared()
  }

  fn states() -> crate::ui::Declared {
    ToggleGroupState::declared()
  }

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
      // Down a column, the line between two toggles runs across.
      .state(ToggleGroupState::Vertical, Separator, |s| s.width(Length::Auto).height(1.0))
      .state(ToggleGroupState::Invalid, Root, |s| {
        s.border(Border::inside(BorderSize::Sm, PaletteColor::Destructive))
      })
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> super::FieldControl for ToggleGroup<T, V> {
  /// Called by the field's label unless it has a name of its own, invalid where the field is, and disabled
  /// where the field is. Each toggle is called by its own label.
  fn in_field(mut self, field: &super::InField) -> Element {
    if self.toggles.label.is_none() {
      self.toggles.label = Some(field.label.clone());
    }
    self.toggles.invalid |= field.judgment == Some(super::Judgment::Invalid);
    self.toggles.disabled |= field.disabled;
    self.into()
  }
}
