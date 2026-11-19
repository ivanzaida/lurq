//! The menu a player's speed is chosen from.

use std::sync::Arc;

use super::{
  MEDIA_RATES, Menu, MenuItem, MenuItemPart, MenuPart, media_rate_text, names,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    ctx::{CollisionStrategy, Placement},
    theme::Theme,
  },
  core::{ElementRef, Signal},
  node::{Element, SpacingValue},
};

/// The parts of a speed menu a recipe styles. Each is laid over the part of a [`Menu`] or a [`MenuItem`] it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MediaSpeedMenuPart {
  /// The panel: a menu's panel, 168 wide.
  Panel,
  /// The heading over the speeds: a menu's label.
  Label,
  /// A speed: a menu item's root.
  Item,
  /// The words of a speed: a menu item's label.
  ItemLabel,
  /// The mark at the end of the speed in use: a menu item's check.
  Check,
}

names!(MediaSpeedMenuPart {
  Panel => "panel",
  Label => "label",
  Item => "item",
  ItemLabel => "item_label",
  Check => "check",
});

/// The states of a speed menu a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MediaSpeedMenuState {
  /// It is drawn in place, and not over the window from its anchor.
  Inline,
}

names!(MediaSpeedMenuState {
  Inline => "inline",
});

/// What builds a speed menu's nodes: a theme's in place of the default one.
pub type MediaSpeedMenuRenderer = Arc<dyn Fn(MediaSpeedMenuView) -> Element + Send + Sync>;

