//! The tokens of a piece of code, for drawing it in colors: what `ui::CodeBlock`, `ui::InlineCode`, the code blocks
//! of Markdown and an editable code field color their code by.
//!
//! A token is a run of the code and a **name** with a dotted hierarchy, as TextMate and tree-sitter name them:
//! `keyword`, `keyword.control`, `string.escape`, `function.macro`. The names lurq's own tokenizers give are
//! constants of [`token`]; any other name is allowed, and its color is looked up dropping the tail
//! ([`lookup_names`]: `keyword.control.rust`, then `keyword.control`, then `keyword`, then plain), in the theme's
//! code colors (`app::theme::CodeScheme`) or a block's own.
//!
//! What tells a language's code apart is its **tokenizer**, one for each language: lurq registers its own for the
//! languages it knows ([`CodeLanguage`], a table each of them is written in), and an app registers its own for any
//! language, in place of lurq's or for one lurq does not know ([`register_tokenizer`]). A language with none is
//! plain text.
//!
//! ```
//! use lurq::code::{CodeToken, register_tokenizer, tokens, token};
//!
//! let code = "let app = App::new(); // one";
//! let named: Vec<_> = tokens(code, "rust")
//!   .iter()
//!   .map(|token| (&code[token.range.clone()], token.kind.as_str()))
//!   .collect();
//! assert_eq!(named[0], ("let", token::KEYWORD));
//! assert_eq!(named[2], ("App", token::TYPE));
//! assert_eq!(named.last(), Some(&("// one", token::COMMENT)));
//!
//! // An app's own tokenizer for a language of its own: every word that starts with `$` is a variable.
//! register_tokenizer("dollars", |code: &str| {
//!   let mut found = Vec::new();
//!   let mut at = 0;
//!   for word in code.split(' ') {
//!     if word.starts_with('$') {
//!       found.push(CodeToken::new(at..at + word.len(), token::VARIABLE));
//!     }
//!     at += word.len() + 1;
//!   }
//!   found
//! });
//! assert_eq!(tokens("pay $sum now", "dollars")[0].range, 4..8);
//! ```

mod cache;
mod language;
mod languages;
mod lines;
#[cfg(test)]
mod lines_tests;
mod registry;
#[cfg(test)]
mod registry_tests;
mod scan;
#[cfg(test)]
mod tests;

use std::ops::Range;

pub use language::{BlockComment, CodeLanguage, CodeMode, KeyRule, NumberForms, StringForm};
pub(crate) use lines::LineCache;
pub use lines::{BufferId, CodeEdit, CodePoint, LineState};
pub use registry::{
  CodeTokenizer, Tokenizer, TokenizerHandle, knows, languages, register_tokenizer, register_tokenizer_with_aliases,
  remove_tokenizer, tokenizer, tokens,
};
pub use scan::ScanState;

/// The name of a kind of token: an interned name, cheap to copy and compare, that reads as a `&str`.
pub type TokenName = crate::app::theme::RoleName;

