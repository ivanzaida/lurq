//! A notification center: the panel that lists an app's notifications, opened from its bell.

use std::sync::Arc;

use super::{
  Link, LinkPart, Notification, NotificationVariant, icons, names,
  part_style::{Length, PartAnimation, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{FADE_IN, FADE_OUT, fading, glyph, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    ctx::{CollisionStrategy, Placement},
    theme::{
      BorderSize, MotionDuration, MotionEasing, PaletteColor, RadiusSize, ShadowStyle, SpacingSize, Theme,
      TypographyStyle,
    },
  },
  components::{Column, Popup, Row, ScrollVertical},
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{
    CursorIcon, Element, FocusGroup, Node, NodeUpdate, SpacingValue, border::Border, dimension::Dimension,
    outline::Outline,
  },
};

/// The parts of a notification center a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NotificationCenterPart {
  /// The panel: its width, fill, edge, corners and shadow.
  Panel,
  /// The row at the top: the title, and what marks all read or clears at its end.
  Header,
  /// What the panel is called.
  Title,
  /// The controls at the end of the header.
  Actions,
  /// A control of the header: what it gives the theme's link.
  Action,
  /// The words of a control of the header: what they give the link's label.
  ActionLabel,
  /// What scrolls: the notifications, one under another.
  List,
  /// What holds a notification that has another under it: the line between the two.
  Item,
  /// What stands in place of the list while there is nothing in it.
  Empty,
  /// The icon of the empty panel.
  EmptyIcon,
  /// What the empty panel says.
  EmptyText,
  /// The row at the bottom that leads to all of them.
  Footer,
}

names!(NotificationCenterPart {
  Panel => "panel",
  Header => "header",
  Title => "title",
  Actions => "actions",
  Action => "action",
  ActionLabel => "action-label",
  List => "list",
  Item => "item",
  Empty => "empty",
  EmptyIcon => "empty-icon",
  EmptyText => "empty-text",
  Footer => "footer",
});

/// The states of a notification center a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NotificationCenterState {
  /// It has the row at its bottom.
  Footer,
  /// It lists nothing.
  Empty,
}

names!(NotificationCenterState {
  Footer => "footer",
  Empty => "empty",
});

/// What builds a notification center's panel: a theme's in place of the default one. Where the panel is shown,
/// and when, stays the center's.
pub type NotificationCenterRenderer = Arc<dyn Fn(NotificationCenterView) -> Element + Send + Sync>;

type Handler = Arc<dyn Fn() + Send + Sync>;

