//! An audio player: the card, the compact row, the voice message and the mini player against the board's numbers,
//! their states, what each control tells, the keys, a title longer than its room, a disabled player, and the mini
//! player under 480 wide.

use std::sync::{Arc, Mutex};

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::Column,
  core::Signal,
  layout::quad::QuadContent,
  node::color::Color,
  ui::{AudioPlayer, AudioPlayerVariant, AudioRepeat},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// A log the handlers write to.
fn log() -> Arc<Mutex<Vec<String>>> {
  Arc::new(Mutex::new(Vec::new()))
}

fn said(log: &Arc<Mutex<Vec<String>>>) -> Vec<String> {
  log.lock().unwrap().clone()
}

/// The board's card: "Night Drive" by "Aurora Lane · Low Tide", 1:24 of 4:10, every handler writing to `log`.
fn card(log: &Arc<Mutex<Vec<String>>>) -> AudioPlayer {
  let say = |log: &Arc<Mutex<Vec<String>>>, what: &'static str| {
    let log = log.clone();
    move || log.lock().unwrap().push(what.to_owned())
  };
  let (a, b, c, d, e, f, g) = (
    log.clone(),
    log.clone(),
    log.clone(),
    log.clone(),
    log.clone(),
    log.clone(),
    log.clone(),
  );
  AudioPlayer::controlled(84.0, 250.0, move |to| a.lock().unwrap().push(format!("seek {to}")))
    .on_change_end(move |to| b.lock().unwrap().push(format!("seek end {to}")))
    .title("Night Drive")
    .description("Aurora Lane · Low Tide")
    .buffered(140.0)
    .playing_controlled(false, move |playing| {
      c.lock().unwrap().push(format!("playing {playing}"))
    })
    .shuffle_controlled(false, move |on| d.lock().unwrap().push(format!("shuffle {on}")))
    .repeat_controlled(AudioRepeat::Off, move |repeat| {
      e.lock().unwrap().push(format!("repeat {repeat:?}"))
    })
    .volume_controlled(0.6, move |volume| f.lock().unwrap().push(format!("volume {volume}")))
    .muted_controlled(false, move |muted| g.lock().unwrap().push(format!("muted {muted}")))
    .on_previous(say(log, "previous"))
    .on_next(say(log, "next"))
    .on_more(say(log, "more"))
    .on_retry(say(log, "retry"))
    .id("p")
}

#[test]
fn the_card_is_the_boards() {
  let log = log();
  let shown = log.clone();
  let mut stage = Stage::new(move |_| card(&shown).build(&lurq::ui::current_theme()));
  let (x, y, width, _) = stage.bounds("p");
  assert_eq!(width, 360.0, "360 wide");
  assert_eq!(stage.fill_color("p"), Some(light(PaletteColor::Card)));

  // The artwork: 64, on the muted fill where there is none, 20 in from the edges.
  let artwork = stage
    .quads()
    .into_iter()
    .find(|quad| (quad.width, quad.height) == (64.0, 64.0))
    .expect("the artwork's box of 64");
  assert_eq!((artwork.x - x, artwork.y - y), (20.0, 20.0));
  assert!(matches!(artwork.content, QuadContent::Rect { color, .. } if color == light(PaletteColor::Muted)));
  // Its title 12 beside it, 14; who it is by 13 in the muted ink.
  let title = stage.text_quad("Night Drive").unwrap();
  assert_eq!(title.x - (artwork.x + 64.0), 12.0);
  assert_eq!(stage.text("Night Drive"), (light(PaletteColor::Foreground), 14.0));
  assert_eq!(
    stage.text("Aurora Lane · Low Tide"),
    (light(PaletteColor::MutedForeground), 13.0)
  );
  // The times at the ends of the bar, mono 11 in the muted ink.
  assert_eq!(stage.text("1:24"), (light(PaletteColor::MutedForeground), 11.0));
  assert_eq!(stage.text("4:10"), (light(PaletteColor::MutedForeground), 11.0));
  assert!(stage.font_of("1:24").to_lowercase().contains("mono"));
  // The transport: shuffle 32, previous 36, play 48, next 36, repeat 32, in a row.
  for (id, side) in [
    ("p-shuffle", 32.0),
    ("p-previous", 36.0),
    ("p-play", 48.0),
    ("p-next", 36.0),
    ("p-repeat", 32.0),
  ] {
    let (_, _, w, h) = stage.bounds(id);
    assert_eq!((w, h), (side, side), "{id}");
  }
  let (play_x, ..) = stage.bounds("p-play");
  assert!(
    (play_x + 24.0 - (x + 180.0)).abs() < 1.0,
    "play is in the middle of the card"
  );
  // The bar is as wide as the card's room, 20 in from each edge.
  let (seek_x, _, seek_width, _) = stage.bounds("p-seek");
  assert_eq!((seek_x - x, seek_width), (20.0, 320.0));
  // The more button 32, and the speed "1×" at the end of the last row.
  assert_eq!(stage.bounds("p-more").2, 32.0);
  assert!(stage.paints("1×"));
  // Shuffle and repeat are off: no accent fill.
  assert_ne!(stage.fill_color("p-shuffle"), Some(light(PaletteColor::Highlight)));
  assert!(said(&log).is_empty());
}

