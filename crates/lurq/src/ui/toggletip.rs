//! A toggletip: a small button that opens a few words about what is beside
//! it, with a link or a button in them.
//!
//! It is a mounted component: it keeps whether it is open, an anchor that is
//! the same from one render to the next, and which side the window let its
//! panel take, none of which a builder can keep. [`Toggletip`] says what it
//! shows, and [`Toggletip::mount`] puts it where it goes.

use std::sync::Arc;

use super::{
  Button, ButtonPart, ButtonShape, ButtonSize, ButtonVariant, LinkPart, TooltipSide, icons, names,
  part_style::{Length, PartAnimation, PartStyle},
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{FADE_IN, FADE_OUT},
  theme::{Themed, current_theme},
  tip::{Placed, arrow, arrow_size, frame, side_of},
};
use crate::{
  app::{
    component::Component,
    ctx::{CollisionStrategy, Ctx, Placement},
    events::MouseEvent,
    theme::{IconSize, MotionDuration, MotionEasing, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{Column, Popup, Row},
  core::{ElementRef, Signal},
  layout::{Alignment, text_style::FontWeight},
  node::{Element, EventHandler, IntoMouseEventHandler, NodeUpdate, border::Border, dimension::Dimension},
};

/// The parts of a toggletip a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToggletipPart {
  /// The panel and its arrow, and the room between them and the button.
  Root,
  /// The button that opens and closes it: what it gives the theme's button.
  Trigger,
  /// The icon of the button: what it gives the button's icon.
  TriggerIcon,
  /// The panel: its fill, its text, its link and what it holds, one under another.
  Panel,
  /// Its text.
  Text,
  /// The link under the text: what it gives the theme's link.
  Link,
  /// The text of the link: what it gives the link's label.
  LinkLabel,
  /// What the arrow is drawn in, between the panel and the button.
  Arrow,
}

names!(ToggletipPart {
  Root => "root",
  Trigger => "trigger",
  TriggerIcon => "trigger-icon",
  Panel => "panel",
  Text => "text",
  Link => "link",
  LinkLabel => "link-label",
  Arrow => "arrow",
});

/// The states of a toggletip a recipe styles. The side of the button the
/// panel is on is a state too, by the names of [`TooltipSide`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToggletipState {
  /// It is open.
  Open,
  /// It has an arrow.
  Arrow,
  /// Its button takes nothing.
  Disabled,
}

names!(ToggletipState {
  Open => "open",
  Arrow => "arrow",
  Disabled => "disabled",
});

/// What builds a toggletip's panel: a theme's in place of the default one.
/// Its button and when the panel shows stay the toggletip's.
pub type ToggletipRenderer = Arc<dyn Fn(ToggletipView) -> Element + Send + Sync>;

/// A link of a toggletip: its text, and what a press of it does.
#[derive(Clone)]
struct TipLink {
  label: Arc<str>,
  on_click: Vec<EventHandler<MouseEvent>>,
}

