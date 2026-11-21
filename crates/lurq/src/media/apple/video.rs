//! A video on macOS and iOS: the system's player (`AVPlayer`) decodes it, plays its sound and keeps the two
//! together, and reads an address by itself; the frames it shows are taken from it and copied into the image a
//! [`Video`](crate::components::Video) draws.
//!
//! The player is the main thread's, as the system has it: it is made there, told there and let go there, by work
//! given to the main queue. The media's thread never touches it. It asks the player's output for the frame of the
//! moment, which an output is made to be asked from another thread, and reads what the main thread last saw of
//! the player. So a video opens in a process whose main thread runs its loop, which an app's does; in a tool or
//! under the test harness, where the main thread only waits, it waits to open for as long as it is held.
//!
//! A frame stays on the graphics card from the decoder to the screen where the `wgpu` renderer draws on Metal
//! ([`CvPixelBufferFrames`]): the buffer the decoder gave is the texture that is drawn, and nothing of it is
//! copied. Where nothing draws such frames (another renderer, none yet) a frame is copied through memory once.
//!
//! What is not here: a clip its track turns (a phone held upright) is drawn as it was filmed, on its side.

use std::{
  cell::RefCell,
  ffi::c_void,
  path::PathBuf,
  sync::{
    Arc, Mutex, MutexGuard,
    atomic::{AtomicBool, AtomicU64, Ordering},
  },
  time::{Duration, Instant},
};

use block2::RcBlock;
use objc2::{
  AnyThread, MainThreadMarker,
  rc::{Retained, autoreleasepool},
  runtime::{AnyObject, Bool},
};
use objc2_av_foundation::{
  AVMediaTypeAudio, AVPlayer, AVPlayerItem, AVPlayerItemStatus, AVPlayerItemVideoOutput, AVPlayerStatus,
  AVPlayerTimeControlStatus, AVURLAsset,
};
use objc2_core_foundation::CFString;
use objc2_core_media::{CMTime, CMTimeFlags, kCMTimeZero};
use objc2_core_video::{
  CVGetCurrentHostTime, CVGetHostClockFrequency, CVPixelBuffer, CVPixelBufferGetBaseAddressOfPlane,
  CVPixelBufferGetBytesPerRowOfPlane, CVPixelBufferGetHeight, CVPixelBufferGetPlaneCount, CVPixelBufferGetWidth,
  CVPixelBufferLockBaseAddress, CVPixelBufferLockFlags, CVPixelBufferUnlockBaseAddress,
  kCVPixelBufferMetalCompatibilityKey, kCVPixelBufferPixelFormatTypeKey,
  kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange,
};
use objc2_foundation::{NSDictionary, NSError, NSNumber, NSString, NSURL};

use super::super::{Command, Headers, MediaError, Opened, Shared};
use crate::images::{
  CvPixelBufferFrames, ImageData, MacosCvPixelBuffer, NativeImageData, cv_pixel_buffer_frames_drawn,
};

/// How often the thread asks for a frame while it plays, and while it waits for one it was sent to.
const STEP: Duration = Duration::from_millis(4);
const STILL: Duration = Duration::from_millis(10);
/// How often the main thread is asked what the player is: while it opens, and while it plays.
const OPENING: Duration = Duration::from_millis(15);
const LOOKS: Duration = Duration::from_millis(50);
/// The media's clock is set to the player's where they are this far apart, in seconds.
const APART: f64 = 0.03;
/// How long a frame is waited for where the player was sent while it stood: at the same frame, none comes.
const EXPECTED: Duration = Duration::from_secs(2);
/// A player that stands gives its first frame by itself; one that has not after this long is sent to where it is,
/// which makes it show the frame there.
const NUDGE: Duration = Duration::from_millis(250);
/// The units of a second the player is sent somewhere in.
const SCALE: i32 = 60_000;
/// How far the system's player fetches ahead of where it is, in seconds, before it was first told to play: a video
/// opened ahead and left paused takes its start and no more, as the app's loader has it on Windows. Left to itself
/// it reads on while it stands, from a host that gives one client only so much, beside the video that plays.
const AHEAD_UNTIL_PLAYED: f64 = 1.0;
/// How far it fetches ahead once it plays: a few seconds against a slow moment of the network, and not the whole
/// video, which it would read as fast as the host gives, at the cost of the radio, the battery and the person's data.
const AHEAD_WHILE_PLAYING: f64 = 10.0;

