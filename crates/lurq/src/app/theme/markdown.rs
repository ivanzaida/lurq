//! How a Markdown document looks: the theme's `ThemeMarkdown`.
//!
//! Its colors are the palette's roles (or a color an app gives), resolved
//! with the palette of the theme a document is drawn in, so a document
//! follows the theme, light and dark, and a theme an app makes. Its sizes are
//! the design's (`pencil/lurq-design-system.pen`, Markdown), relative to the
//! document's own text where they are of type.

use std::{sync::Arc, time::Duration};

use super::{PaletteColor, ThemePalette};
use crate::{
  layout::{
    scrollbar::ScrollBarStyle,
    text_style::{FontFeatures, FontStyle, FontWeight, TextStrikethrough, TextStyle, TextUnderline},
  },
  node::{TextColor, border::BorderRadius, padding::Padding},
};

/// What a part of a document changes of the text it is in: each set value
/// replaces the base's, and the rest is kept.
#[derive(Clone, Default, PartialEq)]
pub struct MarkdownTextStyle {
  pub font_family: Option<Arc<str>>,
  pub font_size: Option<f32>,
  pub font_size_scale: Option<f32>,
  pub line_height: Option<f32>,
  /// Letter spacing in logical pixels; not multiplied by `font_size_scale`.
  pub letter_spacing: Option<f32>,
  /// OpenType feature settings; replaces the base style's settings.
  pub font_features: Option<FontFeatures>,
  pub weight: Option<FontWeight>,
  pub style: Option<FontStyle>,
  /// The text's color: a palette role, which follows the theme, or a color.
  pub color: Option<TextColor>,
  /// Underlines the text; replaces the base style's underline.
  pub underline: Option<TextUnderline>,
  /// Draws a line through the text; replaces the base style's.
  pub strikethrough: Option<TextStrikethrough>,
}

impl MarkdownTextStyle {
  pub fn new() -> Self {
    Self::default()
  }

  /// `base` with what this style sets. A palette role is drawn as the
  /// document's theme resolves it; on its own (outside a document) it keeps
  /// the base's color.
  pub fn apply(&self, base: &TextStyle) -> TextStyle {
    let mut style = base.clone();
    if let Some(font_family) = &self.font_family {
      style.font_family = font_family.clone();
    }
    if let Some(font_size) = self.font_size {
      style.font_size = font_size;
    }
    if let Some(font_size_scale) = self.font_size_scale {
      style.font_size *= font_size_scale;
    }
    if let Some(line_height) = self.line_height {
      style.line_height = line_height;
    }
    if let Some(letter_spacing) = self.letter_spacing {
      style.letter_spacing = letter_spacing;
    }
    if let Some(font_features) = &self.font_features {
      style.font_features = font_features.clone();
    }
    if let Some(weight) = self.weight {
      style.weight = weight;
    }
    if let Some(font_style) = self.style {
      style.style = font_style;
    }
    if let Some(TextColor::Color(color)) = &self.color {
      style.color = *color;
    }
    if let Some(underline) = self.underline {
      style.underline = Some(underline);
    }
    if let Some(strikethrough) = self.strikethrough {
      style.strikethrough = Some(strikethrough);
    }
    style
  }

  /// This style with its color resolved by `palette`.
  pub fn resolve(&self, palette: &ThemePalette) -> Self {
    Self {
      color: resolve(&self.color, palette),
      ..self.clone()
    }
  }
}

/// A palette role in `color` turned into the color `palette` gives it.
fn resolve(color: &Option<TextColor>, palette: &ThemePalette) -> Option<TextColor> {
  match color {
    Some(TextColor::Palette(role)) => palette.try_get(role.clone()).map(TextColor::Color),
    other => other.clone(),
  }
}

#[derive(Clone, Default, PartialEq)]
pub struct MarkdownInlineStyle {
  pub text: MarkdownTextStyle,
}

impl MarkdownInlineStyle {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn apply(&self, base: &TextStyle) -> TextStyle {
    self.text.apply(base)
  }