/// A notification center: the panel that lists an app's notifications, shown beside its bell while it is open.
///
/// ```
/// use lurq::{
///   core::{ElementRef, Signal},
///   ui::{Avatar, Badge, Button, ButtonVariant, Notification, NotificationCenter, icons},
/// };
///
/// // What the app keeps from one render to the next: the bell, and whether the panel is open.
/// let (bell, open) = (ElementRef::new(), Signal::new(false));
///
/// // The bell with its count opens and closes it; the list and what was read are the app's.
/// let center = NotificationCenter::new(open, bell)
///   .trigger(
///     Badge::count(2).label("unread notifications").on(
///       Button::new("Notifications")
///         .icon(icons::BELL)
///         .icon_only(true)
///         .variant(ButtonVariant::Ghost),
///     ),
///   )
///   .item(
///     Notification::new("Ivy Chen requested a review")
///       .description("feat/node-blur")
///       .media(Avatar::new("Ivy Chen"))
///       .time("2m")
///       .unread(true)
///       .on_click(|_| println!("open the review")),
///   )
///   .item(Notification::new("Mark published 0.60").description("Release notes are live").time("1d"))
///   .on_mark_all_read(|| println!("all read"))
///   .on_view_all(|| println!("the inbox"));
/// ```
///
/// It keeps nothing but what a popover keeps: whether it is open is the app's signal, which Escape, a press
/// outside it, its trigger and its footer turn. The list, what was read and what was dismissed are the app's: the
/// panel tells what a person did ([`on_mark_all_read`](Self::on_mark_all_read), [`on_clear`](Self::on_clear),
/// [`on_view_all`](Self::on_view_all), and each notification's own `on_click` and `on_close`) and the app gives it
/// the list as it is then. Every notification is drawn as a row ([`NotificationVariant::Row`]), whatever its own
/// variant. The list scrolls where it is higher than the panel lets it be, and the arrow keys move between its
/// rows. As it opens the focus goes into it, and back to the bell as it closes.
pub struct NotificationCenter {
  open: Signal<bool>,
  anchor: ElementRef,
  trigger: Option<Element>,
  title: Arc<str>,
  items: Vec<Notification>,
  empty: Option<Element>,
  empty_text: Arc<str>,
  mark_all_read_label: Arc<str>,
  clear_label: Arc<str>,
  view_all_label: Arc<str>,
  on_mark_all_read: Vec<Handler>,
  on_clear: Vec<Handler>,
  on_view_all: Vec<Handler>,
  on_dismiss: Vec<Handler>,
  placement: Placement,
  offset: SpacingValue,
  collision: CollisionStrategy,
  width: Option<Length>,
  max_height: Option<Length>,
  inline: bool,
  dismiss_on_escape: bool,
  dismiss_on_outside_press: bool,
  label: Option<Arc<str>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl NotificationCenter {
  /// A notification center shown beside `anchor`, the bell, while `open` is `true`: under it, its end at the
  /// bell's end.
  pub fn new(open: Signal<bool>, anchor: impl Into<ElementRef>) -> Self {
    Self {
      open,
      anchor: anchor.into(),
      trigger: None,
      title: super::current_locale().navigation.notifications.clone(),
      items: Vec::new(),
      empty: None,
      empty_text: super::current_locale().navigation.no_notifications.clone(),
      mark_all_read_label: super::current_locale().navigation.mark_all_read.clone(),
      clear_label: super::current_locale().navigation.clear_all.clone(),
      view_all_label: super::current_locale().navigation.view_all.clone(),
      on_mark_all_read: Vec::new(),
      on_clear: Vec::new(),
      on_view_all: Vec::new(),
      on_dismiss: Vec::new(),
      placement: Placement::BottomEnd,
      offset: SpacingSize::Xs.into(),
      collision: CollisionStrategy::FlipThenClamp,
      width: None,
      max_height: None,
      inline: false,
      dismiss_on_escape: true,
      dismiss_on_outside_press: true,
      label: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// What opens and closes it: the bell, a [`Button`](super::Button) or a [`Badge`](super::Badge) with its count
  /// on one. It is put before the panel, is its anchor, and a press of it turns the signal. It says to what reads
  /// the tree whether the panel is open.
  pub fn trigger(mut self, trigger: impl Into<Element>) -> Self {
    self.trigger = Some(trigger.into());
    self
  }

  /// What the panel is called, at its top and to tooling: "Notifications" by default.
  pub fn title(mut self, title: &str) -> Self {
    self.title = Arc::from(title);
    self
  }

  /// Adds a notification under those it lists. It is drawn as a row.
  pub fn item(mut self, item: Notification) -> Self {
    self.items.push(item);
    self
  }

  /// Adds several notifications.
  pub fn items(mut self, items: impl IntoIterator<Item = Notification>) -> Self {
    self.items.extend(items);
    self
  }

  /// What stands in place of the list while there is nothing in it, in place of the icon and the words it draws:
  /// an [`EmptyState`](super::EmptyState) of the app's.
  pub fn empty(mut self, empty: impl Into<Element>) -> Self {
    self.empty = Some(empty.into());
    self
  }

  /// What it says while there is nothing in it: "No notifications" by default.
  pub fn empty_text(mut self, text: &str) -> Self {
    self.empty_text = Arc::from(text);
    self
  }

  /// What the control that marks all read says: "Mark all read" by default.
  pub fn mark_all_read_label(mut self, label: &str) -> Self {
    self.mark_all_read_label = Arc::from(label);
    self
  }

  /// What the control that clears the list says: "Clear all" by default.
  pub fn clear_label(mut self, label: &str) -> Self {
    self.clear_label = Arc::from(label);
    self
  }

  /// What the row at the bottom says: "View all" by default.
  pub fn view_all_label(mut self, label: &str) -> Self {
    self.view_all_label = Arc::from(label);
    self
  }

  /// Called on a press of "Mark all read", which the header has once it is given one: the app marks its
  /// notifications read. The control takes no press while none of them is unread. A second handler is called
  /// after the first.
  pub fn on_mark_all_read(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_mark_all_read.push(Arc::new(handler));
    self
  }

  /// Called on a press of "Clear all", which the header has once it is given one: the app empties its list. The
  /// control takes no press while the list is empty. A second handler is called after the first.
  pub fn on_clear(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_clear.push(Arc::new(handler));
    self
  }

  /// Called on a press of the row at the bottom, "View all", which the panel has once it is given one, after the
  /// panel closed: the app goes to where all of them are. A second handler is called after the first.
  pub fn on_view_all(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_view_all.push(Arc::new(handler));
    self
  }

  /// What runs after a person closed the panel by a press outside it or Escape; not when the app sets the signal,
  /// and not on "View all". A second handler is called after the first.
  pub fn on_dismiss(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_dismiss.push(Arc::new(handler));
    self
  }

  /// Whether Escape closes the panel. `true` by default.
  pub fn dismiss_on_escape(mut self, dismiss: bool) -> Self {
    self.dismiss_on_escape = dismiss;
    self
  }

  /// Whether a press outside the panel and its bell closes it. `true` by default.
  pub fn dismiss_on_outside_press(mut self, dismiss: bool) -> Self {
    self.dismiss_on_outside_press = dismiss;
    self
  }

  /// Which side of the bell the panel is on, and which edge of it it lines up with: under it, its end at the
  /// bell's end (`Placement::BottomEnd`), by default.
  pub fn placement(mut self, placement: Placement) -> Self {
    self.placement = placement;
    self
  }

  /// How far the panel is from the bell, along the side it is on: the theme's smallest spacing by default.
  pub fn offset(mut self, offset: impl Into<SpacingValue>) -> Self {
    self.offset = offset.into();
    self
  }

  /// What the panel does where there is no room for it on its side: goes to the other side, then is moved into
  /// the window, by default.
  pub fn collision(mut self, collision: CollisionStrategy) -> Self {
    self.collision = collision;
    self
  }

  /// How wide the panel is, in place of what the theme gives it (360 in the design's).
  pub fn width(mut self, width: impl Into<Length>) -> Self {
    self.width = Some(width.into());
    self
  }

  /// How high the list is at most before it scrolls, in place of what the theme gives it (360 in the design's).
  pub fn max_height(mut self, height: impl Into<Length>) -> Self {
    self.max_height = Some(height.into());
    self
  }

  /// Draws the panel where it is declared, whether or not it is open, and not over the window beside its bell:
  /// for a panel that is part of a page, a gallery, a test. Its trigger is not drawn, and Escape and a press
  /// outside do not close it.
  pub fn inline(mut self, inline: bool) -> Self {
    self.inline = inline;
    self
  }

  /// What the panel is called to tooling that reads the tree, in place of its title.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Puts the notification center in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this notification center over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this notification center.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws its first notification that is pressed as it is under the pointer, held down or focused from the
  /// keyboard, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the panel. Its controls are `<id>-mark-all-read`, `<id>-clear` and `<id>-view-all`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the notification center in `theme`. Converting one into an element does this in the theme of what
  /// is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(NotificationCenterState::Footer, !self.on_view_all.is_empty())
    .state(NotificationCenterState::Empty, self.items.is_empty());
    // A width and a height given here are under what `style` and `rules` say of the panel and the list.
    let mut rules = Recipe::new();
    if let Some(width) = self.width {
      rules = rules.base(NotificationCenterPart::Panel, |s| s.width(width));
    }
    if let Some(height) = self.max_height {
      rules = rules.base(NotificationCenterPart::List, |s| s.max_height(height));
    }
    let rules = rules.extend(&self.rules);
    let all = |handlers: Vec<Handler>| -> Option<Handler> {
      (!handlers.is_empty()).then(|| {
        Arc::new(move || {
          for handler in &handlers {
            handler();
          }
        }) as Handler
      })
    };
    // "View all" leads away: the panel closes first, so that what the handler opens keeps the focus it takes.
    let view_all = all(self.on_view_all).map(|told| {
      let open = self.open.clone();
      let inline = self.inline;
      Arc::new(move || {
        if !inline {
          open.set(false);
        }
        told()
      }) as Handler
    });
    let view = NotificationCenterView {
      title: self.title,
      unread: self.items.iter().filter(|item| item.is_unread()).count(),
      items: self.items,
      empty: self.empty,
      empty_text: self.empty_text,
      mark_all_read_label: self.mark_all_read_label,
      clear_label: self.clear_label,
      view_all_label: self.view_all_label,
      recipe: theme.resolve::<NotificationCenter>(&case, Some(&rules)),
      mark_all_read: all(self.on_mark_all_read),
      clear: all(self.on_clear),
      view_all,
      label: self.label,
      forced: self.forced,
      id: self.id,
      key: self.key,
    };
    let panel = match theme.renderer::<NotificationCenter>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    };
    if self.inline {
      return panel;
    }
    // The offset is away from the bell: along the side the panel is on.
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
    let mut popup = Popup::new(self.anchor.clone(), panel)
      .open(self.open.clone())
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
    let Some(mut trigger) = self.trigger else {
      return popup.into();
    };
    // The trigger's own node is the anchor: a badge on a button is as large as the button. What is pressed is the
    // first thing in it that takes the focus, the button, so that Enter and Space on it open the panel too.
    let node = &mut *trigger.node;
    let (open, expanded) = (self.open.clone(), self.open.get());
    NodeUpdate::ref_element(node, self.anchor);
    let node = pressed(node);
    NodeUpdate::on_click(node, move |_: crate::app::events::MouseEvent| {
      open.update(|open| *open = !*open)
    });
    NodeUpdate::describe(node, Arc::from("a11y_haspopup"), Arc::from("dialog"));
    NodeUpdate::describe(
      node,
      Arc::from("a11y_expanded"),
      Arc::from(if expanded { "true" } else { "false" }),
    );
    Row::new().overflow_visible().child(trigger).child(popup).into()
  }
}

/// The first node in `node` that takes the focus, or `node` itself where none does: what a press of a trigger
/// is a press of.
fn pressed(node: &mut Node) -> &mut Node {
  fn path(node: &Node, to: &mut Vec<usize>) -> bool {
    if node.is_focusable() {
      return true;
    }
    for (index, child) in node.children.iter().enumerate() {
      to.push(index);
      if path(child, to) {
        return true;
      }
      to.pop();
    }
    false
  }
  let mut to = Vec::new();
  if !path(node, &mut to) {
    return node;
  }
  to.into_iter().fold(node, |node, index| &mut node.children[index])
}

impl From<NotificationCenter> for Element {
  fn from(center: NotificationCenter) -> Self {
    center.build(&current_theme())
  }
}

/// A notification center's panel as a renderer sees it: what it was given and the style of its parts.
pub struct NotificationCenterView {
  pub title: Arc<str>,
  /// How many of its notifications were not read.
  pub unread: usize,
  /// What it lists.
  pub items: Vec<Notification>,
  /// What the app put in place of the empty panel's icon and words.
  pub empty: Option<Element>,
  pub empty_text: Arc<str>,
  pub mark_all_read_label: Arc<str>,
  pub clear_label: Arc<str>,
  pub view_all_label: Arc<str>,
  /// The style of each part of this notification center.
  pub recipe: ResolvedRecipe,
  mark_all_read: Option<Handler>,
  clear: Option<Handler>,
  view_all: Option<Handler>,
  label: Option<Arc<str>>,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl NotificationCenterView {
  /// The style of a part, as this notification center is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// A control of the header: the theme's link in the style of the center's parts, which takes no press while
  /// there is nothing for it to do.
  fn action(&self, label: &str, name: &str, disabled: bool, handler: &Handler) -> Link {
    let handler = handler.clone();
    let link = Link::new(label)
      .disabled(disabled)
      .rules(&self.recipe.rules_for(NotificationCenterPart::Action, LinkPart::Root))
      .rules(
        &self
          .recipe
          .rules_for(NotificationCenterPart::ActionLabel, LinkPart::Label),
      )
      .on_click(move |_| handler());
    match &self.id {
      Some(id) => link.id(format!("{id}-{name}")),
      None => link,
    }
  }

  /// The row at the top: the title, and at its end what marks all read and what clears, where the center was
  /// given handlers for them.
  pub fn header(&self) -> Row {
    let title = self.part(NotificationCenterPart::Title);
    let mut header = Row::new()
      .width(Dimension::Pct(100.0))
      .flex_shrink(0.0)
      .align_items(Alignment::Center)
      .overflow_visible()
      .part(&self.part(NotificationCenterPart::Header))
      .child(
        Column::new()
          .flex(1.0)
          .min_width(Dimension::Px(0.0))
          .part(&title)
          .child(super::shared::one_line(&title, &self.title)),
      );
    let mut actions = Row::new()
      .flex_shrink(0.0)
      .align_items(Alignment::Center)
      .part(&self.part(NotificationCenterPart::Actions));
    if let Some(handler) = &self.mark_all_read {
      actions = actions.child(self.action(&self.mark_all_read_label, "mark-all-read", self.unread == 0, handler));
    }
    if let Some(handler) = &self.clear {
      actions = actions.child(self.action(&self.clear_label, "clear", self.items.is_empty(), handler));
    }
    if self.mark_all_read.is_some() || self.clear.is_some() {
      header = header.child(actions);
    }
    header
  }

  /// What stands in place of the list while there is nothing in it: what the app gave, or an inbox over the
  /// words.
  pub fn empty(&mut self) -> Element {
    let held = Column::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .part(&self.part(NotificationCenterPart::Empty))
      .describe("a11y_role", "status");
    match self.empty.take() {
      Some(empty) => held.child(empty).into(),
      None => {
        let text = self.part(NotificationCenterPart::EmptyText);
        held
          .child(glyph(
            &self.part(NotificationCenterPart::EmptyIcon),
            Name::from(icons::INBOX),
            false,
          ))
          .child(
            Column::new()
              .width(Dimension::Pct(100.0))
              .align_items(Alignment::Center)
              .part(&text)
              .child(text.text(&self.empty_text)),
          )
          .into()
      }
    }
  }

  /// The notifications as rows, one under another with a line between two, in what scrolls: the arrow keys move
  /// between the rows that are pressed, and Tab stops in them once.
  pub fn list(&mut self) -> Element {
    let items = std::mem::take(&mut self.items);
    let mut rows = Column::new()
      .width(Dimension::Pct(100.0))
      .focus_group(FocusGroup::vertical().wrap(false).one_stop(true))
      .describe("a11y_role", "list")
      .describe("a11y_name", self.label.clone().unwrap_or_else(|| self.title.clone()));
    let mut forced = self.forced;
    let last = items.len().saturating_sub(1);
    for (index, item) in items.into_iter().enumerate() {
      let row = item
        .variant(NotificationVariant::Row)
        .force(forced.take())
        .build(self.theme());
      // The line is drawn in the row's own box, over its bottom edge, and takes no room.
      let mut held = Column::new().width(Dimension::Pct(100.0));
      if index < last {
        held = held.part(&self.part(NotificationCenterPart::Item));
      }
      rows = rows.child(held.child(row));
    }
    ScrollVertical::new(rows)
      .width(Dimension::Pct(100.0))
      // As high as what is in it up to the most, and gives way when the window has less.
      .flex_full(0.0, 1.0, None)
      .part(&self.part(NotificationCenterPart::List))
      .into()
  }

  /// The row at the bottom that leads to all of them. `None` where the center was given no handler for it.
  pub fn footer(&self) -> Option<Element> {
    let handler = self.view_all.clone()?;
    let part = self.part(NotificationCenterPart::Footer);
    let mut footer = pressable(&part, false)
      .width(Dimension::Pct(100.0))
      .flex_shrink(0.0)
      .align_items(Alignment::Center)
      .describe("a11y_role", "button")
      .describe("a11y_name", self.view_all_label.clone())
      .child(super::shared::one_line(&part, &self.view_all_label))
      .on_click(move |_| handler());
    if let Some(id) = &self.id {
      footer = footer.id(format!("{id}-view-all"));
    }
    Some(footer.into())
  }

  /// The panel with nothing in it yet: a dialog to what reads the tree, called by its label or its title.
  pub fn panel(&self) -> Column {
    let mut panel = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(NotificationCenterPart::Panel))
      .describe("a11y_role", "dialog")
      .describe("a11y_name", self.label.clone().unwrap_or_else(|| self.title.clone()));
    if let Some(id) = &self.id {
      panel = panel.id(id.clone());
    }
    if let Some(key) = &self.key {
      panel = panel.key(key.clone());
    }
    panel
  }

