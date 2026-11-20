//! The events of a media and of the players of a source: it opened, it failed, it ended, it began or stopped
//! playing and waiting. Each is told on the tree's pass, once each time it happens, in the order a media lives
//! them. The medias here are driven by the tests (`lurq::media::drive_media`, `Media::driven`): no thread, no
//! decoder, nothing heard, no clock read.

use std::sync::{Arc, Mutex};

use lurq::{
  app::ctx::Ctx,
  components::Column,
  core::{MediaError, MediaInfo, Signal},
  media::{Media, drive_media},
  node::Element,
  resources::Source,
  ui::{AudioPlayer, VideoPlayer},
};

use super::stage::Stage;

/// What the handlers were told, in order.
#[derive(Clone, Default)]
struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
  fn say(&self) -> impl Fn(String) + Send + Sync + Clone + 'static {
    let log = self.0.clone();
    move |what| log.lock().unwrap().push(what)
  }

  fn take(&self) -> Vec<String> {
    std::mem::take(&mut *self.0.lock().unwrap())
  }
}

/// How an event is written in the log.
fn loaded(info: &MediaInfo) -> String {
  let size = info.size.map_or("no picture".to_owned(), |(w, h)| format!("{w}x{h}"));
  format!("load {:?} {size} sound {}", info.duration, info.sound)
}

const CLIP: MediaInfo = MediaInfo {
  duration: Some(15.0),
  size: Some((1280, 720)),
  sound: true,
};

const SONG: MediaInfo = MediaInfo {
  duration: Some(180.0),
  size: None,
  sound: true,
};

fn stage(scene: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::first_frame(800.0, 520.0, move |ctx| {
    Column::new().size(800.0, 520.0).child(scene(ctx)).into()
  });
  stage.pass();
  stage
}

#[test]
fn a_media_tells_each_event_once_on_the_trees_pass_in_the_order_it_lives_them() {
  let (media, driver) = Media::driven();
  let (log, here) = (Log::default(), std::thread::current().id());
  let say = log.say();
  media.on_load({
    let say = say.clone();
    move |info| {
      assert_eq!(std::thread::current().id(), here, "on the UI thread");
      say(loaded(info));
    }
  });
  media.on_error({
    let say = say.clone();
    move |error| say(format!("error {error}"))
  });
  media.on_ended({
    let say = say.clone();
    move || say("ended".to_owned())
  });
  media.on_playing_change({
    let say = say.clone();
    move |plays| say(format!("playing {plays}"))
  });
  media.on_waiting_change(move |waits| say(format!("waiting {waits}")));
  let mut stage = stage(|_| Column::new().into());
  assert!(log.take().is_empty(), "opening is not waiting, and nothing opened yet");

  // Told to play before it opened, it plays once it did: it opened first.
  media.play();
  driver.open(CLIP);
  assert!(log.take().is_empty(), "not before the tree's pass");
  stage.pass();
  assert_eq!(log.take(), ["load Some(15.0) 1280x720 sound true", "playing true"]);
  stage.pass();
  assert!(log.take().is_empty(), "each once");

  // It waits for what it plays, and goes on.
  driver.wait(true);
  stage.pass();
  assert_eq!(log.take(), ["waiting true"]);
  driver.wait(false);
  stage.pass();
  assert_eq!(log.take(), ["waiting false"]);

  // It ends: it stopped first. Once, until it played again.
  driver.end();
  stage.pass();
  assert_eq!(log.take(), ["playing false", "ended"]);
  stage.pass();
  assert!(log.take().is_empty());
  media.play();
  stage.pass();
  assert_eq!(log.take(), ["playing true"], "played again, from its start");
  driver.end();
  stage.pass();
  assert_eq!(log.take(), ["playing false", "ended"]);

  // Paused, it says so; a seek is no event.
  media.play();
  stage.pass();
  media.pause();
  media.seek(4.0);
  stage.pass();
  assert_eq!(log.take(), ["playing true", "playing false"]);

  // It fails: once, a kind in the locale's words.
  driver.fail(MediaError::Network(Arc::from(
    "No such host is known. (os error 11001)",
  )));
  stage.pass();
  assert_eq!(log.take(), ["error Network error"]);
  driver.fail(MediaError::Unsupported);
  stage.pass();
  assert!(log.take().is_empty(), "a media that failed is not told it again");
}

