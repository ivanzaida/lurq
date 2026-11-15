//! The Signals tab: every signal, memo and effect of the inspected window in a list, and beside it the one
//! selected in the list: what owns it, what it notifies, and how it changed. Built from the board
//! (`pencil/lurq-devtools.pen`, Signals · dark) with the design system's components.
//!
//! The changes are the engine's own record of them (`ComponentValueChangeDebug`), every one of the last 64 of each
//! signal and memo with the moment it happened. The tab records nothing itself: recording is which of them it shows.

use std::sync::Arc;

use super::{
  kit::{self, FILL, KeyFocus, counted, line, mono, sans, short_tag, short_type_name},
  snapshot::{DevToolsNode, DevToolsSnapshot},
};
use crate::{
  app::{
    ctx::{ComponentValueChangeDebug, Ctx},
    theme::{BorderSize, PaletteColor, RadiusSize},
  },
  components::{Column, Row, Spacer},
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, border::Border},
  ui::{
    Badge, BadgeSize, BadgeVariant, Button, ButtonSize, ButtonVariant, EmptyState, Icon, Recipe, SearchField,
    SearchFieldSize, Splitter, SplitterPane, Table, TableColumn, TableCount, Toggle, TogglePart, ToggleSize,
    ToggleState, icons,
  },
};

/// The sizes the board gives the tab.
const LIST_WIDTH: f32 = 540.0;
const LIST_MIN_WIDTH: f32 = 360.0;
const DETAIL_MIN_WIDTH: f32 = 480.0;
const HEADER_HEIGHT: f32 = 48.0;
const PADDING: f32 = kit::DETAIL_PADDING;
const ID_WIDTH: f32 = 60.0;
const VALUE_WIDTH: f32 = 88.0;
const TYPE_WIDTH: f32 = 152.0;
const SUBS_WIDTH: f32 = 40.0;
const TIME_WIDTH: f32 = 148.0;
const ARROW_WIDTH: f32 = 48.0;
const NODE_HEIGHT: f32 = 64.0;
const RELATION_WIDTH: f32 = 103.0;

/// What a row of the list is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, crate::DevtoolsInspectable)]
pub(crate) enum ReactiveKind {
  Signal,
  Memo,
  Effect,
}

impl ReactiveKind {
  fn label(self) -> &'static str {
    match self {
      Self::Signal => "Signal",
      Self::Memo => "Memo",
      Self::Effect => "Effect",
    }
  }

  fn icon(self) -> &'static str {
    match self {
      Self::Signal => icons::BOLT,
      Self::Memo => icons::PULSE,
      Self::Effect => icons::PLAY_CIRCLE,
    }
  }
}

/// What a row of the list is told apart by: a signal and a memo may have the same number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, crate::DevtoolsInspectable)]
pub(crate) struct ReactiveKey {
  kind: ReactiveKind,
  id: usize,
}

/// A signal, a memo or an effect of the inspected window.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Reactive {
  key: ReactiveKey,
  /// What it holds, as it is written. An effect holds nothing.
  value: Option<String>,
  type_name: String,
  /// How many read it and are told when it changes.
  subscribers: usize,
  /// The component it was made in.
  owner: String,
  /// Its changes as the engine recorded them, the oldest first.
  changes: Vec<ComponentValueChangeDebug>,
}

/// Everything reactive in the inspected window: the signals, then the memos, then the effects, each in the order
/// they were made.
pub(crate) fn reactives(snapshot: &DevToolsSnapshot) -> Vec<Reactive> {
  fn collect(node: &DevToolsNode, all: &mut Vec<Reactive>) {
    let owner = short_tag(&node.tag);
    let key = |kind, id| ReactiveKey { kind, id };
    all.extend(node.signals.iter().map(|signal| Reactive {
      key: key(ReactiveKind::Signal, signal.id),
      value: signal.formatted_value(),
      type_name: format!("Signal<{}>", short_type_name(&signal.type_name)),
      subscribers: signal.subscriber_count(),
      owner: owner.to_owned(),
      changes: signal.history(),
    }));
    all.extend(node.memos.iter().map(|memo| Reactive {
      key: key(ReactiveKind::Memo, memo.id),
      value: memo.formatted_value(),
      type_name: format!("Memo<{}>", short_type_name(&memo.type_name)),
      subscribers: memo.subscriber_count(),
      owner: owner.to_owned(),
      changes: memo.history(),
    }));
    all.extend(node.effects.iter().map(|effect| Reactive {
      key: key(ReactiveKind::Effect, effect.id),
      value: None,
      type_name: "Effect".to_owned(),
      subscribers: 0,
      owner: owner.to_owned(),
      changes: Vec::new(),
    }));
    node.children.iter().for_each(|child| collect(child, all));
  }
  let mut all = Vec::new();
  if let Some(root) = &snapshot.root {
    collect(root, &mut all);
  }
  all.sort_by_key(|reactive| reactive.key);
  all
}

