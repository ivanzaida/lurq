//! Markdown links: how the app presents them, underlines, trailing runs, and
//! that a paragraph with links stays one text.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::Ctx,
    theme::{MarkdownInlineStyle, MarkdownLinkState, ThemeMarkdown},
  },
  components::{Markdown, MarkdownLink, MarkdownLinkPresentation, MarkdownProps, MarkdownRun},
  core::ElementRect,
  layout::{
    quad::{Quad, QuadContent, RichTextSpan},
    text_style::TextUnderline,
  },
  node::{Element, color::Color},
};

use crate::support::TestSurface;

pub(super) struct MarkdownRoot;

impl Component for MarkdownRoot {
  type Props = MarkdownProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    Markdown::mount(ctx, ctx.props::<Self::Props>().clone())
  }
}

pub(super) fn mount(props: MarkdownProps) -> (App, Tree) {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<MarkdownRoot>(&mut app, props);
  tree.pass(&mut app, &TestSurface);
  (app, tree)
}

pub(super) fn pass(app: &mut App, tree: &mut Tree) {
  tree.request_redraw();
  tree.pass(app, &TestSurface);
}

pub(super) fn quads(tree: &Tree) -> Vec<Quad> {
  tree.resolve_quads(tree.last_layout().expect("a layout"))
}

pub(super) fn rich_text_quads(quads: &[Quad]) -> Vec<&Vec<RichTextSpan>> {
  quads
    .iter()
    .filter_map(|quad| match &quad.content {
      QuadContent::RichText { spans, .. } => Some(spans),
      _ => None,
    })
    .collect()
}

pub(super) fn span<'a>(spans: &'a [RichTextSpan], text: &str) -> &'a RichTextSpan {
  spans
    .iter()
    .find(|span| span.text == text)
    .unwrap_or_else(|| panic!("no span {text:?} in {:?}", texts(spans)))
}

/// The spans' texts, without the invisible isolate marks around a link's
/// words and trailing runs.
pub(super) fn texts(spans: &[RichTextSpan]) -> Vec<&str> {
  spans
    .iter()
    .map(|span| span.text.as_str())
    .filter(|text| !matches!(*text, "\u{2068}" | "\u{2069}"))
    .collect()
}

pub(super) fn rects_of(quads: &[Quad], color: Color) -> Vec<&Quad> {
  quads
    .iter()
    .filter(|quad| matches!(quad.content, QuadContent::Rect { color: rect_color, .. } if rect_color == color))
    .collect()
}

/// The bounds of the element whose own text is `text`: a link node.
pub(super) fn bounds_of(tree: &mut Tree, text: &str) -> ElementRect {
  tree
    .find_element(|element| element.text_content() == Some(text))
    .unwrap_or_else(|| panic!("no element with text {text:?}"))
    .bounds()
}

pub(super) fn has_element_with_text(tree: &mut Tree, text: &str) -> bool {
  tree
    .find_element(|element| element.text_content() == Some(text))
    .is_some()
}

pub(super) fn colored(color: &str) -> MarkdownInlineStyle {
  let mut style = MarkdownInlineStyle::default();
  style.text.color = Some(Color::from_hex(color));
  style
}

pub(super) fn underlined(color: &str, underline: &str) -> MarkdownInlineStyle {
  let mut style = colored(color);
  style.text.underline = Some(TextUnderline::new().color(Color::from_hex(underline)).thickness(1.0));
  style
}

/// A link to the app (`orchester:`), one to the web (`https:`) and any other.
fn present_by_kind(link: &MarkdownLink) -> MarkdownLinkPresentation {
  let destination = link.destination();
  if destination.starts_with("orchester:") {
    MarkdownLinkPresentation::link().style(underlined("#aa0000", "#aa0001"))
  } else if let Some(rest) = destination.strip_prefix("https://") {
    let host = rest.split('/').next().unwrap_or_default().to_owned();
    MarkdownLinkPresentation::link()
      .style(colored("#00aa00"))
      .trailing([MarkdownRun::new(format!("\u{a0}{host}"), colored("#777777"))])
  } else {
    MarkdownLinkPresentation::text().trailing([MarkdownRun::new(format!(" ({destination})"), colored("#555555"))])
  }
}

