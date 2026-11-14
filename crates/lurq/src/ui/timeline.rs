//! A timeline: what happened, in its order, each thing a mark on a line with its title, its description and its
//! time.

use std::{marker::PhantomData, sync::Arc};

use super::{
  button::{Button, ButtonSize, ButtonVariant},
  names,
  part_style::{FontFamily, Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  scroll_area::ScrollArea,
  shared::{Orientation, fading, glyph, pressable},
  spinner::{Spinner, SpinnerPart},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::MouseEvent,
    theme::{BorderSize, Breakpoint, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{Column, Row},
  core::{ElementRef, Signal},
  layout::{
    Alignment,
    layout_kind::Justify,
    text_style::{FontWeight, TextAlign},
  },
  node::{CursorIcon, Element, FocusGroup, border::Border, dimension::Dimension, outline::Outline},
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

/// How large a timeline is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TimelineSize {
  /// A log: a dot of 8, a title of 13 with its time beside it, items as close as rows, which follow the theme's
  /// density.
  Sm,
  /// The board's: a dot of 10, a title of 14, its description and its time under it, 20 between items.
  #[default]
  Md,
}

names!(TimelineSize {
  Sm => "sm",
  Md => "md",
});

/// Where an item's text stands about the line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TimelineAlign {
  /// After the line, every item: the board's.
  #[default]
  Start,
  /// On either side of the line, item by item, the time on the other side: a history read down the middle. Where
  /// there is too little room it is `Start`: a [mounted](Timeline::mount) timeline given less than
  /// [`Timeline::ALTERNATE_LEAST`] across, or one built as an element in a window narrower than the theme's `md`
  /// breakpoint.
  Alternate,
  /// The time in a column of its own before the line, the rest after it: a log read by its times. Across, the
  /// time is over the line.
  TimeColumn,
}

names!(TimelineAlign {
  Start => "start",
  Alternate => "alternate",
  TimeColumn => "time_column",
});

/// The parts of a timeline a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimelinePart {
  /// The column of the items, or the row of them across.
  Root,
  /// An item: its rail beside its content. What is pressed where it is pressed.
  Item,
  /// The mark and the line from it to the next item: down an item's start, or across its top.
  Rail,
  /// The dot of an item that has no icon and no media.
  Dot,
  /// The well an item's icon stands in, or what holds its media, or its spinner while it is pending.
  Mark,
  /// The icon in the mark.
  Icon,
  /// The line from an item's mark to the next item's.
  Separator,
  /// The column of the title, the description, the time and the body.
  Content,
  /// The row of the title and, in the small size, the time beside it.
  Header,
  /// What happened.
  Title,
  /// The text under the title.
  Description,
  /// When it happened, as the app writes it.
  Time,
  /// What the app puts under the time: a card, a comment, a list of files.
  Body,
  /// The action at the end of the item, beside what is pressed.
  Action,
  /// The other side of the line, where the time stands apart from the rest: the time column, or the side an
  /// alternate item's text is not on.
  Opposite,
  /// The item at the end that shows more: its button.
  More,
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
  Header => "header",
  Title => "title",
  Description => "description",
  Time => "time",
  Body => "body",
  Action => "action",
  Opposite => "opposite",
  More => "more",
});

/// The states of a timeline and of its items a recipe styles, besides what the pointer and the keyboard do to an
/// item that is pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimelineState {
  /// Of the timeline: its items one under another, the line down. One of `Vertical` and `Horizontal` is on.
  Vertical,
  /// Of the timeline: its items side by side, the line across.
  Horizontal,
  /// Of the timeline: the text after the line ([`TimelineAlign::Start`]). One of `Start`, `Alternate` and
  /// `TimeColumn` is on.
  Start,
  /// Of the timeline: the text on either side of the line.
  Alternate,
  /// Of the timeline: the time in a column of its own.
  TimeColumn,
  /// Of the timeline: an item of it has an icon or media, and every rail is as wide as a mark.
  Mark,
  /// Of an item: its text is before the line, on the side the time column is on.
  Before,
  /// Of an item: the one that is now.
  Current,
  /// Of an item: it is to come, its dot hollow.
  Upcoming,
  /// Of an item: it is running, a spinner for its mark.
  Pending,
  /// Of an item: the last, with no line from its mark and no room after its content.
  Last,
  /// Of an item: it has an icon in a well for its mark.
  Icon,
  /// Of an item: it has the app's media (an avatar) for its mark.
  Media,
  /// Of an item: it is pressed, the timeline's keys moving between it and the others.
  Pressable,
  /// Of an item: it is pressed and takes nothing now.
  Disabled,
  /// Of the item at the end that shows more.
  More,
  /// Of the item at the end that shows more: it is loading.
  Loading,
}

names!(TimelineState {
  Vertical => "vertical",
  Horizontal => "horizontal",
  Start => "start",
  Alternate => "alternate",
  TimeColumn => "time_column",
  Mark => "mark",
  Before => "before",
  Current => "current",
  Upcoming => "upcoming",
  Pending => "pending",
  Last => "last",
  Icon => "icon",
  Media => "media",
  Pressable => "pressable",
  Disabled => "disabled",
  More => "more",
  Loading => "loading",
});

/// What builds a timeline's nodes: a theme's in place of the default one.
pub type TimelineRenderer = Arc<dyn Fn(TimelineView) -> Element + Send + Sync>;

type Select = Arc<dyn Fn() + Send + Sync>;

/// What stands for an item's mark.
#[derive(Clone)]
enum Mark {
  Icon(Name),
  Media(Element),
}

