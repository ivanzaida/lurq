//! A dock layout: a tree of splits whose leaves are groups of panels shown as tabs, which a person rearranges by
//! dragging a tab or a header onto a group or an edge, floats over the layout, hides into a strip at an edge,
//! maximizes and closes; its arrangement a value the app holds, stores and restores ([`DockState`]).

mod state;

use std::{
  collections::{HashMap, HashSet},
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
  },
};

pub use state::*;

use super::{
  context_menu::ContextMenu,
  document_tabs::{DocumentTab, DocumentTabs, DocumentTabsPart},
  floating_panel::{DockHooks, FloatingPanel, FloatingPanels, PanelHeader, button_style, header_button},
  icon::Icon,
  icons,
  menu::MenuItem,
  names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{Orientation, fading, one_line, pressable},
  splitter::{Splitter, SplitterPane},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{DragEvent, KeyboardEvent, MouseButtonMask},
    theme::{BorderSize, IconSize, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle},
  },
  components::{Column, Row, Stack},
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, FocusGroup, HitTestBehavior, border::Border, dimension::Dimension, outline::Outline},
};

/// How far the pointer moves before a press is a drag: what is pressed and let go is a press.
const DRAG_START: f32 = 4.0;

/// The side of a zone of the indicator, of the board's 32.
const ZONE: f32 = 32.0;

/// How far a square of the layout's own edge stands from that edge.
const EDGE_INSET: f32 = 8.0;

/// The width of a panel shown from a strip, where its least size is less, and its height at the top or the bottom.
const FLYOUT: (f32, f32) = (280.0, 200.0);

/// The parts of a dock layout a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DockLayoutPart {
  /// The layout: what holds its groups, its strips and what floats over them.
  Root,
  /// A group of panels: its bar over the panel it shows.
  Group,
  /// The bar of a group of tool panels shown as tabs in capitals.
  Bar,
  /// A tab of such a bar.
  Tab,
  /// The title of a tab, in capitals.
  Label,
  /// A count after a tab's title (problems: 3).
  Count,
  /// The buttons at the end of a bar.
  Actions,
  /// A button of a bar: maximize, restore, close.
  Button,
  /// Where the panel a group shows is, under its bar.
  Body,
  /// What an area of documents with no document in it shows.
  Empty,
  /// A strip at an edge with a button for each panel hidden there.
  Strip,
  /// A button of a strip.
  StripButton,
  /// A hidden panel shown over the layout beside its strip.
  Flyout,
  /// Where a panel dragged would land, laid over the layout.
  Preview,
  /// One of the squares a dragged panel is let go over: the middle of a group, one of its sides, an edge of the
  /// layout.
  Zone,
}

names!(DockLayoutPart {
  Root => "root",
  Group => "group",
  Bar => "bar",
  Tab => "tab",
  Label => "label",
  Count => "count",
  Actions => "actions",
  Button => "button",
  Body => "body",
  Empty => "empty",
  Strip => "strip",
  StripButton => "strip_button",
  Flyout => "flyout",
  Preview => "preview",
  Zone => "zone",
});

/// The states of a dock layout a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DockLayoutState {
  /// A tab is the one its group shows; a strip's button is the one shown.
  Selected,
  /// A zone of the indicator is the one under the pointer: where the panel lands if it is let go.
  Hot,
  /// A strip stands along the left or the right edge: its buttons one under another.
  Vertical,
  /// A strip stands along the left edge: its line on its inner side, the right.
  Left,
  /// A strip stands along the right edge: its line on its inner side, the left.
  Right,
  /// A strip stands along the top: its line along its bottom.
  Top,
  /// A strip stands along the bottom: its line along its top.
  Bottom,
  /// A group fills the layout.
  Maximized,
}

names!(DockLayoutState {
  Selected => "selected",
  Hot => "hot",
  Vertical => "vertical",
  Left => "left",
  Right => "right",
  Top => "top",
  Bottom => "bottom",
  Maximized => "maximized",
});

/// What builds a dock layout's nodes: a theme's in place of the default one.
pub type DockLayoutRenderer = Arc<dyn Fn(DockLayoutView) -> Element + Send + Sync>;

/// What tells the app a panel's close control was pressed, which it may refuse.
pub struct DockCloseEvent {
  panel: Arc<str>,
  refused: AtomicBool,
}

impl DockCloseEvent {
  /// The id of the panel closed.
  pub fn panel(&self) -> &str {
    &self.panel
  }

  /// Refuses the close: the panel stays where it is (a document with changes not saved asks first).
  pub fn prevent_default(&self) {
    self.refused.store(true, Ordering::Relaxed);
  }

  /// Whether a handler refused the close.
  pub fn default_prevented(&self) -> bool {
    self.refused.load(Ordering::Relaxed)
  }
}

/// A panel of a [`DockLayout`]: its id, which the [`DockState`] names it by, its title, and what it holds.
///
/// ```
/// use lurq::{components::Text, ui::{DockPanel, icons}};
///
/// let terminal = DockPanel::new("terminal", "Terminal")
///   .icon(icons::TERMINAL)
///   .child(Text::new("$ cargo run"))
///   .floatable(false);
/// ```
#[derive(Clone)]
pub struct DockPanel {
  info: PanelInfo,
  children: Vec<Element>,
}

/// A panel but for what it holds: what every render reads of it.
#[derive(Clone)]
struct PanelInfo {
  id: Arc<str>,
  title: Arc<str>,
  icon: Option<Name>,
  close_button: bool,
  floatable: bool,
  hideable: bool,
  movable: bool,
  min_size: (f32, f32),
  count: Option<u32>,
  dirty: bool,
  label: Option<Arc<str>>,
  on_close: Vec<Arc<dyn Fn(&DockCloseEvent) + Send + Sync>>,
}

impl DockPanel {
  /// A panel called `title`, which the state names `id`.
  pub fn new(id: &str, title: &str) -> Self {
    Self {
      info: PanelInfo {
        id: Arc::from(id),
        title: Arc::from(title),
        icon: None,
        close_button: true,
        floatable: true,
        hideable: true,
        movable: true,
        min_size: (120.0, 80.0),
        count: None,
        dirty: false,
        label: None,
        on_close: Vec::new(),
      },
      children: Vec::new(),
    }
  }

  /// The icon of the panel: on its button in a strip, and before its title in the documents' tabs.
  pub fn icon(mut self, icon: impl Into<Name>) -> Self {
    self.info.icon = Some(icon.into());
    self
  }

  /// What the panel holds, after what it has. It is kept while the panel moves: what is typed in it, where it is
  /// scrolled, what is open in it.
  pub fn child(mut self, child: impl Into<Element>) -> Self {
    self.children.push(child.into());
    self
  }

  /// What the panel holds, after what it has.
  pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.children.extend(children.into_iter().map(Into::into));
    self
  }

  /// Whether it has a close control, and its menu a Close: it has unless told.
  pub fn close_button(mut self, close_button: bool) -> Self {
    self.info.close_button = close_button;
    self
  }

  /// Whether it may float over the layout: let go over nothing, or from its menu. It may unless told.
  pub fn floatable(mut self, floatable: bool) -> Self {
    self.info.floatable = floatable;
    self
  }

  /// Whether it may be hidden into a strip at an edge. It may unless told.
  pub fn hideable(mut self, hideable: bool) -> Self {
    self.info.hideable = hideable;
    self
  }

  /// Whether a person may move it at all: drag it, float it, move it from its menu. It may unless told.
  pub fn movable(mut self, movable: bool) -> Self {
    self.info.movable = movable;
    self
  }

  /// The least size of the panel, which the splits that hold it keep: 120 by 80 unless told.
  pub fn min_size(mut self, (width, height): (f32, f32)) -> Self {
    self.info.min_size = (width.max(0.0), height.max(0.0));
    self
  }

  /// A count after its title in a tab of capitals (problems: 3); `None` shows none.
  pub fn count(mut self, count: impl Into<Option<u32>>) -> Self {
    self.info.count = count.into();
    self
  }

  /// Whether what it holds has changes not saved: a dot on its tab in the area of documents.
  pub fn dirty(mut self, dirty: bool) -> Self {
    self.info.dirty = dirty;
    self
  }

  /// What it is called to tooling: its title unless told.
  pub fn label(mut self, label: &str) -> Self {
    self.info.label = Some(Arc::from(label));
    self
  }

  /// Called when its close control or its menu's Close is pressed, before it is taken out of the state; a handler
  /// that refuses (`event.prevent_default()`) keeps it. Each call adds.
  pub fn on_close(mut self, handler: impl Fn(&DockCloseEvent) + Send + Sync + 'static) -> Self {
    self.info.on_close.push(Arc::new(handler));
    self
  }
}

/// What a dock layout writes: the names of its controls, which an app says in its own words.
#[derive(Clone)]
struct Words {
  maximize: Arc<str>,
  restore: Arc<str>,
  close: Arc<str>,
  more: Arc<str>,
  float: Arc<str>,
  hide: Arc<str>,
  put_back: Arc<str>,
  move_left: Arc<str>,
  move_right: Arc<str>,
  move_top: Arc<str>,
  move_bottom: Arc<str>,
  strip: Arc<str>,
}

impl Default for Words {
  fn default() -> Self {
    Self {
      maximize: Arc::from("Maximize"),
      restore: Arc::from("Restore"),
      close: Arc::from("Close"),
      more: Arc::from("More"),
      float: Arc::from("Float"),
      hide: Arc::from("Hide"),
      put_back: Arc::from("Put back"),
      move_left: Arc::from("Move to the left"),
      move_right: Arc::from("Move to the right"),
      move_top: Arc::from("Move to the top"),
      move_bottom: Arc::from("Move to the bottom"),
      strip: Arc::from("Hidden panels"),
    }
  }
}

/// The arrangement of a layout: the app's signal, or the app's value with what it is told.
#[derive(Clone)]
enum Hold {
  Signal(Signal<DockState>),
  Value(DockState),
}

