//! `ui::CodeBlock`: the design's Code Block (`kyzrV`), its colors, its lines, its copy button and its scrolling.

use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App,
    events::ScrollPhase,
    theme::{CodeScheme, PaletteColor, ThemePalette},
  },
  code::{CodeToken, CodeTokenizer, token},
  components::Column,
  core::Signal,
  layout::{
    quad::{Quad, QuadContent, RichTextSpan},
    text_style::{FontStyle, FontWeight},
  },
  node::{Element, color::Color},
  ui::{CodeBlock, InlineCode, Length, ScrollArea},
};

use super::stage::Stage;

const MAIN: &str = "fn main() {\n  let app = App::new();\n  app.theme().set_mode(Mode::Dark);\n}";

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// A stage in the light palette, whatever the system's appearance.
fn staged(scene: impl Fn() -> CodeBlock + Send + Sync + 'static) -> Stage {
  let app = App::new();
  app
    .theme()
    .set_preferred_appearance(Some(lurq::app::window::Appearance::Light));
  Stage::in_app(app, move |_| Column::new().width(420.0).child(scene()).into())
}

/// A stage in the light palette of an element built on it.
fn staged_element(scene: impl Fn() -> Element + Send + Sync + 'static) -> Stage {
  let app = App::new();
  app
    .theme()
    .set_preferred_appearance(Some(lurq::app::window::Appearance::Light));
  Stage::in_app(app, move |_| Column::new().width(420.0).child(scene()).into())
}

/// The runs of every text drawn in runs, in the order they were drawn.
fn runs(stage: &Stage) -> Vec<Vec<RichTextSpan>> {
  stage
    .quads()
    .into_iter()
    .filter_map(|quad| match quad.content {
      QuadContent::RichText { spans, .. } => Some(spans),
      _ => None,
    })
    .collect()
}

/// The run that says `text`.
fn run(stage: &Stage, text: &str) -> RichTextSpan {
  runs(stage)
    .into_iter()
    .flatten()
    .find(|span| span.text == text)
    .unwrap_or_else(|| panic!("no run {text:?}"))
}

fn fills(stage: &Stage, color: Color) -> Vec<Quad> {
  stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color))
    .collect()
}

#[test]
fn the_block_is_the_boards() {
  let mut stage = staged(|| CodeBlock::new(MAIN).language("rust").title("main.rs").id("block"));
  let (x, y, width, height) = stage.bounds("block");
  assert_eq!((x, y, width), (0.0, 0.0, 420.0), "it fills its room");
  // A bar of 36 and four lines of 17, 4 apart, padded 14: 144, as the board's.
  assert!((height - 144.0).abs() < 0.6, "{height}");
  let block = stage.fill("block").expect("the block's fill");
  assert!(matches!(block.content, QuadContent::Rect { color, .. } if color == light(PaletteColor::Background)));
  assert_eq!(block.border_radius.expect("round").top_left, 8.0, "the large radius");
  assert!(stage.border("block").is_some(), "an edge");
  // The bar: 36 high on the bar's fill.
  let bars = fills(&stage, light(PaletteColor::Bar));
  assert!(
    bars.iter().any(|bar| bar.height == 36.0 && bar.width >= 418.0),
    "{:?}",
    bars.iter().map(|bar| (bar.width, bar.height)).collect::<Vec<_>>()
  );
  // The title in 11 of the family of code, in MutedForeground, 14 in.
  let (color, size) = stage.text("main.rs");
  assert_eq!((color, size), (light(PaletteColor::MutedForeground), 11.0));
  let (title_x, ..) = stage.bounds("block-title");
  assert_eq!(title_x, 14.0);
  // The copy button: 28 square, 6 from the end, in the bar's middle.
  let (copy_x, copy_y, copy_width, copy_height) = stage.bounds("block-copy");
  assert_eq!((copy_width, copy_height), (28.0, 28.0));
  assert_eq!((copy_x + copy_width, copy_y), (414.0, 4.0));
}

