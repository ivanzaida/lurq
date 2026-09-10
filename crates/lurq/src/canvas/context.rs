use super::*;

impl Context2D {
  pub fn canvas(&self) -> CanvasHandle {
    self.canvas.clone()
  }

  fn pixels(&self, draw: impl FnOnce(&mut Surface) -> bool) {
    let wake = {
      let mut s = self.canvas.inner.lock();
      let was_clean = !s.pending_paint;
      if !draw(&mut s) {
        return;
      }
      s.revision += 1;
      s.pending_paint = true;
      if was_clean { s.window.clone() } else { None }
    };
    if let Some(window) = wake {
      window.wake();
    }
  }

  pub fn save(&self) {
    let mut s = self.canvas.inner.lock();
    if s.stack.len() == MAX_SAVE_DEPTH {
      s.error = Some(CanvasError::StateLimit);
      return;
    }
    let state = s.state.clone();
    s.stack.push(state);
  }
  pub fn restore(&self) {
    let mut s = self.canvas.inner.lock();
    if let Some(state) = s.stack.pop() {
      s.state = state;
    }
  }
  pub fn reset(&self) {
    self.pixels(|s| {
      s.state = s.defaults.clone();
      s.stack.clear();
      s.path = Path2D::new();
      s.error = None;
      if let Some(pixels) = &mut s.pixels {
        pixels.fill(tiny_skia::Color::TRANSPARENT);
      }
      true
    });
  }
  /// Erases the entire surface independently of transform and clip; keeps drawing state and path.
  pub fn clear(&self) {
    self.pixels(|s| {
      if let Some(pixels) = &mut s.pixels {
        pixels.fill(tiny_skia::Color::TRANSPARENT);
        true
      } else {
        false
      }
    });
  }

  pub fn set_fill_style(&self, color: impl CanvasColor) {
    if let Some(c) = color.canvas_color() {
      self.canvas.inner.lock().state.fill = c;
    }
  }
  pub fn fill_style(&self) -> Color {
    self.canvas.inner.lock().state.fill
  }
  pub fn set_stroke_style(&self, color: impl CanvasColor) {
    if let Some(c) = color.canvas_color() {
      self.canvas.inner.lock().state.stroke_color = c;
    }
  }
  pub fn stroke_style(&self) -> Color {
    self.canvas.inner.lock().state.stroke_color
  }
  pub fn set_global_alpha(&self, alpha: f32) {
    if (0.0..=1.0).contains(&alpha) {
      self.canvas.inner.lock().state.alpha = alpha;
    }
  }
  pub fn global_alpha(&self) -> f32 {
    self.canvas.inner.lock().state.alpha
  }
  pub fn set_line_width(&self, width: f32) {
    if width.is_finite() && width > 0.0 {
      self.canvas.inner.lock().state.stroke.width = width;
    }
  }
  pub fn line_width(&self) -> f32 {
    self.canvas.inner.lock().state.stroke.width
  }
  pub fn set_line_cap(&self, cap: LineCap) {
    self.canvas.inner.lock().state.stroke.line_cap = cap;
  }
  pub fn line_cap(&self) -> LineCap {
    self.canvas.inner.lock().state.stroke.line_cap
  }
  pub fn set_line_join(&self, join: LineJoin) {
    self.canvas.inner.lock().state.stroke.line_join = join;
  }
  pub fn line_join(&self) -> LineJoin {
    self.canvas.inner.lock().state.stroke.line_join
  }
  pub fn set_miter_limit(&self, limit: f32) {
    if limit.is_finite() && limit > 0.0 {
      self.canvas.inner.lock().state.stroke.miter_limit = limit;
    }
  }
  pub fn miter_limit(&self) -> f32 {
    self.canvas.inner.lock().state.stroke.miter_limit
  }
  pub fn set_line_dash(&self, dash: &[f32]) {
    if dash.len() > 4096 || dash.iter().any(|v| !v.is_finite() || *v < 0.0) {
      return;
    }
    let mut values = dash.to_vec();
    if values.len() % 2 == 1 {
      values.extend_from_slice(dash);
    }
    let mut s = self.canvas.inner.lock();
    s.state.stroke.dash = StrokeDash::new(values.clone(), s.state.dash_offset);
    s.state.dash = values;
  }
  pub fn line_dash(&self) -> Vec<f32> {
    self.canvas.inner.lock().state.dash.clone()
  }
  pub fn set_line_dash_offset(&self, offset: f32) {
    if !offset.is_finite() {
      return;
    }
    let mut s = self.canvas.inner.lock();
    s.state.dash_offset = offset;
    s.state.stroke.dash = StrokeDash::new(s.state.dash.clone(), offset);
  }
  pub fn line_dash_offset(&self) -> f32 {
    self.canvas.inner.lock().state.dash_offset
  }
  pub fn set_image_smoothing_enabled(&self, enabled: bool) {
    self.canvas.inner.lock().state.smoothing = enabled;
  }
  pub fn image_smoothing_enabled(&self) -> bool {
    self.canvas.inner.lock().state.smoothing
  }

