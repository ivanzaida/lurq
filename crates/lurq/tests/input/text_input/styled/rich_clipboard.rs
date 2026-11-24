//! An input that keeps its formatting through the clipboard: a copy keeps the Markdown of the selection beside
//! its text, and a paste that has Markdown, or a text that reads as it, comes with its marks and kinds.
use std::sync::{Arc, Mutex};

use lurq::{
  app::{ClipboardReader, ClipboardWriter, events::ClipboardContent},
  components::{
    ParagraphKind::{self, Body, Bullet, Heading, Quote},
    ParagraphStyles, RichText, TextInput,
  },
  core::Signal,
};

use super::{Editor, bold, span};

/// A clipboard of the test's: what was written to it, and read back as the OS clipboard would give it.
#[derive(Default)]
struct Clipboard {
  text: Mutex<Option<String>>,
  /// The Markdown written beside the text.
  markdown: Mutex<Option<String>>,
  /// How each write came: `text` or `rich`.
  writes: Mutex<Vec<&'static str>>,
}

impl ClipboardWriter for Clipboard {
  fn write_text(&self, text: &str) -> bool {
    *self.text.lock().unwrap() = Some(text.to_owned());
    *self.markdown.lock().unwrap() = None;
    self.writes.lock().unwrap().push("text");
    true
  }

  fn write_rich(&self, text: &str, markdown: &str) -> bool {
    *self.text.lock().unwrap() = Some(text.to_owned());
    *self.markdown.lock().unwrap() = Some(markdown.to_owned());
    self.writes.lock().unwrap().push("rich");
    true
  }
}

impl ClipboardReader for Clipboard {
  fn read_text(&self) -> Option<String> {
    self.text.lock().unwrap().clone()
  }

  fn read(&self) -> Option<ClipboardContent> {
    self.read_text().map(ClipboardContent::text)
  }

  fn read_markdown(&self, text: &str) -> Option<String> {
    (self.text.lock().unwrap().as_deref() == Some(text))
      .then(|| self.markdown.lock().unwrap().clone())
      .flatten()
  }
}

/// An editor of `document` with the test's clipboard; `rich` says whether it keeps its formatting through it.
fn opened(document: &RichText, rich: bool) -> (Editor, Arc<Clipboard>) {
  let kinds = Signal::new(document.paragraphs.clone());
  let mut editor = Editor::with(&document.text, document.spans.clone(), move |input: TextInput| {
    input
      .paragraphs(kinds.clone(), ParagraphStyles::new())
      .rich_clipboard(rich)
  });
  let clipboard = Arc::new(Clipboard::default());
  editor.tree.set_clipboard_writer(Some(clipboard.clone()));
  editor.tree.set_clipboard_reader(Some(clipboard.clone()));
  (editor, clipboard)
}

fn document(editor: &Editor) -> RichText {
  editor.controller.rich_text().unwrap().expect("a styled input")
}

fn kinds(editor: &Editor) -> Vec<ParagraphKind> {
  document(editor).paragraphs
}

fn notes() -> RichText {
  RichText::from_markdown("## Release notes\n\nBackdrop blur **follows clips**.\n\n- Faster shaping\n- Fewer passes")
}

/// A line of a block of code with no language.
const CODE: ParagraphKind = ParagraphKind::Code { language: None };
#[test]
fn styled_a_copy_keeps_the_markdown_of_the_selection_beside_its_text() {
  let (mut editor, clipboard) = opened(&notes(), true);
  editor.shortcut("a", "KeyA", false);
  editor.shortcut("c", "KeyC", false);
  // The clipboard's text is the text, as any input copies it; the Markdown is beside it.
  assert_eq!(
    clipboard.text.lock().unwrap().as_deref(),
    Some("Release notes\nBackdrop blur follows clips.\nFaster shaping\nFewer passes")
  );
  assert_eq!(
    clipboard.markdown.lock().unwrap().as_deref(),
    Some("## Release notes\n\nBackdrop blur **follows clips**.\n\n- Faster shaping\n- Fewer passes")
  );
  // A part of a paragraph is copied with its marks and the kind of its paragraph.
  let start = "Release notes\nBackdrop blur ".len();
  editor.select(start, start + "follows".len());
  editor.shortcut("c", "KeyC", false);
  assert_eq!(clipboard.text.lock().unwrap().as_deref(), Some("follows"));
  assert_eq!(clipboard.markdown.lock().unwrap().as_deref(), Some("**follows**"));
  // A cut is a copy that takes the text away.
  editor.select(0, "Release notes\n".len());
  editor.shortcut("x", "KeyX", false);
  assert_eq!(
    clipboard.markdown.lock().unwrap().as_deref(),
    Some("## Release notes\n\n")
  );
  assert_eq!(*clipboard.writes.lock().unwrap(), ["rich", "rich", "rich"]);
  assert_eq!(
    editor.value.get(),
    "Backdrop blur follows clips.
Faster shaping
Fewer passes"
  );
}

