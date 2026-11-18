//! What a player tells of the media of its source: it opened, it failed, it ended, it began or stopped waiting,
//! and whether it plays. [`VideoPlayer`](super::VideoPlayer) and [`AudioPlayer`](super::AudioPlayer) share it.
//!
//! The media tells its events on the UI thread ([`Media::on_load`](crate::media::Media::on_load) and the rest); a
//! player listens to each media it opens and passes them on to the handlers of its last render. A player without a
//! source tells none of them: what it shows of loading, ending and failing is the app's, and a handler is not told
//! what the app set.

use std::sync::Arc;

use crate::core::{MediaError, MediaInfo};

/// The handlers a player was given for its media's events, in the order they were added.
#[derive(Clone, Default)]
pub(crate) struct MediaEvents {
  pub load: Vec<Arc<dyn Fn(&MediaInfo) + Send + Sync>>,
  pub error: Vec<Arc<dyn Fn(&MediaError) + Send + Sync>>,
  pub ended: Vec<Arc<dyn Fn() + Send + Sync>>,
  pub waiting: Vec<Arc<dyn Fn(bool) + Send + Sync>>,
}

#[cfg(feature = "media")]
pub(crate) use listener::Listener;

#[cfg(feature = "media")]
mod listener {
  use std::sync::{Arc, Mutex, MutexGuard};

  use super::MediaEvents;
  use crate::media::Media;

  type Playing = Arc<dyn Fn(bool) + Send + Sync>;

  fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
  }

  /// What a mounted player keeps to tell its media's events: the handlers of its last render, and what the app
  /// was last told, or said, of whether it plays.
  #[derive(Clone, Default)]
  pub(crate) struct Listener {
    events: Arc<Mutex<MediaEvents>>,
    /// The app's handlers of playing: a signal it is bound to, the one it holds, `on_playing_change`.
    playing: Arc<Mutex<Vec<Playing>>>,
    /// Whether it plays, as the app knows it: what a press told it, what it set, what the media told it.
    known: Arc<Mutex<Option<bool>>>,
    /// What the media last told the app of playing, until the app's value answers it: a signal it is bound to
    /// that follows the media is not the app telling the media to play or to pause.
    echo: Arc<Mutex<Option<bool>>>,
  }

  impl Listener {
    /// Listens to `media`, which the player just opened: each of its events goes to the handlers of the last
    /// render. Whether it plays is told only where the app does not know it already.
    pub(crate) fn listen(&self, media: &Media) {
      let events = self.events.clone();
      media.on_load(move |info| {
        let handlers = lock(&events).load.clone();
        handlers.iter().for_each(|handler| handler(info));
      });
      let events = self.events.clone();
      media.on_error(move |error| {
        let handlers = lock(&events).error.clone();
        handlers.iter().for_each(|handler| handler(error));
      });
      let events = self.events.clone();
      media.on_ended(move || {
        let handlers = lock(&events).ended.clone();
        handlers.iter().for_each(|handler| handler());
      });
      let events = self.events.clone();
      media.on_waiting_change(move |waiting| {
        let handlers = lock(&events).waiting.clone();
        handlers.iter().for_each(|handler| handler(waiting));
      });
      let (playing, known, echo) = (self.playing.clone(), self.known.clone(), self.echo.clone());
      media.on_playing_change(move |plays| {
        if lock(&known).replace(plays) == Some(plays) {
          return;
        }
        *lock(&echo) = Some(plays);
        let handlers = lock(&playing).clone();
        handlers.iter().for_each(|handler| handler(plays));
      });
    }

    /// The handlers of this render: its events', and the app's of playing.
    pub(crate) fn update(&self, events: &MediaEvents, playing: &[Playing]) {
      *lock(&self.events) = events.clone();
      *lock(&self.playing) = playing.to_vec();
    }

    /// The app knows whether it plays: a press told it, or it set it. The media's saying so is not told again.
    pub(crate) fn known(&self, plays: bool) {
      *lock(&self.known) = Some(plays);
      *lock(&self.echo) = None;
    }

    /// Whether the app's value of playing, changed to `plays`, is its answer to what the media told it.
    pub(crate) fn answers(&self, plays: bool) -> bool {
      lock(&self.echo).take() == Some(plays)
    }

    /// What the app holds is what it knows, where nothing told it otherwise before.
    pub(crate) fn first(&self, plays: bool) {
      lock(&self.known).get_or_insert(plays);
    }
  }
}
