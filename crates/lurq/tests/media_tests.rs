//! A media decoding a real clip with the system's decoder: it opens, plays by its own clock, goes where it is
//! sent and ends. The clip is the gallery's, 15 seconds of 640 by 360 at 30 frames a second, with a tone.
//!
//! And a sound: a file, bytes and an address, which is loaded once and kept. Every media here is silenced
//! before it plays, so that a run of the tests is not heard; the one test that listens is ignored unless asked
//! for (`-- --ignored`), and plays a quiet tone for a moment.
//!
//! The tests of a sound run on macOS as well, where a media plays one with the system's own player. The tests of
//! a video (`media/video.rs`) are run from here on Windows; on macOS the system's player of a video is the main
//! thread's, which the test harness keeps waiting, so `media_video_apple.rs` runs them. The one that reads the
//! system's meters is Windows'.
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
  media::{Media, MediaError, MediaState},
  resources::{Loader, Source},
};

#[path = "media/video.rs"]
mod video;
use video::{clip, quiet, until};

/// The gallery's own track: 24 seconds of AAC in an MP4 file.
fn track() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../gallery/assets/media/track.m4a")
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

#[cfg(windows)]
#[test]
fn a_file_opens_with_its_size_its_length_and_its_first_frame() {
  video::a_file_opens_with_its_size_its_length_and_its_first_frame();
}

#[cfg(windows)]
#[test]
fn it_plays_by_its_clock_shows_frames_and_stops_where_it_is_paused() {
  video::it_plays_by_its_clock_shows_frames_and_stops_where_it_is_paused();
}

#[cfg(windows)]
#[test]
fn it_goes_where_it_is_sent_and_ends_at_its_end() {
  video::it_goes_where_it_is_sent_and_ends_at_its_end();
}

#[cfg(windows)]
#[test]
fn it_goes_near_where_it_is_sent_and_plays_on_from_the_frame_it_shows() {
  video::it_goes_near_where_it_is_sent_and_plays_on_from_the_frame_it_shows();
}

#[cfg(windows)]
#[test]
fn bytes_play_as_a_file_does_and_what_is_no_video_says_why() {
  video::bytes_play_as_a_file_does_and_what_is_no_video_says_why();
}

#[cfg(windows)]
#[test]
fn a_clip_has_its_sound_and_what_is_only_heard_of_it_has_no_picture() {
  video::a_clip_has_its_sound_and_what_is_only_heard_of_it_has_no_picture();
}

#[cfg(all(windows, feature = "http"))]
#[test]
fn a_clip_at_an_address_is_asked_for_with_the_headers_of_its_source_and_goes_where_it_is_sent() {
  video::a_clip_at_an_address_is_asked_for_with_the_headers_of_its_source_and_goes_where_it_is_sent();
}

#[cfg(all(windows, feature = "http"))]
#[test]
fn the_loaders_rule_gives_a_clip_its_headers_and_a_sources_own_take_their_place() {
  video::the_loaders_rule_gives_a_clip_its_headers_and_a_sources_own_take_their_place();
}

#[cfg(all(windows, feature = "http"))]
#[test]
fn a_clip_waits_for_what_the_network_has_not_given_yet_and_fails_with_what_the_server_answered_on_the_way() {
  video::a_clip_waits_for_what_the_network_has_not_given_yet_and_fails_with_what_the_server_answered_on_the_way();
}

#[cfg(all(windows, feature = "http"))]
#[test]
fn a_clip_whose_reads_ahead_come_in_time_does_not_wait_while_it_plays() {
  video::a_clip_whose_reads_ahead_come_in_time_does_not_wait_while_it_plays();
}

#[cfg(windows)]
#[test]
fn a_looped_clip_goes_on_from_its_start_at_its_end_and_does_not_stop() {
  video::a_looped_clip_goes_on_from_its_start_at_its_end_and_does_not_stop();
}

#[cfg(all(windows, feature = "http"))]
#[test]
fn a_media_says_how_far_it_is_fetched_and_not_its_whole_length() {
  video::a_media_says_how_far_it_is_fetched_and_not_its_whole_length();
}

