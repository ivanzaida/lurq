//! A menu: a list of commands in a panel over the window, opened from an anchor.

use std::sync::Arc;

use super::{
  icons, names,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{fading, glyph, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    ctx::{OpenState, Placement},
    theme::{BorderSize, IconSize, OpacityLevel, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle},
  },
  components::{self, Column, Popup, Row},
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, border::Border},
};

/// How far the panel stands from its anchor, in the direction it opens.
const ANCHOR_GAP: f32 = 4.0;

/// The parts of a menu a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MenuPart {
  /// The panel that floats over the window and holds the items.
  Panel,
  /// The room around the line between two groups of items.
  Separator,
  /// The line between two groups of items.
  SeparatorLine,
  /// The text over a group of items: what they are about.
  Label,
}

names!(MenuPart {
  Panel => "panel",
  Separator => "separator",
  SeparatorLine => "separator_line",
  Label => "label",
});

/// What builds a menu's nodes: a theme's in place of the default one.
pub type MenuRenderer = Arc<dyn Fn(MenuView) -> Element + Send + Sync>;

/// What a menu holds, in order.
pub enum MenuEntry {
  /// A command.
  Item(MenuItem),
  /// A line between two groups of items.
  Separator,
  /// A text over the items after it.
  Label(Arc<str>),
}