#[test]
fn a_media_that_never_opens_tells_that_it_failed_and_nothing_else() {
  let (media, driver) = Media::driven();
  let log = Log::default();
  let say = log.say();
  media.on_load({
    let say = say.clone();
    move |info| say(loaded(info))
  });
  media.on_playing_change({
    let say = say.clone();
    move |plays| say(format!("playing {plays}"))
  });
  media.on_error(move |error| say(format!("error {error:?}")));
  let mut stage = stage(|_| Column::new().into());
  media.play();
  driver.fail(MediaError::NotFound);
  stage.pass();
  assert_eq!(log.take(), ["error NotFound"]);
}

#[test]
fn what_a_media_did_before_it_was_listened_to_is_told_in_the_next_pass() {
  let (media, driver) = Media::driven();
  driver.open(SONG);
  let mut stage = stage(|_| Column::new().into());
  let log = Log::default();
  let say = log.say();
  media.on_load(move |info| say(loaded(info)));
  stage.pass();
  assert_eq!(log.take(), ["load Some(180.0) no picture sound true"]);
}

/// A video player of a source, with a handler of each event.
fn video(log: &Log) -> VideoPlayer {
  let say = log.say();
  let (a, b, c, d, e) = (say.clone(), say.clone(), say.clone(), say.clone(), say);
  VideoPlayer::new()
    .id("v")
    .width(640.0)
    .source(&Source::file("clips/lake.mp4"))
    .on_load(move |info| a(loaded(info)))
    .on_error(move |error| b(format!("error {error}")))
    .on_ended(move || c("ended".to_owned()))
    .on_playing_change(move |plays| d(format!("playing {plays}")))
    .on_waiting_change(move |waits| e(format!("waiting {waits}")))
}

#[test]
fn a_video_player_tells_its_sources_events_and_a_press_is_not_told_twice() {
  let drive = drive_media();
  let log = Log::default();
  let made = log.clone();
  let mut stage = stage(move |ctx| video(&made).mount(ctx));
  assert_eq!(drive.count(), 1, "its source is opened once");
  let media = drive.last();
  assert!(log.take().is_empty());

  media.open(CLIP);
  stage.pass();
  assert_eq!(log.take(), ["load Some(15.0) 1280x720 sound true"]);

  // A press tells the app, and the media's playing is not told again.
  stage.click("v-play");
  assert_eq!(log.take(), ["playing true"]);
  stage.pass();
  assert!(log.take().is_empty(), "the press told it");

  media.wait(true);
  stage.pass();
  assert_eq!(log.take(), ["waiting true"]);
  media.wait(false);
  stage.pass();
  assert_eq!(log.take(), ["waiting false"]);

  // It ends by itself: the app is told it stopped, and that it ended.
  media.end();
  stage.pass();
  assert_eq!(log.take(), ["playing false", "ended"]);
  stage.pass();
  assert!(log.take().is_empty());
  // Replay, and a second end.
  stage.click("v-play");
  assert_eq!(log.take(), ["playing true"]);
  stage.pass();
  media.end();
  stage.pass();
  assert_eq!(log.take(), ["playing false", "ended"]);
  assert_eq!(drive.count(), 1);
}

#[test]
fn a_video_player_tells_an_error_once_and_again_after_the_person_tried_again() {
  let drive = drive_media();
  let log = Log::default();
  let made = log.clone();
  let mut stage = stage(move |ctx| video(&made).mount(ctx));
  drive.last().fail(MediaError::Unsupported);
  stage.pass();
  assert_eq!(log.take(), ["error Unsupported format"]);
  assert!(
    stage.paints("Unsupported format"),
    "the same kind, in the words the player shows"
  );
  stage.pass();
  assert!(log.take().is_empty(), "once");
  // Trying again opens the source anew: its failure is told again, once.
  stage.click("v-retry");
  stage.pass();
  assert_eq!(drive.count(), 2);
  drive.last().fail(MediaError::Unsupported);
  stage.pass();
  assert_eq!(log.take(), ["error Unsupported format"]);
}

