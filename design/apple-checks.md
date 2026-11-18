# What to check on macOS and iOS before the design system is closed

Everything below was written on Windows. For Apple's systems it was at most `cargo check`ed: nothing was linked,
run, looked at or heard. Go through it on a Mac and an iPhone (an iPad with a trackpad and an Apple Pencil where a
line asks for one), top to bottom; each line says what to run and what must happen. What fails is fixed, not
written down; what passes gets a tick and the date.

Older notes this gathers: the plan's [13.3](design-system-plan.md#133-to-check-on-macos-and-ios), the media's
[apple-verification.md](../docs/evidence/media-sound/apple-verification.md), the handoffs in
[handoff/](handoff/), and the decisions in [decisions.md](decisions.md) and
[mobile-decisions.md](mobile-decisions.md).

Feature names are those after the feature cleanup of 2026-11-17 (`default` is the desktop set; `svg` is in
`canvas`). If that branch is not merged yet, add `form,router,canvas,image` by hand.

## 1. It builds, links and starts

- [x] **The workspace on a Mac.** `cargo build -p lurq`, `cargo build -p gallery --features devtools,mcp`,
  `cargo build -p lurq --example lifecycle --features mcp`. Must link (no missing framework) and start.
  2026-11-17: all three link; the gallery starts and draws.
- [x] **The test matrix on a Mac.** `bash target/suites2.sh` from the design-system worktree (the runs A–G of the
  feature cleanup), or by hand: every suite on `default` + `markdown,mcp`, `--no-default-features` check, a
  minimal app's tests (`--no-default-features --features winit,wgpu --tests`), `devtools` and `perf_profile`;
  and the checks of a video, which the harness cannot run on a Mac (`--features media --test media_video_apple`).
  All green, the same counts as on Windows give or take what is Windows-only.
  2026-11-17: green, `native_macos` with the Mac unlocked (behind the login window its window is never key).
  What failed was fixed: the tests pressed Ctrl for the editing commands,
  which are Cmd on Apple (`support::shortcut`, `Stage::shortcut`, `redo`); some counted on Segoe UI's or
  Consolas's widths; the first layout of an app resolves its theme once, which the allocation tests now leave
  out; and two were lurq's own (Markdown's gap beside a key, the sound's clock at a pause and its play again).
