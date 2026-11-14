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
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  separator::{Separator, SeparatorPart},
  theme::{Themed, current_theme},
  toggle::{Toggle, TogglePart},
  toggle_group::ToggleGroup,
  tooltip::{Tooltip, TooltipSide},
};
use crate::{
  app::{
    component::Component,
    ctx::{Ctx, Placement},
    events::{EventControl, MouseButton, MouseEvent, MouseEventKind, PointerKind},
    theme::{BorderSize, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle},
  },
  components::{Column, Row},
  core::{ElementRef, NodeId, Signal},
  layout::{Alignment, layout_kind::ShrinkLimit},
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
    }
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

  /// What choosing it does.
  pub fn on_select(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.run = Arc::new(handler);
    self
  }

  /// The item of the menu, with `shortcut` beside it.
  fn item(&self, shortcut: Option<&str>) -> MenuItem {
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
  /// The end of the bar while some controls do not fit: the line and the More button.
  More,
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
  More => "more",
});

/// The states of a toolbar a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToolbarState {
  /// Some controls did not fit, and are in the menu at the end.
  Overflowing,
}

names!(ToolbarState {
  Overflowing => "overflowing",
});

/// What builds a toolbar's nodes: a theme's in place of the default one.
pub type ToolbarRenderer = Arc<dyn Fn(ToolbarView) -> Element + Send + Sync>;

/// What a control of a toolbar is.
enum Kind {
  Button(Button),
  Toggle(Toggle),
  ToggleGroup(ToggleGroup),
  /// A select, a field, anything else: it keeps its own keys, and goes into the menu only with what the app
  /// gives to stand for it.
  Element(Element, Option<ToolbarCommand>),
  Separator,
  Spacer,
  Group(ToolbarGroup),
}

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

impl From<ToggleGroup> for ToolbarItem {
  fn from(group: ToggleGroup) -> Self {
    Self::of(Kind::ToggleGroup(group))
  }
}

