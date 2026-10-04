use super::*;
impl Surface {
  pub(super) fn straight_pixels(&self) -> Vec<u8> {
    let Some(pixels) = self.presentation_pixels.as_ref().or(self.pixels.as_ref()) else {
      return Vec::new();
    };
    let mut rgba = Vec::with_capacity(pixels.data().len());
    for pixel in pixels.pixels() {
      let c = pixel.demultiply();
      rgba.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    rgba
  }

  /// Where a software draw lands: the innermost open layer, or the surface.
  pub(super) fn target_pixels(&mut self) -> Option<&mut Pixmap> {
    match self.software_layers.last_mut() {
      Some(layer) => layer.pixels.as_mut(),
      None => self.pixels.as_mut(),
    }
  }

  pub(super) fn surface_size(&self) -> (u32, u32) {
    (self.metrics.pixel_width, self.metrics.pixel_height)
  }

  /// One user unit as device pixels, including rotation and skew. Shadow offsets,
  /// blur radii and spreads are written in user units and scaled by this.
  pub(super) fn user_to_device(&self) -> Transform2D {
    Transform2D::scale_uniform(self.metrics.scale_factor).then(&self.state.transform.linear_part())
  }

  pub(super) fn begin_layer(&mut self, alpha: f32, blend: BlendMode) -> Result<(), CanvasError> {
    if !self.software {
      return self.open_layer(alpha, blend);
    }
    if !self.attached {
      self.error = Some(CanvasError::Detached);
      return Err(CanvasError::Detached);
    }
    if self.software_layers.len() >= MAX_LAYER_DEPTH {
      self.error = Some(CanvasError::StateLimit);
      return Err(CanvasError::StateLimit);
    }
    let (width, height) = self.surface_size();
    self.software_layers.push(SoftwareLayer {
      pixels: Pixmap::new(width, height),
      alpha,
      blend,
    });
    Ok(())
  }

  /// Composites the innermost layer onto its parent. `false` when there was
  /// nothing to composite, which is what tells the caller no pixels changed.
  pub(super) fn end_layer(&mut self) -> bool {
    if !self.software {
      return self.close_layer();
    }
    let Some(layer) = self.software_layers.pop() else {
      return false;
    };
    let (alpha, blend) = (layer.alpha, layer.blend);
    let Some(source) = layer.pixels else {
      return false;
    };
    let Some(target) = self.target_pixels() else {
      return false;
    };
    blend::composite_premultiplied(target.data_mut(), source.data(), alpha, blend);
    true
  }

  /// One already rasterised premultiplied source — shaped text — with the blend
  /// mode, shadow and filter in force. `matrix` maps its own pixels into user
  /// space. Spread has no meaning for a raster and is not applied.
  pub(super) fn paint_pixmap(&mut self, pixels: &Pixmap, data: &Arc<Vec<u8>>, id: u64, matrix: Transform2D) -> bool {
    let blend = self.state.blend;
    let isolated = !blend.is_normal() && self.begin_layer(1.0, blend).is_ok();
    let device = Transform2D::scale_uniform(self.metrics.scale_factor)
      .then(&self.state.transform)
      .then(&matrix);
    let user = self.user_to_device();
    let shadow = self.state.shadow.filter(Shadow::is_valid);
    let mut drew = false;
    if let Some(shadow) = shadow.filter(|s| !s.inset) {
      drew |= self.pixmap_shadow(pixels, id, device, user, &shadow);
    }
    let radius = self.state.filter.radius();
    drew |= if self.state.filter.is_valid() && radius > 0.0 {
      match effect::pixmap_blur(pixels, id, device, user, self.surface_size(), radius) {
        Ok(Some(image)) => self.draw_effect(&image),
        Ok(None) => false,
        Err(error) => {
          self.error = Some(error);
          false
        }
      }
    } else {
      self.draw_source(pixels, data, id, matrix)
    };
    if let Some(shadow) = shadow.filter(|s| s.inset) {
      drew |= self.pixmap_shadow(pixels, id, device, user, &shadow);
    }
    if isolated {
      drew |= self.end_layer();
    }
    drew
  }

  pub(super) fn pixmap_shadow(
    &mut self,
    pixels: &Pixmap,
    id: u64,
    device: Transform2D,
    user: Transform2D,
    shadow: &Shadow,
  ) -> bool {
    match effect::pixmap_shadow(pixels, id, device, user, self.surface_size(), shadow) {
      Ok(Some(image)) => self.draw_effect(&image),
      Ok(None) => false,
      Err(error) => {
        self.error = Some(error);
        false
      }
    }
  }

  pub(super) fn draw_source(&mut self, pixels: &Pixmap, data: &Arc<Vec<u8>>, id: u64, matrix: Transform2D) -> bool {
    if !self.software {
      let device = Transform2D::scale_uniform(self.metrics.scale_factor)
        .then(&self.state.transform)
        .then(&matrix);
      return self.enqueue(gpu::Command::Image {
        asset: gpu::Asset {
          id,
          width: pixels.width(),
          height: pixels.height(),
          data: data.clone(),
          premultiplied: true,
        },
        matrix: device,
        source: [0., 0., pixels.width() as f32, pixels.height() as f32],
        alpha: self.state.alpha,
        smooth: self.state.smoothing,
        clip: self.state.gpu_clip.clone(),
        scale: self.metrics.scale_factor,
      });
    }
    context::blit(self, pixels, matrix, None)
  }

  /// A fill or a stroke with the paint, blend mode, shadow and filter in force.
  /// The order is the one a design tool draws in: the drop shadow, the shape,
  /// then the inner shadow, all inside one isolation when a blend mode is set.
  pub(super) fn paint_path(
    &mut self,
    path: &path::Geometry,
    matrix: Transform2D,
    stroke: bool,
    clear: bool,
    rule: FillRule,
  ) -> bool {
    if clear {
      return self.paint_direct(path, matrix, stroke, true, rule);
    }
    let blend = self.state.blend;
    let isolated = !blend.is_normal() && self.begin_layer(1.0, blend).is_ok();
    let shadow = self.state.shadow.filter(Shadow::is_valid);
    let mut drew = false;
    if let Some(shadow) = shadow.filter(|s| !s.inset) {
      drew |= self.paint_shadow(path, matrix, stroke, rule, &shadow);
    }
    drew |= if self.state.filter.is_valid() && self.state.filter.radius() > 0.0 {
      self.paint_filtered(path, matrix, stroke, rule)
    } else {
      self.paint_direct(path, matrix, stroke, false, rule)
    };
    if let Some(shadow) = shadow.filter(|s| s.inset) {
      drew |= self.paint_shadow(path, matrix, stroke, rule, &shadow);
    }
    if isolated {
      drew |= self.end_layer();
    }
    drew
  }

  /// The geometry an effect is cast from: a stroke's own outline, or the fill.
  pub(super) fn effect_geometry(
    &self,
    path: &path::Geometry,
    stroke: bool,
    rule: FillRule,
  ) -> Option<(path::Geometry, FillRule)> {
    if !stroke {
      return Some((path.clone(), rule));
    }
    context::stroke_outline(path, &self.state.stroke).map(|outline| (path::Geometry::new(outline), FillRule::NonZero))
  }

  pub(super) fn paint_shadow(
    &mut self,
    path: &path::Geometry,
    matrix: Transform2D,
    stroke: bool,
    rule: FillRule,
    shadow: &Shadow,
  ) -> bool {
    let Some((geometry, rule)) = self.effect_geometry(path, stroke, rule) else {
      return false;
    };
    let device = Transform2D::scale_uniform(self.metrics.scale_factor).then(&matrix);
    let user = self.user_to_device();
    match effect::shadow_image(&geometry, rule, device, user, self.surface_size(), shadow) {
      Ok(Some(image)) => self.draw_effect(&image),
      Ok(None) => false,
      Err(error) => {
        self.error = Some(error);
        false
      }
    }
  }

  /// A layer blur: the shape is rasterised with its own paint into a reduced
  /// raster, blurred there, and drawn as one image.
  pub(super) fn paint_filtered(
    &mut self,
    path: &path::Geometry,
    matrix: Transform2D,
    stroke: bool,
    rule: FillRule,
  ) -> bool {
    let Some((geometry, rule)) = self.effect_geometry(path, stroke, rule) else {
      return false;
    };
    let radius = self.state.filter.radius();
    let device = Transform2D::scale_uniform(self.metrics.scale_factor).then(&matrix);
    let user = self.user_to_device();
    let Some(paint) = self.resolved_paint(stroke, matrix) else {
      return false;
    };
    let planned = match effect::blur_plan(&geometry, device, user, self.surface_size(), radius) {
      Ok(Some(planned)) => planned,
      Ok(None) => return false,
      Err(error) => {
        self.error = Some(error);
        return false;
      }
    };
    let key = effect::blur_identity(&[
      geometry.content_hash(),
      u64::from(rule == FillRule::EvenOdd),
      u64::from(device.a.to_bits()) ^ (u64::from(device.b.to_bits()) << 32),
      u64::from(device.c.to_bits()) ^ (u64::from(device.d.to_bits()) << 32),
      ((planned.image.origin.0 as i64) as u64) ^ (((planned.image.origin.1 as i64) as u64) << 32),
      (u64::from(planned.image.width) << 32) | u64::from(planned.image.height),
      u64::from(planned.radius) ^ (u64::from(planned.image.reduction) << 32),
      paint.identity(),
    ]);
    let image = match effect::lookup(key) {
      Some(image) => image,
      None => {
        let effect::BlurPlan {
          mut pixmap,
          image,
          radius,
          raster,
        } = planned;
        let raster_from_device = Transform2D::scale_uniform(1.0 / image.reduction as f32)
          .then(&Transform2D::translate(-image.origin.0 as f32, -image.origin.1 as f32));
        let frame_to_raster = raster_from_device.then(&self.device_from_frame(&paint));
        let conic = self.angular_pattern(&paint, frame_to_raster, (0, 0), image.width, image.height);
        let skia = paint_of(&paint, frame_to_raster, conic.as_ref().map(|p| (p, (0, 0))), 1.0);
        pixmap.fill_path(&geometry, &skia, rule.skia(), transform(raster), None);
        drop(skia);
        drop(conic);
        let image = effect::blur_finish(pixmap, image, radius, key);
        effect::store(key, &image);
        image
      }
    };
    self.draw_effect(&image)
  }

  /// Places a rasterised effect in device pixels, under the current clip and
  /// global alpha but under no drawing transform: it is already rasterised.
  pub(super) fn draw_effect(&mut self, image: &effect::EffectImage) -> bool {
    if image.width == 0 || image.height == 0 {
      return false;
    }
    let alpha = self.state.alpha;
    let smooth = image.reduction > 1;
    if !self.software {
      return self.enqueue(gpu::Command::Image {
        asset: gpu::Asset {
          id: image.id,
          width: image.width,
          height: image.height,
          data: image.data.clone(),
          premultiplied: true,
        },
        matrix: image.matrix(),
        source: [0., 0., image.width as f32, image.height as f32],
        alpha,
        smooth,
        clip: self.state.gpu_clip.clone(),
        scale: self.metrics.scale_factor,
      });
    }
    let Some(size) = tiny_skia::IntSize::from_wh(image.width, image.height) else {
      return false;
    };
    let Some(source) = Pixmap::from_vec(image.data.as_ref().clone(), size) else {
      return false;
    };
    let matrix = transform(image.matrix());
    let clip = self.state.clip.clone();
    let Some(target) = self.target_pixels() else {
      return false;
    };
    target.draw_pixmap(
      0,
      0,
      source.as_ref(),
      &PixmapPaint {
        opacity: alpha,
        quality: if smooth {
          tiny_skia::FilterQuality::Bilinear
        } else {
          tiny_skia::FilterQuality::Nearest
        },
        ..Default::default()
      },
      matrix,
      clip.as_deref(),
    );
    true
  }

  /// The paint a fill or stroke uses. `None` when a gradient cannot be placed,
  /// in which case nothing is drawn rather than something else in its place.
  pub(super) fn resolved_paint(&self, stroke: bool, matrix: Transform2D) -> Option<ResolvedPaint> {
    let paint = if stroke {
      &self.state.stroke_paint
    } else {
      &self.state.fill
    };
    let Some((gradient, bounds)) = paint.gradient() else {
      return Some(ResolvedPaint::Solid(paint.color().unwrap_or(Color::new(0, 0, 0, 255))));
    };
    if !gradient.is_valid() {
      return None;
    }
    let frame_from_user = gradient.frame_from_box(bounds)?;
    let user_from_model = self.state.transform.inverse_affine()?.then(&matrix);
    Some(ResolvedPaint::Gradient {
      gradient: gradient.clone(),
      frame_from_model: frame_from_user.then(&user_from_model),
      user_from_frame: frame_from_user.inverse_affine()?,
    })
  }

  /// Maps the gradient's own frame onto device pixels.
  pub(super) fn device_from_frame(&self, paint: &ResolvedPaint) -> Transform2D {
    match paint {
      ResolvedPaint::Solid(_) => Transform2D::IDENTITY,
      ResolvedPaint::Gradient { user_from_frame, .. } => Transform2D::scale_uniform(self.metrics.scale_factor)
        .then(&self.state.transform)
        .then(user_from_frame),
    }
  }

  /// An angular gradient has no tiny-skia shader, so the software backend
  /// evaluates one into a pattern bounded by the raster it paints into.
  pub(super) fn angular_pattern(
    &self,
    paint: &ResolvedPaint,
    frame_to_target: Transform2D,
    origin: (i32, i32),
    width: u32,
    height: u32,
  ) -> Option<Pixmap> {
    let ResolvedPaint::Gradient { gradient, .. } = paint else {
      return None;
    };
    if gradient.kind() != GradientKind::Angular || width == 0 || height == 0 {
      return None;
    }
    paint::angular_pixmap(gradient, frame_to_target.inverse_affine()?, origin, width, height)
  }

  pub(super) fn paint_direct(
    &mut self,
    path: &path::Geometry,
    matrix: Transform2D,
    stroke: bool,
    clear: bool,
    rule: FillRule,
  ) -> bool {
    let Some(paint) = self.resolved_paint(stroke, matrix) else {
      return false;
    };
    let alpha = self.state.alpha;
    let device = Transform2D::scale_uniform(self.metrics.scale_factor).then(&matrix);
    if !self.software {
      let path = if stroke {
        context::stroke_outline(path, &self.state.stroke).map(path::Geometry::new)
      } else {
        Some(path.clone())
      };
      let Some(path) = path else {
        return false;
      };
      let (color, gradient) = match &paint {
        ResolvedPaint::Solid(color) => {
          let a = f32::from(color.a()) / 255.0 * alpha;
          (
            [
              f32::from(color.r()) / 255.0 * a,
              f32::from(color.g()) / 255.0 * a,
              f32::from(color.b()) / 255.0 * a,
              a,
            ],
            None,
          )
        }
        ResolvedPaint::Gradient {
          gradient,
          frame_from_model,
          ..
        } => {
          let ramp = gradient.ramp();
          (
            [alpha; 4],
            Some(gpu::GradientPaint {
              kind: gradient.kind(),
              ramp: gpu::Asset {
                id: ramp.id,
                width: RAMP_TEXELS as u32,
                height: 1,
                data: ramp.texels.clone(),
                premultiplied: true,
              },
              frame: *frame_from_model,
            }),
          )
        }
      };
      return self.enqueue(gpu::Command::Path {
        path,
        matrix: device,
        rule,
        color,
        gradient,
        erase: clear,
        clip: self.state.gpu_clip.clone(),
        scale: self.metrics.scale_factor,
      });
    }
    let frame_to_device = self.device_from_frame(&paint);
    let (origin, width, height) = device_window(path, device, self.surface_size());
    let conic = self.angular_pattern(&paint, frame_to_device, origin, width, height);
    let mut skia = paint_of(
      &paint,
      frame_to_device,
      conic.as_ref().map(|pixmap| (pixmap, origin)),
      alpha,
    );
    if clear {
      skia.blend_mode = tiny_skia::BlendMode::Clear;
    }
    let stroke_params = self.state.stroke.clone();
    let clip = self.state.clip.clone();
    let matrix = transform(device);
    let Some(pixels) = self.target_pixels() else {
      return false;
    };
    if stroke {
      pixels.stroke_path(path, &skia, &stroke_params, matrix, clip.as_deref());
    } else {
      pixels.fill_path(path, &skia, rule.skia(), matrix, clip.as_deref());
    }
    true
  }
}

/// The tiny-skia paint for a resolved paint, with the global alpha applied once.
pub(super) fn paint_of<'a>(
  paint: &ResolvedPaint,
  frame_to_target: Transform2D,
  conic: Option<(&'a Pixmap, (i32, i32))>,
  alpha: f32,
) -> SkiaPaint<'a> {
  let mut skia = SkiaPaint {
    anti_alias: true,
    ..SkiaPaint::default()
  };
  match paint {
    ResolvedPaint::Solid(color) => skia.set_color_rgba8(
      color.r(),
      color.g(),
      color.b(),
      (f32::from(color.a()) * alpha).round() as u8,
    ),
    ResolvedPaint::Gradient { gradient, .. } => {
      if let Some(mut shader) = paint::skia_shader(gradient, frame_to_target, conic) {
        shader.apply_opacity(alpha);
        skia.shader = shader;
      }
    }
  }
  skia
}