#[test]
fn shuffle_and_repeat_on_take_the_accent_fill() {
  let mut stage = Stage::new(|_| {
    AudioPlayer::controlled(84.0, 250.0, |_| {})
      .shuffle_controlled(true, |_| {})
      .repeat_controlled(AudioRepeat::One, |_| {})
      .playing_controlled(true, |_| {})
      .id("p")
      .build(&lurq::ui::current_theme())
  });
  assert_eq!(stage.fill_color("p-shuffle"), Some(light(PaletteColor::Highlight)));
  assert_eq!(stage.fill_color("p-repeat"), Some(light(PaletteColor::Highlight)));
}

#[test]
fn each_control_tells_what_it_is_to_be_and_changes_nothing() {
  let log = log();
  let shown = log.clone();
  let mut stage = Stage::new(move |_| card(&shown).build(&lurq::ui::current_theme()));
  for id in [
    "p-play",
    "p-shuffle",
    "p-repeat",
    "p-previous",
    "p-next",
    "p-more",
    "p-volume",
  ] {
    stage.click(id);
  }
  assert_eq!(
    said(&log),
    [
      "playing true",
      "shuffle true",
      "repeat All",
      "previous",
      "next",
      "more",
      "muted true"
    ]
  );
  // The app did not answer: the play button still says play, shuffle is still off.
  assert_ne!(stage.fill_color("p-shuffle"), Some(light(PaletteColor::Highlight)));
}

#[test]
fn a_bound_player_sets_the_apps_signals_first() {
  let (playing, repeat) = (Signal::new(false), Signal::new(AudioRepeat::One));
  let told = log();
  let (shown_playing, shown_repeat, shown_told) = (playing.clone(), repeat.clone(), told.clone());
  let mut stage = Stage::new(move |_| {
    let (seen, again) = (shown_playing.clone(), shown_told.clone());
    AudioPlayer::controlled(0.0, 60.0, |_| {})
      .playing(shown_playing.clone())
      .on_playing_change(move |next| {
        // The signal is set before the handler is told.
        again.lock().unwrap().push(format!("{next} {}", seen.get_untracked()));
      })
      .repeat(shown_repeat.clone())
      .id("p")
      .build(&lurq::ui::current_theme())
  });
  stage.click("p-play");
  assert!(playing.get_untracked());
  assert_eq!(said(&told), ["true true"]);
  stage.click("p-repeat");
  assert_eq!(repeat.get_untracked(), AudioRepeat::Off, "one steps back to off");
}

#[test]
fn space_plays_from_the_bar_and_the_arrows_move_it() {
  let log = log();
  let shown = log.clone();
  let mut stage = Stage::new(move |_| card(&shown).build(&lurq::ui::current_theme()));
  // Tab reaches the more button, then the seek bar.
  stage.key("Tab");
  assert!(
    stage.has_ring(light(PaletteColor::Ring)),
    "the focus shows from the keyboard"
  );
  stage.key("Tab");
  stage.key(" ");
  assert_eq!(said(&log), ["playing true"], "Space on the bar plays");
  log.lock().unwrap().clear();
  stage.key("ArrowRight");
  assert!(
    said(&log).iter().any(|what| what.starts_with("seek")),
    "an arrow moves the bar: {:?}",
    said(&log)
  );
}

