//! An action bar: the toolbar of a selection, over the list it acts on.

use std::{
  hash::{DefaultHasher, Hash, Hasher},
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
};

use parking_lot::Mutex;

use super::{
  button::{Button, ButtonPart, ButtonVariant},
  icons,
  menu::Menu,
  names,
  part_style::{Length, PartAnimation},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  separator::{Separator, SeparatorPart},
  theme::{Themed, current_theme},
  tooltip::Tooltip,
};
use crate::{
  animation::KeyframesId,
  app::{
    component::Component,
    ctx::{Ctx, Placement},
    events::KeyboardEvent,
    theme::{
      BorderSize, IconSize, MotionDuration, MotionEasing, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle,
    },
  },
  components::{Column, Row, TextOverflow},
  core::{ElementRef, Signal},
  layout::{
    Alignment,
    layout_kind::{Justify, ShrinkLimit},
    text_style::FontWeight,
  },
  node::{Element, FocusGroup, HitTestBehavior, NodeUpdate, border::Border, node::Node},
};

/// The keyframes a toast comes in and leaves with, which every window has
/// (`ui/toast.rs`): a fade, and a move of 16 from the edge it stands at.
const ENTER_FROM_BELOW: KeyframesId = KeyframesId::new(0xFF11);
const ENTER_FROM_ABOVE: KeyframesId = KeyframesId::new(0xFF12);
const LEAVE_DOWNWARD: KeyframesId = KeyframesId::new(0xFF13);
const LEAVE_UPWARD: KeyframesId = KeyframesId::new(0xFF14);

/// The edge of what it acts on that the bar stands at: the variant of its recipe.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ActionBarSide {
  /// At the bottom, in the middle; it comes up from the edge and goes back down.
  #[default]
  Bottom,
  /// At the top, in the middle; it comes down from the edge and goes back up.
  Top,
}

names!(ActionBarSide {
  Bottom => "bottom",
  Top => "top",
});

/// The parts of an action bar a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActionBarPart {
  /// The room over what the bar acts on, which it stands in: as large as that, and taking no press itself.
  Root,
  /// The bar: its fill, its edge, its shadow, how it comes in and leaves.
  Bar,
  /// How many are selected: "3 selected".
  Count,
  /// What the bar says of the line between the count, the actions and the clear button: laid over a separator's root.
  Separator,
  /// What the bar says of each action: laid over the theme's recipe for a button's root.
  Action,
  /// What the bar says of an action's icon: laid over a button's icon.
  ActionIcon,
  /// What the bar says of an action's label: laid over a button's label.
  ActionLabel,
  /// What the bar says of an action that destroys what it acts on: laid over a button's root, after `Action`.
  Destructive,
  /// What the bar says of the button that clears the selection: laid over a button's root.
  Clear,
  /// What the bar says of the icon of the clear button: laid over a button's icon.
  ClearIcon,
}

names!(ActionBarPart {
  Root => "root",
  Bar => "bar",
  Count => "count",
  Separator => "separator",
  Action => "action",
  ActionIcon => "action_icon",
  ActionLabel => "action_label",
  Destructive => "destructive",
  Clear => "clear",
  ClearIcon => "clear_icon",
});

/// The states of an action bar a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActionBarState {
  /// Nothing is selected: the bar is gone, or leaving.
  Empty,
}

names!(ActionBarState {
  Empty => "empty",
});

/// What builds an action bar's nodes: a theme's in place of the default one.
pub type ActionBarRenderer = Arc<dyn Fn(ActionBarView) -> Element + Send + Sync>;

/// One action of the bar: its button, and whether it destroys what it acts on.
#[derive(Clone)]
pub struct ActionBarAction {
  pub button: Button,
  pub destructive: bool,
}

