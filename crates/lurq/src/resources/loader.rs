//! The app's loader: what reads a [`Source`], for everything that loads.
//!
//! A picture asks it for all the bytes of its source ([`Loader::load`]) and is told how far the load has come,
//! when it ended and why it failed; a video or a sound asks it for a [`Reader`] ([`Loader::open`]) and reads the
//! parts it needs. The loader holds the app's resource root and what the network may do, reads on threads of its
//! own, makes one load of the same source asked for twice, and lets go of a load nobody waits for any more.
//! Where the app gave its tokio runtime (`App::with_tokio_handle`, feature `tokio`), it reads on that and starts
//! no threads of its own.
//!
//! It keeps no bytes in memory: once what asked has taken them they are let go. What is decoded from them is kept
//! by who decodes (the pixels of a picture in [`decoded_images`](crate::images::decoded_images)). What came from
//! an address is kept on the disk where the app gave a folder for it ([`Loader::enable_media_disk_cache`]), and
//! is then read from the folder and not from the network; see [`MediaDiskCache`].

use std::{
  collections::HashMap,
  fmt, io,
  path::PathBuf,
  sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
  },
  time::Duration,
};

use parking_lot::{Mutex, RwLock};

use super::{
  Source,
  disk_cache::MediaDiskCache,
  reader::{BytesReader, FileReader, Reader, read_all},
  source::SourceKind,
  thread_pool::ThreadPool,
};
use crate::time::Instant;

/// How long a load of an address may take before it fails, unless the app says.
pub const DEFAULT_LOAD_TIMEOUT: Duration = Duration::from_secs(30);
/// The most bytes [`Loader::load`] reads of one source, unless the app says: 64 MiB. A reader has no such limit.
pub const DEFAULT_MAX_LOAD_BYTES: u64 = 64 * 1024 * 1024;
/// The threads that read: how many loads are read at a time, on the loader's own threads or on the app's
/// runtime.
const LOADER_THREADS: usize = 4;
/// How long a load nobody holds is kept before it is stopped: what holds it is built again by every render, and
/// lets go of it for a moment each time.
const IDLE_GRACE: Duration = Duration::from_millis(250);
/// A load wakes who waits for it once it has come this much further.
const PROGRESS_STEP_BYTES: u64 = 256 * 1024;

/// Called when a load ends or comes further: a window's loop is woken, so that what waits asks again.
pub type LoadWaker = Arc<dyn Fn() + Send + Sync>;

/// Why a load failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadError {
  /// There is no such file, or the server has none at the address.
  NotFound,
  /// The app's loader does not allow the address ([`Loader::allow`]).
  Denied,
  /// The server answered with this status.
  Status(u16),
  /// The source is larger than the loader reads whole ([`Loader::max_bytes`]).
  TooLarge { limit: u64 },
  /// The load took longer than the loader waits ([`Loader::timeout`]).
  TimedOut,
  /// Nobody waited for it any more.
  Cancelled,
  /// This build cannot load such a source, and why: an address in a build without the feature `http`.
  Unsupported(&'static str),
  /// Reading failed, as the system or the connection said.
  Io(String),
}

impl fmt::Display for LoadError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::NotFound => f.write_str("it was not found"),
      Self::Denied => f.write_str("the app does not allow this address"),
      Self::Status(status) => write!(f, "the server answered {status}"),
      Self::TooLarge { limit } => write!(f, "it is larger than {limit} bytes"),
      Self::TimedOut => f.write_str("it took too long"),
      Self::Cancelled => f.write_str("the load was stopped"),
      Self::Unsupported(why) => f.write_str(why),
      Self::Io(reason) => f.write_str(reason),
    }
  }
}

impl std::error::Error for LoadError {}

impl From<io::Error> for LoadError {
  fn from(error: io::Error) -> Self {
    match error.kind() {
      io::ErrorKind::NotFound => Self::NotFound,
      io::ErrorKind::TimedOut => Self::TimedOut,
      io::ErrorKind::Interrupted => Self::Cancelled,
      _ => Self::Io(error.to_string()),
    }
  }
}

/// Where a load is.
#[derive(Clone, Debug)]
pub enum LoadState {
  /// On its way: the bytes read so far, and how many there are where the source says.
  Loading {
    loaded: u64,
    total: Option<u64>,
  },
  Ready(Arc<[u8]>),
  Failed(LoadError),
}

impl LoadState {
  pub fn is_loading(&self) -> bool {
    matches!(self, Self::Loading { .. })
  }
}

