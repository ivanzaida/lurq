//! The tokens of a piece of code, for drawing it in colors: what a code block of Markdown and `ui::CodeBlock` color
//! their code by. It knows Rust (`rust`, `rs`); the code of a language it does not know is one plain token.
//!
//! ```
//! use lurq::code::{CodeTokenKind, tokens};
//!
//! let code = "let app = App::new(); // one";
//! let kinds: Vec<_> = tokens(code, "rust")
//!   .into_iter()
//!   .filter(|token| token.kind != CodeTokenKind::Plain)
//!   .map(|token| (&code[token.range], token.kind))
//!   .collect();
//! assert_eq!(
//!   kinds,
//!   [
//!     ("let", CodeTokenKind::Keyword),
//!     ("=", CodeTokenKind::Punctuation),
//!     ("App", CodeTokenKind::Type),
//!     ("::", CodeTokenKind::Punctuation),
//!     ("();", CodeTokenKind::Punctuation),
//!     ("// one", CodeTokenKind::Comment),
//!   ]
//! );
//! ```

use std::ops::Range;

/// What a token of code is, which says its color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CodeTokenKind {
  /// What is none of the others: names, a `.`, white space.
  Plain,
  /// A word of the language: `fn`, `let`, `match`.
  Keyword,
  /// The name of a type, a trait or a module: a name before `::` (`lurq::`, `ButtonVariant::`), or one that starts
  /// with a capital (`App`, `String`).
  Type,
  /// Brackets, operators and the other marks between names: `(`, `::`, `=`, `;`, and `_` alone.
  Punctuation,
  /// A string, from its quote to its quote.
  String,
  /// A number.
  Number,
  /// A comment: to the end of its line, or from `/*` to `*/`.
  Comment,
}

/// A run of code of one kind: its bytes in the code and what it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeToken {
  pub range: Range<usize>,
  pub kind: CodeTokenKind,
}

/// Whether `language` is one whose code is told apart in tokens; the code of another is one plain token.
pub fn knows(language: &str) -> bool {
  matches!(language.trim().to_ascii_lowercase().as_str(), "rust" | "rs")
}

/// The tokens of `code` in `language`, in order and covering it whole: neighbours of one kind are one token.
pub fn tokens(code: &str, language: &str) -> Vec<CodeToken> {
  let mut tokens = Vec::new();
  if code.is_empty() {
    return tokens;
  }
  if !knows(language) {
    tokens.push(CodeToken {
      range: 0..code.len(),
      kind: CodeTokenKind::Plain,
    });
    return tokens;
  }
  let mut at = 0;
  while at < code.len() {
    let rest = &code[at..];
    let (len, kind) = if let Some(comment) = rest.strip_prefix("//") {
      (
        comment.find('\n').map_or(rest.len(), |index| index + 2),
        CodeTokenKind::Comment,
      )
    } else if let Some(comment) = rest.strip_prefix("/*") {
      (
        comment.find("*/").map_or(rest.len(), |index| index + 4),
        CodeTokenKind::Comment,
      )
    } else {
      let ch = rest.chars().next().expect("not at the end");
      if ch == '"' {
        (quoted_len(rest), CodeTokenKind::String)
      } else if let Some(len) = char_literal_len(rest) {
        (len, CodeTokenKind::String)
      } else if ch.is_ascii_digit() {
        let len = rest
          .char_indices()
          .find(|(_, ch)| !ch.is_ascii_digit() && *ch != '.')
          .map_or(rest.len(), |(index, _)| index);
        (len, CodeTokenKind::Number)
      } else if ch == '_' || ch.is_ascii_alphabetic() {
        let len = rest
          .char_indices()
          .find(|(_, ch)| *ch != '_' && !ch.is_ascii_alphanumeric())
          .map_or(rest.len(), |(index, _)| index);
        let word = &rest[..len];
        let kind = if word == "_" {
          CodeTokenKind::Punctuation
        } else if is_rust_keyword(word) {
          CodeTokenKind::Keyword
        } else if rest[len..].starts_with("::") || word.starts_with(|first: char| first.is_ascii_uppercase()) {
          CodeTokenKind::Type
        } else {
          CodeTokenKind::Plain
        };
        (len, kind)
      } else if ch == '.' || ch.is_whitespace() {
        (ch.len_utf8(), CodeTokenKind::Plain)
      } else {
        (ch.len_utf8(), CodeTokenKind::Punctuation)
      }
    };
    match tokens.last_mut() {
      Some(CodeToken { range, kind: last }) if *last == kind => range.end = at + len,
      _ => tokens.push(CodeToken {
        range: at..at + len,
        kind,
      }),
    }
    at += len;
  }
  tokens
}

