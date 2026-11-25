//! The page of the rich text editor: its playground, the board's document bound to a signal with the Markdown
//! it writes, a comment box with a few tools, a document with pictures in it, a checklist and a divider, its
//! states, and a controller that works it from outside.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{component::Component, ctx::Ctx, events::ClipboardImage},
  components::{Column, InlineObject, ParagraphKind, RichText, Row, TextMark},
  core::Signal,
  images::ImageData,
  layout::Alignment,
  node::Element,
  ui::{
    Button, ButtonSize, ButtonVariant, FontFamily, Picture, RICH_TEXT_MAX_ROWS, RICH_TEXT_MIN_ROWS, RichTextController,
    RichTextEditor, RichTextEditorPart, RichTextTool, icons,
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

/// A document with a picture in a paragraph of its own and one among the words of a line.
const FIELD_NOTES: &str = "## Field notes\n\nThe lake at dusk, from the north shore:\n\n<img src=\"lake\" alt=\"The \
                           lake at dusk\" width=\"320\">\n\nA small one <img src=\"lake\" alt=\"the lake, small\" \
                           width=\"28\"> stands among the words of its line, and the row makes room for it.";

/// A checklist between two sections, and a numbered list.
const TASKS: &str = "### Acceptance\n\n- [x] The brief reads as it is edited\n- [ ] A divider between sections\n- [ ] \
                     Items closer than the blocks round them\n\n---\n\n### Steps\n\n1. Write the brief\n2. Check \
                     the items\n\n```rust\nfn main() {\n    brief::check();\n}\n```";

/// The pictures pasted into the page's editors as pixels: a document names one `pasted-0`, `pasted-1`, by its
/// place here. An app would write them to its files; the gallery has none, and keeps them while it runs.
type Pasted = Arc<Mutex<Vec<ImageData>>>;

/// What the page's editors draw for a picture. The gallery has no files of its own, so its documents name the
/// page's lake or a picture that was pasted as pixels, and the pixels are painted; a picture the document names
/// by a path or an address (a file pasted from a file manager) is that file's, as the editor draws it unless
/// told. An app whose pictures are all files or addresses draws nothing itself.
fn painted(lake: &ImageData, pasted: &Pasted) -> impl Fn(&InlineObject) -> Element + Send + Sync + 'static {
  let (lake, pasted) = (lake.clone(), pasted.clone());
  move |picture| {
    let source = picture.source.as_ref();
    let kept = source
      .strip_prefix("pasted-")
      .and_then(|index| index.parse::<usize>().ok())
      .and_then(|index| pasted.lock().unwrap().get(index).cloned());
    let mut drawn = match kept {
      Some(pixels) => Picture::new(pixels),
      None if source == "lake" => Picture::new(lake.clone()),
      None if source.starts_with("http://") || source.starts_with("https://") => Picture::from_url(source),
      None => Picture::from_file(source),
    }
    .label(&picture.label);
    if let Some(width) = picture.width {
      drawn = drawn.width(width as f32);
    }
    drawn.into()
  }
}

/// What the page's editors do with a picture pasted as pixels, a screenshot: they keep it, and name it.
fn keeps(pasted: &Pasted) -> impl Fn(&ClipboardImage) -> Option<String> + Send + Sync + 'static {
  let pasted = pasted.clone();
  move |image| {
    let mut pasted = pasted.lock().unwrap();
    pasted.push(ImageData::from_rgba(
      image.rgba().to_vec(),
      image.width(),
      image.height(),
    ));
    Some(format!("pasted-{}", pasted.len() - 1))
  }
}

/// The rows an editor can be told to show at least, and at most: the selects of the playground.
const LEAST: [(&str, usize); 4] = [("2", 2), ("4", 4), ("6", 6), ("10", 10)];
const MOST: [(&str, usize); 4] = [("6", 6), ("10", 10), ("16", 16), ("24", 24)];

