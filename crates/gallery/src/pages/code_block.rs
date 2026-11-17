//! The page of the code block: its playground, the board's block, numbered lines with one picked out, long lines
//! that scroll or wrap, a long file in a block of a greatest height, ten thousand lines of which only those in view
//! are drawn, and a tokenizer of another crate plugged in.

use std::sync::{Arc, LazyLock};

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{CodeScheme, CodeStyle},
  },
  code::CodeHighlighter,
  components::Column,
  core::Signal,
  node::{Element, color::Color},
  ui::{CodeBlock, SegmentedControl},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
  syntect_code::{self, Syntect},
};

/// The board's code (`kyzrV`).
const MAIN: &str = "fn main() {\n  let app = App::new();\n  app.theme().set_mode(Mode::Dark);\n}";

const CARGO: &str = "[package]\nname = \"notes\"\nversion = \"0.1.0\"\n\n[dependencies]\nlurq = { version = \"0.62\", features = [\"winit\", \"wgpu\"] }";

const LONG: &str = "let window = WinitWindow::new(app, tree).with_title(\"Notes\").with_size(1280, 800).with_min_size(720, 480).with_decorations(false);\nwindow.run();";

/// The code the page shows by two tokenizers and in two schemes of colors.
const NOTE: &str = "/// A note, kept by its title.\n#[derive(Debug, Clone)]\npub struct Note<'a> {\n  title: &'a str,\n  words: usize,\n}\n\nfn main() {\n  let note = Note { title: \"lurq\", words: 0x2A };\n  if note.words > 40 {\n    println!(\"{} has {} words\", note.title, note.words); // a lot\n  }\n}";

const QUERY: &str = "-- The notes with the most words.\nSELECT n.title, COUNT(w.id) AS words\nFROM notes n\nLEFT JOIN words w ON w.note_id = n.id\nWHERE n.title LIKE 'lurq%' AND n.created > $1\nGROUP BY n.title\nORDER BY words DESC\nLIMIT 10;";

const WORKFLOW: &str = "# The checks of a pull request.\nname: CI\non:\n  pull_request:\n    branches: [master]\njobs:\n  test:\n    runs-on: windows-latest\n    timeout-minutes: 30\n    steps:\n      - uses: actions/checkout@v4\n      - run: |\n          cargo test --workspace\n          cargo check -p gallery";

const NOTES_PY: &str = "\"\"\"The notes, by their words.\"\"\"\nfrom dataclasses import dataclass\n\n@dataclass(frozen=True)\nclass Note:\n    title: str\n    words: int = 0\n\n    def summary(self) -> str:\n        if self.words > 40:\n            return f\"{self.title}: a lot\"\n        return None  # few";

const APP_TS: &str = "interface Note { title: string; words: number }\n\nexport async function load(url: string): Promise<Note[]> {\n  const response = await fetch(`${url}/notes`);\n  if (!response.ok) throw new Error(\"no notes\");\n  return (await response.json()).filter((note: Note) => /lurq/i.test(note.title));\n}";

const PAGE_HTML: &str = "<!DOCTYPE html>\n<html lang=\"en\">\n  <!-- The notes. -->\n  <body class=\"notes\">\n    <h1 id=\"title\">Notes &amp; words</h1>\n    <a href=\"/new\" hidden>New</a>\n  </body>\n</html>";

const STYLE_CSS: &str = "/* A note's card. */\n.note > h1:hover, #title {\n  color: #2563eb;\n  margin: 0 -4px 1.5em;\n  font-family: \"Inter\", sans-serif;\n}\n@media (max-width: 640px) {\n  .note { padding: 8px !important; }\n}";

const PACKAGE_JSON: &str = "{\n  \"name\": \"notes\",\n  \"version\": \"1.0.0\",\n  \"private\": true,\n  \"scripts\": { \"start\": \"vite\" },\n  \"words\": [1, 2.5, null]\n}";

const BUILD_SH: &str = "#!/usr/bin/env bash\n# Builds the gallery and opens it on a page.\nset -euo pipefail\nexport ROUTE=\"${1:-/code-block}\"\nif cargo build -p gallery --features mcp; then\n  ./target/debug/gallery.exe --route \"$ROUTE\" | tee gallery.log\nfi";

const README_MD: &str = "# Notes\n\nA *small* app of **notes**, built with `lurq`: see [the guide](https://lurq.dev).\n\n- one note a line\n- 1 000 words at most\n> Kept on the disk.\n\n```sh\ncargo run\n```";

