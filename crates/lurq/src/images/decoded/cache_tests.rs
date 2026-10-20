//! The cache stays consistent when a decoder panics and when `insert` races
//! a running decode.

use std::{
  sync::{Arc, Mutex, mpsc},
  time::Duration,
};

use super::*;
use crate::app::window::WindowWaker;

const WAIT: Duration = Duration::from_secs(10);

fn pixels(width: u32, height: u32) -> ImageData {
  ImageData::from_rgba(vec![255; (width * height * 4) as usize], width, height)
}

#[test]
fn a_panicking_decoder_fails_its_entry_and_the_worker_goes_on() {
  let cache = DecodedImages::new();
  let key = ImageKey::new(1);

  let state = cache.load_with(key, None, None, || {
    || -> Result<ImageData, ImageDecodeError> {
      panic!("a decoder bug");
    }
  });
  assert!(state.is_loading());
  assert!(cache.wait_idle(WAIT), "the entry is finished although the job panicked");
  assert_eq!(cache.pending(), 0);
  let error = cache
    .state(key, None)
    .expect("entry kept")
    .error()
    .expect("failed")
    .to_string();
  assert!(error.contains("panicked"), "{error}");

  // The worker that caught the panic still decodes.
  let next = ImageKey::new(2);
  cache.load_with(next, None, None, || || Ok(pixels(2, 2)));
  assert!(cache.wait_idle(WAIT));
  assert!(cache.state(next, None).is_some_and(|state| state.ready().is_some()));
  assert_eq!(cache.completed(), 2);
}

#[test]
fn an_insert_while_the_decode_runs_wins_and_the_late_result_is_dropped() {
  let cache = DecodedImages::new();
  let key = ImageKey::new(1);
  let (release, wait_for_release) = mpsc::channel::<()>();
  let wake_calls = Arc::new(Mutex::new(0));
  let wake: WindowWaker = {
    let calls = wake_calls.clone();
    Arc::new(move || *calls.lock().unwrap() += 1)
  };

  cache.load_with(key, None, Some(wake), move || {
    move || {
      wait_for_release.recv().ok();
      Ok(pixels(4, 4))
    }
  });
  assert_eq!(cache.pending(), 1);
  let inserted = pixels(2, 2);
  let inserted_id = inserted.id();
  cache.insert(key, None, inserted);
  assert_eq!(
    *wake_calls.lock().unwrap(),
    1,
    "the window waiting for the decode is woken by the insert"
  );
  assert_eq!(cache.pending(), 1, "the decode still runs");
  assert_eq!(cache.used_bytes(), 2 * 2 * 4);

  release.send(()).unwrap();
  assert!(cache.wait_idle(WAIT));

  assert_eq!(cache.pending(), 0);
  let ready = cache
    .state(key, None)
    .and_then(|state| state.ready().map(|data| data.id()));
  assert_eq!(ready, Some(inserted_id), "the inserted pixels stay");
  assert_eq!(cache.used_bytes(), 2 * 2 * 4, "the late result is not counted");
  assert_eq!(*wake_calls.lock().unwrap(), 1, "no second wake for the dropped result");
}

#[test]
fn an_insert_over_a_ready_entry_replaces_its_bytes() {
  let cache = DecodedImages::new();
  let key = ImageKey::new(1);
  cache.insert(key, None, pixels(4, 4));
  assert_eq!(cache.used_bytes(), 64);

  cache.insert(key, None, pixels(2, 2));

  assert_eq!(cache.used_bytes(), 16, "the old bytes are subtracted");
}

#[test]
fn one_waker_per_window_is_kept_for_a_loading_entry() {
  let cache = DecodedImages::new();
  let key = ImageKey::new(1);
  let (release, wait_for_release) = mpsc::channel::<()>();
  let wake_calls = Arc::new(Mutex::new(0));
  let wake: WindowWaker = {
    let calls = wake_calls.clone();
    Arc::new(move || *calls.lock().unwrap() += 1)
  };

  cache.load_with(key, None, Some(wake.clone()), move || {
    move || {
      wait_for_release.recv().ok();
      Ok(pixels(1, 1))
    }
  });
  // Every pass asks again with the same waker.
  for _ in 0..3 {
    assert!(
      cache
        .load(key, None, || unreachable!("loading"), Some(wake.clone()))
        .is_loading()
    );
  }
  release.send(()).unwrap();
  assert!(cache.wait_idle(WAIT));

  assert_eq!(*wake_calls.lock().unwrap(), 1, "woken once, not once per pass");
}
