//! What a long document costs: a few thousand blocks of the kinds a document
//! has, built and laid out once, rendered again, and scrolled a frame at a
//! time. Run by hand, in release:
//! `cargo test -p lurq --release --features markdown --test markdown_tests -- speed --ignored --nocapture`.

use std::time::{Duration, Instant};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Markdown, MarkdownProps, ScrollVertical},
  core::Signal,
  layout::{Constraints, Size, layout_kind::ScrollState},
};

use crate::support::render_pass_with_app;

/// `sections` sections of a heading, two paragraphs with marks, a list, a
/// task list, a table, a quote and a code block: ten blocks each.
fn document(sections: usize) -> String {
  let mut source = String::new();
  for section in 0..sections {
    source.push_str(&format!(
      "## Section {section}\n\n\
       A paragraph with **bold**, *italic*, `code`, ~~struck~~ and a [link](https://example.com/{section}).\n\n\
       Another paragraph, long enough to wrap over a line or two in a column of a few hundred pixels wide.\n\n\
       - one\n- two\n  - nested\n\n\
       - [x] done\n- [ ] open\n\n\
       | Property | Type | Default |\n| --- | --- | ---: |\n| radius | f32 | 0.0 |\n\n\
       > A quote.\n\n\
       ```rust\nfn main() {{ println!(\"{section}\"); }}\n```\n\n\
       ---\n\n"
    ));
  }
  source
}

thread_local! {
  static SOURCE: std::cell::RefCell<Option<(Signal<String>, ScrollState)>> = const { std::cell::RefCell::new(None) };
}

struct Long;

impl Component for Long {
  type Props = ();

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<lurq::node::Element> {
    let (source, state) = SOURCE.with(|slot| slot.borrow().clone().expect("installed"));
    let markdown = Markdown::mount(ctx, MarkdownProps::new(source.get()));
    ScrollVertical::new(markdown)
      .with_scroll_state(state)
      .size(720.0, 900.0)
  }
}

fn millis(duration: Duration) -> String {
  format!("{:.1} ms", duration.as_secs_f64() * 1000.0)
}

/// One measure: the first build and layout, the build again, and the median frame of scrolling.
fn measure(sections: usize) -> (Duration, Duration, Duration) {
  let source = Signal::new(document(sections));
  let state = ScrollState::new();
  SOURCE.with(|slot| *slot.borrow_mut() = Some((source.clone(), state.clone())));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(720.0, 900.0))));
  let started = Instant::now();
  tree.mount_root::<Long>(&mut app, ());
  render_pass_with_app(&mut tree, &mut app);
  let first = started.elapsed();

  // The source changes at its end, as a streamed answer does: the document is built again.
  let started = Instant::now();
  source.update(|text| text.push_str("One more paragraph.\n"));
  render_pass_with_app(&mut tree, &mut app);
  let again = started.elapsed();

  // Sixty frames of scrolling, 120 pixels each: the median one.
  #[cfg(feature = "perf_profile")]
  lurq::app::profiler::take_frame_sections();
  let mut frames = Vec::new();
  for frame in 0..60 {
    let started = Instant::now();
    state.set_scroll(0.0, 120.0 * (frame + 1) as f32);
    tree.request_redraw();
    render_pass_with_app(&mut tree, &mut app);
    frames.push(started.elapsed());
  }
  // With `perf_profile`, where a frame of scrolling spends its time: each section's mean over the sixty frames.
  #[cfg(feature = "perf_profile")]
  if std::env::var_os("LURQ_FRAME_SECTIONS").is_some() {
    for (name, total, count) in lurq::app::profiler::take_frame_sections() {
      println!(
        "  {sections} sections, {name}: {:.3} ms a frame ({count} runs)",
        total.as_secs_f64() * 1000.0 / 60.0
      );
    }
  }
  frames.sort();
  (first, again, frames[frames.len() / 2])
}

#[test]
#[ignore = "a measure, run by hand in release"]
fn speed_of_a_long_document() {
  for sections in [100, 300] {
    // The least of five runs: what the work costs, without what else the machine does.
    let runs: Vec<_> = (0..5).map(|_| measure(sections)).collect();
    let first = runs.iter().map(|run| run.0).min().unwrap_or_default();
    let again = runs.iter().map(|run| run.1).min().unwrap_or_default();
    let scroll = runs.iter().map(|run| run.2).min().unwrap_or_default();
    println!(
      "{} blocks: first build and layout {}, built again {}, a frame of scrolling {}",
      sections * 10,
      millis(first),
      millis(again),
      millis(scroll)
    );
  }
}

