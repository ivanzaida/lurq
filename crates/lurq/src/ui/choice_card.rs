//! A checkbox card and a radio card: a checkbox and a radio whose whole card is pressed, and the groups of them.

use std::sync::Arc;

use super::{
  CheckboxPart, CheckboxState, RadioPart, RadioState, icons, names,
  part_style::{Length, PartStyle, PartTransition},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{Chosen, fading, glyph, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  animation::Transition,
  app::theme::{BorderSize, IconSize, MotionDuration, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  components::{self, Column, Row},
  core::{Signal, SignalValue},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, FocusGroup, border::Border, dimension::Dimension},
};

/// The parts of a checkbox card or a radio card a recipe styles. The box of a checkbox card is the
/// [`Checkbox`](super::Checkbox)'s own box and mark, and the mark of a radio card the [`Radio`](super::Radio)'s own
/// mark and dot: a theme that styles those styles them here too.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChoiceCardPart {
  /// The card: its fill, its edge, its corners. What is pressed and focused.
  Root,
  /// The icon before the text, where the card has one.
  Icon,
  /// The heading over the description.
  Text,
  /// The title, and the price at its end.
  Heading,
  /// What the choice is, in a word or a line: what the card is called.
  Title,
  /// What the choice costs, at the end of the title.
  Price,
  /// More of it, under the title, in the muted color.
  Description,
}

names!(ChoiceCardPart {
  Root => "root",
  Icon => "icon",
  Text => "text",
  Heading => "heading",
  Title => "title",
  Price => "price",
  Description => "description",
});

/// The states of a checkbox card or a radio card a recipe styles, besides what the pointer and the keyboard do to
/// it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChoiceCardState {
  /// It is checked, or it is the choice made: its edge is the primary color.
  Chosen,
  /// What it is part of was answered wrongly, or not at all.
  Invalid,
  /// It takes no input.
  Disabled,
  /// It is as wide as what holds it.
  FullWidth,
}

names!(ChoiceCardState {
  Chosen => "chosen",
  Invalid => "invalid",
  Disabled => "disabled",
  FullWidth => "full_width",
});

/// Which of the two a card is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChoiceCardKind {
  /// A [`CheckboxCard`]: a choice made or not, among others a form takes at once.
  Checkbox,
  /// A [`RadioCard`]: one of a few choices that exclude each other.
  Radio,
}

/// What builds a checkbox card's or a radio card's nodes: a theme's in place of the default one.
pub type ChoiceCardRenderer = Arc<dyn Fn(ChoiceCardView) -> Element + Send + Sync>;