  /// This style with its color resolved by `palette`.
  pub fn resolve(&self, palette: &ThemePalette) -> Self {
    Self {
      text: self.text.resolve(palette),
    }
  }
}

/// The box of a part of a document: its fill, its edge, its radius and its padding.
#[derive(Clone, Default, PartialEq)]
pub struct MarkdownBlockStyle {
  /// The fill: a palette role or a color.
  pub background: Option<TextColor>,
  /// The edge's color: a palette role or a color.
  pub border_color: Option<TextColor>,
  pub border_width: Option<f32>,
  pub radius: Option<f32>,
  /// The room between the box's edges and its content: [`Padding::all`] for
  /// every side, [`Padding::symmetric`] for horizontal and vertical.
  pub padding: Option<Padding>,
}

impl MarkdownBlockStyle {
  pub fn new() -> Self {
    Self::default()
  }

  /// This box with its colors resolved by `palette`.
  pub fn resolve(&self, palette: &ThemePalette) -> Self {
    Self {
      background: resolve(&self.background, palette),
      border_color: resolve(&self.border_color, palette),
      ..self.clone()
    }
  }
}

/// How a Markdown link looks while the pointer is on it or while it has
/// keyboard focus. Every unset value keeps the link's look at rest. These
/// change only colors and fills, never the text's metrics, so a link does not
/// move or rewrap when its state changes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MarkdownLinkState {
  /// The color of the link's words.
  pub color: Option<TextColor>,
  /// The color of the words' underline, where they have one.
  pub underline_color: Option<TextColor>,
  /// A fill behind the link's words, one rectangle per laid-out line, with
  /// the link's corner radius.
  pub background: Option<TextColor>,
  /// The color of what the link presentation puts after the words (its
  /// trailing runs).
  pub trailing_color: Option<TextColor>,
}

impl MarkdownLinkState {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn color(mut self, color: impl Into<TextColor>) -> Self {
    self.color = Some(color.into());
    self
  }

  pub fn underline_color(mut self, color: impl Into<TextColor>) -> Self {
    self.underline_color = Some(color.into());
    self
  }

  pub fn background(mut self, color: impl Into<TextColor>) -> Self {
    self.background = Some(color.into());
    self
  }

  pub fn trailing_color(mut self, color: impl Into<TextColor>) -> Self {
    self.trailing_color = Some(color.into());
    self
  }

  /// This look with its colors resolved by `palette`.
  pub fn resolve(&self, palette: &ThemePalette) -> Self {
    Self {
      color: resolve(&self.color, palette),
      underline_color: resolve(&self.underline_color, palette),
      background: resolve(&self.background, palette),
      trailing_color: resolve(&self.trailing_color, palette),
    }
  }
}

/// The ring a Markdown link shows while it has keyboard focus (CSS
/// `:focus-visible`), drawn around each laid-out line of its words, outside
/// them on all four sides. It is drawn over the text, not clipped by the
/// paragraph, so it shows in full even where it reaches past the paragraph's
/// box.
#[derive(Clone, Debug, PartialEq)]
pub struct MarkdownFocusRing {
  /// The ring's line width.
  pub width: f32,
  /// The gap between the words' box and the ring's inner edge.
  pub offset: f32,
  /// A palette role or a color.
  pub color: TextColor,
  /// Corner radius of the ring's outer edge.
  pub radius: f32,
}

impl MarkdownFocusRing {
  pub fn new(width: f32, color: impl Into<TextColor>) -> Self {
    Self {
      width,
      offset: 0.0,
      color: color.into(),
      radius: 0.0,
    }
  }

  pub fn offset(mut self, offset: f32) -> Self {
    self.offset = offset;
    self
  }

  pub fn radius(mut self, radius: f32) -> Self {
    self.radius = radius;
    self
  }

  /// This ring with its color resolved by `palette`.
  pub fn resolve(&self, palette: &ThemePalette) -> Self {
    Self {
      color: resolve(&Some(self.color.clone()), palette).unwrap_or_else(|| self.color.clone()),
      ..self.clone()
    }
  }
}

