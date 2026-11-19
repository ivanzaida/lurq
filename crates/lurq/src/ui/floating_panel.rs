//! A floating panel: a tool window inside the app's window, moved by its header and resized by its edges; the header
//! of a panel on its own; and a layer of floating panels that brings the one pressed to the front.

use std::sync::{
  Arc, Mutex,
  atomic::{AtomicBool, Ordering},
};

use super::{
  icon::Icon,
  icons,
  menu::{Menu, MenuItem},
  names,
  part_style::{Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{fading, glyph, one_line, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{DragEvent, KeyboardEvent, MouseButtonMask},
    theme::{BorderSize, IconSize, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle},
  },
  components::{self, Column, Row, ScrollVertical, Stack},
  core::{ElementRef, Signal, SignalValue},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, HitTestBehavior, border::Border, dimension::Dimension},
};

/// What a press of a header's button runs.
type Handler = Arc<dyn Fn() + Send + Sync>;

/// A button of a header: its icon, what it is called, and what a press of it runs.
#[derive(Clone)]
struct HeaderButton {
  icon: Name,
  label: Arc<str>,
  on_click: Handler,
}

/// A header's menu: whether it is open, the app's, and what makes its items.
#[derive(Clone)]
struct HeaderMenu {
  open: Signal<bool>,
  items: Arc<dyn Fn() -> Vec<MenuItem> + Send + Sync>,
}

/// A button of a header in the style of `part`: 22 square, its icon of 14 in it.
pub(super) fn header_button(
  part: &ResolvedPart,
  icon: &Name,
  label: &Arc<str>,
  on_click: Handler,
) -> components::Button {
  pressable(part, false)
    .align_items(Alignment::Center)
    .describe("a11y_role", "button")
    .describe("a11y_name", label.clone())
    .on_click(move |_| on_click())
    .child(Icon::new(icon.clone()).size(IconSize::Sm).build(part.theme()))
}

/// A header's menu button, the "more" of the board, with its menu hung from it.
fn header_menu(part: &ResolvedPart, label: &Arc<str>, menu: HeaderMenu) -> Element {
  let anchor = ElementRef::new();
  let toggle = menu.open.clone();
  let button = header_button(
    part,
    &Name::new(icons::MORE_HORIZONTAL),
    label,
    Arc::new(move || toggle.update(|open| *open = !*open)),
  )
  .ref_element(anchor.clone())
  .describe("a11y_haspopup", "menu")
  .describe("a11y_expanded", if menu.open.get() { "true" } else { "false" });
  Column::new()
    .overflow_visible()
    .child(button)
    .child(Menu::new(menu.open, anchor).items((menu.items)()))
    .into()
}

// ---------------------------------------------------------------------------------------------------------------------
// The header of a panel
// ---------------------------------------------------------------------------------------------------------------------

/// The parts of a panel's header a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PanelHeaderPart {
  /// The bar: its height, its fill, the line along its bottom.
  Root,
  /// What stands before the title: an element of the app's.
  Media,
  /// The title, on one line, cut with an ellipsis where it has no room.
  Title,
  /// The buttons at its end, in a row.
  Actions,
  /// One button: a square with its icon.
  Button,
}

names!(PanelHeaderPart {
  Root => "root",
  Media => "media",
  Title => "title",
  Actions => "actions",
  Button => "button",
});

/// The states of a panel's header a recipe styles. It has nothing of its own the pointer or the keyboard does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PanelHeaderState {
  /// It has something before its title.
  Media,
}

names!(PanelHeaderState {
  Media => "media",
});

/// What builds a panel header's nodes: a theme's in place of the default one.
pub type PanelHeaderRenderer = Arc<dyn Fn(PanelHeaderView) -> Element + Send + Sync>;

