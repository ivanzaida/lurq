//! Markdown links at block level: a paragraph stays one selectable text, an
//! app draws a list itself, and many links and reference definitions cost
//! one parse and linear work.

use std::{
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
  time::{Duration, Instant},
};

use lurq::{
  app::events::MouseButton,
  components::{Column, MarkdownLinkPresentation, MarkdownProps, Row, Text},
  layout::quad::QuadContent,
  markdown::{MarkdownBlock, MarkdownInline, parse_markdown},
  node::{Element, color::Color},
};

use super::links::{bounds_of, has_element_with_text, mount, pass, quads, rich_text_quads, texts};

#[test]
fn markdown_paragraph_with_a_link_and_emphasis_is_one_selectable_text() {
  let (mut app, mut tree) = mount(
    MarkdownProps::new("Read *the* [docs](/docs) **now** please.")
      .selectable(true)
      .width(500.0),
  );

  let before = quads(&tree);
  let paragraphs = rich_text_quads(&before);
  assert_eq!(paragraphs.len(), 1, "one text node for the paragraph");
  assert_eq!(
    texts(paragraphs[0]),
    vec!["Read ", "the", " ", "docs", " ", "now", " please."]
  );
  let paragraph = before
    .iter()
    .find(|quad| matches!(quad.content, QuadContent::RichText { .. }))
    .unwrap();
  let (left, top, height) = (paragraph.x, paragraph.y, paragraph.height);
  let right = bounds_of(&mut tree, "docs").x + 120.0;

  // Drag from the first word, across the emphasis and the link, past "now".
  tree.mouse_down(left + 1.0, top + height * 0.5, MouseButton::Left);
  tree.mouse_move(right, top + height * 0.5);
  tree.mouse_up(right, top + height * 0.5, MouseButton::Left);
  pass(&mut app, &mut tree);

  let selections = quads(&tree)
    .into_iter()
    .filter(|quad| matches!(quad.content, QuadContent::Rect { color, .. } if color == Color::from_hex("#bfdbfe")))
    .collect::<Vec<_>>();
  assert_eq!(selections.len(), 1, "one selection over one line");
  assert!(selections[0].x <= left + 1.0);
  assert!(
    selections[0].x + selections[0].width > bounds_of(&mut tree, "docs").x + 20.0,
    "the selection runs across the link"
  );
}

/// Whether every item of a list is "Label: link — detail".
fn is_labelled(items: &[lurq::markdown::MarkdownListItem]) -> bool {
  items.iter().all(|item| match item.blocks.as_slice() {
    [MarkdownBlock::Paragraph(inlines)] => {
      matches!(inlines.first(), Some(MarkdownInline::Text(text)) if text.ends_with(": "))
        && inlines
          .iter()
          .any(|inline| matches!(inline, MarkdownInline::Link { .. }))
    }
    _ => false,
  })
}

#[test]
fn markdown_list_view_draws_matching_lists_and_keeps_their_links() {
  let source = "- Status: [status page](https://status.example) — up\n- Runbook: [docs](/runbook) — steps\n\n\
                Then:\n\n- a plain item\n- [another link](/other)";
  let (mut app, mut tree) = mount(
    MarkdownProps::new(source)
      .list_view(|list| {
        if !is_labelled(list.items()) {
          return None;
        }
        let mut grid = Column::new().id("labelled-grid");
        for item in list.items() {
          let MarkdownBlock::Paragraph(inlines) = &item.blocks[0] else {
            unreachable!("checked by is_labelled");
          };
          let MarkdownInline::Text(label) = &inlines[0] else {
            unreachable!("checked by is_labelled");
          };
          grid = grid.child(
            Row::new()
              .child(Text::new(label.trim_end_matches(": ")).width(80.0))
              .child(list.inlines(&inlines[1..], list.style())),
          );
        }
        Some(Element::from(grid))
      })
      .link_presentation(|_| MarkdownLinkPresentation::link())
      .width(500.0),
  );

  assert!(
    tree.get_element_by_id("labelled-grid").is_some(),
    "the app drew the labelled list"
  );
  assert!(has_element_with_text(&mut tree, "Status"));
  assert!(has_element_with_text(&mut tree, "status page"), "its links are links");
  assert!(
    has_element_with_text(&mut tree, "-"),
    "the other list is drawn as usual"
  );

  let mut order = Vec::new();
  for _ in 0..3 {
    tree.key_down("Tab".into(), "Tab".into(), false, false, false);
    pass(&mut app, &mut tree);
    order.push(
      tree
        .focused_element()
        .and_then(|element| element.text_content().map(str::to_owned))
        .unwrap_or_default(),
    );
  }
  assert_eq!(order, ["status page", "docs", "another link"]);
}

#[test]
fn markdown_many_reference_definitions_and_links_lay_out_in_linear_time() {
  const COUNT: usize = 3000;
  let mut source = String::new();
  for index in 0..COUNT {
    source.push_str(&format!("Block {index} links to [item {index}][ref{index}].\n\n"));
  }
  for index in 0..COUNT {
    source.push_str(&format!("[ref{index}]: https://example.test/items/{index}\n"));
  }

  let parse_start = Instant::now();
  let document = parse_markdown(&source);
  let parse_time = parse_start.elapsed();
  assert_eq!(document.blocks.len(), COUNT);

  let presented = Arc::new(AtomicUsize::new(0));
  let props = MarkdownProps::new(source.as_str())
    .link_presentation({
      let presented = presented.clone();
      move |link| {
        presented.fetch_add(1, Ordering::Relaxed);
        assert!(link.destination().ends_with(&format!("/{}", link.index())));
        MarkdownLinkPresentation::link()
      }
    })
    .width(600.0);
  let render_start = Instant::now();
  let (mut app, mut tree) = mount(props);
  let first_pass = render_start.elapsed();
  assert_eq!(presented.load(Ordering::Relaxed), COUNT, "each link presented once");

  let again = Instant::now();
  pass(&mut app, &mut tree);
  let second_pass = again.elapsed();
  assert_eq!(
    presented.load(Ordering::Relaxed),
    COUNT,
    "an unchanged document is not parsed or drawn again"
  );

  // About 25 ms and 250 ms in an unoptimized build; the budgets leave room
  // for a loaded machine. Parsing the document once per block, or resolving
  // every definition per link, takes far longer.
  assert!(parse_time < Duration::from_secs(1), "parse took {parse_time:?}");
  assert!(first_pass < Duration::from_secs(5), "first pass took {first_pass:?}");
  assert!(second_pass < first_pass, "{second_pass:?} vs {first_pass:?}");
}
