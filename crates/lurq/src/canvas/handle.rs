use super::*;
impl CanvasHandle {
  pub(crate) fn new() -> Self {
    Self {
      inner: Arc::new(Mutex::new(Surface {
        id: CanvasId(NEXT_CANVAS_ID.fetch_add(1, Ordering::Relaxed)),
        metrics: CanvasMetrics {
          size: Size::new(0.0, 0.0),
          pixel_width: 0,
          pixel_height: 0,
          scale_factor: 1.0,
          revision: 0,
        },
        pixels: None,
        software: false,
        commands: Vec::new(),
        command_bytes: 0,
        layers: Vec::new(),
        layer_bytes: 0,
        layer_commands: 0,
        software_layers: Vec::new(),
        presentation_pixels: None,
        artwork_pixels: None,
        next_artwork_pixels: None,
        presentation_revision: 0,
        visible_revision: 0,
        presentation_serial: 0,
        presentation: None,
        inflight_bytes: 0,
        gpu_bytes: 0,
        gpu: CanvasGpuStats::default(),
        native: None,
        readbacks: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        state: DrawingState::default(),
        defaults: DrawingState::default(),
        stack: Vec::new(),
        path: Path2D::new(),
        revision: 0,
        exported_revision: 0,
        pending_paint: false,
        image: None,
        attached: false,
        window: None,
        to_window: Transform2D::IDENTITY,
        observers: Vec::new(),
        text: None,
        error: None,
        items: Arc::from([]),
        warned_duplicates: None,
      })),
    }
  }

  /// Schedules an ordinary paint pass, including a retry after GPU presentation backpressure.
  pub fn request_paint(&self) {
    let mut s = self.inner.lock();
    if !s.attached {
      return;
    }
    s.pending_paint = true;
    let window = s.window.clone();
    drop(s);
    if let Some(window) = window {
      window.wake();
    }
  }
  pub fn surface_id(&self) -> CanvasId {
    self.inner.lock().id
  }
  pub fn context_2d(&self) -> Context2D {
    Context2D { canvas: self.clone() }
  }
  pub fn size(&self) -> Size {
    self.inner.lock().metrics.size
  }
  pub fn pixel_size(&self) -> (u32, u32) {
    let s = self.inner.lock();
    (s.metrics.pixel_width, s.metrics.pixel_height)
  }
  pub fn scale_factor(&self) -> f32 {
    self.inner.lock().metrics.scale_factor
  }
  pub fn metrics(&self) -> CanvasMetrics {
    self.inner.lock().metrics
  }
  pub fn is_attached(&self) -> bool {
    self.inner.lock().attached
  }
  pub fn status(&self) -> CanvasStatus {
    let s = self.inner.lock();
    CanvasStatus {
      attached: s.attached,
      metrics: s.metrics,
      content_revision: s.revision,
      error: s.error.clone(),
      pending_bytes: s.command_bytes + s.layer_bytes + s.inflight_bytes,
      gpu_bytes: s.gpu_bytes,
      software: s.software,
      gpu: s.gpu,
    }
  }

  /// Receives the current metrics immediately, then committed size/scale changes.
  pub fn observe_metrics(&self, callback: impl Fn(CanvasMetrics) + Send + Sync + 'static) -> CanvasObserver {
    let callback: Arc<MetricsCallback> = Arc::new(callback);
    let metrics = {
      let mut s = self.inner.lock();
      s.observers.retain(|o| o.strong_count() > 0);
      s.observers.push(Arc::downgrade(&callback));
      s.metrics
    };
    callback(metrics);
    CanvasObserver { _callback: callback }
  }

  /// Converts window-logical input to content coordinates, without undoing the drawing transform.
  pub fn point_from_window(&self, x: f32, y: f32) -> Option<(f32, f32)> {
    let s = self.inner.lock();
    if !s.attached || !x.is_finite() || !y.is_finite() {
      return None;
    }
    s.to_window.inverse_affine().map(|m| m.transform_point(x, y))
  }

