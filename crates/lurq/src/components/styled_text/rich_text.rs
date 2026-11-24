//! A document as a styled input edits it: its text, the marks over it and
//! what each of its paragraphs is; read from Markdown and written as it.
use std::sync::Arc;

use super::{
  InlineObject, OBJECT_CHARACTER, ParagraphKind, SpanStyle, StyledSpan, StyledText, normalize, normalize_kinds,
  normalize_objects, paragraph_ranges, slice_objects, text_with_labels,
};
use crate::{
  app::component::{DevtoolsFormatter, DevtoolsInspectable},
  markdown::{MarkdownBlock, MarkdownHeadingLevel, MarkdownInline, parse_markdown},
};

/// Formatted text: what a styled input with paragraphs holds in its
/// signals, as one value. Its paragraphs are the lines of `text`; `spans` are
/// the marks over it as UTF-8 byte ranges; `paragraphs` says what each line
/// is; `objects` are the pictures in it, each one character of `text`
/// ([`OBJECT_CHARACTER`]) and what stands there ([`InlineObject`]).
///
/// ```
/// use lurq::components::{ParagraphKind, RichText};
///
/// let notes = RichText::from_markdown("## Release notes\n\nBackdrop blur **follows clips**.\n\n- Faster shaping");
/// assert_eq!(notes.text, "Release notes\nBackdrop blur follows clips.\nFaster shaping");
/// assert_eq!(
///   notes.paragraphs,
///   [ParagraphKind::Heading(2), ParagraphKind::Body, ParagraphKind::Bullet]
/// );
/// assert_eq!(notes.to_markdown(), "## Release notes\n\nBackdrop blur **follows clips**.\n\n- Faster shaping");
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct RichText {
  pub text: String,
  pub spans: Vec<StyledSpan>,
  pub paragraphs: Vec<ParagraphKind>,
  /// The things in the text that are not text, in its order: see [`InlineObject`].
  pub objects: Vec<InlineObject>,
}

impl DevtoolsInspectable for RichText {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    formatter.value_debug(self);
  }
}

impl RichText {
  /// `text` with `spans` and the kinds of its paragraphs, put in the form an
  /// input keeps: the spans canonical (see [`StyledText::new`]), one kind for
  /// each line of the text, body where none was given. It holds no objects: [`Self::with_objects`] gives it them.
  pub fn new(text: impl Into<String>, spans: Vec<StyledSpan>, paragraphs: Vec<ParagraphKind>) -> Self {
    let text = text.into();
    let spans = normalize(&text, spans);
    let paragraphs = normalize_kinds(paragraphs, paragraph_ranges(&text).len());
    Self {
      text,
      spans,
      paragraphs,
      objects: Vec::new(),
    }
  }

  /// The document with `objects` for the [`OBJECT_CHARACTER`]s of its text, put in the form an input keeps: in
  /// the order of the text, and only those whose byte holds the character.
  ///
  /// ```
  /// use lurq::components::{InlineObject, RichText};
  ///
  /// let notes = RichText::plain("Dusk: \u{FFFC}").with_objects(vec![InlineObject::new(6, "photos/lake.jpg")]);
  /// assert_eq!(notes.to_markdown(), "Dusk: ![](photos/lake.jpg)");
  /// ```
  pub fn with_objects(mut self, objects: Vec<InlineObject>) -> Self {
    self.objects = normalize_objects(&self.text, objects);
    self
  }

  /// Body text with no marks.
  pub fn plain(text: impl Into<String>) -> Self {
    Self::new(text, Vec::new(), Vec::new())
  }

  /// Whether there is no text.
  pub fn is_empty(&self) -> bool {
    self.text.is_empty()
  }

  /// The text and its marks, without the kinds of its paragraphs.
  pub fn styled(&self) -> StyledText {
    StyledText {
      text: self.text.clone(),
      spans: self.spans.clone(),
    }
  }

  /// The text as plain text shows it: the label of each object in place of its character.
  pub fn plain_text(&self) -> String {
    text_with_labels(&self.text, &self.objects)
  }

