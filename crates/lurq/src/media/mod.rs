//! Playing a video or a sound: a [`Media`] opened from a [`Source`], its picture, its sound, and where it is.
//!
//! A media decodes on a thread of its own with the system's decoder (Media Foundation on Windows), keeps its own
//! clock, and puts each frame into an image a [`Video`](crate::components::Video) draws: the window is drawn
//! again for a new frame, and no component is rendered for it. On macOS and iOS the system's player (`AVPlayer`)
//! decodes and is heard, and the media's thread takes its frames and follows its clock; that player is the main
//! thread's, so a video opens there in a process whose main thread runs its loop, as an app's does. Its sound goes to
//! the system's default output, and the media's clock then follows what is heard. What holds a media tells it to play,
//! to pause, to go to a time, how fast to go and how loud to be, asks it where it is ([`Media::state`]), and is woken
//! when that changes in a way a clock does not tell: it opened, it ended, it failed.
//!
//! What happens to it is told to who listens, on the UI thread, once each time it happens: it opened
//! ([`Media::on_load`]), it failed ([`Media::on_error`]), it really started to play ([`Media::on_started`]), it
//! played to its end ([`Media::on_ended`]), it began or stopped playing ([`Media::on_playing_change`]) and waiting
//! for what it plays ([`Media::on_waiting_change`]).
//!
//! [`Media::open`] is what is seen, with its sound; [`Media::sound`] what is only heard: a song, a recording, a
//! message. A sound at an address is loaded whole through the app's loader and kept, so that it plays again at
//! once: in memory ([`sound_cache`]), or as a file in the folder of an app that turned the cache on the disk on
//! (`App::enable_media_disk_cache`), where it is there for the next run of the app too.
//!
//! `ui::VideoPlayer::source` and `ui::AudioPlayer::source` do all of this for a player. An app that draws its
//! own asks for a media itself.
//!
//! The headers of a source at an address (the loader's [rule](Loader::headers), then [`Source::header`]) are on
//! every request of what is read as it plays. On Windows the app's loader reads every address
//! ([`Loader::open`], by ranges, the feature `http`) and the decoder only decodes: where it has nothing left to
//! show while the part it needs is on the network, the media waits (and not while it reads ahead), and a read the
//! server refuses is why the media fails. On macOS and iOS the system's player reads it, given the headers. A
//! browser's `<audio>` cannot send them.
//!
//! On Windows, where the `wgpu` renderer draws on Direct3D 12, a frame stays on the graphics card from the
//! decoder to the screen: decoded there, copied there once, drawn there. Elsewhere it is copied through memory
//! once.
//!
//! What is not here yet: a speed that keeps the pitch of the sound on Windows (faster is higher; Apple's players
//! keep it); on macOS and iOS, an address read through the app's loader (the system's player reads it itself),
//! and a clip its track turns is drawn as it was filmed.

use std::{
  collections::VecDeque,
  sync::{Arc, Condvar, Mutex, MutexGuard},
};

pub use crate::core::{MediaError, MediaInfo};
use crate::{
  images::ImageData,
  resources::{Load, LoadError, LoadState, Loader, MediaDiskCache, Reader, Source, SourceKind},
  time::{Duration, Instant},
};

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod apple;
mod cache;
mod driven;
mod events;
#[cfg(windows)]
mod mf;
mod session;

/// Has the next frame a media shows on a card find its device removed, as a card that is reset removes it.
#[cfg(all(test, windows))]
pub(crate) fn lose_card_for_test() {
  mf::LOSE_CARD.store(true, std::sync::atomic::Ordering::Release);
}
#[cfg(windows)]
mod output;
#[cfg(target_arch = "wasm32")]
mod web;

pub use cache::{DEFAULT_SOUND_CACHE_BYTES, SoundCache, sound_cache};
#[doc(hidden)]
pub use driven::{DrivenMedias, MediaDriver, drive_media};
pub(crate) use events::{LoopNoted, note_loop, tell_news};
pub use session::AudioCategory;
pub(crate) use session::set_category as set_audio_category;

/// What changed in a media is told to these, from its own thread.
type Waker = Arc<dyn Fn() + Send + Sync>;

/// Where a media is, and what it is: read with [`Media::state`].
#[derive(Clone, Debug, PartialEq)]
pub struct MediaState {
  /// How long it is, in seconds, once that is known. `None` for what has no end, and before it opened.
  pub duration: Option<f64>,
  /// Where it is, in seconds.
  pub position: f64,
  /// How far it is fetched from where it is, in seconds: the end of what is there to play from its position with
  /// nothing more asked of the network. All of a file, of bytes and of a sound that was loaded whole. For an
  /// address the app's loader reads as it plays (Windows), the end of the ranges it holds in a row from the
  /// position, taken by the share of its bytes: it grows as ranges come, and after it is sent to a part that is
  /// not fetched it is no further than there. `None` where that is not known: before it opened, for what has no
  /// length, and for an address the system's own player reads (macOS, iOS, a browser).
  pub buffered: Option<f64>,
  /// How large its picture is, once it opened. `None` for what has no picture, and for what is only heard.
  pub size: Option<(u32, u32)>,
  /// Whether it has sound, once it opened.
  pub sound: bool,
  /// How loud its sound is, from 0 to 1, and whether it is silenced.
  pub volume: f32,
  pub muted: bool,
  /// Whether it plays: it was told to, and has not ended or failed.
  pub playing: bool,
  /// Whether it waits for what it plays: opening, or reading what is not there yet.
  pub waiting: bool,
  /// Whether it played to its end.
  pub ended: bool,
  /// Why it cannot play, where it cannot: the kind of failure, which the app's
  /// [`Locale`](crate::core::Locale) puts into words ([`MediaWords::error`](crate::core::MediaWords::error)).
  pub error: Option<MediaError>,
  /// How fast it goes: 1 is as it was made.
  pub rate: f64,
}

