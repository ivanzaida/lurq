# Changelog

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
