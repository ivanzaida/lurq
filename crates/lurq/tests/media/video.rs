//! The checks of a video, and what they and the checks of a sound share: run by the test harness on Windows
//! (`media_tests.rs`), and on macOS by a program whose main thread runs its loop (`media_video_apple.rs`), since
//! the system's player of a video is the main thread's there.

use std::{
  path::PathBuf,
  sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
  },
  time::{Duration, Instant},
};

use lurq::{
  media::{Media, MediaError, MediaState},
  resources::{Loader, Source},
};

pub fn clip() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../gallery/assets/media/sample.mp4")
}

/// The clip, opened and silenced.
pub fn quiet(source: &Source) -> Media {
  let media = Media::open(&Loader::new(), source);
  media.set_muted(true);
  media
}

/// Waits until `is` holds of the media's state, for at most ten seconds.
#[track_caller]
pub fn until(media: &Media, what: &str, is: impl Fn(&MediaState) -> bool) -> MediaState {
  let started = Instant::now();
  loop {
    let state = media.state();
    if is(&state) {
      return state;
    }
    assert!(
      started.elapsed() < Duration::from_secs(10),
      "it never came to: {what}. It is {state:?}"
    );
    std::thread::sleep(Duration::from_millis(10));
  }
}

pub fn a_file_opens_with_its_size_its_length_and_its_first_frame() {
  let woken = Arc::new(AtomicUsize::new(0));
  let media = quiet(&Source::file(clip()));
  media.on_change({
    let woken = woken.clone();
    move || {
      woken.fetch_add(1, Ordering::Relaxed);
    }
  });
  let state = until(&media, "opened", |state| !state.waiting);
  assert_eq!(state.error, None);
  assert_eq!(state.size, Some((640, 360)));
  assert!(
    state.duration.is_some_and(|duration| (duration - 15.0).abs() < 0.2),
    "{:?}",
    state.duration
  );
  assert!(!state.playing && !state.ended && state.position == 0.0);
  let image = media.frame().expect("its picture");
  assert_eq!((image.width(), image.height()), (640, 360));
  // The first frame comes without playing: the picture is not the black it starts as, which no frame has been
  // put into. It may have come by now: with the system's player it comes as the clip opens.
  let started = Instant::now();
  while image.version() == 0 {
    assert!(started.elapsed() < Duration::from_secs(10), "no first frame");
    std::thread::sleep(Duration::from_millis(10));
  }
}

pub fn it_plays_by_its_clock_shows_frames_and_stops_where_it_is_paused() {
  let media = quiet(&Source::file(clip()));
  until(&media, "opened", |state| !state.waiting);
  let image = media.frame().expect("its picture");
  media.play();
  until(&media, "playing", |state| state.playing);
  let before = image.version();
  std::thread::sleep(Duration::from_millis(600));
  let state = media.state();
  assert!(
    (0.4..0.9).contains(&state.position),
    "about 0.6 s after 0.6 s: {}",
    state.position
  );
  let shown = image.version() - before;
  assert!(
    (10..=22).contains(&shown),
    "about 18 frames of 30 a second in 0.6 s: {shown}"
  );
  media.pause();
  let paused = until(&media, "paused", |state| !state.playing).position;
  let held = image.version();
  std::thread::sleep(Duration::from_millis(200));
  assert_eq!(media.state().position, paused, "the clock stands");
  assert_eq!(image.version(), held, "and no frame comes");
  // Twice as fast.
  media.set_rate(2.0);
  media.play();
  until(&media, "playing", |state| state.playing && state.rate == 2.0);
  let from = media.state().position;
  std::thread::sleep(Duration::from_millis(500));
  let gone = media.state().position - from;
  assert!((0.8..1.3).contains(&gone), "about a second in half of one: {gone}");
}

pub fn it_goes_where_it_is_sent_and_ends_at_its_end() {
  let media = quiet(&Source::file(clip()));
  until(&media, "opened", |state| !state.waiting);
  let image = media.frame().expect("its picture");
  let before = image.version();
  media.seek(10.0);
  let state = until(&media, "at 10", |state| state.position == 10.0);
  assert!(!state.playing, "sent somewhere while paused, it stays paused");
  let started = Instant::now();
  while image.version() == before {
    assert!(started.elapsed() < Duration::from_secs(10), "no frame at 10");
    std::thread::sleep(Duration::from_millis(10));
  }
  // Near its end, playing: it ends, stops, and says so.
  media.seek(14.6);
  media.play();
  let ended = until(&media, "ended", |state| state.ended);
  assert!(!ended.playing);
  assert!(ended.duration.is_some_and(|duration| ended.position == duration));
  // Played again, it starts from its start.
  media.play();
  let again = until(&media, "playing again", |state| state.playing && !state.ended);
  assert!(again.position < 1.0, "{}", again.position);
}

