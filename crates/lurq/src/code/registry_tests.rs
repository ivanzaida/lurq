//! The registry: one highlighter for each language, registered, replaced and taken away, and what they gave kept.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use super::{
  CodeHighlighter, CodeLanguage, CodeToken, highlighter, knows, register_highlighter,
  register_highlighter_with_aliases, remove_highlighter, token, tokens,
};

/// A highlighter that calls every text a keyword, and counts how often it is asked.
fn counting(name: &'static str) -> (CodeHighlighter, Arc<AtomicUsize>) {
  let asked = Arc::new(AtomicUsize::new(0));
  let counter = asked.clone();
  let highlighter = CodeHighlighter::new(move |code: &str| {
    counter.fetch_add(1, Ordering::SeqCst);
    vec![CodeToken::new(0..code.len(), name)]
  });
  (highlighter, asked)
}

#[test]
fn lurq_knows_its_languages_by_their_names_and_aliases() {
  for name in [
    "rust",
    "rs",
    "Rust",
    "toml",
    "json",
    "jsonc",
    "javascript",
    "js",
    "typescript",
    "ts",
    "shell",
    "bash",
    "sh",
    "python",
    "py",
    "css",
    "html",
    "xml",
    "svg",
    "yaml",
    "yml",
    "sql",
    "markdown",
    "md",
  ] {
    assert!(knows(name), "{name}");
  }
  // What a fence of Markdown says after the language is no part of it.
  assert!(knows("rust,ignore"));
  assert!(knows("rust no_run"));
  assert!(!knows("cobol-of-the-tests"));
  assert!(tokens("let x", "cobol-of-the-tests").is_empty());
}

#[test]
fn registering_a_language_gives_its_code_colors_and_replacing_one_takes_the_new() {
  let (first, asked_first) = counting(token::KEYWORD);
  register_highlighter_with_aliases("lang-of-the-registry", &["lotr"], first);
  assert_eq!(tokens("a b", "lotr")[0].kind, token::KEYWORD);
  // Asked again for the same code, it is not asked: what it gave is kept.
  let _ = tokens("a b", "lang-of-the-registry");
  let _ = tokens("a b", "LOTR");
  assert_eq!(asked_first.load(Ordering::SeqCst), 1);

  // A new one under an alias replaces the language's, under every name of it.
  let (second, asked_second) = counting(token::STRING);
  register_highlighter("lotr", second);
  assert_eq!(tokens("a b", "lang-of-the-registry")[0].kind, token::STRING);
  assert_eq!(tokens("a b", "lotr")[0].kind, token::STRING);
  assert_eq!(asked_second.load(Ordering::SeqCst), 1);
  assert_eq!(asked_first.load(Ordering::SeqCst), 1);

  assert!(remove_highlighter("lotr"));
  assert!(!knows("lang-of-the-registry"));
  assert!(tokens("a b", "lotr").is_empty());
  assert!(!remove_highlighter("lotr"));
}

#[test]
fn lurqs_own_is_replaced_as_any_other() {
  // A language of lurq's, changed and registered under a name of its own, so that the other tests see Rust as it is.
  register_highlighter_with_aliases(
    "rust-of-the-tests",
    &[],
    CodeLanguage::rust().words(token::KEYWORD, &["yeet"]),
  );
  let found = tokens("yeet x", "rust-of-the-tests");
  assert_eq!(found[0], CodeToken::new(0..4, token::KEYWORD));
  assert!(
    tokens("yeet x", "rust")
      .iter()
      .all(|token| token.kind != token::KEYWORD)
  );
}

#[test]
fn a_render_follows_its_languages_highlighter() {
  // What a render reads is the language's signal: reading it twice gives the same highlighter, replacing it a new.
  let (one, _) = counting(token::KEYWORD);
  register_highlighter("lang-of-the-follow", one.clone());
  assert_eq!(highlighter("lang-of-the-follow"), Some(one.clone()));
  let (two, _) = counting(token::KEYWORD);
  register_highlighter("lang-of-the-follow", two.clone());
  assert_eq!(highlighter("lang-of-the-follow"), Some(two));
  assert_ne!(highlighter("lang-of-the-follow"), Some(one));
}
