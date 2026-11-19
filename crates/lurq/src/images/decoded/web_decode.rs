//! Pictures decoded by the browser: `createImageBitmap` decodes off the page's one thread, where the `image`
//! crate's decoder would hold the loop for as long as a large picture takes. The bitmap's pixels are read back
//! through a 2D canvas kept in memory (`willReadFrequently`), unpremultiplied and with the picture's own colors, as
//! the `image` crate gives them. What the browser does not decode (a TIFF, an ICO in some browsers) and SVG,
//! which lurq draws itself, are decoded as before.

use js_sys::{Array, Object, Reflect, Uint8Array};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

use super::ImageSizeCap;
use crate::images::ImageData;

/// The pixels of the picture in `bytes`, no larger than `cap`; `None` where the browser cannot decode it.
pub(super) async fn decode(bytes: &[u8], cap: Option<ImageSizeCap>) -> Option<ImageData> {
  let window = web_sys::window()?;
  let blob = web_sys::Blob::new_with_u8_array_sequence(&Array::of1(&Uint8Array::from(bytes))).ok()?;
  let options = web_sys::ImageBitmapOptions::new();
  options.set_premultiply_alpha(web_sys::PremultiplyAlpha::None);
  options.set_color_space_conversion(web_sys::ColorSpaceConversion::None);
  let bitmap: web_sys::ImageBitmap = JsFuture::from(
    window
      .create_image_bitmap_with_blob_and_image_bitmap_options(&blob, &options)
      .ok()?,
  )
  .await
  .ok()?
  .dyn_into()
  .ok()?;
  let (width, height) = fit(bitmap.width(), bitmap.height(), cap);
  let bitmap = if (width, height) == (bitmap.width(), bitmap.height()) {
    bitmap
  } else {
    // Made smaller by the browser, off the thread too.
    options.set_resize_width(width);
    options.set_resize_height(height);
    options.set_resize_quality(web_sys::ResizeQuality::Medium);
    let smaller = JsFuture::from(
      window
        .create_image_bitmap_with_image_bitmap_and_image_bitmap_options(&bitmap, &options)
        .ok()?,
    )
    .await
    .ok()?
    .dyn_into::<web_sys::ImageBitmap>()
    .ok()?;
    bitmap.close();
    smaller
  };
  let pixels = read(&bitmap, width, height);
  bitmap.close();
  pixels
}

/// The bitmap's pixels as RGBA.
fn read(bitmap: &web_sys::ImageBitmap, width: u32, height: u32) -> Option<ImageData> {
  if width == 0 || height == 0 {
    return None;
  }
  let canvas = web_sys::OffscreenCanvas::new(width, height).ok()?;
  let settings = Object::new();
  Reflect::set(&settings, &JsValue::from_str("willReadFrequently"), &JsValue::TRUE).ok()?;
  let context: web_sys::OffscreenCanvasRenderingContext2d = canvas
    .get_context_with_context_options("2d", &settings)
    .ok()??
    .dyn_into()
    .ok()?;
  context.draw_image_with_image_bitmap(bitmap, 0.0, 0.0).ok()?;
  let data = context
    .get_image_data(0.0, 0.0, f64::from(width), f64::from(height))
    .ok()?;
  Some(ImageData::from_rgba(data.data().0, width, height))
}

/// The size a picture of `width` by `height` is decoded at: as large as it is, or made smaller to fit `cap` with
/// its sides in proportion. Never larger.
fn fit(width: u32, height: u32, cap: Option<ImageSizeCap>) -> (u32, u32) {
  match cap {
    Some(cap) if width > cap.width || height > cap.height => {
      let scale =
        (f64::from(cap.width.max(1)) / f64::from(width)).min(f64::from(cap.height.max(1)) / f64::from(height));
      (
        ((f64::from(width) * scale).round() as u32).max(1),
        ((f64::from(height) * scale).round() as u32).max(1),
      )
    }
    _ => (width, height),
  }
}