#[test]
fn styled_an_input_that_does_not_ask_copies_and_pastes_as_it_did() {
  let (mut editor, clipboard) = opened(&notes(), false);
  editor.shortcut("a", "KeyA", false);
  editor.shortcut("c", "KeyC", false);
  assert_eq!(*clipboard.writes.lock().unwrap(), ["text"]);
  // Markdown beside a pasted text, and a text that reads as Markdown, are text to it.
  editor.select(0, 0);
  assert!(
    editor
      .tree
      .paste(ClipboardContent::text("Title").with_markdown("# Title"))
  );
  assert!(editor.tree.paste(ClipboardContent::text("- item\n")));
  assert_eq!(
    editor.value.get(),
    "Title- item\nRelease notes\nBackdrop blur follows clips.\nFaster shaping\nFewer passes"
  );
  assert_eq!(kinds(&editor)[..2], [Heading(2), Heading(2)]);
}

#[test]
fn styled_a_paste_with_markdown_beside_its_text_comes_with_its_marks_and_kinds() {
  let (mut editor, _) = opened(&RichText::plain("First\n\nLast"), true);
  // Into the empty line between: every pasted paragraph comes as what it is, as one step of the undo.
  editor.select(6, 6);
  let pasted = notes();
  assert!(
    editor
      .tree
      .paste(ClipboardContent::text(pasted.text.clone()).with_markdown(pasted.to_markdown()))
  );
  let now = document(&editor);
  assert_eq!(
    now.text,
    "First\nRelease notes\nBackdrop blur follows clips.\nFaster shaping\nFewer passes\nLast"
  );
  assert_eq!(now.paragraphs, [Body, Heading(2), Body, Bullet, Bullet, Body]);
  let bold_at = "First\nRelease notes\nBackdrop blur ".len();
  assert_eq!(now.spans, vec![span(bold_at..bold_at + "follows clips".len(), bold())]);
  editor.shortcut("z", "KeyZ", false);
  assert_eq!(document(&editor), RichText::plain("First\n\nLast"));
}

#[test]
fn styled_a_pasted_paragraph_goes_into_the_line_it_is_put_in() {
  // One paragraph into text: its marks come, the line stays what it is.
  let (mut editor, _) = opened(&RichText::plain("Hello world"), true);
  editor.select(6, 6);
  assert!(
    editor
      .tree
      .paste(ClipboardContent::text("big").with_markdown("# **big**"))
  );
  let now = document(&editor);
  assert_eq!(
    (now.text.as_str(), now.paragraphs.as_slice()),
    ("Hello bigworld", &[Body][..])
  );
  assert_eq!(now.spans, vec![span(6..9, bold())]);

  // Several into the middle of a line: the first goes on from the text before it, the last gives its kind to
  // the text after it.
  let (mut editor, _) = opened(&RichText::plain("abcd"), true);
  editor.select(2, 2);
  assert!(
    editor
      .tree
      .paste(ClipboardContent::text("x\ny\nz").with_markdown("# x\n\n> y\n\n- z"))
  );
  let now = document(&editor);
  assert_eq!(now.text, "abx\ny\nzcd");
  assert_eq!(now.paragraphs, [Body, Quote, Bullet]);

  // A line that holds nothing else takes the kind of the one paragraph pasted into it.
  let (mut editor, _) = opened(&RichText::plain(""), true);
  assert!(
    editor
      .tree
      .paste(ClipboardContent::text("Title").with_markdown("# Title"))
  );
  assert_eq!(kinds(&editor), [Heading(1)]);
}

