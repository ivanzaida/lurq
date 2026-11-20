//! A navbar: the way round a site or an app along the top of its window, its brand at the start, its links, and
//! what stands at its end.
//!
//! It is a mounted component, not a builder that converts into an element: its links are a [`NavigationMenu`],
//! which is mounted, and on a narrow window they are a [`Sidebar`] over the page, which is mounted too and whose
//! open signal the navbar keeps where the app gives none.

use std::sync::Arc;

use super::{
  button::{Button, ButtonVariant},
  icons, names,
  navigation_menu::{NavItem, NavLink, NavigationMenu, NavigationMenuVariant, OpenHandler},
  part_style::PartStyle,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{fading, glyph, pressable},
  sidebar::{Sidebar, SidebarItem},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{BorderSize, Breakpoint, IconSize, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::Row,
  core::Signal,
  layout::{Alignment, layout_kind::Sticky, text_style::FontWeight},
  node::{CursorIcon, Element, NodeUpdate, border::Border},
};

/// What a choice runs.
type Handler = Arc<dyn Fn() + Send + Sync>;

/// The parts of a navbar a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NavbarPart {
  /// The bar: its height, its fill, the line along its bottom, the room between what it holds.
  Root,
  /// The brand at the start: its mark beside its name, and what is pressed where it goes somewhere.
  Brand,
  /// The mark of the brand: an icon of the theme's set, or the element its media is.
  BrandMark,
  /// The name of the brand.
  BrandName,
  /// The room of the links, which takes what the bar has to spare.
  Links,
  /// What stands after the links: a search, actions, an avatar with its menu.
  Trailing,
}

names!(NavbarPart {
  Root => "root",
  Brand => "brand",
  BrandMark => "brand_mark",
  BrandName => "brand_name",
  Links => "links",
  Trailing => "trailing",
});

/// The states of a navbar a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NavbarState {
  /// The window is narrow: the links are behind the menu button.
  Narrow,
  /// It stays at the top while the page under it scrolls.
  Sticky,
  /// The brand is pressed: it goes somewhere.
  Interactive,
}

names!(NavbarState {
  Narrow => "narrow",
  Sticky => "sticky",
  Interactive => "interactive",
});

/// What builds a navbar's nodes: a theme's in place of the default one.
pub type NavbarRenderer = Arc<dyn Fn(NavbarView) -> Element + Send + Sync>;

/// Whether the window is too narrow for the links in the bar: narrower than the `md` breakpoint, as for a sidebar.
fn window_is_narrow() -> bool {
  super::current_breakpoint().is_none_or(|breakpoint| breakpoint < Breakpoint::Md)
}

