//! A collapsible: a header that is pressed, and what it shows while it is open.

use std::sync::Arc;

use super::{
  icon::Icon,
  icons, names,
  part_style::{Length, PartAnimation, PartTransition},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{FADE_IN, FADE_OUT, fading, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  animation::Transition,
  app::theme::{
    IconSize, MotionDuration, MotionEasing, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle,
  },
  components::{self, Column, Row},
  layout::{Alignment, layout_kind::Justify},
  node::{CursorIcon, Element, color::Color},
};

/// The parts of a collapsible a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CollapsiblePart {
  /// The header and the panel, one over the other.
  Root,
  /// What is pressed: the header, with the label and the indicator.
  Trigger,
  /// What the header says.
  Label,
  /// The icon at the end of the header that says it opens.
  Indicator,
  /// What opens and closes: as high as what it holds while the collapsible is open, of no height while it is
  /// closed, and clipped on the way between them.
  Panel,
  /// What the panel holds, in its padding: the app's content.
  Content,
}

names!(CollapsiblePart {
  Root => "root",
  Trigger => "trigger",
  Label => "label",
  Indicator => "indicator",
  Panel => "panel",
  Content => "content",
});

/// The states of a collapsible a recipe styles, besides what the pointer and
/// the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CollapsibleState {
  /// It is open.
  Open,
  /// It takes no input.
  Disabled,
}

names!(CollapsibleState {
  Open => "open",
  Disabled => "disabled",
});

/// What builds a collapsible's nodes: a theme's in place of the default one.
pub type CollapsibleRenderer = Arc<dyn Fn(CollapsibleView) -> Element + Send + Sync>;

/// What a collapsible shows while it is open: the app's element, or a text in the content's type.
pub(crate) enum Body {
  Element(Element),
  Text(Arc<str>),
}

