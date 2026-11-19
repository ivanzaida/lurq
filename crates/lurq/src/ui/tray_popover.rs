//! A tray popover: the small panel a press of an app's tray icon opens, with who it is, how it is doing, a few
//! switches and a few commands.

use std::sync::Arc;

use super::{
  Switch, names,
  part_style::{Length, PartAnimation, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{FADE_IN, FADE_OUT, fading, glyph, one_line, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    ctx::{CollisionStrategy, Placement},
    events::MouseEvent,
    theme::{
      BorderSize, IconSize, MotionDuration, MotionEasing, OpacityLevel, PaletteColor, RadiusSize, ShadowStyle, Theme,
      TypographyStyle,
    },
  },
  components::{Column, Popup, Row},
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, FocusGroup, SpacingValue, border::Border, dimension::Dimension, outline::Outline},
};

/// The parts of a tray popover a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TrayPopoverPart {
  /// The panel: its width, fill, edge, corners and shadow, and the room between what it holds.
  Panel,
  /// The row at the top: the app's mark and its text.
  Header,
  /// The tile the app's mark stands in.
  Media,
  /// The icon in the tile.
  Icon,
  /// The title over the status.
  Text,
  /// What the app is called.
  Title,
  /// The dot and the words of how it is doing.
  Status,
  /// The dot of the status, in the color of its variant.
  StatusDot,
  /// The words of the status.
  Description,
  /// The room around a line between two groups of rows.
  Separator,
  /// The line itself.
  Line,
  /// A row with a switch at its end.
  Toggle,
  /// What a row with a switch says.
  ToggleLabel,
  /// A row that runs a command.
  Item,
  /// What the app puts among the rows.
  Body,
}

names!(TrayPopoverPart {
  Panel => "panel",
  Header => "header",
  Media => "media",
  Icon => "icon",
  Text => "text",
  Title => "title",
  Status => "status",
  StatusDot => "status-dot",
  Description => "description",
  Separator => "separator",
  Line => "line",
  Toggle => "toggle",
  ToggleLabel => "toggle-label",
  Item => "item",
  Body => "body",
});

/// How an app says it is doing in its tray popover: the color of the dot before the words.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TrayStatusVariant {
  /// Nothing to say of it: "Idle".
  Neutral,
  /// Something is going on: "Syncing 3 files".
  Info,
  /// All is well: "Synced just now". The design's.
  #[default]
  Success,
  /// Something to look at: "Offline".
  Warning,
  /// Something went wrong: "Sync failed".
  Destructive,
}

names!(TrayStatusVariant {
  Neutral => "neutral",
  Info => "info",
  Success => "success",
  Warning => "warning",
  Destructive => "destructive",
});

/// The states of a tray popover a recipe styles, besides what the pointer and the keyboard do to its rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TrayPopoverState {
  /// The row takes no input. Resolved on the row's part.
  Disabled,
}

names!(TrayPopoverState {
  Disabled => "disabled",
});

/// What builds a tray popover's panel: a theme's in place of the default one. Where the panel is shown, and when,
/// stays the popover's.
pub type TrayPopoverRenderer = Arc<dyn Fn(TrayPopoverView) -> Element + Send + Sync>;

type Handler = Arc<dyn Fn() + Send + Sync>;
type Changed = Arc<dyn Fn(bool) + Send + Sync>;

