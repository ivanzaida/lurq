//! An outcome: what a page or a dialog says when something is over. The design calls it a result; it is not
//! named so here, so that a glob import of `lurq::ui` does not hide `std::result::Result`.

use std::sync::Arc;

use super::{
  empty_state::centred,
  icons, names,
  part_style::{Length, PartStyle},
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::glyph,
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{BorderSize, PaletteColor, RadiusSize, Theme, TypographyStyle},
  components::{Column, Row},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, border::Border as Line, dimension::Dimension},
};

/// What came of it, which its mark and its colors say.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum OutcomeVariant {
  /// It went well: a check in the success color.
  #[default]
  Success,
  /// It failed: a cross in the destructive color.
  Error,
  /// It went, with something to look at: a warning in its color.
  Warning,
  /// It is under way, or there is something to know: an `i` in the info color.
  Info,
  /// What was asked for is not there, a 404: a crossed magnifier in the foreground on the muted fill.
  NotFound,
}

names!(OutcomeVariant {
  Success => "success",
  Error => "error",
  Warning => "warning",
  Info => "info",
  NotFound => "not_found",
});

impl OutcomeVariant {
  /// The icon of the theme's set the variant is marked with.
  pub fn icon(self) -> &'static str {
    match self {
      Self::Success => icons::CHECK,
      Self::Error => icons::CLOSE,
      Self::Warning => icons::WARNING,
      Self::Info => icons::INFO,
      Self::NotFound => icons::SEARCH_CLOSE,
    }
  }
}

/// The parts of an outcome a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OutcomePart {
  /// What holds the rest, centred: its fill, its edge, its padding.
  Root,
  /// The round mark over the title, in the variant's muted color.
  Mark,
  /// The icon in the mark, in the variant's color.
  Icon,
  /// The title and the description, one over the other.
  Text,
  /// What came of it, in a line: "Payment received".
  Title,
  /// More of it, under the title in the muted color.
  Description,
  /// What the app puts under the text: a summary, a list, a code.
  Extra,
  /// The row of buttons at the bottom.
  Actions,
}

names!(OutcomePart {
  Root => "root",
  Mark => "mark",
  Icon => "icon",
  Text => "text",
  Title => "title",
  Description => "description",
  Extra => "extra",
  Actions => "actions",
});

/// The states of an outcome a recipe styles. It has nothing the pointer or the keyboard does to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OutcomeState {
  /// It is a box of its own, with a fill and an edge.
  Bordered,
}

names!(OutcomeState {
  Bordered => "bordered",
});

/// What builds an outcome's nodes: a theme's in place of the default one.
pub type OutcomeRenderer = Arc<dyn Fn(OutcomeView) -> Element + Send + Sync>;