pub fn bytes_play_as_a_file_does_and_what_is_no_video_says_why() {
  let bytes = std::fs::read(clip()).expect("the clip");
  let media = quiet(&Source::bytes(bytes));
  let state = until(&media, "opened", |state| !state.waiting);
  assert_eq!((state.error, state.size), (None, Some((640, 360))));
  let broken = Media::open(&Loader::new(), &Source::bytes(b"this is not a video".to_vec()));
  let state = until(&broken, "failed", |state| state.error.is_some());
  assert!(!state.playing && !state.waiting);
  assert_eq!(
    state.error,
    Some(MediaError::Unsupported),
    "what is no video is a kind it does not play"
  );
  let missing = Media::open(&Loader::new(), &Source::file("no/such/clip.mp4"));
  let state = until(&missing, "failed", |state| state.error.is_some());
  assert_eq!(state.error, Some(MediaError::NotFound));
  // An address the app does not allow is not asked for.
  let loader = Loader::new();
  loader.allow(|_| false);
  let refused = Media::open(&loader, &Source::url("https://example.com/clip.mp4"));
  assert_eq!(refused.state().error, Some(MediaError::Denied));
}

pub fn a_clip_has_its_sound_and_what_is_only_heard_of_it_has_no_picture() {
  let seen = quiet(&Source::file(clip()));
  let state = until(&seen, "opened", |state| !state.waiting);
  assert!(state.sound, "the clip's tone");
  // The same clip as a sound: its picture is not decoded.
  let heard = Media::sound(&Loader::new(), &Source::file(clip()));
  heard.set_muted(true);
  let state = until(&heard, "opened", |state| !state.waiting);
  assert_eq!((state.error, state.size, state.sound), (None, None, true));
  assert!(heard.frame().is_none());
  assert!(
    state.duration.is_some_and(|duration| (duration - 15.0).abs() < 0.2),
    "{:?}",
    state.duration
  );
  // It goes by what is heard, at any speed, and where it is sent.
  heard.play();
  until(&heard, "playing", |state| state.playing);
  std::thread::sleep(Duration::from_millis(600));
  let position = heard.state().position;
  assert!((0.4..0.9).contains(&position), "about 0.6 s after 0.6 s: {position}");
  heard.set_rate(2.0);
  until(&heard, "twice as fast", |state| state.rate == 2.0);
  let from = heard.state().position;
  std::thread::sleep(Duration::from_millis(500));
  let gone = heard.state().position - from;
  assert!((0.8..1.3).contains(&gone), "about a second in half of one: {gone}");
  heard.seek(10.0);
  std::thread::sleep(Duration::from_millis(300));
  let position = heard.state().position;
  assert!(
    (10.3..11.2).contains(&position),
    "0.6 s on from 10 at twice the speed: {position}"
  );
  heard.pause();
  let paused = until(&heard, "paused", |state| !state.playing).position;
  std::thread::sleep(Duration::from_millis(200));
  assert_eq!(heard.state().position, paused, "the clock stands");
}

/// The `Referer` a [`Pages`] server wants.
pub const PAGES: &str = "http://127.0.0.1/pages/";

/// A server on this machine that serves a file as a host of videos that serves only its own pages does: a request
/// without `Referer: PAGES` is answered 403, and one with it is given the part its `Range` asks for.
pub struct Pages {
  pub address: String,
  /// How many requests it answered 403.
  pub refused: Arc<AtomicUsize>,
  /// How many requests it answered with a part of the file.
  pub served: Arc<AtomicUsize>,
  /// Set, it answers 403 to every request from then on.
  pub refusing: Arc<AtomicBool>,
  /// How long it waits before it answers, in milliseconds.
  pub slow: Arc<AtomicU64>,
}

