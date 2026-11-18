//! The chrome of a desktop window the app draws itself: its title bar, with the app's icon, its title, its own
//! controls (a menubar, tabs, a search) and the window's controls, as Windows, macOS, GNOME and KDE draw it.

use std::sync::Arc;

use super::{
  IconChoice, icons, names,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{fading, glyph, one_line},
  theme::{Themed, current_theme},
};
/// One of the window's controls: the engine's, which a control of the bar is marked with
/// (`.window_control(..)`) for the system to know it.
pub use crate::node::WindowControl;
use crate::{
  app::{
    ctx::Ctx,
    events::MouseEvent,
    theme::{BorderSize, PaletteColor, RadiusSize, Theme},
    window::WindowHandle,
  },
  components::{self, ChromeBorderPolicy, ChromeTitleBar, Column, ResizeHandlePolicy, Row, Stack, WindowChromeMode},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{BackgroundColor, Element, border::Border, color::Color, dimension::Dimension},
};

/// The look and the order of a window's title bar: the desktop the app runs on, or another one an app chooses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindowPlatform {
  /// Windows 11: the icon and the title at the start, three square controls at the end, 46 wide and as high as the
  /// bar.
  Windows,
  /// macOS: three lights at the start, close, minimize and zoom, and the title in the middle.
  Macos,
  /// Linux with GNOME: a header bar 44 high, the title in the middle and round filled controls at the end.
  Gnome,
  /// Linux with KDE Plasma: the icon at the start, the title in the middle and round controls at the end.
  Kde,
}

names!(WindowPlatform {
  Windows => "windows",
  Macos => "macos",
  Gnome => "gnome",
  Kde => "kde",
});

impl WindowPlatform {
  /// The desktop the app runs on: Windows, macOS, and on Linux KDE where the session says it is
  /// (`XDG_CURRENT_DESKTOP`), GNOME otherwise.
  pub fn current() -> Self {
    if cfg!(target_os = "windows") {
      Self::Windows
    } else if cfg!(target_os = "macos") {
      Self::Macos
    } else {
      let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
      if desktop.to_ascii_uppercase().contains("KDE") {
        Self::Kde
      } else {
        Self::Gnome
      }
    }
  }

  /// Every platform, in the order a gallery shows them.
  pub const ALL: [Self; 4] = [Self::Windows, Self::Macos, Self::Gnome, Self::Kde];

  /// Whether the window's controls stand at the start of the bar, and in the order close, minimize, zoom.
  pub fn controls_at_start(self) -> bool {
    self == Self::Macos
  }
}

/// The parts of a window's chrome a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindowChromePart {
  /// The window: its fill, its edge and its corners, drawn by the still form; a window's own are the system's.
  Root,
  /// The title bar.
  Bar,
  /// The line along the bottom of a bar with tabs, under its tabs: the selected tab stands over it, open to the
  /// content, where a bar without tabs has its underline.
  Line,
  /// The app's icon in the bar.
  Icon,
  /// The window's title.
  Title,
  /// The room on the other side of a centred title that keeps it in the middle of the bar.
  Balance,
  /// The window's controls, together.
  Controls,
  /// The control that minimizes the window.
  Minimize,
  /// The control that maximizes the window, or restores it.
  Maximize,
  /// The control that closes the window.
  Close,
  /// The glyph of the minimize control.
  MinimizeGlyph,
  /// The glyph of the maximize control.
  MaximizeGlyph,
  /// The glyph of the close control.
  CloseGlyph,
}

names!(WindowChromePart {
  Root => "root",
  Bar => "bar",
  Line => "line",
  Icon => "icon",
  Title => "title",
  Balance => "balance",
  Controls => "controls",
  Minimize => "minimize",
  Maximize => "maximize",
  Close => "close",
  MinimizeGlyph => "minimize_glyph",
  MaximizeGlyph => "maximize_glyph",
  CloseGlyph => "close_glyph",
});

/// The states of a window's chrome a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindowChromeState {
  /// The window is not the one in front.
  Inactive,
  /// The window fills the screen's work area: its maximize control restores it.
  Maximized,
  /// The bar holds tabs ([`WindowChrome::tabs`]): it is higher, has no title and no line under it.
  Tabs,
}

names!(WindowChromeState {
  Inactive => "inactive",
  Maximized => "maximized",
  Tabs => "tabs",
});

/// What builds a chrome's nodes: a theme's in place of the default one.
pub type WindowChromeRenderer = Arc<dyn Fn(WindowChromeView) -> Element + Send + Sync>;

type Handler = Arc<dyn Fn() + Send + Sync>;

/// The window's controls in a [`WindowChrome`]: minimize, maximize (restore while maximized) and close, by the
/// platform's look and order, each named to what reads the tree.
///
/// ```
/// use lurq::ui::WindowControls;
///
/// let controls = WindowControls::new().maximizable(false).close_label("Quit").on_close(|| println!("closing"));
/// ```
///
/// The close control asks the window to close (`WindowHandle::request_close`), which an app that must confirm
/// first hears with `ctx.window().on_close_requested(..)`; minimize and maximize act on the window.
#[derive(Clone)]
pub struct WindowControls {
  minimizable: bool,
  maximizable: bool,
  minimize_label: Arc<str>,
  maximize_label: Arc<str>,
  restore_label: Arc<str>,
  close_label: Arc<str>,
  on_close: Vec<Handler>,
}

impl Default for WindowControls {
  fn default() -> Self {
    Self::new()
  }
}

impl WindowControls {
  /// The three controls, named "Minimize", "Maximize" ("Restore" while maximized) and "Close".
  pub fn new() -> Self {
    Self {
      minimizable: true,
      maximizable: true,
      minimize_label: super::current_locale().navigation.minimize.clone(),
      maximize_label: super::current_locale().navigation.maximize.clone(),
      restore_label: super::current_locale().navigation.restore.clone(),
      close_label: super::current_locale().common.close.clone(),
      on_close: Vec::new(),
    }
  }

  /// Whether the window has a minimize control. On by default.
  pub fn minimizable(mut self, minimizable: bool) -> Self {
    self.minimizable = minimizable;
    self
  }

  /// Whether the window has a maximize control: off for a window of a size of its own. On by default.
  pub fn maximizable(mut self, maximizable: bool) -> Self {
    self.maximizable = maximizable;
    self
  }

  /// The minimize control's name to what reads the tree. "Minimize" by default.
  pub fn minimize_label(mut self, label: &str) -> Self {
    self.minimize_label = Arc::from(label);
    self
  }

  /// The maximize control's name while the window is not maximized. "Maximize" by default.
  pub fn maximize_label(mut self, label: &str) -> Self {
    self.maximize_label = Arc::from(label);
    self
  }

  /// The maximize control's name while the window is maximized, when it restores it. "Restore" by default.
  pub fn restore_label(mut self, label: &str) -> Self {
    self.restore_label = Arc::from(label);
    self
  }

  /// The close control's name to what reads the tree. "Close" by default.
  pub fn close_label(mut self, label: &str) -> Self {
    self.close_label = Arc::from(label);
    self
  }

  /// Called when the close control is pressed, before the window is asked to close. A second handler is called
  /// after the first.
  pub fn on_close(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_close.push(Arc::new(handler));
    self
  }
}