  /// Queue an ordered GPU readback. Never wait for it on the rendering thread.
  pub fn snapshot(&self) -> CanvasReadback {
    let (ticket, mut done) = CanvasReadback::pair();
    let wake = {
      let mut s = self.inner.lock();
      if s.software {
        done.finish(Ok(CanvasSnapshot {
          width: s.metrics.pixel_width,
          height: s.metrics.pixel_height,
          rgba: s.straight_pixels(),
          revision: if s.presentation_pixels.is_some() {
            s.presentation_revision
          } else {
            s.revision
          },
        }));
        return ticket;
      }
      if !s.attached {
        done.finish(Err(CanvasError::Detached));
        return ticket;
      }
      if !done.reserve(&s.readbacks) {
        done.finish(Err(CanvasError::QueueFull));
        return ticket;
      }
      let (metrics, revision) = (s.metrics, s.revision);
      if s.enqueue(gpu::Command::Readback(done, metrics, revision)) {
        s.pending_paint = true;
        s.window.clone()
      } else {
        None
      }
    };
    if let Some(window) = wake {
      window.wake();
    }
    ticket
  }

  pub(crate) fn update_placement(&self, matrix: Transform2D) {
    self.inner.lock().to_window = matrix;
  }

  pub(crate) fn clone_empty(&self) -> Self {
    let next = Self::new();
    next.inner.lock().software = self.inner.lock().software;
    next
  }
  pub(crate) fn downgrade(&self) -> CanvasWeak {
    CanvasWeak {
      inner: Arc::downgrade(&self.inner),
    }
  }
  pub(crate) fn dirty(&self) -> bool {
    let s = self.inner.lock();
    s.attached && s.pending_paint
  }
  pub(crate) fn consume_paint(&self) -> bool {
    let mut s = self.inner.lock();
    let pending = s.attached && s.pending_paint;
    s.pending_paint = false;
    pending
  }
  pub(crate) fn detach(&self) {
    let mut s = self.inner.lock();
    s.attached = false;
    s.window = None;
    s.commands.clear();
    s.command_bytes = 0;
    s.discard_layers();
    s.software_layers.clear();
    s.presentation_pixels = None;
    s.artwork_pixels = None;
    s.next_artwork_pixels = None;
    s.presentation = None;
    s.presentation_serial = s.presentation_serial.saturating_add(1);
  }