/// An item of a [`Timeline`]: what happened, with its description, its time, its mark and what the app puts under
/// it. Its `T` is the value a timeline's [`on_select`](Timeline::on_select) is told, which
/// [`value`](Self::value) gives it.
#[derive(Clone)]
pub struct TimelineItem<T = ()> {
  value: T,
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
      value: (),
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
}

impl<T> TimelineItem<T> {
  /// The item with `value`, which a timeline's [`on_select`](Timeline::on_select) is told when it is pressed: the
  /// id of the event, of the commit, of the step.
  pub fn value<V>(self, value: V) -> TimelineItem<V> {
    TimelineItem {
      value,
      title: self.title,
      description: self.description,
      time: self.time,
      mark: self.mark,
      variant: self.variant,
      current: self.current,
      upcoming: self.upcoming,
      pending: self.pending,
      body: self.body,
      action: self.action,
      on_select: self.on_select,
      disabled: self.disabled,
      id: self.id,
      key: self.key,
    }
  }

  /// The text under the title. It wraps.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// When it happened, as the app writes it: "Nov 10", "2 hours ago", "14:02". It is under the description, beside
  /// the title in the small size, or in a column of its own ([`TimelineAlign`]).
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

  /// Whether it is the one that is now: its mark is ringed, and it says `a11y_current` to tooling.
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

  /// The one action, at the end of the item beside its title: a [`Button`] or a [`Link`](super::Link). A second
  /// takes the place of the first.
  pub fn action(mut self, action: impl Into<Element>) -> Self {
    self.action = Some(action.into());
    self
  }

  /// Called when the item is pressed, or Enter or Space is pressed while it has the focus, before the timeline's
  /// [`on_select`](Timeline::on_select). An item with a handler is pressed. A second handler is called after the
  /// first.
  pub fn on_select(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_select.push(Arc::new(handler));
    self
  }

  /// Whether an item that is pressed takes nothing now: it is faded and the keys pass it by.
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
/// use lurq::ui::{Orientation, Timeline, TimelineAlign, TimelineItem, TimelineSize, TimelineVariant};
///
/// let history = Timeline::new([
///   TimelineItem::new("Released 0.60").description("Blur and frosted glass land in the renderer.").time("Nov 10"),
///   TimelineItem::new("Merged node-blur").description("Blurs follow transforms and rounded clips.").time("Nov 8"),
///   TimelineItem::new("Opened pull request").description("Blur a node and its subtree as one layer.").time("Nov 3"),
/// ])
/// .label("History");
/// // A deploy: done, failed, running, to come.
/// let deploy = Timeline::new([
///   TimelineItem::new("Built").icon("check").variant(TimelineVariant::Success),
///   TimelineItem::new("Tests failed").icon("close").variant(TimelineVariant::Destructive),
///   TimelineItem::new("Retrying").pending(true).current(true),
///   TimelineItem::new("Deploy").upcoming(true),
/// ]);
/// // An order's tracking across, each step told by its value when it is pressed.
/// let order = Timeline::new([
///   TimelineItem::new("Ordered").value(1u32),
///   TimelineItem::new("Shipped").value(2),
///   TimelineItem::new("Delivered").value(3).upcoming(true),
/// ])
/// .orientation(Orientation::Horizontal)
/// .on_select(|step: &u32| println!("step {step}"));
/// // A log: small, its times in a column, more loaded at the end.
/// let log = Timeline::new([TimelineItem::new("Deployed").time("14:02"), TimelineItem::new("Built").time("13:58")])
///   .size(TimelineSize::Sm)
///   .align(TimelineAlign::TimeColumn)
///   .on_load_more(|| println!("fetch the next page"));
/// ```
///
/// Each item is a mark on a line that runs to the next item's, beside its title, its description and its time,
/// which wrap: the line is as long as what is beside it. The mark is a dot in the color of the item's
/// [`variant`](TimelineItem::variant), ringed where the item is [`current`](TimelineItem::current) and hollow where
/// it is [`upcoming`](TimelineItem::upcoming); an [`icon`](TimelineItem::icon) in a well, the app's
/// [`media`](TimelineItem::media), or a spinner while it is [`pending`](TimelineItem::pending). The last item has
/// no line from its mark.
///
/// It is read and not operated, but where an item is pressed (its own [`on_select`](TimelineItem::on_select), or
/// the timeline's, which makes every item pressed) the pressed items are one stop of Tab, and the arrows, Home, End
/// and the first letters of a title move between them. To tooling it is a `list` of `listitem`s, the current one
/// saying `a11y_current`.
///
/// Across ([`orientation`](Self::orientation)) the items share the width, each at least as wide as its recipe says,
/// and a timeline wider than its room scrolls sideways in a [`ScrollArea`], keeping the focused item in view.
///
/// A long history is loaded a page at a time, as a feed is: with [`on_load_more`](Self::on_load_more) the line
/// runs on to an item at the end that shows more, and [`loading`](Self::loading) turns its mark into a spinner
/// while the next page comes. The timeline builds every item it holds.
///
/// Its two sides of [`TimelineAlign::Alternate`] need room: [mounted](Self::mount), a timeline folds them to one
/// side where it is given less than [`ALTERNATE_LEAST`](Self::ALTERNATE_LEAST) across, whatever the window, and
/// shows the form that fits from its first frame and as a panel is dragged across that width. Built as an element
/// of its own it has no room to read, and folds them in a window narrower than the theme's `md` breakpoint.
#[derive(Clone)]
pub struct Timeline<T = ()> {
  items: Vec<TimelineItem<T>>,
  orientation: Orientation,
  size: Name,
  align: TimelineAlign,
  on_select: Vec<Arc<dyn Fn(&T) + Send + Sync>>,
  on_load_more: Vec<Select>,
  loading: bool,
  more_label: Option<Arc<str>>,
  loading_text: Option<Arc<str>>,
  label: Option<Arc<str>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
  /// Whether its room, as a mounted timeline last measured it, is wide enough for alternate sides.
  room: Option<bool>,
}

impl<T: Send + Sync + 'static> Timeline<T> {
  /// The least width across which a mounted timeline keeps the two sides of [`TimelineAlign::Alternate`]: in less,
  /// they fold to one. Each side then has some 220 for its text.
  pub const ALTERNATE_LEAST: f32 = 480.0;

