//! A toolbar: the tools of what is in front of a person, in a row.
//!
//! It is a mounted component, not a builder that converts into an element:
//! the menu of the controls that do not fit is open or not, it is made of
//! what the last layout dropped, and the bar remembers which of its controls
//! the focus left, none of which a builder can keep. [`Toolbar`] says what is
//! in it, and [`Toolbar::mount`] puts it in the tree.

use std::sync::Arc;

use parking_lot::Mutex;

use super::{
  button::{Button, ButtonPart, ButtonSize, ButtonVariant},
  icons,
  menu::{Menu, MenuItem},
  names,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  separator::{Separator, SeparatorPart},
  theme::{Themed, current_theme},
  toggle::{Toggle, TogglePart},
  toggle_group::{ToggleGroup, Toggles},
  tooltip::Tooltip,
};
use crate::{
  app::{
    component::Component,
    ctx::{Ctx, Placement},
    events::{EventControl, MouseButton, MouseEvent, MouseEventKind, PointerKind},
    theme::{BorderSize, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle},
  },
  components::{Column, Row, ScrollHorizontal},
  core::{ElementRef, NodeId, Signal},
  layout::{Alignment, layout_kind::ShrinkLimit, scrollbar::ScrollBarStyle},
  node::{Element, FocusGroup, NodeUpdate, border::Border, node::Node, outline::Outline},
};

/// The click a press of the keyboard makes: what a menu item that stands for
/// a button gives the button's handlers.
pub(crate) fn keyboard_click() -> MouseEvent {
  MouseEvent {
    x: 0.0,
    y: 0.0,
    button: MouseButton::Left,
    kind: MouseEventKind::Click,
    shift: false,
    ctrl: false,
    alt: false,
    meta: false,
    from_keyboard: true,
    pointer: PointerKind::Mouse,
    target_id: NodeId::UNASSIGNED,
    time: crate::time::Instant::now(),
    control: EventControl::new(),
  }
}

/// What stands for a control of a toolbar in the menu of what does not fit:
/// its label and icon, its shortcut, whether it is checked, and what choosing
/// it does. A toolbar makes one from a button, a toggle, the toggles of a
/// group; the app gives one for a control the toolbar cannot read, a select
/// or a field ([`ToolbarItem::in_menu`]).
///
/// ```
/// use lurq::ui::{ToolbarCommand, icons};
///
/// let columns = ToolbarCommand::new("Columns").icon(icons::SLIDERS).on_select(|| println!("columns"));
/// ```
#[derive(Clone)]
pub struct ToolbarCommand {
  pub(crate) label: Arc<str>,
  pub(crate) icon: Option<Name>,
  /// Whether the control is its icon alone in the bar: it has a tooltip there.
  pub(crate) icon_only: bool,
  pub(crate) disabled: bool,
  /// Whether it is checked, for a toggle; `None` for what is not one.
  pub(crate) checked: Option<bool>,
  pub(crate) run: Arc<dyn Fn() + Send + Sync>,
  /// The id of the control: its item has it with `-item` after it.
  pub(crate) id: Option<Arc<str>>,
  /// The key of the item in the menu.
  pub(crate) key: Option<Arc<str>>,
}

impl ToolbarCommand {
  /// An item called `label`, which does nothing until it is given [`on_select`](Self::on_select).
  pub fn new(label: &str) -> Self {
    Self {
      label: Arc::from(label),
      icon: None,
      icon_only: false,
      disabled: false,
      checked: None,
      run: Arc::new(|| {}),
      id: None,
      key: None,
    }
  }

  /// The key of the item in the menu.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The id of the item in the menu.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// An icon before the label, by its name in the theme's set.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// Whether it is checked: an item with a check mark, on or off.
  pub fn checked(mut self, checked: bool) -> Self {
    self.checked = Some(checked);
    self
  }

  /// Whether it cannot be chosen.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// What choosing it does, after the handlers it has.
  pub fn on_select(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    let before = self.run.clone();
    self.run = Arc::new(move || {
      before();
      handler();
    });
    self
  }

  /// The item of the menu, with `shortcut` beside it.
  pub(crate) fn item(&self, shortcut: Option<&str>) -> MenuItem {
    let run = self.run.clone();
    let mut item = MenuItem::new(&self.label)
      .disabled(self.disabled)
      .on_select(move || run());
    if let Some(icon) = self.icon {
      item = item.icon(icon);
    }
    if let Some(checked) = self.checked {
      item = item.checked(checked);
    }
    if let Some(keys) = shortcut {
      item = item.shortcut(keys);
    }
    if let Some(id) = &self.id {
      item = item.id(format!("{id}-item"));
    }
    if let Some(key) = &self.key {
      item = item.key(key.clone());
    }
    item
  }
}

/// How a toolbar is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ToolbarVariant {
  /// No fill and no edge: a bar in a header, a pane's title, a bar of its own.
  Plain,
  /// The background's fill with an edge around it: the tools over an editor, the bar over a table.
  #[default]
  Surface,
  /// The popover's fill, an edge and a shadow: a bar that floats over what it works on.
  Floating,
}

names!(ToolbarVariant {
  Plain => "plain",
  Surface => "surface",
  Floating => "floating",
});

/// The parts of a toolbar a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToolbarPart {
  /// The bar: its fill, its edge, its padding and the room between its controls.
  Root,
  /// What the bar says of each button and toggle in it: laid over the theme's recipe for their root.
  Item,
  /// A control as the bar lays it out, with the line before it where one stands there: the room between the two.
  Slot,
  /// What the bar says of the line between two groups: laid over the theme's recipe for a separator's root.
  Separator,
  /// What pushes the controls after it to the end of the bar.
  Spacer,
  /// A group with a label under it (the command bar's): its room and the line at its end.
  Group,
  /// The row of a labelled group's controls.
  GroupItems,
  /// The label under a group.
  GroupLabel,
  /// Controls one over another in one place of the bar.
  Stack,
  /// The end of the bar while some controls do not fit: the line and the More button.
  More,
  /// What the bar says of the More button, after `Item`: laid over a button's root.
  MoreButton,
}

