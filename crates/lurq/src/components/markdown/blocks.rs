//! The blocks of a document that the design draws on the system's own parts:
//! lists with their numbers, bullets and tasks, admonitions (a callout with a
//! bar), details that open and close, an image with its caption, and the
//! footnotes at the end with the way back from each.

use std::sync::Arc;

use super::{
  MarkdownRenderContext, TaskCallback,
  paragraph::{FOOTNOTE_ANCHOR, REFERENCE_ANCHOR},
  render_block, render_inline_block,
  sizing::FILL_WIDTH,
};
use crate::{
  app::theme::ThemeMarkdown,
  components::{Button, Column, Row, Text},
  layout::{Alignment, layout_kind::Justify, text_style::TextStyle},
  markdown::{MarkdownAdmonitionKind, MarkdownBlock, MarkdownInline, MarkdownListItem},
  node::{Element, NodeUpdate},
  ui::{AlertVariant, Callout, Checkbox, Icon, icons},
};

/// The height of one line of `style`: what a marker beside a block's first line is centered in.
fn line_height(style: &TextStyle) -> f32 {
  style.font_size * style.line_height
}

/// A bullet of a list `depth` lists deep: a dot at the first level, a ring
/// under it, in the marker's color, centered on the item's first line. It
/// is a `-` that draws nothing as well: a selection over the item copies it.
fn bullet(depth: usize, style: &TextStyle, theme: &ThemeMarkdown) -> Element {
  let color = theme.list_marker.apply(style).color;
  let side = theme.bullet_size;
  let mut dot = Column::new().size(side, side).rounded(side / 2.0);
  dot = if depth == 0 {
    dot.background(color)
  } else {
    dot.border_inside(1.0, color)
  };
  let mut unseen = style.clone();
  unseen.color = crate::node::color::Color::new(0, 0, 0, 0);
  crate::components::Stack::new()
    .width(theme.unordered_list_marker_width)
    .height(line_height(style))
    .child(Text::styled("-", unseen).width(theme.unordered_list_marker_width))
    .child(
      Column::new()
        .width(theme.unordered_list_marker_width)
        .height(line_height(style))
        .padding_left(theme.bullet_indent)
        .justify(Justify::Center)
        .child(dot),
    )
    .into()
}

/// An ordered list's number, at the end of its width, in the marker's color.
fn number(text: &str, style: &TextStyle, theme: &ThemeMarkdown) -> Element {
  let mut marker = theme.list_marker.apply(style);
  marker.text_align = crate::layout::text_style::TextAlign::Right;
  Text::styled(text, marker).width(theme.ordered_list_marker_width).into()
}

/// Whether `item` is a task: its first paragraph starts with `[ ]` or `[x]`.
pub(super) fn task_of(item: &MarkdownListItem) -> Option<bool> {
  match item.blocks.first() {
    Some(MarkdownBlock::Paragraph(inlines)) => match inlines.first() {
      Some(MarkdownInline::TaskListMarker(checked)) => Some(*checked),
      _ => None,
    },
    _ => None,
  }
}

/// The blocks of a task's item without its `[ ]`: the box stands for it.
fn without_marker(item: &MarkdownListItem) -> Vec<MarkdownBlock> {
  let mut blocks = item.blocks.clone();
  if let Some(MarkdownBlock::Paragraph(inlines)) = blocks.first_mut() {
    inlines.remove(0);
    // The space after the marker.
    if let Some(MarkdownInline::Text(text)) = inlines.first_mut() {
      let trimmed = text.trim_start().to_owned();
      *text = trimmed;
    }
  }
  blocks
}

/// The words of a task, which name its box to what reads the tree.
fn task_words(blocks: &[MarkdownBlock]) -> String {
  match blocks.first() {
    Some(MarkdownBlock::Paragraph(inlines)) => {
      crate::markdown::markdown_inline_rich_text(inlines, &TextStyle::default(), &ThemeMarkdown::default())
        .into_iter()
        .map(|span| span.text)
        .collect()
    }
    _ => String::new(),
  }
}

