//! Backend-neutral, bounded canvas work. No canvas-sized CPU bitmap lives here.
#![cfg_attr(
  not(any(feature = "wgpu", all(feature = "dx12", target_os = "windows"))),
  allow(dead_code)
)]
use std::{collections::HashMap, ops::Range, time::Duration};

use lyon::{math::point, path::Path as LyonPath, tessellation::*};

use super::{BlendMode, FillRule, GradientKind, path::Geometry, *};

mod artwork;
pub(crate) use artwork::artwork_vertices;
mod tessellation;
pub(crate) use tessellation::{Prepared, layer_depth, touches};
mod cache;
pub(crate) use cache::MeshCache;

#[cfg(test)]
mod benchmark;

pub(crate) const TILE: u32 = 512;
pub(crate) const MAX_QUEUE_BYTES: usize = 64 * 1024 * 1024;
const MAX_VERTICES: usize = 1_048_576;
/// Commands one surface may hold, an open layer's own commands included.
pub(crate) const MAX_COMMANDS: usize = 8192;
/// Isolated layers that may be open at once. Each costs one tile-sized save
/// and one tile-sized layer target in the renderer, not a surface-sized one.
pub const MAX_LAYER_DEPTH: usize = 8;
static READBACKS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
struct ReadbackPermit(Arc<std::sync::atomic::AtomicUsize>);
impl Drop for ReadbackPermit {
  fn drop(&mut self) {
    self.0.fetch_sub(1, Ordering::AcqRel);
    READBACKS.fetch_sub(1, Ordering::AcqRel);
  }
}

#[derive(Clone)]
pub(crate) struct Clip {
  pub path: Option<Geometry>,
  pub matrix: Transform2D,
  pub rule: FillRule,
  pub previous: Option<Arc<Clip>>,
  pub depth: u32,
  pub bytes: usize,
}

/// A gradient paint: the ramp to sample, and the map from the path's own model
/// coordinates onto the gradient's frame. The ramp is an ordinary asset, so it
/// is uploaded once and shared by every draw that names it.
#[derive(Clone)]
pub(crate) struct GradientPaint {
  pub kind: GradientKind,
  pub ramp: Asset,
  pub frame: Transform2D,
}

#[derive(Clone)]
pub(crate) struct Asset {
  pub id: u64,
  pub width: u32,
  pub height: u32,
  pub data: Arc<Vec<u8>>,
  pub premultiplied: bool,
}

pub(crate) enum Command {
  Resize {
    width: u32,
    height: u32,
    preserve: bool,
  },
  Clear,
  BeginPresentation(u64, u64),
  CommitPresentation(u64, u64),
  AbortPresentation(u64),
  CaptureArtwork,
  ForgetArtwork,
  RetainedArtwork(Transform2D),
  Path {
    path: Geometry,
    matrix: Transform2D,
    rule: FillRule,
    color: [f32; 4],
    gradient: Option<GradientPaint>,
    erase: bool,
    clip: Option<Arc<Clip>>,
    scale: f32,
  },
  Image {
    asset: Asset,
    matrix: Transform2D,
    source: [f32; 4],
    alpha: f32,
    smooth: bool,
    clip: Option<Arc<Clip>>,
    scale: f32,
  },
  /// Opens an isolated layer. Everything until the matching `EndLayer` composites
  /// into a target of its own at full alpha; the layer then composites onto its
  /// parent once, with `alpha` and `blend`.
  BeginLayer {
    alpha: f32,
    blend: BlendMode,
  },
  EndLayer,
  Readback(Completion, CanvasMetrics, u64),
}

impl Command {
  pub fn bytes(&self) -> usize {
    std::mem::size_of::<Self>()
      + match self {
        Self::Path {
          path, gradient, clip, ..
        } => {
          path.points().len() * 16
            + clip.as_ref().map_or(0, |c| c.bytes)
            + gradient.as_ref().map_or(0, |g| g.ramp.data.len())
        }
        Self::Image { asset, clip, .. } => asset.data.len() + clip.as_ref().map_or(0, |c| c.bytes),
        _ => 0,
      }
  }
}