/// A window's chrome drawn by the app: the title bar of a window that has none of the system's, with the app's
/// icon, its title, room for the app's own controls (a menubar, tabs, a search) and the window's controls, in the
/// look of the platform the app runs on or of the one it names, around the window's content.
///
/// ```
/// use lurq::{
///   components::Text,
///   core::Signal,
///   ui::{MenuItem, Menubar, MenubarMenu, WindowChrome, WindowPlatform},
/// };
///
/// # fn build(open: Signal<Option<usize>>) {
/// let chrome = WindowChrome::new()
///   .title("Untitled — Notes")
///   .leading(Menubar::new(open).menu(MenubarMenu::new("File", [MenuItem::new("New")])))
///   .content(Text::new("the document"));
/// let bar = WindowChrome::new().platform(WindowPlatform::Macos).title("Notes");
/// # }
/// ```
///
/// Mounted (`mount(ctx)`) it is the window's frame, through the engine's [`components::WindowChrome`]: the window
/// loses the system's title bar, and the bar drags the window from wherever nothing in it takes the press, a
/// double press maximizes it and restores it, a right press shows the system's menu of the window (on Windows),
/// and the edges resize it. The bar is drawn as the window is: in front or not ([`WindowChromeState::Inactive`]),
/// maximized or not. Converted into an element on its own, it is the still form of itself: the window and its bar
/// as a picture, for a gallery or a test, in the states the app gives it with `state(..)`.
pub struct WindowChrome {
  platform: Option<WindowPlatform>,
  icon: WindowChromeIcon,
  title: Option<Arc<str>>,
  label: Option<Arc<str>>,
  leading: Option<Element>,
  trailing: Option<Element>,
  tabs: Option<Element>,
  controls: WindowControls,
  content: Option<Element>,
  overlays: Vec<Element>,
  mode: WindowChromeMode,
  resize_handles: ResizeHandlePolicy,
  border: ChromeBorderPolicy,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Default for WindowChrome {
  fn default() -> Self {
    Self::new()
  }
}

impl WindowChrome {
  /// The chrome of the platform the app runs on, with the window's icon and the window's controls.
  pub fn new() -> Self {
    Self {
      platform: None,
      icon: WindowChromeIcon::Window,
      title: None,
      label: None,
      leading: None,
      trailing: None,
      tabs: None,
      controls: WindowControls::new(),
      content: None,
      overlays: Vec::new(),
      mode: WindowChromeMode::PlatformDefault,
      resize_handles: ResizeHandlePolicy::PlatformDefault,
      border: ChromeBorderPolicy::PlatformDefault,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// Draws the bar as `platform` has it, in place of the platform the app runs on ([`WindowPlatform::current`]).
  pub fn platform(mut self, platform: WindowPlatform) -> Self {
    self.platform = Some(platform);
    self
  }

  /// The app's icon in the bar, where the platform shows one (Windows and KDE): a name of the theme's icons, an
  /// image of the app's ([`ImageData`](crate::images::ImageData), with the `raster` feature), or `None` for none.
  /// By default the window's own: the image the app set as the window's icon
  /// ([`WindowHandle::set_icon`](crate::app::window::WindowHandle::set_icon) or the shell's `with_icon`), or a box
  /// where it has none. See [`WindowChromeIcon`].
  pub fn icon(mut self, icon: impl Into<WindowChromeIcon>) -> Self {
    self.icon = icon.into();
    self
  }

  /// The window's title, shown in the bar (before the tabs in a bar with tabs) and given to the window, where the
  /// system shows it (the task bar, the window switcher). It ends with an ellipsis where the bar has no room for it.
  pub fn title(mut self, title: &str) -> Self {
    self.title = Some(Arc::from(title));
    self
  }

  /// The bar's name to what reads the tree. Its title by default.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// What the app puts at the start of the bar, after the icon (after the lights on macOS): a menubar, as an
  /// editor has its menus there; a button. A press there is the element's: it does not drag the window.
  pub fn leading(mut self, element: impl Into<Element>) -> Self {
    self.leading = Some(element.into());
    self
  }

  /// What the app puts at the end of the bar, before the window's controls (after the title on macOS): a search,
  /// an account. A press there is the element's: it does not drag the window.
  pub fn trailing(mut self, element: impl Into<Element>) -> Self {
    self.trailing = Some(element.into());
    self
  }

  /// Tabs in the bar ([`WindowTabs`](super::WindowTabs)): the bar is the board's bar with tabs, higher, with no line
  /// under it, the app's icon and title (where it has them) before the tabs, the tabs on its bottom edge so that the
  /// selected one runs into the content. The bar's room the tabs leave still drags the window.
  pub fn tabs(mut self, tabs: impl Into<Element>) -> Self {
    self.tabs = Some(tabs.into());
    self
  }

  /// The window's controls: which there are, their names, and what the close one tells the app.
  pub fn controls(mut self, controls: WindowControls) -> Self {
    self.controls = controls;
    self
  }

  /// What the window shows under the bar.
  pub fn content(mut self, content: impl Into<Element>) -> Self {
    self.content = Some(content.into());
    self
  }

  /// A layer laid over the whole window, under the title bar's inset, as the engine's chrome has them.
  pub fn overlay(mut self, overlay: impl Into<Element>) -> Self {
    self.overlays.push(overlay.into());
    self
  }

  /// Layers laid over the whole window, in order.
  pub fn overlays(mut self, overlays: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.overlays.extend(overlays.into_iter().map(Into::into));
    self
  }

  /// When the app draws the window's chrome: the engine's [`WindowChromeMode`]. On Windows and macOS by default.
  pub fn mode(mut self, mode: WindowChromeMode) -> Self {
    self.mode = mode;
    self
  }

  /// The zones along the window's edges that resize it: the engine's [`ResizeHandlePolicy`].
  pub fn resize_handles(mut self, handles: ResizeHandlePolicy) -> Self {
    self.resize_handles = handles;
    self
  }

  /// The line around the window: the engine's [`ChromeBorderPolicy`], in the theme's `Border` by default.
  pub fn border(mut self, border: ChromeBorderPolicy) -> Self {
    self.border = border;
    self
  }

  /// Puts the chrome in a state the theme's recipe names, or takes it out: [`WindowChromeState::Inactive`] and
  /// [`WindowChromeState::Maximized`] for the still form, which has no window to read them from.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this chrome over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this chrome.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the close control as it is under the pointer or held down, with nothing pointing at it: for a gallery or
  /// a test. On macOS, the lights show their glyphs as the pointer on them does.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the bar. Its icon is `{id}-icon`, its title `{id}-title`, its controls `{id}-minimize`, `{id}-maximize`
  /// and `{id}-close`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The view of this chrome in `theme`, in its own states and those `window` is in.
  fn view(self, theme: &Theme, window: Option<WindowHandle>) -> (WindowChromeView, Option<Element>) {
    let platform = self.platform.unwrap_or_else(WindowPlatform::current);
    let (inactive, maximized) = match &window {
      Some(window) => (!window.is_focused, window.is_maximized),
      None => (
        self.states.contains(&Name::from(WindowChromeState::Inactive)),
        self.states.contains(&Name::from(WindowChromeState::Maximized)),
      ),
    };
    let case = Case {
      variant: Some(platform.into()),
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(WindowChromeState::Inactive, inactive)
    .state(WindowChromeState::Maximized, maximized)
    .state(WindowChromeState::Tabs, self.tabs.is_some());
    let tabbed = self.tabs.is_some();
    // The window whose icon the bar shows by default: its own where it is mounted, the one it is drawn in where it
    // is a picture.
    let shown_in = window
      .as_ref()
      .map(WindowHandle::state)
      .or_else(super::theme::current_window);
    let icon = self.icon.shown(shown_in.as_ref());
    let view = WindowChromeView {
      platform,
      icon,
      title: self.title,
      label: self.label,
      leading: self.leading,
      trailing: self.trailing,
      tabs: self.tabs,
      controls: self.controls,
      inactive,
      maximized,
      tabbed,
      recipe: theme.resolve::<WindowChrome>(&case, Some(&self.rules)),
      forced: self.forced,
      window,
      id: self.id,
      key: self.key,
    };
    (view, self.content)
  }

  /// The bar of this chrome, in `theme`, as the theme's renderer draws it.
  fn bar_in(view: WindowChromeView) -> Element {
    let theme = view.theme().clone();
    match theme.renderer::<WindowChrome>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// The window: the engine's chrome around the content, its title bar this chrome's bar for the window as it is
  /// now, which the bar reads (in front or not, maximized or not) so that it is drawn again as it changes.
  pub fn mount(mut self, ctx: &mut Ctx) -> Element {
    let theme = ctx.theme().clone();
    let window = ctx.window();
    if let Some(title) = &self.title
      && window.title().as_deref() != Some(&**title)
    {
      window.set_title(title.to_string());
    }
    let content = self.content.take().unwrap_or_default();
    let overlays = std::mem::take(&mut self.overlays);
    let (mode, handles, border) = (self.mode, self.resize_handles, self.border.clone());
    // What the engine's bar draws is built once per render; the engine calls it with the window.
    let chrome = Arc::new(std::sync::Mutex::new(Some(self)));
    let height = {
      let held = chrome.lock().expect("the chrome is held");
      let platform = held
        .as_ref()
        .and_then(|chrome| chrome.platform)
        .unwrap_or_else(WindowPlatform::current);
      bar_height(platform, held.as_ref().is_some_and(|chrome| chrome.tabs.is_some()))
    };
    let title_bar = ChromeTitleBar::new()
      .height(height)
      .background(BackgroundColor::Color(Color::new(0, 0, 0, 0)))
      .border_bottom(None)
      .draw(move |window, _height| {
        let Some(chrome) = chrome.lock().expect("the chrome is held").take() else {
          return Element::new();
        };
        let (view, _) = chrome.view(&theme, Some(window.clone()));
        Self::bar_in(view)
      });
    components::WindowChrome::new()
      .mode(mode)
      .resize_handles(handles)
      .border(border)
      .title_bar(title_bar)
      .content(content)
      .overlays(overlays)
      .mount(ctx)
  }

  /// Builds the still form in `theme`: the window, its bar over its content. Converting a chrome into an element
  /// does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let (view, content) = self.view(theme, None);
    let root = view.part(WindowChromePart::Root);
    let bar = Self::bar_in(view);
    let mut window = Column::new().align_items(Alignment::Stretch).part(&root).child(bar);
    if let Some(content) = content {
      window = window.child(content);
    }
    window.into()
  }
}

impl From<WindowChrome> for Element {
  fn from(chrome: WindowChrome) -> Self {
    chrome.build(&current_theme())
  }
}

/// The bar's height on `platform`, with tabs or not: what the engine lays the content under.
fn bar_height(platform: WindowPlatform, tabs: bool) -> f32 {
  match (platform, tabs) {
    (WindowPlatform::Windows | WindowPlatform::Macos, false) => 36.0,
    (WindowPlatform::Windows | WindowPlatform::Macos, true) => 40.0,
    (WindowPlatform::Gnome, _) => 44.0,
    (WindowPlatform::Kde, false) => 32.0,
    (WindowPlatform::Kde, true) => 38.0,
  }
}

/// The icon of a window's title bar: what [`WindowChrome::icon`] takes.
///
/// ```
/// use lurq::ui::{WindowChromeIcon, icons};
///
/// let glyph: WindowChromeIcon = icons::FILE_CODE.into();
/// let none: WindowChromeIcon = None.into();
/// assert_eq!(WindowChromeIcon::default(), WindowChromeIcon::Window);
/// # let _ = (glyph, none);
/// ```
#[derive(Clone, Default)]
pub enum WindowChromeIcon {
  /// The window's own: the image the app set as the window's icon, followed as it changes; the board's box where
  /// the window has none. A bar drawn as a picture shows the icon of the window it is drawn in. The default.
  #[default]
  Window,
  /// One of the theme's icons, by its name.
  Glyph(Name),
  /// An image of the app's: its logo. Kept by the app and given again at each render, it is uploaded once.
  #[cfg(feature = "raster")]
  Image(crate::images::ImageData),
  /// No icon.
  None,
}

impl std::fmt::Debug for WindowChromeIcon {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Window => f.write_str("Window"),
      Self::Glyph(name) => f.debug_tuple("Glyph").field(name).finish(),
      #[cfg(feature = "raster")]
      Self::Image(image) => write!(f, "Image({}x{})", image.width(), image.height()),
      Self::None => f.write_str("None"),
    }
  }
}

impl PartialEq for WindowChromeIcon {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (Self::Window, Self::Window) | (Self::None, Self::None) => true,
      (Self::Glyph(a), Self::Glyph(b)) => a == b,
      #[cfg(feature = "raster")]
      (Self::Image(a), Self::Image(b)) => a.id() == b.id(),
      _ => false,
    }
  }
}