struct Entry {
  state: Mutex<LoadState>,
  /// Nobody waits for it: what reads it stops at its next step.
  cancelled: AtomicBool,
  /// How many [`Load`]s hold it.
  holders: AtomicUsize,
  /// Since when nobody holds it.
  idle_since: Mutex<Option<Instant>>,
  wakers: Mutex<Vec<LoadWaker>>,
  /// The bytes its wakers were last told of.
  told: AtomicU64,
}

impl Entry {
  fn new(state: LoadState) -> Arc<Self> {
    Arc::new(Self {
      state: Mutex::new(state),
      cancelled: AtomicBool::new(false),
      holders: AtomicUsize::new(0),
      idle_since: Mutex::new(Some(Instant::now())),
      wakers: Mutex::new(Vec::new()),
      told: AtomicU64::new(0),
    })
  }

  fn wake(&self) {
    // Taken out of the lock: a waker may ask the load's state.
    let wakers = self.wakers.lock().clone();
    for wake in wakers {
      wake();
    }
  }

  /// The load came to `loaded` of `total`. Answers whether it is to go on: `false` once nobody waits for it.
  fn progress(&self, loaded: u64, total: Option<u64>) -> bool {
    *self.state.lock() = LoadState::Loading { loaded, total };
    let step = total.map_or(PROGRESS_STEP_BYTES, |total| (total / 100).max(16 * 1024));
    if loaded.saturating_sub(self.told.load(Ordering::Relaxed)) >= step {
      self.told.store(loaded, Ordering::Relaxed);
      self.wake();
    }
    !self.cancelled.load(Ordering::Acquire)
  }

  fn finish(&self, result: Result<Vec<u8>, LoadError>) {
    *self.state.lock() = match result {
      Ok(bytes) => LoadState::Ready(Arc::from(bytes)),
      Err(error) => LoadState::Failed(error),
    };
    self.wake();
  }
}

/// Ends a load whose read was dropped before it ended: a runtime that stopped drops what it had not started.
struct Unread(Arc<Entry>);

impl Drop for Unread {
  fn drop(&mut self) {
    if self.0.state.lock().is_loading() {
      self.0.finish(Err(LoadError::Cancelled));
    }
  }
}

/// A load of all the bytes of a source, held by who asked for it. The same source asked for again while it loads
/// is the same load. Dropped by everyone that held it, a load still on its way is stopped a moment later.
pub struct Load {
  entry: Arc<Entry>,
}

impl Load {
  fn of(entry: &Arc<Entry>) -> Self {
    entry.holders.fetch_add(1, Ordering::AcqRel);
    *entry.idle_since.lock() = None;
    Self { entry: entry.clone() }
  }

  /// Where the load is now.
  pub fn state(&self) -> LoadState {
    self.entry.state.lock().clone()
  }

  /// Calls `wake` when the load ends, and as it comes further. The same waker is kept once.
  pub fn wake_with(&self, wake: LoadWaker) {
    let mut wakers = self.entry.wakers.lock();
    if !wakers.iter().any(|known| Arc::ptr_eq(known, &wake)) {
      wakers.push(wake);
    }
  }
}

impl Clone for Load {
  fn clone(&self) -> Self {
    Self::of(&self.entry)
  }
}

impl Drop for Load {
  fn drop(&mut self) {
    if self.entry.holders.fetch_sub(1, Ordering::AcqRel) == 1 {
      *self.entry.idle_since.lock() = Some(Instant::now());
    }
  }
}

type Allow = Arc<dyn Fn(&str) -> bool + Send + Sync>;

struct Settings {
  root: Option<PathBuf>,
  timeout: Duration,
  max_bytes: u64,
  allow: Option<Allow>,
}

struct Shared {
  settings: RwLock<Settings>,
  entries: Mutex<HashMap<u64, Arc<Entry>>>,
  /// Made by the first load that reads: an app that loads nothing has no threads for it.
  pool: OnceLock<ThreadPool>,
  /// What was loaded from addresses, kept in the folder the app gave: off until it gives one.
  disk: MediaDiskCache,
  /// The app's tokio runtime, where it gave one: loads are read on it, and the pool is not made.
  #[cfg(feature = "tokio")]
  runtime: Mutex<Option<tokio::runtime::Handle>>,
  /// The room for reads on that runtime: as many at a time as the pool has threads.
  #[cfg(feature = "tokio")]
  room: Arc<tokio::sync::Semaphore>,
}

impl Drop for Shared {
  fn drop(&mut self) {
    // The pool waits for its threads as it is dropped: what they read is told to stop first.
    for entry in self.entries.lock().values() {
      entry.cancelled.store(true, Ordering::Release);
    }
  }
}