  /// How many words the text has: its runs of what is not white space. An object is no word, and stands between
  /// two as a space does.
  pub fn word_count(&self) -> usize {
    if self.objects.is_empty() {
      return self.text.split_whitespace().count();
    }
    let mut objects = self.objects.iter().peekable();
    let (mut words, mut in_word) = (0, false);
    for (at, character) in self.text.char_indices() {
      let object = objects.next_if(|object| object.at == at).is_some();
      let part_of_word = !character.is_whitespace() && !object;
      words += usize::from(part_of_word && !in_word);
      in_word = part_of_word;
    }
    words
  }

  /// Each paragraph: its kind, and its text with its marks. An object of a paragraph is the
  /// [`OBJECT_CHARACTER`] in its text; `objects` holds what each is, in the order of the text.
  pub fn blocks(&self) -> Vec<(ParagraphKind, StyledText)> {
    let kinds = normalize_kinds(self.paragraphs.clone(), paragraph_ranges(&self.text).len());
    paragraph_ranges(&self.text)
      .into_iter()
      .zip(kinds)
      .map(|(range, kind)| {
        let text = StyledText {
          text: self.text[range.clone()].to_owned(),
          spans: super::slice(&self.spans, range),
        };
        (kind, text)
      })
      .collect()
  }

  /// The document a Markdown source says, as far as a styled input holds it:
  /// paragraphs, headings, flat lists, quotes and code blocks, with bold,
  /// italic, strikethrough, inline code, links and `<u>` underlines. A line
  /// break inside a paragraph starts a new one; a nested list is flattened;
  /// an image, `![label](source)` or `<img src alt width>`, is an object
  /// ([`InlineObject`]) where it stands, alone in its paragraph or among
  /// words; a table, a rule and other raw HTML are left out.
  pub fn from_markdown(source: &str) -> Self {
    let mut reader = Reader::default();
    for block in &parse_markdown(source).blocks {
      reader.block(block, ParagraphKind::Body);
    }
    reader.finish()
  }

  /// Whether a text reads as Markdown: one of its lines starts a heading (`# ` to `###### `), an item of a list
  /// (`- `, `* `, `+ `, `1. `, `1) `), a quote (`>`) or a fence of code (three backticks or tildes), after at most
  /// three spaces; or Markdown reads a mark in it (`**bold**`, `_italic_`, `` `code` ``, `~~struck~~`,
  /// `[a link](target)`, `<u>under</u>`) or an image (`![label](source)`, `<img src="source">`). What
  /// [`Self::to_markdown`] writes of formatted text reads as Markdown;
  /// a text with neither is plain, whatever asterisks it has (`2 * 3`). What an input that keeps its formatting
  /// through the clipboard asks of a pasted text.
  ///
  /// ```
  /// use lurq::components::RichText;
  ///
  /// assert!(RichText::reads_as_markdown("## Notes\n\n- one\n- two"));
  /// assert!(RichText::reads_as_markdown("and **that** is all"));
  /// assert!(!RichText::reads_as_markdown("2 * 3 = 6, and that is all"));
  /// ```
  pub fn reads_as_markdown(text: &str) -> bool {
    text.lines().any(|line| {
      let indent = line.len() - line.trim_start_matches(' ').len();
      if indent > 3 {
        return false;
      }
      let line = &line[indent..];
      let hashes = line.len() - line.trim_start_matches('#').len();
      if (1..=6).contains(&hashes) && line[hashes..].starts_with(' ') {
        return true;
      }
      if line.starts_with("```") || line.starts_with("~~~") || line.starts_with('>') {
        return true;
      }
      let item = |rest: &str| rest.starts_with(' ') && !rest.trim().is_empty();
      if line.starts_with(['-', '*', '+']) && item(&line[1..]) {
        return true;
      }
      let digits = line.len() - line.trim_start_matches(|ch: char| ch.is_ascii_digit()).len();
      (1..=9).contains(&digits) && line[digits..].starts_with(['.', ')']) && item(&line[digits + 1..])
    }) || Self::has_inline_marks(text)
  }

  /// Whether Markdown reads a mark or an image in `text`: only a text with one of the characters that open one
  /// is read.
  fn has_inline_marks(text: &str) -> bool {
    if !text.contains(['*', '_', '`', '~', '[', '<']) {
      return false;
    }
    let read = Self::from_markdown(text);
    !read.spans.is_empty() || !read.objects.is_empty()
  }

