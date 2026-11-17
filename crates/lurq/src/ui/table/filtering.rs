//! Filtering a data grid as a person does it: the mark of a filtered column's header, the panel the column's menu
//! opens to filter by it, and the row of filters under the header, each as the column is filtered (its text, the
//! values ticked, a number, a day, a yes or no).

use std::{collections::HashMap, sync::Arc};

use super::{
  super::{
    button::{Button, ButtonSize, ButtonVariant},
    checkbox::Checkbox,
    date_field::DateField,
    input::{Input, InputSize},
    number_input::NumberInput,
    popover::Popover,
    recipe::{Name, ResolvedPart},
    select::Select,
  },
  ColumnFilter, HostState, TableHeader, TablePart, TableView, TextMatch, glyph, icons,
};
use crate::{
  app::ctx::Placement,
  components::{Column, Row, ScrollVertical},
  core::{Date, ElementRef, Signal},
  layout::{Alignment, layout_kind::Sticky},
  node::{Element, dimension::Dimension},
};

/// Every word of filtering a data grid: the column menu's items, the panel's, and the filter row's.
#[derive(Clone, Debug, PartialEq)]
pub struct FilterLabels {
  /// The menu's item that opens the column's filter.
  pub filter: String,
  /// The menu's item and the panel's button that take the column's filter off.
  pub clear_filter: String,
  /// The menu's item that takes every filter off.
  pub clear_all: String,
  pub contains: String,
  pub starts_with: String,
  pub equals: String,
  pub at_least: String,
  pub at_most: String,
  pub from: String,
  pub to: String,
  /// No filter, in a choice of values or of yes and no.
  pub all: String,
  pub yes: String,
  pub no: String,
  pub select_all: String,
  pub select_none: String,
  /// What an empty filter of text says in the filter row.
  pub placeholder: String,
  /// What an empty filter of numbers or days says in the filter row: how a range is written.
  pub range_placeholder: String,
  /// What the filter row's choice of values says where several are ticked: `{count}` is how many, in the form
  /// the locale's language gives the word after it (a plain sentence, `"{count} chosen".into()`, is the same
  /// after every number).
  pub several: crate::core::Plural,
}

impl Default for FilterLabels {
  fn default() -> Self {
    let locale = crate::ui::current_locale();
    Self {
      filter: locale.data.filter_placeholder.to_string(),
      clear_filter: locale.data.clear_filter.to_string(),
      clear_all: locale.data.clear_all_filters.to_string(),
      contains: locale.data.contains.to_string(),
      starts_with: locale.data.starts_with.to_string(),
      equals: locale.data.equals.to_string(),
      at_least: locale.data.at_least.to_string(),
      at_most: locale.data.at_most.to_string(),
      from: locale.data.from.to_string(),
      to: locale.data.to.to_string(),
      all: locale.data.all.to_string(),
      yes: locale.common.yes.to_string(),
      no: locale.common.no.to_string(),
      select_all: locale.common.select_all.to_string(),
      select_none: locale.data.select_none.to_string(),
      placeholder: locale.data.filter.to_string(),
      range_placeholder: "5..20".into(),
      several: locale.data.values.clone(),
    }
  }
}

/// How a column is filtered, as its filter's editors see it.
#[derive(Clone)]
pub(super) enum FilterShape {
  Text,
  /// The values the column has, which are ticked.
  Set(Arc<Vec<String>>),
  Number,
  Date,
  Bool,
}

/// A filtering column's header and its grid's host: what its filter is and what changes it.
pub(super) struct FilterLink {
  pub id: Arc<str>,
  pub title: Arc<str>,
  pub shape: FilterShape,
  pub current: Option<ColumnFilter>,
  /// Puts a filter on the column, or takes it off.
  pub change: Arc<dyn Fn(Option<ColumnFilter>) + Send + Sync>,
  /// Whether its panel is open.
  pub open: bool,
  pub host: Arc<HostState>,
  pub labels: Arc<FilterLabels>,
}

/// What a grid's host keeps of filtering: which column's panel is open, and what is typed in a filter of text.
#[derive(Default)]
pub(super) struct FilterState {
  pub column: parking_lot::Mutex<Option<Arc<str>>>,
  pub texts: parking_lot::Mutex<HashMap<Arc<str>, Signal<String>>>,
  /// The values of a column filtered by its values, kept for the rows they were taken from.
  pub values: parking_lot::Mutex<HashMap<Arc<str>, (usize, Arc<Vec<String>>)>>,
}