pub fn serve_to_its_pages(body: Vec<u8>) -> Pages {
  use std::io::{Read, Write};
  let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
  let pages = Pages {
    address: format!("http://{}/clip.mp4", listener.local_addr().unwrap()),
    refused: Arc::new(AtomicUsize::new(0)),
    served: Arc::new(AtomicUsize::new(0)),
    refusing: Arc::new(AtomicBool::new(false)),
    slow: Arc::new(AtomicU64::new(0)),
  };
  let body = Arc::new(body);
  let (refused, refusing, slow) = (pages.refused.clone(), pages.refusing.clone(), pages.slow.clone());
  let served = pages.served.clone();
  std::thread::spawn(move || {
    for stream in listener.incoming() {
      let Ok(mut stream) = stream else {
        return;
      };
      let (body, refused, refusing, slow) = (body.clone(), refused.clone(), refusing.clone(), slow.clone());
      let served = served.clone();
      std::thread::spawn(move || {
        let (mut request, mut chunk) = (Vec::new(), [0_u8; 1024]);
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
          match stream.read(&mut chunk) {
            Ok(read) if read > 0 => request.extend_from_slice(&chunk[..read]),
            _ => return,
          }
        }
        std::thread::sleep(Duration::from_millis(slow.load(Ordering::SeqCst)));
        let request = String::from_utf8_lossy(&request).to_lowercase();
        if !request.contains(&format!("referer: {PAGES}")) || refusing.load(Ordering::SeqCst) {
          refused.fetch_add(1, Ordering::SeqCst);
          let _ = stream.write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
          return;
        }
        served.fetch_add(1, Ordering::SeqCst);
        let len = body.len();
        let (from, to) = request
          .lines()
          .find_map(|line| line.strip_prefix("range: bytes="))
          .and_then(|range| {
            let (from, to) = range.split_once('-')?;
            Some((
              from.parse().ok()?,
              to.parse().map_or(len - 1, |to: usize| to.min(len - 1)),
            ))
          })
          .unwrap_or((0, len - 1));
        let head = format!(
          "HTTP/1.1 206 Partial Content\r\nContent-Type: video/mp4\r\nAccept-Ranges: bytes\r\nContent-Range: bytes {from}-{to}/{len}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
          to + 1 - from
        );
        let _ = stream.write_all(head.as_bytes());
        if !request.starts_with("head ") {
          let _ = stream.write_all(&body[from..=to]);
        }
      });
    }
  });
  pages
}

/// The clip, opened with `loader` and silenced.
fn quiet_with(loader: &Loader, source: &Source) -> Media {
  let media = Media::open(loader, source);
  media.set_muted(true);
  media
}

pub fn a_clip_at_an_address_is_asked_for_with_the_headers_of_its_source_and_goes_where_it_is_sent() {
  let pages = serve_to_its_pages(std::fs::read(clip()).expect("the clip"));
  let media = quiet(&Source::url(&pages.address).header("Referer", PAGES));
  let state = until(&media, "opened", |state| !state.waiting);
  assert_eq!((state.error, state.size), (None, Some((640, 360))));
  assert!(
    state.duration.is_some_and(|duration| (duration - 15.0).abs() < 0.2),
    "{:?}",
    state.duration
  );
  let image = media.frame().expect("its picture");
  let before = image.version();
  media.seek(10.0);
  until(&media, "at 10", |state| state.position == 10.0);
  let started = Instant::now();
  while image.version() == before {
    assert!(started.elapsed() < Duration::from_secs(10), "no frame at 10");
    std::thread::sleep(Duration::from_millis(10));
  }
  media.seek(14.6);
  media.play();
  until(&media, "ended", |state| state.ended);
  assert_eq!(pages.refused.load(Ordering::SeqCst), 0, "every request had the header");

  // Without it, or with one the server refuses, the server's answer is what the media says.
  for source in [
    Source::url(&pages.address),
    Source::url(&pages.address).header("Referer", "http://127.0.0.1/elsewhere/"),
  ] {
    let refused = quiet(&source);
    let state = until(&refused, "failed", |state| state.error.is_some());
    assert!(!state.playing && !state.waiting);
    #[cfg(windows)]
    assert_eq!(state.error, Some(MediaError::Server(403)));
  }
}

pub fn the_loaders_rule_gives_a_clip_its_headers_and_a_sources_own_take_their_place() {
  let pages = serve_to_its_pages(std::fs::read(clip()).expect("the clip"));
  let loader = Loader::new();
  loader.headers(|_| vec![("Referer".to_owned(), PAGES.to_owned())]);
  let media = quiet_with(&loader, &Source::url(&pages.address));
  let state = until(&media, "opened", |state| !state.waiting);
  assert_eq!((state.error, state.size), (None, Some((640, 360))));
  assert_eq!(pages.refused.load(Ordering::SeqCst), 0);
  let overridden = quiet_with(
    &loader,
    &Source::url(&pages.address).header("referer", "http://127.0.0.1/elsewhere/"),
  );
  let state = until(&overridden, "failed", |state| state.error.is_some());
  #[cfg(windows)]
  assert_eq!(state.error, Some(MediaError::Server(403)));
  let _ = state;
}

