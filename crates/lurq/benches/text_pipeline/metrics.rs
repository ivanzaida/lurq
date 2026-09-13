//! Repeated CPU frame samples, including setup that Criterion deliberately excludes.
//! Output stays outside the measured intervals. No window or GPU is created.

use std::{
  fs::File,
  io::{BufWriter, Write},
  path::Path,
  time::{Duration, Instant},
};

use lurq::app::profiler::FrameProfile;

use super::*;

fn mount(case: &str, app: &mut App) -> Tree {
  let mut tree = if case == "readme_tall" {
    tree()
  } else {
    realistic_viewport_tree()
  };
  match case {
    "readme_tall" | "readme" => tree.mount_root::<MarkdownRoot>(app, readme_props(usize::MAX)),
    "long_text" => tree.mount_root::<LongTextRoot>(app, long_text_source()),
    "unique_long_text" => tree.mount_root::<LongTextRoot>(app, unique_long_text_source()),
    "flow_long_text" => tree.mount_root::<FlowLongTextRoot>(app, long_text_source()),
    _ => unreachable!(),
  }
  tree
}

fn record(out: &mut impl Write, case: &str, sample: usize, phase: &str, wall: Duration, p: &FrameProfile) {
  write!(out, "{case},{sample},{phase}").unwrap();
  let g = &p.glyph_engine;
  for duration in [
    wall,
    p.total,
    p.layout,
    p.quad_resolve,
    p.glyph_rasterize,
    g.shape_text,
    g.shape_rich_text,
    g.rich_buffer_set_text,
    g.rich_cosmic_shape,
    g.vertical_extents,
    g.vertical_extents_shape,
    g.raster_acquire_buffer,
    g.raster_set_text,
    g.swash_lookup,
    g.atlas_pack,
    g.append_cached,
  ] {
    write!(out, ",{:.6}", duration.as_secs_f64() * 1000.0).unwrap();
  }
  for count in [
    p.glyph_count,
    p.text_measure_cache_hits,
    p.text_measure_cache_misses,
    p.glyph_cache_hits,
    p.glyph_cache_misses,
    g.swash_requests,
    g.atlas_packs,
    g.vertical_extents_runs,
    g.vertical_extents_glyphs,
    usize::from(p.layout_recalculated),
  ] {
    write!(out, ",{count}").unwrap();
  }
  for duration in [
    g.plain_buffer_build,
    g.plain_paragraph_shape,
    g.plain_paragraph_layout,
    g.plain_buffer_finalize,
    g.plain_buffer_retain,
    p.gpu_submit,
    p.render.atlas_upload,
    p.render.present,
  ] {
    write!(out, ",{:.6}", duration.as_secs_f64() * 1000.0).unwrap();
  }
  for count in [
    g.plain_shaped_paragraphs,
    g.plain_laid_out_paragraphs,
    g.plain_retained_paragraphs,
    g.plain_cache_bytes,
    g.plain_cache_budget,
    g.plain_cache_compactions,
    g.plain_cache_evictions,
    g.plain_cache_bypasses,
    g.plain_reindexed_paragraphs,
    g.plain_accounted_paragraphs,
  ] {
    write!(out, ",{count}").unwrap();
  }
  for duration in [g.caret_total, g.caret_buffer, g.caret_extract] {
    write!(out, ",{:.6}", duration.as_secs_f64() * 1000.0).unwrap();
  }
  for count in [
    g.caret_requests,
    g.caret_hits,
    g.caret_misses,
    g.caret_returned_positions,
    g.caret_layout_reuses,
    g.caret_built_paragraphs,
    g.caret_reused_paragraphs,
    g.caret_built_positions,
  ] {
    write!(out, ",{count}").unwrap();
  }
  writeln!(
    out,
    ",{},{:.6}",
    g.plain_reflow_reuses,
    g.plain_reflow_analysis.as_secs_f64() * 1000.0
  )
  .unwrap();
}

fn sample_count() -> usize {
  let samples: usize = std::env::var("LURQ_TEXT_METRICS_SAMPLES")
    .map(|value| {
      value
        .parse()
        .expect("LURQ_TEXT_METRICS_SAMPLES must be a positive integer")
    })
    .unwrap_or(15);
  assert!(samples > 0, "LURQ_TEXT_METRICS_SAMPLES must be positive");
  samples
}

fn header(out: &mut impl Write) {
  writeln!(
    out,
    "case,sample,phase,wall_ms,profile_total_ms,layout_ms,quads_ms,glyphs_ms,measure_shape_ms,rich_shape_ms,rich_set_ms,rich_cosmic_ms,vertical_extents_ms,vertical_extents_shape_ms,raster_acquire_ms,raster_set_ms,swash_ms,atlas_pack_ms,append_ms,glyph_count,measure_hits,measure_misses,glyph_hits,glyph_misses,swash_requests,atlas_packs,extents_runs,extents_glyphs,layout_recalculated,plain_build_ms,paragraph_shape_ms,paragraph_layout_ms,plain_finalize_ms,plain_retain_ms,gpu_submit_ms,atlas_upload_ms,present_ms,shaped_paragraphs,laid_out_paragraphs,retained_paragraphs,cache_bytes,cache_budget,cache_compactions,cache_evictions,cache_bypasses,reindexed_paragraphs,accounted_paragraphs,caret_ms,caret_buffer_ms,caret_extract_ms,caret_requests,caret_hits,caret_misses,caret_positions,caret_layout_reuses,caret_built_paragraphs,caret_reused_paragraphs,caret_built_positions,reflow_reuses,reflow_analysis_ms"
  )
  .unwrap();
}

