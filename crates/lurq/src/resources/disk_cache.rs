//! What the app's loader loaded from addresses, kept as files in a folder the app gave: it is there for the next
//! run of the app, and asked for again it is read from the folder and not from the network.
//!
//! It is off until the app gives the folder ([`Loader::enable_media_disk_cache`](super::Loader), which
//! `App::enable_media_disk_cache` calls): lurq picks no folder itself. Off, nothing is written, and what a sound
//! loaded is kept in memory (`media::sound_cache()`).
//!
//! A file is named by its source's [`cache_key`](Source::cache_key) and its length, `9f3a41c2d07be815-8a3f10.mp3`:
//! one whose size is not the length in its name was cut short by somebody, and is deleted and loaded again. It
//! is written under another name and renamed when it is whole, so a half-written file is never read. Over the
//! budget, the one used longest ago is deleted; when a file was last used is the time the system says it was
//! changed, which is set at each use. Only files with such names are counted, read and deleted: anything else
//! in the folder is left alone.
//!
//! What is kept is what the server sent, for as long as it fits: nothing asks the server whether it has changed,
//! and nothing is let go for being old. An app that knows a source changed forgets it.

use std::{
  collections::HashMap,
  fs,
  io::Write,
  path::{Path, PathBuf},
  sync::{
    Arc, Weak,
    atomic::{AtomicU64, Ordering},
  },
};

use parking_lot::{Mutex, MutexGuard};

use super::Source;

/// How many bytes are kept in the folder unless the app says: 512 MiB.
pub const DEFAULT_MEDIA_DISK_CACHE_BYTES: u64 = 512 * 1024 * 1024;

/// The ending of a file that is being written.
const PART: &str = "part";
/// A file being written that was last touched this long ago was left by a run of the app that ended meanwhile.
#[cfg(not(target_arch = "wasm32"))]
const STALE_PART: std::time::Duration = std::time::Duration::from_secs(60 * 60);
/// The endings of an address that are kept in the name of its file: the ones of a sound or a video, by which a
/// system's player may tell what the file is. No other ending is written, so nothing in the folder has a name
/// the system would run.
const ENDINGS: [&str; 20] = [
  "3gp", "aac", "aif", "aiff", "amr", "caf", "flac", "m4a", "m4b", "m4v", "mka", "mov", "mp3", "mp4", "oga", "ogg",
  "opus", "wav", "weba", "webm",
];

/// A file kept.
struct Entry {
  /// Its name in the folder.
  name: String,
  len: u64,
  /// The turn it was last used at: what has the lowest was used longest ago.
  turn: u64,
}

struct Kept {
  /// The folder the app gave. `None`: the cache is off.
  folder: Option<PathBuf>,
  files: HashMap<u64, Entry>,
  bytes: u64,
  budget: u64,
  /// Counts each use.
  turn: u64,
}

impl Kept {
  /// The file kept under `key`, which is used by this: its path, its length, and whether it was not already the
  /// one used last, so that the time of its use is to be written.
  fn used(&mut self, key: u64) -> Option<(PathBuf, u64, bool)> {
    let folder = self.folder.as_ref()?;
    let entry = self.files.get_mut(&key)?;
    let moved = entry.turn != self.turn;
    if moved {
      self.turn += 1;
      entry.turn = self.turn;
    }
    Some((folder.join(&entry.name), entry.len, moved))
  }

  /// Deletes what was used longest ago until what is kept is within the budget. A file the system does not let
  /// go of stays, and is counted.
  fn fit(&mut self) {
    if self.bytes <= self.budget {
      return;
    }
    let Some(folder) = self.folder.clone() else {
      return;
    };
    let mut oldest: Vec<(u64, u64)> = self.files.iter().map(|(key, entry)| (entry.turn, *key)).collect();
    oldest.sort_unstable();
    for (_, key) in oldest {
      if self.bytes <= self.budget {
        return;
      }
      if self
        .files
        .get(&key)
        .is_some_and(|entry| gone(&folder.join(&entry.name)))
        && let Some(entry) = self.files.remove(&key)
      {
        self.bytes -= entry.len;
      }
    }
  }
}

struct Shared {
  kept: Mutex<Kept>,
}

