//! A document as a styled input edits it: its text, the marks over it and
//! what each of its paragraphs is; read from Markdown and written as it.
use std::sync::Arc;

use super::{ParagraphKind, SpanStyle, StyledSpan, StyledText, normalize, normalize_kinds, paragraph_ranges};
use crate::{
  app::component::{DevtoolsFormatter, DevtoolsInspectable},
  markdown::{MarkdownBlock, MarkdownHeadingLevel, MarkdownInline, parse_markdown},
};

/// Formatted text: what a styled input with paragraphs holds in its three
/// signals, as one value. Its paragraphs are the lines of `text`; `spans` are
/// the marks over it as UTF-8 byte ranges; `paragraphs` says what each line
/// is.
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
}

impl DevtoolsInspectable for RichText {
  fn inspect(&self, formatter: &mut DevtoolsFormatter<'_>) {
    formatter.value_debug(self);
  }
}

impl RichText {
  /// `text` with `spans` and the kinds of its paragraphs, put in the form an
  /// input keeps: the spans canonical (see [`StyledText::new`]), one kind for
  /// each line of the text, body where none was given.
  pub fn new(text: impl Into<String>, spans: Vec<StyledSpan>, paragraphs: Vec<ParagraphKind>) -> Self {
    let text = text.into();
    let spans = normalize(&text, spans);
    let paragraphs = normalize_kinds(paragraphs, paragraph_ranges(&text).len());
    Self {
      text,
      spans,
      paragraphs,
    }
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

  /// How many words the text has: its runs of what is not white space.
  pub fn word_count(&self) -> usize {
    self.text.split_whitespace().count()
  }

  /// Each paragraph: its kind, and its text with its marks.
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
  /// an image is its alternative text linked to its source; a table, a rule
  /// and raw HTML are left out.
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
  /// `[a link](target)`, `<u>under</u>`). What [`Self::to_markdown`] writes of formatted text reads as Markdown;
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

  /// Whether Markdown reads a mark in `text`: only a text with one of the characters that open a mark is read.
  fn has_inline_marks(text: &str) -> bool {
    text.contains(['*', '_', '`', '~', '[', '<']) && !Self::from_markdown(text).spans.is_empty()
  }

  /// The document as Markdown, which [`Self::from_markdown`] reads back as
  /// the same document. An underline is written `<u>…</u>`.
  pub fn to_markdown(&self) -> String {
    let blocks = self.blocks();
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
          (ParagraphKind::Code, ParagraphKind::Code) => "\n",
          (ParagraphKind::Bullet, ParagraphKind::Bullet) | (ParagraphKind::Ordered, ParagraphKind::Ordered) => "\n",
          (ParagraphKind::Quote, ParagraphKind::Quote) => "\n>\n",
          _ => "\n\n",
        });
      }
      if *kind == ParagraphKind::Code {
        if !fenced {
          out.push_str("```\n");
          fenced = true;
        }
        out.push_str(&text.text);
        if blocks.get(index + 1).is_none_or(|next| next.0 != ParagraphKind::Code) {
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
        _ => {}
      }
      write_inline(&mut out, text, *kind == ParagraphKind::Body);
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
          for block in &entry.blocks {
            self.block(block, item);
          }
        }
      }
      MarkdownBlock::CodeBlock { text, .. } | MarkdownBlock::Math { text } => {
        for line in text.strip_suffix('\n').unwrap_or(text).split('\n') {
          self.start(ParagraphKind::Code);
          self.text.push_str(line.trim_end_matches('\r'));
        }
      }
      MarkdownBlock::FootnoteDefinition { blocks, .. } => {
        for block in blocks {
          self.block(block, kind);
        }
      }
      MarkdownBlock::Table { .. } | MarkdownBlock::Html(_) | MarkdownBlock::ThematicBreak => {}
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
          self.inlines(alt, &style.clone().link(destination.as_str()));
        }
        MarkdownInline::Html(html) => match html.trim().to_ascii_lowercase().as_str() {
          "<u>" => style.underline = true,
          "</u>" => style.underline = false,
          "<br>" | "<br/>" | "<br />" => self.push("\n", &style),
          _ => {}
        },
        MarkdownInline::FootnoteReference(label) => self.push(&format!("[^{label}]"), &style),
        MarkdownInline::SoftBreak => self.push(" ", &style),
        MarkdownInline::HardBreak => self.push("\n", &style),
        MarkdownInline::TaskListMarker(checked) => self.push(if *checked { "[x] " } else { "[ ] " }, &style),
      }
    }
  }

  fn finish(self) -> RichText {
    RichText::new(self.text, self.spans, self.kinds)
  }
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
fn write_inline(out: &mut String, text: &StyledText, body: bool) {
  let runs: Vec<(&str, Vec<Mark>)> = text
    .runs()
    .into_iter()
    .map(|(run, style)| {
      // A run of white space alone carries no mark of its own.
      let marks = if run.trim().is_empty() {
        Vec::new()
      } else {
        Mark::of(&style)
      };
      (run, marks)
    })
    .collect();
  let mut open: Vec<Mark> = Vec::new();
  let line_start = out.len();
  for (index, (run, marks)) in runs.iter().enumerate() {
    let core = run.trim();
    let lead = &run[..run.len() - run.trim_start().len()];
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
      mark.open(out);
      open.push(mark.clone());
    }
    if marks.last() == Some(&Mark::Code) {
      out.push_str(core);
    } else {
      escape(out, core, body && out.len() == line_start);
    }
    // What the next run with text does not carry closes before this run's trailing space.
    let next = runs[index + 1..].iter().find(|(run, _)| !run.trim().is_empty());
    let kept = next.map_or(0, |(_, next)| open.iter().zip(next).take_while(|(a, b)| a == b).count());
    for mark in open.drain(kept..).rev() {
      mark.close(out);
    }
    out.push_str(trail);
  }
}

/// Writes `text` with what the source would read as markup escaped; at the
/// start of a body paragraph, also what would make it a heading, a quote or
/// an item.
fn escape(out: &mut String, text: &str, paragraph_start: bool) {
  if paragraph_start {
    let digits = text.chars().take_while(char::is_ascii_digit).count();
    let after = text[digits..].chars().next();
    if digits > 0 && matches!(after, Some('.' | ')')) {
      out.push_str(&text[..digits]);
      out.push('\\');
      escape(out, &text[digits..], false);
      return;
    }
    let first = text.chars().next();
    let second = text.chars().nth(1);
    if matches!(first, Some('#' | '>' | '-' | '+')) && second.is_none_or(|next| next == ' ' || next == '#') {
      out.push('\\');
    }
  }
  for character in text.chars() {
    if matches!(character, '\\' | '*' | '_' | '`' | '[' | ']' | '<' | '~') {
      out.push('\\');
    }
    out.push(character);
  }
}

#[cfg(test)]
#[path = "../../../tests/input/text_input/rich_text_markdown.rs"]
mod tests;