#[test]
fn the_words_of_rust_are_in_the_colors_of_their_roles() {
  // The code as the text a Markdown document selects with its own words, whose runs are read here.
  let stage = staged(|| {
    CodeBlock::new("fn main() {\n  let n = App::new(42); // the answer\n  println!(\"{n}\");\n}")
      .language("rust")
      .selectable(false)
  });
  assert_eq!(
    run(&stage, "fn").style.color,
    light(PaletteColor::Info),
    "the board's keywords"
  );
  assert_eq!(run(&stage, "App").style.color, light(PaletteColor::Info), "a type");
  assert_eq!(run(&stage, "42").style.color, light(PaletteColor::Warning));
  assert_eq!(run(&stage, "\"{n}\"").style.color, light(PaletteColor::Success));
  assert_eq!(
    run(&stage, "// the answer").style.color,
    light(PaletteColor::MutedForeground)
  );
  assert_eq!(
    run(&stage, "()").style.color,
    light(PaletteColor::MutedForeground),
    "marks are quiet"
  );
  let code = run(&stage, " main");
  assert_eq!(code.style.color, light(PaletteColor::Foreground));
  assert_eq!(code.style.font_size, 13.0);
  // A language with no tokenizer is one run in the code's color.
  let stage = staged(|| {
    CodeBlock::new("IDENTIFICATION DIVISION.\nPROGRAM-ID. NOTES.")
      .language("cobol")
      .selectable(false)
  });
  assert_eq!(runs(&stage).iter().flatten().count(), 1);
}

#[test]
fn a_theme_colors_a_kind_of_word_otherwise() {
  let stage = staged(|| {
    CodeBlock::new("let x = 1;")
      .language("rust")
      .selectable(false)
      .token_style("keyword", |s| s.foreground(PaletteColor::Destructive))
  });
  assert_eq!(run(&stage, "let").style.color, light(PaletteColor::Destructive));
}

/// The numbers painted beside the code, top to bottom: their text, color and box.
fn numbers(stage: &Stage) -> Vec<(String, Color, Quad)> {
  let mut numbers: Vec<(String, Color, Quad)> = stage
    .quads()
    .into_iter()
    .filter_map(|quad| {
      let (text, color) = match &quad.content {
        QuadContent::Text { text, style, .. } if text.parse::<usize>().is_ok() => (text.clone(), style.color),
        _ => return None,
      };
      Some((text, color, quad))
    })
    .collect();
  numbers.sort_by(|a, b| a.2.y.total_cmp(&b.2.y));
  numbers
}

/// The bands behind picked lines: fills of Highlight less high than the bar, which is on Muted, as the light
/// palette's Highlight is.
fn bands(stage: &Stage) -> Vec<Quad> {
  fills(stage, light(PaletteColor::Highlight))
    .into_iter()
    .filter(|quad| quad.height != 36.0)
    .collect()
}

#[test]
fn numbered_lines_stand_beside_the_code_and_a_picked_line_has_its_band() {
  let mut stage = staged(|| {
    CodeBlock::new(MAIN)
      .language("rust")
      .line_numbers(true)
      .first_line(9)
      .highlight_lines([10])
      .id("block")
  });
  let shown = numbers(&stage);
  let texts: Vec<&str> = shown.iter().map(|(text, ..)| text.as_str()).collect();
  assert_eq!(texts, ["9", "10", "11", "12"]);
  assert_eq!(shown[0].1, light(PaletteColor::MutedForeground));
  assert_eq!(shown[1].1, light(PaletteColor::Foreground), "a picked line's number");
  // Each number on its line, a row of 21 apart, before the code.
  let (code_x, ..) = stage.bounds("block-code");
  assert!((shown[1].2.y - shown[0].2.y - 21.0).abs() < 0.01);
  assert!(
    shown[0].2.x + shown[0].2.width <= code_x + 14.0 + 24.0,
    "before the code"
  );
  // The band of line 10, the second: a row of 21 under the bar, the padding and the first line, across the block.
  let (_, top, ..) = stage.bounds("block");
  let bands = bands(&stage);
  assert_eq!(bands.len(), 1);
  assert!((bands[0].y - (top + 36.0 + 12.0 + 21.0)).abs() < 0.6, "{}", bands[0].y);
  assert!((bands[0].height - 21.0).abs() < 0.01);
  assert!(bands[0].width >= 418.0, "across the block: {}", bands[0].width);
}

#[test]
fn the_text_a_document_selects_numbers_its_lines_and_bands_them_too() {
  let stage = staged(|| {
    CodeBlock::new(MAIN)
      .language("rust")
      .line_numbers(true)
      .highlight_lines([2])
      .selectable(false)
  });
  let numbers: Vec<RichTextSpan> = runs(&stage)
    .into_iter()
    .find(|spans| spans.first().is_some_and(|span| span.text.starts_with('1')))
    .expect("the numbers");
  let texts: Vec<&str> = numbers.iter().map(|span| span.text.as_str()).collect();
  assert_eq!(texts, ["1\n", "2\n", "3\n", "4"]);
  assert_eq!(numbers[1].style.color, light(PaletteColor::Foreground));
}

