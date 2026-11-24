//! Document tabs: the open documents of an editor in a bar across its top, one of them the one shown.

use std::{
  collections::HashMap,
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
  },
};

use super::{
  context_menu::ContextMenu,
  icons,
  menu::{Menu, MenuItem},
  names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{fading, glyph, one_line, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::{Ctx, Placement},
    events::{DragEvent, KeyboardEvent, MouseButton, MouseButtonMask, MouseEvent},
    theme::{BorderSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{self, Row, Stack},
  core::{ElementRef, Signal, SignalValue},
  layout::{
    Alignment,
    layout_kind::{Justify, ShrinkLimit},
    text_style::{FontStyle, FontWeight},
  },
  node::{CursorIcon, Element, FocusGroup, border::Border, dimension::Dimension, outline::Outline},
};

/// How far the pointer moves with a tab held before the tab is being dragged: a press that shakes is a press.
const DRAG_START: f32 = 4.0;

/// The parts of document tabs a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DocumentTabsPart {
  /// The bar: its height.
  Root,
  /// The box under the tabs: the bar's fill and the line along the bottom, which the selected tab covers.
  Bar,
  /// The row of the tabs, the new tab button and the More button.
  List,
  /// One tab: what is pressed.
  Tab,
  /// The icon of a tab, before its name; a pinned tab's pin.
  Icon,
  /// The name of a tab.
  Label,
  /// The close control at the end of a tab.
  Close,
  /// The cross of the close control.
  CloseIcon,
  /// The dot in place of the cross of a document that is not saved.
  Dirty,
  /// The button after the tabs that opens a new one.
  Add,
  /// The plus of the new tab button.
  AddIcon,
  /// The button at the end that opens the menu of the tabs that do not fit.
  More,
  /// How many tabs the More button holds.
  MoreLabel,
  /// The chevron of the More button.
  MoreIcon,
}

names!(DocumentTabsPart {
  Root => "root",
  Bar => "bar",
  List => "list",
  Tab => "tab",
  Icon => "icon",
  Label => "label",
  Close => "close",
  CloseIcon => "close_icon",
  Dirty => "dirty",
  Add => "add",
  AddIcon => "add_icon",
  More => "more",
  MoreLabel => "more_label",
  MoreIcon => "more_icon",
});

/// The states of document tabs a recipe styles, besides what the pointer and the keyboard do to them. All but
/// `Overflowing` are a tab's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DocumentTabsState {
  /// The tab of the document shown.
  Selected,
  /// The document has changes that are not saved: a dot in place of the close control.
  Dirty,
  /// The tab is kept at the start, as its icon alone.
  Pinned,
  /// The tab is a preview, which the next document opened takes the place of: its name in italics.
  Preview,
  /// The tab takes no input.
  Disabled,
  /// The tab is being dragged to another place.
  Dragged,
  /// The dragged tab goes before this one.
  DropBefore,
  /// The dragged tab goes after this one, the last.
  DropAfter,
  /// Some tabs do not fit, and are in the menu of the More button.
  Overflowing,
}

names!(DocumentTabsState {
  Selected => "selected",
  Dirty => "dirty",
  Pinned => "pinned",
  Preview => "preview",
  Disabled => "disabled",
  Dragged => "dragged",
  DropBefore => "drop_before",
  DropAfter => "drop_after",
  Overflowing => "overflowing",
});

/// What builds document tabs' nodes: a theme's in place of the default one.
pub type DocumentTabsRenderer = Arc<dyn Fn(DocumentTabsView) -> Element + Send + Sync>;

/// What is told the new value of the tabs: `T`, or `Option<T>`.
type Handler<V> = Arc<dyn Fn(&V) + Send + Sync>;
/// What is told a tab's value: closing it, keeping a preview.
type Told<T> = Arc<dyn Fn(&T) + Send + Sync>;
/// What is told that the tab at the first index goes to the second.
type Reorder = Arc<dyn Fn(usize, usize) + Send + Sync>;
/// What is called with nothing to tell.
type Call = Arc<dyn Fn() + Send + Sync>;
/// What the context menu of a tab holds, asked for each time it is built.
type Items = Arc<dyn Fn() -> Vec<MenuItem> + Send + Sync>;

