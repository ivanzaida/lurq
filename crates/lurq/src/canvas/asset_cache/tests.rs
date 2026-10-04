use super::*;

const KIB: usize = 1024;

/// A frame drawing assets `ids` of 1 KiB each, uploading those it lacks.
/// Returns the ids it uploaded and the textures the frame retired.
fn frame(cache: &mut AssetCache<u64>, ids: impl IntoIterator<Item = u64>) -> (Vec<u64>, Vec<u64>, AssetCacheStats) {
  let (mut uploaded, mut retired) = (Vec::new(), Vec::new());
  for id in ids {
    if cache.get(id).is_none() {
      uploaded.push(id);
      cache.insert(id, id, KIB, |texture| retired.push(texture));
    }
    assert_eq!(cache.peek(id), Some(&id));
  }
  let stats = cache.finish_frame(|texture| retired.push(texture));
  retired.sort_unstable();
  (uploaded, retired, stats)
}

#[test]
fn the_budget_is_clamped_and_the_ceiling_is_twice_it() {
  assert_eq!(CanvasAssetBudget::default().bytes(), 64 * KIB * KIB);
  assert_eq!(CanvasAssetBudget::new(usize::MAX).bytes(), CanvasAssetBudget::MAX_BYTES);
  assert_eq!(CanvasAssetBudget::new(usize::MAX).ceiling_bytes(), 2 * 1024 * KIB * KIB);
  assert_eq!(CanvasAssetBudget::new(10).ceiling_bytes(), 20);
}

#[test]
fn small_textures_are_charged_64_kib() {
  let mut cache = AssetCache::new(CanvasAssetBudget::default());
  cache.insert(1, 1, 10, drop);
  cache.insert(2, 2, 100 * KIB, drop);
  assert_eq!(cache.bytes(), 164 * KIB);
}

#[test]
fn a_frame_over_the_ceiling_uploads_its_overflow_once_and_keeps_the_rest() {
  // Four textures of budget, eight of ceiling.
  let mut cache = AssetCache::new(CanvasAssetBudget::new(4 * 64 * KIB));
  let (uploaded, retired, stats) = frame(&mut cache, (0..10).chain(8..10));
  assert_eq!(
    uploaded,
    (0..10).collect::<Vec<_>>(),
    "a repeat within the frame is not uploaded again"
  );
  assert_eq!(retired, [8, 9], "what was not kept lives until the frame ends");
  assert_eq!(
    stats,
    AssetCacheStats {
      evictions: 0,
      uncached: 2,
      uncached_bytes: 2 * 64 * KIB,
      stretch_bytes: 4 * 64 * KIB,
    }
  );
  for _ in 0..3 {
    let (uploaded, retired, stats) = frame(&mut cache, 0..10);
    assert_eq!(uploaded, [8, 9]);
    assert_eq!(retired, [8, 9]);
    assert_eq!((stats.evictions, stats.uncached), (0, 2));
  }
  assert_eq!((cache.len(), cache.bytes()), (8, 8 * 64 * KIB));
}

#[test]
fn a_zero_budget_keeps_nothing_between_frames() {
  let mut cache = AssetCache::new(CanvasAssetBudget::new(0));
  for _ in 0..2 {
    let (uploaded, retired, stats) = frame(&mut cache, [1, 2, 1]);
    assert_eq!((uploaded, retired), (vec![1, 2], vec![1, 2]));
    assert_eq!(stats.uncached, 2);
  }
  assert_eq!(cache.len(), 0);
}

#[test]
fn textures_two_frames_old_make_room_and_are_retired() {
  let mut cache = AssetCache::new(CanvasAssetBudget::new(4 * 64 * KIB));
  frame(&mut cache, 0..4);
  frame(&mut cache, 4..8);
  assert_eq!(cache.bytes(), 8 * 64 * KIB, "the previous frame's textures stay");
  let (uploaded, retired, stats) = frame(&mut cache, 8..10);
  assert_eq!(uploaded, [8, 9]);
  assert_eq!(
    retired,
    [0, 1, 2, 3],
    "only textures neither of the last two frames drew"
  );
  assert_eq!(stats.evictions, 4);
  let (uploaded, retired, _) = frame(&mut cache, 8..10);
  assert!(uploaded.is_empty());
  assert_eq!(retired, [4, 5], "oldest first, back to the budget");
  assert_eq!(cache.bytes(), cache.budget());
}
