# Web: what is not done

What a lurq app in a browser still does differently from the same app on a desktop, or does not do at all.
Kept next to the work on `feat/gallery-web`; an entry goes when its gap is closed. The guide for what does
work is `docs/src/content/docs/web.md`.

"Not checked" means nobody has run it in a browser, not that it is known to be broken. What was run was run in
Chromium and in Firefox, on Windows, with a mouse and a hardware keyboard.

## Browsers

- **Chromium and Firefox were run.** In Firefox the gallery was looked through by hand; which of WebGPU and
  WebGL2 it drew through there was not noted. Safari: not checked.
- **WebGL2 was asked for, not fallen back to.** The gallery was run through WebGL2 with
  `index.html?backend=webgl` in a browser that has WebGPU. A browser that has none, where
  `WgpuDevice::request()` has to find that out and take WebGL2 by itself: not checked.
- **Neither WebGPU nor WebGL2**: `WgpuDevice::request()` returns an error and the app says so; nothing is
  drawn.
- **A lost device or context.** A browser takes a GPU device away when the tab is long in the background or
  the driver restarts. The engine makes its surface again when the surface is lost, with the device it was
  given: a device that is itself gone is not asked for again. Not checked what a person then sees.

## Text input

- **Input methods, dead keys, a phone's keyboard.** winit hears a canvas's key presses and nothing else: text
  that comes through composition (CJK, a dead key and a letter, a phone's on-screen keyboard, dictation) does
  not reach a text input. It needs what iOS got (`winit_shell/ios_text.rs`): an element of the page that takes
  the text, here a hidden `<input>` or `contenteditable` the shell keeps in step with the focused input.
- **A key held when the page gets the focus.** winit tells a pressed key before the modifiers that came with
  it, so the first key of a page that was focused with Ctrl already down is taken without it. A keyboard that
  presses Ctrl as its own key first, as every one does, is not affected.
- **The clipboard's images and files.** A paste shortcut reads text (`navigator.clipboard.readText`). Images
  and files reach a page through a `paste` event, which the canvas's key handling cancels. `on_paste` handlers
  therefore see text only, and `copy_image_to_clipboard` and `copy_files_to_clipboard` return `false`.
- **Reading the clipboard without a paste.** `read_from_clipboard` and `read_clipboard` return what the page
  copied last: a page cannot read the system clipboard without waiting.

## Pointer and touch

- **Touch.** A finger on a canvas arrives as winit's touch events, which the shell turns into a pointer only
  on iOS (`winit_shell/touch.rs` is built for iOS and tests). On a phone's or tablet's browser: not checked,
  and a finger likely does nothing.
- **Hover before the pointer moves.** The wheel says where the pointer is (`web::WheelPointer`), so a page
  scrolls under a pointer at rest. What is under that pointer is not hovered until it moves.
- **Dragging the window, resizing it, its minimize and maximize**: a page has none. `WindowChrome` in its
  default mode draws no title bar; one in `AlwaysCustom` mode draws controls that have nothing to act on.

## Windows

- **More than one window.** winit makes a canvas for every window, and the shell puts only the first into the
  page: a secondary window's canvas is made and never shown. What it takes: a place in the page for each
  (a floating panel, or one the page docks), its size and position from what the window asked for, a title
  bar the shell moves it by (`drag_window` does nothing in a browser), and the canvas taken away when the
  window closes. The GPU device is already shared: `with_device` gives every window's engine the same one.
  A WebGL2 device is one canvas's, so further windows there need a device each, or one canvas for all.
- **DevTools** is a tree in a secondary window, so it waits for the above. Past that, three places of it are a
  desktop's: the save and open dialogs (`rfd` on a thread of its own; a page downloads and uploads), a
  screenshot's read-back (waits for the GPU on a thread; a page is called back), and the profiler's writer
  thread.
- **Native menus, file dialogs, file drags onto the window**: not there.

## Features

- **Run in a browser:** `winit`, `wgpu`, `webgl`, `router`, `clipboard`, `image` (the gallery's).
- **Build for it, not run:** `markdown`, `svg`, `form`, `i18n`, `query`.
- **`canvas` does not build.** Its uploads and its readiness wait on a thread and a condition variable
  (`canvas/gpu.rs`, `wgpu_render/canvas.rs`).
- **Not tried:** `mcp` (a socket to listen on), `devtools` (see Windows), `resources` (`ureq`; a page fetches),
  `persistent_storage` (its files are `redb`'s, and a page has IndexedDB or the origin's private file system;
  its in-memory default was not tried either), `tokio`
  (a runtime with threads), `screenshot` (waits for the GPU on a thread), `dx12`.
- **On WebGL2, not looked at:** the `svg` pipeline and video frames (NV12 images).

## The page

- **The address does not follow the router.** The gallery reads the page to open on from the fragment once
  (`index.html#/button`). Navigating does not change the address, the browser's back button leaves the app,
  and a reload opens the page the address names, not the one that was shown. It takes a `hashchange` (or
  `popstate`) listener that navigates, and the router telling the page when it has.
- **A page reader finds nothing.** Everything is pixels of one canvas: no text to select by the browser, to
  search with Ctrl+F, to translate or to read out. The element tree an MCP client reads is not given to the
  page.
- **Fonts.** `scripts/gallery-web.py` copies Segoe UI and Consolas from Windows, which may not be published.
  A deployed gallery needs fonts it may ship (`--fonts DIR`); which ones is not decided. cosmic-text has no
  list of fallback fonts for a browser, so a character the named fonts lack is looked for in every loaded
  font in turn: a page that shows text of many scripts has to load fonts for them.
- **Size.** The gallery's `.wasm` is 16 MB (12 MB without `webgl`), built for speed and not passed through
  `wasm-opt`. Nothing is shown while it downloads but the page's own "Loading" line.
- **Images decode on the page's thread**, one after the turn of the loop that asked for it: a large image
  holds frames back while it decodes.
