//! A menubar: the menus of an app in a row, File, Edit, View, each opened from its title.

use std::{
  collections::HashMap,
  sync::{
    Arc, LazyLock, Mutex, Weak,
    atomic::{AtomicBool, Ordering},
  },
};

use super::{
  menu::{Menu, MenuEntry},
  names,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{fading, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    events::KeyboardEvent,
    theme::{BorderSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{self, Column, Row},
  core::{ElementRef, Signal, WatchHandle},
  layout::{
    Alignment,
    text_style::{FontWeight, TextUnderline},
  },
  node::{CursorIcon, Element, FocusGroup, border::Border},
};

/// Whether a key was pressed with no modifier held but Shift.
fn plain(event: &KeyboardEvent) -> bool {
  !event.ctrl && !event.alt && !event.meta
}

/// The parts of a menubar a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MenubarPart {
  /// The bar that holds the titles.
  Bar,
  /// A title: what is pressed to open its menu, and what is lit while the menu is open.
  Title,
  /// The text of a title.
  Label,
}

names!(MenubarPart {
  Bar => "bar",
  Title => "title",
  Label => "label",
});

/// The states of a menubar a recipe styles. A title is resolved in the states of the bar and its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MenubarState {
  /// The title's menu is open.
  Open,
  /// The bar, or the title, cannot be opened.
  Disabled,
}

names!(MenubarState {
  Open => "open",
  Disabled => "disabled",
});

/// What builds a menubar's nodes: a theme's in place of the default one.
pub type MenubarRenderer = Arc<dyn Fn(MenubarView) -> Element + Send + Sync>;

/// One menu of a menubar: its title, what it holds, and whether it can be opened.
pub struct MenubarMenu {
  pub title: Arc<str>,
  /// What the menu holds, in order.
  pub entries: Vec<MenuEntry>,
  pub disabled: bool,
}

