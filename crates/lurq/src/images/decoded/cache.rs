//! The cache of decoded images: one entry per content key and size cap,
//! loading, ready or failed, within a byte budget.

use std::{
  collections::HashMap,
  sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
  },
  time::{Duration, Instant},
};

use parking_lot::{Condvar, Mutex};

use super::{ImageDecodeError, ImageKey, ImageLoadState, ImageSizeCap, ImageSource, decode_image, worker::Workers};
use crate::{app::window::WindowWaker, images::ImageData};

/// Decoded RGBA the cache keeps at most, before the least recently used
/// ready images are dropped from it (an image a node still shows is kept by
/// that node).
pub const DEFAULT_BUDGET_BYTES: usize = 128 * 1024 * 1024;

const WORKER_COUNT: usize = 2;

type CacheKey = (ImageKey, Option<ImageSizeCap>);

struct Entry {
  state: ImageLoadState,
  /// The RGBA bytes of a `Ready` state; 0 otherwise. `Inner::used` is their
  /// sum over the entries.
  bytes: usize,
  last_used: u64,
  /// Windows to wake when the decode ends, one per window.
  wakers: Vec<WindowWaker>,
}

struct Inner {
  entries: HashMap<CacheKey, Entry>,
  /// Decode jobs submitted and not finished: each job calls `finish` once.
  pending: usize,
  budget: usize,
  used: usize,
  clock: u64,
}

/// A decode that must end its entry even when the decoder panics: the entry
/// would stay `Loading` forever, with `pending` never counted down.
struct FinishOnDrop {
  shared: Arc<Shared>,
  cache_key: CacheKey,
  result: Option<Result<ImageData, ImageDecodeError>>,
}

impl Drop for FinishOnDrop {
  fn drop(&mut self) {
    let result = self
      .result
      .take()
      .unwrap_or_else(|| Err(ImageDecodeError::new("the image decoder panicked")));
    self.shared.finish(self.cache_key, result);
  }
}

/// A cache of decoded images with its own decode workers. The runtime uses
/// the process-wide one, [`decoded_images`](super::decoded_images); a test
/// may make one of its own.
pub struct DecodedImages {
  shared: Arc<Shared>,
}

struct Shared {
  inner: Mutex<Inner>,
  idle: Condvar,
  workers: Workers,
  /// Counts finished decodes, so a tree can tell that something it waits
  /// for ended.
  completed: AtomicU64,
}

impl Default for DecodedImages {
  fn default() -> Self {
    Self::new()
  }
}

impl DecodedImages {
  /// An empty cache with the default budget and its own worker threads.
  pub fn new() -> Self {
    Self {
      shared: Arc::new(Shared {
        inner: Mutex::new(Inner {
          entries: HashMap::new(),
          pending: 0,
          budget: DEFAULT_BUDGET_BYTES,
          used: 0,
          clock: 0,
        }),
        idle: Condvar::new(),
        workers: Workers::new(WORKER_COUNT),
        completed: AtomicU64::new(0),
      }),
    }
  }

  /// The image under `key` decoded within `cap`: its state now. An unknown
  /// key starts a decode of `source()` on a worker and is `Loading`; `wake`
  /// is called when that decode ends, so the window draws the image. A
  /// second request for the same key joins the first.
  pub fn load(
    &self,
    key: ImageKey,
    cap: Option<ImageSizeCap>,
    source: impl FnOnce() -> ImageSource,
    wake: Option<WindowWaker>,
  ) -> ImageLoadState {
    self.load_with(key, cap, wake, || {
      let source = source();
      move || decode_image(&source, cap)
    })
  }

  /// [`load`](Self::load) with the decode given as a job: `decode()` builds
  /// it on the caller's thread (taking the source), the job runs on a worker.
  pub(super) fn load_with<J>(
    &self,
    key: ImageKey,
    cap: Option<ImageSizeCap>,
    wake: Option<WindowWaker>,
    decode: impl FnOnce() -> J,
  ) -> ImageLoadState
  where
    J: FnOnce() -> Result<ImageData, ImageDecodeError> + Send + 'static,
  {
    let cache_key = (key, cap);
    let mut inner = self.shared.inner.lock();
    inner.clock += 1;
    let clock = inner.clock;
    if let Some(entry) = inner.entries.get_mut(&cache_key) {
      entry.last_used = clock;
      if let (ImageLoadState::Loading, Some(wake)) = (&entry.state, wake)
        && !entry.wakers.iter().any(|known| Arc::ptr_eq(known, &wake))
      {
        entry.wakers.push(wake);
      }
      return entry.state.clone();
    }
    inner.entries.insert(
      cache_key,
      Entry {
        state: ImageLoadState::Loading,
        bytes: 0,
        last_used: clock,
        wakers: wake.into_iter().collect(),
      },
    );
    inner.pending += 1;
    drop(inner);
    let job = decode();
    let mut finish = FinishOnDrop {
      shared: Arc::clone(&self.shared),
      cache_key,
      result: None,
    };
    let submitted = self.shared.workers.submit(move || {
      finish.result = Some(job());
      drop(finish);
    });
    if !submitted {
      self.shared.finish(
        cache_key,
        Err(ImageDecodeError::new("no image decode worker is running")),
      );
    }
    ImageLoadState::Loading
  }