/// What a checkbox card and a radio card are both given.
struct Content {
  title: Arc<str>,
  description: Option<Arc<str>>,
  icon: Option<Name>,
  price: Option<Arc<str>>,
  invalid: bool,
  disabled: bool,
  full_width: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Content {
  fn new(title: &str) -> Self {
    Self {
      title: Arc::from(title),
      description: None,
      icon: None,
      price: None,
      invalid: false,
      disabled: false,
      full_width: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// The view of a card of `kind` made of this, in `theme`, with the style of the box or the mark it holds from
  /// the recipe of `C`.
  fn view<C: Themed>(
    self,
    theme: &Theme,
    kind: ChoiceCardKind,
    chosen: bool,
    press: Option<Press>,
    indicator: Case,
  ) -> ChoiceCardView {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(ChoiceCardState::Chosen, chosen)
    .state(ChoiceCardState::Invalid, self.invalid)
    .state(ChoiceCardState::Disabled, self.disabled)
    .state(ChoiceCardState::FullWidth, self.full_width);
    let recipe = match kind {
      ChoiceCardKind::Checkbox => theme.resolve::<CheckboxCard>(&case, Some(&self.rules)),
      ChoiceCardKind::Radio => theme.resolve::<RadioCard>(&case, Some(&self.rules)),
    };
    ChoiceCardView {
      kind,
      title: self.title,
      description: self.description,
      icon: self.icon,
      price: self.price,
      chosen,
      invalid: self.invalid,
      disabled: self.disabled,
      recipe,
      indicator: theme.resolve::<C>(&indicator.breakpoint(super::current_breakpoint()), None),
      forced: self.forced,
      press: press.filter(|_| !self.disabled),
      id: self.id,
      key: self.key,
    }
  }
}

/// What a press of a card does.
type Press = Arc<dyn Fn() + Send + Sync>;

/// The props a checkbox card and a radio card both have.
macro_rules! card_props {
  ($card:ident) => {
    impl $card {
      /// More of it, under the title, in the muted color: what the card is described by to what reads the tree.
      pub fn description(mut self, description: &str) -> Self {
        self.content.description = Some(Arc::from(description));
        self
      }

      /// An icon before the text, by its name in the theme's set.
      pub fn icon(mut self, name: impl Into<Name>) -> Self {
        self.content.icon = Some(name.into());
        self
      }

      /// What the choice costs, at the end of the title: "$12 / month".
      pub fn price(mut self, price: &str) -> Self {
        self.content.price = Some(Arc::from(price));
        self
      }

      /// Whether what the card is part of was answered wrongly, or not at all: its edge is the destructive color.
      pub fn invalid(mut self, invalid: bool) -> Self {
        self.content.invalid = invalid;
        self
      }

      /// Whether the card takes no input: it is dimmed and is not pressed.
      pub fn disabled(mut self, disabled: bool) -> Self {
        self.content.disabled = disabled;
        self
      }

      /// Whether the card is as wide as what holds it, in place of the 220 the board gives it.
      pub fn full_width(mut self, full_width: bool) -> Self {
        self.content.full_width = full_width;
        self
      }

      /// Puts the card in a state the theme's recipe names, or takes it out.
      pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
        let state = state.into();
        self.content.states.retain(|held| *held != state);
        if on {
          self.content.states.push(state);
        }
        self
      }

      /// Styles one part of this card over what the theme gives it.
      pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
        self.content.rules = self.content.rules.base(part, style);
        self
      }

      /// Lays rules over the theme's recipe for this card: what the group it is in says of it.
      pub fn rules(mut self, rules: &Recipe) -> Self {
        self.content.rules = self.content.rules.extend(rules);
        self
      }

      /// Draws the card as it is under the pointer, held down or focused from the keyboard, with nothing pointing
      /// at it: for a gallery or a test.
      pub fn force(mut self, interaction: Option<Interaction>) -> Self {
        self.content.forced = interaction;
        self
      }

      /// The id of the card: what is pressed.
      pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
        self.content.id = Some(id.into());
        self
      }

      pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
        self.content.key = Some(key.into());
        self
      }
    }
  };
}

/// A checkbox card: a checkbox whose whole card is pressed, with a title, a description and an icon or a price.
/// The add-ons of a plan, the parts of an install, the reports a team is sent.
///
/// ```
/// use lurq::ui::{CheckboxCard, icons};
///
/// let analytics = CheckboxCard::new("Analytics")
///   .description("Share anonymous usage data.")
///   .checked(true)
///   .on_change(|checked| println!("{checked}"));
/// let backups = CheckboxCard::new("Backups").description("Every night, kept for 30 days.").price("$4 / month");
/// let sync = CheckboxCard::new("Sync").icon(icons::REFRESH);
/// ```
///
/// The card is one stop of Tab, and a click of it or Space presses it: the box in it is drawn and takes no focus
/// of its own. A checkbox card does not keep whether it is checked: the app says so with [`CheckboxCard::checked`],
/// and is told of a press with [`CheckboxCard::on_change`]. Cards over one list are a [`CheckboxCardGroup`].
pub struct CheckboxCard {
  content: Content,
  checked: bool,
  on_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
}

impl CheckboxCard {
  /// A checkbox card called `title`.
  pub fn new(title: &str) -> Self {
    Self {
      content: Content::new(title),
      checked: false,
      on_change: Vec::new(),
    }
  }

  /// Whether the card is checked: its box is, and its edge is the primary color.
  pub fn checked(mut self, checked: bool) -> Self {
    self.checked = checked;
    self
  }

