//! What a media tells who listens: it opened, it failed, it ended, it began or stopped playing, it began or stopped
//! waiting.
//!
//! The media's own thread changes its state and wakes the loops of the thread that listens: each tree of that
//! thread registered its loop ([`note_loop`]). The tree's next turn ([`tell_news`], from its pass and from the tick
//! of its timers) looks at each media of its thread that has news, compares its state with what it last told, and
//! calls the handlers: on the UI thread, once per change, in the order a media lives them.

use std::{
  cell::RefCell,
  sync::{
    Arc, Mutex, MutexGuard, OnceLock, Weak,
    atomic::{AtomicBool, AtomicU64, Ordering},
  },
  thread::ThreadId,
};

use super::{Inner, MediaError, MediaInfo, Shared};

type Waker = Arc<dyn Fn() + Send + Sync>;

/// The handlers of a media's events, in the order they were added.
#[derive(Clone, Default)]
pub(crate) struct Handlers {
  pub load: Vec<Arc<dyn Fn(&MediaInfo) + Send + Sync>>,
  pub error: Vec<Arc<dyn Fn(&MediaError) + Send + Sync>>,
  pub ended: Vec<Arc<dyn Fn() + Send + Sync>>,
  pub playing: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
  pub waiting: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
}

/// What happened to a media, as it is told.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum MediaEvent {
  Load(MediaInfo),
  Waiting(bool),
  Playing(bool),
  Ended,
  Error(MediaError),
}

/// What a media last told of itself.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Told {
  loaded: bool,
  failed: bool,
  playing: bool,
  waiting: bool,
  ended: bool,
  /// How many of the times a looped media went on from its start were told as ends.
  rounds: u64,
}

impl Told {
  /// What changed in `inner` since it was last told, in order: it opened, it began or stopped waiting, it began or
  /// stopped playing, it ended, it failed. Nothing is told before it opened but that it failed; waiting and
  /// playing are told while it is open and has not failed.
  pub(crate) fn news(&mut self, inner: &Inner) -> Vec<MediaEvent> {
    let mut news = Vec::new();
    let failed = inner.error.is_some();
    if !self.loaded && inner.opened && !failed {
      self.loaded = true;
      news.push(MediaEvent::Load(MediaInfo {
        duration: inner.duration,
        size: inner.size,
        sound: inner.sound,
      }));
    }
    let waiting = self.loaded && inner.waiting && !failed;
    if waiting != self.waiting {
      self.waiting = waiting;
      news.push(MediaEvent::Waiting(waiting));
    }
    let playing = self.loaded && inner.playing && !failed;
    if playing != self.playing {
      self.playing = playing;
      news.push(MediaEvent::Playing(playing));
    }
    // A looped media came to its end and went on from its start: an end each time, with no stop of its playing.
    if self.loaded {
      for _ in self.rounds..inner.rounds {
        news.push(MediaEvent::Ended);
      }
    }
    self.rounds = inner.rounds;
    let ended = self.loaded && inner.ended;
    if ended != self.ended {
      self.ended = ended;
      if ended {
        news.push(MediaEvent::Ended);
      }
    }
    if let (false, Some(error)) = (self.failed, &inner.error) {
      self.failed = true;
      news.push(MediaEvent::Error(error.clone()));
    }
    news
  }
}

/// A media that is listened to: its handlers, what it last told, and whether it has news.
pub(crate) struct Listening {
  /// The thread that listens, whose trees tell the news.
  owner: ThreadId,
  handlers: Mutex<Handlers>,
  told: Mutex<Told>,
  news: AtomicBool,
  shared: Weak<Shared>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
  mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Listening {
  /// The listening of `shared`, made on the first handler added: on this thread.
  pub(crate) fn of(shared: &Arc<Shared>) -> &Arc<Self> {
    let mut made = false;
    let listening = shared.listening.get_or_init(|| {
      made = true;
      Arc::new(Self {
        owner: std::thread::current().id(),
        handlers: Mutex::default(),
        told: Mutex::default(),
        // What it did before anyone listened is told in the next turn.
        news: AtomicBool::new(true),
        shared: Arc::downgrade(shared),
      })
    });
    if made {
      LISTENED.with(|listened| listened.borrow_mut().push(Arc::downgrade(listening)));
      wake_loops(listening.owner);
    }
    listening
  }

  pub(crate) fn handlers(&self) -> MutexGuard<'_, Handlers> {
    lock(&self.handlers)
  }

  /// The media changed: the next turn of the listening thread looks at it.
  pub(crate) fn note(&self) {
    self.news.store(true, Ordering::Release);
    wake_loops(self.owner);
  }
}

thread_local! {
  /// The medias this thread listens to.
  static LISTENED: RefCell<Vec<Weak<Listening>>> = const { RefCell::new(Vec::new()) };
}

/// The loops of every thread's trees, each with the key it was noted under.
fn loops() -> MutexGuard<'static, Vec<(ThreadId, u64, Waker)>> {
  static LOOPS: OnceLock<Mutex<Vec<(ThreadId, u64, Waker)>>> = OnceLock::new();
  lock(LOOPS.get_or_init(Mutex::default))
}

/// A tree's loop, noted for the medias of its thread: what wakes it while it lives.
pub(crate) struct LoopNoted(u64);

impl Drop for LoopNoted {
  fn drop(&mut self) {
    loops().retain(|(_, key, _)| *key != self.0);
  }
}

/// Notes `wake` as a loop of this thread: a media this thread listens to wakes it when it has news.
pub(crate) fn note_loop(wake: Waker) -> LoopNoted {
  static KEYS: AtomicU64 = AtomicU64::new(0);
  let key = KEYS.fetch_add(1, Ordering::Relaxed);
  loops().push((std::thread::current().id(), key, wake));
  LoopNoted(key)
}

