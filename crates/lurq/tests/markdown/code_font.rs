//! The default Markdown theme draws inline code and code blocks in the
//! generic `monospace` family: every glyph of "iiii" and "MMMM" advances the
//! same distance there, while body text draws "M" far wider than "i".

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Markdown, MarkdownProps},
  layout::text_style::TextStyle,
};

use crate::support::render_pass_with_app;

struct MarkdownRoot;

impl Component for MarkdownRoot {
  type Props = MarkdownProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<lurq::node::Element> {
    Markdown::mount(ctx, ctx.props::<Self::Props>().clone())
  }
}

/// The distances from each drawn glyph of `source` to the next, in pixels.
/// Glyph positions are whole pixels, so equal advances differ by up to one.
fn advances(source: &str) -> Vec<f32> {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(800, 300);
  tree.mount_root::<MarkdownRoot>(
    &mut app,
    MarkdownProps::new(source)
      .style(TextStyle {
        font_size: 24.0,
        ..TextStyle::default()
      })
      .width(760.0),
  );
  let mut xs: Vec<f32> = render_pass_with_app(&mut tree, &mut app)
    .glyphs
    .iter()
    .filter(|glyph| glyph.width > 0.0)
    .map(|glyph| glyph.x)
    .collect();
  assert_eq!(xs.len(), 4, "{source:?} draws its four letters and nothing else");
  xs.sort_by(f32::total_cmp);
  xs.windows(2).map(|pair| pair[1] - pair[0]).collect()
}

fn assert_monospaced(narrow: &str, wide: &str) {
  let all: Vec<f32> = advances(narrow).into_iter().chain(advances(wide)).collect();
  let (min, max) = all.iter().fold((f32::MAX, f32::MIN), |(min, max), &advance| {
    (min.min(advance), max.max(advance))
  });
  assert!(
    max - min <= 1.0,
    "{narrow:?} and {wide:?} advance every glyph alike, got {all:?}"
  );
}

fn mean(advances: &[f32]) -> f32 {
  advances.iter().sum::<f32>() / advances.len() as f32
}

#[test]
fn markdown_code_font_inline_code_is_monospaced() {
  assert_monospaced("`iiii`", "`MMMM`");
}

#[test]
fn markdown_code_font_code_block_is_monospaced() {
  assert_monospaced("```\niiii\n```", "```\nMMMM\n```");
}

#[test]
fn markdown_code_font_body_text_is_proportional() {
  let narrow = mean(&advances("iiii"));
  let wide = mean(&advances("MMMM"));
  assert!(
    wide > 2.0 * narrow,
    "body text draws \"M\" wider than \"i\": {narrow} and {wide}"
  );
}
