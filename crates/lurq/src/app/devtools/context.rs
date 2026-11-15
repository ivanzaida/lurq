//! The Context tab: every context a component of the inspected window provides, in a list, and beside it the one
//! selected in the list: what provides it, what reads it, and how what it holds changed. Built from the board
//! (`pencil/lurq-devtools.pen`, Context · dark) with the design system's components.
//!
//! What a context holds and its changes are the engine's record (`ComponentContextDebug`): the value as its
//! type writes itself for DevTools, and each change of it with the moment it happened.

use std::sync::Arc;

use super::{
  changes::{self, GraphNode, ListState, Recording},
  kit::{self, FILL, counted, line, mono, sans, short_tag, short_type_name},
  snapshot::{DevToolsNode, DevToolsSnapshot},
};
use crate::{
  app::{
    component::ComponentInfo,
    ctx::{ComponentContextChangeDebug, ComponentContextKind, Ctx},
    theme::PaletteColor,
  },
  components::{Column, Row},
  core::{NodeId, Signal},
  layout::{Alignment, text_style::FontWeight},
  node::Element,
  ui::{
    EmptyState, EmptyStateSize, Icon, Length, ScrollArea, Splitter, SplitterPane, Table, TableColumn, TableCount, icons,
  },
};

/// The sizes the board gives the tab.
const VALUE_WIDTH: f32 = 102.0;
const USES_WIDTH: f32 = 38.0;
const PROVIDER_WIDTH: f32 = 102.0;
const LINE_HEIGHT: f32 = 20.0;
const FIELD_INDENT: f32 = 16.0;
const FIELD_NAME_WIDTH: f32 = 104.0;

/// What a provided context is told apart by: the node of the component that provides it, and its type. Two
/// components may provide the same type, each to what is under it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, crate::DevtoolsInspectable)]
pub(crate) struct ContextKey {
  provider: NodeId,
  type_name: String,
}

/// A context a component of the inspected window provides.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Provided {
  key: ContextKey,
  /// Its type without the modules of anything in it, and with them.
  name: String,
  path: String,
  /// It was made with `create_context`: its value is set, and what reads it renders again.
  reactive: bool,
  /// What it holds, as its type writes itself: `None` where the type writes nothing.
  value: Option<Vec<ComponentInfo>>,
  /// How many components read it.
  uses: usize,
  /// The component that provides it.
  provider: String,
  /// Its changes as the engine recorded them, the oldest first.
  changes: Vec<ComponentContextChangeDebug>,
}

impl Provided {
  /// The type of what it holds: of a reactive context, the type inside it.
  fn held(&self) -> &str {
    held_type(&self.name)
  }

  /// What it holds on one line, or `None` where that is not known.
  fn written(&self) -> Option<String> {
    self.value.as_deref().and_then(|fields| one_line(self.held(), fields))
  }

  /// What the filter looks in: its type, its value and its provider.
  fn said(&self) -> String {
    format!("{} {} {}", self.name, self.written().unwrap_or_default(), self.provider)
  }

  /// Its changes that are shown, the newest first.
  fn shown(&self, recording: &Recording) -> Vec<ComponentContextChangeDebug> {
    self
      .changes
      .iter()
      .rev()
      .filter(|change| recording.shows(change.sequence))
      .cloned()
      .collect()
  }
}

/// The type a context holds: `ReactiveContext<Locale>` holds a `Locale`, a plain context is what it holds.
pub(crate) fn held_type(name: &str) -> &str {
  name
    .strip_prefix("ReactiveContext<")
    .and_then(|inner| inner.strip_suffix('>'))
    .unwrap_or(name)
}