/// The items of a list: numbered, bulleted or tasks, each marker beside its
/// item's first line and its blocks beside it.
pub(super) fn render_items(
  ordered: bool,
  start: u64,
  items: &[MarkdownListItem],
  base_style: &TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext<'_>,
) -> Element {
  let tasks = items.iter().any(|item| task_of(item).is_some());
  let spacing = if tasks {
    theme.task_item_spacing
  } else {
    theme.list_item_spacing
  };
  let depth = render_ctx.depth.get();
  let mut list = render_ctx
    .width
    .blocks(Column::new().overflow_visible().spacing(spacing));
  render_ctx.depth.set(depth + 1);
  for (index, item) in items.iter().enumerate() {
    let checked = task_of(item);
    let item_style = match checked {
      Some(true) => theme.task_done.apply(base_style),
      _ => base_style.clone(),
    };
    let blocks = match checked {
      Some(_) => without_marker(item),
      None => item.blocks.clone(),
    };
    let marker = match checked {
      Some(checked) => task_box(checked, &task_words(&blocks), &item_style, render_ctx),
      None if ordered => number(&format!("{}.", start + index as u64), &item_style, theme),
      None => bullet(depth, &item_style, theme),
    };
    let gap = if ordered && checked.is_none() {
      theme.list_marker_gap
    } else {
      theme.bullet_gap
    };
    let mut body = render_ctx.width.row_body(
      Column::new()
        .overflow_visible()
        .spacing(theme.nested_block_spacing)
        .min_width(0.0),
    );
    for block in &blocks {
      body = body.child(render_block(block, &item_style, theme, render_ctx));
    }
    list = list.child(
      render_ctx.width.row(
        Row::new()
          .overflow_visible()
          .spacing(gap)
          .align_items(Alignment::Start)
          .child(marker)
          .child(body),
      ),
    );
  }
  render_ctx.depth.set(depth);
  list.into()
}

/// A task's box, the checkbox's look, centered on its first line: read only,
/// unless the app takes its change.
fn task_box(checked: bool, words: &str, style: &TextStyle, render_ctx: &MarkdownRenderContext<'_>) -> Element {
  let index = render_ctx.tasks.get();
  render_ctx.tasks.set(index + 1);
  let mut checkbox = Checkbox::new(words).label_hidden(true).checked(checked);
  let callback: Option<TaskCallback> = render_ctx.on_task_change.cloned();
  if let Some(callback) = &callback {
    let callback = callback.clone();
    checkbox = checkbox.on_change(move |checked| callback(index, checked));
  }
  let mut element = checkbox.build(&render_ctx.ui);
  if callback.is_none() {
    // Read only: it takes neither the focus nor the pointer, and says it is checked.
    NodeUpdate::inert(&mut *element.node, true);
  }
  Column::new()
    .height(line_height(style))
    .justify(Justify::Center)
    .child(element)
    .into()
}

/// An admonition: a callout with a bar, its icon and its title in the tone of its kind.
pub(super) fn render_admonition(
  kind: MarkdownAdmonitionKind,
  blocks: &[MarkdownBlock],
  base_style: &TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext<'_>,
) -> Element {
  let (variant, icon, title) = match kind {
    MarkdownAdmonitionKind::Note => (AlertVariant::Info, icons::INFO, "Note"),
    MarkdownAdmonitionKind::Tip => (AlertVariant::Success, icons::LIGHTBULB, "Tip"),
    MarkdownAdmonitionKind::Important => (AlertVariant::Neutral, icons::MEGAPHONE, "Important"),
    MarkdownAdmonitionKind::Warning => (AlertVariant::Warning, icons::WARNING, "Warning"),
    MarkdownAdmonitionKind::Caution => (AlertVariant::Destructive, icons::ALERT_OCTAGON, "Caution"),
  };
  // Its text is the callout's: 14 at a line height of 1.5, in the design's 15.
  let mut style = base_style.clone();
  style.font_size *= 14.0 / 15.0;
  style.line_height = 1.5;
  let children: Vec<Element> = blocks
    .iter()
    .map(|block| render_block(block, &style, theme, render_ctx))
    .collect();
  render_ctx.width.leaf(
    Callout::new("")
      .title(title)
      .variant(variant)
      .icon(icon)
      .bar(true)
      .children(children)
      .build(&render_ctx.ui),
  )
}

/// A `<details>` block: its summary, which opens and closes it, over what it
/// holds while it is open.
pub(super) fn render_details(
  summary: &str,
  open: bool,
  blocks: &[MarkdownBlock],
  base_style: &TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext<'_>,
) -> Element {
  let ordinal = render_ctx.details.get();
  render_ctx.details.set(ordinal + 1);
  // A person's press turns it from how it began: the ordinals the document keeps are the turned ones.
  let open = open != render_ctx.details_turned.contains(&ordinal);
  let summary_style = theme.details_summary.apply(base_style);
  let chevron = if open {
    icons::CHEVRON_DOWN
  } else {
    icons::CHEVRON_RIGHT
  };
  let summary_box = &theme.details_summary_box;
  let mut head = Button::empty()
    .tab_index(0)
    .width(FILL_WIDTH)
    .justify(Justify::Start)
    .align_items(Alignment::Center)
    .spacing(8.0)
    .describe("a11y_expanded", if open { "true" } else { "false" })
    .describe("a11y_name", Arc::<str>::from(summary))
    .child(
      Icon::new(chevron)
        .size(16.0)
        .build(&render_ctx.ui)
        .map_node(|node| node.foreground(summary_style.color)),
    )
    .child(Text::styled(summary, summary_style).flex(1.0));
  if let Some(padding) = summary_box.padding {
    head = head.padding(padding);
  }
  if let Some(background) = summary_box.background.clone() {
    head = head.background(background);
  }
  if open && let (Some(width), Some(color)) = (summary_box.border_width, summary_box.border_color.clone()) {
    head = head.border_bottom(crate::node::border::Border::inside(width, color));
  }
  if let Some(turned) = render_ctx.turn_details.clone() {
    head = head.on_click(move |_| {
      turned.update(|turned| match turned.iter().position(|at| *at == ordinal) {
        Some(index) => {
          turned.remove(index);
        }
        None => turned.push(ordinal),
      })
    });
  }
  let mut column = render_ctx.width.blocks(Column::new()).child(head);
  if open {
    let mut content = render_ctx
      .width
      .blocks(Column::new().overflow_visible().spacing(theme.nested_block_spacing))
      .padding(theme.details_padding);
    let mut style = base_style.clone();
    style.font_size *= 14.0 / 15.0;
    style.line_height = 1.5;
    for block in blocks {
      content = content.child(render_block(block, &style, theme, render_ctx));
    }
    column = column.child(content);
  } else {
    // What it holds is still the document's: it renders, for the text the
    // document's highlights index, and draws nothing.
    let checkpoint = render_ctx.links.checkpoint();
    for block in blocks {
      let _ = render_block(block, base_style, theme, render_ctx);
    }
    render_ctx.links.restore(checkpoint);
  }
  super::apply_column_box(column, &theme.details_box, true).into()
}