- [x] **`http` on Apple.** Never compiled there (its TLS, `ring`, needs Apple's C compiler):
  `cargo build -p lurq --features http` on the Mac, and for `aarch64-apple-ios`. Link Preview, a picture at an
  address and a sound at an address depend on it. 2026-11-17: builds for both; those pages are not looked at yet.
- [x] **The phone app.** Build `crates/mobile_demo` for the simulator and a device from Xcode
  (`crates/mobile_demo/ios/MobileDemo.xcodeproj`; Xcode runs `ios/build-rust.sh`). It starts and draws.
  2026-11-17: builds for both; starts and draws on the iPhone 17 simulator and on Ivan's iPhone.
- [ ] **The gallery on iOS** (`crates/gallery/ios/Gallery.xcodeproj`, built as the phone app is, its bundle
  `<BUNDLE_ID_PREFIX>.lurq.gallery`): Ivan's tabs, the pages at a phone's width, the pointer toggle.
  2026-11-17: made; runs on the iPhone 17 simulator and on Ivan's iPhone at a phone's width, its header under the
  status bar. Ivan went through pages on the phone; what he found is fixed (2026-11-17/18, each with its test):
  the sidebar over the page keeps where its items were scrolled, opens with a finger from the left edge, and
  keeps clear of the status bar and the home indicator (a `ui::Sheet` at a side pads by the safe area); a video
  in full screen keeps clear of them and is left by a drag down; the colour picker's area and rails and the
  angle slider are dragged from where a finger lands, in a page that scrolls (`Node::finger_drags`); a date and a
  time picker open at a tap of the field, with no keyboard on the screen (`KeyboardKind::None`); a finger that
  taps while another is held clicks what it taps (three toasts held open, one closed by another finger; see
  [mobile-decisions.md](mobile-decisions.md)). Still open: the on-screen keyboard stays up when a field loses
  the focus; a video at an address lagged the app (not reproduced on a Mac); the pages not yet gone through.

## 2. Drawing on Metal

- [x] **Borders and rings.** `quad.wgsl` was rewritten for strokes and compared with its formulas on Direct3D only.
  Gallery, pages Button, Card, Input, Tabs: borders of 1 and 2, rounded corners, a focus ring growing in (Tab
  into a button), a dashed edge where a page shows one. They look as on Windows.
  2026-11-18, on a Mac through MCP's screenshots: the borders of Button, Card, Input, Tabs and the Borders page
  (one side fading along its corners, dashed, a dashed pill, widths 1, 2 and 3) are even and round; Tab draws the
  ring round a button, apart from its border. The ring growing in is quicker than MCP can see. `quad_tests` pass
  on Metal (`cargo test -p lurq --lib quad_tests -- --ignored`). Not set beside Windows.
- [ ] **A lost device** (`edab8590`). Hard to cause on purpose; if a Mac sleeps with the gallery open and wakes, the
  canvases (charts, Signature Pad, QR Code) are drawn again and a video goes on. At least: nothing goes black.
- [x] **The canvas on the GPU.** Pages Bar Chart, Line Chart, Signature Pad (draw, Undo, Clear, Make a picture), QR
  Code (the code is scanned by a phone's camera), Watermark (on the long scrolled page).
  2026-11-18: Ivan: the Signature Pad works on a Mac.
  2026-11-18, through MCP: Bar Chart and Line Chart draw; the QR Code is read back as its text in the light look and
  the dark (by the system's `CIDetector`, from the screenshot); the Watermark lies over a card, over a picture, and
  over every line of the long page as it scrolls.
- [ ] **Icon hinting on a 1x display**, if one is at hand: `cargo run -p lurq --example icon_hinting --features
  winit,wgpu,screenshot`. On a 2x display hinting matters little.
- [x] **Blur and shadows.** Pages with a backdrop blur (a sheet, a popover over content) and shadows (Card, Menu).
  2026-11-18, the Visual page through MCP: the theme's three shadows, a layered one, a hard offset, a glow and an
  inset well; a layer blurred by growing radii and faded; frosted glass that blurs the stripes behind it more as its
  radius grows, and turns its blur with it. A sheet dims the page under it and does not blur it.

## 3. The system and the windows

- [ ] **Reduce motion, macOS.** `cargo test -p lurq --lib motion`: `the_platform_has_the_setting` passes. Then
  System Settings › Accessibility › Display › Reduce motion on, click back into the gallery: a transition (a
  sheet, a dialog) ends at once; off again, it moves again.
  2026-11-18: `the_platform_has_the_setting` passes on a Mac; the setting turned on live is not looked at.
- [ ] **Reduce motion, iOS.** Settings › Accessibility › Motion › Reduce Motion, back to `mobile_demo`: a sheet and a
  stack push switch without moving. It links on the simulator and on a device.
- [ ] **Light and dark** follow the system, and change live when the system's appearance changes (macOS: System
  Settings › Appearance; iOS: Control Centre).
  2026-11-18, macOS: the gallery on its theme System went dark and light again as the system's appearance was
  changed under it, with no click (`appearance preferences` of System Events). iOS is not looked at.
- [ ] **The window's chrome on a Mac** (`ui::WindowChrome`): the traffic lights where macOS puts them and working,
  the title bar dragging the window, double-click zooming it, the gallery's tabs in the title bar (open, close,
  reorder, a new tab with Cmd+T), full screen and back.
  2026-11-18: the lights are the system's own now, moved to where the bar's controls are
  (`winit_shell/macos_controls.rs`): the menu of tilings under the pointer on the green one, full screen at its
  press, the window's corners rounded by the system. Leaving full screen by the system's light or keys shows the
  bar and its lights again (whether the window is in full screen is read every turn). In full screen the bar
  stays with its tabs, a video and a sheet over it. Ivan saw the lights and full screen and back; the bar in full
  screen, the tabs, the drag and the double click are still to be looked at.
- [ ] **Picture in picture.** Video Player page: picture in picture opens a small window that stays over other
  apps' windows (`always_on_top`), and closes back.
- [ ] **Opening an address** (`open_url`). Link Preview page and a `ui::Link` with `href`: macOS opens the default
  browser (`NSWorkspace openURL:`), iOS opens Safari (`UIApplication openURL:`). A `file:` address is refused.
- [ ] **File dialogs** (macOS): Path Field page — Browse… picks a folder, Save as… saves a file with the filter and
  the name; Cancel gives nothing. iOS has no dialog: the button says so or does nothing, and nothing crashes.
- [ ] **The clipboard.** Copy from a Markdown page (rich text pastes as Markdown into a text editor, plain into a
  field), a picture copied and pasted, Signature Pad's PNG. Paste into a lurq field from another app.
  2026-11-18: text selected in the Markdown document and copied is plain text on the system's clipboard; lurq
  writes no Markdown or HTML there on any system (`clipboard.rs` has text, a picture and files). Pasting from
  another app needs a person: MCP does not read the clipboard.
- [ ] **Fonts and text.** The system fonts look right (San Francisco, the mono one in code); the caret blinks and
  stops while typing; selection by drag and by double-click; IME: Ukrainian, then Japanese with its candidate
  window in the right place.
  2026-11-18, through MCP: San Francisco and the mono font in code; a drag selects across a heading and a
  paragraph, a double click selects a word. The caret and IME need a person.

## 4. The keyboard (the editing commands of 2026-11-17)

- [ ] **Cmd, not Ctrl.** In a text field and on the Signature Pad: Cmd+Z undoes, Cmd+Shift+Z redoes, Cmd+Y does
  **not** redo; Cmd+C, Cmd+X, Cmd+V, Cmd+A. In a Ukrainian layout the same keys by their place.
  2026-11-18, keys of MCP into an Input: Cmd+Z undoes, Cmd+Shift+Z redoes, Cmd+Y and Ctrl+Z do nothing, Cmd+A,
  Cmd+C and Cmd+X go through the system's clipboard (`pbpaste` reads what was copied). Cmd+V is not tried: MCP
  refuses a paste of the person's clipboard. Undo takes back one character at a time, on every system: typing is
  not grouped into one step. The keys of a Ukrainian layout need a keyboard.
- [x] **A native Edit menu.** If the app has a menu bar with Undo/Redo/Copy/Paste items with their accelerators,
  the menu takes Cmd+Z before the window: each item must run `window.edit(EditCommand::..)` and do the same thing
  the key does. The gallery's menu, if it has one, as the example.
  2026-11-18: the gallery has no menu of its own; lurq puts an Edit menu in every app's bar now (decisions.md, "On
  macOS every app has an Edit menu"), and the bar shows "gallery" and "Edit". Ivan tried its items and their keys:
  they work.
- [ ] **Option + a letter** types the letter's character and is not a command (Option is Alt).
- [ ] **Tab and the focus** go through the controls as on Windows; Full Keyboard Access on macOS where it changes
  what Tab reaches.
  2026-11-18, through MCP: Tab goes from button to button and Shift+Tab back. Full Keyboard Access not tried.

## 5. A finger, a pencil, a trackpad (iOS, the pointer of 2026-11-17)

- [ ] **The pointer starts `Coarse`.** On a phone, Lightbox, Picture and Video Player are in their phone form from
  the first frame, with no setting.
- [ ] **An iPad with a trackpad or a mouse** goes `Fine` when the trackpad moves, and back to `Coarse` at the next
  touch; an open lightbox keeps its form until it closes.
- [ ] **A tap is a click, once.** Buttons, list rows, links: one tap, one action. A tap of two fingers clicks
  nothing. A double tap on an element that listens for one (`on_double_tap`) holds the single tap back; elsewhere a
  tap is never delayed.
- [ ] **Gestures**: pinch and rotate (Zoom, a picture in the lightbox), swipe (a lightbox closes when dragged down;
  a carousel), a long press (opens a context menu), the swipe in from the left edge (back in the stack).
- [ ] **Apple Pencil**: `pointer_kind` is `Pen` (Signature Pad draws with the pencil's pressure); a pencil tap
  clicks.
- [ ] **A disabled control** (E3) takes no tap, no long press and shows no pressed look, and a finger still scrolls
  over it and inside it.
- [ ] **Lists**: scrolling with momentum, pull to refresh, a swipe row's actions.
- [ ] **The safe area**: Lightbox, Sheet, the tab bar and the navigation bar keep clear of the notch and the home
  indicator, upright and on its side.
- [ ] **The phone pieces' colors** (on recipes since 2026-11-17): the tab bar, the navigation bar, a sheet and pull
  to refresh in light and dark, as the board's Mobile frames.
- [ ] **The gestures added on 2026-11-17, on a real phone, and adapted to how iOS does them** (time and distance
  thresholds, what the system keeps for itself): one-finger zoom (double tap held and dragged); three fingers
  swiped left/right undo/redo, pinched in/out copy/paste, tapped open the editing menu — and that iOS's own
  three-finger gestures do not fire twice with lurq's; in a text, the loupe under a long press and the caret moving
  with the finger, the two selection handles and the menu over a selection; the bounce at a list's edge; a
  pencil's hover (iPad with Apple Pencil 2 or Pro) shows hover styles.
- [ ] **lurq's text by touch is off on iOS.** In a text field on the phone, the system draws the caret, the loupe,
  the handles and its edit menu (`set_native_text_selection`): lurq's loupe, handles and menu of editing commands
  must not show beside them, and three fingers' undo, copy and paste in a focused field must happen once (lurq
  leaves them to the system there). Outside a field (a canvas, a list), lurq's three-finger gestures and their menu
  work, and do not fight the system's own three-finger bar.
- [ ] **Two fingers' frame.** A pinch and a rotation are measured once both fingers of a frame moved (4 ms apart at
  most, a finger still for 40 ms is a pivot): on a 120 Hz iPhone a pinch is smooth, a two-finger swipe over a photo
  does not turn it, and a turn about one still finger rotates.