/// Panels an app arranges in groups, splits, floating windows and strips, which a person rearranges: the layout of an
/// editor, its explorer at the start, its documents in the middle and its terminal along the bottom.
///
/// ```
/// use lurq::{
///   app::ctx::Ctx,
///   components::Text,
///   core::Signal,
///   node::Element,
///   ui::{DockLayout, DockNode, DockPanel, DockState},
/// };
///
/// fn editor(ctx: &mut Ctx, layout: Signal<DockState>) -> Element {
///   DockLayout::new(
///     layout,
///     [
///       DockPanel::new("explorer", "Explorer").child(Text::new("crates")),
///       DockPanel::new("blur.rs", "blur.rs").child(Text::new("fn blur() {}")),
///       DockPanel::new("terminal", "Terminal").child(Text::new("$ cargo run")),
///     ],
///   )
///   .on_change(|state| println!("{state}"))
///   .mount(ctx)
/// }
///
/// let first = DockState::new(DockNode::row([
///   DockNode::group(["explorer"]),
///   DockNode::column([
///     DockNode::documents(DockNode::group(["blur.rs"])),
///     DockNode::group(["terminal"]),
///   ])
///   .shares([0.7, 0.3]),
/// ])
/// .shares([0.25, 0.75]));
/// ```
///
/// Its arrangement is a [`DockState`]: plain data the app holds in a signal (`new`) or holds itself (`controlled`),
/// stores and restores. A state is always made valid, never refused: a panel it names that the app no longer gives
/// is left out, one the app gives that it does not name is not shown until the app shows it
/// ([`DockState::show`]), an empty group goes and a split of one folds.
///
/// A group in the area of documents has the documents' tabs; one of tool panels has its panel's header where it holds
/// one, and tabs in capitals with a maximize and a close at its end where it holds more. A tab or a header dragged
/// shows where it can be let go: the middle of a group and its four sides, and the edges of the layout. Let go over
/// nothing it floats; outside the window it is told to the app ([`on_detach`](Self::on_detach)). Escape gives a drag
/// up. What a panel holds is kept while it moves.
pub struct DockLayout {
  hold: Hold,
  panels: Vec<DockPanel>,
  on_change: Vec<Arc<dyn Fn(&DockState) + Send + Sync>>,
  on_detach: Vec<Arc<dyn Fn(&str) + Send + Sync>>,
  empty: Option<Element>,
  label: Arc<str>,
  words: Words,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

/// An element held where what holds it is shared between threads: what a panel holds, what an empty area shows.
type Held = Arc<Mutex<Element>>;

fn held(element: &Held) -> Element {
  element.lock().unwrap().clone()
}

impl DockLayout {
  fn with(hold: Hold, panels: impl IntoIterator<Item = DockPanel>) -> Self {
    Self {
      hold,
      panels: panels.into_iter().collect(),
      on_change: Vec::new(),
      on_detach: Vec::new(),
      empty: None,
      label: Arc::from("Dock layout"),
      words: Words::default(),
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// A layout of `panels` arranged as the signal says; what a person changes is set in it.
  pub fn new(state: Signal<DockState>, panels: impl IntoIterator<Item = DockPanel>) -> Self {
    Self::with(Hold::Signal(state), panels)
  }

  /// A layout of `panels` arranged as the app's `state` says; what a person changes is told to `on_change`, and the
  /// layout shows it once the app gives it back.
  pub fn controlled(
    state: DockState,
    panels: impl IntoIterator<Item = DockPanel>,
    on_change: impl Fn(&DockState) + Send + Sync + 'static,
  ) -> Self {
    Self::with(Hold::Value(state), panels).on_change(on_change)
  }

  /// A panel, after those it has.
  pub fn panel(mut self, panel: DockPanel) -> Self {
    self.panels.push(panel);
    self
  }

  /// Panels, after those it has.
  pub fn panels(mut self, panels: impl IntoIterator<Item = DockPanel>) -> Self {
    self.panels.extend(panels);
    self
  }

  /// Called with the new arrangement when a person changes it: after the signal is set. Not when the app sets it.
  /// Each call adds.
  pub fn on_change(mut self, handler: impl Fn(&DockState) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Called with the id of a panel let go outside the window: the app may open it in a window of its own. The layout
  /// changes nothing by itself. Each call adds.
  pub fn on_detach(mut self, handler: impl Fn(&str) + Send + Sync + 'static) -> Self {
    self.on_detach.push(Arc::new(handler));
    self
  }

  /// What the area of documents shows while no document is in it.
  pub fn empty(mut self, empty: impl Into<Element>) -> Self {
    self.empty = Some(empty.into());
    self
  }

  /// What the layout is called to tooling: "Dock layout" unless told.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Arc::from(label);
    self
  }

  /// The name of the button and the item that maximize a group: "Maximize".
  pub fn maximize_label(mut self, label: &str) -> Self {
    self.words.maximize = Arc::from(label);
    self
  }

  /// The name of the button and the item that give a maximized group its place back: "Restore".
  pub fn restore_label(mut self, label: &str) -> Self {
    self.words.restore = Arc::from(label);
    self
  }

  /// The name of a panel's close control and its menu's item: "Close".
  pub fn close_label(mut self, label: &str) -> Self {
    self.words.close = Arc::from(label);
    self
  }

  /// The name of a panel header's button that opens its menu: "More".
  pub fn more_label(mut self, label: &str) -> Self {
    self.words.more = Arc::from(label);
    self
  }

  /// The menu's item that floats a panel: "Float".
  pub fn float_label(mut self, label: &str) -> Self {
    self.words.float = Arc::from(label);
    self
  }

  /// The menu's item that hides a panel into a strip: "Hide".
  pub fn hide_label(mut self, label: &str) -> Self {
    self.words.hide = Arc::from(label);
    self
  }

  /// The button and the item that put a hidden panel back in its group: "Put back".
  pub fn put_back_label(mut self, label: &str) -> Self {
    self.words.put_back = Arc::from(label);
    self
  }

  /// The menu's item that docks a panel along the left edge: "Move to the left".
  pub fn move_left_label(mut self, label: &str) -> Self {
    self.words.move_left = Arc::from(label);
    self
  }

  /// The menu's item that docks a panel along the right edge: "Move to the right".
  pub fn move_right_label(mut self, label: &str) -> Self {
    self.words.move_right = Arc::from(label);
    self
  }

  /// The menu's item that docks a panel along the top: "Move to the top".
  pub fn move_top_label(mut self, label: &str) -> Self {
    self.words.move_top = Arc::from(label);
    self
  }

  /// The menu's item that docks a panel along the bottom: "Move to the bottom".
  pub fn move_bottom_label(mut self, label: &str) -> Self {
    self.words.move_bottom = Arc::from(label);
    self
  }

  /// What a strip of hidden panels is called to tooling: "Hidden panels".
  pub fn strip_label(mut self, label: &str) -> Self {
    self.words.strip = Arc::from(label);
    self
  }

  /// Puts the layout in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of the layout over what the theme gives it.
  pub fn style(
    mut self,
    part: impl Into<Name>,
    style: impl FnOnce(super::part_style::PartStyle) -> super::part_style::PartStyle,
  ) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this layout.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the shown tab of each bar of capitals as if pointed at, pressed or focused.
  pub fn force(mut self, interaction: impl Into<Option<Interaction>>) -> Self {
    self.forced = interaction.into();
    self
  }

  /// The id of the layout's root; its parts are found by ids after it: `{id}-tab-{panel}`, `{id}-group-{panel}` (a
  /// group by the first of its panels, and `{id}-body-{panel}` its body, which F6 focuses), `{id}-strip-{panel}`,
  /// `{id}-flyout-{panel}`, `{id}-float-{panel}` (a floating group by its first panel), `{id}-maximize-{panel}`,
  /// `{id}-close-{panel}`, and `{id}-split-{path}` a split, by the
  /// indices from the root (`root` for the root), whose dividers are `{id}-split-{path}-divider-{n}`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// What tells the layout apart from what stands beside it.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the layout in `theme` as its state says: a still layout, which keeps nothing a panel holds and is not
  /// dragged. Converting it into an element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let layout = Arc::new(self.prepare());
    let host = Kept::new();
    let raw = layout.raw();
    let state = layout.valid(&raw);
    let mut builder = Builder::new(None, theme, layout, &host, state);
    let element = builder.render();
    builder.finish(element)
  }

  /// Mounts the layout: it keeps what each panel holds while it moves, what is dragged, and what is shown from a
  /// strip.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone().unwrap_or_else(|| Arc::from("dock-layout"));
    ctx.mount_keyed::<DockHost>(
      &key,
      DockProps {
        layout: Arc::new(self.prepare()),
      },
    )
  }

  fn prepare(self) -> Prepared {
    let panels: Vec<PreparedPanel> = self
      .panels
      .into_iter()
      .map(|panel| {
        let content = Column::new()
          .width(Dimension::Pct(100.0))
          .height(Dimension::Pct(100.0))
          .align_items(Alignment::Stretch)
          .with_children(panel.children);
        PreparedPanel {
          content: Arc::new(Mutex::new(content.into())),
          panel: panel.info,
        }
      })
      .collect();
    let index = panels
      .iter()
      .enumerate()
      .map(|(at, panel)| (panel.panel.id.to_string(), at))
      .collect();
    Prepared {
      hold: self.hold,
      panels,
      index,
      on_change: self.on_change.into(),
      on_detach: self.on_detach.into(),
      empty: self.empty.map(|empty| Arc::new(Mutex::new(empty))),
      label: self.label,
      words: self.words,
      states: self.states,
      rules: self.rules,
      forced: self.forced,
      id: self.id,
    }
  }
}

impl From<DockLayout> for Element {
  fn from(layout: DockLayout) -> Self {
    layout.build(&current_theme())
  }
}

/// A panel with what it holds made one element, which every render of the layout gives again: what it holds is
/// built when the app builds the layout, not when the layout renders for a drag.
#[derive(Clone)]
struct PreparedPanel {
  panel: PanelInfo,
  content: Held,
}

/// A layout as the app built it, ready for every render of its host.
struct Prepared {
  hold: Hold,
  panels: Vec<PreparedPanel>,
  index: HashMap<String, usize>,
  on_change: Arc<[Arc<dyn Fn(&DockState) + Send + Sync>]>,
  on_detach: Arc<[Arc<dyn Fn(&str) + Send + Sync>]>,
  empty: Option<Held>,
  label: Arc<str>,
  words: Words,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
}

impl Prepared {
  fn panel(&self, id: &str) -> Option<&PreparedPanel> {
    self.index.get(id).map(|at| &self.panels[*at])
  }

  /// The arrangement as the app holds it now.
  fn raw(&self) -> DockState {
    match &self.hold {
      Hold::Signal(signal) => signal.get(),
      Hold::Value(state) => state.clone(),
    }
  }

  /// `state` made valid for the panels the app gives.
  fn valid(&self, state: &DockState) -> DockState {
    state.clone().valid(|id| self.index.contains_key(id))
  }

