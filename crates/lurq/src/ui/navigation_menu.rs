//! A navigation menu: the way round a site along a row of links, some of which open a panel of links under them.
//!
//! It is a mounted component, not a builder that converts into an element: it keeps the timers of its delays (a
//! panel opens under the pointer after a moment, and goes when the pointer has left it and its item for a moment),
//! where the pointer is, how the open panel was opened, and an anchor and an open signal for each panel, none of
//! which a builder can keep. [`NavigationMenu`] says what it holds, and [`NavigationMenu::mount`] puts it where it
//! goes.

use std::{sync::Arc, time::Duration};

use parking_lot::Mutex;

use super::{
  icons, names,
  part_style::{PartAnimation, PartStyle, TransitionDuration},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{FADE_IN, FADE_OUT, fading, glyph, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::{CollisionStrategy, Ctx, OutsidePress, Placement, Timeout},
    events::{KeyboardEvent, MouseEvent},
    theme::{
      BorderSize, IconSize, MotionDuration, MotionEasing, OpacityLevel, PaletteColor, RadiusSize, ShadowStyle, Theme,
      TypographyStyle,
    },
  },
  components::{self, Column, Popup, Row},
  core::{ElementRef, Signal, WatchHandle},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, FocusGroup, HitTestBehavior, border::Border, color::Color, dimension::Dimension},
};

/// What a choice runs.
type Handler = Arc<dyn Fn() + Send + Sync>;

/// What hears the open panel change.
pub(crate) type OpenHandler = Arc<dyn Fn(Option<usize>) + Send + Sync>;

#[cfg(feature = "router")]
type Navigator = crate::router::Navigator;
#[cfg(not(feature = "router"))]
type Navigator = ();

/// Whether a key was pressed with no modifier held but Shift.
fn plain(event: &KeyboardEvent) -> bool {
  !event.ctrl && !event.alt && !event.meta
}

/// The parts of a navigation menu a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NavigationMenuPart {
  /// The row of items.
  Bar,
  /// An item of the row: a link, or what opens a panel.
  Item,
  /// The icon before an item's text.
  ItemIcon,
  /// The text of an item.
  Label,
  /// The chevron after the text of an item that opens a panel.
  Chevron,
  /// The room between an item and its panel, which the pointer crosses without the panel going.
  Viewport,
  /// The panel: its fill, its edge, its shadow, the room between its columns.
  Panel,
  /// A column of the panel's links.
  Column,
  /// What the app features at the start of a panel: a card of the newest release, a picture.
  Featured,
  /// A link of a panel: what is pressed.
  Link,
  /// The icon at the start of a link of a panel.
  LinkIcon,
  /// The title over the description of a link of a panel.
  LinkText,
  /// The title of a link of a panel.
  LinkTitle,
  /// A line or two about where a link of a panel goes.
  LinkDescription,
}

names!(NavigationMenuPart {
  Bar => "bar",
  Item => "item",
  ItemIcon => "item-icon",
  Label => "label",
  Chevron => "chevron",
  Viewport => "viewport",
  Panel => "panel",
  Column => "column",
  Featured => "featured",
  Link => "link",
  LinkIcon => "link-icon",
  LinkText => "link-text",
  LinkTitle => "link-title",
  LinkDescription => "link-description",
});

/// The states of a navigation menu a recipe styles. An item and a link of a panel are resolved in the states of
/// the menu and their own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NavigationMenuState {
  /// The item's panel is open.
  Open,
  /// The item is the page shown, or its panel holds it; the link of a panel is the page shown.
  Current,
  /// The menu, the item or the link cannot be pressed.
  Disabled,
}

names!(NavigationMenuState {
  Open => "open",
  Current => "current",
  Disabled => "disabled",
});

/// How a navigation menu's items look.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NavigationMenuVariant {
  /// Each item a box that is lit under the pointer and while its panel is open: the board's Navigation Menu.
  #[default]
  Menu,
  /// Each item its text alone, in the muted foreground, the page shown in the foreground: the links of a
  /// [`Navbar`](super::Navbar).
  Links,
}

names!(NavigationMenuVariant {
  Menu => "menu",
  Links => "links",
});

/// What builds a navigation menu's nodes: a theme's in place of the default one. It builds what is shown, not when.
pub type NavigationMenuRenderer = Arc<dyn Fn(NavigationMenuView) -> Element + Send + Sync>;

/// A link of a panel of a [`NavigationMenu`]: a title, a line about where it goes, an icon.
///
/// ```
/// use lurq::ui::{NavLink, icons};
///
/// let start = NavLink::new("Getting started")
///   .description("Install lurq and open your first window.")
///   .icon(icons::HOME)
///   .href("/docs/start");
/// ```
#[derive(Clone)]
pub struct NavLink {
  pub(crate) title: Arc<str>,
  pub(crate) description: Option<Arc<str>>,
  pub(crate) icon: Option<Name>,
  pub(crate) href: Option<Arc<str>>,
  pub(crate) on_select: Vec<Handler>,
  pub(crate) current: bool,
  pub(crate) disabled: bool,
  pub(crate) id: Option<Arc<str>>,
  pub(crate) key: Option<Arc<str>>,
}

impl NavLink {
  /// A link called `title`.
  pub fn new(title: &str) -> Self {
    Self {
      title: Arc::from(title),
      description: None,
      icon: None,
      href: None,
      on_select: Vec::new(),
      current: false,
      disabled: false,
      id: None,
      key: None,
    }
  }

  /// A line or two about where it goes, under its title.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// An icon of the theme's set at its start.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// The path of the app's router it goes to when it is chosen.
  pub fn href(mut self, path: &str) -> Self {
    self.href = Some(Arc::from(path));
    self
  }

  /// What choosing it does, beside going to its path. Choosing it closes its panel first, and then calls this.
  pub fn on_select(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_select.push(Arc::new(handler));
    self
  }

  /// Whether it is the page shown: its item is marked as holding it.
  pub fn current(mut self, current: bool) -> Self {
    self.current = current;
    self
  }

  /// Whether it cannot be chosen: it is dimmed, and the keys pass it by.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// The id of the link: what is pressed.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// What tells the link apart from those beside it, where two have one title.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }
}

