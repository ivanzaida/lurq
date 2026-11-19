//! A tokenizer with state, as a code block and a code field use it: one that says its tokens are stale, a block whose
//! code changes re-tokenized from its first changed line, and a field's edits told in order.

use std::sync::{
  Arc, Mutex,
  atomic::{AtomicBool, AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App,
    events::ClipboardContent,
    theme::{PaletteColor, ThemePalette},
  },
  code::{BufferId, CodeEdit, CodeLanguage, CodeToken, CodeTokenizer, LineState, Tokenizer, token},
  components::{Column, TextInput},
  core::Signal,
  layout::quad::{QuadContent, RichTextSpan},
  node::{Element, color::Color},
  ui::CodeBlock,
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn staged(scene: impl Fn(&mut lurq::app::ctx::Ctx) -> Element + Send + Sync + 'static) -> Stage {
  let app = App::new();
  app
    .theme()
    .set_preferred_appearance(Some(lurq::app::window::Appearance::Light));
  Stage::in_app(app, move |ctx| Column::new().width(420.0).child(scene(ctx)).into())
}

fn run(stage: &Stage, text: &str) -> Option<RichTextSpan> {
  stage
    .quads()
    .into_iter()
    .filter_map(|quad| match quad.content {
      QuadContent::RichText { spans, .. } => Some(spans),
      _ => None,
    })
    .flatten()
    .find(|span| span.text == text)
}

#[test]
fn a_tokenizer_that_says_its_tokens_are_stale_draws_again() {
  const LANGUAGE: &str = "lang-of-the-stale-tests";
  let loaded = Arc::new(AtomicBool::new(false));
  let grammar = loaded.clone();
  let handle = lurq::code::register_tokenizer(LANGUAGE, move |code: &str| {
    if grammar.load(Ordering::SeqCst) {
      vec![CodeToken::new(0..code.find(' ').unwrap_or(code.len()), token::KEYWORD)]
    } else {
      Vec::new()
    }
  });
  let mut stage = staged(|_| CodeBlock::new("select 1").language(LANGUAGE).selectable(false).into());
  assert_eq!(
    run(&stage, "select 1").map(|span| span.style.color),
    Some(light(PaletteColor::Foreground)),
    "plain while its grammar loads"
  );
  // The grammar is loaded: the tokenizer says its tokens are stale, and the block is drawn with the new ones.
  loaded.store(true, Ordering::SeqCst);
  handle.invalidate();
  stage.pass();
  assert_eq!(
    run(&stage, "select").map(|span| span.style.color),
    Some(light(PaletteColor::Info))
  );
}

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

#[test]
fn a_block_whose_code_grows_re_tokenizes_its_last_lines() {
  let lines = Arc::new(AtomicUsize::new(0));
  let tokenizer = CodeTokenizer::new(Counting {
    rust: CodeLanguage::rust(),
    lines: lines.clone(),
  });
  let code = Signal::new(
    (0..3_000)
      .map(|index| format!("let a_{index} = {index};"))
      .collect::<Vec<_>>()
      .join("\n"),
  );
  let shown = code.clone();
  let mut stage = staged(move |ctx| {
    CodeBlock::new(&shown.get())
      .tokenizer(tokenizer.clone())
      .selectable(false)
      .max_height(300.0)
      .mount(ctx)
  });
  assert_eq!(lines.load(Ordering::SeqCst), 3_000);
  // A line streamed in at its end: the last lines are tokenized, not the three thousand.
  lines.store(0, Ordering::SeqCst);
  code.update(|code| code.push_str("\nlet streamed = 1;"));
  stage.pass();
  let asked = lines.load(Ordering::SeqCst);
  assert!((1..=64).contains(&asked), "{asked} lines tokenized");
}

/// A tokenizer that records what it is told of each buffer.
#[derive(Default)]
struct Recording {
  edits: Mutex<Vec<(BufferId, CodeEdit)>>,
  asked: Mutex<Vec<(BufferId, String)>>,
  dropped: Mutex<Vec<BufferId>>,
}

struct Recorder(Arc<Recording>);

impl Tokenizer for Recorder {
  fn tokens(&self, _: &str) -> Vec<CodeToken> {
    Vec::new()
  }

  fn tokens_in(&self, buffer: BufferId, code: &str) -> Vec<CodeToken> {
    self.0.asked.lock().unwrap().push((buffer, code.to_owned()));
    Vec::new()
  }

  fn edited(&self, buffer: BufferId, edit: &CodeEdit) {
    self.0.edits.lock().unwrap().push((buffer, *edit));
  }

  fn dropped(&self, buffer: BufferId) {
    self.0.dropped.lock().unwrap().push(buffer);
  }
}

/// The bytes of each edit told: its start, its old end and its new end.
fn bytes(recording: &Recording) -> Vec<(usize, usize, usize)> {
  recording
    .edits
    .lock()
    .unwrap()
    .iter()
    .map(|(_, edit)| (edit.start_byte, edit.old_end_byte, edit.new_end_byte))
    .collect()
}

#[test]
fn a_code_fields_tokenizer_is_told_each_edit_in_order_and_that_it_is_gone() {
  let recording = Arc::new(Recording::default());
  let tokenizer = CodeTokenizer::new(Recorder(recording.clone()));
  let value = Signal::new("fn a".to_owned());
  let shown = Signal::new(true);
  let (field, on) = (value.clone(), shown.clone());
  let mut stage = staged(move |_| {
    if !on.get() {
      return TextInput::new(Signal::new(String::new())).id("other").into();
    }
    TextInput::new(field.clone())
      .multiline()
      .code_tokenizer(tokenizer.clone())
      .id("field")
      .into()
  });
  let buffer = recording
    .asked
    .lock()
    .unwrap()
    .first()
    .map(|(buffer, _)| *buffer)
    .expect("asked once");
  // The caret at the end: a character typed, one taken back, a paste, and an undo of the paste.
  stage.click("field");
  stage.tree.key_down("End".into(), "End".into(), false, true, false);
  stage.tree.key_up("End".into(), "End".into(), false, true, false);
  stage.key("x");
  stage.key("Backspace");
  assert!(stage.tree.paste(ClipboardContent::text("(b)")));
  stage.pass();
  stage.shortcut("z");
  stage.pass();
  assert_eq!(value.get_untracked(), "fn a");
  assert_eq!(
    bytes(&recording),
    [(4, 4, 5), (4, 5, 4), (4, 4, 7), (4, 7, 4)],
    "typed, taken back, pasted, undone"
  );
  let edits = recording.edits.lock().unwrap().clone();
  assert!(edits.iter().all(|(of, _)| *of == buffer), "all of the one buffer");
  assert_eq!(edits[2].1.new_end_point.column, 7);
  // Each tokens asked after an edit are of the text after it.
  assert_eq!(
    recording.asked.lock().unwrap().last().map(|(_, text)| text.as_str()),
    Some("fn a")
  );
  // The field goes away: its tokenizer is told.
  // The field goes away, and the focus it had with it.
  shown.set(false);
  stage.pass();
  stage.click("other");
  let dropped = recording
    .dropped
    .lock()
    .map(|dropped| dropped.clone())
    .unwrap_or_default();
  assert_eq!(dropped, [buffer]);
}
