//! The text forms of a data grid's values an app stores between runs: the arrangement of its columns, its sort and
//! its groups open. Each reads back to an equal value, through `parse` or `FromStr`, and is stored as that text by
//! the persistent storage, as a dock's state is.

use std::{fmt, str::FromStr, sync::Arc};

use super::{ColumnLayout, ColumnPin, ColumnPlace, GroupKey, SortDirection, TableSort};

/// What `FromStr` returns for text that is not the text form of a table's value: a [`ColumnLayout`], a
/// [`TableSort`], a [`GroupKey`] or a list of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TableTextError;

impl fmt::Display for TableTextError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("not the text form of a table's value")
  }
}

impl std::error::Error for TableTextError {}

fn write_string(out: &mut String, text: &str) {
  out.push('"');
  for c in text.chars() {
    if c == '"' || c == '\\' {
      out.push('\\');
    }
    out.push(c);
  }
  out.push('"');
}

fn space(out: &mut String) {
  if !out.is_empty() && !out.ends_with('(') {
    out.push(' ');
  }
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
  Open,
  Close,
  Colon,
  Text(String),
  Word(String),
}

fn tokenize(text: &str) -> Option<Vec<Token>> {
  let mut tokens = Vec::new();
  let mut chars = text.chars().peekable();
  while let Some(&c) = chars.peek() {
    match c {
      c if c.is_whitespace() => {
        chars.next();
      }
      '(' | ')' | ':' => {
        chars.next();
        tokens.push(match c {
          '(' => Token::Open,
          ')' => Token::Close,
          _ => Token::Colon,
        });
      }
      '"' => {
        chars.next();
        let mut string = String::new();
        loop {
          match chars.next()? {
            '"' => break,
            '\\' => match chars.next()? {
              escaped @ ('"' | '\\') => string.push(escaped),
              _ => return None,
            },
            c => string.push(c),
          }
        }
        tokens.push(Token::Text(string));
      }
      _ => {
        let mut word = String::new();
        while let Some(&c) = chars.peek() {
          if c.is_whitespace() || matches!(c, '(' | ')' | ':' | '"') {
            break;
          }
          word.push(c);
          chars.next();
        }
        tokens.push(Token::Word(word));
      }
    }
  }
  Some(tokens)
}

struct Reader {
  tokens: Vec<Token>,
  at: usize,
}

impl Reader {
  fn new(text: &str) -> Option<Self> {
    Some(Self {
      tokens: tokenize(text)?,
      at: 0,
    })
  }

  fn done(&self) -> bool {
    self.at >= self.tokens.len()
  }

  fn peek(&self) -> Option<&Token> {
    self.tokens.get(self.at)
  }

  fn next(&mut self) -> Option<Token> {
    let token = self.tokens.get(self.at).cloned();
    self.at += 1;
    token
  }

  fn expect(&mut self, token: Token) -> Option<()> {
    (self.next()? == token).then_some(())
  }

  fn text(&mut self) -> Option<String> {
    match self.next()? {
      Token::Text(text) => Some(text),
      _ => None,
    }
  }

  fn word(&mut self) -> Option<String> {
    match self.next()? {
      Token::Word(word) => Some(word),
      _ => None,
    }
  }

  /// `name(`, where the next word is `name`.
  fn opens(&mut self, name: &str) -> Option<()> {
    (self.word()? == name).then_some(())?;
    self.expect(Token::Open)
  }

  /// The quoted texts up to the `)` that closes them.
  fn texts_to_close(&mut self) -> Option<Vec<String>> {
    let mut texts = Vec::new();
    loop {
      match self.next()? {
        Token::Close => return Some(texts),
        Token::Text(text) => texts.push(text),
        _ => return None,
      }
    }
  }
}

// The arrangement of the columns.