/// A value on one line, from what its type wrote of it: the value itself where it is one, or its type and its
/// fields between braces. `None` for a type that wrote nothing.
pub(crate) fn one_line(held: &str, fields: &[ComponentInfo]) -> Option<String> {
  fn field(info: &ComponentInfo) -> String {
    match (info.formatted_value(), info.children()) {
      (Some(value), _) => format!("{}: {value}", info.name()),
      (None, []) => format!("{}: {}", info.name(), info.type_name()),
      (None, children) => format!("{}: {{ {} }}", info.name(), fields_of(children)),
    }
  }
  fn fields_of(fields: &[ComponentInfo]) -> String {
    fields.iter().map(field).collect::<Vec<_>>().join(", ")
  }
  match fields {
    [] => None,
    [only] if is_whole(only) => only.formatted_value().map(str::to_owned),
    fields => Some(format!("{held} {{ {} }}", fields_of(fields))),
  }
}

/// Whether what a type wrote is its whole value in one piece (a number, a tuple struct as it is debugged, the
/// variant of an enum) and not one of its fields.
fn is_whole(info: &ComponentInfo) -> bool {
  matches!(info.name(), "value" | "variant") && info.formatted_value().is_some()
}

/// The contexts of the inspected window, in the order of its tree, each with how many components read it. A
/// component reads the nearest one of the type that something it is in provides, or its own where nothing does.
pub(crate) fn contexts(snapshot: &DevToolsSnapshot) -> Vec<Provided> {
  fn collect<'a>(node: &'a DevToolsNode, scope: &mut Vec<(&'a str, usize)>, all: &mut Vec<Provided>) {
    let inherited = scope.len();
    for context in node
      .contexts
      .iter()
      .filter(|context| context.kind == ComponentContextKind::Provided)
    {
      scope.push((&context.type_name, all.len()));
      all.push(Provided {
        key: ContextKey {
          provider: node.id,
          type_name: context.type_name.to_string(),
        },
        name: short_type_name(&context.type_name),
        path: context.type_name.to_string(),
        reactive: context.reactive,
        value: context.value().filter(|fields| !fields.is_empty()),
        uses: 0,
        provider: short_tag(&node.tag).to_owned(),
        changes: context.history(),
      });
    }
    for context in node
      .contexts
      .iter()
      .filter(|context| context.kind == ComponentContextKind::Consumed)
    {
      let of_type = |(type_name, _): &&(&str, usize)| *type_name == &*context.type_name;
      let read = scope[..inherited]
        .iter()
        .rev()
        .find(of_type)
        .or_else(|| scope[inherited..].iter().find(of_type));
      if let Some((_, at)) = read {
        all[*at].uses += 1;
      }
    }
    for child in &node.children {
      collect(child, scope, all);
    }
    scope.truncate(inherited);
  }
  let mut all = Vec::new();
  if let Some(root) = &snapshot.root {
    collect(root, &mut Vec::new(), &mut all);
  }
  all
}

/// The number of the last change the engine recorded of any of them.
pub(crate) fn latest_change(all: &[Provided]) -> u64 {
  all
    .iter()
    .filter_map(|context| context.changes.last())
    .map(|change| change.sequence)
    .max()
    .unwrap_or(0)
}

/// What the tab keeps between renders.
#[derive(Clone)]
pub(crate) struct ContextState {
  pub(crate) list: ListState,
  /// The row selected in the list: the first one until a person selects another.
  selected: Signal<Option<ContextKey>>,
}

impl ContextState {
  pub(crate) fn new(ctx: &mut Ctx) -> Self {
    Self {
      list: ListState::new(ctx),
      selected: ctx.signal(None),
    }
  }
}