/// The device-pixel window a path can touch, clipped to the surface.
fn device_window(path: &path::Geometry, matrix: Transform2D, surface: (u32, u32)) -> ((i32, i32), u32, u32) {
  let b = path.bounds();
  let mut box_ = [f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY];
  for (x, y) in [
    (b.left(), b.top()),
    (b.right(), b.top()),
    (b.right(), b.bottom()),
    (b.left(), b.bottom()),
  ] {
    let (x, y) = matrix.transform_point(x, y);
    if !x.is_finite() || !y.is_finite() {
      return ((0, 0), 0, 0);
    }
    box_[0] = box_[0].min(x);
    box_[1] = box_[1].min(y);
    box_[2] = box_[2].max(x);
    box_[3] = box_[3].max(y);
  }
  let left = box_[0].floor().max(0.0).min(surface.0 as f32);
  let top = box_[1].floor().max(0.0).min(surface.1 as f32);
  let right = (box_[2].ceil() + 1.0).max(0.0).min(surface.0 as f32);
  let bottom = (box_[3].ceil() + 1.0).max(0.0).min(surface.1 as f32);
  (
    (left as i32, top as i32),
    (right - left).max(0.0) as u32,
    (bottom - top).max(0.0) as u32,
  )
}

pub(super) fn rescale_clip(mask: &Mask, width: u32, height: u32, factor: f32) -> Option<Arc<Mask>> {
  let mut pixels = Pixmap::new(mask.width(), mask.height())?;
  for (pixel, alpha) in pixels.data_mut().chunks_exact_mut(4).zip(mask.data()) {
    pixel.fill(*alpha);
  }
  let mut next = Pixmap::new(width, height)?;
  next.draw_pixmap(
    0,
    0,
    pixels.as_ref(),
    &PixmapPaint {
      quality: tiny_skia::FilterQuality::Bilinear,
      ..Default::default()
    },
    tiny_skia::Transform::from_scale(factor, factor),
    None,
  );
  Some(Arc::new(Mask::from_pixmap(next.as_ref(), tiny_skia::MaskType::Alpha)))
}

pub(crate) fn transform(m: Transform2D) -> tiny_skia::Transform {
  tiny_skia::Transform::from_row(m.a, m.b, m.c, m.d, m.tx, m.ty)
}

/// Accepted solid paints. String parsing is checked and never panics.
pub trait CanvasColor {
  fn canvas_color(&self) -> Option<Color>;
}