/// On Windows, where the app's loader reads what a media plays.
pub fn a_clip_waits_for_what_the_network_has_not_given_yet_and_fails_with_what_the_server_answered_on_the_way() {
  let pages = serve_to_its_pages(std::fs::read(clip()).expect("the clip"));
  let loader = Loader::new();
  loader.read_block(16 * 1024);
  let media = quiet_with(&loader, &Source::url(&pages.address).header("Referer", PAGES));
  until(&media, "opened", |state| !state.waiting && state.error.is_none());

  // Sent where nothing was read yet, it waits while the server takes its time.
  pages.slow.store(400, Ordering::SeqCst);
  media.seek(12.0);
  until(&media, "waiting", |state| state.waiting);
  let state = until(&media, "at 12, read", |state| !state.waiting);
  assert_eq!(state.error, None);
  pages.slow.store(0, Ordering::SeqCst);

  // The server refuses it on the way: the media fails, and says why as the server said it.
  pages.refusing.store(true, Ordering::SeqCst);
  media.seek(3.0);
  media.play();
  let state = until(&media, "failed", |state| state.error.is_some());
  assert_eq!(state.error, Some(MediaError::Server(403)));
  assert!(!state.playing && !state.waiting);
}

/// On Windows, where the app's loader reads what a media plays.
pub fn a_clip_whose_reads_ahead_come_in_time_does_not_wait_while_it_plays() {
  let pages = serve_to_its_pages(std::fs::read(clip()).expect("the clip"));
  // Blocks of 16 KiB, more of them than are kept, each a moment late: less than what the decoder has ahead of
  // the picture, which it reads ahead of.
  pages.slow.store(60, Ordering::SeqCst);
  let loader = Loader::new();
  loader.read_block(16 * 1024);
  let media = quiet_with(&loader, &Source::url(&pages.address).header("Referer", PAGES));
  until(&media, "opened", |state| !state.waiting && state.error.is_none());
  media.play();
  // Its first samples need blocks from the server one after another, with nothing else to show: it may wait.
  until(&media, "playing past its start", |state| {
    !state.waiting && state.position > 0.5
  });
  let started = Instant::now();
  let mut waited = Vec::new();
  while started.elapsed() < Duration::from_secs(3) {
    let state = media.state();
    if state.waiting {
      waited.push(state.position);
    }
    std::thread::sleep(Duration::from_millis(2));
  }
  assert!(
    waited.is_empty(),
    "it plays on while it reads ahead; it waited at {waited:?}"
  );
  assert!(media.state().position > 3.0, "{}", media.state().position);
}

/// Plays `media` from near its end, and gives how its state went until it has played on from its start: whether it
/// ever said it ended or stopped playing.
fn played_over_its_end(media: &Media, near_end: f64) -> (bool, bool) {
  media.seek(near_end);
  media.play();
  until(media, "playing near its end", |state| {
    state.playing && state.position >= near_end
  });
  let (mut ended, mut stopped) = (false, false);
  let started = Instant::now();
  loop {
    let state = media.state();
    ended |= state.ended;
    stopped |= !state.playing;
    if state.position < near_end - 1.0 && state.position > 0.2 {
      return (ended, stopped);
    }
    assert!(
      started.elapsed() < Duration::from_secs(10),
      "it never played on from its start. It is {state:?}"
    );
    std::thread::sleep(Duration::from_millis(2));
  }
}

pub fn a_looped_clip_goes_on_from_its_start_at_its_end_and_does_not_stop() {
  let media = quiet(&Source::file(clip()));
  until(&media, "opened", |state| !state.waiting);
  media.set_looped(true);
  let image = media.frame().expect("its picture");
  let (ended, stopped) = played_over_its_end(&media, 14.3);
  assert!(!ended, "its state never says it ended");
  assert!(!stopped, "nor that it stopped playing");
  // It plays on: frames of its start are shown.
  let before = image.version();
  std::thread::sleep(Duration::from_millis(300));
  assert!(image.version() > before, "frames come after the seam");

  // Told not to any more, it ends at its end as any other.
  media.set_looped(false);
  media.seek(14.6);
  let state = until(&media, "ended", |state| state.ended);
  assert!(!state.playing);
}

/// On Windows, where the app's loader reads what a media plays.
pub fn a_looped_clip_at_an_address_asks_the_server_for_nothing_again() {
  let pages = serve_to_its_pages(std::fs::read(clip()).expect("the clip"));
  let media = quiet(&Source::url(&pages.address).header("Referer", PAGES));
  until(&media, "opened", |state| !state.waiting && state.error.is_none());
  media.set_looped(true);
  let (ended, _) = played_over_its_end(&media, 14.3);
  let asked = pages.served.load(Ordering::SeqCst);
  assert!(!ended);
  let (ended, _) = played_over_its_end(&media, 14.3);
  assert!(!ended);
  assert_eq!(
    pages.served.load(Ordering::SeqCst),
    asked,
    "a second time around reads what is held"
  );
}