/// Wakes the loops of `owner`, outside the lock: a waker may take other locks.
fn wake_loops(owner: ThreadId) {
  let wakers: Vec<Waker> = loops()
    .iter()
    .filter(|(thread, ..)| *thread == owner)
    .map(|(_, _, wake)| wake.clone())
    .collect();
  for wake in wakers {
    wake();
  }
}

/// Tells the handlers of each media this thread listens to what changed since it last told them. Called by a
/// tree in its turn; whether anything was told.
pub(crate) fn tell_news() -> bool {
  super::driven::answer();
  let listened: Vec<Arc<Listening>> = LISTENED.with(|listened| {
    let mut listened = listened.borrow_mut();
    listened.retain(|listening| listening.strong_count() > 0);
    listened
      .iter()
      .filter_map(Weak::upgrade)
      .filter(|listening| listening.news.swap(false, Ordering::AcqRel))
      .collect()
  });
  let mut told = false;
  for listening in listened {
    let Some(shared) = listening.shared.upgrade() else {
      continue;
    };
    let news = {
      let inner = shared.inner();
      lock(&listening.told).news(&inner)
    };
    if news.is_empty() {
      continue;
    }
    told = true;
    // Called with no lock held: a handler may play, pause or listen to the media.
    let handlers = listening.handlers().clone();
    for event in news {
      match event {
        MediaEvent::Load(info) => handlers.load.iter().for_each(|handler| handler(&info)),
        MediaEvent::Waiting(waiting) => handlers.waiting.iter().for_each(|handler| handler(waiting)),
        MediaEvent::Playing(playing) => handlers.playing.iter().for_each(|handler| handler(playing)),
        MediaEvent::Ended => handlers.ended.iter().for_each(|handler| handler()),
        MediaEvent::Error(error) => handlers.error.iter().for_each(|handler| handler(&error)),
      }
    }
  }
  told
}

#[cfg(test)]
mod tests {
  use super::*;

  fn inner() -> Inner {
    Inner {
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
      at: 0.0,
      since: crate::time::Instant::now(),
      sent: 0,
      frame: None,
    }
  }

  #[test]
  fn a_media_tells_it_opened_before_it_plays_and_that_it_ended_after_it_stopped() {
    let (mut told, mut state) = (Told::default(), inner());
    // Opening is no waiting to tell, and a play asked for before it opened waits for it.
    state.playing = true;
    assert_eq!(told.news(&state), []);
    state.opened = true;
    state.waiting = false;
    state.duration = Some(3.0);
    state.size = Some((640, 360));
    state.sound = true;
    assert_eq!(
      told.news(&state),
      [
        MediaEvent::Load(MediaInfo {
          duration: Some(3.0),
          size: Some((640, 360)),
          sound: true
        }),
        MediaEvent::Playing(true)
      ]
    );
    assert_eq!(told.news(&state), [], "each once");
    state.waiting = true;
    assert_eq!(told.news(&state), [MediaEvent::Waiting(true)]);
    state.waiting = false;
    assert_eq!(told.news(&state), [MediaEvent::Waiting(false)]);
    state.playing = false;
    state.ended = true;
    assert_eq!(told.news(&state), [MediaEvent::Playing(false), MediaEvent::Ended]);
    assert_eq!(told.news(&state), []);
    // Played again, it ends again.
    state.ended = false;
    state.playing = true;
    assert_eq!(told.news(&state), [MediaEvent::Playing(true)]);
    state.ended = true;
    state.playing = false;
    assert_eq!(told.news(&state), [MediaEvent::Playing(false), MediaEvent::Ended]);
  }

  #[test]
  fn a_change_on_the_medias_thread_wakes_the_loops_of_the_listening_thread_and_is_told_there() {
    use std::sync::atomic::AtomicUsize;
    let woken = Arc::new(AtomicUsize::new(0));
    let _noted = note_loop({
      let woken = woken.clone();
      Arc::new(move || {
        woken.fetch_add(1, Ordering::SeqCst);
      })
    });
    let (media, driver) = super::super::Media::driven();
    let told = Arc::new(Mutex::new(Vec::new()));
    media.on_load({
      let told = told.clone();
      move |info| lock(&told).push((std::thread::current().id(), *info))
    });
    let before = woken.load(Ordering::SeqCst);
    std::thread::spawn(move || driver.open(MediaInfo::default()))
      .join()
      .unwrap();
    assert!(
      woken.load(Ordering::SeqCst) > before,
      "the listening thread's loop is woken"
    );
    assert!(lock(&told).is_empty(), "and told nothing on the media's thread");
    assert!(tell_news());
    assert_eq!(*lock(&told), [(std::thread::current().id(), MediaInfo::default())]);
    assert!(!tell_news(), "once");
  }

  #[test]
  fn a_media_that_fails_tells_it_once_and_stops_playing_and_waiting_first() {
    let (mut told, mut state) = (Told::default(), inner());
    state.opened = true;
    state.waiting = true;
    state.playing = true;
    assert!(matches!(
      &told.news(&state)[..],
      [
        MediaEvent::Load(_),
        MediaEvent::Waiting(true),
        MediaEvent::Playing(true)
      ]
    ));
    state.error = Some(MediaError::Unsupported);
    assert_eq!(
      told.news(&state),
      [
        MediaEvent::Waiting(false),
        MediaEvent::Playing(false),
        MediaEvent::Error(MediaError::Unsupported)
      ]
    );
    assert_eq!(told.news(&state), []);
    // What never opened tells that it failed, and nothing else.
    let (mut told, mut state) = (Told::default(), inner());
    state.error = Some(MediaError::NotFound);
    assert_eq!(told.news(&state), [MediaEvent::Error(MediaError::NotFound)]);
  }
}
