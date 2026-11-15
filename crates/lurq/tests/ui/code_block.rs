//! `ui::CodeBlock`: the design's Code Block (`kyzrV`), its colors, its lines, its copy button and its scrolling.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    theme::{PaletteColor, ThemePalette},
  },
  components::Column,
  layout::quad::{Quad, QuadContent, RichTextSpan},
  node::color::Color,
  ui::{CodeBlock, CodeBlockPart},
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
  // The bar: 36 high on Muted.
  let bars = fills(&stage, light(PaletteColor::Muted));
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
  let stage =
    staged(|| CodeBlock::new("fn main() {\n  let n = 42; // the answer\n  println!(\"{n}\");\n}").language("rust"));
  assert_eq!(
    run(&stage, "fn").style.color,
    light(PaletteColor::Info),
    "the board's keywords"
  );
  assert_eq!(run(&stage, "42").style.color, light(PaletteColor::Warning));
  assert_eq!(run(&stage, "\"{n}\"").style.color, light(PaletteColor::Success));
  assert_eq!(
    run(&stage, "// the answer").style.color,
    light(PaletteColor::MutedForeground)
  );
  let code = run(&stage, " main() {\n  ");
  assert_eq!(code.style.color, light(PaletteColor::Foreground));
  assert_eq!(code.style.font_size, 13.0);
  // A language the block does not know is one run in the code's color.
  let stage = staged(|| CodeBlock::new("[package]\nname = \"lurq\"").language("toml"));
  assert_eq!(runs(&stage).iter().flatten().count(), 1);
}

#[test]
fn a_theme_colors_a_kind_of_word_otherwise() {
  let stage = staged(|| {
    CodeBlock::new("let x = 1;")
      .language("rust")
      .style(CodeBlockPart::Keyword, |s| s.foreground(PaletteColor::Destructive))
  });
  assert_eq!(run(&stage, "let").style.color, light(PaletteColor::Destructive));
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
  let numbers: Vec<RichTextSpan> = runs(&stage)
    .into_iter()
    .find(|spans| spans.first().is_some_and(|span| span.text.starts_with("9")))
    .expect("the numbers");
  let texts: Vec<&str> = numbers.iter().map(|span| span.text.as_str()).collect();
  assert_eq!(texts, ["9\n", "10\n", "11\n", "12"]);
  assert_eq!(numbers[0].style.color, light(PaletteColor::MutedForeground));
  assert_eq!(
    numbers[1].style.color,
    light(PaletteColor::Foreground),
    "a picked line's number"
  );
  // The band of line 10, the second: a line of 21 under the bar and the first line.
  // The band of line 10, the second: a line of 21 under the bar, 12 of padding and the first line, across the
  // block, in the stops of the gradient down the room the lines are in.
  let (_, top, ..) = stage.bounds("block");
  let room = stage
    .quads()
    .into_iter()
    .find_map(|quad| match quad.content {
      QuadContent::Rect {
        gradient: Some(gradient),
        ..
      } => Some((quad.y, quad.width, quad.height, gradient)),
      _ => None,
    })
    .expect("the lines' room has the bands");
  let (y, width, height, gradient) = room;
  assert!(width >= 418.0, "across the block: {width}");
  assert!((y - (top + 36.0)).abs() < 0.6);
  let band: Vec<f32> = gradient
    .stops
    .iter()
    .filter(|stop| stop.color[3] > 0.0)
    .map(|stop| y + stop.position * height)
    .collect();
  assert_eq!(band.len(), 2, "one band");
  assert!((band[0] - (top + 36.0 + 12.0 + 21.0)).abs() < 0.6, "{band:?}");
  assert!((band[1] - band[0] - 21.0).abs() < 0.6, "{band:?}");
  // The code is a text of its own: a selection does not take the numbers.
  let code = runs(&stage)
    .into_iter()
    .find(|spans| spans.iter().any(|span| span.text == "fn"));
  assert!(code.is_some_and(|spans| !spans.iter().any(|span| span.text.contains("10"))));
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
  assert!(fills(&stage, light(PaletteColor::Muted)).is_empty());
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

#[test]
fn a_person_selects_the_code_across_its_lines_and_not_its_numbers() {
  let mut stage = staged(|| CodeBlock::new(MAIN).language("rust").line_numbers(true).id("block"));
  let (code_x, code_y, ..) = stage.bounds("block-code");
  // From the start of the first line to the middle of the third.
  stage.tree.mouse_move(code_x + 1.0, code_y + 10.0);
  stage
    .tree
    .mouse_down(code_x + 1.0, code_y + 10.0, lurq::app::events::MouseButton::Left);
  stage.tree.mouse_move(code_x + 60.0, code_y + 21.0 * 2.0 + 10.0);
  stage.tree.mouse_up(
    code_x + 60.0,
    code_y + 21.0 * 2.0 + 10.0,
    lurq::app::events::MouseButton::Left,
  );
  stage.pass();
  let selected = stage.tree.selected_text().expect("a selection");
  assert!(selected.starts_with("fn main() {\n  let app"), "{selected:?}");
  assert!(selected.contains('\n') && !selected.contains("2\n"), "{selected:?}");
}

#[test]
fn wrapped_lines_keep_their_numbers_and_a_picked_line_its_band_across_the_block() {
  let long = "let window = WinitWindow::new(app, tree).with_title(\"Notes\").with_size(1280, 800).with_min_size(720, 480);\nwindow.run();";
  let stage = staged(move || {
    CodeBlock::new(long)
      .language("rust")
      .line_numbers(true)
      .wrap(true)
      .highlight_lines([2])
  });
  let numbers: Vec<String> = runs(&stage)
    .into_iter()
    .filter(|spans| spans.len() == 1 && spans[0].text.parse::<usize>().is_ok())
    .map(|spans| spans[0].text.clone())
    .collect();
  assert_eq!(numbers, ["1", "2"], "each line has its number");
  let painted = stage
    .quads()
    .into_iter()
    .find(|quad| matches!(&quad.content, QuadContent::RichText { spans, .. } if spans[0].text == "1"))
    .expect("the first number");
  assert!(painted.width > 0.0);
  let band = fills(&stage, light(PaletteColor::Highlight))
    .into_iter()
    .find(|quad| quad.height < 40.0)
    .expect("the band");
  assert!(band.width >= 418.0, "across the block: {}", band.width);
}
