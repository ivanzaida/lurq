//! The filters of a data grid: what each column filtered lets through, as a value the app holds and stores, and
//! how a row is let through by them. A column says how it is filtered ([`TableColumn::filter_text`] and the
//! others); the grid shows only the rows every filter lets through.
//!
//! [`TableColumn::filter_text`]: super::TableColumn::filter_text

use std::sync::Arc;

use crate::core::Date;

/// How a filter of text matches a cell's text, whatever its case.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextMatch {
  /// The text is anywhere in it.
  #[default]
  Contains,
  /// It starts with the text.
  StartsWith,
  /// It is the text.
  Equals,
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for TextMatch {}

/// What the filter of a column lets through.
#[derive(Clone, Debug, PartialEq)]
pub enum ColumnFilter {
  /// The rows whose text matches, whatever its case. No text lets every row through.
  Text { how: TextMatch, text: String },
  /// The rows whose number is at least the one and at most the other, where each is given: a range, or a
  /// comparison where one end is open, or one number where both are it.
  Number {
    at_least: Option<f64>,
    at_most: Option<f64>,
  },
  /// The rows whose date is from the one and to the other, both days in, where each is given.
  Date { from: Option<Date>, to: Option<Date> },
  /// The rows whose value is one of these: the values ticked among those the column has.
  Set(Vec<String>),
  /// The rows whose value is this.
  Bool(bool),
}

impl ColumnFilter {
  pub fn contains(text: &str) -> Self {
    Self::Text {
      how: TextMatch::Contains,
      text: text.to_owned(),
    }
  }

  pub fn starts_with(text: &str) -> Self {
    Self::Text {
      how: TextMatch::StartsWith,
      text: text.to_owned(),
    }
  }

  pub fn equals(text: &str) -> Self {
    Self::Text {
      how: TextMatch::Equals,
      text: text.to_owned(),
    }
  }

  /// The numbers from `low` to `high`, both in.
  pub fn between(low: f64, high: f64) -> Self {
    Self::Number {
      at_least: Some(low),
      at_most: Some(high),
    }
  }

  pub fn at_least(low: f64) -> Self {
    Self::Number {
      at_least: Some(low),
      at_most: None,
    }
  }

  pub fn at_most(high: f64) -> Self {
    Self::Number {
      at_least: None,
      at_most: Some(high),
    }
  }

  /// The days from `from` to `to`, both in.
  pub fn dates(from: Date, to: Date) -> Self {
    Self::Date {
      from: Some(from),
      to: Some(to),
    }
  }

  /// The rows whose value is one of `values`.
  pub fn one_of<S: AsRef<str>>(values: impl IntoIterator<Item = S>) -> Self {
    Self::Set(values.into_iter().map(|value| value.as_ref().to_owned()).collect())
  }

  pub fn is(value: bool) -> Self {
    Self::Bool(value)
  }

  /// Whether it lets every row through: a text of nothing, a range open at both ends. A grid holds no such filter.
  pub fn is_open(&self) -> bool {
    match self {
      Self::Text { text, .. } => text.is_empty(),
      Self::Number { at_least, at_most } => at_least.is_none() && at_most.is_none(),
      Self::Date { from, to } => from.is_none() && to.is_none(),
      Self::Set(_) | Self::Bool(_) => false,
    }
  }
}

/// The filters of a data grid: a filter for each column filtered, by the column's id, in the order they were
/// set. The app holds it ([`DataGrid::filters`](super::super::DataGrid::filters)); its text form and its storage are
/// a column layout's.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TableFilters {
  filters: Vec<(Arc<str>, ColumnFilter)>,
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for TableFilters {}

impl TableFilters {
  pub fn new() -> Self {
    Self::default()
  }

  /// With `filter` on `column`, in place of the one it had: a filter that lets every row through takes it off.
  pub fn with(mut self, column: &str, filter: ColumnFilter) -> Self {
    self.set(column, filter);
    self
  }

