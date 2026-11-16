//! The page of the rich text editor: its playground, the board's document bound to a signal with the Markdown
//! it writes, a comment box with a few tools, its states, and a controller that works it from outside.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, ParagraphKind, RichText, Row, TextMark},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{
    Button, ButtonSize, ButtonVariant, RICH_TEXT_MAX_ROWS, RICH_TEXT_MIN_ROWS, RichTextController, RichTextEditor,
    RichTextTool, icons,
  },
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, plain_block, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// The board's document.
const NOTES: &str = "## Release notes\n\nBackdrop blur now **follows rounded clips** and transforms.\n\n- Faster text \
                     shaping\n- Fixes for window teardown";

/// The rows an editor can be told to show at least, and at most: the selects of the playground.
const LEAST: [(&str, usize); 4] = [("2", 2), ("4", 4), ("6", 6), ("10", 10)];
const MOST: [(&str, usize); 4] = [("6", 6), ("10", 10), ("16", 16), ("24", 24)];

/// What is on a stage, one under another, as wide as the stage.
fn stack() -> Column {
  Column::new().width(FILL).spacing(16.0).align_items(Alignment::Start)
}

pub struct RichTextEditorPage {
  frame: Frame,
  playground: Playground,
  value: Signal<RichText>,
  placeholder: Signal<String>,
  min_rows: Signal<usize>,
  max_rows: Signal<usize>,
  toolbar: Signal<bool>,
  footer: Signal<bool>,
  shortcuts: Signal<bool>,
  invalid: Signal<bool>,
  read_only: Signal<bool>,
  disabled: Signal<bool>,
  full_width: Signal<bool>,
  notes: Signal<RichText>,
  comment: Signal<RichText>,
  driven: Signal<RichText>,
  controller: RichTextController,
  /// The documents of the states: one empty, one the three other editors share.
  empty: Signal<RichText>,
  terms: Signal<RichText>,
}

impl RichTextEditorPage {
  /// The playground: the editor as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let placeholder = self.placeholder.get();
    let (min_rows, max_rows) = (self.min_rows.get(), self.max_rows.get());
    let (toolbar, footer, shortcuts) = (self.toolbar.get(), self.footer.get(), self.shortcuts.get());
    let flags = [
      ("invalid", self.invalid.get()),
      ("read_only", self.read_only.get()),
      ("disabled", self.disabled.get()),
      ("full_width", self.full_width.get()),
    ];
    let plain = {
      let (value, placeholder) = (self.value.clone(), placeholder.clone());
      let set: Vec<bool> = flags.iter().map(|(_, set)| *set).collect();
      move || {
        RichTextEditor::new(value.clone())
          .placeholder(&placeholder)
          .rows(min_rows, max_rows)
          .toolbar(toolbar)
          .footer(footer)
          .markdown_shortcuts(shortcuts)
          .invalid(set[0])
          .read_only(set[1])
          .disabled(set[2])
          .full_width(set[3])
      }
    };
    self.playground.watch(ctx, &self.value, |document| {
      format!("on_change({} words)", document.word_count())
    });
    {
      let plain = plain.clone();
      self.playground.window_shows(move |ctx| plain().mount(ctx));
    }

    let mut code = vec![
      "// value: Signal<RichText>".to_owned(),
      "RichTextEditor::new(value.clone())".to_owned(),
      format!("    .placeholder({placeholder:?})"),
    ];
    if (min_rows, max_rows) != (RICH_TEXT_MIN_ROWS, RICH_TEXT_MAX_ROWS) {
      code.push(format!("    .rows({min_rows}, {max_rows})"));
    }
    for (name, on) in [
      ("toolbar", toolbar),
      ("footer", footer),
      ("markdown_shortcuts", shortcuts),
    ] {
      if !on {
        code.push(format!("    .{name}(false)"));
      }
    }
    for (flag, set) in flags {
      if set {
        code.push(format!("    .{flag}(true)"));
      }
    }
    code.push("    .mount(ctx)".to_owned());

    let held = self.value.get();
    let tree = vec![
      format!("root        {}", self.playground.measured()),
      "├─ toolbar   40 · muted · tools of 28, icons of 15".to_owned(),
      format!(
        "├─ content   {} paragraphs · body 14 on 1.3 · 10 between",
        held.paragraphs.len()
      ),
      format!("└─ footer    28 · {} words", held.word_count()),
    ];