/// A toggletip: a tooltip opened by a press, for what must be reachable by a
/// finger and told by a screen reader. A small button with an "i" beside a
/// label, which opens a few words about it and, where there is more, a link.
///
/// ```no_run
/// use lurq::{app::ctx::Ctx, node::Element, ui::Toggletip};
///
/// fn workspace_help(ctx: &mut Ctx) -> Element {
///   Toggletip::new("What is a workspace?")
///     .content("A workspace holds the projects a team shares.")
///     .link("Learn more", |_| println!("to the docs"))
///     .mount(ctx)
/// }
/// ```
///
/// A press of the button opens the panel and another closes it; a press
/// outside it and Escape close it too. The focus stays on the button as it
/// opens, and Tab goes from the button into the panel's link and on. To
/// tooling the button is called by its label and says whether it is
/// expanded, and the panel is a status, which a screen reader reads as it
/// opens. Where there is no room on its side the panel goes to the other, its
/// arrow turned with it, and it is kept in the window.
///
/// Converted into an element on its own, a toggletip is its panel as it
/// shows, anchored to nothing: for a gallery, a test, or a host of an app's own.
#[derive(Clone)]
pub struct Toggletip {
  label: Arc<str>,
  icon: Name,
  content: Option<Arc<str>>,
  link: Option<TipLink>,
  children: Vec<Element>,
  placement: Placement,
  arrow: bool,
  open: Option<bool>,
  disabled: bool,
  width: Option<Length>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Toggletip {
  /// A toggletip whose button is called `label` to tooling, "What is a
  /// workspace?", with an "i" for its icon, its panel above it, with no arrow.
  pub fn new(label: &str) -> Self {
    Self {
      label: Arc::from(label),
      icon: Name::new(icons::INFO),
      content: None,
      link: None,
      children: Vec::new(),
      placement: Placement::Top,
      arrow: false,
      open: None,
      disabled: false,
      width: None,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// The icon of the button, by its name in the theme's set, in place of the "i".
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = name.into();
    self
  }

  /// What the panel says.
  pub fn content(mut self, content: &str) -> Self {
    self.content = Some(Arc::from(content));
    self
  }

  /// A link under the text, "Learn more", with an arrow after it, and what
  /// its press does.
  pub fn link(mut self, label: &str, on_click: impl IntoMouseEventHandler) -> Self {
    self.link = Some(TipLink {
      label: Arc::from(label),
      on_click: vec![on_click.into_event_handler()],
    });
    self
  }

  /// Adds something the panel holds, under its text and its link: a button.
  pub fn child(mut self, child: impl Into<Element>) -> Self {
    self.children.push(child.into());
    self
  }

  /// Adds several things the panel holds.
  pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.children.extend(children.into_iter().map(Into::into));
    self
  }

  /// Which side of the button the panel is on, and which edge of it it lines
  /// up with: above it and centered by default.
  pub fn placement(mut self, placement: Placement) -> Self {
    self.placement = placement;
    self
  }

  /// Whether a triangle points from the panel to the button.
  pub fn arrow(mut self, arrow: bool) -> Self {
    self.arrow = arrow;
    self
  }

  /// Shows the panel whatever the button says (`Some(true)`), or never
  /// (`Some(false)`); `None`, as it starts, leaves it to the button. For a
  /// gallery, a test.
  pub fn open(mut self, open: Option<bool>) -> Self {
    self.open = open;
    self
  }

  /// Whether the button takes nothing.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// How wide the panel is, in place of what the theme gives it.
  pub fn width(mut self, width: impl Into<Length>) -> Self {
    self.width = Some(width.into());
    self
  }

  /// Puts the toggletip in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this toggletip over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this toggletip.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of its button. The root of its panel, what shows, is `<id>-panel`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Puts the toggletip where it goes: its button, and its panel over the
  /// window while it is open.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    ctx.mount::<ToggletipHost>(ToggletipProps { tip: self })
  }

  /// The toggletip as it is drawn on `side`, open or not, in `theme`.
  fn view(&self, theme: &Theme, side: TooltipSide, open: bool) -> ToggletipView {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(side, true)
    .state(ToggletipState::Open, open)
    .state(ToggletipState::Arrow, self.arrow)
    .state(ToggletipState::Disabled, self.disabled);
    // A width given here is under what `style` and `rules` say of the panel.
    let mut rules = Recipe::new();
    if let Some(width) = self.width {
      rules = rules.base(ToggletipPart::Panel, |s| s.width(width));
    }
    let rules = rules.extend(&self.rules);
    ToggletipView {
      label: self.label.clone(),
      icon: self.icon,
      content: self.content.clone(),
      link: self.link.as_ref().map(|link| link.label.clone()),
      link_handlers: self.link.as_ref().map(|link| link.on_click.clone()).unwrap_or_default(),
      children: self.children.clone(),
      side,
      arrow: self.arrow,
      open,
      disabled: self.disabled,
      aim: 0.0,
      recipe: theme.resolve::<Toggletip>(&case, Some(&rules)),
      id: self.id.clone(),
      key: self.key.clone(),
    }
  }

  /// Builds what `view` shows: the theme's renderer, or the default one.
  fn render_view(view: ToggletipView) -> Element {
    match view.theme().renderer::<Toggletip>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// Builds the toggletip's panel as it shows, anchored to nothing, in
  /// `theme`. Converting a toggletip into an element does this in the theme
  /// of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    Self::render_view(self.view(theme, side_of(self.placement), true))
  }
}

impl From<Toggletip> for Element {
  fn from(tip: Toggletip) -> Self {
    tip.build(&current_theme())
  }
}