/// What a load that failed is, as a media says it.
fn load_failed(error: LoadError) -> MediaError {
  match error {
    LoadError::NotFound => MediaError::NotFound,
    LoadError::Denied => MediaError::Denied,
    LoadError::Status(status) => MediaError::Server(status),
    LoadError::TooLarge { .. } => MediaError::TooLarge,
    LoadError::TimedOut => MediaError::TimedOut,
    LoadError::Unsupported(why) => MediaError::Unavailable(Arc::from(why)),
    LoadError::Io(said) => MediaError::Network(Arc::from(said)),
    LoadError::Cancelled => MediaError::Other(Arc::from("the load was stopped")),
  }
}

/// What the thread of a media is told.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Command {
  Play,
  Pause,
  Seek(f64),
  /// A seek to the frame nearest a time that the media shows quickest: see [`Media::seek_near`].
  SeekNear(f64),
  Rate(f64),
  Volume(f32),
  Muted(bool),
  Close,
}

/// What a media and its thread share.
pub(crate) struct Shared {
  inner: Mutex<Inner>,
  commands: Mutex<VecDeque<Command>>,
  /// Told when a command comes: the thread waits on it while it has nothing to decode.
  told: Condvar,
  wakers: Mutex<Vec<Waker>>,
  /// Its handlers of events and what it last told them, once something listens.
  listening: std::sync::OnceLock<Arc<events::Listening>>,
  /// Why a read of what it plays failed, where the app's loader reads it.
  read_failed: Mutex<Option<LoadError>>,
  /// Since when its thread waits for the decoder's next sample, while it does.
  decoding_since: Mutex<Option<Instant>>,
  /// The reads of what the app's loader reads that wait for the network.
  reads_waiting: std::sync::atomic::AtomicUsize,
  /// It waits because its decoder waited on the network with nothing left to show: until the sample comes.
  starved: std::sync::atomic::AtomicBool,
  /// What it presents next is its start, which who listens is to be told: see [`Shared::presented`].
  pub(crate) awaits_start: std::sync::atomic::AtomicBool,
  /// It was told to play, once: what it reads of an address is fetched ahead from then on, and until then only
  /// what its decoder asks for, so that a video opened ahead and left paused takes its start and no more.
  eager: std::sync::atomic::AtomicBool,
  /// What the app's loader reads of it, where it plays an address.
  stream: Mutex<Option<Arc<dyn Reader>>>,
  /// All of what it plays is there: a file, or bytes.
  whole: std::sync::atomic::AtomicBool,
}

/// How far a media `duration` long is fetched from `position`, in seconds, where `held` are the ranges of bytes
/// there of the `len` it has: the end of the range the position is in, by the share of its bytes. Where the
/// position is in none, it is fetched no further than the position.
///
/// A file's index stands before what it indexes, so the bytes of a time are a little after its share: a range
/// that begins a little after the position's share is taken to hold it.
#[cfg_attr(not(windows), allow(dead_code))]
fn fetched_to(position: f64, duration: f64, len: u64, held: &[std::ops::Range<u64>]) -> f64 {
  if duration <= 0.0 || len == 0 {
    return position;
  }
  let position = position.clamp(0.0, duration);
  let at = ((position / duration) * len as f64) as u64;
  let slack = (len / 50).max(1 << 20);
  let mut end = None;
  for range in held {
    // The ranges are in order: one that takes up where the last ended goes on the run.
    match end {
      None if range.start <= at.saturating_add(slack) && at < range.end => end = Some(range.end),
      Some(run) if range.start <= run => end = Some(run.max(range.end)),
      _ => {}
    }
  }
  match end {
    Some(end) if end >= len => duration,
    Some(end) => (end as f64 / len as f64 * duration).max(position),
    None => position,
  }
}

/// The state of a media, and the clock it is read from: where it was at `since`, going at `rate` while it plays.
pub(crate) struct Inner {
  pub duration: Option<f64>,
  pub size: Option<(u32, u32)>,
  pub sound: bool,
  /// How loud its sound is, from 0 to 1, and whether it is silenced: what its output starts with.
  pub volume: f32,
  pub muted: bool,
  /// Whether it opened and is ready to play: what it is told of is known.
  pub opened: bool,
  pub playing: bool,
  pub waiting: bool,
  pub ended: bool,
  pub error: Option<MediaError>,
  pub rate: f64,
  /// At its end it goes on from its start, in what it has open: see [`Media::set_looped`].
  pub looped: bool,
  /// How many times it went on from its start at its end, looped: each is told as an end ([`Media::on_ended`]).
  pub rounds: u64,
  /// How many times it showed a frame, or gave its output sound where it has no picture: what it presents after
  /// it could move is its start ([`Media::on_started`]).
  pub presented: u64,
  /// How far it is fetched from where it is, in seconds, where what plays it says so itself: a media a test
  /// drives. What the app's loader reads, and a file, are told from what is held ([`Media::state`]).
  pub fetched: Option<f64>,
  pub at: f64,
  pub since: Instant,
  /// How many times it was sent somewhere ([`Media::seek`]). The thread counts the ones it has done, and does
  /// not set the clock by what is heard while one is on its way: that would be where it was.
  pub sent: u64,
  pub frame: Option<ImageData>,
}

impl Inner {
  /// Where it is now, in seconds.
  pub fn position(&self) -> f64 {
    let moved = if self.playing && !self.waiting {
      self.since.elapsed().as_secs_f64() * self.rate
    } else {
      0.0
    };
    let position = (self.at + moved).max(0.0);
    match self.duration {
      Some(duration) => position.min(duration),
      None => position,
    }
  }

  /// Holds the clock at where it is, to start again from there: before what moves it changes.
  pub fn hold(&mut self) {
    self.at = self.position();
    self.since = Instant::now();
  }
}