/// What an item of a [`NavigationMenu`] opens under it: links in columns, and what the app features before them.
///
/// ```
/// use lurq::ui::{NavLink, NavPanel};
///
/// let docs = NavPanel::new()
///   .columns(2)
///   .link(NavLink::new("Getting started").description("Install lurq and open your first window."))
///   .link(NavLink::new("Theming").description("Roles, palettes and dark mode."));
/// ```
#[derive(Clone)]
pub struct NavPanel {
  pub(crate) links: Vec<NavLink>,
  pub(crate) columns: usize,
  pub(crate) featured: Option<Element>,
}

impl Default for NavPanel {
  fn default() -> Self {
    Self::new()
  }
}

impl NavPanel {
  /// A panel with nothing in it, of one column.
  pub fn new() -> Self {
    Self {
      links: Vec::new(),
      columns: 1,
      featured: None,
    }
  }

  /// A link, after those it holds.
  pub fn link(mut self, link: NavLink) -> Self {
    self.links.push(link);
    self
  }

  /// Links, in order, after those it holds.
  pub fn links(mut self, links: impl IntoIterator<Item = NavLink>) -> Self {
    self.links.extend(links);
    self
  }

  /// How many columns its links are in: the first column is filled first, each holding as many as the others or
  /// one more. One unless told.
  pub fn columns(mut self, columns: usize) -> Self {
    self.columns = columns.max(1);
    self
  }

  /// What the app features at the start of the panel, a column of its own beside the links: a [`Card`](super::Card)
  /// of the newest release.
  pub fn featured(mut self, featured: impl Into<Element>) -> Self {
    self.featured = Some(featured.into());
    self
  }
}

/// An item of a [`NavigationMenu`] or a [`Navbar`](super::Navbar): a link, or what opens a panel of links.
///
/// ```
/// use lurq::ui::{NavItem, NavLink, NavPanel};
///
/// let blog = NavItem::new("Blog").href("/blog").current(true);
/// let docs = NavItem::new("Docs").panel(NavPanel::new().link(NavLink::new("Getting started").href("/docs")));
/// ```
#[derive(Clone)]
pub struct NavItem {
  pub(crate) label: Arc<str>,
  pub(crate) href: Option<Arc<str>>,
  pub(crate) on_select: Vec<Handler>,
  pub(crate) current: bool,
  pub(crate) icon: Option<Name>,
  pub(crate) disabled: bool,
  pub(crate) panel: Option<NavPanel>,
  pub(crate) id: Option<Arc<str>>,
  pub(crate) key: Option<Arc<str>>,
}

impl NavItem {
  /// An item called `label`.
  pub fn new(label: &str) -> Self {
    Self {
      label: Arc::from(label),
      href: None,
      on_select: Vec::new(),
      current: false,
      icon: None,
      disabled: false,
      panel: None,
      id: None,
      key: None,
    }
  }

  /// The path of the app's router it goes to when it is chosen. An item that opens a panel goes nowhere itself.
  pub fn href(mut self, path: &str) -> Self {
    self.href = Some(Arc::from(path));
    self
  }

  /// What choosing it does, beside going to its path. An item that opens a panel is not chosen.
  pub fn on_select(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_select.push(Arc::new(handler));
    self
  }

  /// Whether it is the page shown.
  pub fn current(mut self, current: bool) -> Self {
    self.current = current;
    self
  }

  /// An icon of the theme's set before its text.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// Whether it cannot be pressed: it is dimmed, and its panel does not open.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// The panel of links it opens under it, in place of going somewhere: it is a button with a chevron.
  pub fn panel(mut self, panel: NavPanel) -> Self {
    self.panel = Some(panel);
    self
  }

  /// The id of the item: what is pressed. Its panel is `{id}-panel`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// What tells the item apart from those beside it, where two have one label.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Whether it is the page shown, or its panel holds the page shown.
  pub(crate) fn holds_current(&self) -> bool {
    self.current
      || self
        .panel
        .as_ref()
        .is_some_and(|panel| panel.links.iter().any(|link| link.current))
  }
}

/// A navigation menu: the way round a site along a row of links, some of which open a panel of links under them,
/// as the Docs, Components and Blog of a site's header.
///
/// ```no_run
/// use lurq::{
///   app::ctx::Ctx,
///   node::Element,
///   ui::{NavItem, NavLink, NavPanel, NavigationMenu},
/// };
///
/// fn site(ctx: &mut Ctx) -> Element {
///   NavigationMenu::new()
///     .label("Main")
///     .item(
///       NavItem::new("Docs").panel(
///         NavPanel::new()
///           .columns(2)
///           .link(NavLink::new("Getting started").description("Install lurq and open your first window.").href("/docs"))
///           .link(NavLink::new("Theming").description("Roles, palettes and dark mode.").href("/docs/theming")),
///       ),
///     )
///     .item(NavItem::new("Blog").href("/blog"))
///     .mount(ctx)
/// }
/// ```
///
/// An item with a panel opens it under itself by a press, and under the pointer after the open delay; one panel is
/// open at a time, and with one open the pointer on another item hands it over at once. The panel goes on Escape,
/// on a press outside it, when one of its links is chosen, and when the pointer has left it and its item for the
/// close delay. Which panel is open is the app's where it gives a [`Signal`] ([`NavigationMenu::open`]); the menu
/// keeps its own where it does not.
///
/// Each item is a stop of Tab, as links are on a page, and Left and Right go between them as well. Enter, Space and
/// Down open an item's panel with the focus on its first link; Up and Down go through the links, Left and Right
/// close it and go to the item beside it, opening that one's panel where it has one. Tab goes from an item into its
/// open panel, and from the panel's last link on to what follows the item, closing the panel. To tooling the menu
/// is a navigation region called by its label, its items links (the page shown is the current page) or buttons that
/// say whether their panel is expanded and which it is, and a panel's links links.
#[derive(Clone)]
pub struct NavigationMenu {
  pub(crate) label: Arc<str>,
  pub(crate) items: Vec<NavItem>,
  pub(crate) open: Option<Signal<Option<usize>>>,
  pub(crate) on_open_change: Vec<OpenHandler>,
  pub(crate) variant: Name,
  pub(crate) open_delay: TransitionDuration,
  pub(crate) close_delay: TransitionDuration,
  pub(crate) disabled: bool,
  /// Whether it is a navigation region to tooling: not where a navbar is one around it.
  pub(crate) landmark: bool,
  pub(crate) states: Vec<Name>,
  pub(crate) rules: Recipe,
  pub(crate) forced: Option<Interaction>,
  pub(crate) id: Option<Arc<str>>,
  pub(crate) key: Option<Arc<str>>,
}

