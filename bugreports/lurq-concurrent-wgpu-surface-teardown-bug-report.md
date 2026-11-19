# Lurq Bug Report: Two Concurrent `WgpuRenderEngine`s Crash The Process With STATUS_ACCESS_VIOLATION

## Status

Open. Reported 2026-09-14 by the Kontur desktop team (KONTUR-94), against `lurq 0.19.2` on Windows.

## Summary

Two `WgpuRenderEngine`s alive at the same time in one process kill the process with
`STATUS_ACCESS_VIOLATION` (`0xc0000005`). The fault is in the **Vulkan** loader, destroying a Vulkan
instance neither engine ever rendered through: this machine presents through D3D12.

`WgpuRenderEngine::new` hard-codes `instance_descriptor.backends = wgpu::Backends::all()`
(`src/app/wgpu_render/mod.rs:383-391`), so every engine stands up a `wgpu::Instance` over DX12 **and**
Vulkan **and** GL, creates a `VkSurfaceKHR` for the window alongside the D3D12 swapchain, and spawns a
`wgpu-hal WGL Instance Thread`. Only the D3D12 half is used. The unused Vulkan half is what crashes.

There is no public way for a consumer to narrow the backends: `new()` is the only constructor, and the
builders are `with_frame_extension`, `with_optional_device_features`, `with_optional_device_limits`
and `with_present_mode`. Narrowing the same reproduction to `wgpu::Backends::DX12` makes it clean in
30 of 30 runs, so the choice of `all()` is the whole difference between crashing and not.

This is filed rather than worked around in our tests. We have serialised our own pixel fixture so only
one native surface is alive at a time, which is enough for us; it is not a fix, and an application that
opens a second window on a second engine has no such option.

## Environment

- OS: Windows 11 Pro 10.0.26200
- Crate: `lurq` `=0.19.2`, default features off, features:
  `winit, wgpu, dx12, i18n, serde, clipboard, persistent_storage, router, tokio, form, image, canvas`
- `wgpu 29.0.3`, `ash 0.38.0+1.3.281`, `raw-window-handle 0.6.2`
- Renderer: `WgpuRenderEngine`, selected adapter
  `AdapterInfo { name: "NVIDIA GeForce RTX 5080", device_type: DiscreteGpu, driver: "32.0.15.9649", backend: Dx12 }`
- Windows: hidden owned `WS_POPUP` 256 × 256 Win32 windows, one per engine
- Profile: `cargo` dev/debug

## Actual Behaviour

Faulting instruction and stack, from `cdb` on our `canvas_manipulation` suite. `feeefeee feeefeee` is
the Windows free-heap fill pattern — the loader state being read has already been released:

```text
(953c.247c): Access violation - code c0000005 (first chance)
vulkan_1!vkResetEvent+0x32490:
00007fff`21bc3870 483938          cmp     qword ptr [rax],rdi ds:feeefeee`feeefeee=????????????????

#  4  Id: 953c.247c  "a_band_and_a_hover_outline_draw_over_the_page_and_leave_it_unchanged"
vulkan_1!vkResetEvent+0x32490
vulkan_1!vkResetEvent+0x13b0a
ash::extensions_generated::ext::debug_utils::Instance::destroy_debug_utils_messenger+0x48
wgpu_hal::vulkan::instance::impl$3::drop::closure$0+0x17
wgpu_hal::vulkan::instance::impl$3::drop+0x44
core::ptr::drop_in_place<wgpu_hal::vulkan::InstanceShared>+0x1f
core::ptr::drop_in_place<wgpu_hal::vulkan::Instance>+0xe
core::ptr::drop_in_place<wgpu_core::instance::Instance>+0x2f
core::ptr::drop_in_place<wgpu::api::instance::Instance>+0xe
core::ptr::drop_in_place<lurq::app::wgpu_render::WgpuRenderEngine>+0x51
core::ptr::drop_in_place<alloc::boxed::Box<dyn$<lurq::app::render_engine::RenderEngine>>>+0x3c
core::ptr::drop_in_place<lurq::app::runtime::Tree>+0x58
```

Under `cargo test` the same run is reported as:

```text
error: test failed, to rerun pass `-p kontur-desktop --test canvas_manipulation`

Caused by:
  process didn't exit successfully: `...\canvas_manipulation-308941d93419b81a.exe`
  (exit code: 0xc0000005, STATUS_ACCESS_VIOLATION)
```

A second shape of the same fault appears when the second engine is *built* rather than destroyed —
`vkEnumeratePhysicalDevices` reading freed loader state during `Instance::request_adapter`:

```text
vulkan_1!vkResetEvent+0x32490
ash::instance::Instance::enumerate_physical_devices+0x16
wgpu_hal::vulkan::instance::impl$4::enumerate_adapters+0x5e
wgpu_core::instance::Instance::request_adapter+0xd31
wgpu::api::instance::Instance::request_adapter+0x3d
```

## Expected Behaviour

Building a second `WgpuRenderEngine` while a first one is alive either works or fails with an error.
It should not fault the process, and least of all inside a backend the application is not rendering
with.

## Minimal Reproduction

Two threads, each building a `Tree` with a `WgpuRenderEngine` and its own hidden Win32 window, each
running one pass, both alive at a barrier, then both dropped. No canvas, no snapshot, no application
code. **7 of 60 fresh processes died with `0xc0000005`; 0 of 20 died when the same two threads were
gated so the second engine is built only after the first is gone.**

```rust
// Cargo.toml: lurq = { version = "=0.19.2", default-features = false, features = [
//   "winit", "wgpu", "dx12", "i18n", "serde", "clipboard", "persistent_storage",
//   "router", "tokio", "form", "image", "canvas" ] }
// plus raw-window-handle = "=0.6.2" and windows-sys = "=0.61.2".
use lurq::{app::{App, Tree}, components::Stack};

fn main() {
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let threads: Vec<_> = (0..2)
        .map(|n| {
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut app = App::new();
                let mut tree = Tree::new();
                tree.resize(256, 256);
                tree.set_render_engine_factory(|| {
                    Box::new(lurq::app::wgpu_render::WgpuRenderEngine::new())
                });
                tree.set_root(Stack::new().width(256.).height(256.));
                let window = Window::new();          // hidden WS_POPUP 256x256, HasWindowHandle
                tree.request_redraw();
                tree.pass(&mut app, &window);
                barrier.wait();                      // both engines alive here
                drop(tree);
                drop(window);
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
}
```

The full sources, including the `Window` helper, are in the Kontur repository at
`docs/evidence/kontur-94/repro/`.

## Where The Defect Is

We narrowed it with a second reproduction that contains **no lurq at all** — plain `wgpu 29.0.3`, two
threads, the same barrier — and ran each shape 30 times:

| What each thread holds | `backends` | Crashes |
| --- | --- | --- |
| `Instance` only | `all()` | 0 of 10 |
| `Instance` + adapter + device | `all()` | 0 of 30 |
| `Instance` + Win32 `Surface` + `compatible_surface` adapter + device + `configure` | `all()` | **3 of 30** |
| the same, narrowed | `DX12` | 0 of 30 |

So the fault itself is below lurq: it is two concurrently live `wgpu::Instance`s that each own a
`VkSurfaceKHR`, and it does not need lurq's code to happen. What lurq owns is that it puts every
consumer in that position on purpose and gives no way out — an application that never asked for Vulkan
gets a Vulkan instance and a Vulkan surface per engine, which are exactly the objects that fault.

## Suggested Fix

In rough order of preference:

1. **Let the consumer choose the backends.** A `WgpuRenderEngine::with_backends(wgpu::Backends)` in the
   same builder family as `with_present_mode`. Our machine is clean in 30 of 30 with `Backends::DX12`.
2. **Do not enumerate backends that will not be presented with.** `all()` costs a Vulkan instance, a
   Vulkan surface and a `wgpu-hal WGL Instance Thread` per engine on every Windows run, used or not.
3. **Share one `wgpu::Instance` across engines in a process**, so a second engine does not create a
   second Vulkan instance at all. This also fixes multi-window applications rather than only tests.
4. If none of those, **document the constraint**: one `WgpuRenderEngine` per process on Windows.

## Two Notes For Whoever Picks This Up

- `Tree::set_render_engine_factory` builds the engine as it stores the factory
  (`src/app/runtime.rs:1233`), so the `wgpu::Instance` — and its Vulkan half — exists from that call,
  not from the first `pass`. We tried to guard against the second engine after installing the factory
  and the guard ran too late; the instance was already there. Worth a line in the docs regardless of
  this bug.
- `Dx12RenderEngine` is unaffected. The same suite, the same parallelism, the same windows, on
  `Dx12RenderEngine`: 0 crashes in 10 runs. The software backend is unaffected too, 0 in 5.

## How To Tell It Is Fixed

1. The minimal reproduction above survives 60 consecutive fresh processes.
2. `KONTUR_TEST_CANVAS_BACKEND=wgpu cargo test -p kontur-desktop --test canvas_rendering
   --test canvas_manipulation --test canvas_handles --test canvas_limits --test canvas_selection
   --test canvas_text --test canvas_retention --test text_pipeline` passes at the harness's default
   thread count, with the Kontur fixture's serialisation removed.