  /// The document as Markdown, which [`Self::from_markdown`] reads back as
  /// the same document. An underline is written `<u>…</u>`, an object as an
  /// image: `![label](source)`, or `<img src="source" alt="label" width="320">`
  /// where it has a width, which Markdown has no other way to say. In code, a
  /// line of a block or a run of inline code, where Markdown reads no image,
  /// an object is written as its label.
  pub fn to_markdown(&self) -> String {
    let blocks = self.blocks();
    let objects: Vec<Vec<InlineObject>> = paragraph_ranges(&self.text)
      .into_iter()
      .map(|range| slice_objects(&self.objects, range))
      .collect();
    let mut out = String::new();
    let mut number = 0;
    let mut fenced = false;
    for (index, (kind, text)) in blocks.iter().enumerate() {
      let previous = index.checked_sub(1).map(|before| blocks[before].0);
      number = if *kind == ParagraphKind::Ordered && previous == Some(ParagraphKind::Ordered) {
        number + 1
      } else {
        1
      };
      if let Some(previous) = previous {
        out.push_str(match (previous, *kind) {
          // Lines of one block; a block of another language after it is a fence of its own.
          (previous, kind) if previous.is_code() && previous == kind => "\n",
          (ParagraphKind::Bullet, ParagraphKind::Bullet)
          | (ParagraphKind::Ordered, ParagraphKind::Ordered)
          | (ParagraphKind::Checklist { .. }, ParagraphKind::Checklist { .. }) => "\n",
          (ParagraphKind::Quote, ParagraphKind::Quote) => "\n>\n",
          _ => "\n\n",
        });
      }
      if kind.is_code() {
        if !fenced {
          out.push_str("```");
          out.push_str(kind.language().unwrap_or_default());
          out.push('\n');
          fenced = true;
        }
        out.push_str(&text_with_labels(&text.text, &objects[index]));
        if blocks.get(index + 1).is_none_or(|next| next.0 != *kind) {
          out.push_str("\n```");
          fenced = false;
        }
        continue;
      }
      match kind {
        ParagraphKind::Heading(level) => {
          out.extend(std::iter::repeat_n('#', usize::from((*level).clamp(1, 6))));
          out.push(' ');
        }
        ParagraphKind::Bullet => out.push_str("- "),
        ParagraphKind::Ordered => out.push_str(&format!("{number}. ")),
        ParagraphKind::Quote => out.push_str("> "),
        ParagraphKind::Checklist { checked } => out.push_str(if *checked { "- [x] " } else { "- [ ] " }),
        // A divider holds no text.
        ParagraphKind::Divider => {
          out.push_str("---");
          continue;
        }
        _ => {}
      }
      write_inline(&mut out, text, &objects[index], *kind == ParagraphKind::Body);
    }
    out
  }
}

/// Builds a document from the blocks of a parsed Markdown source.
#[derive(Default)]
struct Reader {
  text: String,
  spans: Vec<StyledSpan>,
  kinds: Vec<ParagraphKind>,
  objects: Vec<InlineObject>,
  /// Whether a paragraph is open: what is read next goes on in it.
  open: bool,
}

impl Reader {
  /// Starts a paragraph of `kind`.
  fn start(&mut self, kind: ParagraphKind) {
    if !self.kinds.is_empty() {
      self.text.push('\n');
    }
    self.kinds.push(kind);
    self.open = true;
  }

  fn push(&mut self, text: &str, style: &SpanStyle) {
    // A line break inside what the source holds as one text starts a paragraph of the kind it is in.
    for (index, line) in text.split('\n').enumerate() {
      if index > 0 || !self.open {
        self.start(self.kinds.last().copied().unwrap_or_default());
      }
      let line = line.trim_end_matches('\r');
      let start = self.text.len();
      self.text.push_str(line);
      if !style.is_plain() && !line.is_empty() {
        self.spans.push(StyledSpan::new(start..self.text.len(), style.clone()));
      }
    }
  }

  /// Puts an image where the text has come to, as an object: in the paragraph that is open, or in one of its own.
  fn object(&mut self, image: Image, style: &SpanStyle) {
    if !self.open {
      self.start(self.kinds.last().copied().unwrap_or_default());
    }
    let at = self.text.len();
    self.text.push(OBJECT_CHARACTER);
    if !style.is_plain() {
      self.spans.push(StyledSpan::new(at..self.text.len(), style.clone()));
    }
    self.objects.push(InlineObject {
      at,
      source: image.source.into(),
      label: image.label.into(),
      width: image.width,
    });
  }