/// The tab: the list and what is known of the row selected in it, with a divider between them that a person drags.
pub(super) fn view(ctx: &mut Ctx, snapshot: &DevToolsSnapshot, state: &ContextState) -> Element {
  state.list.filter_key.answer(ctx);
  let all = contexts(snapshot);
  let latest = latest_change(&all);
  let recording = state.list.recording.get();
  let filter = state.list.filter.get();
  let listed: Vec<Provided> = all
    .iter()
    .filter(|context| changes::passes(&context.said(), &filter))
    .cloned()
    .collect();
  // The row a person selected, while the inspected window still has it; else the first of the list.
  let selected = state
    .selected
    .get()
    .and_then(|key| all.iter().find(|context| context.key == key))
    .or(listed.first())
    .cloned();

  let detail = detail(selected.as_ref(), &recording);
  let list = Column::new()
    .size(FILL, FILL)
    .child(changes::toolbar(&state.list, latest, "contexts"))
    .child(Column::new().width(FILL).flex(1.0).border_top(line()).child(list(
      ctx,
      listed,
      selected.as_ref().map(|context| context.key.clone()),
      !filter.trim().is_empty(),
      state,
    )));
  let panes = Splitter::new()
    .id("devtools-contexts")
    .pane(
      SplitterPane::new(list)
        .length(changes::LIST_WIDTH)
        .min(changes::LIST_MIN_WIDTH)
        .label("Contexts"),
    )
    .pane(
      SplitterPane::new(detail)
        .min(changes::DETAIL_MIN_WIDTH)
        .label("Selected context"),
    )
    .mount(ctx);
  Row::new().size(FILL, FILL).flex(1.0).child(panes).into()
}

fn list(
  ctx: &mut Ctx,
  listed: Vec<Provided>,
  selected: Option<ContextKey>,
  filtering: bool,
  state: &ContextState,
) -> Element {
  let reactive = listed.iter().filter(|context| context.reactive).count();
  let plain = listed.len() - reactive;
  let select = state.selected.clone();
  let columns = vec![
    TableColumn::new("context", "Context").cell(|context: &Provided| {
      Row::new()
        .width(FILL)
        .spacing(6.0)
        .align_items(Alignment::Center)
        .child(
          Icon::new(if context.reactive {
            icons::BOLT
          } else {
            icons::SHARE_NODES
          })
          .size(14.0)
          .color(PaletteColor::MutedForeground),
        )
        .child(changes::cut(mono(&context.name, 12.0, FontWeight::Normal)).flex(1.0))
        .into()
    }),
    TableColumn::new("value", "Value")
      .width(VALUE_WIDTH)
      .cell(|context: &Provided| match context.written() {
        Some(value) => changes::cut(mono(&value, 12.0, FontWeight::Medium)).into(),
        None => mono("<unknown>", 12.0, FontWeight::Normal)
          .color(PaletteColor::MutedForeground)
          .into(),
      }),
    TableColumn::new("uses", "Uses")
      .width(USES_WIDTH)
      .text(|context: &Provided| context.uses.to_string())
      .numeric(),
    TableColumn::new("provider", "Provider")
      .width(PROVIDER_WIDTH)
      .text(|context: &Provided| context.provider.clone())
      .mono(true),
  ];
  let table = Table::new_shared(columns, Arc::new(listed))
    .row_key(|context: &Provided| context.key.clone())
    .label("Contexts")
    .id("devtools-contexts-list")
    .checkboxes(false)
    .selected_controlled(selected, move |keys: Vec<ContextKey>| {
      // A press with Ctrl on the selected row would leave none selected: the detail always shows one.
      if let Some(key) = keys.into_iter().next_back() {
        select.set(Some(key));
      }
    })
    .count(true)
    .count_text(move |count: TableCount| {
      format!(
        "{} · {reactive} reactive · {plain} plain",
        counted(count.total, "context")
      )
    })
    .empty_text(if filtering {
      "Nothing matches the filter"
    } else {
      "No component of the inspected window provides a context"
    });
  kit::dense_table(table).mount(ctx)
}

