//! Where a GPU renderer's Canvas cache frame ends.
//!
//! The asset and shaped-text caches never evict what the current or the
//! previous frame used, so a page drawn in the same order every frame stays
//! cached even when it needs more than the budget. That holds only while a
//! frame is one whole drawing of the page. An ordinary canvas is drawn in one
//! encode, so each encode ends a frame. A replacement presentation
//! (`begin_presentation`) may be drawn in bounded batches over several encodes
//! while the completed front stays visible. Ending a frame at each of those
//! encodes would split one page over several frames, and a page charged more
//! than the budget would evict its own first batches before the next
//! replacement drew them again.
//!
//! So an encode does not end the frame while one of the renderer's canvases
//! holds a replacement that is still open after the encode and none of whose
//! replacements ended during it. The frame ends at the encode that commits,
//! aborts or supersedes the replacement, or resizes or detaches its canvas.
//!
//! The completed front is a render target of its own, not a cached texture. It
//! holds the pixels drawn before the replacement began, so neither cache's
//! eviction can release it. A pass that only reprojects retained artwork uses
//! neither cache, and ending a frame with no use since the previous end does
//! not age entries.
// Only the GPU backends encode; tests compile it without them.
#![cfg_attr(
  not(any(feature = "wgpu", all(feature = "dx12", target_os = "windows"))),
  allow(dead_code)
)]
use std::collections::{HashMap, HashSet};

use super::{CanvasHandle, CanvasId, CanvasWeak};

#[cfg(test)]
mod tests;

#[derive(Default)]
pub(crate) struct FrameBoundary {
  /// Canvases whose replacement ended during the current encode.
  ended: HashSet<CanvasId>,
  /// Canvases encoded since the frame last ended. Their text engines' frames
  /// end with it, because their text was shaped for the same page.
  text: Vec<(CanvasId, CanvasWeak)>,
}

impl FrameBoundary {
  pub(crate) fn begin_encode(&mut self) {
    // An encode that failed part-way has not ended any replacement.
    self.ended.clear();
  }

  /// Takes the open replacement of canvas `id` out of `fronts`, noting that
  /// this encode ended it.
  pub(crate) fn end_replacement<B>(
    &mut self,
    fronts: &mut HashMap<CanvasId, (u64, B)>,
    id: CanvasId,
  ) -> Option<(u64, B)> {
    let front = fronts.remove(&id);
    if front.is_some() {
      self.ended.insert(id);
    }
    front
  }

  /// Notes that the encode takes a batch of `canvas`.
  pub(crate) fn encodes(&mut self, canvas: &CanvasHandle) {
    let id = canvas.surface_id();
    if !self.text.iter().any(|(known, _)| *known == id) {
      self.text.push((id, canvas.downgrade()));
    }
  }

  /// Ends the encode, given the replacements still open after it, and returns
  /// whether it also ends the cache frame. When it does, the text frames of the
  /// canvases encoded since the previous end end as well.
  pub(crate) fn end_encode<B>(&mut self, fronts: &HashMap<CanvasId, (u64, B)>) -> bool {
    if fronts.keys().any(|id| !self.ended.contains(id)) {
      return false;
    }
    for (_, canvas) in self.text.drain(..) {
      if let Some(canvas) = canvas.upgrade() {
        canvas.finish_text_frame();
      }
    }
    true
  }
}
