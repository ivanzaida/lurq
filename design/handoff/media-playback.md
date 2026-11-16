## Changed outside the component's own files

- **`lurq::media`** (`crates/lurq/src/media/`, feature `media`): new. A `Media` plays a video; see below.
- **`crates/lurq/Cargo.toml`**: the feature `media = ["raster", "windows/Win32_Media_Audio", "windows/Win32_Media_MediaFoundation", "windows/Win32_System_Com", "windows/Win32_System_Com_StructuredStorage", "windows/Win32_System_Variant", "windows/Win32_UI_Shell"]`, and the test target `media_tests`. The `windows` dev-dependency has `Win32_Media_Audio`, `Win32_Media_Audio_Endpoints` and `Win32_System_Com` for the test that listens.
- **`lurq::media`** has three files more: `output.rs` (the system's output on Windows), `cache.rs` (`SoundCache`, `sound_cache()`), and the Apple backend (see "Other systems" below).
- **`resources::Loader`**: `allows(address)` and `resource_path(resource)` are `pub(crate)`: a media reads an address and a resource by itself, and asks the loader what it allows and where the root is.
- **`crates/gallery/assets/media/sample.mp4`**: the gallery's own clip, 15 s of 640 by 360 at 30 frames a second with a tone, 477 KB, made with `ffmpeg -f lavfi -i testsrc2=size=640x360:rate=30:duration=15 -f lavfi -i sine=frequency=440:duration=15 -c:v libx264 -profile:v main -pix_fmt yuv420p -preset veryslow -crf 36 -g 30 -c:a aac -b:a 48k -movflags +faststart`. It is in the gallery's binary (`include_bytes!`) and is what the tests decode.
- **`crates/gallery/assets/media/track.m4a`**: the gallery's own track, 24 s of plucked notes over four chords, AAC at 64 kbit/s, 199 KB, made of sines by `python scripts/make-gallery-track.py` (it writes a WAV and has `ffmpeg` encode it). It is in the gallery's binary, and the Audio Player page plays it from those bytes and from a file it writes to the temporary folder.
- **The gallery** enables `media` and `http` outside a browser: `http` for the sound at an address.

## For `docs/src/content/docs/design-system.md`

## Playing a video

`VideoPlayer::source(&Source)` (feature `media`) makes a mounted player play what the source holds: a file, bytes, a resource of the app, an address.

```rust
VideoPlayer::new()
  .source(&Source::file("clips/lake.mp4"))
  .width(640.0)
  .max_width(Length::Fill)
  .mount(ctx)
```

The player opens the source with the app's loader, decodes it with the system's decoder and draws its frames as its picture. Where it is, how long it is, and whether it plays, waits, ended or failed are the media's, in place of what the app would say. A press on a control tells the app's handlers and does what it says to the media: play, pause, the seek bar, the speed, the keys. An app that holds `playing` or `rate` (a signal, or a value it sets) moves the media by changing them. With no source the player is drawn from the values it is given, as before.

An app that draws its own player asks for a media itself:

```rust
let media = Media::open(app.loader(), &Source::file("clips/lake.mp4"));
media.play();
let picture = media.frame().map(Video::new); // drawn again by itself as frames come
let state = media.state(); // duration, position, size, playing, waiting, ended, error, rate
media.seek(42.0);
media.set_rate(1.5);
media.on_change(|| { /* it opened, ended, failed: from the media's thread */ });
```

## Playing a sound

`AudioPlayer::source(&Source)` (feature `media`) makes a player play what the source holds: a file, bytes, a resource of the app, an address.

```rust
AudioPlayer::new(position, 0.0)
  .source(&Source::url("https://example.com/night-drive.mp3"))
  .title("Night Drive")
  .volume(volume)
```

The player opens the source as a sound and plays it through the system's output. Where it is, how long it is, and whether it plays, loads or failed are the sound's, in place of what the app would say: the length given to `new` is drawn only until it opened. A press on a control tells the app's handlers and does what it says to the sound: play, pause, the seek bar, the volume, the mute, the speed. An app that holds `playing`, `volume`, `muted` or `rate` moves the sound by changing them. With `AudioRepeat::One` the track starts over at its end. "Try again" loads the source anew.

A sound at an address is loaded whole through the app's loader (its headers, its timeout, what it allows; the feature `http`), and kept: played again, by this player or another, it starts at once and asks the network for nothing. While it loads the player is drawn loading. What is kept is what the server sent, up to 64 MiB of sounds; over that, what was played longest ago is let go.

```rust
sound_cache().budget(256 * 1024 * 1024); // keep more
sound_cache().forget(&source);           // this one has changed: load it again
sound_cache().clear();
```

An app that draws its own player asks for the sound itself:

```rust
let song = Media::sound(app.loader(), &Source::file("songs/night-drive.mp3"));
song.set_volume(0.8);
song.play();
song.set_muted(true);
let state = song.state(); // duration, position, sound, volume, muted, playing, waiting, ended, error, rate
```

A video has its sound too: `Media::open` and `VideoPlayer::source` play it, and the picture goes by what is heard.

## For `CHANGELOG.md`

- `lurq::media` (feature `media`): `Media::open(loader, source)` plays a video with the system's decoder on a thread of its own, keeps its clock, and gives an image a `Video` draws; `play`, `pause`, `seek`, `set_rate`, `state`, `on_change`. On Windows it decodes with Media Foundation.
- `lurq::media`: a media plays its sound through the system's output (WASAPI on Windows), and its clock follows what is heard; `Media::set_volume`, `Media::set_muted`, and `sound`, `volume`, `muted` in `MediaState`.
- `lurq::media`: `Media::sound(loader, source)` is what is only heard. At an address it is loaded whole through the app's loader and kept in `media::sound_cache()` (`SoundCache`: `budget`, `bytes`, `len`, `holds`, `forget`, `clear`; `DEFAULT_SOUND_CACHE_BYTES`).
- `ui::VideoPlayer::source(&Source)` (feature `media`): a mounted player plays the source, and its controls move the video, its volume and its mute.
- `ui::AudioPlayer::source(&Source)` (feature `media`): a player plays the source as a sound, and its controls move it.

## Decided

- **Two stages, and this is the first** (told to Ivan, 2026-11-16). Stage 1, here: a video really plays, with a frame copied through memory once. Stage 2: the frame stays on the graphics card. Ivan's rule is that a frame never goes through the CPU's memory (one copy from GPU to GPU is fine); stage 1 breaks it on purpose, to have the decoder, the clock and the player's binding working and tested first. Nothing above the frame's transport changes in stage 2: `Media::frame()` gives another kind of `ImageData`.
- **Why the frame goes through memory now.** The gallery draws with `wgpu` (`WgpuRenderEngine`), and on Windows the `wgpu` renderer draws NV12 only from bytes in memory (`ImageData::streaming_nv12`) or from a macOS `CVPixelBuffer`; a D3D12 NV12 image (`Dx12Nv12Image`) is drawn by the `dx12` renderer alone and is passed over under `wgpu`. So stage 2 is: decode with a D3D11 device manager (hardware decoding stays on the card), copy the frame once into a shared NV12 texture, open that texture on the device `wgpu` runs on (`wgpu-hal`'s `dx12` is not a dependency on Windows yet), and draw it with the NV12 pipeline that is there, as the macOS path does with its pixel buffer. The `dx12` renderer already takes such a texture through `Dx12VideoSurfaceAllocator::open_shared_nv12_surface`.
- **The media is the truth of a player that has a source.** Its time, its length and whether it plays are read from the media at each render; the app's values for them are not drawn. What a person presses is done to the media by the player itself, after the app's handlers heard of it, so a player given a source and nothing else is whole. What the app says of `playing` and of `rate` moves the media when it changes from one render to the next. An app that holds `playing` and refuses a press (`playing_controlled` whose handler does nothing) still has the media play: the press is the person's.
- **A media has a thread of its own, and a clock of its own.** The thread decodes one frame ahead and shows it when the clock comes to its time; a frame more than 0.2 s late is passed over. The clock is where it was and when, going at its speed while it plays: `state().position` is read from it, with no frame needed. Pause and seek change the state at once, so a seek bar being dragged does not see the time go back; the thread then goes to the last place it was sent to, not to each.
- **A new frame draws the window and renders nothing.** The image is one that asks for continuous redraw (`ImageData::streaming_nv12`), which the engine already has for video. The player itself is rendered four times a second while it plays, for its time and its seek bar, and when the media says it changed.
- **Seeking is to the frame.** The reader goes to the key frame before the time, and the frames from there to the time are decoded and not shown.
- **Fullscreen and picture in picture draw the same media.** The player in the small window is given the picture and no source, so nothing is opened twice.
- **Sound** (asked for by Ivan, 2026-11-16: "sound first, with a cache, loaded from bytes, from a local file and from a link"). On Windows the reader's first audio stream is decoded to plain samples (PCM, or float where the stream is float), brought to two channels, and given to the default output in shared mode. The media's thread does both the picture and the sound: it keeps a quarter of a second of sound decoded ahead and the device a fifth of a second.
- **The clock is the sound's.** A media that has sound and somewhere to play it sets its clock to what the device says was heard, where the two are more than 30 ms apart; nearer than that the clock goes on by itself, which is smoother than the device's steps. So the picture follows the sound, a pause stops both at once, and a media with no sound, or with no device, goes by the clock it had.
- **The sound starts with the picture it was sent to.** After a seek the output is given nothing until the frame at that time is shown.
- **The volume and the mute are of the media's own stream** (`IAudioStreamVolume`), not of the samples: they are heard at once, and not a fifth of a second later. They are linear, as a browser's are. Muted, the volume is kept.
- **A speed plays the sound that much faster, and higher.** The output is opened anew with the samples said to be that many times as many in a second, and the system brings them to the device's rate. What the old output held and had not played is given to the new one, so nothing is skipped. The pitch is not kept: that needs the samples stretched in time, which is not built.
- **A device that goes away** (unplugged, or another one made the default) is answered by opening the output again on the one the system has now; with none, the media goes on unheard.
- **What is only heard is opened as a sound** (`Media::sound`): its picture is not decoded, and an address is loaded whole and kept. A video at an address is still read by the system as it plays, so that it starts before it is all there; a sound is small enough to load first, and loading it is what makes it possible to keep.
- **The cache keeps what the server sent, in memory, for the process** (`sound_cache()`), by the source's `cache_key`, so `Source::key` names a sound whose address changes. The loader keeps no bytes by its own rule ("what is decoded from them is kept by who decodes"); a sound is decoded as it plays, so what it keeps is the encoded bytes. 64 MiB unless told, the one played longest ago let go first, one larger than the whole budget not kept. Nothing is written to the disk: a sound is loaded again by the next run of the app. A file and bytes are not kept: they are where they are.
- **Where the loader cannot give the bytes, the system reads the address itself**: larger than `Loader::max_bytes`, or a build without `http`. Nothing is kept then. Every other failure of the load is the media's error, in the loader's words.
- **Bytes that came by a name are opened with it** (`MF_BYTESTREAM_ORIGIN_NAME`), so that the system's reader for `.mp3` is asked for what ends in `.mp3`; bytes with no name are told from themselves.
- **A sound at an address is loaded when it is first played, not when its player is drawn.** A list of a hundred players of a hundred addresses loads nothing until one is played; until then a player is drawn from the length the app gave it. A press of play opens it, and it plays once it is loaded. A file, bytes and a sound that is kept already are opened as soon as the player is in a tree, so their length is shown.
- **The clock is not set to the sound while a seek is on its way.** `Media::seek` moves the state at once and tells the thread; between the two, what is heard is still where it was, and setting the clock by it would send a seek bar back for a moment. The media counts the times it was sent, the thread the ones it has done.
- **`AudioPlayer::source` is `VideoPlayer::source`'s way**: the media is the truth of where it is, how long, and whether it plays; the app's values for those are not drawn. `loading` is the media's `waiting`, the error line is the media's reason. `AudioRepeat::One` starts the track over, because that is of one sound; `All` and shuffle are of a queue, and the app's.

## Not done

- **A speed that keeps the pitch.** Faster is higher, slower is lower.
- **A cache on the disk.** What is kept is kept for the run of the app.
- **What a sound says of itself**: its title, who it is by and its artwork from the file's tags are not read.
- **A queue that goes on by itself.** There is no `on_ended` to tell an app that a track ended: it reads `Media::state().ended`, or the player's `playing` turning false at the end. `AudioQueue` and `AudioPlayer` are not tied together.
- **More than two channels** are brought to two simply: the first two, with the third on both. The layouts of five and more deserve their own weights.
- **Which device.** The system's default output, always.
- **The frame on the graphics card** (stage 2, above).
- **Other systems.** See "Other systems" at the end.
- **An address through the app's loader.** An address is opened by Media Foundation itself, after the loader's `allow` said yes: its headers (`Source::header`) and the loader's timeout are not used, and `MediaState::waiting` is true only while it opens, not while it waits for more of the file. `Loader::open` has no reader of an address yet; when it has, a media reads through it (an `IMFByteStream` over a `Reader`).
- **`position` held by the app.** A source's time is the media's; an app that sets its own `position` signal does not seek by that. It seeks with `Media::seek`, or the person does with the bar.
- **Color.** The renderer's NV12 shader is BT.709 limited range for every video.
- **Tracks.** The first video stream is played; there is no choice of audio or subtitle tracks from the file.

## Other systems

- **macOS and iOS play a sound, and no video.** `crates/lurq/src/media/apple.rs`: `Media::sound` of a file or of bytes is the system's own `AVAudioPlayer`, on the media's thread, with the media's clock set to the player's `currentTime`. An address is a sound once the app's loader has loaded it (the shared path, with the cache); one the loader could not give (no `http`, or over 64 MiB) is an error there, since that player reads no network. `Media::open` says "this build shows no video on this system yet". The speed is the player's own, which keeps the pitch and takes 0.5 to 2. The end is read from `isPlaying` turning false at the start or the end of the sound; turned false anywhere else, the system stopped it, and the media is paused there. The audio session of an iOS app is not touched.
- **That code was compiled and never run.** `cargo check` for `aarch64-apple-darwin` and `aarch64-apple-ios` passes; nothing was linked or heard, and `http` was not compiled for them (its TLS needs Apple's compiler). **`docs/evidence/media-sound/apple-verification.md` is what to run and listen for on a Mac and on a phone**, with the nine things the code takes on trust. Ivan asked for that document on 2026-11-16.
- **New dependencies, on macOS and iOS alone**: `objc2-avf-audio` 0.3 (optional, with `media`), and `objc2-foundation` gains `NSData`, `NSError` and `NSURL` on macOS and is new on iOS. `Cargo.lock` has them.
- **`crates/mobile_demo`** has `media`, a feature `http` of its own (`CARGO_FEATURES=http` to Xcode's build), and a screen **Sound** (`src/sound.rs`): a media driven by buttons with its state in plain lines, and two of the design system's players, of a file and of an address.
- **`tests/media_tests.rs` and the audio player's tests of a source are built on macOS too**; the tests of a video, and the one that reads Windows' meters, are Windows'.
- **Other than these**, `Media::open` and `Media::sound` say the build has no decoder for the system. A video on macOS has the pixel-buffer path in the renderer and needs AVFoundation or VideoToolbox behind `Media`; a browser needs an `<audio>` or `<video>` element, or WebCodecs.

## Checked

- `cargo test -p lurq --features media,http --test media_tests` (9 tests, all silenced): a file opens with its size, its length and its first frame; it plays by its clock at 30 frames a second, pauses, plays twice as fast; it goes where it is sent and ends at its end, and plays again from its start; bytes play as a file does; what is no video, a missing file and an address the app does not allow say why. The clip has its sound, and as a sound it has no picture, goes by what is heard at 1× and 2×, where it is sent, and stands when paused. A WAV plays from a file and from bytes to its end and again. A sound at an address (a server of the test's own) is loaded once for two plays, kept, and loaded again once forgotten; a 404 is "it was not found"; an address the app refuses is not asked for.
- `cargo test -p lurq --features media --test media_tests -- --ignored --test-threads 1 --nocapture` (**it is heard**: a quiet tone for two seconds, and nothing else may play meanwhile). It reads the system's meters. Measured on Ivan's machine, 2026-11-16: out of the device 0.031 at a volume of 0.1, 0.062 at 0.2, 0 muted, 0.062 heard again, 0 paused; the process gives the output the tone only while it plays.
- `cargo test -p lurq --features media --lib media::`: the cache lets go of what was used longest ago, keeps nothing larger than its budget, and a smaller budget lets go at once.
- `tests/ui/video_player.rs`, `given_a_source_it_plays_the_file_and_its_controls_move_the_video`, and `tests/ui/audio_player.rs`, `given_a_source_it_plays_the_sound_and_its_controls_move_it` (Windows, feature `media`): the length is the sound's, play, the bar, pause, and what is no sound shows "Try again". `a_sound_at_an_address_is_loaded_when_it_is_first_played_and_not_for_being_drawn` (with `http`): drawn five times, a player asks its server for nothing and shows the app's length; played, it loads once, shows its own length and is kept; a second player of the address opens at once with no second request.
- `a_song_in_an_mp4_file_plays_as_a_sound_at_any_speed_and_where_it_is_sent`: the gallery's track, AAC.
- In a running gallery on Windows, 2026-11-16: the Audio Player page's "Plays a source". The players of the bytes and of the file show 0:24; the player of the address shows no length until it is played, then loads `https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3` in about a second, shows 6:12, plays (0:09 nine seconds later), and the line under it reads "this one is kept · 8.5 MB of sounds kept".
- In a running gallery on Windows (the `wgpu` renderer): the clip's first frame, its length, play, the time and the frames going on, pause, and a press on the seek bar.
