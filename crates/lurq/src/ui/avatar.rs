//! An avatar: who a person is, at a glance; and a group of them laid over one another.

use std::sync::Arc;

use super::{
  icons, names,
  part_style::Length,
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::glyph,
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{PaletteColor, RadiusSize, Theme, TypographyStyle},
  components::{Row, Stack},
  layout::{Alignment, layout_kind::Justify, stack_alignment::StackAlignment, text_style::FontWeight},
  node::{Element, border::Border},
};

/// The sizes of an avatar: the side of its square, from 20 to 56.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AvatarSize {
  /// 20: beside a line of small text.
  Xs,
  /// 24: in a list of suggestions, a header.
  Sm,
  /// 32: in a row of a list, a bar.
  #[default]
  Md,
  /// 40: an account in a drawer, a notification.
  Lg,
  /// 56: a profile.
  Xl,
}

names!(AvatarSize {
  Xs => "xs",
  Sm => "sm",
  Md => "md",
  Lg => "lg",
  Xl => "xl",
});

/// The shape of an avatar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AvatarShape {
  /// A circle: a person.
  #[default]
  Circle,
  /// A square with the corners of a control: a team, a workspace, a bot.
  Square,
}

/// Whether the person an avatar shows is there: the dot at its corner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AvatarPresence {
  /// There, and free to answer.
  Online,
  /// There, but away from the keyboard.
  Away,
  /// There, and not to be disturbed.
  Busy,
  /// Not there.
  Offline,
}

names!(AvatarPresence {
  Online => "online",
  Away => "away",
  Busy => "busy",
  Offline => "offline",
});

/// The parts of an avatar a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AvatarPart {
  /// The avatar with its dot of presence: as large as its face, which the dot stands over.
  Root,
  /// The shape: its side, its corners, its fill, and the ring it has in a group. What it holds is clipped to it.
  Face,
  /// The letters of the name, on the face's fill.
  Initials,
  /// The icon in place of the letters: for what is not a person, or a name that has none.
  Icon,
  /// The picture, which fills the face.
  Image,
  /// The dot at the bottom end of the face that says whether the person is there.
  Presence,
}

names!(AvatarPart {
  Root => "root",
  Face => "face",
  Initials => "initials",
  Icon => "icon",
  Image => "image",
  Presence => "presence",
});

/// The states of an avatar a recipe styles. The [`AvatarPresence`] it has is
/// one too, under its own name: `.state(AvatarPresence::Away, AvatarPart::Presence, ..)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AvatarState {
  /// Its shape is [`AvatarShape::Square`].
  Square,
}

names!(AvatarState {
  Square => "square",
});

/// What builds an avatar's nodes: a theme's in place of the default one.
pub type AvatarRenderer = Arc<dyn Fn(AvatarView) -> Element + Send + Sync>;

/// The letters an avatar shows for `name`: the first of its first word and the first of its last, in capitals.
/// "Ivan Zaida" is "IZ", "Ivan" is "I", and a name with no letter or digit has none.
fn initials(name: &str) -> String {
  let first = |word: &str| word.chars().find(|char| char.is_alphanumeric());
  let words: Vec<&str> = name.split_whitespace().filter(|word| first(word).is_some()).collect();
  let picked = match words.as_slice() {
    [] => Vec::new(),
    [one] => vec![*one],
    [first_word, .., last_word] => vec![*first_word, *last_word],
  };
  picked
    .into_iter()
    .filter_map(first)
    .flat_map(char::to_uppercase)
    .collect()
}