/// The length of the character `text` starts with (`'a'`, `'\n'`), where it starts with one and not with a
/// lifetime (`'a`).
fn char_literal_len(text: &str) -> Option<usize> {
  let mut chars = text.char_indices();
  if chars.next()?.1 != '\'' {
    return None;
  }
  let (_, first) = chars.next()?;
  if first == '\\' {
    // An escape: to the closing quote, a few characters on at most.
    return text
      .char_indices()
      .skip(3)
      .take(10)
      .find(|(_, ch)| *ch == '\'')
      .map(|(index, _)| index + 1);
  }
  let (index, closing) = chars.next()?;
  (closing == '\'').then_some(index + 1)
}

/// The length of the string `text` starts with, to its closing quote or to the end of `text`.
fn quoted_len(text: &str) -> usize {
  let mut escaped = false;
  for (index, ch) in text.char_indices().skip(1) {
    if escaped {
      escaped = false;
      continue;
    }
    if ch == '\\' {
      escaped = true;
      continue;
    }
    if ch == '"' {
      return index + ch.len_utf8();
    }
  }
  text.len()
}

fn is_rust_keyword(token: &str) -> bool {
  matches!(
    token,
    "as"
      | "async"
      | "await"
      | "break"
      | "const"
      | "continue"
      | "crate"
      | "else"
      | "enum"
      | "false"
      | "fn"
      | "for"
      | "if"
      | "impl"
      | "in"
      | "let"
      | "loop"
      | "match"
      | "mod"
      | "move"
      | "mut"
      | "pub"
      | "ref"
      | "return"
      | "self"
      | "Self"
      | "static"
      | "struct"
      | "super"
      | "trait"
      | "true"
      | "type"
      | "unsafe"
      | "use"
      | "where"
      | "while"
  )
}

#[cfg(test)]
mod tests {
  use super::{CodeTokenKind::*, tokens};

  /// The runs of `code` that are not plain, with what they are.
  fn kinds(code: &str) -> Vec<(&str, super::CodeTokenKind)> {
    tokens(code, "rust")
      .into_iter()
      .filter(|token| token.kind != Plain)
      .map(|token| (&code[token.range], token.kind))
      .collect()
  }

  #[test]
  fn a_line_of_the_gallerys_reads_as_it_did_there_and_more() {
    // What the gallery's own view colored: a name before `::` in the color of information, a string in the
    // color of success, `_` and the marks quiet, a comment quiet; and what it did not: the words of the language,
    // a type by its capital, a number.
    let code = "let save = Button::new(\"Save\").variant(ButtonVariant::Ghost).size(_); // 2 of them";
    assert_eq!(
      kinds(code),
      [
        ("let", Keyword),
        ("=", Punctuation),
        ("Button", Type),
        ("::", Punctuation),
        ("(", Punctuation),
        ("\"Save\"", String),
        (")", Punctuation),
        ("(", Punctuation),
        ("ButtonVariant", Type),
        ("::", Punctuation),
        ("Ghost", Type),
        (")", Punctuation),
        ("(_);", Punctuation),
        ("// 2 of them", Comment),
      ]
    );
  }

  #[test]
  fn a_module_before_its_path_is_a_type_and_a_character_is_a_string() {
    assert_eq!(
      kinds("use lurq::ui;"),
      [
        ("use", Keyword),
        ("lurq", Type),
        ("::", Punctuation),
        (";", Punctuation)
      ]
    );
    assert_eq!(
      kinds("let c = '\n'; let q = '\'';"),
      [
        ("let", Keyword),
        ("=", Punctuation),
        ("'\n'", String),
        (";", Punctuation),
        ("let", Keyword),
        ("=", Punctuation),
        ("'\''", String),
        (";", Punctuation),
      ]
    );
    // A lifetime is no character.
    assert!(kinds("fn f<'a>(x: &'a str)").iter().all(|(_, kind)| *kind != String));
    assert_eq!(kinds("/* a\nb */ 1"), [("/* a\nb */", Comment), ("1", Number)]);
  }
}