/// The header of a panel that stands on its own in a layout: a side bar's Explorer, an inspector's Properties. Its
/// title in small capitals, and its buttons at its end: a menu, a close.
///
/// ```
/// use lurq::{core::Signal, ui::{MenuItem, PanelHeader, icons}};
///
/// # fn header(more: Signal<bool>) {
/// let explorer = PanelHeader::new("Explorer")
///   .button(icons::REFRESH, "Refresh", || println!("refresh"))
///   .menu(more, || [MenuItem::new("Collapse all"), MenuItem::new("Reveal in files")])
///   .on_close(|| println!("hide the explorer"));
/// # }
/// ```
///
/// To tooling it is a heading of the level `level` (2 unless told), named by its title, with its buttons after it.
pub struct PanelHeader {
  title: Arc<str>,
  media: Option<Element>,
  buttons: Vec<HeaderButton>,
  menu: Option<HeaderMenu>,
  on_close: Vec<Handler>,
  more_label: Arc<str>,
  close_label: Arc<str>,
  level: u32,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl PanelHeader {
  /// A header called `title`.
  pub fn new(title: &str) -> Self {
    Self {
      title: Arc::from(title),
      media: None,
      buttons: Vec::new(),
      menu: None,
      on_close: Vec::new(),
      more_label: super::current_locale().common.more.clone(),
      close_label: super::current_locale().common.close.clone(),
      level: 2,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// An element of the app's before the title: an icon, an avatar.
  pub fn media(mut self, media: impl Into<Element>) -> Self {
    self.media = Some(media.into());
    self
  }

  /// A button at its end, after those it has: its icon, what it is called to tooling, and what a press runs.
  pub fn button(mut self, icon: impl Into<Name>, label: &str, on_click: impl Fn() + Send + Sync + 'static) -> Self {
    self.buttons.push(HeaderButton {
      icon: icon.into(),
      label: Arc::from(label),
      on_click: Arc::new(on_click),
    });
    self
  }

  /// The menu its "more" button opens, after its buttons: whether it is open is the app's, and `items` makes its
  /// items.
  pub fn menu<I: IntoIterator<Item = MenuItem>>(
    mut self,
    open: Signal<bool>,
    items: impl Fn() -> I + Send + Sync + 'static,
  ) -> Self {
    self.menu = Some(HeaderMenu {
      open,
      items: Arc::new(move || items().into_iter().collect()),
    });
    self
  }

  /// Called on a press of its close button, which it shows once this is given. A second handler is called after the
  /// first.
  pub fn on_close(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_close.push(Arc::new(handler));
    self
  }

  /// What its "more" button is called to tooling: "More" unless told.
  pub fn more_label(mut self, label: &str) -> Self {
    self.more_label = Arc::from(label);
    self
  }

  /// What its close button is called to tooling: "Close" unless told.
  pub fn close_label(mut self, label: &str) -> Self {
    self.close_label = Arc::from(label);
    self
  }

  /// The level of the heading it is to tooling: 2 unless told.
  pub fn level(mut self, level: u32) -> Self {
    self.level = level;
    self
  }

  /// Puts it in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of it over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for it.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws its buttons as they are under the pointer or focused from the keyboard: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the bar.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// What tells it apart from those beside it.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds it in `theme`. Converting it into an element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(PanelHeaderState::Media, self.media.is_some());
    let recipe = theme
      .resolve::<PanelHeader>(&case, Some(&self.rules))
      .owned_by(self.id.as_ref());
    let view = PanelHeaderView {
      title: self.title,
      media: self.media,
      buttons: self.buttons,
      menu: self.menu,
      on_close: self.on_close,
      more_label: self.more_label,
      close_label: self.close_label,
      level: self.level,
      recipe,
      forced: self.forced,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<PanelHeader>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<PanelHeader> for Element {
  fn from(header: PanelHeader) -> Self {
    header.build(&current_theme())
  }
}

/// A panel's header as a renderer sees it: what it was given, the style of its parts, and its buttons.
pub struct PanelHeaderView {
  pub title: Arc<str>,
  pub media: Option<Element>,
  buttons: Vec<HeaderButton>,
  menu: Option<HeaderMenu>,
  on_close: Vec<Handler>,
  more_label: Arc<str>,
  close_label: Arc<str>,
  level: u32,
  /// The style of each part of the header.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl PanelHeaderView {
  /// The style of a part, as this header is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The title in the style of its part, in capitals, on one line.
  pub fn title(&self) -> Element {
    let part = self.part(PanelHeaderPart::Title);
    Row::new()
      .flex(1.0)
      .min_width(0.0)
      .part(&part)
      .describe("a11y_role", "heading")
      .describe("a11y_name", self.title.clone())
      .describe("a11y_level", self.level.to_string())
      .child(one_line(&part, &self.title.to_uppercase()))
      .into()
  }

  /// Its buttons at its end: the app's, its menu's, and its close.
  pub fn actions(&self) -> Row {
    let button = self.part(PanelHeaderPart::Button);
    let mut actions = Row::new()
      .align_items(Alignment::Center)
      .overflow_visible()
      .part(&self.part(PanelHeaderPart::Actions));
    for each in &self.buttons {
      actions = actions.child(header_button(&button, &each.icon, &each.label, each.on_click.clone()));
    }
    if let Some(menu) = self.menu.clone() {
      actions = actions.child(header_menu(&button.clone().keyed("more"), &self.more_label, menu));
    }
    if !self.on_close.is_empty() {
      let handlers = self.on_close.clone();
      actions = actions.child(header_button(
        &button.clone().keyed("close"),
        &Name::new(icons::CLOSE),
        &self.close_label,
        Arc::new(move || handlers.iter().for_each(|handler| handler())),
      ));
    }
    actions
  }

  /// How the default renderer builds a header: what stands before the title, the title, and its buttons.
  pub fn render(mut self) -> Element {
    let mut root = Row::new()
      .align_items(Alignment::Center)
      .overflow_visible()
      .part(&self.part(PanelHeaderPart::Root));
    if let Some(media) = self.media.take() {
      root = root.child(Row::new().part(&self.part(PanelHeaderPart::Media)).child(media));
    }
    root = root.child(self.title()).child(self.actions());
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

impl crate::ui::Styled for PanelHeader {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for PanelHeader {
  const NAME: &'static str = "panel_header";
  type Renderer = PanelHeaderRenderer;

  fn parts() -> crate::ui::Declared {
    PanelHeaderPart::declared()
  }

  fn states() -> crate::ui::Declared {
    PanelHeaderState::declared()
  }

  /// The design's panel header (`pencil/lurq-design-system.pen`, Panel Header): a bar 30 high on the bar's fill with
  /// a line of 1 in the border's color along its bottom, 10 in at its start and 4 at its end. Its title is 11 and
  /// semibold, in capitals 0.6 apart, in the muted foreground; its buttons are 22 square with the small radius, an icon
  /// of 14 in the foreground in each, lit under the pointer.
  fn recipe() -> Recipe {
    use PaletteColor::{Border as Line, Foreground, Highlight, MutedForeground};
    use PanelHeaderPart::{Actions, Button, Media, Root, Title};
    Recipe::new()
      .base(Root, |s| {
        s.height(30.0)
          .padding_start(10.0)
          .padding_end(4.0)
          .gap(8.0)
          .background(PaletteColor::Bar)
          .underline(Border::inside(BorderSize::Sm, Line))
      })
      .base(Media, |s| s.foreground(MutedForeground))
      .base(Title, |s| {
        s.typography(TypographyStyle::Label)
          .font_size(11.0)
          .weight(FontWeight::SemiBold)
          .letter_spacing(0.6)
          .foreground(MutedForeground)
      })
      .base(Actions, |s| s.gap(0.0))
      .base(Button, button_style)
      .hovered(Button, |s| s.background(Highlight))
      .focus_visible(Button, |s| s.focus_ring())
      .base(Button, |s| s.foreground(Foreground))
  }
}

/// A button of a header at rest: 22 square with the small radius, its icon of 14.
pub(super) fn button_style(style: PartStyle) -> PartStyle {
  fading(style.square(22.0).radius(RadiusSize::Sm).cursor(CursorIcon::Pointer))
}

// ---------------------------------------------------------------------------------------------------------------------
// The floating panel
// ---------------------------------------------------------------------------------------------------------------------

/// The parts of a floating panel a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FloatingPanelPart {
  /// The panel: its fill, its edge, its corners and its shadow.
  Root,
  /// The title bar: what is dragged to move the panel.
  Header,
  /// The grip at the start of the title bar: what takes the focus and the arrows that move the panel.
  Grip,
  /// What stands between the grip and the title: an element of the app's.
  Media,
  /// The title, on one line, cut with an ellipsis where it has no room.
  Title,
  /// The buttons at the end of the title bar.
  Actions,
  /// One button of the title bar.
  Button,
  /// Where what the panel holds goes.
  Body,
  /// The row along its bottom, which holds the resize grip.
  Footer,
  /// The grip in its corner: what is dragged, and takes the focus and the arrows, to resize the panel.
  ResizeGrip,
  /// The room along an edge that a drag resizes the panel by.
  Edge,
}

names!(FloatingPanelPart {
  Root => "root",
  Header => "header",
  Grip => "grip",
  Media => "media",
  Title => "title",
  Actions => "actions",
  Button => "button",
  Body => "body",
  Footer => "footer",
  ResizeGrip => "resize_grip",
  Edge => "edge",
});

/// The states of a floating panel a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FloatingPanelState {
  /// It is the one in front, the last pressed, of the panels of a layer; one on its own is.
  Active,
  /// It shows its title bar alone.
  Collapsed,
  /// It has a size of its own, which a person gave it: what it holds scrolls in it.
  Sized,
  /// Its edges and its corner resize it.
  Resizable,
  /// It has something between its grip and its title.
  Media,
}

names!(FloatingPanelState {
  Active => "active",
  Collapsed => "collapsed",
  Sized => "sized",
  Resizable => "resizable",
  Media => "media",
});

/// What builds a floating panel's nodes: a theme's in place of the default one.
pub type FloatingPanelRenderer = Arc<dyn Fn(FloatingPanelView) -> Element + Send + Sync>;

/// A place in the room the panel floats in, from its top start: `(x, y)` in logical pixels.
pub type PanelPosition = (f32, f32);

/// A size: `(width, height)` in logical pixels.
pub type PanelSize = (f32, f32);

/// A floating panel: a tool window inside the app's window, over its content. Layers, a color picker, the
/// properties of what is chosen. A person moves it by its title bar, resizes it by its edges and its corner,
/// collapses it to its title bar and closes it.
///
/// ```
/// use lurq::{
///   app::ctx::Ctx,
///   components::{Column, Stack},
///   core::Signal,
///   node::Element,
///   ui::FloatingPanel,
/// };
///
/// fn editor(ctx: &mut Ctx, layers: Signal<bool>, place: Signal<(f32, f32)>, canvas: Element) -> Element {
///   let mut stage = Stack::new().child(canvas);
///   if layers.get() {
///     let close = layers.clone();
///     stage = stage.child(
///       FloatingPanel::new("Layers")
///         .position(place)
///         .on_dismiss(move || close.set(false))
///         .child(Column::new())
///         .mount(ctx),
///     );
///   }
///   stage.into()
/// }
/// ```
///
/// It is mounted ([`FloatingPanel::mount`]): it keeps where a drag began, and where the app gives no signal of its
/// own, its place, its size and whether it is open. Mounted, it is a layer as large as what holds it (a
/// [`Stack`](crate::components::Stack) over the content) that lets a press go through it where the panel is not, with
/// the panel in it at its place; a drag keeps the panel inside that layer. Several panels over one content are a
/// [`FloatingPanels`], which brings the one pressed to the front.
///
/// From the keyboard: its grip, at the start of its title bar, is a stop of Tab, and the arrow keys move the panel by
/// 10 (by 1 with Shift); the grip in its corner resizes it by them in the same way; its buttons are pressed with Enter
/// and Space, and Escape anywhere in it closes it where it can be closed.
#[derive(Clone)]
pub struct FloatingPanel {
  title: Arc<str>,
  media: Option<Element>,
  children: Vec<Element>,
  buttons: Vec<HeaderButton>,
  menu: Option<HeaderMenu>,
  open: Option<Signal<bool>>,
  default_open: bool,
  on_open_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  collapsible: bool,
  on_dismiss: Vec<Handler>,
  close_button: bool,
  dismiss_on_escape: bool,
  position: Kept<PanelPosition>,
  size: Kept<Option<PanelSize>>,
  min_size: PanelSize,
  resizable: bool,
  label: Option<Arc<str>>,
  move_label: Arc<str>,
  resize_label: Arc<str>,
  collapse_label: Arc<str>,
  more_label: Arc<str>,
  close_label: Arc<str>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
  on_active_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  /// What a layer of panels makes of the panel, where it is in one.
  layered: Option<Layered>,
  /// What a dock layout hears of the panel, where it floats in one.
  pub(crate) docked: Option<DockHooks>,
}

/// What a dock layout hears of a panel that floats in it: each move of a drag of its title bar and the end of it (the
/// event whose `drop_result` is `Some`), and the end of any move or resize, by the pointer or the keys.
#[derive(Clone)]
pub(crate) struct DockHooks {
  pub(crate) drag: Arc<dyn Fn(&DragEvent) + Send + Sync>,
  pub(crate) settled: Handler,
}

/// What a layer of panels gives a panel in it: what a press in it or the focus coming into it does (it brings the
/// panel to the front and makes it the active one), what a press outside it or the focus going outside it does (it is
/// no longer the active one), and whether it is the active one.
#[derive(Clone)]
struct Layered {
  activate: Handler,
  deactivate: Handler,
  active: bool,
}

/// A value of a panel: the app's signal, the app's value with its handler, or the panel's own with what starts it;
/// and what hears it change.
#[derive(Clone)]
enum Hold<T: SignalValue> {
  Signal(Signal<T>),
  Value(T),
  Own(T),
}

/// A value of a panel and what hears it change.
#[derive(Clone)]
struct Kept<T: SignalValue> {
  hold: Hold<T>,
  heard: Vec<Arc<dyn Fn(T) + Send + Sync>>,
}

impl<T: Clone + Send + Sync + SignalValue + 'static> Kept<T> {
  fn own(value: T) -> Self {
    Self {
      hold: Hold::Own(value),
      heard: Vec::new(),
    }
  }

  /// The value now, given the panel's own.
  fn get(&self, own: &Signal<T>) -> T {
    match &self.hold {
      Hold::Signal(signal) => signal.get(),
      Hold::Value(value) => value.clone(),
      Hold::Own(_) => own.get(),
    }
  }

  /// What sets the value and tells it: the signal, or the panel's own, and every handler.
  fn setter(&self, own: &Signal<T>) -> Arc<dyn Fn(T) + Send + Sync>
  where
    T: PartialEq,
  {
    let (hold, heard, own) = (self.hold.clone(), self.heard.clone(), own.clone());
    Arc::new(move |value: T| {
      let now = match &hold {
        Hold::Signal(signal) => signal.get_untracked(),
        Hold::Value(value) => value.clone(),
        Hold::Own(_) => own.get_untracked(),
      };
      if now == value {
        return;
      }
      match &hold {
        Hold::Signal(signal) => signal.set(value.clone()),
        Hold::Value(_) => {}
        Hold::Own(_) => own.set(value.clone()),
      }
      for handler in &heard {
        handler(value.clone());
      }
    })
  }
}

impl FloatingPanel {
  /// A panel called `title`.
  pub fn new(title: &str) -> Self {
    Self {
      title: Arc::from(title),
      media: None,
      children: Vec::new(),
      buttons: Vec::new(),
      menu: None,
      open: None,
      default_open: true,
      on_open_change: Vec::new(),
      collapsible: true,
      on_dismiss: Vec::new(),
      close_button: true,
      dismiss_on_escape: true,
      position: Kept::own((16.0, 16.0)),
      size: Kept::own(None),
      min_size: (160.0, 80.0),
      resizable: true,
      label: None,
      move_label: super::current_locale().navigation.move_panel.clone(),
      resize_label: super::current_locale().navigation.resize.clone(),
      collapse_label: super::current_locale().navigation.collapse.clone(),
      more_label: super::current_locale().common.more.clone(),
      close_label: super::current_locale().common.close.clone(),
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
      on_active_change: Vec::new(),
      layered: None,
      docked: None,
    }
  }

  /// Free content of the panel, after what it holds.
  pub fn child(mut self, child: impl Into<Element>) -> Self {
    self.children.push(child.into());
    self
  }

  /// Free content of the panel, in order, after what it holds.
  pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.children.extend(children.into_iter().map(Into::into));
    self
  }

  /// An element of the app's between the grip and the title: an icon, a swatch.
  pub fn media(mut self, media: impl Into<Element>) -> Self {
    self.media = Some(media.into());
    self
  }

  /// A button of the title bar, after those it has and before its own: its icon, what it is called to tooling, and
  /// what a press runs.
  pub fn button(mut self, icon: impl Into<Name>, label: &str, on_click: impl Fn() + Send + Sync + 'static) -> Self {
    self.buttons.push(HeaderButton {
      icon: icon.into(),
      label: Arc::from(label),
      on_click: Arc::new(on_click),
    });
    self
  }

  /// The menu the "more" button of its title bar opens: whether it is open is the app's, and `items` makes its items
  /// each time the panel is drawn.
  pub fn menu<I: IntoIterator<Item = MenuItem>>(
    mut self,
    open: Signal<bool>,
    items: impl Fn() -> I + Send + Sync + 'static,
  ) -> Self {
    self.menu = Some(HeaderMenu {
      open,
      items: Arc::new(move || items().into_iter().collect()),
    });
    self
  }

  /// Whether it shows what it holds (`true`) or its title bar alone: the app's signal, which its collapse button
  /// turns. Without one the panel keeps its own.
  pub fn open(mut self, open: Signal<bool>) -> Self {
    self.open = Some(open);
    self
  }

  /// Whether it is open as it first shows, where the app gives no signal: it is unless told.
  pub fn default_open(mut self, open: bool) -> Self {
    self.default_open = open;
    self
  }

  /// Called with whether it is open when its collapse button turns it. Not when the app sets the signal. A second
  /// handler is called after the first.
  pub fn on_open_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_open_change.push(Arc::new(handler));
    self
  }

  /// Called with whether it is the active panel, its title in the foreground, when that changes by what a person
  /// does: a press in it, or the focus coming into it, makes it active; a press outside it (on the content it floats
  /// over, on another panel) or the focus going to something outside it makes it not. A panel starts not active. Not
  /// when the app sets the active panel of a [`FloatingPanels`]. A second handler is called after the first.
  pub fn on_active_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_active_change.push(Arc::new(handler));
    self
  }

  /// Whether its title bar has the button that collapses it to its title bar: it has unless told.
  pub fn collapsible(mut self, collapsible: bool) -> Self {
    self.collapsible = collapsible;
    self
  }

  /// Called when a person closes it: its close button, or Escape in it. The app takes it away. A second handler is
  /// called after the first.
  pub fn on_dismiss(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_dismiss.push(Arc::new(handler));
    self
  }

  /// Whether its title bar has a close button where it has `on_dismiss`: it has unless told.
  pub fn close_button(mut self, close_button: bool) -> Self {
    self.close_button = close_button;
    self
  }

  /// Whether Escape anywhere in it closes it, where it has `on_dismiss`: it does unless told.
  pub fn dismiss_on_escape(mut self, dismiss: bool) -> Self {
    self.dismiss_on_escape = dismiss;
    self
  }

  /// Its place in the room it floats in, from its top start: the app's signal, which a drag and the keys set.
  /// Without one the panel keeps its own, 16 in from the top start unless [`FloatingPanel::default_position`] says.
  pub fn position(mut self, position: Signal<PanelPosition>) -> Self {
    self.position.hold = Hold::Signal(position);
    self
  }

  /// Its place, the app's to hold: `on_position_change` is told where a drag and the keys move it, and it stands
  /// where the app says next.
  pub fn position_controlled(
    mut self,
    position: PanelPosition,
    on_position_change: impl Fn(PanelPosition) + Send + Sync + 'static,
  ) -> Self {
    self.position.hold = Hold::Value(position);
    self.on_position_change(on_position_change)
  }

  /// Where it first stands, where the app gives no place of its own.
  pub fn default_position(mut self, position: PanelPosition) -> Self {
    if let Hold::Own(_) = self.position.hold {
      self.position.hold = Hold::Own(position);
    }
    self
  }

  /// Called with its new place when a person moves it. Not when the app sets the signal. A second handler is called
  /// after the first.
  pub fn on_position_change(mut self, handler: impl Fn(PanelPosition) + Send + Sync + 'static) -> Self {
    self.position.heard.push(Arc::new(handler));
    self
  }

  /// Its size: the app's signal, which a person resizing it sets. `None` is the board's width of 240 and the height
  /// of what it holds; a size of its own makes what it holds scroll in it. Without a signal the panel keeps its own.
  pub fn size(mut self, size: Signal<Option<PanelSize>>) -> Self {
    let heard = std::mem::take(&mut self.size.heard);
    self.size = Kept {
      hold: Hold::Signal(size),
      heard,
    };
    self
  }

  /// Its size, the app's to hold: `on_size_change` is told the size a person resizes it to, and it has the size the
  /// app says next.
  pub fn size_controlled(
    mut self,
    size: Option<PanelSize>,
    on_size_change: impl Fn(PanelSize) + Send + Sync + 'static,
  ) -> Self {
    self.size.hold = Hold::Value(size);
    self.on_size_change(on_size_change)
  }

  /// The size it first has, where the app gives none of its own.
  pub fn default_size(mut self, size: PanelSize) -> Self {
    if let Hold::Own(_) = self.size.hold {
      self.size.hold = Hold::Own(Some(size));
    }
    self
  }

  /// Called with its new size when a person resizes it. Not when the app sets the signal. A second handler is called
  /// after the first.
  pub fn on_size_change(mut self, handler: impl Fn(PanelSize) + Send + Sync + 'static) -> Self {
    self.size.heard.push(Arc::new(move |size: Option<PanelSize>| {
      if let Some(size) = size {
        handler(size);
      }
    }));
    self
  }

  /// The least size a person resizes it to: 160 by 80 unless told.
  pub fn min_size(mut self, size: PanelSize) -> Self {
    self.min_size = size;
    self
  }

  /// Whether its edges and its corner resize it: they do unless told.
  pub fn resizable(mut self, resizable: bool) -> Self {
    self.resizable = resizable;
    self
  }

  /// What it is called to tooling: its title unless told.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// What its grip, which moves it, is called to tooling: "Move" unless told.
  pub fn move_label(mut self, label: &str) -> Self {
    self.move_label = Arc::from(label);
    self
  }

  /// What its corner grip, which resizes it, is called to tooling: "Resize" unless told.
  pub fn resize_label(mut self, label: &str) -> Self {
    self.resize_label = Arc::from(label);
    self
  }

  /// What its collapse button is called to tooling, open or not: "Collapse" unless told. Whether it is open the
  /// button says on its own.
  pub fn collapse_label(mut self, label: &str) -> Self {
    self.collapse_label = Arc::from(label);
    self
  }

  /// What its "more" button is called to tooling: "More" unless told.
  pub fn more_label(mut self, label: &str) -> Self {
    self.more_label = Arc::from(label);
    self
  }

  /// What its close button is called to tooling: "Close" unless told.
  pub fn close_label(mut self, label: &str) -> Self {
    self.close_label = Arc::from(label);
    self
  }

  /// Puts it in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of it over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for it.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws its grips and buttons as they are under the pointer or focused from the keyboard: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the panel. Its grip is `{id}-grip`, its corner grip `{id}-resize`, its collapse button
  /// `{id}-collapse` and its close button `{id}-close`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// What tells it apart from the panels beside it, in a [`FloatingPanels`]: its title unless told.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds it in `theme` where it stands, as the app's signals say: a still panel that a drag does not move.
  /// Converting it into an element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let own = Own::detached(&self);
    self.core().build(theme, &own)
  }

  /// Mounts it: a layer as large as what holds it, with the panel in it, which a drag moves and resizes.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone().unwrap_or_else(|| self.title.clone());
    ctx.mount_keyed::<PanelHost>(&key, PanelProps { core: self.core() })
  }

