//! A media decoding a real clip with the system's decoder: it opens, plays by its own clock, goes where it is
//! sent and ends. The clip is the gallery's, 15 seconds of 640 by 360 at 30 frames a second, with a tone.
//!
//! And a sound: a file, bytes and an address, which is loaded once and kept. Every media here is silenced
//! before it plays, so that a run of the tests is not heard; the one test that listens is ignored unless asked
//! for (`-- --ignored`), and plays a quiet tone for a moment.
//!
//! The tests of a sound run on macOS as well, where a media plays one with the system's own player; the tests
//! of a video, and the one that reads the system's meters, are Windows'.
#![cfg(any(windows, target_os = "macos"))]
#![cfg_attr(not(windows), allow(unused_imports, dead_code))]

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

/// The gallery's own track: 24 seconds of AAC in an MP4 file.
fn track() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../gallery/assets/media/track.m4a")
}

/// The clip, opened and silenced.
fn quiet(source: &Source) -> Media {
  let media = Media::open(&Loader::new(), source);
  media.set_muted(true);
  media
}

/// A tone of 440 Hz, `seconds` long, as the bytes of a WAV file: two channels of 16 bits at 44100 a second.
fn tone(seconds: f64) -> Vec<u8> {
  const RATE: u32 = 44_100;
  let frames = (seconds * f64::from(RATE)) as u32;
  let data = frames * 4;
  let mut wav = Vec::with_capacity(44 + data as usize);
  wav.extend_from_slice(b"RIFF");
  wav.extend_from_slice(&(36 + data).to_le_bytes());
  wav.extend_from_slice(b"WAVEfmt ");
  wav.extend_from_slice(&16_u32.to_le_bytes());
  wav.extend_from_slice(&1_u16.to_le_bytes());
  wav.extend_from_slice(&2_u16.to_le_bytes());
  wav.extend_from_slice(&RATE.to_le_bytes());
  wav.extend_from_slice(&(RATE * 4).to_le_bytes());
  wav.extend_from_slice(&4_u16.to_le_bytes());
  wav.extend_from_slice(&16_u16.to_le_bytes());
  wav.extend_from_slice(b"data");
  wav.extend_from_slice(&data.to_le_bytes());
  for frame in 0..frames {
    let at = f64::from(frame) / f64::from(RATE);
    let sample = ((at * 440.0 * std::f64::consts::TAU).sin() * 8_000.0) as i16;
    wav.extend_from_slice(&sample.to_le_bytes());
    wav.extend_from_slice(&sample.to_le_bytes());
  }
  wav
}