impl FilterState {
  /// What is typed in the filter of text of `column`, which shows `now` where nothing else is typed.
  pub fn text(&self, column: &Arc<str>, now: &str) -> Signal<String> {
    let signal = self
      .texts
      .lock()
      .entry(column.clone())
      .or_insert_with(|| Signal::new(now.to_owned()))
      .clone();
    // A filter taken off elsewhere (clear all) empties what is typed.
    if now.is_empty() && !signal.get_untracked().is_empty() {
      signal.set(String::new());
    }
    signal
  }
}

/// The most values a filter of values lists.
pub(super) const MOST_VALUES: usize = 500;

impl FilterLink {
  /// The editors of the filter, in the panel.
  fn editors(&self, theme: &crate::app::theme::Theme, label: &ResolvedPart) -> Element {
    // Each editor under the word of what it takes, which the panel shows.
    let titled = |word: &str, editor: Element| -> Element {
      Column::new()
        .spacing(4.0)
        .width(Dimension::Pct(100.0))
        .child(label.text(word).part(label))
        .child(editor)
        .into()
    };
    let labels = &self.labels;
    let change = self.change.clone();
    let mut column = Column::new().spacing(8.0).width(Dimension::Pct(100.0));
    match &self.shape {
      FilterShape::Text => {
        let (how, text) = match &self.current {
          Some(ColumnFilter::Text { how, text }) => (*how, text.clone()),
          _ => (TextMatch::Contains, String::new()),
        };
        let typed = self.host.filters.text(&self.id, &text);
        let (on_how, on_text) = (change.clone(), change.clone());
        let held = typed.clone();
        column = column
          .child(
            Select::controlled(
              how,
              [
                (TextMatch::Contains, labels.contains.as_str()),
                (TextMatch::StartsWith, labels.starts_with.as_str()),
                (TextMatch::Equals, labels.equals.as_str()),
              ],
              move |how: &TextMatch| {
                on_how(Some(ColumnFilter::Text {
                  how: *how,
                  text: held.get_untracked(),
                }))
              },
            )
            .size(InputSize::Sm)
            .full_width(true)
            .label(&self.title)
            .build(theme),
          )
          .child(
            Input::new(typed)
              .size(InputSize::Sm)
              .full_width(true)
              .label(&self.title)
              .placeholder(&labels.placeholder)
              .on_input(move |event| {
                on_text(Some(ColumnFilter::Text {
                  how,
                  text: event.new_value.clone(),
                }))
              })
              .build(theme),
          );
      }
      FilterShape::Set(values) => {
        let ticked: Option<Vec<String>> = match &self.current {
          Some(ColumnFilter::Set(ticked)) => Some(ticked.clone()),
          _ => None,
        };
        let (all, none) = (change.clone(), change.clone());
        column = column.child(
          Row::new()
            .spacing(6.0)
            .child(
              Button::new(&labels.select_all)
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Sm)
                .on_click(move |_| all(None))
                .build(theme),
            )
            .child(
              Button::new(&labels.select_none)
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Sm)
                .on_click(move |_| none(Some(ColumnFilter::Set(Vec::new()))))
                .build(theme),
            ),
        );
        let mut list = Column::new().spacing(6.0);
        for value in values.iter() {
          let on = ticked.as_ref().is_none_or(|ticked| ticked.contains(value));
          let (change, values, ticked, value_now) = (change.clone(), values.clone(), ticked.clone(), value.clone());
          list = list.child(
            Checkbox::new(value)
              .checked(on)
              .on_change(move |checked| {
                // Every value ticked is no filter.
                let mut next: Vec<String> = ticked.clone().unwrap_or_else(|| (*values).clone());
                next.retain(|held| held != &value_now);
                if checked {
                  next.push(value_now.clone());
                }
                if next.len() == values.len() {
                  change(None);
                } else {
                  next.sort_by_key(|held| values.iter().position(|value| value == held));
                  change(Some(ColumnFilter::Set(next)));
                }
              })
              .build(theme),
          );
        }
        column = column.child(ScrollVertical::new(list).max_height(240.0).width(Dimension::Pct(100.0)));
      }
      FilterShape::Number => {
        let (low, high) = match &self.current {
          Some(ColumnFilter::Number { at_least, at_most }) => (*at_least, *at_most),
          _ => (None, None),
        };
        let (on_low, on_high) = (change.clone(), change.clone());
        column = column
          .child(titled(
            &labels.at_least,
            NumberInput::controlled(low, move |at_least| {
              on_low(Some(ColumnFilter::Number {
                at_least,
                at_most: high,
              }))
            })
            .size(InputSize::Sm)
            .full_width(true)
            .full_width(true)
            .label(&labels.at_least)
            .build(theme),
          ))
          .child(titled(
            &labels.at_most,
            NumberInput::controlled(high, move |at_most| {
              on_high(Some(ColumnFilter::Number { at_least: low, at_most }))
            })
            .size(InputSize::Sm)
            .full_width(true)
            .full_width(true)
            .label(&labels.at_most)
            .build(theme),
          ));
      }
      FilterShape::Date => {
        let (from, to) = match &self.current {
          Some(ColumnFilter::Date { from, to }) => (*from, *to),
          _ => (None, None),
        };
        let (on_from, on_to) = (change.clone(), change.clone());
        column = column
          .child(titled(
            &labels.from,
            DateField::controlled_optional(from, move |from| on_from(Some(ColumnFilter::Date { from, to })))
              .size(InputSize::Sm)
              .full_width(true)
              .full_width(true)
              .label(&labels.from)
              .build(theme),
          ))
          .child(titled(
            &labels.to,
            DateField::controlled_optional(to, move |to| on_to(Some(ColumnFilter::Date { from, to })))
              .size(InputSize::Sm)
              .full_width(true)
              .full_width(true)
              .label(&labels.to)
              .build(theme),
          ));
      }
      FilterShape::Bool => column = column.child(bool_select(self, theme)),
    }
    let clear = change.clone();
    column
      .child(
        Row::new().child(
          Button::new(&labels.clear_filter)
            .variant(ButtonVariant::Ghost)
            .size(ButtonSize::Sm)
            .disabled(self.current.is_none())
            .on_click(move |_| clear(None))
            .build(theme),
        ),
      )
      .into()
  }

  /// The cell of the filter row: a box for a text, a number or a day, a choice for values or a yes or no.
  fn row_cell(&self, theme: &crate::app::theme::Theme, id: Option<String>) -> Element {
    let labels = &self.labels;
    let change = self.change.clone();
    match &self.shape {
      FilterShape::Text | FilterShape::Number | FilterShape::Date => {
        let shape = self.shape.clone();
        let now = match &self.current {
          Some(filter) => written(filter),
          None => String::new(),
        };
        let mut input = Input::new(self.host.filters.text(&self.id, &now));
        if let Some(id) = id {
          input = input.id(id);
        }
        input
          .size(InputSize::Sm)
          .full_width(true)
          .label(&self.title)
          .placeholder(if matches!(shape, FilterShape::Text) {
            &labels.placeholder
          } else {
            &labels.range_placeholder
          })
          .on_input(move |event| {
            let text = event.new_value.trim();
            if text.is_empty() {
              change(None);
              return;
            }
            // What is typed and is not a range yet leaves the filter as it was.
            if let Some(filter) = read(&shape, text) {
              change(Some(filter));
            }
          })
          .build(theme)
      }
      FilterShape::Set(values) => {
        // One value ticked is chosen; several, from the panel, are said by their count.
        let several = match &self.current {
          Some(ColumnFilter::Set(ticked)) if ticked.len() != 1 => Some(ticked.len()),
          _ => None,
        };
        let chosen = match &self.current {
          Some(ColumnFilter::Set(ticked)) if ticked.len() == 1 => values.iter().position(|value| value == &ticked[0]),
          Some(ColumnFilter::Set(_)) => Some(usize::MAX),
          _ => None,
        };
        let options = std::iter::once((None, labels.all.clone()))
          .chain(several.map(|count| (Some(usize::MAX), crate::ui::current_locale().count(&labels.several, count))))
          .chain(values.iter().enumerate().map(|(at, value)| (Some(at), value.clone())));
        let values = values.clone();
        Select::controlled(chosen, options, move |at: &Option<usize>| {
          if *at == Some(usize::MAX) {
            return;
          }
          change(
            at.and_then(|at| values.get(at))
              .map(|value| ColumnFilter::Set(vec![value.clone()])),
          )
        })
        .size(InputSize::Sm)
        .full_width(true)
        .label(&self.title)
        .build(theme)
      }
      FilterShape::Bool => bool_select(self, theme),
    }
  }
}