/// An action bar: the toolbar of a selection. It stands over the list or the
/// grid whose rows are selected, says how many are, offers what can be done
/// to them, and clears them.
///
/// ```
/// use lurq::{
///   components::Stack,
///   node::Element,
///   ui::{ActionBar, Button, icons},
/// };
///
/// # fn build(list: Element, selected: usize) -> Stack {
/// Stack::new().child(list).child(
///   ActionBar::new(selected)
///     .action(Button::new("Move").icon(icons::FOLDER_IMPORT))
///     .action(Button::new("Duplicate").icon(icons::COPY))
///     .destructive_action(Button::new("Delete").icon(icons::TRASH))
///     .on_clear(|| println!("clear"))
///     .escape_clears(true),
/// )
/// # }
/// ```
///
/// It is as large as the room it is given, and takes no press itself: put it
/// over what it acts on, in a [`Stack`](crate::components::Stack). The bar
/// stands at the bottom of that room (or at its top), comes in while
/// something is selected and leaves as nothing is. It is a toolbar: Tab
/// stops in it once, and Left and Right move between its actions. Escape
/// clears the selection where the app says so ([`escape_clears`](Self::escape_clears)).
/// A text longer than the room gives way, cut with an ellipsis: the actions
/// and the clear button stay whole. Mounted ([`mount`](Self::mount)), a bar
/// that leaves with the focus in it gives the focus to what
/// [`return_focus`](Self::return_focus) names: the list it acted on; and a
/// bar wider than its room draws each action that has an icon as its icon
/// alone, called by its label in a tooltip, before its text gives way: the
/// first frame at a width is already so, and with room again the labels are
/// back. An action with no icon keeps its label. Narrower than that too, the
/// text is cut, and then the actions go, the last first, into the menu of a
/// More button before the clear button, which stays, as the text's room does
/// not: nothing of the bar is past its room.
#[derive(Clone)]
pub struct ActionBar {
  count: usize,
  count_text: Option<Arc<str>>,
  label: Option<Arc<str>>,
  actions: Vec<ActionBarAction>,
  on_clear: Vec<Arc<dyn Fn() + Send + Sync>>,
  side: ActionBarSide,
  escape_clears: bool,
  clear_label: Arc<str>,
  more_label: Arc<str>,
  return_focus: Option<ElementRef>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ActionBar {
  /// The bar of a selection of `count` things: shown while it is more than none.
  pub fn new(count: usize) -> Self {
    Self {
      count,
      count_text: None,
      label: None,
      actions: Vec::new(),
      on_clear: Vec::new(),
      side: ActionBarSide::default(),
      escape_clears: false,
      clear_label: Arc::from("Clear selection"),
      more_label: Arc::from("More actions"),
      return_focus: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// What the bar says of the selection, in place of "3 selected": "3 files".
  pub fn count_text(mut self, text: &str) -> Self {
    self.count_text = Some(Arc::from(text));
    self
  }

  /// What the bar is called to what reads the tree, in place of what it says
  /// of the selection: "Actions on 3 files".
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Adds an action on what is selected, after the ones the bar has: a
  /// button, which the bar makes a ghost one of its size. A bar has several.
  pub fn action(mut self, button: Button) -> Self {
    self.actions.push(ActionBarAction {
      button,
      destructive: false,
    });
    self
  }

  /// Adds actions on what is selected, after the ones the bar has.
  pub fn actions(self, buttons: impl IntoIterator<Item = Button>) -> Self {
    buttons.into_iter().fold(self, Self::action)
  }

  /// Adds an action that destroys what is selected: drawn in the destructive color.
  pub fn destructive_action(mut self, button: Button) -> Self {
    self.actions.push(ActionBarAction {
      button,
      destructive: true,
    });
    self
  }

  /// Adds actions that destroy what is selected, after the ones the bar has.
  pub fn destructive_actions(self, buttons: impl IntoIterator<Item = Button>) -> Self {
    buttons.into_iter().fold(self, Self::destructive_action)
  }

  /// What clearing the selection does, after the handlers there are: the
  /// button at the end of the bar, and Escape where
  /// [`escape_clears`](Self::escape_clears). Without one the bar has no clear
  /// button.
  pub fn on_clear(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_clear.push(Arc::new(handler));
    self
  }

  /// What the clear button is called, to what reads the tree: "Clear selection" by default.
  pub fn clear_label(mut self, label: &str) -> Self {
    self.clear_label = Arc::from(label);
    self
  }

  /// What the button that opens the menu of the actions a mounted bar has no room for is called: "More actions"
  /// by default.
  pub fn more_label(mut self, label: &str) -> Self {
    self.more_label = Arc::from(label);
    self
  }

  /// The edge of what it acts on that the bar stands at: the bottom, or the top.
  pub fn side(mut self, side: ActionBarSide) -> Self {
    self.side = side;
    self
  }

  /// Whether Escape clears the selection while the bar is shown, wherever the
  /// focus is, unless what has the focus took the key itself.
  pub fn escape_clears(mut self, clears: bool) -> Self {
    self.escape_clears = clears;
    self
  }

  /// Where the focus goes when the bar leaves with the focus in it: the
  /// selection cleared by its clear button or by Escape, or emptied by one of
  /// its actions. The list the bar acts on, or the row the selection started
  /// at; without it the focus leaves with the bar. A bar does this where it
  /// is mounted ([`mount`](Self::mount)), which keeps where the focus is.
  pub fn return_focus(mut self, element: ElementRef) -> Self {
    self.return_focus = Some(element);
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

  /// Puts the bar in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Draws the clear button as it is under the pointer, held down or focused
  /// from the keyboard, with nothing pointing at it: for a gallery or a test.
  /// An action is forced with its own `force`.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the bar. Its clear button has this id with `-clear` after it.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Puts the bar in the tree of `ctx`'s component, in the theme of what is
  /// being rendered: as [`build`](Self::build) does, and it keeps whether the
  /// focus is in the bar, so that the focus goes where
  /// [`return_focus`](Self::return_focus) says as the bar leaves with it.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let mut element = ctx.mount::<ActionBarHost>(ActionBarProps { bar: self });
    if let Some(key) = key {
      element = element.key(key);
    }
    element
  }

  /// Builds the bar in `theme`, with nothing kept: its actions with their
  /// labels in any room, its text giving way. Converting a bar into an
  /// element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    self.build_with(theme, None, None)
  }

  /// What the bar shows that makes it as wide as it is, as one number.
  fn shown(&self, count_text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    (count_text, self.actions.len(), self.on_clear.is_empty()).hash(&mut hasher);
    for action in &self.actions {
      let command = action.button.command();
      (
        &command.label,
        command.icon.map(|icon| icon.as_str()),
        command.icon_only,
      )
        .hash(&mut hasher);
    }
    hasher.finish()
  }

  /// The bar in `theme`. A mounted bar gives what it measures, `fits`, and its context, which its tooltips are
  /// mounted in: it draws its actions as their icons while it is wider than its room.
  fn build_with(self, theme: &Theme, fits: Option<&Fits>, ctx: Option<&mut Ctx>) -> Element {
    let count = self.actions.len();
    let mut case = Case::new()
      .variant(self.side)
      .breakpoint(super::current_breakpoint())
      .state(ActionBarState::Empty, self.count == 0);
    for state in &self.states {
      case = case.state(state.clone(), true);
    }
    let count_text = self
      .count_text
      .clone()
      .unwrap_or_else(|| Arc::from(format!("{} selected", self.count)));
    let recipe = theme.resolve::<ActionBar>(&case, Some(&self.rules));
    let fits = fits.filter(|_| self.count > 0);
    let compact = fits.is_some_and(|fits| {
      let spacing = theme.spacing();
      let sides = |part: ActionBarPart| {
        let [_, end, _, start] = recipe.part(part).rest().paddings();
        [start, end].map(|side| side.map(|side| side.resolve(&spacing, 0.0)).unwrap_or(0.0))
      };
      let [start, end] = sides(ActionBarPart::Root);
      fits.compact(self.shown(&count_text), sides(ActionBarPart::Bar)[1], start + end)
    });
    // Every handler of `on_clear`, in order, as one.
    let on_clear = (!self.on_clear.is_empty()).then(|| {
      let handlers = self.on_clear;
      Arc::new(move || {
        for handler in &handlers {
          handler();
        }
      }) as Arc<dyn Fn() + Send + Sync>
    });
    let mut view = ActionBarView {
      count: self.count,
      label: self.label.unwrap_or_else(|| count_text.clone()),
      count_text,
      actions: self.actions,
      on_clear,
      side: self.side,
      escape_clears: self.escape_clears,
      clear_label: self.clear_label,
      recipe,
      compact,
      tipped: Vec::new(),
      more: fits
        .filter(|_| compact)
        .map(|fits| fits.more(count, self.more_label.clone())),
      room: fits.map(|fits| fits.root.clone()),
      last: fits.map(|fits| fits.held.lock().last.clone()),
      forced: self.forced,
      id: self.id,
      key: self.key,
    };
    // An action drawn as its icon alone is called by its label in a tooltip, which is mounted with the bar.
    if compact && let Some(ctx) = ctx {
      view.tipped = (0..view.actions.len())
        .map(|index| {
          let command = view.actions[index].button.command();
          (command.icon.is_some() && !command.icon_only).then(|| {
            let element = view.dressed(index);
            Tooltip::new(&command.label)
              .placement(match view.side {
                ActionBarSide::Bottom => Placement::Top,
                ActionBarSide::Top => Placement::Bottom,
              })
              .mount(ctx, element)
          })
        })
        .collect();
    }
    match theme.renderer::<ActionBar>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<ActionBar> for Element {
  fn from(bar: ActionBar) -> Self {
    bar.build(&current_theme())
  }
}

#[derive(Clone)]
struct ActionBarProps {
  bar: ActionBar,
}

/// The bar is made anew by every render of what holds it, so props are never
/// the same as before.
impl PartialEq for ActionBarProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for ActionBarProps {}

/// What a mounted bar keeps from one render to the next: whether the focus is
/// on one of its buttons, and what it measures to draw its actions as icons.
struct ActionBarHost {
  focused: Arc<AtomicBool>,
  fits: Fits,
}

/// What a mounted bar measures, before a frame is painted, to draw its actions as their icons alone while it
/// is wider than its room.
#[derive(Clone)]
struct Fits {
  /// Read by the host and written where the form changes: what renders the bar again, and has the frame laid
  /// out again before it is painted.
  turn: Signal<u32>,
  /// Which actions the last layout dropped, whether the menu of them is open, and the button it hangs from.
  dropped: Signal<Vec<bool>>,
  open: Signal<bool>,
  anchor: ElementRef,
  /// The root: its width less its padding is the room of the bar.
  root: ElementRef,
  held: Arc<Mutex<Held>>,
}

struct Held {
  room: Option<f32>,
  /// How wide the bar is with every label, measured where it was last laid out so: with its icons alone it is
  /// narrower, and does not say whether the labels would fit.
  needs: Option<f32>,
  /// The padding of the bar after its last button, and of the root on both sides.
  end: f32,
  sides: f32,
  /// Whether the bar has no room for its labels: its actions are their icons.
  compact: bool,
  /// Whether the bar was last built with its labels, of what `needs` was measured of.
  whole: bool,
  /// What the bar shows that makes it as wide as it is: `needs` is of this.
  shown: u64,
  /// The ref of what holds the last button of the bar with its labels: where it ends is how wide the bar is.
  last: ElementRef,
  /// Where each action was laid out with its icon alone: one the bar has no room for has no size.
  slots: Vec<ElementRef>,
}

/// The actions a mounted bar has no room for even as icons, and the More button that opens the menu of them.
struct More {
  /// Which actions the last layout dropped, by index: they are in the menu.
  dropped: Vec<bool>,
  told: Signal<Vec<bool>>,
  /// Whether the menu is open, and the button it hangs from.
  open: Signal<bool>,
  anchor: ElementRef,
  slots: Vec<ElementRef>,
  label: Arc<str>,
}

impl Held {
  /// Takes the form the room has room for, and says whether that is another.
  fn settle(&mut self) -> bool {
    let (Some(room), Some(needs)) = (self.room, self.needs) else {
      return false;
    };
    let compact = needs > room - self.sides + 0.5;
    std::mem::replace(&mut self.compact, compact) != compact
  }
}

impl Fits {
  fn new(turn: Signal<u32>, dropped: Signal<Vec<bool>>, open: Signal<bool>) -> Self {
    let fits = Self {
      turn,
      dropped,
      open,
      anchor: ElementRef::new(),
      root: ElementRef::new(),
      held: Arc::new(Mutex::new(Held {
        room: None,
        needs: None,
        end: 0.0,
        sides: 0.0,
        compact: false,
        whole: false,
        shown: 0,
        last: ElementRef::new(),
        slots: Vec::new(),
      })),
    };
    // Written only where the room crosses what the labels need: a panel dragged within one side renders nothing.
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

  /// Whether a bar that shows `shown`, padded `end` after its last button in a root padded `sides`, is to be
  /// built with its actions as icons. A bar that shows something else is built with its labels, which is
  /// measured before anything is painted.
  fn compact(&self, shown: u64, end: f32, sides: f32) -> bool {
    let mut held = self.held.lock();
    (held.end, held.sides) = (end, sides);
    if held.shown != shown {
      (held.shown, held.needs, held.compact, held.whole) = (shown, None, false, false);
    }
    if !held.compact && !held.whole {
      // A ref that was laid out before tells of a box only where it is another: a new one tells of its first.
      let last = ElementRef::new();
      let (state, turn) = (self.held.clone(), self.turn.clone());
      last.observe_rect_before_paint(move |rect| {
        let changed = {
          let mut held = state.lock();
          if !held.whole {
            return;
          }
          held.needs = Some(rect.relative_x + rect.width + held.end);
          held.settle()
        };
        if changed {
          turn.update(|turn| *turn = turn.wrapping_add(1));
        }
      });
      held.last = last;
    }
    held.whole = !held.compact;
    held.compact
  }

  /// What the bar of `count` actions with their icons alone drops, and the More button of it called `label`.
  /// Reading it renders the bar again as an action goes or comes back.
  fn more(&self, count: usize, label: Arc<str>) -> More {
    let mut dropped = self.dropped.get();
    dropped.resize(count, false);
    let mut held = self.held.lock();
    while held.slots.len() < count {
      held.slots.push(ElementRef::new());
    }
    More {
      dropped,
      told: self.dropped.clone(),
      open: self.open.clone(),
      anchor: self.anchor.clone(),
      slots: held.slots[..count].to_vec(),
      label,
    }
  }
}

impl More {
  /// `action`, the one at `index`, as what drops out of the bar whole where the bar has no room for it, after the
  /// text has given way, and says so through its ref.
  fn slot(&self, index: usize, action: Element) -> Element {
    let slot = self.slots[index].clone();
    let (told, open) = (self.told.clone(), self.open.clone());
    // A dropped action is laid out at no size: that is how the bar learns which are in the menu, before the frame
    // is painted.
    slot.observe_rect_before_paint(move |rect| {
      let now = rect.width <= 0.0 && rect.height <= 0.0;
      if told.get_untracked().get(index).copied().unwrap_or(false) == now {
        return;
      }
      told.update(|told| {
        if told.len() <= index {
          told.resize(index + 1, false);
        }
        told[index] = now;
      });
      // With every action back in the bar the More button is gone, and its menu with it.
      if open.get_untracked() && !told.get_untracked().contains(&true) {
        open.set(false);
      }
    });
    Row::new()
      .overflow_visible()
      .flex_shrink(1.0)
      .shrink_limit(ShrinkLimit::Drop)
      .shrink_order(1)
      .ref_element(slot)
      .child(action)
      .into()
  }
}

/// Has each stop of Tab in `node` say whether it has the focus in `focused`.
fn watch_focus(node: &mut Node, focused: &Arc<AtomicBool>) {
  if node.tab_index_value().is_some_and(|index| index >= 0) && node.is_focusable() {
    let (gained, lost) = (focused.clone(), focused.clone());
    NodeUpdate::on_focus(node, move || gained.store(true, Ordering::Relaxed));
    NodeUpdate::on_blur(node, move || lost.store(false, Ordering::Relaxed));
  }
  for child in &mut node.children {
    watch_focus(child, focused);
  }
}

impl Component for ActionBarHost {
  type Props = ActionBarProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      focused: Arc::new(AtomicBool::new(false)),
      fits: Fits::new(ctx.signal(0), ctx.signal(Vec::new()), ctx.signal(false)),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let ActionBarProps { bar } = ctx.props::<ActionBarProps>().clone();
    // The bar leaves with the focus in it: the focus goes where the app said, and is not lost with the bar.
    if bar.count == 0
      && self.focused.swap(false, Ordering::Relaxed)
      && let Some(element) = &bar.return_focus
    {
      ctx.focus(element);
    }
    // Reading it renders the bar again where what it measured asks for the other form. A bar a theme builds its
    // own way is as that builds it.
    let _ = self.fits.turn.get();
    let theme = current_theme();
    let fits = theme.renderer::<ActionBar>().is_none().then_some(&self.fits);
    let mut element = bar.build_with(&theme, fits, Some(ctx));
    watch_focus(&mut element.node, &self.focused);
    element
  }
}

/// An action bar as a renderer sees it: what it says, its actions, the
/// style of its parts, and what clears the selection.
pub struct ActionBarView {
  pub count: usize,
  /// What it says of the selection: "3 selected".
  pub count_text: Arc<str>,
  /// What it is called to what reads the tree: its label, or what it says.
  pub label: Arc<str>,
  pub actions: Vec<ActionBarAction>,
  /// Every handler of `on_clear`, in order, as one.
  pub on_clear: Option<Arc<dyn Fn() + Send + Sync>>,
  pub side: ActionBarSide,
  pub escape_clears: bool,
  pub clear_label: Arc<str>,
  /// The style of each part of this bar.
  pub recipe: ResolvedRecipe,
  /// Whether the bar is wider than its room with its labels: an action that has an icon is its icon alone.
  pub compact: bool,
  /// The actions that are their icons alone, each with its tooltip: built with the bar, once.
  tipped: Vec<Option<Element>>,
  /// Of a mounted bar whose actions are their icons: which of them it dropped, and the More button of those.
  more: Option<More>,
  /// Of a mounted bar: the ref of the box whose width is its room, and of what holds its last button.
  room: Option<ElementRef>,
  last: Option<ElementRef>,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ActionBarView {
  /// The style of a part, as this bar is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The line between the count, the actions and the clear button.
  pub fn separator(&self) -> Element {
    Separator::vertical()
      .rules(&self.recipe.rules_for(ActionBarPart::Separator, SeparatorPart::Root))
      .build(self.theme())
  }

  /// How many are selected, on one line: where the bar is narrower than
  /// what it holds, this gives way and is cut with an ellipsis. A mounted
  /// bar draws its actions as icons first: with their labels it is measured
  /// whole, and this keeps its width.
  pub fn count(&self) -> Element {
    let part = self.part(ActionBarPart::Count);
    let whole = self.last.is_some() && !self.compact;
    Row::new()
      .flex_shrink(if whole { 0.0 } else { 1.0 })
      // It gives way before any action goes into the menu.
      .shrink_order(0)
      .min_width(0.0)
      .child(
        part
          .text(&self.count_text)
          .nowrap()
          .text_overflow(TextOverflow::Elipsis)
          .part(&part),
      )
      .into()
  }

  /// The action at `index`, a ghost button in the bar's dress: its icon alone, with its tooltip, where the bar
  /// is wider than its room and it has one.
  pub fn action(&mut self, index: usize) -> Element {
    match self.tipped.get_mut(index).and_then(Option::take) {
      Some(tipped) => tipped,
      None => self.dressed(index),
    }
  }

  /// The button of the action at `index`.
  fn dressed(&self, index: usize) -> Element {
    let action = &self.actions[index];
    let mut rules = self
      .recipe
      .rules_for(ActionBarPart::Action, ButtonPart::Root)
      .extend(&self.recipe.rules_for(ActionBarPart::ActionIcon, ButtonPart::Icon))
      .extend(&self.recipe.rules_for(ActionBarPart::ActionLabel, ButtonPart::Label));
    if action.destructive {
      rules = rules.extend(&self.recipe.rules_for(ActionBarPart::Destructive, ButtonPart::Root));
    }
    let mut button = action.button.clone().variant(ButtonVariant::Ghost).rules(&rules);
    if self.compact && action.button.command().icon.is_some() {
      button = button.icon_only(true);
    }
    button.build(self.theme())
  }

  /// The button that clears the selection, if the app said what clearing does.
  pub fn clear(&self) -> Option<Element> {
    let clear = self.on_clear.clone()?;
    let rules = self
      .recipe
      .rules_for(ActionBarPart::Clear, ButtonPart::Root)
      .extend(&self.recipe.rules_for(ActionBarPart::ClearIcon, ButtonPart::Icon));
    let mut button = Button::new(&self.clear_label)
      .icon(icons::CLOSE)
      .icon_only(true)
      .variant(ButtonVariant::Ghost)
      .rules(&rules)
      .force(self.forced)
      .on_click(move |_| clear());
    if let Some(id) = &self.id {
      button = button.id(format!("{id}-clear"));
    }
    Some(button.build(self.theme()))
  }

  /// The More button of a mounted bar whose actions are their icons, and the menu it opens of the actions the
  /// bar has no room for, in their order, the one that destroys in the destructive color. The bar drops it
  /// whole while every action is in it.
  fn more_button(&self) -> Option<Element> {
    let more = self.more.as_ref()?;
    let rules = self
      .recipe
      .rules_for(ActionBarPart::Action, ButtonPart::Root)
      .extend(&self.recipe.rules_for(ActionBarPart::ActionIcon, ButtonPart::Icon));
    let open = more.open.clone();
    let mut button = Button::new(&more.label)
      .icon(icons::MORE_HORIZONTAL)
      .icon_only(true)
      .variant(ButtonVariant::Ghost)
      .rules(&rules)
      .on_click(move |_| open.update(|open| *open = !*open));
    if let Some(id) = &self.id {
      button = button.id(format!("{id}-more"));
    }
    let mut element = button.build(self.theme());
    NodeUpdate::describe(&mut *element.node, Arc::from("a11y_haspopup"), Arc::from("menu"));
    let expanded = if more.open.get() { "true" } else { "false" };
    NodeUpdate::describe(&mut *element.node, Arc::from("a11y_expanded"), Arc::from(expanded));
    NodeUpdate::ref_element(&mut *element.node, more.anchor.clone());
    let mut menu = Menu::new(more.open.clone(), more.anchor.clone())
      .label(&more.label)
      .placement(match self.side {
        ActionBarSide::Bottom => Placement::TopEnd,
        ActionBarSide::Top => Placement::BottomEnd,
      });
    if let Some(id) = &self.id {
      menu = menu.id(format!("{id}-menu"));
    }
    for (index, action) in self.actions.iter().enumerate() {
      if more.dropped.get(index).copied().unwrap_or(false) {
        menu = menu.item(action.button.command().item(None).destructive(action.destructive));
      }
    }
    Some(
      Row::new()
        .overflow_visible()
        .drop_marker(true)
        .child(element)
        .child(menu.build(self.theme()))
        .into(),
    )
  }

  /// The bar: the count, a line, the actions, a line and the clear button.
  /// It is a toolbar to what reads the tree, called by its count, and one
  /// stop of Tab whose Left and Right move between its buttons.
  pub fn bar(&mut self) -> Row {
    let mut bar = Row::new()
      .part(&self.part(ActionBarPart::Bar))
      .align_items(Alignment::Center)
      .focus_group(FocusGroup::horizontal().one_stop(true))
      .key("action-bar")
      .describe("a11y_role", "toolbar")
      .describe("a11y_name", self.label.clone())
      .describe("a11y_orientation", "horizontal")
      .child(self.count());
    if !self.actions.is_empty() {
      bar = bar.child(self.separator());
    }
    // With its labels a mounted bar is measured whole: nothing in it gives way, and what it ends with is held by
    // what says where it ends, its last action or the line and the clear button.
    let whole = self.last.is_some() && !self.compact;
    let clear = self.clear();
    let count = self.actions.len();
    for index in 0..count {
      let mut action = self.action(index);
      if whole {
        NodeUpdate::flex_shrink(&mut *action.node, 0.0);
      }
      if let Some(more) = &self.more {
        action = more.slot(index, action);
      }
      if let Some(last) = self
        .last
        .as_ref()
        .filter(|_| whole && clear.is_none() && index + 1 == count)
      {
        action = Row::new()
          .overflow_visible()
          .flex_shrink(0.0)
          .ref_element(last.clone())
          .child(action)
          .into();
      }
      bar = bar.child(action);
    }
    if let Some(more) = self.more_button() {
      bar = bar.child(more);
    }
    match (clear, &self.last) {
      (Some(clear), Some(last)) => {
        let gap = self
          .part(ActionBarPart::Bar)
          .rest()
          .gap
          .map(|gap| gap.resolve(&self.theme().spacing(), 0.0))
          .unwrap_or(0.0);
        bar = bar.child(
          Row::new()
            .spacing(gap)
            .align_items(Alignment::Center)
            .overflow_visible()
            .flex_shrink(0.0)
            .ref_element(last.clone())
            .child(self.separator())
            .child(clear),
        );
      }
      (Some(clear), None) => bar = bar.child(self.separator()).child(clear),
      (None, _) => {}
    }
    if let Some(id) = &self.id {
      bar = bar.id(id.clone());
    }
    bar
  }

  /// How the default renderer builds an action bar: the room over what it
  /// acts on, and in it the bar while something is selected. The room keeps
  /// the bar while it leaves.
  pub fn render(mut self) -> Element {
    let justify = match self.side {
      ActionBarSide::Bottom => Justify::End,
      ActionBarSide::Top => Justify::Start,
    };
    let mut root = Column::new()
      .part(&self.part(ActionBarPart::Root))
      .presence()
      .justify(justify)
      .align_items(Alignment::Center)
      .overflow_visible()
      .hit_test(HitTestBehavior::ContentOnly);
    if let Some(room) = &self.room {
      root = root.ref_element(room.clone());
    }
    if self.escape_clears
      && self.count > 0
      && let Some(clear) = self.on_clear.clone()
    {
      root = root.on_key_down(move |event: KeyboardEvent| {
        let plain = !event.ctrl && !event.alt && !event.meta && !event.shift;
        if plain && event.key == "Escape" && !event.default_prevented() {
          event.prevent_default();
          clear();
        }
      });
    }
    if self.count > 0 {
      root = root.child(self.bar());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

impl Themed for ActionBar {
  const NAME: &'static str = "action_bar";
  type Renderer = ActionBarRenderer;

  /// The design's action bar (`pencil/lurq-design-system.pen`, Action Bar): a
  /// bar 44 high of the popover's color, round at its ends, with an edge of 1
  /// in the border's color and the large shadow, padded 16 before its count
  /// and 6 after its clear button, its parts 4 apart. The count is 13/600; a
  /// line of 20 with 6 on either side sets it apart from the actions, which
  /// are round, 32 high, padded 10, with an icon of 14 and a label of 13, 6
  /// apart; the one that destroys is in the destructive color. The clear
  /// button is a round 32 with a cross of 15. It stands 16 from the bottom of
  /// what it acts on, and comes in with a fade and a move of 16 from the edge.
  fn recipe() -> Recipe {
    use ActionBarPart::*;
    use PaletteColor::{Destructive as Danger, Foreground, Popover, PopoverForeground};
    Recipe::new()
      .base(Root, |s| {
        s.width(Length::Fill)
          .height(Length::Fill)
          .padding_x(16.0)
          .padding_y(16.0)
      })
      .base(Bar, |s| {
        s.height(44.0)
          .padding_start(16.0)
          .padding_end(6.0)
          .gap(4.0)
          .radius(RadiusSize::Full)
          .background(Popover)
          .foreground(PopoverForeground)
          .border(Border::inside(BorderSize::Sm, PaletteColor::Border))
          .shadow(ShadowStyle::Lg)
      })
      .variant(ActionBarSide::Bottom, Bar, |s| {
        s.enter(PartAnimation::new(
          ENTER_FROM_BELOW,
          MotionDuration::Base,
          MotionEasing::Enter,
        ))
        .exit(PartAnimation::new(
          LEAVE_DOWNWARD,
          MotionDuration::Fast,
          MotionEasing::Exit,
        ))
      })
      .variant(ActionBarSide::Top, Bar, |s| {
        s.enter(PartAnimation::new(
          ENTER_FROM_ABOVE,
          MotionDuration::Base,
          MotionEasing::Enter,
        ))
        .exit(PartAnimation::new(
          LEAVE_UPWARD,
          MotionDuration::Fast,
          MotionEasing::Exit,
        ))
      })
      .base(Count, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .weight(FontWeight::SemiBold)
          .foreground(Foreground)
      })
      .base(Separator, |s| s.padding_x(6.0).height(20.0))
      .base(Action, |s| {
        s.height(32.0)
          .padding_x(10.0)
          .gap(6.0)
          .radius(RadiusSize::Full)
          .foreground(Foreground)
      })
      .base(ActionIcon, |s| s.square(IconSize::Sm))
      .base(ActionLabel, |s| s.font_size(13.0).weight(FontWeight::Normal))
      .base(Destructive, |s| s.foreground(Danger))
      .base(Clear, |s| s.square(32.0).padding_x(0.0).radius(RadiusSize::Full))
      .base(ClearIcon, |s| s.square(15.0))
  }
}