impl Shared {
  /// What a media and its thread share, as a media starts: waiting to open, paused at its start.
  fn new() -> Arc<Self> {
    Arc::new(Self {
      inner: Mutex::new(Inner {
        duration: None,
        size: None,
        sound: false,
        volume: 1.0,
        muted: false,
        opened: false,
        playing: false,
        waiting: true,
        ended: false,
        error: None,
        rate: 1.0,
        looped: false,
        rounds: 0,
        presented: 0,
        fetched: None,
        at: 0.0,
        since: Instant::now(),
        sent: 0,
        frame: None,
      }),
      commands: Mutex::new(VecDeque::new()),
      told: Condvar::new(),
      wakers: Mutex::new(Vec::new()),
      listening: std::sync::OnceLock::new(),
      read_failed: Mutex::new(None),
      decoding_since: Mutex::new(None),
      reads_waiting: std::sync::atomic::AtomicUsize::new(0),
      starved: std::sync::atomic::AtomicBool::new(false),
      awaits_start: std::sync::atomic::AtomicBool::new(false),
      eager: std::sync::atomic::AtomicBool::new(false),
      stream: Mutex::new(None),
      whole: std::sync::atomic::AtomicBool::new(false),
    })
  }

  pub fn inner(&self) -> MutexGuard<'_, Inner> {
    self.inner.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
  }

  /// Tells who listens that the media changed.
  pub fn wake(&self) {
    let wakers = self
      .wakers
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner())
      .clone();
    for waker in wakers {
      waker();
    }
    self.noted();
  }

  /// It showed a frame, or gave its output sound where it has no picture. Who listens for its start is woken by
  /// the one that is the start, and by no other: a media that plays presents many times a second.
  pub(crate) fn presented(&self) {
    let mut inner = self.inner();
    self.presented_in(&mut inner);
    let first = inner.presented == 1;
    drop(inner);
    // Its picture is there from now on ([`Media::has_frame`]): what draws it is told once.
    if first {
      self.wake();
    }
  }

  /// [`presented`](Self::presented), for what holds the media's state already.
  pub(crate) fn presented_in(&self, inner: &mut Inner) {
    inner.presented += 1;
    if self.awaits_start.swap(false, std::sync::atomic::Ordering::AcqRel) {
      self.noted();
    }
  }

  /// Tells the thread that listens to the media's events that it changed, where something listens: its next turn
  /// looks at it.
  fn noted(&self) {
    if let Some(listening) = self.listening.get() {
      listening.note();
    }
  }

  /// The commands a driven media was told and has not done, taken where `take`.
  fn take_driven(&self, take: bool) -> Vec<Command> {
    let mut commands = self.commands.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if take {
      commands.drain(..).collect()
    } else {
      commands.iter().copied().collect()
    }
  }

  /// It cannot play, and why.
  pub fn fail(&self, why: MediaError) {
    {
      let mut inner = self.inner();
      inner.hold();
      // What the app's loader could not read is why the decoder failed, in the server's words.
      let read = self
        .read_failed
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take();
      inner.error = Some(read.map_or(why, load_failed));
      inner.playing = false;
      inner.waiting = false;
    }
    self.wake();
  }

  /// The media's thread waits for its decoder's next sample, which it asks for only when it has nothing else to
  /// show or to give the output, until what this gives is dropped. Where that takes longer than
  /// [`STARVED_AFTER`] while a read waits for the network, the media waits: it is starved, and not merely reading
  /// ahead.
  #[cfg_attr(not(windows), allow(dead_code))]
  pub(crate) fn decoding(&self) -> Decoding<'_> {
    *self
      .decoding_since
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Instant::now());
    Decoding(self)
  }

  /// Whether its decoder has waited for a sample longer than [`STARVED_AFTER`] while a read waits for the
  /// network.
  #[cfg_attr(not(windows), allow(dead_code))]
  fn is_starved(&self) -> bool {
    let since = *self
      .decoding_since
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner());
    since.is_some_and(|since| since.elapsed() >= STARVED_AFTER)
      && self.reads_waiting.load(std::sync::atomic::Ordering::Acquire) > 0
  }

  /// Whether it waits for what it plays: once it opened, the media waits while it is starved, and its clock
  /// stands.
  #[cfg_attr(not(windows), allow(dead_code))]
  fn set_reading(&self, waits: bool) {
    {
      let mut inner = self.inner();
      if !inner.opened || inner.error.is_some() || inner.waiting == waits {
        return;
      }
      inner.hold();
      inner.waiting = waits;
    }
    self.wake();
  }

  /// The commands that came, in order. With `wait`, it waits that long for one where there is none.
  pub fn take(&self, wait: Option<Duration>) -> Vec<Command> {
    let mut commands = self.commands.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if commands.is_empty()
      && let Some(wait) = wait
    {
      commands = self
        .told
        .wait_timeout(commands, wait)
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .0;
    }
    commands.drain(..).collect()
  }

  fn tell(&self, command: Command) {
    self
      .commands
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner())
      .push_back(command);
    self.told.notify_all();
    // A page has no thread of the media's: what it is told is done now.
    #[cfg(target_arch = "wasm32")]
    web::told(self);
  }

  /// Whether nothing holds the media any more. It waits `wait` to be told so, and takes no command.
  fn closed(&self, wait: Duration) -> bool {
    let is = |commands: &VecDeque<Command>| commands.contains(&Command::Close);
    let commands = self.commands.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if is(&commands) {
      return true;
    }
    let commands = self
      .told
      .wait_timeout(commands, wait)
      .unwrap_or_else(|poisoned| poisoned.into_inner())
      .0;
    is(&commands)
  }
}

/// What tells the thread to end once nothing holds the media.
struct Handle {
  shared: Arc<Shared>,
}

impl Drop for Handle {
  fn drop(&mut self) {
    self.shared.tell(Command::Close);
  }
}

