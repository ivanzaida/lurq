//! An alert: a message in the flow of a page about what is on it, and what
//! an alert, a banner and a callout share: their variants, the icon and the
//! colors of each, and the close control.

use std::sync::Arc;

use super::{
  icon::Icon,
  icons, names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{fading, glyph, pressable},
  theme::{Themed, current_breakpoint, current_theme},
};
use crate::{
  app::theme::{BorderSize, IconSize, PaletteColor, RadiusSize, Theme, TypographyStyle},
  components::{self, Column, Row},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, border::Border, dimension::Dimension},
};

/// What a message says it is, for an alert, a banner and a callout alike: its
/// icon, and the colors of its fill, its edge and its icon.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AlertVariant {
  /// Something to know, with no weight of its own: "Update available".
  #[default]
  Neutral,
  /// Something to know that is news: "New shortcut".
  Info,
  /// Something went well: "Backup complete".
  Success,
  /// Something to look at before it is a problem: "Low disk space".
  Warning,
  /// Something went wrong: "Could not save".
  Destructive,
}

names!(AlertVariant {
  Neutral => "neutral",
  Info => "info",
  Success => "success",
  Warning => "warning",
  Destructive => "destructive",
});

impl AlertVariant {
  /// The variant called `name`, where it is one of these: a variant a theme adds is none of them.
  pub fn named(name: Name) -> Option<Self> {
    Self::all().iter().copied().find(|variant| Name::from(*variant) == name)
  }

  /// The icon of the variant in the theme's set, as the board draws it on an
  /// alert: the information sign for a neutral and an informative one, the
  /// check in a circle for a success, the warning sign for a warning and for
  /// what went wrong.
  pub fn icon(self) -> &'static str {
    match self {
      Self::Neutral | Self::Info => icons::INFO,
      Self::Success => icons::CHECK_CIRCLE,
      Self::Warning | Self::Destructive => icons::WARNING,
    }
  }

  /// The color of the variant: its icon, its edge, its bar, its link. A
  /// neutral message has the foreground's.
  pub fn accent(self) -> PaletteColor {
    match self {
      Self::Neutral => PaletteColor::Foreground,
      Self::Info => PaletteColor::Info,
      Self::Success => PaletteColor::Success,
      Self::Warning => PaletteColor::Warning,
      Self::Destructive => PaletteColor::Destructive,
    }
  }

  /// The fill of the variant: the muted form of its color. A neutral message
  /// has the muted fill.
  pub fn tint(self) -> PaletteColor {
    match self {
      Self::Neutral => PaletteColor::Muted,
      Self::Info => PaletteColor::InfoMuted,
      Self::Success => PaletteColor::SuccessMuted,
      Self::Warning => PaletteColor::WarningMuted,
      Self::Destructive => PaletteColor::DestructiveMuted,
    }
  }

  /// What a message of the variant is to tooling that reads the tree: an
  /// `alert`, which a screen reader says at once, for a warning and for what
  /// went wrong; a `status`, said when the reader is done, for the others.
  pub fn role(self) -> &'static str {
    match self {
      Self::Warning | Self::Destructive => "alert",
      _ => "status",
    }
  }
}

pub(crate) use super::shared::{IconChoice, shown_icon};

/// A text of a message in the style of `part`, as wide as the room it has: a
/// longer one goes on to the next line. A row would lay it out as wide as it
/// is and cut it.
pub(crate) fn wrapping(part: &ResolvedPart, content: &str) -> Column {
  let fill = Dimension::Pct(100.0);
  Column::new()
    .width(fill)
    .part(part)
    .child(part.text(content).width(fill))
}

/// The close control of a message, in the style of `part`: a press of it, or
/// Enter or Space while it has the focus, tells every handler. It is called
/// "Close" to what reads the tree.
pub(crate) fn close_control(part: &ResolvedPart, handlers: &[Arc<dyn Fn() + Send + Sync>]) -> components::Button {
  let side = part.rest().width.unwrap_or(Length::Icon(IconSize::Md));
  let mut close = pressable(part, false)
    .describe("a11y_role", "button")
    .describe("a11y_name", super::current_locale().common.close.clone())
    .child(Icon::new(icons::CLOSE).size(side).build(part.theme()));
  if !handlers.is_empty() {
    let handlers = handlers.to_vec();
    close = close.on_click(move |_| {
      for handler in &handlers {
        handler();
      }
    });
  }
  close
}

