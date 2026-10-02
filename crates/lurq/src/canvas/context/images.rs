use super::*;

impl Context2D {
  pub fn draw_image(&self, image: &ImageData, x: f32, y: f32) -> Result<(), CanvasError> {
    self.draw_image_scaled(image, x, y, image.width() as f32, image.height() as f32)
  }
  pub fn draw_image_scaled(&self, image: &ImageData, x: f32, y: f32, w: f32, h: f32) -> Result<(), CanvasError> {
    self.draw_image_region(
      image,
      [0.0, 0.0, image.width() as f32, image.height() as f32],
      [x, y, w, h],
    )
  }
  /// Source `[x, y, width, height]` is in image pixels; destination is canvas-logical.
  pub fn draw_image_region(
    &self,
    image: &ImageData,
    source: [f32; 4],
    destination: [f32; 4],
  ) -> Result<(), CanvasError> {
    if !image.canvas_compatible() {
      return Err(CanvasError::UnsupportedImage);
    }
    if source.iter().chain(destination.iter()).any(|v| !v.is_finite()) {
      return Err(CanvasError::InvalidGeometry);
    }
    if image.width() > 16384
      || image.height() > 16384
      || u64::from(image.width()) * u64::from(image.height()) > MAX_PIXELS
    {
      return Err(CanvasError::SurfaceTooLarge);
    }
    let [mut sx, mut sy, mut sw, mut sh] = source;
    let [mut dx, mut dy, mut dw, mut dh] = destination;
    if sw < 0.0 {
      sx += sw;
      sw = -sw;
    }
    if sh < 0.0 {
      sy += sh;
      sh = -sh;
    }
    if dw < 0.0 {
      dx += dw;
      dw = -dw;
    }
    if dh < 0.0 {
      dy += dh;
      dh = -dh;
    }
    if sw == 0.0 || sh == 0.0 || dw == 0.0 || dh == 0.0 {
      return Ok(());
    }
    // Clip source bounds and shrink the destination by the same proportions.
    let (left, top, right, bottom) = (
      sx.max(0.0),
      sy.max(0.0),
      (sx + sw).min(image.width() as f32),
      (sy + sh).min(image.height() as f32),
    );
    if right <= left || bottom <= top {
      return Ok(());
    }
    let (fx, fy) = (dw / sw, dh / sh);
    let dest = [
      dx + (left - sx) * fx,
      dy + (top - sy) * fy,
      (right - left) * fx,
      (bottom - top) * fy,
    ];
    let mut result = Ok(());
    self.pixels(|s| {
      // An image is a paint like any other, so a blend mode isolates it first.
      let blend = s.state.blend;
      let isolated = !blend.is_normal() && s.begin_layer(1.0, blend).is_ok();
      let drew = (|s: &mut Surface| {
        let matrix = Transform2D::translate(dx - sx * fx, dy - sy * fy).then(&Transform2D::scale(fx, fy));
        if !s.software {
          let matrix = Transform2D::scale_uniform(s.metrics.scale_factor)
            .then(&s.state.transform)
            .then(&matrix);
          let accepted = s.enqueue(gpu::Command::Image {
            asset: gpu::Asset {
              id: image.id(),
              width: image.width(),
              height: image.height(),
              data: image.data_arc(),
              premultiplied: false,
            },
            matrix,
            source: [left, top, right - left, bottom - top],
            alpha: s.state.alpha,
            smooth: s.state.smoothing,
            clip: s.state.gpu_clip.clone(),
            scale: s.metrics.scale_factor,
          });
          if !accepted {
            result = Err(s.error.clone().unwrap_or(CanvasError::QueueFull));
          }
          return accepted;
        }
        let mut data = image.data_arc().as_ref().clone();
        for p in data.chunks_exact_mut(4) {
          let a = u16::from(p[3]);
          for c in &mut p[..3] {
            *c = ((u16::from(*c) * a + 127) / 255) as u8;
          }
        }
        let Some(size) = tiny_skia::IntSize::from_wh(image.width(), image.height()) else {
          result = Err(CanvasError::InvalidImage);
          return false;
        };
        let Some(pixels) = Pixmap::from_vec(data, size) else {
          result = Err(CanvasError::InvalidImage);
          return false;
        };
        blit(s, &pixels, matrix, Some(dest))
      })(s);
      if isolated { s.end_layer() | drew } else { drew }
    });
    result
  }
}
