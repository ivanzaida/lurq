//! Window tabs: the tabs of a desktop app's title bar, as a browser or an editor has them.

use std::{
  cell::OnceCell,
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
};

use parking_lot::Mutex;

use super::{
  icons, names,
  part_style::PartAnimation,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  select::{px, spacing},
  shared::{FADE_IN, FADE_OUT, fading, glyph, one_line, pressable, tab_stop},
  theme::{Themed, current_theme},
  tooltip::Tooltip,
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{DragEvent, KeyboardEvent, MouseButton, MouseButtonMask, MouseEvent},
    theme::{
      BorderSize, IconSize, MotionDuration, MotionEasing, OpacityLevel, PaletteColor, RadiusSize, ShadowStyle, Theme,
      TypographyStyle,
    },
  },
  components::{self, Column, Row, ScrollBoth, Stack},
  core::{ElementRef, Signal, SignalValue},
  layout::{
    Alignment,
    layout_kind::{Justify, ScrollState},
    scrollbar::ScrollBarStyle,
    text_style::FontWeight,
  },
  node::{CursorIcon, Element, FocusGroup, color::Color, dimension::Dimension, outline::Outline},
};

mod all_tabs;
mod list;

/// How far a press on a tab moves before it is a drag: the distance past which the engine clicks nothing on the
/// release either.
const DRAG_START: f32 = 4.0;

/// The parts of window tabs a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindowTabsPart {
  /// The strip: the tabs and the add button.
  Root,
  /// The row of the tabs, which scrolls sideways where they do not fit: what reads the tree sees a tab list.
  List,
  /// One tab: what is pressed, focused and dragged.
  Tab,
  /// The icon of a tab, before its label.
  Icon,
  /// The text of a tab.
  Label,
  /// The label of a tab with what stands right after it: the dot of unsaved changes where it is after the label,
  /// and the app's marks. It takes the room the icon and the close slot leave.
  Title,
  /// The app's elements after a tab's label ([`WindowTab::trailing`]).
  Marks,
  /// The slot at the end of a tab: its close button, or the dot of a tab with unsaved changes.
  Close,
  /// The x in the close slot.
  CloseIcon,
  /// The dot in the close slot of a tab with unsaved changes.
  Dot,
  /// The line between two tabs that are not the selected one, and before the add button.
  Separator,
  /// The button at the end that asks for a new tab.
  Add,
  /// The plus of the add button.
  AddIcon,
  /// Of a strip whose tabs leave it where they do not fit ([`WindowTabsOverflow::List`]): the button at the end
  /// of its tabs that says how many left and opens the menu of them.
  More,
  /// How many tabs the More button holds.
  MoreLabel,
  /// The chevron of the More button: down, and up while its menu is open.
  MoreIcon,
  /// Of the list of every tab ([`WindowTabsMenu::All`]): the panel under the More button.
  Panel,
  /// The search field at the head of the list, where it holds more than eight tabs.
  Search,
  /// The rows of the list, under the search field.
  Items,
  /// One row of the list: a tab, which a press selects.
  Item,
  /// The icon of a row, before its label.
  ItemIcon,
  /// The label of a row.
  ItemLabel,
  /// The quiet text after a row's label ([`WindowTab::list_detail`]), which takes the room the label leaves.
  ItemDetail,
  /// The app's elements in a row ([`WindowTab::list_trailing`]), after the mark of a tab not in the strip.
  ItemMarks,
  /// The mark of a row whose tab is not in the strip.
  ItemHidden,
  /// The mark of the row of the selected tab.
  ItemCurrent,
  /// The close button at a row's end.
  ItemClose,
  /// The x of a row's close button.
  ItemCloseIcon,
  /// What the list says where no tab matches the search.
  Empty,
}

names!(WindowTabsPart {
  Root => "root",
  List => "list",
  Tab => "tab",
  Icon => "icon",
  Label => "label",
  Title => "title",
  Marks => "marks",
  Close => "close",
  CloseIcon => "close_icon",
  Dot => "dot",
  Separator => "separator",
  Add => "add",
  AddIcon => "add_icon",
  More => "more",
  MoreLabel => "more_label",
  MoreIcon => "more_icon",
  Panel => "panel",
  Search => "search",
  Items => "items",
  Item => "item",
  ItemIcon => "item_icon",
  ItemLabel => "item_label",
  ItemDetail => "item_detail",
  ItemMarks => "item_marks",
  ItemHidden => "item_hidden",
  ItemCurrent => "item_current",
  ItemClose => "item_close",
  ItemCloseIcon => "item_close_icon",
  Empty => "empty",
});

/// The states of window tabs a recipe styles, besides what the pointer and the keyboard do to them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindowTabsState {
  /// Of a tab: it is the selected one, the tab of what the window shows.
  Selected,
  /// Of a tab: what it holds has changes that are not saved; its close slot shows a dot.
  Dirty,
  /// Of a tab: it is being dragged along the strip.
  Dragged,
  /// Of a tab: it cannot be selected, closed or dragged. Of the strip: nothing in it can.
  Disabled,
  /// Of a tab: it shows its icon alone ([`WindowTab::icon_only`]).
  IconOnly,
  /// Of the strip: some of its tabs did not fit and are in the menu of its More button.
  Overflowing,
  /// Of the strip: the menu of its More button is open.
  MenuOpen,
  /// Of the strip: its list of every tab leads with its search field, holding more than eight tabs.
  Searchable,
}

names!(WindowTabsState {
  Selected => "selected",
  Dirty => "dirty",
  Dragged => "dragged",
  Disabled => "disabled",
  IconOnly => "icon_only",
  Overflowing => "overflowing",
  MenuOpen => "menu_open",
  Searchable => "searchable",
});

/// What window tabs do where they have less room than their tabs at their most: what
/// [`WindowTabs::overflow`] takes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WindowTabsOverflow {
  /// The tabs share the room down to their least width, and past that the strip scrolls sideways. The default.
  #[default]
  Scroll,
  /// The tabs keep their width, and those that do not fit leave the strip, the last first and never the selected
  /// one, into the menu of a button at the end of the tabs that shows how many left. Choosing one there selects it.
  List,
}

/// What the More button of the list mode opens: what [`WindowTabs::menu`] takes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WindowTabsMenu {
  /// A menu of the tabs that left the strip, in their order. The default.
  #[default]
  Hidden,
  /// A list of every open tab, in the strip's order: a search field at its head once it holds more than eight, a
  /// mark on the selected tab and on each tab not in the strip, and a close button on each row.
  All,
}

/// Where the dot of a tab with unsaved changes stands: what [`WindowTabs::dirty_mark`] takes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WindowTabsDirtyMark {
  /// In the close slot, the x in its place while the pointer is on the tab. The default.
  #[default]
  InCloseSlot,
  /// Right after the label, the close button in its own slot and shown as a clean tab's: both at once.
  AfterLabel,
}

/// What builds window tabs' nodes: a theme's in place of the default one.
pub type WindowTabsRenderer = Arc<dyn Fn(WindowTabsView) -> Element + Send + Sync>;

/// What is told a value of the app's type: the tab selected, closed or detached.
type Handler<T> = Arc<dyn Fn(&T) + Send + Sync>;

/// What is told nothing but that it happened.
type Heard = Arc<dyn Fn() + Send + Sync>;

/// What is told a tab moved: from where, and to where.
type Moved = Arc<dyn Fn(usize, usize) + Send + Sync>;

/// What sets the signal of bound tabs to a value, and says whether it was another.
type Write<T> = Arc<dyn Fn(&T) -> bool + Send + Sync>;

/// What is told the More button's menu opened or closed.
type OpenChange = Arc<dyn Fn(bool) + Send + Sync>;

/// What says the More button's tooltip for how many tabs are hidden.
type CountText = Arc<dyn Fn(usize) -> String + Send + Sync>;

/// How far the list of every tab stays inside the window's edges unless told otherwise.
const LIST_INSET: f32 = 8.0;

/// The list's settings of the app's: whether it is open, who hears it open, by what it stands, and the More
/// button's tooltip.
#[derive(Clone, Default)]
struct ListSettings {
  open: Option<Signal<bool>>,
  on_open_change: Vec<OpenChange>,
  clear_of: Option<ElementRef>,
  inset: Option<f32>,
  more_tooltip: Option<CountText>,
}

