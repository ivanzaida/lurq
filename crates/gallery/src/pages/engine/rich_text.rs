//! The page of rich text: a paragraph of runs, each in a style of its own (`Text::rich`), with elements standing in
//! its lines among the words (`Text::object`), and the styled input, which keeps marks, links, kinds of paragraphs
//! and objects over a plain string and reads and writes them as Markdown (`TextInput::spans`, `RichText`).

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{
    Column, InlineObject, ParagraphKind, ParagraphMarker, ParagraphStyle, ParagraphStyles, RichText, Row, SpanStyle,
    StyledSpan, Text, TextInput, TextInputController, TextMark,
  },
  core::Signal,
  layout::{
    Alignment,
    quad::RichTextSpan,
    text_style::{FontStyle, FontWeight, TextStrikethrough, TextStyle, TextUnderline},
  },
  node::{CursorIcon, Element, border::Border},
  ui::{Badge, Button, ButtonSize, ButtonVariant},
};

use super::kit::{controls, named, number, tile, well};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, mono, plain_block, props_table, quiet, text},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

const WEIGHTS: [(&str, usize); 4] = [("Normal", 0), ("Medium", 1), ("SemiBold", 2), ("Bold", 3)];
const WEIGHT_VALUES: [FontWeight; 4] = [
  FontWeight::Normal,
  FontWeight::Medium,
  FontWeight::SemiBold,
  FontWeight::Bold,
];
/// How large the run is beside the line's own size.
const SIZES: [(&str, usize); 3] = [("the line's", 0), ("larger", 1), ("smaller", 2)];
const SIZE_FACTORS: [f32; 3] = [1.0, 1.3, 0.85];
/// The run's color: none, which follows the foreground, or a role of the theme the page reads its color from.
const COLORS: [(&str, usize); 5] = [
  ("the foreground", 0),
  ("Primary", 1),
  ("Destructive", 2),
  ("MutedForeground", 3),
  ("Chart2", 4),
];
const COLOR_ROLES: [Option<PaletteColor>; 5] = [
  None,
  Some(PaletteColor::Primary),
  Some(PaletteColor::Destructive),
  Some(PaletteColor::MutedForeground),
  Some(PaletteColor::Chart2),
];
/// How far the run is drawn above the baseline: a superscript, or below it.
const SHIFTS: [(&str, i32); 3] = [("0", 0), ("6", 6), ("-4", -4)];

/// What a Markdown source the page loads into a styled input says: a heading, marks, a list with a link, a quote,
/// a line of code and a picture among words.
const MARKDOWN: &str = "## Release notes\n\nBackdrop blur **follows clips**, and *italic* runs keep their \
                        `code`.\n\n- Faster shaping\n- A [guide](docs/text-input-styled.md) to styled \
                        inputs\n\n> A quote stands on a bar.\n\n```\ncargo run -p gallery\n```\n\nThe lake ![at \
                        dusk](photos/lake.jpg) from the north shore.";

/// The run of the playground as its controls make it.
#[derive(Clone, Copy)]
struct Run {
  weight: usize,
  size: usize,
  color: usize,
  shift: i32,
  italic: bool,
  underline: bool,
  strikethrough: bool,
  object: bool,
}

/// The playground's sentence, in `theme`: the middle run in the style the controls give it.
fn sentence(theme: &Theme, run: Run) -> Text {
  let body = theme.typography_style(TypographyStyle::Body);
  let mut style = TextStyle {
    weight: WEIGHT_VALUES[run.weight.min(3)],
    font_size: body.font_size * SIZE_FACTORS[run.size.min(2)],
    baseline_shift: run.shift as f32,
    ..body.clone()
  };
  if let Some(role) = COLOR_ROLES[run.color.min(4)].clone() {
    style.color = theme.palette_color(role);
  }
  if run.italic {
    style.style = FontStyle::Italic;
  }
  if run.underline {
    style.underline = Some(TextUnderline::new());
  }
  if run.strikethrough {
    style.strikethrough = Some(TextStrikethrough::new());
  }
  let mut spans = vec![
    RichTextSpan::new("Saved ", body.clone()),
    RichTextSpan::new("three files", style),
  ];
  if run.object {
    spans.push(RichTextSpan::new(" ", body.clone()));
    spans.push(RichTextSpan::object(body.clone()));
  }
  spans.push(RichTextSpan::new(" to the shared folder.", body));
  let text = Text::rich(spans);
  if run.object {
    text.object(Badge::new("new"))
  } else {
    text
  }
}