/// An avatar: who a person is, at a glance. The initials of the name on a
/// fill, a picture in their place, or an icon for what is not a person.
///
/// ```
/// use lurq::ui::{Avatar, AvatarPresence, AvatarShape, AvatarSize, icons};
///
/// let ivan = Avatar::new("Ivan Zaida").presence(AvatarPresence::Online);
/// let profile = Avatar::new("Ivy Chen").size(AvatarSize::Xl);
/// let bot = Avatar::new("Build bot").icon(icons::TERMINAL).shape(AvatarShape::Square);
/// ```
///
/// With [`image`](Self::image) it shows a picture, clipped to its shape; the
/// initials stay in its place while the picture loads and when it fails. An
/// avatar is read, not operated: to tooling that reads the tree it is an image
/// called by the name.
pub struct Avatar {
  name: Arc<str>,
  /// What the face shows in place of the initials: the count at the end of a group.
  text: Option<Arc<str>>,
  icon: Option<Name>,
  #[cfg(feature = "raster")]
  image: Option<crate::components::Image>,
  size: Name,
  shape: AvatarShape,
  presence: Option<AvatarPresence>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Avatar {
  /// The avatar of `name`: its initials on a fill. A name with no letters shows the icon of a person.
  pub fn new(name: &str) -> Self {
    Self {
      name: Arc::from(name),
      text: None,
      icon: None,
      #[cfg(feature = "raster")]
      image: None,
      size: AvatarSize::default().into(),
      shape: AvatarShape::default(),
      presence: None,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// The face of a group's count: "+3", called "3 more".
  pub(crate) fn count(hidden: usize) -> Self {
    let mut avatar = Self::new(&format!("{hidden} more"));
    avatar.text = Some(Arc::from(format!("+{hidden}")));
    avatar
  }

  /// A picture of the person in place of the initials, which fills the
  /// avatar and is clipped to its shape: the engine's image element as the
  /// app builds it, from pixels it has (`Image::new`) or from a file or bytes
  /// it decodes (`Image::from_file`, `Image::from_bytes`, with feature
  /// `image`), with a cache key and a size to decode to where it wants them.
  /// While a picture decodes, and when it fails, the avatar shows its initials
  /// or its icon: they are the image's placeholder and its error view.
  ///
  /// ```
  /// use lurq::{components::Image, images::ImageData, ui::Avatar};
  ///
  /// let pixels = ImageData::from_rgba(vec![200; 4 * 4 * 4], 4, 4);
  /// let ivan = Avatar::new("Ivan Zaida").image(Image::new(pixels));
  /// ```
  #[cfg(feature = "raster")]
  pub fn image(mut self, image: crate::components::Image) -> Self {
    self.image = Some(image);
    self
  }

  /// An icon in place of the initials, by its name in the theme: for what is not a person, a bot or a team.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// An [`AvatarSize`], or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  pub fn shape(mut self, shape: AvatarShape) -> Self {
    self.shape = shape;
    self
  }

  /// The dot at its corner that says whether the person is there; `None` for no dot.
  pub fn presence(mut self, presence: impl Into<Option<AvatarPresence>>) -> Self {
    self.presence = presence.into();
    self
  }

  /// Puts the avatar in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this avatar over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this avatar: what the component
  /// it is in says of it in this place, as a group gives its avatars a ring.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the avatar's root.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the avatar in `theme`. Converting an avatar into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let mut case = Case {
      variant: None,
      size: Some(self.size),
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(AvatarState::Square, self.shape == AvatarShape::Square);
    if let Some(presence) = self.presence {
      case = case.state(presence, true);
    }
    let view = AvatarView {
      initials: self.text.as_deref().map_or_else(|| initials(&self.name), str::to_owned),
      name: self.name,
      icon: self.icon,
      #[cfg(feature = "raster")]
      image: self.image,
      size: self.size,
      shape: self.shape,
      presence: self.presence,
      recipe: theme.resolve::<Avatar>(&case, Some(&self.rules)),
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Avatar>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Avatar> for Element {
  fn from(avatar: Avatar) -> Self {
    avatar.build(&current_theme())
  }
}

/// An avatar as a renderer sees it: what it was given and the style of its parts.
pub struct AvatarView {
  /// Who it is: what tooling that reads the tree calls it.
  pub name: Arc<str>,
  /// The letters it shows: the initials of the name, or the count at the end of a group.
  pub initials: String,
  pub icon: Option<Name>,
  /// The picture it shows in place of its initials.
  #[cfg(feature = "raster")]
  pub image: Option<crate::components::Image>,
  pub size: Name,
  pub shape: AvatarShape,
  pub presence: Option<AvatarPresence>,
  /// The style of each part of this avatar.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl AvatarView {
  /// The style of a part, as this avatar is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// What tooling that reads the tree calls the avatar: its name, and whether the person is there.
  pub fn label(&self) -> String {
    match self.presence {
      Some(presence) => format!("{}, {}", self.name, presence.as_str()),
      None => self.name.to_string(),
    }
  }

  /// What the face shows where it has no picture, and while its picture
  /// loads or once it failed: the icon it was given, its initials, or the
  /// icon of a person for a name with no letters.
  pub fn fallback(&self) -> Element {
    self.fallback_maker()()
  }

  /// What builds [`fallback`](Self::fallback), again for each time it is shown: an element is built for one place.
  fn fallback_maker(&self) -> impl Fn() -> Element + Send + Sync + 'static {
    let icon = match (&self.icon, self.initials.is_empty()) {
      (Some(icon), _) => Some(*icon),
      (None, true) => Some(Name::from(icons::USER)),
      (None, false) => None,
    };
    let (icon_part, initials_part) = (self.part(AvatarPart::Icon), self.part(AvatarPart::Initials));
    let initials = self.initials.clone();
    move || match icon {
      Some(icon) => glyph(&icon_part, icon, false),
      None => Row::new()
        .part(&initials_part)
        .child(initials_part.text(&initials).nowrap())
        .into(),
    }
  }

  /// The face: the shape in its fill, with what it shows centered in it and clipped to it.
  pub fn face(&self) -> Row {
    Row::new()
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .part(&self.part(AvatarPart::Face))
      .clip()
  }

  /// The picture in the style of its part, with the fallback in its place while it loads and once it failed.
  #[cfg(feature = "raster")]
  fn picture(&self, image: crate::components::Image) -> Element {
    let image = image.cover().part(&self.part(AvatarPart::Image));
    #[cfg(feature = "image")]
    let image = {
      let fallback = self.fallback_maker();
      image.placeholder(fallback()).error_view(move |_| fallback())
    };
    image.into()
  }

  /// The dot of presence in the style of its part, or nothing where the avatar has none.
  pub fn presence_dot(&self) -> Option<Row> {
    self.presence.map(|_| Row::new().part(&self.part(AvatarPart::Presence)))
  }

  /// How the default renderer builds an avatar: the face with its picture or
  /// its fallback, and the dot of presence over its bottom end corner.
  pub fn render(self) -> Element {
    #[cfg(feature = "raster")]
    let mut this = self;
    #[cfg(not(feature = "raster"))]
    let this = self;
    let face = this.face();
    #[cfg(feature = "raster")]
    let face = match this.image.take() {
      Some(image) => face.child(this.picture(image)),
      None => face.child(this.fallback()),
    };
    #[cfg(not(feature = "raster"))]
    let face = face.child(this.fallback());
    // The dot and the ring of a group stand outside the face: nothing of the avatar clips them.
    let mut root = Stack::new()
      .stack_align(StackAlignment::BottomEnd)
      .part(&this.part(AvatarPart::Root))
      .overflow_visible()
      .describe("a11y_role", "img")
      .describe("a11y_name", this.label())
      .child(face);
    if let Some(dot) = this.presence_dot() {
      root = root.child(dot);
    }
    if let Some(id) = this.id {
      root = root.id(id);
    }
    if let Some(key) = this.key {
      root = root.key(key);
    }
    root.into()
  }
}

impl Themed for Avatar {
  const NAME: &'static str = "avatar";
  type Renderer = AvatarRenderer;

  /// The design's avatar (`pencil/lurq-design-system.pen`, Avatar, Presence and
  /// Avatar sizes): a circle of 32 in the muted fill with the initials in the
  /// foreground, 12 and medium; 20, 24, 40 and 56 with initials of 9, 10, 14
  /// and 18; a square one with the corners of a control; the icon of a person,
  /// half the side, in the muted foreground where there are no initials. The
  /// dot of presence is 10 at the bottom end of a 32, ringed 2 in the
  /// background's color, in the success color while the person is online.
  fn recipe() -> Recipe {
    use AvatarPart::{Face, Icon, Image, Initials, Presence};
    use PaletteColor::{Background, Destructive, Foreground, Muted, MutedForeground, Success, Warning};
    let recipe = Recipe::new()
      .base(Face, |s| {
        s.square(32.0)
          .radius(RadiusSize::Full)
          .background(Muted)
          .foreground(Foreground)
      })
      .base(Initials, |s| {
        s.typography(TypographyStyle::Label)
          .font_size(12.0)
          .weight(FontWeight::Medium)
          .line_height(1.0)
          .foreground(Foreground)
      })
      .base(Icon, |s| s.width(16.0).foreground(MutedForeground))
      .base(Image, |s| s.square(Length::Fill).radius(RadiusSize::Full))
      .base(Presence, |s| {
        s.square(10.0)
          .radius(RadiusSize::Full)
          .border(Border::outside(2.0, Background))
          .background(Success)
      })
      .state(AvatarState::Square, Face, |s| s.radius(RadiusSize::Md))
      .state(AvatarState::Square, Image, |s| s.radius(RadiusSize::Md))
      .state(AvatarPresence::Online, Presence, |s| s.background(Success))
      .state(AvatarPresence::Away, Presence, |s| s.background(Warning))
      .state(AvatarPresence::Busy, Presence, |s| s.background(Destructive))
      .state(AvatarPresence::Offline, Presence, |s| s.background(MutedForeground));
    // The side, the letters, the icon at half the side, and a dot that keeps to the corner of the face.
    [
      (AvatarSize::Xs, 20.0, 9.0, 6.0),
      (AvatarSize::Sm, 24.0, 10.0, 8.0),
      (AvatarSize::Lg, 40.0, 14.0, 12.0),
      (AvatarSize::Xl, 56.0, 18.0, 14.0),
    ]
    .into_iter()
    .fold(recipe, |recipe, (size, side, letters, dot)| {
      recipe
        .size(size, Face, |s| s.square(side))
        .size(size, Initials, |s| s.font_size(letters))
        .size(size, Icon, |s| s.width(side / 2.0))
        .size(size, Presence, |s| s.square(dot))
    })
  }
}

/// The parts of a group of avatars a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AvatarGroupPart {
  /// The row of avatars: how far each lies over the one before it, as a gap below nothing.
  Root,
  /// What the group says of the face of each avatar in it: laid over the theme's recipe for an avatar's face.
  Item,
  /// What the group says of the face of the count at its end, over what it says of an item.
  More,
}

names!(AvatarGroupPart {
  Root => "root",
  Item => "item",
  More => "more",
});

/// What builds a group of avatars' nodes: a theme's in place of the default one.
pub type AvatarGroupRenderer = Arc<dyn Fn(AvatarGroupView) -> Element + Send + Sync>;

/// Avatars laid over one another: who is in a document, who reviews a change.
///
/// ```
/// use lurq::ui::{Avatar, AvatarGroup};
///
/// let reviewers = AvatarGroup::new(
///   ["Ivan Zaida", "Anna K.", "Mark R.", "Ivy Chen", "Ivo Kovac", "Lena Q."].map(Avatar::new),
/// )
/// .max(3)
/// .label("Reviewers");
/// ```
///
/// Each avatar lies over the end of the one before it, with a ring of the
/// surface behind the group around it. With [`max`](Self::max) the group
/// shows that many and then how many it leaves out, "+3". The group gives its
/// avatars its size; each keeps everything else it was given. To tooling that
/// reads the tree it is a group called by its label.
pub struct AvatarGroup {
  avatars: Vec<Avatar>,
  max: Option<usize>,
  size: Name,
  label: Option<Arc<str>>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl AvatarGroup {
  pub fn new(avatars: impl IntoIterator<Item = Avatar>) -> Self {
    Self {
      avatars: avatars.into_iter().collect(),
      max: None,
      size: AvatarSize::default().into(),
      label: None,
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// The most avatars the group shows; after them it shows how many it leaves out.
  pub fn max(mut self, max: usize) -> Self {
    self.max = Some(max);
    self
  }

  /// The size of every avatar in the group: an [`AvatarSize`], or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// What the group is called to tooling that reads the tree: "Reviewers". Without one it is called by the
  /// names of the avatars it shows and how many it leaves out.
  pub fn label(mut self, label: impl Into<Arc<str>>) -> Self {
    self.label = Some(label.into());
    self
  }

  /// Styles one part of this group over what the theme gives it: on a card, the ring is the card's color.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
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

  /// Builds the group in `theme`. Converting a group into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case::new().size(self.size).breakpoint(super::current_breakpoint());
    let recipe = theme.resolve::<AvatarGroup>(&case, Some(&self.rules));
    let shown = self.max.unwrap_or(usize::MAX).min(self.avatars.len());
    let hidden = self.avatars.len() - shown;
    let label = self.label.map_or_else(
      || {
        let names: Vec<&str> = self.avatars[..shown].iter().map(|avatar| &*avatar.name).collect();
        match hidden {
          0 => names.join(", "),
          more => format!("{} and {more} more", names.join(", ")),
        }
      },
      |label| label.to_string(),
    );
    let item = recipe.rules_for(AvatarGroupPart::Item, AvatarPart::Face);
    let avatars = self
      .avatars
      .into_iter()
      .take(shown)
      .map(|avatar| avatar.size(self.size).rules(&item))
      .collect();
    let view = AvatarGroupView {
      avatars,
      hidden,
      size: self.size,
      label,
      recipe,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<AvatarGroup>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<AvatarGroup> for Element {
  fn from(group: AvatarGroup) -> Self {
    group.build(&current_theme())
  }
}

/// A group of avatars as a renderer sees it.
pub struct AvatarGroupView {
  /// The avatars it shows, with its size and what it says of an item laid over them.
  pub avatars: Vec<Avatar>,
  /// How many it leaves out.
  pub hidden: usize,
  pub size: Name,
  /// What tooling that reads the tree calls it.
  pub label: String,
  /// The style of each part of this group.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl AvatarGroupView {
  /// The style of a part, as this group is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The count at the end of the group, "+3", in the style of an item and of its own part; `None` where the
  /// group leaves nobody out.
  pub fn more(&self) -> Option<Avatar> {
    (self.hidden > 0).then(|| {
      let rules = self
        .recipe
        .rules_for(AvatarGroupPart::Item, AvatarPart::Face)
        .extend(&self.recipe.rules_for(AvatarGroupPart::More, AvatarPart::Face));
      Avatar::count(self.hidden).size(self.size).rules(&rules)
    })
  }

  /// How the default renderer builds a group: its avatars in a row, each over the end of the one before, and the
  /// count after them.
  pub fn render(self) -> Element {
    let theme = self.theme().clone();
    let more = self.more();
    // The rings stand outside the faces: the row does not clip them.
    let mut root = Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(AvatarGroupPart::Root))
      .overflow_visible()
      .describe("a11y_role", "group")
      .describe("a11y_name", self.label);
    for avatar in self.avatars.into_iter().chain(more) {
      root = root.child(avatar.build(&theme));
    }
    if let Some(id) = self.id {
      root = root.id(id);
    }
    if let Some(key) = self.key {
      root = root.key(key);
    }
    root.into()
  }
}

impl Themed for AvatarGroup {
  const NAME: &'static str = "avatar_group";
  type Renderer = AvatarGroupRenderer;

  /// The design's group (`pencil/lurq-design-system.pen`, Avatar Group): avatars
  /// of 32 that lie 8 over one another, each ringed 2 outside its face in the
  /// background's color, and the count in the secondary fill. A smaller or a
  /// larger avatar lies over the next by a quarter of its side.
  fn recipe() -> Recipe {
    use AvatarGroupPart::{Item, More, Root};
    let recipe = Recipe::new()
      .base(Root, |s| s.gap(-8.0))
      .base(Item, |s| s.border(Border::outside(2.0, PaletteColor::Background)))
      .base(More, |s| s.background(PaletteColor::Secondary));
    [
      (AvatarSize::Xs, 20.0),
      (AvatarSize::Sm, 24.0),
      (AvatarSize::Lg, 40.0),
      (AvatarSize::Xl, 56.0),
    ]
    .into_iter()
    .fold(recipe, |recipe, (size, side)| {
      recipe.size(size, Root, |s| s.gap(-side / 4.0))
    })
  }
}

#[cfg(test)]
mod tests {
  use super::initials;

  #[test]
  fn the_initials_are_the_first_letters_of_the_first_and_the_last_word() {
    assert_eq!(initials("Ivan Zaida"), "IZ");
    assert_eq!(initials("Ivan"), "I");
    assert_eq!(initials("ivan van der berg"), "IB");
    assert_eq!(initials("  Anna   K. "), "AK");
    assert_eq!(initials("(Mark) Rossi"), "MR");
    assert_eq!(initials(""), "");
    assert_eq!(initials("— · —"), "");
  }
}
