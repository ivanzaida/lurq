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
    Arc,
    atomic::{AtomicBool, AtomicU8, AtomicU64, AtomicUsize, Ordering},
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
/// How much of an address one request of a reader asks for: its first two, and the most, which the ones
/// after them grow to. On a host that sends about 600 KB a second a first block of 1 MiB kept a video's first
/// frame waiting two seconds; smaller than a quarter of one, the requests cost more than they save.
const DEFAULT_FIRST_READ_BLOCK: u64 = 256 << 10;
const DEFAULT_READ_BLOCK: u64 = 1 << 20;
/// The threads that read: how many loads are read at a time, on the loader's own threads or on the app's
/// runtime.
const LOADER_THREADS: usize = 4;

/// Of how many addresses the start is kept once their readers are dropped, unless told
/// ([`Loader::keep_starts`]).
const KEPT_STARTS: usize = 8;
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
    // A reader of an address fails with what the server answered, carried in the error a reader gives.
    if let Some(error) = error.get_ref().and_then(|inner| inner.downcast_ref::<LoadError>()) {
      return error.clone();
    }
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

/// How soon a load is read, among those that wait their turn: see [`Load::set_priority`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LoadPriority {
  /// After the others: what is far from what is shown, fetched for later.
  Low,
  /// As it comes.
  #[default]
  Normal,
  /// Before the others: what is on the screen now.
  High,
}

impl LoadPriority {
  fn of(level: u8) -> Self {
    match level {
      0 => Self::Low,
      1 => Self::Normal,
      _ => Self::High,
    }
  }
}

/// The count of the loads asked for, in the order they were: the one asked for last has the largest.
static ASKED: AtomicU64 = AtomicU64::new(1);

struct Entry {
  state: Mutex<LoadState>,
  /// Nobody waits for it: what reads it stops at its next step, and what has not started it does not.
  cancelled: AtomicBool,
  /// How soon it is read among those that wait, and when it was last asked for.
  priority: AtomicU8,
  asked: AtomicU64,
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
      priority: AtomicU8::new(LoadPriority::Normal as u8),
      asked: AtomicU64::new(ASKED.fetch_add(1, Ordering::Relaxed)),
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

