//! A command bar: the tabs of a window's commands over the commands of the
//! selected one, in labelled groups. A ribbon.
//!
//! It is built of a [`Tabs`](super::Tabs) and a [`Toolbar`]: the bar's
//! groups, its overflow, its focus group and its More menu are the toolbar's.

use std::sync::Arc;

use super::{
  icons, names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{fading, glyph, one_line, pressable},
  tabs::{Tab, Tabs, TabsPart, TabsState, TabsVariant},
  theme::{Themed, current_theme},
  toolbar::{Toolbar, ToolbarCommand, ToolbarItem, ToolbarPart, ToolbarSize, ToolbarVariant},
};
use crate::{
  app::{
    ctx::Ctx,
    theme::{BorderSize, IconSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::Column,
  core::{Signal, SignalValue},
  layout::{Alignment, text_style::FontWeight},
  node::{Element, border::Border, outline::Outline},
};

/// The parts of a command bar a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CommandBarPart {
  /// The bar: its fill, its edge, its radius.
  Root,
  /// What the bar says of the row of its tabs: laid over the tabs' list.
  Tabs,
  /// What the bar says of a tab: laid over the tabs' tab.
  Tab,
  /// What the bar says of a tab's label: laid over the tabs' label.
  TabLabel,
  /// What the bar says of the row of the selected tab's commands: laid over the toolbar's root.
  Bar,
  /// What the bar says of the chevron at its end: laid over the toolbar's More button.
  More,
}

names!(CommandBarPart {
  Root => "root",
  Tabs => "tabs",
  Tab => "tab",
  TabLabel => "tab_label",
  Bar => "bar",
  More => "more",
});

/// The states of a command bar a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CommandBarState {
  /// Its tabs and its commands take no input.
  Disabled,
}

names!(CommandBarState {
  Disabled => "disabled",
});

/// What builds a command bar's nodes: a theme's in place of the default one.
pub type CommandBarRenderer = Arc<dyn Fn(CommandBarView) -> Element + Send + Sync>;