    let (to_value, to_placeholder) = (self.value.clone(), self.placeholder.clone());
    let (to_min, to_max) = (self.min_rows.clone(), self.max_rows.clone());
    let on = [self.toolbar.clone(), self.footer.clone(), self.shortcuts.clone()];
    let off = [
      self.invalid.clone(),
      self.read_only.clone(),
      self.disabled.clone(),
      self.full_width.clone(),
    ];
    let (min, max) = (self.min_rows.clone(), self.max_rows.clone());
    let shown = plain().force(self.playground.forced());
    let make = {
      let plain = plain.clone();
      move || plain().into()
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Rich Text Editor",
        make: Arc::new(make),
        shown: Shown::built(move |ctx| shown.mount(ctx)),
        // As wide as the stage has room for, up to its own 560: a stage narrower than that holds it whole.
        fill: true,
        forceable: false,
        facts: format!("{} words", held.word_count()),
        event: "on_change",
        code,
        tree,
        controls: vec![
          prop("placeholder", "&str", field(&self.placeholder)),
          prop(
            "rows: least",
            "usize",
            self.playground.select(0, &LEAST, &min_rows, move |rows| min.set(rows)),
          ),
          prop(
            "rows: most",
            "usize",
            self.playground.select(1, &MOST, &max_rows, move |rows| max.set(rows)),
          ),
          divider(),
          flag("toolbar", &self.toolbar),
          flag("footer", &self.footer),
          flag("markdown_shortcuts", &self.shortcuts),
          divider(),
          flag("invalid", &self.invalid),
          flag("read_only", &self.read_only),
          flag("disabled", &self.disabled),
          flag("full_width", &self.full_width),
        ],
        changed: [
          held != RichText::from_markdown(NOTES),
          placeholder != "Write the release notes",
          min_rows != RICH_TEXT_MIN_ROWS,
          max_rows != RICH_TEXT_MAX_ROWS,
          !toolbar,
          !footer,
          !shortcuts,
        ]
        .into_iter()
        .chain(flags.iter().map(|(_, set)| *set))
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_value.set(RichText::from_markdown(NOTES));
          to_placeholder.set("Write the release notes".to_owned());
          to_min.set(RICH_TEXT_MIN_ROWS);
          to_max.set(RICH_TEXT_MAX_ROWS);
          for flag in &on {
            flag.set(true);
          }
          for flag in &off {
            flag.set(false);
          }
        }),
      },
    )
  }
}