/// A row of a tray popover that runs a command: "Open lurq", "Quit".
///
/// ```
/// use lurq::ui::TrayAction;
///
/// let quit = TrayAction::new("Quit").on_select(|| std::process::exit(0));
/// ```
#[derive(Clone)]
pub struct TrayAction {
  label: Arc<str>,
  disabled: bool,
  on_select: Vec<Handler>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl TrayAction {
  /// A row that says `label`.
  pub fn new(label: &str) -> Self {
    Self {
      label: Arc::from(label),
      disabled: false,
      on_select: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// What a press of the row does, or Enter and Space while it is focused. An anchored popover closes first. A
  /// second handler is called after the first.
  pub fn on_select(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_select.push(Arc::new(handler));
    self
  }

  /// Whether the row takes no input: it is dimmed, and is not pressed.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// The id of the row's element.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }
}

/// A row of a tray popover with a switch at its end: "Sync in background".
///
/// ```
/// use lurq::{core::Signal, ui::TrayToggle};
///
/// // Bound to a signal the app keeps, or told what a press asks for.
/// let sync = TrayToggle::new("Sync in background").bind(Signal::new(true));
/// let pause = TrayToggle::new("Pause notifications").on(false).on_change(|on| println!("pause: {on}"));
/// ```
#[derive(Clone)]
pub struct TrayToggle {
  label: Arc<str>,
  on: bool,
  bound: Option<Signal<bool>>,
  disabled: bool,
  on_change: Vec<Changed>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl TrayToggle {
  /// A row that says `label`, its switch off.
  pub fn new(label: &str) -> Self {
    Self {
      label: Arc::from(label),
      on: false,
      bound: None,
      disabled: false,
      on_change: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// Whether the switch is on: the app's value, which a press does not change by itself.
  pub fn on(mut self, on: bool) -> Self {
    self.on = on;
    self
  }

  /// Binds the switch to a signal: it shows the signal's value, and a press sets it.
  pub fn bind(mut self, on: Signal<bool>) -> Self {
    self.on = on.get();
    self.bound = Some(on);
    self
  }

  /// Told the value a press asks for, after a bound signal is set. Not told when the app sets the signal. A
  /// second handler is called after the first.
  pub fn on_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Whether the row takes no input: it is dimmed, and its switch is not turned.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// The id of the row's switch: what is pressed and focused.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }
}

/// What a tray popover holds under its header: a command, a switch, a line, or something of the app's.
#[derive(Clone)]
pub enum TrayItem {
  /// A row that runs a command.
  Action(TrayAction),
  /// A row with a switch.
  Toggle(TrayToggle),
  /// A line between two groups of rows.
  Separator,
  /// Something of the app's own: a progress, a slider, an account.
  Element(Element),
}

impl From<TrayAction> for TrayItem {
  fn from(action: TrayAction) -> Self {
    Self::Action(action)
  }
}

impl From<TrayToggle> for TrayItem {
  fn from(toggle: TrayToggle) -> Self {
    Self::Toggle(toggle)
  }
}

/// What shows an anchored tray popover: whether it is open, and beside what.
struct Anchored {
  open: Signal<bool>,
  anchor: ElementRef,
}

/// A tray popover: the small panel a press of an app's icon in the system tray or the menu bar opens. Who the app
/// is and how it is doing, a few switches, a few commands.
///
/// ```
/// use lurq::{
///   core::{ElementRef, Signal},
///   ui::{TrayAction, TrayPopover, TrayToggle, icons},
/// };
///
/// // The panel where it is declared: what fills a window of its own beside the tray icon.
/// let panel = TrayPopover::new("lurq")
///   .icon(icons::BOX)
///   .status("Synced just now")
///   .item(TrayToggle::new("Sync in background").bind(Signal::new(true)))
///   .item(TrayToggle::new("Pause notifications").bind(Signal::new(false)))
///   .separator()
///   .item(TrayAction::new("Open lurq").on_select(|| println!("open")))
///   .item(TrayAction::new("Preferences…").on_select(|| println!("preferences")))
///   .item(TrayAction::new("Quit").on_select(|| println!("quit")));
///
/// // Over the app's own window, beside an element of its own: a status item of its title bar.
/// let (open, anchor) = (Signal::new(false), ElementRef::new());
/// let over = TrayPopover::new("lurq").item(TrayAction::new("Quit")).anchored(open, anchor);
/// ```
///
/// Declared plainly it is the panel in the flow, as wide as its recipe says: an app that has a tray icon opens a
/// window without a frame at the icon and renders the panel in it (lurq has no tray icon of its own: the icon and
/// its press come from the platform or a crate for it). [`anchored`](Self::anchored) shows it over the app's
/// window beside an element instead, as a popover is shown: Escape and a press outside close it, a press of a
/// command closes it and then runs the command, and the focus goes into it as it opens and back as it closes. The
/// arrow keys move through its switches and its commands.
pub struct TrayPopover {
  title: Arc<str>,
  media: Option<Element>,
  icon: Option<Name>,
  status: Option<Arc<str>>,
  status_variant: Name,
  items: Vec<TrayItem>,
  anchored: Option<Anchored>,
  placement: Placement,
  offset: SpacingValue,
  collision: CollisionStrategy,
  dismiss_on_escape: bool,
  dismiss_on_outside_press: bool,
  on_dismiss: Vec<Handler>,
  width: Option<Length>,
  label: Option<Arc<str>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl TrayPopover {
  /// A tray popover of the app called `title`, with nothing under its header yet.
  pub fn new(title: &str) -> Self {
    Self {
      title: Arc::from(title),
      media: None,
      icon: None,
      status: None,
      status_variant: TrayStatusVariant::Success.into(),
      items: Vec::new(),
      anchored: None,
      placement: Placement::BottomEnd,
      offset: 6.0.into(),
      collision: CollisionStrategy::FlipThenClamp,
      dismiss_on_escape: true,
      dismiss_on_outside_press: true,
      on_dismiss: Vec::new(),
      width: None,
      label: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// The app's mark before its title, in place of an icon on the tile: its own picture, 28 by 28.
  pub fn media(mut self, media: impl Into<Element>) -> Self {
    self.media = Some(media.into());
    self
  }

  /// The app's mark as an icon of the theme's set, on a tile of the primary color.
  pub fn icon(mut self, icon: impl Into<Name>) -> Self {
    self.icon = Some(icon.into());
    self
  }

  /// How the app is doing, under its title after a dot: "Synced just now".
  pub fn status(mut self, status: &str) -> Self {
    self.status = Some(Arc::from(status));
    self
  }

  /// The tone of the status, which colors its dot: a [`TrayStatusVariant`], `Success` by default, or a name the
  /// theme's recipe gives.
  pub fn status_variant(mut self, variant: impl Into<Name>) -> Self {
    self.status_variant = variant.into();
    self
  }

  /// Adds a row under those it has: a [`TrayAction`], a [`TrayToggle`], or a [`TrayItem`].
  pub fn item(mut self, item: impl Into<TrayItem>) -> Self {
    self.items.push(item.into());
    self
  }

  /// Adds several rows.
  pub fn items(mut self, items: impl IntoIterator<Item = impl Into<TrayItem>>) -> Self {
    self.items.extend(items.into_iter().map(Into::into));
    self
  }

  /// Adds a line under the rows it has, between two groups of them. The line under the header is there by itself.
  pub fn separator(mut self) -> Self {
    self.items.push(TrayItem::Separator);
    self
  }

  /// Adds something of the app's own under the rows it has: a progress, a slider, an account.
  pub fn child(mut self, child: impl Into<Element>) -> Self {
    self.items.push(TrayItem::Element(child.into()));
    self
  }

  /// Adds several things of the app's own.
  pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self
      .items
      .extend(children.into_iter().map(|child| TrayItem::Element(child.into())));
    self
  }

  /// Shows the panel over the window beside `anchor` while `open` is `true`, in place of where it is declared:
  /// under it, its end at the anchor's end, 6 from it.
  pub fn anchored(mut self, open: Signal<bool>, anchor: impl Into<ElementRef>) -> Self {
    self.anchored = Some(Anchored {
      open,
      anchor: anchor.into(),
    });
    self
  }

  /// Which side of its anchor an anchored panel is on, and which edge of it it lines up with:
  /// `Placement::BottomEnd` by default, as under a menu bar; `Placement::TopEnd` over a taskbar.
  pub fn placement(mut self, placement: Placement) -> Self {
    self.placement = placement;
    self
  }

  /// How far an anchored panel is from its anchor, along the side it is on: 6 by default.
  pub fn offset(mut self, offset: impl Into<SpacingValue>) -> Self {
    self.offset = offset.into();
    self
  }

  /// What an anchored panel does where there is no room for it on its side: goes to the other side, then is
  /// moved into the window, by default.
  pub fn collision(mut self, collision: CollisionStrategy) -> Self {
    self.collision = collision;
    self
  }

  /// Whether Escape closes an anchored panel. `true` by default.
  pub fn dismiss_on_escape(mut self, dismiss: bool) -> Self {
    self.dismiss_on_escape = dismiss;
    self
  }

  /// Whether a press outside an anchored panel and its anchor closes it. `true` by default.
  pub fn dismiss_on_outside_press(mut self, dismiss: bool) -> Self {
    self.dismiss_on_outside_press = dismiss;
    self
  }

  /// What runs after a person closed an anchored panel by a press outside it or Escape; not when the app sets the
  /// signal, and not on a command. A second handler is called after the first.
  pub fn on_dismiss(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_dismiss.push(Arc::new(handler));
    self
  }

  /// How wide the panel is, in place of what the theme gives it (280 in the design's).
  pub fn width(mut self, width: impl Into<Length>) -> Self {
    self.width = Some(width.into());
    self
  }

  /// What the panel is called to tooling that reads the tree, in place of its title.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Puts the tray popover in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this tray popover over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this tray popover.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws its first command as it is under the pointer, held down or focused from the keyboard, with nothing
  /// pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
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

  /// Builds the tray popover in `theme`. Converting one into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: Some(self.status_variant),
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    };
    // A width given here is under what `style` and `rules` say of the panel.
    let mut rules = Recipe::new();
    if let Some(width) = self.width {
      rules = rules.base(TrayPopoverPart::Panel, |s| s.width(width));
    }
    let rules = rules.extend(&self.rules);
    let recipe = theme
      .resolve::<TrayPopover>(&case, Some(&rules))
      .owned_by(self.id.as_ref());
    let disabled = theme
      .resolve::<TrayPopover>(&case.clone().state(TrayPopoverState::Disabled, true), Some(&rules))
      .owned_like(&recipe);
    let view = TrayPopoverView {
      title: self.title,
      icon: self.icon,
      status: self.status,
      status_variant: self.status_variant,
      anchored: self.anchored.is_some(),
      recipe,
      disabled,
      media: self.media,
      items: self.items,
      close: self.anchored.as_ref().map(|anchored| anchored.open.clone()),
      label: self.label,
      forced: self.forced,
      id: self.id,
      key: self.key,
    };
    let panel = match theme.renderer::<TrayPopover>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    };
    let Some(Anchored { open, anchor }) = self.anchored else {
      return panel;
    };
    // The offset is away from the anchor: along the side the panel is on.
    let offset = self.offset.resolve(&theme.spacing(), 0.0);
    let (x, y) = match self.placement {
      Placement::LeftStart
      | Placement::Left
      | Placement::LeftEnd
      | Placement::RightStart
      | Placement::Right
      | Placement::RightEnd => (offset, 0.0),
      _ => (0.0, offset),
    };
    let mut popup = Popup::new(anchor, panel)
      .open(open)
      .placement(self.placement)
      .offset(x, y)
      .collision(self.collision)
      .dismiss_on_escape(self.dismiss_on_escape)
      .dismiss_on_outside_click(self.dismiss_on_outside_press)
      .takes_focus(true);
    if !self.on_dismiss.is_empty() {
      let handlers = self.on_dismiss;
      popup = popup.on_dismiss(move || {
        for handler in &handlers {
          handler();
        }
      });
    }
    popup.into()
  }
}

impl From<TrayPopover> for Element {
  fn from(popover: TrayPopover) -> Self {
    popover.build(&current_theme())
  }
}

/// A tray popover's panel as a renderer sees it: what it was given and the style of its parts.
pub struct TrayPopoverView {
  pub title: Arc<str>,
  /// The icon on the tile, where the app gave no media of its own.
  pub icon: Option<Name>,
  pub status: Option<Arc<str>>,
  /// The tone of the status: a [`TrayStatusVariant`]'s name, or a theme's.
  pub status_variant: Name,
  /// Whether it is shown over the window beside an anchor, and not where it is declared.
  pub anchored: bool,
  /// The style of each part of this tray popover.
  pub recipe: ResolvedRecipe,
  /// The style of the parts of a row that takes no input.
  disabled: ResolvedRecipe,
  media: Option<Element>,
  items: Vec<TrayItem>,
  /// What a command closes before it runs: the signal of an anchored one.
  close: Option<Signal<bool>>,
  label: Option<Arc<str>>,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl TrayPopoverView {
  /// The style of a part, as this tray popover is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The style of a row's part: the disabled case's for a row that takes no input.
  fn row_part(&self, part: TrayPopoverPart, disabled: bool) -> ResolvedPart {
    if disabled {
      self.disabled.part(part)
    } else {
      self.recipe.part(part)
    }
  }

  /// The row at the top: the app's mark on its tile, its title, and how it is doing under it.
  pub fn header(&mut self) -> Row {
    let mut header = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .part(&self.part(TrayPopoverPart::Header));
    let tile = Row::new()
      .flex_shrink(0.0)
      .justify(Justify::Center)
      .align_items(Alignment::Center);
    match (self.media.take(), self.icon) {
      (Some(media), _) => header = header.child(tile.clip().child(media)),
      (None, Some(icon)) => {
        header = header.child(tile.part(&self.part(TrayPopoverPart::Media)).child(glyph(
          &self.part(TrayPopoverPart::Icon),
          icon,
          false,
        )));
      }
      (None, None) => {}
    }
    let title = self.part(TrayPopoverPart::Title);
    let mut text = Column::new()
      .flex(1.0)
      .min_width(Dimension::Px(0.0))
      .part(&self.part(TrayPopoverPart::Text))
      .child(one_line(&title, &self.title));
    if let Some(status) = &self.status {
      let description = self.part(TrayPopoverPart::Description);
      text = text.child(
        Row::new()
          .width(Dimension::Pct(100.0))
          .align_items(Alignment::Center)
          .part(&self.part(TrayPopoverPart::Status))
          .describe("a11y_role", "status")
          .describe("a11y_name", status.clone())
          .child(Row::new().flex_shrink(0.0).part(&self.part(TrayPopoverPart::StatusDot)))
          .child(one_line(&description, status)),
      );
    }
    header.child(text)
  }

  /// A line between two groups of rows, with the room around it.
  pub fn separator(&self) -> Element {
    Row::new()
      .width(Dimension::Pct(100.0))
      .part(&self.part(TrayPopoverPart::Separator))
      .describe("a11y_role", "separator")
      .child(
        Row::new()
          .width(Dimension::Pct(100.0))
          .part(&self.part(TrayPopoverPart::Line)),
      )
      .into()
  }

  /// A row with a switch at its end: a press of the row turns the switch as a press of the switch does.
  pub fn toggle(&self, toggle: TrayToggle) -> Element {
    let part = self.part(TrayPopoverPart::Toggle);
    let says = self.row_part(TrayPopoverPart::ToggleLabel, toggle.disabled);
    let TrayToggle {
      label,
      on,
      bound,
      disabled,
      on_change,
      id,
      key,
    } = toggle;
    // What a press asks for: the other value, set in a bound signal first.
    let turn = {
      let (bound, on_change) = (bound.clone(), on_change.clone());
      move || {
        let asked = match &bound {
          Some(signal) => {
            let asked = !signal.get_untracked();
            signal.set(asked);
            asked
          }
          None => !on,
        };
        for handler in &on_change {
          handler(asked);
        }
      }
    };
    let mut switch = Switch::new(&label).label_hidden(true).on(on).disabled(disabled);
    if let Some(signal) = bound {
      switch = switch.bind(signal);
    }
    for handler in on_change {
      switch = switch.on_change(move |on| handler(on));
    }
    if let Some(id) = id {
      switch = switch.id(id);
    }
    let mut row = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .part(&part)
      .child(
        Column::new()
          .flex(1.0)
          .min_width(Dimension::Px(0.0))
          .part(&says)
          .child(one_line(&part.clone().unnamed(), &label)),
      )
      // A press of the switch is the switch's own: the row does not turn it a second time.
      .child(
        Row::new()
          .flex_shrink(0.0)
          .on_click(|event: MouseEvent| event.stop_propagation())
          .child(switch),
      );
    if !disabled {
      row = row.on_click(move |_: MouseEvent| turn());
    }
    match key {
      Some(key) => row.key(key).into(),
      None => row.into(),
    }
  }

  /// A row that runs a command: an anchored popover closes, and then every handler is told.
  pub fn action(&self, action: TrayAction, forced: Option<Interaction>) -> Element {
    let row = if action.disabled { &self.disabled } else { &self.recipe };
    // A row with an id names its parts after it; one with a key only, after the popover's id and the key.
    let row = match (&action.id, &action.key) {
      (None, Some(key)) => row.keyed_item(key, TrayPopoverPart::Item),
      (id, _) => row.item(id.as_ref(), TrayPopoverPart::Item),
    };
    let part = row.part(TrayPopoverPart::Item).forced(forced);
    let (close, handlers) = (self.close.clone(), action.on_select);
    let mut row = pressable(&part, action.disabled)
      .width(Dimension::Pct(100.0))
      .justify(Justify::Start)
      .align_items(Alignment::Center)
      .focus_label(action.label.clone())
      .describe("a11y_role", "menuitem")
      .describe("a11y_name", action.label.clone())
      // Its label is in the row's style, and is no node of the row's part.
      .child(one_line(&part.clone().unnamed(), &action.label))
      .on_click(move |_: MouseEvent| {
        if let Some(open) = &close {
          open.set(false);
        }
        for handler in &handlers {
          handler();
        }
      });
    if action.disabled {
      row = row.describe("a11y_disabled", "true");
    }
    if let Some(id) = action.id {
      row = row.id(id);
    }
    if let Some(key) = action.key {
      row = row.key(key);
    }
    row.into()
  }

  /// The panel with nothing in it yet: the arrow keys move through what it holds. A dialog to what reads the tree
  /// where it is anchored, a group where it is declared, called by its label or its title.
  pub fn panel(&self) -> Column {
    let mut panel = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(TrayPopoverPart::Panel))
      .focus_group(FocusGroup::vertical().wrap(false))
      .describe("a11y_role", if self.anchored { "dialog" } else { "group" })
      .describe("a11y_name", self.label.clone().unwrap_or_else(|| self.title.clone()));
    if let Some(id) = &self.id {
      panel = panel.id(id.clone());
    }
    if let Some(key) = &self.key {
      panel = panel.key(key.clone());
    }
    panel
  }

  /// How the default renderer builds a tray popover: the header, a line, and its rows one under another.
  pub fn render(mut self) -> Element {
    let items = std::mem::take(&mut self.items);
    let mut panel = self.panel().child(self.header());
    if !items.is_empty() {
      panel = panel.child(self.separator());
    }
    let mut forced = self.forced;
    for item in items {
      panel = panel.child(match item {
        TrayItem::Action(action) => self.action(action, forced.take()),
        TrayItem::Toggle(toggle) => self.toggle(toggle),
        TrayItem::Separator => self.separator(),
        TrayItem::Element(element) => Row::new()
          .width(Dimension::Pct(100.0))
          .part(&self.part(TrayPopoverPart::Body))
          .child(element)
          .into(),
      });
    }
    panel.into()
  }
}

impl crate::ui::Styled for TrayPopover {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for TrayPopover {
  const NAME: &'static str = "tray_popover";
  const ROOT: &'static str = "panel";
  type Renderer = TrayPopoverRenderer;

  fn parts() -> crate::ui::Declared {
    TrayPopoverPart::declared()
  }

  fn states() -> crate::ui::Declared {
    TrayPopoverState::declared()
  }

  /// The design's tray popover (`pencil/lurq-design-system.pen`, Tray Popover `otBsr`, its Popover): a panel 280
  /// wide in the popover's colors, with an edge of 1 in the border's color, a radius of 12 and the large shadow,
  /// padded 10, what it holds 8 apart. Its header, 4 from the sides and 2 from the top and the bottom, has the
  /// app's mark, an icon of 16 on a tile of 28 in the primary color with the medium radius, 10 before its title
  /// (13, semibold) over the status 1 under it: a dot of 6 in the success color, 5 before words of 11 in the muted
  /// color. A line of 1 in the border's color has 4 over and under it. A row with a switch is 30 high and one
  /// that runs a command 28, both 4 from their sides with words of 13; the command under the pointer is in the
  /// highlight (`$accent`) with the small radius. The board draws the tray the popover hangs from, which is the
  /// platform's and not lurq's, and no row focused or disabled: a command has the ring from the keyboard inside
  /// it, and a row that takes no input is faded. Rows follow the theme's density. Anchored, it fades in and out as
  /// the popover does.
  fn recipe() -> Recipe {
    use PaletteColor::{
      Border as Edge, Destructive, Foreground, Highlight, Info, MutedForeground, Popover, PopoverForeground, Primary,
      PrimaryForeground, Ring, Success, Warning,
    };
    use TrayPopoverPart as P;
    use TrayStatusVariant as Tone;
    let words = |s: PartStyle| {
      s.typography(TypographyStyle::Body)
        .font_size(13.0)
        .foreground(Foreground)
    };
    Recipe::new()
      .base(P::Panel, |s| {
        s.width(280.0)
          .max_width(Length::Fill)
          .padding_x(10.0)
          .padding_y(10.0)
          .gap(8.0)
          .radius(RadiusSize::Xl)
          .background(Popover)
          .foreground(PopoverForeground)
          .border(Border::inside(BorderSize::Sm, Edge))
          .shadow(ShadowStyle::Lg)
          .enter(PartAnimation::new(FADE_IN, MotionDuration::Base, MotionEasing::Enter))
          .exit(PartAnimation::new(FADE_OUT, MotionDuration::Fast, MotionEasing::Exit))
      })
      .base(P::Header, |s| s.padding_x(4.0).padding_y(2.0).gap(10.0))
      .base(P::Media, |s| s.square(28.0).radius(RadiusSize::Md).background(Primary))
      .base(P::Icon, |s| {
        s.square(Length::Icon(IconSize::Md)).foreground(PrimaryForeground)
      })
      .base(P::Text, |s| s.gap(1.0))
      // The board's lines: 16 for the title and 13 for the status, so that the header's text is 30 high.
      .base(P::Title, |s| words(s).line_height(16.0 / 13.0).weight(FontWeight::SemiBold))
      .base(P::Status, |s| s.gap(5.0))
      .base(P::StatusDot, |s| {
        s.square(6.0).radius(RadiusSize::Full).background(MutedForeground)
      })
      .variant(Tone::Info, P::StatusDot, |s| s.background(Info))
      .variant(Tone::Success, P::StatusDot, |s| s.background(Success))
      .variant(Tone::Warning, P::StatusDot, |s| s.background(Warning))
      .variant(Tone::Destructive, P::StatusDot, |s| s.background(Destructive))
      .base(P::Description, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(11.0)
          .line_height(13.0 / 11.0)
          .foreground(MutedForeground)
      })
      .base(P::Separator, |s| s.padding_y(4.0))
      .base(P::Line, |s| s.height(1.0).background(Edge))
      .base(P::Toggle, |s| words(s).height(30.0).padding_x(4.0).gap(8.0))
      .row_density(Selector::new(), P::Toggle, 30.0)
      .base(P::Item, |s| {
        fading(
          words(s)
            .height(28.0)
            .padding_x(4.0)
            .radius(RadiusSize::Sm)
            .cursor(CursorIcon::Pointer),
        )
      })
      .row_density(Selector::new(), P::Item, 28.0)
      .hovered(P::Item, |s| s.background(Highlight))
      // The ring is inside the row: the rows are 4 from the panel's edge.
      .focus_visible(P::Item, |s| s.outline(Outline::new(BorderSize::Md, Ring).offset(-2.0)))
      .state(TrayPopoverState::Disabled, P::Item, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
      .state(TrayPopoverState::Disabled, P::ToggleLabel, |s| s.opacity(OpacityLevel::Disabled))
  }
}