/// The app's loader; see the [module](self). A clone is the same loader.
///
/// ```
/// use std::time::Duration;
///
/// use lurq::resources::{LoadState, Loader, Source};
///
/// let loader = Loader::new();
/// loader
///   .root("assets")
///   .timeout(Duration::from_secs(10))
///   .allow(|address| address.starts_with("https://cdn.example.com/"));
///
/// let load = loader.load(&Source::bytes(vec![1, 2, 3]));
/// assert!(matches!(load.state(), LoadState::Ready(bytes) if bytes.len() == 3));
/// ```
#[derive(Clone)]
pub struct Loader {
  shared: Arc<Shared>,
}

impl Default for Loader {
  fn default() -> Self {
    Self::new()
  }
}

impl Loader {
  pub fn new() -> Self {
    Self {
      shared: Arc::new(Shared {
        settings: RwLock::new(Settings {
          root: None,
          timeout: DEFAULT_LOAD_TIMEOUT,
          max_bytes: DEFAULT_MAX_LOAD_BYTES,
          allow: None,
        }),
        entries: Mutex::new(HashMap::new()),
        pool: OnceLock::new(),
        disk: MediaDiskCache::new(),
        #[cfg(feature = "tokio")]
        runtime: Mutex::new(None),
        #[cfg(feature = "tokio")]
        room: Arc::new(tokio::sync::Semaphore::new(LOADER_THREADS)),
      }),
    }
  }

  /// The folder the app's resources are under: [`Source::resource`] is a path in it. Without one a resource is
  /// a path from where the app runs.
  pub fn root(&self, root: impl Into<PathBuf>) -> &Self {
    self.shared.settings.write().root = Some(root.into());
    self
  }

  /// Takes the resource root away.
  pub fn reset_root(&self) -> &Self {
    self.shared.settings.write().root = None;
    self
  }

  /// How long a load of an address may take: 30 seconds unless told.
  pub fn timeout(&self, timeout: Duration) -> &Self {
    self.shared.settings.write().timeout = timeout;
    self
  }

  /// The most bytes [`load`](Self::load) reads of one source: 64 MiB unless told. A larger one fails with
  /// [`LoadError::TooLarge`], and is not read into memory. A [`Reader`] has no such limit.
  pub fn max_bytes(&self, max_bytes: u64) -> &Self {
    self.shared.settings.write().max_bytes = max_bytes;
    self
  }

  /// Which addresses may be loaded: every one the app asks for, unless told. An address `allow` refuses fails
  /// with [`LoadError::Denied`] and nothing is sent. It is what keeps a text from a stranger, shown by the app,
  /// from fetching what the stranger chose.
  pub fn allow(&self, allow: impl Fn(&str) -> bool + Send + Sync + 'static) -> &Self {
    self.shared.settings.write().allow = Some(Arc::new(allow));
    self
  }

  /// Reads on the tokio runtime `handle` is of, and not on threads of the loader's own: a load is a blocking
  /// task of the runtime (`spawn_blocking`), four of them at a time, as the loader's own threads would read. A
  /// loader that has loaded nothing yet starts no threads at all. A load that was on its way when the handle
  /// was given ends on the thread that reads it; the ones asked for after it are read on the runtime. What a
  /// load does is the same on both: how far it came, its timeout, its limit, how it is stopped, who it wakes.
  ///
  /// The runtime is to run for as long as the loader is asked to load: a load given to one that has stopped
  /// fails with [`LoadError::Cancelled`]. `App::with_tokio_handle` and `App::set_tokio_handle` tell the app's
  /// loader; this is for a loader made by itself.
  #[cfg(feature = "tokio")]
  pub fn tokio_handle(&self, handle: tokio::runtime::Handle) -> &Self {
    *self.shared.runtime.lock() = Some(handle);
    self
  }

  /// Takes the runtime away: what is asked for from now on is read on threads of the loader's own.
  #[cfg(feature = "tokio")]
  pub fn reset_tokio_handle(&self) -> &Self {
    *self.shared.runtime.lock() = None;
    self
  }

  /// Whether the loader was given a runtime to read on.
  #[cfg(all(test, feature = "tokio"))]
  pub(crate) fn reads_on_a_runtime(&self) -> bool {
    self.shared.runtime.lock().is_some()
  }

  /// Turns on the cache on the disk, in `folder`: what is loaded from an address is kept there as a file, is
  /// there for the next run of the app, and is read from the folder and not from the network when it is asked
  /// for again. Not called, nothing is written to the disk. The folder is the app's to choose: a cache folder
  /// of its own, which the system may empty. `App::enable_media_disk_cache` is this, said where the app is
  /// made.
  ///
  /// ```no_run
  /// use lurq::resources::Loader;
  ///
  /// let loader = Loader::new();
  /// loader.enable_media_disk_cache("cache");
  /// loader.media_disk_cache().budget(1024 * 1024 * 1024);
  /// ```
  pub fn enable_media_disk_cache(&self, folder: impl Into<PathBuf>) -> &Self {
    self.shared.disk.turn_on(folder.into());
    self
  }

