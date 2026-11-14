//! A context menu: a menu of what can be done with an element, opened where the element was pressed with the
//! secondary button or held with a finger.

use std::{
  cell::RefCell,
  collections::HashMap,
  sync::{Arc, LazyLock, Mutex, Weak},
};

use super::{
  menu::{Menu, MenuEntry, MenuItem, MenuItemPart, MenuPart},
  names,
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  separator::Separator,
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    events::{KeyboardEvent, LongPressEvent, MouseButton, MouseEvent},
    theme::{BorderSize, PaletteColor, RadiusSize, ShadowStyle, Theme},
  },
  components::{Popup, Row},
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::LayoutKind},
  node::{Element, FocusGroup, Node, NodeUpdate, border::Border},
};

/// The parts of a context menu a recipe styles, laid over the parts of the [`Menu`] it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ContextMenuPart {
  /// The panel, laid over a menu's panel.
  Panel,
  /// The panel of a context menu laid out as a bar ([`ContextMenu::bar`]): one row of commands.
  Bar,
  /// An item of the bar, laid over a menu item's root.
  BarItem,
  /// The label of an item of the bar, laid over a menu item's label.
  BarLabel,
}

names!(ContextMenuPart {
  Panel => "panel",
  Bar => "bar",
  BarItem => "bar_item",
  BarLabel => "bar_label",
});

/// The states of a context menu a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ContextMenuState {
  /// Its commands are in a row: the board's edit menu, Cut, Copy, Paste.
  Bar,
}

names!(ContextMenuState {
  Bar => "bar",
});

/// What builds a context menu's nodes: a theme's in place of the default one.
pub type ContextMenuRenderer = Arc<dyn Fn(ContextMenuView) -> Element + Send + Sync>;

/// What a context menu keeps between builds: the point it was asked for at, and its anchor where its element has
/// none. One for each open signal a context menu is given, while its element is in the tree: `alive` is held by the
/// element's handlers.
struct Kept {
  point: Signal<Option<(f32, f32)>>,
  anchor: ElementRef,
  alive: Weak<()>,
}

static KEPT: LazyLock<Mutex<HashMap<usize, Kept>>> = LazyLock::new(Default::default);

fn kept(open: usize) -> (Signal<Option<(f32, f32)>>, ElementRef, Arc<()>) {
  let mut all = KEPT.lock().unwrap();
  all.retain(|_, kept| kept.alive.strong_count() > 0);
  if let Some(kept) = all.get(&open)
    && let Some(alive) = kept.alive.upgrade()
  {
    return (kept.point.clone(), kept.anchor.clone(), alive);
  }
  let (point, anchor, alive) = (Signal::new(None), ElementRef::new(), Arc::new(()));
  all.insert(
    open,
    Kept {
      point: point.clone(),
      anchor: anchor.clone(),
      alive: Arc::downgrade(&alive),
    },
  );
  (point, anchor, alive)
}

thread_local! {
  /// The open signal of the context menu opened last: one context menu is open at a time.
  static OPEN: RefCell<Option<Signal<bool>>> = const { RefCell::new(None) };
}

/// Opens the context menu of `open`, and closes the one that was open before it.
fn take_turn(open: &Signal<bool>) {
  let before = OPEN.with(|last| last.borrow_mut().replace(open.clone()));
  if let Some(before) = before
    && before.id() != open.id()
    && before.get_untracked()
  {
    before.set(false);
  }
  open.set(true);
}