  fn core(self) -> Core {
    Core {
      title: self.title,
      media: self.media,
      children: self.children,
      buttons: self.buttons,
      menu: self.menu,
      open: self.open,
      default_open: self.default_open,
      on_open_change: self.on_open_change,
      collapsible: self.collapsible,
      on_dismiss: self.on_dismiss,
      close_button: self.close_button,
      dismiss_on_escape: self.dismiss_on_escape,
      position: self.position,
      size: self.size,
      min_size: self.min_size,
      resizable: self.resizable,
      label: self.label,
      move_label: self.move_label,
      resize_label: self.resize_label,
      collapse_label: self.collapse_label,
      more_label: self.more_label,
      close_label: self.close_label,
      states: self.states,
      rules: self.rules,
      forced: self.forced,
      id: self.id,
      on_active_change: self.on_active_change,
      layered: self.layered,
      docked: self.docked,
    }
  }
}

impl From<FloatingPanel> for Element {
  fn from(panel: FloatingPanel) -> Self {
    panel.build(&current_theme())
  }
}

/// A floating panel as it is built.
#[derive(Clone)]
struct Core {
  title: Arc<str>,
  media: Option<Element>,
  children: Vec<Element>,
  buttons: Vec<HeaderButton>,
  menu: Option<HeaderMenu>,
  open: Option<Signal<bool>>,
  default_open: bool,
  on_open_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  collapsible: bool,
  on_dismiss: Vec<Handler>,
  close_button: bool,
  dismiss_on_escape: bool,
  position: Kept<PanelPosition>,
  size: Kept<Option<PanelSize>>,
  min_size: PanelSize,
  resizable: bool,
  label: Option<Arc<str>>,
  move_label: Arc<str>,
  resize_label: Arc<str>,
  collapse_label: Arc<str>,
  more_label: Arc<str>,
  close_label: Arc<str>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  on_active_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  layered: Option<Layered>,
  docked: Option<DockHooks>,
}

