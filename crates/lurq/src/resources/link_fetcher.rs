//! What fetches what a page says about itself for a link preview, and keeps it.
//!
//! A preview asks a [`LinkFetcher`] for the [`LinkMeta`] of its address, and is told whether it is there yet
//! ([`LinkFetch`]); the fetcher wakes it once it has more, and it asks again. It asks off the UI thread, on a
//! thread of lurq's (in a browser, once the frame is drawn), and shows its skeleton until the fetcher answers: a
//! fetcher may block, reading a cache on the disk or asking a server of the app's, and the window is drawn on. What
//! a fetcher already holds it gives on the UI thread ([`LinkFetcher::held`]), and the preview shows it in its
//! first frame. lurq's own is [`PageFetcher`]: it reads the
//! start of the page through the app's [`Loader`] (a [`load_start`](Loader::load_start) of
//! [`PageFetcher::head_bytes`]), finds what the page says in it ([`LinkMeta::from_html`]), and keeps that in memory
//! by the address, so that a page shown twice is read once. Loading an address needs the feature `http`: a build
//! without it fetches nothing, and a preview shows what it was given and the address.
//!
//! An app gives a fetcher of its own where it knows better: its server's previews, a cache on the disk, a test's
//! pages. It is any `Fn(&str) -> LinkFetch`, or a type of the trait; it is given to one preview
//! (`ui::LinkPreview::fetcher`), or to every one that has none ([`set_link_fetcher`]).
//!
//! ```
//! use lurq::resources::{LinkFetch, LinkMeta, set_link_fetcher};
//!
//! set_link_fetcher(|url: &str| {
//!   LinkFetch::ready(LinkMeta::new(url).title("A page the app knew").site("example.com"))
//! });
//! ```

use std::{collections::HashMap, fmt, sync::Arc, time::Duration};

use parking_lot::{Condvar, Mutex, RwLock};

#[cfg(not(target_arch = "wasm32"))]
use super::thread_pool::ThreadPool;
use super::{Load, LoadError, LoadState, LoadWaker, Loader, Source, link_meta::LinkMeta};

/// How much of a page [`PageFetcher`] reads unless told: its first 512 KiB, which hold the head of a page that
/// puts a deal of script before its tags.
pub const DEFAULT_LINK_HEAD_BYTES: u64 = 512 * 1024;

/// Why what a page says about itself could not be had.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkPreviewError {
  /// The server has no page at the address.
  NotFound,
  /// The app's loader does not allow the address ([`Loader::allow`]).
  Denied,
  /// The page took longer than the app's loader waits.
  TimedOut,
  /// The server answered with this status.
  Server(u16),
  /// The connection failed, in the words of what failed.
  Network(Arc<str>),
  /// This build has nothing to fetch a page with, and why: one without the feature `http`, a browser.
  Unavailable(Arc<str>),
  /// Anything else, in the words of what failed.
  Other(Arc<str>),
}

impl LinkPreviewError {
  /// What the system or the connection said in its own words, where it said something: for a log, not for a
  /// person.
  pub fn detail(&self) -> Option<&str> {
    match self {
      Self::Network(detail) | Self::Unavailable(detail) | Self::Other(detail) => Some(detail),
      _ => None,
    }
  }

  /// The kind of a load that failed.
  pub fn of_load(error: &LoadError) -> Self {
    match error {
      LoadError::NotFound => Self::NotFound,
      LoadError::Denied => Self::Denied,
      LoadError::TimedOut => Self::TimedOut,
      LoadError::Status(status) => Self::Server(*status),
      LoadError::Unsupported(why) => Self::Unavailable(Arc::from(*why)),
      LoadError::Io(why) => Self::Network(Arc::from(why.as_str())),
      LoadError::TooLarge { .. } | LoadError::Cancelled => Self::Other(Arc::from(error.to_string())),
    }
  }
}

impl fmt::Display for LinkPreviewError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::NotFound => f.write_str("the page was not found"),
      Self::Denied => f.write_str("the app does not allow this address"),
      Self::TimedOut => f.write_str("the page took too long"),
      Self::Server(status) => write!(f, "the server answered {status}"),
      Self::Network(detail) | Self::Unavailable(detail) | Self::Other(detail) => f.write_str(detail),
    }
  }
}

impl std::error::Error for LinkPreviewError {}

/// Where the fetch of a page's [`LinkMeta`] is.
#[derive(Clone, Debug, PartialEq)]
pub enum LinkFetch {
  /// On its way: the fetcher wakes who asked once it has more.
  Loading,
  /// What the page says. Where it says nothing ([`LinkMeta::is_empty`]) a preview shows the address.
  Ready(Arc<LinkMeta>),
  Failed(LinkPreviewError),
}

