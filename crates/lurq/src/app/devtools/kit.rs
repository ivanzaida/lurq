//! What the DevTools' own layout is written with between the design system's components: text at the board's
//! sizes in the theme's families, and the theme's line. Nothing here names a color or a font of its own.

use std::{
  hash::Hash,
  sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
  },
};

use crate::{
  app::{
    ctx::Ctx,
    events::KeyboardEvent,
    theme::{BorderSize, Density, PaletteColor, TypographyStyle},
  },
  components::{Row, Text, TextOverflow},
  core::{ElementRef, Signal},
  layout::{Alignment, text_style::FontWeight},
  node::{border::Border, dimension::Dimension},
  ui::{InputPart, Length, Recipe, Table, TablePart, TogglePart, ToggleState},
};

pub(super) const FILL: Dimension = Dimension::Pct(100.0);

/// The sizes the board gives what every tab has (`pencil/lurq-devtools.pen`): the toolbar over a list, a row of a
/// dense list and its header, the labels over a list of details and the room at its sides.
pub(super) const TOOLBAR_HEIGHT: f32 = 44.0;
pub(super) const DENSE_ROW_HEIGHT: f32 = 26.0;
pub(super) const DENSE_HEADER_HEIGHT: f32 = 28.0;
pub(super) const LABELS_HEIGHT: f32 = 32.0;
pub(super) const DETAIL_PADDING: f32 = 20.0;

/// The key that goes to the search of a tab, as its box writes it.
pub(super) const FIND_KEY: &str = if cfg!(target_os = "macos") { "⌘F" } else { "Ctrl+F" };

/// Whether a key pressed is the one that goes to the search of a tab.
pub(super) fn is_find_key(event: &KeyboardEvent) -> bool {
  (event.ctrl || event.meta) && !event.alt && !event.shift && event.code == "KeyF"
}

/// The search box of a tab and the key of the window that goes to it: the focus goes to the box once for each
/// press. A handler of keys has no `Ctx` to focus with, so it counts the presses and the tab's render answers.
#[derive(Clone)]
pub(super) struct KeyFocus {
  /// The ref of the box: given to it, and focused.
  pub(super) field: ElementRef,
  asked: Signal<u64>,
  answered: Arc<AtomicU64>,
}

impl KeyFocus {
  pub(super) fn new(ctx: &mut Ctx) -> Self {
    Self {
      field: ElementRef::new(),
      asked: ctx.signal(0),
      answered: Arc::default(),
    }
  }

  /// One that is in no window: what a test acts on.
  #[cfg(test)]
  pub(super) fn detached() -> Self {
    Self {
      field: ElementRef::new(),
      asked: Signal::new(0),
      answered: Arc::default(),
    }
  }

  /// The key was pressed: the focus goes to the box at the next render.
  pub(super) fn ask(&self) {
    self.asked.update(|asked| *asked += 1);
  }

  /// Whether the key was pressed since this was last asked. It reads the count, so what asks is rendered again
  /// at the next press.
  pub(super) fn wanted(&self) -> bool {
    let asked = self.asked.get();
    self.answered.swap(asked, Ordering::Relaxed) != asked
  }

  /// Focuses the box if its key was pressed: called by the render that builds the box.
  pub(super) fn answer(&self, ctx: &Ctx) {
    if self.wanted() {
      ctx.focus(&self.field);
    }
  }
}

/// The row of controls over a list.
pub(super) fn toolbar() -> Row {
  Row::new()
    .width(FILL)
    .height(TOOLBAR_HEIGHT)
    .padding_horizontal(12.0)
    .spacing(8.0)
    .align_items(Alignment::Center)
}