/// An image that stands alone, with its caption: the image where the app
/// draws images, or else a box of the image's place, and under it its title,
/// or its words.
pub(super) fn render_figure(
  image: Element,
  caption: Option<&[MarkdownInline]>,
  title: &str,
  base_style: &TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext<'_>,
) -> Element {
  let mut figure = render_ctx
    .width
    .blocks(
      Column::new()
        .overflow_visible()
        .spacing(8.0)
        .align_items(Alignment::Center),
    )
    .child(image);
  let caption_style = theme.image_caption.apply(base_style);
  if !title.is_empty() {
    figure = figure.child(Text::styled(title, caption_style));
  } else if let Some(words) = caption.filter(|words| !words.is_empty()) {
    figure = figure.child(render_inline_block(words, caption_style, theme, render_ctx));
  }
  figure.into()
}

/// The place of an image the app does not draw: a box of the image's height
/// with an image's glyph in it.
pub(super) fn image_place(
  base_style: &TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext<'_>,
) -> Element {
  let glyph = theme.image_caption.apply(base_style).color;
  let place = Column::new()
    .width(FILL_WIDTH)
    .height(theme.image_height)
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(
      Icon::new(icons::IMAGE)
        .size(28.0)
        .build(&render_ctx.ui)
        .map_node(|node| node.foreground(glyph)),
    );
  super::apply_column_box(place, &theme.image_box, true).into()
}

/// The footnotes at the end of the document: a short rule over them, and
/// each by its number, with the way back to where the text refers to it.
pub(super) fn render_footnotes(
  footnotes: &[(usize, &str, &[MarkdownBlock])],
  base_style: &TextStyle,
  theme: &ThemeMarkdown,
  render_ctx: &MarkdownRenderContext<'_>,
) -> Element {
  let style = theme.footnote.apply(base_style);
  let mut section = render_ctx
    .width
    .blocks(Column::new().overflow_visible().spacing(8.0))
    .child(
      Column::new()
        .size(theme.footnotes_rule_width, 1.0)
        .background(theme.rule.background.clone().unwrap_or_else(|| style.color.into())),
    );
  for (number, label, blocks) in footnotes {
    let mut body = render_ctx.width.row_body(
      Column::new()
        .overflow_visible()
        .spacing(theme.nested_block_spacing)
        .min_width(0.0),
    );
    for block in *blocks {
      body = body.child(render_block(block, &style, theme, render_ctx));
    }
    let mut row = Row::new()
      .overflow_visible()
      .spacing(6.0)
      .align_items(Alignment::Start)
      .child(Text::styled(&format!("{number}."), style.clone()))
      .child(body);
    // The way back: to the paragraph that first refers to the footnote.
    if let Some(jump) = render_ctx.links.jump() {
      let target = format!("{REFERENCE_ANCHOR}{label}");
      let back = Button::empty()
        .tab_index(0)
        .describe("a11y_name", Arc::<str>::from(format!("Back to the reference {number}")))
        .height(line_height(&style))
        .align_items(Alignment::Center)
        .child(
          Icon::new(icons::ENTER)
            .size(12.0)
            .build(&render_ctx.ui)
            .map_node(|node| node.foreground(style.color)),
        )
        .on_click(move |_| jump(&target));
      row = row.child(back);
    }
    let mut element: Element = render_ctx.width.row(row).into();
    NodeUpdate::ref_element(
      &mut *element.node,
      render_ctx.links.anchor(&format!("{FOOTNOTE_ANCHOR}{label}")),
    );
    section = section.child(element);
  }
  section.into()
}
