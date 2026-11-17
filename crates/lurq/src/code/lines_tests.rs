//! A tokenizer with state: lines re-tokenized from an edit until they agree, the state a line starts in, an edit as
//! tree-sitter has it, and a tokenizer that says its tokens are stale.

use std::sync::{
  Arc,
  atomic::{AtomicBool, AtomicUsize, Ordering},
};

use super::{
  CodeEdit, CodeLanguage, CodePoint, CodeToken, CodeTokenizer, LineCache, LineState, Tokenizer, normalized, token,
};

/// lurq's Rust, counting the lines it is given.
struct Counting {
  rust: CodeLanguage,
  lines: Arc<AtomicUsize>,
}

impl Tokenizer for Counting {
  fn tokens(&self, code: &str) -> Vec<CodeToken> {
    self.lines.fetch_add(code.split('\n').count(), Ordering::SeqCst);
    self.rust.tokens(code)
  }

  fn tokens_from(&self, lines: &[&str], start: &LineState) -> Option<Vec<(Vec<CodeToken>, LineState)>> {
    self.lines.fetch_add(lines.len(), Ordering::SeqCst);
    self.rust.tokens_from(lines, start)
  }
}

fn counting() -> (CodeTokenizer, Arc<AtomicUsize>) {
  let lines = Arc::new(AtomicUsize::new(0));
  let tokenizer = CodeTokenizer::new(Counting {
    rust: CodeLanguage::rust(),
    lines: lines.clone(),
  });
  (tokenizer, lines)
}

fn ten_thousand() -> String {
  (0..10_000)
    .map(|index| format!("fn step_{index}(ctx: &mut Ctx) -> u32 {{ {index} * 2 }} // step"))
    .collect::<Vec<_>>()
    .join("\n")
}

/// What the whole text gives, tokenized from its start.
fn whole(code: &str) -> Vec<CodeToken> {
  normalized(code, CodeLanguage::rust().tokens(code))
}

#[test]
fn a_typed_character_re_tokenizes_a_few_lines_of_ten_thousand() {
  let (tokenizer, lines) = counting();
  let mut kept = LineCache::default();
  let mut code = ten_thousand();
  let first = kept.tokens(&tokenizer, &code);
  assert_eq!(lines.load(Ordering::SeqCst), 10_000);
  assert_eq!(&*first, whole(&code).as_slice());
  // One character typed in the middle: the lines from its line on, a batch of them, until they start as they did.
  let at = code.find("step_5000(").expect("the line") + 4;
  code.insert(at, 'x');
  lines.store(0, Ordering::SeqCst);
  let next = kept.tokens(&tokenizer, &code);
  let asked = lines.load(Ordering::SeqCst);
  assert!((1..=64).contains(&asked), "{asked} lines tokenized for one character");
  assert_eq!(&*next, whole(&code).as_slice(), "as the whole text tokenized again");
  // The same text again asks nothing.
  lines.store(0, Ordering::SeqCst);
  let _ = kept.tokens(&tokenizer, &code);
  assert_eq!(lines.load(Ordering::SeqCst), 0);
}

#[test]
fn an_edit_that_opens_a_comment_goes_on_until_the_lines_agree_again() {
  let (tokenizer, lines) = counting();
  let mut kept = LineCache::default();
  let mut code = (0..2_000)
    .map(|index| format!("let a_{index} = {index};"))
    .collect::<Vec<_>>()
    .join("\n");
  let _ = kept.tokens(&tokenizer, &code);
  // A comment opened at line 100 and not closed: every line after it starts inside it.
  let at = code.find("let a_100 ").expect("the line");
  code.insert_str(at, "/* ");
  lines.store(0, Ordering::SeqCst);
  let opened = kept.tokens(&tokenizer, &code);
  assert!(lines.load(Ordering::SeqCst) >= 1_900, "every line after it");
  assert_eq!(&*opened, whole(&code).as_slice());
  // Closed on the next line: from there the lines start as they did, and the rest are taken as they were.
  let close = code.find("let a_101 ").expect("the line");
  code.insert_str(close, "*/ ");
  lines.store(0, Ordering::SeqCst);
  let closed = kept.tokens(&tokenizer, &code);
  assert!(
    lines.load(Ordering::SeqCst) >= 1_800,
    "the lines after it start outside the comment again"
  );
  assert_eq!(&*closed, whole(&code).as_slice());
  // Lines added and taken away at the end and the start.
  for edited in [
    format!("{code}\nlet tail = 1;"),
    code.replacen("let a_0 = 0;\n", "", 1),
    format!("// head\n{code}"),
    String::new(),
  ] {
    let tokens = kept.tokens(&tokenizer, &edited);
    assert_eq!(&*tokens, whole(&edited).as_slice());
  }
}

