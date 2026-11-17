//! A tokenizer of another crate plugged into lurq: `syntect`, which reads the grammars of Sublime Text, as an app
//! would bring it. It is the gallery's alone, as an example; lurq has no dependency on it.
//!
//! What it does is all an adapter has to: run syntect over the code line by line, and name each run by the scope
//! syntect gives it (`keyword.control.rust`, `string.quoted.double`), mapped to lurq's dotted names, so that the
//! theme's code colors draw it.

use std::{
  collections::HashMap,
  sync::{LazyLock, Mutex},
};

use lurq::code::{CodeToken, CodeTokenizer, Tokenizer, token};
use syntect::parsing::{ParseState, Scope, ScopeStack, SyntaxReference, SyntaxSet};

/// The grammars syntect comes with, loaded once.
static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);

/// What each of syntect's scopes is named in lurq, or `None` for one whose run is plain.
static NAMES: LazyLock<Mutex<HashMap<Scope, Option<&'static str>>>> = LazyLock::new(Default::default);

/// The scopes of TextMate's grammars, from the more particular, and the names lurq gives them.
const SCOPES: &[(&str, &str)] = &[
  ("comment.line.documentation", token::COMMENT_DOC),
  ("comment.block.documentation", token::COMMENT_DOC),
  ("comment", token::COMMENT),
  ("constant.character.escape", token::STRING_ESCAPE),
  ("string.regexp", token::REGEX),
  ("string", token::STRING),
  ("constant.numeric", token::NUMBER),
  ("constant.language", token::CONSTANT_LANGUAGE),
  ("constant", token::CONSTANT),
  ("keyword.control", token::KEYWORD_CONTROL),
  ("keyword.operator", token::OPERATOR),
  ("keyword", token::KEYWORD),
  // `let`, `fn` and `u32` are all `storage.type` in syntect's Rust: a word of the language.
  ("storage.modifier.lifetime", token::LABEL),
  ("storage", token::KEYWORD),
  ("support.macro", token::FUNCTION_MACRO),
  ("entity.name.macro", token::FUNCTION_MACRO),
  ("entity.name.function", token::FUNCTION),
  ("support.function", token::FUNCTION),
  ("variable.function", token::FUNCTION),
  ("entity.name.namespace", token::NAMESPACE),
  ("entity.name.module", token::NAMESPACE),
  ("entity.name.tag", token::TAG),
  ("entity.other.attribute-name", token::ATTRIBUTE),
  ("meta.annotation", token::ATTRIBUTE),
  ("entity.name.label", token::LABEL),
  ("entity.name", token::TYPE),
  ("support.type.property-name", token::PROPERTY),
  ("support.type", token::TYPE),
  ("support.class", token::TYPE),
  ("variable.language", token::VARIABLE_BUILTIN),
  ("variable.other.constant", token::CONSTANT),
  ("variable.parameter", token::VARIABLE),
  ("variable", token::VARIABLE),
  ("punctuation", token::PUNCTUATION),
  ("markup.heading", token::MARKUP_HEADING),
  ("markup.bold", token::MARKUP_BOLD),
  ("markup.italic", token::MARKUP_ITALIC),
  ("markup.raw", token::MARKUP_CODE),
  ("markup.underline.link", token::MARKUP_LINK),
  ("markup.list", token::MARKUP_LIST),
  ("markup.quote", token::MARKUP_QUOTE),
];

/// lurq's name for a scope of syntect's: the first of [`SCOPES`] the scope is, or is inside.
fn name_of(scope: Scope) -> Option<&'static str> {
  let mut names = NAMES.lock().unwrap();
  *names.entry(scope).or_insert_with(|| {
    let scope = scope.build_string();
    SCOPES
      .iter()
      .find(|(prefix, _)| scope == *prefix || scope.starts_with(&format!("{prefix}.")))
      .map(|(_, name)| *name)
  })
}

/// The name of a run inside `scopes`: the innermost scope that has one. The marks that open and close a construct
/// (`punctuation.definition.comment`, a string's quotes) are the construct's, where a scope round them names it.
fn name_inside(scopes: &[Scope]) -> Option<&'static str> {
  let mut marks = None;
  for scope in scopes.iter().rev() {
    let Some(name) = name_of(*scope) else {
      continue;
    };
    if name == token::PUNCTUATION && definition_of(*scope) {
      marks.get_or_insert(name);
      continue;
    }
    if name == token::PUNCTUATION && marks.is_some() {
      continue;
    }
    return Some(name);
  }
  marks
}

