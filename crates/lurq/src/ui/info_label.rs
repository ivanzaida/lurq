//! An info label: a label with a mark after it that explains it.
//!
//! What explains it is a [`Tooltip`], which is mounted: [`InfoLabel::mount`]
//! puts the label where it goes with its tooltip on it. Converted into an
//! element on its own, an info label is its still form: the label and the
//! mark, which says what it explains to tooling and shows nothing.

use std::sync::Arc;

use super::{
  icon::Icon,
  icons, names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{fading, one_line},
  theme::{Themed, current_theme},
  tooltip::Tooltip,
};
use crate::{
  app::{
    ctx::{CollisionStrategy, Ctx, Placement},
    theme::{IconSize, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::Row,
  layout::{Alignment, layout_kind::Justify},
  node::{Element, SpacingValue},
};

/// The parts of an info label a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InfoLabelPart {
  /// The label and its mark, side by side.
  Root,
  /// The text.
  Label,
  /// The mark after the text, an "i" in a circle: its side, its color, and the ring it has while the keyboard's
  /// focus is on it.
  Mark,
}

names!(InfoLabelPart {
  Root => "root",
  Label => "label",
  Mark => "mark",
});

/// The states of an info label a recipe styles, besides what the pointer and
/// the keyboard do to its mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InfoLabelState {
  /// It has something to explain: the mark is there.
  Info,
}

names!(InfoLabelState {
  Info => "info",
});

/// What builds an info label's nodes: a theme's in place of the default one.
/// When its tooltip shows stays the label's.
pub type InfoLabelRenderer = Arc<dyn Fn(InfoLabelView) -> Element + Send + Sync>;

/// An info label: what a field or a setting is called, with a mark after it
/// that explains it. "API key", and that it is stored in the keychain.
///
/// ```no_run
/// use lurq::{app::ctx::Ctx, node::Element, ui::InfoLabel};
///
/// fn api_key(ctx: &mut Ctx) -> Element {
///   InfoLabel::new("API key").info("Stored in the OS keychain").mount(ctx)
/// }
/// ```
///
/// The explanation is a [`Tooltip`] above the label, from its start: it
/// shows while the pointer rests on the label or its mark, and at once when
/// the keyboard's focus comes to the mark, which is a stop of Tab, and goes
/// on Escape. A label longer than the room it has ends in an ellipsis, and
/// the mark keeps its place after it. To tooling that reads the tree the
/// mark is an image called "More information", or by
/// [`info_label`](Self::info_label), and described by the explanation.
///
/// An explanation with a link in it, or one that must open on a press, is a
/// [`Toggletip`](super::Toggletip)'s, put beside a text. A
/// [`Field`](super::Field) draws the same mark after its own label
/// ([`Field::info`](super::Field::info)).
///
/// Converted into an element on its own, an info label is its still form:
/// nothing shows on it.
#[derive(Clone)]
pub struct InfoLabel {
  label: Arc<str>,
  info: Option<Arc<str>>,
  info_label: Arc<str>,
  icon: Name,
  placement: Placement,
  offset: Option<SpacingValue>,
  collision: CollisionStrategy,
  arrow: bool,
  force_open: Option<bool>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl InfoLabel {
  /// A label that says `label`, with no mark until it has something to explain ([`info`](Self::info)).
  pub fn new(label: &str) -> Self {
    Self {
      label: Arc::from(label),
      info: None,
      info_label: super::current_locale().fields.more_information.clone(),
      icon: Name::new(icons::INFO),
      placement: Placement::TopStart,
      offset: None,
      collision: CollisionStrategy::FlipThenClamp,
      arrow: false,
      force_open: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// What the mark explains, in a few words: the text of its tooltip. An empty one is none.
  pub fn info(mut self, info: &str) -> Self {
    self.info = (!info.is_empty()).then(|| Arc::from(info));
    self
  }

  /// What the mark is called to tooling that reads the tree, in place of "More information".
  pub fn info_label(mut self, label: &str) -> Self {
    self.info_label = Arc::from(label);
    self
  }

  /// The icon of the mark, by its name in the theme's set, in place of the "i".
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = name.into();
    self
  }

  /// Which side of the label the tooltip is on, and which edge of it it lines up with: above it, from its start
  /// (`Placement::TopStart`), as the design draws it, unless told.
  pub fn placement(mut self, placement: Placement) -> Self {
    self.placement = placement;
    self
  }

  /// How far the tooltip stands from the label, in place of what the theme gives a tooltip (6 in the design's).
  pub fn offset(mut self, offset: impl Into<SpacingValue>) -> Self {
    self.offset = Some(offset.into());
    self
  }

  /// What the tooltip does where there is no room for it on its side: goes to the other side, then is moved
  /// into the window, by default.
  pub fn collision(mut self, collision: CollisionStrategy) -> Self {
    self.collision = collision;
    self
  }

  /// Whether a triangle points from the tooltip to the label.
  pub fn arrow(mut self, arrow: bool) -> Self {
    self.arrow = arrow;
    self
  }

  /// Shows the tooltip whatever the pointer and the focus do (`Some(true)`), or never (`Some(false)`); `None`,
  /// as it starts, leaves it to them. For a gallery, a test.
  pub fn force_open(mut self, open: Option<bool>) -> Self {
    self.force_open = open;
    self
  }

  /// Puts the label in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this label over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this label: what the component it is in says of it in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the mark as it is under the pointer or focused from the keyboard, with nothing pointing at it: for a
  /// gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the label's root. Its mark is `<id>-mark`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The label as it is drawn in `theme`.
  fn view(&self, theme: &Theme) -> InfoLabelView {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(InfoLabelState::Info, self.info.is_some());
    InfoLabelView {
      label: self.label.clone(),
      info: self.info.clone(),
      info_label: self.info_label.clone(),
      icon: self.icon,
      recipe: theme.resolve::<InfoLabel>(&case, Some(&self.rules)),
      forced: self.forced,
      id: self.id.clone(),
      key: self.key.clone(),
    }
  }

  /// Builds what `view` shows: the theme's renderer, or the default one.
  fn render_view(view: InfoLabelView) -> Element {
    match view.theme().renderer::<InfoLabel>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// The tooltip that says `info`, as this label places it.
  fn tooltip(&self, info: &str) -> Tooltip {
    let tooltip = Tooltip::new(info)
      .placement(self.placement)
      .collision(self.collision)
      .arrow(self.arrow)
      .force_open(self.force_open);
    match self.offset {
      Some(offset) => tooltip.offset(offset),
      None => tooltip,
    }
  }

  /// Puts the label where it goes, with its tooltip on it: over the window while the pointer rests on the label
  /// or the keyboard's focus is on its mark. A label with nothing to explain is the label alone.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let label = Self::render_view(self.view(&current_theme()));
    match &self.info {
      Some(info) => self.tooltip(info).mount(ctx, label),
      None => label,
    }
  }

  /// The mark alone, which explains `info`, with its tooltip above it where there is a `ctx` to mount it with:
  /// what a [`Field`](super::Field) draws after its own label.
  pub(crate) fn mark_alone(info: &str, theme: &Theme, ctx: Option<&mut Ctx>) -> Element {
    let label = Self::new("").info(info);
    let mark = label.view(theme).mark().map_or_else(Element::new, Element::from);
    match ctx {
      Some(ctx) => label.tooltip(info).placement(Placement::Top).mount(ctx, mark),
      None => mark,
    }
  }

  /// Builds the label's still form in `theme`: the label and its mark, with nothing that shows on them.
  /// Converting an info label into an element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    Self::render_view(self.view(theme))
  }
}

impl From<InfoLabel> for Element {
  fn from(label: InfoLabel) -> Self {
    label.build(&current_theme())
  }
}

/// An info label as a renderer sees it: what it says and the style of its parts.
pub struct InfoLabelView {
  /// The text of the label.
  pub label: Arc<str>,
  /// What the mark explains; `None` where it has nothing to explain and no mark.
  pub info: Option<Arc<str>>,
  /// What the mark is called to tooling.
  pub info_label: Arc<str>,
  /// The name of the mark's icon.
  pub icon: Name,
  /// The style of each part of this label.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl InfoLabelView {
  /// The style of a part, as this label is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The root in the style of its part, with nothing in it yet.
  pub fn root(&self) -> Row {
    let mut root = Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(InfoLabelPart::Root))
      .overflow_visible();
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// The mark: the icon at the side its part gives, a stop of Tab that takes no press, an image to tooling
  /// called by [`info_label`](Self::info_label) and described by what it explains. `None` where the label has
  /// nothing to explain.
  pub fn mark(&self) -> Option<Row> {
    let info = self.info.as_ref()?;
    let part = self.part(InfoLabelPart::Mark).forced(self.forced);
    let side = part.rest().width.unwrap_or(Length::Icon(IconSize::Sm));
    let mut mark = Row::new()
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .flex_shrink(0.0)
      .tab_index(0)
      .part(&part)
      .describe("a11y_role", "img")
      .describe("a11y_name", self.info_label.clone())
      .describe("a11y_description", info.clone())
      .child(Icon::new(self.icon).size(side).build(self.theme()));
    if let Some(id) = &self.id {
      mark = mark.id(format!("{id}-mark"));
    }
    Some(mark)
  }