impl WindowChromeIcon {
  /// The icon drawn: the window's own found in `window`, or the board's box; `None` for none.
  fn shown(self, window: Option<&crate::app::window::Window>) -> Option<Self> {
    match self {
      Self::Window => {
        #[cfg(feature = "raster")]
        if let Some(image) = window.and_then(|window| {
          window.track_access();
          window.icon_image()
        }) {
          return Some(Self::Image(image));
        }
        #[cfg(not(feature = "raster"))]
        let _ = window;
        Some(Self::Glyph(Name::from(icons::BOX)))
      }
      Self::None => None,
      icon => Some(icon),
    }
  }
}

impl From<&str> for WindowChromeIcon {
  fn from(name: &str) -> Self {
    Self::Glyph(Name::from(name))
  }
}

impl From<Name> for WindowChromeIcon {
  fn from(name: Name) -> Self {
    Self::Glyph(name)
  }
}

impl From<Option<Name>> for WindowChromeIcon {
  fn from(name: Option<Name>) -> Self {
    name.map_or(Self::None, Self::Glyph)
  }
}

impl From<IconChoice> for WindowChromeIcon {
  fn from(choice: IconChoice) -> Self {
    choice.0.into()
  }
}

#[cfg(feature = "raster")]
impl From<crate::images::ImageData> for WindowChromeIcon {
  fn from(image: crate::images::ImageData) -> Self {
    Self::Image(image)
  }
}