/// What a panel keeps where the app keeps nothing, and what a drag needs.
#[derive(Clone)]
struct Own {
  open: Signal<bool>,
  position: Signal<PanelPosition>,
  size: Signal<Option<PanelSize>>,
  /// The layer the panel floats in, whose size a drag keeps it inside.
  layer: ElementRef,
  /// The panel, whose size a drag reads.
  panel: ElementRef,
  /// Where a drag began: the place and the size then.
  drag: Arc<Mutex<Option<(PanelPosition, PanelSize)>>>,
  /// Whether it is mounted: a drag moves it.
  mounted: bool,
  /// Whether it is the active panel, where it is in no layer.
  active: Signal<bool>,
  /// Whether its menu was open as it was last drawn: a press that closes the menu, which is outside the panel, does
  /// not make the panel inactive.
  menu_open: Arc<AtomicBool>,
}

impl Own {
  /// What a panel that is not mounted is built with: its starting values, and nothing a drag can keep.
  fn detached(panel: &FloatingPanel) -> Self {
    let start = |kept: &Hold<PanelPosition>| match kept {
      Hold::Own(value) | Hold::Value(value) => *value,
      Hold::Signal(signal) => signal.get_untracked(),
    };
    let size = match &panel.size.hold {
      Hold::Own(value) | Hold::Value(value) => *value,
      Hold::Signal(signal) => signal.get_untracked(),
    };
    Self {
      open: Signal::new(panel.default_open),
      position: Signal::new(start(&panel.position.hold)),
      size: Signal::new(size),
      layer: ElementRef::new(),
      panel: ElementRef::new(),
      drag: Arc::new(Mutex::new(None)),
      mounted: false,
      // A still panel is drawn as the board draws one: in front.
      active: Signal::new(true),
      menu_open: Arc::new(AtomicBool::new(false)),
    }
  }
}