#[test]
fn the_keyboard_presses_the_play_button() {
  let log = log();
  let shown = log.clone();
  let mut stage = Stage::new(move |_| {
    let log = shown.clone();
    AudioPlayer::controlled(0.0, 60.0, |_| {})
      .variant(AudioPlayerVariant::Compact)
      .playing_controlled(false, move |next| log.lock().unwrap().push(format!("playing {next}")))
      .id("c")
      .build(&lurq::ui::current_theme())
  });
  stage.key("Tab");
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  stage.key(" ");
  assert_eq!(said(&log), ["playing true"]);
}

#[test]
fn loading_draws_lines_and_takes_no_press() {
  let log = log();
  let shown = log.clone();
  let mut stage = Stage::new(move |_| card(&shown).loading(true).build(&lurq::ui::current_theme()));
  assert!(!stage.paints("Night Drive"), "lines stand for the title");
  assert!(!stage.paints("1:24"));
  let unknown = stage
    .quads()
    .iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == "-:--"))
    .count();
  assert_eq!(unknown, 2, "both times are not known");
  for id in ["p-play", "p-previous", "p-next"] {
    stage.click(id);
  }
  assert!(said(&log).is_empty(), "{:?}", said(&log));
}

#[test]
fn an_error_says_what_went_wrong_and_tries_again() {
  let log = log();
  let shown = log.clone();
  let mut stage = Stage::new(move |_| {
    card(&shown)
      .error(Some("This track can't be played."))
      .build(&lurq::ui::current_theme())
  });
  assert_eq!(
    stage.text("This track can't be played."),
    (light(PaletteColor::Destructive), 12.0)
  );
  assert!(!stage.paints("1:24"), "the error stands in place of the times");
  stage.click("p-play");
  assert!(said(&log).is_empty(), "play takes no press after an error");
  stage.click("p-retry");
  assert_eq!(said(&log), ["retry"]);
}

#[test]
fn a_long_title_is_cut_and_the_buttons_keep_their_room() {
  let long = "A title far longer than the room the card leaves it beside its artwork and its button";
  let mut stage = Stage::new(move |_| {
    AudioPlayer::controlled(0.0, 60.0, |_| {})
      .title(long)
      .description("Someone")
      .id("p")
      .build(&lurq::ui::current_theme())
  });
  let (x, _, width, _) = stage.bounds("p");
  let title = stage.cut_text_quad(long).expect("the title, cut");
  assert!(
    title.x + title.width <= x + width - 20.0 - 32.0 + 0.5,
    "the title ends before the button"
  );
  assert_eq!(stage.bounds("p-more").2, 32.0, "the more button keeps its room");
  assert_eq!(width, 360.0, "the card keeps its width");
}

#[test]
fn a_disabled_player_takes_nothing() {
  let log = log();
  let shown = log.clone();
  let mut stage = Stage::new(move |_| card(&shown).disabled(true).build(&lurq::ui::current_theme()));
  for id in ["p-play", "p-shuffle", "p-next", "p-more"] {
    stage.click(id);
  }
  stage.key("Tab");
  stage.key(" ");
  assert!(said(&log).is_empty(), "{:?}", said(&log));
}

#[test]
fn the_compact_row_is_48_with_a_play_button_of_32() {
  let log = log();
  let shown = log.clone();
  let mut stage = Stage::new(move |_| {
    let (log, retry) = (shown.clone(), shown.clone());
    Column::new()
      .spacing(12.0)
      .child(
        AudioPlayer::controlled(84.0, 250.0, |_| {})
          .variant(AudioPlayerVariant::Compact)
          .playing_controlled(true, move |next| log.lock().unwrap().push(format!("playing {next}")))
          .id("c")
          .build(&lurq::ui::current_theme()),
      )
      .child(
        AudioPlayer::controlled(0.0, 250.0, |_| {})
          .variant(AudioPlayerVariant::Compact)
          .error(Some("Couldn't load the audio."))
          .on_retry(move || retry.lock().unwrap().push("retry".to_owned()))
          .id("e")
          .build(&lurq::ui::current_theme()),
      )
      .into()
  });
  let (x, _, width, height) = stage.bounds("c");
  assert_eq!((width, height), (320.0, 48.0));
  assert_eq!(stage.fill_color("c"), Some(light(PaletteColor::Card)));
  let (play_x, _, play_width, play_height) = stage.bounds("c-play");
  assert_eq!((play_width, play_height), (32.0, 32.0));
  assert_eq!(play_x - x, 8.0, "8 from the start");
  assert_eq!(stage.text("1:24"), (light(PaletteColor::MutedForeground), 11.0));
  // The bar fills the room between the button and the time, 10 from each.
  let (seek_x, _, seek_width, _) = stage.bounds("c-seek");
  let time = stage.text_quad("1:24").unwrap();
  assert_eq!(seek_x - (play_x + 32.0), 10.0);
  assert!(
    (time.x - (seek_x + seek_width) - 10.0).abs() < 0.5,
    "10 before the time"
  );
  // The error: a line for the bar, and the button retries.
  assert_eq!(
    stage.text("Couldn't load the audio."),
    (light(PaletteColor::Destructive), 12.0)
  );
  assert_eq!(stage.bounds("e-retry").2, 32.0);
  stage.click("e-retry");
  stage.click("c-play");
  assert_eq!(said(&log), ["retry", "playing false"]);
}