#[test]
fn a_long_line_scrolls_sideways_or_wraps() {
  let long = "let a_very_long_name_that_runs_past_the_edge_of_the_block = another_long_name_on_the_same_line;";
  // No title, no language and no copy button: no bar.
  let mut stage = staged(move || CodeBlock::new(long).copy_button(false).id("block"));
  let (_, _, _, one) = stage.bounds("block");
  assert!(
    (one - (21.0 + 24.0)).abs() < 1.0,
    "one line padded 12 above and below: {one}"
  );
  let mut stage = staged(move || CodeBlock::new(long).copy_button(false).wrap(true).id("block"));
  let (_, _, width, wrapped) = stage.bounds("block");
  assert_eq!(width, 420.0);
  assert!(
    wrapped > one + 15.0,
    "the line wraps onto another: {wrapped} after {one}"
  );
}

#[test]
fn a_least_and_a_greatest_height() {
  let tall = (1..=40)
    .map(|line| format!("line {line}"))
    .collect::<Vec<_>>()
    .join("\n");
  let mut stage = staged(move || CodeBlock::new(&tall).max_height(200.0).id("block"));
  let (.., height) = stage.bounds("block");
  assert!((height - 200.0).abs() < 1.0, "{height}");
  let mut stage = staged(|| CodeBlock::new("ls").min_height(160.0).id("block"));
  let (.., height) = stage.bounds("block");
  assert!((height - 160.0).abs() < 1.0, "{height}");
}

#[test]
fn a_block_with_nothing_to_name_and_no_copy_button_has_no_bar() {
  let mut stage = staged(|| CodeBlock::new("ls -la").copy_button(false).id("block"));
  let (.., height) = stage.bounds("block");
  assert!(height < 50.0, "{height}");
  assert!(
    fills(&stage, light(PaletteColor::Bar))
      .iter()
      .all(|fill| fill.height != 36.0),
    "no bar of 36"
  );
}

#[test]
fn a_long_title_ends_in_an_ellipsis_and_the_copy_button_keeps_its_room() {
  let mut stage = staged(|| {
    CodeBlock::new("ls")
      .title("crates/lurq/src/ui/a/very/deep/folder/of/the/project/that/goes/on/and/on/code_block.rs")
      .id("block")
  });
  let (copy_x, _, copy_width, _) = stage.bounds("block-copy");
  assert_eq!((copy_x + copy_width, copy_width), (414.0, 28.0));
  let (title_x, _, title_width, _) = stage.bounds("block-title");
  assert!(title_x + title_width <= copy_x, "the title stops before the button");
}

/// A clipboard of the test's own.
#[derive(Default)]
struct Written(Mutex<Vec<String>>);

impl lurq::app::ClipboardWriter for Written {
  fn write_text(&self, text: &str) -> bool {
    self.0.lock().unwrap().push(text.to_owned());
    true
  }
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn the_copy_button_copies_the_code_and_shows_it_did_for_a_while() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let heard = told.clone();
  let app = App::new();
  let mut stage = Stage::in_app(app, move |ctx| {
    let heard = heard.clone();
    CodeBlock::new(MAIN)
      .language("rust")
      .id("block")
      .on_copy(move |code| heard.lock().unwrap().push(code.to_owned()))
      .mount(ctx)
  });
  let written = Arc::new(Written::default());
  stage.tree.set_clipboard_writer(Some(written.clone()));
  let name = |stage: &mut Stage| {
    stage
      .tree
      .get_element_by_id_mut("block-copy")
      .and_then(|element| element.description("a11y_name"))
  };
  assert_eq!(name(&mut stage).as_deref(), Some("Copy code"));
  stage.click("block-copy");
  stage.pass();
  assert_eq!(*written.0.lock().unwrap(), [MAIN]);
  assert_eq!(*told.lock().unwrap(), [MAIN]);
  assert_eq!(name(&mut stage).as_deref(), Some("Copied"), "it shows it copied");
  stage
    .tree
    .tick_timers_at(std::time::Instant::now() + lurq::ui::CODE_BLOCK_COPIED_FOR + std::time::Duration::from_millis(10));
  stage.pass();
  assert_eq!(
    name(&mut stage).as_deref(),
    Some("Copy code"),
    "and then it is the copy button again"
  );
}

/// A clipboard that takes nothing.
struct Refusing;