/// The number of the last change the engine recorded of any of them: the moment "now" in its count of changes.
pub(crate) fn latest_change(all: &[Reactive]) -> u64 {
  all
    .iter()
    .filter_map(|reactive| reactive.changes.last())
    .map(|change| change.sequence)
    .max()
    .unwrap_or(0)
}

/// Whether a row is one the filter asks for: every word of it is in the row's number, value, type or owner.
pub(crate) fn passes(reactive: &Reactive, filter: &str) -> bool {
  let row = format!(
    "#{} {} {} {}",
    reactive.key.id,
    reactive.value.as_deref().unwrap_or(""),
    reactive.type_name,
    reactive.owner
  )
  .to_lowercase();
  filter.split_whitespace().all(|word| row.contains(&word.to_lowercase()))
}

/// Which of the engine's changes the tab shows: those of the stretches it was recording in, by the numbers the
/// engine gives the changes. It records while its last stretch has no end.
#[derive(Clone, Debug, PartialEq, Eq, crate::DevtoolsInspectable)]
pub(crate) struct Recording {
  stretches: Vec<Stretch>,
}

/// A stretch of recording: the changes after `after`, up to and with `until`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, crate::DevtoolsInspectable)]
struct Stretch {
  after: u64,
  until: Option<u64>,
}

impl Default for Recording {
  /// Recording from the start: every change the engine still has is shown.
  fn default() -> Self {
    Self {
      stretches: vec![Stretch { after: 0, until: None }],
    }
  }
}

impl Recording {
  pub(crate) fn on(&self) -> bool {
    self.stretches.last().is_some_and(|stretch| stretch.until.is_none())
  }

  /// Stops recording at the change `latest`, or starts again after it: what changed in between is not shown.
  pub(crate) fn toggle(&mut self, latest: u64) {
    match self.stretches.last_mut() {
      Some(stretch) if stretch.until.is_none() => stretch.until = Some(latest),
      _ => self.stretches.push(Stretch {
        after: latest,
        until: None,
      }),
    }
  }

  /// Forgets what was shown: only what changes after `latest` is, and only while recording.
  pub(crate) fn clear(&mut self, latest: u64) {
    let until = (!self.on()).then_some(latest);
    self.stretches = vec![Stretch { after: latest, until }];
  }

  pub(crate) fn shows(&self, change: &ComponentValueChangeDebug) -> bool {
    self
      .stretches
      .iter()
      .any(|stretch| change.sequence > stretch.after && stretch.until.is_none_or(|until| change.sequence <= until))
  }

  /// The changes of `reactive` that are shown, the newest first.
  fn shown(&self, reactive: &Reactive) -> Vec<ComponentValueChangeDebug> {
    reactive
      .changes
      .iter()
      .rev()
      .filter(|change| self.shows(change))
      .cloned()
      .collect()
  }
}

/// What the tab keeps between renders.
#[derive(Clone)]
pub(crate) struct SignalsState {
  recording: Signal<Recording>,
  /// The row selected in the list: the first one until a person selects another.
  selected: Signal<Option<ReactiveKey>>,
  filter: Signal<String>,
  /// The filter's box and the key that goes to it.
  pub(crate) filter_key: KeyFocus,
}

impl SignalsState {
  pub(crate) fn new(ctx: &mut Ctx) -> Self {
    Self {
      recording: ctx.signal(Recording::default()),
      selected: ctx.signal(None),
      filter: ctx.signal(String::new()),
      filter_key: KeyFocus::new(ctx),
    }
  }
}