#[test]
fn a_voice_message_steps_its_speed_while_it_plays() {
  let log = log();
  let shown = log.clone();
  let peaks = [0.3, 0.8, 0.5, 1.0, 0.4, 0.6, 0.2, 0.9];
  let mut stage = Stage::new(move |_| {
    let log = shown.clone();
    Column::new()
      .spacing(12.0)
      .child(
        AudioPlayer::controlled(17.0, 42.0, |_| {})
          .variant(AudioPlayerVariant::Voice)
          .peaks(&peaks)
          .playing_controlled(true, |_| {})
          .rate_controlled(1.0, move |rate| log.lock().unwrap().push(format!("rate {rate}")))
          .id("v")
          .build(&lurq::ui::current_theme()),
      )
      .child(
        AudioPlayer::controlled(0.0, 42.0, |_| {})
          .variant(AudioPlayerVariant::Voice)
          .peaks(&peaks)
          .unheard(true)
          .id("u")
          .build(&lurq::ui::current_theme()),
      )
      .into()
  });
  // 52 high: a play button of 36 between paddings of 8.
  let (_, y, _, height) = stage.bounds("v");
  assert_eq!(height, 52.0);
  let (_, play_y, play_width, play_height) = stage.bounds("v-play");
  assert_eq!((play_width, play_height, play_y - y), (36.0, 36.0, 8.0));
  // Playing: the elapsed time and the speed.
  assert!(stage.paints("0:17"));
  assert!(stage.paints("1×"));
  stage.click("v-rate");
  assert_eq!(said(&log), ["rate 1.5"]);
  // Not heard: the length, and a dot of 8 in the info color in place of the speed.
  assert!(stage.paints("0:42"));
  let dot = stage
    .quads()
    .into_iter()
    .find(|quad| (quad.width, quad.height) == (8.0, 8.0))
    .expect("the dot");
  assert!(matches!(dot.content, QuadContent::Rect { color, .. } if color == light(PaletteColor::Info)));
}

fn mini(log: &Arc<Mutex<Vec<String>>>) -> AudioPlayer {
  let (queue, expand) = (log.clone(), log.clone());
  AudioPlayer::controlled(84.0, 250.0, |_| {})
    .variant(AudioPlayerVariant::Mini)
    .title("Night Drive")
    .description("Aurora Lane")
    .on_queue(move || queue.lock().unwrap().push("queue".to_owned()))
    .on_expand(move || expand.lock().unwrap().push("expand".to_owned()))
    .id("m")
}

