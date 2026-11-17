//! The blocks of the design's document: GitHub's admonitions, `<details>`
//! and `<kbd>` as the parser reads them; numbered lists, bullets and tasks,
//! the table's header and stripes, a note on the callout's look, a details
//! block that opens and closes, the footnotes at the end and the way to and
//! from them, code and keys on their boxes, and the colors of the theme's
//! roles in light and dark.

use std::sync::{
  Arc, Mutex,
  atomic::{AtomicBool, Ordering},
};

use lurq::{
  app::{App, Tree, events::MouseButton, theme::PaletteColor, window::Appearance},
  components::MarkdownProps,
  layout::quad::QuadContent,
  markdown::{MarkdownAdmonitionKind, MarkdownBlock, MarkdownInline, parse_markdown},
};

use super::links::{bounds_of, has_element_with_text, mount, pass, quads};
use crate::support::{TestSurface, pointer_click};

fn palette(app: &App, role: PaletteColor) -> lurq::node::color::Color {
  app.theme().palette().get(role)
}

/// The fills of `color` drawn, as `(x, y, width, height)`.
fn fills(tree: &Tree, color: lurq::node::color::Color) -> Vec<(f32, f32, f32, f32)> {
  quads(tree)
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color))
    .map(|quad| (quad.x, quad.y, quad.width, quad.height))
    .collect()
}

/// The color the words `text` are drawn in, among the document's runs.
fn color_of(tree: &Tree, text: &str) -> Option<lurq::node::color::Color> {
  quads(tree).into_iter().find_map(|quad| match quad.content {
    QuadContent::RichText { spans, .. } => spans.iter().find(|span| span.text == text).map(|span| span.style.color),
    QuadContent::Text { text: words, style, .. } if words == text => Some(style.color),
    _ => None,
  })
}

fn click(app: &mut App, tree: &mut Tree, text: &str) {
  let rect = bounds_of(tree, text);
  pointer_click(
    tree,
    rect.x + rect.width / 2.0,
    rect.y + rect.height / 2.0,
    MouseButton::Left,
  );
  pass(app, tree);
  pass(app, tree);
}

#[test]
fn the_parser_reads_admonitions_details_and_keys() {
  let document = parse_markdown(
    "> [!TIP]\n> Use it.\n\n<details open>\n<summary>Platform notes</summary>\n\nTwo passes.\n\n</details>\n\nPress \
     <kbd>Ctrl</kbd>.\n",
  );
  assert!(matches!(
    &document.blocks[0],
    MarkdownBlock::Admonition { kind: MarkdownAdmonitionKind::Tip, blocks } if blocks.len() == 1
  ));
  match &document.blocks[1] {
    MarkdownBlock::Details { summary, open, blocks } => {
      assert_eq!(summary, "Platform notes");
      assert!(*open);
      assert!(matches!(&blocks[0], MarkdownBlock::Paragraph(_)));
    }
    other => panic!("a details block: {other:?}"),
  }
  match &document.blocks[2] {
    MarkdownBlock::Paragraph(inlines) => assert!(inlines.contains(&MarkdownInline::Kbd("Ctrl".to_owned()))),
    other => panic!("a paragraph: {other:?}"),
  }
  // A quote with no kind is a quote.
  assert!(matches!(
    &parse_markdown("> Just a quote.").blocks[0],
    MarkdownBlock::BlockQuote(_)
  ));
}

#[test]
fn a_list_is_numbered_at_the_end_of_its_column_or_bulleted_and_copies_its_marker() {
  let (_app, mut tree) = mount(MarkdownProps::new("1. one\n2. two\n\n- dot\n  - ring\n").width(400.0));
  let one = bounds_of(&mut tree, "1.");
  let words = bounds_of(&mut tree, "one");
  assert_eq!(one.width, 18.0, "a number in a column of 18");
  assert_eq!(words.x - (one.x + one.width), 8.0, "8 before the item");
  // A bullet is a dot, and draws its `-` in no color: a selection copies it.
  assert!(has_element_with_text(&mut tree, "-"));
}

