# From the Mac to the PC, 2026-11-18

What was done on a Mac on `feat/design-system` that touches Windows, what nobody compiled there, and what is left
to do on a PC. The decisions are in [decisions.md](../decisions.md); what was seen on Apple's systems is in
[apple-checks.md](../apple-checks.md).

## 1. Compile and test it on Windows first

None of this was built on Windows. Run, on the PC:

```bash
cargo build -p gallery --release
cargo test -p lurq --lib --features raster,media,screenshot,mcp,devtools
cargo test -p lurq --lib --features raster,media card_frames                  # the card's frames, on a GPU
cargo test -p lurq --lib --features raster i420                               # I420 on the card, any backend
cargo test -p lurq --test input_tests edit_commands
cargo test -p lurq --test ui_tests
cargo build -p lurq --features dx12,raster
```

What may not compile, by where it was changed without a Windows compiler:

- **`app/wgpu_render/card_frames_tests.rs`**: its `drawn` now calls the shared `app/wgpu_render/planes_drawn.rs`
  (moved out of it so the Apple test draws planes the same way). `mod planes_drawn` is
  `cfg(all(test, feature = "raster", any(all(windows, feature = "media"), target_os = "macos", target_os = "ios")))`.
- **`app/dx12_render/mod.rs`**: `ImagePixelFormat::I420` is new. The draw returns early for it (an empty box and a
  warning once, as for a `GpuViewport`); the pipeline match takes `Nv12 | I420`; the texture match returns an error
  for it.
- **`app/wgpu_render/mod.rs`**: a new cache variant `CachedImageTexture::I420` and pipeline `i420_image_pipeline`
  (all platforms), `CvRgba` (Apple only), and the NV12 instance's `opacity.zw` (the matrix and the range of a
  frame; Windows passes 0, 0, which is BT.709 of the video range, as before).
- **`app/wgpu_render/shaders/image_nv12.wgsl`** now calls `yuv_to_rgb` of `shaders/yuv.wgsl`, concatenated after
  it at pipeline creation. Same result for 0, 0. The `dx12` HLSL is untouched.
- **`app/winit_shell.rs`**: the menu's items are drained with the window in front (`drain_to`), and an item that
  asks for an edit has the pass asked for at once. `Window::has_requests` is in `Tree::needs_redraw` on every
  system.

## 2. Look at these on Windows

- **Undo groups typing by word** (decisions.md, "Undo takes back what was typed as the platform's own fields do").
  On Windows a word typed with the spaces after it is a step: type `hello big world`, Ctrl+Z three times gives
  `hello big `, `hello `, nothing. A pause of more than a second ends a step too (`App::set_typing_undo_pause`).
  This was written from what is known of WinUI's `TextBox`, Notepad and Word, not seen: compare with Notepad, and
  change `UndoGrouping::PLATFORM` (`node/node_kind/text_input/mod.rs`) if Windows does otherwise.
- **A tap outside a text input lets it go** on every system now ([mobile-decisions.md](../mobile-decisions.md), 2026-11-18): with
  a touch screen, a tap outside a focused input takes the focus from it; a finger that scrolls does not.
- **The gallery's Video Player page** plays as before (the NV12 shader was rewritten through `yuv_to_rgb`).

## 3. To do on the PC

1. **I420 in the `dx12` renderer** (decisions.md, "A frame of three planes (I420) is drawn as it is"). The `wgpu`
   renderer's way: three `R8` textures, gray then the two of color at half size, uploaded as they are
   (`write_i420_image_textures`), and the NV12 shader reading the color from two textures (`i420_shader_source`).
   In `dx12`: three SRVs, a root signature of three, and `image_nv12.hlsl` with `uv` made of two samples.
