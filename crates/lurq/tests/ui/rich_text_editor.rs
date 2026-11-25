//! A rich text editor: its box, bar, tools and footer against the board, how each kind of paragraph is laid out,
//! what a tool and a key do to the selection, the Markdown shortcuts, undo, the bound and the held value, read
//! only and disabled, the panel of a link and where the focus goes, a narrow room, and what it is to tooling.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    theme::{PaletteColor, RadiusSize, ThemePalette},
  },
  components::{
    Column,
    ParagraphKind::{self, Body, Bullet, Heading, Ordered, Quote},
    RichText, SpanStyle, StyledSpan,
  },
  core::Signal,
  layout::quad::QuadContent,
  node::color::Color,
  ui::{RichTextController, RichTextEditor, RichTextEditorPart, RichTextTool},
};

use super::stage::Stage;

const ID: &str = "editor";
const CONTENT: &str = "editor-content";

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn close(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.6
}

fn span(range: std::ops::Range<usize>, style: SpanStyle) -> StyledSpan {
  StyledSpan::new(range, style)
}

/// The board's document: a heading, a paragraph with bold words, two items.
fn notes() -> RichText {
  RichText::from_markdown(
    "## Release notes\n\nBackdrop blur now **follows rounded clips** and transforms.\n\n- Faster text shaping\n- Fixes for window teardown",
  )
}

/// What the editor told the app, in order: each document, as Markdown.
type Log = Arc<Mutex<Vec<String>>>;

/// An editor of `document` in a window of `width` by 480, the app's signal of it, its controller, and what it
/// told `on_change`.
struct Editor {
  stage: Stage,
  value: Signal<RichText>,
  editor: RichTextController,
  log: Log,
}

impl Editor {
  fn new(document: RichText) -> Self {
    Self::with(640.0, document, |editor| editor)
  }

  fn with(
    width: f32,
    document: RichText,
    adjust: impl Fn(RichTextEditor) -> RichTextEditor + Send + Sync + 'static,
  ) -> Self {
    Self::in_app(App::new(), width, document, adjust)
  }

  fn in_app(
    app: App,
    width: f32,
    document: RichText,
    adjust: impl Fn(RichTextEditor) -> RichTextEditor + Send + Sync + 'static,
  ) -> Self {
    let value = Signal::new(document);
    let editor = RichTextController::new();
    let log = Log::default();
    let (held, handle, told) = (value.clone(), editor.clone(), log.clone());
    let mut stage = Stage::first_frame_in(app, width, 480.0, move |ctx| {
      let told = told.clone();
      let editor = RichTextEditor::new(held.clone())
        .id(ID)
        .controller(&handle)
        .on_change(move |document| told.lock().unwrap().push(document.to_markdown()));
      Column::new()
        .size(width, 480.0)
        .child(adjust(editor).mount(ctx))
        .child(lurq::components::Button::new("Elsewhere").id("elsewhere"))
        .into()
    });
    stage.pass();
    Self {
      stage,
      value,
      editor,
      log,
    }
  }

  /// Puts the focus in the box, the caret at the end.
  fn focus(&mut self) {
    self.stage.tree.get_element_by_id_mut(CONTENT).unwrap().focus();
    self.stage.pass();
  }

  fn select(&mut self, anchor: usize, caret: usize) {
    assert!(self.editor.select(anchor, caret));
    self.stage.pass();
  }

  fn type_text(&mut self, text: &str) {
    for character in text.chars() {
      let key = character.to_string();
      self.stage.tree.key_down(key.clone(), "Key".into(), false, false, false);
      self.stage.tree.key_up(key, "Key".into(), false, false, false);
    }
    self.stage.pass();
  }

  /// Ctrl+key, or Cmd+key on a Mac.
  fn shortcut(&mut self, key: &str, code: &str, shift: bool, alt: bool) {
    let (ctrl, meta) = (cfg!(not(target_vendor = "apple")), cfg!(target_vendor = "apple"));
    self
      .stage
      .tree
      .key_down_with_meta(key.into(), code.into(), shift, ctrl, alt, meta);
    self
      .stage
      .tree
      .key_up_with_meta(key.into(), code.into(), shift, ctrl, alt, meta);
    self.stage.pass();
  }

  fn document(&self) -> RichText {
    self.value.get()
  }

  fn kinds(&self) -> Vec<ParagraphKind> {
    self.document().paragraphs
  }

  fn told(&self) -> Vec<String> {
    self.log.lock().unwrap().clone()
  }

  fn focused(&self) -> Option<String> {
    self
      .stage
      .tree
      .focused_element()
      .and_then(|element| element.id().map(str::to_owned))
  }

  /// Whether the control with `id` is drawn disabled: its icon at half its opacity.
  fn dimmed(&mut self, id: &str) -> bool {
    let (x, y, width, height) = self.stage.bounds(id);
    self.stage.quads().iter().any(|quad| {
      matches!(quad.content, QuadContent::Text { .. })
        && quad.x >= x
        && quad.y >= y
        && quad.x + quad.width <= x + width
        && quad.y + quad.height <= y + height
        && quad.opacity < 0.75
    })
  }

  /// Whether the tool with the id's `suffix` is drawn on: filled with the background, on the bar's muted fill.
  fn on(&mut self, suffix: &str) -> bool {
    // A tool under the pointer is a step darker: the pointer is taken off it first.
    self.stage.tree.mouse_move(630.0, 470.0);
    self.stage.pass();
    self.stage.fill_color(&format!("{ID}-{suffix}")) == Some(light(PaletteColor::Background))
  }
}

#[test]
fn the_editor_is_the_boards_box_with_its_bar_its_content_and_its_footer() {
  let mut editor = Editor::new(notes());
  let stage = &mut editor.stage;
  let (x, y, width, height) = stage.bounds(ID);
  assert_eq!(width, 560.0, "the design's width in a room of 640");
  assert_eq!(stage.fill_color(ID), Some(light(PaletteColor::Background)));
  let edge = stage
    .border(ID)
    .and_then(|quad| quad.border)
    .and_then(|border| border.top);
  assert_eq!(
    edge.map(|side| (side.width, side.color)),
    Some((1.0, light(PaletteColor::Border)))
  );
  let radius = App::new().theme().radii().get(RadiusSize::Lg);
  assert_eq!(
    stage
      .border(ID)
      .and_then(|quad| quad.border_radius)
      .map(|radius| radius.top_left),
    Some(radius)
  );
  // The bar: 40 high on the bar's fill, padded 6, its tools 28 square and 2 apart.
  let (bar_x, bar_y, bar_width, bar_height) = stage.bounds("editor-toolbar");
  assert_eq!((bar_x, bar_y, bar_width, bar_height), (x, y, 560.0, 40.0));
  assert_eq!(stage.fill_color("editor-toolbar"), Some(light(PaletteColor::Bar)));
  let (style_x, _, style_width, style_height) = stage.bounds("editor-style");
  assert!(close(style_x - x, 6.0), "padded 6: {style_x}");
  // The style's control hugs the name it shows: 10, the name, 6, a chevron of 12, 6.
  assert_eq!(style_height, 28.0);
  let name = stage.text_quad("Paragraph").expect("the style of the first paragraph");
  assert!(
    close(style_width, 10.0 + name.width + 6.0 + 12.0 + 6.0),
    "{style_width} for {}",
    name.width
  );
  let (bold_x, bold_y, bold_width, bold_height) = stage.bounds("editor-bold");
  assert_eq!((bold_width, bold_height), (28.0, 28.0));
  assert!(close(bold_y - y, 6.0), "centred in the bar's 40");
  // A line of 1 with 4 on either side, 2 from each: 13 from the style to the first mark.
  assert!(close(bold_x - (style_x + style_width), 13.0), "{bold_x}");
  let (italic_x, ..) = stage.bounds("editor-italic");
  assert!(close(italic_x - (bold_x + 28.0), 2.0));
  // The history is at the bar's end.
  let (redo_x, ..) = stage.bounds("editor-redo");
  assert!(close(redo_x + 28.0, x + 560.0 - 6.0), "{redo_x}");
  // The box under it, as wide, and the footer of 28 at the bottom.
  let (content_x, content_y, content_width, content_height) = stage.bounds(CONTENT);
  assert_eq!((content_x, content_y, content_width), (x, y + 40.0, 560.0));
  let (count_x, count_y, count_width, count_height) = stage.bounds("editor-count");
  assert!(close(count_x + count_width, x + 560.0 - 12.0), "padded 12");
  assert!(count_y > content_y + content_height - 0.5 && count_y + count_height <= y + height + 0.5);
  assert!(close(y + height - (content_y + content_height), 28.0), "a footer of 28");
  let (color, size) = stage.text("17 words");
  assert_eq!((color, size), (light(PaletteColor::MutedForeground), 11.0));
  assert_eq!(stage.font_of("17 words"), App::new().theme().mono_family().to_string());
  assert!(stage.paints("Markdown shortcuts on"));
}

#[test]
fn the_document_is_laid_out_as_the_board_draws_it() {
  let mut editor = Editor::new(notes());
  let input = editor.editor.input();
  let text = editor.document().text;
  let (x, y, ..) = editor.stage.bounds(CONTENT);
  // The heading: 22 on a line of 1.2, 18 in and 16 down.
  let heading = input.caret_point(0).unwrap();
  let (left, top) = (x + 18.0, y + 16.0);
  let _ = (left, top);
  assert!(close(heading.x, 0.0), "{}", heading.x);
  let paragraph = input.caret_point(text.find("Backdrop").unwrap()).unwrap();
  // The heading's row is 26 high and a row of text 18, whatever the font's own line is, and 10 stand between
  // two paragraphs.
  let (heading_row, row) = (26.0, 18.0);
  // A caret point stands as far from the top of its row in every row: the rows are measured from the heading's.
  assert!(
    (paragraph.y - heading.y - (heading_row + 10.0)).abs() < 0.05,
    "{}",
    paragraph.y - heading.y
  );
  // An item's text starts 21 in, a row and 10 under the paragraph.
  let first = input.caret_point(text.find("Faster").unwrap()).unwrap();
  let second = input.caret_point(text.find("Fixes").unwrap()).unwrap();
  assert!(close(first.x, 21.0), "{}", first.x);
  let pitch = first.y - paragraph.y;
  assert!((pitch - (row + 10.0)).abs() < 0.05, "{pitch}");
  assert!(close(second.y - first.y, pitch));
  // Its dot: 5 across in the muted foreground, 6 from the start of the text's box.
  let dots: Vec<_> = editor
    .stage
    .quads()
    .into_iter()
    .filter(|quad| {
      quad.width == 5.0
        && quad.height == 5.0
        && matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::MutedForeground))
    })
    .collect();
  assert_eq!(dots.len(), 2);
  assert!(close(dots[0].x, x + 18.0 + 6.0), "{}", dots[0].x);
  // The box is as high as its rows say: 16, the heading, three rows and three gaps, 16.
  let (.., height) = editor.stage.bounds(CONTENT);
  let board = 16.0 + heading_row + 3.0 * (row + 10.0) + 16.0;
  assert!((height - board).abs() < 0.05, "{height}");
}

