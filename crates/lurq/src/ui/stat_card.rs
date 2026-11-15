//! A stat card: one number with what it is and how it moved. Frames per second, the users of a week, the time a
//! build takes.

use std::sync::Arc;

use super::{
  badge::{Badge, BadgeVariant},
  names,
  part_style::{FontFamily, Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{fading, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    events::MouseEvent,
    theme::{BorderSize, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle},
  },
  components::{Column, Row},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{
    CursorIcon, Element, EventHandler, IntoMouseEventHandler, border::Border as Edge, dimension::Dimension,
    outline::Outline,
  },
};

/// The tone of a stat card's change: whether it is good or not.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StatCardVariant {
  /// A change with no judgment of its own.
  #[default]
  Neutral,
  /// A change to know of.
  Info,
  /// A change for the better: more frames, fewer errors.
  Success,
  /// A change to look at.
  Warning,
  /// A change for the worse.
  Destructive,
}

names!(StatCardVariant {
  Neutral => "neutral",
  Info => "info",
  Success => "success",
  Warning => "warning",
  Destructive => "destructive",
});

impl StatCardVariant {
  /// The variant called `name`, where it is one of these: a variant a theme adds is none of them.
  pub fn named(name: Name) -> Option<Self> {
    Self::all().iter().copied().find(|variant| Name::from(*variant) == name)
  }

  fn badge(self) -> BadgeVariant {
    match self {
      Self::Neutral => BadgeVariant::Secondary,
      Self::Info => BadgeVariant::Info,
      Self::Success => BadgeVariant::Success,
      Self::Warning => BadgeVariant::Warning,
      Self::Destructive => BadgeVariant::Destructive,
    }
  }
}

/// How large a stat card is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StatCardSize {
  /// The card around the board's sparkline: 16 in, its label 12 and its value 24.
  Sm,
  /// The board's stat card: 20 in, its label 13 and its value 28.
  #[default]
  Md,
}

names!(StatCardSize {
  Sm => "sm",
  Md => "md",
});

/// The parts of a stat card a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StatCardPart {
  /// The card. What is pressed where it is pressed.
  Root,
  /// What the number is: "Frames per second".
  Label,
  /// The row of the value and the media beside it.
  Body,
  /// The number, as the app writes it: "144".
  Value,
  /// What stands at the end of the value's row: a sparkline.
  Media,
  /// The row of the change and what it is measured against.
  Footer,
  /// What the change is measured against: "vs 0.59".
  Description,
}

names!(StatCardPart {
  Root => "root",
  Label => "label",
  Body => "body",
  Value => "value",
  Media => "media",
  Footer => "footer",
  Description => "description",
});

/// The states of a stat card a recipe styles, besides what the pointer and the keyboard do to one that is pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StatCardState {
  /// It is pressed.
  Pressable,
  /// It has media beside its value.
  Media,
}

names!(StatCardState {
  Pressable => "pressable",
  Media => "media",
});

/// What builds a stat card's nodes: a theme's in place of the default one.
pub type StatCardRenderer = Arc<dyn Fn(StatCardView) -> Element + Send + Sync>;

