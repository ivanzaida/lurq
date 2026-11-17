//! The scanner, language by language: each fixture in `fixtures/` is tokenized and compared with its listing,
//! `fixtures/<language>.tokens`, one token a line: its name, a tab, its text. `LURQ_BLESS=1` writes the listings
//! anew, to be read before they are committed.

use std::path::PathBuf;

use super::{CodeLanguage, CodeToken, ScanState, token, tokens_by_line};

/// The tokens of `code` in `language`, each as its name and its text.
fn named(language: &CodeLanguage, code: &str) -> Vec<(String, String)> {
  language
    .tokens(code)
    .into_iter()
    .map(|token| (token.kind.as_str().to_owned(), code[token.range].to_owned()))
    .collect()
}

fn listing(language: &CodeLanguage, code: &str) -> String {
  named(language, code)
    .into_iter()
    .map(|(name, text)| format!("{name}\t{text:?}\n"))
    .collect()
}

fn fixture(name: &str, language: CodeLanguage) {
  let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/code/fixtures");
  let code = std::fs::read_to_string(folder.join(format!("{name}.txt")))
    .expect("the fixture")
    .replace("\r\n", "\n");
  let found = listing(&language, &code);
  let expected_path = folder.join(format!("{name}.tokens"));
  if std::env::var_os("LURQ_BLESS").is_some() {
    std::fs::write(&expected_path, &found).expect("the listing written");
    return;
  }
  let expected = std::fs::read_to_string(&expected_path)
    .expect("the listing: run with LURQ_BLESS=1 and read it")
    .replace("\r\n", "\n");
  if found != expected {
    let first = found
      .lines()
      .zip(expected.lines())
      .position(|(found, expected)| found != expected)
      .unwrap_or(found.lines().count().min(expected.lines().count()));
    panic!(
      "{name}: the tokens differ from the listing at its line {}: found {:?}, the listing has {:?}",
      first + 1,
      found.lines().nth(first),
      expected.lines().nth(first)
    );
  }
  // Read from any line with the state it starts in, the code has the same tokens.
  let states = language.states(&code);
  let mut start = 0;
  for (index, line) in code.split('\n').enumerate() {
    let mut state = states[index].clone();
    let by_line = language.line_tokens(line, &mut state);
    let whole: Vec<CodeToken> = language
      .tokens(&code)
      .into_iter()
      .filter(|token| token.range.start >= start && token.range.end <= start + line.len())
      .map(|token| CodeToken::new(token.range.start - start..token.range.end - start, token.kind))
      .collect();
    let inside: Vec<CodeToken> = by_line
      .into_iter()
      .filter(|token| whole.iter().any(|held| held.range.start == token.range.start))
      .collect();
    assert!(
      inside.iter().all(|token| whole.contains(token)),
      "{name}: line {} read alone from its state differs",
      index + 1
    );
    start += line.len() + 1;
  }
}

#[test]
fn rust() {
  fixture("rust", CodeLanguage::rust());
}

#[test]
fn toml() {
  fixture("toml", CodeLanguage::toml());
}

#[test]
fn json() {
  fixture("json", CodeLanguage::json());
}

#[test]
fn javascript() {
  fixture("js", CodeLanguage::javascript());
}

#[test]
fn typescript() {
  fixture("ts", CodeLanguage::typescript());
}

#[test]
fn shell() {
  fixture("shell", CodeLanguage::shell());
}

#[test]
fn python() {
  fixture("python", CodeLanguage::python());
}

#[test]
fn css() {
  fixture("css", CodeLanguage::css());
}

#[test]
fn html() {
  fixture("html", CodeLanguage::html());
}

#[test]
fn xml() {
  fixture("xml", CodeLanguage::xml());
}

#[test]
fn yaml() {
  fixture("yaml", CodeLanguage::yaml());
}

#[test]
fn sql() {
  fixture("sql", CodeLanguage::sql());
}

#[test]
fn markdown() {
  fixture("markdown", CodeLanguage::markdown());
}

