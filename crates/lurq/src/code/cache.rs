//! What each tokenizer gave for a code, kept so that a render does not tokenize again what did not change.
//!
//! Kept by the tokenizer and the code: a tokenizer registered in place of another is another, so what the old one
//! gave is never taken for the new one's, and it is forgotten at once. The cache holds a few hundred codes and a few
//! megabytes of them, and lets the one used longest ago go first.

use std::{
  collections::HashMap,
  hash::{DefaultHasher, Hash, Hasher},
  sync::{Arc, LazyLock},
};

use parking_lot::Mutex;

use super::CodeToken;

/// How many codes are kept, of every tokenizer together.
const MOST_CODES: usize = 256;
/// How many bytes of code are kept, of every tokenizer together.
const MOST_BYTES: usize = 32 << 20;

struct Kept {
  code: Arc<str>,
  tokens: Arc<[CodeToken]>,
  used: u64,
}

#[derive(Default)]
struct Cache {
  /// By the tokenizer, its generation and the code's hash.
  kept: HashMap<(u64, u64, u64), Kept>,
  bytes: usize,
  clock: u64,
}

static CACHE: LazyLock<Mutex<Cache>> = LazyLock::new(Default::default);

fn hash(code: &str) -> u64 {
  let mut hasher = DefaultHasher::new();
  code.hash(&mut hasher);
  hasher.finish()
}

/// What the tokenizer `id` gave for `code`, or what `tokenize` gives, kept. The tokenizer is asked with the cache
/// let go.
pub(super) fn tokens(
  id: u64,
  generation: u64,
  code: &str,
  tokenize: impl FnOnce() -> Vec<CodeToken>,
) -> Arc<[CodeToken]> {
  let key = (id, generation, hash(code));
  {
    let mut cache = CACHE.lock();
    cache.clock += 1;
    let clock = cache.clock;
    if let Some(kept) = cache.kept.get_mut(&key)
      && &*kept.code == code
    {
      kept.used = clock;
      return kept.tokens.clone();
    }
  }
  let tokens: Arc<[CodeToken]> = tokenize().into();
  let mut cache = CACHE.lock();
  let used = cache.clock;
  if let Some(old) = cache.kept.insert(
    key,
    Kept {
      code: Arc::from(code),
      tokens: tokens.clone(),
      used,
    },
  ) {
    cache.bytes -= old.code.len();
  }
  cache.bytes += code.len();
  while cache.kept.len() > MOST_CODES || (cache.bytes > MOST_BYTES && cache.kept.len() > 1) {
    let Some(oldest) = cache.kept.iter().min_by_key(|(_, kept)| kept.used).map(|(key, _)| *key) else {
      break;
    };
    if let Some(gone) = cache.kept.remove(&oldest) {
      cache.bytes -= gone.code.len();
    }
  }
  tokens
}

/// Forgets what the tokenizer `id` gave: it is no language's now.
pub(super) fn forget(id: u64) {
  let mut cache = CACHE.lock();
  let mut freed = 0;
  cache.kept.retain(|key, kept| {
    let keep = key.0 != id;
    if !keep {
      freed += kept.code.len();
    }
    keep
  });
  cache.bytes -= freed;
}