#[derive(Clone, PartialEq)]
pub struct ThemeMarkdown {
  pub document_spacing: f32,
  /// The room between two items of an ordered or a bulleted list.
  pub list_item_spacing: f32,
  /// The room between two tasks of a task list.
  pub task_item_spacing: f32,
  pub nested_block_spacing: f32,
  pub code_block_spacing: f32,
  pub blockquote_gap: f32,
  /// The room between an ordered list's number and its item.
  pub list_marker_gap: f32,
  /// The room between a bullet or a task's box and its item.
  pub bullet_gap: f32,
  /// The room before a bullet, from the start of the list.
  pub bullet_indent: f32,
  /// The side of a bullet: a dot at the first level, a ring under it.
  pub bullet_size: f32,
  pub unordered_list_marker_width: f32,
  /// The width an ordered list's number is set in, at its end.
  pub ordered_list_marker_width: f32,
  pub table_row_spacing: f32,
  pub table_column_spacing: f32,
  pub table_cell_min_width: f32,
  /// The fill of every other body row of a table: `None` for none.
  pub table_stripe: Option<TextColor>,
  pub heading: MarkdownInlineStyle,
  pub heading_1: MarkdownInlineStyle,
  pub heading_2: MarkdownInlineStyle,
  pub heading_3: MarkdownInlineStyle,
  pub heading_4: MarkdownInlineStyle,
  pub heading_5: MarkdownInlineStyle,
  pub heading_6: MarkdownInlineStyle,
  /// The line under a heading of the second level: `None` for none.
  pub heading_2_rule: Option<TextColor>,
  pub strong: MarkdownInlineStyle,
  pub emphasis: MarkdownInlineStyle,
  pub inline_code: MarkdownInlineStyle,
  /// A key of the keyboard, `<kbd>Ctrl</kbd>`: its type; its box is `ui::Kbd`'s.
  pub kbd: MarkdownInlineStyle,
  pub code_block: MarkdownInlineStyle,
  pub code_block_label: MarkdownInlineStyle,
  pub link: MarkdownInlineStyle,
  /// A link's look while the pointer is on it.
  pub link_hovered: MarkdownLinkState,
  /// A link's look while it has keyboard focus (focus-visible).
  pub link_focused: MarkdownLinkState,
  /// The ring around a link with keyboard focus; `None` draws none.
  pub link_focus_ring: Option<MarkdownFocusRing>,
  /// Corner radii of a link's hover and focus fills.
  pub link_radius: BorderRadius,
  /// The text of a link's tooltip.
  pub link_tooltip: MarkdownInlineStyle,
  /// The box of a link's tooltip.
  pub link_tooltip_box: MarkdownBlockStyle,
  /// How long the pointer rests on a link before its tooltip shows. Keyboard
  /// focus shows it at once.
  pub link_tooltip_delay: Duration,
  pub strikethrough: MarkdownInlineStyle,
  pub list_marker: MarkdownInlineStyle,
  /// The words of a task that is done.
  pub task_done: MarkdownInlineStyle,
  pub blockquote_marker: MarkdownInlineStyle,
  pub table_header: MarkdownInlineStyle,
  pub table_cell: MarkdownInlineStyle,
  pub table_marker: MarkdownInlineStyle,
  pub code_block_box: MarkdownBlockStyle,
  /// The horizontal scrollbar of a code block whose lines are wider than its
  /// box, drawn over the box's bottom padding while the block overflows.
  /// `None` draws the app theme's scrollbar.
  pub code_block_scrollbar: Option<ScrollBarStyle>,
  pub blockquote_box: MarkdownBlockStyle,
  pub table_box: MarkdownBlockStyle,
  pub table_header_box: MarkdownBlockStyle,
  pub table_cell_box: MarkdownBlockStyle,
  /// The line between blocks (`---`): its color is the box's fill.
  pub rule: MarkdownBlockStyle,
  /// The box an image stands in where the app draws no image: its fill, its edge, its radius.
  pub image_box: MarkdownBlockStyle,
  /// The height of that box.
  pub image_height: f32,
  /// The caption under an image: its title, or its words.
  pub image_caption: MarkdownInlineStyle,
  /// The box of a `<details>` block.
  pub details_box: MarkdownBlockStyle,
  /// The box of its summary, which opens and closes it.
  pub details_summary_box: MarkdownBlockStyle,
  /// The words of its summary.
  pub details_summary: MarkdownInlineStyle,
  /// The room around what it holds.
  pub details_padding: Padding,
  /// The number of a footnote, where the text refers to it.
  pub footnote_reference: MarkdownInlineStyle,
  /// The text of a footnote at the end of the document.
  pub footnote: MarkdownInlineStyle,
  /// The line over the footnotes: its width, and its color.
  pub footnotes_rule_width: f32,
}

