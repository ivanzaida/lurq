//! Tabs: a list of tabs of which one is selected, and the panel of the selected one.

use std::{fmt::Display, sync::Arc};

use super::{
  icons,
  menu::{Menu, MenuItem},
  names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{Orientation, fading, glyph, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::{Ctx, Placement},
    theme::{BorderSize, IconSize, OpacityLevel, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle},
  },
  components::{self, Column, Row, ScrollHorizontal, Stack},
  core::{ElementRef, Signal, SignalValue},
  layout::{
    Alignment,
    layout_kind::{Justify, ScrollState},
    scrollbar::ScrollBarStyle,
    text_style::FontWeight,
  },
  node::{CursorIcon, Element, FocusGroup, border::Border, dimension::Dimension, outline::Outline},
};

/// How much of the room it shows a press of a scroll button moves the list of an overflowing form.
const SCROLL_STEP: f32 = 0.8;

/// How the tabs are drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TabsVariant {
  /// A muted rail with the selected tab lifted out of it: the views of one thing, few and short.
  #[default]
  Default,
  /// A row of words over a line, the selected one underlined: the sections of a page, with counts.
  Line,
  /// Round tabs with no rail, the selected one in the primary fill: a filter over a list.
  Pill,
}

names!(TabsVariant {
  Default => "default",
  Line => "line",
  Pill => "pill",
});

/// Which way the list of tabs runs: `Horizontal`, along the top with the panel under it, where Left and Right
/// move the selection; or `Vertical`, down the start side with the panel beside it, where Up and Down do.
pub type TabsOrientation = Orientation;

/// The parts of tabs a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TabsPart {
  /// The list and the panel.
  Root,
  /// The list of tabs: the rail, the row over a line.
  List,
  /// One tab: what is pressed.
  Tab,
  /// The text of a tab.
  Label,
  /// The icon of a tab, before its label.
  Icon,
  /// The count of a tab, after its label: how many things are in it.
  Count,
  /// The line that sets a tab apart from those before it.
  Separator,
  /// The panel of the selected tab.
  Panel,
  /// Of the overflowing form: the row with the list that scrolls, its scroll buttons and the More button.
  Bar,
  /// Of the overflowing form: a button at an end of the list that scrolls it.
  ScrollButton,
  /// The chevron of a scroll button.
  ScrollIcon,
  /// Of the overflowing form: the button at the end that opens the menu of every tab.
  More,
  /// The text of the More button.
  MoreLabel,
  /// The chevron of the More button.
  MoreIcon,
}

names!(TabsPart {
  Root => "root",
  List => "list",
  Tab => "tab",
  Label => "label",
  Icon => "icon",
  Count => "count",
  Separator => "separator",
  Panel => "panel",
  Bar => "bar",
  ScrollButton => "scroll_button",
  ScrollIcon => "scroll_icon",
  More => "more",
  MoreLabel => "more_label",
  MoreIcon => "more_icon",
});

/// The states of tabs a recipe styles, besides what the pointer and the keyboard do to them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TabsState {
  /// Of a tab: it is the selected one.
  Selected,
  /// Of a tab: it cannot be selected.
  Disabled,
  /// The list runs along the top.
  Horizontal,
  /// The list runs down the start side.
  Vertical,
  /// The list scrolls between two scroll buttons, with a menu of every tab at its end.
  Overflow,
}

names!(TabsState {
  Selected => "selected",
  Disabled => "disabled",
  Horizontal => "horizontal",
  Vertical => "vertical",
  Overflow => "overflow",
});

/// What builds tabs' nodes: a theme's in place of the default one.
pub type TabsRenderer = Arc<dyn Fn(TabsView) -> Element + Send + Sync>;

/// What builds the panel of a tab, while it is the selected one.
type PanelBuilder = Arc<dyn Fn() -> Element + Send + Sync>;