impl LinkFetch {
  pub fn ready(meta: LinkMeta) -> Self {
    Self::Ready(Arc::new(meta))
  }
}

/// What a preview asks a [`LinkFetcher`] for.
pub struct LinkRequest<'a> {
  url: &'a str,
  loader: Option<&'a Loader>,
  wake: &'a LoadWaker,
}

impl<'a> LinkRequest<'a> {
  /// A request for `url`, from a preview in an app with `loader`, which `wake` renders again.
  pub fn new(url: &'a str, loader: Option<&'a Loader>, wake: &'a LoadWaker) -> Self {
    Self { url, loader, wake }
  }

  /// The address of the page.
  pub fn url(&self) -> &str {
    self.url
  }

  /// The loader of the app the preview is in: none outside an app.
  pub fn loader(&self) -> Option<&Loader> {
    self.loader
  }

  /// Asks the fetcher again for the preview: to be called once a fetch that answered [`LinkFetch::Loading`] has
  /// more. It may be called from any thread.
  pub fn wake(&self) -> &LoadWaker {
    self.wake
  }
}

/// What fetches what a page says about itself; see the [module](self).
pub trait LinkFetcher: Send + Sync {
  /// Where the fetch of `request`'s page is now. One not asked for before is started, and the request's waker is
  /// called once it comes further. Asked again for a page it has, it answers at once.
  ///
  /// A preview asks it off the UI thread, so it may block: the preview is its skeleton until it answers. Most
  /// fetchers start the work and answer [`LinkFetch::Loading`], as [`PageFetcher`] does; one that blocks holds one
  /// of the few threads previews ask on, and the previews after it wait for one.
  fn fetch(&self, request: &LinkRequest<'_>) -> LinkFetch;

  /// What it already holds of `url`, ready or failed, without blocking and without starting anything: asked on
  /// the UI thread as a preview renders, so that a page it holds is shown in the preview's first frame and not
  /// after a frame of its skeleton. `None`, as unless told, and a fetch still on its way, ask
  /// [`fetch`](Self::fetch) off the UI thread.
  fn held(&self, url: &str) -> Option<LinkFetch> {
    let _ = url;
    None
  }
}

/// A function of the address is a fetcher: an app's own previews, a test's pages, a cache on the disk.
impl<F: Fn(&str) -> LinkFetch + Send + Sync> LinkFetcher for F {
  fn fetch(&self, request: &LinkRequest<'_>) -> LinkFetch {
    self(request.url())
  }
}

enum Kept {
  Loading(Load),
  Ended(LinkFetch),
}

/// lurq's own fetcher: it reads the start of a page through the app's loader and keeps what the page says in
/// memory, by its address. A page that failed stays failed until it is [forgotten](Self::forget).
pub struct PageFetcher {
  head_bytes: RwLock<u64>,
  kept: Mutex<HashMap<Arc<str>, Kept>>,
}

impl Default for PageFetcher {
  fn default() -> Self {
    Self::new()
  }
}

impl PageFetcher {
  pub fn new() -> Self {
    Self {
      head_bytes: RwLock::new(DEFAULT_LINK_HEAD_BYTES),
      kept: Mutex::new(HashMap::new()),
    }
  }

  /// How much of a page is read: [`DEFAULT_LINK_HEAD_BYTES`] unless told. What a page says after it is not seen.
  pub fn head_bytes(&self, bytes: u64) -> &Self {
    *self.head_bytes.write() = bytes.max(1);
    self
  }

  /// Forgets what is kept of `url`, so that its page is read again when it is next asked for: the way to try
  /// again one that failed, or to see one that changed. `true` where something was kept.
  pub fn forget(&self, url: &str) -> bool {
    self.kept.lock().remove(url).is_some()
  }

  /// Forgets every page.
  pub fn clear(&self) {
    self.kept.lock().clear();
  }

  /// The request lurq makes of a page: what tells the server a page is wanted, and who asks. A text that is no
  /// address is a resource of the app.
  fn source(url: &str) -> Source {
    Source::resource(url)
      .header("Accept", "text/html,application/xhtml+xml;q=0.9,*/*;q=0.5")
      .header("User-Agent", "Mozilla/5.0 (compatible; lurq-link-preview)")
  }
}

impl LinkFetcher for PageFetcher {
  fn held(&self, url: &str) -> Option<LinkFetch> {
    match self.kept.lock().get(url) {
      Some(Kept::Ended(fetch)) => Some(fetch.clone()),
      _ => None,
    }
  }

