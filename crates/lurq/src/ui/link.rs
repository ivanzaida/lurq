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
/// let docs = Link::new("Read the docs").external().on_click(|_| println!("to the docs"));
/// ```
///
/// Where it goes is what its press does: an app with a router goes to a
/// route there, another opens a page. It is pressed by a click, and by Enter
/// or Space while it is focused.
#[derive(Clone)]
pub struct Link {
  label: Arc<str>,
  icon: Option<Name>,
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
      icon: None,
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
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// The icon of a link that leaves the app: an arrow up and to the right.
  pub fn external(self) -> Self {
    self.icon(icons::ARROW_UP_RIGHT)
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
      icon: self.icon,
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
  /// The name of the icon after its text, if it has one.
  pub icon: Option<Name>,
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
    let mut root = pressable(&self.part(LinkPart::Root), self.disabled);
    for handler in &self.on_click {
      root = root.on_click(handler.clone());
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
    if let Some(name) = self.icon {
      root = root.child(glyph(&self.part(LinkPart::Icon), name, false));
    }
    root.into()
  }
}

impl Themed for Link {
  const NAME: &'static str = "link";
  type Renderer = LinkRenderer;

  /// The design's link (`pencil/lurq-design-system.pen`, Link): text at
  /// 14/500 in the foreground, an icon of 14 four after it, and a line under
  /// both while the pointer is on it.
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
      .hovered(Root, |s| s.underline(Border::inside(1.0, Foreground)))
      .focus_visible(Root, |s| s.focus_ring())
      .state(LinkState::Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
  }
}