/// A chrome as a renderer sees it: what its bar holds, the window's state, the style of its parts, and the
/// window's controls ready to be put in it.
pub struct WindowChromeView {
  /// The platform the bar is drawn as.
  pub platform: WindowPlatform,
  /// The app's icon, if it shows one: a glyph or an image, the window's own already found
  /// ([`WindowChromeIcon::Glyph`] or [`WindowChromeIcon::Image`]).
  pub icon: Option<WindowChromeIcon>,
  pub title: Option<Arc<str>>,
  /// Its name to what reads the tree, if the app gave it one; its title otherwise.
  pub label: Option<Arc<str>>,
  /// Whether the window is not the one in front.
  pub inactive: bool,
  /// Whether the window is maximized.
  pub maximized: bool,
  /// Whether the bar holds tabs.
  pub tabbed: bool,
  /// The style of each part of this chrome.
  pub recipe: ResolvedRecipe,
  leading: Option<Element>,
  trailing: Option<Element>,
  tabs: Option<Element>,
  controls: WindowControls,
  forced: Option<Interaction>,
  window: Option<WindowHandle>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl WindowChromeView {
  /// The style of a part, in the chrome's states.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// Whether the bar holds tabs.
  pub fn has_tabs(&self) -> bool {
    self.tabbed
  }

  /// The bar in its style, with nothing in it yet: a group called by its label or its title to what reads the
  /// tree.
  pub fn bar(&self) -> Row {
    let part = self.part(WindowChromePart::Bar);
    let mut bar = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(if self.has_tabs() {
        Alignment::End
      } else {
        Alignment::Center
      })
      .part(&part)
      .describe("a11y_role", "group");
    if let Some(name) = self.label.as_ref().or(self.title.as_ref()) {
      bar = bar.describe("a11y_name", name.clone());
    }
    if let Some(id) = &self.id {
      bar = bar.id(id.clone());
    }
    if let Some(key) = &self.key {
      bar = bar.key(key.clone());
    }
    bar
  }

  /// A bar with tabs over its line, which is drawn under the tabs so that the selected tab covers it where it
  /// stands, open to the content: a border of the bar's own would be drawn over them. The bar's fill is under the
  /// line, the bar itself clear. A bar without tabs is as it is, its line its underline.
  pub fn over_line(&self, bar: Row) -> Element {
    if !self.has_tabs() {
      return bar.into();
    }
    let mut line = Row::new()
      .width(Dimension::Pct(100.0))
      .part(&self.part(WindowChromePart::Line));
    if let Some(id) = &self.id {
      line = line.id(format!("{id}-line"));
    }
    let mut layers = Stack::new().width(Dimension::Pct(100.0)).child(
      Column::new()
        .absolute(0.0, 0.0, Dimension::Pct(100.0), Dimension::Pct(100.0))
        .justify(Justify::End)
        .child(line),
    );
    if let Some(fill) = self.part(WindowChromePart::Bar).rest().background.clone() {
      layers = layers.background(fill);
    }
    layers.child(bar.background(Color::new(0, 0, 0, 0))).into()
  }

  /// The app's icon, where it has one.
  #[allow(deprecated)]
  pub fn icon(&self) -> Option<Element> {
    let part = self.part(WindowChromePart::Icon);
    let drawn = match self.icon.clone()? {
      WindowChromeIcon::Glyph(name) => glyph(&part, name, false),
      #[cfg(feature = "raster")]
      WindowChromeIcon::Image(image) => Row::new()
        .part(&part)
        .child(
          components::Image::new(image)
            .contain()
            .width(Dimension::Pct(100.0))
            .height(Dimension::Pct(100.0)),
        )
        .into(),
      WindowChromeIcon::Window | WindowChromeIcon::None => return None,
    };
    let mut icon = Column::new()
      .justify(Justify::Center)
      .height(Dimension::Pct(100.0))
      .child(drawn);
    if let Some(id) = &self.id {
      icon = icon.id(format!("{id}-icon"));
    }
    Some(icon.into())
  }

  /// The title, on one line, ending with an ellipsis where it has no room; centred where the platform centres it.
  pub fn title(&self) -> Option<Element> {
    let title = self.title.as_ref()?;
    let part = self.part(WindowChromePart::Title);
    let centred = self.platform != WindowPlatform::Windows;
    let mut row = Row::new()
      .flex_full(1.0, 1.0, Some(0.0))
      .min_width(0.0)
      .align_items(Alignment::Center)
      .justify(if centred { Justify::Center } else { Justify::Start })
      .child(one_line(&part, title));
    if let Some(id) = &self.id {
      row = row.id(format!("{id}-title"));
    }
    Some(row.into())
  }

  /// What stands before the tabs in a bar with tabs: the app's icon (where `with_icon` and the platform shows one)
  /// and its title, which does not grow and ends with an ellipsis where the bar has no room, centred on the bar's
  /// height while the tabs stand on its bottom edge. `None` where there is neither.
  pub fn tab_lead(&mut self, with_icon: bool) -> Option<Element> {
    let icon = if with_icon { self.icon() } else { None };
    let title = self.title.clone().map(|title| {
      let part = self.part(WindowChromePart::Title);
      let mut row = Row::new()
        .flex_shrink(1.0)
        .min_width(0.0)
        .max_width(TAB_LEAD_TITLE_MAX)
        .align_items(Alignment::Center)
        .child(one_line(&part, &title));
      if let Some(id) = &self.id {
        row = row.id(format!("{id}-title"));
      }
      Element::from(row)
    });
    if icon.is_none() && title.is_none() {
      return None;
    }
    let gap = self.part(WindowChromePart::Bar).rest().gap.unwrap_or_default();
    Some(
      Row::new()
        .height(Dimension::Pct(100.0))
        .align_items(Alignment::Center)
        .spacing(gap)
        .flex_shrink(1.0)
        .min_width(0.0)
        .padding_right(gap)
        .with_children([icon, title].into_iter().flatten())
        .into(),
    )
  }

  /// The room that keeps a centred title in the middle: as wide as what stands on the bar's other side.
  pub fn balance(&self) -> Element {
    Row::new().part(&self.part(WindowChromePart::Balance)).into()
  }

  /// An element of the app's in the bar, whose presses are its own: they do not drag the window.
  fn own(element: Element) -> Element {
    Row::new()
      .align_items(Alignment::Center)
      .on_mouse_down(|event: MouseEvent| event.stop_propagation())
      .child(element)
      .into()
  }

  /// What the app put at the start of the bar.
  pub fn leading(&mut self) -> Option<Element> {
    self.leading.take().map(Self::own)
  }

  /// What the app put at the end of the bar.
  pub fn trailing(&mut self) -> Option<Element> {
    self.trailing.take().map(Self::own)
  }

  /// The tabs, on the bar's bottom edge.
  pub fn tabs(&mut self) -> Option<Element> {
    let tabs = self.tabs.take()?;
    Some(
      Column::new()
        .height(Dimension::Pct(100.0))
        .justify(Justify::End)
        .flex_full(0.0, 1.0, None)
        .min_width(0.0)
        .child(tabs)
        .into(),
    )
  }

  /// The bar's room that drags the window: what is left of its width.
  pub fn drag_room(&self) -> Element {
    Row::new().flex(1.0).into()
  }

  /// One of the window's controls in its style, with its glyph, its name, and what it does to the window.
  pub fn control(&self, kind: WindowControl) -> Element {
    let (part, glyph_part, name, label) = match kind {
      WindowControl::Minimize => (
        WindowChromePart::Minimize,
        WindowChromePart::MinimizeGlyph,
        self.minimize_icon(),
        self.controls.minimize_label.clone(),
      ),
      WindowControl::Maximize => (
        WindowChromePart::Maximize,
        WindowChromePart::MaximizeGlyph,
        self.maximize_icon(),
        if self.maximized {
          self.controls.restore_label.clone()
        } else {
          self.controls.maximize_label.clone()
        },
      ),
      WindowControl::Close => (
        WindowChromePart::Close,
        WindowChromePart::CloseGlyph,
        icons::CLOSE,
        self.controls.close_label.clone(),
      ),
    };
    let forced = self
      .forced
      .filter(|_| kind == WindowControl::Close || self.platform == WindowPlatform::Macos);
    let mut button = components::Button::empty()
      .justify(Justify::Center)
      .align_items(Alignment::Center)
      .part(&self.part(part).forced(forced))
      // On macOS the lights are one group: the pointer on any of them shows the three glyphs.
      .interaction_group(self.platform != WindowPlatform::Macos)
      .describe("a11y_role", "button")
      .describe("a11y_name", label)
      // A press of a control is the control's: it does not drag the window.
      .on_mouse_down(|event: MouseEvent| event.stop_propagation())
      .child(glyph(&self.part(glyph_part).forced(forced), Name::new(name), false));
    if let Some(id) = &self.id {
      let which = match kind {
        WindowControl::Minimize => "minimize",
        WindowControl::Maximize => "maximize",
        WindowControl::Close => "close",
      };
      button = button.id(format!("{id}-{which}"));
    }
    if let Some(window) = self.window.clone() {
      let (maximized, on_close) = (self.maximized, self.controls.on_close.clone());
      // A control of a window says which it is, for the shell to tell the system: Windows 11 opens its snap layouts
      // over the maximize one. A control of a picture of a window is none.
      button = button.window_control(kind).on_click(move |event: MouseEvent| {
        match kind {
          WindowControl::Minimize => window.set_minimized(true),
          // In full screen the maximize control is the way back.
          WindowControl::Maximize if window.is_full_screen => window.set_full_screen(false),
          WindowControl::Maximize => window.set_maximized(!maximized),
          WindowControl::Close => {
            for handler in &on_close {
              handler();
            }
            window.request_close();
          }
        }
        event.stop_propagation();
      });
    } else if kind == WindowControl::Close {
      let on_close = self.controls.on_close.clone();
      button = button.on_click(move |_| {
        for handler in &on_close {
          handler();
        }
      });
    }
    // Where the system draws the window's controls itself, over these (macOS: its own lights, with its menu of
    // tilings and full screen), each keeps its room and its name and is not seen: the system's is what is pressed.
    if self.window.as_ref().is_some_and(|window| window.native_controls) {
      button = button.opacity(0.0);
    }
    button.into()
  }

  fn minimize_icon(&self) -> &'static str {
    match self.platform {
      WindowPlatform::Kde => icons::CHEVRON_DOWN,
      _ => icons::MINUS,
    }
  }

