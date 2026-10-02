//! Content-free DX12 asset resource work, aggregated across one Canvas encode.
#![cfg_attr(
  not(all(
    feature = "canvas",
    feature = "dx12",
    target_os = "windows",
    feature = "perf_profile"
  )),
  allow(dead_code)
)]

use std::time::Duration;
#[cfg(feature = "perf_profile")]
use std::time::Instant;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CanvasAssetUploadProfile {
  /// These four disjoint CPU subscopes are nested in CanvasProfile::asset_upload.
  pub texture_creation: Duration,
  pub pixel_packing: Duration,
  pub upload_staging_commands: Duration,
  pub descriptor_writes: Duration,
  /// Budget scan/removal/retirement after drawing; outside asset_upload.
  pub cache_eviction: Duration,
  /// One cache lookup per prepared draw carrying an asset, including repeats.
  pub cache_hits: usize,
  pub cache_misses: usize,
  pub texture_creations: usize,
  /// Asset SRV pairs only; excludes backing/layer/composite descriptors.
  pub descriptor_pairs: usize,
  /// Row-padded CPU payload copied to staging; excludes placement alignment gaps.
  pub padded_upload_bytes: usize,
  pub arena_uploads: usize,
  pub dedicated_uploads: usize,
  pub cache_evictions: usize,
  /// Existing cache accounting (max(payload, 64 KiB)); not actual GPU/RSS bytes.
  pub cache_charged_bytes_before: usize,
  pub cache_charged_bytes_peak: usize,
  pub cache_charged_bytes_after: usize,
  pub cache_entries_before: usize,
  pub cache_entries_after: usize,
}

impl CanvasAssetUploadProfile {
  pub(crate) fn capture(active: bool, charged_bytes: usize, entries: usize) -> Option<Self> {
    #[cfg(feature = "perf_profile")]
    {
      active.then_some(Self {
        cache_charged_bytes_before: charged_bytes,
        cache_charged_bytes_peak: charged_bytes,
        cache_charged_bytes_after: charged_bytes,
        cache_entries_before: entries,
        cache_entries_after: entries,
        ..Self::default()
      })
    }
    #[cfg(not(feature = "perf_profile"))]
    {
      let _ = (active, charged_bytes, entries);
      None
    }
  }

  #[cfg(feature = "perf_profile")]
  pub(crate) fn cache_state(&mut self, charged_bytes: usize, entries: usize) {
    self.cache_charged_bytes_peak = self.cache_charged_bytes_peak.max(charged_bytes);
    self.cache_charged_bytes_after = charged_bytes;
    self.cache_entries_after = entries;
  }

  #[cfg(feature = "perf_profile")]
  pub(crate) fn uploaded(&mut self, padded_bytes: usize, dedicated: bool) {
    self.padded_upload_bytes += padded_bytes;
    if dedicated {
      self.dedicated_uploads += 1;
    } else {
      self.arena_uploads += 1;
    }
  }

  /// No clock read for uncaptured encodes; no allocation or collector lock.
  #[cfg(feature = "perf_profile")]
  pub(crate) fn start_timer(profile: Option<&Self>) -> Option<Instant> {
    profile.map(|_| Instant::now())
  }

  #[cfg(feature = "perf_profile")]
  pub(crate) fn add_stage(&mut self, stage: AssetUploadStage, started: Option<Instant>) {
    if let Some(started) = started {
      self.add_duration(stage, started.elapsed());
    }
  }

  #[cfg(feature = "perf_profile")]
  pub(crate) fn add_duration(&mut self, stage: AssetUploadStage, elapsed: Duration) {
    *match stage {
      AssetUploadStage::TextureCreation => &mut self.texture_creation,
      AssetUploadStage::PixelPacking => &mut self.pixel_packing,
      AssetUploadStage::UploadStagingCommands => &mut self.upload_staging_commands,
      AssetUploadStage::DescriptorWrites => &mut self.descriptor_writes,
      AssetUploadStage::CacheEviction => &mut self.cache_eviction,
    } += elapsed;
  }
}

#[cfg(feature = "perf_profile")]
pub(crate) enum AssetUploadStage {
  TextureCreation,
  PixelPacking,
  UploadStagingCommands,
  DescriptorWrites,
  CacheEviction,
}