/// A stat card: one number with what it is and how it moved. Frames per second, the users of a week, the time a
/// build takes.
///
/// ```
/// use lurq::{
///   app::theme::PaletteColor,
///   ui::{Sparkline, StatCard, StatCardSize, StatCardVariant},
/// };
///
/// let frames = StatCard::new("Frames per second", "144")
///   .delta("+12.5%")
///   .variant(StatCardVariant::Success)
///   .description("vs 0.59");
/// // With a sparkline beside its number, in the small size the board draws it in.
/// let trend = StatCard::new("Frames per second", "144")
///   .size(StatCardSize::Sm)
///   .media(Sparkline::new([118.0, 124.0, 121.0, 133.0, 144.0]).color(PaletteColor::Success));
/// ```
///
/// Its number is the app's, written as the app writes it; so is its change, a [`Badge`] in the
/// [`variant`](Self::variant)'s tone, and what that change is measured against. Beside its number it holds what the
/// app puts there with [`media`](Self::media): a [`Sparkline`](super::Sparkline) of the number's last values, as
/// the board draws one. A label or a value longer than its room wraps.
///
/// To tooling it is a `group` called by its label and described by its value, its change and what it is measured
/// against; with [`on_click`](Self::on_click) it is pressed as a card is, a `button`.
pub struct StatCard {
  label: Arc<str>,
  value: Arc<str>,
  delta: Option<Arc<str>>,
  description: Option<Arc<str>>,
  media: Option<Element>,
  variant: Name,
  size: Name,
  on_click: Vec<EventHandler<MouseEvent>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl StatCard {
  /// A card of the number `value` and what it is, `label`: `StatCard::new("Frames per second", "144")`.
  pub fn new(label: &str, value: &str) -> Self {
    Self {
      label: Arc::from(label),
      value: Arc::from(value),
      delta: None,
      description: None,
      media: None,
      variant: StatCardVariant::default().into(),
      size: StatCardSize::default().into(),
      on_click: Vec::new(),
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// How the number moved, as the app writes it: "+12.5%". Drawn as a badge in the variant's tone.
  pub fn delta(mut self, delta: &str) -> Self {
    self.delta = Some(Arc::from(delta));
    self
  }

  /// What the change is measured against: "vs 0.59", "since last week".
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// What stands at the end of the number's row: a [`Sparkline`](super::Sparkline) of its last values.
  pub fn media(mut self, media: impl Into<Element>) -> Self {
    self.media = Some(media.into());
    self
  }

  /// A [`StatCardVariant`], or a variant a theme adds: the tone of the change, whether it is for the better or the
  /// worse.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// A [`StatCardSize`], or a size the theme's recipe names: `Md`, the board's stat card, unless given.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// Called when the card is pressed, given the event, or when Enter or Space is pressed while it has the focus: a
  /// card with a handler is pressed and a stop of Tab. A second handler is called after the first.
  pub fn on_click(mut self, handler: impl IntoMouseEventHandler) -> Self {
    self.on_click.push(handler.into_event_handler());
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

  /// Styles one part of this card over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this card.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws a card that is pressed as it is under the pointer, held down or focused from the keyboard, with nothing
  /// pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the card.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the card in `theme`. Converting a card into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let pressable = !self.on_click.is_empty();
    let case = Case {
      variant: Some(self.variant),
      size: Some(self.size),
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(StatCardState::Pressable, pressable)
    .state(StatCardState::Media, self.media.is_some());
    let view = StatCardView {
      label: self.label,
      value: self.value,
      delta: self.delta,
      description: self.description,
      media: self.media,
      tone: self.variant,
      on_click: self.on_click,
      forced: self.forced,
      recipe: theme.resolve::<StatCard>(&case, Some(&self.rules)),
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<StatCard>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<StatCard> for Element {
  fn from(card: StatCard) -> Self {
    card.build(&current_theme())
  }
}

/// A stat card as a renderer sees it.
pub struct StatCardView {
  /// What the number is.
  pub label: Arc<str>,
  /// The number, as the app writes it.
  pub value: Arc<str>,
  /// How it moved.
  pub delta: Option<Arc<str>>,
  /// What the change is measured against.
  pub description: Option<Arc<str>>,
  media: Option<Element>,
  /// The tone of the change.
  pub tone: Name,
  on_click: Vec<EventHandler<MouseEvent>>,
  forced: Option<Interaction>,
  recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl StatCardView {
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// A text of `part` that wraps in the room it is given, in the color of its part.
  fn text(&self, part: StatCardPart, content: &str) -> Column {
    let part = self.part(part);
    let fill = Dimension::Pct(100.0);
    Column::new()
      .width(fill)
      .part(&part)
      .child(part.text(content).width(fill))
  }

  /// The label.
  pub fn label(&self) -> Column {
    self.text(StatCardPart::Label, &self.label)
  }

  /// The value, with the media at the end of its row where it has some.
  pub fn body(&mut self) -> Row {
    let value = Column::new()
      .flex(1.0)
      .min_width(Dimension::Px(0.0))
      .child(self.text(StatCardPart::Value, &self.value));
    let mut body = Row::new()
      .width(Dimension::Pct(100.0))
      .justify(Justify::SpaceBetween)
      .align_items(Alignment::End)
      .part(&self.part(StatCardPart::Body))
      .child(value);
    if let Some(media) = self.media.take() {
      body = body.child(Row::new().part(&self.part(StatCardPart::Media)).child(media));
    }
    body
  }

  /// The change as a badge in its tone and what it is measured against, where it has either.
  pub fn footer(&self) -> Option<Row> {
    if self.delta.is_none() && self.description.is_none() {
      return None;
    }
    let mut footer = Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(StatCardPart::Footer));
    if let Some(delta) = &self.delta {
      let variant = StatCardVariant::named(self.tone).unwrap_or_default().badge();
      footer = footer.child(Badge::new(delta).variant(variant).build(self.theme()));
    }
    if let Some(description) = &self.description {
      let part = self.part(StatCardPart::Description);
      footer = footer.child(Row::new().part(&part).child(part.text(description)));
    }
    Some(footer)
  }

  /// The card in the style of its part, holding `content`: a `group` to tooling called by its label and described
  /// by its value and its change; a button where it is pressed.
  pub fn root(&self, content: Vec<Element>) -> Element {
    let part = self.part(StatCardPart::Root);
    let mut said = self.value.to_string();
    if let Some(delta) = &self.delta {
      said.push_str(&format!(", {delta}"));
    }
    if let Some(description) = &self.description {
      said.push_str(&format!(" {description}"));
    }
    if self.on_click.is_empty() {
      let mut root = Column::new()
        .align_items(Alignment::Stretch)
        .part(&part)
        .describe("a11y_role", "group")
        .describe("a11y_name", self.label.clone())
        .describe("a11y_description", said);
      if let Some(id) = &self.id {
        root = root.id(id.clone());
      }
      if let Some(key) = &self.key {
        root = root.key(key.clone());
      }
      return root.with_children(content).into();
    }
    let handlers = self.on_click.clone();
    let mut button = pressable(&part, false)
      .justify(Justify::Start)
      .align_items(Alignment::Stretch)
      .describe("a11y_role", "button")
      .describe("a11y_name", self.label.clone())
      .describe("a11y_description", said)
      .on_click(move |event: MouseEvent| {
        for handler in &handlers {
          handler.call(&event);
        }
      });
    if let Some(id) = &self.id {
      button = button.id(id.clone());
    }
    if let Some(key) = &self.key {
      button = button.key(key.clone());
    }
    button
      .child(
        Column::new()
          .width(Dimension::Pct(100.0))
          .align_items(Alignment::Stretch)
          .spacing(part.rest().gap.unwrap_or_default())
          .with_children(content),
      )
      .into()
  }

  /// How the default renderer builds a stat card: its label, its value and its change, one under another.
  pub fn render(mut self) -> Element {
    let mut content: Vec<Element> = vec![self.label().into(), self.body().into()];
    content.extend(self.footer().map(Into::into));
    self.root(content)
  }
}

impl Themed for StatCard {
  const NAME: &'static str = "stat_card";
  type Renderer = StatCardRenderer;

  /// The design's stat card (`pencil/lurq-design-system.pen`, Stat Card, in Charts' Examples): a card (the card's
  /// fill, an edge of 1 in the border's color, the extra large radius, the small shadow) 200 on the board and as
  /// wide as what holds it here, 20 in, its label, its value and its change 8 apart. The label is 13 in the muted
  /// color, the value 28 at 600 in the card's foreground, 0.5 closer, and the change a badge in its tone 8 from what
  /// it is measured against, the mono type at 11 in the muted color.
  ///
  /// The small size is the card the board draws around its sparkline: 16 in, the label 12, the value 24, 0.4
  /// closer, the media at the end of the value's row, at its foot. A card that is pressed has the border's ring
  /// under the pointer and a ring of 2 in the ring color from the keyboard, as a card has.
  fn recipe() -> Recipe {
    use PaletteColor::{Card, CardForeground, MutedForeground, Ring};
    use StatCardPart::{Body, Description, Footer, Label, Root, Value};
    use StatCardSize::Sm;
    use StatCardState::Pressable;
    let edge = |color: PaletteColor| Edge::inside(BorderSize::Sm, color);
    Recipe::new()
      .base(Root, |s| {
        fading(
          s.width(Length::Fill)
            .background(Card)
            .foreground(CardForeground)
            .border(edge(PaletteColor::Border))
            .radius(RadiusSize::Xl)
            .shadow(ShadowStyle::Sm)
            .padding_x(20.0)
            .padding_y(20.0)
            .gap(8.0),
        )
      })
      .size(Sm, Root, |s| s.padding_x(16.0).padding_y(16.0))
      .state(Pressable, Root, |s| s.cursor(CursorIcon::Pointer))
      .when(Selector::new().state(Pressable).hovered(), Root, |s| {
        s.border(edge(Ring)).shadow(ShadowStyle::Md)
      })
      .focus_visible(Root, |s| s.outline(Outline::new(BorderSize::Md, Ring).offset(2.0)))
      .base(Label, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .foreground(MutedForeground)
      })
      .size(Sm, Label, |s| s.font_size(12.0))
      .base(Body, |s| s.gap(12.0))
      .base(Value, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(28.0)
          .weight(FontWeight::SemiBold)
          .letter_spacing(-0.5)
          .foreground(CardForeground)
      })
      .size(Sm, Value, |s| s.font_size(24.0).letter_spacing(-0.4))
      .base(Footer, |s| s.gap(8.0))
      .base(Description, |s| {
        s.font_family(FontFamily::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
  }
}
