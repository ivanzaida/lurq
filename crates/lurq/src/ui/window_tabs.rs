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
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  select::px,
  shared::{fading, glyph, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{DragEvent, KeyboardEvent, MouseButton, MouseButtonMask, MouseEvent},
    theme::{BorderSize, IconSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{self, Row, ScrollBoth, Stack},
  core::{ElementRef, Signal, SignalValue},
  layout::{
    Alignment,
    layout_kind::{Justify, ScrollState},
    scrollbar::ScrollBarStyle,
  },
  node::{
    CursorIcon, Element, FocusGroup, color::Color, dimension::Dimension, node_kind::TextOverflow, outline::Outline,
  },
};

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
}

names!(WindowTabsPart {
  Root => "root",
  List => "list",
  Tab => "tab",
  Icon => "icon",
  Label => "label",
  Close => "close",
  CloseIcon => "close_icon",
  Dot => "dot",
  Separator => "separator",
  Add => "add",
  AddIcon => "add_icon",
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
}

names!(WindowTabsState {
  Selected => "selected",
  Dirty => "dirty",
  Dragged => "dragged",
  Disabled => "disabled",
});

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
  on_close: Vec<Heard>,
  disabled: bool,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<T> WindowTab<T> {
  /// A tab of `value`, called `label`.
  pub fn new(value: T, label: &str) -> Self {
    Self {
      value,
      label: Arc::from(label),
      icon: None,
      dirty: false,
      close_button: true,
      on_close: Vec::new(),
      disabled: false,
      id: None,
      key: None,
    }
  }

  /// An icon before the label, by its name in the theme's set.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// Whether what the tab holds has changes that are not saved: its close slot shows a dot, and the x in its place
  /// while the pointer is on the tab.
  pub fn dirty(mut self, dirty: bool) -> Self {
    self.dirty = dirty;
    self
  }

  /// Whether the tab has a close button: shown unless told otherwise. A tab without one is not closed by a person,
  /// by the keys either.
  pub fn close_button(mut self, shown: bool) -> Self {
    self.close_button = shown;
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

  /// The id of the tab: what is pressed. Its close button has this id with `-close` after it.
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
      on_close: self.on_close,
      disabled: self.disabled,
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
  pub disabled: bool,
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
  on_reorder: Vec<Moved>,
  on_detach: Vec<Handler<T>>,
  shortcuts: bool,
  disabled: bool,
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
      on_reorder: Vec::new(),
      on_detach: Vec::new(),
      shortcuts: true,
      disabled: false,
      label: None,
      words: Words {
        add: super::current_locale().navigation.new_tab.clone(),
        close: super::current_locale().common.close.clone(),
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

  /// The id of the strip. Its list of tabs has this id with `-list` after it, and its add button with `-add`.
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
          disabled: tab.disabled,
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
      reorder,
      shortcuts: self.shortcuts,
      disabled: self.disabled,
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
  /// How many tabs there are.
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

  /// The place the tab is let go at: that of the tab whose room it is over most.
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
  reorder: Moved,
  shortcuts: bool,
  disabled: bool,
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
  fn build(self, theme: &Theme, kept: Kept) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(WindowTabsState::Disabled, self.disabled);
    let view = WindowTabsView {
      tabs: self.tabs,
      disabled: self.disabled,
      shortcuts: self.shortcuts,
      label: self.label,
      theme: theme.clone(),
      case,
      rules: self.rules,
      recipes: std::array::from_fn(|_| OnceCell::new()),
      add: self.add,
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
    tabs.build(&current_theme(), self.kept.clone())
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
  pub label: Option<Arc<str>>,
  theme: Theme,
  case: Case,
  rules: Recipe,
  /// The recipe of a tab in each mix of its own states, resolved when it is first asked for.
  recipes: [OnceCell<ResolvedRecipe>; 16],
  add: Option<Heard>,
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
  fn recipe(&self, selected: bool, dirty: bool, dragged: bool, disabled: bool) -> &ResolvedRecipe {
    let index =
      usize::from(selected) | usize::from(dirty) << 1 | usize::from(dragged) << 2 | usize::from(disabled) << 3;
    self.recipes[index].get_or_init(|| {
      self.theme.resolve::<WindowTabs>(
        &self
          .case
          .clone()
          .state(WindowTabsState::Selected, selected)
          .state(WindowTabsState::Dirty, dirty)
          .state(WindowTabsState::Dragged, dragged)
          .state(WindowTabsState::Disabled, disabled),
        Some(&self.rules),
      )
    })
  }

  /// The style of a part that is no tab's, as these tabs are to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe(false, false, false, self.disabled).part(part)
  }

  /// The style of a part of the tab at `index`: in the states of that tab, and in the interaction forced where
  /// it is the tab Tab stops on.
  pub fn tab_part(&self, index: usize, part: impl Into<Name>) -> ResolvedPart {
    let tab = &self.tabs[index];
    let dragged = self.drag.is_some_and(|drag| drag.from == index);
    let forced = self.forced.filter(|_| self.entry() == Some(index));
    self
      .recipe(tab.selected, tab.dirty, dragged, self.disabled || tab.disabled)
      .part(part)
      .forced(forced)
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

  /// Whether the add button is shown: the app is told when it is pressed.
  pub fn has_add(&self) -> bool {
    self.add.is_some()
  }

  /// The index of the tab being dragged, where the tabs are mounted and a drag has begun.
  pub fn dragged(&self) -> Option<usize> {
    self.drag.map(|drag| drag.from)
  }

  /// Whether a tab can be closed by a person: it has a close button, and neither it nor the strip is disabled.
  fn closable(&self, index: usize) -> bool {
    let tab = &self.tabs[index];
    tab.close_button && !tab.disabled && !self.disabled
  }

  /// The close slot of the tab at `index`: its close button, which shows the
  /// dot of unsaved changes until the pointer is on the tab; or the dot
  /// alone where the tab has no close button. None where it has neither.
  /// The button is for the pointer, called "Close" and the tab's label to
  /// what reads the tree: the keyboard closes the tab with Delete.
  pub fn close(&self, index: usize) -> Option<Element> {
    let entry = &self.tabs[index];
    let part = |part: WindowTabsPart| self.tab_part(index, part);
    let centred = |child: Element| -> Element {
      Row::new()
        .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
        .justify(Justify::Center)
        .align_items(Alignment::Center)
        .child(child)
        .into()
    };
    let dot = || centred(Row::new().part(&part(WindowTabsPart::Dot)).into());
    // A tab drawn as under the pointer is not its close slot under the pointer: the slot keeps its own state.
    let slot_part = part(WindowTabsPart::Close).forced(None);
    if !entry.close_button {
      if !entry.dirty {
        return None;
      }
      return Some(
        Row::new()
          .part(&slot_part)
          .justify(Justify::Center)
          .align_items(Alignment::Center)
          .child(dot())
          .into(),
      );
    }
    let x = centred(glyph(&part(WindowTabsPart::CloseIcon), Name::new(icons::CLOSE), false));
    let slot: Element = if entry.dirty {
      Stack::new()
        .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
        .child(x)
        .child(dot())
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

  /// The tab at `index`: what is pressed, with its icon, label and close
  /// slot. It is a tab to what reads the tree, called by its label, and says
  /// whether it is the selected one; the list's arrow keys move to it unless
  /// it is disabled, and Delete on it closes it. A press goes no further than
  /// it, and a drag moves it along the strip or out of it.
  pub fn tab(&self, index: usize) -> components::Button {
    let entry = &self.tabs[index];
    let part = |part: WindowTabsPart| self.tab_part(index, part);
    let disabled = self.disabled || entry.disabled;
    let (select, kept) = (entry.select.clone(), self.kept.clone());
    let mut tab = pressable(&part(WindowTabsPart::Tab), disabled)
      .justify(Justify::Start)
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
    if !disabled {
      tab = self.draggable(index, tab);
      let closable = self.closable(index);
      let (close, kept) = (entry.close.clone(), self.kept.clone());
      tab = tab.on_key_down(move |event: KeyboardEvent| {
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
      });
      if closable {
        // A press of the middle button on the tab, let go on it, closes it.
        let (pressed, released) = (self.kept.middle.clone(), self.kept.middle.clone());
        let close = entry.close.clone();
        tab = tab
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
          });
      }
    }
    if let Some(name) = entry.icon {
      tab = tab.child(glyph(&part(WindowTabsPart::Icon), name, false));
    }
    let label = part(WindowTabsPart::Label);
    // The label takes the room the icon and the close slot leave, and ends in an ellipsis where it is not enough.
    tab = tab.child(
      Row::new().flex_full(1.0, 1.0, Some(0.0)).min_width(0.0).child(
        label
          .text(&entry.label)
          .nowrap()
          .text_overflow(TextOverflow::Elipsis)
          .part(&label),
      ),
    );
    if let Some(close) = self.close(index) {
      tab = tab.child(close);
    }
    if let Some(id) = &entry.id {
      tab = tab.id(id.clone());
    }
    if let Some(key) = &entry.key {
      tab = tab.key(key.clone());
    }
    // It shares the room of the list with the other tabs: from its least width, growing alike with them to its
    // most.
    tab.flex_full(1.0, 0.0, Some(self.widths().0))
  }

  /// The widths of a tab in the recipe: the least, and the most, which is what a tab is with room enough.
  fn widths(&self) -> (f32, f32) {
    let tab = self.part(WindowTabsPart::Tab).rest();
    let most = px(tab.max_width, &self.theme).or_else(|| px(tab.width, &self.theme));
    let least = px(tab.min_width, &self.theme).unwrap_or(0.0);
    (least, most.unwrap_or(least).max(least))
  }

  /// What a press and a move of the tab at `index` do: past a few pixels it is a drag, which selects the tab and
  /// draws it under the pointer where the tabs are mounted; let go, it tells where the tab went.
  fn draggable(&self, index: usize, tab: components::Button) -> components::Button {
    let count = self.tabs.len();
    let kept = self.kept.clone();
    let (start, moved, end) = (kept.clone(), kept.clone(), kept);
    let select = self.tabs[index].select.clone();
    let detach = self.tabs[index].detach.clone();
    let reorder = self.reorder.clone();
    tab
      .start_drag_buttons(MouseButtonMask::LEFT)
      .on_drag_start(move |_: DragEvent| {
        start.dragged.store(false, Ordering::Relaxed);
        let at = start.tab_ref(index);
        let refs = start.tab_refs.lock();
        let list_x = start.list.x();
        let span = refs
          .get(count.saturating_sub(1))
          .map(|last| last.x() + last.width() - list_x)
          .unwrap_or(0.0);
        *start.drag.lock() = Some(Drag {
          from: index,
          count,
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
        let to = Drag {
          moved: event.total_delta_x,
          ..drag
        }
        .target();
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
  /// the strip's `add_label` to what reads the tree. A press of it goes no further than it.
  pub fn add(&self) -> Option<components::Button> {
    let add = self.add.clone()?;
    let mut button = pressable(&self.part(WindowTabsPart::Add), self.disabled)
      .describe("a11y_name", self.words.add.clone())
      .align_items(Alignment::Center)
      .on_mouse_down(|event: MouseEvent| event.stop_propagation())
      .on_click(move |_| add())
      .child(glyph(
        &self.part(WindowTabsPart::AddIcon),
        Name::new(icons::PLUS),
        false,
      ));
    if let Some(id) = &self.id {
      button = button.id(format!("{id}-add"));
    }
    Some(button)
  }

  /// What stands in the list in order: the tabs, and while a tab is dragged the others with the room it would be
  /// let go into among them.
  fn places(&self) -> Vec<Place> {
    let mut places: Vec<Place> = (0..self.tabs.len()).map(Place::Tab).collect();
    if let Some(drag) = self.drag {
      places.retain(|place| *place != Place::Tab(drag.from));
      places.insert(drag.target().min(places.len()), Place::Room);
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

  /// The list of the tabs with the separators between them: one stop of Tab,
  /// at the selected tab, whose Left and Right select. It is a tab list to
  /// what reads the tree. While a tab is dragged, the others make room for it
  /// where it would be let go, and it is drawn over them under the pointer.
  pub fn list(&self) -> Element {
    let places = self.places();
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
      if at > 0 && self.plain(places[at - 1]) && self.plain(*place) {
        row = row.child(self.separator());
      }
      row = match place {
        Place::Tab(index) => row.child(self.tab(*index)),
        Place::Room => row.child(Row::new().part(&self.part(WindowTabsPart::Tab)).flex_full(
          1.0,
          0.0,
          Some(self.widths().0),
        )),
      };
    }
    let Some(drag) = self.drag else {
      return row.into();
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
    move |event: KeyboardEvent| {
      if event.default_prevented() || event.composing || event.alt || tabs.is_empty() {
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

  /// How the default renderer builds window tabs: the list, which scrolls
  /// sideways where its tabs do not fit at their least width, and the add
  /// button after it, with a separator before it where the last tab is not
  /// the selected one. The window's keys are heard on the strip.
  pub fn render(self) -> Element {
    // The tabs share the room the strip is given, down to their least width, and past that the list scrolls.
    // A wheel turned up and down over it scrolls it sideways, as a browser's strip of tabs.
    let scroll = ScrollBoth::new(self.list())
      .fill_width(true)
      .wheel_across(true)
      .with_scroll_state(self.kept.scroll.clone())
      .scrollbar(ScrollBarStyle::hidden())
      .flex_full(1.0, 1.0, Some(0.0))
      .min_width(0.0);
    // The strip is as wide as its tabs at their most and the add button, and gives way where it has less room.
    let places = self.places();
    let separator = px(self.part(WindowTabsPart::Separator).rest().width, &self.theme).unwrap_or(0.0);
    let separators = (1..places.len())
      .filter(|at| self.plain(places[at - 1]) && self.plain(places[*at]))
      .count();
    let add = self.add.as_ref().map_or(0.0, |_| {
      let before = if self.tabs.last().is_some_and(|tab| !tab.selected) {
        separator
      } else {
        0.0
      };
      before + px(self.part(WindowTabsPart::Add).rest().width, &self.theme).unwrap_or(0.0)
    });
    let full = match self.drag {
      Some(drag) => drag.strip_width,
      None => places.len() as f32 * self.widths().1 + separators as f32 * separator + add,
    };
    let mut root = Row::new()
      .part(&self.part(WindowTabsPart::Root))
      .align_items(Alignment::Center)
      .width(full)
      .flex_shrink(1.0)
      .min_width(0.0)
      // Never wider than what holds it: a row gives way by the shrink, a column by this.
      .max_width(Dimension::Pct(100.0))
      .ref_element(self.kept.root.clone())
      .child(scroll);
    if let Some(add) = self.add() {
      if self.tabs.last().is_some_and(|tab| !tab.selected) {
        root = root.child(self.separator());
      }
      root = root.child(add);
    }
    if !self.disabled && self.shortcuts {
      root = root.on_key_down(self.shortcut());
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
  /// - The selected tab is the background's color, the content's, its icon and label in the foreground. Another has no
  ///   fill, its icon and label in the muted foreground and its x at 0.6; under the pointer it is the secondary hover
  ///   fill and its x whole.
  /// - A tab with unsaved changes has a dot of 7 in the foreground in its close slot, the x in its place under the
  ///   pointer.
  /// - A separator is 1 by 16 in the border's color, between two tabs that are not the selected one and before the add
  ///   button. The add button is 30 by 28, round by the small radius, with a plus of 14 in the foreground.
  /// - Where the board is silent: a tab is at least 72 wide (the icon and the close slot); the close slot and the add
  ///   button under the pointer are filled (muted, and the secondary hover); the ring of the focus is inside a tab and
  ///   the add button; a disabled tab is dimmed.
  fn recipe() -> Recipe {
    use PaletteColor::{Background, Foreground, Muted, MutedForeground, Ring, SecondaryHover};
    use WindowTabsPart::{Add, AddIcon, Close, CloseIcon, Dot, Icon, Label, List, Root, Separator, Tab};
    use WindowTabsState::{Dirty, Disabled, Selected};
    let inside = || Outline::new(BorderSize::Md, Ring).offset(-2.0);
    let clear = Color::new(0, 0, 0, 0);
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
      .base(Close, |s| {
        fading(s.square(18.0).radius(RadiusSize::Sm).cursor(CursorIcon::Default))
      })
      .base(CloseIcon, |s| {
        s.square(IconSize::Xs).foreground(MutedForeground).opacity(0.6)
      })
      .base(Dot, |s| s.square(7.0).radius(RadiusSize::Full).background(Foreground))
      .base(Separator, |s| s.width(1.0).height(16.0).background(PaletteColor::Border))
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
      .hovered(Tab, |s| s.background(SecondaryHover))
      .hovered(Close, |s| s.background(Muted))
      .hovered(Add, |s| s.background(SecondaryHover))
      .focus_visible(Tab, |s| s.outline(inside()))
      .focus_visible(Add, |s| s.outline(inside()))
      .within_hovered(CloseIcon, |s| s.opacity(1.0))
      .state(Selected, Tab, |s| s.background(Background).foreground(Foreground))
      .when(Selector::new().state(Selected).hovered(), Tab, |s| s.background(Background))
      .state(Selected, CloseIcon, |s| s.opacity(1.0))
      // A tab with unsaved changes shows its dot, and the x in its place under the pointer.
      .state(Dirty, CloseIcon, |s| s.opacity(0.0))
      .when(Selector::new().state(Dirty).within().hovered(), CloseIcon, |s| s.opacity(1.0))
      .when(Selector::new().state(Dirty).within().hovered(), Dot, |s| s.opacity(0.0))
      .state(Disabled, Tab, |s| s.opacity(OpacityLevel::Disabled))
      .when(Selector::new().state(Disabled).hovered(), Tab, |s| s.background(clear))
      .when(Selector::new().state(Selected).state(Disabled).hovered(), Tab, |s| s.background(Background))
      .state(Disabled, Add, |s| s.opacity(OpacityLevel::Disabled))
      .when(Selector::new().state(Disabled).hovered(), Add, |s| s.background(clear))
  }
}