  /// The cache on the disk: how much it keeps and may keep, and what it is told to forget. It is off until
  /// [`enable_media_disk_cache`](Self::enable_media_disk_cache) gives it a folder.
  pub fn media_disk_cache(&self) -> &MediaDiskCache {
    &self.shared.disk
  }

  /// Whether `address` may be loaded: what [`allow`](Self::allow) says of it, and yes where nothing was said.
  /// For what reads an address by itself and not through a load: a video.
  #[cfg_attr(not(feature = "media"), allow(dead_code))]
  pub(crate) fn allows(&self, address: &str) -> bool {
    let allow = self.shared.settings.read().allow.clone();
    allow.is_none_or(|allow| allow(address))
  }

  /// The file a resource is: its path under the root.
  pub(crate) fn resource_path(&self, resource: &str) -> PathBuf {
    match &self.shared.settings.read().root {
      Some(root) => root.join(resource),
      None => PathBuf::from(resource),
    }
  }

  /// Loads all the bytes of `source`, off the caller's thread. The source's
  /// [`cache_key`](Source::cache_key) tells it from another: a load of the same key that is on its way, or that
  /// failed, is joined. A failed one stays failed until it is [forgotten](Self::forget); a finished one is let
  /// go once nobody holds it, and asked for again it is loaded again: from the file the
  /// [cache on the disk](Self::media_disk_cache) keeps of an address, where it is on.
  pub fn load(&self, source: &Source) -> Load {
    let key = source.cache_key();
    let mut entries = self.shared.entries.lock();
    if let Some(entry) = entries.get(&key) {
      return Load::of(entry);
    }
    let entry = match &source.kind {
      SourceKind::Bytes(bytes) => Entry::new(LoadState::Ready(bytes.clone())),
      _ => Entry::new(LoadState::Loading { loaded: 0, total: None }),
    };
    entries.insert(key, entry.clone());
    drop(entries);
    let load = Load::of(&entry);
    if !matches!(source.kind, SourceKind::Bytes(_)) {
      self.start(source, entry);
    }
    load
  }

  /// Starts reading `source` into `entry`.
  fn start(&self, source: &Source, entry: Arc<Entry>) {
    #[cfg(target_arch = "wasm32")]
    {
      let _ = source;
      entry.finish(Err(LoadError::Unsupported(
        "a file or an address is not loaded in a browser yet",
      )));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
      let (timeout, max_bytes, allow) = {
        let settings = self.shared.settings.read();
        (settings.timeout, settings.max_bytes, settings.allow.clone())
      };
      let path = match &source.kind {
        SourceKind::File(path) => Some(path.clone()),
        SourceKind::Resource(resource) => Some(self.resource_path(resource)),
        _ => None,
      };
      if let SourceKind::Url(address) = &source.kind
        && allow.as_ref().is_some_and(|allow| !allow(address))
      {
        entry.finish(Err(LoadError::Denied));
        return;
      }
      let source = source.clone();
      let disk = self.shared.disk.clone();
      let unread = Unread(entry.clone());
      let read = move || {
        let _unread = unread;
        let told = |loaded, total| entry.progress(loaded, total);
        let result = match (path, &source.kind) {
          (Some(path), _) => FileReader::open(&path)
            .and_then(|reader| read_all(&reader, Some(max_bytes), told))
            .map_err(|error| match error.kind() {
              io::ErrorKind::FileTooLarge => LoadError::TooLarge { limit: max_bytes },
              _ => LoadError::from(error),
            }),
          // What the cache on the disk keeps of the address is read from there; what is fetched is kept.
          (None, SourceKind::Url(address)) => match disk.read(source.cache_key(), max_bytes) {
            Some(bytes) => Ok(bytes),
            None => fetch(address, &source, timeout, max_bytes, told).inspect(|bytes| {
              disk.put(source.cache_key(), address, bytes);
            }),
          },
          (None, _) => Err(LoadError::Unsupported("nothing reads this source")),
        };
        entry.finish(result);
      };
      // On the app's runtime where it gave one: a blocking task of it, once there is room for one more read.
      #[cfg(feature = "tokio")]
      if let Some(handle) = self.shared.runtime.lock().clone() {
        let room = self.shared.room.clone();
        drop(handle.spawn(async move {
          if let Ok(_turn) = room.acquire().await {
            let _ = tokio::task::spawn_blocking(read).await;
          }
        }));
        return;
      }
      let pool = self.shared.pool.get_or_init(|| ThreadPool::new(LOADER_THREADS));
      pool.execute(read);
    }
  }