#[test]
fn a_player_bound_to_playing_is_not_told_what_the_app_set_and_follows_the_media_at_its_end() {
  let drive = drive_media();
  let (log, playing) = (Log::default(), Signal::new(false));
  let (made, bound) = (log.clone(), playing.clone());
  let mut stage = stage(move |ctx| video(&made).playing(bound.clone()).mount(ctx));
  let media = drive.last();
  media.open(CLIP);
  stage.pass();
  assert_eq!(log.take(), ["load Some(15.0) 1280x720 sound true"]);
  // The app plays it: it plays, and the app is not told what it set.
  playing.set(true);
  stage.pass();
  stage.pass();
  assert!(media.pending() == 0 && log.take().is_empty());
  // At its end the signal follows the media, and that is not the app pausing it.
  media.end();
  stage.pass();
  assert_eq!(log.take(), ["playing false", "ended"]);
  assert!(!playing.get_untracked());
  stage.pass();
  assert_eq!(media.pending(), 0, "nothing was told to the media");
  // The app plays it again.
  playing.set(true);
  stage.pass();
  stage.pass();
  assert!(log.take().is_empty());
  media.end();
  stage.pass();
  assert_eq!(log.take(), ["playing false", "ended"]);
}

#[test]
fn a_player_without_a_source_tells_none_of_its_values_the_app_holds() {
  let log = Log::default();
  let made = log.clone();
  let state = Signal::new(0_u8);
  let shown = state.clone();
  let mut stage = stage(move |ctx| {
    let say = made.say();
    let (a, b, c, d) = (say.clone(), say.clone(), say.clone(), say);
    let player = VideoPlayer::new()
      .id("v")
      .width(640.0)
      .duration(15.0)
      .on_load(move |info| a(loaded(info)))
      .on_error(move |error| b(format!("error {error}")))
      .on_ended(move || c("ended".to_owned()))
      .on_waiting_change(move |waits| d(format!("waiting {waits}")));
    match shown.get() {
      0 => player,
      1 => player.waiting(true),
      2 => player.ended(true),
      _ => player.error("It cannot play"),
    }
    .mount(ctx)
  });
  for at in 1..4 {
    state.set(at);
    stage.pass();
  }
  assert!(log.take().is_empty());
}

#[test]
fn an_audio_player_tells_its_sources_events_and_each_end_of_a_track_it_repeats() {
  use lurq::ui::AudioRepeat;
  let drive = drive_media();
  let (log, repeat) = (Log::default(), Signal::new(AudioRepeat::Off));
  let (made, repeats) = (log.clone(), repeat.clone());
  let mut stage = stage(move |_| {
    let say = made.say();
    let (a, b, c, d, e) = (say.clone(), say.clone(), say.clone(), say.clone(), say);
    AudioPlayer::controlled(0.0, 0.0, |_| {})
      .source(&Source::bytes(b"a song".to_vec()))
      .title("Night Drive")
      .id("p")
      .repeat(repeats.clone())
      .on_load(move |info| a(loaded(info)))
      .on_error(move |error| b(format!("error {error}")))
      .on_ended(move || c("ended".to_owned()))
      .on_playing_change(move |plays| d(format!("playing {plays}")))
      .on_waiting_change(move |waits| e(format!("waiting {waits}")))
      .into()
  });
  assert_eq!(drive.count(), 1, "bytes are opened at once");
  let media = drive.last();
  media.open(SONG);
  stage.pass();
  assert_eq!(log.take(), ["load Some(180.0) no picture sound true"]);
  stage.click("p-play");
  assert_eq!(log.take(), ["playing true"]);
  stage.pass();
  assert!(log.take().is_empty(), "the press told it");
  media.end();
  stage.pass();
  assert_eq!(log.take(), ["playing false", "ended"]);

  // One track again and again: each end is told, and it plays on by itself.
  repeat.set(AudioRepeat::One);
  stage.click("p-play");
  assert_eq!(log.take(), ["playing true"]);
  stage.pass();
  media.end();
  stage.pass();
  stage.pass();
  assert_eq!(log.take(), ["playing false", "ended", "playing true"]);
  media.end();
  stage.pass();
  stage.pass();
  assert_eq!(log.take(), ["playing false", "ended", "playing true"]);
  // It fails: once, in the words of its line.
  media.fail(MediaError::Server(503));
  stage.pass();
  assert_eq!(log.take(), ["playing false", "error Server error 503"]);
  assert!(stage.paints("Server error 503"));
}