#[test]
fn a_tool_marks_the_selection_tells_the_app_and_leaves_the_focus_in_the_text() {
  let mut editor = Editor::new(RichText::plain("plain words"));
  editor.focus();
  editor.select(0, 5);
  assert!(!editor.on("bold"));
  editor.stage.click("editor-bold");
  assert_eq!(editor.document().spans, vec![span(0..5, SpanStyle::new().bold())]);
  assert_eq!(
    editor.told(),
    ["**plain** words"],
    "the signal is set, and the app is told"
  );
  assert_eq!(
    editor.focused().as_deref(),
    Some(CONTENT),
    "the focus stays in the text"
  );
  assert!(editor.on("bold"), "the tool shows what the selection is");
  editor.stage.click("editor-italic");
  editor.stage.click("editor-underline");
  editor.stage.click("editor-strikethrough");
  editor.stage.click("editor-code");
  let style = SpanStyle::new().bold().italic().underline().strikethrough().code();
  assert_eq!(editor.document().spans, vec![span(0..5, style)]);
  // A second press takes the mark away.
  editor.stage.click("editor-bold");
  assert!(!editor.on("bold"));
  assert!(!editor.document().spans[0].style.bold);
  // With the caret in plain text no tool is on.
  editor.select(8, 8);
  assert!(!editor.on("italic") && !editor.on("code"));
}

#[test]
fn a_mark_with_nothing_selected_is_of_what_is_typed_next() {
  let mut editor = Editor::new(RichText::plain("ab "));
  editor.focus();
  editor.stage.click("editor-bold");
  assert!(editor.on("bold"), "the tool is on for what is typed next");
  editor.type_text("cd");
  assert_eq!(editor.document().text, "ab cd");
  assert_eq!(editor.document().spans, vec![span(3..5, SpanStyle::new().bold())]);
}

#[test]
fn the_tools_of_a_list_and_a_quote_turn_the_paragraphs_the_selection_touches() {
  let mut editor = Editor::new(RichText::plain("one\ntwo\nthree"));
  editor.focus();
  editor.select(1, 6);
  editor.stage.click("editor-bullet");
  assert_eq!(editor.kinds(), vec![Bullet, Bullet, Body]);
  assert!(editor.on("bullet") && !editor.on("ordered"));
  editor.stage.click("editor-ordered");
  assert_eq!(editor.kinds(), vec![Ordered, Ordered, Body]);
  // A second press makes them paragraphs again.
  editor.stage.click("editor-ordered");
  assert_eq!(editor.kinds(), vec![Body, Body, Body]);
  editor.select(9, 9);
  editor.stage.click("editor-quote");
  assert_eq!(editor.kinds(), vec![Body, Body, Quote]);
  assert_eq!(editor.told().last().unwrap(), "one\n\ntwo\n\n> three");
}

#[test]
fn the_keys_mark_the_selection_and_turn_its_paragraphs() {
  let mut editor = Editor::new(RichText::plain("one\ntwo"));
  editor.focus();
  editor.select(0, 3);
  editor.shortcut("b", "KeyB", false, false);
  editor.shortcut("i", "KeyI", false, false);
  editor.shortcut("u", "KeyU", false, false);
  assert_eq!(
    editor.document().spans,
    vec![span(0..3, SpanStyle::new().bold().italic().underline())]
  );
  editor.shortcut("b", "KeyB", false, false);
  editor.shortcut("X", "KeyX", true, false);
  editor.shortcut("e", "KeyE", false, false);
  assert_eq!(
    editor.document().spans,
    vec![span(0..3, SpanStyle::new().italic().underline().strikethrough().code())]
  );
  assert_eq!(editor.document().text, "one\ntwo", "no key typed its letter");
  editor.shortcut("2", "Digit2", false, true);
  assert_eq!(editor.kinds(), vec![Heading(2), Body]);
  editor.shortcut("0", "Digit0", false, true);
  assert_eq!(editor.kinds(), vec![Body, Body]);
  editor.select(5, 5);
  editor.shortcut("*", "Digit8", true, false);
  assert_eq!(editor.kinds(), vec![Body, Bullet]);
  editor.shortcut("&", "Digit7", true, false);
  assert_eq!(editor.kinds(), vec![Body, Ordered]);
  editor.shortcut("(", "Digit9", true, false);
  assert_eq!(editor.kinds(), vec![Body, Quote]);
}

#[test]
fn the_style_control_shows_what_the_paragraph_is_and_makes_it_something_else() {
  let mut editor = Editor::new(notes());
  editor.focus();
  editor.select(3, 3);
  assert!(editor.stage.paints("Heading 2"), "the caret is in the heading");
  let paragraph = editor.document().text.find("Backdrop").unwrap();
  editor.select(paragraph, paragraph);
  assert!(editor.stage.paints("Paragraph") && !editor.stage.paints("Heading 2"));
  // Chosen from its list, a style is the paragraph's, and the focus goes back to the text.
  editor.stage.click("editor-style");
  let option = editor.stage.text_quad("Heading 1").expect("the list of styles is open");
  // The list is as wide as its longest name with the check beside it, not as the control: every name is whole.
  let names = ["Paragraph", "Heading 1", "Heading 2", "Heading 3", "Code block"];
  let widest = names
    .iter()
    .map(|name| {
      let quads = editor.stage.quads();
      let found = quads
        .iter()
        .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == name))
        .next_back()
        .unwrap_or_else(|| panic!("{name} is whole in the list"));
      found.width
    })
    .fold(0.0, f32::max);
  let (.., control) = {
    let (x, y, width, _) = editor.stage.bounds("editor-style");
    (x, y, width)
  };
  // The list's edge: the nearest box with an edge that holds the option, the editor's own being further out.
  let list = editor
    .stage
    .quads()
    .into_iter()
    .filter(|quad| quad.border.is_some() && quad.y < option.y && quad.x <= option.x && quad.height > 100.0)
    .max_by(|a, b| a.y.total_cmp(&b.y))
    .map_or(0.0, |quad| quad.width);
  assert!(
    list > widest + 16.0 && list < widest + 72.0,
    "a list of {list} for a name of {widest}"
  );
  assert!(list > control, "wider than the control of {control}");
  let whole = editor
    .stage
    .quads()
    .iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == "Paragraph"))
    .count();
  assert_eq!(whole, 2, "in the control and in its list");
  assert!(editor.stage.paints("Code block"));
  let (x, y) = (option.x + option.width / 2.0, option.y + option.height / 2.0);
  editor.stage.tree.mouse_move(x, y);
  editor.stage.tree.mouse_down(x, y, lurq::app::events::MouseButton::Left);
  editor.stage.tree.mouse_up(x, y, lurq::app::events::MouseButton::Left);
  editor.stage.pass();
  editor.stage.pass();
  assert_eq!(editor.kinds()[1], Heading(1));
  assert!(editor.stage.paints("Heading 1"), "the control shows the style chosen");
  assert_eq!(editor.focused().as_deref(), Some(CONTENT));
}

#[test]
fn markdown_typed_at_the_start_of_a_paragraph_makes_it_what_it_says() {
  for (typed, kind) in [
    ("# ", Heading(1)),
    ("## ", Heading(2)),
    ("### ", Heading(3)),
    ("- ", Bullet),
    ("* ", Bullet),
    ("1. ", Ordered),
    ("> ", Quote),
    ("``` ", ParagraphKind::Code { language: None }),
  ] {
    let mut editor = Editor::new(RichText::default());
    editor.focus();
    editor.type_text(typed);
    assert_eq!(editor.kinds(), vec![kind], "{typed:?}");
    assert_eq!(editor.document().text, "", "the mark is taken away: {typed:?}");
    editor.type_text("text");
    assert_eq!(editor.document().text, "text");
  }
  // Not in the middle of a paragraph, and not where the shortcuts are off.
  let mut editor = Editor::new(RichText::plain("a "));
  editor.focus();
  editor.type_text("# ");
  assert_eq!((editor.document().text.as_str(), editor.kinds()), ("a # ", vec![Body]));
  let mut editor = Editor::with(640.0, RichText::default(), |editor| editor.markdown_shortcuts(false));
  editor.focus();
  editor.type_text("## ");
  assert_eq!((editor.document().text.as_str(), editor.kinds()), ("## ", vec![Body]));
  assert!(
    !editor.stage.paints("Markdown shortcuts on"),
    "the footer says so only while they are on"
  );
}

#[test]
fn markdown_typed_around_words_marks_them_as_its_last_character_is_typed() {
  for (typed, text, style) in [
    ("a **bold**", "a bold", SpanStyle::new().bold()),
    ("a *slanted*", "a slanted", SpanStyle::new().italic()),
    ("a _slanted_", "a slanted", SpanStyle::new().italic()),
    ("a `code`", "a code", SpanStyle::new().code()),
    ("a ~~struck~~", "a struck", SpanStyle::new().strikethrough()),
  ] {
    let mut editor = Editor::new(RichText::default());
    editor.focus();
    editor.type_text(typed);
    assert_eq!(editor.document().text, text, "{typed:?}");
    assert_eq!(editor.document().spans, vec![span(2..text.len(), style)], "{typed:?}");
    // What is typed next is plain again.
    editor.type_text(" x");
    assert_eq!(editor.document().spans.len(), 1, "{typed:?}");
    assert_eq!(editor.document().spans[0].range, 2..text.len());
  }
  // Two stars with a space after them mark nothing.
  let mut editor = Editor::new(RichText::default());
  editor.focus();
  editor.type_text("2 * 3 * 4");
  assert_eq!(editor.document().text, "2 * 3 * 4");
  assert!(editor.document().spans.is_empty());
}

#[test]
fn enter_ends_a_list_in_an_empty_item_and_starts_a_paragraph_after_a_heading() {
  let mut editor = Editor::new(RichText::new("Title\nitem", vec![], vec![Heading(2), Bullet]));
  editor.focus();
  editor.stage.key("Enter");
  assert_eq!(
    editor.kinds(),
    vec![Heading(2), Bullet, Bullet],
    "an item after an item"
  );
  editor.stage.key("Enter");
  assert_eq!(
    editor.kinds(),
    vec![Heading(2), Bullet, Body],
    "Enter in the empty item ends the list"
  );
  assert_eq!(editor.document().text, "Title\nitem\n", "and adds no line");
  // At the end of the heading: a paragraph under it.
  editor.select(5, 5);
  editor.stage.key("Enter");
  assert_eq!(editor.document().text, "Title\n\nitem\n");
  assert_eq!(editor.kinds(), vec![Heading(2), Body, Bullet, Body]);
  // In the middle of a heading: two headings.
  editor.select(2, 2);
  editor.stage.key("Enter");
  assert_eq!(editor.kinds()[..2], [Heading(2), Heading(2)]);
}

#[test]
fn backspace_at_the_start_of_an_item_makes_it_a_paragraph_before_it_joins_the_one_above() {
  let mut editor = Editor::new(RichText::new("one\ntwo", vec![], vec![Body, Bullet]));
  editor.focus();
  editor.select(4, 4);
  editor.stage.key("Backspace");
  assert_eq!(
    (editor.document().text.as_str(), editor.kinds()),
    ("one\ntwo", vec![Body, Body])
  );
  editor.stage.key("Backspace");
  assert_eq!(
    (editor.document().text.as_str(), editor.kinds()),
    ("onetwo", vec![Body])
  );
}