#[test]
fn styled_a_pasted_text_that_reads_as_markdown_with_block_marks_is_markdown() {
  let (mut editor, _) = opened(&RichText::plain(""), true);
  assert!(
    editor
      .tree
      .paste(ClipboardContent::text("# Plan\n\n1. Ship **it**\n2. Rest\n\n> Later"))
  );
  let now = document(&editor);
  assert_eq!(now.text, "Plan\nShip it\nRest\nLater");
  assert_eq!(
    now.paragraphs,
    [Heading(1), ParagraphKind::Ordered, ParagraphKind::Ordered, Quote]
  );
  assert_eq!(now.spans, vec![span(10..12, bold())]);

  // A mark Markdown reads makes a text Markdown too; a text with no mark and no block mark is pasted as it is.
  let (mut editor, _) = opened(&RichText::plain(""), true);
  assert!(editor.tree.paste(ClipboardContent::text("2 * 3 = 6, **really**")));
  assert_eq!(
    document(&editor),
    RichText::new("2 * 3 = 6, really", vec![span(11..17, bold())], vec![Body])
  );
  let (mut editor, _) = opened(&RichText::plain(""), true);
  assert!(editor.tree.paste(ClipboardContent::text("2 * 3 = 6 [sic]")));
  assert_eq!(document(&editor), RichText::plain("2 * 3 = 6 [sic]"));
}

#[test]
fn styled_the_markdown_a_copy_writes_is_read_back_as_its_text_by_another_program() {
  // The OS clipboard holds one text: a copy of formatted text gives it the Markdown, which is all another program,
  // or another lurq process, reads. Pasted as text with nothing beside it, it comes back with its marks and kinds.
  let (mut source, clipboard) = opened(&notes(), true);
  source.shortcut("a", "KeyA", false);
  source.shortcut("c", "KeyC", false);
  let markdown = clipboard
    .markdown
    .lock()
    .unwrap()
    .clone()
    .expect("the Markdown of the selection");
  let (mut editor, _) = opened(&RichText::plain(""), true);
  assert!(editor.tree.paste(ClipboardContent::text(markdown.clone())));
  assert_eq!(document(&editor), notes());

  // So does a part of a paragraph, which has marks and no kind.
  let start = "Release notes
Backdrop blur "
    .len();
  source.select(start, start + "follows clips".len());
  source.shortcut("c", "KeyC", false);
  let part = clipboard.markdown.lock().unwrap().clone().unwrap();
  assert_eq!(part, "**follows clips**");
  let (mut editor, _) = opened(&RichText::plain(""), true);
  assert!(editor.tree.paste(ClipboardContent::text(part.clone())));
  assert_eq!(
    document(&editor),
    RichText::new("follows clips", vec![span(0..13, bold())], vec![Body])
  );

  // A plain input takes the same text as it is written.
  let mut plain = Editor::with("", vec![], |input: TextInput| input);
  assert!(plain.tree.paste(ClipboardContent::text(markdown.clone())));
  assert_eq!(plain.value.get(), markdown);
}

#[test]
fn styled_a_paste_into_a_line_of_code_is_the_text_as_it_is_written() {
  let (mut editor, _) = opened(&RichText::new("let x;", vec![], vec![CODE]), true);
  editor.select(6, 6);
  assert!(editor.tree.paste(ClipboardContent::text("\n# not a heading")));
  let now = document(&editor);
  assert_eq!(now.text, "let x;\n# not a heading");
  assert_eq!(now.paragraphs, [CODE, CODE]);
}

#[test]
fn styled_what_is_copied_from_the_input_is_pasted_with_its_formatting_by_the_shortcut() {
  let (mut editor, clipboard) = opened(&notes(), true);
  // The heading, copied whole, pasted at the end of the document after a new line.
  editor.select(0, "Release notes".len());
  editor.shortcut("c", "KeyC", false);
  let end = editor.value.get().len();
  editor.select(end, end);
  editor.key("Enter");
  // The shortcut reads the clipboard: its text, and the Markdown kept beside that text.
  let ctrl = cfg!(not(target_vendor = "apple"));
  editor
    .tree
    .key_down_with_meta("v".into(), "KeyV".into(), false, ctrl, false, !ctrl);
  let now = document(&editor);
  assert!(now.text.ends_with("Fewer passes\nRelease notes"), "{:?}", now.text);
  assert_eq!(now.paragraphs, [Heading(2), Body, Bullet, Bullet, Heading(2)]);

  // What another program copied has no Markdown beside it: it is pasted as text.
  assert!(clipboard.write_text("another"));
  editor.key("Enter");
  editor
    .tree
    .key_down_with_meta("v".into(), "KeyV".into(), false, ctrl, false, !ctrl);
  let now = document(&editor);
  assert!(now.text.ends_with("Release notes\nanother"), "{:?}", now.text);
  assert_eq!(now.spans, notes().spans);
}
