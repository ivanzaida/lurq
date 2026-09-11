# Changelog

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
