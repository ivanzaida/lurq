//! A checkbox card and a radio card: a checkbox and a radio whose whole card is pressed, and the groups of them.

use std::sync::Arc;

use super::{
  CheckboxPart, CheckboxState, RadioPart, RadioState,
  checkbox::{Turn, toggling, turning},
  icons, names,
  part_style::{Length, PartStyle, PartTransition},
  radio::{RadioSelection, selecting},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{Orientation, fading, glyph, pressable},
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
  /// What the app puts under the description: a picker, the steps of what is chosen.
  Content,
  /// Why the card cannot be chosen, under the description.
  Reason,
  /// The lock that stands in place of the box or the mark of a card that cannot be chosen, for a reason.
  Lock,
}

names!(ChoiceCardPart {
  Root => "root",
  Icon => "icon",
  Text => "text",
  Heading => "heading",
  Title => "title",
  Price => "price",
  Description => "description",
  Content => "content",
  Reason => "reason",
  Lock => "lock",
});

/// The states of a checkbox card or a radio card a recipe styles, besides what the pointer and the keyboard do to
/// it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChoiceCardState {
  /// A checkbox card that is checked: its edge is the primary color.
  Checked,
  /// A radio card that is the one of several selected: its edge is the primary color.
  Selected,
  /// What it is part of was answered wrongly, or not at all.
  Invalid,
  /// It takes no input.
  Disabled,
  /// It is as wide as what holds it.
  FullWidth,
  /// What it chose is under way: it takes no press, and keeps its box or its mark.
  Loading,
}