/// A video or a sound that plays. It is cheap to clone, and its thread ends when the last of it is dropped.
///
/// ```no_run
/// use lurq::{
///   components::Video,
///   media::Media,
///   resources::{Loader, Source},
/// };
///
/// # fn show(loader: &Loader) {
/// let media = Media::open(loader, &Source::file("clips/lake.mp4"));
/// media.play();
/// // Its picture, once it opened: drawn again by itself as frames come.
/// let picture = media.frame().map(Video::new);
/// let seconds = media.state().position;
///
/// // A song: only heard, and kept once it is loaded from its address.
/// let song = Media::sound(loader, &Source::url("https://example.com/song.mp3"));
/// song.set_volume(0.8);
/// song.play();
/// # }
/// ```
#[derive(Clone)]
pub struct Media(Arc<Handle>);

/// The headers of a source's requests.
pub(crate) type Headers = Arc<[(Arc<str>, Arc<str>)]>;

/// What a source is, for what opens it: the file it is read from, the bytes it is, or its address.
#[derive(Clone)]
pub(crate) enum Opened {
  File(std::path::PathBuf),
  /// The bytes, and the name they came by where they have one: its ending tells the decoder what they are.
  Bytes(Arc<[u8]>, Option<Arc<str>>),
  /// An address the system's player reads itself, with the headers its requests are to carry: on Apple's
  /// systems, in a browser, and on Windows in a build without the feature `http`.
  Url(Arc<str>, Headers),
  /// What the app's loader reads ([`Loader::open`]), and the address it is at, whose ending tells the decoder
  /// what it is where its type does not.
  #[cfg_attr(not(windows), allow(dead_code))]
  Reader(Arc<dyn Reader>, Arc<str>),
}

/// What a media opens of `source` at `address`, read as it plays. On Windows, macOS and iOS the app's loader reads
/// it, and the decoder only decodes; elsewhere the system's player reads it, given the headers. `None` where it
/// cannot be opened, which the media's state then says.
fn stream(shared: &Arc<Shared>, loader: &Loader, source: &Source, address: Arc<str>) -> Option<Opened> {
  #[cfg(any(windows, target_os = "macos", target_os = "ios"))]
  match loader.open_to_play(source, shared.eager.load(std::sync::atomic::Ordering::Acquire)) {
    // A stream of HTTP Live Streaming is a list of the addresses of its parts, which the system's player reads
    // itself on Apple's systems: it is given the address.
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    Ok(reader) if is_playlist(reader.content_type().as_deref(), &address) => drop(reader),
    Ok(reader) => {
      // Until it is told to play it reads what its decoder asks for and nothing ahead of that. Kept before
      // that is read: one told to play meanwhile turns the reading ahead on itself.
      *shared.stream.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(reader.clone());
      reader.read_ahead(shared.eager.load(std::sync::atomic::Ordering::Acquire));
      return Some(Opened::Reader(Arc::new(Watched::new(reader, shared)), address));
    }
    // A build without the feature `http`: Media Foundation reads the address itself, and sends no header.
    Err(LoadError::Unsupported(_)) => {}
    Err(error) => {
      shared.fail(load_failed(error));
      return None;
    }
  }
  let _ = shared;
  Some(Opened::Url(address, Arc::from(loader.headers_of(source))))
}

/// Whether what a server says is `content_type`, at `address`, is a playlist of HTTP Live Streaming.
#[cfg_attr(not(any(target_os = "macos", target_os = "ios")), allow(dead_code))]
fn is_playlist(content_type: Option<&str>, address: &str) -> bool {
  let kind = content_type.map(str::to_ascii_lowercase);
  let path = address.split(['?', '#']).next().unwrap_or(address).to_ascii_lowercase();
  kind.is_some_and(|kind| kind.contains("mpegurl")) || path.ends_with(".m3u8") || path.ends_with(".m3u")
}

/// A reader of what a media plays: it counts the reads that wait for the network, from which the media tells it is
/// starved, and the reason a read failed is the reason the media fails.
#[cfg_attr(not(windows), allow(dead_code))]
struct Watched {
  reader: Arc<dyn Reader>,
  shared: std::sync::Weak<Shared>,
}

#[cfg_attr(not(windows), allow(dead_code))]
impl Watched {
  fn new(reader: Arc<dyn Reader>, shared: &Arc<Shared>) -> Self {
    watch_for_starving(shared);
    Self {
      reader,
      shared: Arc::downgrade(shared),
    }
  }
}

/// How long the decoder may wait for its next sample while a read waits for the network before the media says it
/// waits: a block that comes at once shows no spinner.
const STARVED_AFTER: Duration = Duration::from_millis(250);

/// Looks a few times a second, for as long as the media lives, whether it is starved: its thread has waited for
/// the decoder's next sample longer than [`STARVED_AFTER`] while a read waits for the network. Then it waits, and
/// its clock stands, until the sample comes.
#[cfg_attr(not(windows), allow(dead_code))]
fn watch_for_starving(shared: &Arc<Shared>) {
  use std::sync::atomic::Ordering;
  let shared = Arc::downgrade(shared);
  let _ = std::thread::Builder::new()
    .name("lurq-media-starving".to_owned())
    .spawn(move || {
      loop {
        std::thread::sleep(std::time::Duration::from_millis(40));
        let Some(shared) = shared.upgrade() else {
          return;
        };
        if shared.is_starved() && !shared.starved.swap(true, Ordering::AcqRel) {
          shared.set_reading(true);
        }
      }
    });
}

/// The media's thread waits for the decoder's next sample while this lives: see [`Shared::decoding`].
pub(crate) struct Decoding<'a>(&'a Shared);

impl Drop for Decoding<'_> {
  fn drop(&mut self) {
    *self
      .0
      .decoding_since
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
    if self.0.starved.swap(false, std::sync::atomic::Ordering::AcqRel) {
      self.0.set_reading(false);
    }
  }
}

