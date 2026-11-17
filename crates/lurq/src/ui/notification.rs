//! A notification: something that happened and stays where the app lists it until a person has dealt with it.
//! A card on its own, and a row of a [`NotificationCenter`](super::NotificationCenter).

use std::sync::Arc;

use super::{
  Button, ButtonPart, ButtonSize, ButtonVariant,
  alert::close_style,
  icon::Icon,
  icons, names,
  part_style::{FontFamily, Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{IconChoice, fading, glyph, pressable, shown_icon},
  theme::{Themed, current_breakpoint, current_theme},
};
use crate::{
  app::{
    events::MouseEvent,
    theme::{BorderSize, IconSize, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle},
  },
  components::{Column, Row},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{
    CursorIcon, Element, EventHandler, FocusGroup, IntoMouseEventHandler, border::Border, dimension::Dimension,
    outline::Outline,
  },
};

/// The parts of a notification a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NotificationPart {
  /// The box: its fill, its edge and the room inside it as a card, its paddings as a row.
  Root,
  /// The dot of one that was not read.
  Unread,
  /// What stands before the text: who or what it is from.
  Media,
  /// The icon before the text, where it is given no media.
  Icon,
  /// The text over the body and the actions.
  Content,
  /// The header over the description.
  Text,
  /// The title, and on a card the time and the dot after it.
  Header,
  /// What happened, in a few words.
  Title,
  /// When, as the app words it: "2m".
  Time,
  /// More of it, under the title.
  Description,
  /// What the app puts under the description.
  Body,
  /// The buttons under the text.
  Actions,
  /// A button of the actions: what it gives the theme's button.
  Action,
  /// What takes a row away, at its end.
  Close,
}

names!(NotificationPart {
  Root => "root",
  Unread => "unread",
  Media => "media",
  Icon => "icon",
  Content => "content",
  Text => "text",
  Header => "header",
  Title => "title",
  Time => "time",
  Description => "description",
  Body => "body",
  Actions => "actions",
  Action => "action",
  Close => "close",
});

/// How a notification is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NotificationVariant {
  /// A card of its own, with an edge and a shadow: the design's Notification.
  #[default]
  Card,
  /// A row of a list of them, as a [`NotificationCenter`](super::NotificationCenter) lists its own.
  Row,
}

names!(NotificationVariant {
  Card => "card",
  Row => "row",
});

/// The states of a notification a recipe styles, besides what the pointer and the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NotificationState {
  /// It was not read.
  Unread,
  /// It is pressed as a whole: it opens what it is about.
  Pressable,
}

names!(NotificationState {
  Unread => "unread",
  Pressable => "pressable",
});

/// What builds a notification's nodes: a theme's in place of the default one.
pub type NotificationRenderer = Arc<dyn Fn(NotificationView) -> Element + Send + Sync>;

type Handler = Arc<dyn Fn() + Send + Sync>;

/// What stands in a notification's actions.
enum Act {
  Button(Arc<str>, Handler),
  Element(Element),
}