  /// How the default renderer builds an info label: the text on one line, which ends in an ellipsis in a room
  /// too narrow for it, and the mark after it.
  pub fn render(self) -> Element {
    let mut root = self
      .root()
      .child(one_line(&self.part(InfoLabelPart::Label), &self.label));
    if let Some(mark) = self.mark() {
      root = root.child(mark);
    }
    root.into()
  }
}

impl crate::ui::Styled for InfoLabel {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for InfoLabel {
  const NAME: &'static str = "info_label";
  type Renderer = InfoLabelRenderer;

  fn parts() -> crate::ui::Declared {
    InfoLabelPart::declared()
  }

  fn states() -> crate::ui::Declared {
    InfoLabelState::declared()
  }

  /// The design's info label (`pencil/lurq-design-system.pen`, Info Label):
  /// a field label's text (13, medium) in the foreground, and 6 after it an
  /// "i" of 14 in the muted foreground, with the design's tooltip above
  /// them. Under the pointer the mark is in the foreground; focused from the
  /// keyboard it has the theme's ring around its circle. It is no wider than
  /// the room it has.
  fn recipe() -> Recipe {
    use InfoLabelPart::{Label, Mark, Root};
    use PaletteColor::{Foreground, MutedForeground};
    Recipe::new()
      .base(Root, |s| s.gap(6.0).max_width(Length::Fill))
      .base(Label, |s| {
        s.typography(TypographyStyle::FieldLabel).foreground(Foreground)
      })
      .base(Mark, |s| {
        fading(s.square(14.0).radius(RadiusSize::Full).foreground(MutedForeground))
      })
      .hovered(Mark, |s| s.foreground(Foreground))
      .focus_visible(Mark, |s| s.focus_ring().foreground(Foreground))
  }
}