/// The board's toggle of a toolbar: in an edge, on the window's fill, picked out while it is on.
pub(super) fn outlined_toggle() -> Recipe {
  use PaletteColor::{Background, Highlight, HighlightForeground, Input, MutedForeground};
  Recipe::new()
    .base(TogglePart::Root, |s| {
      s.background(Background)
        .border(Border::inside(BorderSize::Sm, Input))
        .foreground(MutedForeground)
        .padding_x(10.0)
        .gap(6.0)
    })
    .state(ToggleState::On, TogglePart::Root, |s| {
      s.background(Highlight).foreground(HighlightForeground)
    })
}

/// The board's search of a toolbar: as high as the toolbar's other controls, in their type.
pub(super) fn compact_input() -> Recipe {
  Recipe::new().base(InputPart::Control, |s| s.height(Density::Compact).font_size(12.0))
}

/// The board's dense list (the tree, the signals): a table that runs to the panel's edges and fills its height,
/// with rows as low as a line of code under a low header, and a low footer where it counts.
pub(super) fn dense_table<R, K>(table: Table<R, K>) -> Table<R, K>
where
  R: Send + Sync + 'static,
  K: Clone + Eq + Hash + Send + Sync + 'static,
{
  table
    .density(Density::Compact)
    .height(Length::Fill)
    .style(TablePart::Root, |s| {
      s.radius(0.0).border(Border::inside(0.0, PaletteColor::Border))
    })
    .style(TablePart::Header, |s| {
      s.height(DENSE_HEADER_HEIGHT).padding_x(12.0).gap(12.0)
    })
    .style(TablePart::HeaderCell, |s| s.font_size(12.0))
    .style(TablePart::Row, |s| s.height(DENSE_ROW_HEIGHT).padding_x(12.0).gap(12.0))
    .style(TablePart::Cell, |s| s.font_size(12.0))
    .style(TablePart::Footer, |s| s.height(DENSE_HEADER_HEIGHT).padding_x(12.0))
    .style(TablePart::Count, |s| s.font_size(12.0))
}

/// The board's list of details (the lists of a node, a signal's history, the storage): a table flat to the
/// panel's edges that fills its height, a row as low as the compact density's, its labels on the window's fill,
/// its columns one after the other with nothing between.
pub(super) fn flat_table<R, K>(table: Table<R, K>) -> Table<R, K>
where
  R: Send + Sync + 'static,
  K: Clone + Eq + Hash + Send + Sync + 'static,
{
  table
    .density(Density::Compact)
    .height(Length::Fill)
    .style(TablePart::Root, |s| {
      s.radius(0.0).border(Border::inside(0.0, PaletteColor::Border))
    })
    .style(TablePart::Header, |s| {
      s.height(LABELS_HEIGHT)
        .padding_x(DETAIL_PADDING)
        .gap(0.0)
        .background(PaletteColor::Background)
    })
    .style(TablePart::HeaderCell, |s| s.font_size(12.0))
    .style(TablePart::Row, |s| s.padding_x(DETAIL_PADDING).gap(0.0))
    .style(TablePart::Footer, |s| {
      s.height(DENSE_HEADER_HEIGHT).padding_x(DETAIL_PADDING)
    })
    .style(TablePart::Count, |s| s.font_size(12.0))
}

/// Interface text at a size of the board, in the theme's family and at its text scale. It names no color, so it
/// takes the foreground of what it is in.
pub(super) fn sans(content: &str, size: f32, weight: FontWeight) -> Text {
  text(content, TypographyStyle::Body, size, weight)
}

/// Text in the theme's family of code: a name of the inspected app, a value, a type.
pub(super) fn mono(content: &str, size: f32, weight: FontWeight) -> Text {
  text(content, TypographyStyle::Mono, size, weight)
}

fn text(content: &str, role: TypographyStyle, size: f32, weight: FontWeight) -> Text {
  let theme = crate::ui::current_theme();
  let mut style = theme.typography_style(role);
  style.font_size = theme.scaled(size);
  style.weight = weight;
  Text::styled(content, style).nowrap()
}