  /// The state of the image under `key` and `cap`, without starting a decode.
  pub fn state(&self, key: ImageKey, cap: Option<ImageSizeCap>) -> Option<ImageLoadState> {
    self
      .shared
      .inner
      .lock()
      .entries
      .get(&(key, cap))
      .map(|entry| entry.state.clone())
  }

  /// Puts an image the app decoded itself under `key`, so nodes that ask for
  /// it find it ready. A decode of the same key still running finishes
  /// without replacing it; windows waiting for that decode are woken now.
  pub fn insert(&self, key: ImageKey, cap: Option<ImageSizeCap>, data: ImageData) {
    let bytes = rgba_bytes(&data);
    let wakers = {
      let mut inner = self.shared.inner.lock();
      inner.clock += 1;
      let last_used = inner.clock;
      let mut wakers = Vec::new();
      if let Some(old) = inner.entries.insert(
        (key, cap),
        Entry {
          state: ImageLoadState::Ready(data),
          bytes,
          last_used,
          wakers: Vec::new(),
        },
      ) {
        inner.used -= old.bytes;
        wakers = old.wakers;
      }
      inner.used += bytes;
      evict(&mut inner);
      wakers
    };
    for wake in wakers {
      wake();
    }
  }

  /// Decodes still running.
  pub fn pending(&self) -> usize {
    self.shared.inner.lock().pending
  }

  /// How many decodes have ended since the cache was made.
  pub fn completed(&self) -> u64 {
    self.shared.completed.load(Ordering::Acquire)
  }

  /// Blocks until no decode is pending, or `timeout` passes (`false`).
  pub fn wait_idle(&self, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    let mut inner = self.shared.inner.lock();
    while inner.pending > 0 {
      if self.shared.idle.wait_until(&mut inner, deadline).timed_out() {
        return inner.pending == 0;
      }
    }
    true
  }

  /// The decoded RGBA kept at most; see [`DEFAULT_BUDGET_BYTES`].
  pub fn budget(&self) -> usize {
    self.shared.inner.lock().budget
  }

  pub fn set_budget(&self, bytes: usize) {
    let mut inner = self.shared.inner.lock();
    inner.budget = bytes;
    evict(&mut inner);
  }

  /// The decoded RGBA the cache holds now.
  pub fn used_bytes(&self) -> usize {
    self.shared.inner.lock().used
  }

  /// Drops every ready and failed entry; running decodes finish and stay.
  pub fn clear(&self) {
    let mut inner = self.shared.inner.lock();
    inner.entries.retain(|_, entry| entry.state.is_loading());
    inner.used = 0;
  }
}

impl Shared {
  /// Ends one decode job: its entry takes the result while it still waits
  /// for it. An entry that `insert` filled meanwhile, or one evicted or
  /// cleared, keeps what it has and the result is dropped. Counts the job
  /// down either way, once its windows are woken, so `wait_idle` returns
  /// after the wakes.
  fn finish(&self, cache_key: CacheKey, result: Result<ImageData, ImageDecodeError>) {
    let wakers = {
      let mut inner = self.inner.lock();
      match inner.entries.get_mut(&cache_key) {
        Some(entry) if entry.state.is_loading() => {
          let bytes = result.as_ref().map(rgba_bytes).unwrap_or(0);
          entry.bytes = bytes;
          entry.state = match result {
            Ok(data) => ImageLoadState::Ready(data),
            Err(error) => ImageLoadState::Failed(Arc::new(error)),
          };
          inner.used += bytes;
          evict(&mut inner);
          match inner.entries.get_mut(&cache_key) {
            Some(entry) => std::mem::take(&mut entry.wakers),
            None => Vec::new(),
          }
        }
        _ => Vec::new(),
      }
    };
    for wake in wakers {
      wake();
    }
    let mut inner = self.inner.lock();
    inner.pending = inner.pending.saturating_sub(1);
    self.completed.fetch_add(1, Ordering::AcqRel);
    self.idle.notify_all();
  }
}

/// Drops the least recently used ready images until the budget holds.
fn evict(inner: &mut Inner) {
  while inner.used > inner.budget {
    let victim = inner
      .entries
      .iter()
      .filter(|(_, entry)| matches!(entry.state, ImageLoadState::Ready(_)))
      .min_by_key(|(_, entry)| entry.last_used)
      .map(|(key, _)| *key);
    let Some(victim) = victim else {
      return;
    };
    if let Some(entry) = inner.entries.remove(&victim) {
      inner.used -= entry.bytes;
    }
  }
}

fn rgba_bytes(data: &ImageData) -> usize {
  (data.width() as usize) * (data.height() as usize) * 4
}