/// One open document: its value, its name, and what it says of the document.
///
/// ```
/// use lurq::ui::{DocumentTab, icons};
///
/// let tab = DocumentTab::new("src/blur.rs", "blur.rs").icon(icons::FILE_CODE).dirty(true);
/// let preview = DocumentTab::new("README.md", "README.md").preview(true);
/// ```
pub struct DocumentTab<T> {
  value: T,
  label: Arc<str>,
  icon: Option<Name>,
  dirty: bool,
  pinned: bool,
  preview: bool,
  disabled: bool,
  on_close: Vec<Call>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<T> DocumentTab<T> {
  /// The tab of `value`, called `label`: the document's name.
  pub fn new(value: T, label: &str) -> Self {
    Self {
      value,
      label: Arc::from(label),
      icon: None,
      dirty: false,
      pinned: false,
      preview: false,
      disabled: false,
      on_close: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// An icon before the name, by its name in the theme's set: the kind of the document.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// Whether the document has changes that are not saved: a dot in place of the close control, which shows again
  /// under the pointer.
  pub fn dirty(mut self, dirty: bool) -> Self {
    self.dirty = dirty;
    self
  }

  /// Whether the tab is pinned: a square of the pin at the start, with no close control, which is never put in
  /// the menu of what does not fit. The app keeps its pinned tabs first; a drag moves a tab among its kind.
  pub fn pinned(mut self, pinned: bool) -> Self {
    self.pinned = pinned;
    self
  }

  /// Whether the tab is a preview, its name in italics: the app opens the next document in its place. A double
  /// press of it tells [`DocumentTabs::on_keep`].
  pub fn preview(mut self, preview: bool) -> Self {
    self.preview = preview;
    self
  }

  /// Whether the tab takes no input: it is dimmed, and the arrow keys pass it by.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Called when a person closes this tab, before the bar's [`on_close`](DocumentTabs::on_close). A second
  /// handler is called after the first.
  pub fn on_close(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_close.push(Arc::new(handler));
    self
  }

  /// The id of the tab.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// The key of the tab, by which the focus follows it when it moves: its name and how many before it have the
  /// same unless told.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }
}

impl<T> From<(T, &str)> for DocumentTab<T> {
  fn from((value, label): (T, &str)) -> Self {
    Self::new(value, label)
  }
}

/// The words document tabs write, each of which an app can say otherwise.
#[derive(Clone)]
struct Words {
  close: Arc<str>,
  add: Arc<str>,
  more: Arc<str>,
}

/// The open documents of an editor, in a bar across its top: one of them is the one shown, a press of another shows
/// it, and each closes by its control.
///
/// ```
/// use lurq::{core::Signal, ui::{DocumentTab, DocumentTabs, icons}};
///
/// # fn editor(open: Signal<&'static str>) {
/// let tabs = DocumentTabs::new(open, [
///   DocumentTab::new("blur.rs", "blur.rs").icon(icons::FILE_CODE),
///   DocumentTab::new("theme.rs", "theme.rs").icon(icons::FILE_CODE).dirty(true),
///   DocumentTab::new("README.md", "README.md").icon(icons::FILE_TEXT).preview(true),
/// ])
/// .on_close(|path| println!("close {path}"))
/// .on_reorder(|from, to| println!("move {from} to {to}"))
/// .on_add(|| println!("new"));
/// # }
/// ```
///
/// Bound to a signal ([`new`](Self::new), [`optional`](Self::optional), where no document may be shown) it sets it,
/// or the app holds which is shown and is told what it is to be ([`controlled`](Self::controlled),
/// [`controlled_optional`](Self::controlled_optional)). [`on_change`](Self::on_change) is told the new value: `&T`
/// of a `DocumentTabs<T>`, `&Option<T>` of a `DocumentTabs<T, Option<T>>`.
///
/// The tabs are one stop of Tab, at the one shown; Left and Right move between them, and Space and Enter show the
/// one focused. A press of a tab's close control, a press of it with the middle button and Ctrl+W on it close it:
/// the bar tells [`on_close`](Self::on_close), and the app takes it out. A tab dragged along the bar, and
/// Ctrl+Shift+PageUp or PageDown on it, move it: [`on_reorder`](Self::on_reorder). Mounted
/// ([`mount`](Self::mount)), the tabs that do not fit leave the bar from its end, but the one shown and the pinned
/// ones, into the menu of the More button at the end, which says how many it holds.
pub struct DocumentTabs<T = (), V: ?Sized = T> {
  tabs: Vec<DocumentTab<T>>,
  selected: Option<T>,
  write: Option<Arc<dyn Fn(&T) + Send + Sync>>,
  tell: fn(&T, &[Handler<V>]),
  on_change: Vec<Handler<V>>,
  on_close: Vec<Told<T>>,
  on_keep: Vec<Told<T>>,
  on_reorder: Vec<Reorder>,
  on_drag_away: Vec<Arc<dyn Fn(&T, &DragEvent) + Send + Sync>>,
  on_add: Vec<Call>,
  context_menu: Option<Arc<dyn Fn(&T) -> Vec<MenuItem> + Send + Sync>>,
  label: Option<Arc<str>>,
  words: Words,
  disabled: bool,
  shortcuts: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> DocumentTabs<T, T> {
  /// Tabs of which the one whose value the signal has is shown. A press of another sets the signal.
  pub fn new(value: Signal<T>, tabs: impl IntoIterator<Item = impl Into<DocumentTab<T>>>) -> Self
  where
    T: SignalValue,
  {
    let selected = Some(value.get());
    Self::of(
      tabs,
      selected,
      Some(Arc::new(move |next: &T| value.set(next.clone()))),
      Self::tell_one,
    )
  }

  /// Tabs of which the one of `value` is shown, the app's: a press of another tells `on_change` the value it is to
  /// be, and shows nothing itself.
  pub fn controlled(
    value: T,
    tabs: impl IntoIterator<Item = impl Into<DocumentTab<T>>>,
    on_change: impl Fn(&T) + Send + Sync + 'static,
  ) -> Self {
    Self::of(tabs, Some(value), None, Self::tell_one).on_change(on_change)
  }

  fn tell_one(value: &T, handlers: &[Handler<T>]) {
    for handler in handlers {
      handler(value);
    }
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> DocumentTabs<T, Option<T>> {
  /// Tabs of which the one whose value the signal has is shown, or none: an editor with no document open, or none
  /// shown yet. A press of a tab sets the signal.
  pub fn optional(value: Signal<Option<T>>, tabs: impl IntoIterator<Item = impl Into<DocumentTab<T>>>) -> Self
  where
    Option<T>: SignalValue,
  {
    let selected = value.get();
    Self::of(
      tabs,
      selected,
      Some(Arc::new(move |next: &T| value.set(Some(next.clone())))),
      Self::tell_optional,
    )
  }

  /// Tabs of which the one of `value` is shown, or none, the app's: a press of a tab tells `on_change` the value it
  /// is to be.
  pub fn controlled_optional(
    value: Option<T>,
    tabs: impl IntoIterator<Item = impl Into<DocumentTab<T>>>,
    on_change: impl Fn(&Option<T>) + Send + Sync + 'static,
  ) -> Self {
    Self::of(tabs, value, None, Self::tell_optional).on_change(on_change)
  }

  fn tell_optional(value: &T, handlers: &[Handler<Option<T>>]) {
    let value = Some(value.clone());
    for handler in handlers {
      handler(&value);
    }
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> DocumentTabs<T, V> {
  fn of(
    tabs: impl IntoIterator<Item = impl Into<DocumentTab<T>>>,
    selected: Option<T>,
    write: Option<Arc<dyn Fn(&T) + Send + Sync>>,
    tell: fn(&T, &[Handler<V>]),
  ) -> Self {
    Self {
      tabs: tabs.into_iter().map(Into::into).collect(),
      selected,
      write,
      tell,
      on_change: Vec::new(),
      on_close: Vec::new(),
      on_keep: Vec::new(),
      on_reorder: Vec::new(),
      on_drag_away: Vec::new(),
      on_add: Vec::new(),
      context_menu: None,
      label: None,
      words: Words {
        close: super::current_locale().common.close.clone(),
        add: super::current_locale().navigation.new_tab.clone(),
        more: super::current_locale().navigation.show_hidden_tabs.clone(),
      },
      disabled: false,
      shortcuts: true,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// A tab, after those it has.
  pub fn tab(mut self, tab: impl Into<DocumentTab<T>>) -> Self {
    self.tabs.push(tab.into());
    self
  }

  /// Tabs, after those it has.
  pub fn tabs(mut self, tabs: impl IntoIterator<Item = impl Into<DocumentTab<T>>>) -> Self {
    self.tabs.extend(tabs.into_iter().map(Into::into));
    self
  }

  /// Called when a person shows another tab, after the signal is set where the tabs are bound, with the new value.
  /// Not when the app sets the signal, nor for a press of the tab shown. A second handler is called after the
  /// first.
  pub fn on_change(mut self, handler: impl Fn(&V) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Called with the value of the tab a person closes, after its own handlers: by its close control, by a press of
  /// it with the middle button, by Ctrl+W while it is focused. The app takes it out of the tabs, and says which is
  /// shown where it was the one shown. A second handler is called after the first.
  pub fn on_close(mut self, handler: impl Fn(&T) + Send + Sync + 'static) -> Self {
    self.on_close.push(Arc::new(handler));
    self
  }

  /// Called with the value of a preview tab a person presses twice: the app keeps the document open, as a tab that
  /// is no preview.
  pub fn on_keep(mut self, handler: impl Fn(&T) + Send + Sync + 'static) -> Self {
    self.on_keep.push(Arc::new(handler));
    self
  }

  /// Called when a person moves a tab: from where it is to where it is to be, both counted in the tabs as they are
  /// before and after the move (`tabs.insert(to, tabs.remove(from))`). By a drag along the bar, of mounted tabs, and
  /// by Ctrl+Shift+PageUp and PageDown on a focused tab. A pinned tab moves among the pinned ones, and another
  /// among the others. The bar moves nothing itself.
  pub fn on_reorder(mut self, handler: impl Fn(usize, usize) + Send + Sync + 'static) -> Self {
    self.on_reorder.push(Arc::new(handler));
    self
  }

  /// Called while a person drags a tab away from the bar, of mounted tabs: with each move of the pointer while it is
  /// farther from the bar than the bar is high (or outside the window), and with the last event when the tab is let
  /// go there, whose `drop_result` is `Some`. While it is away the bar shows no place among the tabs and moves
  /// nothing when it is let go: where it goes is the app's (a dock layout docks it elsewhere). Back over the bar, it
  /// is a drag along the bar again.
  pub fn on_drag_away(mut self, handler: impl Fn(&T, &DragEvent) + Send + Sync + 'static) -> Self {
    self.on_drag_away.push(Arc::new(handler));
    self
  }

  /// Called on a press of the new tab button after the tabs, which is there once this is given.
  pub fn on_add(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_add.push(Arc::new(handler));
    self
  }

  /// The commands of a tab's context menu, for its value, which a press of a tab with the secondary button, a long
  /// press, the Menu key and Shift+F10 open on mounted tabs: Close Others, Copy Path, Reveal.
  pub fn context_menu(mut self, items: impl Fn(&T) -> Vec<MenuItem> + Send + Sync + 'static) -> Self {
    self.context_menu = Some(Arc::new(items));
    self
  }

  /// What the tabs are called to what reads the tree: "Open files".
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// What a tab's close control is called to what reads the tree, before the tab's name: "Close" unless told.
  pub fn close_label(mut self, label: &str) -> Self {
    self.words.close = Arc::from(label);
    self
  }

  /// What the new tab button is called: "New tab" unless told.
  pub fn add_label(mut self, label: &str) -> Self {
    self.words.add = Arc::from(label);
    self
  }

  /// What the More button is called to what reads the tree: "Show the hidden tabs" unless told. It shows how many it
  /// holds.
  pub fn more_label(mut self, label: &str) -> Self {
    self.words.more = Arc::from(label);
    self
  }

  /// Whether no tab takes input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Whether the editor's keys work from anywhere in the window, the focus in the document or elsewhere: Ctrl+Tab
  /// and Ctrl+Shift+Tab (and on Windows and Linux Ctrl+PageDown and Ctrl+PageUp) show the next tab and the one
  /// before, round at the ends; Ctrl+W (Cmd+W on macOS) closes the tab shown, but a pinned one;
  /// Ctrl+Shift+PageDown and PageUp move it. On unless told otherwise. Where the window's title bar has
  /// [`WindowTabs`](super::WindowTabs) with its keys on too, a press is the documents': the window's tabs pass by
  /// a key these took. An app that wants the window's to have them turns these off.
  pub fn shortcuts(mut self, on: bool) -> Self {
    self.shortcuts = on;
    self
  }

  /// Puts the bar and each of its tabs in a state the theme's recipe names, or takes them out.
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

  /// Draws the tab Tab stops on (the one shown, else the first that is not disabled) as it is under the pointer,
  /// held down or focused from the keyboard, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the bar. Its tabs have none unless given; its buttons have ids after it: `{id}-add`, `{id}-more`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// The key of the bar, among what is beside it.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The tabs with what each press does, of no type of the app's.
  fn prepare(self) -> Prepared {
    let handlers: Arc<[Handler<V>]> = self.on_change.into();
    let on_close: Arc<[Told<T>]> = self.on_close.into();
    let on_keep: Arc<[Told<T>]> = self.on_keep.into();
    let on_drag_away: Arc<[Arc<dyn Fn(&T, &DragEvent) + Send + Sync>]> = self.on_drag_away.into();
    let (write, tell) = (self.write, self.tell);
    let mut seen: HashMap<Arc<str>, usize> = HashMap::new();
    let tabs = self
      .tabs
      .into_iter()
      .map(|tab| {
        let selected = self.selected.as_ref() == Some(&tab.value);
        let key = tab.key.unwrap_or_else(|| {
          let count = seen.entry(tab.label.clone()).or_insert(0);
          *count += 1;
          Arc::from(format!("document-tab-{}-{count}", tab.label))
        });
        let select: Call = {
          let (value, write, handlers) = (tab.value.clone(), write.clone(), handlers.clone());
          Arc::new(move || {
            if selected {
              return;
            }
            if let Some(write) = &write {
              write(&value);
            }
            tell(&value, &handlers);
          })
        };
        let close: Call = {
          let (value, own, told) = (tab.value.clone(), tab.on_close, on_close.clone());
          Arc::new(move || {
            for handler in &own {
              handler();
            }
            for handler in told.iter() {
              handler(&value);
            }
          })
        };
        let keep: Call = {
          let (value, told) = (tab.value.clone(), on_keep.clone());
          Arc::new(move || {
            for handler in told.iter() {
              handler(&value);
            }
          })
        };
        let away: Option<Away> = (!on_drag_away.is_empty()).then(|| {
          let (value, told) = (tab.value.clone(), on_drag_away.clone());
          Arc::new(move |event: &DragEvent| {
            for handler in told.iter() {
              handler(&value, event);
            }
          }) as Away
        });
        let context: Option<Items> = self.context_menu.clone().map(|items| {
          let value = tab.value.clone();
          Arc::new(move || items(&value)) as Items
        });
        DocumentTabEntry {
          label: tab.label,
          icon: tab.icon,
          dirty: tab.dirty,
          pinned: tab.pinned,
          preview: tab.preview,
          disabled: tab.disabled,
          selected,
          select,
          close,
          keep,
          away,
          context,
          id: tab.id,
          key,
        }
      })
      .collect();
    Prepared {
      tabs,
      on_reorder: self.on_reorder.into(),
      on_add: self.on_add.into(),
      label: self.label,
      words: self.words,
      disabled: self.disabled,
      shortcuts: self.shortcuts,
      states: self.states,
      rules: self.rules,
      forced: self.forced,
      id: self.id,
      key: self.key,
    }
  }

  /// Puts the tabs in the tree of `ctx`'s component, in the theme of what is being rendered: they keep which tabs
  /// did not fit and the menu of them, a tab being dragged, and each tab's context menu.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let mut tabs = self.prepare();
    let key = tabs.key.take();
    // Keyed, it keeps its host where what is before it comes and goes.
    let props = DocumentTabsProps { tabs };
    match key {
      Some(key) => ctx.mount_keyed::<DocumentTabsHost>(&key, props).key(key),
      None => ctx.mount::<DocumentTabsHost>(props),
    }
  }

  /// Builds the tabs in `theme`, with nothing kept: every tab in the bar, whether it fits or not, with no drag and
  /// no context menu. Converting tabs into an element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    self.prepare().build(theme, None)
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> From<DocumentTabs<T, V>> for Element {
  fn from(tabs: DocumentTabs<T, V>) -> Self {
    tabs.build(&current_theme())
  }
}

/// A tab as a renderer sees it: what it shows, and what a press of it, of its close control and a double press do.
#[derive(Clone)]
pub struct DocumentTabEntry {
  pub label: Arc<str>,
  pub icon: Option<Name>,
  pub dirty: bool,
  pub pinned: bool,
  pub preview: bool,
  pub disabled: bool,
  pub selected: bool,
  select: Call,
  close: Call,
  keep: Call,
  away: Option<Away>,
  context: Option<Items>,
  pub id: Option<Arc<str>>,
  pub key: Arc<str>,
}

/// Document tabs as they are built, once each press knows what it does.
#[derive(Clone)]
struct Prepared {
  tabs: Vec<DocumentTabEntry>,
  on_reorder: Arc<[Reorder]>,
  on_add: Arc<[Call]>,
  label: Option<Arc<str>>,
  words: Words,
  disabled: bool,
  shortcuts: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

/// A tab being dragged: where it was, where it is to go, whether the pointer has moved far enough to say so, and
/// whether it is away from the bar, where the app has it.
#[derive(Clone, Copy, Debug, Default, PartialEq, crate::DevtoolsInspectable)]
struct Drag {
  from: usize,
  to: usize,
  moved: bool,
  away: bool,
}

/// What hears a tab dragged away from the bar.
type Away = Arc<dyn Fn(&DragEvent) + Send + Sync>;

/// What mounted tabs keep from one render to the next.
#[derive(Clone)]
struct Kept {
  /// Which tabs the last layout dropped, by index.
  dropped: Signal<Vec<bool>>,
  /// Whether the menu of the More button is open, and what it hangs from.
  open: Signal<bool>,
  anchor: ElementRef,
  /// Where each tab was laid out, by index: what a drag finds its place by, and how the bar learns what dropped.
  refs: Arc<Mutex<Vec<ElementRef>>>,
  /// The tab being dragged.
  drag: Signal<Option<Drag>>,
  /// Whether each tab's context menu is open, by the tab's key.
  menus: Arc<Mutex<HashMap<Arc<str>, Signal<bool>>>>,
  /// The key of the tab the focus goes to after the next render: one chosen from the menu of what did not fit,
  /// which was not in the bar to be focused.
  focus: Signal<Option<Arc<str>>>,
}

impl Kept {
  fn refs(&self, count: usize) -> Vec<ElementRef> {
    let mut refs = self.refs.lock().unwrap();
    while refs.len() < count {
      refs.push(ElementRef::new());
    }
    refs[..count].to_vec()
  }

  fn menu(&self, key: &Arc<str>) -> Signal<bool> {
    self
      .menus
      .lock()
      .unwrap()
      .entry(key.clone())
      .or_insert_with(|| Signal::new(false))
      .clone()
  }
}

impl Prepared {
  fn build(self, theme: &Theme, kept: Option<Kept>) -> Element {
    let breakpoint = super::current_breakpoint();
    let count = self.tabs.len();
    let dropped: Vec<bool> = match &kept {
      Some(kept) => {
        let read = kept.dropped.get();
        (0..count)
          .map(|index| read.get(index).copied().unwrap_or(false) && droppable(&self.tabs[index]))
          .collect()
      }
      None => vec![false; count],
    };
    let drag = kept.as_ref().and_then(|kept| kept.drag.get()).filter(|drag| drag.moved);
    let overflowing = dropped.contains(&true);
    let case = Case {
      breakpoint,
      states: self.states.clone(),
      ..Case::new()
    }
    .state(DocumentTabsState::Overflowing, overflowing);
    let recipe = theme
      .resolve::<DocumentTabs>(&case, Some(&self.rules))
      .owned_by(self.id.as_ref());
    let recipe_of_bar = &recipe;
    let tab_recipes = self
      .tabs
      .iter()
      .enumerate()
      .map(|(index, tab)| {
        let (before, after) = match drag {
          Some(drag) => drop_place(&self.tabs, drag, index),
          None => (false, false),
        };
        let recipe = theme.resolve::<DocumentTabs>(
          &case
            .clone()
            .state(DocumentTabsState::Selected, tab.selected)
            .state(DocumentTabsState::Dirty, tab.dirty)
            .state(DocumentTabsState::Pinned, tab.pinned)
            .state(DocumentTabsState::Preview, tab.preview)
            .state(DocumentTabsState::Disabled, self.disabled || tab.disabled)
            .state(DocumentTabsState::Dragged, drag.is_some_and(|drag| drag.from == index))
            .state(DocumentTabsState::DropBefore, before)
            .state(DocumentTabsState::DropAfter, after),
          Some(&self.rules),
        );
        // A tab with an id names its parts after it, one without after the bar's id and its key.
        let recipe = recipe.owned_like(recipe_of_bar);
        match &tab.id {
          Some(id) => recipe.item(Some(id), DocumentTabsPart::Tab),
          None => recipe.keyed_item(&tab.key, DocumentTabsPart::Tab),
        }
      })
      .collect();
    let refs = kept.as_ref().map(|kept| kept.refs(count)).unwrap_or_default();
    let view = DocumentTabsView {
      tabs: self.tabs,
      dropped,
      label: self.label,
      disabled: self.disabled,
      shortcuts: self.shortcuts,
      recipe,
      tab_recipes,
      words: self.words,
      on_reorder: self.on_reorder,
      on_add: self.on_add,
      kept,
      refs,
      forced: self.forced,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<DocumentTabs>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

/// Whether a tab may leave the bar for the menu: not the one shown, nor a pinned one.
fn droppable(tab: &DocumentTabEntry) -> bool {
  !tab.selected && !tab.pinned
}

/// The tabs a tab at `from` may move among: the pinned ones, or the others, as indices from and to.
fn kind_range(tabs: &[DocumentTabEntry], from: usize) -> (usize, usize) {
  let pinned = tabs[from].pinned;
  let first = tabs.iter().position(|tab| tab.pinned == pinned).unwrap_or(0);
  let last = tabs.iter().rposition(|tab| tab.pinned == pinned).unwrap_or(from);
  (first, last)
}

/// Whether the dragged tab goes before the tab at `index`, or after it where it is the last of the others.
fn drop_place(tabs: &[DocumentTabEntry], drag: Drag, index: usize) -> (bool, bool) {
  if drag.away || drag.to == drag.from || index == drag.from {
    return (false, false);
  }
  // The others, as they are without the dragged tab: the dragged one goes before the one at `to` among them.
  let others: Vec<usize> = (0..tabs.len()).filter(|other| *other != drag.from).collect();
  match others.get(drag.to) {
    Some(next) => (*next == index, false),
    None => (false, others.last() == Some(&index)),
  }
}

#[derive(Clone)]
struct DocumentTabsProps {
  tabs: Prepared,
}

/// The tabs are prepared anew by every render of what holds them, so props are never the same as before.
impl PartialEq for DocumentTabsProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl crate::app::component::DevtoolsInspectable for DocumentTabsProps {}

/// What mounted document tabs keep from one render to the next.
struct DocumentTabsHost {
  kept: Kept,
}

impl Component for DocumentTabsHost {
  type Props = DocumentTabsProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      kept: Kept {
        dropped: ctx.signal(Vec::new()),
        open: ctx.signal(false),
        anchor: ElementRef::new(),
        refs: Arc::new(Mutex::new(Vec::new())),
        drag: ctx.signal(None),
        menus: Arc::new(Mutex::new(HashMap::new())),
        focus: ctx.signal(None),
      },
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let DocumentTabsProps { tabs } = ctx.props::<DocumentTabsProps>().clone();
    // A tab chosen from the menu comes back into the bar: the focus goes to it once the layout has it there (a
    // dropped tab takes no focus), after the menu has given it back to the More button as it closed.
    if let Some(key) = self.kept.focus.get() {
      match tabs.tabs.iter().position(|tab| tab.key == key) {
        Some(index) if self.kept.dropped.get().get(index).copied().unwrap_or(false) => {}
        Some(index) => {
          self.kept.focus.set(None);
          ctx.focus(&self.kept.refs(tabs.tabs.len())[index]);
        }
        None => self.kept.focus.set(None),
      }
    }
    tabs.build(&current_theme(), Some(self.kept.clone()))
  }
}

/// Document tabs as a renderer sees them: the tabs, the style of their parts, and their behavior ready to be dressed.
pub struct DocumentTabsView {
  pub tabs: Vec<DocumentTabEntry>,
  /// Which tabs are in the menu of the More button, by index.
  pub dropped: Vec<bool>,
  pub label: Option<Arc<str>>,
  pub disabled: bool,
  /// Whether the editor's keys work from anywhere in the window.
  pub shortcuts: bool,
  /// The recipe of every part that is no tab's.
  recipe: ResolvedRecipe,
  /// Each tab's recipe, in its own states.
  tab_recipes: Vec<ResolvedRecipe>,
  words: Words,
  on_reorder: Arc<[Reorder]>,
  on_add: Arc<[Call]>,
  kept: Option<Kept>,
  refs: Vec<ElementRef>,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl DocumentTabsView {
  /// The style of a part that is no tab's.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  /// The style of a part of the tab at `index`, in its states, and in the interaction forced where it is the tab Tab
  /// stops on.
  pub fn tab_part(&self, index: usize, part: impl Into<Name>) -> ResolvedPart {
    let forced = self.forced.filter(|_| self.entry() == Some(index));
    self.tab_recipes[index].part(part).forced(forced)
  }

  /// The tab Tab stops on: the one shown, else the first that is not disabled. None where all are.
  pub fn entry(&self) -> Option<usize> {
    if self.disabled {
      return None;
    }
    let open = |tab: &DocumentTabEntry| !tab.disabled;
    self
      .tabs
      .iter()
      .position(|tab| tab.selected && open(tab))
      .or_else(|| self.tabs.iter().position(open))
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// Tells the bar's handlers that the tab at `from` goes to `to`, where that moves it.
  fn reorder_call(&self) -> impl Fn(usize, usize) + Send + Sync + 'static {
    let handlers = self.on_reorder.clone();
    move |from, to| {
      if from != to {
        for handler in handlers.iter() {
          handler(from, to);
        }
      }
    }
  }

  /// The close control of the tab at `index`: a cross, or the dot of a document not saved, which the cross takes
  /// the place of under the pointer. It is not a stop of Tab: Ctrl+W closes the tab focused.
  fn close(&self, index: usize) -> components::Button {
    let entry = &self.tabs[index];
    let close = entry.close.clone();
    let icon = glyph(
      &self.tab_part(index, DocumentTabsPart::CloseIcon),
      Name::new(icons::CLOSE),
      false,
    );
    let mut button = components::Button::empty()
      .focusable(false)
      .part(&self.tab_part(index, DocumentTabsPart::Close))
      .describe("a11y_name", format!("{} {}", self.words.close, entry.label))
      .disabled(self.disabled || entry.disabled)
      .on_click(move |event: MouseEvent| {
        event.stop_propagation();
        close();
      });
    if entry.dirty {
      // The dot and the cross in one place, the middle of the control, the recipe showing one of them.
      let fill = Dimension::Pct(100.0);
      let middle = |child: Element| {
        Row::new()
          .size(fill, fill)
          .align_items(Alignment::Center)
          .justify(Justify::Center)
          .child(child)
      };
      let dot = Row::new().part(&self.tab_part(index, DocumentTabsPart::Dirty));
      button = button.child(
        Stack::new()
          .size(fill, fill)
          .child(middle(icon))
          .child(middle(dot.into())),
      );
    } else {
      button = button.child(icon);
    }
    if let Some(id) = &entry.id {
      button = button.id(format!("{id}-close"));
    }
    button
  }

  /// The tab at `index`: its icon and name and its close control, or a pinned tab's pin. It is a tab to what
  /// reads the tree, called by its name, and says whether it is the one shown.
  pub fn tab(&self, index: usize) -> Element {
    let entry = &self.tabs[index];
    let part = |part: DocumentTabsPart| self.tab_part(index, part);
    let disabled = self.disabled || entry.disabled;
    let (select, close, keep) = (entry.select.clone(), entry.close.clone(), entry.keep.clone());
    let mut tab = pressable(&part(DocumentTabsPart::Tab), disabled)
      .interaction_group(true)
      .focus_entry(entry.selected)
      .key(entry.key.clone())
      .describe("a11y_role", "tab")
      .describe("a11y_name", entry.label.clone())
      .describe("a11y_selected", if entry.selected { "true" } else { "false" })
      .on_click(move |event: MouseEvent| {
        if event.button == MouseButton::Left {
          select();
        }
      });
    if !disabled {
      let middle = close.clone();
      tab = tab.on_mouse_up(move |event: MouseEvent| {
        if event.button == MouseButton::Middle {
          middle();
        }
      });
      if entry.preview {
        // A finger's double tap is no double click: it keeps the tab as one does.
        let tapped = keep.clone();
        tab = tab
          .on_dblclick(move |_: MouseEvent| keep())
          .on_double_tap(move |event: crate::app::events::TapEvent| {
            if event.fingers == 1 {
              tapped();
            }
          });
      }
      tab = tab.on_key_down(self.keys(index));
    }
    if entry.pinned {
      tab = tab.child(glyph(&part(DocumentTabsPart::Icon), Name::new(icons::PIN), false));
    } else {
      if let Some(name) = entry.icon {
        tab = tab.child(glyph(&part(DocumentTabsPart::Icon), name, false));
      }
      tab = tab.child(one_line(&part(DocumentTabsPart::Label), &entry.label));
      tab = tab.child(self.close(index));
    }
    if let Some(id) = &entry.id {
      tab = tab.id(id.clone());
    }
    if let Some(at) = self.refs.get(index) {
      tab = tab.ref_element(at.clone());
    }
    if let Some(kept) = &self.kept
      && !disabled
    {
      tab = self.draggable(tab, kept, index);
    }
    let mut element: Element = match (&self.kept, &entry.context) {
      (Some(kept), Some(items)) if !disabled => ContextMenu::new(kept.menu(&entry.key), tab)
        .label(&entry.label)
        .items(items())
        .into(),
      _ => tab.into(),
    };
    if let Some(kept) = &self.kept
      && let Some(at) = self.refs.get(index)
    {
      element = self.place(element, kept, at.clone(), index, droppable(entry));
    }
    element
  }

  /// What the keys do on the tab at `index`: Ctrl+W closes it (but a pinned one) and sends the focus to the tab
  /// beside it, Ctrl+Shift+PageUp and PageDown move it.
  fn keys(&self, index: usize) -> impl Fn(KeyboardEvent) + Send + Sync + 'static {
    let entry = &self.tabs[index];
    let close = entry.close.clone();
    let own = entry.key.clone();
    let beside = self
      .tabs
      .get(index + 1)
      .or_else(|| index.checked_sub(1).and_then(|before| self.tabs.get(before)))
      .map(|tab| tab.key.clone());
    let (first, last) = kind_range(&self.tabs, index);
    let reorder = self.reorder_call();
    let pinned = entry.pinned;
    move |event: KeyboardEvent| {
      // A key reaches every tab: it is the focused one's, where nothing took it before.
      if !event.focus_within || event.default_prevented() {
        return;
      }
      let command = event.ctrl || event.meta;
      if command && !event.shift && !event.alt && event.key.eq_ignore_ascii_case("w") {
        // A pinned tab has no close control: the keys do not close it either.
        if pinned {
          return;
        }
        event.prevent_default();
        close();
        if let Some(beside) = &beside {
          event.focus_key(beside.clone());
        }
        return;
      }
      if event.ctrl && event.shift && !event.alt {
        let to = match event.key.as_str() {
          "PageUp" if index > first => index - 1,
          "PageDown" if index < last => index + 1,
          "PageUp" | "PageDown" => {
            event.prevent_default();
            return;
          }
          _ => return,
        };
        reorder(index, to);
        // The focus stays on the tab moved, at its new place.
        event.focus_key(own.clone());
      }
    }
  }

  /// The tab at `index` as a drag moves it along the bar: where it is to go follows the pointer among the tabs of
  /// its kind, and is told when it is let go.
  fn draggable(&self, tab: components::Button, kept: &Kept, index: usize) -> components::Button {
    let (first, last) = kind_range(&self.tabs, index);
    let refs = self.refs.clone();
    let (start, moving, ending) = (kept.drag.clone(), kept.drag.clone(), kept.drag.clone());
    let reorder = self.reorder_call();
    let away = self.tabs[index].away.clone();
    let (away_moving, away_ending) = (away.clone(), away);
    let (bar, bar_end) = (refs.clone(), refs.clone());
    // Away from the bar: farther from it than it is high, or outside the window; only where the app hears it.
    let far = move |refs: &[ElementRef], event: &DragEvent| {
      let Some(at) = refs.get(index) else {
        return false;
      };
      let (top, height) = (at.y(), at.height());
      event.outside_window || event.y < top - height || event.y > top + 2.0 * height
    };
    tab
      .start_drag_buttons(MouseButtonMask::LEFT)
      .on_drag_start(move |_: DragEvent| {
        start.set(Some(Drag {
          from: index,
          to: index,
          moved: false,
          away: false,
        }))
      })
      .on_drag_move(move |event: DragEvent| {
        let Some(mut drag) = moving.get_untracked() else {
          return;
        };
        drag.moved |= event.total_delta_x.abs() >= DRAG_START || event.total_delta_y.abs() >= DRAG_START;
        drag.away = away_moving.is_some() && far(&bar, &event);
        if drag.away {
          if let Some(away) = &away_moving {
            away(&event);
          }
          if moving.get_untracked() != Some(drag) {
            moving.set(Some(drag));
          }
          return;
        }
        // Where it goes: before the first of the others whose middle is past the pointer, among its kind.
        let mut to = 0;
        for (other, at) in refs.iter().enumerate() {
          if other != index && at.x() + at.width() / 2.0 < event.x {
            to += 1;
          }
        }
        // In the tabs without it, its kind runs from `first` to `last - 1`.
        drag.to = to.clamp(first, last);
        if moving.get_untracked() != Some(drag) {
          moving.set(Some(drag));
        }
      })
      .on_drag_end(move |event: DragEvent| {
        let drag = ending.get_untracked();
        ending.set(None);
        if let Some(away) = &away_ending
          && (drag.is_some_and(|drag| drag.away) || far(&bar_end, &event))
        {
          away(&event);
          return;
        }
        if let Some(drag) = drag
          && drag.moved
        {
          reorder(drag.from, drag.to);
        }
      })
  }

  /// A tab in its place in the bar, the same element whether it may leave the bar or not, so that the tab keeps the
  /// focus as it becomes the one shown. One that `drops` leaves the bar, the last first, where the bar is too
  /// narrow, and says so through its ref.
  fn place(&self, element: Element, kept: &Kept, at: ElementRef, index: usize, drops: bool) -> Element {
    let (dropped, open, count) = (kept.dropped.clone(), kept.open.clone(), self.tabs.len());
    at.observe_rect(move |rect| {
      let now = rect.width <= 0.0 && rect.height <= 0.0;
      if dropped.get_untracked().get(index).copied() != Some(now) {
        dropped.update(|dropped| {
          dropped.resize(count.max(dropped.len()), false);
          dropped[index] = now;
        });
        // With every tab back in the bar the More button is gone, and its menu with it.
        if !dropped.get_untracked().contains(&true) && open.get_untracked() {
          open.set(false);
        }
      }
    });
    let place = Row::new().align_items(Alignment::Stretch).overflow_visible();
    let place = if drops {
      place.flex_shrink(1.0).shrink_limit(ShrinkLimit::Drop)
    } else {
      place
    };
    place.child(element).into()
  }

  /// The new tab button, where the bar has `on_add`.
  pub fn add(&self) -> Option<components::Button> {
    if self.on_add.is_empty() {
      return None;
    }
    let handlers = self.on_add.clone();
    let mut button = pressable(&self.part(DocumentTabsPart::Add), self.disabled)
      .describe("a11y_name", self.words.add.clone())
      .child(glyph(
        &self.part(DocumentTabsPart::AddIcon),
        Name::new(icons::PLUS),
        false,
      ))
      .on_click(move |_| {
        for handler in handlers.iter() {
          handler();
        }
      });
    if let Some(id) = &self.id {
      button = button.id(format!("{id}-add"));
    }
    Some(button)
  }

  /// The More button and the menu of the tabs that did not fit, of mounted tabs where some did not: how many there
  /// are, and a chevron.
  pub fn more(&self) -> Option<(components::Button, Menu)> {
    let kept = self.kept.as_ref()?;
    let hidden = self.dropped.iter().filter(|dropped| **dropped).count();
    if hidden == 0 {
      return None;
    }
    let open = kept.open.clone();
    let label = self.part(DocumentTabsPart::MoreLabel);
    let mut button = pressable(&self.part(DocumentTabsPart::More), self.disabled)
      .ref_element(kept.anchor.clone())
      .describe("a11y_name", self.words.more.clone())
      .describe("a11y_haspopup", "menu")
      .describe("a11y_expanded", if open.get() { "true" } else { "false" })
      .child(label.text(&hidden.to_string()).nowrap().part(&label))
      .child(glyph(
        &self.part(DocumentTabsPart::MoreIcon),
        Name::new(icons::CHEVRON_DOWN),
        false,
      ))
      .on_click(move |_| open.set(!open.get_untracked()));
    if let Some(id) = &self.id {
      button = button.id(format!("{id}-more"));
    }
    let mut menu = Menu::new(kept.open.clone(), kept.anchor.clone())
      .placement(Placement::BottomEnd)
      .label(&self.words.more);
    for (index, entry) in self.tabs.iter().enumerate() {
      if !self.dropped[index] {
        continue;
      }
      let (select, focus, key) = (entry.select.clone(), kept.focus.clone(), entry.key.clone());
      let mut item = MenuItem::new(&entry.label)
        .disabled(self.disabled || entry.disabled)
        .on_select(move || {
          select();
          focus.set(Some(key.clone()));
        });
      if let Some(icon) = entry.icon {
        item = item.icon(icon);
      }
      if let Some(id) = &entry.id {
        item = item.id(format!("{id}-item"));
      }
      menu = menu.item(item);
    }
    Some((button, menu))
  }

  /// The row of the tabs: one stop of Tab, at the one shown, whose arrow keys move the focus and Space and Enter
  /// show the tab focused; then the new tab button, a stop of its own. It is a tab list to what reads the tree. It
  /// takes the room the More button leaves, and its tabs leave it where they do not fit.
  pub fn list(&self) -> Element {
    let mut list = Row::new()
      .part(&self.part(DocumentTabsPart::List))
      .flex(1.0)
      .align_items(Alignment::Stretch)
      .focus_group(FocusGroup::horizontal().activate(false).one_stop(true))
      .describe("a11y_role", "tablist")
      .describe("a11y_name", self.label.clone().unwrap_or_default())
      .describe("a11y_orientation", "horizontal");
    for index in 0..self.tabs.len() {
      list = list.child(self.tab(index));
    }
    // What is beside the tabs is no tab: a stop of Tab of its own, and not one the arrows move to.
    if let Some(add) = self.add() {
      list = list.child(
        Row::new()
          .focus_group(FocusGroup::none())
          .align_items(Alignment::Stretch)
          .child(add),
      );
    }
    list.into()
  }

  /// What the editor's keys do from anywhere in the window: show the next tab and the one before, close the one
  /// shown, move it. `in_bar` says whether the focus is in the bar, which the bar's root learns first; where it is,
  /// the focus follows the tab shown.
  fn shortcut(&self, in_bar: Arc<AtomicBool>) -> impl Fn(KeyboardEvent) + Send + Sync + 'static {
    // Of each tab: whether it is the one shown, whether it can be shown, whether it is pinned, what shows it, what
    // closes it, its key.
    let tabs: Vec<(bool, bool, bool, Call, Call, Arc<str>)> = self
      .tabs
      .iter()
      .map(|tab| {
        (
          tab.selected,
          !tab.disabled,
          tab.pinned,
          tab.select.clone(),
          tab.close.clone(),
          tab.key.clone(),
        )
      })
      .collect();
    let ranges: Vec<(usize, usize)> = (0..self.tabs.len())
      .map(|index| kind_range(&self.tabs, index))
      .collect();
    let reorder = self.reorder_call();
    move |event: KeyboardEvent| {
      // A key a focused tab, or anything before, took is not the window's.
      if event.default_prevented() || event.composing || event.alt || tabs.is_empty() {
        return;
      }
      let mac = cfg!(target_os = "macos");
      let at = tabs.iter().position(|tab| tab.0);
      let key = event.key.as_str();
      let (select, close, moves) = match key {
        "Tab" if event.ctrl && !event.meta => (Some(if event.shift { -1 } else { 1 }), false, None),
        "PageDown" | "PageUp" if event.ctrl && !event.meta => {
          let by: isize = if key == "PageDown" { 1 } else { -1 };
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
      let follow = in_bar.load(Ordering::Relaxed);
      if let Some(by) = select {
        // The next tab that can be shown, `by` one way or the other, round at the ends.
        let count = tabs.len() as isize;
        let from = at.map(|at| at as isize).unwrap_or(if by > 0 { -1 } else { count });
        let next = (1..=count)
          .map(|n| (from + by * n).rem_euclid(count) as usize)
          .find(|index| tabs[*index].1 && Some(*index) != at);
        if let Some(next) = next {
          (tabs[next].3)();
          if follow {
            event.focus_key(tabs[next].5.clone());
          }
        }
      } else if close {
        if let Some(at) = at.filter(|at| tabs[*at].1 && !tabs[*at].2) {
          (tabs[at].4)();
          let beside = tabs
            .get(at + 1)
            .or_else(|| at.checked_sub(1).and_then(|before| tabs.get(before)));
          if let (true, Some(beside)) = (follow, beside) {
            event.focus_key(beside.5.clone());
          }
        }
      } else if let (Some(by), Some(from)) = (moves, at) {
        let (first, last) = ranges[from];
        let to = from as isize + by;
        if to >= first as isize && to <= last as isize {
          reorder(from, to as usize);
          if follow {
            event.focus_key(tabs[from].5.clone());
          }
        }
      }
    }
  }

  /// How the default renderer builds the tabs: the bar's box, with its line, under the row of tabs, so that the
  /// tab shown covers the line. The editor's keys are heard after the tabs, so that a focused tab's own come first.
  pub fn render(self) -> Element {
    let fill = Dimension::Pct(100.0);
    let mut row = Row::new()
      .size(fill, fill)
      .align_items(Alignment::Stretch)
      .child(self.list());
    if let Some((more, menu)) = self.more() {
      row = row.child(more).child(menu);
    }
    let mut root = Stack::new()
      .part(&self.part(DocumentTabsPart::Root))
      .child(Row::new().size(fill, fill).part(&self.part(DocumentTabsPart::Bar)))
      .child(row);
    if self.shortcuts && !self.disabled {
      // Keys reach every element in the order of the tree: the root learns whether the focus is in the bar, the
      // tabs act on their own keys, and the element after them on the window's.
      let in_bar = Arc::new(AtomicBool::new(false));
      let heard = in_bar.clone();
      root = root
        .on_key_down(move |event: KeyboardEvent| heard.store(event.focus_within, Ordering::Relaxed))
        .child(Row::new().on_key_down(self.shortcut(in_bar)));
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

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> crate::ui::Styled for DocumentTabs<T, V> {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

/// Every set of document tabs has the one recipe and renderer, whatever its values are.
impl<T: Send + Sync + 'static, V: ?Sized + 'static> Themed for DocumentTabs<T, V> {
  const NAME: &'static str = "document_tabs";
  type Renderer = DocumentTabsRenderer;

  fn parts() -> crate::ui::Declared {
    DocumentTabsPart::declared()
  }

  fn states() -> crate::ui::Declared {
    DocumentTabsState::declared()
  }

  /// The design's document tabs (`pencil/lurq-design-system.pen`, Document Tabs): a bar 38 high in the bar's fill
  /// over a line of the border's color. A tab is padded 12 at its start and 8 at its end, 8 between its icon of 14,
  /// its name of 13 and its close control of 20, of the small radius, with a cross of 13; a line of the border's
  /// color at its end. The tab shown is the background's color with a line of 2 in the primary color along its top,
  /// over the bar's line, its icon, name and cross in the foreground; another's are in the muted foreground, its
  /// cross at half. A tab under the pointer shows its cross whole, and the close control under the pointer is on the
  /// secondary hover fill. A document not saved has a dot of 8 in the foreground in place of the cross. A preview's
  /// name is in italics. A pinned tab is a square of 38 with its pin. The new tab button is a square of 38 with a
  /// plus of 16; the More button at the end is padded 10, its count of 12 and a chevron of 14 in the muted
  /// foreground 4 apart, a line at its start.
  fn recipe() -> Recipe {
    use DocumentTabsPart::{
      Add, AddIcon, Bar, Close, CloseIcon, Dirty, Icon, Label, List, More, MoreIcon, MoreLabel, Root, Tab,
    };
    use DocumentTabsState::{Disabled, Dragged, DropAfter, DropBefore, Pinned, Preview, Selected};
    use PaletteColor::{Background, Foreground, MutedForeground, Primary, Ring, SecondaryHover, StateHover};
    let line = |width: BorderSize, color: PaletteColor| Border::inside(width, color);
    let inner_ring = || Outline::new(BorderSize::Md, Ring).offset(-2.0);
    let dirty = Selector::new().state(DocumentTabsState::Dirty);
    Recipe::new()
      .base(Root, |s| s.height(38.0).width(Length::Fill))
      .base(Bar, |s| {
        s.background(PaletteColor::Bar)
          .underline(line(BorderSize::Sm, PaletteColor::Border))
      })
      .base(List, |s| s.width(Length::Fill))
      .base(Tab, |s| {
        fading(
          s.padding_start(12.0)
            .padding_end(8.0)
            .gap(8.0)
            .max_width(240.0)
            .end_line(line(BorderSize::Sm, PaletteColor::Border))
            .foreground(MutedForeground)
            .cursor(CursorIcon::Pointer),
        )
      })
      .base(Icon, |s| s.square(14.0).foreground(MutedForeground))
      .base(Label, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .weight(FontWeight::Normal)
          .foreground(MutedForeground)
      })
      .base(Close, |s| {
        fading(
          s.square(20.0)
            .radius(RadiusSize::Sm)
            .cursor(CursorIcon::Pointer),
        )
      })
      .base(CloseIcon, |s| s.square(13.0).foreground(MutedForeground).opacity(0.5))
      .base(Dirty, |s| s.square(8.0).radius(RadiusSize::Full).background(Foreground))
      .base(Add, |s| {
        fading(s.width(38.0).foreground(Foreground).cursor(CursorIcon::Pointer))
      })
      .base(AddIcon, |s| s.square(16.0))
      .base(More, |s| {
        fading(
          s.padding_x(10.0)
            .gap(4.0)
            .start_line(line(BorderSize::Sm, PaletteColor::Border))
            .cursor(CursorIcon::Pointer),
        )
      })
      .base(MoreLabel, |s| s.typography(TypographyStyle::Caption).foreground(MutedForeground))
      .base(MoreIcon, |s| s.square(14.0).foreground(MutedForeground))
      .state(Pinned, Tab, |s| s.width(38.0).padding_x(0.0).gap(0.0))
      // The bar is a row of the density's height; its squares stay square.
      .row_density(Selector::new(), Root, 38.0)
      .by_density(Selector::new(), Add, |s, step| s.width(38.0 + step))
      .by_density(Selector::new().state(Pinned), Tab, |s, step| s.width(38.0 + step))
      // A tab under the pointer, or holding the focus, shows its cross whole; the control under the pointer is lit.
      .within(CloseIcon, |s| s.foreground(Foreground).opacity(1.0))
      .hovered(Close, |s| s.background(SecondaryHover))
      .hovered(Add, |s| s.background(StateHover))
      .hovered(More, |s| s.background(StateHover))
      .focus_visible(Tab, |s| s.outline(inner_ring()))
      .focus_visible(Add, |s| s.outline(inner_ring()))
      .focus_visible(More, |s| s.outline(inner_ring()))
      .state(Selected, Tab, |s| {
        s.background(Background)
          .foreground(Foreground)
          .top_line(line(BorderSize::Md, Primary))
          .end_line(line(BorderSize::Sm, Background))
      })
      .state(Selected, Icon, |s| s.foreground(Foreground))
      .state(Selected, Label, |s| s.foreground(Foreground))
      .state(Selected, CloseIcon, |s| s.foreground(Foreground).opacity(1.0))
      // A document not saved shows its dot, and its cross only under the pointer.
      .state(DocumentTabsState::Dirty, CloseIcon, |s| s.opacity(0.0))
      .when(dirty.clone().within(), CloseIcon, |s| s.foreground(Foreground).opacity(1.0))
      .when(dirty.within(), Dirty, |s| s.opacity(0.0))
      .state(Preview, Label, |s| s.font_style(FontStyle::Italic))
      .state(Disabled, Tab, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
      .state(Dragged, Tab, |s| s.opacity(OpacityLevel::Muted))
      .state(DropBefore, Tab, |s| s.start_line(line(BorderSize::Md, Primary)))
      .state(DropAfter, Tab, |s| s.end_line(line(BorderSize::Md, Primary)))
  }
}
