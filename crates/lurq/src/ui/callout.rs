//! A callout: a note set apart inside prose or a form.

use std::sync::Arc;

use super::{
  AlertVariant,
  alert::{AlertIcon, shown_icon, wrapping},
  names,
  part_style::Length,
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::glyph,
  theme::{Themed, current_breakpoint, current_theme},
};
use crate::{
  app::theme::{IconSize, PaletteColor, RadiusSize, Theme, TypographyStyle},
  components::{Column, Row},
  layout::{Alignment, text_style::FontWeight},
  node::{Element, border::Border, dimension::Dimension},
};

/// The parts of a callout a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CalloutPart {
  /// The box: its fill, its corners, its bar, and the room inside it.
  Root,
  /// The icon of its variant: at the start, or before the title where it
  /// has a bar.
  Icon,
  /// The icon and the title side by side, over the content, where it has a
  /// bar and a title.
  Heading,
  /// The title, the text and what the app put in it, one over the other.
  Content,
  /// What it is, in a word: "Note", "Tip".
  Title,
  /// What it says.
  Text,
}

names!(CalloutPart {
  Root => "root",
  Icon => "icon",
  Heading => "heading",
  Content => "content",
  Title => "title",
  Text => "text",
});

/// The states of a callout a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CalloutState {
  /// It has a bar along its start, as a note of a document has.
  Bar,
  /// It has a title.
  Titled,
}

names!(CalloutState {
  Bar => "bar",
  Titled => "titled",
});

/// What builds a callout's nodes: a theme's in place of the default one.
pub type CalloutRenderer = Arc<dyn Fn(CalloutView) -> Element + Send + Sync>;