  pub fn set_transform(&self, matrix: Transform2D) {
    if [matrix.a, matrix.b, matrix.c, matrix.d, matrix.tx, matrix.ty]
      .iter()
      .all(|v| v.is_finite())
    {
      self.canvas.inner.lock().state.transform = matrix;
    }
  }
  pub fn get_transform(&self) -> Transform2D {
    self.canvas.inner.lock().state.transform
  }
  pub fn reset_transform(&self) {
    self.set_transform(Transform2D::IDENTITY);
  }
  pub fn transform(&self, matrix: Transform2D) {
    let mut s = self.canvas.inner.lock();
    let matrix = s.state.transform.then(&matrix);
    if [matrix.a, matrix.b, matrix.c, matrix.d, matrix.tx, matrix.ty]
      .iter()
      .all(|v| v.is_finite())
    {
      s.state.transform = matrix;
    }
  }
  pub fn translate(&self, x: f32, y: f32) {
    self.transform(Transform2D::translate(x, y));
  }
  pub fn scale(&self, x: f32, y: f32) {
    self.transform(Transform2D::scale(x, y));
  }
  /// Rotation in radians.
  pub fn rotate(&self, radians: f32) {
    self.transform(Transform2D::rotate(radians));
  }

  pub fn begin_path(&self) {
    self.canvas.inner.lock().path = Path2D::new();
  }
  pub fn close_path(&self) {
    self.canvas.inner.lock().path.close_path();
  }
  pub fn move_to(&self, x: f32, y: f32) {
    let mut s = self.canvas.inner.lock();
    let p = s.state.transform.transform_point(x, y);
    s.path.move_to(p.0, p.1);
  }
  pub fn line_to(&self, x: f32, y: f32) {
    let mut s = self.canvas.inner.lock();
    let p = s.state.transform.transform_point(x, y);
    s.path.line_to(p.0, p.1);
  }
  pub fn quadratic_curve_to(&self, cx: f32, cy: f32, x: f32, y: f32) {
    let mut s = self.canvas.inner.lock();
    let m = s.state.transform;
    let (c, p) = (m.transform_point(cx, cy), m.transform_point(x, y));
    s.path.quadratic_curve_to(c.0, c.1, p.0, p.1);
  }
  pub fn bezier_curve_to(&self, ax: f32, ay: f32, bx: f32, by: f32, x: f32, y: f32) {
    let mut s = self.canvas.inner.lock();
    let m = s.state.transform;
    let (a, b, p) = (
      m.transform_point(ax, ay),
      m.transform_point(bx, by),
      m.transform_point(x, y),
    );
    s.path.bezier_curve_to(a.0, a.1, b.0, b.1, p.0, p.1);
  }
  fn path_geometry(
    &self,
    connect: bool,
    draw: impl FnOnce(&mut Path2D) -> Result<(), CanvasError>,
  ) -> Result<(), CanvasError> {
    let mut s = self.canvas.inner.lock();
    let mut path = Path2D::new();
    draw(&mut path)?;
    if let Some(path) = path.finish().and_then(|p| p.transform(transform(s.state.transform))) {
      s.path.append(&path, connect);
    }
    Ok(())
  }
  pub fn rect(&self, x: f32, y: f32, w: f32, h: f32) {
    let _ = self.path_geometry(false, |p| {
      p.rect(x, y, w, h);
      Ok(())
    });
  }
  pub fn round_rect(&self, x: f32, y: f32, w: f32, h: f32, r: f32) -> Result<(), CanvasError> {
    self.path_geometry(false, |p| p.round_rect(x, y, w, h, r))
  }
  pub fn arc(&self, x: f32, y: f32, r: f32, start: f32, end: f32, direction: ArcDirection) -> Result<(), CanvasError> {
    self.path_geometry(true, |p| p.arc(x, y, r, start, end, direction))
  }
  #[allow(clippy::too_many_arguments)]
  pub fn ellipse(
    &self,
    x: f32,
    y: f32,
    rx: f32,
    ry: f32,
    rotation: f32,
    start: f32,
    end: f32,
    direction: ArcDirection,
  ) -> Result<(), CanvasError> {
    self.path_geometry(true, |p| p.ellipse(x, y, rx, ry, rotation, start, end, direction))
  }
  pub fn arc_to(&self, x1: f32, y1: f32, x2: f32, y2: f32, r: f32) -> Result<(), CanvasError> {
    if r < 0.0 {
      return Err(CanvasError::InvalidGeometry);
    }
    let mut s = self.canvas.inner.lock();
    let mut path = Path2D::new();
    let Some(inverse) = s.state.transform.inverse_affine() else {
      return Ok(());
    };
    if let Some(current) = s.path.current() {
      let p = inverse.transform_point(current.x, current.y);
      path.move_to(p.0, p.1);
    }
    path.arc_to(x1, y1, x2, y2, r)?;
    if let Some(path) = path.finish().and_then(|p| p.transform(transform(s.state.transform))) {
      s.path.append(&path, true);
    }
    Ok(())
  }

