//! The sounds an app loaded from addresses, kept as they came: a sound asked for again plays at once, and with
//! no network.
//!
//! The app's [`Loader`](crate::resources::Loader) keeps no bytes once who asked has taken them, and a sound is
//! what asks; so a media that is only heard ([`Media::sound`](super::Media::sound)) keeps what it loaded here, by
//! its source's [`cache_key`](crate::resources::Source::cache_key). What is kept is what the server sent (an MP3
//! is kept as an MP3), up to a number of bytes; over it, what was played longest ago is let go. A file and bytes
//! are not kept: they are where they are.

use std::{
  collections::HashMap,
  sync::{Arc, Mutex, MutexGuard, OnceLock},
};

use crate::resources::Source;

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
}

impl SoundCache {
  fn kept(&self) -> MutexGuard<'_, Kept> {
    self.kept.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
  }

  /// How many bytes of sounds are kept at most: 64 MiB unless told. With less than is kept now, what was played
  /// longest ago is let go at once; with none, nothing is kept.
  pub fn budget(&self, bytes: u64) -> &Self {
    let mut kept = self.kept();
    kept.budget = bytes;
    kept.fit();
    drop(kept);
    self
  }

  /// How many bytes are kept now.
  pub fn bytes(&self) -> u64 {
    self.kept().bytes
  }

  /// How many sounds are kept now.
  pub fn len(&self) -> usize {
    self.kept().sounds.len()
  }

  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  /// Whether the sound of `source` is kept.
  pub fn holds(&self, source: &Source) -> bool {
    self.kept().sounds.contains_key(&source.cache_key())
  }

  /// Lets go of the sound of `source`, so that it is loaded again when it is next played. `true` where it was
  /// kept.
  pub fn forget(&self, source: &Source) -> bool {
    let mut kept = self.kept();
    match kept.sounds.remove(&source.cache_key()) {
      Some((bytes, _)) => {
        kept.bytes -= bytes.len() as u64;
        true
      }
      None => false,
    }
  }

  /// Lets go of every sound.
  pub fn clear(&self) {
    let mut kept = self.kept();
    kept.sounds.clear();
    kept.bytes = 0;
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
}
