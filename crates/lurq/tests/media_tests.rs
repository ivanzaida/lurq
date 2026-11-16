//! A media decoding a real clip with the system's decoder: it opens, plays by its own clock, goes where it is
//! sent and ends. The clip is the gallery's, 15 seconds of 640 by 360 at 30 frames a second.
#![cfg(windows)]

use std::{
  path::PathBuf,
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
  time::{Duration, Instant},
};

use lurq::{
  media::{Media, MediaState},
  resources::{Loader, Source},
};

fn clip() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../gallery/assets/media/sample.mp4")
}

/// Waits until `is` holds of the media's state, for at most ten seconds.
#[track_caller]
fn until(media: &Media, what: &str, is: impl Fn(&MediaState) -> bool) -> MediaState {
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

#[test]
fn a_file_opens_with_its_size_its_length_and_its_first_frame() {
  let woken = Arc::new(AtomicUsize::new(0));
  let media = Media::open(&Loader::new(), &Source::file(clip()));
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
  // The first frame comes without playing: the picture is not the black it starts as.
  let black = image.version();
  let started = Instant::now();
  while image.version() == black {
    assert!(started.elapsed() < Duration::from_secs(10), "no first frame");
    std::thread::sleep(Duration::from_millis(10));
  }
}

#[test]
fn it_plays_by_its_clock_shows_frames_and_stops_where_it_is_paused() {
  let media = Media::open(&Loader::new(), &Source::file(clip()));
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

#[test]
fn it_goes_where_it_is_sent_and_ends_at_its_end() {
  let media = Media::open(&Loader::new(), &Source::file(clip()));
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

#[test]
fn bytes_play_as_a_file_does_and_what_is_no_video_says_why() {
  let bytes = std::fs::read(clip()).expect("the clip");
  let media = Media::open(&Loader::new(), &Source::bytes(bytes));
  let state = until(&media, "opened", |state| !state.waiting);
  assert_eq!((state.error, state.size), (None, Some((640, 360))));
  let broken = Media::open(&Loader::new(), &Source::bytes(b"this is not a video".to_vec()));
  let state = until(&broken, "failed", |state| state.error.is_some());
  assert!(!state.playing && !state.waiting);
  let missing = Media::open(&Loader::new(), &Source::file("no/such/clip.mp4"));
  until(&missing, "failed", |state| state.error.is_some());
  // An address the app does not allow is not asked for.
  let loader = Loader::new();
  loader.allow(|_| false);
  let refused = Media::open(&loader, &Source::url("https://example.com/clip.mp4"));
  assert_eq!(
    refused.state().error.as_deref(),
    Some("the app does not allow this address")
  );
}