#[test]
fn the_mini_player_is_56_high_with_every_tool_from_480() {
  let log = log();
  let shown = log.clone();
  let mut stage = Stage::first_frame(800.0, 300.0, move |ctx| {
    Column::new().width(720.0).child(mini(&shown).mount(ctx)).into()
  });
  stage.pass();
  let (_, _, width, height) = stage.bounds("m");
  assert_eq!((width, height), (720.0, 56.0));
  assert_eq!(stage.fill_color("m"), Some(light(PaletteColor::Background)));
  for id in ["m-previous", "m-play", "m-next", "m-volume", "m-queue", "m-expand"] {
    stage.bounds(id);
  }
  let (_, _, play_width, play_height) = stage.bounds("m-play");
  assert_eq!((play_width, play_height), (36.0, 36.0));
  assert!(stage.paints("1:24"));
  assert_eq!(stage.text("Night Drive"), (light(PaletteColor::Foreground), 13.0));
  assert_eq!(stage.text("Aurora Lane"), (light(PaletteColor::MutedForeground), 12.0));
  // The progress line: 2 high, along the top edge, a third of it played.
  let (line_x, line_y, ..) = stage.bounds("m-line");
  let played = stage
    .quads()
    .into_iter()
    .find(|quad| {
      matches!(quad.content, QuadContent::Rect { color, .. } if color == light(PaletteColor::Primary))
        && quad.height == 2.0
        && quad.y == line_y
    })
    .expect("the played line");
  assert!((played.width - 720.0 * 84.0 / 250.0).abs() < 1.0);
  assert_eq!(played.x, line_x);
  stage.click("m-queue");
  stage.click("m-expand");
  assert_eq!(said(&log), ["queue", "expand"]);
}

#[test]
fn nothing_at_the_end_of_a_mini_player_is_cut_at_any_width() {
  for step in 0..=21 {
    let width = 480.0 + 20.0 * step as f32;
    let log = log();
    let mut stage = Stage::first_frame(940.0, 300.0, move |ctx| {
      Column::new().width(width).child(mini(&log).mount(ctx)).into()
    });
    stage.pass();
    let (left, _, across, _) = stage.bounds("m");
    assert_eq!(across, width);
    let (next_x, _, next_width, _) = stage.bounds("m-next");
    let mut edge = next_x + next_width;
    // The time is read out from 560, whole: it starts after the transport.
    if width >= 560.0 {
      let time = stage.text_quad("1:24").unwrap_or_else(|| panic!("the time at {width}"));
      assert!(
        time.x >= edge - 0.5,
        "the time is whole in a mini player {width} wide: from {} after {edge}",
        time.x
      );
      edge = time.x + time.width;
    } else {
      assert!(!stage.paints("1:24"), "no time under 560, at {width}");
    }
    for id in ["m-volume", "m-queue", "m-expand"] {
      let (x, _, w, _) = stage.bounds(id);
      assert!(
        x >= edge - 0.5 && x + w <= left + width + 0.5,
        "{id} keeps its room in a mini player {width} wide: from {x}, {w} wide, after {edge}"
      );
      edge = x + w;
    }
  }
}

#[test]
fn under_480_the_mini_player_keeps_the_track_play_and_next() {
  let log = log();
  let shown = log.clone();
  let mut stage = Stage::first_frame(400.0, 300.0, move |ctx| {
    Column::new().width(360.0).child(mini(&shown).mount(ctx)).into()
  });
  // The first frame already has the narrow form.
  assert!(stage.tree.get_element_by_id_mut("m-queue").is_none());
  stage.pass();
  assert!(stage.paints("Night Drive"));
  stage.bounds("m-play");
  stage.bounds("m-next");
  for id in ["m-previous", "m-volume", "m-queue", "m-expand"] {
    assert!(
      stage.tree.get_element_by_id_mut(id).is_none(),
      "{id} is not there under 480"
    );
  }
  assert!(!stage.paints("1:24 "), "no time");
}

#[test]
fn given_to_a_tree_as_it_is_a_mini_player_has_the_form_of_its_width_from_the_first_frame() {
  let log = log();
  // No `mount` and no `Ctx`: the player is a child as any element is.
  let mut stage = Stage::first_frame(400.0, 300.0, move |_| {
    Column::new().width(360.0).child(mini(&log)).into()
  });
  assert!(
    stage.tree.get_element_by_id_mut("m-queue").is_none(),
    "the first frame already has the narrow form"
  );
  stage.pass();
  stage.bounds("m-play");
  stage.bounds("m-next");
  for id in ["m-previous", "m-volume", "m-queue", "m-expand"] {
    assert!(
      stage.tree.get_element_by_id_mut(id).is_none(),
      "{id} is not there under 480"
    );
  }
}

/// A tone of 440 Hz, `seconds` long, as the bytes of a WAV file: two channels of 16 bits at 44100 a second.
#[cfg(all(any(windows, target_os = "macos"), feature = "media"))]
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