/// A choice of all, yes or no.
fn bool_select(link: &FilterLink, theme: &crate::app::theme::Theme) -> Element {
  let labels = &link.labels;
  let chosen = match &link.current {
    Some(ColumnFilter::Bool(value)) => Some(*value),
    _ => None,
  };
  let change = link.change.clone();
  Select::controlled(
    chosen,
    [
      (None, labels.all.as_str()),
      (Some(true), labels.yes.as_str()),
      (Some(false), labels.no.as_str()),
    ],
    move |value: &Option<bool>| change(value.map(ColumnFilter::Bool)),
  )
  .size(InputSize::Sm)
  .full_width(true)
  .label(&link.title)
  .build(theme)
}

/// A filter as the filter row writes it: its text, or its range (`>=10`, `<=5`, `5..20`).
fn written(filter: &ColumnFilter) -> String {
  match filter {
    ColumnFilter::Text { text, .. } => text.clone(),
    ColumnFilter::Number { at_least, at_most } => {
      range(at_least.map(|n| n.to_string()), at_most.map(|n| n.to_string()))
    }
    ColumnFilter::Date { from, to } => range(from.map(|day| day.to_string()), to.map(|day| day.to_string())),
    ColumnFilter::Set(values) => values.join(", "),
    ColumnFilter::Bool(value) => value.to_string(),
  }
}