impl Default for NavigationMenu {
  fn default() -> Self {
    Self::new()
  }
}

impl NavigationMenu {
  /// A menu with no items, its panels closed.
  pub fn new() -> Self {
    Self {
      label: Arc::from("Main"),
      items: Vec::new(),
      open: None,
      on_open_change: Vec::new(),
      variant: Name::from(NavigationMenuVariant::Menu),
      open_delay: TransitionDuration::Role(MotionDuration::Base),
      close_delay: TransitionDuration::Role(MotionDuration::Slow),
      disabled: false,
      landmark: true,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// What it is called to tooling that reads the tree: "Main", "Docs". "Main" unless told otherwise.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Arc::from(label);
    self
  }

  /// An item, after those it holds.
  pub fn item(mut self, item: NavItem) -> Self {
    self.items.push(item);
    self
  }

  /// Items, in order, after those it holds.
  pub fn items(mut self, items: impl IntoIterator<Item = NavItem>) -> Self {
    self.items.extend(items);
    self
  }

  /// Which item's panel is open, by its index among the items: the app's, which a press, the pointer and the keys
  /// set, and Escape, a press outside, a choice and the pointer going clear. Without one the menu keeps its own.
  pub fn open(mut self, open: Signal<Option<usize>>) -> Self {
    self.open = Some(open);
    self
  }

  /// What hears a panel open, another take its place, or the open one close: the index of the item whose panel is
  /// open now, `None` when none is.
  pub fn on_open_change(mut self, handler: impl Fn(Option<usize>) + Send + Sync + 'static) -> Self {
    self.on_open_change.push(Arc::new(handler));
    self
  }

  /// How its items look: a [`NavigationMenuVariant`], or a variant a theme's recipe names. `Menu` unless told.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// How long the pointer rests on an item before its panel opens: the theme's base duration unless told.
  pub fn open_delay(mut self, delay: impl Into<TransitionDuration>) -> Self {
    self.open_delay = delay.into();
    self
  }

  /// How long an open panel stays after the pointer has left it and its item: the theme's slow duration unless
  /// told, time to cross from the item to the panel.
  pub fn close_delay(mut self, delay: impl Into<TransitionDuration>) -> Self {
    self.close_delay = delay.into();
    self
  }

  /// Whether no item can be pressed.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Whether it is a navigation region of its own to tooling, or a list in one around it.
  pub(crate) fn landmark(mut self, landmark: bool) -> Self {
    self.landmark = landmark;
    self
  }

  /// Puts the menu in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this menu over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this menu.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the items as they are under the pointer, held down or focused from the keyboard, with nothing pointing
  /// at them: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the row. Its items are `{id}-0`, `{id}-1` and on, unless they have ids of their own, the panel of
  /// each `{id}-0-panel`, and the links of a panel `{id}-0-0`, `{id}-0-1` and on.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Puts the menu where it goes, with the timers of its delays.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    ctx.mount::<NavigationMenuHost>(NavigationMenuProps { menu: self })
  }

  /// The menu's recipe in `case`.
  fn resolve(&self, theme: &Theme, case: &Case) -> ResolvedRecipe {
    theme.resolve::<NavigationMenu>(case, Some(&self.rules))
  }
}

/// A navigation menu converted into an element on its own opens its panels by a press and the keys, with no
/// delays of the pointer and nothing kept from one build to the next: for a gallery, a test, or a window of its own.
/// Which panel is open is then the signal [`NavigationMenu::open`] gives, without which none opens.
/// [`NavigationMenu::mount`] is what an app puts in its window.
impl From<NavigationMenu> for Element {
  fn from(menu: NavigationMenu) -> Self {
    let own = Signal::new(None);
    let shared = Arc::new(Shared::new());
    build(&menu, &shared, &own, None, None, None)
  }
}

/// A navigation menu.
#[derive(Clone)]
struct NavigationMenuProps {
  menu: NavigationMenu,
}

/// The menu is built anew by every render of what holds it, so props are never the same as before.
impl PartialEq for NavigationMenuProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for NavigationMenuProps {}

/// How the open panel was opened: what closes it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Opened {
  /// The pointer rested on its item: the pointer going clear closes it, and a press keeps it.
  #[default]
  Pointer,
  /// A press of the pointer: the pointer going clear closes it, and a press of its item.
  Press,
  /// A key: the focus is in it, and only the keys, a press and the focus leaving close it.
  Key,
}

/// Where the pointer is.
#[derive(Default)]
struct Pointer {
  /// The item that opens a panel the pointer is on.
  on_item: Option<usize>,
  /// Whether it is on the open panel, or crossing to it.
  on_panel: bool,
}

/// What a panel keeps from one render to the next.
#[derive(Clone)]
struct Kept {
  /// Whether it is open: one with the menu's open panel, and what Escape and a press outside set.
  open: Signal<bool>,
  /// Its item, which it hangs from.
  anchor: ElementRef,
  /// The panel, for the focus in it.
  panel: ElementRef,
  /// Its first link that can be chosen, for the focus to go to.
  first: ElementRef,
}

/// The watches that keep each panel's open signal and the menu's one.
#[derive(Default)]
struct Watches {
  /// The signal they are for.
  bound: Option<usize>,
  /// Each panel open while the menu's signal names it.
  _leads: Option<WatchHandle<Option<usize>>>,
  /// The menu's signal following each panel closing itself.
  _follows: Vec<WatchHandle<bool>>,
}