/// Renders `stage` again until `is` holds, for at most ten seconds: a media works on a thread of its own.
#[cfg(all(any(windows, target_os = "macos"), feature = "media"))]
#[track_caller]
fn until(stage: &mut Stage, what: &str, mut is: impl FnMut(&mut Stage) -> bool) {
  let started = std::time::Instant::now();
  loop {
    // The player's clock is a timer of its window, which a window with no loop does not run by itself.
    stage.tree.tick_timers_at(std::time::Instant::now());
    stage.pass();
    if is(stage) {
      return;
    }
    assert!(
      started.elapsed() < std::time::Duration::from_secs(10),
      "it never came to: {what}"
    );
    std::thread::sleep(std::time::Duration::from_millis(20));
  }
}

#[cfg(all(any(windows, target_os = "macos"), feature = "media"))]
#[test]
fn given_a_source_it_plays_the_sound_and_its_controls_move_it() {
  use lurq::resources::Source;
  let log = log();
  let heard = log.clone();
  let source = Source::bytes(tone(12.0));
  // Given to the tree as it is, and silenced: a run of the tests is not heard.
  let mut stage = Stage::new(move |_| {
    let heard = heard.clone();
    Column::new()
      .width(360.0)
      .child(
        AudioPlayer::controlled(0.0, 0.0, |_| {})
          .source(&source)
          .muted_controlled(true, |_| {})
          .on_playing_change(move |to| heard.lock().unwrap().push(format!("playing {to}")))
          .title("Tone")
          .id("p"),
      )
      .into()
  });
  // It opened: its length is the sound's, and it stands at its start.
  until(&mut stage, "its length", |stage| stage.paints("0:12"));
  assert!(stage.paints("0:00"));
  // Play: the app hears of it, and the sound goes, by its own clock.
  stage.click("p-play");
  assert_eq!(said(&log), ["playing true"]);
  until(&mut stage, "playing", |stage| stage.paints("0:01"));
  // Sent on by the keys of its bar, it is there.
  stage.click("p-seek");
  until(&mut stage, "sent to the middle", |stage| stage.paints("0:06") || stage.paints("0:07"));
  // Pause: it stands where it is.
  stage.click("p-play");
  assert_eq!(said(&log), ["playing true", "playing false"]);
  stage.pass();
  let stands = ["0:06", "0:07", "0:08"]
    .into_iter()
    .find(|time| stage.paints(time))
    .expect("the time it was paused at");
  std::thread::sleep(std::time::Duration::from_millis(1200));
  stage.tree.tick_timers_at(std::time::Instant::now());
  stage.pass();
  assert!(stage.paints(stands), "paused, the time stands");

  // What is no sound says so in the player's own line.
  let mut broken = Stage::new(|_| {
    Column::new()
      .width(360.0)
      .child(
        AudioPlayer::controlled(0.0, 0.0, |_| {})
          .source(&Source::bytes(b"not a sound".to_vec()))
          .title("Nothing")
          .id("p"),
      )
      .into()
  });
  until(&mut broken, "the error", |stage| {
    stage.tree.get_element_by_id_mut("p-retry").is_some()
  });
}

