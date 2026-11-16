//! Playing a video or a sound: a [`Media`] opened from a [`Source`], its picture, its sound, and where it is.
//!
//! A media decodes on a thread of its own with the system's decoder (Media Foundation on Windows), keeps its own
//! clock, and puts each frame into an image a [`Video`](crate::components::Video) draws: the window is drawn
//! again for a new frame, and no component is rendered for it. Its sound goes to the system's default output,
//! and the media's clock then follows what is heard. What holds a media tells it to play, to pause, to go to a
//! time, how fast to go and how loud to be, asks it where it is ([`Media::state`]), and is woken when that
//! changes in a way a clock does not tell: it opened, it ended, it failed.
//!
//! [`Media::open`] is what is seen, with its sound; [`Media::sound`] what is only heard: a song, a recording, a
//! message. A sound at an address is loaded whole through the app's loader and kept ([`sound_cache`]), so that
//! it plays again at once.
//!
//! `ui::VideoPlayer::source` and `ui::AudioPlayer::source` do all of this for a player. An app that draws its
//! own asks for a media itself.
//!
//! What is not here yet: a speed that keeps the pitch of the sound (faster is higher); a video at an address
//! read through the app's loader (the system reads it itself where it can); a frame that stays on the graphics
//! card from the decoder to the screen (a frame is copied through memory once).

use std::{
  collections::VecDeque,
  sync::{Arc, Condvar, Mutex, MutexGuard},
  time::{Duration, Instant},
};

use crate::{
  images::ImageData,
  resources::{Load, LoadError, LoadState, Loader, Source, SourceKind},
};
pub use crate::core::MediaError;

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod apple;
mod cache;
#[cfg(windows)]
mod mf;
#[cfg(windows)]
mod output;

pub use cache::{DEFAULT_SOUND_CACHE_BYTES, SoundCache, sound_cache};

/// What changed in a media is told to these, from its own thread.
type Waker = Arc<dyn Fn() + Send + Sync>;

/// Where a media is, and what it is: read with [`Media::state`].
#[derive(Clone, Debug, PartialEq)]
pub struct MediaState {
  /// How long it is, in seconds, once that is known. `None` for what has no end, and before it opened.
  pub duration: Option<f64>,
  /// Where it is, in seconds.
  pub position: f64,
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
}

