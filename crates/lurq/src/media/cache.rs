//! The sounds an app loaded from addresses, kept as they came: a sound asked for again plays at once, and with
//! no network.
//!
//! The app's [`Loader`](crate::resources::Loader) keeps no bytes once who asked has taken them, and a sound is
//! what asks; so a media that is only heard ([`Media::sound`](super::Media::sound)) keeps what it loaded here, by
//! its source's [`cache_key`](crate::resources::Source::cache_key). What is kept is what the server sent (an MP3
//! is kept as an MP3), up to a number of bytes; over it, what was played longest ago is let go. A file and bytes
//! are not kept: they are where they are.
//!
//! This is what an app has that turned nothing on: the sounds are kept in memory, for the run of the app. An app
//! that gave a folder (`App::enable_media_disk_cache`) has them kept there as files by its loader's
//! [`MediaDiskCache`], for its next run too, and a sound plays from its file: its bytes are then not kept in
//! memory as well. [`sound_cache`] answers for both: whether a sound is kept, how much is, and what is forgotten.
//! How much the folder may keep is told to the cache of the folder (`app.loader().media_disk_cache().budget(..)`).

use std::{
  collections::HashMap,
  sync::{Arc, Mutex, MutexGuard, OnceLock},
};

use crate::resources::{MediaDiskCache, Source, WeakMediaDiskCache};

/// How many bytes of sounds are kept unless the app says: 64 MiB.
pub const DEFAULT_SOUND_CACHE_BYTES: u64 = 64 * 1024 * 1024;

struct Kept {
  sounds: HashMap<u64, (Arc<[u8]>, u64)>,
  bytes: u64,
  budget: u64,
  /// Counts each use: what has the lowest was used longest ago.
  turn: u64,
}

impl Kept {
  /// Lets go of what was used longest ago until what is kept is within the budget.
  fn fit(&mut self) {
    while self.bytes > self.budget {
      let Some(oldest) = self
        .sounds
        .iter()
        .min_by_key(|(_, (_, turn))| *turn)
        .map(|(key, _)| *key)
      else {
        return;
      };
      if let Some((bytes, _)) = self.sounds.remove(&oldest) {
        self.bytes -= bytes.len() as u64;
      }
    }
  }
}

/// The sounds kept; see the [module](self). There is one for the process: [`sound_cache`].
pub struct SoundCache {
  kept: Mutex<Kept>,
  /// The caches on the disk of the apps of this process that turned one on: what they keep is kept too.
  disks: Mutex<Vec<WeakMediaDiskCache>>,
}

impl SoundCache {
  fn kept(&self) -> MutexGuard<'_, Kept> {
    self.kept.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
  }

  /// The caches on the disk that are followed, of the apps that are still there.
  fn disks(&self) -> Vec<MediaDiskCache> {
    let mut disks = self.disks.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    disks.retain(|disk| disk.upgrade().is_some());
    disks.iter().filter_map(WeakMediaDiskCache::upgrade).collect()
  }

  /// Follows `disk` where it is on: what it keeps is then answered for here. An app's cache is followed when
  /// the app turns it on, and a loader's when a sound is first opened with it.
  pub(crate) fn follow(&self, disk: &MediaDiskCache) {
    if !disk.is_on() || self.disks().iter().any(|followed| followed.same(disk)) {
      return;
    }
    self
      .disks
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner())
      .push(disk.downgrade());
  }

  /// How many bytes of sounds are kept in memory at most: 64 MiB unless told. With less than is kept now, what
  /// was played longest ago is let go at once; with none, nothing is kept. How much the folder of an app that
  /// turned the cache on the disk on may keep is another number, told to that cache
  /// ([`MediaDiskCache::budget`]).
  pub fn budget(&self, bytes: u64) -> &Self {
    let mut kept = self.kept();
    kept.budget = bytes;
    kept.fit();
    drop(kept);
    self
  }

  /// How many bytes are kept now: in memory, and in the folder of an app that turned the cache on the disk on.
  /// The folder is counted whole: what the app's loader keeps there of anything it loaded from an address.
  pub fn bytes(&self) -> u64 {
    let memory = self.kept().bytes;
    memory + self.disks().iter().map(MediaDiskCache::bytes).sum::<u64>()
  }

  /// How many sounds are kept now: in memory, and the files in the folder of the cache on the disk.
  pub fn len(&self) -> usize {
    let memory = self.kept().sounds.len();
    memory + self.disks().iter().map(MediaDiskCache::len).sum::<usize>()
  }

  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  /// Whether the sound of `source` is kept: in memory, or as a file by the cache on the disk.
  pub fn holds(&self, source: &Source) -> bool {
    let memory = self.kept().sounds.contains_key(&source.cache_key());
    memory || self.disks().iter().any(|disk| disk.holds(source))
  }

  /// Lets go of the sound of `source`, so that it is loaded again when it is next played: from memory, and its
  /// file where the cache on the disk keeps it. `true` where it was kept.
  pub fn forget(&self, source: &Source) -> bool {
    let memory = {
      let mut kept = self.kept();
      match kept.sounds.remove(&source.cache_key()) {
        Some((bytes, _)) => {
          kept.bytes -= bytes.len() as u64;
          true
        }
        None => false,
      }
    };
    let disk = self.disks().iter().filter(|disk| disk.forget(source)).count() > 0;
    memory || disk
  }

  /// Lets go of every sound: in memory, and every file the cache on the disk keeps, which does not tell a sound
  /// from anything else its loader loaded from an address.
  pub fn clear(&self) {
    {
      let mut kept = self.kept();
      kept.sounds.clear();
      kept.bytes = 0;
    }
    for disk in self.disks() {
      disk.clear();
    }
  }

  /// The sound kept under `key`, which is used by this.
  pub(crate) fn get(&self, key: u64) -> Option<Arc<[u8]>> {
    let mut kept = self.kept();
    kept.turn += 1;
    let turn = kept.turn;
    let (bytes, used) = kept.sounds.get_mut(&key)?;
    *used = turn;
    Some(bytes.clone())
  }

  /// Keeps `bytes` under `key`. What is larger than the whole budget is not kept.
  pub(crate) fn put(&self, key: u64, bytes: Arc<[u8]>) {
    let mut kept = self.kept();
    let size = bytes.len() as u64;
    if size > kept.budget {
      return;
    }
    kept.turn += 1;
    let turn = kept.turn;
    if let Some((old, _)) = kept.sounds.insert(key, (bytes, turn)) {
      kept.bytes -= old.len() as u64;
    }
    kept.bytes += size;
    kept.fit();
  }
}