/// The files the app's loader keeps of what it loaded from addresses; see the [module](self). Every loader has
/// one, which is off until it is given a folder; a clone is the same cache.
///
/// ```no_run
/// use lurq::{app::App, resources::Source};
///
/// let app = App::new().enable_media_disk_cache("cache");
/// let kept = app.loader().media_disk_cache();
/// // A player of long recordings keeps more of them, and lets go of one it knows has changed.
/// kept.budget(2 * 1024 * 1024 * 1024);
/// kept.forget(&Source::url("https://example.com/episode-12.mp3"));
/// ```
#[derive(Clone)]
pub struct MediaDiskCache {
  shared: Arc<Shared>,
}

/// A [`MediaDiskCache`] that does not keep it alive: what follows the cache of an app that may be gone.
#[cfg_attr(not(feature = "media"), allow(dead_code))]
#[derive(Clone)]
pub(crate) struct WeakMediaDiskCache(Weak<Shared>);

#[cfg_attr(not(feature = "media"), allow(dead_code))]
impl WeakMediaDiskCache {
  pub(crate) fn upgrade(&self) -> Option<MediaDiskCache> {
    self.0.upgrade().map(|shared| MediaDiskCache { shared })
  }
}

impl MediaDiskCache {
  /// A cache that is off.
  pub(crate) fn new() -> Self {
    Self {
      shared: Arc::new(Shared {
        kept: Mutex::new(Kept {
          folder: None,
          files: HashMap::new(),
          bytes: 0,
          budget: DEFAULT_MEDIA_DISK_CACHE_BYTES,
          turn: 0,
        }),
      }),
    }
  }