#[test]
fn markdown_link_presentation_styles_each_link_by_its_destination() {
  let (_app, mut tree) = mount(
    MarkdownProps::new(
      "See [task](orchester:/tasks/1), [PR 42](https://github.com/acme/pull/42) and [mail](mailto:a@b.c).",
    )
    .link_presentation(present_by_kind)
    .width(600.0),
  );

  let quads = quads(&tree);
  let paragraphs = rich_text_quads(&quads);
  assert_eq!(paragraphs.len(), 1, "the paragraph stays one rich text");
  let spans = paragraphs[0];
  assert_eq!(
    texts(spans),
    vec![
      "See ",
      "task",
      ", ",
      "PR 42",
      "\u{a0}github.com",
      " and ",
      "mail",
      " (mailto:a@b.c)",
      "."
    ]
  );
  assert_eq!(span(spans, "task").style.color, Color::from_hex("#aa0000"));
  assert_eq!(
    span(spans, "task")
      .style
      .underline
      .and_then(|underline| underline.color),
    Some(Color::from_hex("#aa0001"))
  );
  assert_eq!(span(spans, "PR 42").style.color, Color::from_hex("#00aa00"));
  assert_eq!(span(spans, "PR 42").style.underline, None);
  assert_eq!(span(spans, "\u{a0}github.com").style.color, Color::from_hex("#777777"));
  assert_eq!(
    span(spans, "mail").style.color,
    span(spans, "See ").style.color,
    "a link shown as text is the text around it"
  );
  assert_eq!(span(spans, " (mailto:a@b.c)").style.color, Color::from_hex("#555555"));

  assert!(has_element_with_text(&mut tree, "task"));
  assert!(has_element_with_text(&mut tree, "PR 42"));
  assert!(
    !has_element_with_text(&mut tree, "mail"),
    "a link shown as text is no link node: nothing to press or focus"
  );
}

#[test]
fn markdown_without_presentation_draws_links_in_the_theme_link_style() {
  let theme = ThemeMarkdown {
    link: underlined("#123456", "#654321"),
    ..ThemeMarkdown::default()
  };
  let (_app, tree) = mount(
    MarkdownProps::new("Read [the docs](/docs) now.")
      .theme(theme)
      .width(400.0),
  );

  let quads = quads(&tree);
  let spans = rich_text_quads(&quads)[0];
  assert_eq!(texts(spans), vec!["Read ", "the docs", " now."]);
  assert_eq!(span(spans, "the docs").style.color, Color::from_hex("#123456"));
  assert_eq!(
    rects_of(&quads, Color::from_hex("#654321")).len(),
    1,
    "the theme's underline is drawn"
  );
}

#[test]
fn markdown_underline_is_drawn_under_every_line_of_a_wrapped_link() {
  let (_app, mut tree) = mount(
    MarkdownProps::new("Intro [a link label long enough to wrap over several lines](orchester:/a) end.")
      .link_presentation(present_by_kind)
      .width(150.0),
  );

  let quads = quads(&tree);
  let paragraph = quads
    .iter()
    .find(|quad| matches!(quad.content, QuadContent::RichText { .. }))
    .expect("the paragraph");
  let underlines = rects_of(&quads, Color::from_hex("#aa0001"));
  assert!(
    underlines.len() >= 2,
    "one underline per laid-out line: {}",
    underlines.len()
  );
  let mut lines = underlines.iter().map(|quad| quad.y.round() as i32).collect::<Vec<_>>();
  lines.dedup();
  assert_eq!(lines.len(), underlines.len(), "each underline is on its own line");
  for underline in &underlines {
    assert_eq!(underline.height, 1.0, "the thickness the style sets");
    assert!(underline.width > 0.0);
    assert!(underline.x >= paragraph.x - 0.5 && underline.x + underline.width <= paragraph.x + 150.5);
    assert!(underline.y > paragraph.y && underline.y < paragraph.y + paragraph.height);
  }
  // The underline sits under the words: below the top of the link's first
  // line and within it.
  let link = bounds_of(&mut tree, "a link label long enough to wrap over several lines");
  let first = underlines.iter().map(|quad| quad.y).fold(f32::INFINITY, f32::min);
  assert!(
    first > link.y + link.height * 0.5 && first < link.y + link.height,
    "{first} in {link:?}"
  );
}

#[test]
fn markdown_trailing_runs_follow_the_link_and_take_its_press() {
  let clicked = Arc::new(Mutex::new(Vec::new()));
  let (_app, mut tree) = mount(
    MarkdownProps::new("Open [PR 42](https://github.com/acme/pull/42) to review.")
      .link_presentation(present_by_kind)
      .on_link_click({
        let clicked = clicked.clone();
        move |link| clicked.lock().unwrap().push(link.destination().to_owned())
      })
      .width(600.0),
  );

  let link = bounds_of(&mut tree, "PR 42");
  // Past the words, on the host after them.
  let host_x = link.x + link.width + 20.0;
  crate::support::pointer_click(
    &mut tree,
    host_x,
    link.y + link.height * 0.5,
    lurq::app::events::MouseButton::Left,
  );
  assert_eq!(
    *clicked.lock().unwrap(),
    vec!["https://github.com/acme/pull/42".to_owned()]
  );

  // Well past the host, on the text after the link: nothing.
  crate::support::pointer_click(
    &mut tree,
    host_x + 120.0,
    link.y + link.height * 0.5,
    lurq::app::events::MouseButton::Left,
  );
  assert_eq!(clicked.lock().unwrap().len(), 1);
}