/// What a frame draws of `sections` sections in a view 720 by 400, scrolled 1,200 down: its rects and its glyphs.
fn drawn(sections: usize) -> (usize, usize) {
  let source = Signal::new(document(sections));
  let state = ScrollState::new();
  SOURCE.with(|slot| *slot.borrow_mut() = Some((source, state.clone())));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(720.0, 400.0))));
  tree.mount_root::<Long>(&mut app, ());
  render_pass_with_app(&mut tree, &mut app);
  state.set_scroll(0.0, 1200.0);
  tree.request_redraw();
  let frame = render_pass_with_app(&mut tree, &mut app);
  (frame.rects.len(), frame.glyph_count)
}

#[test]
fn a_long_document_draws_only_what_is_in_view() {
  // Ten times the document, the same view: what is drawn is what the view shows, a code block's bar, name and copy
  // button out of view not drawn, whatever the document's length.
  let (short_rects, short_glyphs) = drawn(20);
  let (long_rects, long_glyphs) = drawn(200);
  assert!(short_rects > 0 && short_glyphs > 0);
  assert!(
    long_rects <= short_rects + short_rects / 5 && long_glyphs <= short_glyphs + short_glyphs / 5,
    "a view of 200 sections draws {long_rects} rects and {long_glyphs} glyphs, of 20 sections {short_rects} and \
     {short_glyphs}"
  );
}

/// `blocks` code blocks, each between two paragraphs.
fn code_document(blocks: usize) -> String {
  (0..blocks)
    .map(|block| format!("Block {block}.\n\n```rust\nfn block_{block}() {{}}\n```\n\n"))
    .collect()
}

/// How many copy buttons are built in the tree, and how many are drawn in its last frame: a copy button's glyph.
fn copy_glyphs(tree: &mut Tree, frame: &crate::support::RenderSnapshot) -> (usize, usize) {
  const COPY: &str = "\u{e022}";
  let built = std::cell::Cell::new(0usize);
  tree.find_element(|element| {
    if element.text_content() == Some(COPY) {
      built.set(built.get() + 1);
    }
    false
  });
  let _ = frame;
  let drawn = tree
    .resolve_quads(tree.last_layout().expect("laid out"))
    .into_iter()
    .filter(|quad| matches!(&quad.content, lurq::layout::quad::QuadContent::Text { text, .. } if text == COPY))
    .count();
  (built.get(), drawn)
}

#[test]
fn a_code_block_out_of_view_is_built_light_and_in_full_from_the_frame_it_comes_into_view() {
  let source = Signal::new(code_document(60));
  let state = ScrollState::new();
  SOURCE.with(|slot| *slot.borrow_mut() = Some((source.clone(), state.clone())));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(720, 900);
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(720.0, 900.0))));
  tree.mount_root::<Long>(&mut app, ());
  let frame = render_pass_with_app(&mut tree, &mut app);
  let (built, drawn) = copy_glyphs(&mut tree, &frame);
  assert!(drawn > 0, "the blocks in view are drawn in full");
  assert!(
    built < 30,
    "of 60 blocks, those near the view are built in full and the rest light: {built} built"
  );
  // The document's text has every block's code, light or not.
  assert!(
    MarkdownProps::new(source.get())
      .rendered_text()
      .contains("fn block_59() {}"),
    "the last block's code is the document's"
  );

  // Scrolled to the end: the last blocks are drawn in full in the frame they come into view in.
  state.set_scroll(0.0, 100_000.0);
  tree.request_redraw();
  let frame = render_pass_with_app(&mut tree, &mut app);
  let (_, drawn) = copy_glyphs(&mut tree, &frame);
  assert!(drawn > 0, "the blocks scrolled into view are drawn in full at once");
  let texts: Vec<_> = tree
    .resolve_quads(tree.last_layout().expect("laid out"))
    .into_iter()
    .filter_map(|quad| match quad.content {
      lurq::layout::quad::QuadContent::RichText { spans, .. } => {
        Some(spans.into_iter().map(|span| span.text).collect::<String>())
      }
      _ => None,
    })
    .collect();
  assert!(
    texts.iter().any(|text| text.contains("fn block_59")),
    "the last block's code is drawn: {texts:?}"
  );
}