  fn id_of(&self, what: &str, panel: &str) -> Option<String> {
    self.id.as_ref().map(|id| format!("{id}-{what}-{panel}"))
  }
}

/// What a person's change of the arrangement does: makes it valid, sets the app's signal and tells the handlers.
#[derive(Clone)]
struct Changer {
  layout: Arc<Prepared>,
  /// The arrangement this render showed: what a change of a layout the app holds starts from.
  shown: DockState,
  written: Arc<Mutex<Option<DockState>>>,
}

impl Changer {
  fn current(&self) -> DockState {
    match &self.layout.hold {
      Hold::Signal(signal) => self.layout.valid(&signal.get_untracked()),
      Hold::Value(_) => self.shown.clone(),
    }
  }

  /// Changes the arrangement by `change`, where it says it changed anything.
  fn change(&self, change: impl FnOnce(&mut DockState) -> bool) {
    let mut next = self.current();
    if !change(&mut next) {
      return;
    }
    let layout = self.layout.clone();
    next.make_valid(|id| layout.index.contains_key(id));
    *self.written.lock().unwrap() = Some(next.clone());
    if let Hold::Signal(signal) = &self.layout.hold {
      signal.set(next.clone());
    }
    for handler in self.layout.on_change.iter() {
      handler(&next);
    }
  }

  /// Closes `panel` unless a handler of its own refuses.
  fn close(&self, panel: &str) {
    if let Some(prepared) = self.layout.panel(panel) {
      let event = DockCloseEvent {
        panel: prepared.panel.id.clone(),
        refused: AtomicBool::new(false),
      };
      for handler in &prepared.panel.on_close {
        handler(&event);
      }
      if event.default_prevented() {
        return;
      }
    }
    let panel = panel.to_owned();
    self.change(move |state| state.close(&panel));
  }
}

/// What is dragged: a panel by its tab or header, or a whole group by its bar's empty room or a floating group by its
/// title bar (named by its first panel).
#[derive(Clone, Debug, PartialEq)]
enum Dragged {
  Panel(String),
  Group(String),
}

impl Dragged {
  fn panel(&self) -> &str {
    match self {
      Dragged::Panel(panel) | Dragged::Group(panel) => panel,
    }
  }
}

/// Where a dragged panel lands: a zone of a group's indicator (its middle or a side), or an edge of the layout.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Zone {
  Center,
  Side(DockSide),
}

#[derive(Clone, Debug, PartialEq)]
enum Target {
  Zone { of: String, zone: Zone },
  Edge(DockSide),
}

/// A drag going on: what, where it is to land, whether it came from a floating group, and whether Escape gave it up.
struct Dragging {
  what: Dragged,
  floating: bool,
  target: Option<Target>,
  cancelled: bool,
  moved: bool,
}

/// What the layer over the layout draws of a drag, in the layout's own coordinates: where the panel would land, the
/// indicator over the group under the pointer, the squares of the layout's edges, and which square is hot.
#[derive(Clone, Copy, Debug, Default, PartialEq, crate::DevtoolsInspectable)]
struct DragView {
  width: f32,
  height: f32,
  preview: bool,
  preview_x: f32,
  preview_y: f32,
  preview_width: f32,
  preview_height: f32,
  indicator: bool,
  indicator_x: f32,
  indicator_y: f32,
  /// The zone of the indicator under the pointer: 0 none, 1 the middle, 2 to 5 the sides in `DockSide::ALL`'s order.
  hot: u8,
  /// The square of the layout's edge under the pointer, as `hot` counts the sides.
  edge_hot: u8,
}

/// Where a group was laid out, what it holds, and whether a press of a key finds it.
#[derive(Clone)]
struct GroupPlace {
  panels: Vec<String>,
  frame: ElementRef,
  body: ElementRef,
}

/// What a mounted layout keeps between renders.
struct Kept {
  /// What the layer over the layout draws of a drag.
  view: Signal<Option<DragView>>,
  /// The drag going on.
  dragging: Arc<Mutex<Option<Dragging>>>,
  /// The hidden panel shown over the layout from its strip.
  flyout: Signal<Option<String>>,
  /// Whether each panel's menu is open.
  menus: Mutex<HashMap<String, Signal<bool>>>,
  /// The layout's box: what a drag's place is reckoned from.
  root: ElementRef,
  /// Where each group was laid out by the last render.
  groups: Arc<Mutex<Vec<GroupPlace>>>,
  /// The arrangement the layout set last, and the one it showed last: another the app set renews the splits.
  written: Arc<Mutex<Option<DockState>>>,
  seen: Mutex<Option<DockState>>,
  generation: AtomicU64,
  /// The place and the size of each floating group while a person moves it, by the generation and its first panel.
  floats: Mutex<HashMap<String, (Signal<(f32, f32)>, Signal<Option<(f32, f32)>>)>>,
  /// The group F6 asked the focus to go to, by its first panel, counted; and the last request the layout carried
  /// out.
  focus: Signal<(u64, String)>,
  focused: AtomicU64,
}

impl Kept {
  fn new() -> Self {
    Self {
      view: Signal::new(None),
      dragging: Arc::new(Mutex::new(None)),
      flyout: Signal::new(None),
      menus: Mutex::new(HashMap::new()),
      root: ElementRef::new(),
      groups: Arc::new(Mutex::new(Vec::new())),
      written: Arc::new(Mutex::new(None)),
      seen: Mutex::new(None),
      generation: AtomicU64::new(0),
      floats: Mutex::new(HashMap::new()),
      focus: Signal::new((0, String::new())),
      focused: AtomicU64::new(0),
    }
  }

  fn menu(&self, panel: &str) -> Signal<bool> {
    self
      .menus
      .lock()
      .unwrap()
      .entry(panel.to_owned())
      .or_insert_with(|| Signal::new(false))
      .clone()
  }
}

/// What a drag does: finds where it would land from where each group was laid out, shows it, and lands it.
#[derive(Clone)]
struct Dragger {
  changer: Changer,
  view: Signal<Option<DragView>>,
  dragging: Arc<Mutex<Option<Dragging>>>,
  root: ElementRef,
  groups: Arc<Mutex<Vec<GroupPlace>>>,
}

impl Dragger {
  /// The square of the side `side` of the layout's own edges, its middle in the layout's coordinates.
  fn edge_square(side: DockSide, width: f32, height: f32) -> (f32, f32) {
    let half = ZONE / 2.0;
    match side {
      DockSide::Left => (EDGE_INSET + half, height / 2.0),
      DockSide::Right => (width - EDGE_INSET - half, height / 2.0),
      DockSide::Top => (width / 2.0, EDGE_INSET + half),
      DockSide::Bottom => (width / 2.0, height - EDGE_INSET - half),
    }
  }

  /// The middle of the zone `zone` of an indicator whose middle is `(x, y)`.
  fn zone_square(zone: Zone, (x, y): (f32, f32)) -> (f32, f32) {
    match zone {
      Zone::Center => (x, y),
      Zone::Side(DockSide::Left) => (x - ZONE, y),
      Zone::Side(DockSide::Right) => (x + ZONE, y),
      Zone::Side(DockSide::Top) => (x, y - ZONE),
      Zone::Side(DockSide::Bottom) => (x, y + ZONE),
    }
  }

  fn over((x, y): (f32, f32), (cx, cy): (f32, f32)) -> bool {
    (x - cx).abs() <= ZONE / 2.0 && (y - cy).abs() <= ZONE / 2.0
  }

  fn side_number(side: DockSide) -> u8 {
    DockSide::ALL.iter().position(|held| *held == side).unwrap_or(0) as u8 + 2
  }

  /// A move of a drag of `what`: where it would land, shown over the layout.
  fn moved(&self, what: Dragged, floating: bool, event: &DragEvent) {
    let mut dragging = self.dragging.lock().unwrap();
    let now = dragging.get_or_insert_with(|| Dragging {
      what: what.clone(),
      floating,
      target: None,
      cancelled: false,
      moved: false,
    });
    if now.cancelled || now.what != what {
      return;
    }
    now.moved |= event.total_delta_x.hypot(event.total_delta_y) >= DRAG_START;
    if !now.moved {
      return;
    }
    let (ox, oy) = (self.root.x(), self.root.y());
    let (width, height) = (self.root.width(), self.root.height());
    let (x, y) = (event.x - ox, event.y - oy);
    let mut view = DragView {
      width,
      height,
      ..DragView::default()
    };
    let mut target = None;
    if !event.outside_window {
      // The layout's own edges first: their squares stand over the groups along them.
      for side in DockSide::ALL {
        if Self::over((x, y), Self::edge_square(side, width, height)) {
          target = Some(Target::Edge(side));
          view.edge_hot = Self::side_number(side);
          let share = 0.25;
          let (px, py, pw, ph) = match side {
            DockSide::Left => (0.0, 0.0, width * share, height),
            DockSide::Right => (width * (1.0 - share), 0.0, width * share, height),
            DockSide::Top => (0.0, 0.0, width, height * share),
            DockSide::Bottom => (0.0, height * (1.0 - share), width, height * share),
          };
          view.preview = true;
          (view.preview_x, view.preview_y, view.preview_width, view.preview_height) = (px, py, pw, ph);
        }
      }
      let groups = self.groups.lock().unwrap();
      let dragged = what.panel().to_owned();
      for group in groups.iter() {
        let frame = group.frame.bounds();
        let inside = event.x >= frame.x
          && event.x < frame.x + frame.width
          && event.y >= frame.y
          && event.y < frame.y + frame.height;
        if !inside || frame.width <= 0.0 {
          continue;
        }
        // A group dragged onto itself, or a panel alone in its group onto that group, lands nowhere new.
        let own = group.panels.iter().any(|panel| *panel == dragged);
        if own && (matches!(what, Dragged::Group(_)) || group.panels.len() == 1) {
          break;
        }
        let body = group.body.bounds();
        let (bx, by) = (body.x - ox, body.y - oy);
        let middle = (bx + body.width / 2.0, by + body.height / 2.0);
        view.indicator = true;
        (view.indicator_x, view.indicator_y) = middle;
        if target.is_some() {
          break;
        }
        let zones = [
          Zone::Center,
          Zone::Side(DockSide::Left),
          Zone::Side(DockSide::Right),
          Zone::Side(DockSide::Top),
          Zone::Side(DockSide::Bottom),
        ];
        for zone in zones {
          if Self::over((x, y), Self::zone_square(zone, middle)) {
            let of = group.panels.first().cloned().unwrap_or_default();
            target = Some(Target::Zone { of, zone });
            view.hot = match zone {
              Zone::Center => 1,
              Zone::Side(side) => Self::side_number(side),
            };
            let (pw, ph) = (body.width, body.height);
            let (px, py, pw, ph) = match zone {
              Zone::Center => (bx, by, pw, ph),
              Zone::Side(DockSide::Left) => (bx, by, pw / 2.0, ph),
              Zone::Side(DockSide::Right) => (bx + pw / 2.0, by, pw / 2.0, ph),
              Zone::Side(DockSide::Top) => (bx, by, pw, ph / 2.0),
              Zone::Side(DockSide::Bottom) => (bx, by + ph / 2.0, pw, ph / 2.0),
            };
            view.preview = true;
            (view.preview_x, view.preview_y, view.preview_width, view.preview_height) = (px, py, pw, ph);
          }
        }
        break;
      }
    }
    now.target = target;
    drop(dragging);
    if self.view.get_untracked() != Some(view) {
      self.view.set(Some(view));
    }
  }

