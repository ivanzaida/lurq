//! The page of Markdown: its playground with the source beside the document,
//! the board's document of two pages, and each block on its own.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Markdown, MarkdownProps, Row},
  core::Signal,
  layout::{Alignment, text_style::TextStyle},
  node::Element,
  ui::Textarea,
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table},
  playground::{Playground, Shown, Spec, divider, flag},
};

/// The board's document, its first page.
const PAGE_ONE: &str = "# Blur and frosted glass

A node can blur itself, or blur whatever is painted behind it. Both follow transforms and rounded clips.

Text can be **bold**, *italic*, ~~struck~~ or a [link](https://lurq.dev \"lurq.dev\").

## Getting started

1. Add the blur feature flag
2. Set a radius on the node
3. Run the demo

### What it supports

- Backdrop blur
  - Follows rounded clips
  - Follows transforms
- Subtree blur

- [x] Blur a node and its subtree
- [x] Blur what is painted behind
- [ ] Animate the radius

> A design system is a set of decisions you no longer have to make.

### Code

Call `ctx.theme()` during render.

```rust
fn main() {
  let app = App::new();
  app.theme().set_mode(Mode::Dark);
}
```
";

/// The board's document, its second page.
const PAGE_TWO: &str = "#### Heading 4

##### Heading 5

###### Heading 6

| Property | Type | Default |
| --- | --- | ---: |
| radius | f32 | 0.0 |
| backdrop | bool | false |
| tint | Color | none |

---

![A frosted panel over a photo](photo.png \"Figure 1. Frosted glass over a photo.\")

> [!NOTE]
> Blur is skipped when the radius is zero.

> [!TIP]
> Use backdrop blur for panels over busy content.

> [!WARNING]
> Large radii cost more on integrated GPUs.

> [!CAUTION]
> Blurring a video node disables direct scanout.

<details open>
<summary>Platform notes</summary>

On DirectX 12 the blur runs in two passes.

</details>

Press <kbd>Ctrl</kbd> <kbd>K</kbd> to search the docs.

Frame time stayed under 7 ms[^1], with a radius of 24[^2].

[^1]: Measured on a 4K display at 144 Hz.
[^2]: Radius is in logical pixels.
";

/// Where the tasks are among the blocks: the one the page writes a task's change into.
const TASKS: usize = 4;

/// Each block on its own: a title, a word of guidance, and its source.
const BLOCKS: [(&str, &str, &str); 11] = [
  (
    "Headings",
    "The theme's type scale: 30 bold, 24 with a line under it, 20, 16, 14, and 12 in capitals in the muted \
     foreground.",
    "# Heading 1\n\n## Heading 2\n\n### Heading 3\n\n#### Heading 4\n\n##### Heading 5\n\n###### Heading 6\n",
  ),
  (
    "Quotes",
    "`ui::Blockquote`: a line at its start, its words in italics in the muted foreground.",
    "> Blur is a tool for hierarchy, not decoration. Use it where depth helps the reader.\n",
  ),
  (
    "Code blocks",
    "`ui::CodeBlock`: a bar named by the language with the button that copies the code, the words of the language \
     in their colors; a line wider than the block scrolls sideways.",
    "```rust\nlet panel = Column::new()\n  .backdrop_blur(24.0) // the radius, in logical pixels\n  \
     .background(\"#ffffff80\");\n```\n",
  ),
  (
    "Lists",
    "Numbers at the end of a column of 18 in the muted foreground; bullets of 5, a dot at the first level and a \
     ring under it.",
    "1. Add the blur feature flag\n2. Set a radius on the node\n3. Run the demo\n\n- Backdrop blur\n  - Follows \
     rounded clips\n  - Follows transforms\n- Subtree blur\n",
  ),
  (
    "Tasks",
    "The checkbox's look: read only, unless the app takes the change of a task (`on_task_change`) and writes it \
     into its source, as this playground does. A task that is done is in the muted foreground.",
    "- [x] Blur a node and its subtree\n- [x] Blur what is painted behind\n- [ ] Animate the radius\n",
  ),
  (
    "Tables",
    "A box of the middle radius, its header on the muted fill, every other row striped, a line between the \
     columns, each column aligned as its delimiter row says.",
    "| Property | Type | Default |\n| --- | :---: | ---: |\n| radius | f32 | 0.0 |\n| backdrop | bool | false |\n| \
     tint | Color | none |\n",
  ),
  (
    "The rule and an image",
    "A line of 1 in the border's color; an image that stands alone with its caption under it, here the box of its \
     place.",
    "Above the rule.\n\n---\n\n![Frosted glass over a photo](photo.png \"Figure 1. Frosted glass over a photo.\")\n",
  ),
  (
    "Admonitions",
    "GitHub's notes, `> [!NOTE]` and the rest: a callout with a bar, in the tone of its kind.",
    "> [!NOTE]\n> Blur is skipped when the radius is zero.\n\n> [!TIP]\n> Use backdrop blur for panels over busy \
     content.\n\n> [!IMPORTANT]\n> The radius is in logical pixels.\n\n> [!WARNING]\n> Large radii cost more on \
     integrated GPUs.\n\n> [!CAUTION]\n> Blurring a video node disables direct scanout.\n",
  ),
  (
    "Details",
    "A summary that opens and closes what it holds: `<details>` and `<summary>`, open to begin with where it says \
     `open`.",
    "<details>\n<summary>Platform notes</summary>\n\nOn DirectX 12 the blur runs in two passes.\n\n</details>\n",
  ),
  (
    "Footnotes",
    "A reference is its number, in the info color, and goes to its footnote; the footnotes stand at the end under \
     a short rule, each with the way back.",
    "Frame time stayed under 7 ms[^1], with a radius of 24[^2].\n\n[^1]: Measured on a 4K display at 144 \
     Hz.\n[^2]: Radius is in logical pixels.\n",
  ),
  (
    "In a sentence",
    "Code on the inline code's box, keys in the key's, struck words in the muted foreground, a link in the info \
     color.",
    "Call `ctx.theme()` during render. Press <kbd>Ctrl</kbd> <kbd>K</kbd> to search. Text can be **bold**, \
     *italic*, ~~struck~~ or a [link](https://lurq.dev).\n",
  ),
];

/// The design's text of a document: 15 at a line height of 1.65.
fn prose() -> TextStyle {
  TextStyle {
    font_size: 15.0,
    line_height: 1.65,
    ..TextStyle::default()
  }
}

/// `source` with its task at `index` checked or not: what an app writes back of a task's change.
fn with_task(source: &str, index: usize, checked: bool) -> String {
  let mut seen = 0;
  let mut lines: Vec<String> = Vec::new();
  for line in source.lines() {
    let trimmed = line.trim_start();
    let marker = ["- [ ] ", "- [x] ", "- [X] ", "* [ ] ", "* [x] "]
      .iter()
      .find(|marker| trimmed.starts_with(*marker));
    if let Some(marker) = marker {
      if seen == index {
        let indent = &line[..line.len() - trimmed.len()];
        let rest = &trimmed[marker.len()..];
        lines.push(format!("{indent}- [{}] {rest}", if checked { "x" } else { " " }));
        seen += 1;
        continue;
      }
      seen += 1;
    }
    lines.push(line.to_owned());
  }
  let mut out = lines.join("\n");
  if source.ends_with('\n') {
    out.push('\n');
  }
  out
}

pub struct MarkdownPage {
  frame: Frame,
  playground: Playground,
  source: Signal<String>,
  tasks: Signal<String>,
  selectable: Signal<bool>,
  fit_content: Signal<bool>,
  take_tasks: Signal<bool>,
}

impl MarkdownPage {
  /// The playground: the source beside the document it draws.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (selectable, fit_content, take_tasks) = (self.selectable.get(), self.fit_content.get(), self.take_tasks.get());
    let record = self.playground.recorder();
    let source = self.source.clone();
    let props = {
      let source = source.clone();
      move |text: String| {
        let mut props = MarkdownProps::styled(text, prose())
          .selectable(selectable)
          .fit_content(fit_content);
        if take_tasks {
          let (source, record) = (source.clone(), record.clone());
          props = props.on_task_change(move |index, checked| {
            source.update(|text| *text = with_task(text, index, checked));
            record(format!("on_task_change({index}, {checked})"));
          });
        }
        props
      }
    };
    // The source over the document: the document has the stage's whole width, as wide as a page of the board.
    // Both are built on the stage, in its theme, which is not the page's while the stage is in the other appearance.
    let scene = {
      let (props, source) = (props.clone(), source.clone());
      move |ctx: &mut Ctx| -> Element {
        let document = Markdown::mount(ctx, props(source.get()));
        Column::new()
          .width(FILL)
          .spacing(24.0)
          .align_items(Alignment::Stretch)
          .child(
            Textarea::new(source.clone())
              .label("Source")
              .rows(8, 14)
              .full_width(true)
              .style(lurq::ui::TextareaPart::Control, |s| {
                s.font_family(lurq::ui::FontFamily::Mono).font_size(12.0)
              }),
          )
          .child(Column::new().width(FILL).min_width(0.0).child(document))
          .into()
      }
    };

    let mut code = vec!["Markdown::mount(ctx, MarkdownProps::styled(source, prose)".to_owned()];
    if selectable {
      code.push("    .selectable(true)".to_owned());
    }
    if fit_content {
      code.push("    .fit_content(true)".to_owned());
    }
    if take_tasks {
      code.push("    .on_task_change(|index, checked| write_task(index, checked))".to_owned());
    }
    code.push(")".to_owned());
    let tree = vec![
      "document     column · gap 16 · text 15 / 1.65".to_owned(),
      "├─ heading   30/700 · 24/600 with a line · 20 · 16 · 14 · 12 muted".to_owned(),
      "├─ list      gap 6 · number 18 · bullet 5 · task: checkbox".to_owned(),
      "├─ table     radius md · header muted · stripes · column lines".to_owned(),
      "├─ note      callout with a bar, in its tone".to_owned(),
      "├─ details   summary on muted · chevron · content padded 12".to_owned(),
      "└─ footnotes rule 120 · 13 muted · the way back".to_owned(),
    ];
    let (to_selectable, to_fit, to_tasks, to_source) = (
      self.selectable.clone(),
      self.fit_content.clone(),
      self.take_tasks.clone(),
      self.source.clone(),
    );
    let make = {
      let props = props.clone();
      let source = source.clone();
      move || Element::from(props(source.get_untracked()))
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Markdown",
        make: Arc::new(make),
        shown: Shown::built(scene),
        fill: true,
        forceable: false,
        facts: "the source over the document".to_owned(),
        event: "on_task_change",
        code,
        tree,
        controls: vec![
          flag("selectable", &self.selectable),
          flag("fit_content", &self.fit_content),
          divider(),
          flag("on_task_change", &self.take_tasks),
        ],
        changed: [selectable, fit_content, !take_tasks]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to_selectable.set(true);
          to_fit.set(false);
          to_tasks.set(true);
          to_source.set(format!("{PAGE_ONE}\n{PAGE_TWO}"));
        }),
      },
    )
  }

  /// The board's document: two pages side by side, each 420 wide.
  fn board(ctx: &mut Ctx) -> Element {
    let page = |ctx: &mut Ctx, source: &str| -> Element {
      Column::new()
        .width(420.0)
        .max_width(FILL)
        .child(Markdown::mount(
          ctx,
          MarkdownProps::styled(source, prose()).selectable(true),
        ))
        .into()
    };
    let one = page(ctx, PAGE_ONE);
    let two = page(ctx, PAGE_TWO);
    // Side by side where the stage has room for both, the second under the first where it has not.
    Row::new()
      .width(FILL)
      .spacing(60.0)
      .wrap()
      .align_items(Alignment::Start)
      .child(one)
      .child(two)
      .into()
  }
}

