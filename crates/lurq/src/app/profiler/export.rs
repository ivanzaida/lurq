//! Versioned content-free JSON export, reusable outside the MCP transport.
use serde_json::{Value, json};

use super::{FrameProfile, model::*};

macro_rules! timings {
  ($value:expr; $($field:ident),* $(,)?) => {{
    let mut map = serde_json::Map::new();
    $(map.insert(stringify!($field).into(), Value::from($value.$field.as_secs_f64() * 1000.));)*
    Value::Object(map)
  }};
}
macro_rules! counts {
  ($value:expr; $($field:ident),* $(,)?) => {{
    let mut map = serde_json::Map::new();
    $(map.insert(stringify!($field).into(), Value::from($value.$field));)*
    Value::Object(map)
  }};
}

impl BuildAvailability {
  pub fn to_json(self) -> Value {
    json!({
      "lurq_version": env!("CARGO_PKG_VERSION"), "target_os": std::env::consts::OS,
      "target_arch": std::env::consts::ARCH, "debug_assertions": self.debug_assertions,
      "cargo_profile": "not_embedded", "optimization_level": "not_embedded",
      "features": { "perf_profile": self.perf_profile, "canvas": self.canvas, "wgpu": self.wgpu,
        "dx12": self.dx12, "devtools": self.devtools },
      "gpu_timestamps": { "available": false, "reason": "GPU timestamp queries are not instrumented" }
    })
  }
}

impl SessionStarted {
  pub fn to_json(&self) -> Value {
    json!({ "schema_version": 1, "status": "recording", "id": format!("profile_{}", self.id.0),
      "started_ms": self.started_ms, "max_samples": self.max_samples, "build": self.build.to_json(),
      "boundary_policy": "whole_completed_operations", "requests_redraw": false })
  }
}

impl ProfileReport {
  /// Export is detached from live state; finalized results stay immutable.
  pub fn to_json(&self) -> Value {
    let status = if !self.in_flight.is_empty() {
      "unfinished_work_observed"
    } else if self.samples.is_empty() {
      "no_completed_samples"
    } else {
      "completed_samples"
    };
    let newest = self.samples.last().map(|sample| sample.completed_ms);
    json!({
      "schema_version": 1, "id": format!("profile_{}", self.id.0), "status": status,
      "finalized": self.finalized, "started_ms": self.started_ms, "ended_ms": self.ended_ms,
      "observed_ms": self.observed_ms, "time_origin": "collector_monotonic_epoch",
      "sample_age_ms": newest.map(|completed| (self.observed_ms - completed).max(0.)),
      "build": self.build.to_json(), "max_samples": self.max_samples,
      "completed_samples": self.completed_samples, "returned_samples": self.samples.len(),
      "dropped_samples": self.dropped_samples, "truncated": self.dropped_samples > 0,
      "boundary_excluded_samples": self.boundary_excluded_samples, "untracked_windows": self.untracked_windows,
      "boundary_policy": "whole_completed_operations; in-flight observations are unfinished and excluded from completed samples",
      "windows": self.windows.iter().map(|window| json!({
        "id": window.id, "open": window.open, "devtools": window.devtools
      })).collect::<Vec<_>>(),
      "in_flight": self.in_flight.iter().map(|current| json!({
        "window": current.window, "frame_id": current.frame_id, "phase": current.phase.name(),
        "started_ms": current.started_ms, "elapsed_so_far_ms": current.elapsed_ms,
        "phase_elapsed_so_far_ms": current.phase_elapsed_ms, "unfinished": true,
        "excluded_from_completed_samples": true, "started_before_session": current.started_before_session
      })).collect::<Vec<_>>(),
      "samples": self.samples.iter().map(|sample| sample_json(sample)).collect::<Vec<_>>(),
      "scope_semantics": {
        "pass": "Tree::pass wall time; includes nested UI/layout/text/Canvas/render; excludes prior input dispatch",
        "ui_update": "inclusive rebuild/refresh with nested commit; may precede or nest in a pass",
        "layout_update": "inclusive rebuild/resources/layout/Canvas binding/component after-layout hooks",
        "layout_compute": "aggregate runtime-owned LayoutEngine calls including overlay measurements and UI text; nested in layout_update",
        "component_after_layout": "entire root/recursive hook sweep; includes application Canvas painting; nested in layout_update",
        "ui_text": "UI GlyphEngine only; excludes document CanvasTextEngine shaping",
        "render_encode": "inclusive backend encoding; DX12 includes Canvas and atlas uploads; WGPU includes buffer/image uploads",
        "canvas": "CPU processing; recording includes uploads and WGPU submission; tessellation includes mesh cache lookup",
        "submit_present": "CPU API wall time, not GPU execution",
        "fps": "on-demand rendering; sample count/interval is not throughput or FPS"
      }
    })
  }
}

