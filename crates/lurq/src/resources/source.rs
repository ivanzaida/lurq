//! Where something is loaded from: a file, bytes in memory, a resource of the app, an address.

use std::{
  fmt,
  hash::{Hash, Hasher},
  path::{Path, PathBuf},
  sync::Arc,
};

/// What a [`Source`] is.
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum SourceKind {
  File(PathBuf),
  Bytes(Arc<[u8]>),
  /// A path under the app's resource root.
  Resource(Arc<str>),
  /// An address on the network: `http://` or `https://`.
  Url(Arc<str>),
}

/// Where something is loaded from, and what its request carries. A picture, a video and a sound are given one
/// alike, and the app's [`Loader`](super::Loader) reads it.
///
/// ```
/// use lurq::resources::Source;
///
/// let photo = Source::file("photos/lake.jpg");
/// let logo = Source::resource("brand/logo.png");
/// let remote = Source::url("https://example.com/lake.jpg").header("Authorization", "Bearer token");
/// ```
#[derive(Clone)]
pub struct Source {
  pub(crate) kind: SourceKind,
  pub(crate) headers: Vec<(Arc<str>, Arc<str>)>,
  key: u64,
}

fn hash_of(tag: &str, hashed: impl Hash) -> u64 {
  let mut hasher = std::collections::hash_map::DefaultHasher::new();
  tag.hash(&mut hasher);
  hashed.hash(&mut hasher);
  hasher.finish()
}

/// Whether `text` is an address on the network.
fn is_address(text: &str) -> bool {
  text.starts_with("http://") || text.starts_with("https://")
}

impl Source {
  fn of(kind: SourceKind, key: u64) -> Self {
    Self {
      kind,
      headers: Vec::new(),
      key,
    }
  }

  /// A file. Its key is its path with its size and the time it was changed, so a file replaced on disk is
  /// another source and an unchanged one is the same.
  pub fn file(path: impl Into<PathBuf>) -> Self {
    let path = path.into();
    let key = Self::key_of_path(&path);
    Self::of(SourceKind::File(path), key)
  }

  fn key_of_path(path: &Path) -> u64 {
    let stamp = std::fs::metadata(path).ok().map(|metadata| {
      let changed = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok());
      (metadata.len(), changed)
    });
    hash_of("file", (path, stamp))
  }

  /// Bytes the app has. They are hashed once, here, for the key.
  pub fn bytes(bytes: impl Into<Arc<[u8]>>) -> Self {
    let bytes: Arc<[u8]> = bytes.into();
    let key = hash_of("bytes", &bytes[..]);
    Self::of(SourceKind::Bytes(bytes), key)
  }

  /// A resource of the app: a path under the root the app gave its loader. A text that is an address
  /// (`http://`, `https://`) is the address, as [`url`](Self::url).
  pub fn resource(path: &str) -> Self {
    if is_address(path) {
      return Self::url(path);
    }
    Self::of(SourceKind::Resource(Arc::from(path)), hash_of("resource", path))
  }

  /// An address on the network, `http://` or `https://`. Loading it needs the feature `http` outside a browser.
  pub fn url(url: &str) -> Self {
    Self::of(SourceKind::Url(Arc::from(url)), hash_of("url", url))
  }

  /// A header of this source's request, where it is an address: what tells the server who asks.
  pub fn header(mut self, name: &str, value: &str) -> Self {
    self.headers.push((Arc::from(name), Arc::from(value)));
    self
  }

  /// A key of the app's own in place of the one made here: the same key is the same content, whatever the
  /// source. An app that stores a hash of what it shows gives it, and bytes are then not hashed twice.
  pub fn key(mut self, key: u64) -> Self {
    self.key = key;
    self
  }

  /// What tells this source from another: what is loaded from it is kept and joined by it.
  pub fn cache_key(&self) -> u64 {
    self.key
  }

  /// Whether the source is an address on the network.
  pub fn is_remote(&self) -> bool {
    matches!(self.kind, SourceKind::Url(_))
  }
}

impl fmt::Debug for Source {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match &self.kind {
      SourceKind::File(path) => f.debug_tuple("File").field(path).finish(),
      SourceKind::Bytes(bytes) => f.debug_struct("Bytes").field("len", &bytes.len()).finish(),
      SourceKind::Resource(path) => f.debug_tuple("Resource").field(path).finish(),
      // The headers may hold a secret: they are not printed.
      SourceKind::Url(url) => f.debug_tuple("Url").field(url).finish(),
    }
  }
}

impl From<PathBuf> for Source {
  fn from(path: PathBuf) -> Self {
    Self::file(path)
  }
}

impl From<&Path> for Source {
  fn from(path: &Path) -> Self {
    Self::file(path)
  }
}

impl From<Vec<u8>> for Source {
  fn from(bytes: Vec<u8>) -> Self {
    Self::bytes(bytes)
  }
}

impl From<Arc<[u8]>> for Source {
  fn from(bytes: Arc<[u8]>) -> Self {
    Self::bytes(bytes)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_same_content_has_the_same_key_and_another_has_another() {
    assert_eq!(
      Source::bytes(vec![1, 2, 3]).cache_key(),
      Source::bytes(vec![1, 2, 3]).cache_key()
    );
    assert_ne!(
      Source::bytes(vec![1, 2, 3]).cache_key(),
      Source::bytes(vec![1, 2, 4]).cache_key()
    );
    assert_eq!(
      Source::resource("a.png").cache_key(),
      Source::resource("a.png").cache_key()
    );
    assert_ne!(Source::resource("a.png").cache_key(), Source::file("a.png").cache_key());
    assert_ne!(
      Source::resource("a.png").cache_key(),
      Source::url("https://example.com/a.png").cache_key()
    );
  }

  #[test]
  fn a_resource_that_is_an_address_is_the_address() {
    let source = Source::resource("https://example.com/a.png");
    assert!(source.is_remote());
    assert_eq!(source.cache_key(), Source::url("https://example.com/a.png").cache_key());
    assert!(!Source::resource("icons/a.png").is_remote());
  }

  #[test]
  fn a_key_of_the_apps_own_takes_the_place_of_the_one_made() {
    assert_eq!(Source::bytes(vec![1]).key(7).cache_key(), 7);
    assert_eq!(Source::url("https://example.com/a.png").key(7).cache_key(), 7);
  }

  #[test]
  fn headers_are_not_printed() {
    let source = Source::url("https://example.com/a.png").header("Authorization", "Bearer secret");
    assert!(!format!("{source:?}").contains("secret"));
  }
}