  /// Called on a press, with what the card is to be: checked if it was not.
  pub fn on_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Builds the card in `theme`. Converting a card into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let (handlers, next) = (self.on_change, !self.checked);
    let press: Option<Press> = (!handlers.is_empty()).then(|| {
      Arc::new(move || {
        for handler in &handlers {
          handler(next);
        }
      }) as Press
    });
    let content = &self.content;
    let indicator = Case::new()
      .state(CheckboxState::Checked, self.checked)
      .state(CheckboxState::Invalid, content.invalid)
      .state(CheckboxState::Disabled, content.disabled);
    let view = self
      .content
      .view::<super::Checkbox>(theme, ChoiceCardKind::Checkbox, self.checked, press, indicator);
    match theme.renderer::<CheckboxCard>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

card_props!(CheckboxCard);

impl From<CheckboxCard> for Element {
  fn from(card: CheckboxCard) -> Self {
    card.build(&current_theme())
  }
}

/// A radio card: a radio whose whole card is pressed, with a title, a description and an icon or a price. The
/// plan of an account, the density of a list, the region of a server.
///
/// ```
/// use lurq::ui::RadioCard;
///
/// let comfortable = RadioCard::new("Comfortable").description("Roomy rows, 36px controls.").chosen(true);
/// let team = RadioCard::new("Team").price("$12 / month").on_choose(|| println!("team"));
/// ```
///
/// The card is pressed by a click, and by Space or Enter while it has the focus; the mark in it is drawn and takes
/// no focus of its own. A radio card does not keep whether it is chosen: the app says so with
/// [`RadioCard::chosen`], and is told with [`RadioCard::on_choose`] that it was pressed while it was not. Cards that
/// are one choice are a [`RadioCardGroup`], which the arrow keys move through.
pub struct RadioCard {
  content: Content,
  chosen: bool,
  on_choose: Vec<Press>,
}

impl RadioCard {
  /// A radio card called `title`.
  pub fn new(title: &str) -> Self {
    Self {
      content: Content::new(title),
      chosen: false,
      on_choose: Vec::new(),
    }
  }

  /// Whether the card is the choice made: the dot in its mark, and its edge in the primary color.
  pub fn chosen(mut self, chosen: bool) -> Self {
    self.chosen = chosen;
    self
  }

  /// Called on a press of the card while it is not chosen: it is to be.
  pub fn on_choose(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_choose.push(Arc::new(handler));
    self
  }