#[test]
fn a_name_is_looked_up_dropping_its_tail() {
  let names: Vec<&str> = super::lookup_names("keyword.control.rust").collect();
  assert_eq!(names, ["keyword.control.rust", "keyword.control", "keyword"]);
  assert_eq!(super::lookup_names("string").collect::<Vec<_>>(), ["string"]);
}

#[test]
fn a_tokenizers_tokens_are_made_fit_to_draw() {
  let code = "héllo wörld";
  let tokens = super::normalized(
    code,
    vec![
      CodeToken::new(6..13, token::STRING),
      // Off a character's boundary, empty, past the end, and over another.
      CodeToken::new(0..2, token::KEYWORD),
      CodeToken::new(3..3, token::KEYWORD),
      CodeToken::new(10..40, token::KEYWORD),
      CodeToken::new(0..1, token::KEYWORD),
      CodeToken::new(0..3, token::NUMBER),
    ],
  );
  assert_eq!(
    tokens,
    [
      CodeToken::new(0..1, token::KEYWORD),
      CodeToken::new(6..13, token::STRING)
    ]
  );
}

#[test]
fn a_token_over_lines_is_a_token_on_each() {
  let code = "a /* one\ntwo */ b\nc";
  let tokens = CodeLanguage::rust().tokens(code);
  let lines = tokens_by_line(code, &tokens);
  assert_eq!(lines.len(), 3);
  assert_eq!(&"a /* one"[lines[0][0].range.clone()], "/* one");
  assert_eq!(&"two */ b"[lines[1][0].range.clone()], "two */");
  assert!(lines[2].is_empty());
}

#[test]
fn a_line_starts_inside_what_an_earlier_one_opened() {
  let rust = CodeLanguage::rust();
  let states = rust.states("let s = r#\"one\ntwo\"#; /* a\nb */\nc");
  assert!(states[0].is_clear());
  assert!(!states[1].is_clear());
  assert!(!states[2].is_clear());
  assert!(states[3].is_clear());
  // The second line read alone, from its state: the rest of the raw string, then a comment that goes on.
  let mut state = states[1].clone();
  let tokens = rust.line_tokens("two\"#; /* a", &mut state);
  assert_eq!(tokens[0], CodeToken::new(0..5, token::STRING));
  assert_eq!(state, states[2]);
  let _ = ScanState::default();
}

#[test]
fn a_language_of_the_apps_own_is_a_table() {
  let ini = CodeLanguage::new("ini")
    .line_comments(&[";"])
    .strings(vec![super::StringForm::new("\"", "\"")])
    .words(token::CONSTANT_LANGUAGE, &["true"])
    .keys(super::KeyRule::Equals)
    .sections(true);
  assert_eq!(
    named(&ini, "[server]\nhost = \"a\" ; the host\nsecure = true"),
    [
      ("punctuation", "["),
      ("namespace", "server"),
      ("punctuation", "]"),
      ("property", "host"),
      ("operator", "="),
      ("string", "\"a\""),
      ("comment", "; the host"),
      ("property", "secure"),
      ("operator", "="),
      ("constant.language", "true"),
    ]
    .map(|(name, text)| (name.to_owned(), text.to_owned()))
  );
}

#[test]
fn every_language_reads_any_text_without_a_panic() {
  let texts = [
    "",
    "\n\n",
    "'",
    "\"",
    "r#\"",
    "/*",
    "<",
    "<a b='",
    "```",
    "`",
    "\\",
    "#[",
    "0x",
    "1e",
    "$",
    "${",
    "@",
    "'\\",
    "é'ü\"ö/*∞*/",
    "a: |\n  b\nc: d",
    "[",
    "[[a",
    "**",
    "[a](",
  ];
  for language in super::languages::all() {
    for text in texts {
      let tokens = language.tokens(text);
      assert!(
        tokens
          .iter()
          .all(|token| text.is_char_boundary(token.range.start) && text.is_char_boundary(token.range.end)),
        "{}: {text:?}",
        language.name()
      );
    }
  }
}
