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
    ctx::{OpenState, OutsidePress, Placement},
    events::KeyboardEvent,
    theme::{BorderSize, IconSize, OpacityLevel, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle},
  },
  components::{self, Column, Popup, Row},
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, FocusGroup, border::Border},
};

/// How far the panel stands from its anchor, in the direction it opens.
const ANCHOR_GAP: f32 = 4.0;

/// Whether a key was pressed with no modifier held but Shift.
fn plain(event: &KeyboardEvent) -> bool {
  !event.ctrl && !event.alt && !event.meta
}

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

/// The states of a menu a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MenuState {
  /// It is the submenu of an item of another menu, and opens beside it.
  Submenu,
}

names!(MenuState {
  Submenu => "submenu",
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
/// # fn build(anchor: ElementRef, open: Signal<bool>, export: ElementRef, exporting: Signal<bool>) {
/// let menu = Menu::new(anchor, open)
///   .label("File")
///   .item(MenuItem::new("Rename").icon(icons::PENCIL).shortcut("F2").on_select(|| println!("rename")))
///   .item(MenuItem::new("Duplicate").icon(icons::COPY).shortcut("Ctrl+D"))
///   .item(
///     MenuItem::new("Export as")
///       .submenu(Menu::new(export, exporting).item(MenuItem::new("PNG")).item(MenuItem::new("SVG"))),
///   )
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
///
/// Up and Down move the focus between the items, Home and End go to the first
/// and the last, and a disabled item is passed by; typing a letter goes to the
/// item whose label starts with it. Tab stops in the menu once.
pub struct Menu {
  anchor: ElementRef,
  open: OpenState,
  entries: Vec<MenuEntry>,
  placement: Option<Placement>,
  inline: bool,
  submenu: bool,
  closes_too: Vec<Signal<bool>>,
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
  ///
  /// The menu of an item's [`submenu`](MenuItem::submenu) hangs from that
  /// item: `anchor` is given to the item, which opens the submenu.
  pub fn new(anchor: impl Into<ElementRef>, open: impl Into<OpenState>) -> Self {
    Self {
      anchor: anchor.into(),
      open: open.into(),
      entries: Vec::new(),
      placement: None,
      inline: false,
      submenu: false,
      closes_too: Vec::new(),
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

  /// Where the panel opens beside its anchor: under it from its start, by
  /// default, and a submenu beside its item from its top.
  pub fn placement(mut self, placement: Placement) -> Self {
    self.placement = Some(placement);
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

  /// Makes this menu the submenu of an item of a menu that `closes` is the
  /// open signals of: choosing one of its items closes those too.
  pub(crate) fn nested(mut self, closes: Vec<Signal<bool>>) -> Self {
    self.submenu = true;
    self.closes_too = closes;
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
    }
    .state(MenuState::Submenu, self.submenu);
    let default = if self.submenu {
      Placement::RightStart
    } else {
      Placement::BottomStart
    };
    // The open signals of the submenus of its items, which the pointer on another item closes.
    let submenus = self
      .entries
      .iter()
      .filter_map(|entry| match entry {
        MenuEntry::Item(item) => item.submenu.as_ref()?.open.signal(),
        _ => None,
      })
      .collect();
    let view = MenuView {
      entries: self.entries,
      submenus,
      placement: self.placement.unwrap_or(default),
      inline: self.inline,
      submenu: self.submenu,
      recipe: theme.resolve::<Menu>(&case, Some(&self.rules)),
      anchor: self.anchor,
      open: self.open,
      closes_too: self.closes_too,
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
  /// Whether it is the submenu of an item of another menu.
  pub submenu: bool,
  /// The style of each part of this menu.
  pub recipe: ResolvedRecipe,
  anchor: ElementRef,
  open: OpenState,
  closes_too: Vec<Signal<bool>>,
  submenus: Vec<Signal<bool>>,
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

  /// The signals choosing an item of this menu sets to `false`: its own, and
  /// those of the menus it is a submenu of.
  fn closes(&self) -> Vec<Signal<bool>> {
    self
      .open
      .signal()
      .into_iter()
      .chain(self.closes_too.iter().cloned())
      .collect()
  }

  /// The panel in the style of its part, with nothing in it yet. It is a menu
  /// to what reads the tree, and a group the arrow keys move the focus in:
  /// Up and Down between its items, Home and End to the first and the last,
  /// a letter to the item whose label starts with it. Tab stops in it once.
  pub fn panel(&self) -> Column {
    let mut panel = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(MenuPart::Panel))
      .focus_group(
        FocusGroup::vertical()
          .one_stop(true)
          .follow_pointer(true)
          .typeahead(true),
      )
      .describe("a11y_role", "menu");
    if let Some(id) = &self.id {
      panel = panel.id(id.clone());
    }
    if let Some(key) = &self.key {
      panel = panel.key(key.clone());
    }
    // Left closes a submenu while the focus is in it; the focus goes back to the item it hangs from.
    if self.submenu
      && let Some(open) = self.open.signal()
    {
      panel = panel.on_key_down(move |event: KeyboardEvent| {
        if event.focus_within && plain(&event) && event.key == "ArrowLeft" {
          event.prevent_default();
          open.set(false);
        }
      });
    }
    panel
  }

  /// The line between two groups of items, with its room.
  ///
  /// The line has no width of its own: it is stretched to the room's, which
  /// the panel stretches to its own. A line that grew into the room there is
  /// would make the menu as wide as the window.
  pub fn separator(&self) -> Element {
    Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(MenuPart::Separator))
      .describe("a11y_role", "separator")
      .child(Row::new().part(&self.part(MenuPart::SeparatorLine)))
      .into()
  }

  /// The text over a group of items.
  pub fn label(&self, label: &str) -> Element {
    let part = self.part(MenuPart::Label);
    Row::new().part(&part).child(part.text(label).nowrap()).into()
  }

  /// An item as this menu draws it, in its theme: choosing it closes the
  /// menu, and the pointer on it closes the submenu of any other item.
  pub fn item(&self, item: MenuItem) -> Element {
    item.in_menu(self.closes(), self.submenus.clone()).build(self.theme())
  }

  /// How far the panel stands from its anchor: along the axis it opens on, and
  /// for a submenu back by the panel's padding, so that its first item is
  /// level with the item it hangs from.
  fn offset(&self) -> (f32, f32) {
    let beside = matches!(
      self.placement,
      Placement::LeftStart
        | Placement::Left
        | Placement::LeftEnd
        | Placement::RightStart
        | Placement::Right
        | Placement::RightEnd
    );
    let (mut x, mut y) = if beside { (ANCHOR_GAP, 0.0) } else { (0.0, ANCHOR_GAP) };
    if self.submenu {
      let (theme, panel) = (self.theme(), self.part(MenuPart::Panel).rest());
      let padding = |padding: Option<crate::node::SpacingValue>| {
        padding
          .map(|padding| padding.resolve(&theme.spacing(), 0.0))
          .unwrap_or(0.0)
      };
      if beside {
        // Out of the panel the item is in, then the gap; up by the padding over the first item.
        x += padding(panel.padding_x);
        y -= padding(panel.padding_y);
      } else {
        y += padding(panel.padding_y);
        x -= padding(panel.padding_x);
      }
    }
    (x, y)
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
    let (x, y) = self.offset();
    // The focus goes to the first item that takes it as the menu opens, and back to what had it as it closes.
    let mut popup = Popup::new(self.anchor, panel)
      .open(self.open)
      .placement(self.placement)
      .offset(x, y)
      .takes_focus(true);
    if self.submenu {
      // A press on the menu it hangs from closes the submenu and still reaches the item pressed there; the item
      // it hangs from opens it, and is no trigger of another dropdown.
      popup = popup
        .outside_press(OutsidePress::PassThrough)
        .switch_on_anchor_press(false);
    }
    popup.into()
  }
}

impl Themed for Menu {
  const NAME: &'static str = "menu";
  type Renderer = MenuRenderer;

  /// The design's menu (`pencil/lurq-design-system.pen`, Menu): a panel of the
  /// popover's color at least 230 wide, padded 4, with the large radius, an
  /// edge of 1 in the border's color and the medium shadow. A submenu is at
  /// least 160 wide. A separator is a line of 1 in the border's color with 4
  /// above and below; the label of a group is 11/600 in the muted foreground,
  /// 8 in from the side.
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
      .state(MenuState::Submenu, Panel, |s| s.min_width(160.0))
      .base(Separator, |s| s.padding_y(4.0))
      .base(SeparatorLine, |s| s.height(1.0).background(PaletteColor::Border))
      .base(Label, |s| {
        // The board has 6 above the text and 4 under it.
        s.padding_x(8.0)
          .padding_top(6.0)
          .padding_bottom(4.0)
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
  /// The mark at the end of an item that is on: a check, or a dot for one of several.
  Check,
  /// The chevron at the end of an item that opens a submenu.
  Chevron,
}

names!(MenuItemPart {
  Root => "root",
  Icon => "icon",
  Label => "label",
  Shortcut => "shortcut",
  Check => "check",
  Chevron => "chevron",
});

/// The states of a menu item a recipe styles, besides what the pointer and
/// the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MenuItemState {
  /// What it turns on is on, or it is the one chosen of several: it has a mark at its end.
  Checked,
  /// It is one of several of which one is chosen: its mark is a dot.
  Radio,
  /// It removes, or cannot be undone: it is in the destructive color.
  Destructive,
  /// It cannot be chosen.
  Disabled,
  /// Its submenu is open.
  Expanded,
}

names!(MenuItemState {
  Checked => "checked",
  Radio => "radio",
  Destructive => "destructive",
  Disabled => "disabled",
  Expanded => "expanded",
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
/// let actual = MenuItem::new("100%").radio(true);
/// let redo = MenuItem::new("Redo").shortcut("Ctrl+Y").disabled(true);
/// let delete = MenuItem::new("Delete").icon(icons::TRASH).destructive(true);
/// ```
///
/// It is chosen by a click, and by Enter or Space while it is focused. Under
/// the pointer or focused from the keyboard it is highlighted.
pub struct MenuItem {
  label: Arc<str>,
  icon: Option<Name>,
  shortcut: Option<Arc<str>>,
  checked: Option<bool>,
  radio: bool,
  destructive: bool,
  disabled: bool,
  submenu: Option<Menu>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_select: Vec<Arc<dyn Fn() + Send + Sync>>,
  closes: Vec<Signal<bool>>,
  submenus: Vec<Signal<bool>>,
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
      radio: false,
      destructive: false,
      disabled: false,
      submenu: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_select: Vec::new(),
      closes: Vec::new(),
      submenus: Vec::new(),
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
    self.radio = false;
    self
  }

  /// Whether the item is the one chosen of several that follow one another,
  /// as a zoom of 50%, 100% or 200%: a dot at its end. An item given this is
  /// a radio, chosen or not, to what reads the tree. The app says which is
  /// chosen, and is told by [`on_select`](Self::on_select).
  pub fn radio(mut self, chosen: bool) -> Self {
    self.checked = Some(chosen);
    self.radio = true;
    self
  }

  /// Whether the item removes, or cannot be undone: it is drawn in the destructive color.
  pub fn destructive(mut self, destructive: bool) -> Self {
    self.destructive = destructive;
    self
  }

  /// Whether the item cannot be chosen: it is dimmed, and the arrow keys pass it by.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// A menu the item opens beside it, with a chevron at its end: "Export as",
  /// and the formats beside it. The submenu hangs from this item, by the
  /// anchor it was made with; a press of the item and the pointer on it open
  /// it, and the pointer on another item of the menu closes it. Choosing an
  /// item of the submenu closes both menus. The item opens its submenu and is
  /// not chosen itself: it has no [`on_select`](Self::on_select).
  pub fn submenu(mut self, menu: Menu) -> Self {
    self.submenu = Some(menu);
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

  /// What the menu the item is in gives it: the signals choosing it sets to
  /// `false`, and the open signals of the submenus of the menu's items, which
  /// the pointer on it closes but for its own.
  pub(crate) fn in_menu(mut self, closes: Vec<Signal<bool>>, submenus: Vec<Signal<bool>>) -> Self {
    self.closes = closes;
    self.submenus = submenus;
    self
  }

  /// Builds the item in `theme`. Converting an item into an element does this
  /// in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let expanded = self.submenu.as_ref().is_some_and(|menu| menu.open.is_open());
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(MenuItemState::Checked, self.checked == Some(true))
    .state(MenuItemState::Radio, self.radio)
    .state(MenuItemState::Destructive, self.destructive)
    .state(MenuItemState::Disabled, self.disabled)
    .state(MenuItemState::Expanded, expanded);
    let view = MenuItemView {
      label: self.label,
      icon: self.icon,
      shortcut: self.shortcut,
      checked: self.checked,
      radio: self.radio,
      destructive: self.destructive,
      disabled: self.disabled,
      expanded,
      recipe: theme.resolve::<MenuItem>(&case, Some(&self.rules)),
      submenu: self.submenu,
      forced: self.forced,
      on_select: self.on_select,
      closes: self.closes,
      submenus: self.submenus,
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
  /// Whether it is on (checked, or the chosen radio), for an item that is on or off.
  pub checked: Option<bool>,
  /// Whether it is one of several of which one is chosen.
  pub radio: bool,
  pub destructive: bool,
  pub disabled: bool,
  /// Whether its submenu is open.
  pub expanded: bool,
  /// The style of each part of this item.
  pub recipe: ResolvedRecipe,
  submenu: Option<Menu>,
  forced: Option<Interaction>,
  on_select: Vec<Arc<dyn Fn() + Send + Sync>>,
  closes: Vec<Signal<bool>>,
  submenus: Vec<Signal<bool>>,
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

  /// Whether the item opens a submenu.
  pub fn has_submenu(&self) -> bool {
    self.submenu.is_some()
  }

  /// The open signal of the item's submenu, if it has one and the app gave it a signal.
  fn own_submenu(&self) -> Option<Signal<bool>> {
    self.submenu.as_ref()?.open.signal()
  }

  /// Closes the submenus of the other items of the menu, and opens this item's own.
  fn reach(&self) -> impl Fn() + Send + Sync + Clone + 'static {
    let own = self.own_submenu();
    let others: Vec<Signal<bool>> = self
      .submenus
      .iter()
      .filter(|signal| own.as_ref().is_none_or(|own| own.id() != signal.id()))
      .cloned()
      .collect();
    move || {
      for other in &others {
        if other.get_untracked() {
          other.set(false);
        }
      }
      if let Some(own) = &own
        && !own.get_untracked()
      {
        own.set(true);
      }
    }
  }

  /// What choosing the item does: its handlers are called, then its menu
  /// closes. An item with a submenu opens it instead.
  fn choose(&self) -> Option<Arc<dyn Fn() + Send + Sync>> {
    if self.disabled {
      return None;
    }
    if self.submenu.is_some() {
      return Some(Arc::new(self.reach()));
    }
    let (handlers, closes) = (self.on_select.clone(), self.closes.clone());
    (!handlers.is_empty() || !closes.is_empty()).then(|| {
      Arc::new(move || {
        for handler in &handlers {
          handler();
        }
        for open in &closes {
          open.set(false);
        }
      }) as Arc<dyn Fn() + Send + Sync>
    })
  }

  /// The item's behavior with the style of its root: the element that is
  /// pressed, focused and disabled, with nothing in it yet. It is a menu item
  /// to what reads the tree, called by its label. An item with a submenu is
  /// the submenu's anchor.
  pub fn root(&self) -> components::Button {
    let role = match self.checked {
      Some(_) if self.radio => "menuitemradio",
      Some(_) => "menuitemcheckbox",
      None => "menuitem",
    };
    let mut root = pressable(&self.part(MenuItemPart::Root), self.disabled)
      .justify(Justify::Start)
      .describe("a11y_role", role)
      .describe("a11y_name", self.label.clone())
      // Typing in the menu matches the label: an item's text is its shortcut too.
      .focus_label(self.label.clone());
    if let Some(checked) = self.checked {
      root = root.describe("a11y_checked", if checked { "true" } else { "false" });
    }
    if let Some(submenu) = &self.submenu {
      root = root
        .ref_element(submenu.anchor.clone())
        .describe("a11y_haspopup", "menu")
        .describe("a11y_expanded", if self.expanded { "true" } else { "false" });
    }
    if let Some(choose) = self.choose() {
      root = root.on_click(move |_| choose());
    }
    if !self.disabled && (self.submenu.is_some() || !self.submenus.is_empty()) {
      root = root.on_mouse_enter(self.reach());
    }
    // Right opens the submenu while the item has the focus, which goes to the submenu's first item.
    if !self.disabled
      && let Some(open) = self.own_submenu()
    {
      root = root.on_key_down(move |event: KeyboardEvent| {
        if event.focus_within && plain(&event) && event.key == "ArrowRight" {
          event.prevent_default();
          open.set(true);
        }
      });
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

  /// The item's text.
  pub fn label(&self) -> Element {
    let part = self.part(MenuItemPart::Label);
    Row::new().part(&part).child(part.text(&self.label).nowrap()).into()
  }

  /// The keys written at the end, if the item has them.
  pub fn shortcut(&self) -> Option<Element> {
    let shortcut = self.shortcut.as_ref()?;
    let part = self.part(MenuItemPart::Shortcut);
    Some(Row::new().part(&part).child(part.text(shortcut).nowrap()).into())
  }

  /// The mark at the end while the item is on: a check, or a dot for the chosen radio.
  pub fn check(&self) -> Option<Element> {
    let name = if self.radio { icons::DOT_CIRCLE } else { icons::CHECK };
    (self.checked == Some(true)).then(|| glyph(&self.part(MenuItemPart::Check), Name::new(name), false))
  }

  /// The chevron at the end of an item that opens a submenu.
  pub fn chevron(&self) -> Option<Element> {
    self.submenu.is_some().then(|| {
      glyph(
        &self.part(MenuItemPart::Chevron),
        Name::new(icons::CHEVRON_RIGHT),
        false,
      )
    })
  }

  /// The item's submenu, hung from the item, in the item's theme: choosing one
  /// of its items closes it and the menus the item is in.
  pub fn submenu(&mut self) -> Option<Element> {
    let menu = self.submenu.take()?;
    Some(menu.nested(self.closes.clone()).build(self.theme()))
  }

  /// How the default renderer builds an item: the root, with its icon and
  /// its label at its start and its shortcut, its mark and its chevron at its
  /// end; and after it, its submenu, which takes no room.
  ///
  /// The two ends are kept apart by the item's width, which the panel gives
  /// it, and not by a part that grows: an item is as wide as what it holds,
  /// and the panel as its widest item or its least width, never as the window.
  pub fn render(mut self) -> Element {
    // The ends hold their parts as far apart as the item holds the ends.
    let gap = self.part(MenuItemPart::Root).rest().gap.unwrap_or_default();
    let mut start = Row::new().spacing(gap.clone()).align_items(Alignment::Center);
    if let Some(icon) = self.icon() {
      start = start.child(icon);
    }
    start = start.child(self.label());
    let mut root = self.root().justify(Justify::SpaceBetween).child(start);
    let end: Vec<Element> = [self.shortcut(), self.check(), self.chevron()]
      .into_iter()
      .flatten()
      .collect();
    if !end.is_empty() {
      root = root.child(
        Row::new()
          .spacing(gap)
          .align_items(Alignment::Center)
          .with_children(end),
      );
    }
    match self.submenu() {
      Some(submenu) => Column::new()
        .align_items(Alignment::Stretch)
        .child(root)
        .child(submenu)
        .into(),
      None => root.into(),
    }
  }
}

impl Themed for MenuItem {
  const NAME: &'static str = "menu_item";
  type Renderer = MenuItemRenderer;

  /// The design's menu item (`pencil/lurq-design-system.pen`, Menu Item): 32
  /// tall, padded 8, with the small radius and 8 between what it holds; its
  /// label in the body's type, an icon of 16 in the muted foreground before
  /// it, and at its end the shortcut in the mono type at 11, muted, the check
  /// (or the dot of a radio) and the chevron of a submenu, muted. Under the
  /// pointer, focused from the keyboard or with its submenu open it is filled
  /// with the highlight. A destructive item is in the destructive color, icon
  /// and all; a disabled one is dimmed.
  fn recipe() -> Recipe {
    use MenuItemPart::{Check, Chevron, Icon, Label, Root, Shortcut};
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
      .base(Chevron, |s| s.square(IconSize::Md).foreground(MutedForeground))
      .hovered(Root, |s| s.background(Highlight))
      .focus_visible(Root, |s| s.background(Highlight))
      .state(MenuItemState::Expanded, Root, |s| s.background(Highlight))
      .state(MenuItemState::Destructive, Root, |s| s.foreground(Destructive))
      .state(MenuItemState::Destructive, Icon, |s| s.foreground(Destructive))
      .state(MenuItemState::Disabled, Root, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
  }
}
