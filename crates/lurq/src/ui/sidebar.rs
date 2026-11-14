//! A sidebar: the way round an app, down one side of its window.
//!
//! It is a mounted component, not a builder that converts into an element:
//! the names of its items on the rail are tooltips, which are mounted; the
//! nested lists it opens and closes, and the menus the rail opens beside an
//! item, are kept from one render to the next, each with its open signal and
//! its anchor, none of which a builder can keep. [`Sidebar`] says what it
//! holds, and [`Sidebar::mount`] puts it where it goes.

use std::{collections::HashMap, sync::Arc};

use parking_lot::Mutex;

use super::{
  FontFamily,
  button::{Button, ButtonVariant},
  collapsible::opening,
  drawer::Drawer,
  icon::Icon,
  icons,
  kbd::{Kbd, KeyPlatform},
  menu::{Menu, MenuItem},
  names,
  part_style::{Length, PartStyle, PartTransition},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{fading, glyph, pressable},
  theme::{Themed, current_theme},
  tooltip::{Tooltip, TooltipSide},
};
use crate::{
  animation::Transition,
  app::{
    component::Component,
    ctx::{Ctx, OpenState, Placement},
    events::KeyboardEvent,
    theme::{
      BorderSize, Breakpoint, Density, IconSize, MotionDuration, MotionEasing, OpacityLevel, PaletteColor, RadiusSize,
      Theme, TypographyStyle,
    },
  },
  components::{Column, Row, ScrollVertical},
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::Justify, scrollbar::ScrollBarStyle, text_style::FontWeight},
  node::{
    CursorIcon, Element, FocusGroup, border::Border as Edge, color::Color, dimension::Dimension, outline::Outline,
  },
};

/// The parts of a sidebar a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SidebarPart {
  /// The sidebar: its width, its fill and the line along the edge it shows the page. Its width moves between the
  /// sidebar's and the rail's.
  Root,
  /// What stays at the top: the app's header, a workspace or the app with its mark.
  Header,
  /// The groups of items, which scroll where they are more than the sidebar's height.
  Body,
  /// What stays at the bottom: the room of the app's footer at the sidebar's sides.
  Footer,
  /// The footer's content, with the line over it: the person signed in, the settings.
  FooterContent,
  /// A group: its label over its items.
  Group,
  /// The row of a group's label, its count and its chevron; pressed, where the group opens and closes.
  GroupLabel,
  /// The text of a group's label.
  GroupText,
  /// How many items a group has, at the end of its label.
  GroupCount,
  /// The chevron of a group that opens and closes.
  GroupChevron,
  /// The room of the action at the end of a group's label.
  GroupAction,
  /// An item: what is pressed.
  Item,
  /// The icon of an item.
  ItemIcon,
  /// The name of an item.
  ItemLabel,
  /// The count at the end of an item.
  ItemCount,
  /// The room of the badge at the end of an item.
  ItemBadge,
  /// The room of the keys at the end of an item.
  ItemShortcut,
  /// The chevron of an item with nested items, before its icon.
  ItemChevron,
  /// A step of a nested item's indent: one for each list it is nested in, before what it holds.
  ItemIndent,
  /// What opens and closes: the nested items of an item, or the items of a group that opens and closes. Of no
  /// height while it is closed and of its content's while it is open.
  Nested,
  /// What a nested list holds while it is open.
  NestedContent,
  /// A [`SidebarHeading`]: the row of a mark, a title and a description, with an icon at its end.
  Heading,
  /// The mark of a heading: the app's or the workspace's icon on its fill.
  HeadingMark,
  /// The title and the description of a heading, one over the other.
  HeadingText,
  HeadingTitle,
  HeadingDescription,
  /// The icon at the end of a heading.
  HeadingTrailing,
}

names!(SidebarPart {
  Root => "root",
  Header => "header",
  Body => "body",
  Footer => "footer",
  FooterContent => "footer-content",
  Group => "group",
  GroupLabel => "group-label",
  GroupText => "group-text",
  GroupCount => "group-count",
  GroupChevron => "group-chevron",
  GroupAction => "group-action",
  Item => "item",
  ItemIcon => "item-icon",
  ItemLabel => "item-label",
  ItemCount => "item-count",
  ItemBadge => "item-badge",
  ItemShortcut => "item-shortcut",
  ItemChevron => "item-chevron",
  ItemIndent => "item-indent",
  Nested => "nested",
  NestedContent => "nested-content",
  Heading => "heading",
  HeadingMark => "heading-mark",
  HeadingText => "heading-text",
  HeadingTitle => "heading-title",
  HeadingDescription => "heading-description",
  HeadingTrailing => "heading-trailing",
});

/// The states of a sidebar a recipe styles, besides what the pointer and the
/// keyboard do to its items.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SidebarState {
  /// It is the rail: its items' icons alone.
  Collapsed,
  /// It lies over the page, on a narrow window.
  Overlay,
  /// Of an item: it is where the app is. On the rail, an item whose nested item is.
  Selected,
  /// Of an item: it takes no input.
  Disabled,
  /// Of an item with nested items, or of a group that opens and closes: it is open.
  Open,
  /// Of a group's label or a heading: it is pressed.
  Interactive,
}

names!(SidebarState {
  Collapsed => "collapsed",
  Overlay => "overlay",
  Selected => "selected",
  Disabled => "disabled",
  Open => "open",
  Interactive => "interactive",
});

/// The kinds of [`SidebarHeading`], as the board draws them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum SidebarHeadingVariant {
  /// The header's: a workspace or the app, its title in the strong weight.
  #[default]
  Workspace,
  /// The footer's: the person signed in, their name in the medium weight.
  Account,
}

names!(SidebarHeadingVariant {
  Workspace => "workspace",
  Account => "account",
});

/// What builds a sidebar's nodes: a theme's in place of the default one.
pub type SidebarRenderer = Arc<dyn Fn(SidebarView) -> Element + Send + Sync>;

type Handler = Arc<dyn Fn() + Send + Sync>;

/// Whether the window is too narrow for a sidebar beside the page: narrower than the `md` breakpoint.
fn window_is_narrow() -> bool {
  super::current_breakpoint().is_none_or(|breakpoint| breakpoint < Breakpoint::Md)
}

/// Whether a key was pressed with no modifier held but Shift.
fn plain(event: &KeyboardEvent) -> bool {
  !event.ctrl && !event.alt && !event.meta
}