/// What is known of the selected row: what it is, where it stands between its provider and what reads it,
/// and its changes.
fn detail(selected: Option<&Provided>, recording: &Recording) -> Element {
  let Some(context) = selected else {
    return changes::nothing_selected(
      "Select a context to see what provides it, what reads it and how it changed.",
      icons::SHARE_NODES,
    );
  };
  let shown = context.shown(recording);
  let (written, uses) = (
    context.written().unwrap_or_else(|| "<unknown>".to_owned()),
    context.uses.to_string(),
  );
  Column::new()
    .size(FILL, FILL)
    .child(changes::header(&context.name, "Context", &context.path))
    .child(changes::graph(
      GraphNode {
        title: &context.provider,
        code: true,
        caption: "Component",
        caption_code: false,
        number: None,
        selected: false,
        share: 1.0,
      },
      "provides",
      // What it holds is long: it has the room of the middle.
      GraphNode {
        title: &context.name,
        code: true,
        caption: &written,
        caption_code: true,
        number: None,
        selected: true,
        share: 1.6,
      },
      "notifies",
      GraphNode {
        title: "consumers",
        code: false,
        caption: "tracked reads",
        caption_code: false,
        number: Some(&uses),
        selected: false,
        share: 0.9,
      },
    ))
    .child(changes::history_title(shown.len()))
    .child(history(context, shown, recording.on()))
    .into()
}

/// The changes shown, the newest first, under the labels of their columns: when each happened, what the context
/// held and what it came to hold, each written out field by field.
fn history(context: &Provided, shown: Vec<ComponentContextChangeDebug>, recording: bool) -> Column {
  let label = |text: &str| sans(text, 12.0, FontWeight::Medium).color(PaletteColor::MutedForeground);
  let labels = Row::new()
    .width(FILL)
    .height(kit::LABELS_HEIGHT)
    .padding_horizontal(changes::PADDING)
    .align_items(Alignment::Center)
    .border_bottom(line())
    .child(Row::new().width(changes::TIME_WIDTH).child(label("Time")))
    // The arrow stands in the room after the value it points from.
    .child(
      Row::new()
        .flex(1.0)
        .padding_right(changes::ARROW_WIDTH)
        .child(label("Previous")),
    )
    .child(Row::new().flex(1.0).child(label("New")));
  let body: Element = if shown.is_empty() {
    changes::centred(
      EmptyState::new(if recording {
        "No changes yet"
      } else {
        "No changes in what was recorded"
      })
      .icon(icons::INBOX)
      // As a table says it has no rows: the list of a signal's changes is one.
      .size(EmptyStateSize::Sm)
      .into(),
    )
  } else {
    let mut rows = Column::new().width(FILL);
    for change in &shown {
      rows = rows.child(
        Row::new()
          .width(FILL)
          .padding_horizontal(changes::PADDING)
          .padding_vertical(9.0)
          .align_items(Alignment::Start)
          .border_bottom(line())
          .child(
            Row::new()
              .width(changes::TIME_WIDTH)
              .height(LINE_HEIGHT)
              .align_items(Alignment::Center)
              .child(mono(&change.timestamp, 13.0, FontWeight::Normal).color(PaletteColor::MutedForeground)),
          )
          .child(written_out(context.held(), &change.from_value, false).flex(1.0))
          .child(
            Row::new()
              .width(changes::ARROW_WIDTH)
              .height(LINE_HEIGHT)
              .align_items(Alignment::Center)
              .child(changes::arrow()),
          )
          .child(written_out(context.held(), &change.to_value, true).flex(1.0)),
      );
    }
    ScrollArea::new()
      .label("Change history")
      .height(Length::Fill)
      .child(rows)
      .into()
  };
  Column::new()
    .width(FILL)
    .flex(1.0)
    .child(labels)
    .child(Column::new().width(FILL).flex(1.0).child(body))
}