/// The brand at the start of a [`Navbar`]: a mark beside a name, which goes home where it is given a handler.
///
/// ```
/// use lurq::ui::{NavbarBrand, icons};
///
/// let brand = NavbarBrand::new("lurq").icon(icons::BOX).on_select(|| println!("home"));
/// ```
///
/// To tooling it is a link called by its name where it is pressed.
#[derive(Clone)]
pub struct NavbarBrand {
  name: Arc<str>,
  icon: Option<Name>,
  media: Option<Element>,
  on_select: Vec<Handler>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl NavbarBrand {
  /// A brand called `name`.
  pub fn new(name: &str) -> Self {
    Self {
      name: Arc::from(name),
      icon: None,
      media: None,
      on_select: Vec::new(),
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// An icon of the theme's set as its mark.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// A picture of the app's as its mark, in place of an icon: a logo.
  pub fn media(mut self, media: impl Into<Element>) -> Self {
    self.media = Some(media.into());
    self
  }

  /// What pressing it does: going home.
  pub fn on_select(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_select.push(Arc::new(handler));
    self
  }

  /// Puts the brand in a state the theme's recipe for a navbar names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of the brand over what the theme's recipe for a navbar gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for a navbar, for the brand alone.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws it as it is under the pointer, held down or focused from the keyboard: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the brand: what is pressed, where it is.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// What tells the brand apart from what stands beside it.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the brand in `theme`, in the parts of a navbar.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(NavbarState::Interactive, !self.on_select.is_empty());
    let recipe = theme
      .resolve::<Navbar>(&case, Some(&self.rules))
      .owned_by(self.id.as_ref());
    let part = |name: NavbarPart| recipe.part(name).forced(self.forced);
    let mark = match (self.media, self.icon) {
      (Some(media), _) => Some(media),
      (None, Some(icon)) => Some(glyph(&part(NavbarPart::BrandMark), icon, false)),
      (None, None) => None,
    };
    let name_part = part(NavbarPart::BrandName);
    let name = Row::new().part(&name_part).child(name_part.text(&self.name).nowrap());
    if self.on_select.is_empty() {
      let mut brand = Row::new()
        .align_items(Alignment::Center)
        .part(&part(NavbarPart::Brand))
        .with_children(mark)
        .child(name);
      if let Some(id) = self.id {
        brand = brand.id(id);
      }
      if let Some(key) = self.key {
        brand = brand.key(key);
      }
      return brand.into();
    }
    let handlers = self.on_select;
    let mut brand = pressable(&part(NavbarPart::Brand), false)
      .align_items(Alignment::Center)
      .describe("a11y_role", "link")
      .describe("a11y_name", self.name.clone())
      .on_click(move |_| {
        for handler in &handlers {
          handler();
        }
      })
      .with_children(mark)
      .child(name);
    if let Some(id) = self.id {
      brand = brand.id(id);
    }
    if let Some(key) = self.key {
      brand = brand.key(key);
    }
    brand.into()
  }
}

impl From<NavbarBrand> for Element {
  fn from(brand: NavbarBrand) -> Self {
    brand.build(&current_theme())
  }
}

/// A navbar: the way round a site or an app along the top of its window. Its brand at the start, its links, some
/// of which open a panel of links, and at its end what the app puts there: a search, actions, an avatar with its
/// menu.
///
/// ```no_run
/// use lurq::{
///   app::ctx::Ctx,
///   node::Element,
///   ui::{Avatar, NavItem, NavLink, NavPanel, Navbar, NavbarBrand, icons},
/// };
///
/// fn header(ctx: &mut Ctx) -> Element {
///   Navbar::new()
///     .label("Main")
///     .brand(NavbarBrand::new("lurq").icon(icons::BOX))
///     .item(
///       NavItem::new("Docs")
///         .current(true)
///         .panel(NavPanel::new().link(NavLink::new("Getting started").href("/docs"))),
///     )
///     .item(NavItem::new("Blog").href("/blog"))
///     .trailing(Avatar::new("Ivan Zaida"))
///     .mount(ctx)
/// }
/// ```
///
/// Its links are a [`NavigationMenu`] in its links variant: each a stop of Tab, the page shown in the foreground,
/// one with a panel opening it under itself by a press, under the pointer after a moment, and with Enter, Space and
/// Down. On a window narrower than the `md` breakpoint (or wherever [`Navbar::narrow`] says) they go behind a menu
/// button at the bar's start, into a [`Sidebar`] over the page: an item with a panel holds its links there,
/// and choosing one closes it. [`Navbar::sticky`] keeps the bar at the top while the page under it scrolls.
///
/// To tooling the bar is a navigation region called by its label, its links links (the page shown the current
/// page), and the menu button a button that says whether the links over the page are expanded.
#[derive(Clone)]
pub struct Navbar {
  label: Arc<str>,
  menu_label: Arc<str>,
  brand: Option<Element>,
  items: Vec<NavItem>,
  trailing: Vec<Element>,
  sticky: bool,
  open: Option<Signal<Option<usize>>>,
  on_open_change: Vec<OpenHandler>,
  menu_open: Option<Signal<bool>>,
  on_menu_open_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  narrow: Option<bool>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Default for Navbar {
  fn default() -> Self {
    Self::new()
  }
}

impl Navbar {
  /// A bar with nothing in it.
  pub fn new() -> Self {
    Self {
      label: super::current_locale().navigation.main.clone(),
      menu_label: super::current_locale().navigation.menu.clone(),
      brand: None,
      items: Vec::new(),
      trailing: Vec::new(),
      sticky: false,
      open: None,
      on_open_change: Vec::new(),
      menu_open: None,
      on_menu_open_change: Vec::new(),
      narrow: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// What it is called to tooling that reads the tree: "Main". "Main" unless told otherwise.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Arc::from(label);
    self
  }

  /// What the button that opens the links over the page on a narrow window is called to tooling: "Menu" unless
  /// told. Whether the links are open it says on its own.
  pub fn menu_label(mut self, label: &str) -> Self {
    self.menu_label = Arc::from(label);
    self
  }

  /// What stands at its start: a [`NavbarBrand`], or an element of the app's.
  pub fn brand(mut self, brand: impl Into<Element>) -> Self {
    self.brand = Some(brand.into());
    self
  }

  /// A link, or what opens a panel of links, after those it holds.
  pub fn item(mut self, item: NavItem) -> Self {
    self.items.push(item);
    self
  }

  /// Items, in order, after those it holds.
  pub fn items(mut self, items: impl IntoIterator<Item = NavItem>) -> Self {
    self.items.extend(items);
    self
  }

  /// What stands after the links, after what stands there already: a [`SearchField`](super::SearchField), a
  /// [`Button`], an [`Avatar`](super::Avatar) with its [`Menu`](super::Menu).
  pub fn trailing(mut self, element: impl Into<Element>) -> Self {
    self.trailing.push(element.into());
    self
  }

  /// Whether it stays at the top of the page that scrolls it.
  pub fn sticky(mut self, sticky: bool) -> Self {
    self.sticky = sticky;
    self
  }

  /// Which item's panel is open, by its index among the items: the app's (see [`NavigationMenu::open`]).
  pub fn open(mut self, open: Signal<Option<usize>>) -> Self {
    self.open = Some(open);
    self
  }

  /// What hears a panel of its links open, another take its place, or the open one close (see
  /// [`NavigationMenu::on_open_change`]).
  pub fn on_open_change(mut self, handler: impl Fn(Option<usize>) + Send + Sync + 'static) -> Self {
    self.on_open_change.push(Arc::new(handler));
    self
  }

  /// What hears the menu button open the links over the page, or close them.
  pub fn on_menu_open_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_menu_open_change.push(Arc::new(handler));
    self
  }

  /// On a narrow window, whether the links are open over the page: the app's signal, which the menu button turns,
  /// and choosing a link, Escape and a press on the veil clear. Without one the bar keeps its own.
  pub fn menu_open(mut self, open: Signal<bool>) -> Self {
    self.menu_open = Some(open);
    self
  }

  /// Whether its links are behind the menu button, whatever the window's width: they are below the `md`
  /// breakpoint unless told.
  pub fn narrow(mut self, narrow: bool) -> Self {
    self.narrow = Some(narrow);
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

  /// Styles one part of this bar over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this bar.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the links as they are under the pointer, held down or focused from the keyboard, with nothing pointing
  /// at them: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the bar. Its links are `{id}-links`, with the ids a [`NavigationMenu`] gives its items from it
  /// (`{id}-links-0`, its panel `{id}-links-0-panel`), the menu button `{id}-menu`, and the links over the page
  /// `{id}-drawer`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Puts the bar where it goes, its links mounted with it.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    ctx.mount::<NavbarHost>(NavbarProps { navbar: self })
  }

  /// The links of the bar, in the links variant of a navigation menu.
  fn menu(&self) -> NavigationMenu {
    let mut menu = NavigationMenu::new()
      .label(&self.label)
      .variant(NavigationMenuVariant::Links)
      .landmark(false)
      .force(self.forced)
      .items(self.items.iter().cloned());
    if let Some(open) = &self.open {
      menu = menu.open(open.clone());
    }
    for handler in &self.on_open_change {
      let handler = handler.clone();
      menu = menu.on_open_change(move |open| handler(open));
    }
    if let Some(id) = &self.id {
      menu = menu.id(format!("{id}-links"));
    }
    menu
  }

  /// The links over the page: a sidebar of the items, an item with a panel an item that holds its links.
  fn drawer(&self, open: &Signal<bool>) -> Sidebar {
    let mut sidebar = Sidebar::new().label(&self.label).narrow(true).open(open.clone());
    for item in &self.items {
      // An item with a panel holds its links, open: they stand in from it, and the items after it beside it, as
      // they do in the bar. A group of them would end under its last link, with the items after it read as its own.
      let mut link = match &item.panel {
        Some(panel) => SidebarItem::new(&item.label)
          .disabled(item.disabled)
          .default_open(true)
          .children(panel.links.iter().map(sidebar_link)),
        None => SidebarItem::new(&item.label)
          .selected(item.current)
          .disabled(item.disabled),
      };
      {
        if let Some(icon) = item.icon {
          link = link.icon(icon);
        }
        if let Some(id) = &item.id {
          link = link.id(id.clone());
        }
        if let Some(key) = &item.key {
          link = link.key(key.clone());
        }
        // An item that opens a panel goes nowhere itself.
        if item.panel.is_none() {
          // A sidebar's item goes to a path only where the app has a router.
          #[cfg(feature = "router")]
          if let Some(href) = &item.href {
            link = link.href(href);
          }
          let handlers = item.on_select.clone();
          if !handlers.is_empty() {
            link = link.on_select(move || handlers.iter().for_each(|handler| handler()));
          }
        }
        sidebar = sidebar.item(link);
      }
    }
    if let Some(id) = &self.id {
      sidebar = sidebar.id(format!("{id}-drawer"));
    }
    sidebar
  }

  /// The button that opens the links over the page and closes them: a ghost button of the `menu` icon.
  fn menu_button(&self, open: &Signal<bool>, theme: &Theme) -> Element {
    let (shown, turned) = (open.get(), open.clone());
    let mut button = Button::new(&self.menu_label)
      .variant(ButtonVariant::Ghost)
      .icon(if shown { icons::CLOSE } else { icons::MENU })
      .icon_only(true)
      .on_click({
        let heard = self.on_menu_open_change.clone();
        move |_| {
          turned.update(|open| *open = !*open);
          let now = turned.get_untracked();
          for handler in &heard {
            handler(now);
          }
        }
      });
    if let Some(id) = &self.id {
      button = button.id(format!("{id}-menu"));
    }
    let controls = self.id.as_ref().map(|id| Arc::<str>::from(format!("{id}-drawer")));
    button.build(theme).map_node(move |mut node| {
      NodeUpdate::describe(
        &mut node,
        Arc::from("a11y_expanded"),
        Arc::from(if shown { "true" } else { "false" }),
      );
      if let Some(controls) = controls {
        NodeUpdate::describe(&mut node, Arc::from("a11y_controls"), controls);
      }
      node
    })
  }

  /// The case of the bar, as the window and the app make it.
  fn case(&self, narrow: bool) -> Case {
    Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(NavbarState::Narrow, narrow)
    .state(NavbarState::Sticky, self.sticky)
  }
}

/// A link of a panel as an item of the sidebar over the page.
fn sidebar_link(link: &NavLink) -> SidebarItem {
  let mut item = SidebarItem::new(&link.title)
    .selected(link.current)
    .disabled(link.disabled);
  if let Some(icon) = link.icon {
    item = item.icon(icon);
  }
  #[cfg(feature = "router")]
  if let Some(href) = &link.href {
    item = item.href(href);
  }
  if let Some(id) = &link.id {
    item = item.id(id.clone());
  }
  if let Some(key) = &link.key {
    item = item.key(key.clone());
  }
  let handlers = link.on_select.clone();
  if !handlers.is_empty() {
    item = item.on_select(move || handlers.iter().for_each(|handler| handler()));
  }
  item
}

/// A navbar converted into an element on its own has its links as a navigation menu does on its own, opened by a
/// press and the keys, and over the page a sidebar on its own: for a gallery, a test, or a window of its own. The
/// signals [`Navbar::open`] and [`Navbar::menu_open`] give are what opens a panel and the links over the page then.
/// [`Navbar::mount`] is what an app puts in its window.
impl From<Navbar> for Element {
  fn from(navbar: Navbar) -> Self {
    let theme = current_theme();
    let narrow = navbar.narrow.unwrap_or_else(window_is_narrow);
    let open = navbar.menu_open.clone().unwrap_or_else(|| Signal::new(false));
    let (links, drawer, menu_button) = if narrow {
      (
        None,
        Some(Element::from(navbar.drawer(&open))),
        Some(navbar.menu_button(&open, &theme)),
      )
    } else {
      (Some(Element::from(navbar.menu())), None, None)
    };
    build(navbar, &theme, narrow, links, drawer, menu_button)
  }
}

/// A navbar.
#[derive(Clone)]
struct NavbarProps {
  navbar: Navbar,
}

/// The bar is built anew by every render of what holds it, so props are never the same as before.
impl PartialEq for NavbarProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl crate::app::component::DevtoolsInspectable for NavbarProps {}

/// What a navbar keeps between renders.
struct NavbarHost {
  /// Whether the links are open over the page, where the app gives no signal of its own.
  menu_open: Signal<bool>,
}

impl Component for NavbarHost {
  type Props = NavbarProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      menu_open: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let navbar = ctx.props::<NavbarProps>().navbar.clone();
    let theme = current_theme();
    let narrow = navbar.narrow.unwrap_or_else(window_is_narrow);
    let open = navbar.menu_open.clone().unwrap_or_else(|| self.menu_open.clone());
    let (links, drawer, menu_button) = if narrow {
      (
        None,
        Some(navbar.drawer(&open).mount(ctx)),
        Some(navbar.menu_button(&open, &theme)),
      )
    } else {
      (Some(navbar.menu().mount(ctx)), None, None)
    };
    build(navbar, &theme, narrow, links, drawer, menu_button)
  }
}

/// Builds the bar around what was built for it: its links, or the links over the page and the button that opens
/// them.
fn build(
  navbar: Navbar,
  theme: &Theme,
  narrow: bool,
  links: Option<Element>,
  drawer: Option<Element>,
  menu_button: Option<Element>,
) -> Element {
  let case = navbar.case(narrow);
  let view = NavbarView {
    label: navbar.label.clone(),
    brand: navbar.brand,
    links,
    trailing: navbar.trailing,
    menu_button,
    drawer,
    narrow,
    sticky: navbar.sticky,
    recipe: theme
      .resolve::<Navbar>(&case, Some(&navbar.rules))
      .owned_by(navbar.id.as_ref()),
    id: navbar.id,
    key: navbar.key,
  };
  match theme.renderer::<Navbar>() {
    Some(renderer) => renderer(view),
    None => view.render(),
  }
}

/// A navbar as a renderer sees it: what it holds, built, and the style of its parts.
pub struct NavbarView {
  /// What it is called to tooling.
  pub label: Arc<str>,
  /// What stands at its start.
  pub brand: Option<Element>,
  /// Its links, a navigation menu, where the window is wide.
  pub links: Option<Element>,
  /// What stands after the links, in order.
  pub trailing: Vec<Element>,
  /// The button that opens the links over the page, where the window is narrow.
  pub menu_button: Option<Element>,
  /// The links over the page, where the window is narrow: a sidebar, which takes no room in the bar.
  pub drawer: Option<Element>,
  /// Whether the window is narrow.
  pub narrow: bool,
  /// Whether it stays at the top of the page that scrolls it.
  pub sticky: bool,
  /// The style of each part of the bar, in its states.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl NavbarView {
  /// The style of a part of the bar.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The bar with nothing in it yet: a navigation region called by the label to tooling, at the top of the page
  /// that scrolls it where it is sticky.
  pub fn root(&self) -> Row {
    let mut root = Row::new()
      .align_items(Alignment::Center)
      .overflow_visible()
      .part(&self.part(NavbarPart::Root))
      .describe("a11y_role", "navigation")
      .describe("a11y_name", self.label.clone());
    if self.sticky {
      root = root.sticky(Sticky::top(0.0));
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// How the default renderer builds a bar: the menu button where the window is narrow, the brand, the links in
  /// the room the bar has to spare, and what stands at its end.
  pub fn render(mut self) -> Element {
    let mut root = self.root();
    if let Some(button) = self.menu_button.take() {
      root = root.child(button);
    }
    if let Some(brand) = self.brand.take() {
      root = root.child(brand);
    }
    // The links take the room the bar has to spare, and no more: where they do not fit they are cut at its end,
    // and never run under what follows them.
    root = root.child(
      Row::new()
        .flex(1.0)
        .min_width(0.0)
        .align_items(Alignment::Center)
        .clip()
        // The links are `x-links`: the room they are in is not named apart.
        .part(&self.part(NavbarPart::Links).unnamed())
        .with_children(self.links.take()),
    );
    let trailing = std::mem::take(&mut self.trailing);
    if !trailing.is_empty() {
      root = root.child(
        Row::new()
          .flex_shrink(0.0)
          .align_items(Alignment::Center)
          .overflow_visible()
          .part(&self.part(NavbarPart::Trailing))
          .with_children(trailing),
      );
    }
    if let Some(drawer) = self.drawer.take() {
      root = root.child(drawer);
    }
    root.into()
  }
}

impl crate::ui::Styled for Navbar {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for Navbar {
  const NAME: &'static str = "navbar";
  type Renderer = NavbarRenderer;

  fn parts() -> crate::ui::Declared {
    NavbarPart::declared()
  }

  fn states() -> crate::ui::Declared {
    NavbarState::declared()
  }

  /// The design's navbar (`pencil/lurq-design-system.pen`, Navbar): a bar 56 tall of the background's color with a
  /// line of 1 in the border's color along its bottom, padded 16 at the sides, 24 between what it holds. The brand
  /// is a mark of 20 in the foreground 8 before its name in 16/600; the links are 20 apart, in 13/500 (their look
  /// is the links variant of the navigation menu's recipe); what stands at the end is 24 apart. On a narrow window,
  /// where the menu button stands first, it is padded 8 at the sides with 8 between what it holds, as the board's
  /// app bar is near its edges.
  fn recipe() -> Recipe {
    use NavbarPart::*;
    use NavbarState::{Interactive, Narrow};
    use PaletteColor::{Background, Foreground};
    Recipe::new()
      .base(Root, |s| {
        s.height(56.0)
          .padding_x(16.0)
          .gap(24.0)
          .background(Background)
          .foreground(Foreground)
          .underline(Border::inside(BorderSize::Sm, PaletteColor::Border))
      })
      .state(Narrow, Root, |s| s.padding_x(8.0).gap(8.0))
      .base(Brand, |s| fading(s.gap(8.0).radius(RadiusSize::Sm)))
      .state(Interactive, Brand, |s| s.cursor(CursorIcon::Pointer))
      .focus_visible(Brand, |s| s.focus_ring())
      .base(BrandMark, |s| s.square(IconSize::Lg).foreground(Foreground))
      .base(BrandName, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(16.0)
          .weight(FontWeight::SemiBold)
          .foreground(Foreground)
      })
      .base(Trailing, |s| s.gap(24.0))
  }
}