/// Which edges a drag of a handle resizes the panel by: its start (left), its end (right), its bottom.
#[derive(Clone, Copy)]
struct Edges {
  start: bool,
  end: bool,
  bottom: bool,
}

impl Core {
  /// What makes the panel active and what makes it not: its layer's, or its own where it is in none, which tell the
  /// handlers of a change.
  fn activity(&self, own: &Own) -> (Handler, Handler) {
    if let Some(layered) = &self.layered {
      return (layered.activate.clone(), layered.deactivate.clone());
    }
    let turn = |to: bool| -> Handler {
      let (active, handlers) = (own.active.clone(), self.on_active_change.clone());
      Arc::new(move || {
        if active.get_untracked() != to {
          active.set(to);
          handlers.iter().for_each(|handler| handler(to));
        }
      })
    };
    (turn(true), turn(false))
  }

  fn build(self, theme: &Theme, own: &Own) -> Element {
    let open = match &self.open {
      Some(open) => open.get(),
      None => own.open.get(),
    };
    let size = self.size.get(&own.size);
    let active = match &self.layered {
      Some(layered) => layered.active,
      None => own.active.get(),
    };
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(FloatingPanelState::Active, active)
    .state(FloatingPanelState::Collapsed, !open)
    .state(FloatingPanelState::Sized, size.is_some() && open)
    .state(FloatingPanelState::Resizable, self.resizable)
    .state(FloatingPanelState::Media, self.media.is_some());
    let recipe = theme
      .resolve::<FloatingPanel>(&case, Some(&self.rules))
      .owned_by(self.id.as_ref());
    let view = FloatingPanelView {
      title: self.title.clone(),
      open,
      position: self.position.get(&own.position),
      size,
      active,
      recipe,
      core: self,
      own: own.clone(),
    };
    match theme.renderer::<FloatingPanel>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

/// A floating panel as a renderer sees it: where it stands, its size, whether it is open and in front, the style of
/// its parts, and its behavior ready to be dressed.
pub struct FloatingPanelView {
  pub title: Arc<str>,
  /// Whether it shows what it holds, or its title bar alone.
  pub open: bool,
  /// Where it stands in the layer it floats in.
  pub position: PanelPosition,
  /// Its size of its own, where a person gave it one.
  pub size: Option<PanelSize>,
  /// Whether it is the active panel: pressed last, or with the focus in it.
  pub active: bool,
  /// The style of each part of the panel.
  pub recipe: ResolvedRecipe,
  core: Core,
  own: Own,
}

impl FloatingPanelView {
  /// The style of a part, as this panel is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.core.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  fn id_of(&self, suffix: &str) -> Option<String> {
    self.core.id.as_ref().map(|id| format!("{id}-{suffix}"))
  }

  /// What moves the panel to a place, kept inside the layer it floats in.
  fn mover(&self) -> impl Fn(PanelPosition) + Send + Sync + Clone + 'static {
    let set = self.core.position.setter(&self.own.position);
    let (layer, panel) = (self.own.layer.clone(), self.own.panel.clone());
    move |(x, y): PanelPosition| {
      let (room_width, room_height) = (layer.width(), layer.height());
      let (width, height) = (panel.width(), panel.height());
      let x = x.min(room_width - width).max(0.0);
      let y = y.min(room_height - height).max(0.0);
      set((x.round(), y.round()));
    }
  }