names!(ToolbarPart {
  Root => "root",
  Item => "item",
  Slot => "slot",
  Separator => "separator",
  Spacer => "spacer",
  Group => "group",
  GroupItems => "group_items",
  GroupLabel => "group_label",
  Stack => "stack",
  More => "more",
  MoreButton => "more_button",
});

/// The states of a toolbar a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToolbarState {
  /// Its controls take no input.
  Disabled,
  /// Some controls did not fit, and are in the menu at the end.
  Overflowing,
}

names!(ToolbarState {
  Disabled => "disabled",
  Overflowing => "overflowing",
});

/// What builds a toolbar's nodes: a theme's in place of the default one.
pub type ToolbarRenderer = Arc<dyn Fn(ToolbarView) -> Element + Send + Sync>;

/// The sizes of a toolbar's buttons and toggles: a button's.
pub type ToolbarSize = ButtonSize;

/// What a control of a toolbar is.
enum Kind {
  Button(Button),
  Toggle(Toggle),
  ToggleGroup(Toggles),
  /// A select, a field, anything else: it keeps its own keys, and goes into the menu only with what the app
  /// gives to stand for it.
  Element(Element, Option<ToolbarCommand>),
  Separator,
  Spacer,
  Group(ToolbarGroup),
  /// Controls one over another in one place of the bar: a command bar's two small commands.
  Stack(Vec<ToolbarItem>),
  /// A control of another component's that the bar builds in its theme, told whether the bar takes no input: a
  /// command of a command bar.
  Made(Box<Made>),
}

/// What builds a control the bar cannot read: its element in the bar's theme, and what stands for it in the menu.
pub(crate) type Made = dyn FnOnce(&Theme, bool) -> (Element, ToolbarCommand);

/// One thing of a toolbar: a control, the line between two groups, the room
/// that pushes what follows to the end, a group.
///
/// A button, a toggle, a group of toggles, a select or any element converts
/// into one, so that [`Toolbar::child`] takes them as they are; this is where
/// a control is given a shortcut, or told to stay in the bar.
///
/// ```
/// use lurq::ui::{Button, ToolbarItem, icons};
///
/// let save = ToolbarItem::from(Button::new("Share").icon(icons::SHARE).icon_only(true)).shortcut("Ctrl+Shift+S");
/// let search = ToolbarItem::from(Button::new("Search").icon(icons::SEARCH).icon_only(true)).stays(true);
/// ```
pub struct ToolbarItem {
  kind: Kind,
  shortcut: Option<Arc<str>>,
  stays: Option<bool>,
  as_given: bool,
}

impl ToolbarItem {
  fn of(kind: Kind) -> Self {
    Self {
      kind,
      shortcut: None,
      stays: None,
      as_given: false,
    }
  }

  /// Any element: a field, a search box, a select. It keeps its own keys
  /// while it is focused, and stays in the bar when the bar is narrow, unless
  /// it is given an item to stand for it in the menu ([`in_menu`](Self::in_menu)).
  pub fn element(element: impl Into<Element>) -> Self {
    Self::of(Kind::Element(element.into(), None))
  }

  /// Controls one over another in one place of the bar, which goes into the
  /// menu whole where it does not fit: a command bar's two small commands, Cut
  /// over Copy.
  pub fn stack<I: Into<ToolbarItem>>(items: impl IntoIterator<Item = I>) -> Self {
    Self::of(Kind::Stack(items.into_iter().map(Into::into).collect()))
  }

  /// A control another component builds in the bar's theme, told whether the
  /// bar takes no input, with what stands for it in the menu.
  pub(crate) fn made(make: impl FnOnce(&Theme, bool) -> (Element, ToolbarCommand) + 'static) -> Self {
    Self::of(Kind::Made(Box::new(make)))
  }

  /// The keys that do what the control does, shown in its tooltip and beside its item in the menu: "Ctrl+B".
  pub fn shortcut(mut self, keys: &str) -> Self {
    self.shortcut = Some(Arc::from(keys));
    self
  }

  /// Whether the control stays in the bar however narrow it is, and never
  /// goes into the menu: what a person reaches for first. A field stays
  /// unless it has an item to stand for it.
  pub fn stays(mut self, stays: bool) -> Self {
    self.stays = Some(stays);
    self
  }

  /// Whether the button or toggle keeps the variant and the size it was
  /// given, and what it was styled with, over the bar's: the outline Export
  /// of a table's bar. Otherwise it is the bar's, ghost and of its size.
  pub fn as_given(mut self, as_given: bool) -> Self {
    self.as_given = as_given;
    self
  }

  /// Whether it is the room that pushes what follows to the end of the bar.
  pub(crate) fn is_spacer(&self) -> bool {
    matches!(self.kind, Kind::Spacer)
  }

  /// What stands for an element (a select, a field) in the menu of what does
  /// not fit: with it, the element goes into the menu as the bar narrows, as
  /// a button does.
  pub fn in_menu(mut self, command: ToolbarCommand) -> Self {
    if let Kind::Element(_, stand_in) = &mut self.kind {
      *stand_in = Some(command);
    }
    self
  }
}

impl From<Button> for ToolbarItem {
  fn from(button: Button) -> Self {
    Self::of(Kind::Button(button))
  }
}

