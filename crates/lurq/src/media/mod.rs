//! Playing a video: a [`Media`] opened from a [`Source`], its picture, and where it is.
//!
//! A media decodes on a thread of its own with the system's decoder (Media Foundation on Windows), keeps its own
//! clock, and puts each frame into an image a [`Video`](crate::components::Video) draws: the window is drawn
//! again for a new frame, and no component is rendered for it. What holds a media tells it to play, to pause, to
//! go to a time and how fast to go, asks it where it is ([`Media::state`]), and is woken when that changes in a
//! way a clock does not tell: it opened, it ended, it failed.
//!
//! `ui::VideoPlayer::source` does all of this for a player. An app that draws its own asks for a media itself.
//!
//! What is not here yet: the sound of what plays; an address read through the app's loader (the system reads it
//! itself where it can); a frame that stays on the graphics card from the decoder to the screen (a frame is
//! copied through memory once).

use std::{
  collections::VecDeque,
  sync::{Arc, Condvar, Mutex, MutexGuard},
  time::{Duration, Instant},
};

use crate::{
  images::ImageData,
  resources::{Loader, Source, SourceKind},
};

#[cfg(windows)]
mod mf;

/// What changed in a media is told to these, from its own thread.
type Waker = Arc<dyn Fn() + Send + Sync>;

/// Where a media is, and what it is: read with [`Media::state`].
#[derive(Clone, Debug, PartialEq)]
pub struct MediaState {
  /// How long it is, in seconds, once that is known. `None` for what has no end, and before it opened.
  pub duration: Option<f64>,
  /// Where it is, in seconds.
  pub position: f64,
  /// How large its picture is, once it opened.
  pub size: Option<(u32, u32)>,
  /// Whether it plays: it was told to, and has not ended or failed.
  pub playing: bool,
  /// Whether it waits for what it plays: opening, or reading what is not there yet.
  pub waiting: bool,
  /// Whether it played to its end.
  pub ended: bool,
  /// Why it cannot play, where it cannot.
  pub error: Option<Arc<str>>,
  /// How fast it goes: 1 is as it was made.
  pub rate: f64,
}

/// What the thread of a media is told.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Command {
  Play,
  Pause,
  Seek(f64),
  Rate(f64),
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
  pub playing: bool,
  pub waiting: bool,
  pub ended: bool,
  pub error: Option<Arc<str>>,
  pub rate: f64,
  pub at: f64,
  pub since: Instant,
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
  pub fn fail(&self, why: impl Into<Arc<str>>) {
    {
      let mut inner = self.inner();
      inner.hold();
      inner.error = Some(why.into());
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

/// A video that plays. It is cheap to clone, and its thread ends when the last of it is dropped.
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
/// # }
/// ```
#[derive(Clone)]
pub struct Media(Arc<Handle>);

/// What a source is, for what opens it: the file it is read from, the bytes it is, or its address.
pub(crate) enum Opened {
  File(std::path::PathBuf),
  Bytes(Arc<[u8]>),
  Url(Arc<str>),
}

impl Media {
  /// Opens `source` with the app's `loader` and shows its first frame, paused at its start. It never fails here:
  /// what cannot be opened says why in its [`state`](Self::state).
  pub fn open(loader: &Loader, source: &Source) -> Self {
    let shared = Arc::new(Shared {
      inner: Mutex::new(Inner {
        duration: None,
        size: None,
        playing: false,
        waiting: true,
        ended: false,
        error: None,
        rate: 1.0,
        at: 0.0,
        since: Instant::now(),
        frame: None,
      }),
      commands: Mutex::new(VecDeque::new()),
      told: Condvar::new(),
      wakers: Mutex::new(Vec::new()),
    });
    let opened = match &source.kind {
      SourceKind::File(path) => Ok(Opened::File(path.clone())),
      SourceKind::Resource(resource) => Ok(Opened::File(loader.resource_path(resource))),
      SourceKind::Bytes(bytes) => Ok(Opened::Bytes(bytes.clone())),
      SourceKind::Url(address) if loader.allows(address) => Ok(Opened::Url(address.clone())),
      SourceKind::Url(_) => Err("the app does not allow this address"),
    };
    match opened {
      Ok(opened) => Self::start(&shared, opened),
      Err(why) => shared.fail(why),
    }
    Self(Arc::new(Handle { shared }))
  }

  #[cfg(windows)]
  fn start(shared: &Arc<Shared>, opened: Opened) {
    let thread = shared.clone();
    let spawned = std::thread::Builder::new()
      .name("lurq-media".to_owned())
      .spawn(move || mf::run(&thread, opened));
    if spawned.is_err() {
      shared.fail("a thread to decode on could not be started");
    }
  }

  #[cfg(not(windows))]
  fn start(shared: &Arc<Shared>, _: Opened) {
    shared.fail("this build has no decoder for this system yet");
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