  /// Whether anybody still waits for it.
  fn wanted(&self) -> bool {
    !self.cancelled.load(Ordering::Acquire)
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

/// A read that waits its turn.
#[cfg(not(target_arch = "wasm32"))]
struct Waits {
  /// What tells it from the others that wait.
  turn: u64,
  entry: Arc<Entry>,
  read: Box<dyn FnOnce() + Send>,
}

/// The reads that wait their turn. As many are read at once as the loader's pool is large; when one of its
/// threads (or a blocking task of the app's runtime) is free, the read it takes is not the one that came first
/// but the one of the highest [priority](LoadPriority), and among those the one asked for last: in a long list
/// scrolled fast, what is on the screen now is read before what went by. A read nobody waits for any more is
/// not started at all.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
struct Waiting {
  reads: Mutex<Vec<Waits>>,
  turns: AtomicU64,
}

#[cfg(not(target_arch = "wasm32"))]
impl Waiting {
  /// Takes a read in, and gives the turn that will read one: its own, or one that came after and goes before.
  fn push(self: &Arc<Self>, entry: Arc<Entry>, read: Box<dyn FnOnce() + Send>) -> Turn {
    let turn = self.turns.fetch_add(1, Ordering::Relaxed);
    self.reads.lock().push(Waits { turn, entry, read });
    Turn {
      waiting: self.clone(),
      turn,
      taken: false,
    }
  }

  /// The read whose turn it is. Those nobody waits for are let go, unread: each ends as a load that was stopped.
  fn next(&self) -> Option<Box<dyn FnOnce() + Send>> {
    let (next, gone) = {
      let mut reads = self.reads.lock();
      let (kept, gone): (Vec<Waits>, Vec<Waits>) = std::mem::take(&mut *reads)
        .into_iter()
        .partition(|waits| waits.entry.wanted());
      *reads = kept;
      let best = reads
        .iter()
        .enumerate()
        .max_by_key(|(_, waits)| {
          (
            waits.entry.priority.load(Ordering::Relaxed),
            waits.entry.asked.load(Ordering::Relaxed),
          )
        })
        .map(|(at, _)| at);
      (best.map(|at| reads.swap_remove(at)), gone)
    };
    // Outside the lock: a read let go tells who waited for it, which may ask the loader again.
    drop(gone);
    next.map(|waits| waits.read)
  }

  /// A turn was dropped unread, by a runtime that stopped: one read will never have its turn, and is let go.
  /// Its own, where it still waits; else the one that would have been read last.
  fn give_up(&self, turn: u64) {
    let gone = {
      let mut reads = self.reads.lock();
      let at = reads.iter().position(|waits| waits.turn == turn).or_else(|| {
        reads
          .iter()
          .enumerate()
          .min_by_key(|(_, waits)| {
            (
              waits.entry.priority.load(Ordering::Relaxed),
              waits.entry.asked.load(Ordering::Relaxed),
            )
          })
          .map(|(at, _)| at)
      });
      at.map(|at| reads.swap_remove(at))
    };
    drop(gone);
  }
}

/// A place in the line of the reads that wait: run, it reads the one whose turn it is; dropped unrun, one read
/// of the line is let go, so that none waits for a turn that never comes.
#[cfg(not(target_arch = "wasm32"))]
struct Turn {
  waiting: Arc<Waiting>,
  turn: u64,
  taken: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl Turn {
  fn run(mut self) {
    self.taken = true;
    if let Some(read) = self.waiting.next() {
      read();
    }
  }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for Turn {
  fn drop(&mut self) {
    if !self.taken {
      self.waiting.give_up(self.turn);
    }
  }
}

/// Ends a load whose read was dropped before it ended: a runtime that stopped drops what it had not started.
#[cfg(not(target_arch = "wasm32"))]
struct Unread(Arc<Entry>);

#[cfg(not(target_arch = "wasm32"))]
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

  /// How soon it is read among the loads that wait their turn: before the others (`High`, what is on the
  /// screen now), as it comes (`Normal`, unless told), or after the others (`Low`). Among loads of one
  /// priority the one asked for last is read first. It can be told again while the load waits: a picture
  /// that scrolls into view goes before the ones that scrolled out. Nothing to a load that is being read.
  pub fn set_priority(&self, priority: LoadPriority) {
    self.entry.priority.store(priority as u8, Ordering::Relaxed);
  }

  /// [`set_priority`](Self::set_priority), on a load as it is asked for.
  pub fn with_priority(self, priority: LoadPriority) -> Self {
    self.set_priority(priority);
    self
  }

  /// How soon it is read among the loads that wait.
  pub fn priority(&self) -> LoadPriority {
    LoadPriority::of(self.entry.priority.load(Ordering::Relaxed))
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
type Proxy = Arc<dyn Fn(&str) -> String + Send + Sync>;
type HeaderRule = Arc<dyn Fn(&str) -> Vec<(String, String)> + Send + Sync>;

struct Settings {
  root: Option<PathBuf>,
  timeout: Duration,
  max_bytes: u64,
  allow: Option<Allow>,
  proxy: Option<Proxy>,
  /// How many loads run at once, and how many HTTP agents are kept: see [`Loader::pool_size`].
  pool_size: usize,
  headers: Option<HeaderRule>,
  /// How much of an address one request of a reader asks for: at first, and at most.
  #[cfg_attr(not(all(feature = "http", not(target_arch = "wasm32"))), allow(dead_code))]
  first_block: u64,
  #[cfg_attr(not(all(feature = "http", not(target_arch = "wasm32"))), allow(dead_code))]
  block: u64,
}

struct Shared {
  settings: RwLock<Settings>,
  entries: Mutex<HashMap<u64, Arc<Entry>>>,
  /// The reads that wait for a thread of the pool, or for room on the app's runtime.
  #[cfg(not(target_arch = "wasm32"))]
  waiting: Arc<Waiting>,
  /// Made by the first load that reads: an app that loads nothing has no threads for it.
  pool: Mutex<Option<ThreadPool>>,
  /// What was loaded from addresses, kept in the folder the app gave: off until it gives one.
  disk: MediaDiskCache,
  /// The HTTP agents lent to its loads and readers, as many kept as the pool is large.
  agents: AgentPool,
  /// The start of the addresses its readers read last, kept once they are dropped.
  #[cfg_attr(not(all(feature = "http", not(target_arch = "wasm32"))), allow(dead_code))]
  starts: StartPool,
  /// The app's tokio runtime, where it gave one: loads are read on it, and the pool is not made.
  #[cfg(feature = "tokio")]
  runtime: Mutex<Option<tokio::runtime::Handle>>,
  /// The room for reads on that runtime: as many at a time as the pool has threads.
  #[cfg(feature = "tokio")]
  room: Mutex<Arc<tokio::sync::Semaphore>>,
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
          proxy: None,
          pool_size: LOADER_THREADS,
          headers: None,
          first_block: DEFAULT_FIRST_READ_BLOCK,
          block: DEFAULT_READ_BLOCK,
        }),
        entries: Mutex::new(HashMap::new()),
        #[cfg(not(target_arch = "wasm32"))]
        waiting: Arc::default(),
        pool: Mutex::new(None),
        agents: agent_pool(LOADER_THREADS),
        starts: start_pool(KEPT_STARTS),
        disk: MediaDiskCache::new(),
        #[cfg(feature = "tokio")]
        runtime: Mutex::new(None),
        #[cfg(feature = "tokio")]
        room: Mutex::new(Arc::new(tokio::sync::Semaphore::new(LOADER_THREADS))),
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

  /// How many loads are read at once, and how many HTTP agents the loader keeps: 4 unless told, 1 at least. The
  /// loads are read on as many threads of the loader's own, or as many blocking tasks at a time of the app's
  /// runtime where it gave one ([`tokio_handle`](Self::tokio_handle)). Each load of an address is lent an agent,
  /// which it gives back when it ends, with the connections it opened kept open: the next load of the same host
  /// goes over one of them, with no new handshake. A reader of an address ([`open`](Self::open)) is lent them
  /// too, for each of its requests.
  ///
  /// Told again, it is for the loads asked for from then on: those on their way end where they are, on the
  /// threads they were given, and an agent given back beyond the new size is let go.
  pub fn pool_size(&self, size: usize) -> &Self {
    let size = size.max(1);
    {
      let mut settings = self.shared.settings.write();
      if settings.pool_size == size {
        return self;
      }
      settings.pool_size = size;
    }
    #[cfg(all(feature = "http", not(target_arch = "wasm32")))]
    self.shared.agents.resize(size);
    #[cfg(feature = "tokio")]
    {
      *self.shared.room.lock() = Arc::new(tokio::sync::Semaphore::new(size));
    }
    // The threads of the size before end once they have read what was given them, away from the caller: a pool
    // waits for its threads as it is dropped.
    if let Some(before) = self.shared.pool.lock().take() {
      let _ = std::thread::Builder::new()
        .name("lurq-loader-ending".to_owned())
        .spawn(move || drop(before));
    }
    self
  }

  /// Which addresses may be loaded: every one the app asks for, unless told. An address `allow` refuses fails
  /// with [`LoadError::Denied`] and nothing is sent. It is what keeps a text from a stranger, shown by the app,
  /// from fetching what the stranger chose.
  pub fn allow(&self, allow: impl Fn(&str) -> bool + Send + Sync + 'static) -> &Self {
    self.shared.settings.write().allow = Some(Arc::new(allow));
    self
  }

  /// Where an address is fetched from: `proxy` is given the address the app asked for and gives the one to fetch,
  /// a server of the app's own that fetches it. Every address is fetched where it is, unless told. A browser
  /// reads an address on another server only where that server lets the page's origin read it (CORS); one that
  /// does not, as most pages of the web do not, is read through such a server:
  ///
  /// ```no_run
  /// # let loader = lurq::resources::Loader::new();
  /// loader.proxy(|address| format!("https://example.com/fetch?url={address}"));
  /// ```
  ///
  /// [`allow`](Self::allow) is asked of the address the app asked for, and the cache on the disk keeps it under
  /// that address.
  pub fn proxy(&self, proxy: impl Fn(&str) -> String + Send + Sync + 'static) -> &Self {
    self.shared.settings.write().proxy = Some(Arc::new(proxy));
    self
  }

  /// The address `address` is fetched from: the [proxy](Self::proxy)'s, or itself.
  fn fetched_from(&self, address: &str) -> String {
    match &self.shared.settings.read().proxy {
      Some(proxy) => proxy(address),
      None => address.to_owned(),
    }
  }

  /// Headers of every request for an address, given by `rule` from the address the app asked for: a picture's
  /// load, a sound's, and each request a video or a sound makes as it plays. A source's own headers
  /// ([`Source::header`]) come after them and take the place of one of the same name.
  ///
  /// ```no_run
  /// # let loader = lurq::resources::Loader::new();
  /// loader.headers(|address| {
  ///   if address.starts_with("https://videos.example.com/") {
  ///     vec![("Referer".to_owned(), "https://example.com/".to_owned())]
  ///   } else {
  ///     Vec::new()
  ///   }
  /// });
  /// ```
  pub fn headers(&self, rule: impl Fn(&str) -> Vec<(String, String)> + Send + Sync + 'static) -> &Self {
    self.shared.settings.write().headers = Some(Arc::new(rule));
    self
  }

  /// The headers of a request for `source`: the [rule](Self::headers)'s, then the source's own in place of one
  /// of the same name.
  pub(crate) fn headers_of(&self, source: &Source) -> Vec<(Arc<str>, Arc<str>)> {
    let rule = self.shared.settings.read().headers.clone();
    let mut headers: Vec<(Arc<str>, Arc<str>)> = match (&source.kind, rule) {
      (SourceKind::Url(address), Some(rule)) => rule(address)
        .into_iter()
        .map(|(name, value)| (Arc::from(name), Arc::from(value)))
        .collect(),
      _ => Vec::new(),
    };
    for (name, value) in &source.headers {
      headers.retain(|(had, _)| !had.eq_ignore_ascii_case(name));
      headers.push((name.clone(), value.clone()));
    }
    headers
  }

  /// How much of an address one request of a [reader](Self::open) asks for, every one of them: for a test that
  /// reads a small file in many requests. An app says [`read_blocks`](Self::read_blocks).
  #[doc(hidden)]
  pub fn read_block(&self, bytes: u64) -> &Self {
    self.read_blocks(bytes, bytes)
  }

  /// How much of an address one request of a [reader](Self::open) asks for: `first` bytes in each of its first
  /// two, twice as much in each one after them up to `most`, and `most` from there on. 256 KiB and 1 MiB unless
  /// told. What opens a video reads its start, and a small first block is there sooner where the host is slow;
  /// what plays it reads on, and larger blocks are fewer requests. For the readers opened from then on.
  pub fn read_blocks(&self, first: u64, most: u64) -> &Self {
    let mut settings = self.shared.settings.write();
    settings.block = most.max(1);
    settings.first_block = first.clamp(1, settings.block);
    drop(settings);
    self
  }

  /// Of how many addresses the start is kept in memory once their [readers](Self::open) are dropped: 8 unless
  /// told, and none with 0. The start is the first blocks a reader read whole, 2 MiB of them at most, for five
  /// minutes: a reader of an address whose start is kept starts with it, with no request, so a video closed and
  /// opened again a moment later (a page of a pager gone and come back) opens at once and asks the network only
  /// for what follows. [`forget`](Self::forget) lets go of the start of a source.
  pub fn keep_starts(&self, addresses: usize) -> &Self {
    #[cfg(all(feature = "http", not(target_arch = "wasm32")))]
    self.shared.starts.resize(addresses);
    #[cfg(not(all(feature = "http", not(target_arch = "wasm32"))))]
    let _ = addresses;
    self
  }

  /// Opens a connection to the host of `address` before the first request to it, away from the caller: where
  /// an app knows the host of a video a moment before it makes its player, the player's first request goes
  /// over a connection that is there, and does not wait for the handshake. The host is asked for the head of
  /// what is at the address (a `HEAD` request, with the [rule's](Self::headers) headers), whose answer has
  /// nothing in it; whatever it answers, the connection is kept for a quarter of a minute. A host asked in the
  /// last ten seconds is not asked again. Nothing is sent to an address the loader does not
  /// [allow](Self::allow), in a browser, or in a build without the feature `http`.
  pub fn connect(&self, address: &str) -> &Self {
    #[cfg(all(feature = "http", not(target_arch = "wasm32")))]
    if self.allows(address) {
      let timeout = self.shared.settings.read().timeout;
      super::http::connect(
        &self.shared.agents,
        self.fetched_from(address),
        self.headers_of(&Source::url(address)),
        timeout,
      );
    }
    #[cfg(not(all(feature = "http", not(target_arch = "wasm32"))))]
    let _ = address;
    self
  }

  /// Reads on the tokio runtime `handle` is of, and not on threads of the loader's own: a load is a blocking
  /// task of the runtime (`spawn_blocking`), as many at a time as the [pool](Self::pool_size) is large, as the loader's
  /// own threads would read. A loader that has loaded nothing yet starts no threads at all. A load that was on its
  /// way when the handle was given ends on the thread that reads it; the ones asked for after it are read on the
  /// runtime. What a load does is the same on both: how far it came, its timeout, its limit, how it is stopped, who
  /// it wakes.
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
      // Asked for again, it is the newest: among the loads that wait it goes first.
      entry
        .asked
        .store(ASKED.fetch_add(1, Ordering::Relaxed), Ordering::Relaxed);
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
      self.start(source, entry, None);
    }
    load
  }

  /// Loads the first `bytes` bytes of `source`, or all of it where it has fewer, off the caller's thread: the
  /// start of a page, which says what the page is about in its head. More than that is not read, and is no
  /// error: the read stops there and the load is ready with what it read. It is a load of its own, kept and
  /// joined as [`load`](Self::load)'s are, apart from a whole load of the same source, and the
  /// [cache on the disk](Self::media_disk_cache) neither keeps nor gives it.
  pub fn load_start(&self, source: &Source, bytes: u64) -> Load {
    let key = {
      let mut hasher = std::collections::hash_map::DefaultHasher::new();
      std::hash::Hash::hash(&("start", source.cache_key(), bytes), &mut hasher);
      std::hash::Hasher::finish(&hasher)
    };
    let mut entries = self.shared.entries.lock();
    if let Some(entry) = entries.get(&key) {
      return Load::of(entry);
    }
    let entry = match &source.kind {
      SourceKind::Bytes(all) => Entry::new(LoadState::Ready(Arc::from(
        &all[..all.len().min(usize::try_from(bytes).unwrap_or(usize::MAX))],
      ))),
      _ => Entry::new(LoadState::Loading { loaded: 0, total: None }),
    };
    entries.insert(key, entry.clone());
    drop(entries);
    let load = Load::of(&entry);
    if !matches!(source.kind, SourceKind::Bytes(_)) {
      self.start(source, entry, Some(bytes));
    }
    load
  }

  /// Starts reading `source` into `entry`: all of it, or its first `head` bytes.
  fn start(&self, source: &Source, entry: Arc<Entry>, head: Option<u64>) {
    // The page's `fetch`: a file and a resource are the page's own files at their paths.
    #[cfg(target_arch = "wasm32")]
    {
      let (timeout, max_bytes, allow) = {
        let settings = self.shared.settings.read();
        (settings.timeout, settings.max_bytes, settings.allow.clone())
      };
      let address = match &source.kind {
        SourceKind::Url(address) if allow.as_ref().is_some_and(|allow| !allow(address)) => {
          entry.finish(Err(LoadError::Denied));
          return;
        }
        SourceKind::Url(address) => self.fetched_from(address),
        SourceKind::File(path) => super::web_fetch::address_of(path),
        SourceKind::Resource(resource) => super::web_fetch::address_of(&self.resource_path(resource)),
        SourceKind::Bytes(_) => return,
      };
      let headers = self.headers_of(source);
      wasm_bindgen_futures::spawn_local(async move {
        let told = |loaded, total| entry.progress(loaded, total);
        let result = super::web_fetch::fetch(
          &address,
          &headers,
          timeout,
          head.unwrap_or(max_bytes),
          head.is_some(),
          told,
        )
        .await;
        entry.finish(result);
      });
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
      let fetched = match &source.kind {
        SourceKind::Url(address) => self.fetched_from(address),
        _ => String::new(),
      };
      let headers = self.headers_of(source);
      let source = source.clone();
      let disk = self.shared.disk.clone();
      let agents = self.shared.agents.clone();
      let unread = Unread(entry.clone());
      let waits = entry.clone();
      let read = move || {
        let _unread = unread;
        // Nobody waits for it any more: it is not started.
        if !entry.wanted() {
          return;
        }
        let told = |loaded, total| entry.progress(loaded, total);
        let result = match (path, &source.kind, head) {
          (Some(path), _, Some(head)) => FileReader::open(&path)
            .and_then(|reader| read_start(&reader, head))
            .map_err(LoadError::from),
          (None, SourceKind::Url(_), Some(head)) => fetch(&agents, &fetched, &headers, timeout, head, true, told),
          (Some(path), _, None) => FileReader::open(&path)
            .and_then(|reader| read_all(&reader, Some(max_bytes), told))
            .map_err(|error| match error.kind() {
              io::ErrorKind::FileTooLarge => LoadError::TooLarge { limit: max_bytes },
              _ => LoadError::from(error),
            }),
          // What the cache on the disk keeps of the address is read from there; what is fetched is kept.
          (None, SourceKind::Url(address), None) => match disk.read(source.cache_key(), max_bytes) {
            Some(bytes) => Ok(bytes),
            None => fetch(&agents, &fetched, &headers, timeout, max_bytes, false, told).inspect(|bytes| {
              disk.put(source.cache_key(), address, bytes);
            }),
          },
          (None, ..) => Err(LoadError::Unsupported("nothing reads this source")),
        };
        entry.finish(result);
      };
      // It waits its turn with the others: what is free next reads the one of the highest priority, the one
      // asked for last among those, and not the one that came first.
      let turn = self.shared.waiting.push(waits, Box::new(read));
      // On the app's runtime where it gave one: a blocking task of it, once there is room for one more read.
      #[cfg(feature = "tokio")]
      if let Some(handle) = self.shared.runtime.lock().clone() {
        let room = self.shared.room.lock().clone();
        drop(handle.spawn(async move {
          if let Ok(_room) = room.acquire().await {
            let _ = tokio::task::spawn_blocking(move || turn.run()).await;
          }
        }));
        return;
      }
      let size = self.shared.settings.read().pool_size;
      self
        .shared
        .pool
        .lock()
        .get_or_insert_with(|| ThreadPool::new(size))
        .execute(move || turn.run());
    }
  }

  /// A reader of `source`, for what reads a part of it and not all: a video, a sound. A file, a resource and
  /// bytes are read where they are.
  ///
  /// An address is read by ranges ([`read_blocks`](Self::read_blocks)) as they are asked for, with the
  /// [rule's](Self::headers) and the source's headers and the loader's timeout on every request. A read is
  /// given its bytes as they come, and does not wait for the range they are in to be whole. The requests go
  /// one after the other over one connection: the range a read waits for, then, where it reads ahead
  /// ([`Reader::read_ahead`]), what follows the last read, and that only while no read waits. A second request
  /// is made beside the first only for a read that waits for bytes the first is not bringing (a video sent to
  /// another place), and a request nobody waits for stands still meanwhile, so that a host that gives one
  /// client so much a second in all gives it to the bytes that are waited for.
  ///
  /// Its first range is asked for, and its answer has come, before this returns, so it blocks: call it off the
  /// UI thread. That answer gives its length and its type ([`Reader::content_type`]), and a server that refuses
  /// it fails here, with what it answered ([`LoadError::Status`], [`LoadError::NotFound`]). An address whose
  /// reader was dropped a moment ago is opened with the start that reader read, with no request
  /// ([`keep_starts`](Self::keep_starts)). A read that fails later gives an [`io::Error`] that
  /// `LoadError::from` turns into the same. What the [cache on the disk](Self::media_disk_cache) keeps of it is
  /// read from its file. An address the loader does not [allow](Self::allow) fails with [`LoadError::Denied`]; in a
  /// build without the feature `http` and in a browser, with [`LoadError::Unsupported`].
  pub fn open(&self, source: &Source) -> Result<Arc<dyn Reader>, LoadError> {
    self.open_reading(source, None)
  }

  /// [`open`](Self::open), by what knows whether the reader is to read ahead before it opens it: a media told
  /// to play, or one opened ahead and left paused. Where it is, the first request to an address asks for what
  /// is read ahead with the first block, and not for the first block alone.
  #[cfg_attr(not(all(feature = "media", windows)), allow(dead_code))]
  pub(crate) fn open_to_play(&self, source: &Source, read_ahead: bool) -> Result<Arc<dyn Reader>, LoadError> {
    self.open_reading(source, Some(read_ahead))
  }

  fn open_reading(&self, source: &Source, read_ahead: Option<bool>) -> Result<Arc<dyn Reader>, LoadError> {
    match &source.kind {
      SourceKind::Bytes(bytes) => Ok(Arc::new(BytesReader::new(bytes.clone()))),
      SourceKind::File(path) => Ok(Arc::new(FileReader::open(path)?)),
      SourceKind::Resource(resource) => Ok(Arc::new(FileReader::open(self.resource_path(resource))?)),
      SourceKind::Url(address) if !self.allows(address) => Err(LoadError::Denied),
      SourceKind::Url(address) => self.open_address(address, source, read_ahead),
    }
  }

  #[cfg(all(feature = "http", not(target_arch = "wasm32")))]
  fn open_address(
    &self,
    address: &str,
    source: &Source,
    read_ahead: Option<bool>,
  ) -> Result<Arc<dyn Reader>, LoadError> {
    if let Some(path) = self.shared.disk.file(source.cache_key())
      && let Ok(reader) = FileReader::open(path)
    {
      return Ok(Arc::new(reader));
    }
    let (timeout, cut) = {
      let settings = self.shared.settings.read();
      let cut = super::http::Cut {
        first: settings.first_block,
        most: settings.block,
      };
      (settings.timeout, cut)
    };
    Ok(Arc::new(super::http::HttpReader::open(
      &self.shared.agents,
      self.shared.starts.clone(),
      super::http::Asked {
        key: source.cache_key(),
        address: self.fetched_from(address),
        headers: self.headers_of(source),
        timeout,
        cut,
        read_ahead,
      },
    )?))
  }

  #[cfg(not(all(feature = "http", not(target_arch = "wasm32"))))]
  fn open_address(
    &self,
    _address: &str,
    _source: &Source,
    _read_ahead: Option<bool>,
  ) -> Result<Arc<dyn Reader>, LoadError> {
    Err(LoadError::Unsupported(
      "this build reads an address only whole: a reader of one needs the feature `http`, outside a browser",
    ))
  }

  /// Forgets the load of `source` that ended, so that it is loaded again when it is next asked for: the way to
  /// try again one that failed. The file the [cache on the disk](Self::media_disk_cache) keeps of it is deleted,
  /// so that an address is asked of the network again: the way to load anew what has changed there. `true`
  /// where there was a load or a file to forget. A load still on its way is left to end.
  pub fn forget(&self, source: &Source) -> bool {
    let key = source.cache_key();
    let kept = self.shared.disk.forget_key(key);
    #[cfg(all(feature = "http", not(target_arch = "wasm32")))]
    let kept = self.shared.starts.forget(key) || kept;
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

/// The first `head` bytes of what `reader` holds, or all of it where it holds fewer.
#[cfg(not(target_arch = "wasm32"))]
fn read_start(reader: &dyn Reader, head: u64) -> io::Result<Vec<u8>> {
  let mut bytes = vec![0u8; usize::try_from(head).unwrap_or(usize::MAX).min(16 * 1024 * 1024)];
  let mut filled = 0;
  while filled < bytes.len() {
    let read = reader.read_at(filled as u64, &mut bytes[filled..])?;
    if read == 0 {
      break;
    }
    filled += read;
  }
  bytes.truncate(filled);
  Ok(bytes)
}

/// The HTTP agents of a loader: none in a build that reads no address.
#[cfg(all(feature = "http", not(target_arch = "wasm32")))]
type AgentPool = Arc<super::http::Agents>;
#[cfg(not(all(feature = "http", not(target_arch = "wasm32"))))]
type AgentPool = ();

#[cfg(all(feature = "http", not(target_arch = "wasm32")))]
fn agent_pool(size: usize) -> AgentPool {
  super::http::Agents::new(size)
}

#[cfg(not(all(feature = "http", not(target_arch = "wasm32"))))]
fn agent_pool(_size: usize) -> AgentPool {}

/// The starts a loader keeps of the addresses its readers read: none in a build that reads no address.
#[cfg(all(feature = "http", not(target_arch = "wasm32")))]
type StartPool = Arc<super::http::Starts>;
#[cfg(not(all(feature = "http", not(target_arch = "wasm32"))))]
type StartPool = ();

#[cfg(all(feature = "http", not(target_arch = "wasm32")))]
fn start_pool(addresses: usize) -> StartPool {
  super::http::Starts::new(addresses)
}

#[cfg(not(all(feature = "http", not(target_arch = "wasm32"))))]
fn start_pool(_addresses: usize) -> StartPool {}

/// Reads all of what is at `address`, or its first `max_bytes` where `cut`.
#[cfg(all(feature = "http", not(target_arch = "wasm32")))]
fn fetch(
  agents: &AgentPool,
  address: &str,
  headers: &[(Arc<str>, Arc<str>)],
  timeout: Duration,
  max_bytes: u64,
  cut: bool,
  told: impl FnMut(u64, Option<u64>) -> bool,
) -> Result<Vec<u8>, LoadError> {
  super::http::fetch(agents, address, headers, timeout, max_bytes, cut, told)
}

/// A build without the feature `http` loads no address, and says so.
#[cfg(all(not(feature = "http"), not(target_arch = "wasm32")))]
fn fetch(
  _agents: &AgentPool,
  _address: &str,
  _headers: &[(Arc<str>, Arc<str>)],
  _timeout: Duration,
  _max_bytes: u64,
  _cut: bool,
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
    static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
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
    assert!(loader.shared.pool.lock().is_none());
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
  fn the_start_of_a_source_is_read_and_no_more_and_is_no_error() {
    let path = file("start.bin", &[1, 2, 3, 4, 5, 6]);
    let loader = loader();
    let source = Source::file(&path);
    assert!(matches!(ended(&loader.load_start(&source, 4)), LoadState::Ready(bytes) if bytes[..] == [1, 2, 3, 4]));
    assert!(
      matches!(ended(&loader.load_start(&source, 100)), LoadState::Ready(bytes) if bytes[..] == [1, 2, 3, 4, 5, 6])
    );
    // A whole load of the same source is another load.
    assert!(matches!(ended(&loader.load(&source)), LoadState::Ready(bytes) if bytes.len() == 6));
    let bytes = Source::bytes(vec![7, 8, 9]);
    assert!(matches!(loader.load_start(&bytes, 2).state(), LoadState::Ready(start) if start[..] == [7, 8]));
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
    assert!(loader.shared.pool.lock().is_none());
  }

  #[cfg(not(feature = "http"))]
  #[test]
  fn a_build_without_http_says_it_loads_no_address() {
    let loader = loader();
    let state = ended(&loader.load(&Source::url("https://example.com/a.png")));
    assert!(matches!(state, LoadState::Failed(LoadError::Unsupported(_))));
  }

  #[test]
  fn the_rules_headers_come_first_and_a_sources_own_take_the_place_of_one_of_the_same_name() {
    let loader = loader();
    loader.headers(|address| {
      if address.starts_with("https://videos.example.com/") {
        vec![
          ("Referer".to_owned(), "https://example.com/".to_owned()),
          ("X-Client".to_owned(), "lurq".to_owned()),
        ]
      } else {
        Vec::new()
      }
    });
    let names = |source: &Source| -> Vec<(String, String)> {
      loader
        .headers_of(source)
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect()
    };
    let pair = |name: &str, value: &str| (name.to_owned(), value.to_owned());
    assert_eq!(
      names(&Source::url("https://videos.example.com/a.mp4").header("referer", "https://example.com/watch")),
      vec![pair("X-Client", "lurq"), pair("referer", "https://example.com/watch")]
    );
    assert_eq!(
      names(&Source::url("https://elsewhere.example.com/a.mp4").header("Authorization", "Bearer token")),
      vec![pair("Authorization", "Bearer token")]
    );
    assert!(names(&Source::bytes(vec![1])).is_empty());
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
    #[cfg(not(feature = "http"))]
    assert!(matches!(
      loader.open(&Source::url("https://example.com/a.mp4")),
      Err(LoadError::Unsupported(_))
    ));
    loader.allow(|_| false);
    assert!(matches!(
      loader.open(&Source::url("https://example.com/a.mp4")),
      Err(LoadError::Denied)
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

  /// What a [`serve_kept`] server counted.
  #[cfg(feature = "http")]
  #[derive(Default)]
  struct Served {
    connections: AtomicUsize,
    now: AtomicUsize,
    most: AtomicUsize,
  }

  /// A server on this machine that keeps its connections open, as a host of pictures does: it answers every
  /// request of a connection with `body`, `pause` after it came. Its address, without a path.
  #[cfg(feature = "http")]
  fn serve_kept(body: &'static [u8], pause: Duration) -> (String, Arc<Served>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let served = Arc::new(Served::default());
    let counts = served.clone();
    std::thread::spawn(move || {
      for stream in listener.incoming() {
        let Ok(mut stream) = stream else {
          return;
        };
        counts.connections.fetch_add(1, Ordering::SeqCst);
        let counts = counts.clone();
        std::thread::spawn(move || {
          let mut request = Vec::new();
          let mut chunk = [0_u8; 1024];
          loop {
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
              match stream.read(&mut chunk) {
                Ok(read) if read > 0 => request.extend_from_slice(&chunk[..read]),
                _ => return,
              }
            }
            request.clear();
            counts
              .most
              .fetch_max(counts.now.fetch_add(1, Ordering::SeqCst) + 1, Ordering::SeqCst);
            std::thread::sleep(pause);
            counts.now.fetch_sub(1, Ordering::SeqCst);
            let head = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", body.len());
            if stream.write_all(head.as_bytes()).is_err() || stream.write_all(body).is_err() {
              return;
            }
          }
        });
      }
    });
    (address, served)
  }

  /// A server on this machine that answers every request with three bytes, `pause` after it came, and keeps
  /// the paths it was asked for, in order.
  #[cfg(feature = "http")]
  fn serve_named(pause: Duration) -> (String, Arc<Mutex<Vec<String>>>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let asked: Arc<Mutex<Vec<String>>> = Arc::default();
    let paths = asked.clone();
    std::thread::spawn(move || {
      for stream in listener.incoming() {
        let Ok(mut stream) = stream else {
          return;
        };
        let paths = paths.clone();
        std::thread::spawn(move || {
          let mut request = Vec::new();
          let mut chunk = [0_u8; 1024];
          loop {
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
              match stream.read(&mut chunk) {
                Ok(read) if read > 0 => request.extend_from_slice(&chunk[..read]),
                _ => return,
              }
            }
            let said = String::from_utf8_lossy(&request).into_owned();
            let path = said.split_whitespace().nth(1).unwrap_or_default().to_owned();
            request.clear();
            paths.lock().push(path);
            std::thread::sleep(pause);
            if stream
              .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabc")
              .is_err()
            {
              return;
            }
          }
        });
      }
    });
    (address, asked)
  }

  /// Waits until the server was asked for `count` paths.
  #[cfg(feature = "http")]
  fn asked_for(asked: &Mutex<Vec<String>>, count: usize) {
    let until = Instant::now() + Duration::from_secs(5);
    while asked.lock().len() < count {
      assert!(Instant::now() < until, "asked for {:?} only", asked.lock());
      std::thread::sleep(Duration::from_millis(2));
    }
  }

  #[cfg(feature = "http")]
  #[test]
  fn the_loads_that_wait_are_read_by_their_priority_and_the_newest_first() {
    let (address, asked) = serve_named(Duration::from_millis(60));
    let loader = loader();
    loader.pool_size(1);
    let load = |name: &str| loader.load(&Source::url(&format!("{address}/{name}")));
    // The one thread reads the first; the others wait their turn.
    let first = load("first");
    asked_for(&asked, 1);
    let (a, b, c) = (load("a"), load("b"), load("c"));
    let later = load("later").with_priority(LoadPriority::Low);
    assert_eq!(later.priority(), LoadPriority::Low);
    // What came into view goes before the rest, though it was asked for first.
    a.set_priority(LoadPriority::High);
    for load in [&first, &a, &b, &c, &later] {
      assert!(matches!(ended(load), LoadState::Ready(bytes) if bytes[..] == *b"abc"));
    }
    assert_eq!(
      *asked.lock(),
      ["/first", "/a", "/c", "/b", "/later"],
      "the one in view, then the newest, then the one for later"
    );
  }

  #[cfg(feature = "http")]
  #[test]
  fn a_load_nobody_waits_for_any_more_is_never_started() {
    let (address, asked) = serve_named(Duration::from_millis(600));
    let loader = loader();
    loader.pool_size(1);
    let load = |name: &str| loader.load(&Source::url(&format!("{address}/{name}")));
    let first = load("first");
    asked_for(&asked, 1);
    // A row scrolled past: asked for, and let go before its turn came.
    let gone = load("gone");
    let wake = Arc::new(AtomicUsize::new(0));
    let woken = wake.clone();
    gone.wake_with(Arc::new(move || {
      woken.fetch_add(1, Ordering::SeqCst);
    }));
    drop(gone);
    std::thread::sleep(IDLE_GRACE + Duration::from_millis(40));
    loader.sweep();
    let kept = load("kept");
    for load in [&first, &kept] {
      assert!(matches!(ended(load), LoadState::Ready(_)));
    }
    assert_eq!(*asked.lock(), ["/first", "/kept"], "the server was never asked for it");
    assert_eq!(loader.pending(), 0);
    // Asked for again, it is loaded as anything new is.
    assert!(matches!(ended(&load("gone")), LoadState::Ready(_)));
  }

  #[cfg(feature = "http")]
  #[test]
  fn the_loads_of_one_host_go_over_the_connection_the_first_opened() {
    let (address, served) = serve_kept(&[1, 2, 3], Duration::ZERO);
    let loader = loader();
    for index in 0..5 {
      let load = loader.load(&Source::url(&format!("{address}/picture-{index}.png")));
      assert!(matches!(ended(&load), LoadState::Ready(bytes) if bytes[..] == [1, 2, 3]));
    }
    assert_eq!(
      served.connections.load(Ordering::SeqCst),
      1,
      "one handshake for the five"
    );
    assert_eq!(loader.shared.agents.made(), 1, "and one agent");
  }

  #[cfg(feature = "http")]
  #[test]
  fn an_address_read_a_moment_ago_is_opened_again_with_no_request_until_its_start_is_forgotten() {
    let (address, asked) = serve_named(Duration::ZERO);
    let loader = Loader::new();
    let source = Source::url(&format!("{address}/clip.mp4"));
    let read = || {
      let reader = loader.open(&source).unwrap();
      let mut start = [0_u8; 8];
      let count = reader.read_at(0, &mut start).unwrap();
      start[..count].to_vec()
    };
    assert_eq!(read(), b"abc");
    assert_eq!(asked.lock().len(), 1);
    // Its reader is dropped, and its start kept: the next reader of it asks for nothing.
    assert_eq!(read(), b"abc");
    assert_eq!(asked.lock().len(), 1, "opened again with what the first read");
    assert!(loader.forget(&source), "there was a start to forget");
    assert_eq!(read(), b"abc");
    assert_eq!(asked.lock().len(), 2, "forgotten, it is asked for again");
    // Told to keep the start of none, every reader asks.
    loader.keep_starts(0);
    assert_eq!(read(), b"abc");
    assert_eq!(read(), b"abc");
    assert_eq!(asked.lock().len(), 4);
  }

  #[cfg(feature = "http")]
  #[test]
  fn no_connection_is_opened_ahead_to_an_address_the_app_does_not_allow() {
    let (address, asked) = serve_named(Duration::ZERO);
    let loader = Loader::new();
    loader.allow(|_| false);
    loader.connect(&format!("{address}/clip.mp4"));
    std::thread::sleep(Duration::from_millis(150));
    assert!(asked.lock().is_empty(), "nothing was sent");
  }

  #[cfg(feature = "http")]
  #[test]
  fn as_many_loads_are_read_at_once_as_the_pool_is_large_and_its_agents_are_kept() {
    let (address, served) = serve_kept(&[9; 64], Duration::from_millis(60));
    let loader = loader();
    let read_at_once = |size: usize, batch: &str| {
      loader.pool_size(size);
      served.most.store(0, Ordering::SeqCst);
      let loads: Vec<Load> = (0..size * 3)
        .map(|index| loader.load(&Source::url(&format!("{address}/{batch}-{index}.png"))))
        .collect();
      for load in &loads {
        assert!(matches!(ended(load), LoadState::Ready(bytes) if bytes.len() == 64));
      }
      served.most.load(Ordering::SeqCst)
    };
    assert_eq!(read_at_once(2, "two"), 2, "two at once");
    assert!(
      loader.shared.agents.made() <= 2,
      "{} agents",
      loader.shared.agents.made()
    );
    // Told again, the loads from then on are read as many at once.
    let most = read_at_once(3, "three");
    assert!((2..=3).contains(&most), "{most} at once");
    assert!(
      loader.shared.agents.made() <= 3,
      "{} agents",
      loader.shared.agents.made()
    );
    assert!(served.connections.load(Ordering::SeqCst) <= 3);
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
      the_start_of_a_source_is_read_and_no_more_and_is_no_error,
      the_same_source_asked_twice_is_one_load_and_wakes_who_waits,
      what_nobody_holds_is_let_go_after_a_moment_and_a_failure_is_kept,
      an_address_the_app_does_not_allow_is_denied_and_nothing_reads_it,
      a_reader_reads_a_part_of_a_file_and_of_bytes,
    );

    #[cfg(feature = "http")]
    again!(
      what_came_from_an_address_is_kept_in_the_folder_the_app_gave_and_read_from_there,
      the_loads_of_one_host_go_over_the_connection_the_first_opened,
      as_many_loads_are_read_at_once_as_the_pool_is_large_and_its_agents_are_kept,
      the_loads_that_wait_are_read_by_their_priority_and_the_newest_first,
      a_load_nobody_waits_for_any_more_is_never_started,
    );

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
        loader.shared.pool.lock().is_none(),
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
      assert!(loader.shared.pool.lock().is_none());
    }

    #[test]
    fn a_handle_given_later_is_where_the_next_load_is_read_and_one_taken_away_gives_the_pool_back() {
      let loader = Loader::new();
      read(&loader, "before-a-handle", 1);
      assert!(
        loader.shared.pool.lock().is_some(),
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
      assert!(loader.shared.pool.lock().is_none());
    }
  }
}