impl From<super::Select> for ToolbarItem {
  fn from(select: super::Select) -> Self {
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
///   .child(Button::new("Undo").icon(icons::UNDO).icon_only(true))
///   .child(Button::new("Redo").icon(icons::REDO).icon_only(true));
/// let clipboard = ToolbarGroup::new().label("Clipboard").child(Button::new("Paste").icon(icons::PASTE));
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
  pub fn child(mut self, item: impl Into<ToolbarItem>) -> Self {
    self.items.push(item.into());
    self
  }

  pub fn children<I: Into<ToolbarItem>>(mut self, items: impl IntoIterator<Item = I>) -> Self {
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
///   .child(ToolbarItem::from(Toggle::new("Bold").icon(icons::BOLD).icon_only(true).on(bold)).shortcut("Ctrl+B"))
///   .child(Toggle::new("Italic").icon(icons::ITALIC).icon_only(true))
///   .child(Toolbar::separator())
///   .child(Button::new("Link").icon(icons::LINK).icon_only(true))
///   .child(Toolbar::spacer())
///   .child(Button::new("Share"))
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
/// does.
pub struct Toolbar {
  items: Vec<ToolbarItem>,
  variant: Name,
  size: Name,
  overflow: bool,
  label: Option<Arc<str>>,
  rules: Recipe,
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
      rules: Recipe::new(),
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
  pub fn child(mut self, item: impl Into<ToolbarItem>) -> Self {
    self.items.push(item.into());
    self
  }

  pub fn children<I: Into<ToolbarItem>>(mut self, items: impl IntoIterator<Item = I>) -> Self {
    self.items.extend(items.into_iter().map(Into::into));
    self
  }

  /// A [`ToolbarVariant`], or a variant the theme's recipe names.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// The size of the bar's buttons and toggles: a [`ButtonSize`], small by
  /// default. The bar is as high as they are and its padding.
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
  /// bar narrower than its controls cuts them off. For a gallery, a test, a
  /// window of its own; converting a toolbar into an element does this in the
  /// theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let (bar, key) = self.prepare(theme);
    let count = bar.slots.len();
    let view = ToolbarView {
      shown: bar
        .slots
        .iter()
        .map(|slot| match &slot.label {
          Some(label) if !slot.parts.is_empty() => {
            let recipe = resolve(theme, bar.variant, bar.size, &bar.rules, false);
            labelled(
              &recipe,
              label,
              slot.parts.iter().map(|part| part.element.clone()).collect(),
            )
          }
          _ => slot.element.clone(),
        })
        .collect(),
      slots: bar.slots,
      dropped: vec![false; count],
      more: None,
      overflow: false,
      label: bar.label,
      recipe: resolve(theme, bar.variant, bar.size, &bar.rules, false),
      refs: (0..count).map(|_| ElementRef::new()).collect(),
      dropped_signal: Signal::new(Vec::new()),
      open: Signal::new(false),
      anchor: ElementRef::new(),
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
    let recipe = resolve(theme, self.variant, self.size, &self.rules, false);
    let mut slots = Vec::new();
    let mut separated = false;
    let count = self.items.len();
    for (index, item) in self.items.into_iter().enumerate() {
      prepare(item, &recipe, self.size, &mut slots, &mut separated, index + 1 < count);
    }
    let bar = Prepared {
      slots,
      variant: self.variant,
      size: self.size,
      overflow: self.overflow,
      label: self.label,
      rules: self.rules,
      id: self.id,
    };
    (bar, self.key)
  }
}

impl From<Toolbar> for Element {
  fn from(toolbar: Toolbar) -> Self {
    toolbar.build(&current_theme())
  }
}

fn resolve(theme: &Theme, variant: Name, size: Name, rules: &Recipe, overflowing: bool) -> ResolvedRecipe {
  let case = Case::new()
    .variant(variant)
    .size(size)
    .breakpoint(super::current_breakpoint())
    .state(ToolbarState::Overflowing, overflowing);
  theme.resolve::<Toolbar>(&case, Some(rules))
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
  /// The controls of a labelled group, which `element` stands empty for: they are drawn over its label.
  pub parts: Vec<ToolbarSlot>,
}

/// Turns `item` into the slots of the bar, after `slots`: a line that ends a
/// group is held in `separated` for the next.
fn prepare(
  item: ToolbarItem,
  recipe: &ResolvedRecipe,
  size: Name,
  slots: &mut Vec<ToolbarSlot>,
  separated: &mut bool,
  more_follow: bool,
) {
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
    };
  match item.kind {
    Kind::Button(button) => {
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
      let commands = group.commands();
      let group = if item.as_given { group } else { group.size(size) };
      let mut element = group.build(theme);
      // Its toggles are among the bar's controls: the bar's arrow keys move through them and on.
      element.node.focus_group = None;
      let made = slot(separated, element, commands, item.stays.unwrap_or(false), false);
      slots.push(made);
    }
    Kind::Element(element, stand_in) => {
      let stays = item.stays.unwrap_or(stand_in.is_none());
      let made = slot(separated, element, stand_in.into_iter().collect(), stays, false);
      slots.push(made);
    }
    Kind::Separator => *separated = !slots.is_empty(),
    Kind::Spacer => {
      let mut made = slot(separated, Element::new(), Vec::new(), true, false);
      made.spacer = true;
      slots.push(made);
    }
    Kind::Group(group) => {
      // A group stands apart from what is before it and from what follows.
      if !slots.is_empty() {
        *separated = true;
      }
      match group.label {
        None => {
          let count = group.items.len();
          for (index, inner) in group.items.into_iter().enumerate() {
            prepare(inner, recipe, size, slots, separated, index + 1 < count);
          }
        }
        Some(label) => {
          let mut inner_slots = Vec::new();
          let mut inner_separated = false;
          let count = group.items.len();
          for (index, inner) in group.items.into_iter().enumerate() {
            prepare(
              inner,
              recipe,
              size,
              &mut inner_slots,
              &mut inner_separated,
              index + 1 < count,
            );
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
      if more_follow {
        *separated = true;
      }
    }
  }
}

/// A labelled group of the bar: its controls in a row over its label.
fn labelled(recipe: &ResolvedRecipe, label: &str, controls: Vec<Element>) -> Element {
  let part = recipe.part(ToolbarPart::GroupLabel);
  let mut row = Row::new()
    .part(&recipe.part(ToolbarPart::GroupItems))
    .align_items(Alignment::Center)
    .overflow_visible();
  for control in controls {
    row = row.child(control);
  }
  Column::new()
    .part(&recipe.part(ToolbarPart::Group))
    .align_items(Alignment::Center)
    .overflow_visible()
    .child(row)
    .child(part.text(label).nowrap().part(&part))
    .into()
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
  rules: Recipe,
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
    let ToolbarProps { bar } = ctx.props::<ToolbarProps>().clone();
    let theme = current_theme();
    let count = bar.slots.len();
    let dropped: Vec<bool> = {
      let read = self.dropped.get();
      (0..count)
        .map(|index| bar.overflow && read.get(index).copied().unwrap_or(false) && !bar.slots[index].stays)
        .collect()
    };
    let overflowing = dropped.iter().any(|dropped| *dropped);
    let recipe = resolve(&theme, bar.variant, bar.size, &bar.rules, overflowing);
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
          let mut tooltip = Tooltip::new(&command.label).side(TooltipSide::Bottom);
          if let Some(keys) = &slot.shortcut {
            tooltip = tooltip.shortcut(keys);
          }
          element = tooltip.mount(ctx, element);
        }
        element
      };
      let element = match &slot.label {
        Some(label) if !slot.parts.is_empty() => {
          let parts = slot.parts.iter().map(&mut show).collect();
          labelled(&recipe, label, parts)
        }
        _ => show(slot),
      };
      shown.push(element);
    }
    let more = if bar.overflow {
      let mut button = Button::new("More")
        .icon(icons::MORE_HORIZONTAL)
        .icon_only(true)
        .variant(ButtonVariant::Ghost)
        .size(bar.size)
        .rules(&recipe.rules_for(ToolbarPart::Item, ButtonPart::Root))
        .on_click({
          let open = self.open.clone();
          move |_| open.update(|open| *open = !*open)
        });
      if let Some(id) = &bar.id {
        button = button.id(format!("{id}-more"));
      }
      let mut element = button.build(&theme);
      NodeUpdate::ref_element(&mut *element.node, self.anchor.clone());
      NodeUpdate::describe(&mut *element.node, Arc::from("a11y_haspopup"), Arc::from("menu"));
      let expanded = if self.open.get() { "true" } else { "false" };
      NodeUpdate::describe(&mut *element.node, Arc::from("a11y_expanded"), Arc::from(expanded));
      mark_stops(&mut element.node, &mut next, last, &self.last);
      Some(Tooltip::new("More").side(TooltipSide::Bottom).mount(ctx, element))
    } else {
      None
    };
    let view = ToolbarView {
      slots: bar.slots.clone(),
      dropped,
      shown,
      more,
      overflow: bar.overflow,
      label: bar.label.clone(),
      recipe,
      refs,
      dropped_signal: self.dropped.clone(),
      open: self.open.clone(),
      anchor: self.anchor.clone(),
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
  pub label: Option<Arc<str>>,
  /// The style of each part of this toolbar.
  pub recipe: ResolvedRecipe,
  refs: Vec<ElementRef>,
  dropped_signal: Signal<Vec<bool>>,
  open: Signal<bool>,
  anchor: ElementRef,
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
      // A dropped slot is laid out at no size: that is how the bar learns which are in the menu.
      reference.observe_rect(move |rect| {
        let now = rect.width <= 0.0 && rect.height <= 0.0;
        if dropped.get_untracked().get(index).copied() != Some(now) {
          dropped.update(|dropped| {
            dropped.resize(count.max(dropped.len()), false);
            dropped[index] = now;
          });
          // With everything back in the bar the More button is gone, and its menu with it.
          if !dropped.get_untracked().contains(&true) {
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
    let mut menu = Menu::new(self.anchor.clone(), self.open.clone()).placement(Placement::BottomEnd);
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
        menu = menu.label(label);
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
    menu
  }

  /// The end of the bar while some controls do not fit: a line, the More
  /// button, and the menu it opens. The line drops it whole while all fit.
  pub fn more(&mut self) -> Option<Element> {
    let button = self.more.take()?;
    Some(
      Row::new()
        .part(&self.part(ToolbarPart::More))
        .align_items(Alignment::Center)
        .overflow_visible()
        .drop_marker(true)
        .child(self.separator())
        .child(button)
        .child(self.menu())
        .into(),
    )
  }

  /// How the default renderer builds a toolbar: the bar, its slots in a
  /// row, and at its end the More button while some do not fit. It is one
  /// stop of Tab, whose arrow keys move between its controls, and a toolbar
  /// to what reads the tree.
  pub fn render(mut self) -> Element {
    let mut root = Row::new()
      .part(&self.part(ToolbarPart::Root))
      .align_items(Alignment::Center)
      .focus_group(FocusGroup::horizontal().one_stop(true))
      .describe("a11y_role", "toolbar")
      .describe("a11y_name", self.label.clone().unwrap_or_default())
      .describe("a11y_orientation", "horizontal");
    for index in 0..self.slots.len() {
      root = root.child(self.slot(index));
    }
    if let Some(more) = self.more() {
      root = root.child(more);
    }
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
    use ToolbarPart::{Group, GroupItems, GroupLabel, Item, More, Root, Separator, Slot};
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
      .base(GroupLabel, |s| {
        s.typography(TypographyStyle::Caption)
          .font_size(10.0)
          .foreground(MutedForeground)
      })
  }
}