/// A place in a [`Sidebar`]: a page, a view, a project. An item with nested
/// items opens and closes them.
///
/// ```
/// use lurq::ui::{Badge, SidebarItem, icons};
///
/// let inbox = SidebarItem::new("Inbox").icon(icons::INBOX).count(12).selected(true);
/// let drafts = SidebarItem::new("Drafts").icon(icons::FILE).badge(Badge::new("New")).shortcut("Mod+D");
/// let components = SidebarItem::new("Components")
///   .icon(icons::FOLDER)
///   .child(SidebarItem::new("Buttons").icon(icons::FILE))
///   .child(SidebarItem::new("Inputs").icon(icons::FILE));
/// ```
///
/// Which item is selected is the app's, as where the app is: with a router,
/// `.selected(ctx.route_path() == "/inbox")` on the item that goes there.
#[derive(Clone)]
pub struct SidebarItem {
  label: Arc<str>,
  icon: Option<Name>,
  badge: Option<Element>,
  count: Option<usize>,
  shortcut: Option<Arc<str>>,
  selected: bool,
  disabled: bool,
  open: Option<bool>,
  on_select: Vec<Handler>,
  href: Option<Arc<str>>,
  children: Vec<SidebarItem>,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl SidebarItem {
  /// An item called `label`.
  pub fn new(label: &str) -> Self {
    Self {
      label: Arc::from(label),
      icon: None,
      badge: None,
      count: None,
      shortcut: None,
      selected: false,
      disabled: false,
      open: None,
      on_select: Vec::new(),
      href: None,
      children: Vec::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// The icon before its name, by its name in the theme's set: what the rail shows of it.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// A badge at its end: a [`Badge`](super::Badge), "New".
  pub fn badge(mut self, badge: impl Into<Element>) -> Self {
    self.badge = Some(badge.into());
    self
  }

  /// How many things it holds, at its end: the unread of an inbox.
  pub fn count(mut self, count: usize) -> Self {
    self.count = Some(count);
    self
  }

  /// The keys that go to it, at its end, as [`Kbd::shortcut`] writes them: "Mod+1".
  pub fn shortcut(mut self, keys: &str) -> Self {
    self.shortcut = Some(Arc::from(keys));
    self
  }

  /// Whether it is where the app is.
  pub fn selected(mut self, selected: bool) -> Self {
    self.selected = selected;
    self
  }

  /// Whether it takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Whether its nested items are open when the sidebar first shows it: they
  /// are where one of them is selected, unless told otherwise. The sidebar
  /// keeps whether they are open from then on.
  pub fn open(mut self, open: bool) -> Self {
    self.open = Some(open);
    self
  }

  /// Called when it is chosen: pressed, or Enter or Space while it has the focus.
  pub fn on_select(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_select.push(Arc::new(handler));
    self
  }

  /// The path of the app's router it goes to when it is chosen. It is a link to tooling.
  #[cfg(feature = "router")]
  pub fn href(mut self, path: &str) -> Self {
    self.href = Some(Arc::from(path));
    self
  }

  /// A nested item, after those it holds already.
  pub fn child(mut self, child: SidebarItem) -> Self {
    self.children.push(child);
    self
  }

  /// Nested items, in order, after those it holds already.
  pub fn children(mut self, children: impl IntoIterator<Item = SidebarItem>) -> Self {
    self.children.extend(children);
    self
  }

  /// Draws it as it is under the pointer, held down or focused from the
  /// keyboard, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of what is pressed.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// What tells it apart from the items beside it, where two have one name: whether it is open is kept by it.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// What tells it apart from the items beside it.
  fn ident(&self) -> &str {
    self.key.as_deref().unwrap_or(&self.label)
  }

  /// Whether one of its nested items, at any depth, is selected.
  fn holds_selected(&self) -> bool {
    self
      .children
      .iter()
      .any(|child| child.selected || child.holds_selected())
  }
}

/// A group of items in a [`Sidebar`], under a label: "Project", "Team".
///
/// ```
/// use lurq::ui::{SidebarGroup, SidebarItem, icons};
///
/// let team = SidebarGroup::new("Team")
///   .count(2)
///   .collapsible(true)
///   .item(SidebarItem::new("Members").icon(icons::USERS))
///   .item(SidebarItem::new("Settings").icon(icons::SETTINGS));
/// ```
#[derive(Clone)]
pub struct SidebarGroup {
  label: Option<Arc<str>>,
  count: Option<usize>,
  action: Option<Element>,
  collapsible: bool,
  open: bool,
  items: Vec<SidebarItem>,
  key: Option<Arc<str>>,
}

impl SidebarGroup {
  /// A group under the label `label`; an empty one has none.
  pub fn new(label: &str) -> Self {
    Self {
      label: (!label.is_empty()).then(|| Arc::from(label)),
      count: None,
      action: None,
      collapsible: false,
      open: true,
      items: Vec::new(),
      key: None,
    }
  }

  /// How many it has, at the end of its label.
  pub fn count(mut self, count: usize) -> Self {
    self.count = Some(count);
    self
  }

  /// An action at the end of its label: a small ghost [`Button`] that adds to the group.
  pub fn action(mut self, action: impl Into<Element>) -> Self {
    self.action = Some(action.into());
    self
  }

  /// Whether its label opens and closes it.
  pub fn collapsible(mut self, collapsible: bool) -> Self {
    self.collapsible = collapsible;
    self
  }

  /// Whether a group that opens and closes is open when the sidebar first shows it. It is, unless told otherwise;
  /// the sidebar keeps whether it is open from then on.
  pub fn open(mut self, open: bool) -> Self {
    self.open = open;
    self
  }

  /// An item, after those it holds already.
  pub fn item(mut self, item: SidebarItem) -> Self {
    self.items.push(item);
    self
  }

  /// Items, in order, after those it holds already.
  pub fn items(mut self, items: impl IntoIterator<Item = SidebarItem>) -> Self {
    self.items.extend(items);
    self
  }

  /// What tells it apart from the groups beside it, where two have one label.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  fn ident(&self) -> &str {
    self.key.as_deref().or(self.label.as_deref()).unwrap_or("")
  }
}

/// A sidebar: the way round an app, down one side of its window. Its groups
/// of places, the one the app is at, a header with the app or the workspace,
/// a footer with who is signed in.
///
/// ```no_run
/// use lurq::{
///   app::ctx::Ctx,
///   core::Signal,
///   node::Element,
///   ui::{Sidebar, SidebarGroup, SidebarHeading, SidebarItem, icons},
/// };
///
/// fn navigation(ctx: &mut Ctx, collapsed: Signal<bool>, open: Signal<bool>) -> Element {
///   let heading = |collapsed| {
///     SidebarHeading::new("lurq").description("Workspace").icon(icons::BOX).collapsed(collapsed)
///   };
///   Sidebar::new()
///     .label("Workspace")
///     .header(heading(false))
///     .header_collapsed(heading(true))
///     .group(
///       SidebarGroup::new("Project")
///         .item(SidebarItem::new("Overview").icon(icons::HOME))
///         .item(SidebarItem::new("Inbox").icon(icons::INBOX).count(12).selected(true)),
///     )
///     .collapsed(collapsed)
///     .open(open)
///     .mount(ctx)
/// }
/// ```
///
/// Which item is selected is the app's. Collapsed ([`Sidebar::collapsed`])
/// it is the rail of the items' icons: its width moves to the rail's, each
/// item says its name in a tooltip beside it, and an item with nested items
/// opens them in a menu there. On a window narrower than the `md` breakpoint
/// it lies over the page as a [`Drawer`] does, while [`Sidebar::open`] holds
/// `true`, and closes when an item is chosen; [`SidebarTrigger`] opens and
/// collapses it from wherever the app puts it.
///
/// Its items are a tree to the keyboard: Tab stops at it once, on the
/// selected item, Up and Down move through the items that are shown, Home and
/// End go to the first and the last, and a letter to the item whose name
/// starts with it; Enter and Space choose. Right opens a closed item and goes
/// into an open one, Left closes an open item and goes from a nested one to
/// the item it is in; on the rail Right opens an item's menu. Its items
/// scroll where they are more than its height, its header and its footer
/// staying. To tooling it is a navigation region called by its label, its
/// items links or buttons, the selected one the current page, and a nested
/// list a group.
#[derive(Clone)]
pub struct Sidebar {
  label: Arc<str>,
  header: Option<Element>,
  header_collapsed: Option<Element>,
  footer: Option<Element>,
  footer_collapsed: Option<Element>,
  groups: Vec<SidebarGroup>,
  collapsed: OpenState,
  open: Option<Signal<bool>>,
  narrow: Option<bool>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Default for Sidebar {
  fn default() -> Self {
    Self::new()
  }
}

impl Sidebar {
  /// A sidebar with nothing in it, open beside the page.
  pub fn new() -> Self {
    Self {
      label: Arc::from("Sidebar"),
      header: None,
      header_collapsed: None,
      footer: None,
      footer_collapsed: None,
      groups: Vec::new(),
      collapsed: OpenState::Static(false),
      open: None,
      narrow: None,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// What it is called to tooling that reads the tree: "Workspace". "Sidebar" unless told otherwise.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Arc::from(label);
    self
  }

  /// What stays at its top: a [`SidebarHeading`] of the app or the workspace, a search.
  pub fn header(mut self, header: impl Into<Element>) -> Self {
    self.header = Some(header.into());
    self
  }

  /// What the rail shows at its top in place of the header: the header's mark alone. Nothing unless told.
  pub fn header_collapsed(mut self, header: impl Into<Element>) -> Self {
    self.header_collapsed = Some(header.into());
    self
  }

  /// What stays at its bottom: a [`SidebarHeading`] of who is signed in, the settings.
  pub fn footer(mut self, footer: impl Into<Element>) -> Self {
    self.footer = Some(footer.into());
    self
  }

  /// What the rail shows at its bottom in place of the footer: an avatar alone. Nothing unless told.
  pub fn footer_collapsed(mut self, footer: impl Into<Element>) -> Self {
    self.footer_collapsed = Some(footer.into());
    self
  }

  /// A group of items, after those it holds already.
  pub fn group(mut self, group: SidebarGroup) -> Self {
    self.groups.push(group);
    self
  }

  /// Groups of items, in order, after those it holds already.
  pub fn groups(mut self, groups: impl IntoIterator<Item = SidebarGroup>) -> Self {
    self.groups.extend(groups);
    self
  }

  /// An item in a group of no label, after what it holds already.
  pub fn item(mut self, item: SidebarItem) -> Self {
    match self.groups.last_mut() {
      Some(group) if group.label.is_none() && group.key.is_none() => group.items.push(item),
      _ => self.groups.push(SidebarGroup::new("").item(item)),
    }
    self
  }

  /// Whether it is the rail of the items' icons: a `bool`, or the app's
  /// `Signal<bool>`, which [`SidebarTrigger`] turns. Its width moves between
  /// the two.
  pub fn collapsed(mut self, collapsed: impl Into<OpenState>) -> Self {
    self.collapsed = collapsed.into();
    self
  }

  /// On a narrow window, whether it is open over the page: the app's signal,
  /// which [`SidebarTrigger`] sets, and choosing an item, Escape and a press
  /// on the veil clear. Without one it keeps its own, which only it closes.
  pub fn open(mut self, open: Signal<bool>) -> Self {
    self.open = Some(open);
    self
  }

  /// Whether it lies over the page, whatever the window's width: a sidebar
  /// does below the `md` breakpoint unless told.
  pub fn narrow(mut self, narrow: bool) -> Self {
    self.narrow = Some(narrow);
    self
  }

  /// Puts the sidebar in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this sidebar over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this sidebar.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of its root.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Puts the sidebar where it goes: beside the page, or over it on a narrow window.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    ctx.mount::<SidebarHost>(SidebarProps { sidebar: self })
  }
}

/// A sidebar.
#[derive(Clone)]
struct SidebarProps {
  sidebar: Sidebar,
}

/// The sidebar is built anew by every render of what holds it, so props are
/// never the same as before.
impl PartialEq for SidebarProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for SidebarProps {}

/// What a sidebar keeps between renders.
struct SidebarHost {
  /// Whether each nested list and each group that opens and closes is open, by where it is in the sidebar.
  opened: Mutex<HashMap<Arc<str>, Signal<bool>>>,
  /// The anchor and the open signal of each menu the rail opens beside an item, by where the item is.
  menus: Mutex<HashMap<Arc<str>, (ElementRef, Signal<bool>)>>,
  /// Whether it is open over the page, where the app gives no signal of its own.
  open: Signal<bool>,
}

impl Component for SidebarHost {
  type Props = SidebarProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      opened: Mutex::new(HashMap::new()),
      menus: Mutex::new(HashMap::new()),
      open: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let sidebar = ctx.props::<SidebarProps>().sidebar.clone();
    #[cfg(feature = "router")]
    let navigator = ctx.navigator();
    #[cfg(not(feature = "router"))]
    let navigator = None;
    build_sidebar(&sidebar, self, Some(ctx), navigator)
  }
}

impl SidebarHost {
  /// What keeps nothing past one build: for a sidebar converted into an element on its own.
  fn detached() -> Self {
    Self {
      opened: Mutex::new(HashMap::new()),
      menus: Mutex::new(HashMap::new()),
      open: Signal::new(false),
    }
  }
}

#[cfg(feature = "router")]
type Navigator = crate::router::Navigator;
#[cfg(not(feature = "router"))]
type Navigator = ();

/// Builds `sidebar` with what `host` keeps, in the theme of what is being rendered. Without `ctx` the rail's items
/// have no tooltips: there is nothing to mount them with.
fn build_sidebar(
  sidebar: &Sidebar,
  host: &SidebarHost,
  ctx: Option<&mut Ctx>,
  navigator: Option<Navigator>,
) -> Element {
  let theme = current_theme();
  let overlay = sidebar.narrow.unwrap_or_else(window_is_narrow);
  // Over the page it is the whole sidebar, whatever the rail would be.
  let collapsed = !overlay && sidebar.collapsed.is_open();
  let open = sidebar.open.clone().unwrap_or_else(|| host.open.clone());
  let build = Build {
    theme: theme.clone(),
    sidebar,
    collapsed,
    overlay,
    close: overlay.then(|| open.clone()),
    host,
    navigator,
  };
  let mut ctx = ctx;
  let groups: Vec<Element> = sidebar
    .groups
    .iter()
    .enumerate()
    .map(|(index, group)| build.group(&mut ctx, group, index))
    .collect();
  let (header, footer) = if collapsed {
    (sidebar.header_collapsed.clone(), sidebar.footer_collapsed.clone())
  } else {
    (sidebar.header.clone(), sidebar.footer.clone())
  };
  // The width it lies over the page at is the one it stands beside the page at.
  let width = build
    .recipe_in(false, false, &[])
    .part(SidebarPart::Root)
    .rest()
    .width
    .and_then(|width| match width.resolve(&theme)? {
      Dimension::Px(px) => Some(px),
      _ => None,
    });
  let view = SidebarView {
    label: sidebar.label.clone(),
    collapsed,
    overlay,
    header,
    footer,
    groups,
    recipe: build.recipe(&[]),
    open,
    width,
    id: sidebar.id.clone(),
    key: sidebar.key.clone(),
  };
  match theme.renderer::<Sidebar>() {
    Some(renderer) => renderer(view),
    None => view.render(),
  }
}

/// A sidebar converted into an element on its own is drawn as it is at rest, its nested lists as they start, with
/// no tooltips on the rail and nothing kept from one build to the next: for a gallery, a test, or a window of its
/// own. [`Sidebar::mount`] is what an app puts in its window.
impl From<Sidebar> for Element {
  fn from(sidebar: Sidebar) -> Self {
    build_sidebar(&sidebar, &SidebarHost::detached(), None, None)
  }
}

/// What a render of a sidebar builds its groups and items with.
struct Build<'a> {
  theme: Theme,
  sidebar: &'a Sidebar,
  collapsed: bool,
  overlay: bool,
  /// What choosing an item closes: the sidebar over the page.
  close: Option<Signal<bool>>,
  host: &'a SidebarHost,
  navigator: Option<Navigator>,
}

impl Build<'_> {
  /// The sidebar's recipe for a part of it in `states`, as the sidebar is now.
  fn recipe(&self, states: &[(SidebarState, bool)]) -> ResolvedRecipe {
    self.recipe_in(self.collapsed, self.overlay, states)
  }