#[cfg(all(windows, feature = "http"))]
#[test]
fn a_looped_clip_at_an_address_asks_the_server_for_nothing_again() {
  video::a_looped_clip_at_an_address_asks_the_server_for_nothing_again();
}

#[cfg(all(windows, feature = "http"))]
#[test]
fn a_clip_opened_ahead_and_left_paused_reads_its_start_and_no_more() {
  video::a_clip_opened_ahead_and_left_paused_reads_its_start_and_no_more();
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
    assert_eq!(
      (state.error.clone(), state.size, state.sound),
      (None, None, true),
      "{from}"
    );
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
  assert_eq!(state.error, Some(MediaError::Unsupported));
  // The app's locale says it: a short line a player shows.
  let said = |tag: &str| lurq::core::Locale::for_tag(tag).media.error(&MediaError::Unsupported);
  assert_eq!(
    (&*said("en"), &*said("uk")),
    ("Unsupported format", "Непідтримуваний формат")
  );
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

#[cfg(all(windows, feature = "http"))]
#[test]
fn a_sound_larger_than_the_loader_reads_whole_is_read_as_it_plays_with_the_headers_of_its_source() {
  let pages = video::serve_to_its_pages(std::fs::read(track()).expect("the track"));
  let loader = Loader::new();
  loader.max_bytes(1000);
  let source = Source::url(&pages.address).header("Referer", video::PAGES);
  let song = Media::sound(&loader, &source);
  song.set_muted(true);
  let state = until(&song, "opened", |state| !state.waiting);
  assert_eq!((state.error, state.sound), (None, true));
  assert!(
    state.duration.is_some_and(|duration| (duration - 24.0).abs() < 0.5),
    "{:?}",
    state.duration
  );
  song.seek(20.0);
  song.play();
  until(&song, "playing from 20", |state| {
    state.playing && state.position >= 20.0
  });
  assert_eq!(pages.refused.load(Ordering::SeqCst), 0, "every request had the header");
  assert!(
    !lurq::media::sound_cache().holds(&source),
    "what is read as it plays is not kept"
  );
}

#[cfg(feature = "http")]
#[test]
fn a_sound_that_is_not_at_its_address_says_so_and_one_the_app_refuses_is_not_asked_for() {
  let (address, asked) = serve(None, "404 Not Found");
  let missing = Media::sound(&Loader::new(), &Source::url(&address));
  let state = until(&missing, "failed", |state| state.error.is_some());
  assert_eq!(state.error, Some(MediaError::NotFound));
  assert!(!state.waiting && !state.playing);
  assert!(!lurq::media::sound_cache().holds(&Source::url(&address)));

  let loader = Loader::new();
  loader.allow(|_| false);
  let before = asked.load(Ordering::Relaxed);
  let refused = Media::sound(&loader, &Source::url(&address));
  assert_eq!(refused.state().error, Some(MediaError::Denied));
  std::thread::sleep(Duration::from_millis(100));
  assert_eq!(asked.load(Ordering::Relaxed), before);
}

/// An empty folder of this test's own, for a cache on the disk.
#[cfg(feature = "http")]
fn cache_folder(test: &str) -> PathBuf {
  let folder = std::env::temp_dir().join(format!("lurq-media-cache-{}-{test}", std::process::id()));
  let _ = std::fs::remove_dir_all(&folder);
  folder
}

/// A loader with its cache on the disk turned on in `folder`: what a run of an app that gave the folder has.
#[cfg(feature = "http")]
fn run_with(folder: &std::path::Path) -> Loader {
  let loader = Loader::new();
  loader.enable_media_disk_cache(folder);
  loader
}

/// The names of the files in `folder`, in order.
#[cfg(feature = "http")]
fn kept_in(folder: &std::path::Path) -> Vec<String> {
  let mut names: Vec<String> = std::fs::read_dir(folder)
    .unwrap()
    .map(|file| file.unwrap().file_name().into_string().unwrap())
    .collect();
  names.sort();
  names
}

/// The sound of `source` by `loader`, silenced and opened, with nothing wrong.
#[cfg(feature = "http")]
#[track_caller]
fn opened(loader: &Loader, source: &Source) -> Media {
  let media = Media::sound(loader, source);
  media.set_muted(true);
  let state = until(&media, "opened", |state| !state.waiting);
  assert_eq!((state.error, state.sound), (None, true));
  media
}

/// Does `change` to a file a media played a moment ago, once the system has let go of it.
#[cfg(feature = "http")]
#[track_caller]
fn once_let_go(mut change: impl FnMut() -> std::io::Result<()>) {
  let started = Instant::now();
  while let Err(error) = change() {
    assert!(
      started.elapsed() < Duration::from_secs(10),
      "the file is still held: {error}"
    );
    std::thread::sleep(Duration::from_millis(20));
  }
}

#[cfg(feature = "http")]
#[test]
fn with_a_folder_given_a_sound_at_an_address_is_a_file_there_and_the_next_run_asks_the_server_for_nothing() {
  use lurq::{media::sound_cache, resources::LoadState};
  let folder = cache_folder("next-run");
  let body = tone(0.6);
  let (address, asked) = serve(Some(body.clone()), "200 OK");
  let source = Source::url(&address);

  let loader = run_with(&folder);
  let disk = loader.media_disk_cache();
  assert!(disk.is_on() && disk.is_empty() && !sound_cache().holds(&source));
  let media = opened(&loader, &source);
  let state = media.state();
  assert!(state.duration.is_some_and(|duration| (duration - 0.6).abs() < 0.05));
  // One file, whole, named by the source and its length, with the ending of its address.
  let names = kept_in(&folder);
  assert_eq!(names.len(), 1, "{names:?}");
  assert!(names[0].ends_with(&format!("-{:x}.wav", body.len())), "{names:?}");
  assert_eq!(std::fs::read(folder.join(&names[0])).unwrap(), body);
  assert!(disk.holds(&source) && sound_cache().holds(&source));
  assert_eq!((disk.len(), disk.bytes()), (1, body.len() as u64));
  media.play();
  until(&media, "ended", |state| state.ended);
  // Played again by the same run: from the file.
  let again = opened(&loader, &source);
  assert_eq!(asked.load(Ordering::Relaxed), 1, "one request for two plays");
  drop((media, again, loader));

  // The next run of the app: a new loader, given the same folder. It knows what is kept before anything is
  // asked for, plays it, and gives its bytes to what reads them whole, with no request to the server.
  let next = run_with(&folder);
  assert!(next.media_disk_cache().holds(&source));
  let media = opened(&next, &source);
  assert!(
    media
      .state()
      .duration
      .is_some_and(|duration| (duration - 0.6).abs() < 0.05)
  );
  media.play();
  until(&media, "ended", |state| state.ended);
  let load = next.load(&source);
  let started = Instant::now();
  let bytes = loop {
    match load.state() {
      LoadState::Ready(bytes) => break bytes,
      LoadState::Failed(error) => panic!("the load failed: {error}"),
      LoadState::Loading { .. } => assert!(started.elapsed() < Duration::from_secs(10), "the load does not end"),
    }
    std::thread::sleep(Duration::from_millis(5));
  };
  assert_eq!(bytes[..], body[..]);
  assert_eq!(
    asked.load(Ordering::Relaxed),
    1,
    "the next run asks the server for nothing"
  );

  // An address with no ending: its file has none, and plays as well.
  let unnamed = Source::url(&address.replace("/tone.wav", "/track/12"));
  let other = opened(&next, &unnamed);
  assert!(
    other
      .state()
      .duration
      .is_some_and(|duration| (duration - 0.6).abs() < 0.05)
  );
  assert_eq!(asked.load(Ordering::Relaxed), 2);
  assert!(
    kept_in(&folder).iter().any(|name| !name.contains('.')),
    "{:?}",
    kept_in(&folder)
  );
  drop((media, other, next));
  let _ = std::fs::remove_dir_all(&folder);
}

#[cfg(feature = "http")]
#[test]
fn forgetting_a_sound_deletes_its_file_and_it_is_loaded_again() {
  use lurq::media::sound_cache;
  let folder = cache_folder("forget");
  let (address, asked) = serve(Some(tone(2.0)), "200 OK");
  let source = Source::url(&address);
  let loader = run_with(&folder);
  let disk = loader.media_disk_cache();

  // Forgotten while it plays: the file is gone at once, and what plays goes on.
  let media = opened(&loader, &source);
  media.play();
  until(&media, "playing", |state| state.playing && state.position > 0.1);
  assert!(sound_cache().forget(&source));
  assert!(kept_in(&folder).is_empty() && !disk.holds(&source) && !sound_cache().holds(&source));
  assert!(!sound_cache().forget(&source));
  std::thread::sleep(Duration::from_millis(200));
  let state = media.state();
  assert!(state.playing && state.error.is_none(), "{state:?}");

  // Asked for again, it is loaded from its address again, and kept again. The loader is told to forget too,
  // as a player's "Try again" tells it: the bytes of a load that ended a moment ago are not given a second time.
  loader.forget(&source);
  let second = opened(&loader, &source);
  assert_eq!(asked.load(Ordering::Relaxed), 2);
  assert_eq!(kept_in(&folder).len(), 1);

  // The loader's own forgetting deletes the file as well: the way to load anew what is not a sound.
  drop((media, second));
  assert!(loader.forget(&source));
  assert!(kept_in(&folder).is_empty() && !disk.holds(&source));
  let third = opened(&loader, &source);
  assert_eq!(asked.load(Ordering::Relaxed), 3);
  drop(third);

  // And clearing deletes every file.
  assert_eq!(disk.len(), 1);
  disk.clear();
  assert!(kept_in(&folder).is_empty() && disk.is_empty());
  drop(loader);
  let _ = std::fs::remove_dir_all(&folder);
}

#[cfg(feature = "http")]
#[test]
fn over_the_budget_of_the_folder_the_sound_played_longest_ago_is_deleted() {
  let folder = cache_folder("budget");
  let body = tone(0.3);
  let served: Vec<_> = (0..3).map(|_| serve(Some(body.clone()), "200 OK")).collect();
  let sources: Vec<Source> = served.iter().map(|(address, _)| Source::url(address)).collect();
  let loader = run_with(&folder);
  let disk = loader.media_disk_cache();
  // Room for two of them, and not for three.
  disk.budget(body.len() as u64 * 5 / 2);

  drop(opened(&loader, &sources[0]));
  drop(opened(&loader, &sources[1]));
  // The first is played again, so the second is the one played longest ago.
  drop(opened(&loader, &sources[0]));
  drop(opened(&loader, &sources[2]));
  assert!(disk.holds(&sources[0]) && !disk.holds(&sources[1]) && disk.holds(&sources[2]));
  assert_eq!((disk.len(), disk.bytes()), (2, body.len() as u64 * 2));
  assert_eq!(kept_in(&folder).len(), 2);
  for (_, asked) in &served {
    assert_eq!(asked.load(Ordering::Relaxed), 1);
  }

  // The next run knows which was played longest ago: the first was played before the third.
  drop(loader);
  let next = run_with(&folder);
  let disk = next.media_disk_cache();
  disk.budget(body.len() as u64 * 5 / 2);
  drop(opened(&next, &sources[1]));
  assert!(!disk.holds(&sources[0]) && disk.holds(&sources[1]) && disk.holds(&sources[2]));
  assert_eq!(
    served[1].1.load(Ordering::Relaxed),
    2,
    "what was deleted is loaded again"
  );

  // One that is larger than the whole budget is not kept in the folder: it is kept in memory, as with no folder.
  disk.budget(100);
  assert!(disk.is_empty() && kept_in(&folder).is_empty());
  drop(opened(&next, &sources[0]));
  assert!(disk.is_empty() && kept_in(&folder).is_empty());
  assert!(lurq::media::sound_cache().holds(&sources[0]));
  assert!(lurq::media::sound_cache().forget(&sources[0]));
  drop(next);
  let _ = std::fs::remove_dir_all(&folder);
}

#[cfg(feature = "http")]
#[test]
fn a_file_of_the_folder_that_is_gone_cut_short_or_spoiled_is_loaded_again_and_not_played_broken() {
  let folder = cache_folder("broken");
  let body = tone(0.6);
  let (address, asked) = serve(Some(body.clone()), "200 OK");
  let source = Source::url(&address);
  drop(opened(&run_with(&folder), &source));
  let file = folder.join(&kept_in(&folder)[0]);
  let whole = |media: &Media| {
    let state = media.state();
    assert!(
      state.duration.is_some_and(|duration| (duration - 0.6).abs() < 0.05),
      "{state:?}"
    );
    assert_eq!(std::fs::read(&file).unwrap(), body, "the file is whole again");
  };

  // Cut short while the app was not running: the next run finds its size is not the length in its name.
  once_let_go(|| std::fs::write(&file, &body[..body.len() / 2]));
  let next = run_with(&folder);
  assert!(!next.media_disk_cache().holds(&source));
  let media = opened(&next, &source);
  assert_eq!(asked.load(Ordering::Relaxed), 2);
  whole(&media);
  drop((media, next));

  // Cut short while the app runs: it is found when the sound is opened.
  let next = run_with(&folder);
  assert!(next.media_disk_cache().holds(&source));
  once_let_go(|| std::fs::write(&file, &body[..body.len() / 2]));
  let media = opened(&next, &source);
  assert_eq!(asked.load(Ordering::Relaxed), 3);
  whole(&media);
  drop((media, next));

  // Deleted by somebody else while the app runs.
  let next = run_with(&folder);
  once_let_go(|| std::fs::remove_file(&file));
  let media = opened(&next, &source);
  assert_eq!(asked.load(Ordering::Relaxed), 4);
  whole(&media);
  drop((media, next));

  // As long as it was written, and not what was written: the media says it cannot play it, the file is let
  // go, and the next try loads it anew.
  let next = run_with(&folder);
  once_let_go(|| std::fs::write(&file, vec![0_u8; body.len()]));
  let spoiled = Media::sound(&next, &source);
  spoiled.set_muted(true);
  let state = until(&spoiled, "failed", |state| state.error.is_some());
  assert!(!state.playing && !state.waiting);
  assert_eq!(asked.load(Ordering::Relaxed), 4, "it was there to open");
  let started = Instant::now();
  while next.media_disk_cache().holds(&source) {
    assert!(
      started.elapsed() < Duration::from_secs(10),
      "the spoiled file is still kept"
    );
    std::thread::sleep(Duration::from_millis(10));
  }
  assert!(kept_in(&folder).is_empty());
  let media = opened(&next, &source);
  assert_eq!(asked.load(Ordering::Relaxed), 5);
  whole(&media);
  drop((spoiled, media, next));
  let _ = std::fs::remove_dir_all(&folder);
}

#[cfg(feature = "http")]
#[test]
fn with_no_folder_given_nothing_is_written_and_a_sound_is_kept_in_memory() {
  use lurq::media::sound_cache;
  let (address, asked) = serve(Some(tone(0.3)), "200 OK");
  let (loader, source) = (Loader::new(), Source::url(&address));
  let disk = loader.media_disk_cache();
  assert!(!disk.is_on() && disk.folder().is_none());
  drop(opened(&loader, &source));
  assert!(sound_cache().holds(&source) && !disk.holds(&source) && disk.is_empty());
  drop(opened(&loader, &source));
  assert_eq!(asked.load(Ordering::Relaxed), 1);
  assert!(sound_cache().forget(&source));
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

  assert!(
    before.0 < 0.001,
    "nothing is given to the output before it plays: {before:?}"
  );
  assert!(quiet.0 > 0.1, "the tone is given to the output: {quiet:?}");
  assert!(paused.0 < 0.001, "paused, nothing is given to it: {paused:?}");
  // The rest is of the device, and holds only where nothing else plays.
  assert!(
    before.1 < 0.001,
    "something else plays, which this test cannot tell from the tone: {before:?}"
  );
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