/// A run of a paragraph in `style`, a change of the body type.
fn run(content: &str, body: &TextStyle, change: impl FnOnce(&mut TextStyle)) -> RichTextSpan {
  let mut style = body.clone();
  change(&mut style);
  RichTextSpan::new(content, style)
}

/// Where in `text` `part` is, as bytes: for spans written over a sentence.
fn bytes(text: &str, part: &str) -> std::ops::Range<usize> {
  let start = text.find(part).unwrap_or(0);
  start..start + part.len()
}

/// The sentence of the styled input, and its marks.
const STYLED: &str = "Select a word and make it bold, italic, code or a link. Ctrl+Z takes a mark back.";

fn styled_spans() -> Vec<StyledSpan> {
  vec![
    StyledSpan::new(bytes(STYLED, "bold"), SpanStyle::new().bold()),
    StyledSpan::new(bytes(STYLED, "italic"), SpanStyle::new().italic()),
    StyledSpan::new(bytes(STYLED, "code"), SpanStyle::new().code()),
    StyledSpan::new(bytes(STYLED, "a link"), SpanStyle::new().link("https://lurq.dev")),
  ]
}

/// The sentence of the input whose links are clicked.
const LINKED: &str = "Read the guide to styled inputs, or the notes on tokens.";

fn linked_spans() -> Vec<StyledSpan> {
  vec![
    StyledSpan::new(
      bytes(LINKED, "the guide to styled inputs"),
      SpanStyle::new().link("docs/text-input-styled.md"),
    ),
    StyledSpan::new(
      bytes(LINKED, "the notes on tokens"),
      SpanStyle::new().link("docs/text-input-tokens.md"),
    ),
  ]
}

/// How the kinds of the document's paragraphs are drawn: in the theme's type and roles.
fn paragraph_styles(theme: &Theme) -> ParagraphStyles {
  let mono = theme.typography_style(TypographyStyle::Mono).font_family;
  ParagraphStyles::new()
    .gap(10.0)
    .with(
      ParagraphKind::Heading(2),
      ParagraphStyle::new()
        .font_size(20.0)
        .weight(FontWeight::SemiBold)
        .line_height(1.25),
    )
    .with(
      ParagraphKind::Bullet,
      ParagraphStyle::new().indent(21.0).marker(ParagraphMarker::Dot {
        size: 5.0,
        gap: 10.0,
        color: PaletteColor::MutedForeground.into(),
      }),
    )
    .with(
      ParagraphKind::Quote,
      ParagraphStyle::new()
        .indent(14.0)
        .color(PaletteColor::MutedForeground)
        .marker(ParagraphMarker::Bar {
          width: 3.0,
          color: PaletteColor::Border.into(),
        }),
    )
    .with(
      ParagraphKind::Code,
      ParagraphStyle::new()
        .font_family(mono)
        .font_size_scale(0.92)
        .background(PaletteColor::Muted, 6.0)
        .indent(8.0)
        .joined(true),
    )
}

/// What stands in the document for one of its objects: a tile in a chart color with what it is in words. A picture
/// would be `ui::Picture::from_file(&object.source)`; the page has none to show.
fn object_tile(object: &InlineObject) -> Element {
  let label = if object.label.is_empty() {
    object.source.to_string()
  } else {
    object.label.to_string()
  };
  Row::new()
    .height(24.0)
    .padding_horizontal(8.0)
    .align_items(Alignment::Center)
    .rounded(RadiusSize::Sm)
    .background(PaletteColor::Chart2)
    .child(
      text(&format!("picture: {label}"), 12.0, FontWeight::Medium)
        .color(PaletteColor::Background)
        .nowrap(),
    )
    .into()
}