#[test]
fn undo_and_redo_are_off_until_there_is_something_to_take_back() {
  let mut editor = Editor::new(RichText::plain("text"));
  editor.focus();
  let disabled = |editor: &mut Editor, id: &str| editor.dimmed(id);
  assert!(disabled(&mut editor, "editor-undo") && disabled(&mut editor, "editor-redo"));
  editor.select(0, 4);
  editor.stage.click("editor-bold");
  assert!(!disabled(&mut editor, "editor-undo") && disabled(&mut editor, "editor-redo"));
  editor.stage.click("editor-undo");
  assert!(editor.document().spans.is_empty(), "the mark is taken back");
  assert_eq!(editor.focused().as_deref(), Some(CONTENT));
  assert!(disabled(&mut editor, "editor-undo") && !disabled(&mut editor, "editor-redo"));
  editor.stage.click("editor-redo");
  assert_eq!(editor.document().spans, vec![span(0..4, SpanStyle::new().bold())]);
  // The keys are the text input's own.
  editor.shortcut("z", "KeyZ", false, false);
  assert!(editor.document().spans.is_empty());
  // Redo: Ctrl+Y, or Cmd+Shift+Z on a Mac, which has no Cmd+Y.
  if cfg!(target_vendor = "apple") {
    editor.shortcut("Z", "KeyZ", true, false);
  } else {
    editor.shortcut("y", "KeyY", false, false);
  }
  assert_eq!(editor.document().spans.len(), 1);
  assert_eq!(editor.told().len(), 5, "each of them is a change the app is told");
}

#[test]
fn the_app_sets_the_signal_and_is_not_told() {
  let mut editor = Editor::new(RichText::plain("old"));
  editor.value.set(notes());
  editor.stage.pass();
  assert!(editor.told().is_empty());
  assert_eq!(
    editor.editor.value(),
    Some(notes()),
    "the editor holds what the app set"
  );
  assert!(editor.stage.paints("17 words"));
  // An edit after it starts from what the app set.
  editor.focus();
  editor.type_text("!");
  assert_eq!(editor.document().text, format!("{}!", notes().text));
  assert_eq!(editor.told().len(), 1);
}

#[test]
fn a_held_document_is_told_each_change_and_shows_what_the_app_answers_with() {
  // The app takes a change only while the document has no more than five letters.
  let held = Signal::new(RichText::plain("abc"));
  let log = Log::default();
  let handle = RichTextController::new();
  let (value, told, controller) = (held.clone(), log.clone(), handle.clone());
  let mut stage = Stage::first_frame(640.0, 480.0, move |ctx| {
    let (answer, heard) = (value.clone(), told.clone());
    let editor = RichTextEditor::controlled(value.get(), move |document| {
      heard.lock().unwrap().push(document.text.clone());
      if document.text.len() <= 5 {
        answer.set(document.clone());
      }
    })
    .id(ID)
    .controller(&controller);
    Column::new().size(640.0, 480.0).child(editor.mount(ctx)).into()
  });
  stage.pass();
  stage.tree.get_element_by_id_mut(CONTENT).unwrap().focus();
  stage.pass();
  for key in ["d", "e", "f"] {
    stage.tree.key_down(key.into(), "Key".into(), false, false, false);
    stage.pass();
  }
  assert_eq!(*log.lock().unwrap(), ["abcd", "abcde", "abcdef"]);
  assert_eq!(held.get().text, "abcde");
  assert_eq!(
    handle.value().map(|document| document.text),
    Some("abcde".to_owned()),
    "the change the app did not take is taken back"
  );
}

#[test]
fn a_read_only_editor_refuses_every_change_and_a_disabled_one_takes_nothing() {
  let mut editor = Editor::with(640.0, RichText::plain("fixed"), |editor| editor.read_only(true));
  editor.focus();
  assert_eq!(editor.focused().as_deref(), Some(CONTENT), "it is still a stop of Tab");
  editor.type_text("x");
  editor.select(0, 5);
  editor.shortcut("b", "KeyB", false, false);
  editor.stage.click("editor-bold");
  assert_eq!(editor.document(), RichText::plain("fixed"));
  assert!(editor.told().is_empty());
  assert_eq!(editor.stage.fill_color(CONTENT), Some(light(PaletteColor::Muted)));
  assert!(editor.dimmed("editor-bold"), "its tools take no input");

  let mut editor = Editor::with(640.0, RichText::plain("fixed"), |editor| editor.disabled(true));
  editor.stage.click(CONTENT);
  assert_ne!(editor.focused().as_deref(), Some(CONTENT));
  editor.type_text("x");
  assert_eq!(editor.document(), RichText::plain("fixed"));
}

#[test]
fn an_empty_editor_shows_its_placeholder_and_keeps_its_rows() {
  let mut editor = Editor::with(640.0, RichText::default(), |editor| {
    editor.placeholder("Write the release notes").rows(4, 8)
  });
  let (color, size) = editor.stage.text("Write the release notes");
  assert_eq!((color, size), (light(PaletteColor::MutedForeground), 14.0));
  let (.., height) = editor.stage.bounds(CONTENT);
  assert!(
    height >= 32.0 + 4.0 * 18.0 - 0.5,
    "four rows of body text and its padding: {height}"
  );
  assert!(editor.stage.paints("0 words"));
  // Past its most rows the text scrolls in the box.
  editor.value.set(RichText::plain("line\n".repeat(30)));
  editor.stage.pass();
  let (.., grown) = editor.stage.bounds(CONTENT);
  assert!(grown < 32.0 + 9.0 * 20.0, "no higher than eight rows: {grown}");
}

#[test]
fn the_link_tool_opens_a_field_that_links_the_selection_and_gives_the_focus_back() {
  let mut editor = Editor::new(RichText::plain("see the docs"));
  editor.focus();
  editor.select(8, 12);
  editor.stage.click("editor-link");
  editor.stage.pass();
  assert_eq!(
    editor.focused().as_deref(),
    Some("editor-link-field"),
    "the field of the link has the focus"
  );
  editor.type_text("https://lurq.dev");
  editor.stage.key("Enter");
  editor.stage.pass();
  assert_eq!(
    editor.document().spans,
    vec![span(8..12, SpanStyle::new().link("https://lurq.dev"))]
  );
  assert_eq!(editor.told().last().unwrap(), "see the [docs](https://lurq.dev)");
  assert_eq!(
    editor.focused().as_deref(),
    Some(CONTENT),
    "the focus is back in the text"
  );
  assert!(editor.on("link"), "the tool is on while the selection is a link");
  // Opened on a link, the field shows its target, and an empty one takes the link away.
  editor.shortcut("k", "KeyK", false, false);
  editor.stage.pass();
  let field = editor.stage.tree.get_element_by_id_mut("editor-link-field").unwrap();
  assert_eq!(field.as_text_input().unwrap().value(), "https://lurq.dev");
  editor.shortcut("a", "KeyA", false, false);
  editor.stage.key("Backspace");
  editor.stage.key("Enter");
  editor.stage.pass();
  assert!(editor.document().spans.is_empty());
  assert_eq!(editor.focused().as_deref(), Some(CONTENT));
}

#[test]
fn escape_closes_the_field_of_a_link_and_changes_nothing() {
  let mut editor = Editor::new(RichText::plain("see the docs"));
  editor.focus();
  editor.select(8, 12);
  editor.shortcut("k", "KeyK", false, false);
  editor.stage.pass();
  editor.type_text("https://lurq.dev");
  editor.stage.key("Escape");
  editor.stage.pass();
  assert!(editor.document().spans.is_empty());
  assert!(editor.told().is_empty());
  assert_eq!(editor.focused().as_deref(), Some(CONTENT));
}

#[test]
fn the_image_tool_is_there_with_its_handler_and_runs_it() {
  let mut editor = Editor::new(RichText::default());
  assert!(
    editor.stage.tree.get_element_by_id_mut("editor-image").is_none(),
    "no tool without a handler"
  );
  let asked = Log::default();
  let heard = asked.clone();
  let mut editor = Editor::with(640.0, RichText::plain("a "), move |editor| {
    let heard = heard.clone();
    editor.on_image(move || heard.lock().unwrap().push("image".into()))
  });
  let (link_x, ..) = editor.stage.bounds("editor-link");
  let (image_x, ..) = editor.stage.bounds("editor-image");
  assert!(close(image_x - (link_x + 28.0), 2.0), "after the link, in its group");
  editor.focus();
  editor.stage.click("editor-image");
  assert_eq!(*asked.lock().unwrap(), ["image"]);
  // The app puts what stands for the picture in the text through the controller.
  editor.editor.insert_link("diagram.png", "files/diagram.png");
  editor.stage.pass();
  assert_eq!(editor.told().last().unwrap(), "a [diagram.png](files/diagram.png)");
}

#[test]
fn the_tools_the_app_names_are_the_bars_and_the_bar_and_the_footer_can_be_left_out() {
  let mut editor = Editor::with(640.0, RichText::plain("text"), |editor| {
    editor.tools([RichTextTool::Bold, RichTextTool::Italic, RichTextTool::Undo])
  });
  let exists = |editor: &mut Editor, id: &str| editor.stage.tree.get_element_by_id_mut(id).is_some();
  assert!(exists(&mut editor, "editor-bold") && exists(&mut editor, "editor-undo"));
  assert!(!exists(&mut editor, "editor-style") && !exists(&mut editor, "editor-link"));
  let (x, .., width, _) = editor.stage.bounds(ID);
  let (undo_x, ..) = editor.stage.bounds("editor-undo");
  assert!(close(undo_x + 28.0, x + width - 6.0), "the history is at the end");

  let mut editor = Editor::with(640.0, RichText::plain("text"), |editor| {
    editor.toolbar(false).footer(false)
  });
  assert!(!exists(&mut editor, "editor-toolbar") && !exists(&mut editor, "editor-count"));
  let (.., height) = editor.stage.bounds(ID);
  let (.., content) = editor.stage.bounds(CONTENT);
  assert_eq!(height, content, "the box alone");
  // The keys still format.
  editor.focus();
  editor.select(0, 4);
  editor.shortcut("b", "KeyB", false, false);
  assert_eq!(editor.document().spans.len(), 1);
}

#[test]
fn the_words_of_the_editor_are_the_apps_to_say() {
  let mut editor = Editor::with(640.0, notes(), |editor| {
    editor
      .shortcuts_text("Shortcuts are on")
      .count_text(|words| format!("words: {words}"))
      .paragraph_text("Body")
      .heading_text(|level| format!("Title {level}"))
  });
  assert!(editor.stage.paints("Shortcuts are on") && editor.stage.paints("words: 17"));
  editor.focus();
  editor.select(2, 2);
  assert!(editor.stage.paints("Title 2"));
}

#[test]
fn a_paragraph_longer_than_the_box_wraps_inside_it() {
  let long = "A sentence that goes on for much longer than one row of the box can hold, so that it has to wrap onto \
              a second row and a third one before it comes to its end at last, some way down the box.";
  let mut editor = Editor::new(RichText::new(long, vec![], vec![Bullet]));
  let input = editor.editor.input();
  let (start, end) = (input.caret_point(0).unwrap(), input.caret_point(long.len()).unwrap());
  assert!(end.y > start.y + 14.0, "it wraps onto more rows");
  assert!(close(start.x, 21.0), "the rows of an item start after its indent");
  let (.., width, _) = editor.stage.bounds(CONTENT);
  assert!(end.x <= width - 36.0 + 0.5, "no row runs past the padding of the box");
}