  pub(crate) fn bind_layout(
    &self,
    size: Size,
    scale: f32,
    to_window: Transform2D,
    window: Window,
    font: CanvasFont,
    text: Arc<Mutex<CanvasTextEngine>>,
  ) -> Option<(CanvasMetrics, Vec<Arc<MetricsCallback>>)> {
    let mut s = self.inner.lock();
    s.attached = true;
    s.window = Some(window);
    s.to_window = to_window;
    s.text = Some(text);
    let scale = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
    let size = Size::new(size.width.max(0.0), size.height.max(0.0));
    if s.metrics.revision != 0 && s.metrics.size == size && s.metrics.scale_factor == scale {
      return None;
    }
    let first = s.metrics.revision == 0;
    let resized = first || s.metrics.size != size;
    let old_scale = s.metrics.scale_factor;
    let dims = (
      f64::from(size.width) * f64::from(scale),
      f64::from(size.height) * f64::from(scale),
    );
    let valid = dims.0.is_finite()
      && dims.1.is_finite()
      && dims.0.ceil() <= 16384.0
      && dims.1.ceil() <= 16384.0
      && dims.0.ceil() * dims.1.ceil() <= MAX_PIXELS as f64;
    let (width, height) = if valid {
      (dims.0.ceil() as u32, dims.1.ceil() as u32)
    } else {
      (0, 0)
    };
    let mut next = if s.software && width > 0 && height > 0 {
      Pixmap::new(width, height)
    } else {
      None
    };
    s.error = if !valid || (s.software && width > 0 && height > 0 && next.is_none()) {
      Some(CanvasError::SurfaceTooLarge)
    } else {
      None
    };
    if !resized && s.error.is_some() {
      // A display-scale allocation failure must not discard the old bitmap.
      return None;
    }
    if !s.software
      && !resized
      && (s.commands.len() + s.layer_commands >= gpu::MAX_COMMANDS
        || s.command_bytes + s.layer_bytes + s.inflight_bytes + std::mem::size_of::<gpu::Command>()
          > gpu::MAX_QUEUE_BYTES)
    {
      s.error = Some(CanvasError::QueueFull);
      return None;
    }
    if resized {
      if first {
        s.defaults.font = font;
      }
      s.state = s.defaults.clone();
      s.stack.clear();
      s.path = Path2D::new();
      s.discard_layers();
      s.software_layers.clear();
      // The pixels the items described are gone; the redraw registers new ones.
      s.items = Arc::from([]);
    } else if s.software {
      if let (Some(old), Some(next)) = (s.pixels.as_ref(), next.as_mut()) {
        next.draw_pixmap(
          0,
          0,
          old.as_ref(),
          &PixmapPaint {
            quality: tiny_skia::FilterQuality::Bilinear,
            ..Default::default()
          },
          tiny_skia::Transform::from_scale(scale / old_scale, scale / old_scale),
          None,
        );
      }
      let factor = scale / old_scale;
      let mut masks = std::collections::HashMap::new();
      for state in s.stack.iter().chain(std::iter::once(&s.state)) {
        if let Some(mask) = &state.clip {
          masks.entry(Arc::as_ptr(mask)).or_insert_with(|| mask.clone());
        }
      }
      // Keep the previous backing scale if saved clips cannot fit at the new scale.
      // Shared clips remain shared, including after a display-scale transition.
      if u64::from(width) * u64::from(height) * masks.len() as u64 > MAX_CLIP_BYTES as u64 {
        s.error = Some(CanvasError::StateLimit);
        return None;
      }
      let masks: std::collections::HashMap<_, _> = masks
        .into_iter()
        .map(|(key, mask)| (key, rescale_clip(&mask, width, height, factor)))
        .collect();
      for state in &mut s.stack {
        if let Some(mask) = &state.clip {
          state.clip = masks[&Arc::as_ptr(mask)].clone();
        }
      }
      if let Some(mask) = &s.state.clip {
        s.state.clip = masks[&Arc::as_ptr(mask)].clone();
      }
    }
    if !s.software {
      if resized {
        s.commands.clear();
        s.command_bytes = 0;
      }
      s.enqueue(gpu::Command::Resize {
        width,
        height,
        preserve: !resized,
      });
      s.native = None;
      s.pending_paint = true;
    }
    s.presentation_pixels = None;
    s.next_artwork_pixels = None;
    s.presentation = None;
    s.presentation_serial = s.presentation_serial.saturating_add(1);
    s.pixels = next;
    s.image = None;
    s.revision += 1;
    s.metrics = CanvasMetrics {
      size,
      pixel_width: width,
      pixel_height: height,
      scale_factor: scale,
      revision: s.metrics.revision + 1,
    };
    let callbacks = s.observers.iter().filter_map(Weak::upgrade).collect();
    Some((s.metrics, callbacks))
  }

  pub(crate) fn image_data(&self) -> Option<ImageData> {
    let mut s = self.inner.lock();
    s.pending_paint = false;
    if !s.software {
      if s.metrics.pixel_width == 0 || s.metrics.pixel_height == 0 {
        return None;
      }
      if s.native.is_none() {
        s.native = Some(NativeImageData::new(
          s.metrics.pixel_width,
          s.metrics.pixel_height,
          ImagePixelFormat::Rgba8,
          NativeImageBackend::Canvas,
          self.downgrade(),
        ));
      }
      return s.native.as_ref().map(NativeImageData::image_data);
    }
    s.pixels.as_ref()?;
    if s.image.is_none() || s.revision != s.exported_revision {
      let data = s.straight_pixels();
      if let Some(image) = &s.image {
        image.set_rgba(data);
      } else {
        s.image = Some(StreamingImage::new_rgba_manual_redraw(
          data,
          s.metrics.pixel_width,
          s.metrics.pixel_height,
        ));
      }
      s.exported_revision = s.revision;
    }
    s.image.as_ref().map(StreamingImage::image_data)
  }
}

/// One open isolated layer on the software backend. The GPU backends keep their
/// layers as recorded commands instead; see `gpu::LayerFrame`.
struct SoftwareLayer {
  /// `None` only for a zero-sized surface, where the layer still has to exist so
  /// that `end_layer` stays balanced with `begin_layer`.
  pixels: Option<Pixmap>,
  alpha: f32,
  blend: BlendMode,
}

/// A paint with its geometry resolved against the box it was given and the
/// transform in force: what both backends draw from.
enum ResolvedPaint {
  Solid(Color),
  Gradient {
    gradient: Gradient,
    /// Model coordinates to the gradient's own frame, for GPU vertices.
    frame_from_model: Transform2D,
    /// The gradient's own frame to user coordinates, for a software shader.
    user_from_frame: Transform2D,
  },
}
