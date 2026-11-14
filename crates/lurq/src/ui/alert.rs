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
  layout::{Alignment, text_style::FontWeight},
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

/// The icon a message is given: one of the theme's set by its name, or none
/// (`None`), in place of its variant's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlertIcon(pub Option<Name>);

impl From<&str> for AlertIcon {
  fn from(name: &str) -> Self {
    Self(Some(Name::from(name)))
  }
}

impl From<Name> for AlertIcon {
  fn from(name: Name) -> Self {
    Self(Some(name))
  }
}

impl From<Option<Name>> for AlertIcon {
  fn from(name: Option<Name>) -> Self {
    Self(name)
  }
}

/// The icon a message shows: the one it was given, none where it was given
/// none, and its variant's (`default`) where it was given nothing.
pub(crate) fn shown_icon(given: Option<AlertIcon>, default: &'static str) -> Option<Name> {
  match given {
    Some(AlertIcon(icon)) => icon,
    None => Some(Name::from(default)),
  }
}

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
    .describe("a11y_name", "Close")
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
  /// What holds the action, before the close control.
  Action,
  /// What closes it, at the end.
  Close,
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
});

/// The states of an alert a recipe styles, besides what the pointer and the
/// keyboard do to its close control.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AlertState {
  /// It has an action: what is in it is centered on the action's height.
  Action,
}

names!(AlertState {
  Action => "action",
});

/// What builds an alert's nodes: a theme's in place of the default one.
pub type AlertRenderer = Arc<dyn Fn(AlertView) -> Element + Send + Sync>;

/// An alert: a message in the flow of a page about what is on it. A file
/// that could not be saved, a disk that is nearly full, a shortcut to know.
///
/// ```
/// use lurq::ui::{Alert, AlertVariant, Button, ButtonSize, ButtonVariant};
///
/// let failed = Alert::new("Could not save")
///   .description("The settings file is read-only.")
///   .variant(AlertVariant::Destructive)
///   .action(Button::new("Retry").variant(ButtonVariant::Outline).size(ButtonSize::Sm));
/// let shortcut = Alert::new("New shortcut")
///   .description("Press Ctrl+K to open the command palette.")
///   .variant(AlertVariant::Info)
///   .on_close(|| println!("dismissed"));
/// ```
///
/// It is read where it stands and is not an overlay: the page lays it out
/// as wide as the room it is given. It is an `alert` to tooling that reads
/// the tree for a warning and for what went wrong, a `status` for the
/// others, called by its title ([`Alert::role`] says otherwise).
pub struct Alert {
  title: Arc<str>,
  description: Option<Arc<str>>,
  variant: AlertVariant,
  icon: Option<AlertIcon>,
  action: Option<Element>,
  body: Vec<Element>,
  role: Option<Arc<str>>,
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
      variant: AlertVariant::Neutral,
      icon: None,
      action: None,
      body: Vec::new(),
      role: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_close: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// More of it, under the title.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  pub fn variant(mut self, variant: AlertVariant) -> Self {
    self.variant = variant;
    self
  }

  /// An icon of the theme's set in place of the variant's, or none: `icon(None)`.
  pub fn icon(mut self, icon: impl Into<AlertIcon>) -> Self {
    self.icon = Some(icon.into());
    self
  }

  /// What can be done about it, at its end before the close control: a
  /// button, a link.
  pub fn action(mut self, action: impl Into<Element>) -> Self {
    self.action = Some(action.into());
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
  /// one. The app takes the alert away.
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
    let case = Case {
      variant: Some(self.variant.into()),
      size: None,
      density: None,
      breakpoint: current_breakpoint(),
      states: self.states.clone(),
    }
    .state(AlertState::Action, self.action.is_some());
    let view = AlertView {
      title: self.title,
      description: self.description,
      variant: self.variant,
      icon: shown_icon(self.icon, self.variant.icon()),
      action: self.action,
      body: self.body,
      role: self.role.unwrap_or_else(|| Arc::from(self.variant.role())),
      closable: !self.on_close.is_empty(),
      recipe: theme.resolve::<Alert>(&case, Some(&self.rules)),
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
  pub variant: AlertVariant,
  /// The icon it shows, by its name in the theme's set: the variant's, the one it was given, or none.
  pub icon: Option<Name>,
  /// What the app put at its end.
  pub action: Option<Element>,
  /// What the app put under the description.
  pub body: Vec<Element>,
  /// What it is to tooling that reads the tree: `alert` or `status`, or what the app said.
  pub role: Arc<str>,
  /// Whether it has a close control: it was given a handler for it.
  pub closable: bool,
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

  /// The title over the description, and the body under them, as wide as the
  /// room the icon, the action and the close control leave.
  pub fn text(&mut self) -> Column {
    let mut text = Column::new()
      .flex(1.0)
      .min_width(Dimension::Px(0.0))
      .part(&self.part(AlertPart::Text))
      .child(wrapping(&self.part(AlertPart::Title), &self.title));
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
    text
  }

  /// The action it was given, in the style of its part.
  pub fn action(&mut self) -> Option<Row> {
    let action = self.action.take()?;
    Some(
      Row::new()
        .align_items(Alignment::Center)
        .part(&self.part(AlertPart::Action))
        .child(action),
    )
  }

  /// What closes it: a press of it tells every handler of `on_close`.
  pub fn close(&self) -> components::Button {
    close_control(&self.part(AlertPart::Close), &self.on_close)
  }

  /// How the default renderer builds an alert: its icon, its text, its action
  /// and its close control side by side. It is what its role says to what
  /// reads the tree, called by its title.
  pub fn render(mut self) -> Element {
    let mut root = Row::new()
      .align_items(if self.action.is_some() {
        Alignment::Center
      } else {
        Alignment::Start
      })
      .part(&self.part(AlertPart::Root))
      .describe("a11y_role", self.role.clone())
      .describe("a11y_name", self.title.clone());
    if let Some(icon) = self.icon() {
      root = root.child(icon);
    }
    root = root.child(self.text());
    if let Some(action) = self.action() {
      root = root.child(action);
    }
    if self.closable {
      root = root.child(self.close());
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

impl Themed for Alert {
  const NAME: &'static str = "alert";
  type Renderer = AlertRenderer;

  /// The design's alert (`pencil/lurq-design-system.pen`, Alert and its
  /// severities): a box as wide as the room it is given, in the card's
  /// color with an edge of the border's color and corners of `radius-lg`,
  /// padded 16. In it the icon of 16, 12 from the title (14, medium) over the
  /// description (13, muted, 1.5 high), 4 apart. An informative, a successful
  /// and a warning alert are in the muted form of their color, with an edge
  /// and an icon of their color and a description in the foreground's. One
  /// that went wrong keeps the card's color, with an edge, an icon and a
  /// title of the destructive color. The close control is the toast's: 16 in
  /// the muted color.
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
      .base(P::Action, |s| s.gap(8.0));
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