#[test]
fn a_task_is_a_checkbox_read_only_unless_the_app_takes_its_change() {
  let source = "- [x] done\n- [ ] open\n";
  let (mut app, mut tree) = mount(MarkdownProps::new(source).width(400.0));
  // The checked box is on the primary fill, the other is not.
  let primary = fills(&tree, palette(&app, PaletteColor::Primary));
  assert_eq!(primary.len(), 1, "one box checked: {primary:?}");
  assert!(
    !has_element_with_text(&mut tree, "[x] done"),
    "the box stands for the marker"
  );
  // A task that is done is in the muted foreground.
  assert_eq!(
    color_of(&tree, "done"),
    Some(palette(&app, PaletteColor::MutedForeground))
  );
  // Read only: a press on the open task's box checks nothing.
  let open = bounds_of(&mut tree, "open");
  pointer_click(&mut tree, open.x - 18.0, open.y + open.height / 2.0, MouseButton::Left);
  pass(&mut app, &mut tree);
  assert_eq!(fills(&tree, palette(&app, PaletteColor::Primary)).len(), 1);

  let told = Arc::new(Mutex::new(Vec::new()));
  let heard = told.clone();
  let (mut app2, mut tree2) = mount(
    MarkdownProps::new(source)
      .width(400.0)
      .on_task_change(move |index, checked| heard.lock().unwrap().push((index, checked))),
  );
  // The box is 10 before the task's words, 16 square.
  let open = bounds_of(&mut tree2, "open");
  pointer_click(&mut tree2, open.x - 18.0, open.y + open.height / 2.0, MouseButton::Left);
  pass(&mut app2, &mut tree2);
  assert_eq!(told.lock().unwrap().as_slice(), [(1, true)]);
}

#[test]
fn a_tables_header_is_on_the_muted_fill_and_every_other_row_is_striped() {
  let (app, tree) =
    mount(MarkdownProps::new("| a | b |\n| --- | ---: |\n| 1 | 2 |\n| 3 | 4 |\n| 5 | 6 |\n").width(400.0));
  let muted = fills(&tree, palette(&app, PaletteColor::Muted));
  // The header's two cells, and the second body row, as wide as the table.
  assert_eq!(muted.len(), 3, "{muted:?}");
  assert_eq!((muted[0].1, muted[1].1), (0.0, 0.0), "the header's cells");
  assert_eq!(
    muted[0].2 + muted[1].2,
    muted[2].2,
    "a striped row as wide as the header"
  );
  assert!(muted[2].1 > muted[0].1);
}

#[test]
fn an_admonition_is_a_callout_with_its_title_in_its_tone() {
  let (app, mut tree) = mount(MarkdownProps::new("> [!WARNING]\n> Large radii cost more.\n").width(400.0));
  assert!(has_element_with_text(&mut tree, "Warning"));
  assert_eq!(color_of(&tree, "Warning"), Some(palette(&app, PaletteColor::Warning)));
  assert!(
    !fills(&tree, palette(&app, PaletteColor::WarningMuted)).is_empty(),
    "on the muted warning fill"
  );
}

#[test]
fn details_open_and_close_by_their_summary() {
  let (mut app, mut tree) = mount(
    MarkdownProps::new("<details>\n<summary>Platform notes</summary>\n\nTwo passes.\n\n</details>\n").width(400.0),
  );
  assert!(has_element_with_text(&mut tree, "Platform notes"));
  assert!(!has_element_with_text(&mut tree, "Two passes."), "closed to begin with");
  click(&mut app, &mut tree, "Platform notes");
  assert!(has_element_with_text(&mut tree, "Two passes."), "opened");
  click(&mut app, &mut tree, "Platform notes");
  assert!(!has_element_with_text(&mut tree, "Two passes."), "closed again");
}

