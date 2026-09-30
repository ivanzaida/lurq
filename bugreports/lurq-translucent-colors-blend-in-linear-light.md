# Lurq Bug Report: Translucent Colors Blend In Linear Light, Not Like CSS

## Status

Resolved on branch `claude/context-and-scrim` (base `9c61a9e`, 0.22.1).

Fix:

- [wgpu renders the surface through its non-sRGB format.](../crates/lurq/src/app/wgpu_render/mod.rs) The surface prefers a non-sRGB format; an sRGB-only surface is rendered through its non-sRGB view.
- [DX12 uses a UNORM render target view.](../crates/lurq/src/app/dx12_render/mod.rs)
- The quad, glyph, image, NV12 image, and SVG shaders of both backends ([wgpu](../crates/lurq/src/app/wgpu_render/shaders), [DX12](../crates/lurq/src/app/dx12_render/shaders)) encode their linear result to sRGB before the blend.
- The clear colour is passed as sRGB channels.

Regression tests (GPU, `#[ignore]`d, run explicitly):

- [`wgpu_render/quad_tests.rs`](../crates/lurq/src/app/wgpu_render/quad_tests.rs): `translucent_quads_blend_like_css` and `opaque_quads_and_the_clear_keep_their_exact_colour`, which use the production quad shader and target format in an offscreen texture.
- [`app/blend_readback_tests.rs`](../crates/lurq/src/app/blend_readback_tests.rs): `wgpu_translucent_content_blends_like_css` and `dx12_translucent_content_blends_like_css`. Each draws a quad, a glyph, and an image through the real engine into a hidden window of its own and reads the frame back with the engine's frame capture.

Each test compares against a CPU reference that applies CSS source-over to the sRGB bytes.

## Summary

A modal scrim (`#000000A6` over the page) dimmed dark backgrounds as designed but left light text and borders much brighter than a design tool shows. `#EEEEEE` under the scrim rendered `#959595`; CSS, Figma, and Pencil show `#535353`. The reporter measured about `#ABABAB` in the app. That is the same effect on lighter or anti-aliased pixels.

Reported by Orchester. Present since the renderers were written. It affected every translucent pixel: `rgba` colours, `.opacity(...)`, anti-aliased edges, text coverage, images with alpha, and canvas composition.

## Root Cause

Both backends rendered into an sRGB render target: wgpu picked an `*UnormSrgb` surface format, and DX12 wrote its UNORM swap chain through an `R8G8B8A8_UNORM_SRGB` render target view. Shaders output linear colour and the hardware encoded it on write. The fixed-function blend on an sRGB target runs in linear light:

```
linear(#EEEEEE) = 0.855
0.855 * (1 - 0.651) = 0.298 linear  ->  sRGB 149 = #959595
```

CSS composites the encoded values:

```
238 * (1 - 0.651) = 83  ->  #535353
```

The devtools CPU screenshot renderer already blended in sRGB bytes, so its screenshots and the window disagreed on every translucent pixel.

## Reproduction

```rust
Stack::new().background("#eeeeee").child(Rect::new(200.0, 100.0).background("#000000a6"))
```

On a window (wgpu or DX12), the scrimmed pixels read `#959595`. The GPU tests above reproduce it offscreen: on `9c61a9e` they report `got [149, 149, 149], expected [83, 83, 83]` for the quad pipeline on both backends.

## Fix

The fix blends on encoded values the way CSS does, for every pipeline that draws into the window:

- The render target is not sRGB, so the fixed-function blend mixes sRGB-encoded channels. wgpu picks a non-sRGB surface format. If only sRGB formats exist, it configures the sRGB surface with its non-sRGB variant in `view_formats` and renders through that view. Pipelines use the view format. `SharedWgpuContext::surface_format` reports the surface's own format, which is now usually non-sRGB. DX12 creates its render target view and pipeline states with the swap chain's UNORM format.
- Each fragment or pixel shader still works in linear light, so texture decoding, gradient ramps, NV12 conversion, and canvas unpremultiplication are unchanged. It encodes its straight-alpha result with the exact sRGB transfer function (`encode_srgb`) on every return. Opaque colours therefore round-trip exactly.
- The clear colour is written as the colour's own sRGB channels.

`RenderList` is unchanged, so custom render engines are not affected. Canvas surfaces already blend internally in premultiplied sRGB; only their composition into the window changes.

## Visible Changes

- Translucent colours, `.opacity(...)`, scrims, shadows, and hover overlays match CSS and design tools. Dark backdrops change little. Light pixels under a dark translucent layer get much darker (`#EEEEEE` under `#000000A6`: `#959595` to `#535353`), and light translucent layers over dark backdrops get less bright.
- Anti-aliased edges of rects, rounded corners, borders, and SVGs blend in sRGB like a browser. Edges of dark shapes on light backgrounds look slightly heavier, and light shapes on dark backgrounds slightly lighter.
- Text: glyph coverage blends in sRGB. Dark text on light backgrounds renders heavier and light text on dark backgrounds thinner than before. At 50 % coverage, black on white goes from 188 to 128. This matches the devtools screenshot renderer and grayscale browser text more closely. The `sharpness` coverage curve is unchanged.
- Images and canvas: opaque pixels are unchanged. Translucent pixels and anti-aliased image edges blend like CSS.
- Gradients: opaque stops look the same, still interpolated in linear light (CSS interpolates in sRGB). Translucent gradients blend over the backdrop like CSS.
- The window's clear colour is unchanged.

## Verification

- `cargo test -p lurq --features wgpu --lib quad_tests -- --ignored`: 4 passed. `translucent_quads_blend_like_css` fails on `9c61a9e` (149 instead of 83).
- `cargo test -p lurq --features wgpu,dx12,raster,screenshot --lib blend_readback -- --ignored --test-threads=1`: 2 passed. Both fail on `9c61a9e` (149 instead of 83).
- `cargo test -p lurq --features wgpu,canvas --lib wgpu_render -- --ignored --test-threads=1` (canvas GPU vs software, camera cache, quad coverage), `wgpu_lifecycle_tests` and `wgpu_embedding_tests`: pass.
- `cargo test -p lurq --features dx12,raster,svg,screenshot --lib dx12` (includes `dx12_hlsl_shaders_compile`): pass.
- The `Validate crates` commands and `cargo check --workspace --all-features --all-targets` pass.

Not checked: macOS Metal and Linux Vulkan/GL surfaces (no device here). On those, the non-sRGB surface format is chosen when the surface offers one, which Metal and Vulkan normally do.