  fn fetch(&self, request: &LinkRequest<'_>) -> LinkFetch {
    let url = request.url();
    let mut kept = self.kept.lock();
    let load = match kept.get(url) {
      Some(Kept::Ended(fetch)) => return fetch.clone(),
      Some(Kept::Loading(load)) => load.clone(),
      None => {
        let Some(loader) = request.loader() else {
          return LinkFetch::Failed(LinkPreviewError::Unavailable(Arc::from(
            "the preview is in no app: nothing loads its page",
          )));
        };
        let load = loader.load_start(&Self::source(url), *self.head_bytes.read());
        kept.insert(Arc::from(url), Kept::Loading(load.clone()));
        load
      }
    };
    // Asked to be told before it is looked at: a load that ends in between is not missed.
    load.wake_with(request.wake().clone());
    let ended = match load.state() {
      LoadState::Loading { .. } => return LinkFetch::Loading,
      LoadState::Ready(bytes) => LinkFetch::ready(LinkMeta::from_html(url, &bytes)),
      LoadState::Failed(error) => LinkFetch::Failed(LinkPreviewError::of_load(&error)),
    };
    kept.insert(Arc::from(url), Kept::Ended(ended.clone()));
    ended
  }
}

static FETCHER: RwLock<Option<Arc<dyn LinkFetcher>>> = RwLock::new(None);

/// Gives every link preview that has no fetcher of its own `fetcher`, in place of lurq's [`PageFetcher`].
pub fn set_link_fetcher(fetcher: impl LinkFetcher + 'static) {
  *FETCHER.write() = Some(Arc::new(fetcher));
}

/// Gives the previews lurq's own fetcher back.
pub fn reset_link_fetcher() {
  *FETCHER.write() = None;
}

/// The fetcher of a preview that has none of its own: the app's, where it set one, or lurq's [`PageFetcher`],
/// one for the process.
pub fn link_fetcher() -> Arc<dyn LinkFetcher> {
  if let Some(fetcher) = FETCHER.read().clone() {
    return fetcher;
  }
  static OWN: std::sync::OnceLock<Arc<PageFetcher>> = std::sync::OnceLock::new();
  OWN.get_or_init(|| Arc::new(PageFetcher::new())).clone()
}

/// How many threads ask fetchers for previews: a fetcher that blocks holds one, and the others go on.
#[cfg(not(target_arch = "wasm32"))]
const ASKING_THREADS: usize = 4;

/// The fetches asked from one thread: what [`wait_link_fetches`] waits for.
#[derive(Default)]
struct Asking {
  counts: Mutex<Counts>,
  answered: Condvar,
}

#[derive(Default)]
struct Counts {
  /// Asked, and not answered yet.
  pending: usize,
  /// Answered since the thread last waited.
  answered: usize,
}

thread_local! {
  static ASKED_HERE: Arc<Asking> = Arc::new(Asking::default());
}

/// Counts a fetch as answered once it has, or once its job was dropped untaken.
struct Answered(Arc<Asking>);

impl Drop for Answered {
  fn drop(&mut self) {
    let mut counts = self.0.counts.lock();
    counts.pending = counts.pending.saturating_sub(1);
    counts.answered += 1;
    if counts.pending == 0 {
      self.0.answered.notify_all();
    }
  }
}

/// Asks `fetcher` for the page at `url` off the calling thread, as a preview in an app with `loader` that `wake`
/// asks again, and gives `answered` what it said. A fetcher that panics has failed.
pub(crate) fn fetch_off_thread(
  fetcher: Arc<dyn LinkFetcher>,
  url: Arc<str>,
  loader: Option<Loader>,
  wake: LoadWaker,
  answered: impl FnOnce(LinkFetch) + Send + 'static,
) {
  let asking = ASKED_HERE.with(Arc::clone);
  asking.counts.lock().pending += 1;
  let counted = Answered(asking);
  run_off_thread(move || {
    let _counted = counted;
    let asked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
      fetcher.fetch(&LinkRequest::new(&url, loader.as_ref(), &wake))
    }));
    answered(asked.unwrap_or_else(|_| {
      tracing::error!(target: "lurq::resources", "a link fetcher panicked on {url}");
      LinkFetch::Failed(LinkPreviewError::Other(Arc::from("the link fetcher panicked")))
    }));
  });
}

/// On one of the threads previews ask on.
#[cfg(not(target_arch = "wasm32"))]
fn run_off_thread(job: impl FnOnce() + Send + 'static) {
  static POOL: std::sync::OnceLock<ThreadPool> = std::sync::OnceLock::new();
  POOL.get_or_init(|| ThreadPool::new(ASKING_THREADS)).execute(job);
}