- [ ] **Apple Pencil hover** (iPadOS 16.1+, Pencil 2 or Pro): `UIHoverGestureRecognizer` with
  `allowedTouchTypes` pencil only, on winit's view. Over the gallery a button shows its hover style under the hovering
  pencil and loses it when the pencil leaves; a trackpad's pointer is still the mouse's, not the pencil's; the
  recognizer takes no touch from winit's view (a pencil's tap still clicks once).
- [ ] **Trackpad pinch and smart zoom on ZoomScroll** (Mac, added 2026-11-18). In the gallery's
  `/engine/zoom-scroll`, and a picture in a lightbox: a pinch on the trackpad zooms the plan about the pointer,
  apart closer and together further, as far as the fingers go, and the point under the pointer stays under it; a
  double tap of two fingers (smart zoom, on in System Settings › Trackpad) goes in at the pointer and again back
  out; two fingers moved together scroll the plan and do not zoom; the mouse's wheel with `wheel_zoom` turned away
  from you is closer. winit's `PinchGesture`, `RotationGesture` and `DoubleTapGesture` reach the tree through
  `winit_shell/trackpad_gestures.rs` (tested on Windows by its mapping alone): check the pinch's pace against
  Preview's, and that a rotation on the trackpad over an element with `on_rotate` turns it the way the fingers turn.
