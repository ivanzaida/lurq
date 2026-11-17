//! A watermark: a text, or a picture, repeated across what it covers, turned and faint, over the content.

use std::sync::Arc;

use super::{
  names,
  part_style::Length,
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{PaletteColor, Theme, TypographyStyle},
  components::{Stack, Text},
  layout::{Repeat, StackAlignment, text_style::FontWeight},
  node::{Element, TextTransformMode, dimension::Dimension, transform::Transform2D},
};

/// The parts of a watermark a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WatermarkPart {
  /// What holds the content and the layer of marks over it.
  Root,
  /// The layer over the content that the marks are repeated across: no pointer hits it.
  Layer,
  /// One mark: its type, its color and how faint it is.
  Mark,
}

names!(WatermarkPart {
  Root => "root",
  Layer => "layer",
  Mark => "mark",
});

/// The states of a watermark a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WatermarkState {
  /// Its mark is a picture, not a text.
  Media,
}

names!(WatermarkState {
  Media => "media",
});

/// What builds a watermark's nodes: a theme's in place of the default one.
pub type WatermarkRenderer = Arc<dyn Fn(WatermarkView) -> Element + Send + Sync>;

/// The design's room between one mark's box and the next, across and down: marks 96 apart along a row and rows 44
/// apart, of a text 69 wide on a line of 15.
pub const WATERMARK_GAP: (f32, f32) = (27.0, 29.0);

/// The design's angle: turned 20 degrees counter-clockwise, rising to the right.
pub const WATERMARK_ANGLE: f32 = -20.0;

/// Where the design's first mark's box is from the top left of what is covered: a little before its start, so
/// the first column is cut as the others are.
pub const WATERMARK_OFFSET: (f32, f32) = (-20.0, 18.0);

/// How much further along the design's every other row is.
pub const WATERMARK_STAGGER: f32 = 40.0;

