//! An explicit bounded presentation transaction; drawing state stays ordinary.
use super::*;
/// Dedicated render-target allowance, distinct from uploaded-asset residency.
/// Four 4K RGBA targets require 126.6MiB; the allowance includes tile workspace headroom.
pub const MAX_PRESENTATION_BYTES: u64 = 256 * 1024 * 1024;
const TILE_WORKSPACE_BYTES: u64 = 32 * 1024 * 1024;
fn presentation_admitted(pixels: u64) -> bool {
  pixels > 0
    && pixels <= MAX_PIXELS
    && pixels
      .checked_mul(16)
      .and_then(|n| n.checked_add(TILE_WORKSPACE_BYTES))
      .is_some_and(|n| n <= MAX_PRESENTATION_BYTES)
}
impl Context2D {
  /// Releases artwork ownership on document/page identity change or close.
  pub fn forget_artwork(&self) {
    let mut s = self.canvas.inner.lock();
    s.artwork_pixels = None;
    s.next_artwork_pixels = None;
    if !s.software {
      s.enqueue(gpu::Command::ForgetArtwork);
    }
  }

  /// Captures artwork before transient editor overlays. No readback occurs on GPU backends.
  pub fn capture_artwork(&self) -> Result<(), CanvasError> {
    let mut s = self.canvas.inner.lock();
    if s.presentation.is_none() || !s.layers.is_empty() || !s.software_layers.is_empty() {
      return Err(CanvasError::StateLimit);
    }
    if s.software {
      s.next_artwork_pixels = s.pixels.clone();
    } else if !s.enqueue(gpu::Command::CaptureArtwork) {
      return Err(s.error.clone().unwrap_or(CanvasError::QueueFull));
    }
    Ok(())
  }
  /// Reprojects the last complete artwork using a device-pixel camera map only.
  pub fn draw_retained_artwork(&self, matrix: Transform2D) -> Result<(), CanvasError> {
    let mut s = self.canvas.inner.lock();
    if s.presentation.is_none() {
      return Err(CanvasError::StateLimit);
    }
    if ![matrix.a, matrix.b, matrix.c, matrix.d, matrix.tx, matrix.ty]
      .iter()
      .all(|v| v.is_finite())
    {
      return Err(CanvasError::StateLimit);
    }
    if s.software {
      let Some(art) = s.artwork_pixels.clone() else {
        return Err(CanvasError::StateLimit);
      };
      if let Some(target) = &mut s.pixels {
        target.draw_pixmap(
          0,
          0,
          art.as_ref(),
          &tiny_skia::PixmapPaint::default(),
          software::transform(matrix),
          None,
        );
      }
    } else if !s.enqueue(gpu::Command::RetainedArtwork(matrix)) {
      return Err(s.error.clone().unwrap_or(CanvasError::QueueFull));
    }
    Ok(())
  }

  /// Records subsequent bounded GPU batches into a replacement surface.
  /// The previous complete surface stays presented until matching commit.
  /// A new begin supersedes an unfinished replacement. Software uses an isolated pixmap.
  pub fn begin_presentation(&self) -> Result<u64, CanvasError> {
    let mut s = self.canvas.inner.lock();
    if !s.attached {
      return Err(CanvasError::Detached);
    }
    if !s.layers.is_empty() || !s.software_layers.is_empty() {
      return Err(CanvasError::UnbalancedLayer);
    }
    let pixels = u64::from(s.metrics.pixel_width) * u64::from(s.metrics.pixel_height);
    if !presentation_admitted(pixels) {
      return Err(CanvasError::SurfaceTooLarge);
    }
    let token = s.presentation_serial.checked_add(1).ok_or(CanvasError::StateLimit)?;
    let front_revision = s.visible_revision;
    if s.software {
      let next = Pixmap::new(s.metrics.pixel_width, s.metrics.pixel_height).ok_or(CanvasError::SurfaceTooLarge)?;
      if s.presentation_pixels.is_none() {
        s.presentation_pixels = s.pixels.take();
        s.presentation_revision = s.revision;
      }
      s.pixels = Some(next);
      s.next_artwork_pixels = None;
    } else if !s.enqueue(gpu::Command::BeginPresentation(token, front_revision)) {
      return Err(s.error.clone().unwrap_or(CanvasError::QueueFull));
    }
    s.presentation_serial = token;
    s.presentation = Some(token);
    s.presentation_refused = false;
    Ok(token)
  }
  /// Commits only this current replacement; superseded tokens cannot publish.
  pub fn commit_presentation(&self, token: u64) -> Result<(), CanvasError> {
    self.finish_presentation(token, true)
  }
  /// Discards this replacement, leaving the previous complete surface visible.
  pub fn abort_presentation(&self, token: u64) -> Result<(), CanvasError> {
    self.finish_presentation(token, false)
  }
  fn finish_presentation(&self, token: u64, commit: bool) -> Result<(), CanvasError> {
    let mut s = self.canvas.inner.lock();
    if s.presentation != Some(token) {
      return Err(CanvasError::StateLimit);
    }
    let command = if commit {
      gpu::Command::CommitPresentation(token, s.revision + 1)
    } else {
      gpu::Command::AbortPresentation(token)
    };
    if commit && (!s.layers.is_empty() || !s.software_layers.is_empty()) {
      return Err(CanvasError::UnbalancedLayer);
    }
    if s.software {
      if !commit {
        s.software_layers.clear();
      }
      if !commit || s.error.is_some() {
        s.pixels = s.presentation_pixels.take();
      } else {
        s.gpu.presentation_commits += 1;
        s.gpu.presentation_token = token;
        s.visible_revision = s.revision + 1;
        s.presentation_pixels = None;
        if let Some(next) = s.next_artwork_pixels.take() {
          s.artwork_pixels = Some(next);
        }
      }
    } else {
      if !commit {
        s.discard_layers();
      }
      if !s.enqueue(command) {
        return Err(s.error.clone().unwrap_or(CanvasError::QueueFull));
      }
    }
    s.next_artwork_pixels = None;
    s.presentation = None;
    s.revision += 1;
    s.pending_paint = true;
    if let Some(native) = &s.native {
      native.bump_version();
    }
    let window = s.window.clone();
    drop(s);
    if let Some(window) = window {
      window.wake();
    }
    Ok(())
  }
}

pub(crate) enum PresentationEvent {
  Commit(u64, u64),
  Quad,
  Allocate,
  Reuse,
  Refuse,
}
impl CanvasHandle {
  pub(crate) fn retain_presentation_revision(&self, revision: u64) {
    let mut s = self.inner.lock();
    s.visible_revision = revision;
    s.presentation_refused = true;
  }
  pub(crate) fn presentation_workspace(&self, bytes: usize) {
    self.inner.lock().gpu.presentation_workspace_bytes = bytes;
  }
  pub(crate) fn presentation_event(&self, event: PresentationEvent) {
    let mut s = self.inner.lock();
    match event {
      PresentationEvent::Commit(token, revision) => {
        s.gpu.presentation_token = token;
        s.visible_revision = revision;
        s.gpu.presentation_commits += 1;
      }
      PresentationEvent::Quad => s.gpu.fallback_quads += 1,
      PresentationEvent::Allocate => s.gpu.presentation_allocations += 1,
      PresentationEvent::Reuse => s.gpu.presentation_reuses += 1,
      PresentationEvent::Refuse => s.gpu.presentation_refusals += 1,
    }
  }

  pub(crate) fn presentation_current(&self, token: u64) -> bool {
    self.inner.lock().presentation_serial == token
  }
}

#[cfg(test)]
#[path = "presentation_tests.rs"]
mod tests;
