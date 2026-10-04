//! The GPU backends' shared policy for uploaded Canvas images and text bitmaps.
// Only the GPU backends upload assets.
#![cfg_attr(
  not(any(feature = "wgpu", all(feature = "dx12", target_os = "windows"))),
  allow(dead_code)
)]
use std::collections::HashMap;

use super::frame_cache::{FrameCache, Limits};

#[cfg(test)]
mod tests;

/// Every texture is charged its RGBA bytes, but at least this much, which
/// bounds the per-texture overhead of many small images and labels.
const MIN_CHARGE: usize = 64 * 1024;

/// How many charged bytes of uploaded Canvas images and text a GPU renderer
/// keeps between frames. Set it with `with_canvas_asset_budget` on the WGPU
/// or DX12 render engine; the default is [`DEFAULT_BYTES`](Self::DEFAULT_BYTES).
///
/// Each texture is charged its RGBA bytes, but at least 64 KiB. A texture that
/// a canvas drew in the current or the previous frame is never evicted. When
/// those two frames alone need more than the budget, the renderer keeps them
/// anyway, up to [`ceiling_bytes`](Self::ceiling_bytes); a texture beyond that
/// is uploaded and drawn but not kept past its frame, so the textures that are
/// kept stay resident instead of being evicted and uploaded again every frame.
/// Once the frames move on to fewer textures, the cache returns to the budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CanvasAssetBudget(usize);

impl CanvasAssetBudget {
  /// 64 MiB.
  pub const DEFAULT_BYTES: usize = 64 * 1024 * 1024;
  /// 1 GiB. A larger budget is clamped to it, so a renderer never keeps more
  /// than 2 GiB of charged textures.
  pub const MAX_BYTES: usize = 1024 * 1024 * 1024;

  /// A budget of `bytes`, at most [`MAX_BYTES`](Self::MAX_BYTES). A budget of
  /// 0 keeps nothing between frames.
  pub const fn new(bytes: usize) -> Self {
    Self(if bytes > Self::MAX_BYTES {
      Self::MAX_BYTES
    } else {
      bytes
    })
  }
  pub const fn bytes(self) -> usize {
    self.0
  }
  /// The most the renderer keeps while the textures of the current and the
  /// previous frame need more than the budget: twice the budget.
  pub const fn ceiling_bytes(self) -> usize {
    self.0 * 2
  }
}

impl Default for CanvasAssetBudget {
  fn default() -> Self {
    Self::new(Self::DEFAULT_BYTES)
  }
}

/// What the cache did during one frame, for the profiler.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(not(any(test, feature = "perf_profile")), allow(dead_code))]
pub(crate) struct AssetCacheStats {
  pub evictions: usize,
  /// Textures uploaded for the frame but not kept past it, because the
  /// textures of the current and previous frame filled the ceiling.
  pub uncached: usize,
  pub uncached_bytes: usize,
  /// The most the cache was charged above its budget during the frame.
  pub stretch_bytes: usize,
}

/// One renderer's uploaded assets, keyed by asset id. A texture that is not
/// kept stays usable until its frame ends, so a frame uploads an asset once.
pub(crate) struct AssetCache<T> {
  resident: FrameCache<T>,
  overflow: HashMap<u64, T>,
  stats: AssetCacheStats,
}

impl<T> AssetCache<T> {
  pub(crate) fn new(budget: CanvasAssetBudget) -> Self {
    Self {
      resident: FrameCache::framed(Limits {
        budget: budget.bytes(),
        ceiling: budget.ceiling_bytes(),
      }),
      overflow: HashMap::new(),
      stats: AssetCacheStats::default(),
    }
  }

  /// Charged bytes of the textures kept between frames.
  #[cfg(any(test, feature = "perf_profile"))]
  pub(crate) fn bytes(&self) -> usize {
    self.resident.bytes()
  }
  #[cfg(any(test, feature = "perf_profile"))]
  pub(crate) fn len(&self) -> usize {
    self.resident.len()
  }
  #[cfg(any(test, feature = "perf_profile"))]
  pub(crate) fn budget(&self) -> usize {
    self.resident.limits().budget
  }

  /// The texture uploaded for asset `id`, counting it as drawn this frame.
  pub(crate) fn get(&mut self, id: u64) -> Option<&T> {
    if self.overflow.contains_key(&id) {
      return self.overflow.get(&id);
    }
    self.resident.get(id)
  }
  /// The texture uploaded for asset `id`, for a draw that already got it.
  #[cfg(any(test, feature = "wgpu"))]
  pub(crate) fn peek(&self, id: u64) -> Option<&T> {
    self.overflow.get(&id).or_else(|| self.resident.peek(id))
  }

  /// Keeps `texture`, just uploaded with `len` RGBA bytes for asset `id`.
  /// Textures evicted to make room are handed to `retire`.
  pub(crate) fn insert(&mut self, id: u64, texture: T, len: usize, mut retire: impl FnMut(T)) {
    let bytes = len.max(MIN_CHARGE);
    let stats = &mut self.stats;
    let kept = self.resident.insert(id, texture, bytes, |old| {
      stats.evictions += 1;
      retire(old);
    });
    match kept {
      Ok(()) => self.note_stretch(),
      Err(texture) => {
        stats.uncached += 1;
        stats.uncached_bytes += bytes;
        self.overflow.insert(id, texture);
      }
    }
  }

  fn note_stretch(&mut self) {
    let above = self.resident.bytes().saturating_sub(self.resident.limits().budget);
    self.stats.stretch_bytes = self.stats.stretch_bytes.max(above);
  }

  /// Ends the frame. Textures that were not kept, and those evicted to return
  /// toward the budget, are handed to `retire`.
  pub(crate) fn finish_frame(&mut self, mut retire: impl FnMut(T)) -> AssetCacheStats {
    for (_, texture) in self.overflow.drain() {
      retire(texture);
    }
    // A frame whose textures were all cached still holds a stretched cache.
    self.note_stretch();
    let stats = &mut self.stats;
    self.resident.close_frame(|old| {
      stats.evictions += 1;
      retire(old);
    });
    std::mem::take(&mut self.stats)
  }
}