  pub fn fill_rect(&self, x: f32, y: f32, w: f32, h: f32) {
    self.paint_rect(x, y, w, h, false, false);
  }
  pub fn stroke_rect(&self, x: f32, y: f32, w: f32, h: f32) {
    self.paint_rect(x, y, w, h, true, false);
  }
  pub fn clear_rect(&self, x: f32, y: f32, w: f32, h: f32) {
    self.paint_rect(x, y, w, h, false, true);
  }
  fn paint_rect(&self, x: f32, y: f32, w: f32, h: f32, stroke: bool, clear: bool) {
    let mut path = Path2D::new();
    path.rect(x, y, w, h);
    let Some(path) = path.finish() else {
      return;
    };
    self.pixels(|s| s.paint_path(&path, s.state.transform, stroke, clear, FillRule::NonZero));
  }
  pub fn fill(&self) {
    self.fill_with_rule(FillRule::NonZero);
  }
  pub fn fill_with_rule(&self, rule: FillRule) {
    self.pixels(|s| {
      let Some(path) = s.path.finish() else {
        return false;
      };
      s.paint_path(&path, Transform2D::IDENTITY, false, false, rule)
    });
  }
  pub fn stroke(&self) {
    self.pixels(|s| {
      let Some(path) = current_stroke_path(s) else {
        return false;
      };
      s.paint_path(&path, s.state.transform, true, false, FillRule::NonZero)
    });
  }
  pub fn fill_path(&self, path: &Path2D, rule: FillRule) {
    let Some(path) = path.finish() else {
      return;
    };
    self.pixels(|s| s.paint_path(&path, s.state.transform, false, false, rule));
  }
  pub fn stroke_path(&self, path: &Path2D) {
    let Some(path) = path.finish() else {
      return;
    };
    self.pixels(|s| s.paint_path(&path, s.state.transform, true, false, FillRule::NonZero));
  }

  pub fn clip(&self) {
    self.clip_with_rule(FillRule::NonZero);
  }
  pub fn clip_with_rule(&self, rule: FillRule) {
    let mut s = self.canvas.inner.lock();
    let path = s.path.finish();
    apply_clip(&mut s, path, Transform2D::IDENTITY, rule);
  }
  pub fn clip_path(&self, path: &Path2D, rule: FillRule) {
    let mut s = self.canvas.inner.lock();
    let m = s.state.transform;
    apply_clip(&mut s, path.finish(), m, rule);
  }
  /// Point coordinates are canvas-logical, unaffected by the current drawing transform.
  pub fn is_point_in_path(&self, x: f32, y: f32, rule: FillRule) -> bool {
    self
      .canvas
      .inner
      .lock()
      .path
      .finish()
      .is_some_and(|p| path::contains(&p, Point::from_xy(x, y), rule))
  }
  pub fn is_point_in_path2d(&self, path: &Path2D, x: f32, y: f32, rule: FillRule) -> bool {
    let s = self.canvas.inner.lock();
    path
      .finish()
      .and_then(|p| p.transform(transform(s.state.transform)))
      .is_some_and(|p| path::contains(&p, Point::from_xy(x, y), rule))
  }
  pub fn is_point_in_stroke(&self, x: f32, y: f32) -> bool {
    let s = self.canvas.inner.lock();
    current_stroke_path(&s)
      .and_then(|p| stroke_outline(&p, &s.state.stroke))
      .and_then(|p| p.transform(transform(s.state.transform)))
      .is_some_and(|p| path::contains(&p, Point::from_xy(x, y), FillRule::NonZero))
  }
  pub fn is_point_in_stroke_path(&self, path: &Path2D, x: f32, y: f32) -> bool {
    let s = self.canvas.inner.lock();
    path
      .finish()
      .and_then(|p| stroke_outline(&p, &s.state.stroke))
      .and_then(|p| p.transform(transform(s.state.transform)))
      .is_some_and(|p| path::contains(&p, Point::from_xy(x, y), FillRule::NonZero))
  }

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
    if u64::from(image.width()) * u64::from(image.height()) > MAX_PIXELS {
      return Err(CanvasError::SurfaceTooLarge);
    }
    let mut data = image.data_arc().as_ref().clone();
    for p in data.chunks_exact_mut(4) {
      let a = u16::from(p[3]);
      for c in &mut p[..3] {
        *c = ((u16::from(*c) * a + 127) / 255) as u8;
      }
    }
    let size = tiny_skia::IntSize::from_wh(image.width(), image.height()).ok_or(CanvasError::InvalidImage)?;
    let pixels = Pixmap::from_vec(data, size).ok_or(CanvasError::InvalidImage)?;
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
    self.pixels(|s| {
      let matrix = Transform2D::translate(dx - sx * fx, dy - sy * fy).then(&Transform2D::scale(fx, fy));
      blit(s, &pixels, matrix, Some(dest))
    });
    Ok(())
  }

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
    let s = self.canvas.inner.lock();
    let engine = s.text.as_ref().ok_or(CanvasError::TextUnavailable)?;
    let shaped = engine.lock().shape(text, &s.state.font, 1.0, s.state.fill)?;
    Ok(shaped.metrics(s.state.align, s.state.baseline))
  }
  pub fn fill_text(&self, text: &str, x: f32, y: f32) -> Result<(), CanvasError> {
    if !x.is_finite() || !y.is_finite() {
      return Ok(());
    }
    let mut error = None;
    self.pixels(|s| {
      let Some(engine) = &s.text else {
        error = Some(CanvasError::TextUnavailable);
        return false;
      };
      let shaped = match engine
        .lock()
        .shape(text, &s.state.font, s.metrics.scale_factor, s.state.fill)
      {
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
      blit(s, pixels, m, None)
    });
    if let Some(error) = error { Err(error) } else { Ok(()) }
  }
}

