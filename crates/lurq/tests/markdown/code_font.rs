//! The default Markdown theme draws inline code and code blocks in the
//! generic `monospace` family: "iiii", "MMMM" and "iMMi" take one width
//! there, while body text draws "M" far wider than "i". The widths are the
//! laid-out widths of a document that fits its content, so they do not
//! depend on where glyph images are placed.

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Column, Markdown, MarkdownProps},
  layout::text_style::TextStyle,
  node::Element,
};

use crate::support::TestSurface;

/// Layout widths are whole device pixels at most this far apart when equal.
const EPSILON: f32 = 0.5;

struct MarkdownRoot;

impl Component for MarkdownRoot {
  type Props = &'static str;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let source = *ctx.props::<Self::Props>();
    let style = TextStyle {
      font_size: 24.0,
      ..TextStyle::default()
    };
    let markdown = Markdown::mount(ctx, MarkdownProps::styled(source, style).fit_content(true));
    Column::new()
      .width(760.0)
      .child(Column::new().id("document").child(markdown))
  }
}

/// The laid-out width of `source` in a document that fits its content.
fn width(source: &'static str) -> f32 {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(800, 300);
  tree.mount_root::<MarkdownRoot>(&mut app, source);
  tree.pass(&mut app, &TestSurface);
  tree
    .find_element(|element| element.id() == Some("document"))
    .expect("the document is laid out")
    .bounds()
    .width
}

fn assert_monospaced(sources: [&'static str; 3]) {
  let widths = sources.map(width);
  assert!(
    widths[0] > 0.0 && widths[0] < 400.0,
    "{:?} fits its content: {}",
    sources[0],
    widths[0]
  );
  assert!(
    widths.iter().all(|width| (width - widths[0]).abs() <= EPSILON),
    "{sources:?} take one width, got {widths:?}"
  );
}

#[test]
fn markdown_code_font_inline_code_is_monospaced() {
  assert_monospaced(["`iiii`", "`MMMM`", "`iMMi`"]);
}

#[test]
fn markdown_code_font_code_block_is_monospaced() {
  assert_monospaced([
    "```
iiii
```",
    "```
MMMM
```",
    "```
iMMi
```",
  ]);
}

#[test]
fn markdown_code_font_body_text_is_proportional() {
  let narrow = width("iiii");
  let wide = width("MMMM");
  assert!(
    wide > 2.0 * narrow,
    "body text draws \"M\" wider than \"i\": {narrow} and {wide}"
  );
}