  /// The end of a drag of `what`: it lands where it was shown, floats where it was let go over nothing, or is told to
  /// the app where it was let go outside the window.
  fn ended(&self, what: Dragged, event: &DragEvent) {
    let dragging = self.dragging.lock().unwrap().take();
    if self.view.get_untracked().is_some() {
      self.view.set(None);
    }
    let Some(dragging) = dragging else {
      return;
    };
    if dragging.cancelled || dragging.what != what || !dragging.moved {
      return;
    }
    let panel = what.panel().to_owned();
    if event.outside_window {
      for handler in self.changer.layout.on_detach.iter() {
        handler(&panel);
      }
      return;
    }
    let place = match dragging.target {
      Some(Target::Zone { of, zone: Zone::Center }) => DockPlace::Tab { of },
      Some(Target::Zone {
        of,
        zone: Zone::Side(side),
      }) => DockPlace::Beside { of, side },
      Some(Target::Edge(side)) => DockPlace::Edge(side),
      None => {
        // A floating group let go over nothing stays where its own drag put it.
        let floatable = self
          .changer
          .layout
          .panel(&panel)
          .is_some_and(|prepared| prepared.panel.floatable);
        if dragging.floating || !floatable {
          return;
        }
        let (x, y) = (event.x - self.root.x(), event.y - self.root.y());
        DockPlace::Float {
          position: ((x - 24.0).max(0.0).round(), (y - 15.0).max(0.0).round()),
          size: None,
        }
      }
    };
    self.changer.change(move |state| match what {
      Dragged::Panel(panel) => state.move_panel(&panel, place),
      Dragged::Group(of) => state.move_group(&of, place),
    });
  }

  /// Escape: the drag going on is given up, and nothing moves when it is let go.
  fn cancel(&self) -> bool {
    let mut dragging = self.dragging.lock().unwrap();
    let Some(now) = dragging.as_mut() else {
      return false;
    };
    if now.cancelled {
      return false;
    }
    now.cancelled = true;
    drop(dragging);
    if self.view.get_untracked().is_some() {
      self.view.set(None);
    }
    true
  }

  /// The handlers of a drag of `what` on `element`.
  fn on<E: DragSource>(&self, element: E, what: Dragged) -> E {
    let (moving, ending) = (self.clone(), self.clone());
    let (moved, ended) = (what.clone(), what);
    element
      .drag_buttons()
      .drag_move(move |event: DragEvent| moving.moved(moved.clone(), false, &event))
      .drag_end(move |event: DragEvent| ending.ended(ended.clone(), &event))
  }
}

/// What a drag can start on.
trait DragSource: Sized {
  fn drag_buttons(self) -> Self;
  fn drag_move(self, handler: impl Fn(DragEvent) + Send + Sync + 'static) -> Self;
  fn drag_end(self, handler: impl Fn(DragEvent) + Send + Sync + 'static) -> Self;
}

macro_rules! drag_source {
  ($($kind:ty),*) => {
    $(
      impl DragSource for $kind {
        fn drag_buttons(self) -> Self {
          self.start_drag_buttons(MouseButtonMask::LEFT)
        }
        fn drag_move(self, handler: impl Fn(DragEvent) + Send + Sync + 'static) -> Self {
          self.on_drag_move(handler)
        }
        fn drag_end(self, handler: impl Fn(DragEvent) + Send + Sync + 'static) -> Self {
          self.on_drag_end(handler)
        }
      }
    )*
  };
}

drag_source!(Row, Column, crate::components::Button);

/// A layout as one render builds it.
struct Builder<'a> {
  ctx: Option<&'a mut Ctx>,
  theme: Theme,
  layout: Arc<Prepared>,
  kept: &'a Kept,
  state: DockState,
  changer: Changer,
  dragger: Dragger,
  recipe: ResolvedRecipe,
  /// The panels this render mounted: every other one is kept offstage.
  mounted: HashSet<String>,
  groups: Vec<GroupPlace>,
  generation: u64,
}

impl<'a> Builder<'a> {
  fn new(ctx: Option<&'a mut Ctx>, theme: &Theme, layout: Arc<Prepared>, kept: &'a Kept, state: DockState) -> Self {
    let changer = Changer {
      layout: layout.clone(),
      shown: state.clone(),
      written: kept.written.clone(),
    };
    let dragger = Dragger {
      changer: changer.clone(),
      view: kept.view.clone(),
      dragging: kept.dragging.clone(),
      root: kept.root.clone(),
      groups: kept.groups.clone(),
    };
    let recipe = theme.resolve::<DockLayout>(&Self::case(&layout, &[]), Some(&layout.rules));
    Self {
      ctx,
      theme: theme.clone(),
      generation: kept.generation.load(Ordering::Relaxed),
      layout,
      kept,
      state,
      changer,
      dragger,
      recipe,
      mounted: HashSet::new(),
      groups: Vec::new(),
    }
  }

  fn case(layout: &Prepared, states: &[(DockLayoutState, bool)]) -> Case {
    let mut case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: layout.states.clone(),
    };
    for (state, on) in states {
      case = case.state(*state, *on);
    }
    case
  }

  fn part(&self, part: DockLayoutPart) -> ResolvedPart {
    self.recipe.part(part)
  }

  fn part_in(&self, part: DockLayoutPart, states: &[(DockLayoutState, bool)]) -> ResolvedPart {
    self
      .theme
      .resolve::<DockLayout>(&Self::case(&self.layout, states), Some(&self.layout.rules))
      .part(part)
  }

  fn mounted(&self) -> bool {
    self.ctx.is_some()
  }

  /// What `panel` holds: mounted in the layout's own tree of components, by its id, so that it is kept wherever it is
  /// shown; the panel's element as it is in a still layout.
  fn content(&mut self, panel: &str) -> Element {
    let Some(prepared) = self.layout.panel(panel) else {
      return Element::new();
    };
    let content = prepared.content.clone();
    self.mounted.insert(panel.to_owned());
    match self.ctx.as_deref_mut() {
      Some(ctx) => ctx.mount_keyed_offstage::<PanelHost>(&format!("dock-panel-{panel}"), PanelProps { content }, true),
      None => held(&content),
    }
  }

  /// Every panel not shown by this render, kept offstage: what it holds is there when it is shown again.
  fn offstage(&mut self) {
    let Some(ctx) = self.ctx.as_deref_mut() else {
      return;
    };
    for prepared in &self.layout.panels {
      if self.mounted.contains(&*prepared.panel.id) {
        continue;
      }
      let _ = ctx.mount_keyed_offstage::<PanelHost>(
        &format!("dock-panel-{}", prepared.panel.id),
        PanelProps {
          content: prepared.content.clone(),
        },
        false,
      );
    }
  }

  fn title(&self, panel: &str) -> Arc<str> {
    self
      .layout
      .panel(panel)
      .map(|prepared| prepared.panel.title.clone())
      .unwrap_or_else(|| Arc::from(panel))
  }

  fn panel_of(&self, panel: &str) -> Option<PanelInfo> {
    self.layout.panel(panel).map(|prepared| prepared.panel.clone())
  }

  /// The least size of the panels of `group`.
  fn least(&self, group: &DockGroup) -> (f32, f32) {
    group.panels.iter().fold((0.0_f32, 0.0_f32), |(width, height), panel| {
      let (w, h) = self.panel_of(panel).map(|panel| panel.min_size).unwrap_or((0.0, 0.0));
      (width.max(w), height.max(h))
    })
  }

  /// The edge a group is nearest: where its panels hide to.
  fn nearest_side(&self, panel: &str) -> DockSide {
    let groups = self.kept.groups.lock().unwrap();
    let place = groups
      .iter()
      .find(|group| group.panels.iter().any(|held| held == panel));
    let (Some(place), width, height) = (place, self.kept.root.width(), self.kept.root.height()) else {
      return DockSide::Right;
    };
    if width <= 0.0 || height <= 0.0 {
      return DockSide::Right;
    }
    let frame = place.frame.bounds();
    let x = (frame.x - self.kept.root.x() + frame.width / 2.0) / width;
    let y = (frame.y - self.kept.root.y() + frame.height / 2.0) / height;
    [
      (x, DockSide::Left),
      (1.0 - x, DockSide::Right),
      (y, DockSide::Top),
      (1.0 - y, DockSide::Bottom),
    ]
    .into_iter()
    .min_by(|a, b| a.0.total_cmp(&b.0))
    .map(|(_, side)| side)
    .unwrap_or(DockSide::Right)
  }