/// The type of a styled input's plain text: the theme's body in its foreground. An input whose text has runs draws
/// a run that names no color in the color of its own style, which is black unless it is given one: in a dark theme
/// the text would be dark on dark.
fn plain() -> TextStyle {
  let theme = lurq::ui::current_theme();
  TextStyle {
    color: theme.palette_color(PaletteColor::Foreground),
    ..theme.typography_style(lurq::app::theme::TypographyStyle::Body)
  }
}

/// A styled input in a frame of the theme's roles, with its edge in the ring's color while it has the focus.
fn framed(input: TextInput) -> TextInput {
  input
    .width(FILL)
    .padding_horizontal(12.0)
    .padding_vertical(10.0)
    .rounded(RadiusSize::Md)
    .background(PaletteColor::Background)
    .border(Border::inside(1.0, PaletteColor::Input))
    .focused(|style| style.border_inside(2.0, PaletteColor::Ring))
    .cursor(CursorIcon::Text)
}

pub struct RichTextPage {
  frame: Frame,
  playground: Playground,
  /// An index into [`WEIGHT_VALUES`].
  weight: Signal<usize>,
  /// An index into [`SIZE_FACTORS`].
  size: Signal<usize>,
  /// An index into [`COLOR_ROLES`].
  color: Signal<usize>,
  shift: Signal<i32>,
  italic: Signal<bool>,
  underline: Signal<bool>,
  strikethrough: Signal<bool>,
  object: Signal<bool>,
  /// How wide the paragraph with objects in it is.
  width: Signal<i32>,
  opened: Signal<i32>,
  linked: Signal<String>,
  linked_spans: Signal<Vec<StyledSpan>>,
  clicked: Signal<String>,
  styled: Signal<String>,
  styled_spans: Signal<Vec<StyledSpan>>,
  controller: TextInputController,
  /// What the styled input writes as Markdown now, and how many spans it holds.
  styled_markdown: Signal<String>,
  span_count: Signal<usize>,
  document: Signal<String>,
  document_spans: Signal<Vec<StyledSpan>>,
  document_kinds: Signal<Vec<ParagraphKind>>,
  document_objects: Signal<Vec<InlineObject>>,
  document_markdown: Signal<String>,
}

impl RichTextPage {
  fn run(&self) -> Run {
    Run {
      weight: self.weight.get(),
      size: self.size.get(),
      color: self.color.get(),
      shift: self.shift.get(),
      italic: self.italic.get(),
      underline: self.underline.get(),
      strikethrough: self.strikethrough.get(),
      object: self.object.get(),
    }
  }