  fn maximize_icon(&self) -> &'static str {
    match (self.platform, self.maximized) {
      (WindowPlatform::Kde, _) => icons::CHEVRON_UP,
      (WindowPlatform::Macos, _) => icons::PLUS,
      (_, true) => icons::COPY,
      (_, false) => icons::SQUARE,
    }
  }

  /// The window's controls in the platform's order: close, minimize and zoom on macOS, minimize, maximize and close
  /// elsewhere. A control the app left out is not there.
  pub fn controls(&self) -> Element {
    // In full screen macOS has its lights in its own bar, which comes down over the app's at the top of the screen:
    // the app's bar draws none under it.
    if self.platform == WindowPlatform::Macos && self.window.as_ref().is_some_and(|window| window.is_full_screen) {
      return Row::new().into();
    }
    let order: &[WindowControl] = if self.platform.controls_at_start() {
      &[WindowControl::Close, WindowControl::Minimize, WindowControl::Maximize]
    } else {
      &[WindowControl::Minimize, WindowControl::Maximize, WindowControl::Close]
    };
    let mut row = Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(WindowChromePart::Controls))
      // The lights show their glyphs together under the pointer.
      .interaction_group(self.platform == WindowPlatform::Macos);
    for kind in order {
      let shown = match kind {
        WindowControl::Minimize => self.controls.minimizable,
        WindowControl::Maximize => self.controls.maximizable,
        WindowControl::Close => true,
      };
      if shown {
        row = row.child(self.control(*kind));
      }
    }
    let align = match (self.platform, self.has_tabs()) {
      // Windows keeps its controls at the top of a bar with tabs, as high as a bar without.
      (WindowPlatform::Windows, true) => Justify::Start,
      _ => Justify::Center,
    };
    Column::new()
      .height(Dimension::Pct(100.0))
      .justify(align)
      .child(row)
      .into()
  }

  /// The bar as Windows draws it: the icon and the title at the start, the app's leading room between them, the
  /// controls at the end. With tabs, the tabs in place of the icon and the title.
  pub fn windows(mut self) -> Element {
    let mut bar = self.bar();
    if self.has_tabs() {
      bar = bar.with_children(
        [
          self.tab_lead(true),
          self.leading(),
          self.tabs(),
          Some(self.drag_room()),
          self.trailing(),
        ]
        .into_iter()
        .flatten(),
      );
    } else {
      let gap = self.part(WindowChromePart::Bar).rest().gap.unwrap_or_default();
      let start = Row::new()
        .align_items(Alignment::Center)
        .spacing(gap)
        .flex_full(1.0, 1.0, Some(0.0))
        .min_width(0.0)
        .with_children([self.icon(), self.leading(), self.title()].into_iter().flatten());
      bar = bar.child(start).with_children(self.trailing());
    }
    self.over_line(bar.child(self.controls()))
  }

  /// The bar as macOS draws it: the lights at the start, the title in the middle of the bar. With tabs, the tabs
  /// after the lights.
  pub fn macos(mut self) -> Element {
    let mut bar = self.bar().child(self.controls());
    if self.has_tabs() {
      bar = bar.with_children(
        [
          self.tab_lead(false),
          self.leading(),
          self.tabs(),
          Some(self.drag_room()),
          self.trailing(),
        ]
        .into_iter()
        .flatten(),
      );
    } else {
      bar = bar
        .with_children(self.leading())
        .with_children(self.title().or_else(|| Some(self.drag_room())))
        .with_children(self.trailing())
        .child(self.balance());
    }
    self.over_line(bar)
  }

  /// The bar as GNOME's header bar draws it: the title in the middle, round controls at the end. With tabs, the
  /// tabs at the start.
  pub fn gnome(mut self) -> Element {
    let mut bar = self.bar();
    if self.has_tabs() {
      bar = bar.with_children(
        [
          self.tab_lead(false),
          self.leading(),
          self.tabs(),
          Some(self.drag_room()),
          self.trailing(),
        ]
        .into_iter()
        .flatten(),
      );
    } else {
      bar = bar
        .child(self.balance())
        .with_children(self.leading())
        .with_children(self.title().or_else(|| Some(self.drag_room())))
        .with_children(self.trailing());
    }
    self.over_line(bar.child(self.controls()))
  }

  /// The bar as KDE draws it: the icon at the start, the title in the middle, round controls at the end. With
  /// tabs, the tabs after the icon.
  pub fn kde(mut self) -> Element {
    let mut bar = self.bar();
    if self.has_tabs() {
      bar = bar.with_children(
        [
          self.tab_lead(true),
          self.leading(),
          self.tabs(),
          Some(self.drag_room()),
          self.trailing(),
        ]
        .into_iter()
        .flatten(),
      );
    } else {
      let icon = self.icon();
      let start = Row::new()
        .align_items(Alignment::Center)
        .part(&self.part(WindowChromePart::Balance))
        .with_children(icon);
      bar = bar
        .child(start)
        .with_children(self.leading())
        .with_children(self.title().or_else(|| Some(self.drag_room())))
        .with_children(self.trailing());
    }
    self.over_line(bar.child(self.controls()))
  }

  /// How the default renderer builds the bar: as the platform it is drawn as.
  pub fn render(self) -> Element {
    match self.platform {
      WindowPlatform::Windows => self.windows(),
      WindowPlatform::Macos => self.macos(),
      WindowPlatform::Gnome => self.gnome(),
      WindowPlatform::Kde => self.kde(),
    }
  }
}

impl crate::ui::Styled for WindowChrome {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

/// The most a title before tabs takes of the bar, so that the tabs keep their room.
const TAB_LEAD_TITLE_MAX: f32 = 220.0;

impl Themed for WindowChrome {
  const NAME: &'static str = "window_chrome";
  type Renderer = WindowChromeRenderer;

  fn parts() -> crate::ui::Declared {
    WindowChromePart::declared()
  }

  fn states() -> crate::ui::Declared {
    WindowChromeState::declared()
  }

