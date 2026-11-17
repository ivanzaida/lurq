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
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{FADE_IN, FADE_OUT},
  theme::{Themed, current_theme},
  tip::{Placed, arrow, arrow_size, frame, side_of, with_offset},
};
use crate::{
  app::{
    component::Component,
    ctx::{CollisionStrategy, Ctx, Placement},
    events::{KeyboardEvent, MouseEvent},
    theme::{BorderSize, MotionDuration, MotionEasing, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle},
  },
  components::{Column, Popup, Row},
  core::{ElementRef, Signal, Subscription},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, FocusGroup, SpacingValue, border::Border, color::Color, dimension::Dimension},
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
  /// The quiet button before them, that leaves: what it gives the theme's button.
  TertiaryAction,
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
  TertiaryAction => "tertiary-action",
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
  Step,
  /// It has media at its top.
  Media,
}

names!(TeachingTipState {
  Arrow => "arrow",
  Step => "step",
  Media => "media",
});

/// What builds a teaching tip's panel: a theme's in place of the default one.
/// Where it is shown, and when, stays the teaching tip's.
pub type TeachingTipRenderer = Arc<dyn Fn(TeachingTipView) -> Element + Send + Sync>;

/// What is told as a teaching tip closes, and what a press of one of its
/// buttons does.
type Closer = Arc<dyn Fn() + Send + Sync>;

/// What a tour does with a key pressed while the focus is in its tip.
type Keys = Arc<dyn Fn(&KeyboardEvent) + Send + Sync>;

/// The keys of a tour's buttons, by which the focus is moved to one of them.
const PRIMARY_KEY: &str = "teaching-tip-primary";
const SECONDARY_KEY: &str = "teaching-tip-secondary";
const TERTIARY_KEY: &str = "teaching-tip-tertiary";

