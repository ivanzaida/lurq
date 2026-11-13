//! A card: a surface that holds one thing, with a header, its content and a
//! footer where it has them.

use std::sync::Arc;

use super::{
  names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{fading, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    events::MouseEvent,
    theme::{BorderSize, OpacityLevel, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle},
  },
  components::{self, Column, Row},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, EventHandler, IntoMouseEventHandler, border::Border as Edge, dimension::Dimension},
};

/// The parts of a card a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CardPart {
  /// The surface: its fill, its edge, its corners, its shadow. What is
  /// pressed, where the card is.
  Root,
  /// The padded column of the header and the content, over the footer.
  Body,
  /// The heading, and the action at its end.
  Header,
  /// The title and the description, one over the other.
  Heading,
  /// What the card is about, in a word or a line.
  Title,
  /// More of it, under the title, in the muted color.
  Description,
  /// What the app puts at the end of the header: a button, a menu, a badge.
  Action,
  /// What the app puts in the card, under the header.
  Content,
  /// The row of buttons at the bottom, at its end, set off by a line.
  Footer,
}

names!(CardPart {
  Root => "root",
  Body => "body",
  Header => "header",
  Heading => "heading",
  Title => "title",
  Description => "description",
  Action => "action",
  Content => "content",
  Footer => "footer",
});

/// The states of a card a recipe styles, besides what the pointer and the
/// keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CardState {
  /// The card is pressed as a whole: it answers the pointer and the keyboard.
  Pressable,
  /// It takes no input.
  Disabled,
  /// It has a footer under its body, which a line sets off.
  Footed,
}

names!(CardState {
  Pressable => "pressable",
  Disabled => "disabled",
  Footed => "footed",
});

/// How a card stands off the page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CardVariant {
  /// A hairline edge and the faintest shadow: the card of most places.
  #[default]
  Outline,
  /// A soft shadow and no edge: a card over a busy page.
  Elevated,
  /// A muted fill and no edge: cards that group what is in them, side by side.
  Filled,
}

names!(CardVariant {
  Outline => "outline",
  Elevated => "elevated",
  Filled => "filled",
});

/// How roomy a card is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CardSize {
  /// Padded 16, a title of 14 and a description of 13: a card of a grid, a
  /// choice, a tile.
  Sm,
  /// Padded 24, a title of 16 and a description of 14.
  #[default]
  Md,
}

names!(CardSize {
  Sm => "sm",
  Md => "md",
});

/// What builds a card's nodes: a theme's in place of the default one.
pub type CardRenderer = Arc<dyn Fn(CardView) -> Element + Send + Sync>;

