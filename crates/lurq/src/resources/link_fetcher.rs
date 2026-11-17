//! What fetches what a page says about itself for a link preview, and keeps it.
//!
//! A preview asks a [`LinkFetcher`] for the [`LinkMeta`] of its address as it renders, and is told whether it is
//! there yet ([`LinkFetch`]); the fetcher wakes it once it has more. lurq's own is [`PageFetcher`]: it reads the
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

use std::{collections::HashMap, fmt, sync::Arc};

use parking_lot::{Mutex, RwLock};

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

  /// Renders the preview again: to be called once a fetch that answered [`LinkFetch::Loading`] has more.
  pub fn wake(&self) -> &LoadWaker {
    self.wake
  }
}

/// What fetches what a page says about itself; see the [module](self).
pub trait LinkFetcher: Send + Sync {
  /// Where the fetch of `request`'s page is now. One not asked for before is started, and the request's waker is
  /// called once it comes further. Asked again for a page it has, it answers at once.
  fn fetch(&self, request: &LinkRequest<'_>) -> LinkFetch;
}

/// A function of the address is a fetcher that answers at once: an app's own previews, a test's pages.
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
    assert_eq!(meta.description.as_deref(), Some("Read once."));
    // Kept: the file gone, it is still what it was.
    std::fs::remove_file(folder.join("page.html")).unwrap();
    assert_eq!(fetcher.fetch(&request), LinkFetch::Ready(meta));
    assert!(fetcher.forget("page.html"));
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