  fn recipe_in(&self, collapsed: bool, overlay: bool, states: &[(SidebarState, bool)]) -> ResolvedRecipe {
    let mut case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.sidebar.states.clone(),
    }
    .state(SidebarState::Collapsed, collapsed)
    .state(SidebarState::Overlay, overlay);
    for (state, on) in states {
      case = case.state(*state, *on);
    }
    self.theme.resolve::<Sidebar>(&case, Some(&self.sidebar.rules))
  }

  /// Whether the list at `key` is open, kept from the first render, which starts it at `initial`.
  fn opened(&self, key: &str, initial: bool) -> Signal<bool> {
    self
      .host
      .opened
      .lock()
      .entry(Arc::from(key))
      .or_insert_with(|| Signal::new(initial))
      .clone()
  }

  /// The anchor and the open signal of the menu of the item at `key`.
  fn menu_state(&self, key: &str) -> (ElementRef, Signal<bool>) {
    self
      .host
      .menus
      .lock()
      .entry(Arc::from(key))
      .or_insert_with(|| (ElementRef::new(), Signal::new(false)))
      .clone()
  }

  /// What choosing `item` does: its handlers, its path, and closing the sidebar over the page. `None` for an
  /// item that does nothing when it is chosen.
  fn choose(&self, item: &SidebarItem) -> Option<Handler> {
    if item.disabled {
      return None;
    }
    let handlers = item.on_select.clone();
    let go = item.href.clone().zip(self.navigator.clone());
    if handlers.is_empty() && go.is_none() {
      return None;
    }
    let close = self.close.clone();
    Some(Arc::new(move || {
      for handler in &handlers {
        handler();
      }
      #[cfg(feature = "router")]
      if let Some((path, navigator)) = &go {
        navigator.push(path.to_string());
      }
      if let Some(close) = &close {
        close.set(false);
      }
    }))
  }

  /// A group: its label, and its items under it, which open and close with its label where it is collapsible. The
  /// rail shows its items alone.
  fn group(&self, ctx: &mut Option<&mut Ctx>, group: &SidebarGroup, index: usize) -> Element {
    let key = format!("{index}:{}", group.ident());
    let collapsible = group.collapsible && !self.collapsed;
    let open = if collapsible {
      self.opened(&key, group.open).get()
    } else {
      true
    };
    let recipe = self.recipe(&[(SidebarState::Open, open), (SidebarState::Interactive, collapsible)]);
    let mut root = Column::new()
      .align_items(if self.collapsed {
        Alignment::Center
      } else {
        Alignment::Stretch
      })
      .overflow_visible()
      .part(&recipe.part(SidebarPart::Group))
      .describe("a11y_role", "group");
    if let Some(label) = &group.label {
      root = root.describe("a11y_name", label.clone());
    }
    if !self.collapsed && (group.label.is_some() || group.count.is_some() || group.action.is_some()) {
      root = root.child(self.group_label(group, &recipe, &key, open, collapsible));
    }
    // To the keys a group that opens and closes is an item its items are in.
    let level = if collapsible { 2 } else { 1 };
    let items: Vec<Element> = group
      .items
      .iter()
      .map(|item| self.item(ctx, item, &format!("{key}/{}", item.ident()), 0, level))
      .collect();
    if !collapsible {
      return root.with_children(items).into();
    }
    root.child(nested(&recipe, open, group.label.as_deref(), items)).into()
  }

  /// The row of a group's label, its count, its chevron and its action. A group that opens and closes is opened
  /// and closed by a press of it.
  fn group_label(
    &self,
    group: &SidebarGroup,
    recipe: &ResolvedRecipe,
    key: &str,
    open: bool,
    collapsible: bool,
  ) -> Row {
    let part = recipe.part(SidebarPart::GroupLabel);
    let text = group.label.as_deref().map(|label| {
      let part = recipe.part(SidebarPart::GroupText);
      // The board writes a group's label in capitals.
      Row::new()
        .flex(1.0)
        .part(&part)
        .child(part.text(&label.to_uppercase()).nowrap())
    });
    let count = group.count.map(|count| {
      let part = recipe.part(SidebarPart::GroupCount);
      Row::new().part(&part).child(part.text(&count.to_string()).nowrap())
    });
    let mut label: Element = if collapsible {
      let (signal, toggle) = (self.opened(key, group.open), self.opened(key, group.open));
      let name = group.label.clone().unwrap_or_else(|| Arc::from("Group"));
      let mut button = pressable(&part, false)
        .flex(1.0)
        .justify(Justify::Start)
        .align_items(Alignment::Center)
        .describe("a11y_role", "button")
        .describe("a11y_name", name.clone())
        .describe("a11y_expanded", if open { "true" } else { "false" })
        .focus_label(name)
        .focus_level(1)
        .focus_expanded(open, move |open| toggle.set(open))
        .on_click(move |_| signal.update(|open| *open = !*open));
      button = button.with_children(text).with_children(count).child(glyph(
        &recipe.part(SidebarPart::GroupChevron),
        Name::new(if open {
          icons::CHEVRON_DOWN
        } else {
          icons::CHEVRON_RIGHT
        }),
        false,
      ));
      button.into()
    } else {
      Row::new()
        .flex(1.0)
        .align_items(Alignment::Center)
        .part(&part)
        .with_children(text)
        .with_children(count)
        .into()
    };
    // The action is beside the label, not in it, and the arrow keys of the items pass it by.
    if let Some(action) = &group.action {
      // As wide as the group, so that the label's count stands at its end, before the action.
      label = Row::new()
        .flex(1.0)
        .align_items(Alignment::Center)
        .overflow_visible()
        .child(label)
        .child(
          Row::new()
            .align_items(Alignment::Center)
            .focus_group(FocusGroup::none())
            .overflow_visible()
            .part(&recipe.part(SidebarPart::GroupAction))
            .child(action.clone()),
        )
        .into();
    }
    Row::new()
      .align_items(Alignment::Stretch)
      .overflow_visible()
      .child(label)
  }

  /// An item at `depth` of nesting and at `level` of the tree the keys go through, with its nested items under it,
  /// or on the rail its icon with its name in a tooltip and its nested items in a menu beside it.
  fn item(&self, ctx: &mut Option<&mut Ctx>, item: &SidebarItem, key: &str, depth: usize, level: u32) -> Element {
    let has_children = !item.children.is_empty();
    let open_signal =
      (has_children && !self.collapsed).then(|| self.opened(key, item.open.unwrap_or_else(|| item.holds_selected())));
    let open = open_signal.as_ref().is_some_and(Signal::get);
    // On the rail an item stands for its nested items, which it does not show.
    let selected = item.selected || (self.collapsed && item.holds_selected());
    let recipe = self.recipe(&[
      (SidebarState::Selected, selected),
      (SidebarState::Disabled, item.disabled),
      (SidebarState::Open, open),
    ]);
    let part = |name: SidebarPart| recipe.part(name).forced(item.forced);
    let mut button = pressable(&part(SidebarPart::Item), item.disabled)
      .justify(if self.collapsed {
        Justify::Center
      } else {
        Justify::Start
      })
      .align_items(Alignment::Center)
      .describe("a11y_role", if item.href.is_some() { "link" } else { "button" })
      .describe("a11y_name", item.label.clone())
      // Typing in the list matches its name, not its count or its keys.
      .focus_label(item.label.clone())
      .focus_level(level);
    if selected {
      button = button.describe("a11y_current", "page").focus_entry(true);
    }
    if let Some(id) = &item.id {
      button = button.id(id.clone());
    }
    let choose = self.choose(item);
    let mut menu = None;
    if self.collapsed && has_children {
      let (anchor, menu_open) = self.menu_state(key);
      button = button
        .ref_element(anchor.clone())
        .describe("a11y_haspopup", "menu")
        .describe("a11y_expanded", if menu_open.get() { "true" } else { "false" });
      if !item.disabled {
        let (press, right) = (menu_open.clone(), menu_open.clone());
        button = button
          .on_click(move |_| press.update(|open| *open = !*open))
          .on_key_down(move |event: KeyboardEvent| {
            if event.focus_within && plain(&event) && event.key == "ArrowRight" {
              event.prevent_default();
              right.set(true);
            }
          });
      }
      menu = Some(self.menu(item, key, anchor, menu_open));
    } else if let Some(signal) = &open_signal {
      button = button.describe("a11y_expanded", if open { "true" } else { "false" });
      if !item.disabled {
        let (press, keys) = (signal.clone(), signal.clone());
        let choose = choose.clone();
        button = button
          .on_click(move |_| {
            press.update(|open| *open = !*open);
            if let Some(choose) = &choose {
              choose();
            }
          })
          // The list is a tree to the keys: Right opens it and goes into it, Left closes it.
          .focus_expanded(open, move |open| keys.set(open));
      }
    } else if let Some(choose) = choose {
      button = button.on_click(move |_| choose());
    }

    if !self.collapsed {
      for _ in 0..depth {
        button = button.child(Row::new().part(&part(SidebarPart::ItemIndent)));
      }
      if has_children {
        let chevron = if open {
          icons::CHEVRON_DOWN
        } else {
          icons::CHEVRON_RIGHT
        };
        button = button.child(glyph(&part(SidebarPart::ItemChevron), Name::new(chevron), false));
      }
    }
    match item.icon {
      Some(icon) => button = button.child(glyph(&part(SidebarPart::ItemIcon), icon, false)),
      // On the rail an item of no icon is the first letter of its name.
      None if self.collapsed => {
        let initial: String = item.label.chars().take(1).collect();
        let label = part(SidebarPart::ItemLabel);
        button = button.child(Row::new().part(&label).child(label.text(&initial).nowrap()));
      }
      None => {}
    }
    if !self.collapsed {
      let label = part(SidebarPart::ItemLabel);
      button = button.child(
        Row::new()
          .flex(1.0)
          .part(&label)
          .child(label.text(&item.label).nowrap()),
      );
      if let Some(badge) = &item.badge {
        button = button.child(
          Row::new()
            .align_items(Alignment::Center)
            .overflow_visible()
            .part(&part(SidebarPart::ItemBadge))
            .child(badge.clone()),
        );
      }
      if let Some(count) = item.count {
        let count_part = part(SidebarPart::ItemCount);
        button = button.child(
          Row::new()
            .part(&count_part)
            .child(count_part.text(&count.to_string()).nowrap()),
        );
      }
      if let Some(keys) = &item.shortcut {
        button = button.child(
          Row::new()
            .align_items(Alignment::Center)
            .part(&part(SidebarPart::ItemShortcut))
            .child(Kbd::shortcut(keys).platform(KeyPlatform::current()).build(&self.theme)),
        );
      }
    }

    if self.collapsed {
      let element: Element = Row::new()
        .align_items(Alignment::Center)
        .overflow_visible()
        .child(button)
        .with_children(menu)
        .into();
      let mut tooltip = Tooltip::new(&item.label).side(TooltipSide::Right);
      if let Some(keys) = &item.shortcut {
        tooltip = tooltip.shortcut(keys);
      }
      return match ctx.as_deref_mut() {
        Some(ctx) => tooltip.mount(ctx, element),
        None => element,
      };
    }
    if !has_children {
      return button.into();
    }
    let children: Vec<Element> = item
      .children
      .iter()
      .map(|child| self.item(ctx, child, &format!("{key}/{}", child.ident()), depth + 1, level + 1))
      .collect();
    Column::new()
      .align_items(Alignment::Stretch)
      .overflow_visible()
      .child(button)
      .child(nested(&recipe, open, Some(&item.label), children))
      .into()
  }

  /// The menu the rail opens beside an item with nested items: one of each, an item with nested items of its own a
  /// submenu.
  fn menu(&self, item: &SidebarItem, key: &str, anchor: ElementRef, open: Signal<bool>) -> Menu {
    let mut menu = Menu::new(anchor, open)
      .placement(Placement::RightStart)
      .label(&item.label);
    for child in &item.children {
      menu = menu.item(self.menu_item(child, &format!("{key}/{}", child.ident())));
    }
    menu
  }

  fn menu_item(&self, item: &SidebarItem, key: &str) -> MenuItem {
    let mut entry = MenuItem::new(&item.label).disabled(item.disabled);
    // What is pressed has the item's id, on the rail as beside the page.
    if let Some(id) = &item.id {
      entry = entry.id(id.clone());
    }
    if let Some(icon) = item.icon {
      entry = entry.icon(icon);
    }
    if let Some(keys) = &item.shortcut {
      entry = entry.shortcut(keys);
    }
    if !item.children.is_empty() {
      let (anchor, open) = self.menu_state(key);
      return entry.submenu(self.menu(item, key, anchor, open));
    }
    // The one the app is at is marked, as a chosen tab is in the menu of the tabs that do not fit.
    entry = entry.radio(item.selected);
    if let Some(choose) = self.choose(item) {
      entry = entry.on_select(move || choose());
    }
    entry
  }
}

