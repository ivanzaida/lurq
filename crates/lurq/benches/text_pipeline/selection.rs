//! Event dispatch through CPU render-list production, with deterministic selection geometry.
//! No native window, GPU, clipboard, or physical input is used.
use std::{
  fs::File,
  io::{BufWriter, Write},
  path::Path,
  sync::{Arc, Mutex},
  time::Instant,
};

use lurq::{
  app::{events::MouseButton, theme::CaretMode},
  components::{ScrollVertical, TextInput},
  core::Signal,
  layout::layout_kind::ScrollState,
  node::{color::Color, dimension::Dimension},
};

use super::*;

#[derive(Clone, Copy, Default)]
struct Snapshot {
  selection_rects: usize,
  signature: u64,
  glyphs: usize,
}

struct Capture(Arc<Mutex<Snapshot>>);

impl RenderEngine for Capture {
  fn resize(&mut self, _: u32, _: u32) {}

  fn render(&mut self, list: &RenderList, _: WindowHandle<'_>, _: DisplayHandle<'_>) -> bool {
    let mut snapshot = Snapshot {
      glyphs: list.glyphs.len(),
      ..Snapshot::default()
    };
    for rect in &list.rects {
      let selection = rect.color == Color::from_hex("#ff00ff");
      if selection || rect.color == Color::from_hex("#00ff00") {
        snapshot.selection_rects += usize::from(selection);
        for n in [rect.x, rect.y, rect.width, rect.height] {
          snapshot.signature = snapshot.signature.wrapping_mul(1099511628211) ^ u64::from(n.to_bits());
        }
      }
    }
    *self.0.lock().unwrap() = snapshot;
    true
  }
}

pub(super) fn run(path: &Path) {
  let samples: usize = std::env::var("LURQ_TEXT_METRICS_SAMPLES")
    .map(|s| s.parse().expect("positive sample count"))
    .unwrap_or(5);
  assert!(samples > 0);
  let mut out = BufWriter::new(File::create(path).unwrap());
  writeln!(out, "case,scale,location,sample,phase,step,event_ms,pass_ms,total_ms,selection_rects,signature,glyphs,layout_recalculated,caret_ms,caret_extract_ms,caret_positions").unwrap();
  for sample in 0..samples {
    for scale in [1.0, 1.5] {
      for input in [false, true] {
        for (location, fraction) in [("top", 0.0), ("middle", 0.5), ("bottom", 1.0)] {
          let mut app = App::new();
          let mut tree = realistic_viewport_tree();
          tree.resize(860, 800);
          tree.set_scale_factor(scale);
          let capture = Arc::new(Mutex::new(Snapshot::default()));
          let sink = capture.clone();
          tree.set_render_engine_factory(move || Box::new(Capture(sink.clone())));
          let scroll = ScrollState::new();
          let source = unique_long_text_source();
          let value = Signal::new(source.clone());
          let content: Element = if input {
            TextInput::new(value.clone())
              .multiline()
              .width(Dimension::Pct(100.0))
              .caret_mode(CaretMode::Persistent)
              .caret_color(Color::from_hex("#00ff00"))
              .selection_color(Color::from_hex("#ff00ff"))
              .into()
          } else {
            Text::new(&source)
              .selectable(true)
              .width(Dimension::Pct(100.0))
              .selection_color(Color::from_hex("#ff00ff"))
              .into()
          };
          tree.set_root(
            ScrollVertical::new(content)
              .with_scroll_state(scroll.clone())
              .width(Dimension::Pct(100.0))
              .height(Dimension::Pct(100.0)),
          );
          run_pass(&mut tree, &mut app);
          scroll.scroll_to_bottom_pending();
          run_pass(&mut tree, &mut app);
          let offset = scroll.scroll_y() * fraction;
          scroll.set_scroll(0.0, offset);
          run_pass(&mut tree, &mut app);
          let case = if input { "input" } else { "selectable" };
          let mut measure = |phase: &str, step: usize, action: &mut dyn FnMut(&mut Tree)| {
            let start = Instant::now();
            action(&mut tree);
            let event = start.elapsed();
            run_pass(&mut tree, &mut app);
            let total = start.elapsed();
            let s = *capture.lock().unwrap();
            let p = tree.profile();
            assert!(s.glyphs > 0, "{case}/{location}/{phase} must render text");
            if phase == "drag" {
              assert!(s.selection_rects > 0, "drag must select text");
            }
            writeln!(
              out,
              "{case},{scale},{location},{sample},{phase},{step},{:.6},{:.6},{:.6},{},{},{},{},{:.6},{:.6},{}",
              event.as_secs_f64() * 1000.0,
              (total - event).as_secs_f64() * 1000.0,
              total.as_secs_f64() * 1000.0,
              s.selection_rects,
              s.signature,
              s.glyphs,
              usize::from(p.layout_recalculated),
              p.glyph_engine.caret_total.as_secs_f64() * 1000.0,
              p.glyph_engine.caret_extract.as_secs_f64() * 1000.0,
              p.glyph_engine.caret_returned_positions
            )
            .unwrap();
          };
          // Include the first hit to expose lazy-index construction, then continuous drag.
          measure("press", 0, &mut |tree| tree.mouse_down(20.0, 180.0, MouseButton::Left));
          for step in 0..24 {
            measure("drag", step, &mut |tree| {
              tree.mouse_move(100.0 + (step % 8) as f32 * 20.0, 220.0 + (step % 4) as f32 * 18.0)
            });
          }
          measure("release", 0, &mut |tree| tree.mouse_up(240.0, 274.0, MouseButton::Left));
          if input {
            for selecting in [false, true] {
              for step in 0..24 {
                let key = if step % 2 == 0 { "ArrowUp" } else { "ArrowDown" };
                measure(if selecting { "shift_arrow" } else { "arrow" }, step, &mut |tree| {
                  tree.key_down(key.to_owned(), key.to_owned(), selecting, false, false)
                });
              }
            }
          }
          assert_eq!(value.get_untracked(), source, "navigation must not edit text");
        }
      }
    }
  }
}