  /// A reader of `source`, for what reads a part of it and not all: a video, a sound. A file, a resource and
  /// bytes are read where they are. A reader of an address is not built yet: it fails with
  /// [`LoadError::Unsupported`].
  pub fn open(&self, source: &Source) -> Result<Arc<dyn Reader>, LoadError> {
    match &source.kind {
      SourceKind::Bytes(bytes) => Ok(Arc::new(BytesReader::new(bytes.clone()))),
      SourceKind::File(path) => Ok(Arc::new(FileReader::open(path)?)),
      SourceKind::Resource(resource) => Ok(Arc::new(FileReader::open(self.resource_path(resource))?)),
      SourceKind::Url(_) => Err(LoadError::Unsupported(
        "a reader of an address is not built yet: only a whole load of one",
      )),
    }
  }

  /// Forgets the load of `source` that ended, so that it is loaded again when it is next asked for: the way to
  /// try again one that failed. The file the [cache on the disk](Self::media_disk_cache) keeps of it is deleted,
  /// so that an address is asked of the network again: the way to load anew what has changed there. `true`
  /// where there was a load or a file to forget. A load still on its way is left to end.
  pub fn forget(&self, source: &Source) -> bool {
    let key = source.cache_key();
    let kept = self.shared.disk.forget_key(key);
    let mut entries = self.shared.entries.lock();
    match entries.get(&key) {
      Some(entry) if !entry.state.lock().is_loading() => {
        entries.remove(&key);
        true
      }
      _ => kept,
    }
  }

  /// Lets go of what nobody has held for a moment: the bytes of a load that ended, and a load still on its way,
  /// which is told to stop. A load that failed is kept, so that what asks for it again is told so and not sent
  /// to the network again. The app's windows call this as they are drawn.
  pub fn sweep(&self) {
    let now = Instant::now();
    self.shared.entries.lock().retain(|_, entry| {
      if entry.holders.load(Ordering::Acquire) > 0 {
        return true;
      }
      let idle = entry
        .idle_since
        .lock()
        .is_some_and(|since| now.duration_since(since) >= IDLE_GRACE);
      match &*entry.state.lock() {
        LoadState::Failed(_) => true,
        LoadState::Ready(_) => !idle,
        LoadState::Loading { .. } => {
          if idle {
            entry.cancelled.store(true, Ordering::Release);
          }
          !idle
        }
      }
    });
  }

  /// How many loads are on their way.
  pub fn pending(&self) -> usize {
    self
      .shared
      .entries
      .lock()
      .values()
      .filter(|entry| entry.state.lock().is_loading())
      .count()
  }
}

/// Reads all of what is at `address`.
#[cfg(all(feature = "http", not(target_arch = "wasm32")))]
fn fetch(
  address: &str,
  source: &Source,
  timeout: Duration,
  max_bytes: u64,
  told: impl FnMut(u64, Option<u64>) -> bool,
) -> Result<Vec<u8>, LoadError> {
  super::http::fetch(address, &source.headers, timeout, max_bytes, told)
}

