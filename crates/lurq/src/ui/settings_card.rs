//! A settings card: one setting in a card of its own, which may open to more of them, or the rows of several
//! settings in one card. A row of it is a [`ListItem`].

use std::sync::Arc;

use super::{
  collapsible::opening,
  list_item::{List, ListItem, ListItemPart, ListItemState},
  names,
  part_style::{Length, PartStyle},
  recipe::{Case, Interaction, Name, ROW_STEPS, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  text_box::edge,
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{Density, OpacityLevel, PaletteColor, RadiusSize, Theme},
  },
  components::Column,
  core::Signal,
  layout::Alignment,
  node::Element,
};

/// The parts of a settings card a recipe styles. Its rows are [`ListItem`]s in their `Setting` state, styled by
/// the list item's recipe.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SettingsCardPart {
  /// The card: its fill, its edge, its corners, and what it holds one under another.
  Root,
  /// What the card says of the root of its own row, the setting it is the card of: the room over and under it.
  Header,
  /// What opens and closes under the card's own row.
  Panel,
  /// What the panel holds: the rows the card opens to, under a line.
  Body,
}

names!(SettingsCardPart {
  Root => "root",
  Header => "header",
  Panel => "panel",
  Body => "body",
});

/// The states of a settings card a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SettingsCardState {
  /// The rows under its own row are shown.
  Open,
  /// It takes no input: nothing in it is pressed, and it is dimmed.
  Disabled,
  /// It has a row of its own: the setting it is the card of.
  Header,
  /// Its own row has a description under its title.
  Description,
}

names!(SettingsCardState {
  Open => "open",
  Disabled => "disabled",
  Header => "header",
  Description => "description",
});

/// What builds a settings card's nodes: a theme's in place of the default one.
pub type SettingsCardRenderer = Arc<dyn Fn(SettingsCardView) -> Element + Send + Sync>;

type OpenHandler = Arc<dyn Fn(bool) + Send + Sync>;