/// One tab: its value, what it shows, and its panel.
///
/// ```
/// use lurq::{components::Text, ui::{Tab, icons}};
///
/// let plain: Tab<u8> = (0, "General").into();
/// let full = Tab::new("inbox", "Inbox")
///   .icon(icons::INBOX)
///   .count(12)
///   .panel(|| Text::new("What came in").into());
/// ```
pub struct Tab<T> {
  value: T,
  label: Arc<str>,
  icon: Option<Name>,
  count: Option<Arc<str>>,
  disabled: bool,
  separated: bool,
  panel: Option<PanelBuilder>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<T> Tab<T> {
  /// A tab of `value`, called `label`.
  pub fn new(value: T, label: &str) -> Self {
    Self {
      value,
      label: Arc::from(label),
      icon: None,
      count: None,
      disabled: false,
      separated: false,
      panel: None,
      id: None,
      key: None,
    }
  }

  /// An icon before the label, by its name in the theme's set.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// A count after the label: how many things are in the tab. "12", "99+".
  pub fn count(mut self, count: impl Display) -> Self {
    self.count = Some(Arc::from(count.to_string()));
    self
  }

  /// Whether the tab cannot be selected: it is dimmed, and the arrow keys pass it by.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Whether a line before the tab sets it apart from those before it: the archived after the rest.
  pub fn separated(mut self, separated: bool) -> Self {
    self.separated = separated;
    self
  }

  /// What the tab shows while it is the selected one, built only then.
  pub fn panel(mut self, panel: impl Fn() -> Element + Send + Sync + 'static) -> Self {
    self.panel = Some(Arc::new(panel));
    self
  }

  /// The id of the tab: what is pressed. Its item in the menu of the
  /// overflowing form has this id with `-item` after it.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// The key of the tab, among the others.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }
}

impl<T> From<(T, &str)> for Tab<T> {
  fn from((value, label): (T, &str)) -> Self {
    Self::new(value, label)
  }
}

/// What the overflowing form of tabs keeps from one render to the next:
/// where its list is scrolled, the button its menu hangs from, and whether
/// that menu is open. Mounted tabs keep it; tabs converted into an element
/// on their own have one that starts again at each build.
#[derive(Clone)]
struct Kept {
  scroll: ScrollState,
  anchor: ElementRef,
  open: Signal<bool>,
}

impl Kept {
  fn new(open: Signal<bool>) -> Self {
    Self {
      scroll: ScrollState::new(),
      anchor: ElementRef::new(),
      open,
    }
  }
}

/// One tab as it is built: what it shows, whether it is the selected one, and what a press of it does.
#[derive(Clone)]
pub struct TabEntry {
  pub label: Arc<str>,
  pub icon: Option<Name>,
  pub count: Option<Arc<str>>,
  pub disabled: bool,
  pub separated: bool,
  pub selected: bool,
  select: Arc<dyn Fn() + Send + Sync>,
  panel: Option<PanelBuilder>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

/// What is told the value of the tab a person selected.
type Handler<T> = Arc<dyn Fn(&T) + Send + Sync>;

/// What sets the signal of bound tabs to a value, and says whether it was another.
type Write<T> = Arc<dyn Fn(&T) -> bool + Send + Sync>;

/// The words of the controls of the overflowing form, which lurq writes itself.
#[derive(Clone)]
struct Words {
  back: Arc<str>,
  on: Arc<str>,
  more: Option<Arc<str>>,
}

/// Tabs: a list of tabs of which one is selected, and under it (or beside
/// it) the panel of the selected one. The sections of a settings page, the
/// views of a record, the filters over a list.
///
/// ```
/// use lurq::{
///   components::Text,
///   core::Signal,
///   ui::{Tab, Tabs, TabsVariant},
/// };
///
/// # fn build(section: Signal<&'static str>) {
/// let tabs = Tabs::new(
///   section,
///   [
///     Tab::new("general", "General").panel(|| Text::new("General").into()),
///     Tab::new("appearance", "Appearance").panel(|| Text::new("Appearance").into()),
///     Tab::new("advanced", "Advanced").disabled(true),
///   ],
/// )
/// .variant(TabsVariant::Line)
/// .on_change(|section| println!("{section}"));
/// // The app holds which is selected, and is told the tab a person selects.
/// let filter = Tabs::controlled(0, [(0, "All"), (1, "Open"), (2, "Closed")], |index| println!("{index}"));
/// # }
/// ```
///
/// Exactly one tab is selected: the app's, in the signal or the value it
/// gives. The list is one stop of Tab, at the selected tab; the arrow keys
/// of its orientation select the next tab and the one before, Home and End
/// the first and the last, and a disabled one is passed by. Tab from the
/// list goes to the panel. The panel of a tab is built only while the tab
/// is selected.
pub struct Tabs<T = ()> {
  tabs: Vec<Tab<T>>,
  selected: Option<T>,
  write: Option<Write<T>>,
  on_change: Vec<Handler<T>>,
  variant: Name,
  orientation: TabsOrientation,
  disabled: bool,
  manual: bool,
  label: Option<Arc<str>>,
  overflow: bool,
  words: Words,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> Tabs<T> {
  fn of(selected: T, tabs: impl IntoIterator<Item = impl Into<Tab<T>>>, write: Option<Write<T>>) -> Self {
    Self {
      tabs: tabs.into_iter().map(Into::into).collect(),
      selected: Some(selected),
      write,
      on_change: Vec::new(),
      variant: TabsVariant::default().into(),
      orientation: TabsOrientation::default(),
      disabled: false,
      manual: false,
      label: None,
      overflow: false,
      words: Words {
        back: Arc::from("Scroll back"),
        on: Arc::from("Scroll on"),
        more: None,
      },
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// Tabs whose selected tab is the one with the signal's value. Selecting another sets the signal.
  pub fn new(value: Signal<T>, tabs: impl IntoIterator<Item = impl Into<Tab<T>>>) -> Self
  where
    T: SignalValue,
  {
    let now = value.get();
    let write: Write<T> = Arc::new(move |next: &T| {
      let other = value.get_untracked() != *next;
      if other {
        value.set(next.clone());
      }
      other
    });
    Self::of(now, tabs, Some(write))
  }

  /// Tabs whose state the app holds: the tab of `value` is the selected
  /// one, and `on_change` is told the value of each other one a person selects. The tabs select nothing
  /// themselves.
  pub fn controlled(
    value: T,
    tabs: impl IntoIterator<Item = impl Into<Tab<T>>>,
    on_change: impl Fn(&T) + Send + Sync + 'static,
  ) -> Self {
    Self::of(value, tabs, None).on_change(on_change)
  }

  /// Called when a person selects a tab other than the selected one, by a press, the arrow keys or the menu of
  /// the overflowing form, with its value: after the signal is set where the tabs are bound. A value the app sets
  /// is told to none of them, and so is a press of the selected tab. A second handler is called after the first.
  pub fn on_change(mut self, handler: impl Fn(&T) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// A [`TabsVariant`], or a variant the theme's recipe names.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// Which way the list runs: along the top, or down the start side with the panel beside it.
  pub fn orientation(mut self, orientation: TabsOrientation) -> Self {
    self.orientation = orientation;
    self
  }

  /// Whether no tab can be selected.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Whether the arrow keys move the focus alone, and Enter or Space
  /// selects the tab it is on: for panels that are slow to build. Otherwise
  /// the selection follows the focus.
  pub fn manual(mut self, manual: bool) -> Self {
    self.manual = manual;
    self
  }

  /// What the list is called, to what reads the tree: "Settings".
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Draws a list that runs along the top as the overflowing form: the list
  /// scrolls between two scroll buttons, and a More button at its end opens
  /// a menu of every tab. Tabs put in the tree with [`mount`](Self::mount)
  /// keep where the list is scrolled and whether the menu is open; converted
  /// into an element on their own they are its still form, whose menu does
  /// not open. A list that runs down scrolls with what holds it, and takes no
  /// notice of this.
  pub fn overflow(mut self, overflow: bool) -> Self {
    self.overflow = overflow;
    self
  }

  /// What the scroll button at the start of the overflowing list is called, to what reads the tree: "Scroll back".
  pub fn scroll_back_label(mut self, label: &str) -> Self {
    self.words.back = Arc::from(label);
    self
  }

  /// What the scroll button at the end of the overflowing list is called, to what reads the tree: "Scroll on".
  pub fn scroll_on_label(mut self, label: &str) -> Self {
    self.words.on = Arc::from(label);
    self
  }

  /// What the More button of the overflowing form says, and what it is called to what reads the tree. It says
  /// "More", and is called "More tabs", unless told otherwise.
  pub fn more_label(mut self, label: &str) -> Self {
    self.words.more = Some(Arc::from(label));
    self
  }

  /// Puts the tabs in a state the theme's recipe names, or takes them out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of these tabs over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for these tabs.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the tab Tab would stop on (the selected one, else the first that is
  /// not disabled) as it is under the pointer, held down or focused from the
  /// keyboard, with nothing pointing at it: for a gallery or a test. The other
  /// tabs, the panel and the buttons of the overflowing form are drawn as they
  /// are.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the root: the list and the panel. In the overflowing form
  /// the scroll buttons have this id with `-back` and `-on` after it, and
  /// the More button with `-more`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// The key of the tabs, among what is beside them.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The tabs with what a press of each does, of no type of the app's.
  fn prepare(mut self) -> Prepared {
    let selected = self.selected.take();
    let (write, handlers): (_, Arc<[Handler<T>]>) = (self.write, self.on_change.into());
    let tabs = self
      .tabs
      .into_iter()
      .map(|tab| {
        let chosen = selected.as_ref() == Some(&tab.value);
        let (value, write, handlers) = (tab.value, write.clone(), handlers.clone());
        TabEntry {
          label: tab.label,
          icon: tab.icon,
          count: tab.count,
          disabled: tab.disabled,
          separated: tab.separated,
          selected: chosen,
          // A press of the selected tab asks for nothing, and tells nothing.
          select: Arc::new(move || {
            if chosen || write.as_ref().is_some_and(|write| !write(&value)) {
              return;
            }
            for handler in handlers.iter() {
              handler(&value);
            }
          }),
          panel: tab.panel,
          id: tab.id,
          key: tab.key,
        }
      })
      .collect();
    Prepared {
      tabs,
      variant: self.variant,
      orientation: self.orientation,
      disabled: self.disabled,
      manual: self.manual,
      label: self.label,
      overflow: self.overflow,
      words: self.words,
      states: self.states,
      rules: self.rules,
      forced: self.forced,
      id: self.id,
      key: self.key,
    }
  }

  /// Puts the tabs in the tree of `ctx`'s component, in the theme of what is
  /// being rendered, where their overflowing form keeps where its list is
  /// scrolled and whether its menu is open from one render to the next.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let mut tabs = self.prepare();
    let key = tabs.key.take();
    let mut element = ctx.mount::<TabsHost>(TabsProps { tabs });
    if let Some(key) = key {
      element = element.key(key);
    }
    element
  }

  /// Builds the tabs in `theme`, with nothing kept: the overflowing form is
  /// its still form, scrolled to its start, whose menu does not open.
  /// Converting tabs into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    self.prepare().build(theme, Kept::new(Signal::new(false)))
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> From<Tabs<T>> for Element {
  fn from(tabs: Tabs<T>) -> Self {
    tabs.build(&current_theme())
  }
}

/// Tabs as they are built, once a press of each tab knows what it does.
#[derive(Clone)]
struct Prepared {
  tabs: Vec<TabEntry>,
  variant: Name,
  orientation: TabsOrientation,
  disabled: bool,
  manual: bool,
  label: Option<Arc<str>>,
  overflow: bool,
  words: Words,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Prepared {
  /// The tabs in `theme`, their overflowing form with what `kept` keeps.
  fn build(self, theme: &Theme, kept: Kept) -> Element {
    let vertical = self.orientation.is_vertical();
    let overflow = (self.overflow && !vertical).then_some(kept);
    let case = Case {
      variant: Some(self.variant),
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(TabsState::Horizontal, !vertical)
    .state(TabsState::Vertical, vertical)
    .state(TabsState::Overflow, overflow.is_some());
    // A tab is in the state of the tabs, and in its own: selected, disabled.
    let resolve = |selected: bool, disabled: bool| -> ResolvedRecipe {
      theme.resolve::<Tabs>(
        &case
          .clone()
          .state(TabsState::Selected, selected)
          .state(TabsState::Disabled, disabled),
        Some(&self.rules),
      )
    };
    let tab_refs = match overflow {
      Some(_) => self.tabs.iter().map(|_| ElementRef::new()).collect(),
      None => Vec::new(),
    };
    let view = TabsView {
      tabs: self.tabs,
      variant: self.variant,
      orientation: self.orientation,
      disabled: self.disabled,
      manual: self.manual,
      label: self.label,
      recipes: [
        resolve(false, false),
        resolve(true, false),
        resolve(false, true),
        resolve(true, true),
      ],
      list_ref: ElementRef::new(),
      tab_refs,
      overflow,
      words: self.words,
      forced: self.forced,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Tabs>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

#[derive(Clone)]
struct TabsProps {
  tabs: Prepared,
}

/// The tabs are prepared anew by every render of what holds them, so props are
/// never the same as before.
impl PartialEq for TabsProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for TabsProps {}

/// What mounted tabs keep from one render to the next: what their overflowing form keeps.
struct TabsHost {
  kept: Kept,
}

impl Component for TabsHost {
  type Props = TabsProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      kept: Kept::new(ctx.signal(false)),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let TabsProps { tabs } = ctx.props::<TabsProps>().clone();
    tabs.build(&current_theme(), self.kept.clone())
  }
}

/// Tabs as a renderer sees them: the tabs, the style of their parts, and
/// their behavior ready to be dressed.
pub struct TabsView {
  pub tabs: Vec<TabEntry>,
  pub variant: Name,
  pub orientation: TabsOrientation,
  pub disabled: bool,
  pub manual: bool,
  pub label: Option<Arc<str>>,
  /// The recipe of a tab neither selected nor disabled, then selected, disabled, and both. The first is
  /// every other part's.
  recipes: [ResolvedRecipe; 4],
  overflow: Option<Kept>,
  words: Words,
  /// Of the overflowing form, where the list and each tab were last laid out: what scrolls a tab chosen from
  /// the menu into view.
  list_ref: ElementRef,
  tab_refs: Vec<ElementRef>,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl TabsView {
  /// The style of a part that is no tab's, as these tabs are to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipes[0].part(part)
  }

  /// The style of a part of the tab at `index`: in the states of that tab, and in the interaction forced where
  /// it is the tab Tab stops on.
  pub fn tab_part(&self, index: usize, part: impl Into<Name>) -> ResolvedPart {
    let tab = &self.tabs[index];
    let disabled = self.disabled || tab.disabled;
    let forced = self.forced.filter(|_| self.entry() == Some(index));
    self.recipes[usize::from(tab.selected) + 2 * usize::from(disabled)]
      .part(part)
      .forced(forced)
  }

  /// The tab Tab stops on: the selected one, else the first that is not disabled. None where all are.
  pub fn entry(&self) -> Option<usize> {
    if self.disabled {
      return None;
    }
    let open = |tab: &TabEntry| !tab.disabled;
    self
      .tabs
      .iter()
      .position(|tab| tab.selected && open(tab))
      .or_else(|| self.tabs.iter().position(open))
  }

  pub fn theme(&self) -> &Theme {
    self.recipes[0].theme()
  }

  fn vertical(&self) -> bool {
    self.orientation.is_vertical()
  }

  /// Whether the list scrolls between two scroll buttons, with a menu of every tab at its end.
  pub fn overflows(&self) -> bool {
    self.overflow.is_some()
  }

  /// The tab at `index`: what is pressed, with its icon, label and count. It
  /// is a tab to what reads the tree, called by its label, and says whether
  /// it is the selected one; the list's arrow keys move to it unless it is disabled.
  pub fn tab(&self, index: usize) -> components::Button {
    let entry = &self.tabs[index];
    let part = |part: TabsPart| self.tab_part(index, part);
    let select = entry.select.clone();
    let mut tab = pressable(&part(TabsPart::Tab), self.disabled || entry.disabled)
      .focus_entry(entry.selected)
      .describe("a11y_role", "tab")
      .describe("a11y_name", entry.label.clone())
      .describe("a11y_selected", if entry.selected { "true" } else { "false" })
      .on_click(move |_| select());
    if self.vertical() {
      tab = tab.justify(Justify::Start);
    }
    if let Some(name) = entry.icon {
      tab = tab.child(glyph(&part(TabsPart::Icon), name, false));
    }
    let label = part(TabsPart::Label);
    tab = tab.child(label.text(&entry.label).nowrap().part(&label));
    if let Some(count) = &entry.count {
      let style = part(TabsPart::Count);
      tab = tab.child(Row::new().part(&style).child(style.text(count).nowrap()));
    }
    if let Some(id) = &entry.id {
      tab = tab.id(id.clone());
    }
    if let Some(key) = &entry.key {
      tab = tab.key(key.clone());
    }
    if let Some(at) = self.tab_refs.get(index) {
      tab = tab.ref_element(at.clone());
    }
    tab
  }

  /// The line that sets a tab apart from those before it.
  pub fn separator(&self) -> Element {
    Row::new().part(&self.part(TabsPart::Separator)).into()
  }

  /// The list of the tabs, with a separator before each that is set apart:
  /// one stop of Tab, at the selected tab, whose arrow keys move the
  /// selection. It is a tab list to what reads the tree.
  pub fn list(&self) -> Element {
    let group = if self.vertical() {
      FocusGroup::vertical()
    } else {
      FocusGroup::horizontal()
    }
    .activate(!self.manual)
    .one_stop(true);
    let mut children: Vec<Element> = Vec::new();
    for index in 0..self.tabs.len() {
      if index > 0 && self.tabs[index].separated {
        children.push(self.separator());
      }
      children.push(self.tab(index).into());
    }
    let part = self.part(TabsPart::List);
    let orientation = if self.vertical() { "vertical" } else { "horizontal" };
    let name = self.label.clone().unwrap_or_default();
    if self.vertical() {
      let mut list = Column::new()
        .part(&part)
        .align_items(Alignment::Stretch)
        .focus_group(group)
        .overflow_visible()
        .describe("a11y_role", "tablist")
        .describe("a11y_name", name)
        .describe("a11y_orientation", orientation);
      for child in children {
        list = list.child(child);
      }
      list.into()
    } else {
      let mut list = Row::new()
        .part(&part)
        .align_items(Alignment::Center)
        .focus_group(group)
        .describe("a11y_role", "tablist")
        .describe("a11y_name", name)
        .describe("a11y_orientation", orientation);
      // A ring outside a tab is drawn past the list, and the line under the selected tab over the list's own.
      // In a list that scrolls the room it scrolls in cuts both off, and its tabs have their rings inside.
      list = list.overflow_visible();
      if self.overflows() {
        list = list.ref_element(self.list_ref.clone());
      }
      for child in children {
        list = list.child(child);
      }
      list.into()
    }
  }

  /// The panel of the selected tab, built now, if it has one: a stop of Tab
  /// after the list, a tab panel to what reads the tree, called by its tab.
  /// Each tab's is keyed apart, so that one tab's panel never takes the
  /// nodes of another's.
  pub fn panel(&self) -> Option<Element> {
    let (index, entry) = self.tabs.iter().enumerate().find(|(_, entry)| entry.selected)?;
    let build = entry.panel.as_ref()?;
    let mut panel = Column::new()
      .part(&self.part(TabsPart::Panel))
      .tab_index(0)
      .key(format!("tab-panel-{index}"))
      .describe("a11y_role", "tabpanel")
      .describe("a11y_name", entry.label.clone())
      .child(build());
    panel = if self.vertical() {
      panel.flex(1.0)
    } else {
      panel.width(Dimension::Pct(100.0))
    };
    Some(panel.into())
  }

  /// A button at an end of the list of the overflowing form, which scrolls
  /// it most of the way it shows. It is for the pointer: the keyboard moves
  /// along the list with the arrow keys, and the list scrolls to the tab focused.
  fn scroll_button(&self, overflow: &Kept, back: bool) -> components::Button {
    let scroll = overflow.scroll.clone();
    let (icon, name) = if back {
      (icons::CHEVRON_LEFT, self.words.back.clone())
    } else {
      (icons::CHEVRON_RIGHT, self.words.on.clone())
    };
    let mut button = components::Button::empty()
      .focusable(false)
      .part(&self.part(TabsPart::ScrollButton))
      .describe("a11y_name", name)
      .child(glyph(&self.part(TabsPart::ScrollIcon), Name::new(icon), false))
      .on_click(move |_| {
        let by = (scroll.viewport_width() * SCROLL_STEP).max(1.0);
        scroll.scroll_by(if back { -by } else { by }, 0.0);
      });
    if let Some(id) = &self.id {
      button = button.id(format!("{id}-{}", if back { "back" } else { "on" }));
    }
    button
  }

  /// The More button of the overflowing form, and the menu of every tab it
  /// opens: the selected one marked, a disabled one that cannot be chosen.
  fn more(&self, overflow: &Kept) -> (components::Button, Menu) {
    let open = overflow.open.clone();
    let label = self.part(TabsPart::MoreLabel);
    let (says, name) = match &self.words.more {
      Some(words) => (words.clone(), words.clone()),
      None => (Arc::from("More"), Arc::from("More tabs")),
    };
    let mut button = pressable(&self.part(TabsPart::More), self.disabled)
      .ref_element(overflow.anchor.clone())
      .describe("a11y_name", name)
      .describe("a11y_haspopup", "menu")
      .describe("a11y_expanded", if open.get() { "true" } else { "false" })
      .child(label.text(&says).nowrap().part(&label))
      .child(glyph(
        &self.part(TabsPart::MoreIcon),
        Name::new(icons::CHEVRON_DOWN),
        false,
      ))
      .on_click(move |_| open.update(|open| *open = !*open));
    if let Some(id) = &self.id {
      button = button.id(format!("{id}-more"));
    }
    let mut menu = Menu::new(overflow.open.clone(), overflow.anchor.clone()).placement(Placement::BottomEnd);
    for (index, entry) in self.tabs.iter().enumerate() {
      let select = entry.select.clone();
      let reveal = self.reveal(overflow, index);
      let mut item = MenuItem::new(&entry.label)
        .radio(entry.selected)
        .disabled(self.disabled || entry.disabled)
        .on_select(move || {
          select();
          reveal();
        });
      if let Some(icon) = entry.icon {
        item = item.icon(icon);
      }
      if let Some(id) = &entry.id {
        item = item.id(format!("{id}-item"));
      }
      menu = menu.item(item);
    }
    (button, menu)
  }

  /// What scrolls the list of the overflowing form the least that shows the
  /// tab at `index` whole, from where the list and the tab were last laid out.
  fn reveal(&self, overflow: &Kept, index: usize) -> impl Fn() + Send + Sync + 'static {
    let (scroll, list, tab) = (
      overflow.scroll.clone(),
      self.list_ref.clone(),
      self.tab_refs.get(index).cloned(),
    );
    move || {
      let Some(tab) = &tab else {
        return;
      };
      // Both are in the list that scrolls, so the tab's place in it is the one less the other.
      let start = tab.x() - list.x();
      let end = start + tab.width();
      let (at, room) = (scroll.scroll_x(), scroll.viewport_width());
      if start < at {
        scroll.set_scroll(start, scroll.scroll_y());
      } else if end > at + room {
        scroll.set_scroll(end - room, scroll.scroll_y());
      }
    }
  }

  /// The list as the overflowing form draws it: between two scroll buttons,
  /// with the More button and its menu at the end.
  pub fn bar(&self) -> Option<Element> {
    let overflow = self.overflow.as_ref()?;
    let scroll = ScrollHorizontal::new(self.list())
      .with_scroll_state(overflow.scroll.clone())
      .scrollbar(ScrollBarStyle::hidden())
      .flex(1.0);
    let (more, menu) = self.more(overflow);
    // The bar's own box, with its line, is under what the bar holds, which is as high as all of it: the line
    // under the selected tab is drawn over the bar's, as the board draws it, and not above it.
    let fill = Dimension::Pct(100.0);
    Some(
      Stack::new()
        .overflow_visible()
        .child(Row::new().part(&self.part(TabsPart::Bar)))
        .child(
          Row::new()
            .size(fill, fill)
            .align_items(Alignment::Stretch)
            .child(self.scroll_button(overflow, true))
            .child(scroll)
            .child(self.scroll_button(overflow, false))
            .child(more)
            .child(menu),
        )
        .into(),
    )
  }

  /// How the default renderer builds tabs: the list (or its overflowing
  /// form), and the panel of the selected tab under it or beside it.
  pub fn render(self) -> Element {
    let list = self.bar().unwrap_or_else(|| self.list());
    let panel = self.panel();
    let root = self.part(TabsPart::Root);
    if self.vertical() {
      let mut row = Row::new()
        .part(&root)
        .align_items(Alignment::Start)
        .overflow_visible()
        .child(list);
      if let Some(panel) = panel {
        row = row.child(panel);
      }
      if let Some(id) = &self.id {
        row = row.id(id.clone());
      }
      if let Some(key) = &self.key {
        row = row.key(key.clone());
      }
      row.into()
    } else {
      let mut column = Column::new()
        .part(&root)
        .align_items(Alignment::Start)
        .overflow_visible()
        .child(list);
      if let Some(panel) = panel {
        column = column.child(panel);
      }
      if let Some(id) = &self.id {
        column = column.id(id.clone());
      }
      if let Some(key) = &self.key {
        column = column.key(key.clone());
      }
      column.into()
    }
  }
}

/// Every tabs have the one recipe and renderer, whatever their values are.
impl<T: Send + Sync + 'static> Themed for Tabs<T> {
  const NAME: &'static str = "tabs";
  type Renderer = TabsRenderer;

  /// The design's tabs (`pencil/lurq-design-system.pen`, Tabs, Tabs/Line,
  /// Tabs/Pill, Tabs/Vertical, Tabs/Overflow).
  ///
  /// - The default: a muted rail of the large radius, padded 3, its tabs 2 apart; a tab is 28 high, padded 12, of the
  ///   middle radius, its label 13/500 in the muted foreground and its count 12 beside it, 6 apart. The selected one is
  ///   the background's color with the small shadow, its label in the foreground.
  /// - Line: a row 44 high over a line of 1 in the border's color, its tabs 28 apart; a label is 14 in the muted
  ///   foreground, its count 13 at 0.7, 8 apart. The selected one is underlined with 2 in the foreground, its label 500
  ///   in the foreground and its count whole. A separator is 1 by 16 in the border's color. Down the side
  ///   (Tabs/Vertical) the list is at least 180 wide with its line at the start, a tab 32 high, padded 12, its label
  ///   13, the selected one marked at the start.
  /// - Pill: round tabs 4 apart, 30 high, padded 14, the label 13/500; the selected one in the primary fill.
  /// - The overflowing form (Tabs/Overflow): a row 40 high, the list between two scroll buttons 28 wide with chevrons
  ///   of 14 in the foreground, its tabs 20 apart with labels of 13; at the end a More button set apart by a line,
  ///   padded 8 and 4, its label 13 and a chevron of 14 in the muted foreground, 4 apart.
  fn recipe() -> Recipe {
    use PaletteColor::{Background, Foreground, Muted, MutedForeground, Primary, PrimaryForeground, Ring};
    use TabsPart::{
      Bar, Count, Icon, Label, List, More, MoreIcon, MoreLabel, Panel, Root, ScrollButton, ScrollIcon, Separator, Tab,
    };
    use TabsState::{Disabled, Horizontal, Overflow, Selected, Vertical};
    use TabsVariant::{Default, Line, Pill};
    let when = |variant: TabsVariant, states: &[TabsState]| {
      states.iter().fold(Selector::new().variant(variant), |selector, state| {
        selector.state(*state)
      })
    };
    let line = |width: BorderSize, color: PaletteColor| Border::inside(width, color);
    Recipe::new()
      .base(Root, |s| s.gap(16.0).width(Length::Fill))
      .base(Tab, |s| fading(s.foreground(MutedForeground).cursor(CursorIcon::Pointer)))
      .base(Label, |s| s.typography(TypographyStyle::Label))
      .base(Icon, |s| s.square(IconSize::Sm))
      .base(Count, |s| s.typography(TypographyStyle::Caption).foreground(MutedForeground))
      .base(Separator, |s| s.width(1.0).height(16.0).background(PaletteColor::Border))
      .base(Panel, |s| s.radius(RadiusSize::Md))
      .base(Bar, |s| s.width(Length::Fill))
      .base(ScrollButton, |s| {
        fading(s.width(28.0).foreground(Foreground).cursor(CursorIcon::Pointer))
      })
      .base(ScrollIcon, |s| s.square(IconSize::Sm))
      .base(More, |s| {
        fading(
          s.gap(4.0)
            .padding_start(8.0)
            .padding_end(4.0)
            .foreground(Foreground)
            .cursor(CursorIcon::Pointer),
        )
      })
      .base(MoreLabel, |s| s.typography(TypographyStyle::Body).font_size(13.0))
      .base(MoreIcon, |s| s.square(IconSize::Sm).foreground(MutedForeground))
      .hovered(Tab, |s| s.foreground(Foreground))
      .focus_visible(Tab, |s| s.focus_ring())
      // A panel's content runs to its edges: its ring stands 4 out, clear of the text.
      .focus_visible(Panel, |s| s.outline(Outline::new(BorderSize::Md, Ring).offset(4.0)))
      .focus_visible(More, |s| s.outline(Outline::new(BorderSize::Md, Ring).offset(-2.0)))
      .state(Vertical, Root, |s| s.gap(24.0))
      .state(Vertical, Separator, |s| s.width(Length::Auto).height(1.0))
      .state(Selected, Tab, |s| s.foreground(Foreground))
      .state(Disabled, Tab, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
      // A tab of a list that scrolls has its ring inside it, where the list does not cut it off.
      .when(Selector::new().state(Overflow).focus_visible(), Tab, |s| {
        s.outline(Outline::new(BorderSize::Md, Ring).offset(-2.0))
      })
      // The default: a muted rail, the selected tab lifted out of it.
      .variant(Default, List, |s| {
        s.padding_x(3.0)
          .padding_y(3.0)
          .gap(2.0)
          .radius(RadiusSize::Lg)
          .background(Muted)
      })
      .variant(Default, Tab, |s| s.height(28.0).padding_x(12.0).gap(6.0).radius(RadiusSize::Md))
      // Each form's tab is of the density's height, the steps of a row from the board's.
      .row_density(Selector::new().variant(Default), Tab, 28.0)
      .when(when(Default, &[Selected]), Tab, |s| {
        s.background(Background).shadow(ShadowStyle::Sm)
      })
      // Line: a row over a line, the selected tab underlined.
      .variant(Line, Tab, |s| s.gap(8.0))
      .variant(Line, Label, |s| s.typography(TypographyStyle::Body))
      .variant(Line, Count, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .opacity(OpacityLevel::Muted)
      })
      .when(when(Line, &[Selected]), Label, |s| s.weight(FontWeight::Medium))
      .when(when(Line, &[Selected]), Count, |s| s.opacity(1.0))
      .when(when(Line, &[Horizontal]), List, |s| {
        s.height(44.0)
          .width(Length::Fill)
          .gap(28.0)
          .underline(line(BorderSize::Sm, PaletteColor::Border))
      })
      .when(when(Line, &[Horizontal]), Tab, |s| s.height(44.0))
      .row_density(when(Line, &[Horizontal]), List, 44.0)
      .row_density(when(Line, &[Horizontal]), Tab, 44.0)
      .when(when(Line, &[Horizontal, Selected]), Tab, |s| {
        s.underline(line(BorderSize::Md, Foreground))
      })
      .when(when(Line, &[Vertical]), List, |s| {
        s.min_width(180.0).start_line(line(BorderSize::Sm, PaletteColor::Border))
      })
      .when(when(Line, &[Vertical]), Tab, |s| s.height(32.0).padding_x(12.0))
      .row_density(when(Line, &[Vertical]), Tab, 32.0)
      .when(when(Line, &[Vertical]), Label, |s| s.font_size(13.0))
      .when(when(Line, &[Vertical, Selected]), Tab, |s| s.start_line(line(BorderSize::Md, Foreground)))
      .when(when(Line, &[Overflow]), Bar, |s| {
        s.height(40.0).underline(line(BorderSize::Sm, PaletteColor::Border))
      })
      .when(when(Line, &[Overflow]), List, |s| s.height(40.0).width(Length::Auto).gap(20.0))
      .when(when(Line, &[Overflow]), Tab, |s| s.height(40.0))
      .row_density(when(Line, &[Overflow]), Bar, 40.0)
      .row_density(when(Line, &[Overflow]), List, 40.0)
      .row_density(when(Line, &[Overflow]), Tab, 40.0)
      .when(when(Line, &[Overflow]), Label, |s| s.font_size(13.0))
      .when(when(Line, &[Overflow]), More, |s| s.start_line(line(BorderSize::Sm, PaletteColor::Border)))
      // Pill: round tabs, the selected one in the primary fill.
      .variant(Pill, List, |s| s.gap(4.0))
      .variant(Pill, Tab, |s| s.height(30.0).padding_x(14.0).gap(6.0).radius(RadiusSize::Full))
      .row_density(Selector::new().variant(Pill), Tab, 30.0)
      .when(when(Pill, &[Selected]), Tab, |s| {
        s.background(Primary).foreground(PrimaryForeground)
      })
      .when(when(Pill, &[Selected]).hovered(), Tab, |s| s.foreground(PrimaryForeground))
      .when(when(Pill, &[Selected]), Count, |s| {
        s.foreground(PrimaryForeground).opacity(OpacityLevel::Muted)
      })
      // A disabled tab does not light up under the pointer.
      .when(Selector::new().state(Disabled).hovered(), Tab, |s| s.foreground(MutedForeground))
      .when(Selector::new().state(Selected).state(Disabled).hovered(), Tab, |s| {
        s.foreground(Foreground)
      })
      .when(when(Pill, &[Selected, Disabled]).hovered(), Tab, |s| {
        s.foreground(PrimaryForeground)
      })
  }
}