/// One tab of a command bar: its label in the row of tabs, and the commands
/// the bar shows while it is selected, as a toolbar has them: labelled
/// groups ([`ToolbarGroup`](super::ToolbarGroup)), stacks
/// ([`ToolbarItem::stack`]), commands, toggles, a select.
///
/// ```
/// use lurq::ui::{CommandBarCommand, CommandBarTab, ToolbarGroup, icons};
///
/// let home = CommandBarTab::new("home", "Home")
///   .item(ToolbarGroup::new().label("Clipboard").item(CommandBarCommand::new("Paste").icon(icons::PASTE).large(true)));
/// ```
pub struct CommandBarTab<T> {
  value: T,
  label: Arc<str>,
  items: Vec<ToolbarItem>,
  secondary: Vec<ToolbarCommand>,
  disabled: bool,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<T> CommandBarTab<T> {
  /// The tab of `value`, called `label`, with no commands yet.
  pub fn new(value: T, label: &str) -> Self {
    Self {
      value,
      label: Arc::from(label),
      items: Vec::new(),
      secondary: Vec::new(),
      disabled: false,
      id: None,
      key: None,
    }
  }

  /// A thing of the tab's bar, after the ones it has: what
  /// [`Toolbar::item`] takes, and a [`CommandBarCommand`].
  pub fn item(mut self, item: impl Into<ToolbarItem>) -> Self {
    self.items.push(item.into());
    self
  }

  pub fn items<I: Into<ToolbarItem>>(mut self, items: impl IntoIterator<Item = I>) -> Self {
    self.items.extend(items.into_iter().map(Into::into));
    self
  }

  /// A command that is never in the bar, only in the menu at its end, after
  /// what did not fit: one a person seldom needs ("Customize").
  pub fn secondary_command(mut self, command: ToolbarCommand) -> Self {
    self.secondary.push(command);
    self
  }

  /// Commands that are only in the menu, after the ones the tab has.
  pub fn secondary_commands(mut self, commands: impl IntoIterator<Item = ToolbarCommand>) -> Self {
    self.secondary.extend(commands);
    self
  }

  /// Whether the tab cannot be selected.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// The id of the tab in the row of tabs.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }
}

type Handler<T> = Arc<dyn Fn(&T) + Send + Sync>;

/// The tabs of a bar, bound to the app's signal or told the app's value.
type MakeTabs<T> = Box<dyn FnOnce(Vec<Tab<T>>) -> Tabs<T>>;

/// A command bar: the tabs of a window's commands ("Home", "Insert",
/// "View") over the commands of the selected one, in groups with a label
/// under each ("Clipboard", "Format"). The bar of an editor's window, of a
/// file manager's.
///
/// ```
/// use lurq::{
///   app::ctx::Ctx,
///   core::Signal,
///   node::Element,
///   ui::{CommandBar, CommandBarCommand, CommandBarTab, Toggle, ToolbarGroup, ToolbarItem, icons},
/// };
///
/// # fn build(ctx: &mut Ctx, tab: Signal<&'static str>) -> Element {
/// let clipboard = ToolbarGroup::new()
///   .label("Clipboard")
///   .item(CommandBarCommand::new("Paste").icon(icons::PASTE).large(true))
///   .item(ToolbarItem::stack([
///     CommandBarCommand::new("Cut").icon(icons::CUT),
///     CommandBarCommand::new("Copy").icon(icons::COPY),
///   ]));
/// let format = ToolbarGroup::new()
///   .label("Format")
///   .item(Toggle::new("Bold").icon(icons::BOLD).icon_only(true));
/// CommandBar::new(tab, [
///   CommandBarTab::new("home", "Home").item(clipboard).item(format),
///   CommandBarTab::new("insert", "Insert"),
/// ])
/// .mount(ctx)
/// # }
/// ```
///
/// What a toolbar is, its commands are: Tab stops in the row of tabs once and
/// in the commands once, Left and Right move in each, and what does not fit
/// goes into the menu at the end of the bar, behind a chevron, the last first
/// and a labelled group whole, under its label. A tab's secondary commands
/// are only in that menu, which is then always there.
pub struct CommandBar<T = ()> {
  tabs: Vec<CommandBarTab<T>>,
  selected: T,
  make_tabs: MakeTabs<T>,
  on_change: Vec<Handler<T>>,
  label: Option<Arc<str>>,
  disabled: bool,
  more_label: Arc<str>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> CommandBar<T> {
  fn of(selected: T, tabs: impl IntoIterator<Item = CommandBarTab<T>>, make_tabs: MakeTabs<T>) -> Self {
    Self {
      tabs: tabs.into_iter().collect(),
      selected,
      make_tabs,
      on_change: Vec::new(),
      label: None,
      disabled: false,
      more_label: super::current_locale().navigation.more_commands.clone(),
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// A bar whose selected tab is the one with the signal's value. Selecting another sets the signal.
  pub fn new(value: Signal<T>, tabs: impl IntoIterator<Item = CommandBarTab<T>>) -> Self
  where
    T: SignalValue,
  {
    let selected = value.get();
    Self::of(selected, tabs, Box::new(move |tabs| Tabs::new(value, tabs)))
  }

  /// A bar whose selected tab the app holds: the tab of `value` is the
  /// selected one, and `on_change` is told the value of each other one a
  /// person selects. The bar selects nothing itself.
  pub fn controlled(
    value: T,
    tabs: impl IntoIterator<Item = CommandBarTab<T>>,
    on_change: impl Fn(&T) + Send + Sync + 'static,
  ) -> Self {
    let held = value.clone();
    Self::of(value, tabs, Box::new(move |tabs| Tabs::controlled(held, tabs, |_| {}))).on_change(on_change)
  }

  /// Called when a person selects a tab other than the selected one, with
  /// its value: after the signal is set where the bar is bound.
  pub fn on_change(mut self, handler: impl Fn(&T) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// What the bar is called, to what reads the tree: "Commands" by default.
  /// The row of its commands is called by the selected tab.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Whether the bar's tabs and commands take no input: a window that is busy.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// What the button at the end of the bar is called, in its tooltip and to
  /// what reads the tree: "More commands" by default.
  pub fn more_label(mut self, label: &str) -> Self {
    self.more_label = Arc::from(label);
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

  /// Draws the button at the end of the bar as it is under the pointer, held
  /// down or focused from the keyboard, with nothing pointing at it: for a
  /// gallery or a test. A command is forced with its own `force`.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
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

  /// The id of the bar. Its row of tabs has this id with `-tabs` after it,
  /// the row of commands `-bar`, the button at its end `-bar-more` and the
  /// menu `-bar-menu`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Puts the bar in the tree of `ctx`'s component, in the theme of what is
  /// being rendered: its tabs and its commands keep what they keep, which
  /// commands did not fit, whether the menu is open, where Tab enters.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    self.make(&current_theme(), Some(ctx))
  }

  /// Builds the bar in `theme` once, with nothing kept: the commands that do
  /// not fit are cut off, and there is no menu. Converting a bar into an
  /// element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    self.make(theme, None)
  }

  fn make(self, theme: &Theme, ctx: Option<&mut Ctx>) -> Element {
    let mut states = self.states;
    if self.disabled {
      states.push(CommandBarState::Disabled.into());
    }
    let mut case = Case::new().breakpoint(super::current_breakpoint());
    for state in &states {
      case = case.state(state.clone(), true);
    }
    let recipe = theme
      .resolve::<CommandBar>(&case, Some(&self.rules))
      .owned_by(self.id.as_ref());
    let label = self
      .label
      .unwrap_or_else(|| super::current_locale().navigation.commands.clone());
    let mut index = None;
    let mut chosen = None;
    let mut tabs = Vec::with_capacity(self.tabs.len());
    for (at, tab) in self.tabs.into_iter().enumerate() {
      let mut entry = Tab::new(tab.value.clone(), &tab.label).disabled(tab.disabled);
      if let Some(id) = &tab.id {
        entry = entry.id(id.clone());
      }
      if let Some(key) = &tab.key {
        entry = entry.key(key.clone());
      }
      tabs.push(entry);
      if tab.value == self.selected {
        index = Some(at);
        chosen = Some((tab.label, tab.items, tab.secondary));
      }
    }
    let (tab_label, mut items, secondary) = chosen.unwrap_or_else(|| (Arc::from(""), Vec::new(), Vec::new()));
    // The chevron is at the far end of the bar, as the board draws it, unless the app put the room somewhere itself.
    if !items.iter().any(ToolbarItem::is_spacer) {
      items.push(Toolbar::spacer());
    }
    let mut row = (self.make_tabs)(tabs)
      .variant(TabsVariant::Line)
      .label(&label)
      .disabled(self.disabled)
      .rules(&tab_rules(&recipe));
    for handler in self.on_change {
      row = row.on_change(move |value: &T| handler(value));
    }
    let mut bar = Toolbar::new()
      .label(&tab_label)
      .variant(ToolbarVariant::Plain)
      .size(ToolbarSize::Xs)
      .overflow(true)
      .more_label(&self.more_label)
      .more_icon(icons::CHEVRON_DOWN)
      // The chevron stands level with the large commands, at the top of the groups and their labels.
      .top(true)
      .disabled(self.disabled)
      .force(self.forced)
      .rules(
        &recipe
          .rules_for(CommandBarPart::Bar, ToolbarPart::Root)
          .extend(&recipe.rules_for(CommandBarPart::More, ToolbarPart::MoreButton)),
      )
      .items(items)
      .secondary_commands(secondary)
      // Each tab's commands are a bar of their own: which of them did not fit is not another tab's.
      .key(format!("command-bar-tab-{}", index.unwrap_or(0)));
    if let Some(id) = &self.id {
      row = row.id(format!("{id}-tabs"));
      bar = bar.id(format!("{id}-bar"));
    }
    let (tabs, bar) = match ctx {
      Some(ctx) => (row.mount(ctx), bar.mount(ctx)),
      None => (row.build(theme), bar.build(theme)),
    };
    let view = CommandBarView {
      tabs,
      bar,
      label,
      recipe,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<CommandBar>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

/// What the bar says of its tabs, laid over the tabs' recipe: the row, a tab
/// and its label, and the selected one's label of 600.
fn tab_rules(recipe: &ResolvedRecipe) -> Recipe {
  recipe
    .rules_for(CommandBarPart::Tabs, TabsPart::List)
    .extend(&recipe.rules_for(CommandBarPart::Tab, TabsPart::Tab))
    .extend(&recipe.rules_for(CommandBarPart::TabLabel, TabsPart::Label))
    .when(Selector::new().state(TabsState::Selected), TabsPart::Label, |s| {
      s.weight(FontWeight::SemiBold)
    })
}

impl<T: Clone + PartialEq + Send + Sync + 'static> From<CommandBar<T>> for Element {
  fn from(bar: CommandBar<T>) -> Self {
    bar.build(&current_theme())
  }
}

/// A command bar as a renderer sees it: its row of tabs and its row of
/// commands, built, and the style of its parts.
pub struct CommandBarView {
  /// The row of tabs.
  pub tabs: Element,
  /// The selected tab's commands: a toolbar.
  pub bar: Element,
  /// What the bar is called to what reads the tree.
  pub label: Arc<str>,
  /// The style of each part of this bar.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl CommandBarView {
  /// The style of a part, as this bar is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// How the default renderer builds a command bar: the row of tabs over the
  /// row of commands, in a frame. It is a group to what reads the tree.
  pub fn render(self) -> Element {
    let mut root = Column::new()
      .part(&self.part(CommandBarPart::Root))
      .describe("a11y_role", "group")
      .describe("a11y_name", self.label.clone())
      .child(self.tabs)
      .child(self.bar);
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> crate::ui::Styled for CommandBar<T> {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

/// Every command bar has the one recipe and renderer, whatever its tabs' values are.
impl<T: Send + Sync + 'static> Themed for CommandBar<T> {
  const NAME: &'static str = "command_bar";
  type Renderer = CommandBarRenderer;

  fn parts() -> crate::ui::Declared {
    CommandBarPart::declared()
  }

  fn states() -> crate::ui::Declared {
    CommandBarState::declared()
  }

  /// The design's command bar (`pencil/lurq-design-system.pen`, Command Bar):
  /// a frame of the background's color with an edge of 1 in the border's color
  /// and the large radius. Its tabs are a row 32 high in the bar's fill over a
  /// line of 1, padded 8, 2 apart; a tab is padded 12, its label 13 in the
  /// muted foreground, the selected one 600 in the foreground and underlined
  /// with 2. Under them the commands are padded 8 and 4, their groups side by
  /// side, each padded 10 with a line of 1 at its end and its label of 10
  /// under its commands; at the end, behind a chevron 28 by 52, what does not fit.
  fn recipe() -> Recipe {
    use CommandBarPart::{Bar, More, Root, Tab, TabLabel, Tabs};
    let line = || Border::inside(BorderSize::Sm, PaletteColor::Border);
    Recipe::new()
      .base(Root, |s| {
        s.background(PaletteColor::Background)
          .border(line())
          .radius(RadiusSize::Lg)
      })
      .base(Tabs, |s| {
        s.height(32.0)
          .padding_x(8.0)
          .gap(2.0)
          .background(PaletteColor::Bar)
          .underline(line())
          .radius(0.0)
      })
      .base(Tab, |s| s.height(32.0).padding_x(12.0))
      .base(TabLabel, |s| s.font_size(13.0))
      .base(Bar, |s| {
        s.width(Length::Fill).padding_x(4.0).padding_y(8.0).gap(0.0).radius(0.0)
      })
      // The chevron is as high as a row of large commands: 28 by 52.
      .base(More, |s| s.height(52.0))
  }
}

/// The parts of a command of a command bar a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CommandBarCommandPart {
  /// What is pressed: its fill, its radius, its height.
  Root,
  /// A large command's icon over its label.
  Body,
  /// The icon.
  Icon,
  /// The label.
  Label,
}

names!(CommandBarCommandPart {
  Root => "root",
  Body => "body",
  Icon => "icon",
  Label => "label",
});

/// The states of a command of a command bar a recipe styles, besides what
/// the pointer and the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CommandBarCommandState {
  /// Its icon over its label: a command a person reaches for first.
  Large,
  /// It is a toggle, and on.
  On,
  /// It cannot be run.
  Disabled,
}

names!(CommandBarCommandState {
  Large => "large",
  On => "on",
  Disabled => "disabled",
});

/// What builds a command's nodes: a theme's in place of the default one.
pub type CommandBarCommandRenderer = Arc<dyn Fn(CommandBarCommandView) -> Element + Send + Sync>;

/// A command of a command bar: an icon and a label, the icon over the label
/// where it is large (Paste, Image, Sidebar), beside it where it is small (Cut,
/// Copy). One that is on or off is a toggle ([`on`](Self::on)). In the menu of
/// what does not fit it is an item with its icon and label, checked for a
/// toggle. A command that is its icon alone is a [`Button`](super::Button) or a
/// [`Toggle`](super::Toggle) with `icon_only`, which the bar gives its size.
///
/// ```
/// use lurq::ui::{CommandBarCommand, icons};
///
/// let paste = CommandBarCommand::new("Paste").icon(icons::PASTE).large(true).on_select(|| println!("paste"));
/// let table = CommandBarCommand::new("Table").icon(icons::TABLE).large(true).on(true).on_change(|on| println!("{on}"));
/// ```
#[derive(Clone)]
pub struct CommandBarCommand {
  label: Arc<str>,
  icon: Option<Name>,
  large: bool,
  on: Option<bool>,
  disabled: bool,
  on_select: Vec<Arc<dyn Fn() + Send + Sync>>,
  on_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl CommandBarCommand {
  /// A command called `label`, which does nothing until it is given [`on_select`](Self::on_select).
  pub fn new(label: &str) -> Self {
    Self {
      label: Arc::from(label),
      icon: None,
      large: false,
      on: None,
      disabled: false,
      on_select: Vec::new(),
      on_change: Vec::new(),
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// Its icon, by its name in the theme's set.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// Whether its icon stands over its label, in a command as high as two
  /// small ones: what a person reaches for first in a group.
  pub fn large(mut self, large: bool) -> Self {
    self.large = large;
    self
  }

  /// Whether it is on: with it the command is a toggle, picked out while on,
  /// and a press tells [`on_change`](Self::on_change) the other value.
  pub fn on(mut self, on: bool) -> Self {
    self.on = Some(on);
    self
  }

  /// Whether it cannot be run.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// What running it does, by a press, the keyboard or its item in the menu,
  /// after the handlers it has.
  pub fn on_select(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_select.push(Arc::new(handler));
    self
  }

  /// Called for a toggle ([`on`](Self::on)) a person presses, with the value
  /// it is to have: after `on_select`'s handlers.
  pub fn on_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Puts the command in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Draws it as it is under the pointer, held down or focused from the
  /// keyboard, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// Styles one part of this command over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this command.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the command: its item in the menu has it with `-item` after it.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// What running the command does: its handlers, and a toggle's told the other value.
  fn run(&self) -> Arc<dyn Fn() + Send + Sync> {
    let (selects, changes, next) = (self.on_select.clone(), self.on_change.clone(), self.on.map(|on| !on));
    Arc::new(move || {
      for handler in &selects {
        handler();
      }
      if let Some(next) = next {
        for handler in &changes {
          handler(next);
        }
      }
    })
  }

  /// What stands for the command in the menu of what does not fit.
  fn command(&self) -> ToolbarCommand {
    let run = self.run();
    let mut command = ToolbarCommand::new(&self.label)
      .disabled(self.disabled)
      .on_select(move || run());
    if let Some(icon) = self.icon {
      command = command.icon(icon);
    }
    if let Some(on) = self.on {
      command = command.checked(on);
    }
    if let Some(id) = &self.id {
      command = command.id(id.clone());
    }
    if let Some(key) = &self.key {
      command = command.key(key.clone());
    }
    command
  }

  /// Builds the command in `theme`. Converting a command into an element
  /// does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let mut case = Case::new()
      .breakpoint(super::current_breakpoint())
      .state(CommandBarCommandState::Large, self.large)
      .state(CommandBarCommandState::On, self.on == Some(true))
      .state(CommandBarCommandState::Disabled, self.disabled);
    for state in &self.states {
      case = case.state(state.clone(), true);
    }
    let view = CommandBarCommandView {
      run: self.run(),
      label: self.label,
      icon: self.icon,
      large: self.large,
      on: self.on,
      disabled: self.disabled,
      recipe: theme
        .resolve::<CommandBarCommand>(&case, Some(&self.rules))
        .owned_by(self.id.as_ref()),
      forced: self.forced,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<CommandBarCommand>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<CommandBarCommand> for Element {
  fn from(command: CommandBarCommand) -> Self {
    command.build(&current_theme())
  }
}

/// A command in a bar: built in the bar's theme, disabled where the bar is,
/// and an item of the menu where it does not fit.
impl From<CommandBarCommand> for ToolbarItem {
  fn from(command: CommandBarCommand) -> Self {
    ToolbarItem::made(move |theme, disabled| {
      let command = if disabled { command.disabled(true) } else { command };
      let stands_for = command.command();
      (command.build(theme), stands_for)
    })
  }
}

/// A command as a renderer sees it: what it was given, the style of its
/// parts, and what running it does.
pub struct CommandBarCommandView {
  pub label: Arc<str>,
  pub icon: Option<Name>,
  pub large: bool,
  /// Whether it is on, for a toggle; `None` for what is not one.
  pub on: Option<bool>,
  pub disabled: bool,
  /// The style of each part of this command.
  pub recipe: ResolvedRecipe,
  run: Arc<dyn Fn() + Send + Sync>,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl CommandBarCommandView {
  /// The style of a part, as this command is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The command's behavior with the style of its root: what is pressed,
  /// focused and disabled, with nothing in it yet. A toggle says whether it
  /// is pressed to what reads the tree.
  pub fn root(&self) -> crate::components::Button {
    let run = self.run.clone();
    let mut root = pressable(&self.part(CommandBarCommandPart::Root), self.disabled)
      .describe("a11y_name", self.label.clone())
      .on_click(move |_| run());
    if let Some(on) = self.on {
      root = root.describe("a11y_pressed", if on { "true" } else { "false" });
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// How the default renderer builds a command: its icon over its label, or
  /// beside it.
  pub fn render(self) -> Element {
    let icon = self
      .icon
      .map(|name| glyph(&self.part(CommandBarCommandPart::Icon), name, false));
    let label = one_line(&self.part(CommandBarCommandPart::Label), &self.label);
    let root = self.root();
    if self.large {
      let body = icon.into_iter().fold(
        Column::new()
          .part(&self.part(CommandBarCommandPart::Body))
          .align_items(Alignment::Center),
        |body, icon| body.child(icon),
      );
      root.child(body.child(label)).into()
    } else {
      icon
        .into_iter()
        .fold(root.align_items(Alignment::Center), |root, icon| root.child(icon))
        .child(label)
        .into()
    }
  }
}

impl crate::ui::Styled for CommandBarCommand {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for CommandBarCommand {
  const NAME: &'static str = "command_bar_command";
  type Renderer = CommandBarCommandRenderer;

  fn parts() -> crate::ui::Declared {
    CommandBarCommandPart::declared()
  }

  fn states() -> crate::ui::Declared {
    CommandBarCommandState::declared()
  }

  /// The design's commands (`pencil/lurq-design-system.pen`, Command Bar): a
  /// small one is 24 high, padded 8, of the small radius, its icon of 14 and
  /// its label of 12 in the foreground 6 apart; a large one is 56 by 52, of
  /// the middle radius, its icon of 20 over its label of 11, 4 apart. Under
  /// the pointer, and while on, it has the fill of what is picked out; held
  /// down, the pressed veil; focused from the keyboard, the ring inside it.
  fn recipe() -> Recipe {
    use CommandBarCommandPart::{Body, Icon, Label, Root};
    use CommandBarCommandState::{Disabled, Large, On};
    use PaletteColor::{Foreground, Highlight, HighlightForeground, Ring, SecondaryHover, StatePressed};
    let picked_out = |s: super::PartStyle| s.background(Highlight).foreground(HighlightForeground);
    Recipe::new()
      .base(Root, |s| {
        fading(
          s.height(24.0)
            .padding_x(8.0)
            .gap(6.0)
            .radius(RadiusSize::Sm)
            .foreground(Foreground),
        )
      })
      .base(Icon, |s| s.square(IconSize::Sm))
      .base(Label, |s| s.typography(TypographyStyle::Body).font_size(12.0))
      .base(Body, |s| s.gap(4.0))
      .hovered(Root, picked_out)
      .pressed(Root, |s| s.veil(StatePressed))
      .focus_visible(Root, |s| s.outline(Outline::new(BorderSize::Md, Ring).offset(-2.0)))
      .state(Large, Root, |s| {
        s.width(56.0).height(52.0).padding_x(0.0).radius(RadiusSize::Md)
      })
      .state(Large, Icon, |s| s.square(20.0))
      .state(Large, Label, |s| s.font_size(11.0))
      .state(On, Root, picked_out)
      .when(Selector::new().state(On).hovered(), Root, |s| {
        s.background(SecondaryHover)
      })
      .state(Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
  }
}