pub(super) fn run_interactions(path: &Path) {
  let samples = sample_count();
  let selectable = std::env::var_os("LURQ_TEXT_SELECTABLE").is_some();
  let mut out = BufWriter::new(File::create(path).expect("create interaction CSV"));
  header(&mut out);
  for sample in 0..samples {
    for (case, scale) in [("document", 1.0), ("document_dpi150", 1.5)] {
      let case = match (selectable, scale == 1.5) {
        (true, false) => "document_selectable",
        (true, true) => "document_selectable_dpi150",
        _ => case,
      };
      let mut app = App::new();
      let mut tree = realistic_viewport_tree();
      tree.set_scale_factor(scale);
      tree.resize(860, 800);
      let mut scenario = scenario::Scenario::new().with_selectable(selectable);
      scenario.mount(&mut tree, &mut app);
      let start = Instant::now();
      run_pass(&mut tree, &mut app);
      record(&mut out, case, sample, "cold", start.elapsed(), tree.profile());
      while let Some(phase) = scenario.advance(&mut tree, false) {
        let start = Instant::now();
        run_pass(&mut tree, &mut app);
        record(&mut out, case, sample, phase, start.elapsed(), tree.profile());
        assert!(tree.profile().glyph_count > 0, "{phase}: document disappeared");
        if phase != "scroll" {
          assert!(tree.profile().layout_recalculated, "{phase}: no layout update");
        }
        let start = Instant::now();
        run_pass(&mut tree, &mut app);
        record(&mut out, case, sample, "warm", start.elapsed(), tree.profile());
        assert_eq!(tree.profile().glyph_engine.swash_requests, 0, "warm raster work");
      }
    }
  }
  out.flush().unwrap();
  eprintln!("Interaction metrics saved to {} ({samples} sequences)", path.display());
}

pub(super) fn run(path: &Path) {
  let samples = sample_count();
  let mut out = BufWriter::new(File::create(path).expect("create metrics CSV (parent directory must exist)"));
  header(&mut out);
  eprintln!(
    "Text metrics: {samples} samples/case; README={} bytes/{} lines; long text={} bytes; viewport={}x{}; debug_assertions={}",
    README.len(),
    README.lines().count(),
    long_text_source().len(),
    VIEWPORT_WIDTH,
    REALISTIC_VIEWPORT_HEIGHT,
    cfg!(debug_assertions),
  );
  let cases = [
    "readme",
    "long_text",
    "flow_long_text",
    "readme_tall",
    "unique_long_text",
  ];
  // Rotate case order so each workload appears throughout the run.
  for sample in 0..samples {
    for offset in 0..cases.len() {
      let case = cases[(sample + offset) % cases.len()];
      let empty = FrameProfile::default();
      let start = Instant::now();
      black_box(parse_markdown(black_box(README)));
      record(&mut out, case, sample, "parse", start.elapsed(), &empty);

      let start = Instant::now();
      let mut app = App::new();
      record(&mut out, case, sample, "app_init", start.elapsed(), &empty);
      let start = Instant::now();
      let mut tree = mount(case, &mut app);
      record(&mut out, case, sample, "mount", start.elapsed(), &empty);

      let start = Instant::now();
      run_pass(&mut tree, &mut app);
      record(&mut out, case, sample, "cold", start.elapsed(), tree.profile());
      assert!(tree.profile().glyph_count > 0, "{case}: no rendered glyphs");
      assert!(tree.profile().layout_recalculated, "{case}: missing cold layout");

      // Sample complete pass wall time; FrameProfile.total can omit work in some fast paths.
      for _ in 0..5 {
        let start = Instant::now();
        run_pass(&mut tree, &mut app);
        record(&mut out, case, sample, "warm", start.elapsed(), tree.profile());
        assert!(!tree.profile().layout_recalculated, "{case}: warm layout recalculated");
        assert_eq!(
          tree.profile().glyph_engine.swash_requests,
          0,
          "{case}: warm raster work"
        );
      }

      // Keep App/font/glyph caches, but rebuild the component tree, as on route re-entry.
      drop(tree);
      let start = Instant::now();
      let mut tree = mount(case, &mut app);
      record(&mut out, case, sample, "remount", start.elapsed(), &empty);
      let start = Instant::now();
      run_pass(&mut tree, &mut app);
      record(&mut out, case, sample, "remount_pass", start.elapsed(), tree.profile());
    }
  }
  out.flush().unwrap();
  eprintln!("Text metrics saved to {}", path.display());
}