impl ThemeMarkdown {
  pub fn new() -> Self {
    Self::default()
  }

  /// This theme with every color resolved by `palette`: what a document
  /// draws with, in the theme it is drawn in.
  pub fn resolve(&self, palette: &ThemePalette) -> Self {
    let inline = |style: &MarkdownInlineStyle| style.resolve(palette);
    let block = |style: &MarkdownBlockStyle| style.resolve(palette);
    Self {
      table_stripe: resolve(&self.table_stripe, palette),
      heading: inline(&self.heading),
      heading_1: inline(&self.heading_1),
      heading_2: inline(&self.heading_2),
      heading_3: inline(&self.heading_3),
      heading_4: inline(&self.heading_4),
      heading_5: inline(&self.heading_5),
      heading_6: inline(&self.heading_6),
      heading_2_rule: resolve(&self.heading_2_rule, palette),
      strong: inline(&self.strong),
      emphasis: inline(&self.emphasis),
      inline_code: inline(&self.inline_code),
      kbd: inline(&self.kbd),
      code_block: inline(&self.code_block),
      code_block_label: inline(&self.code_block_label),
      link: inline(&self.link),
      link_hovered: self.link_hovered.resolve(palette),
      link_focused: self.link_focused.resolve(palette),
      link_focus_ring: self.link_focus_ring.as_ref().map(|ring| ring.resolve(palette)),
      link_tooltip: inline(&self.link_tooltip),
      link_tooltip_box: block(&self.link_tooltip_box),
      strikethrough: inline(&self.strikethrough),
      list_marker: inline(&self.list_marker),
      task_done: inline(&self.task_done),
      blockquote_marker: inline(&self.blockquote_marker),
      table_header: inline(&self.table_header),
      table_cell: inline(&self.table_cell),
      table_marker: inline(&self.table_marker),
      code_block_box: block(&self.code_block_box),
      blockquote_box: block(&self.blockquote_box),
      table_box: block(&self.table_box),
      table_header_box: block(&self.table_header_box),
      table_cell_box: block(&self.table_cell_box),
      rule: block(&self.rule),
      image_box: block(&self.image_box),
      image_caption: inline(&self.image_caption),
      details_box: block(&self.details_box),
      details_summary_box: block(&self.details_summary_box),
      details_summary: inline(&self.details_summary),
      footnote_reference: inline(&self.footnote_reference),
      footnote: inline(&self.footnote),
      ..self.clone()
    }
  }
}