unsafe extern "C" {
  /// The main queue of libdispatch, which `dispatch_get_main_queue()` returns.
  static _dispatch_main_q: c_void;
  fn dispatch_async_f(queue: *const c_void, context: *mut c_void, work: extern "C" fn(*mut c_void));
}

/// Runs `work` on the main thread, as the main queue gets to it: after what was given to it before.
fn on_main(work: impl FnOnce(MainThreadMarker) + Send + 'static) {
  type Work = Box<dyn FnOnce(MainThreadMarker) + Send>;
  extern "C" fn run(context: *mut c_void) {
    // SAFETY: `context` is the box made below, given to this call once.
    let work = unsafe { Box::from_raw(context.cast::<Work>()) };
    // The main queue runs on the main thread.
    if let Some(main) = MainThreadMarker::new() {
      work(main);
    }
  }
  let context = Box::into_raw(Box::new(Box::new(work) as Work)).cast::<c_void>();
  // SAFETY: the main queue lives as long as the process; `run` takes the box back exactly once.
  unsafe { dispatch_async_f(&raw const _dispatch_main_q, context, run) };
}

/// What only the main thread reads and writes, held by what any thread may hold.
struct MainOnly<T>(T);

// SAFETY: what is in it is reached with a marker of the main thread alone.
unsafe impl<T> Send for MainOnly<T> {}
// SAFETY: as above.
unsafe impl<T> Sync for MainOnly<T> {}

impl<T> MainOnly<T> {
  fn get(&self, _: MainThreadMarker) -> &T {
    &self.0
  }
}

/// The output the frames are taken from, on its way to the main thread, which gives it to the player's item.
struct Output(Retained<AVPlayerItemVideoOutput>);

// SAFETY: an output is asked for its frames from a thread other than the player's: it is made for a display's
// own thread. The main thread only gives it to the item.
unsafe impl Send for Output {}

/// The player, on the main thread.
struct Player {
  player: Retained<AVPlayer>,
  item: Retained<AVPlayerItem>,
  /// How fast it goes while it plays: a player told its speed plays, so one that stands is told when it starts.
  rate: f32,
}

/// Whether the player's item opened.
#[derive(Clone, PartialEq)]
enum State {
  Opening,
  Ready,
  Failed(MediaError),
}

/// What the player does about time.
#[derive(Clone, Copy, PartialEq)]
enum Control {
  Paused,
  /// Told to play, it waits for what it plays.
  Waiting,
  Playing,
}

/// What the main thread last saw of the player.
#[derive(Clone)]
struct Seen {
  /// How many of the things the player was told it had done by then.
  done: u64,
  state: State,
  duration: Option<f64>,
  size: Option<(u32, u32)>,
  sound: bool,
  control: Control,
  at: f64,
}

/// What the media's thread and the main thread share.
struct Link {
  player: MainOnly<RefCell<Option<Player>>>,
  seen: Mutex<Seen>,
  /// How many of the things the player was told it has done.
  done: AtomicU64,
  /// How many of the times it was sent somewhere it has arrived, or was sent on before it did.
  arrived: AtomicU64,
  /// Whether the main thread has been asked to look at the player, and has not yet.
  asked: AtomicBool,
}

impl Link {
  fn seen(&self) -> MutexGuard<'_, Seen> {
    self.seen.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
  }

  /// Has the main thread do `work` with the player, and count it done.
  fn tell(self: &Arc<Self>, work: impl FnOnce(&mut Player) + Send + 'static) {
    let link = self.clone();
    on_main(move |main| {
      if let Some(player) = link.player.get(main).borrow_mut().as_mut() {
        work(player);
      }
      link.done.fetch_add(1, Ordering::Release);
    });
  }

  /// Has the main thread look at the player, unless it was asked to and has not yet.
  fn look(self: &Arc<Self>) {
    if self.asked.swap(true, Ordering::AcqRel) {
      return;
    }
    let link = self.clone();
    on_main(move |main| {
      link.asked.store(false, Ordering::Release);
      look(&link, main);
    });
  }
}