impl ColumnLayout {
  /// The arrangement as text that `parse` reads back to the same arrangement: each column in its order as
  /// `column(..)` of its id quoted, then `hidden`, `fit`, `width:` and its width, `pin:start` or `pin:end` where it
  /// is so.
  ///
  /// ```
  /// use lurq::ui::{ColumnLayout, ColumnPin, ColumnPlace};
  ///
  /// let layout = ColumnLayout {
  ///   columns: vec![
  ///     ColumnPlace { id: "size".into(), hidden: false, width: Some(150.0), fit: false, pin: ColumnPin::None },
  ///     ColumnPlace { id: "name".into(), hidden: false, width: None, fit: false, pin: ColumnPin::Start },
  ///     ColumnPlace { id: "status".into(), hidden: true, width: None, fit: false, pin: ColumnPin::None },
  ///   ],
  /// };
  /// assert_eq!(layout.to_text(), r#"column("size" width:150) column("name" pin:start) column("status" hidden)"#);
  /// assert_eq!(ColumnLayout::parse(&layout.to_text()), Some(layout));
  /// ```
  pub fn to_text(&self) -> String {
    let mut out = String::new();
    for place in &self.columns {
      space(&mut out);
      out.push_str("column(");
      write_string(&mut out, &place.id);
      if place.hidden {
        out.push_str(" hidden");
      }
      if place.fit {
        out.push_str(" fit");
      }
      if let Some(width) = place.width {
        out.push_str(&format!(" width:{width}"));
      }
      match place.pin {
        ColumnPin::Start => out.push_str(" pin:start"),
        ColumnPin::End => out.push_str(" pin:end"),
        ColumnPin::None => {}
      }
      out.push(')');
    }
    out
  }

  /// The arrangement `text` writes (see `to_text`), as written: `None` for text that is not the form. The grid makes
  /// it valid for its columns as it shows it; [`valid`](Self::valid) does it for the app.
  pub fn parse(text: &str) -> Option<ColumnLayout> {
    let mut reader = Reader::new(text)?;
    let mut columns = Vec::new();
    while !reader.done() {
      reader.opens("column")?;
      let mut place = ColumnPlace {
        id: Arc::from(reader.text()?),
        hidden: false,
        width: None,
        fit: false,
        pin: ColumnPin::None,
      };
      loop {
        match reader.next()? {
          Token::Close => break,
          Token::Word(word) => match word.as_str() {
            "hidden" => place.hidden = true,
            "fit" => place.fit = true,
            "width" => {
              reader.expect(Token::Colon)?;
              place.width = Some(reader.word()?.parse().ok()?);
            }
            "pin" => {
              reader.expect(Token::Colon)?;
              place.pin = match reader.word()?.as_str() {
                "start" => ColumnPin::Start,
                "end" => ColumnPin::End,
                _ => return None,
              };
            }
            _ => return None,
          },
          _ => return None,
        }
      }
      columns.push(place);
    }
    Some(ColumnLayout { columns })
  }

  /// The arrangement brought to the columns `ids` the app has now, in their order: a column it no longer has is
  /// left out, a column named twice is kept once, a width that is no width is the column's own, and a column the
  /// arrangement does not name takes its place after the one before it among `ids`.
  pub fn valid<S: AsRef<str>>(self, ids: impl IntoIterator<Item = S>) -> Self {
    let ids: Vec<String> = ids.into_iter().map(|id| id.as_ref().to_owned()).collect();
    let mut places: Vec<ColumnPlace> = Vec::with_capacity(ids.len());
    for mut place in self.columns {
      if !ids.iter().any(|id| **id == *place.id) || places.iter().any(|kept| kept.id == place.id) {
        continue;
      }
      place.width = place.width.filter(|width| width.is_finite() && *width >= 0.0);
      places.push(place);
    }
    for (index, id) in ids.iter().enumerate() {
      if places.iter().any(|place| *place.id == **id) {
        continue;
      }
      let at = ids[..index]
        .iter()
        .rev()
        .find_map(|before| places.iter().position(|place| *place.id == **before))
        .map_or(0, |before| before + 1);
      places.insert(
        at,
        ColumnPlace {
          id: Arc::from(id.as_str()),
          hidden: false,
          width: None,
          fit: false,
          pin: ColumnPin::None,
        },
      );
    }
    ColumnLayout { columns: places }
  }
}

impl fmt::Display for ColumnLayout {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.to_text())
  }
}

impl FromStr for ColumnLayout {
  type Err = TableTextError;

  fn from_str(text: &str) -> Result<Self, Self::Err> {
    ColumnLayout::parse(text).ok_or(TableTextError)
  }
}

// The sort.

impl TableSort {
  /// The sort as text that `parse` reads back to the same sort: the column's id quoted, then `ascending` or
  /// `descending`: `"name" descending`.
  pub fn to_text(&self) -> String {
    let mut out = String::new();
    write_string(&mut out, &self.column);
    out.push_str(match self.direction {
      SortDirection::Ascending => " ascending",
      SortDirection::Descending => " descending",
    });
    out
  }

