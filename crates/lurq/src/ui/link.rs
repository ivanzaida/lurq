//! A link: text that goes somewhere when it is pressed.

use std::sync::Arc;

use super::{
  icons, names,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{fading, glyph, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{IconSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  layout::text_style::FontWeight,
  node::{CursorIcon, Element, EventHandler, IntoMouseEventHandler, border::Border},
};

/// The parts of a link a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LinkPart {
  /// The link itself: what is pressed, and the line under it.
  Root,
  /// Its text.
  Label,
  /// The icon after its text.
  Icon,
}

names!(LinkPart {
  Root => "root",
  Label => "label",
  Icon => "icon",
});

/// The states of a link a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LinkState {
  /// It takes no input.
  Disabled,
}

names!(LinkState {
  Disabled => "disabled",
});

/// What builds a link's nodes: a theme's in place of the default one.
pub type LinkRenderer = Arc<dyn Fn(LinkView) -> Element + Send + Sync>;

/// A link: text that goes somewhere when it is pressed. It has no box, so
/// it sits in a line of text or beside one, and it is underlined under the
/// pointer.
///
/// ```
/// use lurq::ui::Link;
///
/// let docs = Link::new("Read the docs").external(true).on_click(|_| println!("to the docs"));
/// // A page on the web: a press opens it in the browser.
/// let site = Link::new("lurq.dev").href("https://lurq.dev").external(true);
/// ```
///
/// Where it goes is what its press does: an app with a router goes to a
/// route there, another opens a page. It is pressed by a click, and by Enter
/// or Space while it is focused.
#[derive(Clone)]
pub struct Link {
  label: Arc<str>,
  trailing_icon: Option<Name>,
  external: bool,
  href: Option<Arc<str>>,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_click: Vec<EventHandler<crate::app::events::MouseEvent>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Link {
  pub fn new(label: &str) -> Self {
    Self {
      label: Arc::from(label),
      trailing_icon: None,
      external: false,
      href: None,
      disabled: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_click: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// An icon after the text, by its name in the theme's set.
  pub fn trailing_icon(mut self, name: impl Into<Name>) -> Self {
    self.trailing_icon = Some(name.into());
    self
  }

  /// Whether the link leaves the app: it has an arrow up and to the right
  /// after its text, where it has no trailing icon of its own.
  pub fn external(mut self, external: bool) -> Self {
    self.external = external;
    self
  }

  /// The address it goes to: a press opens it in the browser ([`open_url`](crate::app::open_url())), after the
  /// link's [`on_click`](Self::on_click) handlers are told, unless one of them calls `prevent_default`. An app
  /// with a router gives it none, and goes to its route in `on_click`.
  pub fn href(mut self, url: &str) -> Self {
    self.href = Some(Arc::from(url.trim()));
    self
  }

  /// Whether the link takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Puts the link in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Called on a click, and on Enter and Space while the link is focused.
  pub fn on_click(mut self, handler: impl IntoMouseEventHandler) -> Self {
    self.on_click.push(handler.into_event_handler());
    self
  }

  /// Styles one part of this link over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this link: what the component it
  /// is in says of the link in this place, under the pointer too.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the link as it is under the pointer, held down or focused from
  /// the keyboard, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the link in `theme`. Converting a link into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(LinkState::Disabled, self.disabled);
    let view = LinkView {
      label: self.label,
      trailing_icon: self
        .trailing_icon
        .or_else(|| self.external.then(|| Name::from(icons::ARROW_UP_RIGHT))),
      href: self.href,
      disabled: self.disabled,
      recipe: theme.resolve::<Link>(&case, Some(&self.rules)),
      forced: self.forced,
      on_click: self.on_click,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Link>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Link> for Element {
  fn from(link: Link) -> Self {
    link.build(&current_theme())
  }
}

/// A link as a renderer sees it: what it was given, the style of its parts,
/// and its behavior ready to be dressed.
pub struct LinkView {
  pub label: Arc<str>,
  /// The name of the icon after its text, if it has one: its own, or the arrow of an external link.
  pub trailing_icon: Option<Name>,
  /// The address a press opens, where it has one.
  pub href: Option<Arc<str>>,
  pub disabled: bool,
  /// The style of each part of this link.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  on_click: Vec<EventHandler<crate::app::events::MouseEvent>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl LinkView {
  /// The style of a part, as this link is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The link's behavior with the style of its root: the element that is
  /// pressed and focused, with nothing in it yet.
  pub fn root(&self) -> crate::components::Button {
    // A group, so that the line under the label shows while the pointer is anywhere on the link: on its icon too.
    let mut root = pressable(&self.part(LinkPart::Root), self.disabled).interaction_group(true);
    for handler in &self.on_click {
      root = root.on_click(handler.clone());
    }
    // Opened after the app was told, where none of its handlers refused it.
    if let Some(url) = self.href.clone().filter(|_| !self.disabled) {
      root = root.on_click(move |event: crate::app::events::MouseEvent| {
        if !event.default_prevented()
          && let Err(error) = crate::app::open_url(&url)
        {
          tracing::warn!("the link did not open {url}: {error}");
        }
      });
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// How the default renderer builds a link: its text, and its icon after it.
  pub fn render(self) -> Element {
    let label = self.part(LinkPart::Label);
    let mut root = self.root().child(label.text(&self.label).nowrap().part(&label));
    if let Some(name) = self.trailing_icon {
      root = root.child(glyph(&self.part(LinkPart::Icon), name, false));
    }
    root.into()
  }
}

impl crate::ui::Styled for Link {
  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for Link {
  const NAME: &'static str = "link";
  type Renderer = LinkRenderer;

  fn parts() -> crate::ui::Declared {
    LinkPart::declared()
  }

  fn states() -> crate::ui::Declared {
    LinkState::declared()
  }

  /// The design's link (`pencil/lurq-design-system.pen`, Link): text at
  /// 14/500 in the foreground, an icon of 14 four after it, and a straight
  /// line of 1 under the text, not under the icon, while the pointer is on
  /// the link or the focus is in it. The line is the label's own edge: the
  /// root's has the radius of the focus ring, and would curl up at its ends.
  fn recipe() -> Recipe {
    use LinkPart::{Icon, Label, Root};
    use PaletteColor::Foreground;
    Recipe::new()
      .base(Root, |s| {
        fading(
          s.gap(4.0)
            .radius(RadiusSize::Sm)
            .foreground(Foreground)
            .cursor(CursorIcon::Pointer),
        )
      })
      .base(Label, |s| {
        s.typography(TypographyStyle::Body).weight(FontWeight::Medium)
      })
      .base(Icon, |s| s.square(IconSize::Sm))
      .within(Label, |s| s.underline(Border::inside(1.0, Foreground)))
      .focus_visible(Root, |s| s.focus_ring())
      .state(LinkState::Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
  }
}