/// What opens and closes with its height: `children` while it is `open`, called `label` to tooling. Its content is
/// in the list of the arrow keys around it, as the items it holds are items of the sidebar.
fn nested(recipe: &ResolvedRecipe, open: bool, label: Option<&str>, children: Vec<Element>) -> Column {
  let mut panel = Column::new()
    .align_items(Alignment::Stretch)
    .clip()
    .presence()
    .part(&recipe.part(SidebarPart::Nested))
    .describe("a11y_role", "group");
  if let Some(label) = label {
    panel = panel.describe("a11y_name", Arc::<str>::from(label));
  }
  if open {
    // Keyed, so that what it held is kept as it closes, only painted.
    panel = panel.child(
      Column::new()
        .key("content")
        .align_items(Alignment::Stretch)
        .part(&recipe.part(SidebarPart::NestedContent))
        .with_children(children),
    );
  }
  panel
}

/// A sidebar as a renderer sees it: its groups built, its header and footer
/// as it is now, and the style of its parts.
pub struct SidebarView {
  pub label: Arc<str>,
  /// Whether it is the rail.
  pub collapsed: bool,
  /// Whether it lies over the page.
  pub overlay: bool,
  /// What stays at its top as it is now: the header, or what the rail shows in its place.
  pub header: Option<Element>,
  /// What stays at its bottom as it is now.
  pub footer: Option<Element>,
  /// Its groups, each with its items.
  pub groups: Vec<Element>,
  /// The style of each part of this sidebar, as it is now.
  pub recipe: ResolvedRecipe,
  open: Signal<bool>,
  width: Option<f32>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl SidebarView {
  /// The style of a part, as this sidebar is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The signal that holds whether it is open over the page.
  pub fn open(&self) -> &Signal<bool> {
    &self.open
  }

  fn alignment(&self) -> Alignment {
    if self.collapsed {
      Alignment::Center
    } else {
      Alignment::Stretch
    }
  }

  /// The header in the style of its part, if there is one.
  pub fn header(&mut self) -> Option<Element> {
    let header = self.header.take()?;
    Some(
      Column::new()
        .align_items(self.alignment())
        .overflow_visible()
        .part(&self.part(SidebarPart::Header))
        .child(header)
        .into(),
    )
  }

  /// The footer in the style of its part, if there is one, its content under its line.
  pub fn footer(&mut self) -> Option<Element> {
    let footer = self.footer.take()?;
    Some(
      Column::new()
        .align_items(Alignment::Stretch)
        .overflow_visible()
        .part(&self.part(SidebarPart::Footer))
        .child(
          Column::new()
            .align_items(self.alignment())
            .overflow_visible()
            .part(&self.part(SidebarPart::FooterContent))
            .child(footer),
        )
        .into(),
    )
  }

  /// The room at the end of the body that keeps the scroll bar off its rows: the lane of the theme's bar (its
  /// width and its inset on either side, from the sidebar's edge), or the body's own padding where that is more.
  /// The bar is drawn over what scrolls, along its edge, and the body reaches the sidebar's edge.
  fn bar_room(&self) -> f32 {
    let theme = self.theme();
    let rest = self.part(SidebarPart::Body).rest();
    let own = rest
      .padding_end
      .or(rest.padding_x)
      .map(|padding| padding.resolve(&theme.spacing(), 0.0))
      .unwrap_or(0.0);
    let bar = theme.scrollbar();
    own.max(bar.width + bar.resolved_edge_inset() * 2.0)
  }

  /// The groups in the style of the body, in what scrolls where they are more than the sidebar's height. They
  /// are one list to the keyboard: Tab stops at it once, on the selected item, Up and Down move through it, and a
  /// letter goes to the item whose name starts with it.
  pub fn body(&mut self) -> Element {
    let mut body = Column::new()
      .width(Dimension::Pct(100.0))
      .align_items(self.alignment())
      .part(&self.part(SidebarPart::Body))
      // A tree to the keys beside the page: Left goes from a nested item to the one it is in. The rail is a list.
      .focus_group(
        FocusGroup::vertical()
          .one_stop(true)
          .typeahead(true)
          .nested(!self.collapsed),
      )
      .with_children(std::mem::take(&mut self.groups));
    // The bar is drawn over what scrolls, along the sidebar's edge: the rows end before its lane.
    if !self.collapsed {
      body = body.padding_right(self.bar_room());
    }
    let scroll = ScrollVertical::new(body)
      .width(Dimension::Pct(100.0))
      .flex_full(1.0, 1.0, None);
    // The rail has no room for a bar beside its icons: it scrolls with the wheel and the keys, and shows none.
    if self.collapsed {
      return scroll.scrollbar(ScrollBarStyle::hidden()).into();
    }
    scroll.into()
  }

  /// The sidebar in the style of its root, with `children` in it: a navigation region to tooling, called by its
  /// label.
  pub fn root(&self, children: Vec<Element>) -> Column {
    let mut root = Column::new()
      .align_items(self.alignment())
      .part(&self.part(SidebarPart::Root))
      .describe("a11y_role", "navigation")
      .describe("a11y_name", self.label.clone())
      .with_children(children);
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key
      && !self.overlay
    {
      root = root.key(key.clone());
    }
    root
  }

  /// How the default renderer builds a sidebar: the header, the groups and the footer in its root, beside the
  /// page, or over the page in a drawer.
  pub fn render(mut self) -> Element {
    let children = [self.header(), Some(self.body()), self.footer()]
      .into_iter()
      .flatten()
      .collect();
    let root = self.root(children);
    if !self.overlay {
      return root.into();
    }
    // The drawer does not scroll what it holds: the sidebar's items scroll between its header and its footer.
    let mut drawer = Drawer::new(self.open.clone())
      .scroll(false)
      .overlay(true)
      .label(&self.label)
      .child(root);
    if let Some(width) = self.width {
      drawer = drawer.width(width);
    }
    if let Some(key) = &self.key {
      drawer = drawer.key(key.clone());
    }
    drawer.build(self.theme())
  }
}

/// A row of a [`Sidebar`]'s header or footer: a mark, a title over a
/// description, and an icon at its end. The workspace or the app at the top,
/// who is signed in at the bottom.
///
/// ```
/// use lurq::ui::{Avatar, SidebarHeading, SidebarHeadingVariant, icons};
///
/// let workspace = SidebarHeading::new("lurq")
///   .description("Workspace")
///   .icon(icons::BOX)
///   .trailing_icon(icons::CHEVRONS_VERTICAL);
/// let account = SidebarHeading::new("Ivan Zaida")
///   .variant(SidebarHeadingVariant::Account)
///   .description("ivan@lurq.dev")
///   .mark(Avatar::new("Ivan Zaida"))
///   .trailing_icon(icons::MORE_HORIZONTAL);
/// // What the rail shows in its place: the mark alone.
/// let rail = SidebarHeading::new("lurq").icon(icons::BOX).collapsed(true);
/// ```
///
/// It is pressed where it is given a handler: a button to tooling, called by its title.
#[derive(Clone)]
pub struct SidebarHeading {
  title: Arc<str>,
  description: Option<Arc<str>>,
  icon: Option<Name>,
  mark: Option<Element>,
  trailing: Option<Name>,
  variant: SidebarHeadingVariant,
  collapsed: bool,
  on_click: Vec<Handler>,
  anchor: Option<ElementRef>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
}

impl SidebarHeading {
  /// A heading of `title`.
  pub fn new(title: &str) -> Self {
    Self {
      title: Arc::from(title),
      description: None,
      icon: None,
      mark: None,
      trailing: None,
      variant: SidebarHeadingVariant::Workspace,
      collapsed: false,
      on_click: Vec::new(),
      anchor: None,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
    }
  }