names!(ChoiceCardState {
  Checked => "checked",
  Selected => "selected",
  Invalid => "invalid",
  Disabled => "disabled",
  FullWidth => "full_width",
  Loading => "loading",
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
  mark: Option<Element>,
  content: Vec<Element>,
  reason: Option<Arc<str>>,
  loading: bool,
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
      mark: None,
      content: Vec::new(),
      reason: None,
      loading: false,
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
    on: bool,
    press: Option<Press>,
    indicator: Case,
  ) -> ChoiceCardView {
    let (checked, selected) = match kind {
      ChoiceCardKind::Checkbox => (on, false),
      ChoiceCardKind::Radio => (false, on),
    };
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(ChoiceCardState::Checked, checked)
    .state(ChoiceCardState::Selected, selected)
    .state(ChoiceCardState::Invalid, self.invalid)
    .state(ChoiceCardState::Disabled, self.disabled)
    .state(ChoiceCardState::FullWidth, self.full_width)
    .state(ChoiceCardState::Loading, self.loading);
    let recipe = match kind {
      ChoiceCardKind::Checkbox => theme.resolve::<CheckboxCard>(&case, Some(&self.rules)),
      ChoiceCardKind::Radio => theme.resolve::<RadioCard>(&case, Some(&self.rules)),
    }
    .owned_by(self.id.as_ref());
    ChoiceCardView {
      kind,
      title: self.title,
      description: self.description,
      icon: self.icon,
      mark: self.mark,
      content: self.content,
      reason: self.reason,
      loading: self.loading,
      price: self.price,
      checked,
      selected,
      invalid: self.invalid,
      disabled: self.disabled,
      recipe: recipe.clone(),
      indicator: theme
        .resolve::<C>(&indicator.breakpoint(super::current_breakpoint()), None)
        .owned_like(&recipe),
      forced: self.forced,
      press: press.filter(|_| !self.disabled && !self.loading),
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

      /// An element of the app's in place of the icon, in the part `Icon`: a mark in its own colors, the maker of
      /// what is chosen.
      pub fn mark(mut self, mark: impl Into<Element>) -> Self {
        self.content.mark = Some(mark.into());
        self
      }

      /// An element under the description, in the part `Content`: a picker, the steps of what is chosen. A second
      /// one stands under the first. A press of what is in it presses the card too, unless it stops its press.
      pub fn content(mut self, content: impl Into<Element>) -> Self {
        self.content.content.push(content.into());
        self
      }

      /// Disables the card for `reason`, which it says under its description (the part `Reason`) and to what reads
      /// the tree, with a lock in place of its box or its mark.
      pub fn disabled_reason(mut self, reason: &str) -> Self {
        self.content.disabled = true;
        self.content.reason = Some(Arc::from(reason));
        self
      }

      /// Whether what the card chose is under way: it takes no press, is dimmed, keeps its box or its mark, and is
      /// busy to what reads the tree.
      pub fn loading(mut self, loading: bool) -> Self {
        self.content.loading = loading;
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

      /// What tells the card apart from those beside it, where two have one title.
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
/// // Bound to the app's signal, which a press turns.
/// # let reports = lurq::core::Signal::new(true);
/// let weekly = CheckboxCard::new("Weekly report").bind(reports);
/// ```
///
/// The card is one stop of Tab, and a click of it or Space presses it: the box in it is drawn and takes no focus
/// of its own. A checkbox card does not keep whether it is checked: the app says so with [`CheckboxCard::checked`],
/// and is told of a press with [`CheckboxCard::on_change`], or binds it to a signal with [`CheckboxCard::bind`].
/// Cards over one list are a [`CheckboxCardGroup`].
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

  /// Checked while `checked` is true, which a press turns. Handlers of [`CheckboxCard::on_change`] are told of a
  /// press after the signal is set, and not when the app sets it.
  pub fn bind(self, checked: Signal<bool>) -> Self {
    let set = checked.clone();
    let mut bound = self.checked(checked.get());
    bound.on_change.insert(0, Arc::new(move |next| set.set(next)));
    bound
  }

  /// Called on a press, with what the card is to be: checked if it was not. A second handler is called after the
  /// first.
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
/// let comfortable = RadioCard::new("Comfortable").description("Roomy rows, 36px controls.").selected(true);
/// let team = RadioCard::new("Team").price("$12 / month").on_change(|selected| println!("team: {selected}"));
/// ```
///
/// The card is pressed by a click, and by Space or Enter while it has the focus; the mark in it is drawn and takes
/// no focus of its own. A radio card does not keep whether it is selected: the app says so with
/// [`RadioCard::selected`], and is told with [`RadioCard::on_change`], given `true`, that it was pressed while it
/// was not. Cards that are one choice are a [`RadioCardGroup`], which the arrow keys move through.
pub struct RadioCard {
  content: Content,
  selected: bool,
  on_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
}

impl RadioCard {
  /// A radio card called `title`.
  pub fn new(title: &str) -> Self {
    Self {
      content: Content::new(title),
      selected: false,
      on_change: Vec::new(),
    }
  }

  /// Whether the card is the one of several selected: the dot in its mark, and its edge in the primary color.
  pub fn selected(mut self, selected: bool) -> Self {
    self.selected = selected;
    self
  }

  /// Called on a press of the card while it is not selected, with what it is to be: selected, `true`. A press of
  /// the one selected calls nothing. A second handler is called after the first.
  pub fn on_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Builds the card in `theme`. Converting a card into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let handlers = self.on_change;
    let press: Option<Press> = (!handlers.is_empty() && !self.selected).then(|| {
      Arc::new(move || {
        for handler in &handlers {
          handler(true);
        }
      }) as Press
    });
    let content = &self.content;
    let indicator = Case::new()
      .state(RadioState::Selected, self.selected)
      .state(RadioState::Invalid, content.invalid)
      .state(RadioState::Disabled, content.disabled);
    let view = self
      .content
      .view::<super::Radio>(theme, ChoiceCardKind::Radio, self.selected, press, indicator);
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
  /// The app's element in place of the icon.
  pub mark: Option<Element>,
  /// The app's elements under the description.
  pub content: Vec<Element>,
  /// Why it cannot be chosen, where it was disabled for a reason.
  pub reason: Option<Arc<str>>,
  /// Whether what it chose is under way.
  pub loading: bool,
  pub price: Option<Arc<str>>,
  /// Whether a checkbox card is checked: false for a radio card.
  pub checked: bool,
  /// Whether a radio card is the one selected: false for a checkbox card.
  pub selected: bool,
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
    let mut root = pressable(&self.part(ChoiceCardPart::Root), self.disabled || self.loading)
      .justify(Justify::Start)
      .align_items(Alignment::Start)
      .describe("a11y_role", role)
      .describe("a11y_name", self.title.clone())
      .describe(
        "a11y_checked",
        if self.checked || self.selected { "true" } else { "false" },
      );
    if let Some(description) = &self.description {
      root = root.describe("a11y_description", description.clone());
    }
    if let Some(reason) = &self.reason {
      root = root.describe("a11y_disabled_reason", reason.clone());
    }
    if self.loading {
      root = root.describe("a11y_busy", "true");
    }
    if self.kind == ChoiceCardKind::Radio {
      // In a group, which Tab stops at once, the selected card is where Tab enters.
      root = root.focus_entry(self.selected);
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

  /// The icon in the style of its part, or the app's mark in its place, where the card has one. It takes the mark
  /// out of the view.
  pub fn take_icon(&mut self) -> Option<Element> {
    match self.mark.take() {
      Some(mark) => Some(Row::new().part(&self.part(ChoiceCardPart::Icon)).child(mark).into()),
      None => self.icon(),
    }
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
    if let Some(reason) = &self.reason {
      text = text.child(wrapping(&self.part(ChoiceCardPart::Reason), reason));
    }
    text
  }

  /// The app's elements under the text, in the part `Content`, taken out of the view: `None` where it has none.
  pub fn take_content(&mut self) -> Option<Column> {
    if self.content.is_empty() {
      return None;
    }
    Some(
      Column::new()
        .width(Dimension::Pct(100.0))
        .min_width(Dimension::Px(0.0))
        .part(&self.part(ChoiceCardPart::Content))
        .with_children(std::mem::take(&mut self.content)),
    )
  }

  /// The box of a checkbox card with its check while it is checked, or the mark of a radio card with its dot while
  /// it is selected: the checkbox's and the radio's own parts, drawn, and not pressed or focused on their own.
  pub fn control(&self) -> Element {
    if self.reason.is_some() {
      return glyph(&self.part(ChoiceCardPart::Lock), Name::new(icons::LOCK), false);
    }
    let held = |part: ResolvedPart| {
      Row::new()
        .justify(Justify::Center)
        .align_items(Alignment::Center)
        .part(&part)
    };
    match self.kind {
      ChoiceCardKind::Checkbox => {
        let mut square = held(self.indicator.part(CheckboxPart::Box));
        if self.checked {
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
        if self.selected {
          mark = mark.child(Row::new().part(&self.indicator.part(RadioPart::Dot)));
        }
        mark.into()
      }
    }
  }

  /// How the default renderer builds a card: the icon, the text and the box or the mark at the end, side by side
  /// on the card.
  pub fn render(mut self) -> Element {
    let mut root = self.root();
    if let Some(icon) = self.take_icon() {
      root = root.child(icon);
    }
    let content = self.take_content();
    root
      .child(self.text().with_children(content))
      .child(self.control())
      .into()
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
/// top. Neither checked nor selected, the edge inside it is 1 of an input's edge; checked or selected, 2 of the
/// primary color. Under the pointer the edge of one that is neither is the ring's color; the ring of the focus is
/// around the card. An invalid one is edged in the destructive color, and a disabled one dimmed, as is one loading.
/// What the app puts under the description is 8 under it, its elements 8 apart; a reason is in the description's
/// type, and a lock of 16 in the muted foreground stands in place of the box or the mark.
fn card_recipe() -> Recipe {
  use ChoiceCardPart::{Content, Description, Heading, Icon, Lock, Price, Reason, Root, Text, Title};
  use ChoiceCardState::{Checked, Disabled, FullWidth, Invalid, Loading, Selected};
  use PaletteColor::{Background, Destructive, Foreground, Input, MutedForeground, Primary, Ring};
  let edge = |size: BorderSize, color: PaletteColor| Border::inside(size, color);
  let invalid = || Selector::new().state(Invalid);
  let recipe = Recipe::new()
    .base(Root, |s| {
      fading(
        s.width(220.0)
          .max_width(Length::Fill)
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
    .base(Content, |s| s.gap(8.0).padding_top(8.0))
    .base(Reason, |s| {
      s.typography(TypographyStyle::Description)
        .font_size(13.0)
        .line_height(1.4)
        .foreground(MutedForeground)
    })
    .base(Lock, |s| s.square(IconSize::Md).foreground(MutedForeground))
    .state(FullWidth, Root, |s| s.width(Length::Fill))
    .hovered(Root, |s| s.border(edge(BorderSize::Sm, Ring)))
    .focus_visible(Root, |s| s.focus_ring())
    // The pointer does not take from a card the edge that says it is checked or selected, or that it is wrong.
    .state(Checked, Root, |s| s.border(edge(BorderSize::Md, Primary)))
    .state(Selected, Root, |s| s.border(edge(BorderSize::Md, Primary)))
    .when(Selector::new().state(Checked).hovered(), Root, |s| {
      s.border(edge(BorderSize::Md, Primary))
    })
    .when(Selector::new().state(Selected).hovered(), Root, |s| {
      s.border(edge(BorderSize::Md, Primary))
    })
    .state(Invalid, Root, |s| s.border(edge(BorderSize::Sm, Destructive)))
    .when(invalid().hovered(), Root, |s| s.border(edge(BorderSize::Sm, Destructive)));
  [Checked, Selected]
    .into_iter()
    .fold(recipe, |recipe, on| {
      let marked = || Selector::new().state(on).state(Invalid);
      recipe
        .when(marked(), Root, |s| s.border(edge(BorderSize::Md, Destructive)))
        .when(marked().hovered(), Root, |s| {
          s.border(edge(BorderSize::Md, Destructive))
        })
    })
    .state(Disabled, Root, |s| {
      s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
    })
    .state(Loading, Root, |s| {
      s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
    })
}

impl crate::ui::Styled for CheckboxCard {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for CheckboxCard {
  const NAME: &'static str = "checkbox_card";
  type Renderer = ChoiceCardRenderer;

  /// The design's checkbox card: see [`RadioCard`]'s, which it shares. Its box is the checkbox's.
  fn recipe() -> Recipe {
    card_recipe()
  }
}

impl crate::ui::Styled for RadioCard {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for RadioCard {
  const NAME: &'static str = "radio_card";
  type Renderer = ChoiceCardRenderer;

  /// The design's radio card (`pencil/lurq-design-system.pen`, Radio Card), as its checkbox card is: 220 wide,
  /// padded 16, in the background's color with corners of `radius-lg`, the text first and the mark 12 after it at
  /// the top. Not selected, the edge inside it is 1 of an input's edge; selected, 2 of the primary color. Its mark
  /// is the radio's.
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

/// The states of a group of checkbox cards or radio cards a recipe styles. One of `Horizontal` and `Vertical` is
/// on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChoiceCardGroupState {
  /// Its cards stand side by side.
  Horizontal,
  /// Its cards stand one under the other.
  Vertical,
}

names!(ChoiceCardGroupState {
  Horizontal => "horizontal",
  Vertical => "vertical",
});

/// What builds a group of checkbox cards' or radio cards' nodes: a theme's in place of the default one.
pub type ChoiceCardGroupRenderer = Arc<dyn Fn(ChoiceCardGroupView) -> Element + Send + Sync>;

/// What a group of cards of either kind is given.
struct Group {
  label: Option<Arc<str>>,
  orientation: Orientation,
  invalid: bool,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Group {
  fn new() -> Self {
    Self {
      label: None,
      orientation: Orientation::Vertical,
      invalid: false,
      disabled: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// The group's recipe from the theme's for `C`, and what it says of each card in it.
  fn resolve<C: Themed>(&self, theme: &Theme) -> (ResolvedRecipe, Recipe) {
    let vertical = self.orientation.is_vertical();
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(ChoiceCardGroupState::Horizontal, !vertical)
    .state(ChoiceCardGroupState::Vertical, vertical);
    let recipe = theme.resolve::<C>(&case, Some(&self.rules)).owned_by(self.id.as_ref());
    // Each card, given no id of its own, is named after the group's: `x-item`, `x-item-2`.
    let item = recipe.rules_for(ChoiceCardGroupPart::Item, ChoiceCardPart::Root);
    (recipe, item)
  }

  /// The view of a group of `kind` of `cards`, built.
  fn view(self, kind: ChoiceCardKind, recipe: ResolvedRecipe, cards: Vec<Element>) -> ChoiceCardGroupView {
    ChoiceCardGroupView {
      kind,
      label: self.label,
      orientation: self.orientation,
      invalid: self.invalid,
      disabled: self.disabled,
      recipe,
      cards,
      id: self.id,
      key: self.key,
    }
  }
}

/// The props the two groups both have.
macro_rules! group_props {
  ($group:ident) => {
    impl<T: Clone + PartialEq + Send + Sync + 'static> $group<T> {
      /// Which way the cards stand: one under the other, unless told. Side by side they go on to the next line
      /// where there is no room.
      pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.group.orientation = orientation;
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

      /// Puts the group in a state the theme's recipe names, or takes it out.
      pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
        let state = state.into();
        self.group.states.retain(|held| *held != state);
        if on {
          self.group.states.push(state);
        }
        self
      }

      /// Styles one part of this group over what the theme gives it.
      pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
        self.group.rules = self.group.rules.base(part, style);
        self
      }

      /// Lays rules over the theme's recipe for this group: what the component it is in says of it in this place.
      pub fn rules(mut self, rules: &Recipe) -> Self {
        self.group.rules = self.group.rules.extend(rules);
        self
      }

      /// Draws the card Tab would stop on as it is under the pointer, held down or focused from the keyboard,
      /// with nothing pointing at it: for a gallery or a test. The other cards are drawn as they are.
      pub fn force(mut self, interaction: Option<Interaction>) -> Self {
        self.group.forced = interaction;
        self
      }

      /// The id of the group.
      pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
        self.group.id = Some(id.into());
        self
      }

      /// What tells the group apart from those beside it.
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

/// Radio cards that are one choice: the one whose value the app holds is selected, and a press of another selects
/// it.
///
/// ```
/// use lurq::{core::Signal, ui::{RadioCard, RadioCardGroup}};
///
/// # fn groups(density: Signal<u8>, region: Signal<Option<&'static str>>) {
/// let density = RadioCardGroup::new(density, [
///   (0, RadioCard::new("Comfortable").description("Roomy rows, 36px controls.")),
///   (1, RadioCard::new("Compact").description("Dense rows, 28px controls.")),
///   (2, RadioCard::new("Spacious").description("For touch.").disabled(true)),
/// ]);
/// // Nothing is selected until a region is: a signal of an `Option`.
/// let regions = RadioCardGroup::new(region, [("eu", RadioCard::new("Europe")), ("us", RadioCard::new("America"))])
///   .on_change(|region| println!("{region}"));
/// # }
/// ```
///
/// Each card is given its value; what the group holds says which is selected, and the group tells each card what
/// it says of all of them (invalid, disabled). The group is one stop of Tab, at the selected card or the first; the
/// arrow keys move the focus to the next card and select it, Home and End the first and the last, and a disabled
/// one is passed by, as in a [`RadioGroup`](super::RadioGroup).
pub struct RadioCardGroup<T = ()> {
  cards: Vec<(T, RadioCard)>,
  selected: Option<T>,
  set: Option<Arc<dyn Fn(T) + Send + Sync>>,
  on_change: Vec<Arc<dyn Fn(&T) + Send + Sync>>,
  group: Group,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> RadioCardGroup<T> {
  fn of(
    selected: Option<T>,
    set: Option<Arc<dyn Fn(T) + Send + Sync>>,
    cards: impl IntoIterator<Item = (T, RadioCard)>,
  ) -> Self {
    Self {
      cards: cards.into_iter().collect(),
      selected,
      set,
      on_change: Vec::new(),
      group: Group::new(),
    }
  }

  /// A group of which the card whose value the signal has is selected: a `Signal<T>`, or a `Signal<Option<T>>` of
  /// which `None` selects none. A press of another sets the signal.
  pub fn new(selected: impl Into<RadioSelection<T>>, cards: impl IntoIterator<Item = (T, RadioCard)>) -> Self {
    let selected = selected.into();
    Self::of(selected.selected, Some(selected.set), cards)
  }

  /// A group whose choice the app holds: the card of `selected` is selected, none if it is `None`, and `on_change`
  /// is told the value of each other one pressed. What is drawn is what the app says next.
  pub fn controlled(
    selected: Option<T>,
    cards: impl IntoIterator<Item = (T, RadioCard)>,
    on_change: impl Fn(&T) + Send + Sync + 'static,
  ) -> Self {
    Self::of(selected, None, cards).on_change(on_change)
  }

  /// Called on a press of a card that is not the one selected, with its value, after the signal the group is
  /// bound to is set. A press of the one selected calls nothing; that the app sets the signal calls nothing. A
  /// second handler is called after the first.
  pub fn on_change(mut self, handler: impl Fn(&T) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Builds the group in `theme`. Converting a group into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let (recipe, item) = self.group.resolve::<Self>(theme);
    let (invalid, disabled, forced) = (self.group.invalid, self.group.disabled, self.group.forced);
    let takes = |card: &RadioCard| !disabled && !card.content.disabled;
    // What Tab stops on: the selected card, else the first that takes a press.
    let entry = self
      .cards
      .iter()
      .position(|(value, card)| self.selected.as_ref() == Some(value) && takes(card))
      .or_else(|| self.cards.iter().position(|(_, card)| takes(card)));
    let (selected, set, handlers) = (self.selected, self.set, self.on_change);
    let cards = self
      .cards
      .into_iter()
      .enumerate()
      .map(|(index, (value, card))| {
        let (own_invalid, own_disabled) = (card.content.invalid, card.content.disabled);
        let on = selected.as_ref() == Some(&value);
        let mut card = card
          .selected(on)
          .invalid(invalid || own_invalid)
          .disabled(disabled || own_disabled)
          .rules(&item);
        if Some(index) == entry {
          card = card.force(forced);
        }
        if !on {
          let press = selecting(value, set.clone(), handlers.clone());
          card = card.on_change(move |_| press());
        }
        card.build(theme)
      })
      .collect();
    let view = self.group.view(ChoiceCardKind::Radio, recipe, cards);
    match theme.renderer::<Self>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

group_props!(RadioCardGroup);

impl<T: Clone + PartialEq + Send + Sync + 'static> From<RadioCardGroup<T>> for Element {
  fn from(group: RadioCardGroup<T>) -> Self {
    group.build(&current_theme())
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> crate::ui::Styled for RadioCardGroup<T> {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl<T: Send + Sync + 'static> Themed for RadioCardGroup<T> {
  const NAME: &'static str = "radio_card_group";
  type Renderer = ChoiceCardGroupRenderer;

  /// The design's radio cards: 12 apart, one under the other or side by side.
  fn recipe() -> Recipe {
    group_recipe()
  }
}

/// Checkbox cards over one list: those whose values the app holds are checked, and a press of one adds its value
/// or takes it out.
///
/// ```
/// use lurq::{core::Signal, ui::{CheckboxCard, CheckboxCardGroup, Orientation}};
///
/// # fn groups(add_ons: Signal<Vec<&'static str>>) {
/// let add_ons = CheckboxCardGroup::new(add_ons, [
///   ("backups", CheckboxCard::new("Backups").price("$4 / month")),
///   ("support", CheckboxCard::new("Priority support").price("$20 / month")),
/// ])
/// .orientation(Orientation::Horizontal)
/// .on_change(|add_ons| println!("{add_ons:?}"));
/// # }
/// ```
///
/// Each card is its own stop of Tab, and Space or Enter presses the one the focus is on.
pub struct CheckboxCardGroup<T = ()> {
  cards: Vec<(T, CheckboxCard)>,
  checked: Vec<T>,
  bound: Option<Turn<T>>,
  on_change: Vec<Arc<dyn Fn(&[T]) + Send + Sync>>,
  group: Group,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> CheckboxCardGroup<T> {
  fn of(checked: Vec<T>, bound: Option<Turn<T>>, cards: impl IntoIterator<Item = (T, CheckboxCard)>) -> Self {
    Self {
      cards: cards.into_iter().collect(),
      checked,
      bound,
      on_change: Vec::new(),
      group: Group::new(),
    }
  }

  /// A group of which the cards whose values the signal has are checked. A press adds a value to the signal, or
  /// takes it out.
  pub fn new(checked: Signal<Vec<T>>, cards: impl IntoIterator<Item = (T, CheckboxCard)>) -> Self
  where
    Vec<T>: SignalValue,
  {
    Self::of(checked.get(), Some(turning(checked)), cards)
  }

  /// A group whose state the app holds: the cards of `checked` are checked, and `on_change` is told the whole new
  /// set a press asks for: `checked` with the value pressed added, or taken out. What is drawn is what the app says
  /// next.
  pub fn controlled(
    checked: impl IntoIterator<Item = T>,
    cards: impl IntoIterator<Item = (T, CheckboxCard)>,
    on_change: impl Fn(&[T]) + Send + Sync + 'static,
  ) -> Self {
    Self::of(checked.into_iter().collect(), None, cards).on_change(on_change)
  }

  /// Called on a press, with the whole new set of values checked, after the signal the group is bound to is set.
  /// That the app sets the signal calls nothing. A second handler is called after the first.
  pub fn on_change(mut self, handler: impl Fn(&[T]) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Builds the group in `theme`. Converting a group into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let (recipe, item) = self.group.resolve::<Self>(theme);
    let (invalid, disabled, forced) = (self.group.invalid, self.group.disabled, self.group.forced);
    let entry = self
      .cards
      .iter()
      .position(|(_, card)| !disabled && !card.content.disabled);
    let (checked, bound, handlers) = (self.checked, self.bound, self.on_change);
    let cards = self
      .cards
      .into_iter()
      .enumerate()
      .map(|(index, (value, card))| {
        let (own_invalid, own_disabled) = (card.content.invalid, card.content.disabled);
        let on = checked.contains(&value);
        let press = toggling(value, &checked, bound.clone(), handlers.clone());
        let mut card = card
          .checked(on)
          .invalid(invalid || own_invalid)
          .disabled(disabled || own_disabled)
          .rules(&item)
          .on_change(move |_| press());
        if Some(index) == entry {
          card = card.force(forced);
        }
        card.build(theme)
      })
      .collect();
    let view = self.group.view(ChoiceCardKind::Checkbox, recipe, cards);
    match theme.renderer::<Self>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

group_props!(CheckboxCardGroup);

impl<T: Clone + PartialEq + Send + Sync + 'static> From<CheckboxCardGroup<T>> for Element {
  fn from(group: CheckboxCardGroup<T>) -> Self {
    group.build(&current_theme())
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> crate::ui::Styled for CheckboxCardGroup<T> {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl<T: Send + Sync + 'static> Themed for CheckboxCardGroup<T> {
  const NAME: &'static str = "checkbox_card_group";
  type Renderer = ChoiceCardGroupRenderer;

  /// The design's checkbox cards: 12 apart, one under the other or side by side.
  fn recipe() -> Recipe {
    group_recipe()
  }
}

/// A group of checkbox cards or radio cards as a renderer sees it: what it was given, the style of its parts, and
/// its cards, built with what the group says of each and what a press of each does.
pub struct ChoiceCardGroupView {
  /// Which of the two its cards are.
  pub kind: ChoiceCardKind,
  pub label: Option<Arc<str>>,
  pub orientation: Orientation,
  pub invalid: bool,
  pub disabled: bool,
  /// The style of each part of this group.
  pub recipe: ResolvedRecipe,
  /// Its cards, in order.
  pub cards: Vec<Element>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ChoiceCardGroupView {
  /// The style of a part, as this group is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// How the default renderer builds a group: its cards one under the other, or side by side and going on to the
  /// next line, as high as the highest in their line. Radio cards are a radio group to what reads the tree, one
  /// stop of Tab whose arrow keys move the focus and the choice with it; checkbox cards are a group.
  pub fn render(self) -> Element {
    let (role, keys) = match self.kind {
      // One choice, as a group of radios is: Tab stops in it once, and an arrow key moves the focus and the choice.
      ChoiceCardKind::Radio => ("radiogroup", Some(FocusGroup::both().activate(true).one_stop(true))),
      ChoiceCardKind::Checkbox => ("group", None),
    };
    let root = self.part(ChoiceCardGroupPart::Root);
    let (id, key, label, cards) = (self.id, self.key, self.label, self.cards);
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
    if self.orientation.is_vertical() {
      group!(Column::new().align_items(Alignment::Stretch))
    } else {
      // Cards side by side are as high as the highest in their line.
      group!(Row::new().wrap().align_items(Alignment::Stretch))
    }
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

impl<T: Clone + PartialEq + Send + Sync + 'static> super::FieldControl for CheckboxCardGroup<T> {
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

impl<T: Clone + PartialEq + Send + Sync + 'static> super::FieldControl for RadioCardGroup<T> {
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
