//! The page of the code block: its playground, the board's block, numbered lines with one picked out, long lines
//! that scroll or wrap, and a long file in a block of a greatest height.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::Column,
  core::Signal,
  node::Element,
  ui::CodeBlock,
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// The board's code (`kyzrV`).
const MAIN: &str = "fn main() {\n  let app = App::new();\n  app.theme().set_mode(Mode::Dark);\n}";

const CARGO: &str = "[package]\nname = \"notes\"\nversion = \"0.1.0\"\n\n[dependencies]\nlurq = { version = \"0.62\", features = [\"winit\", \"wgpu\"] }";

const LONG: &str = "let window = WinitWindow::new(app, tree).with_title(\"Notes\").with_size(1280, 800).with_min_size(720, 480).with_decorations(false);\nwindow.run();";

/// A file long enough to scroll down in a block of a greatest height.
fn file() -> String {
  let mut lines = vec!["use lurq::prelude::*;".to_owned(), String::new()];
  for index in 1..=24 {
    lines.push(format!(
      "fn step_{index}(ctx: &mut Ctx) -> u32 {{ {index} * 2 }} // step {index}"
    ));
  }
  lines.join("\n")
}

/// The samples the playground's code is chosen from: their name, their code, their language and their title.
fn samples() -> [(&'static str, String, &'static str, &'static str); 4] {
  [
    ("main.rs", MAIN.to_owned(), "rust", "main.rs"),
    ("Cargo.toml", CARGO.to_owned(), "toml", "Cargo.toml"),
    ("a long line", LONG.to_owned(), "rust", ""),
    ("a long file", file(), "rust", "steps.rs"),
  ]
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .width(lurq::node::dimension::Dimension::Pct(100.0))
    .spacing(10.0)
    .child(shown)
    .child(quiet(caption, 12.0))
}

/// The numbers of lines written as `2, 4-5`.
fn lines_of(text: &str) -> Vec<usize> {
  text
    .split(',')
    .flat_map(|part| {
      let part = part.trim();
      match part.split_once('-') {
        Some((from, to)) => match (from.trim().parse::<usize>(), to.trim().parse::<usize>()) {
          (Ok(from), Ok(to)) if from <= to => (from..=to).collect(),
          _ => Vec::new(),
        },
        None => part.parse().into_iter().collect(),
      }
    })
    .collect()
}

pub struct CodeBlockPage {
  frame: Frame,
  playground: Playground,
  sample: Signal<usize>,
  language: Signal<String>,
  title: Signal<String>,
  line_numbers: Signal<bool>,
  highlighted: Signal<String>,
  wrap: Signal<bool>,
  max_height: Signal<usize>,
  copy_button: Signal<bool>,
  selectable: Signal<bool>,
}

/// The greatest heights the playground offers.
const HEIGHTS: [(&str, Option<f32>); 3] = [("none", None), ("200", Some(200.0)), ("320", Some(320.0))];

impl CodeBlockPage {
  /// The playground: the block as the panel's props make it, mounted, so that its copy button shows it copied.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let sample = self.sample.get().min(3);
    let (_, code, ..) = samples()[sample].clone();
    let (language, title) = (self.language.get(), self.title.get());
    let (line_numbers, wrap) = (self.line_numbers.get(), self.wrap.get());
    let (copy_button, selectable) = (self.copy_button.get(), self.selectable.get());
    let highlighted = self.highlighted.get();
    let max_height = HEIGHTS[self.max_height.get().min(HEIGHTS.len() - 1)].1;
    let record = self.playground.recorder();
    let forced = self.playground.forced();
    let make = {
      let (code, language, title, highlighted) = (code.clone(), language.clone(), title.clone(), highlighted.clone());
      move || -> CodeBlock {
        let mut block = CodeBlock::new(&code)
          .line_numbers(line_numbers)
          .highlight_lines(lines_of(&highlighted))
          .wrap(wrap)
          .copy_button(copy_button)
          .selectable(selectable);
        if !language.is_empty() {
          block = block.language(&language);
        }
        if !title.is_empty() {
          block = block.title(&title);
        }
        if let Some(height) = max_height {
          block = block.max_height(height);
        }
        block
      }
    };
    let shown = {
      let (make, playground) = (make.clone(), &self.playground);
      Shown::built(move |ctx| {
        playground
          .styled(
            make()
              .force(forced)
              .on_copy(move |code| record(format!("on_copy({} lines)", code.lines().count()))),
          )
          .mount(ctx)
      })
    };

    let mut code_lines = vec![format!("CodeBlock::new({:?})", samples()[sample].0)];
    if !language.is_empty() {
      code_lines.push(format!("    .language({language:?})"));
    }
    if !title.is_empty() {
      code_lines.push(format!("    .title({title:?})"));
    }
    if line_numbers {
      code_lines.push("    .line_numbers(true)".to_owned());
    }
    let picked = lines_of(&highlighted);
    if !picked.is_empty() {
      code_lines.push(format!("    .highlight_lines({picked:?})"));
    }
    if wrap {
      code_lines.push("    .wrap(true)".to_owned());
    }
    if let Some(height) = max_height {
      code_lines.push(format!("    .max_height({height})"));
    }
    if !copy_button {
      code_lines.push("    .copy_button(false)".to_owned());
    }
    if !selectable {
      code_lines.push("    .selectable(false)".to_owned());
    }
    code_lines.push("    .on_copy(|code| ..)".to_owned());
    code_lines.push("    .mount(ctx)".to_owned());

    let header = !title.is_empty() || !language.is_empty() || copy_button;
    let mut tree = vec![format!(
      "root        {}   Background · Border · radius lg",
      self.playground.measured()
    )];
    if header {
      tree.push("├─ header   36 · Muted · title 11 mono · copy 28".to_owned());
    }
    tree.push(format!(
      "└─ body     {}{} · code 13 mono, lines 21",
      if line_numbers { "numbers · " } else { "" },
      if wrap { "wraps" } else { "scrolls across" }
    ));

    let set_sample = {
      let (sample, language, title) = (self.sample.clone(), self.language.clone(), self.title.clone());
      move |index: usize| {
        let (_, _, sample_language, sample_title) = samples()[index].clone();
        sample.set(index);
        language.set(sample_language.to_owned());
        title.set(sample_title.to_owned());
      }
    };
    let set_height = self.max_height.clone();
    let names: Vec<(&str, usize)> = samples()
      .iter()
      .enumerate()
      .map(|(index, sample)| (sample.0, index))
      .collect();
    let heights: Vec<(&str, usize)> = HEIGHTS
      .iter()
      .enumerate()
      .map(|(index, height)| (height.0, index))
      .collect();
    let reset = {
      let set_sample = set_sample.clone();
      let (highlighted, height) = (self.highlighted.clone(), self.max_height.clone());
      let on = [self.copy_button.clone(), self.selectable.clone()];
      let off = [self.line_numbers.clone(), self.wrap.clone()];
      Arc::new(move || {
        set_sample(0);
        highlighted.set(String::new());
        height.set(0);
        for flag in &on {
          flag.set(true);
        }
        for flag in &off {
          flag.set(false);
        }
      })
    };
    self.playground.card(
      ctx,
      Spec {
        title: "CodeBlock",
        make: Arc::new(move || make().into()),
        shown,
        fill: true,
        forceable: true,
        facts: format!("{} lines", code.lines().count()),
        event: "on_copy",
        code: code_lines,
        tree,
        controls: vec![
          prop("new", "&str", self.playground.select(0, &names, &sample, set_sample)),
          prop("language", "&str", field(&self.language)),
          prop("title", "&str", field(&self.title)),
          prop("highlight_lines", "lines: 2, 4-5", field(&self.highlighted)),
          prop(
            "max_height",
            "impl Into<Length>",
            self
              .playground
              .select(1, &heights, &self.max_height.get(), move |index| set_height.set(index)),
          ),
          divider(),
          flag("line_numbers", &self.line_numbers),
          flag("wrap", &self.wrap),
          flag("copy_button", &self.copy_button),
          flag("selectable", &self.selectable),
        ],
        changed: [
          sample != 0,
          language != "rust",
          title != "main.rs",
          !highlighted.is_empty(),
          max_height.is_some(),
          line_numbers,
          wrap,
          !copy_button,
          !selectable,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset,
      },
    )
  }
}

impl Component for CodeBlockPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      sample: ctx.signal(0),
      language: ctx.signal("rust".to_owned()),
      title: ctx.signal("main.rs".to_owned()),
      line_numbers: ctx.signal(false),
      highlighted: ctx.signal(String::new()),
      wrap: ctx.signal(false),
      max_height: ctx.signal(0),
      copy_button: ctx.signal(true),
      selectable: ctx.signal(true),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = item(
      CodeBlock::new(MAIN).language("rust").title("main.rs").mount(ctx),
      "main.rs",
    );
    let numbered = item(
      CodeBlock::new(CARGO)
        .language("toml")
        .title("Cargo.toml")
        .line_numbers(true)
        .highlight_lines([6])
        .mount(ctx),
      "line_numbers · highlight_lines([6])",
    );
    let long = Column::new()
      .width(lurq::node::dimension::Dimension::Pct(100.0))
      .spacing(16.0)
      .child(item(CodeBlock::new(LONG).language("rust").mount(ctx), "scrolls across"))
      .child(item(
        CodeBlock::new(LONG)
          .language("rust")
          .line_numbers(true)
          .wrap(true)
          .mount(ctx),
        "wrap(true) · line_numbers",
      ));
    let tall = item(
      CodeBlock::new(&file())
        .language("rust")
        .title("steps.rs")
        .line_numbers(true)
        .max_height(240.0)
        .mount(ctx),
      "max_height(240)",
    );
    let rows = [
      (
        "new",
        "&str",
        "",
        "The code. A newline at its end is not a line of its own.",
      ),
      (
        "language",
        "&str",
        "",
        "What colors its words where the block knows it (Rust), what the bar shows where it has no title, and what \
         tooling is told.",
      ),
      (
        "title",
        "&str",
        "the language",
        "What the bar shows: a file's name. It ends in an ellipsis where it has no room.",
      ),
      ("label", "&str", "the title", "What it is called to tooling."),
      (
        "line_numbers",
        "bool",
        "false",
        "The numbers of the lines, in a column a selection does not take.",
      ),
      (
        "first_line",
        "usize",
        "1",
        "The number of the first line, where the code is a part of a file.",
      ),
      (
        "highlight_lines",
        "impl IntoIterator<Item = usize>",
        "",
        "Lines picked out by their numbers: a band behind each across the block.",
      ),
      (
        "wrap",
        "bool",
        "false",
        "Wraps the lines at its width; otherwise each is one line that scrolls sideways.",
      ),
      (
        "min_height / max_height",
        "impl Into<Length>",
        "",
        "The least and the greatest height, its bar with it: past the greatest, the code scrolls down.",
      ),
      (
        "copy_button",
        "bool",
        "true",
        "The button in the bar that copies the code.",
      ),
      (
        "copy_label / copied_text",
        "&str",
        "Copy code / Copied",
        "The copy button's name, and its name once it copied.",
      ),
      (
        "on_copy",
        "Fn(&str)",
        "",
        "Told the code a press of the copy button put on the clipboard.",
      ),
      (
        "selectable",
        "bool",
        "true",
        "Whether a person selects the code with the pointer, across its lines.",
      ),
      (
        "mount",
        "&mut Ctx",
        "",
        "Mounted, the copy button shows a check for 2 seconds after it copied.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Code Block"],
        title: "Code Block",
        description: "A block of code: its lines in the family of code, its words in the colors of what they are, \
                      under a bar with its name and a button that copies it.",
        import: Some("use lurq::ui::CodeBlock;"),
        status: Some("Beta"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of CodeBlock is a control on the right.",
          playground,
        ),
        Section::new(
          "Code block",
          "The board's block: a bar of 36 with the file's name and the copy button, the code in the family of code.",
          self.frame.demo(
            0,
            board,
            "CodeBlock::new(..).language(\"rust\").title(\"main.rs\")",
            &[
              "CodeBlock::new(\"fn main() {\\n  let app = App::new();\\n  app.theme().set_mode(Mode::Dark);\\n}\")",
              "    .language(\"rust\")",
              "    .title(\"main.rs\")",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Numbered lines",
          "The numbers of the lines stand in a column of their own; a line picked out has a band behind it.",
          self.frame.demo(
            1,
            numbered,
            ".line_numbers(true).highlight_lines([6])",
            &[
              "CodeBlock::new(cargo_toml)",
              "    .language(\"toml\")",
              "    .title(\"Cargo.toml\")",
              "    .line_numbers(true)",
              "    .highlight_lines([6])",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Long lines",
          "A line longer than the block scrolls sideways under the scroll area's bar, or wraps.",
          self.frame.demo(
            2,
            long,
            ".wrap(true)",
            &[
              "CodeBlock::new(long).language(\"rust\").mount(ctx)",
              "CodeBlock::new(long).language(\"rust\").line_numbers(true).wrap(true).mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "A greatest height",
          "Past its greatest height, the code scrolls down; the bar stays.",
          self.frame.demo(
            3,
            tall,
            ".max_height(240)",
            &[
              "CodeBlock::new(steps_rs)",
              "    .language(\"rust\")",
              "    .title(\"steps.rs\")",
              "    .line_numbers(true)",
              "    .max_height(240)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a code block is given.",
          props_table(&rows, &["new"], self.frame.room()),
        ),
      ],
    )
  }
}
