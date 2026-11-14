//! An empty state: what stands where content would be when there is none, and what to do about it.

use std::sync::Arc;

use super::{
  names,
  part_style::{Length, PartStyle},
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::glyph,
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{BorderSize, PaletteColor, RadiusSize, Theme, TypographyStyle},
  components::{Column, Row},
  layout::{
    Alignment,
    layout_kind::Justify,
    text_style::{FontWeight, TextAlign},
  },
  node::{Element, border::Border as Line, color::Color, dimension::Dimension},
};

/// How large an empty state is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum EmptyStateSize {
  /// The icon alone, 22, in the muted color, and everything 10 apart: in a table, a list, a panel.
  Sm,
  /// The icon in a well of 40, in a box with an edge: where a whole view is empty.
  #[default]
  Md,
  /// The icon in a round well of 64, a title of 18: a screen of a phone.
  Lg,
}

names!(EmptyStateSize {
  Sm => "sm",
  Md => "md",
  Lg => "lg",
});

/// The parts of an empty state a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EmptyStatePart {
  /// What holds the rest, centred: its edge, its padding.
  Root,
  /// The well the icon is in.
  Media,
  /// The icon in the well.
  Icon,
  /// The title and the description, one over the other.
  Text,
  /// What is empty, in a line: "No results".
  Title,
  /// Why, or what to do, under the title in the muted color.
  Description,
  /// The row of the one or two things the app gives to do.
  Actions,
}

names!(EmptyStatePart {
  Root => "root",
  Media => "media",
  Icon => "icon",
  Text => "text",
  Title => "title",
  Description => "description",
  Actions => "actions",
});

/// The states of an empty state a recipe styles. It has nothing the pointer or the keyboard does to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EmptyStateState {
  /// It has an edge around it.
  Bordered,
}

names!(EmptyStateState {
  Bordered => "bordered",
});

/// What builds an empty state's nodes: a theme's in place of the default one.
pub type EmptyStateRenderer = Arc<dyn Fn(EmptyStateView) -> Element + Send + Sync>;

