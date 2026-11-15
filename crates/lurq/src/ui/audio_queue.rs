//! A queue: the tracks a player plays one after another, the one it is on marked, the arrow keys moving between
//! them.

use std::sync::Arc;

use super::{
  Button, ButtonSize, ButtonVariant, Tooltip, icons, media_time_text, names,
  part_style::{FontFamily, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{fading, glyph, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    ctx::Ctx,
    events::MouseEvent,
    theme::{BorderSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{Column, Row, Stack, TextOverflow},
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, FocusGroup, dimension::Dimension, outline::Outline},
};

/// The parts of a queue a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AudioQueuePart {
  /// The list of the rows.
  Root,
  /// A row: what is pressed to play its track, and focused.
  Item,
  /// The 20 at the start of a row: the number, the play glyph under the pointer, the equalizer while it plays.
  Lead,
  /// The number of the row.
  Number,
  /// The glyph that stands for the number: `play` under the pointer, `equalizer` on the row that plays.
  LeadIcon,
  /// The artwork's box: the app's picture, or the `music` glyph on `Muted`.
  Media,
  /// The `music` glyph where there is no artwork.
  MediaIcon,
  /// The column of the title and who it is by.
  Text,
  /// The title.
  Title,
  /// Who it is by.
  Description,
  /// How long the track is.
  Duration,
  /// The "more" button, shown under the pointer and with the focus in the row.
  More,
}

names!(AudioQueuePart {
  Root => "root",
  Item => "item",
  Lead => "lead",
  Number => "number",
  LeadIcon => "lead_icon",
  Media => "media",
  MediaIcon => "media_icon",
  Text => "text",
  Title => "title",
  Description => "description",
  Duration => "duration",
  More => "more",
});

/// The states of a queue and of its rows a recipe styles, besides what the pointer and the keyboard do to them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AudioQueueState {
  /// A row is the track the player is on: the accent fill, the title heavier.
  Current,
  /// A row is the track the player is on, and it plays: the equalizer for the number.
  Playing,
  /// A row's track cannot be played: half the opacity, and a press does nothing.
  Unavailable,
  /// A row has artwork.
  Media,
  /// The queue takes no input.
  Disabled,
}

names!(AudioQueueState {
  Current => "current",
  Playing => "playing",
  Unavailable => "unavailable",
  Media => "media",
  Disabled => "disabled",
});

/// What builds a queue's nodes: a theme's in place of the default one.
pub type AudioQueueRenderer = Arc<dyn Fn(AudioQueueView) -> Element + Send + Sync>;

type Command = Arc<dyn Fn() + Send + Sync>;

