//! A timeline: what happened, in its order, each thing a mark on a line with its title, its description and its
//! time.

use std::sync::Arc;

use super::{
  alert::wrapping,
  names,
  part_style::{FontFamily, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{fading, glyph, pressable},
  spinner::{Spinner, SpinnerPart},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{BorderSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  components::{Column, Row},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, border::Border, dimension::Dimension, outline::Outline},
};

/// The tone of an item of a timeline: the color of its mark.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TimelineVariant {
  /// What happened, with no weight of its own: the board's dot in the primary color.
  #[default]
  Neutral,
  /// Something to know: "Review requested".
  Info,
  /// Something went well: "Deployed".
  Success,
  /// Something to look at: "Build slow".
  Warning,
  /// Something went wrong: "Build failed".
  Destructive,
}

names!(TimelineVariant {
  Neutral => "neutral",
  Info => "info",
  Success => "success",
  Warning => "warning",
  Destructive => "destructive",
});

impl TimelineVariant {
  /// The variant called `name`, where it is one of these: a variant a theme adds is none of them.
  pub fn named(name: Name) -> Option<Self> {
    Self::all().iter().copied().find(|variant| Name::from(*variant) == name)
  }

  /// The palette's color of the tone: the primary color of the neutral one, as the board draws it.
  fn color(self) -> PaletteColor {
    match self {
      Self::Neutral => PaletteColor::Primary,
      Self::Info => PaletteColor::Info,
      Self::Success => PaletteColor::Success,
      Self::Warning => PaletteColor::Warning,
      Self::Destructive => PaletteColor::Destructive,
    }
  }
}

/// The parts of a timeline a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimelinePart {
  /// The column of the items.
  Root,
  /// An item: its rail beside its content. What is pressed where it has `on_select`.
  Item,
  /// The column at the start of an item: its mark over the line down to the next item.
  Rail,
  /// The dot of an item that has no icon and no media.
  Dot,
  /// The well an item's icon stands in, or what holds its media, or its spinner while it is pending.
  Mark,
  /// The icon in the mark.
  Icon,
  /// The line from an item's mark down to the next item's.
  Separator,
  /// The column beside the rail: the title, the description, the time and the body.
  Content,
  /// What happened.
  Title,
  /// The action at the end of the item, beside its texts.
  Action,
  /// The text under the title.
  Description,
  /// When it happened, as the app writes it.
  Time,
  /// What the app puts under the time: a card, a comment, a list of files.
  Body,
}

names!(TimelinePart {
  Root => "root",
  Item => "item",
  Rail => "rail",
  Dot => "dot",
  Mark => "mark",
  Icon => "icon",
  Separator => "separator",
  Content => "content",
  Title => "title",
  Action => "action",
  Description => "description",
  Time => "time",
  Body => "body",
});

/// The states of a timeline and of its items a recipe styles, besides what the pointer and the keyboard do to an
/// item that is pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimelineState {
  /// Of the timeline: an item of it has an icon or media, and every rail is as wide as a mark.
  Mark,
  /// Of an item: the one that is now.
  Current,
  /// Of an item: it is to come, its dot hollow.
  Upcoming,
  /// Of an item: it is running, a spinner for its mark.
  Pending,
  /// Of an item: the last, with no line under its mark and no room under its content.
  Last,
  /// Of an item: it has an icon in a well for its mark.
  Icon,
  /// Of an item: it has the app's media (an avatar) for its mark.
  Media,
  /// Of an item: it is pressed, a stop of Tab.
  Pressable,
  /// Of an item: it is pressed and takes nothing now.
  Disabled,
}

names!(TimelineState {
  Mark => "mark",
  Current => "current",
  Upcoming => "upcoming",
  Pending => "pending",
  Last => "last",
  Icon => "icon",
  Media => "media",
  Pressable => "pressable",
  Disabled => "disabled",
});

/// What builds a timeline's nodes: a theme's in place of the default one.
pub type TimelineRenderer = Arc<dyn Fn(TimelineView) -> Element + Send + Sync>;

type Select = Arc<dyn Fn() + Send + Sync>;