impl Reader for Watched {
  fn len(&self) -> Option<u64> {
    self.reader.len()
  }

  fn content_type(&self) -> Option<Arc<str>> {
    self.reader.content_type()
  }

  fn buffered(&self) -> Vec<std::ops::Range<u64>> {
    self.reader.buffered()
  }

  fn read_at(&self, offset: u64, buf: &mut [u8]) -> std::io::Result<usize> {
    use std::sync::atomic::Ordering;
    let there = self.reader.buffered().iter().any(|range| range.contains(&offset));
    let shared = self.shared.upgrade();
    // A read on the network is counted, and no more: the decoder reads ahead of what plays, and only one it
    // waits for while it has nothing to show makes the media wait (`Shared::decoding`).
    if !there && let Some(shared) = &shared {
      shared.reads_waiting.fetch_add(1, Ordering::AcqRel);
    }
    let read = self.reader.read_at(offset, buf);
    if !there && let Some(shared) = &shared {
      shared.reads_waiting.fetch_sub(1, Ordering::AcqRel);
    }
    read.map_err(|error| {
      let why = LoadError::from(error);
      if let Some(shared) = &shared {
        *shared
          .read_failed
          .lock()
          .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(why.clone());
      }
      std::io::Error::other(why)
    })
  }
}

/// What a media starts from: what is there to open, or a load of it that is on its way.
enum Asked {
  Opened(Opened),
  /// The file `disk` keeps under `key`: a sound that was loaded before, by this run of the app or an earlier.
  Kept {
    path: std::path::PathBuf,
    key: u64,
    disk: MediaDiskCache,
  },
  /// The bytes at `address`, loaded whole by the app's loader and kept under `key`: by `disk` where it is on,
  /// and in memory where it is not. Where they are more than the loader reads whole, `source` is read as it
  /// plays.
  Load {
    load: Load,
    key: u64,
    address: Arc<str>,
    disk: MediaDiskCache,
    loader: Loader,
    source: Source,
  },
  /// `source`, at `address`, read as it plays: opened on the media's thread, since opening asks the server.
  Stream {
    loader: Loader,
    source: Source,
    address: Arc<str>,
  },
}

/// Waits for `load` to end. `None` once nothing holds the media.
fn loaded(shared: &Arc<Shared>, load: &Load) -> Option<Result<Arc<[u8]>, LoadError>> {
  // The load wakes the thread when it ends, and the thread looks again a few times a second besides.
  load.wake_with({
    let shared = shared.clone();
    Arc::new(move || shared.told.notify_all())
  });
  loop {
    match load.state() {
      LoadState::Ready(bytes) => return Some(Ok(bytes)),
      LoadState::Failed(error) => return Some(Err(error)),
      LoadState::Loading { .. } => {}
    }
    if shared.closed(Duration::from_millis(50)) {
      return None;
    }
  }
}

/// What `asked` is once it is there to open: a load is waited for, and what it loaded is kept. With it, the
/// cache on the disk and the key of the file that is opened, where it is one that cache keeps. `None` where
/// there is nothing to open: the media was let go, or the load failed, which the media's state then says.
fn settle(shared: &Arc<Shared>, asked: Asked) -> Option<(Opened, Option<(MediaDiskCache, u64)>)> {
  match asked {
    Asked::Opened(opened) => Some((opened, None)),
    Asked::Kept { path, key, disk } => Some((Opened::File(path), Some((disk, key)))),
    Asked::Load {
      load,
      key,
      address,
      disk,
      loader,
      source,
    } => match loaded(shared, &load)? {
      // The app's loader wrote the file as it loaded it; where it did not (the load was one that had ended
      // before the cache was told to forget), it is written now. The sound plays from the file, and its bytes
      // are let go.
      Ok(bytes) => Some(match disk.put(key, &address, &bytes) {
        Some(path) => (Opened::File(path), Some((disk, key))),
        // No cache on the disk, or one that cannot keep this: it is kept in memory.
        None => {
          sound_cache().put(key, bytes.clone());
          (Opened::Bytes(bytes, Some(address)), None)
        }
      }),
      // More than the loader reads whole, or a build that loads no address: it is read as it plays, and
      // nothing of it is kept.
      Err(LoadError::TooLarge { .. } | LoadError::Unsupported(_)) => {
        Some((stream(shared, &loader, &source, address)?, None))
      }
      Err(error) => {
        shared.fail(load_failed(error));
        None
      }
    },
    Asked::Stream {
      loader,
      source,
      address,
    } => Some((stream(shared, &loader, &source, address)?, None)),
  }
}

/// The life of a media on its thread: `asked` is waited for, and `run` plays it until nothing holds the media.
#[cfg_attr(not(any(windows, target_os = "macos", target_os = "ios")), allow(dead_code))]
fn live(shared: &Arc<Shared>, asked: Asked, heard: bool, run: impl FnOnce(&Arc<Shared>, Opened, bool)) {
  let Some((opened, kept)) = settle(shared, asked) else {
    return;
  };
  shared.whole.store(
    matches!(opened, Opened::File(_) | Opened::Bytes(..)),
    std::sync::atomic::Ordering::Release,
  );
  run(shared, opened, heard);
  // A file of the cache on the disk that could not be played is let go: it has the length it was written
  // with, and is not what was written. The next try loads it anew.
  if let Some((disk, key)) = kept
    && shared.inner().error.is_some()
  {
    disk.forget_key(key);
  }
}

impl Media {
  /// Opens `source` with the app's `loader` and shows its first frame, paused at its start: what is seen, with
  /// its sound where it has some. It never fails here: what cannot be opened says why in its
  /// [`state`](Self::state). An address is read as it plays, with the source's headers on its requests.
  pub fn open(loader: &Loader, source: &Source) -> Self {
    Self::start_from(loader, source, false)
  }