  /// A timeline of `items`, the first at the top, or at the start across.
  pub fn new(items: impl IntoIterator<Item = TimelineItem<T>>) -> Self {
    Self {
      items: items.into_iter().collect(),
      orientation: Orientation::Vertical,
      size: TimelineSize::default().into(),
      align: TimelineAlign::default(),
      on_select: Vec::new(),
      on_load_more: Vec::new(),
      loading: false,
      more_label: None,
      loading_text: None,
      label: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
      room: None,
    }
  }

  /// Adds an item after those it has.
  pub fn item(mut self, item: TimelineItem<T>) -> Self {
    self.items.push(item);
    self
  }

  /// Adds items after those it has.
  pub fn items(mut self, items: impl IntoIterator<Item = TimelineItem<T>>) -> Self {
    self.items.extend(items);
    self
  }

  /// Which way the items run: `Vertical`, one under another, the line down (the default and the board's), or
  /// `Horizontal`, side by side, the line across and each item's text under its mark.
  pub fn orientation(mut self, orientation: Orientation) -> Self {
    self.orientation = orientation;
    self
  }

  /// A [`TimelineSize`], or a size the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// Where the items' text stands about the line: a [`TimelineAlign`].
  pub fn align(mut self, align: TimelineAlign) -> Self {
    self.align = align;
    self
  }

  /// Called with the value of the item pressed, after the item's own handlers: every item that is not disabled is
  /// then pressed. A second handler is called after the first.
  pub fn on_select(mut self, handler: impl Fn(&T) + Send + Sync + 'static) -> Self {
    self.on_select.push(Arc::new(handler));
    self
  }

  /// Called when the item at the end that shows more is pressed: there is more to load. A timeline with it has
  /// that item; the app stops giving it once it has loaded everything. A second handler is called after the first.
  pub fn on_load_more(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_load_more.push(Arc::new(handler));
    self
  }

  /// Whether the next page is being loaded: the item that shows more has a spinner for its mark and takes no press.
  pub fn loading(mut self, loading: bool) -> Self {
    self.loading = loading;
    self
  }

  /// What the button that shows more says: "Show more" when not given.
  pub fn more_label(mut self, label: &str) -> Self {
    self.more_label = Some(Arc::from(label));
    self
  }