/// A menu: a list of commands in a panel over the window, opened from an
/// anchor. The actions of a file, the account's menu under an avatar, what a
/// split button offers beside its action.
///
/// ```
/// use lurq::{
///   core::{ElementRef, Signal},
///   ui::{Menu, MenuItem, icons},
/// };
///
/// # fn build(anchor: ElementRef, open: Signal<bool>) {
/// let menu = Menu::new(anchor, open)
///   .label("File")
///   .item(MenuItem::new("Rename").icon(icons::PENCIL).shortcut("F2").on_select(|| println!("rename")))
///   .item(MenuItem::new("Duplicate").icon(icons::COPY).shortcut("Ctrl+D"))
///   .separator()
///   .item(MenuItem::new("Delete").icon(icons::TRASH).destructive(true));
/// # }
/// ```
///
/// A menu does not keep whether it is open: the app does, in the
/// `Signal<bool>` it gives the menu, and opens it from its anchor (a press of
/// the button the menu hangs from sets it). Choosing an item calls the item's
/// handler and closes the menu; a press outside it and Escape close it too.
/// The menu is declared where its anchor is, open or not, so that lurq knows
/// the anchor is its trigger.
pub struct Menu {
  anchor: ElementRef,
  open: OpenState,
  entries: Vec<MenuEntry>,
  placement: Placement,
  inline: bool,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Menu {
  /// A menu that hangs from `anchor` and is open while `open` is: the app's
  /// `Signal<bool>`, which choosing an item, a press outside and Escape set
  /// to `false`. A `bool` holds it open or closed whatever is done to it: for
  /// a gallery or a test.
  pub fn new(anchor: impl Into<ElementRef>, open: impl Into<OpenState>) -> Self {
    Self {
      anchor: anchor.into(),
      open: open.into(),
      entries: Vec::new(),
      placement: Placement::BottomStart,
      inline: false,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// A command, after what the menu holds already.
  pub fn item(mut self, item: MenuItem) -> Self {
    self.entries.push(MenuEntry::Item(item));
    self
  }

  /// Commands, in order, after what the menu holds already.
  pub fn items(mut self, items: impl IntoIterator<Item = MenuItem>) -> Self {
    self.entries.extend(items.into_iter().map(MenuEntry::Item));
    self
  }

  /// A line that ends one group of items and starts the next.
  pub fn separator(mut self) -> Self {
    self.entries.push(MenuEntry::Separator);
    self
  }

  /// A text over the items after it, which says what they are about: "Zoom", "Workspace".
  pub fn label(mut self, label: &str) -> Self {
    self.entries.push(MenuEntry::Label(Arc::from(label)));
    self
  }

  /// Where the panel opens beside its anchor: under it from its start, by default.
  pub fn placement(mut self, placement: Placement) -> Self {
    self.placement = placement;
    self
  }

  /// Draws the panel where the menu is in the tree, open or not, and not over
  /// the window from its anchor: for a gallery, a test, or a list of commands
  /// that is part of a page. Choosing an item still calls its handler and
  /// sets the open signal to `false`.
  pub fn inline(mut self, inline: bool) -> Self {
    self.inline = inline;
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
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this menu.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the panel.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the menu in `theme`. Converting a menu into an element does this
  /// in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    };
    let view = MenuView {
      entries: self.entries,
      placement: self.placement,
      inline: self.inline,
      recipe: theme.resolve::<Menu>(&case, Some(&self.rules)),
      anchor: self.anchor,
      open: self.open,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Menu>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Menu> for Element {
  fn from(menu: Menu) -> Self {
    menu.build(&current_theme())
  }
}

/// A menu as a renderer sees it: what it holds, the style of its parts, and
/// what opens and closes it.
pub struct MenuView {
  /// What the menu holds, in order.
  pub entries: Vec<MenuEntry>,
  pub placement: Placement,
  pub inline: bool,
  /// The style of each part of this menu.
  pub recipe: ResolvedRecipe,
  anchor: ElementRef,
  open: OpenState,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl MenuView {
  /// The style of a part, as this menu is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// Whether the menu is open now.
  pub fn is_open(&self) -> bool {
    self.open.is_open()
  }

  /// The signal that says whether the menu is open, if the app gave it one.
  pub fn open_signal(&self) -> Option<Signal<bool>> {
    self.open.signal()
  }

  /// The panel in the style of its part, with nothing in it yet. It is a menu
  /// to what reads the tree.
  pub fn panel(&self) -> Column {
    let mut panel = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(MenuPart::Panel))
      .describe("a11y_role", "menu");
    if let Some(id) = &self.id {
      panel = panel.id(id.clone());
    }
    if let Some(key) = &self.key {
      panel = panel.key(key.clone());
    }
    panel
  }

  /// The line between two groups of items, with its room.
  pub fn separator(&self) -> Element {
    Row::new()
      .part(&self.part(MenuPart::Separator))
      .describe("a11y_role", "separator")
      .child(Row::new().flex(1.0).part(&self.part(MenuPart::SeparatorLine)))
      .into()
  }

  /// The text over a group of items.
  pub fn label(&self, label: &str) -> Element {
    let part = self.part(MenuPart::Label);
    Row::new().part(&part).child(part.text(label).nowrap()).into()
  }

  /// An item as this menu draws it, in its theme: choosing it closes the menu.
  pub fn item(&self, item: MenuItem) -> Element {
    item.closing(self.open.signal()).build(self.theme())
  }

  /// How the default renderer builds a menu: the panel with what it holds,
  /// hung from the anchor over the window, or in place where it is inline.
  pub fn render(mut self) -> Element {
    let mut panel = self.panel();
    for entry in std::mem::take(&mut self.entries) {
      panel = panel.child(match entry {
        MenuEntry::Item(item) => self.item(item),
        MenuEntry::Separator => self.separator(),
        MenuEntry::Label(label) => self.label(&label),
      });
    }
    if self.inline {
      return panel.into();
    }
    let (x, y) = match self.placement {
      Placement::LeftStart
      | Placement::Left
      | Placement::LeftEnd
      | Placement::RightStart
      | Placement::Right
      | Placement::RightEnd => (ANCHOR_GAP, 0.0),
      _ => (0.0, ANCHOR_GAP),
    };
    Popup::new(self.anchor, panel)
      .open(self.open)
      .placement(self.placement)
      .offset(x, y)
      .into()
  }
}

impl Themed for Menu {
  const NAME: &'static str = "menu";
  type Renderer = MenuRenderer;

  /// The design's menu (`pencil/lurq-design-system.pen`, Menu): a panel of the
  /// popover's color at least 230 wide, padded 4, with the large radius, an
  /// edge of 1 in the border's color and the medium shadow. A separator is a
  /// line of 1 in the border's color with 4 above and below; the label of a
  /// group is 11/600 in the muted foreground, 8 in from the side.
  fn recipe() -> Recipe {
    use MenuPart::{Label, Panel, Separator, SeparatorLine};
    use PaletteColor::{MutedForeground, Popover, PopoverForeground};
    Recipe::new()
      .base(Panel, |s| {
        s.min_width(230.0)
          .padding_x(4.0)
          .padding_y(4.0)
          .radius(RadiusSize::Lg)
          .background(Popover)
          .foreground(PopoverForeground)
          .border(Border::inside(BorderSize::Sm, PaletteColor::Border))
          .shadow(ShadowStyle::Md)
      })
      .base(Separator, |s| s.padding_y(4.0))
      .base(SeparatorLine, |s| s.height(1.0).background(PaletteColor::Border))
      .base(Label, |s| {
        // The board has 6 above the text and 4 under it; a part has one padding for both.
        s.padding_x(8.0)
          .padding_y(5.0)
          .typography(TypographyStyle::Label)
          .font_size(11.0)
          .weight(FontWeight::SemiBold)
          .foreground(MutedForeground)
      })
  }
}

/// The parts of a menu item a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MenuItemPart {
  /// The item itself: what is pressed, and what is highlighted under the pointer or the keyboard.
  Root,
  /// An icon before the label.
  Icon,
  /// Its text.
  Label,
  /// The keys that do the same, at the end: "Ctrl+Z".
  Shortcut,
  /// The check at the end of an item that is checked.
  Check,
}

names!(MenuItemPart {
  Root => "root",
  Icon => "icon",
  Label => "label",
  Shortcut => "shortcut",
  Check => "check",
});

/// The states of a menu item a recipe styles, besides what the pointer and
/// the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MenuItemState {
  /// What it turns on is on: it has a check at its end.
  Checked,
  /// It removes, or cannot be undone: it is in the destructive color.
  Destructive,
  /// It cannot be chosen.
  Disabled,
}

names!(MenuItemState {
  Checked => "checked",
  Destructive => "destructive",
  Disabled => "disabled",
});

/// What builds a menu item's nodes: a theme's in place of the default one.
pub type MenuItemRenderer = Arc<dyn Fn(MenuItemView) -> Element + Send + Sync>;

/// A command in a [`Menu`].
///
/// ```
/// use lurq::ui::{MenuItem, icons};
///
/// let undo = MenuItem::new("Undo").icon(icons::UNDO).shortcut("Ctrl+Z").on_select(|| println!("undo"));
/// let grid = MenuItem::new("Show grid").checked(true);
/// let redo = MenuItem::new("Redo").shortcut("Ctrl+Y").disabled(true);
/// let delete = MenuItem::new("Delete").icon(icons::TRASH).destructive(true);
/// ```
///
/// It is chosen by a click, and by Enter or Space while it is focused; Tab
/// reaches it while its menu is open. Under the pointer or focused from the
/// keyboard it is highlighted.
pub struct MenuItem {
  label: Arc<str>,
  icon: Option<Name>,
  shortcut: Option<Arc<str>>,
  checked: Option<bool>,
  destructive: bool,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_select: Vec<Arc<dyn Fn() + Send + Sync>>,
  closes: Option<Signal<bool>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl MenuItem {
  /// An item called `label`.
  pub fn new(label: &str) -> Self {
    Self {
      label: Arc::from(label),
      icon: None,
      shortcut: None,
      checked: None,
      destructive: false,
      disabled: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_select: Vec::new(),
      closes: None,
      id: None,
      key: None,
    }
  }

  /// An icon before the label, by its name in the theme's set ([`icons`]).
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// The keys that do the same, written at the end of the item: "Ctrl+Z",
  /// "F2". The item only shows them: the app binds the keys.
  pub fn shortcut(mut self, keys: &str) -> Self {
    self.shortcut = Some(Arc::from(keys));
    self
  }

  /// Whether what the item turns on is on: a check at its end. An item given
  /// this is one that is checked or not, to what reads the tree.
  pub fn checked(mut self, checked: bool) -> Self {
    self.checked = Some(checked);
    self
  }

  /// Whether the item removes, or cannot be undone: it is drawn in the destructive color.
  pub fn destructive(mut self, destructive: bool) -> Self {
    self.destructive = destructive;
    self
  }

  /// Whether the item cannot be chosen: it is dimmed, and Tab passes it by.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Puts the item in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Called when the item is chosen, before its menu closes.
  pub fn on_select(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_select.push(Arc::new(handler));
    self
  }

  /// Styles one part of this item over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this item.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the item as it is under the pointer, held down or focused from the
  /// keyboard, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the item: what is pressed.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The signal set to `false` once the item is chosen: its menu's.
  pub(crate) fn closing(mut self, open: Option<Signal<bool>>) -> Self {
    self.closes = open;
    self
  }

  /// Builds the item in `theme`. Converting an item into an element does this
  /// in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(MenuItemState::Checked, self.checked == Some(true))
    .state(MenuItemState::Destructive, self.destructive)
    .state(MenuItemState::Disabled, self.disabled);
    let view = MenuItemView {
      label: self.label,
      icon: self.icon,
      shortcut: self.shortcut,
      checked: self.checked,
      destructive: self.destructive,
      disabled: self.disabled,
      recipe: theme.resolve::<MenuItem>(&case, Some(&self.rules)),
      forced: self.forced,
      on_select: self.on_select,
      closes: self.closes,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<MenuItem>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<MenuItem> for Element {
  fn from(item: MenuItem) -> Self {
    item.build(&current_theme())
  }
}

/// A menu item as a renderer sees it: what it was given, the style of its
/// parts, and its behavior ready to be dressed.
pub struct MenuItemView {
  pub label: Arc<str>,
  /// The name of the icon before the label, if it has one.
  pub icon: Option<Name>,
  /// The keys written at its end, if it has them.
  pub shortcut: Option<Arc<str>>,
  /// Whether it is checked, for an item that is checked or not.
  pub checked: Option<bool>,
  pub destructive: bool,
  pub disabled: bool,
  /// The style of each part of this item.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  on_select: Vec<Arc<dyn Fn() + Send + Sync>>,
  closes: Option<Signal<bool>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl MenuItemView {
  /// The style of a part, as this item is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// What choosing the item does: its handlers are called, then its menu closes.
  fn choose(&self) -> Option<impl Fn() + Send + Sync + Clone + 'static> {
    let (handlers, closes) = (self.on_select.clone(), self.closes.clone());
    (!self.disabled && (!handlers.is_empty() || closes.is_some())).then(|| {
      move || {
        for handler in &handlers {
          handler();
        }
        if let Some(open) = &closes {
          open.set(false);
        }
      }
    })
  }

  /// The item's behavior with the style of its root: the element that is
  /// pressed, focused and disabled, with nothing in it yet. It is a menu item
  /// to what reads the tree, called by its label.
  pub fn root(&self) -> components::Button {
    let role = if self.checked.is_some() {
      "menuitemcheckbox"
    } else {
      "menuitem"
    };
    let mut root = pressable(&self.part(MenuItemPart::Root), self.disabled)
      .justify(Justify::Start)
      .describe("a11y_role", role)
      .describe("a11y_name", self.label.clone());
    if let Some(checked) = self.checked {
      root = root.describe("a11y_checked", if checked { "true" } else { "false" });
    }
    if let Some(choose) = self.choose() {
      root = root.on_click(move |_| choose());
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// The icon before the label, if the item has one.
  pub fn icon(&self) -> Option<Element> {
    self.icon.map(|name| glyph(&self.part(MenuItemPart::Icon), name, false))
  }

  /// The item's text, taking the room the item has to spare, so that what
  /// follows it stands at the end.
  pub fn label(&self) -> Element {
    let part = self.part(MenuItemPart::Label);
    Row::new()
      .flex(1.0)
      .part(&part)
      .child(part.text(&self.label).nowrap())
      .into()
  }

  /// The keys written at the end, if the item has them.
  pub fn shortcut(&self) -> Option<Element> {
    let shortcut = self.shortcut.as_ref()?;
    let part = self.part(MenuItemPart::Shortcut);
    Some(Row::new().part(&part).child(part.text(shortcut).nowrap()).into())
  }

  /// The check at the end, while the item is checked.
  pub fn check(&self) -> Option<Element> {
    (self.checked == Some(true)).then(|| glyph(&self.part(MenuItemPart::Check), Name::new(icons::CHECK), false))
  }

  /// How the default renderer builds an item: the root, with its icon, its
  /// label, its shortcut and its check.
  pub fn render(self) -> Element {
    let mut root = self.root();
    if let Some(icon) = self.icon() {
      root = root.child(icon);
    }
    root = root.child(self.label());
    if let Some(shortcut) = self.shortcut() {
      root = root.child(shortcut);
    }
    if let Some(check) = self.check() {
      root = root.child(check);
    }
    root.into()
  }
}

impl Themed for MenuItem {
  const NAME: &'static str = "menu_item";
  type Renderer = MenuItemRenderer;

  /// The design's menu item (`pencil/lurq-design-system.pen`, Menu Item): 32
  /// tall, padded 8, with the small radius and 8 between what it holds; its
  /// label in the body's type, an icon of 16 in the muted foreground before
  /// it, and at its end the shortcut in the mono type at 11, muted, and the
  /// check. Under the pointer or focused from the keyboard it is filled with
  /// the highlight. A destructive item is in the destructive color, icon and
  /// all; a disabled one is dimmed.
  fn recipe() -> Recipe {
    use MenuItemPart::{Check, Icon, Label, Root, Shortcut};
    use PaletteColor::*;
    Recipe::new()
      .base(Root, |s| {
        fading(
          s.height(32.0)
            .padding_x(8.0)
            .gap(8.0)
            .radius(RadiusSize::Sm)
            .cursor(CursorIcon::Pointer),
        )
      })
      .base(Icon, |s| s.square(IconSize::Md).foreground(MutedForeground))
      .base(Label, |s| s.typography(TypographyStyle::Body))
      .base(Shortcut, |s| {
        s.typography(TypographyStyle::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      .base(Check, |s| s.square(IconSize::Md))
      .hovered(Root, |s| s.background(Highlight))
      .focus_visible(Root, |s| s.background(Highlight))
      .state(MenuItemState::Destructive, Root, |s| s.foreground(Destructive))
      .state(MenuItemState::Destructive, Icon, |s| s.foreground(Destructive))
      .state(MenuItemState::Disabled, Root, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
  }
}