impl From<Toggle> for ToolbarItem {
  fn from(toggle: Toggle) -> Self {
    Self::of(Kind::Toggle(toggle))
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> From<ToggleGroup<T, V>> for ToolbarItem {
  fn from(group: ToggleGroup<T, V>) -> Self {
    Self::of(Kind::ToggleGroup(group.erase()))
  }
}

impl<V: 'static> From<super::Select<V>> for ToolbarItem {
  fn from(select: super::Select<V>) -> Self {
    Self::element(select)
  }
}

impl From<ToolbarGroup> for ToolbarItem {
  fn from(group: ToolbarGroup) -> Self {
    Self::of(Kind::Group(group))
  }
}

impl From<Element> for ToolbarItem {
  fn from(element: Element) -> Self {
    Self::element(element)
  }
}

/// Controls of a toolbar that belong together: a line stands between it and
/// what is beside it. With a label under it, it is a group of a command bar
/// ("Clipboard", "Format"), and goes into the menu whole, under its label.
///
/// ```
/// use lurq::ui::{Button, ToolbarGroup, icons};
///
/// let history = ToolbarGroup::new()
///   .item(Button::new("Undo").icon(icons::UNDO).icon_only(true))
///   .item(Button::new("Redo").icon(icons::REDO).icon_only(true));
/// let clipboard = ToolbarGroup::new().label("Clipboard").item(Button::new("Paste").icon(icons::PASTE));
/// ```
#[derive(Default)]
pub struct ToolbarGroup {
  items: Vec<ToolbarItem>,
  label: Option<Arc<str>>,
}

impl ToolbarGroup {
  pub fn new() -> Self {
    Self::default()
  }

  /// A control of the group, after the ones it has.
  pub fn item(mut self, item: impl Into<ToolbarItem>) -> Self {
    self.items.push(item.into());
    self
  }

  pub fn items<I: Into<ToolbarItem>>(mut self, items: impl IntoIterator<Item = I>) -> Self {
    self.items.extend(items.into_iter().map(Into::into));
    self
  }

  /// What the group is, under its controls and over them in the menu: "Clipboard".
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }
}

/// A toolbar: the tools of what is in front of a person, in a row. The marks
/// of a text, the actions on a table, the views of a canvas.
///
/// ```
/// use lurq::{
///   app::ctx::Ctx,
///   node::Element,
///   ui::{Button, Toggle, Toolbar, ToolbarItem, icons},
/// };
///
/// # fn build(ctx: &mut Ctx, bold: bool) -> Element {
/// Toolbar::new()
///   .label("Formatting")
///   .item(ToolbarItem::from(Toggle::new("Bold").icon(icons::BOLD).icon_only(true).on(bold)).shortcut("Ctrl+B"))
///   .item(Toggle::new("Italic").icon(icons::ITALIC).icon_only(true))
///   .item(Toolbar::separator())
///   .item(Button::new("Link").icon(icons::LINK).icon_only(true))
///   .item(Toolbar::spacer())
///   .item(Button::new("Share"))
///   .overflow(true)
///   .mount(ctx)
/// # }
/// ```
///
/// Tab stops in the bar once, on the control the focus last left (the first,
/// the first time); Left and Right move between its controls, Home and End go
/// to the first and the last, and a disabled one is passed by. A field or a
/// select in it keeps the arrow keys while it is focused, and Tab leaves it
/// as it leaves the bar. A button that is its icon alone is called by its
/// label, which its tooltip says with its shortcut.
///
/// The bar gives its buttons and toggles its size, and makes its buttons
/// ghost ones, unless an item says [`as_given`](ToolbarItem::as_given). With
/// [`overflow`](Self::overflow), the controls that do not fit go into a menu
/// at the end of the bar, behind a More button, the last first: each as an
/// item of the menu with its label, its icon, its shortcut, checked for a
/// toggle, and its press; a control that [`stays`](ToolbarItem::stays) never
/// does, nor a select, a field or any element that was given nothing to
/// stand for it ([`in_menu`](ToolbarItem::in_menu)). What is kept is never
/// past the bar: a bar narrower than what stays in it, and one that puts
/// nothing in a menu and is narrower than its controls, scrolls sideways to
/// it, by the wheel, a drag, and the keys, which bring the control the focus
/// goes to into view.
pub struct Toolbar {
  items: Vec<ToolbarItem>,
  variant: Name,
  size: Name,
  overflow: bool,
  label: Option<Arc<str>>,
  more_label: Arc<str>,
  more_icon: Name,
  secondary: Vec<ToolbarCommand>,
  /// Whether its controls stand at its top, where they are centered across it.
  top: bool,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Default for Toolbar {
  fn default() -> Self {
    Self::new()
  }
}

impl Toolbar {
  pub fn new() -> Self {
    Self {
      items: Vec::new(),
      variant: ToolbarVariant::default().into(),
      size: ButtonSize::Sm.into(),
      overflow: false,
      label: None,
      more_label: super::current_locale().common.more.clone(),
      more_icon: Name::from(icons::MORE_HORIZONTAL),
      secondary: Vec::new(),
      top: false,
      disabled: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// The line between two groups of controls.
  pub fn separator() -> ToolbarItem {
    ToolbarItem::of(Kind::Separator)
  }

  /// The room that pushes the controls after it to the end of the bar.
  pub fn spacer() -> ToolbarItem {
    ToolbarItem::of(Kind::Spacer)
  }

  /// A thing of the bar, after the ones it has: a button, a toggle, a group
  /// of toggles, a select, a [`ToolbarItem`], a [`ToolbarGroup`],
  /// [`Toolbar::separator`], [`Toolbar::spacer`].
  pub fn item(mut self, item: impl Into<ToolbarItem>) -> Self {
    self.items.push(item.into());
    self
  }

  pub fn items<I: Into<ToolbarItem>>(mut self, items: impl IntoIterator<Item = I>) -> Self {
    self.items.extend(items.into_iter().map(Into::into));
    self
  }

  /// A [`ToolbarVariant`], or a variant the theme's recipe names.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// The size of the bar's buttons and toggles: a [`ToolbarSize`], small by
  /// default, as the board draws the bar. The bar is as high as they are and
  /// its padding.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// Whether the controls that do not fit go into a menu at the end of the
  /// bar. Without it, a bar narrower than its controls cuts them off.
  pub fn overflow(mut self, overflow: bool) -> Self {
    self.overflow = overflow;
    self
  }

  /// What the bar is called, to what reads the tree: "Formatting".
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// What the More button is called, in its tooltip and to what reads the tree: "More" by default.
  pub fn more_label(mut self, label: &str) -> Self {
    self.more_label = Arc::from(label);
    self
  }

  /// Puts the controls at the top of the bar, where they are centered across
  /// it: a command bar's, whose chevron is level with its large commands.
  pub(crate) fn top(mut self, top: bool) -> Self {
    self.top = top;
    self
  }

  /// A command that is never in the bar, only in the menu of its More button,
  /// after what did not fit: a command a person seldom needs ("Settings",
  /// "Customize"). With one, the More button is always there.
  pub fn secondary_command(mut self, command: ToolbarCommand) -> Self {
    self.secondary.push(command);
    self
  }

  /// Commands that are only in the menu, after the ones the bar has.
  pub fn secondary_commands(mut self, commands: impl IntoIterator<Item = ToolbarCommand>) -> Self {
    self.secondary.extend(commands);
    self
  }

  /// The icon of the More button, by its name in the theme's set: three dots
  /// by default, a command bar's chevron.
  pub fn more_icon(mut self, icon: impl Into<Name>) -> Self {
    self.more_icon = icon.into();
    self
  }

  /// Whether the bar's controls take no input: a bar of a document that is
  /// read only. Each button, toggle and group of toggles in it is disabled,
  /// and so are the items of its menu and its More button. An element given
  /// as it is ([`ToolbarItem::element`]) is the app's to disable.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Puts the bar in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Draws the More button as it is under the pointer, held down or focused
  /// from the keyboard, with nothing pointing at it: for a gallery or a test.
  /// A control of the bar is forced with its own `force`.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// Styles one part of this toolbar over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this toolbar.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the bar. Its More button has this id with `-more` after it, and the menu `-menu`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Puts the toolbar in the tree of `ctx`'s component, in the theme of what
  /// is being rendered.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let (bar, key) = self.prepare(&current_theme());
    let mut element = ctx.mount::<ToolbarHost>(ToolbarProps { bar });
    if let Some(key) = key {
      element = element.key(key);
    }
    element
  }

  /// Builds the bar in `theme` once, with nothing kept: its controls without
  /// their tooltips, the focus entering at the first, and no menu, so that a
  /// bar narrower than its controls scrolls sideways to them. For a gallery, a test, a
  /// window of its own; converting a toolbar into an element does this in the
  /// theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let (bar, key) = self.prepare(theme);
    let count = bar.slots.len();
    let recipe = resolve(theme, bar.variant, bar.size, &bar.rules, &bar.states, false);
    let view = ToolbarView {
      shown: bar
        .slots
        .iter()
        .map(|slot| compose(&recipe, slot, &mut |part| part.element.clone()))
        .collect(),
      slots: bar.slots,
      dropped: vec![false; count],
      more: None,
      overflow: false,
      secondary: bar.secondary,
      label: bar.label,
      recipe,
      refs: (0..count).map(|_| ElementRef::new()).collect(),
      dropped_signal: Signal::new(Vec::new()),
      open: Signal::new(false),
      anchor: ElementRef::new(),
      top: bar.top,
      id: bar.id,
    };
    let mut element = match theme.renderer::<Toolbar>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    };
    if let Some(key) = key {
      element = element.key(key);
    }
    element
  }