/// A callout: a note set apart inside prose or a form. A hint beside a
/// setting, a caution in a page of documentation.
///
/// ```
/// use lurq::ui::{AlertVariant, Callout, icons};
///
/// let note = Callout::new("Theme changes rerender only the components that read the theme.")
///   .variant(AlertVariant::Info);
/// // A note of a document: a bar along its start, the icon before its title.
/// let tip = Callout::new("Use backdrop blur for panels over busy content.")
///   .title("Tip")
///   .variant(AlertVariant::Success)
///   .icon(icons::LIGHTBULB)
///   .bar(true);
/// ```
///
/// It is read, and is no stop of Tab: a `note` to tooling that reads the
/// tree, called by its title where it has one.
pub struct Callout {
  text: Arc<str>,
  title: Option<Arc<str>>,
  content: Vec<Element>,
  variant: AlertVariant,
  icon: Option<AlertIcon>,
  bar: bool,
  role: Option<Arc<str>>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Callout {
  /// A callout that says `text`. An empty one says nothing, for a callout of
  /// a title and elements alone.
  pub fn new(text: &str) -> Self {
    Self {
      text: Arc::from(text),
      title: None,
      content: Vec::new(),
      variant: AlertVariant::Neutral,
      icon: None,
      bar: false,
      role: None,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// What it is, in a word, over its text: "Note", "Tip", "Warning".
  pub fn title(mut self, title: &str) -> Self {
    self.title = Some(Arc::from(title));
    self
  }

  /// Adds an element under the text: a link, a list, a row of code.
  pub fn child(mut self, child: impl Into<Element>) -> Self {
    self.content.push(child.into());
    self
  }

  /// Adds elements under the text.
  pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.content.extend(children.into_iter().map(Into::into));
    self
  }

  pub fn variant(mut self, variant: AlertVariant) -> Self {
    self.variant = variant;
    self
  }

  /// An icon of the theme's set in place of the variant's, or none: `icon(None)`.
  pub fn icon(mut self, icon: impl Into<AlertIcon>) -> Self {
    self.icon = Some(icon.into());
    self
  }

  /// Whether it has a bar along its start in the variant's color, its title
  /// in that color with the icon before it, as a note of a document has.
  pub fn bar(mut self, bar: bool) -> Self {
    self.bar = bar;
    self
  }

  /// What it is to tooling that reads the tree, in place of a `note`.
  pub fn role(mut self, role: &str) -> Self {
    self.role = Some(Arc::from(role));
    self
  }

  /// Puts the callout in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this callout over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this callout.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the box.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the callout in `theme`. Converting a callout into an element
  /// does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: Some(self.variant.into()),
      size: None,
      density: None,
      breakpoint: current_breakpoint(),
      states: self.states.clone(),
    }
    .state(CalloutState::Bar, self.bar)
    .state(CalloutState::Titled, self.title.is_some());
    let view = CalloutView {
      text: self.text,
      title: self.title,
      content: self.content,
      variant: self.variant,
      icon: shown_icon(self.icon, self.variant.icon()),
      bar: self.bar,
      role: self.role.unwrap_or_else(|| Arc::from("note")),
      recipe: theme.resolve::<Callout>(&case, Some(&self.rules)),
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Callout>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Callout> for Element {
  fn from(callout: Callout) -> Self {
    callout.build(&current_theme())
  }
}

/// A callout as a renderer sees it: what it was given and the style of its
/// parts.
pub struct CalloutView {
  pub text: Arc<str>,
  pub title: Option<Arc<str>>,
  /// What the app put under the text.
  pub content: Vec<Element>,
  pub variant: AlertVariant,
  /// The icon it shows, by its name in the theme's set: the variant's, the one it was given, or none.
  pub icon: Option<Name>,
  pub bar: bool,
  /// What it is to tooling that reads the tree: a `note`, or what the app said.
  pub role: Arc<str>,
  /// The style of each part of this callout.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl CalloutView {
  /// The style of a part, as this callout is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The icon in the style of its part, if the callout has one.
  pub fn icon(&self) -> Option<Element> {
    self.icon.map(|icon| glyph(&self.part(CalloutPart::Icon), icon, false))
  }

  /// The title in the style of its part, as wide as the room it has.
  pub fn title(&self) -> Option<Column> {
    let title = self.title.as_ref()?;
    Some(wrapping(&self.part(CalloutPart::Title), title))
  }

  /// The icon and the title side by side: what a callout with a bar and a
  /// title has over its content.
  pub fn heading(&self) -> Option<Row> {
    let title = self.title()?;
    let mut heading = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .part(&self.part(CalloutPart::Heading));
    if let Some(icon) = self.icon() {
      heading = heading.child(icon);
    }
    Some(heading.child(title.flex(1.0).min_width(Dimension::Px(0.0))))
  }

  /// The text, and what the app put under it, one over the other: with the
  /// title over them where `titled`.
  pub fn content(&mut self, titled: bool) -> Column {
    let mut content = Column::new()
      .flex(1.0)
      .min_width(Dimension::Px(0.0))
      .part(&self.part(CalloutPart::Content));
    if titled && let Some(title) = self.title() {
      content = content.child(title);
    }
    if !self.text.is_empty() {
      content = content.child(wrapping(&self.part(CalloutPart::Text), &self.text));
    }
    content.with_children(std::mem::take(&mut self.content))
  }

  /// How the default renderer builds a callout: the icon at the start and
  /// the content beside it, or, with a bar and a title, the heading over the
  /// content. It is what its role says to what reads the tree, called by its
  /// title.
  pub fn render(mut self) -> Element {
    // The root is a row or a column: each is given the root's style, role, name, id and key alike.
    macro_rules! dressed {
      ($root:expr) => {{
        let mut root = $root
          .part(&self.part(CalloutPart::Root))
          .describe("a11y_role", self.role.clone());
        if let Some(title) = &self.title {
          root = root.describe("a11y_name", title.clone());
        }
        if let Some(id) = &self.id {
          root = root.id(id.clone());
        }
        if let Some(key) = &self.key {
          root = root.key(key.clone());
        }
        Element::from(root)
      }};
    }
    match self.heading().filter(|_| self.bar) {
      Some(heading) => {
        let content = self.content(false).flex(0.0).width(Dimension::Pct(100.0));
        dressed!(Column::new().child(heading).child(content))
      }
      None => {
        let mut row = Row::new().align_items(Alignment::Start);
        if let Some(icon) = self.icon() {
          row = row.child(icon);
        }
        let content = self.content(true);
        dressed!(row.child(content))
      }
    }
  }
}

impl Themed for Callout {
  const NAME: &'static str = "callout";
  type Renderer = CalloutRenderer;

  /// The design's callout (`pencil/lurq-design-system.pen`, Callout): a box
  /// as wide as the room it is given in the muted form of the variant's
  /// color (the muted fill for a neutral one), corners of `radius-md`,
  /// padded 12 and 14. The icon of 16 in the variant's color is 10 from the
  /// text, 13 in the foreground's and 1.5 high; a title over it is semibold.
  /// With a bar it is the board's note of a document (Markdown/Admonition):
  /// a bar of 3 along its start in the variant's color and square corners on
  /// that side, the icon 8 before a title of 14, semibold, in the variant's
  /// color, the text of 14 4 under them.
  fn recipe() -> Recipe {
    use CalloutPart as P;
    use CalloutState::{Bar, Titled};
    let mut recipe = Recipe::new()
      .base(P::Root, |s| {
        s.width(Length::Fill)
          .background(PaletteColor::Muted)
          .foreground(PaletteColor::Foreground)
          .radius(RadiusSize::Md)
          .padding_x(14.0)
          .padding_y(12.0)
          .gap(10.0)
      })
      .base(P::Icon, |s| {
        s.square(Length::Icon(IconSize::Md))
          .foreground(PaletteColor::Foreground)
      })
      .base(P::Heading, |s| s.gap(8.0))
      .base(P::Content, |s| s.gap(4.0))
      .base(P::Title, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .weight(FontWeight::SemiBold)
          .foreground(PaletteColor::Foreground)
      })
      .base(P::Text, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .line_height(1.5)
          .foreground(PaletteColor::Foreground)
      })
      .state(Bar, P::Root, |s| s.corners(0.0, RadiusSize::Md, RadiusSize::Md, 0.0))
      .when(Selector::new().state(Bar).state(Titled), P::Root, |s| s.gap(4.0))
      .state(Bar, P::Title, |s| s.font_size(14.0))
      .state(Bar, P::Text, |s| s.font_size(14.0));
    for variant in AlertVariant::all().iter().copied() {
      let barred = || Selector::new().variant(variant).state(Bar);
      recipe = recipe
        .variant(variant, P::Root, |s| s.background(variant.tint()))
        .variant(variant, P::Icon, |s| s.foreground(variant.accent()))
        .when(barred(), P::Root, |s| {
          s.start_line(Border::inside(3.0, variant.accent()))
        })
        .when(barred(), P::Title, |s| s.foreground(variant.accent()));
    }
    recipe
  }
}
