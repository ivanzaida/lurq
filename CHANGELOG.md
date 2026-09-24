# Changelog

## 0.22.1 — 2026-09-24

- Fix a press that stayed held after moving or resizing the window from custom chrome. The native move or size loop (Windows `WM_NCLBUTTONDOWN`, macOS `performWindowDragWithEvent:`, X11/Wayland `drag_window`) consumes the button release, so lurq never saw it. The pressed element stayed active, and a text-selection, slider, scrollbar or `on_drag_*` session begun by the press kept following the pointer. The next release then completed the stale press and could fire a click or a drop. When `start_drag` or `start_resize` hands the press to a native loop, the winit shell now ends it with the new `Tree::mouse_press_taken_by_os`. That runs the normal mouse-up path without a click, ends drag sessions as a miss, and clears the click suppression they arm. Present since custom chrome was added.

## 0.22.0 — 2026-09-23

- Make `WindowControls` styleable. `button_width`, `button_height`, and `button_size` size the Windows-style buttons (default 46 wide, title-bar height). `content(WindowControlKind, WindowControlContent)` replaces what a control draws: a glyph in a typography role (`WindowControlContent::glyph`, for example an icon-font code point with an `Extra` typography role) or an app-built element (`WindowControlContent::element`, which receives the foreground color). `foreground`, `background`, `hover_background`, and `active_background` set every control at once, including close, so the red close hover can be replaced; `colors` and `control_colors` take a `WindowControlColors` for all controls or one. Colors accept `Color`, hex strings, and `PaletteColor` roles, including `PaletteColor::Extra`. Hover and active change the background only; the foreground stays constant. Without these calls the controls look as before. `WindowControls` moves to its own module; the `lurq::components` paths are unchanged.
- Make the macOS traffic lights configurable with `WindowControls::traffic_lights(TrafficLightColors)`. The default minimize color changes from `#ffbd2e` (the pre-Big Sur value) to `#febc2e`, matching the macOS 11+ close `#ff5f57` and zoom `#28c840` lurq already used. Apple does not publish these colors; they are sampled from the system controls.
- Stop resize handles from shrinking the content area. The new `ResizeHandlePlacement` (`WindowChromeProps::resize_placement`, `WindowChrome::resize_placement`) defaults to `Overlay`: the invisible edge and corner hit zones cover the outermost pixels of the content, which now fills the window below the title bar. Before, content and overlays were inset by the handle size on the left, right, and bottom, so a 1440-wide window gave 1434 px of content. `ResizeHandlePlacement::Inset` restores that layout. `WindowChromeMetrics::resize_inset` returns 0 for `Overlay`; the new `resize_handle_size_for(WindowInfo)` returns the active hit-zone size.
- Hide the Windows 11 compositor border around custom chrome. DWM draws a 1px border around undecorated windows, so a `WindowChrome` with `ChromeBorderPolicy::Hidden` still showed an outline. Add `WindowBorderColor` (`Default`, `None`, `Color`) with `WindowHandle::set_border_color` and `border_color`, mapped to `DWMWA_BORDER_COLOR` by the winit shell on Windows 11 build 22000+ and ignored elsewhere. `WindowChrome` sets `WindowBorderColor::None` when custom chrome is active; the frame outline comes only from `ChromeBorderPolicy`.
- Fix a faint 1px line on every window edge (and a soft 2px edge on every rect). The quad shaders (wgpu and DX12) average four half-pixel subsamples but gave each one a full-pixel anti-alias ramp, so a pixel entirely inside an axis-aligned edge got 87.5% coverage and the pixel outside got 12.5%. A window-sized root rect therefore let the clear colour show through its outermost pixels, maximized or not. Each subsample now uses a half-pixel ramp: pixel-aligned edges are exact, and curved and fractional edges stay anti-aliased. Rect edges throughout the UI render slightly crisper.
- Resolve the frame clear colour through the theme and past the overlay host. It was the root node's concrete background only, so a palette-coloured root, or any root while a `Modal`/overlay was mounted (which includes every `WindowChrome`), cleared to white.
- Fix live window resizing on Windows from custom chrome. Dragging a resize handle stretched the last frame for the whole drag, then froze the app for up to several seconds before it drew the new size. The shell started the native sizing loop with a synchronous `SendMessageW`, so the loop ran inside the lurq event handler, where winit queues every event. No frame was drawn during the drag, and afterwards the wgpu engine reconfigured its surface once per queued size event, which took about 2.4 ms each in a debug build. Now the shell posts the non-client press, at the real cursor position instead of (0, 0), so the loop runs from winit's pump. A `Resized` for the window's current size is painted immediately; WM_PAINT is starved during the loop. `WgpuRenderEngine::resize` records the size, and the next frame reconfigures the surface once. `start_drag` uses the same path. Present since custom chrome was added, not a regression.
- Breaking: `WindowChromeProps` and `WindowChromeMetrics` struct literals need `resize_placement` or `..Default::default()`.