/// What a menu keeps, as its renders, its handlers and its timers share it.
struct Shared {
  /// The menu's open panel: the app's signal, or the menu's own.
  app: Mutex<Option<Signal<Option<usize>>>>,
  /// What hears it change, as the last render gave it.
  heard: Arc<Mutex<Vec<OpenHandler>>>,
  panels: Arc<Mutex<Vec<Kept>>>,
  watches: Mutex<Watches>,
  opened: Mutex<Opened>,
  pointer: Mutex<Pointer>,
  /// The item the open delay runs for.
  pending: Mutex<Option<usize>>,
  /// The panel whose first link the focus goes to at the next render: Down on an item whose panel is open already.
  focus_first: Mutex<Option<usize>>,
  /// Renders the menu again, for `focus_first`.
  nudge: Signal<u64>,
  /// The open panel the keyboard's focus has been in, or on its item: it closes as the focus leaves both.
  held: Mutex<Option<usize>>,
  /// The panel open at the last render.
  shown: Mutex<Option<usize>>,
}

impl Shared {
  fn new() -> Self {
    Self {
      app: Mutex::new(None),
      heard: Arc::new(Mutex::new(Vec::new())),
      panels: Arc::new(Mutex::new(Vec::new())),
      watches: Mutex::new(Watches::default()),
      opened: Mutex::new(Opened::Pointer),
      pointer: Mutex::new(Pointer::default()),
      pending: Mutex::new(None),
      focus_first: Mutex::new(None),
      nudge: Signal::new(0),
      held: Mutex::new(None),
      shown: Mutex::new(None),
    }
  }

  /// The menu's open signal.
  fn app(&self) -> Option<Signal<Option<usize>>> {
    self.app.lock().clone()
  }

  /// The panel open now.
  fn now(&self) -> Option<usize> {
    self.app().and_then(|app| app.get_untracked())
  }

  fn opened(&self) -> Opened {
    *self.opened.lock()
  }

  /// What panel `index` keeps.
  fn kept(&self, index: usize) -> Option<Kept> {
    self.panels.lock().get(index).cloned()
  }

  /// Opens the panel `index`, as `how` opened it.
  fn open(&self, index: usize, how: Opened) {
    *self.opened.lock() = how;
    *self.held.lock() = None;
    if let Some(app) = self.app()
      && app.get_untracked() != Some(index)
    {
      app.set(Some(index));
    }
  }

  /// Closes the open panel.
  fn close(&self) {
    if let Some(app) = self.app()
      && app.get_untracked().is_some()
    {
      app.set(None);
    }
  }

  /// Keeps the panels for `app`, the menu's open signal now, and `count` items.
  fn bind(&self, app: &Signal<Option<usize>>, count: usize) {
    let mut watches = self.watches.lock();
    if watches.bound != Some(app.id()) {
      *self.app.lock() = Some(app.clone());
      watches.bound = Some(app.id());
      let panels = self.panels.lock().clone();
      for (index, kept) in panels.iter().enumerate() {
        let want = app.get_untracked() == Some(index);
        if kept.open.get_untracked() != want {
          kept.open.set(want);
        }
      }
      watches._follows = panels
        .iter()
        .enumerate()
        .map(|(index, kept)| follow(app, &kept.open, index))
        .collect();
      watches._leads = Some(lead(app, &self.panels, &self.heard));
    }
    let mut panels = self.panels.lock();
    while panels.len() < count {
      let index = panels.len();
      let open = Signal::new(app.get_untracked() == Some(index));
      watches._follows.push(follow(app, &open, index));
      panels.push(Kept {
        open,
        anchor: ElementRef::new(),
        panel: ElementRef::new(),
        first: ElementRef::new(),
      });
    }
  }
}

/// The menu's signal following the panel `index` closing itself: Escape, a press outside, the keys going to its
/// neighbour.
fn follow(app: &Signal<Option<usize>>, open: &Signal<bool>, index: usize) -> WatchHandle<bool> {
  let (own, app) = (open.clone(), app.clone());
  open.watch(move || {
    let (on, now) = (own.get_untracked(), app.get_untracked());
    if on && now != Some(index) {
      app.set(Some(index));
    } else if !on && now == Some(index) {
      app.set(None);
    }
  })
}

/// Each of `panels` open while the menu's signal names it, and what hears it told.
fn lead(
  app: &Signal<Option<usize>>,
  panels: &Arc<Mutex<Vec<Kept>>>,
  heard: &Arc<Mutex<Vec<OpenHandler>>>,
) -> WatchHandle<Option<usize>> {
  let (signal, panels, heard) = (app.clone(), panels.clone(), heard.clone());
  app.watch(move || {
    let now = signal.get_untracked();
    // Out of the lock: setting one runs what follows it.
    let opens: Vec<Signal<bool>> = panels.lock().iter().map(|kept| kept.open.clone()).collect();
    for (index, open) in opens.iter().enumerate() {
      let want = now == Some(index);
      if open.get_untracked() != want {
        open.set(want);
      }
    }
    let heard = heard.lock().clone();
    for handler in heard {
      handler(now);
    }
  })
}

/// The timers of a mounted menu.
#[derive(Clone)]
struct Timers {
  /// The open delay: the panel of the item the pointer rests on opens.
  opening: Timeout,
  /// The close delay: the open panel goes where the pointer has left it and its item.
  closing: Timeout,
  /// After the focus moved: the open panel goes where the focus has left it and its item.
  leaving: Timeout,
  /// Whether the open delay is none: the panel opens as the pointer comes.
  instant: bool,
}

/// What keeps a mounted menu between renders.
struct NavigationMenuHost {
  /// Which panel is open, where the app gives no signal of its own.
  own: Signal<Option<usize>>,
  shared: Arc<Shared>,
  timers: Timers,
}

/// A duration of `theme`'s, or a set one.
fn length(duration: TransitionDuration, theme: &Theme) -> Duration {
  match duration {
    TransitionDuration::Role(role) => theme.durations().try_get(role).unwrap_or_default(),
    TransitionDuration::Set(duration) => duration,
  }
}

impl Component for NavigationMenuHost {
  type Props = NavigationMenuProps;