/// An explicit asynchronous copy from the canvas. Poll on the UI thread, or
/// wait on a worker while the host continues rendering.
pub struct CanvasReadback {
  inner: Arc<ReadbackState>,
}
struct ReadbackState {
  result: Mutex<Option<Result<CanvasSnapshot, CanvasError>>>,
  ready: parking_lot::Condvar,
  permit: Mutex<Option<ReadbackPermit>>,
}
pub(crate) struct Completion(Option<Arc<ReadbackState>>);
impl CanvasReadback {
  pub(crate) fn pair() -> (Self, Completion) {
    let inner = Arc::new(ReadbackState {
      result: Mutex::new(None),
      ready: parking_lot::Condvar::new(),
      permit: Mutex::new(None),
    });
    (Self { inner: inner.clone() }, Completion(Some(inner)))
  }
  pub fn try_take(&self) -> Option<Result<CanvasSnapshot, CanvasError>> {
    self.inner.result.lock().take()
  }
  pub fn wait_timeout(&self, timeout: Duration) -> Option<Result<CanvasSnapshot, CanvasError>> {
    let mut result = self.inner.result.lock();
    let deadline = std::time::Instant::now() + timeout;
    while result.is_none() {
      if self.inner.ready.wait_until(&mut result, deadline).timed_out() {
        break;
      }
    }
    result.take()
  }
}
impl Completion {
  pub fn reserve(&mut self, pending: &Arc<std::sync::atomic::AtomicUsize>) -> bool {
    if pending
      .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| (n < 2).then_some(n + 1))
      .is_err()
    {
      return false;
    }
    if READBACKS
      .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| (n < 8).then_some(n + 1))
      .is_err()
    {
      pending.fetch_sub(1, Ordering::AcqRel);
      return false;
    }
    *self.0.as_ref().unwrap().permit.lock() = Some(ReadbackPermit(pending.clone()));
    true
  }
  pub fn finish(mut self, result: Result<CanvasSnapshot, CanvasError>) {
    let inner = self.0.take().unwrap();
    inner.permit.lock().take();
    *inner.result.lock() = Some(result);
    inner.ready.notify_all();
  }
}
impl Drop for Completion {
  fn drop(&mut self) {
    if let Some(inner) = &self.0 {
      inner.permit.lock().take();
      *inner.result.lock() = Some(Err(CanvasError::RendererLost));
      inner.ready.notify_all();
    }
  }
}

/// A lease retains its memory charge until submission. Dropping an unsubmitted
/// lease restores commands in front of concurrent writes, preserving order.
pub(crate) struct Batch {
  canvas: CanvasHandle,
  pub commands: Vec<Command>,
  bytes: usize,
  submitted: bool,
  metrics_revision: u64,
  replayable: bool,
}
impl Batch {
  pub fn take_commands(&mut self) -> Vec<Command> {
    self.replayable = false;
    std::mem::take(&mut self.commands)
  }
  pub fn submit(mut self) {
    self.submitted = true;
    let mut s = self.canvas.inner.lock();
    s.inflight_bytes -= self.bytes;
  }
}
impl Drop for Batch {
  fn drop(&mut self) {
    if !self.submitted {
      let mut s = self.canvas.inner.lock();
      s.inflight_bytes -= self.bytes;
      if !self.replayable {
        s.error = Some(CanvasError::RendererLost);
      } else if s.attached && s.metrics.revision == self.metrics_revision {
        self.commands.append(&mut s.commands);
        s.commands = std::mem::take(&mut self.commands);
        s.command_bytes += self.bytes;
        s.pending_paint = true;
      }
    }
  }
}
impl CanvasHandle {
  pub(crate) fn take_batch(&self) -> Option<Batch> {
    let mut s = self.inner.lock();
    if s.software || s.commands.is_empty() {
      return None;
    }
    let bytes = std::mem::take(&mut s.command_bytes);
    s.inflight_bytes += bytes;
    Some(Batch {
      canvas: self.clone(),
      commands: std::mem::take(&mut s.commands),
      bytes,
      submitted: false,
      metrics_revision: s.metrics.revision,
      replayable: true,
    })
  }
  pub(crate) fn unsupported(&self) {
    let mut s = self.inner.lock();
    if !s.software {
      s.error = Some(CanvasError::UnsupportedBackend);
      for command in s.commands.drain(..) {
        if let Command::Readback(done, ..) = command {
          done.finish(Err(CanvasError::UnsupportedBackend));
        }
      }
      s.command_bytes = 0;
    }
  }
  pub(crate) fn set_gpu_bytes(&self, bytes: usize) {
    self.inner.lock().gpu_bytes = bytes;
  }
  pub(crate) fn set_gpu_error(&self, error: CanvasError) {
    self.inner.lock().error = Some(error);
  }
  pub(crate) fn record_mesh_cache(&self, before: cache::MeshCacheStats, after: cache::MeshCacheStats) {
    let mut surface = self.inner.lock();
    surface.gpu.mesh_cache_hits += after.hits - before.hits;
    surface.gpu.mesh_cache_misses += after.misses - before.misses;
    surface.gpu.mesh_cache_evictions += after.evictions - before.evictions;
    surface.gpu.mesh_cache_entries = after.entries;
    surface.gpu.mesh_cache_bytes = after.bytes;
  }
  pub(crate) fn record_gpu_update(&self, vertices: usize, tiles: usize, uploaded: usize) {
    let mut s = self.inner.lock();
    s.gpu.batches += 1;
    s.gpu.vertices += vertices as u64;
    s.gpu.tiles += tiles as u64;
    s.gpu.uploaded_bytes += uploaded as u64;
  }
  pub(crate) fn software(&self) {
    self.inner.lock().software = true;
  }
}