/// A header that shows or hides what is under it: the crates of a
/// workspace, the details of an order, the options a person rarely needs.
///
/// ```
/// use lurq::{components::Column, ui::Collapsible};
///
/// let crates = Collapsible::new("3 crates in this workspace", Column::new())
///   .open(true)
///   .on_change(|open| println!("open: {open}"));
/// // A text in the content's type.
/// let note = Collapsible::text("Why is this here?", "Because the build needs it.");
/// ```
///
/// A collapsible does not keep whether it is open: the app says so with
/// [`Collapsible::open`], and is told of a press of the header with
/// [`Collapsible::on_change`]. What it holds opens and closes with its height
/// over the theme's duration, at once where motion is reduced; while it is
/// closed it is not there, and the controls in it are not stops of Tab.
pub struct Collapsible {
  label: Arc<str>,
  body: Body,
  open: bool,
  disabled: bool,
  icons: (Name, Name),
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Collapsible {
  /// A collapsible whose header says `label`, and that shows `content` while it is open.
  pub fn new(label: &str, content: impl Into<Element>) -> Self {
    Self::of(label, Body::Element(content.into()))
  }

  /// A collapsible whose header says `label`, and that shows the text `body` while it is open.
  pub fn text(label: &str, body: &str) -> Self {
    Self::of(label, Body::Text(Arc::from(body)))
  }

  fn of(label: &str, body: Body) -> Self {
    Self {
      label: Arc::from(label),
      body,
      open: false,
      disabled: false,
      icons: (icons::CHEVRONS_VERTICAL.into(), icons::CHEVRONS_VERTICAL.into()),
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_change: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// Whether what it holds is shown.
  pub fn open(mut self, open: bool) -> Self {
    self.open = open;
    self
  }

  /// Whether the header takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// The icons of the indicator while the collapsible is closed and while it is open, by their names in the
  /// theme's set. Both are `chevrons-vertical` unless told otherwise.
  pub fn icons(mut self, closed: impl Into<Name>, open: impl Into<Name>) -> Self {
    self.icons = (closed.into(), open.into());
    self
  }

  /// Puts the collapsible in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Called on a press of the header, with what the collapsible is to be: open if it was closed.
  pub fn on_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Styles one part of this collapsible over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this collapsible: what the
  /// component it is in says of it in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the header as it is under the pointer, held down or focused from
  /// the keyboard, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the header: what is pressed.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the collapsible in `theme`. Converting a collapsible into an
  /// element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(CollapsibleState::Open, self.open)
    .state(CollapsibleState::Disabled, self.disabled);
    let (closed, opened) = self.icons;
    let view = CollapsibleView {
      label: self.label,
      open: self.open,
      disabled: self.disabled,
      icon: if self.open { opened } else { closed },
      recipe: theme.resolve::<Collapsible>(&case, Some(&self.rules)),
      root: CollapsiblePart::Root.into(),
      body: Some(self.body),
      forced: self.forced,
      on_change: self.on_change,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Collapsible>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Collapsible> for Element {
  fn from(collapsible: Collapsible) -> Self {
    collapsible.build(&current_theme())
  }
}

/// A collapsible as a renderer sees it: what it was given, the style of its
/// parts, and its behavior ready to be dressed. An item of an
/// [`Accordion`](super::Accordion) is one too, in the accordion's recipe.
pub struct CollapsibleView {
  pub label: Arc<str>,
  pub open: bool,
  pub disabled: bool,
  /// The icon of the indicator as the collapsible is now: open or closed.
  pub icon: Name,
  /// The style of each part of this collapsible.
  pub recipe: ResolvedRecipe,
  /// The part that holds the header and the panel: the root of a collapsible, an item of an accordion.
  pub(crate) root: Name,
  body: Option<Body>,
  forced: Option<Interaction>,
  on_change: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl CollapsibleView {
  /// A view of an item of a group, in the group's `recipe`, whose `root` is the group's part for an item.
  #[allow(clippy::too_many_arguments)]
  pub(crate) fn item(
    label: Arc<str>,
    body: Body,
    open: bool,
    disabled: bool,
    icon: Name,
    recipe: ResolvedRecipe,
    root: Name,
    press: Arc<dyn Fn(bool) + Send + Sync>,
  ) -> Self {
    Self {
      label,
      open,
      disabled,
      icon,
      recipe,
      root,
      body: Some(body),
      forced: None,
      on_change: vec![press],
      id: None,
      key: None,
    }
  }

  /// The style of a part, as this collapsible is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// What a press does: every handler is told what the collapsible is to be.
  fn press(&self) -> Option<impl Fn() + Send + Sync + Clone + 'static> {
    let (handlers, next) = (self.on_change.clone(), !self.open);
    (!handlers.is_empty() && !self.disabled).then(|| {
      move || {
        for handler in &handlers {
          handler(next);
        }
      }
    })
  }

  /// The collapsible's behavior with the style of its header: the element
  /// that is pressed, focused and disabled, with nothing in it yet. It is a
  /// button to what reads the tree, called by its label, and says whether
  /// what it opens is open.
  pub fn trigger(&self) -> components::Button {
    let mut trigger = pressable(&self.part(CollapsiblePart::Trigger), self.disabled)
      .justify(Justify::Start)
      .align_items(Alignment::Center)
      .describe("a11y_role", "button")
      .describe("a11y_name", self.label.clone())
      .describe("a11y_expanded", if self.open { "true" } else { "false" });
    if let Some(press) = self.press() {
      trigger = trigger.on_click(move |_| press());
    }
    if let Some(id) = &self.id {
      trigger = trigger.id(id.clone());
    }
    if let Some(key) = &self.key {
      trigger = trigger.key(key.clone());
    }
    trigger
  }

  /// The label in the style of its part, taking the room of the header the indicator does not.
  pub fn label(&self) -> Row {
    let part = self.part(CollapsiblePart::Label);
    Row::new().flex(1.0).part(&part).child(part.text(&self.label))
  }

  /// The indicator: the icon of the collapsible as it is now, an icon of the theme's middle size in the middle
  /// of its part.
  pub fn indicator(&self) -> Row {
    let part = self.part(CollapsiblePart::Indicator);
    let icon = Icon::new(self.icon)
      .size(Length::Icon(IconSize::Md))
      .build(self.theme());
    Row::new()
      .justify(Justify::Center)
      .align_items(Alignment::Center)
      .part(&part)
      .child(icon)
  }

  /// What opens and closes, with what it holds while the collapsible is open. Its height is the recipe's: of
  /// none while it is closed and its content's while it is open, with a transition between the two. What it
  /// held stays while the panel closes over it, only painted, and is gone with the content's exit animation.
  pub fn panel(&mut self) -> Column {
    let mut panel = Column::new()
      .align_items(Alignment::Stretch)
      .clip()
      .presence()
      .part(&self.part(CollapsiblePart::Panel));
    if self.open
      && let Some(body) = self.body.take()
    {
      let part = self.part(CollapsiblePart::Content);
      let content: Element = match body {
        Body::Element(element) => element,
        Body::Text(text) => part.text(&text).into(),
      };
      // Keyed, so that what the panel held is told apart from what it holds next, and kept as it leaves.
      panel = panel.child(
        Column::new()
          .key("content")
          .align_items(Alignment::Stretch)
          .part(&part)
          .child(content),
      );
    }
    panel
  }

  /// How the default renderer builds a collapsible: the header with its label and indicator, and the panel
  /// under it.
  pub fn render(mut self) -> Element {
    let trigger = self.trigger().child(self.label()).child(self.indicator());
    let panel = self.panel();
    Column::new()
      .align_items(Alignment::Stretch)
      // The ring of the focus is drawn around the header, outside it.
      .overflow_visible()
      .part(&self.part(self.root))
      .child(trigger)
      .child(panel)
      .into()
  }
}

/// What opens and closes in the design's collapsible and accordion: a panel
/// of no height while closed and of its content's while open, over the
/// theme's base duration; the content fades in as it opens and out as it
/// closes.
pub(crate) fn opening(recipe: Recipe, open: Name, panel: Name, content: Name) -> Recipe {
  recipe
    .base(panel, |s| {
      s.height(0.0)
        .transition(PartTransition::new(Transition::height(), MotionDuration::Base))
    })
    .state(open, panel, |s| s.height(Length::Auto))
    .base(content, |s| {
      s.enter(PartAnimation::new(
        FADE_IN,
        MotionDuration::Base,
        MotionEasing::Standard,
      ))
      .exit(PartAnimation::new(
        FADE_OUT,
        MotionDuration::Base,
        MotionEasing::Standard,
      ))
    })
}

impl Themed for Collapsible {
  const NAME: &'static str = "collapsible";
  type Renderer = CollapsibleRenderer;

  /// The design's collapsible (`pencil/lurq-design-system.pen`, Collapsible):
  /// a header with its label in the label's type and, at its end, the
  /// `chevrons-vertical` icon of 16 in a ghost button of 32, which the
  /// highlight fills under the pointer; what it opens is 8 under it.
  fn recipe() -> Recipe {
    use CollapsiblePart::{Content, Indicator, Label, Panel, Root, Trigger};
    use PaletteColor::*;
    let recipe = Recipe::new()
      .base(Root, |s| s.foreground(Foreground))
      .base(Trigger, |s| {
        fading(s.gap(8.0).radius(RadiusSize::Md).cursor(CursorIcon::Pointer))
      })
      .base(Label, |s| s.typography(TypographyStyle::Label))
      .base(Indicator, |s| {
        fading(s.square(32.0).radius(RadiusSize::Md))
      })
      .base(Content, |s| s.padding_top(8.0).typography(TypographyStyle::Body))
      .hovered(Indicator, |s| s.background(Highlight).foreground(HighlightForeground))
      .focus_visible(Trigger, |s| s.focus_ring())
      // The header is dimmed, not what it opens: that is the app's, with controls of its own.
      .state(CollapsibleState::Disabled, Trigger, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
      // A disabled one does not light its button under the pointer.
      .when(Selector::new().state(CollapsibleState::Disabled).hovered(), Indicator, |s| {
        s.background(Color::new(0, 0, 0, 0)).foreground(Foreground)
      });
    opening(recipe, CollapsibleState::Open.into(), Panel.into(), Content.into())
  }
}
