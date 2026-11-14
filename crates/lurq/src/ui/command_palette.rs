//! A command palette: a search over the window that finds a command or a place by what is typed.

use std::{
  collections::HashMap,
  sync::{Arc, LazyLock, Mutex, Weak},
};

use super::{
  input::{Input, InputPart},
  kbd::{Kbd, KbdPart, KbdVariant, KeyPlatform},
  menu::{MenuItem, MenuItemPart, MenuItemState},
  names,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  separator::Separator,
  shared::glyph,
  spinner::Spinner,
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    events::{KeyboardEvent, MouseEvent, TextInputEvent},
    theme::{BorderSize, IconSize, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle},
  },
  components::{Column, Modal, Root, Row, ScrollVertical},
  core::{ElementRef, Signal},
  layout::{
    Alignment,
    layout_kind::{Justify, ScrollState},
    text_style::FontWeight,
  },
  node::{
    Element, Style, border::Border, color::Color, dimension::Dimension, node_kind::TextOverflow, outline::Outline,
  },
};

/// The parts of a command palette a recipe styles. Its rows are a menu's items ([`MenuItemPart`]), laid over by what
/// the palette's recipe has for [`CommandPalettePart::Row`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CommandPalettePart {
  /// What covers the window behind the panel, and holds the panel near its top.
  Veil,
  /// The palette itself: its width, its edge, its corners, its shadow.
  Panel,
  /// The row at the top that is typed in: the icon, the input and the key that closes the palette.
  Search,
  /// The magnifier at the start of the search row.
  SearchIcon,
  /// The box that is typed in, laid over the theme's input: it has no edge or fill of its own.
  Input,
  /// A key of the search row's Escape and of the footer, laid over a [`Kbd`]'s key.
  Key,
  /// What scrolls when the commands are more than it has room for: its most height.
  Results,
  /// The list of commands in it: the room around them.
  List,
  /// The text over a group of commands.
  GroupLabel,
  /// A command's row, laid over a menu item's root.
  Row,
  /// The second text of a row, after its label.
  Detail,
  /// What the list says when nothing is found: "No results".
  Empty,
  /// The row that says more commands are on their way: a spinner and its text.
  Status,
  /// The row at the bottom with the keys that work in the palette.
  Footer,
  /// What a key of the footer does: "to run".
  FooterHint,
}

names!(CommandPalettePart {
  Veil => "veil",
  Panel => "panel",
  Search => "search",
  SearchIcon => "search_icon",
  Input => "input",
  Key => "key",
  Results => "results",
  List => "list",
  GroupLabel => "group_label",
  Row => "row",
  Detail => "detail",
  Empty => "empty",
  Status => "status",
  Footer => "footer",
  FooterHint => "footer_hint",
});

/// The states of a command palette a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CommandPaletteState {
  /// More commands are on their way: the list ends with a spinner.
  Loading,
  /// Nothing is found for what is typed.
  Empty,
}

names!(CommandPaletteState {
  Loading => "loading",
  Empty => "empty",
});

/// What builds a command palette's nodes: a theme's in place of the default one.
pub type CommandPaletteRenderer = Arc<dyn Fn(CommandPaletteView) -> Element + Send + Sync>;

/// Whether a command is shown while `query` is typed: what a palette is given in place of its own filter.
pub type CommandFilter = Arc<dyn Fn(&CommandItem, &str) -> bool + Send + Sync>;

type Handler = Arc<dyn Fn() + Send + Sync>;
type QueryHandler = Arc<dyn Fn(&str) + Send + Sync>;