/// What stands for an item's mark.
enum Mark {
  Icon(Name),
  Media(Element),
}

/// An item of a [`Timeline`]: what happened, with its description, its time, its mark and what the app puts under
/// it.
pub struct TimelineItem {
  title: Arc<str>,
  description: Option<Arc<str>>,
  time: Option<Arc<str>>,
  mark: Option<Mark>,
  variant: Name,
  current: bool,
  upcoming: bool,
  pending: bool,
  body: Vec<Element>,
  action: Option<Element>,
  on_select: Vec<Select>,
  disabled: bool,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl TimelineItem {
  /// An item that says what happened: "Released 0.60".
  pub fn new(title: &str) -> Self {
    Self {
      title: Arc::from(title),
      description: None,
      time: None,
      mark: None,
      variant: TimelineVariant::default().into(),
      current: false,
      upcoming: false,
      pending: false,
      body: Vec::new(),
      action: None,
      on_select: Vec::new(),
      disabled: false,
      id: None,
      key: None,
    }
  }

  /// The text under the title. It wraps.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// When it happened, as the app writes it: "Nov 10", "2 hours ago", "14:02". It is under the description.
  pub fn time(mut self, time: &str) -> Self {
    self.time = Some(Arc::from(time));
    self
  }

  /// An icon in a well for its mark, in place of the dot.
  pub fn icon(mut self, icon: impl Into<Name>) -> Self {
    self.mark = Some(Mark::Icon(icon.into()));
    self
  }

  /// The app's mark in place of the dot: an [`Avatar`](super::Avatar) of who did it.
  pub fn media(mut self, media: impl Into<Element>) -> Self {
    self.mark = Some(Mark::Media(media.into()));
    self
  }

  /// A [`TimelineVariant`], or a variant a theme adds: the color of its mark. One that failed is
  /// `Destructive`.
  pub fn variant(mut self, variant: impl Into<Name>) -> Self {
    self.variant = variant.into();
    self
  }

  /// Whether it is the one that is now: its dot is ringed, and it says `a11y_current` to tooling.
  pub fn current(mut self, current: bool) -> Self {
    self.current = current;
    self
  }

  /// Whether it is to come: its dot is hollow.
  pub fn upcoming(mut self, upcoming: bool) -> Self {
    self.upcoming = upcoming;
    self
  }

  /// Whether it is running: a spinner stands for its mark, and it says `a11y_busy` to tooling.
  pub fn pending(mut self, pending: bool) -> Self {
    self.pending = pending;
    self
  }

  /// Content under the time: a card, a comment, a list of files. A second is put after the first.
  pub fn child(mut self, child: impl Into<Element>) -> Self {
    self.body.push(child.into());
    self
  }

  /// Content under the time, after what it has.
  pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.body.extend(children.into_iter().map(Into::into));
    self
  }

  /// The one action, at the end of the item beside its title: a [`Button`](super::Button) or a [`Link`](super::Link). A
  /// second takes the place of the first.
  pub fn action(mut self, action: impl Into<Element>) -> Self {
    self.action = Some(action.into());
    self
  }

  /// Called when the item is pressed, or Enter or Space is pressed while it has the focus: an item with a handler
  /// is pressed and is a stop of Tab. A second handler is called after the first.
  pub fn on_select(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_select.push(Arc::new(handler));
    self
  }

  /// Whether an item that is pressed takes nothing now: it is faded and passed by Tab.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// The id of the item.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }
}

