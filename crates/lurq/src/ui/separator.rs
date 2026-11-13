//! A separator: a line between things, along a row or down a column.

use std::sync::Arc;

use super::{
  names,
  part_style::Length,
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{PaletteColor, Theme, TypographyStyle},
  components::{Column, Row},
  layout::Alignment,
  node::Element,
};

/// The parts of a separator a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SeparatorPart {
  /// What holds the line, and the label where it has one.
  Root,
  /// The line: on either side of the label where it has one.
  Line,
  /// A word in the middle of the line: "or".
  Label,
}

names!(SeparatorPart {
  Root => "root",
  Line => "line",
  Label => "label",
});

/// Which way a separator runs: the variant of its recipe.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SeparatorOrientation {
  /// Across a column, between what is over it and what is under it.
  #[default]
  Horizontal,
  /// Down a row, between what is before it and what is after it.
  Vertical,
}

names!(SeparatorOrientation {
  Horizontal => "horizontal",
  Vertical => "vertical",
});

/// What builds a separator's nodes: a theme's in place of the default one.
pub type SeparatorRenderer = Arc<dyn Fn(SeparatorView) -> Element + Send + Sync>;

/// A separator: a line between things. Sections of a form, groups of a
/// toolbar, the two ways to sign in.
///
/// ```
/// use lurq::{
///   components::{Column, Row},
///   ui::{Button, Separator},
/// };
///
/// let form = Column::new()
///   .child(Button::new("Continue with GitHub"))
///   .child(Separator::horizontal().label("or"))
///   .child(Button::new("Continue with email"));
/// let toolbar = Row::new()
///   .child(Button::new("Undo"))
///   .child(Separator::vertical())
///   .child(Button::new("Bold"));
/// ```
///
/// A separator across a column is as wide as the room the column gives it.
/// One down a row is as high as the row where the row stretches its
/// children (`align_items(Alignment::Stretch)`); in a row that does not, it
/// is given a [`Separator::length`]. One given a length is as long as that. It is decoration to
/// tooling that reads the tree, unless the app says it separates something
/// a person moves between ([`Separator::decorative`]).
pub struct Separator {
  orientation: SeparatorOrientation,
  label: Option<Arc<str>>,
  decorative: bool,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Separator {
  /// A separator that runs `orientation`.
  pub fn new(orientation: SeparatorOrientation) -> Self {
    Self {
      orientation,
      label: None,
      decorative: true,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// A line across a column.
  pub fn horizontal() -> Self {
    Self::new(SeparatorOrientation::Horizontal)
  }

  /// A line down a row.
  pub fn vertical() -> Self {
    Self::new(SeparatorOrientation::Vertical)
  }

  /// A word in the middle of the line, with the line on either side of it: "or".
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// How long the separator is, in logical pixels, over the room it would
  /// take: its width across a column, its height down a row.
  pub fn length(mut self, length: f32) -> Self {
    let orientation = self.orientation;
    self.rules = self.rules.base(SeparatorPart::Root, |s| match orientation {
      SeparatorOrientation::Horizontal => s.width(length),
      SeparatorOrientation::Vertical => s.height(length),
    });
    self
  }

  /// Whether the separator is decoration alone, which tooling that reads the
  /// tree passes by. On by default; off for one that separates parts a
  /// person moves between, which tooling is then told is a separator.
  pub fn decorative(mut self, decorative: bool) -> Self {
    self.decorative = decorative;
    self
  }

  /// Puts the separator in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this separator over what the theme gives it: an
  /// inset is the root's padding.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this separator.
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

  /// Builds the separator in `theme`. Converting a separator into an element
  /// does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: Some(self.orientation.into()),
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    };
    let view = SeparatorView {
      orientation: self.orientation,
      label: self.label,
      decorative: self.decorative,
      recipe: theme.resolve::<Separator>(&case, Some(&self.rules)),
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Separator>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Separator> for Element {
  fn from(separator: Separator) -> Self {
    separator.build(&current_theme())
  }
}

/// A separator as a renderer sees it: what it was given and the style of its parts.
pub struct SeparatorView {
  pub orientation: SeparatorOrientation,
  pub label: Option<Arc<str>>,
  pub decorative: bool,
  /// The style of each part of this separator.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl SeparatorView {
  /// The style of a part, as this separator is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// A line in the style of its part, which takes the length the root has
  /// left it: all of it, or half of what the label leaves.
  pub fn line(&self) -> Element {
    let part = self.part(SeparatorPart::Line);
    match self.orientation {
      SeparatorOrientation::Horizontal => Row::new().flex(1.0).part(&part).into(),
      SeparatorOrientation::Vertical => Column::new().flex(1.0).part(&part).into(),
    }
  }

  /// The label in the style of its part: `None` for a separator without one.
  pub fn label(&self) -> Option<Row> {
    let label = self.label.as_ref()?;
    let part = self.part(SeparatorPart::Label);
    Some(Row::new().part(&part).child(part.text(label).nowrap()))
  }

  /// How the default renderer builds a separator: the line, or a line on
  /// either side of the label.
  pub fn render(self) -> Element {
    let mut children = vec![self.line()];
    if let Some(label) = self.label() {
      children.push(label.into());
      children.push(self.line());
    }
    let part = self.part(SeparatorPart::Root);
    macro_rules! whole {
      ($root:expr) => {{
        let mut root = $root.align_items(Alignment::Center).part(&part).with_children(children);
        if !self.decorative {
          root = root
            .describe("a11y_role", "separator")
            .describe("a11y_orientation", self.orientation.as_str());
          if let Some(label) = &self.label {
            root = root.describe("a11y_name", label.clone());
          }
        }
        if let Some(id) = self.id.clone() {
          root = root.id(id);
        }
        if let Some(key) = self.key.clone() {
          root = root.key(key);
        }
        root.into()
      }};
    }
    match self.orientation {
      SeparatorOrientation::Horizontal => whole!(Row::new()),
      SeparatorOrientation::Vertical => whole!(Column::new()),
    }
  }
}

impl Themed for Separator {
  const NAME: &'static str = "separator";
  type Renderer = SeparatorRenderer;

  /// The design's separator (`pencil/lurq-design-system.pen`, Separator): a
  /// line of 1 in the border's color, as long as the room it is given. A
  /// label stands 8 from the line on either side, in the caption's type of
  /// the muted color.
  fn recipe() -> Recipe {
    use PaletteColor::*;
    use SeparatorOrientation::{Horizontal, Vertical};
    use SeparatorPart::{Label, Line, Root};
    Recipe::new()
      .base(Root, |s| s.gap(8.0))
      // Down a row it takes no height of its own, which would be all the room the row is offered: a row that
      // stretches its children makes it as high as the row.
      .variant(Horizontal, Root, |s| s.width(Length::Fill))
      .base(Line, |s| s.background(Border))
      .variant(Horizontal, Line, |s| s.height(1.0))
      .variant(Vertical, Line, |s| s.width(1.0))
      .base(Label, |s| {
        s.typography(TypographyStyle::Caption).foreground(MutedForeground)
      })
  }
}