/// A command of a [`CommandPalette`]: what it does or where it goes.
///
/// ```
/// use lurq::ui::{CommandItem, icons};
///
/// let theme = CommandItem::new("Toggle dark mode")
///   .icon(icons::MOON)
///   .shortcut("Ctrl+D")
///   .keywords(["theme", "appearance"])
///   .on_select(|| println!("toggle"));
/// let file = CommandItem::new("Open file").icon(icons::FILE).detail("src/main.rs");
/// let later = CommandItem::new("Publish").disabled(true);
/// ```
#[derive(Clone)]
pub struct CommandItem {
  label: Arc<str>,
  icon: Option<Name>,
  detail: Option<Arc<str>>,
  shortcut: Option<Arc<str>>,
  keywords: Vec<Arc<str>>,
  disabled: bool,
  on_select: Vec<Handler>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl CommandItem {
  /// A command called `label`: what it does, "Open file", or where it goes, "Settings".
  pub fn new(label: &str) -> Self {
    Self {
      label: Arc::from(label),
      icon: None,
      detail: None,
      shortcut: None,
      keywords: Vec::new(),
      disabled: false,
      on_select: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// An icon before the label, by its name in the theme's set ([`icons`](super::icons)).
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// A second text after the label, in the muted color: the folder of a file, the page a setting is on.
  pub fn detail(mut self, detail: &str) -> Self {
    self.detail = Some(Arc::from(detail));
    self
  }

  /// The keys that do the same, at the end of the row, as a menu writes them: "Ctrl+D". The palette only shows
  /// them: the app binds the keys.
  pub fn shortcut(mut self, keys: &str) -> Self {
    self.shortcut = Some(Arc::from(keys));
    self
  }

  /// Words that find the command and are not shown: "theme" for "Toggle dark mode".
  pub fn keywords<S: AsRef<str>>(mut self, words: impl IntoIterator<Item = S>) -> Self {
    self
      .keywords
      .extend(words.into_iter().map(|word| Arc::from(word.as_ref())));
    self
  }

  /// Whether the command cannot be run now: it is shown dimmed, and the keys and the pointer pass it by.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Called when the command is run, before the palette closes.
  pub fn on_select(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_select.push(Arc::new(handler));
    self
  }

  /// The id of the row.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  pub fn label(&self) -> &str {
    &self.label
  }

  pub fn detail_text(&self) -> Option<&str> {
    self.detail.as_deref()
  }

  pub fn keyword_list(&self) -> &[Arc<str>] {
    &self.keywords
  }

  pub fn is_disabled(&self) -> bool {
    self.disabled
  }
}

/// The default filter: the commands whose label, detail or keywords hold every word typed, without regard to case.
pub fn matches_every_word(item: &CommandItem, query: &str) -> bool {
  let text: String = std::iter::once(&*item.label)
    .chain(item.detail.as_deref())
    .chain(item.keywords.iter().map(|word| &**word))
    .collect::<Vec<_>>()
    .join(" ")
    .to_lowercase();
  query.split_whitespace().all(|word| text.contains(&word.to_lowercase()))
}

/// A group of commands with what they are about over them.
#[derive(Clone)]
struct CommandGroup {
  label: Option<Arc<str>>,
  items: Vec<CommandItem>,
}

/// What a palette keeps between builds while it is open: what is typed, the row that is highlighted, and where its
/// list is scrolled to. One for each open signal a palette is given, while that palette is in the tree: `alive` is
/// held by its input, and what was typed goes when the palette closes.
struct Kept {
  query: Signal<String>,
  highlight: Signal<usize>,
  scroll: ScrollState,
  results: ElementRef,
  rows: Vec<ElementRef>,
  alive: Weak<()>,
}

static KEPT: LazyLock<Mutex<HashMap<usize, Kept>>> = LazyLock::new(Default::default);

/// What the palette opened by `open` keeps, and what keeps it while the palette is in the tree. `rows` is how many
/// rows it shows now: each has a reference the list is scrolled to it by.
fn kept(
  open: usize,
  rows: usize,
) -> (
  Signal<String>,
  Signal<usize>,
  ScrollState,
  ElementRef,
  Vec<ElementRef>,
  Arc<()>,
) {
  let mut all = KEPT.lock().unwrap();
  all.retain(|_, kept| kept.alive.strong_count() > 0);
  let mut alive = None;
  if let Some(kept) = all.get(&open) {
    alive = kept.alive.upgrade();
  }
  let alive = match alive {
    Some(alive) => alive,
    None => {
      let alive = Arc::new(());
      all.insert(
        open,
        Kept {
          query: Signal::new(String::new()),
          highlight: Signal::new(0),
          scroll: ScrollState::new(),
          results: ElementRef::new(),
          rows: Vec::new(),
          alive: Arc::downgrade(&alive),
        },
      );
      alive
    }
  };
  let kept = all.get_mut(&open).expect("kept just now");
  while kept.rows.len() < rows {
    kept.rows.push(ElementRef::new());
  }
  (
    kept.query.clone(),
    kept.highlight.clone(),
    kept.scroll.clone(),
    kept.results.clone(),
    kept.rows[..rows].to_vec(),
    alive,
  )
}

/// A command palette: a search over the window that finds a command or a place by typing. Every command of an
/// editor, the pages of an app, the files of a project.
///
/// ```
/// use lurq::{
///   core::Signal,
///   ui::{CommandItem, CommandPalette, icons},
/// };
///
/// let open = Signal::new(false);
/// let palette = CommandPalette::new(open.clone())
///   .placeholder("Type a command or search")
///   .group(
///     "Suggestions",
///     [
///       CommandItem::new("Toggle dark mode").icon(icons::MOON).shortcut("Ctrl+D"),
///       CommandItem::new("Open DevTools").icon(icons::TERMINAL).shortcut("F12"),
///       CommandItem::new("New window").icon(icons::PLUS).shortcut("Ctrl+N"),
///     ],
///   )
///   .footer(true);
/// ```
///
/// A palette does not keep whether it is open: the app's signal does, and the app opens it (a shortcut, a button).
/// It stands near the top of the window over a veil, and the focus goes to its input as it opens and back to what had
/// it as it closes; Tab stays inside it. What is typed filters the commands, in the order the app gave them, across
/// their groups: by default those whose label, detail or keywords hold every word typed ([`Self::filter`] for
/// another way).
///
/// The focus stays in the input. Down and Up move the highlight through the rows that are shown, across groups and
/// round at the ends, passing a disabled one by; the pointer moves it too. Enter runs the highlighted command and
/// closes the palette, as a click on a row does. Escape clears what was typed, and closes the palette when nothing is.
/// A press on the veil closes it. To tooling it is a dialog holding a combobox and a listbox of options.
pub struct CommandPalette {
  open: Signal<bool>,
  placeholder: Arc<str>,
  groups: Vec<CommandGroup>,
  filter: Option<CommandFilter>,
  on_input: Vec<QueryHandler>,
  query: Option<Signal<String>>,
  loading: bool,
  empty: Arc<str>,
  footer: bool,
  inline: bool,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl CommandPalette {
  /// A palette shown while `open` holds `true`. Running a command, Escape on an empty input and a press on the veil set
  /// it to `false`.
  pub fn new(open: Signal<bool>) -> Self {
    Self {
      open,
      placeholder: Arc::from("Type a command or search"),
      groups: Vec::new(),
      filter: None,
      on_input: Vec::new(),
      query: None,
      loading: false,
      empty: Arc::from("No results"),
      footer: false,
      inline: false,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// What the input says while nothing is typed. "Type a command or search" by default.
  pub fn placeholder(mut self, placeholder: &str) -> Self {
    self.placeholder = Arc::from(placeholder);
    self
  }

  /// A command with no group, after what the palette holds already.
  pub fn item(mut self, item: CommandItem) -> Self {
    match self.groups.last_mut() {
      Some(group) if group.label.is_none() => group.items.push(item),
      _ => self.groups.push(CommandGroup {
        label: None,
        items: vec![item],
      }),
    }
    self
  }

  /// Commands with no group, in order, after what the palette holds already.
  pub fn items(self, items: impl IntoIterator<Item = CommandItem>) -> Self {
    items.into_iter().fold(self, Self::item)
  }

  /// A group of commands under `label`, what they are about: "Suggestions", "Pages", "Settings". A group with no
  /// command shown for what is typed is not shown.
  pub fn group(mut self, label: &str, items: impl IntoIterator<Item = CommandItem>) -> Self {
    self.groups.push(CommandGroup {
      label: Some(Arc::from(label)),
      items: items.into_iter().collect(),
    });
    self
  }

  /// Which commands are shown while something is typed, in place of the palette's own filter
  /// ([`matches_every_word`]): a fuzzy match, or every command for an app that fetches its own ([`Self::on_input`]).
  pub fn filter(mut self, filter: impl Fn(&CommandItem, &str) -> bool + Send + Sync + 'static) -> Self {
    self.filter = Some(Arc::new(filter));
    self
  }

  /// Called with what the input will hold, each time it is edited: an app that fetches its commands asks for those of
  /// the new text, and gives the palette what it found.
  pub fn on_input(mut self, handler: impl Fn(&str) + Send + Sync + 'static) -> Self {
    self.on_input.push(Arc::new(handler));
    self
  }

  /// The app's own signal for what is typed, in place of the palette's: to read it, or to open the palette with
  /// something typed already.
  pub fn query(mut self, query: Signal<String>) -> Self {
    self.query = Some(query);
    self
  }

  /// Whether more commands are on their way: the list ends with a spinner while they are.
  pub fn loading(mut self, loading: bool) -> Self {
    self.loading = loading;
    self
  }

  /// What the list says when nothing is found for what is typed. "No results" by default.
  pub fn empty(mut self, text: &str) -> Self {
    self.empty = Arc::from(text);
    self
  }

  /// Whether the palette ends with a row of the keys that work in it: the arrows, Enter and Escape.
  pub fn footer(mut self, footer: bool) -> Self {
    self.footer = footer;
    self
  }

  /// Draws the panel where the palette is declared, open or not, without the veil and without taking the window: for
  /// a gallery or a test. It still filters, highlights and runs what is chosen.
  pub fn inline(mut self, inline: bool) -> Self {
    self.inline = inline;
    self
  }

  /// Puts the palette in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this palette over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this palette.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the panel.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// The key of what the palette lays over the window.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the palette in `theme`. Converting a palette into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    if !self.inline && !self.open.get() {
      // Declared, taking no room: the modal is laid over the window only while it is open.
      return Modal::new(Column::new()).open(self.open).target(Root).into();
    }
    let filter = self.filter.clone().unwrap_or_else(|| Arc::new(matches_every_word));
    let count: usize = self.groups.iter().map(|group| group.items.len()).sum();
    let (own_query, highlight, scroll, results, rows, alive) = kept(self.open.id(), count);
    let query = self.query.clone().unwrap_or(own_query);
    let shown = shown(&self.groups, &query.get(), &filter);
    let enabled: Vec<bool> = shown.iter().map(|at| !self.command(*at).disabled).collect();
    let highlighted = settle(&enabled, highlight.get());
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(CommandPaletteState::Loading, self.loading)
    .state(CommandPaletteState::Empty, shown.is_empty() && !self.loading);
    let view = CommandPaletteView {
      placeholder: self.placeholder,
      loading: self.loading,
      empty: self.empty,
      footer: self.footer,
      inline: self.inline,
      highlighted,
      recipe: theme.resolve::<CommandPalette>(&case, Some(&self.rules)),
      groups: self.groups,
      shown,
      enabled,
      filter,
      on_input: self.on_input,
      open: self.open,
      query,
      highlight,
      scroll,
      results,
      rows,
      alive,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<CommandPalette>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  fn command(&self, (group, item): (usize, usize)) -> &CommandItem {
    &self.groups[group].items[item]
  }
}

impl From<CommandPalette> for Element {
  fn from(palette: CommandPalette) -> Self {
    palette.build(&current_theme())
  }
}

/// The commands shown while `query` is typed, each by its group and its place in it: all of them while nothing is.
fn shown(groups: &[CommandGroup], query: &str, filter: &CommandFilter) -> Vec<(usize, usize)> {
  let all = query.trim().is_empty();
  groups
    .iter()
    .enumerate()
    .flat_map(|(group, commands)| {
      commands
        .items
        .iter()
        .enumerate()
        .filter(move |(_, item)| all || filter(item, query))
        .map(move |(item, _)| (group, item))
    })
    .collect()
}

/// The row that is highlighted when `wanted` is asked for: it, or the first that can be run after it, or before it.
fn settle(enabled: &[bool], wanted: usize) -> Option<usize> {
  let wanted = wanted.min(enabled.len().saturating_sub(1));
  (wanted..enabled.len()).chain(0..wanted).find(|index| enabled[*index])
}

/// The row that can be run `step` rows from `from`, round at the ends.
fn step(enabled: &[bool], from: Option<usize>, forward: bool) -> Option<usize> {
  let count = enabled.len();
  if count == 0 {
    return None;
  }
  let mut at = from.unwrap_or(if forward { count - 1 } else { 0 });
  for _ in 0..count {
    at = if forward {
      (at + 1) % count
    } else {
      (at + count - 1) % count
    };
    if enabled[at] {
      return Some(at);
    }
  }
  None
}

/// A palette as a renderer sees it: what it shows, the style of its parts, and its behavior ready to be dressed.
pub struct CommandPaletteView {
  pub placeholder: Arc<str>,
  pub loading: bool,
  /// What the list says when nothing is found.
  pub empty: Arc<str>,
  pub footer: bool,
  pub inline: bool,
  /// The row that is highlighted, among those shown.
  pub highlighted: Option<usize>,
  /// The style of each part of this palette.
  pub recipe: ResolvedRecipe,
  groups: Vec<CommandGroup>,
  /// The commands shown, by their group and their place in it.
  shown: Vec<(usize, usize)>,
  enabled: Vec<bool>,
  filter: CommandFilter,
  on_input: Vec<QueryHandler>,
  open: Signal<bool>,
  query: Signal<String>,
  highlight: Signal<usize>,
  scroll: ScrollState,
  results: ElementRef,
  rows: Vec<ElementRef>,
  alive: Arc<()>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl CommandPaletteView {
  /// The style of a part, as this palette is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// How many rows are shown.
  pub fn shown(&self) -> usize {
    self.shown.len()
  }

  /// The command of the row `row` among those shown.
  pub fn command(&self, row: usize) -> &CommandItem {
    let (group, item) = self.shown[row];
    &self.groups[group].items[item]
  }

  /// The signal that holds whether the palette is open.
  pub fn open(&self) -> &Signal<bool> {
    &self.open
  }

  /// What runs the command of row `row` and closes the palette: nothing for one that is disabled.
  fn runner(&self, row: usize) -> Option<impl Fn() + Send + Sync + Clone + 'static> {
    let command = self.command(row);
    if command.disabled {
      return None;
    }
    let (handlers, open) = (command.on_select.clone(), self.open.clone());
    Some(move || {
      for handler in &handlers {
        handler();
      }
      open.set(false);
    })
  }

  /// Scrolls the list by the least that shows row `row` whole.
  fn reveal(scroll: &ScrollState, results: &ElementRef, row: &ElementRef) {
    let (view, row) = (results.bounds(), row.bounds());
    if view.height <= 0.0 || row.height <= 0.0 {
      return;
    }
    let by = if row.y < view.y {
      row.y - view.y
    } else if row.y + row.height > view.y + view.height {
      row.y + row.height - view.y - view.height
    } else {
      return;
    };
    scroll.set_scroll_pending(scroll.scroll_x(), (scroll.scroll_y() + by).max(0.0));
  }

  /// The panel in the style of its part, with nothing in it yet: a dialog to what reads the tree, called by what its
  /// input says. A press on it goes no further, so that it never reaches the veil.
  pub fn panel(&self) -> Column {
    let mut panel = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(CommandPalettePart::Panel))
      // Never more than the room the veil leaves it.
      .max_width(Dimension::Pct(100.0))
      .max_height(Dimension::Pct(100.0))
      .on_click(|event: MouseEvent| event.stop_propagation())
      .describe("a11y_role", "dialog")
      .describe("a11y_name", self.placeholder.clone());
    if let Some(id) = &self.id {
      panel = panel.id(id.clone());
    }
    panel
  }

  /// A key as the theme's [`Kbd`] draws one, in the platform's names, with [`CommandPalettePart::Key`] laid over its
  /// key: "Esc", "Enter" (↩ on a Mac). It keeps its width in a row that has too little room.
  pub fn key(&self, key: &str) -> Element {
    Row::new()
      .flex_full(0.0, 0.0, None)
      .align_items(Alignment::Center)
      .child(
        Kbd::new(key)
          .platform(KeyPlatform::current())
          .rules(&self.recipe.rules_for(CommandPalettePart::Key, KbdPart::Key))
          .build(self.theme()),
      )
      .into()
  }

  /// The input, which keeps the focus while the palette is open: the theme's input, with no edge or fill of its own.
  /// It is a combobox to what reads the tree, and takes the keys that move the highlight, run and close.
  pub fn input(&self) -> Element {
    let rules = self.recipe.rules_for(CommandPalettePart::Input, InputPart::Control);
    let mut typed = Input::new(self.query.clone())
      .placeholder(&self.placeholder)
      .full_width(true)
      .label(&self.placeholder)
      .rules(&rules);
    // What is typed puts the highlight on the first row it shows, and tells the app.
    {
      let (groups, filter, highlight, handlers) = (
        self.groups.clone(),
        self.filter.clone(),
        self.highlight.clone(),
        self.on_input.clone(),
      );
      typed = typed.on_input(move |event: &TextInputEvent| {
        let shown = shown(&groups, &event.new_value, &filter);
        let enabled: Vec<bool> = shown
          .iter()
          .map(|(group, item)| !groups[*group].items[*item].disabled)
          .collect();
        let first = settle(&enabled, 0).unwrap_or(0);
        if highlight.get_untracked() != first {
          highlight.set(first);
        }
        for handler in &handlers {
          handler(&event.new_value);
        }
      });
    }
    let view = typed.view(self.theme());
    let keys = self.keys();
    let mut control = view
      .control()
      .describe("a11y_role", "combobox")
      .describe("a11y_expanded", "true")
      .describe("a11y_autocomplete", "list")
      .on_key_down(keys);
    // The option the keys are on, by its label: the focus stays here.
    if let Some(row) = self.highlighted {
      control = control.describe("a11y_activedescendant", self.command(row).label.clone());
    }
    view.render_with_end(control, None).into()
  }

  /// What the keys do while the focus is in the palette: Down and Up move the highlight round the rows that can be
  /// run, Enter runs the highlighted one, Escape clears what is typed and then closes the palette.
  fn keys(&self) -> impl Fn(KeyboardEvent) + Send + Sync + 'static {
    let runners: Vec<Option<Handler>> = (0..self.shown())
      .map(|row| self.runner(row).map(|run| Arc::new(run) as Handler))
      .collect();
    let (enabled, highlighted) = (self.enabled.clone(), self.highlighted);
    let (highlight, query, open) = (self.highlight.clone(), self.query.clone(), self.open.clone());
    let (groups, filter) = (self.groups.clone(), self.filter.clone());
    let (scroll, results, rows, alive) = (
      self.scroll.clone(),
      self.results.clone(),
      self.rows.clone(),
      self.alive.clone(),
    );
    move |event: KeyboardEvent| {
      // Held while the input is: what the palette keeps is kept while it is in the tree.
      let _ = &alive;
      if !event.focus_within || event.ctrl || event.alt || event.meta || event.composing {
        return;
      }
      match event.key.as_str() {
        "ArrowDown" | "ArrowUp" => {
          event.prevent_default();
          if let Some(next) = step(&enabled, highlighted, event.key == "ArrowDown") {
            highlight.set(next);
            if let Some(row) = rows.get(next) {
              Self::reveal(&scroll, &results, row);
            }
          }
        }
        "Enter" => {
          event.prevent_default();
          if let Some(Some(run)) = highlighted.and_then(|row| runners.get(row)) {
            run();
          }
        }
        "Escape" => {
          event.prevent_default();
          if query.get_untracked().is_empty() {
            open.set(false);
          } else {
            query.set(String::new());
            let all = shown(&groups, "", &filter);
            let enabled: Vec<bool> = all
              .iter()
              .map(|(group, item)| !groups[*group].items[*item].disabled)
              .collect();
            highlight.set(settle(&enabled, 0).unwrap_or(0));
            scroll.set_scroll_pending(0.0, 0.0);
          }
        }
        _ => {}
      }
    }
  }

  /// The row at the top: the magnifier, the input, and the key that closes the palette.
  pub fn search(&self) -> Row {
    Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(CommandPalettePart::Search))
      .child(glyph(
        &self.part(CommandPalettePart::SearchIcon),
        Name::new(super::icons::SEARCH),
        false,
      ))
      // The input takes the room the icon and the key leave, and gives way first.
      .child(
        Row::new()
          .flex_full(1.0, 1.0, Some(0.0))
          .align_items(Alignment::Center)
          .child(self.input()),
      )
      .child(self.key("Esc"))
  }

  /// The text over a group of commands.
  pub fn group_label(&self, label: &str) -> Element {
    let part = self.part(CommandPalettePart::GroupLabel);
    Row::new().part(&part).child(part.text(label).nowrap()).into()
  }

  /// The row of the command `row` among those shown: a menu item's parts, highlighted as a menu item is under the
  /// keyboard while it is the highlighted one. It is an option of the list to what reads the tree. The pointer on it
  /// highlights it, and a click runs it.
  pub fn row(&self, row: usize) -> Element {
    let command = self.command(row).clone();
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: Vec::new(),
    }
    .state(MenuItemState::Disabled, command.disabled);
    let rules = self.recipe.rules_for(CommandPalettePart::Row, MenuItemPart::Root);
    let item = self.theme().resolve::<MenuItem>(&case, Some(&rules));
    let on = self.highlighted == Some(row);
    let forced = on.then_some(Interaction::FocusVisible);
    let part = |name: MenuItemPart| item.part(name).forced(forced);
    // The highlight is the palette's alone: the pointer moves it, and draws nothing of its own.
    let mut root = Row::new()
      .align_items(Alignment::Center)
      .justify(Justify::SpaceBetween)
      .part(&part(MenuItemPart::Root))
      .hovered_style(Style::default())
      .active_style(Style::default())
      .describe("a11y_role", "option")
      .describe("a11y_name", command.label.clone())
      .describe("a11y_selected", if on { "true" } else { "false" });
    if command.disabled {
      root = root.describe("a11y_disabled", "true");
    } else {
      let highlight = self.highlight.clone();
      root = root.on_mouse_enter(move || {
        if highlight.get_untracked() != row {
          highlight.set(row);
        }
      });
      if let Some(run) = self.runner(row) {
        root = root.on_click(move |_| run());
      }
    }
    if let Some(at) = self.rows.get(row) {
      root = root.ref_element(at.clone());
    }
    if let Some(id) = &command.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &command.key {
      root = root.key(key.clone());
    }
    let gap = part(MenuItemPart::Root).rest().gap.unwrap_or_default();
    // The texts take the room the shortcut leaves, and give way before it.
    let mut start = Row::new()
      .flex_full(1.0, 1.0, Some(0.0))
      .spacing(gap.clone())
      .align_items(Alignment::Center);
    if let Some(icon) = command.icon {
      start = start.child(glyph(&part(MenuItemPart::Icon), icon, false));
    }
    let label = part(MenuItemPart::Label);
    start = start.child(
      Row::new()
        .part(&label)
        .child(label.text(&command.label).nowrap().text_overflow(TextOverflow::Elipsis)),
    );
    if let Some(detail) = &command.detail {
      let part = self.part(CommandPalettePart::Detail);
      start = start.child(
        Row::new()
          .flex_full(1.0, 1.0, Some(0.0))
          .part(&part)
          .child(part.text(detail).nowrap().text_overflow(TextOverflow::Elipsis)),
      );
    }
    root = root.child(start);
    // The shortcut as a menu writes it: the theme's keys in one text, with the menu item's shortcut over its label.
    if let Some(shortcut) = &command.shortcut {
      root = root.child(
        Row::new()
          .flex_full(0.0, 0.0, None)
          .align_items(Alignment::Center)
          .child(
            Kbd::shortcut(shortcut)
              .variant(KbdVariant::Plain)
              .platform(KeyPlatform::current())
              .rules(&item.rules_for(MenuItemPart::Shortcut, KbdPart::Label))
              .build(self.theme()),
          ),
      );
    }
    root.into()
  }

  /// What the list says when nothing is found.
  pub fn empty_state(&self) -> Element {
    let part = self.part(CommandPalettePart::Empty);
    Row::new()
      .justify(Justify::Center)
      .part(&part)
      .child(part.text(&self.empty))
      .into()
  }

  /// The row that says more commands are on their way.
  pub fn status(&self) -> Element {
    let part = self.part(CommandPalettePart::Status);
    Row::new()
      .align_items(Alignment::Center)
      .part(&part)
      .describe("a11y_role", "status")
      .child(Spinner::new().decorative(true).build(self.theme()))
      .child(part.text("Loading…").nowrap())
      .into()
  }

  /// The list: the groups with their labels and the rows shown in them, the status while more are on their way, or
  /// what it says when nothing is found. It scrolls past the most height of its part. A listbox to what reads the
  /// tree.
  pub fn results(&self) -> Element {
    let mut list = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(CommandPalettePart::List))
      .describe("a11y_role", "listbox");
    let mut group = None;
    for row in 0..self.shown() {
      let at = self.shown[row].0;
      if group != Some(at) {
        group = Some(at);
        if let Some(label) = &self.groups[at].label {
          list = list.child(self.group_label(label));
        }
      }
      list = list.child(self.row(row));
    }
    if self.loading {
      list = list.child(self.status());
    } else if self.shown.is_empty() {
      list = list.child(self.empty_state());
    }
    // As high as what is in it up to the most, and gives way when the window has less.
    ScrollVertical::new(list)
      .with_scroll_state(self.scroll.clone())
      .width(Dimension::Pct(100.0))
      .flex_full(0.0, 1.0, None)
      .part(&self.part(CommandPalettePart::Results))
      .ref_element(self.results.clone())
      .into()
  }