/// The tab: the list and what is known of the row selected in it, with a divider between them that a person drags.
pub(super) fn view(ctx: &mut Ctx, snapshot: &DevToolsSnapshot, state: &SignalsState) -> Element {
  state.filter_key.answer(ctx);
  let all = reactives(snapshot);
  let latest = latest_change(&all);
  let recording = state.recording.get();
  let filter = state.filter.get();
  let listed: Vec<Reactive> = all
    .iter()
    .filter(|reactive| passes(reactive, &filter))
    .cloned()
    .collect();
  // The row a person selected, while the inspected window still has it; else the first of the list.
  let selected = state
    .selected
    .get()
    .and_then(|key| all.iter().find(|reactive| reactive.key == key))
    .or(listed.first())
    .cloned();

  let detail = detail(selected.as_ref(), &recording);
  let list = Column::new()
    .size(FILL, FILL)
    .child(toolbar(state, &recording, latest))
    .child(Column::new().width(FILL).flex(1.0).border_top(line()).child(list(
      ctx,
      listed,
      selected.as_ref().map(|reactive| reactive.key),
      !filter.trim().is_empty(),
      state,
    )));
  let panes = Splitter::new()
    .id("devtools-signals")
    .pane(
      SplitterPane::new(list)
        .length(LIST_WIDTH)
        .min(LIST_MIN_WIDTH)
        .label("Signals"),
    )
    .pane(SplitterPane::new(detail).min(DETAIL_MIN_WIDTH).label("Selected signal"))
    .mount(ctx);
  Row::new().size(FILL, FILL).flex(1.0).child(panes).into()
}

fn toolbar(state: &SignalsState, recording: &Recording, latest: u64) -> Row {
  let on = recording.on();
  let (toggled, cleared) = (state.recording.clone(), state.recording.clone());
  kit::toolbar()
    .child(
      Toggle::new(if on { "Recording..." } else { "Record" })
        .icon(if on { icons::DOT_CIRCLE } else { icons::CIRCLE })
        .size(ToggleSize::Xs)
        .rules(&recording_toggle())
        .on(on)
        .on_change(move |_| toggled.update(|recording| recording.toggle(latest)))
        .id("devtools-signals-record"),
    )
    .child(
      Button::new("Clear")
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Xs)
        .icon(icons::TRASH)
        .on_click(move |_| cleared.update(|recording| recording.clear(latest)))
        .id("devtools-signals-clear"),
    )
    .child(
      Row::new().flex(1.0).child(
        SearchField::new(state.filter.clone())
          .placeholder("Filter signals...")
          .label("Filter signals")
          .shortcut(kit::FIND_KEY)
          .ref_element(state.filter_key.field.clone())
          .size(SearchFieldSize::Sm)
          .input_rules(&kit::compact_input())
          .full_width(true)
          .id("devtools-signals-filter"),
      ),
    )
}

/// The board's Recording: the toolbar's toggle, in the destructive color on its muted fill while it records.
fn recording_toggle() -> Recipe {
  use PaletteColor::{Destructive, DestructiveMuted};
  kit::outlined_toggle().state(ToggleState::On, TogglePart::Root, |s| {
    s.background(DestructiveMuted)
      .foreground(Destructive)
      .border(Border::inside(0.0, DestructiveMuted))
  })
}

