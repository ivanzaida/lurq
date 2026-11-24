//! A short text that fades its new words in over its old ones, in one slot: a count, a short time, a share.

use std::sync::Arc;

use super::{
  names,
  part_style::PartAnimation,
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{FADE_IN, FADE_OUT},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{MotionDuration, MotionEasing, Theme},
  components::{Row, Stack},
  layout::StackAlignment,
  node::Element,
};

/// The parts of a fading text a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FadingTextPart {
  /// The slot the words stand in, old and new over each other while they change.
  Root,
  /// The words: their type and color, and what they play as they come and go (a fade in the fast time).
  Text,
}

names!(FadingTextPart {
  Root => "root",
  Text => "text",
});

/// What builds the nodes of a fading text: a theme's in place of the default one.
pub type FadingTextRenderer = Arc<dyn Fn(FadingTextView) -> Element + Send + Sync>;

/// A short text that changes in its slot by a fade: the new words fade in over the old ones, which fade out, in the
/// theme's fast time; on the page's first paint it is there at once. A count (*4*, *12*), a short time (*12m*), a
/// share (*41 %*) that a live view updates.
///
/// ```
/// use lurq::ui::FadingText;
///
/// # fn count(waiting: usize) {
/// let badge = FadingText::new(&waiting.to_string());
/// # }
/// ```
///
/// Its words are told apart by what they say: a text that says the same again does not fade. The slot is as wide as
/// the longer of the two while they change. It stays from one render to the next where it stays in one place of the
/// tree, or has a key.
pub struct FadingText {
  text: Arc<str>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl FadingText {
  /// The words `text`.
  pub fn new(text: &str) -> Self {
    Self {
      text: Arc::from(text),
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// Puts the text in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Lays rules over the theme's recipe for this text.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Styles one part of this text over what the theme gives it: the type and the color of its words.
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

  /// Builds the text in `theme`. Converting it into an element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = self
      .states
      .iter()
      .fold(Case::new(), |case, state| case.state(*state, true));
    let view = FadingTextView {
      text: self.text,
      recipe: theme
        .resolve::<FadingText>(&case, Some(&self.rules))
        .owned_by(self.id.as_ref()),
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<FadingText>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<FadingText> for Element {
  fn from(text: FadingText) -> Self {
    text.build(&current_theme())
  }
}

/// A fading text as a renderer sees it.
pub struct FadingTextView {
  /// The words it says now.
  pub text: Arc<str>,
  /// The style of each part of this text.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl FadingTextView {
  /// The style of a part, as this text is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  /// How the default renderer builds a fading text: the slot, over which the words leave and come, each keyed by
  /// what it says.
  pub fn render(self) -> Element {
    let part = self.part(FadingTextPart::Text);
    let words = Row::new()
      .key(format!("words:{}", self.text))
      .part(&part)
      .child(part.text(&self.text).nowrap());
    let mut root = Stack::new()
      .stack_align(StackAlignment::CenterStart)
      .presence()
      .part(&self.part(FadingTextPart::Root))
      .child(words);
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

impl crate::ui::Styled for FadingText {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for FadingText {
  const NAME: &'static str = "fading_text";
  type Renderer = FadingTextRenderer;

  fn parts() -> crate::ui::Declared {
    FadingTextPart::declared()
  }

  fn states() -> crate::ui::Declared {
    crate::ui::Declared::default()
  }

  /// Words that fade in over the old ones, which fade out, in the fast time; with the page, there at once.
  fn recipe() -> Recipe {
    Recipe::new().base(FadingTextPart::Text, |s| {
      s.enter(PartAnimation::new(
        FADE_IN,
        MotionDuration::Fast,
        MotionEasing::Standard,
      ))
      .exit(PartAnimation::new(
        FADE_OUT,
        MotionDuration::Fast,
        MotionEasing::Standard,
      ))
      .enter_with_parent(false)
    })
  }
}
