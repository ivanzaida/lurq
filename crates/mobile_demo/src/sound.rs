//! A sound, played by the system's own player: from bytes in the app, from a file, and from an address, which
//! is loaded once and kept. What a media is told and what it says back, on buttons and in plain lines, and
//! the design system's player over the same thing.
//!
//! What to check here on a phone is in `docs/evidence/media-sound/apple-verification.md`.

use std::time::Duration;

use lurq::{
  app::{
    component::Component,
    ctx::{Ctx, Interval},
  },
  components::{Column, Row, StackOutlet},
  core::Signal,
  media::{Media, sound_cache},
  node::Element,
  resources::{Loader, Source},
  ui::{AudioPlayer, AudioPlayerVariant},
};

use crate::ui::{BG, FILL, button, caption, card, label};

/// The gallery's own track: 24 seconds of plucked notes, AAC in an MP4 file, in the app's binary.
const TRACK: &[u8] = include_bytes!("../../gallery/assets/media/track.m4a");

/// A sound anyone may fetch: an MP3 of six minutes.
const ADDRESS: &str = "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3";

/// The track as a file in the app's temporary folder, written there once.
fn track_file() -> Source {
  static FILE: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
  let path = FILE.get_or_init(|| {
    let path = std::env::temp_dir().join("lurq-mobile-demo-track.m4a");
    let _ = std::fs::write(&path, TRACK);
    path
  });
  Source::file(path)
}

fn track_bytes() -> Source {
  static BYTES: std::sync::OnceLock<Source> = std::sync::OnceLock::new();
  BYTES.get_or_init(|| Source::bytes(TRACK.to_vec())).clone()
}

pub(crate) struct SoundScreen {
  /// The sound the buttons drive: the track, from the app's own bytes.
  media: Media,
  /// Counted up a few times a second, and when the media says it changed: the lines are read again.
  beat: Signal<u64>,
  muted: Signal<bool>,
  /// Where the design system's players are, which their sounds say.
  file_at: Signal<f32>,
  address_at: Signal<f32>,
  _clock: Interval,
}

impl Component for SoundScreen {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    // The app's loader, with its resource root and what it allows.
    let loader: Loader = ctx.app_ref().loader().clone();
    let media = Media::sound(&loader, &track_bytes());
    media.set_volume(0.5);
    let beat = ctx.signal(0_u64);
    media.on_change({
      let beat = beat.clone();
      move || beat.update(|count| *count += 1)
    });
    let clock = ctx.create_interval(Duration::from_millis(250), {
      let beat = beat.clone();
      move || beat.update(|count| *count += 1)
    });
    clock.start();
    Self {
      media,
      beat,
      muted: ctx.signal(false),
      file_at: ctx.signal(0.0),
      address_at: ctx.signal(0.0),
      _clock: clock,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    self.beat.get();
    let state = self.media.state();
    let does = |what: fn(&Media)| {
      let media = self.media.clone();
      move || what(&media)
    };
    let mute = {
      let (media, muted) = (self.media.clone(), self.muted.clone());
      move || {
        let to = !muted.get_untracked();
        muted.set(to);
        media.set_muted(to);
      }
    };
    let said = Column::new()
      .spacing(4.0)
      .child(label(&format!(
        "{:.1} s of {}",
        state.position,
        state
          .duration
          .map_or("an unknown length".to_owned(), |duration| format!("{duration:.1} s"))
      )))
      .child(label(&format!(
        "{}{}{}",
        if state.playing { "playing" } else { "not playing" },
        if state.waiting { ", waiting" } else { "" },
        if state.ended { ", ended" } else { "" },
      )))
      .child(label(&format!(
        "speed {:.1}×, volume {:.1}{}, sound: {}",
        state.rate,
        state.volume,
        if state.muted { ", muted" } else { "" },
        if state.sound { "yes" } else { "no" },
      )))
      .child(label(&match &state.error {
        Some(why) => format!("error: {why}"),
        None => "no error".to_owned(),
      }));
    let pair = |one: Row, other: Row| {
      Row::new()
        .width(FILL)
        .spacing(8.0)
        .child(one.flex(1.0))
        .child(other.flex(1.0))
    };
    let address = Source::url(ADDRESS);
    let kept = sound_cache().holds(&address);
    let content = Column::new()
      .width(FILL)
      .padding(16.0)
      .spacing(12.0)
      .child(
        caption("A media that is only heard, from the bytes of a track in the app. Its volume starts at a half.")
          .width(FILL),
      )
      .child(card(said))
      .child(pair(
        button("Play", does(Media::play)),
        button("Pause", does(Media::pause)),
      ))
      .child(pair(
        button("To 10 s", does(|media| media.seek(10.0))),
        button("To 22 s", does(|media| media.seek(22.0))),
      ))
      .child(pair(
        button("Speed 1×", does(|media| media.set_rate(1.0))),
        button("Speed 2×", does(|media| media.set_rate(2.0))),
      ))
      .child(pair(
        button("Volume 0.2", does(|media| media.set_volume(0.2))),
        button("Volume 1", does(|media| media.set_volume(1.0))),
      ))
      .child(button(if self.muted.get() { "Unmute" } else { "Mute" }, mute).width(FILL))
      .child(caption("The design system's player, of the same track read from a file:").width(FILL))
      .child(
        AudioPlayer::new(self.file_at.clone(), 0.0)
          .variant(AudioPlayerVariant::Compact)
          .source(&track_file()),
      )
      .child(
        caption(&format!(
          "And of a sound at an address, loaded when it is first played. Now it is {}; {:.1} MB of sounds are kept.",
          if kept { "kept" } else { "not kept" },
          sound_cache().bytes() as f64 / (1024.0 * 1024.0)
        ))
        .width(FILL),
      )
      .child(
        AudioPlayer::new(self.address_at.clone(), 0.0)
          .variant(AudioPlayerVariant::Compact)
          .source(&address),
      )
      .child(
        button("Forget the sound that is kept", move || {
          sound_cache().forget(&Source::url(ADDRESS));
        })
        .width(FILL),
      );
    // Longer than a phone is high: it scrolls under the bar.
    let screen: Element = StackOutlet::large_title_scroll(ctx, content)
      .size(FILL, FILL)
      .background(BG)
      .into();
    screen
  }
}