/// A file of this test's own, with `bytes` in it.
fn file(name: &str, bytes: &[u8]) -> PathBuf {
  let folder = std::env::temp_dir().join(format!("lurq-media-{}", std::process::id()));
  std::fs::create_dir_all(&folder).unwrap();
  let path = folder.join(name);
  std::fs::write(&path, bytes).unwrap();
  path
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

#[cfg(windows)]
#[test]
fn a_file_opens_with_its_size_its_length_and_its_first_frame() {
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
  // The first frame comes without playing: the picture is not the black it starts as.
  let black = image.version();
  let started = Instant::now();
  while image.version() == black {
    assert!(started.elapsed() < Duration::from_secs(10), "no first frame");
    std::thread::sleep(Duration::from_millis(10));
  }
}

#[cfg(windows)]
#[test]
fn it_plays_by_its_clock_shows_frames_and_stops_where_it_is_paused() {
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

#[cfg(windows)]
#[test]
fn it_goes_where_it_is_sent_and_ends_at_its_end() {
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

#[cfg(windows)]
#[test]
fn bytes_play_as_a_file_does_and_what_is_no_video_says_why() {
  let bytes = std::fs::read(clip()).expect("the clip");
  let media = quiet(&Source::bytes(bytes));
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

#[cfg(windows)]
#[test]
fn a_clip_has_its_sound_and_what_is_only_heard_of_it_has_no_picture() {
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

#[test]
fn a_sound_plays_from_a_file_and_from_bytes_to_its_end_and_again() {
  let wav = tone(1.2);
  let path = file("tone.wav", &wav);
  for (from, source) in [("a file", Source::file(&path)), ("bytes", Source::bytes(wav))] {
    let media = Media::sound(&Loader::new(), &source);
    media.set_muted(true);
    media.set_volume(0.5);
    let state = until(&media, "opened", |state| !state.waiting);
    assert_eq!((state.error.clone(), state.size, state.sound), (None, None, true), "{from}");
    assert!(
      state.duration.is_some_and(|duration| (duration - 1.2).abs() < 0.05),
      "{from}: {:?}",
      state.duration
    );
    let started = Instant::now();
    media.play();
    let ended = until(&media, "ended", |state| state.ended);
    let took = started.elapsed().as_secs_f64();
    assert!((1.0..2.0).contains(&took), "{from}: 1.2 s of sound took {took} s");
    assert!(!ended.playing);
    assert!(ended.duration.is_some_and(|duration| ended.position == duration));
    // Played again, it starts from its start.
    media.play();
    let again = until(&media, "playing again", |state| state.playing && !state.ended);
    assert!(again.position < 0.6, "{from}: {}", again.position);
  }
  let broken = Media::sound(&Loader::new(), &Source::bytes(b"this is not a sound".to_vec()));
  let state = until(&broken, "failed", |state| state.error.is_some());
  assert!(!state.playing && !state.waiting);
}

#[test]
fn a_song_in_an_mp4_file_plays_as_a_sound_at_any_speed_and_where_it_is_sent() {
  let media = Media::sound(&Loader::new(), &Source::file(track()));
  media.set_muted(true);
  let state = until(&media, "opened", |state| !state.waiting);
  assert_eq!((state.error, state.size, state.sound), (None, None, true));
  assert!(
    state.duration.is_some_and(|duration| (duration - 24.0).abs() < 0.2),
    "{:?}",
    state.duration
  );
  media.play();
  until(&media, "playing", |state| state.playing);
  std::thread::sleep(Duration::from_millis(600));
  let position = media.state().position;
  assert!((0.4..0.9).contains(&position), "about 0.6 s after 0.6 s: {position}");
  media.set_rate(2.0);
  until(&media, "twice as fast", |state| state.rate == 2.0);
  let from = media.state().position;
  std::thread::sleep(Duration::from_millis(500));
  let gone = media.state().position - from;
  assert!((0.8..1.3).contains(&gone), "about a second in half of one: {gone}");
  media.seek(10.0);
  std::thread::sleep(Duration::from_millis(300));
  let position = media.state().position;
  assert!(
    (10.3..11.2).contains(&position),
    "0.6 s on from 10 at twice the speed: {position}"
  );
  media.pause();
  let paused = until(&media, "paused", |state| !state.playing).position;
  std::thread::sleep(Duration::from_millis(200));
  assert_eq!(media.state().position, paused, "the clock stands");
  // Near its end it ends, and says so.
  media.set_rate(1.0);
  media.seek(23.6);
  media.play();
  let ended = until(&media, "ended", |state| state.ended);
  assert!(!ended.playing);
  assert!(ended.duration.is_some_and(|duration| ended.position == duration));
}

/// A server on this machine that answers each request with `body`, or with `status` where it has none, and
/// counts the requests it answered.
#[cfg(feature = "http")]
fn serve(body: Option<Vec<u8>>, status: &'static str) -> (String, Arc<AtomicUsize>) {
  use std::io::{Read, Write};
  let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
  let address = format!("http://{}/tone.wav", listener.local_addr().unwrap());
  let asked = Arc::new(AtomicUsize::new(0));
  let counted = asked.clone();
  std::thread::spawn(move || {
    for stream in listener.incoming() {
      let Ok(mut stream) = stream else {
        return;
      };
      let (mut request, mut chunk) = (Vec::new(), [0_u8; 1024]);
      while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        match stream.read(&mut chunk) {
          Ok(read) if read > 0 => request.extend_from_slice(&chunk[..read]),
          _ => break,
        }
      }
      counted.fetch_add(1, Ordering::Relaxed);
      let body = body.as_deref().unwrap_or_default();
      let head = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
      );
      let _ = stream.write_all(head.as_bytes());
      let _ = stream.write_all(body);
    }
  });
  (address, asked)
}

#[cfg(feature = "http")]
#[test]
fn a_sound_at_an_address_is_loaded_once_and_kept() {
  use lurq::media::sound_cache;
  let (address, asked) = serve(Some(tone(0.6)), "200 OK");
  let (loader, source) = (Loader::new(), Source::url(&address));
  assert!(!sound_cache().holds(&source));
  let media = Media::sound(&loader, &source);
  media.set_muted(true);
  assert!(media.state().waiting, "it waits while it loads");
  let state = until(&media, "opened", |state| !state.waiting);
  assert_eq!((state.error, state.sound), (None, true));
  assert!(state.duration.is_some_and(|duration| (duration - 0.6).abs() < 0.05));
  assert!(sound_cache().holds(&source), "what was loaded is kept");
  media.play();
  until(&media, "ended", |state| state.ended);
  drop(media);

  // Asked for again, by another media and after the loader let go of its bytes: nothing is asked of the server.
  loader.sweep();
  let again = Media::sound(&loader, &source);
  again.set_muted(true);
  let state = until(&again, "opened", |state| !state.waiting);
  assert_eq!((state.error, state.sound), (None, true));
  assert_eq!(asked.load(Ordering::Relaxed), 1, "one request for two plays");

  // Forgotten, it is loaded again.
  assert!(sound_cache().forget(&source));
  let third = Media::sound(&loader, &source);
  third.set_muted(true);
  until(&third, "opened", |state| !state.waiting && state.sound);
  assert_eq!(asked.load(Ordering::Relaxed), 2);
}

#[cfg(feature = "http")]
#[test]
fn a_sound_that_is_not_at_its_address_says_so_and_one_the_app_refuses_is_not_asked_for() {
  let (address, asked) = serve(None, "404 Not Found");
  let missing = Media::sound(&Loader::new(), &Source::url(&address));
  let state = until(&missing, "failed", |state| state.error.is_some());
  assert_eq!(state.error.as_deref(), Some("it was not found"));
  assert!(!state.waiting && !state.playing);
  assert!(!lurq::media::sound_cache().holds(&Source::url(&address)));

  let loader = Loader::new();
  loader.allow(|_| false);
  let before = asked.load(Ordering::Relaxed);
  let refused = Media::sound(&loader, &Source::url(&address));
  assert_eq!(
    refused.state().error.as_deref(),
    Some("the app does not allow this address")
  );
  std::thread::sleep(Duration::from_millis(100));
  assert_eq!(asked.load(Ordering::Relaxed), before);
}

/// How loud it is right now, from 0 to 1, by the system's own meters: what this process gives the default
/// output, before the loudness it is played at, and what comes out of the output, of everything that plays.
#[cfg(windows)]
fn loudness() -> (f32, f32) {
  use windows::{
    Win32::{
      Media::Audio::{
        Endpoints::IAudioMeterInformation, IAudioSessionControl2, IAudioSessionManager2, IMMDeviceEnumerator,
        MMDeviceEnumerator, eMultimedia, eRender,
      },
      System::Com::{CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx},
    },
    core::Interface,
  };
  // SAFETY: COM is started on this thread, and every interface is used on it.
  unsafe {
    let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    let devices: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).expect("the devices");
    let device = devices
      .GetDefaultAudioEndpoint(eRender, eMultimedia)
      .expect("an output to listen on");
    let out: IAudioMeterInformation = device.Activate(CLSCTX_ALL, None).expect("the output's meter");
    let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None).expect("its sessions");
    let sessions = manager.GetSessionEnumerator().expect("its sessions");
    let mut ours = 0.0_f32;
    for index in 0..sessions.GetCount().unwrap_or(0) {
      let Ok(session) = sessions.GetSession(index) else {
        continue;
      };
      let this = session
        .cast::<IAudioSessionControl2>()
        .and_then(|session| session.GetProcessId())
        .is_ok_and(|process| process == std::process::id());
      if this && let Ok(meter) = session.cast::<IAudioMeterInformation>() {
        ours = ours.max(meter.GetPeakValue().unwrap_or(0.0));
      }
    }
    (ours, out.GetPeakValue().unwrap_or(0.0))
  }
}

/// The loudest it is over the next `milliseconds`: of this process, and out of the device.
#[cfg(windows)]
fn loudest_over(milliseconds: u64) -> (f32, f32) {
  let started = Instant::now();
  let mut loudest = (0.0_f32, 0.0_f32);
  while started.elapsed() < Duration::from_millis(milliseconds) {
    let now = loudness();
    loudest = (loudest.0.max(now.0), loudest.1.max(now.1));
    std::thread::sleep(Duration::from_millis(20));
  }
  loudest
}

/// This one is heard: a quiet tone for about two seconds. It asks the system how loud this process is and how
/// loud the device is while the media plays, so that what the other tests take on trust (that the samples
/// reach the output, as loud as told) is measured. Nothing else may play while it runs: the device's meter is
/// of everything. Run alone:
/// `cargo test -p lurq --features media --test media_tests -- --ignored --test-threads 1 --nocapture`.
#[cfg(windows)]
#[test]
#[ignore = "it is heard: a quiet tone for two seconds"]
fn its_sound_reaches_the_output_as_loud_as_it_is_told() {
  let media = Media::sound(&Loader::new(), &Source::bytes(tone(4.0)));
  media.set_volume(0.1);
  until(&media, "opened", |state| !state.waiting && state.sound);
  let before = loudest_over(200);
  media.play();
  until(&media, "playing", |state| state.playing);
  std::thread::sleep(Duration::from_millis(300));
  let quiet = loudest_over(250);
  media.set_volume(0.2);
  std::thread::sleep(Duration::from_millis(150));
  let louder = loudest_over(250);
  media.set_muted(true);
  std::thread::sleep(Duration::from_millis(150));
  let muted = loudest_over(200);
  media.set_muted(false);
  std::thread::sleep(Duration::from_millis(150));
  let again = loudest_over(200);
  media.pause();
  std::thread::sleep(Duration::from_millis(250));
  let paused = loudest_over(200);
  println!("(process, device) before {before:?} at 0.1 {quiet:?} at 0.2 {louder:?} muted {muted:?}");
  println!("(process, device) heard again {again:?} paused {paused:?}");

  assert!(before.0 < 0.001, "nothing is given to the output before it plays: {before:?}");
  assert!(quiet.0 > 0.1, "the tone is given to the output: {quiet:?}");
  assert!(paused.0 < 0.001, "paused, nothing is given to it: {paused:?}");
  // The rest is of the device, and holds only where nothing else plays.
  assert!(before.1 < 0.001, "something else plays, which this test cannot tell from the tone: {before:?}");
  assert!(quiet.1 > 0.005, "it comes out of the device: {quiet:?}");
  assert!(
    (quiet.1 * 1.6..quiet.1 * 2.4).contains(&louder.1),
    "twice as loud at twice the volume: {louder:?} after {quiet:?}"
  );
  assert!(muted.1 < 0.001, "silenced, nothing comes out: {muted:?}");
  assert!(
    (louder.1 * 0.8..louder.1 * 1.2).contains(&again.1),
    "and it comes back as loud as it was: {again:?} after {louder:?}"
  );
  assert!(paused.1 < 0.001, "paused, nothing comes out: {paused:?}");
}
