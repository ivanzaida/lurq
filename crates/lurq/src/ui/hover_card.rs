//! A hover card: more about what the pointer rests on, a person, a link, a
//! file, in a panel beside it that can be read and pressed.
//!
//! It is a mounted component, as the tooltip is: it keeps the timers of its
//! delays, an anchor that is the same from one render to the next, and where
//! the pointer is, none of which a builder can keep. [`HoverCard`] says what
//! it shows, and [`HoverCard::mount`] puts it on the element it is about.

use std::{cell::RefCell, sync::Arc, time::Duration};

use parking_lot::Mutex;

use super::{
  TooltipSide, names,
  part_style::{Length, PartAnimation, PartStyle, TransitionDuration},
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{FADE_IN, FADE_OUT, glyph},
  theme::{Themed, current_theme},
  tip::{Placed, arrow, arrow_size, frame, holds_children, put, side_of},
};
use crate::{
  app::{
    component::Component,
    ctx::{CollisionStrategy, Ctx, Overlay, Placement, Timeout},
    theme::{
      BorderSize, IconSize, MotionDuration, MotionEasing, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle,
    },
  },
  components::{Column, Row},
  core::{ElementRef, Signal},
  layout::{Alignment, text_style::FontWeight},
  node::{Element, HitTestBehavior, Node, NodeUpdate, border::Border, dimension::Dimension},
};

/// The parts of a hover card a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HoverCardPart {
  /// The panel and its arrow, and the room between them and what the card is about.
  Root,
  /// The panel: its fill, its edge, its shadow, the media beside the text.
  Panel,
  /// What stands at the start of the panel: an avatar, an icon of a file.
  Media,
  /// The title, the description, the facts and what the card holds, one under another.
  Text,
  /// What the card is about: a name, a title of a page.
  Title,
  /// A line or two about it.
  Description,
  /// The row of facts under the description.
  Facts,
  /// One fact: its icon and its text.
  Fact,
  /// The icon of a fact.
  FactIcon,
  /// What the arrow is drawn in, between the panel and what the card is about.
  Arrow,
}

names!(HoverCardPart {
  Root => "root",
  Panel => "panel",
  Media => "media",
  Text => "text",
  Title => "title",
  Description => "description",
  Facts => "facts",
  Fact => "fact",
  FactIcon => "fact-icon",
  Arrow => "arrow",
});

/// The states of a hover card a recipe styles. The side of what it is about
/// the card is on is a state too, by the names of [`TooltipSide`]: `"top"`,
/// `"bottom"`, `"left"`, `"right"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HoverCardState {
  /// It has an arrow.
  Arrow,
  /// It holds a control: it is a dialog to tooling, not a tooltip.
  Controls,
}

names!(HoverCardState {
  Arrow => "arrow",
  Controls => "controls",
});

/// What builds a hover card's nodes: a theme's in place of the default one.
/// It builds what is shown, not when.
pub type HoverCardRenderer = Arc<dyn Fn(HoverCardView) -> Element + Send + Sync>;