  /// Opens `source` as what is only heard: a song, a recording, a message. Where it has a picture too, the
  /// picture is not decoded.
  ///
  /// At an address it is loaded whole through the app's `loader` (with its headers, its timeout and what the
  /// loader allows) and kept: played again, it starts at once and asks the network for nothing. It is kept in
  /// memory ([`sound_cache`]) for the run of the app; where the loader's cache on the disk is on
  /// ([`Loader::enable_media_disk_cache`]), it is kept there as a file, for the next run too, and plays from the
  /// file. What is larger than the loader reads whole is read as it plays, with the source's headers, and not
  /// kept. While it loads, its state says it waits.
  pub fn sound(loader: &Loader, source: &Source) -> Self {
    Self::start_from(loader, source, true)
  }

  fn of(shared: Arc<Shared>) -> Self {
    Self(Arc::new(Handle { shared }))
  }

  fn start_from(loader: &Loader, source: &Source, heard: bool) -> Self {
    let shared = Shared::new();
    // A test drives it: nothing is opened.
    if driven::take(&shared) {
      return Self::of(shared);
    }
    let asked = match &source.kind {
      SourceKind::File(path) => Ok(Asked::Opened(Opened::File(path.clone()))),
      SourceKind::Resource(resource) => Ok(Asked::Opened(Opened::File(loader.resource_path(resource)))),
      SourceKind::Bytes(bytes) => Ok(Asked::Opened(Opened::Bytes(bytes.clone(), None))),
      SourceKind::Url(address) if !loader.allows(address) => Err(MediaError::Denied),
      // A sound is loaded whole and kept; kept already, it is there: as a file where the app's cache on the
      // disk is on, in memory where it is not. A browser reads it itself as it plays: nothing is loaded first,
      // nor kept.
      #[cfg(not(target_arch = "wasm32"))]
      SourceKind::Url(address) if heard => {
        let (key, disk) = (source.cache_key(), loader.media_disk_cache());
        sound_cache().follow(disk);
        Ok(if let Some(path) = disk.file(key) {
          Asked::Kept {
            path,
            key,
            disk: disk.clone(),
          }
        } else if let Some(bytes) = sound_cache().get(key) {
          Asked::Opened(Opened::Bytes(bytes, Some(address.clone())))
        } else {
          Asked::Load {
            load: loader.load(source),
            key,
            address: address.clone(),
            disk: disk.clone(),
            loader: loader.clone(),
            source: source.clone(),
          }
        })
      }
      SourceKind::Url(address) => Ok(Asked::Stream {
        loader: loader.clone(),
        source: source.clone(),
        address: address.clone(),
      }),
    };
    match asked {
      Ok(asked) => Self::start(&shared, asked, heard),
      Err(why) => shared.fail(why),
    }
    Self::of(shared)
  }

  #[cfg(windows)]
  fn start(shared: &Arc<Shared>, asked: Asked, heard: bool) {
    let thread = shared.clone();
    let spawned = std::thread::Builder::new()
      .name("lurq-media".to_owned())
      .spawn(move || live(&thread, asked, heard, mf::run));
    if spawned.is_err() {
      shared.fail(MediaError::Other(Arc::from(
        "a thread to decode on could not be started",
      )));
    }
  }

  #[cfg(any(target_os = "macos", target_os = "ios"))]
  fn start(shared: &Arc<Shared>, asked: Asked, heard: bool) {
    let thread = shared.clone();
    let spawned = std::thread::Builder::new()
      .name("lurq-media".to_owned())
      .spawn(move || live(&thread, asked, heard, apple::run));
    if spawned.is_err() {
      shared.fail(MediaError::Other(Arc::from("a thread to play on could not be started")));
    }
  }

  /// The page's `<audio>`: see [`web`].
  #[cfg(target_arch = "wasm32")]
  fn start(shared: &Arc<Shared>, asked: Asked, heard: bool) {
    let opened = match asked {
      Asked::Opened(opened) => opened,
      Asked::Kept { path, .. } => Opened::File(path),
      Asked::Load {
        loader,
        source,
        address,
        ..
      }
      | Asked::Stream {
        loader,
        source,
        address,
      } => Opened::Url(address, Arc::from(loader.headers_of(&source))),
    };
    web::start(shared, opened, heard);
  }

  #[cfg(not(any(windows, target_os = "macos", target_os = "ios", target_arch = "wasm32")))]
  fn start(shared: &Arc<Shared>, _: Asked, _: bool) {
    shared.fail(MediaError::Unavailable(Arc::from(
      "this build has no decoder for this system yet",
    )));
  }

  fn shared(&self) -> &Shared {
    &self.0.shared
  }

