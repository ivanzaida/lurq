//! A teaching tip: a panel that points at one thing in the app and says what
//! it is for, and a tour, the app's teaching tips shown one after another.
//!
//! Both are mounted components: a teaching tip keeps which side the window
//! let it take and what it tells when Escape closes it, and a tour which of
//! its steps it showed last, none of which a builder can keep. What is open,
//! and which step, is the app's: a `Signal` it gives them.

use std::sync::Arc;

use parking_lot::Mutex;

use super::{
  Button, ButtonPart, ButtonSize, ButtonVariant, FontFamily, TooltipSide, icons, names,
  part_style::{Length, PartAnimation, PartStyle},
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{FADE_IN, FADE_OUT},
  theme::{Themed, current_theme},
  tip::{Placed, arrow, arrow_size, frame, side_of},
};
use crate::{
  app::{
    component::Component,
    ctx::{CollisionStrategy, Ctx, Placement},
    events::MouseEvent,
    theme::{BorderSize, MotionDuration, MotionEasing, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle},
  },
  components::{Column, Popup, Row},
  core::{ElementRef, Signal, Subscription},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, EventHandler, IntoMouseEventHandler, border::Border, color::Color, dimension::Dimension},
};

/// The parts of a teaching tip a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TeachingTipPart {
  /// The panel and its arrow, and the room between them and what it points at.
  Root,
  /// The panel: its fill, its edge, its shadow, the media over the content.
  Panel,
  /// A picture or an icon at the top of the panel, as wide as it.
  Media,
  /// The text over the footer, under the media.
  Content,
  /// The header over the description.
  Text,
  /// The title and the close control at its end.
  Header,
  /// What it points at, in a few words.
  Title,
  /// The control that closes it: what it gives the theme's button.
  Close,
  /// What it is for.
  Description,
  /// The step count at its start and the actions at its end.
  Footer,
  /// The step count, "2 of 4".
  Step,
  /// The buttons of the footer.
  Actions,
  /// The button that goes on: what it gives the theme's button.
  PrimaryAction,
  /// The button beside it, that goes back: what it gives the theme's button.
  SecondaryAction,
  /// What the arrow is drawn in, between the panel and what it points at.
  Arrow,
}

names!(TeachingTipPart {
  Root => "root",
  Panel => "panel",
  Media => "media",
  Content => "content",
  Text => "text",
  Header => "header",
  Title => "title",
  Close => "close",
  Description => "description",
  Footer => "footer",
  Step => "step",
  Actions => "actions",
  PrimaryAction => "primary-action",
  SecondaryAction => "secondary-action",
  Arrow => "arrow",
});

/// How a teaching tip looks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TeachingTipVariant {
  /// In the popover's colors, with an edge: the design's.
  #[default]
  Plain,
  /// In the primary color: one that is to be seen first.
  Primary,
}

names!(TeachingTipVariant {
  Plain => "plain",
  Primary => "primary",
});

/// The states of a teaching tip a recipe styles. The side of what it points
/// at it is on is a state too, by the names of [`TooltipSide`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TeachingTipState {
  /// It has an arrow.
  Arrow,
  /// It is a step of several, with a count: the design's tour.
  Stepped,
  /// It has media at its top.
  Media,
}

names!(TeachingTipState {
  Arrow => "arrow",
  Stepped => "stepped",
  Media => "media",
});

/// What builds a teaching tip's panel: a theme's in place of the default one.
/// Where it is shown, and when, stays the teaching tip's.
pub type TeachingTipRenderer = Arc<dyn Fn(TeachingTipView) -> Element + Send + Sync>;

/// What is told as a teaching tip closes.
type Closer = Arc<dyn Fn() + Send + Sync>;

/// A button of a teaching tip: its label, and what a press of it does.
#[derive(Clone)]
struct Action {
  label: Arc<str>,
  on_click: Vec<EventHandler<MouseEvent>>,
}