#[test]
fn footnotes_stand_at_the_end_by_number_with_the_way_to_and_from_them() {
  let source = "First[^b] and second[^a].\n\n[^a]: The a note.\n\n[^b]: The b note.\n\nA last paragraph.\n";
  let (app, mut tree) = mount(MarkdownProps::new(source).width(400.0));
  // Numbered by the first reference: b is 1, a is 2.
  assert_eq!(color_of(&tree, "1"), Some(palette(&app, PaletteColor::Info)));
  let one = bounds_of(&mut tree, "1.");
  let two = bounds_of(&mut tree, "2.");
  let last = bounds_of(&mut tree, "A last paragraph.");
  assert!(one.y > last.y && two.y > one.y, "at the end, in their order");
  assert!(has_element_with_text(&mut tree, "The b note."));
  assert_eq!(
    color_of(&tree, "The b note."),
    Some(palette(&app, PaletteColor::MutedForeground))
  );
}

#[test]
fn code_and_keys_in_a_sentence_are_on_their_boxes() {
  let (app, tree) = mount(MarkdownProps::new("Call `ctx.theme()` and press <kbd>Ctrl</kbd>.").width(500.0));
  let muted = palette(&app, PaletteColor::Muted);
  let boxes = fills(&tree, muted);
  assert_eq!(boxes.len(), 2, "the code's and the key's: {boxes:?}");
  // The key's box has the key's edge.
  let edged = quads(&tree)
    .into_iter()
    .filter(|quad| {
      quad
        .border
        .and_then(|border| border.top)
        .is_some_and(|side| side.color == palette(&app, PaletteColor::Border))
    })
    .count();
  assert!(edged >= 1);
}

#[test]
fn the_document_follows_the_theme_light_and_dark() {
  let mut app = App::new();
  app.theme().set_preferred_appearance(Some(Appearance::Dark));
  let mut tree = Tree::new();
  tree
    .mount_root::<super::links::MarkdownRoot>(&mut app, MarkdownProps::new("A [link](https://lurq.dev).").width(400.0));
  tree.pass(&mut app, &TestSurface);
  let dark = palette(&app, PaletteColor::Info);
  assert_eq!(color_of(&tree, "link"), Some(dark), "the dark palette's info");
  app.theme().set_preferred_appearance(Some(Appearance::Light));
  pass(&mut app, &mut tree);
  pass(&mut app, &mut tree);
  let light = palette(&app, PaletteColor::Info);
  assert_ne!(light, dark);
  assert_eq!(color_of(&tree, "link"), Some(light), "and the light one's");
  let _ = AtomicBool::new(false).load(Ordering::SeqCst);
}

thread_local! {
  static SCROLLED: std::cell::RefCell<Option<lurq::layout::layout_kind::ScrollState>> = const { std::cell::RefCell::new(None) };
}

/// A long document in a view 200 high, its footnotes far below its first line.
struct Scrolled;

impl lurq::app::component::Component for Scrolled {
  type Props = ();

  fn create(_: &mut lurq::app::ctx::Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut lurq::app::ctx::Ctx) -> impl Into<lurq::node::Element> {
    let state = SCROLLED.with(|slot| slot.borrow().clone().expect("installed"));
    let filler: String = (0..30).map(|line| format!("Paragraph {line}.\n\n")).collect();
    let source = format!("See the note[^n].\n\n{filler}[^n]: The note at the end.\n");
    // Narrower than the view: the way back is clear of the view's bar.
    lurq::components::ScrollVertical::new(lurq::components::Markdown::mount(
      ctx,
      MarkdownProps::new(source).width(360.0),
    ))
    .with_scroll_state(state)
    .size(400.0, 200.0)
  }
}