#[test]
fn in_a_narrow_room_the_editor_is_as_wide_as_the_room_and_its_bar_puts_tools_in_a_menu() {
  let mut editor = Editor::with(320.0, notes(), |editor| editor);
  let (x, _, width, _) = editor.stage.bounds(ID);
  assert_eq!((x, width), (0.0, 320.0));
  // The style and the first tools stay, the last are behind the More button, at the end of the bar.
  let (more_x, _, more_width, _) = editor.stage.bounds("editor-toolbar-more");
  assert!(close(more_x + more_width, 320.0 - 6.0), "{more_x} {more_width}");
  let (bold_x, ..) = editor.stage.bounds("editor-bold");
  assert!(bold_x < more_x);
  let (style_x, _, style_width, _) = editor.stage.bounds("editor-style");
  assert!(style_x >= 0.0 && style_x + style_width < more_x);
  assert!(editor.stage.paints("17 words"));
  for quad in editor.stage.quads() {
    if let QuadContent::Text { text, .. } = &quad.content {
      assert!(
        quad.x + quad.width <= 320.5,
        "{text:?} is laid out past the room, at {}",
        quad.x
      );
    }
  }
}

#[test]
fn the_first_frame_of_a_narrow_editor_is_the_settled_one() {
  let value = Signal::new(notes());
  let mut stage = Stage::first_frame(320.0, 480.0, move |ctx| {
    Column::new()
      .size(320.0, 480.0)
      .child(RichTextEditor::new(value.clone()).id(ID).mount(ctx))
      .into()
  });
  let first = stage.picture();
  stage.frame();
  assert_eq!(
    first,
    stage.picture(),
    "nothing moves between the first frame and the next"
  );
}

#[test]
fn a_judgment_is_the_color_of_the_edge() {
  let mut editor = Editor::with(640.0, RichText::plain("text"), |editor| editor.invalid(true));
  let edge = editor
    .stage
    .border(ID)
    .and_then(|quad| quad.border)
    .and_then(|border| border.top);
  assert_eq!(edge.map(|side| side.color), Some(light(PaletteColor::Destructive)));
}

#[test]
fn a_rule_of_the_theme_changes_the_editor_and_how_an_item_is_drawn() {
  let app = App::new();
  app.theme().patch_recipe::<RichTextEditor>(|recipe| {
    recipe
      .base(RichTextEditorPart::Root, |s| s.width(480.0))
      .base(RichTextEditorPart::Bullet, |s| s.padding_start(40.0))
  });
  let mut editor = Editor::in_app(app, 640.0, notes(), |editor| editor);
  assert_eq!(editor.stage.bounds(ID).2, 480.0);
  let item = editor.document().text.find("Faster").unwrap();
  let point = editor.editor.input().caret_point(item).unwrap();
  assert!(close(point.x, 40.0), "{}", point.x);
}

#[test]
fn the_still_form_draws_the_document_with_nothing_kept() {
  let mut stage = Stage::first_frame(640.0, 480.0, |_| {
    Column::new()
      .size(640.0, 480.0)
      .child(RichTextEditor::new(Signal::new(notes())).id(ID))
      .into()
  });
  stage.pass();
  assert_eq!(stage.bounds(ID).2, 560.0);
  assert!(stage.paints("17 words"));
  assert_eq!(stage.bounds("editor-bold").2, 28.0);
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn it_is_a_group_with_a_toolbar_of_named_tools_and_a_textbox_of_several_lines() {
  let mut editor = Editor::with(640.0, notes(), |editor| editor.label("Release notes"));
  let describe = |editor: &mut Editor, id: &str, key: &str| {
    editor
      .stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(key))
  };
  assert_eq!(describe(&mut editor, ID, "a11y_role").as_deref(), Some("group"));
  assert_eq!(describe(&mut editor, ID, "a11y_name").as_deref(), Some("Release notes"));
  assert_eq!(describe(&mut editor, CONTENT, "a11y_role").as_deref(), Some("textbox"));
  assert_eq!(
    describe(&mut editor, CONTENT, "a11y_multiline").as_deref(),
    Some("true")
  );
  assert_eq!(
    describe(&mut editor, CONTENT, "a11y_name").as_deref(),
    Some("Release notes")
  );
  assert_eq!(
    describe(&mut editor, "editor-toolbar", "a11y_role").as_deref(),
    Some("toolbar")
  );
  assert_eq!(
    describe(&mut editor, "editor-toolbar", "a11y_name").as_deref(),
    Some("Formatting")
  );
  assert_eq!(
    describe(&mut editor, "editor-bold", "a11y_name").as_deref(),
    Some("Bold")
  );
  assert_eq!(
    describe(&mut editor, "editor-ordered", "a11y_name").as_deref(),
    Some("Numbered list")
  );
  assert_eq!(
    describe(&mut editor, "editor-style", "a11y_name").as_deref(),
    Some("Text style")
  );
}

#[test]
fn a_click_on_a_link_with_the_modifier_tells_the_app_its_target() {
  let opened = Log::default();
  let heard = opened.clone();
  let document = RichText::new(
    "docs",
    vec![span(0..4, SpanStyle::new().link("https://lurq.dev"))],
    vec![],
  );
  let mut editor = Editor::with(640.0, document, move |editor| {
    let heard = heard.clone();
    editor.on_link_click(move |target| heard.lock().unwrap().push(target.to_owned()))
  });
  let (x, y, ..) = editor.stage.bounds(CONTENT);
  let (ctrl, meta) = (cfg!(not(target_vendor = "apple")), cfg!(target_vendor = "apple"));
  let button = lurq::app::events::MouseButton::Left;
  let tree = &mut editor.stage.tree;
  tree.mouse_down_with_meta(x + 26.0, y + 24.0, button, false, ctrl, false, meta);
  tree.mouse_up_with_meta(x + 26.0, y + 24.0, button, false, ctrl, false, meta);
  assert_eq!(*opened.lock().unwrap(), ["https://lurq.dev"]);
}

#[test]
fn a_mounted_field_mounts_the_editor_names_it_and_judges_it() {
  let value = Signal::new(RichText::plain("text"));
  let held = value.clone();
  let mut stage = Stage::first_frame(640.0, 480.0, move |ctx| {
    let field = lurq::ui::Field::new("Release notes", RichTextEditor::new(held.clone()).id(ID))
      .error("Say what changed.")
      .mount(ctx);
    Column::new().size(640.0, 480.0).child(field).into()
  });
  stage.pass();
  let edge = stage
    .border(ID)
    .and_then(|quad| quad.border)
    .and_then(|border| border.top);
  assert_eq!(
    edge.map(|side| side.color),
    Some(light(PaletteColor::Destructive)),
    "the error of the field is the judgment of the editor"
  );
  // It is mounted: what is typed stays, render after render.
  stage.tree.get_element_by_id_mut(CONTENT).unwrap().focus();
  stage.pass();
  for key in ["a", "b"] {
    stage.tree.key_down(key.into(), "Key".into(), false, false, false);
    stage.pass();
  }
  assert_eq!(value.get().text, "textab");
  #[cfg(any(feature = "devtools", feature = "mcp"))]
  {
    let name = stage
      .tree
      .get_element_by_id_mut(CONTENT)
      .and_then(|element| element.description("a11y_name"));
    assert_eq!(name.as_deref(), Some("Release notes"));
  }
}

#[test]
fn in_a_room_of_200_the_editor_and_everything_it_says_stay_in_the_room() {
  let value = Signal::new(notes());
  let mut stage = Stage::new(move |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .child(
        lurq::components::Row::new()
          .id("room")
          .width(200.0)
          .child(RichTextEditor::new(value.clone()).id(ID).mount(ctx)),
      )
      .into()
  });
  stage.pass();
  let (room, _, room_width, _) = stage.bounds("room");
  let (x, _, width, _) = stage.bounds(ID);
  assert!(
    x >= room - 0.5 && x + width <= room + room_width + 0.5,
    "{width} wide at {x}"
  );
  for quad in stage.quads() {
    if let QuadContent::Text { text, .. } = &quad.content {
      assert!(
        quad.x + quad.width <= room + room_width + 0.5,
        "{text:?} is {} wide at {}, past the end of the room",
        quad.width,
        quad.x
      );
    }
  }
}

#[test]
fn what_is_typed_right_after_a_markdown_mark_is_plain() {
  // Typed in one go, as fast as keys come: no frame between the last star and the comma.
  let mut editor = Editor::new(RichText::default());
  editor.focus();
  editor.type_text("a **bold**, *slanted*, ~~struck~~ and `code`.");
  assert_eq!(editor.document().text, "a bold, slanted, struck and code.");
  assert_eq!(
    editor.document().spans,
    vec![
      span(2..6, SpanStyle::new().bold()),
      span(8..15, SpanStyle::new().italic()),
      span(17..23, SpanStyle::new().strikethrough()),
      span(28..32, SpanStyle::new().code()),
    ]
  );
}

#[test]
fn an_empty_editors_placeholder_is_drawn_as_its_first_paragraph_is() {
  // A heading: the placeholder in the heading's type, in the placeholder's color.
  let mut editor = Editor::with(640.0, RichText::new("", vec![], vec![Heading(1)]), |editor| {
    editor.placeholder("Title")
  });
  assert_eq!(editor.stage.text("Title"), (light(PaletteColor::MutedForeground), 28.0));
  let (x, y, ..) = editor.stage.bounds(CONTENT);
  let title = editor.stage.text_quad("Title").unwrap();
  assert!(
    close(title.x, x + 18.0) && close(title.y, y + 16.0),
    "{} {}",
    title.x,
    title.y
  );

  // An item of a list: its dot, and the placeholder after the item's indent.
  let mut editor = Editor::with(640.0, RichText::new("", vec![], vec![Bullet]), |editor| {
    editor.placeholder("An item")
  });
  let item = editor.stage.text_quad("An item").unwrap();
  let (x, ..) = editor.stage.bounds(CONTENT);
  assert!(close(item.x, x + 18.0 + 21.0), "{}", item.x);
  let dots = editor
    .stage
    .quads()
    .into_iter()
    .filter(|quad| quad.width == 5.0 && quad.height == 5.0)
    .count();
  assert_eq!(dots, 1, "the dot of the empty item");

  // What is typed takes the placeholder's place, and the placeholder comes back with the last character gone.
  let mut editor = Editor::with(640.0, RichText::new("", vec![], vec![Heading(2)]), |editor| {
    editor.placeholder("Title")
  });
  editor.focus();
  editor.type_text("N");
  assert!(!editor.stage.paints("Title"));
  editor.stage.key("Backspace");
  assert_eq!(editor.stage.text("Title").1, 22.0);
  assert_eq!(editor.kinds(), [Heading(2)]);
}

/// A clipboard of the test's: the text written to it and the Markdown beside it.
#[derive(Default)]
struct Clipboard {
  text: Mutex<Option<String>>,
  markdown: Mutex<Option<String>>,
}

impl lurq::app::ClipboardWriter for Clipboard {
  fn write_text(&self, text: &str) -> bool {
    *self.text.lock().unwrap() = Some(text.to_owned());
    *self.markdown.lock().unwrap() = None;
    true
  }

  fn write_rich(&self, text: &str, markdown: &str) -> bool {
    *self.text.lock().unwrap() = Some(text.to_owned());
    *self.markdown.lock().unwrap() = Some(markdown.to_owned());
    true
  }
}

