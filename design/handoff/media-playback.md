## Changed outside the component's own files

- **`lurq::media`** (`crates/lurq/src/media/`, feature `media`): new. A `Media` plays a video; see below.
- **`crates/lurq/Cargo.toml`**: the feature `media = ["raster", "windows/Win32_Media_MediaFoundation", "windows/Win32_System_Com", "windows/Win32_System_Com_StructuredStorage", "windows/Win32_System_Variant", "windows/Win32_UI_Shell"]`, and the test target `media_tests`.
- **`resources::Loader`**: `allows(address)` and `resource_path(resource)` are `pub(crate)`: a media reads an address and a resource by itself, and asks the loader what it allows and where the root is.
- **`crates/gallery/assets/media/sample.mp4`**: the gallery's own clip, 15 s of 640 by 360 at 30 frames a second with a tone, 477 KB, made with `ffmpeg -f lavfi -i testsrc2=size=640x360:rate=30:duration=15 -f lavfi -i sine=frequency=440:duration=15 -c:v libx264 -profile:v main -pix_fmt yuv420p -preset veryslow -crf 36 -g 30 -c:a aac -b:a 48k -movflags +faststart`. It is in the gallery's binary (`include_bytes!`) and is what the tests decode.
- **The gallery** enables `media` outside a browser.

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

## For `CHANGELOG.md`

- `lurq::media` (feature `media`): `Media::open(loader, source)` plays a video with the system's decoder on a thread of its own, keeps its clock, and gives an image a `Video` draws; `play`, `pause`, `seek`, `set_rate`, `state`, `on_change`. On Windows it decodes with Media Foundation. No sound yet.
- `ui::VideoPlayer::source(&Source)` (feature `media`): a mounted player plays the source, and its controls move the video.

## Decided

- **Two stages, and this is the first** (told to Ivan, 2026-11-16). Stage 1, here: a video really plays, with a frame copied through memory once. Stage 2: the frame stays on the graphics card. Ivan's rule is that a frame never goes through the CPU's memory (one copy from GPU to GPU is fine); stage 1 breaks it on purpose, to have the decoder, the clock and the player's binding working and tested first. Nothing above the frame's transport changes in stage 2: `Media::frame()` gives another kind of `ImageData`.
- **Why the frame goes through memory now.** The gallery draws with `wgpu` (`WgpuRenderEngine`), and on Windows the `wgpu` renderer draws NV12 only from bytes in memory (`ImageData::streaming_nv12`) or from a macOS `CVPixelBuffer`; a D3D12 NV12 image (`Dx12Nv12Image`) is drawn by the `dx12` renderer alone and is passed over under `wgpu`. So stage 2 is: decode with a D3D11 device manager (hardware decoding stays on the card), copy the frame once into a shared NV12 texture, open that texture on the device `wgpu` runs on (`wgpu-hal`'s `dx12` is not a dependency on Windows yet), and draw it with the NV12 pipeline that is there, as the macOS path does with its pixel buffer. The `dx12` renderer already takes such a texture through `Dx12VideoSurfaceAllocator::open_shared_nv12_surface`.
- **The media is the truth of a player that has a source.** Its time, its length and whether it plays are read from the media at each render; the app's values for them are not drawn. What a person presses is done to the media by the player itself, after the app's handlers heard of it, so a player given a source and nothing else is whole. What the app says of `playing` and of `rate` moves the media when it changes from one render to the next. An app that holds `playing` and refuses a press (`playing_controlled` whose handler does nothing) still has the media play: the press is the person's.
- **A media has a thread of its own, and a clock of its own.** The thread decodes one frame ahead and shows it when the clock comes to its time; a frame more than 0.2 s late is passed over. The clock is where it was and when, going at its speed while it plays: `state().position` is read from it, with no frame needed. Pause and seek change the state at once, so a seek bar being dragged does not see the time go back; the thread then goes to the last place it was sent to, not to each.
- **A new frame draws the window and renders nothing.** The image is one that asks for continuous redraw (`ImageData::streaming_nv12`), which the engine already has for video. The player itself is rendered four times a second while it plays, for its time and its seek bar, and when the media says it changed.
- **Seeking is to the frame.** The reader goes to the key frame before the time, and the frames from there to the time are decoded and not shown.
- **Fullscreen and picture in picture draw the same media.** The player in the small window is given the picture and no source, so nothing is opened twice.

## Not done

- **Sound.** The clip's sound is not decoded or played; the volume and the mute are told and drawn and change nothing. It needs the audio stream decoded to PCM, an output (WASAPI), and the sound's clock as the media's.
- **The frame on the graphics card** (stage 2, above).
- **Other systems.** Only Windows has a decoder. Elsewhere `Media::open` says so in the state's error, and the player shows it as its error. macOS has the pixel-buffer path in the renderer and needs AVFoundation or VideoToolbox behind `Media`; a browser needs a `<video>` element or WebCodecs.
- **An address through the app's loader.** An address is opened by Media Foundation itself, after the loader's `allow` said yes: its headers (`Source::header`) and the loader's timeout are not used, and `MediaState::waiting` is true only while it opens, not while it waits for more of the file. `Loader::open` has no reader of an address yet; when it has, a media reads through it (an `IMFByteStream` over a `Reader`).
- **`position` held by the app.** A source's time is the media's; an app that sets its own `position` signal does not seek by that. It seeks with `Media::seek`, or the person does with the bar.
- **Color.** The renderer's NV12 shader is BT.709 limited range for every video.
- **Tracks.** The first video stream is played; there is no choice of audio or subtitle tracks from the file.

## Checked

- `cargo test -p lurq --features media --test media_tests`: a file opens with its size, its length and its first frame; it plays by its clock at 30 frames a second, pauses, plays twice as fast; it goes where it is sent and ends at its end, and plays again from its start; bytes play as a file does; what is no video, a missing file and an address the app does not allow say why.
- `tests/ui/video_player.rs`, `given_a_source_it_plays_the_file_and_its_controls_move_the_video` (Windows, feature `media`).
- In a running gallery on Windows (the `wgpu` renderer): the clip's first frame, its length, play, the time and the frames going on, pause, and a press on the seek bar.