  /// With no filter on `column`.
  pub fn without(mut self, column: &str) -> Self {
    self.filters.retain(|(id, _)| &**id != column);
    self
  }

  /// Puts `filter` on `column`, in place of the one it had: a filter that lets every row through takes it off.
  pub fn set(&mut self, column: &str, filter: ColumnFilter) {
    if filter.is_open() {
      self.filters.retain(|(id, _)| &**id != column);
      return;
    }
    match self.filters.iter_mut().find(|(id, _)| &**id == column) {
      Some((_, held)) => *held = filter,
      None => self.filters.push((Arc::from(column), filter)),
    }
  }

  /// The filter on `column`.
  pub fn get(&self, column: &str) -> Option<&ColumnFilter> {
    self
      .filters
      .iter()
      .find(|(id, _)| &**id == column)
      .map(|(_, filter)| filter)
  }

  /// Every filter, by its column's id, in the order they were set.
  pub fn iter(&self) -> impl Iterator<Item = (&str, &ColumnFilter)> {
    self.filters.iter().map(|(id, filter)| (&**id, filter))
  }

  pub fn len(&self) -> usize {
    self.filters.len()
  }

  pub fn is_empty(&self) -> bool {
    self.filters.is_empty()
  }

  /// With the filters of the columns `ids` names only: those of columns the grid no longer has are out.
  pub fn valid<'a>(mut self, ids: impl IntoIterator<Item = &'a str>) -> Self {
    let ids: Vec<&str> = ids.into_iter().collect();
    self.filters.retain(|(id, _)| ids.contains(&&**id));
    self
  }
}

/// What a grid over rows the app gives on demand asks for its rows by: their sort, the columns' filters and the
/// search, which the app applies. The grid tells it each time a person changes one. Its text form and its storage
/// are a column layout's.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GridQuery {
  pub sort: Vec<super::TableSort>,
  pub filters: TableFilters,
  pub search: String,
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for GridQuery {}

/// What a column is filtered by: its text, the values a function gives, a number, a date or a yes or no.
pub(crate) enum FilterBy<R> {
  /// Its text, matched by a filter of text.
  Text,
  /// Its text, ticked among the values the column has.
  Set,
  /// The values a function gives, ticked among those the column has.
  Values(Arc<dyn Fn(&R) -> String + Send + Sync>),
  Number(Arc<dyn Fn(&R) -> f64 + Send + Sync>),
  Date(Arc<dyn Fn(&R) -> Option<Date> + Send + Sync>),
  Bool(Arc<dyn Fn(&R) -> bool + Send + Sync>),
}

impl<R> Clone for FilterBy<R> {
  fn clone(&self) -> Self {
    match self {
      Self::Text => Self::Text,
      Self::Set => Self::Set,
      Self::Values(values) => Self::Values(values.clone()),
      Self::Number(number) => Self::Number(number.clone()),
      Self::Date(date) => Self::Date(date.clone()),
      Self::Bool(value) => Self::Bool(value.clone()),
    }
  }
}

/// How a filter tests a row of a column: made once for a filter, then asked of each row.
pub(crate) struct Test<R> {
  test: Box<dyn Fn(&R) -> bool + Send + Sync>,
}