/// A menubar: the menus of an app in a row, File, Edit, View, each opened from its title, as the title bar of a
/// window on Windows or Linux has them.
///
/// ```
/// use lurq::{
///   core::Signal,
///   ui::{MenuEntry, MenuItem, Menubar},
/// };
///
/// let open: Signal<Option<usize>> = Signal::new(None);
/// let bar = Menubar::new(open)
///   .menu(
///     "File",
///     [
///       MenuItem::new("New").shortcut("Ctrl+N").on_select(|| println!("new")).into(),
///       MenuItem::new("Open…").shortcut("Ctrl+O").into(),
///       MenuEntry::Separator,
///       MenuItem::new("Quit").shortcut("Ctrl+Q").into(),
///     ],
///   )
///   .menu("Edit", [MenuItem::new("Undo").shortcut("Ctrl+Z"), MenuItem::new("Redo").disabled(true)])
///   .menu("View", [MenuItem::new("Show grid").checked(true)]);
/// ```
///
/// Its menus are [`Menu`]s, of the same [`MenuItem`](super::MenuItem)s: submenus, checks, radio items, shortcuts,
/// disabled items, separators and labels. Which of them is open is the app's: the `Signal<Option<usize>>` it gives the
/// bar, which a press of a title sets and choosing an item, a press outside and Escape set to `None`. One is open at a
/// time.
///
/// The bar is one stop of Tab. Left and Right move between the titles, a letter goes to the title that starts with
/// it, and Down, Enter and Space open the focused title's menu at its first item, Up at its last. While a menu is
/// open, Left and Right go to the menu beside it (Left first closes a submenu that has the focus, and Right opens
/// the submenu of the item it is on, if it has one), and the pointer coming onto another title opens that one.
/// Escape closes the menu and gives the focus back to its title. Alt pressed alone puts the focus on the first
/// title and underlines the first letter of each, and Alt again gives the focus back; Alt with that letter opens
/// the title's menu from wherever the focus is.
pub struct Menubar {
  open: Signal<Option<usize>>,
  menus: Vec<MenubarMenu>,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Menubar {
  /// A menubar whose open menu is `open`'s: the index of the menu among the bar's, `None` while all are closed.
  pub fn new(open: Signal<Option<usize>>) -> Self {
    Self {
      open,
      menus: Vec::new(),
      disabled: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// A menu called `title`, after the bar's others, holding `entries`: [`MenuItem`](super::MenuItem)s, and
  /// [`MenuEntry::Separator`] and [`MenuEntry::Label`] between them.
  pub fn menu(mut self, title: &str, entries: impl IntoIterator<Item = impl Into<MenuEntry>>) -> Self {
    self.menus.push(MenubarMenu {
      title: Arc::from(title),
      entries: entries.into_iter().map(Into::into).collect(),
      disabled: false,
    });
    self
  }

  /// Whether the menu called `title` cannot be opened: its title is dimmed, and the arrows pass it by.
  pub fn title_disabled(mut self, title: &str, disabled: bool) -> Self {
    for menu in self.menus.iter_mut().filter(|menu| &*menu.title == title) {
      menu.disabled = disabled;
    }
    self
  }

  /// Whether no menu of the bar can be opened.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
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
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this bar.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the titles as they are under the pointer, held down or focused from the keyboard, with nothing pointing
  /// at them: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the bar. Its titles are `{id}-0`, `{id}-1` and on, and the panel of each menu `{id}-0-menu`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the bar in `theme`. Converting a bar into an element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(MenubarState::Disabled, self.disabled);
    let kept = kept(&self.open, self.menus.len());
    let open = self.open.get();
    let mnemonics =
      !self.disabled && (kept.held.get() || (kept.stay.get() && (kept.bar.focus_visible_within() || open.is_some())));
    let view = MenubarView {
      menus: self.menus,
      disabled: self.disabled,
      open: open.filter(|_| !self.disabled),
      mnemonics,
      recipe: theme.resolve::<Menubar>(&case, Some(&self.rules)),
      case,
      rules: self.rules,
      forced: self.forced,
      kept,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Menubar>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Menubar> for Element {
  fn from(bar: Menubar) -> Self {
    bar.build(&current_theme())
  }
}

/// What a menubar keeps between builds, for the open signal it was given, while it is in the tree (`alive` is held
/// by its handlers): an open signal for each menu, which a [`Menu`] takes, kept as one with the app's, and the
/// anchor each menu hangs from; whether the menu opens at its last item; whether Alt shows the titles' letters; and
/// the bar, whose focus they follow.
struct Kept {
  state: State,
  /// What keeps each menu's signal and the app's one: the app's open menu follows a menu closing itself, and each
  /// menu the app's.
  _follows: Vec<WatchHandle<bool>>,
  _leads: WatchHandle<Option<usize>>,
  alive: Weak<()>,
}

/// What a bar keeps, as its builds and its handlers share it.
#[derive(Clone)]
struct State {
  open: Signal<Option<usize>>,
  menus: Arc<Mutex<Vec<(Signal<bool>, ElementRef)>>>,
  /// The menu opens with the focus on its last item: it was opened with Up.
  from_end: Arc<AtomicBool>,
  /// Alt is down.
  held: Signal<bool>,
  /// Alt went down and up alone: the letters show while the bar or one of its menus has the keyboard.
  stay: Signal<bool>,
  /// Alt is down and nothing else was pressed since.
  alone: Arc<AtomicBool>,
  bar: ElementRef,
}

/// What a bar keeps, with what keeps it: the handlers of the bar built hold `alive`.
#[derive(Clone)]
struct Shared {
  state: State,
  alive: Arc<()>,
}

impl std::ops::Deref for Shared {
  type Target = State;

  fn deref(&self) -> &State {
    &self.state
  }
}

impl State {
  /// The open signal and the anchor of the menu `index`.
  fn menu(&self, index: usize) -> Option<(Signal<bool>, ElementRef)> {
    self.menus.lock().unwrap().get(index).cloned()
  }

  /// Opens the menu `index`, at its last item where `from_end`.
  fn open_at(&self, index: usize, from_end: bool) {
    self.from_end.store(from_end, Ordering::Relaxed);
    if self.open.get_untracked() != Some(index) {
      self.open.set(Some(index));
    }
  }
}

static KEPT: LazyLock<Mutex<HashMap<usize, Kept>>> = LazyLock::new(Default::default);

/// What the bar of `open` keeps, with a signal and an anchor for each of its `count` menus.
fn kept(open: &Signal<Option<usize>>, count: usize) -> Shared {
  let mut all = KEPT.lock().unwrap();
  all.retain(|_, kept| kept.alive.strong_count() > 0);
  let held = all
    .get(&open.id())
    .and_then(|kept| Some((kept.alive.upgrade()?, kept.state.clone())));
  let (alive, state) = match held {
    Some(held) => held,
    None => {
      let state = State {
        open: open.clone(),
        menus: Arc::new(Mutex::new(Vec::new())),
        from_end: Arc::new(AtomicBool::new(false)),
        held: Signal::new(false),
        stay: Signal::new(false),
        alone: Arc::new(AtomicBool::new(false)),
        bar: ElementRef::new(),
      };
      // Each menu is open while the app's signal names it.
      let (menus, from_end, app) = (state.menus.clone(), state.from_end.clone(), open.clone());
      let leads = open.watch(move || {
        let now = app.get_untracked();
        if now.is_none() {
          from_end.store(false, Ordering::Relaxed);
        }
        let menus = menus.lock().unwrap().clone();
        for (index, (menu, _)) in menus.iter().enumerate() {
          let want = now == Some(index);
          if menu.get_untracked() != want {
            menu.set(want);
          }
        }
      });
      let alive = Arc::new(());
      all.insert(
        open.id(),
        Kept {
          state: state.clone(),
          _follows: Vec::new(),
          _leads: leads,
          alive: Arc::downgrade(&alive),
        },
      );
      (alive, state)
    }
  };
  let kept = all.get_mut(&open.id()).expect("kept above");
  let mut menus = state.menus.lock().unwrap();
  while menus.len() < count {
    let index = menus.len();
    let menu = Signal::new(open.get_untracked() == Some(index));
    // A menu that closes itself (an item chosen, Escape, a press outside) closes the app's.
    let (own, app) = (menu.clone(), open.clone());
    kept._follows.push(menu.watch(move || {
      let (on, now) = (own.get_untracked(), app.get_untracked());
      if on && now != Some(index) {
        app.set(Some(index));
      } else if !on && now == Some(index) {
        app.set(None);
      }
    }));
    menus.push((menu, ElementRef::new()));
  }
  drop(menus);
  Shared { state, alive }
}

/// A menubar as a renderer sees it: its menus, the style of its parts, and its behavior ready to be dressed.
pub struct MenubarView {
  /// The bar's menus, in order. [`menu`](Self::menu) takes what one holds.
  pub menus: Vec<MenubarMenu>,
  /// Whether no menu of the bar can be opened.
  pub disabled: bool,
  /// The menu open now.
  pub open: Option<usize>,
  /// Whether the first letter of each title is underlined: Alt is down, or put the focus on the bar.
  pub mnemonics: bool,
  /// The style of each part of the bar, in the bar's states.
  pub recipe: ResolvedRecipe,
  case: Case,
  rules: Recipe,
  forced: Option<Interaction>,
  kept: Shared,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl MenubarView {
  /// The style of a part of the bar.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// Whether the menu `index` cannot be opened.
  pub fn is_disabled(&self, index: usize) -> bool {
    self.disabled || self.menus.get(index).is_some_and(|menu| menu.disabled)
  }

  /// The style of a part of the title `index`, in its states: open, disabled.
  pub fn title_part(&self, index: usize, part: impl Into<Name>) -> ResolvedPart {
    let case = self
      .case
      .clone()
      .state(MenubarState::Open, self.open == Some(index))
      .state(MenubarState::Disabled, self.is_disabled(index));
    self
      .theme()
      .resolve::<Menubar>(&case, Some(&self.rules))
      .part(part)
      .forced(self.forced)
  }

  /// The anchor the menu `index` hangs from: its title. It is kept with the bar, so that the runtime knows the
  /// menu for the one it was in the build before.
  fn anchor(&self, index: usize) -> ElementRef {
    self.kept.menu(index).map_or_else(ElementRef::new, |(_, anchor)| anchor)
  }

  /// The bar in the style of its part, with nothing in it yet: a menubar to what reads the tree, one stop of Tab
  /// that the arrows move in, and a row of menus to the runtime, which hands an open menu to its neighbour with
  /// Left and Right and takes Alt. It notes Alt going down and up, for the titles' letters.
  pub fn bar(&self) -> Row {
    let mut bar = Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(MenubarPart::Bar))
      .ref_element(self.kept.bar.clone())
      .focus_group(
        FocusGroup::horizontal()
          .one_stop(true)
          .typeahead(true)
          .follow_pointer(true)
          .menu_row(true),
      )
      .describe("a11y_role", "menubar");
    // The handlers hold what the bar keeps while it is in the tree, disabled or not.
    {
      let (down, up) = (self.kept.clone(), self.kept.clone());
      bar = bar
        .on_key_down(move |event: KeyboardEvent| {
          let kept = &down;
          if event.key == "Alt" && !event.ctrl && !event.meta {
            kept.alone.store(true, Ordering::Relaxed);
            if !kept.held.get_untracked() {
              kept.held.set(true);
            }
          } else {
            kept.alone.store(false, Ordering::Relaxed);
            // A key with the keyboard away from the bar and its menus: the letters go.
            if !event.focus_within && kept.open.get_untracked().is_none() && kept.stay.get_untracked() {
              kept.stay.set(false);
            }
          }
        })
        .on_key_up(move |event: KeyboardEvent| {
          let kept = &up;
          if event.key != "Alt" {
            return;
          }
          if kept.held.get_untracked() {
            kept.held.set(false);
          }
          if kept.alone.swap(false, Ordering::Relaxed) && !kept.stay.get_untracked() {
            kept.stay.set(true);
          }
        });
    }
    if let Some(id) = &self.id {
      bar = bar.id(id.clone());
    }
    if let Some(key) = &self.key {
      bar = bar.key(key.clone());
    }
    bar
  }

  /// The title `index`'s behavior with the style of its part, with nothing in it yet: a menu item that opens a
  /// menu to what reads the tree, and the anchor its menu hangs from. A press opens its menu or closes it; the
  /// pointer coming onto it while another menu is open opens its own; Down opens it at its first item, Up at its
  /// last.
  pub fn title(&self, index: usize) -> components::Button {
    let Some(menu) = self.menus.get(index) else {
      return components::Button::empty();
    };
    let disabled = self.is_disabled(index);
    let open = self.open == Some(index);
    let mut title = pressable(&self.title_part(index, MenubarPart::Title), disabled)
      .ref_element(self.anchor(index))
      .describe("a11y_role", "menuitem")
      .describe("a11y_name", menu.title.clone())
      .describe("a11y_haspopup", "menu")
      .describe("a11y_expanded", if open { "true" } else { "false" })
      // A letter typed on the bar goes to the title that starts with it.
      .focus_label(menu.title.clone());
    if !disabled {
      let (press, enter, keys) = (self.kept.clone(), self.kept.clone(), self.kept.clone());
      title = title
        .on_click(move |_| {
          let _alive = &press.alive;
          if press.open.get_untracked() == Some(index) {
            press.open.set(None);
          } else {
            press.open_at(index, false);
          }
        })
        .on_mouse_enter(move || {
          if enter.open.get_untracked().is_some_and(|open| open != index) {
            enter.open_at(index, false);
          }
        })
        .on_key_down(move |event: KeyboardEvent| {
          if !event.focus_within || !plain(&event) {
            return;
          }
          let from_end = match event.key.as_str() {
            "ArrowDown" => false,
            "ArrowUp" => true,
            _ => return,
          };
          event.prevent_default();
          keys.open_at(index, from_end);
        });
    }
    if let Some(id) = &self.id {
      title = title.id(format!("{id}-{index}"));
    }
    title
  }

  /// The text of the title `index`, its first letter underlined while [`mnemonics`](Self::mnemonics) show.
  pub fn label(&self, index: usize) -> Element {
    let part = self.title_part(index, MenubarPart::Label);
    let title = self.menus.get(index).map(|menu| menu.title.clone()).unwrap_or_default();
    let mut letters = title.chars();
    let first = letters.next();
    let row = Row::new().part(&part);
    match first {
      Some(first) if self.mnemonics && !self.is_disabled(index) => row
        .child(
          part
            .text(&first.to_string())
            .nowrap()
            .underline(TextUnderline::default()),
        )
        .child(part.text(letters.as_str()).nowrap())
        .into(),
      _ => row.child(part.text(&title).nowrap()).into(),
    }
  }

  /// The menu `index`, hung from its title, open while the bar's open menu is it. It takes what the menu holds out
  /// of the view: it is built once.
  pub fn menu(&mut self, index: usize) -> Element {
    let Some((signal, anchor)) = self.kept.menu(index) else {
      return Row::new().into();
    };
    let Some(menu) = self.menus.get_mut(index) else {
      return Row::new().into();
    };
    let entries = std::mem::take(&mut menu.entries);
    // Opened with Up, the focus goes to its last item that can be chosen.
    let last = (self.open == Some(index) && self.kept.from_end.load(Ordering::Relaxed))
      .then(|| {
        entries
          .iter()
          .rposition(|entry| matches!(entry, MenuEntry::Item(item) if !item.is_disabled()))
      })
      .flatten();
    let mut built = Menu::new(anchor, signal);
    for (at, entry) in entries.into_iter().enumerate() {
      built = match entry {
        MenuEntry::Item(item) => built.item(item.autofocus(Some(at) == last)),
        MenuEntry::Separator => built.separator(),
        MenuEntry::Label(label) => built.label(&label),
      };
    }
    if let Some(id) = &self.id {
      built = built.id(format!("{id}-{index}-menu"));
    }
    built.build(self.theme())
  }

  /// How the default renderer builds a bar: its titles in a row, each with its label and its menu, which takes no
  /// room, after it.
  pub fn render(mut self) -> Element {
    let mut bar = self.bar();
    for index in 0..self.menus.len() {
      let title = self.title(index).child(self.label(index));
      let menu = self.menu(index);
      bar = bar.child(Column::new().child(title).child(menu));
    }
    bar.into()
  }
}

impl Themed for Menubar {
  const NAME: &'static str = "menubar";
  type Renderer = MenubarRenderer;

  /// The design's menubar (`pencil/lurq-design-system.pen`, Menubar): a bar 36 tall of the background's color,
  /// padded 4, with the medium radius and an edge of 1 in the border's color, its titles 2 apart. A title is 28
  /// tall, padded 10 at the sides, with the small radius; its text is 13/500 in the foreground. The title whose menu
  /// is open is filled with the highlight (the board's accent), as one under the pointer or focused from the
  /// keyboard is; a disabled one is dimmed.
  fn recipe() -> Recipe {
    use MenubarPart::{Bar, Label, Title};
    use PaletteColor::{Background, Foreground, Highlight};
    Recipe::new()
      .base(Bar, |s| {
        s.height(36.0)
          .padding_x(4.0)
          .padding_y(4.0)
          .gap(2.0)
          .radius(RadiusSize::Md)
          .background(Background)
          .border(Border::inside(BorderSize::Sm, PaletteColor::Border))
      })
      .base(Title, |s| {
        fading(
          s.height(28.0)
            .padding_x(10.0)
            .radius(RadiusSize::Sm)
            .cursor(CursorIcon::Pointer),
        )
      })
      .base(Label, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .weight(FontWeight::Medium)
          .foreground(Foreground)
      })
      .hovered(Title, |s| s.background(Highlight))
      .focus_visible(Title, |s| s.background(Highlight))
      .state(MenubarState::Open, Title, |s| s.background(Highlight))
      .state(MenubarState::Disabled, Title, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
  }
}