  /// Builds the card in `theme`. Converting a card into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let handlers = self.on_choose;
    let press: Option<Press> = (!handlers.is_empty() && !self.chosen).then(|| {
      Arc::new(move || {
        for handler in &handlers {
          handler();
        }
      }) as Press
    });
    let content = &self.content;
    let indicator = Case::new()
      .state(RadioState::Chosen, self.chosen)
      .state(RadioState::Invalid, content.invalid)
      .state(RadioState::Disabled, content.disabled);
    let view = self
      .content
      .view::<super::Radio>(theme, ChoiceCardKind::Radio, self.chosen, press, indicator);
    match theme.renderer::<RadioCard>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

card_props!(RadioCard);

impl From<RadioCard> for Element {
  fn from(card: RadioCard) -> Self {
    card.build(&current_theme())
  }
}

/// A checkbox card or a radio card as a renderer sees it: what it was given, the style of its parts, and its
/// behavior ready to be dressed.
pub struct ChoiceCardView {
  pub kind: ChoiceCardKind,
  pub title: Arc<str>,
  pub description: Option<Arc<str>>,
  pub icon: Option<Name>,
  pub price: Option<Arc<str>>,
  /// Whether it is checked, or the choice made.
  pub chosen: bool,
  pub invalid: bool,
  pub disabled: bool,
  /// The style of each part of this card.
  pub recipe: ResolvedRecipe,
  /// The style of the parts of the checkbox or the radio whose box or mark the card holds.
  pub indicator: ResolvedRecipe,
  forced: Option<Interaction>,
  press: Option<Press>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ChoiceCardView {
  /// The style of a part, as this card is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The card's behavior with the style of its root: the element that is pressed, focused and disabled, with
  /// nothing in it yet. It is a checkbox or a radio to what reads the tree, called by the title, described by the
  /// description, and says whether it is checked.
  pub fn root(&self) -> components::Button {
    let role = match self.kind {
      ChoiceCardKind::Checkbox => "checkbox",
      ChoiceCardKind::Radio => "radio",
    };
    let mut root = pressable(&self.part(ChoiceCardPart::Root), self.disabled)
      .justify(Justify::Start)
      .align_items(Alignment::Start)
      .describe("a11y_role", role)
      .describe("a11y_name", self.title.clone())
      .describe("a11y_checked", if self.chosen { "true" } else { "false" });
    if let Some(description) = &self.description {
      root = root.describe("a11y_description", description.clone());
    }
    if self.kind == ChoiceCardKind::Radio {
      // In a group, which Tab stops at once, the chosen card is where Tab enters.
      root = root.focus_entry(self.chosen);
    }
    if let Some(press) = self.press.clone() {
      root = root.on_click(move |_| press());
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// The icon in the style of its part, where the card has one.
  pub fn icon(&self) -> Option<Element> {
    let name = self.icon.clone()?;
    Some(glyph(&self.part(ChoiceCardPart::Icon), name, false))
  }

  /// The title with the price at its end, over the description, as wide as the room the card leaves them.
  pub fn text(&self) -> Column {
    let title = wrapping(&self.part(ChoiceCardPart::Title), &self.title).flex(1.0);
    let mut heading = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Start)
      .part(&self.part(ChoiceCardPart::Heading))
      .child(title);
    if let Some(price) = &self.price {
      let part = self.part(ChoiceCardPart::Price);
      heading = heading.child(Row::new().part(&part).child(part.text(price).nowrap()));
    }
    let mut text = Column::new()
      .flex(1.0)
      .min_width(Dimension::Px(0.0))
      .part(&self.part(ChoiceCardPart::Text))
      .child(heading);
    if let Some(description) = &self.description {
      text = text.child(wrapping(&self.part(ChoiceCardPart::Description), description));
    }
    text
  }

  /// The box of a checkbox card with its check while it is checked, or the mark of a radio card with its dot while
  /// it is chosen: the checkbox's and the radio's own parts, drawn, and not pressed or focused on their own.
  pub fn control(&self) -> Element {
    let held = |part: ResolvedPart| {
      Row::new()
        .justify(Justify::Center)
        .align_items(Alignment::Center)
        .part(&part)
    };
    match self.kind {
      ChoiceCardKind::Checkbox => {
        let mut square = held(self.indicator.part(CheckboxPart::Box));
        if self.chosen {
          square = square.child(glyph(
            &self.indicator.part(CheckboxPart::Indicator),
            Name::new(icons::CHECK),
            false,
          ));
        }
        square.into()
      }
      ChoiceCardKind::Radio => {
        let mut mark = held(self.indicator.part(RadioPart::Mark));
        if self.chosen {
          mark = mark.child(Row::new().part(&self.indicator.part(RadioPart::Dot)));
        }
        mark.into()
      }
    }
  }

  /// How the default renderer builds a card: the icon, the text and the box or the mark at the end, side by side
  /// on the card.
  pub fn render(self) -> Element {
    let mut root = self.root();
    if let Some(icon) = self.icon() {
      root = root.child(icon);
    }
    root.child(self.text()).child(self.control()).into()
  }
}

/// A text of a card in the style of `part`, as wide as the room it has: a longer one goes on to the next line.
fn wrapping(part: &ResolvedPart, content: &str) -> Column {
  let fill = Dimension::Pct(100.0);
  Column::new()
    .width(fill)
    .min_width(Dimension::Px(0.0))
    .part(part)
    .child(part.text(content).width(fill))
}

/// The design's checkbox card and radio card (`pencil/lurq-design-system.pen`, Checkbox Card and Radio Card): 220
/// wide, padded 16, in the background's color with corners of `radius-lg`. The text is first, the title in 14 and
/// medium over the description 4 under it in 13 of the muted color, and the box or the mark is 12 after it, at the
/// top. Not chosen, the edge inside it is 1 of an input's edge; chosen, 2 of the primary color. Under the pointer
/// the edge of one not chosen is the ring's color; the ring of the focus is around the card. An invalid one is
/// edged in the destructive color, and a disabled one dimmed.
fn card_recipe() -> Recipe {
  use ChoiceCardPart::{Description, Heading, Icon, Price, Root, Text, Title};
  use ChoiceCardState::{Chosen, Disabled, FullWidth, Invalid};
  use PaletteColor::{Background, Destructive, Foreground, Input, MutedForeground, Primary, Ring};
  let edge = |size: BorderSize, color: PaletteColor| Border::inside(size, color);
  let chosen = || Selector::new().state(Chosen);
  let invalid = || Selector::new().state(Invalid);
  Recipe::new()
    .base(Root, |s| {
      fading(
        s.width(220.0)
          .padding_x(16.0)
          .padding_y(16.0)
          .gap(12.0)
          .background(Background)
          .foreground(Foreground)
          .radius(RadiusSize::Lg)
          .border(edge(BorderSize::Sm, Input))
          .cursor(CursorIcon::Pointer),
      )
      .transition(PartTransition::new(Transition::border_color(), MotionDuration::Fast))
    })
    .base(Icon, |s| s.square(IconSize::Lg).foreground(Foreground))
    .base(Text, |s| s.gap(4.0))
    .base(Heading, |s| s.gap(8.0))
    .base(Title, |s| {
      s.typography(TypographyStyle::Body)
        .weight(FontWeight::Medium)
        .foreground(Foreground)
    })
    .base(Price, |s| {
      s.typography(TypographyStyle::Body)
        .weight(FontWeight::SemiBold)
        .foreground(Foreground)
    })
    .base(Description, |s| {
      s.typography(TypographyStyle::Description)
        .font_size(13.0)
        .line_height(1.4)
        .foreground(MutedForeground)
    })
    .state(FullWidth, Root, |s| s.width(Length::Fill))
    .hovered(Root, |s| s.border(edge(BorderSize::Sm, Ring)))
    .focus_visible(Root, |s| s.focus_ring())
    // The pointer does not take from a card the edge that says it is chosen, or that it is wrong.
    .state(Chosen, Root, |s| s.border(edge(BorderSize::Md, Primary)))
    .when(chosen().hovered(), Root, |s| s.border(edge(BorderSize::Md, Primary)))
    .state(Invalid, Root, |s| s.border(edge(BorderSize::Sm, Destructive)))
    .when(invalid().hovered(), Root, |s| s.border(edge(BorderSize::Sm, Destructive)))
    .when(chosen().state(Invalid), Root, |s| {
      s.border(edge(BorderSize::Md, Destructive))
    })
    .when(chosen().state(Invalid).hovered(), Root, |s| {
      s.border(edge(BorderSize::Md, Destructive))
    })
    .state(Disabled, Root, |s| {
      s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
    })
}

impl Themed for CheckboxCard {
  const NAME: &'static str = "checkbox_card";
  type Renderer = ChoiceCardRenderer;

  /// The design's checkbox card: see [`RadioCard`]'s, which it shares. Its box is the checkbox's.
  fn recipe() -> Recipe {
    card_recipe()
  }
}

impl Themed for RadioCard {
  const NAME: &'static str = "radio_card";
  type Renderer = ChoiceCardRenderer;

  /// The design's radio card (`pencil/lurq-design-system.pen`, Radio Card), as its checkbox card is: 220 wide,
  /// padded 16, in the background's color with corners of `radius-lg`, the text first and the mark 12 after it at
  /// the top. Not chosen, the edge inside it is 1 of an input's edge; chosen, 2 of the primary color. Its mark is
  /// the radio's.
  fn recipe() -> Recipe {
    card_recipe()
  }
}

/// The parts of a group of checkbox cards or radio cards a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChoiceCardGroupPart {
  /// The group: the cards one under the other, or side by side.
  Root,
  /// What the group says of each card in it: laid over the theme's recipe for a card's root.
  Item,
}

names!(ChoiceCardGroupPart {
  Root => "root",
  Item => "item",
});

/// The states of a group of checkbox cards or radio cards a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChoiceCardGroupState {
  /// Its cards stand side by side.
  Horizontal,
}

names!(ChoiceCardGroupState {
  Horizontal => "horizontal",
});

/// What a group of cards of either kind is given, and how it lays them out.
struct Group {
  label: Option<Arc<str>>,
  horizontal: bool,
  invalid: bool,
  disabled: bool,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Group {
  fn new() -> Self {
    Self {
      label: None,
      horizontal: false,
      invalid: false,
      disabled: false,
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// The group's recipe from the theme's for `C`, and what it says of each card in it.
  fn resolve<C: Themed>(&self, theme: &Theme) -> (ResolvedRecipe, Recipe) {
    let case = Case::new()
      .breakpoint(super::current_breakpoint())
      .state(ChoiceCardGroupState::Horizontal, self.horizontal);
    let recipe = theme.resolve::<C>(&case, Some(&self.rules));
    let item = recipe.rules_for(ChoiceCardGroupPart::Item, ChoiceCardPart::Root);
    (recipe, item)
  }

  /// The cards laid out one under the other, or side by side, as `role` to what reads the tree.
  fn hold(self, recipe: &ResolvedRecipe, cards: Vec<Element>, role: &'static str, keys: Option<FocusGroup>) -> Element {
    let root = recipe.part(ChoiceCardGroupPart::Root);
    let (id, key, label) = (self.id, self.key, self.label);
    macro_rules! group {
      ($holder:expr) => {{
        let mut holder = $holder.part(&root).describe("a11y_role", role);
        if let Some(keys) = keys {
          holder = holder.focus_group(keys);
        }
        if let Some(label) = label {
          holder = holder.describe("a11y_name", label);
        }
        for card in cards {
          holder = holder.child(card);
        }
        if let Some(id) = id {
          holder = holder.id(id);
        }
        if let Some(key) = key {
          holder = holder.key(key);
        }
        holder.into()
      }};
    }
    if self.horizontal {
      // Cards side by side are as high as the highest in their line.
      group!(Row::new().wrap().align_items(Alignment::Stretch))
    } else {
      group!(Column::new().align_items(Alignment::Stretch))
    }
  }
}

/// The props the two groups both have.
macro_rules! group_props {
  ($group:ident) => {
    impl $group {
      /// Whether the cards stand side by side, going on to the next line where there is no room, in place of one
      /// under the other.
      pub fn horizontal(mut self, horizontal: bool) -> Self {
        self.group.horizontal = horizontal;
        self
      }

      /// What the group is called, to what reads the tree. It draws nothing: a field or a fieldset gives it its
      /// own label.
      pub fn label(mut self, label: &str) -> Self {
        self.group.label = Some(Arc::from(label));
        self
      }

      /// Whether what is chosen is not accepted: every card's edge says so.
      pub fn invalid(mut self, invalid: bool) -> Self {
        self.group.invalid = invalid;
        self
      }

      /// Whether the whole group takes no input.
      pub fn disabled(mut self, disabled: bool) -> Self {
        self.group.disabled = disabled;
        self
      }

      /// Styles one part of this group over what the theme gives it.
      pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
        self.group.rules = self.group.rules.base(part, style);
        self
      }

      pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
        self.group.id = Some(id.into());
        self
      }

      pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
        self.group.key = Some(key.into());
        self
      }
    }
  };
}

/// The group's recipe for either kind: cards 12 apart, one under the other or side by side.
fn group_recipe() -> Recipe {
  Recipe::new().base(ChoiceCardGroupPart::Root, |s| s.gap(12.0))
}

/// Radio cards that are one choice: the one whose value the app holds is chosen, and a press of another chooses
/// it.
///
/// ```
/// use lurq::{core::Signal, ui::{RadioCard, RadioCardGroup}};
///
/// # fn groups(density: Signal<u8>) {
/// let density = RadioCardGroup::new(density, [
///   (0, RadioCard::new("Comfortable").description("Roomy rows, 36px controls.")),
///   (1, RadioCard::new("Compact").description("Dense rows, 28px controls.")),
///   (2, RadioCard::new("Spacious").description("For touch.").disabled(true)),
/// ]);
/// # }
/// ```
///
/// Each card is given its value; what the group holds says which is chosen, and the group tells each card what it
/// says of all of them (invalid, disabled). The group is one stop of Tab, at the chosen card or the first; the arrow
/// keys move the focus to the next card and choose it, Home and End the first and the last, and a disabled one is
/// passed by, as in a [`RadioGroup`](super::RadioGroup).
pub struct RadioCardGroup {
  cards: Vec<(RadioCard, bool, Press)>,
  group: Group,
}

impl RadioCardGroup {
  fn of<T: Clone + PartialEq + Send + Sync + 'static>(
    chosen: Chosen<T>,
    cards: impl IntoIterator<Item = (T, RadioCard)>,
  ) -> Self {
    let cards = cards
      .into_iter()
      .map(|(value, card)| (card, chosen.has(&value), Arc::new(chosen.picker(value)) as Press))
      .collect();
    Self {
      cards,
      group: Group::new(),
    }
  }

