//! Reuse requires the existing DX12 submission fence, never wall-clock or frame-index guesses.
use super::*;
pub(super) struct Spare {
  pub(super) id: CanvasId,
  fence: u64,
  back: Backing,
}
pub(super) struct ArtworkLease {
  pub(super) fence: u64,
  pub(super) art: std::sync::Arc<Artwork>,
}
impl Renderer {
  pub(super) unsafe fn reap(&mut self, state: &Dx12State, live: &HashSet<CanvasId>) {
    let completed = state.fence.GetCompletedValue();
    if completed == u64::MAX {
      return;
    }
    self.retired_artwork.retain(|lease| lease.fence > completed);
    self
      .spares
      .retain(|lease| live.contains(&lease.id) || lease.fence > completed);
  }
  pub(super) fn retire_back(&mut self, state: &Dx12State, id: CanvasId, mut back: Backing) {
    if let Some(art) = back.artwork.take() {
      self.retired_artwork.push(ArtworkLease {
        fence: state.next_fence_value,
        art,
      });
    }
    self.spares.push(Spare {
      id,
      fence: state.next_fence_value,
      back,
    });
  }
  pub(super) unsafe fn take_spare(
    &mut self,
    state: &Dx12State,
    id: CanvasId,
    width: u32,
    height: u32,
  ) -> Option<Backing> {
    let completed = state.fence.GetCompletedValue();
    if completed == u64::MAX {
      return None;
    }
    self.retired_artwork.retain(|lease| lease.fence > completed);
    self.spares.retain(|lease| {
      lease.id != id || lease.fence > completed || (lease.back.width == width && lease.back.height == height)
    });
    let position = self.spares.iter().position(|lease| {
      lease.id == id && lease.fence <= completed && lease.back.width == width && lease.back.height == height
    })?;
    Some(self.spares.swap_remove(position).back)
  }
  pub(super) unsafe fn allocation_admitted(&self, state: &Dx12State, bytes: usize) -> bool {
    replacement_admitted(
      self.target_bytes(),
      bytes,
      resources::workspace_ceiling_bytes(&state.device),
    )
  }
  pub(super) unsafe fn workspace_bytes(&self, state: &Dx12State) -> usize {
    [&self.scratch, &self.resolve, &self._stencil]
      .into_iter()
      .chain(self.saved.iter())
      .chain(self.layer.iter())
      .fold(0usize, |bytes, resource| {
        bytes.saturating_add(resources::resource_allocation_bytes(&state.device, resource))
      })
  }
  pub(super) fn target_bytes(&self) -> usize {
    let mut charge = TargetCharge::default();
    for back in self
      .surfaces
      .values()
      .chain(self.fronts.values().map(|(_, b)| b))
      .chain(self.spares.iter().map(|s| &s.back))
    {
      charge.add(back.bytes);
      if let Some(art) = &back.artwork {
        charge.shared(std::sync::Arc::as_ptr(art) as usize, art.bytes);
      }
    }
    for lease in &self.retired_artwork {
      charge.shared(std::sync::Arc::as_ptr(&lease.art) as usize, lease.art.bytes);
    }
    charge.bytes()
  }
}