/// Lets the player go, on the main thread, once nothing plays it.
struct Ends(Arc<Link>);

impl Drop for Ends {
  fn drop(&mut self) {
    let link = self.0.clone();
    on_main(move |main| {
      if let Some(player) = link.player.get(main).borrow_mut().take() {
        // SAFETY: on the main thread, as every call of the player.
        unsafe { player.player.pause() };
      }
    });
  }
}

/// A time of the player's, in seconds: `None` for one that is no number (unknown, or without end).
fn seconds(time: CMTime) -> Option<f64> {
  if !time.flags.contains(CMTimeFlags::Valid) {
    return None;
  }
  // SAFETY: a time that is valid is read.
  Some(unsafe { time.seconds() }).filter(|seconds| seconds.is_finite())
}

/// What the system's error is, as a media says it: by the tables AVFoundation and the loading of an address
/// answer from.
fn failed(error: &NSError) -> MediaError {
  // A file that is no media, a kind there is no decoder for, a file it cannot read through.
  const UNSUPPORTED: [isize; 6] = [-11_828, -11_829, -11_833, -11_838, -11_849, -11_821];
  let said = || Arc::<str>::from(error.localizedDescription().to_string());
  match (error.domain().to_string().as_str(), error.code()) {
    ("AVFoundationErrorDomain", code) if UNSUPPORTED.contains(&code) => MediaError::Unsupported,
    ("NSURLErrorDomain", -1100) | ("NSCocoaErrorDomain", 4 | 260) => MediaError::NotFound,
    ("NSURLErrorDomain", -1001) => MediaError::TimedOut,
    ("NSURLErrorDomain", _) => MediaError::Network(said()),
    _ => MediaError::Other(said()),
  }
}

/// Writes what the player is into what the media's thread reads.
fn look(link: &Link, main: MainThreadMarker) {
  let player = link.player.get(main).borrow();
  let Some(Player { player, item, .. }) = player.as_ref() else {
    return;
  };
  // SAFETY: on the main thread, as every call of the player and its item.
  let seen = unsafe {
    let state = if item.status() == AVPlayerItemStatus::Failed {
      State::Failed(item.error().map_or_else(
        || MediaError::Other(Arc::from("the system's player could not open it")),
        |error| failed(&error),
      ))
    } else if player.status() == AVPlayerStatus::Failed {
      State::Failed(player.error().map_or_else(
        || MediaError::Other(Arc::from("the system's player failed")),
        |error| failed(&error),
      ))
    } else if item.status() == AVPlayerItemStatus::ReadyToPlay {
      State::Ready
    } else {
      State::Opening
    };
    let tracks = item.tracks();
    let sound = (0..tracks.count()).any(|index| {
      let kind = tracks.objectAtIndex(index).assetTrack().map(|track| track.mediaType());
      matches!((kind, AVMediaTypeAudio), (Some(kind), Some(audio)) if kind.isEqualToString(audio))
    });
    let shown = item.presentationSize();
    // Two planes of which the second is half as wide and as high: a picture is an even number each way.
    let size = Some((shown.width.round() as u32 & !1, shown.height.round() as u32 & !1))
      .filter(|(width, height)| *width > 0 && *height > 0);
    let control = match player.timeControlStatus() {
      AVPlayerTimeControlStatus::Playing => Control::Playing,
      AVPlayerTimeControlStatus::WaitingToPlayAtSpecifiedRate => Control::Waiting,
      _ => Control::Paused,
    };
    Seen {
      done: link.done.load(Ordering::Acquire),
      state,
      duration: seconds(item.duration()).filter(|duration| *duration > 0.0),
      size,
      sound,
      control,
      at: seconds(player.currentTime()).unwrap_or(0.0),
    }
  };
  *link.seen() = seen;
}

