//! The thread of a media on macOS and iOS. What is only heard ([`Media::sound`](super::Media::sound)) is played by
//! the system's own player of a sound (`AVAudioPlayer`), a file or bytes, and the media's clock follows where the
//! player says it is. What is seen ([`Media::open`](super::Media::open)) is played by the system's player of a
//! video (`AVPlayer`): see [`video`].
//!
//! An address is a sound once the app's loader has loaded it whole, which is what a media does before it comes
//! here; an address the loader could not load (a build without `http`, or more than the loader reads whole) is
//! played by the player of a video, without its picture, since that player reads an address and this one does
//! not.
//!
//! Written on Windows; its tests (`tests/media_tests.rs`) pass on a Mac since 2026-11-17, muted. What is still to
//! check by ear, and on a phone, is in `docs/evidence/media-sound/apple-verification.md`.

use std::{
  sync::Arc,
  time::{Duration, Instant},
};

use objc2::{
  AnyThread,
  rc::{Retained, autoreleasepool},
};
use objc2_avf_audio::AVAudioPlayer;
use objc2_foundation::{NSData, NSError, NSURL};

use super::{Command, MediaError, Opened, Shared};

mod feed;
mod video;

/// How often the thread looks at the player while it plays: where it is, and whether it came to its end.
const STEP: Duration = Duration::from_millis(20);
/// The media's clock is set to the player's where they are this far apart, in seconds.
const APART: f64 = 0.03;
/// How long a player told to play may still say it does not: started again from its end, it says so for a moment
/// after `play` returned. Not playing past this, it stopped.
const STARTING: Duration = Duration::from_millis(500);
/// How often a sound the system paused looks whether the interruption ended with it to go on.
const INTERRUPTED: Duration = Duration::from_millis(250);

/// Runs a sound to its end: opens `opened`, and does what it is told until nothing holds the media.
pub(super) fn run(shared: &Arc<Shared>, opened: Opened, heard: bool) {
  // What is seen is the system's player of a video's, and so is a sound at an address that was not loaded whole
  // (more than the loader reads, or a build without `http`): that player reads an address by itself as it plays.
  if !heard || matches!(opened, Opened::Url(..) | Opened::Reader(..)) {
    return video::run(shared, opened, !heard);
  }
  // What the system makes while the thread runs is let go when the thread ends.
  autoreleasepool(|_| match open(opened) {
    Ok(player) => play(shared, &player),
    Err(why) => shared.fail(why),
  });
}

/// What the system's error is, as a media says it. The player answers a file it does not play with one of the
/// audio file's own codes ('typ?', 'fmt?', 'dta?': a kind, a format or data it has no reader for); the rest is
/// kept in the system's words.
fn failed(error: &NSError) -> MediaError {
  const UNSUPPORTED: [isize; 3] = [0x7479_703F, 0x666D_743F, 0x6474_613F];
  if UNSUPPORTED.contains(&error.code()) {
    MediaError::Unsupported
  } else {
    MediaError::Other(Arc::from(error.localizedDescription().to_string()))
  }
}

/// A player of `opened`, ready to play.
fn open(opened: Opened) -> Result<Retained<AVAudioPlayer>, MediaError> {
  // SAFETY: the player is made, used and let go on this thread alone; its initializers take what they are given
  // and answer an error where they cannot read it.
  let player = unsafe {
    match opened {
      Opened::File(path) => {
        let address = NSURL::from_file_path(&path).ok_or(MediaError::NotFound)?;
        AVAudioPlayer::initWithContentsOfURL_error(AVAudioPlayer::alloc(), &address)
      }
      // The player tells what the bytes are from the bytes themselves.
      Opened::Bytes(bytes, _) => AVAudioPlayer::initWithData_error(AVAudioPlayer::alloc(), &NSData::with_bytes(&bytes)),
      // Played by the player of a video, which reads an address: see `run`.
      Opened::Url(..) | Opened::Reader(..) => return Err(MediaError::Unsupported),
    }
  }
  .map_err(|error| failed(&error))?;
  // SAFETY: as above.
  unsafe {
    // A speed other than 1 is taken only by a player told so before it plays.
    player.setEnableRate(true);
    player.prepareToPlay();
  }
  Ok(player)
}