impl lurq::app::ClipboardReader for Clipboard {
  fn read_text(&self) -> Option<String> {
    self.text.lock().unwrap().clone()
  }

  fn read(&self) -> Option<lurq::app::events::ClipboardContent> {
    self.read_text().map(lurq::app::events::ClipboardContent::text)
  }

  fn read_markdown(&self, text: &str) -> Option<String> {
    (self.text.lock().unwrap().as_deref() == Some(text))
      .then(|| self.markdown.lock().unwrap().clone())
      .flatten()
  }
}

#[test]
fn what_is_copied_from_the_editor_comes_back_with_its_marks_and_kinds() {
  let mut editor = Editor::new(notes());
  let clipboard = Arc::new(Clipboard::default());
  editor.stage.tree.set_clipboard_writer(Some(clipboard.clone()));
  editor.stage.tree.set_clipboard_reader(Some(clipboard.clone()));
  editor.focus();
  // The paragraph with its bold words and the first item under it.
  let text = editor.document().text;
  let (from, to) = (text.find("Backdrop").unwrap(), text.find("Fixes").unwrap() - 1);
  editor.select(from, to);
  editor.shortcut("c", "KeyC", false, false);
  assert_eq!(
    clipboard.text.lock().unwrap().as_deref(),
    Some("Backdrop blur now follows rounded clips and transforms.\nFaster text shaping"),
    "the clipboard's text is the text"
  );
  assert_eq!(
    clipboard.markdown.lock().unwrap().as_deref(),
    Some("Backdrop blur now **follows rounded clips** and transforms.\n\n- Faster text shaping"),
    "its Markdown is beside it"
  );
  // Pasted into a new paragraph at the end of the document: one change, told once, undone as one.
  let end = text.len();
  editor.select(end, end);
  editor.stage.key("Enter");
  editor.stage.key("Enter");
  assert_eq!(editor.kinds(), [Heading(2), Body, Bullet, Bullet, Body]);
  let told = editor.told().len();
  editor.shortcut("v", "KeyV", false, false);
  assert_eq!(editor.kinds(), [Heading(2), Body, Bullet, Bullet, Body, Bullet]);
  assert!(
    editor
      .document()
      .to_markdown()
      .ends_with("Backdrop blur now **follows rounded clips** and transforms.\n\n- Faster text shaping"),
    "{}",
    editor.document().to_markdown()
  );
  assert_eq!(editor.told().len(), told + 1);
  editor.shortcut("z", "KeyZ", false, false);
  assert_eq!(editor.kinds(), [Heading(2), Body, Bullet, Bullet, Body]);

  // Text from elsewhere that reads as Markdown is the document it says; any other is the text.
  assert!(
    editor
      .stage
      .tree
      .paste(lurq::app::events::ClipboardContent::text("> Quoted **words**"))
  );
  editor.stage.pass();
  assert_eq!(editor.kinds().last(), Some(&Quote));
  assert!(editor.document().text.ends_with("Quoted words"));
  editor.stage.key("Enter");
  editor.stage.key("Enter");
  assert!(
    editor
      .stage
      .tree
      .paste(lurq::app::events::ClipboardContent::text("2 * 3 = 6"))
  );
  editor.stage.pass();
  assert!(editor.document().text.ends_with("2 * 3 = 6"));
  assert_eq!(editor.kinds().last(), Some(&Body));
}

#[cfg(feature = "form")]
#[test]
fn a_named_editor_gives_its_document_to_the_form_as_markdown() {
  use lurq::components::{Form, FormHandle, FormOptions, FormProps, FormValues};
  let submitted = Arc::new(Mutex::new(None::<FormValues>));
  let held = submitted.clone();
  let document = Signal::new(notes());
  let bound = document.clone();
  let mut stage = Stage::new(move |ctx| {
    let held = held.clone();
    Form::element(
      FormProps::new(FormHandle::new(FormOptions::new()).on_submit(move |values| {
        *held.lock().unwrap() = Some(values);
      })),
      Column::new()
        .size(640.0, 480.0)
        .child(RichTextEditor::new(bound.clone()).id(ID).name("notes").mount(ctx))
        .child(
          lurq::ui::Input::new(Signal::new("Ivan".to_owned()))
            .name("who")
            .id("who"),
        ),
    )
  });
  stage.pass();
  let submit = |stage: &mut Stage| {
    stage.tree.get_element_by_id_mut("who").unwrap().focus();
    stage.pass();
    stage.key("Enter");
    submitted.lock().unwrap().take().expect("the form is submitted")
  };
  let values = submit(&mut stage);
  assert_eq!(values.get_string("notes"), Some(notes().to_markdown().as_str()));
  assert_eq!(values.get_string("who"), Some("Ivan"));
  // The document as it is when the form is sent: an edit is in what is sent next.
  document.set(RichText::from_markdown("# Changed\n\n- **one**"));
  stage.pass();
  let values = submit(&mut stage);
  assert_eq!(values.get_string("notes"), Some("# Changed\n\n- **one**"));
}

#[test]
fn the_panel_of_a_link_hangs_from_the_selection_a_press_made() {
  let mut editor = Editor::new(notes());
  editor.focus();
  // The caret at the end of the document first, then a press in the heading puts it there.
  let end = editor.document().text.len();
  editor.select(end, end);
  editor.stage.pass();
  let (x, y, ..) = editor.stage.bounds(CONTENT);
  let (px, py) = (x + 18.0 + 30.0, y + 16.0 + 13.0);
  editor.stage.tree.mouse_move(px, py);
  editor
    .stage
    .tree
    .mouse_down(px, py, lurq::app::events::MouseButton::Left);
  editor.stage.tree.mouse_up(px, py, lurq::app::events::MouseButton::Left);
  editor.stage.pass();
  let caret = editor.editor.input().selection().unwrap().range().start;
  assert!(caret < "Release notes".len(), "the caret is in the heading: {caret}");
  editor.shortcut("k", "KeyK", false, false);
  editor.stage.pass();
  editor.stage.pass();
  let (_, field_y, ..) = editor.stage.bounds("editor-link-field");
  assert!(
    field_y < y + 16.0 + 26.0 + 30.0,
    "the panel is under the heading, not under the last item: {field_y} in a box at {y}"
  );
}

/// The color the tests draw a picture in.
const PICTURE: Color = Color::new(200, 30, 60, 255);

/// An editor of `document` whose pictures are drawn as a red box 50 high, 80 wide or as wide as the document says.
fn pictured(document: RichText) -> Editor {
  Editor::with(640.0, document, |editor| {
    editor.picture(|picture| {
      let width = picture.width.unwrap_or(80) as f32;
      lurq::components::Rect::new(width, 50.0).background(PICTURE).into()
    })
  })
}

/// Where a picture is painted.
#[derive(Clone, Copy, Debug)]
struct Drawn {
  x: f32,
  y: f32,
  width: f32,
}

/// The boxes of the pictures as they are painted, from the top of the document down.
fn pictures(editor: &Editor) -> Vec<Drawn> {
  let mut drawn: Vec<_> = editor
    .stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(quad.content, QuadContent::Rect { color, .. } if color == PICTURE))
    .map(|quad| Drawn {
      x: quad.x,
      y: quad.y,
      width: quad.width,
    })
    .collect();
  drawn.sort_by(|a, b| a.y.total_cmp(&b.y));
  drawn
}

fn press(editor: &mut Editor, x: f32, y: f32) {
  editor.stage.tree.mouse_move(x, y);
  editor.stage.tree.mouse_down(x, y, lurq::app::events::MouseButton::Left);
  editor.stage.tree.mouse_up(x, y, lurq::app::events::MouseButton::Left);
  editor.stage.pass();
}

#[test]
fn the_pictures_of_a_document_are_drawn_in_its_text_and_are_no_words() {
  let source = "One ![the lake](lake.jpg) two.\n\n<img src=\"hill.png\" alt=\"the hill\" width=\"200\">\n\nThree.";
  let mut editor = pictured(RichText::from_markdown(source));
  let drawn = pictures(&editor);
  assert_eq!(drawn.len(), 2);
  let (content_x, content_y, ..) = editor.stage.bounds(CONTENT);
  // The one among words stands after the first of them, in the first paragraph.
  assert!(drawn[0].x > content_x + 18.0 + 20.0 && close(drawn[0].width, 80.0));
  assert!(drawn[0].y >= content_y + 16.0 - 0.6);
  // The one alone in its paragraph starts where the text does, as wide as the document says, under the first.
  assert!(close(drawn[1].x, content_x + 18.0) && close(drawn[1].width, 200.0));
  assert!(drawn[1].y >= drawn[0].y + 50.0);
  assert!(editor.stage.paints("3 words"), "a picture is no word");
  assert_eq!(editor.document().to_markdown(), source);
}

#[test]
fn the_controller_puts_a_picture_at_the_caret_and_undo_takes_it_back() {
  let mut editor = pictured(RichText::plain("ab"));
  editor.focus();
  editor.select(1, 1);
  assert!(editor.editor.insert_picture("lake.jpg", "The lake"));
  editor.stage.pass();
  assert_eq!(editor.document().to_markdown(), "a![The lake](lake.jpg)b");
  assert_eq!(
    editor.told().last().map(String::as_str),
    Some("a![The lake](lake.jpg)b")
  );
  assert_eq!(pictures(&editor).len(), 1);
  // The caret is after it: what is typed next follows it.
  editor.type_text("!");
  assert_eq!(editor.document().to_markdown(), "a![The lake](lake.jpg)!b");
  // One with a width of its own, in place of a selection.
  editor.select(0, 1);
  assert!(
    editor
      .editor
      .insert_object(&lurq::components::InlineObject::new(0, "hill.png").width(120))
  );
  editor.stage.pass();
  assert_eq!(
    editor.document().to_markdown(),
    "<img src=\"hill.png\" alt=\"\" width=\"120\">![The lake](lake.jpg)!b"
  );
  let drawn = pictures(&editor);
  assert!(close(drawn[0].width, 120.0) && close(drawn[1].width, 80.0), "{drawn:?}");
  // Each is one step of the undo.
  assert!(editor.editor.undo());
  editor.stage.pass();
  assert_eq!(editor.document().to_markdown(), "a![The lake](lake.jpg)!b");
  assert!(editor.editor.undo() && editor.editor.undo());
  editor.stage.pass();
  assert_eq!(editor.document().to_markdown(), "ab");
  assert!(pictures(&editor).is_empty());
  assert!(editor.editor.redo());
  editor.stage.pass();
  assert_eq!(editor.document().objects.len(), 1);
}

#[test]
fn the_image_tool_runs_the_apps_handler_and_the_app_puts_the_picture_in() {
  let asked = Arc::new(Mutex::new(0));
  let count = asked.clone();
  let mut editor = Editor::with(640.0, RichText::plain("ab"), move |editor| {
    let count = count.clone();
    editor
      .picture(|_| lurq::components::Rect::new(80.0, 50.0).background(PICTURE).into())
      .on_image(move || *count.lock().unwrap() += 1)
  });
  editor.focus();
  editor.stage.click("editor-image");
  assert_eq!(*asked.lock().unwrap(), 1);
  assert_eq!(
    editor.focused().as_deref(),
    Some(CONTENT),
    "the focus stays in the text"
  );
  // What the app does once a person chose a file.
  assert!(editor.editor.insert_picture("photos/lake.jpg", "The lake"));
  editor.stage.pass();
  assert_eq!(editor.document().to_markdown(), "ab![The lake](photos/lake.jpg)");
  assert_eq!(pictures(&editor).len(), 1);
}

