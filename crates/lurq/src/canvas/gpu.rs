//! Backend-neutral, bounded canvas work. No canvas-sized CPU bitmap lives here.
#![cfg_attr(
  not(any(feature = "wgpu", all(feature = "dx12", target_os = "windows"))),
  allow(dead_code)
)]
use std::{collections::HashMap, ops::Range, time::Duration};

use lyon::{math::point, path::Path as LyonPath, tessellation::*};

use super::{FillRule, *};

pub(crate) const TILE: u32 = 512;
pub(crate) const MAX_QUEUE_BYTES: usize = 64 * 1024 * 1024;
const MAX_VERTICES: usize = 1_048_576;
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
  pub path: Option<Path>,
  pub rule: FillRule,
  pub previous: Option<Arc<Clip>>,
  pub depth: u32,
  pub bytes: usize,
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
  Path {
    path: Path,
    rule: FillRule,
    color: [f32; 4],
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
  Readback(Completion, CanvasMetrics, u64),
}

impl Command {
  pub fn bytes(&self) -> usize {
    std::mem::size_of::<Self>()
      + match self {
        Self::Path { path, clip, .. } => path.points().len() * 16 + clip.as_ref().map_or(0, |c| c.bytes),
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

impl Surface {
  pub(super) fn enqueue(&mut self, command: Command) -> bool {
    if !self.attached {
      self.error = Some(CanvasError::Detached);
      return false;
    }
    if matches!(command, Command::Clear) {
      // Preserve snapshot barriers; everything after the last one is obsolete.
      let keep = self
        .commands
        .iter()
        .rposition(|c| matches!(c, Command::Readback(..) | Command::Resize { .. }))
        .map_or(0, |i| i + 1);
      self.commands.truncate(keep);
      self.command_bytes = self.commands.iter().map(Command::bytes).sum();
    }
    let bytes = command.bytes();
    if self.commands.len() >= 8192 || self.command_bytes + self.inflight_bytes + bytes > MAX_QUEUE_BYTES {
      self.error = Some(CanvasError::QueueFull);
      if let Command::Readback(done, ..) = command {
        done.finish(Err(CanvasError::QueueFull));
      }
      return false;
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
pub(crate) struct Draw {
  pub vertices: Range<u32>,
  pub clips: Vec<Range<u32>>,
  pub bounds: [f32; 4],
  pub asset: Option<Asset>,
  pub smooth: bool,
  pub erase: bool,
}
#[derive(Default)]
pub(crate) struct Prepared {
  pub vertices: Vec<Vertex>,
  pub draws: Vec<Draw>,
  pub clear: bool,
}
impl Prepared {
  pub fn new(commands: &[Command]) -> Result<Self, CanvasError> {
    let mut result = Self::default();
    let mut clips = HashMap::new();
    for command in commands {
      if matches!(command, Command::Clear) {
        result = Self {
          clear: true,
          ..Self::default()
        };
        clips.clear();
        continue;
      }
      let (clip, scale) = match command {
        Command::Path { clip, scale, .. } | Command::Image { clip, scale, .. } => (clip, *scale),
        _ => continue,
      };
      let mut clip_ranges = Vec::new();
      let mut chain = Vec::new();
      let mut clip = clip.as_ref();
      while let Some(c) = clip {
        chain.push(c);
        clip = c.previous.as_ref();
      }
      for clip in chain.into_iter().rev() {
        let key = (Arc::as_ptr(clip) as usize, scale.to_bits());
        let range = if let Some(range) = clips.get(&key) {
          Range::<u32>::clone(range)
        } else {
          let start = result.vertices.len() as u32;
          if let Some(path) = &clip.path {
            let path = path.clone().transform(tiny_skia::Transform::from_scale(scale, scale));
            if let Some(path) = path {
              mesh(&path, clip.rule, [0.; 4], &mut result.vertices)?;
            }
          }
          let range = start..result.vertices.len() as u32;
          clips.insert(key, range.clone());
          range
        };
        clip_ranges.push(range);
      }
      let start = result.vertices.len() as u32;
      let (asset, smooth, erase) = match command {
        Command::Path {
          path,
          rule,
          color,
          erase,
          ..
        } => {
          mesh(path, *rule, *color, &mut result.vertices)?;
          (None, false, *erase)
        }
        Command::Image {
          asset,
          matrix,
          source: [x, y, w, h],
          alpha,
          smooth,
          ..
        } => {
          let corners = [[*x, *y], [x + w, *y], [x + w, y + h], [*x, y + h]];
          for index in [0, 1, 2, 0, 2, 3] {
            let [x, y] = corners[index];
            let (px, py) = matrix.transform_point(x, y);
            result.vertices.push(Vertex {
              position: [px, py],
              uv: [x / asset.width as f32, y / asset.height as f32],
              color: [*alpha; 4],
            });
          }
          (Some(asset.clone()), *smooth, false)
        }
        _ => unreachable!(),
      };
      let end = result.vertices.len() as u32;
      if end as usize > MAX_VERTICES {
        return Err(CanvasError::StateLimit);
      }
      let mut bounds = [f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY];
      for v in &result.vertices[start as usize..end as usize] {
        bounds[0] = bounds[0].min(v.position[0]);
        bounds[1] = bounds[1].min(v.position[1]);
        bounds[2] = bounds[2].max(v.position[0]);
        bounds[3] = bounds[3].max(v.position[1]);
      }
      result.draws.push(Draw {
        vertices: start..end,
        clips: clip_ranges,
        bounds,
        asset,
        smooth,
        erase,
      });
    }
    Ok(result)
  }
  pub fn tiles(&self, width: u32, height: u32) -> Vec<[u32; 4]> {
    let mut result = Vec::new();
    for y in (0..height).step_by(TILE as usize) {
      for x in (0..width).step_by(TILE as usize) {
        let tile = [x, y, TILE.min(width - x), TILE.min(height - y)];
        if self.draws.iter().any(|d| d.intersects(tile)) {
          result.push(tile);
        }
      }
    }
    result
  }
}
impl Draw {
  pub fn intersects(&self, [x, y, w, h]: [u32; 4]) -> bool {
    self.bounds[0] < (x + w) as f32
      && self.bounds[1] < (y + h) as f32
      && self.bounds[2] > x as f32
      && self.bounds[3] > y as f32
  }
}
fn mesh(path: &Path, rule: FillRule, color: [f32; 4], output: &mut Vec<Vertex>) -> Result<(), CanvasError> {
  let mut builder = LyonPath::builder();
  let mut open = false;
  for segment in path.segments() {
    use tiny_skia::PathSegment::*;
    match segment {
      MoveTo(p) => {
        if open {
          builder.end(true);
        }
        builder.begin(point(p.x, p.y));
        open = true;
      }
      LineTo(p) => {
        builder.line_to(point(p.x, p.y));
      }
      QuadTo(a, b) => {
        builder.quadratic_bezier_to(point(a.x, a.y), point(b.x, b.y));
      }
      CubicTo(a, b, c) => {
        builder.cubic_bezier_to(point(a.x, a.y), point(b.x, b.y), point(c.x, c.y));
      }
      Close => {
        if open {
          builder.end(true);
          open = false;
        }
      }
    }
  }
  if open {
    builder.end(true);
  }
  let path = builder.build();
  let options = FillOptions::default().with_tolerance(0.1).with_fill_rule(match rule {
    FillRule::NonZero => lyon::path::FillRule::NonZero,
    FillRule::EvenOdd => lyon::path::FillRule::EvenOdd,
  });
  // The custom builder limits expansion while tessellating, including malicious
  // self-intersecting paths whose triangulation is much larger than their input.
  let mut geometry = LimitedGeometry {
    vertices: Vec::new(),
    output,
    color,
    overflow: false,
  };
  FillTessellator::new()
    .tessellate_path(&path, &options, &mut geometry)
    .map_err(|_| CanvasError::StateLimit)?;
  if geometry.overflow {
    return Err(CanvasError::StateLimit);
  }
  Ok(())
}
struct LimitedGeometry<'a> {
  vertices: Vec<[f32; 2]>,
  output: &'a mut Vec<Vertex>,
  color: [f32; 4],
  overflow: bool,
}
impl GeometryBuilder for LimitedGeometry<'_> {
  fn begin_geometry(&mut self) {}
  fn end_geometry(&mut self) {}
  fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId) {
    if self.output.len() + 3 <= MAX_VERTICES {
      for i in [a, b, c] {
        self.output.push(Vertex {
          position: self.vertices[i.to_usize()],
          uv: [0.; 2],
          color: self.color,
        });
      }
    } else {
      self.overflow = true;
    }
  }
  fn abort_geometry(&mut self) {}
}
impl FillGeometryBuilder for LimitedGeometry<'_> {
  fn add_fill_vertex(&mut self, vertex: FillVertex<'_>) -> Result<VertexId, GeometryBuilderError> {
    if self.vertices.len() >= MAX_VERTICES || self.output.len() + 3 > MAX_VERTICES {
      return Err(GeometryBuilderError::TooManyVertices);
    }
    let id = VertexId(self.vertices.len() as u32);
    let p = vertex.position();
    self.vertices.push([p.x, p.y]);
    Ok(id)
  }
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