  /// A group of which the card whose value the signal has is chosen. A press of another sets the signal.
  pub fn new<T>(value: Signal<T>, cards: impl IntoIterator<Item = (T, RadioCard)>) -> Self
  where
    T: Clone + PartialEq + Send + Sync + SignalValue + 'static,
  {
    Self::of(Chosen::one(value), cards)
  }

  /// A group whose choice the app holds: the card of `chosen` is chosen, none if it is `None`, and `choose` is told
  /// the value of each other one pressed.
  pub fn controlled<T: Clone + PartialEq + Send + Sync + 'static>(
    chosen: Option<T>,
    cards: impl IntoIterator<Item = (T, RadioCard)>,
    choose: impl Fn(T) + Send + Sync + 'static,
  ) -> Self {
    Self::of(Chosen::held(chosen.into_iter().collect(), choose), cards)
  }

  /// Builds the group in `theme`. Converting a group into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let (recipe, item) = self.group.resolve::<RadioCardGroup>(theme);
    let (invalid, disabled) = (self.group.invalid, self.group.disabled);
    let cards = self
      .cards
      .into_iter()
      .map(|(card, chosen, pick)| {
        let (own_invalid, own_disabled) = (card.content.invalid, card.content.disabled);
        card
          .chosen(chosen)
          .invalid(invalid || own_invalid)
          .disabled(disabled || own_disabled)
          .rules(&item)
          .on_choose(move || pick())
          .build(theme)
      })
      .collect();
    // One choice, as a group of radios is: Tab stops in it once, and an arrow key moves the focus and the choice.
    let keys = FocusGroup::both().activate(true).one_stop(true);
    self.group.hold(&recipe, cards, "radiogroup", Some(keys))
  }
}