#[test]
fn a_press_on_a_picture_selects_it_and_backspace_deletes_it() {
  let mut editor = pictured(RichText::from_markdown("ab ![the lake](lake.jpg) cd"));
  editor.focus();
  let drawn = pictures(&editor)[0];
  press(&mut editor, drawn.x + 30.0, drawn.y + 20.0);
  assert_eq!(editor.editor.input().selection().unwrap().range(), 3..6);
  editor.stage.key("Backspace");
  assert_eq!(editor.document().to_markdown(), "ab  cd");
  assert!(pictures(&editor).is_empty());
  // Backspace after one selects it first, and deletes it at the second press.
  editor.shortcut("z", "KeyZ", false, false);
  assert_eq!(editor.document().objects.len(), 1);
  editor.select(6, 6);
  editor.stage.key("Backspace");
  assert_eq!(editor.editor.input().selection().unwrap().range(), 3..6);
  assert_eq!(editor.document().objects.len(), 1, "the first press deletes nothing");
  editor.stage.key("Backspace");
  assert!(editor.document().objects.is_empty());
}

#[test]
fn a_line_of_code_and_an_editor_that_takes_no_change_take_no_picture() {
  let mut editor = pictured(RichText::from_markdown("```\nlet a = 1;\n```"));
  editor.focus();
  assert!(!editor.editor.insert_picture("lake.jpg", "The lake"));
  assert!(editor.document().objects.is_empty());
  let mut editor = Editor::with(640.0, RichText::plain("ab"), |editor| editor.read_only(true));
  editor.focus();
  assert!(!editor.editor.insert_picture("lake.jpg", "The lake"));
  assert_eq!(editor.document().to_markdown(), "ab");
}

#[test]
fn a_picture_the_app_does_not_draw_is_a_picture_of_its_source() {
  // The editor's own picture loads the file the document names: one that is not there is a box that says so,
  // and the document keeps it as it was written.
  let source = "A lake: ![The lake](no/such/lake.jpg)";
  let mut editor = Editor::new(RichText::from_markdown(source));
  editor.stage.pass();
  editor.focus();
  assert_eq!(editor.document().to_markdown(), source);
  // It is a character of the text like any other picture: selected by the arrows, deleted, and brought back.
  let end = editor.document().text.len();
  editor.select(end, end);
  editor.stage.key("Backspace");
  assert_eq!(editor.editor.input().selection().unwrap().range(), end - 3..end);
  editor.stage.key("Backspace");
  assert_eq!(editor.document().to_markdown(), "A lake: ");
  editor.shortcut("z", "KeyZ", false, false);
  assert_eq!(editor.document().to_markdown(), source);
}

/// A clipboard that holds a picture as pixels, two by two: what a screenshot is.
fn screenshot() -> lurq::app::events::ClipboardContent {
  let image = lurq::app::events::ClipboardImage::from_rgba(2, 2, vec![255; 16]).expect("four pixels");
  lurq::app::events::ClipboardContent::image(image)
}

#[test]
fn a_pasted_screenshot_is_given_to_the_app_to_keep_and_put_in_as_the_picture_it_names() {
  let kept = Arc::new(Mutex::new(Vec::new()));
  let store = kept.clone();
  let mut editor = Editor::with(640.0, RichText::plain("ab"), move |editor| {
    let store = store.clone();
    editor
      .picture(|_| lurq::components::Rect::new(80.0, 50.0).background(PICTURE).into())
      .on_paste_image(move |image| {
        let mut store = store.lock().unwrap();
        store.push((image.width(), image.height()));
        Some(format!("attachments/pasted-{}.png", store.len()))
      })
  });
  editor.focus();
  editor.select(1, 1);
  assert!(editor.stage.tree.paste(screenshot()));
  editor.stage.pass();
  assert_eq!(*kept.lock().unwrap(), [(2, 2)], "the app was given the pixels once");
  assert_eq!(editor.document().to_markdown(), "a![](attachments/pasted-1.png)b");
  assert_eq!(pictures(&editor).len(), 1);
  // It is one step of the undo.
  assert!(editor.editor.undo());
  editor.stage.pass();
  assert_eq!(editor.document().to_markdown(), "ab");
}

#[test]
fn a_pasted_screenshot_does_nothing_where_the_app_keeps_none_or_the_editor_takes_no_change() {
  // No handler: a document names its pictures and has nowhere to keep pixels.
  let mut editor = pictured(RichText::plain("ab"));
  editor.focus();
  editor.stage.tree.paste(screenshot());
  editor.stage.pass();
  assert_eq!(editor.document().to_markdown(), "ab");
  // A handler that refuses puts nothing in.
  let mut editor = Editor::with(640.0, RichText::plain("ab"), |editor| editor.on_paste_image(|_| None));
  editor.focus();
  editor.stage.tree.paste(screenshot());
  editor.stage.pass();
  assert_eq!(editor.document().to_markdown(), "ab");
  // Read only, the app is not asked.
  let asked = Arc::new(Mutex::new(0));
  let count = asked.clone();
  let mut editor = Editor::with(640.0, RichText::plain("ab"), move |editor| {
    let count = count.clone();
    editor.read_only(true).on_paste_image(move |_| {
      *count.lock().unwrap() += 1;
      Some("kept.png".to_owned())
    })
  });
  editor.focus();
  editor.stage.tree.paste(screenshot());
  editor.stage.pass();
  assert_eq!(
    (*asked.lock().unwrap(), editor.document().to_markdown().as_str()),
    (0, "ab")
  );
}

#[test]
fn a_clipboard_with_words_is_pasted_as_text_and_pasted_picture_files_go_in_by_their_paths() {
  let mut editor = Editor::with(640.0, RichText::plain("ab"), |editor| {
    editor
      .picture(|_| lurq::components::Rect::new(80.0, 50.0).background(PICTURE).into())
      .on_paste_image(|_| Some("kept.png".to_owned()))
  });
  editor.focus();
  // Words and pixels at once, as a sheet's cells are copied: the words are what is pasted.
  let both = screenshot().with_text("words");
  assert!(editor.stage.tree.paste(both));
  editor.stage.pass();
  assert_eq!(editor.document().to_markdown(), "abwords");
  // Files copied in a file manager: the pictures among them, each called by its name.
  let files = lurq::app::events::ClipboardContent::files([
    std::path::PathBuf::from("photos/lake.JPG"),
    std::path::PathBuf::from("notes.txt"),
    std::path::PathBuf::from("photos/hill.png"),
  ]);
  assert!(editor.stage.tree.paste(files));
  editor.stage.pass();
  assert_eq!(
    editor.document().to_markdown(),
    "abwords![lake](photos/lake.JPG)![hill](photos/hill.png)"
  );
  assert_eq!(pictures(&editor).len(), 2);
}

/// Drags from `(x, y)` by `dx`, as the pointer drags the handle of a picture.
fn drag(editor: &mut Editor, x: f32, y: f32, dx: f32) {
  let left = lurq::app::events::MouseButton::Left;
  editor.stage.tree.mouse_move(x, y);
  editor.stage.tree.mouse_down(x, y, left);
  editor.stage.tree.mouse_move(x + dx * 0.5, y);
  editor.stage.pass();
  editor.stage.tree.mouse_move(x + dx, y);
  editor.stage.tree.mouse_up(x + dx, y, left);
  editor.stage.pass();
}

#[test]
fn the_handle_of_a_selected_picture_makes_it_wider_and_the_app_is_told_once() {
  let mut editor = pictured(RichText::from_markdown("ab ![the lake](lake.jpg) cd"));
  editor.focus();
  let drawn = pictures(&editor)[0];
  press(&mut editor, drawn.x + 30.0, drawn.y + 20.0);
  assert_eq!(editor.editor.input().selection().unwrap().range(), 3..6);
  let told = editor.told().len();
  // The handle is on the lower corner at the end of the picture, which is 80 by 50.
  drag(&mut editor, drawn.x + 80.0, drawn.y + 50.0, 40.0);
  assert_eq!(editor.document().objects[0].width, Some(120));
  assert_eq!(
    editor.document().to_markdown(),
    "ab <img src=\"lake.jpg\" alt=\"the lake\" width=\"120\"> cd"
  );
  assert!(close(pictures(&editor)[0].width, 120.0));
  assert_eq!(
    editor.told().len(),
    told + 1,
    "told when the handle is let go, not at each step of the drag"
  );
  // Undo takes the whole drag back.
  assert!(editor.editor.undo());
  editor.stage.pass();
  assert_eq!(editor.document().to_markdown(), "ab ![the lake](lake.jpg) cd");
}

#[test]
fn a_read_only_editor_has_no_handle_on_its_pictures() {
  let mut editor = Editor::with(
    640.0,
    RichText::from_markdown("ab ![the lake](lake.jpg) cd"),
    |editor| {
      editor
        .read_only(true)
        .picture(|_| lurq::components::Rect::new(80.0, 50.0).background(PICTURE).into())
    },
  );
  editor.focus();
  let drawn = pictures(&editor)[0];
  press(&mut editor, drawn.x + 30.0, drawn.y + 20.0);
  drag(&mut editor, drawn.x + 79.0, drawn.y + 49.0, 40.0);
  assert_eq!(editor.document().objects[0].width, None);
}

#[test]
fn what_is_written_after_a_picture_goes_after_it_and_enter_starts_a_paragraph_under_it() {
  let mut editor = Editor::with(640.0, RichText::plain("ab"), |editor| {
    editor
      .picture(|_| lurq::components::Rect::new(80.0, 50.0).background(PICTURE).into())
      .on_paste_image(|_| Some("kept.png".to_owned()))
  });
  editor.focus();
  // Pasted, the picture has the caret after it: what is typed next follows it.
  assert!(editor.stage.tree.paste(screenshot()));
  editor.stage.pass();
  editor.type_text("c");
  assert_eq!(editor.document().to_markdown(), "ab![](kept.png)c");
  // A press on the picture selects it. Enter then does not take its place: a paragraph starts after it.
  let drawn = pictures(&editor)[0];
  press(&mut editor, drawn.x + 30.0, drawn.y + 20.0);
  assert_eq!(editor.editor.input().selection().unwrap().range(), 2..5);
  editor.stage.key("Enter");
  editor.type_text("d");
  assert_eq!(editor.document().to_markdown(), "ab![](kept.png)\n\ndc");
  assert_eq!(pictures(&editor).len(), 1, "the picture is still there");
  // The arrow from a selected picture puts the caret after it.
  let drawn = pictures(&editor)[0];
  press(&mut editor, drawn.x + 30.0, drawn.y + 20.0);
  editor.stage.key("ArrowRight");
  editor.type_text("e");
  assert_eq!(editor.document().to_markdown(), "ab![](kept.png)e\n\ndc");
}