/// A hover card: more about what the pointer rests on, in a panel beside it.
/// A person's name in a comment shows who they are; a link shows the page it
/// goes to; a file shows its size and when it changed. Unlike a tooltip it
/// holds what a person reads and presses: a link, a button.
///
/// ```no_run
/// use lurq::{
///   app::ctx::Ctx,
///   node::Element,
///   ui::{Avatar, HoverCard, Link, icons},
/// };
///
/// fn author(ctx: &mut Ctx) -> Element {
///   HoverCard::new()
///     .media(Avatar::new("LQ"))
///     .title("@lurq")
///     .description("A Rust UI toolkit with GPU-backed rendering.")
///     .fact(icons::CALENDAR_DAYS, "Joined March 2024")
///     .mount(ctx, Link::new("@lurq"))
/// }
/// ```
///
/// It opens when the pointer has rested on the element for its open delay,
/// and at once when the keyboard's focus comes to the element or to anything
/// in it. It stays open while the pointer is on the element, on the card or
/// crossing the room between them, and while the focus is in either; it
/// closes when the pointer has left both for its close delay, when the focus
/// leaves, and on Escape, which it takes before anything under it. Tab goes
/// from the element into the card's controls and on to what follows the
/// element; the focus is not moved into it as it opens. One hover card of the
/// pointer shows at a time. Where there is no room on its side it goes to the
/// other, its arrow turned with it, and it is kept in the window.
///
/// To tooling the card is a dialog where it holds a control and a tooltip
/// where it does not, called by its title; the element is described by its
/// title and description.
///
/// Converted into an element on its own, a hover card is its panel as it
/// shows, anchored to nothing: for a gallery, a test, or a host of an app's own.
#[derive(Clone)]
pub struct HoverCard {
  title: Option<Arc<str>>,
  description: Option<Arc<str>>,
  media: Option<Element>,
  facts: Vec<(Option<Name>, Arc<str>)>,
  children: Vec<Element>,
  placement: Placement,
  arrow: bool,
  open_delay: TransitionDuration,
  close_delay: TransitionDuration,
  open: Option<bool>,
  disabled: bool,
  width: Option<Length>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Default for HoverCard {
  fn default() -> Self {
    Self::new()
  }
}

impl HoverCard {
  /// A hover card with nothing in it yet, under what it is about, with no arrow.
  pub fn new() -> Self {
    Self {
      title: None,
      description: None,
      media: None,
      facts: Vec::new(),
      children: Vec::new(),
      placement: Placement::Bottom,
      arrow: false,
      open_delay: TransitionDuration::Role(MotionDuration::Slower),
      close_delay: TransitionDuration::Role(MotionDuration::Slow),
      open: None,
      disabled: false,
      width: None,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// What the card is about, at the top of its text: a name, a page's title.
  pub fn title(mut self, title: &str) -> Self {
    self.title = Some(Arc::from(title));
    self
  }

  /// A line or two about it, under the title.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// What stands at the start of the panel, beside the text: an avatar, the
  /// icon of a file.
  pub fn media(mut self, media: impl Into<Element>) -> Self {
    self.media = Some(media.into());
    self
  }

  /// Adds a fact under the description, after those it has: an icon of the
  /// theme's set by its name and a short text, "Joined March 2024".
  pub fn fact(mut self, icon: impl Into<Name>, text: &str) -> Self {
    self.facts.push((Some(icon.into()), Arc::from(text)));
    self
  }

  /// Adds a fact with no icon.
  pub fn plain_fact(mut self, text: &str) -> Self {
    self.facts.push((None, Arc::from(text)));
    self
  }

  /// Adds something the card holds, under its text: a link, a row of buttons.
  pub fn child(mut self, child: impl Into<Element>) -> Self {
    self.children.push(child.into());
    self
  }

  /// Adds several things the card holds.
  pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.children.extend(children.into_iter().map(Into::into));
    self
  }

  /// Which side of what it is about the card is on, and which edge of it it
  /// lines up with: under it and centered by default.
  pub fn placement(mut self, placement: Placement) -> Self {
    self.placement = placement;
    self
  }

  /// Whether a triangle points from the panel to what it is about.
  pub fn arrow(mut self, arrow: bool) -> Self {
    self.arrow = arrow;
    self
  }

  /// How long the pointer rests on the element before the card opens: a
  /// duration of the theme's (the slower one by default), or a set one. The
  /// keyboard's focus opens it at once.
  pub fn open_delay(mut self, delay: impl Into<TransitionDuration>) -> Self {
    self.open_delay = delay.into();
    self
  }

  /// How long the card stays after the pointer left both it and the
  /// element: time to cross the room between them. The theme's slow
  /// duration by default.
  pub fn close_delay(mut self, delay: impl Into<TransitionDuration>) -> Self {
    self.close_delay = delay.into();
    self
  }

  /// Shows the card whatever the pointer and the focus do (`Some(true)`), or
  /// never (`Some(false)`); `None`, as it starts, leaves it to them. For a
  /// gallery, a test.
  pub fn open(mut self, open: Option<bool>) -> Self {
    self.open = open;
    self
  }

  /// Whether the card never opens: the element is as it would be without it.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// How wide the panel is, in place of what the theme gives it.
  pub fn width(mut self, width: impl Into<Length>) -> Self {
    self.width = Some(width.into());
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

  /// The id of the card's root: what shows.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Puts the card on `element`, what it is about, and returns that element.
  /// The pointer and the focus are the element's, or those of anything in it.
  pub fn mount(self, ctx: &mut Ctx, element: impl Into<Element>) -> Element {
    ctx.mount::<HoverCardHost>(HoverCardProps {
      card: self,
      child: element.into(),
    })
  }

  /// Whether what the card holds takes the focus: a link, a button.
  fn has_controls(&self) -> bool {
    fn takes(node: &Node) -> bool {
      node.is_focusable() || node.children().iter().any(takes)
    }
    self.children.iter().chain(&self.media).any(|child| takes(&child.node))
  }

  /// The card as it is drawn on `side`, in `theme`.
  fn view(&self, theme: &Theme, side: TooltipSide) -> HoverCardView {
    let controls = self.has_controls();
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(side, true)
    .state(HoverCardState::Arrow, self.arrow)
    .state(HoverCardState::Controls, controls);
    // A width given here is under what `style` and `rules` say of the panel.
    let mut rules = Recipe::new();
    if let Some(width) = self.width {
      rules = rules.base(HoverCardPart::Panel, |s| s.width(width));
    }
    let rules = rules.extend(&self.rules);
    HoverCardView {
      title: self.title.clone(),
      description: self.description.clone(),
      media: self.media.clone(),
      facts: self.facts.clone(),
      children: self.children.clone(),
      side,
      arrow: self.arrow,
      controls,
      aim: 0.0,
      recipe: theme.resolve::<HoverCard>(&case, Some(&rules)),
      id: self.id.clone(),
      key: self.key.clone(),
    }
  }

  /// Builds what `view` shows: the theme's renderer, or the default one.
  fn render_view(view: HoverCardView) -> Element {
    match view.theme().renderer::<HoverCard>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// Builds the card's panel as it shows, anchored to nothing, in `theme`.
  /// Converting a hover card into an element does this in the theme of what
  /// is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    Self::render_view(self.view(theme, side_of(self.placement)))
  }
}

impl From<HoverCard> for Element {
  fn from(card: HoverCard) -> Self {
    card.build(&current_theme())
  }
}

/// A hover card as a renderer sees it: what it shows, the side it is on, and
/// the style of its parts.
pub struct HoverCardView {
  pub title: Option<Arc<str>>,
  pub description: Option<Arc<str>>,
  pub media: Option<Element>,
  /// The facts under the description: the name of each one's icon, and its text.
  pub facts: Vec<(Option<Name>, Arc<str>)>,
  /// What the card holds, under its text.
  pub children: Vec<Element>,
  /// The side it is on: the side its placement gives, or the other where there was no room.
  pub side: TooltipSide,
  pub arrow: bool,
  /// Whether it holds a control: it is a dialog to tooling, not a tooltip.
  pub controls: bool,
  /// How far along the panel's edge from its middle the arrow is, toward the
  /// middle of what the card is about.
  pub aim: f32,
  /// The style of each part of this card.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl HoverCardView {
  /// The style of a part, as this card is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// A text in the style of `part`, as wide as the room it has: a longer one
  /// goes on to the next line.
  fn wrapping(&self, part: HoverCardPart, content: &str) -> Column {
    let part = self.part(part);
    let fill = Dimension::Pct(100.0);
    Column::new()
      .width(fill)
      .min_width(Dimension::Px(0.0))
      .part(&part)
      .child(part.text(content).width(fill))
  }

  /// The facts in a row, each its icon and its text. `None` for a card with none.
  pub fn facts(&self) -> Option<Row> {
    if self.facts.is_empty() {
      return None;
    }
    let (fact, icon) = (self.part(HoverCardPart::Fact), self.part(HoverCardPart::FactIcon));
    let mut row = Row::new()
      .wrap()
      .align_items(Alignment::Center)
      .part(&self.part(HoverCardPart::Facts));
    for (name, text) in &self.facts {
      let mut item = Row::new().align_items(Alignment::Center).part(&fact);
      if let Some(name) = name {
        item = item.child(glyph(&icon, *name, false));
      }
      row = row.child(item.child(fact.text(text).nowrap()));
    }
    Some(row)
  }

  /// The text: the title, the description, the facts and what the card
  /// holds, one under another, as wide as the room the media leaves it.
  pub fn text(&mut self) -> Column {
    let mut text = Column::new()
      .flex(1.0)
      .min_width(Dimension::Px(0.0))
      .align_items(Alignment::Stretch)
      .part(&self.part(HoverCardPart::Text));
    if let Some(title) = &self.title {
      text = text.child(self.wrapping(HoverCardPart::Title, title));
    }
    if let Some(description) = &self.description {
      text = text.child(self.wrapping(HoverCardPart::Description, description));
    }
    if let Some(facts) = self.facts() {
      text = text.child(facts);
    }
    for child in std::mem::take(&mut self.children) {
      text = text.child(child);
    }
    text
  }

  /// The panel: the media at its start, beside the text. A dialog to what
  /// reads the tree where it holds a control, else a tooltip, called by its
  /// title.
  pub fn panel(&mut self) -> Row {
    let mut panel = Row::new()
      .align_items(Alignment::Start)
      .part(&self.part(HoverCardPart::Panel))
      .describe("a11y_role", if self.controls { "dialog" } else { "tooltip" });
    if let Some(title) = &self.title {
      panel = panel.describe("a11y_name", title.clone());
    }
    if let Some(media) = self.media.take() {
      panel = panel.child(
        Row::new()
          .flex_shrink(0.0)
          .part(&self.part(HoverCardPart::Media))
          .child(media),
      );
    }
    panel.child(self.text())
  }

  /// How long the arrow is along the panel's edge, and how far out from it
  /// it goes. `None` without an arrow.
  pub fn arrow_size(&self) -> Option<(f32, f32)> {
    if !self.arrow {
      return None;
    }
    arrow_size(&self.part(HoverCardPart::Arrow), self.side)
  }

  /// The arrow, in the panel's fill and with its edge, on the side toward
  /// what the card is about. `None` without one.
  pub fn arrow(&self) -> Option<Element> {
    if !self.arrow {
      return None;
    }
    arrow(
      &self.part(HoverCardPart::Arrow),
      &self.part(HoverCardPart::Panel),
      self.side,
      self.aim,
    )
  }

  /// How the default renderer builds a hover card: the panel, and the arrow
  /// on its side toward what the card is about.
  pub fn render(mut self) -> Element {
    let away = self.arrow_size().map_or(0.0, |(_, away)| away);
    let arrow = self.arrow();
    let panel = self.panel();
    let mut root: Element = frame(&self.part(HoverCardPart::Root), panel.into(), arrow, away, self.side).into();
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }
}

impl Themed for HoverCard {
  const NAME: &'static str = "hover-card";
  type Renderer = HoverCardRenderer;

  /// The design's hover card (`pencil/lurq-design-system.pen`, Hover Card):
  /// a panel 300 wide in the popover's colors, with an edge of 1 in the
  /// border's color, the large radius and the medium shadow, 16 of padding,
  /// the media 12 before the text. The text is 6 apart: the title in 14 at
  /// medium weight, the description in 13 of the muted foreground in lines of
  /// 1.5, and the facts, each an icon of 14 6 before its text in 12, both
  /// muted. It stands 8 from what it is about; its arrow is 18 along the
  /// panel and 9 out from it. It fades in over the base duration and out over
  /// the fast one, as the popover does.
  fn recipe() -> Recipe {
    use HoverCardPart::{Arrow, Description, Fact, FactIcon, Facts, Panel, Root, Text, Title};
    use PaletteColor::{Border as Edge, MutedForeground, Popover, PopoverForeground};
    use TooltipSide::{Bottom, Left, Right, Top};
    Recipe::new()
      .base(Root, |s| {
        s.enter(PartAnimation::new(FADE_IN, MotionDuration::Base, MotionEasing::Enter))
          .exit(PartAnimation::new(FADE_OUT, MotionDuration::Fast, MotionEasing::Exit))
      })
      .base(Panel, |s| {
        s.width(300.0)
          .padding_x(16.0)
          .padding_y(16.0)
          .gap(12.0)
          .radius(RadiusSize::Lg)
          .background(Popover)
          .foreground(PopoverForeground)
          .border(Border::inside(BorderSize::Sm, Edge))
          .shadow(ShadowStyle::Md)
      })
      .base(Text, |s| s.gap(6.0))
      .base(Title, |s| {
        s.typography(TypographyStyle::Body).weight(FontWeight::Medium)
      })
      .base(Description, |s| {
        s.typography(TypographyStyle::Description)
          .font_size(13.0)
          .line_height(1.5)
          .foreground(MutedForeground)
      })
      .base(Facts, |s| s.gap(12.0))
      .base(Fact, |s| {
        s.gap(6.0)
          .typography(TypographyStyle::Caption)
          .foreground(MutedForeground)
      })
      .base(FactIcon, |s| s.square(IconSize::Sm))
      .base(Arrow, |s| s.width(18.0).height(9.0))
      .state(Left, Arrow, |s| s.width(9.0).height(18.0))
      .state(Right, Arrow, |s| s.width(9.0).height(18.0))
      // The room between the card and what it is about, toward it.
      .state(Top, Root, |s| s.padding_bottom(8.0))
      .state(Bottom, Root, |s| s.padding_top(8.0))
      .state(Left, Root, |s| s.padding_end(8.0))
      .state(Right, Root, |s| s.padding_start(8.0))
  }
}

/// A hover card and the element it is about.
#[derive(Clone)]
struct HoverCardProps {
  card: HoverCard,
  child: Element,
}

/// The element is built anew by every render of what holds it, so props are
/// never the same as before.
impl PartialEq for HoverCardProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for HoverCardProps {}

/// What puts away a card the pointer opened.
type Hide = Arc<dyn Fn() + Send + Sync>;

thread_local! {
  /// The card the pointer opened last, by its host, and what puts it away.
  static SHOWING: RefCell<Option<(usize, Hide)>> = const { RefCell::new(None) };
}

/// The pointer opened the card of the host `id`: the one it opened before,
/// still there for its close delay, goes, so that one card shows at a time.
fn take_turn(id: usize, hide: &Hide) {
  let before = SHOWING.with(|showing| showing.borrow_mut().replace((id, hide.clone())));
  if let Some((before, hide)) = before
    && before != id
  {
    hide();
  }
}

/// A duration of `theme`'s, or a set one.
fn length(duration: TransitionDuration, theme: &Theme) -> Duration {
  match duration {
    TransitionDuration::Role(role) => theme.durations().try_get(role).unwrap_or_default(),
    TransitionDuration::Set(duration) => duration,
  }
}

/// Where the pointer is: on the element, on the card.
#[derive(Default)]
struct Pointer {
  on_element: bool,
  on_card: bool,
}

/// What shows a hover card on the element it is about, and when.
struct HoverCardHost {
  /// Whether the pointer has rested on the element for the open delay, and not left both it and the card since.
  hovered: Signal<bool>,
  /// Whether the card may show: Escape (the runtime closes the top layer that takes it) turns it off, and it
  /// stays off until the pointer comes again, or until nothing asks for the card (the pointer and the focus
  /// both gone), when a new one takes its place.
  allowed: Mutex<Signal<bool>>,
  placed: Placed,
  pointer: Arc<Mutex<Pointer>>,
  /// The open delay.
  opening: Timeout,
  /// The close delay.
  closing: Timeout,
  /// The element's anchor where the element has none of its own.
  anchor: ElementRef,
  /// What shows.
  shown: ElementRef,
  /// Who this host is among the cards the pointer opens, and what puts its card away.
  id: usize,
  hide: Hide,
}

impl Component for HoverCardHost {
  type Props = HoverCardProps;

  fn create(ctx: &mut Ctx) -> Self {
    let hovered = ctx.signal(false);
    let pointer = Arc::new(Mutex::new(Pointer::default()));
    let id = Arc::as_ptr(&pointer) as usize;
    let hide: Hide = {
      let hovered = hovered.clone();
      Arc::new(move || put(&hovered, false))
    };
    let opening = {
      let (hovered, hide) = (hovered.clone(), hide.clone());
      ctx.create_timeout(Duration::ZERO, move || {
        put(&hovered, true);
        take_turn(id, &hide);
      })
    };
    let closing = {
      let (hovered, pointer) = (hovered.clone(), pointer.clone());
      ctx.create_timeout(Duration::ZERO, move || {
        let gone = {
          let pointer = pointer.lock();
          !pointer.on_element && !pointer.on_card
        };
        if gone {
          put(&hovered, false);
        }
      })
    };
    let (anchor, shown) = (ElementRef::new(), ElementRef::new());
    let placed = Placed::new(ctx, &shown);
    Self {
      hovered,
      allowed: Mutex::new(ctx.signal(true)),
      placed,
      pointer,
      opening,
      closing,
      anchor,
      shown,
      id,
      hide,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let HoverCardProps { card, child } = ctx.props::<HoverCardProps>().clone();
    if card.disabled {
      return child;
    }
    let theme = current_theme();
    self.opening.set_duration(length(card.open_delay, &theme));
    self.closing.set_duration(length(card.close_delay, &theme));
    let child = holds_children(child);
    // An element that has an anchor of its own keeps it, and the card is anchored to it.
    let anchor = child.node.element_ref.clone().unwrap_or_else(|| self.anchor.clone());

    // Reading them subscribes: this renders again when the pointer, the focus or Escape changes them. The focus
    // is the element's or that of anything in it, or in the card, where Tab took it.
    let asked = self.hovered.get() || anchor.focus_visible_within() || self.shown.focus_visible_within();
    let allowed = {
      let mut allowed = self.allowed.lock();
      if !asked {
        // Nothing asks for it: whatever put it away is over, and the next time it is asked for it shows.
        *allowed = Signal::new(true);
      }
      allowed.clone()
    };
    let side = self.placed.side(card.placement);
    let mut view = card.view(&theme, side);
    view.aim = self.placed.aim();
    self.placed.note(
      card.placement,
      &anchor,
      view.arrow_size().map_or(0.0, |(along, _)| along),
    );
    let shows = card.open.unwrap_or(asked);
    // A card that went from under the pointer is told of no leave: the pointer is on it no longer.
    if !shows || (card.open.is_none() && !allowed.get()) {
      self.pointer.lock().on_card = false;
    }
    let overlay = if shows {
      let card_enter = {
        let (pointer, closing) = (self.pointer.clone(), self.closing.clone());
        move || {
          pointer.lock().on_card = true;
          closing.cancel();
        }
      };
      let card_leave = {
        let (pointer, closing, hovered) = (self.pointer.clone(), self.closing.clone(), self.hovered.clone());
        move || {
          pointer.lock().on_card = false;
          if hovered.get_untracked() {
            closing.restart();
          }
        }
      };
      let shown = HoverCard::render_view(view).map_node(|node| {
        node
          .ref_element(self.shown.clone())
          .on_mouse_enter(card_enter)
          .on_mouse_leave(card_leave)
      });
      let overlay = Overlay::new(shown)
        .anchor(anchor.clone())
        .placement(card.placement)
        .collision(CollisionStrategy::FlipThenClamp)
        // The pointer can be moved onto the card, its root and the room in it toward the element included (with
        // `ContentOnly` the root, which holds the handlers, would not be hit). It stands beside the element.
        .hit_test(HitTestBehavior::Auto)
        // Tab goes from the element into the card's controls and on, and the focus is not moved as it opens.
        .tab_after_anchor(true);
      match card.open {
        // Shown by the app: Escape is not the card's to take.
        Some(_) => overlay,
        // The runtime turns `allowed` off on an Escape while this is the top layer that takes it, so the key
        // closes the card and not the dialog under it.
        None => overlay.open(allowed.clone()).dismiss_on_escape(true),
      }
    } else {
      Overlay::new(Element::new()).open_when(false)
    };
    // An element is not shared between threads as it is: the decoration, which is, holds it in a lock.
    let overlay = Arc::new(Mutex::new(Element::from(overlay)));

    let enter = {
      let (hovered, allowed, opening, closing, pointer) = (
        self.hovered.clone(),
        allowed.clone(),
        self.opening.clone(),
        self.closing.clone(),
        self.pointer.clone(),
      );
      let (id, hide, instant) = (self.id, self.hide.clone(), length(card.open_delay, &theme).is_zero());
      move || {
        pointer.lock().on_element = true;
        closing.cancel();
        put(&allowed, true);
        if hovered.get_untracked() {
          return;
        }
        if instant {
          opening.cancel();
          hovered.set(true);
          take_turn(id, &hide);
        } else {
          opening.restart();
        }
      }
    };
    let leave = {
      let (hovered, allowed, opening, closing, pointer) = (
        self.hovered.clone(),
        allowed.clone(),
        self.opening.clone(),
        self.closing.clone(),
        self.pointer.clone(),
      );
      move || {
        pointer.lock().on_element = false;
        opening.cancel();
        if !hovered.get_untracked() {
          return;
        }
        if allowed.get_untracked() {
          // It stays for its close delay: time to move onto it.
          closing.restart();
        } else {
          // Put away already: nothing is there to move onto.
          hovered.set(false);
        }
      }
    };

    let described: Option<Arc<str>> = match (&card.title, &card.description) {
      (Some(title), Some(description)) => Some(Arc::from(format!("{title}. {description}"))),
      (Some(text), None) | (None, Some(text)) => Some(text.clone()),
      (None, None) => None,
    };
    let controls = card.has_controls();
    let describe = move |node: Node| {
      let mut node = node
        .ref_element(anchor.clone())
        .on_mouse_enter(enter.clone())
        .on_mouse_leave(leave.clone());
      if let Some(text) = &described {
        NodeUpdate::describe(&mut node, Arc::from("a11y_description"), text.clone());
      }
      if controls {
        NodeUpdate::describe(&mut node, Arc::from("a11y_haspopup"), Arc::from("dialog"));
      }
      // The overlay is declared in the element: a layer is found among the children of a container.
      NodeUpdate::child(&mut node, overlay.lock().clone().into_node());
      node
    };
    child.decorate_root(describe)
  }
}
