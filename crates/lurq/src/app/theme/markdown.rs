use std::sync::Arc;

use crate::{
  layout::text_style::{FontStyle, FontWeight, TextStyle},
  node::color::Color,
};

#[derive(Clone, Default, PartialEq)]
pub struct MarkdownTextStyle {
  pub font_family: Option<Arc<str>>,
  pub font_size: Option<f32>,
  pub font_size_scale: Option<f32>,
  pub line_height: Option<f32>,
  pub weight: Option<FontWeight>,
  pub style: Option<FontStyle>,
  pub color: Option<Color>,
}

impl MarkdownTextStyle {
  pub fn new() -> Self {
    Self::default()
  }

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
    if let Some(weight) = self.weight {
      style.weight = weight;
    }
    if let Some(font_style) = self.style {
      style.style = font_style;
    }
    if let Some(color) = self.color {
      style.color = color;
    }
    style
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
}

#[derive(Clone, Default, PartialEq)]
pub struct MarkdownBlockStyle {
  pub background: Option<Color>,
  pub border_color: Option<Color>,
  pub border_width: Option<f32>,
  pub radius: Option<f32>,
  pub padding: Option<f32>,
}

impl MarkdownBlockStyle {
  pub fn new() -> Self {
    Self::default()
  }
}

#[derive(Clone, PartialEq)]
pub struct ThemeMarkdown {
  pub document_spacing: f32,
  pub list_item_spacing: f32,
  pub nested_block_spacing: f32,
  pub code_block_spacing: f32,
  pub blockquote_gap: f32,
  pub list_marker_gap: f32,
  pub unordered_list_marker_width: f32,
  pub ordered_list_marker_width: f32,
  pub table_row_spacing: f32,
  pub table_column_spacing: f32,
  pub table_cell_min_width: f32,
  pub heading: MarkdownInlineStyle,
  pub heading_1: MarkdownInlineStyle,
  pub heading_2: MarkdownInlineStyle,
  pub heading_3: MarkdownInlineStyle,
  pub heading_4: MarkdownInlineStyle,
  pub heading_5: MarkdownInlineStyle,
  pub heading_6: MarkdownInlineStyle,
  pub strong: MarkdownInlineStyle,
  pub emphasis: MarkdownInlineStyle,
  pub inline_code: MarkdownInlineStyle,
  pub code_block: MarkdownInlineStyle,
  pub code_block_label: MarkdownInlineStyle,
  pub link: MarkdownInlineStyle,
  pub strikethrough: MarkdownInlineStyle,
  pub list_marker: MarkdownInlineStyle,
  pub blockquote_marker: MarkdownInlineStyle,
  pub table_header: MarkdownInlineStyle,
  pub table_cell: MarkdownInlineStyle,
  pub table_marker: MarkdownInlineStyle,
  pub code_block_box: MarkdownBlockStyle,
  pub blockquote_box: MarkdownBlockStyle,
  pub table_box: MarkdownBlockStyle,
  pub table_header_box: MarkdownBlockStyle,
  pub table_cell_box: MarkdownBlockStyle,
}

impl ThemeMarkdown {
  pub fn new() -> Self {
    Self::default()
  }
}

impl Default for ThemeMarkdown {
  fn default() -> Self {
    let mut heading = MarkdownInlineStyle::default();
    heading.text.weight = Some(FontWeight::Bold);
    heading.text.line_height = Some(1.15);

    let heading_1 = heading_style(1.85);
    let heading_2 = heading_style(1.55);
    let heading_3 = heading_style(1.3);
    let heading_4 = heading_style(1.15);
    let heading_5 = heading_style(1.0);
    let heading_6 = heading_style(0.95);

    let mut strong = MarkdownInlineStyle::default();
    strong.text.weight = Some(FontWeight::Bold);

    let mut emphasis = MarkdownInlineStyle::default();
    emphasis.text.style = Some(FontStyle::Italic);

    let mut inline_code = MarkdownInlineStyle::default();
    inline_code.text.font_family = Some(Arc::from("monospace"));
    inline_code.text.font_size_scale = Some(0.92);
    inline_code.text.weight = Some(FontWeight::Medium);

    let mut code_block = inline_code.clone();
    code_block.text.line_height = Some(1.45);

    let mut code_block_label = code_block.clone();
    code_block_label.text.font_size_scale = Some(0.82);
    code_block_label.text.color = Some(Color::from_hex("#64748b"));

    let mut link = MarkdownInlineStyle::default();
    link.text.color = Some(Color::from_hex("#2563eb"));

    let mut list_marker = MarkdownInlineStyle::default();
    list_marker.text.weight = Some(FontWeight::Medium);
    list_marker.text.color = Some(Color::from_hex("#64748b"));

    let mut blockquote_marker = MarkdownInlineStyle::default();
    blockquote_marker.text.weight = Some(FontWeight::Bold);
    blockquote_marker.text.color = Some(Color::from_hex("#64748b"));

    let mut table_header = inline_code.clone();
    table_header.text.weight = Some(FontWeight::Bold);

    let table_cell = inline_code.clone();

    let mut table_marker = inline_code.clone();
    table_marker.text.color = Some(Color::from_hex("#64748b"));

    let code_block_box = MarkdownBlockStyle {
      background: Some(Color::from_hex("#0f172a")),
      border_color: Some(Color::from_hex("#334155")),
      border_width: Some(1.0),
      radius: Some(6.0),
      padding: Some(14.0),
    };

    let blockquote_box = MarkdownBlockStyle {
      background: Some(Color::from_hex("#f8fafc")),
      border_color: Some(Color::from_hex("#94a3b8")),
      border_width: Some(3.0),
      radius: Some(0.0),
      padding: Some(12.0),
    };

    let table_box = MarkdownBlockStyle {
      background: None,
      border_color: Some(Color::from_hex("#cbd5e1")),
      border_width: Some(1.0),
      radius: Some(4.0),
      padding: None,
    };

    let table_header_box = MarkdownBlockStyle {
      background: Some(Color::from_hex("#f8fafc")),
      border_color: None,
      border_width: None,
      radius: None,
      padding: Some(10.0),
    };

    let table_cell_box = MarkdownBlockStyle {
      background: None,
      border_color: None,
      border_width: None,
      radius: None,
      padding: Some(10.0),
    };

    Self {
      document_spacing: 16.0,
      list_item_spacing: 8.0,
      nested_block_spacing: 8.0,
      code_block_spacing: 8.0,
      blockquote_gap: 12.0,
      list_marker_gap: 8.0,
      unordered_list_marker_width: 24.0,
      ordered_list_marker_width: 36.0,
      table_row_spacing: 1.0,
      table_column_spacing: 1.0,
      table_cell_min_width: 120.0,
      heading,
      heading_1,
      heading_2,
      heading_3,
      heading_4,
      heading_5,
      heading_6,
      strong,
      emphasis,
      inline_code,
      code_block,
      code_block_label,
      link,
      strikethrough: MarkdownInlineStyle::default(),
      list_marker,
      blockquote_marker,
      table_header,
      table_cell,
      table_marker,
      code_block_box,
      blockquote_box,
      table_box,
      table_header_box,
      table_cell_box,
    }
  }
}

fn heading_style(font_size_scale: f32) -> MarkdownInlineStyle {
  let mut style = MarkdownInlineStyle::default();
  style.text.font_size_scale = Some(font_size_scale);
  style
}