  /// What that button says while the next page is loading: "Loading" when not given.
  pub fn loading_text(mut self, text: &str) -> Self {
    self.loading_text = Some(Arc::from(text));
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

  /// The id of the list of the items.
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
    match theme.renderer::<Self>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// How the items are laid out where the timeline is built: alternate sides need room, the timeline's own where
  /// it is mounted and has measured it, else a window at the theme's `md` breakpoint or wider; in less they are one
  /// side. Across, the text is under the marks.
  fn laid_out(&self) -> TimelineAlign {
    let roomy = self
      .room
      .unwrap_or_else(|| super::current_breakpoint() >= Some(Breakpoint::Md));
    match self.align {
      TimelineAlign::Alternate if self.orientation.is_vertical() && roomy => TimelineAlign::Alternate,
      TimelineAlign::TimeColumn => TimelineAlign::TimeColumn,
      _ => TimelineAlign::Start,
    }
  }

  fn view(self, theme: &Theme) -> TimelineView {
    let align = self.laid_out();
    let horizontal = !self.orientation.is_vertical();
    let marked = self.items.iter().any(|item| item.mark.is_some());
    let base = Case {
      variant: None,
      size: Some(self.size),
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(TimelineState::Vertical, !horizontal)
    .state(TimelineState::Horizontal, horizontal)
    .state(TimelineState::Start, align == TimelineAlign::Start)
    .state(TimelineState::Alternate, align == TimelineAlign::Alternate)
    .state(TimelineState::TimeColumn, align == TimelineAlign::TimeColumn)
    .state(TimelineState::Mark, marked);
    let recipe = theme.resolve::<Self>(&base, Some(&self.rules));
    let more = !self.on_load_more.is_empty();
    let count = self.items.len();
    let chosen: Arc<[Arc<dyn Fn(&T) + Send + Sync>]> = self.on_select.into();
    let pressed_at = |item: &TimelineItem<T>| (!item.on_select.is_empty() || !chosen.is_empty()) && !item.disabled;
    let forced_at = self.items.iter().position(pressed_at);
    let grouped = self
      .items
      .iter()
      .any(|item| !item.on_select.is_empty() || !chosen.is_empty());
    let small = Name::from(TimelineSize::Sm) == self.size;
    let items = self
      .items
      .into_iter()
      .enumerate()
      .map(|(index, item)| {
        let before = align == TimelineAlign::Alternate && index % 2 == 1;
        let last = index + 1 == count && !more;
        let mut handlers = item.on_select;
        if !chosen.is_empty() {
          let (chosen, value) = (chosen.clone(), item.value);
          handlers.push(Arc::new(move || {
            for handler in chosen.iter() {
              handler(&value);
            }
          }));
        }
        let case = Case {
          variant: Some(item.variant),
          ..base.clone()
        }
        .state(TimelineState::Before, before)
        .state(TimelineState::Current, item.current)
        .state(TimelineState::Upcoming, item.upcoming)
        .state(TimelineState::Pending, item.pending)
        .state(TimelineState::Last, last)
        .state(
          TimelineState::Icon,
          matches!(item.mark, Some(Mark::Icon(_))) && !item.pending,
        )
        .state(
          TimelineState::Media,
          matches!(item.mark, Some(Mark::Media(_))) && !item.pending,
        )
        .state(TimelineState::Pressable, !handlers.is_empty())
        .state(TimelineState::Disabled, item.disabled);
        TimelineItemView {
          title: item.title,
          description: item.description,
          time: item.time,
          mark: item.mark,
          current: item.current,
          pending: item.pending,
          last,
          before,
          horizontal,
          align,
          small,
          body: item.body,
          action: item.action,
          on_select: handlers,
          disabled: item.disabled,
          forced: if forced_at == Some(index) { self.forced } else { None },
          recipe: theme.resolve::<Self>(&case, Some(&self.rules)),
          id: item.id,
          key: item.key,
        }
      })
      .collect();
    let more = more.then(|| {
      // Its dot is hollow, more to come, and a spinner while the next page loads.
      let case = base
        .clone()
        .state(TimelineState::More, true)
        .state(TimelineState::Last, true)
        .state(TimelineState::Upcoming, !self.loading)
        .state(TimelineState::Pending, self.loading)
        .state(TimelineState::Loading, self.loading);
      TimelineMore {
        label: self.more_label.unwrap_or_else(|| Arc::from("Show more")),
        loading_text: self.loading_text.unwrap_or_else(|| Arc::from("Loading")),
        loading: self.loading,
        on_load_more: self.on_load_more,
        horizontal,
        align,
        recipe: theme.resolve::<Self>(&case, Some(&self.rules)),
      }
    });
    TimelineView {
      items,
      more,
      label: self.label,
      horizontal,
      grouped,
      recipe,
      id: self.id,
      key: self.key,
    }
  }
}

impl<T: Clone + Send + Sync + 'static> Timeline<T> {
  /// The timeline as a component that reads its own room: alternate sides fold to one where it is given less than
  /// [`ALTERNATE_LEAST`](Self::ALTERNATE_LEAST) across, in a narrow panel of a wide window too, with no frame of the
  /// other form when it first shows or when the panel is dragged across that width. It takes the width of what holds
  /// it.
  ///
  /// ```
  /// use lurq::{app::ctx::Ctx, node::Element, ui::{Timeline, TimelineAlign, TimelineItem}};
  ///
  /// # fn history(ctx: &mut Ctx) -> Element {
  /// Timeline::new([TimelineItem::new("0.60").time("Nov 10"), TimelineItem::new("0.59").time("Oct 21")])
  ///   .align(TimelineAlign::Alternate)
  ///   .mount(ctx)
  /// # }
  /// ```
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    ctx.mount::<TimelineHost<T>>(TimelineHostProps { timeline: self })
  }
}

struct TimelineHostProps<T> {
  timeline: Timeline<T>,
}

impl<T> PartialEq for TimelineHostProps<T> {
  /// A timeline built again is drawn again: its items are the app's as it builds them.
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl<T> crate::app::component::DevtoolsInspectable for TimelineHostProps<T> {
  fn inspect(&self, formatter: &mut crate::app::component::DevtoolsFormatter<'_>) {
    formatter.value("Timeline", format!("{} items", self.timeline.items.len()));
  }
}

/// A mounted timeline: the column that takes the room it is given, which it measures before the frame is painted,
/// around the timeline laid out for that room.
struct TimelineHost<T> {
  anchor: ElementRef,
  roomy: Signal<Option<bool>>,
  _values: PhantomData<fn() -> T>,
}

impl<T: Clone + Send + Sync + 'static> Component for TimelineHost<T> {
  type Props = TimelineHostProps<T>;

  fn create(ctx: &mut Ctx) -> Self {
    let (anchor, roomy) = (ElementRef::new(), ctx.signal(None));
    let watch = roomy.clone();
    // Written only where the room crosses the least width: a panel dragged within one side of it renders nothing.
    anchor.observe_rect_before_paint(move |rect| {
      let fits = Some(rect.width >= Timeline::<T>::ALTERNATE_LEAST);
      if watch.get_untracked() != fits {
        watch.set(fits);
      }
    });
    Self {
      anchor,
      roomy,
      _values: PhantomData,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let mut timeline = ctx.props::<TimelineHostProps<T>>().timeline.clone();
    timeline.room = self.roomy.get();
    Column::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Stretch)
      .overflow_visible()
      .ref_element(self.anchor.clone())
      .child(timeline.build(&current_theme()))
  }
}

impl<T: Send + Sync + 'static> From<Timeline<T>> for Element {
  fn from(timeline: Timeline<T>) -> Self {
    timeline.build(&current_theme())
  }
}

/// A text of `part` that wraps in the room it is given, its lines at the end of it where `end`.
fn text_of(part: &ResolvedPart, content: &str, end: bool) -> Column {
  let fill = Dimension::Pct(100.0);
  let mut text = part.text(content).width(fill);
  if end {
    text = text.text_align(TextAlign::End);
  }
  Column::new().width(fill).part(part).child(text)
}

/// A timeline as a renderer sees it: its items, the item that shows more, and the style of its list.
pub struct TimelineView {
  /// The items, the first at the top or the start.
  pub items: Vec<TimelineItemView>,
  /// The item at the end that shows more, where the timeline has `on_load_more`.
  pub more: Option<TimelineMore>,
  /// What the timeline is called, to tooling.
  pub label: Option<Arc<str>>,
  /// Whether the items run across.
  pub horizontal: bool,
  /// Whether an item is pressed: the pressed items are then one stop of Tab, and the keys move between them.
  pub grouped: bool,
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