#[test]
fn a_reference_goes_to_its_footnote_and_the_way_back_returns() {
  let state = lurq::layout::layout_kind::ScrollState::new();
  SCROLLED.with(|slot| *slot.borrow_mut() = Some(state.clone()));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<Scrolled>(&mut app, ());
  tree.pass(&mut app, &TestSurface);
  assert_eq!(state.scroll_y(), 0.0);
  click(&mut app, &mut tree, "1");
  let down = state.scroll_y();
  assert!(down > 500.0, "at the footnote: {down}");
  // The way back: the arrow after the footnote's words.
  let note = bounds_of(&mut tree, "The note at the end.");
  pointer_click(
    &mut tree,
    note.x + note.width + 12.0,
    note.y + note.height / 2.0,
    MouseButton::Left,
  );
  pass(&mut app, &mut tree);
  pass(&mut app, &mut tree);
  assert!(state.scroll_y() < 50.0, "back at the reference: {}", state.scroll_y());
}

/// The style the words `text` are drawn in.
fn style_of(tree: &Tree, text: &str) -> Option<lurq::layout::text_style::TextStyle> {
  quads(tree).into_iter().find_map(|quad| match quad.content {
    QuadContent::RichText { spans, .. } => spans
      .iter()
      .find(|span| span.text == text)
      .map(|span| span.style.clone()),
    QuadContent::Text { text: words, style, .. } if words == text => Some(style),
    _ => None,
  })
}

#[test]
fn a_quote_and_a_code_block_are_the_systems() {
  let (app, mut tree) = mount(MarkdownProps::new("> Said once.\n\n```rust\nfn main() {}\n```\n").width(400.0));
  // A quote is `ui::Blockquote`: its words 14 in italics in the muted foreground, after a line of 2.
  let said = style_of(&tree, "Said once.").expect("the quote's words are drawn");
  assert_eq!(said.font_size, 14.0);
  assert_eq!(said.style, lurq::layout::text_style::FontStyle::Italic);
  assert_eq!(said.color, palette(&app, PaletteColor::MutedForeground));
  // A code block is `ui::CodeBlock`: its bar on the muted fill, named by its language, a keyword in the info color.
  assert!(has_element_with_text(&mut tree, "rust"), "the language names the block");
  let bar = fills(&tree, palette(&app, PaletteColor::Muted));
  assert!(bar.iter().any(|fill| fill.3 == 36.0), "a bar of 36: {bar:?}");
  assert_eq!(color_of(&tree, "fn"), Some(palette(&app, PaletteColor::Info)));
}

#[test]
fn a_fence_is_colored_by_its_languages_highlighter_and_a_new_one_draws_it_again() {
  // lurq's own for SQL and YAML, and a language of the app's, which has none yet.
  let source = "```sql\nSELECT 1\n```\n\n```yaml\nname: x\n```\n\n```lang-of-the-markdown-tests\nalpha beta\n```\n";
  let (mut app, mut tree) = mount(MarkdownProps::new(source).width(400.0));
  assert_eq!(color_of(&tree, "SELECT"), Some(palette(&app, PaletteColor::Info)));
  assert_eq!(color_of(&tree, "1"), Some(palette(&app, PaletteColor::Warning)));
  assert_eq!(
    color_of(&tree, "name"),
    Some(palette(&app, PaletteColor::Info)),
    "a key"
  );
  assert_eq!(
    color_of(&tree, "alpha beta"),
    Some(palette(&app, PaletteColor::Foreground)),
    "plain"
  );
  // A highlighter registered for it: the block, mounted and kept while its code is the same, is drawn again.
  app.register_highlighter("lang-of-the-markdown-tests", |_: &str| {
    vec![lurq::code::CodeToken::new(0..5, lurq::code::token::STRING)]
  });
  for _ in 0..2 {
    pass(&mut app, &mut tree);
  }
  assert_eq!(color_of(&tree, "alpha"), Some(palette(&app, PaletteColor::Success)));
}