#[cfg(all(any(windows, target_os = "macos"), feature = "media", feature = "http"))]
#[test]
fn a_sound_at_an_address_is_loaded_when_it_is_first_played_and_not_for_being_drawn() {
  use std::{
    io::{Read, Write},
    sync::atomic::{AtomicUsize, Ordering},
  };

  use lurq::resources::Source;
  // A server on this machine that answers each request with the tone, and counts them.
  let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
  let address = format!("http://{}/tone.wav", listener.local_addr().unwrap());
  let asked = Arc::new(AtomicUsize::new(0));
  let counted = asked.clone();
  std::thread::spawn(move || {
    let body = tone(8.0);
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
      let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
      );
      let _ = stream.write_all(head.as_bytes());
      let _ = stream.write_all(&body);
    }
  });
  let source = Source::url(&address);
  let shown = source.clone();
  let mut stage = Stage::new(move |_| {
    Column::new()
      .width(360.0)
      .child(
        AudioPlayer::controlled(0.0, 95.0, |_| {})
          .source(&shown)
          .muted_controlled(true, |_| {})
          .title("Tone")
          .id("p"),
      )
      .into()
  });
  // Drawn, and drawn again: nothing is asked of the server, and its length is what the app said.
  for _ in 0..5 {
    stage.tree.tick_timers_at(std::time::Instant::now());
    stage.pass();
    std::thread::sleep(std::time::Duration::from_millis(40));
  }
  assert_eq!(asked.load(Ordering::Relaxed), 0, "a player that is only drawn loads nothing");
  assert!(stage.paints("1:35"), "the length the app gave");
  assert!(!lurq::media::sound_cache().holds(&source));

  // Play: it is loaded, its length is its own, it plays, and it is kept.
  stage.click("p-play");
  until(&mut stage, "its own length", |stage| stage.paints("0:08"));
  until(&mut stage, "playing", |stage| stage.paints("0:01"));
  assert_eq!(asked.load(Ordering::Relaxed), 1);
  assert!(lurq::media::sound_cache().holds(&source));

  // Another player of the same address, later: kept, it is opened at once and asks the server for nothing.
  let shown = source.clone();
  let mut second = Stage::new(move |_| {
    Column::new()
      .width(360.0)
      .child(
        AudioPlayer::controlled(0.0, 95.0, |_| {})
          .source(&shown)
          .muted_controlled(true, |_| {})
          .id("p"),
      )
      .into()
  });
  until(&mut second, "its own length", |stage| stage.paints("0:08"));
  assert_eq!(asked.load(Ordering::Relaxed), 1, "one request for two players");
}

#[cfg(all(any(windows, target_os = "macos"), feature = "media", feature = "http"))]
#[test]
fn a_sound_the_apps_folder_keeps_from_an_earlier_run_is_opened_as_it_is_drawn_and_asks_the_server_for_nothing() {
  use std::{
    io::{Read, Write},
    sync::atomic::{AtomicUsize, Ordering},
  };

  use lurq::{app::App, resources::Source};
  // A server on this machine that answers each request with the tone, and counts them.
  let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
  let address = format!("http://{}/tone.wav", listener.local_addr().unwrap());
  let asked = Arc::new(AtomicUsize::new(0));
  let counted = asked.clone();
  std::thread::spawn(move || {
    let body = tone(8.0);
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
      let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
      );
      let _ = stream.write_all(head.as_bytes());
      let _ = stream.write_all(&body);
    }
  });
  let folder = std::env::temp_dir().join(format!("lurq-audio-player-cache-{}", std::process::id()));
  let _ = std::fs::remove_dir_all(&folder);
  let source = Source::url(&address);
  // An app that gave a folder for its cache on the disk, with a player of the address.
  let run = |folder: &std::path::Path| {
    let shown = source.clone();
    Stage::in_app(App::new().enable_media_disk_cache(folder), move |_| {
      Column::new()
        .width(360.0)
        .child(
          AudioPlayer::controlled(0.0, 95.0, |_| {})
            .source(&shown)
            .muted_controlled(true, |_| {})
            .title("Tone")
            .id("p"),
        )
        .into()
    })
  };

  // The first run: drawn, it loads nothing; played, it loads once, and the sound is a file in the folder.
  let mut first = run(&folder);
  for _ in 0..3 {
    first.tree.tick_timers_at(std::time::Instant::now());
    first.pass();
  }
  assert_eq!(asked.load(Ordering::Relaxed), 0);
  assert!(first.paints("1:35"), "the length the app gave");
  first.click("p-play");
  until(&mut first, "its own length", |stage| stage.paints("0:08"));
  until(&mut first, "playing", |stage| stage.paints("0:01"));
  assert_eq!(asked.load(Ordering::Relaxed), 1);
  assert!(first.app.loader().media_disk_cache().holds(&source));
  assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 1);
  drop(first);

  // The next run of the app: the sound is kept, so the player opens it as it is drawn, with its own length,
  // and plays it, and the server is asked for nothing.
  let mut next = run(&folder);
  assert!(lurq::media::sound_cache().holds(&source));
  until(&mut next, "its own length", |stage| stage.paints("0:08"));
  next.click("p-play");
  until(&mut next, "playing", |stage| stage.paints("0:01"));
  assert_eq!(asked.load(Ordering::Relaxed), 1, "one request for two runs of the app");
  drop(next);
  let _ = std::fs::remove_dir_all(&folder);
}
