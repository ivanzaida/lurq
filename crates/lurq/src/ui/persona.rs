//! A persona: a person as an avatar with a name and a line under it.

use std::sync::Arc;

use super::{
  avatar::{Avatar, AvatarPart, AvatarPresence, AvatarSize},
  names,
  part_style::Length,
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::one_line,
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{PaletteColor, Theme, TypographyStyle},
  components::{Column, Row},
  layout::{Alignment, text_style::FontWeight},
  node::Element,
};

/// The sizes of a persona: its avatar's, with the text beside it to match.
pub type PersonaSize = AvatarSize;

/// The parts of a persona a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PersonaPart {
  /// The avatar and the text, side by side.
  Root,
  /// What the persona says of the face of its avatar, laid over the theme's recipe for an avatar's face.
  Avatar,
  /// The name over the description.
  Text,
  /// Who it is.
  Name,
  /// The line under the name: where the person is, what they do, when they are back.
  Description,
}

names!(PersonaPart {
  Root => "root",
  Avatar => "avatar",
  Text => "text",
  Name => "name",
  Description => "description",
});

/// The states of a persona a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PersonaState {
  /// It has a line under its name.
  Description,
}

names!(PersonaState {
  Description => "description",
});

/// What builds a persona's nodes: a theme's in place of the default one.
pub type PersonaRenderer = Arc<dyn Fn(PersonaView) -> Element + Send + Sync>;