  /// What resizes the panel from where a drag of `edges` began, by `(dx, dy)`: no smaller than its least size, kept
  /// inside the layer.
  fn resizer(&self, edges: Edges) -> impl Fn(PanelPosition, PanelSize, f32, f32) + Send + Sync + Clone + 'static {
    let set_size = self.core.size.setter(&self.own.size);
    let set_position = self.core.position.setter(&self.own.position);
    let (layer, min) = (self.own.layer.clone(), self.core.min_size);
    move |(x, y): PanelPosition, (width, height): PanelSize, dx: f32, dy: f32| {
      let (room_width, room_height) = (layer.width(), layer.height());
      let (mut x, mut width, mut height) = (x, width, height);
      if edges.end {
        width = (width + dx).max(min.0).min(room_width - x);
      }
      if edges.start {
        let moved = dx.min(width - min.0).max(-x);
        x += moved;
        width -= moved;
      }
      if edges.bottom {
        height = (height + dy).max(min.1).min(room_height - y);
      }
      set_position((x.round(), y.round()));
      set_size(Some((width.round(), height.round())));
    }
  }

  /// Where the panel stands now and its size, as a drag or a key starts from it.
  fn now(&self) -> (PanelPosition, PanelSize) {
    let size = self
      .size
      .unwrap_or_else(|| (self.own.panel.width(), self.own.panel.height()));
    (self.position, size)
  }

  /// The grip at the start of the title bar: a stop of Tab whose arrows move the panel.
  pub fn grip(&self) -> Element {
    let part = self.part(FloatingPanelPart::Grip);
    let mut grip = pressable(&part, false)
      .align_items(Alignment::Center)
      .describe("a11y_role", "button")
      .describe("a11y_name", format!("{} {}", self.core.move_label, self.title))
      .describe("a11y_description", super::current_locale().navigation.move_hint.clone())
      .child(glyph(&part, Name::new(icons::GRIP_VERTICAL), false));
    if self.own.mounted {
      let (mover, position, docked) = (self.mover(), self.position, self.core.docked.clone());
      grip = grip.on_key_down(move |event: KeyboardEvent| {
        if !event.focus_within || event.ctrl || event.alt || event.meta {
          return;
        }
        let step = if event.shift { 1.0 } else { 10.0 };
        let (x, y) = position;
        let to = match event.key.as_str() {
          "ArrowLeft" => (x - step, y),
          "ArrowRight" => (x + step, y),
          "ArrowUp" => (x, y - step),
          "ArrowDown" => (x, y + step),
          _ => return,
        };
        event.prevent_default();
        mover(to);
        if let Some(docked) = &docked {
          (docked.settled)();
        }
      });
    }
    if let Some(id) = self.id_of("grip") {
      grip = grip.id(id);
    }
    grip.into()
  }

  /// The title bar: the grip, what stands before the title, the title, and the buttons; what a drag moves the panel
  /// by.
  pub fn header(&self) -> Row {
    let mut header = Row::new()
      .align_items(Alignment::Center)
      .overflow_visible()
      .part(&self.part(FloatingPanelPart::Header))
      .child(self.grip());
    if self.own.mounted {
      let (mover, drag, start) = (self.mover(), self.own.drag.clone(), self.now());
      let (began, ended) = (drag.clone(), drag.clone());
      let (moving, ending) = (self.core.docked.clone(), self.core.docked.clone());
      header = header
        .start_drag_buttons(MouseButtonMask::LEFT)
        .on_drag_start(move |_: DragEvent| *began.lock().unwrap() = Some(start))
        .on_drag_move(move |event: DragEvent| {
          let start = *drag.lock().unwrap();
          if let Some(((x, y), _)) = start {
            mover((x + event.total_delta_x, y + event.total_delta_y));
          }
          if let Some(docked) = &moving {
            (docked.drag)(&event);
          }
        })
        .on_drag_end(move |event: DragEvent| {
          *ended.lock().unwrap() = None;
          if let Some(docked) = &ending {
            (docked.drag)(&event);
            (docked.settled)();
          }
        });
    }
    header
  }

  /// The title in the style of its part, on one line.
  pub fn title(&self) -> Element {
    let part = self.part(FloatingPanelPart::Title);
    Row::new()
      .flex(1.0)
      .min_width(0.0)
      .part(&part)
      .child(one_line(&part, &self.title))
      .into()
  }

  /// The buttons of the title bar: the app's, its menu's, its collapse and its close.
  pub fn actions(&self) -> Row {
    let button = self.part(FloatingPanelPart::Button);
    let mut actions = Row::new()
      .align_items(Alignment::Center)
      .overflow_visible()
      .part(&self.part(FloatingPanelPart::Actions));
    for each in &self.core.buttons {
      actions = actions.child(header_button(&button, &each.icon, &each.label, each.on_click.clone()));
    }
    if let Some(menu) = self.core.menu.clone() {
      actions = actions.child(header_menu(&button, &self.core.more_label, menu));
    }
    if self.core.collapsible {
      let (open, own, heard) = (
        self.core.open.clone(),
        self.own.open.clone(),
        self.core.on_open_change.clone(),
      );
      let now = self.open;
      let mut collapse = header_button(
        &button,
        &Name::new(if now { icons::MINUS } else { icons::PLUS }),
        &self.core.collapse_label,
        Arc::new(move || {
          match &open {
            Some(open) => open.set(!now),
            None => own.set(!now),
          }
          for handler in &heard {
            handler(!now);
          }
        }),
      )
      .describe("a11y_expanded", if now { "true" } else { "false" });
      if let Some(id) = self.id_of("collapse") {
        collapse = collapse.id(id);
      }
      actions = actions.child(collapse);
    }
    if self.core.close_button && !self.core.on_dismiss.is_empty() {
      let handlers = self.core.on_dismiss.clone();
      let mut close = header_button(
        &button,
        &Name::new(icons::CLOSE),
        &self.core.close_label,
        Arc::new(move || handlers.iter().for_each(|handler| handler())),
      );
      if let Some(id) = self.id_of("close") {
        close = close.id(id);
      }
      actions = actions.child(close);
    }
    actions
  }

  /// What the panel holds, in the style of its part; where it has a size of its own, in a room that scrolls.
  pub fn body(&mut self) -> Element {
    let children = std::mem::take(&mut self.core.children);
    let body = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(FloatingPanelPart::Body))
      .with_children(children);
    if self.size.is_some() {
      ScrollVertical::new(body)
        .width(Dimension::Pct(100.0))
        .flex_full(1.0, 1.0, None)
        .into()
    } else {
      body.into()
    }
  }

  /// A handle that a drag resizes the panel by, the edges it moves: the room along an edge, or the grip in its
  /// corner.
  fn resize_handle(&self, handle: components::Button, edges: Edges) -> components::Button {
    if !self.own.mounted {
      return handle;
    }
    let (resize, drag, start) = (self.resizer(edges), self.own.drag.clone(), self.now());
    let (began, ended) = (drag.clone(), drag.clone());
    let docked = self.core.docked.clone();
    handle
      .start_drag_buttons(MouseButtonMask::LEFT)
      .on_drag_start(move |_: DragEvent| *began.lock().unwrap() = Some(start))
      .on_drag_move(move |event: DragEvent| {
        let start = *drag.lock().unwrap();
        if let Some((position, size)) = start {
          resize(position, size, event.total_delta_x, event.total_delta_y);
        }
      })
      .on_drag_end(move |_: DragEvent| {
        *ended.lock().unwrap() = None;
        if let Some(docked) = &docked {
          (docked.settled)();
        }
      })
  }

  /// The grip in the corner: what a drag resizes the panel by, and a stop of Tab whose arrows resize it.
  pub fn resize_grip(&self) -> Element {
    let part = self.part(FloatingPanelPart::ResizeGrip);
    let edges = Edges {
      start: false,
      end: true,
      bottom: true,
    };
    let mut grip = pressable(&part, false)
      .describe("a11y_role", "button")
      .describe("a11y_name", format!("{} {}", self.core.resize_label, self.title))
      .describe(
        "a11y_description",
        super::current_locale().navigation.resize_hint.clone(),
      )
      .child(glyph(&part, Name::new(icons::GRIP), false));
    grip = self.resize_handle(grip, edges);
    if self.own.mounted {
      let (resize, (position, size), docked) = (self.resizer(edges), self.now(), self.core.docked.clone());
      grip = grip.on_key_down(move |event: KeyboardEvent| {
        if !event.focus_within || event.ctrl || event.alt || event.meta {
          return;
        }
        let step = if event.shift { 1.0 } else { 10.0 };
        let (dx, dy) = match event.key.as_str() {
          "ArrowLeft" => (-step, 0.0),
          "ArrowRight" => (step, 0.0),
          "ArrowUp" => (0.0, -step),
          "ArrowDown" => (0.0, step),
          _ => return,
        };
        event.prevent_default();
        resize(position, size, dx, dy);
        if let Some(docked) = &docked {
          (docked.settled)();
        }
      });
    }
    if let Some(id) = self.id_of("resize") {
      grip = grip.id(id);
    }
    grip.into()
  }

  /// The row along the bottom with the corner's grip at its end: a drag of the row resizes the panel's height.
  pub fn footer(&self) -> Element {
    let row = pressable(&self.part(FloatingPanelPart::Footer), false)
      .tab_index(-1)
      .justify(Justify::End)
      .align_items(Alignment::End)
      .child(self.resize_grip());
    self
      .resize_handle(
        row,
        Edges {
          start: false,
          end: false,
          bottom: true,
        },
      )
      .into()
  }

  /// The rooms along its start and its end that a drag resizes its width by: no stops of Tab, the corner's grip
  /// resizes from the keyboard.
  pub fn edges(&self) -> Vec<Element> {
    let width = self.size.map(|(width, _)| width);
    let part = self.part(FloatingPanelPart::Edge);
    let side = match part.rest().width {
      Some(Length::Px(px)) => px,
      _ => 4.0,
    };
    let edge = |x: f32, edges: Edges, cursor: CursorIcon| {
      let mut handle = pressable(&part, false)
        .tab_index(-1)
        .absolute(x, 0.0, Dimension::Px(side), Dimension::Pct(100.0))
        .cursor(cursor);
      if let Some(id) = self.id_of(if edges.start { "edge-start" } else { "edge-end" }) {
        handle = handle.id(id);
      }
      self.resize_handle(handle, edges).into()
    };
    let mut edges = vec![edge(
      0.0,
      Edges {
        start: true,
        end: false,
        bottom: false,
      },
      CursorIcon::EwResize,
    )];
    if let Some(width) = width.or_else(|| self.panel_width()) {
      edges.push(edge(
        width - side,
        Edges {
          start: false,
          end: true,
          bottom: false,
        },
        CursorIcon::EwResize,
      ));
    }
    edges
  }

  /// The panel's width as the recipe gives it, where it has no size of its own.
  fn panel_width(&self) -> Option<f32> {
    match self.part(FloatingPanelPart::Root).rest().width {
      Some(Length::Px(px)) => Some(px),
      _ => None,
    }
  }

  /// The panel with its behavior and the style of its root, with nothing in it yet: a stack, its column of title
  /// bar, body and footer under the edges a drag resizes it by. A dialog to tooling, called by its label, which a
  /// press anywhere in brings to the front and Escape in closes.
  pub fn root(&self) -> Stack {
    let (x, y) = self.position;
    let mut root = Stack::new()
      .part(&self.part(FloatingPanelPart::Root))
      .ref_element(self.own.panel.clone())
      .absolute_position(x, y)
      .describe("a11y_role", "dialog")
      .describe(
        "a11y_name",
        self.core.label.clone().unwrap_or_else(|| self.title.clone()),
      );
    if let Some((width, height)) = self.size {
      root = root.width(width);
      if self.open {
        root = root.height(height);
      }
    }
    if self.own.mounted {
      let (activate, deactivate) = self.core.activity(&self.own);
      let pressed = activate.clone();
      root = root.on_mouse_down(move |_| pressed());
      // Its root hears every key of the window: after one, whether the focus is in it says whether it is active.
      // Tab into it makes it active, Tab out of it to the content makes it not; the keys of its menu, which is
      // outside it, leave it as it is.
      let (menu, menu_open) = (
        self.core.menu.as_ref().map(|menu| menu.open.clone()),
        self.own.menu_open.clone(),
      );
      root = root.on_key_up(move |event: KeyboardEvent| {
        if event.focus_within {
          activate();
        } else if !menu.as_ref().is_some_and(|open| open.get_untracked()) && !menu_open.load(Ordering::Relaxed) {
          deactivate();
        }
      });
    }
    if self.core.dismiss_on_escape && !self.core.on_dismiss.is_empty() {
      let handlers = self.core.on_dismiss.clone();
      root = root.on_key_down(move |event: KeyboardEvent| {
        if event.focus_within && event.key == "Escape" && !event.default_prevented() {
          event.prevent_default();
          handlers.iter().for_each(|handler| handler());
        }
      });
    }
    if let Some(id) = &self.core.id {
      root = root.id(id.clone());
    }
    root
  }

  /// How the default renderer builds a panel: in its layer, the title bar, what it holds and the row with the corner
  /// grip while it is open, and the edges a drag resizes it by.
  pub fn render(mut self) -> Element {
    let mut header = self.header();
    if let Some(media) = self.core.media.take() {
      header = header.child(Row::new().part(&self.part(FloatingPanelPart::Media)).child(media));
    }
    header = header.child(self.title()).child(self.actions());
    let mut column = Column::new()
      .align_items(Alignment::Stretch)
      .width(Dimension::Pct(100.0))
      .child(header);
    if self.size.is_some() && self.open {
      column = column.height(Dimension::Pct(100.0));
    }
    let mut panel = self.root();
    if self.open {
      column = column.child(self.body());
      if self.core.resizable {
        column = column.child(self.footer());
        panel = panel.child(column).with_children(self.edges());
      } else {
        panel = panel.child(column);
      }
    } else {
      panel = panel.child(column);
    }
    // Placed over what holds it, as large as it is once laid out: a stack whose height is its content's has none to
    // give a child a share of.
    Stack::new()
      .absolute(0.0, 0.0, Dimension::Pct(100.0), Dimension::Pct(100.0))
      .hit_test(HitTestBehavior::ContentOnly)
      .overflow_visible()
      .ref_element(self.own.layer.clone())
      .child(panel)
      .into()
  }
}