impl<R: 'static> Test<R> {
  /// The test of `filter` on a column filtered `by`, whose text `text` gives: `None` where the filter is not of the
  /// column's kind, which lets every row through.
  pub fn new(
    by: &FilterBy<R>,
    text: Option<Arc<dyn Fn(&R) -> String + Send + Sync>>,
    filter: &ColumnFilter,
  ) -> Option<Self> {
    let test: Box<dyn Fn(&R) -> bool + Send + Sync> = match (by, filter) {
      (FilterBy::Text | FilterBy::Set, ColumnFilter::Text { how, text: wanted }) => {
        let text = text?;
        let (how, wanted) = (*how, wanted.to_lowercase());
        Box::new(move |row: &R| matches_text(&text(row).to_lowercase(), how, &wanted))
      }
      (FilterBy::Values(values), ColumnFilter::Text { how, text: wanted }) => {
        let (values, how, wanted) = (values.clone(), *how, wanted.to_lowercase());
        Box::new(move |row: &R| matches_text(&values(row).to_lowercase(), how, &wanted))
      }
      (FilterBy::Text | FilterBy::Set, ColumnFilter::Set(ticked)) => {
        let text = text?;
        let ticked: std::collections::HashSet<String> = ticked.iter().cloned().collect();
        Box::new(move |row: &R| ticked.contains(&text(row)))
      }
      (FilterBy::Values(values), ColumnFilter::Set(ticked)) => {
        let values = values.clone();
        let ticked: std::collections::HashSet<String> = ticked.iter().cloned().collect();
        Box::new(move |row: &R| ticked.contains(&values(row)))
      }
      (FilterBy::Number(number), ColumnFilter::Number { at_least, at_most }) => {
        let (number, low, high) = (number.clone(), *at_least, *at_most);
        Box::new(move |row: &R| {
          let value = number(row);
          low.is_none_or(|low| value >= low) && high.is_none_or(|high| value <= high)
        })
      }
      (FilterBy::Date(date), ColumnFilter::Date { from, to }) => {
        let (date, from, to) = (date.clone(), *from, *to);
        Box::new(move |row: &R| {
          date(row).is_some_and(|day| from.is_none_or(|from| day >= from) && to.is_none_or(|to| day <= to))
        })
      }
      (FilterBy::Bool(value), ColumnFilter::Bool(wanted)) => {
        let (value, wanted) = (value.clone(), *wanted);
        Box::new(move |row: &R| value(row) == wanted)
      }
      _ => return None,
    };
    Some(Self { test })
  }

  pub fn passes(&self, row: &R) -> bool {
    (self.test)(row)
  }
}

fn matches_text(text: &str, how: TextMatch, wanted: &str) -> bool {
  match how {
    TextMatch::Contains => text.contains(wanted),
    TextMatch::StartsWith => text.starts_with(wanted),
    TextMatch::Equals => text == wanted,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_filter_that_lets_every_row_through_takes_the_columns_off() {
    let filters = TableFilters::new()
      .with("name", ColumnFilter::contains("lurq"))
      .with("size", ColumnFilter::at_least(10.0));
    assert_eq!(filters.len(), 2);
    let filters = filters.with("name", ColumnFilter::contains(""));
    assert_eq!(filters.len(), 1);
    assert!(filters.get("name").is_none());
    assert_eq!(filters.get("size"), Some(&ColumnFilter::at_least(10.0)));
    assert!(filters.clone().valid(["name"]).is_empty());
  }

  #[test]
  fn each_kind_tests_its_rows() {
    let text: Arc<dyn Fn(&(String, f64)) -> String + Send + Sync> = Arc::new(|row: &(String, f64)| row.0.clone());
    let row = ("crates/Lurq".to_owned(), 12.0);
    let test = |by: &FilterBy<(String, f64)>, filter: &ColumnFilter| {
      Test::new(by, Some(text.clone()), filter).unwrap().passes(&row)
    };
    assert!(
      test(&FilterBy::Text, &ColumnFilter::contains("lurq")),
      "whatever its case"
    );
    assert!(test(&FilterBy::Text, &ColumnFilter::starts_with("CRATES")));
    assert!(!test(&FilterBy::Text, &ColumnFilter::equals("lurq")));
    assert!(test(&FilterBy::Set, &ColumnFilter::one_of(["crates/Lurq", "x"])));
    let number = FilterBy::Number(Arc::new(|row: &(String, f64)| row.1));
    assert!(test(&number, &ColumnFilter::between(10.0, 12.0)), "both ends in");
    assert!(!test(&number, &ColumnFilter::at_least(12.5)));
    assert!(
      Test::new(&number, None, &ColumnFilter::contains("1")).is_none(),
      "a filter of another kind tests nothing"
    );
  }
}
