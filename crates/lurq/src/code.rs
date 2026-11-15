//! The tokens of a piece of code, for drawing it in colors: what a code block of Markdown and `ui::CodeBlock` color
//! their code by. It knows Rust (`rust`, `rs`); the code of a language it does not know is one plain token.
//!
//! ```
//! use lurq::code::{CodeTokenKind, tokens};
//!
//! let code = "let x = 1; // one";
//! let kinds: Vec<_> = tokens(code, "rust")
//!   .into_iter()
//!   .filter(|token| token.kind != CodeTokenKind::Plain)
//!   .map(|token| (&code[token.range], token.kind))
//!   .collect();
//! assert_eq!(
//!   kinds,
//!   [("let", CodeTokenKind::Keyword), ("1", CodeTokenKind::Number), ("// one", CodeTokenKind::Comment)]
//! );
//! ```

use std::ops::Range;

/// What a token of code is, which says its color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CodeTokenKind {
  /// What is none of the others: names, punctuation, white space.
  Plain,
  /// A word of the language: `fn`, `let`, `match`.
  Keyword,
  /// A string, from its quote to its quote.
  String,
  /// A number.
  Number,
  /// A comment, to the end of its line.
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
    } else {
      let ch = rest.chars().next().expect("not at the end");
      if ch == '"' {
        (quoted_len(rest), CodeTokenKind::String)
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
        let kind = if is_rust_keyword(&rest[..len]) {
          CodeTokenKind::Keyword
        } else {
          CodeTokenKind::Plain
        };
        (len, kind)
      } else {
        (ch.len_utf8(), CodeTokenKind::Plain)
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