/// A value written out: on one line where it is one piece, or its type over a line for each of its fields. The
/// value a context came to hold stands out from the one it held before.
fn written_out(held: &str, fields: &[ComponentInfo], new: bool) -> Column {
  let quiet = |text: &str| mono(text, 13.0, FontWeight::Normal).color(PaletteColor::MutedForeground);
  let value = |text: &str| {
    if new {
      mono(text, 13.0, FontWeight::Medium)
    } else {
      quiet(text)
    }
  };
  let name = |text: &str| {
    if new {
      mono(text, 13.0, FontWeight::Normal)
    } else {
      quiet(text)
    }
  };
  let line = |what: Row| what.height(LINE_HEIGHT).align_items(Alignment::Center);
  let mut written = Column::new();
  match fields {
    [] => written = written.child(line(Row::new().child(quiet("<unknown>")))),
    [only] if is_whole(only) => {
      written = written.child(line(
        Row::new().child(value(only.formatted_value().unwrap_or_default())),
      ));
    }
    fields => {
      written = written.child(line(Row::new().child(value(held))));
      let mut rows = Vec::new();
      field_rows(fields, 1, &mut rows);
      for (depth, field, held) in rows {
        written = written.child(line(
          Row::new()
            .padding_left(depth as f32 * FIELD_INDENT)
            .child(Row::new().width(FIELD_NAME_WIDTH).child(name(&field)))
            .child(value(&held)),
        ));
      }
    }
  }
  written
}