  fn kept(&self) -> MutexGuard<'_, Kept> {
    self.shared.kept.lock()
  }

  #[cfg_attr(not(feature = "media"), allow(dead_code))]
  pub(crate) fn downgrade(&self) -> WeakMediaDiskCache {
    WeakMediaDiskCache(Arc::downgrade(&self.shared))
  }

  /// Whether `other` is this cache.
  #[cfg_attr(not(feature = "media"), allow(dead_code))]
  pub(crate) fn same(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.shared, &other.shared)
  }

  /// Turns the cache on, in `folder`: the folder is made where there is none, and what a run before this one
  /// kept in it is found. A file whose size is not the length in its name is deleted, and so is one that was
  /// being written when a run of the app ended. Given another folder later, the cache goes on in that one, and
  /// the files of the first stay where they are. Nothing is kept in a browser, which has a cache of its own.
  pub(crate) fn turn_on(&self, folder: PathBuf) {
    #[cfg(target_arch = "wasm32")]
    let _ = folder;
    #[cfg(not(target_arch = "wasm32"))]
    {
      if let Err(error) = fs::create_dir_all(&folder) {
        tracing::warn!(
          "the folder of the media disk cache, {}, could not be made: {error}",
          folder.display()
        );
      }
      let now = std::time::SystemTime::now();
      let mut found = Vec::new();
      for file in fs::read_dir(&folder).into_iter().flatten().flatten() {
        let (Ok(name), Ok(metadata)) = (file.file_name().into_string(), file.metadata()) else {
          continue;
        };
        if !metadata.is_file() {
          continue;
        }
        let used = metadata.modified().unwrap_or(std::time::UNIX_EPOCH);
        if is_part(&name) {
          if now.duration_since(used).is_ok_and(|age| age >= STALE_PART) {
            let _ = fs::remove_file(file.path());
          }
          continue;
        }
        let Some((key, len)) = parse(&name) else {
          continue;
        };
        if metadata.len() == len {
          found.push((used, key, name, len));
        } else {
          let _ = fs::remove_file(file.path());
        }
      }
      // In the order they were used: the turns of this run go on from the last of them.
      found.sort();
      let mut kept = self.kept();
      kept.files.clear();
      kept.bytes = 0;
      kept.turn = 0;
      for (_, key, name, len) in found {
        kept.turn += 1;
        let turn = kept.turn;
        // Two files of one source: the one used last is kept.
        if let Some(older) = kept.files.insert(key, Entry { name, len, turn }) {
          kept.bytes -= older.len;
          let _ = fs::remove_file(folder.join(&older.name));
        }
        kept.bytes += len;
      }
      kept.folder = Some(folder);
    }
  }

  /// The folder the app gave. `None`: the cache is off.
  pub fn folder(&self) -> Option<PathBuf> {
    self.kept().folder.clone()
  }

  /// Whether the app gave a folder.
  pub fn is_on(&self) -> bool {
    self.kept().folder.is_some()
  }

  /// How many bytes are kept in the folder at most: 512 MiB unless told. It may be told before the cache is
  /// turned on. With less than is kept now, what was used longest ago is deleted at once; with none, nothing is
  /// kept. What a run before this one left over the budget is deleted when something more is kept, or when the
  /// budget is told: an app that keeps more than 512 MiB loses nothing by turning the cache on first.
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

  /// How many files are kept now.
  pub fn len(&self) -> usize {
    self.kept().files.len()
  }

  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  /// Whether what `source` loads is kept. It answers from what the cache knows, and reads nothing: a file
  /// somebody else deleted is found missing when it is next opened, and loaded again.
  pub fn holds(&self, source: &Source) -> bool {
    self.kept().files.contains_key(&source.cache_key())
  }

  /// Deletes the file of `source`, so that it is loaded from its address again when it is next asked for.
  /// `true` where it was kept. A file that plays goes on playing to its end.
  pub fn forget(&self, source: &Source) -> bool {
    self.forget_key(source.cache_key())
  }

  /// Deletes every file the cache keeps. Nothing else in the folder is touched.
  pub fn clear(&self) {
    let mut kept = self.kept();
    if let Some(folder) = kept.folder.clone() {
      for entry in kept.files.values() {
        let _ = fs::remove_file(folder.join(&entry.name));
      }
    }
    kept.files.clear();
    kept.bytes = 0;
  }

  /// [`forget`](Self::forget), by the source's key.
  pub(crate) fn forget_key(&self, key: u64) -> bool {
    let mut kept = self.kept();
    let Some(entry) = kept.files.remove(&key) else {
      return false;
    };
    kept.bytes -= entry.len;
    if let Some(folder) = &kept.folder {
      let _ = fs::remove_file(folder.join(&entry.name));
    }
    true
  }

  /// Lets go of the file `path` kept under `key`, which was found missing or cut short.
  fn broken(&self, key: u64, path: &Path) {
    let mut kept = self.kept();
    let this = kept
      .folder
      .as_ref()
      .zip(kept.files.get(&key))
      .is_some_and(|(folder, entry)| folder.join(&entry.name) == path);
    if this && let Some(entry) = kept.files.remove(&key) {
      kept.bytes -= entry.len;
      let _ = fs::remove_file(path);
    }
  }

  /// The file kept under `key`, for what plays it from where it is: a sound. It is used by this. `None` where
  /// nothing is kept, and where the file is gone or is not as long as it was written: it is then forgotten, to
  /// be loaded again.
  #[cfg_attr(not(feature = "media"), allow(dead_code))]
  pub(crate) fn file(&self, key: u64) -> Option<PathBuf> {
    let used = self.kept().used(key);
    let (path, len, moved) = used?;
    match fs::metadata(&path) {
      Ok(metadata) if metadata.is_file() && metadata.len() == len => {
        if moved {
          touch(&path);
        }
        Some(path)
      }
      _ => {
        self.broken(key, &path);
        None
      }
    }
  }

  /// The bytes kept under `key`, for what reads them whole: a picture. `None` where nothing is kept, where there
  /// is more than `limit`, and where the file is gone or cut short.
  #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
  pub(crate) fn read(&self, key: u64, limit: u64) -> Option<Vec<u8>> {
    let (path, len, moved) = {
      let mut kept = self.kept();
      if kept.files.get(&key).is_none_or(|entry| entry.len > limit) {
        return None;
      }
      kept.used(key)?
    };
    match fs::read(&path) {
      Ok(bytes) if bytes.len() as u64 == len => {
        if moved {
          touch(&path);
        }
        Some(bytes)
      }
      _ => {
        self.broken(key, &path);
        None
      }
    }
  }

  /// Keeps `bytes`, which came from `address`, under `key`, and answers the file they are in. They are written
  /// under another name and renamed when they are all there. `None` where they are not kept: the cache is off,
  /// they are more than the whole budget, or the folder cannot be written.
  #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
  pub(crate) fn put(&self, key: u64, address: &str, bytes: &[u8]) -> Option<PathBuf> {
    let len = bytes.len() as u64;
    let folder = {
      let kept = self.kept();
      if len > kept.budget {
        return None;
      }
      kept.folder.clone()?
    };
    let name = name_of(key, len, address);
    let path = folder.join(&name);
    // The same file is there already: it is not written twice.
    let there = self.kept().files.get(&key).is_some_and(|entry| entry.name == name);
    if there && let Some(there) = self.file(key) {
      return Some(there);
    }
    static WRITTEN: AtomicU64 = AtomicU64::new(0);
    let part = folder.join(format!(
      "{key:016x}-{:x}-{:x}.{PART}",
      std::process::id(),
      WRITTEN.fetch_add(1, Ordering::Relaxed)
    ));
    let written = fs::create_dir_all(&folder)
      .and_then(|()| fs::File::create(&part))
      .and_then(|mut file| {
        file.write_all(bytes)?;
        // On the disk before it has its name: a file with the name is whole after the power went, too.
        file.sync_all()
      })
      .and_then(|()| fs::rename(&part, &path));
    if let Err(error) = written {
      let _ = fs::remove_file(&part);
      // Another loader of the folder may have put the same file there first, and hold it open.
      if !fs::metadata(&path).is_ok_and(|metadata| metadata.len() == len) {
        static SAID: std::sync::Once = std::sync::Once::new();
        SAID.call_once(|| {
          tracing::warn!(
            "the media disk cache could not write to {}: {error}. What is loaded is not kept there",
            folder.display()
          );
        });
        return None;
      }
    }
    let mut kept = self.kept();
    // The app gave another folder meanwhile: the file is whole, and not this cache's to count.
    if kept.folder.as_deref() != Some(folder.as_path()) {
      return Some(path);
    }
    kept.turn += 1;
    let turn = kept.turn;
    let entry = Entry {
      name: name.clone(),
      len,
      turn,
    };
    if let Some(old) = kept.files.insert(key, entry) {
      kept.bytes -= old.len;
      if old.name != name {
        let _ = fs::remove_file(folder.join(&old.name));
      }
    }
    kept.bytes += len;
    kept.fit();
    Some(path)
  }
}