fn sample_json(sample: &ProfileSample) -> Value {
  let data = match &sample.data {
    SampleData::Pass(pass) => json!({
      "kind": "pass", "frame_id": pass.frame_id, "rendered": pass.rendered,
      "cached_render_list": pass.cached_render_list, "backend": pass.backend,
      "layout_recalculated": pass.layout_recalculated,
      "cpu_timings_ms": timings!(pass; total, layout_update, layout_compute, component_after_layout, canvas_recording, canvas_preparation),
      "frame": pass.frame.as_ref().map(|frame| frame_json(frame, pass.backend)),
      "gpu_timing_ms": null
    }),
    SampleData::UiUpdate(update) => json!({
      "kind": "ui_update", "frame_id": update.frame_id,
      "operation": match update.kind { UiUpdateKind::RootRebuild => "root_rebuild", UiUpdateKind::SubtreeRefresh => "subtree_refresh" },
      "cpu_timings_ms": timings!(update; total, commit), "inclusive_in_pass": update.frame_id.is_some()
    }),
    SampleData::InputDispatch(input) => json!({
      "kind": "input_dispatch", "frame_id": input.frame_id, "rendered": false,
      "category": input.kind.name(), "cpu_timings_ms": timings!(input; total),
      "inclusive_in_pass": input.frame_id.is_some()
    }),
  };
  json!({ "sequence": sample.sequence, "window": sample.window, "started_ms": sample.started_ms,
    "completed_ms": sample.completed_ms, "data": data })
}

fn frame_json(frame: &FrameProfile, backend: &str) -> Value {
  let render = frame.render;
  let text = frame.glyph_engine;
  json!({
    "cpu_timings_ms": timings!(frame; total, layout, quad_resolve, glyph_rasterize, gpu_submit),
    "legacy_gpu_submit_semantics": "CPU Canvas preparation + render call; not GPU time",
    "counts": counts!(frame; quad_count, rect_count, glyph_count, glyph_cache_hits, glyph_cache_misses,
      text_measure_cache_hits, text_measure_cache_misses),
    "render": {
      "profile_available": frame.render_profile_available,
      "coverage": {
        "canvas_cpu": cfg!(feature = "canvas") && frame.render_profile_available && matches!(backend, "wgpu" | "dx12"),
        "canvas_buffer_upload": "WGPU vertex/globals; DX12 vertex only (constants remain inside recording)",
        "ui_buffer_image_upload": "WGPU separate; DX12 included in encode without independent sub-timers",
        "gpu_execution": false
      },
      "cpu_timings_ms": frame.render_profile_available.then(|| timings!(render; total, init, acquire, globals_upload, atlas_upload,
        buffer_upload, image_upload, encode, submit, present)),
      "glyph_atlas": counts!(render; glyph_atlas_upload_bytes, glyph_atlas_upload_rects,
        glyph_atlas_full_uploads, glyph_atlas_arena_uploads, glyph_atlas_dedicated_uploads),
      "canvas": {
        "cpu_timings_ms": timings!(render.canvas; total, tessellation, asset_upload, buffer_upload, recording, submit),
        "counts": counts!(render.canvas; batches, command_groups, vertices, tiles, uploaded_asset_bytes)
      }
    },
    "text": {
      "cpu_timings_ms": timings!(text; shape_text, shape_rich_text, caret_total, caret_buffer, caret_extract,
        vertical_extents, vertical_extents_shape, raster_acquire_buffer, raster_set_text, plain_buffer_build,
        plain_paragraph_shape, plain_paragraph_layout, plain_buffer_finalize, plain_buffer_retain,
        plain_reflow_analysis, rich_acquire_buffer, rich_set_text, rich_prepare_spans, rich_buffer_set_text,
        rich_align_lines, rich_cosmic_shape, rich_measure, rich_extract, pack_rich_shaped,
        swash_lookup, atlas_pack, append_cached),
      "counts": counts!(text; caret_requests, caret_hits, caret_misses, caret_returned_positions,
        caret_layout_reuses, caret_built_paragraphs, caret_reused_paragraphs, caret_built_positions,
        vertical_extents_runs, vertical_extents_glyphs, plain_buffer_hits, plain_buffer_misses,
        plain_reused_lines, plain_shaped_paragraphs, plain_laid_out_paragraphs, plain_reflow_reuses,
        plain_retained_paragraphs, plain_reindexed_paragraphs, plain_accounted_paragraphs,
        plain_cache_bytes, plain_cache_budget, plain_cache_compactions, plain_cache_evictions,
        plain_cache_bypasses, swash_requests, atlas_packs, rich_text_loads, rich_single_span_loads,
        rich_multi_span_loads, rich_loaded_spans, rich_loaded_bytes)
    },
    "memory_bytes": counts!(frame.memory; total_bytes, runtime_struct_bytes, root_tree_bytes,
      root_context_bytes, root_component_bytes, last_layout_bytes, glyph_engine_bytes, render_engine_bytes,
      hover_path_bytes, active_path_bytes, dragging_scroll_bytes),
    "memory_sampling": "cached estimate sampled at most once per second; not a heap allocation trace"
  })
}