  /// The sort `text` writes (see `to_text`): `None` for text that is not the form.
  pub fn parse(text: &str) -> Option<TableSort> {
    let mut reader = Reader::new(text)?;
    let column = reader.text()?;
    let direction = match reader.word()?.as_str() {
      "ascending" => SortDirection::Ascending,
      "descending" => SortDirection::Descending,
      _ => return None,
    };
    reader.done().then(|| TableSort {
      column: Arc::from(column),
      direction,
    })
  }
}

impl fmt::Display for TableSort {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.to_text())
  }
}

impl FromStr for TableSort {
  type Err = TableTextError;

  fn from_str(text: &str) -> Result<Self, Self::Err> {
    TableSort::parse(text).ok_or(TableTextError)
  }
}

// The groups.

impl GroupKey {
  /// The group as text that `parse` reads back to the same group: `group(..)` of its values quoted:
  /// `group("Passing" "Ada")`.
  pub fn to_text(&self) -> String {
    let mut out = String::from("group(");
    for value in &self.values {
      space(&mut out);
      write_string(&mut out, value);
    }
    out.push(')');
    out
  }

  /// The group `text` writes (see `to_text`): `None` for text that is not the form.
  pub fn parse(text: &str) -> Option<GroupKey> {
    let mut groups = GroupKey::parse_list(text)?;
    (groups.len() == 1).then(|| groups.remove(0))
  }

  /// Groups as text, the groups open of a grid: each group's text in turn, which `parse_list` reads back.
  pub fn list_to_text(groups: &[GroupKey]) -> String {
    groups.iter().map(GroupKey::to_text).collect::<Vec<_>>().join(" ")
  }

  /// The groups `text` writes (see `list_to_text`): `None` for text that is not the form.
  pub fn parse_list(text: &str) -> Option<Vec<GroupKey>> {
    let mut reader = Reader::new(text)?;
    let mut groups = Vec::new();
    while !reader.done() {
      reader.opens("group")?;
      groups.push(GroupKey {
        values: reader.texts_to_close()?,
      });
    }
    Some(groups)
  }
}

impl fmt::Display for GroupKey {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.to_text())
  }
}

impl FromStr for GroupKey {
  type Err = TableTextError;

  fn from_str(text: &str) -> Result<Self, Self::Err> {
    GroupKey::parse(text).ok_or(TableTextError)
  }
}

// Stored as their text forms, the way a string is.

#[cfg(feature = "persistent_storage")]
mod stored {
  use super::{ColumnLayout, GroupKey, TableSort};
  use crate::persistent_storage::{IntoPersistentValue, PersistentValue};

  fn text(bytes: &[u8]) -> Option<String> {
    <String as PersistentValue>::decode_persistent_value(bytes)
  }

  /// Stored as its text form, the way a string is.
  impl PersistentValue for ColumnLayout {
    fn decode_persistent_value(bytes: &[u8]) -> Option<Self> {
      ColumnLayout::parse(&text(bytes)?)
    }
  }

  /// Stored as its text form, the way a string is.
  impl IntoPersistentValue for ColumnLayout {
    fn encode_persistent_value(self) -> Vec<u8> {
      self.to_text().encode_persistent_value()
    }
  }

  /// Stored as its text form, the way a string is.
  impl PersistentValue for TableSort {
    fn decode_persistent_value(bytes: &[u8]) -> Option<Self> {
      TableSort::parse(&text(bytes)?)
    }
  }

  /// Stored as its text form, the way a string is.
  impl IntoPersistentValue for TableSort {
    fn encode_persistent_value(self) -> Vec<u8> {
      self.to_text().encode_persistent_value()
    }
  }

  /// A grid's sort, none as no text.
  impl PersistentValue for Option<TableSort> {
    fn decode_persistent_value(bytes: &[u8]) -> Option<Self> {
      let text = text(bytes)?;
      if text.trim().is_empty() {
        return Some(None);
      }
      Some(Some(TableSort::parse(&text)?))
    }
  }

  /// A grid's sort, none as no text.
  impl IntoPersistentValue for Option<TableSort> {
    fn encode_persistent_value(self) -> Vec<u8> {
      self
        .map(|sort| sort.to_text())
        .unwrap_or_default()
        .encode_persistent_value()
    }
  }

  /// A grid's groups open, as their list's text.
  impl PersistentValue for Vec<GroupKey> {
    fn decode_persistent_value(bytes: &[u8]) -> Option<Self> {
      GroupKey::parse_list(&text(bytes)?)
    }
  }

  /// A grid's groups open, as their list's text.
  impl IntoPersistentValue for Vec<GroupKey> {
    fn encode_persistent_value(self) -> Vec<u8> {
      GroupKey::list_to_text(&self).encode_persistent_value()
    }
  }
}