fn list(
  ctx: &mut Ctx,
  listed: Vec<Reactive>,
  selected: Option<ReactiveKey>,
  filtering: bool,
  state: &SignalsState,
) -> Element {
  let of = |kind| listed.iter().filter(|reactive| reactive.key.kind == kind).count();
  let (signals, memos, effects) = (
    of(ReactiveKind::Signal),
    of(ReactiveKind::Memo),
    of(ReactiveKind::Effect),
  );
  let select = state.selected.clone();
  let columns = vec![
    TableColumn::new("id", "ID")
      .width(ID_WIDTH)
      .cell(|reactive: &Reactive| {
        Row::new()
          .spacing(8.0)
          .align_items(Alignment::Center)
          .child(
            Icon::new(reactive.key.kind.icon())
              .size(14.0)
              .color(PaletteColor::MutedForeground),
          )
          .child(mono(&format!("#{}", reactive.key.id), 12.0, FontWeight::Normal).color(PaletteColor::MutedForeground))
          .into()
      }),
    TableColumn::new("value", "Value")
      .width(VALUE_WIDTH)
      .cell(|reactive: &Reactive| value(reactive.value.as_deref(), 12.0)),
    TableColumn::new("type", "Type")
      .width(TYPE_WIDTH)
      .text(|reactive: &Reactive| reactive.type_name.clone())
      .mono(true)
      .muted(true),
    TableColumn::new("subs", "Subs")
      .width(SUBS_WIDTH)
      .text(|reactive: &Reactive| reactive.subscribers.to_string())
      .numeric(),
    TableColumn::new("owner", "Owner")
      .text(|reactive: &Reactive| reactive.owner.clone())
      .mono(true),
  ];
  let table = Table::new_shared(columns, Arc::new(listed))
    .row_key(|reactive: &Reactive| reactive.key)
    .label("Signals, memos and effects")
    .id("devtools-signals-list")
    .checkboxes(false)
    .selected_controlled(selected, move |keys: Vec<ReactiveKey>| {
      // A press with Ctrl on the selected row would leave none selected: the detail always shows one.
      if let Some(key) = keys.into_iter().next_back() {
        select.set(Some(key));
      }
    })
    .count(true)
    .count_text(move |_: TableCount| {
      format!(
        "{} · {} · {}",
        counted(signals, "signal"),
        counted(memos, "memo"),
        counted(effects, "effect")
      )
    })
    .empty_text(if filtering {
      "Nothing matches the filter"
    } else {
      "The inspected window has no signals, memos or effects"
    });
  kit::dense_table(table).mount(ctx)
}

/// A value as it is written, or the dash of what has none.
fn value(value: Option<&str>, size: f32) -> Element {
  match value {
    Some(value) => mono(value, size, FontWeight::Medium).into(),
    None => mono("—", size, FontWeight::Normal)
      .color(PaletteColor::MutedForeground)
      .into(),
  }
}

/// What is known of the selected row: what it is, where it stands between its owner and its subscribers, and
/// its changes.
fn detail(selected: Option<&Reactive>, recording: &Recording) -> Element {
  let Some(reactive) = selected else {
    return Column::new()
      .size(FILL, FILL)
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .child(
        EmptyState::new("Nothing selected")
          .description("Select a signal, a memo or an effect to see what owns it and how it changed.")
          .icon(icons::BOLT),
      )
      .into();
  };
  let changes = recording.shown(reactive);
  let said = match changes.len() {
    0 => "no changes".to_owned(),
    1 => "last change".to_owned(),
    count => format!("last {count} changes"),
  };
  Column::new()
    .size(FILL, FILL)
    .child(
      Row::new()
        .width(FILL)
        .height(HEADER_HEIGHT)
        .padding_left(PADDING)
        .padding_right(16.0)
        .spacing(10.0)
        .align_items(Alignment::Center)
        .border_bottom(line())
        .child(mono(&format!("#{}", reactive.key.id), 15.0, FontWeight::SemiBold))
        .child(
          Badge::new(reactive.key.kind.label())
            .variant(BadgeVariant::Outline)
            .size(BadgeSize::Sm),
        )
        .child(mono(&reactive.type_name, 12.0, FontWeight::Normal).color(PaletteColor::MutedForeground)),
    )
    .child(graph(reactive))
    .child(
      Row::new()
        .width(FILL)
        .height(40.0)
        .padding_horizontal(PADDING)
        .spacing(8.0)
        .align_items(Alignment::Center)
        .border_bottom(line())
        .child(sans("Change history", 13.0, FontWeight::Medium))
        .child(Spacer::new().flex(1.0))
        .child(sans(&said, 12.0, FontWeight::Normal).color(PaletteColor::MutedForeground)),
    )
    .child(
      Column::new()
        .width(FILL)
        .flex(1.0)
        .child(history(changes, reactive.key.kind, recording.on())),
    )
    .into()
}