/// An outcome: what a page or a dialog says when something is over. The design's Result: payment received, build
/// failed, nothing here.
///
/// ```
/// use lurq::ui::{Button, ButtonVariant, Outcome, OutcomeVariant};
///
/// let built = Outcome::new(OutcomeVariant::Success, "Build succeeded")
///   .description("412 tests passed in 38 seconds.")
///   .action(Button::new("View log").variant(ButtonVariant::Outline))
///   .action(Button::new("Deploy"));
/// // On a page of its own, with no box.
/// let missing = Outcome::new(OutcomeVariant::NotFound, "Nothing here")
///   .description("This page was moved or never existed.")
///   .bordered(false)
///   .action(Button::new("Home"));
/// ```
///
/// It is a box of 300 with a fill and an edge, as the design draws it, everything in it centred; one with no box
/// ([`bordered`](Self::bordered)) stands on a page or in a dialog. Its title and description wrap in the width it
/// has. It is read, and its buttons are operated as themselves: to what reads the tree it is a `status`, called by
/// its title and described by its description.
pub struct Outcome {
  variant: OutcomeVariant,
  title: Arc<str>,
  description: Option<Arc<str>>,
  icon: Option<Name>,
  media: Option<Element>,
  extra: Option<Element>,
  actions: Vec<Element>,
  bordered: bool,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Outcome {
  /// An outcome of `variant` that says `title`.
  pub fn new(variant: OutcomeVariant, title: &str) -> Self {
    Self {
      variant,
      title: Arc::from(title),
      description: None,
      icon: None,
      media: None,
      extra: None,
      actions: Vec::new(),
      bordered: true,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// More of it, under the title.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// Another icon of the theme's set in the mark, in place of the variant's: still in the variant's colors.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// An element of the app's in place of the mark: an illustration, a picture.
  pub fn media(mut self, media: impl Into<Element>) -> Self {
    self.media = Some(media.into());
    self
  }

  /// What stands under the text, over the buttons: a summary, a list, a code. It is as wide as the outcome.
  pub fn extra(mut self, extra: impl Into<Element>) -> Self {
    self.extra = Some(extra.into());
    self
  }

  /// Adds a button at the bottom, after those there: the least first, the main one last.
  pub fn action(mut self, action: impl Into<Element>) -> Self {
    self.actions.push(action.into());
    self
  }

  /// Whether it is a box of its own, with a fill and an edge: it is unless told. One with none stands on a page or
  /// in a dialog.
  pub fn bordered(mut self, bordered: bool) -> Self {
    self.bordered = bordered;
    self
  }

  /// A width of its own in place of the design's 300: [`Length::Fill`] for all of the room it is given.
  pub fn width(mut self, width: impl Into<Length>) -> Self {
    let width = width.into();
    self.rules = self.rules.base(OutcomePart::Root, |s| s.width(width));
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

  /// Lays rules over the theme's recipe for this outcome.
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

  /// Builds the outcome in `theme`. Converting it into an element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: Some(self.variant.into()),
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(OutcomeState::Bordered, self.bordered);
    let view = OutcomeView {
      variant: self.variant,
      title: self.title,
      description: self.description,
      icon: self.icon.unwrap_or_else(|| Name::new(self.variant.icon())),
      media: self.media,
      extra: self.extra,
      actions: self.actions,
      bordered: self.bordered,
      recipe: theme.resolve::<Outcome>(&case, Some(&self.rules)),
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Outcome>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Outcome> for Element {
  fn from(outcome: Outcome) -> Self {
    outcome.build(&current_theme())
  }
}

/// An outcome as a renderer sees it: what it was given, and the style of its parts.
pub struct OutcomeView {
  pub variant: OutcomeVariant,
  pub title: Arc<str>,
  pub description: Option<Arc<str>>,
  /// The icon in its mark: the variant's, or the app's.
  pub icon: Name,
  /// The app's element in place of the mark.
  pub media: Option<Element>,
  /// What the app put under the text.
  pub extra: Option<Element>,
  /// The app's buttons.
  pub actions: Vec<Element>,
  pub bordered: bool,
  /// The style of each part.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl OutcomeView {
  /// The style of a part, as the outcome is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The root in its style, with nothing in it yet: what it holds centred in it, a `status` to what reads the
  /// tree, called by the title and described by the description.
  pub fn root(&self) -> Column {
    let mut root = Column::new()
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .part(&self.part(OutcomePart::Root))
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

  /// The app's element, or the icon in its round mark.
  pub fn mark(&mut self) -> Element {
    if let Some(media) = self.media.take() {
      return media;
    }
    Row::new()
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .part(&self.part(OutcomePart::Mark))
      .child(glyph(&self.part(OutcomePart::Icon), self.icon, false))
      .into()
  }

  /// The title over the description, each centred and wrapping in the width it has.
  pub fn text(&self) -> Column {
    let mut text = Column::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .part(&self.part(OutcomePart::Text))
      .child(centred(&self.part(OutcomePart::Title), &self.title));
    if let Some(description) = &self.description {
      text = text.child(centred(&self.part(OutcomePart::Description), description));
    }
    text
  }

  /// What the app put under the text, as wide as the outcome: `None` for an outcome without it.
  pub fn extra(&mut self) -> Option<Column> {
    let extra = self.extra.take()?;
    Some(
      Column::new()
        .width(Dimension::Pct(100.0))
        .part(&self.part(OutcomePart::Extra))
        .child(extra),
    )
  }

  /// The row of the app's buttons: `None` for an outcome with none.
  pub fn actions(&mut self) -> Option<Row> {
    if self.actions.is_empty() {
      return None;
    }
    Some(
      Row::new()
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .part(&self.part(OutcomePart::Actions))
        .with_children(std::mem::take(&mut self.actions)),
    )
  }

  /// How the default renderer builds an outcome: the mark over the text, what the app put under it and the
  /// buttons, centred.
  pub fn render(mut self) -> Element {
    let mut root = self.root().child(self.mark()).child(self.text());
    if let Some(extra) = self.extra() {
      root = root.child(extra);
    }
    if let Some(actions) = self.actions() {
      root = root.child(actions);
    }
    root.into()
  }
}

impl Themed for Outcome {
  const NAME: &'static str = "outcome";
  type Renderer = OutcomeRenderer;

  /// The design's result (`pencil/lurq-design-system.pen`: Result, and its states error, warning, info and not
  /// found): 300 wide, padded 24 with 16 between the mark, the text and the buttons, on `Background` with an edge of
  /// `Border` and the radius `Lg`. The mark is a circle of 48 in the variant's muted color, its icon 24 in the
  /// variant's color: `Success`, `Destructive`, `Warning`, `Info`, and `Foreground` on `Muted` for not found. The
  /// title is 16/600 in `Foreground` with the description 4 under it in 13 of `MutedForeground`; the buttons are 8
  /// apart.
  fn recipe() -> Recipe {
    use OutcomePart::{Actions, Description, Extra, Icon, Mark, Root, Text, Title};
    use OutcomeVariant::{Error, Info, NotFound, Success, Warning};
    use PaletteColor::*;
    let mut recipe = Recipe::new()
      .base(Root, |s| s.width(300.0).gap(16.0))
      .base(Mark, |s| s.square(48.0).radius(RadiusSize::Full).background(Muted))
      .base(Icon, |s| s.square(24.0).foreground(Foreground))
      .base(Text, |s| s.gap(4.0))
      .base(Title, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(16.0)
          .weight(FontWeight::SemiBold)
          .foreground(Foreground)
      })
      .base(Description, |s| {
        s.typography(TypographyStyle::Description)
          .font_size(13.0)
          .foreground(MutedForeground)
      })
      .base(Extra, |s| s.gap(8.0))
      .base(Actions, |s| s.gap(8.0))
      .state(OutcomeState::Bordered, Root, |s| {
        s.padding_x(24.0)
          .padding_y(24.0)
          .radius(RadiusSize::Lg)
          .background(Background)
          .border(Line::inside(BorderSize::Sm, PaletteColor::Border))
      });
    for (variant, strong, muted) in [
      (Success, PaletteColor::Success, SuccessMuted),
      (Error, Destructive, DestructiveMuted),
      (Warning, PaletteColor::Warning, WarningMuted),
      (Info, PaletteColor::Info, InfoMuted),
      (NotFound, Foreground, Muted),
    ] {
      recipe = recipe
        .variant(variant, Mark, |s| s.background(muted))
        .variant(variant, Icon, |s| s.foreground(strong));
    }
    recipe
  }
}
