//! A sound in a browser: the page's own `<audio>` element plays it, and the media's state follows the element's
//! events. A page has one thread, so there is no thread of the media: what it is told is done at once
//! ([`told`]), and what the element says comes on the page's thread between turns of the loop.
//!
//! An address is given to the element as it is: the browser reads it as it plays, and plays it from another server
//! without asking that server (an `<audio>` is not bound by CORS, as a `fetch` is). A file or a resource is the
//! page's own file at that path. Bytes are given as a blob of the page. What is seen ([`Media::open`]) is not
//! played: a video in a browser waits for wgpu to take the page's frames without a copy (decided 2026-11-17).
//!
//! [`Media::open`]: super::Media::open

use std::{cell::RefCell, collections::HashMap, sync::Arc};

use wasm_bindgen::{JsCast, JsValue, closure::Closure};

use super::{Command, MediaError, Opened, Shared};
use crate::time::Instant;

/// The media's clock is set to the element's where they are this far apart, in seconds.
const APART: f64 = 0.03;

/// A sound's element, and what listens to it.
struct Player {
  shared: Arc<Shared>,
  element: web_sys::HtmlAudioElement,
  /// The listeners on the element, kept while it lives: dropped, they are gone from the page.
  listeners: Vec<(&'static str, Closure<dyn FnMut(web_sys::Event)>)>,
  /// The blob's address bytes were given as, let go with the element.
  blob: Option<String>,
  /// How many of the times it was sent somewhere ([`Command::Seek`]) the element has done.
  gone: u64,
}

thread_local! {
  /// The players of the page, by their media.
  static PLAYERS: RefCell<HashMap<usize, Player>> = RefCell::new(HashMap::new());
}

fn key(shared: &Shared) -> usize {
  std::ptr::from_ref(shared) as usize
}

/// Starts playing `opened` as a sound, or says why it cannot.
pub(super) fn start(shared: &Arc<Shared>, opened: Opened, heard: bool) {
  if !heard {
    shared.fail(MediaError::Unavailable(Arc::from(
      "a browser's build plays no video yet: it waits for wgpu to take the page's frames",
    )));
    return;
  }
  let element = match web_sys::HtmlAudioElement::new() {
    Ok(element) => element,
    Err(_) => {
      shared.fail(MediaError::Unavailable(Arc::from("the page could not make an <audio>")));
      return;
    }
  };
  element.set_preload("auto");
  let (address, blob) = match opened {
    Opened::Url(address, headers) => {
      // A page's `<audio>` sends the browser's own headers, and a page cannot give it others: some, as
      // `Referer`, no page may set at all.
      if !headers.is_empty() {
        static SAID: std::sync::Once = std::sync::Once::new();
        SAID.call_once(|| {
          tracing::warn!(target: "lurq::media", "a browser plays an address without the source's headers");
        });
      }
      (address.to_string(), None)
    }
    Opened::Reader(..) => {
      shared.fail(MediaError::Unavailable(Arc::from(
        "the page's <audio> reads an address itself",
      )));
      return;
    }
    Opened::File(path) => (path.to_string_lossy().replace('\\', "/"), None),
    Opened::Bytes(bytes, _) => match blob_address(&bytes) {
      Some(address) => (address.clone(), Some(address)),
      None => {
        shared.fail(MediaError::Other(Arc::from(
          "the page could not hold the bytes as a blob",
        )));
        return;
      }
    },
  };
  let mut player = Player {
    shared: shared.clone(),
    element: element.clone(),
    listeners: Vec::new(),
    blob,
    gone: 0,
  };
  for kind in [
    "loadedmetadata",
    "playing",
    "waiting",
    "pause",
    "ended",
    "error",
    "timeupdate",
    "seeked",
  ] {
    let shared = shared.clone();
    let listener = Closure::<dyn FnMut(web_sys::Event)>::new(move |_: web_sys::Event| heard_from(&shared, kind));
    let _ = element.add_event_listener_with_callback(kind, listener.as_ref().unchecked_ref());
    player.listeners.push((kind, listener));
  }
  {
    let inner = shared.inner();
    element.set_volume(f64::from(if inner.muted { 0.0 } else { inner.volume }));
  }
  element.set_src(&address);
  PLAYERS.with(|players| players.borrow_mut().insert(key(shared), player));
  // What was told before it was made.
  told(shared);
}

/// A blob of the page that holds `bytes`, by its address.
fn blob_address(bytes: &[u8]) -> Option<String> {
  let array = js_sys::Uint8Array::from(bytes);
  let parts = js_sys::Array::of1(&array);
  let blob = web_sys::Blob::new_with_u8_array_sequence(&parts).ok()?;
  web_sys::Url::create_object_url_with_blob(&blob).ok()
}

/// What the element said, `kind`, as the media's state.
fn heard_from(shared: &Arc<Shared>, kind: &str) {
  let Some(element) = PLAYERS.with(|players| players.borrow().get(&key(shared)).map(|player| player.element.clone()))
  else {
    return;
  };
  let at = element.current_time();
  let mut inner = shared.inner();
  match kind {
    "loadedmetadata" => {
      let duration = element.duration();
      inner.duration = (duration.is_finite() && duration > 0.0).then_some(duration);
      inner.sound = true;
      inner.opened = true;
      inner.waiting = false;
    }
    // The element's own word for it: it has what it plays, and plays.
    "playing" => {
      inner.waiting = false;
      inner.at = at;
      inner.since = Instant::now();
      shared.presented_in(&mut inner);
    }
    "waiting" => {
      inner.hold();
      inner.waiting = true;
    }
    "pause" if inner.playing && !element.ended() => {
      // Paused by the browser itself (another tab took the output, the system's media keys): it is paused there.
      inner.at = at;
      inner.since = Instant::now();
      inner.playing = false;
    }
    // Looped: the element goes on from its start, with what it has loaded.
    "ended" if inner.looped => {
      element.set_current_time(0.0);
      let _ = element.play();
      inner.at = 0.0;
      inner.rounds += 1;
      inner.since = Instant::now();
    }
    "ended" => {
      inner.at = inner.duration.unwrap_or(at);
      inner.since = Instant::now();
      inner.playing = false;
      inner.ended = true;
    }
    "error" => {
      let why = failed(&element);
      drop(inner);
      shared.fail(why);
      return;
    }
    "seeked" => {
      PLAYERS.with(|players| {
        if let Some(player) = players.borrow_mut().get_mut(&key(shared)) {
          player.gone += 1;
        }
      });
      return;
    }
    "timeupdate" => {
      let gone = PLAYERS.with(|players| players.borrow().get(&key(shared)).map_or(0, |player| player.gone));
      // The media's clock is the element's: what is heard is where it is. Not while it was sent elsewhere and has
      // not come there: that would be where it was.
      if inner.playing && !inner.waiting && inner.sent == gone && (at - inner.position()).abs() > APART {
        inner.at = at;
        inner.since = Instant::now();
      }
      // It went on: where the element said it plays before who listens had heard it was told to, this is what
      // tells them it started.
      if inner.playing && !inner.waiting {
        shared.presented_in(&mut inner);
      }
      return;
    }
    _ => return,
  }
  drop(inner);
  shared.wake();
}

/// Why the element cannot play, as a media says it.
fn failed(element: &web_sys::HtmlAudioElement) -> MediaError {
  let Some(error) = element.error() else {
    return MediaError::Other(Arc::from("the browser could not play it"));
  };
  let said: Arc<str> = Arc::from(error.message());
  match error.code() {
    web_sys::MediaError::MEDIA_ERR_NETWORK => MediaError::Network(said),
    // The browser does not say whether the address had nothing, or something it does not play.
    web_sys::MediaError::MEDIA_ERR_DECODE | web_sys::MediaError::MEDIA_ERR_SRC_NOT_SUPPORTED => MediaError::Unsupported,
    _ => MediaError::Other(said),
  }
}

/// Does what the media was told, at once: called by [`Shared::tell`].
pub(super) fn told(shared: &Shared) {
  let Some((shared, element)) = PLAYERS.with(|players| {
    players
      .borrow()
      .get(&key(shared))
      .map(|player| (player.shared.clone(), player.element.clone()))
  }) else {
    // Not made yet, or failed: what is told waits, or is nothing to anybody.
    return;
  };
  let shared = &shared;
  for command in shared.take(None) {
    match command {
      Command::Close => {
        close(shared);
        return;
      }
      Command::Play => {
        {
          let mut inner = shared.inner();
          if inner.ended {
            element.set_current_time(0.0);
            inner.ended = false;
            inner.at = 0.0;
          }
          element.set_playback_rate(inner.rate);
          inner.since = Instant::now();
          inner.playing = true;
        }
        if let Ok(started) = element.play() {
          // The browser may refuse to start a sound no person asked for (its autoplay rule): it is paused again.
          let shared = shared.clone();
          let refused = Closure::once(move |_: JsValue| {
            {
              let mut inner = shared.inner();
              inner.hold();
              inner.playing = false;
            }
            shared.wake();
          });
          let _ = started.catch(&refused);
          refused.forget();
        }
        shared.wake();
      }
      Command::Pause => {
        let _ = element.pause();
        let at = shared.inner().at;
        element.set_current_time(at);
      }
      Command::Seek(to) => {
        let to = shared.inner().duration.map_or(to, |duration| to.min(duration));
        element.set_current_time(to);
      }
      Command::Rate(rate) => {
        let mut inner = shared.inner();
        inner.hold();
        inner.rate = rate;
        element.set_playback_rate(rate);
      }
      Command::Volume(_) | Command::Muted(_) => {
        let inner = shared.inner();
        element.set_volume(f64::from(if inner.muted { 0.0 } else { inner.volume }));
      }
    }
  }
}

/// Nothing holds the media: its element is stopped and let go, with its blob.
fn close(shared: &Arc<Shared>) {
  let Some(player) = PLAYERS.with(|players| players.borrow_mut().remove(&key(shared))) else {
    return;
  };
  let _ = player.element.pause();
  for (kind, listener) in &player.listeners {
    let _ = player
      .element
      .remove_event_listener_with_callback(kind, listener.as_ref().unchecked_ref());
  }
  player.element.remove_attribute("src").ok();
  player.element.load();
  if let Some(blob) = &player.blob {
    let _ = web_sys::Url::revoke_object_url(blob);
  }
}