/// A watermark: a text (a name, "Confidential", who is looking) or a picture repeated across what it covers,
/// turned and faint, over the content.
///
/// ```
/// use lurq::{components::Column, ui::Watermark};
///
/// # let page = Column::new();
/// let shared = Watermark::new("ivan@lurq.dev · 16 Nov 2026").child(page);
/// // Steeper, closer together.
/// # let page = Column::new();
/// let draft = Watermark::new("Draft").angle(-35.0).gap(16.0, 24.0).child(page);
/// ```
///
/// The marks are over the content and take nothing from it: no pointer hits them, so the content is pressed,
/// selected and scrolled through them, and tooling does not read them: what it reads is the content. They are one
/// mark in the tree, painted again and again by the engine (`.repeat(Repeat)`), only where they can be seen: over a
/// page of any length inside a scroll a watermark costs the marks in view, and scrolling builds nothing again.
/// What is laid out again is the mark, when its text or its look changes, and the layer, when the size of what it
/// covers does.
///
/// A mark is drawn turned and still sharp at every scale: its glyphs are drawn at the angle (`TextTransformMode::
/// Rasterized`). Its color is the theme's `Foreground`, faint (0.12), so it is dark on the light look and light on
/// the dark one.
pub struct Watermark {
  text: Arc<str>,
  media: Option<Element>,
  gap: (f32, f32),
  angle: f32,
  offset: (f32, f32),
  stagger: f32,
  children: Vec<Element>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Watermark {
  /// A watermark of `text`, over what it is given with [`child`](Self::child).
  pub fn new(text: &str) -> Self {
    Self {
      text: Arc::from(text),
      media: None,
      gap: WATERMARK_GAP,
      angle: WATERMARK_ANGLE,
      offset: WATERMARK_OFFSET,
      stagger: WATERMARK_STAGGER,
      children: Vec::new(),
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// A picture repeated in place of the text: a logo, a stamp. It is the app's, at the size it gives it; it is
  /// turned as the text would be, and faint.
  pub fn media(mut self, media: impl Into<Element>) -> Self {
    self.media = Some(media.into());
    self
  }

  /// The room between one mark's box and the next, across and down, before the marks are turned: the design's
  /// 27 and 29.
  pub fn gap(mut self, x: f32, y: f32) -> Self {
    self.gap = (x.max(0.0), y.max(0.0));
    self
  }

  /// How far each mark is turned, in degrees, clockwise where it is more than nothing: the design's -20, rising
  /// to the right. Nothing lays the marks level.
  pub fn angle(mut self, degrees: f32) -> Self {
    self.angle = degrees;
    self
  }

  /// Where the first mark's box is from the top left of what is covered, before it is turned: the design's -20
  /// and 18. The marks go on from it every way.
  pub fn offset(mut self, x: f32, y: f32) -> Self {
    self.offset = (x, y);
    self
  }

  /// How much further along every other row is: the design's 40, so the rows do not line up. Nothing lines them up.
  pub fn stagger(mut self, stagger: f32) -> Self {
    self.stagger = stagger;
    self
  }

  /// What the watermark covers.
  pub fn child(mut self, child: impl Into<Element>) -> Self {
    self.children.push(child.into());
    self
  }

  /// What the watermark covers, more than one thing laid one over another from its top left.
  pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.children.extend(children.into_iter().map(Into::into));
    self
  }

  /// Puts the watermark in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this watermark over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this watermark.
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

  /// Builds the watermark in `theme`. Converting a watermark into an element does this in the theme of what is
  /// being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(WatermarkState::Media, self.media.is_some());
    let view = WatermarkView {
      text: self.text,
      media: self.media,
      gap: self.gap,
      angle: self.angle,
      offset: self.offset,
      stagger: self.stagger,
      children: self.children,
      recipe: theme.resolve::<Watermark>(&case, Some(&self.rules)),
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Watermark>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Watermark> for Element {
  fn from(watermark: Watermark) -> Self {
    watermark.build(&current_theme())
  }
}

/// A watermark as a renderer sees it.
pub struct WatermarkView {
  /// The text of its marks.
  pub text: Arc<str>,
  /// The picture of its marks, in place of the text.
  pub media: Option<Element>,
  /// The room between one mark's box and the next, across and down.
  pub gap: (f32, f32),
  /// How far each mark is turned, in degrees.
  pub angle: f32,
  /// Where the first mark's box is.
  pub offset: (f32, f32),
  /// How much further along every other row is.
  pub stagger: f32,
  /// What it covers.
  pub children: Vec<Element>,
  /// The style of each part of this watermark.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl WatermarkView {
  /// The style of a part, as this watermark is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The mark: the text in the style of its part, or the picture, faint and turned, where the first one is. Its
  /// glyphs are drawn at the angle, so it is sharp at every scale.
  pub fn mark(&mut self) -> Element {
    let part = self.part(WatermarkPart::Mark);
    let turn = Transform2D::rotate_deg(self.angle);
    let (x, y) = self.offset;
    match self.media.take() {
      Some(media) => Stack::new()
        .part(&part)
        .absolute_position(x, y)
        .transform(turn)
        .child(media)
        .into(),
      None => Text::styled(&self.text, part.rest().text.resolve(self.theme()))
        .nowrap()
        .part(&part)
        .absolute_position(x, y)
        .transform(turn)
        .text_transform_mode(TextTransformMode::Rasterized)
        .into(),
    }
  }

  /// The layer the marks are repeated across: as large as what it covers, over it, hit by no pointer, and
  /// nothing to tooling. Its id is the watermark's with `-layer` after it.
  pub fn layer(&mut self) -> Stack {
    let full = Dimension::Pct(100.0);
    let mut layer = Stack::new()
      .part(&self.part(WatermarkPart::Layer))
      .absolute(0.0, 0.0, full, full)
      .pointer_events_none()
      .repeat(Repeat::new().gap(self.gap.0, self.gap.1).stagger(self.stagger))
      .describe("a11y_hidden", "true")
      .child(self.mark());
    if let Some(id) = &self.id {
      layer = layer.id(format!("{id}-layer"));
    }
    layer
  }

  /// How the default renderer builds a watermark: what it covers, laid one over another from the top left, and
  /// the layer of marks over all of it.
  pub fn render(mut self) -> Element {
    let layer = self.layer();
    let mut root = Stack::new()
      .stack_align(StackAlignment::TopStart)
      .part(&self.part(WatermarkPart::Root))
      .with_children(std::mem::take(&mut self.children))
      .child(layer);
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

impl Themed for Watermark {
  const NAME: &'static str = "watermark";
  type Renderer = WatermarkRenderer;

  /// The design's watermark (`pencil/lurq-design-system.pen`, Watermark): "lurq internal" in the sans of 12,
  /// medium, in `foreground` at 0.12, turned 20 degrees counter-clockwise, 96 apart along a row and 44 from row to
  /// row, every other row 40 further along. The board draws it over a card of 220 by 120: the card is the content
  /// it covers, not the watermark's.
  fn recipe() -> Recipe {
    use WatermarkPart::{Mark, Root};
    // As large as what it covers, and no wider than what holds it, so that what it covers can be held to it.
    Recipe::new().base(Root, |s| s.max_width(Length::Fill)).base(Mark, |s| {
      s.typography(TypographyStyle::Caption)
        .weight(FontWeight::Medium)
        .foreground(PaletteColor::Foreground)
        .opacity(0.12)
    })
  }
}
