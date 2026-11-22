//! An address the app's loader reads, fed to the system's player: its asset is given an address of a scheme of its
//! own (`lurq-https://…`), which the player cannot read, so it asks the asset's resource loader for each range it
//! wants, and the delegate here answers from the loader's reader ([`Loader::open`](crate::resources::Loader::open)).
//!
//! Read by the player itself, an address was read on as fast as its host gave it, a slow host's whole speed, and
//! a range a seek asked for came beside that, on the same connection: measured on a Mac, a video of 51 MB read on
//! at 690 KB a second while it played, and a seek in it showed its frame 1.1 to 2.1 s later. The loader reads what
//! is waited for before what is read ahead, keeps the starts of videos and a cache on the disk, and opens a
//! connection ahead, as it does on Windows; and the reading is held to what plays: a range is answered as far as
//! [`AHEAD_WHILE_PLAYING`](super::video::AHEAD_WHILE_PLAYING) seconds past where the media is (before it is first
//! told to play, [`AHEAD_UNTIL_PLAYED`](super::video::AHEAD_UNTIL_PLAYED)), and the rest of it waits until the
//! media comes nearer.
//!
//! Each range is answered on a thread of its own, which reads from the reader, gives the player what it read, and
//! looks between two reads whether the player still wants it: a seek drops what it asked for before.

use std::{
  sync::{Arc, Weak, atomic::Ordering},
  time::Duration,
};

use dispatch2::{DispatchQueue, DispatchRetained};
use objc2::{
  AnyThread, DefinedClass, Message, define_class, msg_send,
  rc::{Retained, autoreleasepool},
  runtime::ProtocolObject,
};
use objc2_av_foundation::{
  AVAssetResourceLoader, AVAssetResourceLoaderDelegate, AVAssetResourceLoadingRequest, AVURLAsset,
};
use objc2_foundation::{NSData, NSError, NSObject, NSObjectProtocol, NSString, NSURL};

use super::video::{AHEAD_UNTIL_PLAYED, AHEAD_WHILE_PLAYING};
use crate::{media::Shared, resources::Reader};

/// What a range is answered with at a time: the player has a part to decode as soon as it is read.
const PART: usize = 256 * 1024;
/// What of a range is answered at once, wherever it starts: the index a player reads at a file's end, and the start
/// of a seek, never wait for what plays to come near them.
const AT_ONCE: u64 = 1024 * 1024;
/// How often a range that waits for what plays to come near looks again.
const PACE: Duration = Duration::from_millis(100);

/// What the delegate answers from: the reader, the media it plays for, and what the reader's bytes are.
pub(super) struct Feed {
  reader: Arc<dyn Reader>,
  shared: Weak<Shared>,
  /// The system's name for what the bytes are: `public.mpeg-4`, `com.apple.quicktime-movie`.
  kind: &'static str,
}

define_class!(
  // SAFETY: `NSObject` has no requirement of its subclasses, and `Feeder` has no `Drop`.
  #[unsafe(super(NSObject))]
  #[name = "LurqMediaFeeder"]
  #[ivars = Feed]
  pub(super) struct Feeder;

  // SAFETY: `NSObjectProtocol` asks nothing.
  unsafe impl NSObjectProtocol for Feeder {}

  // SAFETY: the methods have the protocol's signatures.
  unsafe impl AVAssetResourceLoaderDelegate for Feeder {
    #[unsafe(method(resourceLoader:shouldWaitForLoadingOfRequestedResource:))]
    fn should_wait(&self, _loader: &AVAssetResourceLoader, request: &AVAssetResourceLoadingRequest) -> bool {
      let feed = self.ivars();
      let (reader, shared, kind) = (feed.reader.clone(), feed.shared.clone(), feed.kind);
      let request = Asked(request.retain());
      std::thread::Builder::new()
        .name("lurq-media-feed".to_owned())
        .spawn(move || {
          let request = request.take();
          autoreleasepool(|_| answer(&request, &*reader, &shared, kind));
        })
        .is_ok()
    }
  }
);

/// A request of the player's, answered from another thread than the one it came on, as the system allows.
struct Asked(Retained<AVAssetResourceLoadingRequest>);

// SAFETY: a loading request is answered from any thread: given data, finished or failed, and asked whether it is
// cancelled. It is used by one thread at a time.
unsafe impl Send for Asked {}

impl Asked {
  /// The request, on the thread that answers it: taken whole, so the closure that moves it moves it whole.
  fn take(self) -> Retained<AVAssetResourceLoadingRequest> {
    self.0
  }
}

impl Feeder {
  /// An asset of `address` whose bytes the delegate gives from `reader`, for `shared`, with the delegate and the
  /// queue it is asked on: the asset holds its delegate weakly, so they are kept as long as the asset is.
  pub(super) fn asset(
    address: &str,
    name: &str,
    reader: Arc<dyn Reader>,
    shared: &Arc<Shared>,
  ) -> Option<(Retained<AVURLAsset>, Retained<Self>, DispatchRetained<DispatchQueue>)> {
    let kind = kind_of(reader.content_type().as_deref(), name);
    let fed = NSURL::URLWithString(&NSString::from_str(&format!("lurq-{address}")))?;
    let this = Self::alloc().set_ivars(Feed {
      reader,
      shared: Arc::downgrade(shared),
      kind,
    });
    // SAFETY: `NSObject`'s `init`.
    let feeder: Retained<Self> = unsafe { msg_send![super(this), init] };
    let queue = DispatchQueue::new("lurq.media.feed", None);
    // SAFETY: the asset is made with an address and no option; the delegate is asked on the queue, which runs
    // `should_wait` and nothing else.
    let asset = unsafe {
      let asset = AVURLAsset::URLAssetWithURL_options(&fed, None);
      asset
        .resourceLoader()
        .setDelegate_queue(Some(ProtocolObject::from_ref(&*feeder)), Some(&queue));
      asset
    };
    Some((asset, feeder, queue))
  }
}

