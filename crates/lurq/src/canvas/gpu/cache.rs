use std::collections::VecDeque;

use super::*;

#[cfg(test)]
mod tests;

// Shared by every canvas in a renderer. Source snapshots, triangle payloads and
// an allowance for keys/table/queue metadata are charged before insertion. This
// fits thousands of ordinary paths at several zoom levels without retaining an
// unbounded drawing history. The entry cap independently bounds table overhead.
const MAX_CACHE_BYTES: usize = 32 * 1024 * 1024;
const MAX_CACHE_ENTRIES: usize = 32_768;

#[derive(Clone, PartialEq, Eq, Hash)]
struct Key {
  path: Geometry,
  rule: FillRule,
  tolerance: u32,
}
struct CachedMesh {
  positions: Arc<[[f32; 2]]>,
  bytes: usize,
}

/// FIFO eviction has constant amortized insertion cost, including workloads
/// larger than the cache. Hits do not grow an access log or scan the cache.
#[derive(Default)]
pub(crate) struct MeshCache {
  entries: HashMap<Key, CachedMesh>,
  order: VecDeque<Key>,
  bytes: usize,
  #[cfg(test)]
  pub(super) misses: usize,
}
impl MeshCache {
  pub(super) fn append(
    &mut self,
    path: &Geometry,
    matrix: Transform2D,
    rule: FillRule,
    color: [f32; 4],
    output: &mut Vec<Vertex>,
  ) -> Result<(), CanvasError> {
    if !finite_bounds(path, matrix) {
      return Ok(());
    }
    let tolerance = tolerance(path, matrix);
    let key = Key {
      path: path.clone(),
      rule,
      tolerance: tolerance.to_bits(),
    };
    let positions = if let Some(mesh) = self.entries.get(&key) {
      mesh.positions.clone()
    } else {
      #[cfg(test)]
      {
        self.misses += 1;
      }
      let mut vertices = Vec::new();
      mesh(path, rule, tolerance, [0.; 4], &mut vertices)?;
      let positions: Arc<[[f32; 2]]> = vertices.iter().map(|v| v.position).collect();
      self.insert(key, positions.clone());
      positions
    };
    if output.len() + positions.len() > MAX_VERTICES {
      return Err(CanvasError::StateLimit);
    }
    output.extend(positions.iter().map(|[x, y]| {
      let (x, y) = matrix.transform_point(*x, *y);
      Vertex {
        position: [x, y],
        uv: [0.; 2],
        color,
      }
    }));
    Ok(())
  }

  fn insert(&mut self, key: Key, positions: Arc<[[f32; 2]]>) {
    let bytes = key.path.bytes() + std::mem::size_of_val(positions.as_ref()) + 256;
    if bytes > MAX_CACHE_BYTES {
      return;
    }
    while self.bytes + bytes > MAX_CACHE_BYTES || self.entries.len() >= MAX_CACHE_ENTRIES {
      let oldest = self.order.pop_front().unwrap();
      self.bytes -= self.entries.remove(&oldest).unwrap().bytes;
    }
    self.bytes += bytes;
    self.order.push_back(key.clone());
    self.entries.insert(key, CachedMesh { positions, bytes });
  }
}

fn finite_bounds(path: &Path, matrix: Transform2D) -> bool {
  let bounds = path.bounds();
  [bounds.left(), bounds.right()].into_iter().all(|x| {
    [bounds.top(), bounds.bottom()].into_iter().all(|y| {
      let (x, y) = matrix.transform_point(x, y);
      x.is_finite() && y.is_finite()
    })
  })
}

fn tolerance(path: &Geometry, matrix: Transform2D) -> f32 {
  if !path.has_curves() {
    return 0.1;
  }
  // Largest singular value of the linear transform, computed in f64 to handle
  // skew, anisotropic scale, reflections and extreme finite f32 transforms.
  let [a, b, c, d] = [matrix.a, matrix.b, matrix.c, matrix.d].map(f64::from);
  let scale = ((a + d).hypot(b - c) + (a - d).hypot(b + c)) * 0.5;
  if scale == 0. {
    return 0.1;
  }
  // Round upward: flattened curve error remains <= 0.1 physical pixels. A pan
  // never changes this key. Polygon meshes are independent of scale altogether.
  let bucket = 2_f64.powf(scale.log2().ceil());
  (0.1 / bucket).min(f64::from(f32::MAX)) as f32
}
