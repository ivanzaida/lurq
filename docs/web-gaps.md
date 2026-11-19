# Web: what is not done

What a lurq app in a browser still does differently from the same app on a desktop, or does not do at all, beyond
what a page cannot do (`docs/src/content/docs/web.md`, "What a page cannot do"). An entry goes when its gap is closed.
What to check by hand in each browser is `design/browser-checks.md`.

"Not checked" means nobody has run it in that browser, not that it is known to be broken. The browser's work of
2026-11-19 was run in Chromium (the Claude app's browser and a headless Chrome, Windows), with a mouse, a keyboard,
a pen and touch emulated by the browser, through WebGPU and WebGL2.

## Browsers

- **Firefox, Safari, a phone's browser: not checked** since the browser became supported. `design/browser-checks.md`.
- **WebGL2 was asked for, not fallen back to.** A browser that has no WebGPU, where `WgpuDevice::request()` has to
  find that out and take WebGL2 by itself: not checked.
- **A lost device or context.** The engine makes its surface again when the surface is lost, with the device it
  was given: a device that is itself gone is not asked for again. Not checked what a person then sees.

## Media

- **Video.** Not played: decided 2026-11-17, a video in a browser waits for wgpu's web backend to implement external
  textures (`importExternalTexture`); wgpu 30.0.2 has `create_external_texture` `unimplemented!` there.
- **A sound's autoplay.** A sound told to play with no press of a person's is paused again by the browser, and its
  state says so. A media has no event for "the browser refused"; it is not playing.

## Input

- **A masked input** (a password) takes keys, not composition: on a phone's keyboard, a password's letters come as
  keys only where the keyboard sends them so.
- **Hover before the pointer moves.** The wheel says where the pointer is (`web::WheelPointer`), so a page scrolls
  under a pointer at rest. What is under that pointer is not hovered until it moves.

## Pictures and the clipboard

- **A picture copied to the clipboard** (`copy_image_to_clipboard`) and **pasted from it** are built and not run in
  a browser: no page of the gallery copies or pastes a picture.
- **The Backgrounds and SVG pages' "from a resource" sections** read pictures by path through the old resource
  loader (`http`), which a page has not: they say so. A `ui::Picture` of a file reads it through the app's `Loader`.

## Windows

- **More than one window.** winit makes a canvas for every window, and the shell puts only the first into the
  page: a secondary window's canvas is made and never shown. What it takes: a place in the page for each (a
  floating panel, or one the page docks), its size and position from what the window asked for, a title bar the
  shell moves it by (`drag_window` does nothing in a browser), and the canvas taken away when the window closes.
- **DevTools** is a tree in a secondary window, so it waits for the above.
- **Native menus**: not there.

## The page

- **The address is the fragment only.** A path of the address itself (`/button`) is not followed: it takes a server
  that sends the page for every path. The page's title does not change with the screen.
- **A history entry's state does not outlive a reload** (`push_with_state`): it is in memory.
- **A page reader finds nothing.** Everything is pixels of one canvas: no text to select by the browser, to search
  with Ctrl+F, to translate or to read out.
- **cosmic-text has no list of fallback fonts** for a browser: a character the loaded fonts lack is looked for in
  every loaded font in turn; a page that shows text of many scripts loads fonts for them.
- **The wasm build has warnings** of code that is dead in a browser (the thread pool, the readers of files).