#[test]
fn a_recipe_spaces_the_items_of_a_list_and_the_room_round_a_quote() {
  let document = RichText::from_markdown("Intro\n\n1. one\n2. two\n\n> said\n\nafter");
  let mut editor = Editor::with(640.0, document, |editor| {
    editor
      .style(RichTextEditorPart::Ordered, |s| s.gap(4.0))
      .style(RichTextEditorPart::OrderedMark, |s| s.font_size(12.0))
      .style(RichTextEditorPart::Quote, |s| s.padding_top(6.0).padding_bottom(8.0))
  });
  editor.focus();
  let input = editor.editor.input();
  let text = editor.document().text;
  let top = |word: &str| input.caret_point(text.find(word).unwrap()).unwrap().y;
  // Rows of 18, 10 between two blocks unless a part says otherwise.
  assert!(
    close(top("one") - top("Intro"), 18.0 + 10.0),
    "{}",
    top("one") - top("Intro")
  );
  assert!(
    close(top("two") - top("one"), 18.0 + 4.0),
    "the items' gap: {}",
    top("two") - top("one")
  );
  assert!(
    close(top("said") - top("two"), 18.0 + 10.0 + 6.0),
    "above the quote: {}",
    top("said") - top("two")
  );
  assert!(
    close(top("after") - top("said"), 18.0 + 8.0 + 10.0),
    "under it: {}",
    top("after") - top("said")
  );
}

#[test]
fn a_checklist_is_typed_checked_and_ended_and_three_hyphens_are_a_divider() {
  const DONE: ParagraphKind = ParagraphKind::Checklist { checked: true };
  const TODO: ParagraphKind = ParagraphKind::Checklist { checked: false };
  let mut editor = Editor::new(RichText::default());
  editor.focus();
  editor.type_text("[ ] ");
  assert_eq!((editor.document().text.as_str(), editor.kinds()), ("", vec![TODO]));
  editor.type_text("ship");
  // Ctrl+Enter checks the item the caret is in, and again unchecks it.
  editor.shortcut("Enter", "Enter", false, false);
  assert_eq!(editor.kinds(), vec![DONE]);
  assert_eq!(editor.told().last().unwrap(), "- [x] ship");
  assert_eq!(editor.document().text, "ship", "and types no line");
  // Enter starts an item that is not checked; in an empty one it ends the list.
  editor.stage.key("Enter");
  assert_eq!(editor.kinds(), vec![DONE, TODO]);
  editor.stage.key("Enter");
  assert_eq!(editor.kinds(), vec![DONE, Body]);
  editor.type_text("---");
  editor.stage.key("Enter");
  assert_eq!(editor.kinds(), vec![DONE, ParagraphKind::Divider, Body]);
  assert_eq!(editor.document().text, "ship\n\n");
  // `[x]` after the bullet's `-` makes a checked item.
  editor.type_text("- [x] ");
  assert_eq!(editor.kinds(), vec![DONE, ParagraphKind::Divider, DONE]);
}

#[test]
fn the_tools_of_a_checklist_and_a_divider_are_the_apps_to_put_on_the_bar() {
  let mut editor = Editor::with(640.0, RichText::plain("one\ntwo"), |editor| {
    editor.tools([RichTextTool::Checklist, RichTextTool::Divider])
  });
  editor.focus();
  editor.select(0, 5);
  editor.stage.click("editor-checklist");
  let todo = ParagraphKind::Checklist { checked: false };
  assert_eq!(editor.kinds(), vec![todo, todo]);
  assert!(editor.on("checklist"));
  editor.select(7, 7);
  editor.stage.click("editor-divider");
  assert_eq!(editor.kinds(), vec![todo, todo, ParagraphKind::Divider, Body]);
  assert_eq!(editor.told().last().unwrap(), "- [ ] one\n- [ ] two\n\n---\n\n");
  // Not on the bar unless the app names them.
  let mut plain = Editor::new(RichText::plain("one"));
  assert!(plain.stage.tree.get_element_by_id_mut("editor-checklist").is_none());
}

#[test]
fn a_recipe_of_inline_code_keeps_room_round_it_and_the_theme_is_kept_without_one() {
  let document = RichText::from_markdown("`ab` cd");
  let after = |editor: &Editor| {
    let text = editor.document().text;
    editor.editor.input().caret_point(text.find("cd").unwrap()).unwrap().x
  };
  let plain = Editor::new(document.clone());
  let padded = Editor::with(640.0, document, |editor| {
    editor.style(RichTextEditorPart::InlineCode, |s| s.padding_x(5.0))
  });
  assert!(
    close(after(&padded) - after(&plain), 10.0),
    "5 before the code and 5 after it: {}",
    after(&padded) - after(&plain)
  );
}

#[test]
fn an_unfocused_editor_keeps_its_selection_across_renders() {
  let mut editor = Editor::with(640.0, notes(), |editor| editor.keep_selection(true));
  editor.select(2, 6);
  let selection = |editor: &Editor| editor.editor.input().selection().ok().map(|s| (s.anchor, s.caret));
  assert_eq!(selection(&editor), Some((Some(2), 6)), "selected while unfocused");
  // The app sets the same document again, and the editor is rendered again.
  editor.value.set(notes());
  editor.stage.pass();
  editor.stage.pass();
  assert_eq!(selection(&editor), Some((Some(2), 6)), "kept across the same document");
}

#[test]
fn an_unfocused_editor_keeps_its_selection_across_a_document_read_again_and_a_new_appearance() {
  let mut editor = Editor::with(640.0, notes(), |editor| editor.keep_selection(true));
  let selection = |editor: &Editor| editor.editor.input().selection().ok().map(|s| (s.anchor, s.caret));
  editor.select(2, 6);
  // The same document as Markdown writes it and reads it again: equal in its text.
  editor.value.set(RichText::from_markdown(&notes().to_markdown()));
  editor.stage.pass();
  assert_eq!(
    selection(&editor),
    Some((Some(2), 6)),
    "kept across the document read again"
  );
  editor
    .stage
    .app
    .theme()
    .set_preferred_appearance(Some(lurq::app::window::Appearance::Dark));
  editor.stage.pass();
  editor.stage.pass();
  assert_eq!(selection(&editor), Some((Some(2), 6)), "kept across the appearance");
}

#[test]
fn an_editor_selected_before_its_first_frame_keeps_the_selection() {
  let mut editor = Editor::with(640.0, notes(), |editor| editor.keep_selection(true));
  let selection = |editor: &Editor| editor.editor.input().selection().ok().map(|s| (s.anchor, s.caret));
  // Selected and then rendered with a document of its own: a new one resets it, the same one does not.
  assert!(editor.editor.select(2, 6));
  editor.value.set(notes());
  editor.stage.pass();
  assert_eq!(selection(&editor), Some((Some(2), 6)));
}

#[test]
fn an_editor_that_lost_the_focus_keeps_its_selection_across_renders() {
  let mut editor = Editor::with(640.0, notes(), |editor| editor.keep_selection(true));
  let selection = |editor: &Editor| editor.editor.input().selection().ok().map(|s| (s.anchor, s.caret));
  editor.focus();
  editor.select(2, 6);
  editor.stage.tree.get_element_by_id_mut("elsewhere").unwrap().focus();
  editor.stage.pass();
  assert_eq!(selection(&editor), Some((Some(2), 6)), "kept as the focus goes");
  editor.value.set(RichText::from_markdown(&notes().to_markdown()));
  editor.stage.pass();
  assert_eq!(
    selection(&editor),
    Some((Some(2), 6)),
    "kept across the document read again"
  );
  editor
    .stage
    .app
    .theme()
    .set_preferred_appearance(Some(lurq::app::window::Appearance::Dark));
  editor.stage.pass();
  assert_eq!(selection(&editor), Some((Some(2), 6)), "kept across the appearance");
  assert!(editor.editor.select(1, 4));
  editor.stage.pass();
  assert_eq!(
    selection(&editor),
    Some((Some(1), 4)),
    "selected by the app while unfocused"
  );
}

#[test]
fn an_editor_mounted_again_keeps_the_selection_its_controller_holds() {
  let value = Signal::new(notes());
  let controller = RichTextController::new();
  let shown = Signal::new(false);
  let (held, handle, toolbar) = (value.clone(), controller.clone(), shown.clone());
  let mut stage = Stage::first_frame_in(App::new(), 640.0, 480.0, move |ctx| {
    let mut page = Column::new().size(640.0, 480.0);
    // A toolbar over the selection, put before the editor once there is one.
    if toolbar.get() {
      page = page.child(lurq::components::Button::new("Bold").id("bold"));
    }
    page
      .child(
        RichTextEditor::new(held.clone())
          .id(ID)
          .controller(&handle)
          .keep_selection(true)
          .mount(ctx),
      )
      .into()
  });
  stage.pass();
  let selection = || controller.input().selection().ok().map(|s| (s.anchor, s.caret));
  assert!(controller.select(2, 6));
  stage.pass();
  shown.set(true);
  stage.pass();
  stage.pass();
  assert_eq!(selection(), Some((Some(2), 6)), "kept as the toolbar comes");
}

#[test]
fn a_controlled_editor_of_markdown_keeps_its_selection_across_renders() {
  let markdown = Signal::new(
    "# Brief\n\nRead **this** and [that](https://example.com).\n\n- one\n- two\n\n```rust\nfn main() {}\n```\n\n> said\n"
      .to_owned(),
  );
  let controller = RichTextController::new();
  let tick = Signal::new(0);
  let (held, handle, ticks) = (markdown.clone(), controller.clone(), tick.clone());
  let mut stage = Stage::first_frame_in(App::new(), 640.0, 480.0, move |ctx| {
    ticks.get();
    let written = held.clone();
    Column::new()
      .size(640.0, 480.0)
      .child(
        RichTextEditor::controlled(RichText::from_markdown(&held.get()), move |document| {
          written.set(document.to_markdown())
        })
        .id(ID)
        .controller(&handle)
        .keep_selection(true)
        .mount(ctx),
      )
      .into()
  });
  stage.pass();
  let selection = || controller.input().selection().ok().map(|s| (s.anchor, s.caret));
  assert!(controller.select(2, 6));
  stage.pass();
  assert_eq!(selection(), Some((Some(2), 6)));
  tick.set(1);
  stage.pass();
  tick.set(2);
  stage.pass();
  assert_eq!(
    selection(),
    Some((Some(2), 6)),
    "kept across renders of the same Markdown"
  );
}

#[test]
fn an_editor_not_mounted_keeps_its_selection_across_renders() {
  let value = Signal::new(notes());
  let controller = RichTextController::new();
  let tick = Signal::new(0);
  let (held, handle, ticks) = (value.clone(), controller.clone(), tick.clone());
  let mut stage = Stage::first_frame_in(App::new(), 640.0, 480.0, move |_| {
    ticks.get();
    Column::new()
      .size(640.0, 480.0)
      .child(
        RichTextEditor::new(held.clone())
          .id(ID)
          .controller(&handle)
          .keep_selection(true),
      )
      .into()
  });
  stage.pass();
  let selection = || controller.input().selection().ok().map(|s| (s.anchor, s.caret));
  assert!(controller.select(2, 6));
  stage.pass();
  assert_eq!(selection(), Some((Some(2), 6)));
  tick.set(1);
  stage.pass();
  assert_eq!(selection(), Some((Some(2), 6)), "kept across a render");
}

/// The words the editor's box has selected, as it was last drawn.
fn selected_words(stage: &mut Stage) -> Option<String> {
  stage
    .tree
    .get_element_by_id_mut(CONTENT)
    .and_then(|element| element.as_text_input())
    .and_then(|input| input.selected_text())
}