/// The style of a close control of a message: an icon of 16 in the muted
/// color, in the foreground's under the pointer, with the ring from the
/// keyboard.
pub(crate) fn close_style(recipe: Recipe, part: impl Into<Name>) -> Recipe {
  let part = part.into();
  recipe
    .base(part, |s| {
      fading(
        s.square(Length::Icon(IconSize::Md))
          .radius(RadiusSize::Sm)
          .foreground(PaletteColor::MutedForeground)
          .cursor(CursorIcon::Pointer),
      )
    })
    .hovered(part, |s| s.foreground(PaletteColor::Foreground))
    .focus_visible(part, |s| s.focus_ring())
}

/// The parts of an alert a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AlertPart {
  /// The box: its fill, its edge, its corners, and the room inside it.
  Root,
  /// The icon of its variant, at the start.
  Icon,
  /// The title, the description and the body, one over the other.
  Text,
  /// What it is about, in a few words.
  Title,
  /// More of it, under the title.
  Description,
  /// What the app puts under the description: a list, a link, a progress.
  Body,
  /// What holds the actions, before the close control, or under the words of a stacked alert.
  Action,
  /// What closes it, at the end.
  Close,
  /// A quieter line after the actions: what the actions do not say.
  Note,
  /// Of a compact alert: the icon and the title beside it, over the rest.
  Head,
}

names!(AlertPart {
  Root => "root",
  Icon => "icon",
  Text => "text",
  Title => "title",
  Description => "description",
  Body => "body",
  Action => "action",
  Close => "close",
  Note => "note",
  Head => "head",
});

/// The states of an alert a recipe styles, besides what the pointer and the
/// keyboard do to its close control.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AlertState {
  /// It has an action: what is in it is centered on the actions' height, where they are at its end.
  Action,
  /// Its actions are under its words, at their start, and not at its end.
  Stacked,
  /// What is in it stands at the top: its actions at its end are on its first line.
  AlignTop,
  /// It has a note after its actions.
  Note,
  /// The icon is beside the title alone, and the rest under them across the alert: an inspector's.
  Compact,
  /// Its actions under its words stand at their end.
  ActionsEnd,
}

names!(AlertState {
  Action => "action",
  Stacked => "stacked",
  AlignTop => "align_top",
  Note => "note",
  Compact => "compact",
  ActionsEnd => "actions_end",
});

/// What builds an alert's nodes: a theme's in place of the default one.
pub type AlertRenderer = Arc<dyn Fn(AlertView) -> Element + Send + Sync>;