  fn block(&mut self, block: &MarkdownBlock, kind: ParagraphKind) {
    match block {
      MarkdownBlock::Paragraph(children) => {
        self.start(kind);
        self.inlines(children, &SpanStyle::default());
      }
      MarkdownBlock::Heading { level, children } => {
        let level = match level {
          MarkdownHeadingLevel::H1 => 1,
          MarkdownHeadingLevel::H2 => 2,
          MarkdownHeadingLevel::H3 => 3,
          MarkdownHeadingLevel::H4 => 4,
          MarkdownHeadingLevel::H5 => 5,
          MarkdownHeadingLevel::H6 => 6,
        };
        // A heading inside a quote or an item is a paragraph of that.
        self.start(if kind == ParagraphKind::Body {
          ParagraphKind::Heading(level)
        } else {
          kind
        });
        self.inlines(children, &SpanStyle::default());
      }
      MarkdownBlock::BlockQuote(blocks) | MarkdownBlock::Admonition { blocks, .. } => {
        for block in blocks {
          self.block(block, ParagraphKind::Quote);
        }
      }
      MarkdownBlock::Details { summary, blocks, .. } => {
        self.start(kind);
        self.push(summary, &SpanStyle::default());
        for block in blocks {
          self.block(block, kind);
        }
      }
      MarkdownBlock::List { ordered, items, .. } => {
        let item = if *ordered {
          ParagraphKind::Ordered
        } else {
          ParagraphKind::Bullet
        };
        for entry in items {
          if entry.blocks.is_empty() {
            self.start(item);
          }
          // An item that starts with `[ ]` or `[x]` is one of a checklist: its box stands for the mark.
          let mut blocks = std::borrow::Cow::Borrowed(&entry.blocks);
          let mut kind = item;
          if !*ordered
            && let Some(MarkdownBlock::Paragraph(inlines)) = entry.blocks.first()
            && let Some(MarkdownInline::TaskListMarker(checked)) = inlines.first()
          {
            kind = ParagraphKind::Checklist { checked: *checked };
            if let Some(MarkdownBlock::Paragraph(inlines)) = blocks.to_mut().first_mut() {
              inlines.remove(0);
              if let Some(MarkdownInline::Text(text)) = inlines.first_mut() {
                *text = text.trim_start().to_owned();
              }
            }
          }
          for block in blocks.iter() {
            self.block(block, kind);
          }
        }
      }
      MarkdownBlock::CodeBlock { text, kind: code } => {
        // The fence's language is the lines', for the block's header and for the fence it is written with.
        let language = match code {
          crate::markdown::MarkdownCodeBlockKind::Fenced { language } => language.as_deref(),
          crate::markdown::MarkdownCodeBlockKind::Indented => None,
        };
        for line in text.strip_suffix('\n').unwrap_or(text).split('\n') {
          self.start(ParagraphKind::code(language));
          self.text.push_str(line.trim_end_matches('\r'));
        }
      }
      MarkdownBlock::Math { text } => {
        for line in text.strip_suffix('\n').unwrap_or(text).split('\n') {
          self.start(ParagraphKind::Code { language: None });
          self.text.push_str(line.trim_end_matches('\r'));
        }
      }
      MarkdownBlock::FootnoteDefinition { blocks, .. } => {
        for block in blocks {
          self.block(block, kind);
        }
      }
      // An image written as a tag alone on its line is a block of HTML to Markdown: a paragraph of the image.
      MarkdownBlock::Html(html) => {
        if let Some(images) = Image::all_of(html) {
          self.start(kind);
          for image in images {
            self.object(image, &SpanStyle::default());
          }
        }
      }
      MarkdownBlock::ThematicBreak => self.start(ParagraphKind::Divider),
      MarkdownBlock::Table { .. } => {}
    }
    self.open = false;
  }