/// The selected row between what owns it and what it notifies.
fn graph(reactive: &Reactive) -> Column {
  let node = |edge: PaletteColor, text: Column, number: Option<Element>| {
    let mut node = Row::new()
      .height(NODE_HEIGHT)
      .flex(1.0)
      .padding_horizontal(16.0)
      .spacing(12.0)
      .align_items(Alignment::Center)
      .rounded(RadiusSize::Md)
      .background(PaletteColor::Background)
      .border(Border::inside(BorderSize::Sm, edge))
      .child(text.flex(1.0).spacing(4.0));
    if let Some(number) = number {
      node = node.child(number);
    }
    node
  };
  let relation = |label: &str| {
    Column::new()
      .width(RELATION_WIDTH)
      .spacing(4.0)
      .padding_horizontal(8.0)
      .padding_bottom(17.0)
      .align_items(Alignment::Center)
      .child(sans(label, 11.0, FontWeight::Normal).color(PaletteColor::MutedForeground))
      .child(
        Row::new()
          .width(FILL)
          .height(12.0)
          .align_items(Alignment::Center)
          .child(Row::new().height(1.0).flex(1.0).background(PaletteColor::Input))
          .child(
            Icon::new(icons::CHEVRON_RIGHT)
              .size(12.0)
              .color(PaletteColor::MutedForeground),
          ),
      )
  };
  let quiet = |text: &str| sans(text, 11.0, FontWeight::Normal).color(PaletteColor::MutedForeground);
  let number = |text: &str| -> Element { mono(text, 15.0, FontWeight::SemiBold).into() };
  Column::new()
    .width(FILL)
    .padding_top(4.0)
    .padding_horizontal(PADDING)
    .padding_bottom(20.0)
    .background(PaletteColor::Card)
    .foreground(PaletteColor::CardForeground)
    .border_bottom(line())
    .child(
      Row::new()
        .height(36.0)
        .padding_top(8.0)
        .align_items(Alignment::Center)
        .child(sans("DEPENDENCY GRAPH", 11.0, FontWeight::SemiBold).color(PaletteColor::MutedForeground)),
    )
    .child(
      Row::new()
        .width(FILL)
        .align_items(Alignment::Center)
        .child(node(
          PaletteColor::Border,
          Column::new()
            .child(mono(&reactive.owner, 13.0, FontWeight::Medium))
            .child(quiet("Component")),
          None,
        ))
        .child(relation("owns"))
        .child(node(
          // The one the graph is of has the stronger edge.
          PaletteColor::Input,
          Column::new()
            .child(mono(&format!("#{}", reactive.key.id), 13.0, FontWeight::Medium))
            .child(mono(&reactive.type_name, 11.0, FontWeight::Normal).color(PaletteColor::MutedForeground)),
          reactive.value.as_deref().map(number),
        ))
        .child(relation("notifies"))
        .child(node(
          PaletteColor::Border,
          Column::new()
            .child(sans("subscribers", 13.0, FontWeight::Medium))
            .child(quiet("tracked reads")),
          Some(number(&reactive.subscribers.to_string())),
        )),
    )
}