#[test]
fn markdown_presentation_sees_the_destination_the_click_follows() {
  let presented = Arc::new(Mutex::new(Vec::new()));
  let clicked = Arc::new(Mutex::new(Vec::new()));
  let source = "A [reference][docs], an <https://auto.example/x> and [inline](/inline \"Title\").\n\n\
                [docs]: https://docs.example/guide";
  let (_app, mut tree) = mount(
    MarkdownProps::new(source)
      .link_presentation({
        let presented = presented.clone();
        move |link| {
          presented
            .lock()
            .unwrap()
            .push((link.index(), link.destination().to_owned(), link.text().to_owned()));
          MarkdownLinkPresentation::link().trailing([MarkdownRun::plain(format!(" <{}>", link.destination()))])
        }
      })
      .on_link_click({
        let clicked = clicked.clone();
        move |link| {
          clicked
            .lock()
            .unwrap()
            .push((link.index(), link.destination().to_owned(), link.text().to_owned()))
        }
      })
      .width(800.0),
  );

  for text in ["reference", "https://auto.example/x", "inline"] {
    let bounds = bounds_of(&mut tree, text);
    crate::support::pointer_click(
      &mut tree,
      bounds.x + 2.0,
      bounds.y + bounds.height * 0.5,
      lurq::app::events::MouseButton::Left,
    );
  }
  let presented = presented.lock().unwrap().clone();
  assert_eq!(
    presented,
    vec![
      (0, "https://docs.example/guide".to_owned(), "reference".to_owned()),
      (
        1,
        "https://auto.example/x".to_owned(),
        "https://auto.example/x".to_owned()
      ),
      (2, "/inline".to_owned(), "inline".to_owned()),
    ]
  );
  assert_eq!(
    *clicked.lock().unwrap(),
    presented,
    "each press follows the link it showed"
  );
  let quads = quads(&tree);
  let spans = rich_text_quads(&quads)[0];
  assert!(texts(spans).contains(&" <https://docs.example/guide>"));
}

#[test]
fn markdown_hover_state_recolors_words_trailing_runs_and_fills() {
  let hovered = MarkdownLinkState::new()
    .color(Color::from_hex("#0000ee"))
    .background(Color::from_hex("#eeeeee"))
    .trailing_color(Color::from_hex("#333333"));
  let (mut app, mut tree) = mount(
    MarkdownProps::new("Open [PR 42](https://github.com/acme/pull/42) now.")
      .link_presentation(move |link| present_by_kind(link).hovered(hovered))
      .width(600.0),
  );

  let link = bounds_of(&mut tree, "PR 42");
  let at_rest = quads(&tree);
  assert!(rects_of(&at_rest, Color::from_hex("#eeeeee")).is_empty());

  tree.mouse_move(link.x + link.width * 0.5, link.y + link.height * 0.5);
  pass(&mut app, &mut tree);
  let hovered_quads = quads(&tree);
  let spans = rich_text_quads(&hovered_quads)[0];
  assert_eq!(span(spans, "PR 42").style.color, Color::from_hex("#0000ee"));
  assert_eq!(span(spans, "\u{a0}github.com").style.color, Color::from_hex("#333333"));
  assert_eq!(span(spans, "Open ").style.color, Color::from_hex("#000000"));
  let fills = rects_of(&hovered_quads, Color::from_hex("#eeeeee"));
  assert_eq!(fills.len(), 1);
  let fill = fills[0];
  assert!((fill.x - link.x).abs() < 0.5 && (fill.width - link.width).abs() < 0.5);
  let fill_order = hovered_quads.iter().position(|quad| std::ptr::eq(quad, fill)).unwrap();
  let text_order = hovered_quads
    .iter()
    .position(|quad| matches!(quad.content, QuadContent::RichText { .. }))
    .unwrap();
  assert!(fill_order < text_order, "the fill is under the words");
  assert_eq!(tree.cursor(), lurq::node::CursorIcon::Pointer);

  tree.mouse_move(link.x + link.width + 300.0, link.y + link.height * 0.5);
  pass(&mut app, &mut tree);
  let after = quads(&tree);
  assert!(rects_of(&after, Color::from_hex("#eeeeee")).is_empty());
  assert_eq!(
    span(rich_text_quads(&after)[0], "PR 42").style.color,
    Color::from_hex("#00aa00")
  );
}

#[test]
fn markdown_link_types_debug_without_their_styles() {
  let presentation = MarkdownLinkPresentation::link()
    .trailing([MarkdownRun::plain(" host")])
    .tooltip("Opens it");
  let debug = format!("{presentation:?}");
  assert!(debug.contains("MarkdownLinkPresentation") && debug.contains(" host") && debug.contains("Opens it"));
  let listed = Arc::new(Mutex::new(String::new()));
  let _ = mount(
    MarkdownProps::new("1. one\n2. two")
      .list_view({
        let listed = listed.clone();
        move |list| {
          *listed.lock().unwrap() = format!("{list:?}");
          None
        }
      })
      .width(300.0),
  );
  let listed = listed.lock().unwrap().clone();
  assert!(
    listed.starts_with("MarkdownList") && listed.contains("ordered: true"),
    "{listed}"
  );
}
