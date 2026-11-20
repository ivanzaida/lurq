---
title: App Runtime
description: App, Tree, render engine factories, windows, frame flow, profiling, and element lookup.
---

# App Runtime

The public runtime surface is split between `App`, `Tree`, and the shell.

## App

`App` is a cloneable handle to shared application services. Mounted component contexts retain those services, so timer, future and input-event renders can use `ctx.app_ref()` and `ctx.app_ref_mut()` after the caller moves or drops its handle. Clones observe changes to fonts, scale, resource configuration and storage-backend selection. `persistent_storage()` returns an owned clone of the currently selected backend handle; an already returned handle continues to refer to that backend if the App later selects another.

`App` stores services shared by a tree pass:

- glyph engine and loaded fonts,
- theme,
- optional scale override,
- optional resource loader and decoded image/SVG caches,
- menus, secondary-window requests, optional storage and Tokio runtime handles.

```rust
let mut app = lurq::app::App::new();

app.load_font_file(std::path::Path::new("assets/Inter.ttf"));
app.register_font("ui", "Inter");

app.set_resource_root(std::path::PathBuf::from("assets"));
```

## Tree

`Tree` stores the retained UI state. It owns:

- root component or static root element,
- component contexts and reactive subscriptions,
- retained nodes,
- layout state and layout cache,
- render engine instance and render engine factory,
- input state,
- animation and transition engines,
- perf overlay state,
- secondary windows and optional DevTools metadata.

```rust
let mut tree = lurq::app::Tree::new();
tree.mount_root::<RootComponent>(&mut app, RootProps::default());
```

For static UI without a component root:

```rust
tree.set_root(lurq::components::Text::new("Hello"));
```

## Render Engine Factory

Use a factory, not a prebuilt render engine. Secondary windows, including DevTools, can inherit the same renderer choice by asking the factory for their own engine instance.

```rust
tree.set_render_engine_factory(|| {
  Box::new(lurq::app::wgpu_render::WgpuRenderEngine::new())
});
```

WGPU creates its instance on the first frame. On Windows it defaults to DX12, avoiding the reported Vulkan-loader race when independent windows render and tear down concurrently. Other platforms retain the full backend selection. Select backends before the first frame if your host needs a different configuration:

```rust
use lurq::app::wgpu_render::{WgpuBackends, WgpuRenderEngine};
let engine = WgpuRenderEngine::new().with_backends(WgpuBackends::DX12);
```

Excluded backend loaders are never initialized by that engine. Explicitly opting into Vulkan on Windows also opts back into its platform loader behavior.

An engine asks for its GPU device at its first frame and waits for it. The engines of an app's windows share one device: an engine takes the device of another window's engine on its thread that was asked for among the same backends with the same optional features and limits, while one draws, so a texture of the app's can be shown in any window. `WgpuRenderEngine::new().with_own_device()` keeps a window's device apart. `WgpuDevice` is a device made ahead: an app awaits `WgpuDevice::request()` and gives the device to each engine the factory makes.

```rust
use lurq::app::wgpu_render::{WgpuDevice, WgpuRenderEngine};

let device = WgpuDevice::request().await?;
tree.set_render_engine_factory(move || {
  Box::new(WgpuRenderEngine::new().with_device(device.clone()))
});
```