/// Deletes the file at `path`. Whether it is gone: deleted now, or not there.
fn gone(path: &Path) -> bool {
  match fs::remove_file(path) {
    Ok(()) => true,
    Err(error) => error.kind() == std::io::ErrorKind::NotFound,
  }
}

/// Writes that the file at `path` was used now, as the time it was changed: what the next run of the app tells
/// the one used longest ago by. Where the system refuses, the file keeps the time it had.
fn touch(path: &Path) {
  #[cfg(target_arch = "wasm32")]
  let _ = path;
  #[cfg(not(target_arch = "wasm32"))]
  {
    let mut options = fs::OpenOptions::new();
    // Windows: the right to write a file's times alone, which is given while the file plays; the right to
    // write the file is not.
    #[cfg(windows)]
    {
      use std::os::windows::fs::OpenOptionsExt;
      const FILE_WRITE_ATTRIBUTES: u32 = 0x0100;
      options.access_mode(FILE_WRITE_ATTRIBUTES);
    }
    #[cfg(not(windows))]
    options.write(true);
    let _ = options
      .open(path)
      .and_then(|file| file.set_modified(std::time::SystemTime::now()));
  }
}

/// The ending of what is at `address`, where it is one of [`ENDINGS`].
fn ending(address: &str) -> Option<&'static str> {
  let path = address.split(['?', '#']).next().unwrap_or(address);
  let path = path.split_once("://").map_or(path, |(_, rest)| rest);
  // What follows the host: a host's own ending (`.com`) is not a file's.
  let (_, path) = path.split_once('/')?;
  let (_, ending) = path.rsplit('/').next()?.rsplit_once('.')?;
  ENDINGS.into_iter().find(|known| known.eq_ignore_ascii_case(ending))
}