/// What the page shows of a language lurq has no highlighter for: syntect's, registered at the gallery's start.
const MAIN_GO: &str = "package main\n\nimport \"fmt\"\n\n// main prints the notes.\nfunc main() {\n\tnotes := []string{\"lurq\", \"code\"}\n\tfor i, note := range notes {\n\t\tfmt.Printf(\"%d: %s\\n\", i, note)\n\t}\n}";

/// Ten thousand lines, made once.
static TEN_THOUSAND: LazyLock<String> = LazyLock::new(|| {
  (1..=10_000)
    .map(|index| format!("fn step_{index}(ctx: &mut Ctx) -> u32 {{ {index} * 2 }} // step {index} of 10 000"))
    .collect::<Vec<_>>()
    .join("\n")
});

/// A scheme of colors of the app's own: warm words, comments that lean.
fn ember() -> CodeScheme {
  CodeScheme::new()
    .color("keyword", Color::from_hex("#ea580c"))
    .style(
      "keyword.control",
      CodeStyle::new().foreground(Color::from_hex("#dc2626")).bold(),
    )
    .color("type", Color::from_hex("#ca8a04"))
    .color("function", Color::from_hex("#0891b2"))
    .color("function.macro", Color::from_hex("#9333ea"))
    .color("attribute", Color::from_hex("#9333ea"))
    .color("string", Color::from_hex("#16a34a"))
    .color("number", Color::from_hex("#db2777"))
    .color("label", Color::from_hex("#db2777"))
    .style(
      "comment",
      CodeStyle::new().foreground(Color::from_hex("#a8a29e")).italic(),
    )
    .color("punctuation", Color::from_hex("#a8a29e"))
    .color("operator", Color::from_hex("#a8a29e"))
}

/// The tokenizers a block is shown by: the language's, or syntect's grammar of the language, where it has one.
const TOKENIZERS: [(usize, &str); 2] = [(0, "lurq"), (1, "syntect")];

/// The schemes of colors a block is shown in.
const SCHEMES: [(usize, &str); 2] = [(0, "the theme's"), (1, "Ember")];

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
fn samples() -> Vec<(&'static str, String, &'static str, &'static str)> {
  vec![
    ("main.rs", MAIN.to_owned(), "rust", "main.rs"),
    ("Cargo.toml", CARGO.to_owned(), "toml", "Cargo.toml"),
    ("a long line", LONG.to_owned(), "rust", ""),
    ("a long file", file(), "rust", "steps.rs"),
    ("note.rs", NOTE.to_owned(), "rust", "note.rs"),
    ("query.sql", QUERY.to_owned(), "sql", "query.sql"),
    ("ci.yml", WORKFLOW.to_owned(), "yaml", "ci.yml"),
    ("notes.py", NOTES_PY.to_owned(), "python", "notes.py"),
    ("app.ts", APP_TS.to_owned(), "typescript", "app.ts"),
    ("page.html", PAGE_HTML.to_owned(), "html", "page.html"),
    ("style.css", STYLE_CSS.to_owned(), "css", "style.css"),
    ("package.json", PACKAGE_JSON.to_owned(), "json", "package.json"),
    ("build.sh", BUILD_SH.to_owned(), "bash", "build.sh"),
    ("README.md", README_MD.to_owned(), "markdown", "README.md"),
    ("main.go", MAIN_GO.to_owned(), "go", "main.go"),
    ("10 000 lines", TEN_THOUSAND.clone(), "rust", "steps.rs"),
  ]
}

/// The sample of ten thousand lines, which the playground shows in a block of a greatest height.
const TEN_THOUSAND_SAMPLE: usize = 15;

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
  /// The playground's tokenizer and scheme: indices of [`TOKENIZERS`] and [`SCHEMES`].
  tokenizer: Signal<usize>,
  scheme: Signal<usize>,
  /// The section "Your own tokenizer"'s.
  own_tokenizer: Signal<usize>,
  own_scheme: Signal<usize>,
}

/// The greatest heights the playground offers.
const HEIGHTS: [(&str, Option<f32>); 3] = [("none", None), ("200", Some(200.0)), ("320", Some(320.0))];

