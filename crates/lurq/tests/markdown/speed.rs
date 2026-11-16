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
  #[cfg(feature = "perf_profile")]
  lurq::app::profiler::take_frame_sections();

  // The source changes at its end, as a streamed answer does: the document is built again.
  let started = Instant::now();
  source.update(|text| text.push_str("One more paragraph.\n"));
  render_pass_with_app(&mut tree, &mut app);
  let again = started.elapsed();
  // With `perf_profile` and `LURQ_REBUILD_SECTIONS`, where the build again spent its time.
  #[cfg(feature = "perf_profile")]
  if std::env::var_os("LURQ_REBUILD_SECTIONS").is_some() {
    for (name, total, count) in lurq::app::profiler::take_frame_sections() {
      println!(
        "  {sections} sections, built again, {name}: {:.3} ms ({count} runs)",
        total.as_secs_f64() * 1000.0
      );
    }
  }

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
  // With `LURQ_FRAME_TIMES`, each frame of scrolling as it came: where the frames that build something stand.
  if std::env::var_os("LURQ_FRAME_TIMES").is_some() {
    let times: Vec<String> = frames
      .iter()
      .map(|frame| format!("{:.2}", frame.as_secs_f64() * 1000.0))
      .collect();
    println!("  {sections} sections, frames: {}", times.join(" "));
  }
  frames.sort();
  (first, again, frames[frames.len() / 2])
}

/// Tab taking the focus from one link of the document to the next, twenty times: the median frame. Nothing is built
/// again; the focus moves.
fn tab_through_links(sections: usize) -> Duration {
  let source = Signal::new(document(sections));
  let state = ScrollState::new();
  SOURCE.with(|slot| *slot.borrow_mut() = Some((source.clone(), state.clone())));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(720.0, 900.0))));
  tree.mount_root::<Long>(&mut app, ());
  render_pass_with_app(&mut tree, &mut app);
  render_pass_with_app(&mut tree, &mut app);
  #[cfg(feature = "perf_profile")]
  lurq::app::profiler::take_frame_sections();
  let mut frames = Vec::new();
  for _ in 0..20 {
    let started = Instant::now();
    tree.key_down("Tab".into(), "Tab".into(), false, false, false);
    tree.key_up("Tab".into(), "Tab".into(), false, false, false);
    render_pass_with_app(&mut tree, &mut app);
    frames.push(started.elapsed());
  }
  #[cfg(feature = "perf_profile")]
  if std::env::var_os("LURQ_COMPONENT_SECTIONS").is_some() {
    for (name, total, count) in lurq::app::profiler::take_frame_sections() {
      println!(
        "  {sections} sections, a Tab, {name}: {:.3} ms a frame ({count} runs)",
        total.as_secs_f64() * 1000.0 / 20.0
      );
    }
  }
  frames.sort();
  frames[frames.len() / 2]
}

thread_local! {
  static TICK: std::cell::RefCell<Option<Signal<u32>>> = const { std::cell::RefCell::new(None) };
}

/// A component of its own beside the document: it alone is built again when its signal changes.
struct Ticker;

impl Component for Ticker {
  type Props = ();

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, _: &mut Ctx) -> impl Into<lurq::node::Element> {
    let tick = TICK.with(|slot| slot.borrow().clone().expect("installed")).get();
    lurq::components::Rect::new(40.0, 10.0 + (tick % 7) as f32)
  }
}

/// The document with the ticker after it, in one scroll container.
struct WithTicker;

impl Component for WithTicker {
  type Props = ();

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<lurq::node::Element> {
    let (source, state) = SOURCE.with(|slot| slot.borrow().clone().expect("installed"));
    let markdown = Markdown::mount(ctx, MarkdownProps::new(source.get()));
    let ticker = ctx.mount::<Ticker>(());
    ScrollVertical::new(lurq::components::Column::new().child(markdown).child(ticker))
      .with_scroll_state(state)
      .size(720.0, 900.0)
  }
}

/// One component built again beside the document, twenty times: the median frame. The document is not built again;
/// what is looked through, numbered and laid out again for it is what this measures.
fn ticker_beside(sections: usize) -> Duration {
  let source = Signal::new(document(sections));
  let state = ScrollState::new();
  let tick = Signal::new(0u32);
  SOURCE.with(|slot| *slot.borrow_mut() = Some((source.clone(), state.clone())));
  TICK.with(|slot| *slot.borrow_mut() = Some(tick.clone()));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(720.0, 900.0))));
  tree.mount_root::<WithTicker>(&mut app, ());
  render_pass_with_app(&mut tree, &mut app);
  render_pass_with_app(&mut tree, &mut app);
  #[cfg(feature = "perf_profile")]
  lurq::app::profiler::take_frame_sections();
  let mut frames = Vec::new();
  for frame in 0..20 {
    let started = Instant::now();
    tick.set(frame + 1);
    render_pass_with_app(&mut tree, &mut app);
    frames.push(started.elapsed());
  }
  #[cfg(feature = "perf_profile")]
  if std::env::var_os("LURQ_COMPONENT_SECTIONS").is_some() {
    for (name, total, count) in lurq::app::profiler::take_frame_sections() {
      println!(
        "  {sections} sections, a component beside it, {name}: {:.3} ms a frame ({count} runs)",
        total.as_secs_f64() * 1000.0 / 20.0
      );
    }
  }
  frames.sort();
  frames[frames.len() / 2]
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
    let tab = (0..5).map(|_| tab_through_links(sections)).min().unwrap_or_default();
    println!("{} blocks: a Tab through its links, {}", sections * 10, millis(tab));
    let ticker = (0..5).map(|_| ticker_beside(sections)).min().unwrap_or_default();
    println!(
      "{} blocks: a component beside it built again, {}",
      sections * 10,
      millis(ticker)
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