  /// The row at the bottom with the keys that work in the palette, under a line.
  pub fn footer_row(&self) -> Element {
    let hint = |keys: &[&str], says: &str| {
      let part = self.part(CommandPalettePart::FooterHint);
      let gap = self.part(CommandPalettePart::Footer).rest().gap.unwrap_or_default();
      let mut row = Row::new()
        .align_items(Alignment::Center)
        .spacing(gap.clone())
        .part(&part);
      for key in keys {
        row = row.child(self.key(key));
      }
      row.child(part.text(says).nowrap())
    };
    Column::new()
      .align_items(Alignment::Stretch)
      .child(Separator::horizontal().build(self.theme()))
      .child(
        Row::new()
          .align_items(Alignment::Center)
          .part(&self.part(CommandPalettePart::Footer))
          .child(hint(&["↑", "↓"], "to move"))
          .child(hint(&["Enter"], "to run"))
          .child(hint(&["Esc"], "to close")),
      )
      .into()
  }

  /// The veil over the window with the panel near its top. A press on it closes the palette.
  pub fn veil(&self, panel: impl Into<Element>) -> Column {
    let open = self.open.clone();
    let mut veil = Column::new()
      .size(Dimension::Pct(100.0), Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .justify(Justify::Start)
      .part(&self.part(CommandPalettePart::Veil))
      .on_click(move |_| open.set(false))
      .child(panel);
    if let Some(key) = &self.key {
      veil = veil.key(key.clone());
    }
    veil
  }

  /// The veil and the panel laid over the window while the palette is open, as the engine's [`Modal`]: Tab stays
  /// inside it, and the focus goes to its input as it opens and back to what had it as it closes. Escape is the
  /// palette's own.
  pub fn modal(&self, panel: impl Into<Element>) -> Element {
    Modal::new(self.veil(panel))
      .open(self.open.clone())
      .target(Root)
      .dismiss_on_escape(false)
      .takes_focus(true)
      .into()
  }

  /// How the default renderer builds a palette: the search row, the list and the footer in the panel, and the panel
  /// over the veil.
  pub fn render(self) -> Element {
    let mut panel = self.panel().child(self.search()).child(self.results());
    if self.footer {
      panel = panel.child(self.footer_row());
    }
    if self.inline { panel.into() } else { self.modal(panel) }
  }
}

impl Themed for CommandPalette {
  const NAME: &'static str = "command_palette";
  type Renderer = CommandPaletteRenderer;