/// A teaching tip: a panel that points at one thing in the app and says what
/// it is for, the first time a person comes to it or when it is new. A title,
/// a line or two, a picture where there is one, a button to go on.
///
/// ```no_run
/// use lurq::{
///   app::ctx::Ctx,
///   core::{ElementRef, Signal},
///   node::Element,
///   ui::TeachingTip,
/// };
///
/// // The anchor is the element it points at, kept from one render to the next, as is whether it is open.
/// fn dock_tip(ctx: &mut Ctx, panels: ElementRef, open: Signal<bool>) -> Element {
///   let close = open.clone();
///   TeachingTip::new(panels, open)
///     .title("Dock panels anywhere")
///     .description("Drag a panel by its header and drop it on any edge of the window.")
///     .primary("Got it", move |_| close.set(false))
///     .mount(ctx)
/// }
/// ```
///
/// It does not keep whether it is open: the app's signal does. Its close
/// control and Escape close it, which sets the signal to `false` and tells
/// [`TeachingTip::on_close`]; a press elsewhere does not, so a person can go
/// on with what they are doing beside it. It does not take the focus from
/// what a person is doing unless the app says so
/// ([`TeachingTip::takes_focus`]): Tab goes from what it points at into its
/// buttons and on. To tooling it is a dialog called by its title. Where there
/// is no room on its side it goes to the other, its arrow turned with it, and
/// it is kept in the window.
///
/// Converted into an element on its own, a teaching tip is its panel as it
/// shows, anchored to nothing: for a gallery, a test, or a host of an app's own.
#[derive(Clone)]
pub struct TeachingTip {
  anchor: ElementRef,
  open: Signal<bool>,
  title: Option<Arc<str>>,
  description: Option<Arc<str>>,
  media: Option<Element>,
  children: Vec<Element>,
  step: Option<(usize, usize)>,
  primary: Option<Action>,
  secondary: Option<Action>,
  on_close: Vec<Closer>,
  close_button: bool,
  variant: Name,
  placement: Placement,
  arrow: bool,
  takes_focus: bool,
  width: Option<Length>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl TeachingTip {
  /// A teaching tip pointing at `anchor` while `open` is `true`: under it,
  /// with an arrow and a close control, in the plain look.
  pub fn new(anchor: impl Into<ElementRef>, open: Signal<bool>) -> Self {
    Self {
      anchor: anchor.into(),
      open,
      title: None,
      description: None,
      media: None,
      children: Vec::new(),
      step: None,
      primary: None,
      secondary: None,
      on_close: Vec::new(),
      close_button: true,
      variant: TeachingTipVariant::Plain.into(),
      placement: Placement::Bottom,
      arrow: true,
      takes_focus: false,
      width: None,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// What it points at, in a few words, at the top of its text.
  pub fn title(mut self, title: &str) -> Self {
    self.title = Some(Arc::from(title));
    self
  }

  /// What it is for, under the title.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// A picture or an icon at the top of the panel, on a muted fill as wide as it.
  pub fn media(mut self, media: impl Into<Element>) -> Self {
    self.media = Some(media.into());
    self
  }

  /// Adds something it holds, under the description.
  pub fn child(mut self, child: impl Into<Element>) -> Self {
    self.children.push(child.into());
    self
  }

  /// That it is the step `step` of `of`, counted from 1: "2 of 4" in the footer.
  pub fn step(mut self, step: usize, of: usize) -> Self {
    self.step = Some((step, of));
    self
  }

  /// The button that goes on, at the end of the footer: "Next", "Got it".
  pub fn primary(mut self, label: &str, on_click: impl IntoMouseEventHandler) -> Self {
    self.primary = Some(Action {
      label: Arc::from(label),
      on_click: vec![on_click.into_event_handler()],
    });
    self
  }

  /// The button before it: "Back", "Not now".
  pub fn secondary(mut self, label: &str, on_click: impl IntoMouseEventHandler) -> Self {
    self.secondary = Some(Action {
      label: Arc::from(label),
      on_click: vec![on_click.into_event_handler()],
    });
    self
  }

  /// What is told as its close control or Escape closes it, after which the
  /// signal is `false`.
  pub fn on_close(mut self, on_close: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_close.push(Arc::new(on_close));
    self
  }

  /// Whether it has a close control at the end of its title. It has by default.
  pub fn close_button(mut self, close_button: bool) -> Self {
    self.close_button = close_button;
    self
  }

  /// How it looks: [`TeachingTipVariant`], or a name the theme's recipe gives.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// Which side of what it points at it is on, and which edge of it it lines
  /// up with: under it and centered by default.
  pub fn placement(mut self, placement: Placement) -> Self {
    self.placement = placement;
    self
  }

  /// Whether a triangle points from the panel at what it is about. It does by default.
  pub fn arrow(mut self, arrow: bool) -> Self {
    self.arrow = arrow;
    self
  }

  /// Whether the focus goes into it as it opens, and back to what had it as
  /// it closes: for a tip a person is to act on before going on. Off by
  /// default, when Tab goes from what it points at into its buttons.
  pub fn takes_focus(mut self, takes: bool) -> Self {
    self.takes_focus = takes;
    self
  }

  /// How wide the panel is, in place of what the theme gives it.
  pub fn width(mut self, width: impl Into<Length>) -> Self {
    self.width = Some(width.into());
    self
  }

  /// Puts the teaching tip in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this teaching tip over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this teaching tip.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of its root, what shows. Its close control is `<id>-close`, and
  /// its buttons `<id>-primary` and `<id>-secondary`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Puts the teaching tip in the app: it shows over the window by its
  /// anchor while its signal is `true`. The element it returns has no size,
  /// and goes anywhere.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    ctx.mount::<TeachingTipHost>(TeachingTipProps { tip: self })
  }

  /// The teaching tip as it is drawn on `side`, in `theme`, with `close` for
  /// what its close control does.
  fn view(&self, theme: &Theme, side: TooltipSide, close: Closer) -> TeachingTipView {
    let case = Case {
      variant: Some(self.variant),
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(side, true)
    .state(TeachingTipState::Arrow, self.arrow)
    .state(TeachingTipState::Stepped, self.step.is_some())
    .state(TeachingTipState::Media, self.media.is_some());
    // A width given here is under what `style` and `rules` say of the panel.
    let mut rules = Recipe::new();
    if let Some(width) = self.width {
      rules = rules.base(TeachingTipPart::Panel, |s| s.width(width));
    }
    let rules = rules.extend(&self.rules);
    TeachingTipView {
      title: self.title.clone(),
      description: self.description.clone(),
      media: self.media.clone(),
      children: self.children.clone(),
      step: self.step,
      primary: self.primary.as_ref().map(|action| action.label.clone()),
      secondary: self.secondary.as_ref().map(|action| action.label.clone()),
      close_button: self.close_button,
      variant: self.variant,
      side,
      arrow: self.arrow,
      aim: 0.0,
      recipe: theme.resolve::<TeachingTip>(&case, Some(&rules)),
      actions: (self.primary.clone(), self.secondary.clone()),
      close,
      id: self.id.clone(),
      key: self.key.clone(),
    }
  }

  /// Builds what `view` shows: the theme's renderer, or the default one.
  fn render_view(view: TeachingTipView) -> Element {
    match view.theme().renderer::<TeachingTip>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// What its close control does: tells what it is to tell, and sets the signal to `false`.
  fn closer(&self) -> Closer {
    let (open, closers) = (self.open.clone(), self.on_close.clone());
    Arc::new(move || {
      for close in &closers {
        close();
      }
      open.set(false);
    })
  }

  /// Builds the teaching tip's panel as it shows, anchored to nothing, in
  /// `theme`. Converting a teaching tip into an element does this in the
  /// theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let close = self.closer();
    Self::render_view(self.view(theme, side_of(self.placement), close))
  }
}

impl From<TeachingTip> for Element {
  fn from(tip: TeachingTip) -> Self {
    tip.build(&current_theme())
  }
}

/// A teaching tip as a renderer sees it: what it says, the side it is on, and
/// the style of its parts.
pub struct TeachingTipView {
  pub title: Option<Arc<str>>,
  pub description: Option<Arc<str>>,
  pub media: Option<Element>,
  /// What it holds, under the description.
  pub children: Vec<Element>,
  /// Its step and how many there are, counted from 1.
  pub step: Option<(usize, usize)>,
  /// The label of the button that goes on.
  pub primary: Option<Arc<str>>,
  /// The label of the button beside it.
  pub secondary: Option<Arc<str>>,
  pub close_button: bool,
  pub variant: Name,
  /// The side it is on: the side its placement gives, or the other where there was no room.
  pub side: TooltipSide,
  pub arrow: bool,
  /// How far along the panel's edge from its middle the arrow is, toward the
  /// middle of what it points at.
  pub aim: f32,
  /// The style of each part of this teaching tip.
  pub recipe: ResolvedRecipe,
  actions: (Option<Action>, Option<Action>),
  close: Closer,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl TeachingTipView {
  /// The style of a part, as this teaching tip is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  fn primary_look(&self) -> bool {
    self.variant == Name::from(TeachingTipVariant::Primary)
  }

  /// A text in the style of `part`, as wide as the room it has.
  fn wrapping(&self, part: TeachingTipPart, content: &str) -> Column {
    let part = self.part(part);
    let fill = Dimension::Pct(100.0);
    Column::new()
      .width(fill)
      .min_width(Dimension::Px(0.0))
      .part(&part)
      .child(part.text(content).width(fill))
  }

  /// The control that closes it: a ghost button that is its icon alone,
  /// styled over the theme's button by [`TeachingTipPart::Close`].
  pub fn close(&self) -> Button {
    let close = self.close.clone();
    let button = Button::new("Close");
    let button = match &self.id {
      Some(id) => button.id(format!("{id}-close")),
      None => button,
    };
    button
      .variant(ButtonVariant::Ghost)
      .size(ButtonSize::Xs)
      .icon(icons::CLOSE)
      .icon_only(true)
      .rules(&self.recipe.rules_for(TeachingTipPart::Close, ButtonPart::Root))
      .on_click(move |_| close())
  }

  /// A button of the footer, styled over the theme's button by `part`.
  fn action(&self, action: &Action, part: TeachingTipPart, variant: ButtonVariant) -> Button {
    let size = if self.step.is_some() {
      ButtonSize::Xs
    } else {
      ButtonSize::Sm
    };
    let mut button = Button::new(&action.label);
    if let Some(id) = &self.id {
      let name = if part == TeachingTipPart::PrimaryAction {
        "primary"
      } else {
        "secondary"
      };
      button = button.id(format!("{id}-{name}"));
    }
    button = button
      .variant(variant)
      .size(size)
      .rules(&self.recipe.rules_for(part, ButtonPart::Root));
    for handler in &action.on_click {
      button = button.on_click(handler.clone());
    }
    button
  }

  /// The title and the close control at its end. `None` for a tip with neither.
  pub fn header(&self) -> Option<Row> {
    if self.title.is_none() && !self.close_button {
      return None;
    }
    let mut header = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .justify(Justify::End)
      .part(&self.part(TeachingTipPart::Header));
    if let Some(title) = &self.title {
      header = header.child(self.wrapping(TeachingTipPart::Title, title).flex(1.0));
    }
    if self.close_button {
      header = header.child(self.close());
    }
    Some(header)
  }

  /// The step count at its start and the buttons at its end. `None` for a tip
  /// with neither.
  pub fn footer(&self) -> Option<Row> {
    let (primary, secondary) = &self.actions;
    if self.step.is_none() && primary.is_none() && secondary.is_none() {
      return None;
    }
    // On the primary color the button that goes on is the light one, and the other a ghost.
    let (go, back) = if self.primary_look() {
      (ButtonVariant::Secondary, ButtonVariant::Ghost)
    } else {
      (ButtonVariant::Primary, ButtonVariant::Outline)
    };
    let mut footer = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .justify(if self.step.is_some() {
        Justify::SpaceBetween
      } else {
        Justify::End
      })
      .part(&self.part(TeachingTipPart::Footer));
    if let Some((step, of)) = self.step {
      let part = self.part(TeachingTipPart::Step);
      footer = footer.child(
        Row::new()
          .part(&part)
          .child(part.text(&format!("{step} of {of}")).nowrap()),
      );
    }
    let mut actions = Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(TeachingTipPart::Actions));
    if let Some(secondary) = secondary {
      actions = actions.child(self.action(secondary, TeachingTipPart::SecondaryAction, back));
    }
    if let Some(primary) = primary {
      actions = actions.child(self.action(primary, TeachingTipPart::PrimaryAction, go));
    }
    Some(footer.child(actions))
  }

  /// The panel: the media at its top over the content, the header, the
  /// description and what it holds over the footer. A dialog to what reads
  /// the tree, called by its title.
  pub fn panel(&mut self) -> Column {
    let mut panel = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(TeachingTipPart::Panel))
      .describe("a11y_role", "dialog");
    if let Some(title) = &self.title {
      panel = panel.describe("a11y_name", title.clone());
    }
    if let Some(media) = self.media.take() {
      // It lies along the panel's rounded top, which cuts it: a part of its own that clipped would be cut square.
      panel = panel.child(
        Row::new()
          .justify(Justify::Center)
          .align_items(Alignment::Center)
          .overflow_visible()
          .part(&self.part(TeachingTipPart::Media))
          .child(media),
      );
    }
    let mut text = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(TeachingTipPart::Text));
    if let Some(header) = self.header() {
      text = text.child(header);
    }
    if let Some(description) = &self.description {
      text = text.child(self.wrapping(TeachingTipPart::Description, description));
    }
    for child in std::mem::take(&mut self.children) {
      text = text.child(child);
    }
    let mut content = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(TeachingTipPart::Content))
      .child(text);
    if let Some(footer) = self.footer() {
      content = content.child(footer);
    }
    panel.child(content)
  }

  /// How long the arrow is along the panel's edge, and how far out from it
  /// it goes. `None` without an arrow.
  pub fn arrow_size(&self) -> Option<(f32, f32)> {
    if !self.arrow {
      return None;
    }
    arrow_size(&self.part(TeachingTipPart::Arrow), self.side)
  }

  /// The arrow, in the panel's fill and with its edge, on the side toward
  /// what it points at. `None` without one.
  pub fn arrow(&self) -> Option<Element> {
    if !self.arrow {
      return None;
    }
    arrow(
      &self.part(TeachingTipPart::Arrow),
      &self.part(TeachingTipPart::Panel),
      self.side,
      self.aim,
    )
  }

  /// How the default renderer builds a teaching tip: the panel, and the
  /// arrow on its side toward what it points at.
  pub fn render(mut self) -> Element {
    let away = self.arrow_size().map_or(0.0, |(_, away)| away);
    let arrow = self.arrow();
    let panel = self.panel();
    let mut root: Element = frame(&self.part(TeachingTipPart::Root), panel.into(), arrow, away, self.side).into();
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }
}