  /// The design's window chrome (`pencil/lurq-design-system.pen`, Window): a bar on `Bar` with a line of `Border`
  /// under it (with tabs, the line is under the tabs, and the selected tab, edged on its other sides, stands over it
  /// open to the content), its title 13 in `Foreground`, by platform:
  ///
  /// - Windows: 36 high, 12 in at the start; the box icon 16 and the title 13/500, 8 apart, at the start; three
  ///   controls 46 by 36 at the end, minus 16, a square 12 (two squares 13 while maximized) and a cross 16 in
  ///   `Foreground`; minimize and maximize on `SecondaryHover` under the pointer, close on `Destructive` with its cross
  ///   in `DestructiveForeground`.
  /// - macOS: 36 high, 12 in; the lights 12 across, 8 apart, at the start, red, yellow and green with a rim of
  ///   `#0000001F`, grey (`Input`) in a window that is not in front, showing their glyphs of 8 in `#00000099` under the
  ///   pointer; the title 13/600 in the middle.
  /// - GNOME: 44 high, 10 in; the title 13/600 in the middle; round controls of 24, 8 apart, on `SecondaryHover`, minus
  ///   14, a square 10 and a cross 14; close on `Destructive`.
  /// - KDE: 32 high, 8 in at the start and 6 at the end; the box icon 16 at the start, the title 13/400 in the middle;
  ///   round controls of 22, 6 apart, a chevron down 16, a chevron up 16 and a cross 14; close on `Destructive`.
  ///
  /// A bar with tabs is 40 high (44 on GNOME, 38 on KDE), has the icon and the title before its tabs (where the app
  /// gave them, Ivan 2026-11-17: the board draws none), its line under its tabs, and holds its tabs on its bottom edge.
  /// The window is `Background` in an edge of `Border`, its corners 8 on Windows, 12 on macOS and GNOME, 6 on KDE.
  fn recipe() -> Recipe {
    use PaletteColor::{
      Background, Destructive, DestructiveForeground, DestructiveHover, Foreground, Input, MutedForeground,
      SecondaryHover, StateHover, StatePressed,
    };
    use WindowChromePart::{
      Balance, Bar, Close, CloseGlyph, Controls, Icon, Line, Maximize, MaximizeGlyph, Minimize, MinimizeGlyph, Root,
      Title,
    };
    use WindowPlatform::{Gnome, Kde, Macos, Windows};
    let on = |platform: WindowPlatform| Selector::new().variant(platform);
    let control = [Minimize, Maximize, Close];
    let glyphs = [MinimizeGlyph, MaximizeGlyph, CloseGlyph];
    let mut recipe = Recipe::new()
      .base(Root, |s| {
        s.background(Background)
          .border(Border::inside(BorderSize::Sm, PaletteColor::Border))
          .radius(RadiusSize::Lg)
      })
      .base(Bar, |s| {
        s.background(PaletteColor::Bar)
          .underline(Border::inside(BorderSize::Sm, PaletteColor::Border))
          .gap(8.0)
      })
      .base(Icon, |s| s.square(16.0).foreground(Foreground))
      .base(Title, |s| {
        s.font_size(13.0).weight(FontWeight::Medium).foreground(Foreground)
      })
      .base(MinimizeGlyph, |s| s.foreground(Foreground))
      .base(MaximizeGlyph, |s| s.foreground(Foreground))
      .base(CloseGlyph, |s| s.foreground(Foreground))
      // Where the board is silent: a window that is not in front has its title and its glyphs muted.
      .state(WindowChromeState::Inactive, Title, |s| s.foreground(MutedForeground))
      .state(WindowChromeState::Inactive, Icon, |s| s.foreground(MutedForeground))
      .state(WindowChromeState::Inactive, MinimizeGlyph, |s| {
        s.foreground(MutedForeground)
      })
      .state(WindowChromeState::Inactive, MaximizeGlyph, |s| {
        s.foreground(MutedForeground)
      })
      .state(WindowChromeState::Inactive, CloseGlyph, |s| {
        s.foreground(MutedForeground)
      })
      // A bar with tabs has its line under its tabs, not over them: the selected tab runs into the content.
      .state(WindowChromeState::Tabs, Bar, |s| {
        s.underline(Border::inside(0.0, PaletteColor::Border))
      })
      .base(Line, |s| s.height(1.0).background(PaletteColor::Border))
      // Windows.
      .when(on(Windows), Bar, |s| s.height(36.0).padding_start(12.0))
      .when(on(Windows).state(WindowChromeState::Tabs), Bar, |s| {
        s.height(40.0).padding_start(8.0)
      })
      .when(on(Windows), Root, |s| s.radius(RadiusSize::Lg))
      .when(on(Windows), MinimizeGlyph, |s| s.square(16.0))
      .when(on(Windows), MaximizeGlyph, |s| s.square(12.0))
      .when(on(Windows).state(WindowChromeState::Maximized), MaximizeGlyph, |s| {
        s.square(13.0)
      })
      .when(on(Windows), CloseGlyph, |s| s.square(16.0))
      // macOS.
      .when(on(Macos), Bar, |s| s.height(36.0).padding_x(12.0))
      .when(on(Macos).state(WindowChromeState::Tabs), Bar, |s| {
        s.height(40.0).padding_start(12.0).padding_end(8.0).gap(14.0)
      })
      .when(on(Macos), Root, |s| s.radius(RadiusSize::Xl))
      .when(on(Macos), Title, |s| s.weight(FontWeight::SemiBold))
      .when(on(Macos), Balance, |s| s.width(52.0))
      .when(on(Macos), Controls, |s| s.gap(8.0))
      // GNOME.
      .when(on(Gnome), Bar, |s| s.height(44.0).padding_x(10.0))
      .when(on(Gnome).state(WindowChromeState::Tabs), Bar, |s| {
        s.padding_start(8.0).padding_end(10.0)
      })
      .when(on(Gnome), Root, |s| s.radius(RadiusSize::Xl))
      .when(on(Gnome), Title, |s| s.weight(FontWeight::SemiBold))
      .when(on(Gnome), Balance, |s| s.width(88.0))
      .when(on(Gnome), Controls, |s| s.gap(8.0))
      .when(on(Gnome), MinimizeGlyph, |s| s.square(14.0))
      .when(on(Gnome), MaximizeGlyph, |s| s.square(10.0))
      .when(on(Gnome), CloseGlyph, |s| s.square(14.0))
      // KDE.
      .when(on(Kde), Bar, |s| s.height(32.0).padding_start(8.0).padding_end(6.0))
      .when(on(Kde).state(WindowChromeState::Tabs), Bar, |s| s.height(38.0))
      .when(on(Kde), Root, |s| s.radius(RadiusSize::Md))
      .when(on(Kde), Title, |s| s.weight(FontWeight::Normal))
      .when(on(Kde), Balance, |s| s.width(78.0))
      .when(on(Kde), Controls, |s| s.gap(6.0))
      .when(on(Kde), MinimizeGlyph, |s| s.square(16.0))
      .when(on(Kde), MaximizeGlyph, |s| s.square(16.0))
      .when(on(Kde), CloseGlyph, |s| s.square(14.0));
    for part in control {
      recipe = recipe
        // Windows: square, as high as the bar without tabs.
        .when(on(Windows), part, |s| fading(s.width(46.0).height(36.0)))
        .when(on(Windows).hovered(), part, |s| s.background(SecondaryHover))
        .when(on(Windows).pressed(), part, |s| s.background(SecondaryHover))
        // macOS: the lights.
        .when(on(Macos), part, |s| {
          s.square(12.0)
            .radius(RadiusSize::Full)
            .border(Border::inside(0.5, Color::from_hex("#0000001F")))
        })
        .when(on(Macos).state(WindowChromeState::Inactive), part, |s| {
          s.background(Input)
        })
        // GNOME: round and filled.
        .when(on(Gnome), part, |s| {
          fading(s.square(24.0).radius(RadiusSize::Full).background(SecondaryHover))
        })
        // Stronger under the pointer and held, in the light appearance and the dark: a veil over the fill.
        .when(on(Gnome).hovered(), part, |s| s.veil(StateHover))
        .when(on(Gnome).pressed(), part, |s| s.veil(StatePressed))
        // KDE: round, filled only under the pointer.
        .when(on(Kde), part, |s| fading(s.square(22.0).radius(RadiusSize::Full)))
        .when(on(Kde).hovered(), part, |s| s.background(SecondaryHover))
        .when(on(Kde).pressed(), part, |s| {
          s.background(SecondaryHover).veil(StatePressed)
        });
    }
    for part in glyphs {
      // macOS: the lights' glyphs, shown under the pointer on the group.
      recipe = recipe
        .when(on(Macos), part, |s| {
          s.square(8.0).foreground(Color::from_hex("#00000099")).opacity(0.0)
        })
        .when(on(Macos).within().hovered(), part, |s| {
          s.opacity(1.0).foreground(Color::from_hex("#00000099"))
        })
        .when(on(Macos).within().pressed(), part, |s| {
          s.opacity(1.0).foreground(Color::from_hex("#00000099"))
        });
    }
    recipe
      // The close control's own colors: red under the pointer, darker held down, its cross white.
      .when(on(Windows).hovered(), Close, |s| s.background(Destructive))
      .when(on(Windows).pressed(), Close, |s| s.background(DestructiveHover))
      .when(on(Gnome).hovered(), Close, |s| {
        s.background(Destructive).veil(Color::new(0, 0, 0, 0))
      })
      .when(on(Gnome).pressed(), Close, |s| {
        s.background(DestructiveHover).veil(Color::new(0, 0, 0, 0))
      })
      .when(on(Kde).hovered(), Close, |s| s.background(Destructive))
      .when(on(Kde).pressed(), Close, |s| {
        s.background(DestructiveHover).veil(Color::new(0, 0, 0, 0))
      })
      .when(Selector::new().within().hovered(), CloseGlyph, |s| {
        s.foreground(DestructiveForeground)
      })
      .when(Selector::new().within().pressed(), CloseGlyph, |s| {
        s.foreground(DestructiveForeground)
      })
      // macOS: the lights' colors are the system's; their glyphs show under the pointer on the group.
      .when(on(Macos), Minimize, |s| s.background(Color::from_hex("#FEBC2E")))
      .when(on(Macos), Maximize, |s| s.background(Color::from_hex("#28C840")))
      .when(on(Macos), Close, |s| s.background(Color::from_hex("#FF5F57")))
      .when(on(Macos).state(WindowChromeState::Inactive), Minimize, |s| {
        s.background(Input)
      })
      .when(on(Macos).state(WindowChromeState::Inactive), Maximize, |s| {
        s.background(Input)
      })
      .when(on(Macos).state(WindowChromeState::Inactive), Close, |s| {
        s.background(Input)
      })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    app::{App, Tree, component::Component, events::MouseButton, window::WindowCommand},
    components::Rect,
  };