  fn play(&self, ctx: &mut Ctx) -> Element {
    let run = self.run();
    let (weight_name, _) = WEIGHTS[run.weight.min(3)];
    let mut style = vec!["let run = TextStyle {".to_owned()];
    if run.weight != 0 {
      style.push(format!("    weight: FontWeight::{weight_name},"));
    }
    if run.size != 0 {
      style.push(format!(
        "    font_size: body.font_size * {:?},",
        SIZE_FACTORS[run.size.min(2)]
      ));
    }
    if let Some(role) = &COLOR_ROLES[run.color.min(4)] {
      style.push(format!("    color: theme.palette_color(PaletteColor::{role:?}),"));
    }
    if run.shift != 0 {
      style.push(format!("    baseline_shift: {}.0,", run.shift));
    }
    if run.italic {
      style.push("    style: FontStyle::Italic,".to_owned());
    }
    if run.underline {
      style.push("    underline: Some(TextUnderline::new()),".to_owned());
    }
    if run.strikethrough {
      style.push("    strikethrough: Some(TextStrikethrough::new()),".to_owned());
    }
    style.push("    ..body.clone()".to_owned());
    style.push("};".to_owned());
    let mut code = vec!["let body = theme.typography_style(TypographyStyle::Body);".to_owned()];
    code.extend(style);
    code.push("Text::rich([".to_owned());
    code.push("    RichTextSpan::new(\"Saved \", body.clone()),".to_owned());
    code.push("    RichTextSpan::new(\"three files\", run),".to_owned());
    if run.object {
      code.push("    RichTextSpan::new(\" \", body.clone()),".to_owned());
      code.push("    RichTextSpan::object(body.clone()),".to_owned());
    }
    code.push("    RichTextSpan::new(\" to the shared folder.\", body),".to_owned());
    if run.object {
      code.push("])".to_owned());
      code.push(".object(Badge::new(\"new\"))".to_owned());
    } else {
      code.push("])".to_owned());
    }

    let mut marks = vec![weight_name.to_lowercase()];
    if run.italic {
      marks.push("italic".to_owned());
    }
    if run.underline {
      marks.push("underline".to_owned());
    }
    if run.strikethrough {
      marks.push("strikethrough".to_owned());
    }
    let mut tree = vec![
      format!("text   {}", self.playground.measured()),
      "├─ run \"Saved \"".to_owned(),
      format!("├─ run \"three files\"   {}", marks.join(", ")),
    ];
    if run.object {
      tree.push("├─ run \" \"".to_owned());
      tree.push("├─ object U+FFFC   Badge \"new\"".to_owned());
    }
    tree.push("└─ run \" to the shared folder.\"".to_owned());

    let (set_weight, set_size, set_color, set_shift) = (
      self.weight.clone(),
      self.size.clone(),
      self.color.clone(),
      self.shift.clone(),
    );
    let reset = {
      let (weight, size, color, shift) = (
        self.weight.clone(),
        self.size.clone(),
        self.color.clone(),
        self.shift.clone(),
      );
      let flags = [
        self.italic.clone(),
        self.underline.clone(),
        self.strikethrough.clone(),
        self.object.clone(),
      ];
      Arc::new(move || {
        weight.set(3);
        size.set(0);
        color.set(0);
        shift.set(0);
        for flag in &flags {
          flag.set(false);
        }
      })
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Rich text",
        make: Arc::new(move || sentence(&lurq::ui::current_theme(), run).into()),
        shown: Shown::built(move |ctx| sentence(ctx.theme(), run).into()),
        fill: false,
        forceable: false,
        facts: if run.object { "4 runs, 1 object" } else { "3 runs" }.to_owned(),
        event: "",
        code,
        tree,
        controls: vec![
          prop(
            "weight",
            "FontWeight",
            self
              .playground
              .select(0, &WEIGHTS, &run.weight, move |value| set_weight.set(value)),
          ),
          prop(
            "font_size",
            "f32",
            self
              .playground
              .select(1, &SIZES, &run.size, move |value| set_size.set(value)),
          ),
          prop(
            "color",
            "Color",
            self
              .playground
              .select(2, &COLORS, &run.color, move |value| set_color.set(value)),
          ),
          prop(
            "baseline_shift",
            "f32",
            self
              .playground
              .select(3, &SHIFTS, &run.shift, move |value| set_shift.set(value)),
          ),
          divider(),
          flag("italic", &self.italic),
          flag("underline", &self.underline),
          flag("strikethrough", &self.strikethrough),
          flag("an object after the run", &self.object),
        ],
        changed: [
          run.weight != 3,
          run.size != 0,
          run.color != 0,
          run.shift != 0,
          run.italic,
          run.underline,
          run.strikethrough,
          run.object,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset,
      },
    )
  }