  fn create(ctx: &mut Ctx) -> Self {
    let shared = Arc::new(Shared::new());
    let opening = {
      let shared = shared.clone();
      ctx.create_timeout(Duration::ZERO, move || {
        let pending = shared.pending.lock().take();
        let on = shared.pointer.lock().on_item;
        if let Some(index) = pending
          && on == Some(index)
        {
          shared.open(index, Opened::Pointer);
        }
      })
    };
    let closing = {
      let shared = shared.clone();
      ctx.create_timeout(Duration::ZERO, move || {
        let gone = {
          let pointer = shared.pointer.lock();
          pointer.on_item.is_none() && !pointer.on_panel
        };
        if gone && shared.opened() != Opened::Key {
          shared.close();
        }
      })
    };
    let leaving = {
      let shared = shared.clone();
      ctx.create_timeout(Duration::ZERO, move || {
        let Some(open) = shared.now() else {
          return;
        };
        if *shared.held.lock() != Some(open) {
          return;
        }
        let Some(kept) = shared.kept(open) else {
          return;
        };
        let within = kept.anchor.focus_visible_within_signal().get_untracked()
          || kept.panel.focus_visible_within_signal().get_untracked();
        if !within {
          *shared.held.lock() = None;
          shared.close();
        }
      })
    };
    Self {
      own: ctx.signal(None),
      shared,
      timers: Timers {
        opening,
        closing,
        leaving,
        instant: false,
      },
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let menu = ctx.props::<NavigationMenuProps>().menu.clone();
    let theme = current_theme();
    let open_delay = length(menu.open_delay, &theme);
    self.timers.opening.set_duration(open_delay);
    self.timers.closing.set_duration(length(menu.close_delay, &theme));
    let timers = Timers {
      instant: open_delay.is_zero(),
      ..self.timers.clone()
    };
    #[cfg(feature = "router")]
    let navigator = ctx.navigator();
    #[cfg(not(feature = "router"))]
    let navigator = None;
    build(&menu, &self.shared, &self.own, Some(&timers), Some(ctx), navigator)
  }
}

/// Builds `menu` with what `shared` keeps, its open panel `own`'s where the app gives none. Without `timers` the
/// pointer opens nothing by resting; without `ctx` no focus is asked for.
fn build(
  menu: &NavigationMenu,
  shared: &Arc<Shared>,
  own: &Signal<Option<usize>>,
  timers: Option<&Timers>,
  ctx: Option<&mut Ctx>,
  navigator: Option<Navigator>,
) -> Element {
  let theme = current_theme();
  let app = menu.open.clone().unwrap_or_else(|| own.clone());
  *shared.heard.lock() = menu.on_open_change.clone();
  shared.bind(&app, menu.items.len());
  // Reading it subscribes: a render follows the menu's open panel, and a request for the focus.
  let open = app.get().filter(|_| !menu.disabled);
  let _ = shared.nudge.get();
  // A panel that went from under the pointer is told of no leave: the pointer is on it no longer.
  if std::mem::replace(&mut *shared.shown.lock(), open) != open {
    shared.pointer.lock().on_panel = false;
  }
  if let Some(open) = open
    && let Some(kept) = shared.kept(open)
  {
    // The focus leaving the open panel and its item closes it, where it was in either: read after the move, both
    // having changed.
    let within = kept.anchor.focus_visible_within() || kept.panel.focus_visible_within();
    let mut held = shared.held.lock();
    if within {
      *held = Some(open);
    } else if *held == Some(open)
      && let Some(timers) = timers
    {
      timers.leaving.restart();
    }
    drop(held);
    let first = shared.focus_first.lock().take();
    if first == Some(open)
      && let Some(ctx) = &ctx
    {
      ctx.focus(&kept.first);
    }
  }
  let case = Case {
    variant: Some(menu.variant.clone()),
    size: None,
    density: None,
    breakpoint: super::current_breakpoint(),
    states: menu.states.clone(),
  }
  .state(NavigationMenuState::Disabled, menu.disabled);
  let view = NavigationMenuView {
    label: menu.label.clone(),
    items: menu.items.clone(),
    open,
    disabled: menu.disabled,
    recipe: menu.resolve(&theme, &case),
    case,
    rules: menu.rules.clone(),
    forced: menu.forced,
    landmark: menu.landmark,
    shared: shared.clone(),
    timers: timers.cloned(),
    navigator,
    id: menu.id.clone(),
    key: menu.key.clone(),
  };
  match theme.renderer::<NavigationMenu>() {
    Some(renderer) => renderer(view),
    None => view.render(),
  }
}

/// A navigation menu as a renderer sees it: its items, the style of its parts, and its behavior ready to be
/// dressed.
pub struct NavigationMenuView {
  /// What it is called to tooling.
  pub label: Arc<str>,
  /// Its items, in order. [`panel`](Self::panel) takes what an item's panel features.
  pub items: Vec<NavItem>,
  /// The item whose panel is open now.
  pub open: Option<usize>,
  /// Whether no item can be pressed.
  pub disabled: bool,
  /// The style of each part of the menu, in its states.
  pub recipe: ResolvedRecipe,
  case: Case,
  rules: Recipe,
  forced: Option<Interaction>,
  landmark: bool,
  shared: Arc<Shared>,
  timers: Option<Timers>,
  navigator: Option<Navigator>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl NavigationMenuView {
  /// The style of a part of the menu.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// Whether the item `index` cannot be pressed.
  pub fn is_disabled(&self, index: usize) -> bool {
    self.disabled || self.items.get(index).is_some_and(|item| item.disabled)
  }

  /// The recipe in the states given.
  fn recipe_in(&self, states: &[(NavigationMenuState, bool)]) -> ResolvedRecipe {
    let mut case = self.case.clone();
    for (state, on) in states {
      case = case.state(*state, *on);
    }
    self.theme().resolve::<NavigationMenu>(&case, Some(&self.rules))
  }

  /// The style of a part of the item `index`, in its states: open, current, disabled.
  pub fn item_part(&self, index: usize, part: impl Into<Name>) -> ResolvedPart {
    let current = self.items.get(index).is_some_and(NavItem::holds_current);
    self
      .recipe_in(&[
        (NavigationMenuState::Open, self.open == Some(index)),
        (NavigationMenuState::Current, current),
        (NavigationMenuState::Disabled, self.is_disabled(index)),
      ])
      .part(part)
      .forced(self.forced)
  }

  /// The id of the item `index`.
  fn item_id(&self, index: usize) -> Option<Arc<str>> {
    let own = self.items.get(index).and_then(|item| item.id.clone());
    own.or_else(|| self.id.as_ref().map(|id| Arc::from(format!("{id}-{index}"))))
  }

  /// The id of the panel of the item `index`: its own of the menu's, or one the menu makes up, which its item names
  /// to tooling.
  fn panel_id(&self, index: usize) -> Arc<str> {
    match &self.id {
      Some(id) => Arc::from(format!("{id}-{index}-panel")),
      None => Arc::from(format!(
        "navigation-menu-{:x}-{index}-panel",
        Arc::as_ptr(&self.shared) as usize
      )),
    }
  }

  /// The row with nothing in it yet: a navigation region called by the label to tooling, or a list in a navbar's;
  /// a row of panels to the runtime, whose items are each a stop of Tab and between which Left and Right move.
  pub fn bar(&self) -> Row {
    let mut bar = Row::new()
      .align_items(Alignment::Center)
      .overflow_visible()
      .part(&self.part(NavigationMenuPart::Bar))
      .focus_group(FocusGroup::horizontal().panel_row(true))
      .describe("a11y_role", if self.landmark { "navigation" } else { "list" });
    if self.landmark {
      bar = bar.describe("a11y_name", self.label.clone());
    }
    if let Some(id) = &self.id {
      bar = bar.id(id.clone());
    }
    if let Some(key) = &self.key {
      bar = bar.key(key.clone());
    }
    bar
  }

  /// What choosing a link does: its handlers, its path, and closing the open panel.
  fn choose(&self, on_select: &[Handler], href: Option<&Arc<str>>) -> impl Fn(MouseEvent) + Send + Sync + 'static {
    let handlers = on_select.to_vec();
    #[cfg(feature = "router")]
    let go = href.cloned().zip(self.navigator.clone());
    #[cfg(not(feature = "router"))]
    let _ = href;
    let shared = self.shared.clone();
    move |_: MouseEvent| {
      // The panel closes first, so that a handler that opens a dialog does not have its focus taken back by it.
      shared.close();
      for handler in &handlers {
        handler();
      }
      #[cfg(feature = "router")]
      if let Some((path, navigator)) = &go {
        navigator.push(path.to_string());
      }
    }
  }