## 0.21.0 — 2026-09-23

- Add the full CSS named font-weight set (`ExtraLight`, `SemiBold`, `ExtraBold` join the existing names) and `FontWeight::Numeric(u16)` for any weight, clamped to `1..=1000`. `FontWeight::value()` returns the number; weights compare, hash, and key text caches by value, so `Numeric(600) == SemiBold`. `Medium` now requests 500 instead of 400.
- Select the nearest loaded face for every weight. cosmic-text (0.12, and still 0.19 for static faces) only takes a face from the requested family when its weight matches exactly and otherwise falls through to fallback families (which is why `Medium` used to request 400). Text and canvas text now resolve the weight with fontdb's CSS font-matching query first, cached per family and cleared when fonts load. Loaded Medium and SemiBold faces render; a family without them uses its nearest face instead of another family. Text styled `Medium` changes appearance only where the family has a 500 face.
- Add application-defined `Extra` roles to `TypographyStyle`, `RadiusSize`, `SpacingSize`, and `BorderSize`, stored in new `extra` maps on `ThemeTypography`, `ThemeRadii`, `ThemeSpacing`, and `ThemeBorderSizes`. Each role has an `extra(name)` constructor and converts from `&str` and `Arc<str>`; the tables gain `try_get`, `resolve`, and `try_resolve`. Names are interned so these roles stay `Copy`. A missing name follows the palette: table `get`/`resolve` panic, while nodes resolve an unknown radius, spacing, or border size to `0` and an unknown typography variant to the default text style. `Breakpoint` has no extras because `Responsive` orders overrides by the enum.
- Document `PaletteColor::Extra`, the new extras, and font-weight matching in the Theme guide.
- Upgrade cosmic-text from 0.12 to 0.19.0 and swash from 0.1.19 to 0.2.10, the version cosmic-text uses, so lurq's rasterizer and cosmic-text share one swash and one fontdb (0.23, also shared with usvg now). Shaping moves from rustybuzz to harfrust; the development-profile overrides add `harfrust`. The text, input, layout, Markdown, and canvas regression suites pass with unchanged expected values, so measurement, wrapping, caret, and selection geometry match 0.12 for the covered fonts and scripts. Text ending in a line break still ends at that break (cosmic-text 0.19 would add an empty final line). The rasterizer now places variable fonts at the `wght` position cosmic-text shapes them with, so outlines match advances; lurq still requests the matched face's own weight, so variable fonts render as before. The cap-height fallback for fonts without the metric reads the Latin H bounds with skrifa.
- Add letter spacing. `TextStyle::letter_spacing` is extra space after every glyph in logical pixels (negative tightens; default `0.0`); like CSS it applies to spaces and after the last glyph of a line. It scales with the display scale factor like `font_size`, is part of every text cache key, and measurement, wrapping, painting, carets, hit testing, and selection use the spaced advances. `Text::letter_spacing(f32)` overrides the resolved style (typography roles included), `TextInput::letter_spacing(f32)` sets the value and placeholder styles, `MarkdownTextStyle::letter_spacing` and `CanvasFont::letter_spacing` cover Markdown and canvas text. Rich-text spans space in pixels whatever their font size.
- Breaking: `TextStyle`, `MarkdownTextStyle`, and `CanvasFont` struct literals without `..Default::default()` (or `CanvasFont::new`) need `letter_spacing`. `FontWeight::to_cosmic()` and `FontStyle::to_cosmic()` return cosmic-text 0.19 types.
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