/// A button of a teaching tip: its label, and what a press of it does.
#[derive(Clone)]
struct Action {
  label: Arc<str>,
  on_click: Closer,
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
/// fn dock_tip(ctx: &mut Ctx, open: Signal<bool>, panels: ElementRef) -> Element {
///   let close = open.clone();
///   TeachingTip::new(open, panels)
///     .title("Dock panels anywhere")
///     .description("Drag a panel by its header and drop it on any edge of the window.")
///     .primary("Got it", move || close.set(false))
///     .mount(ctx)
/// }
/// ```
///
/// It is under what it points at and centered on it unless its placement
/// says otherwise. It does not keep whether it is open: the app's signal
/// does. Its close control and Escape close it, which sets the signal to
/// `false` and then tells [`TeachingTip::on_dismiss`]; a press elsewhere does
/// not, so a person can go on with what they are doing beside it. Its own
/// buttons do what their handlers say. It does not take the focus from what
/// a person is doing unless the app says so ([`TeachingTip::takes_focus`]):
/// Tab goes from what it points at into its buttons and on. To tooling it is
/// a dialog called by its label or else its title. Where there is no room on
/// its side it goes to the other, its arrow turned with it, and it is kept in
/// the window.
///
/// Converted into an element on its own, a teaching tip is its panel as it
/// shows, anchored to nothing: for a gallery, a test, or a host of an app's own.
#[derive(Clone)]
pub struct TeachingTip {
  open: Signal<bool>,
  anchor: ElementRef,
  label: Option<Arc<str>>,
  title: Option<Arc<str>>,
  description: Option<Arc<str>>,
  media: Option<Element>,
  children: Vec<Element>,
  step: Option<(usize, usize)>,
  primary: Option<Action>,
  secondary: Option<Action>,
  tertiary: Option<Action>,
  /// What a tour does with a key while the focus is in the panel.
  keys: Option<Keys>,
  /// Whether it is a tour's: its buttons are a group the focus is moved in by their keys.
  grouped: bool,
  /// Whether a press of the secondary button takes that button away: the focus goes to the primary one.
  refocus: bool,
  on_dismiss: Vec<Closer>,
  dismiss_on_escape: bool,
  dismiss_on_outside_press: bool,
  close_button: bool,
  variant: Name,
  placement: Placement,
  offset: Option<SpacingValue>,
  collision: CollisionStrategy,
  arrow: bool,
  takes_focus: bool,
  traps_tab: bool,
  width: Option<Length>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl TeachingTip {
  /// A teaching tip pointing at `anchor` while `open` is `true`: under it and
  /// centered on it (`Placement::Bottom`), with an arrow and a close control,
  /// in the plain look.
  pub fn new(open: Signal<bool>, anchor: impl Into<ElementRef>) -> Self {
    Self {
      open,
      anchor: anchor.into(),
      label: None,
      title: None,
      description: None,
      media: None,
      children: Vec::new(),
      step: None,
      primary: None,
      secondary: None,
      tertiary: None,
      keys: None,
      grouped: false,
      refocus: false,
      on_dismiss: Vec::new(),
      dismiss_on_escape: true,
      dismiss_on_outside_press: false,
      close_button: true,
      variant: TeachingTipVariant::Plain.into(),
      placement: Placement::Bottom,
      offset: None,
      collision: CollisionStrategy::FlipThenClamp,
      arrow: true,
      takes_focus: false,
      traps_tab: false,
      width: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// What tooling calls it, in place of its title: for a tip with no title,
  /// or one whose title says too little alone.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// What it points at, in a few words, at the top of its text. What tooling
  /// calls it, unless its label says otherwise.
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

  /// Adds several things it holds.
  pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.children.extend(children.into_iter().map(Into::into));
    self
  }

  /// That it is the step `step` of `of`, counted from 1: "2 of 4" in the footer.
  pub fn step(mut self, step: usize, of: usize) -> Self {
    self.step = Some((step, of));
    self
  }

  /// The button that goes on, at the end of the footer, "Next", "Got it", and
  /// what a press of it does. A second call takes the place of the first.
  pub fn primary(mut self, label: &str, on_click: impl Fn() + Send + Sync + 'static) -> Self {
    self.primary = Some(Action {
      label: Arc::from(label),
      on_click: Arc::new(on_click),
    });
    self
  }

  /// The button before it, "Back", "Not now", and what a press of it does.
  pub fn secondary(mut self, label: &str, on_click: impl Fn() + Send + Sync + 'static) -> Self {
    self.secondary = Some(Action {
      label: Arc::from(label),
      on_click: Arc::new(on_click),
    });
    self
  }

  /// A quiet button before the others, "Skip", "Remind me later", and what a press of it does.
  pub fn tertiary(mut self, label: &str, on_click: impl Fn() + Send + Sync + 'static) -> Self {
    self.tertiary = Some(Action {
      label: Arc::from(label),
      on_click: Arc::new(on_click),
    });
    self
  }

  /// What is told after its close control, Escape or a press outside it (where
  /// [`dismiss_on_outside_press`](Self::dismiss_on_outside_press) says so) closed it: the signal is `false` already.
  /// A second handler is called after the first.
  pub fn on_dismiss(mut self, on_dismiss: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_dismiss.push(Arc::new(on_dismiss));
    self
  }

  /// Whether Escape closes it, as its close control does. It does by default.
  pub fn dismiss_on_escape(mut self, dismiss: bool) -> Self {
    self.dismiss_on_escape = dismiss;
    self
  }

  /// Whether a press outside it closes it, as its close control does. It does not by default: a press elsewhere
  /// is the person going on with what they do beside it.
  pub fn dismiss_on_outside_press(mut self, dismiss: bool) -> Self {
    self.dismiss_on_outside_press = dismiss;
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
  /// up with: under it and centered (`Placement::Bottom`) by default.
  pub fn placement(mut self, placement: Placement) -> Self {
    self.placement = placement;
    self
  }

  /// How far it stands from what it points at, in place of what the theme
  /// gives it (4 in the design's, and its arrow's 9).
  pub fn offset(mut self, offset: impl Into<SpacingValue>) -> Self {
    self.offset = Some(offset.into());
    self
  }

  /// What it does where there is no room for it on its side: goes to the
  /// other side, then is moved into the window, by default.
  pub fn collision(mut self, collision: CollisionStrategy) -> Self {
    self.collision = collision;
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

  /// Whether Tab stays in it while the focus is in it, going round from its last button to its first: for a tip
  /// a person is to finish before going on. Off by default, when Tab goes on from its last button to what
  /// follows what it points at; a [`Tour`] turns it on for its card.
  pub fn traps_tab(mut self, traps: bool) -> Self {
    self.traps_tab = traps;
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

  /// Draws its close control as it is under the pointer, held down or
  /// focused from the keyboard, with nothing pointing at it: for a gallery or
  /// a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of its root, what shows. Its close control is `<id>-close`, and
  /// its buttons `<id>-primary`, `<id>-secondary` and `<id>-tertiary`.
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
    .state(TeachingTipState::Step, self.step.is_some())
    .state(TeachingTipState::Media, self.media.is_some());
    // A width and an offset given here are under what `style` and `rules` say of the panel and the root.
    let mut rules = Recipe::new();
    if let Some(width) = self.width {
      rules = rules.base(TeachingTipPart::Panel, |s| s.width(width));
    }
    if let Some(offset) = self.offset {
      rules = with_offset(rules, TeachingTipPart::Root, side, offset);
    }
    let rules = rules.extend(&self.rules);
    TeachingTipView {
      label: self.label.clone().or_else(|| self.title.clone()),
      title: self.title.clone(),
      description: self.description.clone(),
      media: self.media.clone(),
      children: self.children.clone(),
      step: self.step,
      primary: self.primary.as_ref().map(|action| action.label.clone()),
      secondary: self.secondary.as_ref().map(|action| action.label.clone()),
      tertiary: self.tertiary.as_ref().map(|action| action.label.clone()),
      close_button: self.close_button,
      variant: self.variant,
      side,
      arrow: self.arrow,
      aim: 0.0,
      recipe: theme.resolve::<TeachingTip>(&case, Some(&rules)),
      actions: (self.primary.clone(), self.secondary.clone(), self.tertiary.clone()),
      keys: self.keys.clone(),
      grouped: self.grouped,
      refocus: self.refocus,
      close,
      forced: self.forced,
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

  /// What its close control does: sets the signal to `false`, and then tells
  /// what it is to tell, so that a handler that opens something else does not
  /// have it closed under it.
  fn closer(&self) -> Closer {
    let (open, closers) = (self.open.clone(), self.on_dismiss.clone());
    Arc::new(move || {
      open.set(false);
      for close in &closers {
        close();
      }
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
  /// What tooling calls it: its label, or else its title.
  pub label: Option<Arc<str>>,
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
  /// The label of the quiet button before them.
  pub tertiary: Option<Arc<str>>,
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
  actions: (Option<Action>, Option<Action>, Option<Action>),
  keys: Option<Keys>,
  grouped: bool,
  refocus: bool,
  close: Closer,
  forced: Option<Interaction>,
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
    let button = Button::new(&super::current_locale().common.close);
    let button = match &self.id {
      Some(id) => button.id(format!("{id}-close")),
      None => button,
    };
    button
      .variant(ButtonVariant::Ghost)
      .size(ButtonSize::Xs)
      .icon(icons::CLOSE)
      .icon_only(true)
      .force(self.forced)
      .rules(&self.recipe.rules_for(TeachingTipPart::Close, ButtonPart::Root))
      .on_click(move |_| close())
  }

  /// A button of the footer, styled over the theme's button by `part`.
  fn action(&self, action: &Action, part: TeachingTipPart, variant: ButtonVariant) -> Button {
    let (name, key) = match part {
      TeachingTipPart::PrimaryAction => ("primary", PRIMARY_KEY),
      TeachingTipPart::SecondaryAction => ("secondary", SECONDARY_KEY),
      _ => ("tertiary", TERTIARY_KEY),
    };
    let mut button = Button::new(&action.label);
    if let Some(id) = &self.id {
      button = button.id(format!("{id}-{name}"));
    }
    if self.grouped {
      // A tour's button is the same one from step to step, so the focus stays on it.
      button = button.key(key);
    }
    let handler = action.on_click.clone();
    // The button that goes back is not there on the first step: the focus it had goes to the one that goes on.
    let refocus = self.refocus && part == TeachingTipPart::SecondaryAction;
    button
      .variant(variant)
      .size(ButtonSize::Sm)
      .rules(&self.recipe.rules_for(part, ButtonPart::Root))
      .on_click(move |event: MouseEvent| {
        handler();
        if refocus {
          event.focus_key(PRIMARY_KEY);
        }
      })
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
    let (primary, secondary, tertiary) = &self.actions;
    if self.step.is_none() && primary.is_none() && secondary.is_none() && tertiary.is_none() {
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
        Row::new().part(&part).child(
          part
            .text(&crate::core::fill(
              &super::current_locale().common.place_of,
              &[("at", &step.to_string()), ("total", &of.to_string())],
            ))
            .nowrap(),
        ),
      );
    }
    let mut actions = Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(TeachingTipPart::Actions));
    if self.grouped {
      // A group for the focus to be moved in by a button's key; its own arrows are the tour's, which steps.
      actions = actions.focus_group(FocusGroup::horizontal().wrap(false));
    }
    if let Some(tertiary) = tertiary {
      actions = actions.child(self.action(tertiary, TeachingTipPart::TertiaryAction, ButtonVariant::Ghost));
    }
    if let Some(secondary) = secondary {
      actions = actions.child(self.action(secondary, TeachingTipPart::SecondaryAction, back));
    }
    if let Some(primary) = primary {
      let button = self.action(primary, TeachingTipPart::PrimaryAction, go);
      actions = if self.grouped {
        // A tour's card takes the focus on the button that goes on, whatever stands before it: Enter on a tour
        // that has just started goes on, and does not skip it or go back.
        actions.child(super::message_box::autofocus(button.build(self.theme())))
      } else {
        actions.child(button)
      };
    }
    Some(footer.child(actions))
  }

  /// The panel: the media at its top over the content, the header, the
  /// description and what it holds over the footer. A dialog to what reads
  /// the tree, called by its label.
  pub fn panel(&mut self) -> Column {
    let mut panel = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(TeachingTipPart::Panel))
      .describe("a11y_role", "dialog");
    if let Some(label) = &self.label {
      panel = panel.describe("a11y_name", label.clone());
    }
    if let Some(keys) = self.keys.clone() {
      panel = panel.on_key_down(move |event: KeyboardEvent| {
        if event.focus_within {
          keys(&event);
        }
      });
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

impl crate::ui::Styled for TeachingTip {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for TeachingTip {
  const NAME: &'static str = "teaching-tip";
  type Renderer = TeachingTipRenderer;

  fn parts() -> crate::ui::Declared {
    TeachingTipPart::declared()
  }

  fn states() -> crate::ui::Declared {
    TeachingTipState::declared()
  }

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
      Step, TertiaryAction, Text, Title,
    };
    use TooltipSide::{Bottom, Left, Right, Top};
    let primary = || Selector::new().variant(TeachingTipVariant::Primary);
    Recipe::new()
      .base(Root, |s| {
        s.enter(PartAnimation::new(FADE_IN, MotionDuration::Base, MotionEasing::Enter))
          .exit(PartAnimation::new(FADE_OUT, MotionDuration::Fast, MotionEasing::Exit))
      })
      .base(Panel, |s| {
        s.width(300.0)
          .max_width(Length::Fill)
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
      .base(TertiaryAction, |s| s.height(30.0).padding_x(12.0))
      .base(Arrow, |s| s.width(18.0).height(9.0))
      .state(Left, Arrow, |s| s.width(9.0).height(18.0))
      .state(Right, Arrow, |s| s.width(9.0).height(18.0))
      // The room between the tip and what it points at, toward it.
      .state(Top, Root, |s| s.padding_bottom(4.0))
      .state(Bottom, Root, |s| s.padding_top(4.0))
      .state(Left, Root, |s| s.padding_end(4.0))
      .state(Right, Root, |s| s.padding_start(4.0))
      // A step of several: the design's tour.
      .state(TeachingTipState::Step, Panel, |s| s.width(280.0).shadow(ShadowStyle::Md))
      .state(TeachingTipState::Step, Content, |s| s.padding_x(16.0).padding_y(16.0).gap(12.0))
      .state(TeachingTipState::Step, Text, |s| s.gap(4.0))
      .state(TeachingTipState::Step, Title, |s| s.weight(FontWeight::Medium))
      .state(TeachingTipState::Step, Description, |s| s.line_height(1.5))
      .state(TeachingTipState::Step, PrimaryAction, |s| s.height(28.0).padding_x(10.0))
      .state(TeachingTipState::Step, SecondaryAction, |s| s.height(28.0).padding_x(10.0))
      .state(TeachingTipState::Step, TertiaryAction, |s| s.height(28.0).padding_x(10.0))
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
      .when(primary(), TertiaryAction, |s| s.foreground(PrimaryForeground))
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
      .collision(tip.collision)
      .takes_focus(tip.takes_focus)
      // Without taking the focus, Tab goes from what it points at into its buttons.
      .tab_after_anchor(!tip.takes_focus)
      .traps_tab(tip.traps_tab);
    let popup = match layer {
      Some(open) => popup
        .open(open)
        .dismiss_on_escape(tip.dismiss_on_escape)
        .dismiss_on_outside_click(tip.dismiss_on_outside_press),
      None => popup.open_when(false).dismiss_on_outside_click(false),
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
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
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
      id: None,
      key: None,
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

  /// The id of its tip's root while it shows, in place of the tour's. Its
  /// buttons are `<id>-primary` and `<id>-secondary`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// The key of its tip's root while it shows, in place of the tour's.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }
}

/// Which step a tour shows: the app's signal, or a value the app holds and is asked to change.
#[derive(Clone)]
enum Current {
  Bound(Signal<Option<usize>>),
  Held(Option<usize>),
}

/// What a tour's change of step is told to.
type StepHandler = Arc<dyn Fn(Option<usize>) + Send + Sync>;

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
///     .skip_button(true)
///     .mount(ctx)
/// }
/// ```
///
/// It shows the step its signal says, and sets the signal: to the step
/// before or after on a press of its buttons, and to `None` after the last,
/// on "Skip" and on Escape; then it tells [`on_change`](Self::on_change) the
/// step it is at. [`Tour::controlled`] is given the step as a value and only
/// tells what it is asked to be. Each step is a [`TeachingTip`] with no close
/// control, under what it points at and centered on it unless the tour or the
/// step says otherwise.
///
/// The focus goes into the step's card as the tour starts, on the button
/// that goes on whatever stands before it, stays on the
/// button that was pressed from step to step, and goes back to what had it as
/// the tour ends ([`takes_focus`](Self::takes_focus) says otherwise). Tab is
/// held in the card: from its last button it goes round to its first
/// ([`traps_tab`](Self::traps_tab) says otherwise). While
/// the focus is in the card the Right arrow goes on a step and the Left one
/// back, Home and End go to the first and the last, and Escape leaves.
///
/// Converted into an element on its own, a tour is the panel of the step its
/// signal says, or of its first step while it says none, anchored to
/// nothing: for a gallery or a test. Its buttons set the signal still.
#[derive(Clone)]
pub struct Tour {
  current: Current,
  steps: Vec<TourStep>,
  back: Arc<str>,
  next: Arc<str>,
  done: Arc<str>,
  skip: Arc<str>,
  skip_button: bool,
  variant: Name,
  placement: Placement,
  offset: Option<SpacingValue>,
  collision: CollisionStrategy,
  arrow: bool,
  takes_focus: bool,
  traps_tab: bool,
  dismiss_on_escape: bool,
  dismiss_on_outside_press: bool,
  on_change: Vec<StepHandler>,
  on_finish: Vec<Closer>,
  on_dismiss: Vec<Closer>,
  states: Vec<(Name, bool)>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Tour {
  /// A tour showing the step `current` says, counted from 0, none while it is `None`.
  pub fn new(current: Signal<Option<usize>>) -> Self {
    Self::showing(Current::Bound(current))
  }

  /// A tour showing the step `current`, counted from 0, none while it is `None`, which the app holds: a press of
  /// its buttons, its keys and Escape tell `on_change` the step it is asked to be at, and the app gives it that
  /// step, or does not.
  pub fn controlled(current: Option<usize>, on_change: impl Fn(Option<usize>) + Send + Sync + 'static) -> Self {
    Self::showing(Current::Held(current)).on_change(on_change)
  }

  fn showing(current: Current) -> Self {
    Self {
      current,
      steps: Vec::new(),
      back: super::current_locale().common.back.clone(),
      next: super::current_locale().common.next.clone(),
      done: super::current_locale().common.done.clone(),
      skip: super::current_locale().common.skip.clone(),
      skip_button: false,
      variant: TeachingTipVariant::Plain.into(),
      placement: Placement::Bottom,
      offset: None,
      collision: CollisionStrategy::FlipThenClamp,
      arrow: true,
      takes_focus: true,
      traps_tab: true,
      dismiss_on_escape: true,
      dismiss_on_outside_press: false,
      on_change: Vec::new(),
      on_finish: Vec::new(),
      on_dismiss: Vec::new(),
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
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

  /// The label of the button that goes back a step: "Back" by default.
  pub fn back_label(mut self, label: &str) -> Self {
    self.back = Arc::from(label);
    self
  }

  /// The label of the button that goes on a step: "Next" by default.
  pub fn next_label(mut self, label: &str) -> Self {
    self.next = Arc::from(label);
    self
  }

  /// The label of the button that goes on from the last step and ends the
  /// tour: "Done" by default.
  pub fn done_label(mut self, label: &str) -> Self {
    self.done = Arc::from(label);
    self
  }

  /// The label of the button that leaves the tour before its end: "Skip" by default.
  pub fn skip_label(mut self, label: &str) -> Self {
    self.skip = Arc::from(label);
    self
  }

  /// Whether every step but the last has a quiet button that leaves the tour, "Skip", before the others. A press
  /// of it leaves as Escape does. It has none by default, as the design draws none.
  pub fn skip_button(mut self, skip_button: bool) -> Self {
    self.skip_button = skip_button;
    self
  }

  /// How its tips look.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// Which side of what they point at its tips are on, where a step does not
  /// say: under it and centered (`Placement::Bottom`) by default.
  pub fn placement(mut self, placement: Placement) -> Self {
    self.placement = placement;
    self
  }

  /// How far its tips stand from what they point at, in place of what the
  /// theme gives them.
  pub fn offset(mut self, offset: impl Into<SpacingValue>) -> Self {
    self.offset = Some(offset.into());
    self
  }

  /// What its tips do where there is no room for them on their side: go to
  /// the other side, then are moved into the window, by default.
  pub fn collision(mut self, collision: CollisionStrategy) -> Self {
    self.collision = collision;
    self
  }

  /// Whether its tips have an arrow. They have by default.
  pub fn arrow(mut self, arrow: bool) -> Self {
    self.arrow = arrow;
    self
  }

  /// Whether the focus goes into the step's card as the tour starts, and back to what had it as the tour ends. It
  /// does by default: a tour is gone through before a person goes on. Without, Tab goes into the card from what
  /// it points at.
  pub fn takes_focus(mut self, takes: bool) -> Self {
    self.takes_focus = takes;
    self
  }

  /// Whether Tab stays in the step's card while the focus is in it: from its last button Tab goes to its first,
  /// and Shift+Tab from its first to its last. It does by default; without, Tab goes on from the card's last
  /// button to what follows what the step points at. The card closes nothing behind it either way: the page
  /// answers the pointer, and Escape leaves the tour.
  pub fn traps_tab(mut self, traps: bool) -> Self {
    self.traps_tab = traps;
    self
  }

  /// Whether Escape leaves the tour. It does by default.
  pub fn dismiss_on_escape(mut self, dismiss: bool) -> Self {
    self.dismiss_on_escape = dismiss;
    self
  }

  /// Whether a press outside the step's tip leaves the tour. It does not by default.
  pub fn dismiss_on_outside_press(mut self, dismiss: bool) -> Self {
    self.dismiss_on_outside_press = dismiss;
    self
  }

  /// Told the step the tour is at after a person changed it, `None` as they end or leave it: after a bound signal
  /// is set, and before [`on_finish`](Self::on_finish) and [`on_dismiss`](Self::on_dismiss). Not told when the app
  /// sets the signal, nor for a step it is at already. A second handler is called after the first.
  pub fn on_change(mut self, on_change: impl Fn(Option<usize>) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(on_change));
    self
  }

  /// What is told when the last step's button is pressed, after the signal is
  /// `None`; not when Escape leaves the tour. A second handler is called
  /// after the first.
  pub fn on_finish(mut self, on_finish: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_finish.push(Arc::new(on_finish));
    self
  }

  /// What is told after Escape, "Skip" or a press outside left the tour, the signal `None` already; not
  /// when its last step's button ends it. A second handler is called after
  /// the first.
  pub fn on_dismiss(mut self, on_dismiss: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_dismiss.push(Arc::new(on_dismiss));
    self
  }

  /// Puts its tips in a state the theme's recipe names, or takes them out of it.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|(held, _)| *held != state);
    self.states.push((state, on));
    self
  }

  /// Styles one part of its tips over what the theme gives them.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for its tips.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the root of the tip that shows, whichever step it is, where
  /// the step has none of its own. Its buttons are `<id>-primary`,
  /// `<id>-secondary` and, for "Skip", `<id>-tertiary`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// The key of the root of the tip that shows, where the step has none of its own.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Puts the tour in the app. The element it returns has no size, and goes anywhere.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    ctx.mount::<TourHost>(TourProps { tour: self })
  }

  /// The step it shows, as the app says it now. Reading a bound signal subscribes what renders.
  fn current(&self) -> Option<usize> {
    match &self.current {
      Current::Bound(signal) => signal.get(),
      Current::Held(current) => *current,
    }
  }

  /// What takes the tour to a step, or out of it with `None`: sets a bound signal, then tells `on_change`. Nothing
  /// for the step it is at already.
  fn go(&self) -> StepHandler {
    let (current, handlers) = (self.current.clone(), self.on_change.clone());
    Arc::new(move |to: Option<usize>| {
      match &current {
        Current::Bound(signal) => {
          if signal.get_untracked() == to {
            return;
          }
          signal.set(to);
        }
        Current::Held(held) => {
          if *held == to {
            return;
          }
        }
      }
      for handler in &handlers {
        handler(to);
      }
    })
  }

  /// The tip of the step `index`, open while `open` is `true`: its count,
  /// its buttons back and on, its keys, and what Escape does. `None` for a
  /// step it does not have.
  fn tip(&self, index: usize, open: Signal<bool>) -> Option<TeachingTip> {
    let step = self.steps.get(index)?;
    let count = self.steps.len();
    let last = index + 1 >= count;
    let go = self.go();
    let mut tip = TeachingTip::new(open, step.anchor.clone())
      .title(&step.title)
      .step(index + 1, count)
      .close_button(false)
      .variant(self.variant)
      .placement(step.placement.unwrap_or(self.placement))
      .collision(self.collision)
      .arrow(self.arrow)
      .takes_focus(self.takes_focus)
      .traps_tab(self.traps_tab)
      .dismiss_on_escape(self.dismiss_on_escape)
      .dismiss_on_outside_press(self.dismiss_on_outside_press)
      .rules(&self.rules);
    tip.grouped = true;
    // From the second step the button that goes back leads to a step without one.
    tip.refocus = index == 1;
    for (state, on) in &self.states {
      tip = tip.state(*state, *on);
    }
    if let Some(offset) = self.offset {
      tip = tip.offset(offset);
    }
    if let Some(id) = step.id.as_ref().or(self.id.as_ref()) {
      tip = tip.id(id.clone());
    }
    if let Some(key) = step.key.as_ref().or(self.key.as_ref()) {
      tip = tip.key(key.clone());
    }
    if let Some(description) = &step.description {
      tip = tip.description(description);
    }
    if let Some(media) = &step.media {
      tip = tip.media(media.clone());
    }
    // Leaving before the end: by "Skip", by Escape, by a press outside.
    let leave = {
      let (go, dismissed) = (go.clone(), self.on_dismiss.clone());
      move || {
        go(None);
        for dismiss in &dismissed {
          dismiss();
        }
      }
    };
    if self.skip_button && !last {
      tip = tip.tertiary(&self.skip, leave.clone());
    }
    if index > 0 {
      let to = go.clone();
      tip = tip.secondary(&self.back, move || to(Some(index - 1)));
    }
    let (to, finish) = (go.clone(), self.on_finish.clone());
    tip = tip.primary(if last { &self.done } else { &self.next }, move || {
      if last {
        to(None);
        for finish in &finish {
          finish();
        }
      } else {
        to(Some(index + 1));
      }
    });
    // The arrows step while the focus is in the card: on, back, to the first and to the last.
    tip.keys = Some(Arc::new(move |event: &KeyboardEvent| {
      let to = match event.key.as_str() {
        "ArrowRight" if !last => index + 1,
        "ArrowLeft" if index > 0 => index - 1,
        "Home" if index > 0 => 0,
        "End" if !last => count - 1,
        // An arrow at an end of the tour does nothing: the group of the buttons does not move the focus either.
        "ArrowRight" | "ArrowLeft" | "Home" | "End" => {
          event.prevent_default();
          return;
        }
        _ => return,
      };
      go(Some(to));
      // The focus stays in the card: on the button that goes on, which every step has.
      event.focus_key(PRIMARY_KEY);
    }));
    Some(tip.on_dismiss(leave))
  }

  /// Builds the panel of the step its signal says, or of its first step while
  /// it says none, anchored to nothing, in `theme`. Converting a tour into an
  /// element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let index = self.current().filter(|index| *index < self.steps.len()).unwrap_or(0);
    match self.tip(index, Signal::new(true)) {
      Some(tip) => tip.build(theme),
      None => Element::new(),
    }
  }
}

impl crate::ui::Styled for Tour {
  type Component = TeachingTip;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl From<Tour> for Element {
  fn from(tour: Tour) -> Self {
    tour.build(&current_theme())
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

impl crate::app::component::DevtoolsInspectable for TourProps {}

/// What shows a tour's step.
struct TourHost {
  /// The step it showed last: what fades out as the tour ends.
  last: Mutex<usize>,
  /// Whether its tip is open: while the app says a step.
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
    let current = tour.current();
    if matches!(tour.current, Current::Held(_)) {
      // A step the app holds may stay as it is after Escape closed the tip: this renders again, and shows it.
      let _ = self.open.get();
    }
    let shown = current.filter(|index| *index < tour.steps.len());
    let index = {
      let mut last = self.last.lock();
      if let Some(index) = shown {
        *last = index;
      }
      *last
    };
    // The tip's signal follows the app's; the tip sets it on Escape, which leaves the tour.
    let Some(tip) = tour.tip(index, self.open.clone()) else {
      return Element::new();
    };
    super::tip::put(&self.open, shown.is_some());
    tip.mount(ctx)
  }
}