/// A build without the feature `http` loads no address, and says so.
#[cfg(all(not(feature = "http"), not(target_arch = "wasm32")))]
fn fetch(
  _address: &str,
  _source: &Source,
  _timeout: Duration,
  _max_bytes: u64,
  _told: impl FnMut(u64, Option<u64>) -> bool,
) -> Result<Vec<u8>, LoadError> {
  static SAID: std::sync::Once = std::sync::Once::new();
  SAID.call_once(|| tracing::warn!("lurq is built without the feature `http`: an address is not loaded"));
  Err(LoadError::Unsupported(
    "this build has no `http` feature: it loads no address",
  ))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
  use std::{path::Path, sync::atomic::AtomicUsize, time::Duration};

  use super::*;

  thread_local! {
    /// Whether the loaders of this test read on a runtime: the tests of `on_a_runtime` say so.
    static ON_A_RUNTIME: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
  }

  /// A runtime of the tests' own.
  #[cfg(feature = "tokio")]
  fn runtime() -> tokio::runtime::Handle {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    let runtime = RUNTIME.get_or_init(|| {
      tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .build()
        .expect("a runtime for the tests")
    });
    runtime.handle().clone()
  }

  /// The loader of a test: one that reads on its own threads, or on a runtime where the test is run again there.
  fn loader() -> Loader {
    let loader = Loader::new();
    #[cfg(feature = "tokio")]
    if ON_A_RUNTIME.get() {
      loader.tokio_handle(runtime());
    }
    loader
  }

  /// What tells the files of a test from the files of the same test run again on a runtime.
  fn run() -> &'static str {
    if ON_A_RUNTIME.get() { "-on-a-runtime" } else { "" }
  }

  /// Waits for `load` to end.
  fn ended(load: &Load) -> LoadState {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
      let state = load.state();
      if !state.is_loading() {
        return state;
      }
      assert!(std::time::Instant::now() < deadline, "the load does not end");
      std::thread::sleep(Duration::from_millis(2));
    }
  }

  /// A file of this test's own, with `bytes` in it.
  fn file(name: &str, bytes: &[u8]) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("lurq-loader-{}{}", std::process::id(), run()));
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
  }

  fn folder_of(path: &Path) -> PathBuf {
    path.parent().unwrap().to_path_buf()
  }

  #[test]
  fn bytes_are_ready_at_once_and_start_no_thread() {
    let loader = loader();
    let load = loader.load(&Source::bytes(vec![1, 2, 3]));
    assert!(matches!(load.state(), LoadState::Ready(bytes) if bytes[..] == [1, 2, 3]));
    assert!(loader.shared.pool.get().is_none());
  }

  #[test]
  fn a_file_and_a_resource_under_the_root_are_read() {
    let path = file("read.bin", &[5, 6, 7]);
    let loader = loader();
    assert!(matches!(ended(&loader.load(&Source::file(&path))), LoadState::Ready(bytes) if bytes[..] == [5, 6, 7]));
    loader.root(folder_of(&path));
    let resource = loader.load(&Source::resource("read.bin"));
    assert!(matches!(ended(&resource), LoadState::Ready(bytes) if bytes[..] == [5, 6, 7]));
  }

  #[test]
  fn a_file_that_is_not_there_fails_and_stays_failed_until_forgotten() {
    let path = std::env::temp_dir().join(format!("lurq-loader-none-{}{}.bin", std::process::id(), run()));
    let source = Source::file(&path);
    let loader = loader();
    assert!(matches!(
      ended(&loader.load(&source)),
      LoadState::Failed(LoadError::NotFound)
    ));
    std::fs::write(&path, [1]).unwrap();
    // The same source: it is told it failed, and nothing reads the file again.
    assert!(matches!(
      loader.load(&source).state(),
      LoadState::Failed(LoadError::NotFound)
    ));
    assert!(loader.forget(&source));
    assert!(matches!(ended(&loader.load(&source)), LoadState::Ready(bytes) if bytes[..] == [1]));
    std::fs::remove_file(&path).unwrap();
  }

  #[test]
  fn a_file_over_the_limit_is_not_read_whole() {
    let path = file("large.bin", &[0; 4096]);
    let loader = loader();
    loader.max_bytes(1024);
    assert!(matches!(
      ended(&loader.load(&Source::file(&path))),
      LoadState::Failed(LoadError::TooLarge { limit: 1024 })
    ));
  }

  #[test]
  fn the_same_source_asked_twice_is_one_load_and_wakes_who_waits() {
    let path = file("twice.bin", &[9; 1000]);
    let loader = loader();
    let source = Source::file(&path);
    let woken = Arc::new(AtomicUsize::new(0));
    let first = loader.load(&source);
    let count = woken.clone();
    let wake: LoadWaker = Arc::new(move || {
      count.fetch_add(1, Ordering::SeqCst);
    });
    first.wake_with(wake.clone());
    first.wake_with(wake);
    let second = loader.load(&source);
    assert!(Arc::ptr_eq(&first.entry, &second.entry));
    assert!(matches!(ended(&second), LoadState::Ready(_)));
    // The waker was kept once, and a load that may have ended before it was given tells nothing.
    assert!(woken.load(Ordering::SeqCst) <= 1);
  }

  #[test]
  fn what_nobody_holds_is_let_go_after_a_moment_and_a_failure_is_kept() {
    let path = file("sweep.bin", &[1, 2]);
    let missing = Source::file(folder_of(&path).join("no-such-file.bin"));
    let loader = loader();
    let source = Source::file(&path);
    let load = loader.load(&source);
    let failed = loader.load(&missing);
    ended(&load);
    ended(&failed);
    drop((load, failed));
    loader.sweep();
    assert_eq!(
      loader.shared.entries.lock().len(),
      2,
      "a rebuild a moment later still finds both"
    );
    std::thread::sleep(IDLE_GRACE + Duration::from_millis(50));
    loader.sweep();
    let entries = loader.shared.entries.lock();
    assert_eq!(entries.len(), 1, "the bytes are let go");
    assert!(entries.contains_key(&missing.cache_key()), "the failure is kept");
  }

  #[test]
  fn an_address_the_app_does_not_allow_is_denied_and_nothing_reads_it() {
    let loader = loader();
    loader.allow(|address| address.starts_with("https://cdn.example.com/"));
    let load = loader.load(&Source::url("https://elsewhere.example.com/a.png"));
    assert!(matches!(load.state(), LoadState::Failed(LoadError::Denied)));
    assert!(loader.shared.pool.get().is_none());
  }

  #[cfg(not(feature = "http"))]
  #[test]
  fn a_build_without_http_says_it_loads_no_address() {
    let loader = loader();
    let state = ended(&loader.load(&Source::url("https://example.com/a.png")));
    assert!(matches!(state, LoadState::Failed(LoadError::Unsupported(_))));
  }

  #[test]
  fn a_reader_reads_a_part_of_a_file_and_of_bytes() {
    let path = file("part.bin", &[1, 2, 3, 4, 5, 6]);
    let loader = loader();
    let mut buf = [0u8; 2];
    let reader = loader.open(&Source::file(&path)).unwrap();
    assert_eq!(reader.read_at(4, &mut buf).unwrap(), 2);
    assert_eq!(buf, [5, 6]);
    let reader = loader.open(&Source::bytes(vec![7, 8, 9])).unwrap();
    assert_eq!(reader.read_at(1, &mut buf).unwrap(), 2);
    assert_eq!(buf, [8, 9]);
    assert!(matches!(
      loader.open(&Source::file(folder_of(&path).join("none.bin"))),
      Err(LoadError::NotFound)
    ));
    assert!(matches!(
      loader.open(&Source::url("https://example.com/a.mp4")),
      Err(LoadError::Unsupported(_))
    ));
  }

  /// A server on this machine that answers each request with `body`, and counts the requests.
  #[cfg(feature = "http")]
  fn serve(body: &'static [u8]) -> (String, Arc<AtomicUsize>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}/picture.png", listener.local_addr().unwrap());
    let asked = Arc::new(AtomicUsize::new(0));
    let counted = asked.clone();
    std::thread::spawn(move || {
      for stream in listener.incoming() {
        let Ok(mut stream) = stream else {
          return;
        };
        let (mut request, mut chunk) = (Vec::new(), [0_u8; 1024]);
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
          match stream.read(&mut chunk) {
            Ok(read) if read > 0 => request.extend_from_slice(&chunk[..read]),
            _ => break,
          }
        }
        counted.fetch_add(1, Ordering::SeqCst);
        let head = format!(
          "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
          body.len()
        );
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(body);
      }
    });
    (address, asked)
  }

  #[cfg(feature = "http")]
  #[test]
  fn what_came_from_an_address_is_kept_in_the_folder_the_app_gave_and_read_from_there() {
    let folder = std::env::temp_dir().join(format!("lurq-loader-cache-{}{}", std::process::id(), run()));
    let _ = std::fs::remove_dir_all(&folder);
    let (address, asked) = serve(&[4, 5, 6, 7]);
    let source = Source::url(&address);
    let first = loader();
    first.enable_media_disk_cache(&folder);
    assert!(matches!(ended(&first.load(&source)), LoadState::Ready(bytes) if bytes[..] == [4, 5, 6, 7]));
    assert!(first.media_disk_cache().holds(&source));
    assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 1);

    // The next run of the app: another loader, given the same folder, asks the server for nothing.
    let next = loader();
    next.enable_media_disk_cache(&folder);
    assert!(matches!(ended(&next.load(&source)), LoadState::Ready(bytes) if bytes[..] == [4, 5, 6, 7]));
    assert_eq!(asked.load(Ordering::SeqCst), 1);

    // Forgotten, its file is deleted, and it is asked of the server again.
    assert!(next.forget(&source));
    assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 0);
    assert!(matches!(ended(&next.load(&source)), LoadState::Ready(bytes) if bytes[..] == [4, 5, 6, 7]));
    assert_eq!(asked.load(Ordering::SeqCst), 2);

    // A loader that was given no folder writes nothing, and asks each time its bytes were let go.
    let plain = loader();
    assert!(matches!(ended(&plain.load(&source)), LoadState::Ready(_)));
    assert_eq!(asked.load(Ordering::SeqCst), 3);
    assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 1);
    let _ = std::fs::remove_dir_all(&folder);
  }

  /// The loader with the app's tokio runtime: its loads are read on the runtime, and it starts no threads.
  #[cfg(feature = "tokio")]
  mod on_a_runtime {
    use super::*;

    /// Each of these tests of the loader again, with its loads read on a runtime. A test added above is to be
    /// named here.
    macro_rules! again {
      ($($test:ident),* $(,)?) => {
        $(
          #[test]
          fn $test() {
            ON_A_RUNTIME.set(true);
            super::$test();
          }
        )*
      };
    }

    again!(
      bytes_are_ready_at_once_and_start_no_thread,
      a_file_and_a_resource_under_the_root_are_read,
      a_file_that_is_not_there_fails_and_stays_failed_until_forgotten,
      a_file_over_the_limit_is_not_read_whole,
      the_same_source_asked_twice_is_one_load_and_wakes_who_waits,
      what_nobody_holds_is_let_go_after_a_moment_and_a_failure_is_kept,
      an_address_the_app_does_not_allow_is_denied_and_nothing_reads_it,
      a_reader_reads_a_part_of_a_file_and_of_bytes,
    );

    #[cfg(feature = "http")]
    again!(what_came_from_an_address_is_kept_in_the_folder_the_app_gave_and_read_from_there);

    /// Loads `count` files of this test's own with `loader`, one after another, to their end.
    fn read(loader: &Loader, test: &str, count: usize) {
      for index in 0..count {
        let path = file(&format!("{test}-{index}.bin"), &[index as u8; 8]);
        let load = loader.load(&Source::file(&path));
        assert!(matches!(ended(&load), LoadState::Ready(bytes) if bytes.len() == 8));
      }
    }

    /// The handle of a runtime that has stopped: a load given to it fails, which tells where a load was given.
    fn stopped() -> tokio::runtime::Handle {
      let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .build()
        .expect("a runtime to stop");
      runtime.handle().clone()
    }

    #[test]
    fn with_a_handle_given_a_load_ends_and_no_thread_of_the_loaders_own_is_started() {
      let loader = Loader::new();
      loader.tokio_handle(runtime());
      read(&loader, "on-a-runtime", 6);
      assert!(
        loader.shared.pool.get().is_none(),
        "no pool of the loader's own was made"
      );
      assert_eq!(loader.pending(), 0);
    }

    #[test]
    fn a_load_given_to_a_runtime_that_stopped_fails_and_does_not_wait_for_ever() {
      let loader = Loader::new();
      loader.tokio_handle(stopped());
      let path = file("stopped.bin", &[1, 2, 3]);
      assert!(matches!(
        ended(&loader.load(&Source::file(&path))),
        LoadState::Failed(LoadError::Cancelled)
      ));
      assert!(loader.shared.pool.get().is_none());
    }

    #[test]
    fn a_handle_given_later_is_where_the_next_load_is_read_and_one_taken_away_gives_the_pool_back() {
      let loader = Loader::new();
      read(&loader, "before-a-handle", 1);
      assert!(
        loader.shared.pool.get().is_some(),
        "with no handle the loader reads on its own threads"
      );
      // The loader has its threads, and is given a runtime: the next load is the runtime's, not theirs.
      loader.tokio_handle(stopped());
      let path = file("given-later.bin", &[1, 2, 3]);
      assert!(matches!(
        ended(&loader.load(&Source::file(&path))),
        LoadState::Failed(LoadError::Cancelled)
      ));
      loader.tokio_handle(runtime());
      read(&loader, "after-a-handle", 1);
      // Taken away, the loader's own threads read again: nothing is given to the runtime that stopped.
      loader.tokio_handle(stopped());
      loader.reset_tokio_handle();
      read(&loader, "after-a-handle-was-taken", 1);
      assert_eq!(loader.pending(), 0);
    }

    #[test]
    fn no_more_loads_are_read_at_a_time_on_a_runtime_than_the_loader_has_threads() {
      // A megabyte is read in sixteen steps, and each step wakes who waits from the thread that reads: a
      // waker that stays a moment counts how many reads there are at once.
      let loader = Loader::new();
      loader.tokio_handle(runtime());
      let (now, most) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
      let megabyte = vec![7_u8; 1 << 20];
      let paths: Vec<PathBuf> = (0..LOADER_THREADS * 3)
        .map(|index| file(&format!("room-{index}.bin"), &megabyte))
        .collect();
      let loads: Vec<Load> = paths
        .iter()
        .map(|path| {
          let load = loader.load(&Source::file(path));
          let (now, most) = (now.clone(), most.clone());
          let wake: LoadWaker = Arc::new(move || {
            most.fetch_max(now.fetch_add(1, Ordering::SeqCst) + 1, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(2));
            now.fetch_sub(1, Ordering::SeqCst);
          });
          load.wake_with(wake);
          load
        })
        .collect();
      for load in &loads {
        assert!(matches!(ended(load), LoadState::Ready(bytes) if bytes.len() == 1 << 20));
      }
      for path in paths {
        let _ = std::fs::remove_file(path);
      }
      let most = most.load(Ordering::SeqCst);
      assert!((1..=LOADER_THREADS).contains(&most), "{most} at a time");
      assert!(loader.shared.pool.get().is_none());
    }
  }
}
