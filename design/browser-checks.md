# What to check in each browser before the browser counts as supported

The browser's work (2026-11-19, `claude/browser-support-926cf3`) was run in Chromium: the Claude app's own browser
and a headless Chrome on Windows, with a mouse, a keyboard, a pen and touch emulated by the browser, through WebGPU
and WebGL2. Firefox and Safari were not run, nor a phone's or a tablet's browser. Go through this in **Chrome**,
**Edge**, **Firefox** and **Safari** (macOS, and iOS on an iPhone or an iPad), top to bottom; each line says what to
do and what must happen. What fails is fixed, not written down; what passes gets a tick, the browser and the date.

Build and serve the gallery: `python scripts/gallery-web.py --serve`, then open `http://127.0.0.1:8123/`. A phone
reaches the computer's address on the same network over HTTPS only for WebGPU: on plain HTTP it draws through
WebGL2, which is a check of its own. `?backend=webgl` forces WebGL2 anywhere.

## 1. It starts

- [ ] **It draws.** The Overview page, with the side bar, the header and text in Inter, within a second or two.
  The console (F12) has no error; it says `drawing through BrowserWebGpu` (or `Gl`).
- [ ] **WebGL2.** The same with `?backend=webgl`. In Safari before WebGPU is on, and in Firefox on a system where
  it has none, the page takes WebGL2 by itself.
- [ ] **Sharp at every scale.** Zoom the page (Ctrl/Cmd + and −) and move the window to a screen of another scale:
  text stays sharp and the layout follows the new width. With DevTools' device mode at a ratio of 2 or 3 the page
  is drawn at the device's size, not larger.
- [ ] **The first frame.** `performance.getEntriesByName("lurq:first-frame")[0].startTime` in the console: under
  1500 ms on a first load from `localhost`, under 500 ms on the next.

## 2. Pages

- [ ] **Every page.** Go down the side bar (the arrow keys move in it). Each page draws its playground and its
  sections; scroll each to its end. Nothing blank, nothing in the wrong font, nothing larger than in the desktop
  gallery.
- [ ] **Overlays.** Open a Dialog, a Sheet, a Popover, a Menu, a Select, a Tooltip and the Command Palette: each
  comes over the page with its shadow and the blur behind it, and Escape closes it.
- [ ] **GPU Viewport.** The rings turn in the viewport, in both backends.
- [ ] **Persistent Storage.** Write a value, reload the page: the value is there. Clear the site's data: it is gone.

## 3. Input

- [ ] **Keyboard.** Type in Input and Textarea; Tab and Shift+Tab move the focus; arrows, Home, End, Backspace,
  Delete; Ctrl/Cmd+A, Z, Y; the browser's own shortcuts for tabs still work.
- [ ] **An input method.** With a Chinese or Japanese input method, type in Input: the composition is shown
  underlined at the caret and the candidates open beside it; Enter commits it. A dead key (´ then e on a
  keyboard that has one) gives é. On macOS, press and hold a letter for its accents.
- [ ] **Dictation and emoji.** Windows' Win+H and Win+. (macOS: Fn twice, Ctrl+Cmd+Space) put their text into Input.
- [ ] **The wheel and a touchpad.** A mouse's notch scrolls smoothly by one step; a touchpad scrolls with it, and
  pinching zooms Zoom Scroll.
- [ ] **Touch** (a phone, a tablet, a touch screen). A tap presses a Button and toggles a Switch; a drag scrolls
  the page and moves a Slider; a long press opens a Context Menu; two fingers pinch Zoom Scroll.
- [ ] **A pen** (an Apple Pencil, a Surface pen). It draws on Signature Pad with its pressure, and presses as a pen.
- [ ] **A phone's keyboard.** On iOS and Android, a tap on Input opens the keyboard; letters, Backspace, the
  suggestions and autocorrect reach the input.

## 4. What a browser gives

- [ ] **The clipboard.** Copy text from a lurq input and paste it elsewhere, and the other way: the browser may
  ask first (Firefox shows a Paste button by the pointer). A picture copied from another program pastes where
  an input takes pictures. Ctrl/Cmd+V is answered a moment later, not lost.
- [ ] **Sound.** Audio Player, source "Bytes", play: the track plays to 0:24 and stops, with `on_ended` in Events.
  Then "An address": the default address plays, its length 6:12. Volume, mute, seek and speed act at once.
- [ ] **Files.** File Upload and Drop Zone: choose files with the dialog, and drag files from the system onto the
  zone (it says "Drop to add" while they hover): the Events tab has them, with their sizes.
- [ ] **Links.** `View source` opens the page's file on GitHub in a new tab.
- [ ] **The address.** Going from page to page changes the address; Back and Forward move the gallery; a reload
  stays on the page.

## 5. It holds

- [ ] **Out of sight.** Leave the tab for a minute and come back: it draws at once. Sleep the computer and wake it:
  the page draws (a lost device is made again).
- [ ] **Size.** The network tab: `gallery_bg.wasm` about 25 MB, 8.6 MB where the server compresses.