  fn inlines(&mut self, children: &[MarkdownInline], style: &SpanStyle) {
    let mut style = style.clone();
    for child in children {
      match child {
        MarkdownInline::Text(text) => self.push(text, &style),
        MarkdownInline::Code(text) | MarkdownInline::Math(text) | MarkdownInline::Kbd(text) => {
          self.push(text, &style.clone().code());
        }
        MarkdownInline::Emphasis(children) => self.inlines(children, &style.clone().italic()),
        MarkdownInline::Strong(children) => self.inlines(children, &style.clone().bold()),
        MarkdownInline::Strikethrough(children) => self.inlines(children, &style.clone().strikethrough()),
        MarkdownInline::Link {
          destination, children, ..
        } => self.inlines(children, &style.clone().link(destination.as_str())),
        MarkdownInline::Image { destination, alt, .. } => {
          let image = Image {
            source: destination.clone(),
            label: plain_inlines(alt),
            width: None,
          };
          self.object(image, &style);
        }
        MarkdownInline::Html(html) => match html.trim().to_ascii_lowercase().as_str() {
          "<u>" => style.underline = true,
          "</u>" => style.underline = false,
          "<br>" | "<br/>" | "<br />" => self.push("\n", &style),
          _ => {
            if let Some(image) = Image::of_tag(html.trim()) {
              self.object(image, &style);
            }
          }
        },
        MarkdownInline::FootnoteReference(label) => self.push(&format!("[^{label}]"), &style),
        MarkdownInline::SoftBreak => self.push(" ", &style),
        MarkdownInline::HardBreak => self.push("\n", &style),
        MarkdownInline::TaskListMarker(checked) => self.push(if *checked { "[x] " } else { "[ ] " }, &style),
      }
    }
  }

  fn finish(self) -> RichText {
    RichText::new(self.text, self.spans, self.kinds).with_objects(self.objects)
  }
}

/// An image as a Markdown source says it.
struct Image {
  source: String,
  label: String,
  width: Option<u32>,
}

impl Image {
  /// The image an `<img>` tag says: its `src`, its `alt` and its `width` in pixels. `None` for any other tag,
  /// and for one with no `src`.
  fn of_tag(tag: &str) -> Option<Self> {
    let inner = tag.strip_prefix('<')?.strip_suffix('>')?;
    let inner = inner.strip_suffix('/').unwrap_or(inner);
    let name_end = inner.find(char::is_whitespace).unwrap_or(inner.len());
    if !inner[..name_end].eq_ignore_ascii_case("img") {
      return None;
    }
    let (mut source, mut label, mut width) = (None, String::new(), None);
    let mut rest = inner[name_end..].trim_start();
    while !rest.is_empty() {
      let name_end = rest
        .find(|ch: char| ch.is_whitespace() || ch == '=')
        .unwrap_or(rest.len());
      if name_end == 0 {
        return None;
      }
      let name = rest[..name_end].to_ascii_lowercase();
      rest = rest[name_end..].trim_start();
      let mut value = "";
      if let Some(after) = rest.strip_prefix('=') {
        let after = after.trim_start();
        let (found, left) = match after.chars().next() {
          Some(quote @ ('"' | '\'')) => {
            let end = after[1..].find(quote)?;
            (&after[1..1 + end], &after[end + 2..])
          }
          _ => {
            let end = after.find(char::is_whitespace).unwrap_or(after.len());
            (&after[..end], &after[end..])
          }
        };
        value = found;
        rest = left.trim_start();
      }
      match name.as_str() {
        "src" => source = Some(unescape_attribute(value)),
        "alt" => label = unescape_attribute(value),
        "width" => width = value.trim().trim_end_matches("px").parse().ok(),
        _ => {}
      }
    }
    Some(Self {
      source: source?,
      label,
      width,
    })
  }

  /// The images a block of HTML is, where it is `<img>` tags and nothing else.
  fn all_of(html: &str) -> Option<Vec<Self>> {
    let mut images = Vec::new();
    let mut rest = html.trim();
    while !rest.is_empty() {
      let end = rest.find('>')? + 1;
      images.push(Self::of_tag(&rest[..end])?);
      rest = rest[end..].trim_start();
    }
    (!images.is_empty()).then_some(images)
  }
}