  /// The item `index`'s behavior with the style of its part, with nothing in it yet. A link is chosen by a press.
  /// What opens a panel is a button and the anchor its panel hangs from: a press opens or closes the panel, the
  /// pointer resting on it opens it, or at once while another is open; Enter, Space and Down open it with the focus
  /// on its first link.
  pub fn item(&self, index: usize) -> components::Button {
    let Some(item) = self.items.get(index) else {
      return components::Button::empty();
    };
    let disabled = self.is_disabled(index);
    let open = self.open == Some(index);
    let mut button = pressable(&self.item_part(index, NavigationMenuPart::Item), disabled)
      .align_items(Alignment::Center)
      .describe("a11y_name", item.label.clone())
      .focus_label(item.label.clone());
    if let Some(id) = self.item_id(index) {
      button = button.id(id);
    }
    if let Some(key) = &item.key {
      button = button.key(key.clone());
    }
    if item.panel.is_none() {
      button = button.describe("a11y_role", "link");
      if item.current {
        button = button.describe("a11y_current", "page");
      }
      if !disabled {
        button = button.on_click(self.choose(&item.on_select, item.href.as_ref()));
      }
      return button;
    }
    let anchor = self.shared.kept(index).map_or_else(ElementRef::new, |kept| kept.anchor);
    button = button
      .ref_element(anchor)
      .describe("a11y_role", "button")
      .describe("a11y_expanded", if open { "true" } else { "false" })
      .describe("a11y_controls", self.panel_id(index));
    if disabled {
      return button;
    }
    let shared = &self.shared;
    let (press, asked, keys) = (shared.clone(), shared.clone(), shared.clone());
    let timers = self.timers.clone();
    button = button
      .on_click(move |event: MouseEvent| {
        if let Some(timers) = &timers {
          timers.opening.cancel();
        }
        if press.now() != Some(index) {
          press.open(index, if event.from_keyboard { Opened::Key } else { Opened::Press });
        } else if press.opened() == Opened::Pointer && !event.from_keyboard {
          // Opened under the pointer a moment before: the press that follows keeps it.
          *press.opened.lock() = Opened::Press;
        } else {
          press.close();
        }
      })
      // The runtime asks it to open as Left or Right come to it from the panel beside it.
      .focus_expanded(open, move |want| {
        if want {
          asked.open(index, Opened::Key);
        } else {
          asked.close();
        }
      })
      .on_key_down(move |event: KeyboardEvent| {
        if !event.focus_within || !plain(&event) || event.key != "ArrowDown" {
          return;
        }
        event.prevent_default();
        if keys.now() == Some(index) {
          *keys.opened.lock() = Opened::Key;
          *keys.focus_first.lock() = Some(index);
          keys.nudge.update(|nudge| *nudge += 1);
        } else {
          keys.open(index, Opened::Key);
        }
      });
    if let Some(timers) = &self.timers {
      let (enter, leave) = ((shared.clone(), timers.clone()), (shared.clone(), timers.clone()));
      button = button
        .on_mouse_enter(move || {
          let (shared, timers) = &enter;
          shared.pointer.lock().on_item = Some(index);
          timers.closing.cancel();
          match shared.now() {
            Some(open) if open == index => {}
            // One is open: it is handed over at once.
            Some(_) => {
              timers.opening.cancel();
              shared.open(index, Opened::Pointer);
            }
            None if timers.instant => shared.open(index, Opened::Pointer),
            None => {
              *shared.pending.lock() = Some(index);
              timers.opening.restart();
            }
          }
        })
        .on_mouse_leave(move || {
          let (shared, timers) = &leave;
          {
            let mut pointer = shared.pointer.lock();
            if pointer.on_item == Some(index) {
              pointer.on_item = None;
            }
          }
          timers.opening.cancel();
          if shared.now().is_some() && shared.opened() != Opened::Key {
            timers.closing.restart();
          }
        });
    }
    button
  }

  /// The icon before the text of the item `index`, where it has one.
  pub fn item_icon(&self, index: usize) -> Option<Element> {
    let name = self.items.get(index)?.icon?;
    Some(glyph(&self.item_part(index, NavigationMenuPart::ItemIcon), name, false))
  }