  /// The bar's slots built in `theme`, with what its host needs, and its key.
  fn prepare(self, theme: &Theme) -> (Prepared, Option<Arc<str>>) {
    let mut states = self.states;
    if self.disabled {
      states.push(ToolbarState::Disabled.into());
    }
    let recipe = resolve(theme, self.variant, self.size, &self.rules, &states, false);
    let mut slots = Vec::new();
    let mut separated = false;
    let count = self.items.len();
    for (index, item) in self.items.into_iter().enumerate() {
      let place = Place {
        recipe: &recipe,
        size: self.size,
        disabled: self.disabled,
      };
      prepare(item, &place, &mut slots, &mut separated, index + 1 < count);
    }
    // A labelled group's line stands between it and what follows: the spacer and the end of the bar are not that.
    for index in 0..slots.len() {
      let line = slots.get(index + 1).is_some_and(|next| !next.spacer);
      slots[index].line_after = line;
    }
    let bar = Prepared {
      slots,
      variant: self.variant,
      size: self.size,
      overflow: self.overflow,
      label: self.label,
      more_label: self.more_label,
      more_icon: self.more_icon,
      secondary: secondary_of(self.secondary, self.disabled),
      top: self.top,
      disabled: self.disabled,
      states,
      rules: self.rules,
      forced: self.forced,
      id: self.id,
    };
    (bar, self.key)
  }
}

/// The commands only in the menu, each disabled where the bar is.
fn secondary_of(commands: Vec<ToolbarCommand>, disabled: bool) -> Vec<ToolbarCommand> {
  commands
    .into_iter()
    .map(|command| if disabled { command.disabled(true) } else { command })
    .collect()
}

impl From<Toolbar> for Element {
  fn from(toolbar: Toolbar) -> Self {
    toolbar.build(&current_theme())
  }
}

fn resolve(
  theme: &Theme,
  variant: Name,
  size: Name,
  rules: &Recipe,
  states: &[Name],
  overflowing: bool,
) -> ResolvedRecipe {
  let mut case = Case::new()
    .variant(variant)
    .size(size)
    .breakpoint(super::current_breakpoint())
    .state(ToolbarState::Overflowing, overflowing);
  for state in states {
    case = case.state(state.clone(), true);
  }
  theme.resolve::<Toolbar>(&case, Some(rules))
}

/// Where the controls of a bar are prepared: its recipe, its size, and whether it takes no input.
struct Place<'a> {
  recipe: &'a ResolvedRecipe,
  size: Name,
  disabled: bool,
}

