//! A quote set apart from the text around it, by a line at its start.

use std::sync::Arc;

use super::{
  names,
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  theme::{Themed, current_breakpoint, current_theme},
};
use crate::{
  app::theme::{BorderSize, PaletteColor, Theme, TypographyStyle},
  components::Column,
  layout::{Alignment, text_style::FontStyle},
  node::{Element, border::Border, dimension::Dimension},
  ui::{Length, PartStyle},
};

/// The parts of a quote a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlockquotePart {
  /// The quote: its line at the start and its padding.
  Root,
  /// Its text.
  Label,
  /// What it holds under its text: paragraphs, a list, a block of code.
  Body,
}

names!(BlockquotePart {
  Root => "root",
  Label => "label",
  Body => "body",
});

/// The states of a quote a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlockquoteState {
  /// It holds elements under its text.
  Body,
}

names!(BlockquoteState {
  Body => "body",
});

/// What builds the nodes of a quote: a theme's in place of the default one.
pub type BlockquoteRenderer = Arc<dyn Fn(BlockquoteView) -> Element + Send + Sync>;

/// A quote set apart from the text around it: words someone said, a passage of another text, Markdown's `>`.
///
/// ```
/// use lurq::{components::Text, ui::Blockquote};
///
/// let said = Blockquote::new("A design system is a set of decisions you no longer have to make.");
/// let passage = Blockquote::new("").child(Text::new("First paragraph.")).child(Text::new("Second paragraph."));
/// ```
///
/// Its text wraps at its width. It is read and not operated: to what reads the tree it is a `blockquote`, called by
/// its label where it has one.
pub struct Blockquote {
  text: Arc<str>,
  body: Vec<Element>,
  label: Option<Arc<str>>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Blockquote {
  /// A quote that says `text`. An empty one says nothing, for a quote of elements alone.
  pub fn new(text: &str) -> Self {
    Self {
      text: Arc::from(text),
      body: Vec::new(),
      label: None,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// Adds an element under the text: a paragraph, a list, a block of code.
  pub fn child(mut self, child: impl Into<Element>) -> Self {
    self.body.push(child.into());
    self
  }

  /// Adds elements under the text.
  pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.body.extend(children.into_iter().map(Into::into));
    self
  }

  /// Its name to what reads the tree: who said it, where it is from.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
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

  /// Lays rules over the theme's recipe for this quote.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
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

  /// Builds the quote in `theme`. Converting it into an element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let mut case = Case::new()
      .breakpoint(current_breakpoint())
      .state(BlockquoteState::Body, !self.body.is_empty());
    for state in &self.states {
      case = case.state(state.clone(), true);
    }
    let view = BlockquoteView {
      text: self.text,
      body: self.body,
      label: self.label,
      recipe: theme.resolve::<Blockquote>(&case, Some(&self.rules)),
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Blockquote>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Blockquote> for Element {
  fn from(quote: Blockquote) -> Self {
    quote.build(&current_theme())
  }
}

/// A quote as a renderer sees it.
pub struct BlockquoteView {
  pub text: Arc<str>,
  pub body: Vec<Element>,
  pub label: Option<Arc<str>>,
  /// The style of each part.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl BlockquoteView {
  /// The style of a part, as the quote is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The quote with nothing in it yet: a `blockquote` to what reads the tree.
  pub fn root(&self) -> Column {
    let mut root = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(BlockquotePart::Root))
      .describe("a11y_role", "blockquote");
    if let Some(label) = &self.label {
      root = root.describe("a11y_name", label.clone());
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// Its text, wrapping at its width; none where it says nothing.
  pub fn label(&self) -> Option<Element> {
    if self.text.is_empty() {
      return None;
    }
    let part = self.part(BlockquotePart::Label);
    Some(part.text(&self.text).part(&part).width(Dimension::Pct(100.0)).into())
  }

  /// What it holds under its text, one under another.
  pub fn body(&mut self) -> Option<Element> {
    if self.body.is_empty() {
      return None;
    }
    let body = std::mem::take(&mut self.body);
    Some(
      Column::new()
        .align_items(Alignment::Stretch)
        .part(&self.part(BlockquotePart::Body))
        .with_children(body)
        .into(),
    )
  }

  /// How the default renderer builds a quote: its text over what it holds.
  pub fn render(mut self) -> Element {
    let root = self.root();
    let label = self.label();
    let body = self.body();
    root.with_children(label.into_iter().chain(body)).into()
  }
}

impl Themed for Blockquote {
  const NAME: &'static str = "blockquote";
  type Renderer = BlockquoteRenderer;

  /// The design's quote (`pencil/lurq-design-system.pen`, Blockquote `mlxnG`, in Prose and the Markdown document): a
  /// line of 2 in `Border` at its start, padded 16 after it and 4 above and below, its text 14 in italics on lines
  /// of 1.5 in `MutedForeground`. Where the board is silent, what it holds under its text is 8 apart.
  fn recipe() -> Recipe {
    use BlockquotePart::{Body, Label, Root};
    Recipe::new()
      .base(Root, |s| {
        s.width(Length::Fill)
          .start_line(Border::inside(BorderSize::Md, PaletteColor::Border))
          .padding_start(16.0)
          .padding_y(4.0)
          .gap(8.0)
          .foreground(PaletteColor::MutedForeground)
      })
      .base(Label, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(14.0)
          .font_style(FontStyle::Italic)
          .line_height(1.5)
          .foreground(PaletteColor::MutedForeground)
      })
      .base(Body, |s| s.gap(8.0))
  }
}