/// Commands recorded while an isolated layer is open. They are held here rather
/// than in the surface queue so a layer always reaches the renderer whole: a
/// batch taken mid-layer would otherwise carry an unmatched `BeginLayer`.
pub(crate) struct LayerFrame {
  pub alpha: f32,
  pub blend: BlendMode,
  pub commands: Vec<Command>,
  pub bytes: usize,
}

impl Surface {
  pub(super) fn open_layer(&mut self, alpha: f32, blend: BlendMode) -> Result<(), CanvasError> {
    if !self.attached {
      self.error = Some(CanvasError::Detached);
      return Err(CanvasError::Detached);
    }
    if self.layers.len() >= MAX_LAYER_DEPTH {
      self.error = Some(CanvasError::StateLimit);
      return Err(CanvasError::StateLimit);
    }
    self.layers.push(LayerFrame {
      alpha,
      blend,
      commands: Vec::new(),
      bytes: 0,
    });
    Ok(())
  }

  /// Closes the innermost layer and hands its recorded work to its parent as one
  /// `BeginLayer … EndLayer` run. An empty layer composites nothing and is dropped.
  pub(super) fn close_layer(&mut self) -> bool {
    let Some(frame) = self.layers.pop() else {
      return false;
    };
    self.layer_bytes -= frame.bytes;
    self.layer_commands -= frame.commands.len();
    if frame.commands.is_empty() {
      return false;
    }
    if !self.enqueue(Command::BeginLayer {
      alpha: frame.alpha,
      blend: frame.blend,
    }) {
      return false;
    }
    for command in frame.commands {
      if !self.enqueue(command) {
        return false;
      }
    }
    self.enqueue(Command::EndLayer)
  }

  pub(super) fn discard_layers(&mut self) {
    self.layers.clear();
    self.layer_bytes = 0;
    self.layer_commands = 0;
  }

  pub(super) fn enqueue(&mut self, command: Command) -> bool {
    if !self.attached {
      self.error = Some(CanvasError::Detached);
      return false;
    }
    if matches!(command, Command::Clear) {
      if let Some(frame) = self.layers.last_mut() {
        // A layer starts transparent, so clearing it only discards its own work.
        self.layer_bytes -= frame.bytes;
        self.layer_commands -= frame.commands.len();
        frame.bytes = 0;
        frame.commands.clear();
        return true;
      }
      // Preserve snapshot barriers; everything after the last one is obsolete.
      let keep = self
        .commands
        .iter()
        .rposition(|c| {
          matches!(
            c,
            Command::Readback(..)
              | Command::Resize { .. }
              | Command::BeginPresentation(..)
              | Command::CommitPresentation(..)
              | Command::AbortPresentation(_)
              | Command::ForgetArtwork
          )
        })
        .map_or(0, |i| i + 1);
      self.commands.truncate(keep);
      self.command_bytes = self.commands.iter().map(Command::bytes).sum();
    }
    let bytes = command.bytes();
    let queued = self.commands.len() + self.layer_commands;
    if queued >= MAX_COMMANDS || self.command_bytes + self.layer_bytes + self.inflight_bytes + bytes > MAX_QUEUE_BYTES {
      self.error = Some(CanvasError::QueueFull);
      if let Command::Readback(done, ..) = command {
        done.finish(Err(CanvasError::QueueFull));
      }
      return false;
    }
    if let Some(frame) = self.layers.last_mut() {
      frame.bytes += bytes;
      frame.commands.push(command);
      self.layer_bytes += bytes;
      self.layer_commands += 1;
      return true;
    }
    self.command_bytes += bytes;
    self.commands.push(command);
    true
  }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Vertex {
  pub position: [f32; 2],
  pub uv: [f32; 2],
  pub color: [f32; 4],
}
/// What a draw's fragments are coloured by.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DrawKind {
  /// The vertex colour, premultiplied.
  Solid,
  /// A premultiplied RGBA asset sampled through `uv`, scaled by the vertex colour.
  Image,
  /// A gradient ramp sampled at the parameter `uv` carries in the gradient's frame.
  Gradient(GradientKind),
}