  /// How the default renderer builds a notification center: the header over the list, or what says it is empty,
  /// over the footer.
  pub fn render(mut self) -> Element {
    let panel = self.panel().child(self.header());
    let panel = if self.items.is_empty() {
      panel.child(self.empty())
    } else {
      panel.child(self.list())
    };
    panel.with_children(self.footer()).into()
  }
}

impl Themed for NotificationCenter {
  const NAME: &'static str = "notification_center";
  type Renderer = NotificationCenterRenderer;

  /// The design's notification center (`pencil/lurq-design-system.pen`, Notification Center `Kg2Z4`): a panel 360
  /// wide in the popover's colors, with an edge of 1 in the border's color, the extra large radius and the large
  /// shadow. Its header is 48 high, 16 from its sides, with a line of the border's color under it: the title at
  /// 15, semibold, and "Mark all read" at 13, medium, in the info color. Its notifications are rows with a line of
  /// 1 in the border's color along the bottom of each that has another under it (the row is [`Notification`]'s),
  /// and along the bottom of the list over the footer. Its footer is 40 high with "View all" at 13, medium, in
  /// its middle. The board draws three rows and no end of the list: the list
  /// scrolls past 360. It draws no empty panel: an inbox of 24 over the words at 13, both muted, 32 over and
  /// under them. It draws nothing under the pointer in the header and the footer: the header's controls are
  /// underlined as a link is, the footer is in the highlight, and both have the ring from the keyboard, the
  /// footer's inside it. It fades in and out as the popover does.
  fn recipe() -> Recipe {
    use NotificationCenterPart as P;
    use PaletteColor::{
      Border as Edge, Foreground, Highlight, Info, MutedForeground, Popover, PopoverForeground, Ring,
    };
    let line = || Border::inside(BorderSize::Sm, Edge);
    Recipe::new()
      .base(P::Panel, |s| {
        s.width(360.0)
          .max_width(Length::Fill)
          .radius(RadiusSize::Xl)
          .background(Popover)
          .foreground(PopoverForeground)
          .border(line())
          .shadow(ShadowStyle::Lg)
          .enter(PartAnimation::new(FADE_IN, MotionDuration::Base, MotionEasing::Enter))
          .exit(PartAnimation::new(FADE_OUT, MotionDuration::Fast, MotionEasing::Exit))
      })
      .base(P::Header, |s| s.height(48.0).padding_x(16.0).gap(12.0).underline(line()))
      .base(P::Title, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(15.0)
          .weight(FontWeight::SemiBold)
          .foreground(Foreground)
      })
      .base(P::Actions, |s| s.gap(12.0))
      .base(P::Action, |s| s.foreground(Info))
      .base(P::ActionLabel, |s| s.font_size(13.0).weight(FontWeight::Medium))
      .within(P::ActionLabel, |s| s.underline(Border::inside(1.0, Info)))
      // Without a footer the list is the panel's bottom, and is round where the panel is; with one, the line over
      // the footer is the list's, which stays as the rows scroll under it.
      .base(P::List, |s| {
        s.max_height(360.0).corners(0.0, 0.0, RadiusSize::Xl, RadiusSize::Xl)
      })
      .state(NotificationCenterState::Footer, P::List, |s| s.radius(0.0).underline(line()))
      .base(P::Item, |s| s.underline(line()))
      .base(P::Empty, |s| s.padding_x(16.0).padding_y(32.0).gap(8.0))
      .state(NotificationCenterState::Footer, P::Empty, |s| s.underline(line()))
      .base(P::EmptyIcon, |s| s.square(24.0).foreground(MutedForeground))
      .base(P::EmptyText, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .foreground(MutedForeground)
      })
      .base(P::Footer, |s| {
        fading(
          s.height(40.0)
            .padding_x(16.0)
            .typography(TypographyStyle::Body)
            .font_size(13.0)
            .weight(FontWeight::Medium)
            .foreground(Foreground)
            .corners(0.0, 0.0, RadiusSize::Xl, RadiusSize::Xl)
            .cursor(CursorIcon::Pointer),
        )
      })
      .hovered(P::Footer, |s| s.background(Highlight))
      .focus_visible(P::Footer, |s| s.outline(Outline::new(BorderSize::Md, Ring).offset(-2.0)))
  }
}