/// Whether `scope` is a mark that opens or closes a construct: `punctuation.definition.…`.
fn definition_of(scope: Scope) -> bool {
  static DEFINITION: LazyLock<Scope> = LazyLock::new(|| Scope::new("punctuation.definition").expect("a scope"));
  DEFINITION.is_prefix_of(scope)
}

/// A tokenizer of syntect's, for one of its grammars.
pub struct Syntect {
  syntax: &'static SyntaxReference,
}

impl Syntect {
  /// The tokenizer of syntect's grammar for `language` (a name, or a file's extension: `rust`, `rs`, `go`), where
  /// syntect has one.
  pub fn for_language(language: &str) -> Option<CodeTokenizer> {
    let syntax = SYNTAXES.find_syntax_by_token(language)?;
    Some(CodeTokenizer::new(Self { syntax }))
  }
}

impl Tokenizer for Syntect {
  fn tokens(&self, code: &str) -> Vec<CodeToken> {
    let mut tokens: Vec<CodeToken> = Vec::new();
    let mut state = ParseState::new(self.syntax);
    let mut scopes = ScopeStack::new();
    let mut start = 0;
    for line in code.split_inclusive('\n') {
      let Ok(ops) = state.parse_line(line, &SYNTAXES) else {
        break;
      };
      let mut at = 0;
      let mut push = |from: usize, to: usize, scopes: &ScopeStack| {
        let Some(name) = name_inside(scopes.as_slice()) else {
          return;
        };
        let range = start + from..start + to.min(line.trim_end_matches('\n').len());
        if range.is_empty() {
          return;
        }
        match tokens.last_mut() {
          Some(last) if last.kind == name && last.range.end == range.start => last.range.end = range.end,
          _ => tokens.push(CodeToken::new(range, name)),
        }
      };
      for (offset, op) in ops {
        push(at, offset, &scopes);
        at = offset;
        if scopes.apply(&op).is_err() {
          break;
        }
      }
      push(at, line.len(), &scopes);
      start += line.len();
    }
    tokens
  }
}

/// The languages the gallery registers syntect for at start: lurq has none of its own for them.
pub const LANGUAGES: [&str; 3] = ["go", "c", "java"];

/// Registers syntect's grammars for [`LANGUAGES`]: a block of Go or C in the gallery is told apart by syntect, in the
/// theme's code colors.
pub fn register(app: &lurq::app::App) {
  for language in LANGUAGES {
    if let Some(tokenizer) = Syntect::for_language(language) {
      app.register_tokenizer(language, tokenizer);
    }
  }
}

/// syntect's tokenizer for Rust, made once: the gallery's page switches a block to it.
pub fn rust() -> CodeTokenizer {
  static RUST: LazyLock<Option<CodeTokenizer>> = LazyLock::new(|| Syntect::for_language("rust"));
  RUST.clone().expect("syntect has a grammar of Rust")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn syntects_scopes_are_lurqs_names() {
    let code = "fn main() {\n  let n = 42; // the answer\n  let name = \"lurq\";\n  println!(\"{n}\");\n}";
    let tokens = rust().tokens(code);
    let named = |text: &str| {
      tokens
        .iter()
        .find(|token| &code[token.range.clone()] == text)
        .map(|token| token.kind.as_str())
    };
    assert_eq!(named("42"), Some(token::NUMBER));
    assert_eq!(named("// the answer"), Some(token::COMMENT));
    assert!(named("let").is_some_and(|name| name.starts_with(token::KEYWORD)));
    assert!(named("main").is_some_and(|name| name.starts_with(token::FUNCTION)));
    assert_eq!(named("\"lurq\""), Some(token::STRING), "a string with its quotes");
  }

  #[test]
  fn a_language_lurq_does_not_know_is_syntects() {
    register(&lurq::app::App::new());
    let found = lurq::code::tokens("package main\n\nfunc main() {}", "go");
    assert!(!found.is_empty());
  }
}