/// What is on a stage, one under another, as wide as the stage.
/// A paragraph, an empty block, and another paragraph.
fn with_an_empty_block() -> RichText {
  RichText::new(
    "A first paragraph.\n\nA third, after an empty one.",
    vec![],
    vec![ParagraphKind::Body; 3],
  )
}

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
  /// A document with an empty block, where the caret's says the block placeholder.
  blocks: Signal<RichText>,
  /// The same, where every empty block says it.
  every: Signal<RichText>,
  terms: Signal<RichText>,
  /// The document with pictures, what works its editor from outside, and the playground's.
  pictured: Signal<RichText>,
  picturing: RichTextController,
  playing: RichTextController,
  /// The checklist, with its divider, and the spacing a recipe gives its paragraphs.
  tasks: Signal<RichText>,
  /// The page's picture, drawn once: the same pixels each render.
  lake: ImageData,
  pasted: Pasted,
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
      let (playing, lake, pasted) = (self.playing.clone(), self.lake.clone(), self.pasted.clone());
      move || {
        let insert = playing.clone();
        RichTextEditor::new(value.clone())
          // The image tool of the playground puts the page's lake in at the caret, and a screenshot is pasted.
          .controller(&playing)
          .picture(painted(&lake, &pasted))
          .on_paste_image(keeps(&pasted))
          .on_image(move || {
            insert.insert_object(&InlineObject::new(0, "lake").label("The lake at dusk").width(240));
          })
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
      "    .controller(&controller)".to_owned(),
      "    .on_image(move || { controller.insert_picture(\"photos/lake.jpg\", \"The lake at dusk\"); })".to_owned(),
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
    let shown = self.playground.styled(plain().force(self.playground.forced()));
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
      blocks: ctx.signal(with_an_empty_block()),
      every: ctx.signal(with_an_empty_block()),
      terms: ctx.signal(RichText::from_markdown(
        "### Terms\n\nBy using lurq you agree to *build something*.",
      )),
      pictured: ctx.signal(RichText::from_markdown(FIELD_NOTES)),
      tasks: ctx.signal(RichText::from_markdown(TASKS)),
      picturing: RichTextController::new(),
      playing: RichTextController::new(),
      lake: super::picture::lake(640, 480),
      pasted: Pasted::default(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    // The board's document, and the Markdown the app would store for it.
    let markdown: Vec<String> = self.notes.get().to_markdown().lines().map(str::to_owned).collect();
    let live = stack()
      .child(
        RichTextEditor::new(self.notes.clone())
          .label("Release notes")
          // Every editor of the page takes a pasted picture, and draws it.
          .picture(painted(&self.lake, &self.pasted))
          .on_paste_image(keeps(&self.pasted))
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
          .picture(painted(&self.lake, &self.pasted))
          .on_paste_image(keeps(&self.pasted))
          .key("comment")
          .mount(ctx),
      )
      .child(quiet(
        &format!("{} words, {} marked runs", comment.word_count(), comment.spans.len()),
        12.0,
      ));

    // Pictures in the document: one alone in its paragraph, one among words, and a tool that puts another in.
    let pictured: Vec<String> = self.pictured.get().to_markdown().lines().map(str::to_owned).collect();
    let insert = self.picturing.clone();
    let pictures = stack()
      .child(
        RichTextEditor::new(self.pictured.clone())
          .label("Field notes")
          .controller(&self.picturing)
          .picture(painted(&self.lake, &self.pasted))
          .on_paste_image(keeps(&self.pasted))
          .on_image(move || {
            insert.insert_object(&InlineObject::new(0, "lake").label("The lake at dusk").width(160));
          })
          .rows(6, 20)
          .key("pictured")
          .mount(ctx),
      )
      .child(
        Column::new()
          .width(560.0)
          .max_width(FILL)
          .spacing(8.0)
          .child(quiet("document.to_markdown()", 12.0))
          .child(plain_block(&pictured)),
      );

    // A checklist and a divider, with the tools that put them in, and a recipe that spaces the paragraphs as a
    // document of Markdown is: items 6 apart, the numbers in the mono face, code in a sentence padded.
    let tasks: Vec<String> = self.tasks.get().to_markdown().lines().map(str::to_owned).collect();
    let checklists = stack()
      .child(
        RichTextEditor::new(self.tasks.clone())
          .label("Acceptance")
          .tools([
            RichTextTool::Style,
            RichTextTool::Bold,
            RichTextTool::Bullet,
            RichTextTool::Ordered,
            RichTextTool::Checklist,
            RichTextTool::Divider,
            RichTextTool::Code,
            RichTextTool::Undo,
            RichTextTool::Redo,
          ])
          .style(RichTextEditorPart::Bullet, |s| s.gap(6.0))
          .style(RichTextEditorPart::Ordered, |s| s.gap(6.0))
          .style(RichTextEditorPart::Checklist, |s| s.gap(6.0))
          .style(RichTextEditorPart::OrderedMark, |s| s.font_family(FontFamily::Mono))
          .style(RichTextEditorPart::InlineCode, |s| s.padding_x(4.0))
          .rows(6, 20)
          .key("tasks")
          .mount(ctx),
      )
      .child(
        Column::new()
          .width(560.0)
          .max_width(FILL)
          .spacing(8.0)
          .child(quiet("document.to_markdown()", 12.0))
          .child(plain_block(&tasks)),
      );

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

    // An empty block says what a person can do in it: the one the caret is in, or each of them.
    let blocks = stack()
      .child(
        RichTextEditor::new(self.blocks.clone())
          .block_placeholder("Type / for blocks")
          .toolbar(false)
          .rows(4, 8)
          .key("blocks")
          .mount(ctx),
      )
      .child(quiet(
        "the block the caret is in: put it on the empty line, or press Enter",
        12.0,
      ))
      .child(
        RichTextEditor::new(self.every.clone())
          .block_placeholder("Type / for blocks")
          .block_placeholder_in_every_empty_block(true)
          .toolbar(false)
          .rows(4, 8)
          .key("every")
          .mount(ctx),
      )
      .child(quiet("every empty block, with the focus or without it", 12.0));

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
          .picture(painted(&self.lake, &self.pasted))
          .on_paste_image(keeps(&self.pasted))
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
        "keep_selection",
        "bool",
        "false",
        "Its selection stays painted, at half its opacity, while another element has the focus: a toolbar of the \
         app's over the selected words.",
      ),
      (
        "tools",
        "impl IntoIterator<Item = RichTextTool>",
        "all but Image, Checklist, Divider",
        "The tools of the bar, in order; a line stands between two groups, and the history is at the end.          Checklist and Divider are the app's to name.",
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
        "code_bar_label",
        "impl Fn(&str) -> String",
        "the fence's word",
        "What the bar over a block of code says for its language: Bash for `bash`. The document keeps the word.",
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
        "What the image tool does; with it the bar has the tool. The app asks for a picture and puts it in with \
         the controller's `insert_picture(source, label)`.",
      ),
      (
        "on_paste_image",
        "impl Fn(&ClipboardImage) -> Option<String>",
        "",
        "What is done with a picture pasted as pixels, a screenshot: the app keeps the pixels and answers with the \
         source the document names them by. Without it a paste of pixels does nothing.",
      ),
      (
        "picture",
        "impl Fn(&InlineObject) -> Element",
        "a Picture of the source",
        "What is drawn for a picture of the document: given its source, its label and its width. Unless told, a \
         `Picture` of the file or the address the source names.",
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
        "\"{n} words\"",
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
           what the selection is. Under the editor is the Markdown the app would store for it. A copy keeps the \
           formatting: the clipboard is given the Markdown, and a paste of Markdown into an editor, of this app or \
           of another, comes back with its marks and kinds. Ctrl+K in the text is a link, not the gallery's search.",
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
          "Pictures",
          "A picture is a character of the document: the caret goes past it, a press selects it, Backspace after \
           it selects it and the next deletes it, and it is copied and pasted with the words around it. A \
           selected picture has a handle on its corner: drag it to make the picture wider or narrower. A \
           screenshot pasted here goes in as a picture too: the page keeps its pixels while it runs, where an app \
           would write a file. One alone \
           in its paragraph and one among words are both in rows of the text, each row as high as it needs. The \
           image tool runs the app's handler, which puts a picture in at the caret; undo takes it back. Markdown \
           writes a picture as an image, and as a tag where it has a width.",
          self.frame.demo(
            4,
            pictures,
            "controller.insert_picture(source, label)",
            &[
              "// controller: RichTextController, kept by the page",
              "RichTextEditor::new(notes.clone())",
              "    .controller(&controller)",
              "    // The app asks for a file, and puts the one chosen in at the caret.",
              "    .on_image(move || { controller.insert_picture(\"photos/lake.jpg\", \"The lake at dusk\"); })",
              "    .mount(ctx)",
              "",
              "// A screenshot pasted: the app keeps the pixels and says what the document calls them.",
              "RichTextEditor::new(notes.clone())",
              "    .on_paste_image(|image| { let path = save(image.to_png().ok()?)?; Some(path) })",
              "",
              "// A picture that is no file and no address is drawn by the app.",
              "RichTextEditor::new(notes.clone())",
              "    .picture(|picture| Picture::from_resource(&picture.source).label(&picture.label).into())",
              "",
              "// What the app stores.",
              "notes.get().to_markdown()",
            ],
          ),
        ),
        Section::new(
          "Checklists and dividers",
          "An item of a checklist has a box before it: a press on the box checks it, and so does Ctrl+Enter in the \
           item. `[ ] ` at the start of a paragraph makes it an item, `[x] ` a checked one, and three hyphens and \
           Enter a divider. The tools of a checklist and of a divider are not on the bar unless the app names them; \
           an app's `/` menu runs `set_paragraphs` and `insert_divider`. Markdown writes them as `- [x]` and `---`. \
           Here a recipe also puts the items of a list 6 apart, closer than the blocks round them, the numbers in \
           the mono face, and room round code in a sentence.",
          self.frame.demo(
            5,
            checklists,
            ".tools([.., RichTextTool::Checklist, RichTextTool::Divider])",
            &[
              "RichTextEditor::new(tasks.clone())",
              "    .tools([",
              "        RichTextTool::Style,",
              "        RichTextTool::Bullet,",
              "        RichTextTool::Checklist,",
              "        RichTextTool::Divider,",
              "        RichTextTool::Undo,",
              "    ])",
              "    // The items of a list 6 apart, the numbers in the mono face.",
              "    .style(RichTextEditorPart::Bullet, |s| s.gap(6.0))",
              "    .style(RichTextEditorPart::Checklist, |s| s.gap(6.0))",
              "    .style(RichTextEditorPart::OrderedMark, |s| s.font_family(FontFamily::Mono))",
              "    .style(RichTextEditorPart::InlineCode, |s| s.padding_x(4.0))",
              "    .mount(ctx)",
              "",
              "// A `/` menu of the app.",
              "controller.set_paragraphs(ParagraphKind::Checklist { checked: false });",
              "controller.insert_divider();",
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
          "Empty blocks",
          "A block placeholder is what an empty block says, as a block editor draws it: in the block the caret is \
           in, in that block's type and in the placeholder's color, while the editor has the focus and nothing is \
           selected. It is drawn over the text: the block keeps its height and the caret its place. Told, every \
           empty block says it. An empty document says its own placeholder in its place.",
          self.frame.demo(
            6,
            blocks,
            ".block_placeholder(\"Type / for blocks\")",
            &[
              "RichTextEditor::new(page.clone())",
              "    .block_placeholder(\"Type / for blocks\")",
              "    .mount(ctx)",
              "",
              "RichTextEditor::new(page.clone())",
              "    .block_placeholder(\"Type / for blocks\")",
              "    .block_placeholder_in_every_empty_block(true)",
              "    .mount(ctx)",
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