/// One thing of the bar as it is laid out: a control, a labelled group or the spacer.
#[derive(Clone)]
pub struct ToolbarSlot {
  /// The control as it is drawn, in the bar's dress.
  pub element: Element,
  /// Whether a line stands before it.
  pub separated: bool,
  /// Whether it is the room that pushes what follows to the end.
  pub spacer: bool,
  /// Whether it never goes into the menu.
  pub stays: bool,
  /// What stands for it in the menu: one item, or a group's.
  pub commands: Vec<ToolbarCommand>,
  /// The label over its items in the menu: a labelled group's.
  pub label: Option<Arc<str>>,
  /// Its keys, shown in its tooltip and beside its item.
  pub shortcut: Option<Arc<str>>,
  /// Whether it is its icon alone: it has a tooltip of its label and shortcut.
  pub tooltip: bool,
  /// The controls of a labelled group or a stack, which `element` stands empty
  /// for: a group's are drawn over its label, a stack's one over another.
  pub parts: Vec<ToolbarSlot>,
  /// Whether its parts stand one over another.
  pub stack: bool,
  /// Whether a labelled group draws the line at its end: where something but the spacer follows it.
  pub line_after: bool,
}

/// Turns `item` into the slots of the bar, after `slots`: a line that ends a
/// group is held in `separated` for the next.
fn prepare(item: ToolbarItem, place: &Place, slots: &mut Vec<ToolbarSlot>, separated: &mut bool, more_follow: bool) {
  let (recipe, size) = (place.recipe, place.size);
  let theme = recipe.theme();
  let shortcut = item.shortcut.clone();
  let slot =
    |separated: &mut bool, element: Element, commands: Vec<ToolbarCommand>, stays: bool, tooltip: bool| ToolbarSlot {
      element,
      separated: std::mem::take(separated),
      spacer: false,
      stays,
      commands,
      label: None,
      shortcut: shortcut.clone(),
      tooltip,
      parts: Vec::new(),
      stack: false,
      line_after: false,
    };
  match item.kind {
    Kind::Button(button) => {
      let button = if place.disabled { button.disabled(true) } else { button };
      let command = button.command();
      let button = if item.as_given {
        button
      } else {
        button
          .variant(ButtonVariant::Ghost)
          .size(size)
          .rules(&recipe.rules_for(ToolbarPart::Item, ButtonPart::Root))
      };
      let tooltip = command.icon_only;
      let made = slot(
        separated,
        button.build(theme),
        vec![command],
        item.stays.unwrap_or(false),
        tooltip,
      );
      slots.push(made);
    }
    Kind::Toggle(toggle) => {
      let toggle = if place.disabled { toggle.disabled(true) } else { toggle };
      let command = toggle.command();
      let toggle = if item.as_given {
        toggle
      } else {
        toggle
          .size(size)
          .rules(&recipe.rules_for(ToolbarPart::Item, TogglePart::Root))
      };
      let tooltip = command.icon_only;
      let made = slot(
        separated,
        toggle.build(theme),
        vec![command],
        item.stays.unwrap_or(false),
        tooltip,
      );
      slots.push(made);
    }
    Kind::ToggleGroup(group) => {
      let group = if place.disabled { group.disabled(true) } else { group };
      let commands = group.commands();
      let group = if item.as_given { group } else { group.size(size) };
      let mut element = group.build(theme);
      // Its toggles are among the bar's controls: the bar's arrow keys move through them and on.
      element.node.focus_group = None;
      let made = slot(separated, element, commands, item.stays.unwrap_or(false), false);
      slots.push(made);
    }
    Kind::Element(element, stand_in) => {
      let stand_in = stand_in.map(|command| {
        if place.disabled {
          command.disabled(true)
        } else {
          command
        }
      });
      let stays = item.stays.unwrap_or(stand_in.is_none());
      let made = slot(separated, element, stand_in.into_iter().collect(), stays, false);
      slots.push(made);
    }
    Kind::Made(make) => {
      let (element, command) = make(theme, place.disabled);
      let made = slot(separated, element, vec![command], item.stays.unwrap_or(false), false);
      slots.push(made);
    }
    Kind::Separator => *separated = !slots.is_empty(),
    Kind::Spacer => {
      let mut made = slot(separated, Element::new(), Vec::new(), true, false);
      made.spacer = true;
      slots.push(made);
    }
    Kind::Stack(items) => {
      let mut inner_slots = Vec::new();
      let mut inner_separated = false;
      for inner in items {
        prepare(inner, place, &mut inner_slots, &mut inner_separated, false);
      }
      let commands = inner_slots
        .iter()
        .flat_map(|inner| inner.commands.iter().cloned())
        .collect();
      let stays = inner_slots.iter().all(|inner| inner.stays);
      let mut made = slot(separated, Element::new(), commands, item.stays.unwrap_or(stays), false);
      made.stack = true;
      made.parts = inner_slots;
      slots.push(made);
    }
    Kind::Group(group) => {
      // A group stands apart from what is before it and from what follows. A
      // labelled group's line is at its end: no other stands after it, nor
      // between it and the labelled group before it.
      let labelled = group.label.is_some();
      let after_labelled = slots.last().is_some_and(|last| last.label.is_some());
      if !slots.is_empty() && !(labelled && after_labelled) {
        *separated = true;
      }
      match group.label {
        None => {
          let count = group.items.len();
          for (index, inner) in group.items.into_iter().enumerate() {
            prepare(inner, place, slots, separated, index + 1 < count);
          }
        }
        Some(label) => {
          let mut inner_slots = Vec::new();
          let mut inner_separated = false;
          let count = group.items.len();
          for (index, inner) in group.items.into_iter().enumerate() {
            prepare(inner, place, &mut inner_slots, &mut inner_separated, index + 1 < count);
          }
          let commands = inner_slots
            .iter()
            .flat_map(|inner| inner.commands.iter().cloned())
            .collect();
          let stays = inner_slots.iter().all(|inner| inner.stays);
          let mut made = slot(separated, Element::new(), commands, item.stays.unwrap_or(stays), false);
          made.label = Some(label);
          made.parts = inner_slots;
          slots.push(made);
        }
      }
      if more_follow && !labelled {
        *separated = true;
      }
    }
  }
}

