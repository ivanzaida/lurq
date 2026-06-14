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
      MarkdownInline::Text(text) | MarkdownInline::Html(text) | MarkdownInline::FootnoteReference(text) => {
        push_span(spans, text, base_style.clone())
      }
      MarkdownInline::Code(text) => push_span(spans, text, theme.inline_code.apply(base_style)),
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