/// A browser has one thread: the job runs once the task that rendered the preview has returned.
#[cfg(target_arch = "wasm32")]
fn run_off_thread(job: impl FnOnce() + Send + 'static) {
  wasm_bindgen_futures::spawn_local(async move { job() });
}

/// Waits until every fetch that the link previews rendered on this thread asked for has answered, for at most
/// `timeout`: for a test that draws previews and then looks at what they show, or a tool that takes a picture
/// of a page. `Some` with how many answered since this thread last waited, which a frame drawn now shows (0: the
/// previews show what they were told); `None` where they had not all answered in time. A browser cannot wait:
/// there it is `None` while one is on its way.
pub fn wait_link_fetches(timeout: Duration) -> Option<usize> {
  let asking = ASKED_HERE.with(Arc::clone);
  let mut counts = asking.counts.lock();
  if !answered_in(&asking, &mut counts, timeout) {
    return None;
  }
  Some(std::mem::take(&mut counts.answered))
}

/// Whether every fetch counted in `counts` has answered within `timeout`.
#[cfg(not(target_arch = "wasm32"))]
fn answered_in(asking: &Asking, counts: &mut parking_lot::MutexGuard<'_, Counts>, timeout: Duration) -> bool {
  let deadline = std::time::Instant::now() + timeout;
  while counts.pending > 0 {
    if asking.answered.wait_until(counts, deadline).timed_out() {
      return counts.pending == 0;
    }
  }
  true
}

/// A browser's one thread cannot wait for the jobs it is to run itself.
#[cfg(target_arch = "wasm32")]
fn answered_in(_: &Asking, counts: &mut parking_lot::MutexGuard<'_, Counts>, _: Duration) -> bool {
  counts.pending == 0
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
  use super::*;

  fn waker() -> LoadWaker {
    Arc::new(|| {})
  }

  #[test]
  fn a_page_in_a_file_is_read_once_and_kept_by_its_address() {
    let folder = std::env::temp_dir().join(format!("lurq-link-fetcher-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(
      folder.join("page.html"),
      b"<head><title>Kept</title><meta name=description content='Read once.'></head>",
    )
    .unwrap();
    let loader = Loader::new();
    loader.root(&folder);
    let fetcher = PageFetcher::new();
    let wake = waker();
    // A resource of the app is read as an address is, through the loader.
    let request = LinkRequest::new("page.html", Some(&loader), &wake);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let meta = loop {
      match fetcher.fetch(&request) {
        LinkFetch::Ready(meta) => break meta,
        LinkFetch::Loading => {
          assert!(std::time::Instant::now() < deadline, "the page does not come");
          std::thread::sleep(std::time::Duration::from_millis(2));
        }
        LinkFetch::Failed(error) => panic!("{error}"),
      }
    };
    assert_eq!(meta.title.as_deref(), Some("Kept"));
    // Held: given at once, as a preview renders.
    assert_eq!(fetcher.held("page.html"), Some(LinkFetch::Ready(meta.clone())));
    assert_eq!(meta.description.as_deref(), Some("Read once."));
    // Kept: the file gone, it is still what it was.
    std::fs::remove_file(folder.join("page.html")).unwrap();
    assert_eq!(fetcher.fetch(&request), LinkFetch::Ready(meta));
    assert!(fetcher.forget("page.html"));
    assert_eq!(fetcher.held("page.html"), None);
    assert!(!fetcher.forget("page.html"));
  }

  #[test]
  fn a_request_from_no_app_fails_and_a_function_answers_at_once() {
    let wake = waker();
    let request = LinkRequest::new("https://lurq.dev", None, &wake);
    assert!(matches!(
      PageFetcher::new().fetch(&request),
      LinkFetch::Failed(LinkPreviewError::Unavailable(_))
    ));
    let known = |url: &str| LinkFetch::ready(LinkMeta::new(url).title("Known"));
    assert!(matches!(known.fetch(&request), LinkFetch::Ready(meta) if meta.title.as_deref() == Some("Known")));
  }

  #[test]
  fn a_failed_load_is_its_kind() {
    assert_eq!(
      LinkPreviewError::of_load(&LoadError::NotFound),
      LinkPreviewError::NotFound
    );
    assert_eq!(
      LinkPreviewError::of_load(&LoadError::Status(503)),
      LinkPreviewError::Server(503)
    );
    let network = LinkPreviewError::of_load(&LoadError::Io("reset".to_owned()));
    assert_eq!(network.detail(), Some("reset"));
  }
}