  /// The list of `items` in the style of its part, a `list` to tooling: a column, or across a row in a scroll area
  /// that scrolls sideways. Where an item is pressed, the items are one focus group whose keys go the way they run.
  pub fn root(&self, items: Vec<Element>) -> Element {
    let part = self.part(TimelinePart::Root);
    let group = if self.horizontal {
      FocusGroup::horizontal()
    } else {
      FocusGroup::vertical()
    }
    .one_stop(true)
    .wrap(false)
    .typeahead(true);
    macro_rules! dress {
      ($list:expr) => {{
        let mut list = $list.overflow_visible().part(&part).describe("a11y_role", "list");
        if self.grouped {
          list = list.focus_group(group);
        }
        if let Some(label) = &self.label {
          list = list.describe("a11y_name", label.clone());
        }
        if let Some(id) = &self.id {
          list = list.id(id.clone());
        }
        if let Some(key) = &self.key {
          list = list.key(key.clone());
        }
        list.with_children(items)
      }};
    }
    if !self.horizontal {
      return dress!(Column::new().align_items(Alignment::Stretch)).into();
    }
    // Across, the items share the room and keep their least width: the row fills the area, and is wider, to
    // scroll, where their least widths are more than it.
    let row = dress!(Row::new().flex(1.0).align_items(Alignment::Stretch));
    ScrollArea::new()
      .orientation(Orientation::Horizontal)
      .fill_width(true)
      .width(Length::Fill)
      .child(row)
      .build(self.theme())
  }

  /// How the default renderer builds a timeline: its items, then the item that shows more.
  pub fn render(mut self) -> Element {
    let mut items: Vec<Element> = std::mem::take(&mut self.items)
      .into_iter()
      .map(TimelineItemView::render)
      .collect();
    if let Some(more) = self.more.take() {
      items.push(more.render());
    }
    self.root(items)
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
  /// Whether its text is before the line: an alternate item on the side the time column is on.
  pub before: bool,
  /// Whether the items run across.
  pub horizontal: bool,
  /// Where its text stands about the line, as the timeline is laid out.
  pub align: TimelineAlign,
  small: bool,
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

  /// Whether the item is pressed: it, or its timeline, has a handler.
  pub fn pressable(&self) -> bool {
    !self.on_select.is_empty()
  }

  /// Whether the time stands apart from the rest of the text, on the other side of the line.
  fn time_apart(&self) -> bool {
    self.align != TimelineAlign::Start
  }

  /// The item's mark: the spinner while it is pending, its icon in a well, its media, or the dot.
  pub fn mark(&mut self) -> Element {
    let part = self.part(TimelinePart::Mark);
    if self.pending {
      return spinner_mark(&part);
    }
    match self.mark.take() {
      Some(Mark::Icon(name)) => well(&part)
        .child(glyph(&self.part(TimelinePart::Icon), name, false))
        .into(),
      Some(Mark::Media(media)) => well(&part).child(media).into(),
      None => Row::new().part(&self.part(TimelinePart::Dot)).into(),
    }
  }

  /// The rail: the mark and the line that runs from it to the next item, down or across, as long as the item is.
  /// The last item has no line.
  pub fn rail(&mut self) -> Element {
    let mark = self.mark();
    rail(
      &self.part(TimelinePart::Rail),
      &self.part(TimelinePart::Separator),
      mark,
      self.last,
      self.horizontal,
    )
  }

  /// The title, and in the small size the time beside it where the time is not apart.
  fn header(&self) -> Element {
    let title = Column::new().flex(1.0).min_width(Dimension::Px(0.0)).child(text_of(
      &self.part(TimelinePart::Title),
      &self.title,
      self.before,
    ));
    let beside = self.small && !self.time_apart();
    let mut header = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Start)
      .part(&self.part(TimelinePart::Header));
    if self.before
      && beside
      && let Some(time) = &self.time
    {
      header = header.child(self.part(TimelinePart::Time).text(time));
    }
    header = header.child(title);
    if !self.before
      && beside
      && let Some(time) = &self.time
    {
      header = header.child(self.part(TimelinePart::Time).text(time));
    }
    header.into()
  }