impl lurq::app::ClipboardWriter for Refusing {
  fn write_text(&self, _text: &str) -> bool {
    false
  }
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn a_copy_the_clipboard_refuses_says_so_for_a_while() {
  let mut stage = Stage::in_app(App::new(), |ctx| CodeBlock::new(MAIN).id("block").mount(ctx));
  stage.tree.set_clipboard_writer(Some(Arc::new(Refusing)));
  let name = |stage: &mut Stage| {
    stage
      .tree
      .get_element_by_id_mut("block-copy")
      .and_then(|element| element.description("a11y_name"))
  };
  stage.click("block-copy");
  stage.pass();
  assert_eq!(name(&mut stage).as_deref(), Some("Couldn't copy"), "not Copied");
  assert!(
    stage.paints(&glyph(lurq::ui::icons::ALERT_CIRCLE)),
    "an alert in place of the copy glyph"
  );
  assert_eq!(
    stage.text(&glyph(lurq::ui::icons::ALERT_CIRCLE)).0,
    light(PaletteColor::Destructive)
  );
  stage
    .tree
    .tick_timers_at(std::time::Instant::now() + lurq::ui::CODE_BLOCK_COPIED_FOR + std::time::Duration::from_millis(10));
  stage.pass();
  assert_eq!(
    name(&mut stage).as_deref(),
    Some("Copy code"),
    "and then it offers to copy again"
  );
}

#[test]
fn the_copy_text_is_copied_in_place_of_the_code_shown() {
  let told = Arc::new(Mutex::new(Vec::new()));
  let heard = told.clone();
  let mut stage = staged(move || {
    let heard = heard.clone();
    CodeBlock::new("deploy --env prod")
      .copy_text("deploy --env prod --token $TOKEN")
      .id("block")
      .on_copy(move |code| heard.lock().unwrap().push(code.to_owned()))
  });
  let written = Arc::new(Written::default());
  stage.tree.set_clipboard_writer(Some(written.clone()));
  stage.click("block-copy");
  stage.pass();
  assert_eq!(*written.0.lock().unwrap(), ["deploy --env prod --token $TOKEN"]);
  assert_eq!(*told.lock().unwrap(), ["deploy --env prod --token $TOKEN"]);
}

/// A clipboard of the app's own: what it was written, and the text it gives a paste.
#[derive(Default)]
struct AppClipboard(Mutex<Vec<String>>);

impl lurq::app::ClipboardWriter for AppClipboard {
  fn write_text(&self, text: &str) -> bool {
    self.0.lock().unwrap().push(text.to_owned());
    true
  }
}

impl lurq::app::ClipboardReader for AppClipboard {
  fn read_text(&self) -> Option<String> {
    self.0.lock().unwrap().last().cloned()
  }