/// A toggletip as a renderer sees it: what it says, the side its panel is
/// on, and the style of its parts.
pub struct ToggletipView {
  /// What the button is called to tooling.
  pub label: Arc<str>,
  /// The name of the button's icon.
  pub icon: Name,
  pub content: Option<Arc<str>>,
  /// The text of the link, if it has one.
  pub link: Option<Arc<str>>,
  link_handlers: Vec<EventHandler<MouseEvent>>,
  /// What the panel holds, under its text and its link.
  pub children: Vec<Element>,
  /// The side of the button the panel is on: the side its placement gives,
  /// or the other where there was no room.
  pub side: TooltipSide,
  pub arrow: bool,
  pub open: bool,
  pub disabled: bool,
  /// How far along the panel's edge from its middle the arrow is, toward the
  /// middle of the button.
  pub aim: f32,
  /// The style of each part of this toggletip.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ToggletipView {
  /// The style of a part, as this toggletip is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The button: a ghost one that is its icon alone, styled over the theme's
  /// button by [`ToggletipPart::Trigger`]. It says whether it is expanded.
  pub fn trigger(&self) -> Button {
    let button = Button::new(&self.label);
    let button = match &self.id {
      Some(id) => button.id(id.clone()),
      None => button,
    };
    button
      .variant(ButtonVariant::Ghost)
      .size(ButtonSize::Xs)
      .shape(ButtonShape::Pill)
      .icon(self.icon)
      .icon_only(true)
      .disabled(self.disabled)
      .rules(
        &self
          .recipe
          .rules_for(ToggletipPart::Trigger, ButtonPart::Root)
          .extend(&self.recipe.rules_for(ToggletipPart::TriggerIcon, ButtonPart::Icon)),
      )
  }

  /// The link under the text, with an arrow after it. `None` without one.
  pub fn link(&self) -> Option<super::Link> {
    let label = self.link.as_ref()?;
    let mut link = super::Link::new(label).icon(icons::ARROW_UP_RIGHT).rules(
      &self
        .recipe
        .rules_for(ToggletipPart::Link, LinkPart::Root)
        .extend(&self.recipe.rules_for(ToggletipPart::LinkLabel, LinkPart::Label)),
    );
    for handler in &self.link_handlers {
      link = link.on_click(handler.clone());
    }
    Some(link)
  }

  /// The panel: its text, its link and what it holds, one under another. A
  /// status to what reads the tree, which a screen reader reads as it opens.
  pub fn panel(&mut self) -> Column {
    let mut panel = Column::new()
      .align_items(Alignment::Start)
      .part(&self.part(ToggletipPart::Panel))
      .describe("a11y_role", "status");
    if let Some(content) = &self.content {
      let part = self.part(ToggletipPart::Text);
      let fill = Dimension::Pct(100.0);
      panel = panel.child(
        Column::new()
          .width(fill)
          .min_width(Dimension::Px(0.0))
          .part(&part)
          .child(part.text(content).width(fill)),
      );
    }
    if let Some(link) = self.link() {
      panel = panel.child(link);
    }
    for child in std::mem::take(&mut self.children) {
      panel = panel.child(child);
    }
    panel
  }

  /// How long the arrow is along the panel's edge, and how far out from it
  /// it goes. `None` without an arrow.
  pub fn arrow_size(&self) -> Option<(f32, f32)> {
    if !self.arrow {
      return None;
    }
    arrow_size(&self.part(ToggletipPart::Arrow), self.side)
  }

  /// The arrow, in the panel's fill, on the side toward the button. `None` without one.
  pub fn arrow(&self) -> Option<Element> {
    if !self.arrow {
      return None;
    }
    arrow(
      &self.part(ToggletipPart::Arrow),
      &self.part(ToggletipPart::Panel),
      self.side,
      self.aim,
    )
  }

  /// How the default renderer builds what a toggletip shows: the panel, and
  /// the arrow on its side toward the button.
  pub fn render(mut self) -> Element {
    let away = self.arrow_size().map_or(0.0, |(_, away)| away);
    let arrow = self.arrow();
    let panel = self.panel();
    let mut root: Element = frame(&self.part(ToggletipPart::Root), panel.into(), arrow, away, self.side).into();
    if let Some(id) = &self.id {
      root = root.id(format!("{id}-panel"));
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }
}

impl Themed for Toggletip {
  const NAME: &'static str = "toggletip";
  type Renderer = ToggletipRenderer;

