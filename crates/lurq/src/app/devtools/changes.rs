//! What the Signals and Context tabs share: both list things whose values change, and show the one selected
//! between what makes it and what it notifies, over its changes. The recording of the changes, the toolbar over
//! the list, and the header, the graph and the title of the history beside it.

use super::kit::{self, FILL, KeyFocus, line, mono, sans};
use crate::{
  app::{
    ctx::Ctx,
    theme::{BorderSize, PaletteColor, RadiusSize},
  },
  components::{Column, Row, Spacer, Text, TextOverflow},
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, border::Border},
  ui::{
    Badge, BadgeSize, BadgeVariant, Button, ButtonSize, ButtonVariant, EmptyState, Icon, Recipe, SearchField,
    SearchFieldSize, Toggle, TogglePart, ToggleSize, ToggleState, icons,
  },
};

/// The sizes the board gives both tabs (`pencil/lurq-devtools.pen`, Signals · dark and Context · dark).
pub(super) const LIST_WIDTH: f32 = 540.0;
pub(super) const LIST_MIN_WIDTH: f32 = 360.0;
pub(super) const DETAIL_MIN_WIDTH: f32 = 480.0;
pub(super) const PADDING: f32 = kit::DETAIL_PADDING;
pub(super) const TIME_WIDTH: f32 = 148.0;
pub(super) const ARROW_WIDTH: f32 = 48.0;
const HEADER_HEIGHT: f32 = 48.0;
const NODE_HEIGHT: f32 = 64.0;
const RELATION_WIDTH: f32 = 103.0;

/// Which of the engine's changes a tab shows: those of the stretches it was recording in, by the numbers the
/// engine gives the changes. It records while its last stretch has no end.
///
/// The engine keeps the last changes of every value whatever a tab does; recording is what is shown of them.
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

  /// Whether the change the engine numbered `sequence` is one of those shown.
  pub(crate) fn shows(&self, sequence: u64) -> bool {
    self
      .stretches
      .iter()
      .any(|stretch| sequence > stretch.after && stretch.until.is_none_or(|until| sequence <= until))
  }
}

/// What a tab keeps of its list between renders, besides which row is selected.
#[derive(Clone)]
pub(crate) struct ListState {
  pub(crate) recording: Signal<Recording>,
  pub(crate) filter: Signal<String>,
  /// The filter's box and the key that goes to it.
  pub(crate) filter_key: KeyFocus,
}

impl ListState {
  pub(crate) fn new(ctx: &mut Ctx) -> Self {
    Self {
      recording: ctx.signal(Recording::default()),
      filter: ctx.signal(String::new()),
      filter_key: KeyFocus::new(ctx),
    }
  }
}

/// Whether a row is one the filter asks for: every word typed is somewhere in what the row says.
pub(crate) fn passes(row: &str, filter: &str) -> bool {
  let row = row.to_lowercase();
  filter.split_whitespace().all(|word| row.contains(&word.to_lowercase()))
}

/// The toolbar over a list: what records, what clears, and the filter. `things` is what the list holds, in the
/// plural and in lower case: it names the controls and their ids. `latest` is the number of the last change
/// the engine recorded: the moment a press of Recording or Clear takes effect from.
pub(super) fn toolbar(state: &ListState, latest: u64, things: &str) -> Row {
  let on = state.recording.get().on();
  let (toggled, cleared) = (state.recording.clone(), state.recording.clone());
  kit::toolbar()
    .child(
      Toggle::new(if on { "Recording..." } else { "Record" })
        .leading(kit::record_mark(false))
        .size(ToggleSize::Xs)
        .rules(&recording_toggle())
        .on(on)
        .on_change(move |_| toggled.update(|recording| recording.toggle(latest)))
        .id(format!("devtools-{things}-record")),
    )
    .child(
      Button::new("Clear")
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Xs)
        .icon(icons::TRASH)
        .on_click(move |_| cleared.update(|recording| recording.clear(latest)))
        .id(format!("devtools-{things}-clear")),
    )
    .child(
      Row::new().flex(1.0).child(
        SearchField::new(state.filter.clone())
          .placeholder(&format!("Filter {things}..."))
          .label(&format!("Filter {things}"))
          .shortcut(kit::FIND_KEY)
          .ref_element(state.filter_key.field.clone())
          .size(SearchFieldSize::Sm)
          .input_rules(&kit::compact_input())
          .full_width(true)
          .id(format!("devtools-{things}-filter")),
      ),
    )
}

/// The board's Recording: the toolbar's toggle, in the destructive color on its muted fill while it records.
pub(super) fn recording_toggle() -> Recipe {
  use PaletteColor::{Destructive, DestructiveMuted};
  kit::outlined_toggle().state(ToggleState::On, TogglePart::Root, |s| {
    s.background(DestructiveMuted)
      .foreground(Destructive)
      .border(Border::inside(0.0, DestructiveMuted))
  })
}

/// What stands beside a list with nothing selected in it.
pub(super) fn nothing_selected(description: &str, icon: &str) -> Element {
  centred(
    EmptyState::new("Nothing selected")
      .description(description)
      .icon(icon)
      .into(),
  )
}

/// Something in the middle of all the room it is given.
pub(super) fn centred(what: Element) -> Element {
  Column::new()
    .size(FILL, FILL)
    .padding(PADDING)
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(what)
    .into()
}

/// A text of one line that ends in an ellipsis where it has no room, in a column of all the room there is.
pub(super) fn cut(text: Text) -> Column {
  Column::new()
    .width(FILL)
    .child(text.width(FILL).text_overflow(TextOverflow::Elipsis))
}