group_props!(RadioCardGroup);

impl From<RadioCardGroup> for Element {
  fn from(group: RadioCardGroup) -> Self {
    group.build(&current_theme())
  }
}

impl Themed for RadioCardGroup {
  const NAME: &'static str = "radio_card_group";
  type Renderer = Arc<dyn Fn(RadioCardGroup) -> Element + Send + Sync>;

  /// The design's radio cards: 12 apart, one under the other or side by side.
  fn recipe() -> Recipe {
    group_recipe()
  }
}

/// Checkbox cards over one list: those whose values the app holds are checked, and a press of one adds its value
/// or takes it out.
///
/// ```
/// use lurq::{core::Signal, ui::{CheckboxCard, CheckboxCardGroup}};
///
/// # fn groups(add_ons: Signal<Vec<&'static str>>) {
/// let add_ons = CheckboxCardGroup::new(add_ons, [
///   ("backups", CheckboxCard::new("Backups").price("$4 / month")),
///   ("support", CheckboxCard::new("Priority support").price("$20 / month")),
/// ])
/// .horizontal(true);
/// # }
/// ```
///
/// Each card is its own stop of Tab, and Space or Enter presses the one the focus is on.
pub struct CheckboxCardGroup {
  cards: Vec<(CheckboxCard, bool, Press)>,
  group: Group,
}