#[test]
fn a_looped_video_player_tells_each_end_and_goes_on_playing() {
  let drive = drive_media();
  let log = Log::default();
  let made = log.clone();
  let mut stage = stage(move |ctx| video(&made).looped(true).mount(ctx));
  let media = drive.last();
  media.open(CLIP);
  stage.pass();
  log.take();
  stage.click("v-play");
  assert_eq!(log.take(), ["playing true"]);
  stage.pass();

  media.end();
  stage.pass();
  stage.pass();
  assert_eq!(log.take(), ["ended"], "the end is told, and it did not stop playing");
  media.end();
  stage.pass();
  assert_eq!(log.take(), ["ended"], "and each time round");
  assert_eq!(drive.count(), 1, "nothing is opened again");
}

#[test]
fn a_video_player_tells_it_started_when_a_frame_is_shown_after_it_could_move() {
  let drive = drive_media();
  let log = Log::default();
  let made = log.clone();
  let mut stage = stage(move |ctx| {
    let say = made.say();
    video(&made)
      .looped(true)
      .on_started(move || say("started".to_owned()))
      .mount(ctx)
  });
  let media = drive.last();
  media.open(CLIP);
  // Its first frame, shown while it stands: it opened, and has not started.
  media.present();
  stage.pass();
  assert_eq!(log.take(), ["load Some(15.0) 1280x720 sound true"]);

  // Told to play is not yet playing: it started when the first frame since is shown, and once.
  stage.click("v-play");
  assert_eq!(log.take(), ["playing true"]);
  stage.pass();
  assert!(log.take().is_empty(), "nothing is shown yet");
  media.present();
  stage.pass();
  assert_eq!(log.take(), ["started"]);
  media.present();
  media.present();
  stage.pass();
  assert!(log.take().is_empty(), "the frames that follow are no start");

  // It waits for what it plays and moves again: a start.
  media.wait(true);
  stage.pass();
  media.present();
  stage.pass();
  assert_eq!(log.take(), ["waiting true"], "a frame shown while it waits is no start");
  media.wait(false);
  stage.pass();
  assert_eq!(log.take(), ["waiting false"]);
  media.present();
  stage.pass();
  assert_eq!(log.take(), ["started"]);

  // A round of a looped one is an end and no stop: no start.
  media.end();
  stage.pass();
  media.present();
  stage.pass();
  assert_eq!(log.take(), ["ended"]);

  // Paused and played again: a start.
  stage.click("v-play");
  assert_eq!(log.take(), ["playing false"]);
  stage.pass();
  media.present();
  stage.pass();
  assert!(log.take().is_empty(), "paused, a frame is no start");
  stage.click("v-play");
  assert_eq!(log.take(), ["playing true"]);
  stage.pass();
  media.present();
  stage.pass();
  assert_eq!(log.take(), ["started"]);
}

#[test]
fn an_audio_player_tells_it_started_when_its_sound_is_heard() {
  let drive = drive_media();
  let log = Log::default();
  let made = log.clone();
  let mut stage = stage(move |ctx| {
    let (started, played) = (made.say(), made.say());
    AudioPlayer::new(lurq::core::Signal::new(0.0), 0.0)
      .id("a")
      .source(&Source::file("songs/night-drive.mp3"))
      .on_playing_change(move |plays| played(format!("playing {plays}")))
      .on_started(move || started("started".to_owned()))
      .mount(ctx)
  });
  let media = drive.last();
  media.open(SONG);
  stage.pass();
  log.take();
  stage.click("a-play");
  assert_eq!(log.take(), ["playing true"]);
  stage.pass();
  assert!(log.take().is_empty());
  media.present();
  stage.pass();
  assert_eq!(log.take(), ["started"]);
}
