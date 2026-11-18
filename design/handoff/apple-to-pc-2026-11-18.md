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