  /// How far a media that opened, `duration` long, is fetched from `position`: see [`MediaState::buffered`].
  fn buffered(&self, duration: f64, position: f64) -> Option<f64> {
    let shared = self.shared();
    if shared.whole.load(std::sync::atomic::Ordering::Acquire) {
      return Some(duration);
    }
    let stream = shared
      .stream
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner())
      .clone()?;
    Some(fetched_to(position, duration, stream.len()?, &stream.buffered()))
  }

  /// Where it is and what it is, now.
  pub fn state(&self) -> MediaState {
    let (mut state, opened, fetched) = {
      let inner = self.shared().inner();
      let state = MediaState {
        duration: inner.duration,
        position: inner.position(),
        buffered: None,
        size: inner.size,
        sound: inner.sound,
        volume: inner.volume,
        muted: inner.muted,
        playing: inner.playing,
        waiting: inner.waiting,
        ended: inner.ended,
        error: inner.error.clone(),
        rate: inner.rate,
      };
      (state, inner.opened, inner.fetched)
    };
    // Of the same moment as the rest, and read with the media's state let go: a reader has locks of its own.
    state.buffered = fetched.or_else(|| {
      let duration = state.duration.filter(|_| opened)?;
      self.buffered(duration, state.position)
    });
    state
  }

  /// Its picture, once it opened: an image that holds the frame of the moment, which a
  /// [`Video`](crate::components::Video) draws. The same image for as long as the media lives.
  pub fn frame(&self) -> Option<ImageData> {
    self.shared().inner().frame.clone()
  }

  /// Plays from where it is; from its start where it had ended. What it reads of an address is fetched ahead
  /// of what plays from now on: until it is first told to play it reads only what opening it asks for, so one
  /// opened ahead and left paused takes its start from a slow host and no more.
  pub fn play(&self) {
    let shared = self.shared();
    shared.eager.store(true, std::sync::atomic::Ordering::Release);
    let stream = shared
      .stream
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner())
      .clone();
    if let Some(stream) = stream {
      stream.read_ahead(true);
    }
    shared.tell(Command::Play);
  }

  /// Stops where it is. Its state says so at once: the thread stops decoding when it hears.
  pub fn pause(&self) {
    {
      let mut inner = self.shared().inner();
      inner.hold();
      inner.playing = false;
    }
    self.shared().noted();
    self.shared().tell(Command::Pause);
  }

  /// Goes to `seconds` and shows the frame there, playing or not. Its state is at that time at once, so what
  /// draws where it is (a seek bar being dragged) does not see it go back: the frame comes when it is decoded.
  pub fn seek(&self, seconds: f64) {
    self.seek_to(seconds, false);
  }

  /// Goes near `seconds`, to the frame there that shows quickest, and plays on from there: on macOS and iOS the
  /// key frame nearest it, a few seconds either side, which needs no frame before it. Its state is at `seconds`
  /// until that frame shows, and at the frame's time from then on.
  ///
  /// What a finger lets go of on a video's line of time is where it is to be about, not to the frame, and an exact
  /// seek is slow where the video is read from an address: the system's player reads and decodes from the key frame
  /// before the time to the time itself. Measured on a Mac over an hour-long video of a slow host: an exact seek
  /// showed its frame 1 to 3.9 s after it was asked for, this one 0.77 s, which is the host's own answer to a request
  /// for a new place. Elsewhere it is [`Media::seek`].
  pub fn seek_near(&self, seconds: f64) {
    self.seek_to(seconds, true);
  }

  fn seek_to(&self, seconds: f64, near: bool) {
    if !seconds.is_finite() {
      return;
    }
    let to = {
      let mut inner = self.shared().inner();
      let to = match inner.duration {
        Some(duration) => seconds.clamp(0.0, duration),
        None => seconds.max(0.0),
      };
      inner.at = to;
      inner.since = Instant::now();
      inner.ended = false;
      inner.sent += 1;
      to
    };
    self.shared().noted();
    self
      .shared()
      .tell(if near { Command::SeekNear(to) } else { Command::Seek(to) });
  }

  /// How fast it goes: 1 is as it was made. Nothing that is not a speed is taken.
  pub fn set_rate(&self, rate: f64) {
    if rate.is_finite() && rate > 0.0 {
      self.shared().tell(Command::Rate(rate));
    }
  }

  /// How loud its sound is, from 0 (nothing) to 1 (as it was made): of this media alone, and at once. Nothing
  /// that is not a number is taken.
  pub fn set_volume(&self, volume: f32) {
    if volume.is_finite() {
      let volume = volume.clamp(0.0, 1.0);
      self.shared().inner().volume = volume;
      self.shared().tell(Command::Volume(volume));
    }
  }

  /// Silences its sound, or lets it be heard again as loud as it was.
  pub fn set_muted(&self, muted: bool) {
    self.shared().inner().muted = muted;
    self.shared().tell(Command::Muted(muted));
  }

  /// Whether it opened: what it is (its length, its picture's size, whether it has sound) is known, and it does
  /// what it is told. A media that failed never did.
  pub fn is_open(&self) -> bool {
    self.shared().inner().opened
  }

  /// Whether its picture shows a frame of it. A media has its image as soon as it opened
  /// ([`frame`](Self::frame)), and its first frame is decoded a moment later: until then the image is black. A
  /// player keeps its poster, or shows nothing, until this is true.
  pub fn has_frame(&self) -> bool {
    let inner = self.shared().inner();
    inner.size.is_some() && inner.presented > 0
  }

  /// Whether it goes on from its start at its end, as a clip in a feed does: it is sent to its start in what it
  /// has open, and nothing is opened or read again that it holds. Each time round `on_ended` is told, as at any
  /// end, and it plays on: its state does not say it ended, and it is not told it stopped playing. Off unless
  /// told.
  pub fn set_looped(&self, looped: bool) {
    self.shared().inner().looped = looped;
    self.shared().told.notify_all();
  }

  pub fn looped(&self) -> bool {
    self.shared().inner().looped
  }

  /// Calls `wake` when the media changes in a way its clock does not tell: it opened, it ended, it failed, it
  /// went to another time. From the media's own thread.
  pub fn on_change(&self, wake: impl Fn() + Send + Sync + 'static) {
    self
      .shared()
      .wakers
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner())
      .push(Arc::new(wake));
  }

  /// Calls `handler` once it opened and is ready to play, with its length, its picture's size and whether it has
  /// sound. On the UI thread, as the events below: see [`on_playing_change`](Self::on_playing_change).
  pub fn on_load(&self, handler: impl Fn(&MediaInfo) + Send + Sync + 'static) {
    self.listening().handlers().load.push(Arc::new(handler));
  }

  /// Calls `handler` once it cannot play, with why: a kind, which the app's
  /// [`Locale`](crate::core::Locale) puts into words ([`MediaWords::error`](crate::core::MediaWords::error)). A
  /// media that failed stays failed: one that tries again is opened anew.
  pub fn on_error(&self, handler: impl Fn(&MediaError) + Send + Sync + 'static) {
    self.listening().handlers().error.push(Arc::new(handler));
  }

  /// Calls `handler` each time it played to its end: again only after it played again.
  pub fn on_ended(&self, handler: impl Fn() + Send + Sync + 'static) {
    self.listening().handlers().ended.push(Arc::new(handler));
  }

  /// Calls `handler` each time it really starts to play: it was told to play, it does not wait for what it plays,
  /// and the first frame since was shown (where it has no picture, its sound was given to the output). That is
  /// after [`on_load`](Self::on_load), which says it opened with nothing moving yet, and after
  /// [`on_playing_change`](Self::on_playing_change) said `true`, which is that it was told to play. It is told
  /// again when it moves again after it stopped: after a pause, after it waited, after its end. A looped media
  /// that goes on from its start has not stopped, and is not told; nor is one sent to another time while it
  /// plays.
  pub fn on_started(&self, handler: impl Fn() + Send + Sync + 'static) {
    self.listening().handlers().started.push(Arc::new(handler));
  }

  /// Calls `handler` with whether it plays, each time that changes once it opened: it was told to play or to
  /// pause, it ended, it failed.
  ///
  /// The events of a media are told on the thread that first listened to it, which is the UI thread: the media's
  /// own thread wakes the loops of that thread's windows, and the next turn of each tells what changed since it last
  /// told, in the order a media lives it (it opened, it waits or not, it plays or not, it ended, it failed), once
  /// each. A handler may play, pause and seek. What changed before it was listened to is told in the next turn. A
  /// second handler is called after the first.
  pub fn on_playing_change(&self, handler: impl Fn(bool) + Send + Sync + 'static) {
    self.listening().handlers().playing.push(Arc::new(handler));
  }

  /// Calls `handler` with whether it waits for what it plays (an address that is read slower than it plays), each
  /// time that changes once it opened: `true` when it began waiting, `false` when it goes on. Opening is not
  /// waiting: what opens is told by [`on_load`](Self::on_load).
  pub fn on_waiting_change(&self, handler: impl Fn(bool) + Send + Sync + 'static) {
    self.listening().handlers().waiting.push(Arc::new(handler));
  }

  fn listening(&self) -> &Arc<events::Listening> {
    events::Listening::of(&self.0.shared)
  }
}