/// The header of what is selected: what it is called, what kind of thing it is, and what else is said of it.
pub(super) fn header(title: &str, kind: &str, more: &str) -> Row {
  Row::new()
    .width(FILL)
    .height(HEADER_HEIGHT)
    .padding_left(PADDING)
    .padding_right(16.0)
    .spacing(10.0)
    .align_items(Alignment::Center)
    .border_bottom(line())
    .child(mono(title, 15.0, FontWeight::SemiBold))
    .child(Badge::new(kind).variant(BadgeVariant::Outline).size(BadgeSize::Sm))
    .child(cut(mono(more, 12.0, FontWeight::Normal).color(PaletteColor::MutedForeground)).flex(1.0))
}

/// A box of the graph: what it is called over a word of it, and a number at its end.
pub(super) struct GraphNode<'a> {
  pub(super) title: &'a str,
  /// The title is a name of the inspected app's, in the type of code.
  pub(super) code: bool,
  pub(super) caption: &'a str,
  pub(super) caption_code: bool,
  pub(super) number: Option<&'a str>,
  /// It is the one the graph is of: its edge is the stronger.
  pub(super) selected: bool,
  /// How much of the row it takes beside the others.
  pub(super) share: f32,
}

impl GraphNode<'_> {
  fn build(&self) -> Row {
    let title = if self.code {
      mono(self.title, 13.0, FontWeight::Medium)
    } else {
      sans(self.title, 13.0, FontWeight::Medium)
    };
    let caption = if self.caption_code {
      mono(self.caption, 11.0, FontWeight::Normal)
    } else {
      sans(self.caption, 11.0, FontWeight::Normal)
    };
    let edge = if self.selected {
      PaletteColor::Input
    } else {
      PaletteColor::Border
    };
    let mut node = Row::new()
      .height(NODE_HEIGHT)
      .flex(self.share)
      .padding_horizontal(16.0)
      .spacing(12.0)
      .align_items(Alignment::Center)
      .rounded(RadiusSize::Md)
      .background(PaletteColor::Background)
      .border(Border::inside(BorderSize::Sm, edge))
      .child(
        Column::new()
          .flex(1.0)
          .spacing(4.0)
          .child(cut(title))
          .child(cut(caption.color(PaletteColor::MutedForeground))),
      );
    if let Some(number) = self.number {
      node = node.child(mono(number, 15.0, FontWeight::SemiBold));
    }
    node
  }
}

/// The graph of what is selected: what makes it, itself, and what it notifies, with a word on each arrow.
pub(super) fn graph(from: GraphNode, makes: &str, selected: GraphNode, notifies: &str, to: GraphNode) -> Column {
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
        .child(from.build())
        .child(relation(makes))
        .child(selected.build())
        .child(relation(notifies))
        .child(to.build()),
    )
}

/// The title over the changes, with how many of them are shown.
pub(super) fn history_title(shown: usize) -> Row {
  let said = match shown {
    0 => "no changes".to_owned(),
    1 => "last change".to_owned(),
    count => format!("last {count} changes"),
  };
  Row::new()
    .width(FILL)
    .height(40.0)
    .padding_horizontal(PADDING)
    .spacing(8.0)
    .align_items(Alignment::Center)
    .border_bottom(line())
    .child(sans("Change history", 13.0, FontWeight::Medium))
    .child(Spacer::new().flex(1.0))
    .child(sans(&said, 12.0, FontWeight::Normal).color(PaletteColor::MutedForeground))
}

/// The arrow between what a value was and what it became.
pub(super) fn arrow() -> Icon {
  Icon::new(icons::ARROW_RIGHT)
    .size(14.0)
    .color(PaletteColor::MutedForeground)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn shown(recording: &Recording) -> Vec<u64> {
    (1..=5).rev().filter(|sequence| recording.shows(*sequence)).collect()
  }

  #[test]
  fn every_change_the_engine_has_is_shown_until_recording_is_stopped() {
    let recording = Recording::default();
    assert!(recording.on());
    assert_eq!(shown(&recording), [5, 4, 3, 2, 1]);
  }

  #[test]
  fn what_changes_while_recording_is_stopped_is_not_shown() {
    let mut recording = Recording::default();
    // Stopped after the second change, started again after the fourth.
    recording.toggle(2);
    assert!(!recording.on());
    assert_eq!(shown(&recording), [2, 1]);

    recording.toggle(4);
    assert!(recording.on());
    assert_eq!(
      shown(&recording),
      [5, 2, 1],
      "the third and the fourth happened in between"
    );
  }

  #[test]
  fn clear_forgets_what_was_shown_and_goes_on_as_it_was() {
    let mut recording = Recording::default();
    recording.clear(3);
    assert!(recording.on());
    assert_eq!(shown(&recording), [5, 4]);

    let mut stopped = Recording::default();
    stopped.toggle(3);
    stopped.clear(3);
    assert!(!stopped.on());
    assert!(
      shown(&stopped).is_empty(),
      "stopped, it shows nothing until it records again"
    );
    stopped.toggle(4);
    assert_eq!(shown(&stopped), [5]);
  }

  #[test]
  fn the_filter_asks_for_every_word_wherever_each_is() {
    let row = "#12 Idle Signal<i32> Counter";
    assert!(passes(row, ""));
    assert!(passes(row, "#12"));
    assert!(passes(row, "idle"));
    assert!(passes(row, "counter i32"));
    assert!(!passes(row, "counter memo"));
  }
}