fn play(shared: &Shared, player: &AVAudioPlayer) {
  // SAFETY: every call of the player in here is on the thread that made it.
  unsafe {
    let duration = Some(player.duration()).filter(|duration| duration.is_finite() && *duration > 0.0);
    let (mut volume, mut muted) = {
      let mut inner = shared.inner();
      inner.duration = duration;
      inner.sound = true;
      inner.opened = true;
      inner.waiting = false;
      inner.since = Instant::now();
      (inner.volume, inner.muted)
    };
    let level = |volume: f32, muted: bool| if muted { 0.0 } else { volume };
    player.setVolume(level(volume, muted));
    shared.wake();
    // How many of the times it was sent somewhere it has done.
    let mut gone = 0_u64;
    // When it was told to play, until the player says it plays.
    let mut starting: Option<Instant> = None;
    // Where the system paused it while it played (a call, another app's sound), how many interruptions had ended
    // then with the system saying a sound may go on: one more, and it plays on by itself. Anything it is told
    // forgets it.
    let mut interrupted: Option<u64> = None;
    loop {
      let playing = shared.inner().playing;
      // Nothing to do until it is told something, unless it plays, or waits for an interruption to end.
      let wait = match (playing, interrupted) {
        (true, _) => STEP,
        (false, Some(_)) => INTERRUPTED,
        (false, None) => Duration::from_secs(3600),
      };
      let mut commands = shared.take(Some(wait));
      if !commands.is_empty() {
        interrupted = None;
      } else if interrupted.is_some_and(|then| super::session::resumes() > then) {
        interrupted = None;
        commands.push(Command::Play);
      }
      for command in commands {
        match command {
          Command::Close => {
            player.stop();
            return;
          }
          Command::Play => {
            let mut inner = shared.inner();
            if inner.ended {
              player.setCurrentTime(0.0);
              inner.ended = false;
              inner.at = 0.0;
            }
            // The player's speed is set again each time it starts: a player stopped at its end forgets it.
            player.setRate(inner.rate as f32);
            if player.play() {
              inner.since = Instant::now();
              inner.playing = true;
              starting = Some(Instant::now());
              drop(inner);
              // Who draws it may have asked whether it plays before this thread heard the command.
              shared.wake();
            } else {
              drop(inner);
              shared.fail(MediaError::Other(Arc::from("the system's output would not start")));
              return;
            }
          }
          Command::Pause => {
            player.pause();
            // The media held its clock where it was when it was told, and says so: the player goes on from there.
            let mut inner = shared.inner();
            player.setCurrentTime(inner.at);
            inner.since = Instant::now();
            inner.playing = false;
            starting = None;
          }
          Command::Seek(to) | Command::SeekNear(to) => {
            gone += 1;
            let to = duration.map_or(to, |duration| to.min(duration));
            player.setCurrentTime(to);
            let mut inner = shared.inner();
            inner.at = to;
            inner.since = Instant::now();
            inner.ended = false;
          }
          Command::Rate(rate) => {
            let mut inner = shared.inner();
            inner.hold();
            inner.rate = rate;
            player.setRate(rate as f32);
          }
          Command::Volume(to) => {
            volume = to;
            player.setVolume(level(volume, muted));
          }
          Command::Muted(to) => {
            muted = to;
            player.setVolume(level(volume, muted));
          }
        }
      }
      let mut inner = shared.inner();
      if !inner.playing {
        continue;
      }
      if !player.isPlaying() && starting.is_some_and(|told| told.elapsed() < STARTING) {
        continue;
      }
      if player.isPlaying() {
        starting = None;
        shared.presented_in(&mut inner);
        // The media's clock is the player's: what is heard is where it is.
        let heard = player.currentTime();
        if inner.sent == gone && (heard - inner.position()).abs() > APART {
          inner.at = heard;
          inner.since = Instant::now();
        }
      } else {
        // The player stopped by itself. At the end of what it plays it is back at its start; anywhere else the
        // system stopped it (a call, another app's sound, the headphones pulled out), and it is paused there.
        // Where the player says it is right after its end is not always its start: the media's own clock, which
        // went on from where it was told to play, says it came to the end too.
        let at = player.currentTime();
        let clock = inner.position();
        let over = duration.is_none_or(|duration| at <= 0.05 || at.max(clock) >= duration - 0.3);
        if over && inner.looped {
          // Looped: it plays on from its start, in the player it has.
          player.setCurrentTime(0.0);
          player.play();
          inner.at = 0.0;
          inner.rounds += 1;
          inner.since = Instant::now();
          starting = Some(Instant::now());
          continue;
        }
        inner.at = match (over, inner.duration) {
          (true, Some(duration)) => duration,
          _ => at,
        };
        inner.since = Instant::now();
        inner.ended = over;
        inner.playing = false;
        drop(inner);
        if !over {
          interrupted = Some(super::session::resumes());
        }
        shared.wake();
      }
    }
  }
}