impl CheckboxCardGroup {
  fn of<T: Clone + PartialEq + Send + Sync + 'static>(
    chosen: Chosen<T>,
    cards: impl IntoIterator<Item = (T, CheckboxCard)>,
  ) -> Self {
    let cards = cards
      .into_iter()
      .map(|(value, card)| (card, chosen.has(&value), Arc::new(chosen.picker(value)) as Press))
      .collect();
    Self {
      cards,
      group: Group::new(),
    }
  }

  /// A group of which the cards whose values the signal has are checked. A press adds a value to the signal, or
  /// takes it out.
  pub fn new<T>(values: Signal<Vec<T>>, cards: impl IntoIterator<Item = (T, CheckboxCard)>) -> Self
  where
    T: Clone + PartialEq + Send + Sync + 'static,
    Vec<T>: SignalValue,
  {
    Self::of(Chosen::many(values), cards)
  }

  /// A group whose state the app holds: the cards of `checked` are checked, and `press` is told the value of each
  /// one pressed.
  pub fn controlled<T: Clone + PartialEq + Send + Sync + 'static>(
    checked: impl IntoIterator<Item = T>,
    cards: impl IntoIterator<Item = (T, CheckboxCard)>,
    press: impl Fn(T) + Send + Sync + 'static,
  ) -> Self {
    Self::of(Chosen::held(checked.into_iter().collect(), press), cards)
  }

  /// Builds the group in `theme`. Converting a group into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let (recipe, item) = self.group.resolve::<CheckboxCardGroup>(theme);
    let (invalid, disabled) = (self.group.invalid, self.group.disabled);
    let cards = self
      .cards
      .into_iter()
      .map(|(card, checked, pick)| {
        let (own_invalid, own_disabled) = (card.content.invalid, card.content.disabled);
        card
          .checked(checked)
          .invalid(invalid || own_invalid)
          .disabled(disabled || own_disabled)
          .rules(&item)
          .on_change(move |_| pick())
          .build(theme)
      })
      .collect();
    self.group.hold(&recipe, cards, "group", None)
  }
}