/// The menu a player's speed is chosen from: the speeds, the one in use checked at the end of its row.
///
/// ```
/// use lurq::{
///   core::{ElementRef, Signal},
///   ui::MediaSpeedMenu,
/// };
///
/// # fn menu(open: Signal<bool>, button: ElementRef) -> MediaSpeedMenu {
/// MediaSpeedMenu::new(open, button, 1.0).on_change(|rate| println!("{rate}"))
/// # }
/// ```
///
/// It is open while `open` is true, anchored to what opens it, and closes on a choice, Escape, a press outside it
/// and Tab, as a [`Menu`] does: it is one, with the heading "Playback speed" and 168 wide. Up and Down move between
/// the speeds; [`MediaSpeedMenu::on_change`] is told the speed chosen where it is another than the one in use.
pub struct MediaSpeedMenu {
  open: Signal<bool>,
  anchor: ElementRef,
  rate: f32,
  rates: Vec<f32>,
  heading: Arc<str>,
  label: Option<Arc<str>>,
  on_change: Vec<Arc<dyn Fn(f64) + Send + Sync>>,
  on_dismiss: Vec<Arc<dyn Fn() + Send + Sync>>,
  on_open_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  placement: Option<Placement>,
  offset: Option<SpacingValue>,
  collision: Option<CollisionStrategy>,
  inline: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl MediaSpeedMenu {
  /// The menu of [`MEDIA_RATES`], open while `open` is true, anchored to `anchor`, with `rate` checked.
  pub fn new(open: Signal<bool>, anchor: impl Into<ElementRef>, rate: f32) -> Self {
    Self {
      open,
      anchor: anchor.into(),
      rate,
      rates: MEDIA_RATES.to_vec(),
      heading: super::current_locale().media.playback_speed.clone(),
      label: None,
      on_change: Vec::new(),
      on_dismiss: Vec::new(),
      on_open_change: Vec::new(),
      placement: None,
      offset: None,
      collision: None,
      inline: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// The speeds it offers, in place of [`MEDIA_RATES`]. Each is named by [`media_rate_text`].
  pub fn rates(mut self, rates: &[f32]) -> Self {
    self.rates = rates.to_vec();
    self
  }

  /// The words over the speeds: "Playback speed" unless told.
  pub fn heading_text(mut self, heading: &str) -> Self {
    self.heading = Arc::from(heading);
    self
  }

  /// The menu's name to what reads the tree: its heading unless told.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Called with the speed a person chooses, where it is another than the one in use, after the menu has closed. A
  /// second handler is called after the first.
  pub fn on_change(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Called when a person closes the menu other than by choosing: Escape, a press outside it, or Tab. Not when the
  /// app sets its signal. A second handler is called after the first.
  pub fn on_dismiss(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_dismiss.push(Arc::new(handler));
    self
  }

  /// Called with `false` when a person closes the menu: a choice, Escape, a press outside it, or Tab; after
  /// `on_dismiss`. A second handler is called after the first.
  pub fn on_open_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_open_change.push(Arc::new(handler));
    self
  }

  /// Where the panel opens beside its anchor: under it from its start unless told. In a player's bar, at the
  /// bottom of a picture, `Placement::TopEnd`.
  pub fn placement(mut self, placement: Placement) -> Self {
    self.placement = Some(placement);
    self
  }

  /// How far the panel is from its anchor, along the side it opens on, in place of the menu's 4.
  pub fn offset(mut self, offset: impl Into<SpacingValue>) -> Self {
    self.offset = Some(offset.into());
    self
  }

  /// What the panel does where the window has no room for it on the side it asks for.
  pub fn collision(mut self, collision: CollisionStrategy) -> Self {
    self.collision = Some(collision);
    self
  }

  /// Draws the panel where the menu is in the tree, open or not: for a gallery or a test. A choice still tells
  /// `on_change` and sets the open signal to `false`.
  pub fn inline(mut self, inline: bool) -> Self {
    self.inline = inline;
    self
  }

  /// Puts the menu in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this menu over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this menu.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the speed in use as it is under the pointer, held down or focused from the keyboard, with nothing
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

  /// Builds the menu in `theme`. Converting it into an element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(MediaSpeedMenuState::Inline, self.inline);
    let view = MediaSpeedMenuView {
      label: self.label.unwrap_or_else(|| self.heading.clone()),
      heading: self.heading,
      rate: self.rate,
      rates: self.rates,
      inline: self.inline,
      recipe: theme
        .resolve::<MediaSpeedMenu>(&case, Some(&self.rules))
        .owned_by(self.id.as_ref()),
      open: self.open,
      anchor: self.anchor,
      on_change: Arc::new(self.on_change),
      on_dismiss: self.on_dismiss,
      on_open_change: self.on_open_change,
      placement: self.placement,
      offset: self.offset,
      collision: self.collision,
      forced: self.forced,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<MediaSpeedMenu>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<MediaSpeedMenu> for Element {
  fn from(menu: MediaSpeedMenu) -> Self {
    menu.build(&current_theme())
  }
}

/// A speed menu as a renderer sees it: what it offers, the style of its parts, and the menu ready to be dressed.
pub struct MediaSpeedMenuView {
  /// The words over the speeds.
  pub heading: Arc<str>,
  /// The menu's name to what reads the tree.
  pub label: Arc<str>,
  /// The speed in use.
  pub rate: f32,
  /// The speeds offered, in order.
  pub rates: Vec<f32>,
  pub inline: bool,
  /// The style of each part of this menu.
  pub recipe: ResolvedRecipe,
  open: Signal<bool>,
  anchor: ElementRef,
  on_change: Arc<Vec<Arc<dyn Fn(f64) + Send + Sync>>>,
  on_dismiss: Vec<Arc<dyn Fn() + Send + Sync>>,
  on_open_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  placement: Option<Placement>,
  offset: Option<SpacingValue>,
  collision: Option<CollisionStrategy>,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl MediaSpeedMenuView {
  /// The style of a part, as this menu is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// Whether `rate` is the speed in use.
  pub fn is_current(&self, rate: f32) -> bool {
    (rate - self.rate).abs() < f32::EPSILON
  }

  /// The item of one speed, dressed by this menu's recipe over the theme's for a menu item: checked where it is the
  /// speed in use. Choosing another speed tells every handler of `on_change`; choosing the one in use only closes
  /// the menu.
  pub fn item(&self, rate: f32) -> MenuItem {
    let rules = self
      .recipe
      .rules_for(MediaSpeedMenuPart::Item, MenuItemPart::Root)
      .extend(
        &self
          .recipe
          .rules_for(MediaSpeedMenuPart::ItemLabel, MenuItemPart::Label),
      )
      .extend(&self.recipe.rules_for(MediaSpeedMenuPart::Check, MenuItemPart::Check));
    let (current, handlers) = (self.is_current(rate), self.on_change.clone());
    let mut item = MenuItem::new(&media_rate_text(rate))
      .checked(current)
      .key(format!("rate-{rate}"))
      .named_by(&self.recipe, MediaSpeedMenuPart::Item)
      .rules(&rules)
      .on_select(move || {
        if !current {
          for handler in handlers.iter() {
            handler(f64::from(rate));
          }
        }
      });
    if current {
      item = item.force(self.forced);
    }
    item
  }

  /// The menu of the speeds, under its heading, dressed by this menu's recipe over the theme's for a menu.
  pub fn menu(&self) -> Menu {
    let rules = self
      .recipe
      .rules_for(MediaSpeedMenuPart::Panel, MenuPart::Panel)
      .extend(&self.recipe.rules_for(MediaSpeedMenuPart::Label, MenuPart::Label));
    let mut menu = Menu::new(self.open.clone(), self.anchor.clone())
      .label(&self.label)
      .heading(&self.heading)
      .items(self.rates.iter().map(|rate| self.item(*rate)))
      .inline(self.inline)
      .rules(&rules);
    if let Some(placement) = self.placement {
      menu = menu.placement(placement);
    }
    if let Some(offset) = self.offset.clone() {
      menu = menu.offset(offset);
    }
    if let Some(collision) = self.collision {
      menu = menu.collision(collision);
    }
    for handler in &self.on_dismiss {
      let handler = handler.clone();
      menu = menu.on_dismiss(move || handler());
    }
    for handler in &self.on_open_change {
      let handler = handler.clone();
      menu = menu.on_open_change(move |open| handler(open));
    }
    if let Some(id) = &self.id {
      menu = menu.id(id.clone());
    }
    if let Some(key) = &self.key {
      menu = menu.key(key.clone());
    }
    menu
  }

  /// How the default renderer builds a speed menu: the menu, in the theme it was resolved in.
  pub fn render(self) -> Element {
    self.menu().build(self.theme())
  }
}

impl crate::ui::Styled for MediaSpeedMenu {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for MediaSpeedMenu {
  const NAME: &'static str = "media_speed_menu";
  const ROOT: &'static str = "panel";
  type Renderer = MediaSpeedMenuRenderer;

  fn parts() -> crate::ui::Declared {
    MediaSpeedMenuPart::declared()
  }

  fn states() -> crate::ui::Declared {
    MediaSpeedMenuState::declared()
  }

  /// The design's speed menu (`pencil/lurq-design-system.pen`, `Media/Speed Menu`, `XUFEp`): the menu's panel, 168
  /// wide where a menu is at least 230, with the group label "Playback speed" and a menu item for each speed, the
  /// one in use with the check at the end of its row. Everything else is the menu's and the menu item's.
  fn recipe() -> Recipe {
    use MediaSpeedMenuPart::Panel;
    Recipe::new().base(Panel, |s| s.width(168.0).min_width(168.0).max_width(168.0))
  }
}