  /// A window of 800 by 600 whose chrome is the app's, on Windows, with a leading element of the app's.
  struct Host;

  impl Component for Host {
    type Props = ();

    fn create(_ctx: &mut Ctx) -> Self {
      Self
    }

    fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
      WindowChrome::new()
        .platform(WindowPlatform::Windows)
        .title("Notes")
        .mode(WindowChromeMode::AlwaysCustom)
        .id("bar")
        .leading(Rect::new(120.0, 24.0).id("menus"))
        .content(Rect::new(800.0, 564.0))
        .mount(ctx)
    }
  }

  /// The same window with tabs in its bar.
  struct Tabbed;

  impl Component for Tabbed {
    type Props = ();

    fn create(_ctx: &mut Ctx) -> Self {
      Self
    }

    fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
      let selected = ctx.signal(1_u32);
      let tabs = super::super::WindowTabs::new(
        selected,
        [
          super::super::WindowTab::new(1, "blur.rs").id("blur"),
          super::super::WindowTab::new(2, "theme.rs").id("theme"),
        ],
      )
      .mount(ctx);
      WindowChrome::new()
        .platform(WindowPlatform::Windows)
        .mode(WindowChromeMode::AlwaysCustom)
        .id("bar")
        .tabs(tabs)
        .content(Rect::new(800.0, 560.0))
        .mount(ctx)
    }
  }

  #[test]
  fn a_press_on_a_tab_is_the_tabs_and_the_room_they_leave_drags_the_window() {
    let mut app = App::new();
    let mut tree = Tree::new();
    tree.resize(800, 600);
    tree.mount_root::<Tabbed>(&mut app, ());
    tree.pass_headless(&mut app);
    tree.pass_headless(&mut app);
    let _ = tree.window().take_commands();
    let (x, y) = centre(&mut tree, "theme");
    // The tab stands on the bar's bottom edge, 40 high.
    let tab = tree
      .get_element_by_id_mut("theme")
      .and_then(|element| element.bounds())
      .unwrap();
    assert_eq!(tab.y + tab.height, 40.0);
    let commands = press(&mut tree, &mut app, x, y, MouseButton::Left);
    // The press selected the tab, which renders the window again (the headless window keeps the decorations the
    // engine's chrome asks it to drop, so it asks again); it dragged nothing.
    assert!(!commands.contains(&WindowCommand::StartDrag), "{commands:?}");
    let commands = press(&mut tree, &mut app, 600.0, 20.0, MouseButton::Left);
    assert!(commands.contains(&WindowCommand::StartDrag), "{commands:?}");
  }

  /// A window whose chrome shows the window's icon (0), a glyph (1) or none (2).
  struct Iconed;

  impl Component for Iconed {
    type Props = u8;

    fn create(_ctx: &mut Ctx) -> Self {
      Self
    }

    fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
      let chrome = WindowChrome::new()
        .platform(WindowPlatform::Windows)
        .mode(WindowChromeMode::AlwaysCustom)
        .id("bar")
        .title("Notes")
        .content(Rect::new(800.0, 564.0));
      let chrome = match *ctx.props::<u8>() {
        0 => chrome,
        1 => chrome.icon(icons::FILE_CODE),
        _ => chrome.icon(None),
      };
      chrome.mount(ctx)
    }
  }

  /// The tags of what the bar's icon is drawn with, or `None` where the bar has no icon.
  fn icon_tags(tree: &Tree) -> Option<Vec<String>> {
    use crate::node::element::ElementRef as Seen;
    fn find<'a>(element: Seen<'a>, id: &str) -> Option<Seen<'a>> {
      if element.id() == Some(id) {
        return Some(element);
      }
      element.children().into_iter().find_map(|child| find(child, id))
    }
    fn tags(element: Seen<'_>, into: &mut Vec<String>) {
      into.push(element.tag_name().to_owned());
      for child in element.children() {
        tags(child, into);
      }
    }
    let icon = find(tree.root()?, "bar-icon")?;
    let mut into = Vec::new();
    tags(icon, &mut into);
    Some(into)
  }

  fn red_icon() -> crate::app::window::WindowIcon {
    crate::app::window::WindowIcon::from_rgba([255, 0, 0, 255].repeat(16 * 16), 16, 16)
  }

  #[cfg(feature = "raster")]
  #[test]
  fn the_bar_shows_the_windows_icon_and_follows_it() {
    let mut app = App::new();
    let mut tree = Tree::new();
    tree.resize(800, 600);
    tree.mount_root::<Iconed>(&mut app, 0);
    tree.pass_headless(&mut app);
    // No icon of the window's: the board's box, a glyph.
    let tags = icon_tags(&tree).expect("an icon");
    assert!(!tags.iter().any(|tag| tag == "Image"), "{tags:?}");
    // The app sets one: the bar draws it, at the icon's size.
    tree.window().handle().set_icon(red_icon());
    tree.pass_headless(&mut app);
    let tags = icon_tags(&tree).expect("an icon");
    assert!(tags.iter().any(|tag| tag == "Image"), "{tags:?}");
    let icon = tree
      .get_element_by_id_mut("bar-icon")
      .and_then(|element| element.bounds())
      .unwrap();
    assert_eq!(icon.width, 16.0);
    // And clears it: the box again.
    tree.window().handle().set_icon(None);
    tree.pass_headless(&mut app);
    let tags = icon_tags(&tree).expect("an icon");
    assert!(!tags.iter().any(|tag| tag == "Image"), "{tags:?}");
  }

  #[test]
  fn a_glyph_or_none_is_shown_in_place_of_the_windows_icon() {
    for (shown, has_icon) in [(1, true), (2, false)] {
      let mut app = App::new();
      let mut tree = Tree::new();
      tree.resize(800, 600);
      tree.window().handle().set_icon(red_icon());
      tree.mount_root::<Iconed>(&mut app, shown);
      tree.pass_headless(&mut app);
      let tags = icon_tags(&tree);
      assert_eq!(tags.is_some(), has_icon, "{shown}: {tags:?}");
      assert!(!tags.unwrap_or_default().iter().any(|tag| tag == "Image"));
    }
  }

  #[cfg(feature = "raster")]
  #[test]
  fn the_icon_is_the_windows_image_a_glyph_or_none() {
    let theme = Theme::new();
    let tree = Tree::new();
    let handle = || Some(tree.window().handle());
    let shown = |chrome: WindowChrome, window| chrome.view(&theme, window).0.icon;
    // A window with no icon: the box.
    assert_eq!(
      shown(WindowChrome::new(), handle()),
      Some(WindowChromeIcon::Glyph(Name::from(icons::BOX)))
    );
    tree.window().handle().set_icon(red_icon());
    // The window's image, the same one at every render, so that it is uploaded once.
    let (Some(WindowChromeIcon::Image(first)), Some(WindowChromeIcon::Image(again))) = (
      shown(WindowChrome::new(), handle()),
      shown(WindowChrome::new(), handle()),
    ) else {
      panic!("the window's image");
    };
    assert_eq!((first.width(), first.height(), first.id()), (16, 16, again.id()));
    assert_eq!(
      shown(WindowChrome::new().icon(icons::FILE_CODE), handle()),
      Some(WindowChromeIcon::Glyph(Name::from(icons::FILE_CODE)))
    );
    assert_eq!(shown(WindowChrome::new().icon(None), handle()), None);
    // An image of the app's.
    let logo = crate::images::ImageData::from_rgba(vec![0; 4 * 4], 2, 2);
    assert_eq!(
      shown(WindowChrome::new().icon(logo.clone()), handle()),
      Some(WindowChromeIcon::Image(logo))
    );
    // A picture drawn outside any window: the box.
    assert_eq!(
      shown(WindowChrome::new(), None),
      Some(WindowChromeIcon::Glyph(Name::from(icons::BOX)))
    );
  }

  fn window() -> (Tree, App) {
    let mut app = App::new();
    let mut tree = Tree::new();
    tree.resize(800, 600);
    tree.mount_root::<Host>(&mut app, ());
    tree.pass_headless(&mut app);
    tree.pass_headless(&mut app);
    let _ = tree.window().take_commands();
    (tree, app)
  }

  fn press(tree: &mut Tree, app: &mut App, x: f32, y: f32, button: MouseButton) -> Vec<WindowCommand> {
    tree.mouse_move(x, y);
    tree.mouse_down(x, y, button);
    tree.mouse_up(x, y, button);
    tree.pass_headless(app);
    tree.window().take_commands()
  }

  fn centre(tree: &mut Tree, id: &str) -> (f32, f32) {
    let rect = tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.bounds())
      .expect(id);
    (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
  }

  #[test]
  fn the_controls_act_on_the_window_and_do_not_drag_it() {
    let (mut tree, mut app) = window();
    let (x, y) = centre(&mut tree, "bar-minimize");
    assert_eq!(
      press(&mut tree, &mut app, x, y, MouseButton::Left),
      [WindowCommand::SetMinimized(true)]
    );
    let (x, y) = centre(&mut tree, "bar-maximize");
    assert_eq!(
      press(&mut tree, &mut app, x, y, MouseButton::Left),
      [WindowCommand::SetMaximized(true)]
    );
    let (x, y) = centre(&mut tree, "bar-close");
    let commands = press(&mut tree, &mut app, x, y, MouseButton::Left);
    assert!(
      matches!(commands.as_slice(), [WindowCommand::RequestClose(_)]),
      "the window is asked to close: {commands:?}"
    );
  }

  #[test]
  fn the_bar_drags_the_window_and_a_right_press_shows_the_systems_menu() {
    let (mut tree, mut app) = window();
    // The empty room of the bar.
    assert_eq!(
      press(&mut tree, &mut app, 500.0, 18.0, MouseButton::Left),
      [WindowCommand::StartDrag]
    );
    assert_eq!(
      press(&mut tree, &mut app, 500.0, 18.0, MouseButton::Right),
      [WindowCommand::ShowSystemMenu]
    );
    // What the app put in the bar is its own: a press there drags nothing.
    let (x, y) = centre(&mut tree, "menus");
    let commands = press(&mut tree, &mut app, x, y, MouseButton::Left);
    assert!(commands.is_empty(), "{commands:?}");
  }

  #[test]
  fn the_window_knows_its_controls_where_nothing_covers_them() {
    let (mut tree, _app) = window();
    // Each control of the bar is the window's, for the shell to tell the system: Windows 11 opens its snap layouts
    // over the maximize one.
    for (id, control) in [
      ("bar-minimize", WindowControl::Minimize),
      ("bar-maximize", WindowControl::Maximize),
      ("bar-close", WindowControl::Close),
    ] {
      let (x, y) = centre(&mut tree, id);
      assert_eq!(tree.window_control_at(x, y), Some(control), "{id}");
    }
    // The rest of the bar is none: its title, the app's own element in it.
    for id in ["bar-title", "menus"] {
      let (x, y) = centre(&mut tree, id);
      assert_eq!(tree.window_control_at(x, y), None, "{id}");
    }
    // Along the window's top edge a zone resizes it: there the pointer is the zone's, not the control's under it.
    let (x, _) = centre(&mut tree, "bar-maximize");
    assert_eq!(tree.window_control_at(x, 1.0), None);
    assert_eq!(tree.window_control_at(x, 6.0), Some(WindowControl::Maximize));
  }

  #[test]
  fn a_maximized_window_has_its_controls_up_to_its_edges() {
    let (mut tree, mut app) = window();
    tree.window().set_maximized(true);
    tree.pass_headless(&mut app);
    tree.pass_headless(&mut app);
    let _ = tree.window().take_commands();
    // No zone resizes a maximized window, and the line of its outline takes no pointer: the close control is the
    // window's up to the corner of the screen, where a thrown pointer stops, and is pressed there.
    assert_eq!(tree.window_control_at(799.0, 0.0), Some(WindowControl::Close));
    let (x, _) = centre(&mut tree, "bar-maximize");
    assert_eq!(tree.window_control_at(x, 0.0), Some(WindowControl::Maximize));
    let commands = press(&mut tree, &mut app, 799.0, 0.0, MouseButton::Left);
    assert!(
      matches!(commands.as_slice(), [WindowCommand::RequestClose(_)]),
      "a press in the corner asks the window to close: {commands:?}"
    );
  }

  #[test]
  fn a_picture_of_a_window_has_no_controls_of_the_window() {
    // The still form: a bar drawn in a page, whose controls act on no window.
    let mut app = App::new();
    let mut tree = Tree::new();
    tree.resize(800, 600);
    tree.set_root(
      WindowChrome::new()
        .platform(WindowPlatform::Windows)
        .title("Notes")
        .id("picture"),
    );
    tree.pass_headless(&mut app);
    for id in ["picture-minimize", "picture-maximize", "picture-close"] {
      let (x, y) = centre(&mut tree, id);
      assert_eq!(tree.window_control_at(x, y), None, "{id}");
    }
  }

  #[test]
  fn the_bar_is_drawn_as_the_window_is() {
    let (mut tree, mut app) = window();
    tree.window().set_maximized(true);
    tree.window().set_focused(false);
    tree.pass_headless(&mut app);
    tree.pass_headless(&mut app);
    let _ = tree.window().take_commands();
    crate::node::with_tooling! {
      let name = tree
        .get_element_by_id_mut("bar-maximize")
        .and_then(|element| element.description("a11y_name"));
      assert_eq!(name.as_deref(), Some("Restore"));
    }
    // Maximized, the middle control restores it.
    let (x, y) = centre(&mut tree, "bar-maximize");
    assert_eq!(
      press(&mut tree, &mut app, x, y, MouseButton::Left),
      [WindowCommand::SetMaximized(false)]
    );
  }
}