  /// The items of the menu of `panel`: moves to each edge, float, hide, maximize or restore, and close.
  fn menu_items(&self, panel: &str) -> Arc<dyn Fn() -> Vec<MenuItem> + Send + Sync> {
    let Some(prepared) = self.panel_of(panel) else {
      return Arc::new(Vec::new);
    };
    let words = self.layout.words.clone();
    let changer = self.changer.clone();
    let floating = self.state.is_floating(panel);
    let maximized = self.state.maximized.as_deref() == Some(panel)
      || self.state.maximized.as_ref().is_some_and(|max| {
        self
          .state
          .group_of(max)
          .is_some_and(|group| group.panels.iter().any(|held| held == panel))
      });
    let hide_side = self.nearest_side(panel);
    let panel = panel.to_owned();
    Arc::new(move || {
      let mut items = Vec::new();
      let go = |label: &Arc<str>, icon: &'static str, place: DockPlace| {
        let (changer, panel) = (changer.clone(), panel.clone());
        MenuItem::new(label).icon(icon).on_select(move || {
          let place = place.clone();
          let panel = panel.clone();
          changer.change(move |state| state.move_panel(&panel, place));
        })
      };
      if prepared.movable {
        items.push(go(
          &words.move_left,
          icons::CHEVRON_LEFT,
          DockPlace::Edge(DockSide::Left),
        ));
        items.push(go(
          &words.move_right,
          icons::CHEVRON_RIGHT,
          DockPlace::Edge(DockSide::Right),
        ));
        items.push(go(&words.move_top, icons::CHEVRON_UP, DockPlace::Edge(DockSide::Top)));
        items.push(go(
          &words.move_bottom,
          icons::CHEVRON_DOWN,
          DockPlace::Edge(DockSide::Bottom),
        ));
        if prepared.floatable && !floating {
          items.push(go(
            &words.float,
            icons::LAYERS,
            DockPlace::Float {
              position: (40.0, 40.0),
              size: None,
            },
          ));
        }
      }
      if prepared.hideable && !floating {
        items.push(go(&words.hide, icons::PIN, DockPlace::Hidden(hide_side)));
      }
      if !floating {
        let (changer, panel) = (changer.clone(), panel.clone());
        items.push(if maximized {
          MenuItem::new(&words.restore)
            .icon(icons::CHEVRON_DOWN)
            .on_select(move || changer.change(|state| state.restore()))
        } else {
          MenuItem::new(&words.maximize)
            .icon(icons::CHEVRON_UP)
            .on_select(move || changer.change(|state| state.maximize(&panel)))
        });
      }
      if prepared.close_button {
        let (changer, panel) = (changer.clone(), panel.clone());
        items.push(
          MenuItem::new(&words.close)
            .icon(icons::CLOSE)
            .on_select(move || changer.close(&panel)),
        );
      }
      items
    })
  }

  /// The whole layout.
  fn render(&mut self) -> Element {
    let root_part = self.part(DockLayoutPart::Root);
    let docked = match self.state.maximized.clone() {
      Some(panel) => self.maximized(&panel),
      None => match self.state.root.clone() {
        Some(node) => self.node(&node, Vec::new(), false).map(|built| built.element),
        None => Some(self.empty()),
      },
    }
    .unwrap_or_else(|| self.empty());
    let mut middle = Row::new().flex(1.0).min_height(0.0).align_items(Alignment::Stretch);
    if let Some(strip) = self.strip(DockSide::Left) {
      middle = middle.child(strip);
    }
    middle = middle.child(
      Column::new()
        .flex(1.0)
        .min_width(0.0)
        .align_items(Alignment::Stretch)
        .child(docked),
    );
    if let Some(strip) = self.strip(DockSide::Right) {
      middle = middle.child(strip);
    }
    let mut column = Column::new()
      .width(Dimension::Pct(100.0))
      .height(Dimension::Pct(100.0))
      .align_items(Alignment::Stretch);
    if let Some(strip) = self.strip(DockSide::Top) {
      column = column.child(strip);
    }
    column = column.child(middle);
    if let Some(strip) = self.strip(DockSide::Bottom) {
      column = column.child(strip);
    }
    let mut root = Stack::new()
      .part(&root_part)
      .ref_element(self.kept.root.clone())
      .describe("a11y_role", "group")
      .describe("a11y_name", self.layout.label.clone())
      .child(column);
    if let Some(flyout) = self.flyout() {
      root = root.child(flyout);
    }
    if let Some(floating) = self.floating() {
      root = root.child(floating);
    }
    if self.mounted() {
      let overlay = self.overlay();
      root = root.child(overlay);
      let (dragger, flyout) = (self.dragger.clone(), self.kept.flyout.clone());
      root = root.on_key_down(move |event: KeyboardEvent| {
        if event.key != "Escape" || event.default_prevented() {
          return;
        }
        if dragger.cancel() {
          event.prevent_default();
        } else if flyout.get_untracked().is_some() {
          event.prevent_default();
          flyout.set(None);
        }
      });
    }
    if let Some(id) = &self.layout.id {
      root = root.id(id.clone());
    }
    self.offstage();
    root.into()
  }

  /// The layout as the theme's renderer makes it, where the theme has one.
  fn finish(&self, element: Element) -> Element {
    let view = DockLayoutView {
      element,
      recipe: self.recipe.clone(),
    };
    match self.theme.renderer::<DockLayout>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// What the area of documents shows while no document is in it: the app's, or nothing.
  fn empty(&self) -> Element {
    let part = self.part(DockLayoutPart::Empty);
    let mut empty = Column::new()
      .part(&part)
      .flex(1.0)
      .width(Dimension::Pct(100.0))
      .height(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .justify(Justify::Center);
    if let Some(content) = &self.layout.empty {
      empty = empty.child(held(content));
    }
    empty.into()
  }

  /// The group of the maximized panel, filling the layout.
  fn maximized(&mut self, panel: &str) -> Option<Element> {
    let group = self.state.group_of(panel)?.clone();
    let documents = self.state.in_documents(panel);
    Some(self.group(&group, documents, true).element)
  }

  /// A node of the tree, at `path`: nothing where it shows no panel (a group whose panels are all hidden).
  fn node(&mut self, node: &DockNode, path: Vec<usize>, documents: bool) -> Option<Built> {
    match node {
      DockNode::Group(group) => (!group.panels.is_empty()).then(|| self.group(group, documents, false)),
      DockNode::Documents(area) => match &area.node {
        Some(inner) => {
          let mut path = path;
          path.push(0);
          self.node(inner, path, true).or_else(|| {
            Some(Built {
              element: self.empty(),
              least: (0.0, 0.0),
              first: None,
            })
          })
        }
        None => Some(Built {
          element: self.empty(),
          least: (0.0, 0.0),
          first: None,
        }),
      },
      DockNode::Split(split) => self.split(split, path, documents),
    }
  }

  /// A split: a splitter of the children that show anything, each at its share.
  fn split(&mut self, split: &DockSplit, path: Vec<usize>, documents: bool) -> Option<Built> {
    let mut shown: Vec<(usize, Built)> = Vec::new();
    for (index, child) in split.children.iter().enumerate() {
      let mut at = path.clone();
      at.push(index);
      if let Some(built) = self.node(child, at, documents) {
        shown.push((index, built));
      }
    }
    match shown.len() {
      0 => return None,
      1 => return shown.pop().map(|(_, built)| built),
      _ => {}
    }
    let horizontal = split.orientation == Orientation::Horizontal;
    let total: f32 = shown
      .iter()
      .map(|(index, _)| split.shares[*index])
      .sum::<f32>()
      .max(f32::EPSILON);
    let mut least = (0.0_f32, 0.0_f32);
    for (_, built) in &shown {
      if horizontal {
        least = (least.0 + built.least.0, least.1.max(built.least.1));
      } else {
        least = (least.0.max(built.least.0), least.1 + built.least.1);
      }
    }
    let first = shown.iter().find_map(|(_, built)| built.first.clone());
    let indices: Vec<usize> = shown.iter().map(|(index, _)| *index).collect();
    let count = shown.len();
    let mut splitter = Splitter::new().orientation(split.orientation).label(&self.layout.label);
    for (at, (index, built)) in shown.into_iter().enumerate() {
      let mut pane = SplitterPane::new(built.element)
        .min(if horizontal { built.least.0 } else { built.least.1 })
        .key(format!("pane-{index}"));
      if at + 1 < count {
        pane = pane.share(split.shares[index] / total);
      }
      splitter = splitter.pane(pane);
    }
    // What a person sets: each shown child's share of what the shown ones had, the hidden ones keeping theirs.
    let (changer, shares, where_) = (self.changer.clone(), split.shares.clone(), path.clone());
    splitter = splitter.on_resize_end(move |sizes: &[f32]| {
      let sum: f32 = sizes.iter().sum();
      if sum <= 0.0 || sizes.len() != indices.len() {
        return;
      }
      let mut next = shares.clone();
      for (size, index) in sizes.iter().zip(&indices) {
        next[*index] = size / sum * total;
      }
      let path = where_.clone();
      changer.change(move |state| state.set_shares(&path, &next));
    });
    let key = format!(
      "dock-split-{}-{}-{}",
      self.generation,
      path.iter().map(|index| index.to_string()).collect::<Vec<_>>().join("."),
      first.as_deref().unwrap_or("")
    );
    splitter = splitter.key(key);
    if let Some(id) = &self.layout.id {
      let at = if path.is_empty() {
        "root".to_owned()
      } else {
        path.iter().map(|index| index.to_string()).collect::<Vec<_>>().join("-")
      };
      splitter = splitter.id(format!("{id}-split-{at}"));
    }
    let element = match self.ctx.as_deref_mut() {
      Some(ctx) => splitter.mount(ctx),
      None => splitter.build(&self.theme),
    };
    Some(Built { element, least, first })
  }

  /// A group: its bar and the panel it shows.
  fn group(&mut self, group: &DockGroup, documents: bool, maximized: bool) -> Built {
    let first = group.panels.first().cloned().unwrap_or_default();
    let shown = group
      .shown
      .clone()
      .filter(|shown| group.panels.contains(shown))
      .unwrap_or_else(|| first.clone());
    let bar: Element = if documents {
      self.documents_bar(group, &shown)
    } else if group.panels.len() == 1 && !maximized {
      self.header_bar(&shown)
    } else {
      self.tabs_bar(group, &shown, maximized, false)
    };
    let body_ref = ElementRef::new();
    let frame_ref = ElementRef::new();
    // The others of the group are kept offstage by the end of the render.
    let content = self.content(&shown);
    let focus_key = format!("dock-focus-{first}");
    let mut body = Column::new()
      .part(&self.part(DockLayoutPart::Body))
      .flex(1.0)
      .min_height(0.0)
      .align_items(Alignment::Stretch)
      .ref_element(body_ref.clone())
      .key(focus_key.clone())
      .tab_index(-1)
      .child(content);
    if let Some(id) = self.layout.id_of("body", &first) {
      body = body.id(id);
    }
    let mut frame = Column::new()
      .part(&self.part_in(DockLayoutPart::Group, &[(DockLayoutState::Maximized, maximized)]))
      .width(Dimension::Pct(100.0))
      .height(Dimension::Pct(100.0))
      .align_items(Alignment::Stretch)
      .ref_element(frame_ref.clone())
      .key(format!("dock-group-{first}"))
      .describe("a11y_role", "group")
      .describe("a11y_name", self.title(&shown))
      .child(bar)
      .child(body);
    // F6 and Shift+F6 go from the group with the focus to the next group, and back: the next render focuses its body.
    if self.mounted() {
      let (groups, focus) = (self.kept.groups.clone(), self.kept.focus.clone());
      let own = first.clone();
      frame = frame.on_key_down(move |event: KeyboardEvent| {
        if event.key != "F6" || !event.focus_within || event.default_prevented() || event.ctrl || event.alt {
          return;
        }
        let groups = groups.lock().unwrap();
        let count = groups.len();
        let Some(at) = groups.iter().position(|group| group.panels.first() == Some(&own)) else {
          return;
        };
        let next = if event.shift {
          (at + count - 1) % count
        } else {
          (at + 1) % count
        };
        if let Some(first) = groups[next].panels.first() {
          event.prevent_default();
          let (count, _) = focus.get_untracked();
          focus.set((count + 1, first.clone()));
        }
      });
      let (count, asked) = self.kept.focus.get();
      if count > self.kept.focused.load(Ordering::Relaxed) && asked == first {
        self.kept.focused.store(count, Ordering::Relaxed);
        if let Some(ctx) = self.ctx.as_deref_mut() {
          ctx.focus(&body_ref);
        }
      }
    }
    if let Some(id) = self.layout.id_of("group", &first) {
      frame = frame.id(id);
    }
    self.groups.push(GroupPlace {
      panels: group.panels.clone(),
      frame: frame_ref,
      body: body_ref,
    });
    let (mut width, mut height) = self.least(group);
    height += 32.0;
    width = width.max(0.0);
    Built {
      element: frame.into(),
      least: (width, height),
      first: Some(first),
    }
  }

  /// The documents' tabs of a group in the area of documents, in the board's compact bar; its empty room drags the
  /// whole group.
  fn documents_bar(&mut self, group: &DockGroup, shown: &str) -> Element {
    let tabs: Vec<DocumentTab<String>> = group
      .panels
      .iter()
      .map(|panel| {
        let prepared = self.panel_of(panel);
        let mut tab = DocumentTab::new(panel.clone(), &self.title(panel));
        if let Some(prepared) = &prepared {
          if let Some(icon) = &prepared.icon {
            tab = tab.icon(icon.clone());
          }
          tab = tab.dirty(prepared.dirty);
          if prepared.close_button {
            let (changer, panel) = (self.changer.clone(), panel.clone());
            tab = tab.on_close(move || changer.close(&panel));
          }
          if let Some(id) = self.layout.id_of("tab", panel) {
            tab = tab.id(id);
          }
        }
        tab
      })
      .collect();
    let select = self.changer.clone();
    let mut bar = DocumentTabs::controlled(shown.to_owned(), tabs, move |panel: &String| {
      let panel = panel.clone();
      select.change(move |state| state.select(&panel));
    })
    .label(&self.layout.label)
    .close_label(&self.layout.words.close)
    .rules(&compact_documents())
    .key(format!(
      "dock-tabs-{}",
      group.panels.first().cloned().unwrap_or_default()
    ));
    let (reorder, panels) = (self.changer.clone(), group.panels.clone());
    bar = bar.on_reorder(move |from, to| {
      let Some(moved) = panels.get(from).cloned() else {
        return;
      };
      let panels = panels.clone();
      reorder.change(move |state| reorder_group(state, &panels, &moved, to));
    });
    if self.mounted() {
      let dragger = self.dragger.clone();
      let movable: HashSet<String> = group
        .panels
        .iter()
        .filter(|panel| self.panel_of(panel).is_some_and(|panel| panel.movable))
        .cloned()
        .collect();
      bar = bar.on_drag_away(move |panel: &String, event: &DragEvent| {
        if !movable.contains(panel) {
          return;
        }
        if event.drop_result.is_some() {
          dragger.ended(Dragged::Panel(panel.clone()), event);
        } else {
          dragger.moved(Dragged::Panel(panel.clone()), false, event);
        }
      });
      let menus: HashMap<String, Arc<dyn Fn() -> Vec<MenuItem> + Send + Sync>> = group
        .panels
        .iter()
        .map(|panel| (panel.clone(), self.menu_items(panel)))
        .collect();
      bar = bar.context_menu(move |panel: &String| menus.get(panel).map(|items| items()).unwrap_or_default());
    }
    let tabs = match self.ctx.as_deref_mut() {
      Some(ctx) => bar.mount(ctx),
      None => bar.build(&self.theme),
    };
    let mut row = Row::new().width(Dimension::Pct(100.0)).child(tabs);
    if self.mounted()
      && group
        .panels
        .iter()
        .all(|panel| self.panel_of(panel).is_some_and(|panel| panel.movable))
    {
      row = self
        .dragger
        .on(row, Dragged::Group(group.panels.first().cloned().unwrap_or_default()));
    }
    row.into()
  }

  /// The header of a group of one tool panel: its title in capitals, its menu and its close.
  fn header_bar(&mut self, panel: &str) -> Element {
    let prepared = self.panel_of(panel);
    let mut header = PanelHeader::new(&self.title(panel)).more_label(&self.layout.words.more);
    if self.mounted() {
      let open = self.kept.menu(panel);
      let items = self.menu_items(panel);
      header = header.menu(open, move || items());
    }
    if prepared.as_ref().is_some_and(|prepared| prepared.close_button) {
      let (changer, panel) = (self.changer.clone(), panel.to_owned());
      header = header
        .close_label(&self.layout.words.close)
        .on_close(move || changer.close(&panel));
    }
    let mut column = Column::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Stretch)
      .child(header.build(&self.theme));
    if self.mounted() && prepared.is_some_and(|prepared| prepared.movable) {
      column = self.dragger.on(column, Dragged::Panel(panel.to_owned()));
    }
    column.into()
  }

  /// The bar of tabs in capitals of a group of several tool panels, with its maximize (or restore) and its close at
  /// its end; a tab drags its panel, the bar's empty room the whole group.
  fn tabs_bar(&mut self, group: &DockGroup, shown: &str, maximized: bool, floating: bool) -> Element {
    let bar_part = self.part(DockLayoutPart::Bar);
    let mut tabs = Row::new()
      .align_items(Alignment::Stretch)
      .height(Dimension::Pct(100.0))
      .describe("a11y_role", "tablist")
      .describe("a11y_name", self.layout.label.clone())
      .focus_group(FocusGroup::horizontal());
    for panel in &group.panels {
      tabs = tabs.child(self.tab(panel, panel == shown));
    }
    let mut actions = Row::new()
      .part(&self.part(DockLayoutPart::Actions))
      .align_items(Alignment::Center);
    let button_part = self.part(DockLayoutPart::Button);
    if !floating {
      let (icon, label) = if maximized {
        (icons::CHEVRON_DOWN, self.layout.words.restore.clone())
      } else {
        (icons::CHEVRON_UP, self.layout.words.maximize.clone())
      };
      let (changer, panel) = (self.changer.clone(), shown.to_owned());
      let mut button = header_button(
        &button_part,
        &Name::new(icon),
        &label,
        Arc::new(move || {
          let panel = panel.clone();
          changer.change(move |state| {
            if maximized {
              state.restore()
            } else {
              state.maximize(&panel)
            }
          });
        }),
      );
      if let Some(id) = self.layout.id_of("maximize", shown) {
        button = button.id(id);
      }
      actions = actions.child(button);
    }
    if self.panel_of(shown).is_some_and(|panel| panel.close_button) {
      let (changer, panel) = (self.changer.clone(), shown.to_owned());
      let mut button = header_button(
        &button_part,
        &Name::new(icons::CLOSE),
        &self.layout.words.close,
        Arc::new(move || changer.close(&panel)),
      );
      if let Some(id) = self.layout.id_of("close", shown) {
        button = button.id(id);
      }
      actions = actions.child(button);
    }
    let mut bar = Row::new()
      .part(&bar_part)
      .align_items(Alignment::Center)
      .width(Dimension::Pct(100.0))
      .child(tabs)
      .child(Row::new().flex(1.0))
      .child(actions);
    let all_movable = group
      .panels
      .iter()
      .all(|panel| self.panel_of(panel).is_some_and(|panel| panel.movable));
    if self.mounted() && all_movable && !floating {
      bar = self
        .dragger
        .on(bar, Dragged::Group(group.panels.first().cloned().unwrap_or_default()));
    }
    bar.into()
  }

  /// A tab in capitals: its title, its count, a press shows it, a drag moves it, its menu has the rest.
  fn tab(&mut self, panel: &str, selected: bool) -> Element {
    let part = self.part_in(DockLayoutPart::Tab, &[(DockLayoutState::Selected, selected)]);
    let part = if selected {
      part.forced(self.layout.forced)
    } else {
      part
    };
    let label = self.part_in(DockLayoutPart::Label, &[(DockLayoutState::Selected, selected)]);
    let prepared = self.panel_of(panel);
    let title = self.title(panel);
    let mut tab = pressable(&part, false)
      .align_items(Alignment::Center)
      .tab_index(if selected { 0 } else { -1 })
      .describe("a11y_role", "tab")
      .describe(
        "a11y_name",
        prepared
          .as_ref()
          .and_then(|prepared| prepared.label.clone())
          .unwrap_or_else(|| title.clone()),
      )
      .describe("a11y_selected", if selected { "true" } else { "false" })
      .key(format!("dock-tab-{panel}"))
      .child(one_line(&label, &title.to_uppercase()));
    if let Some(count) = prepared.as_ref().and_then(|prepared| prepared.count) {
      let part = self.part(DockLayoutPart::Count);
      tab = tab.child(Row::new().part(&part).child(part.text(&count.to_string())));
    }
    let (changer, select) = (self.changer.clone(), panel.to_owned());
    tab = tab.on_click(move |_| {
      let panel = select.clone();
      changer.change(move |state| state.select(&panel));
    });
    if let Some(id) = self.layout.id_of("tab", panel) {
      tab = tab.id(id);
    }
    if self.mounted() && prepared.as_ref().is_some_and(|prepared| prepared.movable) {
      tab = self.dragger.on(tab, Dragged::Panel(panel.to_owned()));
    }
    if self.mounted() {
      let items = self.menu_items(panel);
      return ContextMenu::new(self.kept.menu(&format!("tab:{panel}")), tab)
        .items(items())
        .into();
    }
    tab.into()
  }

  /// The strip along `side` with a button for each panel hidden there.
  fn strip(&mut self, side: DockSide) -> Option<Element> {
    let hidden: Vec<String> = self
      .state
      .hidden()
      .into_iter()
      .filter(|hidden| hidden.side == side)
      .map(|hidden| hidden.panel)
      .collect();
    if hidden.is_empty() || self.state.maximized.is_some() {
      return None;
    }
    let vertical = matches!(side, DockSide::Left | DockSide::Right);
    let part = self.part_in(
      DockLayoutPart::Strip,
      &[
        (DockLayoutState::Vertical, vertical),
        (DockLayoutState::Left, side == DockSide::Left),
        (DockLayoutState::Right, side == DockSide::Right),
        (DockLayoutState::Top, side == DockSide::Top),
        (DockLayoutState::Bottom, side == DockSide::Bottom),
      ],
    );
    let shown = self.kept.flyout.get();
    let mut buttons: Vec<Element> = Vec::new();
    for panel in hidden {
      let selected = shown.as_deref() == Some(panel.as_str());
      let button_part = self.part_in(DockLayoutPart::StripButton, &[(DockLayoutState::Selected, selected)]);
      let icon = self
        .panel_of(&panel)
        .and_then(|prepared| prepared.icon.clone())
        .unwrap_or_else(|| Name::new(icons::SQUARE));
      let (flyout, toggled) = (self.kept.flyout.clone(), panel.clone());
      let mut button = pressable(&button_part, false)
        .align_items(Alignment::Center)
        .describe("a11y_role", "button")
        .describe("a11y_name", self.title(&panel))
        .describe("a11y_expanded", if selected { "true" } else { "false" })
        .key(format!("dock-strip-{panel}"))
        .on_click(move |_| {
          let open = flyout.get_untracked().as_deref() == Some(toggled.as_str());
          flyout.set(if open { None } else { Some(toggled.clone()) });
        })
        .child(Icon::new(icon).size(IconSize::Sm).build(&self.theme));
      if let Some(id) = self.layout.id_of("strip", &panel) {
        button = button.id(id);
      }
      buttons.push(button.into());
    }
    let strip: Element = if vertical {
      Column::new()
        .part(&part)
        .align_items(Alignment::Center)
        .describe("a11y_role", "toolbar")
        .describe("a11y_name", self.layout.words.strip.clone())
        .focus_group(FocusGroup::vertical())
        .with_children(buttons)
        .into()
    } else {
      Row::new()
        .part(&part)
        .align_items(Alignment::Center)
        .describe("a11y_role", "toolbar")
        .describe("a11y_name", self.layout.words.strip.clone())
        .focus_group(FocusGroup::horizontal())
        .with_children(buttons)
        .into()
    };
    Some(strip)
  }

  /// The hidden panel shown from its strip, over the layout beside the strip: its header with Put back and its close,
  /// and what it holds. A press outside it or Escape hides it again.
  fn flyout(&mut self) -> Option<Element> {
    let panel = self.kept.flyout.get()?;
    let side = self.state.is_hidden(&panel)?;
    let prepared = self.panel_of(&panel)?;
    let (width, height) = (self.kept.root.width(), self.kept.root.height());
    let strip = 30.0;
    let across = prepared.min_size.0.max(FLYOUT.0);
    let along = prepared.min_size.1.max(FLYOUT.1);
    let (x, y, w, h) = match side {
      DockSide::Right => ((width - strip - across).max(0.0), 0.0, across, height),
      DockSide::Left => (strip, 0.0, across, height),
      DockSide::Top => (0.0, strip, width, along),
      DockSide::Bottom => (0.0, (height - strip - along).max(0.0), width, along),
    };
    let mut header = PanelHeader::new(&prepared.title).more_label(&self.layout.words.more);
    let (changer, back) = (self.changer.clone(), panel.clone());
    let flyout = self.kept.flyout.clone();
    header = header.button(icons::PIN, &self.layout.words.put_back, move || {
      flyout.set(None);
      let panel = back.clone();
      changer.change(move |state| state.put_back(&panel));
    });
    if prepared.close_button {
      let (changer, closed) = (self.changer.clone(), panel.clone());
      header = header
        .close_label(&self.layout.words.close)
        .on_close(move || changer.close(&closed));
    }
    let content = self.content(&panel);
    let at = ElementRef::new();
    if let Some(ctx) = self.ctx.as_deref_mut() {
      let (flyout, shown) = (self.kept.flyout.clone(), panel.clone());
      ctx.on_click_outside(at.clone(), move |_| {
        if flyout.get_untracked().as_deref() == Some(shown.as_str()) {
          flyout.set(None);
        }
      });
    }
    let mut element = Column::new()
      .part(&self.part(DockLayoutPart::Flyout))
      .absolute(x, y, Dimension::Px(w), Dimension::Px(h))
      .align_items(Alignment::Stretch)
      .ref_element(at)
      .describe("a11y_role", "dialog")
      .describe("a11y_name", prepared.title.clone())
      .key(format!("dock-flyout-{panel}"))
      .child(header.build(&self.theme))
      .child(
        Column::new()
          .flex(1.0)
          .min_height(0.0)
          .align_items(Alignment::Stretch)
          .child(content),
      );
    if let Some(id) = self.layout.id_of("flyout", &panel) {
      element = element.id(id);
    }
    Some(element.into())
  }

  /// The groups that float over the layout, each a floating panel of the board's.
  fn floating(&mut self) -> Option<Element> {
    if self.state.floating.is_empty() || self.state.maximized.is_some() {
      return None;
    }
    let floats = self.state.floating.clone();
    let mut panels = Vec::new();
    let mut kept = self.kept.floats.lock().unwrap().clone();
    let mut keys = HashSet::new();
    for float in &floats {
      let Some(first) = float.group.panels.first().cloned() else {
        continue;
      };
      let shown = float
        .group
        .shown
        .clone()
        .filter(|shown| float.group.panels.contains(shown))
        .unwrap_or_else(|| first.clone());
      let key = format!("{}:{first}", self.generation);
      keys.insert(key.clone());
      let (position, size) = kept
        .entry(key)
        .or_insert_with(|| (Signal::new(float.position), Signal::new(float.size)))
        .clone();
      let mut body = Column::new().align_items(Alignment::Stretch).flex(1.0).min_height(0.0);
      if float.group.panels.len() > 1 {
        body = body.child(self.tabs_bar(&float.group, &shown, false, true));
      }
      body = body.child(self.content(&shown));
      let least = self.least(&float.group);
      let mut panel = FloatingPanel::new(&self.title(&shown))
        .key(first.clone())
        .position(position.clone())
        .size(size.clone())
        .min_size((least.0.max(160.0), least.1.max(80.0)))
        .more_label(&self.layout.words.more)
        .close_label(&self.layout.words.close)
        .child(body);
      if let Some(id) = self.layout.id_of("float", &first) {
        panel = panel.id(id);
      }
      if self.mounted() {
        let items = self.menu_items(&shown);
        panel = panel.menu(self.kept.menu(&format!("float:{first}")), move || items());
      }
      if self.panel_of(&shown).is_some_and(|panel| panel.close_button) {
        let (changer, closed) = (self.changer.clone(), shown.clone());
        panel = panel.on_dismiss(move || changer.close(&closed));
      } else {
        panel = panel.close_button(false);
      }
      if self.mounted() {
        let (moving, ending) = (self.dragger.clone(), self.dragger.clone());
        let (moved, ended) = (Dragged::Group(first.clone()), Dragged::Group(first.clone()));
        let (changer, settled) = (self.changer.clone(), first.clone());
        panel.docked = Some(DockHooks {
          drag: Arc::new(move |event: &DragEvent| {
            if event.drop_result.is_some() {
              ending.ended(ended.clone(), event);
            } else {
              moving.moved(moved.clone(), true, event);
            }
          }),
          settled: Arc::new(move || {
            let (position, size) = (position.get_untracked(), size.get_untracked());
            let first = settled.clone();
            changer.change(move |state| {
              let Some(index) = state
                .floating
                .iter()
                .position(|float| float.group.panels.first() == Some(&first))
              else {
                return false;
              };
              state.set_float(index, position, size)
            });
          }),
        });
      }
      panels.push(panel);
    }
    kept.retain(|key, _| keys.contains(key));
    *self.kept.floats.lock().unwrap() = kept;
    let layer = FloatingPanels::new().panels(panels).key("dock-floating");
    Some(match self.ctx.as_deref_mut() {
      Some(ctx) => layer.mount(ctx),
      None => Stack::new()
        .absolute(0.0, 0.0, Dimension::Pct(100.0), Dimension::Pct(100.0))
        .hit_test(HitTestBehavior::ContentOnly)
        .overflow_visible()
        .into(),
    })
  }

  /// The layer over the layout that draws a drag: the preview of where the panel lands, the indicator over the group
  /// under the pointer, and the squares of the layout's edges.
  fn overlay(&mut self) -> Element {
    let parts = OverlayParts {
      preview: self.part(DockLayoutPart::Preview),
      zone: self.part(DockLayoutPart::Zone),
      hot: self.part_in(DockLayoutPart::Zone, &[(DockLayoutState::Hot, true)]),
    };
    let view = self.kept.view.clone();
    let ctx = self
      .ctx
      .as_deref_mut()
      .expect("the overlay is drawn by a mounted layout");
    ctx.mount_keyed::<OverlayHost>(
      "dock-overlay",
      OverlayProps {
        view,
        parts: Arc::new(parts),
      },
    )
  }
}