  /// The text of the item `index`.
  pub fn label(&self, index: usize) -> Element {
    let part = self.item_part(index, NavigationMenuPart::Label);
    let label = self.items.get(index).map(|item| item.label.clone()).unwrap_or_default();
    Row::new().part(&part).child(part.text(&label).nowrap()).into()
  }

  /// The chevron of the item `index`, where it opens a panel: down while the panel is closed, up while it is open.
  pub fn chevron(&self, index: usize) -> Option<Element> {
    self.items.get(index)?.panel.as_ref()?;
    let name = if self.open == Some(index) {
      icons::CHEVRON_UP
    } else {
      icons::CHEVRON_DOWN
    };
    Some(glyph(
      &self.item_part(index, NavigationMenuPart::Chevron),
      Name::new(name),
      false,
    ))
  }

  /// A link of the panel of the item `index`, the `at`th of its links: its icon, its title over its description.
  /// The first that can be chosen is where the focus goes as the panel opens from the keyboard.
  fn link(&self, index: usize, at: usize, link: &NavLink, first: Option<&ElementRef>) -> Element {
    let recipe = self.recipe_in(&[
      (NavigationMenuState::Current, link.current),
      (NavigationMenuState::Disabled, link.disabled),
    ]);
    let part = |name: NavigationMenuPart| recipe.part(name).forced(self.forced);
    let fill = Dimension::Pct(100.0);
    let mut button = pressable(&part(NavigationMenuPart::Link), link.disabled)
      .width(fill)
      .justify(Justify::Start)
      .align_items(Alignment::Start)
      .describe("a11y_role", "link")
      .describe("a11y_name", link.title.clone())
      .focus_label(link.title.clone());
    if let Some(description) = &link.description {
      button = button.describe("a11y_description", description.clone());
    }
    if link.current {
      button = button.describe("a11y_current", "page");
    }
    // The focus goes to it as the panel opens from the keyboard, past what the panel features.
    if let Some(first) = first {
      button = button.ref_element(first.clone()).autofocus(true);
    }
    let id = link
      .id
      .clone()
      .or_else(|| self.id.as_ref().map(|id| Arc::from(format!("{id}-{index}-{at}"))));
    if let Some(id) = id {
      button = button.id(id);
    }
    if let Some(key) = &link.key {
      button = button.key(key.clone());
    }
    if !link.disabled {
      button = button.on_click(self.choose(&link.on_select, link.href.as_ref()));
    }
    if let Some(icon) = link.icon {
      button = button.child(glyph(&part(NavigationMenuPart::LinkIcon), icon, false));
    }
    let wrapping = |name: NavigationMenuPart, content: &str| {
      let style = part(name);
      Column::new()
        .width(fill)
        .min_width(Dimension::Px(0.0))
        .part(&style)
        .child(style.text(content).width(fill))
    };
    let mut text = Column::new()
      .flex(1.0)
      .min_width(Dimension::Px(0.0))
      .align_items(Alignment::Stretch)
      .part(&part(NavigationMenuPart::LinkText))
      .child(wrapping(NavigationMenuPart::LinkTitle, &link.title));
    if let Some(description) = &link.description {
      text = text.child(wrapping(NavigationMenuPart::LinkDescription, description));
    }
    button.child(text).into()
  }

  /// The panel of the item `index` as it is drawn, with nothing around it: what it features, then its links in
  /// columns, Up and Down going through them. It takes what the panel features out of the view: it is built once.
  pub fn panel_content(&mut self, index: usize) -> Option<Row> {
    let panel = self.items.get_mut(index)?.panel.as_mut()?;
    let featured = panel.featured.take();
    let panel = panel.clone();
    let first_at = panel.links.iter().position(|link| !link.disabled);
    let first = self.shared.kept(index).map(|kept| kept.first);
    let mut row = Row::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(NavigationMenuPart::Panel))
      .focus_group(FocusGroup::vertical())
      .describe("a11y_role", "group")
      .id(self.panel_id(index));
    if let Some(featured) = featured {
      row = row.child(
        Column::new()
          .align_items(Alignment::Stretch)
          .part(&self.part(NavigationMenuPart::Featured))
          .focus_group(FocusGroup::none())
          .child(featured),
      );
    }
    let per = panel.links.len().div_ceil(panel.columns).max(1);
    for (column_at, links) in panel.links.chunks(per).enumerate() {
      let mut column = Column::new()
        .align_items(Alignment::Stretch)
        .overflow_visible()
        .part(&self.part(NavigationMenuPart::Column));
      for (in_column, link) in links.iter().enumerate() {
        let at = column_at * per + in_column;
        let first = first.as_ref().filter(|_| Some(at) == first_at);
        column = column.child(self.link(index, at, link, first));
      }
      row = row.child(column);
    }
    if let Some(item) = self.items.get(index) {
      row = row.describe("a11y_name", item.label.clone());
    }
    if let Some(kept) = self.shared.kept(index) {
      row = row.ref_element(kept.panel);
    }
    Some(row)
  }

  /// The panel of the item `index` hung from its item, open while the menu's open panel is it: the room the pointer
  /// crosses to it, which keeps it as the panel does, and the panel. Opened from the keyboard it takes the focus;
  /// opened by the pointer the focus stays where it is, and Tab goes from the item into it. `None` for an item with
  /// no panel.
  pub fn panel(&mut self, index: usize) -> Option<Element> {
    let content = self.panel_content(index)?;
    let kept = self.shared.kept(index)?;
    let mut viewport = Column::new()
      .align_items(Alignment::Start)
      // The panel's shadow falls past the room it is in.
      .overflow_visible()
      .part(&self.part(NavigationMenuPart::Viewport))
      .child(content);
    if let Some(timers) = &self.timers {
      let (enter, leave) = (
        (self.shared.clone(), timers.clone()),
        (self.shared.clone(), timers.clone()),
      );
      viewport = viewport
        .on_mouse_enter(move || {
          let (shared, timers) = &enter;
          shared.pointer.lock().on_panel = true;
          timers.closing.cancel();
        })
        .on_mouse_leave(move || {
          let (shared, timers) = &leave;
          shared.pointer.lock().on_panel = false;
          if shared.opened() != Opened::Key {
            timers.closing.restart();
          }
        });
    }
    let keyed = self.open == Some(index) && self.shared.opened() == Opened::Key;
    let popup = Popup::new(kept.anchor, viewport)
      .open(kept.open)
      .placement(Placement::BottomStart)
      .collision(CollisionStrategy::FlipThenClamp)
      // The room between the item and the panel is the pointer's way to it: it is hit.
      .hit_test(HitTestBehavior::Auto)
      // A press on the page does what it does, as well as closing the panel.
      .outside_press(OutsidePress::PassThrough);
    let popup = if keyed {
      popup.takes_focus(true)
    } else {
      popup.tab_after_anchor(true)
    };
    Some(popup.into())
  }