/// Answers `request` from `reader`: what the bytes are, where the player asks, and the range it asks for, as far
/// as what plays allows.
fn answer(request: &AVAssetResourceLoadingRequest, reader: &dyn Reader, shared: &Weak<Shared>, kind: &str) {
  let length = reader.len();
  // SAFETY: the request is the player's, answered from this thread alone.
  unsafe {
    if let Some(info) = request.contentInformationRequest() {
      info.setContentType(Some(&NSString::from_str(kind)));
      if let Some(length) = length {
        info.setContentLength(length as i64);
      }
      info.setByteRangeAccessSupported(true);
    }
    let Some(data) = request.dataRequest() else {
      request.finishLoading();
      return;
    };
    let from = data.currentOffset().max(0) as u64;
    let asked_to = if data.requestsAllDataToEndOfResource() {
      u64::MAX
    } else {
      (data.requestedOffset().max(0) as u64).saturating_add(data.requestedLength().max(0) as u64)
    };
    let to = length.map_or(asked_to, |length| asked_to.min(length));
    let mut part = vec![0; PART];
    let mut at = from;
    while at < to {
      if request.isCancelled() {
        return;
      }
      if at >= from.saturating_add(AT_ONCE)
        && let (Some(length), Some(shared)) = (length, shared.upgrade())
        && plays_to(&shared, length).is_some_and(|plays_to| at >= plays_to)
      {
        drop(shared);
        std::thread::sleep(PACE);
        continue;
      }
      let wanted = usize::try_from(to - at).unwrap_or(usize::MAX).min(PART);
      match reader.read_at(at, &mut part[..wanted]) {
        Ok(0) => break,
        Ok(read) => {
          data.respondWithData(&NSData::with_bytes(&part[..read]));
          at += read as u64;
        }
        Err(error) => {
          tracing::debug!(%error, at, "a range of a video could not be read");
          let error = NSError::new(-1, &NSString::from_str("lurq.media"));
          request.finishLoadingWithError(Some(&error));
          return;
        }
      }
    }
    request.finishLoading();
  }
}

/// How far into its `length` bytes the media is to be read now: as far as what is read ahead of where it is, by
/// the share of its length that is. `None` until its length in time is known.
fn plays_to(shared: &Shared, length: u64) -> Option<u64> {
  let (position, duration) = {
    let inner = shared.inner();
    // A media that waits for bytes is given them, whatever is read ahead: where it is in its bytes is only told
    // by its share of its time, and where what it plays takes more bytes than that, it would wait for them for
    // good.
    if inner.waiting {
      return None;
    }
    (inner.position(), inner.duration?)
  };
  if duration <= 0.0 {
    return None;
  }
  let ahead = if shared.eager.load(Ordering::Acquire) {
    AHEAD_WHILE_PLAYING
  } else {
    AHEAD_UNTIL_PLAYED
  };
  let share = ((position + ahead) / duration).clamp(0.0, 1.0);
  Some((share * length as f64) as u64)
}

/// The system's name for what is in a reader whose server says `content_type`, at an address named `name`.
fn kind_of(content_type: Option<&str>, name: &str) -> &'static str {
  let named = |ending: &str| match ending {
    "mov" | "qt" => Some("com.apple.quicktime-movie"),
    "m4v" => Some("com.apple.m4v-video"),
    "m4a" => Some("com.apple.m4a-audio"),
    "mp3" => Some("public.mp3"),
    "wav" => Some("com.microsoft.waveform-audio"),
    "aac" => Some("public.aac-audio"),
    "mp4" => Some("public.mpeg-4"),
    _ => None,
  };
  let said = content_type.map(|kind| kind.split(';').next().unwrap_or(kind).trim().to_ascii_lowercase());
  let by_type = match said.as_deref() {
    Some("video/quicktime") => Some("com.apple.quicktime-movie"),
    Some("video/x-m4v") => Some("com.apple.m4v-video"),
    Some("audio/mp4" | "audio/x-m4a" | "audio/m4a") => Some("com.apple.m4a-audio"),
    Some("audio/mpeg" | "audio/mp3") => Some("public.mp3"),
    Some("audio/wav" | "audio/x-wav" | "audio/wave") => Some("com.microsoft.waveform-audio"),
    Some("audio/aac") => Some("public.aac-audio"),
    Some("video/mp4") => Some("public.mpeg-4"),
    _ => None,
  };
  let ending = name
    .split(['?', '#'])
    .next()
    .and_then(|name| name.rsplit_once('.'))
    .map(|(_, ending)| ending.to_ascii_lowercase());
  by_type
    .or_else(|| ending.as_deref().and_then(named))
    .unwrap_or("public.mpeg-4")
}

#[cfg(test)]
mod tests {
  use super::kind_of;

  #[test]
  fn what_the_server_says_names_the_kind_before_the_ending() {
    assert_eq!(kind_of(Some("video/quicktime"), "a.mp4"), "com.apple.quicktime-movie");
    assert_eq!(kind_of(Some("video/mp4; codecs=avc1"), "a.mov"), "public.mpeg-4");
  }

  #[test]
  fn an_ending_names_the_kind_where_the_server_says_nothing_it_knows() {
    assert_eq!(kind_of(None, "https://h/clip.MOV?x=1"), "com.apple.quicktime-movie");
    assert_eq!(
      kind_of(Some("application/octet-stream"), "https://h/song.m4a"),
      "com.apple.m4a-audio"
    );
    assert_eq!(kind_of(None, "https://h/clip"), "public.mpeg-4");
  }
}