  /// The line under the title: "Workspace", an address.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// The mark: an icon of the theme's set on the mark's fill.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// The mark: an element of the app's in place of an icon, an [`Avatar`](super::Avatar).
  pub fn mark(mut self, mark: impl Into<Element>) -> Self {
    self.mark = Some(mark.into());
    self
  }

  /// The icon at its end: `chevrons-vertical` of a switcher, `more-horizontal` of a menu.
  pub fn trailing_icon(mut self, name: impl Into<Name>) -> Self {
    self.trailing = Some(name.into());
    self
  }

  pub fn variant(mut self, variant: SidebarHeadingVariant) -> Self {
    self.variant = variant;
    self
  }

  /// Whether it is its mark alone, as the rail shows it.
  pub fn collapsed(mut self, collapsed: bool) -> Self {
    self.collapsed = collapsed;
    self
  }

  /// Called on a press. A heading with a handler is pressed: a button.
  pub fn on_click(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_click.push(Arc::new(handler));
    self
  }

  /// The anchor of what it opens: the [`Menu`] of a workspace switcher hangs from it.
  pub fn anchor(mut self, anchor: ElementRef) -> Self {
    self.anchor = Some(anchor);
    self
  }

  /// Puts the heading in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this heading over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// Builds the heading in `theme`, in the sidebar's recipe. Converting a heading into an element does this in the
  /// theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let pressed = !self.on_click.is_empty() || self.anchor.is_some();
    let case = Case {
      variant: Some(self.variant.into()),
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(SidebarState::Collapsed, self.collapsed)
    .state(SidebarState::Interactive, pressed);
    let recipe = theme.resolve::<Sidebar>(&case, Some(&self.rules));
    let part = |name: SidebarPart| recipe.part(name);
    let mut children: Vec<Element> = Vec::new();
    match (self.mark, self.icon) {
      (Some(mark), _) => children.push(mark),
      (None, Some(icon)) => {
        let mark = part(SidebarPart::HeadingMark);
        children.push(
          Row::new()
            .justify(Justify::Center)
            .align_items(Alignment::Center)
            .part(&mark)
            .child(Icon::new(icon).size(Length::Icon(IconSize::Md)).build(theme))
            .into(),
        );
      }
      (None, None) => {}
    }
    if !self.collapsed {
      let title = part(SidebarPart::HeadingTitle);
      let mut text = Column::new()
        .flex(1.0)
        .part(&part(SidebarPart::HeadingText))
        .child(Row::new().part(&title).child(title.text(&self.title).nowrap()));
      if let Some(description) = &self.description {
        let line = part(SidebarPart::HeadingDescription);
        text = text.child(Row::new().part(&line).child(line.text(description).nowrap()));
      }
      children.push(text.into());
      if let Some(trailing) = self.trailing {
        children.push(glyph(&part(SidebarPart::HeadingTrailing), trailing, false));
      }
    }
    let root = part(SidebarPart::Heading);
    if !pressed {
      let mut row = Row::new()
        .align_items(Alignment::Center)
        .part(&root)
        .with_children(children);
      if let Some(id) = self.id {
        row = row.id(id);
      }
      return row.into();
    }
    let mut button = pressable(&root, false)
      .justify(if self.collapsed {
        Justify::Center
      } else {
        Justify::Start
      })
      .align_items(Alignment::Center)
      .describe("a11y_role", "button")
      .describe("a11y_name", self.title.clone())
      .with_children(children);
    if !self.on_click.is_empty() {
      let handlers = self.on_click;
      button = button.on_click(move |_| {
        for handler in &handlers {
          handler();
        }
      });
    }
    if let Some(anchor) = self.anchor {
      button = button.ref_element(anchor);
    }
    if let Some(id) = self.id {
      button = button.id(id);
    }
    button.into()
  }
}

impl From<SidebarHeading> for Element {
  fn from(heading: SidebarHeading) -> Self {
    heading.build(&current_theme())
  }
}

/// The button that collapses a [`Sidebar`] to its rail and brings it back,
/// or on a narrow window opens it over the page, wherever the app puts it: a
/// ghost button of the `panel-left` icon, called by which way it goes.
///
/// ```
/// use lurq::{core::Signal, ui::SidebarTrigger};
///
/// let (collapsed, open) = (Signal::new(false), Signal::new(false));
/// let trigger = SidebarTrigger::new(collapsed.clone()).open(open.clone());
/// ```
///
/// It is given the signals the sidebar is: whether it is collapsed, and
/// whether it is open over the page.
#[derive(Clone)]
pub struct SidebarTrigger {
  collapsed: Signal<bool>,
  open: Option<Signal<bool>>,
  narrow: Option<bool>,
  id: Option<Arc<str>>,
}

impl SidebarTrigger {
  /// A button that turns `collapsed`, the sidebar's.
  pub fn new(collapsed: Signal<bool>) -> Self {
    Self {
      collapsed,
      open: None,
      narrow: None,
      id: None,
    }
  }

