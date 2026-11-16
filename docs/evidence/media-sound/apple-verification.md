# A sound on macOS and iOS: what is to be checked

`lurq::media` plays a sound on macOS and iOS with the system's own player (`AVAudioPlayer`). That code,
`crates/lurq/src/media/apple.rs`, was written on Windows, on the branch `claude/ds-media`, by a session that
had no Mac and no phone. This is what it did and did not do, what it took on trust, and what to run and
listen for. An entry goes when it was tried; what it found goes under "What was found" at the end, and a gap
that stays goes to `docs/ios-gaps.md`.

## What was done, and what was not

- **Compiled, never linked and never run.** `cargo check -p lurq --features media` passes for
  `aarch64-apple-darwin` and `aarch64-apple-ios`, with the tests for the Mac, and
  `cargo check -p mobile_demo --target aarch64-apple-ios` passes. A check does not link: that the frameworks are
  found and the app starts is not known.
- **`http` was not compiled for these systems at all.** Its TLS (`ring`) needs Apple's own C compiler, which
  Windows does not have. Nothing in `lurq::media` differs with it, but the first build with `http` on a Mac
  is the first.
- **Everything below "By ear" is unknown.** Nobody has heard this code play.

The same media, players and tests run on Windows, where they were run and listened to: see
`design/handoff/media-playback.md`, "Checked". What is shared (the cache, the load, the player's binding,
`Media`'s clock and state) is tried there. What is new here is only how a sound is opened and played.

## What plays, and what does not

| | Windows | macOS, iOS |
| --- | --- | --- |
| `Media::sound` of a file, of bytes | plays | to be checked |
| `Media::sound` of an address (feature `http`) | loaded whole, kept, plays | to be checked |
| `Media::sound` of an address without `http`, or larger than the loader reads whole (64 MiB) | the system reads it as it plays | **an error, by design**: `AVAudioPlayer` reads no network |
| `Media::open` (a video, with its sound) | plays | **an error, by design**: "this build shows no video on this system yet" |
| A speed other than 1 | the pitch rises with it | the system keeps the pitch; it takes 0.5 to 2 |

## What the code takes on trust

Each of these is a guess a run settles. They are the first places to look when something is wrong.

1. **A player made and used on a thread that is not the main one, with no run loop.** The media's thread
   (`lurq-media`) makes the `AVAudioPlayer`, calls it, and lets it go. No delegate is set, so nothing waits for
   a callback. If the player needs the main thread or a run loop, it will not start or will not advance.
2. **The end is read, not told.** Every 20 ms while it plays, the thread asks `isPlaying`. When it says no:
   at the start (within 0.05 s) or within 0.3 s of the end it is taken for the end; anywhere else for a stop
   by the system (a call, another app's sound, headphones pulled out), and the media is paused there, not
   ended. A player that finished and did not go back to its start is still read as ended, by the second of
   the two conditions; one that stops somewhere else at its end would read as paused.
3. **The player tells what bytes are from the bytes** (`initWithData:`): no hint of the kind is given. Tried
   here with WAV, AAC in an MP4 file and MP3.
4. **`setEnableRate(true)` before `prepareToPlay`, and `setRate` again at each play.** A player is said to
   forget its speed when it stops at its end.
5. **The clock follows `currentTime`.** Where the media's clock and the player's are more than 30 ms apart
   the media's is set to the player's. If `currentTime` moves in large steps the seek bar will jump.
6. **A seek while it plays is `setCurrentTime`**, and the player goes on from there by itself.
7. **Volume and mute are `setVolume`**: muted is a volume of nothing, and the volume is kept to come back to.
8. **The audio session is not touched.** On iOS the app's default session plays, is silenced by the ring
   switch, and stops when the app goes to the background. An app that is a player sets its own session
   (`AVAudioSession`, the category for playback); `lurq` does not do it for it. Whether that should be an
   option of `lurq` is an open question to answer after hearing it.
9. **The framework is found.** The binding links `AVFAudio`, which is its own framework from macOS 11.3 and
   iOS 14.5. On an older system the app may not start.
10. **A sound the cache on the disk keeps is opened from its file** (`initWithContentsOfURL:`). The file is
    named by a key and its length, with the ending of its address where that is a sound's
    (`9f3a41c2d07be815-8a3f10.mp3`) and with none where the address has none. Nothing else tells the player
    what the file is. On Windows a file with no ending, and one with a wrong ending, open. Here it is a guess.
11. **The demo's folder on a phone is `$HOME/Library/Caches/lurq-mobile-demo/media`**: that `HOME` is the
    app's own container on iOS is taken on trust. Where the folder cannot be made, the log says so once
    ("the folder of the media disk cache ... could not be made"), nothing is written, and the sound is kept in
    memory as it was before the cache on the disk: the screen then works, and nothing is there after a restart.

The cache on the disk (`App::enable_media_disk_cache`, the gallery and the demo turn it on) was built after the
rest, on the branch `claude/media-disk-cache`, and is the same code on every system: the files, their names,
the budget and the forgetting were run on Windows (`design/handoff/media-playback.md`, "The cache on the
disk"). What is new on these systems is guesses 10 and 11, and that a file is deleted while it plays, which
these systems always allowed.

## macOS

### It builds

```sh
git fetch && git checkout claude/ds-media
cargo check -p lurq --features media,http
cargo build -p gallery
```

- [ ] Both end without an error. With one from `objc2-avf-audio` or `objc2-foundation`, the features asked of
      them in `crates/lurq/Cargo.toml` (the table for macOS and iOS) are the first thing to look at.

### The tests, which are silent

Every media in them is silenced before it plays.

```sh
cargo test -p lurq --features media,http --test media_tests
cargo test -p lurq --features media --lib media::
cargo test -p lurq --features form,router,mcp,raster,media,http --test ui_tests audio_player
```

- [ ] `media_tests`: nine run on a Mac (the tests of a video and of Windows' meters are not built there), and
      pass:
  - `a_sound_plays_from_a_file_and_from_bytes_to_its_end_and_again`: a WAV of 1.2 s from a file and from
    bytes takes about 1.2 s, ends, says so, and plays again from its start. **This is the test of guess 2.**
  - `a_song_in_an_mp4_file_plays_as_a_sound_at_any_speed_and_where_it_is_sent`: the gallery's track goes
    0.6 s in 0.6 s, a second in half of one at twice the speed, on from 10 s when sent there, stands when
    paused, and ends near its end. **Guesses 4, 5 and 6.**
  - `a_sound_at_an_address_is_loaded_once_and_kept`: one request of a server of the test's own for two
    plays; a third after it is forgotten.
  - `a_sound_that_is_not_at_its_address_says_so_and_one_the_app_refuses_is_not_asked_for`.
  - The five of the cache on the disk, each with a folder of its own under the temporary one:
    `with_a_folder_given_a_sound_at_an_address_is_a_file_there_and_the_next_run_asks_the_server_for_nothing`
    (its last part plays a file with no ending: **guess 10**),
    `forgetting_a_sound_deletes_its_file_and_it_is_loaded_again` (the file is deleted while it plays),
    `over_the_budget_of_the_folder_the_sound_played_longest_ago_is_deleted`,
    `a_file_of_the_folder_that_is_gone_cut_short_or_spoiled_is_loaded_again_and_not_played_broken` (its last
    part gives the player a file of nothing: it is to say it cannot play it, not to crash), and
    `with_no_folder_given_nothing_is_written_and_a_sound_is_kept_in_memory`.
- [ ] The four tests of the sounds' cache pass (`--lib media::`), and
      `cargo test -p lurq --features http --lib resources::`, with the eleven of the cache on the disk: they
      are the same on every system.
- [ ] `ui_tests audio_player`: among them `given_a_source_it_plays_the_sound_and_its_controls_move_it`,
      `a_sound_at_an_address_is_loaded_when_it_is_first_played_and_not_for_being_drawn` and
      `a_sound_the_apps_folder_keeps_from_an_earlier_run_is_opened_as_it_is_drawn_and_asks_the_server_for_nothing`
      pass.

A test that waits ten seconds and fails with "it never came to: ended" is guess 2; "it never came to: opened"
or "playing" is guess 1.

### By ear, in the gallery

```sh
cargo run -p gallery
```

Components, Media, **Audio Player**. The player of the **Playground** really plays once it is given a source:
in the panel **Props**, the select **source** (`None`, `Bytes`, `A file`, `An address`). Its volume starts at
0.6. Choosing another source stops what played.

**source: Bytes** (the gallery's own track, from the app's binary):

- [ ] The player shows `0:00` and `0:24` before anything is pressed: it opened, and its length is the track's.
- [ ] Play (the button, or the switch **playing** in the panel): plucked notes over a low tone are heard, and
      the time and the bar move.
- [ ] Pause: silence at once, and the time stands. Play: it goes on from there.
- [ ] A press in the middle of the bar: it is heard from there. A drag of the handle: it follows, and plays
      from where it is let go.
- [ ] The volume's slider: louder and quieter at once, not a moment later. All the way down: nothing.
- [ ] The volume's button, or the switch **muted**: silence, and the slider goes to nothing; again: as loud as
      it was.
- [ ] The speed, `1×`: at 1.5 and 2 it is faster and **the notes are as high as they were**; at 0.5 slower.
      (On Windows they are higher and lower. That difference is known.)
- [ ] Left to play to its end: it stops, the button is "play" again and the time is `0:24`. Play: from its
      start.
- [ ] **repeat: One** in the panel: at its end it starts over by itself.
- [ ] **variant**: `Compact`, `Voice` and `Mini` play the same sound, each with the controls it has.

**source: A file** (a field **path** comes under the select, with the same track in it, written to the temporary folder and read from the disk):

- [ ] Everything above holds for it.
- [ ] The path of a sound of your own typed in place of it (an `.mp3`, an `.m4a`, a `.wav`): it plays, and its length is its own. A path that is not there: an error line, "Not found".

**source: An address** (a field **address** comes under the select, with an MP3 of six minutes in it):

- [ ] Before anything is pressed the player shows `-:--`, and the line under the field says "loaded when it
      is first played, then kept": it has not loaded anything.
- [ ] Play: lines and a spinner while it loads (8.5 MB), then music, and its length is `6:12`. The line under
      the field says "kept · 8.5 MB of sounds", with **Forget** at its end.
- [ ] **source** to `Bytes` and back to `An address`: the player shows `6:12` at once, and plays at once.
- [ ] With the network off: it still plays (it is kept).
- [ ] **Forget**, then play, still with the network off: an error line in the player, with "Try again".
      Network on, "Try again": it loads and plays.
- [ ] While it plays, a key in the field **address**: it stops at the first key. An address that is not a
      sound, then play: an error line, no crash.

**The gallery closed and opened again** (it keeps what it loaded in `~/Library/Caches/lurq-gallery/media`):

- [ ] After the address was played once: that folder holds one file of 8.5 MB, named by a key and its length
      and ending in `.mp3`, and nothing ending in `.part`.
- [ ] The gallery quit and run again, the network off, **source** to `An address`: the player shows `6:12`
      before anything is pressed, and the line under the field says "kept". Play: music at once. (**Guess 10**,
      with an MP3.)
- [ ] **Forget** while it plays: the file is gone from the folder at once, and the music goes on to where it
      is paused.
- [ ] The file cut short by hand while the gallery is closed (`truncate -s 1000000 <file>`), the network on,
      the gallery run again: the line does not say "kept", and play loads it anew and plays all of it.

Around them:

- [ ] Headphones plugged in and pulled out while it plays, and another output chosen in the system: what
      happens. Nothing was written for this on these systems: on a Mac the player may go on by itself on the
      new output, or stop (then the media is paused where it was, by guess 2).
- [ ] The window closed while the player plays: the sound stops.
- [ ] The playground's player opened in a window of its own (the button at the end of the bar over the stage): it is shown there with no source, so that one sound is not heard twice.
- [ ] **Video Player**, the section with the clip: the player says it cannot play, with "this build shows no
      video on this system yet" under it. That is what it is meant to say for now.

## iOS

### The demo with the Sound screen

`crates/mobile_demo` has a screen for this, **Sound**, in the first tab's list. It is built with `media`
always, and with `http` when asked for, which the player of an address needs:

```sh
cd crates/mobile_demo/ios
# A simulator:
xcodebuild -project MobileDemo.xcodeproj -scheme MobileDemo -sdk iphonesimulator \
  -destination "id=<simulator id>" -derivedDataPath ../../../target/xcode CARGO_FEATURES=http build
xcrun simctl install <simulator id> ../../../target/xcode/Build/Products/Release-iphonesimulator/MobileDemo.app
xcrun simctl launch --console-pty <simulator id> <bundle id>
# A phone, plugged in and trusted:
xcodebuild -project MobileDemo.xcodeproj -scheme MobileDemo -sdk iphoneos \
  -destination "id=<device id>" -derivedDataPath ../../../target/xcode CARGO_FEATURES=http \
  BUNDLE_ID_PREFIX=<yours> DEVELOPMENT_TEAM=<yours> -allowProvisioningUpdates build
xcrun devicectl device install app --device <device id> \
  ../../../target/xcode/Build/Products/Release-iphoneos/MobileDemo.app
```

- [ ] It builds with `CARGO_FEATURES=http`. If only that fails (the TLS), build without it: everything but
      the last player is still there to check, and that player says the build loads no address.
- [ ] The app starts, and the first tab lists **Sound**. (If it does not start at all, guess 9.)

### The Sound screen, on a phone with its sound on

The card at the top says what the media says of itself, four times a second: where it is and how long, whether
it plays, its speed and volume, and its error.

- [ ] It reads `0.0 s of 24.0 s`, `not playing`, `speed 1.0×, volume 0.5, sound: yes`, `no error`. "an
      unknown length" or "waiting" that stays is guess 1 or 3.
- [ ] **Play**: the track is heard from the phone, and the first line counts up in step with it.
- [ ] **Pause**: silence, `not playing`, and the time stands. **Play**: on from there.
- [ ] **To 10 s** while it plays: heard from there, and the line goes on from 10. While paused: the line
      reads 10.0 and nothing is heard until Play.
- [ ] **Speed 2×**: twice as fast, as high as it was, and the line counts two seconds a second.
      **Speed 1×**: back.
- [ ] **Volume 0.2**, **Volume 1**: quieter, louder, at once. **Mute**, **Unmute**: silence, and back as loud
      as it was.
- [ ] **To 22 s**, Play: after two seconds it stops, and the lines read `24.0 s of 24.0 s`, `not playing,
      ended`. **Play**: from its start.
- [ ] The player under "of the same track read from a file": a play button, a bar, the time. Play, pause and
      the bar work as the buttons above do, and the two sounds are heard together.
- [ ] The last player: Play loads the address (a spinner), then plays six minutes of music, and the line over
      it says "Now it is kept; 8.5 MB of sounds are kept". Back to the list and into Sound again: it plays at
      once. **Forget the sound that is kept**: the line says "not kept".
- [ ] The app closed (swiped away) after the address was played, the phone's network off, the app opened
      again, **Sound**: the line over the last player says it is kept before anything is pressed, and Play
      plays at once. That is the cache on the disk, in `Library/Caches/lurq-mobile-demo/media` of the app's
      container (**guesses 10 and 11**). If the line says "not kept": the console has the warning of guess 11,
      or the container (Xcode, Devices and Simulators, the app, "Download Container") has no such folder.
- [ ] **Forget the sound that is kept**, the app closed and opened again: "not kept".

### What a phone does to a sound

- [ ] The ring switch on silent: nothing is heard, and the lines go on as if it played. Expected with the
      app's default session (guess 8); write down whether that is acceptable for a player.
- [ ] The app sent to the background while it plays, and brought back: what the lines say, and whether it
      plays on. Expected: the system stops it, and the media is paused where it was.
- [ ] A call, an alarm or Siri while it plays, then ended: `not playing` with the time where it was cut off,
      **not** `ended`. Play goes on from there. (Guess 2.)
- [ ] Headphones pulled out while it plays: the same, paused where it was.
- [ ] The phone locked while it plays.
- [ ] A simulator is enough for everything but the ring switch, the call and the headphones.

## What was found

Nothing yet. Write here, with the date, the system and its version: which entries held, which did not and
what happened in place of what is written, and the output of a test that failed.