/// A slot of the bar as it is drawn: a labelled group's controls over its
/// label, a stack's one over another, each control as `show` makes it.
fn compose(recipe: &ResolvedRecipe, slot: &ToolbarSlot, show: &mut dyn FnMut(&ToolbarSlot) -> Element) -> Element {
  match &slot.label {
    Some(label) if !slot.parts.is_empty() => {
      let parts = slot.parts.iter().map(|part| compose(recipe, part, show)).collect();
      labelled(recipe, label, parts, slot.line_after)
    }
    _ if slot.stack => slot
      .parts
      .iter()
      .fold(
        Column::new().part(&recipe.part(ToolbarPart::Stack)).overflow_visible(),
        |column, part| column.child(compose(recipe, part, show)),
      )
      .into(),
    _ => show(slot),
  }
}

/// A labelled group of the bar: its controls in a row over its label, and
/// the line at its end where `line` says.
fn labelled(recipe: &ResolvedRecipe, label: &str, controls: Vec<Element>, line: bool) -> Element {
  let part = recipe.part(ToolbarPart::GroupLabel);
  let mut row = Row::new()
    .part(&recipe.part(ToolbarPart::GroupItems))
    .align_items(Alignment::Center)
    .overflow_visible();
  for control in controls {
    row = row.child(control);
  }
  let mut group: Element = Column::new()
    .part(&recipe.part(ToolbarPart::Group))
    .align_items(Alignment::Center)
    .overflow_visible()
    .child(row)
    .child(part.text(label).nowrap().part(&part))
    .into();
  if !line {
    // The last group of the bar, or the last before its spacer: no line stands after it.
    NodeUpdate::border_right(&mut *group.node, Border::inside(0.0, PaletteColor::Border));
  }
  group
}

/// What the host of a toolbar is given: the bar as it was prepared in the
/// theme of what mounted it.
#[derive(Clone)]
struct Prepared {
  slots: Vec<ToolbarSlot>,
  variant: Name,
  size: Name,
  overflow: bool,
  label: Option<Arc<str>>,
  more_label: Arc<str>,
  more_icon: Name,
  secondary: Vec<ToolbarCommand>,
  top: bool,
  disabled: bool,
  /// The states the app put the bar in, and `Disabled` where it is.
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
}

#[derive(Clone)]
struct ToolbarProps {
  bar: Prepared,
}

/// The bar is prepared anew by every render of what holds it, so props are
/// never the same as before.
impl PartialEq for ToolbarProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for ToolbarProps {}

/// What a toolbar keeps from one render to the next.
struct ToolbarHost {
  /// Whether the menu of what does not fit is open.
  open: Signal<bool>,
  /// Which slots the last layout dropped, by index.
  dropped: Signal<Vec<bool>>,
  /// Which of the bar's stops of Tab the focus was last on: where Tab enters the bar.
  last: Signal<usize>,
  /// Where each slot was laid out: a dropped one has no size.
  refs: Mutex<Vec<ElementRef>>,
  /// The More button, which the menu hangs from.
  anchor: ElementRef,
}

/// Sets `signal` to `value` where it is not that already: a signal set renders again what reads it.
fn put<T: Clone + PartialEq + Send + Sync + crate::core::SignalValue + 'static>(signal: &Signal<T>, value: T) {
  if signal.get_untracked() != value {
    signal.set(value);
  }
}

/// Marks the stops of Tab in `node`, from `next` on: the one at `last` is
/// where Tab enters the bar, and each tells `remember` when it takes the focus.
fn mark_stops(node: &mut Node, next: &mut usize, last: usize, remember: &Signal<usize>) {
  if node.tab_index_value().is_some_and(|index| index >= 0) && node.is_focusable() {
    let stop = *next;
    *next += 1;
    NodeUpdate::focus_entry(node, stop == last);
    let remember = remember.clone();
    NodeUpdate::on_focus(node, move || put(&remember, stop));
    // A button's content is part of the button.
    if node.button_kind_value().is_some() && !node.has_stops_inside() {
      return;
    }
  }
  for child in &mut node.children {
    mark_stops(child, next, last, remember);
  }
}