  /// The signal of the sidebar over the page, which the button opens on a narrow window.
  pub fn open(mut self, open: Signal<bool>) -> Self {
    self.open = Some(open);
    self
  }

  /// Whether the window is narrow, as the sidebar is told: below the `md` breakpoint unless told.
  pub fn narrow(mut self, narrow: bool) -> Self {
    self.narrow = Some(narrow);
    self
  }

  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// Builds the button in `theme`.
  pub fn build(self, theme: &Theme) -> Element {
    let narrow = self.narrow.unwrap_or_else(window_is_narrow);
    let (label, turned) = match (&self.open, narrow) {
      (Some(open), true) => (if open.get() { "Close sidebar" } else { "Open sidebar" }, open.clone()),
      _ => (
        if self.collapsed.get() {
          "Expand sidebar"
        } else {
          "Collapse sidebar"
        },
        self.collapsed.clone(),
      ),
    };
    let mut button = Button::new(label)
      .variant(ButtonVariant::Ghost)
      .icon(icons::PANEL_LEFT)
      .icon_only(true)
      .on_click(move |_| turned.update(|on| *on = !*on));
    if let Some(id) = self.id {
      button = button.id(id);
    }
    button.build(theme)
  }
}

impl From<SidebarTrigger> for Element {
  fn from(trigger: SidebarTrigger) -> Self {
    trigger.build(&current_theme())
  }
}

impl Themed for Sidebar {
  const NAME: &'static str = "sidebar";
  type Renderer = SidebarRenderer;

