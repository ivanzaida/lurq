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

- [ ] **The workspace on a Mac.** `cargo build -p lurq`, `cargo build -p gallery --features devtools,mcp`,
  `cargo build -p demo`. Must link (no missing framework) and start.
- [ ] **The test matrix on a Mac.** `bash target/suites2.sh` from the design-system worktree (the runs A–G of the
  feature cleanup), or by hand: every suite on `default` + `markdown,mcp`, `--no-default-features` check, a
  minimal app's tests (`--no-default-features --features winit,wgpu --tests`), `devtools` and `perf_profile`.
  All green, the same counts as on Windows give or take what is Windows-only.
- [ ] **`http` on Apple.** Never compiled there (its TLS, `ring`, needs Apple's C compiler):
  `cargo build -p lurq --features http` on the Mac, and for `aarch64-apple-ios`. Link Preview, a picture at an
  address and a sound at an address depend on it.
- [ ] **The phone app.** Build `crates/mobile_demo` for the simulator and a device from Xcode
  (`crates/mobile_demo/ios/MobileDemo.xcodeproj`; Xcode runs `ios/build-rust.sh`). It starts and draws.
- [ ] **The gallery on iOS**, if it is built for it: Ivan's tabs, the pages at a phone's width, the pointer toggle.

## 2. Drawing on Metal

- [ ] **Borders and rings.** `quad.wgsl` was rewritten for strokes and compared with its formulas on Direct3D only.
  Gallery, pages Button, Card, Input, Tabs: borders of 1 and 2, rounded corners, a focus ring growing in (Tab
  into a button), a dashed edge where a page shows one. They look as on Windows.
- [ ] **A lost device** (`edab8590`). Hard to cause on purpose; if a Mac sleeps with the gallery open and wakes, the
  canvases (charts, Signature Pad, QR Code) are drawn again and a video goes on. At least: nothing goes black.
- [ ] **The canvas on the GPU.** Pages Bar Chart, Line Chart, Signature Pad (draw, Undo, Clear, Make a picture), QR
  Code (the code is scanned by a phone's camera), Watermark (on the long scrolled page).
- [ ] **Icon hinting on a 1x display**, if one is at hand: `cargo run -p lurq --example icon_hinting --features
  winit,wgpu,screenshot`. On a 2x display hinting matters little.
- [ ] **Blur and shadows.** Pages with a backdrop blur (a sheet, a popover over content) and shadows (Card, Menu).

## 3. The system and the windows

- [ ] **Reduce motion, macOS.** `cargo test -p lurq --lib motion`: `the_platform_has_the_setting` passes. Then
  System Settings › Accessibility › Display › Reduce motion on, click back into the gallery: a transition (a
  sheet, a dialog) ends at once; off again, it moves again.
- [ ] **Reduce motion, iOS.** Settings › Accessibility › Motion › Reduce Motion, back to `mobile_demo`: a sheet and a
  stack push switch without moving. It links on the simulator and on a device.
- [ ] **Light and dark** follow the system, and change live when the system's appearance changes (macOS: System
  Settings › Appearance; iOS: Control Centre).
- [ ] **The window's chrome on a Mac** (`ui::WindowChrome`): the traffic lights where macOS puts them and working,
  the title bar dragging the window, double-click zooming it, the gallery's tabs in the title bar (open, close,
  reorder, a new tab with Cmd+T), full screen and back.
- [ ] **Picture in picture.** Video Player page: picture in picture opens a small window that stays over other
  apps' windows (`always_on_top`), and closes back.
- [ ] **Opening an address** (`open_url`). Link Preview page and a `ui::Link` with `href`: macOS opens the default
  browser (`NSWorkspace openURL:`), iOS opens Safari (`UIApplication openURL:`). A `file:` address is refused.
- [ ] **File dialogs** (macOS): Path Field page — Browse… picks a folder, Save as… saves a file with the filter and
  the name; Cancel gives nothing. iOS has no dialog: the button says so or does nothing, and nothing crashes.
- [ ] **The clipboard.** Copy from a Markdown page (rich text pastes as Markdown into a text editor, plain into a
  field), a picture copied and pasted, Signature Pad's PNG. Paste into a lurq field from another app.
- [ ] **Fonts and text.** The system fonts look right (San Francisco, the mono one in code); the caret blinks and
  stops while typing; selection by drag and by double-click; IME: Ukrainian, then Japanese with its candidate
  window in the right place.

## 4. The keyboard (the editing commands of 2026-11-17)

- [ ] **Cmd, not Ctrl.** In a text field and on the Signature Pad: Cmd+Z undoes, Cmd+Shift+Z redoes, Cmd+Y does
  **not** redo; Cmd+C, Cmd+X, Cmd+V, Cmd+A. In a Ukrainian layout the same keys by their place.
- [ ] **A native Edit menu.** If the app has a menu bar with Undo/Redo/Copy/Paste items with their accelerators,
  the menu takes Cmd+Z before the window: each item must run `window.edit(EditCommand::..)` and do the same thing
  the key does. The gallery's menu, if it has one, as the example.
- [ ] **Option + a letter** types the letter's character and is not a command (Option is Alt).
- [ ] **Tab and the focus** go through the controls as on Windows; Full Keyboard Access on macOS where it changes
  what Tab reaches.

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
- [ ] **A video on macOS and iOS does not play** (no decoder yet): the player says so in the locale's words, and
  nothing crashes.

## 7. The tools

- [ ] **DevTools** on a Mac: `cargo run -p gallery --features devtools,mcp -- --devtools` — F12 opens it; the tree,
  a value inspected, the profiler records.
- [ ] **MCP** on a Mac: the gallery's log gives the address and the token; the tree is read, a screenshot is taken,
  a click and a key reach the window, `lurq_pointer` reads and sets the pointer.