#[test]
fn a_tokenizer_that_does_not_go_line_by_line_is_asked_for_all_of_the_text() {
  let asked = Arc::new(AtomicUsize::new(0));
  let counter = asked.clone();
  let plain = CodeTokenizer::new(move |code: &str| {
    counter.fetch_add(1, Ordering::SeqCst);
    vec![CodeToken::new(0..code.find(' ').unwrap_or(code.len()), token::KEYWORD)]
  });
  let mut kept = LineCache::default();
  let tokens = kept.tokens(&plain, "let x\nlet y");
  assert_eq!(&*tokens, &[CodeToken::new(0..3, token::KEYWORD)]);
  let _ = kept.tokens(&plain, "let x\nlet y");
  let _ = kept.tokens(&plain, "fn x\nlet y");
  assert_eq!(asked.load(Ordering::SeqCst), 2);
}

#[test]
fn a_line_state_is_any_value_that_compares() {
  assert_eq!(LineState::start(), LineState::default());
  assert_eq!(LineState::new(3u8), LineState::new(3u8));
  assert_ne!(LineState::new(3u8), LineState::new(4u8));
  assert_ne!(LineState::new(3u8), LineState::new(3u16), "of another type");
  assert_ne!(LineState::new(3u8), LineState::start());
  assert_eq!(LineState::new(vec![1, 2]).get::<Vec<i32>>(), Some(&vec![1, 2]));
}

#[test]
fn an_edit_is_told_in_bytes_and_places_as_tree_sitter_has_it() {
  // A character typed at the end of the second line.
  assert_eq!(
    CodeEdit::between("ab\ncd", "ab\ncde"),
    Some(CodeEdit {
      start_byte: 5,
      old_end_byte: 5,
      new_end_byte: 6,
      start_point: CodePoint { row: 1, column: 2 },
      old_end_point: CodePoint { row: 1, column: 2 },
      new_end_point: CodePoint { row: 1, column: 3 },
    })
  );
  // A line taken away.
  assert_eq!(
    CodeEdit::between("ab\ncd\nef", "ab\nef"),
    Some(CodeEdit {
      start_byte: 3,
      old_end_byte: 6,
      new_end_byte: 3,
      start_point: CodePoint { row: 1, column: 0 },
      old_end_point: CodePoint { row: 2, column: 0 },
      new_end_point: CodePoint { row: 1, column: 0 },
    })
  );
  // On characters' boundaries: é and è differ in their second byte.
  let edit = CodeEdit::between("é", "è").expect("an edit");
  assert_eq!((edit.start_byte, edit.old_end_byte, edit.new_end_byte), (0, 2, 2));
  assert_eq!(CodeEdit::between("same", "same"), None);
}

#[test]
fn a_tokenizer_that_says_its_tokens_are_stale_is_asked_again() {
  let loaded = Arc::new(AtomicBool::new(false));
  let grammar = loaded.clone();
  let tokenizer = CodeTokenizer::new(move |code: &str| {
    if grammar.load(Ordering::SeqCst) {
      vec![CodeToken::new(0..code.len(), token::KEYWORD)]
    } else {
      Vec::new()
    }
  });
  let handle = tokenizer.handle();
  assert!(tokenizer.tokens("select").is_empty());
  // The grammar loads; what was kept stays until the tokenizer says it is stale.
  loaded.store(true, Ordering::SeqCst);
  assert!(tokenizer.tokens("select").is_empty());
  handle.invalidate();
  assert_eq!(&*tokenizer.tokens("select"), &[CodeToken::new(0..6, token::KEYWORD)]);
  // What a text's lines keep is of the generation before, and is not taken either.
  let mut kept = LineCache::default();
  loaded.store(false, Ordering::SeqCst);
  handle.invalidate();
  assert!(kept.tokens(&tokenizer, "select").is_empty());
  loaded.store(true, Ordering::SeqCst);
  handle.invalidate();
  assert_eq!(kept.tokens(&tokenizer, "select").len(), 1);
}