#[test]
fn a_selection_across_the_systems_parts_copies_words_and_no_glyphs() {
  let source = "First.\n\n- [x] done\n\n> [!NOTE]\n> Noted.\n\n```rust\nfn main() {}\n```\n\nLast.\n";
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(440, 1200);
  tree.mount_root::<super::links::MarkdownRoot>(&mut app, MarkdownProps::new(source).selectable(true).width(400.0));
  for _ in 0..3 {
    pass(&mut app, &mut tree);
  }
  let first = bounds_of(&mut tree, "First.");
  let last = bounds_of(&mut tree, "Last.");
  let mut page = crate::support::selection::Page::without_inks(app, tree, 1.0);
  page.drag(
    (first.x + 1.0, first.y + first.height / 2.0),
    (last.x + last.width - 1.0, last.y + last.height / 2.0),
  );
  let copied = page.selected().expect("a selection across the document");
  assert!(
    copied.contains("done") && copied.contains("Noted.") && copied.contains("fn main() {}"),
    "{copied:?}"
  );
  assert!(
    !copied.chars().any(|ch| ('\u{e000}'..='\u{f8ff}').contains(&ch)),
    "no glyph of a checkbox, a callout or a copy button: {copied:?}"
  );
}

/// A document that fits its content, at the start of a row: as wide as its line.
struct Hugged;

impl lurq::app::component::Component for Hugged {
  type Props = MarkdownProps;

  fn create(_ctx: &mut lurq::app::ctx::Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut lurq::app::ctx::Ctx) -> impl Into<lurq::node::Element> {
    let props = ctx.props::<Self::Props>().clone().fit_content(true);
    lurq::components::Row::new()
      .align_items(lurq::layout::Alignment::Start)
      .child(lurq::components::Markdown::mount(ctx, props))
  }
}

/// Where the paragraph of a document that fits its content is drawn: its start and its width.
fn line_of(source: &str) -> (Tree, f32, f32) {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<Hugged>(&mut app, MarkdownProps::new(source));
  tree.pass(&mut app, &TestSurface);
  let quad = quads(&tree)
    .into_iter()
    .find(|quad| matches!(&quad.content, QuadContent::RichText { .. } | QuadContent::Text { .. }))
    .expect("the paragraph is drawn");
  let (x, width) = (quad.x, quad.width);
  (tree, x, width)
}

/// The width of `source` as a line of the document's text.
fn width_of(source: &str) -> f32 {
  line_of(source).2
}

/// The boxes of keys and code in `tree`, from the start of the line.
fn boxes(tree: &Tree) -> Vec<(f32, f32, f32, f32)> {
  let mut boxes = fills(tree, App::new().theme().palette().get(PaletteColor::Muted));
  boxes.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.total_cmp(&b.0)));
  boxes
}

fn near(a: f32, b: f32, what: &str) {
  assert!((a - b).abs() <= 0.75, "{what}: {a} where {b}");
}

#[test]
fn keys_stand_between_the_spaces_of_their_sentence_the_designs_gap_apart() {
  // The design's Keyboard run: Press, Ctrl, K and the rest 6 apart.
  let gap = 6.0;
  let (tree, x, width) = line_of("Press <kbd>Ctrl</kbd> <kbd>K</kbd> to search the docs.");
  let keys = boxes(&tree);
  assert_eq!(keys.len(), 2, "{keys:?}");
  let (ctrl, k) = (keys[0], keys[1]);
  near(ctrl.0, x + width_of("Press") + gap, "Ctrl 6 after Press");
  near(k.0, ctrl.0 + ctrl.2 + gap, "K 6 after Ctrl");
  near(
    x + width,
    k.0 + k.2 + gap + width_of("to search the docs."),
    "the rest 6 after K",
  );
  // The spaces are the sentence's own: it copies as written.
  assert_eq!(
    MarkdownProps::new("Press <kbd>Ctrl</kbd> <kbd>K</kbd> to search the docs.").rendered_text(),
    "Press Ctrl K to search the docs."
  );
}

#[test]
fn a_key_at_the_start_or_the_end_of_a_paragraph_is_inside_its_line() {
  let (tree, x, _) = line_of("<kbd>Esc</kbd> closes it.");
  near(
    boxes(&tree)[0].0,
    x,
    "a key that starts the line, its box at the line's start",
  );
  let (tree, x, width) = line_of("Close it with <kbd>Esc</kbd>");
  let key = boxes(&tree)[0];
  near(
    key.0 + key.2,
    x + width,
    "a key that ends the line, its box at the line's end",
  );
}