fn range(low: Option<String>, high: Option<String>) -> String {
  match (low, high) {
    (Some(low), Some(high)) if low == high => low,
    (Some(low), Some(high)) => format!("{low}..{high}"),
    (Some(low), None) => format!(">={low}"),
    (None, Some(high)) => format!("<={high}"),
    (None, None) => String::new(),
  }
}

/// What the filter row reads in `text` for a column of `shape`: a text contains it; a number or a day is one, a
/// comparison (`>`, `>=`, `<`, `<=`, `=`) or a range (`a..b`). `None` for what is not one yet.
pub(super) fn read(shape: &FilterShape, text: &str) -> Option<ColumnFilter> {
  match shape {
    FilterShape::Text => Some(ColumnFilter::contains(text)),
    FilterShape::Number => {
      let (low, high) = bounds(text, |part| part.trim().parse::<f64>().ok())?;
      Some(ColumnFilter::Number {
        at_least: low,
        at_most: high,
      })
    }
    FilterShape::Date => {
      let (from, to) = bounds(text, |part| parse_day(part.trim()))?;
      Some(ColumnFilter::Date { from, to })
    }
    FilterShape::Set(_) | FilterShape::Bool => None,
  }
}

/// The ends of a comparison or a range. A `>` or a `<` is taken as its `>=` or `<=`: the grid's ranges are closed.
fn bounds<T: Copy>(text: &str, value: impl Fn(&str) -> Option<T>) -> Option<(Option<T>, Option<T>)> {
  if let Some((low, high)) = text.split_once("..") {
    let low = if low.trim().is_empty() { None } else { Some(value(low)?) };
    let high = if high.trim().is_empty() {
      None
    } else {
      Some(value(high)?)
    };
    return Some((low, high));
  }
  for (sign, low) in [(">=", true), ("<=", false), (">", true), ("<", false), ("=", true)] {
    if let Some(rest) = text.strip_prefix(sign) {
      let at = value(rest)?;
      return Some(match (sign, low) {
        ("=", _) => (Some(at), Some(at)),
        (_, true) => (Some(at), None),
        _ => (None, Some(at)),
      });
    }
  }
  let at = value(text)?;
  Some((Some(at), Some(at)))
}

fn parse_day(text: &str) -> Option<Date> {
  let mut parts = text.splitn(3, '-');
  Date::new(
    parts.next()?.parse().ok()?,
    parts.next()?.parse().ok()?,
    parts.next()?.parse().ok()?,
  )
}

