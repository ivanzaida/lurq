---
title: Web
description: Run a lurq app as a page of a browser, through WebGPU or WebGL2, and what a page has to give it that a desktop gives at once.
---

A lurq app builds for `wasm32-unknown-unknown` and runs as a page: the `winit` shell with the `wgpu` renderer
draws the same component tree to a canvas, through WebGPU, or through WebGL2 in a browser that has no WebGPU
(feature `webgl`). The components, the layout, the theme and the events are the ones of a desktop. This page
says what an app has to set up, what a page does differently, and how to build and serve one.

The browser is a supported platform, beside Windows, macOS and iOS: every page of the gallery works there, and
what a desktop has works there or has the browser's own equivalent. What a page cannot do is the platform's limit,
said in [What a page cannot do](#what-a-page-cannot-do); what is not done yet is [at the end](#not-done-yet), and
in full in `docs/web-gaps.md` in the repository. What to look at by hand in Chrome, Edge, Firefox and Safari is
`design/browser-checks.md`.

The gallery (`crates/gallery`) is a complete example:

```powershell
python scripts/gallery-web.py --serve
```

builds it into `target/gallery-web` and serves it on `http://127.0.0.1:8123`. See
[The gallery's script](#the-gallerys-script).

## Setting up

### The tools

1. Add the target: `rustup target add wasm32-unknown-unknown`.
2. Install wasm-bindgen's CLI at the version of the `wasm-bindgen` crate in the app's `Cargo.lock`. The two
   must be the same, and the CLI says so when they are not:
   `cargo install wasm-bindgen-cli --version <that version> --locked`.

### The manifest

The app depends on lurq with a window and a renderer, as on a desktop, and on what its `main` needs of the browser
when it is built for one. lurq's default features are the desktop set, which a page does not take: the app names
its list for every target and takes the default on a desktop alone.

```toml
[dependencies]
lurq = { version = "0.62", default-features = false, features = ["winit", "wgpu", "clipboard", "image", "canvas", "form", "router", "query"] }

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
# The desktop set, the system's file dialog with it.
lurq = "0.62"

[target.'cfg(target_arch = "wasm32")'.dependencies]
# WebGL2 for a browser without WebGPU; leave it out to draw through WebGPU alone.
lurq = { version = "0.62", features = ["webgl"] }
wasm-bindgen-futures = "0.4"
# To fetch the fonts; how is the app's own choice.
web-sys = { version = "0.3", features = ["Window", "Response"] }
js-sys = "0.3"
```

The app stays a binary: wasm-bindgen runs its `main` when the page loads the module.

### The page

A page with a canvas for the window and a module script that loads the app:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <style>
      html, body { margin: 0; height: 100%; overflow: hidden; }
      #lurq { display: block; width: 100vw; height: 100vh; outline: none; }
    </style>
  </head>
  <body>
    <canvas id="lurq"></canvas>
    <script type="module">
      import init from "./app.js";
      init();
    </script>
  </body>
</html>
```

`crates/gallery/web/index.html` is the gallery's, with a line that says what the page waits for and why it did
not start.

### Building and serving

```powershell
cargo build -p app --target wasm32-unknown-unknown --profile web
wasm-bindgen --target web --no-typescript --remove-name-section --out-dir dist --out-name app target/wasm32-unknown-unknown/web/app.wasm
```

`dist` then has `app.js` and `app_bg.wasm`. Put the page and the fonts beside them and serve the directory
with any server of static files that sends `.wasm` as `application/wasm`. A page opened from the disk
(`file://`) does not load a module.

### Size

What a page downloads is its size, so a page is built for it. The workspace's `web` profile is release with
`opt-level = "z"`, and `--remove-name-section` drops the names of the module's functions, which only a debugger
reads:

```toml
[profile.web]
inherits = "release"
opt-level = "z"
```

The gallery, every component and its pages, is a module of 25.0 MB, 8.6 MB as a server that compresses sends it
(gzip). Built with release's `opt-level = 3` it is 38.7 MB (11.6 MB): its frames and its first frame measured the
same (`scripts/gallery-web-bench.py`). Serve the module compressed: a static server that does not, sends the
whole of it. `wasm-opt` (binaryen) takes a little more off the size of the module and is not needed.

### The first frame

The page's performance timeline has a mark, `lurq:first-frame`, at the first frame the app presents: what a page's
own script, a profiler or a test measures the start of the app by.

```js
performance.getEntriesByName("lurq:first-frame")[0].startTime
```

The gallery's, served from `localhost`: about 750 ms on its first load, which compiles the module, and 150 to 400
ms after, when the browser has the compiled module cached.

A browser has WebGPU for a page served from `localhost` or over HTTPS, and for no other. WebGL2 is there on
any page. The clipboard is the browser's to give on the same terms as WebGPU.

## What `main` does in a browser

A desktop gives an app its GPU device and its fonts at once. A page gets both by asking and waiting, and a
page cannot wait inside a frame, so `main` waits for them before it starts the window:

```rust
use lurq::{
  app::{
    App, Tree,
    wgpu_render::{WgpuDevice, WgpuRenderEngine},
    winit_shell::WinitWindow,
  },
  layout::text_style::GenericFamily,
};

fn main() {
  #[cfg(not(target_arch = "wasm32"))]
  run(App::new(), WgpuRenderEngine::new);

  #[cfg(target_arch = "wasm32")]
  wasm_bindgen_futures::spawn_local(async {
    // The device comes from a promise.
    let device = WgpuDevice::request().await.expect("a browser with WebGPU or WebGL2");

    let mut app = App::new();
    // A page cannot read the system's fonts: it fetches its own and says which is which.
    app.load_font(fetch_bytes("fonts/Inter.ttf").await);
    app.load_font(fetch_bytes("fonts/JetBrainsMono.ttf").await);
    app.set_generic_family(GenericFamily::SansSerif, "Inter");
    app.set_generic_family(GenericFamily::Monospace, "JetBrains Mono");

    run(app, move || WgpuRenderEngine::new().with_device(device.clone()));
  });
}

fn run(mut app: App, engine: impl Fn() -> WgpuRenderEngine + 'static) {
  let mut tree = Tree::new();
  tree.set_render_engine_factory(move || Box::new(engine()));
  tree.mount_root::<Root>(&mut app, RootProps::default());
  // In a browser this hands the app to the browser's loop and returns at once.
  WinitWindow::new(app, tree).with_title("App").run();
}
```

`fetch_bytes` is the app's: `crates/gallery/src/web.rs` has one over the browser's `fetch`, and tells the
person on the page when the device or a font did not come.

### The device

`WgpuDevice::request()` asks for the device an engine would ask for at its first frame, and
`WgpuRenderEngine::with_device` gives it to an engine, which then asks for none. An engine without one panics
at its first frame in a browser, and says what to do. On a desktop both are optional: they move the wait for
the device off the first frame.

A `WgpuDevice` is cheap to clone, and the factory gives every engine the same one.

### The fonts

`App::new()` finds no font in a browser, and with none loaded no text is drawn: lurq's own icons are the only
font in the binary.

- `App::load_font(bytes)` takes a TrueType or OpenType file, or a collection of them. It does not read WOFF
  or WOFF2: fetch the `.ttf` or `.otf`.
- `App::set_generic_family(generic, family)` names the font a generic family draws with.
  `GenericFamily::SansSerif` is also the font of text with no family, so name it at least;
  `GenericFamily::Monospace` is what the theme's code text asks for.
- Load a face for each weight and slant the app draws. A weight with no face is drawn with the nearest one.
- A browser has no list of fonts to fall back on. A character the named font lacks is looked for in every
  loaded font, so an app that shows other scripts, symbols or emoji loads a font that has them.

A font put into the binary with `include_bytes!` is loaded as on a desktop and needs no fetch; it is then part
of the page's first download.

The fonts are the app's to ship: a font of the system it was built on may not be the app's to publish.
The gallery ships Inter and JetBrains Mono, under the SIL Open Font License (`crates/gallery/web/fonts`).

### The loop

`WinitWindow::run` returns as soon as the app is handed to the browser. Nothing after it is "after the window
closed", and an app does not close its page: `WindowHandle::close` stops the loop and leaves the last frame on
the canvas.

## WebGPU and WebGL

With the `wgpu` feature alone a page draws through WebGPU, and `WgpuDevice::request()` fails in a browser
that has none. The `webgl` feature adds WebGL2 for those browsers: the browser is asked for WebGPU first, and
the device is WebGL2's when it has none. `WgpuDevice::backend()` says which it is:

```rust
use lurq::app::wgpu_render::{WgpuBackend, WgpuDevice};

let device = WgpuDevice::request().await?;
match device.backend() {
  WgpuBackend::BrowserWebGpu => { /* WebGPU */ }
  WgpuBackend::Gl => { /* WebGL2 */ }
  _ => {}
}
```

`WgpuDevice::request_with(backends)` asks among some backends only: `WgpuBackends::GL` for WebGL2 also where
there is WebGPU, to see an app through it, and `WgpuBackends::BROWSER_WEBGPU` for WebGPU alone. The gallery
does so for `index.html?backend=webgl` and `?backend=webgpu`, and says in the console which it draws through.

The feature adds wgpu's own implementation and its shader translator to the page: the gallery's `.wasm` grew
from 12 MB to 16 MB. It changes nothing outside a browser.

What is drawn is the same. Two things are done another way on WebGL2, which an app does not see:

- It has no storage buffers. The quad shader reads gradient stops and the colors of a stroke's sides out of
  a float texture there (`wgpu_render/gradients.rs`).
- Its surface cannot be copied from, which a backdrop blur of the window does. A frame with one is painted
  into a texture beside the surface and put onto it in a last pass (`wgpu_render/surface_proxy.rs`); a frame
  without one goes straight to the surface.

A frame extension (`WgpuFrameExtension`) draws with the device it is given and keeps to the same limits: no
storage buffers and no compute on WebGL2.

## The address

A router can be the one the page's address follows:

```rust
router.follow_address();
```

The address then has the router's path as its fragment (`index.html#/button`), navigating adds to the browser's history, the browser's back and forward buttons move the router, an address a person types is navigated to, and a reload stays on the screen it was on. It does nothing outside a browser, so an app calls it on every platform. [The Address Of A Page](../routing/#the-address-of-a-page) has the rules.

## The canvas

The page lays the window out, not the app:

- A `<canvas id="lurq">` of the page is drawn to where the page's CSS put it.
- Without one, a canvas with that id is added to the body and fills the viewport.

The canvas is looked for, or added, when the device is asked for: WebGL gives a device for a canvas.

`with_size`, `with_min_size`, `with_max_size` and `with_position` are a desktop's and are not given to the
canvas. The tree is laid out at the canvas's size and the display's pixel ratio, and again when either changes:
a breakpoint follows the browser's window as it follows a desktop's.

A `WindowChrome` in its default mode draws no title bar, as on any platform but Windows and macOS.

## What differs

### Time

std's `Instant::now` and `SystemTime::now` panic in a browser. `lurq::time` is the clock chosen by the
platform: `std::time` everywhere else, the page's clock there.

```rust
use lurq::time::{Duration, Instant};

let started = Instant::now();
```

lurq reads the time through it, and an app that runs in a browser does too. On a desktop the types are std's
own, so nothing changes for code that passes them to std or to another crate.

### Threads

A page has one thread, and `std::thread::spawn` fails on it.

What the browser calls on that thread between the loop's turns, such as a promise of the app's own that settled or a listener it put on the page, may write a signal as another thread may on a desktop: the loop is woken and draws what changed.

A picture is decoded by the browser, off that thread (`createImageBitmap`), and made smaller there when the
picture is asked for at a smaller size: a large photo does not hold frames back. Its pixels are read back as the
`image` crate gives them, unpremultiplied and in the picture's own colors. What the browser does not decode (a TIFF;
an ICO in some browsers) and an SVG, which lurq draws itself, are decoded on the page's thread after the turn of
the loop that asked for them. `DecodedImages::wait_idle` cannot wait there: it tells whether a decode is pending.

### The clipboard

With the `clipboard` feature:

- A copy (`Ctrl+C`, `copy_to_clipboard`) goes to the system clipboard.
- A paste shortcut reads the system clipboard through the browser, which may ask the person first. The text
  arrives a turn of the loop later and is pasted then, through the same
  [`PasteEvent`](../styling-events/#paste). When the browser refuses, what the page copied last is pasted.
- `read_from_clipboard` and `read_clipboard` return what the page copied last: a page cannot read the system
  clipboard without waiting.
- A picture crosses as a PNG. `copy_image_to_clipboard` writes one; a paste reads the clipboard's items, a PNG and
  text, and gives the picture to [`PasteEvent`](../styling-events/#paste) as on a desktop. Chrome and Edge ask the
  person the first time a page reads the clipboard; Firefox shows a button to press beside the pointer; Safari
  reads only in the press of a paste the person made. A browser that refuses the items gives the text alone.
- Files do not cross: `copy_files_to_clipboard` returns `false`, and a paste has no files. A page has no paths.

### Sharing

[`window.share(Share::url(..))`](../ctx/#sharing-an-address) is the browser's `navigator.share`, asked within the
press that shares, since a browser gives its sheet to nothing else, and only on a page that is a secure one
(HTTPS, `localhost`). Not every browser has it: the browsers of a phone do, and Firefox on a desktop does not.
Where there is none the call answers `ShareError::Unsupported`, and `share::supported()` says so ahead.

### Input

- Keys of a hardware keyboard reach the app as on a desktop, text inputs included. While the canvas has the
  focus the browser is asked not to act on a key itself, which it grants for all but the shortcuts it keeps
  to itself, such as the ones for its tabs.
- The wheel scrolls what is under the pointer, also on a page that has just loaded under a pointer at rest.
  A notch scrolls as far as it does in the desktop app, not as far as the browser would scroll a page of its
  own: a browser tells a notch as about a hundred pixels, which the shell counts back into notches. On macOS
  the browser's distance is the system's and is taken as it is.
- A notch of a mouse's wheel is [animated](../styling-events/#the-mouse-wheel) as in the desktop app. A browser
  tells a mouse's wheel and a touchpad the same way, in pixels, so the shell takes for a mouse's what comes to a
  whole number of notches, give or take a tenth, along one axis alone. On Windows, Chrome and Edge tell a
  notch as 100 pixels at every display scale and Firefox as six lines of the page's font, 102 or 108 pixels
  with the default one; each scrolls 40 logical pixels. Anything else is a touchpad's travel and moves the
  content at once, as on the desktop. So does a notch the browser tells as another distance: on a page
  Chrome or Edge has zoomed (80 pixels a notch at 125%), on a system set to scroll other than three lines a
  notch, in Firefox on a page whose root font is much larger, and on macOS, where a notch is the distance the
  system made of it and nothing tells it from a touchpad's.
- Text that comes other than as keys reaches a text input as on a desktop: an input method's composition (shown
  underlined until it is committed), a dead key and its letter, a phone's keyboard, dictation and the system's
  panel of emoji. While a text input has the focus, the page's focus is on a transparent textarea of the shell's
  own at the input's caret, where the input method's window opens; its keys go on to the canvas as before, and
  what it is given as text is the input's. A delete a phone's keyboard asks for is the input's Backspace.
- A finger is a touch, as on iOS: a tap presses, a drag scrolls, the gestures [iOS](../ios/#touch) tells are told.
  A pen is the browser's pen (`pointerType`): it presses as `PointerKind::Pen`, with its pressure.

### Sound

With the `media` feature, `Media::sound` and `ui::AudioPlayer::source` play in a page's own `<audio>`: an address
as it is, which the browser reads as it plays and may read from another server without asking it (an `<audio>` is
not bound by CORS, as a `fetch` is); a file or a resource as the page's own file at that path; bytes as a blob of
the page. Play, pause, seek, the rate, the volume and muted are done at once, and the media's events are told as
on a desktop: `on_load`, `on_playing_change`, `on_waiting_change`, `on_ended`, `on_error`. Nothing is loaded
whole before it plays, and nothing is kept: the browser keeps what it keeps of what pages load.

A browser starts a sound only in the press or the key that asked for it, or a moment after: a sound told to play
with no person's press (as the page opens) is paused again, and its state says it does not play.

A video is not played in a browser: `Media::open` says `MediaError::Unavailable`. What a desktop does with no
copy of each frame, a browser does through an external texture (`importExternalTexture`), which wgpu's backend
for the web does not implement yet (wgpu 30).

### Loading files and addresses

A load of the app's [`Loader`](../resources-media/#loading) reads with the page's `fetch`: an address, and a file or a
resource of the app, which in a browser is the page's own file at that path, read from beside the page. The
loader's headers, timeout and limit hold; a header the browser keeps to itself (`User-Agent`, `Cookie`) is not
sent.

The browser decides what a page may read. An address on another server is read only where that server lets the
page's origin read it (CORS); most pages of the web do not, and a link preview of one fails with a sentence that
names both causes, the network and CORS: the browser tells the page no more. An app reads such addresses through
a server of its own:

```rust
loader.proxy(|address| format!("https://example.com/fetch?url={address}"));
```

`Loader::proxy` is there on every platform; what the app's server does with the address is the app's.

### Files

With the `file_dialog` feature, `FileDialog::pick` opens the browser's file input, which only a press may open.
A file chosen has its name, its size and its bytes, read as it is chosen: `DroppedFile::read` gives them on every
platform (from its path on a desktop). `FileDialog::save` hands bytes to the browser as a download of the dialog's
file name, where on a desktop it opens the dialog for where to save and writes the file. There is no dialog for a
folder or for where to save: `pick_folder` and `pick_save` give nothing.

Files dragged onto the page from the system reach the element under them as on a desktop
([file drag and drop](../styling-events/#file-drag-and-drop)). While they hover the browser tells only
their kinds, so each is named `dragged/<n>.<extension of its kind>` until it is dropped; on the drop they are read,
and `DroppedFile::from_path` of a dropped path is the file with its bytes.

### The store

With the `persistent_storage` feature, `PersistentStorage::open(name)` keeps the values in the page's
`localStorage`, under `lurq:<name>:`, where a desktop keeps a file: what a page keeps between visits of the same
origin. A browser keeps a few megabytes for an origin, and the person may clear them with the site's data; a store
that is full says so in its error.

### Shaders

A browser's WGSL compiler refuses a derivative (`fwidth`, `dpdx`, `dpdy`, a `textureSample`) under a branch
on a value of the instance, which naga does not check. lurq's shaders turn that check off with
`diagnostic(off, derivative_uniformity);`. A frame extension's shader that branches the same way needs the
line too, or its pipeline is invalid in a browser and valid everywhere else.

## The gallery's script

`scripts/gallery-web.py` builds `crates/gallery` for the browser:

| Command | Does |
| --- | --- |
| `python scripts/gallery-web.py` | Builds into `target/gallery-web` with the `web` profile: the module, the page, the fonts |
| `... --serve` | Builds, then serves on `http://127.0.0.1:8123` |
| `... --no-build` | Serves what is already built |
| `... --port 9000` | Serves on another port |
| `... --dev` | An unoptimised build: quicker to build, slower to run |
| `... --fonts DIR` | Copies the page's fonts from `DIR` |
| `... --system-fonts` | Copies Windows' Segoe UI and Consolas, as the desktop gallery draws there; not to publish |

The page fetches its fonts from `fonts/` beside it. `fonts/index.txt` names them: a file a line, and the
families the generic ones draw with.

```text
sans-serif = Inter
monospace = JetBrains Mono
Inter-Regular.ttf
Inter-SemiBold.ttf
Inter-Bold.ttf
JetBrainsMono-Regular.ttf
```

The script copies the gallery's own fonts, Inter and JetBrains Mono, under the SIL Open Font License
(`crates/gallery/web/fonts`): a gallery put on a server may ship them. `--system-fonts`, on Windows, copies the
system's Segoe UI and Consolas instead, so the page looks as the desktop gallery does there; those are Microsoft's
and are not to be published.

The address names the page and the backend: `index.html#/button` is the Button page, and follows the
gallery as it goes from page to page; `?backend=webgl` draws through WebGL2.

## When nothing is drawn

- **The page says the gallery stopped, or stays empty.** The browser's console has the panic: lurq's own, or
  wgpu's when a pipeline or a pass is refused.
- **"The GPU device comes from a promise".** An engine was made without `with_device`; see
  [The device](#the-device).
- **No text, everything else there.** No font is loaded, or no family is named for `GenericFamily::SansSerif`.
- **No WebGPU on a page that has it elsewhere.** The page is not served from `localhost` or over HTTPS.
- **The module does not load.** The CLI and the crate of wasm-bindgen differ, or the server does not send
  `.wasm` as `application/wasm`.

## What a page cannot do

What follows is the browser's, not lurq's: no app in a page does it.

- **Threads.** A page has one thread: `std::thread::spawn` fails, nothing waits for another thread, and what a
  desktop does off the loop (a decode, a load) the browser does, and answers between the loop's turns.
- **The clipboard without asking.** A page reads the system clipboard only through the browser, which may ask the
  person first; at once it reads only what it copied itself. Files do not cross the clipboard.
- **Paths.** A page reads no file by its path and is given none: a file chosen or dropped comes with its bytes, a
  file is saved as a download, and there is no dialog for a folder.
- **Another server's addresses.** A `fetch` of another server is read only where that server allows the page's
  origin (CORS): an app reads the others through a server of its own ([`Loader::proxy`](#loading-files-and-addresses)).
- **Sound by itself.** A sound starts only in, or right after, a person's press or key.
- **Listening.** A page cannot listen on a socket: no [MCP server](../mcp/) runs in it.
- **The system's fonts.** A page cannot read them: it brings its own ([The fonts](#the-fonts)).
- **Its window.** The page lays out the canvas: no size, position, title bar, minimize or maximize of the app's,
  and `WindowHandle::close` stops the loop and leaves the page.

## Not done yet

`docs/web-gaps.md` in the repository has each of these with what it takes.

- **Video.** A video waits for wgpu to take the page's frames without a copy ([Sound](#sound)).
- More than one window, and with it [DevTools](../devtools/). Native menus.
- Browsers. Chrome and Edge (Chromium) were run, with a mouse, a keyboard, a pen and touch emulated by the
  browser, through WebGPU and WebGL2; Firefox and Safari, and a phone's browser, are checked by hand with
  `design/browser-checks.md`.
- The address is the fragment only: a path of the address itself (`/button`, which a server has to send
  the page for) is not followed, and the page's title does not change with the screen.
- A page reader finds nothing in a canvas, and the browser's own search and selection see no text.