- [ ] **The emulator against the device.** The same page on the phone and on a Mac with `LURQ_DEVICE=phone`: the
  gestures feel and do the same; note what the mouse cannot imitate.
- [ ] **The on-screen keyboard** over a field: the field stays in view (or write down that lurq does not move it).

## 6. Sound and video

The steps of [apple-verification.md](../docs/evidence/media-sound/apple-verification.md), which this only lists:

- [ ] **A sound plays** (`Media::sound`, `AVAudioPlayer`, made on a thread that is not the main one with no run
  loop): Audio Player page, a file and bytes; pause, seek, volume, mute; it ends and the player says so.
- [ ] **Interrupted** by a call or another app's sound, it is paused, not ended.
- [ ] **iOS's audio session**: with the ring switch on silent the app's sound is silent (the default session);
  whether lurq should set a playback category is a decision to take then.
- [ ] **The cache on the disk** keeps a sound in the app's own cache folder (`mobile_demo`'s `cache_folder()`), and
  a second play reads it from there.
- [ ] **A video plays on macOS and iOS** (the system's `AVPlayer`, since 2026-11-17; see
  [decisions.md](decisions.md)): Video Player page, the section with the clip, and the playground with a source.
  It opens with its first frame and its length; play, pause, the seek bar, the speed, the volume and mute; it
  ends and says so, and plays again from its start. Its sound is heard with its picture.
  2026-11-17, on a Mac: `cargo test -p lurq --features media --test media_video_apple` passes (five checks, the
  ones Windows runs), and in the gallery the clip opens, plays and its time goes on, its frames drawn from the
  decoder's buffers with no copy (see 8). On Ivan's iPhone the video at an address plays and the app lags while
  it does: not explained yet. Not yet looked at: by ear; a clip filmed upright (drawn on its side, a known gap).