impl Themed for TeachingTip {
  const NAME: &'static str = "teaching-tip";
  type Renderer = TeachingTipRenderer;

  /// The design's teaching tip and tour (`pencil/lurq-design-system.pen`,
  /// Teaching Tip and Tour). The teaching tip is a panel 300 wide in the
  /// popover's colors, with an edge of 1 in the border's color, the large
  /// radius and the large shadow: media 96 high on the muted fill, then 14 of
  /// padding around the text and, 10 under it, the footer. The title is 14
  /// at 600 with a close control of 20 at its end, its icon of 16 muted; the
  /// description 6 under it is 13 of the muted foreground in lines of 1.45;
  /// the buttons, 8 apart at the end of the footer, are 30 high and 12 from
  /// their sides. Its arrow is 18 along the panel and 9 out from it, with the
  /// panel's edge, and it stands 4 from what it points at. A tip that is a
  /// step of several, the tour, is 280 wide and padded 16, its title at 500
  /// over the description 4 under it in lines of 1.5, its footer 12 under
  /// them with the count, "2 of 4", in mono 11 of the muted foreground at its
  /// start, and buttons 28 high and 10 from their sides. In the primary look
  /// it is in the primary color and its foreground, edge and all. It fades in
  /// and out as the popover does.
  fn recipe() -> Recipe {
    use PaletteColor::{
      Border as Edge, Muted, MutedForeground, Popover, PopoverForeground, Primary, PrimaryForeground,
    };
    use TeachingTipPart::{
      Actions, Arrow, Close, Content, Description, Footer, Header, Media, Panel, PrimaryAction, Root, SecondaryAction,
      Step, Text, Title,
    };
    use TeachingTipState::Stepped;
    use TooltipSide::{Bottom, Left, Right, Top};
    let primary = || Selector::new().variant(TeachingTipVariant::Primary);
    Recipe::new()
      .base(Root, |s| {
        s.enter(PartAnimation::new(FADE_IN, MotionDuration::Base, MotionEasing::Enter))
          .exit(PartAnimation::new(FADE_OUT, MotionDuration::Fast, MotionEasing::Exit))
      })
      .base(Panel, |s| {
        s.width(300.0)
          .radius(RadiusSize::Lg)
          .background(Popover)
          .foreground(PopoverForeground)
          .border(Border::inside(BorderSize::Sm, Edge))
          .shadow(ShadowStyle::Lg)
      })
      .base(Media, |s| {
        s.height(96.0).background(Muted).foreground(MutedForeground)
      })
      .base(Content, |s| s.padding_x(14.0).padding_y(14.0).gap(10.0))
      .base(Text, |s| s.gap(6.0))
      .base(Header, |s| s.gap(8.0))
      .base(Title, |s| {
        s.typography(TypographyStyle::Body).weight(FontWeight::SemiBold)
      })
      .base(Close, |s| {
        s.square(20.0)
          .padding_x(0.0)
          .radius(RadiusSize::Sm)
          .foreground(MutedForeground)
      })
      .base(Description, |s| {
        s.typography(TypographyStyle::Description)
          .font_size(13.0)
          .line_height(1.45)
          .foreground(MutedForeground)
      })
      .base(Footer, |s| s.gap(8.0))
      .base(Step, |s| {
        s.font_family(FontFamily::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      .base(Actions, |s| s.gap(8.0))
      .base(PrimaryAction, |s| s.height(30.0).padding_x(12.0))
      .base(SecondaryAction, |s| s.height(30.0).padding_x(12.0))
      .base(Arrow, |s| s.width(18.0).height(9.0))
      .state(Left, Arrow, |s| s.width(9.0).height(18.0))
      .state(Right, Arrow, |s| s.width(9.0).height(18.0))
      // The room between the tip and what it points at, toward it.
      .state(Top, Root, |s| s.padding_bottom(4.0))
      .state(Bottom, Root, |s| s.padding_top(4.0))
      .state(Left, Root, |s| s.padding_end(4.0))
      .state(Right, Root, |s| s.padding_start(4.0))
      // A step of several: the design's tour.
      .state(Stepped, Panel, |s| s.width(280.0))
      .state(Stepped, Content, |s| s.padding_x(16.0).padding_y(16.0).gap(12.0))
      .state(Stepped, Text, |s| s.gap(4.0))
      .state(Stepped, Title, |s| s.weight(FontWeight::Medium))
      .state(Stepped, Description, |s| s.line_height(1.5))
      .state(Stepped, PrimaryAction, |s| s.height(28.0).padding_x(10.0))
      .state(Stepped, SecondaryAction, |s| s.height(28.0).padding_x(10.0))
      // The primary look: the panel and all it says in the primary color and its foreground.
      .when(primary(), Panel, |s| {
        s.background(Primary)
          .foreground(PrimaryForeground)
          .border(Border::inside(BorderSize::Sm, Primary))
      })
      .when(primary(), Media, |s| {
        s.background(Color::from_hex("#FFFFFF1A"))
          .foreground(PrimaryForeground)
      })
      .when(primary(), Description, |s| s.foreground(PrimaryForeground))
      .when(primary(), Step, |s| s.foreground(PrimaryForeground))
      .when(primary(), Close, |s| s.foreground(PrimaryForeground))
      .when(primary(), SecondaryAction, |s| s.foreground(PrimaryForeground))
  }
}

/// A teaching tip.
#[derive(Clone)]
struct TeachingTipProps {
  tip: TeachingTip,
}

/// The tip is built anew by every render of what holds it, so props are
/// never the same as before.
impl PartialEq for TeachingTipProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for TeachingTipProps {}

/// The layer's own open signal while the app's is `true`, and what is told
/// when the runtime turns it off on Escape.
struct Layer {
  open: Signal<bool>,
  watch: Option<Subscription<bool>>,
}

/// What shows a teaching tip by its anchor while the app says so.
struct TeachingTipHost {
  /// What shows.
  shown: ElementRef,
  placed: Placed,
  layer: Mutex<Layer>,
  /// What the last render says its close control does: Escape does it too.
  closer: Arc<Mutex<Option<Closer>>>,
}

impl Component for TeachingTipHost {
  type Props = TeachingTipProps;

  fn create(ctx: &mut Ctx) -> Self {
    let shown = ElementRef::new();
    let placed = Placed::new(ctx, &shown);
    Self {
      shown,
      placed,
      layer: Mutex::new(Layer {
        open: Signal::new(false),
        watch: None,
      }),
      closer: Arc::new(Mutex::new(None)),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let TeachingTipProps { tip } = ctx.props::<TeachingTipProps>().clone();
    let close = tip.closer();
    *self.closer.lock() = Some(close.clone());
    // Reading it subscribes: this renders again when the app opens or closes it.
    let open = tip.open.get();
    let layer = {
      let mut layer = self.layer.lock();
      if !open {
        layer.watch = None;
        None
      } else {
        if layer.watch.is_none() {
          // It opens: the layer has a signal of its own, which the runtime turns off on Escape. That closes it as
          // its close control does.
          let signal = Signal::new(true);
          let closer = self.closer.clone();
          layer.watch = Some(signal.subscribe(move |shown| {
            let close = closer.lock().clone();
            if !*shown && let Some(close) = close {
              close();
            }
          }));
          layer.open = signal;
        }
        Some(layer.open.clone())
      }
    };

    let side = self.placed.side(tip.placement);
    let mut view = tip.view(&current_theme(), side, close);
    view.aim = self.placed.aim();
    self.placed.note(
      tip.placement,
      &tip.anchor,
      view.arrow_size().map_or(0.0, |(along, _)| along),
    );
    let shown = TeachingTip::render_view(view).map_node(|node| node.ref_element(self.shown.clone()));
    let popup = Popup::new(tip.anchor.clone(), shown)
      .placement(tip.placement)
      .collision(CollisionStrategy::FlipThenClamp)
      // A press elsewhere is the person going on with what they do beside it.
      .dismiss_on_outside_click(false)
      .takes_focus(tip.takes_focus)
      // Without taking the focus, Tab goes from what it points at into its buttons.
      .tab_after_anchor(!tip.takes_focus);
    let popup = match layer {
      Some(open) => popup.open(open).dismiss_on_escape(true),
      None => popup.open_when(false),
    };
    Row::new().child(popup)
  }
}

/// A step of a [`Tour`]: what it points at and what it says.
#[derive(Clone)]
pub struct TourStep {
  anchor: ElementRef,
  title: Arc<str>,
  description: Option<Arc<str>>,
  media: Option<Element>,
  placement: Option<Placement>,
}

impl TourStep {
  /// A step pointing at `anchor`, called `title`.
  pub fn new(anchor: impl Into<ElementRef>, title: &str) -> Self {
    Self {
      anchor: anchor.into(),
      title: Arc::from(title),
      description: None,
      media: None,
      placement: None,
    }
  }

  /// What it is for, under the title.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// A picture or an icon at the top of the panel.
  pub fn media(mut self, media: impl Into<Element>) -> Self {
    self.media = Some(media.into());
    self
  }

  /// Which side of what it points at it is on, in place of the tour's.
  pub fn placement(mut self, placement: Placement) -> Self {
    self.placement = Some(placement);
    self
  }
}

/// A tour: the app's teaching tips shown one after another, each with its
/// count, "2 of 4", a button back and one on, the last one's "Done".
///
/// ```no_run
/// use lurq::{
///   app::ctx::Ctx,
///   core::{ElementRef, Signal},
///   node::Element,
///   ui::{Tour, TourStep},
/// };
///
/// // Which step shows, kept by the app: `None` when the tour is not on.
/// fn tour(ctx: &mut Ctx, step: Signal<Option<usize>>, theme: ElementRef, panels: ElementRef) -> Element {
///   Tour::new(step)
///     .step(TourStep::new(theme, "Switch themes here").description("Light and dark palettes share the same roles."))
///     .step(TourStep::new(panels, "Dock panels anywhere"))
///     .mount(ctx)
/// }
/// ```
///
/// It shows the step its signal says, and sets the signal: to the step
/// before or after on a press of its buttons, and to `None` after the last
/// and on Escape. Each step is a [`TeachingTip`] with no close control.
#[derive(Clone)]
pub struct Tour {
  current: Signal<Option<usize>>,
  steps: Vec<TourStep>,
  back: Arc<str>,
  next: Arc<str>,
  done: Arc<str>,
  variant: Name,
  placement: Placement,
  arrow: bool,
  on_finish: Vec<Closer>,
}

impl Tour {
  /// A tour showing the step `current` says, counted from 0, none while it is `None`.
  pub fn new(current: Signal<Option<usize>>) -> Self {
    Self {
      current,
      steps: Vec::new(),
      back: Arc::from("Back"),
      next: Arc::from("Next"),
      done: Arc::from("Done"),
      variant: TeachingTipVariant::Plain.into(),
      placement: Placement::Bottom,
      arrow: true,
      on_finish: Vec::new(),
    }
  }

  /// Adds a step after those it has.
  pub fn step(mut self, step: TourStep) -> Self {
    self.steps.push(step);
    self
  }

  /// Adds several steps.
  pub fn steps(mut self, steps: impl IntoIterator<Item = TourStep>) -> Self {
    self.steps.extend(steps);
    self
  }

  /// The labels of its buttons: back, on, and on from the last step.
  /// "Back", "Next" and "Done" by default.
  pub fn labels(mut self, back: &str, next: &str, done: &str) -> Self {
    (self.back, self.next, self.done) = (Arc::from(back), Arc::from(next), Arc::from(done));
    self
  }

  /// How its tips look.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// Which side of what they point at its tips are on, where a step does not
  /// say: under it by default.
  pub fn placement(mut self, placement: Placement) -> Self {
    self.placement = placement;
    self
  }

  /// Whether its tips have an arrow. They have by default.
  pub fn arrow(mut self, arrow: bool) -> Self {
    self.arrow = arrow;
    self
  }

  /// What is told when the last step's button is pressed, not when Escape
  /// leaves the tour.
  pub fn on_finish(mut self, on_finish: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_finish.push(Arc::new(on_finish));
    self
  }

  /// Puts the tour in the app. The element it returns has no size, and goes anywhere.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    ctx.mount::<TourHost>(TourProps { tour: self })
  }
}

/// A tour.
#[derive(Clone)]
struct TourProps {
  tour: Tour,
}

impl PartialEq for TourProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for TourProps {}

/// What shows a tour's step.
struct TourHost {
  /// The step it showed last: what fades out as the tour ends.
  last: Mutex<usize>,
  /// Whether its tip is open: while the app's signal says a step.
  open: Signal<bool>,
}

impl Component for TourHost {
  type Props = TourProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      last: Mutex::new(0),
      open: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let TourProps { tour } = ctx.props::<TourProps>().clone();
    let current = tour.current.get();
    let count = tour.steps.len();
    let shown = current.filter(|index| *index < count);
    let index = {
      let mut last = self.last.lock();
      if let Some(index) = shown {
        *last = index;
      }
      *last
    };
    let Some(step) = tour.steps.get(index) else {
      return Element::new();
    };
    // The tip's signal follows the app's; the tip sets it on Escape, which ends the tour.
    super::tip::put(&self.open, shown.is_some());
    let mut tip = TeachingTip::new(step.anchor.clone(), self.open.clone())
      .title(&step.title)
      .step(index + 1, count)
      .close_button(false)
      .variant(tour.variant)
      .placement(step.placement.unwrap_or(tour.placement))
      .arrow(tour.arrow);
    if let Some(description) = &step.description {
      tip = tip.description(description);
    }
    if let Some(media) = &step.media {
      tip = tip.media(media.clone());
    }
    if index > 0 {
      let to = tour.current.clone();
      tip = tip.secondary(&tour.back, move |_| to.set(Some(index - 1)));
    }
    let last = index + 1 >= count;
    let (to, finish) = (tour.current.clone(), tour.on_finish.clone());
    tip = tip.primary(if last { &tour.done } else { &tour.next }, move |_| {
      if last {
        to.set(None);
        for finish in &finish {
          finish();
        }
      } else {
        to.set(Some(index + 1));
      }
    });
    let to = tour.current.clone();
    tip.on_close(move || to.set(None)).mount(ctx)
  }
}