/// The sounds this process keeps.
///
/// ```
/// use lurq::{media::sound_cache, resources::Source};
///
/// // A player of long recordings keeps more of them, and lets go of one it knows has changed.
/// sound_cache().budget(256 * 1024 * 1024);
/// sound_cache().forget(&Source::url("https://example.com/episode-12.mp3"));
/// ```
pub fn sound_cache() -> &'static SoundCache {
  static CACHE: OnceLock<SoundCache> = OnceLock::new();
  CACHE.get_or_init(|| SoundCache {
    kept: Mutex::new(Kept {
      sounds: HashMap::new(),
      bytes: 0,
      budget: DEFAULT_SOUND_CACHE_BYTES,
      turn: 0,
    }),
    disks: Mutex::new(Vec::new()),
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn cache(budget: u64) -> SoundCache {
    SoundCache {
      kept: Mutex::new(Kept {
        sounds: HashMap::new(),
        bytes: 0,
        budget,
        turn: 0,
      }),
      disks: Mutex::new(Vec::new()),
    }
  }

  #[test]
  fn what_was_used_longest_ago_is_let_go_over_the_budget() {
    let cache = cache(10);
    cache.put(1, Arc::from(vec![0_u8; 4]));
    cache.put(2, Arc::from(vec![0_u8; 4]));
    // The first is used again, so the second is the one used longest ago.
    assert!(cache.get(1).is_some());
    cache.put(3, Arc::from(vec![0_u8; 4]));
    assert!(cache.get(1).is_some() && cache.get(2).is_none() && cache.get(3).is_some());
    assert_eq!((cache.len(), cache.bytes()), (2, 8));
  }

  #[test]
  fn what_is_larger_than_the_budget_is_not_kept_and_a_smaller_budget_lets_go_at_once() {
    let cache = cache(10);
    cache.put(1, Arc::from(vec![0_u8; 11]));
    assert!(cache.is_empty());
    cache.put(1, Arc::from(vec![0_u8; 6]));
    cache.put(2, Arc::from(vec![0_u8; 4]));
    cache.budget(5);
    assert!(cache.get(1).is_none() && cache.get(2).is_some());
    assert_eq!(cache.bytes(), 4);
  }

  #[test]
  fn a_sound_kept_again_takes_the_place_of_the_one_it_was() {
    let cache = cache(10);
    cache.put(1, Arc::from(vec![0_u8; 6]));
    cache.put(1, Arc::from(vec![0_u8; 3]));
    assert_eq!((cache.len(), cache.bytes()), (1, 3));
    cache.clear();
    assert_eq!((cache.len(), cache.bytes()), (0, 0));
  }

  #[cfg(not(target_arch = "wasm32"))]
  #[test]
  fn what_the_cache_on_the_disk_of_an_app_keeps_is_answered_for_too() {
    let folder = std::env::temp_dir().join(format!("lurq-sound-cache-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let cache = cache(10);
    let loader = crate::resources::Loader::new();
    // Off, it is not followed: there is nothing of it to answer for.
    cache.follow(loader.media_disk_cache());
    assert!(cache.disks().is_empty());
    loader.enable_media_disk_cache(&folder);
    cache.follow(loader.media_disk_cache());
    cache.follow(loader.media_disk_cache());
    assert_eq!(cache.disks().len(), 1, "followed once");

    let (song, other) = (
      Source::url("https://example.com/song.mp3"),
      Source::url("https://example.com/other.mp3"),
    );
    let disk = loader.media_disk_cache();
    disk.put(song.cache_key(), "https://example.com/song.mp3", &[0; 6]);
    cache.put(other.cache_key(), Arc::from(vec![0_u8; 3]));
    assert!(cache.holds(&song) && cache.holds(&other));
    assert_eq!((cache.len(), cache.bytes()), (2, 9));
    // The budget is of what is kept in memory: the file stays.
    cache.budget(0);
    assert!(cache.holds(&song) && !cache.holds(&other));
    assert!(cache.forget(&song) && !cache.forget(&song));
    assert!(!disk.holds(&song) && std::fs::read_dir(&folder).unwrap().next().is_none());
    disk.put(song.cache_key(), "https://example.com/song.mp3", &[0; 6]);
    cache.clear();
    assert!(disk.is_empty());
    // The app is gone: its cache is followed no more.
    drop(loader);
    assert!(cache.disks().is_empty());
    let _ = std::fs::remove_dir_all(&folder);
  }
}