/// The names lurq's own tokenizers give their tokens, and the theme's code colors know. A tokenizer may give
/// any other name, more particular (`keyword.control.rust`) or of its own (`markup.heading`).
pub mod token {
  /// A word of the language: `fn`, `let`, `select`.
  pub const KEYWORD: &str = "keyword";
  /// A word that changes the flow: `if`, `return`, `match`.
  pub const KEYWORD_CONTROL: &str = "keyword.control";
  /// A string, from its quote to its quote.
  pub const STRING: &str = "string";
  /// An escape in a string: `\n`, `\u{1F600}`.
  pub const STRING_ESCAPE: &str = "string.escape";
  /// A number, with its suffix or its unit: `0x1F`, `2.5e3`, `10px`.
  pub const NUMBER: &str = "number";
  /// A comment.
  pub const COMMENT: &str = "comment";
  /// A comment that documents what follows it: `///`, `/** */`.
  pub const COMMENT_DOC: &str = "comment.doc";
  /// The name of a type, a trait or a class: `App`, `String`.
  pub const TYPE: &str = "type";
  /// A type the language has of itself: `u32`, `bool`, `VARCHAR`.
  pub const TYPE_BUILTIN: &str = "type.builtin";
  /// A function where it is called or declared: `main`, `count(`.
  pub const FUNCTION: &str = "function";
  /// A macro of Rust: `println!`.
  pub const FUNCTION_MACRO: &str = "function.macro";
  /// A variable: `$HOME`.
  pub const VARIABLE: &str = "variable";
  /// A variable the language gives: `self`, `this`.
  pub const VARIABLE_BUILTIN: &str = "variable.builtin";
  /// A constant: `MAX_LEN`, `#fff`.
  pub const CONSTANT: &str = "constant";
  /// A constant of the language: `true`, `null`, `None`.
  pub const CONSTANT_LANGUAGE: &str = "constant.language";
  /// A key or a property: a key of JSON, YAML or TOML, a property of CSS.
  pub const PROPERTY: &str = "property";
  /// An attribute or a decorator: `#[derive`, `@cached`, an attribute of a tag.
  pub const ATTRIBUTE: &str = "attribute";
  /// An operator: `=`, `+`, `=>`.
  pub const OPERATOR: &str = "operator";
  /// Brackets and the other marks between names: `(`, `::`, `;`.
  pub const PUNCTUATION: &str = "punctuation";
  /// The name of a tag of HTML or XML, or an element a selector of CSS names.
  pub const TAG: &str = "tag";
  /// A regular expression: `/a+b/g`.
  pub const REGEX: &str = "regex";
  /// The name of a module or a table: `lurq::`, `[dependencies]`.
  pub const NAMESPACE: &str = "namespace";
  /// A label or a lifetime: `'a`, `'outer`.
  pub const LABEL: &str = "label";
  /// A heading of Markdown.
  pub const MARKUP_HEADING: &str = "markup.heading";
  /// Strong text of Markdown.
  pub const MARKUP_BOLD: &str = "markup.bold";
  /// Emphasis of Markdown.
  pub const MARKUP_ITALIC: &str = "markup.italic";
  /// Code in Markdown: a span or a fenced block.
  pub const MARKUP_CODE: &str = "markup.code";
  /// The text of a link of Markdown.
  pub const MARKUP_LINK: &str = "markup.link";
  /// The marker of an item of a list of Markdown.
  pub const MARKUP_LIST: &str = "markup.list";
  /// A quote of Markdown.
  pub const MARKUP_QUOTE: &str = "markup.quote";
}

/// A run of code and what it is: its bytes in the code and its name. What no token covers is plain.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CodeToken {
  pub range: Range<usize>,
  pub kind: TokenName,
}

impl CodeToken {
  /// A token of `range` named `kind`: one of [`token`]'s names, or any other.
  pub fn new(range: Range<usize>, kind: impl Into<TokenName>) -> Self {
    Self {
      range,
      kind: kind.into(),
    }
  }
}

/// The names a token's color is looked up by, from its own to its first part: `keyword.control.rust`,
/// `keyword.control`, `keyword`.
pub fn lookup_names(name: &str) -> impl Iterator<Item = &str> {
  let mut next = Some(name);
  std::iter::from_fn(move || {
    let name = next?;
    next = name.rfind('.').map(|dot| &name[..dot]).filter(|rest| !rest.is_empty());
    Some(name)
  })
}

/// `tokens` made fit to color `code` with: in order, each inside the code on its characters' boundaries, none empty
/// and none over another (of two that overlap, the one that starts first stays).
pub(crate) fn normalized(code: &str, mut tokens: Vec<CodeToken>) -> Vec<CodeToken> {
  tokens.retain(|token| {
    token.range.start < token.range.end
      && token.range.end <= code.len()
      && code.is_char_boundary(token.range.start)
      && code.is_char_boundary(token.range.end)
  });
  if !tokens.is_sorted_by_key(|token| token.range.start) {
    tokens.sort_by_key(|token| token.range.start);
  }
  let mut end = 0;
  tokens.retain(|token| {
    let keep = token.range.start >= end;
    if keep {
      end = token.range.end;
    }
    keep
  });
  tokens
}

/// The tokens of each line of `code`, as ranges of the line: a token over several lines is a token on each.
pub fn tokens_by_line(code: &str, tokens: &[CodeToken]) -> Vec<Vec<CodeToken>> {
  let mut lines = Vec::new();
  let mut next = 0;
  let mut start = 0;
  for line in code.split('\n') {
    let end = start + line.len();
    let mut own = Vec::new();
    while next < tokens.len() && tokens[next].range.start < end + 1 {
      let token = &tokens[next];
      let from = token.range.start.max(start);
      let to = token.range.end.min(end);
      if from < to {
        own.push(CodeToken {
          range: from - start..to - start,
          kind: token.kind,
        });
      }
      if token.range.end > end + 1 {
        // It goes on to the next line.
        break;
      }
      next += 1;
    }
    lines.push(own);
    start = end + 1;
  }
  lines
}
