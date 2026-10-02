use super::*;

impl Context2D {
  pub fn set_font(&self, font: CanvasFont) {
    if font.size.is_finite() && font.size > 0.0 && font.size <= 4096.0 {
      self.canvas.inner.lock().state.font = font;
    }
  }
  pub fn font(&self) -> CanvasFont {
    self.canvas.inner.lock().state.font.clone()
  }
  pub fn set_text_align(&self, align: TextAlign) {
    self.canvas.inner.lock().state.align = align;
  }
  pub fn text_align(&self) -> TextAlign {
    self.canvas.inner.lock().state.align
  }
  pub fn set_text_baseline(&self, baseline: TextBaseline) {
    self.canvas.inner.lock().state.baseline = baseline;
  }
  pub fn text_baseline(&self) -> TextBaseline {
    self.canvas.inner.lock().state.baseline
  }
  pub fn measure_text(&self, text: &str) -> Result<TextMetrics, CanvasError> {
    #[cfg(feature = "perf_profile")]
    crate::app::profiler::canvas_text::call(crate::app::profiler::canvas_text::Call::Measure);
    let s = self.canvas.inner.lock();
    let engine = s.text.as_ref().ok_or(CanvasError::TextUnavailable)?;
    let color = s.state.fill.color().ok_or(CanvasError::UnsupportedPaint)?;
    let shaped = engine.lock().measure(text, &s.state.font, color)?;
    Ok(shaped.metrics(s.state.align, s.state.baseline))
  }
  pub fn fill_text(&self, text: &str, x: f32, y: f32) -> Result<(), CanvasError> {
    #[cfg(feature = "perf_profile")]
    crate::app::profiler::canvas_text::call(crate::app::profiler::canvas_text::Call::Fill);
    if !x.is_finite() || !y.is_finite() {
      return Ok(());
    }
    let mut error = None;
    self.pixels(|s| {
      let Some(engine) = &s.text else {
        error = Some(CanvasError::TextUnavailable);
        return false;
      };
      let Some(color) = s.state.fill.color() else {
        error = Some(CanvasError::UnsupportedPaint);
        return false;
      };
      let shaped = match engine.lock().shape(text, &s.state.font, s.metrics.scale_factor, color) {
        Ok(shaped) => shaped,
        Err(e) => {
          error = Some(e);
          return false;
        }
      };
      let Some(pixels) = &shaped.pixels else {
        return false;
      };
      let (ox, oy) = shaped.origin(s.state.align, s.state.baseline);
      let m = Transform2D::translate(x + ox, y + oy).then(&Transform2D::scale_uniform(1.0 / s.metrics.scale_factor));
      let pixels = pixels.clone();
      let data = shaped.data.clone();
      s.paint_pixmap(&pixels, &data, shaped.asset_id, m)
    });
    if let Some(error) = error { Err(error) } else { Ok(()) }
  }
}