  /// The design's toggletip (`pencil/lurq-design-system.pen`, Toggletip): a
  /// panel 230 wide in the primary color with a radius of 6, 12 of padding
  /// and 8 between what it holds. Its text is in 13 of the primary's
  /// foreground in lines of 1.4, and its link in 13 at 600 with an arrow of
  /// 14 4 after it, underlined in the same color under the pointer. The
  /// button is a ghost one of 20 that is a circle, its icon of 14 in the
  /// muted foreground. The panel stands 6 from the button and its arrow is
  /// the tooltip's, 12 along and 6 out; it fades in and out as the popover.
  fn recipe() -> Recipe {
    use PaletteColor::{Foreground, MutedForeground, Primary, PrimaryForeground};
    use ToggletipPart::{Arrow, Link, LinkLabel, Panel, Root, Text, Trigger, TriggerIcon};
    use TooltipSide::{Bottom, Left, Right, Top};
    Recipe::new()
      .base(Root, |s| {
        s.enter(PartAnimation::new(FADE_IN, MotionDuration::Base, MotionEasing::Enter))
          .exit(PartAnimation::new(FADE_OUT, MotionDuration::Fast, MotionEasing::Exit))
      })
      .base(Trigger, |s| {
        s.square(20.0).padding_x(0.0).foreground(MutedForeground)
      })
      .hovered(Trigger, |s| s.foreground(Foreground))
      .state(ToggletipState::Open, Trigger, |s| s.foreground(Foreground))
      .base(TriggerIcon, |s| s.square(IconSize::Sm))
      .base(Panel, |s| {
        s.width(230.0)
          .padding_x(12.0)
          .padding_y(12.0)
          .gap(8.0)
          .radius(RadiusSize::Md)
          .background(Primary)
          .foreground(PrimaryForeground)
      })
      .base(Text, |s| {
        s.typography(TypographyStyle::Description)
          .font_size(13.0)
          .line_height(1.4)
      })
      .base(Link, |s| s.foreground(PrimaryForeground))
      .hovered(Link, |s| s.underline(Border::inside(1.0, PrimaryForeground)))
      .base(LinkLabel, |s| s.font_size(13.0).weight(FontWeight::SemiBold))
      .base(Arrow, |s| s.width(12.0).height(6.0))
      .state(Left, Arrow, |s| s.width(6.0).height(12.0))
      .state(Right, Arrow, |s| s.width(6.0).height(12.0))
      // The room between the panel and the button, toward it.
      .state(Top, Root, |s| s.padding_bottom(6.0))
      .state(Bottom, Root, |s| s.padding_top(6.0))
      .state(Left, Root, |s| s.padding_end(6.0))
      .state(Right, Root, |s| s.padding_start(6.0))
  }
}

/// A toggletip.
#[derive(Clone)]
struct ToggletipProps {
  tip: Toggletip,
}

/// The toggletip is built anew by every render of what holds it, so props
/// are never the same as before.
impl PartialEq for ToggletipProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for ToggletipProps {}

/// What opens a toggletip's panel by its button, and shows it.
struct ToggletipHost {
  /// Whether it is open: a press of the button turns it, a press outside it and Escape turn it off.
  open: Signal<bool>,
  /// The button.
  anchor: ElementRef,
  /// What shows.
  shown: ElementRef,
  placed: Placed,
}

impl Component for ToggletipHost {
  type Props = ToggletipProps;

  fn create(ctx: &mut Ctx) -> Self {
    let shown = ElementRef::new();
    let placed = Placed::new(ctx, &shown);
    Self {
      open: ctx.signal(false),
      anchor: ElementRef::new(),
      shown,
      placed,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let ToggletipProps { tip } = ctx.props::<ToggletipProps>().clone();
    // Reading it subscribes: this renders again when the button, a press outside or Escape turns it.
    let open = !tip.disabled && tip.open.unwrap_or_else(|| self.open.get());
    let side = self.placed.side(tip.placement);
    let mut view = tip.view(&current_theme(), side, open);
    view.aim = self.placed.aim();
    self.placed.note(
      tip.placement,
      &self.anchor,
      view.arrow_size().map_or(0.0, |(along, _)| along),
    );

    let mut trigger = view.trigger();
    if tip.open.is_none() {
      let turn = self.open.clone();
      trigger = trigger.on_click(move |_| turn.update(|open| *open = !*open));
    }
    // The button's own node is the anchor, so the engine knows it for the popup's trigger: a press of it is not
    // a press outside the panel.
    let (anchor, expanded) = (self.anchor.clone(), if open { "true" } else { "false" });
    let trigger = Element::from(trigger).map_node(move |mut node| {
      NodeUpdate::ref_element(&mut node, anchor);
      NodeUpdate::describe(&mut node, Arc::from("a11y_expanded"), Arc::from(expanded));
      node
    });

    let shown = Toggletip::render_view(view).map_node(|node| node.ref_element(self.shown.clone()));
    let popup = Popup::new(self.anchor.clone(), shown)
      .placement(tip.placement)
      .collision(CollisionStrategy::FlipThenClamp)
      // The focus stays on the button as it opens, and Tab goes from the button into the panel's link.
      .tab_after_anchor(true);
    let popup = match tip.open {
      Some(open) => popup
        .open_when(open && !tip.disabled)
        .dismiss_on_outside_click(false)
        .dismiss_on_escape(false),
      None if tip.disabled => popup.open_when(false),
      None => popup.open(self.open.clone()),
    };
    Row::new().align_items(Alignment::Center).child(trigger).child(popup)
  }
}