/// What is played: a file, or an address the player reads by itself, with the headers its requests carry.
enum Place {
  File(PathBuf),
  Url(Arc<str>, Headers),
}

/// The player's item of what is at `address`, whose requests carry `headers`.
///
/// `AVURLAssetHTTPHeaderFieldsKey` is not in the SDK's headers: it is the key AVFoundation reads the headers of
/// an asset's requests from, which players on Apple's systems have used for years, and the only way to give them
/// short of reading the address through a delegate of the asset's resource loader.
fn item_at(address: &NSURL, headers: &Headers, main: MainThreadMarker) -> Retained<AVPlayerItem> {
  // SAFETY: on the main thread, as every call of the player and its item; the dictionaries hold strings, as the
  // asset reads them.
  unsafe {
    if headers.is_empty() {
      return AVPlayerItem::playerItemWithURL(address, main);
    }
    let names: Vec<Retained<NSString>> = headers.iter().map(|(name, _)| NSString::from_str(name)).collect();
    let values: Vec<Retained<NSString>> = headers.iter().map(|(_, value)| NSString::from_str(value)).collect();
    let fields = NSDictionary::<NSString, NSString>::from_retained_objects(
      &names.iter().map(|name| &**name).collect::<Vec<_>>(),
      &values,
    );
    let fields: &AnyObject = &fields;
    let options = NSDictionary::<NSString, AnyObject>::from_slices(
      &[&*NSString::from_str("AVURLAssetHTTPHeaderFieldsKey")],
      &[fields],
    );
    let asset = AVURLAsset::URLAssetWithURL_options(address, Some(&options));
    AVPlayerItem::playerItemWithAsset(&asset, main)
  }
}

/// The ending of a file's name that says what `bytes` are: the player tells a file's kind by its name first.
fn ending(bytes: &[u8], name: Option<&str>) -> String {
  let named = name
    .map(|name| name.split(['?', '#']).next().unwrap_or(name))
    .and_then(|name| name.rsplit('/').next())
    .and_then(|name| name.rsplit_once('.'))
    .map(|(_, ending)| ending)
    .filter(|ending| (1..=5).contains(&ending.len()) && ending.bytes().all(|byte| byte.is_ascii_alphanumeric()));
  if let Some(ending) = named {
    return ending.to_ascii_lowercase();
  }
  let brand = (bytes.get(4..8) == Some(b"ftyp")).then(|| bytes.get(8..12)).flatten();
  match (brand, bytes) {
    (Some(b"qt  "), _) => "mov",
    (Some(b"M4A "), _) => "m4a",
    (Some(_), _) => "mp4",
    (None, [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'A', b'V', b'E', ..]) => "wav",
    (None, [b'I', b'D', b'3', ..] | [0xFF, 0xFB | 0xF3 | 0xF2, ..]) => "mp3",
    _ => "mp4",
  }
  .to_owned()
}

/// A file of the system's temporary folder with `bytes` in it: the player opens files and addresses, not memory.
fn kept(bytes: &[u8], name: Option<&str>) -> std::io::Result<PathBuf> {
  static NEXT: AtomicU64 = AtomicU64::new(0);
  let path = std::env::temp_dir().join(format!(
    "lurq-media-{}-{}.{}",
    std::process::id(),
    NEXT.fetch_add(1, Ordering::Relaxed),
    ending(bytes, name)
  ));
  std::fs::write(&path, bytes)?;
  Ok(path)
}