/// The state of a media, and the clock it is read from: where it was at `since`, going at `rate` while it plays.
pub(crate) struct Inner {
  pub duration: Option<f64>,
  pub size: Option<(u32, u32)>,
  pub sound: bool,
  /// How loud its sound is, from 0 to 1, and whether it is silenced: what its output starts with.
  pub volume: f32,
  pub muted: bool,
  pub playing: bool,
  pub waiting: bool,
  pub ended: bool,
  pub error: Option<MediaError>,
  pub rate: f64,
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
  }

  /// It cannot play, and why.
  pub fn fail(&self, why: MediaError) {
    {
      let mut inner = self.inner();
      inner.hold();
      inner.error = Some(why);
      inner.playing = false;
      inner.waiting = false;
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

/// What a source is, for what opens it: the file it is read from, the bytes it is, or its address.
pub(crate) enum Opened {
  File(std::path::PathBuf),
  /// The bytes, and the name they came by where they have one: its ending tells the decoder what they are.
  Bytes(Arc<[u8]>, Option<Arc<str>>),
  Url(Arc<str>),
}

/// What a media starts from: what is there to open, or a load of it that is on its way.
enum Asked {
  Opened(Opened),
  /// The bytes at `address`, loaded whole by the app's loader and kept under `key`.
  Load {
    load: Load,
    key: u64,
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

/// What `asked` is once it is there to open: a load is waited for, and what it loaded is kept. `None` where
/// there is nothing to open: the media was let go, or the load failed, which the media's state then says.
fn settle(shared: &Arc<Shared>, asked: Asked) -> Option<Opened> {
  match asked {
    Asked::Opened(opened) => Some(opened),
    Asked::Load { load, key, address } => match loaded(shared, &load)? {
      Ok(bytes) => {
        sound_cache().put(key, bytes.clone());
        Some(Opened::Bytes(bytes, Some(address)))
      }
      // More than the loader reads whole, or a build that loads no address: the system reads it itself, and
      // nothing of it is kept.
      Err(LoadError::TooLarge { .. } | LoadError::Unsupported(_)) => Some(Opened::Url(address)),
      Err(error) => {
        shared.fail(load_failed(error));
        None
      }
    },
  }
}

impl Media {
  /// Opens `source` with the app's `loader` and shows its first frame, paused at its start: what is seen, with
  /// its sound where it has some. It never fails here: what cannot be opened says why in its
  /// [`state`](Self::state).
  pub fn open(loader: &Loader, source: &Source) -> Self {
    Self::start_from(loader, source, false)
  }

  /// Opens `source` as what is only heard: a song, a recording, a message. Where it has a picture too, the
  /// picture is not decoded.
  ///
  /// At an address it is loaded whole through the app's `loader` (with its headers, its timeout and what the
  /// loader allows) and kept in [`sound_cache`]: played again, it starts at once and asks the network for
  /// nothing. What is larger than the loader reads whole is read by the system itself as it plays, and not kept.
  /// While it loads, its state says it waits.
  pub fn sound(loader: &Loader, source: &Source) -> Self {
    Self::start_from(loader, source, true)
  }

  fn start_from(loader: &Loader, source: &Source, heard: bool) -> Self {
    let shared = Arc::new(Shared {
      inner: Mutex::new(Inner {
        duration: None,
        size: None,
        sound: false,
        volume: 1.0,
        muted: false,
        playing: false,
        waiting: true,
        ended: false,
        error: None,
        rate: 1.0,
        at: 0.0,
        since: Instant::now(),
        sent: 0,
        frame: None,
      }),
      commands: Mutex::new(VecDeque::new()),
      told: Condvar::new(),
      wakers: Mutex::new(Vec::new()),
    });
    let asked = match &source.kind {
      SourceKind::File(path) => Ok(Asked::Opened(Opened::File(path.clone()))),
      SourceKind::Resource(resource) => Ok(Asked::Opened(Opened::File(loader.resource_path(resource)))),
      SourceKind::Bytes(bytes) => Ok(Asked::Opened(Opened::Bytes(bytes.clone(), None))),
      SourceKind::Url(address) if !loader.allows(address) => Err(MediaError::Denied),
      // A sound is loaded whole and kept; kept already, it is there.
      SourceKind::Url(address) if heard => Ok(match sound_cache().get(source.cache_key()) {
        Some(bytes) => Asked::Opened(Opened::Bytes(bytes, Some(address.clone()))),
        None => Asked::Load {
          load: loader.load(source),
          key: source.cache_key(),
          address: address.clone(),
        },
      }),
      SourceKind::Url(address) => Ok(Asked::Opened(Opened::Url(address.clone()))),
    };
    match asked {
      Ok(asked) => Self::start(&shared, asked, heard),
      Err(why) => shared.fail(why),
    }
    Self(Arc::new(Handle { shared }))
  }

  #[cfg(windows)]
  fn start(shared: &Arc<Shared>, asked: Asked, heard: bool) {
    let thread = shared.clone();
    let spawned = std::thread::Builder::new()
      .name("lurq-media".to_owned())
      .spawn(move || {
        if let Some(opened) = settle(&thread, asked) {
          mf::run(&thread, opened, heard);
        }
      });
    if spawned.is_err() {
      shared.fail(MediaError::Other(Arc::from("a thread to decode on could not be started")));
    }
  }

  #[cfg(any(target_os = "macos", target_os = "ios"))]
  fn start(shared: &Arc<Shared>, asked: Asked, heard: bool) {
    let thread = shared.clone();
    let spawned = std::thread::Builder::new()
      .name("lurq-media".to_owned())
      .spawn(move || {
        if let Some(opened) = settle(&thread, asked) {
          apple::run(&thread, opened, heard);
        }
      });
    if spawned.is_err() {
      shared.fail(MediaError::Other(Arc::from("a thread to play on could not be started")));
    }
  }

  #[cfg(not(any(windows, target_os = "macos", target_os = "ios")))]
  fn start(shared: &Arc<Shared>, _: Asked, _: bool) {
    let _ = settle;
    shared.fail(MediaError::Unavailable(Arc::from(
      "this build has no decoder for this system yet",
    )));
  }

  fn shared(&self) -> &Shared {
    &self.0.shared
  }

  /// Where it is and what it is, now.
  pub fn state(&self) -> MediaState {
    let inner = self.shared().inner();
    MediaState {
      duration: inner.duration,
      position: inner.position(),
      size: inner.size,
      sound: inner.sound,
      volume: inner.volume,
      muted: inner.muted,
      playing: inner.playing,
      waiting: inner.waiting,
      ended: inner.ended,
      error: inner.error.clone(),
      rate: inner.rate,
    }
  }

  /// Its picture, once it opened: an image that holds the frame of the moment, which a
  /// [`Video`](crate::components::Video) draws. The same image for as long as the media lives.
  pub fn frame(&self) -> Option<ImageData> {
    self.shared().inner().frame.clone()
  }

  /// Plays from where it is; from its start where it had ended.
  pub fn play(&self) {
    self.shared().tell(Command::Play);
  }

  /// Stops where it is. Its state says so at once: the thread stops decoding when it hears.
  pub fn pause(&self) {
    {
      let mut inner = self.shared().inner();
      inner.hold();
      inner.playing = false;
    }
    self.shared().tell(Command::Pause);
  }

  /// Goes to `seconds` and shows the frame there, playing or not. Its state is at that time at once, so what
  /// draws where it is (a seek bar being dragged) does not see it go back: the frame comes when it is decoded.
  pub fn seek(&self, seconds: f64) {
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
    self.shared().tell(Command::Seek(to));
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
}