impl CodeBlockPage {
  /// The playground: the block as the panel's props make it, mounted, so that its copy button shows it copied.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let sample = self.sample.get().min(samples().len() - 1);
    let (_, code, ..) = samples()[sample].clone();
    let (language, title) = (self.language.get(), self.title.get());
    let (line_numbers, wrap) = (self.line_numbers.get(), self.wrap.get());
    let (copy_button, selectable) = (self.copy_button.get(), self.selectable.get());
    let highlighted = self.highlighted.get();
    let max_height = HEIGHTS[self.max_height.get().min(HEIGHTS.len() - 1)].1;
    let record = self.playground.recorder();
    let forced = self.playground.forced();
    let (tokenizer, scheme) = (self.tokenizer.get().min(1), self.scheme.get().min(1));
    // syntect's grammar of the language, where it has one: the block's own tokenizer in place of the language's.
    let syntect = (tokenizer == 1).then(|| Syntect::for_language(&language)).flatten();
    let make = {
      let (code, language, title, highlighted) = (code.clone(), language.clone(), title.clone(), highlighted.clone());
      let syntect = syntect.clone();
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
        if let Some(syntect) = &syntect {
          block = block.highlighter(syntect.clone());
        }
        if scheme == 1 {
          block = block.code_scheme(ember());
        }
        block
      }
    };
    let shown = {
      let make = make.clone();
      Shown::built(move |ctx| {
        make()
          .force(forced)
          .on_copy(move |code| record(format!("on_copy({} lines)", code.lines().count())))
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
    if syntect.is_some() {
      code_lines.push(format!("    .highlighter(Syntect::for_language({language:?}))"));
    }
    if scheme == 1 {
      code_lines.push("    .code_scheme(ember())".to_owned());
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
      let height = self.max_height.clone();
      move |index: usize| {
        let (_, _, sample_language, sample_title) = samples()[index].clone();
        sample.set(index);
        language.set(sample_language.to_owned());
        title.set(sample_title.to_owned());
        // Ten thousand lines are shown in a block of a greatest height, which draws the lines in view.
        if index == TEN_THOUSAND_SAMPLE {
          height.set(2);
        }
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
      let (tokenizer, scheme) = (self.tokenizer.clone(), self.scheme.clone());
      let on = [self.copy_button.clone(), self.selectable.clone()];
      let off = [self.line_numbers.clone(), self.wrap.clone()];
      Arc::new(move || {
        set_sample(0);
        highlighted.set(String::new());
        height.set(0);
        tokenizer.set(0);
        scheme.set(0);
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
        facts: match (tokenizer, &syntect) {
          (1, None) => format!(
            "{} lines · syntect has no grammar of {language:?}: the language's",
            code.lines().count()
          ),
          _ => format!("{} lines", code.lines().count()),
        },
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
          divider(),
          prop(
            "highlighter",
            "impl Into<CodeHighlighter>",
            self.playground.select(2, &by_label(&TOKENIZERS), &tokenizer, {
              let chosen = self.tokenizer.clone();
              move |index| chosen.set(index)
            }),
          ),
          prop(
            "code_scheme",
            "CodeScheme",
            self.playground.select(3, &by_label(&SCHEMES), &scheme, {
              let chosen = self.scheme.clone();
              move |index| chosen.set(index)
            }),
          ),
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
          tokenizer != 0,
          scheme != 0,
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
      playground: Playground::new(ctx, 4),
      sample: ctx.signal(0),
      language: ctx.signal("rust".to_owned()),
      title: ctx.signal("main.rs".to_owned()),
      line_numbers: ctx.signal(false),
      highlighted: ctx.signal(String::new()),
      wrap: ctx.signal(false),
      max_height: ctx.signal(0),
      copy_button: ctx.signal(true),
      selectable: ctx.signal(true),
      tokenizer: ctx.signal(0),
      scheme: ctx.signal(0),
      own_tokenizer: ctx.signal(0),
      own_scheme: ctx.signal(0),
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
    let windowed = item(
      CodeBlock::new(&TEN_THOUSAND)
        .language("rust")
        .title("steps.rs")
        .line_numbers(true)
        .max_height(320.0)
        .mount(ctx),
      "10 000 lines · max_height(320)",
    );
    let own = self.own_tokenizer.get().min(1);
    let own_scheme = self.own_scheme.get().min(1);
    let mut note = CodeBlock::new(NOTE).language("rust").title("note.rs");
    if own == 1 {
      note = note.highlighter(syntect_code::rust());
    }
    if own_scheme == 1 {
      note = note.code_scheme(ember());
    }
    let tokenizers = Column::new()
      .width(lurq::node::dimension::Dimension::Pct(100.0))
      .spacing(12.0)
      .child(
        lurq::components::Row::new()
          .width(lurq::node::dimension::Dimension::Pct(100.0))
          .wrap()
          .spacing(12.0)
          .child(SegmentedControl::new(self.own_tokenizer.clone(), TOKENIZERS))
          .child(SegmentedControl::new(self.own_scheme.clone(), SCHEMES)),
      )
      .child(item(
        note.mount(ctx),
        match (own, own_scheme) {
          (0, 0) => "lurq's own tokenizer, the theme's colors",
          (0, _) => "lurq's own tokenizer, Ember",
          (_, 0) => "syntect's grammar of Rust, the theme's colors",
          _ => "syntect's grammar of Rust, Ember",
        },
      ));
    let mut own_code = vec![
      "CodeBlock::new(note_rs)".to_owned(),
      "    .language(\"rust\")".to_owned(),
    ];
    if own == 1 {
      own_code.push("    // src/syntect_code.rs: syntect's scopes as lurq's names".to_owned());
      own_code.push("    .highlighter(Syntect::for_language(\"rust\"))".to_owned());
    }
    if own_scheme == 1 {
      own_code.push("    // ember(): CodeScheme::new().color(\"keyword\", ..).style(\"comment\", ..)".to_owned());
      own_code.push("    .code_scheme(ember())".to_owned());
    }
    own_code.push("    .mount(ctx)".to_owned());
    let go = item(
      CodeBlock::new(MAIN_GO).language("go").title("main.go").mount(ctx),
      "go · syntect's, registered at start",
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
        "Whose highlighter colors its words (lurq's for Rust, TOML, JSON, JavaScript, TypeScript, shells, Python, \
         CSS, HTML, XML, YAML, SQL and Markdown, or the app's), what the bar shows where it has no title, and what \
         tooling is told. A language with none is plain.",
      ),
      (
        "highlighter",
        "impl Into<CodeHighlighter>",
        "the language's",
        "A tokenizer of the block's own in place of its language's: any Fn(&str) -> Vec<CodeToken>, or a \
         CodeLanguage table.",
      ),
      (
        "token_style",
        "name, Fn(PartStyle) -> PartStyle",
        "",
        "Styles the tokens of a name (\"keyword\", \"string.escape\") over the theme's code colors; a name with no \
         style takes its less particular name's.",
      ),
      (
        "code_scheme",
        "CodeScheme",
        "the theme's",
        "The block's own colors of code in place of the theme's.",
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
          "Ten thousand lines",
          "A block of a greatest height with more lines than a few screens lays out and paints the lines in view, \
           and a chunk of lines around them: ten thousand lines cost what a screen of them costs.",
          self.frame.demo(
            4,
            windowed,
            ".max_height(320)",
            &[
              "CodeBlock::new(steps_rs) // 10 000 lines",
              "    .language(\"rust\")",
              "    .title(\"steps.rs\")",
              "    .line_numbers(true)",
              "    .max_height(320)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Your own tokenizer",
          "A highlighter tells a language's tokens apart and names them (keyword, string.escape, function.macro); \
           the theme's code colors draw each name, dropping the tail of a name they have nothing for. The same code \
           by lurq's own tokenizer and by syntect's grammar of Rust, which this gallery plugs in as an app would \
           (src/syntect_code.rs), in the theme's colors or in a scheme of the app's.",
          self.frame.demo(
            5,
            tokenizers,
            ".highlighter(..) · .code_scheme(..)",
            &own_code_refs(&own_code),
          ),
        ),
        Section::new(
          "A language lurq does not know",
          "lurq has no highlighter for Go: the gallery registers syntect's for it at start, and every block of Go \
           is colored by it. Registered for a language lurq knows, an app's highlighter replaces lurq's.",
          self.frame.demo(
            6,
            go,
            "app.register_highlighter(\"go\", ..)",
            &[
              "// At start, once:",
              "app.register_highlighter(\"go\", Syntect::for_language(\"go\").unwrap());",
              "",
              "CodeBlock::new(main_go).language(\"go\").title(\"main.go\").mount(ctx)",
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

/// The lines of a demo's code as the frame takes them.
fn own_code_refs(lines: &[String]) -> Vec<&str> {
  lines.iter().map(String::as_str).collect()
}

/// Choices of a segmented control as a select of the playground takes them: by their labels.
fn by_label<'a>(choices: &[(usize, &'a str)]) -> Vec<(&'a str, usize)> {
  choices.iter().map(|(value, label)| (*label, *value)).collect()
}
