//! Reading from a source at any place in it: what a picture reads to its end and a video reads a part of.

use std::{
  fs::File,
  io::{self, Read, Seek, SeekFrom},
  ops::Range,
  path::Path,
  sync::Arc,
};

use parking_lot::Mutex;

/// Something read at any offset, whose length may not be known. A picture reads all of it
/// ([`read_all`]); a video's decoder reads the parts it asks for, from its own threads, so reading blocks.
///
/// It is the one thing under every kind of source: a file, bytes in memory, and an address read by ranges.
pub trait Reader: Send + Sync {
  /// How many bytes there are, where that is known.
  fn len(&self) -> Option<u64>;

  /// Whether it is known to hold nothing.
  fn is_empty(&self) -> bool {
    self.len() == Some(0)
  }

  /// What it holds, as a MIME type (`video/mp4`), where it says: a server's `Content-Type`. `None` for a file and
  /// bytes, which a decoder tells from their name or their start.
  fn content_type(&self) -> Option<Arc<str>> {
    None
  }

  /// Reads into `buf` from `offset`, as many bytes as are there for it at once: 0 at the end. It waits for
  /// bytes that are on their way.
  fn read_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<usize>;

  /// The ranges it can give without waiting: what a seek bar draws as buffered. All of it, for a reader that
  /// has everything.
  fn buffered(&self) -> Vec<Range<u64>> {
    self.len().map(|len| 0..len).into_iter().collect()
  }

  /// Whether it fetches what follows a read before that is asked for. A reader of an address does unless told
  /// not to, so that what plays does not wait for each part; told not to, it fetches what it is asked for and
  /// no more, which is what a video opened ahead and left paused wants of a slow host. Told to again, it
  /// fetches what follows its last read at once. Nothing to a reader that has everything.
  fn read_ahead(&self, _ahead: bool) {}
}

/// A reader of bytes in memory.
pub struct BytesReader(Arc<[u8]>);

impl BytesReader {
  pub fn new(bytes: impl Into<Arc<[u8]>>) -> Self {
    Self(bytes.into())
  }
}

impl Reader for BytesReader {
  fn len(&self) -> Option<u64> {
    Some(self.0.len() as u64)
  }

  fn read_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
    let Ok(offset) = usize::try_from(offset) else {
      return Ok(0);
    };
    let rest = self.0.get(offset..).unwrap_or(&[]);
    let count = rest.len().min(buf.len());
    buf[..count].copy_from_slice(&rest[..count]);
    Ok(count)
  }
}

/// A reader of a file. Reads from several threads take turns: each seeks and reads under one lock.
pub struct FileReader {
  file: Mutex<File>,
  len: u64,
}

impl FileReader {
  pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
    let file = File::open(path)?;
    let len = file.metadata()?.len();
    Ok(Self {
      file: Mutex::new(file),
      len,
    })
  }
}

impl Reader for FileReader {
  fn len(&self) -> Option<u64> {
    Some(self.len)
  }

  fn read_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
    let mut file = self.file.lock();
    file.seek(SeekFrom::Start(offset))?;
    file.read(buf)
  }
}

/// Reads `reader` from its start to its end. `limit` is the most it may hold: one byte more is an error, so a
/// source larger than was meant is not read into memory whole. `told` hears how far it has come, after each
/// read, and stops the read by answering `false`.
pub fn read_all(
  reader: &dyn Reader,
  limit: Option<u64>,
  mut told: impl FnMut(u64, Option<u64>) -> bool,
) -> io::Result<Vec<u8>> {
  let total = reader.len();
  let too_large = |size: u64| limit.is_some_and(|limit| size > limit);
  if total.is_some_and(too_large) {
    return Err(io::Error::new(io::ErrorKind::FileTooLarge, "larger than the limit"));
  }
  let mut bytes = Vec::with_capacity(total.unwrap_or(0).min(64 * 1024 * 1024) as usize);
  let mut chunk = vec![0u8; 64 * 1024];
  loop {
    let read = reader.read_at(bytes.len() as u64, &mut chunk)?;
    if read == 0 {
      return Ok(bytes);
    }
    bytes.extend_from_slice(&chunk[..read]);
    if too_large(bytes.len() as u64) {
      return Err(io::Error::new(io::ErrorKind::FileTooLarge, "larger than the limit"));
    }
    if !told(bytes.len() as u64, total) {
      return Err(io::Error::new(io::ErrorKind::Interrupted, "the read was stopped"));
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn bytes_are_read_from_any_place() {
    let reader = BytesReader::new(vec![1u8, 2, 3, 4, 5]);
    assert_eq!(reader.len(), Some(5));
    let mut buf = [0u8; 2];
    assert_eq!(reader.read_at(3, &mut buf).unwrap(), 2);
    assert_eq!(buf, [4, 5]);
    assert_eq!(reader.read_at(4, &mut buf).unwrap(), 1, "one byte is left");
    assert_eq!(reader.read_at(5, &mut buf).unwrap(), 0, "the end");
    assert_eq!(reader.read_at(99, &mut buf).unwrap(), 0, "past the end");
    assert_eq!(reader.buffered(), vec![0..5]);
  }

  #[test]
  fn reading_everything_tells_how_far_it_came() {
    let reader = BytesReader::new(vec![7u8; 200_000]);
    let mut steps = Vec::new();
    let bytes = read_all(&reader, None, |loaded, total| {
      steps.push((loaded, total));
      true
    })
    .unwrap();
    assert_eq!(bytes.len(), 200_000);
    assert_eq!(steps.last(), Some(&(200_000, Some(200_000))));
    assert!(steps.len() > 1, "more than one read: {steps:?}");
  }

  #[test]
  fn a_source_over_the_limit_is_not_read() {
    let reader = BytesReader::new(vec![7u8; 1000]);
    let error = read_all(&reader, Some(999), |_, _| true).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::FileTooLarge);
    assert_eq!(read_all(&reader, Some(1000), |_, _| true).unwrap().len(), 1000);
  }

  #[test]
  fn a_read_that_is_stopped_ends() {
    let reader = BytesReader::new(vec![7u8; 500_000]);
    let error = read_all(&reader, None, |_, _| false).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Interrupted);
  }

  #[test]
  fn a_file_is_read_from_any_place() {
    let path = std::env::temp_dir().join(format!("lurq-reader-{}.bin", std::process::id()));
    std::fs::write(&path, [10u8, 20, 30, 40]).unwrap();
    let reader = FileReader::open(&path).unwrap();
    assert_eq!(reader.len(), Some(4));
    let mut buf = [0u8; 2];
    assert_eq!(reader.read_at(2, &mut buf).unwrap(), 2);
    assert_eq!(buf, [30, 40]);
    assert_eq!(read_all(&reader, None, |_, _| true).unwrap(), vec![10, 20, 30, 40]);
    drop(reader);
    std::fs::remove_file(&path).unwrap();
  }
}