/// A timeline: what happened, in its order. The releases of a project, the history of a deploy, the tracking of an
/// order, the activity of a document.
///
/// ```
/// use lurq::ui::{Timeline, TimelineItem, TimelineVariant};
///
/// let history = Timeline::new([
///   TimelineItem::new("Released 0.60").description("Blur and frosted glass land in the renderer.").time("Nov 10"),
///   TimelineItem::new("Merged node-blur").description("Blurs follow transforms and rounded clips.").time("Nov 8"),
///   TimelineItem::new("Opened pull request").description("Blur a node and its subtree as one layer.").time("Nov 3"),
/// ])
/// .label("History");
/// // A deploy: done, failed, running.
/// let deploy = Timeline::new([
///   TimelineItem::new("Built").icon("check").variant(TimelineVariant::Success),
///   TimelineItem::new("Tests failed").icon("close").variant(TimelineVariant::Destructive),
///   TimelineItem::new("Retrying").pending(true).current(true),
///   TimelineItem::new("Deploy").upcoming(true),
/// ]);
/// ```
///
/// Each item is a mark on a line that runs down to the next item's, beside its title, its description and its
/// time, which wrap: the line is as long as what is beside it. The mark is a dot in the color of the item's
/// [`variant`](TimelineItem::variant), ringed where the item is [`current`](TimelineItem::current) and hollow where
/// it is [`upcoming`](TimelineItem::upcoming); an [`icon`](TimelineItem::icon) in a well, the app's
/// [`media`](TimelineItem::media), or a spinner while it is [`pending`](TimelineItem::pending). The last item has
/// no line under its mark.
///
/// It is read and not operated: an item is a stop of Tab only where it has [`on_select`](TimelineItem::on_select),
/// and is then pressed as a button is. To tooling it is a `list` of `listitem`s, the current one saying
/// `a11y_current`.
///
/// A timeline builds every item it is given: a long history is the app's to page, with a button after the last
/// item that shows more.
pub struct Timeline {
  items: Vec<TimelineItem>,
  label: Option<Arc<str>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Timeline {
  /// A timeline of `items`, the first at the top.
  pub fn new(items: impl IntoIterator<Item = TimelineItem>) -> Self {
    Self {
      items: items.into_iter().collect(),
      label: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// Adds an item after those it has.
  pub fn item(mut self, item: TimelineItem) -> Self {
    self.items.push(item);
    self
  }

  /// Adds items after those it has.
  pub fn items(mut self, items: impl IntoIterator<Item = TimelineItem>) -> Self {
    self.items.extend(items);
    self
  }

  /// What the timeline is called, to what reads the tree: "History".
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Puts the timeline in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this timeline over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this timeline.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the first item that is pressed as it is under the pointer, held down or focused from the keyboard, with
  /// nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the column of the items.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the timeline in `theme`. Converting a timeline into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let view = self.view(theme);
    match theme.renderer::<Timeline>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  fn view(self, theme: &Theme) -> TimelineView {
    let marked = self.items.iter().any(|item| item.mark.is_some());
    let base = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(TimelineState::Mark, marked);
    let recipe = theme.resolve::<Timeline>(&base, Some(&self.rules));
    let count = self.items.len();
    let forced_at = self
      .items
      .iter()
      .position(|item| !item.on_select.is_empty() && !item.disabled);
    let items = self
      .items
      .into_iter()
      .enumerate()
      .map(|(index, item)| {
        let pressable = !item.on_select.is_empty();
        let case = Case {
          variant: Some(item.variant),
          ..base.clone()
        }
        .state(TimelineState::Current, item.current)
        .state(TimelineState::Upcoming, item.upcoming)
        .state(TimelineState::Pending, item.pending)
        .state(TimelineState::Last, index + 1 == count)
        .state(
          TimelineState::Icon,
          matches!(item.mark, Some(Mark::Icon(_))) && !item.pending,
        )
        .state(
          TimelineState::Media,
          matches!(item.mark, Some(Mark::Media(_))) && !item.pending,
        )
        .state(TimelineState::Pressable, pressable)
        .state(TimelineState::Disabled, item.disabled);
        TimelineItemView {
          title: item.title,
          description: item.description,
          time: item.time,
          mark: item.mark,
          current: item.current,
          pending: item.pending,
          last: index + 1 == count,
          body: item.body,
          action: item.action,
          on_select: item.on_select,
          disabled: item.disabled,
          forced: if forced_at == Some(index) { self.forced } else { None },
          recipe: theme.resolve::<Timeline>(&case, Some(&self.rules)),
          id: item.id,
          key: item.key,
        }
      })
      .collect();
    TimelineView {
      items,
      label: self.label,
      recipe,
      id: self.id,
      key: self.key,
    }
  }
}

impl From<Timeline> for Element {
  fn from(timeline: Timeline) -> Self {
    timeline.build(&current_theme())
  }
}

/// A timeline as a renderer sees it: its items, and the style of its column.
pub struct TimelineView {
  /// The items, the first at the top.
  pub items: Vec<TimelineItemView>,
  /// What the timeline is called, to tooling.
  pub label: Option<Arc<str>>,
  recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl TimelineView {
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The column of the items in the style of its part, a `list` to tooling, holding `items`.
  pub fn root(&self, items: Vec<Element>) -> Column {
    let mut root = Column::new()
      .align_items(Alignment::Stretch)
      .overflow_visible()
      .part(&self.part(TimelinePart::Root))
      .describe("a11y_role", "list");
    if let Some(label) = &self.label {
      root = root.describe("a11y_name", label.clone());
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.with_children(items)
  }

  /// How the default renderer builds a timeline: its items, one under another.
  pub fn render(mut self) -> Element {
    let items = std::mem::take(&mut self.items)
      .into_iter()
      .map(TimelineItemView::render)
      .collect();
    self.root(items).into()
  }
}

/// An item of a timeline as a renderer sees it.
pub struct TimelineItemView {
  /// What happened.
  pub title: Arc<str>,
  /// The text under the title.
  pub description: Option<Arc<str>>,
  /// When it happened, as the app writes it.
  pub time: Option<Arc<str>>,
  mark: Option<Mark>,
  /// Whether it is the one that is now.
  pub current: bool,
  /// Whether it is running.
  pub pending: bool,
  /// Whether it is the last item.
  pub last: bool,
  body: Vec<Element>,
  action: Option<Element>,
  on_select: Vec<Select>,
  /// Whether an item that is pressed takes nothing now.
  pub disabled: bool,
  forced: Option<Interaction>,
  recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl TimelineItemView {
  /// The style of a part, as this item is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// Whether the item is pressed: it has a handler.
  pub fn pressable(&self) -> bool {
    !self.on_select.is_empty()
  }

  /// The item's mark: the spinner while it is pending, its icon in a well, its media, or the dot.
  pub fn mark(&mut self) -> Element {
    let well = |part: &ResolvedPart| {
      Row::new()
        .justify(Justify::Center)
        .align_items(Alignment::Center)
        .overflow_visible()
        .part(part)
    };
    let part = self.part(TimelinePart::Mark);
    if self.pending {
      let side = part.rest().width.unwrap_or(12.0.into());
      return well(&part)
        .child(
          Spinner::new()
            .decorative(true)
            .style(SpinnerPart::Indicator, move |s| s.square(side))
            .build(self.theme()),
        )
        .into();
    }
    match self.mark.take() {
      Some(Mark::Icon(name)) => well(&part)
        .child(glyph(&self.part(TimelinePart::Icon), name, false))
        .into(),
      Some(Mark::Media(media)) => well(&part).child(media).into(),
      None => Row::new().part(&self.part(TimelinePart::Dot)).into(),
    }
  }

  /// The rail: the mark over the line that runs down to the next item, as long as the item is. The last item has
  /// no line.
  pub fn rail(&mut self) -> Column {
    let mut rail = Column::new()
      .align_items(Alignment::Center)
      .overflow_visible()
      .part(&self.part(TimelinePart::Rail))
      .child(self.mark());
    if !self.last {
      rail = rail.child(Column::new().flex(1.0).part(&self.part(TimelinePart::Separator)));
    }
    rail
  }

  /// The title, the description, the time and the body, as wide as the room the rail and the action leave them: the
  /// texts wrap.
  pub fn content(&mut self) -> Column {
    let fill = Dimension::Pct(100.0);
    let mut content = Column::new()
      .flex(1.0)
      .min_width(Dimension::Px(0.0))
      .part(&self.part(TimelinePart::Content))
      .child(wrapping(&self.part(TimelinePart::Title), &self.title));
    if let Some(description) = &self.description {
      content = content.child(wrapping(&self.part(TimelinePart::Description), description));
    }
    if let Some(time) = &self.time {
      content = content.child(wrapping(&self.part(TimelinePart::Time), time));
    }
    if !self.body.is_empty() {
      content = content.child(
        Column::new()
          .width(fill)
          .align_items(Alignment::Stretch)
          .part(&self.part(TimelinePart::Body))
          .with_children(std::mem::take(&mut self.body)),
      );
    }
    content
  }

  /// The item in the style of its part, holding `children` side by side: a `listitem` to tooling, and inside it a
  /// button where the item is pressed.
  pub fn root(&self, children: Vec<Element>) -> Element {
    let part = self.part(TimelinePart::Item);
    let mut item = Row::new()
      .align_items(Alignment::Stretch)
      .overflow_visible()
      .describe("a11y_role", "listitem")
      .describe("a11y_name", self.title.clone());
    if let Some(description) = &self.description {
      item = item.describe("a11y_description", description.clone());
    }
    if self.current {
      item = item.describe("a11y_current", "true");
    }
    if self.pending {
      item = item.describe("a11y_busy", "true");
    }
    if let Some(key) = &self.key {
      item = item.key(key.clone());
    }
    if !self.pressable() {
      item = item.part(&part).with_children(children);
      if let Some(id) = &self.id {
        item = item.id(id.clone());
      }
      return item.into();
    }
    let handlers = self.on_select.clone();
    let mut button = pressable(&part, self.disabled)
      .flex(1.0)
      .justify(Justify::Start)
      .align_items(Alignment::Stretch)
      .describe("a11y_role", "button")
      .describe("a11y_name", self.title.clone())
      .focus_label(self.title.clone())
      .on_click(move |_| {
        for handler in &handlers {
          handler();
        }
      });
    if self.disabled {
      button = button.describe("a11y_disabled", "true");
    }
    if let Some(id) = &self.id {
      button = button.id(id.clone());
    }
    item.child(button.with_children(children)).into()
  }

  /// The action it was given, in the style of its part, at the top of the item's end: it keeps its room, and the
  /// texts beside it wrap.
  pub fn action(&mut self) -> Option<Row> {
    let action = self.action.take()?;
    Some(
      Row::new()
        .align_items(Alignment::Start)
        .part(&self.part(TimelinePart::Action))
        .child(action),
    )
  }

  /// How the default renderer builds an item: its rail beside its content, and its action at its end.
  pub fn render(mut self) -> Element {
    let rail = self.rail();
    let content = self.content();
    let mut children: Vec<Element> = vec![rail.into(), content.into()];
    children.extend(self.action().map(Into::into));
    self.root(children)
  }
}

impl Themed for Timeline {
  const NAME: &'static str = "timeline";
  type Renderer = TimelineRenderer;

  /// The design's timeline (`pencil/lurq-design-system.pen`, Timeline, in Data Display's Records): items of a rail
  /// of 12 beside their content, 12 apart. The rail is 4 down to a dot of 10 in the primary color and, 4 under it,
  /// a line of 1 in the border's color down to the next item. The content is the title, the body's type at 14 at 500
  /// in the foreground, the description 13 in the muted color with lines 1.4 high, and the time the mono type at 11
  /// in the muted color, 2 apart, with 20 under them. The last item has a hollow dot (the background with an edge of
  /// 1.5 in the input's color inside it), no line and no room under it.
  ///
  /// The board draws nothing else of it. An upcoming item has the hollow dot; a current one a ring of 2 in the
  /// dot's color, 2 out from it; a tone colors the dot. An icon is 14 in a well of 24 on the muted fill, media
  /// stands in a mark of 24, and every rail of a timeline that has one is 24 wide; a pending item's mark is a
  /// spinner of 12. The mark of a current item is ringed as its dot would be, and the well of an upcoming one is
  /// hollow. An item that is pressed has the veils of the pointer and the ring inside it, with the medium
  /// radius, and is faded where it is disabled. A body is 8 under the time.
  fn recipe() -> Recipe {
    use PaletteColor::{Background, Foreground, Input, Muted, MutedForeground, Ring, StateHover, StatePressed};
    use TimelinePart::{Action, Body, Content, Description, Dot, Icon, Item, Mark, Rail, Separator, Time, Title};
    use TimelineState::{Current, Disabled, Last, Pressable, Upcoming};
    let marked = TimelineState::Mark;
    let icon = TimelineState::Icon;
    let mut recipe = Recipe::new()
      .base(Item, |s| fading(s.gap(12.0).radius(RadiusSize::Md)))
      .state(Pressable, Item, |s| s.cursor(CursorIcon::Pointer))
      .when(Selector::new().state(Pressable).hovered(), Item, |s| s.veil(StateHover))
      .when(Selector::new().state(Pressable).pressed(), Item, |s| s.veil(StatePressed))
      .focus_visible(Item, |s| s.outline(Outline::new(BorderSize::Md, Ring).offset(-2.0)))
      .state(Disabled, Item, |s| s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default))
      .base(Rail, |s| s.width(12.0).padding_top(4.0).gap(4.0))
      // A rail of a timeline with marks is as wide as one, and its line stops 4 over the next item's mark.
      .state(marked, Rail, |s| s.width(24.0).padding_top(0.0).padding_bottom(4.0))
      .when(Selector::new().state(marked).state(Last), Rail, |s| s.padding_bottom(0.0))
      .base(Dot, |s| s.square(10.0).radius(RadiusSize::Full).background(PaletteColor::Primary))
      .state(Upcoming, Dot, |s| {
        s.background(Background)
          .border(Border::inside(1.5, Input))
      })
      .base(Mark, |s| s.square(24.0).radius(RadiusSize::Full))
      .state(icon, Mark, |s| s.background(Muted))
      .state(TimelineState::Pending, Mark, |s| s.square(12.0))
      .when(Selector::new().state(marked).state(TimelineState::Pending), Mark, |s| s.square(24.0))
      .base(Icon, |s| s.square(14.0).foreground(MutedForeground))
      .base(Separator, |s| s.width(1.0).background(PaletteColor::Border))
      .base(Content, |s| s.gap(2.0).padding_bottom(20.0))
      .state(Last, Content, |s| s.padding_bottom(0.0))
      .base(Title, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(14.0)
          .weight(FontWeight::Medium)
          .foreground(Foreground)
      })
      .base(Action, |s| s.gap(0.0))
      .base(Description, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .line_height(1.4)
          .foreground(MutedForeground)
      })
      .base(Time, |s| {
        s.font_family(FontFamily::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      .base(Body, |s| s.padding_top(6.0));
    for variant in TimelineVariant::all().iter().copied() {
      recipe = recipe
        .variant(variant, Dot, move |s| s.background(variant.color()))
        .when(Selector::new().variant(variant).state(Current), Dot, move |s| {
          s.outline(Outline::new(BorderSize::Md, variant.color()).offset(2.0))
        })
        // A mark of the current item is ringed as its dot would be.
        .when(Selector::new().variant(variant).state(Current), Mark, move |s| {
          s.outline(Outline::new(BorderSize::Md, variant.color()).offset(2.0))
        });
      if variant != TimelineVariant::Neutral {
        recipe = recipe.when(Selector::new().variant(variant).state(icon), Icon, move |s| {
          s.foreground(variant.color())
        });
      }
    }
    // The hollow dot keeps its fill whatever its tone, which colors its edge.
    for variant in TimelineVariant::all().iter().copied() {
      let color = if variant == TimelineVariant::Neutral {
        Input
      } else {
        variant.color()
      };
      recipe = recipe
        .when(Selector::new().variant(variant).state(Upcoming), Dot, {
          let color = color.clone();
          move |s| s.background(Background).border(Border::inside(1.5, color))
        })
        // The well of an upcoming item's icon is hollow as its dot would be.
        .when(Selector::new().variant(variant).state(Upcoming).state(icon), Mark, move |s| {
          s.background(Background).border(Border::inside(1.5, color))
        });
    }
    recipe
  }
}