/// Moves `moved` among the panels of its group to `to`, as the documents' tabs count it.
fn reorder_group(state: &mut DockState, panels: &[String], moved: &str, to: usize) -> bool {
  fn walk(node: &mut DockNode, panels: &[String], moved: &str, to: usize) -> bool {
    match node {
      DockNode::Group(group) if group.panels == panels => {
        let Some(from) = group.panels.iter().position(|panel| panel == moved) else {
          return false;
        };
        let panel = group.panels.remove(from);
        group.panels.insert(to.min(group.panels.len()), panel);
        from != to
      }
      DockNode::Group(_) => false,
      DockNode::Split(split) => split.children.iter_mut().any(|child| walk(child, panels, moved, to)),
      DockNode::Documents(area) => area
        .node
        .as_deref_mut()
        .is_some_and(|node| walk(node, panels, moved, to)),
    }
  }
  state.root.as_mut().is_some_and(|root| walk(root, panels, moved, to))
}

/// The board's compact documents' bar in a dock layout: 32 high, its titles 12, its tabs 6 between what they hold and
/// 12 at their end, their crosses of 12 shown on every tab.
fn compact_documents() -> Recipe {
  use DocumentTabsPart::{Close, CloseIcon, Label, Root, Tab};
  Recipe::new()
    .base(Root, |s| s.height(32.0))
    .row_density(super::recipe::Selector::new(), Root, 32.0)
    .base(Tab, |s| s.padding_end(12.0).gap(6.0))
    .base(Label, |s| s.font_size(12.0))
    .base(Close, |s| s.square(16.0))
    .base(CloseIcon, |s| s.square(12.0).opacity(1.0))
}