  /// How the default renderer builds a menu: its items in a row, each its icon, its text and its chevron, with its
  /// panel, which takes no room, after it.
  pub fn render(mut self) -> Element {
    let mut bar = self.bar();
    for index in 0..self.items.len() {
      let item = self
        .item(index)
        .with_children(self.item_icon(index))
        .child(self.label(index))
        .with_children(self.chevron(index));
      match self.panel(index) {
        Some(panel) => bar = bar.child(Row::new().overflow_visible().child(item).child(panel)),
        None => bar = bar.child(item),
      }
    }
    bar.into()
  }
}

impl Themed for NavigationMenu {
  const NAME: &'static str = "navigation-menu";
  type Renderer = NavigationMenuRenderer;

  /// The design's navigation menu (`pencil/lurq-design-system.pen`, Navigation Menu): items 4 apart, each 36 tall,
  /// padded 12 at the sides with the medium radius, its text 13/500 in the foreground and, where it opens a panel, a
  /// chevron of 14 in the muted foreground 6 after it; the item whose panel is open is filled with the highlight (the
  /// board's accent), as one under the pointer or focused from the keyboard is. The panel is 6 under the item, in
  /// the popover's colors, with an edge of 1 in the border's color, the large radius and the medium shadow, padded 8
  /// with 8 between its columns, each 208 wide. A link of a panel is padded 12 with the medium radius, its title
  /// 14/500 over its description in 13 and the muted foreground, 4 apart and 1.4 high; it is filled with the
  /// highlight under the pointer and focused. The panel fades in over the theme's base duration and out over the
  /// fast one. In a navbar (the links variant, Navbar on the board) an item is its text alone, 20 apart, in the muted
  /// foreground, and in the foreground under the pointer, while open, and where it is the page shown.
  fn recipe() -> Recipe {
    use NavigationMenuPart::*;
    use NavigationMenuState::{Current, Disabled, Open};
    use NavigationMenuVariant::Links;
    use PaletteColor::{
      Border as Edge, Foreground, Highlight, Muted, MutedForeground, Popover as Fill, PopoverForeground,
    };
    let clear = || Color::new(0, 0, 0, 0);
    let links = || Selector::new().variant(Links);
    Recipe::new()
      .base(Bar, |s| s.gap(4.0))
      .base(Item, |s| {
        fading(
          s.height(36.0)
            .padding_x(12.0)
            .gap(6.0)
            .radius(RadiusSize::Md)
            .foreground(Foreground)
            .cursor(CursorIcon::Pointer),
        )
      })
      .base(ItemIcon, |s| s.square(IconSize::Md))
      .base(Label, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .weight(FontWeight::Medium)
      })
      .base(Chevron, |s| s.square(IconSize::Sm).foreground(MutedForeground))
      .hovered(Item, |s| s.background(Highlight))
      .focus_visible(Item, |s| s.background(Highlight))
      .state(Open, Item, |s| s.background(Highlight))
      .state(Current, Item, |s| s.background(Muted))
      .state(Disabled, Item, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
      .base(Viewport, |s| s.padding_top(6.0))
      .base(Panel, |s| {
        s.padding_x(8.0)
          .padding_y(8.0)
          .gap(8.0)
          .radius(RadiusSize::Lg)
          .background(Fill)
          .foreground(PopoverForeground)
          .border(Border::inside(BorderSize::Sm, Edge))
          .shadow(ShadowStyle::Md)
          .enter(PartAnimation::new(FADE_IN, MotionDuration::Base, MotionEasing::Enter))
          .exit(PartAnimation::new(FADE_OUT, MotionDuration::Fast, MotionEasing::Exit))
      })
      .base(Column, |s| s.width(208.0).gap(4.0))
      .base(Featured, |s| s.width(208.0).radius(RadiusSize::Md))
      .base(Link, |s| {
        fading(
          s.padding_x(12.0)
            .padding_y(12.0)
            .gap(12.0)
            .radius(RadiusSize::Md)
            .cursor(CursorIcon::Pointer),
        )
      })
      .base(LinkIcon, |s| {
        s.square(IconSize::Md)
          .padding_top(2.0)
          .foreground(MutedForeground)
      })
      .base(LinkText, |s| s.gap(4.0))
      .base(LinkTitle, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(14.0)
          .weight(FontWeight::Medium)
          .foreground(Foreground)
      })
      .base(LinkDescription, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .line_height(1.4)
          .foreground(MutedForeground)
      })
      .hovered(Link, |s| s.background(Highlight))
      .focus_visible(Link, |s| s.background(Highlight))
      .state(Current, Link, |s| s.background(Muted))
      .state(Disabled, Link, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
      // The links of a navbar: the text alone, which the pointer, the open panel and the page shown bring forward.
      .when(links(), Bar, |s| s.gap(20.0))
      .when(links(), Item, |s| {
        s.height(32.0)
          .padding_x(0.0)
          .gap(6.0)
          .radius(RadiusSize::Sm)
          .foreground(MutedForeground)
      })
      .when(links().hovered(), Item, |s| s.background(clear()).foreground(Foreground))
      .when(links().focus_visible(), Item, |s| {
        s.background(clear()).foreground(Foreground).focus_ring()
      })
      .when(links().state(Open), Item, |s| s.background(clear()).foreground(Foreground))
      .when(links().state(Current), Item, |s| s.background(clear()).foreground(Foreground))
      .when(links(), Viewport, |s| s.padding_top(18.0))
  }
}