/// The name of the file that keeps the `len` bytes of the source `key`, which came from `address`.
fn name_of(key: u64, len: u64, address: &str) -> String {
  match ending(address) {
    Some(ending) => format!("{key:016x}-{len:x}.{ending}"),
    None => format!("{key:016x}-{len:x}"),
  }
}

fn is_hex(text: &str, most: usize) -> bool {
  (1..=most).contains(&text.len()) && text.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

/// The key and the length in the name of a file this cache wrote. `None` for any other name.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn parse(name: &str) -> Option<(u64, u64)> {
  let stem = match name.split_once('.') {
    Some((stem, ending)) => ENDINGS.contains(&ending).then_some(stem)?,
    None => name,
  };
  let (key, len) = stem.split_once('-')?;
  if key.len() != 16 || !is_hex(key, 16) || !is_hex(len, 16) {
    return None;
  }
  Some((u64::from_str_radix(key, 16).ok()?, u64::from_str_radix(len, 16).ok()?))
}

/// Whether `name` is of a file this cache was writing: `key-process-count.part`.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn is_part(name: &str) -> bool {
  let Some(stem) = name.strip_suffix(PART).and_then(|stem| stem.strip_suffix('.')) else {
    return false;
  };
  let mut parts = stem.split('-');
  let named = parts.next().is_some_and(|key| key.len() == 16 && is_hex(key, 16))
    && parts.next().is_some_and(|process| is_hex(process, 16))
    && parts.next().is_some_and(|count| is_hex(count, 16));
  named && parts.next().is_none()
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
  use std::time::{Duration, SystemTime};

  use super::*;

  /// An empty folder of this test's own.
  fn folder(test: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("lurq-disk-cache-{}-{test}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    folder
  }

  /// A cache turned on in `folder`: what a run of the app has.
  fn run(folder: &Path) -> MediaDiskCache {
    let cache = MediaDiskCache::new();
    cache.turn_on(folder.to_path_buf());
    cache
  }

  /// The names of the files in `folder`, in order.
  fn names(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(folder)
      .unwrap()
      .map(|file| file.unwrap().file_name().into_string().unwrap())
      .collect();
    names.sort();
    names
  }

  /// Says the file at `path` was last used `seconds` ago.
  fn used_ago(path: &Path, seconds: u64) {
    let file = fs::OpenOptions::new().write(true).open(path).unwrap();
    file
      .set_modified(SystemTime::now() - Duration::from_secs(seconds))
      .unwrap();
  }

  const SONG: &str = "https://example.com/songs/night-drive.mp3?token=1";

  #[test]
  fn a_file_is_named_by_its_key_and_its_length_with_the_ending_of_a_sound_alone() {
    assert_eq!(name_of(0x9f3a, 0x8a3f10, SONG), "0000000000009f3a-8a3f10.mp3");
    assert_eq!(name_of(1, 2, "https://example.com/a.MP3#t=4"), "0000000000000001-2.mp3");
    // Not a sound's ending, a host's ending, and no ending: none is written.
    for address in [
      "https://example.com/setup.exe",
      "https://example.com",
      "https://example.com/",
      "https://example.com/track/12",
      "https://example.com/a.b/track",
    ] {
      assert_eq!(name_of(1, 2, address), "0000000000000001-2", "{address}");
    }
    assert_eq!(parse("0000000000009f3a-8a3f10.mp3"), Some((0x9f3a, 0x8a3f10)));
    assert_eq!(parse("0000000000000001-2"), Some((1, 2)));
    for other in [
      "notes.txt",
      "0000000000000001-2.exe",
      "0000000000000001-2.mp3.exe",
      "000000000000001-2",
      "0000000000000001-",
      "0000000000000001-2-3",
      "000000000000000G-2",
      "0000000000000001-1-2.part",
    ] {
      assert_eq!(parse(other), None, "{other}");
    }
    assert!(is_part("0000000000000001-1f4-2.part"));
    assert!(!is_part("0000000000000001-2") && !is_part("download.part") && !is_part("0000000000000001-2.part"));
  }

  #[test]
  fn off_it_keeps_nothing_and_writes_nothing() {
    let cache = MediaDiskCache::new();
    assert!(!cache.is_on() && cache.folder().is_none());
    assert_eq!(cache.put(1, SONG, &[1, 2, 3]), None);
    assert_eq!((cache.file(1), cache.read(1, 100)), (None, None));
    assert!(!cache.forget_key(1) && cache.is_empty());
    cache.clear();
  }

  #[test]
  fn what_is_kept_is_a_whole_file_in_the_folder_and_is_there_for_the_next_run() {
    let folder = folder("kept");
    let cache = run(&folder);
    assert!(cache.is_on() && cache.is_empty());
    let path = cache.put(7, SONG, &[1, 2, 3, 4]).expect("it is kept");
    assert_eq!(names(&folder), ["0000000000000007-4.mp3"], "and nothing half-written");
    assert_eq!(fs::read(&path).unwrap(), [1, 2, 3, 4]);
    assert_eq!((cache.len(), cache.bytes()), (1, 4));
    assert!(cache.holds(&Source::url(SONG).key(7)));
    assert_eq!(cache.file(7), Some(path.clone()));
    assert_eq!(cache.read(7, 100), Some(vec![1, 2, 3, 4]));
    assert_eq!(cache.read(7, 3), None, "more than the loader reads whole");
    // Kept again, the same: the file is the one that was there.
    assert_eq!(cache.put(7, SONG, &[1, 2, 3, 4]), Some(path.clone()));
    assert_eq!((cache.len(), cache.bytes()), (1, 4));

    let next = run(&folder);
    assert_eq!((next.len(), next.bytes()), (1, 4));
    assert_eq!(next.file(7), Some(path));
    assert_eq!(next.read(7, 100), Some(vec![1, 2, 3, 4]));
  }

  #[test]
  fn what_a_source_loads_anew_takes_the_place_of_what_it_was() {
    let folder = folder("anew");
    let cache = run(&folder);
    cache.put(7, SONG, &[1, 2, 3, 4]);
    cache.put(7, SONG, &[5, 6]);
    assert_eq!(names(&folder), ["0000000000000007-2.mp3"]);
    assert_eq!((cache.len(), cache.bytes()), (1, 2));
    assert_eq!(cache.read(7, 100), Some(vec![5, 6]));
  }

  #[test]
  fn over_the_budget_the_one_used_longest_ago_is_deleted() {
    let folder = folder("budget");
    let cache = run(&folder);
    cache.budget(10);
    cache.put(1, SONG, &[0; 4]);
    cache.put(2, SONG, &[0; 4]);
    // The first is used again, so the second is the one used longest ago.
    assert!(cache.file(1).is_some());
    cache.put(3, SONG, &[0; 4]);
    assert_eq!(names(&folder), ["0000000000000001-4.mp3", "0000000000000003-4.mp3"]);
    assert_eq!((cache.len(), cache.bytes()), (2, 8));
    // What is larger than the whole budget is not kept, and a smaller budget deletes at once.
    assert_eq!(cache.put(4, SONG, &[0; 11]), None);
    assert!(cache.file(1).is_some());
    cache.budget(5);
    assert_eq!(names(&folder), ["0000000000000001-4.mp3"]);
    cache.budget(0);
    assert!(names(&folder).is_empty() && cache.is_empty());
  }

  #[test]
  fn the_next_run_knows_which_was_used_longest_ago_and_deletes_nothing_by_being_turned_on() {
    let folder = folder("next-run");
    let cache = run(&folder);
    let first = cache.put(1, SONG, &[0; 4]).unwrap();
    let second = cache.put(2, SONG, &[0; 4]).unwrap();
    used_ago(&first, 300);
    used_ago(&second, 200);
    // The first is used: that is written to the file, where the next run finds it.
    assert!(cache.file(1).is_some());

    // A budget less than the folder holds, and the cache turned on: an app that tells a larger one next has
    // lost nothing.
    let next = MediaDiskCache::new();
    next.budget(4);
    next.turn_on(folder.clone());
    assert_eq!((next.len(), next.bytes()), (2, 8));
    next.budget(10);
    assert_eq!(next.len(), 2);
    next.put(3, SONG, &[0; 4]);
    assert_eq!(names(&folder), ["0000000000000001-4.mp3", "0000000000000003-4.mp3"]);
  }

  #[test]
  fn forgetting_deletes_the_file_and_clearing_deletes_them_all_and_nothing_else() {
    let folder = folder("forget");
    let cache = run(&folder);
    fs::write(folder.join("notes.txt"), b"the app's own").unwrap();
    cache.put(1, SONG, &[0; 4]);
    cache.put(2, SONG, &[0; 4]);
    assert!(cache.forget(&Source::url(SONG).key(1)));
    assert!(!cache.forget(&Source::url(SONG).key(1)));
    assert_eq!(names(&folder), ["0000000000000002-4.mp3", "notes.txt"]);
    assert_eq!((cache.len(), cache.bytes()), (1, 4));
    cache.clear();
    assert_eq!(names(&folder), ["notes.txt"]);
    assert_eq!((cache.len(), cache.bytes()), (0, 0));
    // The next run counts and deletes its own files alone.
    let next = run(&folder);
    next.budget(0);
    assert_eq!(names(&folder), ["notes.txt"]);
  }

  #[test]
  fn a_file_that_is_gone_or_cut_short_is_forgotten_and_not_given() {
    let folder = folder("broken");
    let cache = run(&folder);
    let gone = cache.put(1, SONG, &[0; 8]).unwrap();
    let cut = cache.put(2, SONG, &[0; 8]).unwrap();
    let read = cache.put(3, SONG, &[0; 8]).unwrap();
    fs::remove_file(&gone).unwrap();
    fs::write(&cut, [0; 3]).unwrap();
    fs::write(&read, [0; 3]).unwrap();
    assert!(cache.holds(&Source::url(SONG).key(1)), "it reads nothing to answer");
    assert_eq!((cache.file(1), cache.file(2), cache.read(3, 100)), (None, None, None));
    assert_eq!((cache.len(), cache.bytes()), (0, 0));
    assert!(names(&folder).is_empty(), "what was cut short is deleted");

    // Cut short between two runs: the next run finds the size is not the length in the name.
    let kept = cache.put(4, SONG, &[0; 8]).unwrap();
    fs::write(&kept, [0; 3]).unwrap();
    let next = run(&folder);
    assert!(next.is_empty() && names(&folder).is_empty());
  }

  #[test]
  fn what_a_run_that_ended_was_writing_is_deleted_and_what_is_being_written_now_is_left() {
    let folder = folder("parts");
    fs::create_dir_all(&folder).unwrap();
    let stale = folder.join("0000000000000001-1f4-0.part");
    let fresh = folder.join("0000000000000002-1f4-1.part");
    fs::write(&stale, [0; 4]).unwrap();
    fs::write(&fresh, [0; 4]).unwrap();
    used_ago(&stale, 2 * 60 * 60);
    let cache = run(&folder);
    assert!(cache.is_empty());
    assert_eq!(names(&folder), ["0000000000000002-1f4-1.part"]);
  }

  #[test]
  fn two_files_of_one_source_are_one_the_next_run_and_it_is_the_one_used_last() {
    let folder = folder("two");
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("0000000000000001-4.mp3"), [1; 4]).unwrap();
    fs::write(folder.join("0000000000000001-2.mp3"), [2; 2]).unwrap();
    used_ago(&folder.join("0000000000000001-4.mp3"), 60);
    let cache = run(&folder);
    assert_eq!(names(&folder), ["0000000000000001-2.mp3"]);
    assert_eq!((cache.len(), cache.bytes()), (1, 2));
  }

  #[test]
  fn many_threads_keeping_the_same_source_leave_one_whole_file() {
    let folder = folder("threads");
    let cache = run(&folder);
    let bytes: Arc<[u8]> = Arc::from(vec![7_u8; 100_000]);
    let threads: Vec<_> = (0..8)
      .map(|_| {
        let (cache, bytes) = (cache.clone(), bytes.clone());
        std::thread::spawn(move || cache.put(9, SONG, &bytes).expect("it is kept"))
      })
      .collect();
    for thread in threads {
      thread.join().unwrap();
    }
    assert_eq!(names(&folder), ["0000000000000009-186a0.mp3"]);
    assert_eq!((cache.len(), cache.bytes()), (1, 100_000));
    assert_eq!(cache.read(9, 1 << 20).as_deref(), Some(&bytes[..]));
  }
}