/// A persona: a person in a list, a menu or a header. Their avatar, with the
/// dot that says whether they are there, their name and a line under it.
///
/// ```
/// use lurq::ui::{Avatar, AvatarPresence, AvatarShape, Persona, PersonaSize, icons};
///
/// let ivan = Persona::new("Ivan Zaida")
///   .description("Available · Kyiv")
///   .presence(AvatarPresence::Online);
/// let ivy = Persona::new("Ivy Chen")
///   .description("Away · back at 15:00")
///   .presence(AvatarPresence::Away)
///   .size(PersonaSize::Lg);
/// // With an avatar of the app's own: a picture, an icon, a shape.
/// let bot = Persona::new("Build bot")
///   .description("Runs on every push")
///   .avatar(Avatar::new("Build bot").icon(icons::TERMINAL).shape(AvatarShape::Square));
/// ```
///
/// The avatar is the name's unless [`avatar`](Self::avatar) gives another;
/// the persona gives it its size and its presence. A name or a description
/// longer than the room there is ends in an ellipsis, each on its one line,
/// and the avatar keeps its size. A persona is read, not operated: to tooling
/// that reads the tree it is a group called by the name and described by the
/// description, with its avatar in it.
pub struct Persona {
  name: Arc<str>,
  description: Option<Arc<str>>,
  avatar: Option<Avatar>,
  presence: Option<AvatarPresence>,
  presence_label: Option<Arc<str>>,
  size: Name,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Persona {
  /// The persona of `name`: the avatar of the name, and the name beside it.
  pub fn new(name: &str) -> Self {
    Self {
      name: Arc::from(name),
      description: None,
      avatar: None,
      presence: None,
      presence_label: None,
      size: PersonaSize::default().into(),
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// The line under the name, in the muted color: "Available · Kyiv".
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// Whether the person is there: the dot at the corner of the avatar. `None` leaves the avatar as it was given.
  pub fn presence(mut self, presence: impl Into<Option<AvatarPresence>>) -> Self {
    self.presence = presence.into();
    self
  }

  /// What the dot says to tooling, in place of the presence's own word: see [`Avatar::presence_label`].
  pub fn presence_label(mut self, label: &str) -> Self {
    self.presence_label = Some(Arc::from(label));
    self
  }

  /// An avatar in place of the one the name gives: with a picture, an icon or a shape of its own. The persona
  /// gives it its size, and its presence where it has one.
  pub fn avatar(mut self, avatar: Avatar) -> Self {
    self.avatar = Some(avatar);
    self
  }

  /// A [`PersonaSize`], or a size the theme's recipe names: the avatar's, 32 unless told, and the text to match.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// Puts the persona in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this persona over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this persona: what the component it is in says of it in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the persona's root.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the persona in `theme`. Converting a persona into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: Some(self.size),
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(PersonaState::Description, self.description.is_some());
    let recipe = theme
      .resolve::<Persona>(&case, Some(&self.rules))
      .owned_by(self.id.as_ref());
    let mut avatar = self
      .avatar
      .unwrap_or_else(|| Avatar::new(&self.name))
      .size(self.size)
      .rules(&recipe.rules_for(PersonaPart::Avatar, AvatarPart::Face));
    if let Some(presence) = self.presence {
      avatar = avatar.presence(presence);
    }
    if let Some(label) = &self.presence_label {
      avatar = avatar.presence_label(label);
    }
    let view = PersonaView {
      name: self.name,
      description: self.description,
      avatar: Some(avatar),
      presence: self.presence,
      size: self.size,
      recipe,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Persona>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Persona> for Element {
  fn from(persona: Persona) -> Self {
    persona.build(&current_theme())
  }
}

/// A persona as a renderer sees it: what it was given and the style of its parts.
pub struct PersonaView {
  /// Who it is.
  pub name: Arc<str>,
  /// The line under the name.
  pub description: Option<Arc<str>>,
  /// The avatar, with the persona's size and presence, until a renderer takes it.
  pub avatar: Option<Avatar>,
  pub presence: Option<AvatarPresence>,
  pub size: Name,
  /// The style of each part of this persona.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl PersonaView {
  /// The style of a part, as this persona is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The root in the style of its part, with nothing in it yet: a group to tooling, called by the name and
  /// described by the description. It clips nothing: the dot's ring reaches past the avatar.
  pub fn root(&self) -> Row {
    let mut root = Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(PersonaPart::Root))
      .overflow_visible()
      .describe("a11y_role", "group")
      .describe("a11y_name", self.name.clone());
    if let Some(description) = &self.description {
      root = root.describe("a11y_description", description.clone());
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// The name over the description, each on one line that ends in an ellipsis where the room is too narrow.
  /// The column gives way before the avatar does.
  pub fn text(&self) -> Column {
    let mut text = Column::new()
      .flex_shrink(1.0)
      .min_width(0.0)
      .part(&self.part(PersonaPart::Text))
      .child(one_line(&self.part(PersonaPart::Name), &self.name));
    if let Some(description) = &self.description {
      text = text.child(one_line(&self.part(PersonaPart::Description), description));
    }
    text
  }

  /// How the default renderer builds a persona: the avatar, which keeps its size, and the text beside it.
  pub fn render(mut self) -> Element {
    let mut root = self.root();
    if let Some(avatar) = self.avatar.take() {
      root = root.child(
        Row::new()
          .flex_shrink(0.0)
          .overflow_visible()
          .child(avatar.build(self.theme())),
      );
    }
    root.child(self.text()).into()
  }
}

impl crate::ui::Styled for Persona {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for Persona {
  const NAME: &'static str = "persona";
  type Renderer = PersonaRenderer;

  fn parts() -> crate::ui::Declared {
    PersonaPart::declared()
  }

  fn states() -> crate::ui::Declared {
    PersonaState::declared()
  }

  /// The design's persona (`pencil/lurq-design-system.pen`, Persona): an
  /// avatar of 32 with its dot, 10 before a name in 14 at 500 in the
  /// foreground and, 1 under it, a line in 12 of the muted foreground. The
  /// board draws the one size; the others follow the avatar's: 20 and 24 with
  /// a name of 12 and 13 over a line of 11, 6 and 8 from the avatar; 40 and 56
  /// with a name of 15 and 18 over a line of 13 and 14, 12 and 14 from it. It
  /// is no wider than the room it has.
  fn recipe() -> Recipe {
    use PaletteColor::{Foreground, MutedForeground};
    use PersonaPart::{Description, Name, Root, Text};
    let recipe = Recipe::new()
      .base(Root, |s| s.gap(10.0).max_width(Length::Fill).foreground(Foreground))
      .base(Text, |s| s.gap(1.0))
      .base(Name, |s| {
        s.typography(TypographyStyle::Label)
          .font_size(14.0)
          .weight(FontWeight::Medium)
          .foreground(Foreground)
      })
      .base(Description, |s| {
        s.typography(TypographyStyle::Caption)
          .font_size(12.0)
          .foreground(MutedForeground)
      });
    // The room between the avatar and the text, the name and the line under it, and the room between those two.
    [
      (PersonaSize::Xs, 6.0, 12.0, 11.0, 0.0),
      (PersonaSize::Sm, 8.0, 13.0, 11.0, 0.0),
      (PersonaSize::Lg, 12.0, 15.0, 13.0, 1.0),
      (PersonaSize::Xl, 14.0, 18.0, 14.0, 2.0),
    ]
    .into_iter()
    .fold(recipe, |recipe, (size, gap, name, description, between)| {
      recipe
        .size(size, Root, |s| s.gap(gap))
        .size(size, Text, |s| s.gap(between))
        .size(size, Name, |s| s.font_size(name))
        .size(size, Description, |s| s.font_size(description))
    })
  }
}