On a desktop this is optional and moves the wait off the first frame. In a browser it is how an engine gets a device at all, since a page cannot wait inside a frame; see [Web](../web/#the-device). The engine's backends and optional features and limits are then the device's: `WgpuDevice::request_with(backends)` chooses the backends, and `WgpuDevice::backend()` says which the device draws through.

On Windows with `dx12`:

```rust
tree.set_render_engine_factory(|| {
  Box::new(lurq::app::dx12_render::Dx12RenderEngine::new())
});
```

An app that draws part of its window with its own wgpu code (a 3D scene, a game or editor viewport, custom shaders) gives each engine the factory makes a frame extension, `WgpuRenderEngine::new().with_frame_extension(..)`, and shows what it draws in a `GpuViewport`. The engine of lurq's DevTools window is made by the same factory and runs none of them. See [Drawing With Your Own GPU Code](../gpu-viewport/).

When a window's graphics device is lost (its card was reset or taken away), lurq replaces the window's engine with one the factory makes and draws the window again; the window's `on_app_event` handler then hears `AppEvent::GraphicsDeviceLost`, on a desktop as on a phone. See [A Lost Device](../gpu-viewport/#a-lost-device).

## Winit Shell

`WinitWindow` creates the OS window, forwards events to `Tree`, and calls `Tree::pass`.

```rust
WinitWindow::new(app, tree)
  .with_title("lurq app")
  .with_size(1200, 800)
  .with_min_size(800, 500)
  .with_title_bar_color(lurq::node::color::Color::from_hex("#101215"))
  .with_icon(lurq::app::WindowIcon::from_rgba(vec![255, 0, 0, 255], 1, 1))
  .with_corner_radius(lurq::app::WindowCornerRadius::RoundedSmall)
  .with_decorations(false)
  .run();
```

`with_icon` sets the window icon: on Windows the title bar's small icon and the taskbar's and Alt+Tab's big icon, on X11 the window icon; macOS shows the application bundle's icon instead and ignores it. A title bar the app draws with `ui::WindowChrome` shows it too, on every platform.

The shell drives timers, async work, animation deadlines, and requested redraws. It waits when idle; continuous video or an installed `on_tick` callback can keep the loop polling. Use `on_tick` only when the app needs continuous custom work.

Runtime window commands requested through `ctx.window()` are applied by the winit shell. This includes closing, minimizing, fullscreen toggles, decoration toggles, the window title, native title bar color, native corner radius, window icon, moving, resizing, native platform window drag or resize requests for custom chrome, synthetic input injection, and — with the `screenshot` feature — full-window, region, and node-scoped frame capture (see [Ctx § Window](../ctx/#window)). `start_drag()` asks the shell to begin an OS-level window move, and `start_resize(direction)` asks it to begin an OS-level edge or corner resize. `stop_drag()` is available for portable shells that track drag state manually.

### A Phone On A Desktop

`App::emulate(Device::phone())` runs an app on a desktop as it would be on a phone, to try its touch forms and its
gestures with a mouse; `Device::tablet()` as on a tablet. Any app does the same with no code when `LURQ_DEVICE=phone`
(or `tablet`) is in its environment, read as its window opens; the gallery takes `--device phone`.

```rust
use lurq::app::{App, Device};

let app = App::new();
app.emulate(Device::phone());
// .. mount the root and run the window as always.
```

While it emulates a device:

- **The window is the device's size**, in logical pixels (`Device::size`: 390 by 844 for the phone, 820 by 1180 for
  the tablet), drawn at the monitor's scale. `Device::scale` is the device's, for an app that reads it.
- **The safe area is none.** A desktop's window has no status bar and no home indicator, so nothing keeps clear of
  them: `ctx.safe_area()` is zero, and the bar, the tab bar, a sheet and the lightbox go to the window's edges. To see
  where a phone's would be, give the device its screen's insets, `Device::phone().with_safe_area(Device::PHONE_SAFE_AREA)`
  (a status bar of 47 at the top and a home indicator of 34 at the bottom; `TABLET_SAFE_AREA` is 24 and 20), or put
  `LURQ_DEVICE_INSETS=1` beside `LURQ_DEVICE`: they then reach `ctx.safe_area()` and those components as a phone's
  do, and the band they keep is the app's background. Until 0.64 the presets had them. A phone keeps the system's
  own.
- **The pointer is pinned `Coarse`**, so every component takes its touch form.
- **The mouse is fingers.** Its moves and its left button reach the tree only as fingers, through the same path a
  touch screen's take, and never as the mouse: a click is a tap, a double click a double tap, a drag a pan, a
  swipe or a scroll with momentum, a drag from an edge of the window an edge pan, a press held still a long press.
  **Alt**+drag is two fingers, the second mirrored about where the drag began, so dragging away from that point or
  toward it pinches and around it rotates; **Alt+Shift**+drag is two fingers side by side; **Alt+Shift+Ctrl**+drag
  three side by side, and **Alt+Shift+Ctrl** with the wheel and no button pinches three about the mouse (away
  spreads them, a paste; toward closes them, a copy), lifted when the keys are let go. While a modifier is held the fingers are drawn as dots. A finger hovers nothing, so nothing
  shows its hovered style. The wheel still scrolls and the keyboard still types.

What is not emulated: the on-screen keyboard, the system's own gestures, a pen's hover, and a phone's speed.

**For a shell of its own.** A finger reaches the tree through `Tree::finger(id, phase, x, y, force)`, in physical
pixels: the finger's own events (`on_touch_*`), and where nothing took it the pointer it stands for (a press, a tap,
a scroll with its momentum, a long press) and the gestures. `Tree::finger_tick(now)` runs what time brings (a long
press, momentum, a list springing back) and `Tree::finger_deadline()` says when it next has something to do. The
winit shell, the emulator and MCP's `lurq_touch` all go through these; a test drives them on its own clock
(`Tree::set_press_clock`). `Tree::pen_hover(x, y)` and `pen_hover_ended()` are a pen in range that touches nothing.

## Frame And Redraw Flow

`Tree` sets `needs_redraw` when state changes:

- signal/store/memo dependency updates,
- input, hover, active, focus, scroll, drag, or cursor changes,
- layout-affecting ref mutation,
- perf overlay updates,
- DevTools pick/overlay state.

The shell observes `needs_redraw`, requests a redraw, then calls `Tree::pass(app, window)`.

Animations and transitions do not set `needs_redraw`. A pass that ticks them records when they next draw something else, `Tree::next_timeline_change()`: the next step of an `Easing::Steps` span, the end of a delay, or at once for a span that changes continuously. The shell schedules the next frame for then, at most one per display refresh, and sleeps until it is due: for a continuous animation it wakes half a refresh after the last present, and the swapchain puts the frame on the next vblank. A minimized or occluded window gets no such frames until it shows again (see [Frame Pacing](../animation-transforms/#frame-pacing)).

`Tree::pass(...)` returns a `PassReport` describing whether the pass was required, whether it rendered, whether it reused a cached render list, whether layout updated or recalculated, and which runtime reasons contributed to the pass. `WinitWindow::on_paint` receives the same report after a presented frame:

```rust
WinitWindow::new(app, tree)
  .on_paint(|tree, delta, report| {
    if report.layout_recalculated {
      eprintln!("layout recalculated after {:?}", delta);
    }

    let _ = tree.frame_count();
  })
  .run();
```

## Input Dispatch

Pointer input is resolved against the latest layout and hit-tested in visual coordinates. A press, a release or a wheel that comes before any frame laid out what an earlier event rebuilt (a hover that opened a box or pushed a row away) lays it out first, so it lands on what the next frame shows. The runtime tracks hover, active, focus, drag, scroll, cursor, text selection, and text click counts across retained nodes. Mouse handlers run before pointer defaults such as input focus, text selection, form submit, and outside-click overlay dismissal, so handlers can call `event.prevent_default()` to block those defaults.

User keyboard handlers run before built-in keyboard defaults. Built-in text behavior handles caret movement, selection replacement, undo/redo, and clipboard shortcuts when the `clipboard` feature is enabled; which keys are undo, redo, cut, copy, paste and select all is `KeyboardEvent::edit_command()`'s, and what an element does for them is its `on_undo`, `on_redo`, `on_select_all`, `on_copy` and `on_paste` (see [Editing commands](../styling-events/#editing-commands-undo-redo-copy-cut-paste)). Call `event.prevent_default()` from an `on_key_down` handler to block those built-in defaults for that key. `TextInput::on_input` runs inside the text-edit default, before the edit is applied, and can mutate the input signal or prevent that edit. Scroll handlers also run before default scroll movement.

## Sizing And Scale

The shell updates the tree from the window before each pass:

```rust
tree.set_scale_factor(window.scale_factor() as f32);
tree.resize(size.width, size.height);
```

Tests can override layout constraints directly:

```rust
tree.set_layout_constraints_override(Some(lurq::layout::Constraints::tight(
  lurq::layout::Size::new(800.0, 600.0),
)));
```

## Element Lookup

Use `find_element` when integration code or tests need a computed rect.

Predicate-based `find_element` and `find_element_mut` use the last completed layout and do not flush pending component renders. Run a pass first when state has changed. The by-ID/by-class lookups below flush dirty subtrees themselves.

```rust
let found = tree.find_element(|el| el.text_content() == Some("Save"));

if let Some(found) = found {
  let bounds = found.bounds();
  println!("{}x{} at {}, {}", bounds.width, bounds.height, bounds.x, bounds.y);
}
```

Use `find_element_mut` only for imperative layout overrides. Declarative component state is the normal path.

Nodes tagged with `.id("...")` / `.class("...")` support browser-style lookup, mutation, and typed interaction:

```rust
let save = tree.get_element_by_id("save");                 // first match in tree order
let rows = tree.get_elements_by_class_name("row");         // all matches in tree order

let mut handle = tree.get_element_by_id_mut("save").unwrap();
handle.click();                                            // DOM el.click() semantics
handle.set_background("#ef4444");                          // transient direct mutation

tree.get_element_by_id_mut("email").unwrap()
  .as_text_input().unwrap()
  .set_value("ada@example.com");                           // signal-backed, no on_input
```

See [Retained Nodes](../retained_nodes/#ids-and-classes) for the full contract (transiency, duplicate ids, pre-layout behavior).

## Perf Overlay

The runtime has a built-in frame perf overlay.

```rust
tree.draw_perf_overlay();
```

Enable the `perf_profile` Cargo feature for frame timing and memory instrumentation. It is independent of `devtools`; there is no runtime `App::set_profiling_enabled` switch. With that feature, profiling data is available through:

```rust
let profile = tree.last_profile();
```

The profile records high-level timings such as layout, resolve, glyph, upload, encode, submit, present, and memory counters used by DevTools.

## Performance Notes

The plain-text pipeline reuses shaped paragraphs, wrapped layouts, and caret geometry. Documents (plain texts of 1,024 bytes and more) share a cache with a 48 MiB accounted budget per glyph engine and a 64-entry limit; shorter texts are kept shaped in a 4 MiB cache of their own and laid out again, not shaped again, when their width or size changes. See [Text Pipeline Optimization](../text-pipeline-optimization/) for implementation details and dated measurements; profile the current app and renderer before choosing the next optimization.

The first text of each font family, weight and slant is slow: the text engine scores every font face on the system for it, 4 to 11 ms a combination on a Windows desktop, on the frame that first shows it. `App::warm_text_styles` does that ahead of time. It blocks for as long, so call it on a thread of your own, after loading fonts and before the first frame:

```rust
let warming = app.clone();
std::thread::spawn(move || {
  let bold = TextStyle { weight: FontWeight::Bold, ..TextStyle::default() };
  warming.warm_text_styles(&[TextStyle::default(), bold]);
});
```

Size, colour and spacing do not matter; one style per family, weight and slant is enough. Loading a font afterwards forgets what was prepared.

## DevTools Window

With `devtools` enabled:

```rust
tree.mount_devtools(&mut app);
```

DevTools is represented as a secondary tree. The shell does not need special inspector logic; it only manages secondary windows and renders each tree.
