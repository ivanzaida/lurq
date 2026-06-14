use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::Ctx,
    theme::{MarkdownInlineStyle, ThemeMarkdown},
  },
  components::{Markdown, MarkdownProps, Text},
  layout::{
    quad::QuadContent,
    text_style::{FontStyle, FontWeight, TextStyle},
  },
  node::color::Color,
};

use crate::support::TestSurface;

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

#[test]
fn markdown_component_renders_rich_inline_spans() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<MarkdownRoot>(
    &mut app,
    MarkdownProps::new("Hello **bold**, *soft*, and `code`")
      .selectable(true)
      .width(320.0),
  );
  tree.pass(&mut app, &TestSurface);

  let quads = tree.resolve_quads(tree.last_layout().expect("layout should be available"));
  let spans = quads
    .iter()
    .find_map(|quad| match &quad.content {
      QuadContent::RichText { spans, .. } => Some(spans),
      _ => None,
    })
    .expect("markdown component should render inline content as rich text");

  assert_eq!(
    spans.iter().map(|span| span.text.as_str()).collect::<Vec<_>>(),
    vec!["Hello ", "bold", ", ", "soft", ", and ", "code"]
  );
  assert!(matches!(spans[1].style.weight, FontWeight::Bold));
  assert!(matches!(spans[3].style.style, FontStyle::Italic));
  assert_eq!(&*spans[5].style.font_family, "monospace");
}

#[test]
fn text_without_markdown_renders_source_text() {
  let mut tree = Tree::new();
  tree.set_root(Text::new("Hello **bold**"));
  tree.pass(&mut App::new(), &TestSurface);

  let quads = tree.resolve_quads(tree.last_layout().expect("layout should be available"));
  assert!(quads.iter().any(|quad| {
    matches!(
      &quad.content,
      QuadContent::Text { text, .. } if text == "Hello **bold**"
    )
  }));
}

#[test]
fn markdown_component_renders_code_block_box() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<MarkdownRoot>(
    &mut app,
    MarkdownProps::styled(
      "```rust\nfn main() {}\n```",
      TextStyle {
        color: Color::from_hex("#f8fafc"),
        ..TextStyle::default()
      },
    )
    .width(320.0),
  );
  tree.pass(&mut app, &TestSurface);

  let quads = tree.resolve_quads(tree.last_layout().expect("layout should be available"));
  assert!(quads.iter().any(|quad| {
    matches!(
      &quad.content,
      QuadContent::Rect { color, .. } if *color == Color::from_hex("#0f172a")
    )
  }));
  assert!(quads.iter().any(|quad| {
    matches!(
      &quad.content,
      QuadContent::Text { text, .. } if text == "fn main() {}"
    )
  }));
}

#[test]
fn markdown_component_uses_theme_markdown_overrides() {
  let mut markdown = ThemeMarkdown::default();
  markdown.strong = MarkdownInlineStyle::new();
  markdown.strong.text.color = Some(Color::from_hex("#dc2626"));

  let mut app = App::new();
  app.theme().set_markdown(markdown);

  let mut tree = Tree::new();
  tree.mount_root::<MarkdownRoot>(&mut app, MarkdownProps::new("Hello **danger**").width(320.0));
  tree.pass(&mut app, &TestSurface);

  let quads = tree.resolve_quads(tree.last_layout().expect("layout should be available"));
  let spans = quads
    .iter()
    .find_map(|quad| match &quad.content {
      QuadContent::RichText { spans, .. } => Some(spans),
      _ => None,
    })
    .expect("markdown component should render inline content as rich text");

  let strong = spans
    .iter()
    .find(|span| span.text == "danger")
    .expect("strong span should be present");
  assert_eq!(strong.style.color, Color::from_hex("#dc2626"));
}