#[test]
fn a_selection_put_while_the_editors_parent_renders_lasts() {
  let value = Signal::new(RichText::plain("Hello world"));
  let controller = RichTextController::new();
  let pin = Signal::new(false);
  let (held, handle, pinning) = (value.clone(), controller.clone(), pin.clone());
  let mut stage = Stage::first_frame_in(App::new(), 640.0, 480.0, move |ctx| {
    let element = RichTextEditor::new(held.clone())
      .id(ID)
      .controller(&handle)
      .keep_selection(true)
      .mount(ctx);
    // After the editor is mounted in this render: an app that pins a selection to float a toolbar over it.
    if pinning.get() {
      handle.input().select(0, 5).expect("a selection");
    }
    Column::new().size(640.0, 480.0).child(element).into()
  });
  stage.pass();
  pin.set(true);
  for _ in 0..3 {
    stage.pass();
  }
  let now = controller.input().selection().expect("mounted");
  assert_eq!((now.anchor, now.caret), (Some(0), 5));
  assert_eq!(selected_words(&mut stage).as_deref(), Some("Hello"));
}

#[test]
fn a_keyed_editor_keeps_its_selection_and_its_undo_when_a_component_comes_before_it() {
  use lurq::components::{Markdown, MarkdownProps};
  let value = Signal::new(RichText::plain("Hello world"));
  let controller = RichTextController::new();
  let before = Signal::new(false);
  let (held, handle, mounts) = (value.clone(), controller.clone(), before.clone());
  let mut stage = Stage::first_frame_in(App::new(), 640.0, 480.0, move |ctx| {
    let mut column = Column::new().size(640.0, 480.0);
    if mounts.get() {
      column = column.child(Markdown::mount(ctx, MarkdownProps::new("A note")));
    }
    column
      .child(
        RichTextEditor::new(held.clone())
          .id(ID)
          .controller(&handle)
          .keep_selection(true)
          .key("editor")
          .mount(ctx),
      )
      .into()
  });
  stage.pass();
  assert!(controller.insert_text("Hi "));
  stage.pass();
  assert!(controller.select(0, 2));
  stage.pass();
  let words = value.get().text[..2].to_owned();
  assert_eq!(selected_words(&mut stage), Some(words.clone()));
  before.set(true);
  stage.pass();
  stage.pass();
  let now = controller.input().selection().expect("mounted");
  assert_eq!((now.anchor, now.caret), (Some(0), 2), "the same host, the same input");
  assert_eq!(selected_words(&mut stage), Some(words));
  assert!(controller.undo(), "its undo is kept");
  stage.pass();
  assert_eq!(value.get().text, "Hello world");
}

#[test]
fn a_block_of_code_in_a_language_has_a_bar_that_names_it_and_the_fence_makes_one() {
  use lurq::components::ParagraphKind;
  let mut editor = Editor::new(RichText::from_markdown("Intro\n\n```rust\nfn main() {}\n```"));
  assert_eq!(editor.kinds(), vec![Body, ParagraphKind::code(Some("rust"))]);
  // The block's line stands under the bar's 28, which has a line of `Border` under it.
  let (intro, code) = (
    editor.editor.input().caret_point(0).unwrap().y,
    editor.editor.input().caret_point(6).unwrap().y,
  );
  assert!(code - intro > 28.0 + 21.0, "{intro} {code}");
  let (_, content_y, ..) = editor.stage.bounds(CONTENT);
  let lines = editor
    .stage
    .quads()
    .iter()
    .filter(|quad| {
      matches!(quad.content, QuadContent::Rect { color, .. } if color == light(PaletteColor::Border))
        && (quad.height - 1.0).abs() < 0.01
        && quad.width > 300.0
        // Between the paragraph before and the block's line, the box's padding of 16 above the text.
        && quad.y > content_y + intro + 16.0
        && quad.y <= content_y + code + 16.0
    })
    .count();
  assert_eq!(lines, 1, "the line under the bar");
  // The style's control shows a block of code, whatever its language.
  editor.focus();
  editor.select(8, 8);
  assert!(editor.stage.paints("Code block"), "in a block of rust");
  // Three backticks and a name, and Enter: a block of code in it, written back with its fence.
  let mut editor = Editor::new(RichText::default());
  editor.focus();
  editor.type_text("```py");
  editor.stage.key("Enter");
  editor.type_text("x = 1");
  assert_eq!(editor.kinds(), vec![ParagraphKind::code(Some("py"))]);
  assert_eq!(editor.document().to_markdown(), "```py\nx = 1\n```");
}

#[test]
fn a_border_on_the_code_block_part_is_one_edge_round_the_block_and_its_bar() {
  use lurq::node::border::Border;
  let markdown = "Intro\n\n```bash\nls -la\ncd ..\n```\n\nOutro";
  let edged = |editor: &Editor| -> Vec<lurq::layout::quad::Quad> {
    editor
      .stage
      .quads()
      .into_iter()
      .filter(|quad| {
        matches!(quad.content, QuadContent::Rect { .. })
          && quad
            .border
            .as_ref()
            .and_then(|border| border.top)
            .is_some_and(|top| top.color == light(PaletteColor::Primary) && close(top.width, 1.0))
      })
      .collect()
  };
  let plain = Editor::new(RichText::from_markdown(markdown));
  assert!(edged(&plain).is_empty(), "no edge unless the part has a border");
  let card = Editor::with(640.0, RichText::from_markdown(markdown), |editor| {
    editor
      .style(RichTextEditorPart::CodeBlock, |s| {
        s.radius(8.0).border(Border::inside(1.0, PaletteColor::Primary))
      })
      .code_bar_label(|language| match language {
        "bash" => "Bash".to_string(),
        other => other.to_string(),
      })
  });
  let edges = edged(&card);
  assert_eq!(edges.len(), 1, "one edge round the whole block");
  let edge = &edges[0];
  let radius = edge.border_radius.expect("in the corners of the block");
  assert!(close(radius.top_left, 8.0), "{radius:?}");
  // The 28 of the bar and the two lines of 21 of the block.
  assert!(edge.height >= 28.0 + 2.0 * 21.0, "{}", edge.height);
  assert!(edge.width > 300.0, "across the box: {}", edge.width);
  // The document keeps the word of the fence.
  assert_eq!(card.document().to_markdown(), markdown);
}

const HINT: &str = "Type / for blocks";

/// The quads that say the block placeholder.
fn hints(editor: &Editor) -> Vec<lurq::layout::quad::Quad> {
  editor
    .stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == HINT))
    .collect()
}

#[test]
fn the_empty_block_the_caret_is_in_says_the_block_placeholder_where_its_text_would_stand() {
  let document = RichText::new("First\n\nThird", vec![], vec![Body, Body, Body]);
  let mut editor = Editor::with(640.0, document, |editor| editor.block_placeholder(HINT));
  assert!(hints(&editor).is_empty(), "none without the focus");
  let height = editor.stage.bounds(CONTENT).3;
  editor.focus();
  editor.select(6, 6);
  let hint = hints(&editor).pop().expect("the hint in the empty block");
  // Where the caret stands in the block: the box's padding is 18 in and 16 down.
  let input = editor.editor.input();
  let (x, y, ..) = editor.stage.bounds(CONTENT);
  let (first, at, third) = (
    input.caret_point(0).unwrap(),
    input.caret_point(6).unwrap(),
    input.caret_point(7).unwrap(),
  );
  assert!(
    first.y < at.y && at.y < third.y,
    "the empty block is the second: {} {} {}",
    first.y,
    at.y,
    third.y
  );
  // The caret's point is the band of the font's text, centered on the row the hint stands on.
  let middle = |top: f32, height: f32| top + height / 2.0;
  assert!(
    close(hint.x, x + 18.0 + at.x) && close(middle(hint.y, hint.height), y + 16.0 + middle(at.y, at.height)),
    "on the caret's line, where the block's text would start: {:?} and {at:?}",
    (hint.x - x - 18.0, hint.y - y - 16.0, hint.height)
  );
  assert_eq!(editor.stage.text(HINT), (light(PaletteColor::MutedForeground), 14.0));
  assert_eq!(editor.stage.bounds(CONTENT).3, height, "the box keeps its height");
  assert_eq!(editor.document().text, "First\n\nThird", "it is not in the text");

  // Not in a block that holds text, nor while something is selected.
  editor.select(2, 2);
  assert!(hints(&editor).is_empty(), "the caret in a block with text");
  editor.select(4, 8);
  assert!(hints(&editor).is_empty(), "a selection across the empty block");

  // What is typed takes its place, and it comes back when the block is empty again.
  editor.select(6, 6);
  editor.type_text("S");
  assert!(hints(&editor).is_empty());
  assert_eq!(editor.document().text, "First\nS\nThird");
  editor.stage.key("Backspace");
  assert_eq!(hints(&editor).len(), 1);
  // Leaving the editor takes it away.
  editor.stage.tree.get_element_by_id_mut("elsewhere").unwrap().focus();
  editor.stage.pass();
  assert!(hints(&editor).is_empty(), "gone with the focus");
}

#[test]
fn a_new_block_after_enter_says_it_in_its_own_type() {
  let mut editor = Editor::with(640.0, RichText::from_markdown("## Notes"), |editor| {
    editor.block_placeholder(HINT)
  });
  editor.focus();
  assert!(hints(&editor).is_empty(), "the caret after the heading's text");
  editor.stage.key("Enter");
  let hint = hints(&editor).pop().expect("the hint in the new block");
  // The heading's row is 26 high, and 10 stand between two blocks: a body's row of 18 under it.
  let (_, y, ..) = editor.stage.bounds(CONTENT);
  assert!(
    close(hint.y, y + 16.0 + 26.0 + 10.0) && close(hint.height, 18.0),
    "on the new block's row, under the heading: {:?}",
    (hint.y - y - 16.0, hint.height)
  );
  let kind = editor.kinds()[1];
  let size = editor.stage.text(HINT).1;
  assert!(
    matches!(kind, Body) && size < 22.0,
    "a body's type after a heading: {kind:?}, {size}"
  );

  // An empty heading says it in the heading's type.
  let document = RichText::new("Intro\n", vec![], vec![Body, Heading(2)]);
  let mut editor = Editor::with(640.0, document, |editor| editor.block_placeholder(HINT));
  editor.focus();
  editor.select(6, 6);
  assert_eq!(editor.stage.text(HINT).1, 22.0);
}

#[test]
fn every_empty_block_says_it_where_told_and_an_empty_document_says_its_own_placeholder() {
  let document = RichText::new("A\n\nB\n\nC", vec![], vec![Body; 5]);
  let editor = Editor::with(640.0, document, |editor| {
    editor
      .block_placeholder(HINT)
      .block_placeholder_in_every_empty_block(true)
  });
  assert_eq!(hints(&editor).len(), 2, "both empty blocks, without the focus");

  // An empty document: its own placeholder, focused or not.
  let mut editor = Editor::with(640.0, RichText::new("", vec![], vec![Body]), |editor| {
    editor.placeholder("Write the release notes").block_placeholder(HINT)
  });
  editor.focus();
  assert!(editor.stage.paints("Write the release notes"));
  assert!(hints(&editor).is_empty(), "the document's placeholder in place of it");

  // With none of its own, the empty document is an empty block the caret is in.
  let mut editor = Editor::with(640.0, RichText::new("", vec![], vec![Body]), |editor| {
    editor.block_placeholder(HINT)
  });
  assert!(hints(&editor).is_empty());
  editor.focus();
  assert_eq!(hints(&editor).len(), 1);
}