/// A card: a surface that holds one thing. A group of settings, a summary
/// of a project, a form of a few fields.
///
/// ```
/// use lurq::ui::{Button, ButtonVariant, Card, CardVariant};
///
/// let appearance = Card::new()
///   .title("Appearance")
///   .description("How lurq looks on this device.")
///   .child(Button::new("Reset to defaults").variant(ButtonVariant::Outline))
///   .footer(Button::new("Cancel").variant(ButtonVariant::Outline))
///   .footer(Button::new("Save"));
/// // A card that is pressed as a whole: one stop of Tab.
/// let project = Card::new()
///   .title("lurq")
///   .description("A Rust UI toolkit with GPU-backed rendering.")
///   .variant(CardVariant::Elevated)
///   .on_click(|_| println!("open the project"));
/// ```
///
/// Every part is optional: a card with children alone is a padded surface.
/// A card is as wide as the room it is given unless it is given a width of
/// its own, and its title and description wrap in the width it has. Cards
/// side by side in a row are each given their share: a column with
/// `flex(1.0)` around each, or a grid. A card given
/// [`Card::on_click`] is pressed as a whole, from the pointer and the
/// keyboard; a press of a control in its action or its footer is that
/// control's and not the card's. A card that is not pressed is read, and
/// the controls in it are each a stop of Tab of their own.
pub struct Card {
  title: Option<Arc<str>>,
  description: Option<Arc<str>>,
  action: Option<Element>,
  content: Vec<Element>,
  footer: Vec<Element>,
  variant: Name,
  size: Name,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_click: Vec<EventHandler<MouseEvent>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Default for Card {
  fn default() -> Self {
    Self::new()
  }
}

impl Card {
  /// A card with nothing in it yet.
  pub fn new() -> Self {
    Self {
      title: None,
      description: None,
      action: None,
      content: Vec::new(),
      footer: Vec::new(),
      variant: CardVariant::default().into(),
      size: CardSize::default().into(),
      disabled: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_click: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// What the card is about: "Appearance". It is also what the card is
  /// called to tooling that reads the tree.
  pub fn title(mut self, title: &str) -> Self {
    self.title = Some(Arc::from(title));
    self
  }

  /// More of it, under the title, in the muted color.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// What stands at the end of the header, beside the title: a button, a
  /// menu, a badge.
  pub fn action(mut self, action: impl Into<Element>) -> Self {
    self.action = Some(action.into());
    self
  }

  /// Adds an element to the content, under the header.
  pub fn child(mut self, child: impl Into<Element>) -> Self {
    self.content.push(child.into());
    self
  }

  /// Adds elements to the content.
  pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.content.extend(children.into_iter().map(Into::into));
    self
  }

  /// Adds an element to the footer, after those already there: the footer is
  /// a row at the end of the card under a line, its least action first and
  /// its main one last.
  pub fn footer(mut self, element: impl Into<Element>) -> Self {
    self.footer.push(element.into());
    self
  }

  /// A [`CardVariant`], or a variant the theme's recipe names.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// A [`CardSize`], or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// A width of its own, in logical pixels, over all the room it is given.
  pub fn width(mut self, width: impl Into<Length>) -> Self {
    let width = width.into();
    self.rules = self.rules.base(CardPart::Root, |s| s.width(width));
    self
  }

  /// Whether the card takes no input: a card that is pressed is dimmed and
  /// is not pressed.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Puts the card in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Called on a press of the card: a click, or Enter or Space while it has
  /// the focus. A card given one is pressed as a whole.
  pub fn on_click(mut self, handler: impl IntoMouseEventHandler) -> Self {
    self.on_click.push(handler.into_event_handler());
    self
  }

  /// Styles one part of this card over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this card.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the card as it is under the pointer, held down or focused from
  /// the keyboard, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the surface: what is pressed, where the card is.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the card in `theme`. Converting a card into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let pressable = !self.on_click.is_empty();
    let has_body =
      self.title.is_some() || self.description.is_some() || self.action.is_some() || !self.content.is_empty();
    let case = Case {
      variant: Some(self.variant),
      size: Some(self.size),
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    // A disabled card is not pressed, so it does not answer the pointer either.
    .state(CardState::Pressable, pressable && !self.disabled)
    .state(CardState::Disabled, self.disabled)
    .state(CardState::Footed, has_body && !self.footer.is_empty());
    let view = CardView {
      title: self.title,
      description: self.description,
      action: self.action,
      content: self.content,
      footer: self.footer,
      pressable,
      disabled: self.disabled,
      recipe: theme.resolve::<Card>(&case, Some(&self.rules)),
      forced: self.forced,
      on_click: self.on_click,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Card>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Card> for Element {
  fn from(card: Card) -> Self {
    card.build(&current_theme())
  }
}

/// A card as a renderer sees it: what it was given, the style of its parts,
/// and its behavior ready to be dressed.
pub struct CardView {
  pub title: Option<Arc<str>>,
  pub description: Option<Arc<str>>,
  /// What the app put at the end of the header.
  pub action: Option<Element>,
  /// What the app put in the content.
  pub content: Vec<Element>,
  /// What the app put in the footer.
  pub footer: Vec<Element>,
  /// Whether the card is pressed as a whole.
  pub pressable: bool,
  pub disabled: bool,
  /// The style of each part of this card.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  on_click: Vec<EventHandler<MouseEvent>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl CardView {
  /// The style of a part, as this card is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// A row that keeps a press of what is in it from the card, where the card
  /// is pressed: a press of a button in the footer is the button's alone.
  fn own_presses(&self, row: Row) -> Row {
    if self.pressable {
      row.on_click(|event: MouseEvent| event.stop_propagation())
    } else {
      row
    }
  }

  /// The title and the description, one over the other, each as wide as the
  /// room the header leaves it: `None` for a card that has neither.
  pub fn heading(&self) -> Option<Column> {
    if self.title.is_none() && self.description.is_none() {
      return None;
    }
    let mut heading = Column::new()
      .flex(1.0)
      .min_width(Dimension::Px(0.0))
      .part(&self.part(CardPart::Heading));
    if let Some(title) = &self.title {
      heading = heading.child(wrapping(&self.part(CardPart::Title), title));
    }
    if let Some(description) = &self.description {
      heading = heading.child(wrapping(&self.part(CardPart::Description), description));
    }
    Some(heading)
  }

  /// The action, in the style of its part: `None` for a card without one.
  pub fn action(&mut self) -> Option<Row> {
    let action = self.action.take()?;
    let row = Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(CardPart::Action))
      .child(action);
    Some(self.own_presses(row))
  }

  /// The header: the heading, and the action at its end. `None` for a card
  /// with neither.
  pub fn header(&mut self) -> Option<Row> {
    let heading = self.heading();
    let action = self.action();
    if heading.is_none() && action.is_none() {
      return None;
    }
    let mut header = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Start)
      .part(&self.part(CardPart::Header));
    match heading {
      Some(heading) => header = header.child(heading),
      // Without a heading the action still stands at the end.
      None => header = header.justify(Justify::End),
    }
    if let Some(action) = action {
      header = header.child(action);
    }
    Some(header)
  }

  /// The content: a column with what the app put there. `None` for a card
  /// with nothing in it.
  pub fn content(&mut self) -> Option<Column> {
    if self.content.is_empty() {
      return None;
    }
    Some(
      Column::new()
        .width(Dimension::Pct(100.0))
        .part(&self.part(CardPart::Content))
        .with_children(std::mem::take(&mut self.content)),
    )
  }

  /// The body: the header over the content, padded. `None` for a card with
  /// neither.
  pub fn body(&mut self) -> Option<Column> {
    let header = self.header();
    let content = self.content();
    if header.is_none() && content.is_none() {
      return None;
    }
    let mut body = Column::new()
      .width(Dimension::Pct(100.0))
      .part(&self.part(CardPart::Body));
    if let Some(header) = header {
      body = body.child(header);
    }
    if let Some(content) = content {
      body = body.child(content);
    }
    Some(body)
  }

  /// The footer: a row at the end of the card with what the app put there.
  /// `None` for a card with nothing in it.
  pub fn footer(&mut self) -> Option<Row> {
    if self.footer.is_empty() {
      return None;
    }
    let footer = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .justify(Justify::End)
      .part(&self.part(CardPart::Footer))
      .with_children(std::mem::take(&mut self.footer));
    Some(self.own_presses(footer))
  }

  /// The surface of a card that is pressed as a whole, in the style of its
  /// root, with nothing in it yet: a click, Enter and Space press it, Tab
  /// reaches it, and a disabled one takes nothing. It is a button to what
  /// reads the tree, called by the card's title.
  pub fn pressed_root(&self) -> components::Button {
    let mut root = pressable(&self.part(CardPart::Root), self.disabled)
      .justify(Justify::Start)
      .align_items(Alignment::Stretch)
      // The controls in its action and its footer are stops of Tab of their own, after the card.
      .stops_inside(true)
      .describe("a11y_role", "button");
    if let Some(title) = &self.title {
      root = root.describe("a11y_name", title.clone());
    }
    if !self.disabled {
      for handler in &self.on_click {
        root = root.on_click(handler.clone());
      }
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// The surface of a card that is read, in the style of its root, with
  /// nothing in it yet: a group to what reads the tree, called by the card's
  /// title.
  pub fn read_root(&self) -> Column {
    let mut root = Column::new()
      .part(&self.part(CardPart::Root))
      .describe("a11y_role", "group");
    if let Some(title) = &self.title {
      root = root.describe("a11y_name", title.clone());
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// How the default renderer builds a card: the body over the footer, on
  /// the surface.
  pub fn render(mut self) -> Element {
    let parts: Vec<Element> = [self.body().map(Element::from), self.footer().map(Element::from)]
      .into_iter()
      .flatten()
      .collect();
    if self.pressable {
      // What is pressed is a row: the body and the footer are in a column as wide as it.
      let column = Column::new()
        .flex(1.0)
        .min_width(Dimension::Px(0.0))
        .with_children(parts);
      self.pressed_root().child(column).into()
    } else {
      self.read_root().with_children(parts).into()
    }
  }
}

/// A text of a card in the style of `part`, as wide as the room it has: a
/// longer one goes on to the next line. A row would lay it out as wide as it
/// is and cut it.
fn wrapping(part: &ResolvedPart, content: &str) -> Column {
  let fill = Dimension::Pct(100.0);
  Column::new()
    .width(fill)
    .part(part)
    .child(part.text(content).width(fill))
}

impl Themed for Card {
  const NAME: &'static str = "card";
  type Renderer = CardRenderer;

  /// The design's card (`pencil/lurq-design-system.pen`, Card and
  /// Card/Simple): a surface as wide as the room it is given, in the card's color with an edge of the
  /// border's color, corners of `radius-xl` and the small shadow. Its header
  /// and its content are padded 24 with 24 between them; the title is 16/600
  /// with the description 6 under it in 14 of the muted color, and what is in
  /// the content is 16 apart. The footer is set off by a line and padded 16
  /// and 24, its buttons 8 apart at its end. The elevated card has the
  /// medium shadow and no edge, the filled one the muted color and no edge.
  /// A small card is padded 16, with a title of 14 and a description of 13.
  /// A card that is pressed takes an edge of the ring's color and the
  /// medium shadow under the pointer.
  fn recipe() -> Recipe {
    use CardPart::{Action, Body, Content, Description, Footer, Header, Heading, Root, Title};
    use CardSize::Sm;
    use CardState::{Disabled, Footed, Pressable};
    use CardVariant::{Elevated, Filled, Outline};
    use PaletteColor::*;
    let edge = |color: PaletteColor| Edge::inside(BorderSize::Sm, color);
    let when_pressable = || Selector::new().state(Pressable);
    Recipe::new()
      .base(Root, |s| {
        fading(
          s.width(Length::Fill)
            .background(Card)
            .foreground(CardForeground)
            .radius(RadiusSize::Xl),
        )
      })
      .variant(Outline, Root, |s| s.border(edge(PaletteColor::Border)).shadow(ShadowStyle::Sm))
      .variant(Elevated, Root, |s| s.shadow(ShadowStyle::Md))
      .variant(Filled, Root, |s| s.background(Muted))
      .base(Body, |s| s.padding_x(24.0).padding_y(24.0).gap(24.0))
      .base(Header, |s| s.gap(16.0))
      .base(Heading, |s| s.gap(6.0))
      .base(Title, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(16.0)
          .weight(FontWeight::SemiBold)
          .foreground(CardForeground)
      })
      .base(Description, |s| {
        s.typography(TypographyStyle::Description)
          .font_size(14.0)
          .foreground(MutedForeground)
      })
      .base(Action, |s| s.gap(8.0))
      .base(Content, |s| s.gap(16.0))
      .base(Footer, |s| s.gap(8.0).padding_x(24.0).padding_y(16.0))
      // The line over the footer is the body's: it is there only where the footer has a body over it.
      .state(Footed, Body, |s| s.underline(edge(PaletteColor::Border)))
      .size(Sm, Body, |s| s.padding_x(16.0).padding_y(16.0).gap(16.0))
      .size(Sm, Header, |s| s.gap(12.0))
      .size(Sm, Title, |s| s.font_size(14.0))
      .size(Sm, Description, |s| s.font_size(13.0).line_height(1.4))
      .size(Sm, Content, |s| s.gap(12.0))
      .size(Sm, Footer, |s| s.padding_x(16.0).padding_y(12.0))
      .state(Pressable, Root, |s| s.cursor(CursorIcon::Pointer))
      .when(when_pressable().hovered(), Root, |s| s.border(edge(Ring)).shadow(ShadowStyle::Md))
      // Held down it sinks back to the page, and keeps the edge it has under the pointer.
      .when(when_pressable().pressed(), Root, |s| s.border(edge(Ring)).shadow(ShadowStyle::Sm))
      .when(when_pressable().focus_visible(), Root, |s| s.focus_ring())
      .state(Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
  }
}
