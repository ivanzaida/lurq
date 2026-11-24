//! What opens and closes in place: rows a choice shows or hides, an item that comes into a live list, an answer
//! that folds to a line. Its height runs from nothing to its content's as it opens and back as it closes, and what
//! it holds fades in and out; closed, what it held stays, inert, until its fade is over.

use std::sync::Arc;

use super::{
  collapsible::opening,
  names,
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::Theme,
  components::Column,
  layout::Alignment,
  node::{Element, FocusGroup},
};

/// The parts of a reveal a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RevealPart {
  /// What opens and closes: of no height while closed, of its content's while open, with a transition of its height
  /// between the two in the base time; it clips what it holds on the way.
  Root,
  /// What it holds while it is open, which fades in as it opens and out as it closes.
  Body,
}

names!(RevealPart {
  Root => "root",
  Body => "body",
});

/// The states of a reveal a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RevealState {
  /// It is open: its height is its content's.
  Open,
}

names!(RevealState {
  Open => "open",
});

/// What builds the nodes of a reveal: a theme's in place of the default one.
pub type RevealRenderer = Arc<dyn Fn(RevealView) -> Element + Send + Sync>;

/// What opens and closes in place, for any element of the app's: its height runs from nothing to its content's
/// in the theme's base time as it opens, and what it holds fades in; closing, the other way, and what it held stays,
/// painted and inert, until its fade is over. Where motion is reduced its height is there at once.
///
/// ```
/// use lurq::{components::Text, ui::Reveal};
///
/// # fn rows(shown: bool) {
/// let details = Reveal::new(shown, Text::new("Opened by a choice"));
/// # }
/// ```
///
/// Its height is a transition of the node, which needs the node to stay from one render to the next: the app keeps
/// it in one place of its tree, or gives it a key.
pub struct Reveal {
  open: bool,
  content: Option<Element>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Reveal {
  /// `content`, shown where `open` is, and closed over otherwise.
  pub fn new(open: bool, content: impl Into<Element>) -> Self {
    Self {
      open,
      content: Some(content.into()),
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// Puts the reveal in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Lays rules over the theme's recipe for this reveal.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Styles one part of this reveal over what the theme gives it: its time, a padding round what it holds.
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

  /// Builds the reveal in `theme`. Converting a reveal into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = self
      .states
      .iter()
      .fold(Case::new(), |case, state| case.state(*state, true))
      .state(RevealState::Open, self.open);
    let view = RevealView {
      open: self.open,
      content: self.content,
      recipe: theme
        .resolve::<Reveal>(&case, Some(&self.rules))
        .owned_by(self.id.as_ref()),
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Reveal>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Reveal> for Element {
  fn from(reveal: Reveal) -> Self {
    reveal.build(&current_theme())
  }
}

/// A reveal as a renderer sees it.
pub struct RevealView {
  /// Whether it is open.
  pub open: bool,
  /// What it holds while it is open.
  pub content: Option<Element>,
  /// The style of each part of this reveal.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl RevealView {
  /// The style of a part, as this reveal is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  /// How the default renderer builds a reveal: what clips and opens, with what it holds while it is open, which
  /// stays as it closes, painted and inert, until its fade is over.
  pub fn render(mut self) -> Element {
    let mut root = Column::new()
      .align_items(Alignment::Stretch)
      .clip()
      .presence()
      .part(&self.part(RevealPart::Root));
    if self.open
      && let Some(content) = self.content.take()
    {
      root = root.child(
        Column::new()
          .key("content")
          .focus_group(FocusGroup::none())
          .align_items(Alignment::Stretch)
          .part(&self.part(RevealPart::Body))
          .child(content),
      );
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

impl crate::ui::Styled for Reveal {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for Reveal {
  const NAME: &'static str = "reveal";
  type Renderer = RevealRenderer;

  fn parts() -> crate::ui::Declared {
    RevealPart::declared()
  }

  fn states() -> crate::ui::Declared {
    RevealState::declared()
  }

  /// What opens and closes as a collapsible's panel does: of no height while closed and of its content's while open,
  /// over the theme's base duration; what it holds fades in as it opens and out as it closes.
  fn recipe() -> Recipe {
    opening(
      Recipe::new(),
      RevealState::Open.into(),
      RevealPart::Root.into(),
      RevealPart::Body.into(),
    )
  }
}