group_props!(CheckboxCardGroup);

impl From<CheckboxCardGroup> for Element {
  fn from(group: CheckboxCardGroup) -> Self {
    group.build(&current_theme())
  }
}

impl Themed for CheckboxCardGroup {
  const NAME: &'static str = "checkbox_card_group";
  type Renderer = Arc<dyn Fn(CheckboxCardGroup) -> Element + Send + Sync>;

  /// The design's checkbox cards: 12 apart, one under the other or side by side.
  fn recipe() -> Recipe {
    group_recipe()
  }
}

impl super::FieldControl for CheckboxCard {
  /// Invalid where the field is, and disabled where the field is. It keeps its title as its name: it is drawn.
  fn in_field(mut self, field: &super::InField) -> Element {
    self.content.invalid |= field.judgment == Some(super::Judgment::Invalid);
    self.content.disabled |= field.disabled;
    self.into()
  }
}

impl super::FieldControl for RadioCard {
  /// Invalid where the field is, and disabled where the field is. It keeps its title as its name: it is drawn.
  fn in_field(mut self, field: &super::InField) -> Element {
    self.content.invalid |= field.judgment == Some(super::Judgment::Invalid);
    self.content.disabled |= field.disabled;
    self.into()
  }
}

impl super::FieldControl for CheckboxCardGroup {
  /// Called by the field's label unless it has a name of its own, every card invalid where the field is, and
  /// disabled where the field is.
  fn in_field(mut self, field: &super::InField) -> Element {
    if self.group.label.is_none() {
      self.group.label = Some(field.label.clone());
    }
    self.group.invalid |= field.judgment == Some(super::Judgment::Invalid);
    self.group.disabled |= field.disabled;
    self.into()
  }
}

impl super::FieldControl for RadioCardGroup {
  /// Called by the field's label unless it has a name of its own, every card invalid where the field is, and
  /// disabled where the field is.
  fn in_field(mut self, field: &super::InField) -> Element {
    if self.group.label.is_none() {
      self.group.label = Some(field.label.clone());
    }
    self.group.invalid |= field.judgment == Some(super::Judgment::Invalid);
    self.group.disabled |= field.disabled;
    self.into()
  }
}
