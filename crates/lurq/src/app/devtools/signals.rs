//! The Signals tab: every signal, memo and effect of the inspected window in a list, and beside it the one
//! selected in the list: what owns it, what it notifies, and how it changed. Built from the board
//! (`pencil/lurq-devtools.pen`, Signals · dark) with the design system's components.
//!
//! The changes are the engine's own record of them (`ComponentValueChangeDebug`), every one of the last 64 of each
//! signal and memo with the moment it happened. The tab records nothing itself: recording is which of them it shows.

use std::sync::Arc;

use super::{
  changes::{self, GraphNode, ListState, Recording},
  kit::{self, FILL, counted, line, mono, short_tag, short_type_name},
  snapshot::{DevToolsNode, DevToolsSnapshot},
};
use crate::{
  app::{
    ctx::{ComponentValueChangeDebug, Ctx},
    theme::PaletteColor,
  },
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, text_style::FontWeight},
  node::Element,
  ui::{Icon, Splitter, SplitterPane, Table, TableColumn, TableCount, icons},
};

/// The widths the board gives the list's columns.
const ID_WIDTH: f32 = 60.0;
const VALUE_WIDTH: f32 = 88.0;
const TYPE_WIDTH: f32 = 152.0;
const SUBS_WIDTH: f32 = 40.0;

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

impl Reactive {
  /// What the filter looks in: its number, its value, its type and its owner.
  fn said(&self) -> String {
    format!(
      "#{} {} {} {}",
      self.key.id,
      self.value.as_deref().unwrap_or(""),
      self.type_name,
      self.owner
    )
  }

  /// Its changes that are shown, the newest first.
  fn shown(&self, recording: &Recording) -> Vec<ComponentValueChangeDebug> {
    self
      .changes
      .iter()
      .rev()
      .filter(|change| recording.shows(change.sequence))
      .cloned()
      .collect()
  }
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

/// What the tab keeps between renders.
#[derive(Clone)]
pub(crate) struct SignalsState {
  pub(crate) list: ListState,
  /// The row selected in the list: the first one until a person selects another.
  selected: Signal<Option<ReactiveKey>>,
}

impl SignalsState {
  pub(crate) fn new(ctx: &mut Ctx) -> Self {
    Self {
      list: ListState::new(ctx),
      selected: ctx.signal(None),
    }
  }
}

/// The tab: the list and what is known of the row selected in it, with a divider between them that a person drags.
pub(super) fn view(ctx: &mut Ctx, snapshot: &DevToolsSnapshot, state: &SignalsState) -> Element {
  state.list.filter_key.answer(ctx);
  let all = reactives(snapshot);
  let latest = latest_change(&all);
  let recording = state.list.recording.get();
  let filter = state.list.filter.get();
  let listed: Vec<Reactive> = all
    .iter()
    .filter(|reactive| changes::passes(&reactive.said(), &filter))
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
    .child(changes::toolbar(&state.list, latest, "signals"))
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
        .length(changes::LIST_WIDTH)
        .min(changes::LIST_MIN_WIDTH)
        .label("Signals"),
    )
    .pane(
      SplitterPane::new(detail)
        .min(changes::DETAIL_MIN_WIDTH)
        .label("Selected signal"),
    )
    .mount(ctx);
  Row::new().size(FILL, FILL).flex(1.0).child(panes).into()
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
    return changes::nothing_selected(
      "Select a signal, a memo or an effect to see what owns it and how it changed.",
      icons::BOLT,
    );
  };
  let shown = reactive.shown(recording);
  let (number, subscribers) = (format!("#{}", reactive.key.id), reactive.subscribers.to_string());
  Column::new()
    .size(FILL, FILL)
    .child(changes::header(&number, reactive.key.kind.label(), &reactive.type_name))
    .child(changes::graph(
      GraphNode {
        title: &reactive.owner,
        code: true,
        caption: "Component",
        caption_code: false,
        number: None,
        selected: false,
        share: 1.0,
      },
      "owns",
      GraphNode {
        title: &number,
        code: true,
        caption: &reactive.type_name,
        caption_code: true,
        number: reactive.value.as_deref(),
        selected: true,
        share: 1.0,
      },
      "notifies",
      GraphNode {
        title: "subscribers",
        code: false,
        caption: "tracked reads",
        caption_code: false,
        number: Some(&subscribers),
        selected: false,
        share: 1.0,
      },
    ))
    .child(changes::history_title(shown.len()))
    .child(
      Column::new()
        .width(FILL)
        .flex(1.0)
        .child(history(shown, reactive.key.kind, recording.on())),
    )
    .into()
}

/// The changes shown, the newest first: when each happened, what the value was and what it became.
fn history(shown: Vec<ComponentValueChangeDebug>, kind: ReactiveKind, recording: bool) -> Element {
  let quiet = |text: &str| -> Element {
    mono(text, 13.0, FontWeight::Normal)
      .color(PaletteColor::MutedForeground)
      .into()
  };
  let columns = vec![
    TableColumn::new("time", "Time")
      .width(changes::TIME_WIDTH)
      .cell(move |change: &ComponentValueChangeDebug| quiet(&change.timestamp)),
    TableColumn::new("previous", "Previous").cell(move |change: &ComponentValueChangeDebug| quiet(&change.from_value)),
    TableColumn::new("arrow", "")
      .width(changes::ARROW_WIDTH)
      .cell(|_: &ComponentValueChangeDebug| changes::arrow().into()),
    TableColumn::new("new", "New").cell(|change: &ComponentValueChangeDebug| value(Some(&change.to_value), 13.0)),
  ];
  let empty = match (kind, recording) {
    (ReactiveKind::Effect, _) => "An effect holds no value: it has no changes to show",
    (_, true) => "No changes yet",
    (_, false) => "No changes in what was recorded",
  };
  kit::flat_table(Table::new(columns, shown).label("Change history").empty_text(empty)).into()
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
  fn the_changes_shown_are_those_of_the_recording_the_newest_first() {
    let row = signal(1, "5", (1..=5).map(change).collect());
    let sequences = |recording: &Recording| {
      row
        .shown(recording)
        .iter()
        .map(|change| change.sequence)
        .collect::<Vec<_>>()
    };
    let mut recording = Recording::default();
    assert_eq!(sequences(&recording), [5, 4, 3, 2, 1]);
    recording.toggle(2);
    assert_eq!(sequences(&recording), [2, 1]);
  }

  #[test]
  fn the_filter_looks_in_the_number_the_value_the_type_and_the_owner() {
    let row = signal(12, "Idle", Vec::new());
    assert_eq!(row.said(), "#12 Idle Signal<i32> Counter");
    assert!(changes::passes(&row.said(), "counter i32"));
    assert!(!changes::passes(&row.said(), "counter memo"));
  }
}