/// The lines of a value's fields: how deep each is, its name, and its value or, of a field with fields of its
/// own, its type over theirs.
fn field_rows(fields: &[ComponentInfo], depth: usize, rows: &mut Vec<(usize, String, String)>) {
  for field in fields {
    let held = field
      .formatted_value()
      .map_or_else(|| short_type_name(field.type_name()), str::to_owned);
    rows.push((depth, field.name().to_owned(), held));
    field_rows(field.children(), depth + 1, rows);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::app::devtools::snapshot::DevToolsNodeKind;

  #[derive(Clone, Hash, crate::DevtoolsInspectable)]
  struct Locale {
    locale: &'static str,
    greeting: &'static str,
  }

  #[derive(Clone, Debug)]
  struct Depth(usize);

  impl crate::app::component::DevtoolsInspectable for Depth {
    fn inspect(&self, formatter: &mut crate::app::component::DevtoolsFormatter<'_>) {
      formatter.value_debug(self);
    }
  }

  #[derive(Clone)]
  struct Opaque;

  impl crate::app::component::DevtoolsInspectable for Opaque {}

  fn node(ids: &crate::core::IdGenerator, tag: &str, ctx: &Ctx, children: Vec<DevToolsNode>) -> DevToolsNode {
    let mut node = DevToolsNode::for_test(ids.next(), tag, DevToolsNodeKind::Component, children);
    node.contexts = ctx.contexts_debug();
    node
  }

  fn snapshot(root: DevToolsNode) -> DevToolsSnapshot {
    DevToolsSnapshot {
      root: Some(root),
      frame: Default::default(),
    }
  }

  #[test]
  fn a_context_is_listed_with_what_it_holds_its_provider_and_how_many_read_it() {
    let ids = crate::core::IdGenerator::new();
    let mut provider = Ctx::new_root();
    provider.provide(Depth(0));
    provider.provide(Opaque);
    let mut reader = Ctx::new_root();
    let _ = reader.use_context::<Depth>();
    let mut other = Ctx::new_root();
    let _ = other.use_context::<Depth>();

    let all = contexts(&snapshot(node(
      &ids,
      "app::Router",
      &provider,
      vec![node(&ids, "Page", &reader, vec![]), node(&ids, "Side", &other, vec![])],
    )));

    assert_eq!(all.len(), 2);
    assert_eq!(all[0].name, "Depth");
    assert!(all[0].path.ends_with("::Depth"));
    assert_eq!(all[0].written().as_deref(), Some("Depth(0)"));
    assert_eq!(all[0].uses, 2);
    assert_eq!(all[0].provider, "Router");
    assert!(!all[0].reactive);
    // A type that writes nothing of itself is listed, and what it holds is not known.
    assert_eq!(all[1].name, "Opaque");
    assert_eq!(all[1].written(), None);
    assert_eq!(all[1].uses, 0);
  }

  #[test]
  fn a_component_reads_the_nearest_context_of_the_type_above_it() {
    let ids = crate::core::IdGenerator::new();
    let mut router = Ctx::new_root();
    router.provide(Depth(0));
    // An outlet reads the depth it is at and provides the next to what is in it.
    let mut outlet = Ctx::new_root();
    let _ = outlet.use_context::<Depth>();
    outlet.provide(Depth(1));
    let mut page = Ctx::new_root();
    let _ = page.use_context::<Depth>();

    let all = contexts(&snapshot(node(
      &ids,
      "Router",
      &router,
      vec![node(&ids, "Outlet", &outlet, vec![node(&ids, "Page", &page, vec![])])],
    )));

    let said = |context: &Provided| (context.provider.clone(), context.written(), context.uses);
    assert_eq!(said(&all[0]), ("Router".to_owned(), Some("Depth(0)".to_owned()), 1));
    assert_eq!(said(&all[1]), ("Outlet".to_owned(), Some("Depth(1)".to_owned()), 1));
    assert_ne!(
      all[0].key, all[1].key,
      "the same type from two providers is two contexts"
    );
  }

  #[test]
  fn a_reactive_context_records_each_value_it_is_set_to() {
    let ids = crate::core::IdGenerator::new();
    let mut provider = Ctx::new_root();
    let locale = provider.create_context(Locale {
      locale: "en-US",
      greeting: "Hello!",
    });
    locale.set(Locale {
      locale: "en-GB",
      greeting: "Hello there!",
    });
    // The same value again is no change.
    locale.set(Locale {
      locale: "en-GB",
      greeting: "Hello there!",
    });

    let all = contexts(&snapshot(node(&ids, "LocaleProvider", &provider, vec![])));

    assert_eq!(all.len(), 1);
    let context = &all[0];
    assert!(context.reactive);
    assert_eq!(context.name, "ReactiveContext<Locale>");
    assert_eq!(context.held(), "Locale");
    assert_eq!(
      context.written().as_deref(),
      Some("Locale { locale: \"en-GB\", greeting: \"Hello there!\" }")
    );
    assert_eq!(context.changes.len(), 1);
    let change = &context.changes[0];
    assert_eq!(
      one_line("Locale", &change.from_value).as_deref(),
      Some("Locale { locale: \"en-US\", greeting: \"Hello!\" }")
    );
    assert_eq!(change.to_value, context.value.clone().unwrap());
    assert_eq!(latest_change(&all), change.sequence);
    assert!(Recording::default().shows(change.sequence));

    // Written out, a line for each field under its type.
    let mut rows = Vec::new();
    field_rows(&change.to_value, 1, &mut rows);
    assert_eq!(
      rows,
      [
        (1, "locale".to_owned(), "\"en-GB\"".to_owned()),
        (1, "greeting".to_owned(), "\"Hello there!\"".to_owned()),
      ]
    );
  }

  #[test]
  fn a_plain_context_provided_with_another_value_records_the_change() {
    let ids = crate::core::IdGenerator::new();
    let mut provider = Ctx::new_root();
    provider.provide(Depth(0));
    provider.provide(Depth(0));
    provider.provide(Depth(2));

    let all = contexts(&snapshot(node(&ids, "Router", &provider, vec![])));

    assert_eq!(all[0].written().as_deref(), Some("Depth(2)"));
    assert_eq!(all[0].changes.len(), 1, "the same value provided again is no change");
    assert_eq!(
      one_line("Depth", &all[0].changes[0].from_value).as_deref(),
      Some("Depth(0)")
    );
  }

  #[test]
  fn the_type_a_context_holds_is_the_one_inside_a_reactive_context() {
    assert_eq!(held_type("ReactiveContext<LocaleContext>"), "LocaleContext");
    assert_eq!(held_type("OutletDepth"), "OutletDepth");
    assert_eq!(held_type("ReactiveContext<Option<u8>>"), "Option<u8>");
  }
}