impl Component for MarkdownPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      source: ctx.signal(format!("{PAGE_ONE}\n{PAGE_TWO}")),
      tasks: ctx.signal(BLOCKS[TASKS].2.to_owned()),
      selectable: ctx.signal(true),
      fit_content: ctx.signal(false),
      take_tasks: ctx.signal(true),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "MarkdownProps::new",
        "(source)",
        "",
        "A document of the source, in the theme's text; `styled(source, style)` in another.",
      ),
      (
        "selectable",
        "bool",
        "false",
        "A drag selects across blocks, and a copy gives each block on its own line.",
      ),
      (
        "fit_content",
        "bool",
        "false",
        "The document is as wide as its widest block, at most its container's width.",
      ),
      (
        "on_task_change",
        "impl Fn(usize, bool)",
        "",
        "A task's box was pressed: its index among the tasks, and whether it is checked. Without it the boxes are \
         read only.",
      ),
      (
        "on_link_select / on_link_activate",
        "impl Fn(&MarkdownLink)",
        "",
        "A link was followed; a footnote's reference goes to its footnote on its own.",
      ),
      (
        "highlights",
        "impl Into<Arc<[TextHighlight]>>",
        "",
        "Fills behind ranges of the rendered text (`rendered_text()`): a search's matches.",
      ),
      (
        "theme",
        "ThemeMarkdown",
        "the theme's",
        "The document's look: its sizes, and its colors as the palette's roles.",
      ),
    ];
    let playground = self.play(ctx);
    let board = Self::board(ctx);
    let mut sections = vec![
      Section::new(
        "Playground",
        "The source over the document: type in the source and the document follows. Check a \
         task: the playground writes it into the source.",
        playground,
      ),
      Section::new(
        "The board's document",
        "Every block the board draws, on its two pages of 420.",
        self.frame.demo(
          0,
          board,
          "MarkdownProps::styled(source, prose)",
          &["Markdown::mount(ctx, MarkdownProps::styled(source, prose).selectable(true))"],
        ),
      ),
    ];
    for (index, (title, guidance, source)) in BLOCKS.iter().enumerate() {
      let props = if index == TASKS {
        let tasks = self.tasks.clone();
        MarkdownProps::styled(tasks.get(), prose()).on_task_change(move |index, checked| {
          tasks.update(|text| *text = with_task(text, index, checked));
        })
      } else {
        MarkdownProps::styled(*source, prose())
      };
      let shown = Column::new()
        .width(FILL)
        .max_width(560.0)
        .child(Markdown::mount(ctx, props.selectable(true)));
      let lines: Vec<&str> = source.lines().collect();
      sections.push(Section::new(
        title,
        guidance,
        self.frame.demo(
          index as u32 + 1,
          shown,
          lines.first().copied().unwrap_or_default(),
          &lines,
        ),
      ));
    }
    sections.push(Section::new(
      "Props",
      "What a document is given.",
      props_table(&rows, &["MarkdownProps::new"], self.frame.room()),
    ));
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Markdown"],
        title: "Markdown",
        description: "A document of Markdown in the system's type and roles: headings, lists and tasks, tables, \
                      quotes and notes, code, details, images and footnotes.",
        import: Some("use lurq::components::{Markdown, MarkdownProps};"),
        status: Some("Beta"),
        since: Some("0.62"),
        source: file!(),
      },
      sections,
    )
  }
}
