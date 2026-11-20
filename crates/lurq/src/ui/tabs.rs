//! Tabs: a list of tabs of which one is selected, and the panel of the selected one.

use std::{
  fmt::Display,
  hash::{DefaultHasher, Hash, Hasher},
  sync::Arc,
};

use parking_lot::Mutex;

use super::{
  icons,
  menu::{Menu, MenuItem},
  names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{Orientation, fading, glyph, one_line, opens_aside, pressable, tab_stop},
  theme::{Themed, current_theme},
  tooltip::Tooltip,
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
  node::{
    CursorIcon, Element, EventHandler, FocusGroup, IntoMouseEventHandler, border::Border, dimension::Dimension,
    outline::Outline,
  },
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
  /// The count of a tab in a pill, after its label: how many things in it wait for the person.
  Badge,
  /// The dot of a tab, after its count and its badge: the state of what it counts.
  Dot,
  /// The icon of a tab after its label, its count, its badge and its dot: a lock on a tab not built yet.
  TrailingIcon,
  /// The bone in the count's place while the count is read: its width, its height and its corners.
  CountSkeleton,
  /// The quiet words at the end of the list's line.
  Note,
  /// What the app puts at the end of the list's line.
  End,
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
  Badge => "badge",
  Dot => "dot",
  TrailingIcon => "trailing_icon",
  CountSkeleton => "count_skeleton",
  Note => "note",
  End => "end",
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
  /// Of a tab: a press leaves it alone. It is drawn as it is, and is neither selected nor focused by a person.
  Inert,
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
  Inert => "inert",
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
/// // Three things wait for the person; the next tab is not there yet, and says why.
/// let review = Tab::new("review", "Review").badge(3);
/// let later = Tab::new("insights", "Insights").disabled_reason("Comes with the next release");
/// // A count of what waits, in a state the theme's recipe paints; a tab that says more of itself while pointed at.
/// let agents = Tab::new("agents", "Agents").count(2).state("waits", true).hint("Two agents wait for you");
/// ```
pub struct Tab<T> {
  value: T,
  label: Arc<str>,
  icon: Option<Name>,
  count: Option<Arc<str>>,
  badge: Option<Arc<str>>,
  reason: Option<Arc<str>>,
  disabled: bool,
  separated: bool,
  dot: Option<PaletteColor>,
  count_pending: bool,
  inert: bool,
  trailing_icon: Option<Name>,
  at_end: bool,
  states: Vec<Name>,
  tooltip: Option<Tooltip>,
  panel: Option<PanelBuilder>,
  on_open_aside: Vec<EventHandler<crate::app::events::MouseEvent>>,
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
      badge: None,
      reason: None,
      disabled: false,
      separated: false,
      dot: None,
      count_pending: false,
      inert: false,
      trailing_icon: None,
      at_end: false,
      states: Vec::new(),
      tooltip: None,
      panel: None,
      on_open_aside: Vec::new(),
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

  /// A count after the label in a pill: how many things in the tab wait for the person. "3", "99+". Beside a
  /// [`count`](Self::count), after it.
  pub fn badge(mut self, count: impl Display) -> Self {
    self.badge = Some(Arc::from(count.to_string()));
    self
  }

  /// A dot after the count and the badge, in `color`, a role of the theme's palette: the state of what the tab
  /// counts, `PaletteColor::Destructive` once one of its services crashed. Its size is the recipe's (part `dot`).
  pub fn dot(mut self, color: PaletteColor) -> Self {
    self.dot = Some(color);
    self
  }

  /// Whether the count is still being read: a bone stands in its place, as wide as one digit, so that a count of
  /// one digit takes its place without moving the tabs after it. The tab is busy to tooling. A
  /// [`count`](Self::count) given beside it is shown in place of the bone.
  pub fn count_pending(mut self, pending: bool) -> Self {
    self.count_pending = pending;
    self
  }

  /// Whether a press leaves the tab alone: it is drawn as it is, selected or not, and not dimmed, but a press of
  /// it, its item in the menu of the overflowing form and the arrow keys neither select it nor focus it, and its
  /// [`on_open_aside`](Self::on_open_aside) is not called. The selected tab of a page that keeps the panel it shows
  /// whatever is pressed; a tab that is there to be seen.
  pub fn inert(mut self, inert: bool) -> Self {
    self.inert = inert;
    self
  }

  /// An icon after the label, and after its count, its badge and its dot, by its name in the theme's set: a lock on a
  /// tab whose page is not built yet. It is drawn in the part [`TabsPart::TrailingIcon`], and is beside an
  /// [`icon`](Self::icon) before the label.
  pub fn trailing_icon(mut self, name: impl Into<Name>) -> Self {
    self.trailing_icon = Some(name.into());
    self
  }

  /// Whether the tab stands at the end of the list's line, with the other tabs so placed, after the room that grows
  /// and after the [note](Tabs::note) and the app's [element](Tabs::end): a page's Settings after its lists. It is
  /// still a tab of the list: its arrow keys reach it after the tabs at the start, and the menu of the overflowing
  /// form lists it in its order. A list that runs down puts it after the others.
  pub fn at_end(mut self, at_end: bool) -> Self {
    self.at_end = at_end;
    self
  }

  /// Whether the tab cannot be selected: it is dimmed, and the arrow keys pass it by.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// The tab cannot be selected, for `reason`: it is disabled, and the reason is its tooltip and its description to
  /// tooling: "Comes with the next release". An empty reason is none, and leaves the tab as it was.
  pub fn disabled_reason(mut self, reason: &str) -> Self {
    if !reason.is_empty() {
      self.reason = Some(Arc::from(reason));
      self.disabled = true;
    }
    self
  }

  /// Puts this tab alone in a state the theme's recipe names, or takes it out: every part of the tab (its label, its
  /// count, its badge, its dot, its icons) is styled in it, beside its own states (selected, disabled, inert), and the
  /// other tabs are not. A count of what waits for the person painted in the warning color:
  /// `recipe.when(Selector::new().state("waits"), TabsPart::Count, |s| s.foreground(PaletteColor::Warning))`.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// A tooltip on the tab, whether it can be selected or not: what a tab that opens says more of itself, an
  /// explanation beside its lock. It takes the place of the tooltip of a [`disabled_reason`](Self::disabled_reason),
  /// which stays the tab's description to tooling. A second takes the place of the first.
  pub fn tooltip(mut self, tooltip: Tooltip) -> Self {
    self.tooltip = Some(tooltip);
    self
  }

  /// A tooltip on the tab that says `text`: [`tooltip`](Self::tooltip) with a [`Tooltip::new`]. An empty text is
  /// none, and leaves the tab as it was.
  pub fn hint(self, text: &str) -> Self {
    if text.is_empty() {
      return self;
    }
    self.tooltip(Tooltip::new(text))
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

  /// Called when a person asks for what the tab stands for beside what is open, in place of selecting it: a press
  /// of the middle button, or a press with Ctrl held (Cmd on macOS), and Enter with it while the tab is focused.
  /// Such a press is the handler's alone: the tab is not selected, takes no focus, and the tabs' `on_change` is not
  /// called. It is given the press, whose `shift` asks for a tab that is switched to. Its item in the menu of the
  /// overflowing form is chosen as before. A second handler is called after the first.
  pub fn on_open_aside(mut self, handler: impl IntoMouseEventHandler) -> Self {
    self.on_open_aside.push(handler.into_event_handler());
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

/// What mounted tabs measure, before a frame is painted, to scroll where their list has no room: the tabs of a
/// list along the top that the app did not ask to overflow are their overflowing form while the list is wider
/// than its room, and the list alone while it fits.
#[derive(Clone)]
struct Fits {
  /// Read by the host and written where the form changes, or the list is scrolled to its selected tab: what
  /// renders the tabs again, and has the frame laid out again before it is painted.
  turn: Signal<u32>,
  /// The root: its width is the room of the list.
  root: ElementRef,
  held: Arc<Mutex<Held>>,
}

struct Held {
  room: Option<f32>,
  /// How wide the list is with every tab whole, measured where it was last laid out alone. The overflowing form
  /// draws its tabs smaller and closer, so its own width does not say whether the list alone would fit.
  needs: Option<f32>,
  /// The padding of the list after its last tab.
  end: f32,
  /// Whether the list has no room: the tabs are their overflowing form.
  scrolls: bool,
  /// Whether the tabs were last built as their list alone, of the tabs `needs` was measured of.
  plain: bool,
  /// What the tabs show that makes their list as wide as it is: `needs` is of these.
  shown: u64,
  /// The ref of the last tab at the start of the list alone: where it ends is how wide the list is, with the tabs at
  /// its end.
  last: ElementRef,
  /// Where the last tab at the start ends, and the padding after the list, as last laid out.
  start: Option<f32>,
  /// Of a list with tabs at its end: the ref of what holds them, and how wide it was last laid out.
  group: ElementRef,
  group_width: Option<f32>,
  /// Of a list with tabs at its end, the room between them and the tabs at the start; `None` for a list with none.
  tail: Option<f32>,
  /// Whether the list is to be scrolled to the selected tab: it has just become one that scrolls.
  reveal: bool,
}

impl Held {
  /// How wide the list alone is, from where its tabs at the start end and how wide those at its end are.
  fn measure(&mut self) {
    self.needs = match (self.start, self.tail) {
      (Some(start), None) => Some(start),
      (Some(start), Some(gap)) => self.group_width.map(|group| start + gap + group),
      (None, _) => None,
    };
  }

  /// Takes the form the room has room for, and says whether that is another.
  fn settle(&mut self) -> bool {
    let (Some(room), Some(needs)) = (self.room, self.needs) else {
      return false;
    };
    let scrolls = needs > room + 0.5;
    if scrolls == self.scrolls {
      return false;
    }
    (self.scrolls, self.reveal) = (scrolls, scrolls);
    true
  }
}

impl Fits {
  fn new(turn: Signal<u32>) -> Self {
    let fits = Self {
      turn,
      root: ElementRef::new(),
      held: Arc::new(Mutex::new(Held {
        room: None,
        needs: None,
        end: 0.0,
        scrolls: false,
        plain: false,
        shown: 0,
        last: ElementRef::new(),
        start: None,
        group: ElementRef::new(),
        group_width: None,
        tail: None,
        reveal: false,
      })),
    };
    // Written only where the room crosses what the list needs: a panel dragged within one side renders nothing.
    let (held, turn) = (fits.held.clone(), fits.turn.clone());
    fits.root.observe_rect_before_paint(move |rect| {
      let changed = {
        let mut held = held.lock();
        held.room = Some(rect.width);
        held.settle()
      };
      if changed {
        turn.update(|turn| *turn = turn.wrapping_add(1));
      }
    });
    fits
  }

  /// Whether tabs that show `shown`, whose list ends with a padding of `end`, are to be built as their
  /// overflowing form. `ends` is of a list with tabs at its end: the room between them and those at the start, and
  /// whether there are tabs at the start. Tabs that show something else are built as their list alone, which is
  /// measured before anything is painted.
  fn scrolls(&self, shown: u64, end: f32, ends: Option<(f32, bool)>) -> bool {
    let mut held = self.held.lock();
    held.end = end;
    if held.shown != shown {
      (held.shown, held.needs, held.scrolls, held.plain) = (shown, None, false, false);
    }
    if !held.scrolls && !held.plain {
      // A ref that was laid out before tells of a box only where it is another: a new one tells of its first.
      let (last, group) = (ElementRef::new(), ElementRef::new());
      let (state, turn) = (self.held.clone(), self.turn.clone());
      last.observe_rect_before_paint(move |rect| {
        let changed = {
          let mut held = state.lock();
          if !held.plain {
            return;
          }
          held.start = Some(rect.relative_x + rect.width + held.end);
          held.measure();
          held.settle()
        };
        if changed {
          turn.update(|turn| *turn = turn.wrapping_add(1));
        }
      });
      let (state, turn) = (self.held.clone(), self.turn.clone());
      group.observe_rect_before_paint(move |rect| {
        let changed = {
          let mut held = state.lock();
          if !held.plain {
            return;
          }
          held.group_width = Some(rect.width);
          held.measure();
          held.settle()
        };
        if changed {
          turn.update(|turn| *turn = turn.wrapping_add(1));
        }
      });
      (held.last, held.group, held.group_width) = (last, group, None);
      // With no tab at the start, the tabs at the end are all the list holds.
      (held.tail, held.start) = match ends {
        Some((gap, true)) => (Some(gap), None),
        Some((_, false)) => (Some(0.0), Some(end)),
        None => (None, None),
      };
    }
    held.plain = !held.scrolls;
    held.scrolls
  }

  /// Has the list scrolled the least that shows the tab of `tab` whole, once, where it has just become one that
  /// scrolls: before the frame that first shows it so is painted.
  fn reveal(&self, tab: &ElementRef, scroll: &ScrollState) {
    if !self.held.lock().reveal {
      return;
    }
    let (held, turn, scroll) = (self.held.clone(), self.turn.clone(), scroll.clone());
    tab.observe_rect_before_paint(move |rect| {
      if !std::mem::take(&mut held.lock().reveal) {
        return;
      }
      // The tab is in the list, which is what scrolls: where it is in the list is where it is in what scrolls.
      let (start, end) = (rect.relative_x, rect.relative_x + rect.width);
      let (at, room) = (scroll.scroll_x(), scroll.viewport_width());
      let to = if start < at {
        start
      } else if end > at + room {
        end - room
      } else {
        return;
      };
      scroll.set_scroll(to, scroll.scroll_y());
      turn.update(|turn| *turn = turn.wrapping_add(1));
    });
  }
}

/// One tab as it is built: what it shows, whether it is the selected one, and what a press of it does.
#[derive(Clone)]
pub struct TabEntry {
  pub label: Arc<str>,
  pub icon: Option<Name>,
  pub count: Option<Arc<str>>,
  /// The count in a pill, of what waits for the person.
  pub badge: Option<Arc<str>>,
  /// Why the tab cannot be selected, where the app said.
  pub reason: Option<Arc<str>>,
  pub disabled: bool,
  pub separated: bool,
  pub selected: bool,
  /// The dot after the count, in a role of the palette.
  pub dot: Option<PaletteColor>,
  /// Whether the count is still read: a bone in its place.
  pub count_pending: bool,
  /// Whether a press leaves the tab alone.
  pub inert: bool,
  /// The icon after the label, if it has one.
  pub trailing_icon: Option<Name>,
  /// Whether the tab stands at the end of the line.
  pub at_end: bool,
  /// The states of its own the tab is in ([`Tab::state`]).
  pub states: Vec<Name>,
  /// The tooltip on the tab, if the app gave it one ([`Tab::tooltip`]).
  tooltip: Option<Tooltip>,
  select: Arc<dyn Fn() + Send + Sync>,
  aside: Vec<EventHandler<crate::app::events::MouseEvent>>,
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
///
/// A list along the top that is wider than its room: tabs put in the tree with [`mount`](Self::mount) measure
/// their room and are their overflowing form while the list does not fit, with no frame of the other form, and
/// the list alone again when it does; tabs converted into an element on their own cannot measure, and end their
/// labels in an ellipsis, every tab still in the room.
pub struct Tabs<T = ()> {
  tabs: Vec<Tab<T>>,
  selected: Option<T>,
  write: Option<Write<T>>,
  on_change: Vec<Handler<T>>,
  variant: Name,
  orientation: TabsOrientation,
  disabled: bool,
  manual: bool,
  tab_stop: bool,
  label: Option<Arc<str>>,
  overflow: bool,
  note: Option<Arc<str>>,
  end: Option<Element>,
  reserve: bool,
  reselect: bool,
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
      tab_stop: true,
      label: None,
      overflow: false,
      note: None,
      end: None,
      reserve: false,
      reselect: false,
      words: Words {
        back: super::current_locale().navigation.scroll_back.clone(),
        on: super::current_locale().navigation.scroll_on.clone(),
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
  /// is told to none of them, and so is a press of the selected tab, unless the tabs [reselect](Self::reselect). A
  /// second handler is called after the first.
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

  /// Whether Tab stops in the list; it does, once, unless told otherwise. `false` is a list the pointer and the
  /// app's own shortcuts reach: Tab passes its tabs and its More button by, and a press of one leaves the focus
  /// where it was. A tab the app focuses is pressed by Enter and Space and its arrow keys move along the list as
  /// before, and the tabs are the same to what reads the tree. The panel is a stop of Tab as before.
  pub fn tab_stop(mut self, stops: bool) -> Self {
    self.tab_stop = stops;
    self
  }

  /// What the list is called, to what reads the tree: "Settings".
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Draws a list that runs along the top as the overflowing form whether
  /// or not it fits: the list scrolls between two scroll buttons, and a More
  /// button at its end opens a menu of every tab. Tabs put in the tree with
  /// [`mount`](Self::mount) keep where the list is scrolled and whether the
  /// menu is open, and take this form themselves while their list has no
  /// room, scrolled to the selected tab; converted into an element on their
  /// own they are its still form, whose menu does not open. A list that runs
  /// down scrolls with what holds it, and takes no notice of this.
  pub fn overflow(mut self, overflow: bool) -> Self {
    self.overflow = overflow;
    self
  }

  /// Quiet words at the end of the list's line, after the tabs: why a tab cannot be opened, when the list was read.
  /// They give way first where the line is short, and end in an ellipsis. In the default form they stand in the
  /// rail after the tabs; in the overflowing form at the end of the list that scrolls. An empty text is none.
  pub fn note(mut self, note: &str) -> Self {
    self.note = (!note.is_empty()).then(|| Arc::from(note));
    self
  }

  /// An element of the app's at the end of the list's line, after the tabs and the note: a button that acts on the
  /// page, a count of the app's. It is no tab: the arrow keys of the list do not move to it, and it is a stop of Tab
  /// of its own where it takes the focus. A second takes the place of the first.
  pub fn end(mut self, element: impl Into<Element>) -> Self {
    self.end = Some(element.into());
    self
  }

  /// Whether each tab's label keeps the room it takes in the selected tab's style, a heavier weight, selected or not,
  /// so that selecting another tab moves none after it, as a browser's tab bar keeps its tabs in place. Off unless
  /// told: a tab is as wide as its label is drawn. A list that gives way in a room too narrow for it (tabs built with
  /// nothing kept) keeps no room.
  pub fn reserve_selected_width(mut self, reserve: bool) -> Self {
    self.reserve = reserve;
    self
  }

  /// Whether a press of the selected tab, and choosing it in the menu of the overflowing form, tells
  /// [`on_change`](Self::on_change) its value again: a page whose selected tab is a place of its own, which a press
  /// opens again (back to the top of a list, out of a record opened from it). The signal of bound tabs is not set,
  /// its value being the same. Off unless told: a press of the selected tab tells nothing. The arrow keys never
  /// reselect, nor does an [inert](Tab::inert) tab.
  pub fn reselect(mut self, reselect: bool) -> Self {
    self.reselect = reselect;
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
    let reselect = self.reselect;
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
          badge: tab.badge,
          reason: tab.reason,
          disabled: tab.disabled,
          separated: tab.separated,
          selected: chosen,
          dot: tab.dot,
          count_pending: tab.count_pending,
          inert: tab.inert,
          trailing_icon: tab.trailing_icon,
          at_end: tab.at_end,
          states: tab.states,
          tooltip: tab.tooltip,
          // A press of the selected tab asks for nothing, and tells nothing unless the tabs reselect.
          select: Arc::new(move || {
            if chosen && !reselect {
              return;
            }
            if !chosen && write.as_ref().is_some_and(|write| !write(&value)) {
              return;
            }
            for handler in handlers.iter() {
              handler(&value);
            }
          }),
          aside: tab.on_open_aside,
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
      tab_stop: self.tab_stop,
      label: self.label,
      overflow: self.overflow,
      note: self.note,
      end: self.end,
      reserve: self.reserve,
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
  /// scrolled and whether its menu is open from one render to the next, and
  /// where a list along the top that has no room for its tabs becomes that
  /// form without being asked.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let mut tabs = self.prepare();
    let key = tabs.key.take();
    // Keyed, they keep what is theirs wherever they are among what else is mounted beside them.
    match key {
      Some(key) => ctx.mount_keyed::<TabsHost>(&key, TabsProps { tabs }).key(key),
      None => ctx.mount::<TabsHost>(TabsProps { tabs }),
    }
  }

  /// Builds the tabs in `theme`, with nothing kept: the overflowing form is
  /// its still form, scrolled to its start, whose menu does not open, and a
  /// list too wide for its room ends its labels in an ellipsis.
  /// Converting tabs into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    self.prepare().build(theme, Kept::new(Signal::new(false)), None)
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
  tab_stop: bool,
  label: Option<Arc<str>>,
  overflow: bool,
  note: Option<Arc<str>>,
  end: Option<Element>,
  reserve: bool,
  words: Words,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Prepared {
  /// What the tabs show that makes their list as wide as it is, as one number.
  fn shown(&self) -> u64 {
    let mut hasher = DefaultHasher::new();
    (self.variant.as_str(), self.tabs.len()).hash(&mut hasher);
    for state in &self.states {
      state.as_str().hash(&mut hasher);
    }
    for tab in &self.tabs {
      (
        &tab.label,
        &tab.count,
        &tab.badge,
        tab.icon.map(|icon| icon.as_str()),
        tab.separated,
        tab.dot.is_some(),
        tab.count_pending,
        tab.trailing_icon.map(|icon| icon.as_str()),
        tab.at_end,
      )
        .hash(&mut hasher);
      for state in &tab.states {
        state.as_str().hash(&mut hasher);
      }
    }
    self.reserve.hash(&mut hasher);
    hasher.finish()
  }

  /// The case of the tabs, as their overflowing form or not.
  fn case(&self, overflow: bool) -> Case {
    let vertical = self.orientation.is_vertical();
    Case {
      variant: Some(self.variant),
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(TabsState::Horizontal, !vertical)
    .state(TabsState::Vertical, vertical)
    .state(TabsState::Overflow, overflow)
  }

  /// The tabs in `theme`, their overflowing form with what `kept` keeps. Mounted tabs give what they measure,
  /// `fits`, and are that form while their list has no room.
  fn build(self, theme: &Theme, kept: Kept, fits: Option<&Fits>) -> Element {
    let vertical = self.orientation.is_vertical();
    // A list that runs down, or one the app asked to overflow, has one form whatever its room is.
    let fits = fits.filter(|_| !vertical && !self.overflow);
    let scrolls = fits.is_some_and(|fits| {
      let list = theme
        .resolve::<Tabs>(&self.case(false), Some(&self.rules))
        .part(TabsPart::List);
      let [_, end, _, _] = list.rest().paddings();
      let end = end.map(|end| end.resolve(&theme.spacing(), 0.0)).unwrap_or(0.0);
      let ends = self.tabs.iter().any(|tab| tab.at_end).then(|| {
        let gap = list
          .rest()
          .gap
          .map(|gap| gap.resolve(&theme.spacing(), 0.0))
          .unwrap_or(0.0);
        (gap, self.tabs.iter().any(|tab| !tab.at_end))
      });
      fits.scrolls(self.shown(), end, ends)
    });
    let overflow = ((self.overflow || scrolls) && !vertical).then_some(kept);
    let case = self.case(overflow.is_some());
    // A tab is in the state of the tabs, and in its own: selected, disabled, inert.
    let resolve = |selected: bool, disabled: bool, inert: bool| -> ResolvedRecipe {
      theme.resolve::<Tabs>(
        &case
          .clone()
          .state(TabsState::Selected, selected)
          .state(TabsState::Disabled, disabled)
          .state(TabsState::Inert, inert),
        Some(&self.rules),
      )
    };
    let tab_refs: Vec<ElementRef> = match overflow {
      Some(_) => self.tabs.iter().map(|_| ElementRef::new()).collect(),
      None => Vec::new(),
    };
    if let (Some(fits), Some(kept)) = (fits, &overflow)
      && let Some(selected) = self.tabs.iter().position(|tab| tab.selected)
    {
      fits.reveal(&tab_refs[selected], &kept.scroll);
    }
    // The list alone, along the top: mounted it is measured with every tab whole, and built with nothing kept its
    // tabs give way in a room too narrow for them.
    let alone = overflow.is_none() && !vertical;
    let plain = resolve(false, false, false).owned_by(self.id.as_ref());
    // By the bits of a tab's states: selected 1, disabled 2, inert 4.
    let recipes: [ResolvedRecipe; 8] = std::array::from_fn(|bits| {
      if bits == 0 {
        plain.clone()
      } else {
        resolve(bits & 1 != 0, bits & 2 != 0, bits & 4 != 0).owned_like(&plain)
      }
    });
    // A tab in states of its own is resolved in them too, as it is and as the selected tab (the room of its label).
    let own_recipes = self
      .tabs
      .iter()
      .map(|tab| {
        (!tab.states.is_empty()).then(|| {
          let disabled = self.disabled || tab.disabled;
          let as_selected = |selected: bool| {
            let case = tab.states.iter().fold(
              case
                .clone()
                .state(TabsState::Selected, selected)
                .state(TabsState::Disabled, disabled)
                .state(TabsState::Inert, tab.inert),
              |case, state| case.state(*state, true),
            );
            theme.resolve::<Tabs>(&case, Some(&self.rules)).owned_like(&plain)
          };
          [as_selected(tab.selected), as_selected(true)]
        })
      })
      .collect();
    let view = TabsView {
      tabs: self.tabs,
      variant: self.variant,
      orientation: self.orientation,
      disabled: self.disabled,
      manual: self.manual,
      tab_stop: self.tab_stop,
      label: self.label,
      note: self.note,
      end: self.end,
      recipes,
      own_recipes,
      list_ref: ElementRef::new(),
      tab_refs,
      room: fits.map(|fits| fits.root.clone()),
      last: fits.filter(|_| alone).map(|fits| fits.held.lock().last.clone()),
      group: fits.filter(|_| alone).map(|fits| fits.held.lock().group.clone()),
      gives: alone && fits.is_none(),
      reserve: self.reserve,
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

impl crate::app::component::DevtoolsInspectable for TabsProps {}

/// What mounted tabs keep from one render to the next: what their overflowing form keeps, and what they
/// measure to become it.
struct TabsHost {
  kept: Kept,
  fits: Fits,
}

impl Component for TabsHost {
  type Props = TabsProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      kept: Kept::new(ctx.signal(false)),
      fits: Fits::new(ctx.signal(0)),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let TabsProps { tabs } = ctx.props::<TabsProps>().clone();
    // Reading it renders the tabs again where what they measured asks for another form.
    let _ = self.fits.turn.get();
    tabs.build(&current_theme(), self.kept.clone(), Some(&self.fits))
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
  /// Whether Tab stops in the list and at its More button.
  tab_stop: bool,
  /// The quiet words at the end of the list's line.
  pub note: Option<Arc<str>>,
  /// What the app put at the end of the list's line.
  end: Option<Element>,
  /// The recipe of a tab in each case of its states, by their bits: selected 1, disabled 2, inert 4. The first,
  /// of none of them, is every other part's.
  recipes: [ResolvedRecipe; 8],
  /// The recipe of each tab in states of its own, by its index, as it is and as the selected tab: `None` for a tab in
  /// none.
  own_recipes: Vec<Option<[ResolvedRecipe; 2]>>,
  overflow: Option<Kept>,
  words: Words,
  /// Of the overflowing form, where the list and each tab were last laid out: what scrolls a tab chosen from
  /// the menu into view.
  list_ref: ElementRef,
  tab_refs: Vec<ElementRef>,
  /// Of mounted tabs that take the overflowing form themselves: the ref of the box whose width is their room.
  room: Option<ElementRef>,
  /// Of those tabs, as their list alone: the ref of the last tab at the start, which says how wide the list is, and
  /// of what holds the tabs at the end.
  last: Option<ElementRef>,
  group: Option<ElementRef>,
  /// Whether each label keeps the room it takes in the selected tab's style.
  reserve: bool,
  /// Whether the tabs give way in a list too wide for its room, their labels ending in an ellipsis: tabs built
  /// with nothing kept, which cannot measure their room.
  gives: bool,
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
    let forced = self.forced.filter(|_| self.entry() == Some(index));
    self.tab_recipe(index).part(part).forced(forced)
  }

  /// The recipe of the tab at `index`, in its states, naming its parts after the tab.
  fn tab_recipe(&self, index: usize) -> ResolvedRecipe {
    let tab = &self.tabs[index];
    let disabled = self.disabled || tab.disabled;
    let bits = usize::from(tab.selected) + 2 * usize::from(disabled) + 4 * usize::from(tab.inert);
    let recipe = match self.own_recipes.get(index) {
      Some(Some([own, _])) => own,
      _ => &self.recipes[bits],
    };
    // A tab with an id names its parts after it; one with a key only, after the tabs' id and the key.
    match (&tab.id, &tab.key) {
      (None, Some(key)) => recipe.keyed_item(key, TabsPart::Tab),
      (id, _) => recipe.item(id.as_ref(), TabsPart::Tab),
    }
  }

  /// The tab Tab stops on: the selected one, else the first that is not disabled. None where all are.
  pub fn entry(&self) -> Option<usize> {
    if self.disabled {
      return None;
    }
    let open = |tab: &TabEntry| !tab.disabled && !tab.inert;
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

  /// Of mounted tabs, the ref a renderer gives the box whose width is the room of the list: the root, which is
  /// as wide as what holds the tabs. They measure it to take their overflowing form while the list has no room;
  /// with the ref on no box they keep the form the app asked for. None for tabs that do not measure.
  pub fn room_ref(&self) -> Option<ElementRef> {
    self.room.clone()
  }

  /// The tab at `index`: what is pressed, with its icon, label and count. It
  /// is a tab to what reads the tree, called by its label, and says whether
  /// it is the selected one; the list's arrow keys move to it unless it is disabled.
  pub fn tab(&self, index: usize) -> components::Button {
    let entry = &self.tabs[index];
    let part = |part: TabsPart| self.tab_part(index, part);
    let select = entry.select.clone();
    let pressed = tab_stop(
      pressable(&part(TabsPart::Tab), self.disabled || entry.disabled),
      self.tab_stop,
    );
    let mut tab = if entry.inert {
      // Drawn as any tab, and taking nothing: no press, no focus, no stop of the arrows.
      pressed.focusable(false)
    } else {
      // Ahead of the selection, which a press that asks for the tab aside does not reach.
      opens_aside(pressed, &entry.aside)
        .focus_entry(entry.selected)
        .on_click(move |_| select())
    }
    .describe("a11y_role", "tab")
    .describe("a11y_name", entry.label.clone())
    .describe("a11y_selected", if entry.selected { "true" } else { "false" });
    if self.vertical() {
      tab = tab.justify(Justify::Start);
    }
    if let Some(name) = entry.icon {
      tab = tab.child(glyph(&part(TabsPart::Icon), name, false));
    }
    let label = part(TabsPart::Label);
    tab = if self.gives {
      tab
        .flex_shrink(1.0)
        .min_width(0.0)
        .child(one_line(&label, &entry.label))
    } else if self.reserve {
      tab.child(self.label_in_room(index, &label))
    } else {
      tab.child(label.text(&entry.label).nowrap().part(&label))
    };
    if let Some(count) = &entry.count {
      let style = part(TabsPart::Count);
      tab = tab.child(Row::new().part(&style).child(style.text(count).nowrap()));
    } else if entry.count_pending {
      tab = tab.child(self.count_bone(index)).describe("a11y_busy", "true");
    }
    if let Some(badge) = &entry.badge {
      let style = part(TabsPart::Badge);
      tab = tab.child(Row::new().part(&style).child(style.text(badge).nowrap()));
    }
    if let Some(color) = &entry.dot {
      tab = tab.child(Row::new().part(&part(TabsPart::Dot)).background(color.clone()));
    }
    if let Some(name) = entry.trailing_icon {
      tab = tab.child(glyph(&part(TabsPart::TrailingIcon), name, false));
    }
    if let Some(reason) = &entry.reason {
      tab = tab.describe("a11y_description", reason.clone());
    }
    if let Some(id) = &entry.id {
      tab = tab.id(id.clone());
    }
    if let Some(key) = &entry.key {
      tab = tab.key(key.clone());
    }
    if let Some(at) = self.tab_refs.get(index) {
      tab = tab.ref_element(at.clone());
    } else if let Some(last) = self
      .last
      .as_ref()
      .filter(|_| Some(index) == self.tabs.iter().rposition(|tab| !tab.at_end))
    {
      tab = tab.ref_element(last.clone());
    }
    tab
  }

  /// The label of the tab at `index` in the style of `label`, in the room it takes in the selected tab's style: a copy
  /// of it so drawn, no higher than nothing and hidden from tooling, holds the width.
  fn label_in_room(&self, index: usize, label: &ResolvedPart) -> Column {
    let entry = &self.tabs[index];
    let disabled = self.disabled || entry.disabled;
    let selected = match self.own_recipes.get(index) {
      Some(Some([_, own])) => own.part(TabsPart::Label),
      _ => self.recipes[1 + 2 * usize::from(disabled) + 4 * usize::from(entry.inert)].part(TabsPart::Label),
    };
    Column::new()
      .align_items(Alignment::Stretch)
      .child(label.text(&entry.label).nowrap().part(label))
      .child(
        Row::new()
          .height(0.0)
          .clip()
          .describe("a11y_hidden", "true")
          .child(selected.text(&entry.label).nowrap()),
      )
  }

  /// The bone in the place of the count of the tab at `index` while its count is read: a skeleton of the size the
  /// recipe gives [`TabsPart::CountSkeleton`] (its corners the skeleton's own), in the part [`TabsPart::Count`], so that a count that comes in its
  /// place moves nothing after it.
  pub fn count_bone(&self, index: usize) -> Element {
    let recipe = self.tab_recipe(index);
    let size = recipe.part(TabsPart::CountSkeleton).rest();
    let bone = super::Skeleton::block(
      size.width.unwrap_or(Length::Px(8.0)),
      size.height.unwrap_or(Length::Px(10.0)),
    )
    .rules(&recipe.lend(TabsPart::CountSkeleton));
    Row::new()
      .align_items(Alignment::Center)
      .part(&recipe.part(TabsPart::Count))
      .child(bone.build(self.theme()))
      .into()
  }

  /// The quiet words and the app's element at the end of the list's line, after a room that grows: they give way
  /// first where the line is short. `None` where the tabs have neither.
  pub fn end_of_line(&self) -> Option<Row> {
    if self.note.is_none() && self.end.is_none() {
      return None;
    }
    let mut row = Row::new()
      .flex_full(1.0, 1.0, None)
      .min_width(0.0)
      .align_items(Alignment::Center)
      .justify(Justify::End)
      .overflow_visible()
      // What is here is no tab: the arrow keys of the list pass it by.
      .focus_group(FocusGroup::none());
    if let Some(note) = &self.note {
      let part = self.part(TabsPart::Note);
      row = row.child(
        Row::new().flex_shrink(1.0).min_width(0.0).part(&part).child(
          part
            .text(note)
            .nowrap()
            .text_overflow(crate::components::TextOverflow::Elipsis),
        ),
      );
    }
    if let Some(end) = &self.end {
      row = row.child(
        Row::new()
          .flex_shrink(0.0)
          .align_items(Alignment::Center)
          .overflow_visible()
          .part(&self.part(TabsPart::End))
          .child(end.clone()),
      );
    }
    Some(row)
  }

  /// The end of a list with tabs at its end, along the top: a room that grows, with the note and the app's element at
  /// its end, and the tabs at `end` after them, as far apart as the list's tabs are.
  fn tail(&self, end: &[usize]) -> Element {
    let gap = self.part(TabsPart::List).rest().gap.unwrap_or_default();
    let mut group = Row::new()
      .flex_shrink(0.0)
      .align_items(Alignment::Center)
      .overflow_visible()
      .spacing(gap);
    for (at, index) in end.iter().enumerate() {
      if at > 0 && self.tabs[*index].separated {
        group = group.child(self.separator());
      }
      group = group.child(self.tab_element(*index));
    }
    if let Some(measured) = &self.group {
      group = group.ref_element(measured.clone());
    }
    Row::new()
      .flex_full(1.0, 1.0, None)
      .min_width(0.0)
      .align_items(Alignment::Center)
      .justify(Justify::End)
      .overflow_visible()
      .spacing(gap)
      .with_children(self.end_of_line())
      .child(group)
      .into()
  }

  /// The tab at `index` as the list holds it: [`tab`](Self::tab), with the app's tooltip where it gave one, else its
  /// reason as its tooltip where it has one.
  pub fn tab_element(&self, index: usize) -> Element {
    let tab = self.tab(index);
    let entry = &self.tabs[index];
    match (&entry.tooltip, &entry.reason) {
      (Some(tooltip), _) => tooltip.clone().declared(tab),
      (None, Some(reason)) => Tooltip::new(reason).declared(tab),
      (None, None) => tab.into(),
    }
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
    // Along the top, the tabs at the end stand after the room that grows; down the side, after the others.
    let (start, end): (Vec<usize>, Vec<usize>) = (0..self.tabs.len())
      .partition(|index| self.vertical() || !self.tabs[*index].at_end);
    for (at, index) in start.into_iter().enumerate() {
      if at > 0 && self.tabs[index].separated {
        children.push(self.separator());
      }
      children.push(self.tab_element(index));
    }
    if end.is_empty() {
      children.extend(self.end_of_line().map(Element::from));
    } else {
      children.push(self.tail(&end));
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
      .part(
        &self
          .part(TabsPart::ScrollButton)
          .keyed(if back { "back" } else { "on" }),
      )
      .describe("a11y_name", name)
      .child(glyph(
        &self.part(TabsPart::ScrollIcon).keyed(if back { "back" } else { "on" }),
        Name::new(icon),
        false,
      ))
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
      None => {
        let locale = super::current_locale();
        (locale.common.more.clone(), locale.navigation.more_tabs.clone())
      }
    };
    let mut button = tab_stop(pressable(&self.part(TabsPart::More), self.disabled), self.tab_stop)
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
        .disabled(self.disabled || entry.disabled || entry.inert)
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
      if let Some(room) = &self.room {
        column = column.ref_element(room.clone());
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

impl<T: Clone + PartialEq + Send + Sync + 'static> crate::ui::Styled for Tabs<T> {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

/// Every tabs have the one recipe and renderer, whatever their values are.
impl<T: Send + Sync + 'static> Themed for Tabs<T> {
  const NAME: &'static str = "tabs";
  type Renderer = TabsRenderer;

  fn parts() -> crate::ui::Declared {
    TabsPart::declared()
  }

  fn states() -> crate::ui::Declared {
    TabsState::declared()
  }

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
  /// - A badge, in every form: a pill of the primary fill after the label and the count, padded 6 and 1, its number
  ///   11/500 in the primary's foreground; on a selected pill tab the other way round. The board has none: it is
  ///   lurq's.
  /// - Pill: round tabs 4 apart, 30 high, padded 14, the label 13/500; the selected one in the primary fill.
  /// - The overflowing form (Tabs/Overflow): a row 40 high, the list between two scroll buttons 28 wide with chevrons
  ///   of 14 in the foreground, its tabs 20 apart with labels of 13; at the end a More button set apart by a line,
  ///   padded 8 and 4, its label 13 and a chevron of 14 in the muted foreground, 4 apart.
  fn recipe() -> Recipe {
    use PaletteColor::{Background, Foreground, Muted, MutedForeground, Primary, PrimaryForeground, Ring};
    use TabsPart::{
      Badge, Bar, Count, CountSkeleton, Dot, End, Icon, Label, List, More, MoreIcon, MoreLabel, Note, Panel, Root,
      ScrollButton, ScrollIcon, Separator, Tab, TrailingIcon,
    };
    use TabsState::{Disabled, Horizontal, Inert, Overflow, Selected, Vertical};
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
      .base(TrailingIcon, |s| s.square(IconSize::Sm))
      .base(Count, |s| s.typography(TypographyStyle::Caption).foreground(MutedForeground))
      .base(Badge, |s| {
        s.padding_x(6.0)
          .padding_y(1.0)
          .radius(RadiusSize::Full)
          .background(Primary)
          .foreground(PrimaryForeground)
          .typography(TypographyStyle::Caption)
          .font_size(11.0)
          .weight(FontWeight::Medium)
      })
      .base(Separator, |s| s.width(1.0).height(16.0).background(PaletteColor::Border))
      // lurq's, the board has none: a dot of 6 in the app's color; a count's bone of one digit, 7 by 10; the note
      // in a caption's type in the muted foreground, 16 from the tabs, and the app's element 8 after it.
      .base(Dot, |s| s.square(6.0).radius(RadiusSize::Full))
      .base(CountSkeleton, |s| s.width(7.0).height(10.0))
      .base(Note, |s| {
        s.padding_start(16.0)
          .typography(TypographyStyle::Caption)
          .foreground(MutedForeground)
      })
      .base(End, |s| s.padding_start(8.0))
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
      // An inert tab is drawn as it is, and does not answer the pointer.
      .state(Inert, Tab, |s| s.cursor(CursorIcon::Default))
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
      // On the primary fill the badge is the other way round.
      .when(when(Pill, &[Selected]), Badge, |s| s.background(PrimaryForeground).foreground(Primary))
      .when(when(Pill, &[Selected]), Count, |s| {
        s.foreground(PrimaryForeground).opacity(OpacityLevel::Muted)
      })
      // A disabled or inert tab does not light up under the pointer.
      .when(Selector::new().state(Disabled).hovered(), Tab, |s| s.foreground(MutedForeground))
      .when(Selector::new().state(Inert).hovered(), Tab, |s| s.foreground(MutedForeground))
      .when(Selector::new().state(Selected).state(Inert).hovered(), Tab, |s| s.foreground(Foreground))
      .when(when(Pill, &[Selected, Inert]).hovered(), Tab, |s| s.foreground(PrimaryForeground))
      .when(Selector::new().state(Selected).state(Disabled).hovered(), Tab, |s| {
        s.foreground(Foreground)
      })
      .when(when(Pill, &[Selected, Disabled]).hovered(), Tab, |s| {
        s.foreground(PrimaryForeground)
      })
  }
}