fn current_stroke_path(s: &Surface) -> Option<Path> {
  s.path
    .finish()?
    .transform(transform(s.state.transform.inverse_affine()?))
}
fn stroke_outline(path: &Path, stroke: &Stroke) -> Option<Path> {
  if let Some(dash) = &stroke.dash {
    path.dash(dash, 1.0)?.stroke(stroke, 1.0)
  } else {
    path.stroke(stroke, 1.0)
  }
}

fn apply_clip(s: &mut Surface, path: Option<Path>, matrix: Transform2D, rule: FillRule) {
  let (width, height) = (s.metrics.pixel_width, s.metrics.pixel_height);
  if width == 0 || height == 0 {
    return;
  }
  let mut retained = std::collections::HashSet::new();
  let mut bytes = width as usize * height as usize;
  for state in s.stack.iter().chain(std::iter::once(&s.state)) {
    if let Some(clip) = &state.clip
      && retained.insert(Arc::as_ptr(clip))
    {
      bytes += clip.data().len();
    }
  }
  if bytes > MAX_CLIP_BYTES {
    s.error = Some(CanvasError::StateLimit);
    return;
  }
  let Some(mut mask) = Mask::new(width, height) else {
    s.error = Some(CanvasError::SurfaceTooLarge);
    return;
  };
  if let Some(path) = path {
    mask.fill_path(
      &path,
      rule.skia(),
      true,
      transform(Transform2D::scale_uniform(s.metrics.scale_factor).then(&matrix)),
    );
  }
  if let Some(previous) = &s.state.clip {
    for (next, old) in mask.data_mut().iter_mut().zip(previous.data()) {
      *next = ((u16::from(*next) * u16::from(*old) + 127) / 255) as u8;
    }
  }
  s.state.clip = Some(Arc::new(mask));
}

fn blit(s: &mut Surface, pixels: &Pixmap, matrix: Transform2D, clip_rect: Option<[f32; 4]>) -> bool {
  let mut mask = None;
  if let Some([x, y, w, h]) = clip_rect {
    let mut p = Path2D::new();
    p.rect(x, y, w, h);
    if let (Some(p), Some(mut m)) = (p.finish(), Mask::new(s.metrics.pixel_width, s.metrics.pixel_height)) {
      m.fill_path(
        &p,
        tiny_skia::FillRule::Winding,
        true,
        transform(Transform2D::scale_uniform(s.metrics.scale_factor).then(&s.state.transform)),
      );
      if let Some(clip) = &s.state.clip {
        for (a, b) in m.data_mut().iter_mut().zip(clip.data()) {
          *a = ((u16::from(*a) * u16::from(*b) + 127) / 255) as u8;
        }
      }
      mask = Some(m);
    }
  }
  let transform = transform(
    Transform2D::scale_uniform(s.metrics.scale_factor)
      .then(&s.state.transform)
      .then(&matrix),
  );
  let Some(target) = &mut s.pixels else {
    return false;
  };
  target.draw_pixmap(
    0,
    0,
    pixels.as_ref(),
    &PixmapPaint {
      opacity: s.state.alpha,
      quality: if s.state.smoothing {
        tiny_skia::FilterQuality::Bilinear
      } else {
        tiny_skia::FilterQuality::Nearest
      },
      ..Default::default()
    },
    transform,
    mask.as_ref().or(s.state.clip.as_deref()),
  );
  true
}