/// An empty state: what stands where content would be when there is none. No results, no windows yet, all caught
/// up.
///
/// ```
/// use lurq::ui::{Button, ButtonSize, ButtonVariant, EmptyState, EmptyStateSize, icons};
///
/// let windows = EmptyState::new("No windows yet")
///   .description("Open a window to see it listed here.")
///   .icon(icons::INBOX)
///   .action(Button::new("New window").icon(icons::PLUS).variant(ButtonVariant::Outline));
/// // In a table: the icon alone, no edge, a small button.
/// let filtered = EmptyState::new("No crates match “blurr”")
///   .description("Check the spelling or clear the filter.")
///   .icon(icons::SEARCH_CLOSE)
///   .size(EmptyStateSize::Sm)
///   .action(Button::new("Clear filter").variant(ButtonVariant::Outline).size(ButtonSize::Sm));
/// ```
///
/// It is as wide as the room it is given and as high as what is in it, all of it centred; given a height
/// ([`height`](Self::height)) it is centred in that too. Its title and description wrap in the width it has. It is
/// read, and what it holds to do is operated as itself: to what reads the tree it is a `status`, called by its title
/// and described by its description.
pub struct EmptyState {
  title: Arc<str>,
  description: Option<Arc<str>>,
  icon: Option<Name>,
  media: Option<Element>,
  actions: Vec<Element>,
  size: Name,
  bordered: Option<bool>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl EmptyState {
  /// An empty state that says `title`.
  pub fn new(title: &str) -> Self {
    Self {
      title: Arc::from(title),
      description: None,
      icon: None,
      media: None,
      actions: Vec::new(),
      size: EmptyStateSize::default().into(),
      bordered: None,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// Why it is empty, or what to do, under the title.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// An icon over the title, in its well, by its name in the theme's set.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// An element of the app's over the title in place of the icon and its well: an illustration, a picture.
  pub fn media(mut self, media: impl Into<Element>) -> Self {
    self.media = Some(media.into());
    self
  }

  /// Adds something to do under the text, after what is there: a button, a link. One or two; the main one last.
  pub fn action(mut self, action: impl Into<Element>) -> Self {
    self.actions.push(action.into());
    self
  }

  /// An [`EmptyStateSize`], or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// Whether it has an edge around it. The medium size has one unless told; the small and the large have none.
  pub fn bordered(mut self, bordered: bool) -> Self {
    self.bordered = Some(bordered);
    self
  }

  /// A height of its own, in which what it holds is centred: [`Length::Fill`] for all of the room a parent that
  /// stretches it gives.
  pub fn height(mut self, height: impl Into<Length>) -> Self {
    let height = height.into();
    self.rules = self.rules.base(EmptyStatePart::Root, |s| s.height(height));
    self
  }

  /// Puts it in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this empty state.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the root.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the empty state in `theme`. Converting it into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let bordered = self.bordered.unwrap_or(self.size == Name::from(EmptyStateSize::Md));
    let case = Case {
      variant: None,
      size: Some(self.size),
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(EmptyStateState::Bordered, bordered);
    let view = EmptyStateView {
      title: self.title,
      description: self.description,
      icon: self.icon,
      media: self.media,
      actions: self.actions,
      size: self.size,
      bordered,
      recipe: theme.resolve::<EmptyState>(&case, Some(&self.rules)),
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<EmptyState>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<EmptyState> for Element {
  fn from(empty: EmptyState) -> Self {
    empty.build(&current_theme())
  }
}

/// An empty state as a renderer sees it: what it was given, and the style of its parts.
pub struct EmptyStateView {
  pub title: Arc<str>,
  pub description: Option<Arc<str>>,
  /// The name of its icon, if it has one.
  pub icon: Option<Name>,
  /// The app's element in place of the icon.
  pub media: Option<Element>,
  /// What the app gives to do.
  pub actions: Vec<Element>,
  pub size: Name,
  pub bordered: bool,
  /// The style of each part.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl EmptyStateView {
  /// The style of a part, as the empty state is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The root in its style, with nothing in it yet: what it holds centred in it, a `status` to what reads the tree,
  /// called by the title and described by the description.
  pub fn root(&self) -> Column {
    let mut root = Column::new()
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .part(&self.part(EmptyStatePart::Root))
      .describe("a11y_role", "status")
      .describe("a11y_name", self.title.clone());
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

  /// The app's element, or the icon in its well: `None` for an empty state with neither.
  pub fn media(&mut self) -> Option<Element> {
    if let Some(media) = self.media.take() {
      return Some(media);
    }
    let name = self.icon?;
    Some(
      Row::new()
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .part(&self.part(EmptyStatePart::Media))
        .child(glyph(&self.part(EmptyStatePart::Icon), name, false))
        .into(),
    )
  }

  /// The title over the description, each centred and wrapping in the width it has.
  pub fn text(&self) -> Column {
    let mut text = Column::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .part(&self.part(EmptyStatePart::Text))
      .child(centred(&self.part(EmptyStatePart::Title), &self.title));
    if let Some(description) = &self.description {
      text = text.child(centred(&self.part(EmptyStatePart::Description), description));
    }
    text
  }

  /// The row of what the app gives to do: `None` for an empty state with nothing.
  pub fn actions(&mut self) -> Option<Row> {
    if self.actions.is_empty() {
      return None;
    }
    Some(
      Row::new()
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .part(&self.part(EmptyStatePart::Actions))
        .with_children(std::mem::take(&mut self.actions)),
    )
  }

  /// How the default renderer builds an empty state: the media over the text over the actions, centred.
  pub fn render(mut self) -> Element {
    let mut root = self.root();
    if let Some(media) = self.media() {
      root = root.child(media);
    }
    root = root.child(self.text());
    if let Some(actions) = self.actions() {
      root = root.child(actions);
    }
    root.into()
  }
}

/// A text in the style of `part`, centred and as wide as the room it has: a longer one goes on to the next line.
pub(crate) fn centred(part: &ResolvedPart, content: &str) -> Column {
  let fill = Dimension::Pct(100.0);
  Column::new()
    .width(fill)
    .part(part)
    .child(part.text(content).width(fill).text_align(TextAlign::Center))
}

impl Themed for EmptyState {
  const NAME: &'static str = "empty_state";
  type Renderer = EmptyStateRenderer;

  /// The design's empty states (`pencil/lurq-design-system.pen`: Empty State, the empty table of Table/Empty and
  /// Mobile/Empty State). The medium one is padded 32 with 16 between the well, the text and the actions, in an edge
  /// of `Border` with the radius `Xl`; the well is 40 on `Muted` with the radius `Lg`, its icon 20 in `Foreground`;
  /// the title is 14/500 in `Foreground` with the description 4 under it in 13 of `MutedForeground`. The small one is
  /// the table's: padded 32 and 16, everything 10 apart, the icon 22 in `MutedForeground` with no well. The large
  /// one is the phone's: padded 32 and 24, a round well of 64 with an icon of 28, a title of 18/600 and a
  /// description of 15, 6 apart. The actions are 8 apart.
  fn recipe() -> Recipe {
    use EmptyStatePart::{Actions, Description, Icon, Media, Root, Text, Title};
    use EmptyStateSize::{Lg, Sm};
    use PaletteColor::*;
    let clear = Color::new(0, 0, 0, 0);
    Recipe::new()
      .base(Root, |s| s.width(Length::Fill).padding_x(32.0).padding_y(32.0).gap(16.0))
      .base(Media, |s| s.square(40.0).radius(RadiusSize::Lg).background(Muted))
      .base(Icon, |s| s.square(20.0).foreground(Foreground))
      .base(Text, |s| s.gap(4.0))
      .base(Title, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(14.0)
          .weight(FontWeight::Medium)
          .foreground(Foreground)
      })
      .base(Description, |s| {
        s.typography(TypographyStyle::Description)
          .font_size(13.0)
          .foreground(MutedForeground)
      })
      .base(Actions, |s| s.gap(8.0))
      .state(EmptyStateState::Bordered, Root, |s| {
        s.radius(RadiusSize::Xl).border(Line::inside(BorderSize::Sm, PaletteColor::Border))
      })
      // The table's: the icon alone, and everything 10 apart.
      .size(Sm, Root, |s| s.padding_x(16.0).gap(10.0))
      .size(Sm, Media, |s| s.square(22.0).radius(0.0).background(clear))
      .size(Sm, Icon, |s| s.square(22.0).foreground(MutedForeground))
      .size(Sm, Text, |s| s.gap(10.0))
      // The phone's.
      .size(Lg, Root, |s| s.padding_x(24.0))
      .size(Lg, Media, |s| s.square(64.0).radius(RadiusSize::Full))
      .size(Lg, Icon, |s| s.square(28.0))
      .size(Lg, Text, |s| s.gap(6.0))
      .size(Lg, Title, |s| s.font_size(18.0).weight(FontWeight::SemiBold))
      .size(Lg, Description, |s| s.font_size(15.0))
  }
}