/// An alert: a message in the flow of a page about what is on it. A file
/// that could not be saved, a disk that is nearly full, a shortcut to know.
/// Its tone is an [`AlertVariant`], or a variant a theme adds.
///
/// ```
/// use lurq::ui::{Alert, AlertVariant, Button, ButtonSize, ButtonVariant};
///
/// let failed = Alert::new("Could not save")
///   .description("The settings file is read-only.")
///   .variant(AlertVariant::Destructive)
///   .action(Button::new("Retry").variant(ButtonVariant::Outline).size(ButtonSize::Sm));
/// // Several actions, under its words: the least first, the main one last.
/// let offline = Alert::new("Working offline")
///   .description("Changes are kept on this computer until the server answers again.")
///   .variant(AlertVariant::Warning)
///   .stacked(true)
///   .action(Button::new("Details").variant(ButtonVariant::Ghost).size(ButtonSize::Sm))
///   .action(Button::new("Reconnect").variant(ButtonVariant::Outline).size(ButtonSize::Sm));
/// // No title, the icon and the action at the top of its first line, and a note after the action.
/// let token = Alert::untitled()
///   .description("The token expires in 3 days.")
///   .variant(AlertVariant::Warning)
///   .align_top(true)
///   .action(Button::new("Rotate").variant(ButtonVariant::Outline).size(ButtonSize::Sm))
///   .note("A new token is shown once.");
/// // An inspector's: the icon beside the title alone, the rest under them, the actions at the end.
/// let narrow = Alert::new("Not synced")
///   .description("This profile has changes the team has not seen.")
///   .compact(true)
///   .actions_end(true)
///   .action(Button::new("Push").size(ButtonSize::Sm));
/// let shortcut = Alert::new("New shortcut")
///   .description("Press Ctrl+K to open the command palette.")
///   .variant(AlertVariant::Info)
///   .on_close(|| println!("dismissed"));
/// ```
///
/// It is read where it stands and is not an overlay: the page lays it out
/// as wide as the room it is given. It is an `alert` to tooling that reads
/// the tree for a warning and for what went wrong, a `status` for the
/// others, called by its title ([`Alert::label`] and [`Alert::role`] say
/// otherwise). It has a close control once it is given
/// [`on_close`](Alert::on_close), and none until then. An alert with an empty
/// title ([`Alert::untitled`]) draws none, and is called by its description.
pub struct Alert {
  title: Arc<str>,
  description: Option<Arc<str>>,
  variant: Name,
  icon: Option<IconChoice>,
  actions: Vec<Element>,
  stacked: bool,
  align_top: bool,
  note: Option<Arc<str>>,
  compact: bool,
  actions_end: bool,
  body: Vec<Element>,
  role: Option<Arc<str>>,
  label: Option<Arc<str>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_close: Vec<Arc<dyn Fn() + Send + Sync>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Alert {
  /// An alert that says `title`.
  pub fn new(title: &str) -> Self {
    Self {
      title: Arc::from(title),
      description: None,
      variant: AlertVariant::Neutral.into(),
      icon: None,
      actions: Vec::new(),
      stacked: false,
      align_top: false,
      note: None,
      compact: false,
      actions_end: false,
      body: Vec::new(),
      role: None,
      label: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_close: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// An alert with no title: its description says it all. It draws no title,
  /// and is called by its description to tooling that reads the tree.
  pub fn untitled() -> Self {
    Self::new("")
  }

  /// More of it, under the title.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// Its tone: an [`AlertVariant`], or a variant a theme adds, which has the
  /// neutral one's icon and role.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// An icon of the theme's set in place of the variant's, or none: `icon(None)`.
  pub fn icon(mut self, icon: impl Into<IconChoice>) -> Self {
    self.icon = Some(icon.into());
    self
  }

  /// Adds something that can be done about it, after the actions already
  /// there: a button, a link. An alert has several, the least first and the
  /// main one last, at its end before the close control, or under its words
  /// where it is [`stacked`](Self::stacked). A second call adds, and does not
  /// take the place of the first.
  pub fn action(mut self, action: impl Into<Element>) -> Self {
    self.actions.push(action.into());
    self
  }

  /// Adds each of `actions`, as [`action`](Self::action) does.
  pub fn actions(mut self, actions: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.actions.extend(actions.into_iter().map(Into::into));
    self
  }

  /// Whether its actions are under its words, at their start, wrapping where
  /// one line does not hold them, and not at its end: for words long enough
  /// to need the alert's width, or more actions than its end has room for.
  pub fn stacked(mut self, stacked: bool) -> Self {
    self.stacked = stacked;
    self
  }

  /// Whether what is in it stands at the top and not on the middle of its
  /// actions: the icon, the words, the actions at its end and the close
  /// control on its first line, however many lines the words take.
  pub fn align_top(mut self, top: bool) -> Self {
    self.align_top = top;
    self
  }

  /// A quieter line after its actions, under its words: what a press of them
  /// does not say ("A new token is shown once").
  pub fn note(mut self, note: &str) -> Self {
    self.note = (!note.is_empty()).then(|| Arc::from(note));
    self
  }

  /// Whether it is laid out for a narrow column, as an inspector has it: the
  /// icon beside the title alone, and the description, the body, the actions
  /// and the note under them across the whole alert. Its actions are under
  /// its words, as a [`stacked`](Self::stacked) one's are.
  pub fn compact(mut self, compact: bool) -> Self {
    self.compact = compact;
    self
  }

  /// Whether its actions, where they are under its words (a stacked or a
  /// compact alert), stand at their far end and not at their start.
  pub fn actions_end(mut self, end: bool) -> Self {
    self.actions_end = end;
    self
  }

  /// Adds an element under the description, for more than text: a list, a
  /// link, a progress.
  pub fn child(mut self, child: impl Into<Element>) -> Self {
    self.body.push(child.into());
    self
  }

  /// Adds elements under the description.
  pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.body.extend(children.into_iter().map(Into::into));
    self
  }

  /// What it is called to tooling that reads the tree, in place of its title.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// What it is to tooling that reads the tree, in place of its variant's:
  /// `"status"` for a warning that is no news, `"alert"` for an informative
  /// one that is urgent.
  pub fn role(mut self, role: &str) -> Self {
    self.role = Some(Arc::from(role));
    self
  }

  /// Puts the alert in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Called on a press of its close control, which it has once it is given
  /// one: an alert with no `on_close` has no close control. The app takes the
  /// alert away. A second handler is called after the first.
  pub fn on_close(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_close.push(Arc::new(handler));
    self
  }

  /// Styles one part of this alert over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this alert.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the close control as it is under the pointer, held down or
  /// focused from the keyboard, with nothing pointing at it: for a gallery
  /// or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the box.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the alert in `theme`. Converting an alert into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    // A variant a theme adds has the neutral one's icon and role.
    let tone = AlertVariant::named(self.variant).unwrap_or_default();
    let case = Case {
      variant: Some(self.variant),
      size: None,
      density: None,
      breakpoint: current_breakpoint(),
      states: self.states.clone(),
    }
    .state(AlertState::Action, !self.actions.is_empty())
    .state(AlertState::Stacked, self.stacked)
    .state(AlertState::AlignTop, self.align_top)
    .state(AlertState::Note, self.note.is_some())
    .state(AlertState::Compact, self.compact)
    .state(AlertState::ActionsEnd, self.actions_end);
    let view = AlertView {
      title: self.title,
      description: self.description,
      variant: self.variant,
      icon: shown_icon(self.icon, tone.icon()),
      actions: self.actions,
      stacked: self.stacked,
      align_top: self.align_top,
      note: self.note,
      compact: self.compact,
      actions_end: self.actions_end,
      body: self.body,
      role: self.role.unwrap_or_else(|| Arc::from(tone.role())),
      label: self.label,
      close_button: !self.on_close.is_empty(),
      recipe: theme
        .resolve::<Alert>(&case, Some(&self.rules))
        .owned_by(self.id.as_ref()),
      forced: self.forced,
      on_close: self.on_close,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Alert>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Alert> for Element {
  fn from(alert: Alert) -> Self {
    alert.build(&current_theme())
  }
}

/// An alert as a renderer sees it: what it was given, the style of its
/// parts, and its behavior ready to be dressed.
pub struct AlertView {
  pub title: Arc<str>,
  pub description: Option<Arc<str>>,
  /// Its tone: an [`AlertVariant`]'s name, or a theme's.
  pub variant: Name,
  /// The icon it shows, by its name in the theme's set: the variant's, the one it was given, or none.
  pub icon: Option<Name>,
  /// What the app gave it to do, in the order it was given.
  pub actions: Vec<Element>,
  /// Whether its actions are under its words and not at its end.
  pub stacked: bool,
  /// Whether what is in it stands at the top, its actions at its end on its first line.
  pub align_top: bool,
  /// A quieter line after its actions.
  pub note: Option<Arc<str>>,
  /// Whether the icon is beside the title alone, the rest under them.
  pub compact: bool,
  /// Whether its actions under its words stand at their end.
  pub actions_end: bool,
  /// What the app put under the description.
  pub body: Vec<Element>,
  /// What it is to tooling that reads the tree: `alert` or `status`, or what the app said.
  pub role: Arc<str>,
  /// What it is called to tooling, where that is not its title.
  pub label: Option<Arc<str>>,
  /// Whether it has a close control: it was given a handler for it.
  pub close_button: bool,
  /// The style of each part of this alert.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  on_close: Vec<Arc<dyn Fn() + Send + Sync>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl AlertView {
  /// The style of a part, as this alert is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The icon in the style of its part, if the alert has one.
  pub fn icon(&self) -> Option<Element> {
    self.icon.map(|icon| glyph(&self.part(AlertPart::Icon), icon, false))
  }

  /// Whether its actions are under its words, and not at its end: it is
  /// stacked, or compact.
  pub fn actions_under(&self) -> bool {
    self.stacked || self.compact
  }

  /// What it is called to tooling: its label, its title, or its description
  /// where it has no title.
  pub fn name(&self) -> Arc<str> {
    match (&self.label, &self.description) {
      (Some(label), _) => label.clone(),
      (None, Some(description)) if self.title.is_empty() => description.clone(),
      _ => self.title.clone(),
    }
  }

  /// The title in the style of its part: `None` where it is empty.
  pub fn title(&self) -> Option<Column> {
    (!self.title.is_empty()).then(|| wrapping(&self.part(AlertPart::Title), &self.title))
  }

  /// The note in the style of its part, if it has one.
  pub fn note(&self) -> Option<Column> {
    let note = self.note.as_ref()?;
    Some(wrapping(&self.part(AlertPart::Note), note))
  }

  /// The title over the description, and the body under them, as wide as the
  /// room the icon, the actions and the close control leave. A stacked
  /// alert's actions are under the body, and the note is last. A compact
  /// alert's title is in its head, and not here.
  pub fn text(&mut self) -> Column {
    let mut text = Column::new()
      .min_width(Dimension::Px(0.0))
      .part(&self.part(AlertPart::Text));
    // Beside the icon it takes the room left of the row; under the head of a compact alert it is as wide as the
    // alert already, and a column that grew would take all the height it may.
    if !self.compact {
      text = text.flex(1.0);
    }
    if !self.compact
      && let Some(title) = self.title()
    {
      text = text.child(title);
    }
    if let Some(description) = &self.description {
      text = text.child(wrapping(&self.part(AlertPart::Description), description));
    }
    if !self.body.is_empty() {
      text = text.child(
        Column::new()
          .width(Dimension::Pct(100.0))
          .part(&self.part(AlertPart::Body))
          .with_children(std::mem::take(&mut self.body)),
      );
    }
    if self.actions_under()
      && let Some(actions) = self.actions()
    {
      text = text.child(actions);
    }
    if let Some(note) = self.note() {
      text = text.child(note);
    }
    text
  }

  /// The actions it was given, in a row in the style of its part: as wide as
  /// the words and wrapping under them, at their start or at their end, in a
  /// stacked or a compact alert, as wide as they are at the end of another.
  /// `None` for an alert with none.
  pub fn actions(&mut self) -> Option<Row> {
    if self.actions.is_empty() {
      return None;
    }
    let row = Row::new().align_items(Alignment::Center);
    let row = match (self.actions_under(), self.actions_end) {
      (true, true) => row.width(Dimension::Pct(100.0)).wrap().justify(Justify::End),
      (true, false) => row.width(Dimension::Pct(100.0)).wrap(),
      (false, _) => row,
    };
    Some(
      row
        .part(&self.part(AlertPart::Action))
        .with_children(std::mem::take(&mut self.actions)),
    )
  }

  /// What closes it: a press of it tells every handler of `on_close`.
  pub fn close(&self) -> components::Button {
    close_control(&self.part(AlertPart::Close), &self.on_close)
  }

  /// The head of a compact alert: its icon and its title beside it, and its
  /// close control at the end.
  pub fn head(&self) -> Row {
    let mut head = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Start)
      .part(&self.part(AlertPart::Head));
    if let Some(icon) = self.icon() {
      head = head.child(icon);
    }
    match self.title() {
      Some(title) => head = head.child(title.flex(1.0).min_width(Dimension::Px(0.0))),
      None => head = head.child(Row::new().flex(1.0)),
    }
    if self.close_button {
      head = head.child(self.close());
    }
    head
  }

  /// How the default renderer builds an alert: its icon, its text, its
  /// actions and its close control side by side, or its actions under its
  /// words where it is stacked; a compact one its head over the rest. It is
  /// what its role says to what reads the tree, called by its label, its
  /// title or its description.
  pub fn render(mut self) -> Element {
    let root = if self.compact {
      let rest =
        self.description.is_some() || !self.body.is_empty() || !self.actions.is_empty() || self.note.is_some();
      let mut root = Column::new()
        .align_items(Alignment::Stretch)
        .part(&self.part(AlertPart::Root))
        .describe("a11y_role", self.role.clone())
        .describe("a11y_name", self.name())
        .child(self.head());
      if rest {
        root = root.child(self.text());
      }
      root.into()
    } else {
      self.side_by_side()
    };
    let root = match &self.id {
      Some(id) => root.id(id.clone()),
      None => root,
    };
    match &self.key {
      Some(key) => root.key(key.clone()),
      None => root,
    }
  }

  /// The icon, the text, the actions and the close control in a row.
  fn side_by_side(&mut self) -> Element {
    let centered = !self.actions.is_empty() && !self.actions_under() && !self.align_top;
    let mut root = Row::new()
      .align_items(if centered { Alignment::Center } else { Alignment::Start })
      .part(&self.part(AlertPart::Root))
      .describe("a11y_role", self.role.clone())
      .describe("a11y_name", self.name());
    if let Some(icon) = self.icon() {
      root = root.child(icon);
    }
    root = root.child(self.text());
    if let Some(actions) = self.actions() {
      root = root.child(actions);
    }
    if self.close_button {
      root = root.child(self.close());
    }
    root.into()
  }
}

impl crate::ui::Styled for Alert {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for Alert {
  const NAME: &'static str = "alert";
  type Renderer = AlertRenderer;

  fn parts() -> crate::ui::Declared {
    AlertPart::declared()
  }

  fn states() -> crate::ui::Declared {
    AlertState::declared()
  }

  /// The design's alert (`pencil/lurq-design-system.pen`, Alert and its
  /// severities): a box as wide as the room it is given, in the card's
  /// color with an edge of the border's color and corners of `radius-lg`,
  /// padded 16. In it the icon of 16, 12 from the title (14, medium) over the
  /// description (13, muted, 1.5 high), 4 apart. An informative, a successful
  /// and a warning alert are in the muted form of their color, with an edge
  /// and an icon of their color and a description in the foreground's. One
  /// that went wrong keeps the card's color, with an edge, an icon and a
  /// title of the destructive color. The close control is the toast's: 16 in
  /// the muted color. Its actions are 8 apart; a stacked alert's are 8 under
  /// its words, as its body is. The board draws no note and no compact
  /// alert: a note is 12 in the muted color, 4 under what is over it; a
  /// compact alert's icon is 8 from its title, and its head 6 over the rest.
  fn recipe() -> Recipe {
    use AlertPart as P;
    use AlertVariant::{Destructive, Info, Success, Warning};
    use PaletteColor::{Card, CardForeground, Foreground, MutedForeground};
    let edge = |color: PaletteColor| Border::inside(BorderSize::Sm, color);
    let mut recipe = Recipe::new()
      .base(P::Root, |s| {
        s.width(Length::Fill)
          .background(Card)
          .foreground(CardForeground)
          .border(edge(PaletteColor::Border))
          .radius(RadiusSize::Lg)
          .padding_x(16.0)
          .padding_y(16.0)
          .gap(12.0)
      })
      .base(P::Icon, |s| s.square(Length::Icon(IconSize::Md)).foreground(Foreground))
      .base(P::Text, |s| s.gap(4.0))
      .base(P::Title, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(14.0)
          .weight(FontWeight::Medium)
          .foreground(Foreground)
      })
      .base(P::Description, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .line_height(1.5)
          .foreground(MutedForeground)
      })
      .base(P::Body, |s| s.gap(8.0).padding_top(4.0))
      .base(P::Action, |s| s.gap(8.0))
      .state(AlertState::Stacked, P::Action, |s| s.padding_top(4.0))
      .state(AlertState::Compact, P::Action, |s| s.padding_top(4.0))
      .base(P::Note, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(12.0)
          .line_height(1.5)
          .foreground(MutedForeground)
      })
      .base(P::Head, |s| s.gap(8.0))
      // Compact, the head is 6 over the rest.
      .state(AlertState::Compact, P::Root, |s| s.gap(6.0));
    recipe = close_style(recipe, P::Close);
    for variant in [Info, Success, Warning] {
      recipe = recipe
        .variant(variant, P::Root, |s| {
          s.background(variant.tint()).border(edge(variant.accent()))
        })
        .variant(variant, P::Description, |s| s.foreground(Foreground));
    }
    for variant in [Info, Success, Warning, Destructive] {
      recipe = recipe.variant(variant, P::Icon, |s| s.foreground(variant.accent()));
    }
    recipe
      .variant(Destructive, P::Root, |s| s.border(edge(PaletteColor::Destructive)))
      .variant(Destructive, P::Title, |s| s.foreground(PaletteColor::Destructive))
  }
}