#[cfg(test)]
mod tests {
  use std::sync::atomic::Ordering;

  use super::*;

  const MIB: u64 = 1 << 20;

  #[test]
  fn how_far_it_is_fetched_is_the_end_of_the_ranges_held_in_a_row_from_where_it_is() {
    // 100 s in 100 MiB: a second to a mebibyte.
    let (long, len) = (100.0, 100 * MIB);
    let to = |position: f64, held: &[std::ops::Range<u64>]| fetched_to(position, long, len, held);
    assert_eq!(to(0.0, &[0..10 * MIB]), 10.0);
    assert_eq!(to(5.0, &[0..10 * MIB]), 10.0, "from anywhere in the range");
    assert_eq!(
      to(0.0, &[0..10 * MIB, 10 * MIB..20 * MIB]),
      20.0,
      "ranges that touch are one"
    );
    // The start and a part further on: each is fetched to its own end, and what is between to nowhere.
    let apart = [0..MIB, 50 * MIB..60 * MIB];
    assert_eq!(to(0.0, &apart), 1.0);
    assert_eq!(to(50.0, &apart), 60.0);
    assert_eq!(to(30.0, &apart), 30.0, "sent to a part that is not fetched");
    assert_eq!(
      to(49.0, &apart),
      60.0,
      "a range that begins a little after its share holds it"
    );
    assert_eq!(
      to(90.0, &[80 * MIB..len]),
      long,
      "to the end of its bytes is its whole length"
    );
    assert_eq!(to(7.0, &[]), 7.0);
    assert_eq!(fetched_to(3.0, 0.0, len, &[0..len]), 3.0, "of no length");
  }

  fn opened() -> Arc<Shared> {
    let shared = Shared::new();
    shared.inner().opened = true;
    shared.inner().waiting = false;
    shared
  }

  fn after(millis: u64) {
    std::thread::sleep(std::time::Duration::from_millis(millis));
  }

  #[test]
  fn a_read_ahead_on_the_network_while_there_is_something_to_show_is_no_waiting() {
    let shared = opened();
    watch_for_starving(&shared);
    // The decoder reads ahead, and its thread asks for nothing.
    shared.reads_waiting.fetch_add(1, Ordering::AcqRel);
    after(400);
    assert!(!shared.inner().waiting);
    // Its thread waits for a sample that takes long to decode, with nothing on the network.
    shared.reads_waiting.fetch_sub(1, Ordering::AcqRel);
    let decoding = shared.decoding();
    after(400);
    assert!(!shared.inner().waiting);
    drop(decoding);
  }

  #[test]
  fn waiting_for_the_next_sample_while_its_read_is_on_the_network_is_waiting_after_a_moment() {
    let shared = opened();
    watch_for_starving(&shared);
    shared.reads_waiting.fetch_add(1, Ordering::AcqRel);
    let decoding = shared.decoding();
    after(100);
    assert!(!shared.inner().waiting, "a block that comes at once shows nothing");
    after(400);
    assert!(shared.inner().waiting, "starved, it waits");
    shared.reads_waiting.fetch_sub(1, Ordering::AcqRel);
    drop(decoding);
    assert!(!shared.inner().waiting, "and goes on as the sample comes");
  }

  #[test]
  fn a_playlist_of_http_live_streaming_is_told_by_its_type_or_its_ending() {
    assert!(is_playlist(Some("application/vnd.apple.mpegurl"), "https://h/live"));
    assert!(is_playlist(Some("audio/x-mpegURL"), "https://h/live"));
    assert!(is_playlist(None, "https://h/live/index.M3U8?token=1"));
    assert!(!is_playlist(Some("video/mp4"), "https://h/clip.mp4"));
    assert!(!is_playlist(None, "https://h/clip.mp4#m3u8"));
  }
}
