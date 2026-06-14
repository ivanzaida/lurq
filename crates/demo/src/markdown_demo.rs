use lurq::{
  app::ctx::Ctx,
  layout::text_style::{FontWeight, TextStyle},
  node::{Element, color::Color, dimension::Dimension},
};

use crate::style::{ACCENT, BG, BORDER, SURFACE, TEXT, TEXT_MUTED, text};

const FILL_WIDTH: Dimension = Dimension::Pct(100.0);
const CONTENT_PAD: f32 = 32.0;
const CARD_RADIUS: f32 = 8.0;

const README_MARKDOWN: &str = include_str!("../../../README.md");

pub(crate) fn markdown_content(ctx: &mut Ctx) -> Element {
  lurq::components::Column::new()
    .spacing(24.0)
    .child(text("README.md", 28.0, FontWeight::Bold, TEXT).width(FILL_WIDTH))
    .child(markdown_card(ctx))
    .child(theme_notes())
    .padding(CONTENT_PAD)
    .width(FILL_WIDTH)
    .background(BG)
    .into()
}

fn markdown_card(ctx: &mut Ctx) -> Element {
  lurq::components::Column::new()
    .spacing(14.0)
    .child(lurq::components::Markdown::mount(
      ctx,
      lurq::components::MarkdownProps::styled(
        README_MARKDOWN,
        TextStyle {
          font_size: 16.0,
          line_height: 1.45,
          color: Color::from_hex(TEXT),
          ..TextStyle::default()
        },
      )
      .width(720.0),
    ))
    .padding(24.0)
    .width(FILL_WIDTH)
    .background(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

fn theme_notes() -> Element {
  lurq::components::Column::new()
    .spacing(8.0)
    .child(text("Theme hooks", 16.0, FontWeight::Bold, TEXT))
    .child(
      text(
        "ThemeMarkdown controls inline roles and block boxes for code, quotes, and tables.",
        14.0,
        FontWeight::Normal,
        TEXT_MUTED,
      )
      .width(720.0),
    )
    .child(
      text(
        "This page embeds the workspace README and renders it through the Markdown component.",
        14.0,
        FontWeight::Normal,
        ACCENT,
      )
      .width(720.0),
    )
    .padding(18.0)
    .width(FILL_WIDTH)
    .background("#111827")
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}