/// The words of inlines without their marks: what an image's alternative text says.
fn plain_inlines(inlines: &[MarkdownInline]) -> String {
  fn walk(out: &mut String, inlines: &[MarkdownInline]) {
    for inline in inlines {
      match inline {
        MarkdownInline::Text(text)
        | MarkdownInline::Code(text)
        | MarkdownInline::Math(text)
        | MarkdownInline::Kbd(text) => out.push_str(text),
        MarkdownInline::Emphasis(children)
        | MarkdownInline::Strong(children)
        | MarkdownInline::Strikethrough(children)
        | MarkdownInline::Link { children, .. } => walk(out, children),
        MarkdownInline::Image { alt, .. } => walk(out, alt),
        MarkdownInline::SoftBreak | MarkdownInline::HardBreak => out.push(' '),
        MarkdownInline::Html(_) | MarkdownInline::FootnoteReference(_) | MarkdownInline::TaskListMarker(_) => {}
      }
    }
  }
  let mut out = String::new();
  walk(&mut out, inlines);
  out
}

fn unescape_attribute(value: &str) -> String {
  value
    .replace("&quot;", "\"")
    .replace("&lt;", "<")
    .replace("&gt;", ">")
    .replace("&amp;", "&")
}

fn escape_attribute(out: &mut String, value: &str) {
  for character in value.chars() {
    match character {
      '&' => out.push_str("&amp;"),
      '"' => out.push_str("&quot;"),
      '<' => out.push_str("&lt;"),
      '>' => out.push_str("&gt;"),
      // A tag is one line of the source.
      '\n' | '\r' => out.push(' '),
      other => out.push(other),
    }
  }
}

/// Writes an object as the image Markdown reads back as it: `![label](source)`, or a tag where it has a width,
/// which Markdown has no other way to say.
fn write_object(out: &mut String, object: &InlineObject) {
  if let Some(width) = object.width {
    out.push_str("<img src=\"");
    escape_attribute(out, &object.source);
    out.push_str("\" alt=\"");
    escape_attribute(out, &object.label);
    out.push_str(&format!("\" width=\"{width}\">"));
    return;
  }
  out.push_str("![");
  escape(out, &object.label.replace(['\n', '\r'], " "), false, 0, &[]);
  out.push_str("](");
  let source = object.source.as_ref();
  // A source with a space or a bracket in it, or none at all, is written between angle brackets.
  let pointed = source.is_empty()
    || source
      .chars()
      .any(|ch| ch.is_whitespace() || ch.is_control() || matches!(ch, '(' | ')' | '<' | '>'));
  if pointed {
    out.push('<');
  }
  for (at, character) in source.char_indices() {
    match character {
      '\\' | '<' | '>' => out.push('\\'),
      // What would read as a character reference (`&copy;`) is kept as it is written.
      '&' if reads_as_entity(&source[at..]) => out.push('\\'),
      '\n' | '\r' => {
        out.push(' ');
        continue;
      }
      _ => {}
    }
    out.push(character);
  }
  if pointed {
    out.push('>');
  }
  out.push(')');
}

/// Whether `text`, which starts with `&`, starts with what Markdown reads as a character reference.
fn reads_as_entity(text: &str) -> bool {
  let body = &text[1..];
  let end = body
    .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '#'))
    .unwrap_or(body.len());
  end > 0 && body[end..].starts_with(';')
}

/// The marks of a run in the order they nest in the source, the outermost first.
#[derive(Clone, PartialEq)]
enum Mark {
  Link(Arc<str>),
  Bold,
  Italic,
  Strikethrough,
  Underline,
  Code,
}

impl Mark {
  fn of(style: &SpanStyle) -> Vec<Self> {
    let mut marks = Vec::new();
    if let Some(target) = &style.link {
      marks.push(Self::Link(target.clone()));
    }
    for (on, mark) in [
      (style.bold, Self::Bold),
      (style.italic, Self::Italic),
      (style.strikethrough, Self::Strikethrough),
      (style.underline, Self::Underline),
      (style.code, Self::Code),
    ] {
      if on {
        marks.push(mark);
      }
    }
    marks
  }

  fn open(&self, out: &mut String) {
    out.push_str(match self {
      Self::Link(_) => "[",
      Self::Bold => "**",
      Self::Italic => "*",
      Self::Strikethrough => "~~",
      Self::Underline => "<u>",
      Self::Code => "`",
    });
  }