  /// The title, the description, the time where it is with them, and the body, as wide as the room the rail and the
  /// other side leave them: the texts wrap. Before the line its lines are at its end.
  pub fn content(&mut self) -> Column {
    let fill = Dimension::Pct(100.0);
    let mut content = Column::new()
      .flex(1.0)
      .min_width(Dimension::Px(0.0))
      .align_items(if self.before {
        Alignment::End
      } else {
        Alignment::Stretch
      })
      .part(&self.part(TimelinePart::Content))
      .child(self.header());
    if let Some(description) = &self.description {
      content = content.child(text_of(&self.part(TimelinePart::Description), description, self.before));
    }
    if !self.small
      && !self.time_apart()
      && let Some(time) = &self.time
    {
      content = content.child(text_of(&self.part(TimelinePart::Time), time, self.before));
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

  /// The other side of the line, where the time stands apart: the time column, the side an alternate item's text
  /// is not on, or over the line across. `None` where the time is with the rest.
  pub fn opposite(&self) -> Option<Column> {
    if !self.time_apart() {
      return None;
    }
    // An alternate item's time faces the line: at the end of the side before it.
    let end = !self.horizontal && !self.before;
    let mut side = Column::new()
      .min_width(Dimension::Px(0.0))
      .align_items(if end { Alignment::End } else { Alignment::Start })
      .part(&self.part(TimelinePart::Opposite));
    if self.align == TimelineAlign::Alternate {
      side = side.flex(1.0);
    }
    if let Some(time) = &self.time {
      side = side.child(text_of(&self.part(TimelinePart::Time), time, end));
    }
    Some(side)
  }

  /// The action it was given, in the style of its part: beside what is pressed, never in it.
  pub fn action(&mut self) -> Option<Row> {
    let action = self.action.take()?;
    Some(
      Row::new()
        .align_items(Alignment::Start)
        .focus_group(FocusGroup::none())
        .part(&self.part(TimelinePart::Action))
        .child(action),
    )
  }

  /// The item in the style of its part, holding `children` (side by side, or one under another across), with its
  /// action beside them: a `listitem` to tooling, and inside it a button around `children` where the item is
  /// pressed.
  pub fn root(&mut self, children: Vec<Element>) -> Element {
    let part = self.part(TimelinePart::Item);
    let action = self.action();
    let mut item: Element = if self.horizontal {
      let column = Column::new()
        .width(Dimension::Pct(100.0))
        .align_items(Alignment::Stretch);
      if self.pressable() {
        column.with_children(children).into()
      } else {
        column.part(&part).with_children(children).into()
      }
    } else if self.pressable() {
      Row::new()
        .flex(1.0)
        .min_width(Dimension::Px(0.0))
        .align_items(Alignment::Stretch)
        .with_children(children)
        .into()
    } else {
      Row::new()
        .flex(1.0)
        .min_width(Dimension::Px(0.0))
        .align_items(Alignment::Stretch)
        .part(&part)
        .with_children(children)
        .into()
    };
    if self.pressable() {
      let handlers = self.on_select.clone();
      let mut button = pressable(&part, self.disabled)
        .flex(1.0)
        .min_width(Dimension::Px(0.0))
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
      if self.horizontal {
        // Across, what is pressed is as wide as the item, and its text wraps in it.
        button = button.width(Dimension::Pct(100.0));
      }
      item = button.child(item).into();
    }
    let least = part.rest().min_width.and_then(|width| width.resolve(self.theme()));
    let gap = part.rest().gap.unwrap_or_default();
    // What the item says to tooling, on the list item around what is pressed and the action.
    macro_rules! said {
      ($outer:expr) => {{
        let mut outer = $outer
          .overflow_visible()
          .describe("a11y_role", "listitem")
          .describe("a11y_name", self.title.clone());
        if let Some(description) = &self.description {
          outer = outer.describe("a11y_description", description.clone());
        }
        if self.current {
          outer = outer.describe("a11y_current", "true");
        }
        if self.pending {
          outer = outer.describe("a11y_busy", "true");
        }
        if !self.pressable()
          && let Some(id) = &self.id
        {
          outer = outer.id(id.clone());
        }
        if let Some(key) = &self.key {
          outer = outer.key(key.clone());
        }
        Element::from(outer)
      }};
    }
    if self.horizontal {
      // Across, the items share the row from their least width up, whatever their text: a long title wraps in its
      // item.
      let mut column = Column::new().flex_full(1.0, 0.0, None).align_items(Alignment::Stretch);
      if let Some(least) = least {
        column = column.width(least);
      }
      let column = column.child(item);
      return said!(match action {
        Some(action) => column.child(action),
        None => column,
      });
    }
    // What is pressed, or the item itself, fills the row beside its action, which is at the outer side.
    let row = Row::new().align_items(Alignment::Start).spacing(gap);
    said!(match (action, self.before) {
      (Some(action), true) => row.child(action).child(item),
      (Some(action), false) => row.child(item).child(action),
      (None, _) => row.child(item),
    })
  }

  /// How the default renderer builds an item: its rail beside its content, the time on the other side where it is
  /// apart, its action at its end; across, the time over the rail and the content under it.
  pub fn render(mut self) -> Element {
    let rail = self.rail();
    let content: Element = self.content().into();
    let opposite: Option<Element> = self.opposite().map(Into::into);
    let children = match (self.horizontal, opposite, self.before) {
      (true, Some(opposite), _) => vec![opposite, rail, content],
      (true, None, _) => vec![rail, content],
      (false, Some(opposite), true) => vec![content, rail, opposite],
      (false, Some(opposite), false) => vec![opposite, rail, content],
      (false, None, _) => vec![rail, content],
    };
    self.root(children)
  }
}

/// A row that holds a mark in its middle, in the style of `part`.
fn well(part: &ResolvedPart) -> Row {
  Row::new()
    .justify(Justify::Center)
    .align_items(Alignment::Center)
    .overflow_visible()
    .part(part)
}

/// A spinner as wide as the mark of `part`, in it.
fn spinner_mark(part: &ResolvedPart) -> Element {
  let side = part.rest().width.unwrap_or(12.0.into());
  well(part)
    .child(
      Spinner::new()
        .decorative(true)
        .style(SpinnerPart::Indicator, move |s| s.square(side))
        .build(part.theme()),
    )
    .into()
}

/// A rail of `mark` and, but for the last item, the line of `line` from it to the next item: down, or across.
fn rail(part: &ResolvedPart, line: &ResolvedPart, mark: Element, last: bool, across: bool) -> Element {
  let rest = (!last).then(|| Element::from(Column::new().flex(1.0).part(line)));
  if across {
    let mut rail = Row::new()
      .align_items(Alignment::Center)
      .overflow_visible()
      .part(part)
      .child(mark);
    if let Some(line) = rest {
      rail = rail.child(line);
    }
    return rail.into();
  }
  let mut rail = Column::new()
    .align_items(Alignment::Center)
    .overflow_visible()
    .part(part)
    .child(mark);
  if let Some(line) = rest {
    rail = rail.child(line);
  }
  rail.into()
}

/// The item at the end of a timeline that shows more, as a renderer sees it.
pub struct TimelineMore {
  /// What its button says.
  pub label: Arc<str>,
  /// What its button says while the next page is loading.
  pub loading_text: Arc<str>,
  /// Whether the next page is loading.
  pub loading: bool,
  on_load_more: Vec<Select>,
  horizontal: bool,
  align: TimelineAlign,
  recipe: ResolvedRecipe,
}

impl TimelineMore {
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  /// Its mark: a hollow dot, more to come, or a spinner while it loads.
  pub fn mark(&self) -> Element {
    if self.loading {
      spinner_mark(&self.part(TimelinePart::Mark))
    } else {
      Row::new().part(&self.part(TimelinePart::Dot)).into()
    }
  }

  /// Its button: a small ghost one that calls `on_load_more`, saying `loading_text` and taking no press while it
  /// loads.
  pub fn button(&self) -> Element {
    let handlers = self.on_load_more.clone();
    let label = if self.loading { &self.loading_text } else { &self.label };
    Button::new(label.as_ref())
      .variant(ButtonVariant::Ghost)
      .size(ButtonSize::Sm)
      .disabled(self.loading)
      .on_click(move |_: MouseEvent| {
        for handler in &handlers {
          handler();
        }
      })
      .build(self.recipe.theme())
  }

  /// How the default renderer builds it: its rail, the end of the line, beside its button.
  pub fn render(self) -> Element {
    let rail = rail(
      &self.part(TimelinePart::Rail),
      &self.part(TimelinePart::Separator),
      self.mark(),
      true,
      self.horizontal,
    );
    let more = Row::new().part(&self.part(TimelinePart::More)).child(self.button());
    let mut item = if self.horizontal {
      Element::from(
        Column::new()
          .align_items(Alignment::Start)
          .part(&self.part(TimelinePart::Item))
          .child(rail)
          .child(more),
      )
    } else {
      // Down, it stands on the line where the items' marks are: after the time column, or in the middle.
      let opposite = (self.align != TimelineAlign::Start).then(|| {
        let side = Column::new().part(&self.part(TimelinePart::Opposite));
        if self.align == TimelineAlign::Alternate {
          side.flex(1.0)
        } else {
          side
        }
      });
      let more = if self.align == TimelineAlign::Alternate {
        more.flex(1.0)
      } else {
        more
      };
      let mut row = Row::new()
        .align_items(Alignment::Start)
        .part(&self.part(TimelinePart::Item));
      if let Some(opposite) = opposite {
        row = row.child(opposite);
      }
      row.child(rail).child(more).into()
    };
    if self.loading {
      item = Column::new().describe("a11y_busy", "true").child(item).into();
    }
    Column::new()
      .align_items(Alignment::Stretch)
      .describe("a11y_role", "listitem")
      .describe(
        "a11y_name",
        if self.loading {
          self.loading_text.clone()
        } else {
          self.label.clone()
        },
      )
      .child(item)
      .into()
  }
}

impl<T: Send + Sync + 'static> Themed for Timeline<T> {
  const NAME: &'static str = "timeline";
  type Renderer = TimelineRenderer;

  /// The design's timeline (`pencil/lurq-design-system.pen`, Timeline, in Data Display's Records): items of a rail
  /// of 12 beside their content, 12 apart. The rail is 4 down to a dot of 10 in the primary color and, 4 under it,
  /// a line of 1 in the border's color down to the next item. The content is the title, the body's type at 14 at 500
  /// in the foreground, the description 13 in the muted color with lines 1.4 high, and the time the mono type at 11
  /// in the muted color, 2 apart, with 20 under them. The last item has a hollow dot (the background with an edge of
  /// 1.5 in the input's color inside it), no line and no room under it.
  ///
  /// The board draws nothing else of it, and the rest is drawn in its parts. An upcoming item has the hollow dot; a
  /// current one a ring of 2 in the dot's color, 2 out from it; a tone colors the dot. An icon is 14 in a well of 24
  /// on the muted fill, media stands in a mark of 24, and every rail of a timeline that has one is 24 wide; a
  /// pending item's mark is a spinner of 12. The mark of a current item is ringed as its dot would be, and the well
  /// of an upcoming one is hollow. An item that is pressed has the veils of the pointer and the ring inside it, with
  /// the medium radius, and is faded where it is disabled. A body is 8 under the time.
  ///
  /// The small size is a log: a dot of 8 on a rail of 8, 10 from the content, the title 13 with the time beside it,
  /// the description 12, items 28 apart at the least, as a row is at each density, with 4 under the content. Across,
  /// the rail is a row 12 high (24 with marks), its line 4 short of the next mark, the content 8 under it and 12
  /// from the next item; an item is at least 160 wide (120 small). The time column is 80 wide (64 small), its time
  /// at its end, 2 down to stand by the title.
  fn recipe() -> Recipe {
    use PaletteColor::{Background, Foreground, Input, Muted, MutedForeground, Ring, StateHover, StatePressed};
    use TimelinePart::{
      Action, Body, Content, Description, Dot, Header, Icon, Item, Mark, More, Opposite, Rail, Separator, Time, Title,
    };
    use TimelineState::{Current, Disabled, Horizontal, Last, Pressable, Upcoming};
    let marked = TimelineState::Mark;
    let icon = TimelineState::Icon;
    let pending = TimelineState::Pending;
    let small = TimelineSize::Sm;
    let mut recipe = Recipe::new()
      .base(Item, |s| fading(s.gap(12.0).radius(RadiusSize::Md)))
      .size(small, Item, |s| s.gap(10.0))
      .state(Horizontal, Item, |s| s.gap(0.0).min_width(160.0))
      .when(Selector::new().size(small).state(Horizontal), Item, |s| s.min_width(120.0))
      .state(Pressable, Item, |s| s.cursor(CursorIcon::Pointer))
      .when(Selector::new().state(Pressable).hovered(), Item, |s| s.veil(StateHover))
      .when(Selector::new().state(Pressable).pressed(), Item, |s| s.veil(StatePressed))
      .focus_visible(Item, |s| s.outline(Outline::new(BorderSize::Md, Ring).offset(-2.0)))
      .state(Disabled, Item, |s| s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default))
      // The rail: down, 12 wide; across, a row as high as the mark whose line stops 4 short of the next mark.
      .base(Rail, |s| s.width(12.0).padding_top(4.0).gap(4.0))
      .size(small, Rail, |s| s.width(8.0).gap(3.0))
      // A rail of a timeline with marks is as wide as one, and its line stops 4 over the next item's mark.
      .state(marked, Rail, |s| s.width(24.0).padding_top(0.0).padding_bottom(4.0))
      .when(Selector::new().size(small).state(marked), Rail, |s| s.width(20.0))
      .when(Selector::new().state(marked).state(Last), Rail, |s| s.padding_bottom(0.0))
      // Across, the rail is as high as a mark whatever the mark: a spinner's item keeps its text where the others do.
      .state(Horizontal, Rail, |s| {
        s.width(Length::Auto)
          .height(12.0)
          .padding_top(0.0)
          .padding_bottom(0.0)
          .padding_end(4.0)
      })
      .when(Selector::new().size(small).state(Horizontal), Rail, |s| s.height(10.0))
      .when(Selector::new().state(marked).state(Horizontal), Rail, |s| s.height(24.0))
      .when(Selector::new().size(small).state(marked).state(Horizontal), Rail, |s| s.height(20.0))
      .base(Dot, |s| s.square(10.0).radius(RadiusSize::Full).background(PaletteColor::Primary))
      .size(small, Dot, |s| s.square(8.0))
      .state(Upcoming, Dot, |s| s.background(Background).border(Border::inside(1.5, Input)))
      .base(Mark, |s| s.square(24.0).radius(RadiusSize::Full))
      .size(small, Mark, |s| s.square(20.0))
      .state(icon, Mark, |s| s.background(Muted))
      .state(pending, Mark, |s| s.square(12.0))
      .when(Selector::new().size(small).state(pending), Mark, |s| s.square(10.0))
      .when(Selector::new().state(marked).state(pending), Mark, |s| s.square(24.0))
      .when(Selector::new().size(small).state(marked).state(pending), Mark, |s| s.square(20.0))
      .base(Icon, |s| s.square(14.0).foreground(MutedForeground))
      .size(small, Icon, |s| s.square(12.0))
      .base(Separator, |s| s.width(1.0).background(PaletteColor::Border))
      .state(Horizontal, Separator, |s| s.width(Length::Auto).height(1.0))
      .base(Content, |s| s.gap(2.0).padding_bottom(20.0))
      .size(small, Content, |s| s.padding_bottom(4.0).min_height(28.0))
      // A small item is a row: as high as one at each density.
      .by_density(Selector::new().size(small).state(TimelineState::Vertical), Content, |s, step| {
        s.min_height(28.0 + step)
      })
      .state(Last, Content, |s| s.padding_bottom(0.0))
      .state(Horizontal, Content, |s| s.padding_bottom(0.0).padding_top(8.0).padding_end(12.0))
      .when(Selector::new().size(small).state(Horizontal), Content, |s| s.padding_top(6.0).min_height(0.0))
      .base(Header, |s| s.gap(8.0))
      .base(Title, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(14.0)
          .weight(FontWeight::Medium)
          .foreground(Foreground)
      })
      .size(small, Title, |s| s.font_size(13.0))
      .base(Description, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .line_height(1.4)
          .foreground(MutedForeground)
      })
      .size(small, Description, |s| s.font_size(12.0))
      .base(Time, |s| {
        s.font_family(FontFamily::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      .base(Body, |s| s.padding_top(6.0))
      .base(Action, |s| s.gap(0.0))
      // The time column: 80, its time at its end, 2 down to stand by the title; across, a line over the rail.
      .state(TimelineState::TimeColumn, Opposite, |s| s.width(80.0).padding_top(2.0))
      .when(Selector::new().size(small).state(TimelineState::TimeColumn), Opposite, |s| {
        s.width(64.0).padding_top(1.0)
      })
      .state(TimelineState::Alternate, Opposite, |s| s.padding_top(2.0))
      .when(Selector::new().state(Horizontal).state(TimelineState::TimeColumn), Opposite, |s| {
        s.width(Length::Auto)
          .padding_top(0.0)
          .padding_bottom(6.0)
          .padding_end(12.0)
          .min_height(16.0)
      })
      // The item that shows more: its small button of 28 in the middle of its mark, down or across.
      .when(Selector::new().state(TimelineState::More).state(TimelineState::Vertical), Rail, |s| s.padding_top(9.0))
      .when(Selector::new().size(small).state(TimelineState::More).state(TimelineState::Vertical), Rail, |s| {
        s.padding_top(10.0)
      })
      .when(
        Selector::new().state(marked).state(TimelineState::More).state(TimelineState::Vertical),
        Rail,
        |s| s.padding_top(2.0),
      )
      .when(
        Selector::new().size(small).state(marked).state(TimelineState::More).state(TimelineState::Vertical),
        Rail,
        |s| s.padding_top(4.0),
      )
      .state(Horizontal, More, |s| s.padding_top(4.0));
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