impl Component for ToolbarHost {
  type Props = ToolbarProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      open: ctx.signal(false),
      dropped: ctx.signal(Vec::new()),
      last: ctx.signal(0),
      refs: Mutex::new(Vec::new()),
      anchor: ElementRef::new(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let ToolbarProps { mut bar } = ctx.props::<ToolbarProps>().clone();
    let theme = current_theme();
    let count = bar.slots.len();
    let dropped: Vec<bool> = {
      let read = self.dropped.get();
      (0..count)
        .map(|index| bar.overflow && read.get(index).copied().unwrap_or(false) && !bar.slots[index].stays)
        .collect()
    };
    let overflowing = dropped.iter().any(|dropped| *dropped);
    // A labelled group's line stands before what is shown after it: none before the menu of what went.
    for index in 0..count {
      let next = (index + 1..count).find(|next| !dropped[*next]);
      bar.slots[index].line_after = next.is_some_and(|next| !bar.slots[next].spacer);
    }
    let recipe = resolve(&theme, bar.variant, bar.size, &bar.rules, &bar.states, overflowing);
    let refs = {
      let mut refs = self.refs.lock();
      while refs.len() < count {
        refs.push(ElementRef::new());
      }
      refs[..count].to_vec()
    };
    // Each control in the bar, its stops marked, an icon alone with its tooltip.
    let (last, mut next) = (self.last.get(), 0);
    let mut shown = Vec::with_capacity(count);
    for slot in &bar.slots {
      let mut show = |slot: &ToolbarSlot| {
        let mut element = slot.element.clone();
        mark_stops(&mut element.node, &mut next, last, &self.last);
        if slot.tooltip
          && let Some(command) = slot.commands.first()
        {
          let mut tooltip = Tooltip::new(&command.label).placement(Placement::Bottom);
          if let Some(keys) = &slot.shortcut {
            tooltip = tooltip.shortcut(keys);
          }
          element = tooltip.mount(ctx, element);
        }
        element
      };
      let element = compose(&recipe, slot, &mut show);
      shown.push(element);
    }
    let more = if bar.overflow || !bar.secondary.is_empty() {
      let mut button = Button::new(&bar.more_label)
        .icon(bar.more_icon)
        .icon_only(true)
        .variant(ButtonVariant::Ghost)
        .size(bar.size)
        .disabled(bar.disabled)
        .force(bar.forced)
        .rules(
          &recipe
            .rules_for(ToolbarPart::Item, ButtonPart::Root)
            .extend(&recipe.rules_for(ToolbarPart::MoreButton, ButtonPart::Root)),
        )
        .on_click({
          let open = self.open.clone();
          move |_| open.update(|open| *open = !*open)
        });
      if let Some(id) = &bar.id {
        button = button.id(format!("{id}-more"));
      }
      let mut element = button.build(&theme);
      NodeUpdate::describe(&mut *element.node, Arc::from("a11y_haspopup"), Arc::from("menu"));
      let expanded = if self.open.get() { "true" } else { "false" };
      NodeUpdate::describe(&mut *element.node, Arc::from("a11y_expanded"), Arc::from(expanded));
      mark_stops(&mut element.node, &mut next, last, &self.last);
      // The menu hangs from the button, and its tooltip is anchored to it too.
      NodeUpdate::ref_element(&mut *element.node, self.anchor.clone());
      Some(
        Tooltip::new(&bar.more_label)
          .placement(Placement::Bottom)
          .mount(ctx, element),
      )
    } else {
      None
    };
    let view = ToolbarView {
      slots: bar.slots.clone(),
      dropped,
      shown,
      more,
      overflow: bar.overflow,
      secondary: bar.secondary.clone(),
      label: bar.label.clone(),
      recipe,
      refs,
      dropped_signal: self.dropped.clone(),
      open: self.open.clone(),
      anchor: self.anchor.clone(),
      top: bar.top,
      id: bar.id.clone(),
    };
    match theme.renderer::<Toolbar>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

/// A toolbar as a renderer sees it: its slots, the style of its parts, which
/// slots did not fit, and its behavior ready to be dressed.
pub struct ToolbarView {
  /// What the bar holds, in order.
  pub slots: Vec<ToolbarSlot>,
  /// Which slots the last layout found did not fit: they are in the menu.
  pub dropped: Vec<bool>,
  /// Each slot's control as the bar shows it: its stops marked, an icon alone with its tooltip.
  shown: Vec<Element>,
  /// The More button, with its tooltip, for a bar that overflows.
  more: Option<Element>,
  pub overflow: bool,
  /// The commands only in the menu, after what did not fit.
  pub secondary: Vec<ToolbarCommand>,
  pub label: Option<Arc<str>>,
  /// The style of each part of this toolbar.
  pub recipe: ResolvedRecipe,
  refs: Vec<ElementRef>,
  dropped_signal: Signal<Vec<bool>>,
  open: Signal<bool>,
  anchor: ElementRef,
  top: bool,
  id: Option<Arc<str>>,
}

impl ToolbarView {
  /// The style of a part, as this toolbar is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The line between two groups.
  pub fn separator(&self) -> Element {
    Separator::vertical()
      .rules(&self.recipe.rules_for(ToolbarPart::Separator, SeparatorPart::Root))
      .build(self.theme())
  }

  /// The slot at `index` as the bar lays it out: its control, after the line
  /// before it. One that may go into the menu drops out of the bar whole, the
  /// last first, where the bar is too narrow, and says so through its ref.
  pub fn slot(&mut self, index: usize) -> Element {
    let slot = &self.slots[index];
    if slot.spacer {
      let mut spacer = Row::new().part(&self.part(ToolbarPart::Spacer)).flex(1.0);
      if slot.separated {
        spacer = spacer.child(self.separator());
      }
      return spacer.into();
    }
    let element = std::mem::replace(&mut self.shown[index], Element::new());
    // The ring of a focused control is drawn outside it.
    let mut row = Row::new()
      .part(&self.part(ToolbarPart::Slot))
      .align_items(Alignment::Center)
      .overflow_visible();
    if slot.separated {
      row = row.child(self.separator());
    }
    row = row.child(element);
    if self.overflow && !slot.stays {
      let reference = self.refs[index].clone();
      let (dropped, open) = (self.dropped_signal.clone(), self.open.clone());
      let count = self.slots.len();
      let always = !self.secondary.is_empty();
      // A dropped slot is laid out at no size: that is how the bar learns which are in the menu, before the frame is
      // painted, so the first frame at a width has its menu, its lines and its More button as they settle.
      reference.observe_rect_before_paint(move |rect| {
        let now = rect.width <= 0.0 && rect.height <= 0.0;
        if dropped.get_untracked().get(index).copied() != Some(now) {
          dropped.update(|dropped| {
            dropped.resize(count.max(dropped.len()), false);
            dropped[index] = now;
          });
          // With everything back in the bar the More button is gone, and its menu with it.
          if !always && !dropped.get_untracked().contains(&true) {
            put(&open, false);
          }
        }
      });
      row = row
        .flex_shrink(1.0)
        .shrink_limit(ShrinkLimit::Drop)
        .ref_element(reference);
    }
    row.into()
  }

  /// The menu of what did not fit, in the order of the bar: a line where one
  /// stood between them, a labelled group under its label.
  pub fn menu(&self) -> Menu {
    let mut menu = Menu::new(self.open.clone(), self.anchor.clone()).placement(Placement::BottomEnd);
    if let Some(id) = &self.id {
      menu = menu.id(format!("{id}-menu"));
    }
    let mut any = false;
    for (index, slot) in self.slots.iter().enumerate() {
      if !self.dropped[index] || slot.commands.is_empty() {
        continue;
      }
      if any && (slot.separated || slot.label.is_some() || slot.commands.len() > 1) {
        menu = menu.separator();
      }
      if let Some(label) = &slot.label {
        menu = menu.heading(label);
      }
      let shortcut = if slot.commands.len() == 1 {
        slot.shortcut.as_deref()
      } else {
        None
      };
      for command in &slot.commands {
        menu = menu.item(command.item(shortcut));
      }
      any = true;
    }
    if any && !self.secondary.is_empty() {
      menu = menu.separator();
    }
    for command in &self.secondary {
      menu = menu.item(command.item(None));
    }
    menu
  }

  /// The end of the bar while some controls do not fit: a line, the More
  /// button, and the menu it opens. The line drops it whole while all fit,
  /// unless the bar has commands that are only in the menu. No line stands
  /// between the spacer and the button.
  pub fn more(&mut self) -> Option<Element> {
    let button = self.more.take()?;
    let mut row = Row::new()
      .part(&self.part(ToolbarPart::More))
      .align_items(Alignment::Center)
      .overflow_visible()
      .drop_marker(self.secondary.is_empty());
    if !self.slots.last().is_some_and(|slot| slot.spacer) {
      row = row.child(self.separator());
    }
    Some(row.child(button).child(self.menu()).into())
  }

  /// How the default renderer builds a toolbar: the bar, its slots in a
  /// row, and at its end the More button while some do not fit. It is one
  /// stop of Tab, whose arrow keys move between its controls, and a toolbar
  /// to what reads the tree.
  pub fn render(mut self) -> Element {
    let across = if self.top { Alignment::Start } else { Alignment::Center };
    let root = self.part(ToolbarPart::Root);
    let gap = root
      .rest()
      .gap
      .map(|gap| gap.resolve(&self.theme().spacing(), 0.0))
      .unwrap_or(0.0);
    let mut root = Row::new()
      .part(&root)
      .align_items(across)
      .focus_group(FocusGroup::horizontal().one_stop(true))
      .describe("a11y_role", "toolbar")
      .describe("a11y_name", self.label.clone().unwrap_or_default())
      .describe("a11y_orientation", "horizontal");
    // The controls are a line of their own, which is as wide as the bar while what stays in it has room there,
    // and as wide as what stays where it has not: the bar then scrolls sideways to it, and nothing is past it.
    let mut line = Row::new().spacing(gap).align_items(across).overflow_visible();
    for index in 0..self.slots.len() {
      line = line.child(self.slot(index));
    }
    if let Some(more) = self.more() {
      line = line.child(more);
    }
    // With a spacer the line takes all the room the bar is given; without one the bar is as wide as its controls.
    let grows = if self.slots.iter().any(|slot| slot.spacer) {
      1.0
    } else {
      0.0
    };
    root = root.child(
      ScrollHorizontal::new(line)
        .fill_width(true)
        .fit_width(true)
        .wheel_across(true)
        .scrollbar(ScrollBarStyle::hidden())
        .flex_full(grows, 1.0, None)
        .min_width(0.0),
    );
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    root.into()
  }
}

impl Themed for Toolbar {
  const NAME: &'static str = "toolbar";
  type Renderer = ToolbarRenderer;

  /// The design's toolbar (`pencil/lurq-design-system.pen`, Toolbar, Overflow
  /// Menu, Table Toolbar, Command Bar): a bar padded 4 with its controls 2
  /// apart, a line of 20 with 4 on either side between two groups, and ghost
  /// buttons of 32 with icons of 16, so 40 high. On a surface it has the
  /// background's fill, an edge of 1 in the border's color and the middle
  /// radius; floating, the popover's fill, the edge, the large shadow and the
  /// large radius. A group of the command bar is padded 10 with a line of 1
  /// at its end, its label of 10 in the muted foreground 4 under its controls.
  /// A focused control has its ring inside it, where the bar does not cut it off.
  fn recipe() -> Recipe {
    use PaletteColor::{Background, MutedForeground, Popover, PopoverForeground, Ring};
    use ToolbarPart::{Group, GroupItems, GroupLabel, Item, More, Root, Separator, Slot, Stack};
    use ToolbarVariant::{Floating, Surface};
    let edge = || Border::inside(BorderSize::Sm, PaletteColor::Border);
    Recipe::new()
      .base(Root, |s| s.padding_x(4.0).padding_y(4.0).gap(2.0).radius(RadiusSize::Md))
      .variant(Surface, Root, |s| s.background(Background).border(edge()))
      .variant(Floating, Root, |s| {
        s.background(Popover)
          .foreground(PopoverForeground)
          .border(edge())
          .shadow(ShadowStyle::Lg)
          .radius(RadiusSize::Lg)
      })
      // The bar cuts off what does not fit: the ring of a focused control is inside it, where nothing cuts it off.
      .focus_visible(Item, |s| s.outline(Outline::new(BorderSize::Md, Ring).offset(-2.0)))
      .base(Separator, |s| s.padding_x(4.0).height(20.0))
      // The line between two groups stands as far from each as the controls stand apart.
      .base(Slot, |s| s.gap(2.0))
      .base(More, |s| s.gap(2.0))
      .base(Group, |s| s.padding_x(10.0).gap(4.0).end_line(edge()))
      .base(GroupItems, |s| s.gap(4.0))
      .base(Stack, |s| s.gap(2.0))
      .base(GroupLabel, |s| {
        s.typography(TypographyStyle::Caption)
          .font_size(10.0)
          .foreground(MutedForeground)
      })
  }
}