impl Component for RichTextEditorPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      value: ctx.signal(RichText::from_markdown(NOTES)),
      placeholder: ctx.signal("Write the release notes".to_owned()),
      min_rows: ctx.signal(RICH_TEXT_MIN_ROWS),
      max_rows: ctx.signal(RICH_TEXT_MAX_ROWS),
      toolbar: ctx.signal(true),
      footer: ctx.signal(true),
      shortcuts: ctx.signal(true),
      invalid: ctx.signal(false),
      read_only: ctx.signal(false),
      disabled: ctx.signal(false),
      full_width: ctx.signal(false),
      notes: ctx.signal(RichText::from_markdown(NOTES)),
      comment: ctx.signal(RichText::default()),
      driven: ctx.signal(RichText::plain("Select a word and press a button under the editor.")),
      controller: RichTextController::new(),
      empty: ctx.signal(RichText::default()),
      terms: ctx.signal(RichText::from_markdown(
        "### Terms\n\nBy using lurq you agree to *build something*.",
      )),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    // The board's document, and the Markdown the app would store for it.
    let markdown: Vec<String> = self.notes.get().to_markdown().lines().map(str::to_owned).collect();
    let live = stack()
      .child(
        RichTextEditor::new(self.notes.clone())
          .label("Release notes")
          .mount(ctx),
      )
      .child(
        Column::new()
          .width(560.0)
          .max_width(FILL)
          .spacing(8.0)
          .child(quiet("document.to_markdown()", 12.0))
          .child(plain_block(&markdown)),
      );

    // A comment box: the marks, a list and a link, and no footer.
    let comment = self.comment.get();
    let comments = stack()
      .child(
        RichTextEditor::new(self.comment.clone())
          .placeholder("Leave a comment")
          .tools([
            RichTextTool::Bold,
            RichTextTool::Italic,
            RichTextTool::Code,
            RichTextTool::Bullet,
            RichTextTool::Link,
          ])
          .footer(false)
          .rows(3, 8)
          .key("comment")
          .mount(ctx),
      )
      .child(quiet(
        &format!("{} words, {} marked runs", comment.word_count(), comment.spans.len()),
        12.0,
      ));

    let fixed = || self.terms.clone();
    let states = stack()
      .child(
        RichTextEditor::new(self.empty.clone())
          .placeholder("Write the release notes")
          .rows(2, 6)
          .key("empty")
          .mount(ctx),
      )
      .child(quiet("empty: its placeholder", 12.0))
      .child(
        RichTextEditor::new(fixed())
          .read_only(true)
          .rows(2, 6)
          .key("read-only")
          .mount(ctx),
      )
      .child(quiet("read only", 12.0))
      .child(
        RichTextEditor::new(fixed())
          .invalid(true)
          .rows(2, 6)
          .key("invalid")
          .mount(ctx),
      )
      .child(quiet("invalid", 12.0))
      .child(
        RichTextEditor::new(fixed())
          .disabled(true)
          .rows(2, 6)
          .key("disabled")
          .mount(ctx),
      )
      .child(quiet("disabled", 12.0));

    // Worked from outside: the app's buttons run the commands a tool runs.
    let command = |label: &str, icon: &'static str, run: fn(&RichTextController)| {
      let controller = self.controller.clone();
      Button::new(label)
        .icon(icon)
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Sm)
        .focuses(&controller.input().element_ref())
        .on_click(move |_| run(&controller))
    };
    let driven = stack()
      .child(
        RichTextEditor::new(self.driven.clone())
          .controller(&self.controller)
          .toolbar(false)
          .rows(3, 8)
          .key("driven")
          .mount(ctx),
      )
      .child(
        Row::new()
          .width(FILL)
          .spacing(8.0)
          .wrap()
          .child(command("Bold", icons::BOLD, |editor| {
            editor.toggle_mark(TextMark::Bold);
          }))
          .child(command("Heading", icons::TEXT_CURSOR, |editor| {
            editor.toggle_paragraphs(ParagraphKind::Heading(2));
          }))
          .child(command("Link to the docs", icons::LINK, |editor| {
            editor.insert_link("the docs", "https://ivanzaida.github.io/lurq/");
          }))
          .child(command("Undo", icons::UNDO, |editor| {
            editor.undo();
          })),
      );

    let props = [
      (
        "value",
        "Signal<RichText>",
        "",
        "The document: its text, the marks over it and what each paragraph is. `controlled(value, on_change)` \
         for one the app holds.",
      ),
      (
        "on_change",
        "impl Fn(&RichText)",
        "",
        "Called with the document after each change a person makes, the signal set first. Not when the app sets it.",
      ),
      (
        "placeholder",
        "&str",
        "",
        "What the box shows while it holds nothing, in the muted foreground.",
      ),
      (
        "rows",
        "(usize, usize)",
        "6, 16",
        "The rows of body text the box shows at least, and at most before its text scrolls inside it.",
      ),
      (
        "tools",
        "impl IntoIterator<Item = RichTextTool>",
        "all but Image",
        "The tools of the bar, in order; a line stands between two groups, and the history is at the end.",
      ),
      (
        "toolbar",
        "bool",
        "true",
        "Whether the editor has its bar. Without it the keys still format.",
      ),
      (
        "footer",
        "bool",
        "true",
        "Whether the editor has the row that says Markdown shortcuts are on and counts the words.",
      ),
      (
        "markdown_shortcuts",
        "bool",
        "true",
        "Whether `## `, `- `, `1. `, `> ` and `**bold**` become what they say as they are typed.",
      ),
      (
        "on_link_click",
        "impl Fn(&str)",
        "",
        "Called with a link's target when it is clicked with Ctrl held (Cmd on a Mac): the app opens it.",
      ),
      (
        "on_image",
        "impl Fn()",
        "",
        "What the image tool does; with it the bar has the tool. The editor draws no picture in its text.",
      ),
      (
        "controller",
        "&RichTextController",
        "",
        "The app's handle on the editor: the commands of its tools, run from outside it.",
      ),
      (
        "invalid",
        "bool",
        "false",
        "What it holds is wrong: its edge is the destructive color. Also `warning` and `valid`.",
      ),
      (
        "read_only",
        "bool",
        "false",
        "It can be focused, selected and copied, and not changed: its tools take no input.",
      ),
      ("disabled", "bool", "false", "Half the opacity; it takes no input."),
      (
        "full_width",
        "bool",
        "false",
        "As wide as what holds it, in place of 560.",
      ),
      (
        "label",
        "&str",
        "",
        "What it is called, to what reads the tree. Each tool's name is `bold_label`, `link_label` and so on.",
      ),
      (
        "name",
        "impl Into<Arc<str>>",
        "",
        "The name its document has in the form it is in: its value there is the document as Markdown.",
      ),
      (
        "count_text",
        "impl Fn(usize) -> String",
        "\"18 words\"",
        "What the footer says of the count of words. `shortcuts_text` is what it says of the shortcuts.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Rich Text Editor"],
        title: "Rich Text Editor",
        description: "Formatted text typed into a box, under a bar of the tools that format it: headings, lists, \
                      quotes and code, and bold, italic, underlined, struck, code and linked words. Its value reads \
                      from Markdown and writes it.",
        import: Some("use lurq::{components::RichText, ui::RichTextEditor};"),
        status: Some("New"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of RichTextEditor is a control on the right. The editor on the stage is live: select words \
           and press a tool or Ctrl+B, type `## ` at the start of a paragraph, `**bold**` in the middle of one.",
          playground,
        ),
        Section::new(
          "Rich Text Editor",
          "The board's document, bound to a signal of the page. A tool works on the selection, and the bar shows \
           what the selection is. Under the editor is the Markdown the app would store for it.",
          self.frame.demo(
            0,
            live,
            "RichTextEditor::new(notes.clone()).mount(ctx)",
            &[
              "// notes: Signal<RichText>, kept by the page",
              "let notes = ctx.signal(RichText::from_markdown(",
              "    \"## Release notes\\n\\nBackdrop blur now **follows rounded clips** and transforms.\\n\\n\\",
              "     - Faster text shaping\\n- Fixes for window teardown\",",
              "));",
              "RichTextEditor::new(notes.clone()).label(\"Release notes\").mount(ctx)",
              "",
              "// What the app stores.",
              "notes.get().to_markdown()",
            ],
          ),
        ),
        Section::new(
          "Tools",
          "The app names the tools of the bar. A comment box has the marks, a list and a link, and no footer.",
          self.frame.demo(
            1,
            comments,
            ".tools([RichTextTool::Bold, ..]).footer(false)",
            &[
              "RichTextEditor::new(comment.clone())",
              "    .placeholder(\"Leave a comment\")",
              "    .tools([",
              "        RichTextTool::Bold,",
              "        RichTextTool::Italic,",
              "        RichTextTool::Code,",
              "        RichTextTool::Bullet,",
              "        RichTextTool::Link,",
              "    ])",
              "    .footer(false)",
              "    .rows(3, 8)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "States",
          "Empty, the box shows its placeholder. Read only, it is on the muted fill, takes the focus and a \
           selection, and its tools take no input. A judgment is the color of its edge. Disabled, it is dimmed.",
          self.frame.demo(
            2,
            states,
            ".read_only(true)",
            &[
              "RichTextEditor::new(empty.clone()).placeholder(\"Write the release notes\").mount(ctx)",
              "RichTextEditor::new(terms.clone()).read_only(true).mount(ctx)",
              "RichTextEditor::new(terms.clone()).invalid(true).mount(ctx)",
              "RichTextEditor::new(terms.clone()).disabled(true).mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Controller",
          "An app works the editor from outside with a controller it keeps: here the editor has no bar, and the \
           page's own buttons run the commands. A button that `focuses` the editor's text leaves the caret in it.",
          self.frame.demo(
            3,
            driven,
            ".controller(&controller)",
            &[
              "// controller: RichTextController, kept by the page",
              "RichTextEditor::new(value.clone())",
              "    .controller(&controller)",
              "    .toolbar(false)",
              "    .mount(ctx)",
              "",
              "Button::new(\"Bold\")",
              "    .focuses(&controller.input().element_ref())",
              "    .on_click(move |_| { controller.toggle_mark(TextMark::Bold); })",
              "Button::new(\"Link to the docs\")",
              "    .on_click(move |_| { controller.insert_link(\"the docs\", \"https://ivanzaida.github.io/lurq/\"); })",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a rich text editor is given.",
          props_table(&props, &["value"], self.frame.room()),
        ),
      ],
    )
  }
}