/// The changes shown, the newest first: when each happened, what the value was and what it became.
fn history(changes: Vec<ComponentValueChangeDebug>, kind: ReactiveKind, recording: bool) -> Element {
  let quiet = |text: &str| -> Element {
    mono(text, 13.0, FontWeight::Normal)
      .color(PaletteColor::MutedForeground)
      .into()
  };
  let columns = vec![
    TableColumn::new("time", "Time")
      .width(TIME_WIDTH)
      .cell(move |change: &ComponentValueChangeDebug| quiet(&change.timestamp)),
    TableColumn::new("previous", "Previous").cell(move |change: &ComponentValueChangeDebug| quiet(&change.from_value)),
    TableColumn::new("arrow", "")
      .width(ARROW_WIDTH)
      .cell(|_: &ComponentValueChangeDebug| {
        Icon::new(icons::ARROW_RIGHT)
          .size(14.0)
          .color(PaletteColor::MutedForeground)
          .into()
      }),
    TableColumn::new("new", "New").cell(|change: &ComponentValueChangeDebug| value(Some(&change.to_value), 13.0)),
  ];
  let empty = match (kind, recording) {
    (ReactiveKind::Effect, _) => "An effect holds no value: it has no changes to show",
    (_, true) => "No changes yet",
    (_, false) => "No changes in what was recorded",
  };
  kit::flat_table(Table::new(columns, changes).label("Change history").empty_text(empty)).into()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::app::devtools::snapshot::DevToolsNodeKind;

  fn change(sequence: u64) -> ComponentValueChangeDebug {
    ComponentValueChangeDebug {
      sequence,
      timestamp: format!("09:41:{sequence:02}.000"),
      from_value: (sequence - 1).to_string(),
      to_value: sequence.to_string(),
    }
  }

  fn signal(id: usize, value: &str, changes: Vec<ComponentValueChangeDebug>) -> Reactive {
    Reactive {
      key: ReactiveKey {
        kind: ReactiveKind::Signal,
        id,
      },
      value: Some(value.to_owned()),
      type_name: "Signal<i32>".to_owned(),
      subscribers: 1,
      owner: "Counter".to_owned(),
      changes,
    }
  }

  fn sequences(recording: &Recording, reactive: &Reactive) -> Vec<u64> {
    recording.shown(reactive).iter().map(|change| change.sequence).collect()
  }

  #[test]
  fn the_signals_of_a_window_are_listed_with_the_component_they_were_made_in() {
    let mut ctx = Ctx::new_root();
    let count = ctx.signal(0_i32);
    count.set(1);
    let ids = crate::core::IdGenerator::new();
    let mut counter = DevToolsNode::for_test(ids.next(), "demo::Counter", DevToolsNodeKind::Component, vec![]);
    counter.signals = ctx.signals_debug();
    let snapshot = DevToolsSnapshot {
      root: Some(DevToolsNode::for_test(
        ids.next(),
        "demo::App",
        DevToolsNodeKind::Component,
        vec![counter],
      )),
      frame: Default::default(),
    };

    let all = reactives(&snapshot);

    assert_eq!(all.len(), 1);
    assert_eq!(all[0].key.kind, ReactiveKind::Signal);
    assert_eq!(all[0].key.id, count.id());
    assert_eq!(all[0].value.as_deref(), Some("1"));
    assert_eq!(all[0].type_name, "Signal<i32>");
    assert_eq!(all[0].owner, "Counter");
    // The engine's own record of the change, with its number.
    assert_eq!(all[0].changes.len(), 1);
    assert_eq!(
      (
        all[0].changes[0].from_value.as_str(),
        all[0].changes[0].to_value.as_str()
      ),
      ("0", "1")
    );
    assert_eq!(latest_change(&all), all[0].changes[0].sequence);
    assert!(all[0].changes[0].sequence > 0);
  }

  #[test]
  fn every_change_the_engine_has_is_shown_until_recording_is_stopped() {
    let recording = Recording::default();
    assert!(recording.on());
    let row = signal(1, "3", vec![change(1), change(2), change(3)]);
    assert_eq!(sequences(&recording, &row), [3, 2, 1], "the newest first");
  }

  #[test]
  fn what_changes_while_recording_is_stopped_is_not_shown() {
    let mut recording = Recording::default();
    // Stopped after the second change, started again after the fourth.
    recording.toggle(2);
    assert!(!recording.on());
    let row = signal(1, "5", (1..=5).map(change).collect());
    assert_eq!(sequences(&recording, &row), [2, 1]);

    recording.toggle(4);
    assert!(recording.on());
    assert_eq!(
      sequences(&recording, &row),
      [5, 2, 1],
      "the third and the fourth happened in between"
    );
  }

  #[test]
  fn clear_forgets_what_was_shown_and_goes_on_as_it_was() {
    let row = signal(1, "5", (1..=5).map(change).collect());

    let mut recording = Recording::default();
    recording.clear(3);
    assert!(recording.on());
    assert_eq!(sequences(&recording, &row), [5, 4]);

    let mut stopped = Recording::default();
    stopped.toggle(3);
    stopped.clear(3);
    assert!(!stopped.on());
    assert!(
      sequences(&stopped, &row).is_empty(),
      "stopped, it shows nothing until it records again"
    );
    stopped.toggle(4);
    assert_eq!(sequences(&stopped, &row), [5]);
  }

  #[test]
  fn the_filter_looks_in_the_number_the_value_the_type_and_the_owner() {
    let row = signal(12, "Idle", Vec::new());
    assert!(passes(&row, ""));
    assert!(passes(&row, "#12"));
    assert!(passes(&row, "idle"));
    assert!(passes(&row, "signal<i32>"));
    assert!(passes(&row, "counter i32"), "every word, wherever each is");
    assert!(!passes(&row, "counter memo"));
  }
}