  fn read(&self) -> Option<lurq::app::events::ClipboardContent> {
    self.read_text().map(lurq::app::events::ClipboardContent::text)
  }
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn a_copy_goes_to_the_clipboard_the_app_gives_and_says_what_it_answered() {
  let app = App::new();
  let own = Arc::new(AppClipboard::default());
  app.set_clipboard_writer(Some(own.clone()));
  let mut stage = Stage::in_app(app.clone(), |ctx| CodeBlock::new(MAIN).id("block").mount(ctx));
  stage.click("block-copy");
  stage.pass();
  assert_eq!(*own.0.lock().unwrap(), [MAIN], "the app's clipboard has the code");
  let name = stage
    .tree
    .get_element_by_id_mut("block-copy")
    .and_then(|element| element.description("a11y_name"));
  assert_eq!(name.as_deref(), Some("Copied"));

  // A clipboard of the app's that refuses: the button says it could not copy.
  app.set_clipboard_writer(Some(Arc::new(Refusing)));
  stage
    .tree
    .tick_timers_at(std::time::Instant::now() + lurq::ui::CODE_BLOCK_COPIED_FOR + std::time::Duration::from_millis(10));
  stage.pass();
  stage.click("block-copy");
  stage.pass();
  let name = stage
    .tree
    .get_element_by_id_mut("block-copy")
    .and_then(|element| element.description("a11y_name"));
  assert_eq!(name.as_deref(), Some("Couldn't copy"));
}

#[test]
fn a_test_writer_on_the_tree_stays_over_the_apps() {
  let app = App::new();
  let own = Arc::new(AppClipboard::default());
  app.set_clipboard_writer(Some(own.clone()));
  let mut stage = Stage::in_app(app, |_| CodeBlock::new("cargo add lurq").id("block").into());
  let written = Arc::new(Written::default());
  stage.tree.set_clipboard_writer(Some(written.clone()));
  stage.click("block-copy");
  stage.pass();
  assert_eq!(*written.0.lock().unwrap(), ["cargo add lurq"]);
  assert!(own.0.lock().unwrap().is_empty(), "the app's is passed by");
}

#[test]
fn the_keys_copy_and_paste_through_the_clipboard_the_app_gives() {
  let app = App::new();
  let own = Arc::new(AppClipboard::default());
  app.set_clipboard_writer(Some(own.clone()));
  app.set_clipboard_reader(Some(own.clone()));
  let value = Signal::new("lurq".to_owned());
  let typed = value.clone();
  let mut stage = Stage::in_app(app, move |_| {
    Column::new()
      .width(300.0)
      .child(lurq::components::TextInput::new(typed.clone()).id("field"))
      .into()
  });
  stage.click("field");
  stage.shortcut("a");
  stage.shortcut("c");
  assert_eq!(*own.0.lock().unwrap(), ["lurq"], "Ctrl+C wrote the app's clipboard");
  stage.key("End");
  stage.shortcut("v");
  assert_eq!(value.get_untracked(), "lurqlurq", "Ctrl+V read it");
}

/// The character lurq's font has for an icon.
fn glyph(name: &str) -> String {
  lurq::ui::icons::ALL
    .iter()
    .find_map(|(known, glyph)| (*known == name).then(|| glyph.to_string()))
    .unwrap_or_else(|| panic!("lurq's font has no icon {name:?}"))
}

#[test]
fn the_still_form_copies_too() {
  let mut stage = staged(|| CodeBlock::new("cargo add lurq").id("block"));
  let written = Arc::new(Written::default());
  stage.tree.set_clipboard_writer(Some(written.clone()));
  stage.click("block-copy");
  stage.pass();
  assert_eq!(*written.0.lock().unwrap(), ["cargo add lurq"]);
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn to_tooling_it_is_code_with_its_language() {
  let mut stage = staged(|| CodeBlock::new(MAIN).language("rust").title("main.rs").id("block"));
  let mut said = |key: &str| {
    stage
      .tree
      .get_element_by_id_mut("block")
      .and_then(|element| element.description(key))
  };
  assert_eq!(said("a11y_role").as_deref(), Some("code"));
  assert_eq!(said("a11y_name").as_deref(), Some("main.rs"));
  assert_eq!(said("a11y_language").as_deref(), Some("rust"));
}

/// What is selected in the code of the block with id `block`.
fn selected(stage: &mut Stage) -> Option<String> {
  stage
    .tree
    .get_element_by_id_mut("block-code")
    .and_then(|element| element.as_text_input())
    .and_then(|input| input.selected_text())
}

/// A drag of the pointer from `from` to `to`.
fn drag(stage: &mut Stage, from: (f32, f32), to: (f32, f32)) {
  stage.tree.mouse_move(from.0, from.1);
  stage
    .tree
    .mouse_down(from.0, from.1, lurq::app::events::MouseButton::Left);
  stage.tree.mouse_move(to.0, to.1);
  stage.tree.mouse_up(to.0, to.1, lurq::app::events::MouseButton::Left);
  stage.pass();
}

#[test]
fn a_person_selects_the_code_across_its_lines_and_not_its_numbers() {
  let mut stage = staged(|| CodeBlock::new(MAIN).language("rust").line_numbers(true).id("block"));
  let (x, y, ..) = stage.bounds("block-code");
  // From the start of the first line to the middle of the third: the code's start is past its numbers.
  let start = x + 14.0 + 24.0;
  drag(
    &mut stage,
    (start, y + 12.0 + 10.0),
    (start + 60.0, y + 12.0 + 42.0 + 10.0),
  );
  let selected = selected(&mut stage).expect("a selection");
  assert!(selected.starts_with("fn main() {\n  let app"), "{selected:?}");
  assert!(!selected.contains("2\n"), "{selected:?}");
}

#[test]
fn wrapped_lines_keep_their_numbers_and_a_picked_line_its_band_on_every_row() {
  let long = "let window = WinitWindow::new(app, tree).with_title(\"Notes\").with_size(1280, 800).with_min_size(720, 480);\nwindow.run();";
  let stage = staged(move || {
    CodeBlock::new(long)
      .language("rust")
      .line_numbers(true)
      .wrap(true)
      .highlight_lines([1])
  });
  let shown = numbers(&stage);
  let texts: Vec<&str> = shown.iter().map(|(text, ..)| text.as_str()).collect();
  assert_eq!(texts, ["1", "2"], "each line has its number");
  assert!(
    shown[1].2.y - shown[0].2.y >= 42.0 - 0.01,
    "the second line's number stands after the rows the first wrapped into"
  );
  let bands = bands(&stage);
  assert_eq!(bands.len(), 1);
  assert!(
    bands[0].height >= 42.0 - 0.01,
    "the band covers every row of the line: {}",
    bands[0].height
  );
  assert!(bands[0].width >= 418.0, "across the block: {}", bands[0].width);
}

#[test]
fn wrapped_a_selection_runs_from_line_to_line_with_its_breaks() {
  let code = "let first = 1;\nlet second = 2;\nlet third = 3;";
  let mut stage = staged(move || CodeBlock::new(code).language("rust").wrap(true).id("block"));
  let (x, y, ..) = stage.bounds("block-code");
  // From the middle of the first line to the middle of the third.
  drag(
    &mut stage,
    (x + 14.0 + 40.0, y + 12.0 + 10.0),
    (x + 14.0 + 40.0, y + 12.0 + 42.0 + 10.0),
  );
  let selected = selected(&mut stage).expect("a selection");
  assert_eq!(selected.lines().count(), 3, "{selected:?}");
  assert!(selected.contains("\nlet second = 2;\n"), "{selected:?}");
}

#[test]
fn from_the_keyboard_the_code_is_a_stop_whose_caret_selects_and_copies() {
  let mut stage = staged(|| CodeBlock::new(MAIN).language("rust").copy_button(false).id("block"));
  let written = Arc::new(Written::default());
  stage.tree.set_clipboard_writer(Some(written.clone()));
  // Tab reaches the code.
  stage.key("Tab");
  let focused = stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned));
  assert_eq!(focused.as_deref(), Some("block-code"));
  // Ctrl+Home takes the caret to the start; Shift and the arrows select from it: the first two letters.
  stage.tree.key_down("Home".into(), "Home".into(), false, true, false);
  stage.tree.key_up("Home".into(), "Home".into(), false, true, false);
  for _ in 0..2 {
    stage
      .tree
      .key_down("ArrowRight".into(), "ArrowRight".into(), true, false, false);
    stage
      .tree
      .key_up("ArrowRight".into(), "ArrowRight".into(), true, false, false);
  }
  stage.pass();
  assert_eq!(selected(&mut stage).as_deref(), Some("fn"));
  // Ctrl+A takes it all, Ctrl+C copies it.
  stage.shortcut("a");
  stage.shortcut("c");
  stage.pass();
  assert_eq!(selected(&mut stage).as_deref(), Some(MAIN));
  assert_eq!(*written.0.lock().unwrap(), [MAIN]);
  // It is read only: a letter typed changes nothing.
  stage.key("x");
  let value = stage
    .tree
    .get_element_by_id_mut("block-code")
    .and_then(|element| element.as_text_input())
    .map(|input| input.value());
  assert_eq!(value.as_deref(), Some(MAIN));
}

#[test]
fn a_block_without_its_frame_and_with_rows_of_its_own() {
  let mut stage = staged(|| {
    CodeBlock::new(MAIN)
      .copy_button(false)
      .framed(false)
      .row_height(24.0)
      .id("block")
  });
  assert!(stage.border("block").is_none(), "no edge");
  assert!(
    fills(&stage, light(PaletteColor::Background))
      .iter()
      .all(|quad| quad.height < 50.0),
    "no fill"
  );
  let (.., height) = stage.bounds("block");
  // Four rows of 24, and the board's 14 above and below less what a row takes past the code's own line.
  let leading = (24.0 - 13.0 * 1.3) / 2.0;
  assert!((height - (4.0 * 24.0 + 2.0 * (14.0 - leading))).abs() < 0.6, "{height}");
}

#[test]
fn a_token_takes_the_style_of_its_less_particular_name() {
  // `if` is a `keyword.control`: with no style of that name, it takes the block's style of a keyword.
  let stage = staged(|| {
    CodeBlock::new("if x { let y = 1; } // done")
      .language("rust")
      .selectable(false)
      .token_style("keyword", |s| s.foreground(PaletteColor::Destructive))
      .token_style("comment", |s| s.italic().bold())
  });
  assert_eq!(run(&stage, "if").style.color, light(PaletteColor::Destructive));
  assert_eq!(run(&stage, "let").style.color, light(PaletteColor::Destructive));
  let comment = run(&stage, "// done").style;
  assert_eq!(
    comment.color,
    light(PaletteColor::MutedForeground),
    "the theme's color, under the block's style"
  );
  assert_eq!((comment.weight, comment.style), (FontWeight::Bold, FontStyle::Italic));
  // The theme's color for the particular name: `if` in it, `let` in the keywords' own.
  let app = App::new();
  app
    .theme()
    .set_preferred_appearance(Some(lurq::app::window::Appearance::Light));
  app.theme().set_code_color("keyword.control", PaletteColor::Warning);
  let stage = Stage::in_app(app, |_| {
    Column::new()
      .width(420.0)
      .child(CodeBlock::new("if x { let y = 1; }").language("rust").selectable(false))
      .into()
  });
  assert_eq!(run(&stage, "if").style.color, light(PaletteColor::Warning));
  assert_eq!(run(&stage, "let").style.color, light(PaletteColor::Info));
}

#[test]
fn a_theme_with_no_code_colors_draws_code_in_one() {
  let app = App::new();
  app.theme().set_code_scheme(CodeScheme::new());
  let stage = Stage::in_app(app, |_| {
    Column::new()
      .width(420.0)
      .child(CodeBlock::new("fn main() {}").language("rust").selectable(false))
      .into()
  });
  assert_eq!(runs(&stage).iter().flatten().count(), 1);
}

#[test]
fn an_apps_tokenizer_for_a_language_colors_what_shows_it_and_a_new_one_draws_it_again() {
  const LANGUAGE: &str = "lang-of-the-block-tests";
  let mut stage = staged(|| CodeBlock::new("alpha beta").language(LANGUAGE).selectable(false));
  assert_eq!(runs(&stage).iter().flatten().count(), 1, "plain while it has none");
  lurq::code::register_tokenizer(LANGUAGE, |_: &str| vec![CodeToken::new(0..5, token::KEYWORD)]);
  stage.pass();
  assert_eq!(run(&stage, "alpha").style.color, light(PaletteColor::Info));
  // Registered again, in place of the first: what showed it is drawn by the new one.
  stage
    .app
    .register_tokenizer(LANGUAGE, |_: &str| vec![CodeToken::new(6..10, token::STRING)]);
  stage.pass();
  assert_eq!(run(&stage, "beta").style.color, light(PaletteColor::Success));
  assert_eq!(run(&stage, "alpha ").style.color, light(PaletteColor::Foreground));
}

#[test]
fn a_blocks_own_tokenizer_is_over_its_languages_and_what_it_gave_is_kept() {
  let asked = Arc::new(AtomicUsize::new(0));
  let counter = asked.clone();
  let own = CodeTokenizer::new(move |code: &str| {
    counter.fetch_add(1, Ordering::SeqCst);
    vec![CodeToken::new(0..code.find(' ').unwrap_or(code.len()), token::STRING)]
  });
  let tick = Signal::new(0);
  let seen = tick.clone();
  let mut stage = staged(move || {
    let _ = seen.get();
    CodeBlock::new("fn main() {}")
      .language("rust")
      .tokenizer(own.clone())
      .selectable(false)
  });
  assert_eq!(
    run(&stage, "fn").style.color,
    light(PaletteColor::Success),
    "the block's own"
  );
  // Rendered again with the same code, the block does not ask its tokenizer again.
  for next in 1..4 {
    tick.set(next);
    stage.pass();
  }
  assert_eq!(asked.load(Ordering::SeqCst), 1);
}

/// The lines of code a frame paints, by the numbers in their names (`step_12`).
fn painted_steps(stage: &Stage) -> Vec<usize> {
  let mut steps: Vec<usize> = runs(stage)
    .into_iter()
    .flatten()
    .flat_map(|span| {
      span
        .text
        .split("step_")
        .skip(1)
        .filter_map(|rest| rest.split(|ch: char| !ch.is_ascii_digit()).next()?.parse().ok())
        .collect::<Vec<usize>>()
    })
    .collect();
  steps.sort_unstable();
  steps.dedup();
  steps
}

/// Ten thousand lines of Rust.
fn long_code() -> String {
  (0..10_000)
    .map(|index| format!("fn step_{index}(ctx: &mut Ctx) -> u32 {{ {index} * 2 }} // step"))
    .collect::<Vec<_>>()
    .join("\n")
}

#[test]
fn a_long_block_lays_out_and_paints_the_lines_in_view() {
  let code = long_code();
  let mut stage = staged(move || {
    CodeBlock::new(&code)
      .language("rust")
      .line_numbers(true)
      .max_height(400.0)
      .id("block")
  });
  // The block is as high as its greatest height; what it scrolls is as high as every line.
  let (x, y, width, height) = stage.bounds("block");
  assert!((height - 400.0).abs() < 0.6, "{height}");
  let (.., code_height) = stage.bounds("block-code");
  assert!((code_height - (10_000.0 * 21.0 + 24.0)).abs() < 1.0, "{code_height}");
  // A frame lays out and paints its first lines and a chunk past them, not the ten thousand.
  let shown = painted_steps(&stage);
  assert_eq!(shown.first(), Some(&0));
  assert!(shown.len() <= 3 * 64, "{} lines painted", shown.len());
  assert!(stage.quads().len() < 200, "{} quads", stage.quads().len());
  // Scrolled half way down, it paints the lines there.
  stage
    .tree
    .scroll(x + width / 2.0, y + height / 2.0, 0.0, -105_000.0, ScrollPhase::Scroll);
  stage.pass();
  stage.pass();
  let shown = painted_steps(&stage);
  assert!(
    shown.iter().any(|step| (4_990..5_020).contains(step)),
    "{:?}..{:?}",
    shown.first(),
    shown.last()
  );
  assert!(!shown.contains(&0));
  assert!(shown.len() <= 4 * 64, "{} lines painted", shown.len());
}

#[test]
fn inline_code_in_a_language_is_colored_as_a_block_is() {
  let stage = staged_element(|| InlineCode::new("let n = 42;").language("rust").into());
  assert_eq!(run(&stage, "let").style.color, light(PaletteColor::Info));
  assert_eq!(run(&stage, "42").style.color, light(PaletteColor::Warning));
  // Without a language it is one text, as it was.
  let stage = staged_element(|| InlineCode::new("let n = 42;").into());
  assert!(runs(&stage).is_empty());
  assert_eq!(stage.text("let n = 42;").0, light(PaletteColor::Foreground));
}

/// A page of a window of 800 by 600 that scrolls down: a heading, `block`, and room after it.
fn page_with(block: impl Fn() -> CodeBlock + Send + Sync + 'static) -> Stage {
  let app = App::new();
  app
    .theme()
    .set_preferred_appearance(Some(lurq::app::window::Appearance::Light));
  let mut stage = Stage::first_frame_in(app, 800.0, 600.0, move |_| {
    ScrollArea::new()
      .width(Length::Fill)
      .height(600.0)
      .id("page")
      .child(
        Column::new()
          .width(780.0)
          .child(Column::new().height(300.0))
          .child(block())
          .child(Column::new().height(300.0)),
      )
      .into()
  });
  stage.pass();
  stage
}

#[test]
fn a_long_block_with_no_greatest_height_paints_the_lines_in_the_window() {
  let code = long_code();
  let mut stage = page_with(move || CodeBlock::new(&code).language("rust").id("block"));
  // As high as every line, on a page that scrolls past it.
  let (.., height) = stage.bounds("block");
  assert!(height > 10_000.0 * 21.0, "{height}");
  let shown = painted_steps(&stage);
  assert_eq!(shown.first(), Some(&0));
  assert!(shown.len() <= 3 * 64, "{} lines painted", shown.len());
  // The page scrolled down to the middle of the block: the lines there are painted, and the first are not.
  stage.tree.scroll(400.0, 300.0, 0.0, -105_000.0, ScrollPhase::Scroll);
  stage.pass();
  stage.pass();
  let shown = painted_steps(&stage);
  assert!(
    shown.iter().any(|step| (4_970..5_030).contains(step)),
    "{:?}..{:?}",
    shown.first(),
    shown.last()
  );
  assert!(!shown.contains(&0));
  assert!(shown.len() <= 4 * 64, "{} lines painted", shown.len());
}

#[test]
fn a_long_wrapped_blocks_first_frame_is_its_settled_one() {
  let code: String = (0..2_000)
    .map(|index| format!("let line_{index} = \"{}\";", "word ".repeat(30)))
    .collect::<Vec<_>>()
    .join("\n");
  let scene = move |_: &mut lurq::app::ctx::Ctx| -> Element {
    Column::new()
      .width(420.0)
      .child(
        CodeBlock::new(&code)
          .language("rust")
          .wrap(true)
          .selectable(false)
          .max_height(400.0)
          .id("block"),
      )
      .into()
  };
  let mut stage = Stage::first_frame(420.0, 600.0, scene);
  let first = stage.bounds("block-code");
  let first_picture = stage.picture();
  stage.frame();
  assert_eq!(
    stage.bounds("block-code"),
    first,
    "the column of the lines is as high in both"
  );
  assert_eq!(stage.picture(), first_picture);
  // Each line wraps at the room's width: the column counts more rows than lines.
  assert!(first.3 > 2_000.0 * 21.0 * 2.0, "{}", first.3);
}

#[test]
fn a_long_block_selected_whole_from_the_keyboard_copies_all_of_it() {
  let code = long_code();
  let whole = code.clone();
  let mut stage = staged(move || CodeBlock::new(&code).language("rust").max_height(400.0).id("block"));
  let written = Arc::new(Written::default());
  stage.tree.set_clipboard_writer(Some(written.clone()));
  // Tab reaches its room, Ctrl+A takes the code whole and Ctrl+C copies all ten thousand lines.
  stage.key("Tab");
  stage.shortcut("a");
  stage.pass();
  assert!(
    !fills(&stage, light(PaletteColor::Selection)).is_empty(),
    "the lines on the selection's color"
  );
  stage.shortcut("c");
  stage.pass();
  assert_eq!(written.0.lock().unwrap().last(), Some(&whole));
  // A press in the code lets the whole go.
  let (x, y, ..) = stage.bounds("block-code");
  stage
    .tree
    .mouse_down(x + 40.0, y + 30.0, lurq::app::events::MouseButton::Left);
  stage
    .tree
    .mouse_up(x + 40.0, y + 30.0, lurq::app::events::MouseButton::Left);
  stage.pass();
  assert!(fills(&stage, light(PaletteColor::Selection)).is_empty());
}
