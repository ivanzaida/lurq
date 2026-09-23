# Changelog

## Unreleased

- Add the full CSS named font-weight set (`ExtraLight`, `SemiBold`, `ExtraBold` join the existing names) and `FontWeight::Numeric(u16)` for any weight, clamped to `1..=1000`. `FontWeight::value()` returns the number; weights compare, hash, and key text caches by value, so `Numeric(600) == SemiBold`. `Medium` now requests 500 instead of 400.
- Select the nearest loaded face for every weight. cosmic-text 0.12 only takes a face from the requested family when its weight matches exactly and otherwise falls through to fallback families (which is why `Medium` used to request 400). Text and canvas text now resolve the weight with fontdb's CSS font-matching query first, cached per family and cleared when fonts load. Loaded Medium and SemiBold faces render; a family without them uses its nearest face instead of another family. Text styled `Medium` changes appearance only where the family has a 500 face.
- Add application-defined `Extra` roles to `TypographyStyle`, `RadiusSize`, `SpacingSize`, and `BorderSize`, stored in new `extra` maps on `ThemeTypography`, `ThemeRadii`, `ThemeSpacing`, and `ThemeBorderSizes`. Each role has an `extra(name)` constructor and converts from `&str` and `Arc<str>`; the tables gain `try_get`, `resolve`, and `try_resolve`. Names are interned so these roles stay `Copy`. A missing name follows the palette: table `get`/`resolve` panic, while nodes resolve an unknown radius, spacing, or border size to `0` and an unknown typography variant to the default text style. `Breakpoint` has no extras because `Responsive` orders overrides by the enum.
- Document `PaletteColor::Extra`, the new extras, and font-weight matching in the Theme guide.
- Letter spacing is not included. cosmic-text 0.12 cannot adjust advances before wrapping, so it would need lurq-owned layouts for every text consumer; cosmic-text 0.14 adds letter spacing to `Attrs`.
- Breaking: exhaustive matches on `FontWeight`, `TypographyStyle`, `RadiusSize`, `SpacingSize`, and `BorderSize` need the new variants. `ThemeTypography`, `ThemeRadii`, `ThemeSpacing`, and `ThemeBorderSizes` struct literals need `extra` or `..Default::default()`. `ThemeRadii`, `ThemeSpacing`, and `ThemeBorderSizes` are no longer `Copy`.

## 0.20.0 — 2026-09-17

- Add gradient paints to `Context2D` fills and strokes: linear, radial and angular, 2 to 16 stops, with a centre, a size and a rotation normalised to a box. A gradient is a fragment-shader paint — one 256-texel ramp per distinct stop list, and paint coordinates carried in the vertex `uv` solid paths leave unused — so the mesh-cache key and the per-batch vertex charge are unchanged. An unusable gradient draws nothing rather than a colour of its own choosing, and is refused for shaped text with `UnsupportedPaint`.
- Add outer and inner shadows (`set_shadow`) and a `filter`-like layer blur (`set_filter`) to fills, strokes and shaped text. Both are rasterised once on the CPU into a premultiplied image and drawn like any other image, so the software backend and both native backends share one implementation. Bounded by a reduced raster for wide blurs, clipping to the surface, `MAX_EFFECT_PIXELS`, and 512-unit caps on blur and spread; rasterisations are cached process-wide at 32 MiB with random eviction. Offsets, blur and spread are in user space and scale with the transform, unlike HTML Canvas.
- Add eighteen blend modes through `set_global_composite_operation`, and isolated group compositing through `begin_layer(alpha, blend)` / `end_layer`, which is what makes a group of overlapping shapes fade as one image. Layers nest to `MAX_LAYER_DEPTH`, hold their commands on the surface until they close, and cost the renderer two shared tile-sized textures per open depth rather than a surface-sized target.
- Breaking: `fill_style()` and `stroke_style()` return `Paint` instead of `Color`; `Paint::color()` recovers the colour of a solid paint. The setters still accept `Color` and colour strings unchanged.
- Backdrop blur is not included: it reads the surface it writes, and its ordering guarantee is not yet stated. Tracked as lurq#17.

## 0.19.5 — 2026-09-16

- Retain shared `App` services in component contexts instead of a raw App pointer. Timers, futures and event-driven renders remain safe after the caller moves or drops its handle. `App` is cloneable; `persistent_storage()` now returns a cloned backend handle.
- Redact masked TextInput values throughout MCP tree/lookup/find data, set-value replies and DevTools snapshots, including shape details. Inspectors expose `masked=true`; typed input handles expose `mask()` and `is_masked()`.
- Preserve focused inputs across sibling insertion, stop recycling node IDs, and deliver blur when a focused control is removed or its tree is dropped. Add `Ctx::focus(&ElementRef)` requests and reactive `ElementRef::focused()` / `focus_signal()` queries.
- Initialize WGPU instances lazily and add `WgpuRenderEngine::with_backends`. Windows defaults to DX12 to avoid the reported concurrent Vulkan-loader teardown crash; other platforms retain their existing backend selection.
- Replace Canvas mesh-cache FIFO eviction with constant-time random eviction so ordered scans above capacity retain useful hits. Expose preparation hits, misses, evictions and shared-cache occupancy in `CanvasGpuStats`; retain the 32 MiB and 32,768-entry limits.

## 0.19.4 — 2026-09-15

- Let a text input's default insertion caret inherit its resolved text color, keeping focused fields visible on dark themes while preserving explicit node and text-style caret colors.
- Add regression coverage for a focused empty input with placeholder text.

## 0.19.3 — 2026-09-13