/// A part of the layout as one render built it, with the least size of what it holds and its first panel.
struct Built {
  element: Element,
  least: (f32, f32),
  first: Option<String>,
}

/// What a mounted panel is given: what it holds, made once by the app's build of the layout.
struct PanelProps {
  content: Held,
}

/// What a panel holds is built again only where the app built it again.
impl PartialEq for PanelProps {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.content, &other.content)
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for PanelProps {}

/// What a panel holds, mounted in the layout by the panel's id: wherever the panel is shown, it is this component,
/// and what is typed, scrolled or opened in it is kept.
struct PanelHost;

impl Component for PanelHost {
  type Props = PanelProps;

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    held(&ctx.props::<PanelProps>().content)
  }
}

/// The parts the layer of a drag is drawn with.
struct OverlayParts {
  preview: ResolvedPart,
  zone: ResolvedPart,
  hot: ResolvedPart,
}

struct OverlayProps {
  view: Signal<Option<DragView>>,
  parts: Arc<OverlayParts>,
}

impl PartialEq for OverlayProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for OverlayProps {}

/// The layer of a drag: a component of its own, so that a move of the pointer draws it again and nothing else.
struct OverlayHost;

impl Component for OverlayHost {
  type Props = OverlayProps;

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<OverlayProps>();
    let parts = props.parts.clone();
    let mut layer = Stack::new()
      .absolute(0.0, 0.0, Dimension::Pct(100.0), Dimension::Pct(100.0))
      .hit_test(HitTestBehavior::None)
      .overflow_visible();
    let Some(view) = props.view.get() else {
      return layer;
    };
    if view.preview {
      layer = layer.child(Row::new().part(&parts.preview).absolute(
        view.preview_x,
        view.preview_y,
        Dimension::Px(view.preview_width),
        Dimension::Px(view.preview_height),
      ));
    }
    let square = |(x, y): (f32, f32), hot: bool| -> Row {
      Row::new().part(if hot { &parts.hot } else { &parts.zone }).absolute(
        x - ZONE / 2.0,
        y - ZONE / 2.0,
        Dimension::Px(ZONE),
        Dimension::Px(ZONE),
      )
    };
    for (number, side) in DockSide::ALL.iter().enumerate() {
      let at = Dragger::edge_square(*side, view.width, view.height);
      layer = layer.child(square(at, view.edge_hot == number as u8 + 2));
    }
    if view.indicator {
      let middle = (view.indicator_x, view.indicator_y);
      layer = layer.child(square(middle, view.hot == 1));
      for (number, side) in DockSide::ALL.iter().enumerate() {
        let at = Dragger::zone_square(Zone::Side(*side), middle);
        layer = layer.child(square(at, view.hot == number as u8 + 2));
      }
    }
    layer
  }
}

