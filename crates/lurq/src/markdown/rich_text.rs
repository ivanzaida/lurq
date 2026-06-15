use super::MarkdownInline;
use crate::{
  app::theme::ThemeMarkdown,
  layout::{quad::RichTextSpan, text_style::TextStyle},
};

pub(crate) fn markdown_inline_rich_text(
  inlines: &[MarkdownInline],
  base_style: &TextStyle,
  theme: &ThemeMarkdown,
) -> Vec<RichTextSpan> {
  let mut spans = Vec::new();
  push_inline_spans(&mut spans, inlines, base_style, theme);
  spans
}

fn push_inline_spans(
  spans: &mut Vec<RichTextSpan>,
  inlines: &[MarkdownInline],
  base_style: &TextStyle,
  theme: &ThemeMarkdown,
) {
  for inline in inlines {
    match inline {
      MarkdownInline::Text(text) => push_span(spans, text, base_style.clone()),
      MarkdownInline::Html(text) => push_span(spans, markdown_html_text(text), base_style.clone()),
      MarkdownInline::FootnoteReference(text) => push_span(spans, format!("[^{text}]"), theme.link.apply(base_style)),
      MarkdownInline::Code(text) => push_span(spans, text, theme.inline_code.apply(base_style)),
      MarkdownInline::Math(text) => push_span(spans, format!("${text}$"), theme.inline_code.apply(base_style)),
      MarkdownInline::Emphasis(children) => {
        let style = theme.emphasis.apply(base_style);
        push_inline_spans(spans, children, &style, theme);
      }
      MarkdownInline::Strong(children) => {
        let style = theme.strong.apply(base_style);
        push_inline_spans(spans, children, &style, theme);
      }
      MarkdownInline::Strikethrough(children) => {
        let style = theme.strikethrough.apply(base_style);
        push_inline_spans(spans, children, &style, theme);
      }
      MarkdownInline::Link { children, .. } => {
        let style = theme.link.apply(base_style);
        push_inline_spans(spans, children, &style, theme);
      }
      MarkdownInline::Image { alt, .. } => push_inline_spans(spans, alt, base_style, theme),
      MarkdownInline::SoftBreak => push_span(spans, " ", base_style.clone()),
      MarkdownInline::HardBreak => push_span(spans, "\n", base_style.clone()),
      MarkdownInline::TaskListMarker(checked) => {
        push_span(spans, if *checked { "[x] " } else { "[ ] " }, base_style.clone());
      }
    }
  }
}

pub(crate) fn markdown_html_text(html: &str) -> String {
  let mut output = String::new();
  let mut in_tag = false;
  let mut chars = html.chars().peekable();
  while let Some(ch) = chars.next() {
    match ch {
      '<' => {
        let mut tag = String::new();
        while let Some(next) = chars.peek().copied() {
          if next == '>' {
            break;
          }
          tag.push(next);
          chars.next();
        }
        in_tag = true;
        if tag.trim_start().starts_with("br") && !output.ends_with('\n') {
          output.push('\n');
        }
      }
      '>' if in_tag => in_tag = false,
      '&' if !in_tag => {
        let entity = read_html_entity(&mut chars);
        match entity.as_deref() {
          Some("amp") => output.push('&'),
          Some("lt") => output.push('<'),
          Some("gt") => output.push('>'),
          Some("quot") => output.push('"'),
          Some("apos") => output.push('\''),
          Some("nbsp") => output.push(' '),
          Some(other) => {
            output.push('&');
            output.push_str(other);
            output.push(';');
          }
          None => output.push('&'),
        }
      }
      _ if !in_tag => output.push(ch),
      _ => {}
    }
  }
  output
}

fn read_html_entity(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Option<String> {
  let mut entity = String::new();
  while let Some(ch) = chars.peek().copied() {
    if ch == ';' {
      chars.next();
      return Some(entity);
    }
    if entity.len() >= 12 || ch.is_whitespace() || ch == '<' || ch == '&' {
      return None;
    }
    entity.push(ch);
    chars.next();
  }
  None
}

fn push_span(spans: &mut Vec<RichTextSpan>, text: impl Into<String>, style: TextStyle) {
  let text = text.into();
  if text.is_empty() {
    return;
  }
  if let Some(last) = spans.last_mut()
    && last.style == style
  {
    last.text.push_str(&text);
    return;
  }
  spans.push(RichTextSpan { text, style });
}