  fn close(&self, out: &mut String) {
    match self {
      Self::Link(target) => {
        out.push_str("](");
        out.push_str(target);
        out.push(')');
      }
      Self::Underline => out.push_str("</u>"),
      mark => mark.open(out),
    }
  }
}

/// Writes a paragraph's text with its marks. A mark opens after the white
/// space its run starts with and closes before the white space it ends with,
/// since the source's emphasis does not take in the space at its edges.
/// `objects` are the paragraph's, their bytes counted from its start: each
/// is written where its character stands.
fn write_inline(out: &mut String, text: &StyledText, objects: &[InlineObject], body: bool) {
  let mut start = 0;
  let runs: Vec<(usize, &str, Vec<Mark>)> = text
    .runs()
    .into_iter()
    .map(|(run, style)| {
      // A run of white space alone carries no mark of its own.
      let marks = if run.trim().is_empty() {
        Vec::new()
      } else {
        Mark::of(&style)
      };
      let at = start;
      start += run.len();
      (at, run, marks)
    })
    .collect();
  let mut open: Vec<Mark> = Vec::new();
  let line_start = out.len();
  for (index, (at, run, marks)) in runs.iter().enumerate() {
    let core = run.trim();
    let lead = &run[..run.len() - run.trim_start().len()];
    let core_at = at + lead.len();
    let trail = &run[run.trim_end().len()..];
    if core.is_empty() {
      // Marks that go on over the white space stay open; the others have closed before it.
      out.push_str(run);
      continue;
    }
    out.push_str(lead);
    let shared = open.iter().zip(marks).take_while(|(a, b)| a == b).count();
    for mark in open.drain(shared..).rev() {
      mark.close(out);
    }
    for mark in &marks[shared..] {
      // A `!` right before a link would make an image of it.
      if matches!(mark, Mark::Link(_)) && out.ends_with('!') {
        out.pop();
        out.push_str("\\!");
      }
      mark.open(out);
      open.push(mark.clone());
    }
    if marks.last() == Some(&Mark::Code) {
      // Code is written as it is and reads no image: an object in it is its label.
      let inside = slice_objects(objects, core_at..core_at + core.len());
      out.push_str(&text_with_labels(core, &inside));
    } else {
      escape(out, core, body && out.len() == line_start, core_at, objects);
    }
    // What the next run with text does not carry closes before this run's trailing space.
    let next = runs[index + 1..].iter().find(|(_, run, _)| !run.trim().is_empty());
    let kept = next.map_or(0, |(_, _, next)| {
      open.iter().zip(next).take_while(|(a, b)| a == b).count()
    });
    for mark in open.drain(kept..).rev() {
      mark.close(out);
    }
    out.push_str(trail);
  }
}

/// Writes `text` with what the source would read as markup escaped; at the
/// start of a body paragraph, also what would make it a heading, a quote or
/// an item. `text` starts at the byte `at` of its paragraph, whose `objects`
/// are written as images where their characters stand.
fn escape(out: &mut String, text: &str, paragraph_start: bool, at: usize, objects: &[InlineObject]) {
  if paragraph_start {
    let digits = text.chars().take_while(char::is_ascii_digit).count();
    let after = text[digits..].chars().next();
    if digits > 0 && matches!(after, Some('.' | ')')) {
      out.push_str(&text[..digits]);
      out.push('\\');
      escape(out, &text[digits..], false, at + digits, objects);
      return;
    }
    let first = text.chars().next();
    let second = text.chars().nth(1);
    if matches!(first, Some('#' | '>' | '-' | '+')) && second.is_none_or(|next| next == ' ' || next == '#') {
      out.push('\\');
    }
  }
  for (offset, character) in text.char_indices() {
    if character == OBJECT_CHARACTER
      && let Some(object) = objects.iter().find(|object| object.at == at + offset)
    {
      write_object(out, object);
      continue;
    }
    if matches!(character, '\\' | '*' | '_' | '`' | '[' | ']' | '<' | '~') {
      out.push('\\');
    }
    out.push(character);
  }
}

#[cfg(test)]
#[path = "../../../tests/input/text_input/rich_text_markdown.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../tests/input/text_input/rich_text_objects.rs"]
mod object_tests;