## 7. The tools

- [x] **DevTools** on a Mac: `cargo run -p gallery --features devtools,mcp -- --devtools` — F12 opens it; the tree,
  a value inspected, the profiler records.
  2026-11-18: F12 (a key of MCP) opens it in a window of its own; the tree of 1236 components, a node's layout
  and place, and a recording of 56 passes with its timeline, its phases and the flame chart.
- [x] **MCP** on a Mac: the gallery's log gives the address and the token; the tree is read, a screenshot is taken,
  a click and a key reach the window, `lurq_pointer` reads and sets the pointer.
  2026-11-18: the address and the token are in the log and the file of `~/Library/Application Support/lurq/mcp`;
  the tree, screenshots, clicks, the fingers of `lurq_touch` and `lurq_pointer` were used all through the work of
  2026-11-17/18. A key of MCP reaches lurq and not AppKit: Ctrl+Cmd+F does not put the window in full screen.
  A screenshot has the system's lights in it as they are on the screen, colored or gray: AppKit draws them as
  pictures as the screenshot is taken (`app/capture_overlay.rs`).

## 8. A video with no copy on Apple's systems: what is to be built

Asked for by Ivan on 2026-11-17, when the same was finished on Windows (`design/handoff/media-playback.md`). On
Windows a video's frames stay on the graphics card from the decoder to the screen (`images::CardFrames`), under
three renderers. None of that is Apple's: `CardFrames` is Direct3D's shared textures and is compiled on Windows
alone. What Apple's systems have is one older piece, and the rest is to be built. In the order to do it:

2026-11-17/18: built on `feat/design-system` (the decision is in [decisions.md](decisions.md), "A video plays on
macOS and iOS"). 1, 3, 4, 5 and 6 are done; 2 in part; 7 and 8 not.

- [x] **1. Run what exists, which was never run.** 2026-11-17: a video of the system's `AVPlayer` gives its
  buffers to the renderer, which draws them in the gallery on a Mac with the colors of the copy through memory.
  Over minutes of playing not watched yet. The `wgpu` renderer on Metal draws a `CVPixelBuffer` where it is:
  `NativeImageData::from_macos_cv_pixel_buffer_nv12(width, height, MacosCvPixelBuffer::retain(buffer))`, whose two
  planes become Metal textures through `CVMetalTextureCache` (`create_macos_native_nv12_cached_image_texture` in
  `wgpu_render/mod.rs`). Write a small example that decodes a clip with VideoToolbox or `AVAssetReader` (output
  `kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange`, `kCVPixelBufferMetalCompatibilityKey` and an IOSurface) and
  gives each buffer to a `ui::Video`. It must show the picture with the colors of the same frame decoded to memory,
  never lock the buffer's bytes, and hold its memory over some minutes of playing.
- [ ] **2. Mend what that run shows of the piece.** 2026-11-17: one texture cache for the device, kept; the
  textures of the frame before kept until the next is drawn. 2026-11-18: the crop holds for a buffer larger than
  its picture, the picture at its top left (`a_buffer_larger_than_its_picture_is_drawn_as_its_picture`); a
  picture elsewhere in the buffer (a clean aperture that does not start at 0, 0) is not read. The depth is 7. Read, not run, these are weak: a texture cache is made and let
  go of for each plane of each frame, where one for the device is meant; the planes are taken to be as large as
  the picture, so a buffer larger than its picture (a codec's 1088 rows for 1080) has no crop; and any buffer is
  taken for 8 bits and gray from 16 to 235 (see 7).
- [x] **3. Frames that change in one image.** 2026-11-17: the sibling type, `images::CvPixelBufferFrames` (`show`,
  `shown`); one image of it, drawn again at each frame; `images::cv_pixel_buffer_frames_drawn()` says whether a
  renderer draws them, and a decoder copies through memory where none does. No crop, lease or `way()` yet. On Windows one image shows a ring of textures: the decoder calls
  `CardFrames::show(frame)` and gets its texture back by the frame's lease. On a Mac an app makes a new image for
  every frame, which rebuilds what shows it. Decide and build one of: a kind of surface of `CardFrames` that is a
  pixel buffer, or a sibling type with the same `show`, `crop`, lease and `way()`. "One type for all of them" was
  talked of and not built (the handoff's "Not done").
- [x] **4. A video in lurq's own player.** 2026-11-17: `media/apple/video.rs`, `AVPlayer` with an
  `AVPlayerItemVideoOutput`, made and told on the main thread; lurq's clock follows it. `Media::open` says the build shows no video here; `media/apple.rs` plays a
  sound alone. Behind it goes AVFoundation: an `AVPlayer` with an `AVPlayerItemVideoOutput` gives pixel buffers
  timed against its own sound, and has seek and speed, which leaves lurq's clock to follow it as it follows
  `AVAudioPlayer` now. VideoToolbox with lurq's own clock is the other way, and more to write. Then
  `ui::VideoPlayer::source` plays on a Mac as on Windows, through 1 and 3.
- [x] **5. iOS.** 2026-11-17: the type and the renderer's path are of both systems; built and run on an iPhone. The pixel buffer's type and the renderer's path are `target_os = "macos"`: compile and run them for
  iOS, where `CVMetalTextureCache` is the same.
- [x] **6. The gallery and the tests.** 2026-11-17: the gallery's half: `crates/gallery/src/cv_demo.rs` writes a
  moving picture into buffers of its own (`MacosCvPixelBuffer::nv12`, `write_nv12`), seen on a Mac. 2026-11-18:
  the renderer's test, `wgpu_render/cv_pixel_buffer_tests.rs`: a frame written into a buffer whose rows are
  longer than its picture is read by the renderer's own code, through `CvPixelBufferFrames`, byte for byte as it
  was written (`cargo test -p lurq --lib --features raster cv_pixel_buffer_tests`); with the planes swapped it
  fails. It draws the planes as they are (`wgpu_render/planes_drawn.rs`, shared with Windows' test), not colors. The Video Player page's table of decoders (`crates/gallery/src/card_demo.rs`)
  and `wgpu_render/card_frames_tests.rs` are Windows'. A stand-in decoder that writes pixel buffers gives the page
  its rows on a Mac, and the tests their first one: a frame drawn from the buffer is the frame memory gives.
- [ ] **7. Color and depth.** One matrix is in the shader: BT.709 with gray from 16 to 235. A buffer of the full
  range (`420f`), of BT.601 or BT.2020, or of ten bits is drawn wrong or not at all. The same is not done on
  Windows.
- [ ] **8. Not looked at.** A Mac with two graphics cards (Intel ones, an external card) and a card taken away
  while the app runs. An IOSurface is shared between cards by the system, which Windows' textures are not, so
  what was built there for a decoder on another card may not be needed.

What a Mac gives for nothing, once 1 holds: an app with a decoder of its own (VideoToolbox) is drawn with no copy
today, by `from_macos_cv_pixel_buffer_nv12`, one image a frame.