/// What a mounted layout is given: the layout as the app built it.
struct DockProps {
  layout: Arc<Prepared>,
}

impl PartialEq for DockProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for DockProps {}

/// A mounted layout.
struct DockHost {
  kept: Kept,
}

impl Component for DockHost {
  type Props = DockProps;

  fn create(ctx: &mut Ctx) -> Self {
    let kept = Kept::new();
    // The signals a render reads, made the component's own so that a change of them renders it again.
    let kept = Kept {
      flyout: ctx.signal(None),
      focus: ctx.signal((0, String::new())),
      ..kept
    };
    Self { kept }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let layout = ctx.props::<DockProps>().layout.clone();
    let raw = layout.raw();
    {
      // Another arrangement than the one the layout set last is the app's: the splits begin again from its shares.
      let mut seen = self.kept.seen.lock().unwrap();
      if seen.as_ref() != Some(&raw) {
        let ours = self.kept.written.lock().unwrap().as_ref() == Some(&raw);
        if seen.is_some() && !ours {
          self.kept.generation.fetch_add(1, Ordering::Relaxed);
        }
        *seen = Some(raw.clone());
      }
    }
    let state = layout.valid(&raw);
    let theme = current_theme();
    let groups = self.kept.groups.clone();
    let mut builder = Builder::new(Some(ctx), &theme, layout, &self.kept, state);
    let element = builder.render();
    *groups.lock().unwrap() = std::mem::take(&mut builder.groups);
    builder.finish(element)
  }
}

/// A dock layout as a renderer sees it: the element the default renderer built, and the style of its parts.
pub struct DockLayoutView {
  /// The layout as the default renderer builds it.
  pub element: Element,
  /// The style of each part of the layout.
  pub recipe: ResolvedRecipe,
}

impl DockLayoutView {
  /// The style of a part, as this layout is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  /// How the default renderer builds a layout.
  pub fn render(self) -> Element {
    self.element
  }
}

impl Themed for DockLayout {
  const NAME: &'static str = "dock_layout";
  type Renderer = DockLayoutRenderer;

  /// The design's dock layout (`pencil/lurq-design-system.pen`, Dock Layout): on the background, in an edge of 1 in
  /// the border's color with the large radius. A group of several tool panels has a bar 30 high on the muted fill with
  /// a line of 1 along its bottom, 4 in at its end: its tabs 12 in at either side and 6 between what they hold, their
  /// titles 11 and semibold in capitals 0.6 apart in the muted foreground, the one shown in the foreground with a line
  /// of 2 under it; a count in mono 11; buttons of 22 with the small radius and an icon of 14 in the foreground. A
  /// strip of hidden panels is 30 across on the muted fill with a line of 1 on its inner side, 6 in along it and 4
  /// between its buttons of 22. While a panel is dragged, its indicator is five squares of 32 on the popover's fill
  /// in an edge of 1 in the input's color with the small radius and the large shadow, the one under the pointer in
  /// the info color; where it lands is laid over in the info's muted fill at seven tenths with an edge of 1 in the
  /// info color.
  fn recipe() -> Recipe {
    use DockLayoutPart::{
      Actions, Bar, Body, Button, Count, Empty, Flyout, Group, Label, Preview, Root, Strip, StripButton, Tab, Zone,
    };
    use DockLayoutState::{Bottom, Hot, Left, Right, Selected, Top, Vertical};
    use PaletteColor::{
      Background, Border as Line, Foreground, Highlight, Info, InfoMuted, Input, Muted, MutedForeground, Popover,
      PopoverForeground,
    };
    let line = |width: BorderSize, color: PaletteColor| Border::inside(width, color);
    Recipe::new()
      .base(Root, |s| {
        s.width(Length::Fill)
          .height(Length::Fill)
          .background(Background)
          .border(line(BorderSize::Sm, Line))
          .radius(RadiusSize::Lg)
      })
      .base(Group, |s| s.background(Background))
      .base(Body, |s| s.background(Background))
      .base(Bar, |s| {
        s.height(30.0)
          .padding_end(4.0)
          .background(Muted)
          .underline(line(BorderSize::Sm, Line))
      })
      .base(Tab, |s| {
        fading(
          s.height(30.0)
            .padding_x(12.0)
            .gap(6.0)
            .foreground(MutedForeground)
            .cursor(CursorIcon::Pointer),
        )
      })
      .base(Label, |s| {
        s.typography(TypographyStyle::Label)
          .font_size(11.0)
          .weight(FontWeight::SemiBold)
          .letter_spacing(0.6)
          .foreground(MutedForeground)
      })
      .base(Count, |s| {
        s.typography(TypographyStyle::Mono)
          .font_size(11.0)
          .weight(FontWeight::Normal)
          .foreground(MutedForeground)
      })
      .state(Selected, Tab, |s| s.underline(line(BorderSize::Md, Foreground)))
      .state(Selected, Label, |s| s.foreground(Foreground))
      .hovered(Tab, |s| s.foreground(Foreground))
      .focus_visible(Tab, |s| {
        s.outline(Outline::new(BorderSize::Md, PaletteColor::Ring).offset(-2.0))
      })
      .base(Actions, |s| s.gap(0.0))
      .base(Button, button_style)
      .base(Button, |s| s.foreground(Foreground))
      .hovered(Button, |s| s.background(Highlight))
      .focus_visible(Button, |s| s.focus_ring())
      .base(Empty, |s| s.foreground(MutedForeground))
      .base(Strip, |s| s.height(30.0).padding_x(6.0).gap(4.0).background(Muted))
      .state(Vertical, Strip, |s| {
        s.width(30.0).height(Length::Fill).padding_x(0.0).padding_y(6.0)
      })
      .state(Left, Strip, |s| s.end_line(line(BorderSize::Sm, Line)))
      .state(Right, Strip, |s| s.start_line(line(BorderSize::Sm, Line)))
      .state(Top, Strip, |s| s.underline(line(BorderSize::Sm, Line)))
      .state(Bottom, Strip, |s| s.top_line(line(BorderSize::Sm, Line)))
      .base(StripButton, button_style)
      .base(StripButton, |s| s.foreground(Foreground))
      .hovered(StripButton, |s| s.background(Highlight))
      .state(Selected, StripButton, |s| s.background(Highlight))
      .focus_visible(StripButton, |s| s.focus_ring())
      .base(Flyout, |s| {
        s.background(Popover)
          .foreground(PopoverForeground)
          .border(line(BorderSize::Sm, Line))
          .shadow(ShadowStyle::Lg)
      })
      .base(Preview, |s| {
        s.background(InfoMuted).opacity(0.7).border(line(BorderSize::Sm, Info))
      })
      .base(Zone, |s| {
        s.background(Popover)
          .border(line(BorderSize::Sm, Input))
          .radius(RadiusSize::Sm)
          .shadow(ShadowStyle::Lg)
      })
      .state(Hot, Zone, |s| s.background(Info).border(line(BorderSize::Sm, Info)))
  }
}