/// Runs a video to its end: opens `opened`, and does what it is told until nothing holds the media. With
/// `picture`, its frames are shown; without, it is only heard.
pub(super) fn run(shared: &Arc<Shared>, opened: Opened, picture: bool) {
  let mut written = None;
  let place = match opened {
    Opened::File(path) => match std::fs::canonicalize(&path) {
      Ok(path) => Place::File(path),
      Err(error) if error.kind() == std::io::ErrorKind::NotFound => return shared.fail(MediaError::NotFound),
      Err(error) => return shared.fail(MediaError::Other(Arc::from(error.to_string()))),
    },
    Opened::Bytes(bytes, name) => match kept(&bytes, name.as_deref()) {
      Ok(path) => {
        written = Some(path.clone());
        Place::File(path)
      }
      Err(error) => return shared.fail(MediaError::Other(Arc::from(error.to_string()))),
    },
    Opened::Url(address, headers) => Place::Url(address, headers),
    // Only Windows reads an address through the app's loader.
    Opened::Reader(..) => {
      return shared.fail(MediaError::Unavailable(Arc::from(
        "the system's player reads an address itself",
      )));
    }
  };
  play(shared, place, picture);
  // The file of its bytes is the media's own, and goes with it.
  if let Some(path) = written {
    let _ = std::fs::remove_file(path);
  }
}

/// An output that gives frames as two planes: gray, and color at half the width and the height, in buffers Metal
/// makes textures of.
fn output() -> Retained<AVPlayerItemVideoOutput> {
  // SAFETY: the keys are strings of the system's, which a dictionary takes as its own; the output is made with
  // what it is told its frames are to be.
  unsafe {
    let key = |key: &'static CFString| -> &'static NSString { &*std::ptr::from_ref::<CFString>(key).cast() };
    let format = NSNumber::new_u32(kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange);
    let metal = NSNumber::new_bool(true);
    let (format, metal): (&AnyObject, &AnyObject) = (&format, &metal);
    let frames = NSDictionary::<NSString, AnyObject>::from_slices(
      &[
        key(kCVPixelBufferPixelFormatTypeKey),
        key(kCVPixelBufferMetalCompatibilityKey),
      ],
      &[format, metal],
    );
    AVPlayerItemVideoOutput::initWithPixelBufferAttributes(AVPlayerItemVideoOutput::alloc(), Some(&frames))
  }
}

/// The time of the machine's clock now, in seconds: what an output is asked the player's time of.
fn now() -> f64 {
  CVGetCurrentHostTime() as f64 / CVGetHostClockFrequency()
}

/// A frame of black, `width` by `height`: the picture before its first frame.
fn black(width: u32, height: u32) -> Vec<u8> {
  let gray = (width * height) as usize;
  let mut frame = vec![128; gray * 3 / 2];
  frame[..gray].fill(16);
  frame
}

/// The image of a media `width` by `height` before its first frame: of frames that stay on the card where
/// something draws them there, with those frames; of memory, black, where nothing does.
fn first_picture(width: u32, height: u32) -> (ImageData, Option<CvPixelBufferFrames>) {
  if cv_pixel_buffer_frames_drawn() {
    let frames = CvPixelBufferFrames::new();
    let image = NativeImageData::from_cv_pixel_buffer_frames(width, height, frames.clone()).image_data();
    (image, Some(frames))
  } else {
    (ImageData::streaming_nv12(black(width, height), width, height), None)
  }
}

/// Shows the frame in `buffer`: as it is, on the card, where something draws it there, and copied into the image
/// of the media where nothing does. The image is made anew where the frame is of another size than it, or goes
/// another way than it did. `true` where the image is another one now.
fn show(image: &mut Option<ImageData>, frames: &mut Option<CvPixelBufferFrames>, buffer: &CVPixelBuffer) -> bool {
  let (width, height) = (CVPixelBufferGetWidth(buffer) & !1, CVPixelBufferGetHeight(buffer) & !1);
  if width == 0 || height == 0 || CVPixelBufferGetPlaneCount(buffer) != 2 {
    return false;
  }
  let another = frames.is_some() != cv_pixel_buffer_frames_drawn()
    || !image
      .as_ref()
      .is_some_and(|image| (image.width() as usize, image.height() as usize) == (width, height));
  if another {
    let (made, on_card) = first_picture(width as u32, height as u32);
    (*image, *frames) = (Some(made), on_card);
  }
  if let Some(frames) = frames.as_ref() {
    // SAFETY: the buffer is one the output gave, and is kept until another frame is shown and drawn.
    frames.show(unsafe { MacosCvPixelBuffer::retain(std::ptr::from_ref(buffer).cast_mut().cast()) });
    return another;
  }
  let Some(image) = image.as_ref() else {
    return false;
  };
  // SAFETY: the buffer is locked to be read while its planes are, and a plane holds its rows of `stride` bytes
  // each: `height` of gray, and half as many of color.
  unsafe {
    if CVPixelBufferLockBaseAddress(buffer, CVPixelBufferLockFlags::ReadOnly) != 0 {
      return another;
    }
    let mut frame = image.take_streaming_nv12_buffer().unwrap_or_default();
    frame.clear();
    frame.reserve(width * height * 3 / 2);
    let mut whole = true;
    for (plane, rows) in [(0, height), (1, height / 2)] {
      let (start, stride) = (
        CVPixelBufferGetBaseAddressOfPlane(buffer, plane).cast::<u8>(),
        CVPixelBufferGetBytesPerRowOfPlane(buffer, plane),
      );
      if start.is_null() || stride < width {
        whole = false;
        break;
      }
      for row in 0..rows {
        frame.extend_from_slice(std::slice::from_raw_parts(start.add(row * stride), width));
      }
    }
    CVPixelBufferUnlockBaseAddress(buffer, CVPixelBufferLockFlags::ReadOnly);
    if whole {
      image.set_streaming_nv12(frame);
    }
  }
  another
}