/// A settings card: a card of a settings page. One setting with its icon, its title, its description and the
/// control that sets it, which may open to more settings under it; or several settings, a row each, in one card.
///
/// ```
/// use lurq::{
///   core::Signal,
///   ui::{Button, ButtonVariant, ListItem, Select, SettingsCard, Switch, icons},
/// };
///
/// # fn build(ctx: &mut lurq::app::ctx::Ctx, theme: Signal<&'static str>) {
/// // One setting in a card of its own.
/// let appearance = SettingsCard::new(
///   ListItem::new("App theme")
///     .icon(icons::PALETTE)
///     .description("Choose how lurq looks.")
///     .trailing(Select::new(theme, [("system", "System"), ("light", "Light"), ("dark", "Dark")])),
/// )
/// // With rows, it opens to them: a press of its own row shows and hides them. Mounted, it keeps whether it is
/// // open.
/// .item(ListItem::new("High contrast").trailing(Switch::new("High contrast").label_hidden(true)))
/// .item(ListItem::new("Accent color").value("Blue").chevron(true).on_click(|_| println!("accent")))
/// .mount(ctx);
///
/// // Several settings in one card, a row each.
/// let startup = SettingsCard::group([
///   ListItem::new("Open at login")
///     .description("Start lurq when you sign in to this computer.")
///     .trailing(Switch::new("Open at login").label_hidden(true)),
///   ListItem::new("Default project folder")
///     .description("Where new projects are created.")
///     .trailing(Button::new("Choose…").variant(ButtonVariant::Outline).size("sm")),
/// ])
/// .label("Startup");
/// # }
/// ```
///
/// A row is a [`ListItem`]: its `icon`, its `description`, the control at its end (`trailing`), a `value` and a
/// `chevron` for one that leads to a page of its own (`on_click`, `href`). The card draws each as a setting, with
/// a line between each two.
///
/// A card with a row of its own and rows under it opens: its own row is pressed, by the pointer and by Enter and
/// Space, shows a chevron at its end, and says whether the card is open to what reads the tree. A press of the
/// control in that row is the control's, not the card's. Whether it is open is the app's ([`open`](Self::open)
/// with [`on_open_change`](Self::on_open_change), or [`bind`](Self::bind)), or the card's own once it is mounted
/// ([`mount`](Self::mount), starting from [`default_open`](Self::default_open)). The rows open and close with
/// their height, as a collapsible's panel does.
#[derive(Clone)]
pub struct SettingsCard {
  header: Option<ListItem>,
  items: Vec<ListItem>,
  open: Option<bool>,
  default_open: bool,
  bound: Option<Signal<bool>>,
  on_open_change: Vec<OpenHandler>,
  label: Option<Arc<str>>,
  disabled: bool,
  density: Option<Density>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl SettingsCard {
  /// The card of one setting: `setting` is its row, with its icon, its title, its description and the control
  /// that sets it at its end. Rows given with [`item`](Self::item) are what it opens to.
  pub fn new(setting: ListItem) -> Self {
    Self {
      header: Some(setting),
      ..Self::group(Vec::new())
    }
  }

  /// A card of several settings, a row each, with a line between each two.
  pub fn group(items: impl IntoIterator<Item = ListItem>) -> Self {
    Self {
      header: None,
      items: items.into_iter().collect(),
      open: None,
      default_open: false,
      bound: None,
      on_open_change: Vec::new(),
      label: None,
      disabled: false,
      density: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// A row, after those it holds already: one of the settings of a group, or one of those the card of one
  /// setting opens to.
  pub fn item(mut self, item: ListItem) -> Self {
    self.items.push(item);
    self
  }

  /// Rows, in order, after those it holds already.
  pub fn items(mut self, items: impl IntoIterator<Item = ListItem>) -> Self {
    self.items.extend(items);
    self
  }

  /// Whether the rows under the card's own row are shown. The card does not keep it: the app says so, and hears
  /// of a press with [`on_open_change`](Self::on_open_change).
  pub fn open(mut self, open: bool) -> Self {
    self.open = Some(open);
    self
  }

  /// Open while `open` is true, which a press of the card's own row turns. The handlers of
  /// [`on_open_change`](Self::on_open_change) are told of a press after the signal is set; a value the app sets
  /// is told to none of them.
  pub fn bind(mut self, open: Signal<bool>) -> Self {
    self.open = Some(open.get());
    self.bound = Some(open);
    self
  }

  /// Whether the card starts open, where it keeps that itself: mounted, with neither [`open`](Self::open) nor
  /// [`bind`](Self::bind).
  pub fn default_open(mut self, open: bool) -> Self {
    self.default_open = open;
    self
  }

  /// Called on a press of the card's own row, with what the card is to be: open if it was closed. Not for what
  /// the app sets. A second handler is called after the first.
  pub fn on_open_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_open_change.push(Arc::new(handler));
    self
  }

  /// What the card is called, to what reads the tree, in place of the title of its own row. It draws nothing: a
  /// heading over a group of settings is the page's.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Whether the card takes no input: nothing in it is pressed or focused, and it is dimmed.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// The density its rows are at, whatever the theme is at.
  pub fn density(mut self, density: Density) -> Self {
    self.density = Some(density);
    self
  }

  /// A width of its own in place of all the room it is given: a number of logical pixels, or another [`Length`].
  pub fn width(mut self, width: impl Into<Length>) -> Self {
    let width = width.into();
    self.rules = self.rules.base(SettingsCardPart::Root, |s| s.width(width));
    self
  }

  /// Puts the card in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this card over what the theme gives it. A row is styled on the row
  /// ([`ListItem::style`]).
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this card.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the row Tab would stop on first as it is under the pointer, held down or focused from the keyboard,
  /// with nothing pointing at it: the card's own row where it opens, every pressed row of a group. For a gallery
  /// or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the card's root. A row has the id the app gave it.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The card mounted where it is declared: where the app says neither [`open`](Self::open) nor
  /// [`bind`](Self::bind), it keeps whether it is open from render to render, starting from
  /// [`default_open`](Self::default_open), and a press of its own row turns it.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let props = SettingsCardProps { card: self };
    match key {
      Some(key) => ctx.mount_keyed::<SettingsCardHost>(&key, props),
      None => ctx.mount::<SettingsCardHost>(props),
    }
  }

  /// Builds the card in `theme`. Converting a card into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let title = self.header.as_ref().map(ListItem::title_text);
    let (header, items) = (self.header, self.items);
    let opens = header.is_some() && !items.is_empty();
    let open = opens && self.open.unwrap_or(self.default_open);
    let case = Case {
      variant: None,
      size: None,
      density: self.density.map(|density| Name::new(density.as_str())),
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(SettingsCardState::Open, open)
    .state(SettingsCardState::Disabled, self.disabled)
    .state(SettingsCardState::Header, header.is_some())
    .state(
      SettingsCardState::Description,
      header.as_ref().is_some_and(ListItem::has_description),
    );
    // A bound card sets its signal first: its handlers are told of a press once it is so.
    let mut on_open_change = self.on_open_change;
    if let Some(bound) = self.bound {
      on_open_change.insert(0, Arc::new(move |next| bound.set(next)));
    }
    let view = SettingsCardView {
      open,
      opens,
      disabled: self.disabled,
      label: self.label.or(title),
      recipe: theme
        .resolve::<SettingsCard>(&case, Some(&self.rules))
        .owned_by(self.id.as_ref()),
      header,
      items,
      density: self.density,
      forced: self.forced,
      on_open_change,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<SettingsCard>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<SettingsCard> for Element {
  fn from(card: SettingsCard) -> Self {
    card.build(&current_theme())
  }
}

/// A settings card to mount.
#[derive(Clone)]
struct SettingsCardProps {
  card: SettingsCard,
}

/// A card is built anew by every render of what holds it, so props are never the same as before.
impl PartialEq for SettingsCardProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl crate::app::component::DevtoolsInspectable for SettingsCardProps {}

/// What keeps whether a mounted settings card is open: nothing until a person presses its row, and the card
/// starts as it was told to.
struct SettingsCardHost {
  open: Signal<Option<bool>>,
}

impl Component for SettingsCardHost {
  type Props = SettingsCardProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self { open: ctx.signal(None) }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let mut card = ctx.props::<SettingsCardProps>().card.clone();
    if card.open.is_none() {
      card.open = Some(self.open.get().unwrap_or(card.default_open));
      // The card's own signal is set first, as a bound one is.
      let held = self.open.clone();
      card
        .on_open_change
        .insert(0, Arc::new(move |next| held.set(Some(next))));
    }
    card.build(&current_theme())
  }
}

/// A settings card as a renderer sees it: its rows, the style of its parts, and its behavior ready to be dressed.
pub struct SettingsCardView {
  /// Whether the rows under the card's own row are shown.
  pub open: bool,
  /// Whether the card opens: it has a row of its own and rows under it.
  pub opens: bool,
  pub disabled: bool,
  /// What the card is called to what reads the tree: its label, or the title of its own row.
  pub label: Option<Arc<str>>,
  /// The style of each part of this card.
  pub recipe: ResolvedRecipe,
  header: Option<ListItem>,
  items: Vec<ListItem>,
  density: Option<Density>,
  forced: Option<Interaction>,
  on_open_change: Vec<OpenHandler>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl SettingsCardView {
  /// The style of a part, as this card is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The root in the style of its part, with nothing in it yet: a group to what reads the tree, called by its
  /// label, and disabled with everything in it where the card is.
  pub fn root(&self) -> Column {
    let mut root = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(SettingsCardPart::Root))
      .describe("a11y_role", "group");
    if let Some(label) = &self.label {
      root = root.describe("a11y_name", label.clone());
    }
    if self.disabled {
      root = root.disabled(true);
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// The card's own row, where it has one: a setting, with the room the card gives it. Where the card opens it is
  /// pressed, after what the app made of its press, and shows which way the card is.
  pub fn header(&mut self) -> Option<Element> {
    let header = self.header.take()?;
    let rules = self.recipe.rules_for(SettingsCardPart::Header, ListItemPart::Root);
    let mut header = header
      .state(ListItemState::Setting, true)
      .rules(&rules)
      .inherit(self.density, self.forced);
    if self.opens {
      let (handlers, next) = (self.on_open_change.clone(), !self.open);
      header = header.expands(self.open).on_click(move |_| {
        for handler in &handlers {
          handler(next);
        }
      });
    }
    Some(header.build(self.theme()))
  }

  /// The card's rows, each a setting, one under another with a line between each two: a list to what reads the
  /// tree. `None` for a card with none.
  pub fn rows(&mut self) -> Option<Element> {
    if self.items.is_empty() {
      return None;
    }
    let items = std::mem::take(&mut self.items)
      .into_iter()
      .map(|item| item.state(ListItemState::Setting, true));
    let mut list = List::new()
      .separators(true)
      .items(items)
      .rules(&self.recipe.lend("list"));
    if let Some(density) = self.density {
      list = list.density(density);
    }
    // In a card that opens, the row Tab stops on first is its own.
    if !self.opens {
      list = list.force(self.forced);
    }
    if let Some(label) = &self.label {
      list = list.label(label);
    }
    Some(list.build(self.theme()))
  }

  /// What opens and closes under the card's own row, with the rows while the card is open. Its height is the
  /// recipe's: none while it is closed and its rows' while it is open, with a transition between the two. What it
  /// held stays while the panel closes over it, and is gone with the body's exit animation.
  pub fn panel(&mut self) -> Column {
    let mut panel = Column::new()
      .align_items(Alignment::Stretch)
      .clip()
      .presence()
      .part(&self.part(SettingsCardPart::Panel));
    if self.open
      && let Some(rows) = self.rows()
    {
      panel = panel.child(
        Column::new()
          .key("content")
          .align_items(Alignment::Stretch)
          .part(&self.part(SettingsCardPart::Body))
          .child(rows),
      );
    }
    panel
  }

  /// How the default renderer builds a card: its own row and the panel under it where it opens, or its rows.
  pub fn render(mut self) -> Element {
    let opens = self.opens;
    let header = self.header();
    let mut root = self.root();
    if let Some(header) = header {
      root = root.child(header);
    }
    if opens {
      root = root.child(self.panel());
    } else if let Some(rows) = self.rows() {
      root = root.child(rows);
    }
    root.into()
  }
}

impl crate::ui::Styled for SettingsCard {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for SettingsCard {
  const NAME: &'static str = "settings_card";
  type Renderer = SettingsCardRenderer;

  fn parts() -> crate::ui::Declared {
    SettingsCardPart::declared()
  }

  fn states() -> crate::ui::Declared {
    SettingsCardState::declared()
  }

  /// The design's settings card (`pencil/lurq-design-system.pen`, Settings Card, and the cards of Settings Rows in
  /// the Preferences window): a card on `Card` with an edge of 1 in the border's color and the large radius, as
  /// wide as its room. Its own row is a setting 14 from the top and the bottom of the card, the board's 61 with a
  /// description, with a chevron of 16 at its end where it opens; its rows are settings with a line of 1 between
  /// each two. The board draws no open card: the rows are under a line, and open with their height over the
  /// theme's base duration. A disabled card is dimmed.
  fn recipe() -> Recipe {
    use PaletteColor::{Card, CardForeground};
    use SettingsCardPart::{Body, Header, Panel, Root};
    let recipe = Recipe::new()
      .base(Root, |s| {
        s.width(Length::Fill)
          .max_width(Length::Fill)
          .background(Card)
          .foreground(CardForeground)
          .border(edge(PaletteColor::Border))
          .radius(RadiusSize::Lg)
      })
      .state(SettingsCardState::Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
      // The card's own row has the card's corners: the ring of its focus and its veil are round with it, and
      // square where the rows are under it.
      .base(Header, |s| s.padding_y(14.0).radius(RadiusSize::Lg))
      .state(SettingsCardState::Open, Header, |s| {
        s.corners(RadiusSize::Lg, RadiusSize::Lg, 0.0, 0.0)
      })
      // Two lines of the theme's type are a pixel under the board's: the row is the board's 61 at the least.
      .state(SettingsCardState::Description, Header, |s| s.min_height(61.0))
      .base(Body, |s| s.top_line(edge(PaletteColor::Border)));
    // The card's own row follows the density, as a setting does.
    let recipe = ROW_STEPS.iter().fold(recipe, |recipe, (density, step)| {
      recipe
        .density(*density, Header, |s| s.padding_y(14.0 + step / 2.0))
        .when(
          Selector::new().density(*density).state(SettingsCardState::Description),
          Header,
          |s| s.min_height(61.0 + step),
        )
    });
    opening(recipe, SettingsCardState::Open.into(), Panel.into(), Body.into())
  }
}