/// What a mounted panel is given: the panel, built anew by every render of what holds it.
struct PanelProps {
  core: Core,
}

impl PartialEq for PanelProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl crate::app::component::DevtoolsInspectable for PanelProps {}

/// What a mounted panel keeps.
struct PanelHost {
  own: Own,
}

impl Component for PanelHost {
  type Props = PanelProps;

  fn create(ctx: &mut Ctx) -> Self {
    let start = ctx.props::<PanelProps>();
    let (open, position, size) = (
      start.core.default_open,
      match &start.core.position.hold {
        Hold::Own(value) | Hold::Value(value) => *value,
        Hold::Signal(signal) => signal.get_untracked(),
      },
      match &start.core.size.hold {
        Hold::Own(value) | Hold::Value(value) => *value,
        Hold::Signal(signal) => signal.get_untracked(),
      },
    );
    Self {
      own: Own {
        open: ctx.signal(open),
        position: ctx.signal(position),
        size: ctx.signal(size),
        layer: ElementRef::new(),
        panel: ElementRef::new(),
        drag: Arc::new(Mutex::new(None)),
        mounted: true,
        active: ctx.signal(false),
        menu_open: Arc::new(AtomicBool::new(false)),
      },
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let core = ctx.props::<PanelProps>().core.clone();
    // A press outside the panel (on the content it floats over, on another panel) makes it no longer active; not one
    // in its menu, which opens outside it.
    let menu = core.menu.as_ref().map(|menu| menu.open.clone());
    let menu_open = self.own.menu_open.clone();
    menu_open.store(
      menu.as_ref().is_some_and(|open| open.get_untracked()),
      Ordering::Relaxed,
    );
    let (_, deactivate) = core.activity(&self.own);
    ctx.on_click_outside(self.own.panel.clone(), move |_| {
      let open = menu.as_ref().is_some_and(|open| open.get_untracked());
      if !open && !menu_open.load(Ordering::Relaxed) {
        deactivate();
      }
    });
    core.build(&current_theme(), &self.own)
  }
}

impl crate::ui::Styled for FloatingPanel {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for FloatingPanel {
  const NAME: &'static str = "floating_panel";
  type Renderer = FloatingPanelRenderer;

  fn parts() -> crate::ui::Declared {
    FloatingPanelPart::declared()
  }

  fn states() -> crate::ui::Declared {
    FloatingPanelState::declared()
  }