  /// The design's sidebar (`pencil/lurq-design-system.pen`, Sidebar,
  /// Sidebar/Collapsed and Sidebar Item): 260 wide on the muted fill, a line
  /// of 1 in the border's color along its end, padded 8, 2 between rows. An
  /// item is 32 high, padded 8 across, 8 between its icon of 16 in the muted
  /// foreground and its name in 13; the selected one is on the background
  /// with an edge of the border's color, its name in the medium weight and
  /// its icon in the foreground. A count is 11 in the mono family. A group's
  /// label is 11/600 in capitals, 0.6 apart, in the muted foreground, with 10
  /// over it and 4 under it. A nested item is 18 further in, after the
  /// chevron of 14 of the item it is in. The heading of the header is 44
  /// high, its mark 28 on the primary fill, its title 13/600 over a
  /// description of 11; the footer's has a line of 1 over it and is 48 high.
  /// The rail is 52 wide, its items 36 by 32 with 4 between them, and its
  /// width moves over the theme's slow duration, as a drawer's does.
  fn recipe() -> Recipe {
    use PaletteColor::*;
    use SidebarHeadingVariant::Account;
    use SidebarPart::*;
    use SidebarState::{Collapsed, Disabled, Interactive, Open, Overlay, Selected};
    let line = || Edge::inside(BorderSize::Sm, PaletteColor::Border);
    let none = || Edge::inside(0.0, PaletteColor::Border);
    let clear = || Color::new(0, 0, 0, 0);
    let ring = || Outline::new(BorderSize::Md, Ring).offset(-2.0);
    let recipe = Recipe::new()
      .base(Root, |s| {
        // The 8 at its sides is its regions': the body reaches its edge, so that the scroll bar is not over its rows.
        s.width(260.0)
          .height(Length::Fill)
          .padding_y(8.0)
          .gap(2.0)
          .background(Muted)
          .foreground(Foreground)
          .end_line(line())
          .transition(PartTransition::new(Transition::width(), MotionDuration::Slow).easing(MotionEasing::Standard))
      })
      .state(Collapsed, Root, |s| s.width(52.0).gap(4.0))
      // Over the page it is what the drawer holds, as high as the drawer: the drawer has the fill and the edge.
      .state(Overlay, Root, |s| {
        s.width(Length::Fill)
          .background(clear())
          .end_line(none())
      })
      .base(Header, |s| s.gap(2.0).padding_x(8.0))
      .state(Collapsed, Header, |s| s.padding_bottom(8.0))
      .base(Body, |s| s.gap(2.0).padding_x(8.0))
      .state(Collapsed, Body, |s| s.gap(4.0))
      .base(Footer, |s| s.padding_x(8.0))
      .base(FooterContent, |s| s.gap(2.0).padding_y(2.0).top_line(line()))
      .state(Collapsed, FooterContent, |s| s.padding_y(0.0).top_line(none()))
      .base(Group, |s| s.gap(2.0))
      .state(Collapsed, Group, |s| s.gap(4.0))
      .base(GroupLabel, |s| {
        s.padding_x(8.0)
          .padding_top(10.0)
          .padding_bottom(4.0)
          .gap(8.0)
          .radius(RadiusSize::Md)
      })
      .state(Interactive, GroupLabel, |s| fading(s.cursor(CursorIcon::Pointer)))
      .when(Selector::new().state(Interactive).hovered(), GroupLabel, |s| {
        s.background(StateHover)
      })
      .when(Selector::new().state(Interactive).focus_visible(), GroupLabel, |s| {
        s.outline(ring())
      })
      .base(GroupText, |s| {
        s.typography(TypographyStyle::Label)
          .font_size(11.0)
          .weight(FontWeight::SemiBold)
          .letter_spacing(0.6)
          .foreground(MutedForeground)
      })
      .base(GroupCount, |s| {
        s.font_family(FontFamily::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      .base(GroupChevron, |s| s.square(14.0).foreground(MutedForeground))
      .base(Item, |s| {
        fading(
          s.height(32.0)
            .padding_x(8.0)
            .gap(8.0)
            .radius(RadiusSize::Md)
            .foreground(Foreground)
            .cursor(CursorIcon::Pointer),
        )
      })
      // A row of the density's height, as the gallery's own lists have it: 32 at the default.
      .density(Density::Compact, Item, |s| s.height(24.0))
      .density(Density::Comfortable, Item, |s| s.height(40.0))
      .density(Density::Touch, Item, |s| s.height(44.0))
      .state(Collapsed, Item, |s| s.width(36.0).padding_x(0.0))
      // The pointer's veil, on whatever fill the sidebar has.
      .hovered(Item, |s| s.background(StateHover))
      .pressed(Item, |s| s.background(StatePressed))
      // The ring is inside the item, so that the scroll the items are in does not cut it.
      .focus_visible(Item, |s| s.outline(ring()))
      .state(Selected, Item, |s| s.background(Background).border(line()))
      .when(Selector::new().state(Selected).hovered(), Item, |s| s.background(Background))
      .when(Selector::new().state(Selected).pressed(), Item, |s| s.background(Background))
      .state(Disabled, Item, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
      .when(Selector::new().state(Disabled).hovered(), Item, |s| s.background(clear()))
      .base(ItemIcon, |s| s.square(16.0).foreground(MutedForeground))
      .state(Selected, ItemIcon, |s| s.foreground(Foreground))
      .base(ItemLabel, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .foreground(Foreground)
      })
      .state(Selected, ItemLabel, |s| s.weight(FontWeight::Medium))
      .base(ItemCount, |s| {
        s.font_family(FontFamily::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      .base(ItemChevron, |s| s.square(14.0).foreground(MutedForeground))
      // With the item's gap after it, a step of 18.
      .base(ItemIndent, |s| s.width(10.0).height(1.0))
      .base(NestedContent, |s| s.gap(2.0))
      .base(Heading, |s| {
        s.height(44.0)
          .padding_x(8.0)
          .gap(10.0)
          .radius(RadiusSize::Md)
          .foreground(Foreground)
      })
      .state(Interactive, Heading, |s| fading(s.cursor(CursorIcon::Pointer)))
      .when(Selector::new().state(Interactive).hovered(), Heading, |s| {
        s.background(StateHover)
      })
      .when(Selector::new().state(Interactive).focus_visible(), Heading, |s| {
        s.outline(ring())
      })
      .state(Collapsed, Heading, |s| s.height(Length::Auto).padding_x(0.0).gap(0.0))
      .base(HeadingMark, |s| {
        s.square(28.0)
          .radius(RadiusSize::Md)
          .background(Primary)
          .foreground(PrimaryForeground)
      })
      .base(HeadingText, |s| s.gap(1.0))
      .base(HeadingTitle, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .weight(FontWeight::SemiBold)
          .foreground(Foreground)
      })
      .variant(Account, HeadingTitle, |s| s.weight(FontWeight::Medium))
      .base(HeadingDescription, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      .base(HeadingTrailing, |s| s.square(14.0).foreground(MutedForeground))
      .variant(Account, HeadingTrailing, |s| s.square(16.0).foreground(Foreground));
    opening(recipe, Open.into(), Nested.into(), NestedContent.into())
  }
}