/// A notification: something that happened which a person may act on now or later. Who or what it is from, a
/// title and a text, when, what can be done about it, whether it was read.
///
/// ```
/// use lurq::ui::{Avatar, Notification, NotificationVariant};
///
/// // The design's card: a bell, a title, a line, a button and what dismisses it.
/// let review = Notification::new("Review requested")
///   .description("Ivy asked you to review feat/node-blur.")
///   .button("Review", || println!("review"))
///   .on_close(|| println!("dismissed"));
///
/// // A row of a list: who it is from, when, and that it was not read.
/// let row = Notification::new("Ivy Chen requested a review")
///   .description("feat/node-blur")
///   .media(Avatar::new("Ivy Chen"))
///   .time("2m")
///   .unread(true)
///   .variant(NotificationVariant::Row)
///   .on_click(|_| println!("opened"));
/// ```
///
/// A notification stays: it leaves when the app takes it out of what it lists, as a toast leaves by itself and
/// an alert is part of its page. It keeps nothing: whether it was read and whether it is there are the app's, and
/// it tells what a person did. A press of it, where it is given [`on_click`](Self::on_click), opens what it is
/// about; its buttons do what their handlers say; given [`on_close`](Self::on_close) it has what dismisses it, a
/// "Dismiss" button after a card's own and a close control at the end of a row.
pub struct Notification {
  title: Arc<str>,
  description: Option<Arc<str>>,
  media: Option<Element>,
  icon: Option<IconChoice>,
  time: Option<Arc<str>>,
  unread: bool,
  actions: Vec<Act>,
  body: Vec<Element>,
  variant: Name,
  label: Option<Arc<str>>,
  close_label: Arc<str>,
  unread_label: Arc<str>,
  on_click: Vec<EventHandler<MouseEvent>>,
  on_close: Vec<Handler>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Notification {
  /// A notification that says `title`: a card with the bell before it.
  pub fn new(title: &str) -> Self {
    Self {
      title: Arc::from(title),
      description: None,
      media: None,
      icon: None,
      time: None,
      unread: false,
      actions: Vec::new(),
      body: Vec::new(),
      variant: NotificationVariant::Card.into(),
      label: None,
      close_label: super::current_locale().common.dismiss.clone(),
      unread_label: super::current_locale().navigation.unread.clone(),
      on_click: Vec::new(),
      on_close: Vec::new(),
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// More of it, under the title.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// Who or what it is from, before the text in place of its icon: an [`Avatar`](super::Avatar), an app's mark.
  pub fn media(mut self, media: impl Into<Element>) -> Self {
    self.media = Some(media.into());
    self
  }

  /// An icon of the theme's set in place of the bell, or none: `icon(None)`.
  pub fn icon(mut self, icon: impl Into<IconChoice>) -> Self {
    self.icon = Some(icon.into());
    self
  }

  /// When it happened, as the app words it: "2m", "Yesterday".
  pub fn time(mut self, time: &str) -> Self {
    self.time = Some(Arc::from(time));
    self
  }

  /// Whether it was not read: it has the dot, and a row's title is heavier.
  pub fn unread(mut self, unread: bool) -> Self {
    self.unread = unread;
    self
  }

  /// Adds a button under the text, and what a press of it does: the first is the primary one, the others ghosts.
  pub fn button(mut self, label: &str, on_click: impl Fn() + Send + Sync + 'static) -> Self {
    self.actions.push(Act::Button(Arc::from(label), Arc::new(on_click)));
    self
  }

  /// Adds something of the app's own to the actions, after what is there: a link, a menu's button. A notification
  /// has several actions, so a second call adds and does not replace.
  pub fn action(mut self, action: impl Into<Element>) -> Self {
    self.actions.push(Act::Element(action.into()));
    self
  }

  /// Adds an element under the description, for more than text: a picture, a quote, a progress.
  pub fn child(mut self, child: impl Into<Element>) -> Self {
    self.body.push(child.into());
    self
  }

  /// Adds elements under the description.
  pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.body.extend(children.into_iter().map(Into::into));
    self
  }

  /// How it is drawn: a [`NotificationVariant`], or a name the theme's recipe gives.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// What it is called to tooling that reads the tree, in place of its title.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// What dismisses it says and is called: "Dismiss" by default.
  pub fn close_label(mut self, label: &str) -> Self {
    self.close_label = Arc::from(label);
    self
  }

  /// What its dot is called to tooling that reads the tree: "Unread" by default.
  pub fn unread_label(mut self, label: &str) -> Self {
    self.unread_label = Arc::from(label);
    self
  }

  /// What a press of the notification does, given the press: it opens what it is about. The notification is
  /// pressed as a whole, a stop of Tab, and a button to what reads the tree; its own buttons are pressed apart
  /// from it. A second handler is called after the first.
  pub fn on_click(mut self, handler: impl IntoMouseEventHandler) -> Self {
    self.on_click.push(handler.into_event_handler());
    self
  }

  /// Called on a press of what dismisses it, which it has once it is given one. The app takes the notification
  /// out of what it lists. A second handler is called after the first.
  pub fn on_close(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_close.push(Arc::new(handler));
    self
  }

  /// Puts the notification in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this notification over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this notification: what the list it is in says of it.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the notification, where it is pressed, and else what dismisses a row, as it is under the pointer, held
  /// down or focused from the keyboard, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the box. What dismisses it is `<id>-close`, and its buttons `<id>-action-0`, `<id>-action-1`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Whether it was not read.
  pub(crate) fn is_unread(&self) -> bool {
    self.unread
  }

  /// Builds the notification in `theme`. Converting a notification into an element does this in the theme of
  /// what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let pressable = !self.on_click.is_empty();
    let case = Case {
      variant: Some(self.variant),
      size: None,
      density: None,
      breakpoint: current_breakpoint(),
      states: self.states.clone(),
    }
    .state(NotificationState::Unread, self.unread)
    .state(NotificationState::Pressable, pressable);
    let view = NotificationView {
      title: self.title,
      description: self.description,
      icon: match self.media {
        Some(_) => None,
        None => shown_icon(self.icon, icons::BELL),
      },
      time: self.time,
      unread: self.unread,
      row: self.variant == Name::from(NotificationVariant::Row),
      variant: self.variant,
      label: self.label,
      close_label: self.close_label,
      unread_label: self.unread_label,
      close_button: !self.on_close.is_empty(),
      recipe: theme.resolve::<Notification>(&case, Some(&self.rules)),
      media: self.media,
      actions: self.actions,
      body: self.body,
      forced: self.forced,
      on_click: self.on_click,
      on_close: self.on_close,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Notification>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Notification> for Element {
  fn from(notification: Notification) -> Self {
    notification.build(&current_theme())
  }
}

/// A notification as a renderer sees it: what it was given, the style of its parts, and its behavior ready to be
/// dressed.
pub struct NotificationView {
  pub title: Arc<str>,
  pub description: Option<Arc<str>>,
  /// The icon before the text, by its name in the theme's set: none where it has media or was given none.
  pub icon: Option<Name>,
  pub time: Option<Arc<str>>,
  pub unread: bool,
  /// Whether it is a row of a list and not a card.
  pub row: bool,
  pub variant: Name,
  /// What it is called to tooling, where that is not its title.
  pub label: Option<Arc<str>>,
  /// What dismisses it says and is called.
  pub close_label: Arc<str>,
  /// What its dot is called to tooling.
  pub unread_label: Arc<str>,
  /// Whether it has what dismisses it: it was given a handler for it.
  pub close_button: bool,
  /// The style of each part of this notification.
  pub recipe: ResolvedRecipe,
  media: Option<Element>,
  actions: Vec<Act>,
  body: Vec<Element>,
  forced: Option<Interaction>,
  on_click: Vec<EventHandler<MouseEvent>>,
  on_close: Vec<Handler>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl NotificationView {
  /// The style of a part, as this notification is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// Whether it is pressed as a whole: it has a handler.
  pub fn pressable(&self) -> bool {
    !self.on_click.is_empty()
  }

  /// A text in the style of `part`, as wide as the room it has: a longer one goes on to the next line.
  fn wrapping(&self, part: NotificationPart, content: &str) -> Column {
    let part = self.part(part);
    let fill = Dimension::Pct(100.0);
    Column::new()
      .width(fill)
      .min_width(Dimension::Px(0.0))
      .part(&part)
      .child(part.text(content).width(fill))
  }

  /// The dot of one that was not read. A row keeps its room when it was read, so that what stands after it lines
  /// up down a list; `None` for a card that was read.
  pub fn unread_dot(&self) -> Option<Element> {
    if !self.unread && !self.row {
      return None;
    }
    let part = self.part(NotificationPart::Unread);
    let mut dot = Row::new().flex_shrink(0.0).part(&part);
    if self.unread {
      dot = dot
        .describe("a11y_role", "status")
        .describe("a11y_name", self.unread_label.clone());
    }
    Some(dot.into())
  }

  /// What stands before the text: the media it was given, or its icon. `None` with neither.
  pub fn media(&mut self) -> Option<Element> {
    let shown: Element = match self.media.take() {
      Some(media) => media,
      None => glyph(&self.part(NotificationPart::Icon), self.icon?, false),
    };
    Some(
      Row::new()
        .flex_shrink(0.0)
        .justify(Justify::Center)
        .align_items(Alignment::Center)
        .overflow_visible()
        .part(&self.part(NotificationPart::Media))
        .child(shown)
        .into(),
    )
  }

  /// When it happened, on one line.
  pub fn time(&self) -> Option<Element> {
    let time = self.time.as_ref()?;
    let part = self.part(NotificationPart::Time);
    Some(
      Row::new()
        .flex_shrink(0.0)
        .part(&part)
        .child(part.text(time).nowrap())
        .into(),
    )
  }

  /// The title over the description: on a card the time and the dot stand after the title.
  pub fn text(&self) -> Column {
    let mut header = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .part(&self.part(NotificationPart::Header))
      .child(self.wrapping(NotificationPart::Title, &self.title).flex(1.0));
    if !self.row {
      header = header.with_children(self.time()).with_children(self.unread_dot());
    }
    let mut text = Column::new()
      .width(Dimension::Pct(100.0))
      .part(&self.part(NotificationPart::Text))
      .child(header);
    if let Some(description) = &self.description {
      text = text.child(self.wrapping(NotificationPart::Description, description));
    }
    text
  }

  /// A button of the actions in the style of [`NotificationPart::Action`] over the theme's button.
  fn button(&self, label: &str, variant: ButtonVariant, id: Option<String>, handler: Handler) -> Button {
    let button = Button::new(label)
      .variant(variant)
      .size(ButtonSize::Sm)
      .rules(&self.recipe.rules_for(NotificationPart::Action, ButtonPart::Root))
      .on_click(move |_| handler());
    match id {
      Some(id) => button.id(id),
      None => button,
    }
  }

  /// What every handler of `on_close` is told by.
  fn closer(&self) -> Handler {
    let handlers = self.on_close.clone();
    Arc::new(move || {
      for handler in &handlers {
        handler();
      }
    })
  }

  /// The buttons under the text: its own, the first the primary one, and on a card what dismisses it after them.
  /// `None` with none.
  pub fn actions(&mut self) -> Option<Row> {
    let dismiss = self.close_button && !self.row;
    if self.actions.is_empty() && !dismiss {
      return None;
    }
    let mut row = Row::new()
      .width(Dimension::Pct(100.0))
      .wrap()
      .align_items(Alignment::Center)
      .part(&self.part(NotificationPart::Actions));
    let mut buttons = 0;
    for (index, act) in std::mem::take(&mut self.actions).into_iter().enumerate() {
      row = match act {
        Act::Button(label, handler) => {
          let variant = if buttons == 0 {
            ButtonVariant::Primary
          } else {
            ButtonVariant::Ghost
          };
          buttons += 1;
          let id = self.id.as_ref().map(|id| format!("{id}-action-{index}"));
          row.child(self.button(&label, variant, id, handler))
        }
        Act::Element(element) => row.child(element),
      };
    }
    if dismiss {
      let id = self.id.as_ref().map(|id| format!("{id}-close"));
      row = row.child(self.button(&self.close_label, ButtonVariant::Ghost, id, self.closer()));
    }
    // A press of a button is the button's alone, where the notification is pressed too.
    Some(self.own_presses(row))
  }

  /// What takes a row away, at its end: its icon alone, called by its label.
  pub fn close(&self) -> Element {
    let part = self.part(NotificationPart::Close).forced(match self.pressable() {
      true => None,
      false => self.forced,
    });
    let side = part.rest().width.unwrap_or(Length::Icon(IconSize::Md));
    let close = self.closer();
    let mut control = pressable(&part, false)
      .flex_shrink(0.0)
      .describe("a11y_role", "button")
      .describe("a11y_name", self.close_label.clone())
      .child(Icon::new(icons::CLOSE).size(side).build(part.theme()))
      .on_click(move |event: MouseEvent| {
        event.stop_propagation();
        close()
      });
    if let Some(id) = &self.id {
      control = control.id(format!("{id}-close"));
    }
    // The arrows of a list move between its rows, not to what is in them.
    Row::new()
      .flex_shrink(0.0)
      .focus_group(FocusGroup::none())
      .child(control)
      .into()
  }

  /// A row that keeps a press of what is in it from the notification.
  fn own_presses(&self, row: Row) -> Row {
    if self.pressable() {
      row.on_click(|event: MouseEvent| event.stop_propagation())
    } else {
      row
    }
  }

  /// The text over the body and the actions, as wide as the room what stands beside it leaves.
  pub fn content(&mut self) -> Column {
    let mut content = Column::new()
      .flex(1.0)
      .min_width(Dimension::Px(0.0))
      .part(&self.part(NotificationPart::Content))
      .child(self.text());
    if !self.body.is_empty() {
      content = content.child(
        Column::new()
          .width(Dimension::Pct(100.0))
          .part(&self.part(NotificationPart::Body))
          .with_children(std::mem::take(&mut self.body)),
      );
    }
    content.with_children(self.actions())
  }

  /// The box in the style of its root, holding `children`: a button that is pressed and focused where the
  /// notification is pressed, and an article that is neither where it is not. It is called by its label or its
  /// title, and described by its description.
  pub fn root(&self, children: Vec<Element>) -> Element {
    let name = self.label.clone().unwrap_or_else(|| self.title.clone());
    let align = if self.row { Alignment::Center } else { Alignment::Start };
    let description = self.description.clone();
    let mut root: Element = if self.pressable() {
      let part = self.part(NotificationPart::Root).forced(self.forced);
      let mut root = pressable(&part, false)
        .justify(Justify::Start)
        .align_items(align)
        .interaction_group(true)
        // Its buttons and what dismisses it are stops of Tab of their own, after it.
        .stops_inside(true)
        .focus_label(name.clone())
        .describe("a11y_role", "button")
        .describe("a11y_name", name);
      if let Some(description) = description {
        root = root.describe("a11y_description", description);
      }
      for handler in &self.on_click {
        root = root.on_click(handler.clone());
      }
      root.with_children(children).into()
    } else {
      let mut root = Row::new()
        .align_items(align)
        .part(&self.part(NotificationPart::Root))
        .describe("a11y_role", "article")
        .describe("a11y_name", name);
      if let Some(description) = description {
        root = root.describe("a11y_description", description);
      }
      root.with_children(children).into()
    };
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// How the default renderer builds a notification: the dot of a row, what it is from, its content, and at the
  /// end of a row its time and what dismisses it.
  pub fn render(mut self) -> Element {
    let mut children: Vec<Element> = Vec::new();
    if self.row {
      children.extend(self.unread_dot());
    }
    children.extend(self.media());
    children.push(self.content().into());
    if self.row {
      children.extend(self.time());
      if self.close_button {
        children.push(self.close());
      }
    }
    self.root(children)
  }
}

impl crate::ui::Styled for Notification {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for Notification {
  const NAME: &'static str = "notification";
  type Renderer = NotificationRenderer;

  fn parts() -> crate::ui::Declared {
    NotificationPart::declared()
  }

  fn states() -> crate::ui::Declared {
    NotificationState::declared()
  }

  /// The design's notification (`pencil/lurq-design-system.pen`, Notification `sd57u`, and the items of the
  /// Notification Center `Kg2Z4`). The card is 340 wide in the popover's colors, with an edge of 1 in the border's
  /// color, the large radius and the medium shadow, padded 16: the bell of 16, 12 from the title (14, medium) over
  /// the description (13, muted, lines of 1.4) 4 under it, and 10 under them the buttons, 28 high, 10 from their
  /// sides and 8 apart, the first the primary one and what dismisses it a ghost. The row is as wide as its list,
  /// 12 from its top, its bottom and its start and 16 from its end, with 10 between the dot of 7 in the info
  /// color, the avatar, the text and the time: the title 13, medium while it was not read, over the description
  /// of 12 in the muted color 1 under it, and the time in mono 11 in the muted color. The board draws the row
  /// under the pointer in the highlight (`$accent`). It draws no time and no dot on a card: they stand after its
  /// title, the time as a row's is. It draws no card pressed and nothing focused: a pressed card has the veils of
  /// those states, and both have the ring inside them. A row is as high as the theme's density makes a row of 56.
  fn recipe() -> Recipe {
    use NotificationPart as P;
    use NotificationState::{Pressable, Unread};
    use NotificationVariant::Row as Listed;
    use PaletteColor::{
      Foreground, Highlight, Info, MutedForeground, Popover, PopoverForeground, Ring, StateHover, StatePressed,
    };
    let listed = || Selector::new().variant(Listed);
    let recipe = Recipe::new()
      .base(P::Root, |s| fading(s.foreground(PopoverForeground).gap(12.0)))
      .variant(NotificationVariant::Card, P::Root, |s| {
        s.width(340.0)
          .max_width(Length::Fill)
          .background(Popover)
          .border(Border::inside(BorderSize::Sm, PaletteColor::Border))
          .radius(RadiusSize::Lg)
          .shadow(ShadowStyle::Md)
          .padding_x(16.0)
          .padding_y(16.0)
      })
      .state(Pressable, P::Root, |s| s.cursor(CursorIcon::Pointer))
      .when(Selector::new().state(Pressable).hovered(), P::Root, |s| s.veil(StateHover))
      .when(Selector::new().state(Pressable).pressed(), P::Root, |s| s.veil(StatePressed))
      // The ring is inside the box: rows touch in a list, and the list clips.
      .focus_visible(P::Root, |s| s.outline(Outline::new(BorderSize::Md, Ring).offset(-2.0)))
      .base(P::Unread, |s| s.square(7.0).radius(RadiusSize::Full))
      .state(Unread, P::Unread, |s| s.background(Info))
      .base(P::Icon, |s| s.square(Length::Icon(IconSize::Md)).foreground(Foreground))
      .base(P::Content, |s| s.gap(10.0))
      .base(P::Text, |s| s.gap(4.0))
      .base(P::Header, |s| s.gap(8.0))
      .base(P::Title, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(14.0)
          .line_height(1.2)
          .weight(FontWeight::Medium)
          .foreground(Foreground)
      })
      .base(P::Time, |s| {
        s.font_family(FontFamily::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      .base(P::Description, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .line_height(1.4)
          .foreground(MutedForeground)
      })
      .base(P::Body, |s| s.gap(8.0))
      .base(P::Actions, |s| s.gap(8.0))
      .base(P::Action, |s| s.height(28.0).padding_x(10.0))
      // A row of a list: no box of its own, and the board's fill under the pointer in place of the veil.
      .variant(Listed, P::Root, |s| {
        s.width(Length::Fill)
          .min_height(56.0)
          .padding_start(12.0)
          .padding_end(16.0)
          .padding_y(12.0)
          .gap(10.0)
      })
      .by_density(listed(), P::Root, |s, step| {
        s.min_height(56.0 + step).padding_y(12.0 + step / 2.0)
      })
      .when(listed().state(Pressable).hovered(), P::Root, |s| s.background(Highlight))
      .variant(Listed, P::Text, |s| s.gap(1.0))
      .variant(Listed, P::Title, |s| s.font_size(13.0).weight(FontWeight::Normal))
      .when(listed().state(Unread), P::Title, |s| s.weight(FontWeight::Medium))
      // Lines of 1.2, so that the two of a row are no higher than the avatar of 32 beside them.
      .variant(Listed, P::Description, |s| s.font_size(12.0).line_height(1.2));
    close_style(recipe, P::Close)
  }
}