2. **The colors of `CardFrames`.** Apple's frames say their matrix (BT.709, BT.601, BT.2020) and their range
   (video, full), and the `wgpu` shader is told them in the instance's `opacity.zw` (`shaders/yuv.wgsl`; tested
   against the standards' K_R and K_B in `cv_pixel_buffer_tests.rs`). Windows' frames are told nothing yet: give
   `CardFrames` (or each `CardFrame`) a way to say them, from the decoder's `MF_MT_YUV_MATRIX` and
   `MF_MT_VIDEO_NOMINAL_RANGE`, pass them in `opacity.zw` for `CachedImageTexture::Card`, and do the same in the
   `dx12` HLSL (`image_nv12.hlsl`, which has BT.709's numbers written in).
3. **A test of I420 drawn whole**: `i420_tests.rs` checks the planes on the card and that the shader is valid; a
   readback of a drawn picture (as `blend_readback_tests.rs` does on Windows) would check the pipeline too.

## 4. What Apple's systems have now, for an app like Parties

For the record of what is the same on both sides: `CvPixelBufferFrames` draws a `CVPixelBuffer` where it is, in
NV12 of 8 or 10 bits with its matrix and range, or BGRA; `show_after(buffer, &event, value)` shows a frame once
the app's `MTLSharedEvent` reaches the value, as a fence does for `CardFrames`; `MacosCvPixelBuffer::nv12`,
`bgra`, `write_nv12` and `write_bgra` make and write such buffers. What `CardFrames` has and Apple's way has not: a
crop anywhere in the texture (Apple's crops at the top left), and a lease the decoder is told of (a pixel buffer
goes back to its pool by itself once lurq lets it go, after the next frame is drawn). Neither side reads HDR's
transfer (PQ, HLG).

## 5. Done on the PC, 2026-11-18

On `claude/apple-to-pc-5b0fa9`, on Windows 11 (build 26100), debug builds.

**Compiled and tested (section 1).** Everything of section 1 compiled on Windows as it came from the Mac:
`card_frames_tests.rs` with the shared `planes_drawn.rs`, `ImagePixelFormat::I420` in `dx12_render`, the caches and
pipelines of `wgpu_render`, `image_nv12.wgsl` through `yuv.wgsl`, and the menu's draining in `winit_shell.rs`. What
was fixed: a warning on Windows (`i420_tests.rs` dropped a shader module wgpu marks `must_use`), and the time a run
of typing was typed at, which was `std`'s `Instant` and panics in the browser build (`crate::time::Instant` now).
`card_frames` 7 passed, `i420` 1, `ui_tests` 2436, `input_tests` 629 (after the change of undo below),
`--lib --features raster,media,screenshot,mcp,devtools` 1261 passed, ten runs in a row, once the two things below
were fixed.

**The whole `--lib` run ended with an access violation now and then** (`STATUS_ACCESS_VIOLATION`, two runs of four
here, and on the coordinator's checkout), and where it did not, tests of `card_frames_tests` failed now and then.
Two causes, both in the tests and neither in what an app runs:

- `i420_tests.rs` made its device of `wgpu::Instance::default()`: an instance among every backend, which on
  Windows has Vulkan in it, a second instance of Vulkan beside the one the tests of a card's frames draw through.
  Letting it go at the end of the test ended the program, as two engines' instances did before the instance was
  shared (`new_instance` in `wgpu_render/mod.rs` says so). Seen apart: the Vulkan tests of `card_frames_tests` with
  the I420 test crashed 6 runs of 30, without it 0 of 30, and with the fix 0 of 40. The test takes the program's
  one instance among the backends lurq takes by itself now (`new_instance(default_backends())`).
- `viewport_tests.rs` starts real engines, and an engine that starts says for the whole program which card draws
  the frames of a card and whether it draws one NV12 texture (`card_frames::announce`). The tests of such frames
  run one at a time for that (`card_read::ON_THE_CARD`); the viewport's did not, and a test under Vulkan that was
  told "NV12" by an engine of theirs failed its assertions (10 of 16 runs) or had its media give Vulkan an NV12
  texture, which an RTX loses its device for. Each test of `viewport_tests.rs` holds `ON_THE_CARD` now.

Not changed: the tests behind `#[ignore]` that make an instance among every backend (`quad_tests.rs`,
`canvas/tests.rs`, `canvas/camera_tests.rs`) are run alone, where there is no second instance.

**Looked at (section 2), through the gallery's MCP server.**

- **Undo groups typing by word: Windows does otherwise.** Windows' own engines were asked in hidden controls of a
  script's own ([evidence](../../docs/evidence/windows-undo-grouping/README.md)): Windows' RichEdit (WordPad's,
  what XAML's `TextBox` is built on) and the Office RichEdit Windows 11's Notepad ships both take back
  `hello big world` in one step. `UndoGrouping::PLATFORM` is `Runs` on Windows too (decisions.md, "On Windows a
  run of typing is one step of undo, as on a Mac"). In the gallery's Input on Windows now: `hello big world`, Ctrl+Z
  gives nothing; `abc`, 1.4 s, `def` gives `abc`, then nothing (lurq's own pause); `hello world` and three
  Backspaces gives `hello world`, then nothing, as Notepad's engine does. Not seen: Notepad's own window, which was
  not driven; Ivan can type in it and press Ctrl+Z.
- **A tap outside a text input lets it go**, through `lurq_touch` on the Input page: a tap on the input focuses it,
  a finger that scrolls the page leaves it the focus, a tap on the empty stage beside it takes the focus away.
- **The Video Player page plays** as before: the gallery's clip, muted, plays on (0:00 to 0:02 and on), its bars
  red, green, yellow, blue, magenta and cyan; the frames of a decoder of your own are drawn on the card and the one
  of another card through memory.

**The work (section 3).**

1. **I420 in `dx12`**: three `R8` textures uploaded as they are, three SRVs under a root signature of three
   (`ImagePipeline::new_i420`), and `image_nv12.hlsl` compiled with `THREE_PLANES`, reading U and V from two
   textures. The warning and the empty box are gone.
2. **The colors of `CardFrames`**: `images::YuvColors` (`YuvMatrix::Bt709`, `Bt601`, `Bt2020`, and `full_range`),
   told of each frame with `CardFrame::colors`. Windows' media reads them from `MF_MT_YUV_MATRIX` and
   `MF_MT_VIDEO_NOMINAL_RANGE` (BT.709 of the video range where the decoder says nothing), and its video processor,
   where it converts for Vulkan, reads by them (BT.2020 as BT.709 there: its color space has no more). Both
   renderers pass them in `opacity.zw` for a card's frame; `image_nv12.hlsl` has the `yuv_to_rgb` of `yuv.wgsl`
   in place of BT.709 written in.
3. **A readback test** (`app/yuv_readback_tests.rs`, `--ignored`, as `blend_readback_tests.rs`): an I420 picture
   of four quarters drawn whole by each renderer, three frames, the second from its textures and the third after a
   new version, against BT.709 of the video range; and a card's frames of four colors (BT.709, BT.601, BT.2020,
   BT.601 of the full range) drawn by each renderer against the standards' K_R and K_B. 4 passed.

**Left.**

- A picture of memory (`ImageData::streaming_nv12`, `streaming_i420`) says no colors: it is drawn as BT.709 of
  the video range, so a video of BT.601 that goes through memory and one that stays on the card differ, and a
  decoder of software (dav1d) cannot say its matrix. It needs the colors on `ImageData` and through `ImageCmd`.
- Nothing of this needs a Mac again: what changed on Apple's side is a public type and the clock of undo, and
  `cargo check` for `aarch64-apple-darwin` and `aarch64-apple-ios` passes.