/// A context menu: a [`Menu`] of what can be done with an element, which opens where the element is pressed with the
/// secondary button, or held with a finger. The actions of a file in a list, of a node of a canvas, of selected text.
///
/// ```
/// use lurq::{
///   components::Text,
///   core::Signal,
///   ui::{ContextMenu, MenuItem, icons},
/// };
///
/// let open = Signal::new(false);
/// let file = ContextMenu::new(Text::new("report.pdf"), open)
///   .item(MenuItem::new("Open").icon(icons::FILE))
///   .item(MenuItem::new("Rename").icon(icons::PENCIL).shortcut("F2"))
///   .separator()
///   .item(MenuItem::new("Delete").icon(icons::TRASH).destructive(true));
/// ```
///
/// It is built of what a menu is built of: the same [`MenuItem`]s, with their icons, shortcuts, checks and submenus,
/// separators and labels. It wraps the element it is for, which lays out as it would without it, and opens:
///
/// - at the pointer, on a press of the secondary button on the element (and on Control and a press on macOS);
/// - at the finger, on a long press of the element;
/// - at the element, under its start, on the Menu key or Shift+F10 while the focus is in the element.
///
/// It stays inside the window, opening up or back from the point where there is no room down or toward the end. As
/// it opens the focus goes to its first item, and as it closes back to what had it. It closes as a menu does: an item
/// chosen, a press outside it, Escape. One context menu is open at a time. A context menu does not keep whether it is
/// open: the app's signal does, which it sets.
pub struct ContextMenu {
  target: Element,
  open: Signal<bool>,
  entries: Vec<MenuEntry>,
  bar: bool,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ContextMenu {
  /// The context menu of `target`, the element it is for, open while `open` is: the app's signal, which the presses
  /// and keys that ask for the menu set to `true`, and choosing an item, a press outside and Escape to `false`.
  pub fn new(target: impl Into<Element>, open: Signal<bool>) -> Self {
    Self {
      target: target.into(),
      open,
      entries: Vec::new(),
      bar: false,
      disabled: false,
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

  /// A text over the items after it, which says what they are about.
  pub fn label(mut self, label: &str) -> Self {
    self.entries.push(MenuEntry::Label(Arc::from(label)));
    self
  }

  /// Whether the commands are in one row, as the board's edit menu over selected text has them: Cut, Copy, Paste,
  /// Select All. The board's items there are labels alone; a separator is a line down the row, and a label of a
  /// group is left out. Left and Right move the focus between them.
  pub fn bar(mut self, bar: bool) -> Self {
    self.bar = bar;
    self
  }

  /// Whether the element has no context menu now: it is as it would be without one.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Puts the context menu in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this context menu over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this context menu.
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

  /// Builds the context menu in `theme`, around its element. Converting a context menu into an element does this in
  /// the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    if self.disabled {
      return self.target;
    }
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(ContextMenuState::Bar, self.bar);
    let (point, kept_anchor, alive) = kept(self.open.id());
    let target = holds_children(self.target);
    // An element that has an anchor of its own keeps it, and the menu hangs from it.
    let anchor = target.node.element_ref.clone().unwrap_or(kept_anchor);
    let view = ContextMenuView {
      entries: self.entries,
      bar: self.bar,
      point: if self.open.get() { point.get() } else { None },
      recipe: theme.resolve::<ContextMenu>(&case, Some(&self.rules)),
      target: Some(target),
      anchor,
      open: self.open,
      asked_at: point,
      alive,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<ContextMenu>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<ContextMenu> for Element {
  fn from(menu: ContextMenu) -> Self {
    menu.build(&current_theme())
  }
}

/// `element`, in a row of its own where its root lays out no children (a text, a wrapper of one child): what declares
/// the menu's layer must be a container, or the layer is never laid out.
fn holds_children(element: Element) -> Element {
  match element.node.layout_kind() {
    LayoutKind::Row { .. } | LayoutKind::Column { .. } | LayoutKind::Stack { .. } | LayoutKind::Grid(_) => element,
    _ => Row::new().child(element).into(),
  }
}

/// A context menu as a renderer sees it: what it holds, where it was asked for, the style of its parts, and what
/// opens it ready to be put on its element.
pub struct ContextMenuView {
  /// What the menu holds, in order.
  pub entries: Vec<MenuEntry>,
  /// Whether its commands are in one row.
  pub bar: bool,
  /// The point of the window it stands at, where a press or a finger asked for it; `None` where the keyboard did, and
  /// it stands at its element.
  pub point: Option<(f32, f32)>,
  /// The style of each part of this context menu.
  pub recipe: ResolvedRecipe,
  target: Option<Element>,
  anchor: ElementRef,
  open: Signal<bool>,
  asked_at: Signal<Option<(f32, f32)>>,
  alive: Arc<()>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ContextMenuView {
  /// The style of a part, as this context menu is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The signal that holds whether the menu is open.
  pub fn open(&self) -> &Signal<bool> {
    &self.open
  }

  /// What asks for the menu at `point`, or at its element for `None`.
  fn opener(&self) -> impl Fn(Option<(f32, f32)>) + Send + Sync + Clone + 'static {
    let (open, asked_at, alive) = (self.open.clone(), self.asked_at.clone(), self.alive.clone());
    move |point| {
      // Held while the element is: the point is kept for the menu while it is in the tree.
      let _ = &alive;
      asked_at.set(point);
      take_turn(&open);
    }
  }

  /// The menu as a [`Menu`] would draw it, in a column at the point it was asked for.
  fn menu(&mut self) -> Element {
    let mut menu = Menu::new(self.anchor.clone(), self.open.clone())
      .context(self.point)
      .rules(&self.recipe.rules_for(ContextMenuPart::Panel, MenuPart::Panel));
    for entry in std::mem::take(&mut self.entries) {
      menu = match entry {
        MenuEntry::Item(item) => menu.item(item),
        MenuEntry::Separator => menu.separator(),
        MenuEntry::Label(label) => menu.label(&label),
      };
    }
    if let Some(id) = &self.id {
      menu = menu.id(id.clone());
    }
    if let Some(key) = &self.key {
      menu = menu.key(key.clone());
    }
    menu.build(self.theme())
  }

  /// The menu with its commands in a row, at the point it was asked for: a menu's items in the style of the bar's.
  /// Left and Right move the focus between them; choosing one closes the bar.
  fn bar(&mut self) -> Element {
    let item_rules = self
      .recipe
      .rules_for(ContextMenuPart::BarItem, MenuItemPart::Root)
      .extend(&self.recipe.rules_for(ContextMenuPart::BarLabel, MenuItemPart::Label));
    let mut panel = Row::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(ContextMenuPart::Bar))
      .focus_group(FocusGroup::horizontal().one_stop(true).follow_pointer(true))
      .describe("a11y_role", "menu")
      .describe("a11y_orientation", "horizontal");
    if let Some(id) = &self.id {
      panel = panel.id(id.clone());
    }
    for entry in std::mem::take(&mut self.entries) {
      panel = match entry {
        MenuEntry::Item(item) => panel.child(
          item
            .rules(&item_rules)
            .in_menu(vec![self.open.clone()], Vec::new())
            .build(self.theme()),
        ),
        MenuEntry::Separator => panel.child(Separator::vertical().build(self.theme())),
        MenuEntry::Label(_) => panel,
      };
    }
    if let Some(key) = &self.key {
      panel = panel.key(key.clone());
    }
    // The focus goes to the first item as the bar opens, and back to what had it as it closes.
    Popup::new(self.anchor.clone(), panel)
      .open(self.open.clone())
      .at(self.point)
      .switch_on_anchor_press(false)
      .takes_focus(true)
      .into()
  }

  /// What opens the menu (`open`), put on `node`, the root of its element, which is `anchor`, with `menu`, its layer,
  /// declared in it:
  ///
  /// - a press of the secondary button, and on macOS Control and a press, at the pointer;
  /// - a long press, at the finger;
  /// - the Menu key, and Shift+F10, while the focus is in the element, at the element.
  ///
  /// A press asks for the menu of the innermost element that has one.
  fn dress(
    node: Node,
    anchor: &ElementRef,
    open: impl Fn(Option<(f32, f32)>) + Send + Sync + Clone + 'static,
    menu: &Mutex<Element>,
  ) -> Node {
    let mut node = node.ref_element(anchor.clone());
    {
      let open = open.clone();
      NodeUpdate::on_mouse_click(&mut node, MouseButton::Right, move |event: MouseEvent| {
        event.stop_propagation();
        open(Some((event.x, event.y)));
      });
    }
    if cfg!(target_os = "macos") {
      let open = open.clone();
      NodeUpdate::on_click(&mut node, move |event: MouseEvent| {
        if event.ctrl && !event.from_keyboard {
          event.stop_propagation();
          event.prevent_default();
          open(Some((event.x, event.y)));
        }
      });
    }
    {
      let open = open.clone();
      NodeUpdate::on_long_press(&mut node, move |event: LongPressEvent| {
        event.stop_propagation();
        open(Some((event.x, event.y)));
      });
    }
    NodeUpdate::on_key_down(&mut node, move |event: KeyboardEvent| {
      let asked = event.key == "ContextMenu" || (event.shift && event.key == "F10");
      if event.focus_within && asked && !event.ctrl && !event.alt && !event.meta {
        event.prevent_default();
        open(None);
      }
    });
    NodeUpdate::describe(&mut node, Arc::from("a11y_haspopup"), Arc::from("menu"));
    // The layer is declared in the element: a layer is found among the children of a container.
    NodeUpdate::child(&mut node, menu.lock().unwrap().clone().into_node());
    node
  }

  /// How the default renderer builds a context menu: the element, with what opens the menu on it and the menu
  /// declared in it, a column of items or a bar.
  pub fn render(mut self) -> Element {
    let menu = if self.bar { self.bar() } else { self.menu() };
    // An element is not shared between threads as it is: the decoration, which is, holds it in a lock.
    let menu = Arc::new(Mutex::new(menu));
    let target = self.target.take().unwrap_or_else(Element::new);
    let (anchor, open) = (self.anchor.clone(), self.opener());
    target.decorate_root(move |node| Self::dress(node, &anchor, open.clone(), &menu))
  }
}

impl Themed for ContextMenu {
  const NAME: &'static str = "context_menu";
  type Renderer = ContextMenuRenderer;

  /// The design's context menu (`pencil/lurq-design-system.pen`, Context Menu): a menu at least 220 wide. Its bar
  /// (Edit Menu) is a row in the popover's color, padded 3, with the large radius, an edge of the border's color and
  /// the large shadow; its items are 28 high, padded 10, their labels 13.
  fn recipe() -> Recipe {
    use ContextMenuPart::{Bar, BarItem, BarLabel, Panel};
    use PaletteColor::{Popover, PopoverForeground};
    Recipe::new()
      .base(Panel, |s| s.min_width(220.0))
      .base(Bar, |s| {
        s.padding_x(3.0)
          .padding_y(3.0)
          .radius(RadiusSize::Lg)
          .background(Popover)
          .foreground(PopoverForeground)
          .border(Border::inside(BorderSize::Sm, PaletteColor::Border))
          .shadow(ShadowStyle::Lg)
      })
      .base(BarItem, |s| s.height(28.0).padding_x(10.0))
      .base(BarLabel, |s| s.font_size(13.0))
  }
}