impl TableHeader {
  /// The mark of a column the rows are filtered by, after its title: `None` for one they are not.
  pub fn filter_mark(&self) -> Option<Element> {
    if !self.filtered {
      return None;
    }
    Some(glyph(
      &self.part(TablePart::FilterIcon),
      Name::new(icons::FILTER),
      false,
    ))
  }

  /// The panel that filters the rows by the column, while it is open: under its header, from its menu.
  pub fn filter_panel(&self) -> Option<Element> {
    let link = self.filter.clone().filter(|link| link.open)?;
    let theme = self.recipe.theme();
    let host = link.host.clone();
    let mut panel = Popover::new(host.filter_open.clone(), host.filter_anchor.clone())
      .label(&link.title)
      .title(&link.title)
      .placement(Placement::BottomEnd)
      .width(260.0)
      .on_dismiss(move || {
        *host.filters.column.lock() = None;
        host.rebuild();
      })
      .child(link.editors(theme, &self.part(TablePart::FilterLabel)));
    if let Some(id) = &self.id {
      panel = panel.id(format!("{id}-filter-panel"));
    }
    Some(panel.build(theme))
  }

  /// Where the filter panel hangs from while it is open for this column: its menu's trigger.
  pub(super) fn filter_anchor(&self) -> Option<ElementRef> {
    self
      .filter
      .as_ref()
      .filter(|link| link.open)
      .map(|link| link.host.filter_anchor.clone())
  }
}

impl TableView {
  /// The row of filters under the header, where the grid has one: a cell for each column, a filter in that of each
  /// column that filters. It stays under the header while the rows scroll.
  pub fn filter_row(&self) -> Option<Row> {
    if !self.filter_row {
      return None;
    }
    let theme = self.theme();
    let part = self.part(TablePart::FilterRow);
    let header = super::px(self.part(TablePart::Header).rest().height, theme).unwrap_or(40.0);
    let mut row = Row::new()
      .part(&part)
      .spacing(0.0)
      .align_items(Alignment::Center)
      .column_tracks(&self.tracks)
      .sticky(Sticky::top(header))
      .describe("a11y_role", "row");
    if self.pinned_start {
      row = row.padding_left(0.0);
    }
    if self.pinned_end {
      row = row.padding_right(0.0);
    }
    if self.numbered {
      row = row.child(Row::new());
    }
    if self.checkboxes {
      row = row.child(Row::new());
    }
    let cells = self.headers.iter().map(|header| {
      let mut cell = Row::new()
        .align_items(Alignment::Center)
        .min_width(Dimension::Px(0.0))
        .padding_right(if header.last { header.trail } else { header.gap });
      if let Some(link) = &header.filter {
        cell = cell.child(
          Column::new()
            .flex_full(1.0, 1.0, Some(0.0))
            .min_width(Dimension::Px(0.0))
            .child(link.row_cell(theme, self.id.as_ref().map(|id| format!("{id}-filter-{}", link.id)))),
        );
      }
      if let Some(pinned) = header.pinned {
        cell = cell.sticky(pinned);
        if let Some(fill) = part.rest().background {
          cell = cell.background(fill);
        }
      }
      (header.column, cell.into())
    });
    Some(row.with_children(super::in_tracks(
      self.columns_built.as_ref().map(|built| built.as_slice()),
      cells,
    )))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_filter_row_reads_comparisons_and_ranges() {
    let number = |text| read(&FilterShape::Number, text);
    assert_eq!(number(">10"), Some(ColumnFilter::at_least(10.0)));
    assert_eq!(number("<=5"), Some(ColumnFilter::at_most(5.0)));
    assert_eq!(number("5..20"), Some(ColumnFilter::between(5.0, 20.0)));
    assert_eq!(number("7"), Some(ColumnFilter::between(7.0, 7.0)));
    assert_eq!(number("5.."), Some(ColumnFilter::at_least(5.0)));
    assert_eq!(number(">"), None, "not a number yet");
    let day = |text| read(&FilterShape::Date, text);
    let first = Date::new(2026, 1, 1).unwrap();
    assert_eq!(
      day(">=2026-01-01"),
      Some(ColumnFilter::Date {
        from: Some(first),
        to: None
      })
    );
    assert_eq!(written(&ColumnFilter::between(5.0, 20.0)), "5..20");
    assert_eq!(written(&ColumnFilter::at_least(5.0)), ">=5");
  }
}
