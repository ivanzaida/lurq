# Decisions

What was decided about lurq, by whom and why, newest first. A decision is written here the day it is taken, before
the work on it starts, so that the work, its review and a later session read the same thing. The design system's
first decisions (D1–D13: where components live, palette roles, recipes, breakpoints) are in
[design-system-plan.md](design-system-plan.md#3-decisions) and are not repeated; a decision here that changes one of
them says so. Decisions about phones and tablets are in [mobile-decisions.md](mobile-decisions.md).

Each entry: **the question**, **the decision**, **why**, and its **state** (decided, in work on a branch, built,
dropped). When a decision is replaced, the old entry stays and says what replaced it.

## 2026-11-18

### A lighter light theme: light fills, crisp lines

- **Question.** The light palette's greys were made darker on 2026-11-13 (`e820b1df`) because an all-white window
  told nothing apart; now there is too much grey. What reads as grey is the large fills (Canvas behind the page,
  Muted on the window's bar, tables' headers, a code block's bar, a tab list, Secondary buttons), not the lines.
- **Decision (Ivan, 2026-11-18, all three).** (1) Light fills: Canvas `#F7F7F8`, Muted / Secondary / Highlight
  `#F1F1F4`, SecondaryHover `#E6E6EB` (final values tuned by eye in the gallery, kept near these). (2) Crisp lines:
  Border `#D4D4DA`; Input stays `#9A9AA5` (a field's edge keeps 3:1). (3) The components that fill a large area with
  Muted move to `Background`/`Card` with a line (the window's bar, tables' headers, the code block's bar, tab lists,
  the sidebar, and what an audit finds); Muted stays for small things (chips, badges, inline code, slider tracks).
  The dark theme is not touched; the legacy theme keeps 0.62's look. First on the board (Ivan): the light
  variables and the light components frame (`Ovt1D`, "Components / light") changed and looked at before the code.
- **State.** Decided; in work in a session of its own, the board first.

### A frame callback for an app: `ctx.on_frame`

- **Question.** lurq's own animations (transitions, keyframes, presence, part animations) get their frames from the
  runtime, which asks the shell for the next frame when something will change. An app's own code has no way to be
  called once per displayed frame in step with the display: an `Interval` timer is not on the vsync (it jitters or
  draws too much), and `GpuViewport`'s `wants_redraw` serves GPU drawing only. Springs, physics, a drag's inertia, a
  game or a simulation on a canvas, smooth following of a value need it.
- **Decision (Ivan).** A `requestAnimationFrame` of lurq's: `ctx.on_frame(|frame| ..)`, told the frame's time and the
  time since the one before, called once per presented frame in step with the display, returning whether it wants
  another; while any wants frames the runtime asks for them; paused while the window is minimized or occluded, as
  animations are; on the tree's clock in tests (`set_animation_clock`), never the wall clock.
- **What the session that built it chose, where the decision left it open.**
  1. The callback returns whether it wants another frame, and `ctx.on_frame` returns a handle too
     (`FrameCallback`: `cancel()`, `start()`, `is_active()`): a spring at rest returns `false`, and the handler that
     moves its end starts it again. Called in `create` it is the component's until it unmounts; called in a render it
     is the callback of that place, as a `ctx.future` is.
  2. It runs before the frame is rendered and laid out, as `requestAnimationFrame` does: what it sets is in the frame
     it ran for. What was dirty before it is rendered first, so a component the frame no longer has runs none.
  3. Outside a component it is `WindowHandle::on_frame`. `App` has none: frames are a window's, each with its
     display's rate.
  4. The first frame after a hidden window shows again has a delta of zero, as a first frame, not the hidden time
     and not a clamp of it; a frame late while the window is shown has its real delta, which the app clamps where
     its integration needs it.
  5. Reduced motion does not stop it: what it moves is the app's.
- **Keyframes stay the engine's (Ivan asked, 2026-11-18, whether they should be put on this system for
  consistency).** They are on one system already, from the other side: a running callback is a timeline to the
  runtime (`Tree::has_active_timeline`, `next_timeline_change`), so the shell's pacing, the pause of a hidden window
  and the rule that a frame is laid out and not drawn from the kept list are one path for both. Keyframes are not
  moved onto `on_frame`: the engine steps them in the layout and knows when they next change, so a `Steps` spinner
  draws its steps and a delay draws nothing, where a callback asks for every refresh.
- **State.** Built on `claude/on-frame-c75375` (`crates/lurq/src/app/ctx/frames.rs`), with tests on the tree's clock
  and a section of the gallery's Animation page; to be merged into `feat/design-system`.

## 2026-11-17

### On macOS the system's lights are the window's controls, and the bar stays in full screen

- **Question.** On a Mac the gallery's bar drew three lights of its own: no menu of tilings under the pointer on the
  green one, no full screen at its press. In full screen the bar went away with its tabs, and after leaving full
  screen by the system's own keys the bar stayed hidden, since the shell never heard that the window came back.
- **Decision (Ivan, 2026-11-18, at the Mac: "на маку немає доку як на віндовсі … і фоулскрін не робить фулскрін",
  then "а чому вкладок в повноекранному нема").** Where an app's bar marks its controls (`.window_control(..)`), the
  window on macOS has a title bar that is not seen, its content under it, and the system's own lights are moved to
  the middles of the app's controls, which keep their room and are not drawn (`WindowInfo::native_controls`). In full
  screen the system shows its lights with its own bar at the top of the screen, and the app's bar stays, its tabs
  with it, as Safari and the Finder keep theirs; what is laid over the whole window (a video that fills the screen, a
  sheet) has all of it there. Whether the window is in full screen is read again every turn. Windows is as before.
- **State.** Built on `feat/design-system`; looked at on a Mac out of full screen, in full screen to be looked at.

### A video plays on macOS and iOS, with the system's player

- **Question.** `lurq::media` played a sound on Apple's systems (`AVAudioPlayer`) and said of a video that this build shows none there. The checks of Apple's systems (`apple-checks.md`) had "a video does not play" as what to expect.
- **Decision (Ivan, 2026-11-17, at the Mac: "так давай додамо").** A video plays there. `Media::open` gives it to the system's `AVPlayer`, which decodes it, plays its sound, keeps the two together and reads an address by itself; the frames are taken from it (`AVPlayerItemVideoOutput`) as two planes of gray and color and copied into the image a `Video` draws, through memory once, as everywhere but Direct3D. Bytes are written to a file of the system's temporary folder for as long as the media lives, since the player opens files and addresses. `Media::sound` stays on `AVAudioPlayer`.
- **Why the system's player and not its reader of samples** (`AVAssetReader`, which is what Media Foundation's reader is on Windows): the reader opens no address, cannot be sent to a time without being made again, and leaves the sound, its output and its clock to be written; the player has all of these. What it costs: the player says what changed on the main queue, so the media's thread asks it where it is instead of being told.
- **Frames stay on the card (Ivan, the same day, at the phone: "нам треба зіро копі як на віндовсі").** Where the `wgpu` renderer draws on Metal, the buffer the decoder gives (`CVPixelBuffer`, of a kind Metal makes textures of) is shown as it is, through `images::CvPixelBufferFrames`: the renderer makes its two textures of the buffer shown now and draws them, and nothing is copied. The renderer says it draws such frames (`images::cv_pixel_buffer_frames_drawn`); where nothing does, a frame is copied through memory once, as before. The same type is an app's own decoder's way to the screen, as `CardFrames` is on Windows (`MacosCvPixelBuffer::nv12`, `write_nv12` for one that decodes in memory); the gallery's "Frames of a decoder of your own" shows it there.
- **State.** In work on `feat/design-system`: built, its checks pass on a Mac, looked at in the gallery on a Mac; on a phone to be looked at.

### `GpuViewport` has no limits that are defects

- **Question.** The guide to `GpuViewport` (`docs/src/content/docs/gpu-viewport.md`) listed eight limits an app had to
  work around: lurq did not re-export wgpu; the frame had no scale factor and only the unclipped rectangle; a
  viewport with no size was one pixel square; nothing checked a texture's format; a validation error of the app's own
  wgpu calls ended the program; the `dx12` renderer failed a frame with a viewport; each window had a device of its
  own, so a slot could not be shown in two, and DevTools' window ran the app's extensions; and only an extension heard
  of a lost device. Ivan: known defects are fixed, not documented.
- **Decision (the session given the task, as the task left each shape to it).**
  1. `lurq::wgpu` is wgpu itself, re-exported with the `wgpu` feature: an app writes `use lurq::wgpu;` and pins
     nothing.
  2. `WgpuFrameInfo::scale_factor` is the window's, and `WgpuFrameInfo::visible_viewport(id)` is the part of the
     viewport that is shown (cut by its parents' clips and the window's edges, `None` when none of it is), beside
     `viewport(id)`, the whole box. Both in physical pixels; the engine is told the scale with the frame
     (`RenderEngine::set_scale_factor`, called by the tree before each frame).
  3. A `GpuViewport` with no size is 300 by 150 logical pixels, as lurq's `Canvas` is and as a browser's `<canvas>`
     is: a surface the app draws on has one default size in lurq, and it is seen. Given one side, the other stays
     the canvas's, as a `Canvas`'s does: there is no picture whose shape it keeps, and a viewport that took a
     picture's would not grow with flex. Filling the container was not taken: a fill is nothing in a column as
     high as its content, and the viewport would vanish as it did at one pixel. A size required in `new` was not
     taken: most viewports are sized by flex and percentages, not numbers.
  4. `set_texture_view` returns `Result<u64, WgpuTextureError>` and logs what it refuses: a texture without
     `TEXTURE_BINDING`, not 2D, multisampled, or of a format lurq's image shader cannot read as filtered color
     (integer, depth and stencil, 32-bit float). Everything else is shown right: an sRGB format is decoded by the
     card, and a linear one (`Rgba8Unorm`, `Rgba16Float`) holds linear color, which is what a shader writes into it;
     both are encoded once for the window. A slot that refuses a view keeps the one it had.
  5. A wgpu error of the app's own calls ends nothing: what an extension's `initialize`, `prepare`, `device_lost` and
     `shutdown` cause is caught in error scopes lurq pushes around them, and what the app causes elsewhere (a thread
     of its own) reaches the device's handler outside lurq's own work. Each is logged and given to the extensions of
     the device's engines, `WgpuFrameExtension::error(&WgpuAppError)`. An error of lurq's own work ends the program as
     before. In a browser only the extension's calls can be told apart, the browser telling the rest later.
  6. The `dx12` renderer leaves a viewport's box empty, what is under it showing through, and says once in the log
     that it draws no `GpuViewport`. Sampling a wgpu texture there would need the texture shared between two APIs'
     devices; an app that draws its own GPU code takes the `wgpu` renderer.
  7. The `wgpu` engines of an app's windows share one device by default: an engine made without `with_device` takes
     the device of another engine on its thread with the same backends, features and limits, while one draws, and
     makes one otherwise. `WgpuRenderEngine::with_own_device()` keeps one apart. So one slot can be shown in two
     windows: both draw the slot's one view, each stretched to its box. lurq's DevTools window runs none of the app's
     extensions (`RenderEngine::draws_tooling`).
  8. A lost device is an app event: `AppEvent::GraphicsDeviceLost`, told to each window's `on_app_event` handler
     once its engine was replaced, after the extensions were told and before the window is drawn with the new device.
- **State.** Built on `claude/gpu-viewport-8dee02` (2026-11-17), to be merged.

### lurq's own Direct3D 12 renderer draws the frames of a card

- **Question.** `images::CardFrames` (a video's frames kept on the graphics card) were drawn by the `wgpu` renderer alone; under `Dx12RenderEngine` a video went through memory and an app's decoder was not drawn.
- **Decision (Ivan: "вливай гілку і давай за вулкан візьмемось", then "працюй далі" when this renderer was named as next after Vulkan).** The order of Windows' backends is Vulkan, then lurq's own `dx12` renderer, and OpenGL only if someone uses it. The `dx12` renderer draws `CardFrames` as `wgpu` does: on the card where its device opens them, through memory where the decoder is on another card.
- **State.** Built on `claude/ds-vulkan`, not merged; see `design/handoff/media-playback.md`.

### The `dx12` renderer's older shared NV12 textures go

- **Question.** `Dx12RenderEngine` had a way of its own to draw a video's frame from the card, older than `images::CardFrames`: `Dx12VideoSurfaceAllocator`, `Dx12Nv12Surface`, `Dx12Nv12Image`, `Dx12Nv12ImageSlot` and `NativeImageData::from_dx12_nv12`. With `CardFrames` drawn by that renderer there are two ways to do one thing.
- **Decision (Ivan, 2026-11-17: "треба прибрати і додати це в брейкінг ченжі").** The older way is removed, and the removal is listed under Breaking in the changelog. An app gives its frames as `CardFrames`.
- **State.** Built on `claude/ds-vulkan`, not merged.

### One NV12 texture under Vulkan on a card of NVIDIA's goes through memory, and that is written down

- **Question.** Under Vulkan on the RTX 5080 one NV12 texture of a decoder is not drawn where it is: given to Vulkan, its color is read from the wrong place and the device is lost (what was tried is in `design/handoff/media-playback.md`). A way round is a copy on the card: each frame drawn into a texture of red, green and blue that Vulkan does take.
- **Decision (Ivan, 2026-11-17: "давай просто тоді позначимо про цю ваду в доках і не будемо нічого переводити").** Nothing is converted. The fault is written in the docs: the handoff, the changelog, and what `images::card_frames_nv12()` and `CardSurface::Nv12` say. Such frames go through memory there, as before; a decoder gives another kind of surface, or the app draws through Direct3D 12.
- **State.** Done on `claude/ds-vulkan`: the docs written, the way round thrown away.

### A video in a browser is not built on a copy

- **Question.** In a browser build lurq decodes no video. What wgpu 29 has there is `Queue::copy_external_image_to_texture` from a `<video>` element or a `VideoFrame`: one copy on the card for each frame. WebGPU's way with no copy, an external texture (`importExternalTexture`), is `unimplemented!` in wgpu's backend for the web.
- **Decision (Ivan, 2026-11-17: "нє це повна параша").** Not built. A video in a browser waits for external textures in wgpu's web backend.
- **State.** Decided; nothing written.

### OpenGL is not worked on

- **Decision (Ivan, 2026-11-17: nobody uses it).** Frames on the card and a lost device are not done for wgpu's OpenGL backend on Windows. A video there goes through memory, as before.
- **State.** Decided.

### Back and forward in one place

- **Question.** Going back and forward is spread like undo was: `router.back()`/`forward()`, `on_history_swipe`
  (a trackpad's swipe, a finger from the edge), the stack's back button calling the navigator; the mouse's side
  buttons arrive as `MouseButton::Other(4/5)` and lead nowhere; Alt+←/→ (Cmd+[ / ] on a Mac), Android's back and a
  browser's back are not handled.
- **Decision (Ivan).** One command, as `EditCommand` is: `NavigationCommand::Back` / `Forward`, given by every
  source — the mouse's side buttons, Alt+← / Alt+→ (Cmd+[ / Cmd+] on Apple), a trackpad's history swipe, a finger
  from the edge, Android's back, a browser's back and forward. Element events **`on_history_back` /
  `on_history_forward`** (named after `on_history_swipe`, for consistency), dispatched from the focused element
  outward, a handler that does nothing passing it on (a dialog may close on back instead of leaving the page). With
  none taking it, the mounted router goes back or forward. `Ctx` runs the same from a menu or a button; MCP and the
  emulator too. `on_history_swipe` and `StackOutlet` move onto it.
- **State.** Built on `claude/history-nav-9fccc7` (2026-11-17), to be merged: `NavigationCommand` and
  `NavigationSource`, `KeyboardEvent::navigation_command` (Alt+←/→, Cmd+[/] on Apple by the bracket or its place,
  `BrowserBack`/`BrowserForward` for a keyboard's keys and Android's back), `MouseButton::Back`/`Forward` on their
  release, `on_history_back`/`on_history_forward` with `HistoryEvent::pass_on`, the router as the default (on the
  way, then elsewhere in the window, innermost first), `on_history_swipe`'s default, a browser's `popstate` to the
  entry beside through the command (put back where something else took it), `Ctx::history_back`/`history_forward`/
  `history_runner`, `WindowHandle::history_back`/`history_forward`, MCP's `history_back`/`history_forward` and
  buttons `back`/`forward`, the emulator's side buttons as the phone's back, and `StackOutlet`'s way back and edge
  from its top screen. Decided in the session where the decision left it open: a key and a request start at the
  focus, with nothing focused at the last element that hears the command; a pointer's source starts under the
  pointer, then the focus; Alt+arrows go back in a text input too (its word moves are Ctrl's; Apple's keys are
  Cmd's); in a browser the side buttons are the browser's. Not built: Android's back with nothing taking it does not
  leave the app (winit takes the key), and `ui`'s overlays do not close on going back by themselves (an app's
  handler does it).

### The gallery shows how to style

- **Question.** lurq styles at four levels — the theme's tokens, a component's recipe in the theme (parts by
  variant, size, state, density, breakpoint), one instance (`.style(part, ..)`, `.rules`, `.state`, `.force`), a
  renderer — and the gallery shows none of it: the props tables leave styling out, the playground has no view of a
  component's parts, Getting Started has no page on it.
- **Decision (Ivan).** A page of its own, **Styling**, in Getting Started: the four levels with live examples (a
  palette role changes everything, a recipe patched changes every button, `.style` one button, a renderer of one's
  own). And in every playground **a section of its own for styling**: the component's parts and states, a part
  picked and restyled live (color, radius, padding, shadow…), the code beside it for one instance
  (`.style(Part, ..)`) and for all of them (`theme.patch_recipe::<X>(..)`). Built once in
  `crates/gallery/src/playground.rs` from the parts and states each component already declares.
- **State.** Built (`claude/gallery-styling-578adc`, 2026-11-17): the Styling page (`/styling`), the playground's
  Styling tab beside its props on every component's page, and in `lurq::ui` `Themed::parts`/`states` (`Declared`)
  and the `Styled` trait that every builder implements.

### The gallery's Engine group replaces the demo

- **Question.** The gallery was to replace `crates/demo`, but it had no page for the engine's primitives (layout,
  scroll, paint, text, motion, the reactive model), which only the demo showed, as hand-made pages in a theme of
  their own.
- **Decision (Ivan).** The gallery gets a group of pages for the engine, built as its component pages are (a
  playground first, then sections with their code), with every demo page's content and the primitives the demo
  never showed (`Canvas`, `Svg`, `VirtualizedList`, `GpuViewport`, `ZoomScroll`, drag and drop, rich text, the
  router and outlets, the engine's own controls and overlays). Then `crates/demo` is removed; `crates/mobile_demo`
  stays. The group is named **Engine** (the session that built it chose, as the task left the name to it): its pages are not all primitives (signals, context, persistent storage and
  events are the app's model, not boxes), the demo called itself the engine's, and a group's name in the side bar is
  one short word, as Actions, Data and Desktop are. Its pages live under `/engine/…`, so a primitive and the design
  system's component of the same name (`/slider`, `/engine/controls`) never share a route. What the demo's binary did
  besides its pages moved: `--renderer dx12` and `--profile-log` to the gallery, the lifecycle window and the
  continuous video probe to `lurq`'s examples.
- **State.** Built (`claude/gallery-engine`, merged 2026-11-17); `crates/demo` is gone.

### Code highlighting is pluggable, its token names open, its colors the theme's

- **Question.** `ui::CodeBlock` and Markdown color code with lurq's own scanner, which knows Rust alone, into seven
  fixed kinds painted in the palette's roles; an app can neither bring its own tokenizer nor color code apart from
  the rest of its interface.
- **Decision (Ivan).**
  1. Token kinds are **open names with a dotted hierarchy** (`keyword`, `keyword.control`, `string.escape`,
     `function.macro`, …), as TextMate and tree-sitter name them; a color is looked up dropping the tail
     (`keyword.control.rust` → `keyword.control` → `keyword` → plain). `CodeTokenKind` as a closed enum goes.
  2. **A code palette in the theme**: a name → color map of its own, for the light and the dark scheme, defaulting to
     the roles used now (so a theme left alone looks as today); `set_code_color`, a whole scheme, and a block's own
     `.token_style(name, ..)` (color, weight, italic) override it.
  3. **A tokenizer is pluggable, one per language**: a `Tokenizer` trait (or any `Fn(&str) -> Vec<CodeToken>`) is
     registered for a language and its aliases (`App::register_tokenizer("sql", ..)`); registering one for a
     language that has one replaces it (lurq's own included). A language with none registered is plain text. A block
     may still take its own (`.tokenizer(..)`) over the registry. It is named a tokenizer, not a highlighter (Ivan):
     it splits code into named tokens and colors nothing. A tokenizer may keep state between renders and may
     be incremental (Ivan, the same day): by line, with an opaque state at each line's start so an edit re-tokenizes
     only from the changed line until the state agrees again; a handle to tell lurq its tokens are stale; and edit
     hints (tree-sitter's shape) for incremental parsers. All optional: a function stays one line. lurq's own becomes a
     table-driven scanner with a public language table (Rust, TOML, JSON, JS/TS, shell, Python, CSS, HTML/XML, YAML,
     SQL, Markdown), no dependency, wasm-safe. **No `syntect` or tree-sitter adapter in lurq**: the gallery crate
     shows one as an example of plugging a tokenizer in.
- **With it.** A cache of tokens by code, language and tokenizer; a long block draws the lines in view; `InlineCode`
  takes a language; the editable text's tokens use the same tokenizer and colors.
- **State.** Built on `claude/code-highlight-ef7166` (2026-11-17), to be merged: `lurq::code` (`CodeToken` with a
  dotted name, `Tokenizer`, `CodeTokenizer`, the registry and its cache, `CodeLanguage` and the scanner, thirteen
  languages), `CodeScheme` and `CodeStyle` in the theme, `CodeBlock::tokenizer`, `token_style` and `code_scheme`, a
  long block windowed past 400 lines (in its own room, or in the window where it has no greatest height),
  `InlineCode::language`, `TextInput::tokens_with` and `code_language`, and the gallery's `syntect` adapter
  (`crates/gallery/src/syntect_code.rs`). A tokenizer with state, the same day: `Tokenizer::tokens_from` with a
  `LineState` (any `Clone + PartialEq` value, kept by lurq at each line's start of a code field's text and of a
  mounted block's code, an edit re-tokenized from its line until the states agree; lurq's scanner and the gallery's
  syntect go by line), `TokenizerHandle::invalidate` (from `register_tokenizer` or `CodeTokenizer::handle`), and
  `tokens_in`, `edited` (a `CodeEdit` in tree-sitter's shape) and `dropped` for a code field's `BufferId`.

### Link Preview works from the link alone; every field overrides

- **Question.** Where does a link preview's site, title, description and picture come from?
- **Decision (Ivan).** `ui::LinkPreview::new(url)` fetches what the page says about itself and shows it;
  `.title`, `.description`, `.site`, `.image`, `.icon`, `.video` each override the fetched field; when every field it
  shows is given, nothing is fetched. The fetcher is lurq's own, behind `http` (the start of the page; `<title>`,
  OpenGraph, Twitter Card, the icon; a small scanner of the `<head>`, no HTML-parser crate; a cache by URL that an app
  can replace). Without `http` it shows what it was given and the address.
- **With it.** Read-only styled text holds inline elements in its line (the inline form "in a sentence"), built on
  the inline objects of the text input; `open_url` in the engine, by each platform's own call, which the preview uses
  unless `on_open` is given.
- **State.** Built (`claude/link-preview`, merged 2026-11-17), with `Ctx::focus_handle`, `.copy_text`, inline objects without `markdown` and the `/favicon.ico` fallback.

### Cargo features

- **Question.** 28 features: five pull no dependency, pictures spread over six, two localization systems, the `ui`
  charts behind `canvas`, `default = []`.
- **Decision (Ivan).** `form`, `router`, `query` stay features and go into `default`. `i18n` goes: `I18n` (an app's
  keyed translations) is always built, beside `core::Locale` (lurq's own words). `default` is the desktop set:
  `winit`, `wgpu`, `clipboard`, `file_dialog`, `image`, `canvas`, `form`, `router`, `query`. Two features for
  pictures and drawing: `image` (codecs) and `canvas` (drawing, with `svg` folded in); `raster` and `render`
  internal; `screenshot` uses `image`; one `png`. `resources` goes. `devtools` and `perf_profile` stay, their
  call-site cfgs behind macros. The browser keeps an explicit list. Tests: seven runs (A–G) in place of five feature
  combinations. SVG is always rasterized at the size it is drawn: the tessellation an `svg`-only build had is removed
  (Ivan, the same day), as no build has it once `canvas` brings `raster`.
- **State.** Built (the session "Clean up lurq's Cargo features", merged 2026-11-17 with the 0.62.0 release from
  `master`): 25 features, the desktop `default`, the runs A–G in `target/suites2.sh`.

### Editing commands in one place

- **Question.** Ctrl+Z on the signature pad did nothing after a stroke, Ctrl+Shift+Z did nothing, and nothing worked
  in a Ukrainian layout; every component matched keys of its own.
- **Decision (Ivan: "undo & redo in one place", then "undo, redo, copy, paste").** `app::events::EditCommand`
  (`Undo`, `Redo`, `Cut`, `Copy`, `Paste`, `SelectAll`) and `KeyboardEvent::edit_command()` are the only code that
  knows those keys: Ctrl (Cmd on Apple), by the Latin letter or by the key's place in any layout. A focused text
  input does it first, then `on_undo` / `on_redo` / `on_select_all` / `on_copy` / `on_paste` from the focused element
  outward, a handler with nothing to do passing it on. `Ctx::edit`, `Ctx::edit_runner`, `WindowHandle::edit` run the
  same from a menu or a toolbar. A press that begins a drag focuses what the element `focuses`.
- **State.** Built, on `feat/design-system` (`77ffe27c`).

### The old `lurq::components` meet the design system

- **Question.** Plan decision D7 (form controls on the new components, `FormTheme` deprecated) was never done, and
  the old styled components live on beside their `ui` twins.
- **Decision (Ivan: "давай роби").** The form controls are rebuilt on `ui` and `FormTheme` deprecated. Every old
  type is an engine primitive (stays, its doc names the `ui` component an app wants), a styled duplicate (deprecated
  in favour of `ui::X`, as `components::Image` is for `ui::Picture`), or styled with hard-coded colors (`new()` /
  `dark()` become recipes). Each break in `migration-v0-63.md`.
- **State.** The old styled components: built (`claude/legacy-styled`, merged 2026-11-17: `Checkbox` and `ActivityIndicator` deprecated for `ui::Checkbox` and `ui::Spinner`; the phone pieces, `SelectStyle`, the stack's shades and the unstyled checkbox and slider nodes on recipes, the legacy theme keeping 0.62's look). The phone pieces as real `ui` components (Tab Bar, App Bar, Large Title, Bottom Sheet, Swipe Row, Pull To Refresh, as the board draws them) are not built. The form controls: built (`claude/legacy-form`, merged 2026-11-17: thin bindings over `ui::Field`, `Input`, `Textarea`, `Checkbox`, `Slider`, `Button`; `FormTheme` deprecated; the gallery's `/form` page), with an engine fix on the way: a mounted component as the root of an `in_theme` scope keeps the theme.

### Words lurq writes are in the locale

- **Decision (Ivan, `AGENTS.md`).** Every word a person sees or a tool reads is a field of `core::Locale`, in six
  languages; a component takes it from `ui::current_locale()`; a single-word method on the component wins.
- **State.** Built: `claude/ds-locale-words` and the thirteenth batch's words, merged.

## 2026-11-16

### Every family on the board is built

- **Decision (Ivan: "давай ці 6 доробимо").** The six families held for a request (Signature Pad, Angle Slider, QR
  Code, Watermark, Transfer, Cascader) were built. With them every family the board draws has a type in `lurq::ui`.
- **State.** Built; see the plan's 14.27.