  /// The design's command palette (`pencil/lurq-design-system.pen`, Command Palette): a panel of 420 in the popover's
  /// color with an edge of the border's color, corners of `radius-xl` and the large shadow, 72 from the top of the
  /// window over the scrim. Its search row is 44 high, 14 in from the sides with 10 between the magnifier of 16, the
  /// input in the body's type and the key that closes it, and a line under it. Its list is padded 4, a group's label
  /// 12/500 in the muted foreground, padded 6 and 8, and its rows are a menu's items. Its keys are the theme's Kbd,
  /// and a row's shortcut the Kbd's plain text, as a menu writes it. The list scrolls past 320.
  fn recipe() -> Recipe {
    use CommandPalettePart::*;
    use PaletteColor::{MutedForeground, Popover, PopoverForeground, Scrim};
    let clear = Color::new(0, 0, 0, 0);
    let unedged = move |s: super::PartStyle| s.background(clear).border(Border::inside(1.0, clear));
    Recipe::new()
      // The margin keeps the panel off the window's edges, and near its top.
      .base(Veil, |s| {
        s.background(Scrim)
          .padding_x(16.0)
          .padding_top(72.0)
          .padding_bottom(16.0)
      })
      .base(Panel, |s| {
        s.width(420.0)
          .background(Popover)
          .foreground(PopoverForeground)
          .border(Border::inside(BorderSize::Sm, PaletteColor::Border))
          .radius(RadiusSize::Xl)
          .shadow(ShadowStyle::Lg)
      })
      .base(Search, |s| {
        s.height(44.0)
          .padding_x(14.0)
          .gap(10.0)
          .underline(Border::inside(BorderSize::Sm, PaletteColor::Border))
      })
      .base(SearchIcon, |s| s.square(IconSize::Md).foreground(MutedForeground))
      .base(Input, move |s| unedged(s).padding_x(0.0).typography(TypographyStyle::Body))
      .hovered(Input, move |s| s.border(Border::inside(1.0, clear)))
      .focus_visible(Input, move |s| s.outline(Outline::new(0.0, clear)))
      .base(Results, |s| s.max_height(320.0))
      .base(List, |s| s.padding_x(4.0).padding_y(4.0))
      .base(GroupLabel, |s| {
        s.padding_x(8.0)
          .padding_y(6.0)
          .typography(TypographyStyle::Label)
          .font_size(12.0)
          .weight(FontWeight::Medium)
          .foreground(MutedForeground)
      })
      .base(Detail, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(12.0)
          .foreground(MutedForeground)
      })
      .base(Empty, |s| {
        s.padding_x(8.0)
          .padding_y(24.0)
          .typography(TypographyStyle::Body)
          .foreground(MutedForeground)
      })
      .base(Status, |s| {
        s.height(32.0)
          .padding_x(8.0)
          .gap(8.0)
          .typography(TypographyStyle::Body)
          .font_size(13.0)
          .foreground(MutedForeground)
      })
      .base(Footer, |s| s.height(36.0).padding_x(12.0).gap(16.0))
      .base(FooterHint, |s| {
        s.gap(4.0)
          .typography(TypographyStyle::Body)
          .font_size(12.0)
          .foreground(MutedForeground)
      })
  }
}