/// What is done once the player arrived where it was sent.
type Then = Box<dyn Fn() + Send + Sync>;

/// Sends the player to `to`, and counts it arrived when the player says so: there, or sent on before it was.
fn go(link: &Arc<Link>, player: &Player, to: f64, then: Option<Then>) {
  let link = link.clone();
  let arrived = RcBlock::new(move |_: Bool| {
    link.arrived.fetch_add(1, Ordering::Release);
    if let Some(then) = &then {
      then();
    }
  });
  // SAFETY: on the main thread; the block is called once, from where the player chooses, and holds what any
  // thread may.
  unsafe {
    player
      .player
      .seekToTime_toleranceBefore_toleranceAfter_completionHandler(
        CMTime::with_seconds(to, SCALE),
        kCMTimeZero,
        kCMTimeZero,
        &arrived,
      );
  }
}

fn play(shared: &Arc<Shared>, place: Place, picture: bool) {
  let output = picture.then(output);
  let link = Arc::new(Link {
    player: MainOnly(RefCell::new(None)),
    seen: Mutex::new(Seen {
      done: 0,
      state: State::Opening,
      duration: None,
      size: None,
      sound: false,
      control: Control::Paused,
      at: 0.0,
    }),
    done: AtomicU64::new(0),
    arrived: AtomicU64::new(0),
    asked: AtomicBool::new(false),
  });
  let (volume, muted) = {
    let inner = shared.inner();
    (inner.volume, inner.muted)
  };
  // The player is made on the main thread, with the output its frames are taken from.
  on_main({
    let (link, output) = (link.clone(), output.clone().map(Output));
    move |main| {
      let output = output.map(|Output(output)| output);
      let (address, headers) = match &place {
        Place::File(path) => (NSURL::from_file_path(path), None),
        Place::Url(address, headers) => (NSURL::URLWithString(&NSString::from_str(address)), Some(headers)),
      };
      let Some(address) = address else {
        link.seen().state = State::Failed(MediaError::NotFound);
        return;
      };
      // SAFETY: on the main thread, as every call of the player and its item.
      unsafe {
        let item = match headers {
          Some(headers) => item_at(&address, headers, main),
          None => AVPlayerItem::playerItemWithURL(&address, main),
        };
        if let Some(output) = &output {
          item.addOutput(output);
        }
        item.setPreferredForwardBufferDuration(AHEAD_UNTIL_PLAYED);
        let player = AVPlayer::playerWithPlayerItem(Some(&item), main);
        player.setVolume(volume);
        player.setMuted(muted);
        *link.player.get(main).borrow_mut() = Some(Player {
          player,
          item,
          rate: 1.0,
        });
      }
      look(&link, main);
    }
  });
  let _ends = Ends(link.clone());

  // How many things the player was told, how many times the media was sent somewhere, and the player.
  let (mut told, mut heard, mut sent) = (0_u64, 0_u64, 0_u64);
  let mut opened = false;
  let mut image: Option<ImageData> = None;
  // The frames that stay on the card, where the image is of them.
  let mut frames: Option<CvPixelBufferFrames> = None;
  // Since when a frame is waited for while it stands (it opened, it was sent somewhere), and whether the player
  // was sent to where it is for it.
  let mut expected: Option<Instant> = None;
  let mut nudged = false;
  let mut looked: Option<Instant> = None;
  loop {
    let playing = shared.inner().playing;
    let wait = match (opened, playing, expected) {
      (false, ..) => OPENING,
      (true, true, _) => STEP,
      (true, false, Some(_)) => STILL,
      (true, false, None) => Duration::from_secs(3600),
    };
    for command in shared.take(Some(wait)) {
      match command {
        Command::Close => return,
        Command::Play => {
          let (again, rate) = {
            let mut inner = shared.inner();
            let again = inner.ended;
            if again {
              inner.ended = false;
              inner.at = 0.0;
            }
            inner.since = Instant::now();
            inner.playing = true;
            (again, inner.rate as f32)
          };
          // SAFETY: on the main thread, as every call of the player.
          let start = |player: &Player| unsafe {
            player.item.setPreferredForwardBufferDuration(AHEAD_WHILE_PLAYING);
            player.player.play();
            if player.rate != 1.0 {
              player.player.setRate(player.rate);
            }
          };
          told += 1;
          if again {
            // From its end it goes to its start first, and plays once it is there: a player told to play at its
            // end is at its end again at once.
            sent += 1;
            let link = link.clone();
            on_main(move |main| {
              let then: Then = Box::new({
                let link = link.clone();
                move || {
                  link.tell(move |player| {
                    player.rate = rate;
                    start(player);
                  });
                }
              });
              match link.player.get(main).borrow().as_ref() {
                Some(player) => go(&link, player, 0.0, Some(then)),
                None => then(),
              }
            });
          } else {
            link.tell(move |player| {
              player.rate = rate;
              start(player);
            });
          }
          // Who draws it may have asked whether it plays before this thread heard the command.
          shared.wake();
        }
        Command::Pause => {
          let mut inner = shared.inner();
          inner.since = Instant::now();
          inner.playing = false;
          drop(inner);
          told += 1;
          // SAFETY: on the main thread, as every call of the player.
          link.tell(|player| unsafe { player.player.pause() });
        }
        Command::Seek(to) => {
          heard += 1;
          sent += 1;
          told += 1;
          {
            let mut inner = shared.inner();
            let to = inner.duration.map_or(to, |duration| to.min(duration));
            inner.at = to;
            inner.since = Instant::now();
            inner.ended = false;
          }
          link.tell({
            let link = link.clone();
            move |player| go(&link, player, to, None)
          });
          expected = Some(Instant::now());
          nudged = true;
        }
        Command::Rate(rate) => {
          let mut inner = shared.inner();
          inner.hold();
          inner.rate = rate;
          drop(inner);
          told += 1;
          // A player that stands is not told its speed, which would start it.
          link.tell(move |player| unsafe {
            player.rate = rate as f32;
            if player.player.timeControlStatus() != AVPlayerTimeControlStatus::Paused {
              player.player.setRate(rate as f32);
            }
          });
        }
        Command::Volume(to) => {
          told += 1;
          // SAFETY: on the main thread, as every call of the player.
          link.tell(move |player| unsafe { player.player.setVolume(to) });
        }
        Command::Muted(to) => {
          told += 1;
          // SAFETY: on the main thread, as every call of the player.
          link.tell(move |player| unsafe { player.player.setMuted(to) });
        }
      }
    }
    let playing = shared.inner().playing;
    // What the player is changes by itself while it opens and while it plays: the main thread is asked then.
    let every = if opened { LOOKS } else { OPENING };
    if (!opened || playing) && looked.is_none_or(|looked| looked.elapsed() >= every) {
      looked = Some(Instant::now());
      link.look();
    }
    let seen = link.seen().clone();
    if let State::Failed(why) = seen.state {
      return shared.fail(why);
    }
    if !opened {
      if seen.state != State::Ready {
        continue;
      }
      opened = true;
      let size = seen.size.filter(|_| picture);
      (image, frames) = match size {
        Some((width, height)) => {
          let (image, frames) = first_picture(width, height);
          (Some(image), frames)
        }
        None => (None, None),
      };
      {
        let mut inner = shared.inner();
        inner.duration = seen.duration;
        inner.size = size;
        inner.sound = seen.sound;
        inner.frame = image.clone();
        inner.opened = true;
        inner.waiting = false;
        inner.since = Instant::now();
      }
      expected = image.is_some().then(Instant::now);
      shared.wake();
    }
    // The frame of the moment, where the player has one that was not shown.
    let mut shown = None;
    if let Some(output) = &output {
      // SAFETY: an output is asked from this thread: for the player's time at a moment of the machine's clock,
      // and for the frame of that time.
      let time = unsafe { output.itemTimeForHostTime(now()) };
      shown = seconds(time);
      if shown.is_some() && (playing || expected.is_some()) {
        let another = autoreleasepool(|_| unsafe {
          if !output.hasNewPixelBufferForItemTime(time) {
            return None;
          }
          output
            .copyPixelBufferForItemTime_itemTimeForDisplay(time, std::ptr::null_mut())
            .map(|buffer| show(&mut image, &mut frames, &buffer))
        });
        if let Some(another) = another {
          (expected, nudged) = (None, false);
          shared.presented();
          if another {
            let mut inner = shared.inner();
            inner.size = image.as_ref().map(|image| (image.width(), image.height()));
            inner.frame = image.clone();
            drop(inner);
            shared.wake();
          }
        }
      }
    }
    if let Some(since) = expected {
      if !nudged && since.elapsed() >= NUDGE {
        nudged = true;
        sent += 1;
        told += 1;
        let (to, at) = (link.clone(), shared.inner().position());
        link.tell(move |player| go(&to, player, at, None));
      }
      if since.elapsed() >= EXPECTED {
        expected = None;
      }
    }
    let mut inner = shared.inner();
    if !inner.playing {
      continue;
    }
    // What was seen of the player is of now once the player has done all it was told.
    let current = seen.done == told;
    let waits = current && seen.control == Control::Waiting;
    let mut changed = false;
    if waits != inner.waiting {
      inner.hold();
      inner.waiting = waits;
      changed = true;
    }
    if current && seen.control == Control::Paused {
      // The player stopped by itself. At the end of what it plays it ended; anywhere else the system stopped it
      // (a call, another app's sound, the headphones pulled out), and it is paused there.
      let over = inner.duration.is_none_or(|duration| seen.at >= duration - 0.3);
      if over && inner.looped && inner.playing {
        // Looped: the player is sent to its start in the item it has, and plays on; it never ends.
        inner.at = 0.0;
        inner.rounds += 1;
        inner.since = Instant::now();
        inner.sent += 1;
        drop(inner);
        shared.tell(Command::Seek(0.0));
        shared.tell(Command::Play);
        shared.wake();
        continue;
      }
      inner.at = match (over, inner.duration) {
        (true, Some(duration)) => duration,
        _ => seen.at,
      };
      inner.since = Instant::now();
      inner.ended = over;
      inner.playing = false;
      changed = true;
    } else if !waits && inner.sent == heard && link.arrived.load(Ordering::Acquire) == sent {
      // The media's clock is the player's: what is seen and heard is where it is. A media that is only heard has
      // no output to ask, and goes by what the main thread saw.
      if output.is_none() && current && seen.control == Control::Playing {
        shared.presented_in(&mut inner);
      }
      let at = shown.or_else(|| (output.is_none() && current && seen.control == Control::Playing).then_some(seen.at));
      if let Some(at) = at
        && (at - inner.position()).abs() > APART
      {
        inner.at = at;
        inner.since = Instant::now();
      }
    }
    drop(inner);
    if changed {
      shared.wake();
    }
  }
}