  /// A paragraph of every kind of run the engine draws, which a selection crosses as one text.
  fn runs(&self, theme: &Theme) -> Column {
    let body = theme.typography_style(TypographyStyle::Body);
    let mono = theme.typography_style(TypographyStyle::Mono);
    let paragraph = Text::rich([
      run("A paragraph of runs is laid out as one text: ", &body, |_| {}),
      run("bold", &body, |style| style.weight = FontWeight::Bold),
      run(", ", &body, |_| {}),
      run("italic", &body, |style| style.style = FontStyle::Italic),
      run(", ", &body, |_| {}),
      run("in the primary color", &body, |style| {
        style.color = theme.palette_color(PaletteColor::Primary)
      }),
      run(", ", &body, |_| {}),
      run("underlined", &body, |style| {
        style.underline = Some(TextUnderline::new())
      }),
      run(", ", &body, |_| {}),
      run("struck through", &body, |style| {
        style.strikethrough = Some(TextStrikethrough::new())
      }),
      run(", ", &body, |_| {}),
      run("in the family of code", &body, |style| {
        style.font_family = mono.font_family.clone();
        style.font_size = body.font_size * 0.92;
      }),
      run(", ", &body, |_| {}),
      run("larger", &body, |style| {
        style.font_size = body.font_size * 1.4;
        style.weight = FontWeight::SemiBold;
      }),
      run(" and with a footnote", &body, |_| {}),
      run("1", &body, |style| {
        style.font_size = body.font_size * 0.75;
        style.baseline_shift = 6.0;
      }),
      run(
        ". It wraps where its width ends, a run broken across lines like a word, and a drag selects from one run \
         into the next.",
        &body,
        |_| {},
      ),
    ])
    .selectable(true)
    .width(FILL);
    Column::new().width(FILL).child(paragraph)
  }

  /// A paragraph with elements in its lines, in a well as wide as the slider says.
  fn objects(&self, theme: &Theme) -> Column {
    let body = theme.typography_style(TypographyStyle::Body);
    let opened = self.opened.clone();
    let paragraph = Text::rich([
      RichTextSpan::new("The release is ", body.clone()),
      RichTextSpan::object(body.clone()),
      RichTextSpan::new(" and waits for a review. ", body.clone()),
      RichTextSpan::object(body.clone()),
      RichTextSpan::new(" shows it, and the color of its build is ", body.clone()),
      RichTextSpan::object(body.clone()),
      RichTextSpan::new(" until it is merged.", body),
    ])
    .object(Badge::new("draft"))
    .object(
      Button::new("Open")
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Xs)
        .on_click(move |_| opened.update(|count| *count += 1)),
    )
    .object(tile("", 2, 14.0, 14.0))
    .width(FILL);
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(
        controls()
          .child(number("width", &self.width, 160, 640))
          .child(quiet(&format!("presses of Open: {}", self.opened.get()), 12.0)),
      )
      .child(
        well()
          .width(self.width.get() as f32)
          .max_width(FILL)
          .padding(16.0)
          .child(paragraph),
      )
  }

  /// A styled input whose runs are links: a press with Ctrl or Cmd held tells the page which.
  fn links(&self) -> Column {
    let clicked = self.clicked.clone();
    let input = framed(
      TextInput::styled(self.linked.clone(), plain())
        .spans(self.linked_spans.clone())
        .multiline()
        .on_link_click(move |link| {
          clicked.set(format!(
            "{}   bytes {}..{}",
            link.target, link.range.start, link.range.end
          ))
        }),
    );
    let last = self.clicked.get();
    Column::new().width(FILL).spacing(12.0).child(input).child(named(
      "on_link_click",
      mono(if last.is_empty() { "nothing yet" } else { &last }, 12.0),
    ))
  }

  /// A styled input with a bar of commands its controller runs, and what it holds under it.
  fn editor(&self) -> Column {
    let command = |label: &str, run: Arc<dyn Fn(&TextInputController) + Send + Sync>| {
      let controller = self.controller.clone();
      Button::new(label)
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Sm)
        .on_click(move |_| run(&controller))
    };
    let mark = |label: &str, mark: TextMark| {
      command(
        label,
        Arc::new(move |controller: &TextInputController| {
          let _ = controller.toggle_mark(mark);
        }),
      )
    };
    let undo = command(
      "Undo",
      Arc::new(|controller: &TextInputController| {
        let _ = controller.undo();
      }),
    )
    .disabled(!self.controller.can_undo());
    let redo = command(
      "Redo",
      Arc::new(|controller: &TextInputController| {
        let _ = controller.redo();
      }),
    )
    .disabled(!self.controller.can_redo());
    let bar = Row::new()
      .width(FILL)
      .spacing(6.0)
      .wrap()
      .child(mark("Bold", TextMark::Bold))
      .child(mark("Italic", TextMark::Italic))
      .child(mark("Underline", TextMark::Underline))
      .child(mark("Code", TextMark::Code))
      .child(mark("Strike", TextMark::Strikethrough))
      .child(command(
        "Link",
        Arc::new(|controller: &TextInputController| {
          let _ = controller.set_link(Some("https://lurq.dev"));
        }),
      ))
      .child(command(
        "Unlink",
        Arc::new(|controller: &TextInputController| {
          let _ = controller.set_link(None);
        }),
      ))
      .child(undo)
      .child(redo);
    let (markdown, count) = (self.styled_markdown.clone(), self.span_count.clone());
    let input = framed(
      TextInput::styled(self.styled.clone(), plain())
        .key("styled-input")
        .spans(self.styled_spans.clone())
        .controller(self.controller.clone())
        .multiline()
        .on_styled_change(move |change| {
          count.set(change.spans().len());
          markdown.set(change.to_rich_text().to_markdown());
        }),
    );
    Column::new()
      .width(FILL)
      .spacing(12.0)
      .child(bar)
      .child(input)
      .child(named("spans", mono(&self.span_count.get().to_string(), 12.0)))
      .child(named(
        "to_markdown",
        mono(&self.styled_markdown.get(), 12.0).width(FILL),
      ))
  }

  /// A Markdown source read into a styled input with kinds of paragraphs and an object, and what it writes back.
  fn document(&self, theme: &Theme) -> Column {
    let markdown = self.document_markdown.clone();
    let input = framed(
      TextInput::styled(self.document.clone(), plain())
        .key("document")
        .spans(self.document_spans.clone())
        .paragraphs(self.document_kinds.clone(), paragraph_styles(theme))
        .objects(self.document_objects.clone(), object_tile)
        .multiline()
        .on_styled_change(move |change| markdown.set(change.to_rich_text().to_markdown())),
    );
    let load = {
      let (value, spans, kinds, objects, markdown) = (
        self.document.clone(),
        self.document_spans.clone(),
        self.document_kinds.clone(),
        self.document_objects.clone(),
        self.document_markdown.clone(),
      );
      move || {
        let document = RichText::from_markdown(MARKDOWN);
        markdown.set(document.to_markdown());
        spans.set(document.spans);
        kinds.set(document.paragraphs);
        objects.set(document.objects);
        value.set(document.text);
      }
    };
    let written: Vec<String> = self.document_markdown.get().lines().map(str::to_owned).collect();
    Column::new()
      .width(FILL)
      .spacing(12.0)
      .child(
        controls().child(
          Button::new("Read the Markdown again")
            .variant(ButtonVariant::Outline)
            .size(ButtonSize::Sm)
            .on_click(move |_| load()),
        ),
      )
      .child(input)
      .child(quiet("to_markdown(), as the input holds it now:", 12.0))
      .child(
        Column::new()
          .width(FILL)
          .rounded(RadiusSize::Md)
          .background(PaletteColor::Muted)
          .child(plain_block(&written)),
      )
  }
}