/// One tab of a window: its value, what it shows, and whether it can be closed.
///
/// ```
/// use lurq::ui::{WindowTab, icons};
///
/// let plain: WindowTab<u8> = (0, "README.md").into();
/// let full = WindowTab::new("blur", "blur.rs")
///   .icon(icons::FILE_CODE)
///   .dirty(true)
///   .on_close(|| println!("closed"));
/// ```
pub struct WindowTab<T> {
  value: T,
  label: Arc<str>,
  icon: Option<Name>,
  dirty: bool,
  close_button: bool,
  closable: bool,
  on_close: Vec<Heard>,
  disabled: bool,
  icon_only: bool,
  width: Option<f32>,
  max_width: Option<f32>,
  fit_content: Option<f32>,
  trailing: Vec<Element>,
  list_label: Option<Arc<str>>,
  list_detail: Option<Arc<str>>,
  list_trailing: Vec<Element>,
  descriptions: Vec<Description>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

/// What a tab says of itself to tooling, under a name.
type Description = (Arc<str>, Arc<str>);

impl<T> WindowTab<T> {
  /// A tab of `value`, called `label`.
  pub fn new(value: T, label: &str) -> Self {
    Self {
      value,
      label: Arc::from(label),
      icon: None,
      dirty: false,
      close_button: true,
      closable: true,
      on_close: Vec::new(),
      disabled: false,
      icon_only: false,
      width: None,
      max_width: None,
      fit_content: None,
      trailing: Vec::new(),
      list_label: None,
      list_detail: None,
      list_trailing: Vec::new(),
      descriptions: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// An icon before the label, by its name in the theme's set.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// Whether what the tab holds has changes that are not saved: a dot, in its close slot with the x in its place
  /// while the pointer is on the tab, or after its label where the strip says so
  /// ([`dirty_mark`](WindowTabs::dirty_mark)).
  pub fn dirty(mut self, dirty: bool) -> Self {
    self.dirty = dirty;
    self
  }

  /// Whether the tab shows a close button: shown unless told otherwise. A tab without one is still closed by a
  /// person in the other ways (the middle button, Delete, the window's shortcut) where it is
  /// [`closable`](Self::closable).
  pub fn close_button(mut self, shown: bool) -> Self {
    self.close_button = shown;
    self
  }

  /// Whether a person may close the tab: they may unless told otherwise. A tab that may not (a home tab) is closed
  /// by nothing a person does, and shows no close button.
  pub fn closable(mut self, closable: bool) -> Self {
    self.closable = closable;
    self
  }

  /// Called when a person closes this tab: its close button pressed, a press of the middle button on it, Delete on
  /// it, or the window's shortcut while it is the selected one. Before the strip's
  /// [`on_close`](WindowTabs::on_close); a second handler is called after the first. The app removes the tab.
  pub fn on_close(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_close.push(Arc::new(handler));
    self
  }

  /// Whether the tab cannot be selected, closed or dragged: it is dimmed, and the keys pass it by.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Whether the tab shows its icon alone, as a home tab does: no label, no close slot, and the width of the
  /// recipe's tab of an icon alone (28). The label is still its name to what reads the tree.
  pub fn icon_only(mut self, icon_only: bool) -> Self {
    self.icon_only = icon_only;
    self
  }

  /// The width of this tab, in place of what the recipe and the room give it: it neither grows nor gives way.
  pub fn width(mut self, width: f32) -> Self {
    self.width = Some(width);
    self
  }

  /// The most this tab is wide, under the recipe's most: a cap of the app's on a long label, which ends in an
  /// ellipsis there.
  pub fn max_width(mut self, width: f32) -> Self {
    self.max_width = Some(width);
    self
  }

  /// Makes the tab as wide as what it shows (its icon, its label, its marks and its close slot) up to `max`, where
  /// its label ends in an ellipsis: in place of the recipe's width, and with no measuring of the app's. It is never
  /// narrower than the recipe's least width. In the list mode it keeps that width and leaves the strip whole; in the
  /// scroll mode it gives way down to its least width where the tabs do not fit, and the strip, which cannot know
  /// the width of a label before it is laid out, is reckoned with it at `max`. A [`width`](Self::width) of its own
  /// wins, and a [`max_width`](Self::max_width) caps it too.
  pub fn fit_content(mut self, max: f32) -> Self {
    self.fit_content = Some(max);
    self
  }

  /// An element of the app's after the label (a count, a mark that something waits, with its tooltip), in the
  /// tab's `marks` part; a second one after the first. It keeps its width, and the label gives way first.
  pub fn trailing(mut self, element: impl Into<Element>) -> Self {
    self.trailing.push(element.into());
    self
  }

  /// What the tab's row in the list of every tab ([`WindowTabsMenu::All`]) says in place of its label, and is called
  /// by to what reads the tree: the page, where the tab says its project and its page. A search matches it as it
  /// matches the label.
  pub fn list_label(mut self, label: &str) -> Self {
    self.list_label = Some(Arc::from(label));
    self
  }

  /// A quiet text after the label of the tab's row in the list of every tab, in its `item_detail` part, which takes
  /// the room the label leaves and ends in an ellipsis where it is not enough: the project of a page. A search
  /// matches it too.
  pub fn list_detail(mut self, detail: &str) -> Self {
    self.list_detail = Some(Arc::from(detail));
    self
  }

  /// An element of the app's in the tab's row in the list of every tab, after the mark of a tab not in the strip and
  /// before the check of the selected one, in the row's `item_marks` part; a second one after the first. It is an
  /// element of its own, apart from the tab's [`trailing`](Self::trailing), and its id should be another.
  pub fn list_trailing(mut self, element: impl Into<Element>) -> Self {
    self.list_trailing.push(element.into());
    self
  }

  /// Says `value` of the tab under `name` to tooling that reads the tree, the DevTools and the MCP tree, as
  /// [`Styled::describe`](super::Styled::describe) does of a component: the state and the route an app's tests and
  /// an agent read of it. It is said on the tab's node before what the strip says of the tab, so a reader that
  /// takes the first value of a name takes the app's. It says nothing unless a tooling feature (`devtools`, `mcp`)
  /// is on.
  pub fn describe(mut self, name: impl Into<Arc<str>>, value: impl Into<Arc<str>>) -> Self {
    self.descriptions.push((name.into(), value.into()));
    self
  }

  /// The id of the tab: what is pressed. Its close button has this id with `-close` after it, and the parts in it
  /// are named after it: `{id}-title`, `{id}-marks`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// The key of the tab, among the others.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }
}

impl<T> WindowTab<T> {
  /// The tab with its value made another: what an optional strip holds of a tab of `T`.
  fn map<U>(self, change: impl FnOnce(T) -> U) -> WindowTab<U> {
    WindowTab {
      value: change(self.value),
      label: self.label,
      icon: self.icon,
      dirty: self.dirty,
      close_button: self.close_button,
      closable: self.closable,
      on_close: self.on_close,
      disabled: self.disabled,
      icon_only: self.icon_only,
      width: self.width,
      max_width: self.max_width,
      fit_content: self.fit_content,
      trailing: self.trailing,
      list_label: self.list_label,
      list_detail: self.list_detail,
      list_trailing: self.list_trailing,
      descriptions: self.descriptions,
      id: self.id,
      key: self.key,
    }
  }
}

impl<T> From<(T, &str)> for WindowTab<T> {
  fn from((value, label): (T, &str)) -> Self {
    Self::new(value, label)
  }
}

/// A tab of `T` in a strip of [`optional`](WindowTabs::optional) tabs, whose value is `Some` of it.
impl<T> From<WindowTab<T>> for WindowTab<Option<T>> {
  fn from(tab: WindowTab<T>) -> Self {
    tab.map(Some)
  }
}

/// One tab as it is built: what it shows, whether it is the selected one, and what a press of it, its close and a
/// drag out of the strip do.
#[derive(Clone)]
pub struct WindowTabEntry {
  pub label: Arc<str>,
  pub icon: Option<Name>,
  pub dirty: bool,
  pub close_button: bool,
  pub closable: bool,
  pub disabled: bool,
  pub icon_only: bool,
  /// The tab's own width and most width, over the recipe's.
  pub width: Option<f32>,
  pub max_width: Option<f32>,
  /// The most a tab as wide as what it shows is ([`WindowTab::fit_content`]).
  pub fit_content: Option<f32>,
  /// The app's elements after the label.
  pub trailing: Vec<Element>,
  /// What the tab's row in the list of every tab says in place of its label, the quiet text after it, and the app's
  /// elements in it.
  pub list_label: Option<Arc<str>>,
  pub list_detail: Option<Arc<str>>,
  pub list_trailing: Vec<Element>,
  /// What the app says of the tab to tooling, said before what the strip says.
  pub descriptions: Vec<(Arc<str>, Arc<str>)>,
  pub selected: bool,
  select: Heard,
  close: Heard,
  detach: Heard,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

/// The words lurq writes itself.
#[derive(Clone)]
struct Words {
  add: Arc<str>,
  close: Arc<str>,
  more: Arc<str>,
}

/// Window tabs: the tabs of a desktop app's title bar, as a browser or an
/// editor has them. One is selected, the tab of what the window shows; a
/// person closes a tab, asks for a new one, drags one to another place
/// along the strip or out of it.
///
/// ```
/// use lurq::{
///   core::Signal,
///   ui::{WindowTab, WindowTabs, icons},
/// };
///
/// # fn build(open: Signal<u32>) {
/// let tabs = WindowTabs::new(
///   open,
///   [
///     WindowTab::new(1, "blur.rs").icon(icons::FILE_CODE),
///     WindowTab::new(2, "theme.rs").icon(icons::FILE_CODE).dirty(true),
///     WindowTab::new(3, "canvas.rs").icon(icons::FILE_CODE),
///   ],
/// )
/// .label("Open files")
/// .on_close(|file| println!("close {file}"))
/// .on_add(|| println!("a new file"))
/// .on_reorder(|from, to| println!("move {from} to {to}"))
/// .on_detach(|file| println!("open {file} in a window of its own"));
/// // The app holds which is selected, and is told the tab a person selects.
/// let held = WindowTabs::controlled(1, [(1, "blur.rs"), (2, "theme.rs")], |file| println!("{file}"));
/// # }
/// ```
///
/// The strip is as wide as its tabs and its add button. Where it is given
/// less room, its tabs share it down to their least width, and then the
/// strip scrolls sideways. A press on a tab, its close or the add button
/// goes no further than them, so a title bar that drags the window from a
/// press drags it from the strip's empty room alone. The tabs are one stop
/// of Tab, whose Left and Right select; Ctrl+Tab and Ctrl+Shift+Tab select
/// from anywhere in the window, Ctrl+W (Cmd+W on macOS) closes the selected
/// tab, and Ctrl+Shift+PageDown and PageUp move it.
pub struct WindowTabs<T = ()> {
  tabs: Vec<WindowTab<T>>,
  selected: Option<T>,
  write: Option<Write<T>>,
  on_change: Vec<Handler<T>>,
  on_close: Vec<Handler<T>>,
  on_add: Vec<Heard>,
  add_reason: Option<Arc<str>>,
  on_reorder: Vec<Moved>,
  on_detach: Vec<Handler<T>>,
  shortcuts: bool,
  disabled: bool,
  overflow: WindowTabsOverflow,
  menu: WindowTabsMenu,
  list_shortcut: bool,
  list: ListSettings,
  dirty_mark: WindowTabsDirtyMark,
  tab_stop: bool,
  label: Option<Arc<str>>,
  words: Words,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

/// What sets `value` to a value, and says whether it was another.
fn writes<T: Clone + PartialEq + SignalValue + Send + Sync + 'static>(value: Signal<T>) -> Write<T> {
  Arc::new(move |next: &T| {
    let other = value.get_untracked() != *next;
    if other {
      value.set(next.clone());
    }
    other
  })
}

impl<T: Clone + PartialEq + Send + Sync + SignalValue + 'static> WindowTabs<T> {
  /// Tabs whose selected tab is the one with the signal's value. Selecting another sets the signal.
  pub fn new(value: Signal<T>, tabs: impl IntoIterator<Item = impl Into<WindowTab<T>>>) -> Self {
    let now = value.get();
    Self::of(now, tabs.into_iter().map(Into::into).collect(), Some(writes(value)))
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> WindowTabs<Option<T>> {
  /// Tabs bound to `value`, which may hold no tab's value: with `None` no tab is selected (a window that shows
  /// nothing of its tabs: a start page). Selecting one sets the signal to `Some` of its value, and `on_change` is
  /// told `&Option<T>`.
  pub fn optional(value: Signal<Option<T>>, tabs: impl IntoIterator<Item = impl Into<WindowTab<T>>>) -> Self
  where
    Option<T>: SignalValue,
  {
    let now = value.get();
    let tabs = tabs.into_iter().map(|tab| tab.into().map(Some)).collect();
    Self::of(now, tabs, Some(writes(value)))
  }

  /// Tabs whose state the app holds, which may be no tab: the tab of `value` is the selected one, none with
  /// `None`, and `on_change` is told `Some` of the value of each other one a person selects.
  pub fn controlled_optional(
    value: Option<T>,
    tabs: impl IntoIterator<Item = impl Into<WindowTab<T>>>,
    on_change: impl Fn(&Option<T>) + Send + Sync + 'static,
  ) -> Self {
    let tabs = tabs.into_iter().map(|tab| tab.into().map(Some)).collect();
    Self::of(value, tabs, None).on_change(on_change)
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> WindowTabs<T> {
  fn of(selected: T, tabs: Vec<WindowTab<T>>, write: Option<Write<T>>) -> Self {
    Self {
      tabs,
      selected: Some(selected),
      write,
      on_change: Vec::new(),
      on_close: Vec::new(),
      on_add: Vec::new(),
      add_reason: None,
      on_reorder: Vec::new(),
      on_detach: Vec::new(),
      shortcuts: true,
      disabled: false,
      overflow: WindowTabsOverflow::Scroll,
      menu: WindowTabsMenu::Hidden,
      list_shortcut: false,
      list: ListSettings::default(),
      dirty_mark: WindowTabsDirtyMark::InCloseSlot,
      tab_stop: true,
      label: None,
      words: Words {
        add: super::current_locale().navigation.new_tab.clone(),
        close: super::current_locale().common.close.clone(),
        more: super::current_locale().navigation.show_hidden_tabs.clone(),
      },
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// Tabs whose state the app holds: the tab of `value` is the selected one, and `on_change` is told the value of
  /// each other one a person selects. The tabs select nothing themselves.
  pub fn controlled(
    value: T,
    tabs: impl IntoIterator<Item = impl Into<WindowTab<T>>>,
    on_change: impl Fn(&T) + Send + Sync + 'static,
  ) -> Self {
    Self::of(value, tabs.into_iter().map(Into::into).collect(), None).on_change(on_change)
  }

  /// A tab after the others.
  pub fn tab(mut self, tab: impl Into<WindowTab<T>>) -> Self {
    self.tabs.push(tab.into());
    self
  }

  /// Tabs after the others.
  pub fn tabs(mut self, tabs: impl IntoIterator<Item = impl Into<WindowTab<T>>>) -> Self {
    self.tabs.extend(tabs.into_iter().map(Into::into));
    self
  }

  /// Called when a person selects a tab other than the selected one, by a press, a drag, the arrow keys or the
  /// window's shortcuts, with its value: after the signal is set where the tabs are bound. A value the app sets is
  /// told to none of them, and so is a press of the selected tab. A second handler is called after the first.
  pub fn on_change(mut self, handler: impl Fn(&T) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Called with the value of a tab a person closes: its close button pressed, a press of the middle button on it,
  /// Delete on it, or Ctrl+W (Cmd+W on macOS) while it is the selected one. After the tab's own
  /// [`on_close`](WindowTab::on_close); the app removes the tab, and selects another where it was the selected
  /// one. A second handler is called after the first.
  pub fn on_close(mut self, handler: impl Fn(&T) + Send + Sync + 'static) -> Self {
    self.on_close.push(Arc::new(handler));
    self
  }

  /// Called when a person presses the add button, which is shown once this is given: the app adds a tab, and
  /// selects it where it should be. A second handler is called after the first.
  pub fn on_add(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_add.push(Arc::new(handler));
    self
  }

  /// The add button cannot be pressed now, for `reason`: it is shown, with or without [`on_add`](Self::on_add),
  /// in the `Disabled` state of its part (faded), a press of it does nothing, and its tooltip says the reason, which
  /// tooling reads as what describes it. An empty reason makes it available again.
  ///
  /// ```
  /// use lurq::ui::WindowTabs;
  ///
  /// let tabs = WindowTabs::new().on_add(|| {}).add_unavailable("A project is opening");
  /// ```
  pub fn add_unavailable(mut self, reason: &str) -> Self {
    self.add_reason = (!reason.is_empty()).then(|| Arc::from(reason));
    self
  }

  /// Called when a person moves a tab: `from` its place, `to` the place it is to have once it is taken out of the
  /// list and put back (`tabs.insert(to, tabs.remove(from))`), by a drag along the strip or by Ctrl+Shift+PageDown
  /// and PageUp. A drag that Escape ends tells nothing. The app reorders its list. A second handler is called after
  /// the first.
  pub fn on_reorder(mut self, handler: impl Fn(usize, usize) + Send + Sync + 'static) -> Self {
    self.on_reorder.push(Arc::new(handler));
    self
  }

  /// Called with the value of a tab a person dragged out of the strip and let go away from it: more than the
  /// strip's height above or below it, or past the window's top or start edge. The app takes the tab out of the
  /// list, to open what it holds in a window of its own. A second handler is called after the first.
  pub fn on_detach(mut self, handler: impl Fn(&T) + Send + Sync + 'static) -> Self {
    self.on_detach.push(Arc::new(handler));
    self
  }

  /// Whether the window's shortcuts work, from anywhere in the window: Ctrl+Tab and Ctrl+Shift+Tab (and on Windows
  /// and Linux Ctrl+PageDown and Ctrl+PageUp) select the next tab and the one before, round at the ends; Ctrl+W
  /// (Cmd+W on macOS) closes the selected tab; Ctrl+Shift+PageDown and PageUp move it. On unless told otherwise.
  pub fn shortcuts(mut self, on: bool) -> Self {
    self.shortcuts = on;
    self
  }

  /// Whether no tab can be selected, closed or dragged, and no new one asked for.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// What the tabs do where the strip has less room than they need at their most: share it and then scroll
  /// ([`WindowTabsOverflow::Scroll`], the default), or keep their width and leave the strip into the menu of a
  /// button at its end ([`WindowTabsOverflow::List`]). The list needs the tabs [`mount`](Self::mount)ed, which
  /// learns which left; converted into an element on their own they are cut at the strip's end.
  pub fn overflow(mut self, overflow: WindowTabsOverflow) -> Self {
    self.overflow = overflow;
    self
  }

  /// What the More button of the list mode opens: a menu of the tabs that left the strip
  /// ([`WindowTabsMenu::Hidden`], the default), or the list of every open tab ([`WindowTabsMenu::All`]), with a
  /// search field once it holds more than eight, a mark on the selected tab and on each tab not in the strip, and a
  /// close button on each row. Ctrl+W (Cmd+W on macOS) on a row of it closes that row's tab.
  pub fn menu(mut self, menu: WindowTabsMenu) -> Self {
    self.menu = menu;
    self
  }

  /// Whether Ctrl+Shift+A (Cmd+Shift+A on macOS) opens and closes the More button's menu of the list mode, from
  /// anywhere in the window, apart from the window's other [`shortcuts`](Self::shortcuts): off unless told. The
  /// menu of the hidden tabs opens only while a tab is hidden; the list of every tab opens at any time, hanging from
  /// the tabs, its end at theirs, while none is hidden and the More button is not laid out. The More button's tooltip
  /// says the keys.
  pub fn list_shortcut(mut self, on: bool) -> Self {
    self.list_shortcut = on;
    self
  }

  /// Whether the More button's menu of the list mode is open: the app's signal, in place of one the strip keeps. The
  /// strip sets it as the menu opens and closes, and the app may set it to open or close the menu: the list of every
  /// tab opens whether a tab is hidden or not, as by the [`list_shortcut`](Self::list_shortcut). The list of every
  /// tab takes its keys while it has the focus (`keys_first`): Ctrl+W (Cmd+W), Enter, Escape, the arrows and every key
  /// without Ctrl, Alt or Cmd are not also heard by the page under it.
  pub fn menu_open(mut self, open: Signal<bool>) -> Self {
    self.list.open = Some(open);
    self
  }

  /// Called when a person opens or closes the More button's menu, with whether it is open now: the button, the list
  /// shortcut, a choice in it, Escape or a press outside. Not when the app sets [`menu_open`](Self::menu_open), nor
  /// when the menu closes as the last hidden tab comes back into the strip. A second handler is called after the
  /// first.
  pub fn on_menu_open_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.list.on_open_change.push(Arc::new(handler));
    self
  }

  /// Makes the list of every tab stand clear of the box of `element` (the title bar the strip is in) on the side it
  /// opens on, in place of the More button's: 4 under its bottom edge, lined up with the button along it.
  pub fn list_clear_of(mut self, element: impl Into<ElementRef>) -> Self {
    self.list.clear_of = Some(element.into());
    self
  }

  /// How far the list of every tab stays inside the window's edges: 8 unless told otherwise. It is moved in from the
  /// end of a window too narrow for it, and is never lower than that above the window's bottom, its rows scrolling.
  pub fn list_inset(mut self, inset: f32) -> Self {
    self.list.inset = Some(inset);
    self
  }

  /// What the More button's tooltip says for how many tabs left the strip, and what the button says of itself to what
  /// reads the tree: "3 hidden tabs" in the locale's words unless told otherwise.
  pub fn more_tooltip(mut self, text: impl Fn(usize) -> String + Send + Sync + 'static) -> Self {
    self.list.more_tooltip = Some(Arc::new(text));
    self
  }

  /// Where the dot of a tab with unsaved changes stands: in the close slot, the x in its place under the pointer
  /// ([`WindowTabsDirtyMark::InCloseSlot`], the default), or right after the label, beside a close button that
  /// stays ([`WindowTabsDirtyMark::AfterLabel`]).
  pub fn dirty_mark(mut self, mark: WindowTabsDirtyMark) -> Self {
    self.dirty_mark = mark;
    self
  }

  /// Whether Tab stops in the strip; it does unless told otherwise. `false` is a strip the pointer and the app's
  /// own shortcuts reach: Tab passes its tabs, its add button and its More button by, and a press of one leaves the
  /// focus where it was. A tab the app focuses still takes Left, Right and Delete, and the window's keys work as
  /// before; what reads the tree sees the same tabs.
  pub fn tab_stop(mut self, stops: bool) -> Self {
    self.tab_stop = stops;
    self
  }

  /// What the list of tabs is called, to what reads the tree: "Open files".
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// What the add button is called, to what reads the tree: "New tab" unless told otherwise.
  pub fn add_label(mut self, label: &str) -> Self {
    self.words.add = Arc::from(label);
    self
  }

  /// The word a close button is called by, before the label of its tab, to what reads the tree: "Close", which
  /// makes "Close blur.rs".
  pub fn close_label(mut self, label: &str) -> Self {
    self.words.close = Arc::from(label);
    self
  }

  /// What the More button of [`WindowTabsOverflow::List`] is called, to what reads the tree, and its menu: "Show
  /// the hidden tabs" unless told otherwise. It shows how many tabs it holds.
  pub fn more_label(mut self, label: &str) -> Self {
    self.words.more = Arc::from(label);
    self
  }

  /// Puts the tabs in a state the theme's recipe names, or takes them out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of these tabs over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for these tabs.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the tab Tab would stop on (the selected one, else the first that is not disabled) as it is under the
  /// pointer, held down or focused from the keyboard, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the strip. Its list of tabs has this id with `-list` after it, its add button `-add`, and its More
  /// button `-more`; a tab chosen in the More button's menu is the tab's id with `-item` after it.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// The key of the strip, among what is beside it.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The tabs with what a press, a close and a drag of each does, of no type of the app's.
  fn prepare(mut self) -> Prepared {
    let selected = self.selected.take();
    let write = self.write;
    let on_change: Arc<[Handler<T>]> = self.on_change.into();
    let on_close: Arc<[Handler<T>]> = self.on_close.into();
    let on_detach: Arc<[Handler<T>]> = self.on_detach.into();
    let tabs = self
      .tabs
      .into_iter()
      .map(|tab| {
        let chosen = selected.as_ref() == Some(&tab.value);
        let value = Arc::new(tab.value);
        let select: Heard = {
          let (value, write, handlers) = (value.clone(), write.clone(), on_change.clone());
          // A press of the selected tab asks for nothing, and tells nothing.
          Arc::new(move || {
            if chosen || write.as_ref().is_some_and(|write| !write(&value)) {
              return;
            }
            for handler in handlers.iter() {
              handler(&value);
            }
          })
        };
        let close: Heard = {
          let (value, own, handlers): (_, Arc<[Heard]>, _) = (value.clone(), tab.on_close.into(), on_close.clone());
          Arc::new(move || {
            for handler in own.iter() {
              handler();
            }
            for handler in handlers.iter() {
              handler(&value);
            }
          })
        };
        let detach: Heard = {
          let handlers = on_detach.clone();
          Arc::new(move || {
            for handler in handlers.iter() {
              handler(&value);
            }
          })
        };
        WindowTabEntry {
          label: tab.label,
          icon: tab.icon,
          dirty: tab.dirty,
          close_button: tab.close_button,
          closable: tab.closable,
          disabled: tab.disabled,
          icon_only: tab.icon_only,
          width: tab.width,
          max_width: tab.max_width,
          fit_content: tab.fit_content,
          trailing: tab.trailing,
          list_label: tab.list_label,
          list_detail: tab.list_detail,
          list_trailing: tab.list_trailing,
          descriptions: tab.descriptions,
          selected: chosen,
          select,
          close,
          detach,
          id: tab.id,
          key: tab.key,
        }
      })
      .collect();
    let add: Option<Heard> = (!self.on_add.is_empty()).then(|| {
      let handlers: Arc<[Heard]> = self.on_add.into();
      Arc::new(move || {
        for handler in handlers.iter() {
          handler();
        }
      }) as Heard
    });
    let reorder: Moved = {
      let handlers: Arc<[Moved]> = self.on_reorder.into();
      Arc::new(move |from, to| {
        for handler in handlers.iter() {
          handler(from, to);
        }
      })
    };
    Prepared {
      tabs,
      add,
      add_reason: self.add_reason,
      reorder,
      shortcuts: self.shortcuts,
      disabled: self.disabled,
      overflow: self.overflow,
      menu: self.menu,
      list_shortcut: self.list_shortcut,
      list: self.list,
      dirty_mark: self.dirty_mark,
      tab_stop: self.tab_stop,
      label: self.label,
      words: self.words,
      states: self.states,
      rules: self.rules,
      forced: self.forced,
      id: self.id,
      key: self.key,
    }
  }

  /// Puts the tabs in the tree of `ctx`'s component, in the theme of what is
  /// being rendered, where they keep a drag from one render to the next: the
  /// dragged tab follows the pointer and the others make room for it. They
  /// also keep the selected tab in view where the strip scrolls, and the
  /// focus on the selected tab when the one it was on is closed by the keys.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let mut tabs = self.prepare();
    let key = tabs.key.take();
    let element = match &key {
      Some(key) => ctx.mount_keyed::<WindowTabsHost>(key, WindowTabsProps { tabs }),
      None => ctx.mount::<WindowTabsHost>(WindowTabsProps { tabs }),
    };
    match key {
      Some(key) => element.key(key),
      None => element,
    }
  }

  /// Builds the tabs in `theme`, with nothing kept: a drag of a tab still
  /// tells where it was let go, but the tab does not follow the pointer on
  /// the way. Converting tabs into an element does this in the theme of what
  /// is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    self.prepare().build(theme, Kept::still())
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> From<WindowTabs<T>> for Element {
  fn from(tabs: WindowTabs<T>) -> Self {
    tabs.build(&current_theme())
  }
}

/// A drag of a tab, from its press to its release.
#[derive(Clone, Copy, Debug)]
struct Drag {
  /// The place of the tab dragged.
  from: usize,
  /// How many tabs are in the strip: all of them, but those that left it for the More button's menu.
  count: usize,
  /// Where the tab was from the start of the list when it was pressed, and how wide and high it is.
  start: f32,
  width: f32,
  height: f32,
  /// How wide the list was: what it keeps while the tab is dragged.
  list: f32,
  /// Where the strip's top was in the window, how high and how wide it is: what a tab let go away from is
  /// detached, and the strip keeps its width while the tab is dragged.
  top: f32,
  strip: f32,
  strip_width: f32,
  /// From the start of the first tab to the end of the last.
  span: f32,
  /// How far the pointer has moved along the strip since the press.
  moved: f32,
  /// Whether the pointer has moved far enough for this to be a drag and not a press.
  begun: bool,
  /// Whether the pointer is away from the strip: let go there, the tab is detached.
  away: bool,
}

impl Drag {
  /// Whether the pointer of `event` is away from the strip: more than its height above or below it, or out of
  /// the window past any of its edges (`DragEvent::outside_window`), beside the strip too.
  fn away_at(&self, event: &DragEvent) -> bool {
    let y = event.y;
    event.outside_window || y < self.top - self.strip || y > self.top + 2.0 * self.strip
  }

  /// Where the dragged tab is drawn, from the start of the list: under the pointer, and within the tabs.
  fn left(&self) -> f32 {
    (self.start + self.moved).clamp(0.0, (self.span - self.width).max(0.0))
  }

  /// The place among the tabs in the strip the tab is let go at: that of the tab whose room it is over most.
  fn target(&self) -> usize {
    if self.count <= 1 {
      return 0;
    }
    let pitch = self.span / self.count as f32;
    ((self.left() / pitch).round().max(0.0) as usize).min(self.count - 1)
  }
}

/// What window tabs keep from one render to the next. Mounted tabs keep it
/// and are rendered again as a drag moves; tabs converted into an element on
/// their own have one that starts again at each build, and is not drawn.
#[derive(Clone)]
struct Kept {
  drag: Arc<Mutex<Option<Drag>>>,
  /// Renders mounted tabs again: a drag moved.
  revision: Option<Signal<u64>>,
  /// Whether the last press of a tab became a drag: its release selects nothing.
  dragged: Arc<AtomicBool>,
  /// Whether the keys closed or selected a tab while the focus was in the strip: the focus goes to the selected tab.
  refocus: Arc<AtomicBool>,
  /// The tab last kept in view, where the strip scrolls.
  shown: Arc<Mutex<Option<usize>>>,
  /// The tab the middle button was last pressed on: let go on it, it is closed.
  middle: Arc<Mutex<Option<usize>>>,
  tab_refs: Arc<Mutex<Vec<ElementRef>>>,
  list: ElementRef,
  root: ElementRef,
  scroll: ScrollState,
  /// Of [`WindowTabsOverflow::List`]: which tabs the last layout left out of the strip, by index.
  dropped: Arc<Mutex<Vec<bool>>>,
  /// Whether the menu of the More button is open, and the button it hangs from.
  open: Signal<bool>,
  anchor: ElementRef,
  /// Of the list of every tab: what its search field holds, the field, the rows by the index of their tab, where
  /// the focus is to go once the list is rendered, and how high the window is.
  query: Signal<String>,
  search: ElementRef,
  item_refs: Arc<Mutex<Vec<ElementRef>>>,
  menu_focus: Arc<Mutex<Option<FocusRequest>>>,
  window_height: Arc<Mutex<Option<f32>>>,
  /// The search and the row the open list last scrolled into view for it.
  in_view: Arc<Mutex<Option<(String, usize)>>>,
}

/// Where the focus is to go in the list of every tab, once the strip holds as many tabs as the second says, where
/// it says.
type FocusRequest = (MenuFocus, Option<usize>);

/// Where the focus goes in the list of every tab, once it is rendered.
#[derive(Clone, Copy, Debug, PartialEq)]
enum MenuFocus {
  Search,
  /// The row of the tab at this index, kept in view.
  Item(usize),
}

impl Kept {
  fn new(revision: Option<Signal<u64>>) -> Self {
    Self {
      drag: Arc::new(Mutex::new(None)),
      revision,
      dragged: Arc::new(AtomicBool::new(false)),
      refocus: Arc::new(AtomicBool::new(false)),
      shown: Arc::new(Mutex::new(None)),
      middle: Arc::new(Mutex::new(None)),
      tab_refs: Arc::new(Mutex::new(Vec::new())),
      list: ElementRef::new(),
      root: ElementRef::new(),
      scroll: ScrollState::new(),
      dropped: Arc::new(Mutex::new(Vec::new())),
      open: Signal::new(false),
      anchor: ElementRef::new(),
      query: Signal::new(String::new()),
      search: ElementRef::new(),
      item_refs: Arc::new(Mutex::new(Vec::new())),
      menu_focus: Arc::new(Mutex::new(None)),
      window_height: Arc::new(Mutex::new(None)),
      in_view: Arc::new(Mutex::new(None)),
    }
  }

  /// What tabs converted into an element on their own keep: nothing that is drawn.
  fn still() -> Self {
    Self::new(None)
  }

  /// The ref of the tab at `index`, the same from one render to the next.
  fn tab_ref(&self, index: usize) -> ElementRef {
    let mut refs = self.tab_refs.lock();
    while refs.len() <= index {
      refs.push(ElementRef::new());
    }
    refs[index].clone()
  }

  /// The ref of the row of the tab at `index` in the list of every tab.
  fn item_ref(&self, index: usize) -> ElementRef {
    let mut refs = self.item_refs.lock();
    while refs.len() <= index {
      refs.push(ElementRef::new());
    }
    refs[index].clone()
  }

  /// Asks for the focus to go to `to` in the list of every tab, once it is rendered.
  fn focus_menu(&self, to: MenuFocus) {
    *self.menu_focus.lock() = Some((to, None));
    self.moved();
  }

  /// Asks for the focus to go to `to` once the strip holds `count` tabs: after the app took a closed tab away.
  fn focus_menu_with(&self, to: MenuFocus, count: usize) {
    *self.menu_focus.lock() = Some((to, Some(count)));
    self.moved();
  }

  /// Renders mounted tabs again, to draw where a drag is.
  fn moved(&self) {
    if let Some(revision) = &self.revision {
      revision.update(|revision| *revision += 1);
    }
  }

  /// The drag drawn now: one that has begun, of mounted tabs.
  fn drawn(&self) -> Option<Drag> {
    self.revision.as_ref()?;
    (*self.drag.lock()).filter(|drag| drag.begun)
  }
}

/// Window tabs as they are built, once a press of each tab knows what it does.
#[derive(Clone)]
struct Prepared {
  tabs: Vec<WindowTabEntry>,
  add: Option<Heard>,
  add_reason: Option<Arc<str>>,
  reorder: Moved,
  shortcuts: bool,
  disabled: bool,
  overflow: WindowTabsOverflow,
  menu: WindowTabsMenu,
  list_shortcut: bool,
  list: ListSettings,
  dirty_mark: WindowTabsDirtyMark,
  tab_stop: bool,
  label: Option<Arc<str>>,
  words: Words,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Prepared {
  /// The tabs in `theme`, with what `kept` keeps.
  fn build(self, theme: &Theme, mut kept: Kept) -> Element {
    if let Some(open) = &self.list.open {
      kept.open = open.clone();
    }
    let hidden = self.hidden(&kept);
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(WindowTabsState::Disabled, self.disabled)
    .state(WindowTabsState::Overflowing, hidden.contains(&true))
    .state(
      WindowTabsState::MenuOpen,
      kept.revision.is_some() && self.overflow == WindowTabsOverflow::List && kept.open.get(),
    )
    .state(WindowTabsState::Searchable, self.searchable());
    let owner = theme
      .resolve::<WindowTabs>(&case, Some(&self.rules))
      .owned_by(self.id.as_ref());
    let view = WindowTabsView {
      tabs: self.tabs,
      disabled: self.disabled,
      shortcuts: self.shortcuts,
      overflow: self.overflow,
      menu: self.menu,
      list_shortcut: self.list_shortcut,
      list: self.list,
      dirty_mark: self.dirty_mark,
      tab_stop: self.tab_stop,
      hidden,
      label: self.label,
      theme: theme.clone(),
      case,
      rules: self.rules,
      recipes: std::array::from_fn(|_| OnceCell::new()),
      owner,
      add: self.add,
      add_reason: self.add_reason,
      reorder: self.reorder,
      words: self.words,
      drag: kept.drawn(),
      kept,
      forced: self.forced,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<WindowTabs>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// Whether the list of every tab leads with its search field.
  fn searchable(&self) -> bool {
    self.overflow == WindowTabsOverflow::List && self.menu == WindowTabsMenu::All && all_tabs::searched(self.tabs.len())
  }

  /// Which tabs are out of the strip, by index: of mounted tabs that leave it where they do not fit, those the last
  /// layout left out, but the selected one, which stays. None of the others.
  fn hidden(&self, kept: &Kept) -> Vec<bool> {
    let mut dropped = kept.dropped.lock();
    if self.overflow != WindowTabsOverflow::List || kept.revision.is_none() {
      dropped.clear();
      return vec![false; self.tabs.len()];
    }
    dropped.resize(self.tabs.len(), false);
    self
      .tabs
      .iter()
      .zip(dropped.iter())
      .map(|(tab, dropped)| *dropped && !tab.selected)
      .collect()
  }
}

#[derive(Clone)]
struct WindowTabsProps {
  tabs: Prepared,
}

/// The tabs are prepared anew by every render of what holds them, so props are never the same as before.
impl PartialEq for WindowTabsProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl crate::app::component::DevtoolsInspectable for WindowTabsProps {}

/// What mounted window tabs keep from one render to the next.
struct WindowTabsHost {
  kept: Kept,
}

impl Component for WindowTabsHost {
  type Props = WindowTabsProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      kept: Kept::new(Some(ctx.signal(0_u64))),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let WindowTabsProps { tabs } = ctx.props::<WindowTabsProps>().clone();
    if let Some(revision) = &self.kept.revision {
      // Read so that a drag that moves renders the tabs again.
      revision.get();
    }
    let selected = tabs.tabs.iter().position(|tab| tab.selected);
    if let Some(index) = selected {
      let at = self.kept.tab_ref(index);
      if self.kept.refocus.swap(false, Ordering::Relaxed) {
        ctx.focus(&at);
      }
      let mut shown = self.kept.shown.lock();
      if *shown != Some(index) {
        *shown = Some(index);
        ctx.scroll_into_view(&at);
      }
    } else {
      self.kept.refocus.store(false, Ordering::Relaxed);
    }
    *self.kept.window_height.lock() = Some(ctx.window().info().logical_height());
    let count = tabs.tabs.len();
    let open = tabs.list.open.clone().unwrap_or_else(|| self.kept.open.clone());
    let in_view = (tabs.menu == WindowTabsMenu::All && open.get_untracked())
      .then(|| {
        let query = self.kept.query.get_untracked();
        all_tabs::kept_in_view(&tabs.tabs, &query).map(|index| (query, index))
      })
      .flatten();
    let element = tabs.build(&current_theme(), self.kept.clone());
    {
      // The open list keeps the selected tab's row, or the first that matches, in view: as it opens and as its search
      // changes, and not again, so that rows scrolled by the wheel stay where they are.
      let mut shown = self.kept.in_view.lock();
      if *shown != in_view {
        if let Some((_, index)) = &in_view {
          ctx.scroll_into_view(&self.kept.item_ref(*index));
        }
        *shown = in_view;
      }
    }
    let request = {
      let mut request = self.kept.menu_focus.lock();
      // A request for after a tab is taken away waits for the strip that no longer holds it; the list closed, none.
      if !open.get_untracked() {
        *request = None;
      }
      match *request {
        Some((_, Some(awaited))) if awaited != count => None,
        _ => request.take().map(|(to, _)| to),
      }
    };
    match request {
      Some(MenuFocus::Search) => ctx.focus(&self.kept.search),
      Some(MenuFocus::Item(index)) => {
        let item = self.kept.item_ref(index);
        ctx.focus(&item);
        ctx.scroll_into_view(&item);
      }
      None => {}
    }
    element
  }
}

/// What stands in the list, one after the other: a tab, or the room a dragged tab is let go into.
#[derive(Clone, Copy, PartialEq)]
enum Place {
  Tab(usize),
  Room,
}

/// Window tabs as a renderer sees them: the tabs, the style of their parts,
/// and their behavior ready to be dressed.
pub struct WindowTabsView {
  pub tabs: Vec<WindowTabEntry>,
  pub disabled: bool,
  pub shortcuts: bool,
  pub overflow: WindowTabsOverflow,
  /// What the More button of the list mode opens.
  pub menu: WindowTabsMenu,
  /// Whether Ctrl+Shift+A (Cmd+Shift+A) opens the More button's menu.
  pub list_shortcut: bool,
  list: ListSettings,
  pub dirty_mark: WindowTabsDirtyMark,
  /// Whether Tab stops in the strip.
  pub tab_stop: bool,
  /// Which tabs left the strip for the More button's menu, by index.
  pub hidden: Vec<bool>,
  pub label: Option<Arc<str>>,
  theme: Theme,
  case: Case,
  rules: Recipe,
  /// The recipe of a tab in each mix of its own states, resolved when it is first asked for.
  recipes: [OnceCell<ResolvedRecipe>; 32],
  /// What the recipes name their parts after, and count them with.
  owner: ResolvedRecipe,
  add: Option<Heard>,
  /// Why the add button cannot be pressed now, where it cannot: it is shown faded, with this in its tooltip.
  pub add_reason: Option<Arc<str>>,
  reorder: Moved,
  words: Words,
  /// The drag drawn now, where the tabs are mounted.
  drag: Option<Drag>,
  kept: Kept,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl WindowTabsView {
  fn recipe(&self, selected: bool, dirty: bool, dragged: bool, disabled: bool, icon_only: bool) -> &ResolvedRecipe {
    let index = usize::from(selected)
      | usize::from(dirty) << 1
      | usize::from(dragged) << 2
      | usize::from(disabled) << 3
      | usize::from(icon_only) << 4;
    self.recipes[index].get_or_init(|| {
      self
        .theme
        .resolve::<WindowTabs>(
          &self
            .case
            .clone()
            .state(WindowTabsState::Selected, selected)
            .state(WindowTabsState::Dirty, dirty)
            .state(WindowTabsState::Dragged, dragged)
            .state(WindowTabsState::Disabled, disabled)
            .state(WindowTabsState::IconOnly, icon_only),
          Some(&self.rules),
        )
        .owned_like(&self.owner)
    })
  }

  /// The style of a part that is no tab's, as these tabs are to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe(false, false, false, self.disabled, false).part(part)
  }

  /// The style of a part of the tab at `index`: in the states of that tab, and in the interaction forced where
  /// it is the tab Tab stops on.
  pub fn tab_part(&self, index: usize, part: impl Into<Name>) -> ResolvedPart {
    self.tab_part_as(index, part, self.tabs[index].dirty)
  }

  /// [`tab_part`](Self::tab_part) with the tab dirty or not: the close slot of a tab whose dot stands after its
  /// label is drawn as a clean tab's.
  fn tab_part_as(&self, index: usize, part: impl Into<Name>, dirty: bool) -> ResolvedPart {
    let tab = &self.tabs[index];
    let dragged = self.drag.is_some_and(|drag| drag.from == index);
    let forced = self.forced.filter(|_| self.entry() == Some(index));
    let recipe = self.recipe(
      tab.selected,
      dirty,
      dragged,
      self.disabled || tab.disabled,
      tab.icon_only,
    );
    // A tab with an id names its parts after it; one with a key only, after the tabs' id and the key.
    let recipe = match (&tab.id, &tab.key) {
      (None, Some(key)) => recipe.keyed_item(key, WindowTabsPart::Tab),
      (id, _) => recipe.item(id.as_ref(), WindowTabsPart::Tab),
    };
    recipe.part(part).forced(forced)
  }

  /// The tab Tab stops on: the selected one, else the first that is not disabled. None where all are.
  pub fn entry(&self) -> Option<usize> {
    if self.disabled {
      return None;
    }
    let open = |tab: &WindowTabEntry| !tab.disabled;
    self
      .tabs
      .iter()
      .position(|tab| tab.selected && open(tab))
      .or_else(|| self.tabs.iter().position(open))
  }

  pub fn theme(&self) -> &Theme {
    &self.theme
  }

  /// Whether the app is told of a press of the add button. It is shown where this is so, or where it cannot be
  /// pressed for a reason ([`add_reason`](Self::add_reason)).
  pub fn has_add(&self) -> bool {
    self.add.is_some()
  }

  /// The index of the tab being dragged, where the tabs are mounted and a drag has begun.
  pub fn dragged(&self) -> Option<usize> {
    self.drag.map(|drag| drag.from)
  }

  /// Whether a tab can be closed by a person: it is closable, and neither it nor the strip is disabled. Whether it
  /// shows its close button does not matter: the middle button, Delete and the window's shortcut close it.
  fn closable(&self, index: usize) -> bool {
    let tab = &self.tabs[index];
    tab.closable && !tab.disabled && !self.disabled
  }

  /// Whether the dot of the tab at `index` stands after its label, not in its close slot.
  fn dot_after_label(&self, index: usize) -> bool {
    self.tabs[index].dirty && self.dirty_mark == WindowTabsDirtyMark::AfterLabel
  }

  /// The dot of unsaved changes of the tab at `index`, in the recipe of a tab `dirty` or not: the dot after a label
  /// is drawn as a clean tab's, so that the pointer does not swap it for the x.
  fn dot(&self, index: usize, dirty: bool) -> Element {
    Row::new()
      .part(&self.tab_part_as(index, WindowTabsPart::Dot, dirty))
      .into()
  }

  /// The close slot of the tab at `index`: its close button, which shows the
  /// dot of unsaved changes until the pointer is on the tab; or the dot
  /// alone where the tab shows no close button. None where it has neither,
  /// and for a tab of its icon alone. Where the dot stands after the label
  /// the button is drawn as a clean tab's, and so is the dot: both show, and
  /// the pointer swaps nothing. The button is for the pointer,
  /// called "Close" and the tab's label to what reads the tree: the keyboard
  /// closes the tab with Delete.
  pub fn close(&self, index: usize) -> Option<Element> {
    let entry = &self.tabs[index];
    if entry.icon_only {
      return None;
    }
    let in_slot = entry.dirty && !self.dot_after_label(index);
    let part = |part: WindowTabsPart| self.tab_part_as(index, part, in_slot);
    let centred = |child: Element| -> Element {
      Row::new()
        .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
        .justify(Justify::Center)
        .align_items(Alignment::Center)
        .child(child)
        .into()
    };
    // A tab drawn as under the pointer is not its close slot under the pointer: the slot keeps its own state.
    let slot_part = part(WindowTabsPart::Close).forced(None);
    if !(entry.close_button && entry.closable) {
      if !in_slot {
        return None;
      }
      return Some(
        Row::new()
          .part(&slot_part)
          .justify(Justify::Center)
          .align_items(Alignment::Center)
          .child(centred(self.dot(index, true)))
          .into(),
      );
    }
    let x = centred(glyph(&part(WindowTabsPart::CloseIcon), Name::new(icons::CLOSE), false));
    let slot: Element = if in_slot {
      Stack::new()
        .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
        .child(x)
        .child(centred(self.dot(index, true)))
        .into()
    } else {
      x
    };
    let close = entry.close.clone();
    let refocus = self.kept.refocus.clone();
    let closable = self.closable(index);
    let mut button = components::Button::empty()
      .focusable(false)
      .disabled(!closable)
      .part(&slot_part)
      .justify(Justify::Center)
      .align_items(Alignment::Center)
      .describe("a11y_name", format!("{} {}", self.words.close, entry.label))
      // A press of it is its own: it neither selects the tab nor drags the window.
      .on_mouse_down(|event: MouseEvent| event.stop_propagation())
      .on_click(move |event: MouseEvent| {
        event.stop_propagation();
        refocus.store(false, Ordering::Relaxed);
        close();
      })
      .child(slot);
    if let Some(id) = &entry.id {
      button = button.id(format!("{id}-close"));
    }
    Some(button.into())
  }

  /// The label of the tab at `index` with what stands right after it: the
  /// dot where it is after the label, and the app's marks. It takes the room
  /// the icon and the close slot leave; the label gives way first, and ends
  /// in an ellipsis where it is not enough.
  fn title(&self, index: usize) -> Element {
    let entry = &self.tabs[index];
    // A tab of a width of its own shares it out: the title takes what the rest leaves. A tab as wide as what it
    // shows is as wide as its title is, which gives way under the tab's cap.
    let (grow, basis) = if self.fits_content(index) {
      (0.0, None)
    } else {
      (1.0, Some(0.0))
    };
    let mut title = Row::new()
      .part(&self.tab_part(index, WindowTabsPart::Title))
      .align_items(Alignment::Center)
      .flex_full(grow, 1.0, basis)
      .min_width(0.0)
      .child(one_line(&self.tab_part(index, WindowTabsPart::Label), &entry.label));
    if self.dot_after_label(index) {
      title = title.child(self.dot(index, false));
    }
    if !entry.trailing.is_empty() {
      title = title.child(
        Row::new()
          .part(&self.tab_part(index, WindowTabsPart::Marks))
          .align_items(Alignment::Center)
          .with_children(entry.trailing.iter().cloned()),
      );
    }
    title.into()
  }

  /// What the keys do on a tab that is not disabled: Escape puts it back where it is being dragged, and Delete
  /// closes it where a person may close it.
  fn keys(&self, index: usize) -> impl Fn(KeyboardEvent) + Send + Sync + 'static {
    let closable = self.closable(index);
    let (close, kept) = (self.tabs[index].close.clone(), self.kept.clone());
    move |event: KeyboardEvent| {
      // Escape puts a dragged tab back where it was: its release tells nothing.
      if event.key == "Escape" {
        let mut drag = kept.drag.lock();
        if drag.is_some_and(|drag| drag.from == index && drag.begun) {
          *drag = None;
          drop(drag);
          event.prevent_default();
          kept.moved();
        }
        return;
      }
      let plain = !(event.ctrl || event.alt || event.meta || event.shift);
      if closable && event.focus_within && event.key == "Delete" && plain {
        event.prevent_default();
        kept.refocus.store(true, Ordering::Relaxed);
        close();
      }
    }
  }

  /// A press of the middle button on the tab at `index`, let go on it, closes it.
  fn middle(&self, index: usize, tab: components::Button) -> components::Button {
    let (pressed, released) = (self.kept.middle.clone(), self.kept.middle.clone());
    let close = self.tabs[index].close.clone();
    tab
      .on_mouse_down(move |event: MouseEvent| {
        if event.button == MouseButton::Middle {
          *pressed.lock() = Some(index);
        }
      })
      .on_mouse_up(move |event: MouseEvent| {
        if event.button == MouseButton::Middle && released.lock().take() == Some(index) {
          event.stop_propagation();
          close();
        }
      })
  }

  /// The tab at `index`: what is pressed, with its icon, its label with what
  /// stands after it, and its close slot; a tab of its icon alone has the
  /// icon only. It is a tab to what reads the tree, called by its label, and
  /// says whether it is the selected one, after what the app says of it; the
  /// list's arrow keys move to it unless it is disabled, and Delete on it
  /// closes it. A press goes no further than it, a press of the middle button
  /// closes it, and a drag moves it along the strip or out of it.
  pub fn tab(&self, index: usize) -> components::Button {
    let entry = &self.tabs[index];
    let part = |part: WindowTabsPart| self.tab_part(index, part);
    let disabled = self.disabled || entry.disabled;
    let (select, kept) = (entry.select.clone(), self.kept.clone());
    let mut tab = tab_stop(pressable(&part(WindowTabsPart::Tab), disabled), self.tab_stop);
    for (name, value) in &entry.descriptions {
      tab = tab.describe(name.clone(), value.clone());
    }
    tab = tab
      .justify(if entry.icon_only {
        Justify::Center
      } else {
        Justify::Start
      })
      .align_items(Alignment::Center)
      .interaction_group(true)
      .focus_entry(entry.selected)
      .ref_element(self.kept.tab_ref(index))
      .describe("a11y_role", "tab")
      .describe("a11y_name", entry.label.clone())
      .describe("a11y_selected", if entry.selected { "true" } else { "false" })
      // The title bar around the strip drags the window from a press: not from a press on a tab.
      .on_mouse_down(|event: MouseEvent| event.stop_propagation())
      .on_click(move |_| {
        if !kept.dragged.swap(false, Ordering::Relaxed) {
          select();
        }
      });
    if self.overflow == WindowTabsOverflow::List {
      // Some of the tabs may be in the More button's menu: each says where it is among them all.
      tab = tab
        .describe("a11y_posinset", (index + 1).to_string())
        .describe("a11y_setsize", self.tabs.len().to_string());
    }
    if !disabled {
      tab = self.draggable(index, tab).on_key_down(self.keys(index));
      if self.closable(index) {
        tab = self.middle(index, tab);
      }
    }
    if let Some(name) = entry.icon {
      tab = tab.child(glyph(&part(WindowTabsPart::Icon), name, false));
    }
    if !entry.icon_only {
      tab = tab.child(self.title(index));
    }
    if let Some(close) = self.close(index) {
      tab = tab.child(close);
    }
    if let Some(id) = &entry.id {
      tab = tab.id(id.clone());
    }
    if let Some(key) = &entry.key {
      tab = tab.key(key.clone());
    }
    let (least, most) = self.widths(index);
    if self.fits_content(index) {
      // As wide as what it shows, between its least width and its cap. Where it leaves the strip rather than give
      // way, it keeps that width.
      let shrink = match self.overflow {
        WindowTabsOverflow::Scroll => 1.0,
        WindowTabsOverflow::List => 0.0,
      };
      return tab.flex_full(0.0, shrink, None).min_width(least).max_width(most);
    }
    if entry.width.is_some() || self.overflow == WindowTabsOverflow::List {
      // A tab of a width of its own, or one that leaves the strip rather than give way, is as wide as it is.
      return tab
        .flex_full(0.0, 0.0, Some(most))
        .width(most)
        .min_width(most)
        .max_width(most);
    }
    // It shares the room of the list with the other tabs: from its least width, growing to its most by as much as
    // it can grow, so that every tab is at its most when the strip has room for all of them so.
    tab
      .flex_full(most - least, 0.0, Some(least))
      .min_width(least)
      .max_width(most)
  }

  /// The widths of the tab at `index`: the least, and the most, which is what it is with room enough. The
  /// recipe's for a tab in its states, under the tab's own most width; both the tab's own width where it has one.
  fn widths(&self, index: usize) -> (f32, f32) {
    let entry = &self.tabs[index];
    let (least, most) = self.recipe_widths(self.tab_part(index, WindowTabsPart::Tab).rest());
    if let Some(width) = entry.width {
      return (width, width);
    }
    // A tab as wide as what it shows reaches its own cap, not the recipe's most.
    let most = match entry.fit_content.filter(|_| !entry.icon_only) {
      Some(cap) => cap.max(least),
      None => most,
    };
    match entry.max_width {
      Some(cap) => (least.min(cap), most.min(cap)),
      None => (least, most),
    }
  }

  /// Whether the tab at `index` is as wide as what it shows: it fits its content, has no width of its own, and is
  /// not of its icon alone.
  fn fits_content(&self, index: usize) -> bool {
    let entry = &self.tabs[index];
    entry.fit_content.is_some() && entry.width.is_none() && !entry.icon_only
  }

  /// The room between two tabs, and before the More button: the gap of the `list` part.
  fn list_gap(&self) -> f32 {
    spacing(self.part(WindowTabsPart::List).rest().gap, &self.theme).unwrap_or(0.0)
  }

  /// The room between the list and what is after it, the add button: the gap of the `root` part.
  fn root_gap(&self) -> f32 {
    spacing(self.part(WindowTabsPart::Root).rest().gap, &self.theme).unwrap_or(0.0)
  }

  /// How wide a separator is: none is laid out where the recipe makes it 0 wide.
  fn separator_width(&self) -> f32 {
    px(self.part(WindowTabsPart::Separator).rest().width, &self.theme).unwrap_or(0.0)
  }

  /// The least and the most width of a tab in the recipe's `style`.
  fn recipe_widths(&self, style: super::PartStyle) -> (f32, f32) {
    let most = px(style.max_width, &self.theme).or_else(|| px(style.width, &self.theme));
    let least = px(style.min_width, &self.theme).unwrap_or(0.0);
    (least, most.unwrap_or(least).max(least))
  }

  /// The tabs in the strip, by index: all of them, but those that left it for the More button's menu.
  fn shown(&self) -> Vec<usize> {
    (0..self.tabs.len()).filter(|index| !self.hidden[*index]).collect()
  }

  /// What a press and a move of the tab at `index` do: past a few pixels it is a drag, which selects the tab and
  /// draws it under the pointer where the tabs are mounted; let go, it tells where the tab went among the tabs in
  /// the strip.
  fn draggable(&self, index: usize, tab: components::Button) -> components::Button {
    let shown: Arc<[usize]> = self.shown().into();
    let kept = self.kept.clone();
    let (start, moved, end) = (kept.clone(), kept.clone(), kept);
    let select = self.tabs[index].select.clone();
    let detach = self.tabs[index].detach.clone();
    let reorder = self.reorder.clone();
    let (starting, ending) = (shown.clone(), shown);
    tab
      .start_drag_buttons(MouseButtonMask::LEFT)
      .on_drag_start(move |_: DragEvent| {
        start.dragged.store(false, Ordering::Relaxed);
        let at = start.tab_ref(index);
        let list_x = start.list.x();
        let span = starting
          .last()
          .map(|last| start.tab_ref(*last))
          .map(|last| last.x() + last.width() - list_x)
          .unwrap_or(0.0);
        *start.drag.lock() = Some(Drag {
          from: index,
          count: starting.len(),
          start: at.x() - list_x,
          width: at.width(),
          height: at.height(),
          list: start.list.width(),
          top: start.root.y(),
          strip: start.root.height(),
          strip_width: start.root.width(),
          span,
          moved: 0.0,
          begun: false,
          away: false,
        });
      })
      .on_drag_move(move |event: DragEvent| {
        let begins = {
          let mut drag = moved.drag.lock();
          let Some(drag) = drag.as_mut() else {
            return;
          };
          drag.moved = event.total_delta_x;
          drag.away = drag.away_at(&event);
          let begins = !drag.begun && event.total_delta_x.hypot(event.total_delta_y) > DRAG_START;
          if begins {
            drag.begun = true;
          }
          if !drag.begun {
            return;
          }
          begins
        };
        if begins {
          moved.dragged.store(true, Ordering::Relaxed);
          select();
        }
        moved.moved();
      })
      .on_drag_end(move |event: DragEvent| {
        let Some(drag) = end.drag.lock().take() else {
          return;
        };
        if !drag.begun {
          return;
        }
        end.moved();
        if drag.away_at(&event) {
          detach();
          return;
        }
        let at = Drag {
          moved: event.total_delta_x,
          ..drag
        }
        .target();
        // The place of the tab in the strip whose room it took: where it goes among them all.
        let to = ending.get(at).copied().unwrap_or(drag.from);
        if to != drag.from {
          reorder(drag.from, to);
        }
      })
  }

  /// The line between two tabs that are not the selected one, and before the add button.
  pub fn separator(&self) -> Element {
    Row::new().part(&self.part(WindowTabsPart::Separator)).into()
  }

  /// The add button, where the app is told of a press of it: a stop of Tab after the tabs, called "New tab" or
  /// the strip's `add_label` to what reads the tree. A press of it goes no further than it. Where it cannot be
  /// pressed for a reason, it is disabled, in the `Disabled` state of its parts, and describes itself by the
  /// reason; [`add_control`](Self::add_control) puts the reason's tooltip on it.
  pub fn add(&self) -> Option<components::Button> {
    let unavailable = self.add_reason.is_some();
    if self.add.is_none() && !unavailable {
      return None;
    }
    let disabled = self.disabled || unavailable;
    let part = |part: WindowTabsPart| match unavailable {
      true => self.recipe(false, false, false, true, false).part(part),
      false => self.part(part),
    };
    let mut button = tab_stop(pressable(&part(WindowTabsPart::Add), disabled), self.tab_stop)
      .describe("a11y_name", self.words.add.clone())
      .align_items(Alignment::Center)
      .on_mouse_down(|event: MouseEvent| event.stop_propagation())
      .child(glyph(&part(WindowTabsPart::AddIcon), Name::new(icons::PLUS), false));
    if let Some(reason) = &self.add_reason {
      button = button.describe("a11y_description", reason.clone());
    } else if let Some(add) = self.add.clone() {
      button = button.on_click(move |_| add());
    }
    if let Some(id) = &self.id {
      button = button.id(format!("{id}-add"));
    }
    Some(button)
  }

  /// The add button as the strip lays it out: [`add`](Self::add), under the tooltip of its reason where it cannot
  /// be pressed.
  pub fn add_control(&self) -> Option<Element> {
    let button = self.add()?;
    Some(match &self.add_reason {
      Some(reason) => Tooltip::new(reason)
        .placement(crate::app::ctx::Placement::Bottom)
        .declared(button),
      None => button.into(),
    })
  }

  /// What stands in the list in order: the tabs, and while a tab is dragged the others with the room it would be
  /// let go into among them, before the tab in the strip whose place it takes.
  fn places(&self) -> Vec<Place> {
    let mut places: Vec<Place> = (0..self.tabs.len()).map(Place::Tab).collect();
    if let Some(drag) = self.drag {
      places.retain(|place| *place != Place::Tab(drag.from));
      let others: Vec<usize> = self.shown().into_iter().filter(|index| *index != drag.from).collect();
      let at = match others.get(drag.target()) {
        Some(before) => places.iter().position(|place| *place == Place::Tab(*before)),
        None => others
          .last()
          .and_then(|last| places.iter().position(|place| *place == Place::Tab(*last)))
          .map(|last| last + 1),
      };
      places.insert(at.unwrap_or(0).min(places.len()), Place::Room);
    }
    places
  }

  /// Whether a separator stands beside `place`: one stands between two tabs that are not the selected one, and
  /// none beside the room of a dragged tab, which is the selected one.
  fn plain(&self, place: Place) -> bool {
    match place {
      Place::Tab(index) => !self.tabs[index].selected,
      Place::Room => false,
    }
  }

  /// Of each place, whether a separator stands before it: between it and the place before it, both tabs that are
  /// not the selected one, whether they are in the strip or in the More button's menu. A tab that leaves the strip
  /// takes the separator before it along, so it needs the same room to come back as it took. None where the recipe
  /// makes a separator 0 wide.
  fn separated(&self, places: &[Place]) -> Vec<bool> {
    let drawn = self.separator_width() > 0.0;
    let mut before: Option<Place> = None;
    places
      .iter()
      .map(|place| {
        let separated = drawn && before.is_some_and(|before| self.plain(before)) && self.plain(*place);
        before = Some(*place);
        separated
      })
      .collect()
  }

  /// The list of the tabs with the separators between them: one stop of Tab,
  /// at the selected tab, whose Left and Right select. It is a tab list to
  /// what reads the tree. While a tab is dragged, the others make room for it
  /// where it would be let go, and it is drawn over them under the pointer.
  /// Where the tabs leave the strip into a list, each but the selected one
  /// leaves it with the separator before it, and the More button follows them.
  pub fn list(&self) -> Element {
    let places = self.places();
    let separated = self.separated(&places);
    let mut row = Row::new()
      .part(&self.part(WindowTabsPart::List))
      .align_items(Alignment::Center)
      .focus_group(FocusGroup::horizontal().activate(true).one_stop(true))
      .ref_element(self.kept.list.clone())
      .describe("a11y_role", "tablist")
      .describe("a11y_name", self.label.clone().unwrap_or_default())
      .describe("a11y_orientation", "horizontal");
    if let Some(id) = &self.id {
      row = row.id(format!("{id}-list"));
    }
    for (at, place) in places.iter().enumerate() {
      let separator = separated[at].then(|| self.separator());
      let element: Element = match place {
        Place::Tab(index) => self.tab(*index).into(),
        Place::Room => {
          let width = self.drag.map_or(0.0, |drag| drag.width);
          Row::new()
            .part(&self.part(WindowTabsPart::Tab))
            .flex_full(0.0, 0.0, Some(width))
            .width(width)
            .into()
        }
      };
      row = match (place, self.leaves(*place)) {
        (Place::Tab(index), true) => row.child(self.leaving(*index, separator, element)),
        _ => row.with_children(separator).child(element),
      };
    }
    if let Some(more) = self.more() {
      row = row.child(more);
    }
    if let Some(list) = self.list_beside() {
      row = row.child(list);
    }
    let Some(drag) = self.drag else {
      return match self.overflow {
        WindowTabsOverflow::Scroll => row.into(),
        // The list is as wide as its tabs, with their gaps and separators, and gives way where the strip has less
        // room: its tabs leave it where they do not fit, and come back as it is given room again.
        WindowTabsOverflow::List => row.flex_shrink(1.0).min_width(0.0).into(),
      };
    };
    // The dragged tab is drawn over the others, as wide as it was, where the pointer has it; the list keeps the
    // width it had.
    Stack::new()
      .child(row.width(drag.list))
      .child(self.tab(drag.from).absolute(drag.left(), 0.0, drag.width, drag.height))
      .into()
  }

  /// What the keys of the window do, from anywhere in it: select the next tab and the one before, close the
  /// selected one, move it.
  fn shortcut(&self) -> impl Fn(KeyboardEvent) + Send + Sync + 'static {
    // Of each tab: whether it is the selected one, whether it can be selected, what selects it, what closes it.
    let tabs: Vec<(bool, bool, Heard, Option<Heard>)> = self
      .tabs
      .iter()
      .enumerate()
      .map(|(index, tab)| {
        let close = self.closable(index).then(|| tab.close.clone());
        (tab.selected, !tab.disabled, tab.select.clone(), close)
      })
      .collect();
    let (reorder, refocus) = (self.reorder.clone(), self.kept.refocus.clone());
    // While the list of every tab is open, the window's keys are its own: Ctrl+W closes the row the keys are on.
    let open = (self.menu == WindowTabsMenu::All).then(|| self.kept.open.clone());
    move |event: KeyboardEvent| {
      if event.default_prevented()
        || event.composing
        || event.alt
        || tabs.is_empty()
        || open.as_ref().is_some_and(|open| open.get_untracked())
      {
        return;
      }
      let mac = cfg!(target_os = "macos");
      let at = tabs.iter().position(|tab| tab.0);
      // The next tab that can be selected, `by` one way or the other, round at the ends.
      let step = |by: isize| -> Option<usize> {
        let count = tabs.len() as isize;
        let from = at.map(|at| at as isize).unwrap_or(if by > 0 { -1 } else { count });
        (1..=count)
          .map(|n| (from + by * n).rem_euclid(count) as usize)
          .find(|index| tabs[*index].1 && Some(*index) != at)
      };
      let key = event.key.as_str();
      let (select, close, moves) = match key {
        "Tab" if event.ctrl && !event.meta => (Some(if event.shift { -1 } else { 1 }), false, None),
        "PageDown" | "PageUp" if event.ctrl && !event.meta => {
          let by = if key == "PageDown" { 1 } else { -1 };
          if event.shift {
            (None, false, Some(by))
          } else if mac {
            return;
          } else {
            (Some(by), false, None)
          }
        }
        _ if key.eq_ignore_ascii_case("w")
          && !event.shift
          && if mac {
            event.meta && !event.ctrl
          } else {
            event.ctrl && !event.meta
          } =>
        {
          (None, true, None)
        }
        _ => return,
      };
      event.prevent_default();
      if event.focus_within {
        refocus.store(true, Ordering::Relaxed);
      }
      if let Some(by) = select {
        if let Some(next) = step(by) {
          (tabs[next].2)();
        }
      } else if close {
        if let Some(close) = at.and_then(|at| tabs[at].3.as_ref()) {
          close();
        }
      } else if let (Some(by), Some(from)) = (moves, at) {
        let to = from as isize + by;
        if to >= 0 && (to as usize) < tabs.len() {
          reorder(from, to as usize);
        }
      }
    }
  }

  /// How wide the strip is with room enough: its tabs at their most, the separators between them, and the add
  /// button with the separator before it.
  fn full_width(&self) -> f32 {
    if let Some(drag) = self.drag {
      return drag.strip_width;
    }
    let places = self.places();
    let separator = px(self.part(WindowTabsPart::Separator).rest().width, &self.theme).unwrap_or(0.0);
    let separators = self
      .separated(&places)
      .into_iter()
      .filter(|separated| *separated)
      .count();
    let tabs: f32 = (0..self.tabs.len()).map(|index| self.widths(index).1).sum();
    // The list's gap stands between every two things in it, tabs and separators alike.
    let items = self.tabs.len() + separators;
    let gaps = items.saturating_sub(1) as f32 * self.list_gap();
    let add = self.add.as_ref().map_or(0.0, |_| {
      let (root_gap, before) = (self.root_gap(), self.separator_before_add());
      let before = if before { separator + root_gap } else { 0.0 };
      root_gap + before + px(self.part(WindowTabsPart::Add).rest().width, &self.theme).unwrap_or(0.0)
    });
    tabs + separators as f32 * separator + gaps + add
  }

  /// Whether a separator is laid out before the add button: where the last tab in the strip is not the selected
  /// one, and in the list mode always, shown or not, so that the room of the strip does not change as tabs leave
  /// it. None where the recipe makes a separator 0 wide.
  fn separator_before_add(&self) -> bool {
    self.separator_width() > 0.0 && (self.overflow == WindowTabsOverflow::List || self.last_plain())
  }

  /// Whether the last tab in the strip is not the selected one: a separator stands between it and the add button.
  fn last_plain(&self) -> bool {
    self.shown().last().is_some_and(|last| !self.tabs[*last].selected)
  }

  /// How the default renderer builds window tabs: the list, which scrolls
  /// sideways where its tabs do not fit at their least width, or which they
  /// leave for the More button's menu ([`WindowTabsOverflow::List`]); and the
  /// add button after it, with a separator before it where the last tab is
  /// not the selected one. The window's keys are heard on the strip.
  pub fn render(self) -> Element {
    let list: Element = match self.overflow {
      // The tabs share the room the strip is given, down to their least width, and past that the list scrolls.
      // A wheel turned up and down over it scrolls it sideways, as a browser's strip of tabs.
      WindowTabsOverflow::Scroll => ScrollBoth::new(self.list())
        .fill_width(true)
        .wheel_across(true)
        .with_scroll_state(self.kept.scroll.clone())
        .scrollbar(ScrollBarStyle::hidden())
        .flex_full(1.0, 1.0, Some(0.0))
        .min_width(0.0)
        .into(),
      // As wide as the list, and giving way where the strip has less room. A column and not a row: what the strip
      // takes from it is the room the list is laid out in, which its tabs leave one by one, where a row in a row
      // would be dropped from whole by the strip's line.
      WindowTabsOverflow::List => Column::new()
        .justify(Justify::Center)
        .flex_full(0.0, 1.0, None)
        .min_width(0.0)
        .child(self.list())
        .into(),
    };
    // The strip is as wide as its tabs at their most and the add button, and gives way where it has less room. In
    // the list mode the layout knows how wide the tabs are, whatever they show.
    let mut root = Row::new()
      .part(&self.part(WindowTabsPart::Root))
      .align_items(Alignment::Center);
    if self.overflow == WindowTabsOverflow::Scroll || self.drag.is_some() {
      root = root.width(self.full_width());
    }
    let mut root = root
      .flex_shrink(1.0)
      .min_width(0.0)
      // Never wider than what holds it: a row gives way by the shrink, a column by this.
      .max_width(Dimension::Pct(100.0))
      .ref_element(self.kept.root.clone())
      .child(list);
    if let Some(add) = self.add_control() {
      // The separator and the add button are one thing after the list, the root's gap between them too.
      let mut end = Row::new()
        .align_items(Alignment::Center)
        .flex_shrink(0.0)
        .spacing(self.root_gap());
      if self.separator_before_add() {
        let shown = self.last_plain();
        end = end.child(
          Row::new()
            .opacity(if shown { 1.0 } else { 0.0 })
            .child(self.separator()),
        );
      }
      root = root.child(end.child(add));
    }
    if !self.disabled && self.shortcuts {
      root = root.on_key_down(self.shortcut());
    }
    if let Some(keys) = self.list_keys() {
      root = root.on_key_down(keys);
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> crate::ui::Styled for WindowTabs<T> {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

/// Every window tabs have the one recipe and renderer, whatever their values are.
impl<T: Send + Sync + 'static> Themed for WindowTabs<T> {
  const NAME: &'static str = "window_tabs";
  type Renderer = WindowTabsRenderer;

  fn parts() -> crate::ui::Declared {
    WindowTabsPart::declared()
  }

  fn states() -> crate::ui::Declared {
    WindowTabsState::declared()
  }

  /// The design's window tabs (`pencil/lurq-design-system.pen`, Window Tabs).
  ///
  /// - The strip is 32 high, its tabs abutting. A tab is 128 by 32, round by 8 at its top corners and square at its
  ///   bottom, padded 10 at its start and 6 at its end, its parts 8 apart: an icon of 14, the label 12 in the regular
  ///   weight taking the rest, and a close slot of 18, round by the small radius, with an x of 12 in the muted
  ///   foreground.
  /// - The selected tab is the background's color, the content's, edged at its top and sides by a line of 1 in the
  ///   border's color and open at its bottom, its icon and label in the foreground. Another has no fill, its icon and
  ///   label in the muted foreground and its x at 0.6; under the pointer it is the secondary hover fill and its x
  ///   whole.
  /// - A tab with unsaved changes has a dot of 7 in the foreground in its close slot, the x in its place under the
  ///   pointer.
  /// - A separator is 1 by 16 in the border's color, between two tabs that are not the selected one and before the add
  ///   button. The add button is 30 by 28, round by the small radius, with a plus of 14 in the foreground.
  /// - Where the board is silent: a tab is at least 72 wide (the icon and the close slot); the close slot and the add
  ///   button under the pointer are filled (muted, and the secondary hover); the ring of the focus is inside a tab and
  ///   the add button; a disabled tab is dimmed. A tab of its icon alone is 28 wide with no padding; the dot after a
  ///   label and the app's marks are 6 from the label, the marks 4 apart. The More button of the list mode is 28 high,
  ///   padded 8 across, its count 12 and a chevron of 12, 4 apart, in the muted foreground, the foreground and the
  ///   secondary hover under the pointer, and pressed while its menu is open.
  /// - The list of every tab is a panel 320 wide in the popover's colors, edged by 1 in the border's color, with the
  ///   large radius and shadow, padded 6, its search field 4 above its rows, which are 2 apart. A row is 28 high, round
  ///   by the small radius, padded 8 at its start and 4 at its end, its parts 8 apart: an icon of 14 and the label 12
  ///   in the muted foreground, a mark of 12 for a tab not in the strip, a check of 14 on the selected tab, and a close
  ///   button of 20 whose x of 12 shows while the row is under the pointer or focused. The selected tab's row has the
  ///   fill of what is picked out and its label 500 in the foreground; another is muted under the pointer and focused,
  ///   with the ring inside it then. Where nothing matches, 12 in the muted foreground, padded 12 and 8. Under the
  ///   search field (`searchable`) the rows abut. A row's quiet text after its label is 12 in the muted foreground; the
  ///   app's elements in it are 4 apart.
  fn recipe() -> Recipe {
    use PaletteColor::{Background, Foreground, Muted, MutedForeground, Ring, SecondaryHover};
    use WindowTabsPart::{
      Add, AddIcon, Close, CloseIcon, Dot, Empty, Icon, Item, ItemClose, ItemCloseIcon, ItemCurrent, ItemDetail,
      ItemHidden, ItemIcon, ItemLabel, ItemMarks, Items, Label, List, Marks, More, MoreIcon, MoreLabel, Panel, Root,
      Search, Separator, Tab, Title,
    };
    use WindowTabsState::{Dirty, Disabled, IconOnly, MenuOpen, Searchable, Selected};
    let inside = || Outline::new(BorderSize::Md, Ring).offset(-2.0);
    let clear = Color::new(0, 0, 0, 0);
    let edge = || crate::node::border::Border::inside(BorderSize::Sm, PaletteColor::Border);
    Recipe::new()
      .base(Root, |s| s.height(32.0))
      .base(List, |s| s.height(32.0))
      .base(Tab, |s| {
        fading(
          s.min_width(72.0)
            .max_width(128.0)
            .height(32.0)
            .corners(8.0, 8.0, 0.0, 0.0)
            .padding_start(10.0)
            .padding_end(6.0)
            .gap(8.0)
            .foreground(MutedForeground)
            .cursor(CursorIcon::Default),
        )
      })
      .base(Icon, |s| s.square(IconSize::Sm))
      .base(Label, |s| s.typography(TypographyStyle::Caption))
      .base(Title, |s| s.gap(6.0))
      .base(Marks, |s| s.gap(4.0))
      .base(Close, |s| {
        fading(s.square(18.0).radius(RadiusSize::Sm).cursor(CursorIcon::Default))
      })
      .base(CloseIcon, |s| {
        s.square(IconSize::Xs).foreground(MutedForeground).opacity(0.6)
      })
      .base(Dot, |s| s.square(7.0).radius(RadiusSize::Full).background(Foreground))
      .base(Separator, |s| {
        s.width(1.0).height(16.0).background(PaletteColor::Border)
      })
      .base(Add, |s| {
        fading(
          s.width(30.0)
            .height(28.0)
            .radius(RadiusSize::Sm)
            .foreground(Foreground)
            .cursor(CursorIcon::Default),
        )
      })
      .base(AddIcon, |s| s.square(IconSize::Sm))
      .base(More, |s| {
        fading(
          s.height(28.0)
            .padding_x(8.0)
            .gap(4.0)
            .radius(RadiusSize::Sm)
            .foreground(MutedForeground)
            .cursor(CursorIcon::Default),
        )
      })
      .base(MoreLabel, |s| s.typography(TypographyStyle::Caption))
      .base(MoreIcon, |s| s.square(IconSize::Xs))
      .hovered(Tab, |s| s.background(SecondaryHover))
      .hovered(Close, |s| s.background(Muted))
      .hovered(Add, |s| s.background(SecondaryHover))
      .hovered(More, |s| s.background(SecondaryHover).foreground(Foreground))
      .focus_visible(Tab, |s| s.outline(inside()))
      .focus_visible(Add, |s| s.outline(inside()))
      .focus_visible(More, |s| s.outline(inside()))
      // A tab of its icon alone is as wide as a square of the strip's height less its corners, the icon in its middle.
      .state(IconOnly, Tab, |s| {
        s.width(28.0)
          .min_width(28.0)
          .max_width(28.0)
          .padding_start(0.0)
          .padding_end(0.0)
      })
      .within_hovered(CloseIcon, |s| s.opacity(1.0))
      // The selected tab is open to the content under it and edged on its other sides by a line of the border: on a
      // bar of the background's white (the light palette's `Bar`) its fill alone would not tell it apart.
      .state(Selected, Tab, |s| {
        s.background(Background)
          .foreground(Foreground)
          .top_line(edge())
          .start_line(edge())
          .end_line(edge())
      })
      .when(Selector::new().state(Selected).hovered(), Tab, |s| {
        s.background(Background)
      })
      .state(Selected, CloseIcon, |s| s.opacity(1.0))
      // A tab with unsaved changes shows its dot, and the x in its place under the pointer.
      .state(Dirty, CloseIcon, |s| s.opacity(0.0))
      .when(Selector::new().state(Dirty).within().hovered(), CloseIcon, |s| {
        s.opacity(1.0)
      })
      .when(Selector::new().state(Dirty).within().hovered(), Dot, |s| s.opacity(0.0))
      .state(Disabled, Tab, |s| s.opacity(OpacityLevel::Disabled))
      .when(Selector::new().state(Disabled).hovered(), Tab, |s| s.background(clear))
      .when(Selector::new().state(Selected).state(Disabled).hovered(), Tab, |s| {
        s.background(Background)
      })
      .state(Disabled, Add, |s| s.opacity(OpacityLevel::Disabled))
      .when(Selector::new().state(Disabled).hovered(), Add, |s| s.background(clear))
      .state(Disabled, More, |s| s.opacity(OpacityLevel::Disabled))
      .when(Selector::new().state(Disabled).hovered(), More, |s| {
        s.background(clear).foreground(MutedForeground)
      })
      .state(MenuOpen, More, |s| {
        s.background(SecondaryHover)
          .veil(PaletteColor::StatePressed)
          .foreground(Foreground)
      })
      .base(Panel, |s| {
        s.width(320.0)
          .padding_x(6.0)
          .padding_y(6.0)
          .radius(RadiusSize::Lg)
          .background(PaletteColor::Popover)
          .foreground(PaletteColor::PopoverForeground)
          .border(crate::node::border::Border::inside(BorderSize::Sm, PaletteColor::Border))
          .shadow(ShadowStyle::Lg)
          .enter(PartAnimation::new(FADE_IN, MotionDuration::Base, MotionEasing::Enter))
          .exit(PartAnimation::new(FADE_OUT, MotionDuration::Fast, MotionEasing::Exit))
      })
      .base(Search, |s| s.padding_bottom(4.0))
      .base(Items, |s| s.gap(2.0))
      .state(Searchable, Items, |s| s.gap(0.0))
      .base(Item, |s| {
        fading(
          s.height(28.0)
            .padding_start(8.0)
            .padding_end(4.0)
            .gap(8.0)
            .radius(RadiusSize::Sm)
            .foreground(MutedForeground)
            .cursor(CursorIcon::Default),
        )
      })
      .hovered(Item, |s| s.background(Muted))
      .focus_visible(Item, |s| s.background(Muted).outline(inside()))
      .state(Selected, Item, |s| {
        s.background(PaletteColor::Highlight).foreground(Foreground)
      })
      .when(Selector::new().state(Selected).hovered(), Item, |s| {
        s.background(PaletteColor::Highlight)
      })
      .state(Disabled, Item, |s| s.opacity(OpacityLevel::Disabled))
      .base(ItemIcon, |s| s.square(IconSize::Sm))
      .base(ItemLabel, |s| s.typography(TypographyStyle::Caption))
      .state(Selected, ItemLabel, |s| s.weight(FontWeight::Medium))
      .base(ItemDetail, |s| {
        s.typography(TypographyStyle::Caption).foreground(MutedForeground)
      })
      .base(ItemMarks, |s| s.gap(4.0))
      .base(ItemHidden, |s| s.square(IconSize::Xs).foreground(MutedForeground))
      .base(ItemCurrent, |s| s.square(IconSize::Sm).foreground(Foreground))
      .base(ItemClose, |s| {
        fading(s.square(20.0).radius(RadiusSize::Sm).cursor(CursorIcon::Default))
      })
      .hovered(ItemClose, |s| s.background(SecondaryHover))
      .base(ItemCloseIcon, |s| {
        s.square(IconSize::Xs).foreground(MutedForeground).opacity(0.0)
      })
      .within(ItemCloseIcon, |s| s.opacity(1.0))
      .base(Empty, |s| {
        s.padding_x(12.0)
          .padding_y(8.0)
          .typography(TypographyStyle::Caption)
          .foreground(MutedForeground)
      })
  }
}