/// The board's mark of what records, before a toggle's label: a dot in the destructive color, and a square of
/// it where a press stops the recording. It is as wide as an icon of the toggle is.
pub(super) fn record_mark(stop: bool) -> Row {
  let mark = Row::new().size(8.0, 8.0).background(PaletteColor::Destructive);
  Row::new()
    .size(14.0, 14.0)
    .align_items(Alignment::Center)
    .justify(crate::layout::layout_kind::Justify::Center)
    .child(if stop {
      mark.rounded(1.0)
    } else {
      mark.rounded(crate::app::theme::RadiusSize::Full)
    })
}

/// A text of a row that is cut where the row has no room for all of it, `first` times as readily as one that
/// gives way once: what a narrow window loses before its numbers and its buttons.
pub(super) fn gives_way(text: Text, first: f32) -> Row {
  Row::new()
    .flex_shrink(first)
    .min_width(0.0)
    .clip()
    .child(text.width(FILL).text_overflow(TextOverflow::Elipsis))
}

/// The line between two parts of the window.
pub(super) fn line() -> Border {
  Border::inside(BorderSize::Sm, PaletteColor::Border)
}

/// A tag without the modules it is in: `demo::pages::Counter` is `Counter`.
pub(crate) fn short_tag(tag: &str) -> &str {
  tag.rsplit("::").next().unwrap_or(tag)
}

/// A type without the modules of anything in it: `core::Signal<demo::Locale>` is `Signal<Locale>`.
pub(super) fn short_type_name(type_name: &str) -> String {
  let mut short = String::new();
  let mut token = String::new();
  let flush = |short: &mut String, token: &mut String| {
    short.push_str(token.rsplit("::").next().unwrap_or(token));
    token.clear();
  };
  for ch in type_name.chars() {
    if matches!(ch, '<' | '>' | ',' | ' ' | '&' | '[' | ']' | '(' | ')' | ';') {
      flush(&mut short, &mut token);
      short.push(ch);
    } else {
      token.push(ch);
    }
  }
  flush(&mut short, &mut token);
  short
}

/// A count with its word: `1 memo`, `2 effects`, `274 bytes`.
pub(super) fn counted(count: usize, word: &str) -> String {
  if count == 1 {
    format!("1 {word}")
  } else {
    format!("{count} {word}s")
  }
}

/// A length of the inspected app, in its pixels: whole where it is whole.
pub(super) fn px(value: f32) -> String {
  if value.fract().abs() < 0.005 {
    format!("{value:.0}px")
  } else {
    format!("{value:.2}px")
  }
}

/// A number of the inspected app without its unit: a side of the box model.
pub(super) fn number(value: f32) -> String {
  if value.fract().abs() < 0.005 {
    format!("{value:.0}")
  } else {
    format!("{value:.1}")
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_short_type_keeps_what_holds_its_parts_together() {
    assert_eq!(
      short_type_name("lurq::core::context::ReactiveContext<demo::context_demo::LocaleContext>"),
      "ReactiveContext<LocaleContext>"
    );
    assert_eq!(short_type_name("&str"), "&str");
    assert_eq!(
      short_type_name("(alloc::string::String, core::option::Option<u8>)"),
      "(String, Option<u8>)"
    );
  }

  #[test]
  fn the_search_is_focused_once_for_each_press_of_its_key() {
    let key = KeyFocus::detached();
    assert!(!key.wanted());
    key.ask();
    assert!(key.wanted());
    assert!(!key.wanted(), "one press is one focus");
  }

  #[test]
  fn a_count_has_its_word_in_the_singular_for_one() {
    assert_eq!(counted(1, "memo"), "1 memo");
    assert_eq!(counted(0, "effect"), "0 effects");
    assert_eq!(counted(274, "byte"), "274 bytes");
  }

  #[test]
  fn a_length_is_whole_where_it_is_whole() {
    assert_eq!(px(280.0), "280px");
    assert_eq!(px(12.5), "12.50px");
    assert_eq!(number(24.0), "24");
    assert_eq!(number(7.3), "7.3");
  }
}