impl Component for RichTextPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let document = RichText::from_markdown(MARKDOWN);
    let styled = RichText::new(STYLED, styled_spans(), Vec::new());
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 4),
      weight: ctx.signal(3),
      size: ctx.signal(0),
      color: ctx.signal(0),
      shift: ctx.signal(0),
      italic: ctx.signal(false),
      underline: ctx.signal(false),
      strikethrough: ctx.signal(false),
      object: ctx.signal(false),
      width: ctx.signal(420),
      opened: ctx.signal(0),
      linked: ctx.signal(LINKED.to_owned()),
      linked_spans: ctx.signal(linked_spans()),
      clicked: ctx.signal(String::new()),
      styled: ctx.signal(STYLED.to_owned()),
      span_count: ctx.signal(styled.spans.len()),
      styled_markdown: ctx.signal(styled.to_markdown()),
      styled_spans: ctx.signal(styled_spans()),
      controller: TextInputController::new(),
      document_markdown: ctx.signal(document.to_markdown()),
      document: ctx.signal(document.text),
      document_spans: ctx.signal(document.spans),
      document_kinds: ctx.signal(document.paragraphs),
      document_objects: ctx.signal(document.objects),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let theme = ctx.theme().clone();
    let rows = [
      (
        "Text::rich",
        "impl IntoIterator<Item = RichTextSpan>",
        "",
        "A paragraph of runs, each in a style of its own, laid out, wrapped, drawn and selected as one text.",
      ),
      (
        "RichTextSpan::new",
        "(impl Into<String>, TextStyle)",
        "",
        "A run: its text and its whole style. A color left as the default follows the foreground of what holds \
         it; baseline_shift draws it above the line or below it.",
      ),
      (
        "RichTextSpan::object",
        "TextStyle",
        "",
        "The place of an element in the line: one OBJECT_CHARACTER in the line's style.",
      ),
      (
        "Text::object",
        "impl Into<Element>",
        "",
        "The element for the paragraph's next object, in the order of the text. It wraps with the words as one of \
         them, and takes presses, the focus and Tab as any element does.",
      ),
      (
        "spans",
        "Signal<Vec<StyledSpan>>",
        "",
        "Makes an input styled: the marks and links over its plain string, as byte ranges the input moves with \
         every edit and keeps in its undo history.",
      ),
      (
        "SpanStyle",
        "bold, italic, code, underline, strikethrough, link",
        "plain",
        "The marks of a span, drawn as the theme's text marks say. A link carries a target the app opens.",
      ),
      (
        "on_link_click",
        "impl Fn(&LinkClick)",
        "",
        "Called when a link is pressed with Ctrl, or Cmd on a Mac, with its target and its bytes. A plain press \
         places the caret.",
      ),
      (
        "on_styled_change",
        "impl Fn(&StyledChange)",
        "",
        "Called after every change the input makes: what the edit was, and the spans, kinds and objects it holds \
         now.",
      ),
      (
        "paragraphs",
        "Signal<Vec<ParagraphKind>>, ParagraphStyles",
        "",
        "What each line of the text is, a heading, an item, a quote or code, and how each kind is drawn.",
      ),
      (
        "objects",
        "Signal<Vec<InlineObject>>, impl FnMut(&InlineObject) -> Element",
        "",
        "The things in the text that are not text: each is one OBJECT_CHARACTER, drawn as the element the \
         function builds for its record.",
      ),
      (
        "RichText::from_markdown",
        "&str",
        "",
        "A document from Markdown, as far as a styled input holds it: its text, spans, kinds and objects. \
         to_markdown writes it back.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Rich Text"],
        title: "Rich Text",
        description: "Text in more than one style: a paragraph of runs with elements in its lines, and an input \
                      that keeps marks, links, kinds of paragraphs and pictures over a plain string, read from \
                      Markdown and written back as it.",
        import: Some("use lurq::components::{RichText, SpanStyle, StyledSpan, Text, TextInput};"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "A sentence of three runs: the middle one takes the style the controls give it, and an object can stand \
           after it in the line.",
          playground,
        ),
        Section::new(
          "Runs of styles",
          "Each run has a style of its own, and the paragraph is still one text: it wraps across runs, and a drag \
           selects from one into the next.",
          self.frame.demo(
            0,
            self.runs(&theme),
            "Text::rich([RichTextSpan::new(\"bold\", bold), ..])",
            &[
              "let body = theme.typography_style(TypographyStyle::Body);",
              "let bold = TextStyle { weight: FontWeight::Bold, ..body.clone() };",
              "let footnote = TextStyle { font_size: body.font_size * 0.75, baseline_shift: 6.0, ..body.clone() };",
              "Text::rich([",
              "    RichTextSpan::new(\"A paragraph of runs is laid out as one text: \", body.clone()),",
              "    RichTextSpan::new(\"bold\", bold),",
              "    // ..",
              "    RichTextSpan::new(\" and with a footnote\", body.clone()),",
              "    RichTextSpan::new(\"1\", footnote),",
              "])",
              ".selectable(true)",
              ".width(Dimension::Pct(100.0))",
            ],
          ),
        ),
        Section::new(
          "Objects in the line",
          "Elements stand among the words, each where an OBJECT_CHARACTER is: a badge, a button and a tile. Make \
           the paragraph narrower and they wrap with the words. The button takes presses as any button does.",
          self.frame.demo(
            1,
            self.objects(&theme),
            "Text::rich([.., RichTextSpan::object(body), ..]).object(Badge::new(\"draft\"))",
            &[
              "Text::rich([",
              "    RichTextSpan::new(\"The release is \", body.clone()),",
              "    RichTextSpan::object(body.clone()),",
              "    RichTextSpan::new(\" and waits for a review. \", body.clone()),",
              "    RichTextSpan::object(body.clone()),",
              "    // ..",
              "])",
              "// The elements of the objects, in the order of the text.",
              ".object(Badge::new(\"draft\"))",
              ".object(Button::new(\"Open\").on_click(move |_| opened.update(|n| *n += 1)))",
              ".object(Rect::new(14.0, 14.0).background(PaletteColor::Chart3))",
            ],
          ),
        ),
        Section::new(
          "Links",
          "A run of a styled input can be a link with a target. A plain press places the caret; a press with Ctrl \
           held, or Cmd on a Mac, tells the app, which opens it. A paragraph of Text::rich has no links of its own: \
           a link there is an element in its line, a ui::Link.",
          self.frame.demo(
            2,
            self.links(),
            ".on_link_click(|link| open(&link.target))",
            &[
              "// text: Signal<String>, spans: Signal<Vec<StyledSpan>>",
              "spans.set(vec![",
              "    StyledSpan::new(9..35, SpanStyle::new().link(\"docs/text-input-styled.md\")),",
              "    StyledSpan::new(37..56, SpanStyle::new().link(\"docs/text-input-tokens.md\")),",
              "]);",
              "TextInput::new(text)",
              "    .spans(spans)",
              "    .multiline()",
              "    .on_link_click(move |link: &LinkClick| clicked.set(link.target.to_string()))",
            ],
          ),
        ),
        Section::new(
          "A styled input",
          "The text stays a plain string; the marks over it are a second signal the input keeps in step with every \
           edit. Select a word and press a command: the controller marks the selection, as one step that Undo takes \
           back.",
          self.frame.demo(
            3,
            self.editor(),
            "controller.toggle_mark(TextMark::Bold)",
            &[
              "let controller = TextInputController::new();",
              "TextInput::new(text)",
              "    .spans(spans)",
              "    .controller(controller.clone())",
              "    .multiline()",
              "    .on_styled_change(move |change| {",
              "        markdown.set(change.to_rich_text().to_markdown());",
              "    })",
              "",
              "// The bar's commands:",
              "controller.toggle_mark(TextMark::Bold)?;",
              "controller.set_link(Some(\"https://lurq.dev\"))?;",
              "controller.set_link(None)?;",
              "controller.undo()?;",
            ],
          ),
        ),
        Section::new(
          "From Markdown and back",
          "A Markdown source read into a styled input: a heading, marks, a list with a link, a quote, a line of code \
           and a picture among words, which is one character of the text drawn as the element the app builds for \
           it. Edit it, and the Markdown under it is what the input holds.",
          self.frame.demo(
            4,
            self.document(&theme),
            "RichText::from_markdown(source)",
            &[
              "let document = RichText::from_markdown(source);",
              "text.set(document.text);",
              "spans.set(document.spans);",
              "kinds.set(document.paragraphs);",
              "objects.set(document.objects);",
              "",
              "TextInput::new(text)",
              "    .spans(spans)",
              "    .paragraphs(kinds, ParagraphStyles::new()",
              "        .with(ParagraphKind::Heading(2), ParagraphStyle::new().font_size(20.0))",
              "        .with(ParagraphKind::Bullet, ParagraphStyle::new().indent(21.0).marker(dot))",
              "        // ..",
              "    )",
              "    .objects(objects, |object| Picture::from_file(&object.source).into())",
              "    .multiline()",
              "    .on_styled_change(move |change| markdown.set(change.to_rich_text().to_markdown()))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a paragraph of runs is built of, and what makes an input styled: from spans on, the methods of \
           TextInput.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
