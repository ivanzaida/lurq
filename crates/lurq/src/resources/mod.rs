//! Loading: where something comes from, what reads it, and the app's loader.
//!
//! - A [`Source`] is where something is loaded from: a file, bytes in memory, a resource of the app, an address. A
//!   picture, a video and a sound are given one alike.
//! - A [`Reader`] reads a source at any place in it. A picture reads it to its end; a video reads the parts its decoder
//!   asks for.
//! - The app's [`Loader`] reads sources off the UI thread: it holds the resource root and what the network may do,
//!   tells how far a load has come and why one failed, and makes one load of what is asked for twice.
//! - Its [`MediaDiskCache`] keeps what was loaded from addresses in a folder the app gave, for the next run of the app.
//!   It is off until the app gives the folder.
//!
//! A file, bytes and a resource are loaded by every build. An address needs the feature `http`, which adds the
//! client and its TLS; a build without it says so in the load's error.
//!
//! - What a page says about itself ([`LinkMeta`]) is read from its start by a [`LinkFetcher`]: lurq's own
//!   [`PageFetcher`] loads it through the loader, and keeps what it found in memory by the address.
//!
//! [`ResourceLoader`] is the loader before this one, which the feature `http` keeps: it is polled, holds the
//! bytes for one read, and is what a resource image and a background image still use.

#[cfg(feature = "http")]
mod core;
mod disk_cache;
#[cfg(all(feature = "http", not(target_arch = "wasm32")))]
mod http;
mod link_fetcher;
mod link_meta;
mod loader;
mod reader;
#[cfg(feature = "http")]
mod resource_cache;
#[cfg(feature = "http")]
mod resource_loader;
mod source;
mod thread_pool;
#[cfg(target_arch = "wasm32")]
mod web_fetch;

#[cfg(feature = "http")]
pub use core::{LoadResourceResult, ResourceConfig, ResourceError};

#[cfg(feature = "media")]
pub(crate) use disk_cache::WeakMediaDiskCache;
pub use disk_cache::{DEFAULT_MEDIA_DISK_CACHE_BYTES, MediaDiskCache};
pub(crate) use link_fetcher::fetch_off_thread;
pub use link_fetcher::{
  DEFAULT_LINK_HEAD_BYTES, LinkFetch, LinkFetcher, LinkPreviewError, LinkRequest, PageFetcher, link_fetcher,
  reset_link_fetcher, set_link_fetcher, wait_link_fetches,
};
pub use link_meta::LinkMeta;
pub use loader::{DEFAULT_LOAD_TIMEOUT, DEFAULT_MAX_LOAD_BYTES, Load, LoadError, LoadState, LoadWaker, Loader};
pub use reader::{BytesReader, FileReader, Reader, read_all};
#[cfg(feature = "http")]
pub use resource_loader::ResourceLoader;
pub use source::Source;
#[cfg_attr(not(feature = "image"), allow(unused_imports))]
pub(crate) use source::SourceKind;
