//! Windows: cargo run -p lurq --example text_interaction_probe --features winit,dx12,perf_profile -- <output.csv>
//! Runs one scripted document sequence in a real DX12 window and closes automatically.
#[cfg(target_os = "windows")]
#[path = "../benches/text_pipeline/scenario.rs"]
mod scenario;

#[cfg(target_os = "windows")]
fn main() {
  use std::{
    fs::File,
    io::{BufWriter, Write},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
  };

  use lurq::app::{App, Tree, dx12_render::Dx12RenderEngine, winit_shell::WinitWindow};

  let path = std::env::args_os().nth(1).expect("pass an output CSV path");
  let selectable = std::env::var_os("LURQ_TEXT_SELECTABLE").is_some();
  let mut output = BufWriter::new(File::create(path).expect("create output CSV"));
  writeln!(output, "phase,step,action_to_paint_ms,pass_ms,layout_ms,shape_ms,wrap_ms,shaped_paragraphs,laid_out_paragraphs,reused_paragraphs,gpu_submit_ms,atlas_upload_ms,atlas_bytes,present_ms,glyph_count,scale_factor,cache_bytes,cache_budget,cache_compactions,cache_evictions,cache_bypasses,reindexed_paragraphs,accounted_paragraphs,caret_ms,caret_buffer_ms,caret_extract_ms,caret_requests,caret_hits,caret_misses,caret_positions,caret_layout_reuses,selectable,caret_built_paragraphs,caret_reused_paragraphs,caret_built_positions,reflow_reuses,reflow_analysis_ms,atlas_arena_uploads,atlas_dedicated_uploads").unwrap();
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_render_engine_factory(|| Box::new(Dx12RenderEngine::new()));
  let mut scenario = scenario::Scenario::new().with_selectable(selectable);
  scenario.mount(&mut tree, &mut app);
  // Each action waits for the previous presentation, followed by an idle interval.
  let state = Arc::new(Mutex::new(("cold", 0usize, Instant::now(), false)));
  let tick_state = state.clone();
  let started = Instant::now();
  WinitWindow::new(app, tree)
    .with_title("lurq text interaction probe")
    .with_size(860, 800)
    .on_tick(move |tree, _| {
      if started.elapsed() > Duration::from_secs(30) {
        scenario.window().close();
        return;
      }
      let mut state = tick_state.lock().unwrap();
      if state.3 && state.2.elapsed() >= Duration::from_millis(150) {
        let action_start = Instant::now();
        if let Some(phase) = scenario.advance(tree, true) {
          *state = (phase, state.1 + 1, action_start, false);
          tree.request_redraw();
        } else {
          scenario.window().close();
        }
      }
    })
    .on_paint(move |tree, _, report| {
      let mut state = state.lock().unwrap();
      if state.3 || !report.rendered {
        return;
      }
      let p = tree.profile();
      // A resize command is asynchronous; ignore presentations before its reflow.
      if state.0 == "resize" && !p.layout_recalculated {
        return;
      }
      let g = &p.glyph_engine;
      let ms = |duration: Duration| duration.as_secs_f64() * 1000.0;
      write!(
        output,
        "{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{},{},{},{:.6},{:.6},{},{:.6},{},{},{},{},{},{},{},{},{}",
        state.0,
        state.1,
        ms(state.2.elapsed()),
        ms(p.total),
        ms(p.layout),
        ms(g.plain_paragraph_shape),
        ms(g.plain_paragraph_layout),
        g.plain_shaped_paragraphs,
        g.plain_laid_out_paragraphs,
        g.plain_retained_paragraphs,
        ms(p.gpu_submit),
        ms(p.render.atlas_upload),
        p.render.glyph_atlas_upload_bytes,
        ms(p.render.present),
        p.glyph_count,
        tree.scale_factor(),
        g.plain_cache_bytes,
        g.plain_cache_budget,
        g.plain_cache_compactions,
        g.plain_cache_evictions,
        g.plain_cache_bypasses,
        g.plain_reindexed_paragraphs,
        g.plain_accounted_paragraphs
      )
      .unwrap();
      for duration in [g.caret_total, g.caret_buffer, g.caret_extract] {
        write!(output, ",{:.6}", ms(duration)).unwrap();
      }
      for count in [
        g.caret_requests,
        g.caret_hits,
        g.caret_misses,
        g.caret_returned_positions,
        g.caret_layout_reuses,
        usize::from(selectable),
        g.caret_built_paragraphs,
        g.caret_reused_paragraphs,
        g.caret_built_positions,
      ] {
        write!(output, ",{count}").unwrap();
      }
      writeln!(
        output,
        ",{},{:.6},{},{}",
        g.plain_reflow_reuses,
        ms(g.plain_reflow_analysis),
        p.render.glyph_atlas_arena_uploads,
        p.render.glyph_atlas_dedicated_uploads
      )
      .unwrap();
      output.flush().unwrap();
      state.3 = true;
      state.2 = Instant::now();
    })
    .run();
}

#[cfg(not(target_os = "windows"))]
fn main() {
  panic!("text_interaction_probe currently requires Windows/DX12");
}