  /// The design's floating panel (`pencil/lurq-design-system.pen`, Floating Panel): 240 wide, on the popover's fill
  /// with an edge of 1 in the border's color inside it, the large radius, and a shadow of 12 down blurred 24 under one
  /// of 3 blurred 6. Its title bar is 30 high on the bar's fill with a line of 1 along its bottom, 6 in at its start
  /// and 4 at its end, 4 between what it holds: a grip of 14 in the muted foreground, the title at 12 and semibold in
  /// the foreground, and buttons of 22 with the small radius, an icon of 14 in the foreground in each. What it holds
  /// is padded 4, its rows 2 apart; the row along its bottom is 22 high, with a grip of 10 in the muted foreground 3
  /// in from its corner. The board draws one panel, in front: one behind it has its title in the muted foreground.
  fn recipe() -> Recipe {
    use FloatingPanelPart::{Actions, Body, Button, Edge, Footer, Grip, Header, Media, ResizeGrip, Root, Title};
    use FloatingPanelState::Active;
    use PaletteColor::{Border as Line, Foreground, Highlight, MutedForeground, Popover, PopoverForeground};
    Recipe::new()
      .base(Root, |s| {
        s.width(240.0)
          .background(Popover)
          .foreground(PopoverForeground)
          .border(Border::inside(BorderSize::Sm, Line))
          .radius(RadiusSize::Lg)
          .shadow(ShadowStyle::Lg)
      })
      .base(Header, |s| {
        s.height(30.0)
          .padding_start(6.0)
          .padding_end(4.0)
          .gap(4.0)
          .background(PaletteColor::Bar)
          .underline(Border::inside(BorderSize::Sm, Line))
          .cursor(CursorIcon::Grab)
      })
      .base(Grip, |s| {
        fading(
          s.square(IconSize::Sm)
            .radius(RadiusSize::Sm)
            .foreground(MutedForeground)
            .cursor(CursorIcon::Grab),
        )
      })
      .focus_visible(Grip, |s| s.focus_ring())
      .base(Media, |s| s.foreground(MutedForeground))
      .base(Title, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(12.0)
          .weight(FontWeight::SemiBold)
          .foreground(MutedForeground)
      })
      .state(Active, Title, |s| s.foreground(Foreground))
      .base(Actions, |s| s.gap(0.0))
      .base(Button, |s| button_style(s).foreground(Foreground))
      .hovered(Button, |s| s.background(Highlight))
      .focus_visible(Button, |s| s.focus_ring())
      .base(Body, |s| s.padding_x(4.0).padding_y(4.0).gap(2.0))
      .base(Footer, |s| {
        s.height(22.0)
          .padding_end(3.0)
          .padding_bottom(3.0)
          .cursor(CursorIcon::NsResize)
      })
      .base(ResizeGrip, |s| {
        fading(
          s.square(10.0)
            .radius(RadiusSize::Sm)
            .foreground(MutedForeground)
            .cursor(CursorIcon::NwseResize),
        )
      })
      .focus_visible(ResizeGrip, |s| s.focus_ring())
      .base(Edge, |s| s.width(4.0))
  }
}

// ---------------------------------------------------------------------------------------------------------------------
// A layer of floating panels
// ---------------------------------------------------------------------------------------------------------------------

/// Floating panels over one content, of which the one pressed last is in front: a press anywhere in a panel brings it
/// to the front. Each panel is told apart from the others by its key (its title unless told).
///
/// ```
/// use lurq::{app::ctx::Ctx, components::Stack, node::Element, ui::{FloatingPanel, FloatingPanels}};
///
/// fn editor(ctx: &mut Ctx, canvas: Element) -> Element {
///   Stack::new()
///     .child(canvas)
///     .child(
///       FloatingPanels::new()
///         .panel(FloatingPanel::new("Layers").default_position((16.0, 16.0)))
///         .panel(FloatingPanel::new("Color").default_position((280.0, 16.0)))
///         .mount(ctx),
///     )
///     .into()
/// }
/// ```
#[derive(Clone)]
pub struct FloatingPanels {
  panels: Vec<FloatingPanel>,
  active: Option<Signal<Option<Arc<str>>>>,
  on_active_change: Vec<Arc<dyn Fn(Option<&str>) + Send + Sync>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Default for FloatingPanels {
  fn default() -> Self {
    Self::new()
  }
}

impl FloatingPanels {
  /// A layer with no panel in it.
  pub fn new() -> Self {
    Self {
      panels: Vec::new(),
      active: None,
      on_active_change: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// A panel, after those it has.
  pub fn panel(mut self, panel: FloatingPanel) -> Self {
    self.panels.push(panel);
    self
  }

  /// Panels, in order, after those it has.
  pub fn panels(mut self, panels: impl IntoIterator<Item = FloatingPanel>) -> Self {
    self.panels.extend(panels);
    self
  }

  /// The active panel, held in the app's signal: the key of a panel (its `key`, or its title where it has none), or
  /// `None` where no panel is active. A press in a panel or the focus coming into it sets its key; a press on the
  /// content the panels float over or the focus going outside every panel sets `None`. A key the app sets shows that
  /// panel active, and brings it to no other place. Without it the layer keeps its own, starting at `None`.
  pub fn active(mut self, active: Signal<Option<Arc<str>>>) -> Self {
    self.active = Some(active);
    self
  }

  /// Called with the key of the active panel, or `None`, when that changes by what a person does: after the signal
  /// is set. Not when the app sets the signal. A second handler is called after the first.
  pub fn on_active_change(mut self, handler: impl Fn(Option<&str>) + Send + Sync + 'static) -> Self {
    self.on_active_change.push(Arc::new(handler));
    self
  }

  /// The id of the layer.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// What tells it apart from what stands beside it.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Mounts it: it keeps the order of its panels, the one pressed last in front, and which is active.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone().unwrap_or_else(|| Arc::from("floating_panels"));
    ctx.mount_keyed::<LayerHost>(&key, LayerProps { layer: self })
  }
}

/// What a mounted layer is given: the layer, built anew by every render of what holds it.
struct LayerProps {
  layer: FloatingPanels,
}

impl PartialEq for LayerProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl crate::app::component::DevtoolsInspectable for LayerProps {}

/// What a mounted layer keeps: the keys of its panels, the one in front last, and the key of the active one where the
/// app keeps none.
struct LayerHost {
  order: Signal<Vec<Arc<str>>>,
  active: Signal<Option<Arc<str>>>,
}

/// What hears the active panel of a layer change: the layer's handlers, and each panel's by its key.
type ActiveHandlers = (
  Vec<Arc<dyn Fn(Option<&str>) + Send + Sync>>,
  Vec<(Arc<str>, Vec<Arc<dyn Fn(bool) + Send + Sync>>)>,
);

impl Component for LayerHost {
  type Props = LayerProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      order: ctx.signal(Vec::new()),
      active: ctx.signal(None),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let layer = ctx.props::<LayerProps>().layer.clone();
    let keyed: Vec<(Arc<str>, FloatingPanel)> = layer
      .panels
      .into_iter()
      .map(|panel| (panel.key.clone().unwrap_or_else(|| panel.title.clone()), panel))
      .collect();
    // The kept order, of the panels there still are, and then the new ones in the order they are given.
    let mut order: Vec<Arc<str>> = self
      .order
      .get()
      .into_iter()
      .filter(|key| keyed.iter().any(|(held, _)| held == key))
      .collect();
    for (key, _) in &keyed {
      if !order.contains(key) {
        order.push(key.clone());
      }
    }
    let active_signal = layer.active.clone().unwrap_or_else(|| self.active.clone());
    let active = active_signal.get();
    let heard: Arc<ActiveHandlers> = Arc::new((
      layer.on_active_change.clone(),
      keyed
        .iter()
        .map(|(key, panel)| (key.clone(), panel.on_active_change.clone()))
        .collect(),
    ));
    // Makes `to` the active panel, or none, and tells who hears it of a change.
    let change = move |to: Option<Arc<str>>| {
      let from = active_signal.get_untracked();
      if from == to {
        return;
      }
      active_signal.set(to.clone());
      for (key, handlers) in &heard.1 {
        if from.as_ref() == Some(key) {
          handlers.iter().for_each(|handler| handler(false));
        }
        if to.as_ref() == Some(key) {
          handlers.iter().for_each(|handler| handler(true));
        }
      }
      heard.0.iter().for_each(|handler| handler(to.as_deref()));
    };
    let mut panels: Vec<(usize, Element)> = Vec::new();
    for (key, panel) in keyed {
      let activate = {
        let (signal, key, now, change) = (self.order.clone(), key.clone(), order.clone(), change.clone());
        Arc::new(move || {
          if now.last() != Some(&key) {
            let mut next = now.clone();
            next.retain(|held| *held != key);
            next.push(key.clone());
            signal.set(next);
          }
          change(Some(key.clone()));
        }) as Handler
      };
      let deactivate = {
        let (key, change) = (key.clone(), change.clone());
        let active = layer.active.clone().unwrap_or_else(|| self.active.clone());
        Arc::new(move || {
          if active.get_untracked().as_ref() == Some(&key) {
            change(None);
          }
        }) as Handler
      };
      let at = order.iter().position(|held| *held == key).unwrap_or(0);
      let mut panel = panel.key(key.clone());
      panel.layered = Some(Layered {
        activate,
        deactivate,
        active: active.as_ref() == Some(&key),
      });
      panels.push((at, panel.mount(ctx)));
    }
    panels.sort_by_key(|(at, _)| *at);
    let mut stack = Stack::new()
      .absolute(0.0, 0.0, Dimension::Pct(100.0), Dimension::Pct(100.0))
      .hit_test(HitTestBehavior::ContentOnly)
      .overflow_visible()
      .with_children(panels.into_iter().map(|(_, panel)| panel));
    if let Some(id) = layer.id {
      stack = stack.id(id);
    }
    stack
  }
}