#[test]
fn two_keys_with_a_plus_between_them_or_nothing() {
  let plus = width_of("a+a") - width_of("aa");
  let (tree, ..) = line_of("<kbd>Ctrl</kbd>+<kbd>K</kbd>");
  let keys = boxes(&tree);
  near(keys[1].0 - (keys[0].0 + keys[0].2), plus, "the plus between the boxes");
  let (tree, ..) = line_of("<kbd>Ctrl</kbd><kbd>K</kbd>");
  let keys = boxes(&tree);
  near(
    keys[1].0,
    keys[0].0 + keys[0].2,
    "with nothing between them, the boxes meet and do not overlap",
  );
}

#[test]
fn code_beside_a_comma_ends_where_the_comma_starts() {
  let (tree, x, width) = line_of("Call `ctx.theme()`, then render.");
  let code = boxes(&tree)[0];
  near(
    x + width,
    code.0 + code.2 + width_of(", then render."),
    "the comma right after the box",
  );
  near(
    code.0,
    x + width_of("Call") + 6.0,
    "the design's gap before the box, where a space stands",
  );
}

#[test]
fn a_key_goes_to_the_next_line_whole_and_a_line_starts_with_no_space() {
  let (_app, tree) = mount(MarkdownProps::new("<kbd>Ctrl</kbd> <kbd>K</kbd>").width(50.0));
  let keys = boxes(&tree);
  assert_eq!(keys.len(), 2, "{keys:?}");
  assert!(keys[1].1 > keys[0].1, "K on the next line: {keys:?}");
  near(keys[1].0, keys[0].0, "at the line's start, as Ctrl is");
}

#[test]
fn what_a_key_copies_is_its_words_and_the_spaces_around_it() {
  let props = MarkdownProps::new("Press <kbd>Ctrl</kbd> <kbd>K</kbd> to search, or `/`.");
  assert_eq!(props.rendered_text(), "Press Ctrl K to search, or /.");
}

/// The size the words `text` are drawn at, among the document's runs.
fn size_of(tree: &Tree, text: &str) -> Option<f32> {
  style_of(tree, text).map(|style| style.font_size)
}

#[test]
fn keys_and_code_keep_their_size_and_the_words_after_them_theirs() {
  let (_, tree) = mount(MarkdownProps::new("<kbd>Esc</kbd> closes it, `Tab` moves on.").width(500.0));
  assert_eq!(size_of(&tree, "Esc"), Some(11.0), "a key at 11");
  assert_eq!(size_of(&tree, "Tab"), Some(13.0), "code at 13");
  assert_eq!(
    size_of(&tree, " closes it, "),
    Some(15.0),
    "the words at the document's 15"
  );
  // Drawn at their sizes too: the words after a key that starts the paragraph are as wide as they are alone.
  let alone = width_of("closes it");
  let (tree, x, width) = line_of("<kbd>Esc</kbd> closes it");
  let key = boxes(&tree)[0];
  near(
    x + width,
    key.0 + key.2 + 6.0 + alone,
    "the words after the key at their own size",
  );
}

#[test]
fn a_footnotes_number_is_raised_above_the_line() {
  let (_, tree) = mount(MarkdownProps::new("Frame time stayed under 7 ms[^1].\n\n[^1]: Measured.").width(500.0));
  let reference = style_of(&tree, "1").expect("the reference is drawn");
  assert_eq!(reference.font_size, 11.0);
  // Its top at the line's top, as the design draws it: 0.45 of the text's 15 above the baseline.
  assert!(
    (reference.baseline_shift - 6.75).abs() < 0.01,
    "{}",
    reference.baseline_shift
  );
  assert_eq!(
    style_of(&tree, "Frame time stayed under 7 ms").map(|style| style.baseline_shift),
    Some(0.0)
  );
}