- Add the optional `query` feature with named async queries through `#[lurq::query]`, lazy typed descriptors, reactive handles, shared cached results, and request deduplication. Publish the accompanying macro in `lurq_macros` 0.1.2.
- Support exact-query and whole-definition invalidation, background refreshes that retain successful data, configurable freshness and inactive retention, and cancellation of obsolete requests.
- Let applications choose cache scope: `QueryClient::new()` creates an independent cache, while clones share data, requests, and invalidation across components and trees. Closing one tree preserves work used by surviving trees.
- Wake each attached tree for ready work and deliver reactive notifications on its own UI tick. Support cooperative futures and optional Tokio execution, including recovery when a shared request's runtime shuts down.
- Add query documentation and regression coverage for cache identity, invalidation, retention, provider scopes, concurrent trees, event-loop wakeups, and runtime lifetimes. Writes continue to use `future_action` with explicit invalidation after success.

## 0.19.2 — 2026-09-13

- Reuse plain-text shaping and full layouts across measurement, optical centering, caret extraction, and clipped painting. Retain unchanged paragraphs through edits and reordering, with a 48 MiB accounted cache budget and layout compaction before eviction.
- Share immutable caret geometry and index visual rows for hit testing, selections, and vertical navigation. Large multiline edits rebuild caret geometry only for changed paragraphs; masked inputs keep their original byte offsets.
- Keep input baselines stable for older text fonts without a cap-height metric by deriving it from the font's Latin H outline; fonts without that reference glyph retain ink centering.
- Preserve wrapped layouts across width changes when conservative word-wrap bounds prove the layout unchanged. Unsupported alignment, bidirectional text, and glyph-fallback wrapping continue through the existing layout path.
- Stage native DX12 glyph-atlas updates in existing frame upload arenas, avoiding routine dedicated upload allocations and an intermediate copy. Keep full-atlas transfers and the oversized-upload fallback; clear reused row padding and missing texels.
- Extend development-only optimization overrides to font outline and hinting dependencies. Release profile settings are unchanged; consuming applications must configure development profile overrides in their own workspace.
- Add detailed text and atlas counters, reproducible CPU/native interaction probes, cache and caret regression tests, and pixel comparisons. Benchmark reports distinguish CPU work from complete frames and mark runs affected by concurrent builds.

## 0.19.1 — 2026-09-11

- Reuse Canvas 2D path and clip meshes across frames on WGPU and native DX12. GPU commands keep geometry separate from the camera transform; unchanged paths no longer tessellate on every pan.
- Cache immutable `Path2D` snapshots and recognize identical rebuilt geometry. Bound the shared mesh cache to a 32 MiB charge and 32,768 entries, with scale buckets preserving curve flattening quality under zoom and DPI changes.
- Preserve existing stroke-width, clipping, image, text, alpha, and software-renderer behavior. Stroke outlines are still computed at recording time, and cached vertices are transformed and uploaded during preparation.
- Add cache invalidation and memory-limit tests, camera/DPI pixel comparisons, and a reproducible 5,000-path benchmark. On the measured Ryzen 9 7950X3D, pan preparation improves from 16.34 ms to 3.97 ms (4.1×); zoom preparation improves by 3.4–3.6×. These are CPU preparation timings, not total frame timings.

## 0.19.0 — 2026-09-10

- Add the optional `canvas` feature and a browser-like Canvas 2D API through existing element refs. Owned contexts support drawing from input handlers, timers, and workers without a required draw callback.
- Render persistent canvas surfaces on WGPU and native DX12, with shared tile antialiasing, incremental GPU updates, lazy backing allocation, and bounded queues and caches. Drawing schedules presentation without triggering layout.
- Support solid fills and strokes, reusable paths, transforms, clipping, geometry hit tests, text, and immutable images. Add asynchronous ordered snapshots, metrics observers, lifecycle diagnostics, and explicit software rendering.
- Separate raw raster transport from image codecs, allowing GPU canvases without image-decoder dependencies.
- Add canvas documentation, examples, CPU/GPU correctness checks, native capture harnesses, and a reproducible performance report. Canvas preserves pixels; applications retain their own scene objects and interaction state.

## 0.18.2 — 2026-09-10

- Maintenance release of the current source, including the U+2022 password mask default. No library behavior changes from 0.18.1.

## 0.18.1 — 2026-09-10

- Add vetoable OS and app close requests for primary and secondary windows. Decisions can be retained for an asynchronous dialog; dropped requests keep the window open. Unconditional `close()` bypasses the handler.
- Route macOS application termination (including Dock Quit) through the primary window close handler while preserving winit callbacks.
- Add declarative native macOS menus, application-menu commands, accelerators, runtime model updates, and a cloneable menu controller. Native key equivalents consume both key down and matching key up before tree shortcuts.
- Add MCP `lurq_menu`, `request_close`, and `menu_activate` for headless inspection and interaction.
- Add the lifecycle demo and native macOS integration coverage.
- Silence routine video/frame timing diagnostics by default, even under broad debug tracing. Set `LURQ_VIDEO_LOGS=1` before startup to opt in; renderer errors remain visible.
- Correct two existing window-chrome tests that assumed Windows title-bar height on macOS.
- Update the macOS native video-texture bridge to wgpu 29 HAL guards and objc2 Metal objects; remove unused legacy Core Video Metal dependencies.
- Masked inputs render U+2022 (`•`) by default (was `*`). Custom mask characters and unmasking remain supported.
- Documented font fallback for mask glyphs and added regression coverage for bullet rendering, width, caret placement, and selection deletion with ASCII and multibyte values.