/// A track of a queue.
///
/// ```
/// use lurq::ui::AudioQueueItem;
///
/// let track = AudioQueueItem::new("Paper Boats").description("Mira Sol").duration(232.0);
/// ```
#[derive(Clone)]
pub struct AudioQueueItem {
  title: Arc<str>,
  description: Option<Arc<str>>,
  media: Option<Element>,
  duration: Option<f64>,
  disabled: bool,
  on_select: Vec<Command>,
  on_more: Vec<Command>,
  more_anchor: Option<ElementRef>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl AudioQueueItem {
  /// A track called `title`.
  pub fn new(title: &str) -> Self {
    Self {
      title: Arc::from(title),
      description: None,
      media: None,
      duration: None,
      disabled: false,
      on_select: Vec::new(),
      on_more: Vec::new(),
      more_anchor: None,
      id: None,
      key: None,
    }
  }

  /// Who it is by, under the title.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// The artwork, cut to a box of 32. Where there is none, the `music` glyph on the muted fill stands for it.
  pub fn media(mut self, media: impl Into<Element>) -> Self {
    self.media = Some(media.into());
    self
  }

  /// How long it is, in seconds.
  pub fn duration(mut self, seconds: impl Into<f64>) -> Self {
    self.duration = Some(seconds.into());
    self
  }

  /// Whether it cannot be played: half the opacity, and a press does nothing.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Called on a press of the row, after the queue's `on_change`. Each call adds a handler.
  pub fn on_select(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_select.push(Arc::new(handler));
    self
  }

  /// Called on a press of the row's "more" button. Each call adds a handler. The button is there once one is given.
  pub fn on_more(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_more.push(Arc::new(handler));
    self
  }

  /// The ref the row's "more" button takes: what a menu of the app hangs from.
  pub fn more_anchor(mut self, anchor: ElementRef) -> Self {
    self.more_anchor = Some(anchor);
    self
  }

  /// The id of the row; its "more" button has it with `-more` after it.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }
}

/// A queue: the rows of the tracks a player plays, the one it is on marked.
///
/// ```
/// use lurq::{
///   core::Signal,
///   ui::{AudioQueue, AudioQueueItem},
/// };
///
/// let current = Signal::new(Some(1_usize));
/// let queue = AudioQueue::new(
///   current,
///   [
///     AudioQueueItem::new("Night Drive").description("Aurora Lane").duration(250.0),
///     AudioQueueItem::new("Paper Boats").description("Mira Sol").duration(232.0),
///   ],
/// )
/// .playing(true);
/// // The app holds which track it is on and is told the one a person presses.
/// let held = AudioQueue::controlled(Some(0), [AudioQueueItem::new("Night Drive")], |index| println!("{index}"));
/// ```
///
/// A press of a row tells [`AudioQueue::on_change`] its index, where it is another than the current one and can be
/// played. The arrow keys move between the rows, and Tab stops in the queue once.
#[derive(Clone)]
pub struct AudioQueue {
  current: Option<usize>,
  on_change: Vec<Arc<dyn Fn(usize) + Send + Sync>>,
  items: Vec<AudioQueueItem>,
  playing: bool,
  label: Arc<str>,
  more_label: Arc<str>,
  playing_label: Arc<str>,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl AudioQueue {
  /// A queue of `items`, on the track whose index `current` holds, which a press of a row sets.
  pub fn new(current: Signal<Option<usize>>, items: impl IntoIterator<Item = AudioQueueItem>) -> Self {
    let set = current.clone();
    let mut queue = Self::controlled(current.get(), items, move |index| set.set(Some(index)));
    queue.current = current.get();
    queue
  }

  /// A queue of `items`, on the track at `current`, held by the app, which is told the index of a row pressed.
  pub fn controlled(
    current: Option<usize>,
    items: impl IntoIterator<Item = AudioQueueItem>,
    on_change: impl Fn(usize) + Send + Sync + 'static,
  ) -> Self {
    Self {
      current,
      on_change: vec![Arc::new(on_change)],
      items: items.into_iter().collect(),
      playing: false,
      label: Arc::from("Queue"),
      more_label: Arc::from("More"),
      playing_label: Arc::from("Now playing"),
      disabled: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// Adds a track at the end.
  pub fn item(mut self, item: AudioQueueItem) -> Self {
    self.items.push(item);
    self
  }

  /// Adds tracks at the end.
  pub fn items(mut self, items: impl IntoIterator<Item = AudioQueueItem>) -> Self {
    self.items.extend(items);
    self
  }

  /// Whether the current track plays: the equalizer for its number.
  pub fn playing(mut self, playing: bool) -> Self {
    self.playing = playing;
    self
  }

  /// Called with the index of a row pressed, where it is another than the current one and can be played. Each call
  /// adds a handler.
  pub fn on_change(mut self, handler: impl Fn(usize) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// What the queue is called, to what reads the tree: "Queue" unless told.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Arc::from(label);
    self
  }

  /// What a row's "more" button is called: "More" unless told.
  pub fn more_label(mut self, label: &str) -> Self {
    self.more_label = Arc::from(label);
    self
  }

  /// What the equalizer of the row that plays is called: "Now playing" unless told.
  pub fn playing_label(mut self, label: &str) -> Self {
    self.playing_label = Arc::from(label);
    self
  }

  /// Whether the queue takes no input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Puts the queue in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this queue over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this queue.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the row Tab would stop on as it is under the pointer, held down or focused from the keyboard: for a
  /// gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Puts the queue in the tree of `ctx`'s component: its rows' "more" buttons with their tooltips.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let theme = current_theme();
    Self::render_view(self.view(&theme, Some(ctx)))
  }

  /// Builds a still picture of the queue in `theme`, for a test or a screenshot: its buttons with no tooltips. An
  /// app mounts the queue ([`AudioQueue::mount`]): it is not an element by itself.
  pub fn build(self, theme: &Theme) -> Element {
    Self::render_view(self.view(theme, None))
  }

  fn render_view(view: AudioQueueView) -> Element {
    match view.theme().renderer::<AudioQueue>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  fn case(&self) -> Case {
    Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(AudioQueueState::Disabled, self.disabled)
  }

  fn view(self, theme: &Theme, mut ctx: Option<&mut Ctx>) -> AudioQueueView {
    let recipe = theme.resolve::<AudioQueue>(&self.case(), Some(&self.rules));
    // Where Tab enters: the current row, or the first that can be played.
    let entry = self
      .current
      .filter(|current| self.items.get(*current).is_some_and(|item| !item.disabled))
      .or_else(|| self.items.iter().position(|item| !item.disabled));
    let base = self.case();
    let handlers = Arc::new(self.on_change);
    let rows = self
      .items
      .into_iter()
      .enumerate()
      .map(|(index, item)| {
        let current = self.current == Some(index);
        let case = base
          .clone()
          .state(AudioQueueState::Current, current)
          .state(AudioQueueState::Playing, current && self.playing)
          .state(AudioQueueState::Unavailable, item.disabled)
          .state(AudioQueueState::Media, item.media.is_some());
        let recipe = theme.resolve::<AudioQueue>(&case, Some(&self.rules));
        let forced = self.forced.filter(|_| entry == Some(index));
        let more = (!item.on_more.is_empty()).then(|| {
          let commands = item.on_more.clone();
          let mut button = Button::new(&self.more_label)
            .icon(icons::MORE_HORIZONTAL)
            .icon_only(true)
            .variant(ButtonVariant::Ghost)
            .size(ButtonSize::Xs)
            .disabled(item.disabled || self.disabled)
            .on_click(move |event: MouseEvent| {
              // The press is the button's, not the row's.
              event.stop_propagation();
              for command in &commands {
                command();
              }
            });
          if let Some(id) = &item.id {
            button = button.id(format!("{id}-more"));
          }
          if let Some(anchor) = &item.more_anchor {
            button = button.ref_element(anchor.clone());
          }
          let element = button.build(theme);
          match ctx.as_deref_mut() {
            Some(ctx) => Tooltip::new(&self.more_label).mount(ctx, element),
            None => element,
          }
        });
        AudioQueueRow {
          index,
          current,
          playing: current && self.playing,
          press: (!current && !item.disabled && !self.disabled).then(|| {
            let (handlers, own) = (handlers.clone(), item.on_select.clone());
            Arc::new(move || {
              for handler in handlers.iter() {
                handler(index);
              }
              for command in &own {
                command();
              }
            }) as Command
          }),
          entry: entry == Some(index),
          recipe,
          forced,
          more,
          item,
        }
      })
      .collect();
    AudioQueueView {
      rows,
      label: self.label,
      playing_label: self.playing_label,
      disabled: self.disabled,
      recipe,
      id: self.id,
      key: self.key,
    }
  }
}

/// A row of a queue as a renderer sees it.
pub struct AudioQueueRow {
  pub index: usize,
  pub current: bool,
  pub playing: bool,
  /// Whether Tab enters the queue at this row.
  pub entry: bool,
  /// The style of each part of this row.
  pub recipe: ResolvedRecipe,
  /// The "more" button, built, with its tooltip where the queue is mounted.
  pub more: Option<Element>,
  press: Option<Command>,
  forced: Option<Interaction>,
  item: AudioQueueItem,
}

impl AudioQueueRow {
  /// The style of a part of this row.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }
}

/// A queue as a renderer sees it: its rows, and the style of its parts.
pub struct AudioQueueView {
  pub rows: Vec<AudioQueueRow>,
  pub label: Arc<str>,
  pub playing_label: Arc<str>,
  pub disabled: bool,
  /// The style of the queue's own parts.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl AudioQueueView {
  /// The style of a part of the queue.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The start of a row: its number, the play glyph over it under the pointer, or the equalizer where it plays.
  pub fn lead(&self, row: &AudioQueueRow) -> Row {
    let part = row.part(AudioQueuePart::Lead);
    let lead = Row::new()
      .part(&part)
      .flex_shrink(0.0)
      .align_items(Alignment::Center)
      .justify(Justify::Center);
    if row.playing {
      let mut icon = glyph(&row.part(AudioQueuePart::LeadIcon), Name::from(icons::EQUALIZER), false);
      crate::node::NodeUpdate::describe(&mut *icon.node, Arc::from("a11y_name"), self.playing_label.clone());
      return lead.child(icon);
    }
    let number = row.part(AudioQueuePart::Number);
    let mut stack = Stack::new().child(
      Row::new()
        .part(&number)
        .child(number.text(&(row.index + 1).to_string()).nowrap()),
    );
    if row.press.is_some() {
      stack = stack.child(glyph(
        &row.part(AudioQueuePart::LeadIcon),
        Name::from(icons::PLAY),
        false,
      ));
    }
    lead.child(stack)
  }

  /// A row's artwork in its box, or the `music` glyph on the muted fill.
  pub fn media(&self, row: &mut AudioQueueRow) -> Row {
    let held = Row::new()
      .part(&row.part(AudioQueuePart::Media))
      .flex_shrink(0.0)
      .clip()
      .align_items(Alignment::Center)
      .justify(Justify::Center);
    match row.item.media.take() {
      Some(media) => held.child(media),
      None => held.child(glyph(
        &row.part(AudioQueuePart::MediaIcon),
        Name::from(icons::MUSIC),
        false,
      )),
    }
  }

  /// A row's title over who it is by, each cut where there is no room.
  pub fn text(&self, row: &AudioQueueRow) -> Column {
    let line = |part: ResolvedPart, text: &str| {
      Column::new()
        .width(Dimension::Pct(100.0))
        .min_width(Dimension::Px(0.0))
        .part(&part)
        .child(
          part
            .text(text)
            .nowrap()
            .text_overflow(TextOverflow::Elipsis)
            .width(Dimension::Pct(100.0)),
        )
    };
    let mut column = Column::new()
      .flex(1.0)
      .min_width(Dimension::Px(0.0))
      .part(&row.part(AudioQueuePart::Text))
      .child(line(row.part(AudioQueuePart::Title), &row.item.title));
    if let Some(description) = &row.item.description {
      column = column.child(line(row.part(AudioQueuePart::Description), description));
    }
    column
  }

  /// A row: what is pressed to play its track.
  pub fn row(&self, mut row: AudioQueueRow) -> Element {
    let part = row.part(AudioQueuePart::Item);
    let mut name = row.item.title.to_string();
    if let Some(description) = &row.item.description {
      name = format!("{name}, {description}");
    }
    let mut root = pressable(&part, row.item.disabled || self.disabled)
      .justify(Justify::Start)
      .align_items(Alignment::Center)
      .width(Dimension::Pct(100.0))
      .interaction_group(true)
      // The "more" button is a stop of Tab of its own, after its row.
      .stops_inside(true)
      .focus_entry(row.entry)
      .describe("a11y_role", "option")
      .describe("a11y_name", name)
      .describe("a11y_selected", if row.current { "true" } else { "false" })
      .child(self.lead(&row));
    root = root.child(self.media(&mut row)).child(self.text(&row));
    if let Some(seconds) = row.item.duration {
      let duration = row.part(AudioQueuePart::Duration);
      root = root.child(
        Row::new()
          .part(&duration)
          .flex_shrink(0.0)
          .child(duration.text(&media_time_text(seconds, seconds >= 3600.0)).nowrap()),
      );
    }
    if let Some(more) = row.more.take() {
      root = root.child(
        Row::new()
          .part(&row.part(AudioQueuePart::More))
          .flex_shrink(0.0)
          // The arrows move between the rows, not to the buttons in them.
          .focus_group(FocusGroup::none())
          .on_click(|event: MouseEvent| event.stop_propagation())
          .child(more),
      );
    }
    if let Some(press) = &row.press {
      let press = press.clone();
      root = root.on_click(move |_| press());
    }
    if let Some(id) = &row.item.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &row.item.key {
      root = root.key(key.clone());
    }
    root.into()
  }

  /// How the default renderer builds a queue: its rows one under another, the arrow keys moving between them.
  pub fn render(mut self) -> Element {
    let rows = std::mem::take(&mut self.rows);
    let mut root = Column::new()
      .part(&self.part(AudioQueuePart::Root))
      .focus_group(FocusGroup::vertical().wrap(false).one_stop(true))
      .describe("a11y_role", "listbox")
      .describe("a11y_name", self.label.clone());
    for row in rows {
      root = root.child(self.row(row));
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

impl Themed for AudioQueue {
  const NAME: &'static str = "audio_queue";
  type Renderer = AudioQueueRenderer;

  /// The design's queue row (`pencil/lurq-design-system.pen`, Audio Player/Queue Row `U2PYO0`): 48 high, padded 8,
  /// 12 between its parts, `radius-md`. The number 20 wide in mono 12 in the muted ink, the artwork 32 with
  /// `radius-sm`, the title 13/500 over who it is by 12 in the muted ink, the length in mono 11, and a "more" button
  /// 28 with an icon of 14 under the pointer. The row that plays: the accent fill, the `equalizer` glyph 16 for the
  /// number, the title 600. Under the pointer: the hover veil, `play` 14 for the number. Focused: a ring of 2 in
  /// the ring's color inside it. Unavailable: half the opacity.
  fn recipe() -> Recipe {
    use AudioQueuePart::*;
    use AudioQueueState::{Current, Disabled, Unavailable};
    use PaletteColor::{Foreground, Highlight, Muted, MutedForeground, Ring, StateHover, StatePressed};
    Recipe::new()
      .base(Root, |s| s.foreground(Foreground))
      .state(Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
      .base(Item, |s| {
        fading(
          s.height(48.0)
            .padding_x(8.0)
            .gap(12.0)
            .radius(RadiusSize::Md)
            .foreground(Foreground)
            .cursor(CursorIcon::Pointer),
        )
      })
      .row_density(Selector::new(), Item, 48.0)
      .hovered(Item, |s| s.veil(StateHover))
      .pressed(Item, |s| s.veil(StatePressed))
      // The ring is inside the row: rows touch, and the list may clip.
      .focus_visible(Item, |s| s.outline(Outline::new(BorderSize::Md, Ring).offset(-2.0)))
      .state(Current, Item, |s| s.background(Highlight).cursor(CursorIcon::Default))
      .state(Unavailable, Item, |s| s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default))
      .base(Lead, |s| s.width(20.0))
      .base(Number, |s| {
        s.font_family(FontFamily::Mono)
          .font_size(12.0)
          .foreground(MutedForeground)
      })
      // Under the pointer the play glyph stands for the number.
      .within_hovered(Number, |s| s.opacity(0.0))
      .base(LeadIcon, |s| s.square(14.0).foreground(Foreground).opacity(0.0))
      .within_hovered(LeadIcon, |s| s.opacity(1.0))
      .state(AudioQueueState::Playing, LeadIcon, |s| s.square(16.0).opacity(1.0))
      .base(Media, |s| s.square(32.0).radius(RadiusSize::Sm).background(Muted))
      .base(MediaIcon, |s| s.square(16.0).foreground(MutedForeground))
      .base(Text, |s| s.gap(2.0))
      .base(Title, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .weight(FontWeight::Medium)
      })
      .state(Current, Title, |s| s.weight(FontWeight::SemiBold))
      .base(Description, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(12.0)
          .foreground(MutedForeground)
      })
      .base(Duration, |s| {
        s.font_family(FontFamily::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      // Shown under the pointer and with the focus in the row.
      .base(More, |s| s.opacity(0.0))
      .within(More, |s| s.opacity(1.0))
  }
}