/// The design's document (`pencil/lurq-design-system.pen`, Markdown), on
/// the palette's roles. Sizes of type are relative to the document's text,
/// which the design sets at 15 with a line height of 1.65.
impl Default for ThemeMarkdown {
  fn default() -> Self {
    use PaletteColor::{Border, Foreground, Info, Muted, MutedForeground, Popover, PopoverForeground, Ring};
    let color = |role: PaletteColor| Some(TextColor::Palette(role));
    // A size the design gives in pixels, as a share of its text's 15.
    let of_15 = |size: f32| Some(size / 15.0);

    // Headings are of the theme's type scale (`TypographyStyle::Heading1` to
    // `Heading6`): the document says only their colors.
    let mut heading = MarkdownInlineStyle::default();
    heading.text.color = color(Foreground);
    let heading_1 = MarkdownInlineStyle::default();
    let heading_2 = MarkdownInlineStyle::default();
    let heading_3 = MarkdownInlineStyle::default();
    let heading_4 = MarkdownInlineStyle::default();
    let heading_5 = MarkdownInlineStyle::default();
    let mut heading_6 = MarkdownInlineStyle::default();
    heading_6.text.color = color(MutedForeground);

    let mut strong = MarkdownInlineStyle::default();
    strong.text.weight = Some(FontWeight::Bold);

    let mut emphasis = MarkdownInlineStyle::default();
    emphasis.text.style = Some(FontStyle::Italic);

    // Code in a sentence: the mono face at 13, on the inline code's fill (`ui::InlineCode`).
    let mut inline_code = MarkdownInlineStyle::default();
    inline_code.text.font_family = Some(Arc::from("monospace"));
    inline_code.text.font_size_scale = of_15(13.0);
    inline_code.text.color = color(Foreground);

    // A key: the mono face at 11 in the muted foreground, in `ui::Kbd`'s box.
    let mut kbd = MarkdownInlineStyle::default();
    kbd.text.font_family = Some(Arc::from("monospace"));
    kbd.text.font_size_scale = of_15(11.0);
    kbd.text.color = color(MutedForeground);

    let mut code_block = inline_code.clone();
    code_block.text.line_height = Some(1.6);

    let mut code_block_label = code_block.clone();
    code_block_label.text.font_size_scale = of_15(12.0);
    code_block_label.text.color = color(MutedForeground);

    let mut link = MarkdownInlineStyle::default();
    link.text.color = color(Info);
    link.text.weight = Some(FontWeight::Medium);

    let link_focus_ring = MarkdownFocusRing::new(2.0, Ring).offset(1.0).radius(3.0);

    let mut link_tooltip = MarkdownInlineStyle::default();
    link_tooltip.text.font_size_scale = of_15(12.0);
    link_tooltip.text.color = color(PopoverForeground);

    let link_tooltip_box = MarkdownBlockStyle {
      background: color(Popover),
      border_color: color(Border),
      border_width: Some(1.0),
      radius: Some(6.0),
      padding: Some(Padding::symmetric(8.0, 4.0)),
    };

    let mut list_marker = MarkdownInlineStyle::default();
    list_marker.text.color = color(MutedForeground);

    let mut task_done = MarkdownInlineStyle::default();
    task_done.text.color = color(MutedForeground);

    let mut blockquote_marker = MarkdownInlineStyle::default();
    blockquote_marker.text.color = color(MutedForeground);

    // A table: its header 14 at 600, its cells the mono face at 13.
    let mut table_header = MarkdownInlineStyle::default();
    table_header.text.font_size_scale = of_15(14.0);
    table_header.text.weight = Some(FontWeight::SemiBold);
    table_header.text.color = color(Foreground);

    let table_cell = inline_code.clone();

    let mut table_marker = MarkdownInlineStyle::default();
    table_marker.text.color = color(Border);

    let code_block_box = MarkdownBlockStyle {
      background: color(Muted),
      border_color: color(Border),
      border_width: Some(1.0),
      radius: Some(8.0),
      padding: Some(Padding::all(14.0)),
    };

    let blockquote_box = MarkdownBlockStyle {
      background: None,
      border_color: color(Border),
      border_width: Some(3.0),
      radius: Some(0.0),
      padding: Some(Padding::symmetric(14.0, 2.0)),
    };

    let table_box = MarkdownBlockStyle {
      background: None,
      border_color: color(Border),
      border_width: Some(1.0),
      radius: Some(6.0),
      padding: None,
    };

    let table_header_box = MarkdownBlockStyle {
      background: color(Muted),
      border_color: None,
      border_width: None,
      radius: None,
      padding: Some(Padding::symmetric(12.0, 8.0)),
    };

    let table_cell_box = MarkdownBlockStyle {
      background: None,
      border_color: None,
      border_width: None,
      radius: None,
      padding: Some(Padding::symmetric(12.0, 8.0)),
    };

    let rule = MarkdownBlockStyle {
      background: color(Border),
      ..MarkdownBlockStyle::default()
    };

    let image_box = MarkdownBlockStyle {
      background: color(Muted),
      border_color: color(Border),
      border_width: Some(1.0),
      radius: Some(8.0),
      padding: None,
    };

    let mut image_caption = MarkdownInlineStyle::default();
    image_caption.text.font_size_scale = of_15(13.0);
    image_caption.text.color = color(MutedForeground);

    let details_box = MarkdownBlockStyle {
      background: None,
      border_color: color(Border),
      border_width: Some(1.0),
      radius: Some(6.0),
      padding: None,
    };

    let details_summary_box = MarkdownBlockStyle {
      background: color(Muted),
      border_color: color(Border),
      border_width: Some(1.0),
      radius: None,
      padding: Some(Padding::symmetric(12.0, 10.0)),
    };

    let mut details_summary = MarkdownInlineStyle::default();
    details_summary.text.font_size_scale = of_15(14.0);
    details_summary.text.weight = Some(FontWeight::Medium);
    details_summary.text.color = color(Foreground);

    let mut footnote_reference = MarkdownInlineStyle::default();
    footnote_reference.text.font_size_scale = of_15(11.0);
    footnote_reference.text.weight = Some(FontWeight::SemiBold);
    footnote_reference.text.color = color(Info);

    let mut footnote = MarkdownInlineStyle::default();
    footnote.text.font_size_scale = of_15(13.0);
    footnote.text.color = color(MutedForeground);

    Self {
      document_spacing: 16.0,
      list_item_spacing: 6.0,
      task_item_spacing: 8.0,
      nested_block_spacing: 6.0,
      code_block_spacing: 8.0,
      blockquote_gap: 12.0,
      list_marker_gap: 8.0,
      bullet_gap: 10.0,
      bullet_indent: 6.0,
      bullet_size: 5.0,
      unordered_list_marker_width: 11.0,
      ordered_list_marker_width: 18.0,
      table_row_spacing: 0.0,
      table_column_spacing: 0.0,
      table_cell_min_width: 80.0,
      table_stripe: color(Muted),
      heading,
      heading_1,
      heading_2,
      heading_3,
      heading_4,
      heading_5,
      heading_6,
      heading_2_rule: color(Border),
      strong,
      emphasis,
      inline_code,
      kbd,
      code_block,
      code_block_label,
      link,
      link_hovered: MarkdownLinkState::default(),
      link_focused: MarkdownLinkState::default(),
      link_focus_ring: Some(link_focus_ring),
      link_radius: BorderRadius::all(2.0),
      link_tooltip,
      link_tooltip_box,
      link_tooltip_delay: Duration::from_millis(500),
      strikethrough: MarkdownInlineStyle {
        text: MarkdownTextStyle {
          strikethrough: Some(TextStrikethrough::new()),
          color: color(MutedForeground),
          ..MarkdownTextStyle::default()
        },
      },
      list_marker,
      task_done,
      blockquote_marker,
      table_header,
      table_cell,
      table_marker,
      code_block_box,
      // The bar of a code block's own, until the code block is the system's (`ui::CodeBlock`).
      code_block_scrollbar: Some(ScrollBarStyle {
        thumb_color: crate::node::color::Color::new(148, 163, 184, 128),
        ..ScrollBarStyle::thin().insets(4.0, 6.0)
      }),
      blockquote_box,
      table_box,
      table_header_box,
      table_cell_box,
      rule,
      image_box,
      image_height: 150.0,
      image_caption,
      details_box,
      details_summary_box,
      details_summary,
      details_padding: Padding::all(12.0),
      footnote_reference,
      footnote,
      footnotes_rule_width: 120.0,
    }
  }
}