pub(crate) struct Draw {
  pub vertices: Range<u32>,
  pub clips: Vec<Range<u32>>,
  pub bounds: [f32; 4],
  pub asset: Option<Asset>,
  pub smooth: bool,
  pub erase: bool,
  pub kind: DrawKind,
}
/// One entry of a prepared batch, in order: a draw, or an isolated layer's
/// boundary.
pub(crate) enum Step {
  Draw(Draw),
  /// `end` is the index of the matching [`Step::End`]; `bounds` covers every
  /// draw inside, so a tile the layer does not touch skips the whole run.
  Begin {
    alpha: f32,
    blend: BlendMode,
    bounds: [f32; 4],
    end: usize,
  },
  End,
}
impl Step {
  pub fn draw(&self) -> Option<&Draw> {
    match self {
      Self::Draw(draw) => Some(draw),
      _ => None,
    }
  }
}
const NO_BOUNDS: [f32; 4] = [f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY];
fn cover(into: &mut [f32; 4], with: [f32; 4]) {
  into[0] = into[0].min(with[0]);
  into[1] = into[1].min(with[1]);
  into[2] = into[2].max(with[2]);
  into[3] = into[3].max(with[3]);
}

pub(crate) fn unpremultiply(mut rgba: Vec<u8>) -> Vec<u8> {
  for p in rgba.chunks_exact_mut(4) {
    if p[3] != 0 {
      let a = u32::from(p[3]);
      for c in &mut p[..3] {
        *c = ((u32::from(*c) * 255 + a / 2) / a).min(255) as u8;
      }
    }
  }
  rgba
}

#[cfg(test)]
mod tests {
  use super::*;
  fn canvas() -> CanvasHandle {
    let c = CanvasHandle::new();
    {
      let mut s = c.inner.lock();
      s.attached = true;
      s.metrics.pixel_width = 100;
      s.metrics.pixel_height = 100;
    }
    c
  }
  #[test]
  fn command_leases_bound_memory_restore_order_and_retire_after_submit() {
    let c = canvas();
    let d = c.context_2d();
    d.set_fill_style("#ff000080");
    d.fill_rect(0., 0., 10., 10.);
    let bytes = c.status().pending_bytes;
    let batch = c.take_batch().unwrap();
    assert_eq!(c.status().pending_bytes, bytes);
    d.set_fill_style("#0000ff");
    d.fill_rect(10., 0., 10., 10.);
    drop(batch);
    let batch = c.take_batch().unwrap();
    assert_eq!(batch.commands.len(), 2);
    assert!(matches!(&batch.commands[0],Command::Path {color,..} if color[0]>0. && color[2]==0.));
    batch.submit();
    assert_eq!(c.status().pending_bytes, 0);
    assert!(c.take_batch().is_none());
    d.fill_rect(0., 0., 1., 1.);
    let mut failed = c.take_batch().unwrap();
    failed.take_commands();
    drop(failed);
    assert_eq!(c.status().pending_bytes, 0, "failed encoding releases its charge");
    assert_eq!(c.status().error, Some(CanvasError::RendererLost));
    for _ in 0..9000 {
      d.fill_rect(0., 0., 1., 1.);
    }
    assert_eq!(c.status().error, Some(CanvasError::QueueFull));
    assert!(c.status().pending_bytes <= MAX_QUEUE_BYTES);
    d.clear();
    assert!(c.status().pending_bytes < 1024);
  }
  #[test]
  fn readback_limit_includes_inflight_work_and_clear_keeps_barriers() {
    let c = canvas();
    let d = c.context_2d();
    d.fill_rect(0., 0., 10., 10.);
    let first = c.snapshot();
    let batch = c.take_batch().unwrap();
    let second = c.snapshot();
    d.clear();
    assert_eq!(c.snapshot().try_take().unwrap().unwrap_err(), CanvasError::QueueFull);
    drop(batch);
    let batch = c.take_batch().unwrap();
    assert_eq!(batch.commands.len(), 4);
    c.detach();
    drop(batch);
    assert!(first.try_take().unwrap().is_err());
    assert!(second.try_take().unwrap().is_err());
    assert_eq!(c.status().pending_bytes, 0);
  }
}
